fn main() {
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-lib=GL");

    println!("cargo:rerun-if-changed=src/test_libdecor_native.c");

    if std::env::var_os("CARGO_FEATURE_LIBDECOR_TEST").is_none() {
        return;
    }

    let decor = pkg_config::Config::new()
        .probe("libdecor-0")
        .expect("libdecor-0 development files not found");

    let wayland = pkg_config::Config::new()
        .probe("wayland-client")
        .expect("wayland-client development files not found");

    let mut compiler = cc::Build::new();
    compiler.file("src/test_libdecor_native.c");

    for path in decor
        .include_paths
        .iter()
        .chain(wayland.include_paths.iter())
    {
        compiler.include(path);
    }

    compiler.compile("screenshaver_libdecor_test");
}