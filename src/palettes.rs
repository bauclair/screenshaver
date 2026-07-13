//! Color palettes for the Screenshaver Procedural Texture
//! Engine.
//!
//! Texture generators should produce normalized scalar values
//! in the range 0.0 through 1.0. This module converts those
//! values into restrained RGBA8 colors.
//!
//! The texture-generating algorithms define structure.
//! Palettes define color and mood.

use std::fmt;
use std::str::FromStr;


// ============================================================
// Public palette definitions
// ============================================================

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
)]
pub enum Palette {
    Slate,
    Sandstone,
    Lichen,
    Mist,
    Bronze,
}


impl Palette {

    /// Every currently supported palette.
    pub const ALL: [Palette; 5] = [
        Palette::Slate,
        Palette::Sandstone,
        Palette::Lichen,
        Palette::Mist,
        Palette::Bronze,
    ];


    /// Return the canonical lowercase configuration and
    /// command-line name.
    pub const fn name(
        self,
    ) -> &'static str {

        match self {

            Palette::Slate => {
                "slate"
            }

            Palette::Sandstone => {
                "sandstone"
            }

            Palette::Lichen => {
                "lichen"
            }

            Palette::Mist => {
                "mist"
            }

            Palette::Bronze => {
                "bronze"
            }
        }
    }


    /// Convert a normalized scalar value to an RGBA8 color.
    ///
    /// Values outside 0.0 through 1.0 are clamped.
    /// Non-finite values are treated as zero.
    pub fn map_rgba(
        self,
        value: f32,
    ) -> [u8; 4] {

        let normalized =
            normalize_value(
                value
            );


        let stops =
            self.stops();


        interpolate_stops(
            stops,
            normalized,
        )
    }


    /// Return only the RGB components.
    pub fn map_rgb(
        self,
        value: f32,
    ) -> [u8; 3] {

        let rgba =
            self.map_rgba(
                value
            );


        [
            rgba[0],
            rgba[1],
            rgba[2],
        ]
    }


    /// Parse a palette name without requiring callers to use
    /// the FromStr trait explicitly.
    pub fn from_name(
        value: &str,
    ) -> Result<Self, String> {

        value.parse()
    }


    /// Names suitable for help text and diagnostics.
    pub fn names() -> Vec<&'static str> {

        Self::ALL
            .iter()
            .map(
                |palette| {
                    palette.name()
                }
            )
            .collect()
    }


    /// Color stops for this palette.
    ///
    /// Each stop consists of:
    ///
    ///     (
    ///         normalized position,
    ///         red,
    ///         green,
    ///         blue,
    ///     )
    ///
    /// Positions must remain sorted from 0.0 to 1.0.
    fn stops(
        self,
    ) -> &'static [ColorStop] {

        match self {

            Palette::Slate => {
                &SLATE_STOPS
            }

            Palette::Sandstone => {
                &SANDSTONE_STOPS
            }

            Palette::Lichen => {
                &LICHEN_STOPS
            }

            Palette::Mist => {
                &MIST_STOPS
            }

            Palette::Bronze => {
                &BRONZE_STOPS
            }
        }
    }
}


// ============================================================
// Name parsing and display
// ============================================================

impl FromStr for Palette {

    type Err = String;


    fn from_str(
        value: &str,
    ) -> Result<Self, Self::Err> {

        let normalized =
            value
                .trim()
                .to_ascii_lowercase();


        match normalized.as_str() {

            "slate" => {
                Ok(
                    Palette::Slate
                )
            }

            "sandstone" => {
                Ok(
                    Palette::Sandstone
                )
            }

            "lichen" => {
                Ok(
                    Palette::Lichen
                )
            }

            "mist" => {
                Ok(
                    Palette::Mist
                )
            }

            "bronze" => {
                Ok(
                    Palette::Bronze
                )
            }

            _ => {
                Err(
                    format!(
                        "Unknown texture palette '{}'. Valid palettes: {}",
                        value,
                        Palette::names()
                            .join(", "),
                    )
                )
            }
        }
    }
}


impl fmt::Display for Palette {

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
// Internal color representation
// ============================================================

#[derive(
    Debug,
    Clone,
    Copy,
)]
struct ColorStop {
    position: f32,
    red: u8,
    green: u8,
    blue: u8,
}


impl ColorStop {

    const fn new(
        position: f32,
        red: u8,
        green: u8,
        blue: u8,
    ) -> Self {

        Self {
            position,
            red,
            green,
            blue,
        }
    }
}


// ============================================================
// Palette color stops
// ============================================================

/// Cool charcoal, blue-gray, steel, and silver.
const SLATE_STOPS: [ColorStop; 6] = [

    ColorStop::new(
        0.00,
        18,
        22,
        27,
    ),

    ColorStop::new(
        0.20,
        39,
        49,
        59,
    ),

    ColorStop::new(
        0.42,
        66,
        82,
        96,
    ),

    ColorStop::new(
        0.64,
        99,
        119,
        134,
    ),

    ColorStop::new(
        0.84,
        145,
        160,
        171,
    ),

    ColorStop::new(
        1.00,
        205,
        214,
        220,
    ),
];


/// Warm earth, beige, stone, and muted gold.
const SANDSTONE_STOPS: [ColorStop; 6] = [

    ColorStop::new(
        0.00,
        38,
        31,
        25,
    ),

    ColorStop::new(
        0.20,
        75,
        60,
        44,
    ),

    ColorStop::new(
        0.42,
        119,
        94,
        65,
    ),

    ColorStop::new(
        0.64,
        161,
        130,
        91,
    ),

    ColorStop::new(
        0.84,
        198,
        171,
        128,
    ),

    ColorStop::new(
        1.00,
        228,
        212,
        177,
    ),
];


