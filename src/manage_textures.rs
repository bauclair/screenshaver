//! Procedural texture management for Screenshaver.
//!
//! This module bridges the CPU-side Procedural Texture Engine
//! and the OpenGL texture channels exposed to ShaderToy shaders.
//! It owns all GPU texture objects used by the frame renderer.

use std::ffi::CString;
use std::sync::atomic::{
    AtomicU64,
    Ordering,
};
use std::time::{
    SystemTime,
    UNIX_EPOCH,
};

use crate::generate_textures::{
    GeneratedTexture,
    TextureFamily,
};
use crate::load_config::{
    TextureOverride,
    TextureSelectionPolicy,
};
use crate::parse_texture_specification::{
    TextureSpecification,
};
use crate::palettes::Palette;
use crate::preprocess_shader::ShaderChannelUsage;


// ============================================================
// Random fallback selection
// ============================================================

/// Texture families available for random fallback selection.
const RANDOM_TEXTURE_FAMILIES: [TextureFamily; 8] = [

    TextureFamily::Marble,

    TextureFamily::Clouds,

    TextureFamily::Cellular,

    TextureFamily::Mesh,

    TextureFamily::Radial,

    TextureFamily::Noise,

    TextureFamily::Bricks,

    TextureFamily::Hexagons,
];


/// Primitive counts available for automatic random texture selection.
///
/// Powers of two provide broad visual variation without selecting every
/// possible count in the supported range.
const RANDOM_PRIMITIVE_COUNTS: [usize; 10] = [
    2,
    4,
    8,
    16,
    32,
    64,
    128,
    256,
    512,
    1024,
];


const CHANNEL_COUNT: usize =
    4;


/// Additional entropy so selections made within the same system
/// clock tick still receive different random streams.
static RANDOM_COUNTER: AtomicU64 =
    AtomicU64::new(
        0
    );


// ============================================================
// Temporary command-line preview selection
// ============================================================

#[derive(
    Debug,
    Clone,
    Copy,
)]
pub enum PreviewSelectionValue<T> {
    Random,
    Specific(T),
}


#[derive(
    Debug,
    Clone,
    Copy,
    Default,
)]
pub struct PreviewTextureSelection {

    pub texture:
        Option<
            PreviewSelectionValue<TextureSpecification>
        >,

    pub palette:
        Option<
            PreviewSelectionValue<Palette>
        >,
}


// ============================================================
// Texture request
// ============================================================

#[derive(
    Debug,
    Clone,
    Copy,
)]
struct TextureRequest {
    texture: TextureSpecification,
    palette: Palette,
    seed: u64,
}

fn resolve_texture_selection(
    shader_name: &str,
    policy: &TextureSelectionPolicy,
    preview_selection: PreviewTextureSelection,
) -> (
    TextureRequest,
    &'static str,
    &'static str,
) {

    let mut state =
        random_state();


    let shader_override =
        matching_override(
            shader_name,
            &policy.texture_overrides,
        );


    let (
        texture,
        texture_source,
    ) =
        match preview_selection.texture {

            Some(
                PreviewSelectionValue::Specific(
                    family
                )
            ) => {
                (
                    family,
                    "command line",
                )
            }

            Some(
                PreviewSelectionValue::Random
            ) => {
                (
                    random_texture_specification(
                        &mut state
                    ),
                    "command-line random",
                )
            }

            None => {
                if let Some(texture) =
                    shader_override
                        .and_then(
                            |texture_override| {
                                texture_override.shader_texture.clone()
                            }
                        )
                {
                    (
                        texture,
                        "shader override",
                    )
                } else if let Some(texture) =
                    policy.global_texture
                {
                    (
                        texture,
                        "global",
                    )
                } else {
                    (
                        random_texture_specification(
                            &mut state
                        ),
                        "random fallback",
                    )
                }
            }
        };


    let (
        palette,
        palette_source,
    ) =
        match preview_selection.palette {

            Some(
                PreviewSelectionValue::Specific(
                    palette
                )
            ) => {
                (
                    palette,
                    "command line",
                )
            }

            Some(
                PreviewSelectionValue::Random
            ) => {
                (
                    random_palette(
                        &mut state
                    ),
                    "command-line random",
                )
            }

            None => {
                if let Some(palette) =
                    shader_override
                        .and_then(
                            |texture_override| {
                                texture_override.shader_palette
                            }
                        )
                {
                    (
                        palette,
                        "shader override",
                    )
                } else if let Some(palette) =
                    policy.global_palette
                {
                    (
                        palette,
                        "global",
                    )
                } else {
                    (
                        random_palette(
                            &mut state
                        ),
                        "random fallback",
                    )
                }
            }
        };


    (
        TextureRequest {
            texture,
            palette,
            seed:
                splitmix64(
                    &mut state
                ),
        },
        texture_source,
        palette_source,
    )
}

