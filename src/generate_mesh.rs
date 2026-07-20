//! Procedural mesh texture generation.
//!
//! The Mesh engine renders a deterministic plain over-under
//! weave. Its structural scale can range from a handful of broad
//! basket-like strands to hundreds of fine screen-like strands.
//!
//! This first implementation focuses on:
//!
//! - independent horizontal and vertical strand frequencies;
//! - rounded strand profiles;
//! - alternating over-under crossings;
//! - crossover shadows and highlights;
//! - deterministic spacing and thickness variation;
//! - palette mapping performed by palettes.rs.

use crate::generate_textures::{
    GeneratedTexture,
    TextureFamily,
    TEXTURE_SIZE,
};

use crate::palettes::Palette;


// ============================================================
// Mesh-generation parameters
// ============================================================

/// Density derived from the requested primitive count.
#[derive(Clone, Copy, Debug)]
struct MeshLayout {
    vertical_frequency: f32,
    horizontal_frequency: f32,
}

impl MeshLayout {
    fn new(requested_primitive_count: usize) -> Self {
        let f=(requested_primitive_count.max(1) as f32).sqrt().max(1.0);
        Self{
            vertical_frequency:f,
            horizontal_frequency:f,
        }
    }
}

/// Strand width as a proportion of each vertical repeat cell.
///
/// Values below 1.0 leave visible openings between strands.
const VERTICAL_WIDTH: f32 =
    0.62;

/// Strand width as a proportion of each horizontal repeat cell.
const HORIZONTAL_WIDTH: f32 =
    0.62;

/// Controls the curvature of each strand.
///
/// Values above 1.0 make the strand center broader and its edges
/// steeper. Values below 1.0 produce a softer profile.
const STRAND_ROUNDNESS: f32 =
    1.45;

/// Strength of the bright center line on the upper strand.
const HIGHLIGHT_STRENGTH: f32 =
    0.24;

/// Darkness added beneath an over-crossing strand.
const CROSSING_SHADOW_STRENGTH: f32 =
    0.30;

/// Width of crossover shadows, in local strand coordinates.
const CROSSING_SHADOW_WIDTH: f32 =
    0.23;

/// Darkness of the open spaces between strands.
const OPENING_DARKNESS: f32 =
    0.18;

/// Amount of deterministic position variation applied to each
/// strand. Keep this small enough that strands never cross.
const SPACING_IRREGULARITY: f32 =
    0.055;

/// Amount of deterministic width variation assigned per strand.
const THICKNESS_IRREGULARITY: f32 =
    0.09;

/// Fine variation along the length of each strand.
const SURFACE_VARIATION_STRENGTH: f32 =
    0.055;

/// Frequency of fine variation along strand surfaces.
const SURFACE_VARIATION_FREQUENCY: f32 =
    7.0;

/// Rotation of the complete weave in degrees.
const ROTATION_DEGREES: f32 =
    0.0;

/// Final output contrast.
const CONTRAST: f32 =
    1.08;

/// Final brightness adjustment.
const BRIGHTNESS: f32 =
    0.0;


// ============================================================
// Public generator
// ============================================================

pub fn generate(
    palette: Palette,
    seed: u64,
    requested_primitive_count: usize,
) -> Result<GeneratedTexture, String> {

    let layout = MeshLayout::new(requested_primitive_count);

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
                    "Mesh texture buffer size overflow"
                        .to_string()
                }
            )?;


    let mut pixels =
        Vec::with_capacity(
            byte_count
        );


    let inverse_size =
        1.0
            / TEXTURE_SIZE as f32;


    for y in
        0..TEXTURE_SIZE
    {
        let normalized_y =
            (
                y as f32
                    + 0.5
            )
            * inverse_size;


        for x in
            0..TEXTURE_SIZE
        {
            let normalized_x =
                (
                    x as f32
                        + 0.5
                )
                * inverse_size;


            let value =
                mesh_value(
                    normalized_x,
                    normalized_y,
                    seed,
                    &layout,
                );


            let color =
                palette.map_rgba(
                    value
                );


            pixels.extend_from_slice(
                &color
            );
        }
    }


    GeneratedTexture::new(
        TEXTURE_SIZE,
        TEXTURE_SIZE,
        pixels,
        TextureFamily::Mesh,
        palette,
        seed,
    )
}


// ============================================================
// Mesh field
// ============================================================

