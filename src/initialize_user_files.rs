use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const DEFAULT_CONFIG: &str = r#"# Screenshaver configuration

[appearance]
  show_splash = true       #Show Screenshaver splash screen when run
  subtitles = true        #Show subtitle info at bottom of each screensaver
#
# Accepted subtitle placement combinations:
# top:left
# top:center
# top:right
# bottom:left
# bottom:center
# bottom:right
  subtitle_placement = "bottom:center"   # Determines subtitle screen placement
#
[operation]
# Displays a single predefined shader.
  mode = "single:default.glsl"
#
# Displays a shader at random every <nn> seconds.
# mode = "random:10"
#
# Displays shaders in alphanumerical order by filename every <nn> seconds.
# mode = "ordered:10"
#
# Idle time in seconds (s)ec, (min) or (h)rs before screensaver activates.
  idle_timeout = "30s"
#
# Background textures available for compatible texture-based shaders.
# Values:
#
#     bricks             Brick/block wall textures.
#     cells              Voronoi / lichen textures.
#     clouds             Soft cloud and smoke textures.
#     hexagons           Hexagonal textures.
#     marble             Marble and stone textures.
#     mesh               Mesh textures.
#     noise              Procedural noise textures.
#     radial             Radial textures.
#
# Color palettes available for compatible texture-based shaders.
# Values:
#
#     brick
#     bronze
#     lichen
#     mist
#     sandstone
#     random (default)
#     slate
#
#
################################
# TEXTURE AND PALETTE OVERRIDES
################################
# ----------------
# Global Overrides
# ----------------
# Override texture and/or palette globally. This forces Screenshaver to
# always use the same texture and palette for shaders that require a texture.
# Without these overrides, textures and/or palettes will be selected
# randomly.
#
# global_texture = "cells:256"
# global_palette = "lichen"
#
# --------------------
# Per-Shader Overrides
# --------------------
# Texture and palette can be defined by [[texture_override]] blocks.
# A [[texture_override]] can be defined for each shader
# requiring a specific background texture and/or palette.
#
# [[texture_override]]
# shader = "Heartfelt.glsl"
# shader_texture = "clouds"
# shader_palette = "mist"
#
##
#
################################
# WALLPAPER MODE
################################
[wallpaper]
# Wallpaper shaders are loaded from:
#     ~/.config/screenshaver/wallpapers/
#
# Existing [operation], [performance], global texture/palette overrides,
# per-shader texture/palette overrides, and FPS overrides also apply to
# wallpaper mode.
#
# Initial multi-monitor support renders the same shader independently on
# every monitor. Each monitor uses its own native resolution while sharing
# the shader, timeline, rotation schedule, textures, palettes, and overrides.
#
# Supported values:
#     mirror
  monitor_mode = "mirror"
#
# Use desktop notifications for wallpaper shader changes and sustained
# performance warnings.
  notifications = true

[performance]
# Frames per second for all shaders
  global_rendered_fps = 30
#
# Frames per second overrides for specific shaders
# [[fps_override]]
# shader = "high_gpu.glsl"
# rendered_fps = 16
#
[locking]
  screen_lock = false     #Invoke lock screen when screensaver deactivates
#
[debug]
 debug_log = true        #Enable screenshaver.log
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
    let shaders_dir = config_dir.join("shaders");
    let wallpapers_dir = config_dir.join("wallpapers");

    let config_file = config_dir.join("screenshaver.toml");
    let default_shader_file = shaders_dir.join("default.glsl");

    // create_dir_all() creates missing parents and succeeds when the
    // directories already exist.
    fs::create_dir_all(&cache_dir)?;
    fs::create_dir_all(&rejected_dir)?;
    fs::create_dir_all(&shaders_dir)?;
    fs::create_dir_all(&wallpapers_dir)?;

    create_file_if_missing(&config_file, DEFAULT_CONFIG)?;
    create_file_if_missing(&default_shader_file, DEFAULT_SHADER)?;

    Ok(config_dir)
}

