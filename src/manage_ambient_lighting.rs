//! Reusable, opt-in OpenRGB ambient-lighting foundation.
//!
//! No device is discovered or modified by importing this module. The caller
//! owns consent, controller selection, mode switching and mode restoration.
//! Keep test_openrgb.rs unchanged until this API has passed hardware regression.
#![allow(dead_code)]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const MAX_PACKET: usize = 16 * 1024 * 1024;
const SDK_UPDATE_LEDS: u32 = 1050;
pub const DEFAULT_INTERVAL: Duration = Duration::from_millis(100);

/// OpenRGB uses BGRA-like four-byte RGBColor records in SDK LED updates.
pub type LedColor = [u8; 4];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MappingMode { Spatial, SoftAmbiance }

/// An OpenRGB matrix entry is an LED index, or u32::MAX for an empty cell.
#[derive(Clone, Debug)]
pub struct LedMatrix {
    pub columns: usize,
    pub rows: usize,
    pub indices: Vec<u32>,
    pub led_count: usize,
}

impl LedMatrix {
    pub fn validate(&self) -> Result<(), String> {
        if self.columns == 0 || self.rows == 0 || self.led_count == 0 {
            return Err("Empty OpenRGB LED topology".into());
        }
        if self.columns.checked_mul(self.rows) != Some(self.indices.len()) {
            return Err("OpenRGB matrix dimensions mismatch".into());
        }
        if self.indices.iter().any(|&i| i != u32::MAX && (i as usize) >= self.led_count) {
            return Err("OpenRGB matrix index out of range".into());
        }
        Ok(())
    }
}

/// Resolve a saved region preset against a verified, row-major OpenRGB matrix.
/// Empty cells are ignored; repeated LED indices are counted once.
/// This is pure topology analysis and never communicates with hardware.
pub fn resolve_led_region(matrix: &LedMatrix, region: &str) -> Result<Vec<usize>, String> {
    matrix.validate()?;
    let mut chosen = vec![false; matrix.led_count];
    let (w, h) = (matrix.columns, matrix.rows);
    for row in 0..h {
        for col in 0..w {
            let index = matrix.indices[row * w + col];
            if index == u32::MAX { continue; }
            // Integer midpoint comparisons avoid floating-point boundary ambiguity.
            let left = 2 * col < w;
            let right = 2 * col >= w;
            let top = 2 * row < h;
            let bottom = 2 * row >= h;
            let included = match region {
                "all" => true,
                "left" => left,
                "right" => right,
                "top" => top,
                "bottom" => bottom,
                "center" => 2 * (2 * col + 1) >= w && 2 * (2 * col + 1) < 3 * w &&
                            2 * (2 * row + 1) >= h && 2 * (2 * row + 1) < 3 * h,
                "perimeter" => row == 0 || col == 0 || row + 1 == h || col + 1 == w,
                "corners" => (row == 0 || row + 1 == h) && (col == 0 || col + 1 == w),
                "top_left" => top && left,
                "top_right" => top && right,
                "bottom_left" => bottom && left,
                "bottom_right" => bottom && right,
                "first_half" | "second_half" => false,
                _ => return Err(format!("Unknown LED region preset: {region}")),
            };
            if included { chosen[index as usize] = true; }
        }
    }
    if region == "first_half" || region == "second_half" {
        // Linear presets operate on controller LED sequence, not matrix geometry.
        let midpoint = (matrix.led_count + 1) / 2;
        for (i, selected) in chosen.iter_mut().enumerate() {
            *selected = if region == "first_half" { i < midpoint } else { i >= midpoint };
        }
    }
    let indices: Vec<usize> = chosen.iter().enumerate()
        .filter_map(|(i, selected)| selected.then_some(i)).collect();
    if indices.is_empty() { return Err(format!("Region '{region}' contains no mapped LEDs")); }
    Ok(indices)
}

#[cfg(test)]
mod led_region_tests {
    use super::*;
    #[test]
    fn holes_and_corners() {
        let matrix = LedMatrix { columns: 3, rows: 3,
            indices: vec![0, 1, 2, 3, u32::MAX, 4, 5, 6, 7], led_count: 8 };
        assert_eq!(resolve_led_region(&matrix, "all").unwrap().len(), 8);
        assert_eq!(resolve_led_region(&matrix, "corners").unwrap(), vec![0, 2, 5, 7]);
        assert_eq!(resolve_led_region(&matrix, "perimeter").unwrap().len(), 8);
        assert!(resolve_led_region(&matrix, "center").is_err());
    }
}

