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
    Interrupted { request:WorkerRequest, error:String },
}
enum WorkerError { Glsl(String), Render(String) }

// Linux worker containment: hard virtual-address-space cap and parent-death
// SIGKILL.  These syscalls are deliberately local to Maker's child process.
#[cfg(target_os="linux")]
mod worker_limits {
    use std::os::raw::{c_int,c_ulong};
    const PR_SET_PDEATHSIG:c_int=1;
    const SIGKILL:c_int=9;
    const RLIMIT_AS:c_int=9;
    #[repr(C)] struct RLimit {cur:u64,max:u64}
    unsafe extern "C" {
        fn prctl(option:c_int, ...) -> c_int;
        fn getppid()->c_int;
        fn setrlimit(resource:c_int,rlim:*const RLimit)->c_int;
        fn kill(pid:c_int,sig:c_int)->c_int;
    }
    pub fn child_setup()->Result<(),String>{
        let parent=unsafe{getppid()};
        if parent<=1{return Err("Maker parent already exited".into())}
        if unsafe{prctl(PR_SET_PDEATHSIG,SIGKILL as c_ulong,0usize,0usize,0usize)}!=0 {
            return Err(format!("PR_SET_PDEATHSIG: {}",std::io::Error::last_os_error()))
        }
        if unsafe{getppid()}!=parent{return Err("Maker parent exited during worker startup".into())}
        // 4 GiB address-space limit.  If the driver needs more, the worker
        // fails independently rather than exhausting the entire machine.
        let limit=RLimit{cur:4*1024*1024*1024,max:4*1024*1024*1024};
        if unsafe{setrlimit(RLIMIT_AS,&limit)}!=0 {
            return Err(format!("RLIMIT_AS: {}",std::io::Error::last_os_error()))
        }
        Ok(())
    }
    pub fn terminate(pid:u32){if pid>1{unsafe{kill(pid as c_int,SIGKILL);}}}
}
#[cfg(not(target_os="linux"))]
mod worker_limits {
    pub fn child_setup()->Result<(),String>{Err("Candidate worker containment requires Linux".into())}
    pub fn terminate(_pid:u32){}
}

// A worker PID is published only while its Child handle remains owned by the
// IPC thread.  The UI's Drop guard kills it even when GPU compilation blocks.
struct WorkerProcessGuard(std::sync::Arc<std::sync::atomic::AtomicU32>);
impl Drop for WorkerProcessGuard {
    fn drop(&mut self){
        use std::sync::atomic::Ordering;
        let pid=self.0.swap(0,Ordering::SeqCst);
        worker_limits::terminate(pid);
    }
}

