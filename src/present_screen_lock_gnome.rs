use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const RUNTIME_SHADER_FILENAME: &str = "screenshaver-gnome-lock-shader.glsl";
const RUNTIME_SHADER_TEMP_FILENAME: &str = "screenshaver-gnome-lock-shader.glsl.tmp";
const RUNTIME_METADATA_FILENAME: &str = "screenshaver-gnome-lock-metadata.txt";
const RUNTIME_METADATA_TEMP_FILENAME: &str = "screenshaver-gnome-lock-metadata.txt.tmp";
const RUNTIME_ADVANCE_FILENAME: &str = "screenshaver-gnome-lock-advance.txt";
const RUNTIME_AUDIO_FILENAME: &str = "screenshaver-gnome-lock-audio.txt";
const RUNTIME_AUDIO_TEMP_FILENAME: &str = "screenshaver-gnome-lock-audio.txt.tmp";
const IDLE_POLL_INTERVAL: Duration = Duration::from_millis(25);
const JOURNAL_FAILURE_ARM_WINDOW: Duration = Duration::from_secs(2);
const JOURNAL_FAILURE_ADVANCE_DELAY: Duration = Duration::from_millis(500);

/// GNOME Test #27 presentation host.
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
        global_rendered_fps: u32,
        fps_policy_entries: Vec<crate::load_config::FpsPolicyEntry>,
        texture_policy: crate::load_config::TexturePolicy,
        postprocess_policy: crate::load_config::PostprocessPolicy,
        audio_bands: Option<crate::audio_backend::SharedAudioBands>,
        subtitles: bool,
        subtitle_placement: crate::parse_subtitle_placement::SubtitlePlacement,
    ) -> Result<Self, String> {
        let producer = GnomeShaderSourceProducer::new(
            logfile,
            session_id,
            shader_manager,
            shader_interval,
            animation_speed_policy,
            global_rendered_fps,
            fps_policy_entries,
            texture_policy,
            postprocess_policy,
            audio_bands,
            subtitles,
            subtitle_placement,
        )?;

        log_information(
            logfile,
            "[LOCK] GNOME Test #27 production shader-source backend initialized",
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
    global_rendered_fps: u32,
    fps_policy_entries: Vec<crate::load_config::FpsPolicyEntry>,
    texture_manager: crate::manage_textures::TextureManager,
    postprocess_policy: crate::load_config::PostprocessPolicy,
    audio_bands: Option<crate::audio_backend::SharedAudioBands>,
    subtitles: bool,
    subtitle_placement: crate::parse_subtitle_placement::SubtitlePlacement,
    last_shader_switch: Instant,
    runtime_shader_path: PathBuf,
    runtime_shader_temp_path: PathBuf,
    runtime_metadata_path: PathBuf,
    runtime_metadata_temp_path: PathBuf,
    runtime_advance_path: PathBuf,
    runtime_audio_path: PathBuf,
    runtime_audio_temp_path: PathBuf,
    last_audio_bands: Option<crate::analyze_audio::AudioBands>,
    journal_watcher: Option<GnomeShellJournalWatcher>,
    journal_failure_armed_until: Option<Instant>,
    journal_failure_consumed: bool,
    journal_advance_due_at: Option<Instant>,
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
        global_rendered_fps: u32,
        fps_policy_entries: Vec<crate::load_config::FpsPolicyEntry>,
        texture_policy: crate::load_config::TexturePolicy,
        postprocess_policy: crate::load_config::PostprocessPolicy,
        audio_bands: Option<crate::audio_backend::SharedAudioBands>,
        subtitles: bool,
        subtitle_placement: crate::parse_subtitle_placement::SubtitlePlacement,
    ) -> Result<Self, String> {
        let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .ok_or_else(|| {
                "XDG_RUNTIME_DIR is unavailable for GNOME shader-source handoff"
                    .to_string()
            })?;

        let runtime_shader_path = runtime_dir.join(RUNTIME_SHADER_FILENAME);
        let runtime_shader_temp_path = runtime_dir.join(RUNTIME_SHADER_TEMP_FILENAME);
        let runtime_metadata_path = runtime_dir.join(RUNTIME_METADATA_FILENAME);
        let runtime_metadata_temp_path = runtime_dir.join(RUNTIME_METADATA_TEMP_FILENAME);
        let runtime_advance_path = runtime_dir.join(RUNTIME_ADVANCE_FILENAME);
        let runtime_audio_path = runtime_dir.join(RUNTIME_AUDIO_FILENAME);
        let runtime_audio_temp_path = runtime_dir.join(RUNTIME_AUDIO_TEMP_FILENAME);
        let _ = fs::remove_file(&runtime_shader_path);
        let _ = fs::remove_file(&runtime_shader_temp_path);
        let _ = fs::remove_file(&runtime_metadata_path);
        let _ = fs::remove_file(&runtime_metadata_temp_path);
        let _ = fs::remove_file(&runtime_advance_path);
        let _ = fs::remove_file(&runtime_audio_path);
        let _ = fs::remove_file(&runtime_audio_temp_path);

        let journal_watcher = match GnomeShellJournalWatcher::start() {
            Ok(watcher) => {
                log_information(
                    logfile,
                    "[LOCK] Test #27 GNOME Shell shader-failure journal watcher started",
                );
                Some(watcher)
            }
            Err(error) => {
                log_warning(
                    logfile,
                    &format!(
                        "[LOCK] Test #27 GNOME Shell journal watcher unavailable; Cogl-only shader failures will not be fast-skipped: {error}"
                    ),
                );
                None
            }
        };

        let selected = select_production_shader(&mut shader_manager)?;

        let animation_speed = animation_speed_policy.animation_speed_for_policy(
            selected.entry.policy_id,
            &selected.entry.name,
            selected.entry.source_path.as_deref(),
            None,
        );

        let configured_fps = resolve_shader_fps(
            global_rendered_fps.max(1),
            &fps_policy_entries,
            selected.entry.policy_id,
            &selected.entry.name,
            selected.entry.source_path.as_deref(),
        );

        let mut texture_manager = crate::manage_textures::TextureManager::new(texture_policy);
        if let Err(error) = texture_manager.prepare_for_policy_with_path(
            selected.entry.policy_id,
            &selected.entry.name,
            selected.entry.source_path.as_deref(),
            selected.channel_usage,
        ) {
            log_warning(
                logfile,
                &format!(
                    "[LOCK] GNOME description metadata texture preparation failed for '{}': {error}",
                    selected.entry.name,
                ),
            );
        }

        let postprocess_profile = postprocess_policy.profile_for_policy(
            selected.entry.policy_id,
            &selected.entry.name,
            selected.entry.source_path.as_deref(),
        );

        let metadata = build_presentation_metadata(
            &selected,
            &texture_manager,
            animation_speed,
            configured_fps,
            postprocess_profile,
            subtitles,
            subtitle_placement,
        );

        // Publish metadata first. The shader file is the extension's generation
        // trigger, so a newly observed shader always has matching metadata ready.
        publish_text_atomically(
            &runtime_metadata_path,
            &runtime_metadata_temp_path,
            &metadata,
            "GNOME lock metadata handoff",
        )?;
        publish_text_atomically(
            &runtime_shader_path,
            &runtime_shader_temp_path,
            &selected.source,
            "GNOME production shader handoff",
        )?;

        log_information(
            logfile,
            &format!(
                "[LOCK] Test #27 published production-preprocessed shader '{}' (policy_id={}, policy='{}', {} bytes, animation_speed={:.3}x, session={})",
                selected.entry.name,
                selected.entry.policy_id,
                selected.entry.policy_name,
                selected.source.len(),
                animation_speed,
                session_id,
            ),
        );

        let initial_audio_bands = read_audio_bands(&audio_bands);
        publish_audio_bands(
            &runtime_audio_path,
            &runtime_audio_temp_path,
            initial_audio_bands,
        )?;

        log_information(
            logfile,
            &format!(
                "[LOCK] Test #33 GNOME audio-band handoff initialized: bass={:.3} mid={:.3} treble={:.3}",
                initial_audio_bands.bass,
                initial_audio_bands.midrange,
                initial_audio_bands.treble,
            ),
        );

        Ok(Self {
            logfile: logfile.to_path_buf(),
            shader_manager,
            shader_interval,
            animation_speed_policy,
            global_rendered_fps: global_rendered_fps.max(1),
            fps_policy_entries,
            texture_manager,
            postprocess_policy,
            audio_bands,
            subtitles,
            subtitle_placement,
            last_shader_switch: Instant::now(),
            runtime_shader_path,
            runtime_shader_temp_path,
            runtime_metadata_path,
            runtime_metadata_temp_path,
            runtime_advance_path,
            runtime_audio_path,
            runtime_audio_temp_path,
            last_audio_bands: Some(initial_audio_bands),
            journal_watcher,
            journal_failure_armed_until: Some(Instant::now() + JOURNAL_FAILURE_ARM_WINDOW),
            journal_failure_consumed: false,
            journal_advance_due_at: None,
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
            "[LOCK] GNOME Test #27 native shader presentation loop started",
        );

        while !lock_finished() {
            // GNOME keeps a single native Shell.GLSLEffect renderer. Rust owns
            // production policy selection/preprocessing and publishes each
            // rotation; the extension polls the handoff and hot-swaps the effect.
            self.poll_journal_shader_failure();

            let extension_advance_requested = self.take_early_advance_request();
            let journal_advance_requested = self.take_due_journal_advance();
            let early_advance_requested =
                extension_advance_requested || journal_advance_requested;
            let normal_interval_elapsed = self.shader_interval > 0
                && self.last_shader_switch.elapsed().as_secs() >= self.shader_interval;

            if early_advance_requested || normal_interval_elapsed {
                if extension_advance_requested {
                    log_warning(
                        &self.logfile,
                        &format!(
                            "[LOCK] Test #27 GNOME could not apply shader '{}' (policy_id={}); truncating its rotation interval and advancing",
                            self.active_shader_name,
                            self.active_policy_id,
                        ),
                    );
                }

                if journal_advance_requested {
                    log_warning(
                        &self.logfile,
                        &format!(
                            "[LOCK] Test #27 GNOME/Cogl shader compilation or link failure confirmed for '{}' (policy_id={}); advancing to the next shader",
                            self.active_shader_name,
                            self.active_policy_id,
                        ),
                    );
                }

                match select_production_shader(&mut self.shader_manager) {
                    Ok(selected) => {
                        let animation_speed =
                            self.animation_speed_policy.animation_speed_for_policy(
                                selected.entry.policy_id,
                                &selected.entry.name,
                                selected.entry.source_path.as_deref(),
                                None,
                            );

                        let configured_fps = resolve_shader_fps(
                            self.global_rendered_fps,
                            &self.fps_policy_entries,
                            selected.entry.policy_id,
                            &selected.entry.name,
                            selected.entry.source_path.as_deref(),
                        );

                        if let Err(error) = self.texture_manager.prepare_for_policy_with_path(
                            selected.entry.policy_id,
                            &selected.entry.name,
                            selected.entry.source_path.as_deref(),
                            selected.channel_usage,
                        ) {
                            log_warning(
                                &self.logfile,
                                &format!(
                                    "[LOCK] GNOME description metadata texture preparation failed for '{}': {error}",
                                    selected.entry.name,
                                ),
                            );
                        }

                        let postprocess_profile = self.postprocess_policy.profile_for_policy(
                            selected.entry.policy_id,
                            &selected.entry.name,
                            selected.entry.source_path.as_deref(),
                        );

                        let metadata = build_presentation_metadata(
                            &selected,
                            &self.texture_manager,
                            animation_speed,
                            configured_fps,
                            postprocess_profile,
                            self.subtitles,
                            self.subtitle_placement,
                        );

                        publish_text_atomically(
                            &self.runtime_metadata_path,
                            &self.runtime_metadata_temp_path,
                            &metadata,
                            "GNOME lock metadata handoff",
                        )?;
                        publish_text_atomically(
                            &self.runtime_shader_path,
                            &self.runtime_shader_temp_path,
                            &selected.source,
                            "GNOME production shader handoff",
                        )?;

                        self.active_shader_name = selected.entry.name.clone();
                        self.active_policy_id = selected.entry.policy_id;
                        self.last_shader_switch = Instant::now();
                        self.arm_journal_failure_detection();
                        let _ = fs::remove_file(&self.runtime_advance_path);

                        log_information(
                            &self.logfile,
                            &format!(
                                "[LOCK] Test #27 published replacement production shader '{}' (policy_id={}, {} bytes, animation_speed={:.3}x); extension will apply through the native GNOME effect",
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
                                "[LOCK] Test #27 could not select replacement shader: {error}"
                            ),
                        );
                        self.last_shader_switch = Instant::now();
                    }
                }
            }

            self.publish_audio_bands_if_changed();
            std::thread::sleep(IDLE_POLL_INTERVAL);
        }

        self.cleanup();

        log_information(
            &self.logfile,
            &format!(
                "[LOCK] GNOME Test #27 native shader presentation loop stopped (last shader='{}', policy_id={})",
                self.active_shader_name,
                self.active_policy_id,
            ),
        );

        Ok(())
    }

    fn publish_audio_bands_if_changed(&mut self) {
        let bands = read_audio_bands(&self.audio_bands);

        if self.last_audio_bands.is_some_and(|previous| audio_bands_equal(previous, bands)) {
            return;
        }

        match publish_audio_bands(
            &self.runtime_audio_path,
            &self.runtime_audio_temp_path,
            bands,
        ) {
            Ok(()) => {
                self.last_audio_bands = Some(bands);
            }
            Err(error) => {
                log_warning(
                    &self.logfile,
                    &format!(
                        "[LOCK] Test #33 unable to publish GNOME audio bands: {error}"
                    ),
                );
            }
        }
    }

    fn take_early_advance_request(&self) -> bool {
        if !self.runtime_advance_path.exists() {
            return false;
        }

        match fs::remove_file(&self.runtime_advance_path) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => {
                log_warning(
                    &self.logfile,
                    &format!(
                        "[LOCK] Test #27 could not consume GNOME early-advance request '{}': {error}",
                        self.runtime_advance_path.display(),
                    ),
                );
                false
            }
        }
    }

    fn arm_journal_failure_detection(&mut self) {
        if let Some(watcher) = self.journal_watcher.as_ref() {
            watcher.drain();
        }

        self.journal_failure_armed_until =
            Some(Instant::now() + JOURNAL_FAILURE_ARM_WINDOW);
        self.journal_failure_consumed = false;
        self.journal_advance_due_at = None;
    }

    fn poll_journal_shader_failure(&mut self) {
        let Some(watcher) = self.journal_watcher.as_ref() else {
            return;
        };

        let now = Instant::now();
        let armed = self
            .journal_failure_armed_until
            .is_some_and(|deadline| now <= deadline);

        let mut detected = false;

        while let Some(line) = watcher.try_recv() {
            if line.contains("Shader compilation failed:")
                || line.contains("Failed to link GLSL program:")
            {
                detected = true;
            }
        }

        if !detected || !armed || self.journal_failure_consumed {
            return;
        }

        self.journal_failure_consumed = true;
        self.journal_advance_due_at =
            Some(Instant::now() + JOURNAL_FAILURE_ADVANCE_DELAY);

        log_warning(
            &self.logfile,
            &format!(
                "[LOCK] Test #27 observed GNOME/Cogl shader failure for '{}' (policy_id={}); truncating interval to {}ms",
                self.active_shader_name,
                self.active_policy_id,
                JOURNAL_FAILURE_ADVANCE_DELAY.as_millis(),
            ),
        );
    }

    fn take_due_journal_advance(&mut self) -> bool {
        let Some(due_at) = self.journal_advance_due_at else {
            return false;
        };

        if Instant::now() < due_at {
            return false;
        }

        self.journal_advance_due_at = None;
        self.journal_failure_armed_until = None;
        true
    }

    fn cleanup(&mut self) {
        let _ = fs::remove_file(&self.runtime_shader_path);
        let _ = fs::remove_file(&self.runtime_shader_temp_path);
        let _ = fs::remove_file(&self.runtime_metadata_path);
        let _ = fs::remove_file(&self.runtime_metadata_temp_path);
        let _ = fs::remove_file(&self.runtime_advance_path);
        let _ = fs::remove_file(&self.runtime_audio_path);
        let _ = fs::remove_file(&self.runtime_audio_temp_path);
    }
}

