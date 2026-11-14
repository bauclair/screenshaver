//! Standalone Maker: isolated SDL2/OpenGL runtime, no Screenshaver services.
use sdl2::video::GLProfile;
use std::time::{Duration,Instant};
pub fn run()->Result<(),String>{
    let sdl=sdl2::init()?;let video=sdl.video()?;
    {let a=video.gl_attr();a.set_context_profile(GLProfile::Core);a.set_context_version(3,3);a.set_double_buffer(true);}
    let window=video.window("Screenshaver Maker — graph prototype",1440,850).position_centered().resizable().opengl().build().map_err(|e|e.to_string())?;
    let _gl_context=window.gl_create_context()?;
    let _=video.gl_set_swap_interval(sdl2::video::SwapInterval::VSync);
    gl::load_with(|s|video.gl_get_proc_address(s) as *const _);
    let mut ui=crate::maker_ui::MakerUi::new(&video)?;
    let mut preview=crate::maker_preview::Preview::new()?;
    let mut events=sdl.event_pump()?;let start=Instant::now();
    'running:loop{
        for event in events.poll_iter(){
            if matches!(event,sdl2::event::Event::Quit{..}){break 'running}
            if let sdl2::event::Event::KeyDown{keycode:Some(sdl2::keyboard::Keycode::Escape),..}=event{break 'running}
            ui.handle_event(&event);
        }
        let (dw, dh) = window.drawable_size();
        let (ww, wh) = window.size();
        let scale_x = if ww > 0 { dw as f32 / ww as f32 } else { 1.0 };
        let scale_y = if wh > 0 { dh as f32 / wh as f32 } else { 1.0 };
        let scale = scale_x.min(scale_y);
        preview.update(ui.graph());
        preview.render(dw as i32,dh as i32,scale,ui.preview_rect(),start.elapsed().as_secs_f32(),ui.graph());
        if !ui.frame(dw,dh,scale,preview.error.as_deref()){break}
        window.gl_swap_window();std::thread::sleep(Duration::from_millis(8));
    }
    ui.destroy();preview.destroy();Ok(())
}
