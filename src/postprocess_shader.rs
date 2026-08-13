use crate::render_dithering::{
    DitheringLevel,
    DitheringRenderer,
};
use crate::render_bloom::BloomRenderer;
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
    precision: crate::select_render_precision::RenderTargetPrecision,
}

impl RenderTarget {
    fn new(
        width: u32,
        height: u32,
        precision: crate::select_render_precision::RenderTargetPrecision,
    ) -> Result<Self, String> {
        let (
            framebuffer,
            texture,
        ) =
            create_render_target(
                width,
                height,
                precision,
            )?;

        Ok(
            Self {
                framebuffer,
                texture,
                precision,
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
/// then runs the selected presentation plan. Highlight Bloom currently uses
/// a diagnostic half-resolution extraction-and-blur path; normal rendering
/// continues through anti-aliasing and optional dithering before returning
/// framebuffer zero to the caller for crisp overlay rendering.
pub(crate) struct PostprocessPipeline {
    scene_target: RenderTarget,
    scratch_target: RenderTarget,
    bloom_target_a: RenderTarget,
    bloom_target_b: RenderTarget,
    output_width: u32,
    output_height: u32,
    scene_width: u32,
    scene_height: u32,
    bloom_width: u32,
    bloom_height: u32,
    render_scale: f32,
    precision_selection:
        crate::select_render_precision::RenderPrecisionSelection,
    passthrough: PassthroughRenderer,
    fxaa: FxaaRenderer,
    dithering: DitheringRenderer,
    bloom: BloomRenderer,
    method: PostprocessMethod,
    dithering_level: DitheringLevel,
    bloom_mode: crate::render_bloom::BloomMode,
    bloom_intensity: f32,
    bloom_threshold: f32,
}

impl PostprocessPipeline {
    pub(crate) fn new(
        output_width: u32,
        output_height: u32,
        profile:
            crate::load_config::PostprocessProfile,
    ) -> Result<Self, String> {
        validate_dimensions(
            output_width,
            output_height,
        )?;

        let render_scale =
            profile.render_scale;

        validate_render_scale(
            render_scale
        )?;

        let (
            scene_width,
            scene_height,
        ) =
            scaled_dimensions(
                output_width,
                output_height,
                render_scale,
            )?;


        let (
            bloom_width,
            bloom_height,
        ) =
            bloom_dimensions(
                output_width,
                output_height,
            )?;

        let passthrough =
            PassthroughRenderer::new()?;

        let fxaa =
            FxaaRenderer::new()?;

        let dithering =
            DitheringRenderer::new()?;

        let bloom =
            BloomRenderer::new()?;

        let requested_precision =
            profile.color_precision;

        let (
            scene_target,
            scratch_target,
            precision_selection,
        ) =
            select_render_targets(
                scene_width,
                scene_height,
                output_width,
                output_height,
                requested_precision,
            )?;


        let bloom_target_a =
            RenderTarget::new(
                bloom_width,
                bloom_height,
                precision_selection.selected,
            )?;

        let bloom_target_b =
            RenderTarget::new(
                bloom_width,
                bloom_height,
                precision_selection.selected,
            )?;

        let pipeline =
            Self {
                scene_target,
                scratch_target,
                bloom_target_a,
                bloom_target_b,
                output_width,
                output_height,
                scene_width,
                scene_height,
                bloom_width,
                bloom_height,
                render_scale,
                precision_selection,
                passthrough,
                fxaa,
                dithering,
                bloom,
                method:
                    method_for_profile(
                        profile
                    ),
                dithering_level:
                    profile.dithering,
                bloom_mode:
                    profile.bloom,
                bloom_intensity:
                    crate::render_bloom::validate_bloom_intensity(
                        profile.bloom_intensity
                    )?,
                bloom_threshold:
                    crate::render_bloom::validate_bloom_threshold(
                        profile.bloom_threshold
                    )?,
            };

        pipeline.log_precision_selection();
        pipeline.log_render_scale();

        Ok(
            pipeline
        )
    }

    /// Binds the off-screen scene target. The caller remains responsible for
    /// clearing it and drawing the active scene shader.
    pub(crate) fn bind_scene_target(
        &self,
    ) {
        self.scene_target.bind(
            self.scene_width,
            self.scene_height,
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

        // Checkpoint 4 diagnostic path: extract highlights into a
        // half-resolution working texture, blur horizontally, blur vertically,
        // then present only the final blurred Bloom texture.
        if matches!(
            self.bloom_mode,
            crate::render_bloom::BloomMode::Highlight
        ) {
            self.bloom_target_a.bind(
                self.bloom_width,
                self.bloom_height,
            );

            unsafe {
                gl::ClearColor(
                    0.0,
                    0.0,
                    0.0,
                    1.0,
                );

                gl::Clear(
                    gl::COLOR_BUFFER_BIT
                );
            }

            self.bloom.render_highlights(
                self.scene_target.texture,
                self.bloom_threshold,
            );

            self.bloom_target_b.bind(
                self.bloom_width,
                self.bloom_height,
            );

            self.bloom.render_blur(
                self.bloom_target_a.texture,
                1.0 / self.bloom_width as f32,
                0.0,
            );

            bind_default_framebuffer(
                self.output_width,
                self.output_height,
            );

            self.bloom.render_blur(
                self.bloom_target_b.texture,
                0.0,
                1.0 / self.bloom_height as f32,
            );

            return;
        }

        if self.dithering_level.is_enabled() {
            self.scratch_target.bind(
                self.output_width,
                self.output_height,
            );

            self.render_primary_pass(
                self.scene_target.texture
            );

            bind_default_framebuffer(
                self.output_width,
                self.output_height,
            );

            self.dithering.render(
                self.scratch_target.texture,
                self.dithering_level,
            );
        } else {
            bind_default_framebuffer(
                self.output_width,
                self.output_height,
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
                    self.scene_width,
                    self.scene_height,
                );
            }
        }
    }



    pub(crate) fn set_profile(
        &mut self,
        profile:
            crate::load_config::PostprocessProfile,
    ) -> Result<(), String> {

        validate_render_scale(
            profile.render_scale
        )?;

        let bloom_intensity =
            crate::render_bloom::validate_bloom_intensity(
                profile.bloom_intensity
            )?;

        let bloom_threshold =
            crate::render_bloom::validate_bloom_threshold(
                profile.bloom_threshold
            )?;


        let precision_changed =
            profile.color_precision
                != self.precision_selection.requested;

        let render_scale_changed =
            (profile.render_scale
                - self.render_scale)
                .abs()
                > f32::EPSILON;


        if precision_changed
            || render_scale_changed
        {
            let (
                scene_width,
                scene_height,
            ) =
                scaled_dimensions(
                    self.output_width,
                    self.output_height,
                    profile.render_scale,
                )?;


            let (
                scene_target,
                scratch_target,
                precision_selection,
            ) =
                select_render_targets(
                    scene_width,
                    scene_height,
                    self.output_width,
                    self.output_height,
                    profile.color_precision,
                )?;


            self.scene_target =
                scene_target;

            self.scratch_target =
                scratch_target;

            self.scene_width =
                scene_width;

            self.scene_height =
                scene_height;

            self.render_scale =
                profile.render_scale;

            self.precision_selection =
                precision_selection;

            self.bloom_target_a =
                RenderTarget::new(
                    self.bloom_width,
                    self.bloom_height,
                    self.precision_selection.selected,
                )?;

            self.bloom_target_b =
                RenderTarget::new(
                    self.bloom_width,
                    self.bloom_height,
                    self.precision_selection.selected,
                )?;


            if precision_changed {
                self.log_precision_selection();
            }


            if render_scale_changed {
                self.log_render_scale();
            }
        }


        self.method =
            method_for_profile(
                profile
            );

        self.dithering_level =
            profile.dithering;

        self.bloom_mode =
            profile.bloom;

        self.bloom_intensity =
            bloom_intensity;

        self.bloom_threshold =
            bloom_threshold;


        Ok(())
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

    #[allow(dead_code)]
    pub(crate) fn bloom_mode(
        &self,
    ) -> crate::render_bloom::BloomMode {
        self.bloom_mode
    }


    #[allow(dead_code)]
    pub(crate) fn bloom_intensity(
        &self,
    ) -> f32 {
        self.bloom_intensity
    }

    #[allow(dead_code)]
    pub(crate) fn bloom_threshold(
        &self,
    ) -> f32 {
        self.bloom_threshold
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

        if self.output_width == width
            && self.output_height == height
        {
            return Ok(());
        }

        let (
            scene_width,
            scene_height,
        ) =
            scaled_dimensions(
                width,
                height,
                self.render_scale,
            )?;


        let (
            bloom_width,
            bloom_height,
        ) =
            bloom_dimensions(
                width,
                height,
            )?;

        // Allocate both replacements before releasing either working target.
        // If allocation fails, the existing pipeline remains usable.
        let replacement_scene =
            RenderTarget::new(
                scene_width,
                scene_height,
                self.precision_selection.selected,
            )?;

        let replacement_scratch =
            RenderTarget::new(
                width,
                height,
                self.precision_selection.selected,
            )?;


        let replacement_bloom_a =
            RenderTarget::new(
                bloom_width,
                bloom_height,
                self.precision_selection.selected,
            )?;

        let replacement_bloom_b =
            RenderTarget::new(
                bloom_width,
                bloom_height,
                self.precision_selection.selected,
            )?;

        self.scene_target =
            replacement_scene;

        self.scratch_target =
            replacement_scratch;

        self.bloom_target_a =
            replacement_bloom_a;

        self.bloom_target_b =
            replacement_bloom_b;

        self.output_width =
            width;

        self.output_height =
            height;

        self.scene_width =
            scene_width;

        self.scene_height =
            scene_height;

        self.bloom_width =
            bloom_width;

        self.bloom_height =
            bloom_height;

        self.log_render_scale();

        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) fn precision_selection(
        &self,
    ) -> &crate::select_render_precision::RenderPrecisionSelection {
        &self.precision_selection
    }


    pub(crate) fn log_precision_selection(
        &self,
    ) {
        let selection =
            &self.precision_selection;

        let fallback =
            if selection.fell_back {
                "yes"
            } else {
                "no"
            };

        crate::logger::information(
            &crate::locate_paths::runtime_log_path(),
            &format!(
                "[POSTPROCESS] Requested precision: {}; selected: {} ({}); fallback: {}",
                selection.requested.name(),
                selection.selected.name(),
                selection.selected.internal_format_name(),
                fallback,
            ),
        );

        if let Some(reason) =
            selection.fallback_reason.as_deref()
        {
            crate::logger::information(
                &crate::locate_paths::runtime_log_path(),
                &format!(
                    "[POSTPROCESS] High-precision render targets were unavailable; standard precision was selected: {}",
                    reason,
                ),
            );
        }
    }


    #[allow(dead_code)]
    pub(crate) fn dimensions(
        &self,
    ) -> (u32, u32) {
        self.output_dimensions()
    }


    pub(crate) fn output_dimensions(
        &self,
    ) -> (u32, u32) {
        (
            self.output_width,
            self.output_height,
        )
    }


    pub(crate) fn scene_dimensions(
        &self,
    ) -> (u32, u32) {
        (
            self.scene_width,
            self.scene_height,
        )
    }


    #[allow(dead_code)]
    pub(crate) fn render_scale(
        &self,
    ) -> f32 {
        self.render_scale
    }


    pub(crate) fn log_render_scale(
        &self,
    ) {
        crate::logger::information(
            &crate::locate_paths::runtime_log_path(),
            &format!(
                "[POSTPROCESS] Render scale: {:.3}; scene: {}x{}; output: {}x{}",
                self.render_scale,
                self.scene_width,
                self.scene_height,
                self.output_width,
                self.output_height,
            ),
        );
    }


    #[allow(dead_code)]
    pub(crate) fn scene_texture(
        &self,
    ) -> u32 {
        self.scene_target.texture
    }
}

fn method_for_profile(
    profile:
        crate::load_config::PostprocessProfile,
) -> PostprocessMethod {

    match profile.anti_aliasing {
        crate::render_fxaa::AntiAliasingMethod::Off => {
            PostprocessMethod::Passthrough
        }

        crate::render_fxaa::AntiAliasingMethod::Fxaa => {
            PostprocessMethod::Fxaa
        }
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

fn create_render_targets(
    scene_width: u32,
    scene_height: u32,
    output_width: u32,
    output_height: u32,
    precision:
        crate::select_render_precision::RenderTargetPrecision,
) -> Result<
    (
        RenderTarget,
        RenderTarget,
    ),
    String,
> {
    let scene_target =
        RenderTarget::new(
            scene_width,
            scene_height,
            precision,
        )?;

    let scratch_target =
        RenderTarget::new(
            output_width,
            output_height,
            precision,
        )?;

    Ok(
        (
            scene_target,
            scratch_target,
        )
    )
}


fn select_render_targets(
    scene_width: u32,
    scene_height: u32,
    output_width: u32,
    output_height: u32,
    requested:
        crate::select_render_precision::ColorPrecisionPolicy,
) -> Result<
    (
        RenderTarget,
        RenderTarget,
        crate::select_render_precision::RenderPrecisionSelection,
    ),
    String,
> {
    use crate::select_render_precision::{
        ColorPrecisionPolicy,
        RenderPrecisionSelection,
        RenderTargetPrecision,
    };

    match requested {
        ColorPrecisionPolicy::Standard => {
            let (
                scene_target,
                scratch_target,
            ) =
                create_render_targets(
                    scene_width,
                    scene_height,
                    output_width,
                    output_height,
                    RenderTargetPrecision::Standard,
                )?;

            Ok(
                (
                    scene_target,
                    scratch_target,
                    RenderPrecisionSelection::direct(
                        requested,
                        RenderTargetPrecision::Standard,
                    ),
                )
            )
        }

        ColorPrecisionPolicy::High => {
            let (
                scene_target,
                scratch_target,
            ) =
                create_render_targets(
                    scene_width,
                    scene_height,
                    output_width,
                    output_height,
                    RenderTargetPrecision::High,
                )?;

            Ok(
                (
                    scene_target,
                    scratch_target,
                    RenderPrecisionSelection::direct(
                        requested,
                        RenderTargetPrecision::High,
                    ),
                )
            )
        }

        ColorPrecisionPolicy::Auto => {
            match create_render_targets(
                scene_width,
                scene_height,
                output_width,
                output_height,
                RenderTargetPrecision::High,
            ) {
                Ok(
                    (
                        scene_target,
                        scratch_target,
                    )
                ) => {
                    Ok(
                        (
                            scene_target,
                            scratch_target,
                            RenderPrecisionSelection::direct(
                                requested,
                                RenderTargetPrecision::High,
                            ),
                        )
                    )
                }

                Err(high_error) => {
                    let (
                        scene_target,
                        scratch_target,
                    ) =
                        create_render_targets(
                            scene_width,
                            scene_height,
                            output_width,
                            output_height,
                            RenderTargetPrecision::Standard,
                        )
                        .map_err(
                            |standard_error| {
                                format!(
                                    "Unable to create post-processing targets: high precision failed ({}); standard precision failed ({})",
                                    high_error,
                                    standard_error,
                                )
                            }
                        )?;

                    Ok(
                        (
                            scene_target,
                            scratch_target,
                            RenderPrecisionSelection::fallback(
                                requested,
                                RenderTargetPrecision::Standard,
                                high_error,
                            ),
                        )
                    )
                }
            }
        }
    }
}


fn create_render_target(
    width: u32,
    height: u32,
    precision: crate::select_render_precision::RenderTargetPrecision,
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
            precision.internal_format(),
            width as i32,
            height as i32,
            0,
            precision.external_format(),
            precision.pixel_type(),
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

fn validate_render_scale(
    render_scale: f32,
) -> Result<(), String> {
    if !render_scale.is_finite()
        || !(crate::define_constants::RENDER_SCALE_MIN
            ..=crate::define_constants::RENDER_SCALE_MAX)
            .contains(
                &render_scale
            )
    {
        return Err(
            format!(
                "Render scale {} is outside the supported range {:.2}-{:.2}",
                render_scale,
                crate::define_constants::RENDER_SCALE_MIN,
                crate::define_constants::RENDER_SCALE_MAX,
            )
        );
    }

    Ok(())
}


fn scaled_dimensions(
    output_width: u32,
    output_height: u32,
    render_scale: f32,
) -> Result<(u32, u32), String> {
    validate_dimensions(
        output_width,
        output_height,
    )?;

    validate_render_scale(
        render_scale
    )?;

    let scene_width =
        ((output_width as f64)
            * render_scale as f64)
            .round()
            .max(1.0);

    let scene_height =
        ((output_height as f64)
            * render_scale as f64)
            .round()
            .max(1.0);

    if scene_width
        > u32::MAX as f64
        || scene_height
            > u32::MAX as f64
    {
        return Err(
            format!(
                "Scaled render dimensions exceed the supported integer range: {:.0}x{:.0}",
                scene_width,
                scene_height,
            )
        );
    }

    Ok(
        (
            scene_width as u32,
            scene_height as u32,
        )
    )
}


fn bloom_dimensions(
    output_width: u32,
    output_height: u32,
) -> Result<(u32, u32), String> {

    validate_dimensions(
        output_width,
        output_height,
    )?;

    Ok(
        (
            (output_width / 2).max(1),
            (output_height / 2).max(1),
        )
    )
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