fn mesh_value(
    x: f32,
    y: f32,
    seed: u64,
    layout: &MeshLayout,
) -> f32 {

    let (
        rotated_x,
        rotated_y,
    ) =
        rotate_about_center(
            x,
            y,
            ROTATION_DEGREES,
        );


    let vertical =
        strand_sample(
            rotated_x,
            rotated_y,
            layout.vertical_frequency,
            VERTICAL_WIDTH,
            StrandDirection::Vertical,
            seed,
        );


    let horizontal =
        strand_sample(
            rotated_y,
            rotated_x,
            layout.horizontal_frequency,
            HORIZONTAL_WIDTH,
            StrandDirection::Horizontal,
            seed.wrapping_add(
                0xA24B_AED4_963E_E407
            ),
        );


    let vertical_present =
        vertical.coverage
            > 0.0;


    let horizontal_present =
        horizontal.coverage
            > 0.0;


    //---------------------------------------------------------
    // Plain weave:
    //
    // At alternating intersections, either the vertical or the
    // horizontal strand passes over the other.
    //---------------------------------------------------------

    let vertical_over =
        (
            vertical.index
                + horizontal.index
        )
        & 1
        == 0;


    let value =
        match (
            vertical_present,
            horizontal_present,
        ) {
            (
                false,
                false,
            ) => {

                opening_value(
                    rotated_x,
                    rotated_y,
                    seed,
                )
            }

            (
                true,
                false,
            ) => {

                strand_value(
                    &vertical,
                    true,
                    0.0,
                )
            }

            (
                false,
                true,
            ) => {

                strand_value(
                    &horizontal,
                    true,
                    0.0,
                )
            }

            (
                true,
                true,
            ) => {

                if vertical_over {

                    let shadow =
                        crossing_shadow(
                            horizontal.center_distance,
                        );

                    strand_value(
                        &vertical,
                        true,
                        shadow,
                    )

                } else {

                    let shadow =
                        crossing_shadow(
                            vertical.center_distance,
                        );

                    strand_value(
                        &horizontal,
                        true,
                        shadow,
                    )
                }
            }
        };


    apply_tone_curve(
        value.clamp(
            0.0,
            1.0,
        )
    )
}


// ============================================================
// Strand synthesis
// ============================================================

#[derive(
    Debug,
    Clone,
    Copy,
)]
enum StrandDirection {
    Vertical,
    Horizontal,
}


#[derive(
    Debug,
    Clone,
    Copy,
)]
struct StrandSample {
    index: i32,
    coverage: f32,
    profile: f32,
    center_distance: f32,
    surface_variation: f32,
}


fn strand_sample(
    across: f32,
    along: f32,
    frequency: f32,
    base_width: f32,
    direction: StrandDirection,
    seed: u64,
) -> StrandSample {

    let scaled =
        across
            * frequency;


    let base_index =
        scaled.floor() as i32;


    let mut best_index =
        base_index;


    let mut best_distance =
        f32::MAX;


    let mut best_width =
        base_width;


    //---------------------------------------------------------
    // Examine the current repeat cell and its neighbors because
    // position irregularity can move a strand slightly across a
    // repeat-cell boundary.
    //---------------------------------------------------------

    for candidate_index in
        (
            base_index - 1
        )..=(
            base_index + 1
        )
    {
        let position_offset =
            (
                strand_attribute(
                    candidate_index,
                    direction,
                    seed,
                    0x9E37_79B9_7F4A_7C15,
                )
                - 0.5
            )
            * SPACING_IRREGULARITY;


        let center =
            candidate_index as f32
                + 0.5
                + position_offset;


        let distance =
            (
                scaled
                    - center
            )
            .abs();


        if distance
            < best_distance
        {
            let width_variation =
                (
                    strand_attribute(
                        candidate_index,
                        direction,
                        seed,
                        0x94D0_49BB_1331_11EB,
                    )
                    - 0.5
                )
                * 2.0
                * THICKNESS_IRREGULARITY;


            best_distance =
                distance;


            best_index =
                candidate_index;


            best_width =
                (
                    base_width
                        * (
                            1.0
                                + width_variation
                        )
                )
                .clamp(
                    0.08,
                    0.94,
                );
        }
    }


    let half_width =
        best_width
            * 0.5;


    let normalized_distance =
        if half_width
            <= f32::EPSILON
        {
            1.0

        } else {

            best_distance
                / half_width
        };


    let coverage =
        1.0
            - smoothstep(
                0.94,
                1.0,
                normalized_distance,
            );


    let profile =
        (
            1.0
                - normalized_distance
                    .clamp(
                        0.0,
                        1.0,
                    )
                    .powi(
                        2
                    )
        )
        .max(
            0.0
        )
        .powf(
            STRAND_ROUNDNESS
        );


    let surface_variation =
        strand_surface_variation(
            along,
            best_index,
            direction,
            seed,
        );


    StrandSample {
        index:
            best_index,

        coverage,

        profile,

        center_distance:
            normalized_distance
                .clamp(
                    0.0,
                    1.0,
                ),

        surface_variation,
    }
}


