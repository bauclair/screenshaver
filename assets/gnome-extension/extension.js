import Cogl from 'gi://Cogl';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import Gio from 'gi://Gio';
import St from 'gi://St';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const TRANSPORT_FILENAME = 'screenshaver-lock-frame.shm';
const TRANSPORT_MAGIC = [0x53, 0x48, 0x56, 0x52, 0x47, 0x4e, 0x4d, 0x31]; // SHVRGNM1
const TRANSPORT_VERSION = 2;
const TRANSPORT_HEADER_BYTES = 64;
const TRANSPORT_SLOT_COUNT = 2;
const POLL_INTERVAL_MS = 8;
const POWER_SAVE_FALLBACK_INTERVAL_MS = 1000;
const RUNTIME_MARKER_FILENAME = 'screenshaver-gnome-lock.active';
const RUNTIME_MARKER_VERSION = 1;
const SESSION_ID_BYTES = 16;
const SESSION_VALIDATION_INTERVAL_MS = 1000;

// Screenshaver authentication-widget geometry, mirrored from
// define_lock_screen_widget.rs. This GNOME implementation remains
// presentation-only: GNOME Shell retains all input, PAM, retry, and unlock
// authority.
const AUTH_BACKGROUND_RADIUS = 180;
const AUTH_PARENT_RADIUS = 130;
const AUTH_CHILD_RADIUS = 24;
const AUTH_CHILD_COUNT = 12;
const AUTH_HALO_DISTANCE = AUTH_BACKGROUND_RADIUS * 0.08;

// Use a 390px container so the 194.4px production halo radius fits cleanly.
// The sub-pixel difference is limited to 0.6px at the outer edge.
const AUTH_WIDGET_DIAMETER = 390;
const AUTH_WIDGET_CENTER = AUTH_WIDGET_DIAMETER / 2;

const AUTH_BACKGROUND_COLOR = '#050505';
const AUTH_CHILD_INACTIVE_COLOR = '#050505';
const AUTH_CHILD_ACTIVE_COLOR = '#FFA500';
const AUTH_CHILD_ERROR_COLOR = '#F21A1A';
const AUTH_CHILD_ACTIVE_FADE_TIME_MS = 300;
const AUTH_CHILD_FADE_STEP_MS = 50;
const AUTHENTICATION_FAILURE_DURATION_MS = 2000;

// Diagnostic-only GNOME authentication observation.
// No keyboard events are connected and no entry text is read or logged.
const AUTH_DIAGNOSTIC_POLL_INTERVAL_MS = 250;
const AUTH_DIAGNOSTIC_FILENAME = 'screenshaver-gnome-auth-observation.log';
const AUTH_DIAGNOSTIC_SAFE_SIGNALS = [
    'verification-failed',
    'verification-complete',
    'reset',
    'cancelled',
    'failed',
    'show-message',
    'show-prompt',

    // Edit-operation experiment. We observe only that an operation occurred;
    // callback arguments are deliberately ignored so no credential content is
    // read, copied, counted, or logged.
    'insert-text',
    'delete-text',
    'text-changed',
    'changed',
];

const HEADER_MAGIC_OFFSET = 0;
const HEADER_VERSION_OFFSET = 8;
const HEADER_SIZE_OFFSET = 12;
const HEADER_WIDTH_OFFSET = 16;
const HEADER_HEIGHT_OFFSET = 20;
const HEADER_ROWSTRIDE_OFFSET = 24;
const HEADER_FRAME_BYTES_OFFSET = 28;
const HEADER_SLOT_COUNT_OFFSET = 32;
const HEADER_ACTIVE_SLOT_OFFSET = 36;
const HEADER_FRAME_COUNTER_OFFSET = 40;
const HEADER_SESSION_ID_OFFSET = 44;

export default class ScreenshaverExtension extends Extension {
    enable() {
        console.log('[Screenshaver] GNOME Shell extension enabled');

        this._lockActor = null;
        this._imageContent = null;
        this._pollSource = null;
        this._mappedFile = null;
        this._lastFrameCounter = 0;
        this._displayedFrames = 0;
        this._transportErrorLogged = false;
        this._sessionValidationSource = null;
        this._sessionWaitSource = null;
        this._activeSessionId = null;
        this._wakeRequestCount = 0;
        this._lastObservedPowerSaveMode = null;
        this._powerSaveResetInFlight = false;
        this._powerWakeCycleCount = 0;
        this._pendingPowerWakeCycle = 0;
        this._lastPowerWakeAttemptUs = 0;
        this._powerSaveSignalId = 0;
        this._powerSaveFallbackSource = null;
        this._powerSaveFallbackQueryInFlight = false;
        this._authWidgetActor = null;
        this._authWidgetWidthSignal = 0;
        this._authWidgetHeightSignal = 0;
        this._authWidgetDialog = null;
        this._authWidgetChildren = [];
        this._authWidgetFadeSources = [];
        this._authWidgetNextChild = 0;
        this._authWidgetEntryObject = null;
        this._authWidgetEntryConnections = [];
        this._authWidgetFailureSource = null;
        this._authWidgetVerificationInProgress = false;
        this._authWidgetFailureLatched = false;
        this._authWidgetSuppressedActors = [];

        this._authDiagnosticConnections = [];
        this._authDiagnosticPollSource = null;
        this._authDiagnosticPath = null;
        this._authDiagnosticLastState = new Map();
        this._authDiagnosticObservedObjects = new Set();

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

        const framePath = GLib.build_filenamev([
            GLib.get_user_runtime_dir(),
            TRANSPORT_FILENAME,
        ]);

        let mappedFile = null;

        try {
            mappedFile = GLib.MappedFile.new(framePath, false);
            const data = mappedFile.get_bytes().get_data();

            if (!this._validateHeader(data, false))
                return null;

            const transportSessionId =
                readSessionIdHex(data, HEADER_SESSION_ID_OFFSET);

            if (transportSessionId !== marker.sessionId)
                return null;

            return marker;
        } catch (_) {
            return null;
        } finally {
            mappedFile = null;
        }
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
        // the shared-memory header as soon as the producer mapping is opened.
        this._imageContent = St.ImageContent.new_with_preferred_size(640, 360);

        this._lockActor = new St.Widget({
            reactive: false,
            can_focus: false,
            content: this._imageContent,
        });

        this._lockActor.set_position(0, 0);
        this._lockActor.set_size(dialog.width, dialog.height);

        backgroundGroup.add_child(this._lockActor);

        console.log('[Screenshaver] Shared-memory lock actor added above GNOME lock background');

        // Add the static Screenshaver authentication-widget geometry after
        // the shader actor. It remains inside GNOME's background group, so
        // GNOME's secure authentication controls stay above it and retain all
        // input/authentication authority.
        this._createAuthWidget(dialog, backgroundGroup);

        // Required presentation invariant for Screenshaver on GNOME:
        // no GNOME clock/date, user/avatar, or password-entry chrome is ever
        // visually presented while Screenshaver owns the lock background.
        // This is opacity-only suppression; GNOME retains mapping, focus,
        // keyboard input, PAM, retries, and unlock authority.
        try {
            this._authWidgetRefreshGnomeAuthChromeSuppression(dialog);
        } catch (error) {
            console.log(
                `[Screenshaver] Initial GNOME legacy-chrome suppression skipped: ${error}`
            );
        }

        // Establish the proven rendering and PowerSave paths first. Diagnostic
        // observation must never be able to prevent shader presentation.
        this._startPowerSaveRecovery();
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

        // Authentication observation is strictly optional. Any incompatibility
        // with GNOME Shell internals is contained here and cannot abort the
        // already-established shader/PowerSave presentation path.
        try {
            this._startAuthDiagnostics(dialog);
        } catch (error) {
            console.log(
                `[Screenshaver][AuthDiag] Observation disabled after startup error: ${error}`
            );
        }
    }


