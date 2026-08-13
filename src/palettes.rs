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


    /// Generate a random palette color from an existing SplitMix64 state.
    pub fn random_from_state(
        state: &mut u64,
    ) -> Self {

        let random_value =
            splitmix64(
                state
            );


        Self::new(
            (
                random_value
                    & 0xFF
            ) as u8,
            (
                (
                    random_value >> 8
                )
                    & 0xFF
            ) as u8,
            (
                (
                    random_value >> 16
                )
                    & 0xFF
            ) as u8,
        )
    }


    /// Generate a random palette color from a standalone seed.
    pub fn random_from_seed(
        seed: u64,
    ) -> Self {

        let mut state =
            seed;


        Self::random_from_state(
            &mut state
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
// Curated palette-color helper catalog
// ============================================================
//
// These colors are UI conveniences only.  Screenshaver continues to store and
// process the canonical #rrggbb value; curated names are never configuration
// values.  Each family is intentionally ordered from light to dark.

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
)]
pub enum CuratedColorFamily {
    Grayscale,
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Indigo,
    Violet,
}


impl CuratedColorFamily {

    pub const ALL: [Self; 8] = [
        Self::Grayscale,
        Self::Red,
        Self::Orange,
        Self::Yellow,
        Self::Green,
        Self::Blue,
        Self::Indigo,
        Self::Violet,
    ];


    pub const fn name(
        self,
    ) -> &'static str {

        match self {
            Self::Grayscale => "Grayscale",
            Self::Red => "Red",
            Self::Orange => "Orange",
            Self::Yellow => "Yellow",
            Self::Green => "Green",
            Self::Blue => "Blue",
            Self::Indigo => "Indigo",
            Self::Violet => "Violet",
        }
    }
}


#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
)]
pub struct CuratedPaletteColor {
    pub family: CuratedColorFamily,
    pub name: &'static str,
    pub color: PaletteColor,
}


impl CuratedPaletteColor {

    pub const fn new(
        family: CuratedColorFamily,
        name: &'static str,
        color: PaletteColor,
    ) -> Self {

        Self {
            family,
            name,
            color,
        }
    }
}


