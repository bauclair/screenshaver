/*
 * Screenshaver Native Shader
 *
 * Title:        Luminous Lattice
 * Filename:     SS_Luminous_Lattice.glsl
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
 *   Demonstration shader for Screenshaver screensaver, wallpaper,
 *   and Windowshader rendering modes.
 *
 * License: GNU General Public License v3.0
 */

#version 330 core

uniform float iTime;
uniform vec3 iResolution;

out vec4 fragColor;

const float TAU = 6.28318530718;

vec3 spectrum(float t)
{
    // Use the same vivid primary-color sequence as SS_Swirlpool: saturated
    // red, yellow, green, blue and violet with broad, smooth transitions.
    const vec3 RED    = vec3(1.00, 0.015, 0.015);
    const vec3 YELLOW = vec3(1.00, 0.92,  0.010);
    const vec3 GREEN  = vec3(0.01, 1.00,  0.035);
    const vec3 BLUE   = vec3(0.01, 0.16,  1.000);
    const vec3 VIOLET = vec3(0.72, 0.01,  1.000);

    float x = fract(t) * 5.0;
    int band = int(floor(x));
    float f = fract(x);

    // Match Swirlpool's cubic interpolation so adjacent primary regions blend
    // smoothly without introducing hard color boundaries into the lattice.
    f = f * f * (3.0 - 2.0 * f);

    if (band == 0) return mix(RED,    YELLOW, f);
    if (band == 1) return mix(YELLOW, GREEN,  f);
    if (band == 2) return mix(GREEN,  BLUE,   f);
    if (band == 3) return mix(BLUE,   VIOLET, f);
    return mix(VIOLET, RED, f);
}

float latticeBand(float coordinate, float spacing, float width)
{
    float cell = abs(fract(coordinate / spacing + 0.5) - 0.5) * spacing;
    return 1.0 - smoothstep(width, width * 2.8, cell);
}

float latticeGlow(float coordinate, float spacing, float radius)
{
    float cell = abs(fract(coordinate / spacing + 0.5) - 0.5) * spacing;
    return exp(-cell * cell / max(radius * radius, 0.000001));
}

void main()
{
    vec2 resolution = max(iResolution.xy, vec2(1.0));
    vec2 p = (2.0 * gl_FragCoord.xy - resolution) / resolution.y;

    float t = iTime;

    // A very gentle coordinate deformation keeps the grid alive without
    // turning it into a liquid surface. The two axes deform differently so
    // the diamonds continually breathe and shear.
    vec2 q = p;
    q.x += 0.055 * sin(p.y * 2.15 + t * 0.47)
         + 0.025 * sin(p.y * 4.70 - t * 0.29);
    q.y += 0.050 * sin(p.x * 1.85 - t * 0.41)
         + 0.022 * sin(p.x * 4.25 + t * 0.33);

    // Rotated coordinates create the two crossing diagonal families.
    const float invSqrt2 = 0.70710678118;
    float diagA = (q.x + q.y) * invSqrt2;
    float diagB = (q.x - q.y) * invSqrt2;

    const float spacing = 0.245;

    // The two families drift independently. Their unequal rates prevent the
    // same set of crossings from simply translating as one rigid image.
    float aCoord = diagA + t * 0.115;
    float bCoord = diagB - t * 0.087;

    float aCore = latticeBand(aCoord, spacing, 0.0105);
    float bCore = latticeBand(bCoord, spacing, 0.0105);
    float aGlow = latticeGlow(aCoord, spacing, 0.040);
    float bGlow = latticeGlow(bCoord, spacing, 0.040);

    // Colors travel along each line family as well as through time. This makes
    // every crossing continually acquire a new pair of saturated hues.
    float phaseA = 0.13 * t + 0.19 * diagA + 0.11 * diagB
                 + 0.045 * sin(q.y * 2.4 - t * 0.31);
    float phaseB = 0.61 - 0.105 * t + 0.17 * diagB - 0.08 * diagA
                 + 0.050 * sin(q.x * 2.1 + t * 0.27);

    vec3 colorA = spectrum(phaseA);
    vec3 colorB = spectrum(phaseB);

    // Dark chromatic atmosphere instead of absolute black. It gives the image
    // depth while preserving very high contrast for bloom extraction.
    float atmosphere = 0.018 + 0.014 * sin(q.x * 1.7 + q.y * 1.3 + t * 0.16);
    vec3 color = spectrum(0.08 * t + 0.035 * q.x) * max(atmosphere, 0.004);

    // Broad halos first, then bright colored cores.
    color += colorA * aGlow * 0.24;
    color += colorB * bGlow * 0.24;
    color += colorA * aCore * 0.88;
    color += colorB * bCore * 0.88;

    // Crossing nodes receive an emissive lift. Mix the two hues rather than
    // forcing them to white, retaining chroma even at the brightest points.
    float crossing = aCore * bCore;
    vec3 nodeColor = normalize(colorA + colorB + vec3(0.001));
    nodeColor *= max(max(colorA.r, colorA.g), colorA.b)
               + max(max(colorB.r, colorB.g), colorB.b);
    color += nodeColor * crossing * 0.72;

    // A slower, wider crossing halo makes intersections feel suspended above
    // the lattice and supplies medium-bright material for audio/loudness bloom.
    float crossingHalo = aGlow * bGlow;
    color += mix(colorA, colorB, 0.5) * crossingHalo * 0.20;

    // Mild compression preserves saturated highlights instead of clipping them
    // abruptly to featureless white.
    color = color / (vec3(1.0) + color * 0.34);
    color = pow(max(color, vec3(0.0)), vec3(0.90));

    fragColor = vec4(color, 1.0);
}
