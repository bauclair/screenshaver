//! Screenshaver Maker: typed node-graph workspace (Batch 03).
use std::{sync::Arc, time::Instant};
use sdl2::{event::Event, mouse::MouseButton};

use crate::maker_graph::{Graph, Kind, NodeId};

pub struct MakerUi {
    context: egui::Context,
    painter: egui_glow::Painter,
    events: Vec<egui::Event>,
    pointer: egui::Pos2,
    modifiers: egui::Modifiers,
    start: Instant,
    graph: Graph,
    selected: Option<NodeId>,
    pending_source: Option<(NodeId, usize)>,
    dragging_wire: Option<(NodeId, usize)>,

    status: String,
    open: bool,
}
impl MakerUi {
    pub fn new(video:&sdl2::VideoSubsystem)->Result<Self,String>{
        let gl=unsafe{glow::Context::from_loader_function(|s|video.gl_get_proc_address(s) as *const _)};
        let painter=egui_glow::Painter::new(Arc::new(gl),"",None,false).map_err(|e|e.to_string())?;
        let graph=Graph::demo();let selected=graph.nodes.iter().find(|n|n.kind==Kind::Palette).map(|n|n.id);
        Ok(Self{context:egui::Context::default(),painter,events:Vec::new(),pointer:egui::Pos2::ZERO,
            modifiers:egui::Modifiers::default(),start:Instant::now(),graph,selected,pending_source:None,dragging_wire:None,
            status:"Click an output port, then a compatible input port".into(),open:true})
    }
    pub fn graph(&self)->&Graph{&self.graph}
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
    pub fn frame(&mut self,dw:u32,dh:u32,scale:f32,compile_error:Option<&str>)->bool{
        let scale=scale.max(0.1);
        let mut input=egui::RawInput::default();
        input.screen_rect=Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(dw as f32/scale,dh as f32/scale)));
        input.viewports.insert(egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(scale),..Default::default()});
        input.time=Some(self.start.elapsed().as_secs_f64());input.modifiers=self.modifiers;
        input.events=std::mem::take(&mut self.events);
        let graph=&mut self.graph;
        let selected=&mut self.selected;
        let pending=&mut self.pending_source;
        let dragging_wire=&mut self.dragging_wire;
        let status=&mut self.status;
        let open=&mut self.open;
        let output=self.context.run(input,|ctx|{
            egui::TopBottomPanel::top("maker_header").show(ctx,|ui|{
                ui.horizontal(|ui|{
                    ui.heading("Screenshaver Maker");ui.separator();ui.label("Untitled graph");
                    if ui.button("Reset demo").clicked(){let revision=graph.revision;*graph=Graph::demo();graph.revision=revision+1;
                        *selected=graph.nodes.iter().find(|n|n.kind==Kind::Palette).map(|n|n.id);*pending=None;*dragging_wire=None;*status="Demo graph reset".into();
                    }
                    if ui.button("Exit").clicked(){*open=false;}
                });
            });
            egui::TopBottomPanel::bottom("maker_footer").show(ctx,|ui|{
                ui.horizontal_wrapped(|ui|{
                    ui.label("Batch 04 · drag-to-connect + typed graph");ui.separator();
                    ui.label(format!("{} nodes / {} connections",graph.nodes.len(),graph.connections.len()));ui.separator();
                    if let Some(e)=compile_error{ui.colored_label(egui::Color32::LIGHT_RED,format!("GLSL: {}",e));}
                    else{ui.label(status.as_str());}
                });
            });
            egui::SidePanel::left("maker_library").resizable(true).default_width(190.0).min_width(155.0).show(ctx,|ui|{
                ui.heading("Module Library");ui.separator();
                ui.small("Add nodes to the graph:");
                for kind in Kind::LIBRARY{
                    if kind==Kind::Output && graph.nodes.iter().any(|n|n.kind==Kind::Output){continue}
                    if ui.button(format!("+ {}",kind.title())).clicked(){
                        let count=graph.nodes.len();
                        let id=graph.add(kind,[80.0+(count%3) as f32*45.0,180.0+(count%4) as f32*42.0]);
                        *selected=Some(id);*status=format!("Added {} #{}",kind.title(),id);
                    }
                }
                ui.separator();ui.small("Drag from an output port to a compatible input, or click output then input. Outputs support fan-out; each input has one source.");
                if let Some((id,port))=*pending{ui.label(format!("Source: node #{id}, port {port}"));if ui.button("Cancel connection").clicked(){*pending=None;*dragging_wire=None;}}
            });
            egui::SidePanel::right("maker_inspector").resizable(true).default_width(260.0).min_width(200.0).show(ctx,|ui|{
                ui.heading("Inspector");ui.separator();
                if let Some(id)=*selected{
                    if let Some(n)=graph.node_mut(id){
                        ui.strong(format!("{} #{}",n.kind.title(),id));ui.separator();
                        edit_parameters(ui,n);
                        ui.separator();ui.checkbox(&mut n.expanded,"Expanded node");
                    }
                    if ui.button("Delete selected node").clicked(){match graph.remove(id){Ok(())=>{*selected=None;*pending=None;*status=format!("Deleted node #{id}")},Err(e)=>*status=e}}
                }else{ui.label("Select a node to edit its parameters.");}
                ui.separator();ui.label("Graph status");
                match graph.validate(){Ok(())=>{ui.small("Typed connections valid; no cycles");},Err(e)=>{ui.colored_label(egui::Color32::LIGHT_RED,e);}}
            });
            egui::TopBottomPanel::bottom("maker_live_preview").resizable(true).default_height(215.0).min_height(120.0)
                .frame(egui::Frame::default().fill(egui::Color32::from_rgba_unmultiplied(10,14,20,42)))
                .show(ctx,|ui|{ui.strong("Live Shader Preview");ui.small("Graph-generated GLSL 330 core · preview retains last valid program on errors");});
            egui::CentralPanel::default().frame(egui::Frame::default().fill(egui::Color32::from_rgba_unmultiplied(12,16,25,228))).show(ctx,|ui|{
                ui.heading("Node Graph");ui.small("Drag from output to input to wire · click-to-connect also works · drag node headers to move");ui.separator();
                let canvas=ui.available_rect_before_wrap();let origin=canvas.min;
                let mut input_ports=std::collections::HashMap::new();
                let mut output_ports=std::collections::HashMap::new();
                let ids:Vec<NodeId>=graph.nodes.iter().map(|n|n.id).collect();
                for id in ids{
                    let Some(node)=graph.node(id) else{continue};
                    let kind=node.kind;let pos=node.pos;
                    let area=egui::Area::new(egui::Id::new(("maker_node",id)))
                        .order(egui::Order::Foreground).movable(false)
                        .current_pos(origin+egui::vec2(pos[0],pos[1]))
                        .show(ctx,|ui|{
                            egui::Frame::window(ui.style()).show(ui,|ui|{
                                ui.set_min_width(170.0);
                                ui.horizontal(|ui|{
                                    let header = ui.add(egui::Label::new(format!("{} #{}",kind.title(),id))
                                        .sense(egui::Sense::click_and_drag()))
                                        .on_hover_text("Drag this header to move the node");
                                    if header.clicked() { *selected = Some(id); }
                                    if header.dragged() {
                                        if let Some(n) = graph.node_mut(id) {
                                            let delta = ui.ctx().input(|i| i.pointer.delta());
                                            n.pos = [pos[0] + delta.x, pos[1] + delta.y];
                                        }
                                        *selected = Some(id);
                                    }
                                    if let Some(n)=graph.node_mut(id){if ui.small_button(if n.expanded{"−"}else{"+"}).clicked(){n.expanded=!n.expanded;}}
                                });
                                if graph.node(id).is_some_and(|n|n.expanded){if let Some(n)=graph.node_mut(id){edit_parameters(ui,n);}}
                                for (port,(name,typ)) in kind.inputs().iter().enumerate(){
                                    ui.horizontal(|ui|{
                                        let connected=graph.input_connection(id,port).is_some();
                                        let response = ui.add(egui::Button::new(if connected { "● in" } else { "○ in" })
                                            .sense(egui::Sense::click_and_drag()));
                                        input_ports.insert((id,port),response.rect.center());
                                        let hint = port_hint(graph, id, port, *pending, *dragging_wire);
                                        let response = response.on_hover_text(hint);
                                        if response.clicked() && dragging_wire.is_none() {
                                            if let Some((from,from_port))=pending.take(){
                                                match graph.connect(from,from_port,id,port){
                                                    Ok(())=>*status=format!("Connected #{from}:{from_port} to #{id}:{port}"),
                                                    Err(e)=>*status=format!("Connection rejected: {e}")
                                                }
                                            }else{graph.disconnect_input(id,port);*status=format!("Disconnected input #{id}:{port}");}
                                            *selected=Some(id);
                                        }
                                        ui.small(format!("{}: {}",name,typ.name()));
                                    });
                                }
                                for (port,(name,typ)) in kind.outputs().iter().enumerate(){
                                    ui.horizontal(|ui|{
                                        ui.small(format!("{}: {}",name,typ.name()));
                                        let active=*pending==Some((id,port));
                                        let response = ui.add(egui::Button::new(if active { "◉ out" } else { "○ out" })
                                            .sense(egui::Sense::click_and_drag()));
                                        output_ports.insert((id,port),response.rect.center());
                                        let response = response.on_hover_text(format!(
                                            "{} output ({})\nConnect to any number of compatible inputs.\nClick to select, or drag to an input.", name, typ.name()));
                                        if response.drag_started() {
                                            *dragging_wire = Some((id, port));
                                            *pending = None;
                                            *selected = Some(id);
                                            *status = format!("Drag {} output to a matching input", typ.name());
                                        }
                                        if response.on_hover_text("Click to select source, or drag to an input").clicked(){
                                            *pending=if active{None}else{Some((id,port))};*selected=Some(id);
                                            *status=if active{"Connection cancelled".into()}else{format!("Select a {} input",typ.name())};
                                        }
                                    });
                                }
                            });
                        });
                    let _ = area; // Only the header is draggable; ports never move nodes.
                }
                // Resolve a wire drop after collecting all port rectangles. This also
                // supports dropping on a port in a node drawn earlier in the frame.
                let pointer_pos = ctx.input(|i| i.pointer.interact_pos());
                let pointer_released = ctx.input(|i| i.pointer.any_released());
                if let Some((from, from_port)) = *dragging_wire {
                    if pointer_released {
                        let target = pointer_pos.and_then(|p| {
                            input_ports.iter().find_map(|(&(id, port), &center)| {
                                // The visible port is a small button, not just its center.
                                let hit = egui::Rect::from_center_size(center, egui::vec2(58.0, 28.0));
                                hit.contains(p).then_some((id, port))
                            })
                        });
                        if let Some((to, to_port)) = target {
                            match graph.connect(from, from_port, to, to_port) {
                                Ok(()) => *status = format!("Connected #{from}:{from_port} → #{to}:{to_port}"),
                                Err(error) => *status = format!("Connection rejected: {error}"),
                            }
                            *selected = Some(to);
                        } else {
                            *status = "Wire drag cancelled (release over an input port)".into();
                        }
                        *dragging_wire = None;
                    }
                }
                let painter=ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground,egui::Id::new("maker_wire_layer")));
                for c in &graph.connections{
                    if let (Some(&a),Some(&b))=(output_ports.get(&(c.from,c.from_port)),input_ports.get(&(c.to,c.to_port))){
                        let points = wire_points(a, b);
                        painter.add(egui::Shape::line(points,egui::Stroke::new(2.5,egui::Color32::from_rgb(80,205,230))));
                    }
                }
                // Show destination compatibility while dragging or after selecting a source.
                if let Some((from, from_port)) = (*dragging_wire).or(*pending) {
                    for (&(to, to_port), &center) in &input_ports {
                        let compatible = graph.check_connection(from, from_port, to, to_port).is_ok();
                        let color = if compatible { egui::Color32::GREEN } else { egui::Color32::RED };
                        painter.circle_stroke(center, 10.0, egui::Stroke::new(1.5, color));
                    }
                }
                // Live wire follows the pointer while the source port is dragged.
                if let Some((id, port)) = *dragging_wire {
                    if let (Some(&start), Some(end)) = (output_ports.get(&(id, port)), pointer_pos) {
                        painter.add(egui::Shape::line(
                            wire_points(start, end),
                            egui::Stroke::new(2.5, egui::Color32::from_rgb(255, 196, 92)),
                        ));
                    }
                }
                ui.allocate_rect(canvas,egui::Sense::hover());
            });
        });
        let shapes=self.context.tessellate(output.shapes,output.pixels_per_point);
        self.painter.paint_and_update_textures([dw,dh],output.pixels_per_point,&shapes,&output.textures_delta);
        self.open
    }
    pub fn destroy(&mut self){self.painter.destroy();}
}
fn edit_parameters(ui:&mut egui::Ui,node:&mut crate::maker_graph::Node){
    match node.kind{
        Kind::Uv=>{ui.small("Normalized screen coordinates (UV2)");}
        Kind::Noise=>{ui.add(egui::Slider::new(&mut node.scale,0.1..=20.0).text("Scale"));}
        Kind::Multiply=>{ui.add(egui::Slider::new(&mut node.factor,0.0..=4.0).text("Factor"));}
        Kind::Palette=>{ui.add(egui::Slider::new(&mut node.frequency,0.1..=5.0).text("Frequency"));ui.color_edit_button_rgb(&mut node.color);}
        Kind::Output=>{ui.small("Final fragment color (Color3)");}
    }
}

