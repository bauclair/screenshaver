//! Cross-process wallpaper pause/resume coordination.
//! A transaction lock serializes requesters; acknowledgment files echo the
//! current request token. Never treat an unverified acknowledgment as success.
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use fs2::FileExt;

const CONTROL_WAIT: Duration = Duration::from_millis(500);
const POLL_INTERVAL: Duration = Duration::from_millis(2);
const ACTIVE_FILE: &str = "screenshaver-wallpaper.active";
const PAUSE_REQUEST_FILE: &str = "screenshaver-wallpaper.pause";
const PAUSE_ACK_FILE: &str = "screenshaver-wallpaper.paused";
const RESUME_ACK_FILE: &str = "screenshaver-wallpaper.resumed";
const RESUME_TOKEN_FILE: &str = "screenshaver-wallpaper.resume-token";
const TRANSACTION_LOCK_FILE: &str = "screenshaver-wallpaper.transaction.lock";
static NEXT_TOKEN: AtomicU64 = AtomicU64::new(0);
// Set only by the early-exit developer diagnostic, before any runtime starts.
static TEST_RUNTIME_DIRECTORY: OnceLock<PathBuf> = OnceLock::new();


// Phase 10C-3B-3B-3A: native ownership wire-format verification.
// Kept in the existing wallpaper-control module and isolated protocol harness.
// No production native grant is published by this code.
const NATIVE_GRANT_FILE: &str = "screenshaver-plasma-ownership.grant";
const NATIVE_REVOKED_FILE: &str = "screenshaver-plasma-ownership.revoked";

// Phase 6T: strict on-disk correlation record with isolated, journal-first
// publication and verified retirement. Production publication and automatic
// recovery remain disabled.
const HANDOFF_JOURNAL_FILE: &str = "screenshaver-wallpaper.handoff";

#[derive(Debug, Clone, PartialEq, Eq)]
struct HandoffJournal {
    pause_token: String,
    native: NativeOwnershipRecord,
}

impl HandoffJournal {
    fn validate(&self) -> Result<(), String> {
        self.native.validate()?;
        if self.pause_token.is_empty() || self.pause_token.len() > 128
            || !self.pause_token.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err("Invalid legacy pause token in handoff journal".into());
        }
        Ok(())
    }

    fn encode(&self) -> Result<String, String> {
        self.validate()?;
        Ok(format!("v1 {} {} {} {}\n", self.pause_token,
            self.native.token, self.native.pid, self.native.start_ticks))
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > 320 || !bytes.is_ascii() || bytes.last() != Some(&b'\n')
            || bytes[..bytes.len().saturating_sub(1)].iter().any(|b| b.is_ascii_control())
        {
            return Err("Invalid handoff journal framing".into());
        }
        let text = std::str::from_utf8(&bytes[..bytes.len() - 1])
            .map_err(|e| e.to_string())?;
        let fields: Vec<_> = text.split(' ').collect();
        if fields.len() != 5 || fields[0] != "v1" {
            return Err("Invalid handoff journal version or field count".into());
        }
        // Reuse the native wire decoder so both formats enforce the same
        // exact token, PID, and process-start canonicalization.
        let native = NativeOwnershipRecord::decode(
            format!("v1 {} {} {}\n", fields[2], fields[3], fields[4]).as_bytes()
        )?;
        let journal = Self { pause_token: fields[1].to_owned(), native };
        journal.validate()?;
        if journal.encode()?.as_bytes() != bytes {
            return Err("Noncanonical handoff journal".into());
        }
        Ok(journal)
    }
}

// Durable, no-replace journal publication primitive. The caller MUST retain
// the exclusive wallpaper transaction lock and acknowledged legacy pause for
// the entire operation. This primitive does not publish a native grant, does
// not authorize resume, and is not itself a production handoff API.
//
// Journal-first publication is essential: an interrupted handoff must leave
// blocking evidence rather than silently allowing another renderer to start.
fn publish_handoff_journal_locked(journal: &HandoffJournal) -> Result<(), String> {
    journal.validate()?;
    let path = required_path(HANDOFF_JOURNAL_FILE)?;
    let seq = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
    let temp = path.with_extension(format!("handoff-tmp-{}-{seq}", std::process::id()));
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&temp)
            .map_err(|e| format!("Cannot stage handoff journal: {e}"))?;
        file.write_all(journal.encode()?.as_bytes()).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        fs::hard_link(&temp, &path)
            .map_err(|e| format!("Cannot publish handoff journal without replacement: {e}"))?;
        // Persist the new directory entry, not just the staged file's bytes.
        // On failure the journal remains blocking evidence; never unlink it.
        let parent = path.parent().ok_or("Handoff journal has no parent directory")?;
        File::open(parent).and_then(|directory| directory.sync_all())
            .map_err(|e| format!("Cannot sync handoff journal directory: {e}"))?;
        Ok(())
    })();
    let _ = fs::remove_file(&temp);
    result
}

// Existing protocol tests continue to use the isolated entry point. This
// wrapper must not be used by the production coordinator.
fn publish_isolated_handoff_journal(journal: &HandoffJournal) -> Result<(), String> {
    if TEST_RUNTIME_DIRECTORY.get().is_none() {
        return Err("Handoff journal publication unavailable outside protocol tests".into());
    }
    publish_handoff_journal_locked(journal)
}

// Read-only: a missing journal is NOT evidence that a paused legacy renderer
// can resume. Reject symlinks, directories, and malformed records.
fn read_handoff_journal() -> Result<Option<HandoffJournal>, String> {
    let path = required_path(HANDOFF_JOURNAL_FILE)?;
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("Cannot inspect handoff journal: {e}")),
        Ok(metadata) if !metadata.file_type().is_file() =>
            Err("Handoff journal is not a regular file".into()),
        Ok(_) => {
            let bytes = fs::read(&path).map_err(|e| format!("Cannot read handoff journal: {e}"))?;
            HandoffJournal::decode(&bytes).map(Some)
        }
    }
}


#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeOwnershipRecord {
    token: String,
    pid: u32,
    start_ticks: u64,
}

impl NativeOwnershipRecord {
    fn validate(&self) -> Result<(), String> {
        if self.token.len() != 32 || !self.token.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
            return Err("Native ownership token must be 32 lowercase hex characters".into());
        }
        if self.pid == 0 || self.start_ticks == 0 {
            return Err("Plasma process identity must be nonzero".into());
        }
        Ok(())
    }

    fn encode(&self) -> Result<String, String> {
        self.validate()?;
        Ok(format!("v1 {} {} {}\n", self.token, self.pid, self.start_ticks))
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > 256 || !bytes.is_ascii() {
            return Err("Native ownership record exceeds 256 bytes or is not ASCII".into());
        }
        // The C++ Plasma reader requires one canonical, newline-terminated
        // record. Rust must reject the same noncanonical bytes; otherwise a
        // grant could be accepted by the supervisor but rejected by Plasma.
        if bytes.last() != Some(&b'\n') || bytes[..bytes.len() - 1]
            .iter().any(|byte| byte.is_ascii_control())
        {
            return Err("Native ownership record must be one newline-terminated ASCII line".into());
        }
        let text = std::str::from_utf8(&bytes[..bytes.len() - 1])
            .map_err(|e| e.to_string())?;
        let fields: Vec<_> = text.split(' ').collect();
        if fields.len() != 4 || fields[0] != "v1" {
            return Err("Invalid native ownership record version or field count".into());
        }
        let canonical_decimal = |value: &str| {
            !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit())
                && (value == "0" || !value.starts_with('0'))
        };
        if !canonical_decimal(fields[2]) || !canonical_decimal(fields[3]) {
            return Err("Noncanonical Plasma process identity".into());
        }
        let record = Self {
            token: fields[1].to_owned(),
            pid: fields[2].parse().map_err(|_| "Invalid Plasma PID")?,
            start_ticks: fields[3].parse().map_err(|_| "Invalid Plasma start ticks")?,
        };
        record.validate()?;
        Ok(record)
    }
}

// A matching native ACK plus absent grant verifies only the native protocol's
// GL-submission fence. It does NOT prove compositor presentation quiescence.
// A future coordinator must also verify the live Plasma process instance.
fn verify_native_revocation(
    grant_path: &std::path::Path,
    ack_path: &std::path::Path,
    expected: &NativeOwnershipRecord,
) -> Result<(), String> {
    expected.validate()?;
    // Never follow a substituted ACK symlink. This is a conservative
    // filesystem precondition, not a replacement for a transaction lock.
    let ack_metadata = fs::symlink_metadata(ack_path)
        .map_err(|e| format!("Cannot inspect native revocation ACK: {e}"))?;
    if !ack_metadata.file_type().is_file() {
        return Err("Native revocation ACK is not a regular file".into());
    }
    let bytes = fs::read(ack_path)
        .map_err(|e| format!("Cannot read native revocation ACK: {e}"))?;
    if NativeOwnershipRecord::decode(&bytes)? != *expected {
        return Err("Native revocation ACK does not match the withdrawn transaction".into());
    }
    match fs::symlink_metadata(grant_path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Cannot verify native grant absence: {e}")),
        Ok(_) => Err("Native grant remains present; legacy resume forbidden".into()),
    }
}

// Phase 7K-4A: read-only eligibility for explicit legacy reauthorization.
// This is NOT an instruction to clear the renderer's native-ownership latch.
// The pause must remain held until a separately verified, transaction-scoped
// reauthorization operation is implemented.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LegacyReauthorizationAssessment {
    Eligible { pause_token: String },
    Unverified(String),
}

fn verify_legacy_reauthorization_evidence(
    expected: &HandoffJournal,
    journal_path: &std::path::Path,
    pause_path: &std::path::Path,
    grant_path: &std::path::Path,
    revoked_path: &std::path::Path,
    current_plasma: impl FnOnce(u32) -> Result<Option<u64>, String>,
) -> Result<(), String> {
    expected.validate()?;
    let journal_meta = fs::symlink_metadata(journal_path)
        .map_err(|e| format!("Cannot inspect handoff journal: {e}"))?;
    if !journal_meta.file_type().is_file() {
        return Err("Handoff journal is not a regular file".into());
    }
    let actual = HandoffJournal::decode(&fs::read(journal_path)
        .map_err(|e| format!("Cannot read handoff journal: {e}"))?)?;
    if actual != *expected {
        return Err("Handoff journal does not match reauthorization transaction".into());
    }
    let pause_meta = fs::symlink_metadata(pause_path)
        .map_err(|e| format!("Cannot inspect retained pause request: {e}"))?;
    if !pause_meta.file_type().is_file() {
        return Err("Retained pause request is not a regular file".into());
    }
    let pause_bytes = fs::read(pause_path)
        .map_err(|e| format!("Cannot read retained pause request: {e}"))?;
    if pause_bytes != format!("{}\n", expected.pause_token).as_bytes() {
        return Err("Retained pause request does not match handoff journal".into());
    }
    if current_plasma(expected.native.pid)? != Some(expected.native.start_ticks) {
        return Err("Plasma process identity changed during native ownership".into());
    }
    verify_native_revocation(grant_path, revoked_path, &expected.native)?;
    // Recheck the durable correlation and pause after observing the ACK.
    // The coordinator must hold the exclusive transaction lock across this
    // verification and the eventual authorization operation.
    if HandoffJournal::decode(&fs::read(journal_path)
        .map_err(|e| format!("Cannot recheck handoff journal: {e}"))?)? != *expected
        || fs::read(pause_path).map_err(|e| format!("Cannot recheck pause: {e}"))?
            != pause_bytes
    {
        return Err("Handoff evidence changed during reauthorization verification".into());
    }
    verify_native_revocation(grant_path, revoked_path, &expected.native)
}

/// Read-only diagnostic: the caller must retain the transaction lock and pause.
/// Eligibility is not compositor quiescence and does not authorize resume.
#[allow(dead_code)]
pub(crate) fn assess_legacy_reauthorization(
    pause_token: &str,
    native_token: &str,
    plasma: &crate::manage_wallpaper_ownership::PlasmaInstance,
) -> LegacyReauthorizationAssessment {
    let expected = HandoffJournal {
        pause_token: pause_token.to_owned(),
        native: NativeOwnershipRecord {
            token: native_token.to_owned(),
            pid: plasma.pid,
            start_ticks: plasma.start_ticks,
        },
    };
    let verify = (|| -> Result<(), String> {
        verify_legacy_reauthorization_evidence(
            &expected,
            &required_path(HANDOFF_JOURNAL_FILE)?,
            &required_path(PAUSE_REQUEST_FILE)?,
            &required_path(NATIVE_GRANT_FILE)?,
            &required_path(NATIVE_REVOKED_FILE)?,
            process_start_ticks,
        )
    })();
    match verify {
        Ok(()) => LegacyReauthorizationAssessment::Eligible {
            pause_token: expected.pause_token,
        },
        Err(error) => LegacyReauthorizationAssessment::Unverified(error),
    }
}

// Phase 7K-4B-1: isolated recovery handshake wire contract. Neither record
// authorizes legacy rendering. Live publication and latch release remain disabled.
const REAUTHORIZE_REQUEST_FILE: &str = "screenshaver-wallpaper.reauthorize-request";
const REAUTHORIZE_ACK_FILE: &str = "screenshaver-wallpaper.reauthorize-ack";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReauthorizationStage { Request, Acknowledged, RetirementPrepared, RetirementCommitted, LatchReleaseAuthorized }

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReauthorizationRecord {
    stage: ReauthorizationStage,
    handoff: HandoffJournal,
}

impl ReauthorizationRecord {
    fn encode(&self) -> Result<String, String> {
        self.handoff.validate()?;
        let stage = match self.stage {
            ReauthorizationStage::Request => "request",
            ReauthorizationStage::Acknowledged => "ack",
            ReauthorizationStage::RetirementPrepared => "retirement-prepared",
            ReauthorizationStage::RetirementCommitted => "retirement-committed",
            ReauthorizationStage::LatchReleaseAuthorized => "latch-release-authorized",
        };
        Ok(format!("v1 {stage} {} {} {} {}\n",
            self.handoff.pause_token, self.handoff.native.token,
            self.handoff.native.pid, self.handoff.native.start_ticks))
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > 320 || !bytes.is_ascii() || bytes.last() != Some(&b'\n')
            || bytes[..bytes.len().saturating_sub(1)].iter().any(|b| b.is_ascii_control()) {
            return Err("Invalid reauthorization record framing".into());
        }
        let text = std::str::from_utf8(&bytes[..bytes.len() - 1])
            .map_err(|e| e.to_string())?;
        let fields: Vec<_> = text.split(' ').collect();
        if fields.len() != 6 || fields[0] != "v1" {
            return Err("Invalid reauthorization record field count".into());
        }
        let stage = match fields[1] {
            "request" => ReauthorizationStage::Request,
            "ack" => ReauthorizationStage::Acknowledged,
            "retirement-prepared" => ReauthorizationStage::RetirementPrepared,
            "retirement-committed" => ReauthorizationStage::RetirementCommitted,
            "latch-release-authorized" => ReauthorizationStage::LatchReleaseAuthorized,
            _ => return Err("Invalid reauthorization stage".into()),
        };
        let handoff = HandoffJournal::decode(
            format!("v1 {} {} {} {}\n", fields[2], fields[3], fields[4], fields[5]).as_bytes())?;
        let record = Self { stage, handoff };
        if record.encode()?.as_bytes() != bytes {
            return Err("Noncanonical reauthorization record".into());
        }
        Ok(record)
    }
}

// The ACK is correlated to the request, but is not permission to resume.
// A future coordinator must recheck native revocation under its retained lock
// and perform an explicit renderer-side latch transition.
fn verify_reauthorization_pair(
    request: &std::path::Path,
    ack: &std::path::Path,
    expected: &HandoffJournal,
) -> Result<(), String> {
    let read = |path: &std::path::Path| -> Result<ReauthorizationRecord, String> {
        let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        if !metadata.file_type().is_file() || metadata.len() > 320 {
            return Err("Reauthorization evidence is not a small regular file".into());
        }
        ReauthorizationRecord::decode(&fs::read(path).map_err(|e| e.to_string())?)
    };
    let req = read(request)?;
    let acknowledged = read(ack)?;
    if req.stage != ReauthorizationStage::Request || req.handoff != *expected
        || acknowledged.stage != ReauthorizationStage::Acknowledged
        || acknowledged.handoff != *expected {
        return Err("Reauthorization request/ACK transaction mismatch".into());
    }
    // Recheck request to reject replacement during inspection.
    if read(request)? != req {
        return Err("Reauthorization request changed during verification".into());
    }
    Ok(())
}

pub(crate) fn protocol_test_runtime_active() -> bool {
    TEST_RUNTIME_DIRECTORY.get().is_some()
}

// Phase 7K-4B-2: renderer-side acknowledgment of a recovery proposal.
// This is an observation while PAUSED, not permission to clear the latch.
// The coordinator is still responsible for holding its transaction lock.
#[allow(dead_code)]
pub(crate) fn acknowledge_reauthorization_while_paused(
    observed_pause_token: &str,
) -> Result<bool, String> {
    let request_path = required_path(REAUTHORIZE_REQUEST_FILE)?;
    let request_meta = match fs::symlink_metadata(&request_path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(format!("Cannot inspect reauthorization request: {e}")),
        Ok(meta) => meta,
    };
    if !request_meta.file_type().is_file() || request_meta.len() > 320 {
        return Err("Reauthorization request is not a small regular file".into());
    }
    let bytes = fs::read(&request_path)
        .map_err(|e| format!("Cannot read reauthorization request: {e}"))?;
    let request = ReauthorizationRecord::decode(&bytes)?;
    if request.stage != ReauthorizationStage::Request
        || request.handoff.pause_token != observed_pause_token {
        return Err("Reauthorization request does not match acknowledged pause".into());
    }
    let expected = &request.handoff;
    verify_legacy_reauthorization_evidence(
        expected,
        &required_path(HANDOFF_JOURNAL_FILE)?,
        &required_path(PAUSE_REQUEST_FILE)?,
        &required_path(NATIVE_GRANT_FILE)?,
        &required_path(NATIVE_REVOKED_FILE)?,
        process_start_ticks,
    )?;
    // Recheck the exact request after verifying revocation and correlation.
    if fs::read(&request_path).map_err(|e| e.to_string())? != bytes {
        return Err("Reauthorization request changed during verification".into());
    }
    let ack_path = required_path(REAUTHORIZE_ACK_FILE)?;
    let ack = ReauthorizationRecord {
        stage: ReauthorizationStage::Acknowledged,
        handoff: expected.clone(),
    };
    let encoded = ack.encode()?;
    // Idempotent only for an identical ACK; never overwrite another transaction.
    match fs::symlink_metadata(&ack_path) {
        Ok(meta) if meta.file_type().is_file() => {
            if fs::read(&ack_path).map_err(|e| e.to_string())? == encoded.as_bytes() {
                return Ok(true);
            }
            return Err("Conflicting reauthorization ACK already exists".into());
        }
        Ok(_) => return Err("Reauthorization ACK is not a regular file".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {},
        Err(e) => return Err(format!("Cannot inspect reauthorization ACK: {e}")),
    }
    let sequence = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
    let staging = ack_path.with_extension(format!("reauth-tmp-{}-{sequence}", std::process::id()));
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&staging)
            .map_err(|e| format!("Cannot stage reauthorization ACK: {e}"))?;
        file.write_all(encoded.as_bytes()).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        // One last check before publication. Neither grant absence nor this ACK
        // independently permits the legacy renderer to submit a frame.
        verify_legacy_reauthorization_evidence(
            expected,
            &required_path(HANDOFF_JOURNAL_FILE)?,
            &required_path(PAUSE_REQUEST_FILE)?,
            &required_path(NATIVE_GRANT_FILE)?,
            &required_path(NATIVE_REVOKED_FILE)?,
            process_start_ticks,
        )?;
        if fs::read(&request_path).map_err(|e| e.to_string())? != bytes {
            return Err("Reauthorization request changed before ACK publication".into());
        }
        fs::hard_link(&staging, &ack_path)
            .map_err(|e| format!("Cannot publish reauthorization ACK without replacement: {e}"))?;
        Ok(())
    })();
    let _ = fs::remove_file(&staging);
    result.map(|()| true)
}


// Phase 7K-4C-1: isolated release authorization. This is a proposal to
// transition ownership, NOT permission to submit a legacy frame. The journal
// and pause remain intact until a separately verified retirement protocol.
const REAUTHORIZE_RELEASE_FILE: &str = "screenshaver-wallpaper.reauthorize-release";

fn verify_release_authorization(expected: &HandoffJournal) -> Result<(), String> {
    if !protocol_test_runtime_active() {
        return Err("Release authorization disabled outside isolated protocol tests".into());
    }
    let path = required_path(REAUTHORIZE_RELEASE_FILE)?;
    let meta = fs::symlink_metadata(&path)
        .map_err(|e| format!("Cannot inspect release authorization: {e}"))?;
    if !meta.file_type().is_file() || meta.len() > 320 {
        return Err("Release authorization is not a small regular file".into());
    }
    let release = ReauthorizationRecord::decode(&fs::read(&path).map_err(|e| e.to_string())?)?;
    if release.stage != ReauthorizationStage::Acknowledged || release.handoff != *expected {
        return Err("Release authorization transaction mismatch".into());
    }
    verify_reauthorization_pair(
        &required_path(REAUTHORIZE_REQUEST_FILE)?,
        &required_path(REAUTHORIZE_ACK_FILE)?, expected)?;
    verify_legacy_reauthorization_evidence(expected,
        &required_path(HANDOFF_JOURNAL_FILE)?,
        &required_path(PAUSE_REQUEST_FILE)?,
        &required_path(NATIVE_GRANT_FILE)?,
        &required_path(NATIVE_REVOKED_FILE)?, process_start_ticks)?;
    let reread = fs::read(&path).map_err(|e| e.to_string())?;
    if reread != release.encode()?.as_bytes() {
        return Err("Release authorization changed during verification".into());
    }
    Ok(())
}

// Called by the isolated coordinator after receiving the paused renderer ACK.
// A production coordinator will need a retained exclusive transaction lock.
fn publish_isolated_release_authorization(expected: &HandoffJournal) -> Result<(), String> {
    if !protocol_test_runtime_active() {
        return Err("Release publisher disabled outside protocol tests".into());
    }
    verify_reauthorization_pair(
        &required_path(REAUTHORIZE_REQUEST_FILE)?,
        &required_path(REAUTHORIZE_ACK_FILE)?, expected)?;
    verify_legacy_reauthorization_evidence(expected,
        &required_path(HANDOFF_JOURNAL_FILE)?,
        &required_path(PAUSE_REQUEST_FILE)?,
        &required_path(NATIVE_GRANT_FILE)?,
        &required_path(NATIVE_REVOKED_FILE)?, process_start_ticks)?;
    let path = required_path(REAUTHORIZE_RELEASE_FILE)?;
    let record = ReauthorizationRecord {
        stage: ReauthorizationStage::Acknowledged, handoff: expected.clone(),
    };
    let bytes = record.encode()?;
    let seq = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
    let staging = path.with_extension(format!("release-tmp-{}-{seq}", std::process::id()));
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&staging)
            .map_err(|e| format!("Cannot stage release authorization: {e}"))?;
        file.write_all(bytes.as_bytes()).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        verify_reauthorization_pair(
            &required_path(REAUTHORIZE_REQUEST_FILE)?,
            &required_path(REAUTHORIZE_ACK_FILE)?, expected)?;
        verify_legacy_reauthorization_evidence(expected,
            &required_path(HANDOFF_JOURNAL_FILE)?,
            &required_path(PAUSE_REQUEST_FILE)?,
            &required_path(NATIVE_GRANT_FILE)?,
            &required_path(NATIVE_REVOKED_FILE)?, process_start_ticks)?;
        fs::hard_link(&staging, &path)
            .map_err(|e| format!("Cannot publish release authorization without replacement: {e}"))?;
        Ok(())
    })();
    let _ = fs::remove_file(&staging);
    result
}

// Independent renderer-side evidence check. Does not mutate the renderer.
pub(crate) fn verify_isolated_renderer_release(pause_token: &str) -> Result<(), String> {
    if !protocol_test_runtime_active() {
        return Err("Renderer release verification disabled outside protocol tests".into());
    }
    let journal = read_handoff_journal()?.ok_or("Missing release handoff journal")?;
    if journal.pause_token != pause_token {
        return Err("Renderer pause identity differs from release transaction".into());
    }
    verify_release_authorization(&journal)
}

// Increment 09A: read-only, transaction-correlated native scene-graph
// retirement OBSERVATION. This is deliberately weaker than compositor
// presentation quiescence and MUST NOT authorize legacy resume.
// No production code publishes this record or consumes it as permission.
const NATIVE_SCENE_RETIREMENT_FILE: &str = "screenshaver-plasma-scene-retirement.observed";

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeSceneRetirementObservation {
    native: NativeOwnershipRecord,
    node_id: u64,
}

impl NativeSceneRetirementObservation {
    fn encode(&self) -> Result<String, String> {
        self.native.validate()?;
        if self.node_id == 0 {
            return Err("Native render-node identity must be nonzero".into());
        }
        Ok(format!("v1 scene-retired {} {} {} {}\n",
            self.native.token, self.native.pid, self.native.start_ticks, self.node_id))
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.is_empty() || bytes.len() > 256 || !bytes.is_ascii()
            || bytes.last() != Some(&b'\n')
            || bytes[..bytes.len() - 1].iter().any(|b| b.is_ascii_control()) {
            return Err("Invalid native scene-retirement framing".into());
        }
        let line = std::str::from_utf8(&bytes[..bytes.len() - 1])
            .map_err(|e| e.to_string())?;
        let fields: Vec<_> = line.split(' ').collect();
        if fields.len() != 6 || fields[0] != "v1" || fields[1] != "scene-retired" {
            return Err("Invalid native scene-retirement record".into());
        }
        let native = NativeOwnershipRecord::decode(
            format!("v1 {} {} {}\n", fields[2], fields[3], fields[4]).as_bytes())?;
        let node_id: u64 = fields[5].parse()
            .map_err(|_| "Invalid native render-node identity")?;
        let observation = Self { native, node_id };
        if observation.encode()?.as_bytes() != bytes {
            return Err("Noncanonical native scene-retirement record".into());
        }
        Ok(observation)
    }
}

// Diagnostic-only verification. A single node observation is NOT proof that
// every monitor's node is gone, nor proof that KWin has retired a frame.
#[allow(dead_code)]
fn verify_native_scene_retirement_observation(
    path: &std::path::Path,
    expected: &NativeOwnershipRecord,
    expected_node_id: u64,
) -> Result<(), String> {
    if !protocol_test_runtime_active() {
        return Err("Native scene-retirement verification disabled outside protocol tests".into());
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|e| format!("Cannot inspect native scene-retirement observation: {e}"))?;
    if !metadata.file_type().is_file() || metadata.len() > 256 {
        return Err("Native scene-retirement observation is not a small regular file".into());
    }
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let observation = NativeSceneRetirementObservation::decode(&bytes)?;
    if observation.native != *expected || observation.node_id != expected_node_id {
        return Err("Native scene-retirement observation does not match transaction and node".into());
    }
    if fs::read(path).map_err(|e| e.to_string())? != bytes {
        return Err("Native scene-retirement observation changed during verification".into());
    }
    Ok(())
}

// Increment 09D: isolated verification of transaction-wide *observed* nodes.
// This is NOT proof of compositor quiescence or exhaustive Qt node discovery.
#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeNodeLedger {
    native: NativeOwnershipRecord,
    // Node identity -> whether the observed node has been destroyed.
    nodes: std::collections::BTreeMap<u64, bool>,
}

impl NativeNodeLedger {
    fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.is_empty() || bytes.len() > 65536 || !bytes.is_ascii()
            || bytes.last() != Some(&b'\n') || bytes.contains(&b'\r')
            || bytes.contains(&0) {
            return Err("Invalid native node-ledger framing".into());
        }
        let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
        let mut lines = text.lines();
        let header = lines.next().ok_or("Missing node-ledger header")?;
        let parts: Vec<_> = header.split(' ').collect();
        if parts.len() != 5 || parts[0] != "v1" || parts[1] != "nodes" {
            return Err("Invalid node-ledger header".into());
        }
        let native = NativeOwnershipRecord::decode(
            format!("v1 {} {} {}\n", parts[2], parts[3], parts[4]).as_bytes())?;
        let mut nodes = std::collections::BTreeMap::new();
        let mut previous = 0u64;
        for line in lines {
            let fields: Vec<_> = line.split(' ').collect();
            if fields.len() != 2 || fields[0].is_empty()
                || fields[0].starts_with('0') {
                return Err("Invalid node-ledger entry".into());
            }
            let id: u64 = fields[0].parse().map_err(|_| "Invalid node ID")?;
            if id <= previous { return Err("Duplicate or unordered node ID".into()); }
            let retired = match fields[1] {
                "retired" => true, "active" => false,
                _ => return Err("Invalid node state".into()),
            };
            nodes.insert(id, retired);
            previous = id;
        }
        if nodes.is_empty() { return Err("Empty node ledger".into()); }
        Ok(Self { native, nodes })
    }
}

#[allow(dead_code)]
fn verify_isolated_native_node_ledger(
    path: &std::path::Path,
    expected: &NativeOwnershipRecord,
    expected_nodes: &[u64],
) -> Result<(), String> {
    if !protocol_test_runtime_active() {
        return Err("Native node-ledger verification disabled outside protocol tests".into());
    }
    let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !meta.file_type().is_file() || meta.len() > 65536 {
        return Err("Node ledger must be a bounded regular file".into());
    }
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let ledger = NativeNodeLedger::decode(&bytes)?;
    if ledger.native != *expected || ledger.nodes.len() != expected_nodes.len()
        || expected_nodes.iter().any(|id| ledger.nodes.get(id) != Some(&true)) {
        return Err("Native node-ledger incomplete or mismatched".into());
    }
    if fs::read(path).map_err(|e| e.to_string())? != bytes {
        return Err("Native node-ledger changed during verification".into());
    }
    Ok(())
}

// Increment 09G: validate the 09F C++ diagnostic census alongside the
// transaction node ledger. This remains isolated test-only evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeNodeCensusState { Open, Closed, Invalid }

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeNodeCensus {
    native: NativeOwnershipRecord,
    state: NativeNodeCensusState,
}

impl NativeNodeCensus {
    fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.is_empty() || bytes.len() > 256 || !bytes.is_ascii()
            || bytes.last() != Some(&b'\n') || bytes.contains(&b'\r')
            || bytes.contains(&0) || bytes[..bytes.len()-1].contains(&b'\n') {
            return Err("Invalid native node-census framing".into());
        }
        let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
        let fields: Vec<_> = text.trim_end_matches('\n').split(' ').collect();
        if fields.len() != 6 || fields[0] != "v1" || fields[1] != "census" {
            return Err("Invalid native node-census header".into());
        }
        let native = NativeOwnershipRecord::decode(
            format!("v1 {} {} {}\n", fields[2], fields[3], fields[4]).as_bytes())?;
        let state = match fields[5] {
            "open" => NativeNodeCensusState::Open,
            "closed" => NativeNodeCensusState::Closed,
            "invalid" => NativeNodeCensusState::Invalid,
            _ => return Err("Invalid native node-census state".into()),
        };
        Ok(Self { native, state })
    }
}

