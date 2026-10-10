//! Stage 5A-5 checkpoint 6: guarded OpenRGB sessions and mock-server restoration tests.
//!
//! Read-only discovery remains available. Explicit guarded acquisition may change
//! modes only after registering durable recovery state. No renderer invokes it.

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

    /// Decode a complete controller snapshot without issuing any SDK writes.
    pub fn decoded_controller(&mut self, controller_id: u32) -> Result<OpenRgbControllerSnapshot, String> {
        parse_controller_snapshot(&self.controller_snapshot(controller_id)?)
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

/// SDK v6 controller snapshot, including the exact raw mode records required
/// for a future verified restoration transaction. No hardware writes occur.
#[derive(Clone, Debug)]
pub struct OpenRgbControllerSnapshot {
    pub name: String,
    pub serial: String,
    pub active: usize,
    pub modes: Vec<(String, Vec<u8>)>,
    pub zone_counts: Vec<u32>,
    pub led_names: Vec<String>,
    pub colors: Vec<[u8; 4]>,
    pub matrix: Vec<u32>,
    pub width: usize,
    pub height: usize,
}

impl OpenRgbControllerSnapshot {
    /// Convert to the hardware-independent selection contract. Controllers
    /// without a single usable matrix remain discoverable but unselectable.
    pub fn descriptor(&self, controller_id: u32) -> crate::select_ambient_device::AmbientDeviceDescriptor {
        let matrix = if self.width > 0 && self.height > 0 &&
            self.width.checked_mul(self.height) == Some(self.matrix.len()) &&
            self.zone_counts.len() == 1 {
            Some(crate::manage_ambient_lighting::LedMatrix {
                columns: self.width, rows: self.height, indices: self.matrix.clone(),
                led_count: self.led_names.len(),
            })
        } else { None };
        crate::select_ambient_device::AmbientDeviceDescriptor {
            controller_id, display_name: self.name.clone(), serial: self.serial.clone(),
            led_count: self.led_names.len(), matrix,
            modes: self.modes.iter().map(|(name, _)| name.clone()).collect(),
            active_mode: self.active,
        }
    }
}

struct ControllerReader<'a> { data: &'a [u8], at: usize }
impl<'a> ControllerReader<'a> {
    fn new(data: &'a [u8]) -> Self { Self { data, at: 0 } }
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.at.checked_add(n).ok_or("Data offset overflow")?;
        let slice = self.data.get(self.at..end).ok_or_else(|| format!("Truncated controller record at offset {}", self.at))?;
        self.at = end;
        Ok(slice)
    }
    fn u16(&mut self) -> Result<u16, String> { Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap())) }
    fn u32(&mut self) -> Result<u32, String> { Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap())) }
    fn i32(&mut self) -> Result<i32, String> { Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap())) }
    fn string(&mut self) -> Result<String, String> {
        let len = self.u16()? as usize;
        if len > 1024 * 1024 { return Err("Implausible string length".into()); }
        Ok(String::from_utf8_lossy(self.take(len)?).trim_end_matches('\0').to_string())
    }
    fn long_string(&mut self) -> Result<String, String> {
        let len = self.u32()? as usize;
        if len > 1024 * 1024 { return Err("Implausible long string length".into()); }
        Ok(String::from_utf8_lossy(self.take(len)?).trim_end_matches('\0').to_string())
    }
    fn matrix(&mut self) -> Result<Option<(u32, u32, usize)>, String> {
        let len = self.u16()? as usize;
        if len == 0 { return Ok(None); }
        if len < 8 || (len - 8) % 4 != 0 { return Err("Malformed LED matrix".into()); }
        let bytes = self.take(len)?;
        let mut r = ControllerReader::new(bytes);
        let height = r.u32()?;
        let width = r.u32()?;
        if (height as u64) * (width as u64) != ((len - 8) / 4) as u64 { return Err("Matrix dimensions mismatch".into()); }
        let mut mapped = 0;
        for _ in 0..((len - 8) / 4) { if r.u32()? != u32::MAX { mapped += 1; } }
        Ok(Some((width, height, mapped)))
    }
    fn mode(&mut self) -> Result<String, String> {
        let name = self.string()?;
        self.take(11 * 4)?;
        let colors = self.u16()? as usize;
        self.take(colors.checked_mul(4).ok_or("Color count overflow")?)?;
        Ok(name)
    }
}

