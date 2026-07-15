//! Procedural jigsaw texture generation.
//!
//! This engine renders a true interlocking puzzle layout rather
//! than a Voronoi mosaic. The texture is based on a regular grid
//! whose shared edges receive deterministic tabs or sockets.
//!
//! The first profile uses a medium-frequency 12 × 12 layout.

use crate::generate_textures::{
    GeneratedTexture,
    TextureFamily,
    TEXTURE_SIZE,
};

use crate::palettes::Palette;


// ============================================================
// Jigsaw parameters
// ============================================================

const COLUMNS: i32 =
    12;

const ROWS: i32 =
    12;

/// Radius of each tab/socket, measured as a fraction of a piece.
const TAB_RADIUS: f32 =
    0.19;

/// Maximum tab/socket depth, measured as a fraction of a piece.
const TAB_DEPTH: f32 =
    0.16;

/// Slight deterministic displacement of tab centers.
const TAB_CENTER_JITTER: f32 =
    0.055;

/// Width of the darkest seam, in normalized piece coordinates.
const SEAM_WIDTH: f32 =
    0.024;

/// Width of the raised bevel around each seam.
const BEVEL_WIDTH: f32 =
    0.090;

/// Darkness of puzzle seams.
const SEAM_DARKENING: f32 =
    0.48;

/// Strength of raised-edge lighting.
const BEVEL_STRENGTH: f32 =
    0.22;

/// Per-piece brightness variation.
const PIECE_VARIATION_STRENGTH: f32 =
    0.15;

/// Subtle planar shading inside each piece.
const PIECE_PLANE_STRENGTH: f32 =
    0.08;

/// Surface variation that prevents perfectly flat fills.
const SURFACE_VARIATION_STRENGTH: f32 =
    0.025;

const SURFACE_VARIATION_FREQUENCY: f32 =
    5.0;

const CONTRAST: f32 =
    1.08;

const BRIGHTNESS: f32 =
    0.01;


// ============================================================
// Public generator
// ============================================================

pub fn generate(
    palette: Palette,
    seed: u64,
) -> Result<GeneratedTexture, String> {

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
                    "Jigsaw texture buffer size overflow"
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
                jigsaw_value(
                    normalized_x,
                    normalized_y,
                    seed,
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
        TextureFamily::Jigsaw,
        palette,
        seed,
    )
}


// ============================================================
// Jigsaw field
// ============================================================

fn jigsaw_value(
    x: f32,
    y: f32,
    seed: u64,
) -> f32 {

    let grid_x =
        x
            * COLUMNS as f32;


    let grid_y =
        y
            * ROWS as f32;


    let column =
        grid_x
            .floor() as i32;


    let row =
        grid_y
            .floor() as i32;


    let local_x =
        grid_x
            - column as f32;


    let local_y =
        grid_y
            - row as f32;


    //---------------------------------------------------------
    // Evaluate distances to the four shared puzzle boundaries.
    // Each boundary uses the same deterministic edge definition
    // from either neighboring piece, so tabs and sockets match.
    //---------------------------------------------------------

    let left_distance =
        vertical_edge_distance(
            local_x,
            local_y,
            column,
            row,
            seed,
        );


    let right_distance =
        vertical_edge_distance(
            1.0 - local_x,
            local_y,
            column + 1,
            row,
            seed,
        );


    let top_distance =
        horizontal_edge_distance(
            local_y,
            local_x,
            row,
            column,
            seed.wrapping_add(
                0xA24B_AED4_963E_E407
            ),
        );


    let bottom_distance =
        horizontal_edge_distance(
            1.0 - local_y,
            local_x,
            row + 1,
            column,
            seed.wrapping_add(
                0xA24B_AED4_963E_E407
            ),
        );


    let boundary_distance =
        left_distance
            .min(
                right_distance
            )
            .min(
                top_distance
            )
            .min(
                bottom_distance
            );


    let seam =
        1.0
            - smoothstep(
                0.0,
                SEAM_WIDTH,
                boundary_distance,
            );


    let bevel_outer =
        1.0
            - smoothstep(
                SEAM_WIDTH,
                BEVEL_WIDTH,
                boundary_distance,
            );


    let bevel =
        (
            bevel_outer
                - seam
        )
        .max(
            0.0
        );


    let piece_variation =
        (
            piece_attribute(
                column,
                row,
                seed.wrapping_add(
                    0x9E37_79B9_7F4A_7C15
                ),
            )
            - 0.5
        )
        * PIECE_VARIATION_STRENGTH;


    let plane_angle =
        piece_attribute(
            column,
            row,
            seed.wrapping_add(
                0x94D0_49BB_1331_11EB
            ),
        )
        * std::f32::consts::TAU;


    let plane =
        (
            (
                local_x - 0.5
            )
                * plane_angle.cos()
            + (
                local_y - 0.5
            )
                * plane_angle.sin()
        )
        * PIECE_PLANE_STRENGTH;


    let surface =
        (
            value_noise(
                x * SURFACE_VARIATION_FREQUENCY,
                y * SURFACE_VARIATION_FREQUENCY,
                seed.wrapping_add(
                    0xD1B5_4A32_D192_ED03
                ),
            )
            - 0.5
        )
        * SURFACE_VARIATION_STRENGTH;


    let bevel_light =
        bevel
            * BEVEL_STRENGTH;


    let value =
        0.60
            + piece_variation
            + plane
            + surface
            + bevel_light
            - seam
                * SEAM_DARKENING;


    apply_tone_curve(
        value.clamp(
            0.0,
            1.0,
        )
    )
}