pub const CURATED_PALETTE_COLORS: [CuratedPaletteColor; 56] = [
    // Grayscale: light -> dark
    CuratedPaletteColor::new(CuratedColorFamily::Grayscale, "White", PaletteColor::new(255, 255, 255)),
    CuratedPaletteColor::new(CuratedColorFamily::Grayscale, "Pearl", PaletteColor::new(232, 232, 228)),
    CuratedPaletteColor::new(CuratedColorFamily::Grayscale, "Silver", PaletteColor::new(192, 192, 192)),
    CuratedPaletteColor::new(CuratedColorFamily::Grayscale, "Gray", PaletteColor::new(128, 128, 128)),
    CuratedPaletteColor::new(CuratedColorFamily::Grayscale, "Slate Gray", PaletteColor::new(112, 128, 144)),
    CuratedPaletteColor::new(CuratedColorFamily::Grayscale, "Charcoal", PaletteColor::new(70, 70, 70)),
    CuratedPaletteColor::new(CuratedColorFamily::Grayscale, "Graphite", PaletteColor::new(45, 45, 45)),

    // Red: light -> dark
    CuratedPaletteColor::new(CuratedColorFamily::Red, "Blush", PaletteColor::new(255, 183, 197)),
    CuratedPaletteColor::new(CuratedColorFamily::Red, "Salmon", PaletteColor::new(250, 128, 114)),
    CuratedPaletteColor::new(CuratedColorFamily::Red, "Coral Red", PaletteColor::new(255, 82, 82)),
    CuratedPaletteColor::new(CuratedColorFamily::Red, "Scarlet", PaletteColor::new(255, 36, 0)),
    CuratedPaletteColor::new(CuratedColorFamily::Red, "Crimson", PaletteColor::new(220, 20, 60)),
    CuratedPaletteColor::new(CuratedColorFamily::Red, "Ruby", PaletteColor::new(180, 20, 50)),
    CuratedPaletteColor::new(CuratedColorFamily::Red, "Burgundy", PaletteColor::new(128, 0, 32)),

    // Orange: light -> dark
    CuratedPaletteColor::new(CuratedColorFamily::Orange, "Peach", PaletteColor::new(255, 203, 164)),
    CuratedPaletteColor::new(CuratedColorFamily::Orange, "Apricot", PaletteColor::new(251, 174, 96)),
    CuratedPaletteColor::new(CuratedColorFamily::Orange, "Tangerine", PaletteColor::new(242, 133, 0)),
    CuratedPaletteColor::new(CuratedColorFamily::Orange, "Pumpkin", PaletteColor::new(255, 117, 24)),
    CuratedPaletteColor::new(CuratedColorFamily::Orange, "Burnt Orange", PaletteColor::new(204, 85, 0)),
    CuratedPaletteColor::new(CuratedColorFamily::Orange, "Copper", PaletteColor::new(184, 115, 51)),
    CuratedPaletteColor::new(CuratedColorFamily::Orange, "Russet", PaletteColor::new(128, 70, 27)),

    // Yellow: light -> dark
    CuratedPaletteColor::new(CuratedColorFamily::Yellow, "Pale Lemon", PaletteColor::new(255, 250, 170)),
    CuratedPaletteColor::new(CuratedColorFamily::Yellow, "Lemon", PaletteColor::new(255, 244, 79)),
    CuratedPaletteColor::new(CuratedColorFamily::Yellow, "Daffodil", PaletteColor::new(255, 225, 53)),
    CuratedPaletteColor::new(CuratedColorFamily::Yellow, "Golden Yellow", PaletteColor::new(255, 192, 0)),
    CuratedPaletteColor::new(CuratedColorFamily::Yellow, "Honey", PaletteColor::new(218, 165, 32)),
    CuratedPaletteColor::new(CuratedColorFamily::Yellow, "Mustard", PaletteColor::new(181, 148, 16)),
    CuratedPaletteColor::new(CuratedColorFamily::Yellow, "Dark Gold", PaletteColor::new(139, 105, 20)),

    // Green: light -> dark
    CuratedPaletteColor::new(CuratedColorFamily::Green, "Mint", PaletteColor::new(170, 240, 190)),
    CuratedPaletteColor::new(CuratedColorFamily::Green, "Spring Green", PaletteColor::new(95, 210, 120)),
    CuratedPaletteColor::new(CuratedColorFamily::Green, "Grass", PaletteColor::new(76, 175, 80)),
    CuratedPaletteColor::new(CuratedColorFamily::Green, "Emerald", PaletteColor::new(46, 160, 100)),
    CuratedPaletteColor::new(CuratedColorFamily::Green, "Forest", PaletteColor::new(34, 139, 34)),
    CuratedPaletteColor::new(CuratedColorFamily::Green, "Moss", PaletteColor::new(100, 120, 65)),
    CuratedPaletteColor::new(CuratedColorFamily::Green, "Dark Green", PaletteColor::new(20, 90, 45)),

    // Blue: light -> dark
    CuratedPaletteColor::new(CuratedColorFamily::Blue, "Powder Blue", PaletteColor::new(176, 224, 230)),
    CuratedPaletteColor::new(CuratedColorFamily::Blue, "Sky", PaletteColor::new(135, 206, 235)),
    CuratedPaletteColor::new(CuratedColorFamily::Blue, "Azure", PaletteColor::new(70, 160, 230)),
    CuratedPaletteColor::new(CuratedColorFamily::Blue, "Cerulean", PaletteColor::new(0, 123, 167)),
    CuratedPaletteColor::new(CuratedColorFamily::Blue, "Cobalt", PaletteColor::new(0, 71, 171)),
    CuratedPaletteColor::new(CuratedColorFamily::Blue, "Sapphire", PaletteColor::new(15, 82, 186)),
    CuratedPaletteColor::new(CuratedColorFamily::Blue, "Navy", PaletteColor::new(0, 0, 128)),

    // Indigo: light -> dark
    CuratedPaletteColor::new(CuratedColorFamily::Indigo, "Pale Periwinkle", PaletteColor::new(204, 204, 255)),
    CuratedPaletteColor::new(CuratedColorFamily::Indigo, "Periwinkle", PaletteColor::new(160, 160, 230)),
    CuratedPaletteColor::new(CuratedColorFamily::Indigo, "Iris", PaletteColor::new(93, 63, 211)),
    CuratedPaletteColor::new(CuratedColorFamily::Indigo, "Indigo", PaletteColor::new(75, 0, 130)),
    CuratedPaletteColor::new(CuratedColorFamily::Indigo, "Deep Indigo", PaletteColor::new(63, 30, 110)),
    CuratedPaletteColor::new(CuratedColorFamily::Indigo, "Midnight Indigo", PaletteColor::new(45, 35, 95)),
    CuratedPaletteColor::new(CuratedColorFamily::Indigo, "Dark Indigo", PaletteColor::new(35, 20, 70)),

    // Violet: light -> dark
    CuratedPaletteColor::new(CuratedColorFamily::Violet, "Lavender", PaletteColor::new(230, 210, 245)),
    CuratedPaletteColor::new(CuratedColorFamily::Violet, "Mauve", PaletteColor::new(210, 160, 220)),
    CuratedPaletteColor::new(CuratedColorFamily::Violet, "Orchid", PaletteColor::new(218, 112, 214)),
    CuratedPaletteColor::new(CuratedColorFamily::Violet, "Violet", PaletteColor::new(143, 0, 255)),
    CuratedPaletteColor::new(CuratedColorFamily::Violet, "Amethyst", PaletteColor::new(153, 102, 204)),
    CuratedPaletteColor::new(CuratedColorFamily::Violet, "Plum", PaletteColor::new(142, 69, 133)),
    CuratedPaletteColor::new(CuratedColorFamily::Violet, "Dark Violet", PaletteColor::new(90, 35, 105)),
];


