use std::path::PathBuf;
use std::sync::{
    atomic::{
        AtomicBool,
        AtomicU8,
        AtomicU64,
        Ordering,
    },
    Arc,
    Mutex,
};
use std::thread::JoinHandle;
use std::sync::mpsc::{self, Sender, Receiver};
use std::time::{Duration, Instant};


const MAX_RESTART_ATTEMPTS: usize = 3;
const RESTART_DELAY_SECONDS: u64 = 2;

// Read-only lifecycle events. These are observations, not ownership transitions.
#[derive(Debug)]
enum WallpaperOwnershipEvent {
    SupervisorStarted,
    AttemptStarted(usize),
    AttemptEnded { attempt: usize, outcome: &'static str },
    SupervisorStopped,
}

/// Decision-only coordinator. No transaction is prepared or published here.
/// Unresolved pause, journal, or grant evidence blocks legacy startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SupervisorOwnershipDecision {
    LegacyEligible,
    NativeTransactionOutstanding,
    RecoveryRequired,
    VerificationRequired,
}

/// A native grant is only one piece of ownership evidence. An outstanding
/// journal or pause marker must block a new legacy supervisor even when the
/// grant is absent. This function observes state; it never performs recovery.
fn decide_wallpaper_ownership(
    assessment: &crate::control_wallpaper::AbandonedHandoffAssessment,
) -> SupervisorOwnershipDecision {
    use crate::control_wallpaper::{AbandonedHandoffAssessment, NativeGrantAssessment};
    match assessment {
        AbandonedHandoffAssessment::Clean => SupervisorOwnershipDecision::LegacyEligible,
        AbandonedHandoffAssessment::Unverified(_) => SupervisorOwnershipDecision::VerificationRequired,
        AbandonedHandoffAssessment::UnresolvedPause { native, .. }
        | AbandonedHandoffAssessment::OrphanedNativeGrant { native } => match native {
            NativeGrantAssessment::CurrentPlasma { .. } =>
                SupervisorOwnershipDecision::NativeTransactionOutstanding,
            NativeGrantAssessment::StalePlasma { .. } =>
                SupervisorOwnershipDecision::RecoveryRequired,
            NativeGrantAssessment::Absent =>
                SupervisorOwnershipDecision::RecoveryRequired,
            NativeGrantAssessment::Unverified(_) =>
                SupervisorOwnershipDecision::VerificationRequired,
        },
    }
}

fn observe_wallpaper_ownership(
    receiver: Receiver<WallpaperOwnershipEvent>,
    logfile: PathBuf,
) {
    while let Ok(event) = receiver.recv() {
        let assessment = crate::control_wallpaper::assess_abandoned_handoff();
        let decision = decide_wallpaper_ownership(&assessment);
        let presentation = crate::control_wallpaper::assess_native_presentations();
        // Correlate only when a verified grant identifies a particular Plasma
        // process instance. This remains read-only; matching status does not
        // grant native ownership or authorize legacy resume.
        let correlation = match &assessment {
            crate::control_wallpaper::AbandonedHandoffAssessment::UnresolvedPause {
                native: crate::control_wallpaper::NativeGrantAssessment::CurrentPlasma { pid, start_ticks }, ..
            } | crate::control_wallpaper::AbandonedHandoffAssessment::OrphanedNativeGrant {
                native: crate::control_wallpaper::NativeGrantAssessment::CurrentPlasma { pid, start_ticks }
            } => Some(crate::control_wallpaper::correlate_native_presentations(
                &crate::manage_wallpaper_ownership::PlasmaInstance {
                    pid: *pid, start_ticks: *start_ticks,
                }
            )),
            _ => None,
        };
        // The observer never modifies the legacy pause marker or the native
        // grant. In particular, "LegacyEligible" is not a resume command:
        // absence of a grant does not establish native presentation quiescence.
        let message = format!(
            "[WALLPAPER OWNERSHIP] event={event:?}; handoff_evidence={assessment:?}; native_presentation={presentation:?}; presentation_correlation={correlation:?}; decision={decision:?}; native_handoff=disabled; observer_only=true"
        );
        if matches!(decision, SupervisorOwnershipDecision::LegacyEligible) {
            crate::logger::information(&logfile, &message);
        } else {
            crate::logger::warning(&logfile, &message);
        }
        if matches!(event, WallpaperOwnershipEvent::SupervisorStopped) { break; }
    }
    crate::logger::information(&logfile,
        "[WALLPAPER OWNERSHIP] lifecycle decision observer stopped; no native authorization or resume issued");
}



// Phase 7F: supervisor-owned, read-only activation preflight.  This is a
// deliberately ungranted coordinator boundary.  It neither pauses the legacy
// renderer nor creates a transaction: production grant publication is disabled.
// Do not infer a target Plasma process from the first item when several
// distinct process identities are observed.
#[derive(Debug)]
enum NativeActivationPreflight {
    Ineligible(&'static str),
    NotReady(String),
    Candidate {
        plasma: crate::manage_wallpaper_ownership::PlasmaInstance,
        matching_instances: usize,
        associated_nodes: usize,
    },
    Unverified(String),
}

fn observe_native_activation_preflight(full_screen_wallpaper: bool) -> NativeActivationPreflight {
    use crate::control_wallpaper::{NativePresentationAssessment, NativePresentationPreflight};
    if !full_screen_wallpaper {
        return NativeActivationPreflight::Ineligible("not a full-screen wallpaper");
    }
    // A transaction or abandoned pause is not a fresh activation opportunity.
    // Monitor readiness is not evidence of a native GL frame.
    let evidence = crate::control_wallpaper::assess_abandoned_handoff();
    if !matches!(evidence, crate::control_wallpaper::AbandonedHandoffAssessment::Clean) {
        return NativeActivationPreflight::Unverified(format!(
            "existing ownership evidence prevents fresh activation: {evidence:?}"));
    }
    let items = match crate::control_wallpaper::assess_native_presentations() {
        NativePresentationAssessment::Absent =>
            return NativeActivationPreflight::NotReady("no native wallpaper item".into()),
        NativePresentationAssessment::Unverified(error) =>
            return NativeActivationPreflight::Unverified(error),
        NativePresentationAssessment::Observed(items) => items,
    };
    let Some(first) = items.first() else {
        return NativeActivationPreflight::NotReady("empty native presentation inventory".into());
    };
    if items.iter().any(|item| item.pid != first.pid || item.start_ticks != first.start_ticks) {
        return NativeActivationPreflight::Unverified(
            "multiple Plasma process identities observed; no unique activation target".into());
    }
    let plasma = crate::manage_wallpaper_ownership::PlasmaInstance {
        pid: first.pid, start_ticks: first.start_ticks,
    };
    match crate::control_wallpaper::assess_native_handoff_preflight(&plasma) {
        NativePresentationPreflight::Candidate { matching_instances, associated_nodes } =>
            NativeActivationPreflight::Candidate { plasma, matching_instances, associated_nodes },
        NativePresentationPreflight::NotReady(reason) => NativeActivationPreflight::NotReady(reason),
        NativePresentationPreflight::Unverified(reason) => NativeActivationPreflight::Unverified(reason),
    }
}

// Phase 7G: an attempt-scoped, supervisor-owned activation state.  A
// presentation observation is not an ownership lease.  In particular, the
// candidate must not survive a rendering attempt or be used as a cached
// authority to issue a grant later.
#[derive(Debug)]
enum NativeActivationState {
    Inactive,
    Observed {
        attempt: usize,
        plasma: crate::manage_wallpaper_ownership::PlasmaInstance,
    },
}

impl NativeActivationState {
    fn observe(&mut self, attempt: usize, observation: &NativeActivationPreflight) {
        *self = match observation {
            NativeActivationPreflight::Candidate { plasma, .. } => Self::Observed {
                attempt,
                plasma: plasma.clone(),
            },
            _ => Self::Inactive,
        };
    }

    fn invalidate(&mut self) {
        *self = Self::Inactive;
    }

    // Never treat this state as authorization: a future coordinator must
    // revalidate both the Plasma identity and preflight while acquiring the
    // verified pause lease.
    fn description(&self) -> String {
        match self {
            Self::Inactive => "inactive".to_owned(),
            Self::Observed { attempt, plasma } => format!(
                "observed_only(attempt={attempt},pid={},start_ticks={})",
                plasma.pid, plasma.start_ticks,
            ),
        }
    }
}

// Phase 7J: bounded, never-granted handoff preparation. The coordinator
// intentionally cannot publish a journal or native grant. It must not hold
// the legacy renderer paused while waiting for native authorization.
fn live_plasma_instance(
    expected: &crate::manage_wallpaper_ownership::PlasmaInstance,
) -> Result<crate::manage_wallpaper_ownership::PlasmaInstance, String> {
    let stat = std::fs::read_to_string(format!("/proc/{}/stat", expected.pid))
        .map_err(|error| format!("Cannot read Plasma process identity: {error}"))?;
    let close = stat.rfind(')').ok_or("Malformed Plasma process stat")?;
    let fields: Vec<&str> = stat[close + 1..].split_whitespace().collect();
    let ticks = fields.get(19).ok_or("Plasma process start time missing")?
        .parse::<u64>().map_err(|_| "Invalid Plasma process start time")?;
    if ticks == 0 { return Err("Zero Plasma process start time".into()); }
    Ok(crate::manage_wallpaper_ownership::PlasmaInstance {
        pid: expected.pid, start_ticks: ticks,
    })
}

// Production ownership tokens are cryptographically random and match the
// native bridge's exact 32-lowercase-hex grant contract.
fn fresh_native_transaction_token() -> Result<String, String> {
    use std::io::Read;
    let mut bytes = [0u8; 16];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut source| source.read_exact(&mut bytes))
        .map_err(|error| format!("Cannot obtain native transaction entropy: {error}"))?;
    let mut token = String::with_capacity(32);
    for byte in bytes { token.push_str(&format!("{byte:02x}")); }
    Ok(token)
}