fn random_texture_specification(
    state: &mut u64,
) -> TextureSpecification {

    let family =
        random_texture_family(
            state
        );


    let primitive_count =
        random_primitive_count(
            state
        );


    TextureSpecification {
        family,

        requested_primitive_count:
            primitive_count,

        count_was_explicit:
            false,
    }
}


fn random_texture_family(
    state: &mut u64,
) -> TextureFamily {

    let family_index =
        random_index(
            state,
            RANDOM_TEXTURE_FAMILIES.len(),
        );


    RANDOM_TEXTURE_FAMILIES[
        family_index
    ]
}


fn random_primitive_count(
    state: &mut u64,
) -> usize {

    let count_index =
        random_index(
            state,
            RANDOM_PRIMITIVE_COUNTS.len(),
        );


    RANDOM_PRIMITIVE_COUNTS[
        count_index
    ]
}


fn random_palette(
    state: &mut u64,
) -> Palette {

    let palette_index =
        random_index(
            state,
            Palette::ALL.len(),
        );


    Palette::ALL[
        palette_index
    ]
}


fn matching_override<'a>(
    shader_name: &str,
    overrides: &'a [TextureOverride],
) -> Option<
    &'a TextureOverride
> {

    overrides
        .iter()
        .find(
            |texture_override| {
                texture_override
                    .shader
                    .eq_ignore_ascii_case(
                        shader_name
                    )
            }
        )
}


fn random_state() -> u64 {

    let time_entropy =
        SystemTime::now()
            .duration_since(
                UNIX_EPOCH
            )
            .map(
                |duration| {
                    duration.as_nanos()
                        as u64
                }
            )
            .unwrap_or(
                0
            );


    let counter =
        RANDOM_COUNTER.fetch_add(
            1,
            Ordering::Relaxed,
        );


    let mut state =
        time_entropy
            ^ counter.rotate_left(
                23
            )
            ^ 0xA076_1D64_78BD_642F;


    splitmix64(
        &mut state
    )
}


fn random_index(
    state: &mut u64,
    length: usize,
) -> usize {

    debug_assert!(
        length > 0
    );


    (
        splitmix64(
            state
        )
            % length as u64
    ) as usize
}


fn splitmix64(
    state: &mut u64,
) -> u64 {

    *state =
        state.wrapping_add(
            0x9E37_79B9_7F4A_7C15
        );


    let mut value =
        *state;


    value =
        (
            value
                ^ (
                    value >> 30
                )
        )
        .wrapping_mul(
            0xBF58_476D_1CE4_E5B9
        );


    value =
        (
            value
                ^ (
                    value >> 27
                )
        )
        .wrapping_mul(
            0x94D0_49BB_1331_11EB
        );


    value
        ^ (
            value >> 31
        )
}



// ============================================================
// GPU texture representation
// ============================================================

#[derive(Debug)]
struct GpuTexture {
    id: u32,
    width: u32,
    height: u32,

    /// Complete texture request that produced this GPU texture.
    ///
    /// This is the authoritative identity of the uploaded texture and
    /// preserves the requested primitive count plus whether that count
    /// was explicitly supplied by the user.
    specification:
        TextureSpecification,

    palette: Palette,
    seed: u64,
}


// ============================================================
// Texture manager
// ============================================================

#[derive(Debug)]
pub struct TextureManager {
    texture: Option<GpuTexture>,
    active_channels: [bool; CHANNEL_COUNT],
    policy:
        TextureSelectionPolicy,
}


impl TextureManager {

    pub fn new(
        policy: TextureSelectionPolicy,
    ) -> Self {

        Self {
            texture:
                None,
            active_channels:
                [false; CHANNEL_COUNT],
            policy,
        }
    }


