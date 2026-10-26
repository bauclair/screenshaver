fn main() {
    println!("cargo:rerun-if-changed=src/test_libdecor_native.c");
    if std::env::var_os("CARGO_FEATURE_LIBDECOR_TEST").is_none() {
        return;
    }
    let decor = pkg_config::Config::new()
        .probe("libdecor-0")
        .expect("libdecor-test requires libdecor-0 development files");
    let wayland = pkg_config::Config::new()
        .probe("wayland-client")
        .expect("libdecor-test requires wayland-client development files");
    let mut compiler = cc::Build::new();
    compiler.file("src/test_libdecor_native.c")
        .flag_if_supported("-std=c11")
        .define("_POSIX_C_SOURCE", "200809L")
        .warnings(true);
    for path in decor.include_paths.iter().chain(wayland.include_paths.iter()) {
        compiler.include(path);
    }
    compiler.compile("screenshaver_libdecor_test");
}
