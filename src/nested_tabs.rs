// Reusable nested-tab layout helpers.
//
// This module owns reusable vertical nested-tab navigation for Control Center
// parent tabs. Configuration and Post-Processing use the same rail and page
// layout with live policy state. Persistence remains owned by editor_layout.rs /
// edit_shader.rs and the existing policy/database modules.

use std::sync::OnceLock;

use crate::editor_layout::{
    AntiAliasingSelection,
    BloomSelection,
    BulkBooleanSelection,
    ColorPrecisionSelection,
    ControlConfiguration,
    DitheringSelection,
    PolicyDisplayRow,
    PolicyTarget,
};



// OpenRGB Control Center diagnostics. Network work is performed on a worker
// thread; UI rendering never waits for TCP, DNS or SDK replies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpenRgbNestedTab { Status, Devices, Mapping, Appearance, Diagnostics, Settings }

impl OpenRgbNestedTab {
    const ALL: [Self; 6] = [Self::Status, Self::Devices, Self::Mapping, Self::Appearance, Self::Diagnostics, Self::Settings];
    fn label(self) -> &'static str {
        match self {
            Self::Status => "Status", Self::Devices => "Devices",
            Self::Mapping => "Mapping", Self::Appearance => "Appearance",
            Self::Diagnostics => "Diagnostics",
            Self::Settings => "Settings",
        }
    }
}

#[derive(Clone, Debug)]
struct OpenRgbDiscoveredDevice {
    id: u32,
    name: String,
    serial: String,
    leds: usize,
    zones: usize,
    modes: usize,
    matrix: bool,
    topology: Option<crate::manage_ambient_lighting::LedMatrix>,
    topology_diagnostic: String,
    identity: String,
    eligible: bool,
    reason: String,
}

#[derive(Clone, Debug)]
struct OpenRgbUiState {
    endpoint: String,
    saved_endpoint: String,
    settings_error: String,
    checked_endpoint: String,
    checking: bool,
    checked: bool,
    connection_ok: bool,
    message: String,
    devices: Vec<OpenRgbDiscoveredDevice>,
    selected_identity: Option<String>,
    selection_message: String,
}

impl Default for OpenRgbUiState {
    fn default() -> Self {
        Self {
            endpoint: "127.0.0.1:6742".into(), saved_endpoint: "127.0.0.1:6742".into(),
            settings_error: String::new(), checked_endpoint: String::new(),
            checking: false, checked: false, connection_ok: false,
            message: String::new(), devices: Vec::new(),
            selected_identity: None, selection_message: String::new(),
        }
    }
}

fn openrgb_ui_state() -> &'static std::sync::Mutex<OpenRgbUiState> {
    static STATE: OnceLock<std::sync::Mutex<OpenRgbUiState>> = OnceLock::new();
    STATE.get_or_init(|| {
        let mut state = OpenRgbUiState::default();
        match openrgb_load_server() {
            Ok(Some(endpoint)) => { state.endpoint = endpoint.clone(); state.saved_endpoint = endpoint; }
            Ok(None) => {}
            Err(error) => state.settings_error = error,
        }
        match openrgb_load_selection() {
            Ok(identity) => state.selected_identity = identity,
            Err(error) => state.selection_message = error,
        }
        std::sync::Mutex::new(state)
    })
}

// Persist connection preferences in the existing schema-v003 server table.
// No schema changes, SDK writes, controller acquisition, or device rescans.
fn openrgb_load_server() -> Result<Option<String>, String> {
    use rusqlite::OptionalExtension;
    let connection = crate::open_database::open()?;
    connection.query_row(
        "SELECT host, port FROM openrgb_servers WHERE server_name = 'Default' COLLATE NOCASE LIMIT 1",
        [],
        |row| { let host: String = row.get(0)?; let port: u16 = row.get(1)?; Ok(if host.contains(':') { format!("[{host}]:{port}") } else { format!("{host}:{port}") }) },
    ).optional().map_err(|error| format!("Cannot read OpenRGB server settings: {error}"))
}

fn openrgb_save_server(endpoint: &str) -> Result<(), String> {
    // Validate syntax without requiring a running OpenRGB server or DNS success.
    let (host, port_text) = endpoint.rsplit_once(':')
        .ok_or("Specify an OpenRGB server as host:port")?;
    let host = host.trim();
    if host.is_empty() || host.chars().any(char::is_whitespace) {
        return Err("OpenRGB server host must not be empty or contain spaces".into());
    }
    let port: u16 = port_text.parse().map_err(|_| "OpenRGB port must be between 1 and 65535")?;
    if port == 0 { return Err("OpenRGB port must be between 1 and 65535".into()); }
    // Bracketed IPv6 is accepted as [::1]:6742; strip brackets for storage.
    let stored_host = if host.starts_with('[') && host.ends_with(']') {
        &host[1..host.len()-1]
    } else { host };
    if stored_host.is_empty() || (stored_host.contains(':') && !host.starts_with('[')) {
        return Err("IPv6 hosts must use [address]:port syntax".into());
    }
    let mut connection = crate::open_database::open()?;
    let transaction = connection.transaction().map_err(|e| e.to_string())?;
    transaction.execute(
        "INSERT INTO openrgb_servers (server_name, host, port, enabled) VALUES ('Default', ?1, ?2, 0) \
         ON CONFLICT(server_name) DO UPDATE SET host=excluded.host, port=excluded.port",
        rusqlite::params![stored_host, i64::from(port)],
    ).map_err(|e| format!("Cannot save OpenRGB server settings: {e}"))?;
    transaction.commit().map_err(|e| format!("Cannot commit OpenRGB server settings: {e}"))
}

// Region presets are stored as disabled mapping drafts. The schema's source
// rectangles describe sampling geometry, not a verified LED membership mask.
// A later guarded renderer must independently derive membership from topology.
const OPENRGB_MATRIX_REGIONS: &[(&str, &str, f64, f64, f64, f64)] = &[
    ("all", "Entire device", 0.0, 0.0, 1.0, 1.0),
    ("left", "Left", 0.0, 0.0, 0.5, 1.0),
    ("right", "Right", 0.5, 0.0, 0.5, 1.0),
    ("top", "Top", 0.0, 0.0, 1.0, 0.5),
    ("bottom", "Bottom", 0.0, 0.5, 1.0, 0.5),
    ("center", "Center", 0.25, 0.25, 0.5, 0.5),
    ("perimeter", "Perimeter", 0.0, 0.0, 1.0, 1.0),
    ("corners", "Four corners", 0.0, 0.0, 1.0, 1.0),
    ("top_left", "Top-left", 0.0, 0.0, 0.5, 0.5),
    ("top_right", "Top-right", 0.5, 0.0, 0.5, 0.5),
    ("bottom_left", "Bottom-left", 0.0, 0.5, 0.5, 0.5),
    ("bottom_right", "Bottom-right", 0.5, 0.5, 0.5, 0.5),
];
const OPENRGB_LINEAR_REGIONS: &[(&str, &str, f64, f64, f64, f64)] = &[
    ("first_half", "First half", 0.0, 0.0, 0.5, 1.0),
    ("second_half", "Second half", 0.5, 0.0, 0.5, 1.0),
];

fn openrgb_load_region(identity: &str) -> Result<String, String> {
    use rusqlite::OptionalExtension;
    let connection = crate::open_database::open()?;
    let region: Option<String> = connection.query_row(
        "SELECT r.region_name FROM openrgb_mappings m \
         JOIN openrgb_devices d ON d.device_id=m.device_id \
         JOIN openrgb_servers s ON s.server_id=d.server_id \
         JOIN openrgb_source_regions r ON r.region_id=m.region_id \
         WHERE s.server_name='Default' COLLATE NOCASE AND d.stable_identity=?1 \
           AND r.region_name LIKE 'screenshaver-led:%' \
         ORDER BY m.mapping_id DESC LIMIT 1",
        [identity], |row| row.get(0),
    ).optional().map_err(|e| format!("Cannot load LED region: {e}"))?;
    Ok(region.and_then(|v| v.strip_prefix("screenshaver-led:").map(str::to_owned))
        .unwrap_or_else(|| "all".into()))
}

fn openrgb_save_region(identity: &str, key: &str) -> Result<(), String> {
    let region = OPENRGB_MATRIX_REGIONS.iter().chain(OPENRGB_LINEAR_REGIONS.iter())
        .find(|r| r.0 == key).ok_or("Unsupported LED region")?;
    let mut connection = crate::open_database::open()?;
    let tx = connection.transaction().map_err(|e| e.to_string())?;
    let device_id: i64 = tx.query_row(
        "SELECT d.device_id FROM openrgb_devices d JOIN openrgb_servers s ON s.server_id=d.server_id \
         WHERE s.server_name='Default' COLLATE NOCASE AND d.stable_identity=?1 AND d.enabled=1",
        [identity], |row| row.get(0),
    ).map_err(|e| format!("Selected device is not saved: {e}"))?;
    let region_name = format!("screenshaver-led:{key}");
    tx.execute(
        "INSERT INTO openrgb_source_regions (region_name,x,y,width,height) VALUES (?1,?2,?3,?4,?5) \
         ON CONFLICT(region_name) DO NOTHING",
        rusqlite::params![region_name, region.2, region.3, region.4, region.5],
    ).map_err(|e| format!("Cannot store region preset: {e}"))?;
    let region_id: i64 = tx.query_row(
        "SELECT region_id FROM openrgb_source_regions WHERE region_name=?1", [&region_name], |row| row.get(0),
    ).map_err(|e| e.to_string())?;
    // Replace only our own disabled drafts. Never touch third-party mappings.
    tx.execute(
        "DELETE FROM openrgb_mappings WHERE device_id=?1 AND enabled=0 AND region_id IN \
         (SELECT region_id FROM openrgb_source_regions WHERE region_name LIKE 'screenshaver-led:%')",
        [device_id],
    ).map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO openrgb_mappings (device_id,region_id,sampling_method,enabled) VALUES (?1,?2,'spatial',0)",
        rusqlite::params![device_id, region_id],
    ).map_err(|e| format!("Cannot save region: {e}"))?;
    tx.commit().map_err(|e| format!("Cannot commit region: {e}"))
}

// Stable across controller re-enumeration. This is a conservative topology
// fingerprint, not a globally unique manufacturer identifier. Duplicate matches
// are deliberately refused rather than guessed.
fn openrgb_identity(snapshot: &crate::manage_openrgb_session::OpenRgbControllerSnapshot) -> String {
    let mut bytes = Vec::new();
    for value in [&snapshot.name, &snapshot.serial] {
        bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
        bytes.extend_from_slice(value.as_bytes());
    }
    for value in [snapshot.led_names.len() as u64, snapshot.width as u64, snapshot.height as u64] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for name in &snapshot.led_names {
        bytes.extend_from_slice(&(name.len() as u64).to_le_bytes());
        bytes.extend_from_slice(name.as_bytes());
    }
    for count in &snapshot.zone_counts { bytes.extend_from_slice(&count.to_le_bytes()); }
    for index in &snapshot.matrix { bytes.extend_from_slice(&index.to_le_bytes()); }
    let hash = bytes.iter().fold(0xcbf29ce484222325u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    });
    format!("screenshaver-topology-v1:{hash:016x}")
}

fn openrgb_load_selection() -> Result<Option<String>, String> {
    use rusqlite::OptionalExtension;
    let connection = crate::open_database::open()?;
    connection.query_row(
        "SELECT d.stable_identity FROM openrgb_devices d JOIN openrgb_servers s ON s.server_id=d.server_id \
         WHERE s.server_name='Default' COLLATE NOCASE AND d.enabled=1 LIMIT 1",
        [], |row| row.get(0),
    ).optional().map_err(|e| format!("Cannot load OpenRGB device selection: {e}"))
}

fn openrgb_save_selection(device: Option<&OpenRgbDiscoveredDevice>) -> Result<(), String> {
    let mut connection = crate::open_database::open()?;
    let transaction = connection.transaction().map_err(|e| e.to_string())?;
    let server_id: i64 = transaction.query_row(
        "SELECT server_id FROM openrgb_servers WHERE server_name='Default' COLLATE NOCASE",
        [], |row| row.get(0),
    ).map_err(|e| format!("Save an OpenRGB server first: {e}"))?;
    transaction.execute("UPDATE openrgb_devices SET enabled=0 WHERE server_id=?1", [server_id])
        .map_err(|e| format!("Cannot clear prior device selection: {e}"))?;
    if let Some(device) = device {
        transaction.execute(
            "INSERT INTO openrgb_devices (server_id, device_name, stable_identity, controller_id_hint, enabled) \
             VALUES (?1, ?2, ?3, ?4, 1) \
             ON CONFLICT(server_id, stable_identity) DO UPDATE SET \
             device_name=excluded.device_name, controller_id_hint=excluded.controller_id_hint, enabled=1",
            rusqlite::params![server_id, device.name, device.identity, i64::from(device.id)],
        ).map_err(|e| format!("Cannot save OpenRGB device selection: {e}"))?;
    }
    transaction.commit().map_err(|e| format!("Cannot commit OpenRGB device selection: {e}"))
}

fn openrgb_probe(endpoint: &str) -> Result<Vec<OpenRgbDiscoveredDevice>, (bool, String)> {
    use std::net::ToSocketAddrs;
    let addresses: Vec<_> = endpoint.to_socket_addrs()
        .map_err(|error| (false, format!("Invalid or unresolved endpoint: {error}")))?
        .collect();
    if addresses.is_empty() {
        return Err((false, "Endpoint resolved to no addresses".into()));
    }
    let mut transport = None;
    let mut last_error = String::new();
    for address in addresses {
        match crate::manage_openrgb_session::OpenRgbReadOnlyTransport::connect(address) {
            Ok(connection) => { transport = Some(connection); break; }
            Err(error) => last_error = error,
        }
    }
    let mut transport = transport.ok_or((false, last_error))?;
    let ids = transport.controller_ids().map_err(|error| (true, error))?;
    let mut devices = Vec::with_capacity(ids.len());
    for id in ids {
        let snapshot = transport.decoded_controller(id).map_err(|error| {
            (true, format!("Controller {id} discovery failed: {error}"))
        })?;
        let descriptor = snapshot.descriptor(id);
        // Configuration eligibility is distinct from guarded hardware acquisition.
        // An already-Direct device can be saved and mapped without taking ownership;
        // the acquisition contract will still reject it until a safe baseline exists.
        let configuration_eligibility = (|| -> Result<(), String> {
            if descriptor.display_name.trim().is_empty() {
                return Err("OpenRGB controller has no display name".into());
            }
            if descriptor.led_count == 0 || descriptor.led_count > u16::MAX as usize {
                return Err("OpenRGB LED count is outside supported limits".into());
            }
            let matrix = descriptor.matrix.as_ref()
                .ok_or("Selected device has no spatial LED matrix")?;
            matrix.validate()?;
            if matrix.led_count != descriptor.led_count {
                return Err("OpenRGB LED count and matrix topology disagree".into());
            }
            Ok(())
        })();
        // Analyze the SDK snapshot as received. No device control or framebuffer access.
        let topology_diagnostic = openrgb_topology_diagnostic(&snapshot);
        let identity = openrgb_identity(&snapshot);
        devices.push(OpenRgbDiscoveredDevice {
            id, name: snapshot.name, serial: snapshot.serial,
            leds: snapshot.led_names.len(), zones: snapshot.zone_counts.len(),
            modes: snapshot.modes.len(), matrix: descriptor.matrix.is_some(), topology: descriptor.matrix.clone(),
            topology_diagnostic, identity, eligible: configuration_eligibility.is_ok(),
            reason: configuration_eligibility.err().unwrap_or_default(),
        });
    }
    Ok(devices)
}

