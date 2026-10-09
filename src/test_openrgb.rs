//! Guarded physical Cynosa lighting test. Dry-run by default.
 //! --test-openrgb --apply requires typed consent and a Spectrum Cycle baseline.
 //! Never rescans, restarts OpenRGB, or modifies non-Cynosa devices.
use std::io::{Read, Write};
use std::fs::{File, OpenOptions};
use fs2::FileExt;
use crate::manage_runtime_state::{self, AmbientRecoveryRecord};
use std::net::{Ipv4Addr, SocketAddrV4, TcpStream};
use std::time::Duration;
use std::io::{self, BufRead};
use std::thread;
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::time::Instant;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::GLProfile;

const PROTOCOL: u32 = 6;
const MAX_PACKET: usize = 16 * 1024 * 1024;

const TEST_KEYS: [(usize, &str, [u8; 3]); 3] = [
    (1, "Escape", [24, 0, 0]),
    (46, "Q", [0, 24, 0]),
    (68, "A", [0, 0, 24]),
];

// Cooperative lifetime lock: held by physical test from before acquisition until
// after restoration, and by recovery during its entire validation/write cycle.
// This does NOT detect unrelated OpenRGB applications or old Screenshaver builds.
fn ambient_owner_lock() -> Result<File, String> {
    let path = crate::locate_paths::state_path().with_extension("ambient-owner.lock");
    let parent = path.parent().ok_or("Missing ambient lock parent")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let file = OpenOptions::new().create(true).read(true).write(true).open(&path)
        .map_err(|e| format!("Cannot open ambient owner lock {}: {e}", path.display()))?;
    file.try_lock_exclusive().map_err(|e| format!(
        "Ambient lighting owner is active or lock unavailable ({}): {e}; refusing device access",
        path.display()))?;
    Ok(file)
}

fn decode_mode_hex(hex: &str) -> Result<Vec<u8>, String> {
    if hex.is_empty() || hex.len() % 2 != 0 || hex.len() > 128 * 1024 {
        return Err("Invalid saved mode length".into());
    }
    (0..hex.len()).step_by(2).map(|i| {
        u8::from_str_radix(&hex[i..i+2], 16).map_err(|e| e.to_string())
    }).collect()
}

/// Stage 3B: manual, guarded recovery; never scans/restarts OpenRGB.
/// Lock excludes cooperating physical tests, not unrelated OpenRGB clients.
pub fn restore_ambient_recovery() -> Result<(), String> {
    let _owner = ambient_owner_lock()?;
    let records = manage_runtime_state::pending_ambient_recoveries()?;
    if records.is_empty() {
        println!("[AMBIENT RECOVERY] No pending recovery records; nothing to restore.");
        return Ok(());
    }
    if records.len() != 1 {
        return Err(format!("{} records pending; manual recovery requires exactly one", records.len()));
    }
    let record = &records[0];
    if record.endpoint != "127.0.0.1:6742" || record.device_name != "Razer Cynosa Chroma"
        || record.original_mode_name != "Spectrum Cycle" {
        return Err("Recovery record is outside the guarded Cynosa/local endpoint scope".into());
    }
    let address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 6742);
    let mut socket = TcpStream::connect_timeout(&address.into(), Duration::from_secs(3))
        .map_err(|e| format!("OpenRGB unavailable: {e}"))?;
    socket.set_read_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
    socket.set_write_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
    send(&mut socket, 0, 40, &PROTOCOL.to_le_bytes())?;
    let (_, version) = receive(&mut socket, 40)?;
    if version.len() != 4 || u32::from_le_bytes(version[..4].try_into().unwrap()) < PROTOCOL {
        return Err("OpenRGB SDK v6 required".into());
    }
    send(&mut socket, 0, 0, &[])?;
    let (_, response) = receive(&mut socket, 0)?;
    let mut reader = Reader::new(&response);
    let count = reader.u32()? as usize;
    if count > 256 || response.len() != 4 + count * 4 {
        return Err("Invalid controller list".into());
    }
    let mut matches = Vec::new();
    for _ in 0..count {
        let id = reader.u32()?;
        let info = read_target(&mut socket, id)?;
        if info.name == record.device_name {
            matches.push((id, info));
        }
    }
    if matches.len() != 1 {
        return Err(format!("Expected exactly one Cynosa; found {}", matches.len()));
    }
    let (id, info) = matches.remove(0);
    verify_target(&info, record)?;
    let original_index = usize::try_from(record.original_mode_index).map_err(|e| e.to_string())?;
    let original_bytes = decode_mode_hex(&record.original_mode_hex)?;
    if info.modes.get(original_index).map(|(name, bytes)|
        name == &record.original_mode_name && bytes == &original_bytes) != Some(true) {
        return Err("Saved mode record differs from current controller; refusing restoration".into());
    }
    let active = info.modes.get(info.active).map(|m| m.0.as_str())
        .ok_or("Invalid active mode")?;
    if active == record.original_mode_name {
        // No write necessary, but still require explicit user consent to clear stale record.
        println!("[AMBIENT RECOVERY] Original mode already active; record can be cleared after confirmation.");
    } else if active != "Direct" {
        return Err(format!("Active mode {active:?} is neither Direct nor original; refusing to override"));
    }
    println!("[AMBIENT RECOVERY] Verified one Cynosa; current mode={active:?}; original={:?}.", record.original_mode_name);
    println!("[AMBIENT RECOVERY] Other OpenRGB clients cannot be detected. Close other lighting controllers first.");
    println!("Type RESTORE CYNOSA to proceed, or anything else to cancel:");
    let mut answer = String::new();
    io::stdin().lock().read_line(&mut answer).map_err(|e| e.to_string())?;
    if answer.trim() != "RESTORE CYNOSA" {
        println!("[AMBIENT RECOVERY] Cancelled; record unchanged.");
        return Ok(());
    }
    // Shared with the mock transaction tests: validate, restore and verify.
    restore_verified_mode(&mut socket, id, record, &original_bytes)?;
    manage_runtime_state::complete_ambient_recovery(&record.device_identity(), &record.ownership_token)?;
    println!("[AMBIENT RECOVERY] Original mode verified; pending record cleared.");
    Ok(())
}

