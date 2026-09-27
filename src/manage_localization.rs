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
