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
        Self { brightness_percent: 75, minimum_percent: 0 }
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
    fn defaults_and_black_without_floor() {
        let mut colors = [[0, 0, 0, 0], [255, 255, 255, 0], [128, 64, 32, 7]];
        AmbientBrightness::default().apply(&mut colors).unwrap();
        assert_eq!(colors[0], [0, 0, 0, 0]);
        assert_eq!(colors[1], [191, 191, 191, 0]);
        assert_eq!(colors[2], [96, 48, 24, 7]);
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

/// Exact SDK command 1050 payload verified by the physical Cynosa test.
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

fn run_worker(mut socket: TcpStream, id: u32, count: usize,
              rx: Receiver<Vec<LedColor>>) -> Result<usize, String> {
    let mut transmitted = 0;
    while let Ok(colors) = rx.recv() {
        if colors.len() != count { return Err("OpenRGB frame LED count changed".into()); }
        let payload = led_update_payload(&colors)?;
        send_packet(&mut socket, id, SDK_UPDATE_LEDS, &payload)?;
        transmitted += 1;
    }
    Ok(transmitted)
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
                let bytes = (self.width as usize).checked_mul(self.height as usize)
                    .and_then(|n| n.checked_mul(3))
                    .ok_or("Ambient sample byte count overflow")?;
                let mut rgb = vec![0u8; bytes];
                gl::Disable(gl::SCISSOR_TEST);
                gl::BindFramebuffer(gl::READ_FRAMEBUFFER, source.framebuffer);
                gl::ReadBuffer(if source.framebuffer == 0 { gl::BACK } else { gl::COLOR_ATTACHMENT0 });
                gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, self.framebuffer);
                // NEAREST also supports resolving a multisampled source to a
                // single-sample destination (subject to GL framebuffer rules).
                gl::BlitFramebuffer(0, 0, source.width as i32, source.height as i32,
                    0, 0, self.width as i32, self.height as i32,
                    gl::COLOR_BUFFER_BIT, gl::NEAREST);
                gl::BindFramebuffer(gl::READ_FRAMEBUFFER, self.framebuffer);
                gl::ReadBuffer(gl::COLOR_ATTACHMENT0);
                gl::BindBuffer(gl::PIXEL_PACK_BUFFER, 0);
                gl::PixelStorei(gl::PACK_ALIGNMENT, 1);
                gl::ReadPixels(0, 0, self.width as i32, self.height as i32,
                    gl::RGB, gl::UNSIGNED_BYTE, rgb.as_mut_ptr().cast());
                let error = gl::GetError();
                if error != gl::NO_ERROR {
                    return Err(format!("Ambient framebuffer readback GL error 0x{error:04x}"));
                }
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


// Guarded, SDL-only ambient keyboard session. This is explicitly opt-in.
// This module deliberately retains the validated physical-test SDK decoder.
mod ambient_cynosa_sdk {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::{Duration, Instant};
    const MAX_PACKET: usize = 16 * 1024 * 1024;
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
    
    pub(super) struct Session {
        control: TcpStream,
        id: u32,
        original_index: usize,
        original_record: Vec<u8>,
        original_name: String,
        pub(super) matrix: super::LedMatrix,
        pub(super) colors: Vec<super::LedColor>,
        pub(super) worker: Option<super::LightingWorker>,
        changed_mode: bool,
    }
    impl Session {
        pub(super) fn connect() -> Result<Self, String> {
            use std::net::{Ipv4Addr, SocketAddrV4};
            let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 6742);
            let mut control = TcpStream::connect_timeout(&addr.into(), Duration::from_secs(3))
                .map_err(|e| format!("OpenRGB connection: {e}"))?;
            control.set_read_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
            control.set_write_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
            send(&mut control, 0, 40, &6u32.to_le_bytes())?;
            let (_, version) = receive(&mut control, 40)?;
            if version.len()!=4 || u32::from_le_bytes(version[..4].try_into().unwrap())<6 {
                return Err("OpenRGB SDK v6 required".into());
            }
            send(&mut control, 0, 0, &[])?;
            let (_, response) = receive(&mut control, 0)?;
            let mut reader = Reader::new(&response);
            let count = reader.u32()? as usize;
            if count>256 || response.len()!=4+count*4 {return Err("Invalid OpenRGB controller list".into());}
            let mut selected = None;
            for _ in 0..count {
                let id=reader.u32()?;
                send(&mut control, id, 1, &6u32.to_le_bytes())?;
                let (returned, body)=receive(&mut control, 1)?;
                if returned!=id {return Err("OpenRGB controller ID mismatch".into());}
                let info=parse_target(&body)?;
                if info.name=="Razer Cynosa Chroma" {
                    if selected.is_some() {return Err("Multiple Cynosa devices: refusing automatic selection".into());}
                    selected=Some((id,info));
                }
            }
            let (id, info)=selected.ok_or("Razer Cynosa Chroma not found")?;
            if info.led_names.len()!=132 || info.colors.len()!=132 || info.zone_counts!=[132]
                || info.width!=22 || info.height!=6 || info.active>=info.modes.len() {
                return Err("Unexpected Cynosa topology; refusing hardware changes".into());
            }
            for (idx, name) in [(1,"Escape"),(46,"Q"),(68,"A")] {
                if info.led_names[idx]!=format!("Key: {name}") {
                    return Err(format!("Cynosa LED {idx} identity mismatch"));
                }
            }
            let matrix=super::LedMatrix {columns:info.width,rows:info.height,indices:info.matrix,led_count:132};
            matrix.validate()?;
            let (original_name, original_record)=info.modes[info.active].clone();
            // Only the baseline already physically verified in the previous test.
            if original_name!="Spectrum Cycle" {
                return Err("Select Spectrum Cycle in OpenRGB before ambient hardware test".into());
            }
            if !info.modes.iter().any(|(name,_)| name=="Direct") {
                return Err("Cynosa Direct mode unavailable".into());
            }
            Ok(Self {control,id,original_index:info.active,original_record,original_name,
                matrix,colors:info.colors,worker:None,changed_mode:false})
        }
        pub(super) fn activate(&mut self) -> Result<(),String> {
            // Mark before writing: restoration is attempted even if the write fails.
            self.changed_mode=true;
            send(&mut self.control,self.id,1100,&[])?;
            std::thread::sleep(Duration::from_millis(300));
            if self.active_mode()? != "Direct" {return Err("Direct mode not confirmed".into());}
            self.worker=Some(super::LightingWorker::start(
                "127.0.0.1:6742".parse().unwrap(),self.id,self.matrix.led_count)?);
            Ok(())
        }
        fn active_mode(&mut self)->Result<String,String> {
            send(&mut self.control,self.id,1,&6u32.to_le_bytes())?;
            let (returned,body)=receive(&mut self.control,1)?;
            if returned!=self.id {return Err("Cynosa controller ID changed".into());}
            let info=parse_target(&body)?;
            if info.name!="Razer Cynosa Chroma" {return Err("Cynosa identity changed".into());}
            info.modes.get(info.active).map(|mode|mode.0.clone()).ok_or("Invalid active mode".into())
        }
        pub(super) fn finish(&mut self)->Result<super::WorkerStats,String> {
            let worker_result=if let Some(worker)=self.worker.take(){worker.stop()}
                else {Ok(super::WorkerStats::default())};
            let restore_result=if self.changed_mode {
                self.changed_mode=false;
                let mut payload=Vec::with_capacity(8+self.original_record.len());
                payload.extend_from_slice(&(8u32+self.original_record.len() as u32).to_le_bytes());
                payload.extend_from_slice(&(self.original_index as u32).to_le_bytes());
                payload.extend_from_slice(&self.original_record);
                send(&mut self.control,self.id,1101,&payload).and_then(|_|{
                    std::thread::sleep(Duration::from_millis(500));
                    let current=self.active_mode()?;
                    if current==self.original_name {Ok(())}
                    else {Err(format!("Mode restoration not verified: {current}"))}
                })
            } else {Ok(())};
            match (worker_result,restore_result) {
                (Ok(stats),Ok(()))=>Ok(stats),
                (Err(a),Ok(()))=>Err(format!("Worker: {a}; original mode restored")),
                (Ok(_),Err(b))=>Err(format!("RESTORATION WARNING: {b}")),
                (Err(a),Err(b))=>Err(format!("Worker: {a}; RESTORATION WARNING: {b}")),
            }
        }
    }
    impl Drop for Session {
        fn drop(&mut self) {
            if self.changed_mode || self.worker.is_some() {
                if let Err(error)=self.finish(){
                    eprintln!("[AMBIENT_OPENRGB] {error}; manually restore Spectrum Cycle in OpenRGB");
                }
            }
        }
    }
}