/// Shared restoration transaction. State cleanup deliberately happens only
/// after this returns Ok; callers retain their record on any failure.
fn restore_verified_mode(
    socket: &mut TcpStream,
    id: u32,
    record: &AmbientRecoveryRecord,
    original_bytes: &[u8],
) -> Result<(), String> {
    let index = usize::try_from(record.original_mode_index).map_err(|e| e.to_string())?;
    let before = read_target(socket, id)?;
    verify_target(&before, record)?;
    if before.modes.get(index).map(|(name, bytes)|
        name == &record.original_mode_name && bytes == original_bytes) != Some(true) {
        return Err("Original mode record changed; refusing restoration".into());
    }
    let active = before.modes.get(before.active).map(|m| m.0.as_str())
        .ok_or("Invalid active mode before restoration")?;
    if active != "Direct" && active != record.original_mode_name {
        return Err("Active mode changed; refusing restoration".into());
    }
    if active == "Direct" {
        let size = u32::try_from(8usize + original_bytes.len())
            .map_err(|e| e.to_string())?;
        let mut payload = Vec::with_capacity(size as usize);
        payload.extend_from_slice(&size.to_le_bytes());
        payload.extend_from_slice(&record.original_mode_index.to_le_bytes());
        payload.extend_from_slice(original_bytes);
        send(socket, id, 1101, &payload)?;
        thread::sleep(Duration::from_millis(500));
    }
    let after = read_target(socket, id)?;
    verify_target(&after, record)?;
    if after.modes.get(after.active).map(|m| m.0.as_str())
        != Some(record.original_mode_name.as_str()) {
        return Err("Restoration not verified; recovery record retained".into());
    }
    Ok(())
}

/// Stage 3A: read-only diagnostics. Never issues OpenRGB lighting or mode writes.
/// A matching device does not establish that a previous owner has exited.
pub fn inspect_ambient_recovery() -> Result<(), String> {
    let records = manage_runtime_state::pending_ambient_recoveries()?;
    println!("[AMBIENT RECOVERY] Read-only inspection; no lighting writes or record changes.");
    if records.is_empty() {
        println!("[AMBIENT RECOVERY] No pending recovery records.");
        return Ok(());
    }
    println!("[AMBIENT RECOVERY] {} pending record(s). Live-owner status is NOT verified.", records.len());
    for (index, record) in records.iter().enumerate() {
        println!("[AMBIENT RECOVERY] Record {}: endpoint={}, device={:?}, serial={:?}, original mode={:?}",
            index + 1, record.endpoint, record.device_name, record.serial, record.original_mode_name);
        // This stage deliberately supports only the local test endpoint.
        // Do not connect to arbitrary endpoints read from state.json.
        if record.endpoint != "127.0.0.1:6742" {
            println!("  UNVERIFIED: endpoint is not the guarded local OpenRGB test endpoint.");
            continue;
        }
        let address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 6742);
        let result = (|| -> Result<(), String> {
            let mut socket = TcpStream::connect_timeout(&address.into(), Duration::from_secs(3))
                .map_err(|e| format!("OpenRGB connection failed: {e}"))?;
            socket.set_read_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
            socket.set_write_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
            send(&mut socket, 0, 40, &PROTOCOL.to_le_bytes())?;
            let (_, version) = receive(&mut socket, 40)?;
            if version.len() != 4 || u32::from_le_bytes(version[..4].try_into().unwrap()) < PROTOCOL {
                return Err("OpenRGB SDK v6 required".into());
            }
            send(&mut socket, 0, 0, &[])?;
            let (_, response) = receive(&mut socket, 0)?;
            let mut reader = Reader::new(&response);
            let count = reader.u32()? as usize;
            if count > 256 || response.len() != 4 + count * 4 {
                return Err("Invalid controller ID list".into());
            }
            let mut matching = Vec::new();
            for _ in 0..count {
                let id = reader.u32()?;
                let info = read_target(&mut socket, id)?;
                if verify_target(&info, record).is_ok() {
                    matching.push((id, info));
                }
            }
            if matching.len() != 1 {
                println!("  UNVERIFIED: {} matching controllers; exactly one required.", matching.len());
                return Ok(());
            }
            let (id, info) = matching.remove(0);
            let original = usize::try_from(record.original_mode_index).map_err(|e| e.to_string())?;
            let mode_matches = info.modes.get(original)
                .map(|(name, raw)| name == &record.original_mode_name && hex_encode(raw) == record.original_mode_hex)
                .unwrap_or(false);
            let active = info.modes.get(info.active).map(|m| m.0.as_str()).unwrap_or("<invalid>");
            println!("  SDK controller ID: {id}; active mode: {active:?}; original mode record matches: {mode_matches}");
            if mode_matches {
                println!("  READ-ONLY MATCH: device topology and original mode match. Ownership remains unknown; NO recovery attempted.");
            } else {
                println!("  UNVERIFIED: original mode index/name/bytes changed; NO recovery attempted.");
            }
            Ok(())
        })();
        if let Err(error) = result {
            println!("  UNVERIFIED: {error}; NO recovery attempted.");
        }
    }
    println!("[AMBIENT RECOVERY] Inspection complete. Pending records remain unchanged.");
    Ok(())
}

