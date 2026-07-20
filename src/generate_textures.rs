//! Procedural texture generation for Screenshaver.
//!
//! Texture generators create in-memory 1024x1024 RGBA8 pixel
//! buffers. OpenGL upload and display are handled elsewhere.

use std::fmt;
use std::str::FromStr;
use crate::palettes::Palette;
use crate::parse_texture_specification::TextureSpecification;


// ============================================================
// Procedural Texture Engine standards
// ============================================================

pub const TEXTURE_SIZE: u32 =
    1024;

pub const CHANNELS_PER_PIXEL: usize =
    4;


// ============================================================
// Texture families
// ============================================================

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
)]
pub enum TextureFamily {


    Marble,

    Clouds,

    Cellular,


    Mesh,

    Radial,


    Noise,

    Bricks,

    Hexagons,
}


impl TextureFamily {

    pub const ALL: [TextureFamily; 8] = [


        TextureFamily::Marble,

        TextureFamily::Clouds,

        TextureFamily::Cellular,


        TextureFamily::Mesh,

        TextureFamily::Radial,


        TextureFamily::Noise,

        TextureFamily::Bricks,

        TextureFamily::Hexagons,
    ];


    pub const fn name(
        self,
    ) -> &'static str {

        match self {

            TextureFamily::Marble => {
                "marble"
            }

            TextureFamily::Clouds => {
                "clouds"
            }

            TextureFamily::Cellular => {
                "cellular"
            }

            TextureFamily::Mesh => {
                "mesh"
            }

            TextureFamily::Radial => {
                "radial"
            }

            TextureFamily::Noise => {
                "noise"
            }

            TextureFamily::Bricks => {
                "bricks"
            }

            TextureFamily::Hexagons => {
                "hexagons"
            }

        }
    }


    pub fn from_name(
        value: &str,
    ) -> Result<Self, String> {

        value.parse()
    }


    pub fn names() -> Vec<&'static str> {

        Self::ALL
            .iter()
            .map(
                |family| {
                    family.name()
                }
            )
            .collect()
    }
}


impl FromStr for TextureFamily {

    type Err =
        String;


    fn from_str(
        value: &str,
    ) -> Result<Self, Self::Err> {

        let normalized =
            value
                .trim()
                .to_ascii_lowercase();


        match normalized.as_str() {

            "marble" => {
                Ok(
                    TextureFamily::Marble
                )
            }

            "clouds" => {
                Ok(
                    TextureFamily::Clouds
                )
            }

            "cellular" => {
                Ok(
                    TextureFamily::Cellular
                )
            }

            "mesh" => {
                Ok(
                    TextureFamily::Mesh
                )
            }

            "radial" => {
                Ok(
                    TextureFamily::Radial
                )
            }

            "noise" => {
                Ok(
                    TextureFamily::Noise
                )
            }

            "bricks" => {
                Ok(
                    TextureFamily::Bricks
                )
            }

            "hexagons" => {
                Ok(
                    TextureFamily::Hexagons
                )
            }

            _ => {
                Err(
                    format!(
                        "Unknown texture family '{}'. Valid families: {}",
                        value,
                        TextureFamily::names()
                            .join(", "),
                    )
                )
            }
        }
    }
}


impl fmt::Display for TextureFamily {

    fn fmt(
        &self,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {

        formatter.write_str(
            self.name()
        )
    }
}


// ============================================================
// Generated texture
// ============================================================

#[derive(Debug, Clone)]
pub struct GeneratedTexture {

    pub width:
        u32,

    pub height:
        u32,

    pub pixels:
        Vec<u8>,

    pub family:
        TextureFamily,

    pub palette:
        Palette,

    pub seed:
        u64,
}


impl GeneratedTexture {

    pub fn new(
        width: u32,
        height: u32,
        pixels: Vec<u8>,
        family: TextureFamily,
        palette: Palette,
        seed: u64,
    ) -> Result<Self, String> {

        let expected_length =
            expected_pixel_buffer_length(
                width,
                height,
            )?;


        if pixels.len()
            != expected_length
        {
            return Err(
                format!(
                    "Invalid texture buffer: expected {} bytes for {}x{} RGBA8, received {}",
                    expected_length,
                    width,
                    height,
                    pixels.len(),
                )
            );
        }


        Ok(
            Self {
                width,
                height,
                pixels,
                family,
                palette,
                seed,
            }
        )
    }


