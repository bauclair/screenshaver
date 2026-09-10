import Clutter from 'gi://Clutter';
import Cogl from 'gi://Cogl';
import GLib from 'gi://GLib';
import Gio from 'gi://Gio';
import GObject from 'gi://GObject';
import Graphene from 'gi://Graphene';
import Pango from 'gi://Pango';
import Shell from 'gi://Shell';
import St from 'gi://St';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as Config from 'resource:///org/gnome/shell/misc/config.js';


let shaderEffectTypeSerial = 0;

function createShaderEffectClass(shaderBody, generation, renderScale, colorPrecision, antiAliasing, dithering, bloomMode, bloomThreshold, bloomIntensity) {
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
        screenshaver_render_size(fallbackWidth, fallbackHeight) {
            const scale = Number.isFinite(renderScale) && renderScale > 0.0
                ? renderScale
                : 1.0;
            return [
                Math.max(1, Math.round(fallbackWidth * scale)),
                Math.max(1, Math.round(fallbackHeight * scale)),
            ];
        }

        screenshaver_set_fxaa_transforms(invertColors, flipHorizontal, flipVertical, hueRotation) {
            this._screenshaverFxaaInvertColors = invertColors ? 1.0 : 0.0;
            this._screenshaverFxaaFlipHorizontal = flipHorizontal ? 1.0 : 0.0;
            this._screenshaverFxaaFlipVertical = flipVertical ? 1.0 : 0.0;
            this._screenshaverFxaaHueRotation = Number.isFinite(hueRotation) ? hueRotation : 0.0;
        }

        screenshaver_create_fxaa_pipeline(coglContext, sceneTexture, width, height) {
            const pipeline = Cogl.Pipeline.new(coglContext);
            pipeline.set_layer_texture(0, sceneTexture);
            pipeline.set_layer_filters(
                0,
                Cogl.PipelineFilter.LINEAR,
                Cogl.PipelineFilter.LINEAR
            );
            pipeline.set_layer_wrap_mode(
                0,
                Cogl.PipelineWrapMode.CLAMP_TO_EDGE
            );

            const snippet = Cogl.Snippet.new(
                Cogl.SnippetHook.FRAGMENT,
                `
                    uniform vec2 screenshaverFxaaInverseResolution;
                    uniform float screenshaverFxaaInvertColors;
                    uniform float screenshaverFxaaFlipHorizontal;
                    uniform float screenshaverFxaaFlipVertical;
                    uniform float screenshaverFxaaHueRotation;

                    const float SCREENSHAVER_FXAA_EDGE_THRESHOLD_MIN = 0.0312;
                    const float SCREENSHAVER_FXAA_EDGE_THRESHOLD_MAX = 0.125;
                    const float SCREENSHAVER_FXAA_SUBPIXEL_QUALITY = 0.75;

                    float screenshaverFxaaLuminance(vec3 color)
                    {
                        return dot(color, vec3(0.299, 0.587, 0.114));
                    }

                    vec3 screenshaverFxaaRotateHue(vec3 color, float degrees)
                    {
                        float angle = radians(degrees);
                        float cosine = cos(angle);
                        float sine = sin(angle);
                        float y = dot(color, vec3(0.299, 0.587, 0.114));
                        float i = dot(color, vec3(0.596, -0.274, -0.322));
                        float q = dot(color, vec3(0.211, -0.523, 0.312));
                        float rotatedI = i * cosine - q * sine;
                        float rotatedQ = i * sine + q * cosine;
                        return clamp(
                            vec3(
                                y + 0.956 * rotatedI + 0.621 * rotatedQ,
                                y - 0.272 * rotatedI - 0.647 * rotatedQ,
                                y - 1.106 * rotatedI + 1.703 * rotatedQ
                            ),
                            0.0,
                            1.0
                        );
                    }

                    vec3 screenshaverFxaaApplyColorEffects(vec3 color)
                    {
                        if (screenshaverFxaaInvertColors > 0.5)
                            color = vec3(1.0) - color;
                        if (abs(screenshaverFxaaHueRotation) > 0.0001)
                            color = screenshaverFxaaRotateHue(color, screenshaverFxaaHueRotation);
                        return color;
                    }
                `,
                null
            );

            snippet.set_replace(`
                vec2 uv = cogl_tex_coord0_in.st;
                if (screenshaverFxaaFlipHorizontal > 0.5)
                    uv.x = 1.0 - uv.x;
                if (screenshaverFxaaFlipVertical > 0.5)
                    uv.y = 1.0 - uv.y;

                vec4 centerSample = texture2D(cogl_sampler0, uv);
                float lumaCenter = screenshaverFxaaLuminance(centerSample.rgb);
                float lumaNorth = screenshaverFxaaLuminance(texture2D(cogl_sampler0, uv + vec2(0.0, screenshaverFxaaInverseResolution.y)).rgb);
                float lumaSouth = screenshaverFxaaLuminance(texture2D(cogl_sampler0, uv - vec2(0.0, screenshaverFxaaInverseResolution.y)).rgb);
                float lumaEast = screenshaverFxaaLuminance(texture2D(cogl_sampler0, uv + vec2(screenshaverFxaaInverseResolution.x, 0.0)).rgb);
                float lumaWest = screenshaverFxaaLuminance(texture2D(cogl_sampler0, uv - vec2(screenshaverFxaaInverseResolution.x, 0.0)).rgb);
                float lumaMinimum = min(lumaCenter, min(min(lumaNorth, lumaSouth), min(lumaEast, lumaWest)));
                float lumaMaximum = max(lumaCenter, max(max(lumaNorth, lumaSouth), max(lumaEast, lumaWest)));
                float lumaRange = lumaMaximum - lumaMinimum;
                float edgeThreshold = max(SCREENSHAVER_FXAA_EDGE_THRESHOLD_MIN, lumaMaximum * SCREENSHAVER_FXAA_EDGE_THRESHOLD_MAX);

                if (lumaRange < edgeThreshold) {
                    cogl_color_out = vec4(screenshaverFxaaApplyColorEffects(centerSample.rgb), 1.0);
                } else {
                    float lumaNorthWest = screenshaverFxaaLuminance(texture2D(cogl_sampler0, uv + vec2(-screenshaverFxaaInverseResolution.x, screenshaverFxaaInverseResolution.y)).rgb);
                    float lumaNorthEast = screenshaverFxaaLuminance(texture2D(cogl_sampler0, uv + vec2(screenshaverFxaaInverseResolution.x, screenshaverFxaaInverseResolution.y)).rgb);
                    float lumaSouthWest = screenshaverFxaaLuminance(texture2D(cogl_sampler0, uv + vec2(-screenshaverFxaaInverseResolution.x, -screenshaverFxaaInverseResolution.y)).rgb);
                    float lumaSouthEast = screenshaverFxaaLuminance(texture2D(cogl_sampler0, uv + vec2(screenshaverFxaaInverseResolution.x, -screenshaverFxaaInverseResolution.y)).rgb);
                    float horizontalEdge = abs(lumaNorthWest + 2.0 * lumaNorth + lumaNorthEast - 2.0 * lumaCenter)
                        + abs(lumaSouthWest + 2.0 * lumaSouth + lumaSouthEast - 2.0 * lumaCenter);
                    float verticalEdge = abs(lumaNorthWest + 2.0 * lumaWest + lumaSouthWest - 2.0 * lumaCenter)
                        + abs(lumaNorthEast + 2.0 * lumaEast + lumaSouthEast - 2.0 * lumaCenter);
                    bool isHorizontal = horizontalEdge >= verticalEdge;
                    float lumaNegative = isHorizontal ? lumaNorth : lumaWest;
                    float lumaPositive = isHorizontal ? lumaSouth : lumaEast;
                    float gradientNegative = abs(lumaNegative - lumaCenter);
                    float gradientPositive = abs(lumaPositive - lumaCenter);
                    bool useNegativeDirection = gradientNegative >= gradientPositive;
                    float gradient = max(gradientNegative, gradientPositive);
                    vec2 stepDirection = isHorizontal
                        ? vec2(screenshaverFxaaInverseResolution.x, 0.0)
                        : vec2(0.0, screenshaverFxaaInverseResolution.y);
                    vec2 normalDirection = isHorizontal
                        ? vec2(0.0, screenshaverFxaaInverseResolution.y)
                        : vec2(screenshaverFxaaInverseResolution.x, 0.0);
                    if (useNegativeDirection)
                        normalDirection = -normalDirection;
                    float lumaReference = 0.5 * (lumaCenter + (useNegativeDirection ? lumaNegative : lumaPositive));
                    vec2 edgeUv = uv + normalDirection * 0.5;
                    vec2 negativeUv = edgeUv - stepDirection;
                    vec2 positiveUv = edgeUv + stepDirection;
                    float gradientThreshold = gradient * 0.25;
                    float negativeDelta = screenshaverFxaaLuminance(texture2D(cogl_sampler0, negativeUv).rgb) - lumaReference;
                    float positiveDelta = screenshaverFxaaLuminance(texture2D(cogl_sampler0, positiveUv).rgb) - lumaReference;
                    bool negativeReached = abs(negativeDelta) >= gradientThreshold;
                    bool positiveReached = abs(positiveDelta) >= gradientThreshold;
                    for (int i = 0; i < 8; ++i) {
                        if (!negativeReached) {
                            negativeUv -= stepDirection;
                            negativeDelta = screenshaverFxaaLuminance(texture2D(cogl_sampler0, negativeUv).rgb) - lumaReference;
                            negativeReached = abs(negativeDelta) >= gradientThreshold;
                        }
                        if (!positiveReached) {
                            positiveUv += stepDirection;
                            positiveDelta = screenshaverFxaaLuminance(texture2D(cogl_sampler0, positiveUv).rgb) - lumaReference;
                            positiveReached = abs(positiveDelta) >= gradientThreshold;
                        }
                        if (negativeReached && positiveReached)
                            break;
                    }
                    float negativeDistance = isHorizontal ? uv.x - negativeUv.x : uv.y - negativeUv.y;
                    float positiveDistance = isHorizontal ? positiveUv.x - uv.x : positiveUv.y - uv.y;
                    negativeDistance = abs(negativeDistance);
                    positiveDistance = abs(positiveDistance);
                    float nearestDistance = min(negativeDistance, positiveDistance);
                    float totalDistance = max(negativeDistance + positiveDistance, 0.000001);
                    float edgeOffset = 0.5 - nearestDistance / totalDistance;
                    bool nearestIsNegative = negativeDistance < positiveDistance;
                    float nearestDelta = nearestIsNegative ? negativeDelta : positiveDelta;
                    bool centerIsDarker = lumaCenter < lumaReference;
                    bool nearestIsDarker = nearestDelta < 0.0;
                    if (centerIsDarker == nearestIsDarker)
                        edgeOffset = 0.0;
                    float averageLuma = (2.0 * (lumaNorth + lumaSouth + lumaEast + lumaWest)
                        + lumaNorthWest + lumaNorthEast + lumaSouthWest + lumaSouthEast) / 12.0;
                    float subpixelContrast = clamp(abs(averageLuma - lumaCenter) / max(lumaRange, 0.000001), 0.0, 1.0);
                    float subpixelOffset = smoothstep(0.0, 1.0, subpixelContrast);
                    subpixelOffset = subpixelOffset * subpixelOffset * SCREENSHAVER_FXAA_SUBPIXEL_QUALITY;
                    float finalOffset = max(edgeOffset, subpixelOffset);
                    vec2 finalUv = uv + normalDirection * finalOffset;
                    vec4 filteredSample = texture2D(cogl_sampler0, finalUv);
                    cogl_color_out = vec4(screenshaverFxaaApplyColorEffects(filteredSample.rgb), 1.0);
                }
            `);
            pipeline.add_snippet(snippet);

            const inverseResolutionLocation = pipeline.get_uniform_location('screenshaverFxaaInverseResolution');
            pipeline.set_uniform_float(
                inverseResolutionLocation,
                2,
                1,
                [1.0 / width, 1.0 / height]
            );

            this._screenshaverFxaaUniformInvert = pipeline.get_uniform_location('screenshaverFxaaInvertColors');
            this._screenshaverFxaaUniformFlipHorizontal = pipeline.get_uniform_location('screenshaverFxaaFlipHorizontal');
            this._screenshaverFxaaUniformFlipVertical = pipeline.get_uniform_location('screenshaverFxaaFlipVertical');
            this._screenshaverFxaaUniformHueRotation = pipeline.get_uniform_location('screenshaverFxaaHueRotation');

            return pipeline;
        }

        screenshaver_update_fxaa_uniforms() {
            const pipeline = this._screenshaverFxaaPipeline;
            if (!pipeline)
                return;
            pipeline.set_uniform_1f(this._screenshaverFxaaUniformInvert, this._screenshaverFxaaInvertColors ?? 0.0);
            pipeline.set_uniform_1f(this._screenshaverFxaaUniformFlipHorizontal, this._screenshaverFxaaFlipHorizontal ?? 0.0);
            pipeline.set_uniform_1f(this._screenshaverFxaaUniformFlipVertical, this._screenshaverFxaaFlipVertical ?? 0.0);
            pipeline.set_uniform_1f(this._screenshaverFxaaUniformHueRotation, this._screenshaverFxaaHueRotation ?? 0.0);
        }

        screenshaver_create_dithering_pipeline(coglContext, sceneTexture) {
            const pipeline = Cogl.Pipeline.new(coglContext);
            pipeline.set_layer_texture(0, sceneTexture);
            pipeline.set_layer_filters(
                0,
                Cogl.PipelineFilter.LINEAR,
                Cogl.PipelineFilter.LINEAR
            );
            pipeline.set_layer_wrap_mode(
                0,
                Cogl.PipelineWrapMode.CLAMP_TO_EDGE
            );

            const snippet = Cogl.Snippet.new(
                Cogl.SnippetHook.FRAGMENT,
                `
                    uniform float screenshaverDitherStrength;

                    float screenshaverBayer4x4(vec2 fragCoord)
                    {
                        int x = int(mod(floor(fragCoord.x), 4.0));
                        int y = int(mod(floor(fragCoord.y), 4.0));
                        int index = y * 4 + x;

                        if (index == 0) return 0.0 / 16.0;
                        if (index == 1) return 8.0 / 16.0;
                        if (index == 2) return 2.0 / 16.0;
                        if (index == 3) return 10.0 / 16.0;
                        if (index == 4) return 12.0 / 16.0;
                        if (index == 5) return 4.0 / 16.0;
                        if (index == 6) return 14.0 / 16.0;
                        if (index == 7) return 6.0 / 16.0;
                        if (index == 8) return 3.0 / 16.0;
                        if (index == 9) return 11.0 / 16.0;
                        if (index == 10) return 1.0 / 16.0;
                        if (index == 11) return 9.0 / 16.0;
                        if (index == 12) return 15.0 / 16.0;
                        if (index == 13) return 7.0 / 16.0;
                        if (index == 14) return 13.0 / 16.0;
                        return 5.0 / 16.0;
                    }
                `,
                null
            );

            snippet.set_replace(`
                vec4 scene = texture2D(cogl_sampler0, cogl_tex_coord0_in.st);
                float signedThreshold = screenshaverBayer4x4(gl_FragCoord.xy) - 0.5;
                vec3 dithered = clamp(
                    scene.rgb + signedThreshold * screenshaverDitherStrength,
                    0.0,
                    1.0
                );
                cogl_color_out = vec4(dithered, 1.0);
            `);
            pipeline.add_snippet(snippet);

            const strengthLocation = pipeline.get_uniform_location('screenshaverDitherStrength');
            pipeline.set_uniform_1f(strengthLocation, 0.5 / 255.0);

            return pipeline;
        }

        screenshaver_set_audio_bands(bass, midrange, treble) {
            this._screenshaverAudioBass = Number.isFinite(bass) ? Math.max(0.0, Math.min(1.0, bass)) : 0.0;
            this._screenshaverAudioMidrange = Number.isFinite(midrange) ? Math.max(0.0, Math.min(1.0, midrange)) : 0.0;
            this._screenshaverAudioTreble = Number.isFinite(treble) ? Math.max(0.0, Math.min(1.0, treble)) : 0.0;
            this.screenshaver_update_audio_bloom_uniforms();
        }

        screenshaver_create_audio_bloom_extraction_pipeline(coglContext, sceneTexture) {
            const pipeline = Cogl.Pipeline.new(coglContext);
            pipeline.set_layer_texture(0, sceneTexture);
            pipeline.set_layer_filters(
                0,
                Cogl.PipelineFilter.LINEAR,
                Cogl.PipelineFilter.LINEAR
            );
            pipeline.set_layer_wrap_mode(
                0,
                Cogl.PipelineWrapMode.CLAMP_TO_EDGE
            );

            const snippet = Cogl.Snippet.new(
                Cogl.SnippetHook.FRAGMENT,
                `
                    uniform float screenshaverBloomThreshold;
                    uniform vec3 screenshaverAudioBands;

                    vec3 screenshaverBloomRgbToHsv(vec3 c)
                    {
                        float maxChannel = max(c.r, max(c.g, c.b));
                        float minChannel = min(c.r, min(c.g, c.b));
                        float chroma = maxChannel - minChannel;
                        float hue = 0.0;

                        if (chroma > 0.00001) {
                            if (maxChannel == c.r)
                                hue = mod((c.g - c.b) / chroma, 6.0);
                            else if (maxChannel == c.g)
                                hue = ((c.b - c.r) / chroma) + 2.0;
                            else
                                hue = ((c.r - c.g) / chroma) + 4.0;

                            hue *= 60.0;
                            if (hue < 0.0)
                                hue += 360.0;
                        }

                        float saturation = maxChannel > 0.00001
                            ? chroma / maxChannel
                            : 0.0;

                        return vec3(hue, saturation, maxChannel);
                    }
                `,
                null
            );

            snippet.set_replace(`
                vec3 sceneColor = texture2D(cogl_sampler0, cogl_tex_coord0_in.st).rgb;
                vec3 hsv = screenshaverBloomRgbToHsv(max(sceneColor, vec3(0.0)));
                float hue = hsv.x;
                float saturation = hsv.y;
                float value = hsv.z;

                float bassEnergy = clamp(screenshaverAudioBands.x, 0.0, 1.0);
                float midEnergy = clamp(screenshaverAudioBands.y, 0.0, 1.0);
                float highEnergy = clamp(screenshaverAudioBands.z, 0.0, 1.0);

                float bassMatch = (hue >= 0.0 && hue < 45.0) ? bassEnergy : 0.0;
                float midMatch = (hue >= 45.0 && hue < 150.0) ? midEnergy : 0.0;
                float highMatch = (hue >= 240.0 && hue < 300.0) ? highEnergy : 0.0;
                float bandMatch = max(bassMatch, max(midMatch, highMatch));

                float colorStrength = saturation * 2.0
                    * smoothstep(0.02, 0.15, value);
                float energy = clamp(bandMatch, 0.0, 1.0);
                float effectiveThreshold = mix(2.0, screenshaverBloomThreshold, energy);
                float response = energy * smoothstep(
                    effectiveThreshold,
                    min(effectiveThreshold + 0.35, 2.0001),
                    colorStrength
                );

                cogl_color_out = vec4(sceneColor * response, 1.0);
            `);
            pipeline.add_snippet(snippet);

            this._screenshaverBloomThresholdUniform =
                pipeline.get_uniform_location('screenshaverBloomThreshold');
            this._screenshaverAudioBandsUniform =
                pipeline.get_uniform_location('screenshaverAudioBands');

            pipeline.set_uniform_1f(
                this._screenshaverBloomThresholdUniform,
                Number.isFinite(bloomThreshold) ? bloomThreshold : 0.80
            );

            return pipeline;
        }

        screenshaver_update_audio_bloom_uniforms() {
            const pipeline = this._screenshaverAudioBloomExtractionPipeline;
            if (!pipeline || this._screenshaverAudioBandsUniform === undefined)
                return;

            pipeline.set_uniform_float(
                this._screenshaverAudioBandsUniform,
                3,
                1,
                [
                    this._screenshaverAudioBass ?? 0.0,
                    this._screenshaverAudioMidrange ?? 0.0,
                    this._screenshaverAudioTreble ?? 0.0,
                ]
            );
        }

        screenshaver_create_bloom_blur_pipeline(coglContext, sourceTexture, texelStepX, texelStepY) {
            const pipeline = Cogl.Pipeline.new(coglContext);
            pipeline.set_layer_texture(0, sourceTexture);
            pipeline.set_layer_filters(
                0,
                Cogl.PipelineFilter.LINEAR,
                Cogl.PipelineFilter.LINEAR
            );
            pipeline.set_layer_wrap_mode(
                0,
                Cogl.PipelineWrapMode.CLAMP_TO_EDGE
            );

            const snippet = Cogl.Snippet.new(
                Cogl.SnippetHook.FRAGMENT,
                `
                    uniform vec2 screenshaverBloomTexelStep;
                `,
                null
            );

            snippet.set_replace(`
                const float w0 = 0.2270270270;
                const float w1 = 0.1945945946;
                const float w2 = 0.1216216216;
                const float w3 = 0.0540540541;
                const float w4 = 0.0162162162;

                vec2 uv = cogl_tex_coord0_in.st;
                vec3 color = texture2D(cogl_sampler0, uv).rgb * w0;
                color += texture2D(cogl_sampler0, uv + screenshaverBloomTexelStep * 1.0).rgb * w1;
                color += texture2D(cogl_sampler0, uv - screenshaverBloomTexelStep * 1.0).rgb * w1;
                color += texture2D(cogl_sampler0, uv + screenshaverBloomTexelStep * 2.0).rgb * w2;
                color += texture2D(cogl_sampler0, uv - screenshaverBloomTexelStep * 2.0).rgb * w2;
                color += texture2D(cogl_sampler0, uv + screenshaverBloomTexelStep * 3.0).rgb * w3;
                color += texture2D(cogl_sampler0, uv - screenshaverBloomTexelStep * 3.0).rgb * w3;
                color += texture2D(cogl_sampler0, uv + screenshaverBloomTexelStep * 4.0).rgb * w4;
                color += texture2D(cogl_sampler0, uv - screenshaverBloomTexelStep * 4.0).rgb * w4;
                cogl_color_out = vec4(color, 1.0);
            `);
            pipeline.add_snippet(snippet);

            const texelStepLocation =
                pipeline.get_uniform_location('screenshaverBloomTexelStep');
            pipeline.set_uniform_float(
                texelStepLocation,
                2,
                1,
                [texelStepX, texelStepY]
            );

            return pipeline;
        }

        screenshaver_create_audio_bloom_composite_pipeline(coglContext, sceneTexture, bloomTexture) {
            const pipeline = Cogl.Pipeline.new(coglContext);
            pipeline.set_layer_texture(0, sceneTexture);
            pipeline.set_layer_texture(1, bloomTexture);
            pipeline.set_layer_filters(
                0,
                Cogl.PipelineFilter.LINEAR,
                Cogl.PipelineFilter.LINEAR
            );
            pipeline.set_layer_filters(
                1,
                Cogl.PipelineFilter.LINEAR,
                Cogl.PipelineFilter.LINEAR
            );
            pipeline.set_layer_wrap_mode(
                0,
                Cogl.PipelineWrapMode.CLAMP_TO_EDGE
            );
            pipeline.set_layer_wrap_mode(
                1,
                Cogl.PipelineWrapMode.CLAMP_TO_EDGE
            );

            const snippet = Cogl.Snippet.new(
                Cogl.SnippetHook.FRAGMENT,
                `
                    uniform float screenshaverBloomIntensity;
                `,
                null
            );

            snippet.set_replace(`
                vec2 uv = cogl_tex_coord0_in.st;
                vec3 sceneColor = texture2D(cogl_sampler0, uv).rgb;
                vec3 bloomColor = texture2D(cogl_sampler1, uv).rgb;
                cogl_color_out = vec4(
                    sceneColor + bloomColor * screenshaverBloomIntensity,
                    1.0
                );
            `);
            pipeline.add_snippet(snippet);

            const intensityLocation =
                pipeline.get_uniform_location('screenshaverBloomIntensity');
            pipeline.set_uniform_1f(
                intensityLocation,
                Number.isFinite(bloomIntensity) ? bloomIntensity : 1.0
            );

            return pipeline;
        }

        vfunc_build_pipeline() {
            this.add_glsl_snippet(
                Cogl.SnippetHook.FRAGMENT,
                `
                    uniform float iTime;
                    uniform vec3 iResolution;
                    uniform float screenshaverInvertColors;
                    uniform float screenshaverFlipHorizontal;
                    uniform float screenshaverFlipVertical;
                    uniform float screenshaverHueRotation;

                    vec3 screenshaverRotateHue(vec3 color, float degrees)
                    {
                        float angle = radians(degrees);
                        float cosine = cos(angle);
                        float sine = sin(angle);

                        float y = dot(color, vec3(0.299, 0.587, 0.114));
                        float i = dot(color, vec3(0.596, -0.274, -0.322));
                        float q = dot(color, vec3(0.211, -0.523, 0.312));

                        float rotatedI = i * cosine - q * sine;
                        float rotatedQ = i * sine + q * cosine;

                        return clamp(
                            vec3(
                                y + 0.956 * rotatedI + 0.621 * rotatedQ,
                                y - 0.272 * rotatedI - 0.647 * rotatedQ,
                                y - 1.106 * rotatedI + 1.703 * rotatedQ
                            ),
                            0.0,
                            1.0
                        );
                    }

                    ${shaderBody}
                `,
                `
                    vec2 uv = cogl_tex_coord0_in.st;
                    vec2 fragCoord = vec2(
                        uv.x * iResolution.x,
                        (1.0 - uv.y) * iResolution.y
                    );

                    if (screenshaverFlipHorizontal > 0.5)
                        fragCoord.x = iResolution.x - fragCoord.x;

                    if (screenshaverFlipVertical > 0.5)
                        fragCoord.y = iResolution.y - fragCoord.y;

                    vec4 fragColor = vec4(0.0);
                    mainImage(fragColor, fragCoord);

                    if (screenshaverInvertColors > 0.5)
                        fragColor.rgb = vec3(1.0) - fragColor.rgb;

                    if (abs(screenshaverHueRotation) > 0.0001)
                        fragColor.rgb = screenshaverRotateHue(
                            fragColor.rgb,
                            screenshaverHueRotation
                        );

                    // Test #30B: Screenshaver is a fullscreen presentation layer.
                    // Match the normal OpenGL framebuffer semantics by making the
                    // final GNOME/Cogl presentation opaque; shader alpha must not
                    // participate in lock-screen compositing.
                    fragColor.a = 1.0;
                    cogl_color_out = fragColor;
                `,
                true
            );
        }

        // Test #30: keep Shell.GLSLEffect's own target at native size. For
        // Render Scale < 1.0, execute the shader pipeline into a separate
        // reduced-resolution Cogl.Offscreen framebuffer, then present that
        // resulting texture fullscreen with a plain Cogl pipeline. This is the
        // first test that separates shader rasterization resolution from final
        // presentation resolution.
        vfunc_paint_target(node, paintContext) {
            let targetValid = false;
            let targetWidth = 0.0;
            let targetHeight = 0.0;
            let sourceTextureWidth = 0;
            let sourceTextureHeight = 0;

            try {
                const targetSize = this.get_target_size();
                const sourceTexture = this.get_texture();

                if (Array.isArray(targetSize)) {
                    if (targetSize.length >= 3) {
                        targetValid = Boolean(targetSize[0]);
                        targetWidth = Number(targetSize[1]);
                        targetHeight = Number(targetSize[2]);
                    } else if (targetSize.length >= 2) {
                        targetValid = true;
                        targetWidth = Number(targetSize[0]);
                        targetHeight = Number(targetSize[1]);
                    }
                }

                sourceTextureWidth = sourceTexture?.get_width?.() ?? 0;
                sourceTextureHeight = sourceTexture?.get_height?.() ?? 0;

                const nativeWidth = Math.max(1, Math.round(targetWidth));
                const nativeHeight = Math.max(1, Math.round(targetHeight));
                const scale = Number.isFinite(renderScale) && renderScale > 0.0
                    ? renderScale
                    : 1.0;
                const renderWidth = Math.max(1, Math.round(nativeWidth * scale));
                const renderHeight = Math.max(1, Math.round(nativeHeight * scale));

                if (!this._screenshaverOffscreenProbeLogged) {
                    console.log(
                        `[Screenshaver] Test #30 native Shell.GLSLEffect target: ` +
                        `valid=${targetValid} target=${targetWidth}x${targetHeight} ` +
                        `texture=${sourceTextureWidth}x${sourceTextureHeight} ` +
                        `generation=${generation}`
                    );
                    this._screenshaverOffscreenProbeLogged = true;
                }

                if (!targetValid || !(targetWidth > 0.0) || !(targetHeight > 0.0))
                    throw new Error(`invalid native target ${targetWidth}x${targetHeight}`);
                if (!sourceTexture)
                    throw new Error('Shell.GLSLEffect source texture unavailable');

                // GNOME Shell/Cogl API compatibility: newer Shell builds may
                // expose the Cogl.Context directly from the effect texture, while
                // GNOME 46 does not. Prefer the texture-owned context when it is
                // available, then fall back to the Clutter stage/backend context.
                // The latter is the public extension-side route used by GNOME 45+.
                let coglContext = sourceTexture?.get_context?.() ?? null;

                if (!coglContext) {
                    const stageContext =
                        global.stage?.get_context?.() ??
                        global.stage?.context ??
                        null;
                    const clutterBackend = stageContext?.get_backend?.() ?? null;
                    coglContext = clutterBackend?.get_cogl_context?.() ?? null;
                }

                if (!coglContext) {
                    // GNOME 46 diagnostic only:
                    //
                    // Probe the Clutter context attached to the actual effect actor.
                    // GNOME's newer extension guidance prefers actor.get_context()
                    // over a process-global/default backend.  Do NOT use the
                    // returned Cogl.Context yet: the previous default-backend
                    // experiment proved that allocating Cogl resources from an
                    // unsuitable context can crash GNOME Shell.
                    const effectActor =
                        this.get_actor?.() ??
                        this.actor ??
                        null;

                    const actorContext =
                        effectActor?.get_context?.() ??
                        effectActor?.context ??
                        null;

                    const actorBackend =
                        actorContext?.get_backend?.() ??
                        null;

                    const actorCoglContext =
                        actorBackend?.get_cogl_context?.() ??
                        null;

                    if (!this._screenshaverActorCoglContextProbeLogged) {
                        console.log(
                            `[Screenshaver] Test #30C GNOME actor-context probe: ` +
                            `actor=${effectActor !== null} ` +
                            `clutter-context=${actorContext !== null} ` +
                            `backend=${actorBackend !== null} ` +
                            `cogl-context=${actorCoglContext !== null} ` +
                            `generation=${generation}`
                        );
                        this._screenshaverActorCoglContextProbeLogged = true;
                    }
                }

                if (!coglContext) {
                    // GNOME 46 diagnostic only:
                    //
                    // Clutter's paint callback is already executing with the
                    // framebuffer that owns this render operation.  Probe that
                    // ownership chain directly:
                    //
                    //   Clutter.PaintContext
                    //       -> Cogl.Framebuffer
                    //       -> Cogl.Context
                    //
                    // Do not allocate or render through this context yet.
                    const paintFramebuffer =
                        paintContext?.get_framebuffer?.() ??
                        null;

                    const paintCoglContext =
                        paintFramebuffer?.get_context?.() ??
                        null;

                    if (!this._screenshaverPaintFramebufferProbeLogged) {
                        console.log(
                            `[Screenshaver] Test #30D GNOME paint-framebuffer probe: ` +
                            `paint-context=${paintContext !== null && paintContext !== undefined} ` +
                            `get-framebuffer=${typeof paintContext?.get_framebuffer === 'function'} ` +
                            `framebuffer=${paintFramebuffer !== null} ` +
                            `get-context=${typeof paintFramebuffer?.get_context === 'function'} ` +
                            `cogl-context=${paintCoglContext !== null} ` +
                            `generation=${generation}`
                        );
                        this._screenshaverPaintFramebufferProbeLogged = true;
                    }

                    // Test #30E: create one tiny texture from the Cogl.Context
                    // owned by the framebuffer currently being painted.  This is
                    // intentionally diagnostic-only: no offscreen framebuffer is
                    // created, no shader is rendered into the texture, and the
                    // production Test #30 path still falls back afterward.
                    if (paintCoglContext &&
                        !this._screenshaverPaintContextTextureProbeAttempted) {
                        this._screenshaverPaintContextTextureProbeAttempted = true;

                        try {
                            const probeTexture =
                                Cogl.Texture2D.new_with_format(
                                    paintCoglContext,
                                    4,
                                    4,
                                    Cogl.PixelFormat.RGBA_8888
                                );

                            console.log(
                                `[Screenshaver] Test #30E GNOME paint-context texture probe: ` +
                                `created=${probeTexture !== null} size=4x4 ` +
                                `generation=${generation}`
                            );

                            // Test #30F: create one tiny offscreen framebuffer
                            // backed by the texture created from the paint-owned
                            // Cogl.Context.  Do not draw into it yet.
                            const probeOffscreen =
                                Cogl.Offscreen.new_with_texture(
                                    probeTexture
                                );

                            console.log(
                                `[Screenshaver] Test #30F GNOME paint-context offscreen probe: ` +
                                `created=${probeOffscreen !== null} size=4x4 ` +
                                `generation=${generation}`
                            );

                            // Test #30G: exercise the smallest production-style
                            // framebuffer operation sequence using the Cogl
                            // context owned by the framebuffer currently being
                            // painted.  No pipeline or shader is attached and
                            // nothing is presented to the lock-screen actor.
                            probeTexture.set_premultiplied(false);
                            probeTexture.allocate();

                            probeOffscreen.allocate();
                            probeOffscreen.set_viewport(
                                0.0,
                                0.0,
                                4.0,
                                4.0
                            );

                            probeOffscreen.clear4f(
                                Cogl.BufferBit.COLOR,
                                0.0,
                                0.0,
                                0.0,
                                1.0
                            );

                            probeOffscreen.flush();

                            console.log(
                                `[Screenshaver] Test #30G GNOME paint-context clear probe: ` +
                                `allocated=true cleared=true flushed=true size=4x4 ` +
                                `generation=${generation}`
                            );

                            // Test #30H: exercise the same textured-rectangle draw
                            // primitive used by the production Test #30 pipeline,
                            // but only with disposable 4x4 resources.  The source
                            // and destination textures are separate to avoid a
                            // feedback loop.  Nothing from this probe is attached
                            // to the lock-screen presentation node.
                            const probeSourceTexture =
                                Cogl.Texture2D.new_with_format(
                                    paintCoglContext,
                                    4,
                                    4,
                                    Cogl.PixelFormat.RGBA_8888
                                );
                            probeSourceTexture.set_premultiplied(false);
                            probeSourceTexture.allocate();

                            const probeDestinationTexture =
                                Cogl.Texture2D.new_with_format(
                                    paintCoglContext,
                                    4,
                                    4,
                                    Cogl.PixelFormat.RGBA_8888
                                );
                            probeDestinationTexture.set_premultiplied(false);
                            probeDestinationTexture.allocate();

                            const probeDestinationOffscreen =
                                Cogl.Offscreen.new_with_texture(
                                    probeDestinationTexture
                                );
                            probeDestinationOffscreen.allocate();
                            probeDestinationOffscreen.set_viewport(
                                0.0,
                                0.0,
                                4.0,
                                4.0
                            );

                            const probePipeline =
                                Cogl.Pipeline.new(paintCoglContext);
                            probePipeline.set_layer_texture(
                                0,
                                probeSourceTexture
                            );
                            probePipeline.set_layer_filters(
                                0,
                                Cogl.PipelineFilter.LINEAR,
                                Cogl.PipelineFilter.LINEAR
                            );

                            probeDestinationOffscreen.clear4f(
                                Cogl.BufferBit.COLOR,
                                0.0,
                                0.0,
                                0.0,
                                1.0
                            );

                            probeDestinationOffscreen.draw_textured_rectangle(
                                probePipeline,
                                -1.0,
                                1.0,
                                1.0,
                                -1.0,
                                0.0,
                                0.0,
                                1.0,
                                1.0
                            );

                            probeDestinationOffscreen.flush();

                            console.log(
                                `[Screenshaver] Test #30H GNOME paint-context draw probe: ` +
                                `pipeline=true textured-rectangle=true flushed=true size=4x4 ` +
                                `generation=${generation}`
                            );


                            // Test #30I: isolate production dimensions from
                            // production pixel precision.  Allocate a real-size
                            // STANDARD RGBA_8888 target only; do not use FP16,
                            // do not run the production shader, and do not
                            // present this texture.
                            const fullSizeStandardTexture =
                                Cogl.Texture2D.new_with_format(
                                    paintCoglContext,
                                    renderWidth,
                                    renderHeight,
                                    Cogl.PixelFormat.RGBA_8888
                                );
                            fullSizeStandardTexture.set_premultiplied(false);
                            fullSizeStandardTexture.allocate();

                            const fullSizeStandardOffscreen =
                                Cogl.Offscreen.new_with_texture(
                                    fullSizeStandardTexture
                                );
                            fullSizeStandardOffscreen.allocate();
                            fullSizeStandardOffscreen.set_viewport(
                                0.0,
                                0.0,
                                renderWidth,
                                renderHeight
                            );

                            fullSizeStandardOffscreen.clear4f(
                                Cogl.BufferBit.COLOR,
                                0.0,
                                0.0,
                                0.0,
                                1.0
                            );
                            fullSizeStandardOffscreen.flush();

                            console.log(
                                `[Screenshaver] Test #30I GNOME full-size standard target probe: ` +
                                `allocated=true cleared=true flushed=true ` +
                                `size=${renderWidth}x${renderHeight} format=RGBA_8888 ` +
                                `generation=${generation}`
                            );


                            // Test #30K: add only the next production operation
                            // after the already-safe full-size target allocation:
                            // create a plain Cogl.Pipeline and bind the full-size
                            // RGBA_8888 texture to layer 0.  Do not draw with it,
                            // do not attach any shader snippet, and do not present
                            // it to the lock-screen actor.
                            const fullSizeStandardPipeline =
                                Cogl.Pipeline.new(paintCoglContext);
                            fullSizeStandardPipeline.set_layer_texture(
                                0,
                                fullSizeStandardTexture
                            );
                            fullSizeStandardPipeline.set_layer_filters(
                                0,
                                Cogl.PipelineFilter.LINEAR,
                                Cogl.PipelineFilter.LINEAR
                            );

                            console.log(
                                `[Screenshaver] Test #30K GNOME full-size pipeline probe: ` +
                                `pipeline=true texture-bound=true filters=true ` +
                                `size=${renderWidth}x${renderHeight} format=RGBA_8888 ` +
                                `generation=${generation}`
                            );


                            // Test #30L: perform one full-size textured draw
                            // using only disposable RGBA_8888 resources created
                            // from the validated PaintContext-owned Cogl.Context.
                            // This still does NOT use the Shell.GLSLEffect source
                            // texture and does not present anything onscreen.
                            const fullSizeDrawSourceTexture =
                                Cogl.Texture2D.new_with_format(
                                    paintCoglContext,
                                    renderWidth,
                                    renderHeight,
                                    Cogl.PixelFormat.RGBA_8888
                                );
                            fullSizeDrawSourceTexture.set_premultiplied(false);
                            fullSizeDrawSourceTexture.allocate();

                            const fullSizeDrawDestinationTexture =
                                Cogl.Texture2D.new_with_format(
                                    paintCoglContext,
                                    renderWidth,
                                    renderHeight,
                                    Cogl.PixelFormat.RGBA_8888
                                );
                            fullSizeDrawDestinationTexture.set_premultiplied(false);
                            fullSizeDrawDestinationTexture.allocate();

                            const fullSizeDrawOffscreen =
                                Cogl.Offscreen.new_with_texture(
                                    fullSizeDrawDestinationTexture
                                );
                            fullSizeDrawOffscreen.allocate();
                            fullSizeDrawOffscreen.set_viewport(
                                0.0,
                                0.0,
                                renderWidth,
                                renderHeight
                            );

                            const fullSizeDrawPipeline =
                                Cogl.Pipeline.new(paintCoglContext);
                            fullSizeDrawPipeline.set_layer_texture(
                                0,
                                fullSizeDrawSourceTexture
                            );
                            fullSizeDrawPipeline.set_layer_filters(
                                0,
                                Cogl.PipelineFilter.LINEAR,
                                Cogl.PipelineFilter.LINEAR
                            );

                            fullSizeDrawOffscreen.clear4f(
                                Cogl.BufferBit.COLOR,
                                0.0,
                                0.0,
                                0.0,
                                1.0
                            );

                            fullSizeDrawOffscreen.draw_textured_rectangle(
                                fullSizeDrawPipeline,
                                -1.0,
                                1.0,
                                1.0,
                                -1.0,
                                0.0,
                                0.0,
                                1.0,
                                1.0
                            );

                            fullSizeDrawOffscreen.flush();

                            console.log(
                                `[Screenshaver] Test #30L GNOME full-size draw probe: ` +
                                `drawn=true flushed=true ` +
                                `size=${renderWidth}x${renderHeight} format=RGBA_8888 ` +
                                `generation=${generation}`
                            );


                            // Test #30M: isolate the actual Shell.GLSLEffect
                            // source texture.  Bind it to a plain pipeline using
                            // the validated PaintContext-owned Cogl.Context, but
                            // DO NOT draw with it.  This distinguishes a source-
                            // texture/context ownership incompatibility from the
                            // draw operation itself.
                            const shellSourcePipeline =
                                Cogl.Pipeline.new(paintCoglContext);
                            shellSourcePipeline.set_layer_texture(
                                0,
                                sourceTexture
                            );
                            shellSourcePipeline.set_layer_filters(
                                0,
                                Cogl.PipelineFilter.LINEAR,
                                Cogl.PipelineFilter.LINEAR
                            );

                            console.log(
                                `[Screenshaver] Test #30M GNOME Shell source-texture bind probe: ` +
                                `pipeline=true source-texture-bound=true filters=true ` +
                                `source=${sourceTexture.get_width?.() ?? 0}x` +
                                `${sourceTexture.get_height?.() ?? 0} ` +
                                `generation=${generation}`
                            );


                            // Test #30N: perform one textured draw using the
                            // actual Shell.GLSLEffect source texture as input,
                            // but render only into a disposable RGBA_8888
                            // offscreen target owned by the validated
                            // PaintContext Cogl.Context.  Do not attach shader
                            // snippets and do not present the result onscreen.
                            const shellSourceDrawDestinationTexture =
                                Cogl.Texture2D.new_with_format(
                                    paintCoglContext,
                                    renderWidth,
                                    renderHeight,
                                    Cogl.PixelFormat.RGBA_8888
                                );
                            shellSourceDrawDestinationTexture.set_premultiplied(false);
                            shellSourceDrawDestinationTexture.allocate();

                            const shellSourceDrawOffscreen =
                                Cogl.Offscreen.new_with_texture(
                                    shellSourceDrawDestinationTexture
                                );
                            shellSourceDrawOffscreen.allocate();
                            shellSourceDrawOffscreen.set_viewport(
                                0.0,
                                0.0,
                                renderWidth,
                                renderHeight
                            );

                            shellSourceDrawOffscreen.clear4f(
                                Cogl.BufferBit.COLOR,
                                0.0,
                                0.0,
                                0.0,
                                1.0
                            );

                            shellSourceDrawOffscreen.draw_textured_rectangle(
                                shellSourcePipeline,
                                -1.0,
                                1.0,
                                1.0,
                                -1.0,
                                0.0,
                                0.0,
                                1.0,
                                1.0
                            );

                            shellSourceDrawOffscreen.flush();

                            console.log(
                                `[Screenshaver] Test #30N GNOME Shell source-texture draw probe: ` +
                                `drawn=true flushed=true ` +
                                `source=${sourceTexture.get_width?.() ?? 0}x` +
                                `${sourceTexture.get_height?.() ?? 0} ` +
                                `target=${renderWidth}x${renderHeight} ` +
                                `generation=${generation}`
                            );


                            // Test #30O: isolate Cogl shader-snippet creation
                            // and attachment.  This deliberately uses a tiny
                            // no-op fragment snippet and DOES NOT draw with the
                            // pipeline, so shader execution remains outside
                            // this probe.
                            const screenshaverSnippetProbePipeline =
                                Cogl.Pipeline.new(paintCoglContext);

                            const screenshaverProbeSnippet =
                                Cogl.Snippet.new(
                                    Cogl.SnippetHook.FRAGMENT,
                                    null,
                                    null
                                );

                            screenshaverProbeSnippet.set_replace(`
                                cogl_color_out = cogl_color_in;
                            `);

                            screenshaverSnippetProbePipeline.add_snippet(
                                screenshaverProbeSnippet
                            );

                            console.log(
                                `[Screenshaver] Test #30O GNOME shader-snippet probe: ` +
                                `pipeline=true snippet-created=true snippet-attached=true ` +
                                `generation=${generation}`
                            );


                            // Test #30P: execute the already-created minimal
                            // no-op snippet pipeline against disposable
                            // PaintContext-owned resources only.  This isolates
                            // snippet execution from the production postprocess
                            // shader code and from Shell.GLSLEffect presentation.
                            const snippetDrawSourceTexture =
                                Cogl.Texture2D.new_with_format(
                                    paintCoglContext,
                                    4,
                                    4,
                                    Cogl.PixelFormat.RGBA_8888
                                );
                            snippetDrawSourceTexture.set_premultiplied(false);
                            snippetDrawSourceTexture.allocate();

                            const snippetDrawDestinationTexture =
                                Cogl.Texture2D.new_with_format(
                                    paintCoglContext,
                                    4,
                                    4,
                                    Cogl.PixelFormat.RGBA_8888
                                );
                            snippetDrawDestinationTexture.set_premultiplied(false);
                            snippetDrawDestinationTexture.allocate();

                            const snippetDrawOffscreen =
                                Cogl.Offscreen.new_with_texture(
                                    snippetDrawDestinationTexture
                                );
                            snippetDrawOffscreen.allocate();
                            snippetDrawOffscreen.set_viewport(
                                0.0,
                                0.0,
                                4.0,
                                4.0
                            );

                            screenshaverSnippetProbePipeline.set_layer_texture(
                                0,
                                snippetDrawSourceTexture
                            );
                            screenshaverSnippetProbePipeline.set_layer_filters(
                                0,
                                Cogl.PipelineFilter.LINEAR,
                                Cogl.PipelineFilter.LINEAR
                            );

                            snippetDrawOffscreen.clear4f(
                                Cogl.BufferBit.COLOR,
                                0.0,
                                0.0,
                                0.0,
                                1.0
                            );

                            snippetDrawOffscreen.draw_textured_rectangle(
                                screenshaverSnippetProbePipeline,
                                -1.0,
                                1.0,
                                1.0,
                                -1.0,
                                0.0,
                                0.0,
                                1.0,
                                1.0
                            );

                            snippetDrawOffscreen.flush();

                            console.log(
                                `[Screenshaver] Test #30P GNOME shader-snippet draw probe: ` +
                                `drawn=true flushed=true size=4x4 ` +
                                `generation=${generation}`
                            );


                            // Test #30R: isolate the first previously untested
                            // operation inside the production FXAA constructor:
                            // setting CLAMP_TO_EDGE wrap mode on layer 0.
                            // Pipeline creation, texture binding, and LINEAR
                            // filtering have already been proven safe.
                            const screenshaverWrapProbePipeline =
                                Cogl.Pipeline.new(paintCoglContext);

                            screenshaverWrapProbePipeline.set_layer_texture(
                                0,
                                fullSizeStandardTexture
                            );
                            screenshaverWrapProbePipeline.set_layer_filters(
                                0,
                                Cogl.PipelineFilter.LINEAR,
                                Cogl.PipelineFilter.LINEAR
                            );

                            console.log(
                                `[Screenshaver] Test #30R GNOME wrap-mode probe: ` +
                                `before-set-layer-wrap-mode=true ` +
                                `generation=${generation}`
                            );

                            screenshaverWrapProbePipeline.set_layer_wrap_mode(
                                0,
                                Cogl.PipelineWrapMode.CLAMP_TO_EDGE
                            );

                            console.log(
                                `[Screenshaver] Test #30R GNOME wrap-mode probe: ` +
                                `after-set-layer-wrap-mode=true mode=CLAMP_TO_EDGE ` +
                                `generation=${generation}`
                            );


                            // Test #30S: split the production FXAA snippet path
                            // into explicit checkpoints. This uses the exact
                            // declaration and replacement GLSL from
                            // screenshaver_create_fxaa_pipeline(), but does not
                            // query/set uniforms and does not draw.
                            const screenshaverFxaaStagePipeline =
                                Cogl.Pipeline.new(paintCoglContext);
                            screenshaverFxaaStagePipeline.set_layer_texture(
                                0,
                                fullSizeStandardTexture
                            );
                            screenshaverFxaaStagePipeline.set_layer_filters(
                                0,
                                Cogl.PipelineFilter.LINEAR,
                                Cogl.PipelineFilter.LINEAR
                            );
                            screenshaverFxaaStagePipeline.set_layer_wrap_mode(
                                0,
                                Cogl.PipelineWrapMode.CLAMP_TO_EDGE
                            );

                            console.log(
                                `[Screenshaver] Test #30S GNOME FXAA snippet stage: ` +
                                `before-snippet-new=true generation=${generation}`
                            );

                            const screenshaverFxaaStageSnippet =
                                Cogl.Snippet.new(
                                    Cogl.SnippetHook.FRAGMENT,
                                    `
                    uniform vec2 screenshaverFxaaInverseResolution;
                    uniform float screenshaverFxaaInvertColors;
                    uniform float screenshaverFxaaFlipHorizontal;
                    uniform float screenshaverFxaaFlipVertical;
                    uniform float screenshaverFxaaHueRotation;

                    const float SCREENSHAVER_FXAA_EDGE_THRESHOLD_MIN = 0.0312;
                    const float SCREENSHAVER_FXAA_EDGE_THRESHOLD_MAX = 0.125;
                    const float SCREENSHAVER_FXAA_SUBPIXEL_QUALITY = 0.75;

                    float screenshaverFxaaLuminance(vec3 color)
                    {
                        return dot(color, vec3(0.299, 0.587, 0.114));
                    }

                    vec3 screenshaverFxaaRotateHue(vec3 color, float degrees)
                    {
                        float angle = radians(degrees);
                        float cosine = cos(angle);
                        float sine = sin(angle);
                        float y = dot(color, vec3(0.299, 0.587, 0.114));
                        float i = dot(color, vec3(0.596, -0.274, -0.322));
                        float q = dot(color, vec3(0.211, -0.523, 0.312));
                        float rotatedI = i * cosine - q * sine;
                        float rotatedQ = i * sine + q * cosine;
                        return clamp(
                            vec3(
                                y + 0.956 * rotatedI + 0.621 * rotatedQ,
                                y - 0.272 * rotatedI - 0.647 * rotatedQ,
                                y - 1.106 * rotatedI + 1.703 * rotatedQ
                            ),
                            0.0,
                            1.0
                        );
                    }

                    vec3 screenshaverFxaaApplyColorEffects(vec3 color)
                    {
                        if (screenshaverFxaaInvertColors > 0.5)
                            color = vec3(1.0) - color;
                        if (abs(screenshaverFxaaHueRotation) > 0.0001)
                            color = screenshaverFxaaRotateHue(color, screenshaverFxaaHueRotation);
                        return color;
                    }
                `,
                                    null
                                );

                            console.log(
                                `[Screenshaver] Test #30S GNOME FXAA snippet stage: ` +
                                `after-snippet-new=true generation=${generation}`
                            );

                            screenshaverFxaaStageSnippet.set_replace(
                                `
                vec2 uv = cogl_tex_coord0_in.st;
                if (screenshaverFxaaFlipHorizontal > 0.5)
                    uv.x = 1.0 - uv.x;
                if (screenshaverFxaaFlipVertical > 0.5)
                    uv.y = 1.0 - uv.y;

                vec4 centerSample = texture2D(cogl_sampler0, uv);
                float lumaCenter = screenshaverFxaaLuminance(centerSample.rgb);
                float lumaNorth = screenshaverFxaaLuminance(texture2D(cogl_sampler0, uv + vec2(0.0, screenshaverFxaaInverseResolution.y)).rgb);
                float lumaSouth = screenshaverFxaaLuminance(texture2D(cogl_sampler0, uv - vec2(0.0, screenshaverFxaaInverseResolution.y)).rgb);
                float lumaEast = screenshaverFxaaLuminance(texture2D(cogl_sampler0, uv + vec2(screenshaverFxaaInverseResolution.x, 0.0)).rgb);
                float lumaWest = screenshaverFxaaLuminance(texture2D(cogl_sampler0, uv - vec2(screenshaverFxaaInverseResolution.x, 0.0)).rgb);
                float lumaMinimum = min(lumaCenter, min(min(lumaNorth, lumaSouth), min(lumaEast, lumaWest)));
                float lumaMaximum = max(lumaCenter, max(max(lumaNorth, lumaSouth), max(lumaEast, lumaWest)));
                float lumaRange = lumaMaximum - lumaMinimum;
                float edgeThreshold = max(SCREENSHAVER_FXAA_EDGE_THRESHOLD_MIN, lumaMaximum * SCREENSHAVER_FXAA_EDGE_THRESHOLD_MAX);

                if (lumaRange < edgeThreshold) {
                    cogl_color_out = vec4(screenshaverFxaaApplyColorEffects(centerSample.rgb), 1.0);
                } else {
                    float lumaNorthWest = screenshaverFxaaLuminance(texture2D(cogl_sampler0, uv + vec2(-screenshaverFxaaInverseResolution.x, screenshaverFxaaInverseResolution.y)).rgb);
                    float lumaNorthEast = screenshaverFxaaLuminance(texture2D(cogl_sampler0, uv + vec2(screenshaverFxaaInverseResolution.x, screenshaverFxaaInverseResolution.y)).rgb);
                    float lumaSouthWest = screenshaverFxaaLuminance(texture2D(cogl_sampler0, uv + vec2(-screenshaverFxaaInverseResolution.x, -screenshaverFxaaInverseResolution.y)).rgb);
                    float lumaSouthEast = screenshaverFxaaLuminance(texture2D(cogl_sampler0, uv + vec2(screenshaverFxaaInverseResolution.x, -screenshaverFxaaInverseResolution.y)).rgb);
                    float horizontalEdge = abs(lumaNorthWest + 2.0 * lumaNorth + lumaNorthEast - 2.0 * lumaCenter)
                        + abs(lumaSouthWest + 2.0 * lumaSouth + lumaSouthEast - 2.0 * lumaCenter);
                    float verticalEdge = abs(lumaNorthWest + 2.0 * lumaWest + lumaSouthWest - 2.0 * lumaCenter)
                        + abs(lumaNorthEast + 2.0 * lumaEast + lumaSouthEast - 2.0 * lumaCenter);
                    bool isHorizontal = horizontalEdge >= verticalEdge;
                    float lumaNegative = isHorizontal ? lumaNorth : lumaWest;
                    float lumaPositive = isHorizontal ? lumaSouth : lumaEast;
                    float gradientNegative = abs(lumaNegative - lumaCenter);
                    float gradientPositive = abs(lumaPositive - lumaCenter);
                    bool useNegativeDirection = gradientNegative >= gradientPositive;
                    float gradient = max(gradientNegative, gradientPositive);
                    vec2 stepDirection = isHorizontal
                        ? vec2(screenshaverFxaaInverseResolution.x, 0.0)
                        : vec2(0.0, screenshaverFxaaInverseResolution.y);
                    vec2 normalDirection = isHorizontal
                        ? vec2(0.0, screenshaverFxaaInverseResolution.y)
                        : vec2(screenshaverFxaaInverseResolution.x, 0.0);
                    if (useNegativeDirection)
                        normalDirection = -normalDirection;
                    float lumaReference = 0.5 * (lumaCenter + (useNegativeDirection ? lumaNegative : lumaPositive));
                    vec2 edgeUv = uv + normalDirection * 0.5;
                    vec2 negativeUv = edgeUv - stepDirection;
                    vec2 positiveUv = edgeUv + stepDirection;
                    float gradientThreshold = gradient * 0.25;
                    float negativeDelta = screenshaverFxaaLuminance(texture2D(cogl_sampler0, negativeUv).rgb) - lumaReference;
                    float positiveDelta = screenshaverFxaaLuminance(texture2D(cogl_sampler0, positiveUv).rgb) - lumaReference;
                    bool negativeReached = abs(negativeDelta) >= gradientThreshold;
                    bool positiveReached = abs(positiveDelta) >= gradientThreshold;
                    for (int i = 0; i < 8; ++i) {
                        if (!negativeReached) {
                            negativeUv -= stepDirection;
                            negativeDelta = screenshaverFxaaLuminance(texture2D(cogl_sampler0, negativeUv).rgb) - lumaReference;
                            negativeReached = abs(negativeDelta) >= gradientThreshold;
                        }
                        if (!positiveReached) {
                            positiveUv += stepDirection;
                            positiveDelta = screenshaverFxaaLuminance(texture2D(cogl_sampler0, positiveUv).rgb) - lumaReference;
                            positiveReached = abs(positiveDelta) >= gradientThreshold;
                        }
                        if (negativeReached && positiveReached)
                            break;
                    }
                    float negativeDistance = isHorizontal ? uv.x - negativeUv.x : uv.y - negativeUv.y;
                    float positiveDistance = isHorizontal ? positiveUv.x - uv.x : positiveUv.y - uv.y;
                    negativeDistance = abs(negativeDistance);
                    positiveDistance = abs(positiveDistance);
                    float nearestDistance = min(negativeDistance, positiveDistance);
                    float totalDistance = max(negativeDistance + positiveDistance, 0.000001);
                    float edgeOffset = 0.5 - nearestDistance / totalDistance;
                    bool nearestIsNegative = negativeDistance < positiveDistance;
                    float nearestDelta = nearestIsNegative ? negativeDelta : positiveDelta;
                    bool centerIsDarker = lumaCenter < lumaReference;
                    bool nearestIsDarker = nearestDelta < 0.0;
                    if (centerIsDarker == nearestIsDarker)
                        edgeOffset = 0.0;
                    float averageLuma = (2.0 * (lumaNorth + lumaSouth + lumaEast + lumaWest)
                        + lumaNorthWest + lumaNorthEast + lumaSouthWest + lumaSouthEast) / 12.0;
                    float subpixelContrast = clamp(abs(averageLuma - lumaCenter) / max(lumaRange, 0.000001), 0.0, 1.0);
                    float subpixelOffset = smoothstep(0.0, 1.0, subpixelContrast);
                    subpixelOffset = subpixelOffset * subpixelOffset * SCREENSHAVER_FXAA_SUBPIXEL_QUALITY;
                    float finalOffset = max(edgeOffset, subpixelOffset);
                    vec2 finalUv = uv + normalDirection * finalOffset;
                    vec4 filteredSample = texture2D(cogl_sampler0, finalUv);
                    cogl_color_out = vec4(screenshaverFxaaApplyColorEffects(filteredSample.rgb), 1.0);
                }
            `
                            );

                            console.log(
                                `[Screenshaver] Test #30S GNOME FXAA snippet stage: ` +
                                `after-set-replace=true generation=${generation}`
                            );

                            screenshaverFxaaStagePipeline.add_snippet(
                                screenshaverFxaaStageSnippet
                            );

                            console.log(
                                `[Screenshaver] Test #30S GNOME FXAA snippet stage: ` +
                                `after-add-snippet=true generation=${generation}`
                            );


                            // Test #30U: avoid Cogl.Pipeline.set_uniform_float()
                            // array marshalling entirely.  Query the same FXAA
                            // vec2 uniform, then use the fixed-arity convenience
                            // API set_uniform_2f() if GNOME 46 exposes it.
                            const screenshaverFxaa2fLocation =
                                screenshaverFxaaStagePipeline.get_uniform_location(
                                    'screenshaverFxaaInverseResolution'
                                );

                            console.log(
                                `[Screenshaver] Test #30U GNOME FXAA uniform-2f probe: ` +
                                `location=${screenshaverFxaa2fLocation} ` +
                                `set-uniform-2f-type=${typeof screenshaverFxaaStagePipeline.set_uniform_2f} ` +
                                `generation=${generation}`
                            );

                            if (typeof screenshaverFxaaStagePipeline.set_uniform_2f === 'function') {
                                console.log(
                                    `[Screenshaver] Test #30U GNOME FXAA uniform-2f probe: ` +
                                    `before-set-uniform-2f=true generation=${generation}`
                                );

                                screenshaverFxaaStagePipeline.set_uniform_2f(
                                    screenshaverFxaa2fLocation,
                                    1.0 / renderWidth,
                                    1.0 / renderHeight
                                );

                                console.log(
                                    `[Screenshaver] Test #30U GNOME FXAA uniform-2f probe: ` +
                                    `after-set-uniform-2f=true generation=${generation}`
                                );
                            } else {
                                console.log(
                                    `[Screenshaver] Test #30U GNOME FXAA uniform-2f probe: ` +
                                    `set-uniform-2f-unavailable=true generation=${generation}`
                                );
                            }
                        } catch (error) {
                            console.log(
                                `[Screenshaver] Test #30E GNOME paint-context texture probe failed: ` +
                                `${error} generation=${generation}`
                            );
                        }
                    }
                }

                if (!coglContext)
                    throw new Error('Cogl context unavailable from Shell.GLSLEffect texture or Clutter stage backend');

                const requestedPrecision = ['standard', 'high', 'auto'].includes(colorPrecision)
                    ? colorPrecision
                    : 'auto';

                const needsRenderTarget =
                    !this._screenshaverRenderTexture ||
                    !this._screenshaverRenderOffscreen ||
                    this._screenshaverRenderWidth !== renderWidth ||
                    this._screenshaverRenderHeight !== renderHeight ||
                    this._screenshaverRequestedPrecision !== requestedPrecision;

                if (needsRenderTarget) {
                    const allocateTarget = format => {
                        const renderTexture = Cogl.Texture2D.new_with_format(
                            coglContext,
                            renderWidth,
                            renderHeight,
                            format
                        );
                        renderTexture.set_premultiplied(false);
                        renderTexture.allocate();

                        const renderOffscreen = Cogl.Offscreen.new_with_texture(renderTexture);
                        renderOffscreen.allocate();
                        renderOffscreen.set_viewport(
                            0.0,
                            0.0,
                            renderWidth,
                            renderHeight
                        );

                        return [renderTexture, renderOffscreen];
                    };

                    let selectedPrecision;
                    let selectedFormat;
                    let renderTexture;
                    let renderOffscreen;
                    let fellBack = false;
                    let fallbackReason = '';

                    if (requestedPrecision === 'standard') {
                        selectedPrecision = 'standard';
                        selectedFormat = Cogl.PixelFormat.RGBA_8888;
                        [renderTexture, renderOffscreen] = allocateTarget(selectedFormat);
                    } else if (requestedPrecision === 'high') {
                        selectedPrecision = 'high';
                        selectedFormat = Cogl.PixelFormat.RGBA_FP_16161616;
                        [renderTexture, renderOffscreen] = allocateTarget(selectedFormat);
                    } else {
                        try {
                            selectedPrecision = 'high';
                            selectedFormat = Cogl.PixelFormat.RGBA_FP_16161616;
                            [renderTexture, renderOffscreen] = allocateTarget(selectedFormat);
                        } catch (highError) {
                            fellBack = true;
                            fallbackReason = String(highError);
                            selectedPrecision = 'standard';
                            selectedFormat = Cogl.PixelFormat.RGBA_8888;
                            [renderTexture, renderOffscreen] = allocateTarget(selectedFormat);
                        }
                    }

                    const presentationPipeline = Cogl.Pipeline.new(coglContext);
                    presentationPipeline.set_layer_texture(0, renderTexture);
                    presentationPipeline.set_layer_filters(
                        0,
                        Cogl.PipelineFilter.LINEAR,
                        Cogl.PipelineFilter.LINEAR
                    );

                    let fxaaPipeline = null;
                    let fxaaTexture = null;
                    let fxaaOffscreen = null;
                    let fxaaPresentationPipeline = null;

                    if (antiAliasing === 'fxaa') {
                        fxaaPipeline = this.screenshaver_create_fxaa_pipeline(
                            coglContext,
                            renderTexture,
                            renderWidth,
                            renderHeight
                        );

                        // Match the production PostprocessPipeline: FXAA is a
                        // primary image-processing pass that writes into the
                        // output-resolution scratch target. GNOME then presents
                        // that texture using the same plain presentation path
                        // proven stable by Test #30B.
                        fxaaTexture = Cogl.Texture2D.new_with_format(
                            coglContext,
                            nativeWidth,
                            nativeHeight,
                            selectedFormat
                        );
                        fxaaTexture.set_premultiplied(false);
                        fxaaTexture.allocate();

                        fxaaOffscreen = Cogl.Offscreen.new_with_texture(fxaaTexture);
                        fxaaOffscreen.allocate();
                        fxaaOffscreen.set_viewport(
                            0.0,
                            0.0,
                            nativeWidth,
                            nativeHeight
                        );

                        fxaaPresentationPipeline = Cogl.Pipeline.new(coglContext);
                        fxaaPresentationPipeline.set_layer_texture(0, fxaaTexture);
                        fxaaPresentationPipeline.set_layer_filters(
                            0,
                            Cogl.PipelineFilter.LINEAR,
                            Cogl.PipelineFilter.LINEAR
                        );
                    }

                    let ditheringPipeline = null;
                    let ditheringTexture = null;
                    let ditheringOffscreen = null;
                    let ditheringPresentationPipeline = null;

                    if (dithering === 'subtle') {
                        const ditheringInputTexture = antiAliasing === 'fxaa' && fxaaTexture
                            ? fxaaTexture
                            : renderTexture;

                        ditheringPipeline = this.screenshaver_create_dithering_pipeline(
                            coglContext,
                            ditheringInputTexture
                        );

                        // Production dithering runs after the primary pass.
                        // Keep a dedicated output-resolution target so the
                        // final Clutter-facing pipeline remains a plain texture
                        // presentation, as proven stable by Test #31A.
                        ditheringTexture = Cogl.Texture2D.new_with_format(
                            coglContext,
                            nativeWidth,
                            nativeHeight,
                            selectedFormat
                        );
                        ditheringTexture.set_premultiplied(false);
                        ditheringTexture.allocate();

                        ditheringOffscreen = Cogl.Offscreen.new_with_texture(ditheringTexture);
                        ditheringOffscreen.allocate();
                        ditheringOffscreen.set_viewport(
                            0.0,
                            0.0,
                            nativeWidth,
                            nativeHeight
                        );

                        ditheringPresentationPipeline = Cogl.Pipeline.new(coglContext);
                        ditheringPresentationPipeline.set_layer_texture(0, ditheringTexture);
                        ditheringPresentationPipeline.set_layer_filters(
                            0,
                            Cogl.PipelineFilter.LINEAR,
                            Cogl.PipelineFilter.LINEAR
                        );
                    }

                    let audioBloomExtractionPipeline = null;
                    let audioBloomExtractionTexture = null;
                    let audioBloomExtractionOffscreen = null;
                    let audioBloomBlurHorizontalPipeline = null;
                    let audioBloomBlurTexture = null;
                    let audioBloomBlurOffscreen = null;
                    let audioBloomBlurVerticalPipeline = null;
                    let audioBloomBlurPresentationPipeline = null;
                    let audioBloomCompositePipeline = null;
                    let audioBloomCompositeTexture = null;
                    let audioBloomCompositeOffscreen = null;
                    let audioBloomCompositePresentationPipeline = null;

                    if (bloomMode === 'audio') {
                        const bloomInputTexture = antiAliasing === 'fxaa' && fxaaTexture
                            ? fxaaTexture
                            : renderTexture;
                        const bloomWidth = Math.max(1, Math.floor(nativeWidth / 2));
                        const bloomHeight = Math.max(1, Math.floor(nativeHeight / 2));

                        audioBloomExtractionPipeline =
                            this.screenshaver_create_audio_bloom_extraction_pipeline(
                                coglContext,
                                bloomInputTexture
                            );

                        audioBloomExtractionTexture = Cogl.Texture2D.new_with_format(
                            coglContext,
                            bloomWidth,
                            bloomHeight,
                            selectedFormat
                        );
                        audioBloomExtractionTexture.set_premultiplied(false);
                        audioBloomExtractionTexture.allocate();

                        audioBloomExtractionOffscreen =
                            Cogl.Offscreen.new_with_texture(audioBloomExtractionTexture);
                        audioBloomExtractionOffscreen.allocate();
                        audioBloomExtractionOffscreen.set_viewport(
                            0.0,
                            0.0,
                            bloomWidth,
                            bloomHeight
                        );

                        audioBloomBlurTexture = Cogl.Texture2D.new_with_format(
                            coglContext,
                            bloomWidth,
                            bloomHeight,
                            selectedFormat
                        );
                        audioBloomBlurTexture.set_premultiplied(false);
                        audioBloomBlurTexture.allocate();

                        audioBloomBlurOffscreen =
                            Cogl.Offscreen.new_with_texture(audioBloomBlurTexture);
                        audioBloomBlurOffscreen.allocate();
                        audioBloomBlurOffscreen.set_viewport(
                            0.0,
                            0.0,
                            bloomWidth,
                            bloomHeight
                        );

                        audioBloomBlurHorizontalPipeline =
                            this.screenshaver_create_bloom_blur_pipeline(
                                coglContext,
                                audioBloomExtractionTexture,
                                1.0 / bloomWidth,
                                0.0
                            );

                        audioBloomBlurVerticalPipeline =
                            this.screenshaver_create_bloom_blur_pipeline(
                                coglContext,
                                audioBloomBlurTexture,
                                0.0,
                                1.0 / bloomHeight
                            );

                        audioBloomBlurPresentationPipeline = Cogl.Pipeline.new(coglContext);
                        audioBloomBlurPresentationPipeline.set_layer_texture(
                            0,
                            audioBloomExtractionTexture
                        );
                        audioBloomBlurPresentationPipeline.set_layer_filters(
                            0,
                            Cogl.PipelineFilter.LINEAR,
                            Cogl.PipelineFilter.LINEAR
                        );

                        audioBloomCompositePipeline =
                            this.screenshaver_create_audio_bloom_composite_pipeline(
                                coglContext,
                                bloomInputTexture,
                                audioBloomExtractionTexture
                            );

                        audioBloomCompositeTexture = Cogl.Texture2D.new_with_format(
                            coglContext,
                            nativeWidth,
                            nativeHeight,
                            selectedFormat
                        );
                        audioBloomCompositeTexture.set_premultiplied(false);
                        audioBloomCompositeTexture.allocate();

                        audioBloomCompositeOffscreen =
                            Cogl.Offscreen.new_with_texture(audioBloomCompositeTexture);
                        audioBloomCompositeOffscreen.allocate();
                        audioBloomCompositeOffscreen.set_viewport(
                            0.0,
                            0.0,
                            nativeWidth,
                            nativeHeight
                        );

                        audioBloomCompositePresentationPipeline = Cogl.Pipeline.new(coglContext);
                        audioBloomCompositePresentationPipeline.set_layer_texture(
                            0,
                            audioBloomCompositeTexture
                        );
                        audioBloomCompositePresentationPipeline.set_layer_filters(
                            0,
                            Cogl.PipelineFilter.LINEAR,
                            Cogl.PipelineFilter.LINEAR
                        );

                        console.log(
                            `[Screenshaver] Test #37 Audio Bloom full pipeline targets allocated: ` +
                            `source=${bloomInputTexture.get_width()}x${bloomInputTexture.get_height()} ` +
                            `bloom=${bloomWidth}x${bloomHeight} threshold=${bloomThreshold.toFixed(3)} ` +
                            `intensity=${bloomIntensity.toFixed(3)} generation=${generation}`
                        );
                    }

                    this._screenshaverRenderTexture = renderTexture;
                    this._screenshaverRenderOffscreen = renderOffscreen;
                    this._screenshaverPresentationPipeline = presentationPipeline;
                    this._screenshaverFxaaPipeline = fxaaPipeline;
                    this._screenshaverFxaaTexture = fxaaTexture;
                    this._screenshaverFxaaOffscreen = fxaaOffscreen;
                    this._screenshaverFxaaPresentationPipeline = fxaaPresentationPipeline;
                    this._screenshaverDitheringPipeline = ditheringPipeline;
                    this._screenshaverDitheringTexture = ditheringTexture;
                    this._screenshaverDitheringOffscreen = ditheringOffscreen;
                    this._screenshaverDitheringPresentationPipeline = ditheringPresentationPipeline;
                    this._screenshaverAudioBloomExtractionPipeline = audioBloomExtractionPipeline;
                    this._screenshaverAudioBloomExtractionTexture = audioBloomExtractionTexture;
                    this._screenshaverAudioBloomExtractionOffscreen = audioBloomExtractionOffscreen;
                    this._screenshaverAudioBloomBlurHorizontalPipeline = audioBloomBlurHorizontalPipeline;
                    this._screenshaverAudioBloomBlurTexture = audioBloomBlurTexture;
                    this._screenshaverAudioBloomBlurOffscreen = audioBloomBlurOffscreen;
                    this._screenshaverAudioBloomBlurVerticalPipeline = audioBloomBlurVerticalPipeline;
                    this._screenshaverAudioBloomBlurPresentationPipeline = audioBloomBlurPresentationPipeline;
                    this._screenshaverAudioBloomCompositePipeline = audioBloomCompositePipeline;
                    this._screenshaverAudioBloomCompositeTexture = audioBloomCompositeTexture;
                    this._screenshaverAudioBloomCompositeOffscreen = audioBloomCompositeOffscreen;
                    this._screenshaverAudioBloomCompositePresentationPipeline = audioBloomCompositePresentationPipeline;
                    this.screenshaver_update_audio_bloom_uniforms();
                    this._screenshaverRenderWidth = renderWidth;
                    this._screenshaverRenderHeight = renderHeight;
                    this._screenshaverRequestedPrecision = requestedPrecision;
                    this._screenshaverSelectedPrecision = selectedPrecision;

                    const textureFormat = renderTexture.get_format();
                    const expectedFormat = selectedPrecision === 'high'
                        ? 'RGBA16F-equivalent'
                        : 'RGBA8-equivalent';

                    console.log(
                        `[Screenshaver] Test #30B Color Precision texture allocated: ` +
                        `requested=${requestedPrecision} selected=${selectedPrecision} ` +
                        `fallback=${fellBack ? 'yes' : 'no'} ` +
                        `texture_format=${textureFormat} expected=${expectedFormat} ` +
                        `native=${nativeWidth}x${nativeHeight} scale=${scale.toFixed(3)} ` +
                        `render=${renderWidth}x${renderHeight} generation=${generation}`
                    );

                    if (fellBack) {
                        console.log(
                            `[Screenshaver] Test #30 Color Precision Auto high-precision allocation failed; ` +
                            `using standard: ${fallbackReason}`
                        );
                    }
                }

                const shaderPipeline = this.get_pipeline();
                if (!shaderPipeline)
                    throw new Error('Shell.GLSLEffect shader pipeline unavailable');

                // Execute the Screenshaver shader at the reduced framebuffer
                // resolution. The pipeline still has Shell.GLSLEffect's native
                // actor texture as layer 0, but the procedural mainImage() body
                // determines the output color.
                this._screenshaverRenderOffscreen.clear4f(
                    Cogl.BufferBit.COLOR,
                    0.0,
                    0.0,
                    0.0,
                    1.0
                );
                this._screenshaverRenderOffscreen.draw_textured_rectangle(
                    shaderPipeline,
                    -1.0,
                    1.0,
                    1.0,
                    -1.0,
                    0.0,
                    0.0,
                    1.0,
                    1.0
                );
                this._screenshaverRenderOffscreen.flush();

                let finalPipeline = this._screenshaverPresentationPipeline;

                if (antiAliasing === 'fxaa' &&
                    this._screenshaverFxaaPipeline &&
                    this._screenshaverFxaaOffscreen &&
                    this._screenshaverFxaaPresentationPipeline) {
                    this.screenshaver_update_fxaa_uniforms();

                    this._screenshaverFxaaOffscreen.clear4f(
                        Cogl.BufferBit.COLOR,
                        0.0,
                        0.0,
                        0.0,
                        1.0
                    );
                    this._screenshaverFxaaOffscreen.draw_textured_rectangle(
                        this._screenshaverFxaaPipeline,
                        -1.0,
                        1.0,
                        1.0,
                        -1.0,
                        0.0,
                        0.0,
                        1.0,
                        1.0
                    );
                    this._screenshaverFxaaOffscreen.flush();

                    finalPipeline = this._screenshaverFxaaPresentationPipeline;
                }

                if (bloomMode === 'audio' &&
                    this._screenshaverAudioBloomExtractionPipeline &&
                    this._screenshaverAudioBloomExtractionOffscreen &&
                    this._screenshaverAudioBloomBlurHorizontalPipeline &&
                    this._screenshaverAudioBloomBlurOffscreen &&
                    this._screenshaverAudioBloomBlurVerticalPipeline &&
                    this._screenshaverAudioBloomCompositePipeline &&
                    this._screenshaverAudioBloomCompositeOffscreen &&
                    this._screenshaverAudioBloomCompositePresentationPipeline) {
                    this.screenshaver_update_audio_bloom_uniforms();

                    // Audio extraction -> half-resolution Bloom target A.
                    this._screenshaverAudioBloomExtractionOffscreen.clear4f(
                        Cogl.BufferBit.COLOR,
                        0.0,
                        0.0,
                        0.0,
                        1.0
                    );
                    this._screenshaverAudioBloomExtractionOffscreen.draw_textured_rectangle(
                        this._screenshaverAudioBloomExtractionPipeline,
                        -1.0,
                        1.0,
                        1.0,
                        -1.0,
                        0.0,
                        0.0,
                        1.0,
                        1.0
                    );
                    this._screenshaverAudioBloomExtractionOffscreen.flush();

                    // Production horizontal Gaussian pass: A -> B.
                    this._screenshaverAudioBloomBlurOffscreen.clear4f(
                        Cogl.BufferBit.COLOR,
                        0.0,
                        0.0,
                        0.0,
                        1.0
                    );
                    this._screenshaverAudioBloomBlurOffscreen.draw_textured_rectangle(
                        this._screenshaverAudioBloomBlurHorizontalPipeline,
                        -1.0,
                        1.0,
                        1.0,
                        -1.0,
                        0.0,
                        0.0,
                        1.0,
                        1.0
                    );
                    this._screenshaverAudioBloomBlurOffscreen.flush();

                    // Production vertical Gaussian pass: B -> A.
                    this._screenshaverAudioBloomExtractionOffscreen.clear4f(
                        Cogl.BufferBit.COLOR,
                        0.0,
                        0.0,
                        0.0,
                        1.0
                    );
                    this._screenshaverAudioBloomExtractionOffscreen.draw_textured_rectangle(
                        this._screenshaverAudioBloomBlurVerticalPipeline,
                        -1.0,
                        1.0,
                        1.0,
                        -1.0,
                        0.0,
                        0.0,
                        1.0,
                        1.0
                    );
                    this._screenshaverAudioBloomExtractionOffscreen.flush();

                    // Test #37: production additive composite at native output
                    // resolution. When Subtle dithering is enabled, the composite
                    // texture becomes the input to the already-proven final
                    // dithering pass below.
                    this._screenshaverAudioBloomCompositeOffscreen.clear4f(
                        Cogl.BufferBit.COLOR,
                        0.0,
                        0.0,
                        0.0,
                        1.0
                    );
                    this._screenshaverAudioBloomCompositeOffscreen.draw_textured_rectangle(
                        this._screenshaverAudioBloomCompositePipeline,
                        -1.0,
                        1.0,
                        1.0,
                        -1.0,
                        0.0,
                        0.0,
                        1.0,
                        1.0
                    );
                    this._screenshaverAudioBloomCompositeOffscreen.flush();

                    finalPipeline = this._screenshaverAudioBloomCompositePresentationPipeline;
                }

                if (dithering === 'subtle' &&
                    this._screenshaverDitheringPipeline &&
                    this._screenshaverDitheringOffscreen &&
                    this._screenshaverDitheringPresentationPipeline) {
                    // Production ordering requires dithering after Bloom
                    // composition. For Audio Bloom, retarget layer 0 from the
                    // primary-pass texture to the completed native-size
                    // composite texture immediately before the dithering draw.
                    if (bloomMode === 'audio' &&
                        this._screenshaverAudioBloomCompositeTexture) {
                        this._screenshaverDitheringPipeline.set_layer_texture(
                            0,
                            this._screenshaverAudioBloomCompositeTexture
                        );
                    }
                    this._screenshaverDitheringOffscreen.clear4f(
                        Cogl.BufferBit.COLOR,
                        0.0,
                        0.0,
                        0.0,
                        1.0
                    );
                    this._screenshaverDitheringOffscreen.draw_textured_rectangle(
                        this._screenshaverDitheringPipeline,
                        -1.0,
                        1.0,
                        1.0,
                        -1.0,
                        0.0,
                        0.0,
                        1.0,
                        1.0
                    );
                    this._screenshaverDitheringOffscreen.flush();

                    finalPipeline = this._screenshaverDitheringPresentationPipeline;
                }

                const rect = new Clutter.ActorBox({
                    x1: 0.0,
                    y1: 0.0,
                    x2: nativeWidth,
                    y2: nativeHeight,
                });
                const presentationNode = Clutter.PipelineNode.new(
                    finalPipeline
                );
                presentationNode.add_texture_rectangle(
                    rect,
                    0.0,
                    0.0,
                    1.0,
                    1.0
                );
                node.add_child(presentationNode);

                if (!this._screenshaverPresentationProbeLogged) {
                    const fbWidth = this._screenshaverRenderOffscreen.get_width();
                    const fbHeight = this._screenshaverRenderOffscreen.get_height();
                    const viewportWidth = this._screenshaverRenderOffscreen.get_viewport_width();
                    const viewportHeight = this._screenshaverRenderOffscreen.get_viewport_height();
                    console.log(
                        `[Screenshaver] Test #31A shader rasterization: ` +
                        `framebuffer=${fbWidth}x${fbHeight} ` +
                        `viewport=${viewportWidth}x${viewportHeight} -> ` +
                        `presentation=${nativeWidth}x${nativeHeight} ` +
                        `scale=${scale.toFixed(3)} precision=${this._screenshaverSelectedPrecision ?? requestedPrecision} ` +
                        `anti_aliasing=${antiAliasing} dithering=${dithering} bloom=${bloomMode} generation=${generation}`
                    );
                    this._screenshaverPresentationProbeLogged = true;
                }
            } catch (error) {
                console.log(
                    `[Screenshaver] Test #30 precision/render-scale shader pass failed; ` +
                    `falling back to Shell.GLSLEffect native paint ` +
                    `generation=${generation}: ${error}`
                );
                super.vfunc_paint_target(node, paintContext);
            }
        }
    });
}

