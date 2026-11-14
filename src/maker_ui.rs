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
    preview_rect: egui::Rect,
}
impl MakerUi {
    pub fn new(video:&sdl2::VideoSubsystem)->Result<Self,String>{
        let gl=unsafe{glow::Context::from_loader_function(|s|video.gl_get_proc_address(s) as *const _)};
        let painter=egui_glow::Painter::new(Arc::new(gl),"",None,false).map_err(|e|e.to_string())?;
        let graph=Graph::demo();let selected=graph.nodes.iter().find(|n|n.kind==Kind::Palette).map(|n|n.id);
        Ok(Self{context:egui::Context::default(),painter,events:Vec::new(),pointer:egui::Pos2::ZERO,
            modifiers:egui::Modifiers::default(),start:Instant::now(),graph,selected,pending_source:None,dragging_wire:None,
            status:"Click an output port, then a compatible input port".into(),open:true,
            preview_rect:egui::Rect::from_min_size(egui::pos2(210.0,600.0),egui::vec2(950.0,210.0))})
    }
    pub fn graph(&self)->&Graph{&self.graph}
    pub fn preview_rect(&self)->egui::Rect{self.preview_rect}
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
        let preview_rect=&mut self.preview_rect;
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
                    // Reuse the node-header palette so the library and graph stay in sync.
                    let add_response = ui.add(
                        egui::Button::new(
                            egui::RichText::new(format!("+ {}", kind.title()))
                                .color(egui::Color32::WHITE),
                        )
                        .fill(node_header_color(kind)),
                    )
                        .on_hover_ui(|ui| {
                            ui.set_max_width(350.0);
                            module_library_tooltip(ui, kind);
                        });
                    if add_response.clicked(){
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
                .show(ctx,|ui|{
                    ui.strong("Live Shader Preview");
                    ui.small("Graph-generated GLSL 330 core · preview retains last valid program on errors");
                    *preview_rect = ui.available_rect_before_wrap();
                });
            egui::CentralPanel::default().frame(egui::Frame::default().fill(egui::Color32::from_rgba_unmultiplied(12,16,25,228))).show(ctx,|ui|{
                ui.heading("Node Graph");ui.small("Drag from output to input to wire · click-to-connect also works · drag node headers to move");ui.separator();
                let canvas=ui.available_rect_before_wrap();let origin=canvas.min;
                let mut input_ports=std::collections::HashMap::new();
                let mut input_hit_rects=std::collections::HashMap::new();
                let mut output_ports=std::collections::HashMap::new();
                let ids:Vec<NodeId>=graph.nodes.iter().map(|n|n.id).collect();
                for id in ids{
                    // Copy the immutable node fields before entering the UI closure.
                    // The closure also needs mutable access to the graph.
                    let Some(node) = graph.node(id) else { continue };
                    let kind = node.kind;
                    let pos = node.pos;
                    let expanded = node.expanded;
                    let area=egui::Area::new(egui::Id::new(("maker_node",id)))
                        .order(egui::Order::Foreground).movable(false)
                        .current_pos(origin+egui::vec2(pos[0],pos[1]))
                        .show(ctx,|ui|{
                            egui::Frame::window(ui.style()).show(ui,|ui|{
                                // Compute the smallest practical width from the actual visible
                                // labels and controls. No inherited fixed node width.
                                let node_width = compact_node_width(ui, kind, expanded, id);
                                ui.set_min_width(node_width);
                                ui.set_max_width(node_width);
                                ui.spacing_mut().slider_width = 92.0;
                                // The full-width, compact title bar is the only node drag handle.
                                // Connection ports remain outside this interactive rectangle.
                                let header_color = node_header_color(kind);
                                let title = format!("{} #{}", kind.title(), id);
                                let header_height = ui.text_style_height(&egui::TextStyle::Button) + 8.0;
                                let (header_rect, header_response) = ui.allocate_exact_size(
                                    egui::vec2(node_width, header_height),
                                    egui::Sense::click_and_drag(),
                                );
                                ui.painter().rect_filled(header_rect, 3.0, header_color);
                                ui.painter().text(
                                    header_rect.left_center() + egui::vec2(7.0, 0.0),
                                    egui::Align2::LEFT_CENTER,
                                    title,
                                    egui::TextStyle::Button.resolve(ui.style()),
                                    egui::Color32::WHITE,
                                );
                                let header_response = header_response.on_hover_text("Drag anywhere on this title bar to move the node");
                                if header_response.clicked() || header_response.dragged() {
                                    *selected = Some(id);
                                }
                                if header_response.dragged() {
                                    let delta = ui.ctx().input(|i| i.pointer.delta());
                                    if let Some(n) = graph.node_mut(id) {
                                        n.pos[0] += delta.x;
                                        n.pos[1] += delta.y;
                                    }
                                }
                                ui.horizontal(|ui| {
                                    if let Some(n) = graph.node_mut(id) {
                                        if ui.small_button(if n.expanded { "−" } else { "+" }).clicked() {
                                            n.expanded = !n.expanded;
                                        }
                                    }
                                });
                                if graph.node(id).is_some_and(|n|n.expanded){if let Some(n)=graph.node_mut(id){edit_parameters(ui,n);}}
                                // Ports occupy the vertical left/right edges of the node.
                                // They remain visible when the node is collapsed.
                                let inputs = kind.inputs();
                                let outputs = kind.outputs();
                                let rows = inputs.len().max(outputs.len());
                                let port_color = |ty: crate::maker_graph::PortType| -> egui::Color32 {
                                    match ty {
                                        crate::maker_graph::PortType::Float => egui::Color32::from_rgb(104,195,237),
                                        crate::maker_graph::PortType::Uv2 => egui::Color32::from_rgb(123,207,152),
                                        crate::maker_graph::PortType::Color3 => egui::Color32::from_rgb(229,173,96),
                                    }
                                };
                                // Parameter controls can make the actual node body wider
                                // than its title bar. Anchor port rows to the resulting
                                // content width, not the title/header width.
                                let body_width = ui.min_rect().width().max(node_width);
                                for row in 0..rows {
                                    let (row_rect, _) = ui.allocate_exact_size(
                                        egui::vec2(body_width, 25.0), egui::Sense::hover());
                                    if let Some(&(name, ty)) = inputs.get(row) {
                                        let center = egui::pos2(row_rect.left(), row_rect.center().y);
                                        let hit = egui::Rect::from_center_size(center, egui::vec2(22.0, 22.0));
                                        let response = ui.interact(hit, ui.id().with((id, "input", row)), egui::Sense::click_and_drag());
                                        input_ports.insert((id,row), center);
                                        input_hit_rects.insert((id,row), hit);
                                        let color = port_color(ty);
                                        ui.painter().circle_filled(center, 5.0, color);
                                        ui.painter().text(center + egui::vec2(11.0, 0.0), egui::Align2::LEFT_CENTER,
                                            name, egui::TextStyle::Small.resolve(ui.style()), ui.visuals().text_color());
                                        let response = response.on_hover_text(port_hint(graph,id,row,*pending,*dragging_wire));
                                        if response.clicked() && dragging_wire.is_none() {
                                            if let Some((from, from_port)) = pending.take() {
                                                match graph.connect(from,from_port,id,row) {
                                                    Ok(()) => *status = format!("Connected #{from}:{from_port} to #{id}:{row}"),
                                                    Err(e) => *status = format!("Connection rejected: {e}"),
                                                }
                                            } else {
                                                graph.disconnect_input(id,row);
                                                *status = format!("Disconnected input #{id}:{row}");
                                            }
                                            *selected = Some(id);
                                        }
                                    }
                                    if let Some(&(name, ty)) = outputs.get(row) {
                                        let center = egui::pos2(row_rect.right(), row_rect.center().y);
                                        let hit = egui::Rect::from_center_size(center, egui::vec2(22.0, 22.0));
                                        let response = ui.interact(hit, ui.id().with((id, "output", row)), egui::Sense::click_and_drag());
                                        output_ports.insert((id,row),center);
                                        ui.painter().circle_filled(center, 5.0, port_color(ty));
                                        ui.painter().text(center - egui::vec2(11.0,0.0), egui::Align2::RIGHT_CENTER,
                                            name, egui::TextStyle::Small.resolve(ui.style()),ui.visuals().text_color());
                                        let response = response.on_hover_text(format!(
                                            "{} output ({})\nMultiple destinations allowed. Click or drag to an input.",name,ty.name()));
                                        if response.drag_started() {
                                            *dragging_wire = Some((id,row));
                                            *pending = None;
                                            *selected = Some(id);
                                            *status = format!("Drag {} output to a compatible input",ty.name());
                                        }
                                        if response.clicked() {
                                            let active = *pending == Some((id,row));
                                            *pending = if active {None} else {Some((id,row))};
                                            *selected = Some(id);
                                            *status = if active {"Connection cancelled".into()} else {format!("Select a {} input",ty.name())};
                                        }
                                    }
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
                                input_hit_rects.get(&(id,port)).is_some_and(|hit|hit.contains(p))
                                    .then_some((id,port))
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
                let wire_paths: Vec<(Vec<egui::Pos2>,egui::Color32)> = graph.connections.iter().filter_map(|c| {
                    let a=*output_ports.get(&(c.from,c.from_port))?;
                    let b=*input_ports.get(&(c.to,c.to_port))?;
                    let ty=graph.node(c.from)?.kind.outputs().get(c.from_port)?.1;
                    let color=match ty {
                        crate::maker_graph::PortType::Float => egui::Color32::from_rgb(104,195,237),
                        crate::maker_graph::PortType::Uv2 => egui::Color32::from_rgb(123,207,152),
                        crate::maker_graph::PortType::Color3 => egui::Color32::from_rgb(229,173,96),
                    };
                    Some((orthogonal_wire(a,b),color))
                }).collect();
                for (index,(path,color)) in wire_paths.iter().enumerate() {
                    let bridged = bridge_crossings(index, path, &wire_paths);
                    painter.add(egui::Shape::line(bridged,egui::Stroke::new(2.5,*color)));
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
                            orthogonal_wire(start, end),
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

/// Right-angle routing with two horizontal runs and a vertical middle segment.
fn orthogonal_wire(a: egui::Pos2,b: egui::Pos2)->Vec<egui::Pos2>{
    let mid = if b.x >= a.x + 32.0 { (a.x+b.x)*0.5 } else { a.x+24.0 };
    vec![a,egui::pos2(mid,a.y),egui::pos2(mid,b.y),b]
}

/// Add a semicircular jump on the vertical segment where another wire's
/// horizontal segment crosses it. This is presentation-only, never connectivity.
fn bridge_crossings(index:usize,path:&[egui::Pos2],all:&[(Vec<egui::Pos2>,egui::Color32)])->Vec<egui::Pos2>{
    if path.len()!=4{return path.to_vec()}
    let x=path[1].x;
    let y0=path[1].y;
    let y1=path[2].y;
    let direction=if y1>=y0{1.0}else{-1.0};
    let mut crossings=Vec::new();
    for (other_index,(other,_)) in all.iter().enumerate(){
        if index==other_index || other.len()!=4 {continue}
        for pair in [(other[0],other[1]),(other[2],other[3])]{
            let (a,b)=pair;
            if x>a.x.min(b.x)+6.0 && x<a.x.max(b.x)-6.0
                && a.y>y0.min(y1)+7.0 && a.y<y0.max(y1)-7.0 {
                crossings.push(a.y);
            }
        }
    }
    crossings.sort_by(|a,b| if direction>0.0{a.total_cmp(b)}else{b.total_cmp(a)});
    crossings.dedup_by(|a,b|(*a-*b).abs()<12.0);
    let mut result=vec![path[0],path[1]];
    let mut last=y0;
    for y in crossings{
        if (y-last)*direction<12.0 || (y1-y)*direction<9.0{continue}
        let radius=6.0;
        result.push(egui::pos2(x,y-radius*direction));
        // A half-circle protruding to the right of the vertical wire.
        for step in 1..=12{
            let theta=std::f32::consts::PI*(step as f32/12.0);
            result.push(egui::pos2(x+radius*theta.sin(),y-radius*direction*theta.cos()));
        }
        last=y+radius*direction;
    }
    result.push(path[2]);result.push(path[3]);result
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

/// Stable module-type colors, independent of selection and connection validity.
fn node_header_color(kind: Kind) -> egui::Color32 {
    match kind {
        Kind::Uv => egui::Color32::from_rgb(40, 124, 155),
        Kind::Noise => egui::Color32::from_rgb(134, 96, 176),
        Kind::Multiply => egui::Color32::from_rgb(93, 111, 175),
        Kind::Palette => egui::Color32::from_rgb(180, 106, 53),
        Kind::Output => egui::Color32::from_rgb(56, 135, 101),
    }
}

/// Determine a node's width from its visible title, port labels and controls.
/// This deliberately avoids the parent canvas's available width and keeps
/// collapsed nodes smaller than expanded nodes when appropriate.
fn compact_node_width(ui: &egui::Ui, kind: Kind, expanded: bool, id: NodeId) -> f32 {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let measure = |text: &str| -> f32 {
        ui.painter()
            .layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::WHITE)
            .size().x
    };
    let title_width = measure(&format!("{} #{}", kind.title(), id)) + 16.0;
    let mut content_width = 0.0_f32;
    for (name, ty) in kind.inputs() {
        content_width = content_width.max(measure(&format!("{}: {}", name, ty.name())) + 66.0);
    }
    for (name, ty) in kind.outputs() {
        content_width = content_width.max(measure(&format!("{}: {}", name, ty.name())) + 66.0);
    }
    // Opposite-edge labels share a row; reserve room for both simultaneously.
    let input_label = kind.inputs().iter().map(|(name,_)|measure(name)).fold(0.0_f32,f32::max);
    let output_label = kind.outputs().iter().map(|(name,_)|measure(name)).fold(0.0_f32,f32::max);
    content_width = content_width.max(input_label + output_label + 38.0);
    if expanded {
        let controls_width: f32 = match kind {
            Kind::Uv | Kind::Output => 155.0,
            Kind::Noise | Kind::Multiply | Kind::Palette => 172.0,
        };
        content_width = content_width.max(controls_width);
    }
    title_width.max(content_width).max(66.0).ceil()
}

/// Module documentation is kept separate from the Module Library's layout.
/// Descriptions reflect the current graph compiler, including disconnected
/// input defaults and the animated behavior of generated GLSL.
fn module_library_tooltip(ui: &mut egui::Ui, kind: Kind) {
    ui.heading(kind.title());
    ui.separator();

    let (purpose, parameters, uses, notes) = match kind {
        Kind::Uv => (
            "Supplies centered, aspect-corrected fragment coordinates. The origin is at the image center; coordinates are scaled by the preview height.",
            "None.",
            "Coordinate source for procedural textures, spatial patterns, and noise sampling.",
            "Coordinates are not confined to 0–1. The output changes with fragment position and preview dimensions.",
        ),
        Kind::Noise => (
            "Samples smooth 2D value noise using the input coordinates. The generated pattern also drifts slowly over time.",
            "Scale (0.1–20): controls spatial frequency. Larger values produce finer patterns.",
            "Organic textures, masks, cloud-like variation, and inputs to color palettes.",
            "When UV is unconnected, the shader uses its built-in centered coordinates. Output values are approximately 0–1.",
        ),
        Kind::Multiply => (
            "Multiplies an incoming scalar by an adjustable constant.",
            "Factor (0–4): sets the scalar multiplier. Zero produces zero.",
            "Adjust noise strength, scale scalar signals, and change the range supplied to downstream modules.",
            "When Value is unconnected, its default is 0.5. This module currently has one input, not two arbitrary signal inputs.",
        ),
        Kind::Palette => (
            "Converts a scalar into an animated RGB color using phase-shifted cosine waves. Applies a configurable RGB tint.",
            "Frequency (0.1–5): controls color cycling over the input value. Color: multiplies the resulting RGB channels.",
            "Colorizing noise, creating procedural gradients, and producing animated shader colors.",
            "When Value is unconnected, its default is 0.5. The generated palette also animates with time.",
        ),
        Kind::Output => (
            "Terminal node defining the final RGB fragment color shown in the Live Shader Preview.",
            "None.",
            "Connect a Color3 output here to display the resulting shader.",
            "Only one Output Color node is permitted. If unconnected, the compiler generates a dark fallback color.",
        ),
    };

    ui.label(purpose);
    ui.add_space(5.0);
    ui.strong("Inputs");
    if kind.inputs().is_empty() {
        ui.label("None");
    } else {
        for (name, ty) in kind.inputs() {
            ui.label(format!("{name} ({})", ty.name()));
        }
    }
    ui.strong("Outputs");
    if kind.outputs().is_empty() {
        ui.label("None (terminal node)");
    } else {
        for (name, ty) in kind.outputs() {
            ui.label(format!("{name} ({})", ty.name()));
        }
    }
    ui.add_space(5.0);
    ui.strong("Parameters");
    ui.label(parameters);
    ui.strong("Typical uses");
    ui.label(uses);
    ui.strong("Behavioral notes");
    ui.label(notes);
}
