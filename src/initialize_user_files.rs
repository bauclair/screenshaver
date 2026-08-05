use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const DEFAULT_CONFIG: &str = r#"# Screenshaver configuration

################################
# APPEARANCE
################################
[appearance]
  show_splash = true       # Show Screenshaver splash screen when run

################################
# SCREENSAVER MODE
################################
[screensaver]
  enabled = true           # Enable automatic screensaver activation
  subtitles = true         # Show subtitle information at bottom of each shader

# Accepted subtitle placement combinations:
#     top:left
#     top:center
#     top:right
#     bottom:left
#     bottom:center
#     bottom:right
  subtitle_placement = "bottom:center"
# Screensaver shaders are loaded from:
#     ~/.config/screenshaver/screensavers/

# Displays a single predefined shader.
# mode = "single:default.glsl"

# Displays a shader at random every <nn> seconds.
  mode = "random:60"

# Displays shaders in alphanumerical order by filename every <nn> seconds.
# mode = "ordered:10"

# Idle time before the screensaver activates.
# Accepted suffixes are (s)ec, (m)in, and (h)rs.
  idle_timeout = "10m"

# Default animation speed for screensaver shaders.
  global_speed = 1.0

# Default texture and palette policy for screensaver shaders.
# These values may differ from the wallpaper defaults.
  global_texture = "random"
  global_palette = "random"

################################
# WALLPAPER MODE
################################
[wallpaper]
  enabled = false          # Enable wallpaper rendering mode
# Wallpaper shaders are loaded from:
#     ~/.config/screenshaver/wallpapers/

# Displays a single predefined wallpaper shader.
# mode = "single:default.glsl"

# Displays a wallpaper shader at random every <nn> seconds.
# mode = "random:60"

# Displays wallpaper shaders in alphanumerical order by filename
# every <nn> seconds.
  mode = "ordered:10"

# Default animation speed for wallpaper shaders.
  global_speed = 0.025

# Default texture and palette policy for wallpaper shaders.
# These values may differ from the screensaver defaults.
  global_texture = "random"
  global_palette = "random"

# Initial multi-monitor support renders the same shader independently on
# every monitor. Each monitor uses its own native resolution while sharing
# the shader, timeline, rotation schedule, textures, palettes, and policies.
#
# Supported values:
#     mirror
  monitor_mode = "mirror"

# Use desktop notifications for wallpaper shader changes and sustained
# performance warnings.
  notifications = true

################################
# TEXTURES AND PALETTES
################################
# Background textures available for compatible texture-based shaders:
#
#     bricks             Brick/block wall textures.
#     cells              Voronoi / lichen textures.
#     clouds             Soft cloud and smoke textures.
#     facets             Tetrahedral textures.
#     hexagons           Hexagonal textures.
#     marble             Marble and stone textures.
#     mesh               Mesh textures.
#     noise              Procedural noise textures.
#     radial             Radial textures.
#     random             Randomly select a texture family.
#
# Color palettes available for compatible texture-based shaders:
#
#     brick
#     bronze
#     lichen
#     mist
#     sandstone
#     random
#     slate

################################
# PER-SHADER POLICIES
################################
# Policy properties may be written in any order.
# Supported properties are:
#     texture:<family>
#     palette:<palette>
#     fps:<frames-per-second>
#     speed:<animation-multiplier>
#     anti_aliasing:<off|fxaa>
#     dithering:<off|subtle>
#     color_precision:<auto|standard|high>
#
# Property names and post-processing values are case-insensitive and are
# normalized internally to lowercase.
#
# Properties not included in a policy continue to use the active mode's
# global setting or normal random fallback.

[screensaver_policies]
# "CandyWarp.fs" = "texture:bricks palette:mist fps:24 speed:0.5 anti_aliasing:fxaa dithering:subtle color_precision:high"

[wallpaper_policies]
# "CandyWarp.fs" = "fps:16 speed:0.125 anti_aliasing:off dithering:off color_precision:standard"

################################
# POST-PROCESSING
################################
[postprocess]
# Built-in defaults are used when a setting is omitted.
# Supported anti-aliasing values: off, fxaa
  anti_aliasing = "fxaa"
# Supported dithering values: off, subtle
  dithering = "subtle"
# Supported color-precision values:
#     auto      Prefer RGBA16F and fall back to RGBA8 when unavailable.
#     standard  Require RGBA8 render targets.
#     high      Require RGBA16F render targets.
  color_precision = "auto"
# Supported render scaling values are from 0.25 to 2.0.
  render_scale = 1.0

################################
# PERFORMANCE
################################
[performance]
# Frames per second for all shaders in all rendering modes.
  global_rendered_fps = 30


################################
# SCREEN LOCKING
################################
[locking]
  screen_lock = false     # Invoke lock screen when screensaver deactivates

################################
# DEBUGGING
################################
[debug]
  debug_log = true        # Enable screenshaver.log
  log_level = 4

"#;

const DEFAULT_SHADER: &str = r#"#version 330 core

uniform float iTime;
uniform vec3 iResolution;

out vec4 fragColor;

void main()
{
    vec2 uv = gl_FragCoord.xy / iResolution.xy;

    float red = uv.x;
    float green = uv.y;
    float blue = 0.5 + 0.5 * sin(iTime);

    fragColor = vec4(red, green, blue, 1.0);
}
"#;

/// Returns the user's Screenshaver configuration directory.
///
/// Uses:
///
///   $XDG_CONFIG_HOME/screenshaver
///
/// or, when XDG_CONFIG_HOME is unset:
///
///   $HOME/.config/screenshaver
///
pub fn config_directory() -> io::Result<PathBuf> {
    if let Some(xdg_config_home) = env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(xdg_config_home).join("screenshaver"));
    }

    let home = env::var_os("HOME").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Unable to locate the user configuration directory because \
             neither XDG_CONFIG_HOME nor HOME is defined",
        )
    })?;

    Ok(PathBuf::from(home)
        .join(".config")
        .join("screenshaver"))
}

/// Creates a file only when it does not already exist.
///
/// `create_new(true)` guarantees that an existing user file will never be
/// overwritten, even if another process creates it between the existence
/// check and the file-open operation.
fn create_file_if_missing(path: &Path, contents: &str) -> io::Result<bool> {
    if path.exists() {
        return Ok(false);
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => {
            file.write_all(contents.as_bytes())?;
            file.sync_all()?;
            Ok(true)
        }

        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            // Another process created the file after path.exists() was checked.
            Ok(false)
        }

        Err(error) => Err(error),
    }
}

/// Ensures that all required Screenshaver user directories and files exist.
///
/// Existing directories and files are preserved unchanged.
pub fn initialize() -> io::Result<PathBuf> {
    let config_dir = config_directory()?;

    let cache_dir = config_dir.join("cache");
    let rejected_dir = config_dir.join("rejected");
    let screensavers_dir = config_dir.join("screensavers");
    let wallpapers_dir = config_dir.join("wallpapers");

    let config_file = config_dir.join("screenshaver.toml");
    let default_shader_file = screensavers_dir.join("default.glsl");

    // create_dir_all() creates missing parents and succeeds when the
    // directories already exist.
    fs::create_dir_all(&cache_dir)?;
    fs::create_dir_all(&rejected_dir)?;
    fs::create_dir_all(&screensavers_dir)?;
    fs::create_dir_all(&wallpapers_dir)?;

    create_file_if_missing(&config_file, DEFAULT_CONFIG)?;
    create_file_if_missing(&default_shader_file, DEFAULT_SHADER)?;

    Ok(config_dir)
}

