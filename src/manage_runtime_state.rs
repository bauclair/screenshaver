//! Centralized, cross-process-safe runtime state storage.
//! All writers must use update_state; do not write state.json independently.
use fs2::FileExt;
use serde_json::{Map, Value};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn read_from_disk(path: &Path) -> Result<Value, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok(Value::Object(Map::new()));
        }
        Err(error) => return Err(format!("Cannot read {}: {error}", path.display())),
    };
    let root: Value = serde_json::from_str(&contents)
        .map_err(|error| format!("Invalid runtime state {}: {error}", path.display()))?;
    if !root.is_object() {
        return Err(format!("Runtime state {} must contain a JSON object", path.display()));
    }
    Ok(root)
}

/// Read the latest complete state snapshot. Writers publish via atomic rename.
pub fn read_state() -> Result<Value, String> {
    read_from_disk(&crate::locate_paths::state_path())
}

/// Lock, read, mutate and durably replace the runtime state as one transaction.
/// The stable .lock file must never be deleted during normal operation.
pub fn update_state<F>(update: F) -> Result<(), String>
where
    F: FnOnce(&mut Value) -> Result<(), String>,
{
    let path = crate::locate_paths::state_path();
    let parent = path.parent().ok_or_else(|| "Runtime state has no parent directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Cannot create {}: {error}", parent.display()))?;
    let lock_path = path.with_extension("json.lock");
    let lock = OpenOptions::new().create(true).read(true).write(true).open(&lock_path)
        .map_err(|error| format!("Cannot open state lock {}: {error}", lock_path.display()))?;
    lock.lock_exclusive()
        .map_err(|error| format!("Cannot lock {}: {error}", lock_path.display()))?;
    let result = (|| {
        let mut root = read_from_disk(&path)?;
        update(&mut root)?;
        if !root.is_object() {
            return Err("Runtime state update produced a non-object root".to_string());
        }
        let payload = serde_json::to_vec_pretty(&root)
            .map_err(|error| format!("Cannot serialize runtime state: {error}"))?;
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let name = format!(".state.json.{}.{}.tmp", std::process::id(), sequence);
        let temp_path = parent.join(name);
        let result = (|| {
            let mut temporary = OpenOptions::new().write(true).create_new(true).open(&temp_path)
                .map_err(|error| format!("Cannot create {}: {error}", temp_path.display()))?;
            temporary.write_all(&payload)
                .map_err(|error| format!("Cannot write {}: {error}", temp_path.display()))?;
            temporary.sync_all()
                .map_err(|error| format!("Cannot sync {}: {error}", temp_path.display()))?;
            fs::rename(&temp_path, &path)
                .map_err(|error| format!("Cannot replace {}: {error}", path.display()))?;
            File::open(parent).and_then(|directory| directory.sync_all())
                .map_err(|error| format!("Cannot sync state directory {}: {error}", parent.display()))?;
            Ok(())
        })();
        if result.is_err() { let _ = fs::remove_file(&temp_path); }
        result
    })();
    let unlock_result = FileExt::unlock(&lock)
        .map_err(|error| format!("Cannot unlock {}: {error}", lock_path.display()));
    result.and(unlock_result)
}


// Ambient recovery records are deliberately inert in this stage. No OpenRGB
// connection or device-mode change is performed by this module.
const AMBIENT_RECOVERY_KEY: &str = "ambient_openrgb_recovery";
const MAX_MODE_RECORD_HEX: usize = 128 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AmbientRecoveryRecord {
    /// OpenRGB endpoint (e.g. "127.0.0.1:6742").
    pub endpoint: String,
    /// Human-readable controller name; not sufficient on its own for identity.
    pub device_name: String,
    /// Device serial if provided by OpenRGB (may be empty).
    pub serial: String,
    /// Stable topology fingerprint, calculated from read-only controller data.
    pub topology_fingerprint: String,
    /// Random per-acquisition ownership token supplied by the coordinator.
    pub ownership_token: String,
    /// Original active mode index (used only after re-enumeration/validation).
    pub original_mode_index: u32,
    pub original_mode_name: String,
    /// Exact original OpenRGB mode record encoded as lowercase hex.
    pub original_mode_hex: String,
}

impl AmbientRecoveryRecord {
    pub fn device_identity(&self) -> String {
        // JSON encoding prevents delimiter collisions in identity components.
        serde_json::to_string(&(
            &self.endpoint, &self.device_name, &self.serial,
            &self.topology_fingerprint,
        )).expect("string tuple serialization")
    }

