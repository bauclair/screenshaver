use std::fmt;
use std::str::FromStr;

use crate::generate_textures::TextureFamily;


pub const DEFAULT_PRIMITIVE_COUNT: usize =
    crate::define_constants::MIN_TEXTURE_PRIMITIVES;


#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
)]
pub struct TextureSpecification {

    pub family:
        TextureFamily,

    pub requested_primitive_count:
        usize,

    pub count_was_explicit:
        bool,
}


impl TextureSpecification {

    pub fn parse(
        value: &str,
    ) -> Result<Self, String> {

        parse_texture_specification(
            value
        )
    }


    /// Return a user-facing texture name for logs and overlay pills.
    ///
    /// The canonical `Display` implementation remains suitable for
    /// configuration and command-line text:
    ///
    /// - `hexagons`
    /// - `hexagons:144`
    ///
    /// This method instead produces presentation text:
    ///
    /// - `Hexagons`
    /// - `Hexagons (144)`
    pub fn display_name(
        &self,
    ) -> String {

        let family_name =
            title_case_family_name(
                self.family.name()
            );


        if self.count_was_explicit {

            format!(
                "{} ({})",
                family_name,
                self.requested_primitive_count,
            )

        } else {

            family_name
        }
    }
}


impl FromStr for TextureSpecification {

    type Err =
        String;


    fn from_str(
        value: &str,
    ) -> Result<Self, Self::Err> {

        parse_texture_specification(
            value
        )
    }
}


impl fmt::Display for TextureSpecification {

    fn fmt(
        &self,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {

        if self.count_was_explicit {

            write!(
                formatter,
                "{}:{}",
                self.family.name(),
                self.requested_primitive_count,
            )

        } else {

            formatter.write_str(
                self.family.name()
            )
        }
    }
}


pub fn parse_texture_specification(
    value: &str,
) -> Result<TextureSpecification, String> {

    let value =
        value.trim();


    if value.is_empty() {

        return Err(
            "Texture specification cannot be empty"
                .to_string()
        );
    }


    let separator_count =
        value.matches(':')
            .count();


    if separator_count
        > 1
    {
        return Err(
            format!(
                "Invalid texture specification '{}'; use FAMILY or FAMILY:COUNT",
                value
            )
        );
    }


    let (
        family_text,
        count_text,
    ) =
        match value.split_once(':') {

            Some(
                (
                    family,
                    count,
                )
            ) => {
                (
                    family.trim(),
                    Some(
                        count.trim()
                    ),
                )
            }


            None => {
                (
                    value,
                    None,
                )
            }
        };


    if family_text.is_empty() {

        return Err(
            format!(
                "Invalid texture specification '{}'; the texture family is missing",
                value
            )
        );
    }


    let family =
        TextureFamily::from_str(
            family_text
        )
        .map_err(
            |error| {
                format!(
                    "Invalid texture specification '{}': {}",
                    value,
                    error,
                )
            }
        )?;


    let (
        requested_primitive_count,
        count_was_explicit,
    ) =
        match count_text {

            None => {
                (
                    DEFAULT_PRIMITIVE_COUNT,
                    false,
                )
            }


            Some(
                ""
            ) => {
                return Err(
                    format!(
                        "Invalid texture specification '{}'; a primitive count must follow ':'",
                        value
                    )
                );
            }


            Some(
                count
            ) => {

                if !count
                    .chars()
                    .all(
                        |character| {
                            character.is_ascii_digit()
                        }
                    )
                {
                    return Err(
                        format!(
                            "Invalid primitive count '{}' in texture specification '{}'; the count must be a positive integer",
                            count,
                            value,
                        )
                    );
                }


                let parsed_count =
                    count
                        .parse::<usize>()
                        .map_err(
                            |_| {
                                format!(
                                    "Primitive count '{}' in texture specification '{}' is too large",
                                    count,
                                    value,
                                )
                            }
                        )?;


                if !(
                    crate::define_constants::MIN_TEXTURE_PRIMITIVES
                        ..=
                    crate::define_constants::MAX_TEXTURE_PRIMITIVES
                )
                    .contains(
                        &parsed_count
                    )
                {
                    return Err(
                        format!(
                            "Invalid primitive count {} in texture specification '{}'; the supported range is {}-{}",
                            parsed_count,
                            value,
                            crate::define_constants::MIN_TEXTURE_PRIMITIVES,
                            crate::define_constants::MAX_TEXTURE_PRIMITIVES,
                        )
                    );
                }


                (
                    parsed_count,
                    true,
                )
            }
        };


    Ok(
        TextureSpecification {
            family,
            requested_primitive_count,
            count_was_explicit,
        }
    )
}


fn title_case_family_name(
    family_name: &str,
) -> String {

    let mut characters =
        family_name.chars();


    match characters.next() {

        Some(
            first_character
        ) => {

            first_character
                .to_uppercase()
                .chain(
                    characters
                )
                .collect()
        }


        None => {
            String::new()
        }
    }
}


#[cfg(test)]
mod tests {