/// Read-only topology report. Distinguish unused matrix cells from LED indices
/// that are present in the controller list but absent from the matrix.
fn openrgb_topology_diagnostic(snapshot: &crate::manage_openrgb_session::OpenRgbControllerSnapshot) -> String {
    use std::collections::BTreeSet;
    let total = snapshot.led_names.len();
    let cells = snapshot.matrix.len();
    let empty = snapshot.matrix.iter().filter(|&&i| i == u32::MAX).count();
    let mut unique = BTreeSet::new();
    let mut duplicates = 0usize;
    let mut invalid = BTreeSet::new();
    for &index in &snapshot.matrix {
        if index == u32::MAX { continue; }
        if index as usize >= total { invalid.insert(index); continue; }
        if !unique.insert(index as usize) { duplicates += 1; }
    }
    let missing: Vec<_> = (0..total).filter(|i| !unique.contains(i)).collect();
    let mut lines = vec![
        format!("Matrix: {} rows x {} columns ({} cells)", snapshot.height, snapshot.width, cells),
        format!("Controller LED records: {total}"),
        format!("Matrix entries: {} populated, {empty} empty (0xFFFFFFFF)", cells.saturating_sub(empty)),
        format!("Distinct valid LED indices referenced: {}", unique.len()),
        format!("Duplicate matrix references: {duplicates}"),
        format!("Out-of-range matrix indices: {}", invalid.len()),
        format!("Controller LED indices absent from matrix: {}", missing.len()),
    ];
    if !missing.is_empty() {
        lines.push(format!("Unreferenced indices: {}", missing.iter().map(usize::to_string).collect::<Vec<_>>().join(", ")));
        for &index in missing.iter().take(40) {
            lines.push(format!("  LED {index}: {}", snapshot.led_names[index]));
        }
        if missing.len() > 40 { lines.push(format!("  ... {} more names", missing.len() - 40)); }
    }
    if !invalid.is_empty() {
        lines.push(format!("Invalid matrix indices: {}", invalid.iter().map(u32::to_string).collect::<Vec<_>>().join(", ")));
    }
    if snapshot.width.checked_mul(snapshot.height) != Some(cells) {
        lines.push("WARNING: Matrix dimensions do not match cell count".into());
    }
    lines.join("\n")
}

fn openrgb_start_probe(ctx: egui::Context) {
    let endpoint = {
        let mut state = openrgb_ui_state().lock().unwrap_or_else(|e| e.into_inner());
        if state.checking { return; }
        state.checking = true;
        state.checked = false;
        state.connection_ok = false;
        state.devices.clear();
        state.message.clear();
        state.checked_endpoint = state.endpoint.trim().to_string();
        state.checked_endpoint.clone()
    };
    std::thread::spawn(move || {
        let result = openrgb_probe(&endpoint);
        let mut state = openrgb_ui_state().lock().unwrap_or_else(|e| e.into_inner());
        state.checking = false;
        state.checked = true;
        match result {
            Ok(devices) => {
                state.connection_ok = true;
                state.message = if devices.is_empty() {
                    "Connected; the server reports no controllers".into()
                } else { format!("Connected; {} controller(s) discovered", devices.len()) };
                if let Some(identity) = &state.selected_identity {
                    let matches = devices.iter().filter(|d| &d.identity == identity).count();
                    state.selection_message = match matches {
                        0 => "Saved device is unavailable or its topology has changed; selection retained.".into(),
                        1 => "Saved device found; lighting control remains disabled.".into(),
                        _ => "Ambiguous device identity: multiple controllers match; no hardware selection is trusted.".into(),
                    };
                }
                state.devices = devices;
            }
            Err((connected, message)) => {
                state.connection_ok = connected;
                state.message = message;
                state.devices.clear();
            }
        }
        drop(state);
        ctx.request_repaint();
    });
}

// Use the desktop clipboard directly when egui's host integration does not
// deliver copy requests (notably some embedded/full-screen Control Centers).
// No shell invocation, extra crate, file chooser, or hardware interaction.
fn openrgb_copy_report(report: &str) -> Result<String, String> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    let x11 = std::env::var_os("DISPLAY").is_some();
    let mut candidates: Vec<(&str, &[&str])> = Vec::new();
    if wayland { candidates.push(("wl-copy", &["--type", "text/plain;charset=utf-8"])); }
    if x11 {
        candidates.push(("xclip", &["-selection", "clipboard"]));
        candidates.push(("xsel", &["--clipboard", "--input"]));
    }
    if candidates.is_empty() {
        return Err("No Wayland or X11 clipboard session detected".into());
    }
    let mut failures = Vec::new();
    for (program, args) in candidates {
        let mut child = match Command::new(program).args(args)
            .stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() {
            Ok(child) => child,
            Err(error) => { failures.push(format!("{program}: {error}")); continue; }
        };
        let write_result = child.stdin.take()
            .ok_or_else(|| format!("{program}: clipboard stdin unavailable"))
            .and_then(|mut stdin| stdin.write_all(report.as_bytes())
                .map_err(|error| format!("{program}: {error}")));
        if let Err(error) = write_result {
            let _ = child.kill();
            let _ = child.wait();
            failures.push(error);
            continue;
        }
        match child.wait() {
            Ok(status) if status.success() => return Ok(format!("Copied using {program}")),
            Ok(status) => failures.push(format!("{program}: exited with {status}")),
            Err(error) => failures.push(format!("{program}: {error}")),
        }
    }
    Err(format!("Clipboard unavailable: {}", failures.join("; ")))
}

// Explicit, bounded hardware verification. Never start from discovery or tab navigation.
#[derive(Clone, Copy, PartialEq, Eq)]
enum OpenRgbTestPattern { All, Row(usize), Led(usize), Unreferenced }

struct OpenRgbPhysicalTest {
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    handle: std::thread::JoinHandle<Result<String, String>>,
}

fn openrgb_physical_state() -> &'static std::sync::Mutex<Option<OpenRgbPhysicalTest>> {
    static TEST: OnceLock<std::sync::Mutex<Option<OpenRgbPhysicalTest>>> = OnceLock::new();
    TEST.get_or_init(|| std::sync::Mutex::new(None))
}
fn openrgb_physical_message() -> &'static std::sync::Mutex<String> {
    static MESSAGE: OnceLock<std::sync::Mutex<String>> = OnceLock::new();
    MESSAGE.get_or_init(|| std::sync::Mutex::new("No physical LED test has run.".into()))
}
fn openrgb_set_physical_message(message: String) {
    *openrgb_physical_message().lock().unwrap_or_else(|e| e.into_inner()) = message;
}
fn openrgb_test_running() -> bool {
    openrgb_physical_state().lock().unwrap_or_else(|e| e.into_inner()).is_some()
}
fn openrgb_reap_test() {
    let finished = {
        let mut slot = openrgb_physical_state().lock().unwrap_or_else(|e| e.into_inner());
        if slot.as_ref().is_some_and(|test| test.handle.is_finished()) { slot.take() } else { None }
    };
    if let Some(test) = finished {
        let result = test.handle.join().unwrap_or_else(|_| Err("LED diagnostic thread panicked; check durable recovery state".into()));
        openrgb_set_physical_message(match result { Ok(msg) => msg, Err(err) => format!("LED test error: {err}") });
    }
}
fn openrgb_stop_test() {
    if let Some(test) = openrgb_physical_state().lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        test.stop.store(true, std::sync::atomic::Ordering::Release);
    }
}
/// Called on confirmed Control Center exit and before GL painter destruction.
/// Blocks until all hardware writes have stopped and mode restoration has been attempted.
pub fn openrgb_shutdown_physical_test() {
    let test = {
        let mut slot = openrgb_physical_state().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(test) = slot.as_ref() { test.stop.store(true, std::sync::atomic::Ordering::Release); }
        slot.take()
    };
    if let Some(test) = test {
        let result = test.handle.join().unwrap_or_else(|_| Err("LED diagnostic thread panicked; durable recovery may be pending".into()));
        openrgb_set_physical_message(match result { Ok(msg) => msg, Err(err) => format!("LED test error: {err}") });
    }
}
fn openrgb_start_physical_test(endpoint: String, controller_id: u32, pattern: OpenRgbTestPattern) {
    openrgb_reap_test();
    let mut slot = openrgb_physical_state().lock().unwrap_or_else(|e| e.into_inner());
    if slot.is_some() { return; }
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop_thread = stop.clone();
    openrgb_set_physical_message("Acquiring guarded OpenRGB session...".into());
    let handle = std::thread::spawn(move || openrgb_run_physical_test(&endpoint, controller_id, pattern, &stop_thread));
    *slot = Some(OpenRgbPhysicalTest { stop, handle });
}
fn openrgb_run_physical_test(endpoint: &str, controller_id: u32, pattern: OpenRgbTestPattern,
    stop: &std::sync::atomic::AtomicBool) -> Result<String, String> {
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};
    use std::net::ToSocketAddrs;
    let address = endpoint.to_socket_addrs().map_err(|e| e.to_string())?.next().ok_or("No OpenRGB endpoint address")?;
    if stop.load(Ordering::Acquire) { return Ok("LED test cancelled before acquisition.".into()); }
    let mut transport = crate::manage_openrgb_session::OpenRgbReadOnlyTransport::connect(address)?;
    let ids = transport.controller_ids()?;
    if !ids.contains(&controller_id) { return Err("Selected OpenRGB controller disappeared".into()); }
    let snapshot = transport.decoded_controller(controller_id)?;
    let selected = crate::select_ambient_device::select_ambient_device(&[snapshot.descriptor(controller_id)], controller_id)?;
    let matrix = selected.matrix.as_ref().ok_or("No verified LED matrix")?;
    let mut selected_leds = vec![false; selected.led_count];
    match pattern {
        OpenRgbTestPattern::All => for &index in &matrix.indices { if index != u32::MAX { selected_leds[index as usize] = true; } },
        OpenRgbTestPattern::Row(row) => {
            if row >= matrix.rows { return Err("Selected LED row no longer exists".into()); }
            for &index in &matrix.indices[row*matrix.columns..(row+1)*matrix.columns] {
                if index != u32::MAX { selected_leds[index as usize] = true; }
            }
        },
        OpenRgbTestPattern::Led(index) => {
            if index >= selected.led_count { return Err("LED index no longer exists".into()); }
            selected_leds[index] = true;
        },
        OpenRgbTestPattern::Unreferenced => {
            selected_leds.fill(true);
            for &index in &matrix.indices { if index != u32::MAX { selected_leds[index as usize] = false; } }
        },
    }
    if !selected_leds.iter().any(|&v| v) { return Err("Test pattern has no LEDs".into()); }
    if stop.load(Ordering::Acquire) { return Ok("LED test cancelled before acquisition.".into()); }
    let guarded = crate::manage_openrgb_session::GuardedOpenRgbSession::acquire(address, &selected)?;
    // Restoration is mandatory even when writer creation or transmission fails.
    let run = (|| -> Result<(), String> {
        let mut worker = crate::manage_ambient_lighting::LightingWorker::start(address, controller_id, selected.led_count)?;
        let deadline = Instant::now() + Duration::from_secs(15);
        let start = Instant::now();
        let mut error = None;
        while Instant::now() < deadline && !stop.load(Ordering::Acquire) {
            let phase = start.elapsed().as_secs_f64().fract();
            let amplitude = (1.0 - (std::f64::consts::TAU * phase).cos()) * 0.5;
            let rgb = [(255.0 * amplitude).round() as u8, (95.0 * amplitude).round() as u8,
                       (31.0 * amplitude).round() as u8, 0];
            let colors = selected_leds.iter().map(|&on| if on { rgb } else { [0,0,0,0] }).collect();
            if let Err(e) = worker.try_submit(colors) { error = Some(e); break; }
            std::thread::sleep(Duration::from_millis(33));
        }
        // Do not enqueue a final black frame: it can race with restoration and
        // leave physical LEDs dark even when the SDK reports the restored mode.
        // Dropping the sender and joining the writer must precede restoration.
        let interrupted = stop.load(Ordering::Acquire);
        crate::logger::information(
            &crate::locate_paths::runtime_log_path(),
            &format!("[OPENRGB LED TEST] {}: stopping LED writer before restoration",
                if interrupted { "manual cancellation" } else { "timeout or completion" }),
        );
        let joined = worker.stop();
        match &joined {
            Ok(stats) => crate::logger::information(
                &crate::locate_paths::runtime_log_path(),
                &format!("[OPENRGB LED TEST] LED writer joined; transmitted={}, dropped={}",
                    stats.transmitted, stats.dropped),
            ),
            Err(e) => crate::logger::warning(
                &crate::locate_paths::runtime_log_path(),
                &format!("[OPENRGB LED TEST] LED writer shutdown failed: {e}"),
            ),
        }
        if let Some(e) = error { return Err(e); }
        joined.map(|_| ())
    })();
    crate::logger::information(
        &crate::locate_paths::runtime_log_path(),
        "[OPENRGB LED TEST] Attempting original-mode restoration and SDK verification",
    );
    let restored = guarded.restore();
    match &restored {
        Ok(()) => crate::logger::information(
            &crate::locate_paths::runtime_log_path(),
            "[OPENRGB LED TEST] Original mode verified by SDK; physical lighting not verified",
        ),
        Err(e) => crate::logger::warning(
            &crate::locate_paths::runtime_log_path(),
            &format!("[OPENRGB LED TEST] Restoration unverified: {e}; recovery record retained"),
        ),
    }
    match (run, restored) {
        (Ok(()), Ok(())) => Ok("LED test stopped; original OpenRGB mode verified by SDK. Confirm physical lighting by observation.".into()),
        (Err(e), Ok(())) => Err(format!("{e}; original mode restored")),
        (Ok(()), Err(e)) => Err(format!("Restoration unverified: {e}; recovery record retained")),
        (Err(e), Err(r)) => Err(format!("{e}; restoration unverified: {r}; recovery record retained")),
    }
}