    pub fn pixel_count(
        &self,
    ) -> usize {

        self.width as usize
            * self.height as usize
    }


    pub fn byte_count(
        &self,
    ) -> usize {

        self.pixels.len()
    }


    pub fn validate_standard(
        &self,
    ) -> Result<(), String> {

        if self.width
            != TEXTURE_SIZE
            || self.height
                != TEXTURE_SIZE
        {
            return Err(
                format!(
                    "Generated texture is {}x{}; Screenshaver requires {}x{}",
                    self.width,
                    self.height,
                    TEXTURE_SIZE,
                    TEXTURE_SIZE,
                )
            );
        }


        let expected_length =
            expected_pixel_buffer_length(
                self.width,
                self.height,
            )?;


        if self.pixels.len()
            != expected_length
        {
            return Err(
                format!(
                    "Generated texture contains {} bytes; expected {}",
                    self.pixels.len(),
                    expected_length,
                )
            );
        }


        Ok(())
    }
}


// ============================================================
// Public generation API
// ============================================================


pub fn generate_from_specification(
    specification: &TextureSpecification,
    palette: Palette,
    seed: u64,
) -> Result<GeneratedTexture, String> {

    match specification.family {

        TextureFamily::Hexagons => {
            crate::generate_hexagons::generate(
                palette,
                seed,
                specification.requested_primitive_count,
            )
        }

        TextureFamily::Bricks => {
            crate::generate_bricks::generate(
                palette,
                seed,
                specification.requested_primitive_count,
            )
        }

        TextureFamily::Cellular => {
            crate::generate_cellular::generate(
                palette,
                seed,
                specification.requested_primitive_count,
            )
        }

        TextureFamily::Clouds => {
            crate::generate_clouds::generate(
                palette,
                seed,
                specification.requested_primitive_count,
            )
        }

        TextureFamily::Marble => {
            crate::generate_marble::generate(
                palette,
                seed,
                specification.requested_primitive_count,
            )
        }

        TextureFamily::Mesh => {
            crate::generate_mesh::generate(
                palette,
                seed,
                specification.requested_primitive_count,
            )
        } 

        TextureFamily::Noise => {
            crate::generate_noise::generate(
                palette,
                seed,
                specification.requested_primitive_count,
            )
        }

        TextureFamily::Radial => {
            crate::generate_radial::generate(
                palette,
                seed,
                specification.requested_primitive_count,
            )
        }       

    }
}


// ============================================================
// Shared helpers
// ============================================================

fn expected_pixel_buffer_length(
    width: u32,
    height: u32,
) -> Result<usize, String> {

    let width =
        usize::try_from(
            width
        )
        .map_err(
            |_| {
                "Texture width cannot be represented as usize"
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


    width
        .checked_mul(
            height
        )
        .and_then(
            |pixels| {
                pixels.checked_mul(
                    CHANNELS_PER_PIXEL
                )
            }
        )
        .ok_or_else(
            || {
                "Texture dimensions overflow the pixel-buffer size"
                    .to_string()
            }
        )
}


// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {

    use super::*;


    #[test]
    fn parses_family_names_case_insensitively() {

        assert_eq!(
            "CLOUDS"
                .parse::<TextureFamily>(),
            Ok(
                TextureFamily::Clouds
            )
        );
    }


    #[test]
    fn rejects_unknown_family_names() {

        assert!(
            "rainbow"
                .parse::<TextureFamily>()
                .is_err()
        );
    }


    #[test]
    fn cloud_diagnostic_generates_standard_texture() {

        let specification =
            TextureSpecification {
                family: TextureFamily::Clouds,

                requested_primitive_count: 144,

                count_was_explicit: false,
            };

        let texture =
            generate_from_specification(
                &specification,
                Palette::Mist,
                1,
            )
            .expect(
                "diagnostic texture generation"
            );

        assert_eq!(
            texture.width,
            TEXTURE_SIZE
        );

        assert_eq!(
            texture.height,
            TEXTURE_SIZE
        );

        assert_eq!(
            texture.byte_count(),
            (TEXTURE_SIZE as usize)
                * (TEXTURE_SIZE as usize)
                * 4
        );

        assert!(
            texture
                .validate_standard()
                .is_ok()
        );
    }



}

