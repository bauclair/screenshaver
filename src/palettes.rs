//! Color palettes for the Screenshaver Procedural Texture Engine.
//!
//! Texture generators produce normalized scalar values in the range
//! 0.0 through 1.0.  This module converts those values into RGBA8 colors.
//!
//! PaletteColor is the canonical #rrggbb color type.
//! Texture intensity is mapped to tonal variation automatically in HSV space.
//! No predefined palette-color set is retained.
//!
//! The texture-generating algorithms define structure.
//! Palette colors define hue and mood.

use std::fmt;
use std::str::FromStr;


// ============================================================
// Canonical RGB / hexadecimal color representation
// ============================================================

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
)]
pub struct PaletteColor {
    red: u8,
    green: u8,
    blue: u8,
}


impl PaletteColor {

    pub const fn new(
        red: u8,
        green: u8,
        blue: u8,
    ) -> Self {

        Self {
            red,
            green,
            blue,
        }
    }


    pub const fn red(
        self,
    ) -> u8 {

        self.red
    }


    pub const fn green(
        self,
    ) -> u8 {

        self.green
    }


    pub const fn blue(
        self,
    ) -> u8 {

        self.blue
    }


    pub const fn rgb(
        self,
    ) -> [u8; 3] {

        [
            self.red,
            self.green,
            self.blue,
        ]
    }


    pub fn to_hex(
        self,
    ) -> String {

        format!(
            "#{:02x}{:02x}{:02x}",
            self.red,
            self.green,
            self.blue,
        )
    }


    pub fn parse_hex(
        value: &str,
    ) -> Result<Self, String> {

        let value =
            value.trim();


        if value.len() != 7
            || !value.starts_with('#')
        {
            return Err(
                format!(
                    "Invalid color '{}'; expected hexadecimal notation #rrggbb",
                    value,
                )
            );
        }


        let digits =
            &value[1..];


        if !digits
            .chars()
            .all(
                |character| {
                    character.is_ascii_hexdigit()
                }
            )
        {
            return Err(
                format!(
                    "Invalid color '{}'; expected hexadecimal notation #rrggbb",
                    value,
                )
            );
        }


        let red =
            u8::from_str_radix(
                &digits[0..2],
                16,
            )
            .map_err(
                |_| {
                    format!(
                        "Invalid red component in color '{}'",
                        value,
                    )
                }
            )?;


        let green =
            u8::from_str_radix(
                &digits[2..4],
                16,
            )
            .map_err(
                |_| {
                    format!(
                        "Invalid green component in color '{}'",
                        value,
                    )
                }
            )?;


        let blue =
            u8::from_str_radix(
                &digits[4..6],
                16,
            )
            .map_err(
                |_| {
                    format!(
                        "Invalid blue component in color '{}'",
                        value,
                    )
                }
            )?;


        Ok(
            Self::new(
                red,
                green,
                blue,
            )
        )
    }


    /// Convert a normalized texture intensity into a shade of this base color.
    ///
    /// The user's RGB choice is treated as the nominal/base color.  Screenshaver
    /// converts it to HSV internally, keeps the hue, and derives darker/lighter
    /// tonal values from the procedural texture intensity.  HSV is intentionally
    /// an implementation detail rather than a user-facing setting.
    pub fn map_rgba(
        self,
        value: f32,
    ) -> [u8; 4] {

        let normalized =
            normalize_value(
                value
            );


        let (
            hue,
            saturation,
            base_value,
        ) =
            rgb_to_hsv(
                self
            );


        let value_multiplier =
            tonal_value_multiplier(
                normalized
            );


        let saturation_multiplier =
            tonal_saturation_multiplier(
                normalized
            );


        let mapped_value =
            (
                base_value
                    * value_multiplier
            )
            .clamp(
                0.0,
                1.0,
            );


        let mapped_saturation =
            (
                saturation
                    * saturation_multiplier
            )
            .clamp(
                0.0,
                1.0,
            );


        let mapped_color =
            hsv_to_rgb(
                hue,
                mapped_saturation,
                mapped_value,
            );


        [
            mapped_color.red,
            mapped_color.green,
            mapped_color.blue,
            255,
        ]
    }


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
}


impl FromStr for PaletteColor {

    type Err =
        String;


    fn from_str(
        value: &str,
    ) -> Result<Self, Self::Err> {

        Self::parse_hex(
            value
        )
    }
}


impl fmt::Display for PaletteColor {

    fn fmt(
        &self,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {

        formatter.write_str(
            &self.to_hex()
        )
    }
}


// ============================================================
// Transitional Palette compatibility
// ============================================================
//
// PaletteColor is the authoritative palette type.  The Palette alias and
// name() display helper remain temporarily so texture-generator and editor
// call sites can be renamed independently of this cleanup.

impl PaletteColor {

