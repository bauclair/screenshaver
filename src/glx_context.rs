// glx_context_20260730_fbconfig_refactor_v1.rs
//
// Native GLX framebuffer configuration and context lifecycle.
//
// Responsibilities:
//
// ✓ Select a modern GLX framebuffer configuration.
// ✓ Obtain the matching X11 visual.
// ✓ Create and activate a GLX rendering context.
// ✓ Release and destroy the GLX context.
//
// Does NOT:
//
// ✗ Create or destroy the X11 window.
// ✗ Compile shaders.
// ✗ Render frames.
// ✗ Load textures.
// ✗ Process runtime events.

use std::os::raw::c_void;
use std::ptr;

use x11::glx;
use x11::xlib;

/// A GLX framebuffer configuration paired with its compatible X11 visual.
///
/// The visual is allocated by `glXGetVisualFromFBConfig()` and released when
/// this value is dropped.
pub struct GlxFramebufferConfig {
    fb_config: glx::GLXFBConfig,
    visual_info: *mut xlib::XVisualInfo,
}

impl GlxFramebufferConfig {
    /// Select a double-buffered RGBA framebuffer configuration suitable for
    /// an X11 window.
    pub fn choose(
        display: *mut xlib::Display,
        screen: i32,
    ) -> Result<Self, String> {
        if display.is_null() {
            return Err(
                "Cannot choose a GLX framebuffer configuration for a null X11 display."
                    .to_string(),
            );
        }

        println!("Choosing GLX framebuffer configuration...");

        let attributes = [
            glx::GLX_X_RENDERABLE,
            xlib::True,
            glx::GLX_DRAWABLE_TYPE,
            glx::GLX_WINDOW_BIT,
            glx::GLX_RENDER_TYPE,
            glx::GLX_RGBA_BIT,
            glx::GLX_X_VISUAL_TYPE,
            glx::GLX_TRUE_COLOR,
            glx::GLX_RED_SIZE,
            8,
            glx::GLX_GREEN_SIZE,
            8,
            glx::GLX_BLUE_SIZE,
            8,
            glx::GLX_ALPHA_SIZE,
            8,
            glx::GLX_DEPTH_SIZE,
            24,
            glx::GLX_STENCIL_SIZE,
            8,
            glx::GLX_DOUBLEBUFFER,
            xlib::True,
            0,
        ];

        let mut config_count = 0;

        let configs = unsafe {
            glx::glXChooseFBConfig(
                display,
                screen,
                attributes.as_ptr(),
                &mut config_count,
            )
        };

        if configs.is_null() || config_count <= 0 {
            return Err(
                "glXChooseFBConfig() did not return a compatible framebuffer configuration."
                    .to_string(),
            );
        }

        // Stage 4 deliberately selects the first compatible configuration.
        // A later refinement may score configurations when multisampling or
        // other optional framebuffer characteristics are introduced.
        let fb_config = unsafe { *configs };

        unsafe {
            xlib::XFree(configs as *mut c_void);
        }

        let visual_info =
            unsafe { glx::glXGetVisualFromFBConfig(display, fb_config) };

        if visual_info.is_null() {
            return Err(
                "glXGetVisualFromFBConfig() failed for the selected framebuffer configuration."
                    .to_string(),
            );
        }

        println!("Selected compatible GLX framebuffer configuration and X11 visual.");

        Ok(Self {
            fb_config,
            visual_info,
        })
    }

    /// Return the selected GLX framebuffer configuration.
    pub fn fb_config(&self) -> glx::GLXFBConfig {
        self.fb_config
    }

    /// Return the X11 visual information associated with the configuration.
    pub fn visual_info(&self) -> &xlib::XVisualInfo {
        // Construction guarantees that this pointer is non-null and it
        // remains valid until `self` is dropped.
        unsafe { &*self.visual_info }
    }
}

impl Drop for GlxFramebufferConfig {
    fn drop(&mut self) {
        if !self.visual_info.is_null() {
            unsafe {
                xlib::XFree(self.visual_info as *mut c_void);
            }

            self.visual_info = ptr::null_mut();
        }
    }
}

/// Owns one GLX rendering context.
///
/// The X11 display and window remain owned by their respective backend
/// modules. This type owns only the GLX context itself.
pub struct GlxContext {
    context: glx::GLXContext,
}

impl GlxContext {
    /// Create a direct-rendering RGBA context for the selected framebuffer
    /// configuration.
    pub fn create(
        display: *mut xlib::Display,
        config: &GlxFramebufferConfig,
    ) -> Result<Self, String> {
        if display.is_null() {
            return Err(
                "Cannot create a GLX context for a null X11 display.".to_string(),
            );
        }

        println!("Creating GLX context...");

        let context = unsafe {
            glx::glXCreateNewContext(
                display,
                config.fb_config(),
                glx::GLX_RGBA_TYPE,
                ptr::null_mut(),
                xlib::True,
            )
        };

        if context.is_null() {
            return Err("glXCreateNewContext() failed.".to_string());
        }

        println!("Created GLX context.");

        Ok(Self { context })
    }

    /// Make this context current for both drawing and reading from `window`.
    pub fn make_current(
        &self,
        display: *mut xlib::Display,
        window: xlib::Window,
    ) -> Result<(), String> {
        if display.is_null() {
            return Err(
                "Cannot activate a GLX context on a null X11 display.".to_string(),
            );
        }

        println!("Making GLX context current...");

        let succeeded = unsafe {
            glx::glXMakeContextCurrent(
                display,
                window,
                window,
                self.context,
            )
        };

        if succeeded == 0 {
            return Err("glXMakeContextCurrent() failed.".to_string());
        }

        println!("GLX context is current.");

        Ok(())
    }

    /// Detach every GLX context from the current thread.
    pub fn release_current(
        display: *mut xlib::Display,
    ) -> Result<(), String> {
        if display.is_null() {
            return Err(
                "Cannot release a GLX context from a null X11 display.".to_string(),
            );
        }

        let succeeded = unsafe {
            glx::glXMakeContextCurrent(
                display,
                0,
                0,
                ptr::null_mut(),
            )
        };

        if succeeded == 0 {
            return Err(
                "glXMakeContextCurrent() failed while releasing the current context."
                    .to_string(),
            );
        }

        Ok(())
    }

    /// Destroy the owned GLX context.
    ///
    /// The caller must release the context from the current thread first.
    pub fn destroy(mut self, display: *mut xlib::Display) {
        if display.is_null() || self.context.is_null() {
            return;
        }

        println!("Destroying GLX context...");

        unsafe {
            glx::glXDestroyContext(display, self.context);
        }

        self.context = ptr::null_mut();

        println!("Destroyed GLX context.");
    }
}

