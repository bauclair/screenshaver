import Clutter from 'gi://Clutter';
import Cogl from 'gi://Cogl';
import GLib from 'gi://GLib';
import Gio from 'gi://Gio';
import GObject from 'gi://GObject';
import Pango from 'gi://Pango';
import Shell from 'gi://Shell';
import St from 'gi://St';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';


let shaderEffectTypeSerial = 0;

function createShaderEffectClass(shaderBody, generation) {
    // GObject type registrations survive effect destruction and can also survive
    // extension disable/enable cycles inside the same GNOME Shell process.
    // Include monotonic time plus a module-local serial so every construction
    // attempt receives a process-unique type name, even when a candidate fails
    // before becoming the active generation.
    shaderEffectTypeSerial++;
    const typeStamp = GLib.get_monotonic_time();
    const gtypeName = `ScreenshaverGnomeLockGLSLEffect${typeStamp}S${shaderEffectTypeSerial}G${generation}`;

    return GObject.registerClass(
    {
        GTypeName: gtypeName,
    },
    class extends Shell.GLSLEffect {
        vfunc_build_pipeline() {
            this.add_glsl_snippet(
                Cogl.SnippetHook.FRAGMENT,
                `
                    uniform float iTime;
                    uniform vec3 iResolution;

                    ${shaderBody}
                `,
                `
                    vec2 uv = cogl_tex_coord0_in.st;
                    vec2 fragCoord = vec2(
                        uv.x * iResolution.x,
                        (1.0 - uv.y) * iResolution.y
                    );
                    vec4 fragColor = vec4(0.0);
                    mainImage(fragColor, fragCoord);
                    cogl_color_out = fragColor;
                `,
                true
            );
        }
    });
}

const RUNTIME_SHADER_FILENAME = 'screenshaver-gnome-lock-shader.glsl';
const RUNTIME_METADATA_FILENAME = 'screenshaver-gnome-lock-metadata.txt';
const CONTROL_FILENAME = 'screenshaver-lock-control.bin';
const FRAME_FILENAME_PREFIX = 'screenshaver-lock-frame-';
const FRAME_FILENAME_SUFFIX = '.rgba';
const CONTROL_MAGIC = [0x53, 0x48, 0x56, 0x52, 0x47, 0x4e, 0x46, 0x31]; // SHVRGNF1
const CONTROL_VERSION = 1;
const CONTROL_BYTES = 64;
const CONTROL_SESSION_ID_BYTES = 16;
const POLL_INTERVAL_MS = 33;
const SHADER_TICK_INTERVAL_MS = 8;
const SHADER_SOURCE_POLL_INTERVAL_MS = 250;
const FPS_AVERAGE_WINDOW_US = 5 * 1000000;
const FPS_CRITICAL_BLINK_INTERVAL_US = 500 * 1000;