    _createAuthWidget(dialog, backgroundGroup) {
        if (this._authWidgetActor)
            return;

        this._authWidgetActor = new St.Widget({
            reactive: false,
            can_focus: false,
        });
        this._authWidgetActor.opacity = 0;
        this._authWidgetActor.set_size(
            AUTH_WIDGET_DIAMETER,
            AUTH_WIDGET_DIAMETER
        );

        // Approximate the OpenGL halo's 14.4px radial fade with three
        // concentric, non-interactive rings. The opaque background circle is
        // added afterward and covers their centers, leaving only the halo.
        const haloLayers = [
            {radius: 195, alpha: 0.18},
            {radius: 190, alpha: 0.35},
            {radius: 185, alpha: 0.65},
        ];

        for (const layer of haloLayers) {
            const diameter = layer.radius * 2;
            const halo = new St.Widget({
                reactive: false,
                can_focus: false,
                style:
                    `background-color: rgba(255, 255, 0, ${layer.alpha}); ` +
                    `border-radius: ${layer.radius}px;`,
            });

            halo.set_size(diameter, diameter);
            halo.set_position(
                Math.round(AUTH_WIDGET_CENTER - layer.radius),
                Math.round(AUTH_WIDGET_CENTER - layer.radius)
            );
            this._authWidgetActor.add_child(halo);
        }

        const backgroundDiameter = AUTH_BACKGROUND_RADIUS * 2;
        const background = new St.Widget({
            reactive: false,
            can_focus: false,
            style:
                `background-color: ${AUTH_BACKGROUND_COLOR}; ` +
                `border-radius: ${AUTH_BACKGROUND_RADIUS}px;`,
        });

        background.set_size(backgroundDiameter, backgroundDiameter);
        background.set_position(
            AUTH_WIDGET_CENTER - AUTH_BACKGROUND_RADIUS,
            AUTH_WIDGET_CENTER - AUTH_BACKGROUND_RADIUS
        );
        this._authWidgetActor.add_child(background);

        // Twelve child circles, 24px radius, placed every 30 degrees around
        // the 130px parent radius. Child 0 begins at 12 o'clock and subsequent
        // children advance clockwise, exactly matching lock_screen_widget.rs.
        for (let childIndex = 0; childIndex < AUTH_CHILD_COUNT; childIndex++) {
            const angle =
                (childIndex * 30.0 - 90.0) * Math.PI / 180.0;

            const childCenterX =
                AUTH_WIDGET_CENTER + AUTH_PARENT_RADIUS * Math.cos(angle);
            const childCenterY =
                AUTH_WIDGET_CENTER + AUTH_PARENT_RADIUS * Math.sin(angle);

            const child = new St.Widget({
                reactive: false,
                can_focus: false,
                style:
                    `background-color: ${AUTH_CHILD_INACTIVE_COLOR}; ` +
                    `border-radius: ${AUTH_CHILD_RADIUS}px;`,
            });

            const childDiameter = AUTH_CHILD_RADIUS * 2;
            child.set_size(childDiameter, childDiameter);
            child.set_position(
                Math.round(childCenterX - AUTH_CHILD_RADIUS),
                Math.round(childCenterY - AUTH_CHILD_RADIUS)
            );
            this._authWidgetActor.add_child(child);
            this._authWidgetChildren.push(child);
            this._authWidgetFadeSources.push(null);
        }

        // The widget is added after the shader:
        // stock GNOME background -> shader -> Screenshaver auth widget
        // -> GNOME secure authentication UI.
        backgroundGroup.add_child(this._authWidgetActor);

        this._authWidgetDialog = dialog;
        this._positionAuthWidget();

        this._authWidgetWidthSignal = dialog.connect(
            'notify::width',
            () => this._positionAuthWidget()
        );
        this._authWidgetHeightSignal = dialog.connect(
            'notify::height',
            () => this._positionAuthWidget()
        );

        console.log(
            `[Screenshaver] GNOME auth widget added: ` +
            `background-radius=${AUTH_BACKGROUND_RADIUS}px, ` +
            `parent-radius=${AUTH_PARENT_RADIUS}px, ` +
            `child-radius=${AUTH_CHILD_RADIUS}px, presentation-only`
        );
    }

    _authWidgetSetChildColor(childIndex, color) {
        const child = this._authWidgetChildren?.[childIndex];

        if (!child)
            return;

        child.set_style(
            `background-color: ${color}; ` +
            `border-radius: ${AUTH_CHILD_RADIUS}px;`
        );
    }

    _authWidgetCancelFade(childIndex) {
        const sourceId =
            this._authWidgetFadeSources?.[childIndex];

        if (sourceId) {
            try {
                GLib.source_remove(sourceId);
            } catch (_) {
                // Best-effort animation cleanup.
            }

            this._authWidgetFadeSources[childIndex] = null;
        }
    }

    _authWidgetResetChildren() {
        for (
            let childIndex = 0;
            childIndex < AUTH_CHILD_COUNT;
            childIndex++
        ) {
            this._authWidgetCancelFade(childIndex);
            this._authWidgetSetChildColor(
                childIndex,
                AUTH_CHILD_INACTIVE_COLOR
            );
        }
    }

    _authWidgetResetSequence() {
        this._authWidgetNextChild = 0;
        this._authWidgetResetChildren();
    }

    _authWidgetBlendHex(
        fromColor,
        toColor,
        fraction
    ) {
        const from = [
            Number.parseInt(fromColor.slice(1, 3), 16),
            Number.parseInt(fromColor.slice(3, 5), 16),
            Number.parseInt(fromColor.slice(5, 7), 16),
        ];
        const to = [
            Number.parseInt(toColor.slice(1, 3), 16),
            Number.parseInt(toColor.slice(3, 5), 16),
            Number.parseInt(toColor.slice(5, 7), 16),
        ];

        const channel = index =>
            Math.round(
                from[index]
                + (to[index] - from[index]) * fraction
            );

        return (
            '#'
            + [0, 1, 2]
                .map(index =>
                    channel(index)
                        .toString(16)
                        .padStart(2, '0')
                )
                .join('')
        );
    }

