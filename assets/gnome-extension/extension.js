import Clutter from 'gi://Clutter';
import Cogl from 'gi://Cogl';
import GLib from 'gi://GLib';
import Gio from 'gi://Gio';
import GObject from 'gi://GObject';
import Shell from 'gi://Shell';
import St from 'gi://St';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';


let test12ShaderDeclarations = null;

const ScreenshaverTest12GLSLEffect = GObject.registerClass(
class ScreenshaverTest12GLSLEffect extends Shell.GLSLEffect {
    vfunc_build_pipeline() {
        if (!test12ShaderDeclarations)
            throw new Error('Test #12 shader source was not loaded');

        this.add_glsl_snippet(
            Cogl.SnippetHook.FRAGMENT,
            test12ShaderDeclarations,
            `
                vec2 uv = cogl_tex_coord0_in.st;
                cogl_color_out = screenshaver_test12_fragment(uv, u_time);
            `,
            true
        );
    }
});

const TEST12_SHADER_FILENAME = 'test12_shader.glsl';
const CONTROL_FILENAME = 'screenshaver-lock-control.bin';
const FRAME_FILENAME_PREFIX = 'screenshaver-lock-frame-';
const FRAME_FILENAME_SUFFIX = '.rgba';
const CONTROL_MAGIC = [0x53, 0x48, 0x56, 0x52, 0x47, 0x4e, 0x46, 0x31]; // SHVRGNF1
const CONTROL_VERSION = 1;
const CONTROL_BYTES = 64;
const CONTROL_SESSION_ID_BYTES = 16;
const POLL_INTERVAL_MS = 33;
const SHADER_TICK_INTERVAL_MS = 8;
const SHADER_METRICS_REPORT_INTERVAL_US = 5 * 1000000;
const POWER_SAVE_FALLBACK_INTERVAL_MS = 1000;
const POST_WAKE_POWER_SAVE_MIN_DELAY_MS = 10000;
const POST_BLANK_SCREENSHIELD_WAKE_DELAY_MS = 250;
const RUNTIME_MARKER_FILENAME = 'screenshaver-gnome-lock.active';
const RUNTIME_MARKER_VERSION = 1;
const SESSION_VALIDATION_INTERVAL_MS = 1000;
const SCREENSHIELD_DIAGNOSTIC_INTERVAL_MS = 1000;
const SCREENSHIELD_DIAGNOSTIC_MAX_DEPTH = 5;
const SCREENSHIELD_DIAGNOSTIC_MAX_ACTORS = 120;

const CONTROL_MAGIC_OFFSET = 0;
const CONTROL_VERSION_OFFSET = 8;
const CONTROL_SIZE_OFFSET = 12;
const CONTROL_WIDTH_OFFSET = 16;
const CONTROL_HEIGHT_OFFSET = 20;
const CONTROL_ROWSTRIDE_OFFSET = 24;
const CONTROL_FRAME_BYTES_OFFSET = 28;
const CONTROL_FRAME_COUNTER_OFFSET = 32;
const CONTROL_SESSION_ID_OFFSET = 36;

export default class ScreenshaverExtension extends Extension {
    enable() {
        console.log('[Screenshaver] GNOME Shell extension enabled');

        this._lockActor = null;
        this._imageContent = null;
        this._shaderEffect = null;
        this._shaderUniformTime = -1;
        this._shaderTickSource = null;
        this._shaderStartedUs = 0;
        this._shaderTicks = 0;
        this._pollSource = null;
        this._transportGeneration = 0;
        this._lastFrameCounter = 0;
        this._displayedFrames = 0;
        this._screenShieldWakeIssued = false;
        this._postWakePowerSaveCorrectionArmed = false;
        this._postWakeNormalObserved = false;
        this._postWakeNormalObservedUs = 0;
        this._postWakePowerSaveCorrectionIssued = false;
        this._postWakePowerSaveCorrectionInFlight = false;
        this._postBlankScreenShieldWakeIssued = false;
        this._postBlankScreenShieldWakeSource = null;
        this._postBlankScreenShieldWakeCompletedUs = 0;
        this._delayedBlankOpacityNudgeIssued = false;
        this._delayedBlankOpacityNudgeSource = null;
        this._refreshCalls = 0;
        this._uploadAttempts = 0;
        this._uploadSuccesses = 0;
        this._transportErrorLogged = false;
        this._sessionValidationSource = null;
        this._sessionWaitSource = null;
        this._activeSessionId = null;
        this._lastObservedPowerSaveMode = null;
        this._powerSaveSignalId = 0;
        this._powerSaveFallbackSource = null;
        this._powerSaveFallbackQueryInFlight = false;
        this._idleInhibitCookie = 0;
        this._idleInhibitRequestGeneration = 0;
        this._idleInhibitRequestPending = false;
        this._idleInhibitWaitState = null;
        this._screenShieldDiagnosticSource = null;
        this._screenShieldDiagnosticPrevious = new Map();
        this._screenShieldDiagnosticStartedUs = 0;

        this._sessionModeSignal = Main.sessionMode.connect(
            'updated',
            () => this._onSessionModeChanged()
        );

        this._onSessionModeChanged();
    }

    disable() {
        console.log('[Screenshaver] GNOME Shell extension disabled');

        if (this._sessionModeSignal) {
            Main.sessionMode.disconnect(this._sessionModeSignal);
            this._sessionModeSignal = null;
        }

        this._removeLockActor();
    }

    _onSessionModeChanged() {
        const mode = Main.sessionMode.currentMode;

        console.log(`[Screenshaver] GNOME session mode changed: ${mode}`);

        if (mode === 'unlock-dialog')
            this._beginSessionParticipation();
        else {
            this._stopSessionWait();
            this._removeLockActor();
        }
    }

