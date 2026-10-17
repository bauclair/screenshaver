/*
 * Screenshaver Native Shader
 *
 * Title:        Plasma Veil
 * Filename:     SS_Plasma_Veil.glsl
 * Author:       Screenshaver Project
 * Shader API:   Screenshaver native GLSL
 * GLSL Version: 330 core
 *
 * Origin:
 *   Original shader created specifically for the Screenshaver project.
 *
 * Provenance:
 *   Independently authored for Screenshaver. This shader is not copied
 *   or adapted from ShaderToy, ISF, or another third-party shader.
 *
 * Purpose:
 *   Lightweight procedural shader for Screenshaver screensaver, wallpaper,
 *   and Windowshader rendering modes.
 *
 * License: GNU General Public License v3.0
 */

#version 330 core

uniform float iTime;
uniform vec3 iResolution;

out vec4 fragColor;

vec3 primarySpectrum(float t)
{
    const vec3 RED    = vec3(1.00, 0.015, 0.015);
    const vec3 YELLOW = vec3(1.00, 0.92,  0.010);
    const vec3 GREEN  = vec3(0.01, 1.00,  0.035);
    const vec3 BLUE   = vec3(0.01, 0.16,  1.000);
    const vec3 VIOLET = vec3(0.72, 0.01,  1.000);

    float x = fract(t) * 5.0;
    int band = int(floor(x));
    float f = fract(x);
    f = f * f * (3.0 - 2.0 * f);

    if (band == 0) return mix(RED,    YELLOW, f);
    if (band == 1) return mix(YELLOW, GREEN,  f);
    if (band == 2) return mix(GREEN,  BLUE,   f);
    if (band == 3) return mix(BLUE,   VIOLET, f);
    return mix(VIOLET, RED, f);
}

mat2 rotate2D(float a)
{
    float c = cos(a);
    float s = sin(a);
    return mat2(c, -s, s, c);
}

// A broad, inexpensive interference field. The frequencies are intentionally
// low so the shader forms large soft sheets rather than fine plasma noise.
float veilField(vec2 p, float time, float phase)
{
    vec2 q = p;

    q.x += 0.24 * sin(q.y * 1.35 + time * 0.31 + phase);
    q.y += 0.20 * sin(q.x * 1.10 - time * 0.27 + phase * 1.3);

    float a = sin(q.x * 1.55 + q.y * 0.72 + time * 0.34 + phase);
    float b = sin(q.y * 1.82 - q.x * 0.48 - time * 0.29 + phase * 1.7);
    float c = sin((q.x + q.y) * 0.91 + time * 0.21 - phase * 0.8);

    return (a + b + c) / 3.0;
}

float softRibbon(float field, float center, float width)
{
    float d = abs(field - center);
    return 1.0 - smoothstep(width * 0.38, width, d);
}

void main()
{
    vec2 resolution = max(iResolution.xy, vec2(1.0));
    vec2 p = (2.0 * gl_FragCoord.xy - resolution) / resolution.y;

    float time = iTime;

    // Very slow coordinate rotation prevents the flow from favoring one screen
    // axis while remaining subtle enough that it does not look like a spinning
    // image.
    vec2 q1 = rotate2D(0.16 + 0.055 * sin(time * 0.09)) * p;
    vec2 q2 = rotate2D(-0.54 + 0.070 * cos(time * 0.07)) * p;
    vec2 q3 = rotate2D(0.92 + 0.045 * sin(time * 0.11 + 1.4)) * p;

    float f1 = veilField(q1 * 1.02, time, 0.3);
    float f2 = veilField(q2 * 0.86, time * 0.91, 2.2);
    float f3 = veilField(q3 * 1.18, time * 1.07, 4.1);

    // Three broad translucent-looking sheets. Each uses a different field,
    // width, phase and motion rate, so their overlaps continually reorganize.
    float veil1 =
        softRibbon(
            f1,
            0.12 * sin(time * 0.17),
            0.54
        );

    float veil2 =
        softRibbon(
            f2,
            -0.10 + 0.11 * cos(time * 0.13 + 0.8),
            0.48
        );

    float veil3 =
        softRibbon(
            f3,
            0.08 + 0.10 * sin(time * 0.15 + 2.0),
            0.43
        );

    // Softer envelopes extend beyond the brighter cores and give each veil a
    // layered, translucent appearance without requiring actual alpha blending.
    float haze1 = smoothstep(-0.80, 0.62, f1) * 0.34;
    float haze2 = smoothstep(-0.72, 0.68, f2) * 0.30;
    float haze3 = smoothstep(-0.76, 0.64, f3) * 0.28;

    float phase1 =
        0.02
        + p.x * 0.055
        + p.y * 0.035
        + time * 0.026
        + f1 * 0.075;

    float phase2 =
        0.34
        - p.x * 0.040
        + p.y * 0.060
        - time * 0.019
        + f2 * 0.070;

    float phase3 =
        0.67
        + p.x * 0.048
        - p.y * 0.052
        + time * 0.022
        + f3 * 0.080;

    vec3 color1 = primarySpectrum(phase1);
    vec3 color2 = primarySpectrum(phase2);
    vec3 color3 = primarySpectrum(phase3);

    // Dark chromatic atmosphere rather than absolute black.
    float atmosphere =
        0.5
        + 0.25 * sin(p.x * 0.72 - time * 0.10)
        + 0.25 * sin(p.y * 0.83 + time * 0.08);

    vec3 background =
        primarySpectrum(
            0.58
            + p.x * 0.020
            - p.y * 0.018
            - time * 0.006
        ) * (0.018 + 0.022 * atmosphere);

    vec3 color = background;

    // Add the broad haze first, then the brighter veil bodies. Overlaps produce
    // richer mixtures while each layer keeps a recognizable spectral identity.
    color += color1 * haze1 * 0.34;
    color += color2 * haze2 * 0.31;
    color += color3 * haze3 * 0.29;

    color += color1 * veil1 * (0.34 + 0.22 * haze1);
    color += color2 * veil2 * (0.32 + 0.22 * haze2);
    color += color3 * veil3 * (0.30 + 0.20 * haze3);

    // Bright, feathered inner folds provide bloom targets. They are deliberately
    // broad enough to remain soft and cloth-like rather than becoming neon lines.
    float fold1 = pow(clamp(veil1, 0.0, 1.0), 2.4);
    float fold2 = pow(clamp(veil2, 0.0, 1.0), 2.6);
    float fold3 = pow(clamp(veil3, 0.0, 1.0), 2.5);

    color += color1 * fold1 * 0.24;
    color += color2 * fold2 * 0.22;
    color += color3 * fold3 * 0.20;

    // Slightly lift complex overlaps so the veil intersections become luminous
    // events for Screenshaver's bloom modes without clipping them to white.
    float overlap =
        veil1 * veil2
        + veil2 * veil3
        + veil3 * veil1;

    vec3 overlapColor =
        primarySpectrum(
            0.16
            + 0.08 * f1
            - 0.06 * f2
            + 0.05 * f3
            + time * 0.018
        );

    color += overlapColor * overlap * 0.12;

    // Gentle compression preserves saturated chroma in the brightest folds.
    color = color / (vec3(1.0) + color * 0.20);

    float luma = dot(color, vec3(0.299, 0.587, 0.114));
    color = clamp(mix(vec3(luma), color, 1.08), 0.0, 1.0);

    fragColor = vec4(color, 1.0);
}