fn parse_controller_snapshot(payload: &[u8]) -> Result<OpenRgbControllerSnapshot, String> {
    let mut r = ControllerReader::new(payload);
    if r.u32()? as usize != payload.len() { return Err("Controller record size mismatch".into()); }
    r.i32()?;
    let name = r.string()?;
    r.string()?; r.string()?; r.string()?;
    let serial = r.string()?;
    r.string()?;
    let count = r.u16()? as usize;
    let active = r.i32()?;
    if count > 512 || active < 0 { return Err("Invalid mode list".into()); }
    let mut modes = Vec::new();
    for _ in 0..count {
        let start = r.at;
        let mode_name = r.mode()?;
        modes.push((mode_name, payload[start..r.at].to_vec()));
    }
    let zones = r.u16()? as usize;
    if zones > 512 { return Err("Invalid zone count".into()); }
    let mut zone_counts = Vec::new();
    let mut matrix = Vec::new();
    let (mut width, mut height) = (0usize, 0usize);
    for _ in 0..zones {
        r.string()?; r.i32()?; r.u32()?; r.u32()?;
        zone_counts.push(r.u32()?);
        let matrix_start = r.at;
        if let Some((w, h, _)) = r.matrix()? {
            if zones == 1 {
                width = w as usize;
                height = h as usize;
                let mut mr = ControllerReader::new(&payload[matrix_start..r.at]);
                let _len = mr.u16()?;
                mr.u32()?; mr.u32()?;
                for _ in 0..width * height { matrix.push(mr.u32()?); }
            }
        }
        let segments = r.u16()? as usize;
        if segments > 2048 { return Err("Invalid segment count".into()); }
        for _ in 0..segments {
            r.string()?; r.i32()?; r.u32()?; r.u32()?; r.matrix()?; r.u32()?;
        }
        r.u32()?; r.i32()?;
        let zmodes = r.u16()? as usize;
        if zmodes > 512 { return Err("Invalid zone mode count".into()); }
        for _ in 0..zmodes { r.mode()?; }
        r.string()?;
    }
    let led_count = r.u16()? as usize;
    if led_count > 10000 { return Err("Invalid LED count".into()); }
    let mut led_names = Vec::new();
    for _ in 0..led_count { led_names.push(r.string()?); }
    let color_count = r.u16()? as usize;
    if color_count > 10000 { return Err("Invalid color count".into()); }
    let mut colors = Vec::new();
    for _ in 0..color_count { colors.push(r.take(4)?.try_into().unwrap()); }
    let alt = r.u16()? as usize;
    for _ in 0..alt { r.string()?; }
    r.u32()?; r.string()?; r.long_string()?;
    if r.at != payload.len() { return Err("Unparsed controller bytes".into()); }
    if active as usize >= modes.len() { return Err("Active OpenRGB mode is out of range".into()); }
    if led_names.len() != colors.len() { return Err("OpenRGB LED and color counts disagree".into()); }
    Ok(OpenRgbControllerSnapshot { name, serial, active: active as usize, modes, zone_counts, led_names, colors, matrix, width, height })
}


#[cfg(test)]
mod controller_snapshot_tests {
    use super::*;

    #[test]
    fn rejects_truncated_and_inconsistent_records() {
        assert!(parse_controller_snapshot(&[]).is_err());
        assert!(parse_controller_snapshot(&[4, 0, 0, 0]).is_err());
    }