    _authWidgetAnimateChild(childIndex) {
        if (
            childIndex < 0
            || childIndex >= AUTH_CHILD_COUNT
        ) {
            return;
        }

        this._authWidgetCancelFade(childIndex);
        this._authWidgetSetChildColor(
            childIndex,
            AUTH_CHILD_ACTIVE_COLOR
        );

        const totalSteps =
            Math.max(
                1,
                Math.round(
                    AUTH_CHILD_ACTIVE_FADE_TIME_MS
                    / AUTH_CHILD_FADE_STEP_MS
                )
            );

        let step = 0;

        const sourceId =
            GLib.timeout_add(
                GLib.PRIORITY_DEFAULT,
                AUTH_CHILD_FADE_STEP_MS,
                () => {
                    if (!this._authWidgetChildren?.[childIndex]) {
                        this._authWidgetFadeSources[childIndex] = null;
                        return GLib.SOURCE_REMOVE;
                    }

                    step++;
                    const fraction =
                        Math.min(1.0, step / totalSteps);

                    this._authWidgetSetChildColor(
                        childIndex,
                        this._authWidgetBlendHex(
                            AUTH_CHILD_ACTIVE_COLOR,
                            AUTH_CHILD_INACTIVE_COLOR,
                            fraction
                        )
                    );

                    if (fraction >= 1.0) {
                        this._authWidgetFadeSources[childIndex] = null;
                        return GLib.SOURCE_REMOVE;
                    }

                    return GLib.SOURCE_CONTINUE;
                }
            );

        this._authWidgetFadeSources[childIndex] = sourceId;
    }

    _authWidgetHandleInsert() {
        const childIndex =
            this._authWidgetNextChild;

        this._authWidgetAnimateChild(childIndex);

        this._authWidgetNextChild =
            (childIndex + 1) % AUTH_CHILD_COUNT;
    }

    _authWidgetHandleDelete() {
        this._authWidgetNextChild =
            (
                this._authWidgetNextChild
                - 1
                + AUTH_CHILD_COUNT
            ) % AUTH_CHILD_COUNT;

        // Match Screenshaver's established Backspace behavior: rewind the
        // sequence and clear any outstanding active/fade presentation.
        this._authWidgetResetChildren();
    }

    _authWidgetUnbindEntryOperations() {
        for (
            const [object, handlerId]
            of this._authWidgetEntryConnections || []
        ) {
            try {
                object.disconnect(handlerId);
            } catch (_) {
                // Best-effort teardown.
            }
        }

        this._authWidgetEntryConnections = [];
        this._authWidgetEntryObject = null;
    }

    _authWidgetBindEntryOperations(label, object) {
        if (
            label !== 'dialog._authPrompt._entry.ClutterText'
            || !object?.connect
            || this._authWidgetEntryObject === object
        ) {
            return;
        }

        this._authWidgetUnbindEntryOperations();
        this._authWidgetEntryObject = object;

        try {
            const insertId =
                object.connect(
                    'insert-text',
                    () => {
                        // Deliberately ignore all signal arguments. The widget
                        // responds only to the fact that an insertion occurred.
                        this._authWidgetHandleInsert();
                    }
                );

            this._authWidgetEntryConnections.push(
                [object, insertId]
            );
        } catch (error) {
            console.log(
                `[Screenshaver] Unable to bind auth insert animation: ${error}`
            );
        }

        try {
            const deleteId =
                object.connect(
                    'delete-text',
                    () => {
                        // Deliberately ignore all signal arguments. No deleted
                        // character, cursor position, or password length is read.
                        this._authWidgetHandleDelete();
                    }
                );

            this._authWidgetEntryConnections.push(
                [object, deleteId]
            );
        } catch (error) {
            console.log(
                `[Screenshaver] Unable to bind auth delete animation: ${error}`
            );
        }

        console.log(
            '[Screenshaver] GNOME auth widget bound to sanitized insert/delete operations'
        );
    }


    _authWidgetCancelFailureDisplay() {
        if (this._authWidgetFailureSource) {
            try {
                GLib.source_remove(
                    this._authWidgetFailureSource
                );
            } catch (_) {
                // Best-effort failure-animation cleanup.
            }

            this._authWidgetFailureSource = null;
        }
    }

    _authWidgetShowFailure() {
        this._authWidgetCancelFailureDisplay();

        for (
            let childIndex = 0;
            childIndex < AUTH_CHILD_COUNT;
            childIndex++
        ) {
            this._authWidgetCancelFade(childIndex);
            this._authWidgetSetChildColor(
                childIndex,
                AUTH_CHILD_ERROR_COLOR
            );
        }

        this._authWidgetNextChild = 0;

        this._diagLog(
            'AUTH-WIDGET: candidate authentication failure -> all children red'
        );

        this._authWidgetFailureSource =
            GLib.timeout_add(
                GLib.PRIORITY_DEFAULT,
                AUTHENTICATION_FAILURE_DURATION_MS,
                () => {
                    this._authWidgetFailureSource = null;
                    this._authWidgetResetSequence();

                    this._diagLog(
                        'AUTH-WIDGET: failure display expired -> widget reset'
                    );

                    return GLib.SOURCE_REMOVE;
                }
            );
    }

    _authWidgetSetVisible(visible, reason) {
        if (!this._authWidgetActor)
            return;

        try {
            const desiredOpacity = visible ? 255 : 0;

            if (Number(this._authWidgetActor.opacity ?? 255) !== desiredOpacity)
                this._authWidgetActor.opacity = desiredOpacity;

            if (reason)
                this._diagLog(`AUTH-WIDGET: ${reason}`);
        } catch (error) {
            console.log(
                `[Screenshaver] Auth-circle visibility update skipped: ${error}`
            );
        }

        if (!visible) {
            this._authWidgetCancelFailureDisplay();
            this._authWidgetVerificationInProgress = false;
            this._authWidgetFailureLatched = false;
            this._authWidgetResetSequence();
        }
    }

    _authWidgetSyncVisibilityToGnomePrompt(dialog) {
        // GNOME 50's UnlockDialog has two presentation pages. The diagnostic
        // trace established that AuthPrompt.mapped is false on the clock page,
        // true on the authentication page, and false again when GNOME returns
        // to the clock page. Mirror that GNOME-owned state directly.
        const authPrompt = dialog?._authPrompt ?? null;

        let promptMapped = false;

        try {
            promptMapped = authPrompt?.mapped === true;
        } catch (_) {
            promptMapped = false;
        }

        this._authWidgetSetVisible(
            promptMapped,
            null
        );
    }