// ============================================================
// Shared puzzle-edge geometry
// ============================================================

/// Distance to a vertical shared edge.
///
/// `distance_from_edge` is zero at the base grid line and grows
/// toward the current piece interior.
fn vertical_edge_distance(
    distance_from_edge: f32,
    along_edge: f32,
    edge_x: i32,
    edge_y: i32,
    seed: u64,
) -> f32 {

    let tab_center =
        0.5
            + (
                edge_attribute(
                    edge_x,
                    edge_y,
                    seed,
                    0xBF58_476D_1CE4_E5B9,
                )
                - 0.5
            )
            * TAB_CENTER_JITTER;


    let tab_direction =
        if edge_attribute(
            edge_x,
            edge_y,
            seed,
            0x8CB9_2BA7_2F3D_8DD7,
        )
            < 0.5
        {
            -1.0
        } else {
            1.0
        };


    edge_distance(
        distance_from_edge,
        along_edge,
        tab_center,
        tab_direction,
    )
}


/// Horizontal counterpart to `vertical_edge_distance`.
fn horizontal_edge_distance(
    distance_from_edge: f32,
    along_edge: f32,
    edge_y: i32,
    edge_x: i32,
    seed: u64,
) -> f32 {

    let tab_center =
        0.5
            + (
                edge_attribute(
                    edge_x,
                    edge_y,
                    seed,
                    0x94D0_49BB_1331_11EB,
                )
                - 0.5
            )
            * TAB_CENTER_JITTER;


    let tab_direction =
        if edge_attribute(
            edge_x,
            edge_y,
            seed,
            0xDB4F_0B91_75AE_2165,
        )
            < 0.5
        {
            -1.0
        } else {
            1.0
        };


    edge_distance(
        distance_from_edge,
        along_edge,
        tab_center,
        tab_direction,
    )
}


/// Convert a straight shared edge into a tab/socket silhouette.
///
/// The semicircular displacement is positive for a tab and
/// negative for a socket. Neighboring pieces evaluate the same
/// shared edge from opposite sides, so the shapes interlock.
fn edge_distance(
    distance_from_edge: f32,
    along_edge: f32,
    tab_center: f32,
    tab_direction: f32,
) -> f32 {

    let along_delta =
        along_edge
            - tab_center;


    let normalized =
        along_delta
            / TAB_RADIUS;


    let tab_profile =
        if normalized
            .abs()
            < 1.0
        {
            (
                1.0
                    - normalized
                        * normalized
            )
            .sqrt()
                * TAB_DEPTH
                * tab_direction
        } else {
            0.0
        };


    (
        distance_from_edge
            - tab_profile
    )
    .abs()
}


// ============================================================
// Deterministic attributes
// ============================================================

fn edge_attribute(
    x: i32,
    y: i32,
    seed: u64,
    salt: u64,
) -> f32 {

    hash_to_unit_float(
        mix_u64(
            seed
                ^ salt
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
                )
        )
    )
}


fn piece_attribute(
    column: i32,
    row: i32,
    seed: u64,
) -> f32 {

    edge_attribute(
        column,
        row,
        seed,
        0xA24B_AED4_963E_E407,
    )
}


// ============================================================
// Surface noise
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
        piece_attribute(
            x0,
            y0,
            seed,
        );


    let value_10 =
        piece_attribute(
            x1,
            y0,
            seed,
        );


    let value_01 =
        piece_attribute(
            x0,
            y1,
            seed,
        );


    let value_11 =
        piece_attribute(
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


// ============================================================
// Hashing, interpolation and tone mapping
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


fn apply_tone_curve(
    value: f32,
) -> f32 {

    let centered =
        (
            value - 0.5
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
        edge_end - edge_start
    )
    .abs()
        <= f32::EPSILON
    {
        return 0.0;
    }


    let normalized =
        (
            (
                value - edge_start
            )
            / (
                edge_end - edge_start
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
    fn jigsaw_value_is_normalized() {

        for y in
            0..32
        {
            for x in
                0..32
            {
                let value =
                    jigsaw_value(
                        (
                            x as f32 + 0.5
                        )
                            / 32.0,

                        (
                            y as f32 + 0.5
                        )
                            / 32.0,

                        12345,
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

        assert_eq!(
            jigsaw_value(
                0.37,
                0.63,
                999,
            ),
            jigsaw_value(
                0.37,
                0.63,
                999,
            )
        );
    }


    #[test]
    fn different_seeds_change_output() {

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
                        x as f32 + 0.5
                    )
                        / 16.0;


                let sample_y =
                    (
                        y as f32 + 0.5
                    )
                        / 16.0;


                let first =
                    jigsaw_value(
                        sample_x,
                        sample_y,
                        1,
                    );


                let second =
                    jigsaw_value(
                        sample_x,
                        sample_y,
                        2,
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
    fn generator_returns_standard_texture() {

        let texture =
            generate(
                Palette::Slate,
                12345,
            )
            .expect(
                "jigsaw generation"
            );


        assert_eq!(
            texture.family,
            TextureFamily::Jigsaw
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

