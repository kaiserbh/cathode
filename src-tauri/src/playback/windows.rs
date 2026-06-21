//! Windows native video surface for the libmpv render API.
//!
//! We create a separate top-level `HWND` positioned directly *behind* the Tauri window
//! and glued to it, so video composites underneath the transparent Dioxus overlay. A
//! child window below the WebView2 does NOT work: windowed WebView2 transparency reveals
//! the desktop backdrop, not lower-z siblings. A separate top-level window does, because
//! the desktop compositor blends a transparent top-level window over the windows behind
//! it — and since the video window is behind, all mouse input still reaches the webview.
//! mpv draws into this window through an OpenGL ES render context provided by **ANGLE**
//! (EGL → Direct3D 11), which gives clean interop with the `d3d11va` hardware decoder
//! (`hwdec=auto-safe`).
//!
//! Unlike macOS (which must render on the main thread for Cocoa), all GL work here runs
//! on a **dedicated render thread** that owns the EGL context. mpv's update callback
//! only signals that thread; `eglSwapBuffers` blocks on vsync, so rendering on the UI
//! thread would starve the window's message pump (frozen, un-draggable window that DWM
//! then ghosts — i.e. it also looks transparent). The render thread keeps the UI thread
//! free.

use std::ffi::c_void;
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread;

use cathode_core::error::AppError;
use khronos_egl as egl;
use libmpv2::render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType};
use libmpv2::Mpv;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, RegisterClassW, SetWindowPos,
    ShowWindow, SIZE_MINIMIZED, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_SHOWWINDOW, SW_HIDE,
    SW_SHOWNOACTIVATE, WM_DESTROY, WM_SIZE, WM_WINDOWPOSCHANGED, WNDCLASSW, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_POPUP,
};

/// The dynamically loaded ANGLE EGL instance. EGL 1.4 covers everything we use
/// (display/config/context/surface + `eglGetProcAddress`).
type Egl = egl::DynamicInstance<egl::EGL1_4>;

/// Subclass id for the parent-window resize hook (arbitrary, unique within the window).
const SUBCLASS_ID: usize = 1;

fn err(context: &str) -> AppError {
    AppError {
        code: "playback".to_string(),
        message: format!("video surface: {context}"),
    }
}

/// libmpv resolves the GL entry points it needs through this callback. ANGLE returns
/// both EGL and GLES pointers from `eglGetProcAddress` (`EGL_KHR_get_all_proc_addresses`),
/// so a single lookup covers everything mpv asks for.
fn get_proc_address(egl: &&'static Egl, name: &str) -> *mut c_void {
    match egl.get_proc_address(name) {
        Some(f) => f as *mut c_void,
        None => std::ptr::null_mut(),
    }
}

/// Wakes the render thread when mpv has a new frame. mpv's update callback fires on an
/// arbitrary thread and just flips the flag; the render thread waits on the condvar.
struct RenderSignal {
    pending: Mutex<bool>,
    cvar: Condvar,
}

impl RenderSignal {
    fn new() -> Self {
        Self {
            pending: Mutex::new(false),
            cvar: Condvar::new(),
        }
    }

    fn notify(&self) {
        *self.pending.lock().unwrap() = true;
        self.cvar.notify_one();
    }

    /// Block until a frame is pending, then consume the flag.
    fn wait(&self) {
        let mut pending = self.pending.lock().unwrap();
        while !*pending {
            pending = self.cvar.wait(pending).unwrap();
        }
        *pending = false;
    }
}

/// The child window's procedure: it never paints itself (the render thread presents via
/// the swapchain), so everything goes to the default handler.
unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// Glue the video window to the owner's client area, in screen coordinates, pinned
/// directly below the owner in z-order. `show` also makes it visible (used on create
/// and on un-minimize).
fn place_behind(owner: HWND, video: HWND, show: bool) {
    let mut rect = RECT::default();
    if unsafe { GetClientRect(owner, &mut rect) }.is_err() {
        return;
    }
    let mut origin = POINT::default();
    let _ = unsafe { ClientToScreen(owner, &mut origin) };
    let mut flags = SWP_NOACTIVATE | SWP_NOOWNERZORDER;
    if show {
        flags |= SWP_SHOWWINDOW;
    }
    // Insert after `owner` => directly below it in z-order, so the transparent UI
    // composites on top of the video.
    unsafe {
        let _ = SetWindowPos(
            video,
            Some(owner),
            origin.x,
            origin.y,
            rect.right - rect.left,
            rect.bottom - rect.top,
            flags,
        );
    }
}

