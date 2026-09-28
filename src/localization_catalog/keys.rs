// Canonical Screenshaver localization keys.
//
// This is the authoritative English fallback catalog.  Locale modules must
// translate these stable keys rather than redefining English text or context.

use super::FactoryTranslationKey;

pub(crate) const TRANSLATION_KEYS: &[FactoryTranslationKey] = &[
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


    FactoryTranslationKey {
        key: "post.tab.visual_quality",
        english_text: "Visual Quality",
        translator_context: "Post-Processing nested-tab label.",
    },

    FactoryTranslationKey {
        key: "post.tab.image_transforms",
        english_text: "Image Transforms",
        translator_context: "Post-Processing nested-tab label.",
    },

    FactoryTranslationKey {
        key: "post.tab.audiovisual",
        english_text: "Audiovisual",
        translator_context: "Post-Processing nested-tab label.",
    },

    FactoryTranslationKey {
        key: "post.tab.audio_motion",
        english_text: "Audio Motion",
        translator_context: "Post-Processing nested-tab label.",
    },

    FactoryTranslationKey {
        key: "post.common.unchanged",
        english_text: "Unchanged",
        translator_context: "Bulk-edit option that leaves a value unchanged.",
    },

    FactoryTranslationKey {
        key: "post.common.enabled",
        english_text: "Enabled",
        translator_context: "Bulk-edit boolean enabled option.",
    },

    FactoryTranslationKey {
        key: "post.common.disabled",
        english_text: "Disabled",
        translator_context: "Bulk-edit boolean disabled option.",
    },

    FactoryTranslationKey {
        key: "post.common.off",
        english_text: "Off",
        translator_context: "Disabled effect option.",
    },

    FactoryTranslationKey {
        key: "post.visual.anti_aliasing",
        english_text: "Anti-Aliasing:",
        translator_context: "Post-processing anti-aliasing label.",
    },

    FactoryTranslationKey {
        key: "post.visual.anti_aliasing_help",
        english_text: "Controls edge smoothing for the rendered shader.",
        translator_context: "Help for anti-aliasing.",
    },

    FactoryTranslationKey {
        key: "post.visual.dithering",
        english_text: "Dithering:",
        translator_context: "Post-processing dithering label.",
    },

    FactoryTranslationKey {
        key: "post.visual.dithering_help",
        english_text: "Controls subtle dithering used to reduce visible color banding.",
        translator_context: "Help for dithering.",
    },

    FactoryTranslationKey {
        key: "post.visual.subtle",
        english_text: "Subtle",
        translator_context: "Subtle dithering option.",
    },

    FactoryTranslationKey {
        key: "post.visual.color_precision",
        english_text: "Color Precision:",
        translator_context: "Post-processing color-precision label.",
    },

    FactoryTranslationKey {
        key: "post.visual.color_precision_help",
        english_text: "Selects the color precision used by post-processing.",
        translator_context: "Help for color precision.",
    },

    FactoryTranslationKey {
        key: "post.visual.automatic",
        english_text: "Automatic",
        translator_context: "Automatic color precision option.",
    },

    FactoryTranslationKey {
        key: "post.visual.standard_precision",
        english_text: "Standard Precision",
        translator_context: "Standard color precision option.",
    },

    FactoryTranslationKey {
        key: "post.visual.high_precision",
        english_text: "High Precision",
        translator_context: "High color precision option.",
    },

    FactoryTranslationKey {
        key: "post.transform.invert_colors",
        english_text: "Invert Colors",
        translator_context: "Invert-colors control.",
    },

    FactoryTranslationKey {
        key: "post.transform.invert_colors_help",
        english_text: "Inverts the final rendered colors.",
        translator_context: "Help for invert colors.",
    },

    FactoryTranslationKey {
        key: "post.transform.flip_horizontal",
        english_text: "Flip Horizontal",
        translator_context: "Horizontal flip control.",
    },

    FactoryTranslationKey {
        key: "post.transform.flip_horizontal_help",
        english_text: "Mirrors the final image horizontally.",
        translator_context: "Help for horizontal flip.",
    },

    FactoryTranslationKey {
        key: "post.transform.flip_vertical",
        english_text: "Flip Vertical",
        translator_context: "Vertical flip control.",
    },

    FactoryTranslationKey {
        key: "post.transform.flip_vertical_help",
        english_text: "Mirrors the final image vertically.",
        translator_context: "Help for vertical flip.",
    },

    FactoryTranslationKey {
        key: "post.transform.hue_rotation",
        english_text: "Hue Rotation:",
        translator_context: "Hue-rotation slider label.",
    },

    FactoryTranslationKey {
        key: "post.transform.hue_rotation_help",
        english_text: "Rotates the displayed shader colors around the hue wheel.",
        translator_context: "Help for hue rotation.",
    },

    FactoryTranslationKey {
        key: "post.audio.effect",
        english_text: "Audiovisual Effect:",
        translator_context: "Audiovisual effect selector label.",
    },

    FactoryTranslationKey {
        key: "post.audio.effect_help",
        english_text: "Selects the audio-driven post-processing effect: Off, Audio Bloom, Spectral Bloom, or experimental Loudness Bloom.",
        translator_context: "Help for audiovisual effect selector.",
    },

    FactoryTranslationKey {
        key: "post.audio.audio_bloom",
        english_text: "Audio Bloom",
        translator_context: "Audio Bloom effect name.",
    },

    FactoryTranslationKey {
        key: "post.audio.spectral_bloom",
        english_text: "Spectral Bloom",
        translator_context: "Spectral Bloom effect name.",
    },

    FactoryTranslationKey {
        key: "post.audio.loudness_bloom",
        english_text: "Loudness Bloom",
        translator_context: "Loudness Bloom effect name.",
    },

    FactoryTranslationKey {
        key: "post.audio.bloom_intensity",
        english_text: "Bloom Intensity:",
        translator_context: "Bloom intensity slider label.",
    },

    FactoryTranslationKey {
        key: "post.audio.bloom_intensity_help",
        english_text: "Controls the strength of the selected Bloom mode.",
        translator_context: "Help for bloom intensity.",
    },

    FactoryTranslationKey {
        key: "post.audio.bloom_saturation",
        english_text: "Bloom Saturation:",
        translator_context: "Bloom saturation slider label.",
    },

    FactoryTranslationKey {
        key: "post.audio.bloom_saturation_help",
        english_text: "Boosts bloom color saturation from the neutral 1.0 level up to 2.0 without changing the displayed shader colors.",
        translator_context: "Help for bloom saturation.",
    },

    FactoryTranslationKey {
        key: "post.audio.bloom_threshold",
        english_text: "Bloom Threshold:",
        translator_context: "Bloom threshold slider label.",
    },

    FactoryTranslationKey {
        key: "post.audio.bloom_threshold_help",
        english_text: "Controls the brightness threshold used to extract bloom.",
        translator_context: "Help for bloom threshold.",
    },

    FactoryTranslationKey {
        key: "post.audio.frequency_rotation",
        english_text: "Frequency Rotation:",
        translator_context: "Frequency rotation slider label.",
    },

    FactoryTranslationKey {
        key: "post.audio.frequency_rotation_help",
        english_text: "Rotates Audio/Spectral frequency-to-color mapping. Disabled for Loudness Bloom.",
        translator_context: "Help for frequency rotation.",
    },

    FactoryTranslationKey {
        key: "post.audio.invert_frequency",
        english_text: "Invert Frequency Mapping",
        translator_context: "Invert-frequency-mapping control.",
    },

    FactoryTranslationKey {
        key: "post.audio.invert_frequency_help",
        english_text: "Reverses Audio/Spectral low-to-high color-frequency mapping. Disabled for Loudness Bloom.",
        translator_context: "Help for inverted frequency mapping.",
    },

    FactoryTranslationKey {
        key: "post.motion.effect",
        english_text: "Audio Motion Effect:",
        translator_context: "Audio Motion effect selector label.",
    },

    FactoryTranslationKey {
        key: "post.motion.effect_help",
        english_text: "Selects an audio-driven full-frame motion effect. Woofer from Hell uses LRCMUX vocal timing for inverse-cone motion. FFT Mirror Warp uses a mirrored 48-channel FFT trace. Polar Propeller uses the same 48-channel FFT response in a rotating radial deformation.",
        translator_context: "Help for Audio Motion selector. Product/effect names and LRCMUX/FFT remain unchanged.",
    },

    FactoryTranslationKey {
        key: "post.motion.woofer",
        english_text: "Woofer from Hell",
        translator_context: "Audio Motion effect name; proper feature name remains unchanged.",
    },

    FactoryTranslationKey {
        key: "post.motion.fft_mirror",
        english_text: "FFT Mirror Warp",
        translator_context: "Audio Motion effect name; proper feature name remains unchanged.",
    },

    FactoryTranslationKey {
        key: "post.motion.polar_propeller",
        english_text: "Polar Propeller",
        translator_context: "Audio Motion effect name; proper feature name remains unchanged.",
    },

    FactoryTranslationKey {
        key: "post.bulk.exclude",
        english_text: "Click to exclude this value from Bulk Edit",
        translator_context: "Tooltip for removing a value from Bulk Edit.",
    },

    FactoryTranslationKey {
        key: "post.bulk.include",
        english_text: "Click to include this value in Bulk Edit",
        translator_context: "Tooltip for adding a value to Bulk Edit.",
    },
];