impl Drop for GnomeShaderSourceProducer {
    fn drop(&mut self) {
        self.cleanup();
    }
}

struct GnomeShellJournalWatcher {
    child: Child,
    receiver: Receiver<String>,
    reader_thread: Option<JoinHandle<()>>,
}

impl GnomeShellJournalWatcher {
    fn start() -> Result<Self, String> {
        let mut child = Command::new("journalctl")
            .args([
                "--follow",
                "--lines=0",
                "--output=cat",
                "_COMM=gnome-shell",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("Unable to start journalctl: {error}"))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "journalctl stdout was unavailable".to_string())?;

        let (sender, receiver) = mpsc::channel();
        let reader_thread = std::thread::spawn(move || {
            let reader = BufReader::new(stdout);

            for line in reader.lines() {
                let Ok(line) = line else {
                    break;
                };

                if sender.send(line).is_err() {
                    break;
                }
            }
        });

        Ok(Self {
            child,
            receiver,
            reader_thread: Some(reader_thread),
        })
    }

    fn try_recv(&self) -> Option<String> {
        match self.receiver.try_recv() {
            Ok(line) => Some(line),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => None,
        }
    }

    fn drain(&self) {
        while self.try_recv().is_some() {}
    }
}

impl Drop for GnomeShellJournalWatcher {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();