    _authWidgetCheckFailureState(dialog) {
        const authPrompt =
            dialog?._authPrompt;
        const entry =
            authPrompt?._entry;
        const spinner =
            authPrompt?._spinner;
        const message =
            authPrompt?._message;

        if (!authPrompt || !entry || !spinner || !message) {
            this._authWidgetVerificationInProgress = false;
            this._authWidgetFailureLatched = false;
            return;
        }

        let entryReactive = false;
        let spinnerOpacity = 0;
        let messageOpacity = 0;
        let promptMapped = false;

        try {
            entryReactive =
                entry.reactive === true;
        } catch (_) {
            return;
        }

        try {
            spinnerOpacity =
                Number(spinner.opacity ?? 0);
        } catch (_) {
            spinnerOpacity = 0;
        }

        try {
            messageOpacity =
                Number(message.opacity ?? 0);
        } catch (_) {
            messageOpacity = 0;
        }

        try {
            promptMapped =
                authPrompt.mapped === true;
        } catch (_) {
            promptMapped = false;
        }

        // Submission/verification is visible without inspecting any credential:
        // GNOME disables the password entry and presents its spinner.
        if (
            promptMapped
            && !entryReactive
            && spinnerOpacity > 0
        ) {
            if (!this._authWidgetVerificationInProgress) {
                this._diagLog(
                    'AUTH-WIDGET: candidate verification-in-progress detected'
                );
            }

            this._authWidgetVerificationInProgress = true;
            this._authWidgetFailureLatched = false;
            return;
        }

        // Candidate failure signature observed in GNOME 50:
        //   verification had been in progress,
        //   spinner is gone,
        //   entry is enabled again,
        //   GNOME displays a message.
        //
        // We intentionally do NOT read the message text.
        if (
            this._authWidgetVerificationInProgress
            && promptMapped
            && entryReactive
            && spinnerOpacity === 0
            && messageOpacity > 0
            && !this._authWidgetFailureLatched
        ) {
            this._authWidgetFailureLatched = true;
            this._authWidgetVerificationInProgress = false;

            this._diagLog(
                'AUTH-WIDGET: candidate failed-verification transition detected'
            );

            this._authWidgetShowFailure();
            return;
        }

        // If GNOME returned to an ordinary editable prompt without showing a
        // message, clear the candidate-verification latch. This prevents an
        // unrelated later message from being interpreted as a failed attempt.
        if (
            this._authWidgetVerificationInProgress
            && promptMapped
            && entryReactive
            && spinnerOpacity === 0
            && messageOpacity === 0
        ) {
            this._diagLog(
                'AUTH-WIDGET: verification candidate ended without failure signature'
            );

            this._authWidgetVerificationInProgress = false;
            this._authWidgetFailureLatched = false;
        }

        if (!promptMapped) {
            this._authWidgetVerificationInProgress = false;
            this._authWidgetFailureLatched = false;
        }
    }


    _authWidgetSuppressGnomeAuthChrome(label, object) {
        if (!object)
            return;

        const suppressLabels = new Set([
            'dialog._clock',
            'dialog._authPrompt._entry',
            'dialog._authPrompt._userWell',
            'dialog._userName',
        ]);

        if (!suppressLabels.has(label))
            return;

        // Presentation-only suppression. These GNOME-owned actors remain
        // mapped/reactive/focusable exactly as GNOME created them; only their
        // rendered opacity is forced to zero.
        //
        // This includes:
        //   - GNOME's lock-screen clock/date presentation,
        //   - the legacy password-entry box,
        //   - GNOME's user-well/avatar presentation,
        //   - GNOME's user-name label when exposed separately.
        //
        // Do NOT hide(), unmap, disable, remove, or reparent these actors.
        let record =
            this._authWidgetSuppressedActors.find(
                item => item.object === object
            );

        if (!record) {
            let previousOpacity = 255;

            try {
                previousOpacity =
                    Number(object.opacity ?? 255);
            } catch (_) {
                previousOpacity = 255;
            }

            record = {
                object,
                label,
                previousOpacity,
            };

            this._authWidgetSuppressedActors.push(record);
        }

        try {
            if (Number(object.opacity ?? 255) !== 0)
                object.opacity = 0;
        } catch (error) {
            console.log(
                `[Screenshaver] Unable to suppress GNOME auth chrome '${label}': ${error}`
            );
            return;
        }

        if (!record.logged) {
            record.logged = true;
            this._diagLog(
                `AUTH-WIDGET: GNOME auth chrome opacity suppressed for ${label}`
            );
        }
    }

    _authWidgetRefreshGnomeAuthChromeSuppression(dialog) {
        const authPrompt =
            dialog?._authPrompt;

        const candidates = [
            ['dialog._clock', dialog?._clock],
            ['dialog._userName', dialog?._userName],
            ['dialog._authPrompt._userWell', authPrompt?._userWell],
            ['dialog._authPrompt._entry', authPrompt?._entry],
        ];

        for (const [label, object] of candidates) {
            try {
                this._authWidgetSuppressGnomeAuthChrome(
                    label,
                    object
                );
            } catch (_) {
                // Best-effort visual suppression only.
            }
        }
    }

    _authWidgetRestoreGnomeAuthChrome() {
        for (
            const record of this._authWidgetSuppressedActors || []
        ) {
            try {
                if (record?.object)
                    record.object.opacity = record.previousOpacity;
            } catch (_) {
                // Best-effort restore during extension teardown.
            }
        }

        this._authWidgetSuppressedActors = [];
    }


    _positionAuthWidget() {
        if (!this._authWidgetActor || !this._authWidgetDialog)
            return;

        const dialogWidth = this._authWidgetDialog.width;
        const dialogHeight = this._authWidgetDialog.height;

        const x = Math.max(0, Math.floor(
            (dialogWidth - AUTH_WIDGET_DIAMETER) / 2
        ));
        const y = Math.max(0, Math.floor(
            (dialogHeight - AUTH_WIDGET_DIAMETER) / 2
        ));

        this._authWidgetActor.set_position(x, y);
    }

    _removeAuthWidget() {
        const dialog = this._authWidgetDialog;

        this._authWidgetUnbindEntryOperations();
        this._authWidgetRestoreGnomeAuthChrome();
        this._authWidgetCancelFailureDisplay();
        this._authWidgetVerificationInProgress = false;
        this._authWidgetFailureLatched = false;
        this._authWidgetResetChildren();
        this._authWidgetChildren = [];
        this._authWidgetFadeSources = [];
        this._authWidgetNextChild = 0;

        if (dialog && this._authWidgetWidthSignal) {
            try {
                dialog.disconnect(this._authWidgetWidthSignal);
            } catch (_) {
                // Extension teardown must remain best-effort.
            }
        }

        if (dialog && this._authWidgetHeightSignal) {
            try {
                dialog.disconnect(this._authWidgetHeightSignal);
            } catch (_) {
                // Extension teardown must remain best-effort.
            }
        }

        this._authWidgetWidthSignal = 0;
        this._authWidgetHeightSignal = 0;
        this._authWidgetDialog = null;

        if (this._authWidgetActor) {
            console.log('[Screenshaver] Removing GNOME static auth widget');
            this._authWidgetActor.destroy();
            this._authWidgetActor = null;
        }
    }


