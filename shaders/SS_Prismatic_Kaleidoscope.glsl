/*
 * Screenshaver Native Shader
 *
 * Title:        Prismatic Kaleidoscope
 * Filename:     SS_Prismatic_Kaleidoscope.glsl
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
 *   Procedural kaleidoscopic shader for Screenshaver screensaver, wallpaper,
 *   and Windowshader rendering modes.
 *
 * License: GNU General Public License v3.0
 */

#version 330 core

uniform float iTime;
uniform vec3 iResolution;

out vec4 fragColor;

const float PI = 3.14159265358979323846;

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

float softDisk(vec2 p, vec2 center, float radius, float softness)
{
    float d = length(p - center);
    return 1.0 - smoothstep(radius - softness, radius + softness, d);
}

float softRing(vec2 p, vec2 center, float radius, float width)
{
    float d = abs(length(p - center) - radius);
    return 1.0 - smoothstep(width * 0.35, width, d);
}

float softDiamond(vec2 p, vec2 center, float radius, float softness)
{
    vec2 q = abs(p - center);
    float d = q.x + q.y;
    return 1.0 - smoothstep(radius - softness, radius + softness, d);
}

float softBox(vec2 p, vec2 center, vec2 halfSize, float softness)
{
    vec2 d = abs(p - center) - halfSize;
    float outside = length(max(d, vec2(0.0)));
    float inside = min(max(d.x, d.y), 0.0);
    float sd = outside + inside;
    return 1.0 - smoothstep(-softness, softness, sd);
}

float petal(vec2 p, vec2 center, float angle, vec2 scale, float softness)
{
    vec2 q = rotate2D(-angle) * (p - center);
    q /= scale;
    float d = length(q);
    return 1.0 - smoothstep(1.0 - softness, 1.0 + softness, d);
}