        if let Some(reader_thread) = self.reader_thread.take() {
            let _ = reader_thread.join();
        }
    }
}

struct ProductionShader {
    entry: crate::manage_shader::ShaderEntry,
    source: String,
    channel_usage: crate::preprocess_shader::ShaderChannelUsage,
    built_in_default: bool,
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
                built_in_default,
                ..
            } => {
                if channel_usage.channels.iter().any(|used| *used) {
                    log_warning_global(&format!(
                        "[LOCK] Test #27 skipping '{}' because GNOME native texture-channel binding is not connected yet",
                        entry.name,
                    ));
                    shader_manager.remove_entry(&entry);
                    continue;
                }

                if !shader_inputs.is_empty() {
                    log_warning_global(&format!(
                        "[LOCK] Test #27 skipping '{}' because GNOME native ISF input binding is not connected yet",
                        entry.name,
                    ));
                    shader_manager.remove_entry(&entry);
                    continue;
                }

                if !source.contains("mainImage") {
                    log_warning_global(&format!(
                        "[LOCK] Test #27 skipping '{}' because this diagnostic bridge currently requires the production ShaderToy mainImage path",
                        entry.name,
                    ));
                    shader_manager.remove_entry(&entry);
                    continue;
                }

                return Ok(ProductionShader {
                    entry,
                    source,
                    channel_usage,
                    built_in_default,
                });
            }

            crate::load_shader::ShaderLoadResult::Rejected { reasons, .. } => {
                log_warning_global(&format!(
                    "[LOCK] Test #27 production shader rejected '{}': {}",
                    entry.name,
                    reasons.join("; "),
                ));
                shader_manager.remove_entry(&entry);
            }

            crate::load_shader::ShaderLoadResult::Unavailable { error, .. } => {
                log_warning_global(&format!(
                    "[LOCK] Test #27 production shader unavailable '{}': {}",
                    entry.name,
                    error,
                ));
                shader_manager.remove_entry(&entry);
            }
        }
    }

    Err("No Test #27-compatible production ShaderToy shader is available".to_string())
}

