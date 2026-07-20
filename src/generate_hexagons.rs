//! Procedural hexagon texture generation.
//!
//! The Hexagons engine renders a regular flat-top honeycomb:
//!
//! - the requested count controls approximate visual density;
//! - valid requested counts range from 1 through 1024;
//! - omitted counts are resolved to 1 before reaching this module;
//! - the grid extends beyond the texture boundaries;
//! - edge hexagons are naturally clipped;
//! - all hexagons use one palette-derived fill color;
//! - shared edges are rendered with a uniform black outline;
//! - no lighting, beveling, or random variation is applied.

use crate::palettes::Palette;
use crate::generate_textures:: {
    GeneratedTexture,
    TextureFamily,
    TEXTURE_SIZE,
};


// ============================================================
// Hexagon-generation parameters
// ============================================================

/// Lowest supported requested primitive count.
pub const MIN_HEXAGON_COUNT: usize =
    crate::define_constants::MIN_TEXTURE_PRIMITIVES;

/// Highest supported requested primitive count.
pub const MAX_HEXAGON_COUNT: usize =
    crate::define_constants::MAX_TEXTURE_PRIMITIVES;

/// Width of the black hexagon boundaries in pixels.
const OUTLINE_WIDTH: f32 =
    2.0;

/// Palette position used for the uniform hexagon fill.
const HEXAGON_FILL_VALUE: f32 =
    0.72;

/// Number of binary-search iterations used to solve the radius.
const LAYOUT_SEARCH_ITERATIONS: usize =
    64;

/// Extra lattice cells examined beyond the visible texture.
const GRID_MARGIN: i32 =
    3;


// ============================================================
// Internal geometry
// ============================================================

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
)]
struct Point {
    x: f32,
    y: f32,
}


impl Point {
    const fn new(
        x: f32,
        y: f32,
    ) -> Self {

        Self {
            x,
            y,
        }
    }
}


#[derive(
    Debug,
    Clone,
    Copy,
)]
struct HexagonLayout {
    radius: f32,
    requested_count: usize,
    actual_center_count: usize,
}


// ============================================================
// Public generator
// ============================================================

pub fn generate(
    palette: Palette,
    seed: u64,
    requested_primitive_count: usize,
) -> Result<GeneratedTexture, String> {

    //---------------------------------------------------------
    // The seed is retained in the public generator contract so
    // this module remains compatible with the other procedural
    // texture generators. Geometry is intentionally deterministic
    // and currently does not use the seed.
    //---------------------------------------------------------

    let _ =
        seed;


    let requested_hexagon_count =
        requested_primitive_count
            .clamp(
                MIN_HEXAGON_COUNT,
                MAX_HEXAGON_COUNT,
            );


    let layout =
        calculate_hexagon_layout(
            TEXTURE_SIZE,
            TEXTURE_SIZE,
            requested_hexagon_count,
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
                    "Hexagon texture buffer size overflow"
                        .to_string()
                }
            )?;


    let mut pixels =
        Vec::with_capacity(
            byte_count
        );


    let fill_color =
        palette.map_rgba(
            HEXAGON_FILL_VALUE
        );


    for pixel_y in
        0..TEXTURE_SIZE
    {
        for pixel_x in
            0..TEXTURE_SIZE
        {
            let sample =
                Point::new(
                    pixel_x as f32
                        + 0.5,

                    pixel_y as f32
                        + 0.5,
                );


            let center =
                nearest_hexagon_center(
                    sample,
                    layout.radius,
                    TEXTURE_SIZE,
                    TEXTURE_SIZE,
                );


            let local_point =
                Point::new(
                    sample.x
                        - center.x,

                    sample.y
                        - center.y,
                );


            let distance_to_edge =
                distance_to_hexagon_edge(
                    local_point,
                    layout.radius,
                );


            let color =
                if distance_to_edge
                    <= OUTLINE_WIDTH
                        * 0.5
                {
                    [
                        0,
                        0,
                        0,
                        255,
                    ]

                } else {
                    fill_color
                };


            pixels.extend_from_slice(
                &color
            );
        }
    }


    GeneratedTexture::new(
        TEXTURE_SIZE,
        TEXTURE_SIZE,
        pixels,
        TextureFamily::Hexagons,
        palette,
        seed,
    )
}


// ============================================================
// Layout solver
// ============================================================

