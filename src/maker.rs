//! Standalone Maker: isolated SDL2/OpenGL runtime, no Screenshaver services.
use sdl2::video::GLProfile;
use std::time::{Duration,Instant};
use std::sync::mpsc::{self, SyncSender, Receiver, TryRecvError};
use crate::maker_graph::Graph;
use crate::maker_ui::{CandidateJob, CandidateAnalysis};

// The worker owns an independent hidden SDL2 window and OpenGL context.
// No GL object or SDL window is shared between threads.
enum WorkerRequest {
    Candidate { epoch:u64, job:CandidateJob },
    Motion { epoch:u64, index:usize, graph:Graph },
    Analysis { epoch:u64, index:usize, graph:Graph, name:String, size:(i32,i32), time:f32 },
}
enum WorkerReply {
    Candidate { epoch:u64, job:CandidateJob, result:Result<Option<Vec<u8>>,WorkerError> },
    Motion { epoch:u64, index:usize, result:Result<f32,String> },
    Analysis { epoch:u64, index:usize, result:CandidateAnalysis },
    Failed(String),
}
enum WorkerError { Glsl(String), Render(String) }

// A dedicated OS process owns SDL2 and OpenGL. The bridge thread handles only
// pipes; it never initializes SDL2 or touches GL objects.
fn graph_to_wire(graph:&Graph)->serde_json::Value {
    serde_json::json!({"nodes":graph.nodes.iter().map(|n|serde_json::json!({
        "id":n.id,"module_id":n.module_id,"pos":n.pos,"expanded":n.expanded,
        "parameters":n.parameters})).collect::<Vec<_>>(),
        "connections":graph.connections.iter().map(|c|serde_json::json!({
        "from":c.from,"from_port":c.from_port,"to":c.to,"to_port":c.to_port
        })).collect::<Vec<_>>(),"revision":graph.revision})
}
fn graph_from_wire(v:&serde_json::Value)->Result<Graph,String>{
    use std::collections::HashMap;
    let nodes=v["nodes"].as_array().ok_or("Missing nodes")?;
    let mut graph=Graph::blank();
    let terminal=graph.nodes[0].id;
    let mut ids=HashMap::new();
    for n in nodes {
        let id=n["id"].as_u64().ok_or("Invalid node ID")?;
        let module=n["module_id"].as_str().ok_or("Invalid module ID")?;
        let new_id=if module=="screenshaver.output.color" {terminal} else {
            graph.add(module,[n["pos"][0].as_f64().unwrap_or(0.0) as f32,
                              n["pos"][1].as_f64().unwrap_or(0.0) as f32])?
        };
        if ids.insert(id,new_id).is_some(){return Err("Duplicate node ID".into())}
        let node=graph.node_mut(new_id).ok_or("Missing reconstructed node")?;
        node.expanded=n["expanded"].as_bool().unwrap_or(true);
        if let Some(params)=n["parameters"].as_object(){
            node.parameters=params.iter().map(|(k,v)|(k.clone(),v.clone())).collect();
        }
    }
    for c in v["connections"].as_array().ok_or("Missing connections")? {
        let from=*ids.get(&c["from"].as_u64().ok_or("Invalid source")?).ok_or("Missing source")?;
        let to=*ids.get(&c["to"].as_u64().ok_or("Invalid destination")?).ok_or("Missing destination")?;
        graph.connect(from,c["from_port"].as_u64().ok_or("Invalid source port")? as usize,
                      to,c["to_port"].as_u64().ok_or("Invalid target port")? as usize)?;
    }
    graph.revision=v["revision"].as_u64().unwrap_or(graph.revision);
    graph.validate()?;
    Ok(graph)
}
fn wire_request(request:&WorkerRequest)->serde_json::Value{
    match request {
        WorkerRequest::Candidate{epoch,job}=>serde_json::json!({"kind":"candidate","epoch":epoch,"graph":graph_to_wire(&job.graph)}),
        WorkerRequest::Motion{epoch,graph,..}=>serde_json::json!({"kind":"motion","epoch":epoch,"graph":graph_to_wire(graph)}),
        WorkerRequest::Analysis{epoch,graph,name,size,time,..}=>serde_json::json!({"kind":"analysis","epoch":epoch,"graph":graph_to_wire(graph),"name":name,"width":size.0,"height":size.1,"time":time}),
    }
}
fn read_wire_line(reader:&mut impl std::io::BufRead)->Result<serde_json::Value,String>{
    let mut line=String::new();
    let n=reader.read_line(&mut line).map_err(|e|e.to_string())?;
    if n==0{return Err("Candidate worker closed its response pipe".into())}
    if n>2_000_000{return Err("Candidate worker response exceeded limit".into())}
    serde_json::from_str(&line).map_err(|e|format!("Invalid candidate worker response: {e}"))
}
fn spawn_candidate_worker() -> (SyncSender<WorkerRequest>,Receiver<WorkerReply>) {
    let (requests_tx,requests_rx)=mpsc::sync_channel::<WorkerRequest>(1);
    let (replies_tx,replies_rx)=mpsc::sync_channel::<WorkerReply>(1);
    std::thread::Builder::new().name("maker-candidate-ipc".into()).spawn(move || {
        let result=(||->Result<(),String>{
            use std::io::{BufReader,Write};
            use std::process::{Command,Stdio};
            let exe=std::env::current_exe().map_err(|e|e.to_string())?;
            let mut child=Command::new(exe).arg("--maker-candidate-worker")
                .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit())
                .spawn().map_err(|e|format!("Unable to start candidate worker process: {e}"))?;
            eprintln!("[MAKER WORKER] spawned child pid={}",child.id());
            let mut input=child.stdin.take().ok_or("Missing worker stdin")?;
            let mut output=BufReader::new(child.stdout.take().ok_or("Missing worker stdout")?);
            // Child confirms SDL2/GL readiness before any candidate is sent.
            let ready=read_wire_line(&mut output)?;
            if ready["status"]!="ready" {return Err(format!("Worker startup failed: {}",ready))}
            while let Ok(request)=requests_rx.recv() {
                let encoded=wire_request(&request).to_string();
                if encoded.len()>2_000_000{return Err("Candidate request exceeded limit".into())}
                input.write_all(encoded.as_bytes()).map_err(|e|e.to_string())?;
                input.write_all(b"\n").map_err(|e|e.to_string())?;
                input.flush().map_err(|e|e.to_string())?;
                let response=read_wire_line(&mut output)?;
                let reply=match request {
                    WorkerRequest::Candidate{epoch,job}=>{
                        let result=if let Some(error)=response["glsl_error"].as_str(){Err(WorkerError::Glsl(error.into()))}
                        else if let Some(error)=response["error"].as_str(){Err(WorkerError::Render(error.into()))}
                        else if response["uniform"].as_bool()==Some(true){Ok(None)}
                        else {let values=response["pixels"].as_array().ok_or("Missing thumbnail pixels")?;
                            let pixels=values.iter().map(|v|v.as_u64().and_then(|n|u8::try_from(n).ok()).ok_or("Invalid pixel"))
                                .collect::<Result<Vec<_>,_>>()?;
                            if pixels.len()!=192*108*4{return Err("Incorrect thumbnail size".into())}
                            Ok(Some(pixels))};
                        WorkerReply::Candidate{epoch,job,result}
                    }
                    WorkerRequest::Motion{epoch,index,..}=>WorkerReply::Motion{epoch,index,
                        result:response["score"].as_f64().map(|x|x as f32).ok_or_else(||response["error"].as_str().unwrap_or("Missing motion score").to_owned())},
                    WorkerRequest::Analysis{epoch,index,name,..}=>WorkerReply::Analysis{epoch,index,
                        result:CandidateAnalysis{name,lines:response["lines"].as_array().ok_or("Missing analysis lines")?
                            .iter().filter_map(|x|x.as_str().map(str::to_owned)).collect()}},
                };
                if replies_tx.send(reply).is_err(){break}
            }
            drop(input); // EOF requests worker shutdown.
            let _=child.kill(); // Never wait on a blocked GL driver call.
            let _=child.wait();
            Ok(())
        })();
        if let Err(error)=result {
            eprintln!("[MAKER WORKER] process failed: {error}");
            let _=replies_tx.try_send(WorkerReply::Failed(error));
        }
    }).expect("Failed to start Maker IPC thread");
    (requests_tx,replies_rx)
}

