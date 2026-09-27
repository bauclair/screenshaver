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


#[derive(Debug, Clone, Copy)]
pub(crate) struct FactoryTranslation {
    pub locale: &'static str,
    pub key: &'static str,
    pub translated_text: &'static str,
}


pub(crate) const FACTORY_LANGUAGES: &[FactoryLanguage] = &[
    FactoryLanguage {
        locale: "en-US",
        english_name: "English (United States)",
        native_name: "English (United States)",
        text_direction: "ltr",
    },

    FactoryLanguage {
        locale: "es-US",
        english_name: "Spanish (United States)",
        native_name: "Español (Estados Unidos)",
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

    FactoryTranslationKey {
        key: "backup.directory_create_failed",
        english_text: "Unable to create Screenshaver backup directory '{path}': {error}",
        translator_context: "Error creating the backup directory. {path} and {error} are supplied values and must remain unchanged.",
    },

    FactoryTranslationKey {
        key: "backup.configuration_unavailable",
        english_text: "Backup configuration unavailable: {error}",
        translator_context: "Control Center message when backup configuration cannot be loaded. {error} is supplied error text and must remain unchanged.",
    },

    FactoryTranslationKey {
        key: "backup.heading",
        english_text: "Full Backups",
        translator_context: "Heading for full Screenshaver backup settings.",
    },

    FactoryTranslationKey {
        key: "backup.schedule_prefix",
        english_text: "Make full Screenshaver backups every",
        translator_context: "Checkbox label immediately before the numeric backup interval and the unit label.",
    },

    FactoryTranslationKey {
        key: "backup.days",
        english_text: "days",
        translator_context: "Unit label following the numeric full-backup interval.",
    },

    FactoryTranslationKey {
        key: "backup.configuration_saved",
        english_text: "Backup configuration saved.",
        translator_context: "Status message after backup settings are saved successfully.",
    },

    FactoryTranslationKey {
        key: "backup.last_reference",
        english_text: "Last backup reference: {reference}",
        translator_context: "Backup settings informational text. {reference} is stored backup reference data and must remain unchanged.",
    },

    FactoryTranslationKey {
        key: "backup.folder",
        english_text: "Backup folder: {path}",
        translator_context: "Backup settings informational text. {path} is a filesystem path and must remain unchanged.",
    },

    FactoryTranslationKey {
        key: "backup.now",
        english_text: "Backup Now",
        translator_context: "Button that immediately creates a full Screenshaver backup.",
    },

    FactoryTranslationKey {
        key: "backup.creating",
        english_text: "Creating full Screenshaver backup...",
        translator_context: "Status message while a full Screenshaver backup is being created.",
    },

    FactoryTranslationKey {
        key: "backup.created",
        english_text: "Backup created: {path}",
        translator_context: "Status message shown after a backup is created. {path} is the backup path and must remain unchanged.",
    },

    FactoryTranslationKey {
        key: "backup.failed",
        english_text: "Backup failed: {error}",
        translator_context: "Status message shown when backup creation fails. {error} is externally supplied error text and must remain unchanged.",
    },

    FactoryTranslationKey {
        key: "backup.restore",
        english_text: "Restore from Backup",
        translator_context: "Button for restoring Screenshaver from a backup.",
    },

    FactoryTranslationKey {
        key: "backup.restore_disabled",
        english_text: "Restore from Backup will be enabled in the restore implementation phase.",
        translator_context: "Tooltip explaining why Restore from Backup is currently disabled.",
    },


    FactoryTranslationKey {
        key: "tab.appearance",
        english_text: "Appearance",
        translator_context: "Configuration nested-tab label.",
    },

    FactoryTranslationKey {
        key: "tab.rendering",
        english_text: "Rendering",
        translator_context: "Configuration nested-tab label.",
    },

    FactoryTranslationKey {
        key: "tab.lyrics",
        english_text: "Lyrics",
        translator_context: "Configuration nested-tab label.",
    },

    FactoryTranslationKey {
        key: "tab.data_io",
        english_text: "Data I/O",
        translator_context: "Configuration nested-tab label.",
    },

    FactoryTranslationKey {
        key: "config.unavailable",
        english_text: "Configuration is not available.",
        translator_context: "Message when configuration data cannot be loaded.",
    },

    FactoryTranslationKey {
        key: "config.save",
        english_text: "Save Configuration",
        translator_context: "Button that saves Control Center configuration.",
    },

    FactoryTranslationKey {
        key: "config.saving",
        english_text: "Saving configuration...",
        translator_context: "Status while configuration is being saved.",
    },

    FactoryTranslationKey {
        key: "common.cancel",
        english_text: "Cancel",
        translator_context: "Generic cancel button.",
    },

    FactoryTranslationKey {
        key: "config.discarded",
        english_text: "Configuration changes discarded.",
        translator_context: "Status after configuration edits are discarded.",
    },

    FactoryTranslationKey {
        key: "data_io.heading",
        english_text: "Data I/O",
        translator_context: "Data import/export and backup page heading.",
    },

    FactoryTranslationKey {
        key: "data_io.description",
        english_text: "Create recovery backups or import/export portable Screenshaver data.",
        translator_context: "Description of the Data I/O page.",
    },

    FactoryTranslationKey {
        key: "data_io.portable_data",
        english_text: "Portable Data",
        translator_context: "Heading for portable import/export data.",
    },

    FactoryTranslationKey {
        key: "data_io.import",
        english_text: "Import...",
        translator_context: "Button opening the import wizard.",
    },

    FactoryTranslationKey {
        key: "data_io.export",
        english_text: "Export...",
        translator_context: "Button opening the export wizard.",
    },

    FactoryTranslationKey {
        key: "appearance.heading",
        english_text: "Appearance Defaults",
        translator_context: "Appearance configuration heading.",
    },

    FactoryTranslationKey {
        key: "appearance.show_splash",
        english_text: "Show splash screen",
        translator_context: "Checkbox controlling the splash screen.",
    },

    FactoryTranslationKey {
        key: "appearance.screensaver_subtitles",
        english_text: "Screensaver subtitles",
        translator_context: "Checkbox controlling screensaver subtitles.",
    },

    FactoryTranslationKey {
        key: "appearance.subtitle_placement",
        english_text: "Subtitle placement:",
        translator_context: "Label for subtitle placement.",
    },

    FactoryTranslationKey {
        key: "appearance.wallpaper_notifications",
        english_text: "Wallpaper Notifications",
        translator_context: "Checkbox controlling wallpaper notifications.",
    },

    FactoryTranslationKey {
        key: "placement.top_left",
        english_text: "Top left",
        translator_context: "Displayed subtitle placement option; stored token remains top:left.",
    },

    FactoryTranslationKey {
        key: "placement.top_center",
        english_text: "Top center",
        translator_context: "Displayed subtitle placement option; stored token remains top:center.",
    },

    FactoryTranslationKey {
        key: "placement.top_right",
        english_text: "Top right",
        translator_context: "Displayed subtitle placement option; stored token remains top:right.",
    },

    FactoryTranslationKey {
        key: "placement.bottom_left",
        english_text: "Bottom left",
        translator_context: "Displayed subtitle placement option; stored token remains bottom:left.",
    },

    FactoryTranslationKey {
        key: "placement.bottom_center",
        english_text: "Bottom center",
        translator_context: "Displayed subtitle placement option; stored token remains bottom:center.",
    },

    FactoryTranslationKey {
        key: "placement.bottom_right",
        english_text: "Bottom right",
        translator_context: "Displayed subtitle placement option; stored token remains bottom:right.",
    },

    FactoryTranslationKey {
        key: "target.screensaver_settings",
        english_text: "Screensaver Settings and Defaults",
        translator_context: "Screensaver configuration page heading.",
    },

    FactoryTranslationKey {
        key: "target.wallpaper_settings",
        english_text: "Wallpaper Settings and Defaults",
        translator_context: "Wallpaper configuration page heading.",
    },

    FactoryTranslationKey {
        key: "common.enabled",
        english_text: "Enabled",
        translator_context: "Generic enabled checkbox label.",
    },

    FactoryTranslationKey {
        key: "target.display_format",
        english_text: "Display Format:",
        translator_context: "Wallpaper display-format label.",
    },

    FactoryTranslationKey {
        key: "target.display_format_help",
        english_text: "Selects whether wallpaper is presented full-screen or in a normal desktop-managed window.",
        translator_context: "Help for wallpaper display format.",
    },

    FactoryTranslationKey {
        key: "target.full_screen",
        english_text: "Full-screen",
        translator_context: "Displayed wallpaper format; stored enum remains unchanged.",
    },

    FactoryTranslationKey {
        key: "target.windowshader",
        english_text: "Windowshader",
        translator_context: "Displayed windowshader wallpaper format.",
    },

    FactoryTranslationKey {
        key: "target.mode",
        english_text: "Mode:",
        translator_context: "Runtime display mode label.",
    },

    FactoryTranslationKey {
        key: "mode.ordered",
        english_text: "Ordered",
        translator_context: "Displayed ordered mode; stored token remains ordered.",
    },

    FactoryTranslationKey {
        key: "mode.random",
        english_text: "Random",
        translator_context: "Displayed random mode; stored token remains random.",
    },

    FactoryTranslationKey {
        key: "mode.single",
        english_text: "Single",
        translator_context: "Displayed single mode; stored token remains single.",
    },

    FactoryTranslationKey {
        key: "mode.playlist",
        english_text: "Playlist",
        translator_context: "Displayed playlist mode; stored token remains playlist.",
    },

    FactoryTranslationKey {
        key: "target.policy",
        english_text: "Policy:",
        translator_context: "Policy selector label.",
    },

    FactoryTranslationKey {
        key: "target.select_policy",
        english_text: "<select policy>",
        translator_context: "Placeholder for an unselected policy.",
    },

    FactoryTranslationKey {
        key: "target.single_policy_selected",
        english_text: "Single {target} policy selected: {name}.",
        translator_context: "Status after selecting a single policy. {target} is localized target text; {name} is user-authored and unchanged.",
    },

    FactoryTranslationKey {
        key: "target.no_eligible_policies",
        english_text: "No eligible policies",
        translator_context: "Disabled selector text when no policies are eligible.",
    },

    FactoryTranslationKey {
        key: "target.playlist",
        english_text: "Playlist:",
        translator_context: "Playlist selector label.",
    },

    FactoryTranslationKey {
        key: "target.select_playlist",
        english_text: "<select playlist>",
        translator_context: "Placeholder for an unselected playlist.",
    },

    FactoryTranslationKey {
        key: "target.no_playlists",
        english_text: "No playlists available",
        translator_context: "Disabled selector text when no playlists exist.",
    },

    FactoryTranslationKey {
        key: "target.playlist_selected",
        english_text: "{target} playlist selected: {name}.",
        translator_context: "Status after selecting a playlist. {target} is localized target text; {name} is user-authored and unchanged.",
    },

    FactoryTranslationKey {
        key: "target.playlists_unavailable",
        english_text: "Unable to load playlists",
        translator_context: "Disabled selector text when playlists cannot be loaded.",
    },

    FactoryTranslationKey {
        key: "target.interval",
        english_text: "Interval:",
        translator_context: "Display interval label.",
    },

    FactoryTranslationKey {
        key: "unit.seconds_lower",
        english_text: "seconds",
        translator_context: "Lowercase seconds unit label.",
    },

    FactoryTranslationKey {
        key: "target.idle_timeout",
        english_text: "Idle timeout:",
        translator_context: "Screensaver idle timeout label.",
    },

    FactoryTranslationKey {
        key: "unit.seconds",
        english_text: "Seconds",
        translator_context: "Seconds unit option.",
    },

    FactoryTranslationKey {
        key: "unit.minutes",
        english_text: "Minutes",
        translator_context: "Minutes unit option.",
    },

    FactoryTranslationKey {
        key: "unit.hours",
        english_text: "Hours",
        translator_context: "Hours unit option.",
    },

    FactoryTranslationKey {
        key: "target.animation_speed",
        english_text: "Animation speed:",
        translator_context: "Animation speed label.",
    },

    FactoryTranslationKey {
        key: "target.texture",
        english_text: "Texture:",
        translator_context: "Texture selector label.",
    },

    FactoryTranslationKey {
        key: "common.random",
        english_text: "Random",
        translator_context: "Displayed random option; stored token remains random.",
    },

    FactoryTranslationKey {
        key: "target.texture_catalog_unavailable",
        english_text: "Texture catalog unavailable",
        translator_context: "Disabled text when texture catalog cannot load.",
    },

    FactoryTranslationKey {
        key: "target.texture_choices_failed",
        english_text: "Unable to load texture choices: {error}",
        translator_context: "Status when texture choices cannot load; {error} remains unchanged.",
    },

    FactoryTranslationKey {
        key: "target.palette",
        english_text: "Palette:",
        translator_context: "Palette selector label.",
    },

    FactoryTranslationKey {
        key: "target.texture_primitives",
        english_text: "Texture primitives:",
        translator_context: "Texture primitive-count label.",
    },

    FactoryTranslationKey {
        key: "target.single_policy_required",
        english_text: "Select a shader policy for Single {target} display mode.",
        translator_context: "Status requiring a policy for Single mode; {target} is localized.",
    },

    FactoryTranslationKey {
        key: "lyrics.heading",
        english_text: "Lyrics",
        translator_context: "Lyrics configuration heading.",
    },

    FactoryTranslationKey {
        key: "lyrics.display",
        english_text: "Display synchronized song lyrics (windowshader only)",
        translator_context: "Checkbox enabling synchronized lyrics.",
    },

    FactoryTranslationKey {
        key: "lyrics.display_help",
        english_text: "Displays synchronized lyrics for the currently playing song over the windowshader. Lyrics are obtained automatically when available.",
        translator_context: "Help for synchronized lyrics.",
    },

    FactoryTranslationKey {
        key: "rendering.heading",
        english_text: "Rendering Defaults",
        translator_context: "Rendering defaults heading.",
    },

    FactoryTranslationKey {
        key: "rendering.fps",
        english_text: "Rendered FPS:",
        translator_context: "Rendered FPS label.",
    },

    FactoryTranslationKey {
        key: "rendering.anti_aliasing",
        english_text: "Anti-aliasing:",
        translator_context: "Anti-aliasing label.",
    },

    FactoryTranslationKey {
        key: "rendering.dithering",
        english_text: "Dithering:",
        translator_context: "Dithering label.",
    },

    FactoryTranslationKey {
        key: "rendering.color_precision",
        english_text: "Color precision:",
        translator_context: "Color precision label.",
    },

    FactoryTranslationKey {
        key: "rendering.render_scale",
        english_text: "Render scale:",
        translator_context: "Render scale label.",
    },

    FactoryTranslationKey {
        key: "rendering.off",
        english_text: "Off",
        translator_context: "Displayed disabled rendering option; stored token remains off.",
    },

    FactoryTranslationKey {
        key: "rendering.fxaa",
        english_text: "FXAA",
        translator_context: "FXAA rendering option.",
    },

    FactoryTranslationKey {
        key: "rendering.subtle",
        english_text: "Subtle",
        translator_context: "Displayed subtle dithering option; stored token remains subtle.",
    },

    FactoryTranslationKey {
        key: "rendering.auto",
        english_text: "Automatic",
        translator_context: "Displayed automatic precision option; stored token remains auto.",
    },

    FactoryTranslationKey {
        key: "rendering.standard",
        english_text: "Standard",
        translator_context: "Displayed standard precision option; stored token remains standard.",
    },

    FactoryTranslationKey {
        key: "rendering.high",
        english_text: "High",
        translator_context: "Displayed high precision option; stored token remains high.",
    },

    FactoryTranslationKey {
        key: "target.palette_unavailable",
        english_text: "Curated palette unavailable",
        translator_context: "Disabled text when curated palette cannot load.",
    },

    FactoryTranslationKey {
        key: "target.palette_choices_failed",
        english_text: "Unable to load curated palette choices: {error}",
        translator_context: "Status when curated palette choices cannot load.",
    },

    FactoryTranslationKey {
        key: "target.palette_selected",
        english_text: "{target} default palette selected: {name}.",
        translator_context: "Status after choosing a default palette. {name} is factory palette description and remains verbatim for now.",
    },
];


pub(crate) const FACTORY_TRANSLATIONS: &[FactoryTranslation] = &[
    FactoryTranslation {
        locale: "es-US",
        key: "target.screensaver",
        translated_text: "Protector de pantalla",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "backup.created",
        translated_text: "Copia de seguridad creada: {path}",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "backup.directory_create_failed",
        translated_text: "No se pudo crear el directorio de copias de seguridad de Screenshaver '{path}': {error}",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "backup.configuration_unavailable",
        translated_text: "La configuración de copias de seguridad no está disponible: {error}",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "backup.heading",
        translated_text: "Copias de seguridad completas",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "backup.schedule_prefix",
        translated_text: "Crear copias de seguridad completas de Screenshaver cada",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "backup.days",
        translated_text: "días",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "backup.configuration_saved",
        translated_text: "Configuración de copias de seguridad guardada.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "backup.last_reference",
        translated_text: "Referencia de la última copia de seguridad: {reference}",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "backup.folder",
        translated_text: "Carpeta de copias de seguridad: {path}",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "backup.now",
        translated_text: "Crear copia ahora",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "backup.creating",
        translated_text: "Creando copia de seguridad completa de Screenshaver...",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "backup.failed",
        translated_text: "Error al crear la copia de seguridad: {error}",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore",
        translated_text: "Restaurar desde copia de seguridad",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore_disabled",
        translated_text: "Restaurar desde copia de seguridad se habilitará en la fase de implementación de la restauración.",
    },


    FactoryTranslation {
        locale: "es-US",
        key: "tab.appearance",
        translated_text: "Apariencia",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "tab.rendering",
        translated_text: "Renderizado",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "tab.lyrics",
        translated_text: "Letras",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "tab.data_io",
        translated_text: "Datos E/S",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "config.unavailable",
        translated_text: "La configuración no está disponible.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "config.save",
        translated_text: "Guardar configuración",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "config.saving",
        translated_text: "Guardando configuración...",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "common.cancel",
        translated_text: "Cancelar",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "config.discarded",
        translated_text: "Cambios de configuración descartados.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "data_io.heading",
        translated_text: "Datos E/S",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "data_io.description",
        translated_text: "Cree copias de seguridad de recuperación o importe y exporte datos portátiles de Screenshaver.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "data_io.portable_data",
        translated_text: "Datos portátiles",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "data_io.import",
        translated_text: "Importar...",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "data_io.export",
        translated_text: "Exportar...",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "appearance.heading",
        translated_text: "Valores predeterminados de apariencia",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "appearance.show_splash",
        translated_text: "Mostrar pantalla de presentación",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "appearance.screensaver_subtitles",
        translated_text: "Subtítulos del protector de pantalla",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "appearance.subtitle_placement",
        translated_text: "Ubicación de subtítulos:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "appearance.wallpaper_notifications",
        translated_text: "Notificaciones del fondo de pantalla",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "placement.top_left",
        translated_text: "Arriba a la izquierda",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "placement.top_center",
        translated_text: "Arriba al centro",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "placement.top_right",
        translated_text: "Arriba a la derecha",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "placement.bottom_left",
        translated_text: "Abajo a la izquierda",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "placement.bottom_center",
        translated_text: "Abajo al centro",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "placement.bottom_right",
        translated_text: "Abajo a la derecha",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.screensaver_settings",
        translated_text: "Configuración y valores predeterminados del protector de pantalla",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.wallpaper_settings",
        translated_text: "Configuración y valores predeterminados del fondo de pantalla",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "common.enabled",
        translated_text: "Habilitado",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.display_format",
        translated_text: "Formato de visualización:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.display_format_help",
        translated_text: "Selecciona si el fondo de pantalla se presenta a pantalla completa o en una ventana normal administrada por el escritorio.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.full_screen",
        translated_text: "Pantalla completa",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.windowshader",
        translated_text: "Windowshader",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.mode",
        translated_text: "Modo:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "mode.ordered",
        translated_text: "Ordenado",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "mode.random",
        translated_text: "Aleatorio",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "mode.single",
        translated_text: "Único",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "mode.playlist",
        translated_text: "Lista de reproducción",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.policy",
        translated_text: "Política:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.select_policy",
        translated_text: "<seleccionar política>",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.single_policy_selected",
        translated_text: "Política única de {target} seleccionada: {name}.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.no_eligible_policies",
        translated_text: "No hay políticas elegibles",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.playlist",
        translated_text: "Lista de reproducción:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.select_playlist",
        translated_text: "<seleccionar lista de reproducción>",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.no_playlists",
        translated_text: "No hay listas de reproducción disponibles",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.playlist_selected",
        translated_text: "Lista de reproducción de {target} seleccionada: {name}.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.playlists_unavailable",
        translated_text: "No se pudieron cargar las listas de reproducción",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.interval",
        translated_text: "Intervalo:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "unit.seconds_lower",
        translated_text: "segundos",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.idle_timeout",
        translated_text: "Tiempo de inactividad:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "unit.seconds",
        translated_text: "Segundos",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "unit.minutes",
        translated_text: "Minutos",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "unit.hours",
        translated_text: "Horas",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.animation_speed",
        translated_text: "Velocidad de animación:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.texture",
        translated_text: "Textura:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "common.random",
        translated_text: "Aleatorio",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.texture_catalog_unavailable",
        translated_text: "Catálogo de texturas no disponible",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.texture_choices_failed",
        translated_text: "No se pudieron cargar las opciones de textura: {error}",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.palette",
        translated_text: "Paleta:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.texture_primitives",
        translated_text: "Primitivas de textura:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.single_policy_required",
        translated_text: "Seleccione una política de shader para el modo de visualización Único de {target}.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "lyrics.heading",
        translated_text: "Letras",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "lyrics.display",
        translated_text: "Mostrar letras sincronizadas de canciones (solo windowshader)",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "lyrics.display_help",
        translated_text: "Muestra letras sincronizadas de la canción que se está reproduciendo sobre el windowshader. Las letras se obtienen automáticamente cuando están disponibles.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "rendering.heading",
        translated_text: "Valores predeterminados de renderizado",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "rendering.fps",
        translated_text: "FPS renderizados:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "rendering.anti_aliasing",
        translated_text: "Antialiasing:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "rendering.dithering",
        translated_text: "Tramado:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "rendering.color_precision",
        translated_text: "Precisión de color:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "rendering.render_scale",
        translated_text: "Escala de renderizado:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "rendering.off",
        translated_text: "Desactivado",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "rendering.fxaa",
        translated_text: "FXAA",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "rendering.subtle",
        translated_text: "Sutil",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "rendering.auto",
        translated_text: "Automática",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "rendering.standard",
        translated_text: "Estándar",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "rendering.high",
        translated_text: "Alta",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.palette_unavailable",
        translated_text: "Paleta seleccionada no disponible",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.palette_choices_failed",
        translated_text: "No se pudieron cargar las opciones de la paleta seleccionada: {error}",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.palette_selected",
        translated_text: "Paleta predeterminada de {target} seleccionada: {name}.",
    },
];


pub(crate) fn seed_localization_catalog(
    connection: &mut Connection,
) -> Result<(), String> {

    synchronize_localization_catalog(
        connection
    )
}


pub(crate) fn synchronize_localization_catalog(
    connection: &mut Connection,
) -> Result<(), String> {

    let transaction =
        connection
            .transaction()
            .map_err(
                |error| {
                    format!(
                        "Unable to begin localization-catalog synchronization transaction: {}",
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
                     VALUES (?1, ?2, ?3, ?4, 1)
                     ON CONFLICT(locale) DO UPDATE SET
                         english_name = excluded.english_name,
                         native_name = excluded.native_name,
                         text_direction = excluded.text_direction,
                         enabled = excluded.enabled"
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to prepare language-catalog synchronization: {}",
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
                            "Unable to synchronize factory language '{}': {}",
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
                     VALUES (?1, ?2, ?3)
                     ON CONFLICT(translation_key) DO UPDATE SET
                         english_text = excluded.english_text,
                         translator_context = excluded.translator_context"
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to prepare translation-key synchronization: {}",
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
                            "Unable to synchronize factory translation key '{}': {}",
                            entry.key,
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
                    "INSERT INTO translations (
                         locale,
                         translation_key,
                         translated_text
                     )
                     VALUES (?1, ?2, ?3)
                     ON CONFLICT(locale, translation_key) DO UPDATE SET
                         translated_text = excluded.translated_text"
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to prepare translation synchronization: {}",
                            error,
                        )
                    }
                )?;


        for entry in FACTORY_TRANSLATIONS {
            statement
                .execute(
                    params![
                        entry.locale,
                        entry.key,
                        entry.translated_text,
                    ]
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to synchronize factory translation '{}:{}': {}",
                            entry.locale,
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
                    "Unable to commit localization factory catalog synchronization: {}",
                    error,
                )
            }
        )
}