const RUNTIME_SHADER_FILENAME = 'screenshaver-gnome-lock-shader.glsl';
const RUNTIME_METADATA_FILENAME = 'screenshaver-gnome-lock-metadata.txt';
const RUNTIME_ADVANCE_FILENAME = 'screenshaver-gnome-lock-advance.txt';
const RUNTIME_AUDIO_FILENAME = 'screenshaver-gnome-lock-audio.txt';
const SHADER_FAILURE_ADVANCE_DELAY_MS = 500;
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
const AUDIO_BAND_POLL_INTERVAL_MS = 100;
const AUDIO_BAND_LOG_INTERVAL_US = 1000000;
const FPS_AVERAGE_WINDOW_US = 5 * 1000000;
const FPS_CRITICAL_BLINK_INTERVAL_MS = 500;

const SHADER_METRICS_REPORT_INTERVAL_US = 5 * 1000000;
const POWER_SAVE_FALLBACK_INTERVAL_MS = 1000;
const POST_WAKE_POWER_SAVE_MIN_DELAY_MS = 10000;
const POST_BLANK_SCREENSHIELD_WAKE_DELAY_MS = 250;
const GNOME_50_STABILIZATION_WAKE_DELAY_MS = 1000;

// Test #39A: resolve GNOME Shell version at module scope so version gating is
// independent of extension instance initialization order.
const GNOME_SHELL_VERSION = String(Config.PACKAGE_VERSION ?? 'unknown');
const GNOME_SHELL_MAJOR =
    Number.parseInt(GNOME_SHELL_VERSION.split('.')[0], 10) || 0;
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
        console.log(
            `[Screenshaver] Test #39A GNOME Shell version=${GNOME_SHELL_VERSION} major=${GNOME_SHELL_MAJOR}`
        );

        this._lockActor = null;
        this._imageContent = null;
        this._shaderEffect = null;
        this._shaderUniformTime = -1;
        this._shaderUniformResolution = -1;
        this._shaderUniformInvertColors = -1;
        this._shaderUniformFlipHorizontal = -1;
        this._shaderUniformFlipVertical = -1;
        this._shaderUniformHueRotation = -1;
        this._shaderTickSource = null;
        this._shaderSourcePoll = null;
        this._audioBandPoll = null;
        this._audioBandReadInFlight = false;
        this._audioBandCancellable = null;
        // Preserve across GNOME disable()/enable() object reuse for the same
        // reason as the idle-inhibitor request generation below.
        this._audioBandRequestGeneration =
            this._audioBandRequestGeneration ?? 0;
        this._lastAudioBands = null;
        this._lastAudioBandLogUs = 0;
        this._activeProductionSource = null;
        this._failedProductionSource = null;
        this._failureAdvanceSource = null;
        this._shaderGeneration = 0;
        this._shaderStartedUs = 0;
        this._activeAnimationSpeed = 1.0;
        this._shaderTicks = 0;
        this._descriptionPill = null;
        this._descriptionPrefix = null;
        this._descriptionFps = null;
        this._descriptionMetadata = null;
        this._activeMetadataSignature = null;
        this._fpsWarningState = 'normal';
        this._fpsWindowStartedUs = 0;
        this._fpsWindowTicks = 0;
        this._fpsCriticalBlinkSource = null;
        this._fpsCriticalBlinkVisible = true;
        this._pollSource = null;
        this._transportGeneration = 0;
        this._lastFrameCounter = 0;
        this._displayedFrames = 0;
        this._screenShieldWakeIssued = false;
        this._gnomeShellMajor = GNOME_SHELL_MAJOR;
        this._gnome50StabilizationWakeIssued = false;
        this._gnome50StabilizationWakeSource = null;
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
        // GNOME may reuse this extension object across disable()/enable()
        // during session-mode transitions. Keep this generation monotonic so
        // callbacks from an earlier activation can never become current again.
        this._idleInhibitRequestGeneration =
            this._idleInhibitRequestGeneration ?? 0;
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

    _runtimeAudioPath() {
        return GLib.build_filenamev([
            GLib.get_user_runtime_dir(),
            RUNTIME_AUDIO_FILENAME,
        ]);
    }

    _parseAudioBands(contents) {
        const values = new Map();
        const text = new TextDecoder().decode(contents);

        for (const rawLine of text.split('\n')) {
            const line = rawLine.trim();

            if (!line)
                continue;

            const separator = line.indexOf('=');

            if (separator <= 0)
                continue;

            values.set(line.slice(0, separator), line.slice(separator + 1));
        }

        const version = Number.parseInt(values.get('version') ?? '', 10);

        if (version !== 1)
            throw new Error(`Unsupported GNOME lock audio-band version: ${version}`);

        const clampBand = value => Math.max(0.0, Math.min(1.0, value));

        return {
            bass: clampBand(Number.parseFloat(values.get('bass') ?? '0') || 0.0),
            midrange: clampBand(Number.parseFloat(values.get('midrange') ?? '0') || 0.0),
            treble: clampBand(Number.parseFloat(values.get('treble') ?? '0') || 0.0),
        };
    }

    _requestAudioBandsAsync() {
        if (this._audioBandReadInFlight || !this._lockActor)
            return;

        const audioFile = Gio.File.new_for_path(this._runtimeAudioPath());
        const requestGeneration = ++this._audioBandRequestGeneration;
        this._audioBandReadInFlight = true;

        audioFile.load_contents_async(
            this._audioBandCancellable,
            (file, result) => {
                if (requestGeneration !== this._audioBandRequestGeneration)
                    return;

                this._audioBandReadInFlight = false;

                if (!this._lockActor)
                    return;

                try {
                    const [ok, contents] = file.load_contents_finish(result);

                    if (!ok)
                        return;

                    const bands = this._parseAudioBands(contents);
                    this._lastAudioBands = bands;
                    this._shaderEffect?.screenshaver_set_audio_bands?.(
                        bands.bass,
                        bands.midrange,
                        bands.treble
                    );

                    const nowUs = GLib.get_monotonic_time();

                    if (this._lastAudioBandLogUs === 0
                        || nowUs - this._lastAudioBandLogUs >= AUDIO_BAND_LOG_INTERVAL_US) {
                        console.log(
                            `[Screenshaver] Test #33A GNOME audio bands: ` +
                            `bass=${bands.bass.toFixed(3)} ` +
                            `mid=${bands.midrange.toFixed(3)} ` +
                            `treble=${bands.treble.toFixed(3)}`
                        );
                        this._lastAudioBandLogUs = nowUs;
                    }
                } catch (_) {
                    // Atomic publication or teardown can briefly leave no readable file.
                }
            }
        );
    }

    _startAudioBandPolling() {
        if (this._audioBandPoll)
            return;

        this._audioBandCancellable = new Gio.Cancellable();
        this._audioBandReadInFlight = false;

        this._audioBandPoll = GLib.timeout_add(
            GLib.PRIORITY_DEFAULT_IDLE,
            AUDIO_BAND_POLL_INTERVAL_MS,
            () => {
                if (!this._lockActor) {
                    this._audioBandPoll = null;
                    return GLib.SOURCE_REMOVE;
                }

                this._requestAudioBandsAsync();
                return GLib.SOURCE_CONTINUE;
            }
        );

        console.log(
            `[Screenshaver] Test #33A GNOME asynchronous audio-band polling started: ${AUDIO_BAND_POLL_INTERVAL_MS}ms`
        );
    }

    _stopAudioBandPolling() {
        // Invalidate callbacks from reads issued by the activation being
        // torn down before cancelling their Gio.Cancellable.
        this._audioBandRequestGeneration++;

        if (this._audioBandPoll) {
            GLib.source_remove(this._audioBandPoll);
            this._audioBandPoll = null;
        }

        if (this._audioBandCancellable) {
            this._audioBandCancellable.cancel();
            this._audioBandCancellable = null;
        }

        this._audioBandReadInFlight = false;
        this._lastAudioBands = null;
        this._lastAudioBandLogUs = 0;
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
            animationSpeed: Math.max(0.0, Number.parseFloat(values.get('animation_speed') ?? '1.0') || 0.0),
            configuredFps: Math.max(1, Number.parseInt(values.get('configured_fps') ?? '1', 10) || 1),
            invertColors: values.get('invert_colors') === '1',
            flipHorizontal: values.get('flip_horizontal') === '1',
            flipVertical: values.get('flip_vertical') === '1',
            hueRotation: Number.parseFloat(values.get('hue_rotation') ?? '0') || 0.0,
            renderScale: Math.max(0.01, Number.parseFloat(values.get('render_scale') ?? '1') || 1.0),
            colorPrecision: (values.get('color_precision') ?? 'auto').toLowerCase(),
            antiAliasing: (values.get('anti_aliasing') ?? 'fxaa').toLowerCase(),
            dithering: (values.get('dithering') ?? 'subtle').toLowerCase(),
            bloomMode: (values.get('bloom') ?? 'off').toLowerCase(),
            bloomIntensity: Number.parseFloat(values.get('bloom_intensity') ?? '1.0') || 0.0,
            bloomThreshold: Number.parseFloat(values.get('bloom_threshold') ?? '0.80') || 0.0,
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
            metadata.animationSpeed,
            metadata.configuredFps,
            metadata.invertColors ? 1 : 0,
            metadata.flipHorizontal ? 1 : 0,
            metadata.flipVertical ? 1 : 0,
            metadata.hueRotation,
            metadata.renderScale,
            metadata.colorPrecision,
            metadata.antiAliasing,
            metadata.dithering,
            metadata.bloomMode,
            metadata.bloomIntensity,
            metadata.bloomThreshold,
            metadata.subtitles ? 1 : 0,
            metadata.placement,
        ].join('\u001f');
    }

    _applyPostprocessTransformMetadata(metadata) {
        if (!this._shaderEffect || !metadata)
            return;

        const fxaaEnabled = metadata.antiAliasing === 'fxaa';
        this._shaderEffect.set_uniform_float(
            this._shaderUniformInvertColors,
            1,
            [fxaaEnabled ? 0.0 : (metadata.invertColors ? 1.0 : 0.0)]
        );
        this._shaderEffect.set_uniform_float(
            this._shaderUniformFlipHorizontal,
            1,
            [fxaaEnabled ? 0.0 : (metadata.flipHorizontal ? 1.0 : 0.0)]
        );
        this._shaderEffect.set_uniform_float(
            this._shaderUniformFlipVertical,
            1,
            [fxaaEnabled ? 0.0 : (metadata.flipVertical ? 1.0 : 0.0)]
        );
        this._shaderEffect.set_uniform_float(
            this._shaderUniformHueRotation,
            1,
            [fxaaEnabled ? 0.0 : metadata.hueRotation]
        );

        this._shaderEffect.screenshaver_set_fxaa_transforms(
            metadata.invertColors,
            metadata.flipHorizontal,
            metadata.flipVertical,
            metadata.hueRotation
        );

        this._shaderEffect.queue_repaint();
    }

    _createDescriptionPill(parent) {
        if (this._descriptionPill)
            return;

        // Use separate text actors for the descriptive prefix and FPS segment.
        // Styling an FPS substring inside one Pango markup label proved unstable
        // under repeated GNOME Shell allocation/measurement.  Separate actors
        // guarantee that warning/critical styling can affect only the FPS text.
        this._descriptionPill = new St.BoxLayout({
            vertical: false,
            reactive: false,
            can_focus: false,
            style: [
                'background-color: rgba(0, 0, 0, 0.588);',
                'border-radius: 999px;',
                'padding: 9px 16px;',
            ].join(' '),
        });

        this._descriptionPrefix = new St.Label({
            reactive: false,
            can_focus: false,
            style: 'color: rgb(245, 245, 245); font-size: 18px;',
        });
        this._descriptionPrefix.clutter_text.single_line_mode = true;
        this._descriptionPrefix.clutter_text.ellipsize = Pango.EllipsizeMode.END;

        this._descriptionFps = new St.Label({
            reactive: false,
            can_focus: false,
            style: 'color: rgb(245, 245, 245); font-size: 18px;',
        });
        this._descriptionFps.clutter_text.single_line_mode = true;
        this._descriptionFps.clutter_text.ellipsize = Pango.EllipsizeMode.NONE;

        this._descriptionPill.add_child(this._descriptionPrefix);
        this._descriptionPill.add_child(this._descriptionFps);
        this._descriptionPill.hide();
        parent.add_child(this._descriptionPill);
    }

    _resetFpsWarningMonitor() {
        const nowUs = GLib.get_monotonic_time();
        this._stopCriticalFpsBlink();
        this._fpsWarningState = 'normal';
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

                if (nextState === 'critical')
                    this._startCriticalFpsBlink();
                else
                    this._stopCriticalFpsBlink();

                this._updateDescriptionPill();

                console.log(
                    `[Screenshaver] GNOME lock FPS state=${nextState} measured=${effectiveFps.toFixed(2)} configured=${configuredFps}`
                );
            }

            this._fpsWindowStartedUs = nowUs;
            this._fpsWindowTicks = 0;
        }

    }

    _startCriticalFpsBlink() {
        if (!this._descriptionFps)
            return;

        this._stopCriticalFpsBlink();
        this._fpsCriticalBlinkVisible = true;
        this._descriptionFps.opacity = 255;

        this._fpsCriticalBlinkSource = GLib.timeout_add(
            GLib.PRIORITY_DEFAULT,
            FPS_CRITICAL_BLINK_INTERVAL_MS,
            () => {
                if (!this._descriptionFps || this._fpsWarningState !== 'critical') {
                    this._fpsCriticalBlinkSource = null;
                    return GLib.SOURCE_REMOVE;
                }

                this._fpsCriticalBlinkVisible = !this._fpsCriticalBlinkVisible;
                this._descriptionFps.opacity =
                    this._fpsCriticalBlinkVisible ? 255 : 0;

                // Opacity changes do not alter preferred size or trigger pill
                // geometry recalculation. Only the independent FPS glyph actor
                // blinks; the capsule and descriptive label remain unchanged.
                return GLib.SOURCE_CONTINUE;
            }
        );
    }

    _stopCriticalFpsBlink() {
        if (this._fpsCriticalBlinkSource) {
            GLib.source_remove(this._fpsCriticalBlinkSource);
            this._fpsCriticalBlinkSource = null;
        }

        this._fpsCriticalBlinkVisible = true;

        if (this._descriptionFps)
            this._descriptionFps.opacity = 255;
    }

    _updateDescriptionPill(reposition = true) {
        if (!this._descriptionPill || !this._descriptionMetadata ||
            !this._descriptionPrefix || !this._descriptionFps)
            return;

        const metadata = this._descriptionMetadata;
        const warningActive = this._fpsWarningState !== 'normal';
        const shouldDisplay = metadata.subtitles || warningActive;

        if (!shouldDisplay) {
            this._descriptionPill.hide();
            return;
        }

        const prefixFields = [];

        if (metadata.subtitles) {
            if (metadata.shader)
                prefixFields.push(`P: ${metadata.shader}`);
            if (metadata.texture)
                prefixFields.push(`T: ${metadata.texture}`);
            if (metadata.palette)
                prefixFields.push(`P: ${metadata.palette}`);
        }

        const prefixText = prefixFields.join(' | ');
        const fpsText = `FPS: ${metadata.configuredFps}`;
        const showFps = metadata.subtitles || warningActive;

        // Put the separator in the normal-text actor. The FPS actor contains
        // only the FPS segment, so severity styling can never spill into the
        // policy name, animation speed, texture, palette, or separator.
        this._descriptionPrefix.set_text(
            prefixText && showFps ? `${prefixText} | ` : prefixText
        );
        this._descriptionFps.set_text(showFps ? fpsText : '');

        if (this._fpsWarningState === 'warning') {
            this._descriptionFps.set_style(
                'color: rgb(255, 221, 64); font-weight: normal;'
            );
        } else if (this._fpsWarningState === 'critical') {
            this._descriptionFps.set_style(
                'color: rgb(255, 72, 72); font-weight: bold;'
            );
        } else {
            this._descriptionFps.set_style(
                'color: rgb(245, 245, 245); font-weight: normal;'
            );
        }

        this._descriptionFps.opacity =
            this._fpsWarningState === 'critical' && !this._fpsCriticalBlinkVisible
                ? 0
                : 255;

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
            `padding: ${paddingY}px ${paddingX}px;`,
        ].join(' '));

        this._descriptionPrefix.set_style(
            `color: rgb(245, 245, 245); font-size: ${fontSize}px; font-weight: normal;`
        );

        const fpsColor = this._fpsWarningState === 'warning'
            ? 'rgb(255, 221, 64)'
            : this._fpsWarningState === 'critical'
                ? 'rgb(255, 72, 72)'
                : 'rgb(245, 245, 245)';
        const fpsWeight = this._fpsWarningState === 'critical' ? 'bold' : 'normal';
        this._descriptionFps.set_style(
            `color: ${fpsColor}; font-size: ${fontSize}px; font-weight: ${fpsWeight};`
        );

        // Measure the two actors independently.  The FPS actor is never
        // ellipsized; when the pill exceeds the production 80% maximum, only
        // the descriptive prefix is shortened.  This keeps the complete FPS
        // value and its severity styling visible at all times.
        this._descriptionPrefix.clutter_text.ellipsize = Pango.EllipsizeMode.NONE;
        this._descriptionFps.clutter_text.ellipsize = Pango.EllipsizeMode.NONE;

        const [, naturalPrefixWidth] =
            this._descriptionPrefix.clutter_text.get_preferred_width(-1);
        const [, naturalFpsWidth] =
            this._descriptionFps.clutter_text.get_preferred_width(-1);
        const measurementAllowance = Math.max(2, Math.ceil(4 * scale));
        const desiredContentWidth =
            Math.ceil(naturalPrefixWidth) + Math.ceil(naturalFpsWidth) + measurementAllowance;
        const desiredWidth = desiredContentWidth + paddingX * 2;
        const width = Math.min(
            maximumWidth,
            Math.max(fontSize + paddingY * 2, desiredWidth)
        );

        const availableContentWidth = Math.max(1, width - paddingX * 2);
        const prefixAvailableWidth = Math.max(
            0,
            availableContentWidth - Math.ceil(naturalFpsWidth) - measurementAllowance
        );

        this._descriptionPrefix.clutter_text.ellipsize =
            desiredWidth > maximumWidth
                ? Pango.EllipsizeMode.END
                : Pango.EllipsizeMode.NONE;

        if (desiredWidth > maximumWidth)
            this._descriptionPrefix.set_width(prefixAvailableWidth);
        else
            this._descriptionPrefix.set_width(-1);

        const [, naturalPrefixHeight] =
            this._descriptionPrefix.clutter_text.get_preferred_height(
                Math.max(1, prefixAvailableWidth || Math.ceil(naturalPrefixWidth))
            );
        const [, naturalFpsHeight] =
            this._descriptionFps.clutter_text.get_preferred_height(
                Math.max(1, Math.ceil(naturalFpsWidth))
            );
        const naturalTextHeight = Math.max(naturalPrefixHeight, naturalFpsHeight);
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

    _requestShaderAdvance(reason, productionSource = null) {
        if (productionSource)
            this._failedProductionSource = productionSource;

        if (this._failureAdvanceSource)
            return;

        console.log(
            `[Screenshaver] Test #27 GNOME shader application failed; requesting next shader in ${SHADER_FAILURE_ADVANCE_DELAY_MS}ms: ${reason}`
        );

        this._failureAdvanceSource = GLib.timeout_add(
            GLib.PRIORITY_DEFAULT,
            SHADER_FAILURE_ADVANCE_DELAY_MS,
            () => {
                this._failureAdvanceSource = null;

                if (Main.sessionMode.currentMode !== 'unlock-dialog')
                    return GLib.SOURCE_REMOVE;

                const advancePath = GLib.build_filenamev([
                    GLib.get_user_runtime_dir(),
                    RUNTIME_ADVANCE_FILENAME,
                ]);

                const policyId = this._descriptionMetadata?.policyId ?? 0;
                const text = [
                    'version=1',
                    `policy_id=${policyId}`,
                    `reason=${String(reason).replace(/[\\r\\n\\0]/g, ' ')}`,
                    '',
                ].join('\\n');

                try {
                    GLib.file_set_contents(advancePath, text);
                    console.log(
                        `[Screenshaver] Test #27 requested early GNOME shader rotation for policy_id=${policyId}`
                    );
                } catch (error) {
                    console.log(
                        `[Screenshaver] Test #27 unable to request early GNOME shader rotation: ${error}`
                    );
                }

                return GLib.SOURCE_REMOVE;
            }
        );
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

    _buildShaderEffect(productionSource, width, height, elapsedSeconds, renderScale, colorPrecision, antiAliasing, dithering, bloomMode, bloomThreshold, bloomIntensity) {
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
            nextGeneration,
            renderScale,
            colorPrecision,
            antiAliasing,
            dithering,
            bloomMode,
            bloomThreshold,
            bloomIntensity
        );
        const effect = new EffectClass();

        console.log(
            `[Screenshaver] Test #27 registered unique shader effect GType generation=${nextGeneration}`
        );

        const uniformTime = effect.get_uniform_location('iTime');
        const uniformResolution = effect.get_uniform_location('iResolution');
        const uniformInvertColors = effect.get_uniform_location('screenshaverInvertColors');
        const uniformFlipHorizontal = effect.get_uniform_location('screenshaverFlipHorizontal');
        const uniformFlipVertical = effect.get_uniform_location('screenshaverFlipVertical');
        const uniformHueRotation = effect.get_uniform_location('screenshaverHueRotation');

        effect.set_uniform_float(
            uniformTime,
            1,
            [elapsedSeconds]
        );
        const initialRenderWidth = Math.max(1, Math.round(width * renderScale));
        const initialRenderHeight = Math.max(1, Math.round(height * renderScale));
        effect.set_uniform_float(
            uniformResolution,
            3,
            [initialRenderWidth, initialRenderHeight, 1.0]
        );
        effect.set_uniform_float(uniformInvertColors, 1, [0.0]);
        effect.set_uniform_float(uniformFlipHorizontal, 1, [0.0]);
        effect.set_uniform_float(uniformFlipVertical, 1, [0.0]);
        effect.set_uniform_float(uniformHueRotation, 1, [0.0]);

        return {
            effect,
            uniformTime,
            uniformResolution,
            uniformInvertColors,
            uniformFlipHorizontal,
            uniformFlipVertical,
            uniformHueRotation,
        };
    }

    _installInitialShaderEffect(dialog) {
        const {
            shaderPath,
            shaderBytes,
            productionSource,
        } = this._readProductionShaderSource();

        let initialMetadata = null;
        try {
            const metadata = this._readPresentationMetadata();
            if (!metadata.sourceBytes || metadata.sourceBytes === shaderBytes.length)
                initialMetadata = metadata;
        } catch (error) {
            console.log(`[Screenshaver] GNOME description metadata unavailable: ${error}`);
        }

        const initialAnimationSpeed = initialMetadata?.animationSpeed ?? 1.0;
        const initialRenderScale = initialMetadata?.renderScale ?? 1.0;
        const initialColorPrecision = initialMetadata?.colorPrecision ?? 'auto';
        const initialAntiAliasing = initialMetadata?.antiAliasing ?? 'fxaa';
        const initialDithering = initialMetadata?.dithering ?? 'subtle';
        const initialBloomMode = initialMetadata?.bloomMode ?? 'off';
        const initialBloomThreshold = initialMetadata?.bloomThreshold ?? 0.80;
        const initialBloomIntensity = initialMetadata?.bloomIntensity ?? 1.0;
        const built = this._buildShaderEffect(
            productionSource,
            dialog.width,
            dialog.height,
            0.0,
            initialRenderScale,
            initialColorPrecision,
            initialAntiAliasing,
            initialDithering,
            initialBloomMode,
            initialBloomThreshold,
            initialBloomIntensity
        );

        this._shaderEffect = built.effect;
        if (this._lastAudioBands) {
            this._shaderEffect.screenshaver_set_audio_bands(
                this._lastAudioBands.bass,
                this._lastAudioBands.midrange,
                this._lastAudioBands.treble
            );
        }
        this._shaderUniformTime = built.uniformTime;
        this._shaderUniformResolution = built.uniformResolution;
        this._shaderUniformInvertColors = built.uniformInvertColors;
        this._shaderUniformFlipHorizontal = built.uniformFlipHorizontal;
        this._shaderUniformFlipVertical = built.uniformFlipVertical;
        this._shaderUniformHueRotation = built.uniformHueRotation;
        this._activeProductionSource = productionSource;
        this._activeAnimationSpeed = initialAnimationSpeed;
        this._shaderGeneration = 1;

        if (initialMetadata) {
            this._descriptionMetadata = initialMetadata;
            this._activeMetadataSignature = this._metadataSignature(initialMetadata);
            this._applyPostprocessTransformMetadata(initialMetadata);
        }

        this._lockActor.add_effect_with_name(
            'screenshaver-production-shader-bridge',
            this._shaderEffect
        );

        console.log(
            `[Screenshaver] Test #27 loaded production-preprocessed shader handoff: ${shaderPath} (${shaderBytes.length} bytes) generation=${this._shaderGeneration}`
        );
        console.log(
            `[Screenshaver] Test #38 GNOME animation speed: generation=${this._shaderGeneration} speed=${this._activeAnimationSpeed.toFixed(3)}x`
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
            `[Screenshaver] Test #27 production shader handoff polling started: ${SHADER_SOURCE_POLL_INTERVAL_MS}ms`
        );
    }

    _stopShaderSourcePolling() {
        if (this._shaderSourcePoll) {
            GLib.source_remove(this._shaderSourcePoll);
            this._shaderSourcePoll = null;
        }

        this._activeProductionSource = null;
        this._failedProductionSource = null;
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
            const activeScale = this._descriptionMetadata?.renderScale ?? 1.0;
            const activePrecision = this._descriptionMetadata?.colorPrecision ?? 'auto';
            const activeAntiAliasing = this._descriptionMetadata?.antiAliasing ?? 'fxaa';
            const activeDithering = this._descriptionMetadata?.dithering ?? 'subtle';
            const activeBloomMode = this._descriptionMetadata?.bloomMode ?? 'off';
            const activeBloomThreshold = this._descriptionMetadata?.bloomThreshold ?? 0.80;
            const activeBloomIntensity = this._descriptionMetadata?.bloomIntensity ?? 1.0;
            const scaleChanged = Math.abs(observedMetadata.renderScale - activeScale) > 0.0001;
            const precisionChanged = observedMetadata.colorPrecision !== activePrecision;
            const antiAliasingChanged = observedMetadata.antiAliasing !== activeAntiAliasing;
            const ditheringChanged = observedMetadata.dithering !== activeDithering;
            const bloomChanged = observedMetadata.bloomMode !== activeBloomMode
                || Math.abs(observedMetadata.bloomThreshold - activeBloomThreshold) > 0.0001
                || Math.abs(observedMetadata.bloomIntensity - activeBloomIntensity) > 0.0001;

            if (!scaleChanged && !precisionChanged && !antiAliasingChanged && !ditheringChanged && !bloomChanged) {
                if (observedMetadataSignature !== this._activeMetadataSignature) {
                    this._descriptionMetadata = observedMetadata;
                    this._activeMetadataSignature = observedMetadataSignature;
                    this._applyPostprocessTransformMetadata(observedMetadata);
                    this._resetFpsWarningMonitor();
                    console.log(
                        `[Screenshaver] GNOME description metadata updated for policy_id=${observedMetadata.policyId}`
                    );
                }
                return;
            }

            if (scaleChanged) {
                console.log(
                    `[Screenshaver] Test #29 Render Scale metadata changed ` +
                    `${activeScale.toFixed(3)} -> ${observedMetadata.renderScale.toFixed(3)}; ` +
                    `rebuilding native effect target`
                );
            }

            if (precisionChanged) {
                console.log(
                    `[Screenshaver] Test #30 Color Precision metadata changed ` +
                    `${activePrecision} -> ${observedMetadata.colorPrecision}; ` +
                    `rebuilding native effect target`
                );
            }

            if (antiAliasingChanged) {
                console.log(
                    `[Screenshaver] Test #31 Anti-Aliasing metadata changed ` +
                    `${activeAntiAliasing} -> ${observedMetadata.antiAliasing}; ` +
                    `rebuilding native effect pipeline`
                );
            }

            if (ditheringChanged) {
                console.log(
                    `[Screenshaver] Test #32 Dithering metadata changed ` +
                    `${activeDithering} -> ${observedMetadata.dithering}; ` +
                    `rebuilding native effect pipeline`
                );
            }

            if (bloomChanged) {
                console.log(
                    `[Screenshaver] Test #34 Audio Bloom metadata changed; ` +
                    `rebuilding extraction pipeline`
                );
            }
        }

        // Test #38: production FrameRenderEngine resets start_time on each
        // shader switch. Build the replacement at shader-local time zero.
        const elapsedSeconds = 0.0;

        let built;

        try {
            built = this._buildShaderEffect(
                handoff.productionSource,
                this._lockActor.width,
                this._lockActor.height,
                elapsedSeconds,
                observedMetadata.renderScale,
                observedMetadata.colorPrecision,
                observedMetadata.antiAliasing,
                observedMetadata.dithering,
                observedMetadata.bloomMode,
                observedMetadata.bloomThreshold,
                observedMetadata.bloomIntensity
            );
        } catch (error) {
            console.log(
                `[Screenshaver] Test #27 replacement shader preparation failed; retaining active shader: ${error}`
            );
            this._requestShaderAdvance(
                `replacement preparation failed: ${error}`,
                handoff.productionSource
            );
            return;
        }

        const replacementMetadata = observedMetadata;

        const previousEffect = this._shaderEffect;
        const previousUniformTime = this._shaderUniformTime;
        const previousUniformResolution = this._shaderUniformResolution;
        const previousUniformInvertColors = this._shaderUniformInvertColors;
        const previousUniformFlipHorizontal = this._shaderUniformFlipHorizontal;
        const previousUniformFlipVertical = this._shaderUniformFlipVertical;
        const previousUniformHueRotation = this._shaderUniformHueRotation;
        const previousDescriptionMetadata = this._descriptionMetadata;
        const previousAnimationSpeed = this._activeAnimationSpeed;

        try {
            // Keep the lock actor itself in place. Only the shader effect is
            // replaced, preserving GNOME's lock/session/power-management state.
            this._lockActor.remove_effect(previousEffect);

            this._shaderEffect = built.effect;
        if (this._lastAudioBands) {
            this._shaderEffect.screenshaver_set_audio_bands(
                this._lastAudioBands.bass,
                this._lastAudioBands.midrange,
                this._lastAudioBands.treble
            );
        }
            this._shaderUniformTime = built.uniformTime;
            this._shaderUniformResolution = built.uniformResolution;
            this._shaderUniformInvertColors = built.uniformInvertColors;
            this._shaderUniformFlipHorizontal = built.uniformFlipHorizontal;
            this._shaderUniformFlipVertical = built.uniformFlipVertical;
            this._shaderUniformHueRotation = built.uniformHueRotation;

            this._descriptionMetadata = replacementMetadata;
            this._activeAnimationSpeed = replacementMetadata.animationSpeed;
            this._shaderStartedUs = GLib.get_monotonic_time();
            this._applyPostprocessTransformMetadata(replacementMetadata);

            this._lockActor.add_effect_with_name(
                'screenshaver-production-shader-bridge',
                this._shaderEffect
            );

            this._activeProductionSource = handoff.productionSource;
            this._failedProductionSource = null;
            this._shaderGeneration++;
            this._activeMetadataSignature = observedMetadataSignature;
            this._resetFpsWarningMonitor();

            this._shaderEffect.queue_repaint();
            this._lockActor.queue_redraw();

            console.log(
                `[Screenshaver] Test #27 hot-swapped production shader generation=${this._shaderGeneration} bytes=${handoff.shaderBytes.length} unique-gtype=true`
            );
            console.log(
                `[Screenshaver] Test #38 GNOME animation speed: generation=${this._shaderGeneration} speed=${this._activeAnimationSpeed.toFixed(3)}x`
            );
        } catch (error) {
            console.log(
                `[Screenshaver] Test #27 replacement effect swap failed: ${error}`
            );
            this._requestShaderAdvance(
                `replacement effect swap failed: ${error}`,
                handoff.productionSource
            );

            // Best-effort rollback to the previously proven effect.
            try {
                if (this._shaderEffect !== previousEffect)
                    this._lockActor.remove_effect(this._shaderEffect);
            } catch (_) {
            }

            this._shaderEffect = previousEffect;
            this._shaderUniformTime = previousUniformTime;
            this._shaderUniformResolution = previousUniformResolution;
            this._shaderUniformInvertColors = previousUniformInvertColors;
            this._shaderUniformFlipHorizontal = previousUniformFlipHorizontal;
            this._shaderUniformFlipVertical = previousUniformFlipVertical;
            this._shaderUniformHueRotation = previousUniformHueRotation;
            this._descriptionMetadata = previousDescriptionMetadata;
            this._activeAnimationSpeed = previousAnimationSpeed;

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
        // Test #27 currently supports the ShaderToy path only, so the first
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

        // Test #27: execute production-preprocessed ShaderToy mainImage() through
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
                `[Screenshaver] ERROR: Unable to create Test #27 ShaderToy Shell.GLSLEffect: ${error}`
            );
            this._requestShaderAdvance(
                `initial effect creation failed: ${error}`,
                this._activeProductionSource
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
            '[Screenshaver] Test #27 shader actor added above GNOME lock background'
        );

        // Preserve the already-proven GNOME lock/power-management handling.
        this._startPowerSaveRecovery();
        this._startShaderSourcePolling();
        this._startAudioBandPolling();

        this._shaderStartedUs = GLib.get_monotonic_time();
        this._shaderTicks = 0;

        // Test #27 keeps the proven GLib callback-rate instrumentation so we can
        // distinguish visible compositor presentation from a blanked output.
        this._shaderMetricsWindowStartedUs = this._shaderStartedUs;
        this._shaderMetricsWindowTicks = 0;
        this._shaderMetricsPreviousTickUs = null;
        this._shaderMetricsMinDeltaUs = Number.POSITIVE_INFINITY;
        this._shaderMetricsMaxDeltaUs = 0;

        console.log(
            `[Screenshaver] Test #27 requested shader tick interval: ${SHADER_TICK_INTERVAL_MS}ms (~${Math.round(1000 / SHADER_TICK_INTERVAL_MS)} Hz maximum)`
        );

        this._shaderTickSource = GLib.timeout_add(
            GLib.PRIORITY_DEFAULT,
            SHADER_TICK_INTERVAL_MS,
            () => {
                if (!this._lockActor || !this._shaderEffect) {
                    this._shaderTickSource = null;
                    return GLib.SOURCE_REMOVE;
                }

                if (this._failedProductionSource === this._activeProductionSource)
                    return GLib.SOURCE_CONTINUE;

                const elapsedSeconds =
                    (GLib.get_monotonic_time() - this._shaderStartedUs) / 1000000.0;

                try {
                    const [renderWidth, renderHeight] =
                        this._shaderEffect.screenshaver_render_size(
                            this._lockActor.width,
                            this._lockActor.height
                        );
                    this._shaderEffect.set_uniform_float(
                        this._shaderUniformResolution,
                        3,
                        [renderWidth, renderHeight, 1.0]
                    );
                    this._shaderEffect.set_uniform_float(
                        this._shaderUniformTime,
                        1,
                        [elapsedSeconds * this._activeAnimationSpeed]
                    );
                    this._shaderEffect.queue_repaint();
                    this._lockActor.queue_redraw();
                    this._positionDescriptionPill();
                } catch (error) {
                    console.log(
                        `[Screenshaver] Shell.GLSLEffect animation update failed: ${error}`
                    );
                    this._requestShaderAdvance(
                        `animation update failed: ${error}`,
                        this._activeProductionSource
                    );
                    return GLib.SOURCE_CONTINUE;
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
                        `[Screenshaver] Test #27 timing: requested=${SHADER_TICK_INTERVAL_MS}ms callbacks=${this._shaderMetricsWindowTicks} elapsed=${metricsElapsedSeconds.toFixed(3)}s effective=${effectiveHz.toFixed(2)}Hz avg=${averageIntervalMs.toFixed(2)}ms min=${minIntervalMs.toFixed(2)}ms max=${maxIntervalMs.toFixed(2)}ms total_ticks=${this._shaderTicks}`
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
                        '[Screenshaver] First Test #27 shader frame requested'
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
            this._scheduleGnome50StabilizationWake();

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


    _scheduleGnome50StabilizationWake() {
        console.log(
            `[Screenshaver] Test #39A stabilization helper entered: major=${this._gnomeShellMajor} issued=${this._gnome50StabilizationWakeIssued} pending=${Boolean(this._gnome50StabilizationWakeSource)} actor=${Boolean(this._lockActor)} mode=${Main.sessionMode.currentMode}`
        );

        if (this._gnomeShellMajor !== 50) {
            console.log(
                `[Screenshaver] Test #39A stabilization wake skipped: GNOME major ${this._gnomeShellMajor} is not 50`
            );
            return;
        }

        if (this._gnome50StabilizationWakeIssued) {
            console.log(
                '[Screenshaver] Test #39A stabilization wake skipped: one-shot wake already issued'
            );
            return;
        }

        if (this._gnome50StabilizationWakeSource) {
            console.log(
                '[Screenshaver] Test #39A stabilization wake skipped: one-shot wake already pending'
            );
            return;
        }

        if (!this._lockActor) {
            console.log(
                '[Screenshaver] Test #39A stabilization wake skipped: lock actor unavailable'
            );
            return;
        }

        console.log(
            `[Screenshaver] Test #39A GNOME 50 stabilization wake scheduled in ${GNOME_50_STABILIZATION_WAKE_DELAY_MS}ms`
        );

        this._gnome50StabilizationWakeSource = GLib.timeout_add(
            GLib.PRIORITY_DEFAULT,
            GNOME_50_STABILIZATION_WAKE_DELAY_MS,
            () => {
                this._gnome50StabilizationWakeSource = null;

                if (this._gnome50StabilizationWakeIssued)
                    return GLib.SOURCE_REMOVE;

                this._gnome50StabilizationWakeIssued = true;

                if (!this._lockActor ||
                    Main.sessionMode.currentMode !== 'unlock-dialog' ||
                    !Main.screenShield?.locked ||
                    !Main.screenShield?.active) {
                    console.log(
                        '[Screenshaver] Test #39A GNOME 50 stabilization wake skipped because secure lock state is no longer active'
                    );
                    return GLib.SOURCE_REMOVE;
                }

                const screenShield = Main.screenShield;

                if (typeof screenShield._wakeUpScreen !== 'function') {
                    console.log(
                        '[Screenshaver] Test #39A GNOME 50 stabilization wake method unavailable'
                    );
                    return GLib.SOURCE_REMOVE;
                }

                try {
                    console.log(
                        '[Screenshaver] Test #39A requesting GNOME 50 one-shot stabilization ScreenShield wake'
                    );
                    screenShield._wakeUpScreen();
                    console.log(
                        '[Screenshaver] Test #39A GNOME 50 one-shot stabilization ScreenShield wake completed'
                    );
                } catch (error) {
                    console.log(
                        `[Screenshaver] Test #39A GNOME 50 stabilization ScreenShield wake failed: ${error}`
                    );
                }

                return GLib.SOURCE_REMOVE;
            }
        );
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
                    `[Screenshaver] Test #27 idle inhibitor waiting: ${waitState}`
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

        console.log('[Screenshaver] Test #27 requesting GNOME session idle inhibitor (flag=8)');

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
                            `[Screenshaver] Test #27 GNOME session idle inhibitor request failed: ${error}`
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
                        `[Screenshaver] Test #27 GNOME session idle inhibitor acquired cookie=${cookie} flag=8`
                    );
                }
            );
        } catch (error) {
            if (requestGeneration === this._idleInhibitRequestGeneration)
                this._idleInhibitRequestPending = false;

            console.log(
                `[Screenshaver] Test #27 unable to dispatch GNOME session idle inhibitor request: ${error}`
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
                            `[Screenshaver] Test #27 GNOME session idle inhibitor released cookie=${cookie} (${reason})`
                        );
                    } catch (error) {
                        console.log(
                            `[Screenshaver] Test #27 GNOME session idle inhibitor release failed cookie=${cookie}: ${error}`
                        );
                    }
                }
            );
        } catch (error) {
            console.log(
                `[Screenshaver] Test #27 unable to dispatch GNOME session idle inhibitor release cookie=${cookie}: ${error}`
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

        // Test #27 keeps the startup wake sequence from the previous tests,
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
                        `[Screenshaver] Test #27 rejecting delayed PowerSaveMode 0 -> 3 after ${Math.floor(elapsedSincePostBlankWakeUs / 1000)}ms; restoring NORMAL without ScreenShield wake`
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
            // handles GNOME's initial lock transition; Test #27 never uses a
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
                `[Screenshaver] Test #27 rejecting delayed PowerSaveMode 0 -> 3 after ${Math.floor(elapsedUs / 1000)}ms; restoring NORMAL without ScreenShield wake`
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
                        console.log('[Screenshaver] Test #27 PowerSaveMode NORMAL correction completed');
                    } catch (error) {
                        if (this._lockActor)
                            console.log(`[Screenshaver] Test #27 PowerSaveMode NORMAL correction failed: ${error}`);
                    }
                }
            );
        } catch (error) {
            this._postWakePowerSaveCorrectionInFlight = false;
            console.log(`[Screenshaver] Test #27 unable to dispatch PowerSaveMode NORMAL correction: ${error}`);
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
        this._stopAudioBandPolling();
        this._stopPowerSaveRecovery();
        this._releaseIdleInhibitor();

        if (this._gnome50StabilizationWakeSource) {
            GLib.source_remove(this._gnome50StabilizationWakeSource);
            this._gnome50StabilizationWakeSource = null;
        }

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

        this._stopCriticalFpsBlink();

        if (this._failureAdvanceSource) {
            GLib.source_remove(this._failureAdvanceSource);
            this._failureAdvanceSource = null;
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
        this._shaderUniformInvertColors = -1;
        this._shaderUniformFlipHorizontal = -1;
        this._shaderUniformFlipVertical = -1;
        this._shaderUniformHueRotation = -1;
        this._shaderStartedUs = 0;
        this._activeAnimationSpeed = 1.0;
        this._shaderTicks = 0;
        this._descriptionPrefix = null;
        this._descriptionFps = null;
        this._descriptionMetadata = null;
        this._activeMetadataSignature = null;
        this._fpsWarningState = 'normal';
        this._fpsWindowStartedUs = 0;
        this._fpsWindowTicks = 0;
        this._fpsCriticalBlinkSource = null;
        this._fpsCriticalBlinkVisible = true;
        this._displayedFrames = 0;
        this._refreshCalls = 0;
        this._uploadAttempts = 0;
        this._uploadSuccesses = 0;
        this._transportErrorLogged = false;
        this._lastObservedPowerSaveMode = null;
        this._gnome50StabilizationWakeIssued = false;
        this._gnome50StabilizationWakeSource = null;
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

