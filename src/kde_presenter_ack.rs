//! Verification of the KDE Plasma presenter acknowledgement protocol.
//! This module does not grant ownership or change Plasma configuration.
use std::{fs, os::unix::fs::MetadataExt, path::Path, time::{Duration, SystemTime, UNIX_EPOCH}};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenterAck {
    pub pid: u32,
    pub device: u64,
    pub inode: u64,
    pub sequence: u64,
    pub epoch_millis: u64,
}

impl PresenterAck {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > 256 { return Err("Presenter ACK exceeds size limit".into()); }
        let data = std::str::from_utf8(bytes).map_err(|_| "Presenter ACK is not UTF-8")?;
        let mut lines = data.lines();
        if lines.next() != Some("SSPLACK01") { return Err("Presenter ACK magic mismatch".into()); }
        let mut number = |name: &str| -> Result<u64, String> {
            lines.next().ok_or_else(|| format!("Presenter ACK missing {name}"))?
                .parse::<u64>().map_err(|_| format!("Presenter ACK invalid {name}"))
        };
        let pid = u32::try_from(number("PID")?).map_err(|_| "Presenter PID overflow")?;
        let device = number("device")?;
        let inode = number("inode")?;
        let sequence = number("sequence")?;
        let epoch_millis = number("timestamp")?;
        if lines.next().is_some() || pid == 0 || inode == 0 || sequence == 0 {
            return Err("Presenter ACK has invalid or extra fields".into());
        }
        Ok(Self { pid, device, inode, sequence, epoch_millis })
    }
}

/// Check an ACK against the actual producer file and a verified Plasma PID.
/// `last_sequence` must be supplied from a previous *accepted* ACK. This
/// function deliberately does not infer compositor presentation from time alone.
pub fn verify(
    ack_path: &Path,
    frame_path: &Path,
    expected_plasma_pid: u32,
    last_sequence: Option<u64>,
    max_age: Duration,
) -> Result<PresenterAck, String> {
    let metadata = fs::symlink_metadata(ack_path).map_err(|e| format!("ACK metadata: {e}"))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err("Presenter ACK must be a regular file".into());
    }
    let ack = PresenterAck::parse(&fs::read(ack_path).map_err(|e| format!("Read ACK: {e}"))?)?;
    if ack.pid != expected_plasma_pid { return Err("Presenter PID does not match Plasma".into()); }
    let frame = fs::metadata(frame_path).map_err(|e| format!("Frame file metadata: {e}"))?;
    if !frame.is_file() || ack.device != frame.dev() || ack.inode != frame.ino() {
        return Err("Presenter ACK does not refer to this producer frame file".into());
    }
    if last_sequence.is_some_and(|previous| ack.sequence <= previous) {
        return Err("Presenter sequence has not advanced".into());
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Clock before epoch: {e}"))?.as_millis();
    if u128::from(ack.epoch_millis) > now + 2000
        || now.saturating_sub(u128::from(ack.epoch_millis)) > max_age.as_millis() {
        return Err("Presenter ACK timestamp is stale or in the future".into());
    }
    Ok(ack)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_protocol() {
        let ack = PresenterAck::parse(b"SSPLACK01\n123\n44\n55\n99\n123456789\n").unwrap();
        assert_eq!((ack.pid, ack.device, ack.inode, ack.sequence), (123, 44, 55, 99));
    }
    #[test]
    fn rejects_bad_records() {
        for record in [
            "SSPLACK00\n123\n44\n55\n99\n123456789\n",
            "SSPLACK01\n0\n44\n55\n99\n123456789\n",
            "SSPLACK01\n123\n44\n55\n0\n123456789\n",
            "SSPLACK01\n123\n44\n55\n99\n123456789\nextra\n",
        ] { assert!(PresenterAck::parse(record.as_bytes()).is_err()); }
    }
}