fn observe_native_activation_during_attempt(
    attempt: usize,
    full_screen_wallpaper: bool,
    running: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    logfile: PathBuf,
) {
    let mut previous = String::new();
    let mut next_preparation = Instant::now();
    // Two independent observations must agree on the exact Plasma instance.
    // This is a stability prerequisite, NOT authorization or presentation proof.
    let mut previous_candidate: Option<crate::manage_wallpaper_ownership::PlasmaInstance> = None;
    while running.load(Ordering::SeqCst) && !stop.load(Ordering::SeqCst) {
        let observation = observe_native_activation_preflight(full_screen_wallpaper);
        let stable_candidate = match &observation {
            NativeActivationPreflight::Candidate { plasma, .. } => {
                let stable = previous_candidate.as_ref() == Some(plasma);
                previous_candidate = Some(plasma.clone());
                stable
            }
            _ => {
                previous_candidate = None;
                false
            }
        };
        let description = format!("{observation:?}; stable_candidate={stable_candidate}");
        if description != previous {
            crate::logger::information(&logfile, &format!(
                "[KDE NATIVE COORDINATOR] attempt={attempt}; preflight={description}; grant_published=false"
            ));
            previous = description;
        }
        if let NativeActivationPreflight::Candidate { plasma, .. } = observation {
            if stable_candidate && Instant::now() >= next_preparation &&
                running.load(Ordering::SeqCst) && !stop.load(Ordering::SeqCst) {
                // Avoid continuously interrupting the legacy wallpaper while
                // the native grant publisher remains deliberately disabled.
                next_preparation = Instant::now() + Duration::from_secs(10);
                let token = match fresh_native_transaction_token() {
                    Ok(token) => token,
                    Err(error) => {
                        crate::logger::error(&logfile, &format!(
                            "[KDE NATIVE COORDINATOR] cannot create production token: {error}"
                        ));
                        break;
                    }
                };
                let transaction = crate::manage_wallpaper_ownership::TransactionId(token);
                let result = crate::control_wallpaper::NativeHandoffPreparation::acquire_with_preflight(
                    transaction,
                    plasma.clone(),
                    || live_plasma_instance(&plasma),
                    |expected| crate::control_wallpaper::assess_native_handoff_preflight(expected),
                );
                match result {
                    Ok(prepared) => {
                        // This is the production handoff: journal first, then
                        // native grant, while the acknowledged legacy pause and
                        // exclusive transaction lock remain held.
                        let mut granted = match crate::control_wallpaper::NativeGrantedTransaction::publish_production(
                            prepared, || live_plasma_instance(&plasma),
                        ) {
                            Ok(granted) => granted,
                            Err(error) => {
                                crate::logger::error(&logfile, &format!(
                                    "[KDE NATIVE COORDINATOR] production grant failed: {error}; stopping attempt"
                                ));
                                // Publication failure can leave durable blocking
                                // evidence; never initiate a new handoff blindly.
                                break;
                            }
                        };
                        crate::logger::information(&logfile, &format!(
                            "[KDE NATIVE COORDINATOR] attempt={attempt}; native grant published; legacy renderer remains paused"
                        ));
                        // Retain ownership until this rendering attempt ends.
                        // Never start a second grant while the first is live.
                        while running.load(Ordering::SeqCst) && !stop.load(Ordering::SeqCst) {
                            if live_plasma_instance(&plasma).ok().as_ref() != Some(&plasma) {
                                crate::logger::error(&logfile,
                                    "[KDE NATIVE COORDINATOR] Plasma instance changed; native ownership unresolved");
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(100));
                        }
                        // Withdrawal is safe only for the exact same Plasma
                        // process. The retained pause/journal remain blocking
                        // until the separate production retirement path exists.
                        match granted.withdraw_production_grant(|| live_plasma_instance(&plasma)) {
                            Ok(()) => {
                                crate::logger::information(&logfile,
                                    "[KDE NATIVE COORDINATOR] native grant withdrawn; awaiting transaction-matched revocation ACK; legacy pause retained");
                                // The Qt GUI watcher polls at 50ms. An immediate
                                // verification races that watcher, so wait for a
                                // bounded interval. Never infer ACK from timeout.
                                let deadline = Instant::now() + Duration::from_secs(2);
                                loop {
                                    match granted.verify_production_revocation_ack(
                                        || live_plasma_instance(&plasma)
                                    ) {
                                        Ok(()) => {
                                            crate::logger::information(&logfile,
                                                "[KDE NATIVE COORDINATOR] native GL revoked; legacy pause retained pending compositor retirement");
                                            break;
                                        }
                                        Err(error) if Instant::now() >= deadline => {
                                            crate::logger::error(&logfile, &format!(
                                                "[KDE NATIVE COORDINATOR] revocation ACK unverified after 2s: {error}; legacy remains blocked"
                                            ));
                                            break;
                                        }
                                        Err(_) => std::thread::sleep(Duration::from_millis(25)),
                                    }
                                }
                            }
                            Err(error) => crate::logger::error(&logfile, &format!(
                                "[KDE NATIVE COORDINATOR] native withdrawal unverified: {error}; legacy remains blocked"
                            )),
                        }
                        break;
                    }
                    Err(error) => crate::logger::warning(&logfile, &format!(
                        "[KDE NATIVE COORDINATOR] attempt={attempt}; preparation_rejected={error}; native_grant=false"
                    )),
                }
            }
        }
        for _ in 0..5 {
            if stop.load(Ordering::SeqCst) || !running.load(Ordering::SeqCst) { break; }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    crate::logger::information(&logfile, &format!(
        "[KDE NATIVE COORDINATOR] attempt={attempt}; coordinator stopped"
    ));
}


fn recover_native_wallpaper_while_legacy_blocked(
    running: &AtomicBool,
    logfile: &std::path::Path,
) {
    // Keep the supervisor alive so a native-only replacement can be issued
    // after Plasma has recreated its ownership-monitor QML item.
    let mut last = String::new();
    while running.load(Ordering::SeqCst) {
        let inventory = crate::control_wallpaper::assess_native_presentations();
        let target = match inventory {
            crate::control_wallpaper::NativePresentationAssessment::Observed(items)
                if !items.is_empty() && items.iter().all(|item|
                    item.pid == items[0].pid && item.start_ticks == items[0].start_ticks) => {
                Some(crate::manage_wallpaper_ownership::PlasmaInstance {
                    pid: items[0].pid, start_ticks: items[0].start_ticks,
                })
            }
            _ => None,
        };
        if let Some(plasma) = target {
            if matches!(crate::control_wallpaper::assess_native_handoff_preflight(&plasma),
                crate::control_wallpaper::NativePresentationPreflight::Candidate { .. }) {
                // Recover a stranded grant before replacement. This operation
                // holds the transaction lock and refuses to act while the old
                // requester is alive. The legacy pause/journal remain intact.
                if let Err(error) = crate::control_wallpaper::withdraw_abandoned_native_grant(&plasma) {
                    let reason = format!("abandoned withdrawal pending: {error}");
                    if reason != last {
                        crate::logger::warning(logfile, &format!(
                            "[KDE NATIVE COORDINATOR] {reason}; legacy remains blocked"));
                        last = reason;
                    }
                    std::thread::sleep(Duration::from_millis(250));
                    continue;
                }
                // A withdrawn grant is not a revocation ACK. The replacement
                // function independently requires exact ACK + absent grant.
                match fresh_native_transaction_token().and_then(|token|
                    crate::control_wallpaper::replace_native_generation(&token, &plasma)) {
                    Ok(()) => {
                        crate::logger::information(logfile,
                            "[KDE NATIVE COORDINATOR] replacement grant published; legacy pause retained");
                        // Keep legacy blocked for this entire supervisor lifetime.
                        while running.load(Ordering::SeqCst) {
                            std::thread::sleep(Duration::from_millis(250));
                        }
                        return;
                    }
                    Err(error) if error != last => {
                        crate::logger::warning(logfile, &format!(
                            "[KDE NATIVE COORDINATOR] replacement pending: {error}"));
                        last = error;
                    }
                    Err(_) => {}
                }
            }
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

#[derive(Clone)]
pub(crate) struct WallpaperPolicyReload {
    pub(crate) animation_speed_policy:
        crate::load_config::AnimationSpeedPolicy,
    pub(crate) fps_policy:
        crate::load_config::FpsPolicy,
    pub(crate) texture_policy:
        crate::load_config::TexturePolicy,
    pub(crate) postprocess_policy:
        crate::load_config::PostprocessPolicy,
}


impl WallpaperPolicyReload {
    pub(crate) fn from_config(
        config: &crate::load_config::Config,
    ) -> Self {
        Self {
            animation_speed_policy:
                config.wallpaper_speed_policy.clone(),
            fps_policy:
                config.wallpaper_fps_policy.clone(),
            texture_policy:
                config.wallpaper_texture_policy.clone(),
            postprocess_policy:
                config.wallpaper_postprocess_policy.clone(),
        }
    }

    pub(crate) fn from_windowshader_config(
        config: &crate::load_config::Config,
    ) -> Self {
        Self {
            animation_speed_policy: config.windowshader_speed_policy.clone(),
            fps_policy: config.windowshader_fps_policy.clone(),
            texture_policy: config.windowshader_texture_policy.clone(),
            postprocess_policy: config.windowshader_postprocess_policy.clone(),
        }
    }
}



/// Coordinates the two independently owned presentation supervisors.
/// A missing supervisor is treated as already quiescent.
#[derive(Clone)]
pub struct PresentationRuntimeControl {
    wallpaper: Option<WallpaperRuntimeControl>,
    windowshader: Option<WallpaperRuntimeControl>,
}

impl PresentationRuntimeControl {
    pub fn new(
        wallpaper: Option<WallpaperRuntimeControl>,
        windowshader: Option<WallpaperRuntimeControl>,
    ) -> Self {
        Self { wallpaper, windowshader }
    }

    pub fn request_pause_after_first_frame(&self, running: &AtomicBool) -> bool {
        let wallpaper_ok = self.wallpaper.as_ref().map_or(true, |control| {
            control.request_pause_after_first_frame(running)
        });
        let windowshader_ok = self.windowshader.as_ref().map_or(true, |control| {
            control.request_pause_after_first_frame(running)
        });
        if !wallpaper_ok || !windowshader_ok {
            crate::logger::error(
                &crate::locate_paths::runtime_log_path(),
                &format!("[PRESENTATION] Pause acknowledgment failed: wallpaper={}, windowshader={}", wallpaper_ok, windowshader_ok),
            );
            // The caller must not proceed with a handoff on failure. Release
            // any successful partial pause to avoid stranding a renderer.
            self.resume_and_wait_for_frame(running);
            return false;
        }
        true
    }

    /// Release both pause requests, then verify each active target independently.
    /// A false result means at least one renderer did not confirm a new frame.
    pub fn resume_and_wait_for_frame(&self, running: &AtomicBool) -> bool {
        // Do not short-circuit: both targets must receive their resume request.
        let wallpaper_ok = self.wallpaper.as_ref().map_or(true, |control| {
            control.resume_and_wait_for_frame(running)
        });
        let windowshader_ok = self.windowshader.as_ref().map_or(true, |control| {
            control.resume_and_wait_for_frame(running)
        });
        if !wallpaper_ok || !windowshader_ok {
            crate::logger::warning(
                &crate::locate_paths::runtime_log_path(),
                &format!("[PRESENTATION] Resume frame unconfirmed: wallpaper={}, windowshader={}", wallpaper_ok, windowshader_ok),
            );
        }
        wallpaper_ok && windowshader_ok
    }

    pub(crate) fn request_policy_reload(&self, config: &crate::load_config::Config) {
        if let Some(control) = &self.wallpaper {
            control.request_policy_reload(WallpaperPolicyReload::from_config(config));
        }
        if let Some(control) = &self.windowshader {
            control.request_policy_reload(WallpaperPolicyReload::from_windowshader_config(config));
        }
    }
}

#[derive(Clone)]
pub struct WallpaperRuntimeControl {
    enabled: bool,
    windowshader: bool,
    active: Arc<AtomicBool>,
    pause_requested: Arc<AtomicBool>,
    pause_acknowledged: Arc<AtomicBool>,
    pause_detected: Arc<AtomicBool>,
    render_checkpoint: Arc<AtomicU8>,
    checkpoint_clock: Arc<Instant>,
    checkpoint_millis: Arc<AtomicU64>,
    resume_frame_ready: Arc<AtomicBool>,
    external_pause_token: Arc<Mutex<Option<String>>>,
    native_ownership_blocked: Arc<AtomicBool>,
    pending_policy_reload:
        Arc<Mutex<Option<WallpaperPolicyReload>>>,
}


impl WallpaperRuntimeControl {

    pub(crate) fn new(
        enabled: bool,
        windowshader: bool,
    ) -> Self {

        Self {
            enabled,
            windowshader,
            active: Arc::new(AtomicBool::new(false)),
            pause_requested: Arc::new(AtomicBool::new(false)),
            pause_acknowledged: Arc::new(AtomicBool::new(false)),
            pause_detected: Arc::new(AtomicBool::new(false)),
            render_checkpoint: Arc::new(AtomicU8::new(0)),
            checkpoint_clock: Arc::new(Instant::now()),
            checkpoint_millis: Arc::new(AtomicU64::new(0)),
            resume_frame_ready: Arc::new(AtomicBool::new(true)),
            external_pause_token: Arc::new(Mutex::new(None)),
            native_ownership_blocked: Arc::new(AtomicBool::new(false)),
            pending_policy_reload:
                Arc::new(Mutex::new(None)),
        }
    }


    // Supervisor-only state transitions; shared pause protocol remains unchanged.
    pub(crate) fn set_supervisor_active(&self, active: bool) {
        self.active.store(active, Ordering::SeqCst);
    }

    pub(crate) fn clear_supervisor_pause(&self) {
        self.pause_requested.store(false, Ordering::SeqCst);
    }

    pub fn request_pause_after_first_frame(
        &self,
        running: &AtomicBool,
    ) -> bool {

        let pipeline = if self.windowshader { "Windowshader" } else { "Wallpaper" };
        let initially_active = self.active.load(Ordering::SeqCst);
        crate::logger::warning(
            &crate::locate_paths::runtime_log_path(),
            &format!(
                "[PRESENTATION_HANDOFF_DIAG] {} pause entry: enabled={}, supervisor_active={}, checkpoint={} ({}), detected={}, acknowledged={}",
                pipeline, self.enabled, initially_active,
                self.render_checkpoint.load(Ordering::SeqCst),
                Self::checkpoint_description(self.render_checkpoint.load(Ordering::SeqCst)),
                self.pause_detected.load(Ordering::SeqCst),
                self.pause_acknowledged.load(Ordering::SeqCst),
            ),
        );
        if !self.enabled || !initially_active {
            return true;
        }


        self.resume_frame_ready.store(
            false,
            Ordering::SeqCst,
        );

        // Reset both signals before publishing the new pause request.
        self.pause_acknowledged.store(false, Ordering::SeqCst);
        self.pause_detected.store(false, Ordering::SeqCst);
        self.pause_requested.store(true, Ordering::SeqCst);

        let started = std::time::Instant::now();
        let detection_deadline = started + Duration::from_millis(500);
        let overall_deadline = started + Duration::from_secs(2);
        crate::logger::warning(
            &crate::locate_paths::runtime_log_path(),
            &format!("[PRESENTATION_HANDOFF_DIAG] {} pause requested (500ms detection, 2000ms overall deadline)", pipeline),
        );

        // Stage 1: confirm the wallpaper renderer has entered its pause path.
        while running.load(Ordering::SeqCst)
            && self.active.load(Ordering::SeqCst)
            && !self.pause_detected.load(Ordering::SeqCst)
            && !self.pause_acknowledged.load(Ordering::SeqCst)
            && std::time::Instant::now() < detection_deadline
        {
            std::thread::sleep(Duration::from_millis(1));
        }

        let detected = self.pause_detected.load(Ordering::SeqCst);
        let early_acknowledged = self.pause_acknowledged.load(Ordering::SeqCst);
        crate::logger::warning(
            &crate::locate_paths::runtime_log_path(),
            &format!(
                "[PRESENTATION_HANDOFF_DIAG] {}: Pause detection stage ended after {}ms: detected={}, acknowledged={}",
                pipeline, started.elapsed().as_millis(), detected, early_acknowledged,
            ),
        );

        if !detected && !early_acknowledged {
            let checkpoint = self.render_checkpoint.load(Ordering::SeqCst);
            let last_ms = self.checkpoint_millis.load(Ordering::SeqCst);
            let checkpoint_age_ms = self.checkpoint_clock.elapsed().as_millis()
                .saturating_sub(last_ms as u128);
            crate::logger::warning(
                &crate::locate_paths::runtime_log_path(),
                &format!(
                    "[PRESENTATION_HANDOFF_DIAG] {}: Pause detection timeout: renderer checkpoint={} ({}), checkpoint_age_ms={}",
                    pipeline, checkpoint,
                    Self::checkpoint_description(checkpoint),
                    checkpoint_age_ms,
                ),
            );
        }

        // Stage 2: a renderer that detected the request may use the
        // remaining budget to acknowledge that it has paused.
        if detected || early_acknowledged {
            while running.load(Ordering::SeqCst)
                && self.active.load(Ordering::SeqCst)
                && !self.pause_acknowledged.load(Ordering::SeqCst)
                && std::time::Instant::now() < overall_deadline
            {
                std::thread::sleep(Duration::from_millis(1));
            }
        }

        let still_active = self.active.load(Ordering::SeqCst);
        let acknowledged = self.pause_acknowledged.load(Ordering::SeqCst);
        let still_running = running.load(Ordering::SeqCst);
        crate::logger::warning(
            &crate::locate_paths::runtime_log_path(),
            &format!(
                "[PRESENTATION_HANDOFF_DIAG] {}: Screensaver pause wait ended after {}ms: detected={}, active={}, acknowledged={}, running={}",
                pipeline, started.elapsed().as_millis(), detected, still_active,
                acknowledged, still_running,
            ),
        );
        // Acknowledgment is authoritative; an inactive wallpaper backend
        // cannot continue presenting frames.
        crate::logger::warning(
            &crate::locate_paths::runtime_log_path(),
            &format!(
                "[PRESENTATION_HANDOFF_DIAG] {} pause result: success={}, supervisor_active={}, checkpoint={} ({}), detected={}, acknowledged={}",
                pipeline, !still_active || acknowledged, still_active,
                self.render_checkpoint.load(Ordering::SeqCst),
                Self::checkpoint_description(self.render_checkpoint.load(Ordering::SeqCst)),
                self.pause_detected.load(Ordering::SeqCst), acknowledged,
            ),
        );
        !still_active || acknowledged
    }

    /// Diagnostic-only, lock-free render-loop checkpoint. Never changes ownership.
    pub fn diagnostic_checkpoint(&self, checkpoint: u8) {
        self.render_checkpoint.store(checkpoint, Ordering::SeqCst);
        self.checkpoint_millis.store(
            self.checkpoint_clock.elapsed().as_millis().min(u64::MAX as u128) as u64,
            Ordering::SeqCst,
        );
    }

    fn checkpoint_description(checkpoint: u8) -> &'static str {
        match checkpoint {
            0 => "not yet in instrumented render loop",
            1 => "processing Wayland events",
            2 => "handling output changes / policy reload",
            3 => "pause checkpoint reached; no pause observed",
            4 => "rendering or GPU sampling",
            5 => "inside eglSwapBuffers",
            6 => "post-presentation processing",
            7 => "frame pacing sleep",
            8 => "paused",
            _ => "unknown",
        }
    }

    /// Called by the wallpaper render thread upon detecting a pause request.
    /// Isolated protocol-harness checkpoint: model an already acknowledged,
    /// native-blocked legacy renderer without starting a compositor or GPU.
    /// Never callable as a production recovery or latch-release operation.
    pub(crate) fn protocol_test_blocked_reauthorization_checkpoint(
        &self, pause_token: &str,
    ) -> Result<(), String> {
        if self.windowshader || !crate::control_wallpaper::protocol_test_runtime_active() {
            return Err("Renderer handshake test requires isolated wallpaper runtime".into());
        }
        *self.external_pause_token.lock()
            .map_err(|_| "Test pause-token mutex poisoned".to_string())? =
            Some(pause_token.to_owned());
        self.pause_acknowledged.store(true, Ordering::SeqCst);
        self.pause_requested.store(true, Ordering::SeqCst);
        self.native_ownership_blocked.store(true, Ordering::SeqCst);
        self.resume_frame_ready.store(false, Ordering::SeqCst);
        if !self.pause_requested() {
            return Err("Blocked renderer unexpectedly exited pause checkpoint".into());
        }
        if !self.native_ownership_blocked.load(Ordering::SeqCst)
            || !self.pause_acknowledged.load(Ordering::SeqCst)
            || self.resume_frame_ready.load(Ordering::SeqCst) {
            return Err("Handshake changed renderer ownership/presentation state".into());
        }
        Ok(())
    }

    // Phase 7K-4C-1: isolated renderer verifies a release proposal but does
    // not clear the ownership latch, retire the journal, or submit a frame.
    pub(crate) fn protocol_test_verify_release_while_blocked(
        &self, pause_token: &str,
    ) -> Result<(), String> {
        if self.windowshader || !crate::control_wallpaper::protocol_test_runtime_active() {
            return Err("Release verification requires isolated wallpaper runtime".into());
        }
        if !self.native_ownership_blocked.load(Ordering::SeqCst)
            || !self.pause_acknowledged.load(Ordering::SeqCst)
            || !self.pause_requested.load(Ordering::SeqCst)
            || self.resume_frame_ready.load(Ordering::SeqCst) {
            return Err("Renderer is not in a verified blocked pause state".into());
        }
        let observed = self.external_pause_token.lock()
            .map_err(|_| "Renderer pause token mutex poisoned".to_string())?.clone();
        if observed.as_deref() != Some(pause_token) {
            return Err("Renderer pause token mismatch".into());
        }
        crate::control_wallpaper::verify_isolated_renderer_release(pause_token)?;
        if !self.pause_requested() || !self.native_ownership_blocked.load(Ordering::SeqCst)
            || self.resume_frame_ready.load(Ordering::SeqCst) {
            return Err("Release verification changed blocked renderer state".into());
        }
        Ok(())
    }

    // Phase 7K-4C-2A: inspect retirement preparation while still blocked.
    // This is deliberately NOT a latch-release or frame-submission operation.
    pub(crate) fn protocol_test_verify_retirement_while_blocked(
        &self, pause_token: &str,
    ) -> Result<(), String> {
        if self.windowshader || !crate::control_wallpaper::protocol_test_runtime_active() {
            return Err("Retirement verification requires isolated wallpaper runtime".into());
        }
        if !self.native_ownership_blocked.load(Ordering::SeqCst)
            || !self.pause_acknowledged.load(Ordering::SeqCst)
            || !self.pause_requested.load(Ordering::SeqCst)
            || self.resume_frame_ready.load(Ordering::SeqCst) {
            return Err("Renderer is not in a verified blocked pause state".into());
        }
        let observed = self.external_pause_token.lock()
            .map_err(|_| "Renderer pause token mutex poisoned".to_string())?.clone();
        if observed.as_deref() != Some(pause_token) {
            return Err("Renderer pause token mismatch".into());
        }
        crate::control_wallpaper::verify_isolated_renderer_retirement_preparation(pause_token)?;
        if !self.pause_requested() || !self.native_ownership_blocked.load(Ordering::SeqCst)
            || !self.pause_acknowledged.load(Ordering::SeqCst)
            || self.resume_frame_ready.load(Ordering::SeqCst) {
            return Err("Retirement verification changed blocked renderer state".into());
        }
        Ok(())
    }

    // Phase 7K-4C-2B: independently observe completed marker withdrawal.
    // A committed retirement is still a blocking fence: never clear the
    // ownership latch or claim a newly submitted frame in this phase.
    pub(crate) fn protocol_test_verify_retirement_committed_while_blocked(
        &self, pause_token: &str,
    ) -> Result<(), String> {
        if self.windowshader || !crate::control_wallpaper::protocol_test_runtime_active() {
            return Err("Retirement completion requires isolated wallpaper runtime".into());
        }
        if !self.native_ownership_blocked.load(Ordering::SeqCst)
            || !self.pause_acknowledged.load(Ordering::SeqCst)
            || !self.pause_requested.load(Ordering::SeqCst)
            || self.resume_frame_ready.load(Ordering::SeqCst) {
            return Err("Renderer is not in verified blocked state".into());
        }
        let observed = self.external_pause_token.lock()
            .map_err(|_| "Renderer pause token mutex poisoned".to_string())?.clone();
        if observed.as_deref() != Some(pause_token) {
            return Err("Renderer pause token mismatch".into());
        }
        crate::control_wallpaper::verify_isolated_renderer_retirement_committed(pause_token)?;
        if !self.native_ownership_blocked.load(Ordering::SeqCst)
            || self.resume_frame_ready.load(Ordering::SeqCst) {
            return Err("Retirement verification changed renderer ownership state".into());
        }
        Ok(())
    }

    // Phase 7K-4C-2C-1: authorization verification only. The renderer
    // retains its ownership latch, pause state, and unconfirmed-frame state.
    pub(crate) fn protocol_test_verify_latch_release_authorization_while_blocked(
        &self, pause_token: &str,
    ) -> Result<(), String> {
        if self.windowshader || !crate::control_wallpaper::protocol_test_runtime_active() {
            return Err("Latch verification requires isolated wallpaper runtime".into());
        }
        if !self.native_ownership_blocked.load(Ordering::SeqCst)
            || !self.pause_acknowledged.load(Ordering::SeqCst)
            || !self.pause_requested.load(Ordering::SeqCst)
            || self.resume_frame_ready.load(Ordering::SeqCst) {
            return Err("Renderer is not in verified blocked state".into());
        }
        let observed = self.external_pause_token.lock()
            .map_err(|_| "Renderer pause token mutex poisoned".to_string())?.clone();
        if observed.as_deref() != Some(pause_token) {
            return Err("Renderer pause identity mismatch".into());
        }
        crate::control_wallpaper::verify_isolated_renderer_latch_release_authorization(pause_token)?;
        if !self.pause_requested() || !self.native_ownership_blocked.load(Ordering::SeqCst)
            || !self.pause_acknowledged.load(Ordering::SeqCst)
            || self.resume_frame_ready.load(Ordering::SeqCst) {
            return Err("Latch verification changed blocked renderer state".into());
        }
        Ok(())
    }

    // Phase 7K-4C-2C-2: isolated transition only. No production call site,
    // no compositor, no GL submission. The global legacy frame gate remains
    // closed by the durable retirement record throughout this test.
    pub(crate) fn protocol_test_release_latch(&self, pause_token: &str) -> Result<(), String> {
        if self.windowshader || !crate::control_wallpaper::protocol_test_runtime_active() {
            return Err("Latch transition requires isolated wallpaper runtime".into());
        }
        if !self.native_ownership_blocked.load(Ordering::SeqCst)
            || !self.pause_acknowledged.load(Ordering::SeqCst)
            || !self.pause_requested.load(Ordering::SeqCst)
            || self.resume_frame_ready.load(Ordering::SeqCst) {
            return Err("Renderer is not in blocked pre-transition state".into());
        }
        let observed = self.external_pause_token.lock()
            .map_err(|_| "Renderer pause token mutex poisoned".to_string())?.clone();
        if observed.as_deref() != Some(pause_token) {
            return Err("Latch transition pause identity mismatch".into());
        }
        crate::control_wallpaper::verify_isolated_latch_transition(pause_token)?;
        // Keep the pause held; the simulated frame confirmation is separate.
        self.native_ownership_blocked.store(false, Ordering::SeqCst);
        if !self.pause_requested.load(Ordering::SeqCst)
            || self.resume_frame_ready.load(Ordering::SeqCst) {
            self.native_ownership_blocked.store(true, Ordering::SeqCst);
            return Err("Renderer state changed during isolated latch transition".into());
        }
        Ok(())
    }

    // Simulated acknowledgment only; does not invoke acknowledge_resumed_frame
    // and does not claim a GPU/compositor frame. Production gate stays closed.
    pub(crate) fn protocol_test_simulated_frame_confirmed(&self, pause_token: &str)
        -> Result<(), String> {
        if self.windowshader || !crate::control_wallpaper::protocol_test_runtime_active() {
            return Err("Simulated confirmation requires isolated wallpaper runtime".into());
        }
        if self.native_ownership_blocked.load(Ordering::SeqCst)
            || !self.pause_requested.load(Ordering::SeqCst)
            || !self.pause_acknowledged.load(Ordering::SeqCst)
            || self.resume_frame_ready.load(Ordering::SeqCst) {
            return Err("Renderer did not complete isolated latch transition".into());
        }
        let observed = self.external_pause_token.lock()
            .map_err(|_| "Renderer pause token mutex poisoned".to_string())?.clone();
        if observed.as_deref() != Some(pause_token) {
            return Err("Simulated frame transaction identity mismatch".into());
        }
        crate::control_wallpaper::verify_isolated_latch_transition(pause_token)?;
        // Explicitly DO NOT mark resume_frame_ready: no real frame was submitted.
        Ok(())
    }

    pub fn acknowledge_pause_detected(&self) {
        // Bind an external request once per pause cycle. Never refresh the
        // identity while the renderer/coordinator is still handling a pause.
        let mut token = self.external_pause_token.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if token.is_none() {
            match if self.windowshader { crate::control_windowshader::detected_pause_token() } else { crate::control_wallpaper::detected_pause_token() } {
                Ok(observed) => *token = observed,
                Err(error) => crate::logger::warning(
                    &crate::locate_paths::runtime_log_path(),
                    &format!("[WALLPAPER_CONTROL] Cannot capture pause identity: {error}"),
                ),
            }
        }
        if !self.windowshader {
            crate::logger::information(
                &crate::locate_paths::runtime_log_path(),
                &format!("[KDE RESUME TRACE] event=pause_token_capture; token={:?}; checkpoint={}",
                    token.as_deref(), self.render_checkpoint.load(Ordering::SeqCst)),
            );
        }
        self.pause_detected.store(true, Ordering::SeqCst);
    }

    pub fn resume_and_wait_for_frame(
        &self,
        running: &AtomicBool,
    ) -> bool {

        if !self.enabled
            || !self.active.load(Ordering::SeqCst)
        {
            return true;
        }


        self.resume_frame_ready.store(
            false,
            Ordering::SeqCst,
        );


        if !self.windowshader && self.native_ownership_blocked.load(Ordering::SeqCst) {
            crate::logger::warning(
                &crate::locate_paths::runtime_log_path(),
                "[WALLPAPER OWNERSHIP] legacy resume refused: native ownership block is latched",
            );
            return false;
        }

        self.pause_requested.store(
            false,
            Ordering::SeqCst,
        );


        let deadline =
            std::time::Instant::now()
                + Duration::from_millis(500);


        while running.load(Ordering::SeqCst)
            && self.active.load(Ordering::SeqCst)
            && !self.resume_frame_ready.load(Ordering::SeqCst)
            && std::time::Instant::now() < deadline
        {
            std::thread::sleep(
                Duration::from_millis(1)
            );
        }

        let confirmed = self.resume_frame_ready.load(Ordering::SeqCst);
        let inactive = !self.active.load(Ordering::SeqCst);
        if !confirmed && !inactive && running.load(Ordering::SeqCst) {
            crate::logger::warning(
                &crate::locate_paths::runtime_log_path(),
                &format!("[PRESENTATION] {} resume wait expired without a confirmed presented frame (500ms)",
                    if self.windowshader { "Windowshader" } else { "Wallpaper" }),
            );
        }
        // Shutdown is not evidence of a presented frame.
        confirmed || inactive
    }


    pub(crate) fn request_policy_reload(
        &self,
        reload: WallpaperPolicyReload,
    ) {
        match self.pending_policy_reload.lock() {
            Ok(mut pending) => {
                *pending = Some(reload);
            }
            Err(poisoned) => {
                let mut pending = poisoned.into_inner();
                *pending = Some(reload);
            }
        }
    }


    pub(crate) fn take_policy_reload(
        &self,
    ) -> Option<WallpaperPolicyReload> {
        match self.pending_policy_reload.lock() {
            Ok(mut pending) => pending.take(),
            Err(poisoned) => poisoned.into_inner().take(),
        }
    }


    pub fn pause_requested(
        &self,
    ) -> bool {

        // Once a native grant is observed (or inspection fails), latch the
        // legacy wallpaper into pause until its supervisor is restarted and
        // independently re-authorized. Never auto-resume on file disappearance.
        let native_blocked = if self.windowshader {
            false
        } else if self.native_ownership_blocked.load(Ordering::SeqCst) {
            true
        } else {
            match crate::control_wallpaper::legacy_frame_gate() {
                Ok(()) => false,
                Err(error) => {
                    if !self.native_ownership_blocked.swap(true, Ordering::SeqCst) {
                        crate::logger::error(
                            &crate::locate_paths::runtime_log_path(),
                            &format!("[WALLPAPER OWNERSHIP] legacy renderer latched paused: {error}; manual verified recovery required"),
                        );
                    }
                    true
                }
            }
        };

        // Recovery handshake observation is safe only while the renderer is
        // already paused and the ownership latch remains engaged. The ACK is
        // not a latch release and never changes frame-submission permission.
        if native_blocked && !self.windowshader
            && self.pause_acknowledged.load(Ordering::SeqCst)
        {
            let token = self.external_pause_token.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()).clone();
            if let Some(token) = token {
                if let Err(error) = crate::control_wallpaper::acknowledge_reauthorization_while_paused(&token) {
                    crate::logger::warning(
                        &crate::locate_paths::runtime_log_path(),
                        &format!("[WALLPAPER OWNERSHIP] reauthorization ACK refused: {error}"),
                    );
                }
            }
        }

        native_blocked || self.pause_requested.load(
            Ordering::SeqCst
        )
            || if self.windowshader { crate::control_windowshader::external_pause_requested() } else { crate::control_wallpaper::external_pause_requested() }
    }


    pub fn acknowledge_paused(
        &self,
    ) {

        self.pause_acknowledged.store(
            true,
            Ordering::SeqCst,
        );


        let token = self.external_pause_token.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()).clone();
        if !self.windowshader {
            crate::logger::information(&crate::locate_paths::runtime_log_path(),
                &format!("[KDE RESUME TRACE] event=paused_callback; token={token:?}; checkpoint={}",
                    self.render_checkpoint.load(Ordering::SeqCst)));
        }
        if let Some(token) = token {
            if self.windowshader { crate::control_windowshader::acknowledge_paused_for(&token); } else { crate::control_wallpaper::acknowledge_paused_for(&token); }
        }
    }


    pub fn acknowledge_resumed_frame(
        &self,
    ) {

        self.pause_acknowledged.store(
            false,
            Ordering::SeqCst,
        );


        self.resume_frame_ready.store(
            true,
            Ordering::SeqCst,
        );


        let token = self.external_pause_token.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()).take();
        if !self.windowshader {
            crate::logger::information(&crate::locate_paths::runtime_log_path(),
                &format!("[KDE RESUME TRACE] event=resumed_frame_callback; token={token:?}; checkpoint={}",
                    self.render_checkpoint.load(Ordering::SeqCst)));
        }
        if let Some(token) = token {
            if self.windowshader { crate::control_windowshader::acknowledge_resumed_frame_for(&token); } else { crate::control_wallpaper::acknowledge_resumed_frame_for(&token); }
        }
    }
}


pub struct WallpaperRuntimeManager {
    running: Arc<AtomicBool>,
    control: WallpaperRuntimeControl,
    thread: Option<JoinHandle<()>>,
    ownership_observer: Option<JoinHandle<()>>,
    ownership_events: Option<Sender<WallpaperOwnershipEvent>>,
}


impl WallpaperRuntimeManager {

    pub fn start(
        enabled: bool,
        configured_mode: String,
        runtime: crate::define_wallpaper::WallpaperRuntime,
        logfile: PathBuf,
        running: Arc<AtomicBool>,
        tray_status: crate::tray_icon::TrayStatusControl,
    ) -> Self {
        Self::start_wallpaper(enabled, configured_mode, runtime, logfile, running, tray_status)
    }

    fn start_wallpaper(
        enabled: bool,
        configured_mode: String,
        runtime: crate::define_wallpaper::WallpaperRuntime,
        logfile: PathBuf,
        running: Arc<AtomicBool>,
        tray_status: crate::tray_icon::TrayStatusControl,
    ) -> Self {

        let control =
            WallpaperRuntimeControl::new(
                enabled,
                false,
            );


        if !enabled {

            crate::logger::information(
                &logfile,
                &crate::manage_localization::runtime_text("wallpaper.runtime.disabled_by_config"),
            );


            return Self {
                running,
                control,
                thread: None,
                ownership_observer: None,
                ownership_events: None,
            };
        }


        // Plasma full-screen wallpapers are presented inside Plasma itself.
        // Never enter the legacy Wayland layer-shell/native-grant path here.
        let kde_plasma = runtime.display_format
            == crate::manage_configuration::WallpaperDisplayFormat::FullScreen
            && std::env::var("XDG_SESSION_TYPE")
                .map(|v| v.eq_ignore_ascii_case("wayland")).unwrap_or(false)
            && std::env::var("XDG_CURRENT_DESKTOP")
                .map(|v| v.split(':').any(|part| part.eq_ignore_ascii_case("KDE")))
                .unwrap_or(false);
        crate::logger::information(&logfile, &format!(
            "[KDE WALLPAPER STARTUP] enabled={enabled} format={:?} session={} desktop={} kde_branch={kde_plasma}",
            runtime.display_format,
            std::env::var("XDG_SESSION_TYPE").unwrap_or_default(),
            std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),
        ));
        if kde_plasma {
            let thread_running = Arc::clone(&running);
            let thread_logfile = logfile.clone();
            let thread_control = control.clone();
            let thread_tray = tray_status.clone();
            let thread = std::thread::Builder::new()
                .name("screenshaver-kde-wallpaper".to_owned())
                .spawn(move || {
                    if let Err(error) = run_kde_plasma_wallpaper(
                        &thread_running, &thread_control, &thread_logfile, &thread_tray,
                    ) {
                        thread_tray.set_wallpaper_disabled();
                        crate::logger::error(&thread_logfile,
                            &format!("[KDE WALLPAPER] Independent producer unavailable: {error}"));
                    }
                });
            return Self {
                running, control,
                thread: thread.ok(),
                ownership_observer: None,
                ownership_events: None,
            };
        }

        crate::logger::information(
            &logfile,
            &crate::manage_localization::runtime_text("wallpaper.runtime.starting"),
        );


        let thread_running =
            Arc::clone(
                &running
            );


        let thread_logfile =
            logfile.clone();


        let thread_control =
            control.clone();


        // Install the receiver before starting the supervisor. An unbounded
        // channel keeps the render thread independent of observer logging.
        let (ownership_sender, ownership_receiver) = mpsc::channel();
        let thread_ownership_sender = ownership_sender.clone();
        let thread =
            std::thread::Builder::new()
                .name(
                    "screenshaver-wallpaper".to_string()
                )
                .spawn(
                    move || {

                        let logfile =
                            thread_logfile;


                        // Do not advertise an active legacy renderer before
                        // verifying the persisted native authorization and handoff journal.
                        // This is an entry gate, NOT a live handoff protocol.
                        let startup_assessment = crate::control_wallpaper::assess_abandoned_handoff();
                        let startup_decision = decide_wallpaper_ownership(&startup_assessment);
                        if startup_decision != SupervisorOwnershipDecision::LegacyEligible {
                            crate::logger::error(&logfile, &format!(
                                "[WALLPAPER OWNERSHIP] legacy startup BLOCKED; handoff_evidence={startup_assessment:?}; decision={startup_decision:?}; recovery must be verified externally; no marker removed"
                            ));
                            crate::logger::warning(&logfile,
                                &crate::control_wallpaper::assess_production_handoff_recovery());
                            let _ = thread_ownership_sender.send(WallpaperOwnershipEvent::SupervisorStarted);
                            // The replacement Plasma process cannot expose an
                            // ownership monitor until its Screenshaver wallpaper
                            // component has been selected. Keep this guard alive
                            // for the entire blocked recovery attempt so its
                            // original wallpaper selection is restored on exit.
                            // Never start the legacy renderer in this branch.
                            let kde_recovery_session = runtime.display_format
                                == crate::manage_configuration::WallpaperDisplayFormat::FullScreen
                                && std::env::var("XDG_SESSION_TYPE")
                                    .map(|value| value.eq_ignore_ascii_case("wayland"))
                                    .unwrap_or(false)
                                && std::env::var("XDG_CURRENT_DESKTOP")
                                    .map(|value| value.split(':').any(|part| part.eq_ignore_ascii_case("KDE")))
                                    .unwrap_or(false);
                            if kde_recovery_session {
                                match crate::manage_wallpaper_kde::KdeWallpaperIntegrationGuard::activate() {
                                    Ok(_recovery_guard) => {
                                        crate::logger::information(&logfile,
                                            "[KDE NATIVE COORDINATOR] recovery wallpaper selection verified; replacing dead Plasma generation with legacy paused");
                                        recover_native_wallpaper_while_legacy_blocked(&thread_running, &logfile);
                                    }
                                    Err(error) => crate::logger::error(&logfile, &format!(
                                        "[KDE NATIVE COORDINATOR] recovery wallpaper activation unverified; legacy remains blocked: {error}")),
                                }
                            }
                            let _ = thread_ownership_sender.send(WallpaperOwnershipEvent::SupervisorStopped);
                            return;
                        }

                        // KDE Plasma full-screen wallpaper uses Plasma's own wallpaper
                        // component. Other Wayland/X11 desktops and windowed modes
                        // retain the existing external renderer path.
                        // Keep this guard alive until the coordinator and renderer
                        // have stopped; its Drop restores the original selections.
                        let kde_native_session = runtime.display_format
                            == crate::manage_configuration::WallpaperDisplayFormat::FullScreen
                            && std::env::var("XDG_SESSION_TYPE")
                                .map(|value| value.eq_ignore_ascii_case("wayland"))
                                .unwrap_or(false)
                            && std::env::var("XDG_CURRENT_DESKTOP")
                                .map(|value| value.split(':').any(|part| part.eq_ignore_ascii_case("KDE")))
                                .unwrap_or(false);
                        let _kde_wallpaper_guard = if kde_native_session {
                            match crate::manage_wallpaper_kde::KdeWallpaperIntegrationGuard::activate() {
                                Ok(guard) => {
                                    crate::logger::information(&logfile,
                                        "[KDE WALLPAPER] Plasma wallpaper selection activated; native monitor discovery may proceed");
                                    Some(guard)
                                }
                                Err(error) => {
                                    crate::logger::error(&logfile, &format!(
                                        "[KDE WALLPAPER] Plasma wallpaper activation failed; refusing competing legacy overlay: {error}"
                                    ));
                                    let _ = thread_ownership_sender.send(WallpaperOwnershipEvent::SupervisorStarted);
                                    let _ = thread_ownership_sender.send(WallpaperOwnershipEvent::SupervisorStopped);
                                    return;
                                }
                            }
                        } else {
                            None
                        };

                        thread_control.active.store(
                            true,
                            Ordering::SeqCst,
                        );


                        crate::control_wallpaper::set_runtime_active(true);
                        let _ = thread_ownership_sender.send(WallpaperOwnershipEvent::SupervisorStarted);

                        // Phase 3B-3E-10B: diagnostic-only ownership model.
                        // This is a local, unmodified initial state, NOT proof
                        // of renderer quiescence or exclusive ownership.
                        let ownership_observation =
                            crate::manage_wallpaper_ownership::WallpaperOwnership::new();
                        crate::logger::information(
                            &logfile,
                            &format!(
                                "[WALLPAPER OWNERSHIP DIAG] supervisor started; model={:?}; native_handoff=disabled; ownership_unverified",
                                ownership_observation.state(),
                            ),
                        );


                        let mut ownership_blocked = false;
                        let mut native_activation = NativeActivationState::Inactive;
                        for attempt in
                            1..=MAX_RESTART_ATTEMPTS
                        {
                            if !thread_running.load(
                                Ordering::SeqCst
                            ) {
                                break;
                            }


                            // Re-evaluate after restart delays. A stale, current,
                            // malformed, or unreadable native grant must not be
                            // bypassed merely because the previous attempt ended.
                            let assessment = crate::control_wallpaper::assess_abandoned_handoff();
                            let decision = decide_wallpaper_ownership(&assessment);
                            if decision != SupervisorOwnershipDecision::LegacyEligible {
                                crate::logger::error(&logfile, &format!(
                                    "[WALLPAPER OWNERSHIP] legacy attempt={attempt} BLOCKED; handoff_evidence={assessment:?}; decision={decision:?}; no automatic recovery or grant removal"
                                ));
                                ownership_blocked = true;
                                break;
                            }

                            crate::logger::information(
                                &logfile,
                                &format!(
                                    "[WALLPAPER OWNERSHIP DIAG] attempt={}/{}; model={:?}; native_handoff=disabled; no_grant_issued",
                                    attempt,
                                    MAX_RESTART_ATTEMPTS,
                                    ownership_observation.state(),
                                ),
                            );

                            let _ = thread_ownership_sender.send(WallpaperOwnershipEvent::AttemptStarted(attempt));
                            // Phase 3B-3E-10C: correlate independent, read-only
                            // observations before this rendering attempt.  A
                            // supervisor-active flag is not frame confirmation;
                            // a Plasma readiness marker is not an ownership grant.
                            let full_screen_wallpaper =
                                runtime.display_format
                                    == crate::manage_configuration::WallpaperDisplayFormat::FullScreen;
                            let plasma_observation =
                                crate::wallpaper_backend::assess_plasma_wallpaper(
                                    full_screen_wallpaper,
                                );
                            // Read-only coordinator boundary. A candidate is
                            // logged for future activation wiring, never granted.
                            // The existing legacy renderer continues unchanged.
                            let native_preflight =
                                observe_native_activation_preflight(full_screen_wallpaper);
                            native_activation.observe(attempt, &native_preflight);
                            crate::logger::information(&logfile, &format!(
                                "[KDE NATIVE HANDOFF PREFLIGHT] attempt={attempt}; result={native_preflight:?}; activation_state={}; grant_published=false; legacy_pause_requested=false",
                                native_activation.description(),
                            ));
                            let supervisor_active =
                                thread_control.active.load(Ordering::SeqCst);
                            crate::logger::information(
                                &logfile,
                                &format!(
                                    "[WALLPAPER OWNERSHIP DIAG] correlation attempt={}/{}; supervisor_active={}; plasma={:?}; model={:?}; native_authorized=false; handoff=disabled; frame_delivery_unverified",
                                    attempt,
                                    MAX_RESTART_ATTEMPTS,
                                    supervisor_active,
                                    plasma_observation,
                                    ownership_observation.state(),
                                ),
                            );

                            let attempt_running =
                                Arc::clone(
                                    &thread_running
                                );


                            // Last supervisor-side gate immediately before the
                            // rendering entry point. Render-loop checkpoints
                            // independently enforce the latched runtime gate.
                            if let Err(error) = crate::control_wallpaper::legacy_frame_gate() {
                                crate::logger::error(&logfile, &format!(
                                    "[WALLPAPER OWNERSHIP] attempt={attempt} cancelled at final entry gate: {error}"
                                ));
                                ownership_blocked = true;
                                break;
                            }

                            // The observation was valid only at this boundary.
                            // Never carry it into a subsequent restart or use
                            // it to authorize native presentation.
                            native_activation.invalidate();
                            // The legacy rendering call below blocks. Keep a
                            // separate, attempt-scoped observer alive during
                            // that call, and join it before any restart.
                            let coordinator_stop = Arc::new(AtomicBool::new(false));
                            let coordinator_running = Arc::clone(&thread_running);
                            let coordinator_stop_worker = Arc::clone(&coordinator_stop);
                            let coordinator_logfile = logfile.clone();
                            let coordinator = std::thread::Builder::new()
                                .name("screenshaver-kde-coordinator".to_owned())
                                .spawn(move || observe_native_activation_during_attempt(
                                    attempt,
                                    full_screen_wallpaper,
                                    coordinator_running,
                                    coordinator_stop_worker,
                                    coordinator_logfile,
                                ));
                            if let Err(error) = &coordinator {
                                crate::logger::warning(&logfile, &format!(
                                    "[KDE NATIVE COORDINATOR] observer unavailable: {error}; legacy rendering unchanged"
                                ));
                            }
                            let result =
                                std::panic::catch_unwind(
                                    std::panic::AssertUnwindSafe(
                                        || {
                                            crate::manage_wallpaper::run(
                                                &configured_mode,
                                                &runtime,
                                                attempt_running,
                                                thread_control.clone(),
                                            )
                                        }
                                    )
                                );


                            // No observer may survive its rendering attempt.
                            coordinator_stop.store(true, Ordering::SeqCst);
                            if let Ok(handle) = coordinator {
                                if handle.join().is_err() {
                                    crate::logger::warning(&logfile,
                                        "[KDE NATIVE COORDINATOR] coordinator panicked; grant publication and withdrawal status UNKNOWN; durable ownership evidence must be verified; legacy resume forbidden");
                                }
                            }

                            match result {

                                Ok(Ok(())) => {
                                    let _ = thread_ownership_sender.send(WallpaperOwnershipEvent::AttemptEnded { attempt, outcome: "clean" });

                                    crate::logger::information(
                                        &logfile,
                                        &crate::manage_localization::runtime_text("wallpaper.runtime.stopped_cleanly"),
                                    );


                                    thread_control.active.store(
                                        false,
                                        Ordering::SeqCst,
                                    );


                                    crate::control_wallpaper::set_runtime_active(false);
                                    let _ = thread_ownership_sender.send(WallpaperOwnershipEvent::SupervisorStopped);
                                    return;
                                }


                                Ok(Err(error)) => {
                                    let _ = thread_ownership_sender.send(WallpaperOwnershipEvent::AttemptEnded { attempt, outcome: "error" });

                                    crate::logger::error(
                                        &logfile,
                                        &crate::manage_localization::runtime_text_with_params(
                                            "wallpaper.runtime.attempt_failed",
                                            &[
                                                ("attempt", &attempt.to_string()),
                                                ("maximum", &MAX_RESTART_ATTEMPTS.to_string()),
                                                ("error", &error.to_string()),
                                            ],
                                        ),
                                    );
                                }


                                Err(_) => {
                                    let _ = thread_ownership_sender.send(WallpaperOwnershipEvent::AttemptEnded { attempt, outcome: "panic" });

                                    crate::logger::error(
                                        &logfile,
                                        &crate::manage_localization::runtime_text_with_params(
                                            "wallpaper.runtime.attempt_panicked",
                                            &[
                                                ("attempt", &attempt.to_string()),
                                                ("maximum", &MAX_RESTART_ATTEMPTS.to_string()),
                                            ],
                                        ),
                                    );
                                }
                            }


                            if attempt
                                == MAX_RESTART_ATTEMPTS
                                || !thread_running.load(
                                    Ordering::SeqCst
                                )
                            {
                                break;
                            }


                            std::thread::sleep(
                                Duration::from_secs(
                                    RESTART_DELAY_SECONDS
                                )
                            );
                        }


                        if thread_running.load(
                            Ordering::SeqCst
                        ) && !ownership_blocked {

                            crate::logger::error(
                                &logfile,
                                &crate::manage_localization::runtime_text("wallpaper.runtime.disabled_after_failures"),
                            );
                        }


                        thread_control.active.store(
                            false,
                            Ordering::SeqCst,
                        );


                        crate::control_wallpaper::set_runtime_active(false);
                        let _ = thread_ownership_sender.send(WallpaperOwnershipEvent::SupervisorStopped);
                    }
                )
                .map_err(
                    |error| {
                        crate::logger::error(
                            &logfile,
                            &crate::manage_localization::runtime_text_with_params(
                                "wallpaper.runtime.thread_create_failed",
                                &[
                                    ("error", &error.to_string()),
                                ],
                            ),
                        );


                        error
                    }
                )
                .ok();


        // Lifecycle receiver never pauses, grants, or changes ownership.
        let ownership_observer = if thread.is_some() {
            let observer_logfile = logfile.clone();
            match std::thread::Builder::new()
                .name("screenshaver-wallpaper-ownership".to_string())
                .spawn(move || observe_wallpaper_ownership(ownership_receiver, observer_logfile)) {
                Ok(handle) => Some(handle),
                Err(error) => {
                    crate::logger::warning(&logfile,
                        &format!("[WALLPAPER OWNERSHIP] lifecycle observer unavailable: {error}"));
                    None
                }
            }
        } else { None };

        let ownership_events = if ownership_observer.is_some() { Some(ownership_sender) } else { None };
        Self {
            running,
            control,
            thread,
            ownership_observer,
            ownership_events,
        }
    }


    pub fn control(
        &self,
    ) -> WallpaperRuntimeControl {

        self.control.clone()
    }


    pub fn stop_and_join(
        &mut self,
    ) {

        self.control.pause_requested.store(
            false,
            Ordering::SeqCst,
        );


        self.running.store(
            false,
            Ordering::SeqCst,
        );


        if let Some(thread) =
            self.thread.take()
        {
            if thread.join().is_err() {

                eprintln!(
                    "{}",
                    crate::manage_localization::runtime_text(
                        "wallpaper.runtime.supervisor_panicked_shutdown"
                    )
                );
            }
        }


        // The sender is dropped only after the supervisor is joined; the
        // receiver drains every queued event before returning.
        self.ownership_events.take();
        if let Some(observer) = self.ownership_observer.take() {
            if observer.join().is_err() {
                crate::logger::warning(
                    &crate::locate_paths::runtime_log_path(),
                    "[WALLPAPER OWNERSHIP] read-only observer panicked during shutdown",
                );
            }
        }

        crate::control_wallpaper::set_runtime_active(false);
    }
}


