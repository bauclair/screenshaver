use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const DEFAULT_CONFIG: &str = r#"# Screenshaver configuration

################################
# SCREENSAVER
################################
[screensaver]
  enabled = true           # Enable automatic screensaver activation

################################
# WALLPAPER
################################
[wallpaper]
  enabled = true           # Enable wallpaper rendering mode

# Initial multi-monitor support renders the same shader independently on
# every monitor. Each monitor uses its own native resolution while sharing
# the shader, timeline, rotation schedule, textures, palettes, and policies.
#
# Supported values:
#     mirror
  monitor_mode = "mirror"

################################
# SCREEN LOCKING
################################
[locking]
  # Securely lock the session when the screensaver idle timeout is reached.
  #
  # When enabled, the configured screensaver idle timeout must be
  # at least 60 seconds.
  screen_lock_enabled = false

################################
# DEBUGGING
################################
[debug]
  debug_log = true         # Enable screenshaver.log
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

    let shaders_dir = config_dir.join("shaders");

    let config_file = config_dir.join("screenshaver.toml");
    let default_shader_file = shaders_dir.join("default.glsl");

    // create_dir_all() creates missing parents and succeeds when the
    // directories already exist.
    fs::create_dir_all(&shaders_dir)?;

    create_file_if_missing(&config_file, DEFAULT_CONFIG)?;
    create_file_if_missing(&default_shader_file, DEFAULT_SHADER)?;

    Ok(config_dir)
}

