//! Exclusive Control Center / Maker coordination, independent of the resident
//! Screenshaver singleton. This module does not own SDL or OpenGL resources.
//!
//! Protocol: one line per connection: `ACTIVATE CONTROL`, `ACTIVATE MAKER`,
//! `SWITCH CONTROL`, or `SWITCH MAKER`. The owner acknowledges receipt, not
//! completion. Only the owner may act on the resulting requests.
use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Application { Control, Maker }
impl Application {
    fn word(self) -> &'static str { match self { Self::Control => "CONTROL", Self::Maker => "MAKER" } }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request { Activate(Application), Switch(Application) }

fn paths() -> Result<(PathBuf, PathBuf), String> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .ok_or("XDG_RUNTIME_DIR is unavailable")?;
    let dir = PathBuf::from(dir);
    Ok((dir.join("screenshaver-editor.lock"), dir.join("screenshaver-editor.sock")))
}

pub enum Entry { Owner(Owner), Forwarded }

pub struct Owner {
    _lock: File,
    socket: PathBuf,
    stop: Sender<()>,
    listener_thread: Option<JoinHandle<()>>,
    requests: Receiver<Request>,
}
impl Owner {
    pub fn try_recv(&self) -> Result<Option<Request>, String> {
        match self.requests.try_recv() {
            Ok(request) => Ok(Some(request)),
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => Err("Application request listener stopped".into()),
        }
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(thread) = self.listener_thread.take() { let _ = thread.join(); }
        let _ = std::fs::remove_file(&self.socket);
    }
}

/// Acquire the editor session or forward the requested mode to its owner.
/// A successful forward means the request was received, not that a transition
/// has finished. The caller must never launch a second editor after forwarding.
pub fn enter(target: Application) -> Result<Entry, String> {
    let (lock_path, socket_path) = paths()?;
    let lock = OpenOptions::new().create(true).read(true).write(true)
        .open(&lock_path).map_err(|e| format!("Editor lock open: {e}"))?;
    match lock.try_lock_exclusive() {
        Ok(()) => {
            // Only the lock owner may remove a socket left by a crashed owner.
            match std::fs::remove_file(&socket_path) {
                Ok(()) => {},
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {},
                Err(e) => return Err(format!("Stale editor socket removal: {e}")),
            }
            let listener = UnixListener::bind(&socket_path)
                .map_err(|e| format!("Editor socket bind: {e}"))?;
            listener.set_nonblocking(true).map_err(|e| e.to_string())?;
            let (tx, rx) = mpsc::channel();
            let (stop_tx, stop_rx) = mpsc::channel();
            let thread = thread::Builder::new().name("screenshaver-editor-ipc".into())
                .spawn(move || {
                    loop {
                        if stop_rx.try_recv().is_ok() { break; }
                        match listener.accept() {
                            Ok((mut stream, _)) => {
                                let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
                                let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));
                                let mut line = String::new();
                                if BufReader::new(&stream).read_line(&mut line).is_ok() {
                                    let request = parse_request(line.trim());
                                    if let Some(request) = request {
                                        // Do not acknowledge a cross-editor request until
                                        // the reciprocal lifecycle integration exists.
                                        let destination = match request {
                                            Request::Activate(app) | Request::Switch(app) => app,
                                        };
                                        if destination != target && !(target == Application::Maker && destination == Application::Control) {
                                            let _ = stream.write_all(b"NOT_READY\n");
                                        } else if tx.send(request).is_ok() {
                                            let _ = stream.write_all(b"ACCEPTED\n");
                                        }
                                    } else { let _ = stream.write_all(b"INVALID\n"); }
                                }
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock =>
                                thread::sleep(Duration::from_millis(15)),
                            Err(_) => break,
                        }
                    }
                }).map_err(|e| format!("Editor listener thread: {e}"))?;
            Ok(Entry::Owner(Owner { _lock: lock, socket: socket_path,
                stop: stop_tx, listener_thread: Some(thread), requests: rx }))
        }
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            // A newly elected owner may still be binding its socket. Bound the
            // startup race; never treat failure as permission to start another UI.
            let mut last = String::new();
            for _ in 0..30 {
                match forward(&socket_path, Request::Activate(target)) {
                    Ok(()) => return Ok(Entry::Forwarded),
                    Err(e) => { last = e; thread::sleep(Duration::from_millis(20)); }
                }
            }
            Err(format!("Editor already owns the lock, but cannot accept activation: {last}"))
        }
        Err(e) => Err(format!("Editor lock acquisition: {e}")),
    }
}

/// Send a request to the currently running editor. Never starts a process.
pub fn request(request: Request) -> Result<(), String> {
    let (_, socket) = paths()?;
    forward(&socket, request)
}
fn forward(socket: &PathBuf, request: Request) -> Result<(), String> {
    let mut stream = UnixStream::connect(socket).map_err(|e| e.to_string())?;
    stream.set_read_timeout(Some(Duration::from_millis(500))).map_err(|e| e.to_string())?;
    stream.set_write_timeout(Some(Duration::from_millis(500))).map_err(|e| e.to_string())?;
    let (verb, target) = match request {
        Request::Activate(target) => ("ACTIVATE", target),
        Request::Switch(target) => ("SWITCH", target),
    };
    stream.write_all(format!("{verb} {}\n", target.word()).as_bytes())
        .map_err(|e| e.to_string())?;
    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply).map_err(|e| e.to_string())?;
    if reply.trim() == "ACCEPTED" { Ok(()) }
    else { Err(format!("Editor refused request: {}", reply.trim())) }
}

fn parse_request(line: &str) -> Option<Request> {
    match line {
        "ACTIVATE CONTROL" => Some(Request::Activate(Application::Control)),
        "ACTIVATE MAKER" => Some(Request::Activate(Application::Maker)),
        "SWITCH CONTROL" => Some(Request::Switch(Application::Control)),
        "SWITCH MAKER" => Some(Request::Switch(Application::Maker)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_defined_commands() {
        assert_eq!(parse_request("ACTIVATE CONTROL"), Some(Request::Activate(Application::Control)));
        assert_eq!(parse_request("ACTIVATE MAKER"), Some(Request::Activate(Application::Maker)));
        assert_eq!(parse_request("SWITCH CONTROL"), Some(Request::Switch(Application::Control)));
        assert_eq!(parse_request("SWITCH MAKER"), Some(Request::Switch(Application::Maker)));
        for invalid in ["", "STOP", "ACTIVATE", "ACTIVATE SCRENSHAVER", "SWITCH MAKER EXTRA", "switch maker"] {
            assert_eq!(parse_request(invalid), None);
        }
    }
}