// A dedicated OS process owns SDL2 and OpenGL. The bridge thread handles only
// pipes; it never initializes SDL2 or touches GL objects.
pub(crate) fn graph_to_wire(graph:&Graph)->serde_json::Value {
    serde_json::json!({"nodes":graph.nodes.iter().map(|n|serde_json::json!({
        "id":n.id,"module_id":n.module_id,"pos":n.pos,"expanded":n.expanded,
        "parameters":n.parameters})).collect::<Vec<_>>(),
        "connections":graph.connections.iter().map(|c|serde_json::json!({
        "from":c.from,"from_port":c.from_port,"to":c.to,"to_port":c.to_port
        })).collect::<Vec<_>>(),"revision":graph.revision})
}
pub(crate) fn graph_from_wire(v:&serde_json::Value)->Result<Graph,String>{
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
// Persist the exact shader and graph BEFORE entering the native OpenGL compiler.
// A worker killed by the watchdog or SIGSEGV cannot write a postmortem report.
fn preserve_candidate_source(graph:&Graph, source:&str)->Result<(),String>{
    use std::io::Write;
    let fingerprint=source.bytes().fold(14695981039346656037u64,
        |h,b|(h^b as u64).wrapping_mul(1099511628211));
    let home=std::env::var_os("HOME").ok_or("HOME not set")?;
    let dir=std::path::PathBuf::from(home).join(".config/screenshaver/maker/diagnostics/candidates");
    std::fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
    let stem=format!("candidate_{fingerprint:016x}");
    let source_path=dir.join(format!("{stem}.glsl"));
    let graph_path=dir.join(format!("{stem}.json"));
    // Write once per fingerprint, without replacing earlier diagnostic evidence.
    if !source_path.exists(){
        let mut f=std::fs::OpenOptions::new().write(true).create_new(true)
            .open(&source_path).map_err(|e|e.to_string())?;
        f.write_all(source.as_bytes()).map_err(|e|e.to_string())?;
    }
    if !graph_path.exists(){
        let data=serde_json::to_vec_pretty(&graph_to_wire(graph)).map_err(|e|e.to_string())?;
        let mut f=std::fs::OpenOptions::new().write(true).create_new(true)
            .open(&graph_path).map_err(|e|e.to_string())?;
        f.write_all(&data).map_err(|e|e.to_string())?;
    }
    eprintln!("[MAKER GLSL SNAPSHOT] fingerprint={fingerprint:016x} bytes={} source={} graph={}",
        source.len(),source_path.display(),graph_path.display());
    Ok(())
}

// Conservative structural complexity guard. Byte size is not a proxy for
// compiler work; repeated field sampling and callable fan-out also matter.
fn candidate_complexity_gate(source:&str)->Result<(),String>{
    let callables=source.matches("(vec3 maker_position){").count();
    let loop_sites=source.matches("for(int ").count();
    let sdf_marchers=source.matches("for(int i=0;i<128;i++)").count();
    let optical_marchers=source.matches("for(int k=0;k<96;k++)").count();
    eprintln!("[MAKER GLSL PREFLIGHT] bytes={} callable_definitions={} loop_sites={} sdf_marchers={} optical_marchers={}",
        source.len(),callables,loop_sites,sdf_marchers,optical_marchers);
    if source.len()>64*1024 {return Err(format!("GLSL source budget exceeded: {} bytes",source.len()))}
    // Conservative hard ceilings; these are safety budgets, not visual-quality filters.
    if callables>96 || loop_sites>24 || sdf_marchers+optical_marchers>4 {
        return Err(format!("GLSL structural safety budget exceeded: {callables} callables, {loop_sites} loops, {} marchers",sdf_marchers+optical_marchers));
    }
    Ok(())
}

fn spawn_candidate_worker() -> (SyncSender<WorkerRequest>,Receiver<WorkerReply>,WorkerProcessGuard) {
    use std::sync::atomic::{AtomicU32,Ordering};
    let active_pid=std::sync::Arc::new(AtomicU32::new(0));
    let guard=WorkerProcessGuard(active_pid.clone());
    let thread_pid=active_pid.clone();
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
            struct ReapChild(std::process::Child);
            impl Drop for ReapChild {fn drop(&mut self){let _=self.0.kill();let _=self.0.wait();}}
            let mut child=ReapChild(child);
            thread_pid.store(child.0.id(),Ordering::SeqCst);
            eprintln!("[MAKER WORKER] spawned child pid={} (4 GiB address-space limit; 15s watchdog)",child.0.id());
            // A detached watchdog never calls GL or blocks the UI.  It kills
            // this exact worker if startup or a request exceeds its deadline.
            let pending=std::sync::Arc::new(std::sync::Mutex::new(Some(Instant::now())));
            let watch_pending=pending.clone();
            let watch_pid=thread_pid.clone();
            std::thread::Builder::new().name("maker-worker-watchdog".into()).spawn(move || {
                loop {
                    std::thread::sleep(Duration::from_millis(200));
                    if watch_pid.load(Ordering::SeqCst)==0 {break}
                    let overdue=watch_pending.lock().map(|p|p.map(|t|t.elapsed()>Duration::from_secs(15)).unwrap_or(false)).unwrap_or(true);
                    if overdue {
                        eprintln!("[MAKER WORKER] 15s deadline exceeded; killing worker pid={}",watch_pid.load(Ordering::SeqCst));
                        worker_limits::terminate(watch_pid.load(Ordering::SeqCst));
                        break;
                    }
                }
            }).map_err(|e|e.to_string())?;
            let mut input=child.0.stdin.take().ok_or("Missing worker stdin")?;
            let mut output=BufReader::new(child.0.stdout.take().ok_or("Missing worker stdout")?);
            // Child confirms SDL2/GL readiness before any candidate is sent.
            let ready=read_wire_line(&mut output)?;
            if ready["status"]!="ready" {return Err(format!("Worker startup failed: {}",ready))}
            *pending.lock().map_err(|e|e.to_string())?=None;
            while let Ok(request)=requests_rx.recv() {
                let encoded=wire_request(&request).to_string();
                if encoded.len()>2_000_000{return Err("Candidate request exceeded limit".into())}
                *pending.lock().map_err(|e|e.to_string())?=Some(Instant::now());
                let response=(||->Result<serde_json::Value,String>{
                    input.write_all(encoded.as_bytes()).map_err(|e|e.to_string())?;
                    input.write_all(b"\n").map_err(|e|e.to_string())?;
                    input.flush().map_err(|e|e.to_string())?;
                    read_wire_line(&mut output)
                })();
                let response=match response {
                    Ok(value)=>value,
                    Err(error)=>{
                        eprintln!("[MAKER WORKER] interrupted request: {error}");
                        let exit_detail=match child.0.try_wait() {
                            Ok(Some(status))=>format!("; child exit status: {status}"),
                            Ok(None)=>"; child status not yet available".to_owned(),
                            Err(e)=>format!("; child wait error: {e}"),
                        };
                        let _=replies_tx.send(WorkerReply::Interrupted{request,error:format!("{error}{exit_detail}")});
                        return Ok(());
                    }
                };
                *pending.lock().map_err(|e|e.to_string())?=None;
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
            let _=child.0.kill(); // Force termination before reaping.
            let _=child.0.wait();
            thread_pid.store(0,Ordering::SeqCst);
            Ok(())
        })();
        thread_pid.store(0,Ordering::SeqCst);
        if let Err(error)=result {
            eprintln!("[MAKER WORKER] process failed: {error}");
            let _=replies_tx.try_send(WorkerReply::Failed(error));
        }
    }).expect("Failed to start Maker IPC thread");
    (requests_tx,replies_rx,guard)
}

