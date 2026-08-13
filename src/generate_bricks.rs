//! Procedural brick-wall texture generation.
//!
//! The Bricks engine renders a deterministic running-bond wall:
//!
//! - alternating rows are offset by half a brick;
//! - shifted rows begin and end with clipped half-bricks;
//! - the left and right texture edges remain fully filled;
//! - brick faces use the selected palette;
//! - mortar remains neutral gray;
//! - individual bricks receive subtle deterministic variation;
//! - shallow beveling gives the wall visible relief.

use crate::generate_textures::{
    GeneratedTexture,
    TextureFamily,
    TEXTURE_SIZE,
};

use crate::palettes::PaletteColor;


// ============================================================
// Brick-wall parameters
// ============================================================

/// Approximate number of full bricks across an unshifted row.
const BRICKS_ACROSS: f32 =
    8.0;

/// Width-to-height ratio of one brick.
const BRICK_ASPECT_RATIO: f32 =
    2.15;

/// Mortar thickness as a fraction of brick height.
const MORTAR_WIDTH: f32 =
    0.085;

/// Width of the bevel transition inside each brick.
const BEVEL_WIDTH: f32 =
    0.075;

/// Strength of upper/left bevel highlights.
const BEVEL_HIGHLIGHT_STRENGTH: f32 =
    0.12;

/// Strength of lower/right bevel shadows.
const BEVEL_SHADOW_STRENGTH: f32 =
    0.15;

/// Base scalar value sent through the selected palette.
const BRICK_BASE_VALUE: f32 =
    0.62;

/// Maximum per-brick scalar offset within the active palette.
const BRICK_VARIATION_STRENGTH: f32 =
    0.16;

/// Strength of fine surface variation on brick faces.
const SURFACE_VARIATION_STRENGTH: f32 =
    0.045;

/// Frequency of fine surface variation.
const SURFACE_VARIATION_FREQUENCY: f32 =
    34.0;

/// Strength of a broader mottled brick-face field.
const MOTTLE_STRENGTH: f32 =
    0.055;

/// Frequency of broad brick-face mottling.
const MOTTLE_FREQUENCY: f32 =
    5.0;

/// Neutral mortar base color.
const MORTAR_RED: f32 =
    112.0;

const MORTAR_GREEN: f32 =
    114.0;

const MORTAR_BLUE: f32 =
    113.0;

/// Mortar brightness variation.
const MORTAR_VARIATION_STRENGTH: f32 =
    16.0;

/// Final brick-face contrast.
const BRICK_CONTRAST: f32 =
    1.05;

/// Final brick-face brightness.
const BRICK_BRIGHTNESS: f32 =
    0.0;



// ============================================================
// Brick layout
// ============================================================

#[derive(Debug, Clone, Copy)]
struct BrickLayout {
    brick_width: f32,
    brick_height: f32,
    mortar_x: f32,
    mortar_y: f32,
}