    fn validate(&self) -> Result<(), String> {
        for (name, value) in [
            ("endpoint", &self.endpoint),
            ("device_name", &self.device_name),
            ("topology_fingerprint", &self.topology_fingerprint),
            ("ownership_token", &self.ownership_token),
            ("original_mode_name", &self.original_mode_name),
        ] {
            if value.trim().is_empty() || value.len() > 1024 {
                return Err(format!("Invalid ambient recovery {name}"));
            }
        }
        if self.serial.len() > 1024 {
            return Err("Ambient recovery serial too long".into());
        }
        let hex = self.original_mode_hex.as_bytes();
        if hex.is_empty() || hex.len() > MAX_MODE_RECORD_HEX || hex.len() % 2 != 0
            || !hex.iter().all(u8::is_ascii_hexdigit)
        {
            return Err("Invalid ambient recovery mode record hex".into());
        }
        Ok(())
    }
}

fn decode_recoveries(root: &Value) -> Result<Vec<AmbientRecoveryRecord>, String> {
    match root.get(AMBIENT_RECOVERY_KEY) {
        None => Ok(Vec::new()),
        Some(value) => {
            let records: Vec<AmbientRecoveryRecord> = serde_json::from_value(value.clone())
                .map_err(|e| format!("Invalid ambient recovery records: {e}"))?;
            let mut identities = std::collections::HashSet::new();
            let mut tokens = std::collections::HashSet::new();
            for record in &records {
                record.validate()?;
                if !identities.insert(record.device_identity()) {
                    return Err("Duplicate ambient recovery device identity".into());
                }
                if !tokens.insert(&record.ownership_token) {
                    return Err("Duplicate ambient recovery ownership token".into());
                }
            }
            Ok(records)
        }
    }
}

fn encode_recoveries(root: &mut Value, records: &[AmbientRecoveryRecord]) -> Result<(), String> {
    let object = root.as_object_mut().ok_or("Runtime state root is not an object")?;
    if records.is_empty() {
        object.remove(AMBIENT_RECOVERY_KEY);
    } else {
        object.insert(AMBIENT_RECOVERY_KEY.to_owned(),
            serde_json::to_value(records).map_err(|e| e.to_string())?);
    }
    Ok(())
}

/// Register recovery intent before issuing ANY device-mode-changing command.
/// Existing pending records are never silently replaced or discarded.
pub fn register_ambient_recovery(record: &AmbientRecoveryRecord) -> Result<(), String> {
    record.validate()?;
    update_state(|root| register_recovery_in(root, record))
}

fn register_recovery_in(root: &mut Value, record: &AmbientRecoveryRecord) -> Result<(), String> {
    record.validate()?;
    let mut records = decode_recoveries(root)?;
    if records.iter().any(|existing|
        existing.device_identity() == record.device_identity()
        || existing.ownership_token == record.ownership_token)
    {
        return Err("Ambient recovery record already pending for device or token".into());
    }
    records.push(record.clone());
    encode_recoveries(root, &records)
}

/// Read pending records. This does not contact or alter OpenRGB devices.
pub fn pending_ambient_recoveries() -> Result<Vec<AmbientRecoveryRecord>, String> {
    decode_recoveries(&read_state()?)
}

/// Remove a record only after a caller has independently verified restoration.
/// A stale or mismatched ownership token cannot clear a newer session's record.
pub fn complete_ambient_recovery(
    device_identity: &str,
    ownership_token: &str,
) -> Result<(), String> {
    update_state(|root| complete_recovery_in(root, device_identity, ownership_token))
}

fn complete_recovery_in(
    root: &mut Value, device_identity: &str, ownership_token: &str,
) -> Result<(), String> {
    let mut records = decode_recoveries(root)?;
    let index = records.iter().position(|record|
        record.device_identity() == device_identity
        && record.ownership_token == ownership_token)
        .ok_or("Ambient recovery record or ownership token not found")?;
    records.remove(index);
    encode_recoveries(root, &records)
}

// Test-only fixture entry points: these never access the live state path.
#[cfg(test)]
pub(crate) fn fixture_register_ambient_recovery(
    root: &mut Value, record: &AmbientRecoveryRecord,
) -> Result<(), String> {
    register_recovery_in(root, record)
}

#[cfg(test)]
pub(crate) fn fixture_pending_ambient_recoveries(
    root: &Value,
) -> Result<Vec<AmbientRecoveryRecord>, String> {
    decode_recoveries(root)
}

#[cfg(test)]
pub(crate) fn fixture_complete_ambient_recovery(
    root: &mut Value, identity: &str, token: &str,
) -> Result<(), String> {
    complete_recovery_in(root, identity, token)
}

#[cfg(test)]
mod ambient_recovery_tests {
    use super::*;

    fn record(name: &str, token: &str) -> AmbientRecoveryRecord {
        AmbientRecoveryRecord {
            endpoint: "127.0.0.1:6742".into(), device_name: name.into(),
            serial: String::new(), topology_fingerprint: format!("topology-{name}"),
            ownership_token: token.into(), original_mode_index: 1,
            original_mode_name: "Spectrum Cycle".into(),
            original_mode_hex: "aabbccdd".into(),
        }
    }