pub fn run(apply: bool, animate: bool) -> Result<(), String> {
    let address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 6742);
    println!("[OPENRGB TEST] Connecting to {address}; no device rescans");
    let mut socket = TcpStream::connect_timeout(&address.into(), Duration::from_secs(3))
        .map_err(|e| format!("Cannot connect to OpenRGB: {e}"))?;
    socket.set_read_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
    socket.set_write_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
    send(&mut socket, 0, 40, &6u32.to_le_bytes())?;
    let (_, version) = receive(&mut socket, 40)?;
    if version.len() != 4 || u32::from_le_bytes(version[..4].try_into().unwrap()) < 6 {
        return Err("OpenRGB SDK v6 required".into());
    }
    send(&mut socket, 0, 0, &[])?;
    let (_, response) = receive(&mut socket, 0)?;
    let mut r = Reader::new(&response);
    let count = r.u32()? as usize;
    if count > 256 || response.len() != 4 + count * 4 {
        return Err("Invalid controller ID list".into());
    }
    let mut matches = Vec::new();
    for _ in 0..count {
        let id = r.u32()?;
        send(&mut socket, id, 1, &6u32.to_le_bytes())?;
        let (returned, data) = receive(&mut socket, 1)?;
        if returned != id { return Err("Controller ID mismatch".into()); }
        let info = parse_target(&data)?;
        println!("[OPENRGB TEST] Discovered SDK ID {id}: {:?}, serial {:?}", info.name, info.serial);
        if info.name == "Razer Cynosa Chroma" {
            matches.push((id, info));
        }
    }
    if matches.len() != 1 {
        return Err(format!("Expected exactly one matching Cynosa, found {}", matches.len()));
    }
    let (id, info) = matches.remove(0);
    if info.led_names.len() != 132 || info.colors.len() != 132 ||
        info.zone_counts != [132] || info.active >= info.modes.len() {
        return Err("Unexpected keyboard topology; refusing physical test".into());
    }
    for (index, label, _) in TEST_KEYS {
        if info.led_names[index] != format!("Key: {label}") {
            return Err(format!("LED {index} identity mismatch; refusing physical test"));
        }
    }
    let (original_name, original_raw) = &info.modes[info.active];
    println!("[OPENRGB TEST] Razer Cynosa Chroma: SDK ID {id}, 132 LEDs; serial {:?}", info.serial);
    println!("[OPENRGB TEST] Current mode: {original_name}");
    for (index, label, color) in TEST_KEYS {
        println!("  {label} (LED {index}) -> RGB {color:?}; original {:?}", info.colors[index]);
    }
    if original_name != "Spectrum Cycle" {
        return Err("Safety prerequisite: select Spectrum Cycle in OpenRGB, then rerun. No lighting commands sent.".into());
    }
    if !info.modes.iter().any(|(name, _)| name == "Direct") {
        return Err("Direct mode unavailable".into());
    }
    // Dry runs validate recovery state too, without writing any state or LEDs.
    let endpoint = address.to_string();
    let pending = manage_runtime_state::pending_ambient_recoveries()?;
    if pending.iter().any(|r| r.endpoint == endpoint && r.device_name == info.name) {
        return Err("A Cynosa recovery record is pending; refusing another physical test".into());
    }
    if !apply {
        println!("[OPENRGB TEST] Dry run complete; recovery state clear, no lighting writes. Use --test-openrgb --apply to request physical test.");
        return Ok(());
    }
    println!("{}", if animate { "This will animate shader colors on the Cynosa for up to 30 seconds, then restore Spectrum Cycle." } else { "This will briefly illuminate Escape, Q, A, then attempt to restore Spectrum Cycle." });
    println!("Restoration is best-effort; manual recovery in OpenRGB may be necessary.");
    println!("Type TEST CYNOSA to continue:");
    let mut answer = String::new();
    io::stdin().lock().read_line(&mut answer).map_err(|e| e.to_string())?;
    if answer.trim() != "TEST CYNOSA" {
        println!("[OPENRGB TEST] Cancelled; no lighting commands sent.");
        return Ok(());
    }
    let _owner = ambient_owner_lock()?;
    // Recheck under the owner lock. The user may have waited at the consent
    // prompt while another process changed the keyboard or recovery state.
    // This is intentionally before writing a recovery record or SDK mode.
    let pending_after_lock = manage_runtime_state::pending_ambient_recoveries()?;
    if pending_after_lock.iter().any(|r|
        r.endpoint == endpoint && r.device_name == info.name)
    {
        return Err("A Cynosa recovery record appeared during confirmation; refusing physical test".into());
    }
    let fresh = read_target(&mut socket, id)?;
    verify_physical_test_baseline(&info, &fresh)?;
    println!("[OPENRGB TEST] Revalidated controller, Spectrum Cycle and recovery state under ownership lock.");
    // The test does not take ownership of a controller while any recovery is
    // outstanding for this endpoint/model. Even a different fingerprint can
    // represent the same physical keyboard after firmware/topology changes.
    let fingerprint = topology_fingerprint(&info);
    let token = random_ownership_token()?;
    let record = AmbientRecoveryRecord {
        endpoint,
        device_name: info.name.clone(),
        serial: info.serial.clone(),
        topology_fingerprint: fingerprint,
        ownership_token: token,
        original_mode_index: u32::try_from(info.active).map_err(|e| e.to_string())?,
        original_mode_name: original_name.clone(),
        original_mode_hex: hex_encode(original_raw),
    };
    manage_runtime_state::register_ambient_recovery(&record)?;
    println!("[OPENRGB TEST] Durable recovery record saved before mode change");
    // Capture the original mode record before any write.
    let original_index = info.active;
    let original_record = original_raw.clone();
    let result = (|| -> Result<(), String> {
        println!("[OPENRGB TEST] Switching to Direct mode...");
        send(&mut socket, id, 1100, &[])?;
        thread::sleep(Duration::from_millis(300));
        if read_active_mode(&mut socket, id)? != "Direct" {
            return Err("Direct mode not confirmed; no LED colors transmitted".into());
        }
        if animate {
            // The GL thread never waits on TCP. A capacity-one channel drops
            // obsolete frames instead of building a latency backlog.
            let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(1);
            return thread::scope(|scope| {
                let worker_socket = &mut socket;
                let worker = scope.spawn(move || -> Result<usize, String> {
                    let mut transmitted = 0usize;
                    while let Ok(payload) = rx.recv() {
                        send(worker_socket, id, 1050, &payload)?;
                        transmitted += 1;
                    }
                    Ok(transmitted)
                });
                let render_result = animate_keyboard(&tx, &info);
                drop(tx);
                let worker_result = worker.join()
                    .map_err(|_| "OpenRGB worker thread panicked".to_string())?;
                match worker_result {
                    Ok(transmitted) => {
                        println!("[OPENRGB TEST] Worker transmitted {transmitted} shader-derived frames");
                        render_result
                    }
                    Err(error) => Err(format!("OpenRGB worker failed: {error}; renderer: {render_result:?}")),
                }
            });
        }
        for (index, label, rgb) in TEST_KEYS {
            let mut data = Vec::with_capacity(8);
            data.extend_from_slice(&(index as i32).to_le_bytes());
            data.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 0]);
            send(&mut socket, id, 1052, &data)?;
            println!("[OPENRGB TEST] Set {label}");
            thread::sleep(Duration::from_millis(250));
        }
        println!("[OPENRGB TEST] Holding colors for 3 seconds...");
        thread::sleep(Duration::from_secs(3));
        Ok(())
    })();
    // Always attempt restoration after the first mode-write attempt.
    println!("[OPENRGB TEST] Restoring original Spectrum Cycle mode...");
    let mut restore = Vec::with_capacity(8 + original_record.len());
    restore.extend_from_slice(&(8u32 + original_record.len() as u32).to_le_bytes());
    restore.extend_from_slice(&(original_index as u32).to_le_bytes());
    restore.extend_from_slice(&original_record);
    // Re-read and verify the controller before sending the restore command.
    // A controller ID alone is not an identity across OpenRGB enumerations.
    let restored = (|| -> Result<(), String> {
        let before = read_target(&mut socket, id)?;
        verify_target(&before, &record)?;
        if before.modes.get(original_index).map(|m| m.0.as_str()) != Some(original_name.as_str()) {
            return Err("Original mode index no longer identifies Spectrum Cycle".into());
        }
        send(&mut socket, id, 1101, &restore)?;
        thread::sleep(Duration::from_millis(500));
        let after = read_target(&mut socket, id)?;
        verify_target(&after, &record)?;
        if after.modes.get(after.active).map(|m| m.0.as_str()) != Some(original_name.as_str()) {
            return Err("Restoration not verified: original mode is not active".into());
        }
        manage_runtime_state::complete_ambient_recovery(&record.device_identity(), &record.ownership_token)?;
        println!("[OPENRGB TEST] Spectrum Cycle restored, verified, and recovery record cleared.");
        Ok(())
    })();
    if let Err(ref error) = restored {
        eprintln!("[OPENRGB TEST] RESTORATION WARNING: {error}");
        eprintln!("[OPENRGB TEST] Recovery record retained; manually select Spectrum Cycle in OpenRGB if necessary.");
    }
    match (result, restored) {
        (Err(test), Err(restore)) => Err(format!("Physical test: {test}; restoration: {restore}")),
        (Err(test), Ok(())) => Err(test),
        (Ok(()), Err(restore)) => Err(restore),
        (Ok(()), Ok(())) => Ok(()),
    }
}

