use crate::render_passthrough::PassthroughRenderer;

/// Owns the off-screen scene framebuffer and the presentation pass.
///
/// Milestone 1 supports only passthrough presentation. Future post-processing
/// methods such as FXAA and SMAA can be added without changing the scene
/// renderer's framebuffer ownership.
pub(crate) struct PostprocessPipeline {
    framebuffer: u32,
    scene_texture: u32,
    width: u32,
    height: u32,
    passthrough: PassthroughRenderer,
}

impl PostprocessPipeline {
    pub(crate) fn new(
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        validate_dimensions(width, height)?;

        let passthrough = PassthroughRenderer::new()?;
        let (framebuffer, scene_texture) = create_scene_target(width, height)?;

        Ok(Self {
            framebuffer,
            scene_texture,
            width,
            height,
            passthrough,
        })
    }

    /// Binds the off-screen scene framebuffer. The caller remains responsible
    /// for clearing it and drawing the active scene shader.
    pub(crate) fn bind_scene_target(&self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.framebuffer);
            gl::Viewport(0, 0, self.width as i32, self.height as i32);
        }
    }

    /// Copies the completed scene texture to framebuffer zero.
    ///
    /// Draw subtitle, FPS-warning, and future editor overlays after this call
    /// so they remain crisp and are not affected by post-processing.
    pub(crate) fn present_scene(&self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::Viewport(0, 0, self.width as i32, self.height as i32);
            gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
            gl::Disable(gl::BLEND);
        }

        self.passthrough.render(self.scene_texture);
    }

    /// Recreates only the size-dependent scene target. The compiled
    /// passthrough program and full-screen geometry remain intact.
    pub(crate) fn resize(
        &mut self,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        validate_dimensions(width, height)?;

        if self.width == width && self.height == height {
            return Ok(());
        }

        let (replacement_framebuffer, replacement_texture) =
            create_scene_target(width, height)?;

        unsafe {
            delete_scene_target(self.framebuffer, self.scene_texture);
        }

        self.framebuffer = replacement_framebuffer;
        self.scene_texture = replacement_texture;
        self.width = width;
        self.height = height;

        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    #[allow(dead_code)]
    pub(crate) fn scene_texture(&self) -> u32 {
        self.scene_texture
    }
}

impl Drop for PostprocessPipeline {
    fn drop(&mut self) {
        unsafe {
            delete_scene_target(self.framebuffer, self.scene_texture);
        }

        self.framebuffer = 0;
        self.scene_texture = 0;
    }
}

fn create_scene_target(
    width: u32,
    height: u32,
) -> Result<(u32, u32), String> {
    validate_dimensions(width, height)?;

    let mut framebuffer = 0_u32;
    let mut texture = 0_u32;

    unsafe {
        gl::GenFramebuffers(1, &mut framebuffer);
        gl::GenTextures(1, &mut texture);

        if framebuffer == 0 || texture == 0 {
            delete_scene_target(framebuffer, texture);
            return Err(
                "OpenGL failed to allocate post-processing framebuffer resources"
                    .to_string(),
            );
        }

        gl::BindTexture(gl::TEXTURE_2D, texture);
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

        gl::BindFramebuffer(gl::FRAMEBUFFER, framebuffer);
        gl::FramebufferTexture2D(
            gl::FRAMEBUFFER,
            gl::COLOR_ATTACHMENT0,
            gl::TEXTURE_2D,
            texture,
            0,
        );

        let draw_buffers = [gl::COLOR_ATTACHMENT0];
        gl::DrawBuffers(draw_buffers.len() as i32, draw_buffers.as_ptr());

        let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);

        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        gl::BindTexture(gl::TEXTURE_2D, 0);

        if status != gl::FRAMEBUFFER_COMPLETE {
            delete_scene_target(framebuffer, texture);
            return Err(format!(
                "Post-processing framebuffer is incomplete (OpenGL status 0x{status:04X})"
            ));
        }
    }

    Ok((framebuffer, texture))
}

unsafe fn delete_scene_target(
    framebuffer: u32,
    texture: u32,
) {
    if framebuffer != 0 {
        gl::DeleteFramebuffers(1, &framebuffer);
    }

    if texture != 0 {
        gl::DeleteTextures(1, &texture);
    }
}

fn validate_dimensions(
    width: u32,
    height: u32,
) -> Result<(), String> {
    if width == 0 || height == 0 {
        return Err(format!(
            "Post-processing dimensions must be nonzero, received {}x{}",
            width, height,
        ));
    }

    if width > i32::MAX as u32 || height > i32::MAX as u32 {
        return Err(format!(
            "Post-processing dimensions exceed OpenGL limits: {}x{}",
            width, height,
        ));
    }

    Ok(())
}