#[allow(dead_code)]
fn verify_isolated_closed_native_node_census(
    census_path: &std::path::Path,
    ledger_path: &std::path::Path,
    expected: &NativeOwnershipRecord,
    expected_nodes: &[u64],
) -> Result<(), String> {
    if !protocol_test_runtime_active() {
        return Err("Native node-census verification disabled outside protocol tests".into());
    }
    // Check both records before and after decoding. A changing census or ledger
    // must fail rather than be treated as a stable diagnostic snapshot.
    fn read_regular_bounded(path: &std::path::Path, limit: u64) -> Result<Vec<u8>, String> {
        let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        if !meta.file_type().is_file() || meta.len() > limit {
            return Err("Diagnostic record is not a bounded regular file".into());
        }
        fs::read(path).map_err(|e| e.to_string())
    }
    let census_bytes = read_regular_bounded(census_path, 256)?;
    let ledger_bytes = read_regular_bounded(ledger_path, 65536)?;
    let census = NativeNodeCensus::decode(&census_bytes)?;
    let ledger = NativeNodeLedger::decode(&ledger_bytes)?;
    if census.native != *expected || ledger.native != *expected
        || census.state != NativeNodeCensusState::Closed
        || expected_nodes.is_empty() || ledger.nodes.len() != expected_nodes.len()
        || expected_nodes.iter().any(|id| ledger.nodes.get(id) != Some(&true)) {
        return Err("Native census is not closed, matching and fully retired".into());
    }
    if read_regular_bounded(census_path, 256)? != census_bytes
        || read_regular_bounded(ledger_path, 65536)? != ledger_bytes {
        return Err("Native census or ledger changed during verification".into());
    }
    Ok(())
}

// Increment 09H: single-file atomic diagnostic snapshot (C++ QSaveFile).
// This removes the torn *cross-file* census/ledger read from this validator.
#[derive(Debug)]
struct NativeNodeSnapshot {
    native: NativeOwnershipRecord,
    state: NativeNodeCensusState,
    nodes: std::collections::BTreeMap<u64, bool>,
}
impl NativeNodeSnapshot {
    fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.is_empty() || bytes.len() > 65536 || !bytes.is_ascii()
            || bytes.last() != Some(&b'\n') || bytes.contains(&b'\r')
            || bytes.contains(&0) {
            return Err("Invalid node snapshot framing".into());
        }
        let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
        let mut lines = text.lines();
        let fields: Vec<_> = lines.next().ok_or("Missing snapshot header")?.split(' ').collect();
        if fields.len() != 6 || fields[0] != "v1" || fields[1] != "snapshot" {
            return Err("Invalid node snapshot header".into());
        }
        let native = NativeOwnershipRecord::decode(
            format!("v1 {} {} {}\n", fields[2], fields[3], fields[4]).as_bytes())?;
        let state = match fields[5] {
            "open" => NativeNodeCensusState::Open,
            "closed" => NativeNodeCensusState::Closed,
            "invalid" => NativeNodeCensusState::Invalid,
            _ => return Err("Invalid node snapshot state".into()),
        };
        let mut nodes = std::collections::BTreeMap::new();
        let mut last = 0u64;
        for line in lines {
            let entry: Vec<_> = line.split(' ').collect();
            if entry.len() != 2 || entry[0].is_empty() || entry[0].starts_with('0')
                || !entry[0].bytes().all(|b| b.is_ascii_digit()) {
                return Err("Invalid snapshot node ID".into());
            }
            let id: u64 = entry[0].parse().map_err(|_| "Invalid snapshot node ID")?;
            if id <= last { return Err("Duplicate or unordered snapshot node ID".into()); }
            let retired = match entry[1] {
                "retired" => true, "active" => false,
                _ => return Err("Invalid snapshot node state".into()),
            };
            nodes.insert(id, retired);
            last = id;
        }
        if nodes.is_empty() { return Err("Empty node snapshot".into()); }
        Ok(Self { native, state, nodes })
    }
}

fn verify_isolated_native_node_snapshot(
    path: &std::path::Path, expected: &NativeOwnershipRecord, expected_nodes: &[u64]
) -> Result<(), String> {
    if !protocol_test_runtime_active() {
        return Err("Snapshot verification disabled outside isolated tests".into());
    }
    let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !meta.file_type().is_file() || meta.len() > 65536 {
        return Err("Snapshot is not a bounded regular file".into());
    }
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let snapshot = NativeNodeSnapshot::decode(&bytes)?;
    if snapshot.native != *expected || snapshot.state != NativeNodeCensusState::Closed
        || expected_nodes.is_empty() || snapshot.nodes.len() != expected_nodes.len()
        || expected_nodes.iter().any(|id| snapshot.nodes.get(id) != Some(&true)) {
        return Err("Snapshot is not matching, closed and fully retired".into());
    }
    Ok(())
}

// Phase 7K-4C-2A: durable retirement PREPARATION, not retirement itself.
// The journal and pause remain present, and legacy_frame_gate() must deny
// submissions. Only the isolated test coordinator can publish this record.
const REAUTHORIZE_RETIREMENT_FILE: &str = "screenshaver-wallpaper.reauthorize-retirement";

fn verify_isolated_retirement_preparation(expected: &HandoffJournal) -> Result<(), String> {
    if !protocol_test_runtime_active() {
        return Err("Retirement preparation disabled outside protocol tests".into());
    }
    verify_release_authorization(expected)?;
    let path = required_path(REAUTHORIZE_RETIREMENT_FILE)?;
    let metadata = fs::symlink_metadata(&path)
        .map_err(|e| format!("Cannot inspect retirement preparation: {e}"))?;
    if !metadata.file_type().is_file() || metadata.len() > 320 {
        return Err("Retirement preparation is not a small regular file".into());
    }
    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
    let record = ReauthorizationRecord::decode(&bytes)?;
    if record.stage != ReauthorizationStage::RetirementPrepared || record.handoff != *expected {
        return Err("Retirement preparation transaction mismatch".into());
    }
    verify_release_authorization(expected)?;
    if fs::read(&path).map_err(|e| e.to_string())? != bytes {
        return Err("Retirement preparation changed during verification".into());
    }
    Ok(())
}

fn publish_isolated_retirement_preparation(expected: &HandoffJournal) -> Result<(), String> {
    if !protocol_test_runtime_active() {
        return Err("Retirement publisher disabled outside protocol tests".into());
    }
    verify_release_authorization(expected)?;
    let path = required_path(REAUTHORIZE_RETIREMENT_FILE)?;
    let bytes = ReauthorizationRecord {
        stage: ReauthorizationStage::RetirementPrepared,
        handoff: expected.clone(),
    }.encode()?;
    let seq = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
    let staging = path.with_extension(format!("retirement-tmp-{}-{seq}", std::process::id()));
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&staging)
            .map_err(|e| format!("Cannot stage retirement preparation: {e}"))?;
        file.write_all(bytes.as_bytes()).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        verify_release_authorization(expected)?;
        fs::hard_link(&staging, &path)
            .map_err(|e| format!("Cannot publish retirement preparation without replacement: {e}"))?;
        Ok(())
    })();
    let _ = fs::remove_file(&staging);
    result
}

pub(crate) fn verify_isolated_renderer_retirement_preparation(pause_token: &str) -> Result<(), String> {
    if !protocol_test_runtime_active() {
        return Err("Renderer retirement verification disabled outside protocol tests".into());
    }
    let journal = read_handoff_journal()?.ok_or("Missing retirement handoff journal")?;
    if journal.pause_token != pause_token {
        return Err("Renderer pause token differs from retirement transaction".into());
    }
    verify_isolated_retirement_preparation(&journal)
}

// Phase 7K-4C-2B: durable decision must outlive both journal and pause.
// Its mere presence is blocking evidence, including interrupted retirement.
const REAUTHORIZE_COMMIT_FILE: &str = "screenshaver-wallpaper.reauthorize-commit";

fn read_retirement_commit() -> Result<Option<HandoffJournal>, String> {
    let path = required_path(REAUTHORIZE_COMMIT_FILE)?;
    let metadata = match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("Cannot inspect retirement decision: {e}")),
        Ok(meta) => meta,
    };
    if !metadata.file_type().is_file() || metadata.len() > 320 {
        return Err("Retirement decision is not a small regular file".into());
    }
    let record = ReauthorizationRecord::decode(&fs::read(&path)
        .map_err(|e| format!("Cannot read retirement decision: {e}"))?)?;
    if record.stage != ReauthorizationStage::RetirementCommitted {
        return Err("Retirement decision has wrong stage".into());
    }
    Ok(Some(record.handoff))
}

fn publish_isolated_retirement_commit(expected: &HandoffJournal) -> Result<(), String> {
    if !protocol_test_runtime_active() {
        return Err("Retirement commit disabled outside protocol tests".into());
    }
    verify_isolated_retirement_preparation(expected)?;
    if read_retirement_commit()?.is_some() {
        return Err("Retirement decision already exists".into());
    }
    let path = required_path(REAUTHORIZE_COMMIT_FILE)?;
    let bytes = ReauthorizationRecord {
        stage: ReauthorizationStage::RetirementCommitted,
        handoff: expected.clone(),
    }.encode()?;
    let seq = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
    let temp = path.with_extension(format!("commit-tmp-{}-{seq}", std::process::id()));
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&temp)
            .map_err(|e| format!("Cannot stage retirement decision: {e}"))?;
        file.write_all(bytes.as_bytes()).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        verify_isolated_retirement_preparation(expected)?;
        fs::hard_link(&temp, &path)
            .map_err(|e| format!("Cannot publish retirement decision without replacement: {e}"))?;
        Ok(())
    })();
    let _ = fs::remove_file(&temp);
    result
}

// The coordinator may retry each operation after an interruption. Each
// remaining marker must match the committed transaction before deletion.
// The decision itself is NEVER retired by this phase.
fn retire_isolated_handoff_markers(expected: &HandoffJournal, withdraw_pause: bool)
    -> Result<(), String> {
    if !protocol_test_runtime_active() {
        return Err("Marker retirement disabled outside protocol tests".into());
    }
    if read_retirement_commit()?.as_ref() != Some(expected) {
        return Err("Missing or mismatched durable retirement decision".into());
    }
    verify_native_revocation(&required_path(NATIVE_GRANT_FILE)?,
        &required_path(NATIVE_REVOKED_FILE)?, &expected.native)?;
    let journal_path = required_path(HANDOFF_JOURNAL_FILE)?;
    if let Some(journal) = read_handoff_journal()? {
        if journal != *expected { return Err("Different handoff journal during retirement".into()); }
        fs::remove_file(&journal_path).map_err(|e| format!("Cannot retire handoff journal: {e}"))?;
    }
    if withdraw_pause {
        let pause_path = required_path(PAUSE_REQUEST_FILE)?;
        match fs::symlink_metadata(&pause_path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {},
            Err(e) => return Err(format!("Cannot inspect pause for retirement: {e}")),
            Ok(meta) => {
                if !meta.file_type().is_file()
                    || fs::read(&pause_path).map_err(|e| e.to_string())?
                        != format!("{}\n", expected.pause_token).as_bytes() {
                    return Err("Different pause request during retirement".into());
                }
                fs::remove_file(&pause_path).map_err(|e| format!("Cannot retire pause: {e}"))?;
            }
        }
    }
    if read_retirement_commit()?.as_ref() != Some(expected) {
        return Err("Retirement decision changed during marker withdrawal".into());
    }
    Ok(())
}

pub(crate) fn verify_isolated_renderer_retirement_committed(pause_token: &str)
    -> Result<(), String> {
    if !protocol_test_runtime_active() {
        return Err("Retirement completion disabled outside protocol tests".into());
    }
    let expected = read_retirement_commit()?.ok_or("Missing retirement decision")?;
    if expected.pause_token != pause_token {
        return Err("Retirement decision differs from renderer pause identity".into());
    }
    if read_handoff_journal()?.is_some() || read_marker(PAUSE_REQUEST_FILE)?.is_some() {
        return Err("Retirement incomplete: handoff journal or pause remains".into());
    }
    verify_native_revocation(&required_path(NATIVE_GRANT_FILE)?,
        &required_path(NATIVE_REVOKED_FILE)?, &expected.native)?;
    if read_retirement_commit()?.as_ref() != Some(&expected) {
        return Err("Retirement decision changed during renderer verification".into());
    }
    Ok(())
}

// Phase 7K-4C-2C-1: an explicit authorization record is distinct from
// retirement commitment. It is evidence to inspect, NOT permission to render.
const REAUTHORIZE_LATCH_RELEASE_FILE: &str = "screenshaver-wallpaper.latch-release-authorization";

fn verify_isolated_latch_release_authorization(expected: &HandoffJournal) -> Result<(), String> {
    if !protocol_test_runtime_active() {
        return Err("Latch release authorization disabled outside protocol tests".into());
    }
    if read_retirement_commit()?.as_ref() != Some(expected) {
        return Err("Missing or mismatched retirement commitment".into());
    }
    if read_handoff_journal()?.is_some() || read_marker(PAUSE_REQUEST_FILE)?.is_some() {
        return Err("Cannot authorize latch release before marker withdrawal".into());
    }
    if process_start_ticks(expected.native.pid)? != Some(expected.native.start_ticks) {
        return Err("Plasma process identity changed before latch release".into());
    }
    verify_native_revocation(&required_path(NATIVE_GRANT_FILE)?,
        &required_path(NATIVE_REVOKED_FILE)?, &expected.native)?;
    let path = required_path(REAUTHORIZE_LATCH_RELEASE_FILE)?;
    let metadata = fs::symlink_metadata(&path)
        .map_err(|e| format!("Cannot inspect latch authorization: {e}"))?;
    if !metadata.file_type().is_file() || metadata.len() > 320 {
        return Err("Latch authorization is not a small regular file".into());
    }
    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
    let record = ReauthorizationRecord::decode(&bytes)?;
    if record.stage != ReauthorizationStage::LatchReleaseAuthorized || record.handoff != *expected {
        return Err("Latch authorization does not match committed transaction".into());
    }
    verify_isolated_renderer_retirement_committed(&expected.pause_token)?;
    if fs::read(&path).map_err(|e| e.to_string())? != bytes {
        return Err("Latch authorization changed during verification".into());
    }
    Ok(())
}

fn publish_isolated_latch_release_authorization(expected: &HandoffJournal) -> Result<(), String> {
    if !protocol_test_runtime_active() {
        return Err("Latch authorization publisher disabled outside protocol tests".into());
    }
    verify_isolated_renderer_retirement_committed(&expected.pause_token)?;
    if read_retirement_commit()?.as_ref() != Some(expected) {
        return Err("Committed retirement does not match authorization request".into());
    }
    if process_start_ticks(expected.native.pid)? != Some(expected.native.start_ticks) {
        return Err("Plasma process identity changed before authorization".into());
    }
    let path = required_path(REAUTHORIZE_LATCH_RELEASE_FILE)?;
    let bytes = ReauthorizationRecord {
        stage: ReauthorizationStage::LatchReleaseAuthorized, handoff: expected.clone(),
    }.encode()?;
    let seq = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
    let temp = path.with_extension(format!("latch-tmp-{}-{seq}", std::process::id()));
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&temp)
            .map_err(|e| format!("Cannot stage latch authorization: {e}"))?;
        file.write_all(bytes.as_bytes()).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        verify_isolated_renderer_retirement_committed(&expected.pause_token)?;
        if read_retirement_commit()?.as_ref() != Some(expected)
            || process_start_ticks(expected.native.pid)? != Some(expected.native.start_ticks) {
            return Err("Latch release evidence changed before publication".into());
        }
        fs::hard_link(&temp, &path)
            .map_err(|e| format!("Cannot publish latch authorization without replacement: {e}"))?;
        Ok(())
    })();
    let _ = fs::remove_file(&temp);
    result
}

pub(crate) fn verify_isolated_renderer_latch_release_authorization(pause_token: &str)
    -> Result<(), String> {
    if !protocol_test_runtime_active() {
        return Err("Renderer latch verification disabled outside protocol tests".into());
    }
    let expected = read_retirement_commit()?.ok_or("Missing retirement commitment")?;
    if expected.pause_token != pause_token {
        return Err("Renderer pause identity does not match latch authorization".into());
    }
    verify_isolated_latch_release_authorization(&expected)
}

// Phase 7K-4C-2C-2: test-only, transaction-scoped latch transition
// eligibility. This is NOT a production frame gate: the durable retirement
// record continues to prohibit every real legacy frame submission.
pub(crate) fn verify_isolated_latch_transition(pause_token: &str) -> Result<(), String> {
    if !protocol_test_runtime_active() {
        return Err("Latch transition unavailable outside protocol tests".into());
    }
    verify_isolated_renderer_latch_release_authorization(pause_token)?;
    // Recheck after authorization; do not silently substitute a fresh handoff.
    let committed = read_retirement_commit()?.ok_or("Retirement commitment missing")?;
    if committed.pause_token != pause_token {
        return Err("Latch transition pause identity mismatch".into());
    }
    if read_handoff_journal()?.is_some() || read_marker(PAUSE_REQUEST_FILE)?.is_some() {
        return Err("Retired markers reappeared before latch transition".into());
    }
    verify_native_revocation(&required_path(NATIVE_GRANT_FILE)?,
        &required_path(NATIVE_REVOKED_FILE)?, &committed.native)?;
    if legacy_frame_gate().is_ok() {
        return Err("Production frame gate unexpectedly opened".into());
    }
    Ok(())
}

// Phase 10C-3B-3B-3D: isolated atomic grant publisher. This helper is
// deliberately restricted to the developer protocol-test runtime; it cannot
// publish into the live Plasma runtime.
fn publish_native_grant_locked(
    path: &std::path::Path,
    record: &NativeOwnershipRecord,
    expected_plasma: &crate::manage_wallpaper_ownership::PlasmaInstance,
    mut current_plasma: impl FnMut() -> Result<crate::manage_wallpaper_ownership::PlasmaInstance, String>,
) -> Result<(), String> {
    record.validate()?;
    if record.pid != expected_plasma.pid || record.start_ticks != expected_plasma.start_ticks {
        return Err("Grant identity does not match expected Plasma instance".into());
    }
    if path != required_path(NATIVE_GRANT_FILE)?.as_path() {
        return Err("Refusing native grant publication outside the current runtime".into());
    }
    if current_plasma()? != *expected_plasma {
        return Err("Plasma identity changed before native grant publication".into());
    }
    // Caller must retain the exclusive transaction lock and verified pause.
    // Fail closed if any grant already exists, including an unreadable file or
    // symlink. Never replace another transaction's authorization.
    match fs::symlink_metadata(path) {
        Ok(_) => return Err("Native grant already exists".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {},
        Err(e) => return Err(format!("Cannot inspect native grant: {e}")),
    }
    let seq = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
    let temp = path.with_extension(format!("grant-tmp-{}-{seq}", std::process::id()));
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&temp)
            .map_err(|e| format!("Cannot stage native grant: {e}"))?;
        file.write_all(record.encode()?.as_bytes()).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        if current_plasma()? != *expected_plasma {
            return Err("Plasma identity changed during grant staging".into());
        }
        // The caller must hold the legacy pause lease throughout this call.
        // This helper is test-only; production will need an interprocess grant
        // transaction lock and crash-recovery ownership journal.
        // `rename` replaces an existing destination on Linux. A second
        // publisher could install its grant after the absence check above,
        // and rename would silently overwrite it. Linking the fully synced
        // staging inode into the destination instead is an atomic
        // no-replace publication: EEXIST is a hard failure.
        fs::hard_link(&temp, path)
            .map_err(|e| format!("Cannot publish native grant without replacement: {e}"))?;
        let parent = path.parent().ok_or("Native grant has no parent directory")?;
        File::open(parent).and_then(|directory| directory.sync_all())
            .map_err(|e| format!("Cannot sync native grant directory: {e}"))?;
        Ok(())
    })();
    // The staging name is never an authorization. After successful linking,
    // the destination continues to reference the complete, synced inode.
    // If unlinking staging fails, leave the published grant intact.
    let _ = fs::remove_file(&temp);
    result
}


// Existing tests retain their isolation boundary. The common primitive is
// intentionally private and may only be invoked while the pause lease owns
// the exclusive transaction lock.
fn publish_test_native_grant(
    path: &std::path::Path,
    record: &NativeOwnershipRecord,
    expected_plasma: &crate::manage_wallpaper_ownership::PlasmaInstance,
    current_plasma: impl FnMut() -> Result<crate::manage_wallpaper_ownership::PlasmaInstance, String>,
) -> Result<(), String> {
    let isolated_root = TEST_RUNTIME_DIRECTORY.get()
        .ok_or("Native grant publisher is disabled outside isolated protocol tests")?;
    if path != isolated_root.join(NATIVE_GRANT_FILE) {
        return Err("Refusing native grant publication outside isolated test path".into());
    }
    publish_native_grant_locked(path, record, expected_plasma, current_plasma)
}


// Batch 3: a lost ACK may be tolerated only when the original pause
// requester's process has exited and no legacy wallpaper runtime is active.
// This is not permission to resume legacy rendering or retire the journal.
fn verify_abandoned_legacy_pause(old: &HandoffJournal) -> Result<(), String> {
    let ack = read_marker(PAUSE_ACK_FILE)?;
    if ack.as_deref() == Some(old.pause_token.as_str()) {
        return Ok(());
    }
    if ack.is_some() {
        return Err("Conflicting legacy pause acknowledgment".into());
    }

    let pid_text = old.pause_token.split('-').next()
        .ok_or("Legacy pause token lacks originating PID")?;
    let pid: u32 = pid_text.parse()
        .map_err(|_| "Legacy pause token has invalid originating PID")?;
    if pid == 0 || pid.to_string() != pid_text {
        return Err("Legacy pause token has noncanonical originating PID".into());
    }
    if process_start_ticks(pid)?.is_some() {
        return Err(format!(
            "Legacy pause acknowledgment absent; originating process {pid} still exists"
        ));
    }

    // ACTIVE_FILE is an advisory presence marker, not a process identity
    // certificate. A present marker always blocks the missing-ACK fallback.
    let active_path = required_path(ACTIVE_FILE)?;
    match fs::symlink_metadata(&active_path) {
        Ok(_) => return Err(
            "Legacy pause acknowledgment absent; wallpaper active marker still exists".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
        Err(error) => return Err(format!("Cannot inspect legacy active marker: {error}")),
    }

    if read_marker(PAUSE_REQUEST_FILE)?.as_deref() != Some(old.pause_token.as_str()) {
        return Err("Legacy pause changed during abandoned-renderer verification".into());
    }
    Ok(())
}

/// Batch 7F-B: withdraw an abandoned production grant without reauthorizing
/// legacy rendering. This is a transaction-locked recovery operation, not a
/// general-purpose grant removal API. The original pause requester must be
/// gone and the active marker absent. A live original Plasma generation must
/// match the requested target; a retired generation can be withdrawn only
/// after its process identity has been independently checked.
/// The retained journal and pause are NEVER removed by this function.
/// Returns true only when this call withdrew the matching grant. A previously
/// withdrawn grant returns false; its revocation ACK must still be verified.
pub(crate) fn withdraw_abandoned_native_grant(
    expected: &crate::manage_wallpaper_ownership::PlasmaInstance,
) -> Result<bool, String> {
    let lock_path = required_path(TRANSACTION_LOCK_FILE)?;
    let lock_meta = fs::symlink_metadata(&lock_path)
        .map_err(|e| format!("Cannot inspect existing transaction lock: {e}"))?;
    if !lock_meta.file_type().is_file() {
        return Err("Transaction lock is not a regular file".into());
    }
    let lock = OpenOptions::new().read(true).write(true).open(&lock_path)
        .map_err(|e| format!("Cannot open transaction lock: {e}"))?;
    lock.try_lock_exclusive()
        .map_err(|e| format!("Abandoned transaction is still owned: {e}"))?;

    let old = read_handoff_journal()?.ok_or("Missing abandoned handoff journal")?;
    // Cross-generation recovery: the requested replacement target must be a
    // live Plasma process. An existing original generation can only be
    // withdrawn through the same-generation path; a retired original may be
    // withdrawn without its (now impossible) revocation ACK. Neither case
    // authorizes the legacy renderer to resume.
    if process_start_ticks(expected.pid)? != Some(expected.start_ticks) {
        return Err("Replacement target Plasma process identity is not live".into());
    }
    let original_ticks = process_start_ticks(old.native.pid)?;
    let original_live = original_ticks == Some(old.native.start_ticks);
    if original_live && (old.native.pid != expected.pid
        || old.native.start_ticks != expected.start_ticks)
    {
        return Err("Original Plasma generation remains live; cross-generation withdrawal forbidden".into());
    }
    let pause_path = required_path(PAUSE_REQUEST_FILE)?;
    let pause_meta = fs::symlink_metadata(&pause_path)
        .map_err(|e| format!("Cannot inspect abandoned legacy pause: {e}"))?;
    if !pause_meta.file_type().is_file()
        || fs::read(&pause_path).map_err(|e| e.to_string())?
            != format!("{}\n", old.pause_token).as_bytes()
    {
        return Err("Abandoned legacy pause does not match journal".into());
    }
    // Even a surviving pause ACK is insufficient to prove abandonment.
    // Refuse recovery if the original requester PID is still allocated.
    let origin = old.pause_token.split('-').next()
        .ok_or("Missing pause origin PID")?;
    let origin_pid: u32 = origin.parse()
        .map_err(|_| "Invalid pause origin PID")?;
    if origin_pid == 0 || origin_pid.to_string() != origin {
        return Err("Noncanonical pause origin PID".into());
    }
    if process_start_ticks(origin_pid)?.is_some() {
        return Err("Original legacy pause requester is still alive".into());
    }
    let active_path = required_path(ACTIVE_FILE)?;
    match fs::symlink_metadata(&active_path) {
        Ok(_) => return Err("Legacy active marker still exists; refusing abandoned withdrawal".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {},
        Err(e) => return Err(format!("Cannot inspect legacy active marker: {e}")),
    }
    verify_abandoned_legacy_pause(&old)?;
    let grant_path = required_path(NATIVE_GRANT_FILE)?;
    let grant_meta = match fs::symlink_metadata(&grant_path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Idempotent only with matching, verified revocation evidence.
            verify_native_revocation(&grant_path, &required_path(NATIVE_REVOKED_FILE)?, &old.native)?;
            return Ok(false);
        }
        Err(e) => return Err(format!("Cannot inspect abandoned native grant: {e}")),
        Ok(meta) => meta,
    };
    if !grant_meta.file_type().is_file() {
        return Err("Abandoned native grant is not a regular file".into());
    }
    if NativeOwnershipRecord::decode(&fs::read(&grant_path)
        .map_err(|e| format!("Cannot read abandoned native grant: {e}"))?)? != old.native
    {
        return Err("Refusing to withdraw a different native transaction".into());
    }
    if read_handoff_journal()?.as_ref() != Some(&old)
        || process_start_ticks(expected.pid)? != Some(expected.start_ticks)
        || process_start_ticks(old.native.pid)? != original_ticks
    {
        return Err("Abandoned transaction or Plasma identity changed before withdrawal".into());
    }
    fs::remove_file(&grant_path)
        .map_err(|e| format!("Cannot withdraw abandoned native grant: {e}"))?;
    File::open(grant_path.parent().ok_or("Native grant parent missing")?)
        .and_then(|directory| directory.sync_all())
        .map_err(|e| format!("Cannot sync withdrawn native grant directory: {e}"))?;
    // The transaction lock is held through the unlink and directory sync.
    // No ACK, node retirement, or compositor quiescence is inferred here.
    drop(lock);
    Ok(true)
}

/// Replace a withdrawn native generation without releasing the legacy pause.
/// This is deliberately separate from legacy reauthorization. The old process
/// must be gone, or its exact GL revocation ACK must be present.
pub(crate) fn replace_native_generation(
    new_token: &str,
    target: &crate::manage_wallpaper_ownership::PlasmaInstance,
) -> Result<(), String> {
    let lock_path = required_path(TRANSACTION_LOCK_FILE)?;
    let lock = OpenOptions::new().read(true).write(true).create(true)
        .open(&lock_path).map_err(|e| e.to_string())?;
    lock.try_lock_exclusive().map_err(|e| format!("Native generation transaction busy: {e}"))?;
    let old = read_handoff_journal()?.ok_or("No existing native handoff to replace")?;
    let pause_path = required_path(PAUSE_REQUEST_FILE)?;
    let pause_meta = fs::symlink_metadata(&pause_path).map_err(|e| e.to_string())?;
    if !pause_meta.file_type().is_file() ||
        fs::read(&pause_path).map_err(|e| e.to_string())?
            != format!("{}\n", old.pause_token).as_bytes() {
        return Err("Existing legacy pause does not match handoff journal".into());
    }
    // Prefer the matching ACK. If shutdown removed it, require proof that
    // the original process exited and no active legacy runtime is advertised.
    verify_abandoned_legacy_pause(&old)?;
    if native_grant_presence()? {
        return Err("Existing native grant must be withdrawn before replacement".into());
    }
    match process_start_ticks(old.native.pid)? {
        None => {},
        Some(ticks) if ticks != old.native.start_ticks => {},
        Some(_) => verify_native_revocation(
            &required_path(NATIVE_GRANT_FILE)?,
            &required_path(NATIVE_REVOKED_FILE)?, &old.native)?,
    }
    let fresh = NativeOwnershipRecord {
        token: new_token.to_owned(), pid: target.pid, start_ticks: target.start_ticks,
    };
    fresh.validate()?;
    if fresh.token == old.native.token {
        return Err("Replacement must use a fresh native transaction token".into());
    }
    if process_start_ticks(target.pid)? != Some(target.start_ticks) {
        return Err("Replacement Plasma process identity is not live".into());
    }
    require_preflight_candidate(assess_native_handoff_preflight(target))?;
    let next = HandoffJournal { pause_token: old.pause_token.clone(), native: fresh.clone() };
    let journal_path = required_path(HANDOFF_JOURNAL_FILE)?;
    let seq = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
    let staged = journal_path.with_extension(format!("replacement-tmp-{}-{seq}", std::process::id()));
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&staged)
            .map_err(|e| e.to_string())?;
        file.write_all(next.encode()?.as_bytes()).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        // Under the transaction lock, the original journal must still match.
        if read_handoff_journal()?.as_ref() != Some(&old) || native_grant_presence()? {
            return Err("Ownership evidence changed during generation replacement".into());
        }
        // Revalidate the legacy side under the retained exclusive lock
        // immediately before the durable journal is replaced.
        verify_abandoned_legacy_pause(&old)?;
        if process_start_ticks(target.pid)? != Some(target.start_ticks) {
            return Err("Replacement Plasma process exited before publication".into());
        }
        fs::rename(&staged, &journal_path).map_err(|e| e.to_string())?;
        File::open(journal_path.parent().ok_or("Missing journal parent")?)
            .and_then(|dir| dir.sync_all()).map_err(|e| e.to_string())?;
        // Journal first, then grant. A failed grant leaves the pause blocking.
        publish_native_grant_locked(&required_path(NATIVE_GRANT_FILE)?, &fresh, target, || {
            match process_start_ticks(target.pid)? {
                Some(ticks) => Ok(crate::manage_wallpaper_ownership::PlasmaInstance {
                    pid: target.pid, start_ticks: ticks,
                }),
                None => Err("Replacement Plasma process exited".into()),
            }
        })
    })();
    let _ = fs::remove_file(&staged);
    drop(lock);
    result
}