fn read_target(socket: &mut TcpStream, id: u32) -> Result<Target, String> {
    send(socket, id, 1, &PROTOCOL.to_le_bytes())?;
    let (returned, data) = receive(socket, 1)?;
    if returned != id { return Err("Controller ID changed".into()); }
    parse_target(&data)
}

// No mode-changing SDK command is permitted unless the post-consent snapshot
// matches the validated baseline, including exact original mode bytes.
fn verify_physical_test_baseline(baseline: &Target, fresh: &Target) -> Result<(), String> {
    if baseline.name != fresh.name || baseline.serial != fresh.serial
        || topology_fingerprint(baseline) != topology_fingerprint(fresh)
        || baseline.active >= baseline.modes.len()
        || fresh.active != baseline.active
        || fresh.modes.len() != baseline.modes.len()
        || fresh.modes != baseline.modes
        || fresh.modes.get(fresh.active).map(|mode| mode.0.as_str()) != Some("Spectrum Cycle")
        || !fresh.modes.iter().any(|mode| mode.0 == "Direct")
    {
        return Err("Cynosa identity, mode table or Spectrum Cycle baseline changed during confirmation; no lighting writes sent".into());
    }
    if fresh.led_names.len() != 132 || fresh.colors.len() != 132
        || fresh.zone_counts != [132]
        || TEST_KEYS.iter().any(|(index, label, _)|
            fresh.led_names.get(*index).map(String::as_str)
                != Some(format!("Key: {label}").as_str()))
    {
        return Err("Cynosa LED layout changed during confirmation; no lighting writes sent".into());
    }
    Ok(())
}

fn verify_target(info: &Target, record: &AmbientRecoveryRecord) -> Result<(), String> {
    if info.name != record.device_name || info.serial != record.serial ||
       topology_fingerprint(info) != record.topology_fingerprint {
        return Err("Controller identity/topology changed; refusing restoration".into());
    }
    Ok(())
}

// Deterministic FNV-1a fingerprint of read-only topology. Not a unique physical
// identifier; matching identical serial-less keyboards remain ambiguous.
fn topology_fingerprint(info: &Target) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    let mut feed = |bytes: &[u8]| {
        for byte in bytes { hash ^= u64::from(*byte); hash = hash.wrapping_mul(0x100000001b3); }
    };
    feed(info.name.as_bytes()); feed(&[0]);
    feed(info.serial.as_bytes()); feed(&[0]);
    feed(&(info.led_names.len() as u64).to_le_bytes());
    for name in &info.led_names { feed(&(name.len() as u64).to_le_bytes()); feed(name.as_bytes()); }
    for count in &info.zone_counts { feed(&count.to_le_bytes()); }
    feed(&(info.width as u64).to_le_bytes());
    feed(&(info.height as u64).to_le_bytes());
    for index in &info.matrix { feed(&index.to_le_bytes()); }
    format!("fnv1a64-v1-{hash:016x}")
}

fn hex_encode(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 15) as usize] as char);
    }
    output
}

fn random_ownership_token() -> Result<String, String> {
    // Linux-only physical test: kernel CSPRNG, not PID or a predictable clock.
    let mut bytes = [0u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut bytes))
        .map_err(|e| format!("Cannot generate recovery ownership token: {e}"))?;
    Ok(hex_encode(&bytes))
}

fn read_active_mode(socket: &mut TcpStream, id: u32) -> Result<String, String> {
    send(socket, id, 1, &6u32.to_le_bytes())?;
    let (returned, data) = receive(socket, 1)?;
    if returned != id { return Err("Controller identity changed".into()); }
    let info = parse_target(&data)?;
    if info.name != "Razer Cynosa Chroma" {
        return Err("Device identity changed".into());
    }
    info.modes.get(info.active).map(|m| m.0.clone())
        .ok_or_else(|| "Invalid active mode".into())
}

struct Target {
    name: String,
    serial: String,
    active: usize,
    modes: Vec<(String, Vec<u8>)>,
    zone_counts: Vec<u32>,
    led_names: Vec<String>,
    colors: Vec<[u8; 4]>,
    matrix: Vec<u32>,
    width: usize,
    height: usize,
}

