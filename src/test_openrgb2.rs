//! OpenRGB2 compatibility and explicitly authorized Cynosa initialization probe.
//! Without --apply, no device writes or mode changes are issued.
//! With --apply, restoration uses Off -> Spectrum Cycle and requires physical confirmation.
use openrgb2::OpenRgbClient;
use std::time::{Duration, Instant};

const ENDPOINT: &str = "127.0.0.1:6742";
const PROTOCOL: u32 = 6;
const TIMEOUT: Duration = Duration::from_secs(5);

pub fn run(apply: bool) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("Tokio runtime initialization: {e}"))?;
    runtime.block_on(probe(apply))
}

async fn probe(apply: bool) -> Result<(), String> {
    println!("[OPENRGB2 TEST] Compatibility probe; write mode explicitly gated by --apply.");
    println!("[OPENRGB2 TEST] Endpoint={ENDPOINT} requested SDK protocol={PROTOCOL}");
    let start = Instant::now();
    let mut client = tokio::time::timeout(
        TIMEOUT,
        OpenRgbClient::connect_to(ENDPOINT, PROTOCOL),
    )
    .await
    .map_err(|_| "Connection/protocol negotiation timed out after 5s".to_string())?
    .map_err(|e| format!("Connection/protocol negotiation failed: {e}"))?;
    println!(
        "[OPENRGB2 TEST] Connected in {:?}; client protocol={}",
        start.elapsed(), client.get_protocol_version()
    );
    if client.get_protocol_version() != PROTOCOL {
        return Err(format!(
            "Unexpected negotiated client protocol {}; expected {PROTOCOL}",
            client.get_protocol_version()
        ));
    }
    let started = Instant::now();
    let controllers = tokio::time::timeout(TIMEOUT, client.get_all_controllers())
        .await
        .map_err(|_| "Controller enumeration timed out after 5s".to_string())?
        .map_err(|e| format!("Controller enumeration failed: {e}"))?;
    println!("[OPENRGB2 TEST] Enumerated {} controller(s) in {:?}", controllers.len(), started.elapsed());
    let mut cynosa_count = 0usize;
    for controller in controllers.iter() {
        println!(
            "[OPENRGB2 TEST] Controller id={} name={:?} vendor={:?} serial={:?} location={:?} LEDs={} modes={}",
            controller.id(), controller.name(), controller.vendor(),
            controller.serial(), controller.location(), controller.num_leds(),
            controller.modes().len()
        );
        if controller.name() == "Razer Cynosa Chroma" {
            cynosa_count += 1;
            println!("[OPENRGB2 TEST] Cynosa active mode: {}", controller.active_mode().name());
            for (index, mode) in controller.modes().iter().enumerate() {
                println!("[OPENRGB2 TEST] Cynosa mode[{index}]: {}", mode.name());
            }
            if controller.num_leds() != 132 {
                return Err(format!("Cynosa LED count changed: {} (expected 132)", controller.num_leds()));
            }
        }
    }
    if cynosa_count != 1 {
        return Err(format!("Expected exactly one Razer Cynosa Chroma; found {cynosa_count}"));
    }
    if apply {
        // Consume the group to obtain an owned Controller. The refresh API
        // requires &mut self, which an iterator of borrowed controllers cannot provide.
        let mut controller = controllers.into_iter()
            .find(|c| c.name() == "Razer Cynosa Chroma")
            .ok_or_else(|| "Cynosa disappeared".to_string())?;
        let baseline = controller.active_mode().name().to_string();
        let restore_name = "Spectrum Cycle";
        if !controller.mode_iter().any(|m| m.name().eq_ignore_ascii_case(restore_name)) {
            return Err(format!("Refusing to initialize: restoration mode {restore_name:?} unavailable"));
        }
        if !baseline.eq_ignore_ascii_case(restore_name) {
            return Err(format!("Refusing to initialize: active mode {baseline:?} is not {restore_name:?}; set baseline first"));
        }
        println!("[OPENRGB2 TEST] WARNING: init() changes physical lighting to rainbow in Direct mode.");
        println!("[OPENRGB2 TEST] Restoration will be attempted even if initialization fails.");
        println!("[OPENRGB2 TEST] Type exactly INIT CYNOSA to authorize: ");
        let mut confirmation = String::new();
        std::io::stdin().read_line(&mut confirmation)
            .map_err(|e| format!("Confirmation input: {e}"))?;
        if confirmation.trim() != "INIT CYNOSA" {
            return Err("Not authorized; no writes issued".to_string());
        }
        let init_result = tokio::time::timeout(TIMEOUT, controller.init()).await;
        println!("[OPENRGB2 TEST] init result: {:?}", init_result);
        // Always attempt restoration, including after initialization timeout/error.
        // The Cynosa has demonstrated a mismatch between OpenRGB's reported
        // Spectrum Cycle mode and the actual keyboard animation after init().
        // An explicit Off -> Spectrum Cycle transition recovered it via the GUI.
        let off = controller.mode_iter()
            .find(|m| m.name().eq_ignore_ascii_case("Off"))
            .ok_or_else(|| "Off mode disappeared; cannot perform guarded restoration".to_string())?;
        let off_result = tokio::time::timeout(TIMEOUT, off.builder().execute(&controller)).await;
        println!("[OPENRGB2 TEST] Off transition result: {:?}", off_result);
        if !matches!(off_result, Ok(Ok(()))) {
            eprintln!("[OPENRGB2 TEST] WARNING: Off transition failed; still attempting Spectrum Cycle.");
        }

        let restore = controller.mode_iter()
            .find(|m| m.name().eq_ignore_ascii_case(restore_name))
            .ok_or_else(|| "Spectrum Cycle mode disappeared".to_string())?;
        let restore_result = tokio::time::timeout(TIMEOUT, restore.builder().execute(&controller)).await;
        println!("[OPENRGB2 TEST] Spectrum Cycle transition result: {:?}", restore_result);
        if !matches!(restore_result, Ok(Ok(()))) {
            return Err("Spectrum Cycle command failed or timed out; restore manually in OpenRGB UI using Off -> Spectrum Cycle".to_string());
        }
        tokio::time::timeout(TIMEOUT, controller.sync_controller_data()).await
            .map_err(|_| "Restoration refresh timed out; state unverified".to_string())?
            .map_err(|e| format!("Restoration refresh failed; state unverified: {e}"))?;
        let active_mode = controller.active_mode();
        let reported_mode = active_mode.name();
        println!("[OPENRGB2 TEST] SDK-reported mode after refresh: {reported_mode}");
        if !reported_mode.eq_ignore_ascii_case(restore_name) {
            return Err(format!("SDK restoration unverified: mode is {reported_mode:?}"));
        }
        if !matches!(off_result, Ok(Ok(()))) {
            return Err("Off transition did not succeed; physical restoration must be checked manually".to_string());
        }
        println!("[OPENRGB2 TEST] Does the keyboard PHYSICALLY display its normal animated Spectrum Cycle? Type exactly YES to confirm:");
        let mut physical_confirmation = String::new();
        std::io::stdin().read_line(&mut physical_confirmation)
            .map_err(|e| format!("Physical confirmation input: {e}"))?;
        if physical_confirmation.trim() != "YES" {
            return Err("Physical restoration NOT confirmed. Use OpenRGB UI Off -> Spectrum Cycle if needed".to_string());
        }
        if !matches!(init_result, Ok(Ok(()))) {
            return Err("Initialization failed, although SDK and physical baseline restoration were confirmed".to_string());
        }
        println!("[OPENRGB2 TEST] PASS: native rainbow initialization, Off -> Spectrum Cycle restoration, SDK mode refresh, and tester-confirmed physical animation.");
    } else {
        println!("[OPENRGB2 TEST] PASS: read-only protocol and Cynosa inventory verified; no writes issued.");
    }
    Ok(())
}


