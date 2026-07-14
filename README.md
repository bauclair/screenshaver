# Screenshaver
<img width="128" height="128" alt="screenshaver" src="https://github.com/user-attachments/assets/9ecad862-dbc8-446c-a3e4-48be86738df8" />

## *Screensavers, reinvented.*

Screenshaver is a modern, ShaderToy-compatible screensaver for Linux that brings GPU-accelerated procedural graphics to the desktop.

Rather than displaying static images or pre-rendered animations, Screenshaver renders visual effects in real time using OpenGL fragment shaders. Every frame is generated live by your GPU, creating an effectively limitless variety of animated imagery.

## Why Screenshaver?

Screenshaver was created with a simple philosophy:

> A screensaver should be more than something that prevents screen burn-in.
> It should be a living gallery of procedural art.

Screenshaver is designed to make discovering, rendering, and sharing shader-based artwork easy while remaining highly configurable and friendly to both casual users and developers.

## Features

- ShaderToy-compatible shader rendering
- Automatic GLSL preprocessing and compatibility pipeline
- Native Wayland support
- Full-screen GPU-accelerated rendering
- Configurable idle activation
- Automatic exit on mouse or keyboard activity
- Deterministic procedural texture generation
- Multiple built-in texture families
- Multiple color palettes
- Shader-specific texture overrides
- Extensive runtime logging
- Developer-friendly command-line tools
- Designed specifically for Linux

## Procedural Textures

Many shaders require texture inputs.

Instead of relying on external image files, Screenshaver can generate textures procedurally at runtime.

Current texture families include:

- Clouds
- Marble
- Cellular
- Minerals
- Fabric (planned)
- Julia (planned)

## Designed for Exploration

Screenshaver is intended to be enjoyable outside of normal screensaver use.

Built-in preview commands allow users to experiment with procedural textures, palettes, and shaders, making it easy to discover interesting combinations before using them as part of a screensaver configuration.

## Open Source

Screenshaver is an open-source project built in Rust.

The project emphasizes:

- clean architecture
- deterministic rendering
- modular design
- high performance
- maintainable code
- extensive documentation

## Status

Screenshaver is currently under active development.

Features, command-line options, configuration syntax, and procedural texture algorithms may evolve before the first stable release.

## Contributing

Bug reports, feature requests, testing, and pull requests are always welcome.

Community feedback plays an important role in helping Screenshaver grow into the best procedural screensaver platform available for Linux.

---

**Screenshaver**

*Screensavers, reinvented.*
