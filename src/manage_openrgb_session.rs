//! Stage 5A-5 checkpoint 3: guarded preflight and read-only OpenRGB SDK transport.
//!
//! This module deliberately performs no OpenRGB network I/O and no lighting
//! writes. It establishes the exclusive-owner and recovery-state prerequisites
//! for a later worker-thread implementation. A successful preflight is NOT an
//! acquired lighting session and must not be treated as one by renderers.

use std::fs::{File, OpenOptions};
use std::path::Path;
use fs2::FileExt;

use crate::manage_runtime_state::{self, AmbientRecoveryRecord};

/// Exclusive cooperative ownership. Keep this object alive through hardware
/// acquisition, all lighting updates, and verified restoration. The stable
/// lock file must never be deleted while Screenshaver is running.
pub struct AmbientOwnerGuard {
    _file: File,
}

impl AmbientOwnerGuard {
    pub fn acquire() -> Result<Self, String> {
        let path = crate::locate_paths::state_path().with_extension("ambient-owner.lock");
        Self::acquire_at(&path)
    }

    fn acquire_at(path: &Path) -> Result<Self, String> {
        let parent = path.parent().ok_or("Ambient ownership lock has no parent directory")?;
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Cannot create ambient lock directory: {error}"))?;
        let file = OpenOptions::new().create(true).read(true).write(true).open(path)
            .map_err(|error| format!("Cannot open ambient owner lock {}: {error}", path.display()))?;
        file.try_lock_exclusive().map_err(|error| format!(
            "Ambient lighting ownership unavailable ({}): {error}", path.display()
        ))?;
        Ok(Self { _file: file })
    }
}

/// A validated, read-only prerequisite for later acquisition. It intentionally
/// does not contain a device handle, TCP socket, or permission to change modes.
pub struct AmbientSessionPreflight {
    _owner: AmbientOwnerGuard,
}

impl AmbientSessionPreflight {
    /// Refuse new device ownership whenever *any* unresolved ambient recovery
    /// exists. This conservative global restriction also prevents collisions
    /// when a device was re-enumerated under a different controller ID.
    pub fn begin() -> Result<Self, String> {
        let owner = AmbientOwnerGuard::acquire()?;
        ensure_no_pending_recovery(&manage_runtime_state::pending_ambient_recoveries()?)?;
        Ok(Self { _owner: owner })
    }
}

fn ensure_no_pending_recovery(records: &[AmbientRecoveryRecord]) -> Result<(), String> {
    if records.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} unresolved OpenRGB recovery record(s); refusing ambient acquisition",
            records.len()
        ))
    }
}

