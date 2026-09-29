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
    FactoryTranslationKey {
        key: "export.window_title",
        english_text: "Export Screenshaver Data",
        translator_context: "Export wizard window title.",
    },
    FactoryTranslationKey {
        key: "export.stage.select_focus",
        english_text: "Select Export Focus",
        translator_context: "Export wizard stage/page label.",
    },
    FactoryTranslationKey {
        key: "export.stage.select_data",
        english_text: "Select Data",
        translator_context: "Export wizard stage label.",
    },
    FactoryTranslationKey {
        key: "export.destination",
        english_text: "Destination",
        translator_context: "Export wizard destination stage/heading.",
    },
    FactoryTranslationKey {
        key: "export.review_confirm",
        english_text: "Review & Confirm",
        translator_context: "Export wizard review stage/heading.",
    },
    FactoryTranslationKey {
        key: "export.results",
        english_text: "Results",
        translator_context: "Export wizard results stage/heading.",
    },
    FactoryTranslationKey {
        key: "export.policies",
        english_text: "Policies",
        translator_context: "Export wizard category label.",
    },
    FactoryTranslationKey {
        key: "export.shaders",
        english_text: "Shaders",
        translator_context: "Export wizard category label.",
    },
    FactoryTranslationKey {
        key: "export.playlists",
        english_text: "Playlists",
        translator_context: "Export wizard category label.",
    },
    FactoryTranslationKey {
        key: "export.select_policies",
        english_text: "Select Policies",
        translator_context: "Export wizard selectable policy category.",
    },
    FactoryTranslationKey {
        key: "export.select_shaders",
        english_text: "Select Shaders",
        translator_context: "Export wizard selectable shader category.",
    },
    FactoryTranslationKey {
        key: "export.select_playlists",
        english_text: "Select Playlists",
        translator_context: "Export wizard selectable playlist category.",
    },
    FactoryTranslationKey {
        key: "export.focus",
        english_text: "Export Focus:",
        translator_context: "Export wizard focus selector label.",
    },
    FactoryTranslationKey {
        key: "export.focus_heading",
        english_text: "Export Focus",
        translator_context: "Export wizard review heading.",
    },
    FactoryTranslationKey {
        key: "export.focus_help",
        english_text: "Choose whether Policies, Shaders, or Playlists will be the selectable focus that drives this export.",
        translator_context: "Help text explaining export focus.",
    },
    FactoryTranslationKey {
        key: "export.destination_help",
        english_text: "Choose the directory where the portable Screenshaver export archive will be created.",
        translator_context: "Tooltip/help for export destination.",
    },
    FactoryTranslationKey {
        key: "export.select_all",
        english_text: "Select All",
        translator_context: "Export wizard selection button.",
    },
    FactoryTranslationKey {
        key: "export.clear_all",
        english_text: "Clear All",
        translator_context: "Export wizard selection button.",
    },
    FactoryTranslationKey {
        key: "export.selected_count",
        english_text: "{selected} of {total} selected",
        translator_context: "Export selection count. Parameters are numeric.",
    },
    FactoryTranslationKey {
        key: "export.policies_included",
        english_text: "Policies Included ({count})",
        translator_context: "Read-only included policy count.",
    },
    FactoryTranslationKey {
        key: "export.shaders_included",
        english_text: "Shaders Included ({count})",
        translator_context: "Read-only included shader count.",
    },
    FactoryTranslationKey {
        key: "export.playlists_included",
        english_text: "Playlists Included ({count})",
        translator_context: "Read-only included playlist count.",
    },
    FactoryTranslationKey {
        key: "export.resolve_failed",
        english_text: "Unable to resolve effective export policies: {error}",
        translator_context: "Export selection error; error is supplied text.",
    },
    FactoryTranslationKey {
        key: "export.policies_resolved",
        english_text: "{count} included policies resolved and validated for portable export.",
        translator_context: "Export selection validation status.",
    },
    FactoryTranslationKey {
        key: "export.select_at_least_one",
        english_text: "Select at least one {type} to continue.",
        translator_context: "Export selection requirement; type is localized category text.",
    },
    FactoryTranslationKey {
        key: "export.destination_folder",
        english_text: "Destination Folder:",
        translator_context: "Export destination folder label.",
    },
    FactoryTranslationKey {
        key: "export.browse",
        english_text: "Browse...",
        translator_context: "Browse for export destination button.",
    },
    FactoryTranslationKey {
        key: "export.filename",
        english_text: "Export Filename:",
        translator_context: "Export filename label.",
    },
    FactoryTranslationKey {
        key: "export.filename_invalid",
        english_text: "Enter a filename without directory separators.",
        translator_context: "Export filename validation message.",
    },
    FactoryTranslationKey {
        key: "export.filename_exists",
        english_text: "That filename already exists. Export will use: {filename}",
        translator_context: "Collision-safe export filename notice.",
    },
    FactoryTranslationKey {
        key: "export.contents",
        english_text: "Export Contents",
        translator_context: "Export review heading.",
    },
    FactoryTranslationKey {
        key: "export.policies_colon",
        english_text: "Policies:",
        translator_context: "Export review/result label.",
    },
    FactoryTranslationKey {
        key: "export.shaders_colon",
        english_text: "Shaders:",
        translator_context: "Export review/result label.",
    },
    FactoryTranslationKey {
        key: "export.playlists_colon",
        english_text: "Playlists:",
        translator_context: "Export review/result label.",
    },
    FactoryTranslationKey {
        key: "export.format",
        english_text: "Export Format",
        translator_context: "Export review heading.",
    },
    FactoryTranslationKey {
        key: "export.review_safety",
        english_text: "No Screenshaver configuration will be changed. Export creates a portable copy of the items shown above.",
        translator_context: "Export review safety statement.",
    },
    FactoryTranslationKey {
        key: "export.export_colon",
        english_text: "Export:",
        translator_context: "Export result label.",
    },
    FactoryTranslationKey {
        key: "export.passed",
        english_text: "PASSED",
        translator_context: "Successful export status.",
    },
    FactoryTranslationKey {
        key: "export.failed",
        english_text: "FAILED",
        translator_context: "Failed export status.",
    },
    FactoryTranslationKey {
        key: "export.archive",
        english_text: "Archive: {path}",
        translator_context: "Successful export archive path; path must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.result_counts",
        english_text: "Policies: {policies}    Shaders: {shaders}    Playlists: {playlists}",
        translator_context: "Successful export counts.",
    },
    FactoryTranslationKey {
        key: "export.no_archive_installed",
        english_text: "No completed export archive was installed.",
        translator_context: "Failed export result note.",
    },
    FactoryTranslationKey {
        key: "export.not_run",
        english_text: "Export has not been run.",
        translator_context: "Export results placeholder.",
    },
    FactoryTranslationKey {
        key: "export.back",
        english_text: "< Back",
        translator_context: "Export wizard navigation button.",
    },
    FactoryTranslationKey {
        key: "export.action",
        english_text: "Export",
        translator_context: "Export wizard action button.",
    },
    FactoryTranslationKey {
        key: "export.next",
        english_text: "Next >",
        translator_context: "Export wizard navigation button.",
    },
    FactoryTranslationKey {
        key: "export.finish",
        english_text: "Finish",
        translator_context: "Export wizard completion button.",
    },
    FactoryTranslationKey {
        key: "import.window_title",
        english_text: "Import Screenshaver Data",
        translator_context: "Import wizard window title.",
    },
    FactoryTranslationKey {
        key: "import.stage.select_archive",
        english_text: "Select Archive",
        translator_context: "Import wizard stage.",
    },
    FactoryTranslationKey {
        key: "import.stage.inspect_archive",
        english_text: "Inspect Archive",
        translator_context: "Import wizard stage.",
    },
    FactoryTranslationKey {
        key: "import.stage.resolve_conflicts",
        english_text: "Resolve Conflicts",
        translator_context: "Import wizard stage.",
    },
    FactoryTranslationKey {
        key: "import.review_confirm",
        english_text: "Review & Confirm",
        translator_context: "Import wizard stage.",
    },
    FactoryTranslationKey {
        key: "import.results",
        english_text: "Results",
        translator_context: "Import wizard stage.",
    },
    FactoryTranslationKey {
        key: "import.select_archive_help",
        english_text: "Select a Screenshaver export archive to inspect. No Screenshaver data will be changed at this stage.",
        translator_context: "Import selection help.",
    },
    FactoryTranslationKey {
        key: "import.archive_colon",
        english_text: "Archive:",
        translator_context: "Import field label.",
    },
    FactoryTranslationKey {
        key: "import.archive_ready",
        english_text: "The archive is ready for read-only inspection.",
        translator_context: "Import status.",
    },
    FactoryTranslationKey {
        key: "import.no_archive_selected",
        english_text: "No archive selected.",
        translator_context: "Import status.",
    },
    FactoryTranslationKey {
        key: "import.inspection_not_run",
        english_text: "Archive inspection has not been run.",
        translator_context: "Import status.",
    },
    FactoryTranslationKey {
        key: "import.archive_inspection_colon",
        english_text: "Archive Inspection:",
        translator_context: "Import heading.",
    },
    FactoryTranslationKey {
        key: "import.pass",
        english_text: "PASS",
        translator_context: "Inspection status.",
    },
    FactoryTranslationKey {
        key: "import.fail",
        english_text: "FAIL",
        translator_context: "Inspection status.",
    },
    FactoryTranslationKey {
        key: "import.export_format_colon",
        english_text: "Export Format:",
        translator_context: "Inspection label.",
    },
    FactoryTranslationKey {
        key: "import.source_screenshaver_colon",
        english_text: "Source Screenshaver:",
        translator_context: "Inspection label.",
    },
    FactoryTranslationKey {
        key: "import.source_db_schema_colon",
        english_text: "Source DB Schema:",
        translator_context: "Inspection label.",
    },
    FactoryTranslationKey {
        key: "import.inspection_read_only",
        english_text: "Inspection is read-only. No changes have been made to Screenshaver.",
        translator_context: "Safety note.",
    },
    FactoryTranslationKey {
        key: "import.conflict.new",
        english_text: "NEW",
        translator_context: "Conflict status.",
    },
    FactoryTranslationKey {
        key: "import.conflict.duplicate",
        english_text: "DUPLICATE",
        translator_context: "Conflict status.",
    },
    FactoryTranslationKey {
        key: "import.conflict.conflict",
        english_text: "CONFLICT",
        translator_context: "Conflict status.",
    },
    FactoryTranslationKey {
        key: "import.conflict_help",
        english_text: "Import is additive and non-destructive. Genuine name conflicts are resolved automatically by renaming the imported object; existing recipient objects are never modified or discarded.",
        translator_context: "Conflict help.",
    },
    FactoryTranslationKey {
        key: "import.conflict_not_run",
        english_text: "Conflict discovery has not been run.",
        translator_context: "Conflict status.",
    },
    FactoryTranslationKey {
        key: "import.conflict_counts",
        english_text: "{new} new, {duplicates} duplicates, {conflicts} conflicts",
        translator_context: "Conflict counts.",
    },
    FactoryTranslationKey {
        key: "import.automatic_rename",
        english_text: "Automatic resolution: Rename → {name}",
        translator_context: "Conflict resolution; name is user data.",
    },
    FactoryTranslationKey {
        key: "import.rename_blocked",
        english_text: "Automatic rename could not be generated; Import is blocked.",
        translator_context: "Conflict error.",
    },
    FactoryTranslationKey {
        key: "import.proposed_dependency_plan",
        english_text: "Proposed Dependency Plan",
        translator_context: "Import heading.",
    },
    FactoryTranslationKey {
        key: "import.dependencies_resolved",
        english_text: "All package objects and dependencies have deterministic destinations.",
        translator_context: "Import status.",
    },
    FactoryTranslationKey {
        key: "import.conflict_read_only",
        english_text: "This report is read-only. No database rows or shader files have been changed.",
        translator_context: "Safety note.",
    },
    FactoryTranslationKey {
        key: "import.keep_existing",
        english_text: "Keep Existing {type} ID {id}",
        translator_context: "Proposed destination.",
    },
    FactoryTranslationKey {
        key: "import.import_new",
        english_text: "Import new {type}",
        translator_context: "Proposed destination.",
    },
    FactoryTranslationKey {
        key: "import.unresolved_conflict",
        english_text: "Unresolved conflict",
        translator_context: "Proposed destination.",
    },
    FactoryTranslationKey {
        key: "import.review_ready",
        english_text: "Screenshaver is ready to apply the validated, dependency-resolved package to this installation.",
        translator_context: "Review text.",
    },
    FactoryTranslationKey {
        key: "import.no_validated_package",
        english_text: "No validated package is available.",
        translator_context: "Review error.",
    },
    FactoryTranslationKey {
        key: "import.conflict_unavailable",
        english_text: "Conflict discovery is unavailable.",
        translator_context: "Review error.",
    },
    FactoryTranslationKey {
        key: "import.before_import",
        english_text: "Before Import",
        translator_context: "Review heading.",
    },
    FactoryTranslationKey {
        key: "import.backup_before_changes",
        english_text: "Screenshaver will create and verify a permanent timestamped backup of screenshaver.db before making persistent changes.",
        translator_context: "Safety text.",
    },
    FactoryTranslationKey {
        key: "import.behavior",
        english_text: "Import behavior",
        translator_context: "Review heading.",
    },
    FactoryTranslationKey {
        key: "import.behavior_detail",
        english_text: "New and automatically renamed shader files will be installed in the managed shader folder, policies will be restored with their exported rendering settings, and playlists will be reconstructed in canonical order. Existing recipient objects are never modified. Truly identical duplicate objects are reused by imported dependencies.",
        translator_context: "Import behavior.",
    },
    FactoryTranslationKey {
        key: "import.toml_unchanged",
        english_text: "screenshaver.toml will not be changed.",
        translator_context: "Safety text.",
    },
    FactoryTranslationKey {
        key: "import.no_result",
        english_text: "No Import result is available.",
        translator_context: "Results status.",
    },
    FactoryTranslationKey {
        key: "import.completed",
        english_text: "Import completed successfully.",
        translator_context: "Results status.",
    },
    FactoryTranslationKey {
        key: "import.failed",
        english_text: "Import failed.",
        translator_context: "Results status.",
    },
    FactoryTranslationKey {
        key: "import.playlist_memberships_colon",
        english_text: "Playlist memberships:",
        translator_context: "Review label.",
    },
    FactoryTranslationKey {
        key: "import.shaders_created_colon",
        english_text: "Shaders created:",
        translator_context: "Results label.",
    },
    FactoryTranslationKey {
        key: "import.policies_created_colon",
        english_text: "Policies created:",
        translator_context: "Results label.",
    },
    FactoryTranslationKey {
        key: "import.playlists_created_colon",
        english_text: "Playlists created:",
        translator_context: "Results label.",
    },
    FactoryTranslationKey {
        key: "import.memberships_created_colon",
        english_text: "Memberships created:",
        translator_context: "Results label.",
    },
    FactoryTranslationKey {
        key: "import.preimport_backup",
        english_text: "Pre-Import database backup:",
        translator_context: "Results label.",
    },
    FactoryTranslationKey {
        key: "import.close",
        english_text: "Close",
        translator_context: "Wizard button.",
    },
    FactoryTranslationKey {
        key: "import.action",
        english_text: "Import",
        translator_context: "Wizard button.",
    },
    FactoryTranslationKey {
        key: "import.diag.shader_identical_same_name",
        english_text: "Identical shader content is already installed under the same filename.",
        translator_context: "Conflict detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.shader_identical_other_name",
        english_text: "Identical shader content is already installed as '{name}'; the existing physical shader will be kept.",
        translator_context: "Conflict detail; name verbatim.",
    },
    FactoryTranslationKey {
        key: "import.diag.shader_name_content_conflict",
        english_text: "The filename already exists in the managed shader inventory, but its content differs from the imported shader.",
        translator_context: "Conflict detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.shader_new",
        english_text: "No installed shader has this content or managed filename.",
        translator_context: "Conflict detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.policy_new",
        english_text: "No {target} policy with this Policy Name exists.",
        translator_context: "Conflict detail; target internal token.",
    },
    FactoryTranslationKey {
        key: "import.diag.policy_duplicate",
        english_text: "An existing policy has the same target, shader content, and rendering configuration.",
        translator_context: "Conflict detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.policy_conflict",
        english_text: "The Policy Name already exists for this target, but its shader or rendering configuration differs.",
        translator_context: "Conflict detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.policy_multiple_match",
        english_text: "More than one receiving policy unexpectedly matches this Policy Name and target.",
        translator_context: "Conflict detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.playlist_new",
        english_text: "No receiving playlist has this name.",
        translator_context: "Conflict detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.playlist_duplicate",
        english_text: "An existing playlist has the same description and resolved policy membership in the same canonical order.",
        translator_context: "Conflict detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.playlist_conflict",
        english_text: "The playlist name already exists, but its description or resolved membership differs.",
        translator_context: "Conflict detail.",
    },
    FactoryTranslationKey {
        key: "import.check.archive_file",
        english_text: "Archive file",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.archive_size",
        english_text: "Archive size",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.zip_container",
        english_text: "ZIP container",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.zip_entry_count",
        english_text: "ZIP entry count",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.zip_entry_access",
        english_text: "ZIP entry access",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.resource_limits",
        english_text: "Archive resource limits",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.member_read",
        english_text: "Archive member read",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.unique_names",
        english_text: "Unique archive names",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.archive_paths",
        english_text: "Archive paths",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.entry_types",
        english_text: "Archive entry types",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.manifest",
        english_text: "Manifest",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.export_schema",
        english_text: "Export schema",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.format_identifier",
        english_text: "Format identifier",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.shader_metadata_structure",
        english_text: "Shader metadata structure",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.shader_archive_paths",
        english_text: "Shader archive paths",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.package_structure",
        english_text: "Package structure",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.missing_content",
        english_text: "Missing archive content",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.unexpected_content",
        english_text: "Unexpected archive content",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.manifest_counts",
        english_text: "Manifest counts",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.validated_package",
        english_text: "Validated package",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.relationships",
        english_text: "Package relationships",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.shader_payloads",
        english_text: "Shader payloads",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.shader_integrity",
        english_text: "Shader integrity",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.shader_encoding",
        english_text: "Shader source encoding",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.shader_payload_inspection",
        english_text: "Shader payload inspection",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.package_sha256",
        english_text: "Package SHA-256",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.diag.not_regular_file",
        english_text: "Selected path is not a regular file.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.resource_limit",
        english_text: "Archive expansion or individual entry exceeds inspection limits.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.no_duplicate_names",
        english_text: "No duplicate ZIP member names detected.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.safe_paths",
        english_text: "No absolute, traversal, NUL, or backslash paths detected.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.safe_entry_types",
        english_text: "No symlink or special-file entries detected.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.manifest_missing",
        english_text: "Required manifest.json is missing.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.counts_match",
        english_text: "Policy, shader, and playlist counts match metadata.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.counts_mismatch",
        english_text: "Manifest counts do not match parsed metadata.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.integrity_record_missing",
        english_text: "Manifest integrity record is missing.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.sha_verified",
        english_text: "SHA-256 verified.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.sha_mismatch",
        english_text: "SHA-256 mismatch or malformed hash.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.relationships_valid",
        english_text: "Policy→shader and playlist→policy references are structurally valid.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.all_shader_hashes_verified",
        english_text: "Every shader SHA-256 value verified.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.all_shader_utf8",
        english_text: "Every shader payload is valid UTF-8 text.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.package_hash_malformed",
        english_text: "Manifest package hash is malformed.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.package_fingerprint_verified",
        english_text: "Canonical package fingerprint verified.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.package_fingerprint_mismatch",
        english_text: "Canonical package fingerprint does not match manifest.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.access_archive",
        english_text: "Unable to access archive: {error}",
        translator_context: "Inspection detail; error verbatim.",
    },
    FactoryTranslationKey {
        key: "import.diag.open_archive",
        english_text: "Unable to open archive: {error}",
        translator_context: "Inspection detail; error verbatim.",
    },
    FactoryTranslationKey {
        key: "import.diag.invalid_zip",
        english_text: "Invalid ZIP archive: {error}",
        translator_context: "Inspection detail; error verbatim.",
    },
    FactoryTranslationKey {
        key: "import.diag.zip_entry_error",
        english_text: "Entry {index}: {error}",
        translator_context: "Inspection detail; error verbatim.",
    },
    FactoryTranslationKey {
        key: "import.diag.member_error",
        english_text: "{name}: {error}",
        translator_context: "Inspection detail; name/error verbatim.",
    },
    FactoryTranslationKey {
        key: "import.diag.manifest_invalid",
        english_text: "manifest.json is invalid: {error}",
        translator_context: "Inspection detail; error verbatim.",
    },
    FactoryTranslationKey {
        key: "import.diag.format_supported",
        english_text: "Screenshaver Export Format {version} is supported.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.diag.shader_path_not_permitted",
        english_text: "'{path}' is not permitted by Export Schema.",
        translator_context: "Inspection detail; path verbatim.",
    },
    FactoryTranslationKey {
        key: "import.diag.shader_files_present",
        english_text: "{count} declared shader files are present.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.playlist_members",
        english_text: "Playlist members",
        translator_context: "Dataset display label.",
    },
    FactoryTranslationKey {
        key: "import.check.metadata_title",
        english_text: "{dataset} metadata",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.metadata_hash_title",
        english_text: "{dataset} metadata hash",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.structure_title",
        english_text: "{dataset} structure",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.required_file_missing",
        english_text: "Required '{file}' is missing.",
        translator_context: "Inspection detail; file is archive member name.",
    },
    FactoryTranslationKey {
        key: "import.check.manifest_integrity_missing",
        english_text: "Manifest integrity record is missing.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.check.sha256_verified",
        english_text: "SHA-256 verified.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.check.sha256_bad",
        english_text: "SHA-256 mismatch or malformed hash.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.check.rows_header_verified",
        english_text: "{rows} rows; schema header verified.",
        translator_context: "Inspection detail; rows is numeric.",
    },
    FactoryTranslationKey {
        key: "import.check.required_members_exact",
        english_text: "All required members are present and no unexpected files were found.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.review_count",
        english_text: "{new} import, {duplicates} identical already present",
        translator_context: "Review count summary.",
    },
    FactoryTranslationKey {
        key: "import.check.package_members",
        english_text: "Package members",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.package_relationships",
        english_text: "Package relationships",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.shader_source_encoding",
        english_text: "Shader source encoding",
        translator_context: "Inspection check title.",
    },
    FactoryTranslationKey {
        key: "import.check.no_duplicate_names",
        english_text: "No duplicate ZIP member names detected.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.check.paths_safe",
        english_text: "No absolute, traversal, NUL, or backslash paths detected.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.check.no_special_entries",
        english_text: "No symlink or special-file entries detected.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.check.manifest_missing",
        english_text: "Required manifest.json is missing.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.check.package_hash_verified",
        english_text: "Canonical package fingerprint verified.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.check.package_hash_mismatch",
        english_text: "Canonical package fingerprint does not match manifest.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.check.relationships_valid",
        english_text: "Policy→shader and playlist→policy references are structurally valid.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.check.all_shader_hashes_verified",
        english_text: "Every shader SHA-256 value verified.",
        translator_context: "Inspection detail.",
    },
    FactoryTranslationKey {
        key: "import.check.all_shader_utf8",
        english_text: "Every shader payload is valid UTF-8 text.",
        translator_context: "Inspection detail.",
    },

    FactoryTranslationKey {
        key: "import.dataset.playlists",
        english_text: "Playlists",
        translator_context: "Import inspection dataset display name.",
    },
    FactoryTranslationKey {
        key: "import.dataset.playlist_memberships",
        english_text: "Playlist memberships",
        translator_context: "Import inspection dataset display name.",
    },


    FactoryTranslationKey {
        key: "import.contents_colon",
        english_text: "Contents:",
        translator_context: "Import wizard package summary label.",
    },
    FactoryTranslationKey {
        key: "import.contents_counts",
        english_text: "{policies} policies, {shaders} shaders, {playlists} playlists, {memberships} memberships",
        translator_context: "Import wizard package contents summary. Parameters are numeric counts.",
    },
    FactoryTranslationKey {
        key: "import.automatic_resolution_rename",
        english_text: "Automatic resolution: Rename → {name}",
        translator_context: "Import conflict-resolution status. {name} is a generated destination name and must remain verbatim.",
    },
    FactoryTranslationKey {
        key: "import.unresolved_rename_dependency_counts",
        english_text: "{renames} conflict rename(s) could not be generated; {dependencies} dependency reference(s) remain unresolved.",
        translator_context: "Import dependency-plan warning. Parameters are numeric counts.",
    },
    FactoryTranslationKey {
        key: "import.blocked_counts",
        english_text: "Import is blocked: {conflicts} conflict(s), {dependencies} unresolved dependency reference(s).",
        translator_context: "Import review warning shown when execution is blocked. Parameters are numeric counts.",
    },
    FactoryTranslationKey {
        key: "import.review_blocked_explanation",
        english_text: "Review is available for inspection, but Import will remain disabled if any deterministic rename or dependency destination cannot be generated.",
        translator_context: "Import review explanation shown while Import remains disabled.",
    },


    FactoryTranslationKey {
        key: "import.diag.archive_bytes_exceed_limit",
        english_text: "{actual} bytes exceeds the {limit} byte inspection limit.",
        translator_context: "Import archive inspection failure. {actual} and {limit} are numeric byte counts.",
    },
    FactoryTranslationKey {
        key: "import.diag.archive_entries_exceed_limit",
        english_text: "{actual} entries exceeds the {limit} entry limit.",
        translator_context: "Import archive inspection failure. {actual} and {limit} are numeric ZIP entry counts.",
    },
    FactoryTranslationKey {
        key: "import.diag.manifest_schema_format_mismatch",
        english_text: "Manifest '{manifest}'; schema '{schema}'.",
        translator_context: "Import format-identifier mismatch detail. {manifest} and {schema} are machine format identifiers and must remain verbatim.",
    },


    FactoryTranslationKey {
        key: "import.diag.local_timestamp_failed",
        english_text: "Unable to determine local Import timestamp: {error}",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.unique_import_name_failed",
        english_text: "Unable to generate a unique imported name for '{name}'.",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.no_deterministic_rename",
        english_text: "{type} '{name}' has no deterministic imported rename.",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.missing_package_shader",
        english_text: "Missing package shader.",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.missing_package_policy",
        english_text: "Missing package policy.",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.missing_package_playlist",
        english_text: "Missing package playlist.",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.conflict_discovery_unavailable",
        english_text: "Conflict discovery unavailable",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.conflict_database_open_failed",
        english_text: "Unable to open screenshaver.db for read-only conflict discovery: {error}",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.shader_hash_prepare_failed",
        english_text: "Unable to prepare shader hash lookup: {error}",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.shader_hash_query_failed",
        english_text: "Unable to query shader hash '{hash}': {error}",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.shader_hash_decode_failed",
        english_text: "Unable to decode shader hash lookup: {error}",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.shader_filename_check_failed",
        english_text: "Unable to check shader filename '{filename}': {error}",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.policy_name_prepare_failed",
        english_text: "Unable to prepare Policy Name lookup: {error}",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.policy_name_query_failed",
        english_text: "Unable to query Policy Name '{name}': {error}",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.policy_name_decode_failed",
        english_text: "Unable to decode Policy Name lookup: {error}",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.playlist_query_failed",
        english_text: "Unable to query playlist '{name}': {error}",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.playlist_member_prepare_failed",
        english_text: "Unable to prepare playlist member lookup: {error}",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.playlist_members_query_failed",
        english_text: "Unable to query playlist members: {error}",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.playlist_members_decode_failed",
        english_text: "Unable to decode playlist members: {error}",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.validated_policy_missing_field",
        english_text: "Validated policy '{policy}' lacks '{field}'.",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.invalid_number_in_field",
        english_text: "Invalid number '{value}' in {field}.",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.invalid_integer_in_field",
        english_text: "Invalid integer '{value}' in {field}.",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.invalid_boolean_in_field",
        english_text: "Invalid boolean '{value}' in {field}.",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.unsupported_policy_target",
        english_text: "Unsupported imported policy target '{target}'.",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.receiving_defaults_texture_mode",
        english_text: "Receiving {target} defaults have unsupported texture mode '{mode}'.",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.receiving_policy_texture_mode",
        english_text: "Receiving policy has unsupported texture mode '{mode}'.",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.receiving_defaults_palette_mode",
        english_text: "Receiving {target} defaults have unsupported palette mode '{mode}'.",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.diag.receiving_policy_palette_mode",
        english_text: "Receiving policy has unsupported palette mode '{mode}'.",
        translator_context: "Import runtime diagnostic. Named parameters are supplied package, database, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.policy_with_id",
        english_text: "Policy {id}",
        translator_context: "Fallback display name for a missing referenced policy. {id} is the package policy ID.",
    },


    FactoryTranslationKey {
        key: "import.exec.archive_reinspection_failed",
        english_text: "The archive no longer passes inspection. No Import changes were made.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.validated_package_unavailable",
        english_text: "Validated package is unavailable.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.conflict_timestamp_unavailable",
        english_text: "The Import conflict timestamp is unavailable. No Import changes were made.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.destination_name_changed",
        english_text: "The receiving installation changed after Review: '{previous}' is no longer the deterministic destination name (now '{current}'). No Import changes were made.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.conflict_set_changed",
        english_text: "The receiving installation changed after Review and the conflict set is no longer the same. No Import changes were made.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.new_conflicts_discovered",
        english_text: "The receiving installation changed after Review and new conflicts were discovered. No Import changes were made.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.conflict_resolution_stale",
        english_text: "Import conflict resolution is incomplete or stale: {error} No Import changes were made.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.unresolved_conflicts",
        english_text: "Execution-time conflict discovery found {count} unresolved conflict(s):\\n\\n{details}\\n\\nNo Import changes were made.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.policy_unresolved_destination",
        english_text: "Policy '{name}' (package ID {id}) has an unresolved destination.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.policy_unresolved_shader",
        english_text: "Policy '{name}' (package ID {id}) requires unresolved shader package ID {shader_id}.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.playlist_unresolved_destination",
        english_text: "Playlist '{name}' (package ID {id}) has an unresolved destination.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.playlist_member_unresolved_policy",
        english_text: "Playlist '{playlist}' member {position} '{policy}' requires unresolved policy package ID {policy_id}.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.unresolved_dependencies",
        english_text: "Execution-time dependency validation found {count} unresolved dependency reference(s):\\n\\n{details}\\n\\nNo Import changes were made.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.failed_database_restored",
        english_text: "{error} The pre-Import database was restored. Newly installed shader files were removed where possible.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.failed_restore_also_failed",
        english_text: "{error} DATABASE RESTORE ALSO FAILED: {restore_error}. The verified backup remains at '{backup}'.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.backup_timestamp_failed",
        english_text: "Unable to determine pre-Import backup timestamp: {error}",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.backup_create_failed",
        english_text: "Unable to create pre-Import database backup '{path}': {error}",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.backup_open_verify_failed",
        english_text: "Unable to open pre-Import database backup for verification: {error}",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.backup_verify_failed",
        english_text: "Unable to verify pre-Import database backup: {error}",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.backup_integrity_failed",
        english_text: "Pre-Import database backup failed integrity verification: {result}",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.restore_copy_failed",
        english_text: "Unable to restore '{database}' from '{backup}': {error}",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.restored_database_open_failed",
        english_text: "Unable to open restored database: {error}",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.restored_database_verify_failed",
        english_text: "Unable to verify restored database: {error}",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.restored_database_integrity_result",
        english_text: "Restored database integrity_check returned '{result}'.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.shader_directory_create_failed",
        english_text: "Unable to create managed shader directory '{path}': {error}",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.archive_reopen_failed",
        english_text: "Unable to reopen Import archive: {error}",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.zip_reopen_failed",
        english_text: "Unable to reopen Import ZIP: {error}",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.shader_overwrite_refused",
        english_text: "Refusing to overwrite unexpected existing shader file '{path}'.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.shader_payload_read_failed",
        english_text: "Unable to read shader payload '{path}': {error}",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.shader_changed_after_inspection",
        english_text: "Shader '{filename}' changed after inspection; SHA-256 no longer matches.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.shader_install_failed",
        english_text: "Unable to install shader '{path}': {error}",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.shader_reconcile_failed",
        english_text: "Unable to reconcile imported shader files: {error}",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.shader_registration_failed",
        english_text: "Imported shader '{filename}' was not registered as expected: {error}",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.transaction_begin_failed",
        english_text: "Unable to begin Import database transaction: {error}",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.policy_no_destination_shader",
        english_text: "Policy '{name}' has no resolved destination shader.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.playlist_import_failed",
        english_text: "Unable to import playlist '{name}': {error}",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.playlist_id_no_mapping",
        english_text: "Playlist package ID {id} has no destination mapping.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.policy_id_no_membership_mapping",
        english_text: "Policy package ID {id} has no destination mapping for playlist membership.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.membership_restore_failed",
        english_text: "Unable to restore playlist membership at position {position}: {error}",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.transaction_commit_failed",
        english_text: "Unable to commit Import database transaction: {error}",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.success_detail",
        english_text: "The validated package was imported successfully. Genuine conflicts were preserved additively under deterministic imported names; existing recipient objects were not modified. Truly identical duplicates were reused, and imported dependencies were mapped to their destination identities. screenshaver.toml was not changed.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.imported_policy_missing_field",
        english_text: "Imported policy '{policy}' lacks '{field}'.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.invalid_policy_integer",
        english_text: "Invalid integer '{value}' in imported policy field '{field}'.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.invalid_policy_number",
        english_text: "Invalid number '{value}' in imported policy field '{field}'.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.invalid_policy_boolean",
        english_text: "Invalid boolean '{value}' in imported policy field '{field}'.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.unsupported_texture_mode",
        english_text: "Unsupported imported texture mode '{mode}'.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.unsupported_palette_mode",
        english_text: "Unsupported imported palette mode '{mode}'.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.unsupported_animation_speed_mode",
        english_text: "Unsupported imported animation speed mode '{mode}'.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.policy_import_failed",
        english_text: "Unable to import policy '{name}': {error}",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.imported_name_length_invalid",
        english_text: "Imported name must contain between 1 and 128 characters; found {length}.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.exec.imported_name_empty_key",
        english_text: "Imported name produced an empty comparison key.",
        translator_context: "Import execution, validation, or recovery message. Named parameters are supplied package, database, filesystem, or error values and must remain unchanged.",
    },


    FactoryTranslationKey {
        key: "import.validation.export_format_unsupported",
        english_text: "Screenshaver Export Format {version} is not supported by this installation.",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.export_schema_load_failed",
        english_text: "Unable to load Export Schema {version}: {error}",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.export_schema_version_mismatch",
        english_text: "Export Schema version does not match the archive.",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.export_schema_integrity_unsupported",
        english_text: "Export Schema requests unsupported integrity behavior.",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.export_schema_manifest_hashing",
        english_text: "Export Schema must exclude its manifest from package hashing.",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.schema_metadata_undeclared",
        english_text: "Schema metadata file '{file}' is not declared in the archive.",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.dataset_not_utf8",
        english_text: "'{file}' is not UTF-8: {error}",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.dataset_empty",
        english_text: "'{file}' is empty.",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.dataset_header_mismatch",
        english_text: "'{file}' header does not match Export Schema.",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.dataset_row_error",
        english_text: "'{file}' row {row}: {error}",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.dataset_column_count",
        english_text: "'{file}' row {row} has {actual} columns; {required} required.",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.tsv_trailing_escape",
        english_text: "Trailing TSV escape.",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.tsv_escape_unsupported",
        english_text: "Unsupported TSV escape '\\{escape}'.",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.schema_dataset_missing_column",
        english_text: "Schema dataset '{file}' lacks column '{column}'.",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.duplicate_id",
        english_text: "Duplicate {label} {id}.",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.not_positive_integer",
        english_text: "{label} '{value}' is not a positive integer.",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.must_be_positive",
        english_text: "{label} must be greater than zero.",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.policy_missing_shader_id",
        english_text: "Policy references missing shader_export_id {id}.",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.membership_unresolved_id",
        english_text: "Playlist membership contains an unresolved package ID.",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.playlist_duplicate_policy",
        english_text: "Playlist {playlist} contains policy {policy} more than once.",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.playlist_duplicate_position",
        english_text: "Playlist {playlist} contains duplicate position {position}.",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.shader_declared_twice",
        english_text: "Shader '{path}' is declared more than once.",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.shader_sha_malformed",
        english_text: "Shader '{path}' has malformed SHA-256.",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.declared_shader_missing",
        english_text: "Declared shader '{path}' is missing.",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.declared_shader_directory",
        english_text: "Declared shader '{path}' is a directory.",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.shader_executable",
        english_text: "Shader '{path}' is marked executable.",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.shader_sha_failed",
        english_text: "Shader '{path}' failed SHA-256 verification.",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "import.validation.shader_not_utf8",
        english_text: "Shader '{path}' is not valid UTF-8 text.",
        translator_context: "Import package, schema, TSV, or shader validation message. Named parameters are stable identifiers, paths, package values, or external error text and must remain unchanged.",
    },


    FactoryTranslationKey {
        key: "export.error.unable_to_load_playlists_for_export_selection",
        english_text: "Unable to load playlists for export selection: {value1}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_load_schema",
        english_text: "Unable to load Screenshaver Export Schema V1: {value1}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.schema_has_an_empty_format_identifier",
        english_text: "Screenshaver Export Schema V1 has an empty format identifier.",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.schema_has_an_invalid_format_version",
        english_text: "Screenshaver Export Schema V1 has an invalid format version.",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.schema_requests_an_unsupported_integrity_algorithm_or_canonicalization",
        english_text: "Screenshaver Export Schema V1 requests an unsupported integrity algorithm or canonicalization.",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.schema_must_exclude_its_manifest_from_the_package_hash",
        english_text: "Screenshaver Export Schema V1 must exclude its manifest from the package hash.",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.schema_dataset_is_not_declared_as_archive_metadata",
        english_text: "Screenshaver Export Schema V1 dataset '{value1}' is not declared as archive metadata.",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.missing_package_local_export_id_for_policy",
        english_text: "Missing package-local export ID for policy '{value1}'",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.missing_package_local_export_id_for_shader_referenced_by_policy",
        english_text: "Missing package-local export ID for shader '{value1}' referenced by policy '{value2}'",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.missing_package_local_export_id_for_playlist",
        english_text: "Missing package-local export ID for playlist '{value1}'",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_load_members_for_playlist_while_exporting",
        english_text: "Unable to load members for playlist {value1} while exporting: {value2}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.missing_package_local_export_id_for_policy_in_playlist",
        english_text: "Missing package-local export ID for policy {value1} in playlist '{value2}'",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_read_shader_at",
        english_text: "Unable to read shader '{value1}' at '{value2}': {value3}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.missing_package_local_export_id_for_shader",
        english_text: "Missing package-local export ID for shader '{value1}'",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_create_in_export_archive",
        english_text: "Unable to create '{value1}' in export archive: {value2}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_write_to_export_archive",
        english_text: "Unable to write '{value1}' to export archive: {value2}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.the_export_destination_is_not_valid",
        english_text: "The export destination is not valid.",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.the_export_destination_folder_is_not_valid",
        english_text: "The export destination folder is not valid.",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.export_destination_folder_does_not_exist",
        english_text: "Export destination folder does not exist: {value1}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.the_effective_policy_snapshot_is_incomplete_return_to_selection_and_try_again",
        english_text: "The effective policy snapshot is incomplete. Return to selection and try again.",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_read_shader_while_calculating_package_integrity",
        english_text: "Unable to read shader '{value1}' while calculating package integrity: {value2}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_serialize_export_manifest",
        english_text: "Unable to serialize export manifest: {value1}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_remove_stale_temporary_export",
        english_text: "Unable to remove stale temporary export '{value1}': {value2}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_create_temporary_export_archive",
        english_text: "Unable to create temporary export archive '{value1}': {value2}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_add_database_snapshot_to_backup_archive",
        english_text: "Unable to add database snapshot to backup archive: {value1}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_write_database_snapshot_to_backup_archive",
        english_text: "Unable to write database snapshot to backup archive: {value1}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_add_managed_shader_to_backup_archive",
        english_text: "Unable to add managed shader '{value1}' to backup archive: {value2}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_write_managed_shader_to_backup_archive",
        english_text: "Unable to write managed shader '{value1}' to backup archive: {value2}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_add_shader_to_export_archive",
        english_text: "Unable to add shader '{value1}' to export archive: {value2}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_open_shader",
        english_text: "Unable to open shader '{value1}': {value2}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_read_shader",
        english_text: "Unable to read shader '{value1}': {value2}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_write_shader_to_export_archive",
        english_text: "Unable to write shader '{value1}' to export archive: {value2}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_finalize_export_archive",
        english_text: "Unable to finalize export archive: {value1}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_move_completed_export_archive_to",
        english_text: "Unable to move completed export archive to '{value1}': {value2}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_enumerate_managed_shader_directory",
        english_text: "Unable to enumerate managed shader directory '{value1}': {value2}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_read_managed_shader_directory_entry",
        english_text: "Unable to read managed shader directory entry: {value1}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_inspect_managed_shader_entry",
        english_text: "Unable to inspect managed shader entry '{value1}': {value2}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_read_managed_shader_for_full_backup",
        english_text: "Unable to read managed shader '{value1}' for full backup: {value2}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_remove_stale_database_snapshot",
        english_text: "Unable to remove stale database snapshot '{value1}': {value2}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_open_database_for_backup_snapshot",
        english_text: "Unable to open database for backup snapshot: {value1}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_create_consistent_database_snapshot",
        english_text: "Unable to create consistent database snapshot: {value1}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_open_database_snapshot_for_verification",
        english_text: "Unable to open database snapshot for verification: {value1}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_verify_database_snapshot",
        english_text: "Unable to verify database snapshot: {value1}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.database_snapshot_failed_integrity_verification",
        english_text: "Database snapshot failed integrity verification: {value1}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_read_verified_database_snapshot",
        english_text: "Unable to read verified database snapshot '{value1}': {value2}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_create_backup_directory",
        english_text: "Unable to create backup directory '{value1}': {value2}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_prepare_full_backup_selection",
        english_text: "Unable to prepare full backup selection: {value1}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_resolve_full_backup_policies",
        english_text: "Unable to resolve full backup policies: {value1}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_determine_backup_filename_timestamp",
        english_text: "Unable to determine backup filename timestamp.",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_prepare_effective_export_policy_query",
        english_text: "Unable to prepare effective export policy query: {value1}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_read_selected_policy_id_while_resolving_export_data",
        english_text: "Unable to read selected policy ID {value1} while resolving export data: {value2}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.policy_has_unsupported_target",
        english_text: "Policy '{value1}' has unsupported target '{value2}'",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.one_or_more_selected_policies_disappeared_while_export_data_was_being_resolved",
        english_text: "One or more selected policies disappeared while export data was being resolved",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.policy_has_a_specific_texture_without_a_texture_family",
        english_text: "Policy '{value1}' has a specific texture without a texture family",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.policy_has_a_specific_texture_without_a_primitive_count",
        english_text: "Policy '{value1}' has a specific texture without a primitive count",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.policy_has_unsupported_texture_mode",
        english_text: "Policy '{value1}' has unsupported texture mode '{value2}'",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.defaults_specify_a_specific_texture_without_a_texture_family_while_resolving_policy",
        english_text: "{value1} defaults specify a specific texture without a texture family while resolving policy '{value2}'",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.defaults_contain_unsupported_texture_mode_while_resolving_policy",
        english_text: "{value1} defaults contain unsupported texture mode '{value2}' while resolving policy '{value3}'",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.policy_has_a_specific_palette_without_a_palette_color",
        english_text: "Policy '{value1}' has a specific palette without a palette color",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.policy_has_unsupported_palette_mode",
        english_text: "Policy '{value1}' has unsupported palette mode '{value2}'",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.defaults_specify_a_specific_palette_without_a_palette_color_while_resolving_policy",
        english_text: "{value1} defaults specify a specific palette without a palette color while resolving policy '{value2}'",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.defaults_contain_unsupported_palette_mode_while_resolving_policy",
        english_text: "{value1} defaults contain unsupported palette mode '{value2}' while resolving policy '{value3}'",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.policy_has_invalid_boolean_value_for",
        english_text: "Policy '{value1}' has invalid boolean value {value2} for {value3}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.assigned_policy_retained_unresolved_target_texture_inheritance",
        english_text: "Assigned policy '{value1}' retained unresolved target texture inheritance",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.assigned_policy_retained_unresolved_target_palette_inheritance",
        english_text: "Assigned policy '{value1}' retained unresolved target palette inheritance",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.assigned_policy_retained_unresolved_target_animation_speed_inheritance",
        english_text: "Assigned policy '{value1}' retained unresolved target animation-speed inheritance",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.policy_resolved_to_an_invalid_specific_texture",
        english_text: "Policy '{value1}' resolved to an invalid specific texture",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.assigned_policy_resolved_to_random_texture_without_an_explicit_primitive_count",
        english_text: "Assigned policy '{value1}' resolved to random texture without an explicit primitive count",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.policy_resolved_to_an_invalid_random_texture_primitive_count",
        english_text: "Policy '{value1}' resolved to an invalid random-texture primitive count",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.policy_resolved_to_one_or_more_invalid_numeric_export_values",
        english_text: "Policy '{value1}' resolved to one or more invalid numeric export values",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.policy_resolved_to_an_invalid_animation_speed",
        english_text: "Policy '{value1}' resolved to an invalid animation speed",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_prepare_export_policy_selection_query",
        english_text: "Unable to prepare export policy selection query: {value1}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_query_policies_for_export_selection",
        english_text: "Unable to query policies for export selection: {value1}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.error.unable_to_decode_export_policy_selection_row",
        english_text: "Unable to decode export policy selection row: {value1}",
        translator_context: "Export workflow user-facing error/status text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "export.review_instruction",
        english_text: "Review the selected export focus, included policies, shaders and playlists, and destination before starting Export.",
        translator_context: "Export wizard Review page instruction shown before the user starts the export.",
    },
    FactoryTranslationKey {
        key: "edit.edit_shader_requires_a_shader_file_not_a_directory",
        english_text: "--edit-shader requires a shader file, not a directory: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.bulk_edit_complete_policies_updated",
        english_text: "Bulk Edit complete: {value1} policies updated.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.bulk_edit_complete_policies_updated_policy_target_was_preserved_for_protect",
        english_text: "Bulk Edit complete: {value1} policies updated. Policy Target was preserved for {value2} protected default {value3}.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.bulk_edit_contains_no_changed_settings",
        english_text: "Bulk Edit contains no changed settings.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.bulk_edit_could_not_suspend_the_active_shader_because_its_database_id_could",
        english_text: "Bulk Edit could not suspend the active shader because its database ID could not be found.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.bulk_edit_could_not_suspend_the_active_shader",
        english_text: "Bulk Edit could not suspend the active shader: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.bulk_edit_ended_but_the_previous_shader_could_not_be_reloaded",
        english_text: "Bulk Edit ended, but the previous shader could not be reloaded: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.bulk_edit_ended_the_previously_loaded_shader_is_no_longer_available",
        english_text: "Bulk Edit ended; the previously loaded shader is no longer available.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.bulk_policies_were_saved_but_configuration_reload_failed",
        english_text: "Bulk policies were saved, but configuration reload failed.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.bulk_policy_creation_canceled",
        english_text: "Bulk policy creation canceled.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.bulk_policy_creation_complete_created_already_existed",
        english_text: "Bulk policy creation complete: {value1} created, {value2} already existed.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.bulk_policy_creation_failed",
        english_text: "Bulk policy creation failed: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.bulk_policy_save_aborted",
        english_text: "Bulk policy save aborted: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.configuration_save_failed",
        english_text: "Configuration save failed.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.configuration_saved",
        english_text: "Configuration saved.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.gl_shader_files",
        english_text: "GL shader files",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.loaded_and_rendering",
        english_text: "Loaded and rendering",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.loaded_existing_screensaver_policy_for_this_shader",
        english_text: "Loaded existing Screensaver policy for this shader.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.loaded_existing_unassigned_policy_for_this_shader",
        english_text: "Loaded existing Unassigned policy for this shader.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.loaded_existing_wallpaper_policy_for_this_shader",
        english_text: "Loaded existing Wallpaper policy for this shader.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.loaded_existing_policy_for_this_shader",
        english_text: "Loaded existing {value1} policy for this shader.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.loaded_shader_using_resolved_defaults_select_a_policy_target_to_create_a_po",
        english_text: "Loaded shader using resolved defaults. Select a policy target to create a policy.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.loaded_shader_with_its_existing_screensaver_policy",
        english_text: "Loaded shader with its existing Screensaver policy.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.loaded_shader_with_its_existing_unassigned_policy",
        english_text: "Loaded shader with its existing Unassigned policy.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.loaded_shader_with_its_existing_wallpaper_policy",
        english_text: "Loaded shader with its existing Wallpaper policy.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.new_unassigned_policy_is_ready_to_save",
        english_text: "New Unassigned policy is ready to save.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.no_screensaver_policy_exists_loaded_screensaver_defaults",
        english_text: "No Screensaver policy exists. Loaded Screensaver defaults.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.no_unassigned_policy_exists_loaded_defaults_for_a_new_unassigned_policy",
        english_text: "No Unassigned policy exists. Loaded defaults for a new Unassigned policy.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.no_wallpaper_policy_exists_loaded_wallpaper_defaults",
        english_text: "No Wallpaper policy exists. Loaded Wallpaper defaults.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.no_existing_shader_policy_found_select_a_policy_target_to_create_one",
        english_text: "No existing shader policy found. Select a policy target to create one.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.no_policies_changed_policy_target_cannot_be_changed_for_protected_default",
        english_text: "No policies changed. Policy Target cannot be changed for {value1} protected default {value2}.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.no_policy_target_was_selected_for_external_shader",
        english_text: "No policy target was selected for external shader {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.no_shader_path_was_supplied_for_editing",
        english_text: "No shader path was supplied for editing",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.no_usable_shaders_were_selected_for_policy_creation",
        english_text: "No usable shaders were selected for policy creation.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.no_policy_exists_loaded_defaults",
        english_text: "No {value1} policy exists. Loaded {value2} defaults.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.not_required",
        english_text: "Not required",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.playlist_display_mode_requires_a_playlist_selection",
        english_text: "Playlist display mode requires a playlist selection.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.playlist_display_mode_requires_a_positive_interval",
        english_text: "Playlist display mode requires a positive interval.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.policies_were_created_but_configuration_reload_failed",
        english_text: "Policies were created, but configuration reload failed.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.policy_has_unsupported_policy_target",
        english_text: "Policy '{value1}' has unsupported policy_target '{value2}'",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.policy_cannot_be_opened_because_its_shader_is_not_renderable",
        english_text: "Policy cannot be opened because its shader is not renderable: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.policy_cloned_as",
        english_text: "Policy cloned as '{value1}'.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.policy_paths_could_not_be_updated_after_moving_the_shader_rollback_also_fai",
        english_text: "Policy paths could not be updated after moving the shader: {value1}. Rollback also failed: {value2}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.policy_renamed_to",
        english_text: "Policy renamed to '{value1}'.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.policy_saved_for",
        english_text: "Policy saved for {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.policy_saved_but_audio_motion_could_not_be_saved",
        english_text: "Policy saved, but Audio Motion could not be saved: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.policy_shader_file_is_unavailable",
        english_text: "Policy shader file is unavailable: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.policy_was_cloned_but_configuration_reload_failed",
        english_text: "Policy was cloned, but configuration reload failed: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.policy_was_renamed_but_configuration_reload_failed",
        english_text: "Policy was renamed, but configuration reload failed: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.recent_files_were_cleared_for_this_session_but_the_history_file_could_not_b",
        english_text: "Recent files were cleared for this session, but the history file could not be updated: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.recent_shader_file_no_longer_exists",
        english_text: "Recent shader file no longer exists: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.recent_shader_file_history_cleared",
        english_text: "Recent shader-file history cleared.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.refreshed_shader_from_disk",
        english_text: "Refreshed shader from disk: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.sdl_initialization_failed",
        english_text: "SDL initialization failed: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.sdl_video_initialization_failed",
        english_text: "SDL video initialization failed: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.screensaver_target_enforced_by_shader_location_new_screensaver_policy_is_re",
        english_text: "Screensaver target enforced by shader location. New Screensaver policy is ready to save.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.control_center",
        english_text: "Screenshaver Control Center",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.export_archive",
        english_text: "Screenshaver Export Archive",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.shader_has_file_status",
        english_text: "Shader '{value1}' has file_status '{value2}'",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.shader_has_invalid_channel_usage_mask",
        english_text: "Shader '{value1}' has invalid channel-usage mask {value2}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.shader_has_validation_status_expected",
        english_text: "Shader '{value1}' has validation_status '{value2}'; expected 'valid'",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.shader_is_not_registered_in_the_database",
        english_text: "Shader '{value1}' is not registered in the Screenshaver database",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.shader_is_rejected",
        english_text: "Shader '{value1}' is rejected: {value2}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.shader_already_exists_in",
        english_text: "Shader already exists in {value1}.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.shader_file_is_unavailable",
        english_text: "Shader file is unavailable: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.shader_file_no_longer_exists",
        english_text: "Shader file no longer exists: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.shader_filename_is_not_valid_utf_8",
        english_text: "Shader filename is not valid UTF-8: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.shader_loaded_but_recent_file_history_could_not_be_saved",
        english_text: "Shader loaded, but recent-file history could not be saved: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.shader_loading_canceled",
        english_text: "Shader loading canceled.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.shader_move_was_rolled_back_because_policy_paths_could_not_be_updated",
        english_text: "Shader move was rolled back because policy paths could not be updated: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.shader_moved_to",
        english_text: "Shader moved to {value1}.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.shader_moved_to_policy_target_updated_to",
        english_text: "Shader moved to {value1}. Policy target updated to {value2}.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.shader_moved_configuration_reload_failed",
        english_text: "Shader moved; configuration reload failed.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.shader_path_has_no_valid_filename",
        english_text: "Shader path has no valid filename: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.shader_was_not_deleted_because_its_associated_policy_could_not_be_deleted",
        english_text: "Shader was not deleted because its associated policy could not be deleted: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.single_display_mode_requires_a_shader_policy_selection",
        english_text: "Single display mode requires a shader policy selection.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.the_selected_policy_target_is_unavailable_in_the_current_editing_session",
        english_text: "The selected policy target is unavailable in the current editing session.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.the_selected_shader_could_not_be_loaded_for_editing",
        english_text: "The selected shader could not be loaded for editing",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.this_shader_cannot_use_a_screensaver_policy_in_the_current_editing_session",
        english_text: "This shader cannot use a Screensaver policy in the current editing session.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.this_shader_cannot_use_a_wallpaper_policy_in_the_current_editing_session",
        english_text: "This shader cannot use a Wallpaper policy in the current editing session.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.this_shader_cannot_use_an_unassigned_policy_in_the_current_editing_session",
        english_text: "This shader cannot use an Unassigned policy in the current editing session.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_clone_policy",
        english_text: "Unable to clone policy: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_create_control_center_state_folder",
        english_text: "Unable to create Control Center state folder {value1}: {value2}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_create_opengl_context",
        english_text: "Unable to create OpenGL context: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_create_sdl_event_pump",
        english_text: "Unable to create SDL event pump: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_create_destination_directory",
        english_text: "Unable to create destination directory {value1} ({value2})",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_create_edit_shader_opengl_context",
        english_text: "Unable to create edit-shader OpenGL context: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_create_edit_shader_sdl_event_pump",
        english_text: "Unable to create edit-shader SDL event pump: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_create_edit_shader_window",
        english_text: "Unable to create edit-shader window: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_decode_policy_list_database_row",
        english_text: "Unable to decode Policy List database row: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_delete_policy",
        english_text: "Unable to delete policy: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_load_shader",
        english_text: "Unable to load shader: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_move_shader_from_to",
        english_text: "Unable to move shader from {value1} to {value2} ({value3})",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_open_database_while_reading_shader_metadata_for",
        english_text: "Unable to open database while reading shader metadata for '{value1}': {value2}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_prepare_policy_list_database_query",
        english_text: "Unable to prepare Policy List database query: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_prepare_policy_clone",
        english_text: "Unable to prepare policy clone: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_query_policy_list_rows_from_database",
        english_text: "Unable to query Policy List rows from database: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_query_shader_id_for",
        english_text: "Unable to query shader ID for '{value1}': {value2}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_read_database_metadata_for",
        english_text: "Unable to read database metadata for '{value1}': {value2}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_refresh_shader",
        english_text: "Unable to refresh shader: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_rename_policy",
        english_text: "Unable to rename policy: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_resolve_shader_id",
        english_text: "Unable to resolve shader_id {value1}: {value2}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_restore_control_center_fullscreen_state",
        english_text: "Unable to restore Screenshaver Control Center fullscreen state: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_restore_the_previous_shader_after_bulk_edit",
        english_text: "Unable to restore the previous shader after Bulk Edit: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_save_bulk_policy_changes",
        english_text: "Unable to save bulk policy changes: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_save_policy",
        english_text: "Unable to save policy: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_serialize_control_center_state",
        english_text: "Unable to serialize Control Center state: {value1}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unable_to_write_control_center_state",
        english_text: "Unable to write Control Center state {value1}: {value2}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.unsupported_display_mode",
        english_text: "Unsupported display mode '{value1}'.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.valid_shader_has_no_channel_usage_metadata",
        english_text: "Valid shader '{value1}' has no channel-usage metadata",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.wallpaper_target_enforced_by_shader_location_new_wallpaper_policy_is_ready",
        english_text: "Wallpaper target enforced by shader location. New Wallpaper policy is ready to save.",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.shader_is_rejected_2",
        english_text: "shader is rejected",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.policy_deleted_for",
        english_text: "{value1} policy deleted for {value2}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.policy_was_deleted_but_the_shader_file_could_not_be_deleted",
        english_text: "{value1} policy was deleted, but the shader file could not be deleted: {value2}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "edit.shader_and_associated_policy_deleted",
        english_text: "{value1} shader and associated {value2} policy deleted: {value3}",
        translator_context: "Control Center and shader-edit user-facing text. Supplied {valueN} parameters are runtime data and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "target.unassigned",
        english_text: "Unassigned",
        translator_context: "User-facing name of the unassigned policy target.",
    },
    FactoryTranslationKey {
        key: "edit.bulk_edit_no_changes_protected_default_one",
        english_text: "No policies changed. Policy Target cannot be changed for {value1} protected default policy.",
        translator_context: "Bulk Edit result when one protected default policy prevents a target change.",
    },
    FactoryTranslationKey {
        key: "edit.bulk_edit_no_changes_protected_default_many",
        english_text: "No policies changed. Policy Target cannot be changed for {value1} protected default policies.",
        translator_context: "Bulk Edit result when multiple protected default policies prevent target changes.",
    },
    FactoryTranslationKey {
        key: "edit.bulk_edit_complete_protected_default_one",
        english_text: "Bulk Edit complete: {value1} policies updated. Policy Target was preserved for {value2} protected default policy.",
        translator_context: "Bulk Edit result when one protected default policy preserves its target.",
    },
    FactoryTranslationKey {
        key: "edit.bulk_edit_complete_protected_default_many",
        english_text: "Bulk Edit complete: {value1} policies updated. Policy Target was preserved for {value2} protected default policies.",
        translator_context: "Bulk Edit result when multiple protected default policies preserve their targets.",
    },
    FactoryTranslationKey {
        key: "edit.select_policy_target_before_saving",
        english_text: "Select a policy target before saving",
        translator_context: "Control Center validation shown when saving without a selected policy target.",
    },
    FactoryTranslationKey {
        key: "edit.delete_shader_from_policy_context_menu",
        english_text: "Delete Shader is available from the Policies row context menu.",
        translator_context: "Control Center guidance for deleting a shader from the Policy List.",
    },
    FactoryTranslationKey {
        key: "edit.select_target_for_selected_unassigned_policies",
        english_text: "Select Screensaver or Wallpaper as the Policy Target for the selected Unassigned policies.",
        translator_context: "Bulk policy assignment validation message.",
    },
    FactoryTranslationKey {
        key: "edit.required",
        english_text: "Required",
        translator_context: "Shader Information value indicating that texture input is required.",
    },

    FactoryTranslationKey {
        key: "assign_shader_policies.all_screensavers",
        english_text: "All Screensavers",
        translator_context: "Assignment choice that creates a screensaver policy for every newly discovered policy-less managed shader.",
    },
    FactoryTranslationKey {
        key: "assign_shader_policies.all_wallpapers",
        english_text: "All Wallpapers",
        translator_context: "Assignment choice that creates a wallpaper policy for every newly discovered policy-less managed shader.",
    },
    FactoryTranslationKey {
        key: "assign_shader_policies.screensavers_and_wallpapers",
        english_text: "Screensavers + Wallpapers",
        translator_context: "Assignment choice that creates both screensaver and wallpaper policies for every newly discovered policy-less managed shader.",
    },
    FactoryTranslationKey {
        key: "assign_shader_policies.all_unassigned",
        english_text: "All Unassigned",
        translator_context: "Assignment choice that creates an unassigned policy for every newly discovered policy-less managed shader.",
    },
    FactoryTranslationKey {
        key: "assign_shader_policies.assignment_title",
        english_text: "Assign New Shader Policies",
        translator_context: "Title of the dialog shown when managed shaders without policies are discovered.",
    },
    FactoryTranslationKey {
        key: "assign_shader_policies.assignment_message.singular",
        english_text: "Screenshaver found {count} shader in the managed shaders folder that does not yet have a policy.\n\nChoose how a policy should be created for this shader.\n\nUnassigned policies cannot be rendered until their Policy Target is changed to Screensaver or Wallpaper.",
        translator_context: "Assignment dialog message when exactly one policy-less managed shader is found. {count} is the numeric shader count.",
    },
    FactoryTranslationKey {
        key: "assign_shader_policies.assignment_message.plural",
        english_text: "Screenshaver found {count} shaders in the managed shaders folder that do not yet have a policy.\n\nChoose how policies should be created for these shaders.\n\nUnassigned policies cannot be rendered until their Policy Target is changed to Screensaver or Wallpaper.",
        translator_context: "Assignment dialog message when multiple policy-less managed shaders are found. {count} is the numeric shader count.",
    },
    FactoryTranslationKey {
        key: "assign_shader_policies.completion_title",
        english_text: "Shader Policies Created",
        translator_context: "Title of the confirmation dialog after policies are created for newly discovered shaders.",
    },
    FactoryTranslationKey {
        key: "assign_shader_policies.completion_message.one_policy_one_shader",
        english_text: "Screenshaver created {policy_count} shader policy for {shader_count} shader using \"{assignment}\".\n\nYou can review or change shader policies at any time by running:\n\nscreenshaver --control\n\nThis opens the Screenshaver Control Center.",
        translator_context: "Completion message for one created policy and one shader. Counts are numeric; {assignment} is localized assignment-choice text; the command screenshaver --control must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "assign_shader_policies.completion_message.one_policy_many_shaders",
        english_text: "Screenshaver created {policy_count} shader policy for {shader_count} shaders using \"{assignment}\".\n\nYou can review or change shader policies at any time by running:\n\nscreenshaver --control\n\nThis opens the Screenshaver Control Center.",
        translator_context: "Completion message for one created policy and multiple shaders. Counts are numeric; {assignment} is localized assignment-choice text; the command screenshaver --control must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "assign_shader_policies.completion_message.many_policies_one_shader",
        english_text: "Screenshaver created {policy_count} shader policies for {shader_count} shader using \"{assignment}\".\n\nYou can review or change shader policies at any time by running:\n\nscreenshaver --control\n\nThis opens the Screenshaver Control Center.",
        translator_context: "Completion message for multiple created policies and one shader. Counts are numeric; {assignment} is localized assignment-choice text; the command screenshaver --control must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "assign_shader_policies.completion_message.many_policies_many_shaders",
        english_text: "Screenshaver created {policy_count} shader policies for {shader_count} shaders using \"{assignment}\".\n\nYou can review or change shader policies at any time by running:\n\nscreenshaver --control\n\nThis opens the Screenshaver Control Center.",
        translator_context: "Completion message for multiple created policies and multiple shaders. Counts are numeric; {assignment} is localized assignment-choice text; the command screenshaver --control must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "assign_shader_policies.error.prepare_query",
        english_text: "Unable to prepare policy-less shader query: {error}",
        translator_context: "Error preparing the database query for managed shaders that do not have policies. {error} is supplied database error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "assign_shader_policies.error.query_shaders",
        english_text: "Unable to query policy-less shaders: {error}",
        translator_context: "Error executing the database query for managed shaders that do not have policies. {error} is supplied database error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "assign_shader_policies.error.decode_row",
        english_text: "Unable to decode policy-less shader row: {error}",
        translator_context: "Error decoding a database row for a managed shader without policies. {error} is supplied database error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "assign_shader_policies.error.display_dialog",
        english_text: "Unable to display new-policy assignment dialog: {error}",
        translator_context: "Error displaying the new-policy assignment dialog. {error} is supplied SDL error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "assign_shader_policies.error.unknown_button",
        english_text: "New-policy assignment dialog returned unknown button id {button_id}",
        translator_context: "Error when the assignment dialog returns an unexpected button identifier. {button_id} is numeric and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "assign_shader_policies.error.begin_transaction",
        english_text: "Unable to begin new-policy assignment transaction: {error}",
        translator_context: "Error starting the database transaction used to create new shader policies. {error} is supplied database error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "assign_shader_policies.error.recheck_policies",
        english_text: "Unable to recheck policies for '{filename}': {error}",
        translator_context: "Error rechecking whether a shader already has policies before creating new ones. {filename} and {error} are supplied values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "assign_shader_policies.error.commit_transaction",
        english_text: "Unable to commit new-policy assignment transaction: {error}",
        translator_context: "Error committing the database transaction used to create new shader policies. {error} is supplied database error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "assign_shader_policies.error.create_policy",
        english_text: "Unable to create {target} policy for '{filename}': {error}",
        translator_context: "Error creating a generated shader policy. {target} is the stored technical target token; {filename} and {error} are supplied values and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "assign_shader_policies.error.validate_policy_name",
        english_text: "Unable to validate generated Policy Name '{policy_name}' for target {target}: {error}",
        translator_context: "Error checking whether a generated Policy Name is available. {policy_name} is generated from user shader data; {target} is the stored technical target token; {error} is supplied database error text.",
    },
    FactoryTranslationKey {
        key: "assign_shader_policies.error.generate_policy_name",
        english_text: "Unable to generate an available suggested Policy Name for '{filename}' in target {target}",
        translator_context: "Error after exhausting generated Policy Name candidates. {filename} is user shader data and {target} is the stored technical target token; both must remain unchanged.",
    },



    FactoryTranslationKey {
        key: "authentication.error.initialize_pam",
        english_text: "Unable to initialize PAM service '{service}': {error}",
        translator_context: "Authentication error when Screenshaver cannot initialize its PAM service. {service} is the technical PAM service name and {error} is externally supplied PAM/system error text; both must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "authentication.error.configure_failure_delay",
        english_text: "Unable to configure PAM failure delay: {error}",
        translator_context: "Authentication error when Screenshaver cannot configure the Linux-PAM failure delay. {error} is externally supplied PAM/system error text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "authentication.error.pam_authentication",
        english_text: "PAM authentication error: {error}",
        translator_context: "Unexpected PAM authentication error. {error} is externally supplied PAM/system error text and must remain unchanged.",
    },



    FactoryTranslationKey {
        key: "compile_shader.kind.vertex",
        english_text: "Vertex",
        translator_context: "User-facing shader-stage name used in shader compilation errors.",
    },
    FactoryTranslationKey {
        key: "compile_shader.kind.fragment",
        english_text: "Fragment",
        translator_context: "User-facing shader-stage name used in shader compilation errors.",
    },
    FactoryTranslationKey {
        key: "compile_shader.kind.unknown",
        english_text: "Unknown",
        translator_context: "User-facing fallback shader-stage name used in shader compilation errors.",
    },
    FactoryTranslationKey {
        key: "compile_shader.error.create_shader_object",
        english_text: "Unable to create OpenGL {kind} shader object",
        translator_context: "Error when OpenGL cannot create a shader object. {kind} is a localized shader-stage name. OpenGL remains unchanged.",
    },
    FactoryTranslationKey {
        key: "compile_shader.error.interior_null",
        english_text: "{kind} shader source contained an interior null byte",
        translator_context: "Error when shader source contains an interior null byte. {kind} is a localized shader-stage name.",
    },
    FactoryTranslationKey {
        key: "compile_shader.error.create_program_object",
        english_text: "Unable to create OpenGL shader program object",
        translator_context: "Error when OpenGL cannot create a shader program object. OpenGL remains unchanged.",
    },
    FactoryTranslationKey {
        key: "compile_shader.error.link_no_diagnostic",
        english_text: "Shader program linking failed without an OpenGL diagnostic",
        translator_context: "Shader-program link failure when OpenGL supplies no diagnostic. OpenGL remains unchanged.",
    },
    FactoryTranslationKey {
        key: "compile_shader.error.link_failed",
        english_text: "Shader program linking failed:\n{error}",
        translator_context: "Shader-program link failure followed by the OpenGL driver diagnostic. {error} is externally supplied OpenGL diagnostic text and must remain unchanged.",
    },
    FactoryTranslationKey {
        key: "compile_shader.error.compile_no_diagnostic",
        english_text: "{kind} shader compilation failed without an OpenGL diagnostic",
        translator_context: "Shader compilation failure when OpenGL supplies no diagnostic. {kind} is a localized shader-stage name; OpenGL remains unchanged.",
    },
    FactoryTranslationKey {
        key: "compile_shader.error.compile_failed",
        english_text: "{kind} shader compilation failed:\n{error}",
        translator_context: "Shader compilation failure followed by the OpenGL driver diagnostic. {kind} is a localized shader-stage name; {error} is externally supplied OpenGL diagnostic text and must remain unchanged.",
    },

    FactoryTranslationKey {
        key: "qbe.query",
        english_text: "Query",
        translator_context: "Button that executes the current Query By Example criteria.",
    },

    FactoryTranslationKey {
        key: "qbe.clear",
        english_text: "Clear",
        translator_context: "Button that clears the current Query By Example criteria.",
    },

    FactoryTranslationKey {
        key: "qbe.boolean.true",
        english_text: "True",
        translator_context: "Displayed boolean true value in Query By Example; stored query token remains true.",
    },

    FactoryTranslationKey {
        key: "qbe.boolean.false",
        english_text: "False",
        translator_context: "Displayed boolean false value in Query By Example; stored query token remains false.",
    },

    FactoryTranslationKey {
        key: "qbe.status.ok",
        english_text: "OK",
        translator_context: "Displayed valid shader status in Query By Example; stored query value remains OK.",
    },

    FactoryTranslationKey {
        key: "qbe.status.rejected",
        english_text: "Rejected",
        translator_context: "Displayed rejected shader status in Query By Example; stored query value remains Rejected.",
    },

    FactoryTranslationKey {
        key: "qbe.status.compile_error",
        english_text: "Compile Error",
        translator_context: "Displayed shader compile-error status in Query By Example; stored query value remains Compile Error.",
    },

    FactoryTranslationKey {
        key: "qbe.status.missing",
        english_text: "Missing",
        translator_context: "Displayed missing shader status in Query By Example; stored query value remains Missing.",
    },

    FactoryTranslationKey {
        key: "qbe.status.unreadable",
        english_text: "Unreadable",
        translator_context: "Displayed unreadable shader status in Query By Example; stored query value remains Unreadable.",
    },

    FactoryTranslationKey {
        key: "preview.target_not_found",
        english_text: "Shader file or directory not found: {path}",
        translator_context: "Control Center error when a supplied shader preview target does not exist. {path} is a filesystem path and must remain unchanged.",
    },



    FactoryTranslationKey {
        key: "tray.tooltip.waiting_for_idle",
        english_text: "Waiting for idle...",
        translator_context: "System tray tooltip shown while Screenshaver is waiting for the desktop idle timeout.",
    },

    FactoryTranslationKey {
        key: "tray.status.enabled",
        english_text: "Enabled",
        translator_context: "System tray status indicating that a Screenshaver feature is enabled.",
    },

    FactoryTranslationKey {
        key: "tray.status.disabled",
        english_text: "Disabled",
        translator_context: "System tray status indicating that a Screenshaver feature is disabled.",
    },

    FactoryTranslationKey {
        key: "tray.status.starting",
        english_text: "Starting...",
        translator_context: "System tray wallpaper status while the wallpaper renderer is starting.",
    },

    FactoryTranslationKey {
        key: "tray.menu.screensaver_status",
        english_text: "Screensaver: {status}",
        translator_context: "Read-only system tray menu row showing whether the screensaver is enabled. {status} is an already localized status label.",
    },

    FactoryTranslationKey {
        key: "tray.menu.wallpaper",
        english_text: "Wallpaper:",
        translator_context: "Read-only system tray menu heading preceding the current wallpaper status or policy name.",
    },

    FactoryTranslationKey {
        key: "tray.menu.edit",
        english_text: "Edit",
        translator_context: "System tray command that opens the Screenshaver Control Center for editing configuration.",
    },

    FactoryTranslationKey {
        key: "tray.menu.restart",
        english_text: "Restart",
        translator_context: "System tray command that restarts Screenshaver.",
    },

    FactoryTranslationKey {
        key: "tray.menu.stop",
        english_text: "Stop",
        translator_context: "System tray command that stops Screenshaver.",
    },

];
