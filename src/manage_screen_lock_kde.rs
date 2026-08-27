//! manage_screen_lock_kde.rs
//!
//! Installs and manages Screenshaver's KDE Plasma / KScreenLocker shell
//! integration.
//!
//! This module deliberately does not invoke KScreenLocker. Its responsibility
//! is limited to:
//!
//!   * installing/updating the user-level `org.screenshaver` Plasma shell;
//!   * writing the generated `LockScreen.qml`;
//!   * preserving the pre-Screenshaver Plasma `ShellPackage` selection;
//!   * selecting `org.screenshaver` in `plasmashellrc`; and
//!   * restoring the previous shell package when Screenshaver integration is
//!     disabled.
//!
//! KScreenLocker remains the security and authentication authority.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::construct_lock_screen_kde;
use crate::define_lock_screen_widget::LockScreenWidgetConfig;

const SCREENSHAVER_SHELL_PACKAGE: &str = "org.screenshaver";
const KDE_DEFAULT_SHELL_PACKAGE: &str = "org.kde.plasma.desktop";
const KDE_STATE_FILENAME: &str = "kde_lock_state.json";

const KDE_METADATA_JSON: &str = r#"{
    "KPackageStructure": "Plasma/Shell",
    "KPlugin": {
        "Authors": [
            {
                "Name": "Screenshaver Project"
            }
        ],
        "Description": "Screenshaver KDE lock-screen shell",
        "Id": "org.screenshaver",
        "License": "GPL-3.0-or-later",
        "Name": "Screenshaver Lock Screen",
        "Version": "1.0"
    },
    "X-Plasma-APIVersion": "2"
}
"#;

#[derive(Clone, Debug, PartialEq, Eq)]
struct KdeLockState {
    previous_shell_package: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KdeIntegrationPaths {
    pub shell_package_dir: PathBuf,
    pub metadata_path: PathBuf,
    pub lockscreen_dir: PathBuf,
    pub lockscreen_qml_path: PathBuf,
    pub plasmashellrc_path: PathBuf,
    pub state_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KdeIntegrationStatus {
    pub shell_package_installed: bool,
    pub lockscreen_qml_installed: bool,
    pub active_shell_package: Option<String>,
    pub screenshaver_selected: bool,
    pub previous_shell_package: Option<String>,
}

/// Returns the KDE/Plasma user integration paths.
///
/// XDG locations are honored when present:
///
///   $XDG_DATA_HOME/plasma/shells/org.screenshaver
///   $XDG_CONFIG_HOME/plasmashellrc
///
/// Otherwise the conventional user locations are used:
///
///   ~/.local/share/plasma/shells/org.screenshaver
///   ~/.config/plasmashellrc
pub fn integration_paths() -> io::Result<KdeIntegrationPaths> {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "Unable to manage KDE lock-screen integration because HOME is not defined",
            )
        })?;

    let data_home = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local").join("share"));

    let config_home = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));

    let shell_package_dir = data_home
        .join("plasma")
        .join("shells")
        .join(SCREENSHAVER_SHELL_PACKAGE);

    let lockscreen_dir = shell_package_dir
        .join("contents")
        .join("lockscreen");

    let screenshaver_config_dir = config_home.join("screenshaver");

    Ok(KdeIntegrationPaths {
        metadata_path: shell_package_dir.join("metadata.json"),
        lockscreen_qml_path: lockscreen_dir.join("LockScreen.qml"),
        plasmashellrc_path: config_home.join("plasmashellrc"),
        state_path: screenshaver_config_dir.join(KDE_STATE_FILENAME),
        shell_package_dir,
        lockscreen_dir,
    })
}

/// Installs or updates the Screenshaver Plasma shell and selects it for
/// KScreenLocker.
///
/// The previous `[Shell] ShellPackage` value is preserved only once. If
/// Screenshaver is already selected, the existing saved value remains
/// untouched.
pub fn install(
    config: &LockScreenWidgetConfig,
) -> io::Result<KdeIntegrationStatus> {
    let paths = integration_paths()?;

    fs::create_dir_all(&paths.lockscreen_dir)?;

    fs::write(
        &paths.metadata_path,
        KDE_METADATA_JSON,
    )?;

    construct_lock_screen_kde::write_lock_screen_kde(
        config,
        &paths.lockscreen_qml_path,
    )?;

    let plasmashellrc =
        read_text_if_exists(&paths.plasmashellrc_path)?;

    let current_shell =
        read_ini_value(
            &plasmashellrc,
            "Shell",
            "ShellPackage",
        );

    let mut state =
        load_state(&paths.state_path)?;

    if current_shell.as_deref()
        != Some(SCREENSHAVER_SHELL_PACKAGE)
        && state.previous_shell_package.is_none()
    {
        state.previous_shell_package =
            Some(
                current_shell.unwrap_or_else(|| {
                    KDE_DEFAULT_SHELL_PACKAGE.to_string()
                })
            );

        save_state(
            &paths.state_path,
            &state,
        )?;
    }

    let updated_plasmashellrc =
        set_ini_value(
            &plasmashellrc,
            "Shell",
            "ShellPackage",
            SCREENSHAVER_SHELL_PACKAGE,
        );

    write_text_atomic(
        &paths.plasmashellrc_path,
        &updated_plasmashellrc,
    )?;

    status_from_paths(&paths)
}