/// Sampled pixels are RGB, row zero at the *bottom* (OpenGL readback order).
/// The caller may provide an initial color for unmapped LEDs.
pub fn map_pixels(
    rgb: &[u8], width: usize, height: usize, matrix: &LedMatrix,
    mode: MappingMode, base: &[LedColor],
) -> Result<Vec<LedColor>, String> {
    matrix.validate()?;
    if width == 0 || height == 0 || width.checked_mul(height)
        .and_then(|n| n.checked_mul(3)) != Some(rgb.len()) {
        return Err("Invalid RGB framebuffer dimensions".into());
    }
    if base.len() != matrix.led_count { return Err("LED base-color count mismatch".into()); }
    let mut result = base.to_vec();
    let ambient = if mode == MappingMode::SoftAmbiance {
        Some(representative_color(rgb))
    } else { None };
    for row in 0..matrix.rows {
        for col in 0..matrix.columns {
            let index = matrix.indices[row * matrix.columns + col];
            if index == u32::MAX { continue; }
            let sample = if let Some(color) = ambient { color } else {
                let x = ((col * 2 + 1) * width / (2 * matrix.columns)).min(width - 1);
                let y = (((matrix.rows - row) * 2 - 1) * height / (2 * matrix.rows)).min(height - 1);
                let offset = (y * width + x) * 3;
                [rgb[offset], rgb[offset + 1], rgb[offset + 2]]
            };
            result[index as usize] = [sample[0], sample[1], sample[2], 0];
        }
    }
    Ok(result)
}

/// Bright, saturated shader features contribute more than dark background pixels.
pub fn representative_color(rgb: &[u8]) -> [u8; 3] {
    let (mut weighted, mut weight_sum) = ([0.0f32; 3], 0.0f32);
    let (mut fallback, mut count) = ([0.0f32; 3], 0.0f32);
    for pixel in rgb.chunks_exact(3) {
        let c = [pixel[0] as f32 / 255.0, pixel[1] as f32 / 255.0, pixel[2] as f32 / 255.0];
        for i in 0..3 { fallback[i] += c[i]; }
        count += 1.0;
        let luminance = 0.2126*c[0] + 0.7152*c[1] + 0.0722*c[2];
        if luminance < 0.055 { continue; }
        let max = c[0].max(c[1]).max(c[2]);
        let min = c[0].min(c[1]).min(c[2]);
        let saturation = if max > 0.0001 { (max-min)/max } else { 0.0 };
        let weight = luminance.sqrt() * (0.35 + 0.65*saturation);
        for i in 0..3 { weighted[i] += c[i]*weight; }
        weight_sum += weight;
    }
    let mut output = [0; 3];
    for i in 0..3 {
        let v = if weight_sum > 0.0001 { weighted[i]/weight_sum }
                else if count > 0.0 { fallback[i]/count } else { 0.0 };
        output[i] = (v.clamp(0.0, 1.0)*255.0 + 0.5) as u8;
    }
    output
}

/// Output transfer settings, expressed as percentages of the device's 8-bit RGB range.
/// This stage is pure color processing: it never accesses OpenGL or OpenRGB.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AmbientBrightness {
    /// Maximum output for a full-brightness shader channel (10..=100).
    pub brightness_percent: u8,
    /// Neutral illumination for a black shader channel (0..=brightness_percent).
    pub minimum_percent: u8,
}

impl Default for AmbientBrightness {
    fn default() -> Self {
        Self { brightness_percent: 100, minimum_percent: 2 }
    }
}

impl AmbientBrightness {
    pub fn validate(self) -> Result<Self, String> {
        if !(10..=100).contains(&self.brightness_percent) {
            return Err("Ambient brightness must be between 10% and 100%".into());
        }
        if self.minimum_percent > self.brightness_percent {
            return Err("Minimum key illumination must not exceed ambient brightness".into());
        }
        Ok(self)
    }