impl Drop for WallpaperRuntimeManager {

    fn drop(
        &mut self,
    ) {
        self.stop_and_join();
    }
}


/// Dedicated Plasma presentation lifecycle. Producer starts before selection;
/// the original Plasma wallpaper is restored before producer termination.
fn run_kde_plasma_wallpaper(
    running: &AtomicBool,
    control: &WallpaperRuntimeControl,
    logfile: &std::path::Path,
    tray_status: &crate::tray_icon::TrayStatusControl,
) -> Result<(), String> {
    use std::time::{Duration, Instant};
    let settings = crate::kde_producer_supervisor::ProducerSettings::from_environment()?;
    crate::logger::information(logfile, &format!(
        "[KDE WALLPAPER STARTUP] producer={} renderer={} geometry={}x{} fps={}",
        settings.executable.display(), settings.library.display(),
        settings.width, settings.height, settings.fps,
    ));
    let mut producer = crate::kde_producer_supervisor::ProducerSupervisor::start(&settings)?;
    crate::logger::information(logfile, "[KDE WALLPAPER STARTUP] producer published frames; activating Plasma wallpaper");
    let runtime_dir = settings.frame_file.parent().ok_or("Frame file has no parent")?;
    let ack_path = runtime_dir.join("screenshaver-plasma-presenter.ready");
    // An old ACK cannot be accepted for the new producer inode. Removing it
    // is unnecessary and could interfere with another Plasma instance.
    let mut accepted_sequence = None;
    let mut accepted_pid = None;
    let guard = crate::manage_wallpaper_kde::KdeWallpaperIntegrationGuard::activate_with_readiness(|| {
        let deadline = Instant::now() + Duration::from_secs(12);
        let mut reason = "No presenter ACK received".to_owned();
        while running.load(Ordering::SeqCst) && Instant::now() < deadline {
            match verify_live_plasma_presenter(
                &ack_path, producer.frame_file(), accepted_pid,
                accepted_sequence, Duration::from_secs(3),
            ) {
                Ok(ack) => {
                    accepted_sequence = Some(ack.sequence);
                    accepted_pid = Some(ack.pid);
                    return Ok(());
                }
                Err(error) => reason = error,
            }
            std::thread::sleep(Duration::from_millis(80));
        }
        Err(format!("Plasma presenter did not acknowledge the producer: {reason}"))
    })?;
    control.active.store(true, Ordering::SeqCst);
    crate::logger::information(logfile,
        "[KDE WALLPAPER] Plasma presenter acknowledged independent producer");
    let mut guard = guard;
    let mut failure = None;
    let mut last_reported_policy: Option<crate::kde_producer_supervisor::ActiveProducerPolicy> = None;
    // Initial state is published only after Plasma has acknowledged frames.
    // Subsequent metadata originates from actual accepted renderer policies.
    if let Some(policy) = producer.active_policy() {
        let path = policy.shader_path.clone().unwrap_or_else(||
            crate::locate_paths::shader_dir().join(&policy.shader_name));
        tray_status.set_wallpaper_active(policy.policy_id, policy.policy_name.clone(), path);
        last_reported_policy = Some(policy);
    }
    // KDE owns its producer directly: never consult the legacy Wayland
    // frame/ownership gate. Observe the same external pause request that
    // the other wallpaper renderer uses, and acknowledge only completed
    // producer transitions (not merely requests).
    let mut producer_paused = false;
    // ACKs are rate-limited by the presenter to once per second. Allow a
    // bounded interval for low frame rates and scheduling jitter.
    let mut last_presenter_progress = Instant::now();
    while running.load(Ordering::SeqCst) {
        match producer.is_alive() {
            Ok(true) => {}
            Ok(false) => { failure = Some("KDE producer exited unexpectedly".to_owned()); break; }
            Err(e) => { failure = Some(e); break; }
        }
        let should_pause = control.pause_requested.load(Ordering::SeqCst)
            || crate::control_wallpaper::external_pause_requested();
        if should_pause && !producer_paused {
            control.acknowledge_pause_detected();
            match producer.pause(Duration::from_millis(1200)) {
                Ok(sequence) => {
                    producer_paused = true;
                    control.acknowledge_paused();
                    crate::logger::information(logfile, &format!(
                        "[KDE WALLPAPER] Producer PAUSED at frame {sequence}"));
                }
                Err(error) => {
                    failure = Some(format!("KDE producer pause failed: {error}"));
                    break;
                }
            }
        } else if !should_pause && producer_paused {
            match producer.resume(Duration::from_millis(1200)) {
                Ok(sequence) => {
                    producer_paused = false;
                    // A producer-side frame alone is insufficient: require
                    // Plasma to consume a later frame before acknowledging
                    // the wallpaper's resumed presentation.
                    let deadline = Instant::now() + Duration::from_secs(5);
                    let mut confirmed = false;
                    while running.load(Ordering::SeqCst) && Instant::now() < deadline {
                        if let Ok(ack) = verify_live_plasma_presenter(
                            &ack_path, producer.frame_file(), accepted_pid,
                            accepted_sequence, Duration::from_secs(3),
                        ) {
                            accepted_sequence = Some(ack.sequence);
                            accepted_pid = Some(ack.pid);
                            last_presenter_progress = Instant::now();
                            confirmed = true;
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(80));
                    }
                    if !confirmed {
                        failure = Some("Plasma presenter did not confirm a resumed frame".into());
                        break;
                    }
                    control.acknowledge_resumed_frame();
                    crate::logger::information(logfile, &format!(
                        "[KDE WALLPAPER] Producer RESUMED at frame {sequence}"));
                }
                Err(error) => {
                    failure = Some(format!("KDE producer resume failed: {error}"));
                    break;
                }
            }
        }
        // Consume pending wallpaper policy changes in the KDE-owned runtime.
        // The renderer's RELOAD is transactional: on rejection it retains the
        // previous engine, so report the failure and continue presenting it.
        // This is valid while paused: no frame publication is required until
        // the next successful RESUME.
        if control.take_policy_reload().is_some() {
            match producer.reload(Duration::from_secs(8)) {
                Ok(sequence) => {
                    crate::logger::information(logfile, &format!(
                        "[KDE WALLPAPER] Policy RELOADED at frame {sequence}; paused={producer_paused}"));
                    // The existing presentation-progress timer is not reset:
                    // an ACK is still required from Plasma during rendering.
                }
                Err(error) => {
                    crate::logger::warning(logfile, &format!(
                        "[KDE WALLPAPER] Policy reload rejected; previous renderer retained: {error}"));
                }
            }
        }
        // A paused producer intentionally has no advancing ACK sequence.
        // On an unpaused producer, continued ACK advancement is mandatory.
        if !producer_paused {
            if let Ok(ack) = verify_live_plasma_presenter(
                &ack_path, producer.frame_file(), accepted_pid,
                accepted_sequence, Duration::from_secs(6),
            ) {
                accepted_sequence = Some(ack.sequence);
                accepted_pid = Some(ack.pid);
                last_presenter_progress = Instant::now();
            }
            if last_presenter_progress.elapsed() > Duration::from_secs(6) {
                failure = Some("Plasma presenter stopped acknowledging new frames".into());
                break;
            }
        } else {
            // Do not penalize the presenter for a deliberate pause, including
            // a long-running screensaver. Resume must establish fresh progress.
            last_presenter_progress = Instant::now();
        }
        // Publish only while a live presenter has been confirmed. Never use
        // the producer's policy selection to infer Plasma ownership.
        if !producer_paused && last_presenter_progress.elapsed() <= Duration::from_secs(6) {
            if let Some(policy) = producer.active_policy() {
                if last_reported_policy.as_ref() != Some(&policy) {
                    let path = policy.shader_path.clone().unwrap_or_else(||
                        crate::locate_paths::shader_dir().join(&policy.shader_name));
                    tray_status.set_wallpaper_active(policy.policy_id, policy.policy_name.clone(), path);
                    last_reported_policy = Some(policy);
                }
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    // Critical order: keep last published frame available while Plasma
    // restores its previous wallpaper plugin.
    let restoration = guard.deactivate();
    control.active.store(false, Ordering::SeqCst);
    tray_status.set_wallpaper_disabled();
    producer.stop();
    restoration?;
    if let Some(error) = failure { return Err(error); }
    Ok(())
}

/// Validate a real ACK against the currently running, same-user Plasma
/// process. The PID is selected from the ACK, never guessed from /proc order.
/// Once a session is accepted, changing its presenter PID is forbidden.
fn verify_live_plasma_presenter(
    ack_path: &std::path::Path,
    frame_path: &std::path::Path,
    accepted_pid: Option<u32>,
    previous_sequence: Option<u64>,
    max_age: std::time::Duration,
) -> Result<crate::kde_presenter_ack::PresenterAck, String> {
    let bytes = std::fs::read(ack_path)
        .map_err(|e| format!("Read Plasma presenter ACK: {e}"))?;
    let candidate = crate::kde_presenter_ack::PresenterAck::parse(&bytes)?;
    if accepted_pid.is_some_and(|pid| pid != candidate.pid) {
        return Err("Plasma presenter PID changed during active session".into());
    }
    if !is_same_user_plasmashell(candidate.pid)? {
        return Err("Presenter ACK PID is not a live same-user plasmashell".into());
    }
    crate::kde_presenter_ack::verify(
        ack_path, frame_path, candidate.pid, previous_sequence, max_age,
    )
}

fn is_same_user_plasmashell(pid: u32) -> Result<bool, String> {
    let current = std::fs::read_to_string("/proc/self/status")
        .map_err(|e| format!("Read own process status: {e}"))?;
    let expected_uid = process_real_uid(&current).ok_or("Cannot determine current UID")?;
    let path = std::path::PathBuf::from(format!("/proc/{pid}"));
    // /proc/<pid>/comm is a mutable thread name, not an executable identity.
    // The Qt wallpaper presenter executes in Plasma's process, so verify its
    // actual executable instead of requiring a particular thread name.
    let Ok(executable) = std::fs::read_link(path.join("exe")) else {
        return Ok(false);
    };
    // NixOS wraps Plasma executables; /proc/<pid>/exe then resolves to
    // .plasmashell-wrapped rather than plasmashell. Accept only these two
    // explicit executable basenames, not arbitrary processes with Plasma-like
    // thread names or command-line arguments.
    let executable_name = executable.file_name().and_then(|name| name.to_str());
    if !matches!(executable_name, Some("plasmashell" | ".plasmashell-wrapped")) {
        return Ok(false);
    }
    let Ok(status) = std::fs::read_to_string(path.join("status")) else {
        return Ok(false);
    };
    Ok(process_real_uid(&status) == Some(expected_uid))
}

fn process_real_uid(status: &str) -> Option<u32> {
    status.lines().find_map(|line| {
        line.strip_prefix("Uid:")?
            .split_whitespace().next()?.parse::<u32>().ok()
    })
}
