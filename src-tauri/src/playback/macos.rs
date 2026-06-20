//! macOS native video surface for the libmpv render API.
//!
//! We place an `NSOpenGLView` inside the Tauri window, *below* the transparent
//! `WKWebView`, so video composites underneath the Dioxus overlay. mpv draws into
//! this view through its OpenGL render context. All GL work happens on the main
//! thread, so there is no cross-thread context juggling or locking: mpv's update
//! callback (which may fire on any thread) only pushes a render onto the main
//! dispatch queue.
//!
//! Apple deprecated the OpenGL APIs in favour of Metal, but they remain fully
//! functional and are the only surface type libmpv's render API exposes, so we
//! deliberately opt out of the deprecation lint for this module.
#![allow(deprecated)]

use std::ffi::{c_void, CString};
use std::ptr::NonNull;

use cathode_core::error::AppError;
use dispatch2::DispatchQueue;
use libmpv2::render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType};
use libmpv2::Mpv;
use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSOpenGLContext, NSOpenGLPFAAccelerated, NSOpenGLPFAAlphaSize,
    NSOpenGLPFAColorSize, NSOpenGLPFADoubleBuffer, NSOpenGLPFAOpenGLProfile, NSOpenGLPixelFormat,
    NSOpenGLProfileVersionLegacy, NSOpenGLView, NSView, NSWindow, NSWindowOrderingMode,
};

// Resolved from the OpenGL framework, which `build.rs` links on macOS.
#[allow(non_snake_case)]
unsafe extern "C" {
    fn glClearColor(red: f32, green: f32, blue: f32, alpha: f32);
    fn glClear(mask: u32);
}
const GL_COLOR_BUFFER_BIT: u32 = 0x0000_4000;

fn err(context: &str) -> AppError {
    AppError {
        code: "playback".to_string(),
        message: format!("video surface: {context}"),
    }
}

/// libmpv resolves the GL entry points it needs through this callback. On macOS
/// every symbol lives in the OpenGL framework (linked by `build.rs`), so a plain
/// `dlsym` against the global namespace finds them all.
fn get_proc_address(_ctx: &(), name: &str) -> *mut c_void {
    let Ok(symbol) = CString::new(name) else {
        return std::ptr::null_mut();
    };
    unsafe { libc::dlsym(libc::RTLD_DEFAULT, symbol.as_ptr()) }
}

/// Owns the native GL view, its context, and the mpv render context. Created once
/// on the main thread and leaked for the lifetime of the app, so the raw pointer
/// captured by the update callback stays valid.
pub struct MacSurface {
    gl_view: Retained<NSOpenGLView>,
    context: Retained<NSOpenGLContext>,
    render_ctx: RenderContext<'static>,
}

impl MacSurface {
    /// Draw the current mpv frame into the view. Must run on the main thread with
    /// the GL context current.
    fn render(&self) {
        self.context.makeCurrentContext();
        // mpv renders into the GL framebuffer, which is sized in physical (backing)
        // pixels — 2x on a Retina display. Convert the view bounds to backing pixels
        // so video fills the whole surface instead of a 1x corner of it. Re-read each
        // frame so window resizes (and display changes) are handled for free.
        let backing = self
            .gl_view
            .convertSizeToBacking(self.gl_view.bounds().size);
        let width = backing.width as i32;
        let height = backing.height as i32;
        if let Err(e) = self.render_ctx.render::<()>(0, width, height, true) {
            tracing::error!("mpv render failed: {e}");
        }
        self.context.flushBuffer();
    }

    /// Clear the surface to opaque black (used before any frame is available).
    fn clear(&self) {
        self.context.makeCurrentContext();
        unsafe {
            glClearColor(0.0, 0.0, 0.0, 1.0);
            glClear(GL_COLOR_BUFFER_BIT);
        }
        self.context.flushBuffer();
    }
}

/// Build the GL view, insert it below the webview, wire mpv's render context, and
/// leak it all for the app's lifetime. Must run on the main thread (the Tauri
/// `setup` closure does).
pub fn attach(window: &tauri::WebviewWindow, mpv: &'static Mpv) -> Result<(), AppError> {
    let _span = tracing::info_span!("video_surface_attach").entered();

    let mtm = MainThreadMarker::new().ok_or_else(|| err("not on main thread"))?;
    let ns_window = window
        .ns_window()
        .map_err(|_| err("ns_window unavailable"))?;
    let ns_view = window.ns_view().map_err(|_| err("ns_view unavailable"))?;

    // Tauri hands us raw pointers to the NSWindow and the WKWebView's NSView; they
    // outlive the app window, so borrowing them is sound.
    let nswindow: &NSWindow = unsafe { &*ns_window.cast::<NSWindow>() };
    let webview: &NSView = unsafe { &*ns_view.cast::<NSView>() };
    let content_view = nswindow
        .contentView()
        .ok_or_else(|| err("no contentView"))?;

    // Double-buffered, accelerated, legacy profile (mpv's GL backend is happy with
    // it and it is the simplest to bring up). Array is null-terminated.
    let attrs: [u32; 9] = [
        NSOpenGLPFADoubleBuffer,
        NSOpenGLPFAAccelerated,
        NSOpenGLPFAColorSize,
        24,
        NSOpenGLPFAAlphaSize,
        8,
        NSOpenGLPFAOpenGLProfile,
        NSOpenGLProfileVersionLegacy,
        0,
    ];
    let pixel_format = unsafe {
        NSOpenGLPixelFormat::initWithAttributes(
            mtm.alloc(),
            NonNull::new(attrs.as_ptr() as *mut u32).unwrap(),
        )
    }
    .ok_or_else(|| err("pixel format creation failed"))?;

    let frame = content_view.bounds();
    let gl_view = NSOpenGLView::initWithFrame_pixelFormat(mtm.alloc(), frame, Some(&pixel_format))
        .ok_or_else(|| err("NSOpenGLView creation failed"))?;
    gl_view.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
    );
    // Render at the display's native resolution; paired with the backing-pixel
    // sizing in `render`, this keeps video sharp and full-window on Retina.
    gl_view.setWantsBestResolutionOpenGLSurface(true);

    // Insert below the webview so the transparent UI composites on top of video.
    content_view.addSubview_positioned_relativeTo(
        &gl_view,
        NSWindowOrderingMode::Below,
        Some(webview),
    );

    let context = gl_view
        .openGLContext()
        .ok_or_else(|| err("view has no GL context"))?;

    // The render context creation calls `get_proc_address`, so the GL context must
    // be current on this (main) thread first.
    context.makeCurrentContext();
    let render_ctx = mpv
        .create_render_context(vec![
            RenderParam::ApiType(RenderParamApiType::OpenGl),
            RenderParam::InitParams(OpenGLInitParams {
                get_proc_address,
                ctx: (),
            }),
        ])
        .map_err(|e| err(&format!("render context creation failed: {e}")))?;

    let surface: &'static mut MacSurface = Box::leak(Box::new(MacSurface {
        gl_view,
        context,
        render_ctx,
    }));
    surface.clear();

    // mpv signals a new frame via the update callback (on an arbitrary thread); we
    // bounce the actual render onto the main thread, where the GL context lives.
    let ptr = surface as *const MacSurface as usize;
    surface.render_ctx.set_update_callback(move || {
        DispatchQueue::main().exec_async(move || {
            let s = unsafe { &*(ptr as *const MacSurface) };
            s.render();
        });
    });

    tracing::info!("video surface attached");
    Ok(())
}
