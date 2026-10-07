// Screenshaver French (France) factory localization catalog.
//
// Generated from the canonical English localization keys. User/external values,
// stored tokens, paths, filenames, diagnostics, and named technical terms remain
// unchanged where required by translator context.

use super::FactoryTranslation;

pub(crate) const TRANSLATIONS: &[FactoryTranslation] = &[
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.screensaver",
        translated_text: "Économiseur d’écran",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.wallpaper",
        translated_text: "Fond d’écran",
    },

    FactoryTranslation {
        locale: "fr-FR",
        key: "warning.desktop_icon_compatibility_title",
        translated_text: "Avertissement de compatibilité des icônes du bureau",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "warning.screen_locking_unavailable_title",
        translated_text: "Verrouillage de l’écran indisponible",
    },

    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.directory_create_failed",
        translated_text: "Impossible de créer le répertoire de sauvegarde Screenshaver « {path} » : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.configuration_unavailable",
        translated_text: "Configuration de sauvegarde indisponible : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.heading",
        translated_text: "Sauvegardes complètes",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.schedule_prefix",
        translated_text: "Effectuer une sauvegarde complète de Screenshaver tous les",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.days",
        translated_text: "jours",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.configuration_saved",
        translated_text: "Configuration de sauvegarde enregistrée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.last_reference",
        translated_text: "Dernière référence de sauvegarde : {reference}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.folder",
        translated_text: "Dossier de sauvegarde : {path}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.now",
        translated_text: "Sauvegarder maintenant",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.creating",
        translated_text: "Création d’une sauvegarde complète de Screenshaver…",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.created",
        translated_text: "Sauvegarde créée : {path}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.failed",
        translated_text: "Échec de la sauvegarde : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore",
        translated_text: "Restaurer depuis une sauvegarde",
    },