// UI-only perimeter editor. No database writes or OpenRGB hardware interaction.
// Coordinates are normalized and independent of the editor's aspect ratio.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpenRgbLayout { EntireDevice, MultiZonePerimeter }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpenRgbEdge { Top, Right, Bottom, Left }
impl OpenRgbEdge {
    const ALL: [Self; 4] = [Self::Top, Self::Right, Self::Bottom, Self::Left];
    fn name(self) -> &'static str {
        match self { Self::Top => "Top", Self::Right => "Right",
            Self::Bottom => "Bottom", Self::Left => "Left" }
    }
}
#[derive(Clone, Debug)]
struct OpenRgbSegment {
    id: u64,
    end: f32, // Clockwise normalized end position on its edge.
    assignment: Option<String>,
    reverse: bool,
}
#[derive(Clone, Debug)]
struct OpenRgbMappingEditor {
    layout: OpenRgbLayout,
    aspect: usize,
    edges: [Vec<OpenRgbSegment>; 4],
    selected: (usize, u64),
    next_id: u64,
    pending_delete: Option<(usize, usize)>,
}
impl Default for OpenRgbMappingEditor {
    fn default() -> Self {
        let edges = std::array::from_fn(|index| vec![OpenRgbSegment {
            id: index as u64 + 1, end: 1.0, assignment: None, reverse: false,
        }]);
        Self { layout: OpenRgbLayout::EntireDevice, aspect: 0, edges,
            selected: (0, 1), next_id: 5, pending_delete: None }
    }
}
fn openrgb_mapping_editor() -> &'static std::sync::Mutex<OpenRgbMappingEditor> {
    static EDITOR: OnceLock<std::sync::Mutex<OpenRgbMappingEditor>> = OnceLock::new();
    EDITOR.get_or_init(|| std::sync::Mutex::new(OpenRgbMappingEditor::default()))
}
fn openrgb_draw_mapping_editor(ui: &mut egui::Ui, state: &OpenRgbUiState) {
    let mut editor = openrgb_mapping_editor().lock().unwrap_or_else(|e| e.into_inner());
    ui.horizontal(|ui| {
        ui.label("Mapping Layout:");
        egui::ComboBox::from_id_source("openrgb_mapping_layout")
            .selected_text(match editor.layout {
                OpenRgbLayout::EntireDevice => "Entire Device",
                OpenRgbLayout::MultiZonePerimeter => "Multi-Zone Perimeter",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut editor.layout, OpenRgbLayout::EntireDevice, "Entire Device");
                ui.selectable_value(&mut editor.layout, OpenRgbLayout::MultiZonePerimeter, "Multi-Zone Perimeter");
            });
    });
    if editor.layout == OpenRgbLayout::EntireDevice {
        ui.label("Full-frame spatial sampling. Used for windowshaders and partial-screen surfaces.");
        ui.label("Device selection remains on the Devices subtab.");
        return;
    }
    const ASPECTS: [(&str, f32); 7] = [
        ("Auto (Current Display)", 16.0/9.0), ("16:9", 16.0/9.0),
        ("21:9", 21.0/9.0), ("32:9", 32.0/9.0),
        ("16:10", 16.0/10.0), ("4:3", 4.0/3.0), ("3:2", 3.0/2.0),
    ];
    ui.horizontal(|ui| {
        ui.label("Editor aspect ratio:");
        egui::ComboBox::from_id_source("openrgb_mapping_aspect")
            .selected_text(ASPECTS[editor.aspect].0)
            .show_ui(ui, |ui| {
                for (i, (name, _)) in ASPECTS.iter().enumerate() {
                    ui.selectable_value(&mut editor.aspect, i, *name);
                }
            });
    });
    let usable = ui.available_width().max(160.0);
    let height = (usable - 40.0) / (32.0 / 9.0);
    let height = height.min(190.0).max(65.0);
    let width = height * ASPECTS[editor.aspect].1;
    let (outer, _) = ui.allocate_exact_size(
        egui::vec2(usable, height + 42.0), egui::Sense::hover());
    let screen = egui::Rect::from_center_size(
        egui::pos2(outer.center().x, outer.center().y),
        egui::vec2(width, height));
    let painter = ui.painter().clone();
    painter.rect_filled(screen, 2.0, egui::Color32::from_rgb(30, 48, 70));
    painter.rect_stroke(screen, 2.0, egui::Stroke::new(1.0, egui::Color32::GRAY), egui::StrokeKind::Inside);
    let mut zone_number = 0usize;
    for edge_index in 0..4 {
        let edge = OpenRgbEdge::ALL[edge_index];
        let len = editor.edges[edge_index].len();
        let mut previous = 0.0f32;
        for segment_index in 0..len {
            zone_number += 1;
            let segment = editor.edges[edge_index][segment_index].clone();
            let start = previous;
            let end = segment.end;
            previous = end;
            let point = |t: f32| match edge {
                OpenRgbEdge::Top => egui::pos2(screen.left() + t * width, screen.top() - 9.0),
                OpenRgbEdge::Right => egui::pos2(screen.right() + 9.0, screen.top() + t * height),
                OpenRgbEdge::Bottom => egui::pos2(screen.right() - t * width, screen.bottom() + 9.0),
                OpenRgbEdge::Left => egui::pos2(screen.left() - 9.0, screen.bottom() - t * height),
            };
            let a = point(start);
            let b = point(end);
            let selected = editor.selected == (edge_index, segment.id);
            painter.line_segment([a,b], egui::Stroke::new(7.0,
                if selected { egui::Color32::LIGHT_BLUE } else { egui::Color32::DARK_GRAY }));
            let mid = point((start + end) / 2.0);
            painter.text(mid, egui::Align2::CENTER_CENTER, zone_number.to_string(),
                egui::FontId::proportional(10.0), egui::Color32::WHITE);
            // Keep zone hit testing clear of the divider's larger drag target.
            let axis_gap = 12.0_f32.min((if edge_index % 2 == 0 {
                (b.x - a.x).abs()
            } else {
                (b.y - a.y).abs()
            }) * 0.25);
            let hit = if edge_index % 2 == 0 {
                egui::Rect::from_min_max(
                    egui::pos2(a.x.min(b.x) + axis_gap, a.y.min(b.y) - 7.0),
                    egui::pos2(a.x.max(b.x) - axis_gap, a.y.max(b.y) + 7.0),
                )
            } else {
                egui::Rect::from_min_max(
                    egui::pos2(a.x.min(b.x) - 7.0, a.y.min(b.y) + axis_gap),
                    egui::pos2(a.x.max(b.x) + 7.0, a.y.max(b.y) - axis_gap),
                )
            };
            let response = ui.interact(hit, egui::Id::new(("openrgb_zone",edge_index,segment.id)),
                egui::Sense::click());
            if response.clicked() { editor.selected = (edge_index,segment.id); }
            response.context_menu(|ui| {
                if ui.button("Split zone").clicked() {
                    // Preserve the old segment's assignment on its first half.
                    let midpoint = (start + end) / 2.0;
                    if (end-start) * (if edge_index%2==0 {width} else {height}) >= 44.0 {
                        let new_id = editor.next_id;
                        editor.next_id += 1;
                        editor.edges[edge_index][segment_index].end = midpoint;
                        editor.edges[edge_index].insert(segment_index+1, OpenRgbSegment {
                            id:new_id,end,assignment:None,reverse:false
                        });
                    }
                    ui.close_menu();
                }
                // The zone and divider hit regions can overlap in egui. Offer
                // deletion here as well, so it is always accessible from the
                // segment immediately before the divider.
                if segment_index + 1 < len && ui.button("Delete divider after this zone").clicked() {
                    editor.pending_delete = Some((edge_index, segment_index));
                    ui.close_menu();
                }
            });
            if segment_index+1 < len {
                let handle_pos = point(end);
                let handle_rect = egui::Rect::from_center_size(handle_pos,egui::vec2(16.0,16.0));
                painter.rect_filled(handle_rect,2.0,egui::Color32::LIGHT_GRAY);
                let handle = ui.interact(handle_rect.expand(3.0),
                    egui::Id::new(("openrgb_handle",edge_index,segment.id)),
                    egui::Sense::click_and_drag());
                if handle.dragged() {
                    if let Some(pointer) = handle.interact_pointer_pos() {
                        let t = match edge {
                            OpenRgbEdge::Top => (pointer.x-screen.left())/width,
                            OpenRgbEdge::Right => (pointer.y-screen.top())/height,
                            OpenRgbEdge::Bottom => (screen.right()-pointer.x)/width,
                            OpenRgbEdge::Left => (screen.bottom()-pointer.y)/height,
                        };
                        let min_gap = 18.0 / if edge_index%2==0 {width} else {height};
                        let lo = if segment_index==0 {0.0} else {editor.edges[edge_index][segment_index-1].end};
                        let hi = editor.edges[edge_index][segment_index+1].end;
                        if hi-lo >= 2.0*min_gap {
                            editor.edges[edge_index][segment_index].end = t.clamp(lo+min_gap,hi-min_gap);
                        }
                    }
                }
                handle.context_menu(|ui| {
                    if ui.button("Delete divider").clicked() {
                        editor.pending_delete=Some((edge_index,segment_index));
                        ui.close_menu();
                    }
                });
            }
        }
    }
    if let Some((edge,index)) = editor.pending_delete.take() {
        if index+1 < editor.edges[edge].len() {
            // Keep the clockwise-later segment's assignment when merging.
            let removed_id=editor.edges[edge][index].id;
            editor.edges[edge].remove(index);
            if editor.selected==(edge,removed_id) {
                editor.selected=(edge,editor.edges[edge][index].id);
            }
        }
    }
    let (edge,id)=editor.selected;
    if let Some(index)=editor.edges[edge].iter().position(|s|s.id==id) {
        let start=if index==0 {0.0} else {editor.edges[edge][index-1].end};
        let end=editor.edges[edge][index].end;
        let number:usize=editor.edges[..edge].iter().map(Vec::len).sum::<usize>()+index+1;
        ui.label(format!("Zone {number} — {} — {:.0}% to {:.0}%",
            OpenRgbEdge::ALL[edge].name(),start*100.0,end*100.0));
        let segment=&mut editor.edges[edge][index];
        let mut assignment=segment.assignment.clone();
        egui::ComboBox::from_id_source("openrgb_zone_assignment")
            .selected_text(assignment.as_deref().unwrap_or("Unassigned"))
            .show_ui(ui,|ui| {
                ui.selectable_value(&mut assignment,None,"Unassigned");
                for device in &state.devices {
                    if device.eligible {
                        ui.selectable_value(&mut assignment,Some(device.identity.clone()),&device.name);
                    }
                }
            });
        segment.assignment=assignment;
        ui.checkbox(&mut segment.reverse,"Reverse LED Direction");
    }
    ui.small("Editor prototype: assignments and dividers are temporary; no database or LED writes.");
}

