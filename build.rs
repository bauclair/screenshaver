// Screenshaver build script.
// The libdecor diagnostic is opt-in and does not affect normal builds.
fn main() {
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-lib=GL");

    println!("cargo:rerun-if-changed=src/test_libdecor_native.c");

    if std::env::var_os("CARGO_FEATURE_LIBDECOR_TEST").is_none() {
        return;
    }

    build_libdecor_test();
}

fn build_libdecor_test() {
    // Suppress pkg-config's automatic linker directives so the static C
    // diagnostic archive is placed before its shared-library dependencies.
    let decor = pkg_config::Config::new()
        .cargo_metadata(false)
        .probe("libdecor-0")
        .expect("libdecor-0 development files not found");

    let wayland = pkg_config::Config::new()
        .cargo_metadata(false)
        .probe("wayland-client")
        .expect("wayland-client development files not found");

    let mut compiler = cc::Build::new();
    compiler.file("src/test_libdecor_native.c");

    for path in decor.include_paths.iter().chain(wayland.include_paths.iter()) {
        compiler.include(path);
    }

    compiler.compile("screenshaver_libdecor_test");

    for path in decor.link_paths.iter().chain(wayland.link_paths.iter()) {
        println!("cargo:rustc-link-search=native={}", path.display());
    }

    println!("cargo:rustc-link-lib=dylib=decor-0");
    println!("cargo:rustc-link-lib=dylib=wayland-client");
    println!("cargo:rustc-link-lib=dylib=m");
}