    _diagLog(message) {
        const timestamp =
            GLib.DateTime.new_now_local()
                .format('%Y-%m-%d %H:%M:%S.%f');

        const line =
            `[${timestamp}] ${message}\n`;

        console.log(
            `[Screenshaver][AuthDiag] ${message}`
        );

        try {
            if (!this._authDiagnosticPath) {
                const runtimeDir =
                    GLib.getenv('XDG_RUNTIME_DIR')
                    || GLib.get_tmp_dir();

                this._authDiagnosticPath =
                    GLib.build_filenamev([
                        runtimeDir,
                        AUTH_DIAGNOSTIC_FILENAME,
                    ]);
            }

            const file =
                Gio.File.new_for_path(
                    this._authDiagnosticPath
                );

            const stream =
                file.append_to(
                    Gio.FileCreateFlags.NONE,
                    null
                );

            const encoded =
                new TextEncoder().encode(line);

            stream.write_all(
                encoded,
                null
            );
            stream.close(null);
        } catch (error) {
            console.log(
                `[Screenshaver][AuthDiag] Unable to append diagnostic log: ${error}`
            );
        }
    }

    _authDiagObjectType(object) {
        if (!object)
            return '<null>';

        try {
            return object.constructor?.$gtype?.name
                || object.constructor?.name
                || typeof object;
        } catch (_) {
            return typeof object;
        }
    }

    _authDiagActorState(actor) {
        if (!actor)
            return null;

        const state = {};

        try {
            state.visible = actor.visible;
        } catch (_) {
        }

        try {
            state.mapped = actor.mapped;
        } catch (_) {
        }

        try {
            state.opacity = actor.opacity;
        } catch (_) {
        }

        try {
            state.reactive = actor.reactive;
        } catch (_) {
        }

        try {
            state.canFocus = actor.can_focus;
        } catch (_) {
        }

        try {
            state.hasKeyFocus =
                global.stage?.get_key_focus?.() === actor;
        } catch (_) {
        }

        return state;
    }

    _authDiagDescribeActor(actor) {
        if (!actor)
            return '<none>';

        const type =
            this._authDiagObjectType(actor);

        const extras = [];

        try {
            const name =
                actor.get_name?.() || '';

            if (name)
                extras.push(`name=${JSON.stringify(name)}`);
        } catch (_) {
        }

        try {
            const styleClass =
                actor.get_style_class_name?.() || '';

            if (styleClass)
                extras.push(`style=${JSON.stringify(styleClass)}`);
        } catch (_) {
        }

        const state =
            this._authDiagActorState(actor);

        if (state) {
            for (const [key, value] of
                Object.entries(state)) {
                extras.push(`${key}=${value}`);
            }
        }

        return (
            `${type}` +
            (extras.length
                ? ` {${extras.join(', ')}}`
                : '')
        );
    }

    _authDiagEnumerateSignals(label, object) {
        if (!object?.constructor)
            return;

        try {
            const signalIds =
                GObject.signal_list_ids(
                    object.constructor
                );

            const names = [];

            for (const signalId of signalIds) {
                const query =
                    GObject.signal_query(signalId);

                const name =
                    query?.signal_name;

                if (!name)
                    continue;

                if (
                    /auth|verif|fail|complete|reset|prompt|message|cancel|focus|mapped|show|hide|activate|text|insert|delete|change|edit/i
                        .test(name)
                ) {
                    names.push(name);
                }
            }

            names.sort();

            if (names.length > 0) {
                this._diagLog(
                    `${label}: relevant GObject signals = ${names.join(', ')}`
                );
            }
        } catch (error) {
            this._diagLog(
                `${label}: unable to enumerate GObject signals: ${error}`
            );
        }
    }

    _authDiagConnectSafeSignal(
        label,
        object,
        signalName
    ) {
        if (!object?.connect)
            return;

        try {
            const signalId =
                GObject.signal_lookup(
                    signalName,
                    object.constructor
                );

            if (!signalId)
                return;

            const handlerId =
                object.connect(
                    signalName,
                    () => {
                        // Deliberately ignore all signal arguments.
                        // Prompt text and password data are never logged.
                        if (
                            label === 'dialog._authPrompt'
                            && signalName === 'cancelled'
                        ) {
                            this._authWidgetCancelFailureDisplay();
                            this._authWidgetVerificationInProgress = false;
                            this._authWidgetFailureLatched = false;
                            this._authWidgetResetSequence();
                            this._authWidgetUnbindEntryOperations();
                        }

                        this._diagLog(
                            `${label}: signal '${signalName}' emitted ` +
                            `(arguments intentionally ignored)`
                        );
                    }
                );

            this._authDiagnosticConnections.push(
                [object, handlerId]
            );

            this._diagLog(
                `${label}: observing safe signal '${signalName}'`
            );
        } catch (error) {
            this._diagLog(
                `${label}: unable to observe '${signalName}': ${error}`
            );
        }
    }

    _authDiagWatchActor(label, actor) {
        if (!actor?.connect)
            return;

        for (const property of [
            'visible',
            'mapped',
            'opacity',
            'reactive',
            'can-focus',
        ]) {
            try {
                const handlerId =
                    actor.connect(
                        `notify::${property}`,
                        () => {
                            this._authDiagRecordState(
                                label,
                                actor,
                                true
                            );
                        }
                    );

                this._authDiagnosticConnections.push(
                    [actor, handlerId]
                );
            } catch (_) {
                // Not every object exposes every actor property.
            }
        }
    }

    _authDiagRecordState(
        label,
        actor,
        force = false
    ) {
        const state =
            this._authDiagActorState(actor);

        if (!state)
            return;

        const serialized =
            JSON.stringify(state);

        if (
            !force
            && this._authDiagnosticLastState.get(label)
                === serialized
        ) {
            return;
        }

        this._authDiagnosticLastState.set(
            label,
            serialized
        );

        this._diagLog(
            `${label}: state ${serialized}`
        );
    }