    _beginSessionParticipation() {
        if (this._lockActor)
            return;

        if (this._activateSessionIfValid())
            return;

        if (this._sessionWaitSource)
            return;

        console.log(
            '[Screenshaver] GNOME lock session has no valid Screenshaver runtime handshake; stock GNOME lock screen remains active'
        );

        this._sessionWaitSource = GLib.timeout_add(
            GLib.PRIORITY_DEFAULT,
            SESSION_VALIDATION_INTERVAL_MS,
            () => {
                if (Main.sessionMode.currentMode !== 'unlock-dialog') {
                    this._sessionWaitSource = null;
                    return GLib.SOURCE_REMOVE;
                }

                if (this._activateSessionIfValid()) {
                    this._sessionWaitSource = null;
                    return GLib.SOURCE_REMOVE;
                }

                this._ensureIdleInhibitor();
                return GLib.SOURCE_CONTINUE;
            }
        );
    }

    _activateSessionIfValid() {
        const session = this._validateRuntimeSession();

        if (!session)
            return false;

        this._activeSessionId = session.sessionId;
        this._createLockActor();

        if (!this._lockActor) {
            this._activeSessionId = null;
            return false;
        }

        this._startSessionValidation();

        console.log(
            `[Screenshaver] GNOME runtime handshake accepted for pid=${session.pid}`
        );

        this._ensureIdleInhibitor();

        return true;
    }

    _startSessionValidation() {
        if (this._sessionValidationSource)
            return;

        this._sessionValidationSource = GLib.timeout_add(
            GLib.PRIORITY_DEFAULT,
            SESSION_VALIDATION_INTERVAL_MS,
            () => {
                if (!this._lockActor) {
                    this._sessionValidationSource = null;
                    return GLib.SOURCE_REMOVE;
                }

                const session = this._validateRuntimeSession();

                if (!session || session.sessionId !== this._activeSessionId) {
                    console.log(
                        '[Screenshaver] GNOME runtime handshake lost; withdrawing Screenshaver lock-screen presentation'
                    );

                    this._sessionValidationSource = null;
                    this._removeLockActor();

                    if (Main.sessionMode.currentMode === 'unlock-dialog')
                        this._beginSessionParticipation();

                    return GLib.SOURCE_REMOVE;
                }

                this._ensureIdleInhibitor();
                return GLib.SOURCE_CONTINUE;
            }
        );
    }

    _stopSessionValidation() {
        if (this._sessionValidationSource) {
            GLib.source_remove(this._sessionValidationSource);
            this._sessionValidationSource = null;
        }
    }

    _stopSessionWait() {
        if (this._sessionWaitSource) {
            GLib.source_remove(this._sessionWaitSource);
            this._sessionWaitSource = null;
        }
    }

    _validateRuntimeSession() {
        const marker = this._readRuntimeMarker();

        if (!marker)
            return null;

        if (!GLib.file_test(`/proc/${marker.pid}`, GLib.FileTest.EXISTS))
            return null;

        // Runtime ownership and presentation readiness are intentionally
        // separate. The marker proves that a live Screenshaver process owns
        // this lock session. During the Shell.GLSLEffect diagnostic, the
        // external frame-transport control record is deliberately ignored.
        return marker;
    }

    _readRuntimeMarker() {
        const markerPath = GLib.build_filenamev([
            GLib.get_user_runtime_dir(),
            RUNTIME_MARKER_FILENAME,
        ]);

        try {
            const file = Gio.File.new_for_path(markerPath);
            const [ok, contents] = file.load_contents(null);

            if (!ok || !contents)
                return null;

            const markerText = new TextDecoder().decode(contents);
            const values = new Map();

            for (const rawLine of markerText.split('\n')) {
                const line = rawLine.trim();

                if (!line)
                    continue;

                const separator = line.indexOf('=');

                if (separator <= 0)
                    continue;

                values.set(
                    line.slice(0, separator),
                    line.slice(separator + 1)
                );
            }

            const version = Number.parseInt(values.get('version') ?? '', 10);
            const pid = Number.parseInt(values.get('pid') ?? '', 10);
            const sessionId = values.get('session_id') ?? '';

            if (version !== RUNTIME_MARKER_VERSION)
                return null;

            if (!Number.isInteger(pid) || pid <= 0)
                return null;

            if (!/^[0-9a-fA-F]{32}$/.test(sessionId))
                return null;

            return {
                pid,
                sessionId: sessionId.toLowerCase(),
            };
        } catch (_) {
            return null;
        }
    }