// Restore from Backup.
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.archive_open_failed",
        translated_text: "Impossible d’ouvrir l’archive de sauvegarde « {path} » : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.cancel",
        translated_text: "Annuler la restauration",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.cancelled",
        translated_text: "Restauration annulée. Aucun fichier Screenshaver actif n’a été modifié.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.configuration_directory_prepare_failed",
        translated_text: "Impossible de préparer le répertoire de configuration Screenshaver « {path} » : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.confirm",
        translated_text: "Confirmer la restauration",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.confirmation_explanation",
        translated_text: "La sauvegarde sélectionnée a réussi la vérification. La confirmation remplacera la base de données Screenshaver actuelle et les shaders gérés. Une copie de retour arrière vérifiée de l’installation actuelle sera créée avant la permutation.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.copy_failed",
        translated_text: "Impossible de copier « {source} » vers « {destination} » : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.database_cutover_begin_failed",
        translated_text: "Impossible de commencer la permutation de la base de données pour la restauration : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.database_cutover_prepare_failed",
        translated_text: "Impossible de préparer la base de données restaurée pour la permutation : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.database_install_failed_retained",
        translated_text: "Impossible d’installer la base de données restaurée ; la base de données d’origine a été conservée : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.database_open_failed",
        translated_text: "Impossible d’ouvrir la base de données de restauration « {path} » : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.database_rollback_failed",
        translated_text: "Impossible de restaurer la base de données antérieure à la restauration : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.directory_create_failed",
        translated_text: "Impossible de créer le répertoire de restauration « {path} » : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.directory_entry_read_failed",
        translated_text: "Impossible de lire une entrée du répertoire « {path} » : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.directory_enumerate_failed",
        translated_text: "Impossible d’énumérer « {path} » : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.duplicate_member",
        translated_text: "L’archive de sauvegarde contient l’élément dupliqué « {member} » ; la restauration a été refusée",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.entry_inspect_failed",
        translated_text: "Impossible d’inspecter « {path} » : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.export_not_backup",
        translated_text: "L’archive Screenshaver sélectionnée est une exportation et non une sauvegarde complète",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.failed",
        translated_text: "Échec de la restauration de la sauvegarde : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.failed_database_preserve_failed",
        translated_text: "Impossible de conserver la base de données restaurée en échec : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.failed_shader_directory_preserve_failed",
        translated_text: "Impossible de conserver le répertoire de shaders restauré en échec : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.final_validation_and_rollback_failed",
        translated_text: "La base de données restaurée a échoué à la validation finale : {error}. Le retour arrière automatique a également échoué : {rollback_error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.final_validation_failed_rolled_back",
        translated_text: "La base de données restaurée a échoué à la validation finale ; l’état antérieur à la restauration a été rétabli : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.installing",
        translated_text: "Installation de la sauvegarde Screenshaver vérifiée…",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.invalid_managed_shader_member",
        translated_text: "La sauvegarde contient un élément de shader géré non valide « {member} »",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.live_database_missing",
        translated_text: "La base de données active « {path} » n’existe pas ; la restauration a été refusée",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.managed_shader_stage_failed",
        translated_text: "Impossible de préparer le shader géré « {filename} » : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.manifest_member_missing",
        translated_text: "Le manifeste de sauvegarde référence l’élément absent « {member} »",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.manifest_parse_failed",
        translated_text: "Impossible d’analyser le manifeste de sauvegarde : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.member_sha256_failed",
        translated_text: "L’élément « {member} » de l’archive de sauvegarde a échoué à la vérification SHA-256",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.package_sha256_failed",
        translated_text: "L’archive de sauvegarde a échoué à la vérification SHA-256 du paquet",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.path_remove_failed",
        translated_text: "Impossible de supprimer « {path} » : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.ready_to_install",
        translated_text: "La restauration est prête à être installée",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.rollback_directory_create_failed",
        translated_text: "Impossible de créer le répertoire de retour arrière de la restauration : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.rollback_snapshot_create_failed",
        translated_text: "Impossible de créer l’instantané de base de données antérieur à la restauration « {path} » : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.schema_mismatch",
        translated_text: "Le manifeste de sauvegarde indique le schéma de base de données {manifest_schema}, mais la base de données préparée indique le schéma {database_schema}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.selected_backup",
        translated_text: "Sauvegarde : {path}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.shader_cutover_begin_failed",
        translated_text: "Impossible de commencer la restauration des shaders gérés ; la base de données d’origine a été restaurée : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.shader_install_and_rollback_failed",
        translated_text: "Impossible d’installer les shaders gérés restaurés : {error}. Le retour arrière automatique a également échoué : {rollback_error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.shader_install_failed_rolled_back",
        translated_text: "Impossible d’installer les shaders gérés restaurés ; l’état antérieur à la restauration a été rétabli : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.snapshot_missing_from_manifest",
        translated_text: "Le manifeste de sauvegarde n’identifie aucun instantané de base de données",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.snapshot_payload_missing",
        translated_text: "L’archive de sauvegarde ne contient pas l’instantané de base de données déclaré",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.staged_database_write_failed",
        translated_text: "Impossible d’écrire la base de données de restauration préparée « {path} » : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.staged_shader_directory_create_failed",
        translated_text: "Impossible de créer le répertoire préparé des shaders gérés « {path} » : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.staged_successfully",
        translated_text: "Sauvegarde vérifiée et préparée avec succès : {archive} | créée {created} | Screenshaver {version} | schéma de base de données {source_schema} -> {staged_schema} | shaders gérés {shader_count} | préparation {staging}. Aucun fichier Screenshaver actif n’a été modifié.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.staging_create_failed",
        translated_text: "Impossible de créer le répertoire de préparation de restauration « {path} » : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.staging_failed",
        translated_text: "Échec de la préparation de la restauration de la sauvegarde : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.stale_staging_remove_failed",
        translated_text: "Impossible de supprimer l’ancien répertoire de préparation de restauration « {path} » : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.success",
        translated_text: "Sauvegarde restaurée avec succès. La base de données et les shaders gérés restaurés ont réussi la validation finale. Fermez le Centre de contrôle afin que Screenshaver puisse recharger la configuration restaurée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.unsafe_member",
        translated_text: "L’archive de sauvegarde contient un chemin d’élément non sécurisé « {member} » ; la restauration a été refusée",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.unsupported_filesystem_entry",
        translated_text: "La restauration a refusé de copier l’entrée de système de fichiers non prise en charge « {path} »",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.unsupported_format",
        translated_text: "Format de sauvegarde Screenshaver non pris en charge « {format} » version {version}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.unsupported_snapshot_path",
        translated_text: "Le manifeste de sauvegarde contient un chemin d’instantané de base de données non pris en charge « {path} »",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.zip_member_inspect_failed",
        translated_text: "Impossible d’inspecter l’élément {index} de l’archive ZIP de sauvegarde : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.zip_member_read_failed",
        translated_text: "Impossible de lire l’élément « {member} » de l’archive ZIP de sauvegarde : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "backup.restore.zip_read_failed",
        translated_text: "Impossible de lire l’archive ZIP de sauvegarde : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "database.restore.historical_staged_replace_failed",
        translated_text: "Impossible de remplacer la base de données historique de restauration préparée « {path} » : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "database.restore.reconstructed_promote_failed",
        translated_text: "Impossible de promouvoir la base de données de restauration préparée reconstruite de « {source} » vers « {destination} » : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "database.restore.reconstruction_failed",
        translated_text: "Échec de la reconstruction de la base de données de restauration préparée du schéma {source_schema} vers le schéma {destination_schema} : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "database.restore.reconstruction_validation_failed",
        translated_text: "La base de données de restauration préparée reconstruite depuis le schéma {source_schema} a échoué à la validation : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "database.restore.staged_database_missing",
        translated_text: "Impossible de préparer la base de données de restauration car « {path} » n’existe pas",
    },

    FactoryTranslation {
        locale: "fr-FR",
        key: "tab.appearance",
        translated_text: "Apparence",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "tab.rendering",
        translated_text: "Rendu",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "tab.lyrics",
        translated_text: "Paroles",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "tab.data_io",
        translated_text: "E/S de données",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "config.unavailable",
        translated_text: "La configuration n’est pas disponible.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "config.save",
        translated_text: "Enregistrer la configuration",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "config.saving",
        translated_text: "Enregistrement de la configuration…",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "common.cancel",
        translated_text: "Annuler",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "config.discarded",
        translated_text: "Modifications de configuration abandonnées.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "data_io.heading",
        translated_text: "E/S de données",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "data_io.description",
        translated_text: "Créer des sauvegardes de récupération ou importer/exporter des données Screenshaver portables.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "data_io.portable_data",
        translated_text: "Données portables",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "data_io.import",
        translated_text: "Importer…",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "data_io.export",
        translated_text: "Exporter…",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "appearance.heading",
        translated_text: "Valeurs par défaut de l’apparence",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "appearance.show_splash",
        translated_text: "Afficher l’écran de démarrage",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "appearance.screensaver_subtitles",
        translated_text: "Sous-titres de l’économiseur d’écran",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "appearance.subtitle_placement",
        translated_text: "Position des sous-titres :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "appearance.wallpaper_notifications",
        translated_text: "Notifications du fond d’écran",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "placement.top_left",
        translated_text: "En haut à gauche",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "placement.top_center",
        translated_text: "En haut au centre",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "placement.top_right",
        translated_text: "En haut à droite",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "placement.bottom_left",
        translated_text: "En bas à gauche",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "placement.bottom_center",
        translated_text: "En bas au centre",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "placement.bottom_right",
        translated_text: "En bas à droite",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.screensaver_settings",
        translated_text: "Paramètres et valeurs par défaut de l’économiseur d’écran",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.wallpaper_settings",
        translated_text: "Paramètres et valeurs par défaut du fond d’écran",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "common.enabled",
        translated_text: "Activé",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.display_format",
        translated_text: "Format d’affichage :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.display_format_help",
        translated_text: "Détermine si le fond d’écran est affiché en plein écran ou dans une fenêtre normale gérée par le bureau.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.full_screen",
        translated_text: "Plein écran",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.windowshader",
        translated_text: "Windowshader",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.mode",
        translated_text: "Mode :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "mode.ordered",
        translated_text: "Ordonné",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "mode.random",
        translated_text: "Aléatoire",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "mode.single",
        translated_text: "Unique",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "mode.playlist",
        translated_text: "Liste de lecture",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.policy",
        translated_text: "Politique :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.select_policy",
        translated_text: "<sélectionner une politique>",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.single_policy_selected",
        translated_text: "Politique {target} unique sélectionnée : {name}.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.no_eligible_policies",
        translated_text: "Aucune politique admissible",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.playlist",
        translated_text: "Liste de lecture :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.select_playlist",
        translated_text: "<sélectionner une liste de lecture>",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.no_playlists",
        translated_text: "Aucune liste de lecture disponible",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.playlist_selected",
        translated_text: "Liste de lecture {target} sélectionnée : {name}.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.playlists_unavailable",
        translated_text: "Impossible de charger les listes de lecture",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.interval",
        translated_text: "Intervalle :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "unit.seconds_lower",
        translated_text: "secondes",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.idle_timeout",
        translated_text: "Délai d’inactivité :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "unit.seconds",
        translated_text: "Secondes",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "unit.minutes",
        translated_text: "Minutes",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "unit.hours",
        translated_text: "Heures",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.animation_speed",
        translated_text: "Vitesse d’animation :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.texture",
        translated_text: "Texture :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "common.random",
        translated_text: "Aléatoire",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.texture_catalog_unavailable",
        translated_text: "Catalogue de textures indisponible",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.texture_choices_failed",
        translated_text: "Impossible de charger les choix de textures : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.palette",
        translated_text: "Palette :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.texture_primitives",
        translated_text: "Primitives de texture :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.single_policy_required",
        translated_text: "Sélectionnez une politique de shader pour le mode d’affichage {target} unique.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "lyrics.heading",
        translated_text: "Paroles",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "lyrics.display",
        translated_text: "Afficher les paroles synchronisées (windowshader uniquement)",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "lyrics.display_help",
        translated_text: "Affiche les paroles synchronisées du morceau en cours de lecture par-dessus le windowshader. Les paroles sont obtenues automatiquement lorsqu’elles sont disponibles.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "rendering.heading",
        translated_text: "Valeurs par défaut du rendu",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "rendering.fps",
        translated_text: "FPS rendues :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "rendering.anti_aliasing",
        translated_text: "Anticrénelage :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "rendering.dithering",
        translated_text: "Tramage :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "rendering.color_precision",
        translated_text: "Précision des couleurs :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "rendering.render_scale",
        translated_text: "Échelle de rendu :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "rendering.off",
        translated_text: "Désactivé",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "rendering.fxaa",
        translated_text: "FXAA",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "rendering.subtle",
        translated_text: "Subtil",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "rendering.auto",
        translated_text: "Automatique",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "rendering.standard",
        translated_text: "Standard",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "rendering.high",
        translated_text: "Élevée",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.palette_unavailable",
        translated_text: "Palette organisée indisponible",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.palette_choices_failed",
        translated_text: "Impossible de charger les choix de palettes organisées : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.palette_selected",
        translated_text: "Palette par défaut {target} sélectionnée : {name}.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.tab.visual_quality",
        translated_text: "Qualité visuelle",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.tab.image_transforms",
        translated_text: "Transformations de l’image",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.tab.audiovisual",
        translated_text: "Audiovisuel",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.tab.audio_motion",
        translated_text: "Mouvement audio",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.common.unchanged",
        translated_text: "Inchangé",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.common.enabled",
        translated_text: "Activé",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.common.disabled",
        translated_text: "Désactivé",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.common.off",
        translated_text: "Désactivé",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.visual.anti_aliasing",
        translated_text: "Anticrénelage :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.visual.anti_aliasing_help",
        translated_text: "Contrôle le lissage des contours du shader rendu.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.visual.dithering",
        translated_text: "Tramage :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.visual.dithering_help",
        translated_text: "Contrôle le tramage subtil utilisé pour réduire les bandes de couleur visibles.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.visual.subtle",
        translated_text: "Subtil",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.visual.color_precision",
        translated_text: "Précision des couleurs :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.visual.color_precision_help",
        translated_text: "Sélectionne la précision des couleurs utilisée par le post-traitement.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.visual.automatic",
        translated_text: "Automatique",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.visual.standard_precision",
        translated_text: "Précision standard",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.visual.high_precision",
        translated_text: "Haute précision",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.transform.invert_colors",
        translated_text: "Inverser les couleurs",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.transform.invert_colors_help",
        translated_text: "Inverse les couleurs finales rendues.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.transform.flip_horizontal",
        translated_text: "Retourner horizontalement",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.transform.flip_horizontal_help",
        translated_text: "Inverse horizontalement l’image finale.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.transform.flip_vertical",
        translated_text: "Retourner verticalement",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.transform.flip_vertical_help",
        translated_text: "Inverse verticalement l’image finale.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.transform.hue_rotation",
        translated_text: "Rotation de teinte :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.transform.hue_rotation_help",
        translated_text: "Fait pivoter les couleurs affichées du shader autour du cercle chromatique.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.audio.effect",
        translated_text: "Effet audiovisuel :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.audio.effect_help",
        translated_text: "Sélectionne l’effet de post-traitement piloté par l’audio : Désactivé, Éclat audio, Éclat spectral ou Éclat de sonie.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.audio.audio_bloom",
        translated_text: "Éclat audio",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.audio.spectral_bloom",
        translated_text: "Éclat spectral",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.audio.loudness_bloom",
        translated_text: "Éclat de sonie",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.audio.bloom_intensity",
        translated_text: "Intensité du bloom :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.audio.bloom_intensity_help",
        translated_text: "Contrôle l’intensité du mode d’éclat sélectionné.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.audio.bloom_saturation",
        translated_text: "Saturation du bloom :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.audio.bloom_saturation_help",
        translated_text: "Augmente la saturation des couleurs du bloom depuis le niveau neutre 1.0 jusqu’à 2.0 sans modifier les couleurs affichées du shader.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.audio.bloom_threshold",
        translated_text: "Seuil du bloom :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.audio.bloom_threshold_help",
        translated_text: "Contrôle le seuil de luminosité utilisé pour extraire l’éclat.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.audio.frequency_rotation",
        translated_text: "Rotation des fréquences :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.audio.frequency_rotation_help",
        translated_text: "Fait pivoter le mappage fréquence-couleur audio/spectral. Désactivé pour l’Éclat de sonie.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.audio.invert_frequency",
        translated_text: "Inverser le mappage des fréquences",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.audio.invert_frequency_help",
        translated_text: "Inverse le mappage couleur-fréquence des basses vers les hautes fréquences pour le mode audio/spectral. Désactivé pour l’Éclat de sonie.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.motion.effect",
        translated_text: "Effet de mouvement audio :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.motion.effect_help",
        translated_text: "Sélectionne un effet de mouvement plein écran piloté par l’audio. Haut-parleur de graves infernal utilise la synchronisation vocale LRCMUX pour un mouvement en cône inversé. Déformation miroir FFT utilise une trace FFT symétrique à 48 canaux. Hélice polaire utilise la même réponse FFT à 48 canaux dans une déformation radiale rotative.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.motion.woofer",
        translated_text: "Haut-parleur de graves infernal",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.motion.fft_mirror",
        translated_text: "Déformation miroir FFT",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.motion.polar_propeller",
        translated_text: "Hélice polaire",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.bulk.exclude",
        translated_text: "Cliquez pour exclure cette valeur de la modification groupée",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "post.bulk.include",
        translated_text: "Cliquez pour inclure cette valeur dans la modification groupée",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.window_title",
        translated_text: "Exporter les données de Screenshaver",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.stage.select_focus",
        translated_text: "Sélectionner la cible de l’exportation",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.stage.select_data",
        translated_text: "Sélectionner les données",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.destination",
        translated_text: "Destination",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.review_confirm",
        translated_text: "Vérifier et confirmer",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.results",
        translated_text: "Résultats",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.policies",
        translated_text: "Politiques",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.shaders",
        translated_text: "Shaders",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.playlists",
        translated_text: "Listes de lecture",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.select_policies",
        translated_text: "Sélectionner les politiques",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.select_shaders",
        translated_text: "Sélectionner les shaders",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.select_playlists",
        translated_text: "Sélectionner les listes de lecture",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.focus",
        translated_text: "Cible de l’exportation :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.focus_heading",
        translated_text: "Cible de l’exportation",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.focus_help",
        translated_text: "Sélectionnez le type de données Screenshaver qui déterminera le contenu de cette exportation. Politiques vous permet de choisir des politiques et inclut automatiquement leurs shaders requis ainsi que les listes de lecture associées. Shaders vous permet de choisir des shaders et inclut automatiquement les politiques et listes de lecture associées. Listes de lecture vous permet de choisir des listes de lecture et inclut automatiquement leurs politiques membres et les shaders requis. Les éléments inclus automatiquement sont en lecture seule.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.destination_help",
        translated_text: "Choisissez le répertoire dans lequel l’archive d’exportation portable de Screenshaver sera créée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.select_all",
        translated_text: "Tout sélectionner",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.clear_all",
        translated_text: "Tout effacer",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.selected_count",
        translated_text: "{selected} sur {total} sélectionnés",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.policies_included",
        translated_text: "Politiques incluses ({count})",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.shaders_included",
        translated_text: "Shaders inclus ({count})",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.playlists_included",
        translated_text: "Listes de lecture incluses ({count})",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.resolve_failed",
        translated_text: "Impossible de résoudre les politiques effectives de l’exportation : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.policies_resolved",
        translated_text: "{count} politiques incluses ont été résolues et validées pour l’exportation portable.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.select_at_least_one",
        translated_text: "Sélectionnez au moins un élément de type {type} pour continuer.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.destination_folder",
        translated_text: "Dossier de destination :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.browse",
        translated_text: "Parcourir…",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.filename",
        translated_text: "Nom du fichier d’exportation :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.filename_invalid",
        translated_text: "Saisissez un nom de fichier sans séparateur de répertoire.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.filename_exists",
        translated_text: "Ce nom de fichier existe déjà. L’exportation utilisera : {filename}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.contents",
        translated_text: "Contenu de l’exportation",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.policies_colon",
        translated_text: "Politiques :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.shaders_colon",
        translated_text: "Shaders :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.playlists_colon",
        translated_text: "Listes de lecture :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.format_name",
        translated_text: "Format d’exportation Screenshaver 1",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.format",
        translated_text: "Format d’exportation",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.review_safety",
        translated_text: "Aucune configuration de Screenshaver ne sera modifiée. L’exportation crée une copie portable des éléments affichés ci-dessus.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.export_colon",
        translated_text: "Exportation :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.passed",
        translated_text: "RÉUSSI",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.failed",
        translated_text: "ÉCHEC",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.archive",
        translated_text: "Archive : {path}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.result_counts",
        translated_text: "Politiques : {policies}    Shaders : {shaders}    Listes de lecture : {playlists}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.no_archive_installed",
        translated_text: "Aucune archive d’exportation terminée n’a été installée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.not_run",
        translated_text: "L’exportation n’a pas été exécutée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.back",
        translated_text: "< Retour",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.action",
        translated_text: "Exporter",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.next",
        translated_text: "Suivant >",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.finish",
        translated_text: "Terminer",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.window_title",
        translated_text: "Importer les données de Screenshaver",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.stage.select_archive",
        translated_text: "Sélectionner une archive",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.stage.inspect_archive",
        translated_text: "Inspecter l’archive",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.stage.resolve_conflicts",
        translated_text: "Résoudre les conflits",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.review_confirm",
        translated_text: "Vérifier et confirmer",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.results",
        translated_text: "Résultats",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.select_archive_help",
        translated_text: "Sélectionnez une archive d’exportation portable Screenshaver à inspecter avant l’importation.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.archive_colon",
        translated_text: "Archive :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.archive_ready",
        translated_text: "L’archive est prête pour une inspection en lecture seule.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.no_archive_selected",
        translated_text: "Aucune archive sélectionnée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.inspection_not_run",
        translated_text: "L’inspection de l’archive n’a pas été exécutée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.archive_inspection_colon",
        translated_text: "Inspection de l’archive :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.pass",
        translated_text: "RÉUSSI",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.fail",
        translated_text: "ÉCHEC",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.export_format_colon",
        translated_text: "Format d’exportation :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.source_screenshaver_colon",
        translated_text: "Source Screenshaver:",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.source_db_schema_colon",
        translated_text: "Schéma de la base de données source :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.inspection_read_only",
        translated_text: "L’inspection est en lecture seule. Aucune modification n’a été apportée à Screenshaver.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.browse",
        translated_text: "Parcourir...",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.archive_inspection_status",
        translated_text: "Inspection de l’archive : {status}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.passed",
        translated_text: "RÉUSSI",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.failed_status",
        translated_text: "ÉCHEC",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.export_focus_colon",
        translated_text: "Cible d’exportation :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.stage.select_contents",
        translated_text: "Sélectionner le contenu à importer",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.package_objects_selected",
        translated_text: "{count} objets du paquet sélectionnés automatiquement",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.policies",
        translated_text: "Politiques",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.policy_requires_shader",
        translated_text: "[{target}] — nécessite l’ID de nuanceur du paquet {id}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.shaders",
        translated_text: "Nuanceurs",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.package_id",
        translated_text: "ID du paquet {id}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.member_count",
        translated_text: "{count} membres",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.shaders_colon",
        translated_text: "Nuanceurs :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.policies_colon",
        translated_text: "Politiques :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.playlists_colon",
        translated_text: "Listes de lecture :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.back",
        translated_text: "< Retour",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.next",
        translated_text: "Suivant >",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.byte_count",
        translated_text: "{count} octets",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.zip_summary",
        translated_text: "{entries} entrées ; {bytes} octets décompressés",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.object_type.shader",
        translated_text: "Nuanceur",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.object_type.policy",
        translated_text: "Politique",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.object_type.playlist",
        translated_text: "Liste de lecture",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.object_type.receiving_installation",
        translated_text: "Installation de destination",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.conflict.new",
        translated_text: "NOUVEAU",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.conflict.duplicate",
        translated_text: "DOUBLON",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.conflict.conflict",
        translated_text: "CONFLIT",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.conflict_help",
        translated_text: "L’importation est additive et non destructive. Les véritables conflits de noms sont résolus automatiquement en renommant l’objet importé ; les objets existants de l’installation destinataire ne sont jamais modifiés ni supprimés.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.conflict_not_run",
        translated_text: "La détection des conflits n’a pas été exécutée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.conflict_counts",
        translated_text: "{new} nouveau(x), {duplicates} doublon(s), {conflicts} conflit(s)",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.automatic_rename",
        translated_text: "Résolution automatique : Renommer → {name}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.rename_blocked",
        translated_text: "Le renommage automatique est bloqué par des dépendances non résolues.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.proposed_dependency_plan",
        translated_text: "Plan de dépendances proposé",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.dependencies_resolved",
        translated_text: "Tous les objets et dépendances du paquet ont des destinations déterministes.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.conflict_read_only",
        translated_text: "Ce rapport est en lecture seule. Aucune ligne de base de données ni aucun fichier shader n’a été modifié.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.keep_existing",
        translated_text: "Conserver l’ID {id} du {type} existant",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.import_new",
        translated_text: "Importer un nouveau {type}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.unresolved_conflict",
        translated_text: "Conflit non résolu",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.review_ready",
        translated_text: "Screenshaver est prêt à appliquer à cette installation le paquet validé dont les dépendances ont été résolues.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.no_validated_package",
        translated_text: "Aucun paquet validé n’est disponible.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.conflict_unavailable",
        translated_text: "La détection des conflits n’est pas disponible.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.before_import",
        translated_text: "Avant l’importation",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.backup_before_changes",
        translated_text: "Screenshaver créera et vérifiera une sauvegarde permanente horodatée de screenshaver.db avant d’effectuer des modifications persistantes.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.behavior",
        translated_text: "Comportement de l’importation",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.behavior_detail",
        translated_text: "Les fichiers de shader nouveaux et renommés automatiquement seront installés dans le dossier des shaders gérés, les politiques seront restaurées avec leurs paramètres de rendu exportés et les listes de lecture seront reconstruites dans l’ordre canonique. Les objets existants de l’installation de destination ne sont jamais modifiés. Les objets en double réellement identiques sont réutilisés par les dépendances importées.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.toml_unchanged",
        translated_text: "screenshaver.toml ne sera pas modifié.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.no_result",
        translated_text: "Aucun résultat d’importation n’est disponible.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.completed",
        translated_text: "Importation terminée avec succès.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.failed",
        translated_text: "Échec de l’importation.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.playlist_memberships_colon",
        translated_text: "Liste de lecture membreships:",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.shaders_created_colon",
        translated_text: "Shaders créés :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.policies_created_colon",
        translated_text: "Politiques créées :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.playlists_created_colon",
        translated_text: "Listes de lecture créées :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.memberships_created_colon",
        translated_text: "Appartenances créées :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.preimport_backup",
        translated_text: "Sauvegarde de la base de données avant importation :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.close",
        translated_text: "Fermer",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.action",
        translated_text: "Importer",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.shader_identical_same_name",
        translated_text: "Un contenu de shader identique est déjà installé sous le même nom de fichier.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.shader_identical_other_name",
        translated_text: "Un contenu de shader identique est déjà installé sous le nom « {name} » ; le shader physique existant sera conservé.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.shader_name_content_conflict",
        translated_text: "Le nom de fichier existe déjà dans l’inventaire des shaders gérés, mais son contenu diffère de celui du shader importé.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.shader_new",
        translated_text: "Aucun shader installé ne possède ce contenu ou ce nom de fichier géré.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.policy_new",
        translated_text: "Aucune politique {target} portant ce nom de politique n’existe.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.policy_duplicate",
        translated_text: "Une politique existante a la même cible, le même contenu de shader et la même configuration de rendu.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.policy_conflict",
        translated_text: "Le nom de politique existe déjà pour cette cible, mais son shader ou sa configuration de rendu diffère.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.policy_multiple_match",
        translated_text: "Plusieurs politiques de destination correspondent de façon inattendue à ce nom de politique et à cette cible.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.playlist_new",
        translated_text: "Aucune liste de lecture de destination ne porte ce nom.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.playlist_duplicate",
        translated_text: "Une liste de lecture existante a la même description et les mêmes politiques résolues, dans le même ordre canonique.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.playlist_conflict",
        translated_text: "Le nom de la liste de lecture existe déjà, mais sa description ou ses membres résolus diffèrent.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.archive_file",
        translated_text: "Fichier d’archive",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.archive_size",
        translated_text: "Taille de l’archive",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.zip_container",
        translated_text: "Conteneur ZIP",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.zip_entry_count",
        translated_text: "Nombre d’entrées ZIP",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.zip_entry_access",
        translated_text: "Accès aux entrées ZIP",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.resource_limits",
        translated_text: "Limites de ressources de l’archive",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.member_read",
        translated_text: "Lecture d’un membre de l’archive",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.unique_names",
        translated_text: "Noms uniques dans l’archive",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.archive_paths",
        translated_text: "Chemins de l’archive",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.entry_types",
        translated_text: "Types d’entrées de l’archive",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.manifest",
        translated_text: "Manifeste",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.export_schema",
        translated_text: "Schéma d’exportation",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.format_identifier",
        translated_text: "Identifiant de format",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.shader_metadata_structure",
        translated_text: "Shader metadonnées structure",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.shader_archive_paths",
        translated_text: "Chemins d’archive des shaders",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.package_structure",
        translated_text: "Structure du paquet",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.missing_content",
        translated_text: "Contenu d’archive manquant",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.unexpected_content",
        translated_text: "Contenu d’archive inattendu",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.manifest_counts",
        translated_text: "Comptages du manifeste",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.validated_package",
        translated_text: "Paquet validé",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.relationships",
        translated_text: "Relations du paquet",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.shader_payloads",
        translated_text: "Contenus des shaders",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.shader_integrity",
        translated_text: "Intégrité des shaders",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.shader_encoding",
        translated_text: "Encodage de la source du shader",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.shader_payload_inspection",
        translated_text: "Inspection du contenu du shader",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.package_sha256",
        translated_text: "SHA-256 du paquet",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.not_regular_file",
        translated_text: "Le chemin sélectionné ne correspond pas à un fichier ordinaire.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.resource_limit",
        translated_text: "Le développement de l’archive ou une entrée individuelle dépasse les limites d’inspection.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.no_duplicate_names",
        translated_text: "Aucun nom de membre ZIP en double détecté.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.safe_paths",
        translated_text: "Aucun chemin absolu, de traversée, contenant NUL ou une barre oblique inverse n’a été détecté.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.safe_entry_types",
        translated_text: "Aucune entrée de lien symbolique ou de fichier spécial détectée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.manifest_missing",
        translated_text: "Le fichier manifest.json requis est manquant.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.counts_match",
        translated_text: "Les nombres de politiques, shaders et listes de lecture correspondent aux métadonnées.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.counts_mismatch",
        translated_text: "Les nombres du manifeste ne correspondent pas aux métadonnées analysées.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.integrity_record_missing",
        translated_text: "L’enregistrement d’intégrité du manifeste est manquant.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.sha_verified",
        translated_text: "SHA-256 vérifié.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.sha_mismatch",
        translated_text: "SHA-256 différent ou hachage mal formé.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.relationships_valid",
        translated_text: "Les références politique→shader et liste de lecture→politique sont structurellement valides.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.all_shader_hashes_verified",
        translated_text: "Toutes les valeurs SHA-256 des shaders ont été vérifiées.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.all_shader_utf8",
        translated_text: "Chaque contenu de shader est un texte UTF-8 valide.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.package_hash_malformed",
        translated_text: "Le hachage du paquet dans le manifeste est mal formé.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.package_fingerprint_verified",
        translated_text: "Empreinte canonique du paquet vérifiée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.package_fingerprint_mismatch",
        translated_text: "L’empreinte canonique du paquet ne correspond pas au manifeste.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.access_archive",
        translated_text: "Impossible d’accéder à l’archive : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.open_archive",
        translated_text: "Impossible d’ouvrir l’archive : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.invalid_zip",
        translated_text: "Invalidee ZIP archive: {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.zip_entry_error",
        translated_text: "Entrée {index} : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.member_error",
        translated_text: "{name}: {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.manifest_invalid",
        translated_text: "manifest.json est invalidee: {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.format_supported",
        translated_text: "Le format d’exportation Screenshaver {version} est pris en charge.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.shader_path_not_permitted",
        translated_text: "« {path} » n’est pas autorisé par le schéma d’exportation.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.shader_files_present",
        translated_text: "{count} fichiers de shader déclarés sont présents.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.playlist_members",
        translated_text: "Membres de la liste de lecture",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.metadata_title",
        translated_text: "{dataset} metadonnées",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.metadata_hash_title",
        translated_text: "Hachage des métadonnées {dataset}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.structure_title",
        translated_text: "{dataset} structure",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.required_file_missing",
        translated_text: "Request '{file}' est manquant.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.manifest_integrity_missing",
        translated_text: "L’enregistrement d’intégrité du manifeste est manquant.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.sha256_verified",
        translated_text: "SHA-256 vérifié.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.sha256_bad",
        translated_text: "SHA-256 différent ou hachage mal formé.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.rows_header_verified",
        translated_text: "{rows} lignes ; en-tête de schéma vérifié.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.required_members_exact",
        translated_text: "Tous les éléments requis sont présents et aucun fichier inattendu n’a été trouvé.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.review_count",
        translated_text: "{new} à importer, {duplicates} identique(s) déjà présent(s)",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.package_members",
        translated_text: "Paquet membres",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.package_relationships",
        translated_text: "Relations du paquet",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.shader_source_encoding",
        translated_text: "Encodage de la source du shader",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.no_duplicate_names",
        translated_text: "Aucun nom de membre ZIP en double détecté.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.paths_safe",
        translated_text: "Aucun chemin absolu, de traversée, contenant NUL ou une barre oblique inverse n’a été détecté.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.no_special_entries",
        translated_text: "Aucune entrée de lien symbolique ou de fichier spécial détectée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.manifest_missing",
        translated_text: "Le fichier manifest.json requis est manquant.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.package_hash_verified",
        translated_text: "Empreinte canonique du paquet vérifiée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.package_hash_mismatch",
        translated_text: "L’empreinte canonique du paquet ne correspond pas au manifeste.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.relationships_valid",
        translated_text: "Les références politique→shader et liste de lecture→politique sont structurellement valides.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.all_shader_hashes_verified",
        translated_text: "Toutes les valeurs SHA-256 des shaders ont été vérifiées.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.check.all_shader_utf8",
        translated_text: "Chaque contenu de shader est un texte UTF-8 valide.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.dataset.playlists",
        translated_text: "Listes de lecture",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.dataset.playlist_memberships",
        translated_text: "Liste de lecture membreships",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.contents_colon",
        translated_text: "Contenu :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.contents_counts",
        translated_text: "{policies} politiques, {shaders} shaders, {playlists} liste de lectures, {memberships} membreships",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.automatic_resolution_rename",
        translated_text: "Résolution automatique : Renommer → {name}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.unresolved_rename_dependency_counts",
        translated_text: "{renames} renommage(s) automatique(s), {dependencies} dépendance(s) non résolue(s)",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.blocked_counts",
        translated_text: "Importation bloquée : {conflicts} conflit(s), {dependencies} référence(s) de dépendance non résolue(s).",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.review_blocked_explanation",
        translated_text: "L’importation ne peut pas continuer tant que tous les conflits et toutes les dépendances ne sont pas résolus.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.archive_bytes_exceed_limit",
        translated_text: "{actual} octets dépassent la limite d’inspection de {limit} octets.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.archive_entries_exceed_limit",
        translated_text: "{actual} entrées dépassent la limite de {limit} entrées.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.manifest_schema_format_mismatch",
        translated_text: "Manifeste « {manifest} » ; schéma « {schema} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.local_timestamp_failed",
        translated_text: "Impossible de déterminer l’horodatage local de l’importation : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.unique_import_name_failed",
        translated_text: "Impossible de générer un nom importé unique pour « {name} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.no_deterministic_rename",
        translated_text: "{type} « {name} » ne dispose d’aucun renommage importé déterministe.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.missing_package_shader",
        translated_text: "Shader du paquet manquant.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.missing_package_policy",
        translated_text: "Manquant paquet politique.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.missing_package_playlist",
        translated_text: "Manquant paquet liste de lecture.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.conflict_discovery_unavailable",
        translated_text: "Détection des conflits indisponible",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.conflict_database_open_failed",
        translated_text: "Impossible d’ouvrir screenshaver.db en lecture seule pour la détection des conflits : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.shader_hash_prepare_failed",
        translated_text: "Impossible de préparer la recherche du hachage du shader : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.shader_hash_query_failed",
        translated_text: "Impossible d’interroger le hachage de shader « {hash} » : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.shader_hash_decode_failed",
        translated_text: "Impossible de décoder la recherche du hachage du shader : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.shader_filename_check_failed",
        translated_text: "Impossible de vérifier le nom de fichier du shader '{filename}' : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.policy_name_prepare_failed",
        translated_text: "Impossible de préparer la recherche du nom de politique : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.policy_name_query_failed",
        translated_text: "Impossible d’interroger le nom de politique '{name}' : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.policy_name_decode_failed",
        translated_text: "Impossible de décoder la recherche du nom de politique : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.playlist_query_failed",
        translated_text: "Impossible d’interroger la liste de lecture '{name}' : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.playlist_member_prepare_failed",
        translated_text: "Impossible de préparer la recherche d’un membre de la liste de lecture : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.playlist_members_query_failed",
        translated_text: "Impossible d’interroger les membres de la liste de lecture : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.playlist_members_decode_failed",
        translated_text: "Impossible de décoder les membres de la liste de lecture : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.validated_policy_missing_field",
        translated_text: "La politique validée '{policy}' ne contient pas '{field}'.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.invalid_number_in_field",
        translated_text: "Nombre '{value}' non valide dans {field}.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.invalid_integer_in_field",
        translated_text: "Entier '{value}' non valide dans {field}.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.invalid_boolean_in_field",
        translated_text: "Valeur booléenne '{value}' non valide dans {field}.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.unsupported_policy_target",
        translated_text: "Cible de politique importée non prise en charge « {target} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.receiving_defaults_texture_mode",
        translated_text: "Les valeurs par défaut {target} de destination utilisent un mode de texture non pris en charge « {mode} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.receiving_policy_texture_mode",
        translated_text: "La politique de destination utilise un mode de texture non pris en charge « {mode} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.receiving_defaults_palette_mode",
        translated_text: "Les valeurs par défaut {target} de destination utilisent un mode de palette non pris en charge « {mode} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.diag.receiving_policy_palette_mode",
        translated_text: "La politique de destination utilise un mode de palette non pris en charge « {mode} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.policy_with_id",
        translated_text: "Politique {id}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.archive_reinspection_failed",
        translated_text: "L’archive ne satisfait plus à l’inspection. Aucune modification d’importation n’a été effectuée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.validated_package_unavailable",
        translated_text: "Valideated paquet est undisponible.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.conflict_timestamp_unavailable",
        translated_text: "L’horodatage des conflits d’importation est indisponible. Aucune modification d’importation n’a été effectuée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.destination_name_changed",
        translated_text: "L’installation de destination a changé après la révision : « {previous} » n’est plus le nom de destination déterministe (désormais « {current} »). Aucune modification d’importation n’a été effectuée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.conflict_set_changed",
        translated_text: "L’installation de destination a changé après la révision et l’ensemble des conflits n’est plus le même. Aucune modification d’importation n’a été effectuée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.new_conflicts_discovered",
        translated_text: "L’installation de destination a changé après la révision et de nouveaux conflits ont été détectés. Aucune modification d’importation n’a été effectuée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.conflict_resolution_stale",
        translated_text: "La résolution des conflits d’importation est incomplète ou obsolète : {error} Aucune modification d’importation n’a été effectuée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.unresolved_conflicts",
        translated_text: "La détection des conflits au moment de l’exécution a trouvé {count} conflit(s) non résolu(s) :\n\n{details}\n\nAucune modification d’importation n’a été effectuée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.policy_unresolved_destination",
        translated_text: "La politique « {name} » (ID de paquet {id}) possède une destination non résolue.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.policy_unresolved_shader",
        translated_text: "La politique « {name} » (ID de paquet {id}) nécessite l’ID de paquet de shader non résolu {shader_id}.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.playlist_unresolved_destination",
        translated_text: "La liste de lecture « {name} » (ID de paquet {id}) possède une destination non résolue.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.playlist_member_unresolved_policy",
        translated_text: "Le membre {position} « {policy} » de la liste de lecture « {playlist} » nécessite l’ID de paquet de politique non résolu {policy_id}.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.unresolved_dependencies",
        translated_text: "La validation des dépendances au moment de l’exécution a trouvé {count} référence(s) de dépendance non résolue(s) :\n\n{details}\n\nAucune modification d’importation n’a été effectuée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.failed_database_restored",
        translated_text: "{error} La base de données préalable à l’importation a été restaurée. Les nouveaux fichiers de shader installés ont été supprimés lorsque cela était possible.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.failed_restore_also_failed",
        translated_text: "{error} LA RESTAURATION DE LA BASE DE DONNÉES A ÉGALEMENT ÉCHOUÉ : {restore_error}. La sauvegarde vérifiée reste disponible à « {backup} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.backup_timestamp_failed",
        translated_text: "Impossible de déterminer l’horodatage de la sauvegarde préalable à l’importation : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.backup_create_failed",
        translated_text: "Impossible de créer la sauvegarde de base de données préalable à l’importation « {path} » : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.backup_open_verify_failed",
        translated_text: "Impossible d’ouvrir la sauvegarde de base de données préalable à l’importation pour vérification : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.backup_verify_failed",
        translated_text: "Impossible de vérifier la sauvegarde de base de données préalable à l’importation : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.backup_integrity_failed",
        translated_text: "La vérification d’intégrité de la sauvegarde de base de données préalable à l’importation a échoué : {result}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.restore_copy_failed",
        translated_text: "Impossible de restaurer '{database}' depuis '{backup}': {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.restored_database_open_failed",
        translated_text: "Impossible d’ouvrir la base de données restaurée : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.restored_database_verify_failed",
        translated_text: "Impossible de vérifier la base de données restaurée : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.restored_database_integrity_result",
        translated_text: "Le contrôle integrity_check de la base de données restaurée a renvoyé '{result}'.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.shader_directory_create_failed",
        translated_text: "Impossible de créer le dossier des shaders gérés « {path} » : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.archive_reopen_failed",
        translated_text: "Impossible de rouvrir l’archive d’importation : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.zip_reopen_failed",
        translated_text: "Impossible de rouvrir l’archive ZIP d’importation : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.shader_overwrite_refused",
        translated_text: "Refus d’écraser le shader existant « {path} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.shader_payload_read_failed",
        translated_text: "Impossible de lire le contenu du shader importé « {path} » : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.shader_changed_after_inspection",
        translated_text: "Le shader importé « {filename} » a changé après l’inspection. Aucune modification d’importation n’a été effectuée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.shader_install_failed",
        translated_text: "Impossible d’installer le shader '{path}' : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.shader_reconcile_failed",
        translated_text: "Impossible de rapprocher les shaders après l’importation : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.shader_registration_failed",
        translated_text: "Le shader importé « {filename} » n’a pas été enregistré comme prévu : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.transaction_begin_failed",
        translated_text: "Impossible de démarrer la transaction d’importation : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.policy_no_destination_shader",
        translated_text: "La politique importée « {name} » ne possède aucun shader de destination résolu.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.playlist_import_failed",
        translated_text: "Impossible d’importer la liste de lecture '{name}' : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.playlist_id_no_mapping",
        translated_text: "L’ID de liste de lecture importé {id} ne possède aucun mappage de destination.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.policy_id_no_membership_mapping",
        translated_text: "L’ID de politique importé {id} ne possède aucun mappage pour l’appartenance à une liste de lecture.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.membership_restore_failed",
        translated_text: "Impossible de restaurer l’appartenance à la liste de lecture à la position {position} : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.transaction_commit_failed",
        translated_text: "Impossible de valider la transaction d’importation : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.success_detail",
        translated_text: "Le paquet validé a été importé avec succès. Les véritables conflits ont été conservés de manière additive sous des noms importés déterministes ; les objets existants de l’installation de destination n’ont pas été modifiés. Les doublons réellement identiques ont été réutilisés et les dépendances importées ont été associées à leurs identités de destination. screenshaver.toml n’a pas été modifié.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.imported_policy_missing_field",
        translated_text: "La politique importée '{policy}' ne contient pas '{field}'.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.invalid_policy_integer",
        translated_text: "Entier '{value}' non valide dans le champ de politique importé '{field}'.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.invalid_policy_number",
        translated_text: "Nombre '{value}' non valide dans le champ de politique importé '{field}'.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.invalid_policy_boolean",
        translated_text: "Valeur booléenne '{value}' non valide dans le champ de politique importé '{field}'.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.unsupported_texture_mode",
        translated_text: "Mode de texture importé non pris en charge « {mode} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.unsupported_palette_mode",
        translated_text: "Mode de palette importé non pris en charge « {mode} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.unsupported_animation_speed_mode",
        translated_text: "Mode de vitesse d’animation importé non pris en charge « {mode} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.policy_import_failed",
        translated_text: "Impossible d’importer la politique '{name}' : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.imported_name_length_invalid",
        translated_text: "Le nom importé doit contenir entre 1 et 128 caractères ; {length} trouvés.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.exec.imported_name_empty_key",
        translated_text: "Le nom importé a produit une clé de comparaison vide.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.export_format_unsupported",
        translated_text: "Le format d’exportation Screenshaver {version} n’est pas pris en charge par cette installation.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.export_schema_load_failed",
        translated_text: "Impossible de charger le schéma d’exportation {version} : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.export_schema_version_mismatch",
        translated_text: "La version du schéma d’exportation ne correspond pas à l’archive.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.export_schema_integrity_unsupported",
        translated_text: "Le schéma d’exportation demande un comportement d’intégrité non pris en charge.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.export_schema_manifest_hashing",
        translated_text: "Le schéma d’exportation doit exclure son manifeste du hachage du paquet.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.schema_metadata_undeclared",
        translated_text: "Le fichier de métadonnées du schéma '{file}' n’est pas déclaré dans l’archive.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.dataset_not_utf8",
        translated_text: "'{file}' n’est pas UTF-8: {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.dataset_empty",
        translated_text: "'{file}' est vide.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.dataset_header_mismatch",
        translated_text: "L’en-tête de « {file} » ne correspond pas au schéma d’exportation.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.dataset_row_error",
        translated_text: "'{file}' ligne {row}: {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.dataset_column_count",
        translated_text: "La ligne {row} de « {file} » contient {actual} colonnes ; {required} requises.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.tsv_trailing_escape",
        translated_text: "Séquence d’échappement TSV finale incomplète.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.tsv_escape_unsupported",
        translated_text: "Séquence d’échappement TSV non prise en charge « \\\\{escape} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.schema_dataset_missing_column",
        translated_text: "Le jeu de données du schéma « {file} » ne contient pas la colonne « {column} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.duplicate_id",
        translated_text: "ID {id} de {label} en double.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.not_positive_integer",
        translated_text: "{label} « {value} » n’est pas un entier positif.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.must_be_positive",
        translated_text: "{label} doit être supérieur à zéro.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.policy_missing_shader_id",
        translated_text: "La politique référence un shader_export_id {id} manquant.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.membership_unresolved_id",
        translated_text: "L’appartenance à la liste de lecture contient un ID de paquet non résolu.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.playlist_duplicate_policy",
        translated_text: "La liste de lecture {playlist} contient la politique {policy} plusieurs fois.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.playlist_duplicate_position",
        translated_text: "La liste de lecture {playlist} contient la position {position} en double.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.shader_declared_twice",
        translated_text: "Le shader « {path} » est déclaré plusieurs fois.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.shader_sha_malformed",
        translated_text: "Le shader « {path} » possède un SHA-256 mal formé.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.declared_shader_missing",
        translated_text: "Le shader déclaré '{path}' est manquant.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.declared_shader_directory",
        translated_text: "Le shader déclaré '{path}' est un répertoire.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.shader_executable",
        translated_text: "Le shader « {path} » est marqué comme exécutable.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.shader_sha_failed",
        translated_text: "Le shader '{path}' a échoué à la vérification SHA-256.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "import.validation.shader_not_utf8",
        translated_text: "Le shader '{path}' n’est pas un texte UTF-8 valide.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_load_playlists_for_export_selection",
        translated_text: "Impossible de charger les listes de lecture pour la sélection d’exportation : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_load_schema",
        translated_text: "Impossible de charger le schéma d’exportation Screenshaver V1 : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.schema_has_an_empty_format_identifier",
        translated_text: "Le schéma d’exportation Screenshaver V1 contient un identifiant de format vide.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.schema_has_an_invalid_format_version",
        translated_text: "Le schéma d’exportation Screenshaver V1 contient une version de format non valide.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.schema_requests_an_unsupported_integrity_algorithm_or_canonicalization",
        translated_text: "Le schéma d’exportation Screenshaver V1 demande un algorithme d’intégrité ou une canonisation non pris en charge.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.schema_must_exclude_its_manifest_from_the_package_hash",
        translated_text: "Le schéma d’exportation Screenshaver V1 doit exclure son manifeste du hachage du paquet.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.schema_dataset_is_not_declared_as_archive_metadata",
        translated_text: "Le jeu de données « {value1} » du schéma d’exportation Screenshaver V1 n’est pas déclaré comme métadonnée d’archive.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.missing_package_local_export_id_for_policy",
        translated_text: "Identifiant d’exportation local au paquet manquant pour la politique « {value1} »",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.missing_package_local_export_id_for_shader_referenced_by_policy",
        translated_text: "Identifiant d’exportation local au paquet manquant pour le shader « {value1} » référencé par la politique « {value2} »",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.missing_package_local_export_id_for_playlist",
        translated_text: "Identifiant d’exportation local au paquet manquant pour la liste de lecture « {value1} »",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_load_members_for_playlist_while_exporting",
        translated_text: "Impossible de charger les membres de la liste de lecture {value1} pendant l’exportation : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.missing_package_local_export_id_for_policy_in_playlist",
        translated_text: "Identifiant d’exportation local au paquet manquant pour la politique {value1} dans la liste de lecture « {value2} »",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_read_shader_at",
        translated_text: "Impossible de lire le shader « {value1} » à l’emplacement « {value2} » : {value3}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.missing_package_local_export_id_for_shader",
        translated_text: "Identifiant d’exportation local au paquet manquant pour le shader « {value1} »",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_create_in_export_archive",
        translated_text: "Impossible de créer « {value1} » dans l’archive d’exportation : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_write_to_export_archive",
        translated_text: "Impossible d’écrire « {value1} » dans l’archive d’exportation : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.the_export_destination_is_not_valid",
        translated_text: "La destination de l’exportation n’est pas valide.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.the_export_destination_folder_is_not_valid",
        translated_text: "Le dossier de destination de l’exportation n’est pas valide.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.export_destination_folder_does_not_exist",
        translated_text: "Le dossier de destination de l’exportation n’existe pas : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.the_effective_policy_snapshot_is_incomplete_return_to_selection_and_try_again",
        translated_text: "L’instantané des politiques effectives est incomplet. Revenez à la sélection et réessayez.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_read_shader_while_calculating_package_integrity",
        translated_text: "Impossible de lire le shader « {value1} » lors du calcul de l’intégrité du paquet : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_serialize_export_manifest",
        translated_text: "Impossible de sérialiser le manifeste d’exportation : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_remove_stale_temporary_export",
        translated_text: "Impossible de supprimer l’exportation temporaire obsolète « {value1} » : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_create_temporary_export_archive",
        translated_text: "Impossible de créer l’archive d’exportation temporaire « {value1} » : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_add_database_snapshot_to_backup_archive",
        translated_text: "Impossible d’ajouter l’instantané de la base de données à l’archive de sauvegarde : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_write_database_snapshot_to_backup_archive",
        translated_text: "Impossible d’écrire l’instantané de la base de données dans l’archive de sauvegarde : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_add_managed_shader_to_backup_archive",
        translated_text: "Impossible d’ajouter le shader géré « {value1} » à l’archive de sauvegarde : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_write_managed_shader_to_backup_archive",
        translated_text: "Impossible d’écrire le shader géré « {value1} » dans l’archive de sauvegarde : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_add_shader_to_export_archive",
        translated_text: "Impossible d’ajouter le shader « {value1} » à l’archive d’exportation : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_open_shader",
        translated_text: "Impossible d’ouvrir le shader « {value1} » : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_read_shader",
        translated_text: "Impossible de lire le shader « {value1} » : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_write_shader_to_export_archive",
        translated_text: "Impossible d’écrire le shader « {value1} » dans l’archive d’exportation : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_finalize_export_archive",
        translated_text: "Impossible de finaliser l’archive d’exportation : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_move_completed_export_archive_to",
        translated_text: "Impossible de déplacer l’archive d’exportation terminée vers « {value1} » : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_enumerate_managed_shader_directory",
        translated_text: "Impossible d’énumérer le répertoire des shaders gérés « {value1} » : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_read_managed_shader_directory_entry",
        translated_text: "Impossible de lire une entrée du répertoire des shaders gérés : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_inspect_managed_shader_entry",
        translated_text: "Impossible d’inspecter l’entrée de shader géré « {value1} » : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_read_managed_shader_for_full_backup",
        translated_text: "Impossible de lire le shader géré « {value1} » pour la sauvegarde complète : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_remove_stale_database_snapshot",
        translated_text: "Impossible de supprimer l’ancien instantané de base de données « {value1} » : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_open_database_for_backup_snapshot",
        translated_text: "Impossible d’ouvrir la base de données pour l’instantané de sauvegarde : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_create_consistent_database_snapshot",
        translated_text: "Impossible de créer un instantané cohérent de la base de données : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_open_database_snapshot_for_verification",
        translated_text: "Impossible d’ouvrir l’instantané de base de données pour vérification : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_verify_database_snapshot",
        translated_text: "Impossible de vérifier l’instantané de base de données : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.database_snapshot_failed_integrity_verification",
        translated_text: "L’instantané de base de données a échoué à la vérification d’intégrité : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_read_verified_database_snapshot",
        translated_text: "Impossible de lire l’instantané de base de données vérifié « {value1} » : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_create_backup_directory",
        translated_text: "Impossible de créer le répertoire de sauvegarde « {value1} » : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_prepare_full_backup_selection",
        translated_text: "Impossible de préparer la sélection de sauvegarde complète : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_resolve_full_backup_policies",
        translated_text: "Impossible de résoudre les politiques de sauvegarde complète : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_determine_backup_filename_timestamp",
        translated_text: "Impossible de déterminer l’horodatage du nom de fichier de sauvegarde.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_prepare_effective_export_policy_query",
        translated_text: "Impossible de préparer la requête des politiques effectives d’exportation : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_read_selected_policy_id_while_resolving_export_data",
        translated_text: "Impossible de lire l’identifiant de politique sélectionné {value1} lors de la résolution des données d’exportation : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.policy_has_unsupported_target",
        translated_text: "La politique « {value1} » possède une cible non prise en charge « {value2} »",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.one_or_more_selected_policies_disappeared_while_export_data_was_being_resolved",
        translated_text: "Une ou plusieurs politiques sélectionnées ont disparu pendant la résolution des données d’exportation",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.policy_has_a_specific_texture_without_a_texture_family",
        translated_text: "La politique « {value1} » possède une texture spécifique sans famille de textures",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.policy_has_a_specific_texture_without_a_primitive_count",
        translated_text: "La politique « {value1} » possède une texture spécifique sans nombre de primitives",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.policy_has_unsupported_texture_mode",
        translated_text: "La politique « {value1} » possède un mode de texture non pris en charge « {value2} »",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.defaults_specify_a_specific_texture_without_a_texture_family_while_resolving_policy",
        translated_text: "Les valeurs par défaut de {value1} spécifient une texture particulière sans famille de textures lors de la résolution de la politique « {value2} »",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.defaults_contain_unsupported_texture_mode_while_resolving_policy",
        translated_text: "Les valeurs par défaut de {value1} contiennent un mode de texture non pris en charge « {value2} » lors de la résolution de la politique « {value3} »",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.policy_has_a_specific_palette_without_a_palette_color",
        translated_text: "La politique « {value1} » possède une palette spécifique sans couleur de palette",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.policy_has_unsupported_palette_mode",
        translated_text: "La politique « {value1} » possède un mode de palette non pris en charge « {value2} »",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.defaults_specify_a_specific_palette_without_a_palette_color_while_resolving_policy",
        translated_text: "Les valeurs par défaut de {value1} spécifient une palette particulière sans couleur de palette lors de la résolution de la politique « {value2} »",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.defaults_contain_unsupported_palette_mode_while_resolving_policy",
        translated_text: "Les valeurs par défaut de {value1} contiennent un mode de palette non pris en charge « {value2} » lors de la résolution de la politique « {value3} »",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.policy_has_invalid_boolean_value_for",
        translated_text: "La politique « {value1} » possède la valeur booléenne non valide {value2} pour {value3}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.assigned_policy_retained_unresolved_target_texture_inheritance",
        translated_text: "La politique attribuée « {value1} » a conservé un héritage non résolu de la texture cible",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.assigned_policy_retained_unresolved_target_palette_inheritance",
        translated_text: "La politique attribuée « {value1} » a conservé un héritage non résolu de la palette cible",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.assigned_policy_retained_unresolved_target_animation_speed_inheritance",
        translated_text: "La politique attribuée « {value1} » a conservé un héritage non résolu de la vitesse d’animation cible",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.policy_resolved_to_an_invalid_specific_texture",
        translated_text: "La politique « {value1} » a été résolue en une texture spécifique non valide",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.assigned_policy_resolved_to_random_texture_without_an_explicit_primitive_count",
        translated_text: "La politique attribuée « {value1} » a été résolue en une texture aléatoire sans nombre explicite de primitives",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.policy_resolved_to_an_invalid_random_texture_primitive_count",
        translated_text: "La politique « {value1} » a été résolue avec un nombre de primitives de texture aléatoire non valide",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.policy_resolved_to_one_or_more_invalid_numeric_export_values",
        translated_text: "La politique « {value1} » a été résolue avec une ou plusieurs valeurs numériques d’exportation non valides",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.policy_resolved_to_an_invalid_animation_speed",
        translated_text: "La politique « {value1} » a été résolue avec une vitesse d’animation non valide",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_prepare_export_policy_selection_query",
        translated_text: "Impossible de préparer la requête de sélection des politiques d’exportation : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_query_policies_for_export_selection",
        translated_text: "Impossible d’interroger les politiques pour la sélection d’exportation : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.error.unable_to_decode_export_policy_selection_row",
        translated_text: "Impossible de décoder la ligne de sélection des politiques d’exportation : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "export.review_instruction",
        translated_text: "Vérifiez la cible d’exportation sélectionnée, les politiques, shaders et listes de lecture inclus, ainsi que la destination avant de lancer l’exportation.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.edit_shader_requires_a_shader_file_not_a_directory",
        translated_text: "--edit-shader nécessite un fichier shader, et non un répertoire : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.bulk_edit_complete_policies_updated",
        translated_text: "Modification groupée terminée : {value1} politiques mises à jour.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.bulk_edit_complete_policies_updated_policy_target_was_preserved_for_protect",
        translated_text: "Modification groupée terminée : {value1} politiques mises à jour. La cible de politique a été conservée pour {value2} {value3} par défaut protégés.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.bulk_edit_contains_no_changed_settings",
        translated_text: "La modification groupée ne contient aucun paramètre modifié.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.bulk_edit_could_not_suspend_the_active_shader_because_its_database_id_could",
        translated_text: "La modification groupée n’a pas pu suspendre le shader actif, car son identifiant de base de données est introuvable.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.bulk_edit_could_not_suspend_the_active_shader",
        translated_text: "La modification groupée n’a pas pu suspendre le shader actif : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.bulk_edit_ended_but_the_previous_shader_could_not_be_reloaded",
        translated_text: "La modification groupée est terminée, mais le shader précédent n’a pas pu être rechargé : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.bulk_edit_ended_the_previously_loaded_shader_is_no_longer_available",
        translated_text: "La modification groupée est terminée ; le shader précédemment chargé n’est plus disponible.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.bulk_policies_were_saved_but_configuration_reload_failed",
        translated_text: "Les politiques modifiées en groupe ont été enregistrées, mais le rechargement de la configuration a échoué.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.bulk_policy_creation_canceled",
        translated_text: "Création groupée de politiques annulée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.bulk_policy_creation_complete_created_already_existed",
        translated_text: "Création groupée de politiques terminée : {value1} créées, {value2} existaient déjà.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.bulk_policy_creation_failed",
        translated_text: "Échec de la création groupée de politiques : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.bulk_policy_save_aborted",
        translated_text: "Enregistrement groupé des politiques interrompu : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.configuration_save_failed",
        translated_text: "Échec de l’enregistrement de la configuration.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.configuration_saved",
        translated_text: "Configuration enregistrée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.gl_shader_files",
        translated_text: "Fichiers shader GL",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.loaded_and_rendering",
        translated_text: "Chargé et en cours de rendu",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.loaded_existing_screensaver_policy_for_this_shader",
        translated_text: "Politique d’économiseur d’écran existante chargée pour ce shader.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.loaded_existing_unassigned_policy_for_this_shader",
        translated_text: "Politique non attribuée existante chargée pour ce shader.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.loaded_existing_wallpaper_policy_for_this_shader",
        translated_text: "Politique de fond d’écran existante chargée pour ce shader.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.loaded_existing_policy_for_this_shader",
        translated_text: "Politique {value1} existante chargée pour ce shader.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.loaded_shader_using_resolved_defaults_select_a_policy_target_to_create_a_po",
        translated_text: "Shader chargé avec les valeurs par défaut résolues. Sélectionnez une cible de politique pour créer une politique.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.loaded_shader_with_its_existing_screensaver_policy",
        translated_text: "Shader chargé avec sa politique d’économiseur d’écran existante.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.loaded_shader_with_its_existing_unassigned_policy",
        translated_text: "Shader chargé avec sa politique non attribuée existante.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.loaded_shader_with_its_existing_wallpaper_policy",
        translated_text: "Shader chargé avec sa politique de fond d’écran existante.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.new_unassigned_policy_is_ready_to_save",
        translated_text: "La nouvelle politique non attribuée est prête à être enregistrée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.no_screensaver_policy_exists_loaded_screensaver_defaults",
        translated_text: "Aucune politique d’économiseur d’écran n’existe. Valeurs par défaut de l’économiseur d’écran chargées.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.no_unassigned_policy_exists_loaded_defaults_for_a_new_unassigned_policy",
        translated_text: "Aucune politique non attribuée n’existe. Valeurs par défaut chargées pour une nouvelle politique non attribuée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.no_wallpaper_policy_exists_loaded_wallpaper_defaults",
        translated_text: "Aucune politique de fond d’écran n’existe. Valeurs par défaut du fond d’écran chargées.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.no_existing_shader_policy_found_select_a_policy_target_to_create_one",
        translated_text: "Aucune politique de shader existante trouvée. Sélectionnez une cible de politique pour en créer une.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.no_policies_changed_policy_target_cannot_be_changed_for_protected_default",
        translated_text: "Aucune politique modifiée. La cible de politique ne peut pas être modifiée pour {value1} {value2} par défaut protégés.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.no_policy_target_was_selected_for_external_shader",
        translated_text: "Aucune cible de politique n’a été sélectionnée pour le shader externe {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.no_shader_path_was_supplied_for_editing",
        translated_text: "Aucun chemin de shader n’a été fourni pour la modification",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.no_usable_shaders_were_selected_for_policy_creation",
        translated_text: "Aucun shader utilisable n’a été sélectionné pour la création de politiques.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.no_policy_exists_loaded_defaults",
        translated_text: "Aucune politique {value1} n’existe. Valeurs par défaut {value2} chargées.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.not_required",
        translated_text: "Non requis",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.playlist_display_mode_requires_a_playlist_selection",
        translated_text: "Le mode d’affichage Liste de lecture nécessite la sélection d’une liste de lecture.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.playlist_display_mode_requires_a_positive_interval",
        translated_text: "Le mode d’affichage Liste de lecture nécessite un intervalle positif.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.policies_were_created_but_configuration_reload_failed",
        translated_text: "Les politiques ont été créées, mais le rechargement de la configuration a échoué.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.policy_has_unsupported_policy_target",
        translated_text: "La politique « {value1} » possède une cible de politique non prise en charge « {value2} »",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.policy_cannot_be_opened_because_its_shader_is_not_renderable",
        translated_text: "La politique ne peut pas être ouverte, car son shader ne peut pas être rendu : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.policy_cloned_as",
        translated_text: "Politique clonée sous le nom « {value1} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.policy_paths_could_not_be_updated_after_moving_the_shader_rollback_also_fai",
        translated_text: "Les chemins des politiques n’ont pas pu être mis à jour après le déplacement du shader : {value1}. Le retour arrière a également échoué : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.policy_renamed_to",
        translated_text: "Politique renommée en « {value1} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.policy_saved_for",
        translated_text: "Politique enregistrée pour {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.policy_saved_but_audio_motion_could_not_be_saved",
        translated_text: "Politique enregistrée, mais le mouvement audio n’a pas pu être enregistré : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.policy_shader_file_is_unavailable",
        translated_text: "Le fichier shader de la politique est indisponible : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.policy_was_cloned_but_configuration_reload_failed",
        translated_text: "La politique a été clonée, mais le rechargement de la configuration a échoué : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.policy_was_renamed_but_configuration_reload_failed",
        translated_text: "La politique a été renommée, mais le rechargement de la configuration a échoué : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.recent_files_were_cleared_for_this_session_but_the_history_file_could_not_b",
        translated_text: "Les fichiers récents ont été effacés pour cette session, mais le fichier d’historique n’a pas pu être mis à jour : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.recent_shader_file_no_longer_exists",
        translated_text: "Le fichier shader récent n’existe plus : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.recent_shader_file_history_cleared",
        translated_text: "Historique des fichiers shader récents effacé.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.refreshed_shader_from_disk",
        translated_text: "Shader actualisé depuis le disque : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.sdl_initialization_failed",
        translated_text: "Échec de l’initialisation de SDL : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.sdl_video_initialization_failed",
        translated_text: "Échec de l’initialisation vidéo de SDL : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.screensaver_target_enforced_by_shader_location_new_screensaver_policy_is_re",
        translated_text: "Cible Économiseur d’écran imposée par l’emplacement du shader. La nouvelle politique d’économiseur d’écran est prête à être enregistrée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.control_center",
        translated_text: "Centre de contrôle Screenshaver",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.export_archive",
        translated_text: "Archive d’exportation Screenshaver",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.shader_has_file_status",
        translated_text: "Le shader « {value1} » possède l’état de fichier « {value2} »",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.shader_has_invalid_channel_usage_mask",
        translated_text: "Le shader « {value1} » possède un masque d’utilisation des canaux non valide {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.shader_has_validation_status_expected",
        translated_text: "Le shader « {value1} » possède l’état de validation « {value2} » ; « valide » était attendu",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.shader_is_not_registered_in_the_database",
        translated_text: "Le shader « {value1} » n’est pas enregistré dans la base de données Screenshaver",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.shader_is_rejected",
        translated_text: "Le shader « {value1} » est rejeté : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.shader_already_exists_in",
        translated_text: "Le shader existe déjà dans {value1}.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.shader_file_is_unavailable",
        translated_text: "Le fichier shader est indisponible : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.shader_file_no_longer_exists",
        translated_text: "Le fichier shader n’existe plus : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.shader_filename_is_not_valid_utf_8",
        translated_text: "Le nom du fichier shader n’est pas en UTF-8 valide : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.shader_loaded_but_recent_file_history_could_not_be_saved",
        translated_text: "Shader chargé, mais l’historique des fichiers récents n’a pas pu être enregistré : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.shader_loading_canceled",
        translated_text: "Chargement du shader annulé.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.shader_move_was_rolled_back_because_policy_paths_could_not_be_updated",
        translated_text: "Le déplacement du shader a été annulé, car les chemins des politiques n’ont pas pu être mis à jour : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.shader_moved_to",
        translated_text: "Shader déplacé vers {value1}.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.shader_moved_to_policy_target_updated_to",
        translated_text: "Shader déplacé vers {value1}. Cible de politique mise à jour vers {value2}.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.shader_moved_configuration_reload_failed",
        translated_text: "Shader déplacé ; le rechargement de la configuration a échoué.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.shader_path_has_no_valid_filename",
        translated_text: "Le chemin du shader ne contient aucun nom de fichier valide : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.shader_was_not_deleted_because_its_associated_policy_could_not_be_deleted",
        translated_text: "Le shader n’a pas été supprimé, car sa politique associée n’a pas pu être supprimée : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.single_display_mode_requires_a_shader_policy_selection",
        translated_text: "Le mode d’affichage Unique nécessite la sélection d’une politique de shader.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.the_selected_policy_target_is_unavailable_in_the_current_editing_session",
        translated_text: "La cible de politique sélectionnée est indisponible dans la session de modification actuelle.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.the_selected_shader_could_not_be_loaded_for_editing",
        translated_text: "Le shader sélectionné n’a pas pu être chargé pour modification",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.this_shader_cannot_use_a_screensaver_policy_in_the_current_editing_session",
        translated_text: "Ce shader ne peut pas utiliser une politique d’économiseur d’écran dans la session de modification actuelle.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.this_shader_cannot_use_a_wallpaper_policy_in_the_current_editing_session",
        translated_text: "Ce shader ne peut pas utiliser une politique de fond d’écran dans la session de modification actuelle.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.this_shader_cannot_use_an_unassigned_policy_in_the_current_editing_session",
        translated_text: "Ce shader ne peut pas utiliser une politique non attribuée dans la session de modification actuelle.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_clone_policy",
        translated_text: "Impossible de cloner la politique : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_create_control_center_state_folder",
        translated_text: "Impossible de créer le dossier d’état du Centre de contrôle {value1} : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_create_opengl_context",
        translated_text: "Impossible de créer le contexte OpenGL : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_create_sdl_event_pump",
        translated_text: "Impossible de créer la pompe d’événements SDL : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_create_destination_directory",
        translated_text: "Impossible de créer le répertoire de destination {value1} ({value2})",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_create_edit_shader_opengl_context",
        translated_text: "Impossible de créer le contexte OpenGL de --edit-shader : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_create_edit_shader_sdl_event_pump",
        translated_text: "Impossible de créer la pompe d’événements SDL de --edit-shader : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_create_edit_shader_window",
        translated_text: "Impossible de créer la fenêtre de --edit-shader : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_decode_policy_list_database_row",
        translated_text: "Impossible de décoder la ligne de base de données de la Liste des politiques : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_delete_policy",
        translated_text: "Impossible de supprimer la politique : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_load_shader",
        translated_text: "Impossible de charger le shader : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_move_shader_from_to",
        translated_text: "Impossible de déplacer le shader de {value1} vers {value2} ({value3})",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_open_database_while_reading_shader_metadata_for",
        translated_text: "Impossible d’ouvrir la base de données pendant la lecture des métadonnées du shader « {value1} » : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_prepare_policy_list_database_query",
        translated_text: "Impossible de préparer la requête de base de données de la Liste des politiques : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_prepare_policy_clone",
        translated_text: "Impossible de préparer le clonage de la politique : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_query_policy_list_rows_from_database",
        translated_text: "Impossible d’interroger les lignes de la Liste des politiques dans la base de données : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_query_shader_id_for",
        translated_text: "Impossible d’interroger l’identifiant du shader « {value1} » : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_read_database_metadata_for",
        translated_text: "Impossible de lire les métadonnées de la base de données pour « {value1} » : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_refresh_shader",
        translated_text: "Impossible d’actualiser le shader : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_rename_policy",
        translated_text: "Impossible de renommer la politique : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_resolve_shader_id",
        translated_text: "Impossible de résoudre shader_id {value1} : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_restore_control_center_fullscreen_state",
        translated_text: "Impossible de restaurer l’état plein écran du Centre de contrôle Screenshaver : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_restore_the_previous_shader_after_bulk_edit",
        translated_text: "Impossible de restaurer le shader précédent après la modification groupée : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_save_bulk_policy_changes",
        translated_text: "Impossible d’enregistrer les modifications groupées des politiques : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_save_policy",
        translated_text: "Impossible d’enregistrer la politique : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_serialize_control_center_state",
        translated_text: "Impossible de sérialiser l’état du Centre de contrôle : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unable_to_write_control_center_state",
        translated_text: "Impossible d’écrire l’état du Centre de contrôle {value1} : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.unsupported_display_mode",
        translated_text: "Mode d’affichage non pris en charge « {value1} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.valid_shader_has_no_channel_usage_metadata",
        translated_text: "Le shader valide « {value1} » ne possède aucune métadonnée d’utilisation des canaux",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.wallpaper_target_enforced_by_shader_location_new_wallpaper_policy_is_ready",
        translated_text: "Cible Fond d’écran imposée par l’emplacement du shader. La nouvelle politique de fond d’écran est prête à être enregistrée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.shader_is_rejected_2",
        translated_text: "Le shader est rejeté",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.policy_deleted_for",
        translated_text: "Politique {value1} supprimée pour {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.policy_was_deleted_but_the_shader_file_could_not_be_deleted",
        translated_text: "La politique {value1} a été supprimée, mais le fichier shader n’a pas pu être supprimé : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.shader_and_associated_policy_deleted",
        translated_text: "Shader {value1} et politique {value2} associée supprimés : {value3}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "target.unassigned",
        translated_text: "Non attribué",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.bulk_edit_no_changes_protected_default_one",
        translated_text: "Aucune politique modifiée. La cible de politique ne peut pas être modifiée pour {value1} politique par défaut protégée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.bulk_edit_no_changes_protected_default_many",
        translated_text: "Aucune politique modifiée. La cible de politique ne peut pas être modifiée pour {value1} politiques par défaut protégées.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.bulk_edit_complete_protected_default_one",
        translated_text: "Modification groupée terminée : {value1} politiques mises à jour. La cible de politique a été conservée pour {value2} politique par défaut protégée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.bulk_edit_complete_protected_default_many",
        translated_text: "Modification groupée terminée : {value1} politiques mises à jour. La cible de politique a été conservée pour {value2} politiques par défaut protégées.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.select_policy_target_before_saving",
        translated_text: "Sélectionnez une cible de politique avant l’enregistrement",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.delete_shader_from_policy_context_menu",
        translated_text: "Supprimer le shader est disponible depuis le menu contextuel de la ligne de politique.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.select_target_for_selected_unassigned_policies",
        translated_text: "Sélectionnez Économiseur d’écran ou Fond d’écran comme cible de politique pour les politiques non attribuées sélectionnées.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "edit.required",
        translated_text: "Requis",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "assign_shader_policies.all_screensavers",
        translated_text: "Tous les Économiseur d’écrans",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "assign_shader_policies.all_wallpapers",
        translated_text: "Tous les Fond d’écrans",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "assign_shader_policies.screensavers_and_wallpapers",
        translated_text: "Économiseur d’écrans + Fond d’écrans",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "assign_shader_policies.all_unassigned",
        translated_text: "Tous non attribués",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "assign_shader_policies.assignment_title",
        translated_text: "Attribuer des politiques aux nouveaux shaders",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "assign_shader_policies.assignment_message.singular",
        translated_text: "Screenshaver a trouvé {count} shader dans le dossier des shaders gérés qui n’a pas encore de politique.\\n\\nChoisissez comment créer une politique pour ce shader.\\n\\nLes politiques non attribuées ne peuvent pas être rendues tant que leur cible de politique n’est pas définie sur Économiseur d’écran ou Fond d’écran.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "assign_shader_policies.assignment_message.plural",
        translated_text: "Screenshaver a trouvé {count} shaders dans le dossier des shaders gérés qui n’ont pas encore de politique.\\n\\nChoisissez comment créer les politiques pour ces shaders.\\n\\nLes politiques non attribuées ne peuvent pas être rendues tant que leur cible de politique n’est pas définie sur Économiseur d’écran ou Fond d’écran.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "assign_shader_policies.completion_title",
        translated_text: "Politiques de shader créées",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "assign_shader_policies.completion_message.one_policy_one_shader",
        translated_text: "Screenshaver a créé {policy_count} politique de shader pour {shader_count} shader avec \"{assignment}\".\\n\\nVous pouvez consulter ou modifier les politiques de shader à tout moment en exécutant :\\n\\nscreenshaver --control\\n\\nCette commande ouvre le Centre de contrôle Screenshaver.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "assign_shader_policies.completion_message.one_policy_many_shaders",
        translated_text: "Screenshaver a créé {policy_count} politique de shader pour {shader_count} shaders avec \"{assignment}\".\\n\\nVous pouvez consulter ou modifier les politiques de shader à tout moment en exécutant :\\n\\nscreenshaver --control\\n\\nCette commande ouvre le Centre de contrôle Screenshaver.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "assign_shader_policies.completion_message.many_policies_one_shader",
        translated_text: "Screenshaver a créé {policy_count} politiques de shader pour {shader_count} shader avec \"{assignment}\".\\n\\nVous pouvez consulter ou modifier les politiques de shader à tout moment en exécutant :\\n\\nscreenshaver --control\\n\\nCette commande ouvre le Centre de contrôle Screenshaver.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "assign_shader_policies.completion_message.many_policies_many_shaders",
        translated_text: "Screenshaver a créé {policy_count} politiques de shader pour {shader_count} shaders avec \"{assignment}\".\\n\\nVous pouvez consulter ou modifier les politiques de shader à tout moment en exécutant :\\n\\nscreenshaver --control\\n\\nCette commande ouvre le Centre de contrôle Screenshaver.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "assign_shader_policies.error.prepare_query",
        translated_text: "Impossible de préparer la requête des shaders sans politique : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "assign_shader_policies.error.query_shaders",
        translated_text: "Impossible d’interroger les shaders sans politique : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "assign_shader_policies.error.decode_row",
        translated_text: "Impossible de décoder la ligne du shader sans politique : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "assign_shader_policies.error.display_dialog",
        translated_text: "Impossible d’afficher la boîte de dialogue d’attribution de la nouvelle politique : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "assign_shader_policies.error.unknown_button",
        translated_text: "La boîte de dialogue d’attribution de la nouvelle politique a renvoyé un identifiant de bouton inconnu {button_id}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "assign_shader_policies.error.begin_transaction",
        translated_text: "Impossible de démarrer la transaction d’attribution de la nouvelle politique : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "assign_shader_policies.error.recheck_policies",
        translated_text: "Impossible de revérifier les politiques pour '{filename}' : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "assign_shader_policies.error.commit_transaction",
        translated_text: "Impossible de valider la transaction d’attribution de la nouvelle politique : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "assign_shader_policies.error.create_policy",
        translated_text: "Impossible de créer une politique {target} pour '{filename}' : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "assign_shader_policies.error.validate_policy_name",
        translated_text: "Impossible de valider le nom de politique généré '{policy_name}' pour la cible {target} : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "assign_shader_policies.error.generate_policy_name",
        translated_text: "Impossible de générer un nom de politique suggéré disponible pour « {filename} » dans la cible {target}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "authentication.error.initialize_pam",
        translated_text: "Impossible d’initialiser le service PAM « {service} » : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "authentication.error.configure_failure_delay",
        translated_text: "Impossible de configurer le délai d’échec PAM : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "authentication.error.pam_authentication",
        translated_text: "Erreur d’authentification PAM : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "compile_shader.kind.vertex",
        translated_text: "Sommet",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "compile_shader.kind.fragment",
        translated_text: "Fragment",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "common.unknown",
        translated_text: "Inconnu",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "compile_shader.kind.unknown",
        translated_text: "Inconnu",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "compile_shader.error.create_shader_object",
        translated_text: "Impossible de créer l’objet shader OpenGL {kind}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "compile_shader.error.interior_null",
        translated_text: "La source du shader {kind} contenait un octet nul interne",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "compile_shader.error.create_program_object",
        translated_text: "Impossible de créer l’objet programme de shader OpenGL",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "compile_shader.error.link_no_diagnostic",
        translated_text: "Échec de l’édition de liens du programme de shader sans diagnostic OpenGL",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "compile_shader.error.link_failed",
        translated_text: "Échec de l’édition de liens du programme de shader :\\n{error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "compile_shader.error.compile_no_diagnostic",
        translated_text: "Échec de la compilation du shader {kind} sans diagnostic OpenGL",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "compile_shader.error.compile_failed",
        translated_text: "{kind} shader compilation échoué:\\n{error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.query",
        translated_text: "Requête",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.clear",
        translated_text: "Effacer",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.boolean.true",
        translated_text: "Vrai",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.boolean.false",
        translated_text: "Faux",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.status.ok",
        translated_text: "OK",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.status.rejected",
        translated_text: "Rejeté",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.status.compile_error",
        translated_text: "Erreur de compilation",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.status.missing",
        translated_text: "Manquant",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.status.unreadable",
        translated_text: "Illisible",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "preview.target_not_found",
        translated_text: "Fichier ou répertoire de shader introuvable : {path}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "tray.tooltip.waiting_for_idle",
        translated_text: "En attente de l’inactivité…",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "tray.status.enabled",
        translated_text: "Activé",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "tray.status.disabled",
        translated_text: "Désactivé",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "tray.status.starting",
        translated_text: "Démarrage…",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "tray.menu.screensaver_status",
        translated_text: "Économiseur d’écran: {status}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "tray.menu.wallpaper",
        translated_text: "Fond d’écran:",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "tray.menu.edit",
        translated_text: "Modifier",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "tray.menu.restart",
        translated_text: "Redémarrer",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "tray.menu.stop",
        translated_text: "Arrêter",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.runtime.disabled_by_config",
        translated_text: "[WALLPAPER] Fond d’écran désactivé par screenshaver.toml",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.runtime.starting",
        translated_text: "[WALLPAPER] Démarrage de l’exécution automatique du fond d’écran",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.runtime.stopped_cleanly",
        translated_text: "[WALLPAPER] Exécution du fond d’écran arrêtée proprement",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.runtime.attempt_failed",
        translated_text: "[WALLPAPER] Échec de la tentative d’exécution {attempt}/{maximum} : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.runtime.attempt_panicked",
        translated_text: "[WALLPAPER] La tentative d’exécution {attempt}/{maximum} a paniqué",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.runtime.disabled_after_failures",
        translated_text: "[WALLPAPER] Fond d’écran désactivé pour la session actuelle après des échecs répétés",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.runtime.thread_create_failed",
        translated_text: "[WALLPAPER] Impossible de créer le fil du fond d’écran : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.runtime.supervisor_panicked_shutdown",
        translated_text: "[WALLPAPER] Le fil superviseur du fond d’écran a paniqué pendant l’arrêt.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.test.window_title",
        translated_text: "Screenshaver Fond d’écran Rendu Test",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.test.error.sdl_initialization",
        translated_text: "Échec de l’initialisation SDL : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.test.error.sdl_video_initialization",
        translated_text: "Échec de l’initialisation vidéo SDL : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.test.error.create_window",
        translated_text: "Impossible de créer la fenêtre de test du rendu de fond d’écran : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.test.error.create_opengl_context",
        translated_text: "Impossible de créer le contexte OpenGL du test de fond d’écran : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.test.error.create_event_pump",
        translated_text: "Impossible de créer la pompe d’événements du test de fond d’écran : {error}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "xfce.lock.desktop.comment",
        translated_text: "Présentation du shader Screenshaver pour l’écran de verrouillage Xfce",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.notification.title",
        translated_text: "Screenshaver Fond d’écran",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.notification.policy",
        translated_text: "Politique: {policy} ({speed})",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.notification.performance_warning",
        translated_text: "Avertissement de performances",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.notification.performance_critical",
        translated_text: "Performances CRITIQUES",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.notification.fps",
        translated_text: "FPS: {fps}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.notification.texture",
        translated_text: "Texture: {texture}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.notification.palette",
        translated_text: "Palette: {palette}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.cli.version",
        translated_text: "Screenshaver {version}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.cli.configuration_heading",
        translated_text: "Configuration du mode Fond d’écran :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.cli.shader_mode",
        translated_text: "    Mode de shader : {mode}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.cli.monitor_mode",
        translated_text: "    Mode de moniteur : {mode}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.cli.animation_speed",
        translated_text: "    Vitesse d’animation globale : {speed}x",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.cli.notifications",
        translated_text: "    Notifications : {state}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.cli.directory",
        translated_text: "    Fond d’écran répertoire: {path}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.cli.eligible_count",
        translated_text: "Shaders de fond d’écran éligibles : {count}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.cli.no_eligible_shaders",
        translated_text: "    Aucun shader présent avec une politique Fond d’écran n’a été trouvé.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.cli.not_started",
        translated_text: "Le rendu du fond d’écran n’a pas démarré.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.cli.rotation_disabled_single_shader",
        translated_text: "Rotation du fond d’écran désactivée : un seul shader admissible est disponible.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "wallpaper.cli.lyrics_enabled_windowpaper",
        translated_text: "Gestionnaire de paroles synchronisées : activé pour Windowpaper",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "texture.family.marble",
        translated_text: "Marbre",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "texture.family.clouds",
        translated_text: "Nuages",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "texture.family.cells",
        translated_text: "Cellules",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "texture.family.mesh",
        translated_text: "Treillis",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "texture.family.radial",
        translated_text: "Radial",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "texture.family.noise",
        translated_text: "Bruit",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "texture.family.bricks",
        translated_text: "Briques",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "texture.family.hexagons",
        translated_text: "Hexagones",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "texture.family.facets",
        translated_text: "Facettes",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "texture.family.skulls",
        translated_text: "Crânes",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "texture.family.scales",
        translated_text: "Échelles",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "texture.family.eyes",
        translated_text: "Yeux",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "overlay.collect_more_shaders",
        translated_text: "Trouvez davantage de shaders sur https://editor.isf.video/shaders et https://shadertoy.com/browse",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.validation.first_item_blank",
        translated_text: "Le premier élément QBE est vide.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.validation.first_operator_blank",
        translated_text: "Le premier opérateur QBE est vide.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.validation.first_value_blank",
        translated_text: "La première valeur QBE est vide.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.validation.first_operator_invalid",
        translated_text: "Le premier opérateur QBE n’est pas valide pour l’élément sélectionné.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.validation.second_item_blank",
        translated_text: "Le deuxième élément QBE est vide.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.validation.second_operator_blank",
        translated_text: "Le deuxième opérateur QBE est vide.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.validation.second_value_blank",
        translated_text: "La deuxième valeur QBE est vide.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.validation.second_operator_invalid",
        translated_text: "Le deuxième opérateur QBE n’est pas valide pour l’élément sélectionné.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.error.invalid_boolean",
        translated_text: "La valeur booléenne QBE '{value}' doit être vraie ou fausse.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.error.invalid_integer",
        translated_text: "La valeur QBE '{value}' n’est pas un entier valide.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.error.invalid_decimal",
        translated_text: "La valeur QBE « {value} » n’est pas un nombre décimal valide.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.error.invalid_date",
        translated_text: "La date QBE « {value} » doit être une date valide au format MM/DD/YYYY.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.error.invalid_shader_type",
        translated_text: "Type de shader inconnu « {value} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.error.invalid_policy_target",
        translated_text: "Cible de politique inconnue '{value}'.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.error.invalid_status",
        translated_text: "État du shader inconnu '{value}'.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.error.invalid_anti_aliasing",
        translated_text: "Valeur d’anticrénelage inconnue « {value} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.error.invalid_dithering",
        translated_text: "Valeur de tramage inconnue « {value} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.error.invalid_color_precision",
        translated_text: "Valeur de précision des couleurs inconnue '{value}'.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.error.invalid_audiovisual_effect",
        translated_text: "Valeur d’effet audiovisuel inconnue « {value} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.shader_file_cannot_be_accessed",
        translated_text: "Impossible d’accéder au fichier shader :\\n{value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.shader_validation_failed",
        translated_text: "Échec de la validation du shader.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.shader_cannot_be_rendered_status_reason_see_screenshaver_log_for",
        translated_text: "Le shader ne peut pas être rendu.\\nÉtat : {value1}\\nRaison : {value2}\\nConsultez screenshaver.log pour plus de détails.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.shader_validation_state_is_unavailable",
        translated_text: "L’état de validation du shader n’est pas disponible.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.unassigned_policy_this_shader_cannot_be_rendered_until_its_policy",
        translated_text: "Politique non attribuée — ce shader ne peut pas être rendu tant que sa cible de politique n’est pas définie sur Économiseur d’écran ou Fond d’écran.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.shader_is_accessible_and_validated",
        translated_text: "Le shader est accessible et validé :\\n{value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.unable_to_create_egui_opengl_painter",
        translated_text: "Impossible de créer le moteur de rendu egui OpenGL : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.unable_to_decode_embedded_control_center_branding_image",
        translated_text: "Impossible de décoder l’image de marque intégrée du Centre de contrôle : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.embedded_control_center_branding_image_has_invalid_dimensions",
        translated_text: "L’image de marque intégrée du Centre de contrôle présente des dimensions non valides.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.screenshaver_control_center_esc_or_q_to_exit",
        translated_text: "Centre de contrôle Screenshaver (Échap ou Q pour quitter)",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.bulk_edit_mode_active_click_cancel_to_return_to_single",
        translated_text: "Mode de modification en masse actif — cliquez sur Annuler pour revenir au mode de modification individuelle.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.select",
        translated_text: "Sélectionner…",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.policy_target",
        translated_text: "Cible de la politique :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.no_change",
        translated_text: "Aucune modification",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.leave_policy_target_unchanged_for_every_checked_policy",
        translated_text: "Laisser la cible de politique inchangée pour chaque politique cochée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.load_or_create_the_policy_used_for_screensaver_rendering",
        translated_text: "Charger ou créer la politique utilisée pour le rendu de l’économiseur d’écran.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.this_editing_session_was_opened_for_the_active_wallpaper_only",
        translated_text: "Cette session de modification a été ouverte uniquement pour le fond d’écran actif. Seule la politique Fond d’écran peut être modifiée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.this_shader_is_unavailable_for_screensaver_use_because_it_does",
        translated_text: "Ce shader n’est pas disponible pour l’économiseur d’écran car il n’existe pas dans le dossier des économiseurs d’écran.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.save_or_cancel_the_current_changes_before_switching_policy_targets",
        translated_text: "Enregistrez ou annulez les modifications en cours avant de changer de cible de politique.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.load_or_create_the_policy_used_for_wallpaper_rendering",
        translated_text: "Charger ou créer la politique utilisée pour le rendu du fond d’écran.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.this_editing_session_was_opened_for_the_active_screensaver_only",
        translated_text: "Cette session de modification a été ouverte uniquement pour l’économiseur d’écran actif. Seule la politique Économiseur d’écran peut être modifiée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.this_shader_is_unavailable_for_wallpaper_use_because_it_does",
        translated_text: "Ce shader n’est pas disponible pour le fond d’écran car il n’existe pas dans le dossier des fonds d’écran.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.keep_this_policy_and_all_of_its_settings_but_exclude",
        translated_text: "Conserver cette politique et tous ses paramètres, mais l’exclure du rendu de l’économiseur d’écran et du fond d’écran jusqu’à sa réattribution.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.no_shader_loaded",
        translated_text: "Aucun shader chargé",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.policy_name",
        translated_text: "Nom de la politique :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.filename",
        translated_text: "Nom du fichier :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.folder",
        translated_text: "Dossier :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.type",
        translated_text: "Type :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.save_policy",
        translated_text: "Enregistrer la politique",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.save_the_current_per_shader_policy",
        translated_text: "Enregistrer la politique actuelle de ce shader.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.saving_policy",
        translated_text: "Enregistrement de la politique…",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.discard_changes_made_during_this_editor_session",
        translated_text: "Abandonner les modifications effectuées pendant cette session de modification.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.bulk_edit_mode_canceled",
        translated_text: "Mode de modification en masse annulé",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.changes_canceled",
        translated_text: "Modifications annulées",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.policy",
        translated_text: "Politique : --",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.policy_modified",
        translated_text: "Politique : modifiée",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.policy_unchanged",
        translated_text: "Politique : inchangée",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.config_modified",
        translated_text: "Configuration : modifiée",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.config_unchanged",
        translated_text: "Configuration : inchangée",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.loaded_and_rendering",
        translated_text: "chargé et en cours de rendu",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.clear_all_policy_selections",
        translated_text: "Effacer toutes les sélections de politiques",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.policy_name_2",
        translated_text: "Nom de la politique",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.policy_type",
        translated_text: "Type de politique",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.no_shader_policies_are_currently_defined",
        translated_text: "Aucune politique de shader n’est actuellement définie.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.include_this_policy_in_bulk_edit_mode",
        translated_text: "Inclure cette politique dans le mode de modification en masse",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.policy_name_shader_path_shader_added_policy_created_policy_modified",
        translated_text: "Nom de la politique : {value1}\\nShader : {value2}\\nChemin : {value3}\\nShader ajouté : {value4}\\nPolitique créée : {value5}\\nPolitique modifiée : {value6}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.edit_policy",
        translated_text: "Modifier la politique…",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.clone_policy",
        translated_text: "Cloner la politique…",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.rename_policy",
        translated_text: "Renommer la politique…",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.add_to_playlist",
        translated_text: "Ajouter à une liste de lecture…",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.refresh_shader",
        translated_text: "Actualiser le shader",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.policy_query_cleared_displaying_all_policies",
        translated_text: "Requête de politiques effacée — affichage des {value1} politiques.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.policy_query_returned_policies",
        translated_text: "La requête de politiques a renvoyé {value1} / {value2} politiques.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.policy_query_failed",
        translated_text: "Échec de la requête de politiques : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.usable_shader_selected",
        translated_text: "{value1} shader{value2} utilisable sélectionné.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.texture_enabled_shader_detected",
        translated_text: "{value1} shader{value2} compatible avec les textures détecté.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.managed_targets_screensaver_wallpaper",
        translated_text: "Cibles gérées : {value1} Économiseur d’écran, {value2} Fond d’écran.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.external_shaders_requiring_a_target",
        translated_text: "Shaders externes nécessitant une cible : {value1}.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.selected_shader_could_not_be_analyzed_and_will_not_be",
        translated_text: "{value1} shader{value2} sélectionné n’a pas pu être analysé et ne sera pas inclus.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.policy_target_for_all_external_shaders",
        translated_text: "Cible de politique pour tous les shaders externes :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.confirm_bulk_policy_changes",
        translated_text: "Confirmer les modifications de politiques en masse",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.changes_will_be_applied_to_policies_click_ok_to_continue",
        translated_text: "Les modifications seront appliquées à {value1} politiques. Cliquez sur OK pour continuer ou sur Annuler pour abandonner.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.policies_were_selected_will_be_updated_and_will_be_skipped",
        translated_text: "{value1} politiques ont été sélectionnées. {value2} seront mises à jour et {value3} seront ignorées car le fichier shader n’est pas disponible.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.excluded_policy",
        translated_text: "Politique exclue :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.excluded_policies",
        translated_text: "Politiques exclues :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.texture_and_palette_settings_will_apply_to_texture_enabled_shader",
        translated_text: "Les paramètres Texture et Palette s’appliqueront à {value1} shader{value2} compatible avec les textures.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.no_selected_policies_are_eligible_for_bulk_edit_because_their",
        translated_text: "Aucune politique sélectionnée n’est admissible à la modification en masse car les fichiers shader correspondants ne sont pas disponibles.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.unsaved_changes",
        translated_text: "Modifications non enregistrées",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.the_screenshaver_control_center_has_unsaved_changes",
        translated_text: "Le Centre de contrôle Screenshaver contient des modifications non enregistrées.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.would_you_like_to_save_those_changes_before_exiting",
        translated_text: "Souhaitez-vous enregistrer ces modifications avant de quitter ?",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.save_and_exit",
        translated_text: "Enregistrer et quitter",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.exit_without_saving",
        translated_text: "Quitter sans enregistrer",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.rename_policy_2",
        translated_text: "Renommer la politique",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.change_the_user_facing_policy_name_the_shader_and_policy",
        translated_text: "Modifier le nom de politique visible par l’utilisateur. Les paramètres du shader et de la politique restent inchangés.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.policy_name_must_contain_between_1_and_128_characters_found",
        translated_text: "Le nom de la politique doit contenir entre 1 et 128 caractères ; {value1} détectés.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.clone_policy_2",
        translated_text: "Cloner la politique",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.move_shader",
        translated_text: "Déplacer le shader ?",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.move_this_shader_to",
        translated_text: "Déplacer ce shader vers {value1} :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.the_existing_policy_will_be_changed_to_a_policy_all",
        translated_text: "La politique {value1} existante sera transformée en politique {value2}. Tous les paramètres de la politique seront conservés.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.the_existing_policy_will_be_retained_and_its_path_will",
        translated_text: "La politique {value1} existante sera conservée et son chemin sera mis à jour automatiquement.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.the_shader_file_will_not_be_deleted",
        translated_text: "Le fichier shader ne sera pas supprimé.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.permanently_delete_this_shader",
        translated_text: "Supprimer définitivement ce shader {value1} :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.the_associated_policy_will_also_be_deleted",
        translated_text: "La politique {value1} associée sera également supprimée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.any_wallpaper_shader_or_wallpaper_policy_with_the_same_filename",
        translated_text: "Tout shader ou toute politique de fond d’écran portant le même nom de fichier restera inchangé.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.any_screensaver_shader_or_screensaver_policy_with_the_same_filename",
        translated_text: "Tout shader ou toute politique d’économiseur d’écran portant le même nom de fichier restera inchangé.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.the_shader_file_will_not_be_changed",
        translated_text: "Le fichier shader ne sera pas modifié.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.unable_to_load_playlists",
        translated_text: "Impossible de charger les listes de lecture : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.unable_to_load_the_playlist_inventory",
        translated_text: "Impossible de charger l’inventaire des listes de lecture.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.unable_to_load_playlist_policies",
        translated_text: "Impossible de charger les politiques de la liste de lecture : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.playlist_name",
        translated_text: "Nom de la liste de lecture",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.no_playlists_have_been_created",
        translated_text: "Aucune liste de lecture n’a été créée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.playlist_created_playlist_modified",
        translated_text: "Liste de lecture créée : {value1}\\nListe de lecture modifiée : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.selected_playlist",
        translated_text: "Liste de lecture « {value1} » sélectionnée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.new_playlist",
        translated_text: "Nouvelle liste de lecture",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.edit_playlist_info",
        translated_text: "Modifier les informations de la liste de lecture",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.selected_playlist_2",
        translated_text: "Liste de lecture sélectionnée",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.this_playlist_contains_no_policies",
        translated_text: "Cette liste de lecture ne contient aucune politique.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.policy_target_2",
        translated_text: "Cible de la politique",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.sorted_playlist_by_compound_criteria",
        translated_text: "Liste de lecture « {value1} » triée selon des critères composés.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.sorted_playlist_by",
        translated_text: "Liste de lecture « {value1} » triée par {value2}.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.unable_to_sort_playlist",
        translated_text: "Impossible de trier la liste de lecture : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.click_to_sort_and_persist_playlist_order_shift_click_adds",
        translated_text: "Cliquez pour trier et enregistrer l’ordre de la liste de lecture. Maj-clic ajoute ou modifie une clé de tri secondaire.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.policy_name_shader_path",
        translated_text: "Nom de la politique : {value1}\\nShader : {value2}\\nChemin : {value3}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.selected_policy_in_playlist",
        translated_text: "Politique « {value1} » sélectionnée dans la liste de lecture « {value2} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.remove_policy",
        translated_text: "Retirer la politique",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.removed_policy_from_playlist",
        translated_text: "Politique « {value1} » retirée de la liste de lecture.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.policy_is_no_longer_a_member_of_this_playlist",
        translated_text: "La politique ne fait plus partie de cette liste de lecture.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.unable_to_remove_policy_from_playlist",
        translated_text: "Impossible de retirer la politique de la liste de lecture : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.add_policy",
        translated_text: "Ajouter une politique",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.removed_policy_from_playlist_2",
        translated_text: "Politique « {value1} » retirée de la liste de lecture « {value2} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.policy_was_not_present_in_playlist",
        translated_text: "La politique « {value1} » n’était pas présente dans la liste de lecture « {value2} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.move_up",
        translated_text: "Monter",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.moved_policy_up",
        translated_text: "Politique « {value1} » déplacée vers le haut.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.unable_to_move_policy",
        translated_text: "Impossible de déplacer la politique : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.move_down",
        translated_text: "Descendre",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.moved_policy_down",
        translated_text: "Politique « {value1} » déplacée vers le bas.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.add_to_playlist_2",
        translated_text: "Ajouter à une liste de lecture",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.add_policy_to_an_existing_playlist",
        translated_text: "Ajouter la politique « {value1} » à une liste de lecture existante.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.this_policy_is_already_a_member_of_every_playlist",
        translated_text: "Cette politique fait déjà partie de toutes les listes de lecture.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.selected_playlist_3",
        translated_text: "liste de lecture sélectionnée",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.added_policy_to_playlist",
        translated_text: "Politique « {value1} » ajoutée à la liste de lecture « {value2} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.policy_is_already_a_member_of_playlist",
        translated_text: "La politique « {value1} » fait déjà partie de la liste de lecture « {value2} ».",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.unable_to_add_policy_to_playlist",
        translated_text: "Impossible d’ajouter la politique à la liste de lecture : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.no_policies_available",
        translated_text: "Aucune politique disponible",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.add_policy_to_playlist",
        translated_text: "Ajouter une politique à la liste de lecture",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.all_available_policies_are_already_members_of_this_playlist",
        translated_text: "Toutes les politiques disponibles font déjà partie de cette liste de lecture.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.selected_policy",
        translated_text: "Politique sélectionnée",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.added_policy_to_playlist_2",
        translated_text: "Politique « {value1} » ajoutée à la liste de lecture.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.that_policy_is_already_a_member_of_the_playlist",
        translated_text: "Cette politique fait déjà partie de la liste de lecture.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.playlist_name_2",
        translated_text: "Nom de la liste de lecture :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.description_optional",
        translated_text: "Description (facultative) :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.created_playlist",
        translated_text: "Liste de lecture « {value1} » créée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.edit_the_playlist_name_and_description_playlist_membership_and_policy",
        translated_text: "Modifier le nom et la description de la liste de lecture. L’appartenance à la liste de lecture et l’ordre des politiques restent inchangés.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.updated_playlist",
        translated_text: "Liste de lecture « {value1} » mise à jour.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.shader_policies_and_shader_files_will_not_be_deleted",
        translated_text: "Les politiques de shader et les fichiers shader ne seront pas supprimés.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.deleted_playlist",
        translated_text: "Liste de lecture « {value1} » supprimée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.unable_to_delete_playlist",
        translated_text: "Impossible de supprimer la liste de lecture « {value1} » : {value2}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.render_controls",
        translated_text: "Commandes de rendu",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.fps_max",
        translated_text: "FPS (max.)",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.fps",
        translated_text: "{value1} FPS",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.click_to_include_fps_in_bulk_edit",
        translated_text: "Cliquer pour inclure les FPS dans la modification en masse",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.click_to_exclude_fps_from_bulk_edit",
        translated_text: "Cliquer pour exclure les FPS de la modification en masse",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.click_the_numeric_fps_value_to_enable_this_slider_for",
        translated_text: "Cliquer sur la valeur numérique des FPS pour activer ce curseur pour la modification en masse.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.set_the_maximum_rendering_frame_rate_hold_shift_for_fine",
        translated_text: "Définir la fréquence d’images maximale du rendu. Maintenez Maj pour un réglage précis.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.animation_speed",
        translated_text: "Vitesse d’animation",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.click_to_include_animation_speed_in_bulk_edit",
        translated_text: "Cliquer pour inclure la vitesse d’animation dans la modification en masse",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.click_to_exclude_animation_speed_from_bulk_edit",
        translated_text: "Cliquer pour exclure la vitesse d’animation de la modification en masse",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.click_the_numeric_animation_speed_value_to_enable_this_slider",
        translated_text: "Cliquer sur la valeur numérique de la vitesse d’animation pour activer ce curseur pour la modification en masse.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.adjust_animation_speed_on_a_logarithmic_scale_the_slider_midpoint",
        translated_text: "Régler la vitesse d’animation sur une échelle non linéaire symétrique. Faites glisser vers la gauche pour inverser l’animation, au centre sur 0x pour la mettre en pause ou vers la droite pour avancer. Maintenez Maj pour un réglage précis.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.starting_offset",
        translated_text: "Décalage initial",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.starting_offset_is_not_available_during_bulk_edit",
        translated_text: "Le décalage initial n’est pas disponible pendant la modification en masse.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.drag_to_choose_where_the_shader_begins_hold_shift_while",
        translated_text: "Faites glisser pour choisir le point de départ du shader. Maintenez Maj pendant le déplacement pour un réglage 10 fois plus précis.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.render_scale",
        translated_text: "Échelle de rendu",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.click_to_include_render_scale_in_bulk_edit",
        translated_text: "Cliquer pour inclure l’échelle de rendu dans la modification en masse",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.click_to_exclude_render_scale_from_bulk_edit",
        translated_text: "Cliquer pour exclure l’échelle de rendu de la modification en masse",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.click_the_numeric_render_scale_value_to_enable_this_slider",
        translated_text: "Cliquer sur la valeur numérique de l’échelle de rendu pour activer ce curseur pour la modification en masse.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.change_internal_rendering_resolution_lower_values_improve_performance_higher_values",
        translated_text: "Modifier la résolution de rendu interne. Des valeurs plus faibles améliorent les performances ; des valeurs plus élevées améliorent la qualité.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.set_the_number_of_graphical_elements_used_to_generate_the",
        translated_text: "Définir le nombre d’éléments graphiques utilisés pour générer la texture procédurale.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.unable_to_generate_texture_thumbnail",
        translated_text: "Impossible de générer la miniature de la texture : {value1}",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.texture_preview_unavailable",
        translated_text: "Aperçu de la texture indisponible",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.palette_color",
        translated_text: "Couleur de la palette :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.custom_color",
        translated_text: "Couleur personnalisée",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.choose_a_curated_palette_color_the_selected_color_is_written",
        translated_text: "Choisir une couleur de la palette organisée. La couleur sélectionnée est inscrite dans le champ hexadécimal.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.enter_a_palette_color_using_six_digit_hexadecimal_notation_rrggbb",
        translated_text: "Saisir une couleur de palette en notation hexadécimale à six chiffres (#rrggbb).",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.policy_actions",
        translated_text: "Actions de la politique",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.save_the_current_per_shader_policy_after_all_mandatory_information",
        translated_text: "Enregistrer la politique actuelle de ce shader une fois toutes les informations obligatoires fournies.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.permanently_remove_the_shader_file_after_confirmation",
        translated_text: "Supprimer définitivement le fichier shader après confirmation.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.about_shader_policies",
        translated_text: "À propos des politiques de shader",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.a_shader_policy_determines_how_the_selected_shader_is_rendered",
        translated_text: "Une politique de shader détermine comment le shader sélectionné est rendu comme économiseur d’écran ou fond d’écran.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.policy_unsaved_changes",
        translated_text: "Politique : modifications non enregistrées",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.bulk_create.usable_shader_selected_one",
        translated_text: "{value1} shader utilisable sélectionné.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.bulk_create.usable_shaders_selected_many",
        translated_text: "{value1} shaders utilisables sélectionnés.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.bulk_create.texture_shader_detected_one",
        translated_text: "{value1} shader compatible avec les textures détecté.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.bulk_create.texture_shaders_detected_many",
        translated_text: "{value1} shaders compatibles avec les textures détectés.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.bulk_create.rejected_shader_one",
        translated_text: "{value1} shader sélectionné n’a pas pu être analysé et ne sera pas inclus.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.bulk_create.rejected_shaders_many",
        translated_text: "{value1} shaders sélectionnés n’ont pas pu être analysés et ne seront pas inclus.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.bulk_edit.texture_settings_one",
        translated_text: "Les paramètres Texture et Palette s’appliqueront à {value1} shader compatible avec les textures.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.bulk_edit.texture_settings_many",
        translated_text: "Les paramètres Texture et Palette s’appliqueront à {value1} shaders compatibles avec les textures.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.field.policy_name",
        translated_text: "Nom de la politique",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.field.playlist_name",
        translated_text: "Nom de la liste de lecture",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.field.shader_added_date",
        translated_text: "Shader Ajouté Date",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.field.policy_created_date",
        translated_text: "Politique Créé Date",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.field.policy_modified_date",
        translated_text: "Date de modification de la politique",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.field.playlist_created_date",
        translated_text: "Liste de lecture Créé Date",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.field.playlist_modified_date",
        translated_text: "Date de modification de la liste de lecture",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.field.shader_filename",
        translated_text: "Nom du fichier shader",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.field.shader_type",
        translated_text: "Type de shader",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.field.policy_target",
        translated_text: "Cible de la politique",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.field.status",
        translated_text: "État",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.field.texture",
        translated_text: "Texture",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.field.palette",
        translated_text: "Palette",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.field.rendered_fps",
        translated_text: "FPS rendues",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.field.animation_speed",
        translated_text: "Vitesse d’animation",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.field.render_scale",
        translated_text: "Échelle de rendu",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.field.anti_aliasing",
        translated_text: "Anticrénelage",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.field.dithering",
        translated_text: "Tramage",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.field.color_precision",
        translated_text: "Précision des couleurs",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.field.audiovisual_effect",
        translated_text: "Effet audiovisuel",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.field.invert_frequency_mapping",
        translated_text: "Inverser le mappage des fréquences",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.operator.is",
        translated_text: "est",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.operator.eq",
        translated_text: "égal",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.operator.ne",
        translated_text: "différent",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.operator.like",
        translated_text: "comme",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.operator.not_like",
        translated_text: "pas comme",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.operator.lt",
        translated_text: "inférieur à",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.operator.le",
        translated_text: "inférieur ou égal à",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.operator.gt",
        translated_text: "supérieur à",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.operator.ge",
        translated_text: "supérieur ou égal à",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.conditional.and",
        translated_text: "ET",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "qbe.conditional.or",
        translated_text: "OU",
    },

    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.tab.policies",
        translated_text: "Politiques",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.tab.playlists",
        translated_text: "Listes de lecture",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.tab.rendering",
        translated_text: "Rendu",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.tab.textures",
        translated_text: "Textures",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.tab.post_processing",
        translated_text: "Post-traitement",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.tab.configuration",
        translated_text: "Configuration",
    },



    FactoryTranslation {
        locale: "fr-FR",
        key: "common.save",
        translated_text: "Enregistrer",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "common.yes",
        translated_text: "Oui",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "common.add",
        translated_text: "Ajouter",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "common.create",
        translated_text: "Créer",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "common.delete",
        translated_text: "Supprimer",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.delete_policy_menu",
        translated_text: "Supprimer la politique…",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.delete_shader_menu",
        translated_text: "Supprimer le shader…",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.select_policy_target_before_saving_modified_policy",
        translated_text: "Sélectionnez une cible de politique avant d’enregistrer la politique modifiée.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.clone_policy_description",
        translated_text: "Crée une nouvelle politique avec le même shader, la même cible et les mêmes paramètres.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.delete_this_policy",
        translated_text: "Supprimer cette politique {value1} :",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.delete_playlist",
        translated_text: "Supprimer la liste de lecture",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.description",
        translated_text: "Description",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.select_playlist_to_view_policies",
        translated_text: "Sélectionnez une liste de lecture pour afficher ses politiques.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.select_policy_to_add_to_playlist",
        translated_text: "Sélectionnez une politique de shader existante à ajouter à cette liste de lecture.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.policy_label",
        translated_text: "Politique",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.create_playlist_description",
        translated_text: "Créez une liste de lecture pour regrouper des politiques de shader.",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.delete_this_playlist",
        translated_text: "Supprimer cette liste de lecture ?",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "editor.primitives",
        translated_text: "Primitives",
    },
    FactoryTranslation {
        locale: "fr-FR",
        key: "common.no",
        translated_text: "Non",
    },
];
