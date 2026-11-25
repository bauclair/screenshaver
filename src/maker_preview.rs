//! OpenGL 3.3 preview with computational admission before compilation and drawing.
use std::ffi::CString;
use std::time::{Duration,Instant};
// Round upward: never submit more frequently than 30 Hz.
const LIVE_FRAME_INTERVAL:Duration=Duration::from_nanos(33_333_334);
use std::sync::atomic::{AtomicU64,Ordering};
static THUMBNAIL_ID: AtomicU64=AtomicU64::new(0);
use crate::maker_graph::Graph;
const VERT:&str="#version 330 core\nconst vec2 p[3]=vec2[3](vec2(-1,-1),vec2(3,-1),vec2(-1,3));\nvoid main(){gl_Position=vec4(p[gl_VertexID],0,1);}";
pub struct Preview { program:u32, vao:u32, revision:Option<(serde_json::Value,i32,i32)>, source:String, admitted:bool, query:u32, first_pending:bool, first_draw:bool, poll_started:Option<Instant>, last_poll:Instant, stats_at:Instant, samples:u64, max_gpu_ms:f64, pub error:Option<String>, fbo:u32, tex:u32, size:(i32,i32), fence:gl::types::GLsync, submitted:Instant, next_submission:Option<Instant>, ready:bool, suspended:bool }
impl Preview {
    pub fn new()->Result<Self,String>{unsafe{let mut vao=0;gl::GenVertexArrays(1,&mut vao);Ok(Self{program:0,vao,revision:None,source:String::new(),admitted:false,query:0,first_pending:false,first_draw:false,poll_started:None,last_poll:Instant::now(),stats_at:Instant::now(),samples:0,max_gpu_ms:0.0,error:None,fbo:0,tex:0,size:(0,0),fence:std::ptr::null(),submitted:Instant::now(),next_submission:None,ready:false,suspended:false})}}
    fn update(&mut self,graph:&Graph,w:i32,h:i32,t:f32){
        // Exact render identity, independent of project-local revision counters and layout.
        let nodes:Vec<_>=graph.nodes.iter().map(|n|serde_json::json!([n.id,n.module_id,n.parameters,n.invalid_reason])).collect();
        let edges:Vec<_>=graph.connections.iter().map(|c|(c.from,c.from_port,c.to,c.to_port)).collect();
        let key=(serde_json::json!([nodes,edges]),w,h);
        if self.revision.as_ref()==Some(&key){return}
        self.revision=Some(key);
        self.ready=false;self.admitted=false;
        let result=(||->Result<(),String>{
            crate::maker_ui::rendering_admission(graph,w,h)?;
            let source=crate::maker_compiler::fragment_source(graph)?;
            if self.program==0 || self.source!=source {
                crate::maker::candidate_complexity_gate(&source)?;
                crate::maker::record_activation("live","before-compile",graph,&source,w,h,t)?;
                let started=Instant::now();
                let program=link(VERT,&source)?;
                unsafe{if self.program!=0{gl::DeleteProgram(self.program);}}
                self.program=program;self.source=source;
                crate::maker::operational_log(format_args!("[MAKER GL] live compile/link completed ms={:.2}",started.elapsed().as_secs_f64()*1000.0));
            }
            Ok(())
        })();
        match result{Ok(())=>{self.admitted=true;self.error=None;self.first_draw=true},Err(e)=>{
            if self.error.as_deref()!=Some(e.as_str()){crate::maker::operational_log(format_args!("[MAKER SAFETY] preview deferred: {e}"));}
            self.error=Some(e)
        }}
    }
    // Poll first, even after presentation stalls. Never infer GPU execution time
    // from time spent away from the polling loop.
    pub fn poll_completion(&mut self){
        if self.suspended || self.fence.is_null(){return}
        unsafe{
            let now=Instant::now();
            let gap=now.duration_since(self.last_poll).as_millis();self.last_poll=now;
            let status=gl::ClientWaitSync(self.fence,0,0);
            if status==gl::WAIT_FAILED{self.suspend("GPU completion fence failed");return}
            if status!=gl::ALREADY_SIGNALED && status!=gl::CONDITION_SATISFIED{
                if gap>100 || self.poll_started.is_none(){self.poll_started=Some(now);}
                if self.poll_started.is_some_and(|p|p.elapsed().as_millis()>250){self.suspend("GPU fence remained pending during 250 ms of active polling");}
                return
            }
            let mut available=0;
            gl::GetQueryObjectiv(self.query,gl::QUERY_RESULT_AVAILABLE,&mut available);
            if available==0{
                if gap>100 || self.poll_started.is_none(){self.poll_started=Some(now);}
                if self.poll_started.is_some_and(|p|p.elapsed().as_millis()>250){self.suspend("GPU timer result unavailable during active polling");}
                return
            }
            let mut ns=0u64;gl::GetQueryObjectui64v(self.query,gl::QUERY_RESULT,&mut ns);
            gl::DeleteSync(self.fence);self.fence=std::ptr::null();self.ready=true;self.poll_started=None;
            let ms=ns as f64/1_000_000.0;
            if ms>250.0{self.suspend(&format!("Measured GPU draw exceeded 250 ms ({ms:.2} ms)"));return}
            self.samples+=1;self.max_gpu_ms=self.max_gpu_ms.max(ms);
            if self.first_pending{crate::maker::operational_log(format_args!("[MAKER GPU] initial frame completed gpu_ms={ms:.3} observed_latency_ms={:.1}",self.submitted.elapsed().as_secs_f64()*1000.0));self.first_pending=false;}
            if self.stats_at.elapsed().as_secs()>=10{crate::maker::operational_log(format_args!("[MAKER GPU] completed_frames={} max_gpu_ms={:.3} interval_ms={:.1}",self.samples,self.max_gpu_ms,now.duration_since(self.stats_at).as_secs_f64()*1000.0));self.stats_at=now;self.samples=0;self.max_gpu_ms=0.0;}
        }
    }
    pub fn render(&mut self,w:i32,h:i32,scale:f32,rect:egui::Rect,t:f32,graph:&Graph){
        unsafe{
            gl::Viewport(0,0,w,h);gl::Disable(gl::SCISSOR_TEST);gl::Disable(gl::DEPTH_TEST);
            gl::ClearColor(0.015,0.02,0.04,1.0);gl::Clear(gl::COLOR_BUFFER_BIT);
            // The GLSL shader is confined to the egui preview content rectangle.
            // egui coordinates are top-left origin; OpenGL is bottom-left origin.
            let x=(rect.min.x*scale).round() as i32;
            let y=h-(rect.max.y*scale).round() as i32;
            let pw=((rect.width()*scale).round() as i32).max(0);
            let ph=((rect.height()*scale).round() as i32).max(0);
            let x=x.clamp(0,w);let y=y.clamp(0,h);
            let pw=pw.min(w-x);let ph=ph.min(h-y);
            if pw<=0 || ph<=0{return}
            if self.suspended{return}
            if !self.fence.is_null(){return}
            // Render at the physical pixel dimensions of the preview canvas.
            // update() must admit this full resolution before allocation or draw.
            let (rw,rh)=(pw,ph);
            // Coalesce edits: only inspect/admit the latest graph when a draw
            // is eligible. Neither graph activation nor framebuffer readiness
            // may reset the submission deadline. Present the last completed
            // image during the short cooldown without executing its shader.
            if self.next_submission.is_some_and(|deadline|Instant::now()<deadline){
                if self.ready && self.admitted && self.size.0>0 && self.size.1>0{
                    gl::BindFramebuffer(gl::READ_FRAMEBUFFER,self.fbo);
                    gl::BlitFramebuffer(0,0,self.size.0,self.size.1,x,y,x+pw,y+ph,gl::COLOR_BUFFER_BIT,gl::LINEAR);
                    gl::BindFramebuffer(gl::READ_FRAMEBUFFER,0);
                }
                return
            }
            self.update(graph,rw,rh,t);
            if !self.admitted || self.program==0{return}
            if self.size!=(rw,rh){
                self.ready=false;
                if self.fbo==0{gl::GenFramebuffers(1,&mut self.fbo);gl::GenTextures(1,&mut self.tex);}
                gl::BindTexture(gl::TEXTURE_2D,self.tex);
                gl::TexParameteri(gl::TEXTURE_2D,gl::TEXTURE_MIN_FILTER,gl::LINEAR as i32);
                gl::TexParameteri(gl::TEXTURE_2D,gl::TEXTURE_MAG_FILTER,gl::LINEAR as i32);
                gl::TexImage2D(gl::TEXTURE_2D,0,gl::RGBA8 as i32,rw,rh,0,gl::RGBA,gl::UNSIGNED_BYTE,std::ptr::null());
                gl::BindFramebuffer(gl::FRAMEBUFFER,self.fbo);
                gl::FramebufferTexture2D(gl::FRAMEBUFFER,gl::COLOR_ATTACHMENT0,gl::TEXTURE_2D,self.tex,0);
                let ok=gl::CheckFramebufferStatus(gl::FRAMEBUFFER)==gl::FRAMEBUFFER_COMPLETE;
                gl::BindFramebuffer(gl::FRAMEBUFFER,0);gl::BindTexture(gl::TEXTURE_2D,0);
                if !ok{self.suspend("Preview framebuffer incomplete");return}
                self.size=(rw,rh);
            }
            if self.ready{
                gl::BindFramebuffer(gl::READ_FRAMEBUFFER,self.fbo);
                gl::BlitFramebuffer(0,0,rw,rh,x,y,x+pw,y+ph,gl::COLOR_BUFFER_BIT,gl::LINEAR);
                gl::BindFramebuffer(gl::READ_FRAMEBUFFER,0);
            }
            if self.query==0{
                if !gl::GenQueries::is_loaded() || !gl::BeginQuery::is_loaded() || !gl::EndQuery::is_loaded() || !gl::GetQueryObjectiv::is_loaded() || !gl::GetQueryObjectui64v::is_loaded() || !gl::DeleteQueries::is_loaded(){self.suspend("GPU timer queries unavailable");return}
                gl::GenQueries(1,&mut self.query);
                if self.query==0{self.suspend("Unable to allocate GPU timer query");return}
            }
            if self.first_draw{
                if let Err(e)=crate::maker::record_activation("live","before-first-draw",graph,&self.source,rw,rh,t){self.suspend(&e);return}
                self.first_draw=false;self.first_pending=true;
            }
            gl::BindFramebuffer(gl::FRAMEBUFFER,self.fbo);
            gl::Viewport(0,0,rw,rh);
            gl::UseProgram(self.program);
            uniform_1f(self.program,"u_time",t);uniform_2f(self.program,"u_resolution",rw as f32,rh as f32);
            uniform_2f(self.program,"u_viewport_origin",0.0,0.0);
            for n in &graph.nodes{
                for p in &n.definition().parameters {
                    if let Some(value)=n.parameters.get(&p.id){
                        let name=format!("u_n{}_{}",n.id,p.id);
                        match p.ty.as_str(){
                            "float"|"choice"=>if let Some(v)=value.as_f64(){uniform_1f(self.program,&name,v as f32)},
                            "color3"=>if let Some(a)=value.as_array(){
                                if a.len()==3{uniform_3f(self.program,&name,[
                                    a[0].as_f64().unwrap_or(0.0) as f32,
                                    a[1].as_f64().unwrap_or(0.0) as f32,
                                    a[2].as_f64().unwrap_or(0.0) as f32,
                                ])}
                            },
                            _=>{},
                        }
                    }
                }
            }
            self.submitted=Instant::now();
            self.next_submission=Some(self.submitted+LIVE_FRAME_INTERVAL);
            self.last_poll=self.submitted;self.poll_started=None;
            gl::BeginQuery(gl::TIME_ELAPSED,self.query);
            gl::BindVertexArray(self.vao);gl::DrawArrays(gl::TRIANGLES,0,3);
            gl::EndQuery(gl::TIME_ELAPSED);
            self.fence=gl::FenceSync(gl::SYNC_GPU_COMMANDS_COMPLETE,0);gl::Flush();
            gl::BindVertexArray(0);gl::UseProgram(0);gl::BindFramebuffer(gl::FRAMEBUFFER,0);gl::Viewport(0,0,w,h);
            if self.fence.is_null(){self.suspend("Unable to create GPU completion fence");}
        }
    }
    fn suspend(&mut self,reason:&str){
        self.suspended=true;self.error=Some(format!("Preview suspended: {reason}. Restart Maker to re-enable rendering."));
        crate::maker::operational_log(format_args!("[MAKER SAFETY] {reason}; live preview suspended"));
    }
    /// Render a candidate without changing the live preview program or framebuffer.
    /// Returns None for invalid/blank/saturated results.
    pub fn candidate_thumbnail(&mut self,graph:&Graph)->Result<Option<Vec<u8>>,String>{
        self.candidate_thumbnail_at(graph,0.0,true,192,108)
    }
    /// Render an evaluated candidate using the GLSL already generated by the
    /// caller. This avoids regenerating the same source on the render path.
    pub fn candidate_thumbnail_with_source(&mut self,graph:&Graph,source:&str)->Result<Option<Vec<u8>>,String>{
        // TEMPORARY DIAGNOSTIC: retain visually uniform frames for inspection.
        // Prefer the first nonuniform frame at t=0,2,5; if none qualifies,
        // display the t=0 image rather than rejecting the candidate.
        // Shader compilation, framebuffer checks, and render errors remain active.
        let mut fallback:Option<Vec<u8>>=None;
        for time in [0.0f32,2.0,5.0] {
            let pixels=self.candidate_thumbnail_at_source(graph,time,false,192,108,source)?
                .ok_or_else(||"Candidate frame unavailable".to_owned())?;
            let first=[pixels[0],pixels[1],pixels[2]];
            let meaningful=pixels.chunks_exact(4).filter(|p|{
                (0..3).any(|i|(p[i] as i16-first[i] as i16).abs()>7)
            }).count();
            let required=((192*108)/1000).max(4);
            crate::maker::operational_log(format_args!("[MAKER CANDIDATE VARIATION] t={time:.1} meaningful={meaningful} required={required} first_rgb={first:?}"));
            if fallback.is_none(){fallback=Some(pixels.clone());}
            if meaningful>=required {
                crate::maker::operational_log(format_args!("[MAKER CANDIDATE ACCEPT] nonuniform frame t={time:.1}"));
                return Ok(Some(pixels));
            }
        }
        crate::maker::operational_log(format_args!("[MAKER CANDIDATE ACCEPT] TEMPORARY uniformity bypass: showing t=0 frame despite uniform t=0,2,5"));
        Ok(fallback)
    }
    /// Offscreen frame sampling used only when Motion Activity sorting is selected.
    pub fn motion_activity(&mut self,graph:&Graph)->Result<f32,String>{
        let mut previous:Option<Vec<u8>>=None;
        let mut total=0.0f32;
        let mut comparisons=0usize;
        for time in [0.0f32,2.0,5.0,10.0] {
            let frame=self.candidate_thumbnail_at(graph,time,false,192,108)?
                .ok_or_else(||"Motion frame unavailable".to_owned())?;
            if let Some(before)=previous.as_ref(){
                // Compare sampled RGB luminance and chroma; discount tiny pixel noise.
                let mut change=0.0f32;
                let mut samples=0usize;
                for (a,b) in before.chunks_exact(4).zip(frame.chunks_exact(4)).step_by(4){
                    let delta=(0..3).map(|i|(a[i] as f32-b[i] as f32).abs()).sum::<f32>()/765.0;
                    change+=((delta-0.025).max(0.0)*3.0).min(1.0);
                    samples+=1;
                }
                total+=change/samples.max(1) as f32;
                comparisons+=1;
            }
            previous=Some(frame);
        }
        Ok((total/comparisons.max(1) as f32).clamp(0.0,1.0))
    }
    fn candidate_thumbnail_at(&mut self,graph:&Graph,time:f32,reject_uniform:bool,w:i32,h:i32)->Result<Option<Vec<u8>>,String>{
        let source=crate::maker_compiler::fragment_source(graph)?;
        self.candidate_thumbnail_at_source(graph,time,reject_uniform,w,h,&source)
    }
    fn candidate_thumbnail_at_source(&mut self,graph:&Graph,time:f32,reject_uniform:bool,w:i32,h:i32,frag:&str)->Result<Option<Vec<u8>>,String>{
        if w>640 || h>360{return Err("Worker frame exceeds temporary 640x360 safety ceiling".into())}
        crate::maker_ui::rendering_admission(graph,w,h)?;
        crate::maker::candidate_complexity_gate(frag)?;
        crate::maker::record_activation("worker","before-compile",graph,frag,w,h,time)?;
        let total_started=Instant::now();
        let (W,H)=(w,h);
        let id=THUMBNAIL_ID.fetch_add(1,Ordering::Relaxed)+1;
        // Source generation is completed by the caller before this GL stage.
        let glsl_ms=0.0;
        crate::maker::operational_log(format_args!("[MAKER GL] id={id} begin compile size={}x{} t={time:.1} glsl_ms={glsl_ms:.1} bytes={}",w,h,frag.len()));
        let compile_started=Instant::now();
        let program=link(VERT,&frag)?;
        crate::maker::operational_log(format_args!("[MAKER GL] id={id} compile finished ms={:.1}",compile_started.elapsed().as_secs_f64()*1000.0));
        if let Err(e)=crate::maker::record_activation("worker","before-draw",graph,frag,w,h,time){unsafe{gl::DeleteProgram(program)};return Err(e)}
        unsafe {
            let mut fbo=0;let mut tex=0;
            gl::GenFramebuffers(1,&mut fbo);gl::GenTextures(1,&mut tex);
            let mut previous_fbo=0;let mut previous_program=0;let mut previous_vao=0;
            let mut previous_viewport=[0i32;4];
            gl::GetIntegerv(gl::FRAMEBUFFER_BINDING,&mut previous_fbo);
            gl::GetIntegerv(gl::CURRENT_PROGRAM,&mut previous_program);
            gl::GetIntegerv(gl::VERTEX_ARRAY_BINDING,&mut previous_vao);
            gl::GetIntegerv(gl::VIEWPORT,previous_viewport.as_mut_ptr());
            let previous_scissor=gl::IsEnabled(gl::SCISSOR_TEST)==gl::TRUE;
            let previous_depth=gl::IsEnabled(gl::DEPTH_TEST)==gl::TRUE;
            let mut previous_texture=0;gl::GetIntegerv(gl::TEXTURE_BINDING_2D,&mut previous_texture);
            gl::BindTexture(gl::TEXTURE_2D,tex);
            gl::TexParameteri(gl::TEXTURE_2D,gl::TEXTURE_MIN_FILTER,gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D,gl::TEXTURE_MAG_FILTER,gl::LINEAR as i32);
            gl::TexImage2D(gl::TEXTURE_2D,0,gl::RGBA8 as i32,W,H,0,gl::RGBA,gl::UNSIGNED_BYTE,std::ptr::null());
            gl::BindFramebuffer(gl::FRAMEBUFFER,fbo);
            gl::FramebufferTexture2D(gl::FRAMEBUFFER,gl::COLOR_ATTACHMENT0,gl::TEXTURE_2D,tex,0);
            let complete=gl::CheckFramebufferStatus(gl::FRAMEBUFFER)==gl::FRAMEBUFFER_COMPLETE;
            let mut pixels=vec![0u8;(W*H*4) as usize];
            if complete {
                gl::Viewport(0,0,W,H);gl::Disable(gl::SCISSOR_TEST);gl::Disable(gl::DEPTH_TEST);
                gl::ClearColor(0.0,0.0,0.0,1.0);gl::Clear(gl::COLOR_BUFFER_BIT);
                gl::UseProgram(program);
                uniform_1f(program,"u_time",time);uniform_2f(program,"u_resolution",W as f32,H as f32);
                 uniform_2f(program,"u_viewport_origin",0.0,0.0);
                for n in &graph.nodes {for p in &n.definition().parameters {
                    if let Some(value)=n.parameters.get(&p.id){let name=format!("u_n{}_{}",n.id,p.id);
                        match p.ty.as_str(){
                            "float"|"choice"=>if let Some(v)=value.as_f64(){uniform_1f(program,&name,v as f32)},
                            "color3"=>if let Some(a)=value.as_array(){if a.len()==3{uniform_3f(program,&name,[a[0].as_f64().unwrap_or(0.0) as f32,a[1].as_f64().unwrap_or(0.0) as f32,a[2].as_f64().unwrap_or(0.0) as f32])}},
                            _=>{},
                        }
                    }
                }}
                gl::BindVertexArray(self.vao);gl::DrawArrays(gl::TRIANGLES,0,3);
                crate::maker::operational_log(format_args!("[MAKER GL] id={id} begin readpixels size={}x{}",W,H));
                let read_started=Instant::now();
                gl::ReadPixels(0,0,W,H,gl::RGBA,gl::UNSIGNED_BYTE,pixels.as_mut_ptr() as *mut _);
                crate::maker::operational_log(format_args!("[MAKER GL] id={id} readpixels finished ms={:.1}",read_started.elapsed().as_secs_f64()*1000.0));
            }
            gl::BindFramebuffer(gl::FRAMEBUFFER,previous_fbo as u32);
            gl::UseProgram(previous_program as u32);gl::BindVertexArray(previous_vao as u32);
            gl::BindTexture(gl::TEXTURE_2D,previous_texture as u32);
            gl::Viewport(previous_viewport[0],previous_viewport[1],previous_viewport[2],previous_viewport[3]);
            if previous_scissor{gl::Enable(gl::SCISSOR_TEST)}else{gl::Disable(gl::SCISSOR_TEST)}
            if previous_depth{gl::Enable(gl::DEPTH_TEST)}else{gl::Disable(gl::DEPTH_TEST)}
            gl::DeleteFramebuffers(1,&fbo);gl::DeleteTextures(1,&tex);gl::DeleteProgram(program);
            if !complete{return Err("Candidate framebuffer incomplete".into())}
            // Reject only essentially uniform output. Preserve dark starfields and
            // predominantly bright compositions with sparse visible structure.
            let first=[pixels[0],pixels[1],pixels[2]];
            let meaningful=pixels.chunks_exact(4).filter(|p|{
                (0..3).any(|i|(p[i] as i16-first[i] as i16).abs()>7)
            }).count();
            let required=((W*H) as usize/1000).max(4);
            if reject_uniform {
                crate::maker::operational_log(format_args!("[MAKER CANDIDATE VARIATION] id={id} t={time:.1} meaningful={meaningful} required={required} first_rgb={first:?}"));
                if meaningful<required{return Ok(None)}
            }
            // OpenGL readback is bottom-up; egui thumbnails are top-down.
            for y in 0..H as usize/2 {
                for x in 0..W as usize*4 {
                    pixels.swap(y*W as usize*4+x,(H as usize-1-y)*W as usize*4+x);
                }
            }
            if total_started.elapsed().as_millis()>=100 {crate::maker::operational_log(format_args!("[MAKER GL] id={id} total_ms={:.1}",total_started.elapsed().as_secs_f64()*1000.0));}
            Ok(Some(pixels))
        }
    }
    /// Diagnostic-only offscreen checks; never changes the live preview program.
    pub fn analyze_candidate(&mut self,graph:&Graph,name:String,live_size:(i32,i32),live_time:f32)->crate::maker_ui::CandidateAnalysis{
        let mut lines=Vec::new();
        match crate::maker_compiler::fragment_source(graph){
            Ok(source)=>{
                let fingerprint=source.bytes().fold(14695981039346656037u64,|h,b|(h^b as u64).wrapping_mul(1099511628211));
                lines.push(format!("GLSL generated: {} bytes; fingerprint {:016x}",source.len(),fingerprint));
            }
            Err(e)=>{
                lines.push(format!("GLSL generation FAILED: {e}"));
                return crate::maker_ui::CandidateAnalysis{name,lines};
            }
        }
        let tests=[(192,108,0.0f32),(192,108,2.0),(192,108,5.0),(192,108,10.0),(192,108,30.0),
            (640,360,0.0),(640,360,10.0),(live_size.0.clamp(1,1920),live_size.1.clamp(1,1080),0.0),
            (live_size.0.clamp(1,1920),live_size.1.clamp(1,1080),live_time)];
        let mut blank=0usize;
        let mut failed=0usize;
        for (w,h,time) in tests {
            match self.candidate_thumbnail_at(graph,time,false,w,h){
                Ok(Some(pixels))=>{
                    let count=pixels.len()/4;
                    let mut sum=0.0f64;let mut sum_sq=0.0f64;
                    let mut min=255u8;let mut max=0u8;
                    for px in pixels.chunks_exact(4){
                        let y=(0.2126*px[0] as f64+0.7152*px[1] as f64+0.0722*px[2] as f64) as u8;
                        min=min.min(y);max=max.max(y);sum+=y as f64;sum_sq+=(y as f64)*(y as f64);
                    }
                    let mean=sum/count.max(1) as f64;
                    let contrast=(sum_sq/count.max(1) as f64-mean*mean).max(0.0).sqrt();
                    let is_blank=max.saturating_sub(min)<8 || (mean<2.0 && contrast<2.0);
                    if is_blank {blank+=1;}
                    lines.push(format!("{}x{} at t={:.1}s: brightness {:.1}/255, contrast {:.1}, range {}..{} — {}",w,h,time,mean,contrast,min,max,if is_blank{"NEAR-UNIFORM"}else{"visible variation"}));
                }
                Ok(None)=>{failed+=1;lines.push(format!("{}x{} at t={time:.1}s: no frame",w,h));}
                Err(e)=>{failed+=1;lines.push(format!("{}x{} at t={time:.1}s: FAILED: {e}",w,h));}
            }
        }
        lines.push(format!("Summary: {blank} near-uniform frames, {failed} render failures across {} samples",tests.len()));
        lines.push("This checks offscreen rendering, not the displayed live-preview pixels or GPU NaN/Inf values.".into());
        lines.push("If all samples are healthy but the live preview is blank, inspect preview compilation, viewport, uniform and graph-application state.".into());
        crate::maker_ui::CandidateAnalysis{name,lines}
    }
    pub fn destroy(&mut self){unsafe{if self.query!=0{gl::DeleteQueries(1,&self.query);}if !self.fence.is_null(){gl::DeleteSync(self.fence);}gl::DeleteFramebuffers(1,&self.fbo);gl::DeleteTextures(1,&self.tex);gl::DeleteVertexArrays(1,&self.vao);if self.program!=0{gl::DeleteProgram(self.program)}}}
}
unsafe fn location(p:u32,n:&str)->i32{unsafe{gl::GetUniformLocation(p,CString::new(n).unwrap().as_ptr())}}
fn uniform_1f(p:u32,n:&str,x:f32){unsafe{gl::Uniform1f(location(p,n),x)}}
fn uniform_2f(p:u32,n:&str,x:f32,y:f32){unsafe{gl::Uniform2f(location(p,n),x,y)}}
fn uniform_3f(p:u32,n:&str,v:[f32;3]){unsafe{gl::Uniform3f(location(p,n),v[0],v[1],v[2])}}
fn link(vertex:&str,fragment:&str)->Result<u32,String>{unsafe{
    let vs=compile(gl::VERTEX_SHADER,vertex)?;
    let fs=match compile(gl::FRAGMENT_SHADER,fragment){Ok(s)=>s,Err(e)=>{gl::DeleteShader(vs);return Err(e)}};
    let p=gl::CreateProgram();gl::AttachShader(p,vs);gl::AttachShader(p,fs);gl::LinkProgram(p);gl::DeleteShader(vs);gl::DeleteShader(fs);
    let mut ok=0;gl::GetProgramiv(p,gl::LINK_STATUS,&mut ok);
    if ok==0{let err=log_program(p);gl::DeleteProgram(p);Err(err)}else{Ok(p)}
}}
fn compile(kind:u32,src:&str)->Result<u32,String>{unsafe{
    let shader=gl::CreateShader(kind);let c=CString::new(src).map_err(|e|e.to_string())?;
    gl::ShaderSource(shader,1,&c.as_ptr(),std::ptr::null());gl::CompileShader(shader);
    let mut ok=0;gl::GetShaderiv(shader,gl::COMPILE_STATUS,&mut ok);
    if ok==0{let err=log_shader(shader);gl::DeleteShader(shader);Err(err)}else{Ok(shader)}
}}
fn log_shader(s:u32)->String{unsafe{let mut len=0;gl::GetShaderiv(s,gl::INFO_LOG_LENGTH,&mut len);let mut buf=vec![0u8;len.max(1) as usize];gl::GetShaderInfoLog(s,len,std::ptr::null_mut(),buf.as_mut_ptr() as *mut _);String::from_utf8_lossy(&buf).into_owned()}}
fn log_program(p:u32)->String{unsafe{let mut len=0;gl::GetProgramiv(p,gl::INFO_LOG_LENGTH,&mut len);let mut buf=vec![0u8;len.max(1) as usize];gl::GetProgramInfoLog(p,len,std::ptr::null_mut(),buf.as_mut_ptr() as *mut _);String::from_utf8_lossy(&buf).into_owned()}}
