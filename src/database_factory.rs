//
// Screenshaver current-version database factory infrastructure.
//
// Factory objects are constructed from the current executable and installed
// resources.  Fresh database initialization and reconstruction migration must
// share these constructors so factory semantics cannot drift between paths.
//

use std::fs;

use rusqlite::{
    params,
    Connection,
};


const RUNTIME_SOURCE_PREPARATION_VERSION: i64 = 1;


pub(crate) fn register_default_shader(
    connection: &mut Connection,
) -> Result<i64, String> {

    let shader_path =
        crate::locate_paths::shader_dir()
            .join(
                "default.glsl"
            );


    let source_bytes =
        fs::read(
            &shader_path
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read default shader '{}': {}",
                    shader_path.display(),
                    error,
                )
            }
        )?;


    let source =
        String::from_utf8(
            source_bytes.clone()
        )
        .map_err(
            |error| {
                format!(
                    "Default shader '{}' is not valid UTF-8: {}",
                    shader_path.display(),
                    error,
                )
            }
        )?;


    let shader_kind =
        crate::classify_shader::classify_shader(
            &source
        );


    let shader_type =
        match shader_kind {

            crate::classify_shader::ShaderKind::NativeGLSL => {
                "native"
            }

            crate::classify_shader::ShaderKind::ShaderToy => {
                return Err(
                    format!(
                        "Default shader '{}' was unexpectedly classified as ShaderToy",
                        shader_path.display(),
                    )
                );
            }

            crate::classify_shader::ShaderKind::Isf => {
                return Err(
                    format!(
                        "Default shader '{}' was unexpectedly classified as ISF",
                        shader_path.display(),
                    )
                );
            }
        };


    let (
        _warnings,
        rejection_reasons,
    ) =
        crate::preprocess_shader::analyze_native_shader(
            &source
        );


    let channel_usage =
        crate::preprocess_shader::analyze_native_channel_usage(
            &source
        );


    let channel_usage_mask: i64 =
        (if channel_usage.channels[0] { 1 } else { 0 })
        | (if channel_usage.channels[1] { 1 << 1 } else { 0 })
        | (if channel_usage.channels[2] { 1 << 2 } else { 0 })
        | (if channel_usage.channels[3] { 1 << 3 } else { 0 })
        | (if channel_usage.requires_mipmaps { 1 << 4 } else { 0 });


    let shader_inputs_json =
        "[]";


    let (
        validation_status,
        validation_reason,
        validation_message,
    ) =
        if rejection_reasons.is_empty() {

            (
                "valid",
                None::<String>,
                None::<String>,
            )

        } else {

            (
                "rejected",
                Some(
                    "static_analysis_failed"
                        .to_string()
                ),
                Some(
                    rejection_reasons
                        .join(
                            "; "
                        )
                ),
            )
        };


    let source_hash =
        crate::hash_shader::hash_source(
            &source_bytes
        );


    let filename =
        shader_path
            .file_name()
            .and_then(
                |value| {
                    value.to_str()
                }
            )
            .ok_or_else(
                || {
                    format!(
                        "Default shader path '{}' has no valid UTF-8 filename",
                        shader_path.display(),
                    )
                }
            )?
            .to_string();


    let source_path =
        shader_path
            .parent()
            .ok_or_else(
                || {
                    format!(
                        "Default shader path '{}' has no parent directory",
                        shader_path.display(),
                    )
                }
            )?
            .to_string_lossy()
            .to_string();


    connection
        .execute(
            "INSERT INTO shaders (
                 shader_added_at,
                 filename,
                 source_path,
                 shader_type,
                 source_hash,
                 file_status,
                 validation_status,
                 validation_reason,
                 validation_message,
                 preprocessed_source,
                 preprocessor_version,
                 channel_usage_mask,
                 shader_inputs_json
             )
             VALUES (
                 strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                 ?1,
                 ?2,
                 ?3,
                 ?4,
                 'present',
                 ?5,
                 ?6,
                 ?7,
                 ?8,
                 ?9,
                 ?10,
                 ?11
             )",
            params![
                filename,
                source_path,
                shader_type,
                source_hash,
                validation_status,
                validation_reason,
                validation_message,
                source_bytes,
                RUNTIME_SOURCE_PREPARATION_VERSION,
                channel_usage_mask,
                shader_inputs_json,
            ],
        )
        .map_err(
            |error| {
                format!(
                    "Unable to register default shader '{}': {}",
                    shader_path.display(),
                    error,
                )
            }
        )?;


    Ok(
        connection.last_insert_rowid()
    )
}




#[derive(Debug, Clone, Copy)]
pub(crate) struct FactoryLanguage {
    pub locale: &'static str,
    pub english_name: &'static str,
    pub native_name: &'static str,
    pub text_direction: &'static str,
}


#[derive(Debug, Clone, Copy)]
pub(crate) struct FactoryTranslationKey {
    pub key: &'static str,
    pub english_text: &'static str,
    pub translator_context: &'static str,
}


pub(crate) const FACTORY_LANGUAGES: &[FactoryLanguage] = &[
    FactoryLanguage {
        locale: "en-US",
        english_name: "English (United States)",
        native_name: "English (United States)",
        text_direction: "ltr",
    },
];


pub(crate) const FACTORY_TRANSLATION_KEYS: &[FactoryTranslationKey] = &[
    FactoryTranslationKey {
        key: "app.name",
        english_text: "Screenshaver",
        translator_context: "Application name.",
    },
    FactoryTranslationKey {
        key: "target.screensaver",
        english_text: "Screensaver",
        translator_context: "User-facing name of the screensaver runtime target.",
    },
    FactoryTranslationKey {
        key: "target.wallpaper",
        english_text: "Wallpaper",
        translator_context: "User-facing name of the wallpaper runtime target.",
    },
];


pub(crate) fn seed_localization_catalog(
    connection: &mut Connection,
) -> Result<(), String> {
    let transaction =
        connection
            .transaction()
            .map_err(
                |error| {
                    format!(
                        "Unable to begin localization-catalog transaction: {}",
                        error,
                    )
                }
            )?;


    {
        let mut statement =
            transaction
                .prepare(
                    "INSERT INTO languages (
                         locale,
                         english_name,
                         native_name,
                         text_direction,
                         enabled
                     )
                     VALUES (?1, ?2, ?3, ?4, 1)"
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to prepare language-catalog insert: {}",
                            error,
                        )
                    }
                )?;


        for language in FACTORY_LANGUAGES {
            statement
                .execute(
                    params![
                        language.locale,
                        language.english_name,
                        language.native_name,
                        language.text_direction,
                    ]
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to insert factory language '{}': {}",
                            language.locale,
                            error,
                        )
                    }
                )?;
        }
    }


    {
        let mut statement =
            transaction
                .prepare(
                    "INSERT INTO translation_keys (
                         translation_key,
                         english_text,
                         translator_context
                     )
                     VALUES (?1, ?2, ?3)"
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to prepare translation-key insert: {}",
                            error,
                        )
                    }
                )?;


        for entry in FACTORY_TRANSLATION_KEYS {
            statement
                .execute(
                    params![
                        entry.key,
                        entry.english_text,
                        entry.translator_context,
                    ]
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to insert factory translation key '{}': {}",
                            entry.key,
                            error,
                        )
                    }
                )?;
        }
    }


    transaction
        .commit()
        .map_err(
            |error| {
                format!(
                    "Unable to commit localization factory catalog: {}",
                    error,
                )
            }
        )
}