// Called only in a separate process, before normal argument parsing.
pub fn run_candidate_worker()->Result<(),String>{
    use std::io::{BufRead,BufReader,Write};
    worker_limits::child_setup()?;
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
                "candidate"=>{
                if let Err(reason)=crate::maker_ui::candidate_admission(&graph) {
                    eprintln!("[MAKER GRAPH COMPLEXITY] {reason}");
                    Ok(serde_json::json!({"glsl_error":reason}))
                } else {match crate::maker_compiler::fragment_source(&graph){
                    Err(e)=>Ok(serde_json::json!({"glsl_error":e})),
                    Ok(source)=>{
                        // Snapshot is mandatory: never submit unreproducible GLSL to GL.
                        preserve_candidate_source(&graph,&source)?;
                        match candidate_complexity_gate(&source){
                            Err(error)=>Ok(serde_json::json!({"glsl_error":error})),
                            Ok(())=>match preview.candidate_thumbnail_with_source(&graph,&source)? {
                        Some(pixels)=>Ok(serde_json::json!({"pixels":pixels})),
                        None=>Ok(serde_json::json!({"uniform":true}))
                            }
                        }
                    }
                }}},
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
    if std::env::var("SCREENSHAVER_MAKER_DISCOVERY_TEST").ok().as_deref()==Some("1"){
        crate::maker_library::initialize()?;
        return crate::maker_ui::run_discovery_test();
    }
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
    let (mut worker_tx,mut worker_rx,mut worker_process_guard)=spawn_candidate_worker();
    let mut worker_busy=false;
    let mut worker_failed=false;
    let mut retry_request:Option<WorkerRequest>=None;
    let mut consecutive_interruptions:u32=0;
    let mut gpu_circuit_open=false;
    const MAX_CONSECUTIVE_INTERRUPTS:u32=3;
    let mut restarts:u32=0;
    const MAX_WORKER_RESTARTS:u32=8;
    let mut next_restart=Instant::now();
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
                    if !matches!(&reply,WorkerReply::Failed(_)|WorkerReply::Interrupted{..}) {
                        consecutive_interruptions=0;
                    }
                    match reply {
                        WorkerReply::Candidate{epoch,job,result}=>{
                            if epoch==ui.candidate_epoch() {
                                match result {
                                    Ok(Some(pixels))=>{ui.candidate_glsl_capable();ui.receive_candidate(job,pixels,192,108)},
                                    Ok(None)=>{ui.candidate_glsl_capable();ui.candidate_rejected_uniform()},
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
                        WorkerReply::Interrupted{request,error}=>{
                            eprintln!("[MAKER WORKER] request interrupted; quarantining request: {error}");
                            consecutive_interruptions+=1;
                            {
                                eprintln!("[MAKER WORKER] abandoning interrupted request without replay");
                                match request {
                                    WorkerRequest::Candidate{epoch,..} if epoch==ui.candidate_epoch()=>ui.candidate_rejected_render(),
                                    WorkerRequest::Motion{epoch,index,..} if epoch==ui.candidate_epoch()=>ui.receive_motion_score(index,Err("Worker resource limit or timeout exceeded".into())),
                                    WorkerRequest::Analysis{epoch,index,name,..} if epoch==ui.candidate_epoch()=>ui.receive_analysis(index,CandidateAnalysis{name,lines:vec!["Worker resource limit or timeout exceeded".into()]}),
                                    _=>{}
                                }
                            }
                            worker_failed=true;
                            if consecutive_interruptions>=MAX_CONSECUTIVE_INTERRUPTS {
                                gpu_circuit_open=true;
                                eprintln!("[MAKER WORKER] GPU CIRCUIT OPEN after {consecutive_interruptions} consecutive interruptions; pending candidates retained; restart Maker to reset");
                            }
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
        if worker_failed && !gpu_circuit_open && Instant::now()>=next_restart && restarts<MAX_WORKER_RESTARTS {
            drop(worker_tx);
            drop(worker_process_guard);
            let (tx,rx,guard)=spawn_candidate_worker();
            worker_tx=tx; worker_rx=rx; worker_process_guard=guard;
            worker_failed=false;
            worker_busy=false;
            restarts+=1;
            next_restart=Instant::now()+Duration::from_secs(2);
            eprintln!("[MAKER WORKER] restarting worker ({restarts}/{MAX_WORKER_RESTARTS})");
        }
        if !worker_busy && !worker_failed && !gpu_circuit_open {
            let epoch=ui.candidate_epoch();
            if retry_request.as_ref().is_some_and(|r|match r {
                WorkerRequest::Candidate{epoch:e,..}|WorkerRequest::Motion{epoch:e,..}|
                WorkerRequest::Analysis{epoch:e,..}=>*e!=epoch
            }) { retry_request=None; }
            let request=if let Some(retry)=retry_request.take(){Some(retry)}
            else if let Some((index,graph,name))=ui.next_analysis_job(){
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
                    Err(mpsc::TrySendError::Disconnected(request))=>{
                        retry_request=Some(request);
                        worker_failed=true;
                    },
                }
            }
        }
        let preview_started=Instant::now();
        if ui.preview_active() {
            preview.update(ui.graph());
            preview.render(dw as i32,dh as i32,scale,ui.preview_rect(),start.elapsed().as_secs_f32(),ui.graph());
        } else {
            // Stopping the shader does not erase pixels already in the SDL
            // default framebuffer. The preview panel is translucent, so those
            // pixels otherwise survive as a static image after Close Project.
            // Clear the entire drawable before egui paints the blank panel;
            // leave the panel's geometry unchanged.
            unsafe {
                gl::Disable(gl::SCISSOR_TEST);
                gl::ClearColor(0.0, 0.0, 0.0, 1.0);
                gl::Clear(gl::COLOR_BUFFER_BIT);
            }
        }
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
    drop(worker_process_guard);
    ui.destroy();preview.destroy();Ok(())
}

fn maker_timing(stage:&str,frame:u64,start:Instant){
    let elapsed=start.elapsed();
    if elapsed>=Duration::from_millis(100){
        eprintln!("[MAKER TIMING] frame={frame} stage={stage} elapsed_ms={:.1}",elapsed.as_secs_f64()*1000.0);
    }
}