    _authDiagCollectNamedObjects(dialog) {
        const objects = [];

        const add = (label, object) => {
            if (!object)
                return;

            if (
                objects.some(
                    entry => entry.object === object
                )
            ) {
                return;
            }

            objects.push({
                label,
                object,
            });
        };

        add(
            'dialog',
            dialog
        );

        const candidates = [
            [
                'dialog._authPrompt',
                dialog?._authPrompt,
            ],
            [
                'dialog._userVerifier',
                dialog?._userVerifier,
            ],
            [
                'dialog._entry',
                dialog?._entry,
            ],
            [
                'dialog._message',
                dialog?._message,
            ],
            [
                'dialog._spinner',
                dialog?._spinner,
            ],
            [
                'dialog._authPrompt._userVerifier',
                dialog?._authPrompt?._userVerifier,
            ],
            [
                'dialog._authPrompt._entry',
                dialog?._authPrompt?._entry,
            ],
            [
                'dialog._authPrompt._message',
                dialog?._authPrompt?._message,
            ],
            [
                'dialog._authPrompt._spinner',
                dialog?._authPrompt?._spinner,
            ],
            [
                'dialog._authPrompt._passwordEntry',
                dialog?._authPrompt?._passwordEntry,
            ],
            [
                'dialog._authPrompt._textEntry',
                dialog?._authPrompt?._textEntry,
            ],
            [
                'dialog._authPrompt._inactiveEntry',
                dialog?._authPrompt?._inactiveEntry,
            ],
        ];

        for (const [label, object] of candidates)
            add(label, object);

        // St.Entry/St.PasswordEntry commonly wrap a ClutterText object. Observe
        // that object too, but never read its text property or signal arguments.
        const entryCandidates = [
            [
                'dialog._authPrompt._entry',
                dialog?._authPrompt?._entry,
            ],
            [
                'dialog._authPrompt._passwordEntry',
                dialog?._authPrompt?._passwordEntry,
            ],
            [
                'dialog._authPrompt._textEntry',
                dialog?._authPrompt?._textEntry,
            ],
        ];

        for (const [label, entry] of entryCandidates) {
            if (!entry)
                continue;

            try {
                const clutterText =
                    entry.get_clutter_text?.()
                    || entry.clutter_text
                    || entry._clutterText
                    || null;

                add(
                    `${label}.ClutterText`,
                    clutterText
                );
            } catch (_) {
                // Some entry implementations do not expose a nested actor.
            }
        }

        return objects;
    }

    _authDiagDumpInterestingKeys(
        label,
        object
    ) {
        if (!object)
            return;

        try {
            const keys =
                Object.getOwnPropertyNames(object)
                    .filter(
                        key =>
                            /auth|prompt|verif|entry|message|spinner|password|login|user|cancel|fail|busy|working/i
                                .test(key)
                    )
                    .sort();

            if (keys.length > 0) {
                this._diagLog(
                    `${label}: relevant object fields = ${keys.join(', ')}`
                );
            }
        } catch (error) {
            this._diagLog(
                `${label}: unable to inspect field names: ${error}`
            );
        }
    }


    _authDiagObserveObject(label, object) {
        if (!object)
            return;

        if (this._authDiagnosticObservedObjects.has(object))
            return;

        this._authDiagnosticObservedObjects.add(object);

        // Presentation-only test: suppress the rendered GNOME password-entry
        // actor while leaving GNOME's focus, keyboard input, PAM, retries, and
        // unlock authority completely intact.
        try {
            this._authWidgetSuppressGnomeAuthChrome(
                label,
                object
            );
        } catch (error) {
            console.log(
                `[Screenshaver] GNOME auth-chrome visual suppression skipped: ${error}`
            );
        }

        // Functional widget animation remains content-blind: bind only to the
        // sanitized insert/delete operation signals exposed by ClutterText.
        try {
            this._authWidgetBindEntryOperations(
                label,
                object
            );
        } catch (error) {
            console.log(
                `[Screenshaver] Auth widget operation binding skipped: ${error}`
            );
        }

        this._diagLog(
            `${label}: dynamically observing object type ` +
            `${this._authDiagObjectType(object)}`
        );

        this._authDiagDumpInterestingKeys(
            label,
            object
        );

        this._authDiagEnumerateSignals(
            label,
            object
        );

        this._authDiagWatchActor(
            label,
            object
        );

        this._authDiagRecordState(
            label,
            object,
            true
        );

        for (
            const signalName
            of AUTH_DIAGNOSTIC_SAFE_SIGNALS
        ) {
            this._authDiagConnectSafeSignal(
                label,
                object,
                signalName
            );
        }

        // Observe only that a relevant property changed. Never read or log
        // the password-entry contents or their length.
        if (object?.connect) {
            for (const property of [
                'text',
                'password-visible',
                'sensitive',
            ]) {
                try {
                    const handlerId =
                        object.connect(
                            `notify::${property}`,
                            () => {
                                this._diagLog(
                                    `${label}: property '${property}' changed`
                                );
                            }
                        );

                    this._authDiagnosticConnections.push(
                        [object, handlerId]
                    );
                } catch (_) {
                    // Property is not exposed by this object.
                }
            }
        }
    }

    _authDiagDumpActorTree(
        actor,
        label = 'dialog',
        depth = 0
    ) {
        if (!actor || depth > 5)
            return;

        this._diagLog(
            `${'  '.repeat(depth)}` +
            `${label}: ` +
            `${this._authDiagDescribeActor(actor)}`
        );

        let children = [];

        try {
            children =
                actor.get_children?.() || [];
        } catch (_) {
            return;
        }

        for (
            let index = 0;
            index < children.length;
            index++
        ) {
            this._authDiagDumpActorTree(
                children[index],
                `${label}.child[${index}]`,
                depth + 1
            );
        }
    }

    _startAuthDiagnostics(dialog) {
        try {
            this._startAuthDiagnosticsInner(dialog);
        } catch (error) {
            console.log(
                `[Screenshaver][AuthDiag] Observation disabled after runtime error: ${error}`
            );
            this._stopAuthDiagnostics();
        }
    }

    _startAuthDiagnosticsInner(dialog) {
        this._stopAuthDiagnostics();

        const runtimeDir =
            GLib.getenv('XDG_RUNTIME_DIR')
            || GLib.get_tmp_dir();

        this._authDiagnosticPath =
            GLib.build_filenamev([
                runtimeDir,
                AUTH_DIAGNOSTIC_FILENAME,
            ]);

        try {
            GLib.file_set_contents(
                this._authDiagnosticPath,
                ''
            );
        } catch (_) {
            // _diagLog() will report append errors if needed.
        }

        this._diagLog(
            '=== Screenshaver GNOME authentication observation started ==='
        );
        this._diagLog(
            'SECURITY: no keyboard handlers are connected; no entry text, length, or signal arguments are read or logged.'
        );
        this._diagLog(
            'EXPERIMENT: GNOME-native AuthPrompt.mapped auth-circle visibility with persistent opacity-only suppression of all legacy GNOME lock/auth chrome.'
        );
        this._diagLog(
            `Diagnostic file: ${this._authDiagnosticPath}`
        );

        this._authDiagDumpInterestingKeys(
            'dialog',
            dialog
        );
        this._authDiagDumpActorTree(
            dialog
        );

        const namedObjects =
            this._authDiagCollectNamedObjects(
                dialog
            );

        for (const {
            label,
            object,
        } of namedObjects) {
            this._authDiagObserveObject(
                label,
                object
            );
        }

        try {
            const handlerId =
                global.stage.connect(
                    'notify::key-focus',
                    () => {
                        const focused =
                            global.stage.get_key_focus();

                        this._diagLog(
                            `stage key focus -> ` +
                            `${this._authDiagDescribeActor(focused)}`
                        );
                    }
                );

            this._authDiagnosticConnections.push(
                [global.stage, handlerId]
            );

            this._diagLog(
                `stage initial key focus -> ` +
                `${this._authDiagDescribeActor(global.stage.get_key_focus())}`
            );
        } catch (error) {
            this._diagLog(
                `Unable to observe stage key focus: ${error}`
            );
        }

        this._authDiagnosticPollSource =
            GLib.timeout_add(
                GLib.PRIORITY_DEFAULT,
                AUTH_DIAGNOSTIC_POLL_INTERVAL_MS,
                () => {
                    if (!this._lockActor)
                        return GLib.SOURCE_REMOVE;

                    for (const {
                        label,
                        object,
                    } of
                        this._authDiagCollectNamedObjects(
                            dialog
                        )) {
                        this._authDiagObserveObject(
                            label,
                            object
                        );

                        this._authDiagRecordState(
                            label,
                            object
                        );
                    }

                    try {
                        this._authWidgetRefreshGnomeAuthChromeSuppression(
                            dialog
                        );
                    } catch (error) {
                        console.log(
                            `[Screenshaver] GNOME auth-chrome suppression refresh skipped: ${error}`
                        );
                    }

                    try {
                        this._authWidgetSyncVisibilityToGnomePrompt(
                            dialog
                        );
                    } catch (error) {
                        console.log(
                            `[Screenshaver] Auth-circle visibility observation skipped: ${error}`
                        );
                    }

                    try {
                        this._authWidgetCheckFailureState(
                            dialog
                        );
                    } catch (error) {
                        console.log(
                            `[Screenshaver] Auth failure-state observation skipped: ${error}`
                        );
                    }

                    return GLib.SOURCE_CONTINUE;
                }
            );
    }