fn send(stream: &mut TcpStream, device: u32, command: u32, data: &[u8]) -> Result<(), String> {
    if !matches!(command, 0 | 1 | 40 | 1050 | 1052 | 1100 | 1101) { return Err("Refusing unsupported OpenRGB command".into()); }
    let length = u32::try_from(data.len()).map_err(|_| "Request too large")?;
    let mut header = [0u8; 16];
    header[..4].copy_from_slice(b"ORGB");
    header[4..8].copy_from_slice(&device.to_le_bytes());
    header[8..12].copy_from_slice(&command.to_le_bytes());
    header[12..16].copy_from_slice(&length.to_le_bytes());
    stream.write_all(&header).and_then(|_| stream.write_all(data)).map_err(|e| e.to_string())
}

fn receive(stream: &mut TcpStream, expected: u32) -> Result<(u32, Vec<u8>), String> {
    // Animated updates can queue many asynchronous notifications.  Bound the
    // wait by elapsed time rather than by a small fixed notice count.
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if Instant::now() >= deadline {
            return Err(format!("Timed out waiting for SDK command {expected}"));
        }
        let mut header = [0u8; 16];
        stream.read_exact(&mut header).map_err(|e| format!("OpenRGB packet header: {e}"))?;
        if &header[..4] != b"ORGB" { return Err("Invalid SDK packet magic".into()); }
        let id = u32::from_le_bytes(header[4..8].try_into().unwrap());
        let command = u32::from_le_bytes(header[8..12].try_into().unwrap());
        let size = u32::from_le_bytes(header[12..16].try_into().unwrap()) as usize;
        if size > MAX_PACKET { return Err("SDK packet exceeds size limit".into()); }
        let mut payload = vec![0u8; size];
        stream.read_exact(&mut payload).map_err(|e| format!("OpenRGB packet body: {e}"))?;
        if command == expected { return Ok((id, payload)); }
        if matches!(command, 10 | 51 | 53 | 100 | 1150 | 1200) {
            // Notification packets are not replies.  Avoid flooding the console
            // after hundreds of animation frames.
            continue;
        }
        return Err(format!("Unexpected SDK command {command}, expected {expected}"));
    }
}

struct Reader<'a> { data: &'a [u8], at: usize }
impl<'a> Reader<'a> {
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
        let mut r = Reader::new(bytes);
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

fn parse_target(payload: &[u8]) -> Result<Target, String> {
    let mut r = Reader::new(payload);
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
                let mut mr = Reader::new(&payload[matrix_start..r.at]);
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
    Ok(Target { name, serial, active: active as usize, modes, zone_counts, led_names, colors, matrix, width, height })
}

const TEST_FRAGMENT_SHADER: &str = r#"
#version 330 core

uniform float iTime;
uniform vec3 iResolution;

out vec4 FragColor;

vec3 palette(float t)
{
    vec3 a = vec3(0.50, 0.50, 0.50);
    vec3 b = vec3(0.50, 0.50, 0.50);
    vec3 c = vec3(1.00, 1.00, 1.00);
    vec3 d = vec3(0.00, 0.33, 0.67);

    return a + b * cos(6.2831853 * (c * t + d));
}

void main()
{
    vec2 uv =
        (2.0 * gl_FragCoord.xy - iResolution.xy)
        / iResolution.y;

    float r = length(uv);
    float angle = atan(uv.y, uv.x);

    float wave =
        sin(3.0 * angle - 2.2 * iTime + 7.0 * r);

    float glow =
        exp(-2.4 * abs(r - (0.48 + 0.12 * wave)));

    float core =
        exp(-3.2 * r);

    float color_phase =
        0.08 * iTime
        + 0.10 * wave
        + 0.12 * r;

    vec3 color =
        palette(color_phase)
        * (0.20 + 1.25 * glow + 0.55 * core);

    color +=
        0.10
        * palette(color_phase + 0.22)
        * (0.5 + 0.5 * sin(4.0 * r - iTime));

    FragColor =
        vec4(
            max(color, vec3(0.0)),
            1.0
        );
}
"#;
fn animate_keyboard(tx: &SyncSender<Vec<u8>>, info: &Target) -> Result<(), String> {
    if info.width != 22 || info.height != 6 || info.matrix.len() != 132 {
        return Err("Unexpected LED matrix; animation refused".into());
    }
    let sdl = sdl2::init().map_err(|e| e.to_string())?;
    let video = sdl.video().map_err(|e| e.to_string())?;
    {
        let attrs = video.gl_attr();
        attrs.set_context_profile(GLProfile::Core);
        attrs.set_context_version(crate::define_constants::GL_MAJOR, crate::define_constants::GL_MINOR);
    }
    let window = video.window("Screenshaver - OpenRGB Physical Shader Test (Esc to stop)", 960, 540)
        .position_centered().opengl().build().map_err(|e| e.to_string())?;
    let _context = window.gl_create_context().map_err(|e| e.to_string())?;
    gl::load_with(|name| video.gl_get_proc_address(name) as *const _);
    let _ = video.gl_set_swap_interval(1);
    let program = crate::compile_shader::build_program(crate::define_constants::VERTEX_SHADER, TEST_FRAGMENT_SHADER)
        .map_err(|e| format!("Shader compilation failed: {e}"))?;
    let mut vao = 0u32;
    let mut fbo = 0u32;
    let mut texture = 0u32;
    unsafe {
        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);
        gl::GenFramebuffers(1, &mut fbo);
        gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);
        gl::GenTextures(1, &mut texture);
        gl::BindTexture(gl::TEXTURE_2D, texture);
        gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGB8 as i32, 32, 18, 0, gl::RGB, gl::UNSIGNED_BYTE, std::ptr::null());
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, texture, 0);
    }
    let result = (|| -> Result<(), String> {
        let status = unsafe { gl::CheckFramebufferStatus(gl::FRAMEBUFFER) };
        if status != gl::FRAMEBUFFER_COMPLETE { return Err("Shader sample framebuffer incomplete".into()); }
        let time_location = unsafe { gl::GetUniformLocation(program, b"iTime\0".as_ptr() as *const _) };
        let resolution_location = unsafe { gl::GetUniformLocation(program, b"iResolution\0".as_ptr() as *const _) };
        let mut events = sdl.event_pump().map_err(|e| e.to_string())?;
        let start = Instant::now();
        let mut pixels = [0u8; 32 * 18 * 3];
        let mut last = Instant::now() - Duration::from_millis(100);
        let mut frames = 0usize;
        let mut dropped = 0usize;
        while start.elapsed() < Duration::from_secs(30) {
            if events.poll_iter().any(|e| matches!(e, Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. })) { break; }
            unsafe {
                gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);
                gl::Viewport(0, 0, 32, 18);
                gl::UseProgram(program);
                if time_location >= 0 { gl::Uniform1f(time_location, start.elapsed().as_secs_f32()); }
                if resolution_location >= 0 { gl::Uniform3f(resolution_location, 32.0, 18.0, 1.0); }
                gl::DrawArrays(gl::TRIANGLES, 0, 3);
                gl::BindFramebuffer(gl::READ_FRAMEBUFFER, fbo);
                gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, 0);
                gl::Viewport(0, 0, 960, 540);
                gl::BlitFramebuffer(0, 0, 32, 18, 0, 0, 960, 540, gl::COLOR_BUFFER_BIT, gl::LINEAR);
            }
            window.gl_swap_window();
            if last.elapsed() >= Duration::from_millis(100) {
                unsafe {
                    gl::BindFramebuffer(gl::READ_FRAMEBUFFER, fbo);
                    gl::PixelStorei(gl::PACK_ALIGNMENT, 1);
                    gl::ReadPixels(0, 0, 32, 18, gl::RGB, gl::UNSIGNED_BYTE, pixels.as_mut_ptr() as *mut _);
                }
                let mut colors = info.colors.clone();
                for row in 0..info.height {
                    for col in 0..info.width {
                        let index = info.matrix[row * info.width + col];
                        if index == u32::MAX { continue; }
                        let index = index as usize;
                        if index >= colors.len() { return Err("LED matrix index out of range".into()); }
                        let x = ((col as f32 + 0.5) * 32.0 / info.width as f32).floor() as usize;
                        let y = ((info.height - row) as f32 - 0.5) * 18.0 / info.height as f32;
                        let y = (y.floor() as usize).min(17);
                        let offset = (y * 32 + x.min(31)) * 3;
                        colors[index] = [pixels[offset], pixels[offset + 1], pixels[offset + 2], 0];
                    }
                }
                // OpenRGB SDK 1050 (update all controller LEDs) uses a 32-bit block size, followed by
                // a 16-bit color count and then 4 bytes per LED.
                let mut payload = Vec::with_capacity(6 + colors.len() * 4);
                payload.extend_from_slice(&((colors.len() * 4 + 6) as u32).to_le_bytes());
                payload.extend_from_slice(&(colors.len() as u16).to_le_bytes());
                for color in colors { payload.extend_from_slice(&color); }
                match tx.try_send(payload) {
                    Ok(()) => frames += 1,
                    Err(TrySendError::Full(_)) => dropped += 1,
                    Err(TrySendError::Disconnected(_)) => return Err("OpenRGB worker disconnected".into()),
                }
                last = Instant::now();
            }
            thread::sleep(Duration::from_millis(2));
        }
        println!("[OPENRGB TEST] Queued {frames} shader-derived frames; dropped {dropped} stale frames");
        Ok(())
    })();
    unsafe {
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        gl::DeleteTextures(1, &texture);
        gl::DeleteFramebuffers(1, &fbo);
        gl::DeleteVertexArrays(1, &vao);
        gl::DeleteProgram(program);
    }
    result
}