    _createLockActor() {
        if (this._lockActor)
            return;

        const dialog = Main.screenShield?._dialog;

        if (!dialog) {
            console.log('[Screenshaver] ERROR: UnlockDialog unavailable');
            return;
        }

        const dialogChildren = dialog.get_children();
        const backgroundGroup = dialogChildren[0];

        if (!backgroundGroup) {
            console.log('[Screenshaver] ERROR: UnlockDialog background group unavailable');
            return;
        }

        // Test #16: paint a simple solid actor and execute GLSL loaded from a
        // separate extension-side shader file through Shell.GLSLEffect/Cogl.
        // This is the first source-ingestion bridge; it intentionally leaves
        // shared Rust rendering/preprocessing code untouched.
        this._lockActor = new St.Widget({
            reactive: false,
            can_focus: false,
            style: 'background-color: black;',
        });

        this._lockActor.set_position(0, 0);
        this._lockActor.set_size(dialog.width, dialog.height);

        try {
            const shaderPath = GLib.build_filenamev([this.path, TEST12_SHADER_FILENAME]);
            const shaderFile = Gio.File.new_for_path(shaderPath);
            const [shaderOk, shaderBytes] = shaderFile.load_contents(null);

            if (!shaderOk)
                throw new Error(`Unable to read ${shaderPath}`);

            test12ShaderDeclarations = new TextDecoder().decode(shaderBytes);

            if (!test12ShaderDeclarations.includes('screenshaver_test12_fragment'))
                throw new Error(
                    `${TEST12_SHADER_FILENAME} does not define screenshaver_test12_fragment()`
                );

            console.log(
                `[Screenshaver] Test #16 loaded GLSL source: ${shaderPath} (${shaderBytes.length} bytes)`
            );

            this._shaderEffect = new ScreenshaverTest12GLSLEffect();
            this._shaderUniformTime = this._shaderEffect.get_uniform_location('u_time');
            this._shaderEffect.set_uniform_float(
                this._shaderUniformTime,
                1,
                [0.0]
            );
            this._lockActor.add_effect_with_name(
                'screenshaver-diagnostic-shader',
                this._shaderEffect
            );
        } catch (error) {
            console.log(
                `[Screenshaver] ERROR: Unable to create Test #12 Shell.GLSLEffect: ${error}`
            );
            this._lockActor.destroy();
            this._lockActor = null;
            this._shaderEffect = null;
            return;
        }

        backgroundGroup.add_child(this._lockActor);

        console.log(
            '[Screenshaver] Test #16 shader actor added above GNOME lock background'
        );

        // Test #16: observe GNOME's native ScreenShield/UnlockDialog actor
        // hierarchy without mutating it.  This diagnostic records only safe
        // scalar actor state and logs subsequent changes once per second.
        this._startScreenShieldHierarchyDiagnostic();

        // Preserve the already-proven GNOME lock/power-management handling.
        this._startPowerSaveRecovery();

        this._shaderStartedUs = GLib.get_monotonic_time();
        this._shaderTicks = 0;

        // Test #16 instrumentation: keep the proven Test #12 GLib timeout
        // measurements only. The Test #14 Clutter.Timeline probe has been
        // removed so this run isolates the GNOME session idle inhibitor.
        this._shaderMetricsWindowStartedUs = this._shaderStartedUs;
        this._shaderMetricsWindowTicks = 0;
        this._shaderMetricsPreviousTickUs = null;
        this._shaderMetricsMinDeltaUs = Number.POSITIVE_INFINITY;
        this._shaderMetricsMaxDeltaUs = 0;

        console.log(
            `[Screenshaver] Test #16 requested shader tick interval: ${SHADER_TICK_INTERVAL_MS}ms (~${Math.round(1000 / SHADER_TICK_INTERVAL_MS)} Hz maximum)`
        );

        this._shaderTickSource = GLib.timeout_add(
            GLib.PRIORITY_DEFAULT,
            SHADER_TICK_INTERVAL_MS,
            () => {
                if (!this._lockActor || !this._shaderEffect) {
                    this._shaderTickSource = null;
                    return GLib.SOURCE_REMOVE;
                }

                const elapsedSeconds =
                    (GLib.get_monotonic_time() - this._shaderStartedUs) / 1000000.0;

                try {
                    this._shaderEffect.set_uniform_float(
                        this._shaderUniformTime,
                        1,
                        [elapsedSeconds]
                    );
                    this._shaderEffect.queue_repaint();
                    this._lockActor.queue_redraw();
                } catch (error) {
                    console.log(
                        `[Screenshaver] Shell.GLSLEffect animation update failed: ${error}`
                    );
                    this._shaderTickSource = null;
                    return GLib.SOURCE_REMOVE;
                }

                this._shaderTicks++;

                const tickNowUs = GLib.get_monotonic_time();
                this._shaderMetricsWindowTicks++;

                if (this._shaderMetricsPreviousTickUs !== null) {
                    const deltaUs = tickNowUs - this._shaderMetricsPreviousTickUs;
                    this._shaderMetricsMinDeltaUs = Math.min(
                        this._shaderMetricsMinDeltaUs,
                        deltaUs
                    );
                    this._shaderMetricsMaxDeltaUs = Math.max(
                        this._shaderMetricsMaxDeltaUs,
                        deltaUs
                    );
                }

                this._shaderMetricsPreviousTickUs = tickNowUs;

                const metricsElapsedUs =
                    tickNowUs - this._shaderMetricsWindowStartedUs;

                if (metricsElapsedUs >= SHADER_METRICS_REPORT_INTERVAL_US) {
                    const metricsElapsedSeconds = metricsElapsedUs / 1000000.0;
                    const effectiveHz =
                        this._shaderMetricsWindowTicks / metricsElapsedSeconds;
                    const averageIntervalMs = effectiveHz > 0
                        ? 1000.0 / effectiveHz
                        : 0.0;
                    const minIntervalMs = Number.isFinite(this._shaderMetricsMinDeltaUs)
                        ? this._shaderMetricsMinDeltaUs / 1000.0
                        : 0.0;
                    const maxIntervalMs = this._shaderMetricsMaxDeltaUs / 1000.0;

                    console.log(
                        `[Screenshaver] Test #16 timing: requested=${SHADER_TICK_INTERVAL_MS}ms callbacks=${this._shaderMetricsWindowTicks} elapsed=${metricsElapsedSeconds.toFixed(3)}s effective=${effectiveHz.toFixed(2)}Hz avg=${averageIntervalMs.toFixed(2)}ms min=${minIntervalMs.toFixed(2)}ms max=${maxIntervalMs.toFixed(2)}ms total_ticks=${this._shaderTicks}`
                    );

                    this._shaderMetricsWindowStartedUs = tickNowUs;
                    this._shaderMetricsWindowTicks = 0;
                    this._shaderMetricsMinDeltaUs = Number.POSITIVE_INFINITY;
                    this._shaderMetricsMaxDeltaUs = 0;
                }

                // Keep the established ScreenShield wake behavior, but gate it
                // on successful shader-effect animation ticks rather than on
                // receipt of external RGBA frames.
                this._maybeWakeScreenShieldForShader();

                if (this._shaderTicks === 1) {
                    console.log(
                        '[Screenshaver] First Test #16 shader frame requested'
                    );
                }

                return GLib.SOURCE_CONTINUE;
            }
        );
    }

    // Retained only for the current diagnostic branch.  The Rust producer may
    // continue publishing its known-good external frames, but this extension
    // deliberately does not consume them while Shell.GLSLEffect is under
    // test.  This isolates GNOME-native shader execution from transport issues.
    _refreshFrame() {
    }

