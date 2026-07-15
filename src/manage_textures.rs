//! Procedural texture management for Screenshaver.
//!
//! This module bridges the CPU-side Procedural Texture Engine
//! and the OpenGL texture channels exposed to ShaderToy shaders.
//! It owns all GPU texture objects used by the frame renderer.

use std::ffi::CString;
use std::path::PathBuf;

use crate::generate_textures::{
    GeneratedTexture,
    TextureFamily,
};
use crate::palettes::Palette;
use crate::preprocess_shader::ShaderChannelUsage;


// ============================================================
// Initial diagnostic selection
// ============================================================

/// Fixed texture selection used until configuration-based and
/// random selection policy is connected to this module.
const DEFAULT_TEXTURE_FAMILY: TextureFamily =
    TextureFamily::Bricks;

const DEFAULT_PALETTE: Palette =
    Palette::Brick;

const DEFAULT_SEED: u64 =
    12_345;

const CHANNEL_COUNT: usize =
    4;


// ============================================================
// GPU texture representation
// ============================================================

#[derive(Debug)]
struct GpuTexture {
    id: u32,
    width: u32,
    height: u32,
    family: TextureFamily,
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
}


impl TextureManager {

    pub fn new() -> Self {
        Self {
            texture: None,
            active_channels: [false; CHANNEL_COUNT],
        }
    }


    /// Satisfy the texture requirements of the selected shader.
    ///
    /// The first implementation deliberately uses one generated
    /// texture for every channel referenced by the shader. Later,
    /// selection will be resolved from per-shader overrides,
    /// global configuration, or random fallback policy.
    pub fn prepare_for_shader(
        &mut self,
        channel_usage: ShaderChannelUsage,
    ) -> Result<(), String> {

        self.active_channels =
            channel_usage.channels;


        if !channel_usage.uses_any_channel() {
            log(
                "[TEXTURE] Active shader does not require texture channels"
            );

            return Ok(());
        }


        if self.texture.is_none() {
            log(
                &format!(
                    "[TEXTURE] Generating diagnostic texture: family={}, palette={}, seed={}",
                    DEFAULT_TEXTURE_FAMILY,
                    DEFAULT_PALETTE,
                    DEFAULT_SEED,
                )
            );


            let generated =
                crate::generate_textures::generate(
                    DEFAULT_TEXTURE_FAMILY,
                    DEFAULT_PALETTE,
                    DEFAULT_SEED,
                )?;


            generated.validate_standard()?;


            let gpu_texture =
                upload_generated_texture(
                    generated
                )?;


            log(
                &format!(
                    "[TEXTURE] Uploaded texture {}x{} as OpenGL object {}",
                    gpu_texture.width,
                    gpu_texture.height,
                    gpu_texture.id,
                )
            );


            self.texture =
                Some(
                    gpu_texture
                );
        }


        log(
            &format!(
                "[TEXTURE] Active channel assignment: {}",
                describe_channels(
                    &self.active_channels
                ),
            )
        );


        Ok(())
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
        let Some(texture) =
            self.texture.take()
        else {
            self.active_channels =
                [false; CHANNEL_COUNT];

            return;
        };


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


        log(
            &format!(
                "[TEXTURE] Deleted OpenGL texture {} ({} / {}, seed={})",
                texture.id,
                texture.family,
                texture.palette,
                texture.seed,
            )
        );


        self.active_channels =
            [false; CHANNEL_COUNT];
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


impl Default for TextureManager {
    fn default() -> Self {
        Self::new()
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
            family:
                generated.family,
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


fn log(
    message: &str,
) {
    let logfile: PathBuf =
        crate::locate_paths::runtime_log_path();


    crate::logger::log(
        &logfile,
        message,
    );
}

