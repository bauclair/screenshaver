use std::collections::HashMap;
use std::sync::OnceLock;

use rusqlite::{
    params,
    Connection,
    OptionalExtension,
};


pub const DEFAULT_LOCALE: &str =
    "en-US";


static RUNTIME_TEXT:
    OnceLock<HashMap<String, String>> =
        OnceLock::new();


pub fn initialize_runtime(
    connection: &Connection,
    requested_locale: &str,
) -> Result<(), String> {

    if RUNTIME_TEXT.get().is_some() {
        return Ok(());
    }


    let localization =
        Localization::load(
            connection,
            requested_locale,
        )?;


    let mut statement =
        connection
            .prepare(
                "SELECT translation_key
                 FROM translation_keys
                 ORDER BY translation_key"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare runtime localization catalog query: {}",
                        error,
                    )
                }
            )?;


    let keys =
        statement
            .query_map(
                [],
                |row| {
                    row.get::<_, String>(
                        0
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to query runtime localization keys: {}",
                        error,
                    )
                }
            )?;


    let mut runtime_text =
        HashMap::new();


    for key_result in keys {

        let key =
            key_result
                .map_err(
                    |error| {
                        format!(
                            "Unable to read runtime localization key: {}",
                            error,
                        )
                    }
                )?;


        let text =
            localization.text(
                connection,
                &key,
            )?;


        runtime_text.insert(
            key,
            text,
        );
    }


    RUNTIME_TEXT
        .set(
            runtime_text
        )
        .map_err(
            |_| {
                "Runtime localization catalog was initialized more than once."
                    .to_string()
            }
        )
}


pub fn runtime_text(
    key: &str,
) -> String {

    RUNTIME_TEXT
        .get()
        .and_then(
            |catalog| {
                catalog.get(
                    key
                )
            }
        )
        .cloned()
        .unwrap_or_else(
            || {
                format!(
                    "[{}]",
                    key,
                )
            }
        )
}


/// Resolve a runtime localization key and substitute named parameters.
///
/// Templates use `{name}` placeholders. Parameter names are deliberately
/// independent of word order so translations may rearrange values naturally.
///
/// Unknown localization keys retain the existing visible `[key]` diagnostic.
/// Unknown placeholders are also left visible in the returned text rather than
/// silently disappearing, which makes incomplete developer catalog entries
/// immediately apparent.
pub fn runtime_text_with_params(
    key: &str,
    parameters: &[(&str, &str)],
) -> String {

    let template =
        runtime_text(
            key
        );

    format_named_parameters(
        &template,
        parameters,
    )
}


/// Substitute `{name}` placeholders in already-localized text.
///
/// Literal braces may be written as `{{` and `}}`. A placeholder is replaced
/// only when its exact name is present in `parameters`; otherwise the original
/// `{name}` text is preserved as a visible development diagnostic.
fn format_named_parameters(
    template: &str,
    parameters: &[(&str, &str)],
) -> String {

    let parameter_map:
        HashMap<&str, &str> =
            parameters
                .iter()
                .copied()
                .collect();

    let mut output =
        String::with_capacity(
            template.len()
        );

    let characters:
        Vec<char> =
            template
                .chars()
                .collect();

    let mut index =
        0usize;

    while index < characters.len() {

        match characters[index] {

            '{' => {
                if index + 1 < characters.len()
                    && characters[index + 1] == '{'
                {
                    output.push(
                        '{'
                    );

                    index += 2;
                    continue;
                }

                let mut end =
                    index + 1;

                while end < characters.len()
                    && characters[end] != '}'
                {
                    end += 1;
                }

                if end < characters.len() {

                    let name:
                        String =
                            characters[
                                index + 1..end
                            ]
                            .iter()
                            .collect();

                    if !name.is_empty() {

                        if let Some(value) =
                            parameter_map.get(
                                name.as_str()
                            )
                        {
                            output.push_str(
                                value
                            );

                            index =
                                end + 1;

                            continue;
                        }
                    }

                    output.extend(
                        characters[
                            index..=end
                        ]
                        .iter()
                    );

                    index =
                        end + 1;

                    continue;
                }

                output.push(
                    '{'
                );

                index += 1;
            }

            '}' => {
                if index + 1 < characters.len()
                    && characters[index + 1] == '}'
                {
                    output.push(
                        '}'
                    );

                    index += 2;
                } else {
                    output.push(
                        '}'
                    );

                    index += 1;
                }
            }

            character => {
                output.push(
                    character
                );

                index += 1;
            }
        }
    }

    output
}


