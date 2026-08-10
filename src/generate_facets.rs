//! Procedural tetrahedral-facet texture generation.
//!
//! The Facets engine samples an infinite equilateral triangular lattice. Each
//! visible triangular primitive represents a tetrahedron viewed toward its apex:
//!
//! - every primitive has an equilateral triangular footprint;
//! - the triangular footprint is divided into three visible triangular faces;
//! - the fourth triangular face is the implied hidden base;
//! - the requested primitive count controls approximate visual density;
//! - geometric correctness takes priority over matching the requested count exactly;
//! - partial tetrahedrons are naturally clipped at the texture borders;
//! - every primitive uses the same fixed lighting and shading pattern;
//! - palette selection controls color, but not lighting direction;
//! - narrow recessed seams separate neighboring tetrahedrons;
//! - palette-derived highlights emphasize the three apex-to-vertex ridges;
//! - geometry is deterministic and does not use the supplied seed.

use crate::generate_textures::{
    GeneratedTexture,
    TextureFamily,
    TEXTURE_SIZE,
};
use crate::palettes::PaletteColor;

// ============================================================
// Facet-generation parameters
// ============================================================

/// Lowest supported requested primitive count.
pub const MIN_FACET_COUNT: usize =
    crate::define_constants::MIN_TEXTURE_PRIMITIVES;

/// Highest supported requested primitive count.
pub const MAX_FACET_COUNT: usize =
    crate::define_constants::MAX_TEXTURE_PRIMITIVES;

/// Bright face, oriented toward the fixed virtual light.
const LIGHT_APEX_VALUE: f32 = 0.94;
const LIGHT_EDGE_VALUE: f32 = 0.78;

/// Middle face.
const MIDDLE_APEX_VALUE: f32 = 0.68;
const MIDDLE_EDGE_VALUE: f32 = 0.52;

/// Shadow face.
const SHADOW_APEX_VALUE: f32 = 0.42;
const SHADOW_EDGE_VALUE: f32 = 0.24;

/// PaletteColor value at the center of a recessed seam.
const SEAM_VALUE: f32 = 0.08;

/// Width of shared tetrahedron boundaries in normalized barycentric units.
const OUTER_SEAM_WIDTH: f32 = 0.018;

/// Bright palette position used for the raised ridge bevels.
const RIDGE_HIGHLIGHT_VALUE: f32 = 0.985;

/// Half-width of the palette-derived apex-to-vertex ridge highlight.
const RIDGE_HIGHLIGHT_WIDTH: f32 = 0.034;

/// Soft transition around seam and highlight edges.
const SEAM_SOFTNESS: f32 = 0.008;
const RIDGE_HIGHLIGHT_SOFTNESS: f32 = 0.012;

/// Height-to-edge ratio of an equilateral triangle: sqrt(3) / 2.
const EQUILATERAL_HEIGHT_RATIO: f32 = 0.866_025_4;

/// Area coefficient of an equilateral triangle: sqrt(3) / 4.
const EQUILATERAL_AREA_COEFFICIENT: f32 = 0.433_012_7;

