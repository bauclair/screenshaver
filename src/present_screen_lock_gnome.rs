use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const RUNTIME_SHADER_FILENAME: &str = "screenshaver-gnome-lock-shader.glsl";
const RUNTIME_SHADER_TEMP_FILENAME: &str = "screenshaver-gnome-lock-shader.glsl.tmp";
const IDLE_POLL_INTERVAL: Duration = Duration::from_millis(25);

/// GNOME Test #20 presentation host.
///
/// GNOME owns authentication, input isolation, and unlock authority. Screenshaver
/// owns shader policy selection and preprocessing. The resulting production
/// fragment source is handed to the GNOME Shell extension through an atomically
/// replaced file in XDG_RUNTIME_DIR. GNOME Shell executes that source natively
/// through Shell.GLSLEffect; no SDL/OpenGL frame renderer runs in this backend.
pub(crate) struct GnomeLockPresenter {
    producer: GnomeShaderSourceProducer,
}

impl GnomeLockPresenter {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn start(
        _sdl: &sdl2::Sdl,
        logfile: &Path,
        session_id: &str,
        shader_manager: crate::manage_shader::ShaderManager,
        shader_interval: u64,
        animation_speed_policy: crate::load_config::AnimationSpeedPolicy,
        _global_rendered_fps: u32,
        _fps_policy_entries: Vec<crate::load_config::FpsPolicyEntry>,
        _texture_policy: crate::load_config::TexturePolicy,
        _postprocess_policy: crate::load_config::PostprocessPolicy,
        _audio_bands: Option<crate::audio_backend::SharedAudioBands>,
        _subtitles: bool,
        _subtitle_placement: crate::parse_subtitle_placement::SubtitlePlacement,
    ) -> Result<Self, String> {
        let producer = GnomeShaderSourceProducer::new(
            logfile,
            session_id,
            shader_manager,
            shader_interval,
            animation_speed_policy,
        )?;

        log_information(
            logfile,
            "[LOCK] GNOME Test #20 production shader-source backend initialized",
        );

        Ok(Self { producer })
    }

    pub(crate) fn run_until<F>(
        &mut self,
        mut lock_finished: F,
    ) -> Result<(), String>
    where
        F: FnMut() -> bool,
    {
        self.producer.run_until(&mut lock_finished)
    }
}

struct GnomeShaderSourceProducer {
    logfile: PathBuf,
    shader_manager: crate::manage_shader::ShaderManager,
    shader_interval: u64,
    animation_speed_policy: crate::load_config::AnimationSpeedPolicy,
    last_shader_switch: Instant,
    runtime_shader_path: PathBuf,
    runtime_shader_temp_path: PathBuf,
    active_shader_name: String,
    active_policy_id: i64,
}

impl GnomeShaderSourceProducer {
    fn new(
        logfile: &Path,
        session_id: &str,
        mut shader_manager: crate::manage_shader::ShaderManager,
        shader_interval: u64,
        animation_speed_policy: crate::load_config::AnimationSpeedPolicy,
    ) -> Result<Self, String> {
        let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .ok_or_else(|| {
                "XDG_RUNTIME_DIR is unavailable for GNOME shader-source handoff"
                    .to_string()
            })?;

        let runtime_shader_path = runtime_dir.join(RUNTIME_SHADER_FILENAME);
        let runtime_shader_temp_path = runtime_dir.join(RUNTIME_SHADER_TEMP_FILENAME);
        let _ = fs::remove_file(&runtime_shader_path);
        let _ = fs::remove_file(&runtime_shader_temp_path);

        let selected = select_production_shader(&mut shader_manager)?;
        publish_shader_source(
            &runtime_shader_path,
            &runtime_shader_temp_path,
            &selected.source,
        )?;

        let animation_speed = animation_speed_policy.animation_speed_for_policy(
            selected.entry.policy_id,
            &selected.entry.name,
            selected.entry.source_path.as_deref(),
            None,
        );

        log_information(
            logfile,
            &format!(
                "[LOCK] Test #20 published production-preprocessed shader '{}' (policy_id={}, policy='{}', {} bytes, animation_speed={:.3}x, session={})",
                selected.entry.name,
                selected.entry.policy_id,
                selected.entry.policy_name,
                selected.source.len(),
                animation_speed,
                session_id,
            ),
        );

        Ok(Self {
            logfile: logfile.to_path_buf(),
            shader_manager,
            shader_interval,
            animation_speed_policy,
            last_shader_switch: Instant::now(),
            runtime_shader_path,
            runtime_shader_temp_path,
            active_shader_name: selected.entry.name,
            active_policy_id: selected.entry.policy_id,
        })
    }

