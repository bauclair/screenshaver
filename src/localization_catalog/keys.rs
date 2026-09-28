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

];
