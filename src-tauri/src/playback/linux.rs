//! Linux native video surface for the libmpv render API.
//!
//! Tauri on Linux is GTK3 + WebKitGTK. We place a `GtkGLArea` *behind* the webview
//! using a `GtkOverlay`, so video composites underneath the transparent Dioxus
//! overlay (Tauri's `transparent: true` makes the WebKitGTK background alpha 0). One
//! code path covers both Wayland and X11: `GtkGLArea` picks the GL backend itself
//! (EGL on Wayland, GLX on X11), so we never touch the windowing protocol directly.
//!
//! Like macOS (and unlike Windows), all GL work happens on the GTK main thread:
//! `GtkGLArea` only renders from its own `render` signal, which fires on the main
//! loop. mpv's update callback (which may fire on any thread) just sends on an async
//! channel; a `spawn_future_local` task on the main loop receives it and calls
//! `queue_render`, so the GL context is only ever touched from one thread.

use std::cell::RefCell;
use std::ffi::{c_void, CString};
use std::rc::Rc;
use std::sync::OnceLock;

use cathode_core::error::AppError;
use gtk::glib;
use gtk::prelude::*;
use libloading::os::unix::Library;
use libmpv2::render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType};
use libmpv2::Mpv;

/// `glGetIntegerv` pname for the bound draw framebuffer. `GtkGLArea` renders into its
/// own FBO (not 0), so we must query it each frame and hand it to mpv.
const GL_DRAW_FRAMEBUFFER_BINDING: u32 = 0x8CA6;

type GlGetIntegerv = unsafe extern "C" fn(u32, *mut i32);

/// A platform `getProcAddress` for resolving GL entry points: `eglGetProcAddress`
/// (Wayland) with a `glXGetProcAddressARB` fallback (X11). Takes a C string and
/// returns the function pointer (or null). Function pointers are `Send + Sync`.
type GetProcAddr = unsafe extern "C" fn(*const std::os::raw::c_char) -> *mut c_void;

/// The GL symbol resolvers, set up once in [`load_gl`]. We resolve through the GL
/// platform rather than dlsym-ing libepoxy: some libepoxy builds export only the
/// `epoxy_`-prefixed dispatch pointers, not the plain `gl*` symbols mpv asks for.
struct GlLoaders {
    egl: Option<GetProcAddr>,
    glx: Option<GetProcAddr>,
}

static GL_LOADERS: OnceLock<GlLoaders> = OnceLock::new();

fn err(context: &str) -> AppError {
    AppError {
        code: "playback".to_string(),
        message: format!("video surface: {context}"),
    }
}

/// dlopen `libname` and pull out its `getProcAddress` symbol, leaking the handle so
/// the resolver stays valid for the process lifetime. Returns `None` if either step
/// fails (e.g. the GLX lib is absent on a pure-Wayland system).
fn load_one(libname: &str, symbol: &str) -> Option<GetProcAddr> {
    let lib = unsafe { Library::new(libname) }.ok()?;
    let name = CString::new(symbol).ok()?;
    let func = unsafe {
        let sym = lib.get::<GetProcAddr>(name.as_bytes_with_nul()).ok()?;
        *sym
    };
    // Keep the library mapped; `func` points into it.
    std::mem::forget(lib);
    Some(func)
}

/// Set up the GL symbol resolvers. Called from `attach` (main thread) before the GL
/// area realizes. At least one of EGL/GLX must be usable.
fn load_gl() -> Result<(), AppError> {
    if GL_LOADERS.get().is_some() {
        return Ok(());
    }
    let egl = load_one("libEGL.so.1", "eglGetProcAddress");
    let glx = load_one("libGL.so.1", "glXGetProcAddressARB");
    if egl.is_none() && glx.is_none() {
        return Err(err("no eglGetProcAddress/glXGetProcAddressARB available"));
    }
    let _ = GL_LOADERS.set(GlLoaders { egl, glx });
    Ok(())
}

/// Resolve a GL entry point through the platform `getProcAddress`. mpv calls this
/// while creating the render context; we also use it for the framebuffer query. EGL
/// (with `EGL_KHR_get_all_proc_addresses`) and GLX both return core GL functions.
fn resolve(name: &str) -> *mut c_void {
    let Some(loaders) = GL_LOADERS.get() else {
        return std::ptr::null_mut();
    };
    let Ok(cname) = CString::new(name) else {
        return std::ptr::null_mut();
    };
    if let Some(egl) = loaders.egl {
        let p = unsafe { egl(cname.as_ptr()) };
        if !p.is_null() {
            return p;
        }
    }
    if let Some(glx) = loaders.glx {
        let p = unsafe { glx(cname.as_ptr()) };
        if !p.is_null() {
            return p;
        }
    }
    std::ptr::null_mut()
}

/// libmpv resolves the GL entry points it needs through this callback.
fn get_proc_address(_ctx: &(), name: &str) -> *mut c_void {
    resolve(name)
}