    fn short_string(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_le_bytes());
        out.extend_from_slice(value.as_bytes());
    }

    #[test]
    fn decodes_generic_controller_and_preserves_original_mode_bytes() {
        let mut data = vec![0u8; 4];
        data.extend_from_slice(&0i32.to_le_bytes());
        for field in ["Generic LED Device", "vendor", "description", "version", "serial", "location"] {
            short_string(&mut data, field);
        }
        data.extend_from_slice(&2u16.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        let mut original = Vec::new();
        short_string(&mut original, "Wave");
        original.extend_from_slice(&[0u8; 44]);
        original.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&original);
        short_string(&mut data, "Direct");
        data.extend_from_slice(&[0u8; 44]);
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes()); // zones
        data.extend_from_slice(&0u16.to_le_bytes()); // LEDs
        data.extend_from_slice(&0u16.to_le_bytes()); // colors
        data.extend_from_slice(&0u16.to_le_bytes()); // alternative names
        data.extend_from_slice(&0u32.to_le_bytes());
        short_string(&mut data, "");
        data.extend_from_slice(&0u32.to_le_bytes());
        let len = data.len() as u32;
        data[..4].copy_from_slice(&len.to_le_bytes());
        let snapshot = parse_controller_snapshot(&data).unwrap();
        assert_eq!(snapshot.name, "Generic LED Device");
        assert_eq!(snapshot.modes[0].1, original);
        assert_eq!(snapshot.descriptor(5).controller_id, 5);
        assert!(snapshot.descriptor(5).matrix.is_none());
    }
}

// Stage 5A-5 checkpoint 5: guarded, opt-in acquisition and verified restoration.
// This API is deliberately not called by any rendering backend.
use std::thread;

fn snapshot_fingerprint(info: &OpenRgbControllerSnapshot) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    let mut feed = |bytes: &[u8]| {
        for byte in bytes { hash ^= u64::from(*byte); hash = hash.wrapping_mul(0x100000001b3); }
    };
    feed(info.name.as_bytes()); feed(&[0]);
    feed(info.serial.as_bytes()); feed(&[0]);
    feed(&(info.led_names.len() as u64).to_le_bytes());
    for name in &info.led_names {
        feed(&(name.len() as u64).to_le_bytes()); feed(name.as_bytes());
    }
    for count in &info.zone_counts { feed(&count.to_le_bytes()); }
    feed(&(info.width as u64).to_le_bytes());
    feed(&(info.height as u64).to_le_bytes());
    for index in &info.matrix { feed(&index.to_le_bytes()); }
    format!("fnv1a64-v1-{hash:016x}")
}

fn mode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes { out.push(HEX[(byte >> 4) as usize] as char); out.push(HEX[(byte & 15) as usize] as char); }
    out
}

fn new_ownership_token() -> Result<String, String> {
    let mut random = [0u8; 32];
    File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut random))
        .map_err(|error| format!("Cannot obtain recovery ownership token: {error}"))?;
    Ok(mode_hex(&random))
}

fn matching_identity(info: &OpenRgbControllerSnapshot, record: &AmbientRecoveryRecord) -> Result<(), String> {
    if info.name != record.device_name || info.serial != record.serial ||
        snapshot_fingerprint(info) != record.topology_fingerprint {
        return Err("OpenRGB controller identity or topology changed; retaining recovery record".into());
    }
    let index = record.original_mode_index as usize;
    if info.modes.get(index).map(|(name, bytes)|
        name == &record.original_mode_name && mode_hex(bytes) == record.original_mode_hex
    ) != Some(true) {
        return Err("Original OpenRGB mode bytes changed; retaining recovery record".into());
    }
    Ok(())
}

fn direct_mode_index(info: &OpenRgbControllerSnapshot) -> Result<usize, String> {
    let direct: Vec<usize> = info.modes.iter().enumerate()
        .filter(|(_, (name, _))| name.eq_ignore_ascii_case("Direct"))
        .map(|(index, _)| index).collect();
    if direct.len() != 1 { return Err("OpenRGB controller must have exactly one Direct mode".into()); }
    Ok(direct[0])
}