/// Validate the identity of a discovered controller and its current lighting
/// mode under the owner lock, before preparing a durable recovery record.
/// The caller must subsequently re-read and compare the exact original mode
/// bytes before registering the record and sending any mode-changing command.
pub fn verify_pre_acquisition_device(
    selected: &crate::select_ambient_device::AmbientDeviceSelection,
    observed: &crate::select_ambient_device::AmbientDeviceDescriptor,
) -> Result<(), String> {
    crate::select_ambient_device::verify_ambient_device_identity(selected, observed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_pending_recovery_without_modification() {
        let record = AmbientRecoveryRecord {
            endpoint: "127.0.0.1:6742".into(),
            device_name: "Example RGB Device".into(),
            serial: String::new(),
            topology_fingerprint: "fixture-topology".into(),
            ownership_token: "fixture-token".into(),
            original_mode_index: 0,
            original_mode_name: "Wave".into(),
            original_mode_hex: "0102".into(),
        };
        assert!(ensure_no_pending_recovery(&[]).is_ok());
        assert!(ensure_no_pending_recovery(&[record]).is_err());
    }

    #[test]
    fn isolated_lock_rejects_second_owner() {
        let path = std::env::temp_dir().join(format!(
            "screenshaver-ambient-session-test-{}-{:?}.lock",
            std::process::id(), std::thread::current().id()
        ));
        let first = AmbientOwnerGuard::acquire_at(&path).unwrap();
        assert!(AmbientOwnerGuard::acquire_at(&path).is_err());
        drop(first);
        let second = AmbientOwnerGuard::acquire_at(&path).unwrap();
        drop(second);
        std::fs::remove_file(path).unwrap();
    }
}

// Stage 5A-5 checkpoint 3: read-only SDK transport. This is deliberately
// separate from the future mode-changing acquisition transaction.
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

const SDK_PROTOCOL: u32 = 6;
const MAX_SDK_PACKET: usize = 16 * 1024 * 1024;
const SDK_TIMEOUT: Duration = Duration::from_secs(5);

/// Read-only connection to an already running OpenRGB SDK server.
/// No server startup, device rescan, or mode/LED write is performed.
pub struct OpenRgbReadOnlyTransport {
    stream: TcpStream,
}

impl OpenRgbReadOnlyTransport {
    pub fn connect(endpoint: SocketAddr) -> Result<Self, String> {
        let stream = TcpStream::connect_timeout(&endpoint, Duration::from_secs(3))
            .map_err(|e| format!("OpenRGB SDK connection to {endpoint} failed: {e}"))?;
        stream.set_read_timeout(Some(SDK_TIMEOUT)).map_err(|e| e.to_string())?;
        stream.set_write_timeout(Some(SDK_TIMEOUT)).map_err(|e| e.to_string())?;
        let mut transport = Self { stream };
        transport.send_read_only(0, 40, &SDK_PROTOCOL.to_le_bytes())?;
        let (_, response) = transport.receive(40)?;
        if response.len() != 4 {
            return Err("Invalid OpenRGB SDK protocol response".into());
        }
        let version = u32::from_le_bytes(response[..4].try_into().unwrap());
        if version < SDK_PROTOCOL {
            return Err(format!("OpenRGB SDK v{SDK_PROTOCOL} or newer required; server reports v{version}"));
        }
        Ok(transport)
    }

    /// Enumerate controller IDs only. This does not acquire any controller.
    pub fn controller_ids(&mut self) -> Result<Vec<u32>, String> {
        self.send_read_only(0, 0, &[])?;
        let (_, response) = self.receive(0)?;
        parse_controller_ids(&response)
    }

    /// Return the unmodified SDK v6 controller data for a specific ID.
    /// The guarded session must parse and validate this before any write.
    pub fn controller_snapshot(&mut self, controller_id: u32) -> Result<Vec<u8>, String> {
        self.send_read_only(controller_id, 1, &SDK_PROTOCOL.to_le_bytes())?;
        let (returned_id, payload) = self.receive(1)?;
        if returned_id != controller_id {
            return Err("OpenRGB controller ID changed during read-only discovery".into());
        }
        Ok(payload)
    }

    fn send_read_only(&mut self, id: u32, command: u32, data: &[u8]) -> Result<(), String> {
        if !matches!(command, 0 | 1 | 40) {
            return Err("Refusing non-read-only OpenRGB SDK command".into());
        }
        let len = u32::try_from(data.len()).map_err(|_| "SDK request too large")?;
        let mut header = [0u8; 16];
        header[..4].copy_from_slice(b"ORGB");
        header[4..8].copy_from_slice(&id.to_le_bytes());
        header[8..12].copy_from_slice(&command.to_le_bytes());
        header[12..16].copy_from_slice(&len.to_le_bytes());
        self.stream.write_all(&header).and_then(|_| self.stream.write_all(data))
            .map_err(|e| format!("OpenRGB SDK request failed: {e}"))
    }

    fn receive(&mut self, expected: u32) -> Result<(u32, Vec<u8>), String> {
        let deadline = Instant::now() + SDK_TIMEOUT;
        loop {
            if Instant::now() >= deadline {
                return Err(format!("OpenRGB SDK reply {expected} timed out"));
            }
            let mut header = [0u8; 16];
            self.stream.read_exact(&mut header)
                .map_err(|e| format!("OpenRGB SDK reply header failed: {e}"))?;
            if &header[..4] != b"ORGB" {
                return Err("Invalid OpenRGB SDK packet signature".into());
            }
            let id = u32::from_le_bytes(header[4..8].try_into().unwrap());
            let command = u32::from_le_bytes(header[8..12].try_into().unwrap());
            let size = u32::from_le_bytes(header[12..16].try_into().unwrap()) as usize;
            if size > MAX_SDK_PACKET {
                return Err("OpenRGB SDK reply exceeds packet limit".into());
            }
            let mut payload = vec![0u8; size];
            self.stream.read_exact(&mut payload)
                .map_err(|e| format!("OpenRGB SDK reply payload failed: {e}"))?;
            if command == expected {
                return Ok((id, payload));
            }
            if matches!(command, 10 | 51 | 53 | 100 | 1150 | 1200) {
                continue;
            }
            return Err(format!("Unexpected OpenRGB SDK command {command}; expected {expected}"));
        }
    }
}

fn parse_controller_ids(payload: &[u8]) -> Result<Vec<u32>, String> {
    if payload.len() < 4 {
        return Err("Truncated OpenRGB controller list".into());
    }
    let count = u32::from_le_bytes(payload[..4].try_into().unwrap()) as usize;
    if count > 256 || payload.len() != 4 + count * 4 {
        return Err("Invalid OpenRGB controller list length".into());
    }
    let mut ids = Vec::with_capacity(count);
    for chunk in payload[4..].chunks_exact(4) {
        let id = u32::from_le_bytes(chunk.try_into().unwrap());
        if ids.contains(&id) {
            return Err("Duplicate OpenRGB controller ID".into());
        }
        ids.push(id);
    }
    Ok(ids)
}

#[cfg(test)]
mod transport_tests {
    use super::*;

    #[test]
    fn parses_controller_ids_and_rejects_duplicates() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&2u32.to_le_bytes());
        payload.extend_from_slice(&3u32.to_le_bytes());
        payload.extend_from_slice(&7u32.to_le_bytes());
        assert_eq!(parse_controller_ids(&payload).unwrap(), vec![3, 7]);
        payload[8..12].copy_from_slice(&3u32.to_le_bytes());
        assert!(parse_controller_ids(&payload).is_err());
    }

    #[test]
    fn rejects_truncated_or_oversized_controller_lists() {
        assert!(parse_controller_ids(&[]).is_err());
        assert!(parse_controller_ids(&1u32.to_le_bytes()).is_err());
        assert!(parse_controller_ids(&257u32.to_le_bytes()).is_err());
    }
}