pub fn draw_openrgb(ui: &mut egui::Ui) {
    let selected_id = egui::Id::new("screenshaver_openrgb_nested_tab");
    let mut selected = ui.ctx().data(|data| {
        data.get_temp::<OpenRgbNestedTab>(selected_id).unwrap_or(OpenRgbNestedTab::Status)
    });
    // A persisted selection is not the same as a live discovery snapshot.
    // Revalidate it asynchronously once on Control Center startup; never
    // discard the database selection when the OpenRGB server is offline.
    let should_restore = {
        let shared = openrgb_ui_state().lock().unwrap_or_else(|e| e.into_inner());
        shared.selected_identity.is_some() && !shared.checked && !shared.checking
    };
    if should_restore {
        openrgb_start_probe(ui.ctx().clone());
    }
    let state = openrgb_ui_state().lock().unwrap_or_else(|e| e.into_inner()).clone();
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            let labels: Vec<String> = OpenRgbNestedTab::ALL.iter()
                .map(|tab| tab.label().to_string()).collect();
            let tab_width = crate::editor_theme::vertical_tab_rail_width(ui, &labels, 126.0);
            ui.set_min_width(tab_width + 6.0);
            for tab in OpenRgbNestedTab::ALL {
                let enabled = match tab {
                    OpenRgbNestedTab::Status | OpenRgbNestedTab::Settings | OpenRgbNestedTab::Mapping | OpenRgbNestedTab::Diagnostics => true,
                    OpenRgbNestedTab::Devices => state.selected_identity.is_some() || (state.connection_ok && !state.devices.is_empty()),
                    OpenRgbNestedTab::Appearance => false,
                };
                let response = crate::editor_theme::vertical_tab(
                    ui, tab.label(), selected == tab, enabled, tab_width,
                );
                if response.clicked() { selected = tab; }
                if !enabled {
                    response.on_disabled_hover_text(match tab {
                        OpenRgbNestedTab::Devices => "Discover a device on the Status page first.",
                        _ => "Device mapping and lighting controls are not enabled in the read-only integration.",
                    });
                }
                ui.add_space(4.0);
            }
        });
        ui.separator();
        ui.add_space(12.0);
        ui.vertical(|ui| {
            ui.set_width(ui.available_width());
            match selected {
                OpenRgbNestedTab::Status => {
                    ui.heading("OpenRGB Status");
                    ui.add_space(6.0);
                    ui.label(format!("Server: {}", state.checked_endpoint.as_str().is_empty().then_some("Not checked").unwrap_or(&state.checked_endpoint)));
                    let connection = if state.checking { "Checking..." }
                        else if !state.checked { "Not checked" }
                        else if state.connection_ok { "Connected" } else { "Unavailable" };
                    ui.label(format!("Connection: {connection}"));
                    if state.checked && !state.message.is_empty() {
                        ui.label(&state.message);
                    }
                    if state.connection_ok {
                        ui.label(format!("Discovered devices: {}", state.devices.len()));
                    }
                    ui.add_space(10.0);
                    if ui.add_enabled(!state.checking, egui::Button::new("Refresh connection and devices")).clicked() {
                        openrgb_start_probe(ui.ctx().clone());
                    }
                    ui.add_space(10.0);
                    ui.label("Read-only SDK discovery. No lighting modes or LED colors are changed.");
                    ui.label("OpenRGB must be running with its SDK server enabled. Configure the endpoint in Settings.");
                }
                OpenRgbNestedTab::Devices => {
                    ui.heading("Devices");
                    ui.add_space(8.0);
                    ui.label("Choose one discovered device for future ambient lighting. Selection does not acquire or control hardware.");
                    ui.add_space(6.0);
                    if let Some(identity) = state.selected_identity.as_deref() {
                        if state.checking {
                            ui.label("Restoring saved device selection: checking OpenRGB discovery...");
                        } else if !state.devices.iter().any(|device| device.identity == identity) {
                            ui.label("Saved device selection retained; device not currently discovered.");
                        }
                    }
                    if !state.selection_message.is_empty() { ui.label(&state.selection_message); }
                    if ui.add_enabled(!state.checking, egui::Button::new("Refresh devices")).clicked() {
                        openrgb_start_probe(ui.ctx().clone());
                    }
                    if ui.button("Clear saved selection").clicked() {
                        let result = openrgb_save_selection(None);
                        let mut shared = openrgb_ui_state().lock().unwrap_or_else(|e| e.into_inner());
                        match result {
                            Ok(()) => { shared.selected_identity = None; shared.selection_message = "Device selection cleared.".into(); }
                            Err(e) => shared.selection_message = e,
                        }
                    }
                    ui.add_space(6.0);
                    for device in &state.devices {
                        ui.group(|ui| {
                            ui.label(egui::RichText::new(&device.name).strong());
                            ui.label(format!("Controller ID: {}  |  LEDs: {}  |  Zones: {}  |  Modes: {}", device.id, device.leds, device.zones, device.modes));
                            if !device.serial.is_empty() { ui.label(format!("Serial: {}", device.serial)); }
                            ui.label(if device.matrix { "Matrix layout: reported" } else { "Matrix layout: unavailable or unsupported" });
                            if !device.eligible { ui.label(format!("Not eligible for spatial mapping: {}", device.reason)); }
                            let matches = state.devices.iter().filter(|other| other.identity == device.identity).count();
                            let selected = state.selected_identity.as_deref() == Some(device.identity.as_str());
                            if selected { ui.label("Saved selection — device already selected"); }
                            if ui.add_enabled(device.eligible && matches == 1 && !selected,
                                egui::Button::new("Select device")).clicked() {
                                let result = openrgb_save_selection(Some(device));
                                let mut shared = openrgb_ui_state().lock().unwrap_or_else(|e| e.into_inner());
                                match result {
                                    Ok(()) => { shared.selected_identity = Some(device.identity.clone());
                                        shared.selection_message = "Device selection saved. No lighting commands sent.".into(); }
                                    Err(e) => shared.selection_message = e,
                                }
                            }
                            if matches != 1 { ui.label("Ambiguous identity: selection disabled"); }
                        });
                        ui.add_space(6.0);
                    }
                    ui.label("Configuration only. Hardware acquisition and LED writes remain disabled.");
                }
                OpenRgbNestedTab::Diagnostics => {
                    ui.heading("OpenRGB Diagnostics");
                    ui.add_space(8.0);
                    ui.label("Read-only topology analysis and physical LED verification.");
                    let matches: Vec<_> = state.devices.iter().filter(|d|
                        state.selected_identity.as_deref() == Some(d.identity.as_str())).collect();
                    if matches.len() != 1 {
                        ui.label("Select and discover a unique device under Devices first.");
                    } else {
                        let device = matches[0];
                        ui.label(format!("Device: {} (controller {})", device.name, device.id));
                        egui::CollapsingHeader::new("LED topology")
                            .default_open(true)
                            .show(ui, |ui| {
                                // Keep long per-LED diagnostics inside a bounded viewport;
                                // never let the report expand the Control Center vertically.
                                egui::ScrollArea::vertical()
                                    .id_source(("openrgb_topology_report_scroll", device.id))
                                    .max_height(210.0)
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new(&device.topology_diagnostic).monospace());
                                    });
                                let report = format!("OpenRGB device: {}\nController ID: {}\n{}",
                                    device.name, device.id, device.topology_diagnostic);
                                ui.horizontal(|ui| {
                                    if ui.button("Copy report").clicked() {
                                        let result = openrgb_copy_report(&report);
                                        let message = match result {
                                            Ok(message) => message,
                                            Err(error) => error,
                                        };
                                        ui.ctx().data_mut(|data| data.insert_temp(
                                            egui::Id::new("openrgb_clipboard_result"), message));
                                    }
                                    if ui.button("Write to runtime log").clicked() {
                                        crate::logger::information(
                                            &crate::locate_paths::runtime_log_path(),
                                            &format!("[OPENRGB TOPOLOGY] {report}"));
                                    }
                                });
                                if let Some(message) = ui.ctx().data(|data| data.get_temp::<String>(
                                    egui::Id::new("openrgb_clipboard_result"))) {
                                    ui.small(message);
                                }
                            });
                        ui.add_space(10.0);
                        egui::CollapsingHeader::new("Physical LED verification")
                            .default_open(true)
                            .show(ui, |ui| {
                                openrgb_reap_test();
                                ui.label("Neon Orange (#FF5F1F), smooth 1 Hz pulse, 15-second maximum. Only explicitly selected LEDs illuminate.");
                                let pattern_id = egui::Id::new("openrgb_physical_pattern");
                                let mut pattern = ui.ctx().data(|d| d.get_temp::<usize>(pattern_id).unwrap_or(0));
                                let rows = device.topology.as_ref().map_or(0, |m| m.rows);
                                let total = device.leds;
                                let options = 2 + rows + total;
                                if pattern >= options { pattern = 0; }
                                egui::ComboBox::from_id_source("openrgb_physical_pattern_selector")
                                    .selected_text(if pattern == 0 { "Entire mapped device".into() }
                                        else if pattern == 1 { "Unreferenced controller LEDs".into() }
                                        else if pattern < 2 + rows { format!("Matrix row {}", pattern - 1) }
                                        else { format!("Controller LED {}", pattern - 2 - rows) })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut pattern, 0, "Entire mapped device");
                                        ui.selectable_value(&mut pattern, 1, "Unreferenced controller LEDs");
                                        for row in 0..rows { ui.selectable_value(&mut pattern, row + 2, format!("Matrix row {}", row + 1)); }
                                        for led in 0..total { ui.selectable_value(&mut pattern, led + 2 + rows, format!("Controller LED {led}")); }
                                    });
                                ui.ctx().data_mut(|d| d.insert_temp(pattern_id, pattern));
                                let running = openrgb_test_running();
                                ui.horizontal(|ui| {
                                    if ui.add_enabled(!running && device.eligible, egui::Button::new("Start physical LED test")).clicked() {
                                        let mode = if pattern == 0 { OpenRgbTestPattern::All }
                                            else if pattern == 1 { OpenRgbTestPattern::Unreferenced }
                                            else if pattern < 2 + rows { OpenRgbTestPattern::Row(pattern - 2) }
                                            else { OpenRgbTestPattern::Led(pattern - 2 - rows) };
                                        openrgb_start_physical_test(state.checked_endpoint.clone(), device.id, mode);
                                    }
                                    if ui.add_enabled(running, egui::Button::new("Stop test")).clicked() { openrgb_stop_test(); }
                                });
                                ui.small(openrgb_physical_message().lock().unwrap_or_else(|e| e.into_inner()).clone());
                                if running { ui.ctx().request_repaint_after(std::time::Duration::from_millis(100)); }
                            });
                        egui::CollapsingHeader::new("Verification results")
                            .show(ui, |ui| {
                                ui.label("No physical LEDs have been visually verified yet.");
                                ui.label("Topology validity does not establish physical visibility.");
                            });
                    }
                }
                OpenRgbNestedTab::Mapping => {
                    ui.heading("Mapping");
                    ui.add_space(4.0);
                    openrgb_draw_mapping_editor(ui, &state);
                }
                OpenRgbNestedTab::Settings => {
                    ui.heading("Settings");
                    ui.add_space(8.0);
                    ui.label("OpenRGB SDK server endpoint (host:port):");
                    let mut endpoint = state.endpoint.clone();
                    if ui.text_edit_singleline(&mut endpoint).changed() {
                        let mut shared = openrgb_ui_state().lock().unwrap_or_else(|e| e.into_inner());
                        shared.endpoint = endpoint;
                    }
                    ui.label("Default: 127.0.0.1:6742. Remote servers are supported.");
                    if !state.settings_error.is_empty() {
                        ui.colored_label(ui.visuals().error_fg_color, &state.settings_error);
                    }
                    ui.horizontal(|ui| {
                        if ui.add_enabled(state.endpoint != state.saved_endpoint, egui::Button::new("Save")).clicked() {
                            let result = openrgb_save_server(state.endpoint.trim());
                            let mut shared = openrgb_ui_state().lock().unwrap_or_else(|e| e.into_inner());
                            match result {
                                Ok(()) => {
                                    shared.saved_endpoint = state.endpoint.trim().to_owned();
                                    shared.endpoint = shared.saved_endpoint.clone();
                                    shared.settings_error.clear();
                                    shared.checked = false;
                                    shared.connection_ok = false;
                                    shared.devices.clear();
                                    shared.selected_identity = None;
                                    shared.selection_message.clear();
                                }
                                Err(error) => shared.settings_error = error,
                            }
                        }
                        if ui.add_enabled(state.endpoint != state.saved_endpoint, egui::Button::new("Cancel")).clicked() {
                            let mut shared = openrgb_ui_state().lock().unwrap_or_else(|e| e.into_inner());
                            shared.endpoint = shared.saved_endpoint.clone();
                            shared.settings_error.clear();
                        }
                    });
                    if ui.add_enabled(!state.checking, egui::Button::new("Test connection")).clicked() {
                        openrgb_start_probe(ui.ctx().clone());
                    }
                }
                _ => {
                    ui.heading(selected.label());
                    ui.label("This page is reserved for a later, guarded OpenRGB implementation.");
                }
            }
        });
    });
    ui.ctx().data_mut(|data| data.insert_temp(selected_id, selected));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConfigurationNestedTab {
    Appearance,
    Screensaver,
    Wallpaper,
    Rendering,
    Lyrics,
    DataIo,
}


impl ConfigurationNestedTab {
    const ALL: [
        ConfigurationNestedTab;
        6
    ] = [
        ConfigurationNestedTab::Appearance,
        ConfigurationNestedTab::Screensaver,
        ConfigurationNestedTab::Wallpaper,
        ConfigurationNestedTab::Rendering,
        ConfigurationNestedTab::Lyrics,
        ConfigurationNestedTab::DataIo,
    ];


    fn label(
        self,
    ) -> String {
        match self {
            ConfigurationNestedTab::Appearance => {
                crate::manage_localization::runtime_text("tab.appearance")
            }

            ConfigurationNestedTab::Screensaver => {
                crate::manage_localization::runtime_text(
                    "target.screensaver"
                )
            }

            ConfigurationNestedTab::Wallpaper => {
                crate::manage_localization::runtime_text(
                    "target.wallpaper"
                )
            }

            ConfigurationNestedTab::Rendering => {
                crate::manage_localization::runtime_text("tab.rendering")
            }

            ConfigurationNestedTab::Lyrics => {
                crate::manage_localization::runtime_text("tab.lyrics")
            }

            ConfigurationNestedTab::DataIo => {
                crate::manage_localization::runtime_text("tab.data_io")
            }
        }
    }
}



#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PostProcessingNestedTab {
    VisualQuality,
    ImageTransforms,
    Audiovisual,
    AudioMotion,
}


impl PostProcessingNestedTab {
    const ALL: [
        PostProcessingNestedTab;
        4
    ] = [
        PostProcessingNestedTab::VisualQuality,
        PostProcessingNestedTab::ImageTransforms,
        PostProcessingNestedTab::Audiovisual,
        PostProcessingNestedTab::AudioMotion,
    ];


    fn label(
        self,
    ) -> String {
        match self {
            PostProcessingNestedTab::VisualQuality => crate::manage_localization::runtime_text("post.tab.visual_quality"),
            PostProcessingNestedTab::ImageTransforms => crate::manage_localization::runtime_text("post.tab.image_transforms"),
            PostProcessingNestedTab::Audiovisual => crate::manage_localization::runtime_text("post.tab.audiovisual"),
            PostProcessingNestedTab::AudioMotion => crate::manage_localization::runtime_text("post.tab.audio_motion"),
        }
    }
}


pub fn draw_configuration(
    ui: &mut egui::Ui,
    configuration: &mut Option<ControlConfiguration>,
    baseline: Option<&ControlConfiguration>,
    policy_rows: &[PolicyDisplayRow],
    save_requested: &mut bool,
    status_message: &mut String,
    export_destination_browse_requested: &mut Option<std::path::PathBuf>,
    import_archive_browse_requested: &mut Option<std::path::PathBuf>,
    restore_archive_browse_requested: &mut Option<std::path::PathBuf>,
) {
    let Some(configuration) =
        configuration.as_mut()
    else {
        ui.label(
            egui::RichText::new(
                crate::manage_localization::runtime_text("config.unavailable")
            )
            .weak(),
        );
        return;
    };


    let selected_id =
        egui::Id::new(
            "screenshaver_configuration_nested_tab"
        );


    let mut selected =
        ui.ctx()
            .data(
                |data| {
                    data.get_temp::<ConfigurationNestedTab>(
                        selected_id
                    )
                    .unwrap_or(
                        ConfigurationNestedTab::Appearance
                    )
                }
            );


    const FOOTER_HEIGHT: f32 =
        40.0;

    let full_width =
        ui.available_width();

    let full_height =
        ui.available_height();

    let content_height =
        (
            full_height
                - FOOTER_HEIGHT
        )
        .max(
            120.0
        );


    ui.allocate_ui_with_layout(
        egui::vec2(
            full_width,
            content_height,
        ),
        egui::Layout::top_down(
            egui::Align::Min
        ),
        |ui| {
            ui.horizontal(
                |ui| {
                    draw_nested_tab_rail(
                        ui,
                        &mut selected,
                        &ConfigurationNestedTab::ALL,
                        ConfigurationNestedTab::label,
                    );


                    ui.separator();


                    ui.add_space(
                        12.0
                    );


                    ui.vertical(
                        |ui| {
                            ui.set_width(
                                ui.available_width()
                            );

                            match selected {
                                ConfigurationNestedTab::Appearance => {
                                    draw_appearance(
                                        ui,
                                        configuration,
                                    );
                                }

                                ConfigurationNestedTab::Screensaver => {
                                    draw_target_page(
                                        ui,
                                        configuration,
                                        policy_rows,
                                        PolicyTarget::Screensaver,
                                        status_message,
                                    );
                                }

                                ConfigurationNestedTab::Wallpaper => {
                                    draw_target_page(
                                        ui,
                                        configuration,
                                        policy_rows,
                                        PolicyTarget::Wallpaper,
                                        status_message,
                                    );
                                }

                                ConfigurationNestedTab::Rendering => {
                                    draw_rendering_placeholders(
                                        ui,
                                        configuration,
                                    );
                                }

                                ConfigurationNestedTab::Lyrics => {
                                    draw_lyrics(
                                        ui,
                                        configuration,
                                    );
                                }

                                ConfigurationNestedTab::DataIo => {
                                    draw_data_io_shell(
                                        ui,
                                        status_message,
                                        export_destination_browse_requested,
                                        import_archive_browse_requested,
                                        restore_archive_browse_requested,
                                    );
                                }
                            }
                        },
                    );
                },
            );
        },
    );


    let dirty =
        baseline
            .map(
                |baseline| {
                    &*configuration
                        != baseline
                }
            )
            .unwrap_or(
                false
            );

    let single_policy_missing =
        (
            configuration.screensaver_display
                == "single"
                && configuration
                    .screensaver_single_policy_id
                    .is_none()
        )
        || (
            configuration.wallpaper_display
                == "single"
                && configuration
                    .wallpaper_single_policy_id
                    .is_none()
        );


    ui.allocate_ui_with_layout(
        egui::vec2(
            full_width,
            FOOTER_HEIGHT,
        ),
        egui::Layout::left_to_right(
            egui::Align::Center
        ),
        |ui| {
            ui.add_space(
                8.0
            );

            let save_response =
                ui.add_enabled(
                    dirty
                        && !single_policy_missing,
                    egui::Button::new(
                        crate::manage_localization::runtime_text("config.save")
                    ),
                );

            if save_response.clicked() {
                *save_requested =
                    true;

                *status_message =
                    crate::manage_localization::runtime_text("config.saving");
            }


            ui.add_space(
                8.0
            );


            let cancel_response =
                ui.add_enabled(
                    dirty,
                    egui::Button::new(
                        crate::manage_localization::runtime_text("common.cancel")
                    ),
                );

            if cancel_response.clicked() {
                if let Some(baseline) =
                    baseline
                {
                    *configuration =
                        baseline.clone();

                    *status_message =
                        crate::manage_localization::runtime_text("config.discarded");
                }
            }
        },
    );


    ui.ctx()
        .data_mut(
            |data| {
                data.insert_temp(
                    selected_id,
                    selected,
                );
            }
        );
}


