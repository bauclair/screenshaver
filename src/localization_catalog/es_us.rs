// Spanish (United States) factory translations for Screenshaver.

use super::FactoryTranslation;

pub(crate) const TRANSLATIONS: &[FactoryTranslation] = &[
    FactoryTranslation {
        locale: "es-US",
        key: "target.screensaver",
        translated_text: "Protector de pantalla",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "target.wallpaper",
        translated_text: "Fondo de pantalla",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "warning.desktop_icon_compatibility_title",
        translated_text: "Advertencia de compatibilidad de iconos del escritorio",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "warning.screen_locking_unavailable_title",
        translated_text: "Bloqueo de pantalla no disponible",
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



// Restore from Backup.
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.archive_open_failed",
        translated_text: "No se pudo abrir el archivo de copia de seguridad '{path}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.cancel",
        translated_text: "Cancelar restauración",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.cancelled",
        translated_text: "Restauración cancelada. No se modificó ningún archivo activo de Screenshaver.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.configuration_directory_prepare_failed",
        translated_text: "No se pudo preparar el directorio de configuración de Screenshaver '{path}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.confirm",
        translated_text: "Confirmar restauración",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.confirmation_explanation",
        translated_text: "La copia de seguridad seleccionada superó la verificación. Al confirmar, se reemplazarán la base de datos actual de Screenshaver y los shaders administrados. Antes del cambio se creará una copia de reversión verificada de la instalación actual.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.copy_failed",
        translated_text: "No se pudo copiar '{source}' a '{destination}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.database_cutover_begin_failed",
        translated_text: "No se pudo iniciar el cambio de la base de datos para la restauración: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.database_cutover_prepare_failed",
        translated_text: "No se pudo preparar la base de datos restaurada para el cambio: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.database_install_failed_retained",
        translated_text: "No se pudo instalar la base de datos restaurada; se conservó la base de datos original: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.database_open_failed",
        translated_text: "No se pudo abrir la base de datos de restauración '{path}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.database_rollback_failed",
        translated_text: "No se pudo restaurar la base de datos previa a la restauración: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.directory_create_failed",
        translated_text: "No se pudo crear el directorio de restauración '{path}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.directory_entry_read_failed",
        translated_text: "No se pudo leer una entrada del directorio '{path}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.directory_enumerate_failed",
        translated_text: "No se pudo enumerar '{path}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.duplicate_member",
        translated_text: "El archivo de copia de seguridad contiene el elemento duplicado '{member}'; se rechazó la restauración",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.entry_inspect_failed",
        translated_text: "No se pudo inspeccionar '{path}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.export_not_backup",
        translated_text: "El archivo de Screenshaver seleccionado es una exportación, no una copia de seguridad completa",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.failed",
        translated_text: "Error al restaurar la copia de seguridad: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.failed_database_preserve_failed",
        translated_text: "No se pudo conservar la base de datos restaurada que falló: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.failed_shader_directory_preserve_failed",
        translated_text: "No se pudo conservar el directorio de shaders restaurado que falló: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.final_validation_and_rollback_failed",
        translated_text: "La base de datos restaurada no superó la validación final: {error}. La reversión automática también falló: {rollback_error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.final_validation_failed_rolled_back",
        translated_text: "La base de datos restaurada no superó la validación final; se restauró el estado previo: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.installing",
        translated_text: "Instalando la copia de seguridad verificada de Screenshaver...",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.invalid_managed_shader_member",
        translated_text: "La copia de seguridad contiene un elemento de shader administrado no válido '{member}'",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.live_database_missing",
        translated_text: "La base de datos activa '{path}' no existe; se rechazó la restauración",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.managed_shader_stage_failed",
        translated_text: "No se pudo preparar el shader administrado '{filename}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.manifest_member_missing",
        translated_text: "El manifiesto de la copia de seguridad hace referencia al elemento ausente '{member}'",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.manifest_parse_failed",
        translated_text: "No se pudo analizar el manifiesto de la copia de seguridad: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.member_sha256_failed",
        translated_text: "El elemento '{member}' de la copia de seguridad no superó la verificación SHA-256",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.package_sha256_failed",
        translated_text: "La copia de seguridad no superó la verificación SHA-256 del paquete",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.path_remove_failed",
        translated_text: "No se pudo eliminar '{path}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.ready_to_install",
        translated_text: "La restauración está lista para instalarse",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.rollback_directory_create_failed",
        translated_text: "No se pudo crear el directorio de reversión de la restauración: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.rollback_snapshot_create_failed",
        translated_text: "No se pudo crear la instantánea de base de datos previa a la restauración '{path}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.schema_mismatch",
        translated_text: "El manifiesto de la copia de seguridad indica el esquema de base de datos {manifest_schema}, pero la base de datos preparada indica el esquema {database_schema}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.selected_backup",
        translated_text: "Copia de seguridad: {path}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.shader_cutover_begin_failed",
        translated_text: "No se pudo iniciar la restauración de shaders administrados; se restauró la base de datos original: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.shader_install_and_rollback_failed",
        translated_text: "No se pudieron instalar los shaders administrados restaurados: {error}. La reversión automática también falló: {rollback_error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.shader_install_failed_rolled_back",
        translated_text: "No se pudieron instalar los shaders administrados restaurados; se restauró el estado previo: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.snapshot_missing_from_manifest",
        translated_text: "El manifiesto de la copia de seguridad no identifica una instantánea de la base de datos",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.snapshot_payload_missing",
        translated_text: "La copia de seguridad no contiene la instantánea de base de datos declarada",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.staged_database_write_failed",
        translated_text: "No se pudo escribir la base de datos de restauración preparada '{path}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.staged_shader_directory_create_failed",
        translated_text: "No se pudo crear el directorio preparado de shaders administrados '{path}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.staged_successfully",
        translated_text: "Copia de seguridad verificada y preparada correctamente: {archive} | creada {created} | Screenshaver {version} | esquema de base de datos {source_schema} -> {staged_schema} | shaders administrados {shader_count} | preparación {staging}. No se modificó ningún archivo activo de Screenshaver.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.staging_create_failed",
        translated_text: "No se pudo crear el directorio de preparación de restauración '{path}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.staging_failed",
        translated_text: "Error al preparar la restauración de la copia de seguridad: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.stale_staging_remove_failed",
        translated_text: "No se pudo eliminar el directorio obsoleto de preparación de restauración '{path}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.success",
        translated_text: "Copia de seguridad restaurada correctamente. La base de datos y los shaders administrados restaurados superaron la validación final. Cierre el Centro de control para que Screenshaver pueda volver a cargar la configuración restaurada.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.unsafe_member",
        translated_text: "El archivo de copia de seguridad contiene una ruta de elemento no segura '{member}'; se rechazó la restauración",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.unsupported_filesystem_entry",
        translated_text: "La restauración se negó a copiar la entrada del sistema de archivos no compatible '{path}'",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.unsupported_format",
        translated_text: "Formato de copia de seguridad de Screenshaver no compatible '{format}' versión {version}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.unsupported_snapshot_path",
        translated_text: "El manifiesto de la copia de seguridad contiene una ruta de instantánea de base de datos no compatible '{path}'",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.zip_member_inspect_failed",
        translated_text: "No se pudo inspeccionar el elemento {index} del ZIP de copia de seguridad: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.zip_member_read_failed",
        translated_text: "No se pudo leer el elemento '{member}' del ZIP de copia de seguridad: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "backup.restore.zip_read_failed",
        translated_text: "No se pudo leer el archivo ZIP de copia de seguridad: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "database.restore.historical_staged_replace_failed",
        translated_text: "No se pudo reemplazar la base de datos histórica de restauración preparada '{path}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "database.restore.reconstructed_promote_failed",
        translated_text: "No se pudo promover la base de datos de restauración preparada reconstruida de '{source}' a '{destination}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "database.restore.reconstruction_failed",
        translated_text: "Falló la reconstrucción de la base de datos de restauración preparada del esquema {source_schema} al esquema {destination_schema}: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "database.restore.reconstruction_validation_failed",
        translated_text: "La base de datos de restauración preparada reconstruida desde el esquema {source_schema} no superó la validación: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "database.restore.staged_database_missing",
        translated_text: "No se pudo preparar la base de datos de restauración porque '{path}' no existe",
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


    FactoryTranslation {
        locale: "es-US",
        key: "post.tab.visual_quality",
        translated_text: "Calidad visual",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.tab.image_transforms",
        translated_text: "Transformaciones de imagen",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.tab.audiovisual",
        translated_text: "Audiovisual",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.tab.audio_motion",
        translated_text: "Movimiento de audio",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.common.unchanged",
        translated_text: "Sin cambios",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.common.enabled",
        translated_text: "Habilitado",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.common.disabled",
        translated_text: "Deshabilitado",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.common.off",
        translated_text: "Desactivado",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.visual.anti_aliasing",
        translated_text: "Antialiasing:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.visual.anti_aliasing_help",
        translated_text: "Controla el suavizado de bordes del shader renderizado.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.visual.dithering",
        translated_text: "Tramado:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.visual.dithering_help",
        translated_text: "Controla un tramado sutil para reducir las bandas de color visibles.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.visual.subtle",
        translated_text: "Sutil",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.visual.color_precision",
        translated_text: "Precisión de color:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.visual.color_precision_help",
        translated_text: "Selecciona la precisión de color utilizada por el posprocesamiento.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.visual.automatic",
        translated_text: "Automática",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.visual.standard_precision",
        translated_text: "Precisión estándar",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.visual.high_precision",
        translated_text: "Alta precisión",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.transform.invert_colors",
        translated_text: "Invertir colores",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.transform.invert_colors_help",
        translated_text: "Invierte los colores finales renderizados.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.transform.flip_horizontal",
        translated_text: "Voltear horizontalmente",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.transform.flip_horizontal_help",
        translated_text: "Refleja horizontalmente la imagen final.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.transform.flip_vertical",
        translated_text: "Voltear verticalmente",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.transform.flip_vertical_help",
        translated_text: "Refleja verticalmente la imagen final.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.transform.hue_rotation",
        translated_text: "Rotación de tono:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.transform.hue_rotation_help",
        translated_text: "Rota los colores mostrados del shader alrededor del círculo cromático.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.audio.effect",
        translated_text: "Efecto audiovisual:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.audio.effect_help",
        translated_text: "Selecciona el efecto de posprocesamiento controlado por audio: Desactivado, Bloom de audio, Bloom espectral o el Bloom de sonoridad experimental.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.audio.audio_bloom",
        translated_text: "Bloom de audio",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.audio.spectral_bloom",
        translated_text: "Bloom espectral",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.audio.loudness_bloom",
        translated_text: "Bloom de sonoridad",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.audio.bloom_intensity",
        translated_text: "Intensidad de bloom:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.audio.bloom_intensity_help",
        translated_text: "Controla la intensidad del modo Bloom seleccionado.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.audio.bloom_saturation",
        translated_text: "Saturación de bloom:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.audio.bloom_saturation_help",
        translated_text: "Aumenta la saturación de color del bloom desde el nivel neutro 1.0 hasta 2.0 sin cambiar los colores mostrados del shader.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.audio.bloom_threshold",
        translated_text: "Umbral de bloom:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.audio.bloom_threshold_help",
        translated_text: "Controla el umbral de brillo utilizado para extraer el bloom.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.audio.frequency_rotation",
        translated_text: "Rotación de frecuencia:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.audio.frequency_rotation_help",
        translated_text: "Rota la asignación de frecuencia a color de Audio/Espectral. Se deshabilita para Bloom de sonoridad.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.audio.invert_frequency",
        translated_text: "Invertir asignación de frecuencia",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.audio.invert_frequency_help",
        translated_text: "Invierte la asignación de frecuencia-color de baja a alta de Audio/Espectral. Se deshabilita para Bloom de sonoridad.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.motion.effect",
        translated_text: "Efecto de movimiento de audio:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.motion.effect_help",
        translated_text: "Selecciona un efecto de movimiento de fotograma completo controlado por audio. Woofer from Hell utiliza la sincronización vocal de LRCMUX para el movimiento de cono inverso. FFT Mirror Warp utiliza una traza FFT reflejada de 48 canales. Polar Propeller utiliza la misma respuesta FFT de 48 canales en una deformación radial giratoria.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.motion.woofer",
        translated_text: "Woofer from Hell",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.motion.fft_mirror",
        translated_text: "FFT Mirror Warp",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.motion.polar_propeller",
        translated_text: "Polar Propeller",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.bulk.exclude",
        translated_text: "Haga clic para excluir este valor de la edición masiva",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "post.bulk.include",
        translated_text: "Haga clic para incluir este valor en la edición masiva",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.window_title",
        translated_text: "Exportar datos de Screenshaver",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.stage.select_focus",
        translated_text: "Seleccionar enfoque de exportación",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.stage.select_data",
        translated_text: "Seleccionar datos",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.destination",
        translated_text: "Destino",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.review_confirm",
        translated_text: "Revisar y confirmar",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.results",
        translated_text: "Resultados",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.policies",
        translated_text: "Políticas",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.shaders",
        translated_text: "Shaders",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.playlists",
        translated_text: "Listas de reproducción",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.select_policies",
        translated_text: "Seleccionar políticas",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.select_shaders",
        translated_text: "Seleccionar shaders",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.select_playlists",
        translated_text: "Seleccionar listas de reproducción",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.focus",
        translated_text: "Enfoque de exportación:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.focus_heading",
        translated_text: "Enfoque de exportación",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.focus_help",
        translated_text: "Elija si las políticas, los shaders o las listas de reproducción serán el enfoque seleccionable que determinará esta exportación.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.destination_help",
        translated_text: "Elija el directorio donde se creará el archivo portátil de exportación de Screenshaver.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.select_all",
        translated_text: "Seleccionar todo",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.clear_all",
        translated_text: "Borrar selección",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.selected_count",
        translated_text: "{selected} de {total} seleccionados",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.policies_included",
        translated_text: "Políticas incluidas ({count})",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.shaders_included",
        translated_text: "Shaders incluidos ({count})",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.playlists_included",
        translated_text: "Listas de reproducción incluidas ({count})",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.resolve_failed",
        translated_text: "No se pudieron resolver las políticas efectivas de exportación: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.policies_resolved",
        translated_text: "{count} políticas incluidas resueltas y validadas para la exportación portátil.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.select_at_least_one",
        translated_text: "Seleccione al menos un elemento de {type} para continuar.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.destination_folder",
        translated_text: "Carpeta de destino:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.browse",
        translated_text: "Examinar...",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.filename",
        translated_text: "Nombre del archivo de exportación:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.filename_invalid",
        translated_text: "Introduzca un nombre de archivo sin separadores de directorio.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.filename_exists",
        translated_text: "Ese nombre de archivo ya existe. La exportación usará: {filename}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.contents",
        translated_text: "Contenido de la exportación",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.policies_colon",
        translated_text: "Políticas:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.shaders_colon",
        translated_text: "Shaders:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.playlists_colon",
        translated_text: "Listas de reproducción:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.format",
        translated_text: "Formato de exportación",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.review_safety",
        translated_text: "No se modificará ninguna configuración de Screenshaver. La exportación crea una copia portátil de los elementos mostrados arriba.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.export_colon",
        translated_text: "Exportación:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.passed",
        translated_text: "CORRECTA",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.failed",
        translated_text: "FALLIDA",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.archive",
        translated_text: "Archivo: {path}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.result_counts",
        translated_text: "Políticas: {policies}    Shaders: {shaders}    Listas de reproducción: {playlists}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.no_archive_installed",
        translated_text: "No se instaló ningún archivo de exportación completado.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.not_run",
        translated_text: "La exportación no se ha ejecutado.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.back",
        translated_text: "< Atrás",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.action",
        translated_text: "Exportar",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.next",
        translated_text: "Siguiente >",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.finish",
        translated_text: "Finalizar",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.window_title",
        translated_text: "Importar datos de Screenshaver",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.stage.select_archive",
        translated_text: "Seleccionar archivo",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.stage.inspect_archive",
        translated_text: "Inspeccionar archivo",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.stage.resolve_conflicts",
        translated_text: "Resolver conflictos",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.review_confirm",
        translated_text: "Revisar y confirmar",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.results",
        translated_text: "Resultados",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.select_archive_help",
        translated_text: "Seleccione un archivo de exportación de Screenshaver para inspeccionarlo. En esta etapa no se modificará ningún dato de Screenshaver.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.archive_colon",
        translated_text: "Archivo:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.archive_ready",
        translated_text: "El archivo está listo para una inspección de solo lectura.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.no_archive_selected",
        translated_text: "No se ha seleccionado ningún archivo.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.inspection_not_run",
        translated_text: "La inspección del archivo no se ha ejecutado.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.archive_inspection_colon",
        translated_text: "Inspección del archivo:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.pass",
        translated_text: "CORRECTO",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.fail",
        translated_text: "FALLO",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.export_format_colon",
        translated_text: "Formato de exportación:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.source_screenshaver_colon",
        translated_text: "Screenshaver de origen:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.source_db_schema_colon",
        translated_text: "Esquema de BD de origen:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.inspection_read_only",
        translated_text: "La inspección es de solo lectura. No se ha realizado ningún cambio en Screenshaver.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.conflict.new",
        translated_text: "NUEVO",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.conflict.duplicate",
        translated_text: "DUPLICADO",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.conflict.conflict",
        translated_text: "CONFLICTO",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.conflict_help",
        translated_text: "La importación es aditiva y no destructiva. Los conflictos de nombre reales se resuelven automáticamente cambiando el nombre del objeto importado; los objetos existentes nunca se modifican ni se descartan.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.conflict_not_run",
        translated_text: "La detección de conflictos no se ha ejecutado.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.conflict_counts",
        translated_text: "{new} nuevos, {duplicates} duplicados, {conflicts} conflictos",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.automatic_rename",
        translated_text: "Resolución automática: Cambiar nombre → {name}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.rename_blocked",
        translated_text: "No se pudo generar un cambio de nombre automático; la importación está bloqueada.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.proposed_dependency_plan",
        translated_text: "Plan de dependencias propuesto",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.dependencies_resolved",
        translated_text: "Todos los objetos y dependencias del paquete tienen destinos deterministas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.conflict_read_only",
        translated_text: "Este informe es de solo lectura. No se ha modificado ninguna fila de la base de datos ni ningún archivo de shader.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.keep_existing",
        translated_text: "Conservar {type} existente ID {id}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.import_new",
        translated_text: "Importar nuevo {type}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.unresolved_conflict",
        translated_text: "Conflicto sin resolver",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.review_ready",
        translated_text: "Screenshaver está listo para aplicar a esta instalación el paquete validado con sus dependencias resueltas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.no_validated_package",
        translated_text: "No hay ningún paquete validado disponible.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.conflict_unavailable",
        translated_text: "La detección de conflictos no está disponible.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.before_import",
        translated_text: "Antes de importar",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.backup_before_changes",
        translated_text: "Screenshaver creará y verificará una copia de seguridad permanente con marca de tiempo de screenshaver.db antes de realizar cambios persistentes.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.behavior",
        translated_text: "Comportamiento de la importación",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.behavior_detail",
        translated_text: "Los archivos de shader nuevos y renombrados automáticamente se instalarán en la carpeta administrada de shaders, las políticas se restaurarán con sus ajustes de renderizado exportados y las listas de reproducción se reconstruirán en orden canónico. Los objetos existentes nunca se modifican. Los objetos duplicados realmente idénticos son reutilizados por las dependencias importadas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.toml_unchanged",
        translated_text: "screenshaver.toml no se modificará.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.no_result",
        translated_text: "No hay ningún resultado de importación disponible.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.completed",
        translated_text: "La importación se completó correctamente.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.failed",
        translated_text: "La importación falló.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.playlist_memberships_colon",
        translated_text: "Membresías de listas de reproducción:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.shaders_created_colon",
        translated_text: "Shaders creados:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.policies_created_colon",
        translated_text: "Políticas creadas:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.playlists_created_colon",
        translated_text: "Listas de reproducción creadas:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.memberships_created_colon",
        translated_text: "Membresías creadas:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.preimport_backup",
        translated_text: "Copia de seguridad de la base de datos previa a la importación:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.close",
        translated_text: "Cerrar",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.action",
        translated_text: "Importar",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.shader_identical_same_name",
        translated_text: "Ya hay contenido de shader idéntico instalado con el mismo nombre de archivo.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.shader_identical_other_name",
        translated_text: "Ya hay contenido de shader idéntico instalado como '{name}'; se conservará el shader físico existente.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.shader_name_content_conflict",
        translated_text: "El nombre de archivo ya existe en el inventario administrado de shaders, pero su contenido difiere del shader importado.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.shader_new",
        translated_text: "Ningún shader instalado tiene este contenido ni este nombre de archivo administrado.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.policy_new",
        translated_text: "No existe ninguna política {target} con este nombre de política.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.policy_duplicate",
        translated_text: "Una política existente tiene el mismo destino, contenido de shader y configuración de renderizado.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.policy_conflict",
        translated_text: "El nombre de política ya existe para este destino, pero su shader o configuración de renderizado difiere.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.policy_multiple_match",
        translated_text: "Más de una política receptora coincide inesperadamente con este nombre de política y destino.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.playlist_new",
        translated_text: "Ninguna lista de reproducción receptora tiene este nombre.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.playlist_duplicate",
        translated_text: "Una lista de reproducción existente tiene la misma descripción y las mismas políticas resueltas en el mismo orden canónico.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.playlist_conflict",
        translated_text: "El nombre de la lista de reproducción ya existe, pero su descripción o membresía resuelta difiere.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.archive_file",
        translated_text: "Archivo de exportación",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.archive_size",
        translated_text: "Tamaño del archivo",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.zip_container",
        translated_text: "Contenedor ZIP",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.zip_entry_count",
        translated_text: "Cantidad de entradas ZIP",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.zip_entry_access",
        translated_text: "Acceso a entrada ZIP",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.resource_limits",
        translated_text: "Límites de recursos del archivo",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.member_read",
        translated_text: "Lectura de miembro del archivo",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.unique_names",
        translated_text: "Nombres únicos en el archivo",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.archive_paths",
        translated_text: "Rutas del archivo",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.entry_types",
        translated_text: "Tipos de entrada del archivo",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.manifest",
        translated_text: "Manifiesto",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.export_schema",
        translated_text: "Esquema de exportación",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.format_identifier",
        translated_text: "Identificador de formato",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.shader_metadata_structure",
        translated_text: "Estructura de metadatos de shaders",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.shader_archive_paths",
        translated_text: "Rutas de shaders en el archivo",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.package_structure",
        translated_text: "Estructura del paquete",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.missing_content",
        translated_text: "Contenido faltante del archivo",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.unexpected_content",
        translated_text: "Contenido inesperado del archivo",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.manifest_counts",
        translated_text: "Conteos del manifiesto",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.validated_package",
        translated_text: "Paquete validado",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.relationships",
        translated_text: "Relaciones del paquete",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.shader_payloads",
        translated_text: "Contenido de shaders",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.shader_integrity",
        translated_text: "Integridad de shaders",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.shader_encoding",
        translated_text: "Codificación del código de shaders",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.shader_payload_inspection",
        translated_text: "Inspección del contenido de shaders",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.package_sha256",
        translated_text: "SHA-256 del paquete",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.not_regular_file",
        translated_text: "La ruta seleccionada no es un archivo normal.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.resource_limit",
        translated_text: "La expansión del archivo o una entrada individual supera los límites de inspección.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.no_duplicate_names",
        translated_text: "No se detectaron nombres de miembros ZIP duplicados.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.safe_paths",
        translated_text: "No se detectaron rutas absolutas, de recorrido, con NUL ni con barras invertidas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.safe_entry_types",
        translated_text: "No se detectaron enlaces simbólicos ni entradas de archivos especiales.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.manifest_missing",
        translated_text: "Falta el archivo obligatorio manifest.json.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.counts_match",
        translated_text: "Los conteos de políticas, shaders y listas de reproducción coinciden con los metadatos.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.counts_mismatch",
        translated_text: "Los conteos del manifiesto no coinciden con los metadatos analizados.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.integrity_record_missing",
        translated_text: "Falta el registro de integridad del manifiesto.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.sha_verified",
        translated_text: "SHA-256 verificado.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.sha_mismatch",
        translated_text: "El SHA-256 no coincide o el hash tiene un formato incorrecto.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.relationships_valid",
        translated_text: "Las referencias política→shader y lista de reproducción→política son estructuralmente válidas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.all_shader_hashes_verified",
        translated_text: "Se verificaron todos los valores SHA-256 de los shaders.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.all_shader_utf8",
        translated_text: "El contenido de todos los shaders es texto UTF-8 válido.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.package_hash_malformed",
        translated_text: "El hash del paquete en el manifiesto tiene un formato incorrecto.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.package_fingerprint_verified",
        translated_text: "Se verificó la huella digital canónica del paquete.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.package_fingerprint_mismatch",
        translated_text: "La huella digital canónica del paquete no coincide con el manifiesto.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.access_archive",
        translated_text: "No se puede acceder al archivo: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.open_archive",
        translated_text: "No se puede abrir el archivo: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.invalid_zip",
        translated_text: "Archivo ZIP no válido: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.zip_entry_error",
        translated_text: "Entrada {index}: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.member_error",
        translated_text: "{name}: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.manifest_invalid",
        translated_text: "manifest.json no es válido: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.format_supported",
        translated_text: "El formato de exportación de Screenshaver {version} es compatible.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.shader_path_not_permitted",
        translated_text: "'{path}' no está permitido por el esquema de exportación.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.shader_files_present",
        translated_text: "Hay {count} archivos de shader declarados presentes.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.playlist_members",
        translated_text: "Miembros de listas de reproducción",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.metadata_title",
        translated_text: "Metadatos de {dataset}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.metadata_hash_title",
        translated_text: "Hash de metadatos de {dataset}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.structure_title",
        translated_text: "Estructura de {dataset}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.required_file_missing",
        translated_text: "Falta el archivo requerido '{file}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.manifest_integrity_missing",
        translated_text: "Falta el registro de integridad del manifiesto.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.sha256_verified",
        translated_text: "SHA-256 verificado.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.sha256_bad",
        translated_text: "El SHA-256 no coincide o el hash tiene un formato incorrecto.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.rows_header_verified",
        translated_text: "{rows} filas; encabezado del esquema verificado.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.required_members_exact",
        translated_text: "Todos los miembros requeridos están presentes y no se encontraron archivos inesperados.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.review_count",
        translated_text: "{new} para importar, {duplicates} idénticos ya presentes",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.package_members",
        translated_text: "Miembros del paquete",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.package_relationships",
        translated_text: "Relaciones del paquete",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.shader_source_encoding",
        translated_text: "Codificación del código fuente de shaders",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.no_duplicate_names",
        translated_text: "No se detectaron nombres duplicados de miembros ZIP.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.paths_safe",
        translated_text: "No se detectaron rutas absolutas, de recorrido, con NUL ni con barras invertidas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.no_special_entries",
        translated_text: "No se detectaron enlaces simbólicos ni entradas de archivos especiales.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.manifest_missing",
        translated_text: "Falta el archivo requerido manifest.json.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.package_hash_verified",
        translated_text: "Huella digital canónica del paquete verificada.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.package_hash_mismatch",
        translated_text: "La huella digital canónica del paquete no coincide con el manifiesto.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.relationships_valid",
        translated_text: "Las referencias política→shader y lista de reproducción→política son estructuralmente válidas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.all_shader_hashes_verified",
        translated_text: "Se verificaron todos los valores SHA-256 de los shaders.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.check.all_shader_utf8",
        translated_text: "El contenido de todos los shaders es texto UTF-8 válido.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "import.dataset.playlists",
        translated_text: "Listas de reproducción",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.dataset.playlist_memberships",
        translated_text: "Membresías de listas de reproducción",
    },


    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.archive_bytes_exceed_limit",
        translated_text: "{actual} bytes supera el límite de inspección de {limit} bytes.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.archive_entries_exceed_limit",
        translated_text: "{actual} entradas supera el límite de {limit} entradas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.manifest_schema_format_mismatch",
        translated_text: "Manifiesto '{manifest}'; esquema '{schema}'.",
    },


    FactoryTranslation {
        locale: "es-US",
        key: "import.contents_colon",
        translated_text: "Contenido:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.contents_counts",
        translated_text: "{policies} políticas, {shaders} shaders, {playlists} listas de reproducción, {memberships} membresías",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.automatic_resolution_rename",
        translated_text: "Resolución automática: Renombrar → {name}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.unresolved_rename_dependency_counts",
        translated_text: "No se pudieron generar {renames} cambio(s) de nombre por conflicto; quedan {dependencies} referencia(s) de dependencia sin resolver.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.blocked_counts",
        translated_text: "La importación está bloqueada: {conflicts} conflicto(s), {dependencies} referencia(s) de dependencia sin resolver.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.review_blocked_explanation",
        translated_text: "La revisión está disponible para inspección, pero Importar permanecerá deshabilitado si no se puede generar algún cambio de nombre determinista o destino de dependencia.",
    },


    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.local_timestamp_failed",
        translated_text: "No se pudo determinar la marca de tiempo local de importación: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.unique_import_name_failed",
        translated_text: "No se pudo generar un nombre importado único para '{name}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.no_deterministic_rename",
        translated_text: "{type} '{name}' no tiene un cambio de nombre importado determinista.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.missing_package_shader",
        translated_text: "Falta el shader del paquete.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.missing_package_policy",
        translated_text: "Falta la política del paquete.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.missing_package_playlist",
        translated_text: "Falta la lista de reproducción del paquete.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.conflict_discovery_unavailable",
        translated_text: "Detección de conflictos no disponible",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.conflict_database_open_failed",
        translated_text: "No se pudo abrir screenshaver.db para la detección de conflictos de solo lectura: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.shader_hash_prepare_failed",
        translated_text: "No se pudo preparar la búsqueda del hash del shader: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.shader_hash_query_failed",
        translated_text: "No se pudo consultar el hash del shader '{hash}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.shader_hash_decode_failed",
        translated_text: "No se pudo decodificar la búsqueda del hash del shader: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.shader_filename_check_failed",
        translated_text: "No se pudo comprobar el nombre de archivo del shader '{filename}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.policy_name_prepare_failed",
        translated_text: "No se pudo preparar la búsqueda de Nombre de política: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.policy_name_query_failed",
        translated_text: "No se pudo consultar el Nombre de política '{name}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.policy_name_decode_failed",
        translated_text: "No se pudo decodificar la búsqueda de Nombre de política: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.playlist_query_failed",
        translated_text: "No se pudo consultar la lista de reproducción '{name}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.playlist_member_prepare_failed",
        translated_text: "No se pudo preparar la búsqueda de miembros de la lista de reproducción: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.playlist_members_query_failed",
        translated_text: "No se pudieron consultar los miembros de la lista de reproducción: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.playlist_members_decode_failed",
        translated_text: "No se pudieron decodificar los miembros de la lista de reproducción: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.validated_policy_missing_field",
        translated_text: "A la política validada '{policy}' le falta '{field}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.invalid_number_in_field",
        translated_text: "Número no válido '{value}' en {field}.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.invalid_integer_in_field",
        translated_text: "Entero no válido '{value}' en {field}.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.invalid_boolean_in_field",
        translated_text: "Booleano no válido '{value}' en {field}.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.unsupported_policy_target",
        translated_text: "Destino de política importada no compatible '{target}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.receiving_defaults_texture_mode",
        translated_text: "Los valores predeterminados de {target} de la instalación receptora tienen un modo de textura no compatible '{mode}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.receiving_policy_texture_mode",
        translated_text: "La política receptora tiene un modo de textura no compatible '{mode}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.receiving_defaults_palette_mode",
        translated_text: "Los valores predeterminados de {target} de la instalación receptora tienen un modo de paleta no compatible '{mode}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.diag.receiving_policy_palette_mode",
        translated_text: "La política receptora tiene un modo de paleta no compatible '{mode}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.policy_with_id",
        translated_text: "Política {id}",
    },


    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.archive_reinspection_failed",
        translated_text: "El archivo ya no supera la inspección. No se realizaron cambios de importación.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.validated_package_unavailable",
        translated_text: "El paquete validado no está disponible.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.conflict_timestamp_unavailable",
        translated_text: "La marca de tiempo de conflictos de importación no está disponible. No se realizaron cambios de importación.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.destination_name_changed",
        translated_text: "La instalación receptora cambió después de Revisar: '{previous}' ya no es el nombre de destino determinista (ahora es '{current}'). No se realizaron cambios de importación.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.conflict_set_changed",
        translated_text: "La instalación receptora cambió después de Revisar y el conjunto de conflictos ya no es el mismo. No se realizaron cambios de importación.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.new_conflicts_discovered",
        translated_text: "La instalación receptora cambió después de Revisar y se detectaron nuevos conflictos. No se realizaron cambios de importación.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.conflict_resolution_stale",
        translated_text: "La resolución de conflictos de importación está incompleta o desactualizada: {error} No se realizaron cambios de importación.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.unresolved_conflicts",
        translated_text: "La detección de conflictos durante la ejecución encontró {count} conflicto(s) sin resolver:\\n\\n{details}\\n\\nNo se realizaron cambios de importación.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.policy_unresolved_destination",
        translated_text: "La política '{name}' (ID de paquete {id}) tiene un destino sin resolver.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.policy_unresolved_shader",
        translated_text: "La política '{name}' (ID de paquete {id}) requiere el ID de paquete de shader {shader_id} sin resolver.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.playlist_unresolved_destination",
        translated_text: "La lista de reproducción '{name}' (ID de paquete {id}) tiene un destino sin resolver.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.playlist_member_unresolved_policy",
        translated_text: "El miembro {position} '{policy}' de la lista de reproducción '{playlist}' requiere el ID de paquete de política {policy_id} sin resolver.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.unresolved_dependencies",
        translated_text: "La validación de dependencias durante la ejecución encontró {count} referencia(s) de dependencia sin resolver:\\n\\n{details}\\n\\nNo se realizaron cambios de importación.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.failed_database_restored",
        translated_text: "{error} Se restauró la base de datos anterior a la importación. Los archivos de shader recién instalados se eliminaron cuando fue posible.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.failed_restore_also_failed",
        translated_text: "{error} LA RESTAURACIÓN DE LA BASE DE DATOS TAMBIÉN FALLÓ: {restore_error}. La copia de seguridad verificada permanece en '{backup}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.backup_timestamp_failed",
        translated_text: "No se pudo determinar la marca de tiempo de la copia de seguridad anterior a la importación: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.backup_create_failed",
        translated_text: "No se pudo crear la copia de seguridad de la base de datos anterior a la importación '{path}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.backup_open_verify_failed",
        translated_text: "No se pudo abrir la copia de seguridad de la base de datos anterior a la importación para verificarla: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.backup_verify_failed",
        translated_text: "No se pudo verificar la copia de seguridad de la base de datos anterior a la importación: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.backup_integrity_failed",
        translated_text: "La copia de seguridad de la base de datos anterior a la importación no superó la verificación de integridad: {result}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.restore_copy_failed",
        translated_text: "No se pudo restaurar '{database}' desde '{backup}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.restored_database_open_failed",
        translated_text: "No se pudo abrir la base de datos restaurada: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.restored_database_verify_failed",
        translated_text: "No se pudo verificar la base de datos restaurada: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.restored_database_integrity_result",
        translated_text: "La comprobación integrity_check de la base de datos restaurada devolvió '{result}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.shader_directory_create_failed",
        translated_text: "No se pudo crear el directorio administrado de shaders '{path}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.archive_reopen_failed",
        translated_text: "No se pudo volver a abrir el archivo de importación: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.zip_reopen_failed",
        translated_text: "No se pudo volver a abrir el ZIP de importación: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.shader_overwrite_refused",
        translated_text: "Se rechaza sobrescribir el archivo de shader existente inesperado '{path}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.shader_payload_read_failed",
        translated_text: "No se pudo leer la carga útil del shader '{path}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.shader_changed_after_inspection",
        translated_text: "El shader '{filename}' cambió después de la inspección; SHA-256 ya no coincide.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.shader_install_failed",
        translated_text: "No se pudo instalar el shader '{path}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.shader_reconcile_failed",
        translated_text: "No se pudieron conciliar los archivos de shader importados: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.shader_registration_failed",
        translated_text: "El shader importado '{filename}' no se registró como se esperaba: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.transaction_begin_failed",
        translated_text: "No se pudo iniciar la transacción de base de datos de importación: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.policy_no_destination_shader",
        translated_text: "La política '{name}' no tiene un shader de destino resuelto.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.playlist_import_failed",
        translated_text: "No se pudo importar la lista de reproducción '{name}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.playlist_id_no_mapping",
        translated_text: "El ID de paquete de lista de reproducción {id} no tiene asignación de destino.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.policy_id_no_membership_mapping",
        translated_text: "El ID de paquete de política {id} no tiene asignación de destino para la membresía de la lista de reproducción.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.membership_restore_failed",
        translated_text: "No se pudo restaurar la membresía de la lista de reproducción en la posición {position}: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.transaction_commit_failed",
        translated_text: "No se pudo confirmar la transacción de base de datos de importación: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.success_detail",
        translated_text: "El paquete validado se importó correctamente. Los conflictos genuinos se conservaron de forma aditiva con nombres importados deterministas; los objetos existentes de la instalación receptora no se modificaron. Los duplicados verdaderamente idénticos se reutilizaron y las dependencias importadas se asignaron a sus identidades de destino. screenshaver.toml no se modificó.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.imported_policy_missing_field",
        translated_text: "A la política importada '{policy}' le falta '{field}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.invalid_policy_integer",
        translated_text: "Entero no válido '{value}' en el campo de política importada '{field}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.invalid_policy_number",
        translated_text: "Número no válido '{value}' en el campo de política importada '{field}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.invalid_policy_boolean",
        translated_text: "Booleano no válido '{value}' en el campo de política importada '{field}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.unsupported_texture_mode",
        translated_text: "Modo de textura importado no compatible '{mode}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.unsupported_palette_mode",
        translated_text: "Modo de paleta importado no compatible '{mode}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.unsupported_animation_speed_mode",
        translated_text: "Modo de velocidad de animación importado no compatible '{mode}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.policy_import_failed",
        translated_text: "No se pudo importar la política '{name}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.imported_name_length_invalid",
        translated_text: "El nombre importado debe contener entre 1 y 128 caracteres; se encontraron {length}.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.exec.imported_name_empty_key",
        translated_text: "El nombre importado produjo una clave de comparación vacía.",
    },


    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.export_format_unsupported",
        translated_text: "El formato de exportación de Screenshaver {version} no es compatible con esta instalación.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.export_schema_load_failed",
        translated_text: "No se pudo cargar el esquema de exportación {version}: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.export_schema_version_mismatch",
        translated_text: "La versión del esquema de exportación no coincide con el archivo.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.export_schema_integrity_unsupported",
        translated_text: "El esquema de exportación solicita un comportamiento de integridad no compatible.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.export_schema_manifest_hashing",
        translated_text: "El esquema de exportación debe excluir su manifiesto del hash del paquete.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.schema_metadata_undeclared",
        translated_text: "El archivo de metadatos del esquema '{file}' no está declarado en el archivo.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.dataset_not_utf8",
        translated_text: "'{file}' no es UTF-8: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.dataset_empty",
        translated_text: "'{file}' está vacío.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.dataset_header_mismatch",
        translated_text: "El encabezado de '{file}' no coincide con el esquema de exportación.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.dataset_row_error",
        translated_text: "'{file}' fila {row}: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.dataset_column_count",
        translated_text: "La fila {row} de '{file}' tiene {actual} columnas; se requieren {required}.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.tsv_trailing_escape",
        translated_text: "Secuencia de escape TSV incompleta al final.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.tsv_escape_unsupported",
        translated_text: "Secuencia de escape TSV no compatible '\\{escape}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.schema_dataset_missing_column",
        translated_text: "Al conjunto de datos del esquema '{file}' le falta la columna '{column}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.duplicate_id",
        translated_text: "{label} {id} duplicado.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.not_positive_integer",
        translated_text: "{label} '{value}' no es un entero positivo.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.must_be_positive",
        translated_text: "{label} debe ser mayor que cero.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.policy_missing_shader_id",
        translated_text: "La política hace referencia al shader_export_id {id} que falta.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.membership_unresolved_id",
        translated_text: "La membresía de la lista de reproducción contiene un ID de paquete sin resolver.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.playlist_duplicate_policy",
        translated_text: "La lista de reproducción {playlist} contiene la política {policy} más de una vez.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.playlist_duplicate_position",
        translated_text: "La lista de reproducción {playlist} contiene la posición duplicada {position}.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.shader_declared_twice",
        translated_text: "El shader '{path}' está declarado más de una vez.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.shader_sha_malformed",
        translated_text: "El shader '{path}' tiene un SHA-256 con formato incorrecto.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.declared_shader_missing",
        translated_text: "Falta el shader declarado '{path}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.declared_shader_directory",
        translated_text: "El shader declarado '{path}' es un directorio.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.shader_executable",
        translated_text: "El shader '{path}' está marcado como ejecutable.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.shader_sha_failed",
        translated_text: "El shader '{path}' no superó la verificación SHA-256.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "import.validation.shader_not_utf8",
        translated_text: "El shader '{path}' no es texto UTF-8 válido.",
    },


    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_load_playlists_for_export_selection",
        translated_text: "No se pudieron cargar las listas de reproducción para la selección de exportación: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_load_schema",
        translated_text: "No se pudo cargar el Esquema de Exportación V1 de Screenshaver: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.schema_has_an_empty_format_identifier",
        translated_text: "El Esquema de Exportación V1 de Screenshaver tiene un identificador de formato vacío.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.schema_has_an_invalid_format_version",
        translated_text: "El Esquema de Exportación V1 de Screenshaver tiene una versión de formato no válida.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.schema_requests_an_unsupported_integrity_algorithm_or_canonicalization",
        translated_text: "El Esquema de Exportación V1 de Screenshaver solicita un algoritmo de integridad o una canonicalización no compatibles.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.schema_must_exclude_its_manifest_from_the_package_hash",
        translated_text: "El Esquema de Exportación V1 de Screenshaver debe excluir su manifiesto del hash del paquete.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.schema_dataset_is_not_declared_as_archive_metadata",
        translated_text: "El conjunto de datos '{value1}' del Esquema de Exportación V1 de Screenshaver no está declarado como metadatos del archivo.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.missing_package_local_export_id_for_policy",
        translated_text: "Falta el ID de exportación local del paquete para la política '{value1}'",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.missing_package_local_export_id_for_shader_referenced_by_policy",
        translated_text: "Falta el ID de exportación local del paquete para el shader '{value1}' al que hace referencia la política '{value2}'",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.missing_package_local_export_id_for_playlist",
        translated_text: "Falta el ID de exportación local del paquete para la lista de reproducción '{value1}'",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_load_members_for_playlist_while_exporting",
        translated_text: "No se pudieron cargar los miembros de la lista de reproducción {value1} durante la exportación: {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.missing_package_local_export_id_for_policy_in_playlist",
        translated_text: "Falta el ID de exportación local del paquete para la política {value1} en la lista de reproducción '{value2}'",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_read_shader_at",
        translated_text: "No se pudo leer el shader '{value1}' en '{value2}': {value3}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.missing_package_local_export_id_for_shader",
        translated_text: "Falta el ID de exportación local del paquete para el shader '{value1}'",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_create_in_export_archive",
        translated_text: "No se pudo crear '{value1}' en el archivo de exportación: {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_write_to_export_archive",
        translated_text: "No se pudo escribir '{value1}' en el archivo de exportación: {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.the_export_destination_is_not_valid",
        translated_text: "El destino de exportación no es válido.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.the_export_destination_folder_is_not_valid",
        translated_text: "La carpeta de destino de exportación no es válida.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.export_destination_folder_does_not_exist",
        translated_text: "La carpeta de destino de exportación no existe: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.the_effective_policy_snapshot_is_incomplete_return_to_selection_and_try_again",
        translated_text: "La instantánea de políticas efectivas está incompleta. Vuelva a la selección e inténtelo de nuevo.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_read_shader_while_calculating_package_integrity",
        translated_text: "No se pudo leer el shader '{value1}' al calcular la integridad del paquete: {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_serialize_export_manifest",
        translated_text: "No se pudo serializar el manifiesto de exportación: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_remove_stale_temporary_export",
        translated_text: "No se pudo eliminar la exportación temporal obsoleta '{value1}': {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_create_temporary_export_archive",
        translated_text: "No se pudo crear el archivo temporal de exportación '{value1}': {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_add_database_snapshot_to_backup_archive",
        translated_text: "No se pudo agregar la instantánea de la base de datos al archivo de copia de seguridad: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_write_database_snapshot_to_backup_archive",
        translated_text: "No se pudo escribir la instantánea de la base de datos en el archivo de copia de seguridad: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_add_managed_shader_to_backup_archive",
        translated_text: "No se pudo agregar el shader administrado '{value1}' al archivo de copia de seguridad: {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_write_managed_shader_to_backup_archive",
        translated_text: "No se pudo escribir el shader administrado '{value1}' en el archivo de copia de seguridad: {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_add_shader_to_export_archive",
        translated_text: "No se pudo agregar el shader '{value1}' al archivo de exportación: {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_open_shader",
        translated_text: "No se pudo abrir el shader '{value1}': {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_read_shader",
        translated_text: "No se pudo leer el shader '{value1}': {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_write_shader_to_export_archive",
        translated_text: "No se pudo escribir el shader '{value1}' en el archivo de exportación: {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_finalize_export_archive",
        translated_text: "No se pudo finalizar el archivo de exportación: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_move_completed_export_archive_to",
        translated_text: "No se pudo mover el archivo de exportación completado a '{value1}': {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_enumerate_managed_shader_directory",
        translated_text: "No se pudo enumerar el directorio de shaders administrados '{value1}': {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_read_managed_shader_directory_entry",
        translated_text: "No se pudo leer la entrada del directorio de shaders administrados: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_inspect_managed_shader_entry",
        translated_text: "No se pudo inspeccionar la entrada de shader administrado '{value1}': {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_read_managed_shader_for_full_backup",
        translated_text: "No se pudo leer el shader administrado '{value1}' para la copia de seguridad completa: {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_remove_stale_database_snapshot",
        translated_text: "No se pudo eliminar la instantánea obsoleta de la base de datos '{value1}': {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_open_database_for_backup_snapshot",
        translated_text: "No se pudo abrir la base de datos para la instantánea de copia de seguridad: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_create_consistent_database_snapshot",
        translated_text: "No se pudo crear una instantánea coherente de la base de datos: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_open_database_snapshot_for_verification",
        translated_text: "No se pudo abrir la instantánea de la base de datos para su verificación: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_verify_database_snapshot",
        translated_text: "No se pudo verificar la instantánea de la base de datos: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.database_snapshot_failed_integrity_verification",
        translated_text: "La instantánea de la base de datos no superó la verificación de integridad: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_read_verified_database_snapshot",
        translated_text: "No se pudo leer la instantánea verificada de la base de datos '{value1}': {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_create_backup_directory",
        translated_text: "No se pudo crear el directorio de copia de seguridad '{value1}': {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_prepare_full_backup_selection",
        translated_text: "No se pudo preparar la selección de copia de seguridad completa: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_resolve_full_backup_policies",
        translated_text: "No se pudieron resolver las políticas de copia de seguridad completa: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_determine_backup_filename_timestamp",
        translated_text: "No se pudo determinar la marca de tiempo del nombre de archivo de la copia de seguridad.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_prepare_effective_export_policy_query",
        translated_text: "No se pudo preparar la consulta de políticas efectivas de exportación: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_read_selected_policy_id_while_resolving_export_data",
        translated_text: "No se pudo leer el ID de política seleccionado {value1} al resolver los datos de exportación: {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.policy_has_unsupported_target",
        translated_text: "La política '{value1}' tiene un destino no compatible '{value2}'",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.one_or_more_selected_policies_disappeared_while_export_data_was_being_resolved",
        translated_text: "Una o más políticas seleccionadas desaparecieron mientras se resolvían los datos de exportación",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.policy_has_a_specific_texture_without_a_texture_family",
        translated_text: "La política '{value1}' tiene una textura específica sin una familia de texturas",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.policy_has_a_specific_texture_without_a_primitive_count",
        translated_text: "La política '{value1}' tiene una textura específica sin un recuento de primitivas",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.policy_has_unsupported_texture_mode",
        translated_text: "La política '{value1}' tiene un modo de textura no compatible '{value2}'",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.defaults_specify_a_specific_texture_without_a_texture_family_while_resolving_policy",
        translated_text: "Los valores predeterminados de {value1} especifican una textura concreta sin una familia de texturas al resolver la política '{value2}'",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.defaults_contain_unsupported_texture_mode_while_resolving_policy",
        translated_text: "Los valores predeterminados de {value1} contienen un modo de textura no compatible '{value2}' al resolver la política '{value3}'",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.policy_has_a_specific_palette_without_a_palette_color",
        translated_text: "La política '{value1}' tiene una paleta específica sin un color de paleta",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.policy_has_unsupported_palette_mode",
        translated_text: "La política '{value1}' tiene un modo de paleta no compatible '{value2}'",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.defaults_specify_a_specific_palette_without_a_palette_color_while_resolving_policy",
        translated_text: "Los valores predeterminados de {value1} especifican una paleta concreta sin un color de paleta al resolver la política '{value2}'",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.defaults_contain_unsupported_palette_mode_while_resolving_policy",
        translated_text: "Los valores predeterminados de {value1} contienen un modo de paleta no compatible '{value2}' al resolver la política '{value3}'",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.policy_has_invalid_boolean_value_for",
        translated_text: "La política '{value1}' tiene el valor booleano no válido {value2} para {value3}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.assigned_policy_retained_unresolved_target_texture_inheritance",
        translated_text: "La política asignada '{value1}' conservó una herencia de textura de destino sin resolver",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.assigned_policy_retained_unresolved_target_palette_inheritance",
        translated_text: "La política asignada '{value1}' conservó una herencia de paleta de destino sin resolver",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.assigned_policy_retained_unresolved_target_animation_speed_inheritance",
        translated_text: "La política asignada '{value1}' conservó una herencia de velocidad de animación de destino sin resolver",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.policy_resolved_to_an_invalid_specific_texture",
        translated_text: "La política '{value1}' se resolvió a una textura específica no válida",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.assigned_policy_resolved_to_random_texture_without_an_explicit_primitive_count",
        translated_text: "La política asignada '{value1}' se resolvió a una textura aleatoria sin un recuento explícito de primitivas",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.policy_resolved_to_an_invalid_random_texture_primitive_count",
        translated_text: "La política '{value1}' se resolvió a un recuento de primitivas de textura aleatoria no válido",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.policy_resolved_to_one_or_more_invalid_numeric_export_values",
        translated_text: "La política '{value1}' se resolvió a uno o más valores numéricos de exportación no válidos",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.policy_resolved_to_an_invalid_animation_speed",
        translated_text: "La política '{value1}' se resolvió a una velocidad de animación no válida",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_prepare_export_policy_selection_query",
        translated_text: "No se pudo preparar la consulta de selección de políticas de exportación: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_query_policies_for_export_selection",
        translated_text: "No se pudieron consultar las políticas para la selección de exportación: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.error.unable_to_decode_export_policy_selection_row",
        translated_text: "No se pudo decodificar la fila de selección de políticas de exportación: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "export.review_instruction",
        translated_text: "Revise el enfoque de exportación seleccionado, las políticas, los shaders y las listas de reproducción incluidos, y el destino antes de iniciar la exportación.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.edit_shader_requires_a_shader_file_not_a_directory",
        translated_text: "--edit-shader requiere un archivo de shader, no un directorio: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.bulk_edit_complete_policies_updated",
        translated_text: "Edición masiva completada: {value1} políticas actualizadas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.bulk_edit_complete_policies_updated_policy_target_was_preserved_for_protect",
        translated_text: "Edición masiva completada: {value1} políticas actualizadas. Se conservó el destino de política para {value2} {value3} predeterminada protegida.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.bulk_edit_contains_no_changed_settings",
        translated_text: "La edición masiva no contiene ajustes modificados.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.bulk_edit_could_not_suspend_the_active_shader_because_its_database_id_could",
        translated_text: "La edición masiva no pudo suspender el shader activo porque no se encontró su ID de base de datos.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.bulk_edit_could_not_suspend_the_active_shader",
        translated_text: "La edición masiva no pudo suspender el shader activo: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.bulk_edit_ended_but_the_previous_shader_could_not_be_reloaded",
        translated_text: "La edición masiva finalizó, pero no se pudo volver a cargar el shader anterior: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.bulk_edit_ended_the_previously_loaded_shader_is_no_longer_available",
        translated_text: "La edición masiva finalizó; el shader cargado anteriormente ya no está disponible.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.bulk_policies_were_saved_but_configuration_reload_failed",
        translated_text: "Las políticas masivas se guardaron, pero no se pudo volver a cargar la configuración.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.bulk_policy_creation_canceled",
        translated_text: "Se canceló la creación masiva de políticas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.bulk_policy_creation_complete_created_already_existed",
        translated_text: "Creación masiva de políticas completada: {value1} creadas, {value2} ya existían.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.bulk_policy_creation_failed",
        translated_text: "Falló la creación masiva de políticas: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.bulk_policy_save_aborted",
        translated_text: "Se canceló el guardado masivo de políticas: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.configuration_save_failed",
        translated_text: "No se pudo guardar la configuración.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.configuration_saved",
        translated_text: "Configuración guardada.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.gl_shader_files",
        translated_text: "Archivos de shader GL",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.loaded_and_rendering",
        translated_text: "Cargado y renderizando",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.loaded_existing_screensaver_policy_for_this_shader",
        translated_text: "Se cargó la política de salvapantallas existente para este shader.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.loaded_existing_unassigned_policy_for_this_shader",
        translated_text: "Se cargó la política sin asignar existente para este shader.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.loaded_existing_wallpaper_policy_for_this_shader",
        translated_text: "Se cargó la política de fondo de pantalla existente para este shader.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.loaded_existing_policy_for_this_shader",
        translated_text: "Se cargó la política {value1} existente para este shader.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.loaded_shader_using_resolved_defaults_select_a_policy_target_to_create_a_po",
        translated_text: "Se cargó el shader usando los valores predeterminados resueltos. Seleccione un destino de política para crear una política.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.loaded_shader_with_its_existing_screensaver_policy",
        translated_text: "Se cargó el shader con su política de salvapantallas existente.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.loaded_shader_with_its_existing_unassigned_policy",
        translated_text: "Se cargó el shader con su política sin asignar existente.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.loaded_shader_with_its_existing_wallpaper_policy",
        translated_text: "Se cargó el shader con su política de fondo de pantalla existente.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.new_unassigned_policy_is_ready_to_save",
        translated_text: "La nueva política sin asignar está lista para guardarse.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.no_screensaver_policy_exists_loaded_screensaver_defaults",
        translated_text: "No existe una política de salvapantallas. Se cargaron los valores predeterminados de salvapantallas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.no_unassigned_policy_exists_loaded_defaults_for_a_new_unassigned_policy",
        translated_text: "No existe una política sin asignar. Se cargaron los valores predeterminados para una nueva política sin asignar.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.no_wallpaper_policy_exists_loaded_wallpaper_defaults",
        translated_text: "No existe una política de fondo de pantalla. Se cargaron los valores predeterminados de fondo de pantalla.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.no_existing_shader_policy_found_select_a_policy_target_to_create_one",
        translated_text: "No se encontró una política de shader existente. Seleccione un destino de política para crear una.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.no_policies_changed_policy_target_cannot_be_changed_for_protected_default",
        translated_text: "No se modificó ninguna política. El destino de política no puede cambiarse para {value1} {value2} predeterminada protegida.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.no_policy_target_was_selected_for_external_shader",
        translated_text: "No se seleccionó un destino de política para el shader externo {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.no_shader_path_was_supplied_for_editing",
        translated_text: "No se proporcionó una ruta de shader para editar",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.no_usable_shaders_were_selected_for_policy_creation",
        translated_text: "No se seleccionaron shaders utilizables para crear políticas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.no_policy_exists_loaded_defaults",
        translated_text: "No existe una política {value1}. Se cargaron los valores predeterminados de {value2}.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.not_required",
        translated_text: "No requerido",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.playlist_display_mode_requires_a_playlist_selection",
        translated_text: "El modo de visualización Lista de reproducción requiere seleccionar una lista de reproducción.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.playlist_display_mode_requires_a_positive_interval",
        translated_text: "El modo de visualización Lista de reproducción requiere un intervalo positivo.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.policies_were_created_but_configuration_reload_failed",
        translated_text: "Se crearon las políticas, pero no se pudo volver a cargar la configuración.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.policy_has_unsupported_policy_target",
        translated_text: "La política '{value1}' tiene un policy_target no compatible '{value2}'",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.policy_cannot_be_opened_because_its_shader_is_not_renderable",
        translated_text: "La política no se puede abrir porque su shader no se puede renderizar: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.policy_cloned_as",
        translated_text: "Política clonada como '{value1}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.policy_paths_could_not_be_updated_after_moving_the_shader_rollback_also_fai",
        translated_text: "No se pudieron actualizar las rutas de las políticas después de mover el shader: {value1}. La reversión también falló: {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.policy_renamed_to",
        translated_text: "Política renombrada a '{value1}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.policy_saved_for",
        translated_text: "Política guardada para {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.policy_saved_but_audio_motion_could_not_be_saved",
        translated_text: "La política se guardó, pero no se pudo guardar Movimiento de audio: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.policy_shader_file_is_unavailable",
        translated_text: "El archivo de shader de la política no está disponible: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.policy_was_cloned_but_configuration_reload_failed",
        translated_text: "La política se clonó, pero no se pudo volver a cargar la configuración: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.policy_was_renamed_but_configuration_reload_failed",
        translated_text: "La política se renombró, pero no se pudo volver a cargar la configuración: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.recent_files_were_cleared_for_this_session_but_the_history_file_could_not_b",
        translated_text: "Los archivos recientes se borraron para esta sesión, pero no se pudo actualizar el archivo de historial: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.recent_shader_file_no_longer_exists",
        translated_text: "El archivo de shader reciente ya no existe: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.recent_shader_file_history_cleared",
        translated_text: "Se borró el historial de archivos de shader recientes.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.refreshed_shader_from_disk",
        translated_text: "Shader actualizado desde el disco: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.sdl_initialization_failed",
        translated_text: "Falló la inicialización de SDL: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.sdl_video_initialization_failed",
        translated_text: "Falló la inicialización de video SDL: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.screensaver_target_enforced_by_shader_location_new_screensaver_policy_is_re",
        translated_text: "El destino Salvapantallas se impuso por la ubicación del shader. La nueva política de salvapantallas está lista para guardarse.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.control_center",
        translated_text: "Centro de control de Screenshaver",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.export_archive",
        translated_text: "Archivo de exportación de Screenshaver",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.shader_has_file_status",
        translated_text: "El shader '{value1}' tiene file_status '{value2}'",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.shader_has_invalid_channel_usage_mask",
        translated_text: "El shader '{value1}' tiene una máscara de uso de canales no válida {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.shader_has_validation_status_expected",
        translated_text: "El shader '{value1}' tiene validation_status '{value2}'; se esperaba 'valid'",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.shader_is_not_registered_in_the_database",
        translated_text: "El shader '{value1}' no está registrado en la base de datos de Screenshaver",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.shader_is_rejected",
        translated_text: "El shader '{value1}' está rechazado: {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.shader_already_exists_in",
        translated_text: "El shader ya existe en {value1}.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.shader_file_is_unavailable",
        translated_text: "El archivo de shader no está disponible: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.shader_file_no_longer_exists",
        translated_text: "El archivo de shader ya no existe: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.shader_filename_is_not_valid_utf_8",
        translated_text: "El nombre del archivo de shader no es UTF-8 válido: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.shader_loaded_but_recent_file_history_could_not_be_saved",
        translated_text: "El shader se cargó, pero no se pudo guardar el historial de archivos recientes: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.shader_loading_canceled",
        translated_text: "Se canceló la carga del shader.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.shader_move_was_rolled_back_because_policy_paths_could_not_be_updated",
        translated_text: "El movimiento del shader se revirtió porque no se pudieron actualizar las rutas de las políticas: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.shader_moved_to",
        translated_text: "Shader movido a {value1}.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.shader_moved_to_policy_target_updated_to",
        translated_text: "Shader movido a {value1}. Destino de política actualizado a {value2}.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.shader_moved_configuration_reload_failed",
        translated_text: "El shader se movió; no se pudo volver a cargar la configuración.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.shader_path_has_no_valid_filename",
        translated_text: "La ruta del shader no tiene un nombre de archivo válido: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.shader_was_not_deleted_because_its_associated_policy_could_not_be_deleted",
        translated_text: "El shader no se eliminó porque no se pudo eliminar su política asociada: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.single_display_mode_requires_a_shader_policy_selection",
        translated_text: "El modo de visualización Único requiere seleccionar una política de shader.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.the_selected_policy_target_is_unavailable_in_the_current_editing_session",
        translated_text: "El destino de política seleccionado no está disponible en la sesión de edición actual.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.the_selected_shader_could_not_be_loaded_for_editing",
        translated_text: "No se pudo cargar el shader seleccionado para editarlo",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.this_shader_cannot_use_a_screensaver_policy_in_the_current_editing_session",
        translated_text: "Este shader no puede usar una política de salvapantallas en la sesión de edición actual.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.this_shader_cannot_use_a_wallpaper_policy_in_the_current_editing_session",
        translated_text: "Este shader no puede usar una política de fondo de pantalla en la sesión de edición actual.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.this_shader_cannot_use_an_unassigned_policy_in_the_current_editing_session",
        translated_text: "Este shader no puede usar una política sin asignar en la sesión de edición actual.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_clone_policy",
        translated_text: "No se pudo clonar la política: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_create_control_center_state_folder",
        translated_text: "No se pudo crear la carpeta de estado del Centro de control {value1}: {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_create_opengl_context",
        translated_text: "No se pudo crear el contexto OpenGL: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_create_sdl_event_pump",
        translated_text: "No se pudo crear la bomba de eventos SDL: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_create_destination_directory",
        translated_text: "No se pudo crear el directorio de destino {value1} ({value2})",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_create_edit_shader_opengl_context",
        translated_text: "No se pudo crear el contexto OpenGL de edición de shader: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_create_edit_shader_sdl_event_pump",
        translated_text: "No se pudo crear la bomba de eventos SDL de edición de shader: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_create_edit_shader_window",
        translated_text: "No se pudo crear la ventana de edición de shader: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_decode_policy_list_database_row",
        translated_text: "No se pudo decodificar la fila de la base de datos de la Lista de políticas: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_delete_policy",
        translated_text: "No se pudo eliminar la política: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_load_shader",
        translated_text: "No se pudo cargar el shader: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_move_shader_from_to",
        translated_text: "No se pudo mover el shader de {value1} a {value2} ({value3})",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_open_database_while_reading_shader_metadata_for",
        translated_text: "No se pudo abrir la base de datos al leer los metadatos del shader para '{value1}': {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_prepare_policy_list_database_query",
        translated_text: "No se pudo preparar la consulta de base de datos de la Lista de políticas: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_prepare_policy_clone",
        translated_text: "No se pudo preparar el clon de la política: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_query_policy_list_rows_from_database",
        translated_text: "No se pudieron consultar las filas de la Lista de políticas en la base de datos: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_query_shader_id_for",
        translated_text: "No se pudo consultar el ID del shader para '{value1}': {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_read_database_metadata_for",
        translated_text: "No se pudieron leer los metadatos de la base de datos para '{value1}': {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_refresh_shader",
        translated_text: "No se pudo actualizar el shader: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_rename_policy",
        translated_text: "No se pudo renombrar la política: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_resolve_shader_id",
        translated_text: "No se pudo resolver shader_id {value1}: {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_restore_control_center_fullscreen_state",
        translated_text: "No se pudo restaurar el estado de pantalla completa del Centro de control de Screenshaver: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_restore_the_previous_shader_after_bulk_edit",
        translated_text: "No se pudo restaurar el shader anterior después de la edición masiva: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_save_bulk_policy_changes",
        translated_text: "No se pudieron guardar los cambios masivos de políticas: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_save_policy",
        translated_text: "No se pudo guardar la política: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_serialize_control_center_state",
        translated_text: "No se pudo serializar el estado del Centro de control: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unable_to_write_control_center_state",
        translated_text: "No se pudo escribir el estado del Centro de control {value1}: {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.unsupported_display_mode",
        translated_text: "Modo de visualización no compatible '{value1}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.valid_shader_has_no_channel_usage_metadata",
        translated_text: "El shader válido '{value1}' no tiene metadatos de uso de canales",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.wallpaper_target_enforced_by_shader_location_new_wallpaper_policy_is_ready",
        translated_text: "El destino Fondo de pantalla se impuso por la ubicación del shader. La nueva política de fondo de pantalla está lista para guardarse.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.shader_is_rejected_2",
        translated_text: "el shader está rechazado",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.policy_deleted_for",
        translated_text: "Política {value1} eliminada para {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.policy_was_deleted_but_the_shader_file_could_not_be_deleted",
        translated_text: "Se eliminó la política {value1}, pero no se pudo eliminar el archivo de shader: {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.shader_and_associated_policy_deleted",
        translated_text: "Shader {value1} y política {value2} asociada eliminados: {value3}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "target.unassigned",
        translated_text: "Sin asignar",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.bulk_edit_no_changes_protected_default_one",
        translated_text: "No se modificó ninguna política. El destino de política no puede cambiarse para {value1} política predeterminada protegida.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.bulk_edit_no_changes_protected_default_many",
        translated_text: "No se modificó ninguna política. El destino de política no puede cambiarse para {value1} políticas predeterminadas protegidas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.bulk_edit_complete_protected_default_one",
        translated_text: "Edición masiva completada: {value1} políticas actualizadas. Se conservó el destino de política para {value2} política predeterminada protegida.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.bulk_edit_complete_protected_default_many",
        translated_text: "Edición masiva completada: {value1} políticas actualizadas. Se conservó el destino de política para {value2} políticas predeterminadas protegidas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.select_policy_target_before_saving",
        translated_text: "Seleccione un destino de política antes de guardar",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.delete_shader_from_policy_context_menu",
        translated_text: "Eliminar shader está disponible en el menú contextual de la fila de Políticas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.select_target_for_selected_unassigned_policies",
        translated_text: "Seleccione Salvapantallas o Fondo de pantalla como destino de política para las políticas Sin asignar seleccionadas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "edit.required",
        translated_text: "Requerido",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "assign_shader_policies.all_screensavers",
        translated_text: "Todos como protectores de pantalla",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "assign_shader_policies.all_wallpapers",
        translated_text: "Todos como fondos de pantalla",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "assign_shader_policies.screensavers_and_wallpapers",
        translated_text: "Protectores de pantalla + fondos de pantalla",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "assign_shader_policies.all_unassigned",
        translated_text: "Todos sin asignar",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "assign_shader_policies.assignment_title",
        translated_text: "Asignar nuevas políticas de shader",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "assign_shader_policies.assignment_message.singular",
        translated_text: "Screenshaver encontró {count} shader en la carpeta de shaders administrados que aún no tiene una política.\n\nElija cómo se debe crear una política para este shader.\n\nLas políticas sin asignar no se pueden renderizar hasta que su Destino de política se cambie a Protector de pantalla o Fondo de pantalla.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "assign_shader_policies.assignment_message.plural",
        translated_text: "Screenshaver encontró {count} shaders en la carpeta de shaders administrados que aún no tienen una política.\n\nElija cómo se deben crear las políticas para estos shaders.\n\nLas políticas sin asignar no se pueden renderizar hasta que su Destino de política se cambie a Protector de pantalla o Fondo de pantalla.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "assign_shader_policies.completion_title",
        translated_text: "Políticas de shader creadas",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "assign_shader_policies.completion_message.one_policy_one_shader",
        translated_text: "Screenshaver creó {policy_count} política de shader para {shader_count} shader usando \"{assignment}\".\n\nPuede revisar o cambiar las políticas de shader en cualquier momento ejecutando:\n\nscreenshaver --control\n\nEsto abre el Centro de control de Screenshaver.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "assign_shader_policies.completion_message.one_policy_many_shaders",
        translated_text: "Screenshaver creó {policy_count} política de shader para {shader_count} shaders usando \"{assignment}\".\n\nPuede revisar o cambiar las políticas de shader en cualquier momento ejecutando:\n\nscreenshaver --control\n\nEsto abre el Centro de control de Screenshaver.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "assign_shader_policies.completion_message.many_policies_one_shader",
        translated_text: "Screenshaver creó {policy_count} políticas de shader para {shader_count} shader usando \"{assignment}\".\n\nPuede revisar o cambiar las políticas de shader en cualquier momento ejecutando:\n\nscreenshaver --control\n\nEsto abre el Centro de control de Screenshaver.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "assign_shader_policies.completion_message.many_policies_many_shaders",
        translated_text: "Screenshaver creó {policy_count} políticas de shader para {shader_count} shaders usando \"{assignment}\".\n\nPuede revisar o cambiar las políticas de shader en cualquier momento ejecutando:\n\nscreenshaver --control\n\nEsto abre el Centro de control de Screenshaver.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "assign_shader_policies.error.prepare_query",
        translated_text: "No se pudo preparar la consulta de shaders sin política: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "assign_shader_policies.error.query_shaders",
        translated_text: "No se pudieron consultar los shaders sin política: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "assign_shader_policies.error.decode_row",
        translated_text: "No se pudo decodificar la fila del shader sin política: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "assign_shader_policies.error.display_dialog",
        translated_text: "No se pudo mostrar el diálogo de asignación de nuevas políticas: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "assign_shader_policies.error.unknown_button",
        translated_text: "El diálogo de asignación de nuevas políticas devolvió el identificador de botón desconocido {button_id}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "assign_shader_policies.error.begin_transaction",
        translated_text: "No se pudo iniciar la transacción de asignación de nuevas políticas: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "assign_shader_policies.error.recheck_policies",
        translated_text: "No se pudieron volver a comprobar las políticas de '{filename}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "assign_shader_policies.error.commit_transaction",
        translated_text: "No se pudo confirmar la transacción de asignación de nuevas políticas: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "assign_shader_policies.error.create_policy",
        translated_text: "No se pudo crear la política {target} para '{filename}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "assign_shader_policies.error.validate_policy_name",
        translated_text: "No se pudo validar el Nombre de política generado '{policy_name}' para el destino {target}: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "assign_shader_policies.error.generate_policy_name",
        translated_text: "No se pudo generar un Nombre de política sugerido disponible para '{filename}' en el destino {target}",
    },



    FactoryTranslation {
        locale: "es-US",
        key: "authentication.error.initialize_pam",
        translated_text: "No se pudo inicializar el servicio PAM '{service}': {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "authentication.error.configure_failure_delay",
        translated_text: "No se pudo configurar el retraso por fallo de autenticación de PAM: {error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "authentication.error.pam_authentication",
        translated_text: "Error de autenticación PAM: {error}",
    },



    FactoryTranslation {
        locale: "es-US",
        key: "compile_shader.kind.vertex",
        translated_text: "Vértice",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "compile_shader.kind.fragment",
        translated_text: "Fragmento",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "compile_shader.kind.unknown",
        translated_text: "Desconocido",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "compile_shader.error.create_shader_object",
        translated_text: "No se pudo crear el objeto de shader {kind} de OpenGL",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "compile_shader.error.interior_null",
        translated_text: "El código fuente del shader {kind} contenía un byte nulo interno",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "compile_shader.error.create_program_object",
        translated_text: "No se pudo crear el objeto de programa de shader de OpenGL",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "compile_shader.error.link_no_diagnostic",
        translated_text: "Falló el enlazado del programa de shader sin un diagnóstico de OpenGL",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "compile_shader.error.link_failed",
        translated_text: "Falló el enlazado del programa de shader:\n{error}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "compile_shader.error.compile_no_diagnostic",
        translated_text: "Falló la compilación del shader {kind} sin un diagnóstico de OpenGL",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "compile_shader.error.compile_failed",
        translated_text: "Falló la compilación del shader {kind}:\n{error}",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "qbe.query",
        translated_text: "Consultar",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "qbe.clear",
        translated_text: "Limpiar",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "qbe.boolean.true",
        translated_text: "Verdadero",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "qbe.boolean.false",
        translated_text: "Falso",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "qbe.status.ok",
        translated_text: "Correcto",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "qbe.status.rejected",
        translated_text: "Rechazado",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "qbe.status.compile_error",
        translated_text: "Error de compilación",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "qbe.status.missing",
        translated_text: "Falta",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "qbe.status.unreadable",
        translated_text: "Ilegible",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "preview.target_not_found",
        translated_text: "No se encontró el archivo o directorio del shader: {path}",
    },



    FactoryTranslation {
        locale: "es-US",
        key: "tray.tooltip.waiting_for_idle",
        translated_text: "Esperando inactividad...",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "tray.status.enabled",
        translated_text: "Activado",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "tray.status.disabled",
        translated_text: "Desactivado",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "tray.status.starting",
        translated_text: "Iniciando...",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "tray.menu.screensaver_status",
        translated_text: "Protector de pantalla: {status}",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "tray.menu.wallpaper",
        translated_text: "Fondo de pantalla:",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "tray.menu.edit",
        translated_text: "Editar",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "tray.menu.restart",
        translated_text: "Reiniciar",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "tray.menu.stop",
        translated_text: "Detener",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.runtime.disabled_by_config",
        translated_text: "[WALLPAPER] El fondo de pantalla está deshabilitado por screenshaver.toml",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.runtime.starting",
        translated_text: "[WALLPAPER] Iniciando el entorno de ejecución automático del fondo de pantalla",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.runtime.stopped_cleanly",
        translated_text: "[WALLPAPER] El entorno de ejecución del fondo de pantalla se detuvo correctamente",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.runtime.attempt_failed",
        translated_text: "[WALLPAPER] El intento {attempt}/{maximum} del entorno de ejecución falló: {error}",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.runtime.attempt_panicked",
        translated_text: "[WALLPAPER] El intento {attempt}/{maximum} del entorno de ejecución produjo un pánico",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.runtime.disabled_after_failures",
        translated_text: "[WALLPAPER] Fondo de pantalla deshabilitado durante la sesión actual después de errores repetidos",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.runtime.thread_create_failed",
        translated_text: "[WALLPAPER] No se pudo crear el hilo del fondo de pantalla: {error}",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.runtime.supervisor_panicked_shutdown",
        translated_text: "[WALLPAPER] El hilo supervisor del fondo de pantalla produjo un pánico durante el cierre.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.test.window_title",
        translated_text: "Prueba de renderizado de fondo de pantalla de Screenshaver",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.test.error.sdl_initialization",
        translated_text: "Error al inicializar SDL: {error}",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.test.error.sdl_video_initialization",
        translated_text: "Error al inicializar el video de SDL: {error}",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.test.error.create_window",
        translated_text: "No se pudo crear la ventana de prueba de renderizado del fondo de pantalla: {error}",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.test.error.create_opengl_context",
        translated_text: "No se pudo crear el contexto OpenGL de prueba del fondo de pantalla: {error}",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.test.error.create_event_pump",
        translated_text: "No se pudo crear el sistema de eventos de prueba del fondo de pantalla: {error}",
    },


    FactoryTranslation {
        locale: "es-US",
        key: "xfce.lock.desktop.comment",
        translated_text: "Presentación de shaders de Screenshaver para la pantalla de bloqueo de Xfce",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.notification.title",
        translated_text: "Fondo de pantalla de Screenshaver",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.notification.policy",
        translated_text: "Política: {policy} ({speed})",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.notification.performance_warning",
        translated_text: "Advertencia de rendimiento",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.notification.performance_critical",
        translated_text: "Rendimiento CRÍTICO",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.notification.fps",
        translated_text: "FPS: {fps}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.notification.texture",
        translated_text: "Textura: {texture}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.notification.palette",
        translated_text: "Paleta: {palette}",
    },


    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.cli.version",
        translated_text: "Screenshaver {version}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.cli.configuration_heading",
        translated_text: "Configuración del modo de fondo de pantalla:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.cli.shader_mode",
        translated_text: "    Modo de shader: {mode}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.cli.monitor_mode",
        translated_text: "    Modo de monitor: {mode}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.cli.animation_speed",
        translated_text: "    Velocidad global de animación: {speed}x",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.cli.notifications",
        translated_text: "    Notificaciones: {state}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.cli.directory",
        translated_text: "    Directorio de fondos de pantalla: {path}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.cli.eligible_count",
        translated_text: "Shaders de fondo de pantalla elegibles: {count}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.cli.no_eligible_shaders",
        translated_text: "    No se encontraron shaders presentes con una política de Fondo de pantalla.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.cli.not_started",
        translated_text: "No se inició el renderizado del fondo de pantalla.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.cli.rotation_disabled_single_shader",
        translated_text: "Rotación del fondo de pantalla desactivada: solo hay un shader elegible disponible.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "wallpaper.cli.lyrics_enabled_windowpaper",
        translated_text: "Administrador de letras sincronizadas: habilitado para Windowpaper",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "texture.family.marble",
        translated_text: "Mármol",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "texture.family.clouds",
        translated_text: "Nubes",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "texture.family.cells",
        translated_text: "Celdas",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "texture.family.mesh",
        translated_text: "Malla",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "texture.family.radial",
        translated_text: "Radial",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "texture.family.noise",
        translated_text: "Ruido",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "texture.family.bricks",
        translated_text: "Ladrillos",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "texture.family.hexagons",
        translated_text: "Hexágonos",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "texture.family.facets",
        translated_text: "Facetas",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "texture.family.skulls",
        translated_text: "Calaveras",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "texture.family.scales",
        translated_text: "Escamas",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "texture.family.eyes",
        translated_text: "Ojos",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "overlay.collect_more_shaders",
        translated_text: "Obtén más shaders en https://editor.isf.video/shaders y https://shadertoy.com/browse",
    },

    // Query By Example validation and parse errors.
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.validation.first_item_blank",
        translated_text: "El primer elemento QBE está vacío.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.validation.first_operator_blank",
        translated_text: "El primer operador QBE está vacío.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.validation.first_value_blank",
        translated_text: "El primer valor QBE está vacío.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.validation.first_operator_invalid",
        translated_text: "El primer operador QBE no es válido para el elemento seleccionado.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.validation.second_item_blank",
        translated_text: "El segundo elemento QBE está vacío.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.validation.second_operator_blank",
        translated_text: "El segundo operador QBE está vacío.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.validation.second_value_blank",
        translated_text: "El segundo valor QBE está vacío.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.validation.second_operator_invalid",
        translated_text: "El segundo operador QBE no es válido para el elemento seleccionado.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.error.invalid_boolean",
        translated_text: "El valor booleano QBE '{value}' debe ser true o false.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.error.invalid_integer",
        translated_text: "El valor QBE '{value}' no es un entero válido.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.error.invalid_decimal",
        translated_text: "El valor QBE '{value}' no es un número decimal válido.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.error.invalid_date",
        translated_text: "La fecha QBE '{value}' debe ser una fecha válida con formato MM/DD/YYYY.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.error.invalid_shader_type",
        translated_text: "Tipo de shader desconocido '{value}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.error.invalid_policy_target",
        translated_text: "Destino de política desconocido '{value}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.error.invalid_status",
        translated_text: "Estado de shader desconocido '{value}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.error.invalid_anti_aliasing",
        translated_text: "Valor de antialiasing desconocido '{value}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.error.invalid_dithering",
        translated_text: "Valor de tramado desconocido '{value}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.error.invalid_color_precision",
        translated_text: "Valor de precisión de color desconocido '{value}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.error.invalid_audiovisual_effect",
        translated_text: "Valor de efecto audiovisual desconocido '{value}'.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "editor.shader_file_cannot_be_accessed",
        translated_text: "No se puede acceder al archivo del shader:\\\n{value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.shader_validation_failed",
        translated_text: "La validación del shader falló.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.shader_cannot_be_rendered_status_reason_see_screenshaver_log_for",
        translated_text: "El shader no se puede renderizar.\\\nEstado: {value1}\\\nMotivo: {value2}\\\nConsulte screenshaver.log para obtener más detalles.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.shader_validation_state_is_unavailable",
        translated_text: "El estado de validación del shader no está disponible.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.unassigned_policy_this_shader_cannot_be_rendered_until_its_policy",
        translated_text: "Política sin asignar — este shader no se puede renderizar hasta que su Destino de política se cambie a Protector de pantalla o Fondo de pantalla.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.shader_is_accessible_and_validated",
        translated_text: "El shader es accesible y está validado:\\\n{value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.unable_to_create_egui_opengl_painter",
        translated_text: "No se pudo crear el pintor OpenGL de egui: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.unable_to_decode_embedded_control_center_branding_image",
        translated_text: "No se pudo decodificar la imagen de marca integrada del Centro de control: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.embedded_control_center_branding_image_has_invalid_dimensions",
        translated_text: "La imagen de marca integrada del Centro de control tiene dimensiones no válidas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.screenshaver_control_center_esc_or_q_to_exit",
        translated_text: "Centro de control de Screenshaver (ESC o Q para salir)",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.bulk_edit_mode_active_click_cancel_to_return_to_single",
        translated_text: "Modo de edición masiva activo-- haga clic en Cancelar para volver al modo de edición individual.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.select",
        translated_text: "Seleccionar...",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.policy_target",
        translated_text: "Destino de política:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.no_change",
        translated_text: "Sin cambios",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.leave_policy_target_unchanged_for_every_checked_policy",
        translated_text: "Dejar el Destino de política sin cambios para cada política marcada.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.load_or_create_the_policy_used_for_screensaver_rendering",
        translated_text: "Cargar o crear la política usada para renderizar el protector de pantalla.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.this_editing_session_was_opened_for_the_active_wallpaper_only",
        translated_text: "Esta sesión de edición se abrió para el fondo de pantalla activo. Solo se puede editar la política de Fondo de pantalla.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.this_shader_is_unavailable_for_screensaver_use_because_it_does",
        translated_text: "Este shader no está disponible para usarse como Protector de pantalla porque no existe en la carpeta de protectores de pantalla.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.save_or_cancel_the_current_changes_before_switching_policy_targets",
        translated_text: "Guarde o cancele los cambios actuales antes de cambiar los destinos de política.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.load_or_create_the_policy_used_for_wallpaper_rendering",
        translated_text: "Cargar o crear la política usada para renderizar el fondo de pantalla.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.this_editing_session_was_opened_for_the_active_screensaver_only",
        translated_text: "Esta sesión de edición se abrió para el protector de pantalla activo. Solo se puede editar la política de Protector de pantalla.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.this_shader_is_unavailable_for_wallpaper_use_because_it_does",
        translated_text: "Este shader no está disponible para usarse como Fondo de pantalla porque no existe en la carpeta de fondos de pantalla.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.keep_this_policy_and_all_of_its_settings_but_exclude",
        translated_text: "Conservar esta política y toda su configuración, pero excluirla del renderizado de protector de pantalla y fondo de pantalla hasta que se reasigne.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.no_shader_loaded",
        translated_text: "No hay ningún shader cargado",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.policy_name",
        translated_text: "Nombre de política:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.filename",
        translated_text: "Nombre de archivo:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.folder",
        translated_text: "Carpeta:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.type",
        translated_text: "Tipo:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.save_policy",
        translated_text: "Guardar política",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.save_the_current_per_shader_policy",
        translated_text: "Guardar la política actual específica del shader.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.saving_policy",
        translated_text: "Guardando política...",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.discard_changes_made_during_this_editor_session",
        translated_text: "Descartar los cambios realizados durante esta sesión de edición.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.bulk_edit_mode_canceled",
        translated_text: "Modo de edición masiva cancelado",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.changes_canceled",
        translated_text: "Cambios cancelados",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.policy",
        translated_text: "Política: --",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.policy_modified",
        translated_text: "Política: Modificada",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.policy_unchanged",
        translated_text: "Política: Sin cambios",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.config_modified",
        translated_text: "Configuración: Modificada",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.config_unchanged",
        translated_text: "Configuración: Sin cambios",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.loaded_and_rendering",
        translated_text: "cargado y renderizando",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.clear_all_policy_selections",
        translated_text: "Borrar todas las selecciones de políticas",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.policy_name_2",
        translated_text: "Nombre de política",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.policy_type",
        translated_text: "Tipo de política",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.no_shader_policies_are_currently_defined",
        translated_text: "Actualmente no hay políticas de shader definidas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.include_this_policy_in_bulk_edit_mode",
        translated_text: "Incluir esta política en el modo de edición masiva",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.policy_name_shader_path_shader_added_policy_created_policy_modified",
        translated_text: "Nombre de política: {value1}\\\nShader: {value2}\\\nRuta: {value3}\\\nShader agregado: {value4}\\\nPolítica creada: {value5}\\\nPolítica modificada: {value6}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.edit_policy",
        translated_text: "Editar política...",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.clone_policy",
        translated_text: "Clonar política...",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.rename_policy",
        translated_text: "Cambiar nombre de política...",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.add_to_playlist",
        translated_text: "Agregar a lista de reproducción...",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.refresh_shader",
        translated_text: "Actualizar shader",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.policy_query_cleared_displaying_all_policies",
        translated_text: "Consulta de políticas borrada — se muestran las {value1} políticas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.policy_query_returned_policies",
        translated_text: "La consulta de políticas devolvió {value1} / {value2} políticas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.policy_query_failed",
        translated_text: "La consulta de políticas falló: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.usable_shader_selected",
        translated_text: "{value1} shader{value2} utilizable seleccionado.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.texture_enabled_shader_detected",
        translated_text: "Se detectaron {value1} shader{value2} con texturas habilitadas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.managed_targets_screensaver_wallpaper",
        translated_text: "Destinos administrados: {value1} Protector de pantalla, {value2} Fondo de pantalla.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.external_shaders_requiring_a_target",
        translated_text: "Shaders externos que requieren un destino: {value1}.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.selected_shader_could_not_be_analyzed_and_will_not_be",
        translated_text: "{value1} shader{value2} seleccionado no se pudo analizar y no se incluirá.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.policy_target_for_all_external_shaders",
        translated_text: "Destino de política para todos los shaders externos:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.confirm_bulk_policy_changes",
        translated_text: "Confirmar cambios masivos de políticas",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.changes_will_be_applied_to_policies_click_ok_to_continue",
        translated_text: "Los cambios se aplicarán a {value1} políticas. Haga clic en Aceptar para continuar o en Cancelar para abortar.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.policies_were_selected_will_be_updated_and_will_be_skipped",
        translated_text: "Se seleccionaron {value1} políticas. Se actualizarán {value2} y se omitirán {value3} porque el archivo del shader no está disponible.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.excluded_policy",
        translated_text: "Política excluida:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.excluded_policies",
        translated_text: "Políticas excluidas:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.texture_and_palette_settings_will_apply_to_texture_enabled_shader",
        translated_text: "La configuración de Textura y Paleta se aplicará a {value1} shader{value2} con texturas habilitadas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.no_selected_policies_are_eligible_for_bulk_edit_because_their",
        translated_text: "Ninguna de las políticas seleccionadas es apta para Edición masiva porque sus archivos de shader no están disponibles.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.unsaved_changes",
        translated_text: "Cambios sin guardar",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.the_screenshaver_control_center_has_unsaved_changes",
        translated_text: "El Centro de control de Screenshaver tiene cambios sin guardar.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.would_you_like_to_save_those_changes_before_exiting",
        translated_text: "¿Desea guardar esos cambios antes de salir?",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.save_and_exit",
        translated_text: "Guardar y salir",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.exit_without_saving",
        translated_text: "Salir sin guardar",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.rename_policy_2",
        translated_text: "Cambiar nombre de política",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.change_the_user_facing_policy_name_the_shader_and_policy",
        translated_text: "Cambiar el Nombre de política visible para el usuario. El shader y la configuración de la política no cambian.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.policy_name_must_contain_between_1_and_128_characters_found",
        translated_text: "El Nombre de política debe contener entre 1 y 128 caracteres; se encontraron {value1}.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.clone_policy_2",
        translated_text: "Clonar política",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.move_shader",
        translated_text: "¿Mover shader?",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.move_this_shader_to",
        translated_text: "Mover este shader a {value1}:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.the_existing_policy_will_be_changed_to_a_policy_all",
        translated_text: "La política {value1} existente se cambiará a una política {value2}. Se conservará toda la configuración de la política.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.the_existing_policy_will_be_retained_and_its_path_will",
        translated_text: "La política {value1} existente se conservará y su ruta se actualizará automáticamente.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.the_shader_file_will_not_be_deleted",
        translated_text: "El archivo del shader no se eliminará.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.permanently_delete_this_shader",
        translated_text: "Eliminar permanentemente este shader {value1}:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.the_associated_policy_will_also_be_deleted",
        translated_text: "También se eliminará la política {value1} asociada.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.any_wallpaper_shader_or_wallpaper_policy_with_the_same_filename",
        translated_text: "No se cambiará ningún shader de Fondo de pantalla ni ninguna política de Fondo de pantalla con el mismo nombre de archivo.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.any_screensaver_shader_or_screensaver_policy_with_the_same_filename",
        translated_text: "No se cambiará ningún shader de Protector de pantalla ni ninguna política de Protector de pantalla con el mismo nombre de archivo.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.the_shader_file_will_not_be_changed",
        translated_text: "El archivo del shader no se modificará.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.unable_to_load_playlists",
        translated_text: "No se pudieron cargar las listas de reproducción: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.unable_to_load_the_playlist_inventory",
        translated_text: "No se pudo cargar el inventario de Listas de reproducción.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.unable_to_load_playlist_policies",
        translated_text: "No se pudieron cargar las políticas de la lista de reproducción: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.playlist_name",
        translated_text: "Nombre de lista de reproducción",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.no_playlists_have_been_created",
        translated_text: "No se han creado listas de reproducción.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.playlist_created_playlist_modified",
        translated_text: "Lista de reproducción creada: {value1}\\\nLista de reproducción modificada: {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.selected_playlist",
        translated_text: "Se seleccionó la lista de reproducción '{value1}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.new_playlist",
        translated_text: "Nueva lista de reproducción",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.edit_playlist_info",
        translated_text: "Editar información de la lista de reproducción",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.selected_playlist_2",
        translated_text: "Lista de reproducción seleccionada",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.this_playlist_contains_no_policies",
        translated_text: "Esta lista de reproducción no contiene políticas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.policy_target_2",
        translated_text: "Destino de política",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.sorted_playlist_by_compound_criteria",
        translated_text: "Se ordenó la lista de reproducción '{value1}' por criterios compuestos.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.sorted_playlist_by",
        translated_text: "Se ordenó la lista de reproducción '{value1}' por {value2}.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.unable_to_sort_playlist",
        translated_text: "No se pudo ordenar la lista de reproducción: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.click_to_sort_and_persist_playlist_order_shift_click_adds",
        translated_text: "Haga clic para ordenar y conservar el orden de la lista de reproducción. Mayús-clic agrega o cambia una clave de orden secundaria.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.policy_name_shader_path",
        translated_text: "Nombre de política: {value1}\\\nShader: {value2}\\\nRuta: {value3}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.selected_policy_in_playlist",
        translated_text: "Se seleccionó la política '{value1}' en la lista de reproducción '{value2}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.remove_policy",
        translated_text: "Eliminar política",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.removed_policy_from_playlist",
        translated_text: "Se eliminó la política '{value1}' de la lista de reproducción.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.policy_is_no_longer_a_member_of_this_playlist",
        translated_text: "La política ya no pertenece a esta lista de reproducción.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.unable_to_remove_policy_from_playlist",
        translated_text: "No se pudo eliminar la política de la lista de reproducción: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.add_policy",
        translated_text: "Agregar política",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.removed_policy_from_playlist_2",
        translated_text: "Se eliminó la política '{value1}' de la lista de reproducción '{value2}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.policy_was_not_present_in_playlist",
        translated_text: "La política '{value1}' no estaba presente en la lista de reproducción '{value2}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.move_up",
        translated_text: "Mover arriba",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.moved_policy_up",
        translated_text: "Se movió la política '{value1}' hacia arriba.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.unable_to_move_policy",
        translated_text: "No se pudo mover la política: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.move_down",
        translated_text: "Mover abajo",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.moved_policy_down",
        translated_text: "Se movió la política '{value1}' hacia abajo.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.add_to_playlist_2",
        translated_text: "Agregar a lista de reproducción",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.add_policy_to_an_existing_playlist",
        translated_text: "Agregar la política '{value1}' a una lista de reproducción existente.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.this_policy_is_already_a_member_of_every_playlist",
        translated_text: "Esta política ya pertenece a todas las listas de reproducción.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.selected_playlist_3",
        translated_text: "lista de reproducción seleccionada",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.added_policy_to_playlist",
        translated_text: "Se agregó la política '{value1}' a la lista de reproducción '{value2}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.policy_is_already_a_member_of_playlist",
        translated_text: "La política '{value1}' ya pertenece a la lista de reproducción '{value2}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.unable_to_add_policy_to_playlist",
        translated_text: "No se pudo agregar la política a la lista de reproducción: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.no_policies_available",
        translated_text: "No hay políticas disponibles",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.add_policy_to_playlist",
        translated_text: "Agregar política a lista de reproducción",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.all_available_policies_are_already_members_of_this_playlist",
        translated_text: "Todas las políticas disponibles ya pertenecen a esta lista de reproducción.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.selected_policy",
        translated_text: "Política seleccionada",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.added_policy_to_playlist_2",
        translated_text: "Se agregó la política '{value1}' a la lista de reproducción.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.that_policy_is_already_a_member_of_the_playlist",
        translated_text: "Esa política ya pertenece a la lista de reproducción.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.playlist_name_2",
        translated_text: "Nombre de lista de reproducción:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.description_optional",
        translated_text: "Descripción (opcional):",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.created_playlist",
        translated_text: "Se creó la lista de reproducción '{value1}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.edit_the_playlist_name_and_description_playlist_membership_and_policy",
        translated_text: "Editar el nombre y la descripción de la lista de reproducción. La pertenencia y el orden de las políticas no cambian.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.updated_playlist",
        translated_text: "Se actualizó la lista de reproducción '{value1}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.shader_policies_and_shader_files_will_not_be_deleted",
        translated_text: "Las políticas de shader y los archivos de shader no se eliminarán.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.deleted_playlist",
        translated_text: "Se eliminó la lista de reproducción '{value1}'.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.unable_to_delete_playlist",
        translated_text: "No se pudo eliminar la lista de reproducción '{value1}': {value2}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.render_controls",
        translated_text: "Controles de renderizado",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.fps_max",
        translated_text: "FPS (Máx.)",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.fps",
        translated_text: "{value1} FPS",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.click_to_include_fps_in_bulk_edit",
        translated_text: "Haga clic para incluir FPS en Edición masiva",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.click_to_exclude_fps_from_bulk_edit",
        translated_text: "Haga clic para excluir FPS de Edición masiva",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.click_the_numeric_fps_value_to_enable_this_slider_for",
        translated_text: "Haga clic en el valor numérico de FPS para habilitar este control deslizante para Edición masiva.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.set_the_maximum_rendering_frame_rate_hold_shift_for_fine",
        translated_text: "Establezca la velocidad máxima de fotogramas de renderizado. Mantenga Mayús para un ajuste preciso.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.animation_speed",
        translated_text: "Velocidad de animación",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.click_to_include_animation_speed_in_bulk_edit",
        translated_text: "Haga clic para incluir Velocidad de animación en Edición masiva",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.click_to_exclude_animation_speed_from_bulk_edit",
        translated_text: "Haga clic para excluir Velocidad de animación de Edición masiva",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.click_the_numeric_animation_speed_value_to_enable_this_slider",
        translated_text: "Haga clic en el valor numérico de Velocidad de animación para habilitar este control deslizante para Edición masiva.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.adjust_animation_speed_on_a_logarithmic_scale_the_slider_midpoint",
        translated_text: "Ajuste la velocidad de animación en una escala logarítmica. El punto medio del control es 1.0x. Mantenga Mayús para un ajuste preciso.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.starting_offset",
        translated_text: "Desplazamiento inicial",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.starting_offset_is_not_available_during_bulk_edit",
        translated_text: "El Desplazamiento inicial no está disponible durante la Edición masiva.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.drag_to_choose_where_the_shader_begins_hold_shift_while",
        translated_text: "Arrastre para elegir dónde comienza el shader. Mantenga Mayús mientras arrastra para un ajuste 10 veces más preciso.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.render_scale",
        translated_text: "Escala de renderizado",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.click_to_include_render_scale_in_bulk_edit",
        translated_text: "Haga clic para incluir Escala de renderizado en Edición masiva",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.click_to_exclude_render_scale_from_bulk_edit",
        translated_text: "Haga clic para excluir Escala de renderizado de Edición masiva",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.click_the_numeric_render_scale_value_to_enable_this_slider",
        translated_text: "Haga clic en el valor numérico de Escala de renderizado para habilitar este control deslizante para Edición masiva.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.change_internal_rendering_resolution_lower_values_improve_performance_higher_values",
        translated_text: "Cambie la resolución interna de renderizado. Los valores menores mejoran el rendimiento; los mayores mejoran la calidad.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.set_the_number_of_graphical_elements_used_to_generate_the",
        translated_text: "Establezca el número de elementos gráficos usados para generar la textura procedural.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.unable_to_generate_texture_thumbnail",
        translated_text: "No se pudo generar la miniatura de textura: {value1}",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.texture_preview_unavailable",
        translated_text: "Vista previa de textura no disponible",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.palette_color",
        translated_text: "Color de paleta:",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.custom_color",
        translated_text: "Color personalizado",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.choose_a_curated_palette_color_the_selected_color_is_written",
        translated_text: "Elija un color de paleta seleccionado. El color seleccionado se escribe en el campo hexadecimal.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.enter_a_palette_color_using_six_digit_hexadecimal_notation_rrggbb",
        translated_text: "Introduzca un color de paleta usando notación hexadecimal de seis dígitos (#rrggbb).",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.policy_actions",
        translated_text: "Acciones de política",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.save_the_current_per_shader_policy_after_all_mandatory_information",
        translated_text: "Guardar la política actual específica del shader después de proporcionar toda la información obligatoria.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.permanently_remove_the_shader_file_after_confirmation",
        translated_text: "Eliminar permanentemente el archivo del shader después de la confirmación.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.about_shader_policies",
        translated_text: "Acerca de las políticas de shader",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.a_shader_policy_determines_how_the_selected_shader_is_rendered",
        translated_text: "Una política de shader determina cómo se renderiza el shader seleccionado como protector de pantalla o fondo de pantalla.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.policy_unsaved_changes",
        translated_text: "Política: Cambios sin guardar",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.bulk_create.usable_shader_selected_one",
        translated_text: "Se seleccionó {value1} shader utilizable.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.bulk_create.usable_shaders_selected_many",
        translated_text: "Se seleccionaron {value1} shaders utilizables.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.bulk_create.texture_shader_detected_one",
        translated_text: "Se detectó {value1} shader con texturas habilitadas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.bulk_create.texture_shaders_detected_many",
        translated_text: "Se detectaron {value1} shaders con texturas habilitadas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.bulk_create.rejected_shader_one",
        translated_text: "No se pudo analizar {value1} shader seleccionado y no se incluirá.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.bulk_create.rejected_shaders_many",
        translated_text: "No se pudieron analizar {value1} shaders seleccionados y no se incluirán.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.bulk_edit.texture_settings_one",
        translated_text: "La configuración de Textura y Paleta se aplicará a {value1} shader con texturas habilitadas.",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.bulk_edit.texture_settings_many",
        translated_text: "La configuración de Textura y Paleta se aplicará a {value1} shaders con texturas habilitadas.",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "qbe.field.policy_name",
        translated_text: "Nombre de política",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.field.playlist_name",
        translated_text: "Nombre de lista de reproducción",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.field.shader_added_date",
        translated_text: "Fecha de adición del shader",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.field.policy_created_date",
        translated_text: "Fecha de creación de política",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.field.policy_modified_date",
        translated_text: "Fecha de modificación de política",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.field.playlist_created_date",
        translated_text: "Fecha de creación de lista",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.field.playlist_modified_date",
        translated_text: "Fecha de modificación de lista",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.field.shader_filename",
        translated_text: "Nombre de archivo del shader",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.field.shader_type",
        translated_text: "Tipo de shader",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.field.policy_target",
        translated_text: "Destino de política",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.field.status",
        translated_text: "Estado",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.field.texture",
        translated_text: "Textura",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.field.palette",
        translated_text: "Paleta",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.field.rendered_fps",
        translated_text: "FPS renderizados",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.field.animation_speed",
        translated_text: "Velocidad de animación",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.field.render_scale",
        translated_text: "Escala de renderizado",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.field.anti_aliasing",
        translated_text: "Antialiasing",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.field.dithering",
        translated_text: "Tramado",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.field.color_precision",
        translated_text: "Precisión de color",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.field.audiovisual_effect",
        translated_text: "Efecto audiovisual",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.field.invert_frequency_mapping",
        translated_text: "Invertir asignación de frecuencia",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.operator.is",
        translated_text: "es",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.operator.eq",
        translated_text: "igual",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.operator.ne",
        translated_text: "distinto",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.operator.like",
        translated_text: "como",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.operator.not_like",
        translated_text: "no como",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.operator.lt",
        translated_text: "menor que",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.operator.le",
        translated_text: "menor o igual",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.operator.gt",
        translated_text: "mayor que",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.operator.ge",
        translated_text: "mayor o igual",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.conditional.and",
        translated_text: "Y",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "qbe.conditional.or",
        translated_text: "O",
    },

    FactoryTranslation {
        locale: "es-US",
        key: "editor.tab.policies",
        translated_text: "Políticas",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.tab.playlists",
        translated_text: "Listas de reproducción",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.tab.rendering",
        translated_text: "Renderizado",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.tab.textures",
        translated_text: "Texturas",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.tab.post_processing",
        translated_text: "Posprocesamiento",
    },
    FactoryTranslation {
        locale: "es-US",
        key: "editor.tab.configuration",
        translated_text: "Configuración",
    },

];
