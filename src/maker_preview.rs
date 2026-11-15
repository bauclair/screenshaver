//! OpenGL 3.3 live preview with last-good-program retention.
use std::ffi::CString;
use crate::maker_graph::Graph;
const VERT:&str="#version 330 core\nconst vec2 p[3]=vec2[3](vec2(-1,-1),vec2(3,-1),vec2(-1,3));\nvoid main(){gl_Position=vec4(p[gl_VertexID],0,1);}";
pub struct Preview { program:u32, vao:u32, revision:Option<u64>, pub error:Option<String> }
impl Preview {
    pub fn new()->Result<Self,String>{unsafe{let mut vao=0;gl::GenVertexArrays(1,&mut vao);Ok(Self{program:0,vao,revision:None,error:None})}}
    pub fn update(&mut self,graph:&Graph){
        if self.revision==Some(graph.revision){return}
        self.revision=Some(graph.revision);
        let result=crate::maker_compiler::fragment_source(graph).and_then(|frag|link(VERT,&frag));
        match result{Ok(p)=>unsafe{if self.program!=0{gl::DeleteProgram(self.program)}self.program=p;self.error=None},Err(e)=>self.error=Some(e)}
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
            if pw<=0 || ph<=0 || self.program==0{return}
            gl::Enable(gl::SCISSOR_TEST);
            gl::Scissor(x,y,pw,ph);
            gl::Viewport(x,y,pw,ph);
            gl::UseProgram(self.program);
            uniform_1f(self.program,"u_time",t);uniform_2f(self.program,"u_resolution",pw as f32,ph as f32);
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
            gl::BindVertexArray(self.vao);gl::DrawArrays(gl::TRIANGLES,0,3);gl::BindVertexArray(0);gl::UseProgram(0);gl::Disable(gl::SCISSOR_TEST);gl::Viewport(0,0,w,h);
        }
    }
    /// Render a candidate without changing the live preview program or framebuffer.
    /// Returns None for invalid/blank/saturated results.
    pub fn candidate_thumbnail(&mut self,graph:&Graph)->Result<Option<Vec<u8>>,String>{
        const W:i32=192;const H:i32=108;
        let frag=crate::maker_compiler::fragment_source(graph)?;
        let program=link(VERT,&frag)?;
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
                uniform_1f(program,"u_time",0.0);uniform_2f(program,"u_resolution",W as f32,H as f32);
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
                gl::ReadPixels(0,0,W,H,gl::RGBA,gl::UNSIGNED_BYTE,pixels.as_mut_ptr() as *mut _);
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
            if meaningful<((W*H) as usize/1000).max(4){return Ok(None)}
            // OpenGL readback is bottom-up; egui thumbnails are top-down.
            for y in 0..H as usize/2 {
                for x in 0..W as usize*4 {
                    pixels.swap(y*W as usize*4+x,(H as usize-1-y)*W as usize*4+x);
                }
            }
            Ok(Some(pixels))
        }
    }
    pub fn destroy(&mut self){unsafe{gl::DeleteVertexArrays(1,&self.vao);if self.program!=0{gl::DeleteProgram(self.program)}}}
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