fn draw_data_io_shell(
    ui: &mut egui::Ui,
    status_message: &mut String,
    export_destination_browse_requested: &mut Option<std::path::PathBuf>,
    import_archive_browse_requested: &mut Option<std::path::PathBuf>,
    restore_archive_browse_requested: &mut Option<std::path::PathBuf>,
) {
    ui.heading(
        crate::manage_localization::runtime_text("data_io.heading")
    );

    ui.add_space(
        8.0
    );

    ui.label(
        crate::manage_localization::runtime_text("data_io.description")
    );

    ui.add_space(
        12.0
    );

    crate::manage_backup::draw_controls(
        ui,
        status_message,
        restore_archive_browse_requested,
    );

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(12.0);

    ui.label(egui::RichText::new(crate::manage_localization::runtime_text("data_io.portable_data")).strong());
    ui.add_space(6.0);

    ui.horizontal(
        |ui| {
            if ui.button(
                crate::manage_localization::runtime_text("data_io.import")
            )
            .clicked()
            {
                crate::import_data::open(
                    ui.ctx()
                );
            }

            ui.add_space(
                8.0
            );

            if ui.button(
                crate::manage_localization::runtime_text("data_io.export")
            )
            .clicked()
            {
                crate::export_data::open(
                    ui.ctx()
                );
            }
        },
    );


    crate::export_data::draw(
        ui.ctx(),
        export_destination_browse_requested,
    );

    crate::import_data::draw(
        ui.ctx(),
        import_archive_browse_requested,
    );
}


fn draw_nested_tab_rail<T>(
    ui: &mut egui::Ui,
    selected: &mut T,
    tabs: &[T],
    label: impl Fn(T) -> String,
)
where
    T: Copy + PartialEq,
{
    ui.vertical(|ui| {
        let labels: Vec<String> = tabs.iter().map(|&tab| label(tab)).collect();
        let tab_width = crate::editor_theme::vertical_tab_rail_width(ui, &labels, 194.0);
        ui.set_min_width(tab_width + 6.0);
        for &tab in tabs {
            let response = crate::editor_theme::vertical_tab(
                ui, &label(tab), *selected == tab, true, tab_width,
            );
            if response.clicked() {
                *selected = tab;
            }
            ui.add_space(4.0);
        }
    });
}


pub fn draw_post_processing(
    ui: &mut egui::Ui,
    scale: f32,
    shift_held: bool,
    anti_aliasing: &mut AntiAliasingSelection,
    dithering: &mut DitheringSelection,
    color_precision: &mut ColorPrecisionSelection,
    invert_colors: &mut bool,
    flip_horizontal: &mut bool,
    flip_vertical: &mut bool,
    hue_rotation: &mut f32,
    hue_rotation_drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bloom: &mut BloomSelection,
    audio_motion: &mut crate::render_audio_motion::AudioMotionEffect,
    bulk_audio_motion_selected: &mut bool,
    bloom_intensity: &mut f32,
    bloom_intensity_drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bloom_saturation: &mut f32,
    bloom_saturation_drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bloom_threshold: &mut f32,
    bloom_threshold_drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bloom_frequency_rotation: &mut f32,
    bloom_frequency_rotation_drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bloom_frequency_invert: &mut bool,
    bulk_edit_mode: bool,
    bulk_invert_colors: &mut BulkBooleanSelection,
    bulk_flip_horizontal: &mut BulkBooleanSelection,
    bulk_flip_vertical: &mut BulkBooleanSelection,
    bulk_anti_aliasing_selected: &mut bool,
    bulk_dithering_selected: &mut bool,
    bulk_color_precision_selected: &mut bool,
    bulk_bloom_selected: &mut bool,
    bulk_bloom_intensity_selected: &mut bool,
    bulk_bloom_saturation_selected: &mut bool,
    bulk_bloom_threshold_selected: &mut bool,
    bulk_bloom_frequency_rotation_selected: &mut bool,
    bulk_bloom_frequency_invert: &mut BulkBooleanSelection,
    bulk_hue_rotation_selected: &mut bool,
) {
    let selected_id =
        egui::Id::new(
            "screenshaver_post_processing_nested_tab"
        );

    let mut selected =
        ui.ctx()
            .data(
                |data| {
                    data.get_temp::<PostProcessingNestedTab>(
                        selected_id
                    )
                    .unwrap_or(
                        PostProcessingNestedTab::VisualQuality
                    )
                }
            );

    let full_width =
        ui.available_width();

    let full_height =
        ui.available_height();

    ui.allocate_ui_with_layout(
        egui::vec2(
            full_width,
            full_height,
        ),
        egui::Layout::top_down(
            egui::Align::Min
        ),
        |ui| {
            ui.horizontal(
                |ui| {
                    draw_nested_tab_rail(
                        ui,
                        &mut selected,
                        &PostProcessingNestedTab::ALL,
                        PostProcessingNestedTab::label,
                    );

                    ui.separator();
                    ui.add_space(12.0);

                    ui.vertical(
                        |ui| {
                            ui.set_width(
                                ui.available_width()
                            );

                            match selected {
                                PostProcessingNestedTab::VisualQuality => {
                                    draw_post_processing_visual_quality(
                                        ui,
                                        scale,
                                        anti_aliasing,
                                        dithering,
                                        color_precision,
                                        bulk_edit_mode,
                                        bulk_anti_aliasing_selected,
                                        bulk_dithering_selected,
                                        bulk_color_precision_selected,
                                    );
                                }

                                PostProcessingNestedTab::ImageTransforms => {
                                    draw_post_processing_image_transforms(
                                        ui,
                                        scale,
                                        shift_held,
                                        invert_colors,
                                        flip_horizontal,
                                        flip_vertical,
                                        hue_rotation,
                                        hue_rotation_drag_state,
                                        bulk_edit_mode,
                                        bulk_invert_colors,
                                        bulk_flip_horizontal,
                                        bulk_flip_vertical,
                                        bulk_hue_rotation_selected,
                                    );
                                }

                                PostProcessingNestedTab::Audiovisual => {
                                    draw_post_processing_audio(
                                        ui,
                                        scale,
                                        shift_held,
                                        bloom,
                                        bloom_intensity,
                                        bloom_intensity_drag_state,
                                        bloom_saturation,
                                        bloom_saturation_drag_state,
                                        bloom_threshold,
                                        bloom_threshold_drag_state,
                                        bloom_frequency_rotation,
                                        bloom_frequency_rotation_drag_state,
                                        bloom_frequency_invert,
                                        bulk_edit_mode,
                                        bulk_bloom_selected,
                                        bulk_bloom_intensity_selected,
                                        bulk_bloom_saturation_selected,
                                        bulk_bloom_threshold_selected,
                                        bulk_bloom_frequency_rotation_selected,
                                        bulk_bloom_frequency_invert,
                                    );
                                }

                                PostProcessingNestedTab::AudioMotion => {
                                    draw_post_processing_audio_motion(
                                        ui,
                                        scale,
                                        audio_motion,
                                        bulk_edit_mode,
                                        bulk_audio_motion_selected,
                                    );
                                }
                            }
                        },
                    );
                },
            );
        },
    );

    ui.ctx()
        .data_mut(
            |data| {
                data.insert_temp(
                    selected_id,
                    selected,
                );
            }
        );
}


const POST_PROCESSING_CONTROL_WIDTH: f32 =
    190.0;

const POST_PROCESSING_SLIDER_WIDTH: f32 =
    240.0;


fn draw_post_processing_audio_motion(
    ui: &mut egui::Ui,
    scale: f32,
    audio_motion: &mut crate::render_audio_motion::AudioMotionEffect,
    bulk_edit_mode: bool,
    bulk_audio_motion_selected: &mut bool,
) {
    ui.heading(crate::manage_localization::runtime_text("post.tab.audio_motion"));
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        ui.label(crate::manage_localization::runtime_text("post.motion.effect"))
            .on_hover_text(crate::manage_localization::runtime_text("post.motion.effect_help"));

        let selected_text = if bulk_edit_mode && !*bulk_audio_motion_selected {
            crate::manage_localization::runtime_text("post.common.unchanged")
        } else {
            audio_motion.display_name().to_string()
        };

        let response = egui::ComboBox::from_id_source("post_processing_audio_motion_effect")
            .selected_text(selected_text)
            .width(POST_PROCESSING_CONTROL_WIDTH)
            .show_ui(ui, |ui| {
                if bulk_edit_mode {
                    if ui.selectable_label(!*bulk_audio_motion_selected, crate::manage_localization::runtime_text("post.common.unchanged")).clicked() {
                        *bulk_audio_motion_selected = false;
                    }
                    ui.separator();
                }

                if ui.selectable_value(audio_motion, crate::render_audio_motion::AudioMotionEffect::Off, crate::manage_localization::runtime_text("post.common.off")).clicked() && bulk_edit_mode {
                    *bulk_audio_motion_selected = true;
                }
                if ui.selectable_value(audio_motion, crate::render_audio_motion::AudioMotionEffect::WooferFromHell, crate::manage_localization::runtime_text("post.motion.woofer")).clicked() && bulk_edit_mode {
                    *bulk_audio_motion_selected = true;
                }
                if ui.selectable_value(audio_motion, crate::render_audio_motion::AudioMotionEffect::FftMirrorWarp, crate::manage_localization::runtime_text("post.motion.fft_mirror")).clicked() && bulk_edit_mode {
                    *bulk_audio_motion_selected = true;
                }
                if ui.selectable_value(audio_motion, crate::render_audio_motion::AudioMotionEffect::PolarPropeller, crate::manage_localization::runtime_text("post.motion.polar_propeller")).clicked() && bulk_edit_mode {
                    *bulk_audio_motion_selected = true;
                }
            })
            .response;

        if bulk_edit_mode && *bulk_audio_motion_selected {
            crate::editor_theme::paint_bulk_edit_border(ui, response.rect, scale);
        }
    });
}


fn draw_post_processing_visual_quality(
    ui: &mut egui::Ui,
    scale: f32,
    anti_aliasing: &mut AntiAliasingSelection,
    dithering: &mut DitheringSelection,
    color_precision: &mut ColorPrecisionSelection,
    bulk_edit_mode: bool,
    bulk_anti_aliasing_selected: &mut bool,
    bulk_dithering_selected: &mut bool,
    bulk_color_precision_selected: &mut bool,
) {
    ui.heading(crate::manage_localization::runtime_text("post.tab.visual_quality"));
    ui.add_space(8.0);

    egui::Grid::new(
        "post_processing_visual_quality_grid"
    )
    .num_columns(2)
    .spacing(egui::vec2(8.0, 8.0))
    .show(
        ui,
        |ui| {
            ui.label(crate::manage_localization::runtime_text("post.visual.anti_aliasing"))
                .on_hover_text(
                    crate::manage_localization::runtime_text("post.visual.anti_aliasing_help")
                );

            let selected_text =
                if bulk_edit_mode
                    && !*bulk_anti_aliasing_selected
                {
                    crate::manage_localization::runtime_text("post.common.unchanged")
                } else {
                    match *anti_aliasing {
                        AntiAliasingSelection::Off => crate::manage_localization::runtime_text("post.common.off"),
                        AntiAliasingSelection::Fxaa => "FXAA".to_string(),
                    }
                };

            let response =
                egui::ComboBox::from_id_source(
                "post_processing_anti_aliasing"
            )
            .selected_text(selected_text)
            .width(POST_PROCESSING_CONTROL_WIDTH)
            .show_ui(
                ui,
                |ui| {
                    if bulk_edit_mode {
                        if ui.selectable_label(
                            !*bulk_anti_aliasing_selected,
                            crate::manage_localization::runtime_text("post.common.unchanged"),
                        ).clicked() {
                            *bulk_anti_aliasing_selected = false;
                        }
                        ui.separator();
                    }

                    if ui.selectable_value(
                        anti_aliasing,
                        AntiAliasingSelection::Off,
                        crate::manage_localization::runtime_text("post.common.off"),
                    ).clicked() && bulk_edit_mode {
                        *bulk_anti_aliasing_selected = true;
                    }

                    if ui.selectable_value(
                        anti_aliasing,
                        AntiAliasingSelection::Fxaa,
                        "FXAA",
                    ).clicked() && bulk_edit_mode {
                        *bulk_anti_aliasing_selected = true;
                    }
                },
            )
                .response;

            if bulk_edit_mode
                && *bulk_anti_aliasing_selected
            {
                crate::editor_theme::paint_bulk_edit_border(
                    ui,
                    response.rect,
                    scale,
                );
            }

            ui.end_row();

            ui.label(crate::manage_localization::runtime_text("rendering.dithering"))
                .on_hover_text(
                    crate::manage_localization::runtime_text("post.visual.dithering_help")
                );

            let selected_text =
                if bulk_edit_mode
                    && !*bulk_dithering_selected
                {
                    crate::manage_localization::runtime_text("post.common.unchanged")
                } else {
                    match *dithering {
                        DitheringSelection::Off => crate::manage_localization::runtime_text("post.common.off"),
                        DitheringSelection::Subtle => crate::manage_localization::runtime_text("post.visual.subtle"),
                    }
                };

            let response =
                egui::ComboBox::from_id_source(
                "post_processing_dithering"
            )
            .selected_text(selected_text)
            .width(POST_PROCESSING_CONTROL_WIDTH)
            .show_ui(
                ui,
                |ui| {
                    if bulk_edit_mode {
                        if ui.selectable_label(
                            !*bulk_dithering_selected,
                            crate::manage_localization::runtime_text("post.common.unchanged"),
                        ).clicked() {
                            *bulk_dithering_selected = false;
                        }
                        ui.separator();
                    }

                    if ui.selectable_value(
                        dithering,
                        DitheringSelection::Off,
                        crate::manage_localization::runtime_text("post.common.off"),
                    ).clicked() && bulk_edit_mode {
                        *bulk_dithering_selected = true;
                    }

                    if ui.selectable_value(
                        dithering,
                        DitheringSelection::Subtle,
                        crate::manage_localization::runtime_text("post.visual.subtle"),
                    ).clicked() && bulk_edit_mode {
                        *bulk_dithering_selected = true;
                    }
                },
            )
                .response;

            if bulk_edit_mode
                && *bulk_dithering_selected
            {
                crate::editor_theme::paint_bulk_edit_border(
                    ui,
                    response.rect,
                    scale,
                );
            }

            ui.end_row();

            ui.label(crate::manage_localization::runtime_text("post.visual.color_precision"))
                .on_hover_text(
                    crate::manage_localization::runtime_text("post.visual.color_precision_help")
                );

            let selected_text =
                if bulk_edit_mode
                    && !*bulk_color_precision_selected
                {
                    crate::manage_localization::runtime_text("post.common.unchanged")
                } else {
                    match *color_precision {
                        ColorPrecisionSelection::Automatic => crate::manage_localization::runtime_text("post.visual.automatic"),
                        ColorPrecisionSelection::Standard => crate::manage_localization::runtime_text("post.visual.standard_precision"),
                        ColorPrecisionSelection::High => crate::manage_localization::runtime_text("post.visual.high_precision"),
                    }
                };

            let response =
                egui::ComboBox::from_id_source(
                "post_processing_color_precision"
            )
            .selected_text(selected_text)
            .width(POST_PROCESSING_CONTROL_WIDTH)
            .show_ui(
                ui,
                |ui| {
                    if bulk_edit_mode {
                        if ui.selectable_label(
                            !*bulk_color_precision_selected,
                            crate::manage_localization::runtime_text("post.common.unchanged"),
                        ).clicked() {
                            *bulk_color_precision_selected = false;
                        }
                        ui.separator();
                    }

                    for (choice, label) in [
                        (ColorPrecisionSelection::Automatic, crate::manage_localization::runtime_text("post.visual.automatic")),
                        (ColorPrecisionSelection::Standard, crate::manage_localization::runtime_text("post.visual.standard_precision")),
                        (ColorPrecisionSelection::High, crate::manage_localization::runtime_text("post.visual.high_precision")),
                    ] {
                        if ui.selectable_value(
                            color_precision,
                            choice,
                            label,
                        ).clicked() && bulk_edit_mode {
                            *bulk_color_precision_selected = true;
                        }
                    }
                },
            )
                .response;

            if bulk_edit_mode
                && *bulk_color_precision_selected
            {
                crate::editor_theme::paint_bulk_edit_border(
                    ui,
                    response.rect,
                    scale,
                );
            }

            ui.end_row();
        },
    );
}