const SHADER_METRICS_REPORT_INTERVAL_US = 5 * 1000000;
const POWER_SAVE_FALLBACK_INTERVAL_MS = 1000;
const POST_WAKE_POWER_SAVE_MIN_DELAY_MS = 10000;
const POST_BLANK_SCREENSHIELD_WAKE_DELAY_MS = 250;
const RUNTIME_MARKER_FILENAME = 'screenshaver-gnome-lock.active';
const RUNTIME_MARKER_VERSION = 1;
const SESSION_VALIDATION_INTERVAL_MS = 1000;

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
        this._shaderUniformResolution = -1;
        this._shaderTickSource = null;
        this._shaderSourcePoll = null;
        this._activeProductionSource = null;
        this._shaderGeneration = 0;
        this._shaderStartedUs = 0;
        this._shaderTicks = 0;
        this._descriptionPill = null;
        this._descriptionMetadata = null;
        this._activeMetadataSignature = null;
        this._fpsWarningState = 'normal';
        this._fpsBlinkVisible = true;
        this._lastFpsBlinkUs = 0;
        this._fpsWindowStartedUs = 0;
        this._fpsWindowTicks = 0;
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

    _runtimeShaderPath() {
        return GLib.build_filenamev([
            GLib.get_user_runtime_dir(),
            RUNTIME_SHADER_FILENAME,
        ]);
    }


    _runtimeMetadataPath() {
        return GLib.build_filenamev([
            GLib.get_user_runtime_dir(),
            RUNTIME_METADATA_FILENAME,
        ]);
    }

    _readPresentationMetadata() {
        const metadataPath = this._runtimeMetadataPath();
        const metadataFile = Gio.File.new_for_path(metadataPath);
        const [ok, contents] = metadataFile.load_contents(null);

        if (!ok)
            throw new Error(`Unable to read GNOME lock metadata handoff ${metadataPath}`);

        const values = new Map();
        const text = new TextDecoder().decode(contents);

        for (const rawLine of text.split('\n')) {
            const line = rawLine.trimEnd();

            if (!line)
                continue;

            const separator = line.indexOf('=');

            if (separator <= 0)
                continue;

            values.set(line.slice(0, separator), line.slice(separator + 1));
        }

        const version = Number.parseInt(values.get('version') ?? '', 10);

        if (version !== 1)
            throw new Error(`Unsupported GNOME lock metadata version: ${version}`);

        return {
            sourceBytes: Number.parseInt(values.get('source_bytes') ?? '0', 10),
            policyId: Number.parseInt(values.get('policy_id') ?? '0', 10),
            shader: values.get('shader') ?? '',
            texture: values.get('texture') ?? '',
            palette: values.get('palette') ?? '',
            configuredFps: Math.max(1, Number.parseInt(values.get('configured_fps') ?? '1', 10) || 1),
            subtitles: values.get('subtitles') === '1',
            placement: values.get('placement') ?? 'bottom:left',
        };
    }

    _metadataSignature(metadata) {
        if (!metadata)
            return null;

        return [
            metadata.sourceBytes,
            metadata.policyId,
            metadata.shader,
            metadata.texture,
            metadata.palette,
            metadata.configuredFps,
            metadata.subtitles ? 1 : 0,
            metadata.placement,
        ].join('\u001f');
    }

    _createDescriptionPill(parent) {
        if (this._descriptionPill)
            return;

        this._descriptionPill = new St.Label({
            reactive: false,
            can_focus: false,
            style: [
                'background-color: rgba(0, 0, 0, 0.588);',
                'border-radius: 999px;',
                'color: rgb(245, 245, 245);',
                'font-size: 18px;',
                'padding: 9px 16px;',
            ].join(' '),
        });

        this._descriptionPill.clutter_text.single_line_mode = true;
        this._descriptionPill.clutter_text.ellipsize = Pango.EllipsizeMode.END;
        this._descriptionPill.hide();
        parent.add_child(this._descriptionPill);
    }

    _resetFpsWarningMonitor() {
        const nowUs = GLib.get_monotonic_time();
        this._fpsWarningState = 'normal';
        this._fpsBlinkVisible = true;
        this._lastFpsBlinkUs = nowUs;
        this._fpsWindowStartedUs = nowUs;
        this._fpsWindowTicks = 0;
        this._updateDescriptionPill();
    }

    _recordGnomePresentationTick(nowUs) {
        if (!this._descriptionMetadata)
            return;

        if (this._fpsWindowStartedUs <= 0)
            this._fpsWindowStartedUs = nowUs;

        this._fpsWindowTicks++;

        const elapsedUs = nowUs - this._fpsWindowStartedUs;

        if (elapsedUs >= FPS_AVERAGE_WINDOW_US) {
            const elapsedSeconds = elapsedUs / 1000000.0;
            const effectiveFps = this._fpsWindowTicks / elapsedSeconds;
            const configuredFps = Math.max(1, this._descriptionMetadata.configuredFps);
            let nextState = 'normal';

            if (effectiveFps < configuredFps / 2.0)
                nextState = 'critical';
            else if (effectiveFps < configuredFps / 1.5)
                nextState = 'warning';

            if (nextState !== this._fpsWarningState) {
                this._fpsWarningState = nextState;
                this._fpsBlinkVisible = true;
                this._lastFpsBlinkUs = nowUs;
                this._updateDescriptionPill();

                console.log(
                    `[Screenshaver] GNOME lock FPS state=${nextState} measured=${effectiveFps.toFixed(2)} configured=${configuredFps}`
                );
            }

            this._fpsWindowStartedUs = nowUs;
            this._fpsWindowTicks = 0;
        }

        if (this._fpsWarningState === 'critical'
            && nowUs - this._lastFpsBlinkUs >= FPS_CRITICAL_BLINK_INTERVAL_US) {
            this._fpsBlinkVisible = !this._fpsBlinkVisible;
            this._lastFpsBlinkUs = nowUs;
            // Preserve the capsule allocation during a CRITICAL blink.
            // The hidden phase keeps identical text metrics and changes only
            // the FPS glyph alpha, matching the production overlay behavior.
            this._updateDescriptionPill(false);
        }
    }

    _escapeMarkup(value) {
        return GLib.markup_escape_text(value ?? '', -1);
    }

    _updateDescriptionPill(reposition = true) {
        if (!this._descriptionPill || !this._descriptionMetadata)
            return;

        const metadata = this._descriptionMetadata;
        const warningActive = this._fpsWarningState !== 'normal';
        const shouldDisplay = metadata.subtitles || warningActive;

        if (!shouldDisplay) {
            this._descriptionPill.hide();
            return;
        }

        const fields = [];

        if (metadata.subtitles) {
            if (metadata.shader)
                fields.push(`P: ${this._escapeMarkup(metadata.shader)}`);
            if (metadata.texture)
                fields.push(`T: ${this._escapeMarkup(metadata.texture)}`);
            if (metadata.palette)
                fields.push(`P: ${this._escapeMarkup(metadata.palette)}`);
        }

        const fpsText = `FPS: ${metadata.configuredFps}`;

        if (this._fpsWarningState === 'warning') {
            fields.push(`<span foreground="#ffdd40">${fpsText}</span>`);
        } else if (this._fpsWarningState === 'critical') {
            // Do not remove the FPS segment during the hidden blink phase.
            // Keeping it in the Pango layout preserves the exact capsule width.
            // Only the glyph alpha changes, so the red FPS text blinks while the
            // black capsule and any descriptive text remain completely stationary.
            if (this._fpsBlinkVisible) {
                fields.push(`<span foreground="#ff4848" weight="bold">${fpsText}</span>`);
            } else {
                fields.push(`<span foreground="#ff4848" alpha="0%" weight="bold">${fpsText}</span>`);
            }
        } else if (metadata.subtitles) {
            fields.push(fpsText);
        }

        if (fields.length === 0) {
            this._descriptionPill.hide();
            return;
        }

        this._descriptionPill.clutter_text.set_markup(fields.join(' | '));
        if (reposition)
            this._positionDescriptionPill();
        this._descriptionPill.show();
    }

    _positionDescriptionPill() {
        if (!this._descriptionPill || !this._descriptionMetadata || !this._lockActor)
            return;

        const outputWidth = Math.max(1, this._lockActor.width);
        const outputHeight = Math.max(1, this._lockActor.height);
        const scale = Math.max(0.75, Math.min(2.0, outputHeight / 1080.0));
        const fontSize = Math.max(10, Math.min(72, Math.round(18 * scale)));
        const paddingX = Math.max(8, Math.round(16 * scale));
        const paddingY = Math.max(5, Math.round(9 * scale));
        const margin = Math.max(12, Math.round(24 * scale));
        const maximumWidth = Math.max(1, Math.round(outputWidth * 0.80));

        this._descriptionPill.set_style([
            'background-color: rgba(0, 0, 0, 0.588);',
            'border-radius: 999px;',
            'color: rgb(245, 245, 245);',
            `font-size: ${fontSize}px;`,
            `padding: ${paddingY}px ${paddingX}px;`,
        ].join(' '));

        // Measure the underlying ClutterText rather than the St.Label actor.
        // The label receives an explicit allocation below; querying the actor's
        // preferred width after that can feed the previous allocation back as its
        // next preferred width, making the capsule appear permanently fixed-size.
        // Measure without ellipsization so a normally sized pill gets enough
        // room for the complete string, including bold CRITICAL FPS text.
        this._descriptionPill.clutter_text.ellipsize = Pango.EllipsizeMode.NONE;

        const [, naturalTextWidth] =
            this._descriptionPill.clutter_text.get_preferred_width(-1);
        const measurementAllowance = Math.max(2, Math.ceil(4 * scale));
        const desiredWidth =
            Math.ceil(naturalTextWidth) + paddingX * 2 + measurementAllowance;
        const width = Math.min(
            maximumWidth,
            Math.max(fontSize + paddingY * 2, desiredWidth)
        );

        // Truncate only when the complete natural text genuinely exceeds the
        // production 80%-of-output maximum. Do not ellipsize ordinary pills.
        this._descriptionPill.clutter_text.ellipsize =
            desiredWidth > maximumWidth
                ? Pango.EllipsizeMode.END
                : Pango.EllipsizeMode.NONE;

        const [, naturalTextHeight] =
            this._descriptionPill.clutter_text.get_preferred_height(
                Math.max(1, width - paddingX * 2)
            );
        const height = Math.max(
            1,
            Math.ceil(naturalTextHeight) + paddingY * 2
        );

        this._descriptionPill.set_size(width, height);

        const [vertical, horizontal] = metadataPlacement(this._descriptionMetadata.placement);
        let x = margin;
        let y = margin;

        if (horizontal === 'center')
            x = Math.max(0, Math.floor((outputWidth - width) / 2));
        else if (horizontal === 'right')
            x = Math.max(0, outputWidth - width - margin);

        if (vertical === 'bottom')
            y = Math.max(0, outputHeight - height - margin);

        this._descriptionPill.set_position(x, y);
    }

    _readProductionShaderSource() {
        const shaderPath = this._runtimeShaderPath();
        const shaderFile = Gio.File.new_for_path(shaderPath);
        const [shaderOk, shaderBytes] = shaderFile.load_contents(null);

        if (!shaderOk)
            throw new Error(`Unable to read production shader handoff ${shaderPath}`);

        return {
            shaderPath,
            shaderBytes,
            productionSource: new TextDecoder().decode(shaderBytes),
        };
    }

    _buildShaderEffect(productionSource, width, height, elapsedSeconds) {
        const shaderBody =
            this._extractShaderToyBodyFromProductionSource(productionSource);

        if (!/\bvoid\s+mainImage\s*\(/.test(shaderBody)) {
            throw new Error(
                `${RUNTIME_SHADER_FILENAME} does not contain a usable preprocessed ShaderToy mainImage()`
            );
        }

        const nextGeneration = this._shaderGeneration + 1;
        const EffectClass = createShaderEffectClass(
            shaderBody,
            nextGeneration
        );
        const effect = new EffectClass();

        console.log(
            `[Screenshaver] Test #24B registered unique shader effect GType generation=${nextGeneration}`
        );

        const uniformTime = effect.get_uniform_location('iTime');
        const uniformResolution = effect.get_uniform_location('iResolution');

        effect.set_uniform_float(
            uniformTime,
            1,
            [elapsedSeconds]
        );
        effect.set_uniform_float(
            uniformResolution,
            3,
            [width, height, 1.0]
        );

        return {
            effect,
            uniformTime,
            uniformResolution,
        };
    }

    _installInitialShaderEffect(dialog) {
        const {
            shaderPath,
            shaderBytes,
            productionSource,
        } = this._readProductionShaderSource();

        const built = this._buildShaderEffect(
            productionSource,
            dialog.width,
            dialog.height,
            0.0
        );

        this._shaderEffect = built.effect;
        this._shaderUniformTime = built.uniformTime;
        this._shaderUniformResolution = built.uniformResolution;
        this._activeProductionSource = productionSource;
        this._shaderGeneration = 1;

        try {
            const metadata = this._readPresentationMetadata();
            if (!metadata.sourceBytes || metadata.sourceBytes === shaderBytes.length) {
                this._descriptionMetadata = metadata;
                this._activeMetadataSignature = this._metadataSignature(metadata);
            }
        } catch (error) {
            console.log(`[Screenshaver] GNOME description metadata unavailable: ${error}`);
        }

        this._lockActor.add_effect_with_name(
            'screenshaver-production-shader-bridge',
            this._shaderEffect
        );

        console.log(
            `[Screenshaver] Test #24B loaded production-preprocessed shader handoff: ${shaderPath} (${shaderBytes.length} bytes) generation=${this._shaderGeneration}`
        );
    }

    _startShaderSourcePolling() {
        if (this._shaderSourcePoll)
            return;

        this._shaderSourcePoll = GLib.timeout_add(
            GLib.PRIORITY_DEFAULT,
            SHADER_SOURCE_POLL_INTERVAL_MS,
            () => {
                if (!this._lockActor || !this._shaderEffect) {
                    this._shaderSourcePoll = null;
                    return GLib.SOURCE_REMOVE;
                }

                this._checkForShaderReplacement();
                return GLib.SOURCE_CONTINUE;
            }
        );

        console.log(
            `[Screenshaver] Test #24B production shader handoff polling started: ${SHADER_SOURCE_POLL_INTERVAL_MS}ms`
        );
    }

    _stopShaderSourcePolling() {
        if (this._shaderSourcePoll) {
            GLib.source_remove(this._shaderSourcePoll);
            this._shaderSourcePoll = null;
        }

        this._activeProductionSource = null;
    }

    _checkForShaderReplacement() {
        let handoff;

        try {
            handoff = this._readProductionShaderSource();
        } catch (_) {
            // Atomic replacement creates only a very short rename boundary.
            // A missing file during teardown or publication is retried on the
            // next poll without disturbing the currently rendering effect.
            return;
        }

        let observedMetadata = null;

        try {
            observedMetadata = this._readPresentationMetadata();
        } catch (_) {
            return;
        }

        if (observedMetadata.sourceBytes
            && observedMetadata.sourceBytes !== handoff.shaderBytes.length) {
            return;
        }

        const observedMetadataSignature = this._metadataSignature(observedMetadata);

        if (handoff.productionSource === this._activeProductionSource) {
            if (observedMetadataSignature !== this._activeMetadataSignature) {
                this._descriptionMetadata = observedMetadata;
                this._activeMetadataSignature = observedMetadataSignature;
                this._resetFpsWarningMonitor();
                console.log(
                    `[Screenshaver] GNOME description metadata updated for policy_id=${observedMetadata.policyId}`
                );
            }
            return;
        }

        const elapsedSeconds = this._shaderStartedUs > 0
            ? (GLib.get_monotonic_time() - this._shaderStartedUs) / 1000000.0
            : 0.0;

        let built;

        try {
            built = this._buildShaderEffect(
                handoff.productionSource,
                this._lockActor.width,
                this._lockActor.height,
                elapsedSeconds
            );
        } catch (error) {
            console.log(
                `[Screenshaver] Test #24B replacement shader preparation failed; retaining active shader: ${error}`
            );
            return;
        }

        const replacementMetadata = observedMetadata;

        const previousEffect = this._shaderEffect;

        try {
            // Keep the lock actor itself in place. Only the shader effect is
            // replaced, preserving GNOME's lock/session/power-management state.
            this._lockActor.remove_effect(previousEffect);

            this._shaderEffect = built.effect;
            this._shaderUniformTime = built.uniformTime;
            this._shaderUniformResolution = built.uniformResolution;

            this._lockActor.add_effect_with_name(
                'screenshaver-production-shader-bridge',
                this._shaderEffect
            );

            this._activeProductionSource = handoff.productionSource;
            this._shaderGeneration++;
            this._descriptionMetadata = replacementMetadata;
            this._activeMetadataSignature = observedMetadataSignature;
            this._resetFpsWarningMonitor();

            this._shaderEffect.queue_repaint();
            this._lockActor.queue_redraw();

            console.log(
                `[Screenshaver] Test #24B hot-swapped production shader generation=${this._shaderGeneration} bytes=${handoff.shaderBytes.length} unique-gtype=true`
            );
        } catch (error) {
            console.log(
                `[Screenshaver] Test #24B replacement effect swap failed: ${error}`
            );

            // Best-effort rollback to the previously proven effect.
            try {
                if (this._shaderEffect !== previousEffect)
                    this._lockActor.remove_effect(this._shaderEffect);
            } catch (_) {
            }

            this._shaderEffect = previousEffect;

            try {
                this._lockActor.add_effect_with_name(
                    'screenshaver-production-shader-bridge',
                    previousEffect
                );
            } catch (_) {
            }
        }
    }

    _extractShaderToyBodyFromProductionSource(source) {
        // Screenshaver's production ShaderToy preprocessor emits a complete
        // OpenGL 3.3 fragment shader. Shell.GLSLEffect instead needs the body
        // inserted into Cogl's fragment pipeline. Remove only Screenshaver's
        // generated OpenGL wrapper; retain the already-preprocessed shader body.
        let body = source.replace(/^\s*#version[^\n]*\n/m, '');

        body = body.replace(/^\s*out\s+vec4\s+fragColor\s*;\s*$/m, '');

        const generatedUniforms = [
            /^\s*uniform\s+float\s+iTime\s*;\s*$/gm,
            /^\s*uniform\s+float\s+iTimeDelta\s*;\s*$/gm,
            /^\s*uniform\s+vec3\s+iResolution\s*;\s*$/gm,
            /^\s*uniform\s+vec3\s+iChannelResolution\s*\[\s*4\s*\]\s*;\s*$/gm,
            /^\s*uniform\s+vec4\s+iMouse\s*;\s*$/gm,
            /^\s*uniform\s+int\s+iFrame\s*;\s*$/gm,
            /^\s*uniform\s+sampler2D\s+iChannel[0-3]\s*;\s*$/gm,
        ];

        for (const pattern of generatedUniforms)
            body = body.replace(pattern, '');

        // The production preprocessor appends this wrapper after mainImage().
        // Test #24B currently supports the ShaderToy path only, so the first
        // top-level generated main() marks the end of the body we hand to Cogl.
        const generatedMain = body.search(/\n\s*void\s+main\s*\(\s*\)\s*\{/m);

        if (generatedMain >= 0)
            body = body.slice(0, generatedMain);

        return body.trim();
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

        // Test #21: execute an external ShaderToy-style mainImage() shader through
        // Shell.GLSLEffect/Cogl.  GNOME supplies only the ShaderToy compatibility
        // uniforms/wrapper; shader source comes from Screenshaver's production Rust preprocessing path.
        this._lockActor = new St.Widget({
            reactive: false,
            can_focus: false,
            style: 'background-color: black;',
        });

        this._lockActor.set_position(0, 0);
        this._lockActor.set_size(dialog.width, dialog.height);

        try {
            this._installInitialShaderEffect(dialog);
        } catch (error) {
            console.log(
                `[Screenshaver] ERROR: Unable to create Test #24B ShaderToy Shell.GLSLEffect: ${error}`
            );
            this._lockActor.destroy();
            this._lockActor = null;
            this._shaderEffect = null;
            return;
        }

        backgroundGroup.add_child(this._lockActor);
        this._createDescriptionPill(backgroundGroup);
        this._resetFpsWarningMonitor();

        console.log(
            '[Screenshaver] Test #24B shader actor added above GNOME lock background'
        );

        // Preserve the already-proven GNOME lock/power-management handling.
        this._startPowerSaveRecovery();
        this._startShaderSourcePolling();

        this._shaderStartedUs = GLib.get_monotonic_time();
        this._shaderTicks = 0;

        // Test #24B keeps the proven GLib callback-rate instrumentation so we can
        // distinguish visible compositor presentation from a blanked output.
        this._shaderMetricsWindowStartedUs = this._shaderStartedUs;
        this._shaderMetricsWindowTicks = 0;
        this._shaderMetricsPreviousTickUs = null;
        this._shaderMetricsMinDeltaUs = Number.POSITIVE_INFINITY;
        this._shaderMetricsMaxDeltaUs = 0;

        console.log(
            `[Screenshaver] Test #24B requested shader tick interval: ${SHADER_TICK_INTERVAL_MS}ms (~${Math.round(1000 / SHADER_TICK_INTERVAL_MS)} Hz maximum)`
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
                        this._shaderUniformResolution,
                        3,
                        [this._lockActor.width, this._lockActor.height, 1.0]
                    );
                    this._shaderEffect.set_uniform_float(
                        this._shaderUniformTime,
                        1,
                        [elapsedSeconds]
                    );
                    this._shaderEffect.queue_repaint();
                    this._lockActor.queue_redraw();
                    this._positionDescriptionPill();
                } catch (error) {
                    console.log(
                        `[Screenshaver] Shell.GLSLEffect animation update failed: ${error}`
                    );
                    this._shaderTickSource = null;
                    return GLib.SOURCE_REMOVE;
                }

                this._shaderTicks++;

                const tickNowUs = GLib.get_monotonic_time();
                this._recordGnomePresentationTick(tickNowUs);
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
                        `[Screenshaver] Test #24B timing: requested=${SHADER_TICK_INTERVAL_MS}ms callbacks=${this._shaderMetricsWindowTicks} elapsed=${metricsElapsedSeconds.toFixed(3)}s effective=${effectiveHz.toFixed(2)}Hz avg=${averageIntervalMs.toFixed(2)}ms min=${minIntervalMs.toFixed(2)}ms max=${maxIntervalMs.toFixed(2)}ms total_ticks=${this._shaderTicks}`
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
                        '[Screenshaver] First Test #24B shader frame requested'
                    );
                }

                return GLib.SOURCE_CONTINUE;
            }
        );
    }

    // Legacy external-frame diagnostic stub retained temporarily. Test #21
    // consumes only the production GLSL source handoff.
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
                    `[Screenshaver] Test #24B idle inhibitor waiting: ${waitState}`
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

        console.log('[Screenshaver] Test #24B requesting GNOME session idle inhibitor (flag=8)');

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
                            `[Screenshaver] Test #24B GNOME session idle inhibitor request failed: ${error}`
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
                        `[Screenshaver] Test #24B GNOME session idle inhibitor acquired cookie=${cookie} flag=8`
                    );
                }
            );
        } catch (error) {
            if (requestGeneration === this._idleInhibitRequestGeneration)
                this._idleInhibitRequestPending = false;

            console.log(
                `[Screenshaver] Test #24B unable to dispatch GNOME session idle inhibitor request: ${error}`
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
                            `[Screenshaver] Test #24B GNOME session idle inhibitor released cookie=${cookie} (${reason})`
                        );
                    } catch (error) {
                        console.log(
                            `[Screenshaver] Test #24B GNOME session idle inhibitor release failed cookie=${cookie}: ${error}`
                        );
                    }
                }
            );
        } catch (error) {
            console.log(
                `[Screenshaver] Test #24B unable to dispatch GNOME session idle inhibitor release cookie=${cookie}: ${error}`
            );
        }
    }

    _startPowerSaveRecovery() {
        this._subscribePowerSaveModeChanges();

        // Test #21: PropertiesChanged is the primary observation path.
        // Keep a slow poll as a safety net; during a validated GNOME lock
        // presentation, a delayed BLANK state is corrected directly to NORMAL.
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

        const securePresentationActive =
            Main.sessionMode.currentMode === 'unlock-dialog' &&
            Boolean(this._activeSessionId) &&
            Boolean(this._lockActor);

        if (!securePresentationActive)
            return;

        // Test #24B keeps the startup wake sequence from the previous tests,
        // but once that sequence has completed it treats any delayed BLANK
        // state as a display-power event rather than simulated user-idle.
        // Directly restore Mutter PowerSaveMode to NORMAL without calling
        // ScreenShield._wakeUpScreen() again.
        if (this._postBlankScreenShieldWakeCompletedUs > 0 && value === 3) {
            const elapsedSincePostBlankWakeUs =
                GLib.get_monotonic_time() - this._postBlankScreenShieldWakeCompletedUs;

            if (elapsedSincePostBlankWakeUs >= POST_WAKE_POWER_SAVE_MIN_DELAY_MS * 1000) {
                if (previousPowerSaveMode !== 3) {
                    console.log(
                        `[Screenshaver] Test #24B rejecting delayed PowerSaveMode 0 -> 3 after ${Math.floor(elapsedSincePostBlankWakeUs / 1000)}ms; restoring NORMAL without ScreenShield wake`
                    );
                }
                this._setPostWakePowerSaveModeNormal();
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

            // Preserve the known startup recovery only.  This one early wake
            // handles GNOME's initial lock transition; Test #24B never uses a
            // ScreenShield wake for the later ~15-second blank attempt.
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
                `[Screenshaver] Test #24B rejecting delayed PowerSaveMode 0 -> 3 after ${Math.floor(elapsedUs / 1000)}ms; restoring NORMAL without ScreenShield wake`
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
                        console.log('[Screenshaver] Test #24B PowerSaveMode NORMAL correction completed');
                    } catch (error) {
                        if (this._lockActor)
                            console.log(`[Screenshaver] Test #24B PowerSaveMode NORMAL correction failed: ${error}`);
                    }
                }
            );
        } catch (error) {
            this._postWakePowerSaveCorrectionInFlight = false;
            console.log(`[Screenshaver] Test #24B unable to dispatch PowerSaveMode NORMAL correction: ${error}`);
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
        this._stopShaderSourcePolling();
        this._stopPowerSaveRecovery();
        this._releaseIdleInhibitor();

        if (this._postBlankScreenShieldWakeSource) {
            GLib.source_remove(this._postBlankScreenShieldWakeSource);
            this._postBlankScreenShieldWakeSource = null;
        }

        if (this._delayedBlankOpacityNudgeSource) {
            GLib.source_remove(this._delayedBlankOpacityNudgeSource);
            }


        if (this._shaderTickSource) {
            GLib.source_remove(this._shaderTickSource);
            this._shaderTickSource = null;
        }

        if (this._pollSource) {
            GLib.source_remove(this._pollSource);
            this._pollSource = null;
        }

        if (this._descriptionPill) {
            this._descriptionPill.destroy();
            this._descriptionPill = null;
        }

        if (this._lockActor) {
            console.log('[Screenshaver] Removing Shell.GLSLEffect diagnostic lock actor');
            this._lockActor.destroy();
            this._lockActor = null;
        }

        this._imageContent = null;
        this._shaderEffect = null;
        this._shaderUniformTime = -1;
        this._shaderUniformResolution = -1;
        this._shaderStartedUs = 0;
        this._shaderTicks = 0;
        this._descriptionMetadata = null;
        this._activeMetadataSignature = null;
        this._fpsWarningState = 'normal';
        this._fpsBlinkVisible = true;
        this._lastFpsBlinkUs = 0;
        this._fpsWindowStartedUs = 0;
        this._fpsWindowTicks = 0;
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
        this._powerSaveFallbackQueryInFlight = false;
        this._activeSessionId = null;
        this._transportGeneration++;
        this._lastFrameCounter = 0;
    }
}


function metadataPlacement(value) {
    const normalized = (value ?? 'bottom:left').toLowerCase();
    const [vertical, horizontal] = normalized.split(':');

    return [
        vertical === 'top' ? 'top' : 'bottom',
        ['left', 'center', 'right'].includes(horizontal) ? horizontal : 'left',
    ];
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
