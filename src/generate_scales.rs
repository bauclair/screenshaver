//! Procedural overlapping-scale texture generation.
//!
//! The Scales engine renders a deterministic fish-scale / scallop pattern:
//!
//! - each scale is a circular primitive whose upper portion is hidden by the
//!   row above it;
//! - alternating rows are offset horizontally by half a scale width;
//! - rows overlap vertically by half a scale height;
//! - scale outlines are neutral black;
//! - scale faces use the selected Screenshaver palette;
//! - individual scales receive subtle deterministic darker shading;
//! - the lattice extends beyond all texture boundaries so edge scales are
//!   naturally clipped;
//! - requested primitive count controls approximate visual density.

use crate::generate_textures::{
    GeneratedTexture,
    TextureFamily,
    TEXTURE_SIZE,
};

use crate::palettes::PaletteColor;


// ============================================================
// Scale-generation parameters
// ============================================================

pub const MIN_SCALE_COUNT: usize =
    2;

pub const MAX_SCALE_COUNT: usize =
    crate::define_constants::MAX_TEXTURE_PRIMITIVES;

/// Scales intentionally render at a denser effective primitive count than
/// other procedural texture families. The user-facing count remains unchanged
/// (2 through 1024), but layout sizing is calculated using this multiplier.
const SCALE_DENSITY_MULTIPLIER: f32 =
    4.0;

const SCALE_BASE_VALUE: f32 =
    0.88;

const SCALE_DARKENING_STRENGTH: f32 =
    0.16;

const OUTLINE_WIDTH_FRACTION: f32 =
    0.060;

const OUTLINE_WIDTH_MIN: f32 =
    1.25;

const OUTLINE_WIDTH_MAX: f32 =
    5.0;

/// Subpixel samples per axis used to antialias scale edges.
///
/// 4x4 = 16 coverage samples per output pixel. This is intentionally more
/// expensive than a single analytic center sample because generated textures
/// are created only when the texture changes, not every rendered frame.
const ANTIALIAS_SAMPLES_PER_AXIS: usize =
    4;

const LATTICE_MARGIN: i32 =
    3;


// ============================================================
// Internal geometry
// ============================================================

#[derive(
    Debug,
    Clone,
    Copy,
)]
struct ScaleLayout {

    radius:
        f32,

    horizontal_pitch:
        f32,

    vertical_pitch:
        f32,

    outline_width:
        f32,
}


impl ScaleLayout {

    fn new(
        requested_primitive_count: usize,
    ) -> Self {

        let requested =
            requested_primitive_count
                .clamp(
                    MIN_SCALE_COUNT,
                    MAX_SCALE_COUNT,
                ) as f32
                * SCALE_DENSITY_MULTIPLIER;


        let texture_area =
            TEXTURE_SIZE as f32
                * TEXTURE_SIZE as f32;


        let radius =
            (
                texture_area
                    / (
                        requested
                            * 2.0
                    )
            )
            .sqrt()
            .max(
                1.0
            );


        let horizontal_pitch =
            radius
                * 2.0;


        let vertical_pitch =
            radius;


        let outline_width =
            (
                radius
                    * OUTLINE_WIDTH_FRACTION
            )
            .clamp(
                OUTLINE_WIDTH_MIN,
                OUTLINE_WIDTH_MAX,
            );


        Self {
            radius,
            horizontal_pitch,
            vertical_pitch,
            outline_width,
        }
    }
}


// ============================================================
// Public generator
// ============================================================

pub fn generate(
    palette: PaletteColor,
    seed: u64,
    requested_primitive_count: usize,
) -> Result<GeneratedTexture, String> {

    let layout =
        ScaleLayout::new(
            requested_primitive_count
        );


    let pixel_count =
        TEXTURE_SIZE as usize
            * TEXTURE_SIZE as usize;


    let byte_count =
        pixel_count
            .checked_mul(
                4
            )
            .ok_or_else(
                || {
                    "Scale texture buffer size overflow"
                        .to_string()
                }
            )?;


    let background =
        palette.map_rgba(
            SCALE_BASE_VALUE
        );


    let mut pixels =
        Vec::with_capacity(
            byte_count
        );


    for _ in
        0..pixel_count
    {
        pixels.extend_from_slice(
            &background
        );
    }


    render_scale_lattice(
        &mut pixels,
        &layout,
        palette,
        seed,
    );


    GeneratedTexture::new(
        TEXTURE_SIZE,
        TEXTURE_SIZE,
        pixels,
        TextureFamily::Scales,
        palette,
        seed,
    )
}


// ============================================================
// Scale lattice
// ============================================================