fn draw_post_processing_image_transforms(
    ui: &mut egui::Ui,
    scale: f32,
    shift_held: bool,
    invert_colors: &mut bool,
    flip_horizontal: &mut bool,
    flip_vertical: &mut bool,
    hue_rotation: &mut f32,
    hue_rotation_drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bulk_edit_mode: bool,
    bulk_invert_colors: &mut BulkBooleanSelection,
    bulk_flip_horizontal: &mut BulkBooleanSelection,
    bulk_flip_vertical: &mut BulkBooleanSelection,
    bulk_hue_rotation_selected: &mut bool,
) {
    ui.heading(crate::manage_localization::runtime_text("post.tab.image_transforms"));
    ui.add_space(8.0);

    if bulk_edit_mode {
        draw_bulk_boolean_row(
            ui,
            "post_processing_bulk_invert_colors",
            scale,
            crate::manage_localization::runtime_text("post.transform.invert_colors"),
            bulk_invert_colors,
            crate::manage_localization::runtime_text("post.transform.invert_colors_help"),
        );
        draw_bulk_boolean_row(
            ui,
            "post_processing_bulk_flip_horizontal",
            scale,
            crate::manage_localization::runtime_text("post.transform.flip_horizontal"),
            bulk_flip_horizontal,
            crate::manage_localization::runtime_text("post.transform.flip_horizontal_help"),
        );
        draw_bulk_boolean_row(
            ui,
            "post_processing_bulk_flip_vertical",
            scale,
            crate::manage_localization::runtime_text("post.transform.flip_vertical"),
            bulk_flip_vertical,
            crate::manage_localization::runtime_text("post.transform.flip_vertical_help"),
        );
    } else {
        ui.checkbox(
            invert_colors,
            crate::manage_localization::runtime_text("post.transform.invert_colors"),
        )
        .on_hover_text(
            crate::manage_localization::runtime_text("post.transform.invert_colors_help")
        );

        ui.checkbox(
            flip_horizontal,
            crate::manage_localization::runtime_text("post.transform.flip_horizontal"),
        )
        .on_hover_text(
            crate::manage_localization::runtime_text("post.transform.flip_horizontal_help")
        );

        ui.checkbox(
            flip_vertical,
            crate::manage_localization::runtime_text("post.transform.flip_vertical"),
        )
        .on_hover_text(
            crate::manage_localization::runtime_text("post.transform.flip_vertical_help")
        );
    }

    ui.add_space(10.0);

    draw_numeric_slider_row(
        ui,
        crate::manage_localization::runtime_text("post.transform.hue_rotation"),
        hue_rotation,
        crate::postprocess_shader::HUE_ROTATION_MIN,
        crate::postprocess_shader::HUE_ROTATION_MAX,
        "°",
        shift_held,
        hue_rotation_drag_state,
        bulk_edit_mode,
        bulk_hue_rotation_selected,
        true,
        scale,
        crate::manage_localization::runtime_text("post.transform.hue_rotation_help"),
    );
}


fn draw_post_processing_audio(
    ui: &mut egui::Ui,
    scale: f32,
    shift_held: bool,
    bloom: &mut BloomSelection,
    bloom_intensity: &mut f32,
    bloom_intensity_drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bloom_saturation: &mut f32,
    bloom_saturation_drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bloom_threshold: &mut f32,
    bloom_threshold_drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bloom_frequency_rotation: &mut f32,
    bloom_frequency_rotation_drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bloom_frequency_invert: &mut bool,
    bulk_edit_mode: bool,
    bulk_bloom_selected: &mut bool,
    bulk_bloom_intensity_selected: &mut bool,
    bulk_bloom_saturation_selected: &mut bool,
    bulk_bloom_threshold_selected: &mut bool,
    bulk_bloom_frequency_rotation_selected: &mut bool,
    bulk_bloom_frequency_invert: &mut BulkBooleanSelection,
) {
    ui.heading(crate::manage_localization::runtime_text("post.tab.audiovisual"));
    ui.add_space(8.0);

    ui.horizontal(
        |ui| {
            ui.label(crate::manage_localization::runtime_text("post.audio.effect"))
                .on_hover_text(
                    crate::manage_localization::runtime_text("post.audio.effect_help")
                );

            let selected_text =
                if bulk_edit_mode
                    && !*bulk_bloom_selected
                {
                    crate::manage_localization::runtime_text("post.common.unchanged")
                } else {
                    match *bloom {
                        BloomSelection::Off => crate::manage_localization::runtime_text("post.common.off"),
                        BloomSelection::Audio => crate::manage_localization::runtime_text("post.audio.audio_bloom"),
                        BloomSelection::Spectral => crate::manage_localization::runtime_text("post.audio.spectral_bloom"),
                        BloomSelection::Loudness => crate::manage_localization::runtime_text("post.audio.loudness_bloom"),
                    }
                };

            let response =
                egui::ComboBox::from_id_source(
                "post_processing_bloom_mode"
            )
            .selected_text(selected_text)
            .width(150.0)
            .show_ui(
                ui,
                |ui| {
                    if bulk_edit_mode {
                        if ui.selectable_label(
                            !*bulk_bloom_selected,
                            crate::manage_localization::runtime_text("post.common.unchanged"),
                        ).clicked() {
                            *bulk_bloom_selected = false;
                        }
                        ui.separator();
                    }

                    if ui.selectable_value(
                        bloom,
                        BloomSelection::Off,
                        crate::manage_localization::runtime_text("post.common.off"),
                    ).clicked() && bulk_edit_mode {
                        *bulk_bloom_selected = true;
                    }

                    if ui.selectable_value(
                        bloom,
                        BloomSelection::Audio,
                        crate::manage_localization::runtime_text("post.audio.audio_bloom"),
                    ).clicked() && bulk_edit_mode {
                        *bulk_bloom_selected = true;
                    }


                    if ui.selectable_value(
                        bloom,
                        BloomSelection::Spectral,
                        crate::manage_localization::runtime_text("post.audio.spectral_bloom"),
                    ).clicked() && bulk_edit_mode {
                        *bulk_bloom_selected = true;
                    }


                    if ui.selectable_value(
                        bloom,
                        BloomSelection::Loudness,
                        crate::manage_localization::runtime_text("post.audio.loudness_bloom"),
                    ).clicked() && bulk_edit_mode {
                        *bulk_bloom_selected = true;
                    }
                },
            )
                .response;

            if bulk_edit_mode
                && *bulk_bloom_selected
            {
                crate::editor_theme::paint_bulk_edit_border(
                    ui,
                    response.rect,
                    scale,
                );
            }
        },
    );

    ui.add_space(8.0);

    // In ordinary editing, Off disables dependent Bloom parameters.
    // In Bulk Edit, an Unchanged Bloom Mode still permits independently
    // applying numeric Bloom values across the selected policies.
    let bloom_controls_available =
        if bulk_edit_mode
            && !*bulk_bloom_selected
        {
            true
        } else {
            *bloom != BloomSelection::Off
        };

    let frequency_controls_available =
        if bulk_edit_mode
            && !*bulk_bloom_selected
        {
            true
        } else {
            bloom_controls_available
                && *bloom != BloomSelection::Loudness
        };

    egui::Grid::new(
        "post_processing_audio_grid"
    )
    .num_columns(2)
    .spacing(egui::vec2(8.0, 8.0))
    .show(
        ui,
        |ui| {
            draw_numeric_slider_grid_row(
                ui,
                crate::manage_localization::runtime_text("post.audio.bloom_intensity"),
                bloom_intensity,
                crate::render_bloom::BLOOM_INTENSITY_MIN,
                crate::render_bloom::BLOOM_INTENSITY_MAX,
                "",
                shift_held,
                bloom_intensity_drag_state,
                bulk_edit_mode,
                bulk_bloom_intensity_selected,
                bloom_controls_available,
                scale,
                crate::manage_localization::runtime_text("post.audio.bloom_intensity_help"),
            );

            draw_numeric_slider_grid_row(
                ui,
                crate::manage_localization::runtime_text("post.audio.bloom_saturation"),
                bloom_saturation,
                crate::render_bloom::BLOOM_SATURATION_MIN,
                crate::render_bloom::BLOOM_SATURATION_MAX,
                "",
                shift_held,
                bloom_saturation_drag_state,
                bulk_edit_mode,
                bulk_bloom_saturation_selected,
                bloom_controls_available,
                scale,
                crate::manage_localization::runtime_text("post.audio.bloom_saturation_help"),
            );

            draw_numeric_slider_grid_row(
                ui,
                crate::manage_localization::runtime_text("post.audio.bloom_threshold"),
                bloom_threshold,
                crate::render_bloom::BLOOM_THRESHOLD_MIN,
                crate::render_bloom::BLOOM_THRESHOLD_MAX,
                "",
                shift_held,
                bloom_threshold_drag_state,
                bulk_edit_mode,
                bulk_bloom_threshold_selected,
                bloom_controls_available,
                scale,
                crate::manage_localization::runtime_text("post.audio.bloom_threshold_help"),
            );

            draw_numeric_slider_grid_row(
                ui,
                crate::manage_localization::runtime_text("post.audio.frequency_rotation"),
                bloom_frequency_rotation,
                crate::render_bloom::BLOOM_FREQUENCY_ROTATION_MIN,
                crate::render_bloom::BLOOM_FREQUENCY_ROTATION_MAX,
                "°",
                shift_held,
                bloom_frequency_rotation_drag_state,
                bulk_edit_mode,
                bulk_bloom_frequency_rotation_selected,
                frequency_controls_available,
                scale,
                crate::manage_localization::runtime_text("post.audio.frequency_rotation_help"),
            );
        },
    );

    ui.add_space(8.0);

    ui.add_enabled_ui(
        frequency_controls_available,
        |ui| {
            if bulk_edit_mode {
                draw_bulk_boolean_row(
                    ui,
                    "post_processing_bulk_bloom_frequency_invert",
                    scale,
                    crate::manage_localization::runtime_text("post.audio.invert_frequency"),
                    bulk_bloom_frequency_invert,
                    crate::manage_localization::runtime_text("post.audio.invert_frequency_help"),
                );
            } else {
                ui.checkbox(
                    bloom_frequency_invert,
                    crate::manage_localization::runtime_text("post.audio.invert_frequency"),
                )
                .on_hover_text(
                    crate::manage_localization::runtime_text("post.audio.invert_frequency_help")
                );
            }
        },
    );
}


fn draw_bulk_boolean_row(
    ui: &mut egui::Ui,
    id: &'static str,
    scale: f32,
    label: String,
    selection: &mut BulkBooleanSelection,
    help: String,
) {
    ui.horizontal(
        |ui| {
            ui.label(label)
                .on_hover_text(help);

            let selected_text =
                match *selection {
                    BulkBooleanSelection::Unchanged => crate::manage_localization::runtime_text("post.common.unchanged"),
                    BulkBooleanSelection::True => crate::manage_localization::runtime_text("post.common.enabled"),
                    BulkBooleanSelection::False => crate::manage_localization::runtime_text("post.common.disabled"),
                };

            let response =
                egui::ComboBox::from_id_source(id)
                .selected_text(selected_text)
                .width(POST_PROCESSING_CONTROL_WIDTH)
                .show_ui(
                    ui,
                    |ui| {
                        ui.selectable_value(
                            selection,
                            BulkBooleanSelection::Unchanged,
                            crate::manage_localization::runtime_text("post.common.unchanged"),
                        );
                        ui.separator();
                        ui.selectable_value(
                            selection,
                            BulkBooleanSelection::True,
                            crate::manage_localization::runtime_text("common.enabled"),
                        );
                        ui.selectable_value(
                            selection,
                            BulkBooleanSelection::False,
                            crate::manage_localization::runtime_text("post.common.disabled"),
                        );
                    },
                )
                .response;


            if !matches!(
                *selection,
                BulkBooleanSelection::Unchanged
            ) {
                crate::editor_theme::paint_bulk_edit_border(
                    ui,
                    response.rect,
                    scale,
                );
            }
        },
    );
}


fn draw_numeric_slider_row(
    ui: &mut egui::Ui,
    label: String,
    value: &mut f32,
    minimum: f32,
    maximum: f32,
    suffix: &'static str,
    shift_held: bool,
    drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bulk_edit_mode: bool,
    bulk_selected: &mut bool,
    control_available: bool,
    scale: f32,
    help: String,
) {
    ui.horizontal(
        |ui| {
            ui.label(label)
                .on_hover_text(help);
            draw_numeric_slider_control(
                ui,
                value,
                minimum,
                maximum,
                suffix,
                shift_held,
                drag_state,
                bulk_edit_mode,
                bulk_selected,
                control_available,
                scale,
            );
        },
    );
}


