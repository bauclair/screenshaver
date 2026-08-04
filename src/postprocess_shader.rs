use crate::render_dithering::{
    DitheringLevel,
    DitheringRenderer,
};
use crate::render_fxaa::FxaaRenderer;
use crate::render_passthrough::PassthroughRenderer;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PostprocessMethod {
    Passthrough,
    Fxaa,
}

impl PostprocessMethod {
    pub(crate) fn name(
        self,
    ) -> &'static str {
        match self {
            Self::Passthrough => "passthrough",
            Self::Fxaa => "FXAA",
        }
    }
}

struct RenderTarget {
    framebuffer: u32,
    texture: u32,
}

impl RenderTarget {
    fn new(
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        let (
            framebuffer,
            texture,
        ) =
            create_render_target(
                width,
                height,
            )?;

        Ok(
            Self {
                framebuffer,
                texture,
            }
        )
    }

    fn bind(
        &self,
        width: u32,
        height: u32,
    ) {
        unsafe {
            gl::BindFramebuffer(
                gl::FRAMEBUFFER,
                self.framebuffer,
            );

            gl::Viewport(
                0,
                0,
                width as i32,
                height as i32,
            );
        }
    }
}

impl Drop for RenderTarget {
    fn drop(
        &mut self,
    ) {
        unsafe {
            delete_render_target(
                self.framebuffer,
                self.texture,
            );
        }

        self.framebuffer =
            0;

        self.texture =
            0;
    }
}

/// Owns the scene and scratch render targets and executes the resolved
/// post-processing plan.
///
/// The caller renders the scene after `bind_scene_target()`. `present_scene()`
/// then runs the selected anti-aliasing presentation method, followed by
/// optional dithering, before returning framebuffer zero to the caller for
/// crisp overlay rendering.
pub(crate) struct PostprocessPipeline {
    scene_target: RenderTarget,
    scratch_target: RenderTarget,
    width: u32,
    height: u32,
    passthrough: PassthroughRenderer,
    fxaa: FxaaRenderer,
    dithering: DitheringRenderer,
    method: PostprocessMethod,
    dithering_level: DitheringLevel,
}

impl PostprocessPipeline {
    pub(crate) fn new(
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        validate_dimensions(
            width,
            height,
        )?;

        let passthrough =
            PassthroughRenderer::new()?;

        let fxaa =
            FxaaRenderer::new()?;

        let dithering =
            DitheringRenderer::new()?;

        let scene_target =
            RenderTarget::new(
                width,
                height,
            )?;

        let scratch_target =
            RenderTarget::new(
                width,
                height,
            )?;

        Ok(
            Self {
                scene_target,
                scratch_target,
                width,
                height,
                passthrough,
                fxaa,
                dithering,
                method:
                    PostprocessMethod::Fxaa,
                dithering_level:
                    DitheringLevel::Subtle,
            }
        )
    }

    /// Binds the off-screen scene target. The caller remains responsible for
    /// clearing it and drawing the active scene shader.
    pub(crate) fn bind_scene_target(
        &self,
    ) {
        self.scene_target.bind(
            self.width,
            self.height,
        );
    }

    /// Executes the current post-processing plan and presents it to
    /// framebuffer zero.
    ///
    /// Draw subtitle, FPS-warning, and future editor overlays after this call
    /// so they remain crisp and are not affected by post-processing.
    pub(crate) fn present_scene(
        &self,
    ) {
        prepare_fullscreen_pass();

        if self.dithering_level.is_enabled() {
            self.scratch_target.bind(
                self.width,
                self.height,
            );

            self.render_primary_pass(
                self.scene_target.texture
            );

            bind_default_framebuffer(
                self.width,
                self.height,
            );

            self.dithering.render(
                self.scratch_target.texture,
                self.dithering_level,
            );
        } else {
            bind_default_framebuffer(
                self.width,
                self.height,
            );

            self.render_primary_pass(
                self.scene_target.texture
            );
        }
    }

    fn render_primary_pass(
        &self,
        input_texture: u32,
    ) {
        match self.method {
            PostprocessMethod::Passthrough => {
                self.passthrough.render(
                    input_texture
                );
            }

            PostprocessMethod::Fxaa => {
                self.fxaa.render(
                    input_texture,
                    self.width,
                    self.height,
                );
            }
        }
    }



    pub(crate) fn set_profile(
        &mut self,
        profile:
            crate::load_config::PostprocessProfile,
    ) {

        self.method =
            match profile.anti_aliasing {
                crate::render_fxaa::AntiAliasingMethod::Off => {
                    PostprocessMethod::Passthrough
                }

                crate::render_fxaa::AntiAliasingMethod::Fxaa => {
                    PostprocessMethod::Fxaa
                }
            };

        self.dithering_level =
            profile.dithering;
    }


    #[allow(dead_code)]
    pub(crate) fn method(
        &self,
    ) -> PostprocessMethod {
        self.method
    }

