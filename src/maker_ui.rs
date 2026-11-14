//! Screenshaver Maker: typed node-graph workspace (Batch 03).
use std::{sync::Arc, time::Instant};
use sdl2::{event::Event, mouse::MouseButton};

use crate::maker_graph::{Graph, NodeId};
#[path = "maker_routing.rs"]
mod maker_routing;
use maker_routing::{Box2, PortPair, Routes};
use std::collections::BTreeMap;
use crate::maker_module::{Module, Trust};
use crate::maker_library;

pub struct MakerUi {
    context: egui::Context,
    painter: egui_glow::Painter,
    events: Vec<egui::Event>,
    pointer: egui::Pos2,
    modifiers: egui::Modifiers,
    start: Instant,
    graph: Graph,
    selected: Option<NodeId>,
    parameter_popup: Option<NodeId>,
    pending_source: Option<(NodeId, usize)>,
    dragging_wire: Option<(NodeId, usize)>,

    status: String,
    open: bool,
    preview_rect: egui::Rect,
    preview_height: Option<f32>,
    routes: Routes,
    canvas_pan: egui::Vec2,
    canvas_rect: egui::Rect,
    panning: bool,
    space_held: bool,
    pixels_per_point: f32,
    routing_undo: Vec<(Graph,Routes)>,
    routing_redo: Vec<(Graph,Routes)>,
}
impl MakerUi {
    pub fn new(video:&sdl2::VideoSubsystem)->Result<Self,String>{
        let gl=unsafe{glow::Context::from_loader_function(|s|video.gl_get_proc_address(s) as *const _)};
        let painter=egui_glow::Painter::new(Arc::new(gl),"",None,false).map_err(|e|e.to_string())?;
        let graph=Graph::demo();let selected=graph.nodes.iter().find(|n|n.module_id=="screenshaver.color.cosine_palette").map(|n|n.id);
        Ok(Self{context:egui::Context::default(),painter,events:Vec::new(),pointer:egui::Pos2::ZERO,
            modifiers:egui::Modifiers::default(),start:Instant::now(),graph,selected,parameter_popup:None,pending_source:None,dragging_wire:None,
            status:"Click an output port, then a compatible input port".into(),open:true,
            preview_rect:egui::Rect::from_min_size(egui::pos2(210.0,600.0),egui::vec2(950.0,210.0)),
            preview_height:None,routes:Routes::new(),canvas_pan:egui::Vec2::ZERO,canvas_rect:egui::Rect::NOTHING,panning:false,space_held:false,pixels_per_point:1.0,routing_undo:Vec::new(),routing_redo:Vec::new()})
    }
    pub fn graph(&self)->&Graph{&self.graph}
    pub fn preview_rect(&self)->egui::Rect{self.preview_rect}
    pub fn handle_event(&mut self, event: &Event) {
        match event {
            Event::MouseMotion { x, y, .. } => {
                let next=egui::pos2(*x as f32/self.pixels_per_point,*y as f32/self.pixels_per_point);
                if self.panning { self.canvas_pan += next-self.pointer; }
                self.pointer = next;
                self.events.push(egui::Event::PointerMoved(self.pointer));
            }
            Event::MouseButtonDown { mouse_btn, x, y, .. } | Event::MouseButtonUp { mouse_btn, x, y, .. } => {
                let pressed = matches!(event, Event::MouseButtonDown { .. });
                let button = match mouse_btn { MouseButton::Left => egui::PointerButton::Primary,
                    MouseButton::Right => egui::PointerButton::Secondary,
                    MouseButton::Middle => egui::PointerButton::Middle, _ => return };
                self.pointer = egui::pos2(*x as f32/self.pixels_per_point, *y as f32/self.pixels_per_point);
                if *mouse_btn==MouseButton::Middle || (*mouse_btn==MouseButton::Left && self.space_held) {
                    if pressed && self.canvas_rect.contains(self.pointer) {self.panning=true;}
                    if !pressed {self.panning=false;}
                    // Keep egui pointer state in sync; otherwise its internal
                    // drag ownership can become stuck after a pan.
                    self.events.push(egui::Event::PointerButton { pos: self.pointer, button, pressed, modifiers: self.modifiers });
                    return;
                }
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
                if *key==Keycode::Space { self.space_held=matches!(event,Event::KeyDown{..}); }
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
        self.pixels_per_point=scale;
        let mut input=egui::RawInput::default();
        input.screen_rect=Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(dw as f32/scale,dh as f32/scale)));
        input.viewports.insert(egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(scale),..Default::default()});
        input.time=Some(self.start.elapsed().as_secs_f64());input.modifiers=self.modifiers;
        input.events=std::mem::take(&mut self.events);
        let old_parameters:Vec<_>=self.graph.nodes.iter().map(|n|(n.id,n.parameters.clone())).collect();
        let graph=&mut self.graph;
        let selected=&mut self.selected;
        let parameter_popup=&mut self.parameter_popup;
        let pending=&mut self.pending_source;
        let dragging_wire=&mut self.dragging_wire;
        let status=&mut self.status;
        let open=&mut self.open;
        let preview_rect=&mut self.preview_rect;
        let preview_height=&mut self.preview_height;
        let routes=&mut self.routes;
        let canvas_pan=&mut self.canvas_pan;
        let canvas_rect=&mut self.canvas_rect;
        let routing_undo=&mut self.routing_undo;
        let routing_redo=&mut self.routing_redo;
        let mut optimize_requested=false;
        let mut fit_requested=false;
        let mut editor_anchor: Option<(NodeId, egui::Pos2)> = None;
        let output=self.context.run(input,|ctx|{
            egui::TopBottomPanel::top("maker_header").show(ctx,|ui|{
                ui.horizontal(|ui|{
                    ui.heading("Screenshaver Maker");ui.separator();ui.label("Untitled graph");
                    if ui.button("Reset demo").clicked(){let revision=graph.revision;*graph=Graph::demo();graph.revision=revision+1;
                        *selected=graph.nodes.iter().find(|n|n.module_id=="screenshaver.color.cosine_palette").map(|n|n.id);*parameter_popup=None;*pending=None;*dragging_wire=None;routes.clear();routing_undo.clear();routing_redo.clear();*status="Demo graph reset".into();
                    }
                    if ui.button("Optimize Routing").clicked(){optimize_requested=true;}
                    if ui.button("Center Graph").clicked(){fit_requested=true;}
                    if ui.add_enabled(!routing_undo.is_empty(),egui::Button::new("Undo Routing")).clicked(){
                        if let Some((previous,old_routes))=routing_undo.pop(){
                            routing_redo.push((graph.clone(),routes.clone()));
                            *graph=previous;*routes=old_routes;graph.revision+=1;
                            *status="Restored previous routing layout".into();
                        }
                    }
                    if ui.add_enabled(!routing_redo.is_empty(),egui::Button::new("Redo Routing")).clicked(){
                        if let Some((next,next_routes))=routing_redo.pop(){
                            routing_undo.push((graph.clone(),routes.clone()));
                            *graph=next;*routes=next_routes;graph.revision+=1;
                            *status="Reapplied routing layout".into();
                        }
                    }
                    if ui.button("Exit").clicked(){*open=false;}
                });
            });
            egui::TopBottomPanel::bottom("maker_footer").show(ctx,|ui|{
                ui.horizontal_wrapped(|ui|{
                    ui.label("Batch 07 · field traversal + anchored parameter editor");ui.separator();
                    ui.label(format!("{} nodes / {} connections",graph.nodes.len(),graph.connections.len()));ui.separator();
                    if let Some(e)=compile_error{ui.colored_label(egui::Color32::LIGHT_RED,format!("GLSL: {}",e));}
                    else{ui.label(status.as_str());}
                });
            });
            egui::SidePanel::left("maker_library").resizable(true).default_width(190.0).min_width(155.0).show(ctx,|ui|{
                ui.heading("Module Library");ui.separator();
                ui.small("Add nodes to the graph:");
                for module in maker_library::get().modules.values(){
                    if module.terminal && graph.nodes.iter().any(|n|n.definition().terminal){continue}
                    // Reuse the node-header palette so the library and graph stay in sync.
                    let add_response = ui.add(
                        egui::Button::new(
                            egui::RichText::new(format!("+ {}", module.name.as_str()))
                                .color(egui::Color32::WHITE),
                        )
                        .fill(node_header_color(module)),
                    )
                        .on_hover_ui(|ui| {
                            ui.set_max_width(350.0);
                            module_library_tooltip(ui, module);
                        });
                    if add_response.clicked(){
                        let count=graph.nodes.len();
                        match graph.add(&module.id,[80.0+(count%3) as f32*45.0,180.0+(count%4) as f32*42.0]){
                            Ok(id)=>{*selected=Some(id);*status=format!("Added {} #{}",module.name,id);},
                            Err(e)=>*status=e,
                        }
                    }
                }
                ui.separator();ui.small("Drag from an output port to a compatible input, or click output then input. Outputs support fan-out; each input has one source.");
                if let Some((id,port))=*pending{ui.label(format!("Source: node #{id}, port {port}"));if ui.button("Cancel connection").clicked(){*pending=None;*dragging_wire=None;}}
            });
            egui::SidePanel::right("maker_inspector").resizable(true).default_width(260.0).min_width(200.0).show(ctx,|ui|{
                ui.heading("Inspector");ui.separator();
                if let Some(id)=*selected{
                    if let Some(n)=graph.node_mut(id){
                        ui.strong(format!("{} #{}",n.definition().name,id));ui.separator();
                        ui.label(n.definition().description.as_str());
                        if !n.definition().parameters.is_empty() {
                            let response=ui.button("Open parameter editor");
                            if response.clicked(){*parameter_popup=if *parameter_popup==Some(id){None}else{Some(id)};}
                            
                        }
                    }
                    if ui.button("Delete selected node").clicked(){match graph.remove(id){Ok(())=>{*selected=None;*pending=None;*status=format!("Deleted node #{id}")},Err(e)=>*status=e}}
                }else{ui.label("Select a node to edit its parameters.");}
                ui.separator();ui.label("Graph status");
                match graph.validate(){Ok(())=>{ui.small("Typed connections valid; no cycles");},Err(e)=>{ui.colored_label(egui::Color32::LIGHT_RED,e);}}
            });
            // Explicitly own the preview height. A default_height() hint alone is
            // not sufficient: egui may restore a previously cached panel size.
            // The saved value changes only when the divider is dragged.
            let available_height = ctx.available_rect().height();
            let minimum_graph_height = 150.0_f32;
            let minimum_preview_height = 120.0_f32;
            let maximum_preview_height = (available_height - minimum_graph_height)
                .max(minimum_preview_height);
            let desired_height = preview_height.get_or_insert_with(|| available_height * 0.5);
            let shown_height = desired_height.clamp(
                minimum_preview_height.min(maximum_preview_height),
                maximum_preview_height,
            );
            egui::TopBottomPanel::bottom("maker_live_preview")
                .resizable(false)
                .exact_height(shown_height)
                .frame(egui::Frame::default().fill(egui::Color32::from_rgba_unmultiplied(10,14,20,42)))
                .show(ctx,|ui|{
                    // A dedicated draggable divider, instead of relying on
                    // TopBottomPanel's internally remembered resize state.
                    let (divider_rect, divider_response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 9.0), egui::Sense::drag());
                    ui.painter().line_segment(
                        [egui::pos2(divider_rect.left(),divider_rect.center().y),
                         egui::pos2(divider_rect.right(),divider_rect.center().y)],
                        egui::Stroke::new(2.0,egui::Color32::from_gray(105)));
                    if divider_response.hovered() || divider_response.dragged() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                    }
                    if divider_response.dragged() {
                        let dy=ui.input(|i|i.pointer.delta().y);
                        *desired_height = (*desired_height - dy).clamp(
                            minimum_preview_height.min(maximum_preview_height),
                            maximum_preview_height,
                        );
                    }
                    ui.strong("Live Shader Preview");
                    ui.small("Graph-generated GLSL 330 core · preview retains last valid program on errors");
                    *preview_rect = ui.available_rect_before_wrap();
                });
            egui::CentralPanel::default().frame(egui::Frame::default().fill(egui::Color32::from_rgba_unmultiplied(12,16,25,228))).show(ctx,|ui|{
                ui.heading("Node Graph");ui.small("Drag from output to input to wire · click-to-connect also works · drag node headers to move");ui.separator();
                let canvas=ui.available_rect_before_wrap();
                *canvas_rect=canvas;
                if fit_requested && !graph.nodes.is_empty() {
                    let left=graph.nodes.iter().map(|n|n.pos[0]).fold(f32::INFINITY,f32::min);
                    let top=graph.nodes.iter().map(|n|n.pos[1]).fold(f32::INFINITY,f32::min);
                    let right=graph.nodes.iter().map(|n|n.pos[0]+240.0).fold(f32::NEG_INFINITY,f32::max);
                    let bottom=graph.nodes.iter().map(|n|n.pos[1]+160.0).fold(f32::NEG_INFINITY,f32::max);
                    *canvas_pan=egui::vec2((canvas.width()-(right-left))*0.5-left,(canvas.height()-(bottom-top))*0.5-top);
                }
                let origin=canvas.min+*canvas_pan;

                let mut input_ports=std::collections::HashMap::new();
                let mut input_hit_rects=std::collections::HashMap::new();
                let mut output_ports=std::collections::HashMap::new();
                let mut module_boxes:BTreeMap<NodeId,Box2>=BTreeMap::new();
                let ids:Vec<NodeId>=graph.nodes.iter().map(|n|n.id).collect();
                for id in ids{
                    // Copy the immutable node fields before entering the UI closure.
                    // The closure also needs mutable access to the graph.
                    let Some(node) = graph.node(id) else { continue };
                    let module = node.definition();
                    let pos = node.pos;
                    let expanded = node.expanded;
                    let area=egui::Area::new(egui::Id::new(("maker_node",id)))
                        .order(egui::Order::Middle).movable(false)
                        .current_pos(origin+egui::vec2(pos[0],pos[1]))
                        .show(ctx,|ui|{
                            ui.set_clip_rect(ui.clip_rect().intersect(canvas));
                            egui::Frame::window(ui.style()).show(ui,|ui|{
                                // Fixed standard width and height; only port count may grow height.
                                let node_width = 220.0;
                                let rows = module.inputs.len().max(module.outputs.len());
                                let port_height = rows as f32 * 25.0;
                                let header_height = 27.0;
                                let controls_height = (140.0_f32 - header_height - port_height - 16.0).max(0.0);
                                ui.set_min_width(node_width);
                                ui.set_max_width(node_width);
                                ui.spacing_mut().slider_width = 100.0;
                                let header_color = node_header_color(module);
                                let title = format!("{} #{}", module.name.as_str(), id);
                                let (header_rect, header_response) = ui.allocate_exact_size(
                                    egui::vec2(node_width, header_height), egui::Sense::click_and_drag(),
                                );
                                ui.painter().rect_filled(header_rect, 3.0, header_color);
                                ui.painter().text(header_rect.left_center()+egui::vec2(7.0,0.0),
                                    egui::Align2::LEFT_CENTER,title,egui::TextStyle::Button.resolve(ui.style()),egui::Color32::WHITE);
                                let header_response=header_response.on_hover_text("Drag title bar to move the module");
                                if header_response.clicked() {
                                    *selected=Some(id);
                                    if *parameter_popup==Some(id) { *parameter_popup=None; }
                                }
                                if header_response.dragged(){*selected=Some(id);}
                                if header_response.dragged(){
                                    let delta=ui.ctx().input(|i|i.pointer.delta());
                                    if let Some(n)=graph.node_mut(id){n.pos[0]+=delta.x;n.pos[1]+=delta.y;}
                                }
                                // Inline controls are strictly bounded by remaining standard height.
                                // Every adjustable module also has the full floating editor.
                                if !module.parameters.is_empty(){
                                    ui.horizontal(|ui|{
                                        let response=ui.small_button("⚙ Parameters ↗").on_hover_text("Open full parameter editor");
                                        if response.clicked(){
                                            *parameter_popup=if *parameter_popup==Some(id){None}else{Some(id)};
                                            *selected=Some(id);
                                        }
                                        
                                    });
                                }
                                if module.parameters.len()<=2 && controls_height>=35.0*module.parameters.len() as f32 + 22.0 {
                                    if let Some(n)=graph.node_mut(id){edit_parameters(ui,n);}
                                }
                                // Fill unused body space without expanding to accommodate controls.
                                let used=ui.min_rect().height();
                                let target=(140.0-port_height-16.0).max(0.0);
                                if used<target{ui.add_space(target-used);}
                                // Ports occupy the vertical left/right edges of the node.
                                // They remain visible when the node is collapsed.
                                let inputs = &module.inputs;
                                let outputs = &module.outputs;
                                let rows = inputs.len().max(outputs.len());
                                let port_color = port_color;
                                // Parameter controls can make the actual node body wider
                                // than its title bar. Anchor port rows to the resulting
                                // content width, not the title/header width.
                                let body_width = node_width;
                                for row in 0..rows {
                                    let (row_rect, _) = ui.allocate_exact_size(
                                        egui::vec2(body_width, 25.0), egui::Sense::hover());
                                    if let Some(port) = inputs.get(row) {
                                        let name=&port.name;let ty=port.ty.as_str();
                                        let center = egui::pos2(row_rect.left(), row_rect.center().y);
                                        let hit = egui::Rect::from_center_size(center, egui::vec2(22.0, 22.0));
                                        let response = ui.interact(hit, ui.id().with((id, "input", row)), egui::Sense::click_and_drag());
                                        input_ports.insert((id,row), center);
                                        input_hit_rects.insert((id,row), hit);
                                        let color = port_color(ty);
                                        // Connector disk is drawn once in the foreground at the measured outer sidewall.
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
                                    if let Some(port) = outputs.get(row) {
                                        let name=&port.name;let ty=port.ty.as_str();
                                        let center = egui::pos2(row_rect.right(), row_rect.center().y);
                                        let hit = egui::Rect::from_center_size(center, egui::vec2(22.0, 22.0));
                                        let response = ui.interact(hit, ui.id().with((id, "output", row)), egui::Sense::click_and_drag());
                                        output_ports.insert((id,row),center);
                                        // Connector disk is drawn once in the foreground at the measured outer sidewall.
                                        ui.painter().text(center - egui::vec2(11.0,0.0), egui::Align2::RIGHT_CENTER,
                                            name, egui::TextStyle::Small.resolve(ui.style()),ui.visuals().text_color());
                                        let response = response.on_hover_text(format!(
                                            "{} output ({})\nMultiple destinations allowed. Click or drag to an input.",name,ty));
                                        if response.drag_started() {
                                            *dragging_wire = Some((id,row));
                                            *pending = None;
                                            *selected = Some(id);
                                            *status = format!("Drag {} output to a compatible input",ty);
                                        }
                                        if response.clicked() {
                                            let active = *pending == Some((id,row));
                                            *pending = if active {None} else {Some((id,row))};
                                            *selected = Some(id);
                                            *status = if active {"Connection cancelled".into()} else {format!("Select a {} input",ty)};
                                        }
                                    }
                                }
                            });
                        });
                    module_boxes.insert(id,Box2{min:[area.response.rect.min.x-origin.x,area.response.rect.min.y-origin.y],
                        max:[area.response.rect.max.x-origin.x,area.response.rect.max.y-origin.y]});
                    // Use the measured outer module rectangle as the single
                    // authoritative left/right sidewall for connector centers.
                    // Port-row rects are inset by egui Frame margins; using their
                    // x coordinates for routing caused all wires to fail validation.
                    let wall = area.response.rect;
                    for (&(port_node, _), center) in input_ports.iter_mut() {
                        if port_node == id { center.x = wall.left(); }
                    }
                    for (&(port_node, _), center) in output_ports.iter_mut() {
                        if port_node == id { center.x = wall.right(); }
                    }
                    // Keep drop targeting centered on the same visible connector.
                    for (&(port_node, port_row), hit) in input_hit_rects.iter_mut() {
                        if port_node == id {
                            if let Some(center) = input_ports.get(&(port_node, port_row)) {
                                *hit = egui::Rect::from_center_size(*center, egui::vec2(22.0, 22.0));
                            }
                        }
                    }
                    // Anchor the editor to the *bottom of the whole module*, not
                    // to the Parameters button or title bar.
                    if *parameter_popup == Some(id) {
                        editor_anchor = Some((id, area.response.rect.left_bottom()));
                    }
                    // A click on the module background also dismisses its editor;
                    // child controls and wiring ports retain their own interactions.
                    if area.response.clicked() && *parameter_popup == Some(id) {
                        *parameter_popup = None;
                    }
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
                // Measure actual port locations after drawing modules; never guess
                // their offsets from the JSON module definitions.
                let measured_ports: BTreeMap<maker_routing::ConnectionKey,PortPair> = {
                    graph.connections.iter().filter_map(|c|{
                        let a=output_ports.get(&(c.from,c.from_port))?;
                        let b=input_ports.get(&(c.to,c.to_port))?;
                        Some((maker_routing::key(c),PortPair{from:[a.x-origin.x,a.y-origin.y],to:[b.x-origin.x,b.y-origin.y]}))
                    }).collect()
                };
                let boxes_now:Vec<_>=module_boxes.iter().map(|(&id,&rect)|(id,rect)).collect();
                if optimize_requested {
                    let original=graph.clone();
                    let sizes:BTreeMap<_,_>=module_boxes.iter().map(|(&id,b)|(id,[b.max[0]-b.min[0],b.max[1]-b.min[1]])).collect();
                    // Evaluate several deterministic row widths using the real router.
                    // The footprint includes every routed wire, not just module boxes.
                    let baseline=maker_routing::compact_positions(graph,&sizes);
                    let mut ordered:Vec<_>=baseline.iter().map(|(&id,&p)|(id,p)).collect();
                    ordered.sort_by(|a,b|a.1[0].total_cmp(&b.1[0]).then_with(||a.1[1].total_cmp(&b.1[1])).then_with(||a.0.cmp(&b.0)));
                    let ids:Vec<NodeId>=ordered.into_iter().map(|(id,_)|id).collect();
                    let margin=24.0_f32;
                    let mut best:Option<(f64,Graph,Routes)>=None;
                    let mut rejected=0usize;
                    for columns in 1..=ids.len().min(12) {
                        for &gap_x in &[60.0_f32,90.0,130.0] {
                            for &gap_y in &[70.0_f32,110.0,160.0] {
                                let mut candidate=BTreeMap::new();
                                let mut y=margin;
                                for row_ids in ids.chunks(columns) {
                                    let mut x=margin;
                                    let mut row_height=0.0_f32;
                                    for &id in row_ids {
                                        let size=sizes[&id];
                                        candidate.insert(id,[x,y]);
                                        x+=size[0]+gap_x;
                                        row_height=row_height.max(size[1]);
                                    }
                                    y+=row_height+gap_y;
                                }
                                let candidate_boxes:Vec<_>=boxes_now.iter().map(|&(id,b)|{
                                    let old=graph.node(id).map(|n|n.pos).unwrap_or([0.0,0.0]);
                                    let new=candidate.get(&id).copied().unwrap_or(old);
                                    let dx=new[0]-old[0];let dy=new[1]-old[1];
                                    (id,Box2{min:[b.min[0]+dx,b.min[1]+dy],max:[b.max[0]+dx,b.max[1]+dy]})
                                }).collect();
                                let shifted:BTreeMap<_,_>=measured_ports.iter().map(|(&k,p)|{
                                    let shift=|id:NodeId,pt:[f32;2]|{
                                        let old=graph.node(id).map(|n|n.pos).unwrap_or([0.0,0.0]);
                                        let new=candidate.get(&id).copied().unwrap_or(old);
                                        [pt[0]+new[0]-old[0],pt[1]+new[1]-old[1]]
                                    };
                                    (k,PortPair{from:shift(k.0,p.from),to:shift(k.2,p.to)})
                                }).collect();
                                let mut proposed=graph.clone();
                                for n in &mut proposed.nodes {if let Some(&pos)=candidate.get(&n.id){n.pos=pos;}}
                                let Ok(mut new_routes)=maker_routing::route_all(&proposed,&shifted,&candidate_boxes) else {rejected+=1;continue};
                                let mut min=[f32::INFINITY;2];let mut max=[f32::NEG_INFINITY;2];
                                for &(_,b) in &candidate_boxes {
                                    for axis in 0..2 {min[axis]=min[axis].min(b.min[axis]);max[axis]=max[axis].max(b.max[axis]);}
                                }
                                let mut wire_length=0.0_f64;
                                let mut bends=0usize;
                                for path in new_routes.values(){
                                    for &pt in path {for axis in 0..2 {min[axis]=min[axis].min(pt[axis]);max[axis]=max[axis].max(pt[axis]);}}
                                    for seg in path.windows(2) {wire_length+=((seg[1][0]-seg[0][0]).abs()+(seg[1][1]-seg[0][1]).abs()) as f64;}
                                    bends+=path.len().saturating_sub(2);
                                }
                                let w=(max[0]-min[0]).max(1.0);let h=(max[1]-min[1]).max(1.0);
                                let fits=w+2.0*margin<=canvas.width() && h+2.0*margin<=canvas.height();
                                let ratio=(w.max(h)/w.min(h)) as f64;
                                let area=(w as f64)*(h as f64);
                                // Fit first, then favor a square compact footprint;
                                // routing length and bends break near ties.
                                let score=(if fits {0.0} else {1.0e12})
                                    +area*(1.0+0.6*(ratio-1.0))
                                    +wire_length*8.0+(bends as f64)*300.0;
                                // Translate both modules and routes so the entire
                                // geometry (including returning wires) has a margin.
                                let dx=margin-min[0];let dy=margin-min[1];
                                for n in &mut proposed.nodes {n.pos[0]+=dx;n.pos[1]+=dy;}
                                for path in new_routes.values_mut(){for pt in path {pt[0]+=dx;pt[1]+=dy;}}
                                if best.as_ref().is_none_or(|(value,_,_)|score<*value) {
                                    best=Some((score,proposed,new_routes));
                                }
                            }
                        }
                    }
                    if let Some((_,proposed,new_routes))=best {
                        routing_undo.push((original,routes.clone()));
                        if routing_undo.len()>32{routing_undo.remove(0);}
                        routing_redo.clear();
                        *graph=proposed;*routes=new_routes;graph.revision+=1;
                        *status="Optimized modules and routed wires for a compact, near-square footprint (Undo available)".into();
                    } else {
                        *status=format!("Routing unchanged: no valid candidate ({rejected} tested)");
                    }
                } else {
                    // Repair only geometry when modules are moved or wired. Never
                    // rearrange modules except via the explicit toolbar action.
                    // A blocked connection must never erase routes for other wires.
                    // Recompute from the measured port geometry on every frame,
                    // including the first one, without requiring module movement.
                    let mut updated=Routes::new();
                    let mut failed=0usize;
                    let mut first_error=None;
                    for c in &graph.connections {
                        let k=maker_routing::key(c);
                        if let Some(pair)=measured_ports.get(&k) {
                            match maker_routing::route(pair.from,pair.to,&boxes_now,c.from,c.to) {
                                Ok(path)=>{updated.insert(k,path);},
                                Err(e)=>{failed+=1;if first_error.is_none(){first_error=Some(e);}}
                            }
                        } else {
                            failed+=1;
                            if first_error.is_none(){first_error=Some("Port geometry not available".into());}
                        }
                    }
                    *routes=updated;
                    if failed>0 {
                        *status=format!("Connection routing unavailable for {failed} wire(s): {}",first_error.unwrap_or_default());
                    }
                }
                let painter=ctx.layer_painter(egui::LayerId::new(egui::Order::Background,egui::Id::new("maker_wire_layer"))).with_clip_rect(canvas);
                let wire_paths: Vec<(Vec<egui::Pos2>,egui::Color32)> = graph.connections.iter().filter_map(|c| {
                    let a=*output_ports.get(&(c.from,c.from_port))?;
                    let b=*input_ports.get(&(c.to,c.to_port))?;
                    let ty=graph.node(c.from)?.definition().outputs.get(c.from_port)?.ty.as_str();
                    let color=port_color(ty);
                    // Never draw a fallback path that could cross a module or
                    // appear to connect anywhere other than its actual port.
                    let path:Vec<egui::Pos2>=routes.get(&maker_routing::key(c))?
                        .iter().map(|v|egui::pos2(v[0]+origin.x,v[1]+origin.y)).collect();
                    Some((path,color))
                }).collect();
                for (index,(path,color)) in wire_paths.iter().enumerate() {
                    let bridged = bridge_crossings(index, path, &wire_paths);
                    painter.add(egui::Shape::line(bridged,egui::Stroke::new(2.5,*color)));
                }
                // Compatibility rings are drawn in a separate foreground pass below.
                // Drawing them on the Middle wire layer hides them behind node bodies.
                // Live wire follows the pointer while the source port is dragged.
                if let Some((id, port)) = *dragging_wire {
                    if let (Some(&start), Some(end)) = (output_ports.get(&(id, port)), pointer_pos) {
                        painter.add(egui::Shape::line(
                            orthogonal_wire(start, end),
                            egui::Stroke::new(2.5, egui::Color32::from_rgb(255, 196, 92)),
                        ));
                    }
                }
                // Draw connector indicators after all node windows and connection lines.
                // This foreground Area is paint-only (no allocated interactive widgets),
                // so the original port hit rectangles remain responsible for wiring.
                // The parameter editor is created after this pass and remains above it.
                egui::Area::new(egui::Id::new("maker_connector_overlay"))
                    .order(egui::Order::Foreground)
                    .fixed_pos(canvas.min)
                    .interactable(false)
                    .show(ctx, |overlay_ui| {
                        overlay_ui.set_clip_rect(overlay_ui.clip_rect().intersect(canvas));
                        let overlay = overlay_ui.painter();
                        // Repaint the actual connector disks in the foreground so
                        // their protruding halves are never covered by node frames.
                        for (&(id, row), &center) in &input_ports {
                            if let Some(node) = graph.node(id) {
                                if let Some(port) = node.definition().inputs.get(row) {
                                    overlay.circle_filled(center, 5.0, port_color(&port.ty));
                                }
                            }
                        }
                        for (&(id, row), &center) in &output_ports {
                            if let Some(node) = graph.node(id) {
                                if let Some(port) = node.definition().outputs.get(row) {
                                    overlay.circle_filled(center, 5.0, port_color(&port.ty));
                                }
                            }
                        }
                        // Green/red availability rings share the same foreground pass.
                        if let Some((from, from_port)) = (*dragging_wire).or(*pending) {
                            for (&(to, to_port), &center) in &input_ports {
                                let compatible = graph.check_connection(from, from_port, to, to_port).is_ok();
                                let color = if compatible { egui::Color32::GREEN } else { egui::Color32::RED };
                                overlay.circle_stroke(center, 10.0, egui::Stroke::new(1.5, color));
                            }
                        }
                    });
                ui.allocate_rect(canvas,egui::Sense::hover());
            });
            // Module-attached editor is a foreground Area, not a Popup. Its
            // lifetime is independent of nested color-picker popups.
            // The contents are only interactive while the editor is open.
            let editor_id = egui::Id::new("maker_attached_parameter_editor");
            let is_open = parameter_popup.is_some();
            let progress = ctx.animate_bool_with_time(editor_id, is_open, 0.18);
            if let Some(id) = *parameter_popup {
                if graph.node(id).is_none() { *parameter_popup = None; }
            }
            if let Some(id) = *parameter_popup {
                if let Some((anchor_id, anchor_pos)) = editor_anchor {
                    if anchor_id == id {
                        let bounds = *canvas_rect;
                        let width = 360.0_f32.min((bounds.width() - 16.0).max(200.0));
                        let max_height = (bounds.height() - 16.0).max(120.0);
                        let target_height = 410.0_f32.min(max_height);
                        // The panel unfolds directly below the module. Keep its
                        // horizontal origin aligned with the module's left edge.
                        let x = anchor_pos.x.clamp(bounds.left() + 8.0,
                            (bounds.right() - width - 8.0).max(bounds.left() + 8.0));
                        // Preserve the bottom-edge attachment. If the module is
                        // near the bottom, reduce the panel's visible height and
                        // scroll its controls rather than overlay the module.
                        let y = anchor_pos.y;
                        let room_below = (bounds.bottom() - y - 8.0).max(0.0);
                        let visible_height = target_height.min(room_below);
                        egui::Area::new(egui::Id::new(("maker_parameter_area", id)))
                            .order(egui::Order::Foreground)
                            .movable(false)
                            .fixed_pos(egui::pos2(x, y))
                            .show(ctx, |ui| {
                                ui.set_clip_rect(ui.clip_rect().intersect(bounds));
                                ui.set_width(width);
                                ui.set_max_height(visible_height);
                                // Clip only the opening animation. Once expanded,
                                // nested widgets use their normal independent layers.
                                if progress < 1.0 || visible_height < target_height {
                                    let clip = egui::Rect::from_min_size(
                                        ui.min_rect().min, egui::vec2(width, visible_height * progress));
                                    ui.set_clip_rect(ui.clip_rect().intersect(clip));
                                }
                                egui::Frame::window(ui.style()).show(ui, |ui| {
                                    ui.set_width(width - 24.0);
                                    ui.horizontal(|ui| {
                                        ui.strong(format!("{} #{} — Parameters",
                                            graph.node(id).unwrap().definition().name, id));
                                        if ui.button("✕").on_hover_text("Close parameter editor").clicked() {
                                            *parameter_popup = None;
                                        }
                                    });
                                    ui.separator();
                                    egui::ScrollArea::vertical()
                                        .max_height((visible_height - 64.0).max(24.0))
                                        .show(ui, |ui| {
                                            if *parameter_popup == Some(id) {
                                                if let Some(node) = graph.node_mut(id) { edit_parameters(ui, node); }
                                            }
                                        });
                                });
                            });
                    }
                }
            }
            if parameter_popup.is_some() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                *parameter_popup = None;
            }

        });
        if graph.nodes.iter().any(|n|old_parameters.iter().find(|(id,_)|*id==n.id).is_some_and(|(_,p)|*p!=n.parameters)){graph.revision+=1;}
        let shapes=self.context.tessellate(output.shapes,output.pixels_per_point);
        self.painter.paint_and_update_textures([dw,dh],output.pixels_per_point,&shapes,&output.textures_delta);
        self.open
    }
    pub fn destroy(&mut self){self.painter.destroy();}
}
fn edit_parameters(ui:&mut egui::Ui,node:&mut crate::maker_graph::Node){
    let module=node.definition();
    if module.parameters.is_empty(){ui.small(module.description.as_str());}
    for p in &module.parameters {
        let Some(value)=node.parameters.get_mut(&p.id) else {continue};
        match p.ty.as_str(){
            "float"=>{
                let mut v=value.as_f64().unwrap_or(0.0) as f32;
                if ui.add(egui::Slider::new(&mut v,p.min..=p.max).text(&p.name)).changed(){
                    *value=serde_json::json!(v);
                }
            }
            "color3"=>{
                if let Some(a)=value.as_array(){
                    if a.len()==3{
                        let mut rgb=[a[0].as_f64().unwrap_or(0.0) as f32,a[1].as_f64().unwrap_or(0.0) as f32,a[2].as_f64().unwrap_or(0.0) as f32];
                        ui.label(&p.name);
                        if ui.color_edit_button_rgb(&mut rgb).changed(){*value=serde_json::json!(rgb);}
                    }
                }
            }
            _=>{}
        }
    }
}