    _readControlRecord(logErrors = true) {
        const controlPath = GLib.build_filenamev([
            GLib.get_user_runtime_dir(),
            CONTROL_FILENAME,
        ]);

        try {
            const file = Gio.File.new_for_path(controlPath);
            const [ok, data] = file.load_contents(null);

            if (!ok || !this._validateControlRecord(data, logErrors))
                return null;

            return {
                width: readU32LE(data, CONTROL_WIDTH_OFFSET),
                height: readU32LE(data, CONTROL_HEIGHT_OFFSET),
                rowstride: readU32LE(data, CONTROL_ROWSTRIDE_OFFSET),
                frameBytes: readU32LE(data, CONTROL_FRAME_BYTES_OFFSET),
                frameCounter: readU32LE(data, CONTROL_FRAME_COUNTER_OFFSET),
                sessionId: readSessionIdHex(data, CONTROL_SESSION_ID_OFFSET),
            };
        } catch (error) {
            if (logErrors && !this._transportErrorLogged) {
                console.log(`[Screenshaver] Waiting for GNOME file transport: ${error}`);
                this._transportErrorLogged = true;
            }
            return null;
        }
    }

    _validateControlRecord(data, logErrors = true) {
        if (!data || data.length < CONTROL_BYTES)
            return false;

        for (let i = 0; i < CONTROL_MAGIC.length; i++) {
            if (data[CONTROL_MAGIC_OFFSET + i] !== CONTROL_MAGIC[i]) {
                if (logErrors)
                    console.log('[Screenshaver] GNOME file-transport magic mismatch');
                return false;
            }
        }

        const version = readU32LE(data, CONTROL_VERSION_OFFSET);
        const controlBytes = readU32LE(data, CONTROL_SIZE_OFFSET);

        if (version !== CONTROL_VERSION || controlBytes !== CONTROL_BYTES) {
            if (logErrors) {
                console.log(
                    `[Screenshaver] Unsupported GNOME file-transport control record: ` +
                    `version=${version} size=${controlBytes}`
                );
            }
            return false;
        }

        return true;
    }

    _maybeWakeScreenShieldForShader() {
        if (this._screenShieldWakeIssued ||
            !this._lockActor ||
            this._shaderTicks === 0 ||
            Main.sessionMode.currentMode !== 'unlock-dialog') {
            return;
        }

        const screenShield = Main.screenShield;

        // A shader frame may arrive while GNOME is still completing the lock
        // transition. Do nothing until ScreenShield itself says the session is
        // securely locked and active. This mirrors the state in which genuine
        // user activity is routed through ScreenShield._wakeUpScreen().
        if (!screenShield?.locked || !screenShield?.active)
            return;

        if (typeof screenShield._wakeUpScreen !== 'function') {
            console.log(
                '[Screenshaver] GNOME ScreenShield wake method unavailable; leaving native lock state unchanged'
            );
            this._screenShieldWakeIssued = true;
            return;
        }

        this._screenShieldWakeIssued = true;
        console.log(
            '[Screenshaver] Requesting one-shot native ScreenShield wake after Shell.GLSLEffect activation'
        );

        // Arm before invoking _wakeUpScreen(): Mutter's 3 -> 0 notification can
        // arrive synchronously while GNOME handles the wake signal.  A failed
        // wake immediately disarms the correction again.
        this._postWakePowerSaveCorrectionArmed = true;
        this._postWakeNormalObserved = false;
        this._postWakeNormalObservedUs = 0;
        this._postWakePowerSaveCorrectionIssued = false;

        try {
            screenShield._wakeUpScreen();
            console.log('[Screenshaver] One-shot native ScreenShield wake completed');

            if (this._lastObservedPowerSaveMode === 0 && !this._postWakeNormalObserved) {
                this._postWakeNormalObserved = true;
                this._postWakeNormalObservedUs = GLib.get_monotonic_time();
                console.log('[Screenshaver] Post-wake PowerSave correction armed after NORMAL mode observed');
            }
        } catch (error) {
            this._postWakePowerSaveCorrectionArmed = false;
            this._postWakeNormalObserved = false;
            this._postWakeNormalObservedUs = 0;
            console.log(`[Screenshaver] One-shot native ScreenShield wake failed: ${error}`);
        }
    }


    _startScreenShieldHierarchyDiagnostic() {
        this._stopScreenShieldHierarchyDiagnostic();

        this._screenShieldDiagnosticPrevious = new Map();
        this._screenShieldDiagnosticStartedUs = GLib.get_monotonic_time();

        console.log('[Screenshaver] Test #16 ScreenShield hierarchy diagnostic started');
        this._sampleScreenShieldHierarchy(true);

        this._screenShieldDiagnosticSource = GLib.timeout_add(
            GLib.PRIORITY_DEFAULT,
            SCREENSHIELD_DIAGNOSTIC_INTERVAL_MS,
            () => {
                if (!this._lockActor || Main.sessionMode.currentMode !== 'unlock-dialog') {
                    this._screenShieldDiagnosticSource = null;
                    return GLib.SOURCE_REMOVE;
                }

                this._sampleScreenShieldHierarchy(false);
                return GLib.SOURCE_CONTINUE;
            }
        );
    }

    _stopScreenShieldHierarchyDiagnostic() {
        if (this._screenShieldDiagnosticSource) {
            GLib.source_remove(this._screenShieldDiagnosticSource);
            this._screenShieldDiagnosticSource = null;
        }

        this._screenShieldDiagnosticPrevious = new Map();
        this._screenShieldDiagnosticStartedUs = 0;
    }