/// Deep moss, olive-gray, weathered stone, and pale growth.
const LICHEN_STOPS: [ColorStop; 6] = [

    ColorStop::new(
        0.00,
        23,
        28,
        22,
    ),

    ColorStop::new(
        0.20,
        48,
        58,
        42,
    ),

    ColorStop::new(
        0.42,
        79,
        91,
        61,
    ),

    ColorStop::new(
        0.64,
        112,
        123,
        82,
    ),

    ColorStop::new(
        0.84,
        151,
        158,
        111,
    ),

    ColorStop::new(
        1.00,
        199,
        201,
        160,
    ),
];


/// Soft blue-gray, muted lavender, fog, and pale cream.
const MIST_STOPS: [ColorStop; 6] = [

    ColorStop::new(
        0.00,
        28,
        31,
        38,
    ),

    ColorStop::new(
        0.20,
        55,
        62,
        75,
    ),

    ColorStop::new(
        0.42,
        89,
        101,
        117,
    ),

    ColorStop::new(
        0.64,
        128,
        142,
        156,
    ),

    ColorStop::new(
        0.84,
        171,
        181,
        188,
    ),

    ColorStop::new(
        1.00,
        221,
        219,
        207,
    ),
];


/// Dark umber, weathered copper, bronze, and muted highlights.
const BRONZE_STOPS: [ColorStop; 6] = [

    ColorStop::new(
        0.00,
        30,
        23,
        20,
    ),

    ColorStop::new(
        0.20,
        64,
        43,
        32,
    ),

    ColorStop::new(
        0.42,
        104,
        67,
        43,
    ),

    ColorStop::new(
        0.64,
        147,
        99,
        59,
    ),

    ColorStop::new(
        0.84,
        183,
        138,
        87,
    ),

    ColorStop::new(
        1.00,
        218,
        190,
        142,
    ),
];


// ============================================================
// Color interpolation
// ============================================================

fn normalize_value(
    value: f32,
) -> f32 {

    if value.is_finite() {

        value.clamp(
            0.0,
            1.0,
        )

    } else {

        0.0
    }
}


fn interpolate_stops(
    stops: &[ColorStop],
    value: f32,
) -> [u8; 4] {

    if stops.is_empty() {

        return [
            0,
            0,
            0,
            255,
        ];
    }


    if value
        <= stops[0].position
    {
        return color_stop_rgba(
            stops[0]
        );
    }


    let last =
        stops[
            stops.len() - 1
        ];


    if value
        >= last.position
    {
        return color_stop_rgba(
            last
        );
    }


    for pair in
        stops.windows(
            2
        )
    {
        let lower =
            pair[0];

        let upper =
            pair[1];


        if value
            > upper.position
        {
            continue;
        }


        let span =
            upper.position
                - lower.position;


        let amount =
            if span
                <= f32::EPSILON
            {
                0.0

            } else {

                (
                    value
                        - lower.position
                )
                / span
            };


        return [
            interpolate_channel(
                lower.red,
                upper.red,
                amount,
            ),

            interpolate_channel(
                lower.green,
                upper.green,
                amount,
            ),

            interpolate_channel(
                lower.blue,
                upper.blue,
                amount,
            ),

            255,
        ];
    }


    color_stop_rgba(
        last
    )
}


fn interpolate_channel(
    start: u8,
    end: u8,
    amount: f32,
) -> u8 {

    let start =
        start as f32;

    let end =
        end as f32;


    (
        start
            + (
                end - start
            )
            * amount.clamp(
                0.0,
                1.0,
            )
    )
    .round()
    .clamp(
        0.0,
        255.0,
    )
        as u8
}


fn color_stop_rgba(
    stop: ColorStop,
) -> [u8; 4] {

    [
        stop.red,
        stop.green,
        stop.blue,
        255,
    ]
}


// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {

    use super::*;


    #[test]
    fn parses_palette_names_case_insensitively() {

        assert_eq!(
            "slate".parse::<Palette>(),
            Ok(
                Palette::Slate
            )
        );


        assert_eq!(
            "SANDSTONE".parse::<Palette>(),
            Ok(
                Palette::Sandstone
            )
        );


        assert_eq!(
            "  Mist  ".parse::<Palette>(),
            Ok(
                Palette::Mist
            )
        );
    }


    #[test]
    fn rejects_unknown_palette_names() {

        assert!(
            "rainbow"
                .parse::<Palette>()
                .is_err()
        );
    }


    #[test]
    fn clamps_values_to_palette_range() {

        let below =
            Palette::Slate.map_rgba(
                -10.0
            );


        let minimum =
            Palette::Slate.map_rgba(
                0.0
            );


        let above =
            Palette::Slate.map_rgba(
                10.0
            );


        let maximum =
            Palette::Slate.map_rgba(
                1.0
            );


        assert_eq!(
            below,
            minimum
        );


        assert_eq!(
            above,
            maximum
        );
    }


    #[test]
    fn colors_are_fully_opaque() {

        for palette in
            Palette::ALL
        {
            for value in [
                0.0,
                0.25,
                0.5,
                0.75,
                1.0,
            ] {
                assert_eq!(
                    palette
                        .map_rgba(
                            value
                        )[3],
                    255,
                );
            }
        }
    }


    #[test]
    fn non_finite_values_map_to_zero() {

        assert_eq!(
            Palette::Bronze.map_rgba(
                f32::NAN
            ),
            Palette::Bronze.map_rgba(
                0.0
            )
        );
    }
}