impl OpenRgbReadOnlyTransport {
    /// Only this private function may send a mode-changing SDK command.
    /// The public transport API remains read-only.
    fn send_guarded_mode(&mut self, id: u32, command: u32, data: &[u8]) -> Result<(), String> {
        if !matches!(command, 1100 | 1101) {
            return Err("Refusing unsupported OpenRGB mode command".into());
        }
        let size = u32::try_from(data.len()).map_err(|_| "OpenRGB mode payload too large")?;
        let mut header = [0u8; 16];
        header[..4].copy_from_slice(b"ORGB");
        header[4..8].copy_from_slice(&id.to_le_bytes());
        header[8..12].copy_from_slice(&command.to_le_bytes());
        header[12..16].copy_from_slice(&size.to_le_bytes());
        self.stream.write_all(&header).and_then(|_| self.stream.write_all(data))
            .map_err(|error| format!("OpenRGB mode command failed: {error}"))
    }
}

/// Owns a controller until `restore` has verified the original mode.
/// Dropping this value does NOT clear recovery state or claim restoration.
/// Do not introduce asynchronous LED writers without joining them first.
pub struct GuardedOpenRgbSession {
    _owner: AmbientSessionPreflight,
    transport: OpenRgbReadOnlyTransport,
    controller_id: u32,
    record: AmbientRecoveryRecord,
}

impl GuardedOpenRgbSession {
    /// Explicit opt-in only; not wired into any renderer or automatic startup.
    /// Registers durable recovery state before attempting the Direct mode switch.
    pub fn acquire(endpoint: SocketAddr,
        selected: &crate::select_ambient_device::AmbientDeviceSelection,
    ) -> Result<Self, String> {
        let owner = AmbientSessionPreflight::begin()?;
        let mut transport = OpenRgbReadOnlyTransport::connect(endpoint)?;
        let ids = transport.controller_ids()?;
        if !ids.contains(&selected.controller_id) {
            return Err("Selected OpenRGB controller is no longer present".into());
        }
        let snapshot = transport.decoded_controller(selected.controller_id)?;
        verify_pre_acquisition_device(selected, &snapshot.descriptor(selected.controller_id))?;
        let direct = direct_mode_index(&snapshot)?;
        if snapshot.active == direct {
            return Err("Cannot acquire a controller already in Direct mode".into());
        }
        let (original_name, original_bytes) = snapshot.modes.get(snapshot.active)
            .ok_or("OpenRGB original mode is missing")?;
        let record = AmbientRecoveryRecord {
            endpoint: endpoint.to_string(), device_name: snapshot.name.clone(),
            serial: snapshot.serial.clone(),
            topology_fingerprint: snapshot_fingerprint(&snapshot),
            ownership_token: new_ownership_token()?,
            original_mode_index: u32::try_from(snapshot.active).map_err(|e| e.to_string())?,
            original_mode_name: original_name.clone(),
            original_mode_hex: mode_hex(original_bytes),
        };
        // Re-read immediately before recording; the mode and topology must be unchanged.
        let fresh = transport.decoded_controller(selected.controller_id)?;
        matching_identity(&fresh, &record)?;
        if fresh.active != snapshot.active || direct_mode_index(&fresh)? != direct {
            return Err("OpenRGB active mode changed during acquisition preflight".into());
        }
        // A second controller with indistinguishable identity cannot be restored safely.
        for id in ids.into_iter().filter(|id| *id != selected.controller_id) {
            let other = transport.decoded_controller(id)?;
            if other.name == record.device_name && other.serial == record.serial &&
                snapshot_fingerprint(&other) == record.topology_fingerprint {
                return Err("Ambiguous identical OpenRGB controllers; refusing acquisition".into());
            }
        }
        manage_runtime_state::register_ambient_recovery(&record)?;
        let mut session = Self { _owner: owner, transport, controller_id: selected.controller_id, record };
        let switched = (|| -> Result<(), String> {
            session.transport.send_guarded_mode(session.controller_id, 1100, &[])?;
            thread::sleep(Duration::from_millis(300));
            let after = session.transport.decoded_controller(session.controller_id)?;
            matching_identity(&after, &session.record)?;
            if after.active != direct_mode_index(&after)? {
                return Err("Direct mode was not verified".into());
            }
            Ok(())
        })();
        if let Err(error) = switched {
            return match session.restore() {
                Ok(()) => Err(format!("OpenRGB acquisition failed; original mode restored: {error}")),
                Err(restore_error) => Err(format!("OpenRGB acquisition failed: {error}; restoration unverified: {restore_error}; recovery record retained")),
            };
        }
        Ok(session)
    }

