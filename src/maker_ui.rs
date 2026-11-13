//! Self-contained SDL2 -> egui bridge and Maker UI experiment.
use std::sync::Arc;
use std::time::Instant;
use sdl2::event::Event;
use sdl2::mouse::MouseButton;

pub struct MakerUi {
    context: egui::Context,
    painter: egui_glow::Painter,
    events: Vec<egui::Event>,
    pointer: egui::Pos2,
    modifiers: egui::Modifiers,
    start: Instant,
    frequency: f32,
    color: [f32;3],
    expanded: bool,
    inspector: bool,
    open: bool,
}
impl MakerUi {
    pub fn new(video:&sdl2::VideoSubsystem)->Result<Self,String>{
        let gl=unsafe{glow::Context::from_loader_function(|s|video.gl_get_proc_address(s) as *const _)};
        let painter=egui_glow::Painter::new(Arc::new(gl),"",None,false).map_err(|e|e.to_string())?;
        Ok(Self{context:egui::Context::default(),painter,events:Vec::new(),pointer:egui::Pos2::ZERO,modifiers:egui::Modifiers::default(),start:Instant::now(),frequency:1.,color:[.25,.65,1.],expanded:true,inspector:false,open:true})
    }
    pub fn frequency(&self)->f32{self.frequency}
    pub fn color(&self)->[f32;3]{self.color}
    pub fn handle_event(&mut self,event:&Event){
        match event {
            Event::MouseMotion{x,y,..}=>{self.pointer=egui::pos2(*x as f32,*y as f32);self.events.push(egui::Event::PointerMoved(self.pointer));}
            Event::MouseButtonDown{mouse_btn,x,y,..}|Event::MouseButtonUp{mouse_btn,x,y,..}=>{
                let pressed=matches!(event,Event::MouseButtonDown{..});
                let button=match mouse_btn{MouseButton::Left=>egui::PointerButton::Primary,MouseButton::Right=>egui::PointerButton::Secondary,MouseButton::Middle=>egui::PointerButton::Middle,_=>return};
                self.pointer=egui::pos2(*x as f32,*y as f32);
                self.events.push(egui::Event::PointerButton{pos:self.pointer,button,pressed,modifiers:self.modifiers});
            }
            Event::MouseWheel{x,y,..}=>self.events.push(egui::Event::MouseWheel{unit:egui::MouseWheelUnit::Line,delta:egui::vec2(*x as f32,*y as f32),modifiers:self.modifiers}),
            Event::TextInput{text,..}=>self.events.push(egui::Event::Text(text.clone())),
            Event::KeyDown{keycode:Some(key),keymod,repeat,..}|Event::KeyUp{keycode:Some(key),keymod,repeat,..}=>{
                use sdl2::keyboard::{Keycode,Mod};
                self.modifiers=egui::Modifiers{alt:keymod.intersects(Mod::LALTMOD|Mod::RALTMOD),ctrl:keymod.intersects(Mod::LCTRLMOD|Mod::RCTRLMOD),shift:keymod.intersects(Mod::LSHIFTMOD|Mod::RSHIFTMOD),mac_cmd:false,command:keymod.intersects(Mod::LCTRLMOD|Mod::RCTRLMOD)};
                if *repeat{return;}
                let mapped=match *key{Keycode::Tab=>Some(egui::Key::Tab),Keycode::Enter=>Some(egui::Key::Enter),Keycode::Backspace=>Some(egui::Key::Backspace),Keycode::Delete=>Some(egui::Key::Delete),Keycode::Space=>Some(egui::Key::Space),Keycode::Left=>Some(egui::Key::ArrowLeft),Keycode::Right=>Some(egui::Key::ArrowRight),Keycode::Up=>Some(egui::Key::ArrowUp),Keycode::Down=>Some(egui::Key::ArrowDown),_=>None};
                if let Some(key)=mapped{self.events.push(egui::Event::Key{key,physical_key:None,pressed:matches!(event,Event::KeyDown{..}),repeat:false,modifiers:self.modifiers});}
            }
            _=>{}
        }
    }
    pub fn frame(&mut self,dw:u32,dh:u32,scale:f32)->bool{
        let scale=scale.max(.1);
        let mut input=egui::RawInput::default();
        input.screen_rect=Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(dw as f32/scale,dh as f32/scale)));
        input.viewports.insert(egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(scale),..Default::default()});
        input.time=Some(self.start.elapsed().as_secs_f64());
        input.modifiers=self.modifiers;
        input.events=std::mem::take(&mut self.events);
        let mut frequency=self.frequency;let mut color=self.color;let mut expanded=self.expanded;let mut inspector=self.inspector;let mut open=self.open;
        let output=self.context.run(input,|ctx|{
            egui::TopBottomPanel::top("maker_header").show(ctx,|ui|{ui.horizontal(|ui|{ui.heading("Screenshaver Maker");ui.label("Experimental node and live GLSL preview");if ui.button("Exit").clicked(){open=false;}});});
            egui::SidePanel::left("maker_tools").default_width(290.).show(ctx,|ui|{
                ui.heading("Module graph");ui.label("Prototype: Oscillator → Color → Output");ui.separator();
                ui.horizontal(|ui|{ui.strong("Oscillator");if ui.button(if expanded{"Collapse"}else{"Expand"}).clicked(){expanded=!expanded;}});
                if expanded{ui.add(egui::Slider::new(&mut frequency,.1..=5.).text("Frequency"));ui.color_edit_button_rgb(&mut color);}
                ui.separator();if ui.button("Inspect module").clicked(){inspector=!inspector;}
                if inspector{ui.label("Oscillator (prototype)");ui.monospace("float wave = sin(x * frequency + time);");ui.label("Future: typed ports, nested graphs and module files.");}
                ui.separator();ui.small("This is a UI/GL rendering harness, not yet a graph compiler.");
            });
            egui::Area::new(egui::Id::new("maker_node")).fixed_pos(egui::pos2(390.,140.)).show(ctx,|ui|{
                egui::Frame::window(ui.style()).show(ui,|ui|{ui.strong("Output Color");ui.label("◉ vec3 → Fragment");});
            });
            egui::TopBottomPanel::bottom("maker_footer").show(ctx,|ui|{ui.label("Live preview • Escape to close • No production services started");});
        });
        self.frequency=frequency;self.color=color;self.expanded=expanded;self.inspector=inspector;self.open=open;
        let shapes=self.context.tessellate(output.shapes,output.pixels_per_point);
        self.painter.paint_and_update_textures([dw,dh],output.pixels_per_point,&shapes,&output.textures_delta);
        self.open
    }
    pub fn destroy(&mut self){self.painter.destroy();}
}