    _sampleScreenShieldHierarchy(initial) {
        const dialog = Main.screenShield?._dialog;
        if (!dialog)
            return;

        const current = new Map();
        let actorCount = 0;

        const visit = (actor, path, depth) => {
            if (!actor || depth > SCREENSHIELD_DIAGNOSTIC_MAX_DEPTH ||
                actorCount >= SCREENSHIELD_DIAGNOSTIC_MAX_ACTORS)
                return;

            actorCount++;

            let name = '';
            let typeName = 'unknown';
            let visible = false;
            let opacity = 0;
            let width = 0;
            let height = 0;

            try {
                name = actor.get_name?.() ?? '';
            } catch (_) {
                name = '';
            }

            try {
                const gtype = actor.constructor?.$gtype;
                if (gtype)
                    typeName = GObject.type_name(gtype) ?? 'unknown';
                else if (actor.constructor?.name)
                    typeName = actor.constructor.name;
            } catch (_) {
                typeName = 'unknown';
            }

            try { visible = Boolean(actor.visible); } catch (_) {}
            try { opacity = Number(actor.opacity); } catch (_) {}
            try { width = Math.round(Number(actor.width)); } catch (_) {}
            try { height = Math.round(Number(actor.height)); } catch (_) {}

            const isShader = actor === this._lockActor;
            const state = `type=${typeName} name=${JSON.stringify(name)} visible=${visible} opacity=${opacity} size=${width}x${height} shader=${isShader}`;
            current.set(path, state);

            let children = [];
            try {
                children = actor.get_children?.() ?? [];
            } catch (_) {
                children = [];
            }

            for (let i = 0; i < children.length; i++)
                visit(children[i], `${path}/${i}`, depth + 1);
        };

        visit(dialog, 'dialog', 0);

        const elapsedMs = this._screenShieldDiagnosticStartedUs > 0
            ? Math.floor((GLib.get_monotonic_time() - this._screenShieldDiagnosticStartedUs) / 1000)
            : 0;

        if (initial) {
            console.log(
                `[Screenshaver] Test #16 ScreenShield snapshot t=${elapsedMs}ms actors=${current.size}`
            );
            for (const [path, state] of current)
                console.log(`[Screenshaver] Test #16 actor ${path} ${state}`);
        } else {
            for (const [path, state] of current) {
                const previous = this._screenShieldDiagnosticPrevious.get(path);
                if (previous !== state) {
                    console.log(
                        `[Screenshaver] Test #16 actor-change t=${elapsedMs}ms ${path} ${previous === undefined ? 'NEW ' : ''}${state}`
                    );
                }
            }

            for (const path of this._screenShieldDiagnosticPrevious.keys()) {
                if (!current.has(path)) {
                    console.log(
                        `[Screenshaver] Test #16 actor-change t=${elapsedMs}ms ${path} REMOVED`
                    );
                }
            }
        }

        this._screenShieldDiagnosticPrevious = current;
    }

    _ensureIdleInhibitor() {
        if (this._idleInhibitCookie || this._idleInhibitRequestPending)
            return;

        const unlockDialog = Main.sessionMode.currentMode === 'unlock-dialog';
        const hasActor = Boolean(this._lockActor);
        const hasHandshake = Boolean(this._activeSessionId);

        if (!hasActor || !unlockDialog || !hasHandshake) {
            const waitState = `actor=${hasActor} unlock-dialog=${unlockDialog} handshake=${hasHandshake}`;
            if (waitState !== this._idleInhibitWaitState) {
                this._idleInhibitWaitState = waitState;
                console.log(
                    `[Screenshaver] Test #16 idle inhibitor waiting: ${waitState}`
                );
            }
            return;
        }

        this._idleInhibitWaitState = null;
        this._acquireIdleInhibitor();
    }

    _acquireIdleInhibitor() {
        if (this._idleInhibitCookie || this._idleInhibitRequestPending)
            return;

        if (!this._lockActor ||
            Main.sessionMode.currentMode !== 'unlock-dialog' ||
            !this._activeSessionId) {
            return;
        }

        const requestGeneration = ++this._idleInhibitRequestGeneration;
        this._idleInhibitRequestPending = true;

        console.log('[Screenshaver] Test #16 requesting GNOME session idle inhibitor (flag=8)');

        try {
            Gio.DBus.session.call(
                'org.gnome.SessionManager',
                '/org/gnome/SessionManager',
                'org.gnome.SessionManager',
                'Inhibit',
                new GLib.Variant(
                    '(susu)',
                    [
                        'screenshaver@screenshaver',
                        0,
                        'Present Screenshaver shader while GNOME lock screen is active',
                        8,
                    ]
                ),
                new GLib.VariantType('(u)'),
                Gio.DBusCallFlags.NONE,
                -1,
                null,
                (_connection, result) => {
                    let cookie = 0;

                    try {
                        const reply = Gio.DBus.session.call_finish(result);
                        [cookie] = reply.deep_unpack();
                    } catch (error) {
                        if (requestGeneration === this._idleInhibitRequestGeneration)
                            this._idleInhibitRequestPending = false;

                        console.log(
                            `[Screenshaver] Test #16 GNOME session idle inhibitor request failed: ${error}`
                        );
                        return;
                    }

                    if (requestGeneration !== this._idleInhibitRequestGeneration ||
                        !this._lockActor ||
                        Main.sessionMode.currentMode !== 'unlock-dialog' ||
                        !this._activeSessionId) {
                        this._uninhibitCookie(cookie, 'late inhibitor reply after GNOME lock presentation ended');
                        return;
                    }

                    this._idleInhibitRequestPending = false;
                    this._idleInhibitCookie = cookie;
                    console.log(
                        `[Screenshaver] Test #16 GNOME session idle inhibitor acquired cookie=${cookie} flag=8`
                    );
                }
            );
        } catch (error) {
            if (requestGeneration === this._idleInhibitRequestGeneration)
                this._idleInhibitRequestPending = false;

            console.log(
                `[Screenshaver] Test #16 unable to dispatch GNOME session idle inhibitor request: ${error}`
            );
        }
    }

    _releaseIdleInhibitor() {
        // Invalidate any in-flight Inhibit() reply. If that reply arrives later,
        // its callback immediately Uninhibit()s the returned cookie.
        this._idleInhibitRequestGeneration++;
        this._idleInhibitRequestPending = false;
        this._idleInhibitWaitState = null;

        const cookie = this._idleInhibitCookie;
        this._idleInhibitCookie = 0;

        if (!cookie)
            return;

        this._uninhibitCookie(cookie, 'secure GNOME lock presentation ended');
    }