    /// Must be called after any future LED-update worker has stopped and joined.
    /// Recovery record is deleted only after a fresh read confirms restoration.
    pub fn restore(mut self) -> Result<(), String> {
        restore_mode_verified(&mut self.transport, self.controller_id, &self.record)?;
        manage_runtime_state::complete_ambient_recovery(
            &self.record.device_identity(), &self.record.ownership_token)
    }
}

/// Restore and verify the original mode without modifying durable recovery state.
/// Separating this operation permits realistic mock-server failure testing.
/// The caller alone may clear the record, and only after this returns `Ok`.
fn restore_mode_verified(
    transport: &mut OpenRgbReadOnlyTransport,
    controller_id: u32,
    record: &AmbientRecoveryRecord,
) -> Result<(), String> {
        let before = transport.decoded_controller(controller_id)?;
        matching_identity(&before, &record)?;
        let direct = direct_mode_index(&before)?;
        let original = record.original_mode_index as usize;
        if before.active != original && before.active != direct {
            return Err("Another OpenRGB mode became active; refusing to overwrite it".into());
        }
        if before.active == direct {
            let original_bytes = &before.modes[original].1;
            let size = u32::try_from(8usize + original_bytes.len())
                .map_err(|e| e.to_string())?;
            let mut payload = Vec::with_capacity(size as usize);
            payload.extend_from_slice(&size.to_le_bytes());
            payload.extend_from_slice(&record.original_mode_index.to_le_bytes());
            payload.extend_from_slice(original_bytes);
            transport.send_guarded_mode(controller_id, 1101, &payload)?;
            thread::sleep(Duration::from_millis(500));
        }
        let after = transport.decoded_controller(controller_id)?;
        matching_identity(&after, &record)?;
        if after.active != original {
            return Err("Original OpenRGB lighting mode not verified; recovery record retained".into());
        }
    Ok(())
}

#[cfg(test)]
mod guarded_transaction_tests {
    use super::*;

    fn sample() -> OpenRgbControllerSnapshot {
        OpenRgbControllerSnapshot {
            name: "Generic Device".into(), serial: "sample-serial".into(), active: 0,
            modes: vec![("Wave".into(), vec![1,2,3]), ("Direct".into(), vec![4,5,6])],
            zone_counts: vec![2], led_names: vec!["A".into(), "B".into()],
            colors: vec![[0;4];2], matrix: vec![0,1], width: 2, height: 1,
        }
    }
    #[test]
    fn fingerprint_detects_topology_changes() {
        let first = sample();
        let mut second = first.clone();
        assert_eq!(snapshot_fingerprint(&first), snapshot_fingerprint(&second));
        second.led_names[0] = "Changed".into();
        assert_ne!(snapshot_fingerprint(&first), snapshot_fingerprint(&second));
    }
    #[test]
    fn rejects_changed_original_mode_and_ambiguous_direct_modes() {
        let mut info = sample();
        let record = AmbientRecoveryRecord {
            endpoint: "127.0.0.1:6742".into(), device_name: info.name.clone(),
            serial: info.serial.clone(), topology_fingerprint: snapshot_fingerprint(&info),
            ownership_token: "test".into(), original_mode_index: 0,
            original_mode_name: "Wave".into(), original_mode_hex: mode_hex(&info.modes[0].1),
        };
        assert!(matching_identity(&info, &record).is_ok());
        info.modes[0].1[0] ^= 1;
        assert!(matching_identity(&info, &record).is_err());
        info.modes.push(("direct".into(), vec![]));
        assert!(direct_mode_index(&info).is_err());
    }
}

