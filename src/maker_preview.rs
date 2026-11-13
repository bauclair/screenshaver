//! OpenGL 3.3 live preview with last-good-program retention.
use std::ffi::CString;
use crate::maker_graph::{Graph,Kind};
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
    pub fn render(&mut self,w:i32,h:i32,t:f32,graph:&Graph){
        unsafe{
            gl::Viewport(0,0,w,h);gl::Disable(gl::SCISSOR_TEST);gl::Disable(gl::DEPTH_TEST);
            gl::ClearColor(0.015,0.02,0.04,1.0);gl::Clear(gl::COLOR_BUFFER_BIT);
            if self.program==0{return}
            gl::UseProgram(self.program);
            uniform_1f(self.program,"u_time",t);uniform_2f(self.program,"u_resolution",w as f32,h as f32);
            for n in &graph.nodes{match n.kind{
                Kind::Noise=>uniform_1f(self.program,&format!("u_n{}_scale",n.id),n.scale),
                Kind::Multiply=>uniform_1f(self.program,&format!("u_n{}_factor",n.id),n.factor),
                Kind::Palette=>{uniform_1f(self.program,&format!("u_n{}_frequency",n.id),n.frequency);uniform_3f(self.program,&format!("u_n{}_color",n.id),n.color)},
                _=>{}
            }}
            gl::BindVertexArray(self.vao);gl::DrawArrays(gl::TRIANGLES,0,3);gl::BindVertexArray(0);gl::UseProgram(0);
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