pub struct WallpaperPauseGuard {
    token: Option<String>,
    // The lock is held until resume/cancellation, preventing another preview
    // from withdrawing this request or consuming its acknowledgment.
    _transaction_lock: Option<File>,
}

impl WallpaperPauseGuard {
    pub fn acquire() -> Result<Self, String> {
        let lock_path = required_path(TRANSACTION_LOCK_FILE)?;
        let lock = OpenOptions::new().read(true).write(true).create(true)
            .open(&lock_path).map_err(|e| format!("Cannot open wallpaper transaction lock: {e}"))?;
        lock.try_lock_exclusive().map_err(|e| format!("Wallpaper pause transaction already owned: {e}"))?;
        // A new legacy pause transaction must not start while a native grant
        // is outstanding. Inspect under the same transaction lock used by
        // the coordinator; uncertainty is a hard failure, not permission.
        match native_grant_presence() {
            Ok(false) => {},
            Ok(true) => return Err("Cannot acquire legacy pause: native ownership grant exists".into()),
            Err(error) => return Err(format!("Cannot acquire legacy pause: {error}")),
        }
        // A crashed coordinator can leave a pause request behind after its
        // advisory lock has been released. Never overwrite that request:
        // its absence of a native grant is not evidence of safe recovery.
        // Recovery must explicitly establish the old transaction's state.
        // A journal without a pause marker is still unresolved evidence.
        // Do not start a new transaction over it.
        match read_handoff_journal() {
            Ok(None) => {},
            Ok(Some(_)) => return Err("Cannot acquire legacy pause: handoff journal remains".into()),
            Err(error) => return Err(format!("Cannot acquire legacy pause: {error}")),
        }
        match read_marker(PAUSE_REQUEST_FILE) {
            Ok(None) => {},
            Ok(Some(_)) => return Err(
                "Cannot acquire legacy pause: unresolved prior pause transaction".into()
            ),
            Err(error) => return Err(format!(
                "Cannot acquire legacy pause: cannot inspect prior transaction: {error}"
            )),
        }
        let token = request_pause()?;
        Ok(Self { token, _transaction_lock: Some(lock) })
    }
}

/// Phase 10C-3B-2A: a confirmed, long-lived pause of the legacy wallpaper.
///
/// Unlike a temporary preview guard, acquisition MUST observe a running
/// legacy renderer and receive its matching pause acknowledgment. The caller
/// retains this value for the entire proposed native ownership interval.
/// This is NOT a native grant and does not itself prove compositor quiescence.
///
/// IMPORTANT: release only after independently verifying that native frame
/// submission has been revoked. Dropping this value also releases the pause
/// as a recovery fallback; ownership coordination must enforce that ordering.
#[allow(dead_code)]
pub(crate) struct LegacyWallpaperPauseLease {
    guard: WallpaperPauseGuard,
}

#[allow(dead_code)]
impl LegacyWallpaperPauseLease {
    pub(crate) fn acquire() -> Result<Self, String> {
        let guard = WallpaperPauseGuard::acquire()?;
        if guard.token.is_none() {
            return Err("Cannot establish legacy wallpaper pause lease: no active legacy renderer was acknowledged".to_string());
        }
        Ok(Self { guard })
    }

    /// The token identifying the acknowledged legacy pause transaction.
    /// It must never be interpreted as a native rendering authorization.
    pub(crate) fn transaction_token(&self) -> &str {
        self.guard.token.as_deref().expect("confirmed legacy pause token")
    }

    /// Fail closed if a native authorization may still exist. Leave the pause
    /// marker in place, but allow the OS to release the transaction lock.
    /// This does not provide cross-process crash recovery.
    fn inhibit_automatic_resume(&mut self) {
        self.guard.token = None;
    }

    /// Caller must first verify native revocation. Consuming the lease drops
    /// its guard, which requests legacy resume and waits for a resumed frame.
    pub(crate) fn release_after_native_revocation(self) {
        drop(self);
    }

    fn cancel_ungranted_verified(mut self) -> Result<(), String> {
        // The guard still owns the transaction lock until this function returns.
        // Take the token to prevent Drop from repeating an attempted resume.
        let result = match self.guard.token.take() {
            Some(token) => resume_and_wait_for_frame_verified(&token),
            None => Err("No acknowledged legacy pause token to cancel".into()),
        };
        result
    }
}


/// Phase 10C-3B-3B-1: isolated, ungranted handoff preparation.
///
/// The coordinator deliberately has NO native grant operation. A live handoff
/// must not use this type until the native side provides a verified, fail-closed
/// grant/revocation protocol. Dropping this preparatory coordinator is safe
/// because native authorization has never been issued through it.
#[allow(dead_code)]
pub(crate) struct NativeHandoffPreparation {
    ownership: crate::manage_wallpaper_ownership::WallpaperOwnership,
    transaction: crate::manage_wallpaper_ownership::TransactionId,
    plasma: crate::manage_wallpaper_ownership::PlasmaInstance,
    lease: LegacyWallpaperPauseLease,
}

#[allow(dead_code)]
impl NativeHandoffPreparation {
    /// `current_plasma` is an injected, read-only process-instance probe.
    /// Both observations must agree with the requested PID AND start ticks.
    /// No native authorization is issued, even when this returns success.
    pub(crate) fn acquire(
        transaction: crate::manage_wallpaper_ownership::TransactionId,
        plasma: crate::manage_wallpaper_ownership::PlasmaInstance,
        mut current_plasma: impl FnMut() -> Result<crate::manage_wallpaper_ownership::PlasmaInstance, String>,
    ) -> Result<Self, String> {
        use crate::manage_wallpaper_ownership::WallpaperOwnership;
        let mut ownership = WallpaperOwnership::new();
        ownership.begin_native_handoff(transaction.clone(), plasma.clone())
            .map_err(|e| format!("Invalid wallpaper handoff: {e:?}"))?;
        if current_plasma()? != plasma {
            return Err("Plasma process instance changed before legacy pause".into());
        }
        let lease = LegacyWallpaperPauseLease::acquire()?;
        // Recheck while the legacy renderer is held paused: a restarted Plasma
        // instance must not inherit an earlier handoff preparation.
        if current_plasma()? != plasma {
            return Err("Plasma process instance changed during legacy pause".into());
        }
        ownership.legacy_quiesced(&transaction)
            .map_err(|e| format!("Cannot record legacy pause: {e:?}"))?;
        Ok(Self { ownership, transaction, plasma, lease })
    }

    /// Phase 7C: optional, explicitly invoked preflight-aware preparation.
    /// The first observation runs before requesting a legacy pause; the second
    /// runs after the pause ACK, before any future native grant publication.
    /// This is not a grant publisher and cannot prove compositor presentation.
    /// Keep `acquire` for the isolated legacy protocol tests; production callers
    /// must opt into this stricter entry point.
    pub(crate) fn acquire_with_preflight(
        transaction: crate::manage_wallpaper_ownership::TransactionId,
        plasma: crate::manage_wallpaper_ownership::PlasmaInstance,
        mut current_plasma: impl FnMut() -> Result<crate::manage_wallpaper_ownership::PlasmaInstance, String>,
        mut preflight: impl FnMut(&crate::manage_wallpaper_ownership::PlasmaInstance) -> NativePresentationPreflight,
    ) -> Result<Self, String> {
        // Reject absent, mismatched, and unverified observations without
        // disturbing a running legacy wallpaper.
        require_preflight_candidate(preflight(&plasma))?;
        let prepared = Self::acquire(transaction, plasma.clone(), &mut current_plasma)?;
        // The presentation item may have disappeared during the pause. Since
        // this preparation has never issued a grant, dropping it safely
        // releases the legacy pause through its existing guard.
        if let Err(error) = require_preflight_candidate(preflight(&plasma)) {
            prepared.cancel();
            return Err(format!("Native presentation changed during legacy pause: {error}"));
        }
        if current_plasma()? != plasma {
            prepared.cancel();
            return Err("Plasma process instance changed after preflight".into());
        }
        Ok(prepared)
    }

    pub(crate) fn state(&self) -> &crate::manage_wallpaper_ownership::OwnershipState {
        self.ownership.state()
    }

    pub(crate) fn pause_token(&self) -> &str {
        self.lease.transaction_token()
    }

    /// Cancellation is only valid for this *never-granted* preparation.
    pub(crate) fn cancel(self) {
        let Self { lease, ownership: _, transaction: _, plasma: _ } = self;
        lease.release_after_native_revocation();
    }

    /// Phase 7J-1: report the actual resumed-frame outcome to the live
    /// coordinator. The ordinary cancellation path remains for existing tests.
    pub(crate) fn cancel_verified(self) -> Result<(), String> {
        let Self { lease, ownership: _, transaction: _, plasma: _ } = self;
        lease.cancel_ungranted_verified()
    }
}

// Phase 10C-3B-3B-3C: lifecycle guard for a *modeled* native grant.
// No grant publisher is provided. Production integration remains disabled.
#[allow(dead_code)]
pub(crate) struct NativeGrantedTransaction {
    ownership: crate::manage_wallpaper_ownership::WallpaperOwnership,
    transaction: crate::manage_wallpaper_ownership::TransactionId,
    plasma: crate::manage_wallpaper_ownership::PlasmaInstance,
    record: NativeOwnershipRecord,
    journal_published: bool,
    lease: Option<LegacyWallpaperPauseLease>,
}

#[allow(dead_code)]
impl NativeGrantedTransaction {
    /// Called only after an external publisher has successfully installed the
    /// grant while the legacy lease remains held. In this phase only tests
    /// construct the wrapper, using synthetic grant files.
    fn from_prepared(
        mut prepared: NativeHandoffPreparation,
        record: NativeOwnershipRecord,
    ) -> Result<Self, String> {
        // This constructor models a grant that an external publisher may
        // already have made visible. On *any* validation/transition failure,
        // dropping `prepared` must not implicitly resume the legacy renderer.
        // Unlike publish_isolated, this function cannot prove that no grant
        // exists when an error is returned.
        let outcome = (|| -> Result<(), String> {
            record.validate()?;
            if prepared.transaction.0 != record.token
                || prepared.plasma.pid != record.pid
                || prepared.plasma.start_ticks != record.start_ticks
            {
                return Err("Granted transaction identity does not match preparation".into());
            }
            prepared.ownership.native_granted(&prepared.transaction, &prepared.plasma)
                .map_err(|e| format!("Cannot record native grant: {e:?}"))?;
            Ok(())
        })();
        if let Err(error) = outcome {
            prepared.lease.inhibit_automatic_resume();
            log_warning("Cannot adopt externally granted native transaction; preserving legacy pause");
            return Err(error);
        }
        let NativeHandoffPreparation { ownership, transaction, plasma, lease } = prepared;
        Ok(Self { ownership, transaction, plasma, record, journal_published: false, lease: Some(lease) })
    }

    /// Production-capable publication, deliberately not wired into the runtime
    /// coordinator yet. Consumes the acknowledged pause lease and retains its
    /// exclusive transaction lock. On any uncertain publication outcome, the
    /// pause remains blocking evidence rather than being automatically resumed.
    /// The caller must have completed native presentation preflight before
    /// acquiring this preparation and must separately verify native activation.
    pub(crate) fn publish_production(
        mut prepared: NativeHandoffPreparation,
        mut current_plasma: impl FnMut() -> Result<crate::manage_wallpaper_ownership::PlasmaInstance, String>,
    ) -> Result<Self, String> {
        let expected = prepared.plasma.clone();
        let record = NativeOwnershipRecord {
            token: prepared.transaction.0.clone(),
            pid: expected.pid,
            start_ticks: expected.start_ticks,
        };
        let publication = (|| -> Result<(), String> {
            record.validate()?;
            if current_plasma()? != expected {
                return Err("Plasma process instance changed before production handoff".into());
            }
            prepared.ownership.native_granted(&prepared.transaction, &expected)
                .map_err(|e| format!("Cannot model native grant: {e:?}"))?;
            publish_handoff_journal_locked(&HandoffJournal {
                pause_token: prepared.pause_token().to_owned(),
                native: record.clone(),
            })?;
            publish_native_grant_locked(
                &required_path(NATIVE_GRANT_FILE)?, &record, &expected, &mut current_plasma,
            )
        })();
        if let Err(error) = publication {
            // Even a journal-only failure must not open the legacy renderer
            // until absence of both durable records has been established.
            let journal_absent = matches!(read_handoff_journal(), Ok(None));
            let grant_absent = matches!(native_grant_presence(), Ok(false));
            if !(journal_absent && grant_absent) {
                prepared.lease.inhibit_automatic_resume();
                log_warning("Production handoff publication uncertain; preserving legacy pause");
            }
            return Err(error);
        }
        let NativeHandoffPreparation { ownership, transaction, plasma, lease } = prepared;
        Ok(Self { ownership, transaction, plasma, record, journal_published: true, lease: Some(lease) })
    }

    /// Isolated end-to-end grant path. This deliberately cannot write to the
    /// live runtime. Validate and model ownership BEFORE making the grant
    /// visible; once published, never let an error resume the legacy lease.
    fn publish_isolated(
        mut prepared: NativeHandoffPreparation,
        record: NativeOwnershipRecord,
        mut current_plasma: impl FnMut() -> Result<crate::manage_wallpaper_ownership::PlasmaInstance, String>,
    ) -> Result<Self, String> {
        let expected = prepared.plasma.clone();
        // Every fallible operation in the grant path must use the same
        // fail-closed cleanup. An early identity/transition error is not
        // evidence that a competing grant is absent.
        let publication = (|| -> Result<(), String> {
            if record.token != prepared.transaction.0
                || record.pid != expected.pid || record.start_ticks != expected.start_ticks {
                return Err("Grant record does not match prepared transaction".into());
            }
            record.validate()?;
            // All fallible state transitions precede grant publication.
            prepared.ownership.native_granted(&prepared.transaction, &expected)
                .map_err(|e| format!("Cannot model native grant: {e:?}"))?;
            // Publish the durable correlation BEFORE the grant can become
            // visible. A crash here leaves a journal that blocks new leases.
            publish_isolated_handoff_journal(&HandoffJournal {
                pause_token: prepared.pause_token().to_owned(),
                native: record.clone(),
            })?;
            let grant = required_path(NATIVE_GRANT_FILE)?;
            publish_test_native_grant(&grant, &record, &expected, &mut current_plasma)
        })();
        if let Err(error) = publication {
            // A competing grant may exist even when failure occurred before
            // our publisher ran. Only verified absence permits auto-resume.
            // The transaction lock remains held through this decision.
            let journal_outstanding = !matches!(read_handoff_journal(), Ok(None));
            match native_grant_presence() {
                Ok(false) if !journal_outstanding => {},
                _ => {
                    prepared.lease.inhibit_automatic_resume();
                    log_warning("Native grant or journal unresolved; preserving legacy pause");
                }
            }
            return Err(error);
        }
        // Publication has succeeded. Do not perform fallible operations while
        // moving the retained lease into the granted transaction.
        let NativeHandoffPreparation { ownership, transaction, plasma, lease } = prepared;
        Ok(Self { ownership, transaction, plasma, record, journal_published: true, lease: Some(lease) })
    }

    /// Production phase-one revocation: withdraw this transaction's exact
    /// native grant while keeping the acknowledged legacy pause and journal.
    ///
    /// This is intentionally NOT recovery or legacy reauthorization. The
    /// native host must subsequently publish its matching GL-submission ACK,
    /// and the supervisor must separately complete the renderer latch and
    /// compositor-lifecycle checks before any legacy frame can resume.
    /// Caller must retain this transaction and its exclusive pause lease.
    pub(crate) fn withdraw_production_grant(
        &mut self,
        mut current_plasma: impl FnMut() -> Result<crate::manage_wallpaper_ownership::PlasmaInstance, String>,
    ) -> Result<(), String> {
        if !self.journal_published || self.lease.is_none() {
            return Err("Production withdrawal requires a retained journal and pause lease".into());
        }
        if current_plasma()? != self.plasma {
            return Err("Plasma identity changed; cannot verify native revocation through the live host".into());
        }
        let journal_path = required_path(HANDOFF_JOURNAL_FILE)?;
        let journal_metadata = fs::symlink_metadata(&journal_path)
            .map_err(|e| format!("Cannot inspect retained handoff journal: {e}"))?;
        if !journal_metadata.file_type().is_file() {
            return Err("Handoff journal is not a regular file".into());
        }
        let journal = HandoffJournal::decode(&fs::read(&journal_path)
            .map_err(|e| format!("Cannot read retained handoff journal: {e}"))?)?;
        if journal.native != self.record || self.lease.as_ref()
            .is_none_or(|lease| lease.transaction_token() != journal.pause_token)
        {
            return Err("Refusing withdrawal: handoff journal identity changed".into());
        }
        let pause_path = required_path(PAUSE_REQUEST_FILE)?;
        let pause_metadata = fs::symlink_metadata(&pause_path)
            .map_err(|e| format!("Cannot inspect retained legacy pause: {e}"))?;
        if !pause_metadata.file_type().is_file()
            || fs::read(&pause_path).map_err(|e| e.to_string())?
                != format!("{}\n", journal.pause_token).as_bytes()
        {
            return Err("Refusing withdrawal: legacy pause is not the retained transaction".into());
        }
        let grant_path = required_path(NATIVE_GRANT_FILE)?;
        let grant_metadata = fs::symlink_metadata(&grant_path)
            .map_err(|e| format!("Cannot inspect native grant before withdrawal: {e}"))?;
        if !grant_metadata.file_type().is_file() {
            return Err("Refusing withdrawal of non-regular native grant".into());
        }
        let bytes = fs::read(&grant_path)
            .map_err(|e| format!("Cannot read native grant before withdrawal: {e}"))?;
        if NativeOwnershipRecord::decode(&bytes)? != self.record {
            return Err("Refusing withdrawal of another native transaction".into());
        }
        if current_plasma()? != self.plasma {
            return Err("Plasma identity changed during grant withdrawal".into());
        }
        // Cooperating publishers must hold the same transaction lock; this
        // method retains it through the unlink and directory sync. The
        // journal and legacy pause deliberately survive both success and error.
        fs::remove_file(&grant_path)
            .map_err(|e| format!("Cannot withdraw native grant: {e}"))?;
        let parent = grant_path.parent().ok_or("Native grant has no parent directory")?;
        File::open(parent).and_then(|directory| directory.sync_all())
            .map_err(|e| format!("Cannot sync withdrawn native grant directory: {e}"))?;
        Ok(())
    }

    /// Phase 05: verify a production native GL-submission revocation ACK.
    /// This is deliberately read-only and does NOT release the legacy pause,
    /// retire the journal, clear the renderer latch, or claim compositor
    /// quiescence. The caller retains this transaction and its exclusive lock.
    pub(crate) fn verify_production_revocation_ack(
        &self,
        mut current_plasma: impl FnMut() -> Result<crate::manage_wallpaper_ownership::PlasmaInstance, String>,
    ) -> Result<(), String> {
        if !self.journal_published || self.lease.is_none() {
            return Err("Production revocation verification requires a retained journal and pause lease".into());
        }
        if current_plasma()? != self.plasma {
            return Err("Plasma identity changed before native revocation verification".into());
        }
        let journal_path = required_path(HANDOFF_JOURNAL_FILE)?;
        let journal_meta = fs::symlink_metadata(&journal_path)
            .map_err(|e| format!("Cannot inspect retained handoff journal: {e}"))?;
        if !journal_meta.file_type().is_file() {
            return Err("Retained handoff journal is not a regular file".into());
        }
        let journal = HandoffJournal::decode(&fs::read(&journal_path)
            .map_err(|e| format!("Cannot read retained handoff journal: {e}"))?)?;
        if journal.native != self.record || self.lease.as_ref()
            .is_none_or(|lease| lease.transaction_token() != journal.pause_token)
        {
            return Err("Native revocation journal does not match retained transaction".into());
        }
        let pause_path = required_path(PAUSE_REQUEST_FILE)?;
        let pause_meta = fs::symlink_metadata(&pause_path)
            .map_err(|e| format!("Cannot inspect retained legacy pause: {e}"))?;
        if !pause_meta.file_type().is_file()
            || fs::read(&pause_path).map_err(|e| e.to_string())?
                != format!("{}\n", journal.pause_token).as_bytes()
        {
            return Err("Native revocation verification requires the original legacy pause".into());
        }
        verify_native_revocation(
            &required_path(NATIVE_GRANT_FILE)?,
            &required_path(NATIVE_REVOKED_FILE)?,
            &self.record,
        )?;
        if current_plasma()? != self.plasma {
            return Err("Plasma identity changed during native revocation verification".into());
        }
        // Recheck the ACK and absent grant after the final identity probe.
        verify_native_revocation(
            &required_path(NATIVE_GRANT_FILE)?,
            &required_path(NATIVE_REVOKED_FILE)?,
            &self.record,
        )
    }

    /// Phase 06: transaction-scoped production recovery checkpoint.
    ///
    /// Combines native GL revocation verification with the existing legacy
    /// reauthorization evidence checks, while retaining the exclusive pause
    /// lease and durable handoff journal. This is a READ-ONLY checkpoint,
    /// not a release authorization: compositor presentation retirement and
    /// the renderer latch transition have not been proven in production.
    /// Consequently this method cannot resume the legacy renderer.
    fn verify_production_reauthorization_checkpoint(
        &self,
        mut current_plasma: impl FnMut() -> Result<crate::manage_wallpaper_ownership::PlasmaInstance, String>,
    ) -> Result<(), String> {
        self.verify_production_revocation_ack(|| current_plasma())?;
        let lease = self.lease.as_ref()
            .ok_or("Production recovery checkpoint requires a retained legacy lease")?;
        let expected = HandoffJournal {
            pause_token: lease.transaction_token().to_owned(),
            native: self.record.clone(),
        };
        if expected.native.token != self.transaction.0
            || expected.native.pid != self.plasma.pid
            || expected.native.start_ticks != self.plasma.start_ticks
        {
            return Err("Production recovery transaction identity mismatch".into());
        }
        verify_legacy_reauthorization_evidence(
            &expected,
            &required_path(HANDOFF_JOURNAL_FILE)?,
            &required_path(PAUSE_REQUEST_FILE)?,
            &required_path(NATIVE_GRANT_FILE)?,
            &required_path(NATIVE_REVOKED_FILE)?,
            |pid| {
                let observed = current_plasma()?;
                if observed.pid != pid { return Ok(None); }
                Ok(Some(observed.start_ticks))
            },
        )?;
        // The final identity probe and revocation recheck occur while the
        // lease still holds the exclusive transaction lock.
        self.verify_production_revocation_ack(|| current_plasma())
    }

    /// Withdraw only our own isolated grant. A changed or malformed grant is
    /// never removed, and a missing ACK leaves the legacy pause in place.
    fn revoke_isolated(&mut self) -> Result<(), String> {
        let root = TEST_RUNTIME_DIRECTORY.get()
            .ok_or("Isolated revocation unavailable outside protocol tests")?;
        let grant = root.join(NATIVE_GRANT_FILE);
        let ack = root.join(NATIVE_REVOKED_FILE);
        match fs::symlink_metadata(&grant) {
            Ok(metadata) => {
                if !metadata.file_type().is_file() {
                    return Err("Native grant is not a regular file".into());
                }
                let contents = fs::read(&grant).map_err(|e| e.to_string())?;
                if NativeOwnershipRecord::decode(&contents)? != self.record {
                    return Err("Refusing to withdraw another native transaction".into());
                }
                fs::remove_file(&grant).map_err(|e| e.to_string())?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {},
            Err(e) => return Err(format!("Cannot inspect native grant: {e}")),
        }
        self.recover(&grant, &ack)
    }

    /// The caller withdraws the grant first. A verified matching ACK plus
    /// absent grant permits releasing the legacy pause. No compositor-present
    /// guarantee is implied by the native ACK.
    fn recover(
        &mut self,
        grant_path: &std::path::Path,
        ack_path: &std::path::Path,
    ) -> Result<(), String> {
        use crate::manage_wallpaper_ownership::OwnershipState;
        if !matches!(self.ownership.state(), OwnershipState::RecoveryPending { .. }) {
            self.ownership.begin_recovery(&self.transaction)
                .map_err(|e| format!("Cannot begin native recovery: {e:?}"))?;
        }
        verify_native_revocation(grant_path, ack_path, &self.record)?;
        if self.journal_published {
            let journal = read_handoff_journal()?
                .ok_or("Missing handoff journal during isolated recovery")?;
            if journal.native != self.record || self.lease.as_ref()
                .is_none_or(|lease| lease.transaction_token() != journal.pause_token)
            {
                return Err("Handoff journal does not match recovering transaction".into());
            }
        }
        // Complete the fallible ownership-state transition BEFORE retiring
        // durable evidence. If this fails, the journal and pause survive.
        self.ownership.native_revoked(&self.transaction, &self.plasma)
            .map_err(|e| format!("Cannot record native revocation: {e:?}"))?;
        if self.journal_published {
            // Only the isolated test coordinator may retire its own journal.
            // A removal failure must retain the lease and prevent resume.
            remove_marker(HANDOFF_JOURNAL_FILE)?;
        }
        if let Some(lease) = self.lease.take() {
            lease.release_after_native_revocation();
        }
        Ok(())
    }
}

impl Drop for NativeGrantedTransaction {
    fn drop(&mut self) {
        if let Some(mut lease) = self.lease.take() {
            // An outstanding or uncertain grant must not trigger the ordinary
            // WallpaperPauseGuard destructor's legacy resume.
            lease.inhibit_automatic_resume();
            log_warning("Native grant not verified revoked; leaving legacy pause request in place");
        }
    }
}

impl Drop for WallpaperPauseGuard {
    fn drop(&mut self) {
        if let Some(token) = self.token.take() {
            resume_and_wait_for_frame(&token);
        }
    }
}

/// Read-only native grant inspection for supervisor lifecycle diagnostics.
/// Presence is NOT evidence of a valid grant or a running native renderer.
/// Absence is NOT proof that Plasma has stopped presenting cached frames.
pub(crate) fn native_grant_presence() -> Result<bool, String> {
    let path = required_path(NATIVE_GRANT_FILE)?;
    match fs::symlink_metadata(&path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("Cannot inspect native grant {}: {error}", path.display())),
    }
}

/// Read-only classification for supervisor lifecycle decisions. Never removes a
/// grant or treats a stale process as proof of native GL/presentation quiescence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NativeGrantAssessment {
    Absent,
    CurrentPlasma { pid: u32, start_ticks: u64 },
    StalePlasma { pid: u32, start_ticks: u64 },
    Unverified(String),
}

/// Read /proc/<pid>/stat starttime (field 22). The comm field may contain
/// spaces and parentheses; parse only after its final closing parenthesis.
fn process_start_ticks(pid: u32) -> Result<Option<u64>, String> {
    let path = PathBuf::from(format!("/proc/{pid}/stat"));
    let contents = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Cannot read {}: {error}", path.display())),
    };
    let closing = contents.rfind(')').ok_or("Malformed process stat: missing comm delimiter")?;
    let tail = contents.get(closing + 1..).ok_or("Malformed process stat")?;
    // Tail begins at field 3 (state), so field 22 is zero-based index 19.
    let ticks = tail.split_whitespace().nth(19)
        .ok_or("Malformed process stat: missing starttime")?
        .parse::<u64>().map_err(|e| format!("Invalid process starttime: {e}"))?;
    Ok(Some(ticks))
}

/// Inspect only. A matching process instance is not a native frame-ready ACK.
pub(crate) fn assess_native_grant() -> NativeGrantAssessment {
    let path = match required_path(NATIVE_GRANT_FILE) {
        Ok(path) => path,
        Err(error) => return NativeGrantAssessment::Unverified(error),
    };
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return NativeGrantAssessment::Absent,
        Err(error) => return NativeGrantAssessment::Unverified(format!("Cannot stat grant: {error}")),
    };
    if !metadata.file_type().is_file() {
        return NativeGrantAssessment::Unverified("Grant is not a regular file".into());
    }
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => return NativeGrantAssessment::Unverified(format!("Cannot read grant: {error}")),
    };
    let record = match NativeOwnershipRecord::decode(&bytes) {
        Ok(record) => record,
        Err(error) => return NativeGrantAssessment::Unverified(format!("Invalid grant: {error}")),
    };
    match process_start_ticks(record.pid) {
        Ok(Some(ticks)) if ticks == record.start_ticks => NativeGrantAssessment::CurrentPlasma {
            pid: record.pid, start_ticks: record.start_ticks,
        },
        Ok(_) => NativeGrantAssessment::StalePlasma {
            pid: record.pid, start_ticks: record.start_ticks,
        },
        Err(error) => NativeGrantAssessment::Unverified(error),
    }
}


// Phase 7K-3: native GL activation evidence. Read-only and deliberately not
// wired into the live coordinator. A matching ACK means a Rust GL render call
// returned successfully under this grant; it is NOT a compositor-present ACK.
const NATIVE_ACTIVATION_PREFIX: &str = "screenshaver-plasma-activation-";
const NATIVE_ACTIVATION_SUFFIX: &str = ".ack";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NativeActivationAssessment {
    Absent,
    GlSubmitted { matching_nodes: usize },
    Unverified(String),
}