// Mock-server tests never access state.json or a physical OpenRGB controller.
// They exercise the same wire transport and restoration verification used by
// GuardedOpenRgbSession, with durable-state completion intentionally excluded.
#[cfg(test)]
mod mock_server_recovery_tests {
    use super::*;
    use std::net::{TcpListener, TcpStream};

    fn short(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_le_bytes());
        out.extend_from_slice(value.as_bytes());
    }

    fn mode(name: &str) -> Vec<u8> {
        let mut result = Vec::new();
        short(&mut result, name);
        result.extend_from_slice(&[0; 44]);
        result.extend_from_slice(&0u16.to_le_bytes());
        result
    }

    fn fixture(active: i32, name: &str, original: &[u8]) -> Vec<u8> {
        let mut out = vec![0; 4];
        out.extend_from_slice(&0i32.to_le_bytes());
        for field in [name, "vendor", "description", "version", "serial", "location"] {
            short(&mut out, field);
        }
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&active.to_le_bytes());
        out.extend_from_slice(original);
        out.extend_from_slice(&mode("Direct"));
        out.extend_from_slice(&0u16.to_le_bytes()); // zones
        out.extend_from_slice(&0u16.to_le_bytes()); // LEDs
        out.extend_from_slice(&0u16.to_le_bytes()); // colors
        out.extend_from_slice(&0u16.to_le_bytes()); // alternative names
        out.extend_from_slice(&0u32.to_le_bytes());
        short(&mut out, "");
        out.extend_from_slice(&0u32.to_le_bytes());
        let size = out.len() as u32;
        out[..4].copy_from_slice(&size.to_le_bytes());
        out
    }

    fn recv_packet(stream: &mut TcpStream) -> (u32, u32, Vec<u8>) {
        let mut header = [0u8; 16];
        stream.read_exact(&mut header).unwrap();
        assert_eq!(&header[..4], b"ORGB");
        let id = u32::from_le_bytes(header[4..8].try_into().unwrap());
        let cmd = u32::from_le_bytes(header[8..12].try_into().unwrap());
        let size = u32::from_le_bytes(header[12..16].try_into().unwrap()) as usize;
        assert!(size < 1024 * 1024);
        let mut data = vec![0; size];
        stream.read_exact(&mut data).unwrap();
        (id, cmd, data)
    }

    fn reply(stream: &mut TcpStream, id: u32, cmd: u32, data: &[u8]) {
        stream.write_all(b"ORGB").unwrap();
        stream.write_all(&id.to_le_bytes()).unwrap();
        stream.write_all(&cmd.to_le_bytes()).unwrap();
        stream.write_all(&(data.len() as u32).to_le_bytes()).unwrap();
        stream.write_all(data).unwrap();
    }

    #[derive(Clone, Copy)]
    enum Outcome { Success, Unchanged, WrongDevice, ChangedMode, Disconnect }

    fn exercise(outcome: Outcome) -> (Result<(), String>, bool) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = listener.local_addr().unwrap();
        let original = mode("Wave");
        let initial = parse_controller_snapshot(&fixture(1, "Generic Device", &original)).unwrap();
        let record = AmbientRecoveryRecord {
            endpoint: endpoint.to_string(), device_name: initial.name.clone(),
            serial: initial.serial.clone(), topology_fingerprint: snapshot_fingerprint(&initial),
            ownership_token: "mock-test-token".into(), original_mode_index: 0,
            original_mode_name: "Wave".into(), original_mode_hex: mode_hex(&original),
        };
        let original_for_server = original.clone();
        let handle = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
            let (_, cmd, _) = recv_packet(&mut socket);
            assert_eq!(cmd, 40);
            reply(&mut socket, 0, 40, &6u32.to_le_bytes());
            let (id, cmd, _) = recv_packet(&mut socket);
            assert_eq!((id, cmd), (3, 1));
            reply(&mut socket, 3, 1, &fixture(1, "Generic Device", &original_for_server));
            if matches!(outcome, Outcome::WrongDevice | Outcome::ChangedMode) {
                // Restore must fail closed before emitting a mode-change command.
                return false;
            }
            let (id, cmd, payload) = recv_packet(&mut socket);
            assert_eq!((id, cmd), (3, 1101));
            let mut expected = Vec::new();
            expected.extend_from_slice(&(8u32 + original_for_server.len() as u32).to_le_bytes());
            expected.extend_from_slice(&0u32.to_le_bytes());
            expected.extend_from_slice(&original_for_server);
            assert_eq!(payload, expected);
            if matches!(outcome, Outcome::Disconnect) { return true; }
            let (id, cmd, _) = recv_packet(&mut socket);
            assert_eq!((id, cmd), (3, 1));
            let active = if matches!(outcome, Outcome::Success) { 0 } else { 1 };
            reply(&mut socket, 3, 1, &fixture(active, "Generic Device", &original_for_server));
            true
        });
        let mut transport = OpenRgbReadOnlyTransport::connect(endpoint).unwrap();
        // For identity and mode-mutation cases, use a separate record that
        // intentionally disagrees with the controller's initial snapshot.
        let mut check_record = record;
        if matches!(outcome, Outcome::WrongDevice) {
            check_record.device_name = "Another Device".into();
        }
        if matches!(outcome, Outcome::ChangedMode) {
            check_record.original_mode_hex = "00".into();
        }
        let result = restore_mode_verified(&mut transport, 3, &check_record);
        drop(transport);
        let sent_restore = handle.join().unwrap();
        (result, sent_restore)
    }

    #[test]
    fn mock_restoration_success_requires_verified_original_mode() {
        let (result, sent) = exercise(Outcome::Success);
        assert!(sent);
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn mock_restoration_failure_is_not_acknowledged() {
        let (result, sent) = exercise(Outcome::Unchanged);
        assert!(sent);
        assert!(result.is_err());
    }

    #[test]
    fn mock_connection_loss_is_not_acknowledged() {
        let (result, sent) = exercise(Outcome::Disconnect);
        assert!(sent);
        assert!(result.is_err());
    }

    #[test]
    fn mock_identity_change_refuses_mode_write() {
        let (result, sent) = exercise(Outcome::WrongDevice);
        assert!(!sent);
        assert!(result.is_err());
    }

    #[test]
    fn mock_original_mode_change_refuses_mode_write() {
        let (result, sent) = exercise(Outcome::ChangedMode);
        assert!(!sent);
        assert!(result.is_err());
    }
}