/// Owner-window subclass: keep the video window matched to the Tauri window's geometry,
/// z-order, and visibility. `WM_WINDOWPOSCHANGED` covers move/resize/z-order; `WM_SIZE`
/// handles minimize/restore; `WM_DESTROY` tears the video window down.
unsafe extern "system" fn owner_subclass(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    refdata: usize,
) -> LRESULT {
    let video = HWND(refdata as *mut c_void);
    match msg {
        WM_WINDOWPOSCHANGED => place_behind(hwnd, video, false),
        WM_SIZE if wparam.0 == SIZE_MINIMIZED as usize => unsafe {
            let _ = ShowWindow(video, SW_HIDE);
        },
        WM_SIZE => {
            unsafe {
                let _ = ShowWindow(video, SW_SHOWNOACTIVATE);
            }
            place_behind(hwnd, video, false);
        }
        WM_DESTROY => unsafe {
            let _ = DestroyWindow(video);
        },
        _ => {}
    }
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

/// Register the child window class once per process.
fn register_class(hinstance: HINSTANCE) -> PCWSTR {
    static REGISTERED: OnceLock<()> = OnceLock::new();
    let name = w!("CathodeMpvSurface");
    REGISTERED.get_or_init(|| {
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance,
            lpszClassName: name,
            ..Default::default()
        };
        unsafe {
            RegisterClassW(&wc);
        }
    });
    name
}

/// The top-level window `HWND`, obtained via `raw-window-handle` (independent of which
/// `windows`-crate version tauri itself uses).
fn parent_hwnd(window: &tauri::WebviewWindow) -> Result<HWND, AppError> {
    let handle = window
        .window_handle()
        .map_err(|e| err(&format!("window handle: {e}")))?;
    match handle.as_raw() {
        RawWindowHandle::Win32(h) => Ok(HWND(h.hwnd.get() as *mut c_void)),
        _ => Err(err("not a Win32 window")),
    }
}

/// Create the top-level video window (no taskbar entry, never activates) and place it
/// directly behind the owner. It is a separate top-level window, not a child, so the
/// transparent Tauri window composites over it.
fn create_video_window(owner: HWND, class: PCWSTR, hinstance: HINSTANCE) -> Result<HWND, AppError> {
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class,
            w!(""),
            WS_POPUP,
            0,
            0,
            0,
            0,
            // No owner: owned windows are forced *above* their owner, but we need it
            // behind. We pin z-order manually via the owner subclass instead.
            None,
            None,
            Some(hinstance),
            None,
        )
    }
    .map_err(|e| err(&format!("CreateWindowExW: {e}")))?;
    place_behind(owner, hwnd, true);
    Ok(hwnd)
}

/// Everything the render thread owns. Built on the render thread (the EGL context is
/// thread-affine), so it never crosses threads after construction.
struct RenderState {
    egl: &'static Egl,
    display: egl::Display,
    surface: egl::Surface,
    render_ctx: RenderContext<'static>,
    hwnd: HWND,
}

impl RenderState {
    /// Draw the current mpv frame and present. The framebuffer is sized from the live
    /// client rect, so window resizes are handled for free.
    fn render(&self) {
        let mut rect = RECT::default();
        if unsafe { GetClientRect(self.hwnd, &mut rect) }.is_err() {
            return;
        }
        let width = (rect.right - rect.left).max(1);
        let height = (rect.bottom - rect.top).max(1);
        // fbo 0 is ANGLE's backbuffer; flip because GL is Y-up but video is Y-down.
        if let Err(e) = self.render_ctx.render::<()>(0, width, height, true) {
            tracing::error!("mpv render failed: {e}");
            return;
        }
        // Blocks on vsync — fine, this is the render thread, not the UI thread.
        let _ = self.egl.swap_buffers(self.display, self.surface);
    }
}