fn calculate_hexagon_layout(
    width: u32,
    height: u32,
    requested_count: usize,
) -> HexagonLayout {

    let width =
        width as f32;

    let height =
        height as f32;


    //---------------------------------------------------------
    // A radius of one pixel produces far more cells than the
    // supported maximum. A radius larger than the texture
    // diagonal produces a single visible center.
    //---------------------------------------------------------

    let mut minimum_radius =
        1.0_f32;

    let mut maximum_radius =
        width.hypot(
            height
        )
        * 2.0;


    let mut best_radius =
        maximum_radius;

    let mut best_actual_count =
        count_visible_hexagon_centers(
            width,
            height,
            best_radius,
        );

    let mut best_difference =
        best_actual_count.abs_diff(
            requested_count
        );


    for _ in
        0..LAYOUT_SEARCH_ITERATIONS
    {
        let candidate_radius =
            (
                minimum_radius
                    + maximum_radius
            )
            * 0.5;


        let candidate_count =
            count_visible_hexagon_centers(
                width,
                height,
                candidate_radius,
            );


        let candidate_difference =
            candidate_count.abs_diff(
                requested_count
            );


        if candidate_difference
            < best_difference
            || (
                candidate_difference
                    == best_difference
                && candidate_radius
                    < best_radius
            )
        {
            best_radius =
                candidate_radius;

            best_actual_count =
                candidate_count;

            best_difference =
                candidate_difference;
        }


        //-----------------------------------------------------
        // Larger radii produce fewer visible cell centers.
        //-----------------------------------------------------

        if candidate_count
            > requested_count
        {
            minimum_radius =
                candidate_radius;

        } else if candidate_count
            < requested_count
        {
            maximum_radius =
                candidate_radius;

        } else {
            //-------------------------------------------------
            // Continue toward the smaller end of the radius
            // interval while preserving the exact count. This
            // selects a dense, well-filled version of that
            // geometrically valid layout.
            //-------------------------------------------------

            maximum_radius =
                candidate_radius;
        }
    }


    HexagonLayout {
        radius:
            best_radius,

        requested_count,

        actual_center_count:
            best_actual_count,
    }
}


fn count_visible_hexagon_centers(
    width: f32,
    height: f32,
    radius: f32,
) -> usize {

    let center_x =
        width
            * 0.5;

    let center_y =
        height
            * 0.5;


    let horizontal_spacing =
        radius
            * 1.5;

    let vertical_spacing =
        radius
            * 3.0_f32.sqrt();


    let maximum_column =
        (
            width
                / horizontal_spacing
        )
        .ceil() as i32
        + GRID_MARGIN;


    let maximum_row =
        (
            height
                / vertical_spacing
        )
        .ceil() as i32
        + maximum_column
        + GRID_MARGIN;


    let mut count =
        0_usize;


    for axial_column in
        -maximum_column
            ..=
        maximum_column
    {
        for axial_row in
            -maximum_row
                ..=
            maximum_row
        {
            let center =
                axial_to_pixel(
                    axial_column,
                    axial_row,
                    radius,
                    center_x,
                    center_y,
                );


            if center.x
                >= 0.0
                && center.x
                    < width
                && center.y
                    >= 0.0
                && center.y
                    < height
            {
                count +=
                    1;
            }
        }
    }


    count
}


// ============================================================
// Hexagonal-grid coordinate conversion
// ============================================================

fn axial_to_pixel(
    axial_column: i32,
    axial_row: i32,
    radius: f32,
    origin_x: f32,
    origin_y: f32,
) -> Point {

    let column =
        axial_column as f32;

    let row =
        axial_row as f32;


    Point::new(
        origin_x
            + radius
                * 1.5
                * column,

        origin_y
            + radius
                * 3.0_f32.sqrt()
                * (
                    row
                        + column
                            * 0.5
                ),
    )
}


fn nearest_hexagon_center(
    point: Point,
    radius: f32,
    width: u32,
    height: u32,
) -> Point {

    let origin_x =
        width as f32
            * 0.5;

    let origin_y =
        height as f32
            * 0.5;


    let relative_x =
        point.x
            - origin_x;

    let relative_y =
        point.y
            - origin_y;


    //---------------------------------------------------------
    // Inverse flat-top axial-coordinate transform.
    //---------------------------------------------------------

    let fractional_column =
        (
            2.0
                / 3.0
            * relative_x
        )
        / radius;


    let fractional_row =
        (
            -1.0
                / 3.0
                * relative_x
            + 3.0_f32.sqrt()
                / 3.0
                * relative_y
        )
        / radius;


    let (
        axial_column,
        axial_row,
    ) =
        round_axial_coordinates(
            fractional_column,
            fractional_row,
        );


    axial_to_pixel(
        axial_column,
        axial_row,
        radius,
        origin_x,
        origin_y,
    )
}


fn round_axial_coordinates(
    fractional_column: f32,
    fractional_row: f32,
) -> (
    i32,
    i32,
) {

    //---------------------------------------------------------
    // Convert axial coordinates into cube coordinates, round
    // all three axes, then repair the axis with the greatest
    // rounding error.
    //---------------------------------------------------------

    let cube_x =
        fractional_column;

    let cube_z =
        fractional_row;

    let cube_y =
        -cube_x
            - cube_z;


    let mut rounded_x =
        cube_x.round();

    let rounded_y =
        cube_y.round();

    let mut rounded_z =
        cube_z.round();


    let difference_x =
        (
            rounded_x
                - cube_x
        )
        .abs();

    let difference_y =
        (
            rounded_y
                - cube_y
        )
        .abs();

    let difference_z =
        (
            rounded_z
                - cube_z
        )
        .abs();


    if difference_x
        > difference_y
        && difference_x
            > difference_z
    {
        rounded_x =
            -rounded_y
                - rounded_z;

    } else if difference_z
        > difference_y
    {
        rounded_z =
            -rounded_x
                - rounded_y;
    }


    (
        rounded_x as i32,
        rounded_z as i32,
    )
}