fn build_presentation_metadata(
    selected: &ProductionShader,
    texture_manager: &crate::manage_textures::TextureManager,
    animation_speed: f32,
    configured_fps: u32,
    postprocess_profile: crate::load_config::PostprocessProfile,
    subtitles: bool,
    subtitle_placement: crate::parse_subtitle_placement::SubtitlePlacement,
) -> String {
    let (texture, palette) = texture_manager
        .active_specification_selection()
        .map(|(specification, palette)| {
            let texture = if specification.count_was_explicit {
                specification.display_name()
            } else {
                format!(
                    "{} ({})",
                    specification.display_name(),
                    specification.requested_primitive_count,
                )
            };

            (Some(texture), Some(palette.to_string()))
        })
        .unwrap_or((None, None));

    let shader_label = if selected.built_in_default
        || Path::new(&selected.entry.name)
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("default.glsl"))
    {
        "Collect more shaders at https://editor.isf.video/shaders and https://shadertoy.com/browse"
            .to_string()
    } else {
        selected.entry.policy_name.clone()
    };

    let shader_label = format!(
        "{} | {}",
        shader_label,
        format_animation_speed(animation_speed),
    );

    let mut lines = Vec::new();
    lines.push("version=1".to_string());
    lines.push(format!("source_bytes={}", selected.source.len()));
    lines.push(format!("policy_id={}", selected.entry.policy_id));
    lines.push(format!("shader={}", sanitize_metadata_value(&shader_label)));
    lines.push(format!(
        "texture={}",
        sanitize_metadata_value(texture.as_deref().unwrap_or(""))
    ));
    lines.push(format!(
        "palette={}",
        sanitize_metadata_value(palette.as_deref().unwrap_or(""))
    ));
    lines.push(format!("configured_fps={}", configured_fps.max(1)));
    lines.push(format!(
        "invert_colors={}",
        if postprocess_profile.invert_colors { 1 } else { 0 }
    ));
    lines.push(format!(
        "flip_horizontal={}",
        if postprocess_profile.flip_horizontal { 1 } else { 0 }
    ));
    lines.push(format!(
        "flip_vertical={}",
        if postprocess_profile.flip_vertical { 1 } else { 0 }
    ));
    lines.push(format!("hue_rotation={}", postprocess_profile.hue_rotation));
    lines.push(format!("render_scale={}", postprocess_profile.render_scale));
    lines.push(format!(
        "color_precision={}",
        postprocess_profile.color_precision.name()
    ));
    lines.push(format!(
        "anti_aliasing={}",
        postprocess_profile.anti_aliasing.name()
    ));
    lines.push(format!(
        "dithering={}",
        postprocess_profile.dithering.name()
    ));
    lines.push(format!("subtitles={}", if subtitles { 1 } else { 0 }));
    lines.push(format!("placement={}", subtitle_placement.name()));
    lines.push(String::new());
    lines.join("\n")
}