// Consolidated, hardware-free recovery regression tests. These tests bind only
// an ephemeral loopback port and never contact the real OpenRGB SDK endpoint.
#[cfg(test)]
mod openrgb_recovery_regression_tests {
    use super::*;
    use std::net::TcpListener;

    fn record_for(info: &Target) -> AmbientRecoveryRecord {
        AmbientRecoveryRecord {
            endpoint: "127.0.0.1:6742".into(),
            device_name: info.name.clone(),
            serial: info.serial.clone(),
            topology_fingerprint: topology_fingerprint(info),
            ownership_token: "test-only-token".into(),
            original_mode_index: 0,
            original_mode_name: "Spectrum Cycle".into(),
            original_mode_hex: hex_encode(&[1, 2, 3, 4]),
        }
    }

    fn target() -> Target {
        Target {
            name: "Razer Cynosa Chroma".into(),
            serial: "test-serial".into(),
            active: 0,
            modes: vec![("Spectrum Cycle".into(), vec![1, 2, 3, 4]),
                        ("Direct".into(), vec![5, 6, 7, 8])],
            zone_counts: vec![132],
            led_names: (0..132).map(|i| format!("LED {i}")).collect(),
            colors: vec![[0, 0, 0, 0]; 132],
            matrix: vec![0, 1, 2],
            width: 22,
            height: 6,
        }
    }