fn inspect_native_activation(
    expected: &NativeOwnershipRecord,
) -> Result<NativeActivationAssessment, String> {
    expected.validate()?;
    // Check both the live process identity and the exact, regular-file grant.
    // An ACK alone never establishes ownership.
    if process_start_ticks(expected.pid)? != Some(expected.start_ticks) {
        return Err("Expected Plasma process instance is no longer live".into());
    }
    let grant_path = required_path(NATIVE_GRANT_FILE)?;
    let grant_metadata = fs::symlink_metadata(&grant_path)
        .map_err(|e| format!("Cannot inspect native activation grant: {e}"))?;
    if !grant_metadata.file_type().is_file() {
        return Err("Native activation grant is not a regular file".into());
    }
    if NativeOwnershipRecord::decode(&fs::read(&grant_path)
        .map_err(|e| format!("Cannot read native activation grant: {e}"))?)? != *expected {
        return Err("Native activation grant does not match expected transaction".into());
    }

    let mut matching_nodes = 0usize;
    let prefix = format!("{NATIVE_ACTIVATION_PREFIX}{}-", expected.pid);
    for entry in fs::read_dir(runtime_directory()?)
        .map_err(|e| format!("Cannot inspect native activation directory: {e}"))? {
        let entry = entry.map_err(|e| format!("Cannot inspect native activation entry: {e}"))?;
        let name = entry.file_name();
        let name = name.to_str().ok_or("Non-UTF8 native activation filename")?;
        if !name.starts_with(NATIVE_ACTIVATION_PREFIX) || !name.ends_with(NATIVE_ACTIVATION_SUFFIX) {
            continue;
        }
        // Only the expected Plasma PID is eligible. Foreign-process evidence
        // is not evidence for this transaction.
        if !name.starts_with(&prefix) { continue; }
        let node = &name[prefix.len()..name.len() - NATIVE_ACTIVATION_SUFFIX.len()];
        if node.is_empty() || !node.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
            return Err("Malformed native activation node identifier".into());
        }
        if !entry.metadata().map_err(|e| e.to_string())?.is_file() ||
            !fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?.file_type().is_file() {
            return Err("Native activation ACK is not a regular file".into());
        }
        let record = NativeOwnershipRecord::decode(&fs::read(entry.path())
            .map_err(|e| format!("Cannot read native activation ACK: {e}"))?)?;
        if record != *expected {
            return Err("Native activation ACK belongs to another transaction".into());
        }
        matching_nodes += 1;
    }
    // Recheck the grant after scanning, to reject a withdrawn/replaced grant.
    if !fs::symlink_metadata(&grant_path).map_err(|e| e.to_string())?.file_type().is_file() ||
        NativeOwnershipRecord::decode(&fs::read(&grant_path).map_err(|e| e.to_string())?)? != *expected {
        return Err("Native activation grant changed during verification".into());
    }
    Ok(if matching_nodes == 0 { NativeActivationAssessment::Absent }
       else { NativeActivationAssessment::GlSubmitted { matching_nodes } })
}

/// Observation only: never publishes a grant, resumes legacy rendering, or
/// interprets successful GL submission as compositor presentation.
#[allow(dead_code)]
pub(crate) fn assess_native_activation(
    token: &str, plasma: &crate::manage_wallpaper_ownership::PlasmaInstance,
) -> NativeActivationAssessment {
    let expected = NativeOwnershipRecord {
        token: token.to_owned(), pid: plasma.pid, start_ticks: plasma.start_ticks,
    };
    match inspect_native_activation(&expected) {
        Ok(result) => result,
        Err(error) => NativeActivationAssessment::Unverified(error),
    }
}

// Phase 6Y: read-only Qt Quick presentation inventory. The native QML item
// publishes these diagnostic records; they are NEVER authorization or a
// native-frame/compositor-ready acknowledgment.
const PRESENTATION_PREFIX: &str = "screenshaver-plasma-presentation.";
const PRESENTATION_SUFFIX: &str = ".status";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativePresentationObservation {
    pub(crate) pid: u32,
    pub(crate) start_ticks: u64,
    pub(crate) instance: String,
    pub(crate) window_associated: bool,
    pub(crate) node_ever_constructed: bool,
    pub(crate) monitor_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NativePresentationAssessment {
    Absent,
    Observed(Vec<NativePresentationObservation>),
    Unverified(String),
}

/// Inventory all live wallpaper-mode observations, never selecting an
/// arbitrary "first" item. Reject stale, malformed, or suspicious entries
/// instead of treating their absence as a readiness signal.
pub(crate) fn assess_native_presentations() -> NativePresentationAssessment {
    match inspect_native_presentations() {
        Ok(items) if items.is_empty() => NativePresentationAssessment::Absent,
        Ok(items) => NativePresentationAssessment::Observed(items),
        Err(error) => NativePresentationAssessment::Unverified(error),
    }
}

fn inspect_native_presentations() -> Result<Vec<NativePresentationObservation>, String> {
    let directory = runtime_directory()?;
    let mut observations = Vec::new();
    let entries = fs::read_dir(&directory)
        .map_err(|e| format!("Cannot list native presentation status directory: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Cannot inspect presentation directory entry: {e}"))?;
        let name = entry.file_name();
        let name = name.to_str().ok_or("Non-UTF8 runtime directory entry")?;
        if !name.starts_with(PRESENTATION_PREFIX) || !name.ends_with(PRESENTATION_SUFFIX) {
            continue;
        }
        let identity = &name[PRESENTATION_PREFIX.len()..name.len()-PRESENTATION_SUFFIX.len()];
        let parts: Vec<_> = identity.split('.').collect();
        if parts.len() != 3 || parts[2].is_empty()
            || !parts[2].bytes().all(|b| b.is_ascii_hexdigit())
            || parts[2].bytes().any(|b| b.is_ascii_uppercase()) {
            return Err(format!("Invalid native presentation filename: {name}"));
        }
        let pid: u32 = parts[0].parse().map_err(|_| format!("Invalid presentation PID: {name}"))?;
        let ticks: u64 = parts[1].parse().map_err(|_| format!("Invalid presentation start ticks: {name}"))?;
        if pid == 0 || ticks == 0 || parts[0] != pid.to_string() || parts[1] != ticks.to_string() {
            return Err(format!("Noncanonical native presentation identity: {name}"));
        }
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|e| format!("Cannot stat native presentation record {name}: {e}"))?;
        if !metadata.file_type().is_file() || metadata.len() > 256 {
            return Err(format!("Native presentation record is not a small regular file: {name}"));
        }
        let bytes = fs::read(&path)
            .map_err(|e| format!("Cannot read native presentation record {name}: {e}"))?;
        if bytes.len() > 256 {
            return Err(format!("Oversized native presentation record: {name}"));
        }
        let content = std::str::from_utf8(&bytes)
            .map_err(|_| format!("Non-UTF8 native presentation record: {name}"))?;
        let fields: Vec<_> = content.trim_end_matches('\n').split(' ').collect();
        if !content.ends_with('\n') || content.ends_with("\n\n") || fields.len() != 7
            || fields[0] != "v1" || fields[1] != parts[0] || fields[2] != parts[1]
            || (fields[3] != "wallpaper" && fields[3] != "screensaver")
            || (fields[4] != "window=associated" && fields[4] != "window=absent")
            || (fields[5] != "monitor_only=0" && fields[5] != "monitor_only=1")
            || (fields[6] != "node_ever_constructed=0" && fields[6] != "node_ever_constructed=1")
            || (fields[5] == "monitor_only=1" && fields[6] != "node_ever_constructed=0") {
            return Err(format!("Malformed or mismatched native presentation record: {name}"));
        }
        // A status file from a dead Plasma instance is stale, not evidence
        // that native GL has stopped. Do not delete it automatically.
        let current_ticks = process_start_ticks(pid)?;
        if current_ticks != Some(ticks) {
            // A verified nonmatching process generation cannot supply a live
            // wallpaper monitor. Preserve its record for forensic recovery,
            // but do not let it poison discovery of the replacement Plasma.
            // process_start_ticks errors still fail closed above.
            continue;
        }
        if fields[3] == "wallpaper" {
            observations.push(NativePresentationObservation {
                pid, start_ticks: ticks, instance: parts[2].to_owned(),
                window_associated: fields[4] == "window=associated",
                node_ever_constructed: fields[6] == "node_ever_constructed=1",
                monitor_only: fields[5] == "monitor_only=1",
            });
        }
    }
    observations.sort_by(|a,b| (a.pid,a.start_ticks,&a.instance).cmp(&(b.pid,b.start_ticks,&b.instance)));
    Ok(observations)
}

// Phase 6Z: correlate presentation observations with the *exact* process
// instance selected by the handoff. This is diagnostic evidence only; it is
// neither a native readiness ACK nor permission to publish a grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NativePresentationCorrelation {
    Absent,
    WrongPlasma { observed_instances: usize },
    MatchingPlasma {
        instances: usize,
        window_associated: usize,
        node_ever_constructed: usize,
        window_and_node: usize,
        associated_monitors: usize,
    },
    Unverified(String),
}

pub(crate) fn correlate_native_presentations(
    expected: &crate::manage_wallpaper_ownership::PlasmaInstance,
) -> NativePresentationCorrelation {
    correlate_presentation_inventory(expected, assess_native_presentations())
}

fn correlate_presentation_inventory(
    expected: &crate::manage_wallpaper_ownership::PlasmaInstance,
    assessment: NativePresentationAssessment,
) -> NativePresentationCorrelation {
    if expected.pid == 0 || expected.start_ticks == 0 {
        return NativePresentationCorrelation::Unverified(
            "Invalid expected Plasma process identity".into());
    }
    match assessment {
        NativePresentationAssessment::Absent => NativePresentationCorrelation::Absent,
        NativePresentationAssessment::Unverified(error) =>
            NativePresentationCorrelation::Unverified(error),
        NativePresentationAssessment::Observed(items) => {
            // Do not select the first item: Plasma may have multiple displays.
            // All matching items are retained in the aggregate counts.
            let matching: Vec<_> = items.iter().filter(|item|
                item.pid == expected.pid && item.start_ticks == expected.start_ticks
            ).collect();
            if matching.is_empty() {
                return NativePresentationCorrelation::WrongPlasma {
                    observed_instances: items.len(),
                };
            }
            NativePresentationCorrelation::MatchingPlasma {
                instances: matching.len(),
                window_associated: matching.iter().filter(|i| i.window_associated).count(),
                node_ever_constructed: matching.iter().filter(|i| i.node_ever_constructed).count(),
                window_and_node: matching.iter().filter(|i|
                    i.window_associated && i.node_ever_constructed && !i.monitor_only
                ).count(),
                associated_monitors: matching.iter().filter(|i|
                    i.window_associated && i.monitor_only && !i.node_ever_constructed
                ).count(),
            }
        }
    }
}

// Phase 7B: a read-only preflight decision. This must never be interpreted as
// ownership authorization, a successful GL frame, or compositor presentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NativePresentationPreflight {
    Candidate { matching_instances: usize, associated_nodes: usize },
    NotReady(String),
    Unverified(String),
}

#[allow(dead_code)]
pub(crate) fn assess_native_handoff_preflight(
    expected: &crate::manage_wallpaper_ownership::PlasmaInstance,
) -> NativePresentationPreflight {
    classify_native_handoff_preflight(correlate_native_presentations(expected))
}

fn require_preflight_candidate(result: NativePresentationPreflight) -> Result<(), String> {
    match result {
        NativePresentationPreflight::Candidate { associated_nodes, .. } if associated_nodes > 0 => Ok(()),
        NativePresentationPreflight::Candidate { .. } => Err("Native presentation candidate has no associated ownership monitor".into()),
        NativePresentationPreflight::NotReady(reason) => Err(format!("Native presentation not ready: {reason}")),
        NativePresentationPreflight::Unverified(reason) => Err(format!("Native presentation unverified: {reason}")),
    }
}

fn classify_native_handoff_preflight(
    correlation: NativePresentationCorrelation,
) -> NativePresentationPreflight {
    match correlation {
        NativePresentationCorrelation::Absent =>
            NativePresentationPreflight::NotReady("No native wallpaper item observed".into()),
        NativePresentationCorrelation::WrongPlasma { observed_instances } =>
            NativePresentationPreflight::NotReady(format!(
                "No item belongs to the expected Plasma process ({observed_instances} other instances)")),
        NativePresentationCorrelation::MatchingPlasma {
            instances, associated_monitors, ..
        } if associated_monitors > 0 => NativePresentationPreflight::Candidate {
            matching_instances: instances,
            associated_nodes: associated_monitors,
        },
        NativePresentationCorrelation::MatchingPlasma { instances, .. } =>
            NativePresentationPreflight::NotReady(format!(
                "{instances} matching item(s), but none has an associated non-rendering ownership monitor")),
        NativePresentationCorrelation::Unverified(error) =>
            NativePresentationPreflight::Unverified(error),
    }
}

/// Read-only crash-recovery inventory. This deliberately never deletes a
/// marker or releases a pause: a missing grant is not a revocation ACK.
/// The coordinator may use this to report what evidence is still missing.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum AbandonedHandoffAssessment {
    Clean,
    UnresolvedPause { token: String, native: NativeGrantAssessment },
    OrphanedNativeGrant { native: NativeGrantAssessment },
    Unverified(String),
}

#[allow(dead_code)]
pub(crate) fn assess_abandoned_handoff() -> AbandonedHandoffAssessment {
    // This function is observational only. In particular, no missing marker
    // is treated as proof of a successful compositor-visible transition.
    let pause = match read_marker(PAUSE_REQUEST_FILE) {
        Ok(value) => value,
        Err(error) => return AbandonedHandoffAssessment::Unverified(error),
    };
    // If a journal exists, the recorded legacy token must agree with the
    // actual pause marker. Never classify contradictory evidence as clean.
    let journal = match read_handoff_journal() {
        Ok(value) => value,
        Err(error) => return AbandonedHandoffAssessment::Unverified(error),
    };
    if let Some(journal) = &journal {
        if pause.as_deref() != Some(journal.pause_token.as_str()) {
            return AbandonedHandoffAssessment::Unverified(
                "Handoff journal does not match legacy pause request".into());
        }
        let grant_path = match required_path(NATIVE_GRANT_FILE) {
            Ok(path) => path,
            Err(error) => return AbandonedHandoffAssessment::Unverified(error),
        };
        match fs::symlink_metadata(&grant_path) {
            Ok(metadata) if !metadata.file_type().is_file() => return
                AbandonedHandoffAssessment::Unverified("Native grant is not a regular file".into()),
            Ok(_) => match fs::read(&grant_path).map_err(|e| e.to_string())
                .and_then(|bytes| NativeOwnershipRecord::decode(&bytes)) {
                Ok(record) if record == journal.native => {},
                Ok(_) => return AbandonedHandoffAssessment::Unverified(
                    "Native grant does not match handoff journal".into()),
                Err(error) => return AbandonedHandoffAssessment::Unverified(error),
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
            Err(error) => return AbandonedHandoffAssessment::Unverified(error.to_string()),
        }
    }
    let native = assess_native_grant();
    if let NativeGrantAssessment::Unverified(error) = &native {
        return AbandonedHandoffAssessment::Unverified(error.clone());
    }
    match (pause, native) {
        (None, NativeGrantAssessment::Absent) => AbandonedHandoffAssessment::Clean,
        (Some(token), native) => {
            if token.is_empty() {
                AbandonedHandoffAssessment::Unverified("Empty legacy pause token".into())
            } else {
                AbandonedHandoffAssessment::UnresolvedPause { token, native }
            }
        }
        (None, native) => AbandonedHandoffAssessment::OrphanedNativeGrant { native },
    }
}

// A Plasma generation change is a prerequisite for recovery, never a
// compositor-retirement certificate. Keep the decision pure so the existing
// isolated protocol harness can exercise it without restarting Plasma.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecordedPlasmaGeneration {
    StillLive,
    Absent,
    Replaced,
}

impl RecordedPlasmaGeneration {
    fn classify(recorded_start_ticks: u64, observed: Option<u64>) -> Self {
        match observed {
            Some(ticks) if ticks == recorded_start_ticks => Self::StillLive,
            Some(_) => Self::Replaced,
            None => Self::Absent,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::StillLive => "matching_live_plasma",
            Self::Absent => "recorded_plasma_absent",
            Self::Replaced => "recorded_plasma_identity_changed",
        }
    }

    fn old_process_retired(self) -> bool {
        !matches!(self, Self::StillLive)
    }
}

// Recovery eligibility is deliberately separate from release authorization.
// A complete set of process/ACK prerequisites still cannot establish that
// KWin has retired the old wallpaper presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoveryPrerequisite {
    OriginalPlasmaStillLive,
    NativeRevocationUnverified,
    NativeGrantStillPresent,
    AwaitingCompositorRetirement,
}

impl RecoveryPrerequisite {
    fn label(self) -> &'static str {
        match self {
            Self::OriginalPlasmaStillLive => "original_plasma_still_live",
            Self::NativeRevocationUnverified => "native_revocation_unverified",
            Self::NativeGrantStillPresent => "native_grant_still_present",
            Self::AwaitingCompositorRetirement => "awaiting_compositor_retirement",
        }
    }
}

fn recovery_prerequisite(
    generation: RecordedPlasmaGeneration,
    revocation_verified: bool,
    grant_present: bool,
) -> RecoveryPrerequisite {
    if !revocation_verified {
        RecoveryPrerequisite::NativeRevocationUnverified
    } else if grant_present {
        RecoveryPrerequisite::NativeGrantStillPresent
    } else if !generation.old_process_retired() {
        RecoveryPrerequisite::OriginalPlasmaStillLive
    } else {
        RecoveryPrerequisite::AwaitingCompositorRetirement
    }
}

// Batch 7F-D: transaction-scoped classification of durable retirement
// evidence.  This is NOT compositor quiescence or permission to resume.
fn classify_retirement_commit(
    expected: &HandoffJournal,
    committed: Option<&HandoffJournal>,
) -> &'static str {
    match committed {
        None => "absent",
        Some(actual) if actual == expected => "exact_transaction_match_observation_only",
        Some(_) => "different_transaction_blocking",
    }
}

// Batch 7F-E: explicit fail-closed retirement gate.  A transaction-matched
// commit and native revocation are necessary, but not sufficient: KWin may
// still be presenting a previously submitted frame.  Until the compositor
// supplies a verified quiescence certificate for this exact transaction,
// automatic retirement must remain unavailable.  This pure predicate keeps
// the boundary independently testable and prevents accidental promotion of
// the single-node observation into a compositor-wide release certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProductionRetirementGate {
    MissingOrMismatchedCommit,
    NativeRevocationUnverified,
    CompositorQuiescenceUnverified,
}

impl ProductionRetirementGate {
    fn label(self) -> &'static str {
        match self {
            Self::MissingOrMismatchedCommit => "missing_or_mismatched_retirement_commit",
            Self::NativeRevocationUnverified => "native_revocation_unverified",
            Self::CompositorQuiescenceUnverified => "compositor_quiescence_unverified",
        }
    }
}

fn production_retirement_gate(
    expected: &HandoffJournal,
    committed: Option<&HandoffJournal>,
    revocation_verified: bool,
) -> ProductionRetirementGate {
    if committed != Some(expected) {
        ProductionRetirementGate::MissingOrMismatchedCommit
    } else if !revocation_verified {
        ProductionRetirementGate::NativeRevocationUnverified
    } else {
        // There is no trusted compositor-quiescence certificate in the
        // current Rust/C++ protocol.  No marker deletion or latch release.
        ProductionRetirementGate::CompositorQuiescenceUnverified
    }
}

/// Production read-only assessment of an interrupted native wallpaper handoff.
/// This is diagnostic evidence, NEVER a release authorization. In particular,
/// native revocation ACK and absence of a grant do not prove scene retirement.
pub(crate) fn assess_production_handoff_recovery() -> String {
    fn inspect() -> Result<String, String> {
        // Do not create the lock file during a read-only assessment.
        let lock_path = required_path(TRANSACTION_LOCK_FILE)?;
        let metadata = fs::symlink_metadata(&lock_path)
            .map_err(|e| format!("Cannot inspect transaction lock: {e}"))?;
        if !metadata.file_type().is_file() {
            return Err("Transaction lock path is not a regular file".into());
        }
        let lock = OpenOptions::new().read(true).open(&lock_path)
            .map_err(|e| format!("Cannot open transaction lock: {e}"))?;
        lock.try_lock_exclusive()
            .map_err(|e| format!("Transaction currently owned or unavailable: {e}"))?;
        // The lock remains held throughout the inventory, including the
        // second check of the correlated revocation evidence.
        let journal = read_handoff_journal()?
            .ok_or("No handoff journal; cannot assess this recovery transaction")?;
        let pause = read_marker(PAUSE_REQUEST_FILE)?;
        if pause.as_deref() != Some(journal.pause_token.as_str()) {
            return Err("Pause token differs from durable handoff journal".into());
        }
        let plasma = process_start_ticks(journal.native.pid)?;
        let generation = RecordedPlasmaGeneration::classify(
            journal.native.start_ticks, plasma);
        let identity = generation.label();
        let grant = native_grant_presence()?;
        let revocation_result = verify_native_revocation(
            &required_path(NATIVE_GRANT_FILE)?,
            &required_path(NATIVE_REVOKED_FILE)?,
            &journal.native,
        );
        let revocation_verified = revocation_result.is_ok();
        let revocation = match revocation_result {
            Ok(()) => "matching_revocation_ack_and_grant_absent".to_owned(),
            Err(e) => format!("unverified: {e}"),
        };
        let prerequisite = recovery_prerequisite(generation, revocation_verified, grant);
        let presentation = assess_native_presentations();
        let retirement = read_retirement_commit()?;
        // This is an observation only. A missing record is not proof that
        // the compositor retired its last frame, and a matching scene record
        // describes only one node, not every monitor or KWin presentation.
        let scene_retirement = match required_path(NATIVE_SCENE_RETIREMENT_FILE)
            .and_then(|path| {
                let meta = match fs::symlink_metadata(&path) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound =>
                        return Ok("absent".to_owned()),
                    Err(e) => return Err(format!("scene observation metadata: {e}")),
                    Ok(meta) => meta,
                };
                if !meta.file_type().is_file() || meta.len() > 256 {
                    return Err("scene observation is not a bounded regular file".into());
                }
                let bytes = fs::read(&path).map_err(|e| e.to_string())?;
                let record = NativeSceneRetirementObservation::decode(&bytes)?;
                if fs::read(&path).map_err(|e| e.to_string())? != bytes {
                    return Err("scene observation changed during inspection".into());
                }
                if record.native != journal.native {
                    return Err("scene observation belongs to another transaction".into());
                }
                Ok(format!("matching_single_node_observation(node_id={})", record.node_id))
            }) {
            Ok(value) => value,
            Err(error) => format!("unverified({error})"),
        };
        // Batch 7F-D: correlate durable retirement evidence with the *exact*
        // handoff generation.  A commit from a previous generation is not a
        // retirement decision for the current journal, even if its pause token
        // happens to match.  This assessment is strictly read-only.
        let retirement_correlation = classify_retirement_commit(
            &journal, retirement.as_ref());
        let retirement_gate = production_retirement_gate(
            &journal, retirement.as_ref(), revocation_verified);
        // Recheck the journal and pause after gathering the evidence.  The
        // transaction lock is held, but Plasma may publish independent scene
        // observations concurrently; no observation authorizes rendering.
        if read_handoff_journal()?.as_ref() != Some(&journal)
            || read_marker(PAUSE_REQUEST_FILE)?.as_deref()
                != Some(journal.pause_token.as_str()) {
            return Err("Handoff transaction changed during retirement assessment".into());
        }
        // A matching ACK alone is intentionally insufficient to authorize
        // legacy frames or marker retirement.
        Ok(format!(
            "pause_token={}; native_token={}; plasma_pid={}; plasma_start_ticks={}; plasma_identity={}; old_process_retired={}; grant_present={}; revocation={}; native_presentations={presentation:?}; scene_retirement={scene_retirement}; retirement_commit_present={}; retirement_commit_correlation={}; retirement_gate={}; compositor_quiescence=unverified; recovery_prerequisite={}; decision=inspection_only_no_resume_no_marker_retirement",
            journal.pause_token, journal.native.token, journal.native.pid,
            journal.native.start_ticks, identity, generation.old_process_retired(), grant, revocation,
            retirement.is_some(), retirement_correlation, retirement_gate.label(), prerequisite.label(),
        ))
    }
    match inspect() {
        Ok(details) => format!("[WALLPAPER RECOVERY ASSESSMENT] {details}"),
        Err(error) => format!("[WALLPAPER RECOVERY ASSESSMENT] unverified={error}; decision=blocked_no_mutation"),
    }
}

/// Fast fail-closed presence gate used at renderer checkpoints. A grant
/// appearing during a running session must never be interpreted as safe.
/// This is not a transaction lock or an atomic compositor handoff.
pub(crate) fn legacy_frame_gate() -> Result<(), String> {
    // A journal can outlive a coordinator even when its native grant has
    // disappeared. Do not let a running legacy renderer infer permission to
    // submit a frame from grant absence alone. The startup supervisor uses
    // the fuller assessment; this is the per-frame fail-closed fence.
    // The durable decision survives marker withdrawal. A malformed decision
    // also fails closed rather than being interpreted as absence.
    if read_retirement_commit()?.is_some() {
        return Err("retirement decision outstanding; legacy frame submission prohibited".into());
    }
    match read_handoff_journal()? {
        Some(_) => return Err(
            "unresolved native handoff journal; legacy frame submission prohibited".into()),
        None => {},
    }
    if native_grant_presence()? {
        Err("native ownership grant exists; legacy frame submission prohibited".into())
    } else {
        Ok(())
    }
}

pub(crate) fn external_pause_requested() -> bool {
    match read_marker(PAUSE_REQUEST_FILE) {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(error) => {
            log_warning(&format!("Cannot inspect external pause request: {error}"));
            true // A broken channel must not be interpreted as permission to render.
        }
    }
}

/// Snapshot the transaction at pause detection, not at acknowledgment time.
/// A late worker must never acknowledge a replacement request.
pub(crate) fn detected_pause_token() -> Result<Option<String>, String> {
    read_marker(PAUSE_REQUEST_FILE)
}

pub(crate) fn acknowledge_paused_for(token: &str) {
    // A legacy pause ACK must never be published while a native ownership
    // grant is already present. This is a fail-closed protocol invariant, not
    // a substitute for the coordinator retaining its pause lease.
    match legacy_frame_gate() {
        Ok(()) => {}
        Err(error) => {
            log_warning(&format!("Refusing legacy pause acknowledgment: {error}"));
            return;
        }
    }
    match read_marker(PAUSE_REQUEST_FILE) {
        Ok(Some(current)) if current == token => {
            // Recheck immediately before publication. The coordinator must
            // still validate the matching ACK and hold the pause lease.
            if let Err(error) = legacy_frame_gate() {
                log_warning(&format!("Refusing legacy pause acknowledgment: {error}"));
                return;
            }
            if let Err(error) = write_marker(PAUSE_ACK_FILE, token) {
                log_warning(&format!("Unable to publish pause acknowledgment: {error}"));
            }
        }
        Ok(_) => log_warning("Refusing stale pause acknowledgment: transaction changed"),
        Err(error) => log_warning(&format!("Cannot verify pause transaction: {error}")),
    }
}

pub(crate) fn acknowledge_resumed_frame_for(token: &str) {
    // A resumed-frame ACK is an ownership assertion. Refuse it whenever
    // native authorization is present or the grant channel is unreadable.
    // This check does not replace the coordinator's transaction fence.
    if let Err(error) = legacy_frame_gate() {
        log_warning(&format!("[KDE RESUME TRACE] event=resume_ack_gate_rejected; token={token}; reason={error}"));
        return;
    }
    // A frame must be associated with the previously observed pause, and the
    // request must have been withdrawn before it counts as resumed.
    match (read_marker(PAUSE_REQUEST_FILE), read_marker(RESUME_TOKEN_FILE)) {
        (Ok(None), Ok(Some(current))) if current == token => {
            // Recheck at the publication boundary, mirroring the pause ACK.
            if let Err(error) = legacy_frame_gate() {
                log_warning(&format!("[KDE RESUME TRACE] event=resume_ack_gate_rejected; token={token}; reason={error}"));
                return;
            }
            match write_marker(RESUME_ACK_FILE, token) {
                Ok(()) => crate::logger::information(&crate::locate_paths::runtime_log_path(), &format!("[KDE RESUME TRACE] event=resume_ack_published; token={token}")),
                Err(error) => log_warning(&format!("[KDE RESUME TRACE] event=resume_ack_write_failed; token={token}; reason={error}")),
            }
        }
        (Ok(pause), Ok(resume)) => log_warning(&format!(
            "[KDE RESUME TRACE] event=resume_ack_rejected; token={token}; pause_request={pause:?}; resume_token={resume:?}"
        )),
        (Err(error), _) | (_, Err(error)) =>
            log_warning(&format!("Cannot verify resume transaction: {error}")),
    }
}

pub(crate) fn set_runtime_active(active: bool) {
    if active {
        for name in [PAUSE_ACK_FILE, RESUME_ACK_FILE, RESUME_TOKEN_FILE] {
            if let Err(e) = remove_marker(name) { log_warning(&format!("Cannot reset {name}: {e}")); }
        }
        if let Err(e) = write_marker(ACTIVE_FILE, &std::process::id().to_string()) {
            log_warning(&format!("Cannot publish wallpaper active state: {e}"));
        }
    } else {
        for name in [ACTIVE_FILE, PAUSE_ACK_FILE, RESUME_ACK_FILE, RESUME_TOKEN_FILE] {
            if let Err(e) = remove_marker(name) { log_warning(&format!("Cannot remove {name}: {e}")); }
        }
    }
}

fn new_token() -> String {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)
        .unwrap_or_default().as_nanos();
    format!("{}-{nanos}-{}", std::process::id(), NEXT_TOKEN.fetch_add(1, Ordering::Relaxed))
}

fn request_pause() -> Result<Option<String>, String> {
    if !runtime_is_active()? { return Ok(None); }
    let token = new_token();
    remove_marker(PAUSE_ACK_FILE)?;
    remove_marker(RESUME_ACK_FILE)?;
    remove_marker(RESUME_TOKEN_FILE)?;
    write_marker(PAUSE_REQUEST_FILE, &token)?;
    let acknowledged = wait_for_token(PAUSE_ACK_FILE, &token);
    if matches!(acknowledged, Ok(true)) { return Ok(Some(token)); }
    // Do not remove a request that another actor has replaced.
    let _ = remove_if_matching(PAUSE_REQUEST_FILE, &token)?;
    if let Err(error) = acknowledged { return Err(error); }
    if !runtime_is_active()? { return Ok(None); }
    let message = format!("Timed out after {} ms waiting for matching wallpaper pause acknowledgment; refusing competing renderer", CONTROL_WAIT.as_millis());
    log_warning(&message);
    Err(message)
}

