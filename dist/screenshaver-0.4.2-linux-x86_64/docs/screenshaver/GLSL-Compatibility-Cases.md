===========================================================
GLSL-CASE-0001
===========================================================

Shader:
    Singularity

Date Investigated:
    2026-07-11

Symptoms:

    Renderer instability.

Root Cause:

    Uninitialized multi-variable declarations.

Implemented Rules:

    GLSL-COMP-0006

Disposition:

    Fixed.

Regression Test:

    tests/fixtures/Singularity.glsl


===========================================================
GLSL-CASE-0002
===========================================================

Shader:
    Dark Transit

Date Investigated:
    2026-07-11

Symptoms:

    Color shift.
    Rendering artifacts.

Root Cause:

    Uninitialized multi-variable declarations.
    Malformed vec3(Z.z,0,-Z).

Implemented Rules:

    GLSL-COMP-0006
    GLSL-COMP-0007

Disposition:

    Fixed.

Regression Test:

    tests/fixtures/Dark Transit.glsl


=======================================================
GLSL-CASE-0003
=======================================================

Status:
    Rejected for now

Reason:
    Requires iChannel0 image texture
    Requires mipmaps through textureLod()

Safety concern:
    Produces a dead black screen under the current renderer

Future resolution:
    Texture-channel and asset support
    Future .shaver resource manifest