// Explicit opt-in live shader-to-LED session. No automatic device selection.
// This wrapper ensures that the update worker is joined before mode restoration.
pub struct ShaderLightingSession {
    guarded: Option<GuardedOpenRgbSession>,
    worker: Option<crate::manage_ambient_lighting::LightingWorker>,
    matrix: crate::manage_ambient_lighting::LedMatrix,
    base: Vec<crate::manage_ambient_lighting::LedColor>,
    previous: Option<Vec<crate::manage_ambient_lighting::LedColor>>,
    started: Instant,
    submitted: u64,
}

impl ShaderLightingSession {
    pub fn start(endpoint: SocketAddr, controller_id: u32) -> Result<Self, String> {
        let mut discovery = OpenRgbReadOnlyTransport::connect(endpoint)?;
        let ids = discovery.controller_ids()?;
        if !ids.contains(&controller_id) {
            return Err(format!("OpenRGB controller {controller_id} is unavailable"));
        }
        let snapshot = discovery.decoded_controller(controller_id)?;
        let descriptor = snapshot.descriptor(controller_id);
        let selected = crate::select_ambient_device::select_ambient_device(&[descriptor], controller_id)?;
        let matrix = selected.matrix.clone().ok_or("Selected controller has no matrix")?;
        let base = snapshot.colors.clone();
        // Acquisition registers durable recovery before switching modes.
        let guarded = GuardedOpenRgbSession::acquire(endpoint, &selected)?;
        let worker = match crate::manage_ambient_lighting::LightingWorker::start(
            endpoint, controller_id, selected.led_count,
        ) {
            Ok(worker) => worker,
            Err(error) => {
                return match guarded.restore() {
                    Ok(()) => Err(format!("OpenRGB worker failed; mode restored: {error}")),
                    Err(restore_error) => Err(format!(
                        "OpenRGB worker failed: {error}; restoration unverified: {restore_error}"
                    )),
                };
            }
        };
        Ok(Self {
            guarded: Some(guarded), worker: Some(worker), matrix, base,
            previous: None, started: Instant::now(), submitted: 0,
        })
    }