fn strand_value(
    strand: &StrandSample,
    upper: bool,
    crossing_shadow: f32,
) -> f32 {

    let upper_boost =
        if upper {
            0.08
        } else {
            -0.05
        };


    let highlight =
        strand.profile
            * HIGHLIGHT_STRENGTH;


    let edge_shading =
        (
            1.0
                - strand.profile
        )
        * 0.20;


    (
        0.50
            + upper_boost
            + highlight
            - edge_shading
            + strand.surface_variation
            - crossing_shadow
    )
    * strand.coverage
}


fn crossing_shadow(
    lower_center_distance: f32,
) -> f32 {

    let proximity =
        1.0
            - smoothstep(
                0.0,
                CROSSING_SHADOW_WIDTH,
                lower_center_distance,
            );


    proximity
        * CROSSING_SHADOW_STRENGTH
}


// ============================================================
// Openings and surface variation
// ============================================================

fn opening_value(
    x: f32,
    y: f32,
    seed: u64,
) -> f32 {

    let variation =
        value_noise(
            x * 24.0,
            y * 24.0,
            seed.wrapping_add(
                0xBF58_476D_1CE4_E5B9
            ),
        );


    (
        OPENING_DARKNESS
            + (
                variation - 0.5
            )
                * 0.035
    )
    .clamp(
        0.0,
        1.0,
    )
}


fn strand_surface_variation(
    along: f32,
    strand_index: i32,
    direction: StrandDirection,
    seed: u64,
) -> f32 {

    let direction_offset =
        match direction {
            StrandDirection::Vertical =>
                0xD1B5_4A32_D192_ED03,

            StrandDirection::Horizontal =>
                0x8CB9_2BA7_2F3D_8DD7,
        };


    let shifted =
        along
            * SURFACE_VARIATION_FREQUENCY
            + strand_attribute(
                strand_index,
                direction,
                seed,
                direction_offset,
            )
                * 17.0;


    let integer =
        shifted.floor() as i32;


    let fraction =
        shifted
            - shifted.floor();


    let start =
        hash_to_unit_float(
            mix_u64(
                seed
                    ^ (
                        strand_index as i64
                            as u64
                    )
                    .wrapping_mul(
                        0x9E37_79B1_85EB_CA87
                    )
                    ^ (
                        integer as i64
                            as u64
                    )
                    .wrapping_mul(
                        0xC2B2_AE3D_27D4_EB4F
                    )
                    ^ direction_offset
            )
        );


    let end =
        hash_to_unit_float(
            mix_u64(
                seed
                    ^ (
                        strand_index as i64
                            as u64
                    )
                    .wrapping_mul(
                        0x9E37_79B1_85EB_CA87
                    )
                    ^ (
                        (
                            integer + 1
                        ) as i64
                            as u64
                    )
                    .wrapping_mul(
                        0xC2B2_AE3D_27D4_EB4F
                    )
                    ^ direction_offset
            )
        );


    (
        interpolate(
            start,
            end,
            fade(
                fraction
            ),
        )
        - 0.5
    )
    * SURFACE_VARIATION_STRENGTH
}


// ============================================================
// Strand attributes
// ============================================================

fn strand_attribute(
    index: i32,
    direction: StrandDirection,
    seed: u64,
    salt: u64,
) -> f32 {

    let direction_value =
        match direction {
            StrandDirection::Vertical =>
                0xA24B_AED4_963E_E407,

            StrandDirection::Horizontal =>
                0x94D0_49BB_1331_11EB,
        };


    let value =
        seed
            ^ (
                index as i64
                    as u64
            )
            .wrapping_mul(
                0x9E37_79B1_85EB_CA87
            )
            ^ direction_value
            ^ salt;


    hash_to_unit_float(
        mix_u64(
            value
        )
    )
}


// ============================================================
// Coordinate helpers
// ============================================================

fn rotate_about_center(
    x: f32,
    y: f32,
    degrees: f32,
) -> (
    f32,
    f32,
) {

    if degrees
        .abs()
        <= f32::EPSILON
    {
        return (
            x,
            y,
        );
    }


    let radians =
        degrees
            .to_radians();


    let cosine =
        radians.cos();


    let sine =
        radians.sin();


    let centered_x =
        x - 0.5;


    let centered_y =
        y - 0.5;


    (
        centered_x
            * cosine
            - centered_y
                * sine
            + 0.5,

        centered_x
            * sine
            + centered_y
                * cosine
            + 0.5,
    )
}


// ============================================================
// Value noise
// ============================================================

