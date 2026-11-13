//! Screenshaver Maker: isolated SDL2/egui node workspace (Batch 02).
use std::{sync::Arc, time::Instant};
use sdl2::{event::Event, mouse::MouseButton};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Selected { Palette, Output }

pub struct MakerUi {
    context: egui::Context,
    painter: egui_glow::Painter,
    events: Vec<egui::Event>,
    pointer: egui::Pos2,
    modifiers: egui::Modifiers,
    start: Instant,
    frequency: f32,
    color: [f32; 3],
    expanded: [bool; 2],
    selected: Selected,
    connected: bool,
    open: bool,
}
impl MakerUi {
    pub fn new(video: &sdl2::VideoSubsystem) -> Result<Self, String> {
        let gl = unsafe { glow::Context::from_loader_function(|s| video.gl_get_proc_address(s) as *const _) };
        let painter = egui_glow::Painter::new(Arc::new(gl), "", None, false).map_err(|e| e.to_string())?;
        Ok(Self { context: egui::Context::default(), painter, events: Vec::new(),
            pointer: egui::Pos2::ZERO, modifiers: egui::Modifiers::default(), start: Instant::now(),
            frequency: 1.0, color: [0.25, 0.65, 1.0], expanded: [true, true],
            selected: Selected::Palette, connected: true, open: true })
    }
    pub fn frequency(&self) -> f32 { self.frequency }
    pub fn color(&self) -> [f32; 3] { self.color }
    pub fn connected(&self) -> bool { self.connected }
    pub fn handle_event(&mut self, event: &Event) {
        match event {
            Event::MouseMotion { x, y, .. } => {
                self.pointer = egui::pos2(*x as f32, *y as f32);
                self.events.push(egui::Event::PointerMoved(self.pointer));
            }
            Event::MouseButtonDown { mouse_btn, x, y, .. } | Event::MouseButtonUp { mouse_btn, x, y, .. } => {
                let pressed = matches!(event, Event::MouseButtonDown { .. });
                let button = match mouse_btn { MouseButton::Left => egui::PointerButton::Primary,
                    MouseButton::Right => egui::PointerButton::Secondary,
                    MouseButton::Middle => egui::PointerButton::Middle, _ => return };
                self.pointer = egui::pos2(*x as f32, *y as f32);
                self.events.push(egui::Event::PointerButton { pos: self.pointer, button, pressed, modifiers: self.modifiers });
            }
            Event::MouseWheel { x, y, .. } => self.events.push(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line, delta: egui::vec2(*x as f32, *y as f32), modifiers: self.modifiers }),
            Event::TextInput { text, .. } => self.events.push(egui::Event::Text(text.clone())),
            Event::KeyDown { keycode: Some(key), keymod, repeat, .. } |
            Event::KeyUp { keycode: Some(key), keymod, repeat, .. } => {
                use sdl2::keyboard::{Keycode, Mod};
                self.modifiers = egui::Modifiers {
                    alt: keymod.intersects(Mod::LALTMOD | Mod::RALTMOD),
                    ctrl: keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD),
                    shift: keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD),
                    mac_cmd: false, command: keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD),
                };
                if *repeat { return; }
                let mapped = match *key {
                    Keycode::Tab => Some(egui::Key::Tab), Keycode::Return | Keycode::KpEnter => Some(egui::Key::Enter),
                    Keycode::Escape => Some(egui::Key::Escape), Keycode::Backspace => Some(egui::Key::Backspace),
                    Keycode::Delete => Some(egui::Key::Delete), Keycode::Space => Some(egui::Key::Space),
                    Keycode::Left => Some(egui::Key::ArrowLeft), Keycode::Right => Some(egui::Key::ArrowRight),
                    Keycode::Up => Some(egui::Key::ArrowUp), Keycode::Down => Some(egui::Key::ArrowDown), _ => None,
                };
                if let Some(key) = mapped { self.events.push(egui::Event::Key {
                    key, physical_key: None, pressed: matches!(event, Event::KeyDown { .. }),
                    repeat: false, modifiers: self.modifiers }); }
            }
            _ => {}
        }
    }
    pub fn frame(&mut self, dw: u32, dh: u32, scale: f32) -> bool {
        let scale = scale.max(0.1);
        let mut input = egui::RawInput::default();
        input.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO,
            egui::vec2(dw as f32 / scale, dh as f32 / scale)));
        input.viewports.insert(egui::ViewportId::ROOT, egui::ViewportInfo {
            native_pixels_per_point: Some(scale), ..Default::default() });
        input.time = Some(self.start.elapsed().as_secs_f64());
        input.modifiers = self.modifiers;
        // SDL coordinates are window-logical; egui coordinates are points.
        // Correct for high-DPI window scaling without changing SDL's logical mouse positions.
        input.events = std::mem::take(&mut self.events);
        let mut frequency = self.frequency;
        let mut color = self.color;
        let mut expanded = self.expanded;
        let mut selected = self.selected;
        let mut connected = self.connected;
        let mut open = self.open;
        let output = self.context.run(input, |ctx| {
            egui::TopBottomPanel::top("maker_header").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Screenshaver Maker");
                    ui.separator(); ui.label("Untitled graph");
                    if ui.button("Reset connection").clicked() { connected = true; }
                    if ui.button("Exit").clicked() { open = false; }
                });
            });
            egui::TopBottomPanel::bottom("maker_footer").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Batch 02 · interactive graph workspace");
                    ui.separator();
                    ui.label(if connected { "Color3 connection: valid" } else { "Color3 connection: disconnected" });
                    ui.separator(); ui.label("Drag node headers · Click ports to connect/disconnect");
                });
            });
            egui::SidePanel::left("maker_library").resizable(true).default_width(195.0)
                .min_width(150.0).show(ctx, |ui| {
                    ui.heading("Module Library"); ui.separator();
                    ui.label("Color & Palettes");
                    if ui.selectable_label(selected == Selected::Palette, "Cosine Palette").clicked() { selected = Selected::Palette; }
                    ui.add_space(6.0); ui.label("Output");
                    if ui.selectable_label(selected == Selected::Output, "Output Color").clicked() { selected = Selected::Output; }
                    ui.separator(); ui.small("This first graph uses two built-in nodes. More modules and module files follow in later batches.");
                });
            egui::SidePanel::right("maker_inspector").resizable(true).default_width(245.0)
                .min_width(195.0).show(ctx, |ui| {
                    ui.heading("Inspector"); ui.separator();
                    match selected {
                        Selected::Palette => {
                            ui.strong("Cosine Palette"); ui.small("Output: Color3");
                            ui.add(egui::Slider::new(&mut frequency, 0.1..=5.0).text("Frequency"));
                            ui.label("Color"); ui.color_edit_button_rgb(&mut color);
                            ui.small("Controls update the live preview without recompiling GLSL.");
                        }
                        Selected::Output => {
                            ui.strong("Output Color"); ui.small("Input: Color3");
                            ui.checkbox(&mut connected, "Connected to Cosine Palette");
                            ui.small("Disconnected output renders a neutral dark preview.");
                        }
                    }
                });
            egui::CentralPanel::default().frame(egui::Frame::default()
                .fill(egui::Color32::from_rgba_unmultiplied(12, 16, 25, 228))).show(ctx, |ui| {
                    ui.heading("Node Graph");
                    ui.small("Drag node headers to reposition. Use the library or node title to select.");
                    ui.separator();
                    let origin = ui.min_rect().min;
                    let canvas = ui.available_rect_before_wrap();
                    // Persistent positions are managed by egui Area memory, not reset each frame.
                    let a = egui::Area::new(egui::Id::new("maker_palette_node"))
                        .order(egui::Order::Foreground).movable(true)
                        .default_pos(origin + egui::vec2(30.0, 115.0))
                        .show(ctx, |ui| {
                            egui::Frame::window(ui.style()).show(ui, |ui| {
                                ui.set_min_width(185.0);
                                ui.horizontal(|ui| {
                                    if ui.selectable_label(selected == Selected::Palette, "Cosine Palette").clicked() { selected = Selected::Palette; }
                                    if ui.small_button(if expanded[0] { "−" } else { "+" }).clicked() { expanded[0] = !expanded[0]; }
                                });
                                if expanded[0] {
                                    ui.add(egui::Slider::new(&mut frequency, 0.1..=5.0).text("Freq"));
                                    ui.color_edit_button_rgb(&mut color);
                                }
                                ui.horizontal(|ui| {
                                    ui.label("Color3");
                                    if ui.small_button("● out").on_hover_text("Toggle Color3 connection").clicked() { connected = !connected; }
                                });
                            });
                        });
                    let b = egui::Area::new(egui::Id::new("maker_output_node"))
                        .order(egui::Order::Foreground).movable(true)
                        .default_pos(origin + egui::vec2(295.0, 190.0))
                        .show(ctx, |ui| {
                            egui::Frame::window(ui.style()).show(ui, |ui| {
                                ui.set_min_width(165.0);
                                ui.horizontal(|ui| {
                                    if ui.selectable_label(selected == Selected::Output, "Output Color").clicked() { selected = Selected::Output; }
                                    if ui.small_button(if expanded[1] { "−" } else { "+" }).clicked() { expanded[1] = !expanded[1]; }
                                });
                                if expanded[1] { ui.label("Fragment output"); }
                                ui.horizontal(|ui| {
                                    if ui.small_button("● in").on_hover_text("Toggle Color3 connection").clicked() { connected = !connected; }
                                    ui.label("Color3");
                                });
                            });
                        });
                    if connected {
                        let from = egui::pos2(a.response.rect.right(), a.response.rect.center().y);
                        let to = egui::pos2(b.response.rect.left(), b.response.rect.center().y);
                        let dx = ((to.x - from.x).abs() * 0.5).max(35.0);
                        let mut points = Vec::with_capacity(25);
                        for i in 0..=24 {
                            let t = i as f32 / 24.0; let u = 1.0 - t;
                            let p = from.to_vec2() * (u*u*u)
                                + (from + egui::vec2(dx, 0.0)).to_vec2() * (3.0*u*u*t)
                                + (to - egui::vec2(dx, 0.0)).to_vec2() * (3.0*u*t*t)
                                + to.to_vec2() * (t*t*t);
                            points.push(egui::pos2(p.x, p.y));
                        }
                        ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground,
                            egui::Id::new("maker_wire_layer"))).add(egui::Shape::line(points,
                            egui::Stroke::new(2.5, egui::Color32::from_rgb(80, 205, 230))));
                    }
                    ui.allocate_rect(canvas, egui::Sense::hover());
                });
        });
        self.frequency = frequency; self.color = color; self.expanded = expanded;
        self.selected = selected; self.connected = connected; self.open = open;
        let shapes = self.context.tessellate(output.shapes, output.pixels_per_point);
        self.painter.paint_and_update_textures([dw, dh], output.pixels_per_point,
            &shapes, &output.textures_delta);
        self.open
    }
    pub fn destroy(&mut self) { self.painter.destroy(); }
}