/// Cubic Bezier wire with stable curvature for both left-to-right and reversed nodes.
fn wire_points(a: egui::Pos2, b: egui::Pos2) -> Vec<egui::Pos2> {
    let dx = ((b.x - a.x).abs() * 0.5).max(35.0);
    (0..=24).map(|i| {
        let t = i as f32 / 24.0;
        let u = 1.0 - t;
        let p = a.to_vec2() * (u * u * u)
            + (a + egui::vec2(dx, 0.0)).to_vec2() * (3.0 * u * u * t)
            + (b - egui::vec2(dx, 0.0)).to_vec2() * (3.0 * u * t * t)
            + b.to_vec2() * (t * t * t);
        egui::pos2(p.x, p.y)
    }).collect()
}

/// Explain the port contract and the exact validation outcome for the selected source.
fn port_hint(graph: &Graph, to: NodeId, to_port: usize,
    pending: Option<(NodeId, usize)>, dragging: Option<(NodeId, usize)>) -> String {
    let Some(node) = graph.node(to) else { return "Destination node missing".into() };
    let Some(&(name, ty)) = node.kind.inputs().get(to_port) else { return "Input port missing".into() };
    let occupied = graph.input_connection(to, to_port).is_some();
    let mut hint = format!("{} input ({})\nOne incoming connection; a new source replaces the old one.", name, ty.name());
    if occupied { hint.push_str("\nCurrently connected."); }
    if let Some((from, from_port)) = dragging.or(pending) {
        match graph.check_connection(from, from_port, to, to_port) {
            Ok(()) => hint.push_str("\nCompatible: connect here."),
            Err(reason) => hint.push_str(&format!("\nCannot connect: {reason}")),
        }
    } else {
        hint.push_str("\nClick a source output first; click this input without a source to disconnect.");
    }
    hint
}