// Stage 3: separate SDK client and hardware-changing diagnostic.
// This intentionally does not declare physical restoration success.
pub fn run_stress(rate: u32, updates: usize) -> Result<(), String> {
    if !(1..=100).contains(&rate) || !(1..=1_000_000).contains(&updates) {
        return Err("Rate must be 1..100 Hz and updates 1..1000000".into());
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("Tokio runtime initialization: {e}"))?;
    runtime.block_on(stress_probe(rate, updates))
}

async fn stress_probe(rate: u32, updates: usize) -> Result<(), String> {
    use std::io::Write;
    println!("[OPENRGB2 STRESS] SDK v6, 132 LEDs, rate={rate} Hz, updates={updates}; no connection rotation.");
    println!("[OPENRGB2 STRESS] WARNING: This test changes physical keyboard lighting.");
    println!("[OPENRGB2 STRESS] WARNING: Automated restoration has NOT been physically validated.");
    println!("[OPENRGB2 STRESS] Before starting, ensure no Screenshaver ambient-lighting owner is active.");
    println!("[OPENRGB2 STRESS] After testing, use OpenRGB GUI Off -> Spectrum Cycle if necessary.");
    println!("[OPENRGB2 STRESS] Type exactly STRESS CYNOSA to authorize: ");
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let mut confirmation = String::new();
    std::io::stdin().read_line(&mut confirmation).map_err(|e| e.to_string())?;
    if confirmation.trim() != "STRESS CYNOSA" { return Err("Not authorized; no writes issued".into()); }

    let mut client = tokio::time::timeout(TIMEOUT, OpenRgbClient::connect_to(ENDPOINT, PROTOCOL))
        .await.map_err(|_| "Initial connection timed out".to_string())?
        .map_err(|e| format!("Initial connection failed: {e}"))?;
    if client.get_protocol_version() != PROTOCOL { return Err("Unexpected SDK protocol".into()); }
    let controllers = tokio::time::timeout(TIMEOUT, client.get_all_controllers())
        .await.map_err(|_| "Initial enumeration timed out".to_string())?
        .map_err(|e| format!("Initial enumeration failed: {e}"))?;
    let matches: Vec<_> = controllers.into_iter().filter(|c| c.name() == "Razer Cynosa Chroma").collect();
    if matches.len() != 1 { return Err(format!("Expected one Cynosa; found {}", matches.len())); }
    let controller = matches.into_iter().next().ok_or("Cynosa missing")?;
    if controller.num_leds() != 132 { return Err("Cynosa LED inventory mismatch".into()); }
    let baseline = controller.active_mode().name().to_string();
    if baseline != "Spectrum Cycle" {
        return Err(format!("Refusing streaming: reported baseline is {baseline:?}; select Spectrum Cycle in GUI first"));
    }

    // A single non-queued writer. Each awaited set_leds call must complete before the next.
    // Use a deterministic, moving color gradient to make stalled physical output visible.
    let mut sent = 0usize;
    let mut max_call = Duration::ZERO;
    let mut sum_call = Duration::ZERO;
    let started = Instant::now();
    let period = Duration::from_secs_f64(1.0 / rate as f64);
    let mut next_deadline = tokio::time::Instant::now();
    let mut streaming_error: Option<String> = None;

    // This reader is started only after typed authorization. In an interactive terminal,
    // press Esc followed by Enter to interrupt (without requiring a new dependency).
    let interrupted = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let reader_interrupted = interrupted.clone();
    std::thread::spawn(move || {
        use std::io::Read;
        let stdin = std::io::stdin();
        for byte in stdin.lock().bytes() {
            match byte {
                Ok(27) => { reader_interrupted.store(true, std::sync::atomic::Ordering::Relaxed); break; }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });
    println!("[OPENRGB2 STRESS] Streaming started. Press Esc then Enter to interrupt.");

    // init() uses the crate's documented native Direct-mode rainbow initialization.
    let init = tokio::time::timeout(TIMEOUT, controller.init()).await;
    if !matches!(init, Ok(Ok(()))) {
        return Err(format!("Native initialization failed: {init:?}; check lighting manually"));
    }

    for frame in 0..updates {
        next_deadline += period;
        let colors: Vec<openrgb2::Color> = (0..132usize).map(|led| {
            let phase = ((led * 3 + frame) % 256) as u8;
            openrgb2::Color::new(phase, 255u8.wrapping_sub(phase), phase / 2)
        }).collect();
        let call_started = Instant::now();
        if interrupted.load(std::sync::atomic::Ordering::Relaxed) {
            streaming_error = Some("Interrupted by user".to_string());
            break;
        }
        let result = tokio::time::timeout(TIMEOUT, controller.set_leds(colors)).await;
        let latency = call_started.elapsed();
        max_call = max_call.max(latency);
        sum_call += latency;
        match result {
            Ok(Ok(())) => sent += 1,
            Ok(Err(error)) => { streaming_error = Some(format!("LED update {} failed: {error}", frame + 1)); break; }
            Err(_) => { streaming_error = Some(format!("LED update {} timed out after 5s", frame + 1)); break; }
        }
        if sent % 1000 == 0 || sent == updates {
            println!("[OPENRGB2 STRESS] completed={sent}/{updates} elapsed={:.2}s max_call={max_call:?}", started.elapsed().as_secs_f64());
        }
        tokio::time::sleep_until(next_deadline).await;
    }
    println!("[OPENRGB2 STRESS] Streaming ended: completed={sent}/{updates} elapsed={:.2}s mean_call={:?} max_call={max_call:?}",
        started.elapsed().as_secs_f64(), if sent > 0 { sum_call / sent as u32 } else { Duration::ZERO });
    if let Some(ref error) = streaming_error { eprintln!("[OPENRGB2 STRESS] STREAMING STOPPED: {error}"); }

    // A fresh connection is an independent SDK responsiveness check, NOT physical validation.
    // Keep the original controller alive until after the check, mirroring the previous test.
    let health_started = Instant::now();
    let health = async {
        let mut check = OpenRgbClient::connect_to(ENDPOINT, PROTOCOL).await
            .map_err(|e| format!("Fresh SDK negotiation failed: {e}"))?;
        if check.get_protocol_version() != PROTOCOL { return Err("Fresh connection negotiated wrong protocol".to_string()); }
        let found = check.get_all_controllers().await
            .map_err(|e| format!("Fresh enumeration failed: {e}"))?;
        if !found.iter().any(|c| c.name() == "Razer Cynosa Chroma" && c.num_leds() == 132) {
            return Err("Cynosa missing from fresh SDK enumeration".into());
        }
        Ok::<(), String>(())
    };
    let health_result = tokio::time::timeout(TIMEOUT, health).await;
    let healthy = matches!(health_result, Ok(Ok(())));
    println!("[OPENRGB2 STRESS] Fresh SDK health: {health_result:?}; elapsed={:?}", health_started.elapsed());
    println!("[OPENRGB2 STRESS] Restoration is not attempted by this diagnostic; manually restore Off -> Spectrum Cycle in OpenRGB GUI.");
    println!("[OPENRGB2 STRESS] SDK success is NOT proof of physical lighting restoration.");
    if let Some(error) = streaming_error { return Err(error); }
    if !healthy { return Err("Fresh SDK health check failed after streaming".into()); }
    println!("[OPENRGB2 STRESS] PASS: streaming calls and fresh SDK health only; physical restoration NOT VERIFIED.");
    Ok(())
}