fn port_color(ty:&str)->egui::Color32{
    match ty {
        "float"=>egui::Color32::from_rgb(104,195,237),
        "vec2"=>egui::Color32::from_rgb(123,207,152),
        "vec3"=>egui::Color32::from_rgb(229,173,96),
        _=>egui::Color32::GRAY,
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
    if path.len()<2{return path.to_vec()}
    let mut result=vec![path[0]];
    for segment in path.windows(2){
        let a=segment[0];let b=segment[1];
        if (a.x-b.x).abs()>0.1 || (a.y-b.y).abs()<18.0 {result.push(b);continue}
        let direction=if b.y>=a.y{1.0}else{-1.0};
        let mut crossings=Vec::new();
        for (other_index,(other,_)) in all.iter().enumerate(){
            if index==other_index {continue}
            for horizontal in other.windows(2){
                let c=horizontal[0];let d=horizontal[1];
                if (c.y-d.y).abs()>0.1{continue}
                if a.x>c.x.min(d.x)+7.0 && a.x<c.x.max(d.x)-7.0 &&
                    c.y>a.y.min(b.y)+9.0 && c.y<a.y.max(b.y)-9.0 {crossings.push(c.y);}
            }
        }
        crossings.sort_by(|x,y|if direction>0.0{x.total_cmp(y)}else{y.total_cmp(x)});
        crossings.dedup_by(|x,y|(*x-*y).abs()<13.0);
        let mut last=a.y;
        for y in crossings {
            if (y-last)*direction<12.0 || (b.y-y)*direction<9.0{continue}
            let radius=6.0;
            result.push(egui::pos2(a.x,y-radius*direction));
            for step in 1..=12{
                let theta=std::f32::consts::PI*(step as f32/12.0);
                result.push(egui::pos2(a.x+radius*theta.sin(),y-radius*direction*theta.cos()));
            }
            last=y+radius*direction;
        }
        result.push(b);
    }
    result
}

/// Explain the port contract and the exact validation outcome for the selected source.
fn port_hint(graph: &Graph, to: NodeId, to_port: usize,
    pending: Option<(NodeId, usize)>, dragging: Option<(NodeId, usize)>) -> String {
    let Some(node) = graph.node(to) else { return "Destination node missing".into() };
    let Some(port) = node.definition().inputs.get(to_port) else { return "Input port missing".into() };
    let name=&port.name;let ty=&port.ty;
    let occupied = graph.input_connection(to, to_port).is_some();
    let mut hint = format!("{} input ({})\nOne incoming connection; a new source replaces the old one.", name, ty);
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
fn node_header_color(module:&Module)->egui::Color32 {
    egui::Color32::from_rgb(module.header_color[0],module.header_color[1],module.header_color[2])
}

fn module_library_tooltip(ui:&mut egui::Ui,module:&Module){
    ui.heading(&module.name);ui.small(&module.category);ui.separator();
    ui.label(&module.description);ui.add_space(5.0);
    ui.strong("Inputs");
    if module.inputs.is_empty(){ui.label("None");}
    for p in &module.inputs{ui.label(format!("{} ({})",p.name,p.ty));}
    ui.strong("Outputs");
    if module.outputs.is_empty(){ui.label("None (terminal node)");}
    for p in &module.outputs{ui.label(format!("{} ({})",p.name,p.ty));}
    ui.strong("Parameters");
    if module.parameters.is_empty(){ui.label("None");}
    for p in &module.parameters{ui.label(format!("{} ({})",p.name,p.ty));}
    ui.strong("Typical uses");ui.label(&module.uses);
    ui.strong("Behavioral notes");ui.label(&module.notes);
    ui.separator();ui.label(format!("Created by: {}",module.created_by));
    if module.trust==Trust::Community{
        ui.colored_label(egui::Color32::YELLOW,"This is a third-party module, not provided by Screenshaver.");
    }else{ui.small("Official Screenshaver Library");}
}