    _stopAuthDiagnostics() {
        if (this._authDiagnosticPollSource) {
            GLib.source_remove(
                this._authDiagnosticPollSource
            );
            this._authDiagnosticPollSource = null;
        }

        for (
            const [object, handlerId]
            of this._authDiagnosticConnections || []
        ) {
            try {
                object.disconnect(
                    handlerId
                );
            } catch (_) {
                // Diagnostic teardown remains best-effort.
            }
        }

        this._authDiagnosticConnections = [];
        this._authDiagnosticLastState?.clear?.();
        this._authDiagnosticObservedObjects?.clear?.();

        if (this._authDiagnosticPath) {
            this._diagLog(
                '=== Screenshaver GNOME authentication observation stopped ==='
            );
        }
    }

    _refreshFrame() {
        if (!this._imageContent)
            return;

        if (!this._mappedFile && !this._openTransport())
            return;

        // GJS converts GLib.Bytes data to a JavaScript Uint8Array snapshot.
        // Do not retain that snapshot across polls: reacquire it from the live
        // GLib.MappedFile each time so changes published by Screenshaver are
        // visible to this process.
        let data;

        try {
            data = this._mappedFile.get_bytes().get_data();
        } catch (error) {
            console.log(`[Screenshaver] Unable to refresh shared-memory view: ${error}`);
            this._closeTransport();
            return;
        }

        if (!this._validateHeader(data)) {
            this._closeTransport();
            return;
        }

        const width = readU32LE(data, HEADER_WIDTH_OFFSET);
        const height = readU32LE(data, HEADER_HEIGHT_OFFSET);
        const rowstride = readU32LE(data, HEADER_ROWSTRIDE_OFFSET);
        const frameBytes = readU32LE(data, HEADER_FRAME_BYTES_OFFSET);
        const activeSlot = readU32LE(data, HEADER_ACTIVE_SLOT_OFFSET);
        const frameCounter = readU32LE(data, HEADER_FRAME_COUNTER_OFFSET);

        if (frameCounter === 0 || frameCounter === this._lastFrameCounter)
            return;

        if (this._lastFrameCounter === 0) {
            console.log(
                `[Screenshaver] First shared-memory frame observed: counter=${frameCounter}, slot=${activeSlot}`
            );
        }

        if (activeSlot >= TRANSPORT_SLOT_COUNT)
            return;

        const frameOffset = TRANSPORT_HEADER_BYTES + activeSlot * frameBytes;
        const frameEnd = frameOffset + frameBytes;

        if (frameEnd > data.length) {
            console.log('[Screenshaver] Shared-memory frame extends beyond mapped transport');
            this._closeTransport();
            return;
        }

        // Copy only the completed active slot into immutable GLib.Bytes for
        // St.ImageContent. The expensive per-frame whole-file read/replace path
        // is gone; the source pixels now come directly from the mmap region.
        const frameView = data.subarray(frameOffset, frameEnd);

        try {
            const coglContext = global.stage.context
                .get_backend()
                .get_cogl_context();

            this._imageContent.set_bytes(
                coglContext,
                GLib.Bytes.new(frameView),
                Cogl.PixelFormat.RGBA_8888,
                width,
                height,
                rowstride
            );

            // St.ImageContent has new pixels, but GNOME Shell can leave the
            // lock-screen background actor visually unchanged while the
            // session is idle. Explicitly queue a redraw so every published
            // frame can become visible without keyboard or pointer activity.
            if (this._lockActor)
                this._lockActor.queue_redraw();

            // Diagnostic build: deliberately do not force ScreenShield wake.
            // We need to observe the native transition caused by real input.

            this._lastFrameCounter = frameCounter;
            this._displayedFrames++;

            if (this._displayedFrames % 300 === 0) {
                console.log(
                    `[Screenshaver] Shared-memory frames displayed: ${this._displayedFrames}`
                );
            }
        } catch (error) {
            console.log(`[Screenshaver] Shared-memory frame upload failed: ${error}`);
        }
    }



