//! Detect the active login display manager and provide distribution-specific guidance.
//! This module never changes the system configuration or invokes privileged commands.
use std::{fs, path::Path, process::Command};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisplayManager { Gdm, Sddm, Lightdm, Other(String), Unknown }

impl DisplayManager {
    pub fn name(&self) -> &str {
        match self { Self::Gdm => "GDM", Self::Sddm => "SDDM", Self::Lightdm => "LightDM", Self::Other(s) => s, Self::Unknown => "Unknown" }
    }
    pub fn is_gdm(&self) -> bool { matches!(self, Self::Gdm) }
}

fn classify(value: &str) -> DisplayManager {
    let value = value.trim().to_ascii_lowercase();
    let basename = value.rsplit('/').next().unwrap_or(&value);
    let name = basename.strip_suffix(".service").unwrap_or(basename);
    match name {
        "gdm" | "gdm3" => DisplayManager::Gdm,
        "sddm" => DisplayManager::Sddm,
        "lightdm" => DisplayManager::Lightdm,
        "" | "display-manager" => DisplayManager::Unknown,
        other => DisplayManager::Other(other.to_string()),
    }
}

/// Only a running systemd display-manager unit is treated as proof of the active DM.
/// A configured default in /etc/X11 is deliberately not used as proof.
pub fn detect() -> DisplayManager {
    let active = Command::new("systemctl")
        .args(["is-active", "--quiet", "display-manager.service"])
        .status().map(|s| s.success()).unwrap_or(false);
    if !active { return DisplayManager::Unknown; }
    if let Ok(output) = Command::new("systemctl")
        .args(["show", "display-manager.service", "--property=FragmentPath", "--value"])
        .output() {
        if output.status.success() {
            let result = classify(&String::from_utf8_lossy(&output.stdout));
            if result != DisplayManager::Unknown { return result; }
        }
    }
    for link in ["/etc/systemd/system/display-manager.service", "/run/systemd/system/display-manager.service"] {
        if let Ok(target) = fs::read_link(link) {
            let result = classify(&target.to_string_lossy());
            if result != DisplayManager::Unknown { return result; }
        }
    }
    DisplayManager::Unknown
}

#[derive(Debug, Clone)]
pub struct Distribution { pub id: String, pub id_like: Vec<String>, pub pretty_name: String }

fn unquote(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len()-1].replace("\\\"", "\"")
    } else { s.to_string() }
}

pub fn distribution() -> Distribution {
    let contents = fs::read_to_string(Path::new("/etc/os-release"))
        .or_else(|_| fs::read_to_string("/usr/lib/os-release")).unwrap_or_default();
    let mut d = Distribution { id: String::new(), id_like: vec![], pretty_name: "Linux".into() };
    for line in contents.lines() {
        if line.starts_with('#') { continue; }
        if let Some((key, value)) = line.split_once('=') {
            let value = unquote(value);
            match key { "ID" => d.id = value.to_ascii_lowercase(),
                "ID_LIKE" => d.id_like = value.split_whitespace().map(|v| v.to_ascii_lowercase()).collect(),
                "PRETTY_NAME" => d.pretty_name = value,
                _ => {} }
        }
    }
    d
}

fn belongs_to(d: &Distribution, ids: &[&str]) -> bool {
    ids.contains(&d.id.as_str()) || d.id_like.iter().any(|id| ids.contains(&id.as_str()))
}

pub fn instructions(d: &Distribution) -> &'static str {
    if d.id == "nixos" {
        "NixOS: Enable GDM in your system's declarative configuration, then rebuild.\nFor example, on releases using this option:\nservices.displayManager.gdm.enable = true;\nDisable the currently selected display manager in the same configuration.\nApply using your normal nixos-rebuild workflow."
    } else if belongs_to(d, &["debian", "ubuntu", "linuxmint", "pop", "elementary", "kali", "raspbian"]) {
        "Install the gdm3 package if necessary, then run:\nsudo dpkg-reconfigure gdm3"
    } else if belongs_to(d, &["opensuse", "suse", "sles"]) {
        "Install GDM if necessary, then select it using your distribution's display-manager configuration.\nOn systems providing the alternative, run:\nsudo update-alternatives --config default-displaymanager"
    } else if belongs_to(d, &["fedora", "rhel", "centos", "rocky", "almalinux", "arch", "manjaro", "endeavouros", "cachyos"]) {
        "Install GDM if necessary, then select its systemd service:\nsudo systemctl enable --force gdm.service\nCheck that the former display manager is no longer enabled."
    } else {
        "Install GDM and consult your distribution's documentation to select it as the default display manager."
    }
}

pub fn warning_message(dm: &DisplayManager, d: &Distribution) -> String {
    format!("Screenshaver's GNOME screen-lock integration requires GDM.\n\nDistribution: {}\nDesktop: GNOME\nDetected active display manager: {}\n\n{}\n\nAfter switching display managers, reboot or start a new login session.\n\nAlternatively, to disable Screenshaver screen locking permanently, edit screenshaver.toml (normally ~/.config/screenshaver/screenshaver.toml) and set:\nscreen_lock_enabled = false\nRestart Screenshaver after saving the file.\n\nScreenshaver screen locking will be disabled for this launch. Screensaver, wallpaper, and windowshader features remain available. Your saved configuration will not change.",
        d.pretty_name, dm.name(), instructions(d))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn managers() {
        assert_eq!(classify("/usr/lib/systemd/system/gdm.service"), DisplayManager::Gdm);
        assert_eq!(classify("sddm.service"), DisplayManager::Sddm);
        assert_eq!(classify("lightdm.service"), DisplayManager::Lightdm);
    }
    #[test] fn guidance() {
        let d = Distribution { id: "debian".into(), id_like: vec![], pretty_name: "Debian".into() };
        assert!(instructions(&d).contains("dpkg-reconfigure gdm3"));
    }
}