fn render_scale_lattice(
    pixels: &mut [u8],
    layout: &ScaleLayout,
    palette: PaletteColor,
    seed: u64,
) {

    let texture_size =
        TEXTURE_SIZE as f32;


    let maximum_row =
        (
            texture_size
                / layout.vertical_pitch
        )
        .ceil() as i32
        + LATTICE_MARGIN;


    let maximum_column =
        (
            texture_size
                / layout.horizontal_pitch
        )
        .ceil() as i32
        + LATTICE_MARGIN;


    for row in
        (
            -LATTICE_MARGIN
                ..=
            maximum_row
        )
        .rev()
    {
        let row_offset =
            if row
                & 1
                == 0
            {
                0.0
            } else {
                layout.radius
            };


        let center_y =
            row as f32
                * layout.vertical_pitch;


        for column in
            -maximum_column
                ..=
            maximum_column
        {
            let center_x =
                column as f32
                    * layout.horizontal_pitch
                + row_offset;


            let face_value =
                scale_face_value(
                    column,
                    row,
                    seed,
                );


            let face_color =
                palette.map_rgba(
                    face_value
                );


            draw_scale_primitive(
                pixels,
                center_x,
                center_y,
                layout.radius,
                layout.outline_width,
                face_color,
            );
        }
    }
}


// ============================================================
// Scale primitive
// ============================================================

fn draw_scale_primitive(
    pixels: &mut [u8],
    center_x: f32,
    center_y: f32,
    radius: f32,
    outline_width: f32,
    face_color: [u8; 4],
) {

    let minimum_x =
        (
            center_x
                - radius
                - outline_width
        )
        .floor() as i32;


    let maximum_x =
        (
            center_x
                + radius
                + outline_width
        )
        .ceil() as i32;


    let minimum_y =
        (
            center_y
                - radius
                - outline_width
        )
        .floor() as i32;


    let maximum_y =
        (
            center_y
                + radius
                + outline_width
        )
        .ceil() as i32;


    let inner_outline_radius =
        (
            radius
                - outline_width
        )
        .max(
            0.0
        );


    let radius_squared =
        radius
            * radius;


    let inner_outline_squared =
        inner_outline_radius
            * inner_outline_radius;


    for pixel_y in
        minimum_y
            ..=
        maximum_y
    {
        if pixel_y < 0
            || pixel_y
                >= TEXTURE_SIZE as i32
        {
            continue;
        }


        let sample_y =
            pixel_y as f32
                + 0.5;


        let delta_y =
            sample_y
                - center_y;


        for pixel_x in
            minimum_x
                ..=
            maximum_x
        {
            if pixel_x < 0
                || pixel_x
                    >= TEXTURE_SIZE as i32
            {
                continue;
            }


            let sample_x =
                pixel_x as f32
                    + 0.5;


            let delta_x =
                sample_x
                    - center_x;


            let distance_squared =
                delta_x
                    * delta_x
                + delta_y
                    * delta_y;


            if distance_squared
                > radius_squared
            {
                continue;
            }


            //-------------------------------------------------
            // 4x4 supersampled coverage.
            //
            // The previous analytic one-pixel blend still sampled the
            // geometry only at each pixel center. That leaves a slight
            // staircase on shallow portions of the circular arc. Here we
            // evaluate 16 evenly distributed subpixel locations and derive
            // true fill/outline coverage from those samples.
            //-------------------------------------------------

            let mut fill_samples =
                0_usize;

            let mut outline_samples =
                0_usize;


            for sample_y_index in
                0..ANTIALIAS_SAMPLES_PER_AXIS
            {
                let subpixel_y =
                    pixel_y as f32
                        + (
                            sample_y_index as f32
                                + 0.5
                        )
                        / ANTIALIAS_SAMPLES_PER_AXIS as f32;


                let sample_delta_y =
                    subpixel_y
                        - center_y;


                for sample_x_index in
                    0..ANTIALIAS_SAMPLES_PER_AXIS
                {
                    let subpixel_x =
                        pixel_x as f32
                            + (
                                sample_x_index as f32
                                    + 0.5
                            )
                            / ANTIALIAS_SAMPLES_PER_AXIS as f32;


                    let sample_delta_x =
                        subpixel_x
                            - center_x;


                    let sample_distance_squared =
                        sample_delta_x
                            * sample_delta_x
                        + sample_delta_y
                            * sample_delta_y;


                    if sample_distance_squared
                        > radius_squared
                    {
                        continue;
                    }


                    let sample_is_outline =
                        sample_delta_y
                            >= 0.0
                        && sample_distance_squared
                            >= inner_outline_squared;


                    if sample_is_outline {
                        outline_samples +=
                            1;
                    } else {
                        fill_samples +=
                            1;
                    }
                }
            }


            let sample_count =
                (
                    ANTIALIAS_SAMPLES_PER_AXIS
                        * ANTIALIAS_SAMPLES_PER_AXIS
                ) as f32;


            let fill_coverage =
                fill_samples as f32
                    / sample_count;


            let outline_coverage =
                outline_samples as f32
                    / sample_count;


            let destination_index =
                (
                    pixel_y as usize
                        * TEXTURE_SIZE as usize
                    + pixel_x as usize
                )
                    * 4;


            if fill_coverage
                <= 0.0
                && outline_coverage
                    <= 0.0
            {
                continue;
            }


            let existing =
                [
                    pixels[
                        destination_index
                    ],
                    pixels[
                        destination_index
                            + 1
                    ],
                    pixels[
                        destination_index
                            + 2
                    ],
                    pixels[
                        destination_index
                            + 3
                    ],
                ];


            let mut blended =
                existing;


            //-------------------------------------------------
            // First blend the scale face, then the black outline.
            //-------------------------------------------------

            for channel in
                0..3
            {
                let existing_value =
                    existing[
                        channel
                    ] as f32;


                let face_value =
                    face_color[
                        channel
                    ] as f32;


                let with_face =
                    existing_value
                        * (
                            1.0
                                - fill_coverage
                        )
                    + face_value
                        * fill_coverage;


                let with_outline =
                    with_face
                        * (
                            1.0
                                - outline_coverage
                        );


                blended[
                    channel
                ] =
                    with_outline
                        .round()
                        .clamp(
                            0.0,
                            255.0,
                        )
                        as u8;
            }


            blended[3] =
                255;


            pixels[
                destination_index
                    .. destination_index
                        + 4
            ]
            .copy_from_slice(
                &blended
            );
        }
    }
}