pub fn curated_colors_for_family(
    family: CuratedColorFamily,
) -> impl Iterator<Item = &'static CuratedPaletteColor> {

    CURATED_PALETTE_COLORS
        .iter()
        .filter(
            move |entry| {
                entry.family == family
            }
        )
}


pub fn curated_color_for_palette(
    color: PaletteColor,
) -> Option<&'static CuratedPaletteColor> {

    CURATED_PALETTE_COLORS
        .iter()
        .find(
            |entry| {
                entry.color == color
            }
        )
}


// ============================================================
// Palette display helpers
// ============================================================

impl PaletteColor {

    pub fn name(
        self,
    ) -> String {

        self.to_hex()
    }
}


fn splitmix64(
    state: &mut u64,
) -> u64 {

    *state =
        state.wrapping_add(
            0x9E37_79B9_7F4A_7C15
        );


    let mut value =
        *state;


    value =
        (
            value
                ^ (
                    value >> 30
                )
        )
        .wrapping_mul(
            0xBF58_476D_1CE4_E5B9
        );


    value =
        (
            value
                ^ (
                    value >> 27
                )
        )
        .wrapping_mul(
            0x94D0_49BB_1331_11EB
        );


    value
        ^ (
            value >> 31
        )
}


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
    fn curated_catalog_contains_seven_colors_per_family() {

        assert_eq!(
            CURATED_PALETTE_COLORS.len(),
            56
        );


        for family in
            CuratedColorFamily::ALL
        {
            assert_eq!(
                curated_colors_for_family(
                    family
                )
                .count(),
                7,
                "unexpected curated color count for {}",
                family.name(),
            );
        }
    }


    #[test]
    fn curated_catalog_lookup_matches_exact_palette_color() {

        let pumpkin =
            curated_color_for_palette(
                PaletteColor::new(
                    255,
                    117,
                    24,
                )
            )
            .expect(
                "Pumpkin should be present in curated catalog"
            );


        assert_eq!(
            pumpkin.name,
            "Pumpkin"
        );

        assert_eq!(
            pumpkin.family,
            CuratedColorFamily::Orange
        );


        assert!(
            curated_color_for_palette(
                PaletteColor::new(
                    1,
                    2,
                    3,
                )
            )
            .is_none()
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

