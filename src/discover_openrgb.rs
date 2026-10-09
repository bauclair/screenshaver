//! Read-only OpenRGB SDK v6 controller discovery.
//! Only packet IDs 40 (negotiate), 0 (controller list), and 1 (controller data)
//! are transmitted. Never changes device modes, colors, or detection state.
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpStream};
use std::time::Duration;

const PROTOCOL: u32 = 6;
const MAX_PACKET: usize = 16 * 1024 * 1024;

pub fn run() -> Result<(), String> {
    let address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 6742);
    println!("[OPENRGB] Read-only discovery at {address}; no rescans or lighting writes");
    let mut socket = TcpStream::connect_timeout(&address.into(), Duration::from_secs(3))
        .map_err(|e| format!("Cannot connect to OpenRGB: {e}"))?;
    socket.set_read_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
    socket.set_write_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
    send(&mut socket, 0, 40, &PROTOCOL.to_le_bytes())?;
    let (_, version) = receive(&mut socket, 40)?;
    if version.len() != 4 { return Err("Invalid protocol negotiation response".into()); }
    let server_version = u32::from_le_bytes(version[..4].try_into().unwrap());
    println!("[OPENRGB] Server protocol: {server_version}; requested: {PROTOCOL}");
    if server_version < PROTOCOL { return Err("Read-only inventory requires OpenRGB SDK protocol v6".into()); }
    send(&mut socket, 0, 0, &[])?;
    let (_, response) = receive(&mut socket, 0)?;
    let mut r = Reader::new(&response);
    let count = r.u32()? as usize;
    if count > 256 || response.len() != 4 + count * 4 { return Err("Invalid controller ID list".into()); }
    let mut ids = Vec::with_capacity(count);
    for _ in 0..count { ids.push(r.u32()?); }
    println!("[OPENRGB] Controllers recognized: {count}");
    for (ordinal, id) in ids.into_iter().enumerate() {
        send(&mut socket, id, 1, &PROTOCOL.to_le_bytes())?;
        let (returned_id, payload) = receive(&mut socket, 1)?;
        if returned_id != id { return Err(format!("Controller ID mismatch: expected {id}, received {returned_id}")); }
        println!("\n[OPENRGB] Controller #{ordinal}; SDK ID {id}");
        describe_controller(&payload)?;
    }
    println!("\n[OPENRGB] Discovery complete; no lighting or device settings modified.");
    Ok(())
}

fn send(stream: &mut TcpStream, device: u32, command: u32, data: &[u8]) -> Result<(), String> {
    if !matches!(command, 0 | 1 | 40) { return Err("Refusing non-inventory command".into()); }
    let length = u32::try_from(data.len()).map_err(|_| "Request too large")?;
    let mut header = [0u8; 16];
    header[..4].copy_from_slice(b"ORGB");
    header[4..8].copy_from_slice(&device.to_le_bytes());
    header[8..12].copy_from_slice(&command.to_le_bytes());
    header[12..16].copy_from_slice(&length.to_le_bytes());
    stream.write_all(&header).and_then(|_| stream.write_all(data)).map_err(|e| e.to_string())
}

fn receive(stream: &mut TcpStream, expected: u32) -> Result<(u32, Vec<u8>), String> {
    for _ in 0..64 {
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
        if matches!(command, 10 | 51 | 53 | 100 | 1150) {
            println!("  [SDK notice] asynchronous packet {command}, {size} bytes");
            continue;
        }
        return Err(format!("Unexpected SDK command {command}, expected {expected}"));
    }
    Err("Too many asynchronous SDK notices".into())
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

fn describe_controller(payload: &[u8]) -> Result<(), String> {
    let mut r = Reader::new(payload);
    let reported_size = r.u32()? as usize;
    if reported_size != payload.len() { return Err("Controller record size mismatch".into()); }
    let kind = r.i32()?;
    let name = r.string()?;
    let vendor = r.string()?;
    let description = r.string()?;
    let version = r.string()?;
    let _serial = r.string()?;
    let location = r.string()?;
    println!("  Name: {name}\n  Vendor: {vendor}\n  Description: {description}\n  Version: {version}\n  Location: {location}\n  Type: {kind}");
    let mode_count = r.u16()? as usize;
    let active = r.i32()?;
    if mode_count > 512 { return Err("Implausible mode count".into()); }
    println!("  Modes: {mode_count}; active mode index: {active}");
    for index in 0..mode_count { println!("    {index}: {}{}", r.mode()?, if index as i32 == active { " [ACTIVE]" } else { "" }); }
    let zone_count = r.u16()? as usize;
    if zone_count > 512 { return Err("Implausible zone count".into()); }
    println!("  Zones: {zone_count}");
    for zone in 0..zone_count {
        let zone_name = r.string()?;
        let zone_kind = r.i32()?;
        let minimum = r.u32()?;
        let maximum = r.u32()?;
        let count = r.u32()?;
        let matrix = r.matrix()?;
        println!("    Zone {zone}: {zone_name}, type {zone_kind}, LEDs {count} (min {minimum}, max {maximum})");
        if let Some((width, height, mapped)) = matrix {
            println!("      Matrix: {width} columns x {height} rows; {mapped} mapped positions");
        } else { println!("      Matrix: not supplied"); }
        let segments = r.u16()? as usize;
        if segments > 2048 { return Err("Implausible segment count".into()); }
        for _ in 0..segments {
            r.string()?;
            r.i32()?;
            r.u32()?;
            r.u32()?;
            r.matrix()?;
            r.u32()?;
        }
        r.u32()?; // zone flags
        r.i32()?; // active zone mode
        let zone_modes = r.u16()? as usize;
        if zone_modes > 512 { return Err("Implausible zone mode count".into()); }
        for _ in 0..zone_modes { r.mode()?; }
        r.string()?; // zone display name
    }
    let led_count = r.u16()? as usize;
    if led_count > 65535 { return Err("Implausible LED count".into()); }
    let mut leds = Vec::with_capacity(led_count);
    for _ in 0..led_count { leds.push(r.string()?); }
    let color_count = r.u16()? as usize;
    r.take(color_count.checked_mul(4).ok_or("Color count overflow")?)?;
    let alternate_count = r.u16()? as usize;
    for _ in 0..alternate_count { r.string()?; }
    let flags = r.u32()?;
    let display = r.string()?;
    let _configuration = r.long_string()?;
    if r.at != payload.len() { return Err(format!("{} unparsed controller bytes", payload.len() - r.at)); }
    println!("  LED records: {led_count}; color records: {color_count}; flags: 0x{flags:08X}");
    println!("  Display name: {display}");
    for label in ["Escape", "Q", "A", "Space"] {
        if let Some(index) = leds.iter().position(|name| name == label) {
            println!("    Reference LED {label}: index {index}");
        }
    }
    Ok(())
}
