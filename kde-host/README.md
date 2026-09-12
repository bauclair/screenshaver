# Screenshaver KDE Host

This directory contains the Qt 6 / QSGRenderNode host used to embed the
production Screenshaver `FrameRenderEngine` inside KDE Plasma's KScreenLocker
scene graph.

It is intentionally separate from `kde-renderer/`:

- `kde-renderer/` builds the Rust `libscreenshaver.so` cdylib.
- `kde-host/` builds the Qt Quick QML plugin that owns the QSGRenderNode and
  calls the Rust C ABI while Qt's OpenGL context is current.

The host does not implement a second shader pipeline. All shader loading,
preprocessing, policy handling, timing, textures, palettes, and postprocessing
remain in the Rust `FrameRenderEngine`.

## Debug build

From the Screenshaver project root:

```bash
cargo build --manifest-path kde-renderer/Cargo.toml
./kde-host/build_kde_host.sh debug
```

## Release build

```bash
cargo build --release --manifest-path kde-renderer/Cargo.toml
./kde-host/build_kde_host.sh release
```

The resulting QML module is placed under:

```text
kde-host/build-<profile>/qml/ScreenshaverNativeGL/
```

It contains the Qt plugin, `qmldir`, and a colocated copy of
`libscreenshaver.so`. The plugin uses `$ORIGIN` runtime lookup so the renderer
can be loaded from that directory.

This component alone does not install or activate a lock screen. Production
KScreenLocker lifecycle installation is handled separately so the safe KDE
shell-overlay mechanism can be introduced and tested independently.