fn format_animation_speed(speed: f32) -> String {
    if speed.fract() == 0.0 {
        format!("×{speed:.1}")
    } else {
        format!("×{speed}")
    }
}

fn sanitize_metadata_value(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '\n' | '\r' | '\0' => ' ',
            _ => character,
        })
        .collect()
}

fn resolve_shader_fps(
    global_rendered_fps: u32,
    fps_policy_entries: &[crate::load_config::FpsPolicyEntry],
    policy_id: i64,
    shader_name: &str,
    source_path: Option<&Path>,
) -> u32 {
    if policy_id > 0 {
        if let Some(entry) = fps_policy_entries
            .iter()
            .find(|entry| entry.policy_id == policy_id)
        {
            return entry.rendered_fps.max(1);
        }
    }

    if let Some(source_path) = source_path {
        if let Some(entry) = fps_policy_entries.iter().find(|entry| {
            entry
                .source_path
                .as_deref()
                .is_some_and(|policy_path| paths_refer_to_same_file(policy_path, source_path))
        }) {
            return entry.rendered_fps.max(1);
        }
    }

    fps_policy_entries
        .iter()
        .find(|entry| {
            entry.source_path.is_none()
                && entry.shader.eq_ignore_ascii_case(shader_name)
        })
        .map(|entry| entry.rendered_fps)
        .unwrap_or(global_rendered_fps)
        .max(1)
}