    /// Satisfy the texture requirements of the selected shader.
    ///
    /// Resolve the selected shader's procedural texture through
    /// per-shader override, global configuration, and random
    /// fallback policy. The same generated texture is bound to
    /// every channel referenced by that shader.
    pub fn prepare_for_shader(
        &mut self,
        shader_name: &str,
        channel_usage: ShaderChannelUsage,
    ) -> Result<(), String> {

        self.prepare_for_shader_with_selection(
            shader_name,
            channel_usage,
            PreviewTextureSelection::default(),
        )
    }


    pub fn prepare_for_shader_with_selection(
        &mut self,
        shader_name: &str,
        channel_usage: ShaderChannelUsage,
        preview_selection: PreviewTextureSelection,
    ) -> Result<(), String> {

        self.active_channels =
            channel_usage.channels;


        if !channel_usage.uses_any_channel() {

            if preview_selection.texture.is_some()
                || preview_selection.palette.is_some()
            {
                log_warning(
                    &format!(
                        "[PREVIEW_SHADER] Texture/palette command-line options ignored because '{}' does not use texture channels",
                        shader_name,
                    )
                );
            }


            if matching_override(
                shader_name,
                &self.policy.texture_overrides,
            )
            .is_some()
            {
                log_warning(
                    &format!(
                        "[TEXTURE] Texture override configured for '{}', but the shader does not use texture channels; override ignored",
                        shader_name,
                    )
                );
            }


            log_debug(
                "[TEXTURE] Active shader does not require texture channels"
            );


            self.delete_current_texture();


            return Ok(());
        }


        let (
            request,
            texture_source,
            palette_source,
        ) =
            resolve_texture_selection(
                shader_name,
                &self.policy,
                preview_selection,
            );


        if matching_override(
            shader_name,
            &self.policy.texture_overrides,
        )
        .is_some()
        {
            log_debug(
                &format!(
                    "[TEXTURE] Shader override matched: {}",
                    shader_name,
                )
            );
        }


        log_information(
            &format!(
                "[TEXTURE] Selected procedural texture: family={}, primitives={}, palette={}, seed={}",
                request.texture.family,
                request.texture.requested_primitive_count,
                request.palette,
                request.seed,
            )
        );


        log_debug(
            &format!(
                "[TEXTURE] Selection source: texture={}, palette={}",
                texture_source,
                palette_source,
            )
        );


        let generated =
            crate::generate_textures::generate_from_specification(
                &request.texture,
                request.palette,
                request.seed,
            )?;


        generated.validate_standard()?;


        let gpu_texture =
            upload_generated_texture(
                generated
            )?;


        log_information(
            &format!(
                "[TEXTURE] Uploaded texture {}x{} as OpenGL object {}",
                gpu_texture.width,
                gpu_texture.height,
                gpu_texture.id,
            )
        );


        let previous_texture =
            self.texture.replace(
                gpu_texture
            );


        if let Some(previous_texture) =
            previous_texture
        {
            delete_gpu_texture(
                previous_texture
            );
        }


        log_debug(
            &format!(
                "[TEXTURE] Active channel assignment: {}",
                describe_channels(
                    &self.active_channels
                ),
            )
        );


        Ok(())
    }


    /// Return the complete procedural texture specification and
    /// palette currently bound for the active shader. Shaders
    /// without texture channels return None.
    pub fn active_specification_selection(
        &self,
    ) -> Option<(
        TextureSpecification,
        Palette,
    )> {

        self.texture
            .as_ref()
            .map(
                |texture| {
                    (
                        texture.specification,
                        texture.palette,
                    )
                }
            )
    }


    /// Assign sampler units and channel-resolution values for a
    /// newly linked shader program.
    pub fn configure_program(
        &self,
        program: u32,
    ) {
        unsafe {
            gl::UseProgram(
                program
            );
        }


        for channel_index in
            0..CHANNEL_COUNT
        {
            configure_sampler_uniform(
                program,
                channel_index,
            );
        }


        self.configure_channel_resolutions(
            program
        );
    }