/// Refreshes the generated KDE lock-screen QML without changing the current
/// Plasma shell selection or the preserved restoration state.
pub fn refresh(
    config: &LockScreenWidgetConfig,
) -> io::Result<KdeIntegrationStatus> {
    let paths = integration_paths()?;

    fs::create_dir_all(&paths.lockscreen_dir)?;

    fs::write(
        &paths.metadata_path,
        KDE_METADATA_JSON,
    )?;

    construct_lock_screen_kde::write_lock_screen_kde(
        config,
        &paths.lockscreen_qml_path,
    )?;

    status_from_paths(&paths)
}

/// Restores the shell package that was active before Screenshaver took
/// ownership.
///
/// Restoration is intentionally guarded: `plasmashellrc` is changed only when
/// it still selects `org.screenshaver`. If the user or another program has
/// selected a different shell in the meantime, that newer choice is preserved.
///
/// The installed Screenshaver Plasma shell files are left in place so KDE can
/// never be left pointing at a missing shell package during a restore. Package
/// removal can be handled separately after restoration is verified.
pub fn restore() -> io::Result<KdeIntegrationStatus> {
    let paths = integration_paths()?;

    let plasmashellrc =
        read_text_if_exists(&paths.plasmashellrc_path)?;

    let current_shell =
        read_ini_value(
            &plasmashellrc,
            "Shell",
            "ShellPackage",
        );

    let state =
        load_state(&paths.state_path)?;

    if current_shell.as_deref()
        == Some(SCREENSHAVER_SHELL_PACKAGE)
    {
        if let Some(previous_shell) =
            state.previous_shell_package.as_deref()
        {
            let restored =
                set_ini_value(
                    &plasmashellrc,
                    "Shell",
                    "ShellPackage",
                    previous_shell,
                );

            write_text_atomic(
                &paths.plasmashellrc_path,
                &restored,
            )?;
        }
    }

    status_from_paths(&paths)
}

/// Returns the current Screenshaver/KDE integration state without modifying
/// any files.
pub fn status() -> io::Result<KdeIntegrationStatus> {
    let paths = integration_paths()?;
    status_from_paths(&paths)
}

fn status_from_paths(
    paths: &KdeIntegrationPaths,
) -> io::Result<KdeIntegrationStatus> {
    let plasmashellrc =
        read_text_if_exists(&paths.plasmashellrc_path)?;

    let active_shell_package =
        read_ini_value(
            &plasmashellrc,
            "Shell",
            "ShellPackage",
        );

    let state =
        load_state(&paths.state_path)?;

    Ok(KdeIntegrationStatus {
        shell_package_installed:
            paths.metadata_path.is_file(),

        lockscreen_qml_installed:
            paths.lockscreen_qml_path.is_file(),

        screenshaver_selected:
            active_shell_package.as_deref()
                == Some(SCREENSHAVER_SHELL_PACKAGE),

        active_shell_package,

        previous_shell_package:
            state.previous_shell_package,
    })
}

fn read_text_if_exists(
    path: &Path,
) -> io::Result<String> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(contents),

        Err(error)
            if error.kind() == io::ErrorKind::NotFound =>
        {
            Ok(String::new())
        }

        Err(error) => Err(error),
    }
}

fn read_ini_value(
    contents: &str,
    section: &str,
    key: &str,
) -> Option<String> {
    let mut active_section: Option<&str> =
        None;

    for line in contents.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[')
            && trimmed.ends_with(']')
            && trimmed.len() >= 2
        {
            active_section =
                Some(
                    &trimmed[
                        1
                        ..
                        trimmed.len() - 1
                    ]
                );

            continue;
        }

        if active_section != Some(section) {
            continue;
        }

        let Some((
            candidate_key,
            candidate_value,
        )) = trimmed.split_once('=')
        else {
            continue;
        };

        if candidate_key.trim() == key {
            return Some(
                candidate_value
                    .trim()
                    .to_string()
            );
        }
    }

    None
}