#[derive(Debug, Clone)]
pub struct Localization {
    requested_locale: String,
    active_locale: String,
    text_direction: String,
    used_fallback_locale: bool,
}


impl Localization {

    pub fn load(
        connection: &Connection,
        requested_locale: &str,
    ) -> Result<Self, String> {

        let requested =
            if requested_locale.trim().is_empty() {
                DEFAULT_LOCALE
            } else {
                requested_locale.trim()
            };

        let selected =
            lookup_enabled_language(
                connection,
                requested,
            )?;

        let (
            active_locale,
            text_direction,
            used_fallback_locale,
        ) =
            match selected {
                Some((locale, direction)) => {
                    (
                        locale,
                        direction,
                        false,
                    )
                }

                None => {
                    let fallback =
                        lookup_enabled_language(
                            connection,
                            DEFAULT_LOCALE,
                        )?
                        .ok_or_else(
                            || {
                                format!(
                                    "Configured locale '{}' is unavailable and required fallback locale '{}' is not enabled",
                                    requested,
                                    DEFAULT_LOCALE,
                                )
                            }
                        )?;

                    (
                        fallback.0,
                        fallback.1,
                        true,
                    )
                }
            };

        Ok(
            Self {
                requested_locale:
                    requested.to_string(),

                active_locale,

                text_direction,

                used_fallback_locale,
            }
        )
    }


    pub fn requested_locale(
        &self,
    ) -> &str {
        &self.requested_locale
    }


    pub fn active_locale(
        &self,
    ) -> &str {
        &self.active_locale
    }


    pub fn text_direction(
        &self,
    ) -> &str {
        &self.text_direction
    }


    pub fn used_fallback_locale(
        &self,
    ) -> bool {
        self.used_fallback_locale
    }


    pub fn text(
        &self,
        connection: &Connection,
        key: &str,
    ) -> Result<String, String> {

        let translation =
            connection
                .query_row(
                    "SELECT translated_text
                     FROM translations
                     WHERE locale = ?1
                       AND translation_key = ?2",
                    params![
                        self.active_locale,
                        key,
                    ],
                    |row| {
                        row.get::<_, String>(
                            0
                        )
                    },
                )
                .optional()
                .map_err(
                    |error| {
                        format!(
                            "Unable to query translation '{}' for locale '{}': {}",
                            key,
                            self.active_locale,
                            error,
                        )
                    }
                )?;

        if let Some(text) =
            translation
        {
            return Ok(
                text
            );
        }

        connection
            .query_row(
                "SELECT english_text
                 FROM translation_keys
                 WHERE translation_key = ?1",
                params![
                    key,
                ],
                |row| {
                    row.get::<_, String>(
                        0
                    )
                },
            )
            .optional()
            .map_err(
                |error| {
                    format!(
                        "Unable to query canonical English text for localization key '{}': {}",
                        key,
                        error,
                    )
                }
            )?
            .ok_or_else(
                || {
                    format!(
                        "Unknown localization key '{}'",
                        key,
                    )
                }
            )
    }
}


fn lookup_enabled_language(
    connection: &Connection,
    locale: &str,
) -> Result<Option<(String, String)>, String> {

    connection
        .query_row(
            "SELECT locale,
                    text_direction
             FROM languages
             WHERE locale = ?1
               AND enabled = 1",
            params![
                locale,
            ],
            |row| {
                Ok(
                    (
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                    )
                )
            },
        )
        .optional()
        .map_err(
            |error| {
                format!(
                    "Unable to query localization language '{}': {}",
                    locale,
                    error,
                )
            }
        )
}