// ============================================================
// Internal geometry
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TriangleOrientation {
    Forward,
    Reverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisibleFace {
    Light,
    Middle,
    Shadow,
}

#[derive(Debug, Clone, Copy)]
struct FacetLayout {
    triangle_edge: f32,
    triangle_height: f32,
    requested_count: usize,
    estimated_count: usize,
}

#[derive(Debug, Clone, Copy)]
struct Barycentric {
    a: f32,
    b: f32,
    c: f32,
}

// ============================================================
// Public generator
// ============================================================

pub fn generate(
    palette: PaletteColor,
    seed: u64,
    requested_primitive_count: usize,
) -> Result<GeneratedTexture, String> {
    // The common generator contract includes a seed. Facet geometry and
    // shading are deliberately deterministic.
    let _ = seed;

    let requested_facet_count = requested_primitive_count.clamp(
        MIN_FACET_COUNT,
        MAX_FACET_COUNT,
    );

    let layout = calculate_facet_layout(
        requested_facet_count,
    );

    let pixel_count = TEXTURE_SIZE as usize
        * TEXTURE_SIZE as usize;

    let byte_count = pixel_count
        .checked_mul(4)
        .ok_or_else(|| {
            "Facet texture buffer size overflow".to_string()
        })?;

    let mut pixels = Vec::with_capacity(
        byte_count,
    );

    let texture_center = TEXTURE_SIZE as f32 * 0.5;

    for pixel_y in 0..TEXTURE_SIZE {
        for pixel_x in 0..TEXTURE_SIZE {
            // The square texture is only a viewport onto an infinite lattice.
            // Centering world coordinates makes border clipping symmetrical.
            let world_x = pixel_x as f32
                + 0.5
                - texture_center;

            let world_y = pixel_y as f32
                + 0.5
                - texture_center;

            // Inverse transform for the equilateral basis vectors:
            //
            //     b1 = (edge, 0)
            //     b2 = (edge / 2, height)
            //
            // No independent horizontal or vertical scaling is performed.
            let lattice_v = world_y
                / layout.triangle_height;

            let lattice_u = world_x
                / layout.triangle_edge
                - lattice_v * 0.5;

            let local_u = lattice_u.rem_euclid(1.0);
            let local_v = lattice_v.rem_euclid(1.0);

            let value = tetrahedral_value(
                local_u,
                local_v,
            );

            let color = palette.map_rgba(
                value,
            );

            pixels.extend_from_slice(
                &color,
            );
        }
    }

    GeneratedTexture::new(
        TEXTURE_SIZE,
        TEXTURE_SIZE,
        pixels,
        TextureFamily::Facets,
        palette,
        seed,
    )
}

// ============================================================
// Layout
// ============================================================

fn calculate_facet_layout(
    requested_count: usize,
) -> FacetLayout {
    let requested_count = requested_count.clamp(
        MIN_FACET_COUNT,
        MAX_FACET_COUNT,
    );

    let texture_area = TEXTURE_SIZE as f32
        * TEXTURE_SIZE as f32;

    // For an equilateral triangle:
    //
    //     area = sqrt(3) / 4 * edge^2
    //
    // Solve for edge length using the requested count as a density target.
    // Border clipping means the exact visible primitive count may vary.
    let target_triangle_area = texture_area
        / requested_count as f32;

    let triangle_edge =
        (target_triangle_area
            / EQUILATERAL_AREA_COEFFICIENT)
            .sqrt()
            .max(1.0);

    let triangle_height = triangle_edge
        * EQUILATERAL_HEIGHT_RATIO;

    let estimated_count =
        (texture_area
            / (EQUILATERAL_AREA_COEFFICIENT
                * triangle_edge
                * triangle_edge))
            .round()
            .max(1.0) as usize;

    FacetLayout {
        triangle_edge,
        triangle_height,
        requested_count,
        estimated_count,
    }
}

// ============================================================
// Tetrahedral geometry and shading
// ============================================================

fn tetrahedral_value(
    local_u: f32,
    local_v: f32,
) -> f32 {
    let orientation =
        if local_u + local_v <= 1.0 {
            TriangleOrientation::Forward
        } else {
            TriangleOrientation::Reverse
        };

    let barycentric = barycentric_coordinates(
        orientation,
        local_u,
        local_v,
    );

    let face = classify_visible_face(
        orientation,
        barycentric,
    );

    // Every barycentric component equals 1/3 at the apex. The minimum
    // component reaches zero at the outside edge of the selected face.
    let minimum_weight = barycentric
        .a
        .min(barycentric.b)
        .min(barycentric.c)
        .clamp(0.0, 1.0 / 3.0);

    let edge_progress =
        (1.0 - minimum_weight * 3.0)
            .clamp(0.0, 1.0);

    let face_value = match face {
        VisibleFace::Light => interpolate(
            LIGHT_APEX_VALUE,
            LIGHT_EDGE_VALUE,
            edge_progress,
        ),

        VisibleFace::Middle => interpolate(
            MIDDLE_APEX_VALUE,
            MIDDLE_EDGE_VALUE,
            edge_progress,
        ),

        VisibleFace::Shadow => interpolate(
            SHADOW_APEX_VALUE,
            SHADOW_EDGE_VALUE,
            edge_progress,
        ),
    };

    // The outer boundary is the nearest of the triangle's three edges.
    let outer_distance = minimum_weight;

    // Internal boundaries divide the triangular footprint into three faces.
    // They are the portions of the barycentric equality lines extending from
    // the centroid to each vertex.
    let inner_distance = internal_seam_distance(
        barycentric,
    );

    let outer_seam = seam_strength(
        outer_distance,
        OUTER_SEAM_WIDTH,
        SEAM_SOFTNESS,
    );

    // Add a palette-derived highlight directly on each internal
    // apex-to-vertex ridge. Unlike the outer tetrahedron boundary,
    // these internal ridges are not darkened.
    let ridge_highlight = seam_strength(
        inner_distance,
        RIDGE_HIGHLIGHT_WIDTH,
        RIDGE_HIGHLIGHT_SOFTNESS,
    );

    let highlighted_value = interpolate(
        face_value,
        RIDGE_HIGHLIGHT_VALUE,
        ridge_highlight,
    );

    interpolate(
        highlighted_value,
        SEAM_VALUE,
        outer_seam,
    )
    .clamp(0.0, 1.0)
}

fn barycentric_coordinates(
    orientation: TriangleOrientation,
    local_u: f32,
    local_v: f32,
) -> Barycentric {
    match orientation {
        // Oblique-lattice vertices: A=(0,0), B=(1,0), C=(0,1).
        TriangleOrientation::Forward => Barycentric {
            a: 1.0 - local_u - local_v,
            b: local_u,
            c: local_v,
        },

        // Oblique-lattice vertices: A=(1,1), B=(0,1), C=(1,0).
        TriangleOrientation::Reverse => Barycentric {
            a: local_u + local_v - 1.0,
            b: 1.0 - local_u,
            c: 1.0 - local_v,
        },
    }
}

fn classify_visible_face(
    orientation: TriangleOrientation,
    barycentric: Barycentric,
) -> VisibleFace {
    // The minimum barycentric coordinate identifies the triangular face
    // adjacent to the opposite outside edge. Face-to-light assignments are
    // fixed and mirrored between the two alternating triangle orientations.
    let minimum_index =
        if barycentric.a <= barycentric.b
            && barycentric.a <= barycentric.c
        {
            0
        } else if barycentric.b <= barycentric.c {
            1
        } else {
            2
        };

    match (orientation, minimum_index) {
        (TriangleOrientation::Forward, 0) => {
            VisibleFace::Shadow
        }
        (TriangleOrientation::Forward, 1) => {
            VisibleFace::Light
        }
        (TriangleOrientation::Forward, 2) => {
            VisibleFace::Middle
        }

        (TriangleOrientation::Reverse, 0) => {
            VisibleFace::Light
        }
        (TriangleOrientation::Reverse, 1) => {
            VisibleFace::Shadow
        }
        (TriangleOrientation::Reverse, 2) => {
            VisibleFace::Middle
        }

        _ => VisibleFace::Middle,
    }
}

fn internal_seam_distance(
    barycentric: Barycentric,
) -> f32 {
    let difference_ab =
        (barycentric.a - barycentric.b).abs();

    let difference_bc =
        (barycentric.b - barycentric.c).abs();

    let difference_ca =
        (barycentric.c - barycentric.a).abs();

    // Only the centroid-to-vertex half of each equality line is a boundary
    // between the three visible faces. Requiring the remaining coordinate to
    // be greatest suppresses the unwanted centroid-to-edge half.
    let seam_ab =
        if barycentric.c >= barycentric.a
            && barycentric.c >= barycentric.b
        {
            difference_ab
        } else {
            f32::INFINITY
        };

    let seam_bc =
        if barycentric.a >= barycentric.b
            && barycentric.a >= barycentric.c
        {
            difference_bc
        } else {
            f32::INFINITY
        };

    let seam_ca =
        if barycentric.b >= barycentric.c
            && barycentric.b >= barycentric.a
        {
            difference_ca
        } else {
            f32::INFINITY
        };

    seam_ab.min(seam_bc).min(seam_ca)
}

fn seam_strength(
    distance: f32,
    half_width: f32,
    softness: f32,
) -> f32 {
    if !distance.is_finite() {
        return 0.0;
    }

    let inner = (half_width - softness).max(0.0);
    let outer = half_width + softness;

    1.0 - smoothstep(
        inner,
        outer,
        distance,
    )
}

fn smoothstep(
    edge_start: f32,
    edge_end: f32,
    value: f32,
) -> f32 {
    if edge_end <= edge_start {
        return if value < edge_start {
            0.0
        } else {
            1.0
        };
    }

    let amount = ((value - edge_start)
        / (edge_end - edge_start))
        .clamp(0.0, 1.0);

    amount * amount * (3.0 - 2.0 * amount)
}

fn interpolate(
    start: f32,
    end: f32,
    amount: f32,
) -> f32 {
    start + (end - start) * amount.clamp(0.0, 1.0)
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
                MIN_FACET_COUNT,
                MAX_FACET_COUNT,
            ),
            1
        );

        assert_eq!(
            2000_usize.clamp(
                MIN_FACET_COUNT,
                MAX_FACET_COUNT,
            ),
            crate::define_constants::MAX_TEXTURE_PRIMITIVES
        );
    }

    #[test]
    fn layout_preserves_equilateral_triangle_ratio() {
        for requested_count in [1, 8, 72, 288, 1024] {
            let layout = calculate_facet_layout(
                requested_count,
            );

            let ratio = layout.triangle_height
                / layout.triangle_edge;

            assert!(
                (ratio - EQUILATERAL_HEIGHT_RATIO)
                    .abs()
                    < 0.000_001
            );
        }
    }

    #[test]
    fn layout_uses_requested_count_as_density_target() {
        let layout = calculate_facet_layout(
            288,
        );

        assert_eq!(
            layout.requested_count,
            288
        );

        assert!(
            layout.estimated_count.abs_diff(288)
                <= 1
        );
    }

    #[test]
    fn increasing_requested_count_reduces_triangle_size() {
        let coarse = calculate_facet_layout(
            72,
        );

        let fine = calculate_facet_layout(
            288,
        );

        assert!(
            fine.triangle_edge
                < coarse.triangle_edge
        );
    }

    #[test]
    fn triangular_orientations_have_valid_barycentric_coordinates() {
        let forward = barycentric_coordinates(
            TriangleOrientation::Forward,
            0.2,
            0.3,
        );

        let reverse = barycentric_coordinates(
            TriangleOrientation::Reverse,
            0.8,
            0.7,
        );

        assert!(
            (forward.a + forward.b + forward.c - 1.0)
                .abs()
                < 0.0001
        );

        assert!(
            (reverse.a + reverse.b + reverse.c - 1.0)
                .abs()
                < 0.0001
        );
    }

    #[test]
    fn ridge_highlight_is_brightest_at_the_ridge_center() {
        let center = seam_strength(
            0.0,
            RIDGE_HIGHLIGHT_WIDTH,
            RIDGE_HIGHLIGHT_SOFTNESS,
        );

        let shoulder = seam_strength(
            RIDGE_HIGHLIGHT_WIDTH,
            RIDGE_HIGHLIGHT_WIDTH,
            RIDGE_HIGHLIGHT_SOFTNESS,
        );

        let outside = seam_strength(
            RIDGE_HIGHLIGHT_WIDTH
                + RIDGE_HIGHLIGHT_SOFTNESS
                * 2.0,
            RIDGE_HIGHLIGHT_WIDTH,
            RIDGE_HIGHLIGHT_SOFTNESS,
        );

        assert!(center > shoulder);
        assert!(shoulder > outside);
    }

    #[test]
    fn generated_texture_has_standard_dimensions() {
        let texture = generate(
            PaletteColor::new(
            128,
            142,
            156,
        ),
            1,
            288,
        )
        .expect(
            "facet texture generation"
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