    _uninhibitCookie(cookie, reason) {
        if (!cookie)
            return;

        try {
            Gio.DBus.session.call(
                'org.gnome.SessionManager',
                '/org/gnome/SessionManager',
                'org.gnome.SessionManager',
                'Uninhibit',
                new GLib.Variant('(u)', [cookie]),
                null,
                Gio.DBusCallFlags.NONE,
                -1,
                null,
                (_connection, result) => {
                    try {
                        Gio.DBus.session.call_finish(result);
                        console.log(
                            `[Screenshaver] Test #16 GNOME session idle inhibitor released cookie=${cookie} (${reason})`
                        );
                    } catch (error) {
                        console.log(
                            `[Screenshaver] Test #16 GNOME session idle inhibitor release failed cookie=${cookie}: ${error}`
                        );
                    }
                }
            );
        } catch (error) {
            console.log(
                `[Screenshaver] Test #16 unable to dispatch GNOME session idle inhibitor release cookie=${cookie}: ${error}`
            );
        }
    }

    _startPowerSaveRecovery() {
        this._subscribePowerSaveModeChanges();

        // Diagnostic build: PropertiesChanged is the normal read-only path.
        // Keep a slow read-only poll as a safety net. No PowerSaveMode writes
        // are issued anywhere from the observation path.
        if (!this._powerSaveFallbackSource) {
            this._powerSaveFallbackSource = GLib.timeout_add(
                GLib.PRIORITY_DEFAULT,
                POWER_SAVE_FALLBACK_INTERVAL_MS,
                () => {
                    if (!this._lockActor) {
                        this._powerSaveFallbackSource = null;
                        return GLib.SOURCE_REMOVE;
                    }

                    this._samplePowerSaveModeFallback();
                    return GLib.SOURCE_CONTINUE;
                }
            );
        }

        this._samplePowerSaveModeFallback();
    }

    _stopPowerSaveRecovery() {
        this._unsubscribePowerSaveModeChanges();

        if (this._powerSaveFallbackSource) {
            GLib.source_remove(this._powerSaveFallbackSource);
            this._powerSaveFallbackSource = null;
        }

        this._powerSaveFallbackQueryInFlight = false;
        this._lastObservedPowerSaveMode = null;
    }

    _subscribePowerSaveModeChanges() {
        if (this._powerSaveSignalId)
            return;

        try {
            this._powerSaveSignalId = Gio.DBus.session.signal_subscribe(
                'org.gnome.Mutter.DisplayConfig',
                'org.freedesktop.DBus.Properties',
                'PropertiesChanged',
                '/org/gnome/Mutter/DisplayConfig',
                'org.gnome.Mutter.DisplayConfig',
                Gio.DBusSignalFlags.NONE,
                (_connection, _senderName, _objectPath, _interfaceName,
                    _signalName, parameters) => {
                    if (!this._lockActor)
                        return;

                    try {
                        const unpacked = parameters.deep_unpack();
                        const changedInterface = unpacked[0];
                        const changedProperties = unpacked[1];

                        if (changedInterface !== 'org.gnome.Mutter.DisplayConfig' ||
                            !changedProperties ||
                            !Object.prototype.hasOwnProperty.call(
                                changedProperties,
                                'PowerSaveMode'
                            )) {
                            return;
                        }

                        let value = changedProperties.PowerSaveMode;
                        if (value && typeof value.deep_unpack === 'function')
                            value = value.deep_unpack();

                        this._handlePowerSaveModeValue(value, true);
                    } catch (error) {
                        console.log(
                            `[Screenshaver] Unable to process Mutter PowerSaveMode change: ${error}`
                        );
                    }
                }
            );
        } catch (error) {
            this._powerSaveSignalId = 0;
            console.log(
                `[Screenshaver] Unable to watch Mutter PowerSaveMode changes; ` +
                `fallback polling remains active: ${error}`
            );
        }
    }

    _unsubscribePowerSaveModeChanges() {
        if (!this._powerSaveSignalId)
            return;

        try {
            Gio.DBus.session.signal_unsubscribe(this._powerSaveSignalId);
        } catch (_) {
            // Extension teardown must remain best-effort.
        }

        this._powerSaveSignalId = 0;
    }