    fn run_until<F>(&mut self, lock_finished: &mut F) -> Result<(), String>
    where
        F: FnMut() -> bool,
    {
        log_information(
            &self.logfile,
            "[LOCK] GNOME Test #20 native shader presentation loop started",
        );

        while !lock_finished() {
            // Test #20 reconnects production selection/preprocessing without
            // starting a second renderer. Rotation publication is retained on
            // the Rust side; the Test #20 extension intentionally consumes the
            // initial handoff only. Dynamic in-place effect replacement is the
            // next production step after this compatibility test succeeds.
            if self.shader_interval > 0
                && self.last_shader_switch.elapsed().as_secs() >= self.shader_interval
            {
                match select_production_shader(&mut self.shader_manager) {
                    Ok(selected) => {
                        publish_shader_source(
                            &self.runtime_shader_path,
                            &self.runtime_shader_temp_path,
                            &selected.source,
                        )?;

                        let animation_speed =
                            self.animation_speed_policy.animation_speed_for_policy(
                                selected.entry.policy_id,
                                &selected.entry.name,
                                selected.entry.source_path.as_deref(),
                                None,
                            );

                        self.active_shader_name = selected.entry.name.clone();
                        self.active_policy_id = selected.entry.policy_id;
                        self.last_shader_switch = Instant::now();

                        log_information(
                            &self.logfile,
                            &format!(
                                "[LOCK] Test #20 published replacement production shader '{}' (policy_id={}, {} bytes, animation_speed={:.3}x); extension reload intentionally deferred",
                                selected.entry.name,
                                selected.entry.policy_id,
                                selected.source.len(),
                                animation_speed,
                            ),
                        );
                    }
                    Err(error) => {
                        log_warning(
                            &self.logfile,
                            &format!(
                                "[LOCK] Test #20 could not select replacement shader: {error}"
                            ),
                        );
                        self.last_shader_switch = Instant::now();
                    }
                }
            }

            std::thread::sleep(IDLE_POLL_INTERVAL);
        }

        self.cleanup();

        log_information(
            &self.logfile,
            &format!(
                "[LOCK] GNOME Test #20 native shader presentation loop stopped (last shader='{}', policy_id={})",
                self.active_shader_name,
                self.active_policy_id,
            ),
        );

        Ok(())
    }

    fn cleanup(&mut self) {
        let _ = fs::remove_file(&self.runtime_shader_path);
        let _ = fs::remove_file(&self.runtime_shader_temp_path);
    }
}

impl Drop for GnomeShaderSourceProducer {
    fn drop(&mut self) {
        self.cleanup();
    }
}

struct ProductionShader {
    entry: crate::manage_shader::ShaderEntry,
    source: String,
}

fn select_production_shader(
    shader_manager: &mut crate::manage_shader::ShaderManager,
) -> Result<ProductionShader, String> {
    let maximum_attempts = shader_manager.shader_count();

    for _ in 0..maximum_attempts {
        let Some(entry) = shader_manager.next_entry() else {
            break;
        };

        let managed_path = crate::locate_paths::shader_dir().join(&entry.name);
        let is_managed = entry
            .source_path
            .as_ref()
            .is_none_or(|path| paths_refer_to_same_file(path, &managed_path));

        let loaded = if is_managed {
            crate::load_shader::load_shader(&entry.name)
        } else if let Some(path) = entry.source_path.as_ref() {
            crate::load_shader::load_shader_for_preview(path)
        } else {
            crate::load_shader::load_shader(&entry.name)
        };

        match loaded {
            crate::load_shader::ShaderLoadResult::Ready {
                source,
                channel_usage,
                shader_inputs,
                ..
            } => {
                if channel_usage.channels.iter().any(|used| *used) {
                    log_warning_global(&format!(
                        "[LOCK] Test #20 skipping '{}' because GNOME native texture-channel binding is not connected yet",
                        entry.name,
                    ));
                    shader_manager.remove_entry(&entry);
                    continue;
                }

                if !shader_inputs.is_empty() {
                    log_warning_global(&format!(
                        "[LOCK] Test #20 skipping '{}' because GNOME native ISF input binding is not connected yet",
                        entry.name,
                    ));
                    shader_manager.remove_entry(&entry);
                    continue;
                }

                if !source.contains("mainImage") {
                    log_warning_global(&format!(
                        "[LOCK] Test #20 skipping '{}' because this diagnostic bridge currently requires the production ShaderToy mainImage path",
                        entry.name,
                    ));
                    shader_manager.remove_entry(&entry);
                    continue;
                }

                return Ok(ProductionShader { entry, source });
            }

            crate::load_shader::ShaderLoadResult::Rejected { reasons, .. } => {
                log_warning_global(&format!(
                    "[LOCK] Test #20 production shader rejected '{}': {}",
                    entry.name,
                    reasons.join("; "),
                ));
                shader_manager.remove_entry(&entry);
            }

            crate::load_shader::ShaderLoadResult::Unavailable { error, .. } => {
                log_warning_global(&format!(
                    "[LOCK] Test #20 production shader unavailable '{}': {}",
                    entry.name,
                    error,
                ));
                shader_manager.remove_entry(&entry);
            }
        }
    }

    Err("No Test #20-compatible production ShaderToy shader is available".to_string())
}

fn publish_shader_source(
    destination: &Path,
    temporary: &Path,
    source: &str,
) -> Result<(), String> {
    let _ = fs::remove_file(temporary);

    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(temporary)
            .map_err(|error| {
                format!(
                    "Unable to create GNOME production shader handoff '{}': {error}",
                    temporary.display(),
                )
            })?;

        file.write_all(source.as_bytes()).map_err(|error| {
            format!(
                "Unable to write GNOME production shader handoff '{}': {error}",
                temporary.display(),
            )
        })?;

        file.flush().map_err(|error| {
            format!(
                "Unable to flush GNOME production shader handoff '{}': {error}",
                temporary.display(),
            )
        })?;
    }

    fs::rename(temporary, destination).map_err(|error| {
        format!(
            "Unable to publish GNOME production shader handoff '{}' -> '{}': {error}",
            temporary.display(),
            destination.display(),
        )
    })
}

fn paths_refer_to_same_file(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn log_information(logfile: &Path, message: &str) {
    crate::logger::information(logfile, message);
}

fn log_warning(logfile: &Path, message: &str) {
    crate::logger::warning(logfile, message);
}

fn log_warning_global(message: &str) {
    let logfile = crate::locate_paths::runtime_log_path();
    crate::logger::warning(&logfile, message);
}