fn resume_and_wait_for_frame(token: &str) {
    // Phase 6L: the guard's destructor is not authorization to resume.
    // Preserve the acknowledged pause whenever native ownership is present
    // or the grant path cannot be inspected. This check occurs while the
    // caller still holds the wallpaper transaction lock.
    match native_grant_presence() {
        Ok(false) => {},
        Ok(true) => {
            log_warning("Refusing legacy resume: native ownership grant remains present");
            return;
        }
        Err(error) => {
            log_warning(&format!("Refusing legacy resume: cannot verify native grant absence: {error}"));
            return;
        }
    }
    if let Err(error) = write_marker(RESUME_TOKEN_FILE, token) {
        log_warning(&format!("Cannot register resume transaction: {error}"));
        return;
    }

    // Only withdrawal of our own request permits waiting for a resumed frame.
    // An absent or replaced request is not proof of a completed resume.
    match remove_if_matching(PAUSE_REQUEST_FILE, token) {
        Ok(true) => {},
        Ok(false) => {
            log_warning("Pause transaction absent or replaced; refusing resume acknowledgment wait");
            let _ = remove_if_matching(RESUME_TOKEN_FILE, token);
            return;
        }
        Err(error) => {
            log_warning(&format!("Cannot withdraw pause transaction: {error}"));
            let _ = remove_if_matching(RESUME_TOKEN_FILE, token);
            return;
        }
    }

    match runtime_is_active() {
        Ok(true) => match wait_for_token(RESUME_ACK_FILE, token) {
            Ok(true) => {},
            Ok(false) => log_warning("Resume presentation not confirmed within deadline; compositor may be withholding frames"),
            Err(e) => log_warning(&format!("Resume acknowledgment unavailable: {e}")),
        },
        Ok(false) => {},
        Err(e) => log_warning(&format!("Cannot verify wallpaper runtime during resume: {e}")),
    }
    let _ = remove_if_matching(RESUME_TOKEN_FILE, token);
}


fn resume_and_wait_for_frame_verified(token: &str) -> Result<(), String> {
    // Phase 6L: the guard's destructor is not authorization to resume.
    // Preserve the acknowledged pause whenever native ownership is present
    // or the grant path cannot be inspected. This check occurs while the
    // caller still holds the wallpaper transaction lock.
    match native_grant_presence() {
        Ok(false) => {},
        Ok(true) => {
            return Err("Refusing legacy resume: native ownership grant remains present".into());
        }
        Err(error) => {
            return Err(format!("Refusing legacy resume: cannot verify native grant absence: {error}"));
        }
    }
    if let Err(error) = write_marker(RESUME_TOKEN_FILE, token) {
        return Err(format!("Cannot register resume transaction: {error}"));
    }

    // Only withdrawal of our own request permits waiting for a resumed frame.
    // An absent or replaced request is not proof of a completed resume.
    match remove_if_matching(PAUSE_REQUEST_FILE, token) {
        Ok(true) => {},
        Ok(false) => {
            let _ = remove_if_matching(RESUME_TOKEN_FILE, token);
            return Err("Pause transaction absent or replaced; refusing resume acknowledgment wait".into());
        }
        Err(error) => {
            let _ = remove_if_matching(RESUME_TOKEN_FILE, token);
            return Err(format!("Cannot withdraw pause transaction: {error}"));
        }
    }

    let outcome = match runtime_is_active() {
        Ok(true) => match wait_for_token(RESUME_ACK_FILE, token) {
            Ok(true) => Ok(()),
            Ok(false) => Err("Resume presentation not confirmed within 500 ms".into()),
            Err(error) => Err(format!("Resume acknowledgment unavailable: {error}")),
        },
        Ok(false) => Err("Wallpaper runtime became inactive before resume confirmation".into()),
        Err(error) => Err(format!("Cannot verify wallpaper runtime during resume: {error}")),
    };
    // Do not erase the evidence of an unverified resume. A subsequent
    // coordinator must not treat an unacknowledged cancellation as clean.
    if outcome.is_ok() {
        remove_if_matching(RESUME_TOKEN_FILE, token)?;
    }
    outcome
}
fn runtime_is_active() -> Result<bool, String> {
    Ok(required_path(ACTIVE_FILE)?.exists())
}