fn read_audio_bands(
    shared: &Option<crate::audio_backend::SharedAudioBands>,
) -> crate::analyze_audio::AudioBands {
    let Some(shared) = shared.as_ref() else {
        return crate::analyze_audio::AudioBands::default();
    };

    shared
        .read()
        .map(|bands| *bands)
        .unwrap_or_default()
}

fn audio_bands_equal(
    left: crate::analyze_audio::AudioBands,
    right: crate::analyze_audio::AudioBands,
) -> bool {
    const EPSILON: f32 = 0.0005;

    (left.bass - right.bass).abs() < EPSILON
        && (left.midrange - right.midrange).abs() < EPSILON
        && (left.treble - right.treble).abs() < EPSILON
}

fn publish_audio_bands(
    destination: &Path,
    temporary: &Path,
    bands: crate::analyze_audio::AudioBands,
) -> Result<(), String> {
    let contents = format!(
        "version=1\nbass={:.6}\nmidrange={:.6}\ntreble={:.6}\n",
        bands.bass.clamp(0.0, 1.0),
        bands.midrange.clamp(0.0, 1.0),
        bands.treble.clamp(0.0, 1.0),
    );

    publish_text_atomically(
        destination,
        temporary,
        &contents,
        "GNOME lock audio-band handoff",
    )
}

fn publish_text_atomically(
    destination: &Path,
    temporary: &Path,
    contents: &str,
    description: &str,
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
                    "Unable to create {description} '{}': {error}",
                    temporary.display(),
                )
            })?;

        file.write_all(contents.as_bytes()).map_err(|error| {
            format!(
                "Unable to write {description} '{}': {error}",
                temporary.display(),
            )
        })?;

        file.flush().map_err(|error| {
            format!(
                "Unable to flush {description} '{}': {error}",
                temporary.display(),
            )
        })?;
    }

    fs::rename(temporary, destination).map_err(|error| {
        format!(
            "Unable to publish {description} '{}' -> '{}': {error}",
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