    #[allow(dead_code)]
    pub(crate) fn dithering_level(
        &self,
    ) -> DitheringLevel {
        self.dithering_level
    }

    /// Recreates both size-dependent render targets while retaining the
    /// compiled post-processing programs and full-screen geometry.
    pub(crate) fn resize(
        &mut self,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        validate_dimensions(
            width,
            height,
        )?;

        if self.width == width
            && self.height == height
        {
            return Ok(());
        }

        // Allocate both replacements before releasing either working target.
        // If allocation fails, the existing pipeline remains usable.
        let replacement_scene =
            RenderTarget::new(
                width,
                height,
            )?;

        let replacement_scratch =
            RenderTarget::new(
                width,
                height,
            )?;

        self.scene_target =
            replacement_scene;

        self.scratch_target =
            replacement_scratch;

        self.width =
            width;

        self.height =
            height;

        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) fn dimensions(
        &self,
    ) -> (u32, u32) {
        (
            self.width,
            self.height,
        )
    }

    #[allow(dead_code)]
    pub(crate) fn scene_texture(
        &self,
    ) -> u32 {
        self.scene_target.texture
    }
}

fn prepare_fullscreen_pass(
) {
    unsafe {
        gl::ColorMask(
            gl::TRUE,
            gl::TRUE,
            gl::TRUE,
            gl::TRUE,
        );

        gl::Disable(
            gl::BLEND
        );
    }
}

fn bind_default_framebuffer(
    width: u32,
    height: u32,
) {
    unsafe {
        gl::BindFramebuffer(
            gl::FRAMEBUFFER,
            0,
        );

        gl::Viewport(
            0,
            0,
            width as i32,
            height as i32,
        );
    }
}

fn create_render_target(
    width: u32,
    height: u32,
) -> Result<(u32, u32), String> {
    validate_dimensions(
        width,
        height,
    )?;

    let mut framebuffer =
        0_u32;

    let mut texture =
        0_u32;

    unsafe {
        gl::GenFramebuffers(
            1,
            &mut framebuffer,
        );

        gl::GenTextures(
            1,
            &mut texture,
        );

        if framebuffer == 0
            || texture == 0
        {
            delete_render_target(
                framebuffer,
                texture,
            );

            return Err(
                "OpenGL failed to allocate post-processing framebuffer resources"
                    .to_string()
            );
        }

        gl::BindTexture(
            gl::TEXTURE_2D,
            texture,
        );

        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_MIN_FILTER,
            gl::LINEAR as i32,
        );

        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_MAG_FILTER,
            gl::LINEAR as i32,
        );

        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_WRAP_S,
            gl::CLAMP_TO_EDGE as i32,
        );

        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_WRAP_T,
            gl::CLAMP_TO_EDGE as i32,
        );

        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA8 as i32,
            width as i32,
            height as i32,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            std::ptr::null(),
        );

        gl::BindFramebuffer(
            gl::FRAMEBUFFER,
            framebuffer,
        );

        gl::FramebufferTexture2D(
            gl::FRAMEBUFFER,
            gl::COLOR_ATTACHMENT0,
            gl::TEXTURE_2D,
            texture,
            0,
        );

        let draw_buffers =
            [
                gl::COLOR_ATTACHMENT0
            ];

        gl::DrawBuffers(
            draw_buffers.len() as i32,
            draw_buffers.as_ptr(),
        );

        let status =
            gl::CheckFramebufferStatus(
                gl::FRAMEBUFFER
            );

        gl::BindFramebuffer(
            gl::FRAMEBUFFER,
            0,
        );

        gl::BindTexture(
            gl::TEXTURE_2D,
            0,
        );

        if status
            != gl::FRAMEBUFFER_COMPLETE
        {
            delete_render_target(
                framebuffer,
                texture,
            );

            return Err(
                format!(
                    "Post-processing framebuffer is incomplete (OpenGL status 0x{status:04X})"
                )
            );
        }
    }

    Ok(
        (
            framebuffer,
            texture,
        )
    )
}

unsafe fn delete_render_target(
    framebuffer: u32,
    texture: u32,
) {
    if framebuffer != 0 {
        gl::DeleteFramebuffers(
            1,
            &framebuffer,
        );
    }

    if texture != 0 {
        gl::DeleteTextures(
            1,
            &texture,
        );
    }
}

fn validate_dimensions(
    width: u32,
    height: u32,
) -> Result<(), String> {
    if width == 0
        || height == 0
    {
        return Err(
            format!(
                "Post-processing dimensions must be nonzero, received {}x{}",
                width,
                height,
            )
        );
    }

    if width > i32::MAX as u32
        || height > i32::MAX as u32
    {
        return Err(
            format!(
                "Post-processing dimensions exceed OpenGL limits: {}x{}",
                width,
                height,
            )
        );
    }

    Ok(())
}

