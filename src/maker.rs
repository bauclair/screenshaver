//! Standalone Maker experimental entry point. No database or resident services.
use sdl2::video::GLProfile;
use std::time::{Duration, Instant};

pub fn run() -> Result<(), String> {
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    {
        let attr = video.gl_attr();
        attr.set_context_profile(GLProfile::Core);
        attr.set_context_version(3, 3);
        attr.set_double_buffer(true);
    }
    let window = video.window("Screenshaver Maker — experimental", 1280, 800)
        .position_centered().resizable().opengl().build().map_err(|e| e.to_string())?;
    let _context = window.gl_create_context()?;
    let _ = video.gl_set_swap_interval(sdl2::video::SwapInterval::VSync);
    gl::load_with(|name| video.gl_get_proc_address(name) as *const _);
    let mut ui = crate::maker_ui::MakerUi::new(&video)?;
    let mut preview = crate::maker_preview::Preview::new()?;
    let mut pump = sdl.event_pump()?;
    let start = Instant::now();
    'running: loop {
        for event in pump.poll_iter() {
            if matches!(event, sdl2::event::Event::Quit { .. }) { break 'running; }
            if let sdl2::event::Event::KeyDown { keycode: Some(sdl2::keyboard::Keycode::Escape), .. } = event {
                break 'running;
            }
            ui.handle_event(&event);
        }
        let (dw, dh) = window.drawable_size();
        let (ww, wh) = window.size();
        let scale = if ww > 0 { dw as f32 / ww as f32 } else { 1.0 };
        preview.render(dw as i32, dh as i32, start.elapsed().as_secs_f32(), ui.frequency(), ui.color())?;
        if !ui.frame(dw, dh, scale) { break; }
        window.gl_swap_window();
        std::thread::sleep(Duration::from_millis(8));
    }
    ui.destroy();
    preview.destroy();
    Ok(())
}