    _handlePowerSaveModeValue(value, fromSignal) {
        const previousPowerSaveMode = this._lastObservedPowerSaveMode;
        this._lastObservedPowerSaveMode = value;

        if (!this._lockActor)
            return;

        if (previousPowerSaveMode !== value) {
            console.log(
                `[Screenshaver] Diagnostic PowerSaveMode: ${previousPowerSaveMode ?? 'unknown'} -> ${value} ` +
                `source=${fromSignal ? 'signal' : 'poll'}`
            );
        }

        // Test #8: Test #7 proved that another ScreenShield wake merely restarts
        // GNOME's roughly 15-second blank countdown.  Re-test the earlier
        // presentation-invalidation technique instead: on the first delayed
        // NORMAL -> BLANK transition, momentarily change the lock actor opacity
        // from 255 to 254 and immediately restore it to 255, forcing a Clutter
        // paint-state change without waking ScreenShield or writing PowerSaveMode.
        if (this._postBlankScreenShieldWakeCompletedUs > 0 &&
            !this._delayedBlankOpacityNudgeIssued &&
            !this._delayedBlankOpacityNudgeSource &&
            previousPowerSaveMode === 0 && value === 3 &&
            Main.sessionMode.currentMode === 'unlock-dialog' &&
            Main.screenShield?.locked && Main.screenShield?.active) {
            const elapsedSincePostBlankWakeUs =
                GLib.get_monotonic_time() - this._postBlankScreenShieldWakeCompletedUs;

            if (elapsedSincePostBlankWakeUs >= POST_WAKE_POWER_SAVE_MIN_DELAY_MS * 1000) {
                console.log(
                    `[Screenshaver] Observed delayed post-lock blank transition 0 -> 3 after ${Math.floor(elapsedSincePostBlankWakeUs / 1000)}ms`
                );
                this._scheduleDelayedBlankOpacityNudge();
                return;
            }
        }

        if (!this._postWakePowerSaveCorrectionArmed ||
            this._postWakePowerSaveCorrectionIssued ||
            Main.sessionMode.currentMode !== 'unlock-dialog' ||
            !Main.screenShield?.locked ||
            !Main.screenShield?.active) {
            return;
        }

        // GNOME Settings Daemon temporarily returns the display to NORMAL for
        // POWER_UP_TIME_ON_AC (15 seconds) after ScreenShield's WakeUpScreen.
        // Only after that NORMAL state has actually been observed do we accept
        // one subsequent NORMAL -> BLANK (PowerSaveMode 0 -> 3) transition as
        // the known temporary-unidle expiry that freezes Screenshaver output.
        if (value === 0) {
            if (!this._postWakeNormalObserved) {
                this._postWakeNormalObserved = true;
                this._postWakeNormalObservedUs = GLib.get_monotonic_time();
                console.log('[Screenshaver] Post-wake PowerSave correction armed after NORMAL mode observed');
            }
            return;
        }

        if (previousPowerSaveMode === 0 && value === 3 && this._postWakeNormalObserved) {
            const elapsedUs = GLib.get_monotonic_time() - this._postWakeNormalObservedUs;
            const minimumDelayUs = POST_WAKE_POWER_SAVE_MIN_DELAY_MS * 1000;

            // Test #6: the previous diagnostic proved that this early 0 -> 3
            // transition is the one that visually blanks the shader even
            // though the Shell.GLSLEffect itself remains alive.  Do not fight
            // Mutter by continuously forcing PowerSaveMode.  Instead, let the
            // blank transition settle, disarm the older PowerSave correction,
            // then issue exactly one deferred ScreenShield wake -- the same
            // native operation that user activity invokes.
            if (elapsedUs < minimumDelayUs) {
                this._postWakePowerSaveCorrectionIssued = true;
                this._postWakePowerSaveCorrectionArmed = false;
                console.log(
                    `[Screenshaver] Observed initial lock blank transition 0 -> 3 after ${Math.floor(elapsedUs / 1000)}ms`
                );
                this._schedulePostBlankScreenShieldWake();
                return;
            }

            this._postWakePowerSaveCorrectionIssued = true;
            this._postWakePowerSaveCorrectionArmed = false;
            console.log(
                `[Screenshaver] Correcting one post-wake PowerSaveMode 0 -> 3 transition after ${Math.floor(elapsedUs / 1000)}ms`
            );
            this._setPostWakePowerSaveModeNormal();
        }
    }


    _schedulePostBlankScreenShieldWake() {
        if (this._postBlankScreenShieldWakeIssued ||
            this._postBlankScreenShieldWakeSource ||
            !this._lockActor) {
            return;
        }

        this._postBlankScreenShieldWakeIssued = true;
        console.log(
            `[Screenshaver] Scheduling one-shot post-blank ScreenShield wake in ${POST_BLANK_SCREENSHIELD_WAKE_DELAY_MS}ms`
        );

        this._postBlankScreenShieldWakeSource = GLib.timeout_add(
            GLib.PRIORITY_DEFAULT,
            POST_BLANK_SCREENSHIELD_WAKE_DELAY_MS,
            () => {
                this._postBlankScreenShieldWakeSource = null;

                if (!this._lockActor ||
                    Main.sessionMode.currentMode !== 'unlock-dialog' ||
                    !Main.screenShield?.locked ||
                    !Main.screenShield?.active) {
                    console.log(
                        '[Screenshaver] One-shot post-blank ScreenShield wake skipped because secure lock state is no longer active'
                    );
                    return GLib.SOURCE_REMOVE;
                }

                const screenShield = Main.screenShield;
                if (typeof screenShield._wakeUpScreen !== 'function') {
                    console.log(
                        '[Screenshaver] Post-blank ScreenShield wake method unavailable; leaving native lock state unchanged'
                    );
                    return GLib.SOURCE_REMOVE;
                }

                try {
                    console.log('[Screenshaver] Requesting one-shot post-blank native ScreenShield wake');
                    screenShield._wakeUpScreen();
                    this._postBlankScreenShieldWakeCompletedUs = GLib.get_monotonic_time();
                    console.log('[Screenshaver] One-shot post-blank native ScreenShield wake completed');
                } catch (error) {
                    console.log(`[Screenshaver] One-shot post-blank native ScreenShield wake failed: ${error}`);
                }

                return GLib.SOURCE_REMOVE;
            }
        );
    }

    _scheduleDelayedBlankOpacityNudge() {
        if (this._delayedBlankOpacityNudgeIssued ||
            this._delayedBlankOpacityNudgeSource ||
            !this._lockActor) {
            return;
        }

        this._delayedBlankOpacityNudgeIssued = true;
        console.log(
            `[Screenshaver] Scheduling delayed lock-screen opacity nudge in ${POST_BLANK_SCREENSHIELD_WAKE_DELAY_MS}ms`
        );

        this._delayedBlankOpacityNudgeSource = GLib.timeout_add(
            GLib.PRIORITY_DEFAULT,
            POST_BLANK_SCREENSHIELD_WAKE_DELAY_MS,
            () => {
                this._delayedBlankOpacityNudgeSource = null;

                if (!this._lockActor ||
                    Main.sessionMode.currentMode !== 'unlock-dialog' ||
                    !Main.screenShield?.locked ||
                    !Main.screenShield?.active) {
                    console.log(
                        '[Screenshaver] Delayed opacity nudge skipped because secure lock state is no longer active'
                    );
                    return GLib.SOURCE_REMOVE;
                }

                try {
                    console.log('[Screenshaver] Applying lock-screen opacity nudge 255 -> 254');
                    this._lockActor.opacity = 254;
                    this._lockActor.queue_redraw();

                    GLib.idle_add(GLib.PRIORITY_DEFAULT_IDLE, () => {
                        if (this._lockActor &&
                            Main.sessionMode.currentMode === 'unlock-dialog' &&
                            Main.screenShield?.locked &&
                            Main.screenShield?.active) {
                            this._lockActor.opacity = 255;
                            this._lockActor.queue_redraw();
                            console.log('[Screenshaver] Restored lock-screen opacity 254 -> 255');
                        }

                        return GLib.SOURCE_REMOVE;
                    });
                } catch (error) {
                    console.log(`[Screenshaver] Lock-screen opacity nudge failed: ${error}`);
                }

                return GLib.SOURCE_REMOVE;
            }
        );
    }