// ============================================================
// Deterministic scale shading
// ============================================================

fn scale_face_value(
    column: i32,
    row: i32,
    seed: u64,
) -> f32 {

    let variation =
        scale_attribute(
            column,
            row,
            seed,
        );


    (
        SCALE_BASE_VALUE
            - variation
                * SCALE_DARKENING_STRENGTH
    )
    .clamp(
        0.0,
        1.0,
    )
}


fn scale_attribute(
    column: i32,
    row: i32,
    seed: u64,
) -> f32 {

    let value =
        seed
            ^ (
                column as i64
                    as u64
            )
            .wrapping_mul(
                0x9E37_79B1_85EB_CA87
            )
            ^ (
                row as i64
                    as u64
            )
            .wrapping_mul(
                0xC2B2_AE3D_27D4_EB4F
            );


    hash_to_unit_float(
        mix_u64(
            value
        )
    )
}


fn hash_to_unit_float(
    value: u64,
) -> f32 {

    let upper =
        (
            value >> 40
        ) as u32;


    upper as f32
        / 16_777_215.0
}


fn mix_u64(
    mut value: u64,
) -> u64 {

    value ^=
        value >> 30;


    value =
        value.wrapping_mul(
            0xBF58_476D_1CE4_E5B9
        );


    value ^=
        value >> 27;


    value =
        value.wrapping_mul(
            0x94D0_49BB_1331_11EB
        );


    value ^=
        value >> 31;


    value
}


// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {

    use super::*;


    #[test]
    fn requested_count_is_clamped_to_supported_range() {

        let minimum =
            ScaleLayout::new(
                0
            );


        let explicit_minimum =
            ScaleLayout::new(
                MIN_SCALE_COUNT
            );


        assert!(
            (
                minimum.radius
                    - explicit_minimum.radius
            )
            .abs()
                < 0.0001
        );


        let maximum =
            ScaleLayout::new(
                usize::MAX
            );


        let explicit_maximum =
            ScaleLayout::new(
                MAX_SCALE_COUNT
            );


        assert!(
            (
                maximum.radius
                    - explicit_maximum.radius
            )
            .abs()
                < 0.0001
        );
    }


    #[test]
    fn denser_request_produces_smaller_scales() {

        let sparse =
            ScaleLayout::new(
                2
            );


        let dense =
            ScaleLayout::new(
                1024
            );


        assert!(
            dense.radius
                < sparse.radius
        );
    }


    #[test]
    fn same_seed_is_deterministic() {

        let first =
            scale_face_value(
                7,
                11,
                12345,
            );


        let second =
            scale_face_value(
                7,
                11,
                12345,
            );


        assert_eq!(
            first,
            second
        );
    }


    #[test]
    fn different_seeds_change_scale_shading() {

        let first =
            scale_face_value(
                7,
                11,
                1,
            );


        let second =
            scale_face_value(
                7,
                11,
                2,
            );


        assert_ne!(
            first,
            second
        );
    }


    #[test]
    fn generator_returns_standard_texture() {

        let texture =
            generate(
                PaletteColor::new(
                    170,
                    170,
                    170,
                ),
                12345,
                64,
            )
            .expect(
                "scale generation"
            );


        assert_eq!(
            texture.specification.family,
            TextureFamily::Scales
        );


        assert_eq!(
            texture.width,
            TEXTURE_SIZE
        );


        assert_eq!(
            texture.height,
            TEXTURE_SIZE
        );


        assert!(
            texture
                .validate_standard()
                .is_ok()
        );
    }
}