/// The framebuffer `GtkGLArea` has bound for this `render` pass. Defaults to 0 if the
/// query symbol is missing, which is the right fallback (the default framebuffer).
fn current_draw_fbo() -> i32 {
    let ptr = resolve("glGetIntegerv");
    if ptr.is_null() {
        return 0;
    }
    let get_integerv: GlGetIntegerv =
        unsafe { std::mem::transmute::<*mut c_void, GlGetIntegerv>(ptr) };
    let mut fbo: i32 = 0;
    unsafe { get_integerv(GL_DRAW_FRAMEBUFFER_BINDING, &mut fbo) };
    fbo
}

/// Build the GL area, slot it behind the webview, wire mpv's render context, and keep
/// everything alive for the app's lifetime. Must run on the GTK main thread (the Tauri
/// `setup` closure does).
pub fn attach(window: &tauri::WebviewWindow, mpv: &'static Mpv) -> Result<(), AppError> {
    let _span = tracing::info_span!("video_surface_attach").entered();

    load_gl()?;

    // tao builds: GtkWindow -> GtkBox (default vbox) -> WebKitGTK webview. We slot a GL
    // area behind the webview with a GtkOverlay (video at the bottom, transparent UI on
    // top). Crucially, Tauri's borderless-resize handler walks `webview.parent().parent()`
    // and unwraps a downcast to `gtk::Window`, so the webview's grandparent must stay the
    // window. We therefore make the overlay the window's *direct* child (replacing the
    // vbox) and re-add the webview as an overlay child, keeping it two hops from the
    // window: webview -> overlay -> window. The vbox only ever holds the webview here (no
    // native menu), so detaching it is safe; wry adapts to whatever container holds it.
    let gtk_window = window
        .gtk_window()
        .map_err(|e| err(&format!("gtk_window unavailable: {e}")))?;
    let vbox = window
        .default_vbox()
        .map_err(|e| err(&format!("default_vbox unavailable: {e}")))?;
    let webview = vbox
        .children()
        .into_iter()
        .next()
        .ok_or_else(|| err("no webview child in vbox"))?;

    let overlay = gtk::Overlay::new();
    let gl_area = gtk::GLArea::new();
    // Opaque video; the transparent webview composites over it.
    gl_area.set_has_alpha(false);
    gl_area.set_hexpand(true);
    gl_area.set_vexpand(true);

    vbox.remove(&webview);
    gtk_window.remove(&vbox);
    overlay.add(&gl_area);
    overlay.add_overlay(&webview);
    gtk_window.add(&overlay);

    // mpv's update callback fires on an arbitrary thread; bounce a redraw onto the main
    // loop, where the GL context lives. The local task captures the (non-Send) widget.
    let (tx, rx) = async_channel::unbounded::<()>();
    {
        let gl_area = gl_area.clone();
        glib::spawn_future_local(async move {
            while rx.recv().await.is_ok() {
                gl_area.queue_render();
            }
        });
    }

    // The render context must outlive `attach`; it is created lazily on `realize` (the
    // GL context is current only then) and shared with the `render` handler. Both run on
    // the main thread, so `Rc`/`RefCell` is sound.
    let render_cell: Rc<RefCell<Option<RenderContext<'static>>>> = Rc::new(RefCell::new(None));

    {
        let render_cell = render_cell.clone();
        gl_area.connect_realize(move |area| {
            area.make_current();
            if let Some(e) = area.error() {
                tracing::error!("GLArea realize failed: {e}");
                return;
            }
            // get_proc_address runs here, so the GL context must already be current.
            match mpv.create_render_context(vec![
                RenderParam::ApiType(RenderParamApiType::OpenGl),
                RenderParam::InitParams(OpenGLInitParams {
                    get_proc_address,
                    ctx: (),
                }),
            ]) {
                Ok(mut ctx) => {
                    let tx = tx.clone();
                    ctx.set_update_callback(move || {
                        let _ = tx.send_blocking(());
                    });
                    *render_cell.borrow_mut() = Some(ctx);
                    tracing::info!("video surface attached");
                }
                Err(e) => tracing::error!("render context creation failed: {e}"),
            }
        });
    }

    {
        let render_cell = render_cell.clone();
        gl_area.connect_render(move |area, _gl_ctx| {
            if let Some(ctx) = render_cell.borrow().as_ref() {
                // Size in physical pixels (scale factor handles HiDPI/fractional scaling),
                // re-read each frame so resizes are handled for free. fbo is GtkGLArea's own
                // framebuffer; flip because GL is Y-up but video is Y-down.
                let scale = area.scale_factor();
                let width = area.allocated_width() * scale;
                let height = area.allocated_height() * scale;
                if let Err(e) = ctx.render::<()>(current_draw_fbo(), width, height, true) {
                    tracing::error!("mpv render failed: {e}");
                }
            }
            glib::Propagation::Stop
        });
    }

    overlay.show_all();
    Ok(())
}