fn draw_numeric_slider_grid_row(
    ui: &mut egui::Ui,
    label: String,
    value: &mut f32,
    minimum: f32,
    maximum: f32,
    suffix: &'static str,
    shift_held: bool,
    drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bulk_edit_mode: bool,
    bulk_selected: &mut bool,
    control_available: bool,
    scale: f32,
    help: String,
) {
    ui.label(label)
        .on_hover_text(help);
    draw_numeric_slider_control(
        ui,
        value,
        minimum,
        maximum,
        suffix,
        shift_held,
        drag_state,
        bulk_edit_mode,
        bulk_selected,
        control_available,
        scale,
    );
    ui.end_row();
}


fn draw_numeric_slider_control(
    ui: &mut egui::Ui,
    value: &mut f32,
    minimum: f32,
    maximum: f32,
    suffix: &'static str,
    shift_held: bool,
    drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bulk_edit_mode: bool,
    bulk_selected: &mut bool,
    control_available: bool,
    scale: f32,
) {
    ui.horizontal(
        |ui| {
            ui.spacing_mut().slider_width =
                POST_PROCESSING_SLIDER_WIDTH;

            let slider_enabled =
                control_available
                    && (
                        !bulk_edit_mode
                            || *bulk_selected
                    );

            ui.add_enabled_ui(
                slider_enabled,
                |ui| {
                    // Reuse the Control Center's established fine-drag slider
                    // behavior. Holding Shift reduces drag sensitivity to 0.1x
                    // (10x finer), and the helper re-anchors if Shift changes
                    // while the pointer is already dragging.
                    ui.allocate_ui_with_layout(
                        egui::vec2(
                            POST_PROCESSING_SLIDER_WIDTH,
                            ui.spacing().interact_size.y,
                        ),
                        egui::Layout::left_to_right(
                            egui::Align::Center
                        ),
                        |ui| {
                            ui.set_width(
                                POST_PROCESSING_SLIDER_WIDTH
                            );

                            crate::editor_layout::draw_fine_slider(
                                ui,
                                value,
                                minimum,
                                maximum,
                                shift_held,
                                scale,
                                drag_state,
                            );
                        },
                    );
                },
            );

            let displayed_value =
                if suffix == "°" {
                    format!("{:.1}°", *value)
                } else {
                    format!("{:.2}", *value)
                };

            if bulk_edit_mode {
                let response =
                    ui.add_enabled(
                        control_available,
                        egui::Button::new(displayed_value),
                    )
                    .on_hover_cursor(
                        egui::CursorIcon::PointingHand
                    )
                    .on_hover_text(
                        if *bulk_selected {
                            crate::manage_localization::runtime_text("post.bulk.exclude")
                        } else {
                            crate::manage_localization::runtime_text("post.bulk.include")
                        }
                    );

                if response.clicked() {
                    *bulk_selected =
                        !*bulk_selected;
                }

                if *bulk_selected {
                    crate::editor_theme::paint_bulk_edit_border(
                        ui,
                        response.rect,
                        scale,
                    );
                }
            } else {
                ui.label(displayed_value);
            }
        },
    );
}


fn draw_appearance(
    ui: &mut egui::Ui,
    configuration: &mut ControlConfiguration,
) {
    ui.heading(crate::manage_localization::runtime_text("appearance.heading"));
    ui.add_space(8.0);

    ui.checkbox(&mut configuration.show_splash, crate::manage_localization::runtime_text("appearance.show_splash"));
    ui.checkbox(&mut configuration.control_center_rgb_enabled, "Enable OpenRGB for Control Center shader preview");
    ui.checkbox(&mut configuration.subtitles, crate::manage_localization::runtime_text("appearance.screensaver_subtitles"));
    ui.add_space(5.0);

    egui::Grid::new("nested_config_grid_appearance")
        .num_columns(2)
        .spacing(egui::vec2(8.0, 6.0))
        .show(ui, |ui| {
            ui.label(crate::manage_localization::runtime_text("appearance.subtitle_placement"));
            egui::ComboBox::from_id_source("nested_config_subtitle_placement")
                .selected_text(
                    match configuration.subtitle_placement.as_str() {
                        "top:left" => crate::manage_localization::runtime_text("placement.top_left"),
                        "top:center" => crate::manage_localization::runtime_text("placement.top_center"),
                        "top:right" => crate::manage_localization::runtime_text("placement.top_right"),
                        "bottom:left" => crate::manage_localization::runtime_text("placement.bottom_left"),
                        "bottom:center" => crate::manage_localization::runtime_text("placement.bottom_center"),
                        "bottom:right" => crate::manage_localization::runtime_text("placement.bottom_right"),
                        _ => configuration.subtitle_placement.clone(),
                    }
                )
                .width(190.0)
                .show_ui(ui, |ui| {
                    for (choice, label_key) in [
                        ("top:left", "placement.top_left"),
                        ("top:center", "placement.top_center"),
                        ("top:right", "placement.top_right"),
                        ("bottom:left", "placement.bottom_left"),
                        ("bottom:center", "placement.bottom_center"),
                        ("bottom:right", "placement.bottom_right"),
                    ] {
                        ui.selectable_value(
                            &mut configuration.subtitle_placement,
                            choice.to_string(),
                            crate::manage_localization::runtime_text(label_key),
                        );
                    }
                });
            ui.end_row();
        });

    ui.add_space(5.0);
    ui.checkbox(&mut configuration.notifications, crate::manage_localization::runtime_text("appearance.wallpaper_notifications"));
}


fn draw_target_page(
    ui: &mut egui::Ui,
    configuration: &mut ControlConfiguration,
    policy_rows: &[PolicyDisplayRow],
    target: PolicyTarget,
    status_message: &mut String,
) {
    match target {
        PolicyTarget::Screensaver => {
            ui.heading(
                crate::manage_localization::runtime_text("target.screensaver_settings")
            );

            ui.add_space(
                8.0
            );

            ui.checkbox(
                &mut configuration.screensaver_enabled,
                crate::manage_localization::runtime_text("common.enabled"),
            );
            ui.checkbox(&mut configuration.screensaver_rgb_enabled, "Enable OpenRGB for screensavers");

            ui.add_space(
                5.0
            );

            draw_target_grid(
                ui,
                target,
                None,
                &mut configuration.screensaver_display,
                &mut configuration.screensaver_interval_seconds,
                &mut configuration.screensaver_single_policy_id,
                &mut configuration.screensaver_single_policy_name,
                &mut configuration.screensaver_playlist_id,
                &mut configuration.screensaver_playlist_name,
                policy_rows,
                Some(
                    &mut configuration.screensaver_idle_timeout_value
                ),
                Some(
                    &mut configuration.screensaver_idle_timeout_unit
                ),
                &mut configuration.screensaver_animation_speed,
                &mut configuration.screensaver_global_texture,
                &mut configuration.screensaver_texture_primitives,
                &mut configuration.screensaver_global_palette,
                status_message,
            );
        }


        PolicyTarget::Wallpaper => {
            ui.heading(
                crate::manage_localization::runtime_text("target.wallpaper_settings")
            );

            ui.add_space(
                8.0
            );

            ui.checkbox(
                &mut configuration.wallpaper_enabled,
                crate::manage_localization::runtime_text("common.enabled"),
            );
            ui.checkbox(&mut configuration.wallpaper_rgb_enabled, "Enable OpenRGB for wallpapers and windowshaders");

            ui.add_space(
                5.0
            );

            draw_target_grid(
                ui,
                target,
                Some(
                    &mut configuration.wallpaper_display_format
                ),
                &mut configuration.wallpaper_display,
                &mut configuration.wallpaper_interval_seconds,
                &mut configuration.wallpaper_single_policy_id,
                &mut configuration.wallpaper_single_policy_name,
                &mut configuration.wallpaper_playlist_id,
                &mut configuration.wallpaper_playlist_name,
                policy_rows,
                None,
                None,
                &mut configuration.wallpaper_animation_speed,
                &mut configuration.wallpaper_global_texture,
                &mut configuration.wallpaper_texture_primitives,
                &mut configuration.wallpaper_global_palette,
                status_message,
            );
        }


        PolicyTarget::Unassigned => {}
    }
}


#[allow(clippy::too_many_arguments)]
fn draw_target_grid(
    ui: &mut egui::Ui,
    target: PolicyTarget,
    wallpaper_display_format: Option<
        &mut crate::manage_configuration::WallpaperDisplayFormat
    >,
    display_mode: &mut String,
    interval_seconds: &mut u64,
    single_policy_id: &mut Option<i64>,
    single_policy_name: &mut String,
    playlist_id: &mut Option<i64>,
    playlist_name: &mut String,
    policy_rows: &[PolicyDisplayRow],
    idle_timeout_value: Option<&mut i64>,
    idle_timeout_unit: Option<&mut String>,
    animation_speed: &mut f64,
    global_texture: &mut String,
    texture_primitives: &mut i64,
    global_palette: &mut String,
    status_message: &mut String,
) {
    const DEFAULT_INTERVAL_SECONDS: u64 =
        600;

    const CONTROL_WIDTH: f32 =
        190.0;

    egui::Grid::new(
        format!(
            "nested_config_grid_{:?}",
            target,
        )
    )
    .num_columns(
        2
    )
    .spacing(
        egui::vec2(
            8.0,
            6.0,
        )
    )
    .show(
        ui,
        |ui| {
            if let Some(
                wallpaper_display_format
            ) = wallpaper_display_format
            {
                ui.label(
                    crate::manage_localization::runtime_text("target.display_format")
                )
                .on_hover_text(
                    crate::manage_localization::runtime_text("target.display_format_help")
                );

                egui::ComboBox::from_id_source(
                    "nested_config_wallpaper_display_format_combo"
                )
                .selected_text(
                    match *wallpaper_display_format {
                        crate::manage_configuration::WallpaperDisplayFormat::FullScreen => { crate::manage_localization::runtime_text("target.full_screen") }

                        crate::manage_configuration::WallpaperDisplayFormat::Windowed => { crate::manage_localization::runtime_text("target.windowshader") }
                    }
                )
                .width(
                    CONTROL_WIDTH
                )
                .show_ui(
                    ui,
                    |ui| {
                        ui.selectable_value(
                            wallpaper_display_format,
                            crate::manage_configuration::WallpaperDisplayFormat::FullScreen,
                            crate::manage_localization::runtime_text("target.full_screen"),
                        );

                        ui.selectable_value(
                            wallpaper_display_format,
                            crate::manage_configuration::WallpaperDisplayFormat::Windowed,
                            crate::manage_localization::runtime_text("target.windowshader"),
                        );
                    },
                );

                ui.end_row();
            }


            ui.label(
                crate::manage_localization::runtime_text("target.mode")
            );


            let previous_display_mode =
                display_mode.clone();


            egui::ComboBox::from_id_source(
                format!(
                    "nested_config_display_{:?}",
                    target,
                )
            )
            .selected_text(
                match display_mode.as_str() {
                    "ordered" => crate::manage_localization::runtime_text("mode.ordered"),
                    "random" => crate::manage_localization::runtime_text("mode.random"),
                    "single" => crate::manage_localization::runtime_text("mode.single"),
                    "playlist" => crate::manage_localization::runtime_text("mode.playlist"),
                    _ => display_mode.clone(),
                }
            )
            .width(
                CONTROL_WIDTH
            )
            .show_ui(
                ui,
                |ui| {
                    for (choice, label_key) in [
                        ("ordered", "mode.ordered"),
                        ("random", "mode.random"),
                        ("single", "mode.single"),
                        ("playlist", "mode.playlist"),
                    ] {
                        ui.selectable_value(
                            display_mode,
                            choice.to_string(),
                            crate::manage_localization::runtime_text(label_key),
                        );
                    }
                },
            );


            if previous_display_mode
                == "single"
                && display_mode.as_str()
                    != "single"
            {
                *interval_seconds =
                    DEFAULT_INTERVAL_SECONDS;
            }


            if display_mode.as_str()
                != "single"
                && *interval_seconds == 0
            {
                *interval_seconds =
                    DEFAULT_INTERVAL_SECONDS;
            }


            ui.end_row();


            if display_mode.as_str()
                == "single"
            {
                ui.label(
                    crate::manage_localization::runtime_text("target.policy")
                );


                let displayed_policy =
                    if single_policy_id.is_none()
                        || single_policy_name
                            .trim()
                            .is_empty()
                    {
                        crate::manage_localization::runtime_text("target.select_policy")
                            .to_string()
                    } else {
                        single_policy_name
                            .clone()
                    };


                ui.menu_button(
                    displayed_policy,
                    |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(
                                320.0
                            )
                            .show(
                                ui,
                                |ui| {
                                    let mut eligible_count =
                                        0_usize;


                                    for row in policy_rows
                                        .iter()
                                        .filter(
                                            |row| {
                                                row.policy_target
                                                    == target
                                                    && row.accessible
                                            }
                                        )
                                    {
                                        eligible_count +=
                                            1;


                                        let response =
                                            ui.selectable_label(
                                                *single_policy_id
                                                    == Some(row.policy_id),
                                                row.policy_key
                                                    .as_str(),
                                            );


                                        if response.clicked() {
                                            *single_policy_id =
                                                Some(
                                                    row.policy_id
                                                );

                                            *single_policy_name =
                                                row.policy_key
                                                    .clone();

                                            *status_message =
                                                crate::manage_localization::runtime_text_with_params(
                                                    "target.single_policy_selected",
                                                    &[
                                                        ("target", &target_name(target)),
                                                        ("name", &row.policy_key),
                                                    ],
                                                );

                                            ui.close();
                                        }
                                    }


                                    if eligible_count == 0 {
                                        ui.add_enabled(
                                            false,
                                            egui::Button::new(
                                                crate::manage_localization::runtime_text("target.no_eligible_policies")
                                            ),
                                        );
                                    }
                                },
                            );
                    },
                );


                ui.end_row();
            } else if display_mode.as_str()
                == "playlist"
            {
                ui.label(
                    crate::manage_localization::runtime_text("target.playlist")
                );


                let displayed_playlist =
                    if playlist_id.is_none()
                        || playlist_name
                            .trim()
                            .is_empty()
                    {
                        crate::manage_localization::runtime_text("target.select_playlist")
                            .to_string()
                    } else {
                        playlist_name
                            .clone()
                    };


                ui.menu_button(
                    displayed_playlist,
                    |ui| {
                        match crate::manage_playlists::list_playlists() {
                            Ok(playlists) => {
                                if playlists.is_empty() {
                                    ui.add_enabled(
                                        false,
                                        egui::Button::new(
                                            crate::manage_localization::runtime_text("target.no_playlists")
                                        ),
                                    );
                                } else {
                                    egui::ScrollArea::vertical()
                                        .max_height(
                                            320.0
                                        )
                                        .show(
                                            ui,
                                            |ui| {
                                                for playlist in playlists {
                                                    let response =
                                                        ui.selectable_label(
                                                            *playlist_id
                                                                == Some(playlist.playlist_id),
                                                            playlist.playlist_name
                                                                .as_str(),
                                                        );

                                                    if response.clicked() {
                                                        *playlist_id =
                                                            Some(
                                                                playlist.playlist_id
                                                            );

                                                        *playlist_name =
                                                            playlist.playlist_name
                                                                .clone();

                                                        *status_message =
                                                            crate::manage_localization::runtime_text_with_params(
                                                                "target.playlist_selected",
                                                                &[
                                                                    ("target", &target_name(target)),
                                                                    ("name", &playlist.playlist_name),
                                                                ],
                                                            );

                                                        ui.close();
                                                    }
                                                }
                                            },
                                        );
                                }
                            }

                            Err(error) => {
                                ui.add_enabled(
                                    false,
                                    egui::Button::new(
                                        crate::manage_localization::runtime_text("target.playlists_unavailable")
                                    ),
                                )
                                .on_hover_text(
                                    error
                                );
                            }
                        }
                    },
                );


                ui.end_row();


                ui.label(
                    crate::manage_localization::runtime_text("target.interval")
                );


                ui.horizontal(
                    |ui| {
                        ui.add(
                            egui::DragValue::new(
                                interval_seconds
                            )
                            .clamp_range(
                                1..=86400
                            ),
                        );

                        ui.label(
                            crate::manage_localization::runtime_text("unit.seconds_lower")
                        );
                    },
                );


                ui.end_row();
            } else {
                ui.label(
                    crate::manage_localization::runtime_text("target.interval")
                );


                ui.horizontal(
                    |ui| {
                        ui.add(
                            egui::DragValue::new(
                                interval_seconds
                            )
                            .clamp_range(
                                1..=86400
                            ),
                        );

                        ui.label(
                            crate::manage_localization::runtime_text("unit.seconds_lower")
                        );
                    },
                );


                ui.end_row();
            }


            if let (
                Some(idle_timeout_value),
                Some(idle_timeout_unit),
            ) = (
                idle_timeout_value,
                idle_timeout_unit,
            ) {
                ui.label(
                    crate::manage_localization::runtime_text("target.idle_timeout")
                );

                ui.horizontal(
                    |ui| {
                        ui.add(
                            egui::DragValue::new(
                                idle_timeout_value
                            )
                            .clamp_range(
                                1..=86400
                            )
                        );

                        egui::ComboBox::from_id_source(
                            "nested_config_idle_timeout_unit"
                        )
                        .selected_text(
                            match idle_timeout_unit.as_str() {
                                "seconds" => crate::manage_localization::runtime_text("unit.seconds"),
                                "minutes" => crate::manage_localization::runtime_text("unit.minutes"),
                                "hours" => crate::manage_localization::runtime_text("unit.hours"),
                                _ => crate::manage_localization::runtime_text("unit.seconds"),
                            }
                        )
                        .width(
                            90.0
                        )
                        .show_ui(
                            ui,
                            |ui| {
                                for (
                                    value,
                                    label,
                                ) in [
                                    (
                                        "seconds",
                                        "unit.seconds",
                                    ),
                                    (
                                        "minutes",
                                        "unit.minutes",
                                    ),
                                    (
                                        "hours",
                                        "unit.hours",
                                    ),
                                ] {
                                    ui.selectable_value(
                                        idle_timeout_unit,
                                        value.to_string(),
                                        crate::manage_localization::runtime_text(label),
                                    );
                                }
                            },
                        );
                    },
                );

                ui.end_row();
            }


            ui.label(
                crate::manage_localization::runtime_text("target.animation_speed")
            );

            ui.add(
                egui::DragValue::new(animation_speed)
                    .speed(0.01)
                    .clamp_range(-10.0..=10.0)
                    .suffix("x")
            );

            ui.end_row();


            ui.label(
                crate::manage_localization::runtime_text("target.texture")
            );

            egui::ComboBox::from_id_source(
                format!(
                    "nested_config_texture_{:?}",
                    target,
                )
            )
            .selected_text(
                if global_texture == "random" {
                    crate::manage_localization::runtime_text("common.random")
                } else {
                    global_texture.clone()
                }
            )
            .width(
                CONTROL_WIDTH
            )
            .show_ui(
                ui,
                |ui| {
                    ui.selectable_value(
                        global_texture,
                        "random".to_string(),
                        crate::manage_localization::runtime_text("common.random"),
                    );

                    match texture_choices() {
                        Ok(choices) => {
                            for choice in choices {
                                ui.selectable_value(
                                    global_texture,
                                    choice.clone(),
                                    choice,
                                );
                            }
                        }

                        Err(error) => {
                            ui.add_enabled(
                                false,
                                egui::Button::new(
                                    crate::manage_localization::runtime_text("target.texture_catalog_unavailable")
                                ),
                            );

                            *status_message =
                                crate::manage_localization::runtime_text_with_params(
                                    "target.texture_choices_failed",
                                    &[("error", &error)],
                                );
                        }
                    }
                },
            );

            ui.end_row();


            ui.label(
                crate::manage_localization::runtime_text("target.palette")
            );

            draw_curated_palette_dropdown(
                ui,
                target,
                global_palette,
                CONTROL_WIDTH,
                status_message,
            );

            ui.end_row();


            ui.label(
                crate::manage_localization::runtime_text("target.texture_primitives")
            );

            ui.add(
                egui::DragValue::new(texture_primitives)
                    .clamp_range(1..=1024)
            );

            ui.end_row();
        },
    );


    if display_mode.as_str()
        == "single"
        && single_policy_id
            .is_none()
    {
        *status_message =
            crate::manage_localization::runtime_text_with_params(
                "target.single_policy_required",
                &[("target", &target_name(target))],
            );
    }
}


