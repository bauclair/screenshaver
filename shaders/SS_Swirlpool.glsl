/*
 * Screenshaver Native Shader
 * --------------------------
 * Name:        Whirlpool
 * File:        SS_Whirlpool.glsl
 * Project:     Screenshaver
 *
 * Original shader created specifically for the Screenshaver project.
 * This shader is not copied from or adapted line-for-line from third-party
 * shader source code.
 *
 * Technique:   Soft primary-color rotating flow field
 * Native API:  Screenshaver GLSL
 * GLSL:        330 core
 *
 * Copyright:   Screenshaver contributors
 * License:     GNU General Public License v3.0 or later
 */

#version 330 core

uniform float iTime;
uniform vec3 iResolution;

out vec4 fragColor;

const float TAU = 6.28318530717958647692;

mat2 rotation(float a)
{
    float s = sin(a);
    float c = cos(a);
    return mat2(c, -s, s, c);
}

vec3 primarySpectrum(float t)
{
    // Smoothly interpolate among saturated red, yellow, green, blue and
    // violet regions. The interpolation is deliberately broad and soft.
    const vec3 RED    = vec3(1.00, 0.015, 0.015);
    const vec3 YELLOW = vec3(1.00, 0.92,  0.010);
    const vec3 GREEN  = vec3(0.01, 1.00,  0.035);
    const vec3 BLUE   = vec3(0.01, 0.16,  1.000);
    const vec3 VIOLET = vec3(0.72, 0.01,  1.000);

    float x = fract(t) * 5.0;
    int band = int(floor(x));
    float f = fract(x);

    // Cubic interpolation gives very gradual transitions with no hard edge.
    f = f * f * (3.0 - 2.0 * f);

    if (band == 0) return mix(RED,    YELLOW, f);
    if (band == 1) return mix(YELLOW, GREEN,  f);
    if (band == 2) return mix(GREEN,  BLUE,   f);
    if (band == 3) return mix(BLUE,   VIOLET, f);
    return mix(VIOLET, RED, f);
}

void main()
{
    vec2 resolution = max(iResolution.xy, vec2(1.0));
    vec2 p = (2.0 * gl_FragCoord.xy - resolution) / resolution.y;

    float radius = length(p);
    float angle = atan(p.y, p.x);

    // A true rotating frame establishes the dominant whirlpool motion.
    float globalSpin = iTime * 0.46;
    vec2 q = rotation(-globalSpin) * p;

    // Stronger curl toward the center. The denominator remains broad so the
    // center bends smoothly rather than forming a pinched spiral line.
    float curl = 2.75 / (0.58 + radius * 1.75);
    q = rotation(curl) * q;

    // Large-scale flowing deformation. These are intentionally low-frequency:
    // the goal is broad melted color masses, not visible ribbons or filaments.
    vec2 flow = q;

    flow.x +=
        0.24 * sin(q.y * 1.45 + iTime * 0.31)
        + 0.13 * sin((q.x + q.y) * 1.05 - iTime * 0.19);

    flow.y +=
        0.23 * sin(q.x * 1.30 - iTime * 0.27)
        + 0.12 * cos((q.x - q.y) * 1.15 + iTime * 0.22);

    // A second broad warp causes the colored masses to stretch and fold as
    // they rotate, giving a liquid/melting character without sharp contours.
    vec2 warped = flow;

    warped +=
        0.16 * vec2(
            sin(flow.y * 2.05 + iTime * 0.24),
            cos(flow.x * 1.85 - iTime * 0.21)
        );

    float warpedRadius = length(warped);
    float warpedAngle = atan(warped.y, warped.x);

    // Color is driven by a continuous two-dimensional field rather than by
    // distance alone, so there are no concentric color rings.
    float colorField =
        warped.x * 0.34
        + warped.y * 0.29
        + sin(warpedAngle * 2.0 - warpedRadius * 1.65) * 0.12
        - iTime * 0.025;

    vec3 colorA = primarySpectrum(colorField);

    // A nearby phase sample is softly mixed in. This produces broad blended
    // transition zones instead of sharply bounded color bands.
    vec3 colorB =
        primarySpectrum(
            colorField
            + 0.10
            + 0.035 * sin(warped.x * 1.3 + warped.y * 1.1)
        );

    float blend =
        0.42
        + 0.18 * sin(
            warped.x * 1.25
            - warped.y * 1.10
            + iTime * 0.16
        );

    vec3 color = mix(colorA, colorB, blend);

    // Broad luminance variation gives Audio Bloom useful bright and dark
    // material to react to, but avoids narrow luminous lines.
    float luminousFlow =
        0.5
        + 0.5 * sin(
            warped.x * 1.65
            + warped.y * 1.35
            + sin(warpedAngle - iTime * 0.20) * 0.75
        );

    float brightness =
        0.70
        + 0.30 * luminousFlow;

    // The center is gently energized, remaining fully chromatic rather than
    // becoming a white hotspot.
    float centerEnergy =
        exp(-radius * radius * 2.1);

    brightness += centerEnergy * 0.14;

    color *= brightness;

    // Preserve high saturation while preventing bright overlap from clipping
    // into white. The output still contains strong luminance variation for
    // Screenshaver's post-processing and Audio Bloom.
    color = color / (vec3(1.0) + color * 0.13);

    float luma = dot(color, vec3(0.299, 0.587, 0.114));
    color = clamp(mix(vec3(luma), color, 1.16), 0.0, 1.0);

    fragColor = vec4(color, 1.0);
}