fn wait_for_token(name: &str, token: &str) -> Result<bool, String> {
    let deadline = Instant::now() + CONTROL_WAIT;
    loop {
        if read_marker(name)?.as_deref() == Some(token) { return Ok(true); }
        if Instant::now() >= deadline { return Ok(false); }
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn required_path(name: &str) -> Result<PathBuf, String> {
    Ok(runtime_directory()?.join(name))
}

fn read_marker(name: &str) -> Result<Option<String>, String> {
    let path = required_path(name)?;
    match fs::read_to_string(&path) {
        Ok(contents) => Ok(Some(contents.trim().to_owned())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Cannot read {}: {error}", path.display())),
    }
}

fn write_marker(name: &str, value: &str) -> Result<(), String> {
    let path = required_path(name)?;
    let sequence = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
    let temp = path.with_extension(format!("tmp-{}-{sequence}", std::process::id()));
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&temp)
            .map_err(|e| e.to_string())?;
        writeln!(file, "{value}").map_err(|e| e.to_string())?;
        file.flush().map_err(|e| e.to_string())?;
        fs::rename(&temp, &path).map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() { let _ = fs::remove_file(&temp); }
    result
}

fn remove_marker(name: &str) -> Result<(), String> {
    let path = required_path(name)?;
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

// true: matching request removed; false: absent or replaced.
// Like the existing implementation, this is a read-then-remove operation,
// not an atomic compare-and-delete across uncooperative writers.
fn remove_if_matching(name: &str, token: &str) -> Result<bool, String> {
    if read_marker(name)?.as_deref() == Some(token) {
        remove_marker(name)?;
        return Ok(true);
    }
    Ok(false)
}

fn control_path(
    name: &str,
) -> Option<PathBuf> {

    match runtime_directory() {
        Ok(runtime_directory) => {
            Some(
                runtime_directory.join(
                    name
                )
            )
        }

        Err(error) => {
            log_warning(
                &error
            );

            None
        }
    }
}


fn runtime_directory() -> Result<PathBuf, String> {
    if let Some(path) = TEST_RUNTIME_DIRECTORY.get() {
        return Ok(path.clone());
    }

    static RUNTIME_DIRECTORY:
        OnceLock<Result<PathBuf, String>> =
            OnceLock::new();

    RUNTIME_DIRECTORY
        .get_or_init(
            resolve_runtime_directory
        )
        .clone()
}


fn resolve_runtime_directory() -> Result<PathBuf, String> {

    if let Some(runtime_directory) =
        std::env::var_os(
            "XDG_RUNTIME_DIR"
        )
    {
        let path =
            PathBuf::from(
                runtime_directory
            );

        if path.is_dir() {
            log_information(
                &format!(
                    "Using XDG runtime directory: {}",
                    path.display(),
                )
            );

            return Ok(
                path
            );
        }

        log_warning(
            &format!(
                "XDG_RUNTIME_DIR does not name an existing directory: {}",
                path.display(),
            )
        );
    }

    let effective_uid =
        unsafe {
            libc::geteuid()
        };

    let fallback =
        PathBuf::from(
            format!(
                "/run/user/{effective_uid}"
            )
        );

    if fallback.is_dir() {
        log_information(
            &format!(
                "XDG_RUNTIME_DIR unavailable; using fallback runtime directory: {}",
                fallback.display(),
            )
        );

        return Ok(
            fallback
        );
    }

    Err(
        format!(
            "Unable to locate a usable wallpaper control runtime directory; checked XDG_RUNTIME_DIR and {}",
            fallback.display(),
        )
    )
}


fn log_information(
    message: &str,
) {

    let logfile =
        crate::locate_paths::runtime_log_path();

    crate::logger::information(
        &logfile,
        &format!(
            "[WALLPAPER_CONTROL] {}",
            message
        ),
    );
}


fn log_warning(
    message: &str,
) {

    let logfile =
        crate::locate_paths::runtime_log_path();

    crate::logger::warning(
        &logfile,
        &format!(
            "[WALLPAPER_CONTROL] {}",
            message
        ),
    );
}


// This developer diagnostic runs in a separate Screenshaver invocation. Its
// OnceLock override is set before any production runtime initialization; no
// environment variables or real runtime markers are touched.
pub(crate) fn run_protocol_tests() -> Result<(), String> {
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    struct TempDirectory(PathBuf);
    impl Drop for TempDirectory {
        fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
    }
    let root = std::env::temp_dir().join(format!(
        "screenshaver-protocol-test-{}-{}", std::process::id(), new_token()
    ));
    fs::create_dir(&root).map_err(|e| format!("Cannot create isolated test directory: {e}"))?;
    let _cleanup = TempDirectory(root.clone());
    TEST_RUNTIME_DIRECTORY.set(root.clone())
        .map_err(|_| "Protocol test directory already initialized".to_string())?;
    println!("[WALLPAPER PROTOCOL TEST] Isolated directory: {}", root.display());

    fn check(label: &str, result: Result<(), String>, failures: &mut Vec<String>) {
        match result {
            Ok(()) => println!("[PASS] {label}"),
            Err(e) => { println!("[FAIL] {label}: {e}"); failures.push(format!("{label}: {e}")); }
        }
    }
    fn ensure(condition: bool, reason: &str) -> Result<(), String> {
        if condition { Ok(()) } else { Err(reason.to_string()) }
    }
    fn reset() -> Result<(), String> {
        // Each protocol case runs in the isolated test directory. Reset both
        // legacy and native state so one case cannot poison the next case.
        // Never use this cleanup outside --test-wallpaper-protocol.
        for name in [ACTIVE_FILE, PAUSE_REQUEST_FILE, PAUSE_ACK_FILE,
                     RESUME_ACK_FILE, RESUME_TOKEN_FILE,
                     NATIVE_GRANT_FILE, NATIVE_REVOKED_FILE,
                     HANDOFF_JOURNAL_FILE] { remove_marker(name)?; }
        Ok(())
    }
    // Worker imitates the wallpaper renderer's token capture, verified pause
    // acknowledgment, and frame-present acknowledgment. It never renders.
    fn worker(stop: Arc<AtomicBool>) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let mut held: Option<String> = None;
            while !stop.load(Ordering::Relaxed) {
                match detected_pause_token() {
                    Ok(Some(token)) => {
                        if held.is_none() {
                            held = Some(token.clone());
                            acknowledge_paused_for(&token);
                        }
                    }
                    Ok(None) => {
                        if let Some(token) = held.take() {
                            acknowledge_resumed_frame_for(&token);
                        }
                    }
                    Err(_) => break,
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        })
    }
    let mut failures = Vec::new();

    check("Plasma generation classification never certifies compositor retirement", (|| {
        let ticks = 2271;
        let live = RecordedPlasmaGeneration::classify(ticks, Some(ticks));
        let gone = RecordedPlasmaGeneration::classify(ticks, None);
        let replaced = RecordedPlasmaGeneration::classify(ticks, Some(ticks + 1));
        ensure(live == RecordedPlasmaGeneration::StillLive && !live.old_process_retired(),
            "Matching live Plasma classified as retired")?;
        ensure(gone == RecordedPlasmaGeneration::Absent && gone.old_process_retired(),
            "Absent Plasma generation not detected")?;
        ensure(replaced == RecordedPlasmaGeneration::Replaced && replaced.old_process_retired(),
            "Replaced Plasma generation not detected")?;
        // This classification is never used as a release authorization.
        ensure(live.label() == "matching_live_plasma"
            && gone.label() == "recorded_plasma_absent"
            && replaced.label() == "recorded_plasma_identity_changed",
            "Generation diagnostic labels changed")
    })(), &mut failures);

    check("Recovery eligibility remains blocked without compositor retirement", (|| {
        use RecoveryPrerequisite as P;
        let live = RecordedPlasmaGeneration::StillLive;
        let gone = RecordedPlasmaGeneration::Absent;
        let replaced = RecordedPlasmaGeneration::Replaced;
        ensure(recovery_prerequisite(live, true, false) == P::OriginalPlasmaStillLive,
            "Live original Plasma incorrectly eligible")?;
        ensure(recovery_prerequisite(gone, false, false) == P::NativeRevocationUnverified,
            "Missing revocation ACK incorrectly eligible")?;
        ensure(recovery_prerequisite(replaced, true, true) == P::NativeGrantStillPresent,
            "Outstanding native grant incorrectly eligible")?;
        ensure(recovery_prerequisite(gone, true, false) == P::AwaitingCompositorRetirement
            && recovery_prerequisite(replaced, true, false) == P::AwaitingCompositorRetirement,
            "Retired Plasma generation must still await compositor retirement")?;
        ensure(P::AwaitingCompositorRetirement.label() == "awaiting_compositor_retirement",
            "Recovery prerequisite label changed")
    })(), &mut failures);

    check("No active wallpaper: no pause requested", (|| {
        reset()?;
        let guard = WallpaperPauseGuard::acquire()?;
        ensure(guard.token.is_none(), "Unexpected pause token")?;
        drop(guard);
        ensure(read_marker(PAUSE_REQUEST_FILE)?.is_none(), "Unexpected pause marker")
    })(), &mut failures);

    check("Stale pause ACK rejected", (|| {
        reset()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        let stale = "old-transaction";
        write_marker(PAUSE_REQUEST_FILE, "current-transaction")?;
        acknowledge_paused_for(stale);
        ensure(read_marker(PAUSE_ACK_FILE)?.is_none(), "Stale ACK was accepted")?;
        reset()
    })(), &mut failures);

    check("Native grant blocks legacy resumed-frame ACK", (|| {
        reset()?;
        let grant = required_path(NATIVE_GRANT_FILE)?;
        let token = "resume-grant-test";
        write_marker(RESUME_TOKEN_FILE, token)?;
        fs::write(&grant, b"unverified native grant")
            .map_err(|e| format!("Cannot create isolated grant: {e}"))?;
        acknowledge_resumed_frame_for(token);
        let rejected = read_marker(RESUME_ACK_FILE)?.is_none();
        fs::remove_file(&grant).map_err(|e| format!("Cannot clean isolated grant: {e}"))?;
        reset()?;
        ensure(rejected, "Legacy resume was acknowledged with native grant present")
    })(), &mut failures);

    check("Guard destructor retains pause when native grant exists", (|| {
        reset()?;
        let grant = required_path(NATIVE_GRANT_FILE)?;
        let token = "guard-grant-test";
        write_marker(PAUSE_REQUEST_FILE, token)?;
        fs::write(&grant, b"unverified native grant")
            .map_err(|e| format!("Cannot create isolated grant: {e}"))?;
        // Exercise the exact destructor path, without starting a renderer.
        let guard = WallpaperPauseGuard {
            token: Some(token.to_string()),
            _transaction_lock: None,
        };
        drop(guard);
        let preserved = read_marker(PAUSE_REQUEST_FILE)?.as_deref() == Some(token);
        let no_resume_token = read_marker(RESUME_TOKEN_FILE)?.is_none();
        fs::remove_file(&grant).map_err(|e| format!("Cannot clean isolated grant: {e}"))?;
        reset()?;
        ensure(preserved, "Guard destructor withdrew pause while native grant existed")?;
        ensure(no_resume_token, "Guard destructor published resume token with native grant")
    })(), &mut failures);

    check("Timeout refuses renderer and withdraws own marker", (|| {
        reset()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        write_marker(PAUSE_ACK_FILE, "stale-ack")?;
        let result = WallpaperPauseGuard::acquire();
        ensure(result.is_err(), "Missing pause ACK incorrectly granted permission")?;
        ensure(read_marker(PAUSE_REQUEST_FILE)?.is_none(), "Timed-out pause marker remains")?;
        reset()
    })(), &mut failures);

    check("Successful transaction and concurrent lock exclusion", (|| {
        reset()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        let stop = Arc::new(AtomicBool::new(false));
        let handle = worker(Arc::clone(&stop));
        let result = (|| {
            let guard = WallpaperPauseGuard::acquire()?;
            ensure(guard.token.is_some(), "No acknowledged transaction")?;
            ensure(WallpaperPauseGuard::acquire().is_err(), "Concurrent requester acquired lock")?;
            drop(guard);
            ensure(read_marker(PAUSE_REQUEST_FILE)?.is_none(), "Pause marker remains after guard drop")
        })();
        stop.store(true, Ordering::Relaxed);
        handle.join().map_err(|_| "Worker panicked".to_string())?;
        reset()?;
        result
    })(), &mut failures);

    check("Abandoned requester lock and marker", (|| {
        reset()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        let lock = OpenOptions::new().read(true).write(true).create(true)
            .open(required_path(TRANSACTION_LOCK_FILE)?).map_err(|e| e.to_string())?;
        lock.try_lock_exclusive().map_err(|e| e.to_string())?;
        write_marker(PAUSE_REQUEST_FILE, "abandoned-token")?;
        drop(lock); // Simulate requester process exit: kernel releases its lock.
        ensure(read_marker(PAUSE_REQUEST_FILE)?.as_deref() == Some("abandoned-token"),
               "Abandoned marker unexpectedly disappeared")?;
        // Lock release does not establish that the abandoned transaction
        // finished. A new requester must neither replace nor remove it.
        let result = WallpaperPauseGuard::acquire();
        ensure(result.is_err(), "Abandoned pause transaction was incorrectly adopted")?;
        ensure(read_marker(PAUSE_REQUEST_FILE)?.as_deref() == Some("abandoned-token"),
               "Abandoned pause marker was replaced or removed")?;
        reset()
    })(), &mut failures);

    check("Legacy frame gate rejects journal without grant", (|| {
        reset()?;
        ensure(legacy_frame_gate().is_ok(), "Clean gate unexpectedly blocked")?;
        let record = NativeOwnershipRecord {
            token: "0123456789abcdef0123456789abcdef".into(),
            pid: std::process::id(),
            start_ticks: process_start_ticks(std::process::id())?
                .ok_or("Test process identity unavailable")?,
        };
        let journal = HandoffJournal {
            pause_token: "legacy-crash-token".into(),
            native: record,
        };
        let path = required_path(HANDOFF_JOURNAL_FILE)?;
        fs::write(&path, journal.encode()?).map_err(|e| e.to_string())?;
        ensure(legacy_frame_gate().is_err(),
            "Journal without native grant did not block legacy rendering")?;
        ensure(path.exists(), "Frame gate unexpectedly retired handoff journal")?;
        fs::write(&path, b"invalid journal").map_err(|e| e.to_string())?;
        ensure(legacy_frame_gate().is_err(),
            "Malformed journal did not block legacy rendering")?;
        fs::remove_file(&path).map_err(|e| e.to_string())?;
        ensure(legacy_frame_gate().is_ok(),
            "Clean gate did not recover after explicit test cleanup")?;
        reset()
    })(), &mut failures);

    check("Read-only recovery inventory preserves abandoned pause", (|| {
        reset()?;
        write_marker(PAUSE_REQUEST_FILE, "abandoned-recovery-token")?;
        let assessment = assess_abandoned_handoff();
        ensure(matches!(assessment,
            AbandonedHandoffAssessment::UnresolvedPause { ref token, native: NativeGrantAssessment::Absent }
            if token == "abandoned-recovery-token"),
            "Abandoned pause not classified as unresolved")?;
        ensure(read_marker(PAUSE_REQUEST_FILE)?.as_deref() == Some("abandoned-recovery-token"),
            "Recovery inventory modified the pause marker")?;
        reset()
    })(), &mut failures);

    check("Read-only recovery inventory detects orphaned grant", (|| {
        reset()?;
        let grant = required_path(NATIVE_GRANT_FILE)?;
        let record = NativeOwnershipRecord {
            token: "0123456789abcdef0123456789abcdef".into(),
            pid: std::process::id(),
            start_ticks: process_start_ticks(std::process::id())?
                .ok_or("Test process identity unavailable")?,
        };
        fs::write(&grant, record.encode()?).map_err(|e| e.to_string())?;
        let assessment = assess_abandoned_handoff();
        ensure(matches!(assessment,
            AbandonedHandoffAssessment::OrphanedNativeGrant {
                native: NativeGrantAssessment::CurrentPlasma { .. }
            }), "Orphaned native grant not detected")?;
        ensure(grant.exists(), "Recovery inventory removed the native grant")?;
        reset()
    })(), &mut failures);

    check("Replaced pause request is not withdrawn or resumed", (|| {
        reset()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        write_marker(PAUSE_REQUEST_FILE, "replacement-token")?;
        ensure(!remove_if_matching(PAUSE_REQUEST_FILE, "old-token")?,
               "Replacement was reported as withdrawn")?;
        ensure(read_marker(PAUSE_REQUEST_FILE)?.as_deref() == Some("replacement-token"),
               "Replacement request was removed")?;
        resume_and_wait_for_frame("old-token");
        ensure(read_marker(PAUSE_REQUEST_FILE)?.as_deref() == Some("replacement-token"),
               "Resume removed replacement request")?;
        ensure(read_marker(RESUME_ACK_FILE)?.is_none(),
               "Resume acknowledgment appeared for replaced request")?;
        ensure(read_marker(RESUME_TOKEN_FILE)?.is_none(),
               "Abandoned resume token was not cleaned up")?;
        reset()
    })(), &mut failures);

    check("Repeated transactions with fresh identities", (|| {
        reset()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        let stop = Arc::new(AtomicBool::new(false));
        let handle = worker(Arc::clone(&stop));
        let result = (|| {
            let first = WallpaperPauseGuard::acquire()?;
            let first_token = first.token.clone().ok_or("Missing first token")?;
            drop(first);
            // Wait for the worker to observe the resumed state before the
            // next request, avoiding a fabricated compositor race.
            std::thread::sleep(Duration::from_millis(10));
            let second = WallpaperPauseGuard::acquire()?;
            ensure(second.token.as_deref() != Some(&first_token), "Token reused")?;
            drop(second);
            Ok(())
        })();
        stop.store(true, Ordering::Relaxed);
        handle.join().map_err(|_| "Worker panicked".to_string())?;
        reset()?;
        result
    })(), &mut failures);

    // Phase 10C-3B-2B: isolated integration of a real acknowledged legacy
    // pause lease with the ownership model. No native grant is issued, and
    // this test never touches the production runtime directory.
    check("Ownership model advances only after confirmed legacy pause lease", (|| {
        use crate::manage_wallpaper_ownership::{
            OwnershipState, PlasmaInstance, TransactionId, WallpaperOwnership,
        };
        reset()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        let stop = Arc::new(AtomicBool::new(false));
        let handle = worker(Arc::clone(&stop));
        let result = (|| {
            let plasma = PlasmaInstance { pid: 1234, start_ticks: 5678 };
            let mut ownership = WallpaperOwnership::new();
            let transaction = TransactionId(new_token());
            ownership.begin_native_handoff(transaction.clone(), plasma)
                .map_err(|e| format!("Cannot begin diagnostic handoff: {e:?}"))?;
            ensure(matches!(ownership.state(), OwnershipState::NativePending {
                legacy_quiesced: false, native_granted: false, ..
            }), "Pending ownership incorrectly indicates quiescence")?;
            let lease = LegacyWallpaperPauseLease::acquire()?;
            ensure(read_marker(PAUSE_REQUEST_FILE)?.as_deref()
                == Some(lease.transaction_token()), "Lease token does not match request")?;
            ensure(read_marker(PAUSE_ACK_FILE)?.as_deref()
                == Some(lease.transaction_token()), "Lease lacks matching acknowledgment")?;
            ownership.legacy_quiesced(&transaction)
                .map_err(|e| format!("Cannot record verified pause: {e:?}"))?;
            ensure(matches!(ownership.state(), OwnershipState::NativePending {
                legacy_quiesced: true, native_granted: false, ..
            }), "Diagnostic must never authorize native rendering")?;
            // Native was never granted in this isolated test. Release the
            // temporary diagnostic lease; no production owner is switched.
            drop(lease);
            ensure(read_marker(PAUSE_REQUEST_FILE)?.is_none(),
                "Diagnostic lease did not withdraw its pause request")?;
            Ok(())
        })();
        stop.store(true, Ordering::Relaxed);
        handle.join().map_err(|_| "Diagnostic worker panicked".to_string())?;
        reset()?;
        result
    })(), &mut failures);

    check("Preflight rejects unready presentation before legacy pause", (|| {
        use crate::manage_wallpaper_ownership::{PlasmaInstance, TransactionId};
        reset()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        let plasma = PlasmaInstance { pid: 1234, start_ticks: 5678 };
        let result = NativeHandoffPreparation::acquire_with_preflight(
            TransactionId(new_token()), plasma.clone(),
            || Ok(plasma.clone()),
            |_| NativePresentationPreflight::NotReady("node absent".into()),
        );
        ensure(result.is_err(), "Unready presentation accepted")?;
        ensure(read_marker(PAUSE_REQUEST_FILE)?.is_none(), "Preflight failure created a pause request")?;
        reset()
    })(), &mut failures);

    check("Preflight withdrawal after ACK cancels ungranted preparation", (|| {
        use crate::manage_wallpaper_ownership::{PlasmaInstance, TransactionId};
        reset()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        let stop = Arc::new(AtomicBool::new(false));
        let handle = worker(Arc::clone(&stop));
        let result = (|| {
            let plasma = PlasmaInstance { pid: 1234, start_ticks: 5678 };
            let mut calls = 0;
            let prepared = NativeHandoffPreparation::acquire_with_preflight(
                TransactionId(new_token()), plasma.clone(),
                || Ok(plasma.clone()),
                |_| {
                    calls += 1;
                    if calls == 1 {
                        NativePresentationPreflight::Candidate { matching_instances: 1, associated_nodes: 1 }
                    } else {
                        NativePresentationPreflight::NotReady("item detached".into())
                    }
                },
            );
            ensure(prepared.is_err(), "Detached presentation accepted")?;
            ensure(calls == 2, "Expected both preflight observations")?;
            ensure(read_marker(PAUSE_REQUEST_FILE)?.is_none(), "Ungranted preparation left pause request")?;
            Ok(())
        })();
        stop.store(true, Ordering::Relaxed);
        handle.join().map_err(|_| "Diagnostic worker panicked".to_string())?;
        reset()?;
        result
    })(), &mut failures);

    check("Preparation rejects stale Plasma identity before pause", (|| {
        use crate::manage_wallpaper_ownership::{PlasmaInstance, TransactionId};
        reset()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        let expected = PlasmaInstance { pid: 1234, start_ticks: 5678 };
        let result = NativeHandoffPreparation::acquire(
            TransactionId(new_token()), expected,
            || Ok(PlasmaInstance { pid: 1234, start_ticks: 9999 }),
        );
        ensure(result.is_err(), "Stale Plasma instance accepted")?;
        ensure(read_marker(PAUSE_REQUEST_FILE)?.is_none(), "Stale identity created pause request")?;
        reset()
    })(), &mut failures);

    check("Preparation rejects unacknowledged legacy pause", (|| {
        use crate::manage_wallpaper_ownership::{PlasmaInstance, TransactionId};
        reset()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        let plasma = PlasmaInstance { pid: 1234, start_ticks: 5678 };
        let result = NativeHandoffPreparation::acquire(
            TransactionId(new_token()), plasma.clone(), || Ok(plasma.clone()),
        );
        ensure(result.is_err(), "Unacknowledged pause accepted")?;
        ensure(read_marker(PAUSE_REQUEST_FILE)?.is_none(), "Timed-out request remains")?;
        reset()
    })(), &mut failures);

    check("Preparation detects Plasma replacement during pause", (|| {
        use crate::manage_wallpaper_ownership::{PlasmaInstance, TransactionId};
        reset()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        let stop = Arc::new(AtomicBool::new(false));
        let handle = worker(Arc::clone(&stop));
        let result = (|| {
            let plasma = PlasmaInstance { pid: 1234, start_ticks: 5678 };
            let mut probes = 0;
            let result = NativeHandoffPreparation::acquire(
                TransactionId(new_token()), plasma.clone(), || {
                    probes += 1;
                    Ok(PlasmaInstance { pid: 1234,
                        start_ticks: if probes == 1 { 5678 } else { 9999 } })
                },
            );
            ensure(result.is_err(), "Replacement Plasma instance accepted")?;
            ensure(read_marker(PAUSE_REQUEST_FILE)?.is_none(), "Lease not released after identity change")
        })();
        stop.store(true, Ordering::Relaxed);
        handle.join().map_err(|_| "Worker panicked".to_string())?;
        reset()?;
        result
    })(), &mut failures);

    check("Preparation holds lease and cancellation never grants native", (|| {
        use crate::manage_wallpaper_ownership::{OwnershipState, PlasmaInstance, TransactionId};
        reset()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        let stop = Arc::new(AtomicBool::new(false));
        let handle = worker(Arc::clone(&stop));
        let result = (|| {
            let plasma = PlasmaInstance { pid: 1234, start_ticks: 5678 };
            let prepared = NativeHandoffPreparation::acquire(
                TransactionId(new_token()), plasma.clone(), || Ok(plasma.clone()),
            )?;
            ensure(matches!(prepared.state(), OwnershipState::NativePending {
                legacy_quiesced: true, native_granted: false, ..
            }), "Preparation unexpectedly granted native rendering")?;
            ensure(read_marker(PAUSE_REQUEST_FILE)?.as_deref() == Some(prepared.pause_token()),
                "Pause lease was not retained")?;
            prepared.cancel();
            ensure(read_marker(PAUSE_REQUEST_FILE)?.is_none(), "Cancellation did not withdraw pause")
        })();
        stop.store(true, Ordering::Relaxed);
        handle.join().map_err(|_| "Worker panicked".to_string())?;
        reset()?;
        result
    })(), &mut failures);

    // Phase 10C-3B-3B-3A: eight native protocol tests use the SAME isolated
    // runtime directory as the twelve existing wallpaper protocol tests.
    fn native_identity() -> NativeOwnershipRecord {
        NativeOwnershipRecord {
            token: "0123456789abcdef0123456789abcdef".into(),
            pid: 1234,
            start_ticks: 5678,
        }
    }
    fn native_paths() -> Result<(PathBuf, PathBuf), String> {
        Ok((required_path(NATIVE_GRANT_FILE)?, required_path(NATIVE_REVOKED_FILE)?))
    }
    fn reset_native() -> Result<(), String> {
        remove_marker(NATIVE_GRANT_FILE)?;
        remove_marker(NATIVE_REVOKED_FILE)?;
        remove_marker(HANDOFF_JOURNAL_FILE)
    }

    check("Native record canonical round trip", (|| {
        reset_native()?;
        let expected = native_identity();
        ensure(NativeOwnershipRecord::decode(expected.encode()?.as_bytes())? == expected,
            "Native record round trip changed identity")
    })(), &mut failures);

    check("Native malformed records rejected", (|| {
        let valid = native_identity().encode()?;
        let malformed = [
            valid.replace("v1 ", "v2 "),
            valid.replace("0123456789abcdef", "0123456789abcdeG"),
            valid.replace(" 1234 ", " 0 "),
            valid.replace(" 5678", " 0005678"),
            valid.replace(" 1234 ", " 1234  "),
            valid.replace(" 1234 ", " 999999999999999999999 "),
            "x".repeat(257),
            valid.trim_end_matches('\n').to_string(),
            format!("{}\n", valid),
            format!(" {}", valid),
            format!("{} \n", valid.trim_end_matches('\n')),
            valid.replace(" 1234 ", "\t1234 "),
            valid.replace(" 1234 ", "\r1234 "),
        ];
        ensure(malformed.iter().all(|record| NativeOwnershipRecord::decode(record.as_bytes()).is_err()),
            "Malformed native record was accepted")
    })(), &mut failures);

    check("Native missing revocation ACK rejected", (|| {
        reset_native()?;
        let (grant, ack) = native_paths()?;
        ensure(verify_native_revocation(&grant, &ack, &native_identity()).is_err(),
            "Missing ACK was accepted")
    })(), &mut failures);

    check("Native matching ACK and absent grant verified", (|| {
        reset_native()?;
        let (grant, ack) = native_paths()?;
        fs::write(&ack, native_identity().encode()?).map_err(|e| e.to_string())?;
        verify_native_revocation(&grant, &ack, &native_identity())?;
        reset_native()
    })(), &mut failures);

    check("Native symlink revocation ACK rejected", (|| {
        use std::os::unix::fs::symlink;
        reset_native()?;
        let (grant, ack) = native_paths()?;
        let target = ack.with_extension("revoked-test-target");
        fs::write(&target, native_identity().encode()?).map_err(|e| e.to_string())?;
        symlink(&target, &ack).map_err(|e| e.to_string())?;
        let denied = verify_native_revocation(&grant, &ack, &native_identity()).is_err();
        fs::remove_file(&ack).map_err(|e| e.to_string())?;
        fs::remove_file(&target).map_err(|e| e.to_string())?;
        reset_native()?;
        ensure(denied, "Symlink revocation ACK was accepted")
    })(), &mut failures);

    check("New legacy pause transaction refuses existing native grant", (|| {
        reset()?;
        reset_native()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        let (grant, _) = native_paths()?;
        fs::write(&grant, native_identity().encode()?).map_err(|e| e.to_string())?;
        let rejected = WallpaperPauseGuard::acquire().is_err();
        let no_request = read_marker(PAUSE_REQUEST_FILE)?.is_none();
        reset_native()?;
        reset()?;
        ensure(rejected, "New legacy pause was permitted with native grant")?;
        ensure(no_request, "Rejected pause acquisition published a pause request")
    })(), &mut failures);

    check("Native present grant prevents legacy resume", (|| {
        reset_native()?;
        let (grant, ack) = native_paths()?;
        let record = native_identity().encode()?;
        fs::write(&ack, &record).map_err(|e| e.to_string())?;
        fs::write(&grant, record).map_err(|e| e.to_string())?;
        let denied = verify_native_revocation(&grant, &ack, &native_identity()).is_err();
        reset_native()?;
        ensure(denied, "Present native grant was accepted")
    })(), &mut failures);

    check("Native stale token ACK rejected", (|| {
        reset_native()?;
        let (grant, ack) = native_paths()?;
        let mut stale = native_identity();
        stale.token = "ffffffffffffffffffffffffffffffff".into();
        fs::write(&ack, stale.encode()?).map_err(|e| e.to_string())?;
        let denied = verify_native_revocation(&grant, &ack, &native_identity()).is_err();
        reset_native()?;
        ensure(denied, "Stale native token was accepted")
    })(), &mut failures);

    check("Native replaced Plasma instance rejected", (|| {
        reset_native()?;
        let (grant, ack) = native_paths()?;
        let mut replaced = native_identity();
        replaced.start_ticks += 1;
        fs::write(&ack, replaced.encode()?).map_err(|e| e.to_string())?;
        let denied = verify_native_revocation(&grant, &ack, &native_identity()).is_err();
        reset_native()?;
        ensure(denied, "Replaced Plasma instance was accepted")
    })(), &mut failures);

    check("Native malformed revocation ACK rejected", (|| {
        reset_native()?;
        let (grant, ack) = native_paths()?;
        fs::write(&ack, "incomplete").map_err(|e| e.to_string())?;
        let denied = verify_native_revocation(&grant, &ack, &native_identity()).is_err();
        reset_native()?;
        ensure(denied, "Malformed native ACK was accepted")
    })(), &mut failures);

    // Phase 10C-3B-3B-3B: exercise the existing ownership model and retained
    // legacy pause lease together. Native grant/ACK files are SYNTHETIC and
    // confined to TEST_RUNTIME_DIRECTORY; no production grant is issued.
    check("Isolated coordinator retains legacy pause through native recovery", (|| {
        use crate::manage_wallpaper_ownership::{OwnershipState, PlasmaInstance, TransactionId};
        reset()?;
        reset_native()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        let stop = Arc::new(AtomicBool::new(false));
        let handle = worker(Arc::clone(&stop));
        let result = (|| {
            let record = native_identity();
            let plasma = PlasmaInstance { pid: record.pid, start_ticks: record.start_ticks };
            let transaction = TransactionId(record.token.clone());
            let mut prepared = NativeHandoffPreparation::acquire(
                transaction.clone(), plasma.clone(), || Ok(plasma.clone()))?;
            prepared.ownership.native_granted(&transaction, &plasma)
                .map_err(|e| format!("Cannot model native grant: {e:?}"))?;
            ensure(matches!(prepared.state(), OwnershipState::NativePending {
                legacy_quiesced: true, native_granted: true, ..
            }), "Modeled native grant not recorded")?;
            ensure(read_marker(PAUSE_REQUEST_FILE)?.as_deref() == Some(prepared.pause_token()),
                "Legacy pause lost after modeled grant")?;
            prepared.ownership.begin_recovery(&transaction)
                .map_err(|e| format!("Cannot begin modeled recovery: {e:?}"))?;
            let (grant, ack) = native_paths()?;
            // The synthetic grant represents an outstanding authorization.
            fs::write(&grant, record.encode()?).map_err(|e| e.to_string())?;
            fs::write(&ack, record.encode()?).map_err(|e| e.to_string())?;
            ensure(verify_native_revocation(&grant, &ack, &record).is_err(),
                "Outstanding grant was treated as revoked")?;
            ensure(read_marker(PAUSE_REQUEST_FILE)?.as_deref() == Some(prepared.pause_token()),
                "Legacy pause lost during incomplete recovery")?;
            remove_marker(NATIVE_GRANT_FILE)?;
            verify_native_revocation(&grant, &ack, &record)?;
            prepared.ownership.native_revoked(&transaction, &plasma)
                .map_err(|e| format!("Cannot finish modeled recovery: {e:?}"))?;
            ensure(matches!(prepared.state(), OwnershipState::LegacyOwned),
                "Ownership model did not return to legacy")?;
            // This explicit release is safe only because the isolated test has
            // verified its synthetic revocation; it is NOT a live handoff.
            prepared.cancel();
            ensure(read_marker(PAUSE_REQUEST_FILE)?.is_none(),
                "Legacy pause not released after isolated recovery")
        })();
        stop.store(true, Ordering::Relaxed);
        handle.join().map_err(|_| "Diagnostic worker panicked".to_string())?;
        reset_native()?;
        reset()?;
        result
    })(), &mut failures);

    check("Isolated coordinator rejects mismatched recovery identity", (|| {
        use crate::manage_wallpaper_ownership::{PlasmaInstance, TransactionId};
        reset()?;
        reset_native()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        let stop = Arc::new(AtomicBool::new(false));
        let handle = worker(Arc::clone(&stop));
        let result = (|| {
            let record = native_identity();
            let plasma = PlasmaInstance { pid: record.pid, start_ticks: record.start_ticks };
            let transaction = TransactionId(record.token.clone());
            let mut prepared = NativeHandoffPreparation::acquire(
                transaction.clone(), plasma.clone(), || Ok(plasma.clone()))?;
            prepared.ownership.native_granted(&transaction, &plasma)
                .map_err(|e| format!("Cannot model native grant: {e:?}"))?;
            prepared.ownership.begin_recovery(&transaction)
                .map_err(|e| format!("Cannot begin recovery: {e:?}"))?;
            let wrong = PlasmaInstance { pid: plasma.pid, start_ticks: plasma.start_ticks + 1 };
            ensure(prepared.ownership.native_revoked(&transaction, &wrong).is_err(),
                "Wrong Plasma instance completed recovery")?;
            ensure(read_marker(PAUSE_REQUEST_FILE)?.as_deref() == Some(prepared.pause_token()),
                "Legacy pause lost after invalid recovery")?;
            // Diagnostic never published a real grant. Cancel is safe here.
            prepared.cancel();
            Ok(())
        })();
        stop.store(true, Ordering::Relaxed);
        handle.join().map_err(|_| "Diagnostic worker panicked".to_string())?;
        reset_native()?;
        reset()?;
        result
    })(), &mut failures);

    // Phase 10C-3B-3B-3C: destructor and recovery safety with synthetic grants.
    check("Granted transaction drop preserves legacy pause", (|| {
        use crate::manage_wallpaper_ownership::{PlasmaInstance, TransactionId};
        reset()?; reset_native()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        let stop = Arc::new(AtomicBool::new(false));
        let handle = worker(Arc::clone(&stop));
        let result = (|| {
            let record = native_identity();
            let plasma = PlasmaInstance { pid: record.pid, start_ticks: record.start_ticks };
            let prepared = NativeHandoffPreparation::acquire(
                TransactionId(record.token.clone()), plasma.clone(), || Ok(plasma.clone()))?;
            let token = prepared.pause_token().to_string();
            let granted = NativeGrantedTransaction::from_prepared(prepared, record.clone())?;
            let (grant, _) = native_paths()?;
            fs::write(&grant, record.encode()?).map_err(|e| e.to_string())?;
            drop(granted);
            ensure(read_marker(PAUSE_REQUEST_FILE)?.as_deref() == Some(token.as_str()),
                "Granted transaction drop resumed legacy without revocation")?;
            ensure(read_marker(RESUME_TOKEN_FILE)?.is_none(),
                "Granted transaction drop published a resume token")
        })();
        stop.store(true, Ordering::Relaxed);
        handle.join().map_err(|_| "Diagnostic worker panicked".to_string())?;
        reset_native()?; reset()?; result
    })(), &mut failures);

    check("Failed native revocation leaves legacy paused", (|| {
        use crate::manage_wallpaper_ownership::{PlasmaInstance, TransactionId};
        reset()?; reset_native()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        let stop = Arc::new(AtomicBool::new(false));
        let handle = worker(Arc::clone(&stop));
        let result = (|| {
            let record = native_identity();
            let plasma = PlasmaInstance { pid: record.pid, start_ticks: record.start_ticks };
            let prepared = NativeHandoffPreparation::acquire(
                TransactionId(record.token.clone()), plasma.clone(), || Ok(plasma.clone()))?;
            let token = prepared.pause_token().to_string();
            let mut granted = NativeGrantedTransaction::from_prepared(prepared, record.clone())?;
            let (grant, ack) = native_paths()?;
            fs::write(&grant, record.encode()?).map_err(|e| e.to_string())?;
            fs::write(&ack, record.encode()?).map_err(|e| e.to_string())?;
            ensure(granted.recover(&grant, &ack).is_err(),
                "Outstanding grant was accepted as revoked")?;
            drop(granted);
            ensure(read_marker(PAUSE_REQUEST_FILE)?.as_deref() == Some(token.as_str()),
                "Failed revocation resumed legacy")
        })();
        stop.store(true, Ordering::Relaxed);
        handle.join().map_err(|_| "Diagnostic worker panicked".to_string())?;
        reset_native()?; reset()?; result
    })(), &mut failures);

    check("Verified native revocation releases legacy pause", (|| {
        use crate::manage_wallpaper_ownership::{PlasmaInstance, TransactionId};
        reset()?; reset_native()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        let stop = Arc::new(AtomicBool::new(false));
        let handle = worker(Arc::clone(&stop));
        let result = (|| {
            let record = native_identity();
            let plasma = PlasmaInstance { pid: record.pid, start_ticks: record.start_ticks };
            let prepared = NativeHandoffPreparation::acquire(
                TransactionId(record.token.clone()), plasma.clone(), || Ok(plasma.clone()))?;
            let mut granted = NativeGrantedTransaction::from_prepared(prepared, record.clone())?;
            let (grant, ack) = native_paths()?;
            fs::write(&grant, record.encode()?).map_err(|e| e.to_string())?;
            remove_marker(NATIVE_GRANT_FILE)?;
            fs::write(&ack, record.encode()?).map_err(|e| e.to_string())?;
            granted.recover(&grant, &ack)?;
            ensure(read_marker(PAUSE_REQUEST_FILE)?.is_none(),
                "Verified revocation did not release legacy pause")?;
            drop(granted);
            Ok(())
        })();
        stop.store(true, Ordering::Relaxed);
        handle.join().map_err(|_| "Diagnostic worker panicked".to_string())?;
        reset_native()?; reset()?; result
    })(), &mut failures);

    check("Externally granted transaction adoption failure preserves pause", (|| {
        use crate::manage_wallpaper_ownership::{PlasmaInstance, TransactionId};
        reset()?; reset_native()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        let stop = Arc::new(AtomicBool::new(false));
        let handle = worker(Arc::clone(&stop));
        let result = (|| {
            let record = native_identity();
            let plasma = PlasmaInstance { pid: record.pid, start_ticks: record.start_ticks };
            let prepared = NativeHandoffPreparation::acquire(
                TransactionId(record.token.clone()), plasma.clone(), || Ok(plasma.clone()))?;
            let token = prepared.pause_token().to_string();
            let mut mismatched = record.clone();
            mismatched.start_ticks += 1;
            ensure(NativeGrantedTransaction::from_prepared(prepared, mismatched).is_err(),
                "Mismatched external grant was adopted")?;
            ensure(read_marker(PAUSE_REQUEST_FILE)?.as_deref() == Some(token.as_str()),
                "Failed external grant adoption resumed legacy")?;
            ensure(read_marker(RESUME_TOKEN_FILE)?.is_none(),
                "Failed external grant adoption published a resume token")
        })();
        stop.store(true, Ordering::Relaxed);
        handle.join().map_err(|_| "Diagnostic worker panicked".to_string())?;
        reset_native()?; reset()?; result
    })(), &mut failures);

    // Phase 10C-3B-3B-3D: isolated publisher preconditions and atomic file output.
    check("Native grant publisher writes canonical record in isolated runtime", (|| {
        use crate::manage_wallpaper_ownership::PlasmaInstance;
        reset_native()?;
        let record = native_identity();
        let plasma = PlasmaInstance { pid: record.pid, start_ticks: record.start_ticks };
        let (grant, _) = native_paths()?;
        publish_test_native_grant(&grant, &record, &plasma, || Ok(plasma.clone()))?;
        ensure(NativeOwnershipRecord::decode(&fs::read(&grant).map_err(|e| e.to_string())?)? == record,
            "Published grant differs from expected record")?;
        reset_native()
    })(), &mut failures);

    check("Native grant publisher rejects replaced Plasma instance", (|| {
        use crate::manage_wallpaper_ownership::PlasmaInstance;
        reset_native()?;
        let record = native_identity();
        let plasma = PlasmaInstance { pid: record.pid, start_ticks: record.start_ticks };
        let replacement = PlasmaInstance { pid: record.pid, start_ticks: record.start_ticks + 1 };
        let (grant, _) = native_paths()?;
        let mut probes = 0;
        ensure(publish_test_native_grant(&grant, &record, &plasma, || {
            probes += 1;
            Ok(if probes == 1 { plasma.clone() } else { replacement.clone() })
        }).is_err(), "Changed Plasma identity was accepted")?;
        ensure(!grant.exists(), "Grant was published after Plasma replacement")?;
        reset_native()
    })(), &mut failures);

    check("Native grant publisher refuses existing grant and outside path", (|| {
        use crate::manage_wallpaper_ownership::PlasmaInstance;
        reset_native()?;
        let record = native_identity();
        let plasma = PlasmaInstance { pid: record.pid, start_ticks: record.start_ticks };
        let (grant, _) = native_paths()?;
        fs::write(&grant, "existing-grant").map_err(|e| e.to_string())?;
        ensure(publish_test_native_grant(&grant, &record, &plasma, || Ok(plasma.clone())).is_err(),
            "Existing grant was overwritten")?;
        ensure(fs::read_to_string(&grant).map_err(|e| e.to_string())? == "existing-grant",
            "Existing grant contents changed")?;
        reset_native()?;
        let outside = std::env::temp_dir().join("screenshaver-grant-outside-test");
        ensure(publish_test_native_grant(&outside, &record, &plasma, || Ok(plasma.clone())).is_err(),
            "Outside path was authorized")?;
        reset_native()
    })(), &mut failures);

    // Consolidated isolated coordinator regression batch. No live Plasma
    // process or production grant path is touched by these tests.
    check("Coordinator publishes only with verified pause", (|| {
        use crate::manage_wallpaper_ownership::{PlasmaInstance, TransactionId};
        reset()?; reset_native()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        let record = native_identity();
        let plasma = PlasmaInstance { pid: record.pid, start_ticks: record.start_ticks };
        let result = NativeHandoffPreparation::acquire(
            TransactionId(record.token), plasma.clone(), || Ok(plasma.clone()));
        ensure(result.is_err(), "Unacknowledged pause was accepted")?;
        ensure(!native_paths()?.0.exists(), "Grant appeared without verified pause")?;
        reset()
    })(), &mut failures);

    check("Coordinator publishes and retains exclusive pause", (|| {
        use crate::manage_wallpaper_ownership::{PlasmaInstance, TransactionId};
        reset()?; reset_native()?; write_marker(ACTIVE_FILE, "test-runtime")?;
        let stop = Arc::new(AtomicBool::new(false));
        let handle = worker(Arc::clone(&stop));
        let result = (|| {
            let record = native_identity();
            let plasma = PlasmaInstance { pid: record.pid, start_ticks: record.start_ticks };
            let prepared = NativeHandoffPreparation::acquire(
                TransactionId(record.token.clone()), plasma.clone(), || Ok(plasma.clone()))?;
            let pause = prepared.pause_token().to_owned();
            let granted = NativeGrantedTransaction::publish_isolated(
                prepared, record.clone(), || Ok(plasma.clone()))?;
            ensure(NativeOwnershipRecord::decode(&fs::read(native_paths()?.0).map_err(|e| e.to_string())?)? == record,
                "Published record mismatch")?;
            ensure(read_handoff_journal()? == Some(HandoffJournal {
                pause_token: pause.clone(), native: record.clone(),
            }), "Published handoff journal does not match paused transaction")?;
            ensure(read_marker(PAUSE_REQUEST_FILE)?.as_deref() == Some(pause.as_str()),
                "Legacy pause lost after publication")?;
            drop(granted);
            ensure(read_marker(PAUSE_REQUEST_FILE)?.as_deref() == Some(pause.as_str()),
                "Dropping published transaction resumed legacy")
        })();
        stop.store(true, Ordering::Relaxed);
        handle.join().map_err(|_| "Worker panicked".to_string())?;
        reset_native()?; reset()?; result
    })(), &mut failures);

    check("Coordinator rejects identity mismatch before grant", (|| {
        use crate::manage_wallpaper_ownership::{PlasmaInstance, TransactionId};
        reset()?; reset_native()?; write_marker(ACTIVE_FILE, "test-runtime")?;
        let stop = Arc::new(AtomicBool::new(false));
        let handle = worker(Arc::clone(&stop));
        let result = (|| {
            let record = native_identity();
            let plasma = PlasmaInstance { pid: record.pid, start_ticks: record.start_ticks };
            let prepared = NativeHandoffPreparation::acquire(
                TransactionId(record.token.clone()), plasma.clone(), || Ok(plasma.clone()))?;
            let mut wrong = record;
            wrong.token = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".into();
            ensure(NativeGrantedTransaction::publish_isolated(prepared, wrong, || Ok(plasma.clone())).is_err(),
                "Mismatched transaction published")?;
            ensure(!native_paths()?.0.exists(), "Mismatched grant exists")?;
            ensure(read_marker(PAUSE_REQUEST_FILE)?.is_none(), "Failed pre-publication transaction retained pause")
        })();
        stop.store(true, Ordering::Relaxed);
        handle.join().map_err(|_| "Worker panicked".to_string())?;
        reset_native()?; reset()?; result
    })(), &mut failures);

    check("Coordinator refuses replaced Plasma during staging", (|| {
        use crate::manage_wallpaper_ownership::{PlasmaInstance, TransactionId};
        reset()?; reset_native()?; write_marker(ACTIVE_FILE, "test-runtime")?;
        let stop = Arc::new(AtomicBool::new(false));
        let handle = worker(Arc::clone(&stop));
        let result = (|| {
            let record = native_identity();
            let plasma = PlasmaInstance { pid: record.pid, start_ticks: record.start_ticks };
            let prepared = NativeHandoffPreparation::acquire(
                TransactionId(record.token.clone()), plasma.clone(), || Ok(plasma.clone()))?;
            let mut probes = 0;
            ensure(NativeGrantedTransaction::publish_isolated(prepared, record, || {
                probes += 1;
                Ok(PlasmaInstance { pid: plasma.pid,
                    start_ticks: plasma.start_ticks + if probes > 1 { 1 } else { 0 } })
            }).is_err(), "Replaced Plasma was granted ownership")?;
            ensure(!native_paths()?.0.exists(), "Grant appeared after Plasma replacement")?;
            // The durable journal was already published before the second
            // identity probe failed. Without a verified native revocation,
            // the only safe outcome is to retain the legacy pause.
            ensure(read_marker(PAUSE_REQUEST_FILE)?.is_some(),
                "Failed staged grant unexpectedly released legacy pause")?;
            ensure(read_handoff_journal()?.is_some(),
                "Failed staged grant lost durable transaction evidence")
        })();
        stop.store(true, Ordering::Relaxed);
        handle.join().map_err(|_| "Worker panicked".to_string())?;
        reset_native()?; reset()?; result
    })(), &mut failures);

    check("Coordinator cannot overwrite competing native grant", (|| {
        use crate::manage_wallpaper_ownership::{PlasmaInstance, TransactionId};
        reset()?; reset_native()?; write_marker(ACTIVE_FILE, "test-runtime")?;
        let stop = Arc::new(AtomicBool::new(false));
        let handle = worker(Arc::clone(&stop));
        let result = (|| {
            let record = native_identity();
            let plasma = PlasmaInstance { pid: record.pid, start_ticks: record.start_ticks };
            let prepared = NativeHandoffPreparation::acquire(
                TransactionId(record.token.clone()), plasma.clone(), || Ok(plasma.clone()))?;
            fs::write(native_paths()?.0, "other-owner").map_err(|e| e.to_string())?;
            ensure(NativeGrantedTransaction::publish_isolated(prepared, record, || Ok(plasma.clone())).is_err(),
                "Existing grant overwritten")?;
            ensure(fs::read_to_string(native_paths()?.0).map_err(|e| e.to_string())? == "other-owner",
                "Competing grant changed")?;
            ensure(read_marker(PAUSE_REQUEST_FILE)?.is_some(),
                "Competing native grant incorrectly resumed legacy wallpaper")
        })();
        stop.store(true, Ordering::Relaxed);
        handle.join().map_err(|_| "Worker panicked".to_string())?;
        reset_native()?; reset()?; result
    })(), &mut failures);

    check("Coordinator refuses recovery without native ACK", (|| {
        use crate::manage_wallpaper_ownership::{PlasmaInstance, TransactionId};
        reset()?; reset_native()?; write_marker(ACTIVE_FILE, "test-runtime")?;
        let stop = Arc::new(AtomicBool::new(false));
        let handle = worker(Arc::clone(&stop));
        let result = (|| {
            let record = native_identity();
            let plasma = PlasmaInstance { pid: record.pid, start_ticks: record.start_ticks };
            let prepared = NativeHandoffPreparation::acquire(
                TransactionId(record.token.clone()), plasma.clone(), || Ok(plasma.clone()))?;
            let pause = prepared.pause_token().to_owned();
            let mut granted = NativeGrantedTransaction::publish_isolated(prepared, record, || Ok(plasma.clone()))?;
            ensure(granted.revoke_isolated().is_err(), "Missing ACK permitted resume")?;
            drop(granted);
            ensure(read_marker(PAUSE_REQUEST_FILE)?.as_deref() == Some(pause.as_str()),
                "Legacy resumed without ACK")
        })();
        stop.store(true, Ordering::Relaxed);
        handle.join().map_err(|_| "Worker panicked".to_string())?;
        reset_native()?; reset()?; result
    })(), &mut failures);

    check("Coordinator rejects stale ACK after grant withdrawal", (|| {
        use crate::manage_wallpaper_ownership::{PlasmaInstance, TransactionId};
        reset()?; reset_native()?; write_marker(ACTIVE_FILE, "test-runtime")?;
        let stop = Arc::new(AtomicBool::new(false));
        let handle = worker(Arc::clone(&stop));
        let result = (|| {
            let record = native_identity();
            let plasma = PlasmaInstance { pid: record.pid, start_ticks: record.start_ticks };
            let prepared = NativeHandoffPreparation::acquire(
                TransactionId(record.token.clone()), plasma.clone(), || Ok(plasma.clone()))?;
            let pause = prepared.pause_token().to_owned();
            let mut granted = NativeGrantedTransaction::publish_isolated(prepared, record.clone(), || Ok(plasma.clone()))?;
            let mut stale = record;
            stale.token = "ffffffffffffffffffffffffffffffff".into();
            fs::write(native_paths()?.1, stale.encode()?).map_err(|e| e.to_string())?;
            ensure(granted.revoke_isolated().is_err(), "Stale ACK accepted")?;
            drop(granted);
            ensure(read_marker(PAUSE_REQUEST_FILE)?.as_deref() == Some(pause.as_str()),
                "Stale ACK resumed legacy")
        })();
        stop.store(true, Ordering::Relaxed);
        handle.join().map_err(|_| "Worker panicked".to_string())?;
        reset_native()?; reset()?; result
    })(), &mut failures);

    check("Production ACK verifier retains pause and rejects missing or stale ACK", (|| {
        use crate::manage_wallpaper_ownership::{PlasmaInstance, TransactionId};
        reset()?; reset_native()?; write_marker(ACTIVE_FILE, "test-runtime")?;
        let stop = Arc::new(AtomicBool::new(false));
        let handle = worker(Arc::clone(&stop));
        let result = (|| {
            let record = native_identity();
            let plasma = PlasmaInstance { pid: record.pid, start_ticks: record.start_ticks };
            let prepared = NativeHandoffPreparation::acquire(
                TransactionId(record.token.clone()), plasma.clone(), || Ok(plasma.clone()))?;
            let pause = prepared.pause_token().to_owned();
            let mut granted = NativeGrantedTransaction::publish_isolated(
                prepared, record.clone(), || Ok(plasma.clone()))?;
            let (grant_path, ack_path) = native_paths()?;
            ensure(granted.verify_production_revocation_ack(|| Ok(plasma.clone())).is_err(),
                "Revocation verifier accepted an outstanding grant")?;
            fs::remove_file(&grant_path).map_err(|e| e.to_string())?;
            ensure(granted.verify_production_revocation_ack(|| Ok(plasma.clone())).is_err(),
                "Revocation verifier accepted missing ACK")?;
            let mut stale = record.clone();
            stale.token = "ffffffffffffffffffffffffffffffff".into();
            fs::write(&ack_path, stale.encode()?).map_err(|e| e.to_string())?;
            ensure(granted.verify_production_revocation_ack(|| Ok(plasma.clone())).is_err(),
                "Revocation verifier accepted stale ACK")?;
            fs::write(&ack_path, record.encode()?).map_err(|e| e.to_string())?;
            ensure(granted.verify_production_revocation_ack(|| Ok(PlasmaInstance {
                pid: plasma.pid, start_ticks: plasma.start_ticks.saturating_add(1),
            })).is_err(), "Revocation verifier accepted changed Plasma identity")?;
            granted.verify_production_revocation_ack(|| Ok(plasma.clone()))?;
            ensure(read_marker(PAUSE_REQUEST_FILE)?.as_deref() == Some(pause.as_str()),
                "Read-only revocation verification released legacy pause")?;
            ensure(read_handoff_journal()?.is_some(),
                "Read-only revocation verification retired journal")?;
            // Complete existing isolated recovery to leave the test harness clean.
            granted.recover(&grant_path, &ack_path)
        })();
        stop.store(true, Ordering::Relaxed);
        handle.join().map_err(|_| "Worker panicked".to_string())?;
        reset_native()?; reset()?; result
    })(), &mut failures);

    check("Production recovery checkpoint preserves pause and rejects changed evidence", (|| {
        use crate::manage_wallpaper_ownership::{PlasmaInstance, TransactionId};
        reset()?; reset_native()?; write_marker(ACTIVE_FILE, "test-runtime")?;
        let stop = Arc::new(AtomicBool::new(false));
        let handle = worker(Arc::clone(&stop));
        let result = (|| {
            let record = native_identity();
            let plasma = PlasmaInstance { pid: record.pid, start_ticks: record.start_ticks };
            let prepared = NativeHandoffPreparation::acquire(
                TransactionId(record.token.clone()), plasma.clone(), || Ok(plasma.clone()))?;
            let pause = prepared.pause_token().to_owned();
            let mut granted = NativeGrantedTransaction::publish_isolated(
                prepared, record.clone(), || Ok(plasma.clone()))?;
            let (grant_path, ack_path) = native_paths()?;
            ensure(granted.verify_production_reauthorization_checkpoint(|| Ok(plasma.clone())).is_err(),
                "Checkpoint accepted outstanding native grant")?;
            fs::remove_file(&grant_path).map_err(|e| e.to_string())?;
            ensure(granted.verify_production_reauthorization_checkpoint(|| Ok(plasma.clone())).is_err(),
                "Checkpoint accepted missing native ACK")?;
            fs::write(&ack_path, record.encode()?).map_err(|e| e.to_string())?;
            ensure(granted.verify_production_reauthorization_checkpoint(|| Ok(PlasmaInstance {
                pid: plasma.pid, start_ticks: plasma.start_ticks.saturating_add(1),
            })).is_err(), "Checkpoint accepted replaced Plasma")?;
            granted.verify_production_reauthorization_checkpoint(|| Ok(plasma.clone()))?;
            ensure(read_marker(PAUSE_REQUEST_FILE)?.as_deref() == Some(pause.as_str()),
                "Checkpoint released legacy pause")?;
            ensure(read_handoff_journal()?.is_some(), "Checkpoint retired handoff journal")?;
            // Corrupt the retained pause and ensure the checkpoint fails closed.
            write_marker(PAUSE_REQUEST_FILE, "different-token")?;
            ensure(granted.verify_production_reauthorization_checkpoint(|| Ok(plasma.clone())).is_err(),
                "Checkpoint accepted replaced pause")?;
            write_marker(PAUSE_REQUEST_FILE, &pause)?;
            granted.verify_production_reauthorization_checkpoint(|| Ok(plasma.clone()))?;
            granted.recover(&grant_path, &ack_path)
        })();
        stop.store(true, Ordering::Relaxed);
        handle.join().map_err(|_| "Worker panicked".to_string())?;
        reset_native()?; reset()?; result
    })(), &mut failures);

    check("Coordinator completes grant revoke and legacy resume", (|| {
        use crate::manage_wallpaper_ownership::{PlasmaInstance, TransactionId, OwnershipState};
        reset()?; reset_native()?; write_marker(ACTIVE_FILE, "test-runtime")?;
        let stop = Arc::new(AtomicBool::new(false));
        let handle = worker(Arc::clone(&stop));
        let result = (|| {
            let record = native_identity();
            let plasma = PlasmaInstance { pid: record.pid, start_ticks: record.start_ticks };
            let prepared = NativeHandoffPreparation::acquire(
                TransactionId(record.token.clone()), plasma.clone(), || Ok(plasma.clone()))?;
            let mut granted = NativeGrantedTransaction::publish_isolated(prepared, record.clone(), || Ok(plasma.clone()))?;
            fs::write(native_paths()?.1, record.encode()?).map_err(|e| e.to_string())?;
            granted.revoke_isolated()?;
            ensure(matches!(granted.ownership.state(), OwnershipState::LegacyOwned),
                "Ownership state not returned to legacy")?;
            ensure(read_marker(PAUSE_REQUEST_FILE)?.is_none(), "Legacy pause not withdrawn")?;
            ensure(!native_paths()?.0.exists(), "Native grant still exists")?;
            ensure(read_handoff_journal()?.is_none(), "Verified recovery retained handoff journal")
        })();
        stop.store(true, Ordering::Relaxed);
        handle.join().map_err(|_| "Worker panicked".to_string())?;
        reset_native()?; reset()?; result
    })(), &mut failures);

    check("Coordinator refuses to remove replaced grant", (|| {
        use crate::manage_wallpaper_ownership::{PlasmaInstance, TransactionId};
        reset()?; reset_native()?; write_marker(ACTIVE_FILE, "test-runtime")?;
        let stop = Arc::new(AtomicBool::new(false));
        let handle = worker(Arc::clone(&stop));
        let result = (|| {
            let record = native_identity();
            let plasma = PlasmaInstance { pid: record.pid, start_ticks: record.start_ticks };
            let prepared = NativeHandoffPreparation::acquire(
                TransactionId(record.token.clone()), plasma.clone(), || Ok(plasma.clone()))?;
            let pause = prepared.pause_token().to_owned();
            let mut granted = NativeGrantedTransaction::publish_isolated(prepared, record, || Ok(plasma.clone()))?;
            fs::write(native_paths()?.0, "other-owner").map_err(|e| e.to_string())?;
            ensure(granted.revoke_isolated().is_err(), "Replaced grant removed")?;
            ensure(fs::read_to_string(native_paths()?.0).map_err(|e| e.to_string())? == "other-owner",
                "Other owner's grant changed")?;
            drop(granted);
            ensure(read_marker(PAUSE_REQUEST_FILE)?.as_deref() == Some(pause.as_str()),
                "Replaced grant resumed legacy")
        })();
        stop.store(true, Ordering::Relaxed);
        handle.join().map_err(|_| "Worker panicked".to_string())?;
        reset_native()?; reset()?; result
    })(), &mut failures);

    check("Legacy frame gate allows absent grant", (|| {
        reset_native()?;
        legacy_frame_gate()
    })(), &mut failures);

    check("Legacy frame gate blocks valid grant", (|| {
        reset_native()?;
        fs::write(native_paths()?.0, native_identity().encode()?).map_err(|e| e.to_string())?;
        ensure(legacy_frame_gate().is_err(), "Present native grant was ignored")?;
        reset_native()
    })(), &mut failures);

    check("Legacy frame gate blocks malformed grant", (|| {
        reset_native()?;
        fs::write(native_paths()?.0, "malformed").map_err(|e| e.to_string())?;
        ensure(legacy_frame_gate().is_err(), "Malformed native grant was ignored")?;
        reset_native()
    })(), &mut failures);

    check("Legacy frame gate blocks stale grant", (|| {
        reset_native()?;
        let mut record = native_identity();
        record.pid = u32::MAX;
        fs::write(native_paths()?.0, record.encode()?).map_err(|e| e.to_string())?;
        ensure(legacy_frame_gate().is_err(), "Stale native grant was ignored")?;
        reset_native()
    })(), &mut failures);

    check("Legacy frame gate blocks symlink grant", (|| {
        reset_native()?;
        let grant = native_paths()?.0;
        std::os::unix::fs::symlink("nonexistent-target", &grant).map_err(|e| e.to_string())?;
        ensure(legacy_frame_gate().is_err(), "Symlink native grant was ignored")?;
        reset_native()
    })(), &mut failures);

    check("Handoff journal canonical round trip", (|| {
        let journal = HandoffJournal {
            pause_token: "pause-123".into(), native: native_identity(),
        };
        let encoded = journal.encode()?;
        ensure(HandoffJournal::decode(encoded.as_bytes())? == journal,
            "Handoff journal round trip failed")?;
        ensure(HandoffJournal::decode(encoded.trim_end().as_bytes()).is_err(),
            "Journal accepted missing newline")?;
        ensure(HandoffJournal::decode(encoded.replace("v1 ", "v2 ").as_bytes()).is_err(),
            "Journal accepted unknown version")?;
        ensure(HandoffJournal::decode(encoded.replace("pause-123", "pause token").as_bytes()).is_err(),
            "Journal accepted embedded whitespace")
    })(), &mut failures);

    check("Handoff journal mismatch preserves pause", (|| {
        reset_native()?; reset()?;
        let journal = HandoffJournal {
            pause_token: "journal-pause".into(), native: native_identity(),
        };
        write_marker(PAUSE_REQUEST_FILE, "other-pause")?;
        fs::write(required_path(HANDOFF_JOURNAL_FILE)?, journal.encode()?)
            .map_err(|e| e.to_string())?;
        ensure(matches!(assess_abandoned_handoff(), AbandonedHandoffAssessment::Unverified(_)),
            "Mismatched journal was not rejected")?;
        ensure(read_marker(PAUSE_REQUEST_FILE)?.as_deref() == Some("other-pause"),
            "Assessment modified pause marker")?;
        ensure(WallpaperPauseGuard::acquire().is_err(),
            "New transaction ignored outstanding journal")?;
        fs::remove_file(required_path(HANDOFF_JOURNAL_FILE)?)
            .map_err(|e| e.to_string())?;
        reset()
    })(), &mut failures);

    check("Handoff journal symlink fails closed", (|| {
        reset_native()?; reset()?;
        let path = required_path(HANDOFF_JOURNAL_FILE)?;
        std::os::unix::fs::symlink("missing-journal", &path)
            .map_err(|e| e.to_string())?;
        ensure(matches!(assess_abandoned_handoff(), AbandonedHandoffAssessment::Unverified(_)),
            "Symlink journal was accepted")?;
        ensure(WallpaperPauseGuard::acquire().is_err(),
            "New transaction ignored symlink journal")?;
        fs::remove_file(path).map_err(|e| e.to_string())
    })(), &mut failures);

    check("Crash boundary: journal published but grant absent", (|| {
        reset()?; reset_native()?;
        let record = native_identity();
        let pause = "crash-after-journal";
        write_marker(PAUSE_REQUEST_FILE, pause)?;
        publish_isolated_handoff_journal(&HandoffJournal {
            pause_token: pause.into(), native: record,
        })?;
        ensure(matches!(assess_abandoned_handoff(),
            AbandonedHandoffAssessment::UnresolvedPause {
                token, native: NativeGrantAssessment::Absent,
            } if token == pause), "Crash after journal was not classified as unresolved")?;
        ensure(WallpaperPauseGuard::acquire().is_err(),
            "New requester acquired abandoned journal transaction")?;
        ensure(read_marker(PAUSE_REQUEST_FILE)?.as_deref() == Some(pause),
            "Crash assessment removed legacy pause")?;
        ensure(read_handoff_journal()?.is_some(),
            "Crash assessment removed handoff journal")?;
        reset_native()?; reset()
    })(), &mut failures);

    check("Crash boundary: journal and grant remain", (|| {
        reset()?; reset_native()?;
        let record = native_identity();
        let pause = "crash-after-grant";
        write_marker(PAUSE_REQUEST_FILE, pause)?;
        publish_isolated_handoff_journal(&HandoffJournal {
            pause_token: pause.into(), native: record.clone(),
        })?;
        // Simulate a crash after publication, not a real compositor handoff.
        fs::write(native_paths()?.0, record.encode()?)
            .map_err(|e| e.to_string())?;
        ensure(legacy_frame_gate().is_err(),
            "Legacy frame gate allowed outstanding grant")?;
        ensure(WallpaperPauseGuard::acquire().is_err(),
            "New requester acquired transaction with outstanding grant")?;
        ensure(read_marker(PAUSE_REQUEST_FILE)?.as_deref() == Some(pause),
            "Crash assessment removed legacy pause")?;
        ensure(read_handoff_journal()?.is_some(),
            "Crash assessment removed handoff journal")?;
        reset_native()?; reset()
    })(), &mut failures);

    check("Crash boundary: grant withdrawn without revocation ACK", (|| {
        reset()?; reset_native()?;
        let record = native_identity();
        let pause = "crash-before-ack";
        write_marker(PAUSE_REQUEST_FILE, pause)?;
        publish_isolated_handoff_journal(&HandoffJournal {
            pause_token: pause.into(), native: record.clone(),
        })?;
        ensure(verify_native_revocation(&native_paths()?.0,
            &native_paths()?.1, &record).is_err(),
            "Missing revocation ACK was accepted")?;
        ensure(WallpaperPauseGuard::acquire().is_err(),
            "Missing revocation ACK allowed new transaction")?;
        ensure(read_marker(PAUSE_REQUEST_FILE)?.as_deref() == Some(pause),
            "Missing revocation ACK removed pause")?;
        reset_native()?; reset()
    })(), &mut failures);

    // Phase 7A: exercise the process-identity correlation without relying on
    // a running KDE instance. These are observations, never ownership grants.
    check("Presentation correlation: absent and invalid expected identity", (|| {
        use crate::manage_wallpaper_ownership::PlasmaInstance;
        let expected = PlasmaInstance { pid: 1234, start_ticks: 5678 };
        ensure(matches!(correlate_presentation_inventory(&expected,
            NativePresentationAssessment::Absent), NativePresentationCorrelation::Absent),
            "Empty inventory was not classified as absent")?;
        let invalid = PlasmaInstance { pid: 0, start_ticks: 5678 };
        ensure(matches!(correlate_presentation_inventory(&invalid,
            NativePresentationAssessment::Absent), NativePresentationCorrelation::Unverified(_)),
            "Invalid expected process identity was accepted")
    })(), &mut failures);

    check("Presentation correlation: wrong Plasma process", (|| {
        use crate::manage_wallpaper_ownership::PlasmaInstance;
        let expected = PlasmaInstance { pid: 1234, start_ticks: 5678 };
        let items = vec![NativePresentationObservation {
            pid: 1234, start_ticks: 9999, instance: "a1".into(),
            window_associated: true, node_ever_constructed: true, monitor_only: false,
        }];
        ensure(matches!(correlate_presentation_inventory(&expected,
            NativePresentationAssessment::Observed(items)),
            NativePresentationCorrelation::WrongPlasma { observed_instances: 1 }),
            "A PID match with different start ticks was accepted")
    })(), &mut failures);

    check("Presentation correlation: aggregate matching displays", (|| {
        use crate::manage_wallpaper_ownership::PlasmaInstance;
        let expected = PlasmaInstance { pid: 1234, start_ticks: 5678 };
        let items = vec![
            NativePresentationObservation { pid: 1234, start_ticks: 5678,
                instance: "a1".into(), window_associated: true,
                node_ever_constructed: true, monitor_only: false },
            NativePresentationObservation { pid: 1234, start_ticks: 5678,
                instance: "a2".into(), window_associated: true,
                node_ever_constructed: false, monitor_only: true },
            NativePresentationObservation { pid: 9999, start_ticks: 1111,
                instance: "a3".into(), window_associated: true,
                node_ever_constructed: true, monitor_only: false },
        ];
        ensure(matches!(correlate_presentation_inventory(&expected,
            NativePresentationAssessment::Observed(items)),
            NativePresentationCorrelation::MatchingPlasma {
                instances: 2, window_associated: 2,
                node_ever_constructed: 1, window_and_node: 1, associated_monitors: 1 }),
            "Multiple matching instances were not counted correctly")
    })(), &mut failures);

    check("Presentation correlation: unverified inventory", (|| {
        use crate::manage_wallpaper_ownership::PlasmaInstance;
        let expected = PlasmaInstance { pid: 1234, start_ticks: 5678 };
        ensure(matches!(correlate_presentation_inventory(&expected,
            NativePresentationAssessment::Unverified("bad record".into())),
            NativePresentationCorrelation::Unverified(_)),
            "Unverified inventory was accepted")
    })(), &mut failures);

    check("Presentation inventory: live record and malformed record", (|| {
        let pid = std::process::id();
        let ticks = process_start_ticks(pid)?
            .ok_or("Cannot inspect test process start ticks")?;
        let name = format!("{PRESENTATION_PREFIX}{pid}.{ticks}.a1{PRESENTATION_SUFFIX}");
        let path = runtime_directory()?.join(name);
        let wire = format!("v1 {pid} {ticks} wallpaper window=associated monitor_only=0 node_ever_constructed=1\n");
        fs::write(&path, &wire).map_err(|e| e.to_string())?;
        let result = (|| {
            let items = inspect_native_presentations()?;
            ensure(items.iter().any(|item| item.pid == pid
                && item.start_ticks == ticks && item.instance == "a1"
                && item.window_associated && item.node_ever_constructed),
                "Valid live wallpaper status not found")?;
            fs::write(&path, format!("v1 {pid} {ticks} wallpaper window=associated monitor_only=0 node_ever_constructed=2\n"))
                .map_err(|e| e.to_string())?;
            ensure(matches!(assess_native_presentations(),
                NativePresentationAssessment::Unverified(_)),
                "Malformed status was accepted")
        })();
        let cleanup = fs::remove_file(&path).map_err(|e| e.to_string());
        result?;
        cleanup
    })(), &mut failures);

    // Phase 7B: a matching process is necessary but not sufficient to attempt
    // native ownership. These checks never publish a grant or change a pause.
    check("Native preflight: no associated ownership monitor", (|| {
        let result = classify_native_handoff_preflight(
            NativePresentationCorrelation::MatchingPlasma {
                instances: 2, window_associated: 1,
                node_ever_constructed: 1, window_and_node: 0, associated_monitors: 0,
            });
        ensure(matches!(result, NativePresentationPreflight::NotReady(_)),
            "Unassociated monitor evidence was accepted")
    })(), &mut failures);

    check("Native preflight: associated ownership monitor is only a candidate", (|| {
        let result = classify_native_handoff_preflight(
            NativePresentationCorrelation::MatchingPlasma {
                instances: 3, window_associated: 2,
                node_ever_constructed: 2, window_and_node: 1, associated_monitors: 1,
            });
        ensure(matches!(result, NativePresentationPreflight::Candidate {
            matching_instances: 3, associated_nodes: 1,
        }), "Matching monitor did not produce candidate preflight")
    })(), &mut failures);

    check("Native preflight: wrong process and unverified evidence rejected", (|| {
        ensure(matches!(classify_native_handoff_preflight(
            NativePresentationCorrelation::WrongPlasma { observed_instances: 2 }),
            NativePresentationPreflight::NotReady(_)),
            "Wrong Plasma instance was accepted")?;
        ensure(matches!(classify_native_handoff_preflight(
            NativePresentationCorrelation::Unverified("malformed".into())),
            NativePresentationPreflight::Unverified(_)),
            "Unverified observation was accepted")
    })(), &mut failures);

    // Phase 7K-3: synthetic activation evidence, isolated from production.
    check("Native activation: matching GL submission and stale evidence rejected", (|| {
        reset_native()?;
        let pid = std::process::id();
        let ticks = process_start_ticks(pid)?.ok_or("Test process has no start ticks")?;
        let record = NativeOwnershipRecord {
            token: "0123456789abcdef0123456789abcdef".into(), pid, start_ticks: ticks,
        };
        let plasma = crate::manage_wallpaper_ownership::PlasmaInstance { pid, start_ticks: ticks };
        let ack = required_path(&format!("{NATIVE_ACTIVATION_PREFIX}{pid}-a1{NATIVE_ACTIVATION_SUFFIX}"))?;
        let result = (|| {
            ensure(matches!(assess_native_activation(&record.token, &plasma),
                NativeActivationAssessment::Unverified(_)), "Missing grant was accepted")?;
            fs::write(required_path(NATIVE_GRANT_FILE)?, record.encode()?).map_err(|e| e.to_string())?;
            ensure(matches!(assess_native_activation(&record.token, &plasma),
                NativeActivationAssessment::Absent), "Missing ACK was accepted")?;
            fs::write(&ack, record.encode()?).map_err(|e| e.to_string())?;
            ensure(matches!(assess_native_activation(&record.token, &plasma),
                NativeActivationAssessment::GlSubmitted { matching_nodes: 1 }), "Matching ACK rejected")?;
            fs::write(&ack, "v1 malformed\n").map_err(|e| e.to_string())?;
            ensure(matches!(assess_native_activation(&record.token, &plasma),
                NativeActivationAssessment::Unverified(_)), "Malformed ACK was accepted")?;
            fs::write(&ack, record.encode()?).map_err(|e| e.to_string())?;
            remove_marker(NATIVE_GRANT_FILE)?;
            ensure(matches!(assess_native_activation(&record.token, &plasma),
                NativeActivationAssessment::Unverified(_)), "Orphaned ACK was accepted")
        })();
        let _ = fs::remove_file(&ack);
        reset_native()?;
        result
    })(), &mut failures);

    // Phase 7K-4A: eligibility never clears the renderer latch or pause.
    check("Legacy reauthorization: matching evidence and fail-closed rejection", (|| {
        reset_native()?;
        let pid = std::process::id();
        let ticks = process_start_ticks(pid)?.ok_or("Missing test process identity")?;
        let expected = HandoffJournal {
            pause_token: "pause-7k4a".into(),
            native: NativeOwnershipRecord {
                token: "abcdef0123456789abcdef0123456789".into(),
                pid, start_ticks: ticks,
            },
        };
        let journal = required_path(HANDOFF_JOURNAL_FILE)?;
        let pause = required_path(PAUSE_REQUEST_FILE)?;
        let grant = required_path(NATIVE_GRANT_FILE)?;
        let revoked = required_path(NATIVE_REVOKED_FILE)?;
        let result = (|| -> Result<(), String> {
            fs::write(&journal, expected.encode()?).map_err(|e| e.to_string())?;
            fs::write(&pause, format!("{}\n", expected.pause_token)).map_err(|e| e.to_string())?;
            fs::write(&revoked, expected.native.encode()?).map_err(|e| e.to_string())?;
            let probe = |_: u32| Ok(Some(ticks));
            ensure(verify_legacy_reauthorization_evidence(
                &expected, &journal, &pause, &grant, &revoked, probe).is_ok(),
                "Matching reauthorization evidence rejected")?;
            fs::write(&grant, expected.native.encode()?).map_err(|e| e.to_string())?;
            ensure(verify_legacy_reauthorization_evidence(
                &expected, &journal, &pause, &grant, &revoked, probe).is_err(),
                "Present native grant accepted")?;
            fs::remove_file(&grant).map_err(|e| e.to_string())?;
            fs::write(&revoked, "v1 malformed\n").map_err(|e| e.to_string())?;
            ensure(verify_legacy_reauthorization_evidence(
                &expected, &journal, &pause, &grant, &revoked, probe).is_err(),
                "Malformed native revocation ACK accepted")?;
            fs::write(&revoked, expected.native.encode()?).map_err(|e| e.to_string())?;
            fs::write(&pause, "wrong-token\n").map_err(|e| e.to_string())?;
            ensure(verify_legacy_reauthorization_evidence(
                &expected, &journal, &pause, &grant, &revoked, probe).is_err(),
                "Mismatched pause token accepted")?;
            fs::write(&pause, format!("{}\n", expected.pause_token)).map_err(|e| e.to_string())?;
            ensure(verify_legacy_reauthorization_evidence(
                &expected, &journal, &pause, &grant, &revoked,
                |_: u32| Ok(Some(ticks + 1))).is_err(),
                "Replaced Plasma identity accepted")?;
            fs::remove_file(&journal).map_err(|e| e.to_string())?;
            ensure(verify_legacy_reauthorization_evidence(
                &expected, &journal, &pause, &grant, &revoked, probe).is_err(),
                "Missing journal accepted")
        })();
        let _ = fs::remove_file(&journal);
        let _ = fs::remove_file(&pause);
        let _ = fs::remove_file(&revoked);
        reset_native()?;
        result
    })(), &mut failures);

    // Phase 7K-4B-1: wire-contract tests only; no live handshake or latch change.
    check("Legacy reauthorization handshake: canonical matching request and ACK", (|| {
        let root = runtime_directory()?;
        let request = root.join(REAUTHORIZE_REQUEST_FILE);
        let ack = root.join(REAUTHORIZE_ACK_FILE);
        let handoff = HandoffJournal {
            pause_token: "pause-7k4b".into(),
            native: NativeOwnershipRecord {
                token: "abcdef0123456789abcdef0123456789".into(),
                pid: std::process::id(),
                start_ticks: process_start_ticks(std::process::id())?
                    .ok_or("Missing test process identity")?,
            },
        };
        let result = (|| -> Result<(), String> {
            let req = ReauthorizationRecord {
                stage: ReauthorizationStage::Request, handoff: handoff.clone(),
            };
            let acknowledged = ReauthorizationRecord {
                stage: ReauthorizationStage::Acknowledged, handoff: handoff.clone(),
            };
            ensure(ReauthorizationRecord::decode(req.encode()?.as_bytes())? == req,
                "Request wire roundtrip failed")?;
            fs::write(&request, req.encode()?).map_err(|e| e.to_string())?;
            fs::write(&ack, acknowledged.encode()?).map_err(|e| e.to_string())?;
            verify_reauthorization_pair(&request, &ack, &handoff)?;
            fs::write(&ack, req.encode()?).map_err(|e| e.to_string())?;
            ensure(verify_reauthorization_pair(&request, &ack, &handoff).is_err(),
                "Request incorrectly accepted as acknowledgment")?;
            fs::write(&ack, acknowledged.encode()?).map_err(|e| e.to_string())?;
            fs::write(&request, "v1 request bad\n").map_err(|e| e.to_string())?;
            ensure(verify_reauthorization_pair(&request, &ack, &handoff).is_err(),
                "Malformed request accepted")?;
            fs::write(&request, req.encode()?).map_err(|e| e.to_string())?;
            let mut other = handoff.clone();
            other.pause_token = "different-pause".into();
            fs::write(&ack, ReauthorizationRecord {
                stage: ReauthorizationStage::Acknowledged, handoff: other,
            }.encode()?).map_err(|e| e.to_string())?;
            ensure(verify_reauthorization_pair(&request, &ack, &handoff).is_err(),
                "Foreign transaction acknowledgment accepted")?;
            Ok(())
        })();
        let _ = fs::remove_file(&request);
        let _ = fs::remove_file(&ack);
        result
    })(), &mut failures);

    // Phase 7K-4B-3: exercise the actual WallpaperRuntimeControl pause
    // checkpoint against isolated on-disk recovery evidence. This is not a
    // compositor/frame test and does not release the native-ownership latch.
    check("Legacy renderer checkpoint: ACK while blocked, fail closed on invalid recovery", (|| {
        reset_native()?;
        remove_marker(REAUTHORIZE_REQUEST_FILE)?;
        remove_marker(REAUTHORIZE_ACK_FILE)?;
        remove_marker(PAUSE_REQUEST_FILE)?;
        let root = runtime_directory()?;
        let request_path = root.join(REAUTHORIZE_REQUEST_FILE);
        let ack_path = root.join(REAUTHORIZE_ACK_FILE);
        let handoff = HandoffJournal {
            pause_token: "pause-7k4b3".into(),
            native: NativeOwnershipRecord {
                token: "abcdef0123456789abcdef0123456789".into(),
                pid: std::process::id(),
                start_ticks: process_start_ticks(std::process::id())?
                    .ok_or("Cannot identify protocol test process")?,
            },
        };
        let result = (|| -> Result<(), String> {
            let renderer = crate::manage_wallpaper_runtime::WallpaperRuntimeControl::new(true, false);
            let request = ReauthorizationRecord {
                stage: ReauthorizationStage::Request, handoff: handoff.clone(),
            };
            fs::write(root.join(HANDOFF_JOURNAL_FILE), handoff.encode()?)
                .map_err(|e| e.to_string())?;
            fs::write(root.join(PAUSE_REQUEST_FILE), format!("{}\n", handoff.pause_token))
                .map_err(|e| e.to_string())?;
            fs::write(root.join(NATIVE_REVOKED_FILE), handoff.native.encode()?)
                .map_err(|e| e.to_string())?;
            fs::write(&request_path, request.encode()?).map_err(|e| e.to_string())?;
            renderer.protocol_test_blocked_reauthorization_checkpoint(&handoff.pause_token)?;
            verify_reauthorization_pair(&request_path, &ack_path, &handoff)?;
            ensure(legacy_frame_gate().is_err(), "Journal no longer blocks legacy frames")?;
            // A second checkpoint is idempotent and must not release ownership.
            renderer.protocol_test_blocked_reauthorization_checkpoint(&handoff.pause_token)?;
            fs::remove_file(&ack_path).map_err(|e| e.to_string())?;
            fs::write(&request_path, "v1 request malformed\n").map_err(|e| e.to_string())?;
            renderer.protocol_test_blocked_reauthorization_checkpoint(&handoff.pause_token)?;
            ensure(!ack_path.exists(), "Malformed request produced an ACK")?;
            fs::write(&request_path, request.encode()?).map_err(|e| e.to_string())?;
            let mut foreign = handoff.clone();
            foreign.pause_token = "foreign-pause".into();
            fs::write(&request_path, ReauthorizationRecord {
                stage: ReauthorizationStage::Request, handoff: foreign,
            }.encode()?).map_err(|e| e.to_string())?;
            renderer.protocol_test_blocked_reauthorization_checkpoint(&handoff.pause_token)?;
            ensure(!ack_path.exists(), "Mismatched request produced an ACK")?;
            fs::write(&request_path, request.encode()?).map_err(|e| e.to_string())?;
            fs::remove_file(root.join(NATIVE_REVOKED_FILE)).map_err(|e| e.to_string())?;
            renderer.protocol_test_blocked_reauthorization_checkpoint(&handoff.pause_token)?;
            ensure(!ack_path.exists(), "Missing revocation produced an ACK")?;
            fs::write(root.join(NATIVE_REVOKED_FILE), handoff.native.encode()?)
                .map_err(|e| e.to_string())?;
            fs::write(root.join(NATIVE_GRANT_FILE), handoff.native.encode()?)
                .map_err(|e| e.to_string())?;
            renderer.protocol_test_blocked_reauthorization_checkpoint(&handoff.pause_token)?;
            ensure(!ack_path.exists(), "Present native grant produced an ACK")?;
            Ok(())
        })();
        let _ = fs::remove_file(&request_path);
        let _ = fs::remove_file(&ack_path);
        let _ = remove_marker(PAUSE_REQUEST_FILE);
        let _ = reset_native();
        result
    })(), &mut failures);

    // Phase 7K-4C-1: the coordinator publishes a no-replace release proposal;
    // the renderer independently verifies it without clearing its latch.
    check("Legacy release authorization: isolated coordinator and blocked renderer", (|| {
        reset_native()?;
        for name in [REAUTHORIZE_REQUEST_FILE, REAUTHORIZE_ACK_FILE,
            REAUTHORIZE_RELEASE_FILE, PAUSE_REQUEST_FILE] { remove_marker(name)?; }
        let expected = HandoffJournal {
            pause_token: "pause-7k4c1".into(),
            native: NativeOwnershipRecord {
                token: "abcdef0123456789abcdef0123456789".into(),
                pid: std::process::id(),
                start_ticks: process_start_ticks(std::process::id())?
                    .ok_or("Missing protocol test process identity")?,
            },
        };
        let result = (|| -> Result<(), String> {
            let root = runtime_directory()?;
            let renderer = crate::manage_wallpaper_runtime::WallpaperRuntimeControl::new(true, false);
            fs::write(root.join(HANDOFF_JOURNAL_FILE), expected.encode()?).map_err(|e| e.to_string())?;
            fs::write(root.join(PAUSE_REQUEST_FILE), format!("{}\n", expected.pause_token)).map_err(|e| e.to_string())?;
            fs::write(root.join(NATIVE_REVOKED_FILE), expected.native.encode()?).map_err(|e| e.to_string())?;
            fs::write(root.join(REAUTHORIZE_REQUEST_FILE), ReauthorizationRecord {
                stage: ReauthorizationStage::Request, handoff: expected.clone(),
            }.encode()?).map_err(|e| e.to_string())?;
            ensure(renderer.protocol_test_verify_release_while_blocked(&expected.pause_token).is_err(),
                "Renderer accepted release before coordinator publication")?;
            ensure(publish_isolated_release_authorization(&expected).is_err(),
                "Coordinator authorized release without renderer ACK")?;
            renderer.protocol_test_blocked_reauthorization_checkpoint(&expected.pause_token)?;
            publish_isolated_release_authorization(&expected)?;
            renderer.protocol_test_verify_release_while_blocked(&expected.pause_token)?;
            ensure(legacy_frame_gate().is_err(), "Release proposal bypassed legacy frame gate")?;
            ensure(publish_isolated_release_authorization(&expected).is_err(),
                "Duplicate release authorization overwrote existing record")?;
            fs::write(root.join(REAUTHORIZE_RELEASE_FILE), "v1 malformed\n")
                .map_err(|e| e.to_string())?;
            ensure(renderer.protocol_test_verify_release_while_blocked(&expected.pause_token).is_err(),
                "Malformed release accepted")?;
            fs::write(root.join(REAUTHORIZE_RELEASE_FILE), ReauthorizationRecord {
                stage: ReauthorizationStage::Acknowledged, handoff: expected.clone(),
            }.encode()?).map_err(|e| e.to_string())?;
            fs::remove_file(root.join(NATIVE_REVOKED_FILE)).map_err(|e| e.to_string())?;
            ensure(renderer.protocol_test_verify_release_while_blocked(&expected.pause_token).is_err(),
                "Release accepted after revocation evidence disappeared")?;
            Ok(())
        })();
        for name in [REAUTHORIZE_REQUEST_FILE, REAUTHORIZE_ACK_FILE,
            REAUTHORIZE_RELEASE_FILE, PAUSE_REQUEST_FILE] { let _ = remove_marker(name); }
        let _ = reset_native();
        result
    })(), &mut failures);

    // Phase 7K-4C-2A: prepare retirement without removing the journal or
    // pause. The existing frame gate must remain closed in every case.
    check("Legacy retirement preparation: durable transaction, no frame release", (|| {
        reset_native()?;
        for name in [REAUTHORIZE_REQUEST_FILE, REAUTHORIZE_ACK_FILE,
            REAUTHORIZE_RELEASE_FILE, REAUTHORIZE_RETIREMENT_FILE, PAUSE_REQUEST_FILE] {
            remove_marker(name)?;
        }
        let expected = HandoffJournal {
            pause_token: "pause-7k4c2a".into(),
            native: NativeOwnershipRecord {
                token: "abcdef0123456789abcdef0123456789".into(),
                pid: std::process::id(),
                start_ticks: process_start_ticks(std::process::id())?
                    .ok_or("Missing protocol test process identity")?,
            },
        };
        let result = (|| -> Result<(), String> {
            let root = runtime_directory()?;
            let renderer = crate::manage_wallpaper_runtime::WallpaperRuntimeControl::new(true, false);
            fs::write(root.join(HANDOFF_JOURNAL_FILE), expected.encode()?).map_err(|e| e.to_string())?;
            fs::write(root.join(PAUSE_REQUEST_FILE), format!("{}\n", expected.pause_token)).map_err(|e| e.to_string())?;
            fs::write(root.join(NATIVE_REVOKED_FILE), expected.native.encode()?).map_err(|e| e.to_string())?;
            fs::write(root.join(REAUTHORIZE_REQUEST_FILE), ReauthorizationRecord {
                stage: ReauthorizationStage::Request, handoff: expected.clone(),
            }.encode()?).map_err(|e| e.to_string())?;
            ensure(publish_isolated_retirement_preparation(&expected).is_err(),
                "Retirement prepared before renderer acknowledgment")?;
            renderer.protocol_test_blocked_reauthorization_checkpoint(&expected.pause_token)?;
            ensure(publish_isolated_retirement_preparation(&expected).is_err(),
                "Retirement prepared before release authorization")?;
            publish_isolated_release_authorization(&expected)?;
            renderer.protocol_test_verify_release_while_blocked(&expected.pause_token)?;
            publish_isolated_retirement_preparation(&expected)?;
            renderer.protocol_test_verify_retirement_while_blocked(&expected.pause_token)?;
            ensure(legacy_frame_gate().is_err(), "Retirement preparation opened frame gate")?;
            ensure(root.join(HANDOFF_JOURNAL_FILE).exists() && root.join(PAUSE_REQUEST_FILE).exists(),
                "Retirement preparation prematurely withdrew ownership evidence")?;
            ensure(publish_isolated_retirement_preparation(&expected).is_err(),
                "Duplicate retirement publication overwrote the record")?;
            fs::write(root.join(REAUTHORIZE_RETIREMENT_FILE), "v1 malformed\n")
                .map_err(|e| e.to_string())?;
            ensure(renderer.protocol_test_verify_retirement_while_blocked(&expected.pause_token).is_err(),
                "Malformed retirement preparation accepted")?;
            fs::write(root.join(REAUTHORIZE_RETIREMENT_FILE), ReauthorizationRecord {
                stage: ReauthorizationStage::RetirementPrepared, handoff: expected.clone(),
            }.encode()?).map_err(|e| e.to_string())?;
            fs::remove_file(root.join(NATIVE_REVOKED_FILE)).map_err(|e| e.to_string())?;
            ensure(renderer.protocol_test_verify_retirement_while_blocked(&expected.pause_token).is_err(),
                "Retirement accepted without native revocation evidence")?;
            ensure(legacy_frame_gate().is_err(), "Invalid retirement opened frame gate")?;
            Ok(())
        })();
        for name in [REAUTHORIZE_REQUEST_FILE, REAUTHORIZE_ACK_FILE,
            REAUTHORIZE_RELEASE_FILE, REAUTHORIZE_RETIREMENT_FILE, PAUSE_REQUEST_FILE] {
            let _ = remove_marker(name);
        }
        let _ = reset_native();
        result
    })(), &mut failures);

    check("Legacy retirement commit: interrupted withdrawal remains blocked", (|| {
        reset_native()?;
        for name in [REAUTHORIZE_REQUEST_FILE, REAUTHORIZE_ACK_FILE,
            REAUTHORIZE_RELEASE_FILE, REAUTHORIZE_RETIREMENT_FILE,
            REAUTHORIZE_COMMIT_FILE, PAUSE_REQUEST_FILE] { remove_marker(name)?; }
        let expected = HandoffJournal {
            pause_token: "pause-7k4c2b".into(),
            native: NativeOwnershipRecord {
                token: "abcdef0123456789abcdef0123456789".into(),
                pid: std::process::id(),
                start_ticks: process_start_ticks(std::process::id())?
                    .ok_or("Missing protocol test process identity")?,
            },
        };
        let result = (|| -> Result<(), String> {
            let root = runtime_directory()?;
            let renderer = crate::manage_wallpaper_runtime::WallpaperRuntimeControl::new(true, false);
            fs::write(root.join(HANDOFF_JOURNAL_FILE), expected.encode()?).map_err(|e| e.to_string())?;
            fs::write(root.join(PAUSE_REQUEST_FILE), format!("{}\n", expected.pause_token)).map_err(|e| e.to_string())?;
            fs::write(root.join(NATIVE_REVOKED_FILE), expected.native.encode()?).map_err(|e| e.to_string())?;
            fs::write(root.join(REAUTHORIZE_REQUEST_FILE), ReauthorizationRecord {
                stage: ReauthorizationStage::Request, handoff: expected.clone(),
            }.encode()?).map_err(|e| e.to_string())?;
            ensure(publish_isolated_retirement_commit(&expected).is_err(),
                "Committed retirement without preparation")?;
            renderer.protocol_test_blocked_reauthorization_checkpoint(&expected.pause_token)?;
            publish_isolated_release_authorization(&expected)?;
            publish_isolated_retirement_preparation(&expected)?;
            ensure(retire_isolated_handoff_markers(&expected, true).is_err(),
                "Retired markers without durable decision")?;
            publish_isolated_retirement_commit(&expected)?;
            ensure(publish_isolated_retirement_commit(&expected).is_err(),
                "Duplicate decision publication succeeded")?;
            ensure(legacy_frame_gate().is_err(), "Commit opened legacy frame gate")?;
            retire_isolated_handoff_markers(&expected, false)?;
            ensure(!root.join(HANDOFF_JOURNAL_FILE).exists() && root.join(PAUSE_REQUEST_FILE).exists(),
                "Interrupted retirement did not preserve pause")?;
            ensure(renderer.protocol_test_verify_retirement_committed_while_blocked(&expected.pause_token).is_err(),
                "Renderer accepted incomplete retirement")?;
            ensure(legacy_frame_gate().is_err(), "Partial retirement opened frame gate")?;
            retire_isolated_handoff_markers(&expected, true)?;
            retire_isolated_handoff_markers(&expected, true)?;
            renderer.protocol_test_verify_retirement_committed_while_blocked(&expected.pause_token)?;
            ensure(legacy_frame_gate().is_err(), "Complete marker withdrawal opened frame gate")?;
            fs::write(root.join(REAUTHORIZE_COMMIT_FILE), "v1 malformed\n")
                .map_err(|e| e.to_string())?;
            ensure(legacy_frame_gate().is_err(), "Malformed decision opened frame gate")?;
            ensure(renderer.protocol_test_verify_retirement_committed_while_blocked(&expected.pause_token).is_err(),
                "Renderer accepted malformed decision")?;
            Ok(())
        })();
        for name in [REAUTHORIZE_REQUEST_FILE, REAUTHORIZE_ACK_FILE,
            REAUTHORIZE_RELEASE_FILE, REAUTHORIZE_RETIREMENT_FILE,
            REAUTHORIZE_COMMIT_FILE, PAUSE_REQUEST_FILE] { let _ = remove_marker(name); }
        let _ = reset_native();
        result
    })(), &mut failures);

    check("Legacy latch release authorization: verified while still blocked", (|| {
        reset_native()?;
        for name in [REAUTHORIZE_REQUEST_FILE, REAUTHORIZE_ACK_FILE,
            REAUTHORIZE_RELEASE_FILE, REAUTHORIZE_RETIREMENT_FILE,
            REAUTHORIZE_COMMIT_FILE, REAUTHORIZE_LATCH_RELEASE_FILE,
            PAUSE_REQUEST_FILE] { remove_marker(name)?; }
        let expected = HandoffJournal {
            pause_token: "pause-7k4c2c1".into(),
            native: NativeOwnershipRecord {
                token: "abcdef0123456789abcdef0123456789".into(),
                pid: std::process::id(),
                start_ticks: process_start_ticks(std::process::id())?
                    .ok_or("Missing test process identity")?,
            },
        };
        let result = (|| -> Result<(), String> {
            let root = runtime_directory()?;
            let renderer = crate::manage_wallpaper_runtime::WallpaperRuntimeControl::new(true, false);
            fs::write(root.join(HANDOFF_JOURNAL_FILE), expected.encode()?).map_err(|e| e.to_string())?;
            fs::write(root.join(PAUSE_REQUEST_FILE), format!("{}\n", expected.pause_token)).map_err(|e| e.to_string())?;
            fs::write(root.join(NATIVE_REVOKED_FILE), expected.native.encode()?).map_err(|e| e.to_string())?;
            fs::write(root.join(REAUTHORIZE_REQUEST_FILE), ReauthorizationRecord {
                stage: ReauthorizationStage::Request, handoff: expected.clone(),
            }.encode()?).map_err(|e| e.to_string())?;
            renderer.protocol_test_blocked_reauthorization_checkpoint(&expected.pause_token)?;
            ensure(publish_isolated_latch_release_authorization(&expected).is_err(),
                "Authorized latch before retirement")?;
            publish_isolated_release_authorization(&expected)?;
            publish_isolated_retirement_preparation(&expected)?;
            publish_isolated_retirement_commit(&expected)?;
            ensure(publish_isolated_latch_release_authorization(&expected).is_err(),
                "Authorized latch before marker withdrawal")?;
            retire_isolated_handoff_markers(&expected, true)?;
            ensure(renderer.protocol_test_verify_latch_release_authorization_while_blocked(&expected.pause_token).is_err(),
                "Renderer accepted missing latch authorization")?;
            publish_isolated_latch_release_authorization(&expected)?;
            ensure(publish_isolated_latch_release_authorization(&expected).is_err(),
                "Duplicate latch authorization succeeded")?;
            renderer.protocol_test_verify_latch_release_authorization_while_blocked(&expected.pause_token)?;
            ensure(legacy_frame_gate().is_err(), "Latch authorization opened frame gate")?;
            ensure(renderer.protocol_test_verify_latch_release_authorization_while_blocked("different-token").is_err(),
                "Renderer accepted different pause identity")?;
            fs::write(root.join(REAUTHORIZE_LATCH_RELEASE_FILE), "v1 malformed\n").map_err(|e| e.to_string())?;
            ensure(renderer.protocol_test_verify_latch_release_authorization_while_blocked(&expected.pause_token).is_err(),
                "Renderer accepted malformed latch authorization")?;
            fs::write(root.join(REAUTHORIZE_LATCH_RELEASE_FILE), ReauthorizationRecord {
                stage: ReauthorizationStage::LatchReleaseAuthorized, handoff: expected.clone(),
            }.encode()?).map_err(|e| e.to_string())?;
            remove_marker(NATIVE_REVOKED_FILE)?;
            ensure(renderer.protocol_test_verify_latch_release_authorization_while_blocked(&expected.pause_token).is_err(),
                "Renderer accepted missing revocation evidence")?;
            Ok(())
        })();
        for name in [REAUTHORIZE_REQUEST_FILE, REAUTHORIZE_ACK_FILE,
            REAUTHORIZE_RELEASE_FILE, REAUTHORIZE_RETIREMENT_FILE,
            REAUTHORIZE_COMMIT_FILE, REAUTHORIZE_LATCH_RELEASE_FILE,
            PAUSE_REQUEST_FILE] { let _ = remove_marker(name); }
        let _ = reset_native();
        result
    })(), &mut failures);

    check("Legacy latch transition: isolated simulated frame confirmation", (|| {
        reset_native()?;
        for name in [REAUTHORIZE_REQUEST_FILE, REAUTHORIZE_ACK_FILE,
            REAUTHORIZE_RELEASE_FILE, REAUTHORIZE_RETIREMENT_FILE,
            REAUTHORIZE_COMMIT_FILE, REAUTHORIZE_LATCH_RELEASE_FILE,
            PAUSE_REQUEST_FILE, HANDOFF_JOURNAL_FILE] { remove_marker(name)?; }
        let expected = HandoffJournal {
            pause_token: "pause-7k4c2c2".into(),
            native: NativeOwnershipRecord {
                token: "abcdef0123456789abcdef0123456789".into(),
                pid: std::process::id(),
                start_ticks: process_start_ticks(std::process::id())?
                    .ok_or("Missing test process identity")?,
            },
        };
        let result = (|| -> Result<(), String> {
            let root = runtime_directory()?;
            let renderer = crate::manage_wallpaper_runtime::WallpaperRuntimeControl::new(true, false);
            fs::write(root.join(HANDOFF_JOURNAL_FILE), expected.encode()?).map_err(|e| e.to_string())?;
            fs::write(root.join(PAUSE_REQUEST_FILE), format!("{}\n", expected.pause_token)).map_err(|e| e.to_string())?;
            fs::write(root.join(NATIVE_REVOKED_FILE), expected.native.encode()?).map_err(|e| e.to_string())?;
            fs::write(root.join(REAUTHORIZE_REQUEST_FILE), ReauthorizationRecord {
                stage: ReauthorizationStage::Request, handoff: expected.clone(),
            }.encode()?).map_err(|e| e.to_string())?;
            renderer.protocol_test_blocked_reauthorization_checkpoint(&expected.pause_token)?;
            ensure(renderer.protocol_test_release_latch(&expected.pause_token).is_err(),
                "Latch released before authorization")?;
            publish_isolated_release_authorization(&expected)?;
            publish_isolated_retirement_preparation(&expected)?;
            publish_isolated_retirement_commit(&expected)?;
            retire_isolated_handoff_markers(&expected, true)?;
            publish_isolated_latch_release_authorization(&expected)?;
            ensure(renderer.protocol_test_release_latch("wrong-token").is_err(),
                "Latch released for incorrect pause identity")?;
            renderer.protocol_test_release_latch(&expected.pause_token)?;
            ensure(renderer.protocol_test_release_latch(&expected.pause_token).is_err(),
                "Duplicate latch release succeeded")?;
            ensure(renderer.protocol_test_simulated_frame_confirmed("wrong-token").is_err(),
                "Accepted simulated confirmation for incorrect transaction")?;
            ensure(legacy_frame_gate().is_err(), "Real frame gate opened on latch transition")?;
            renderer.protocol_test_simulated_frame_confirmed(&expected.pause_token)?;
            ensure(legacy_frame_gate().is_err(), "Simulated confirmation opened real frame gate")?;
            Ok(())
        })();
        for name in [REAUTHORIZE_REQUEST_FILE, REAUTHORIZE_ACK_FILE,
            REAUTHORIZE_RELEASE_FILE, REAUTHORIZE_RETIREMENT_FILE,
            REAUTHORIZE_COMMIT_FILE, REAUTHORIZE_LATCH_RELEASE_FILE,
            PAUSE_REQUEST_FILE, HANDOFF_JOURNAL_FILE] { let _ = remove_marker(name); }
        let _ = reset_native();
        result
    })(), &mut failures);

    // Increment 09C: exercise the exact 09B C++ wire format. These tests
    // verify diagnostic parsing only; they never authorize legacy rendering.
    check("Native node ledger: all observed nodes retired", (|| {
        let native = native_identity();
        let path = required_path("screenshaver-node-ledger-test.observed")?;
        let wire = format!("v1 nodes {} {} {}\n101 retired\n202 retired\n",
            native.token, native.pid, native.start_ticks);
        let result = (|| -> Result<(), String> {
            fs::write(&path, wire).map_err(|e| e.to_string())?;
            verify_isolated_native_node_ledger(&path, &native, &[101, 202])?;
            ensure(verify_isolated_native_node_ledger(&path, &native, &[101]).is_err(),
                "Unaccounted node accepted")?;
            ensure(verify_isolated_native_node_ledger(&path, &native, &[101, 303]).is_err(),
                "Unknown node accepted")
        })();
        let _ = fs::remove_file(&path);
        result
    })(), &mut failures);

    check("Native node ledger: active, duplicate, stale and malformed rejected", (|| {
        let native = native_identity();
        let path = required_path("screenshaver-node-ledger-test.observed")?;
        let header = format!("v1 nodes {} {} {}\n", native.token, native.pid, native.start_ticks);
        let result = (|| -> Result<(), String> {
            for entries in ["101 active\n202 retired\n", "101 retired\n101 retired\n",
                "202 retired\n101 retired\n", "0 retired\n", "101 done\n",
                "101 retired\n202 retired\nextra\n", ""] {
                fs::write(&path, format!("{header}{entries}"))
                    .map_err(|e| e.to_string())?;
                ensure(verify_isolated_native_node_ledger(&path, &native, &[101, 202]).is_err(),
                    "Invalid node ledger accepted")?;
            }
            fs::write(&path, format!("{header}101 retired\n202 retired\n"))
                .map_err(|e| e.to_string())?;
            let mut stale = native.clone(); stale.start_ticks += 1;
            ensure(verify_isolated_native_node_ledger(&path, &stale, &[101, 202]).is_err(),
                "Stale Plasma identity accepted")?;
            let target = path.with_extension("target");
            fs::rename(&path, &target).map_err(|e| e.to_string())?;
            let symlink_result = std::os::unix::fs::symlink(&target, &path)
                .map_err(|e| e.to_string());
            if symlink_result.is_ok() {
                ensure(verify_isolated_native_node_ledger(&path, &native, &[101, 202]).is_err(),
                    "Symlink node ledger accepted")?;
            }
            let _ = fs::remove_file(&target);
            Ok(())
        })();
        let _ = fs::remove_file(&path);
        result
    })(), &mut failures);

    check("Native census: matching closed census and retired ledger", (|| {
        let native = native_identity();
        let census = required_path("screenshaver-node-census-test.observed")?;
        let ledger = required_path("screenshaver-node-ledger-census-test.observed")?;
        let result = (|| -> Result<(), String> {
            fs::write(&census, format!("v1 census {} {} {} closed\n",
                native.token, native.pid, native.start_ticks)).map_err(|e| e.to_string())?;
            fs::write(&ledger, format!("v1 nodes {} {} {}\n101 retired\n202 retired\n",
                native.token, native.pid, native.start_ticks)).map_err(|e| e.to_string())?;
            verify_isolated_closed_native_node_census(&census, &ledger, &native, &[101, 202])?;
            ensure(verify_isolated_closed_native_node_census(
                &census, &ledger, &native, &[101]).is_err(), "Partial inventory accepted")?;
            ensure(verify_isolated_closed_native_node_census(
                &census, &ledger, &native, &[101, 303]).is_err(), "Unknown node accepted")
        })();
        let _ = fs::remove_file(&census);
        let _ = fs::remove_file(&ledger);
        result
    })(), &mut failures);

    check("Native census: open, invalid, stale and malformed rejected", (|| {
        let native = native_identity();
        let census = required_path("screenshaver-node-census-test.observed")?;
        let ledger = required_path("screenshaver-node-ledger-census-test.observed")?;
        let header = format!("v1 census {} {} {}",
            native.token, native.pid, native.start_ticks);
        let result = (|| -> Result<(), String> {
            fs::write(&ledger, format!("v1 nodes {} {} {}\n101 retired\n202 retired\n",
                native.token, native.pid, native.start_ticks)).map_err(|e| e.to_string())?;
            for wire in [
                format!("{header} open\n"), format!("{header} invalid\n"),
                format!("{header} CLOSED\n"), format!("{header} closed\nextra\n"),
                format!("{header} closed\r\n"), format!("{header} closed"),
                format!("{header}  closed\n"),
                format!("v2 census {} {} {} closed\n", native.token, native.pid, native.start_ticks),
            ] {
                fs::write(&census, wire).map_err(|e| e.to_string())?;
                ensure(verify_isolated_closed_native_node_census(
                    &census, &ledger, &native, &[101, 202]).is_err(),
                    "Invalid census accepted")?;
            }
            fs::write(&census, format!("{header} closed\n")).map_err(|e| e.to_string())?;
            let mut stale = native.clone(); stale.start_ticks += 1;
            ensure(verify_isolated_closed_native_node_census(
                &census, &ledger, &stale, &[101, 202]).is_err(),
                "Stale census identity accepted")?;
            fs::write(&ledger, format!("v1 nodes {} {} {}\n101 active\n202 retired\n",
                native.token, native.pid, native.start_ticks)).map_err(|e| e.to_string())?;
            ensure(verify_isolated_closed_native_node_census(
                &census, &ledger, &native, &[101, 202]).is_err(),
                "Active ledger accepted")?;
            let target = census.with_extension("target");
            fs::rename(&census, &target).map_err(|e| e.to_string())?;
            let link_result = std::os::unix::fs::symlink(&target, &census)
                .map_err(|e| e.to_string());
            if link_result.is_ok() {
                ensure(verify_isolated_closed_native_node_census(
                    &census, &ledger, &native, &[101, 202]).is_err(),
                    "Symlink census accepted")?;
            }
            let _ = fs::remove_file(&target);
            Ok(())
        })();
        let _ = fs::remove_file(&census);
        let _ = fs::remove_file(&ledger);
        result
    })(), &mut failures);

    check("Native atomic snapshot: closed and fully retired", (|| {
        let native = native_identity();
        let path = required_path("screenshaver-native-snapshot-test.observed")?;
        let result = (|| -> Result<(), String> {
            fs::write(&path, format!("v1 snapshot {} {} {} closed\n101 retired\n202 retired\n",
                native.token, native.pid, native.start_ticks)).map_err(|e| e.to_string())?;
            verify_isolated_native_node_snapshot(&path, &native, &[101, 202])?;
            ensure(verify_isolated_native_node_snapshot(&path, &native, &[101]).is_err(),
                "Incomplete expected node set accepted")?;
            Ok(())
        })();
        let _ = fs::remove_file(&path);
        result
    })(), &mut failures);

    check("Native atomic snapshot: malformed and incomplete rejected", (|| {
        let native = native_identity();
        let path = required_path("screenshaver-native-snapshot-test.observed")?;
        let prefix = format!("v1 snapshot {} {} {}", native.token, native.pid, native.start_ticks);
        let result = (|| -> Result<(), String> {
            for wire in [
                format!("{prefix} open\n101 retired\n"),
                format!("{prefix} invalid\n101 retired\n"),
                format!("{prefix} closed\n101 active\n"),
                format!("{prefix} closed\n101 retired\n101 retired\n"),
                format!("{prefix} closed\n202 retired\n101 retired\n"),
                format!("{prefix} closed\n"),
                format!("{prefix} closed\n01 retired\n"),
                format!("{prefix} closed\n101 retired"),
                format!("{prefix} closed\r\n101 retired\n"),
                format!("{prefix} closed\n101 retired\nextra\n"),
            ] {
                fs::write(&path, wire).map_err(|e| e.to_string())?;
                ensure(verify_isolated_native_node_snapshot(&path, &native, &[101]).is_err(),
                    "Malformed snapshot accepted")?;
            }
            fs::write(&path, format!("{prefix} closed\n101 retired\n"))
                .map_err(|e| e.to_string())?;
            let mut stale = native.clone(); stale.start_ticks += 1;
            ensure(verify_isolated_native_node_snapshot(&path, &stale, &[101]).is_err(),
                "Stale snapshot accepted")?;
            let target = path.with_extension("target");
            fs::rename(&path, &target).map_err(|e| e.to_string())?;
            std::os::unix::fs::symlink(&target, &path).map_err(|e| e.to_string())?;
            ensure(verify_isolated_native_node_snapshot(&path, &native, &[101]).is_err(),
                "Symlink snapshot accepted")?;
            let _ = fs::remove_file(&target);
            Ok(())
        })();
        let _ = fs::remove_file(&path);
        result
    })(), &mut failures);

    check("Batch 7F-D: retirement commit correlation is transaction-scoped", (|| {
        let native = native_identity();
        let expected = HandoffJournal {
            pause_token: "123-456-1".to_owned(), native: native.clone(),
        };
        ensure(classify_retirement_commit(&expected, None) == "absent",
            "Missing commit must not be interpreted as retirement")?;
        ensure(classify_retirement_commit(&expected, Some(&expected))
            == "exact_transaction_match_observation_only",
            "Matching commit classification failed")?;
        let mut other = expected.clone();
        other.native.token = "ffffffffffffffffffffffffffffffff".to_owned();
        ensure(classify_retirement_commit(&expected, Some(&other))
            == "different_transaction_blocking",
            "Stale native generation commit was accepted")?;
        other = expected.clone();
        other.pause_token = "123-456-2".to_owned();
        ensure(classify_retirement_commit(&expected, Some(&other))
            == "different_transaction_blocking",
            "Stale pause transaction commit was accepted")?;
        Ok(())
    })(), &mut failures);

    check("Batch 7F-E: production retirement never infers compositor quiescence", (|| {
        let expected = HandoffJournal {
            pause_token: "123-456-1".to_owned(), native: native_identity(),
        };
        let mut stale = expected.clone();
        stale.native.token = "ffffffffffffffffffffffffffffffff".to_owned();
        ensure(production_retirement_gate(&expected, None, true)
            == ProductionRetirementGate::MissingOrMismatchedCommit,
            "Missing commit permitted retirement")?;
        ensure(production_retirement_gate(&expected, Some(&stale), true)
            == ProductionRetirementGate::MissingOrMismatchedCommit,
            "Stale commit permitted retirement")?;
        ensure(production_retirement_gate(&expected, Some(&expected), false)
            == ProductionRetirementGate::NativeRevocationUnverified,
            "Missing revocation permitted retirement")?;
        ensure(production_retirement_gate(&expected, Some(&expected), true)
            == ProductionRetirementGate::CompositorQuiescenceUnverified,
            "Commit plus ACK incorrectly certified compositor quiescence")?;
        Ok(())
    })(), &mut failures);

    check("Native scene retirement: canonical C++ record and matching node", (|| {
        let record = native_identity();
        let node_id = 140737488355328u64;
        let path = required_path(NATIVE_SCENE_RETIREMENT_FILE)?;
        let result = (|| -> Result<(), String> {
            let observation = NativeSceneRetirementObservation { native: record.clone(), node_id };
            let wire = observation.encode()?;
            ensure(wire == format!("v1 scene-retired {} {} {} {}\n",
                record.token, record.pid, record.start_ticks, node_id),
                "C++ retirement wire format diverges from Rust")?;
            fs::write(&path, wire).map_err(|e| e.to_string())?;
            verify_native_scene_retirement_observation(&path, &record, node_id)
        })();
        let _ = fs::remove_file(&path);
        result
    })(), &mut failures);

    check("Native scene retirement: stale transaction, Plasma and node rejected", (|| {
        let record = native_identity();
        let node_id = 42u64;
        let path = required_path(NATIVE_SCENE_RETIREMENT_FILE)?;
        let result = (|| -> Result<(), String> {
            fs::write(&path, NativeSceneRetirementObservation {
                native: record.clone(), node_id,
            }.encode()?).map_err(|e| e.to_string())?;
            ensure(verify_native_scene_retirement_observation(&path, &record, node_id + 1).is_err(),
                "Wrong render node accepted")?;
            let mut wrong = record.clone(); wrong.token = "00000000000000000000000000000000".into();
            ensure(verify_native_scene_retirement_observation(&path, &wrong, node_id).is_err(),
                "Wrong ownership token accepted")?;
            wrong = record.clone(); wrong.pid += 1;
            ensure(verify_native_scene_retirement_observation(&path, &wrong, node_id).is_err(),
                "Wrong Plasma PID accepted")?;
            wrong = record.clone(); wrong.start_ticks += 1;
            ensure(verify_native_scene_retirement_observation(&path, &wrong, node_id).is_err(),
                "Wrong Plasma start time accepted")
        })();
        let _ = fs::remove_file(&path);
        result
    })(), &mut failures);

    check("Native scene retirement: missing, malformed and symlink evidence rejected", (|| {
        let record = native_identity();
        let path = required_path(NATIVE_SCENE_RETIREMENT_FILE)?;
        let target = path.with_extension("09c-target");
        let result = (|| -> Result<(), String> {
            let _ = fs::remove_file(&path);
            ensure(verify_native_scene_retirement_observation(&path, &record, 1).is_err(),
                "Missing observation accepted")?;
            for payload in ["v1 scene-retired malformed\n", "v1 scene-retired 00000000000000000000000000000000 1 1 01\n",
                "v1 scene-retired 00000000000000000000000000000000 1 1 0\n",
                "v1 scene-retired 00000000000000000000000000000000 1 1 1\nextra\n"] {
                fs::write(&path, payload).map_err(|e| e.to_string())?;
                ensure(verify_native_scene_retirement_observation(&path, &record, 1).is_err(),
                    "Malformed observation accepted")?;
            }
            fs::write(&target, NativeSceneRetirementObservation {
                native: record.clone(), node_id: 1,
            }.encode()?).map_err(|e| e.to_string())?;
            fs::remove_file(&path).map_err(|e| e.to_string())?;
            std::os::unix::fs::symlink(&target, &path).map_err(|e| e.to_string())?;
            ensure(verify_native_scene_retirement_observation(&path, &record, 1).is_err(),
                "Symlink observation accepted")
        })();
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(&target);
        result
    })(), &mut failures);

    check("Native scene retirement: one node cannot certify multiple displays", (|| {
        let record = native_identity();
        let path = required_path(NATIVE_SCENE_RETIREMENT_FILE)?;
        let result = (|| -> Result<(), String> {
            fs::write(&path, NativeSceneRetirementObservation {
                native: record.clone(), node_id: 101,
            }.encode()?).map_err(|e| e.to_string())?;
            verify_native_scene_retirement_observation(&path, &record, 101)?;
            ensure(verify_native_scene_retirement_observation(&path, &record, 202).is_err(),
                "Single-node evidence accepted for another display")?;
            // This verifier has no compositor-present or all-nodes-retired result.
            // Its success must remain diagnostic and disconnected from resume.
            Ok(())
        })();
        let _ = fs::remove_file(&path);
        result
    })(), &mut failures);

    if failures.is_empty() {
        println!("[WALLPAPER PROTOCOL TEST] All protocol checks passed.");
        Ok(())
    } else {
        Err(format!("{} protocol checks failed: {}", failures.len(), failures.join("; ")))
    }
}