fn draw_lyrics(
    ui: &mut egui::Ui,
    configuration: &mut ControlConfiguration,
) {
    ui.heading(
        crate::manage_localization::runtime_text("lyrics.heading")
    );

    ui.add_space(
        8.0
    );

    ui.checkbox(
        &mut configuration.lyrics_enabled,
        crate::manage_localization::runtime_text("lyrics.display"),
    )
    .on_hover_text(
        crate::manage_localization::runtime_text("lyrics.display_help")
    );
}


fn draw_rendering_placeholders(
    ui: &mut egui::Ui,
    configuration: &mut ControlConfiguration,
) {
    const CONTROL_WIDTH: f32 = 190.0;

    ui.heading(crate::manage_localization::runtime_text("rendering.heading"));
    ui.add_space(8.0);

    egui::Grid::new("rendering_defaults")
        .num_columns(2)
        .spacing(egui::vec2(8.0, 6.0))
        .show(ui, |ui| {
            ui.label(crate::manage_localization::runtime_text("rendering.fps"));
            ui.add(egui::DragValue::new(&mut configuration.rendered_fps).clamp_range(16..=120));
            ui.end_row();

            ui.label(crate::manage_localization::runtime_text("rendering.anti_aliasing"));
            egui::ComboBox::from_id_source("rendering_default_aa")
                .selected_text(match configuration.anti_aliasing.as_str() {
                    "off" => crate::manage_localization::runtime_text("rendering.off"),
                    "fxaa" => crate::manage_localization::runtime_text("rendering.fxaa"),
                    _ => configuration.anti_aliasing.clone(),
                })
                .width(CONTROL_WIDTH)
                .show_ui(ui, |ui| {
                    for (choice, key) in [("off", "rendering.off"), ("fxaa", "rendering.fxaa")] {
                        ui.selectable_value(&mut configuration.anti_aliasing, choice.to_string(), crate::manage_localization::runtime_text(key));
                    }
                });
            ui.end_row();

            ui.label(crate::manage_localization::runtime_text("rendering.dithering"));
            egui::ComboBox::from_id_source("rendering_default_dithering")
                .selected_text(match configuration.dithering.as_str() {
                    "off" => crate::manage_localization::runtime_text("rendering.off"),
                    "subtle" => crate::manage_localization::runtime_text("rendering.subtle"),
                    _ => configuration.dithering.clone(),
                })
                .width(CONTROL_WIDTH)
                .show_ui(ui, |ui| {
                    for (choice, key) in [("off", "rendering.off"), ("subtle", "rendering.subtle")] {
                        ui.selectable_value(&mut configuration.dithering, choice.to_string(), crate::manage_localization::runtime_text(key));
                    }
                });
            ui.end_row();

            ui.label(crate::manage_localization::runtime_text("rendering.color_precision"));
            egui::ComboBox::from_id_source("rendering_default_precision")
                .selected_text(match configuration.color_precision.as_str() {
                    "auto" => crate::manage_localization::runtime_text("rendering.auto"),
                    "standard" => crate::manage_localization::runtime_text("rendering.standard"),
                    "high" => crate::manage_localization::runtime_text("rendering.high"),
                    _ => configuration.color_precision.clone(),
                })
                .width(CONTROL_WIDTH)
                .show_ui(ui, |ui| {
                    for (choice, key) in [
                        ("auto", "rendering.auto"),
                        ("standard", "rendering.standard"),
                        ("high", "rendering.high"),
                    ] {
                        ui.selectable_value(&mut configuration.color_precision, choice.to_string(), crate::manage_localization::runtime_text(key));
                    }
                });
            ui.end_row();

            ui.label(crate::manage_localization::runtime_text("rendering.render_scale"));
            ui.add(
                egui::DragValue::new(&mut configuration.render_scale)
                    .speed(0.05)
                    .clamp_range(0.25..=2.0)
                    .suffix("x")
            );
            ui.end_row();
        });
}


fn draw_disabled_grid(
    ui: &mut egui::Ui,
    grid_id: &str,
    rows: &[(&str, &str)],
) {
    const CONTROL_WIDTH: f32 =
        190.0;


    egui::Grid::new(
        format!(
            "nested_config_disabled_grid_{}",
            grid_id,
        )
    )
    .num_columns(
        2
    )
    .spacing(
        egui::vec2(
            8.0,
            6.0,
        )
    )
    .show(
        ui,
        |ui| {
            for (
                label,
                value,
            ) in rows
            {
                ui.label(
                    *label
                );

                ui.add_enabled(
                    false,
                    egui::Button::new(
                        *value
                    ),
                );

                ui.end_row();
            }
        },
    );
}


fn texture_choices(
) -> &'static Result<Vec<String>, String> {
    static CHOICES:
        OnceLock<
            Result<
                Vec<String>,
                String,
            >
        > =
        OnceLock::new();


    CHOICES.get_or_init(
        crate::manage_configuration::load_texture_choices
    )
}


fn curated_palette_choices(
) -> &'static Result<
    Vec<crate::manage_configuration::CuratedPaletteChoice>,
    String,
> {
    static CHOICES:
        OnceLock<
            Result<
                Vec<crate::manage_configuration::CuratedPaletteChoice>,
                String,
            >
        > =
        OnceLock::new();

    CHOICES.get_or_init(
        crate::manage_configuration::load_curated_palette_choices
    )
}


fn draw_curated_palette_dropdown(
    ui: &mut egui::Ui,
    target: PolicyTarget,
    global_palette: &mut String,
    control_width: f32,
    status_message: &mut String,
) {
    let choices = curated_palette_choices();

    let selected_text =
        if global_palette.eq_ignore_ascii_case("random") {
            crate::manage_localization::runtime_text("common.random")
        } else {
            choices
                .as_ref()
                .ok()
                .and_then(|choices| {
                    choices.iter().find(|entry| {
                        entry.color_hex.eq_ignore_ascii_case(global_palette)
                    })
                })
                .map(|entry| entry.description.clone())
                .unwrap_or_else(|| global_palette.clone())
        };

    egui::ComboBox::from_id_source(
        format!("nested_config_palette_{:?}", target)
    )
    .selected_text(selected_text)
    .width(control_width)
    .show_ui(ui, |ui| {
        let random_selected =
            global_palette.eq_ignore_ascii_case("random");

        if ui.selectable_label(random_selected, crate::manage_localization::runtime_text("common.random")).clicked() {
            *global_palette = "random".to_string();
            ui.close();
        }

        match choices {
            Ok(choices) => {
                ui.separator();

                egui::ScrollArea::vertical()
                    .max_height(235.0)
                    .show(ui, |ui| {
                        for entry in choices {
                            let selected =
                                entry.color_hex.eq_ignore_ascii_case(global_palette);

                            let response = curated_palette_choice_button(
                                ui,
                                entry,
                                selected,
                                control_width,
                            );

                            if response.clicked() {
                                *global_palette = entry.color_hex.clone();
                                *status_message = crate::manage_localization::runtime_text_with_params(
                                    "target.palette_selected",
                                    &[
                                        ("target", &target_name(target)),
                                        ("name", &entry.description),
                                    ],
                                );
                                ui.close();
                            }
                        }
                    });
            }

            Err(error) => {
                ui.add_enabled(
                    false,
                    egui::Button::new(crate::manage_localization::runtime_text("target.palette_unavailable")),
                );
                *status_message = crate::manage_localization::runtime_text_with_params(
                    "target.palette_choices_failed",
                    &[("error", error)],
                );
            }
        }
    });
}


fn curated_palette_choice_button(
    ui: &mut egui::Ui,
    entry: &crate::manage_configuration::CuratedPaletteChoice,
    selected: bool,
    width: f32,
) -> egui::Response {
    let row_height = ui.spacing().interact_size.y;
    let desired_size = egui::vec2(width.max(128.0), row_height);
    let (rect, response) =
        ui.allocate_exact_size(desired_size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let visuals =
            ui.style().interact_selectable(&response, selected);

        ui.painter().rect_filled(
            rect,
            visuals.rounding(),
            visuals.bg_fill,
        );

        ui.painter().rect_stroke(
            rect,
            visuals.rounding(),
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );

        let swatch_size = 14.0;
        let swatch_rect = egui::Rect::from_min_size(
            egui::pos2(
                rect.left() + 6.0,
                rect.center().y - swatch_size * 0.5,
            ),
            egui::vec2(swatch_size, swatch_size),
        );

        if let Ok(color) =
            crate::palettes::PaletteColor::parse_hex(&entry.color_hex)
        {
            ui.painter().rect_filled(
                swatch_rect,
                2.0,
                egui::Color32::from_rgb(
                    color.red(),
                    color.green(),
                    color.blue(),
                ),
            );

            ui.painter().rect_stroke(
                swatch_rect,
                2.0,
                egui::Stroke::new(
                    1.0,
                    ui.visuals().widgets.noninteractive.fg_stroke.color,
                ),
                egui::StrokeKind::Inside,
            );
        }

        let mut entry_font =
            egui::TextStyle::Button.resolve(ui.style());
        entry_font.size = (entry_font.size - 1.0).max(1.0);

        ui.painter().text(
            egui::pos2(
                swatch_rect.right() + 7.0,
                rect.center().y,
            ),
            egui::Align2::LEFT_CENTER,
            &entry.description,
            entry_font,
            visuals.text_color(),
        );
    }

    response
}


fn target_name(
    target: PolicyTarget,
) -> String {
    match target {
        PolicyTarget::Screensaver => crate::manage_localization::runtime_text("target.screensaver"),
        PolicyTarget::Wallpaper => crate::manage_localization::runtime_text("target.wallpaper"),
        PolicyTarget::Unassigned => "unassigned".to_string(),
    }
}