// Called only in a separate process, before normal argument parsing.
pub fn run_candidate_worker()->Result<(),String>{
    use std::io::{BufRead,BufReader,Write};
    let sdl=sdl2::init()?;
    let video=sdl.video()?;
    {let a=video.gl_attr();a.set_context_profile(GLProfile::Core);a.set_context_version(3,3);}
    let window=video.window("Screenshaver Maker candidate renderer",256,256)
        .hidden().opengl().build().map_err(|e|e.to_string())?;
    let _context=window.gl_create_context()?;
    gl::load_with(|name|video.gl_get_proc_address(name) as *const _);
    crate::maker_library::initialize()?;
    let mut preview=crate::maker_preview::Preview::new()?;
    let stdout=std::io::stdout();let mut output=stdout.lock();
    writeln!(output,"{{\"status\":\"ready\"}}").map_err(|e|e.to_string())?;
    output.flush().map_err(|e|e.to_string())?;
    for line in BufReader::new(std::io::stdin()).lines(){
        let line=line.map_err(|e|e.to_string())?;
        if line.len()>2_000_000{return Err("Oversized candidate request".into())}
        let request:serde_json::Value=serde_json::from_str(&line).map_err(|e|e.to_string())?;
        let result=(||->Result<serde_json::Value,String>{
            let graph=graph_from_wire(&request["graph"])?;
            match request["kind"].as_str().ok_or("Missing request kind")? {
                "candidate"=>match crate::maker_compiler::fragment_source(&graph){
                    Err(e)=>Ok(serde_json::json!({"glsl_error":e})),
                    Ok(source)=>match preview.candidate_thumbnail_with_source(&graph,&source)? {
                        Some(pixels)=>Ok(serde_json::json!({"pixels":pixels})),
                        None=>Ok(serde_json::json!({"uniform":true}))
                    }
                },
                "motion"=>Ok(serde_json::json!({"score":preview.motion_activity(&graph)?})),
                "analysis"=>{let report=preview.analyze_candidate(&graph,
                    request["name"].as_str().unwrap_or("Candidate").to_owned(),
                    (request["width"].as_i64().unwrap_or(192) as i32,
                     request["height"].as_i64().unwrap_or(108) as i32),
                    request["time"].as_f64().unwrap_or(0.0) as f32);
                    Ok(serde_json::json!({"lines":report.lines}))},
                _=>Err("Unknown worker request".into()),
            }
        })();
        let reply=match result{Ok(value)=>value,Err(e)=>serde_json::json!({"error":e})};
        writeln!(output,"{}",reply).map_err(|e|e.to_string())?;
        output.flush().map_err(|e|e.to_string())?;
    }
    preview.destroy();Ok(())
}