    pub fn name(
        self,
    ) -> String {

        self.to_hex()
    }
}


/// Transitional source-level alias.  `Palette` and `PaletteColor` are the
/// exact same Rust type.
pub type Palette =
    PaletteColor;


// ============================================================
// Generated tonal curve
// ============================================================
//
// The procedural texture intensity still ranges from 0.0 to 1.0.  Instead of
// interpolating six hard-coded RGB colors, Screenshaver now interpolates six
// tonal multipliers around the user's base color.
//
// The selected/base RGB value is reproduced at intensity 0.64.
// Darker intensity values reduce HSV Value; brighter values increase it.
// Saturation is gently reduced toward highlights so light regions do not become
// unnaturally vivid.
//
// These constants are deliberately centralized so the visual character of the
// generated ramp can be tuned later without changing the public color model.

const TONAL_POSITIONS: [f32; 6] = [
    0.00,
    0.20,
    0.42,
    0.64,
    0.84,
    1.00,
];


const TONAL_VALUE_MULTIPLIERS: [f32; 6] = [
    0.28,
    0.48,
    0.68,
    1.00,
    1.18,
    1.35,
];


const TONAL_SATURATION_MULTIPLIERS: [f32; 6] = [
    1.08,
    1.06,
    1.03,
    1.00,
    0.94,
    0.86,
];


fn tonal_value_multiplier(
    value: f32,
) -> f32 {

    interpolate_curve(
        value,
        &TONAL_POSITIONS,
        &TONAL_VALUE_MULTIPLIERS,
    )
}


fn tonal_saturation_multiplier(
    value: f32,
) -> f32 {

    interpolate_curve(
        value,
        &TONAL_POSITIONS,
        &TONAL_SATURATION_MULTIPLIERS,
    )
}


fn interpolate_curve(
    value: f32,
    positions: &[f32],
    values: &[f32],
) -> f32 {

    debug_assert_eq!(
        positions.len(),
        values.len()
    );


    if positions.is_empty()
        || values.is_empty()
    {
        return 1.0;
    }


    if value
        <= positions[0]
    {
        return values[0];
    }


    let last_index =
        positions.len() - 1;


    if value
        >= positions[last_index]
    {
        return values[last_index];
    }


    for index in
        0..last_index
    {
        let lower_position =
            positions[index];

        let upper_position =
            positions[index + 1];


        if value
            > upper_position
        {
            continue;
        }


        let span =
            upper_position
                - lower_position;


        let amount =
            if span
                <= f32::EPSILON
            {
                0.0
            } else {
                (
                    value
                        - lower_position
                )
                    / span
            };


        return values[index]
            + (
                values[index + 1]
                    - values[index]
            )
                * amount.clamp(
                    0.0,
                    1.0,
                );
    }


    values[last_index]
}


// ============================================================
// RGB <-> HSV conversion
// ============================================================

fn rgb_to_hsv(
    color: PaletteColor,
) -> (
    f32,
    f32,
    f32,
) {

    let red =
        color.red as f32
            / 255.0;

    let green =
        color.green as f32
            / 255.0;

    let blue =
        color.blue as f32
            / 255.0;


    let maximum =
        red.max(
            green.max(
                blue
            )
        );

    let minimum =
        red.min(
            green.min(
                blue
            )
        );

    let delta =
        maximum - minimum;


    let hue =
        if delta
            <= f32::EPSILON
        {
            0.0

        } else if maximum
            == red
        {
            (
                (
                    green - blue
                )
                    / delta
            )
                .rem_euclid(
                    6.0
                )
                / 6.0

        } else if maximum
            == green
        {
            (
                (
                    blue - red
                )
                    / delta
                    + 2.0
            )
                / 6.0

        } else {
            (
                (
                    red - green
                )
                    / delta
                    + 4.0
            )
                / 6.0
        };


    let saturation =
        if maximum
            <= f32::EPSILON
        {
            0.0
        } else {
            delta
                / maximum
        };


    (
        hue,
        saturation,
        maximum,
    )
}


fn hsv_to_rgb(
    hue: f32,
    saturation: f32,
    value: f32,
) -> PaletteColor {

    let hue =
        hue.rem_euclid(
            1.0
        );

    let saturation =
        saturation.clamp(
            0.0,
            1.0,
        );

    let value =
        value.clamp(
            0.0,
            1.0,
        );


    if saturation
        <= f32::EPSILON
    {
        let channel =
            float_channel_to_u8(
                value
            );


        return PaletteColor::new(
            channel,
            channel,
            channel,
        );
    }


    let scaled_hue =
        hue * 6.0;

    let sector =
        scaled_hue.floor() as i32;

    let fraction =
        scaled_hue
            - sector as f32;


    let p =
        value
            * (
                1.0 - saturation
            );

    let q =
        value
            * (
                1.0
                    - saturation
                        * fraction
            );

    let t =
        value
            * (
                1.0
                    - saturation
                        * (
                            1.0 - fraction
                        )
            );


    let (
        red,
        green,
        blue,
    ) =
        match sector.rem_euclid(6) {

            0 => (
                value,
                t,
                p,
            ),

            1 => (
                q,
                value,
                p,
            ),

            2 => (
                p,
                value,
                t,
            ),

            3 => (
                p,
                q,
                value,
            ),

            4 => (
                t,
                p,
                value,
            ),

            _ => (
                value,
                p,
                q,
            ),
        };


    PaletteColor::new(
        float_channel_to_u8(
            red
        ),
        float_channel_to_u8(
            green
        ),
        float_channel_to_u8(
            blue
        ),
    )
}


fn float_channel_to_u8(
    value: f32,
) -> u8 {

    (
        value.clamp(
            0.0,
            1.0,
        )
            * 255.0
    )
        .round()
        .clamp(
            0.0,
            255.0,
        )
            as u8
}


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


// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {

    use super::*;


    const TEST_COLORS: [PaletteColor; 6] = [
        PaletteColor::new(
            0,
            0,
            0,
        ),
        PaletteColor::new(
            255,
            255,
            255,
        ),
        PaletteColor::new(
            255,
            0,
            0,
        ),
        PaletteColor::new(
            0,
            128,
            255,
        ),
        PaletteColor::new(
            183,
            201,
            154,
        ),
        PaletteColor::new(
            92,
            47,
            131,
        ),
    ];


    #[test]
    fn parses_hex_colors_case_insensitively() {

        assert_eq!(
            PaletteColor::parse_hex(
                "#b7c99a"
            ),
            Ok(
                PaletteColor::new(
                    183,
                    201,
                    154,
                )
            )
        );


        assert_eq!(
            PaletteColor::parse_hex(
                "#B7C99A"
            ),
            Ok(
                PaletteColor::new(
                    183,
                    201,
                    154,
                )
            )
        );
    }


    #[test]
    fn formats_hex_colors_canonically() {

        assert_eq!(
            PaletteColor::new(
                183,
                201,
                154,
            )
            .to_hex(),
            "#b7c99a"
        );
    }


    #[test]
    fn rejects_invalid_hex_colors() {

        for invalid in [
            "b7c99a",
            "#fff",
            "#b7c99aff",
            "#zzc99a",
            "",
        ] {
            assert!(
                PaletteColor::parse_hex(
                    invalid
                )
                .is_err(),
                "unexpectedly accepted {}",
                invalid,
            );
        }
    }


    #[test]
    fn base_color_is_reproduced_at_nominal_tonal_position() {

        for palette in
            TEST_COLORS
        {
            assert_eq!(
                palette.map_rgb(
                    0.64
                ),
                palette
                    .rgb(),
            );
        }
    }


    #[test]
    fn tonal_mapping_preserves_full_opacity() {

        for palette in
            TEST_COLORS
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
    fn tonal_mapping_clamps_intensity() {

        let below =
            PaletteColor::new(
                112,
                123,
                82,
            ).map_rgba(
                -10.0
            );

        let minimum =
            PaletteColor::new(
                112,
                123,
                82,
            ).map_rgba(
                0.0
            );

        let above =
            PaletteColor::new(
                112,
                123,
                82,
            ).map_rgba(
                10.0
            );

        let maximum =
            PaletteColor::new(
                112,
                123,
                82,
            ).map_rgba(
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
    fn non_finite_intensity_maps_to_zero() {

        assert_eq!(
            PaletteColor::new(
                147,
                99,
                59,
            ).map_rgba(
                f32::NAN
            ),
            PaletteColor::new(
                147,
                99,
                59,
            ).map_rgba(
                0.0
            )
        );
    }


    #[test]
    fn darker_intensity_produces_lower_hsv_value_than_brighter_intensity() {

        let color =
            PaletteColor::new(
                183,
                201,
                154,
            );


        let dark =
            color.map_rgb(
                0.20
            );

        let bright =
            color.map_rgb(
                0.84
            );


        let dark_value =
            *dark.iter()
                .max()
                .unwrap();

        let bright_value =
            *bright.iter()
                .max()
                .unwrap();


        assert!(
            dark_value
                < bright_value
        );
    }
}