void main()
{
    vec2 resolution = max(iResolution.xy, vec2(1.0));
    vec2 p = (2.0 * gl_FragCoord.xy - resolution) / resolution.y;

    // Slightly enlarge the complete kaleidoscope so its visible circular field
    // reaches the top and bottom edges of the rendering surface.  Keeping this
    // as a single coordinate-scale adjustment preserves the approved geometry,
    // animation, palette, and relative proportions.
    p *= 0.88;

    float time = iTime;

    // Slowly rotate the symmetry frame. Internal material motion below uses
    // different rates so the image never behaves like a rigid spinning disk.
    p *= rotate2D(time * 0.075);

    float radius = length(p);
    float angle = atan(p.y, p.x);

    // Twelve mirrored sectors: six repeated wedges, each reflected around its
    // centerline. This creates exact kaleidoscopic symmetry without textures.
    const float SECTORS = 12.0;
    float sectorAngle = 2.0 * PI / SECTORS;
    float foldedAngle =
        abs(mod(angle + 0.5 * sectorAngle, sectorAngle) - 0.5 * sectorAngle);

    vec2 k = vec2(cos(foldedAngle), sin(foldedAngle)) * radius;

    // Independent drift and breathing occur inside the folded wedge.
    vec2 drift = vec2(
        0.20 * sin(time * 0.19),
        0.16 * cos(time * 0.16 + 0.7)
    );

    vec2 q = k * (2.15 + 0.10 * sin(time * 0.13)) + drift;

    // Broad procedural masses form the primary visual material. Their centers
    // and radii move independently, causing shapes to merge and separate while
    // the mirrored sector boundaries remain exact.
    vec2 c1 = vec2(
        0.48 + 0.18 * sin(time * 0.23),
        0.22 + 0.15 * cos(time * 0.17)
    );
    vec2 c2 = vec2(
        0.86 + 0.16 * cos(time * 0.14 + 1.2),
        0.55 + 0.18 * sin(time * 0.21 + 0.4)
    );
    vec2 c3 = vec2(
        1.18 + 0.20 * sin(time * 0.11 + 2.1),
        0.15 + 0.12 * cos(time * 0.27)
    );

    float mass1 = softDisk(
        q, c1,
        0.34 + 0.055 * sin(time * 0.31),
        0.10
    );

    float mass2 = softDiamond(
        q, c2,
        0.39 + 0.060 * cos(time * 0.24),
        0.11
    );

    float mass3 = softDisk(
        q, c3,
        0.30 + 0.050 * sin(time * 0.29 + 1.5),
        0.09
    );

    // Secondary structures add detail at different scales and positions. Their
    // motions are deliberately incommensurate with the large masses so the
    // folded facets continually develop new internal arrangements.
    float box1 = softBox(
        q,
        vec2(
            0.72 + 0.11 * sin(time * 0.37 + 0.4),
            0.27 + 0.09 * cos(time * 0.33)
        ),
        vec2(0.16, 0.095),
        0.055
    );

    float diamond2 = softDiamond(
        q,
        vec2(
            1.37 + 0.12 * cos(time * 0.26),
            0.39 + 0.10 * sin(time * 0.35 + 1.1)
        ),
        0.205 + 0.025 * sin(time * 0.41),
        0.060
    );

    float petal1 = petal(
        q,
        vec2(
            0.96 + 0.12 * sin(time * 0.28 + 2.0),
            0.18 + 0.08 * cos(time * 0.39)
        ),
        0.42 + 0.22 * sin(time * 0.17),
        vec2(0.28, 0.095),
        0.16
    );

    float petal2 = petal(
        q,
        vec2(
            1.52 + 0.10 * cos(time * 0.31 + 0.7),
            0.22 + 0.07 * sin(time * 0.43)
        ),
        -0.58 + 0.18 * cos(time * 0.21),
        vec2(0.22, 0.075),
        0.17
    );

    // Broad curved bands add structure without reducing the shader to outlines.
    float arc1 = softRing(
        q,
        vec2(0.58, 0.12),
        0.54 + 0.08 * sin(time * 0.18),
        0.115
    );

    float arc2 = softRing(
        q,
        vec2(1.04, 0.44),
        0.47 + 0.07 * cos(time * 0.22 + 0.8),
        0.105
    );

    float arc3 = softRing(
        q,
        vec2(1.46, 0.10),
        0.36 + 0.045 * sin(time * 0.34 + 2.2),
        0.070
    );

    // A low-frequency field gently reshapes brightness and color distribution.
    float field =
        0.5
        + 0.25 * sin(q.x * 3.1 + q.y * 2.0 - time * 0.43)
        + 0.25 * sin(q.x * 1.6 - q.y * 3.4 + time * 0.31);

    field = clamp(field, 0.0, 1.0);

    // Finer crossing waves introduce small-scale facet detail, but remain
    // smooth enough to avoid noisy pixel-scale sparkle.
    float fineField =
        0.5
        + 0.25 * sin(q.x * 7.1 + q.y * 4.6 + time * 0.52)
        + 0.25 * sin(q.x * 5.3 - q.y * 8.0 - time * 0.39);

    fineField = clamp(fineField, 0.0, 1.0);

    float filled =
        clamp(
            mass1 * 0.82
            + mass2 * 0.74
            + mass3 * 0.70
            + box1 * 0.54
            + diamond2 * 0.52
            + petal1 * 0.48
            + petal2 * 0.45
            + arc1 * 0.42
            + arc2 * 0.38
            + arc3 * 0.34,
            0.0,
            1.0
        );

    // Carve gentle internal subdivisions into some of the larger forms. This
    // creates nested facets rather than simply adding more overlapping blobs.
    float subdivision =
        smoothstep(0.30, 0.72, fineField)
        * smoothstep(0.16, 0.62, filled);

    filled = clamp(filled * (0.82 + 0.18 * subdivision), 0.0, 1.0);

    // The collection-wide Screenshaver palette fills substantial areas rather
    // than merely tracing edges. Spatial and temporal terms move independently.
    float paletteA =
        radius * 0.30
        + q.x * 0.10
        - q.y * 0.075
        + time * 0.035;

    float paletteB =
        paletteA
        + 0.18
        + 0.055 * sin(q.x * 2.2 + time * 0.27);

    vec3 colorA = primarySpectrum(paletteA);
    vec3 colorB = primarySpectrum(paletteB);
    vec3 colorC = primarySpectrum(
        paletteA + 0.31 + 0.07 * sin(q.y * 4.2 - time * 0.23)
    );

    vec3 spectral = mix(colorA, colorB, field);
    spectral = mix(spectral, colorC, 0.28 * fineField);

    // Dark chromatic base keeps the entire frame alive while retaining enough
    // contrast for bright kaleidoscopic forms and Screenshaver bloom modes.
    vec3 background =
        primarySpectrum(
            0.61 + radius * 0.07 - time * 0.012
        ) * (0.025 + 0.025 * field);

    vec3 color = mix(background, spectral * (0.54 + 0.58 * field), filled);

    // Luminous boundaries emerge from changes in the filled-shape field.
    // fwidth provides screen-space anti-aliasing and avoids brittle hard lines.
    float edgeWidth = max(fwidth(filled), 0.0025);
    float luminousEdge =
        smoothstep(0.10, 0.10 + edgeWidth * 2.0, filled)
        * (1.0 - smoothstep(0.78, 0.78 + edgeWidth * 3.0, filled));

    color += spectral * luminousEdge * 0.20;

    // Give the center a small prismatic jewel rather than a black singularity.
    float centerJewel =
        1.0 - smoothstep(0.025, 0.115, radius);

    vec3 centerColor =
        primarySpectrum(time * 0.055 + foldedAngle / sectorAngle * 0.16);

    color += centerColor * centerJewel * 0.48;

    // Soft radial falloff keeps attention inside the kaleidoscopic field while
    // preserving useful geometry at arbitrary Windowshader aspect ratios.
    float outerFade =
        1.0 - smoothstep(0.92, 1.65, radius);

    color *= 0.35 + 0.65 * outerFade;

    // Bright intersections should remain saturated rather than washing to white.
    color = color / (vec3(1.0) + color * 0.20);

    float luma = dot(color, vec3(0.299, 0.587, 0.114));
    color = clamp(mix(vec3(luma), color, 1.09), 0.0, 1.0);

    fragColor = vec4(color, 1.0);
}
