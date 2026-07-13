use serde::Deserialize;
use std::fs;
use std::path::Path;


//
// ------------------------------------------------------------
// Default values
// ------------------------------------------------------------
//

fn default_show_splash() -> bool {
    true
}


//
// ------------------------------------------------------------
// Structures that exactly match screenshaver.toml
// ------------------------------------------------------------
//

#[derive(Debug, Deserialize)]
struct AppearanceSection {

    subtitles: bool,

    #[serde(default = "default_show_splash")]
    show_splash: bool,
}


#[derive(Debug, Deserialize)]
struct OperationSection {

    mode: String,

    idle_timeout: String,
}


#[derive(Debug, Deserialize)]
struct LockingSection {

    screen_lock: bool,
}


#[derive(Debug, Deserialize)]
struct DebugSection {

    debug_log: bool,
}


#[derive(Debug, Deserialize)]
struct RawToml {

    appearance: AppearanceSection,

    operation: OperationSection,

    locking: LockingSection,

    debug: DebugSection,
}


//
// ------------------------------------------------------------
// Public structure returned to main.rs
// ------------------------------------------------------------
//

#[derive(Debug)]
pub struct Config {

    pub subtitles: bool,

    pub show_splash: bool,

    pub mode: String,

    pub idle_timeout: String,

    pub screen_lock: bool,

    pub debug_log: bool,
}


//
// ------------------------------------------------------------
// Diagnostic messages
// ------------------------------------------------------------
//

#[derive(Debug)]
pub struct ConfigResult {

    pub config: Config,

    pub diagnostics: Vec<String>,
}


//
// ------------------------------------------------------------
// Load configuration file
// ------------------------------------------------------------
//

pub fn load_config(
    path: &Path,
) -> Result<ConfigResult, String> {

    //---------------------------------------------------------
    // Read file
    //---------------------------------------------------------

    let text =
        fs::read_to_string(path)
            .map_err(|err| {

                format!(
                    "Unable to read configuration file {} ({})",
                    path.display(),
                    err,
                )
            })?;


    //---------------------------------------------------------
    // Parse TOML
    //---------------------------------------------------------

    let raw: RawToml =
        toml::from_str(&text)
            .map_err(|err| {

                format!(
                    "Invalid TOML in {} ({})",
                    path.display(),
                    err,
                )
            })?;


    //---------------------------------------------------------
    // Flatten configuration
    //---------------------------------------------------------

    let config =
        Config {

            subtitles:
                raw.appearance.subtitles,

            show_splash:
                raw.appearance.show_splash,

            mode:
                raw.operation.mode,

            idle_timeout:
                raw.operation.idle_timeout,

            screen_lock:
                raw.locking.screen_lock,

            debug_log:
                raw.debug.debug_log,
        };


    //---------------------------------------------------------
    // Build diagnostics
    //---------------------------------------------------------

    let diagnostics =
        vec![

            format!(
                "[CONFIG] subtitles = {}",
                config.subtitles,
            ),

            format!(
                "[CONFIG] show_splash = {}",
                config.show_splash,
            ),

            format!(
                "[CONFIG] mode = {}",
                config.mode,
            ),

            format!(
                "[CONFIG] idle_timeout = {}",
                config.idle_timeout,
            ),

            format!(
                "[CONFIG] screen_lock = {}",
                config.screen_lock,
            ),

            format!(
                "[CONFIG] debug_log = {}",
                config.debug_log,
            ),
        ];


    Ok(
        ConfigResult {

            config,

            diagnostics,
        }
    )
}