// ============================================================
// Hexagon boundary distance
// ============================================================

fn distance_to_hexagon_edge(
    point: Point,
    radius: f32,
) -> f32 {

    let vertices =
        flat_top_hexagon_vertices(
            radius
        );


    let mut minimum_distance =
        f32::INFINITY;


    for edge_index in
        0..6
    {
        let start =
            vertices[
                edge_index
            ];

        let end =
            vertices[
                (
                    edge_index
                        + 1
                )
                % 6
            ];


        minimum_distance =
            minimum_distance.min(
                distance_to_line_segment(
                    point,
                    start,
                    end,
                )
            );
    }


    minimum_distance
}


fn flat_top_hexagon_vertices(
    radius: f32,
) -> [Point; 6] {

    let half_radius =
        radius
            * 0.5;

    let vertical_radius =
        radius
            * 3.0_f32.sqrt()
            * 0.5;


    [
        Point::new(
            radius,
            0.0,
        ),

        Point::new(
            half_radius,
            vertical_radius,
        ),

        Point::new(
            -half_radius,
            vertical_radius,
        ),

        Point::new(
            -radius,
            0.0,
        ),

        Point::new(
            -half_radius,
            -vertical_radius,
        ),

        Point::new(
            half_radius,
            -vertical_radius,
        ),
    ]
}


fn distance_to_line_segment(
    point: Point,
    start: Point,
    end: Point,
) -> f32 {

    let segment_x =
        end.x
            - start.x;

    let segment_y =
        end.y
            - start.y;


    let point_x =
        point.x
            - start.x;

    let point_y =
        point.y
            - start.y;


    let segment_length_squared =
        segment_x
            * segment_x
        + segment_y
            * segment_y;


    if segment_length_squared
        <= f32::EPSILON
    {
        return (
            point_x
                * point_x
            + point_y
                * point_y
        )
        .sqrt();
    }


    let projection =
        (
            point_x
                * segment_x
            + point_y
                * segment_y
        )
        / segment_length_squared;


    let projection =
        projection.clamp(
            0.0,
            1.0,
        );


    let nearest_x =
        start.x
            + segment_x
                * projection;

    let nearest_y =
        start.y
            + segment_y
                * projection;


    let distance_x =
        point.x
            - nearest_x;

    let distance_y =
        point.y
            - nearest_y;


    (
        distance_x
            * distance_x
        + distance_y
            * distance_y
    )
    .sqrt()
}


// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {

    use super::*;


    #[test]
    fn requested_count_is_clamped_to_supported_range() {

        assert_eq!(
            0_usize.clamp(
                MIN_HEXAGON_COUNT,
                MAX_HEXAGON_COUNT,
            ),
            1
        );


        assert_eq!(
            2000_usize.clamp(
                MIN_HEXAGON_COUNT,
                MAX_HEXAGON_COUNT,
            ),
            crate::define_constants::MAX_TEXTURE_PRIMITIVES
        );
    }


    #[test]
    fn axial_origin_is_texture_center() {

        let center =
            axial_to_pixel(
                0,
                0,
                50.0,
                512.0,
                512.0,
            );


        assert_eq!(
            center,
            Point::new(
                512.0,
                512.0,
            )
        );
    }


    #[test]
    fn nearest_center_of_origin_is_origin() {

        let center =
            nearest_hexagon_center(
                Point::new(
                    512.0,
                    512.0,
                ),
                50.0,
                1024,
                1024,
            );


        assert!(
            (
                center.x
                    - 512.0
            )
            .abs()
                < 0.001
        );


        assert!(
            (
                center.y
                    - 512.0
            )
            .abs()
                < 0.001
        );
    }


    #[test]
    fn layout_solver_approximates_requested_count() {

        let layout =
            calculate_hexagon_layout(
                1024,
                1024,
                144,
            );


        assert!(
            layout.actual_center_count
                .abs_diff(
                    layout.requested_count
                )
                <= 4
        );
    }

    #[test]
    fn layout_solver_supports_maximum_primitive_count() {

        let layout =
            calculate_hexagon_layout(
                1024,
                1024,
                crate::define_constants::MAX_TEXTURE_PRIMITIVES,
            );


        // A centered hexagonal lattice can only change its visible
        // center count in discrete row-and-column steps. At the
        // 1024-primitive boundary, the closest valid layout contains
        // 1033 visible centers, a difference of nine.
        assert!(
            layout.actual_center_count
                .abs_diff(
                    layout.requested_count
                )
                <= 16
        );
    }


#[test]
fn generated_texture_has_standard_dimensions() {

    let texture =
        generate(
            Palette::Mist,
            1,
            144,
        )
        .expect(
            "hexagon texture generation"
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
}

}

