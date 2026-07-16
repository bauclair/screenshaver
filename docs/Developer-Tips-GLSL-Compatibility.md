# Developer Tips: Best Practices for Converting GLSL Shader Files for Use with Screenshaver

## Purpose

This document summarizes the compatibility practices developed during
Screenshaver development.

## Best Practices

### Preserve Original Shaders

-   Keep downloaded `.glsl` files unchanged whenever possible.
-   Prefer fixing compatibility in the preprocessor.

### Version Directives

-   Do not include `#version`; Screenshaver injects the correct version.

### Entry Point

-   Continue using `mainImage(out vec4 fragColor, in vec2 fragCoord)`.
-   Let Screenshaver generate `main()`.

### Initialize Output

Always begin `mainImage()` with:

``` glsl
fragColor = vec4(0.0);
```

### Initialize Variables

Always initialize local variables before use.

Good:

``` glsl
float value = 0.0;
vec3 color = vec3(0.0);
```

Avoid:

``` glsl
vec3 color;
color += texture(iChannel0, uv).rgb;
```

### Texture Channels

Continue using ShaderToy conventions: - `iChannel0`--`iChannel3` -
`texture()` - `textureLod()`

Screenshaver automatically binds procedural textures when required.

Texture selection hierarchy: 1. Command-line preview overrides 2.
`[[texture_override]]` 3. `global_texture` / `global_palette` 4. Random
selection

### textureLod()

-   Supported with generated mipmaps.
-   Do not reject shaders solely because they use `textureLod()`.

### Floating Point

Prefer explicit floating-point constants (`1.0`, `0.5`) over implicit
conversions.

### Standard Uniforms

Use: - `iTime` - `iTimeDelta` - `iResolution` - `iMouse` - `iFrame` -
`iChannel0`--`iChannel3`

### Resolution Independence

Use:

``` glsl
uv = fragCoord.xy / iResolution.xy;
```

Avoid hard-coded screen dimensions.

### Multi-pass Shaders

Do not flatten multipass shaders into a single GLSL file. Future support
should preserve the render graph in a `.shaver` container.

### Cache Management

After changing preprocessing rules:

    screenshaver --delete-cache

### Recommended Debug Workflow

1.  Preview the shader.
2.  Enable `debug_log = true`.
3.  Inspect the runtime log.
4.  Delete the cache after preprocessor changes.
5.  Retest.

## Philosophy

-   Preserve original shaders.
-   Make compatibility improvements globally.
-   Minimize shader-specific edits.
-   Build reusable preprocessing rules.