    _setPostWakePowerSaveModeNormal() {
        if (this._postWakePowerSaveCorrectionInFlight || !this._lockActor)
            return;

        this._postWakePowerSaveCorrectionInFlight = true;

        try {
            Gio.DBus.session.call(
                'org.gnome.Mutter.DisplayConfig',
                '/org/gnome/Mutter/DisplayConfig',
                'org.freedesktop.DBus.Properties',
                'Set',
                new GLib.Variant(
                    '(ssv)',
                    [
                        'org.gnome.Mutter.DisplayConfig',
                        'PowerSaveMode',
                        new GLib.Variant('i', 0),
                    ]
                ),
                null,
                Gio.DBusCallFlags.NONE,
                2000,
                null,
                (connection, result) => {
                    this._postWakePowerSaveCorrectionInFlight = false;

                    try {
                        connection.call_finish(result);
                        console.log('[Screenshaver] One-shot post-wake PowerSaveMode correction completed');
                    } catch (error) {
                        if (this._lockActor)
                            console.log(`[Screenshaver] One-shot post-wake PowerSaveMode correction failed: ${error}`);
                    }
                }
            );
        } catch (error) {
            this._postWakePowerSaveCorrectionInFlight = false;
            console.log(`[Screenshaver] Unable to dispatch one-shot post-wake PowerSaveMode correction: ${error}`);
        }
    }

    _samplePowerSaveModeFallback() {
        if (this._powerSaveFallbackQueryInFlight || !this._lockActor)
            return;

        this._powerSaveFallbackQueryInFlight = true;

        try {
            Gio.DBus.session.call(
                'org.gnome.Mutter.DisplayConfig',
                '/org/gnome/Mutter/DisplayConfig',
                'org.freedesktop.DBus.Properties',
                'Get',
                new GLib.Variant(
                    '(ss)',
                    ['org.gnome.Mutter.DisplayConfig', 'PowerSaveMode']
                ),
                new GLib.VariantType('(v)'),
                Gio.DBusCallFlags.NONE,
                2000,
                null,
                (connection, result) => {
                    this._powerSaveFallbackQueryInFlight = false;

                    try {
                        const reply = connection.call_finish(result);
                        const unpacked = reply.deep_unpack();
                        let value = unpacked[0];

                        if (value && typeof value.deep_unpack === 'function')
                            value = value.deep_unpack();

                        this._handlePowerSaveModeValue(value, false);
                    } catch (error) {
                        if (this._lockActor) {
                            console.log(
                                `[Screenshaver] Mutter PowerSaveMode fallback query failed: ${error}`
                            );
                        }
                    }
                }
            );
        } catch (error) {
            this._powerSaveFallbackQueryInFlight = false;
            console.log(
                `[Screenshaver] Unable to dispatch Mutter PowerSaveMode fallback query: ${error}`
            );
        }
    }

    _removeLockActor() {
        this._stopSessionValidation();
        this._stopPowerSaveRecovery();
        this._stopScreenShieldHierarchyDiagnostic();
        this._releaseIdleInhibitor();

        if (this._postBlankScreenShieldWakeSource) {
            GLib.source_remove(this._postBlankScreenShieldWakeSource);
            this._postBlankScreenShieldWakeSource = null;
        }

        if (this._delayedBlankOpacityNudgeSource) {
            GLib.source_remove(this._delayedBlankOpacityNudgeSource);
            this._delayedBlankOpacityNudgeSource = null;
        }


        if (this._shaderTickSource) {
            GLib.source_remove(this._shaderTickSource);
            this._shaderTickSource = null;
        }

        if (this._pollSource) {
            GLib.source_remove(this._pollSource);
            this._pollSource = null;
        }

        if (this._lockActor) {
            console.log('[Screenshaver] Removing Shell.GLSLEffect diagnostic lock actor');
            this._lockActor.destroy();
            this._lockActor = null;
        }

        this._imageContent = null;
        this._shaderEffect = null;
        this._shaderUniformTime = -1;
        this._shaderStartedUs = 0;
        this._shaderTicks = 0;
        this._displayedFrames = 0;
        this._refreshCalls = 0;
        this._uploadAttempts = 0;
        this._uploadSuccesses = 0;
        this._transportErrorLogged = false;
        this._lastObservedPowerSaveMode = null;
        this._postWakePowerSaveCorrectionArmed = false;
        this._postWakeNormalObserved = false;
        this._postWakeNormalObservedUs = 0;
        this._postWakePowerSaveCorrectionIssued = false;
        this._postWakePowerSaveCorrectionInFlight = false;
        this._postBlankScreenShieldWakeIssued = false;
        this._postBlankScreenShieldWakeSource = null;
        this._postBlankScreenShieldWakeCompletedUs = 0;
        this._delayedBlankOpacityNudgeIssued = false;
        this._delayedBlankOpacityNudgeSource = null;
        this._powerSaveFallbackQueryInFlight = false;
        this._activeSessionId = null;
        this._transportGeneration++;
        this._lastFrameCounter = 0;
    }
}


function readSessionIdHex(data, offset) {
    if (!data || data.length < offset + CONTROL_SESSION_ID_BYTES)
        return null;

    let result = '';

    for (let i = 0; i < CONTROL_SESSION_ID_BYTES; i++)
        result += data[offset + i].toString(16).padStart(2, '0');

    return result;
}

function readU32LE(data, offset) {
    return (
        data[offset]
        | (data[offset + 1] << 8)
        | (data[offset + 2] << 16)
        | (data[offset + 3] << 24)
    ) >>> 0;
}