    /// Apply the neutral minimum and output ceiling to spatially mapped LEDs.
    /// Channel mapping is affine, so spatial variation is preserved, no channel
    /// exceeds the ceiling, and black remains black with a zero minimum.
    /// The SDK's fourth color byte is preserved unchanged.
    pub fn apply(self, colors: &mut [LedColor]) -> Result<(), String> {
        let settings = self.validate()?;
        let ceiling = u32::from(settings.brightness_percent) * 255;
        let floor = u32::from(settings.minimum_percent) * 255;
        for color in colors {
            for channel in &mut color[..3] {
                // Combine percentage and input scaling before rounding, to
                // avoid double-rounding at low illumination levels.
                let numerator = floor * 255 + (ceiling - floor) * u32::from(*channel);
                *channel = ((numerator + 12_750) / 25_500).min(255) as u8;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod ambient_brightness_tests {
    use super::*;

    #[test]
    fn defaults_with_neutral_two_percent_floor() {
        let mut colors = [[0, 0, 0, 0], [255, 255, 255, 0], [128, 64, 32, 7]];
        AmbientBrightness::default().apply(&mut colors).unwrap();
        assert_eq!(colors[0], [5, 5, 5, 0]); // #050505: neutral dark gray
        assert_eq!(colors[1], [255, 255, 255, 0]);
        assert_eq!(colors[2], [131, 68, 36, 7]);
    }

    #[test]
    fn neutral_floor_and_ceiling() {
        let mut colors = [[0, 0, 0, 0], [255, 255, 255, 0], [255, 0, 128, 0]];
        AmbientBrightness { brightness_percent: 75, minimum_percent: 10 }
            .apply(&mut colors).unwrap();
        assert_eq!(colors[0], [26, 26, 26, 0]);
        assert_eq!(colors[1], [191, 191, 191, 0]);
        assert_eq!(colors[2], [191, 26, 109, 0]);
    }

    #[test]
    fn rejects_invalid_limits_without_modifying_colors() {
        let original = [[123, 45, 67, 9]];
        for settings in [
            AmbientBrightness { brightness_percent: 9, minimum_percent: 0 },
            AmbientBrightness { brightness_percent: 101, minimum_percent: 0 },
            AmbientBrightness { brightness_percent: 50, minimum_percent: 51 },
        ] {
            let mut colors = original;
            assert!(settings.apply(&mut colors).is_err());
            assert_eq!(colors, original);
        }
    }
}

/// Optional per-LED saturation enhancement. Preserve the HSV value (maximum channel)
/// while expanding channel differences; neutral greys remain unchanged.
/// This never modifies the rendered shader or framebuffer samples.
pub fn boost_led_saturation(colors: &mut [LedColor], factor: f32) -> Result<(), String> {
    if !factor.is_finite() || !(1.0..=2.0).contains(&factor) {
        return Err("Ambient saturation factor must be between 1.0 and 2.0".into());
    }
    for color in colors {
        let max = *color[..3].iter().max().unwrap() as f32;
        for channel in &mut color[..3] {
            *channel = (max - (max - *channel as f32) * factor)
                .clamp(0.0, 255.0).round() as u8;
        }
    }
    Ok(())
}

#[cfg(test)]
mod ambient_saturation_tests {
    use super::*;
    #[test]
    fn increases_saturation_preserves_value_and_alpha() {
        let mut colors = [[180, 120, 90, 7], [100, 100, 100, 3], [255, 0, 0, 0]];
        boost_led_saturation(&mut colors, 1.5).unwrap();
        assert_eq!(colors[0], [180, 90, 45, 7]);
        assert_eq!(colors[1], [100, 100, 100, 3]);
        assert_eq!(colors[2], [255, 0, 0, 0]);
    }
    #[test]
    fn rejects_invalid_factor() {
        let mut colors = [[10, 20, 30, 0]];
        assert!(boost_led_saturation(&mut colors, f32::NAN).is_err());
        assert_eq!(colors[0], [10, 20, 30, 0]);
    }
}

/// Smooths successive per-LED colors; first frame initializes immediately.
pub fn smooth_colors(previous: &mut [LedColor], next: &[LedColor], alpha: f32) -> Result<(), String> {
    if previous.len() != next.len() { return Err("LED smoothing length mismatch".into()); }
    let a = alpha.clamp(0.0, 1.0);
    for (old, new) in previous.iter_mut().zip(next) {
        for channel in 0..3 {
            old[channel] = (old[channel] as f32 + a*(new[channel] as f32-old[channel] as f32))
                .round().clamp(0.0,255.0) as u8;
        }
    }
    Ok(())
}

/// OpenRGB SDK command 1050 LED update payload.
pub fn led_update_payload(colors: &[LedColor]) -> Result<Vec<u8>, String> {
    let count = u16::try_from(colors.len()).map_err(|_| "Too many OpenRGB LEDs")?;
    let size = 6usize.checked_add(colors.len().checked_mul(4).ok_or("LED payload overflow")?)
        .ok_or("LED payload overflow")?;
    let mut payload = Vec::with_capacity(size);
    payload.extend_from_slice(&(size as u32).to_le_bytes());
    payload.extend_from_slice(&count.to_le_bytes());
    for color in colors { payload.extend_from_slice(color); }
    Ok(payload)
}

fn send_packet(stream: &mut TcpStream, id: u32, command: u32, payload: &[u8]) -> Result<(), String> {
    if payload.len() > MAX_PACKET { return Err("OpenRGB packet exceeds size limit".into()); }
    let mut header = [0u8; 16];
    header[0..4].copy_from_slice(b"ORGB");
    header[4..8].copy_from_slice(&id.to_le_bytes());
    header[8..12].copy_from_slice(&command.to_le_bytes());
    header[12..16].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    stream.write_all(&header).and_then(|_| stream.write_all(payload))
        .map_err(|e| format!("OpenRGB SDK write failed: {e}"))
}

/// For future discovery/verification: bounded read, skipping SDK notifications.
pub fn receive_reply(stream: &mut TcpStream, expected: u32, deadline: Instant)
    -> Result<(u32, Vec<u8>), String> {
    loop {
        if Instant::now() >= deadline { return Err("OpenRGB SDK response deadline exceeded".into()); }
        let remaining = deadline.saturating_duration_since(Instant::now());
        stream.set_read_timeout(Some(remaining.min(Duration::from_millis(250))))
            .map_err(|e| e.to_string())?;
        let mut header = [0u8; 16];
        if let Err(e) = stream.read_exact(&mut header) {
            // A partial header cannot safely be retried as a new packet.
            return Err(format!("OpenRGB SDK response header: {e}"));
        }
        if &header[0..4] != b"ORGB" { return Err("Invalid OpenRGB packet magic".into()); }
        let id = u32::from_le_bytes(header[4..8].try_into().unwrap());
        let command = u32::from_le_bytes(header[8..12].try_into().unwrap());
        let size = u32::from_le_bytes(header[12..16].try_into().unwrap()) as usize;
        if size > MAX_PACKET { return Err("OpenRGB SDK packet too large".into()); }
        let mut body = vec![0; size];
        stream.read_exact(&mut body).map_err(|e| format!("OpenRGB SDK response body: {e}"))?;
        if command == expected { return Ok((id, body)); }
        if !matches!(command, 10 | 51 | 53 | 100 | 1150 | 1200) {
            return Err(format!("Unexpected OpenRGB SDK command {command}"));
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WorkerStats { pub transmitted: usize, pub dropped: usize }

/// Caller explicitly chooses the OpenRGB controller ID and must first establish
/// Direct mode. This worker never changes modes or selects devices.
pub struct LightingWorker {
    sender: Option<SyncSender<Vec<LedColor>>>,
    handle: Option<JoinHandle<Result<usize, String>>>,
    dropped: usize,
}

impl LightingWorker {
    pub fn start(address: SocketAddr, controller_id: u32, led_count: usize) -> Result<Self, String> {
        if led_count == 0 || led_count > u16::MAX as usize {
            return Err("Invalid OpenRGB LED count".into());
        }
        // Establish connection before starting worker; no implicit discovery or mode writes.
        let socket = TcpStream::connect_timeout(&address, Duration::from_secs(3))
            .map_err(|e| format!("OpenRGB worker connection failed: {e}"))?;
        socket.set_write_timeout(Some(Duration::from_secs(2))).map_err(|e| e.to_string())?;
        let (tx, rx) = mpsc::sync_channel(1);
        let handle = thread::spawn(move || run_worker(socket, controller_id, led_count, rx));
        Ok(Self { sender: Some(tx), handle: Some(handle), dropped: 0 })
    }

    /// Never blocks the renderer. Returns false if an obsolete frame was dropped.
    pub fn try_submit(&mut self, colors: Vec<LedColor>) -> Result<bool, String> {
        let tx = self.sender.as_ref().ok_or("OpenRGB worker stopped")?;
        match tx.try_send(colors) {
            Ok(()) => Ok(true),
            Err(TrySendError::Full(_)) => { self.dropped += 1; Ok(false) }
            Err(TrySendError::Disconnected(_)) => Err("OpenRGB worker disconnected".into()),
        }
    }

    /// Join worker before caller attempts mode restoration on its control socket.
    pub fn stop(mut self) -> Result<WorkerStats, String> {
        self.sender.take();
        let transmitted = self.handle.take().ok_or("OpenRGB worker already stopped")?
            .join().map_err(|_| "OpenRGB worker panicked".to_string())??;
        Ok(WorkerStats { transmitted, dropped: self.dropped })
    }
}

// Diagnostic-only counters. No changes to LED command frequency or wire format.
fn log_worker_diagnostic(message: &str) {
    crate::logger::warning(&crate::locate_paths::runtime_log_path(), message);
}

fn run_worker(mut socket: TcpStream, id: u32, count: usize,
              rx: Receiver<Vec<LedColor>>) -> Result<usize, String> {
    let started = Instant::now();
    let mut interval_started = started;
    let mut transmitted = 0usize;
    let mut interval_transmitted = 0usize;
    let mut interval_write_ns = 0u128;
    let mut interval_max_write_ns = 0u128;
    let mut total_write_ns = 0u128;
    let mut max_write_ns = 0u128;
    let mut write_errors = 0usize;
    let mut result: Result<(), String> = Ok(());

    while let Ok(colors) = rx.recv() {
        if colors.len() != count {
            result = Err("OpenRGB frame LED count changed".into());
            break;
        }
        let payload = match led_update_payload(&colors) {
            Ok(payload) => payload,
            Err(error) => { result = Err(error); break; }
        };
        let write_started = Instant::now();
        let write_result = send_packet(&mut socket, id, SDK_UPDATE_LEDS, &payload);
        let write_ns = write_started.elapsed().as_nanos();
        total_write_ns += write_ns;
        max_write_ns = max_write_ns.max(write_ns);
        interval_write_ns += write_ns;
        interval_max_write_ns = interval_max_write_ns.max(write_ns);
        if let Err(error) = write_result {
            write_errors += 1;
            log_worker_diagnostic(&format!(
                "[AMBIENT_OPENRGB_WORKER_DIAG] event=write_failed controller={} elapsed_s={:.1} transmitted={} write_ms={:.3} error={}",
                id, started.elapsed().as_secs_f64(), transmitted, write_ns as f64 / 1_000_000.0, error
            ));
            result = Err(error);
            break;
        }
        transmitted += 1;
        interval_transmitted += 1;
        if interval_started.elapsed() >= Duration::from_secs(60) {
            let seconds = interval_started.elapsed().as_secs_f64();
            log_worker_diagnostic(&format!(
                "[AMBIENT_OPENRGB_WORKER_DIAG] event=interval controller={} elapsed_s={:.1} transmitted={} interval_transmitted={} interval_fps={:.2} interval_avg_write_ms={:.3} interval_max_write_ms={:.3}",
                id, started.elapsed().as_secs_f64(), transmitted, interval_transmitted,
                interval_transmitted as f64 / seconds,
                interval_write_ns as f64 / interval_transmitted as f64 / 1_000_000.0,
                interval_max_write_ns as f64 / 1_000_000.0
            ));
            interval_started = Instant::now();
            interval_transmitted = 0;
            interval_write_ns = 0;
            interval_max_write_ns = 0;
        }
    }
    let elapsed_s = started.elapsed().as_secs_f64();
    log_worker_diagnostic(&format!(
        "[AMBIENT_OPENRGB_WORKER_DIAG] event=final controller={} elapsed_s={:.1} transmitted={} avg_fps={:.2} avg_write_ms={:.3} max_write_ms={:.3} write_errors={} result={}",
        id, elapsed_s, transmitted, transmitted as f64 / elapsed_s.max(0.000001),
        total_write_ns as f64 / (transmitted + write_errors).max(1) as f64 / 1_000_000.0,
        max_write_ns as f64 / 1_000_000.0, write_errors,
        if result.is_ok() { "ok" } else { "error" }
    ));
    result.map(|()| transmitted)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn payload_is_sdk_1050_format() {
        let data = led_update_payload(&[[1,2,3,0], [4,5,6,0]]).unwrap();
        assert_eq!(data.len(), 14);
        assert_eq!(&data[0..4], &14u32.to_le_bytes());
        assert_eq!(&data[4..6], &2u16.to_le_bytes());
        assert_eq!(&data[6..], &[1,2,3,0,4,5,6,0]);
    }
    #[test]
    fn matrix_skips_unmapped_and_flips_y() {
        let matrix = LedMatrix { columns: 2, rows: 2, indices: vec![0,u32::MAX,1,2], led_count: 3 };
        let rgb = [10,0,0, 20,0,0, 30,0,0, 40,0,0]; // bottom row, then top row
        let mapped = map_pixels(&rgb, 2, 2, &matrix, MappingMode::Spatial, &[[0;4];3]).unwrap();
        assert_eq!(mapped, vec![[30,0,0,0], [10,0,0,0], [20,0,0,0]]);
    }
    #[test]
    fn rejects_bad_matrix() {
        let matrix = LedMatrix { columns: 2, rows: 2, indices: vec![0,1], led_count: 2 };
        assert!(matrix.validate().is_err());
    }
}

/// A framebuffer observation made after postprocessing and before overlays.
/// The framebuffer ID is owned by the presentation host. Observing does not
/// imply permission to read or modify it; OpenGL work must stay on its thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameSource {
    pub framebuffer: u32,
    pub width: u32,
    pub height: u32,
}

/// Gate optional framebuffer sampling independently of shader FPS.
/// This is pure scheduling logic: it does not touch OpenGL or OpenRGB.
#[derive(Debug)]
pub struct SampleClock {
    interval: Duration,
    last: Option<Instant>,
}

impl SampleClock {
    pub fn new(interval: Duration) -> Result<Self, String> {
        if interval.is_zero() {
            return Err("Ambient sample interval must be nonzero".into());
        }
        Ok(Self { interval, last: None })
    }

    pub fn due(&mut self, now: Instant) -> bool {
        if self.last.is_some_and(|last| now.saturating_duration_since(last) < self.interval) {
            return false;
        }
        self.last = Some(now);
        true
    }

    pub fn reset(&mut self) { self.last = None; }
}

#[cfg(test)]
mod frame_sampling_tests {
    use super::*;
    #[test]
    fn sample_clock_limits_rate() {
        let mut clock = SampleClock::new(Duration::from_millis(100)).unwrap();
        let start = Instant::now();
        assert!(clock.due(start));
        assert!(!clock.due(start + Duration::from_millis(99)));
        assert!(clock.due(start + Duration::from_millis(100)));
    }
}


// --- Opt-in GPU framebuffer sampling (no OpenRGB device access) ---

/// A small RGB image in OpenGL bottom-to-top row order, suitable for `map_pixels`.
#[derive(Clone, Debug)]
pub struct SampledFrame {
    pub width: usize,
    pub height: usize,
    pub rgb: Vec<u8>,
    pub readback_time: Duration,
}

/// A render-thread-owned downsampling target. All methods, including `destroy`,
/// must run with the same OpenGL context current. The owner MUST call `destroy`
/// before destroying that context; Drop deliberately performs no GL calls.
/// This object never connects to OpenRGB and does not install itself in a renderer.
pub struct FramebufferSampler {
    clock: SampleClock,
    width: u32,
    height: u32,
    framebuffer: u32,
    texture: u32,
}

impl FramebufferSampler {
    /// `width`/`height` describe the *small* sample grid, not the display.
    /// A 22x12 grid at 100 ms is a reasonable initial diagnostic setting.
    pub fn new(width: u32, height: u32, interval: Duration) -> Result<Self, String> {
        if width == 0 || height == 0 || width > 256 || height > 256 {
            return Err("Ambient sample grid must be between 1x1 and 256x256".into());
        }
        Ok(Self { clock: SampleClock::new(interval)?, width, height,
                  framebuffer: 0, texture: 0 })
    }

    /// Returns None when the rate gate is closed. The output is sampled after
    /// postprocessing by calling this from the existing ambient frame hook.
    /// No device control, disk writes, or network activity occurs here.
    pub fn sample(&mut self, source: FrameSource) -> Result<Option<SampledFrame>, String> {
        if source.width == 0 || source.height == 0 {
            return Err("Ambient source framebuffer has zero dimensions".into());
        }
        if !self.clock.due(Instant::now()) { return Ok(None); }
        if !gl::BlitFramebuffer::is_loaded() || !gl::ReadPixels::is_loaded() {
            return Err("OpenGL framebuffer blit/readback functions unavailable".into());
        }
        let start = Instant::now();
        unsafe {
            // Diagnostic build: distinguish an inherited GL error from one
            // generated by the ambient sampling operations.
            let inherited = gl::GetError();
            if inherited != gl::NO_ERROR {
                eprintln!("[AMBIENT_GL_DIAG] preexisting GL error 0x{inherited:04x} before sampler; source_fbo={} size={}x{}", source.framebuffer, source.width, source.height);
            }

            let mut previous_read_fbo = 0;
            let mut previous_draw_fbo = 0;
            let mut previous_read_buffer = 0;
            let mut previous_pack_alignment = 0;
            let mut previous_pack_buffer = 0;
            let mut previous_scissor = gl::FALSE;
            gl::GetIntegerv(gl::READ_FRAMEBUFFER_BINDING, &mut previous_read_fbo);
            gl::GetIntegerv(gl::DRAW_FRAMEBUFFER_BINDING, &mut previous_draw_fbo);
            gl::GetIntegerv(gl::READ_BUFFER, &mut previous_read_buffer);
            gl::GetIntegerv(gl::PACK_ALIGNMENT, &mut previous_pack_alignment);
            gl::GetIntegerv(gl::PIXEL_PACK_BUFFER_BINDING, &mut previous_pack_buffer);
            previous_scissor = gl::IsEnabled(gl::SCISSOR_TEST);

            let result = (|| -> Result<Vec<u8>, String> {
                self.ensure_target()?;
                Self::check_gl_stage("ensure_target", source, self.framebuffer)?;
                let bytes = (self.width as usize).checked_mul(self.height as usize)
                    .and_then(|n| n.checked_mul(3))
                    .ok_or("Ambient sample byte count overflow")?;
                let mut rgb = vec![0u8; bytes];
                gl::Disable(gl::SCISSOR_TEST);
                Self::check_gl_stage("disable_scissor", source, self.framebuffer)?;
                gl::BindFramebuffer(gl::READ_FRAMEBUFFER, source.framebuffer);
                gl::ReadBuffer(if source.framebuffer == 0 { gl::BACK } else { gl::COLOR_ATTACHMENT0 });
                Self::check_gl_stage("bind_source_and_read_buffer", source, self.framebuffer)?;
                gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, self.framebuffer);
                Self::check_gl_stage("bind_destination", source, self.framebuffer)?;
                // NEAREST also supports resolving a multisampled source to a
                // single-sample destination (subject to GL framebuffer rules).
                gl::BlitFramebuffer(0, 0, source.width as i32, source.height as i32,
                    0, 0, self.width as i32, self.height as i32,
                    gl::COLOR_BUFFER_BIT, gl::NEAREST);
                Self::check_gl_stage("blit_framebuffer", source, self.framebuffer)?;
                gl::BindFramebuffer(gl::READ_FRAMEBUFFER, self.framebuffer);
                gl::ReadBuffer(gl::COLOR_ATTACHMENT0);
                Self::check_gl_stage("bind_sample_read_buffer", source, self.framebuffer)?;
                gl::BindBuffer(gl::PIXEL_PACK_BUFFER, 0);
                gl::PixelStorei(gl::PACK_ALIGNMENT, 1);
                Self::check_gl_stage("configure_pixel_pack", source, self.framebuffer)?;
                gl::ReadPixels(0, 0, self.width as i32, self.height as i32,
                    gl::RGB, gl::UNSIGNED_BYTE, rgb.as_mut_ptr().cast());
                Self::check_gl_stage("read_pixels", source, self.framebuffer)?;
                Ok(rgb)
            })();

            // Restore all modified host GL state, including separate FBO
            // bindings (important for Qt/embedded presentation backends).
            gl::BindFramebuffer(gl::READ_FRAMEBUFFER, previous_read_fbo as u32);
            gl::ReadBuffer(previous_read_buffer as u32);
            gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, previous_draw_fbo as u32);
            gl::BindBuffer(gl::PIXEL_PACK_BUFFER, previous_pack_buffer as u32);
            gl::PixelStorei(gl::PACK_ALIGNMENT, previous_pack_alignment);
            if previous_scissor == gl::TRUE { gl::Enable(gl::SCISSOR_TEST); }
            else { gl::Disable(gl::SCISSOR_TEST); }
            result.map(|rgb| Some(SampledFrame {
                width: self.width as usize, height: self.height as usize,
                rgb, readback_time: start.elapsed(),
            }))
        }
    }

    /// On failure, collect state only after the error has been captured.
    /// No additional framebuffer binding or allocation is performed here.
    unsafe fn check_gl_stage(stage: &str, source: FrameSource, target: u32) -> Result<(), String> {
        let error = gl::GetError();
        if error == gl::NO_ERROR { return Ok(()); }
        let mut read_fbo = 0;
        let mut draw_fbo = 0;
        let mut read_buffer = 0;
        let mut samples = 0;
        gl::GetIntegerv(gl::READ_FRAMEBUFFER_BINDING, &mut read_fbo);
        gl::GetIntegerv(gl::DRAW_FRAMEBUFFER_BINDING, &mut draw_fbo);
        gl::GetIntegerv(gl::READ_BUFFER, &mut read_buffer);
        gl::GetIntegerv(gl::SAMPLES, &mut samples);
        let read_status = gl::CheckFramebufferStatus(gl::READ_FRAMEBUFFER);
        let draw_status = gl::CheckFramebufferStatus(gl::DRAW_FRAMEBUFFER);
        Err(format!(
            "Ambient GL stage={stage} error=0x{error:04x} source_fbo={} source_size={}x{} target_fbo={target} read_fbo={read_fbo} draw_fbo={draw_fbo} read_buffer=0x{read_buffer:04x} samples={samples} read_status=0x{read_status:04x} draw_status=0x{draw_status:04x}",
            source.framebuffer, source.width, source.height
        ))
    }

    unsafe fn ensure_target(&mut self) -> Result<(), String> {
        if self.framebuffer != 0 { return Ok(()); }
        let mut old_texture = 0;
        let mut old_draw_fbo = 0;
        gl::GetIntegerv(gl::TEXTURE_BINDING_2D, &mut old_texture);
        gl::GetIntegerv(gl::DRAW_FRAMEBUFFER_BINDING, &mut old_draw_fbo);
        gl::GenTextures(1, &mut self.texture);
        gl::BindTexture(gl::TEXTURE_2D, self.texture);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGB8 as i32,
            self.width as i32, self.height as i32, 0,
            gl::RGB, gl::UNSIGNED_BYTE, std::ptr::null());
        gl::GenFramebuffers(1, &mut self.framebuffer);
        gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, self.framebuffer);
        gl::FramebufferTexture2D(gl::DRAW_FRAMEBUFFER, gl::COLOR_ATTACHMENT0,
            gl::TEXTURE_2D, self.texture, 0);
        let status = gl::CheckFramebufferStatus(gl::DRAW_FRAMEBUFFER);
        gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, old_draw_fbo as u32);
        gl::BindTexture(gl::TEXTURE_2D, old_texture as u32);
        if status != gl::FRAMEBUFFER_COMPLETE {
            self.destroy();
            return Err(format!("Ambient sample framebuffer incomplete: 0x{status:04x}"));
        }
        Ok(())
    }

    /// Release GPU resources while their owning GL context is current.
    pub unsafe fn destroy(&mut self) {
        if self.framebuffer != 0 {
            gl::DeleteFramebuffers(1, &self.framebuffer);
            self.framebuffer = 0;
        }
        if self.texture != 0 {
            gl::DeleteTextures(1, &self.texture);
            self.texture = 0;
        }
        self.clock.reset();
    }
}