    fn packet(command: u32, id: u32, body: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"ORGB");
        bytes.extend_from_slice(&id.to_le_bytes());
        bytes.extend_from_slice(&command.to_le_bytes());
        bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
        bytes.extend_from_slice(body);
        bytes
    }

    fn mock_exchange(payload: Vec<u8>, expected: u32) -> Result<(u32, Vec<u8>), String> {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let worker = std::thread::spawn(move || {
            let (mut connection, _) = listener.accept().unwrap();
            connection.write_all(&payload).unwrap();
        });
        let mut client = TcpStream::connect(addr).unwrap();
        client.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let result = receive(&mut client, expected);
        worker.join().unwrap();
        result
    }

    #[test]
    fn mock_transaction_rejects_unexpected_restore_response() {
        // No physical server: deliberately return a mismatched SDK response.
        let response = packet(1101, 0, &[1, 2, 3, 4]);
        assert!(mock_exchange(response, 1).is_err());
    }

    #[test]
    fn mock_transaction_accepts_valid_controller_reply() {
        let response = packet(1, 4, &[9, 8, 7]);
        let (id, payload) = mock_exchange(response, 1).unwrap();
        assert_eq!(id, 4);
        assert_eq!(payload, vec![9, 8, 7]);
    }

    #[test]
    fn isolated_owner_lock_rejects_concurrent_acquisition() {
        // Uses a disposable path; never opens Screenshaver's live owner lock.
        let mut path = std::env::temp_dir();
        path.push(format!("screenshaver-openrgb-lock-test-{}-{:?}",
            std::process::id(), std::thread::current().id()));
        let first = OpenOptions::new().create_new(true).read(true).write(true)
            .open(&path).expect("unique disposable lock fixture");
        let second = OpenOptions::new().read(true).write(true).open(&path).unwrap();
        first.try_lock_exclusive().unwrap();
        assert!(second.try_lock_exclusive().is_err());
        FileExt::unlock(&first).unwrap();
        second.try_lock_exclusive().unwrap();
        FileExt::unlock(&second).unwrap();
        drop(first);
        drop(second);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn recovery_identity_accepts_unchanged_device() {
        let info = target();
        assert!(verify_target(&info, &record_for(&info)).is_ok());
    }

    #[test]
    fn recovery_identity_rejects_changed_serial_and_topology() {
        let info = target();
        let record = record_for(&info);
        let mut changed = target();
        changed.serial = "replacement-device".into();
        assert!(verify_target(&changed, &record).is_err());
        changed = target();
        changed.led_names[3] = "changed".into();
        assert!(verify_target(&changed, &record).is_err());
    }

    #[test]
    fn recovery_mode_hex_roundtrip_and_bad_input() {
        let bytes = [0, 1, 127, 128, 255];
        assert_eq!(decode_mode_hex(&hex_encode(&bytes)).unwrap(), bytes);
        assert!(decode_mode_hex("0").is_err());
        assert!(decode_mode_hex("zz").is_err());
        assert!(decode_mode_hex("").is_err());
    }

    #[test]
    fn mock_server_accepts_expected_response_and_skips_notification() {
        let mut response = packet(10, 0, &[]);
        response.extend_from_slice(&packet(1, 7, &[4, 3, 2, 1]));
        let (id, body) = mock_exchange(response, 1).unwrap();
        assert_eq!(id, 7);
        assert_eq!(body, [4, 3, 2, 1]);
    }

    #[test]
    fn mock_server_rejects_unexpected_command_and_bad_magic() {
        assert!(mock_exchange(packet(1101, 7, &[]), 1).is_err());
        let mut bad = packet(1, 7, &[]);
        bad[0] = b'X';
        assert!(mock_exchange(bad, 1).is_err());
    }

    #[test]
    fn mock_server_rejects_oversized_and_truncated_payload() {
        let mut huge = packet(1, 7, &[]);
        huge[12..16].copy_from_slice(&((MAX_PACKET as u32) + 1).to_le_bytes());
        assert!(mock_exchange(huge, 1).is_err());
        let mut short = packet(1, 7, &[1, 2]);
        short.truncate(short.len() - 1);
        assert!(mock_exchange(short, 1).is_err());
    }

    #[test]
    fn mock_server_rejects_truncated_controller_record() {
        assert!(parse_target(&[0, 0, 0, 0]).is_err());
    }
    // Construct a valid SDK v6 controller record independently of parse_target.
    fn short_string(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_le_bytes());
        out.extend_from_slice(value.as_bytes());
    }

    fn sdk_mode(name: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        short_string(&mut bytes, name);
        bytes.extend_from_slice(&[0; 44]);
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes
    }

    fn sdk_controller(active: i32, serial: &str) -> Vec<u8> {
        let mut bytes = vec![0; 4];
        bytes.extend_from_slice(&0i32.to_le_bytes());
        for field in ["Razer Cynosa Chroma", "vendor", "description", "version", serial, "location"] {
            short_string(&mut bytes, field);
        }
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&active.to_le_bytes());
        bytes.extend_from_slice(&sdk_mode("Spectrum Cycle"));
        bytes.extend_from_slice(&sdk_mode("Direct"));
        bytes.extend_from_slice(&0u16.to_le_bytes()); // zones
        bytes.extend_from_slice(&0u16.to_le_bytes()); // leds
        bytes.extend_from_slice(&0u16.to_le_bytes()); // colors
        bytes.extend_from_slice(&0u16.to_le_bytes()); // alt names
        bytes.extend_from_slice(&0u32.to_le_bytes());
        short_string(&mut bytes, "");
        bytes.extend_from_slice(&0u32.to_le_bytes()); // long string
        let size = bytes.len() as u32;
        bytes[..4].copy_from_slice(&size.to_le_bytes());
        bytes
    }

    fn mock_recovery_case(
        original_active: i32,
        after_active: i32,
        changed_serial: bool,
        expect_write: bool,
    ) -> (Result<(), String>, usize) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
            let mut writes = 0;
            for (phase, active) in [original_active, after_active].into_iter().enumerate() {
                let mut header = [0u8; 16];
                stream.read_exact(&mut header).unwrap();
                assert_eq!(&header[..4], b"ORGB");
                assert_eq!(u32::from_le_bytes(header[8..12].try_into().unwrap()), 1);
                let size = u32::from_le_bytes(header[12..16].try_into().unwrap()) as usize;
                let mut body = vec![0; size];
                stream.read_exact(&mut body).unwrap();
                let serial = if changed_serial { "different" } else { "fixture" };
                stream.write_all(&packet(1, 3, &sdk_controller(active, serial))).unwrap();
                if phase == 0 && original_active == 1 && !changed_serial {
                    stream.read_exact(&mut header).unwrap();
                    assert_eq!(u32::from_le_bytes(header[8..12].try_into().unwrap()), 1101);
                    let size = u32::from_le_bytes(header[12..16].try_into().unwrap()) as usize;
                    let mut body = vec![0; size];
                    stream.read_exact(&mut body).unwrap();
                    assert_eq!(body[4..8], 0u32.to_le_bytes());
                    assert_eq!(&body[8..], sdk_mode("Spectrum Cycle"));
                    writes += 1;
                }
                if changed_serial || (original_active != 0 && original_active != 1) { break; }
            }
            writes
        });
        let baseline = parse_target(&sdk_controller(0, "fixture")).unwrap();
        let record = AmbientRecoveryRecord {
            original_mode_hex: hex_encode(&baseline.modes[0].1),
            ..record_for(&baseline)
        };
        let mut socket = TcpStream::connect(address).unwrap();
        socket.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        let result = restore_verified_mode(&mut socket, 3, &record, &baseline.modes[0].1);
        drop(socket);
        let writes = server.join().unwrap();
        assert_eq!(writes > 0, expect_write);
        (result, writes)
    }

    #[test]
    fn mock_complete_restoration_writes_and_verifies() {
        let (result, writes) = mock_recovery_case(1, 0, false, true);
        assert!(result.is_ok(), "{result:?}");
        assert_eq!(writes, 1);
    }

    #[test]
    fn mock_restoration_rejects_unverified_result() {
        let (result, writes) = mock_recovery_case(1, 1, false, true);
        assert!(result.is_err());
        assert_eq!(writes, 1);
    }

    #[test]
    fn mock_already_restored_skips_device_write() {
        let (result, writes) = mock_recovery_case(0, 0, false, false);
        assert!(result.is_ok(), "{result:?}");
        assert_eq!(writes, 0);
    }

    #[test]
    fn mock_identity_mismatch_rejects_before_write() {
        let (result, writes) = mock_recovery_case(1, 0, true, false);
        assert!(result.is_err());
        assert_eq!(writes, 0);
    }

    // Full physical-layout fixture for Stage 3E. The minimal SDK controller
    // fixture used by Stage 3C/3D intentionally has no LEDs or zones.
    fn stage3e_cynosa_fixture() -> Target {
        let mut info = target();
        for (index, label, _) in TEST_KEYS {
            info.led_names[index] = format!("Key: {label}");
        }
        info
    }

    #[test]
    fn stage3e_post_consent_baseline_accepts_unchanged_controller() {
        let baseline = stage3e_cynosa_fixture();
        assert!(verify_physical_test_baseline(&baseline, &baseline).is_ok());
    }

    #[test]
    fn stage3e_post_consent_baseline_rejects_changed_mode_and_bytes() {
        let baseline = stage3e_cynosa_fixture();
        let mut changed = { let mut info = stage3e_cynosa_fixture(); info.active = 1; info };
        assert!(verify_physical_test_baseline(&baseline, &changed).is_err());
        changed.active = baseline.active;
        changed.modes[0].1.push(0x7f);
        assert!(verify_physical_test_baseline(&baseline, &changed).is_err());
    }

    #[test]
    fn stage3e_post_consent_baseline_rejects_changed_identity_and_layout() {
        let baseline = stage3e_cynosa_fixture();
        let mut changed = stage3e_cynosa_fixture();
        changed.serial.push('x');
        assert!(verify_physical_test_baseline(&baseline, &changed).is_err());
        changed.serial = baseline.serial.clone();
        changed.led_names[1].push('x');
        assert!(verify_physical_test_baseline(&baseline, &changed).is_err());
    }

    // Stage 3D: an end-to-end *fixture* lifecycle. The SDK connection uses
    // mock_recovery_case's ephemeral loopback listener; the state is in memory.
    // Neither the live state.json nor the physical OpenRGB endpoint is touched.
    #[test]
    fn stage3d_successful_interrupted_session_lifecycle() {
        use crate::manage_runtime_state::{
            fixture_register_ambient_recovery as register,
            fixture_pending_ambient_recoveries as pending,
            fixture_complete_ambient_recovery as complete,
        };
        let baseline = parse_target(&sdk_controller(0, "fixture")).unwrap();
        let mut record = record_for(&baseline);
        record.original_mode_hex = hex_encode(&baseline.modes[0].1);
        record.ownership_token = "stage3d-owner-success".into();
        let original = serde_json::json!({"ui": {"selected_policy": 42}, "wallpaper": true});
        let mut state = original.clone();
        register(&mut state, &record).unwrap();
        let persisted = serde_json::to_vec(&state).unwrap();
        let mut after_crash: serde_json::Value = serde_json::from_slice(&persisted).unwrap();
        assert_eq!(pending(&after_crash).unwrap(), vec![record.clone()]);
        let (result, writes) = mock_recovery_case(1, 0, false, true);
        assert!(result.is_ok(), "{result:?}");
        assert_eq!(writes, 1);
        complete(&mut after_crash, &record.device_identity(), &record.ownership_token).unwrap();
        assert!(pending(&after_crash).unwrap().is_empty());
        assert_eq!(after_crash, original);
    }

    #[test]
    fn stage3d_unverified_restoration_retains_recovery_and_other_state() {
        use crate::manage_runtime_state::{
            fixture_register_ambient_recovery as register,
            fixture_pending_ambient_recoveries as pending,
        };
        let baseline = parse_target(&sdk_controller(0, "fixture")).unwrap();
        let mut record = record_for(&baseline);
        record.original_mode_hex = hex_encode(&baseline.modes[0].1);
        record.ownership_token = "stage3d-owner-failure".into();
        let mut state = serde_json::json!({"ui": {"selected_policy": 77}});
        register(&mut state, &record).unwrap();
        let snapshot = serde_json::to_vec(&state).unwrap();
        let (result, writes) = mock_recovery_case(1, 1, false, true);
        assert!(result.is_err());
        assert_eq!(writes, 1);
        assert_eq!(serde_json::to_vec(&state).unwrap(), snapshot);
        assert_eq!(pending(&state).unwrap(), vec![record]);
    }

    #[test]
    fn stage3d_active_owner_blocks_recovery_before_network_access() {
        use crate::manage_runtime_state::{
            fixture_register_ambient_recovery as register,
            fixture_pending_ambient_recoveries as pending,
        };
        let baseline = parse_target(&sdk_controller(0, "fixture")).unwrap();
        let mut record = record_for(&baseline);
        record.ownership_token = "stage3d-lock-owner".into();
        let mut state = serde_json::json!({"ui": "untouched"});
        register(&mut state, &record).unwrap();
        let mut path = std::env::temp_dir();
        path.push(format!("screenshaver-stage3d-owner-{}-{:?}",
            std::process::id(), std::thread::current().id()));
        let owner = OpenOptions::new().create_new(true).read(true).write(true)
            .open(&path).unwrap();
        let recovery = OpenOptions::new().read(true).write(true).open(&path).unwrap();
        owner.try_lock_exclusive().unwrap();
        // Mirrors the production ordering: an unsuccessful owner acquisition
        // must stop recovery before it connects to the SDK or modifies state.
        assert!(recovery.try_lock_exclusive().is_err());
        assert_eq!(pending(&state).unwrap(), vec![record]);
        FileExt::unlock(&owner).unwrap();
        recovery.try_lock_exclusive().unwrap();
        FileExt::unlock(&recovery).unwrap();
        drop(owner);
        drop(recovery);
        std::fs::remove_file(path).unwrap();
    }

}
