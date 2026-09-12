Screenshaver KDE renderer secondary crate

This crate intentionally builds only the KDE shared renderer library.
It reuses the production Screenshaver source modules through #[path] declarations.

Build:
    cargo build --release --manifest-path kde-renderer/Cargo.toml

Expected library:
    kde-renderer/target/release/libscreenshaver.so

The main Screenshaver Cargo.toml uses autolib = false, so ordinary cargo/Nix
builds remain binary-only and do not build this cdylib unless explicitly requested.