pub struct AmbientCynosaSession {
    inner: ambient_cynosa_sdk::Session,
    previous: Option<Vec<LedColor>>,
    started: Instant,
    submitted: u64,
}
impl AmbientCynosaSession {
    pub fn start() -> Result<Self,String> {
        let mut inner=ambient_cynosa_sdk::Session::connect()?;
        inner.activate()?;
        Ok(Self {inner,previous:None,started:Instant::now(),submitted:0})
    }
    /// CPU mapping only; bounded queue means the render thread never waits on TCP updates.
    pub fn submit(&mut self,frame:&SampledFrame)->Result<bool,String> {
        // Automatically stop sending after 30 seconds, even if the screen stays active.
        if self.started.elapsed()>=Duration::from_secs(30) {return Ok(false);}
        let next=map_pixels(&frame.rgb,frame.width,frame.height,&self.inner.matrix,
            MappingMode::Spatial,&self.inner.colors)?;
        let colors=if let Some(ref mut prev)=self.previous {
            smooth_colors(prev,&next,0.35)?;
            prev.clone()
        } else {self.previous=Some(next.clone());next};
        let accepted=self.inner.worker.as_mut().ok_or("OpenRGB worker missing")?.try_submit(colors)?;
        if accepted {self.submitted+=1;}
        Ok(accepted)
    }
    pub fn expired(&self)->bool { self.started.elapsed() >= Duration::from_secs(30) }
    pub fn stop(mut self)->Result<(u64,WorkerStats),String> {
        let stats=self.inner.finish()?;
        Ok((self.submitted,stats))
    }
}
