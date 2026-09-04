import Cogl from 'gi://Cogl';
import GLib from 'gi://GLib';
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
        this._lastObservedPowerSaveMode = null;
        this._powerSaveResetInFlight = false;
        this._powerWakeCycleCount = 0;
        this._pendingPowerWakeCycle = 0;
        this._lastPowerWakeAttemptUs = 0;
        this._powerSaveSignalId = 0;
        this._powerSaveFallbackSource = null;
        this._powerSaveFallbackQueryInFlight = false;

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

        // Start the proven rendering and PowerSave recovery paths.
        // GNOME retains its native lock/authentication UI above this actor.
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

        // A successful Set(PowerSaveMode=0) normally produces this signal.
        // Clearing the pending recovery cycle is sufficient: Mutter restoring
        // display power must not be followed by ScreenShield._wakeUpScreen(),
        // because that private wake path performs GNOME's fade-out/fade-in
        // transition. GNOME remains fully responsible for its native lock UI.
        if (value === 0 && fromSignal && this._pendingPowerWakeCycle !== 0)
            this._pendingPowerWakeCycle = 0;
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

                    // Normal path: PropertiesChanged(0) confirms the display
                    // power restoration. If that signal never arrives, verify
                    // after 500 ms. No ScreenShield wake animation is requested.
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
        this._stopPowerSaveRecovery();

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