pub fn run()->Result<(),String>{
    let sdl=sdl2::init()?;let video=sdl.video()?;
    {let a=video.gl_attr();a.set_context_profile(GLProfile::Core);a.set_context_version(3,3);a.set_double_buffer(true);}
    let window=video.window("Screenshaver Maker — graph prototype",1440,850).position_centered().resizable().opengl().build().map_err(|e|e.to_string())?;
    let _gl_context=window.gl_create_context()?;
    let _=video.gl_set_swap_interval(sdl2::video::SwapInterval::VSync);
    gl::load_with(|s|video.gl_get_proc_address(s) as *const _);
    crate::maker_library::initialize()?;
    for error in &crate::maker_library::get().errors { eprintln!("[MAKER MODULE] {error}"); }
    let mut ui=crate::maker_ui::MakerUi::new(&video)?;
    // Maker owns its SDL2 event loop; egui clipboard requests are not
    // automatically forwarded to the native system clipboard.
    let clipboard=video.clipboard();
    let mut preview=crate::maker_preview::Preview::new()?;
    let (worker_tx,worker_rx)=spawn_candidate_worker();
    let mut worker_busy=false;
    let mut worker_failed=false;
    let mut events=sdl.event_pump()?;let start=Instant::now();
    let mut diagnostic_frame: u64=0;
    eprintln!("[MAKER TIMING] enabled; slow stage threshold 100 ms; GL compile/readback start/end markers");
    'running:loop{
        diagnostic_frame+=1;
        let frame_started=Instant::now();
        let event_started=Instant::now();
        for event in events.poll_iter(){
            if matches!(event,sdl2::event::Event::Quit{..}){break 'running}
            if let sdl2::event::Event::KeyDown{keycode:Some(sdl2::keyboard::Keycode::Escape),..}=event{break 'running}
            ui.handle_event(&event);
        }
        maker_timing("events",diagnostic_frame,event_started);
        let (dw, dh) = window.drawable_size();
        let (ww, wh) = window.size();
        let scale_x = if ww > 0 { dw as f32 / ww as f32 } else { 1.0 };
        let scale_y = if wh > 0 { dh as f32 / wh as f32 } else { 1.0 };
        let scale = scale_x.min(scale_y);
        // Never wait for candidate GPU work on the UI thread. At most one
        // in-flight request and one buffered reply: bounded CPU/GPU work.
        loop {
            match worker_rx.try_recv() {
                Ok(reply)=>{
                    worker_busy=false;
                    match reply {
                        WorkerReply::Candidate{epoch,job,result}=>{
                            if epoch==ui.candidate_epoch() {
                                match result {
                                    Ok(Some(pixels))=>ui.receive_candidate(job,pixels,192,108),
                                    Ok(None)=>ui.candidate_rejected_uniform(),
                                    Err(WorkerError::Glsl(e))=>{
                                        eprintln!("[MAKER CANDIDATE GLSL] {}: {}",job.name,e);
                                        ui.candidate_rejected_glsl();
                                    }
                                    Err(WorkerError::Render(e))=>{
                                        eprintln!("[MAKER CANDIDATE RENDER] {}: {}",job.name,e);
                                        ui.candidate_rejected_render();
                                    }
                                }
                            }
                        }
                        WorkerReply::Motion{epoch,index,result}=>{
                            if epoch==ui.candidate_epoch(){ui.receive_motion_score(index,result)}
                        }
                        WorkerReply::Analysis{epoch,index,result}=>{
                            if epoch==ui.candidate_epoch(){ui.receive_analysis(index,result)}
                        }
                        WorkerReply::Failed(error)=>{
                            eprintln!("[MAKER WORKER] unavailable: {error}");
                            worker_failed=true;
                        }
                    }
                }
                Err(TryRecvError::Empty)=>break,
                Err(TryRecvError::Disconnected)=>{
                    worker_failed=true;
                    break;
                }
            }
        }
        if !worker_busy && !worker_failed {
            let epoch=ui.candidate_epoch();
            let request=if let Some((index,graph,name))=ui.next_analysis_job(){
                Some(WorkerRequest::Analysis{epoch,index,graph,name,
                    size:(ui.preview_rect().width().max(1.0) as i32,
                          ui.preview_rect().height().max(1.0) as i32),
                    time:start.elapsed().as_secs_f32()})
            } else if let Some((index,graph))=ui.next_motion_job(){
                Some(WorkerRequest::Motion{epoch,index,graph})
            } else {
                ui.next_candidate_job().map(|job|WorkerRequest::Candidate{epoch,job})
            };
            if let Some(request)=request {
                match worker_tx.try_send(request){
                    Ok(())=>worker_busy=true,
                    Err(mpsc::TrySendError::Full(_))=>{
                        // Single-flight protocol makes this unreachable.
                        eprintln!("[MAKER WORKER] request channel unexpectedly full");
                        worker_busy=true;
                    }
                    Err(mpsc::TrySendError::Disconnected(_))=>worker_failed=true,
                }
            }
        }
        let preview_started=Instant::now();
        preview.update(ui.graph());
        preview.render(dw as i32,dh as i32,scale,ui.preview_rect(),start.elapsed().as_secs_f32(),ui.graph());
        maker_timing("preview",diagnostic_frame,preview_started);
        let ui_started=Instant::now();
        if !ui.frame(dw,dh,scale,preview.error.as_deref()){break}
        maker_timing("ui-frame",diagnostic_frame,ui_started);
        if let Some((source,save))=ui.take_glsl_export(){
            if save {
                let fingerprint=source.bytes().fold(14695981039346656037u64,|h,b|(h^b as u64).wrapping_mul(1099511628211));
                let result=(||->Result<std::path::PathBuf,String>{
                    let home=std::env::var_os("HOME").ok_or("HOME is not set")?;
                    let dir=std::path::PathBuf::from(home).join(".config/screenshaver/maker/exports");
                    std::fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
                    let path=dir.join(format!("maker_candidate_{fingerprint:016x}.glsl"));
                    std::fs::write(&path,source.as_bytes()).map_err(|e|e.to_string())?;
                    Ok(path)
                })();
                ui.glsl_export_result(match result {Ok(path)=>format!("GLSL saved: {}",path.display()),Err(e)=>format!("Save GLSL failed: {e}")});
            } else {
                ui.glsl_export_result(match clipboard.set_clipboard_text(&source){Ok(())=>"Candidate GLSL copied to system clipboard".into(),Err(e)=>format!("Copy GLSL failed: {e}")});
            }
        }
        if let Some(report)=ui.take_clipboard_request(){
            ui.clipboard_result(clipboard.set_clipboard_text(&report));
        }
        let swap_started=Instant::now();
        window.gl_swap_window();
        maker_timing("swap",diagnostic_frame,swap_started);
        maker_timing("whole-frame",diagnostic_frame,frame_started);
        std::thread::sleep(Duration::from_millis(8));
    }
    // Do not join a worker blocked inside a GPU driver call: the process must
    // remain able to exit. Dropping the sender requests shutdown when idle.
    drop(worker_tx);
    ui.destroy();preview.destroy();Ok(())
}

fn maker_timing(stage:&str,frame:u64,start:Instant){
    let elapsed=start.elapsed();
    if elapsed>=Duration::from_millis(100){
        eprintln!("[MAKER TIMING] frame={frame} stage={stage} elapsed_ms={:.1}",elapsed.as_secs_f64()*1000.0);
    }
}