impl BrickLayout {
    fn new(requested_primitive_count: usize) -> Self {
        let requested = requested_primitive_count.clamp(1, 4096) as f32;

        // For normalized texture dimensions:
        //
        //     brick_width  = 1 / columns
        //     brick_height = 1 / rows
        //
        // Therefore:
        //
        //     brick_width / brick_height = rows / columns
        //
        // To preserve BRICK_ASPECT_RATIO, the wall needs roughly
        // BRICK_ASPECT_RATIO times as many rows as columns.
        let columns =
            ((requested / BRICK_ASPECT_RATIO).sqrt())
                .max(1.0)
                .round();

        let rows =
            (requested / columns)
                .max(1.0)
                .round();

        let brick_width =
            1.0 / columns;

        let brick_height =
            1.0 / rows;

        Self {
            brick_width,
            brick_height,
            mortar_x: brick_width * MORTAR_WIDTH * 0.5,
            mortar_y: brick_height * MORTAR_WIDTH * 0.5,
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

    let layout = BrickLayout::new(requested_primitive_count);

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
                    "Brick texture buffer size overflow"
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


            let color =
                brick_wall_color(
                    normalized_x,
                    normalized_y,
                    &layout,
                    palette,
                    seed,
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
        TextureFamily::Bricks,
        palette,
        seed,
    )
}


// ============================================================
// Brick-wall field
// ============================================================

fn brick_wall_color(
    x: f32,
    y: f32,
    layout: &BrickLayout,
    palette: PaletteColor,
    seed: u64,
) -> [u8; 4] {

    let brick_width = layout.brick_width;

    let brick_height = layout.brick_height;


    let row_position =
        y
            / brick_height;


    let row_index =
        row_position
            .floor() as i32;


    let local_y =
        row_position
            - row_index as f32;


    //---------------------------------------------------------
    // Running bond:
    //
    // Even rows begin with full bricks.
    // Odd rows are shifted by half a brick, which naturally
    // produces clipped half-bricks at both texture edges.
    //---------------------------------------------------------

    let row_offset =
        if row_index
            & 1
            == 0
        {
            0.0
        } else {
            0.5
        };


    let column_position =
        x
            / brick_width
            + row_offset;


    let column_index =
        column_position
            .floor() as i32;


    let local_x =
        column_position
            - column_index as f32;


    let mortar_x = layout.mortar_x;

    let mortar_y = layout.mortar_y;


    let distance_left =
        local_x;


    let distance_right =
        1.0
            - local_x;


    let distance_top =
        local_y;


    let distance_bottom =
        1.0
            - local_y;


    let nearest_vertical_edge =
        distance_left.min(
            distance_right
        );


    let nearest_horizontal_edge =
        distance_top.min(
            distance_bottom
        );


    let in_vertical_mortar =
        nearest_vertical_edge
            < mortar_x;


    let in_horizontal_mortar =
        nearest_horizontal_edge
            < mortar_y;


    if in_vertical_mortar
        || in_horizontal_mortar
    {
        return mortar_color(
            x,
            y,
            seed,
            nearest_vertical_edge,
            nearest_horizontal_edge,
            mortar_x,
            mortar_y,
        );
    }


    let brick_value =
        brick_face_value(
            x,
            y,
            local_x,
            local_y,
            column_index,
            row_index,
            mortar_x,
            mortar_y,
            seed,
        );


    palette.map_rgba(
        brick_value
    )
}


// ============================================================
// Brick-face shading
// ============================================================

#[allow(
    clippy::too_many_arguments
)]
fn brick_face_value(
    x: f32,
    y: f32,
    local_x: f32,
    local_y: f32,
    column_index: i32,
    row_index: i32,
    mortar_x: f32,
    mortar_y: f32,
    seed: u64,
) -> f32 {

    let brick_variation =
        (
            brick_attribute(
                column_index,
                row_index,
                seed.wrapping_add(
                    0xA24B_AED4_963E_E407
                ),
            )
            - 0.5
        )
        * BRICK_VARIATION_STRENGTH;


    let surface =
        (
            value_noise(
                x * SURFACE_VARIATION_FREQUENCY,
                y * SURFACE_VARIATION_FREQUENCY,
                seed.wrapping_add(
                    0x9E37_79B9_7F4A_7C15
                ),
            )
            - 0.5
        )
        * SURFACE_VARIATION_STRENGTH;


    let mottle =
        (
            value_noise(
                x * MOTTLE_FREQUENCY,
                y * MOTTLE_FREQUENCY,
                seed.wrapping_add(
                    0xD1B5_4A32_D192_ED03
                ),
            )
            - 0.5
        )
        * MOTTLE_STRENGTH;


    let usable_left =
        mortar_x;


    let usable_right =
        1.0
            - mortar_x;


    let usable_top =
        mortar_y;


    let usable_bottom =
        1.0
            - mortar_y;


    let left_bevel =
        1.0
            - smoothstep(
                usable_left,
                usable_left
                    + BEVEL_WIDTH,
                local_x,
            );


    let right_bevel =
        smoothstep(
            usable_right
                - BEVEL_WIDTH,
            usable_right,
            local_x,
        );


    let top_bevel =
        1.0
            - smoothstep(
                usable_top,
                usable_top
                    + BEVEL_WIDTH,
                local_y,
            );


    let bottom_bevel =
        smoothstep(
            usable_bottom
                - BEVEL_WIDTH,
            usable_bottom,
            local_y,
        );


    let highlight =
        (
            left_bevel
                + top_bevel
        )
        * 0.5
        * BEVEL_HIGHLIGHT_STRENGTH;


    let shadow =
        (
            right_bevel
                + bottom_bevel
        )
        * 0.5
        * BEVEL_SHADOW_STRENGTH;


    apply_brick_tone_curve(
        (
            BRICK_BASE_VALUE
                + brick_variation
                + surface
                + mottle
                + highlight
                - shadow
        )
        .clamp(
            0.0,
            1.0,
        )
    )
}