/// Bring up ANGLE EGL bound to the child HWND and wire mpv's render context to it. Runs
/// on the render thread, where the context is made current and stays current.
fn init_render(
    hwnd: HWND,
    mpv: &'static Mpv,
    signal: &Arc<RenderSignal>,
) -> Result<RenderState, AppError> {
    let egl: &'static Egl = {
        let instance = unsafe {
            egl::DynamicInstance::<egl::EGL1_4>::load_required_from_filename("libEGL.dll")
        }
        .map_err(|e| err(&format!("load libEGL.dll: {e}")))?;
        Box::leak(Box::new(instance))
    };
    let display = unsafe { egl.get_display(egl::DEFAULT_DISPLAY) }
        .ok_or_else(|| err("eglGetDisplay failed"))?;
    egl.initialize(display)
        .map_err(|e| err(&format!("eglInitialize: {e}")))?;
    egl.bind_api(egl::OPENGL_ES_API)
        .map_err(|e| err(&format!("eglBindAPI: {e}")))?;

    // ALPHA_SIZE 0: an opaque swapchain. With an alpha channel, DWM composites the
    // video using mpv's framebuffer alpha (which is not opaque), so on a transparent
    // window the video shows through to the desktop. No alpha => the video paints solid.
    let config_attribs = [
        egl::SURFACE_TYPE,
        egl::WINDOW_BIT,
        egl::RENDERABLE_TYPE,
        egl::OPENGL_ES2_BIT,
        egl::RED_SIZE,
        8,
        egl::GREEN_SIZE,
        8,
        egl::BLUE_SIZE,
        8,
        egl::ALPHA_SIZE,
        0,
        egl::NONE,
    ];
    let config = egl
        .choose_first_config(display, &config_attribs)
        .map_err(|e| err(&format!("eglChooseConfig: {e}")))?
        .ok_or_else(|| err("no matching EGL config"))?;

    let surface = unsafe {
        egl.create_window_surface(display, config, hwnd.0 as egl::NativeWindowType, None)
    }
    .map_err(|e| err(&format!("eglCreateWindowSurface: {e}")))?;

    // Request an ES3 context (ANGLE grants it on ES2-renderable configs); mpv's GL
    // renderer uses the extra features when present.
    let context_attribs = [egl::CONTEXT_CLIENT_VERSION, 3, egl::NONE];
    let context = egl
        .create_context(display, config, None, &context_attribs)
        .map_err(|e| err(&format!("eglCreateContext: {e}")))?;

    egl.make_current(display, Some(surface), Some(surface), Some(context))
        .map_err(|e| err(&format!("eglMakeCurrent: {e}")))?;

    // The render context creation calls `get_proc_address`, so the context must be
    // current on this thread first.
    let mut render_ctx = mpv
        .create_render_context(vec![
            RenderParam::ApiType(RenderParamApiType::OpenGl),
            RenderParam::InitParams(OpenGLInitParams {
                get_proc_address,
                ctx: egl,
            }),
        ])
        .map_err(|e| err(&format!("render context creation failed: {e}")))?;

    // mpv signals a new frame on an arbitrary thread; just wake the render thread.
    let signal = Arc::clone(signal);
    render_ctx.set_update_callback(move || signal.notify());

    Ok(RenderState {
        egl,
        display,
        surface,
        render_ctx,
        hwnd,
    })
}

/// Render thread entry point: init EGL/mpv, report the result, then render on each frame.
fn render_thread(
    hwnd_val: isize,
    mpv: &'static Mpv,
    signal: Arc<RenderSignal>,
    ready: mpsc::Sender<Result<(), AppError>>,
) {
    let hwnd = HWND(hwnd_val as *mut c_void);
    let state = match init_render(hwnd, mpv, &signal) {
        Ok(state) => {
            let _ = ready.send(Ok(()));
            state
        }
        Err(e) => {
            let _ = ready.send(Err(e));
            return;
        }
    };

    loop {
        signal.wait();
        state.render();
    }
}

/// Build the child window, spawn the render thread, and wire mpv's render context to an
/// ANGLE GL surface. Must run on the main thread (the Tauri `setup` closure does); the
/// GL work itself happens on the spawned render thread.
pub fn attach(window: &tauri::WebviewWindow, mpv: &'static Mpv) -> Result<(), AppError> {
    let _span = tracing::info_span!("video_surface_attach").entered();

    let parent = parent_hwnd(window)?;
    let hinstance: HINSTANCE = {
        let module = unsafe { GetModuleHandleW(None) }
            .map_err(|e| err(&format!("GetModuleHandleW: {e}")))?;
        HINSTANCE(module.0)
    };
    let class = register_class(hinstance);
    let hwnd = create_video_window(parent, class, hinstance)?;

    // Keep the video window glued behind the Tauri window (move/resize/minimize).
    // Cross-thread SetWindowPos/ShowWindow from this subclass is fine.
    unsafe {
        let _ = SetWindowSubclass(parent, Some(owner_subclass), SUBCLASS_ID, hwnd.0 as usize);
    }

    // Hand GL work to a dedicated thread; the UI thread must stay free for the message
    // pump. `init_render` reports success/failure back so we keep clean error handling.
    let signal = Arc::new(RenderSignal::new());
    let thread_signal = Arc::clone(&signal);
    let hwnd_val = hwnd.0 as isize;
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("mpv-render".to_string())
        .spawn(move || render_thread(hwnd_val, mpv, thread_signal, tx))
        .map_err(|e| err(&format!("spawn render thread: {e}")))?;

    match rx.recv() {
        Ok(Ok(())) => {
            tracing::info!("video surface attached");
            Ok(())
        }
        Ok(Err(e)) => Err(e),
        Err(e) => Err(err(&format!("render thread init: {e}"))),
    }
}