    /// Bind the active GPU texture to each channel referenced by
    /// the current shader. Unused channels are explicitly cleared
    /// so stale OpenGL state cannot leak between shaders.
    pub fn bind_channels(
        &self,
    ) {
        unsafe {
            for channel_index in
                0..CHANNEL_COUNT
            {
                gl::ActiveTexture(
                    gl::TEXTURE0
                        + channel_index as u32
                );


                let texture_id =
                    if self.active_channels[
                        channel_index
                    ] {
                        self.texture
                            .as_ref()
                            .map_or(
                                0,
                                |texture| {
                                    texture.id
                                },
                            )
                    } else {
                        0
                    };


                gl::BindTexture(
                    gl::TEXTURE_2D,
                    texture_id,
                );
            }


            gl::ActiveTexture(
                gl::TEXTURE0
            );
        }
    }


    /// Delete all texture objects while the renderer's OpenGL
    /// context is still current.
    pub fn delete_all(
        &mut self,
    ) {

        self.delete_current_texture();


        self.active_channels =
            [false; CHANNEL_COUNT];
    }


    fn delete_current_texture(
        &mut self,
    ) {

        if let Some(texture) =
            self.texture.take()
        {
            delete_gpu_texture(
                texture
            );
        }
    }


    fn configure_channel_resolutions(
        &self,
        program: u32,
    ) {
        let mut resolutions =
            [0.0_f32; CHANNEL_COUNT * 3];


        if let Some(texture) =
            &self.texture
        {
            for channel_index in
                0..CHANNEL_COUNT
            {
                if !self.active_channels[
                    channel_index
                ] {
                    continue;
                }


                let offset =
                    channel_index * 3;


                resolutions[offset] =
                    texture.width as f32;

                resolutions[offset + 1] =
                    texture.height as f32;

                resolutions[offset + 2] =
                    1.0;
            }
        }


        let uniform_name =
            CString::new(
                "iChannelResolution[0]"
            )
            .expect(
                "static channel-resolution uniform name"
            );


        unsafe {
            let location =
                gl::GetUniformLocation(
                    program,
                    uniform_name
                        .as_ptr(),
                );


            if location
                != -1
            {
                gl::Uniform3fv(
                    location,
                    CHANNEL_COUNT as i32,
                    resolutions.as_ptr(),
                );
            }
        }
    }
}



// ============================================================
// OpenGL upload
// ============================================================

fn upload_generated_texture(
    generated: GeneratedTexture,
) -> Result<GpuTexture, String> {

    let width =
        i32::try_from(
            generated.width
        )
        .map_err(
            |_| {
                "Texture width exceeds OpenGL i32 range"
                    .to_string()
            }
        )?;


    let height =
        i32::try_from(
            generated.height
        )
        .map_err(
            |_| {
                "Texture height exceeds OpenGL i32 range"
                    .to_string()
            }
        )?;


    let flipped_pixels =
        flip_rgba_rows(
            &generated.pixels,
            generated.width,
            generated.height,
        )?;


    clear_gl_errors();


    let mut texture_id =
        0_u32;


    unsafe {
        gl::GenTextures(
            1,
            &mut texture_id,
        );
    }


    if texture_id
        == 0
    {
        return Err(
            "OpenGL failed to create a texture object"
                .to_string()
        );
    }


    unsafe {
        gl::BindTexture(
            gl::TEXTURE_2D,
            texture_id,
        );


        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_WRAP_S,
            gl::REPEAT as i32,
        );


        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_WRAP_T,
            gl::REPEAT as i32,
        );


        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_MIN_FILTER,
            gl::LINEAR_MIPMAP_LINEAR as i32,
        );


        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_MAG_FILTER,
            gl::LINEAR as i32,
        );


        gl::PixelStorei(
            gl::UNPACK_ALIGNMENT,
            1,
        );


        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA8 as i32,
            width,
            height,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            flipped_pixels
                .as_ptr()
                .cast(),
        );


        gl::GenerateMipmap(
            gl::TEXTURE_2D
        );


        gl::BindTexture(
            gl::TEXTURE_2D,
            0,
        );
    }


    if let Some(error) =
        take_gl_error(
            "uploading procedural texture"
        )
    {
        unsafe {
            gl::DeleteTextures(
                1,
                &texture_id,
            );
        }


        return Err(
            error
        );
    }


    Ok(
        GpuTexture {
            id:
                texture_id,
            width:
                generated.width,
            height:
                generated.height,
            specification:
                generated.specification,
            palette:
                generated.palette,
            seed:
                generated.seed,
        }
    )
}