    pub fn expired(&self) -> bool {
        self.started.elapsed() >= Duration::from_secs(30)
    }

    pub fn submit(&mut self, frame: &crate::manage_ambient_lighting::SampledFrame)
        -> Result<bool, String>
    {
        if self.expired() { return Ok(false); }
        let mut colors = crate::manage_ambient_lighting::map_pixels(
            &frame.rgb, frame.width, frame.height, &self.matrix,
            crate::manage_ambient_lighting::MappingMode::Spatial, &self.base,
        )?;
        crate::manage_ambient_lighting::AmbientBrightness::default().apply(&mut colors)?;
        if let Some(previous) = self.previous.as_mut() {
            crate::manage_ambient_lighting::smooth_colors(previous, &colors, 0.35)?;
            colors = previous.clone();
        } else {
            self.previous = Some(colors.clone());
        }
        let accepted = self.worker.as_mut().ok_or("OpenRGB worker stopped")?
            .try_submit(colors)?;
        if accepted { self.submitted += 1; }
        Ok(accepted)
    }

    fn finish(&mut self) -> Result<(u64, crate::manage_ambient_lighting::WorkerStats), String> {
        // Never attempt restoration until the writer thread has terminated.
        let worker_result = match self.worker.take() {
            Some(worker) => worker.stop(),
            None => Err("OpenRGB worker was already stopped".into()),
        };
        let restoration = match self.guarded.take() {
            Some(guarded) => guarded.restore(),
            None => Err("OpenRGB session was already restored".into()),
        };
        match (worker_result, restoration) {
            (Ok(stats), Ok(())) => Ok((self.submitted, stats)),
            (Err(worker_error), Ok(())) => Err(format!(
                "OpenRGB worker error (original lighting restored): {worker_error}"
            )),
            (Ok(_), Err(error)) => Err(format!(
                "OpenRGB restoration unverified; durable recovery retained: {error}"
            )),
            (Err(worker_error), Err(error)) => Err(format!(
                "OpenRGB worker error: {worker_error}; restoration unverified: {error}"
            )),
        }
    }

    pub fn stop(mut self) -> Result<(u64, crate::manage_ambient_lighting::WorkerStats), String> {
        self.finish()
    }
}

impl Drop for ShaderLightingSession {
    fn drop(&mut self) {
        if self.guarded.is_some() || self.worker.is_some() {
            if let Err(error) = self.finish() {
                eprintln!("[AMBIENT_OPENRGB] {error}");
            }
        }
    }
}
