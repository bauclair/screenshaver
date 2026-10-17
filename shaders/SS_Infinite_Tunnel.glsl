/*
 * Screenshaver Native Shader
 *
 * Title:        Infinite Tunnel
 * Filename:     SS_Infinite_Tunnel.glsl
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
 *   Texture-showcase shader for Screenshaver screensaver, wallpaper,
 *   and Windowshader rendering modes. The same generated texture is mapped
 *   independently and at equal world-space scale onto the tunnel's four walls.
 *
 * License: GNU General Public License v3.0
 */

#version 330 core

uniform float iTime;
uniform vec3 iResolution;
uniform sampler2D iChannel0;

out vec4 fragColor;

const float HALF_WIDTH  = 1.35;
const float HALF_HEIGHT = 0.82;
const float TEX_SCALE   = 0.52;
const float EPSILON     = 0.00001;

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

vec3 sampleWallTexture(int wall, vec3 p, float travel)
{
    float longitudinal = p.z + travel;
    vec2 uv;

    // Every surface uses exactly the same TEX_SCALE.  Only orientation differs.
    // This keeps texture features the same physical size on all four walls.
    if (wall == 0) {
        // Floor
        uv = vec2(p.x, longitudinal) * TEX_SCALE;
    } else if (wall == 1) {
        // Ceiling
        uv = vec2(p.x, -longitudinal) * TEX_SCALE;
    } else if (wall == 2) {
        // Left wall
        uv = vec2(longitudinal, p.y) * TEX_SCALE;
    } else {
        // Right wall
        uv = vec2(-longitudinal, p.y) * TEX_SCALE;
    }

    return texture(iChannel0, uv).rgb;
}

void considerHit(
    float t,
    int wall,
    vec3 ro,
    vec3 rd,
    inout float bestT,
    inout int bestWall,
    inout vec3 bestPos)
{
    if (t <= EPSILON || t >= bestT) {
        return;
    }

    vec3 p = ro + rd * t;

    if (wall <= 1) {
        if (abs(p.x) <= HALF_WIDTH + 0.0001) {
            bestT = t;
            bestWall = wall;
            bestPos = p;
        }
    } else {
        if (abs(p.y) <= HALF_HEIGHT + 0.0001) {
            bestT = t;
            bestWall = wall;
            bestPos = p;
        }
    }
}

void main()
{
    vec2 resolution = max(iResolution.xy, vec2(1.0));
    vec2 screen = (2.0 * gl_FragCoord.xy - resolution) / resolution.y;

    float travel = iTime * 2.15;

    // Keep the tunnel itself geometrically straight so the texture is never
    // bent into rings or onion-like arcs.  A small camera drift supplies motion
    // beyond forward travel without deforming the four planar wall surfaces.
    vec2 cameraDrift = vec2(
        0.16 * sin(iTime * 0.23),
        0.09 * sin(iTime * 0.19 + 1.1)
    );

    vec3 ro = vec3(cameraDrift, 0.0);
    vec3 rd = normalize(vec3(screen, 1.35));

    float bestT = 1.0e20;
    int bestWall = -1;
    vec3 hitPos = vec3(0.0);

    // Analytic intersection with four infinite tunnel planes.  Unlike the
    // earlier finite-depth marcher, these planes continue all the way toward
    // the vanishing point, so there is no central black opening.
    if (rd.y < -EPSILON) {
        considerHit(
            (-HALF_HEIGHT - ro.y) / rd.y,
            0, ro, rd, bestT, bestWall, hitPos
        );
    }

    if (rd.y > EPSILON) {
        considerHit(
            (HALF_HEIGHT - ro.y) / rd.y,
            1, ro, rd, bestT, bestWall, hitPos
        );
    }

    if (rd.x < -EPSILON) {
        considerHit(
            (-HALF_WIDTH - ro.x) / rd.x,
            2, ro, rd, bestT, bestWall, hitPos
        );
    }

    if (rd.x > EPSILON) {
        considerHit(
            (HALF_WIDTH - ro.x) / rd.x,
            3, ro, rd, bestT, bestWall, hitPos
        );
    }

    // Only the mathematically exact center ray can miss all four parallel
    // planes. Fill that sub-pixel vanishing point with colored distant light
    // rather than creating a visible black hole.
    if (bestWall < 0) {
        vec3 vanishingColor =
            primarySpectrum(iTime * 0.055 + 0.58) * 0.34;
        fragColor = vec4(vanishingColor, 1.0);
        return;
    }

    vec3 texel = sampleWallTexture(bestWall, hitPos, travel);

    // Preserve the selected Screenshaver texture as the dominant material.
    vec3 material = pow(max(texel, vec3(0.0)), vec3(0.90));
    float textureLuma = dot(material, vec3(0.299, 0.587, 0.114));
    material = mix(vec3(textureLuma), material, 0.88);

    // The Screenshaver primary palette acts as broad illumination.  Crucially,
    // there is no periodic depth-wave term here, so lighting cannot form
    // concentric bands across the perspective surfaces.
    float transverse =
        (bestWall <= 1)
            ? (hitPos.x / (2.0 * HALF_WIDTH))
            : (hitPos.y / (2.0 * HALF_HEIGHT));

    float colorPhase =
        iTime * 0.045
        + transverse * 0.34
        + float(bestWall) * 0.11;

    vec3 spectralLight = primarySpectrum(colorPhase);

    // Mild depth attenuation gives perspective without drawing depth rings.
    float depth = max(hitPos.z, 0.0);
    float depthFade = 0.52 + 0.48 / (1.0 + depth * 0.018);

    // Keep enough neutral illumination that every texture remains identifiable,
    // then layer vivid spectral light over it.
    vec3 color =
        material
        * (vec3(0.38) + spectralLight * 0.74)
        * depthFade;

    // Thin colored corner seams clarify the four-wall geometry.  Their width
    // depends only on distance from the physical corners, not tunnel depth.
    float cornerDistance;
    if (bestWall <= 1) {
        cornerDistance = HALF_WIDTH - abs(hitPos.x);
    } else {
        cornerDistance = HALF_HEIGHT - abs(hitPos.y);
    }

    float seam = 1.0 - smoothstep(0.0, 0.045, cornerDistance);
    color += spectralLight * seam * 0.30 * depthFade;

    // Preserve chroma in bright regions instead of allowing them to wash white.
    color = color / (vec3(1.0) + color * 0.18);

    float luma = dot(color, vec3(0.299, 0.587, 0.114));
    color = clamp(mix(vec3(luma), color, 1.07), 0.0, 1.0);

    fragColor = vec4(color, 1.0);
}