    #[test]
    fn multiple_devices_and_unrelated_state_survive() {
        let mut root = serde_json::json!({"ordered": {"last_wallpaper_policy_id": 9}});
        let first = record("Keyboard", "owner-a");
        let second = record("Mouse", "owner-b");
        register_recovery_in(&mut root, &first).unwrap();
        register_recovery_in(&mut root, &second).unwrap();
        assert_eq!(decode_recoveries(&root).unwrap().len(), 2);
        assert_eq!(root["ordered"]["last_wallpaper_policy_id"], 9);
        complete_recovery_in(&mut root, &first.device_identity(), "owner-a").unwrap();
        assert_eq!(decode_recoveries(&root).unwrap(), vec![second]);
    }

    #[test]
    fn duplicate_or_wrong_owner_never_erases_pending() {
        let mut root = serde_json::json!({});
        let first = record("Keyboard", "owner-a");
        register_recovery_in(&mut root, &first).unwrap();
        assert!(register_recovery_in(&mut root, &record("Keyboard", "owner-b")).is_err());
        assert!(register_recovery_in(&mut root, &record("Mouse", "owner-a")).is_err());
        assert!(complete_recovery_in(&mut root, &first.device_identity(), "owner-b").is_err());
        assert_eq!(decode_recoveries(&root).unwrap(), vec![first]);
    }

    #[test]
    fn invalid_records_fail_closed() {
        let mut root = serde_json::json!({});
        let mut bad = record("Keyboard", "owner-a");
        bad.original_mode_hex = "invalid!".into();
        assert!(register_recovery_in(&mut root, &bad).is_err());
        root[AMBIENT_RECOVERY_KEY] = serde_json::json!({"corrupt": true});
        assert!(register_recovery_in(&mut root, &record("Mouse", "owner-b")).is_err());
    }

    #[test]
    fn isolated_fixture_recovery_transaction_preserves_unrelated_keys() {
        // Entirely in-memory disposable state fixture: no live state.json access.
        let mut fixture = serde_json::json!({
            "ui": {"selected_policy": 42},
            "wallpaper": {"running": true}
        });
        let before = fixture.clone();
        let first = record("Keyboard", "token-1");
        register_recovery_in(&mut fixture, &first).unwrap();
        assert_eq!(decode_recoveries(&fixture).unwrap(), vec![first.clone()]);
        assert_eq!(fixture["ui"], before["ui"]);
        assert_eq!(fixture["wallpaper"], before["wallpaper"]);
        let serialized = serde_json::to_vec(&fixture).unwrap();
        let mut restarted: Value = serde_json::from_slice(&serialized).unwrap();
        assert_eq!(decode_recoveries(&restarted).unwrap(), vec![first.clone()]);
        complete_recovery_in(&mut restarted, &first.device_identity(), "token-1").unwrap();
        assert_eq!(restarted, before);
    }

    #[test]
    fn isolated_fixture_failed_completion_is_byte_for_byte_unchanged() {
        let mut fixture = serde_json::json!({"other": [1, 2, 3]});
        let first = record("Keyboard", "token-1");
        register_recovery_in(&mut fixture, &first).unwrap();
        let baseline = serde_json::to_vec(&fixture).unwrap();
        assert!(complete_recovery_in(&mut fixture, &first.device_identity(), "wrong-token").is_err());
        assert!(complete_recovery_in(&mut fixture, "wrong-device", "token-1").is_err());
        assert_eq!(serde_json::to_vec(&fixture).unwrap(), baseline);
    }

    #[test]
    fn isolated_fixture_corruption_and_duplicate_tokens_fail_closed() {
        let first = record("Keyboard", "token-1");
        let second = record("Mouse", "token-1");
        let mut fixture = serde_json::json!({
            "ambient_openrgb_recovery": [first, second], "other": true
        });
        let before = fixture.clone();
        assert!(decode_recoveries(&fixture).is_err());
        assert!(register_recovery_in(&mut fixture, &record("Lamp", "token-3")).is_err());
        assert_eq!(fixture, before);
        fixture[AMBIENT_RECOVERY_KEY] = serde_json::json!([{"missing": "required fields"}]);
        assert!(decode_recoveries(&fixture).is_err());
    }

    #[test]
    fn round_trip_serialization_preserves_original_mode_bytes() {
        let first = record("Keyboard", "owner-a");
        let encoded = serde_json::to_string(&first).unwrap();
        let decoded: AmbientRecoveryRecord = serde_json::from_str(&encoded).unwrap();
        assert_eq!(first, decoded);
    }
}