fn value_noise(
    x: f32,
    y: f32,
    seed: u64,
) -> f32 {

    let floor_x =
        x.floor();


    let floor_y =
        y.floor();


    let x0 =
        floor_x as i32;


    let y0 =
        floor_y as i32;


    let x1 =
        x0 + 1;


    let y1 =
        y0 + 1;


    let fractional_x =
        x - floor_x;


    let fractional_y =
        y - floor_y;


    let smooth_x =
        fade(
            fractional_x
        );


    let smooth_y =
        fade(
            fractional_y
        );


    let value_00 =
        lattice_value(
            x0,
            y0,
            seed,
        );


    let value_10 =
        lattice_value(
            x1,
            y0,
            seed,
        );


    let value_01 =
        lattice_value(
            x0,
            y1,
            seed,
        );


    let value_11 =
        lattice_value(
            x1,
            y1,
            seed,
        );


    let lower =
        interpolate(
            value_00,
            value_10,
            smooth_x,
        );


    let upper =
        interpolate(
            value_01,
            value_11,
            smooth_x,
        );


    interpolate(
        lower,
        upper,
        smooth_y,
    )
}


fn lattice_value(
    x: i32,
    y: i32,
    seed: u64,
) -> f32 {

    let value =
        seed
            ^ (
                x as i64
                    as u64
            )
            .wrapping_mul(
                0x9E37_79B1_85EB_CA87
            )
            ^ (
                y as i64
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


// ============================================================
// Hashing and interpolation
// ============================================================

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


fn fade(
    value: f32,
) -> f32 {

    value
        * value
        * value
        * (
            value
                * (
                    value * 6.0
                        - 15.0
                )
                + 10.0
        )
}


fn interpolate(
    start: f32,
    end: f32,
    amount: f32,
) -> f32 {

    start
        + (
            end - start
        )
        * amount
}


// ============================================================
// Tone mapping
// ============================================================

fn apply_tone_curve(
    value: f32,
) -> f32 {

    let centered =
        (
            value
                - 0.5
        )
        * CONTRAST
        + 0.5
        + BRIGHTNESS;


    smoothstep(
        0.0,
        1.0,
        centered.clamp(
            0.0,
            1.0,
        ),
    )
}


fn smoothstep(
    edge_start: f32,
    edge_end: f32,
    value: f32,
) -> f32 {

    if (
        edge_end
            - edge_start
    )
    .abs()
        <= f32::EPSILON
    {
        return 0.0;
    }


    let normalized =
        (
            (
                value
                    - edge_start
            )
            / (
                edge_end
                    - edge_start
            )
        )
        .clamp(
            0.0,
            1.0,
        );


    normalized
        * normalized
        * (
            3.0
                - 2.0
                    * normalized
        )
}


// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {

    use super::*;


    #[test]
    fn mesh_value_is_normalized() {

        let layout = MeshLayout::new(144);


        for y in
            0..32
        {
            for x in
                0..32
            {
                let value =
                    mesh_value(
                        (
                            x as f32
                                + 0.5
                        )
                            / 32.0,

                        (
                            y as f32
                                + 0.5
                        )
                            / 32.0,

                        12345,
                        &layout,
                    );


                assert!(
                    (
                        0.0..=1.0
                    )
                    .contains(
                        &value
                    )
                );
            }
        }
    }


    #[test]
    fn same_seed_is_deterministic() {

        let layout = MeshLayout::new(144);


        let first =
            mesh_value(
                0.37,
                0.63,
                999,
                &layout,
            );


        let second =
            mesh_value(
                0.37,
                0.63,
                999,
                &layout,
            );


        assert_eq!(
            first,
            second
        );
    }


    #[test]
    fn different_seeds_change_output() {

        let layout = MeshLayout::new(144);


        let mut found_difference =
            false;


        for y in
            0..16
        {
            for x in
                0..16
            {
                let sample_x =
                    (
                        x as f32
                            + 0.5
                    )
                        / 16.0;


                let sample_y =
                    (
                        y as f32
                            + 0.5
                    )
                        / 16.0;


                let first =
                    mesh_value(
                        sample_x,
                        sample_y,
                        1,
                        &layout,
                    );


                let second =
                    mesh_value(
                        sample_x,
                        sample_y,
                        2,
                        &layout,
                    );


                if (
                    first - second
                )
                .abs()
                    > f32::EPSILON
                {
                    found_difference =
                        true;

                    break;
                }
            }


            if found_difference
            {
                break;
            }
        }


        assert!(
            found_difference,
            "Different seeds produced identical values across every sampled coordinate"
        );
    }


    #[test]
    fn layout_uses_requested_primitive_count() {
        let layout=MeshLayout::new(256);
        assert_eq!(layout.vertical_frequency,16.0);
        assert_eq!(layout.horizontal_frequency,16.0);
    }

    #[test]
    fn generator_returns_standard_texture() {

        let texture =
            generate(
                Palette::Slate,
                12345,
                144,
            )
            .expect(
                "mesh generation"
            );


        assert_eq!(
            texture.family,
            TextureFamily::Mesh
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

