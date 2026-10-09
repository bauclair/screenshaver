//! Centralized, cross-process-safe runtime state storage.
//! All writers must use update_state; do not write state.json independently.
use fs2::FileExt;
use serde_json::{Map, Value};
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