    _startPowerSaveRecovery() {
        this._subscribePowerSaveModeChanges();

        // PropertiesChanged is the normal path. Keep a slow read-only poll as
        // a safety net in case Mutter misses or suppresses a property signal.
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

    _stopPowerSaveRecovery() {
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

        if (value === 3 && previousPowerSaveMode !== 3) {
            this._maybePowerSaveThenWake();
            return;
        }

        // The successful Set(PowerSaveMode=0) normally produces this signal.
        // Treat it as the synchronization point and wake ScreenShield
        // immediately; the fallback readback below is only for missing signals.
        if (value === 0 && fromSignal && this._pendingPowerWakeCycle !== 0) {
            const cycle = this._pendingPowerWakeCycle;
            this._pendingPowerWakeCycle = 0;

            const screenShield = Main.screenShield;
            if (screenShield?._isActive &&
                Main.sessionMode.currentMode === 'unlock-dialog' &&
                this._lockActor) {
                this._requestLockScreenWake(cycle);
            }
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

    _maybePowerSaveThenWake() {
        if (this._powerSaveResetInFlight)
            return;

        // Guard against an accidental D-Bus feedback loop. The observed Mutter
        // reblank interval is much longer than this cooldown.
        const nowUs = GLib.get_monotonic_time();
        if (this._lastPowerWakeAttemptUs !== 0 &&
            nowUs - this._lastPowerWakeAttemptUs < 3000000) {
            return;
        }

        const screenShield = Main.screenShield;

        if (Main.sessionMode.currentMode !== 'unlock-dialog' ||
            !screenShield?._isActive ||
            !this._lockActor ||
            this._displayedFrames === 0) {
            return;
        }

        this._lastPowerWakeAttemptUs = nowUs;
        this._powerWakeCycleCount++;
        const cycle = this._powerWakeCycleCount;
        this._pendingPowerWakeCycle = cycle;
        this._powerSaveResetInFlight = true;

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
                    this._powerSaveResetInFlight = false;

                    try {
                        connection.call_finish(result);
                    } catch (error) {
                        if (this._pendingPowerWakeCycle === cycle)
                            this._pendingPowerWakeCycle = 0;

                        console.log(
                            `[Screenshaver] Unable to restore Mutter PowerSaveMode: ${error}`
                        );
                        return;
                    }

                    // Normal path: PropertiesChanged(0) already requested the
                    // wake. If that signal never arrives, verify after 500 ms.
                    GLib.timeout_add(
                        GLib.PRIORITY_DEFAULT,
                        500,
                        () => {
                            if (this._pendingPowerWakeCycle === cycle)
                                this._verifyPowerSaveThenWake(cycle);
                            return GLib.SOURCE_REMOVE;
                        }
                    );
                }
            );
        } catch (error) {
            this._powerSaveResetInFlight = false;
            if (this._pendingPowerWakeCycle === cycle)
                this._pendingPowerWakeCycle = 0;

            console.log(
                `[Screenshaver] Unable to dispatch Mutter PowerSaveMode recovery: ${error}`
            );
        }
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

                        const screenShield = Main.screenShield;
                        if (value === 0 && screenShield?._isActive &&
                            Main.sessionMode.currentMode === 'unlock-dialog' &&
                            this._pendingPowerWakeCycle === cycle) {
                            this._pendingPowerWakeCycle = 0;
                            this._requestLockScreenWake(cycle);
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

    _requestLockScreenWake(_cycle) {
        const screenShield = Main.screenShield;

        if (!screenShield) {
            console.log('[Screenshaver] GNOME ScreenShield unavailable for wake request');
            return;
        }

        try {
            if (typeof screenShield._wakeUpScreen === 'function') {
                screenShield._wakeUpScreen();
                this._wakeRequestCount++;
            } else {
                console.log('[Screenshaver] GNOME ScreenShield wake method unavailable');
            }
        } catch (error) {
            console.log(`[Screenshaver] GNOME lock-screen wake request failed: ${error}`);
        }
    }

    _openTransport() {
        const framePath = GLib.build_filenamev([
            GLib.get_user_runtime_dir(),
            TRANSPORT_FILENAME,
        ]);

        try {
            this._mappedFile = GLib.MappedFile.new(framePath, false);
            const mappedData = this._mappedFile.get_bytes().get_data();

            if (!mappedData || mappedData.length < TRANSPORT_HEADER_BYTES) {
                this._closeTransport();
                return false;
            }

            if (!this._validateHeader(mappedData)) {
                this._closeTransport();
                return false;
            }

            if (
                !this._activeSessionId
                || readSessionIdHex(mappedData, HEADER_SESSION_ID_OFFSET)
                    !== this._activeSessionId
            ) {
                console.log(
                    '[Screenshaver] Shared-memory transport session identity mismatch'
                );
                this._closeTransport();
                return false;
            }

            const width = readU32LE(mappedData, HEADER_WIDTH_OFFSET);
            const height = readU32LE(mappedData, HEADER_HEIGHT_OFFSET);
            const frameBytes = readU32LE(mappedData, HEADER_FRAME_BYTES_OFFSET);
            const requiredBytes = TRANSPORT_HEADER_BYTES
                + frameBytes * TRANSPORT_SLOT_COUNT;

            if (mappedData.length < requiredBytes) {
                console.log(
                    `[Screenshaver] Shared-memory transport is ${mappedData.length} bytes; ` +
                    `${requiredBytes} bytes are required`
                );
                this._closeTransport();
                return false;
            }

            this._imageContent = St.ImageContent.new_with_preferred_size(
                width,
                height
            );
            this._lockActor.content = this._imageContent;

            this._transportErrorLogged = false;

            console.log(
                `[Screenshaver] Shared-memory transport mapped: ` +
                `${width}x${height}, ${mappedData.length} bytes`
            );

            return true;
        } catch (error) {
            if (!this._transportErrorLogged) {
                console.log(
                    `[Screenshaver] Waiting for shared-memory transport: ${error}`
                );
                this._transportErrorLogged = true;
            }

            this._closeTransport();
            return false;
        }
    }

    _validateHeader(data, logErrors = true) {
        if (!data || data.length < TRANSPORT_HEADER_BYTES)
            return false;

        for (let i = 0; i < TRANSPORT_MAGIC.length; i++) {
            if (data[HEADER_MAGIC_OFFSET + i] !== TRANSPORT_MAGIC[i]) {
                if (logErrors)
                    console.log('[Screenshaver] Shared-memory transport magic mismatch');
                return false;
            }
        }

        const version = readU32LE(data, HEADER_VERSION_OFFSET);
        const headerBytes = readU32LE(data, HEADER_SIZE_OFFSET);
        const slotCount = readU32LE(data, HEADER_SLOT_COUNT_OFFSET);

        if (version !== TRANSPORT_VERSION) {
            if (logErrors) {
                console.log(
                    `[Screenshaver] Unsupported shared-memory transport version: ${version}`
                );
            }
            return false;
        }

        if (headerBytes !== TRANSPORT_HEADER_BYTES || slotCount !== TRANSPORT_SLOT_COUNT) {
            if (logErrors)
                console.log('[Screenshaver] Shared-memory transport layout mismatch');
            return false;
        }

        return true;
    }

    _closeTransport() {
        this._mappedFile = null;
        this._lastFrameCounter = 0;
    }

    _removeLockActor() {
        this._stopSessionValidation();
        this._stopAuthDiagnostics();
        this._stopPowerSaveRecovery();
        this._removeAuthWidget();

        if (this._pollSource) {
            GLib.source_remove(this._pollSource);
            this._pollSource = null;
        }

        if (this._lockActor) {
            console.log('[Screenshaver] Removing shared-memory lock actor');
            this._lockActor.destroy();
            this._lockActor = null;
        }

        this._imageContent = null;
        this._displayedFrames = 0;
        this._transportErrorLogged = false;
        this._wakeRequestCount = 0;
        this._lastObservedPowerSaveMode = null;
        this._powerSaveResetInFlight = false;
        this._powerWakeCycleCount = 0;
        this._pendingPowerWakeCycle = 0;
        this._lastPowerWakeAttemptUs = 0;
        this._powerSaveFallbackQueryInFlight = false;
        this._activeSessionId = null;
        this._closeTransport();
    }
}



function readSessionIdHex(data, offset) {
    if (!data || data.length < offset + SESSION_ID_BYTES)
        return null;

    let result = '';

    for (let i = 0; i < SESSION_ID_BYTES; i++)
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
