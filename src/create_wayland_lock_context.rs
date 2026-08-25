use std::ffi::c_void;

use khronos_egl as egl;

use wayland_client::{
    protocol::wl_surface,
    Connection,
    Proxy,
};

use wayland_egl::WlEglSurface;


pub struct WaylandLockContext {
    egl: egl::Instance<egl::Static>,
    display: egl::Display,
    context: egl::Context,
    surface: egl::Surface,
    wayland_surface: WlEglSurface,
    width: i32,
    height: i32,
}


impl WaylandLockContext {
    pub fn new(
        connection: &Connection,
        wl_surface: &wl_surface::WlSurface,
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        if width == 0
            || height == 0
        {
            return Err(
                "Cannot create an EGL lock context with a zero-sized surface"
                    .to_string()
            );
        }


        if !wayland_egl::is_available() {
            return Err(
                "libwayland-egl is unavailable"
                    .to_string()
            );
        }


        let width_i32 =
            i32::try_from(width)
                .map_err(
                    |_| {
                        format!(
                            "Lock-surface width {} exceeds EGL limits",
                            width,
                        )
                    }
                )?;

        let height_i32 =
            i32::try_from(height)
                .map_err(
                    |_| {
                        format!(
                            "Lock-surface height {} exceeds EGL limits",
                            height,
                        )
                    }
                )?;


        let egl =
            egl::Instance::new(
                egl::Static
            );


        let display_ptr =
            connection
                .backend()
                .display_ptr()
                as *mut c_void;


        if display_ptr.is_null() {
            return Err(
                "Wayland display pointer is null; EGL interop is unavailable"
                    .to_string()
            );
        }


        let display =
            unsafe {
                egl.get_display(
                    display_ptr
                )
            }
            .ok_or_else(
                || {
                    "eglGetDisplay returned EGL_NO_DISPLAY"
                        .to_string()
                }
            )?;


        let (
            egl_major,
            egl_minor,
        ) =
            egl.initialize(
                display
            )
            .map_err(
                |error| {
                    format!(
                        "eglInitialize failed: {:?}",
                        error,
                    )
                }
            )?;


        println!(
            "[LOCK TEST] EGL initialized: {}.{}",
            egl_major,
            egl_minor,
        );


        egl.bind_api(
            egl::OPENGL_API
        )
        .map_err(
            |error| {
                format!(
                    "eglBindAPI(EGL_OPENGL_API) failed: {:?}",
                    error,
                )
            }
        )?;


        let config_attributes =
            [
                egl::SURFACE_TYPE,
                egl::WINDOW_BIT,

                egl::RENDERABLE_TYPE,
                egl::OPENGL_BIT,

                egl::RED_SIZE,
                8,

                egl::GREEN_SIZE,
                8,

                egl::BLUE_SIZE,
                8,

                egl::ALPHA_SIZE,
                8,

                egl::NONE,
            ];


        let config =
            egl.choose_first_config(
                display,
                &config_attributes,
            )
            .map_err(
                |error| {
                    format!(
                        "eglChooseConfig failed: {:?}",
                        error,
                    )
                }
            )?
            .ok_or_else(
                || {
                    "No EGL window configuration supports desktop OpenGL"
                        .to_string()
                }
            )?;


        let context_attributes =
            [
                egl::CONTEXT_MAJOR_VERSION,
                3,

                egl::CONTEXT_MINOR_VERSION,
                3,

                egl::CONTEXT_OPENGL_PROFILE_MASK,
                egl::CONTEXT_OPENGL_CORE_PROFILE_BIT,

                egl::NONE,
            ];


        let context =
            egl.create_context(
                display,
                config,
                None,
                &context_attributes,
            )
            .map_err(
                |error| {
                    format!(
                        "eglCreateContext(OpenGL 3.3 core) failed: {:?}",
                        error,
                    )
                }
            )?;


        let wayland_surface =
            WlEglSurface::new(
                wl_surface.id(),
                width_i32,
                height_i32,
            )
            .map_err(
                |error| {
                    let _ =
                        egl.destroy_context(
                            display,
                            context,
                        );

                    format!(
                        "Unable to create wl_egl_window: {}",
                        error,
                    )
                }
            )?;


        let surface =
            match unsafe {
                egl.create_window_surface(
                    display,
                    config,
                    wayland_surface.ptr()
                        as *mut c_void,
                    None,
                )
            } {
                Ok(surface) => {
                    surface
                }

                Err(error) => {
                    let _ =
                        egl.destroy_context(
                            display,
                            context,
                        );

                    return Err(
                        format!(
                            "eglCreateWindowSurface failed: {:?}",
                            error,
                        )
                    );
                }
            };


        if let Err(error) =
            egl.make_current(
                display,
                Some(surface),
                Some(surface),
                Some(context),
            )
        {
            let _ =
                egl.destroy_surface(
                    display,
                    surface,
                );

            let _ =
                egl.destroy_context(
                    display,
                    context,
                );

            return Err(
                format!(
                    "eglMakeCurrent failed: {:?}",
                    error,
                )
            );
        }


        let _ =
            egl.swap_interval(
                display,
                1,
            );


        gl::load_with(
            |name| {
                egl.get_proc_address(
                    name
                )
                .map(
                    |function| {
                        function as *const ()
                            as *const c_void
                    }
                )
                .unwrap_or(
                    std::ptr::null()
                )
            }
        );


        unsafe {
            gl::Viewport(
                0,
                0,
                width_i32,
                height_i32,
            );
        }


        Ok(
            Self {
                egl,
                display,
                context,
                surface,
                wayland_surface,
                width: width_i32,
                height: height_i32,
            }
        )
    }