fn configure_sampler_uniform(
    program: u32,
    channel_index: usize,
) {
    let uniform_name =
        CString::new(
            format!(
                "iChannel{channel_index}"
            )
        )
        .expect(
            "generated sampler uniform name contains no null byte"
        );


    unsafe {
        let location =
            gl::GetUniformLocation(
                program,
                uniform_name.as_ptr(),
            );


        if location
            != -1
        {
            gl::Uniform1i(
                location,
                channel_index as i32,
            );
        }
    }
}


// ============================================================
// Pixel orientation
// ============================================================

fn flip_rgba_rows(
    pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<Vec<u8>, String> {

    let row_bytes =
        usize::try_from(
            width
        )
        .map_err(
            |_| {
                "Texture width cannot be represented as usize"
                    .to_string()
            }
        )?
        .checked_mul(
            crate::generate_textures::CHANNELS_PER_PIXEL
        )
        .ok_or_else(
            || {
                "Texture row size overflow"
                    .to_string()
            }
        )?;


    let height =
        usize::try_from(
            height
        )
        .map_err(
            |_| {
                "Texture height cannot be represented as usize"
                    .to_string()
            }
        )?;


    let expected_length =
        row_bytes
            .checked_mul(
                height
            )
            .ok_or_else(
                || {
                    "Texture buffer size overflow"
                        .to_string()
                }
            )?;


    if pixels.len()
        != expected_length
    {
        return Err(
            format!(
                "Cannot flip texture rows: expected {} bytes, received {}",
                expected_length,
                pixels.len(),
            )
        );
    }


    let mut flipped =
        vec![
            0_u8;
            pixels.len()
        ];


    for source_row in
        0..height
    {
        let destination_row =
            height
                - 1
                - source_row;


        let source_start =
            source_row
                * row_bytes;


        let destination_start =
            destination_row
                * row_bytes;


        flipped[
            destination_start
                ..destination_start
                    + row_bytes
        ]
        .copy_from_slice(
            &pixels[
                source_start
                    ..source_start
                        + row_bytes
            ]
        );
    }


    Ok(
        flipped
    )
}


fn delete_gpu_texture(
    texture: GpuTexture,
) {

    unsafe {
        if texture.id
            != 0
        {
            gl::DeleteTextures(
                1,
                &texture.id,
            );
        }
    }


    log_information(
        &format!(
            "[TEXTURE] Deleted OpenGL texture {} ({} / {}, seed={})",
            texture.id,
            texture.specification.display_name(),
            texture.palette,
            texture.seed,
        )
    );
}


// ============================================================
// Diagnostics
// ============================================================

fn describe_channels(
    channels: &[bool; CHANNEL_COUNT],
) -> String {
    let names =
        channels
            .iter()
            .enumerate()
            .filter_map(
                |(index, active)| {
                    if *active {
                        Some(
                            format!(
                                "iChannel{index}"
                            )
                        )
                    } else {
                        None
                    }
                }
            )
            .collect::<Vec<_>>();


    if names.is_empty() {
        "none".to_string()
    } else {
        names.join(
            ", "
        )
    }
}


fn clear_gl_errors() {
    unsafe {
        while gl::GetError()
            != gl::NO_ERROR
        {}
    }
}


fn take_gl_error(
    operation: &str,
) -> Option<String> {
    let error =
        unsafe {
            gl::GetError()
        };


    if error
        == gl::NO_ERROR
    {
        None
    } else {
        Some(
            format!(
                "OpenGL error 0x{error:04X} while {operation}"
            )
        )
    }
}


fn log_warning(
    message: &str,
) {
    let logfile =
        crate::locate_paths::runtime_log_path();


    crate::logger::warning(
        &logfile,
        message,
    );
}


fn log_information(
    message: &str,
) {
    let logfile =
        crate::locate_paths::runtime_log_path();


    crate::logger::information(
        &logfile,
        message,
    );
}


fn log_debug(
    message: &str,
) {
    let logfile =
        crate::locate_paths::runtime_log_path();


    crate::logger::debug(
        &logfile,
        message,
    );
}