fn set_ini_value(
    contents: &str,
    section: &str,
    key: &str,
    value: &str,
) -> String {
    let lines:
        Vec<&str> =
        contents.lines().collect();

    let section_header =
        format!("[{}]", section);

    let mut output:
        Vec<String> =
        Vec::with_capacity(
            lines.len() + 3
        );

    let mut section_found =
        false;

    let mut key_written =
        false;

    let mut inside_target_section =
        false;

    for line in lines {
        let trimmed = line.trim();

        let is_section_header =
            trimmed.starts_with('[')
                && trimmed.ends_with(']')
                && trimmed.len() >= 2;

        if is_section_header {
            if inside_target_section
                && !key_written
            {
                output.push(
                    format!(
                        "{}={}",
                        key,
                        value
                    )
                );

                key_written = true;
            }

            inside_target_section =
                trimmed == section_header;

            if inside_target_section {
                section_found = true;
            }

            output.push(line.to_string());
            continue;
        }

        if inside_target_section {
            if let Some((
                candidate_key,
                _,
            )) = trimmed.split_once('=')
            {
                if candidate_key.trim() == key {
                    if !key_written {
                        output.push(
                            format!(
                                "{}={}",
                                key,
                                value
                            )
                        );

                        key_written = true;
                    }

                    continue;
                }
            }
        }

        output.push(line.to_string());
    }

    if inside_target_section
        && !key_written
    {
        output.push(
            format!(
                "{}={}",
                key,
                value
            )
        );

        key_written = true;
    }

    if !section_found {
        if !output.is_empty()
            && !output
                .last()
                .is_some_and(
                    |line| line.is_empty()
                )
        {
            output.push(String::new());
        }

        output.push(section_header);

        output.push(
            format!(
                "{}={}",
                key,
                value
            )
        );
    }

    let mut result = output.join("\n");

    if !result.ends_with('\n') {
        result.push('\n');
    }

    result
}

fn load_state(
    path: &Path,
) -> io::Result<KdeLockState> {
    let contents =
        match fs::read_to_string(path) {
            Ok(contents) => contents,

            Err(error)
                if error.kind() == io::ErrorKind::NotFound =>
            {
                return Ok(
                    KdeLockState {
                        previous_shell_package:
                            None,
                    }
                );
            }

            Err(error) => {
                return Err(error);
            }
        };

    Ok(
        KdeLockState {
            previous_shell_package:
                parse_json_string_field(
                    &contents,
                    "previous_shell_package",
                ),
        }
    )
}

fn save_state(
    path: &Path,
    state: &KdeLockState,
) -> io::Result<()> {
    let previous =
        state.previous_shell_package
            .as_deref()
            .map(json_escape)
            .map(
                |value| {
                    format!(
                        "\"{}\"",
                        value
                    )
                }
            )
            .unwrap_or_else(|| {
                "null".to_string()
            });

    let contents =
        format!(
            concat!(
                "{{\n",
                "  \"previous_shell_package\": {}\n",
                "}}\n"
            ),
            previous
        );

    write_text_atomic(
        path,
        &contents,
    )
}

fn parse_json_string_field(
    contents: &str,
    field: &str,
) -> Option<String> {
    let field_token =
        format!("\"{}\"", field);

    let field_index =
        contents.find(&field_token)?;

    let after_field =
        &contents[
            field_index + field_token.len()
            ..
        ];

    let colon_index =
        after_field.find(':')?;

    let value =
        after_field[
            colon_index + 1
            ..
        ]
            .trim_start();

    if value.starts_with("null") {
        return None;
    }

    let quoted =
        value.strip_prefix('"')?;

    let mut result = String::new();
    let mut escaped = false;

    for character in quoted.chars() {
        if escaped {
            match character {
                '"' => result.push('"'),
                '\\' => result.push('\\'),
                'n' => result.push('\n'),
                'r' => result.push('\r'),
                't' => result.push('\t'),
                other => result.push(other),
            }

            escaped = false;
            continue;
        }

        match character {
            '\\' => {
                escaped = true;
            }

            '"' => {
                return Some(result);
            }

            other => {
                result.push(other);
            }
        }
    }

    None
}

fn json_escape(
    value: &str,
) -> String {
    let mut escaped =
        String::with_capacity(
            value.len()
        );

    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }

    escaped
}

fn write_text_atomic(
    path: &Path,
    contents: &str,
) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let file_name =
        path.file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "Unable to construct temporary file name for {}",
                        path.display(),
                    ),
                )
            })?;

    let temporary_path =
        path.with_file_name(
            format!(
                ".{}.screenshaver.tmp",
                file_name
            )
        );

    fs::write(
        &temporary_path,
        contents,
    )?;

    fs::rename(
        &temporary_path,
        path,
    )
}