    use super::*;


    #[test]
    fn texture_without_count_uses_default_primitive_count() {

        let specification =
            parse_texture_specification(
                "hexagons"
            )
            .expect(
                "The texture specification should parse"
            );


        assert_eq!(
            specification.family,
            TextureFamily::Hexagons
        );

        assert_eq!(
            specification.requested_primitive_count,
            DEFAULT_PRIMITIVE_COUNT
        );

        assert!(
            !specification.count_was_explicit
        );
    }


    #[test]
    fn texture_with_count_preserves_count() {

        let specification =
            parse_texture_specification(
                "hexagons:144"
            )
            .expect(
                "The texture specification should parse"
            );


        assert_eq!(
            specification.family,
            TextureFamily::Hexagons
        );

        assert_eq!(
            specification.requested_primitive_count,
            144
        );

        assert!(
            specification.count_was_explicit
        );
    }


    #[test]
    fn texture_family_is_case_insensitive() {

        let specification =
            parse_texture_specification(
                "HeXaGoNs:32"
            )
            .expect(
                "The texture specification should parse"
            );


        assert_eq!(
            specification.family,
            TextureFamily::Hexagons
        );

        assert_eq!(
            specification.requested_primitive_count,
            32
        );
    }


    #[test]
    fn surrounding_whitespace_is_ignored() {

        let specification =
            parse_texture_specification(
                "  hexagons : 144  "
            )
            .expect(
                "The texture specification should parse"
            );


        assert_eq!(
            specification.family,
            TextureFamily::Hexagons
        );

        assert_eq!(
            specification.requested_primitive_count,
            144
        );

        assert!(
            specification.count_was_explicit
        );
    }


    #[test]
    fn canonical_display_omits_defaulted_count() {

        let specification =
            parse_texture_specification(
                "hexagons"
            )
            .expect(
                "The texture specification should parse"
            );


        assert_eq!(
            specification.to_string(),
            "hexagons"
        );
    }


    #[test]
    fn canonical_display_preserves_explicit_count() {

        let specification =
            parse_texture_specification(
                "hexagons:144"
            )
            .expect(
                "The texture specification should parse"
            );


        assert_eq!(
            specification.to_string(),
            "hexagons:144"
        );
    }


    #[test]
    fn display_name_omits_defaulted_count() {

        let specification =
            parse_texture_specification(
                "hexagons"
            )
            .expect(
                "The texture specification should parse"
            );


        assert_eq!(
            specification.display_name(),
            "Hexagons"
        );
    }


    #[test]
    fn display_name_includes_explicit_count() {

        let specification =
            parse_texture_specification(
                "hexagons:144"
            )
            .expect(
                "The texture specification should parse"
            );


        assert_eq!(
            specification.display_name(),
            "Hexagons (144)"
        );
    }


    #[test]
    fn rejects_empty_texture_specification() {

        assert!(
            parse_texture_specification(
                ""
            )
            .is_err()
        );
    }


    #[test]
    fn rejects_missing_family() {

        assert!(
            parse_texture_specification(
                ":144"
            )
            .is_err()
        );
    }


    #[test]
    fn rejects_missing_count() {

        assert!(
            parse_texture_specification(
                "hexagons:"
            )
            .is_err()
        );
    }


    #[test]
    fn rejects_zero_count() {

        assert!(
            parse_texture_specification(
                "hexagons:0"
            )
            .is_err()
        );
    }


    #[test]
    fn rejects_negative_count() {

        assert!(
            parse_texture_specification(
                "hexagons:-4"
            )
            .is_err()
        );
    }


    #[test]
    fn rejects_decimal_count() {

        assert!(
            parse_texture_specification(
                "hexagons:12.5"
            )
            .is_err()
        );
    }


    #[test]
    fn accepts_maximum_primitive_count() {

        let specification =
            parse_texture_specification(
                &format!(
                    "hexagons:{}",
                    crate::define_constants::MAX_TEXTURE_PRIMITIVES,
                )
            )
            .expect(
                "The maximum primitive count should parse"
            );


        assert_eq!(
            specification.requested_primitive_count,
            crate::define_constants::MAX_TEXTURE_PRIMITIVES
        );
    }


    #[test]
    fn rejects_count_above_maximum() {

        let value =
            format!(
                "hexagons:{}",
                crate::define_constants::MAX_TEXTURE_PRIMITIVES
                    + 1,
            );


        assert!(
            parse_texture_specification(
                &value
            )
            .is_err()
        );
    }


    #[test]
    fn rejects_multiple_separators() {

        assert!(
            parse_texture_specification(
                "hexagons:12:4"
            )
            .is_err()
        );
    }


    #[test]
    fn rejects_unknown_texture_family() {

        assert!(
            parse_texture_specification(
                "unknown:12"
            )
            .is_err()
        );
    }
}

