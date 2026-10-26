//! Isolated, opt-in native libdecor test. Never used by production renderers.
#[cfg(feature = "libdecor-test")]
unsafe extern "C" {
    fn screenshaver_test_libdecor_run() -> libc::c_int;
}

pub(crate) fn run() -> Result<(), String> {
    if std::env::var("XDG_SESSION_TYPE").as_deref() != Ok("wayland") {
        return Err("This diagnostic requires a Wayland desktop session".into());
    }
    #[cfg(feature = "libdecor-test")]
    {
        println!("[LIBDECOR TEST] Starting isolated native test window");
        // SAFETY: the native entry point owns all Wayland resources it creates.
        let status = unsafe { screenshaver_test_libdecor_run() };
        if status == 0 { Ok(()) } else { Err(format!("Native test exited with status {status}")) }
    }
    #[cfg(not(feature = "libdecor-test"))]
    {
        Err("libdecor diagnostic was not compiled in. Build with: cargo build --features libdecor-test".into())
    }
}
