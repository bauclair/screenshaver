import Cogl from 'gi://Cogl';
import GLib from 'gi://GLib';
import Gio from 'gi://Gio';
import St from 'gi://St';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const CONTROL_FILENAME = 'screenshaver-lock-control.bin';
const FRAME_FILENAME_PREFIX = 'screenshaver-lock-frame-';
const FRAME_FILENAME_SUFFIX = '.rgba';
const CONTROL_MAGIC = [0x53, 0x48, 0x56, 0x52, 0x47, 0x4e, 0x46, 0x31]; // SHVRGNF1
const CONTROL_VERSION = 1;
const CONTROL_BYTES = 64;
const CONTROL_SESSION_ID_BYTES = 16;
const POLL_INTERVAL_MS = 33;
const POWER_SAVE_FALLBACK_INTERVAL_MS = 1000;
const POWER_SAVE_STARTUP_SAMPLE_INTERVAL_MS = 500;
const POWER_SAVE_STARTUP_MAX_RESET_ATTEMPTS = 4;
const POWER_SAVE_STARTUP_WINDOW_MS = 6000;
const RUNTIME_MARKER_FILENAME = 'screenshaver-gnome-lock.active';
const RUNTIME_MARKER_VERSION = 1;
const SESSION_VALIDATION_INTERVAL_MS = 1000;
const STARTUP_DIAGNOSTIC_INTERVAL_MS = 200;
const STARTUP_DIAGNOSTIC_WINDOW_MS = 20000;

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
        this._pollSource = null;
        this._frameReadInFlight = false;
        this._pendingFrameCounter = 0;
        this._frameReadCancellable = null;
        this._transportGeneration = 0;
        this._lastFrameCounter = 0;
        this._displayedFrames = 0;
        this._refreshCalls = 0;
        this._uploadAttempts = 0;
        this._uploadSuccesses = 0;
        this._transportErrorLogged = false;
        this._sessionValidationSource = null;
        this._sessionWaitSource = null;
        this._activeSessionId = null;
        this._lastObservedPowerSaveMode = null;
        this._powerSaveStartupSource = null;
        this._powerSaveStartupDeadlineUs = 0;
        this._powerSaveStartupResetAttempts = 0;
        this._powerSaveResetInFlight = false;
        this._powerWakeCycleCount = 0;
        this._pendingPowerWakeCycle = 0;
        this._lastPowerWakeAttemptUs = 0;
        this._powerSaveSignalId = 0;
        this._powerSaveFallbackSource = null;
        this._powerSaveFallbackQueryInFlight = false;
        this._startupDiagnosticSource = null;
        this._startupDiagnosticDeadlineUs = 0;
        this._startupDiagnosticLastState = '';
        this._startupDiagnosticLastLogUs = 0;

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

        // Runtime ownership and frame-transport readiness are intentionally
        // separate.  The marker proves that a live Screenshaver process owns
        // this lock session.  The control record is presentation state and may
        // be absent briefly while the producer initializes or atomically
        // replaces it; that must not invalidate the runtime handshake or tear
        // down the GNOME lock actor.  _refreshFrame() independently validates
        // the control record and requires its session ID to match this marker
        // before displaying any frame.
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

        // The final preferred image size is replaced with the dimensions from
        // the file-transport control record as soon as the producer mapping is opened.
        this._imageContent = St.ImageContent.new_with_preferred_size(640, 360);

        this._lockActor = new St.Widget({
            reactive: false,
            can_focus: false,
            content: this._imageContent,
        });

        this._lockActor.set_position(0, 0);
        this._lockActor.set_size(dialog.width, dialog.height);

        backgroundGroup.add_child(this._lockActor);

        console.log('[Screenshaver] File-transport lock actor added above GNOME lock background');

        // GNOME retains its native lock/authentication UI above this actor.
        // For this diagnostic build, PowerSave observation is strictly read-only:
        // no D-Bus Set(PowerSaveMode) calls are issued.  Capture the lock actor,
        // ScreenShield/dialog, focus, and observed power state before and after
        // the first real keyboard/mouse activity.
        this._startPowerSaveRecovery();
        this._startStartupDiagnostic();
        this._refreshFrame();

        this._pollSource = GLib.timeout_add(
            GLib.PRIORITY_DEFAULT,
            POLL_INTERVAL_MS,
            () => {
                if (!this._lockActor)
                    return GLib.SOURCE_REMOVE;

                this._refreshFrame();
                return GLib.SOURCE_CONTINUE;
            }
        );
    }

    _refreshFrame() {
        if (!this._imageContent || !this._lockActor)
            return;

        const control = this._readControlRecord(true);
        if (!control)
            return;

        if (!this._activeSessionId || control.sessionId !== this._activeSessionId) {
            if (!this._transportErrorLogged) {
                console.log('[Screenshaver] File transport session identity mismatch');
                this._transportErrorLogged = true;
            }
            return;
        }

        this._transportErrorLogged = false;
        this._refreshCalls++;

        if (control.frameCounter === 0 ||
            control.frameCounter === this._lastFrameCounter) {
            return;
        }

        if (this._frameReadInFlight) {
            this._pendingFrameCounter = control.frameCounter;
            return;
        }

        this._startFrameRead(control);
    }

    _startFrameRead(control) {
        if (!this._lockActor || this._frameReadInFlight)
            return;

        const framePath = GLib.build_filenamev([
            GLib.get_user_runtime_dir(),
            `${FRAME_FILENAME_PREFIX}${control.frameCounter.toString().padStart(10, '0')}${FRAME_FILENAME_SUFFIX}`,
        ]);

        const file = Gio.File.new_for_path(framePath);
        const generation = this._transportGeneration;
        const cancellable = new Gio.Cancellable();
        this._frameReadCancellable = cancellable;
        this._frameReadInFlight = true;
        this._uploadAttempts++;

        file.load_bytes_async(cancellable, (source, result) => {
            if (generation !== this._transportGeneration)
                return;

            this._frameReadInFlight = false;
            this._frameReadCancellable = null;

            try {
                const [bytes] = source.load_bytes_finish(result);
                const data = bytes.get_data();

                if (!this._lockActor || !this._imageContent)
                    return;

                if (!data || data.length !== control.frameBytes) {
                    console.log(
                        `[Screenshaver] GNOME frame ${control.frameCounter} has ` +
                        `${data?.length ?? 0} bytes; expected ${control.frameBytes}`
                    );
                } else {
                    const coglContext = global.stage.context
                        .get_backend()
                        .get_cogl_context();

                    this._imageContent.set_bytes(
                        coglContext,
                        bytes,
                        Cogl.PixelFormat.RGBA_8888,
                        control.width,
                        control.height,
                        control.rowstride
                    );

                    this._lockActor.queue_redraw();
                    this._lastFrameCounter = control.frameCounter;
                    this._displayedFrames++;
                    this._uploadSuccesses++;

                    if (this._displayedFrames === 1) {
                        console.log(
                            `[Screenshaver] First file-transport frame displayed: ` +
                            `counter=${control.frameCounter}`
                        );

                        // Diagnostic only: record the exact GNOME/actor state
                        // when the first real shader frame becomes available.
                        // No display-power mutation occurs in this build.
                        this._logStartupDiagnosticState('first-frame', true);
                    } else if (this._displayedFrames % 300 === 0) {
                        console.log(
                            `[Screenshaver] File-transport frames displayed: ${this._displayedFrames}`
                        );
                    }
                }
            } catch (error) {
                if (!cancellable.is_cancelled()) {
                    console.log(
                        `[Screenshaver] Unable to asynchronously read GNOME frame ` +
                        `${control.frameCounter}: ${error}`
                    );
                }
            }

            if (!this._lockActor || generation !== this._transportGeneration)
                return;

            // Backpressure: while one immutable frame was being read, remember
            // only the newest counter. Intermediate frames are intentionally
            // dropped so GNOME Shell never accumulates asynchronous reads.
            const pending = this._pendingFrameCounter;
            this._pendingFrameCounter = 0;

            if (pending !== 0 && pending !== this._lastFrameCounter) {
                const latest = this._readControlRecord(false);
                if (latest && latest.frameCounter !== this._lastFrameCounter)
                    this._startFrameRead(latest);
            }
        });
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

    _startStartupDiagnostic() {
        if (!this._lockActor || this._startupDiagnosticSource)
            return;

        this._startupDiagnosticDeadlineUs =
            GLib.get_monotonic_time() + STARTUP_DIAGNOSTIC_WINDOW_MS * 1000;
        this._startupDiagnosticLastState = '';
        this._startupDiagnosticLastLogUs = 0;

        console.log('[Screenshaver] Startup-state diagnostic window opened (read-only)');
        this._logStartupDiagnosticState('start', true);

        this._startupDiagnosticSource = GLib.timeout_add(
            GLib.PRIORITY_DEFAULT,
            STARTUP_DIAGNOSTIC_INTERVAL_MS,
            () => {
                if (!this._lockActor ||
                    GLib.get_monotonic_time() >= this._startupDiagnosticDeadlineUs) {
                    this._startupDiagnosticSource = null;
                    console.log('[Screenshaver] Startup-state diagnostic window closed');
                    return GLib.SOURCE_REMOVE;
                }

                this._logStartupDiagnosticState('sample', false);
                return GLib.SOURCE_CONTINUE;
            }
        );
    }

    _stopStartupDiagnostic() {
        if (this._startupDiagnosticSource) {
            GLib.source_remove(this._startupDiagnosticSource);
            this._startupDiagnosticSource = null;
        }

        this._startupDiagnosticDeadlineUs = 0;
        this._startupDiagnosticLastState = '';
        this._startupDiagnosticLastLogUs = 0;
    }

    _logStartupDiagnosticState(reason, force = false) {
        if (!this._lockActor)
            return;

        try {
            const actor = this._lockActor;
            const parent = actor.get_parent();
            const dialog = Main.screenShield?._dialog ?? null;
            const shield = Main.screenShield ?? null;
            const focus = global.stage.get_key_focus?.() ?? null;
            const siblings = parent?.get_children?.() ?? [];
            const siblingIndex = siblings.indexOf(actor);

            const transitionNames = [
                'opacity', 'x', 'y', 'width', 'height',
                'translation-x', 'translation-y', 'scale-x', 'scale-y',
            ];
            const actorTransitions = transitionNames.filter(
                name => actor.get_transition(name) !== null
            );
            const dialogTransitions = dialog
                ? transitionNames.filter(name => dialog.get_transition(name) !== null)
                : [];

            let focusDescription = 'none';
            if (focus) {
                const focusName = focus.get_name?.() ?? '';
                const focusType = focus.constructor?.name ?? 'actor';
                focusDescription = `${focusType}${focusName ? ':' + focusName : ''}`;
            }

            const state = [
                `mode=${Main.sessionMode.currentMode}`,
                `power=${this._lastObservedPowerSaveMode ?? 'unknown'}`,
                `frame=${this._lastFrameCounter}`,
                `displayed=${this._displayedFrames}`,
                `actorVisible=${actor.visible}`,
                `actorMapped=${actor.mapped}`,
                `actorRealized=${actor.realized}`,
                `actorPaintVisible=${actor.get_paint_visibility()}`,
                `actorOpacity=${actor.opacity}`,
                `actorPaintOpacity=${actor.get_paint_opacity()}`,
                `actorSibling=${siblingIndex}/${siblings.length}`,
                `actorTransitions=${actorTransitions.length ? actorTransitions.join(',') : 'none'}`,
                `parentVisible=${parent?.visible ?? 'none'}`,
                `parentMapped=${parent?.mapped ?? 'none'}`,
                `parentOpacity=${parent?.opacity ?? 'none'}`,
                `dialogVisible=${dialog?.visible ?? 'none'}`,
                `dialogMapped=${dialog?.mapped ?? 'none'}`,
                `dialogOpacity=${dialog?.opacity ?? 'none'}`,
                `dialogTransitions=${dialogTransitions.length ? dialogTransitions.join(',') : 'none'}`,
                `shieldActive=${shield?._isActive ?? 'unknown'}`,
                `shieldLocked=${shield?._isLocked ?? 'unknown'}`,
                `focus=${focusDescription}`,
            ].join(' ');

            const nowUs = GLib.get_monotonic_time();
            const changed = state !== this._startupDiagnosticLastState;
            const heartbeat = nowUs - this._startupDiagnosticLastLogUs >= 2000000;

            if (force || changed || heartbeat) {
                console.log(`[Screenshaver] Startup state (${reason}): ${state}`);
                this._startupDiagnosticLastState = state;
                this._startupDiagnosticLastLogUs = nowUs;
            }
        } catch (error) {
            console.log(`[Screenshaver] Startup-state diagnostic failed: ${error}`);
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
                    if (!this._lockActor)
                        return GLib.SOURCE_REMOVE;

                    this._samplePowerSaveModeFallback();
                    return GLib.SOURCE_CONTINUE;
                }
            );
        }

        this._samplePowerSaveModeFallback();
    }

    _startPowerSaveStartupRecovery() {
        if (!this._lockActor || this._displayedFrames === 0)
            return;

        if (this._powerSaveStartupSource)
            return;

        this._powerSaveStartupResetAttempts = 0;
        this._powerSaveStartupDeadlineUs =
            GLib.get_monotonic_time() + POWER_SAVE_STARTUP_WINDOW_MS * 1000;

        console.log(
            '[Screenshaver] GNOME startup PowerSave recovery window opened'
        );

        // Sample once immediately now that the actor contains a real frame.
        this._samplePowerSaveModeFallback();

        this._powerSaveStartupSource = GLib.timeout_add(
            GLib.PRIORITY_DEFAULT,
            POWER_SAVE_STARTUP_SAMPLE_INTERVAL_MS,
            () => {
                if (!this._lockActor ||
                    GLib.get_monotonic_time() >= this._powerSaveStartupDeadlineUs ||
                    this._powerSaveStartupResetAttempts >=
                        POWER_SAVE_STARTUP_MAX_RESET_ATTEMPTS) {
                    this._powerSaveStartupSource = null;
                    console.log(
                        `[Screenshaver] GNOME startup PowerSave recovery window closed; ` +
                        `reset-attempts=${this._powerSaveStartupResetAttempts}`
                    );
                    return GLib.SOURCE_REMOVE;
                }

                this._samplePowerSaveModeFallback();
                return GLib.SOURCE_CONTINUE;
            }
        );
    }

    _stopPowerSaveStartupRecovery() {
        if (this._powerSaveStartupSource) {
            GLib.source_remove(this._powerSaveStartupSource);
            this._powerSaveStartupSource = null;
        }

        this._powerSaveStartupDeadlineUs = 0;
        this._powerSaveStartupResetAttempts = 0;
    }

    _stopPowerSaveRecovery() {
        this._stopPowerSaveStartupRecovery();
        this._unsubscribePowerSaveModeChanges();

        if (this._powerSaveFallbackSource) {
            GLib.source_remove(this._powerSaveFallbackSource);
            this._powerSaveFallbackSource = null;
        }

        this._powerSaveFallbackQueryInFlight = false;
        this._lastObservedPowerSaveMode = null;
        this._powerSaveResetInFlight = false;
        this._powerWakeCycleCount = 0;
        this._pendingPowerWakeCycle = 0;
        this._lastPowerWakeAttemptUs = 0;
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
            this._logStartupDiagnosticState('power-change', true);
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

    _maybePowerSaveThenWake(_startupRecovery = false) {
        // Intentionally disabled in this diagnostic build.  We are observing
        // what genuine user activity changes, not attempting to reproduce it.
        return;
    }

    _verifyPowerSaveThenWake(cycle) {
        if (!this._lockActor || Main.sessionMode.currentMode !== 'unlock-dialog')
            return;

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
                    try {
                        const reply = connection.call_finish(result);
                        const unpacked = reply.deep_unpack();
                        let value = unpacked[0];

                        if (value && typeof value.deep_unpack === 'function')
                            value = value.deep_unpack();

                        this._lastObservedPowerSaveMode = value;

                        if (value === 0 &&
                            Main.sessionMode.currentMode === 'unlock-dialog' &&
                            this._pendingPowerWakeCycle === cycle) {
                            this._pendingPowerWakeCycle = 0;
                        }
                    } catch (error) {
                        console.log(
                            `[Screenshaver] Mutter PowerSaveMode recovery verification failed: ${error}`
                        );
                    }
                }
            );
        } catch (error) {
            console.log(
                `[Screenshaver] Unable to dispatch Mutter PowerSaveMode recovery verification: ${error}`
            );
        }
    }

    _removeLockActor() {
        this._stopSessionValidation();
        this._stopStartupDiagnostic();
        this._stopPowerSaveRecovery();

        if (this._pollSource) {
            GLib.source_remove(this._pollSource);
            this._pollSource = null;
        }

        if (this._lockActor) {
            console.log('[Screenshaver] Removing file-transport lock actor');
            this._lockActor.destroy();
            this._lockActor = null;
        }

        this._imageContent = null;
        this._displayedFrames = 0;
        this._refreshCalls = 0;
        this._uploadAttempts = 0;
        this._uploadSuccesses = 0;
        this._transportErrorLogged = false;
        this._lastObservedPowerSaveMode = null;
        this._powerSaveResetInFlight = false;
        this._powerWakeCycleCount = 0;
        this._pendingPowerWakeCycle = 0;
        this._lastPowerWakeAttemptUs = 0;
        this._powerSaveFallbackQueryInFlight = false;
        this._activeSessionId = null;
        this._transportGeneration++;
        this._pendingFrameCounter = 0;
        this._frameReadInFlight = false;
        if (this._frameReadCancellable) {
            this._frameReadCancellable.cancel();
            this._frameReadCancellable = null;
        }
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