// ============================================================
// Mortar shading
// ============================================================

#[allow(
    clippy::too_many_arguments
)]
fn mortar_color(
    x: f32,
    y: f32,
    seed: u64,
    nearest_vertical_edge: f32,
    nearest_horizontal_edge: f32,
    mortar_x: f32,
    mortar_y: f32,
) -> [u8; 4] {

    let mortar_noise =
        value_noise(
            x * 52.0,
            y * 52.0,
            seed.wrapping_add(
                0x94D0_49BB_1331_11EB
            ),
        );


    let vertical_depth =
        if mortar_x
            <= f32::EPSILON
        {
            0.0
        } else {
            1.0
                - (
                    nearest_vertical_edge
                        / mortar_x
                )
                .clamp(
                    0.0,
                    1.0,
                )
        };


    let horizontal_depth =
        if mortar_y
            <= f32::EPSILON
        {
            0.0
        } else {
            1.0
                - (
                    nearest_horizontal_edge
                        / mortar_y
                )
                .clamp(
                    0.0,
                    1.0,
                )
        };


    let recess =
        vertical_depth.max(
            horizontal_depth
        );


    let variation =
        (
            mortar_noise
                - 0.5
        )
        * MORTAR_VARIATION_STRENGTH
        - recess
            * 22.0;


    [
        clamp_channel(
            MORTAR_RED
                + variation
        ),

        clamp_channel(
            MORTAR_GREEN
                + variation
        ),

        clamp_channel(
            MORTAR_BLUE
                + variation
        ),

        255,
    ]
}


// ============================================================
// Deterministic attributes
// ============================================================

fn brick_attribute(
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
        brick_attribute(
            x0,
            y0,
            seed,
        );


    let value_10 =
        brick_attribute(
            x1,
            y0,
            seed,
        );


    let value_01 =
        brick_attribute(
            x0,
            y1,
            seed,
        );


    let value_11 =
        brick_attribute(
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


fn apply_brick_tone_curve(
    value: f32,
) -> f32 {

    (
        (
            value
                - 0.5
        )
        * BRICK_CONTRAST
        + 0.5
        + BRICK_BRIGHTNESS
    )
    .clamp(
        0.0,
        1.0,
    )
}


fn clamp_channel(
    value: f32,
) -> u8 {

    value
        .round()
        .clamp(
            0.0,
            255.0,
        )
        as u8
}


// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {

    use super::*;


    #[test]
    fn same_seed_is_deterministic() {

        let layout = BrickLayout::new(144);

        let first =
            brick_wall_color(
                0.37,
                0.63,
                &layout,
                PaletteColor::new(
            154,
            66,
            42,
        ),
                999,
            );


        let second =
            brick_wall_color(
                0.37,
                0.63,
                &layout,
                PaletteColor::new(
            154,
            66,
            42,
        ),
                999,
            );


        assert_eq!(
            first,
            second
        );
    }


    #[test]
    fn different_seeds_change_output() {

        let layout = BrickLayout::new(144);

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
                    brick_wall_color(
                        sample_x,
                        sample_y,
                        &layout,
                        PaletteColor::new(
            154,
            66,
            42,
        ),
                        1,
                    );


                let second =
                    brick_wall_color(
                        sample_x,
                        sample_y,
                        &layout,
                        PaletteColor::new(
            154,
            66,
            42,
        ),
                        2,
                    );


                if first
                    != second
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
            "Different seeds produced identical brick walls"
        );
    }


    #[test]
    fn mortar_is_neutral_gray() {

        let color =
            mortar_color(
                0.5,
                0.5,
                123,
                0.0,
                0.0,
                0.05,
                0.05,
            );


        let red_green =
            (
                color[0] as i16
                    - color[1] as i16
            )
            .abs();


        let green_blue =
            (
                color[1] as i16
                    - color[2] as i16
            )
            .abs();


        assert!(
            red_green
                <= 3
        );


        assert!(
            green_blue
                <= 3
        );
    }


    #[test]
    fn generator_returns_standard_texture() {

        let texture =
            generate(
                PaletteColor::new(
            154,
            66,
            42,
        ),
                12345,
                144,
            )
            .expect(
                "brick generation"
            );


        assert_eq!(
            texture.specification.family,
            TextureFamily::Bricks
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