    pub fn make_current(
        &self,
    ) -> Result<(), String> {
        self.egl
            .make_current(
                self.display,
                Some(self.surface),
                Some(self.surface),
                Some(self.context),
            )
            .map_err(
                |error| {
                    format!(
                        "eglMakeCurrent failed: {:?}",
                        error,
                    )
                }
            )
    }


    pub fn swap_buffers(
        &self,
    ) -> Result<(), String> {
        self.egl
            .swap_buffers(
                self.display,
                self.surface,
            )
            .map_err(
                |error| {
                    format!(
                        "eglSwapBuffers failed: {:?}",
                        error,
                    )
                }
            )
    }


    pub fn render_test_frame(
        &self,
        phase: f32,
    ) -> Result<(), String> {
        self.make_current()?;


        let red =
            0.12
                + 0.38
                    * (
                        phase * 1.7
                    )
                    .sin()
                    .abs();

        let green =
            0.10
                + 0.42
                    * (
                        phase * 1.1
                            + 1.0
                    )
                    .sin()
                    .abs();

        let blue =
            0.18
                + 0.52
                    * (
                        phase * 0.8
                            + 2.0
                    )
                    .sin()
                    .abs();


        unsafe {
            gl::Viewport(
                0,
                0,
                self.width,
                self.height,
            );

            gl::ClearColor(
                red,
                green,
                blue,
                1.0,
            );

            gl::Clear(
                gl::COLOR_BUFFER_BIT
            );
        }


        self.swap_buffers()
    }
}


impl Drop for WaylandLockContext {
    fn drop(
        &mut self,
    ) {
        let _ =
            self.egl
                .make_current(
                    self.display,
                    None,
                    None,
                    None,
                );


        let _ =
            self.egl
                .destroy_surface(
                    self.display,
                    self.surface,
                );


        let _ =
            self.egl
                .destroy_context(
                    self.display,
                    self.context,
                );


        // Keep the wl_egl_window alive until after the EGLSurface has been
        // destroyed.  Its own Drop implementation then releases it safely.
        let _ =
            &self.wayland_surface;
    }
}
