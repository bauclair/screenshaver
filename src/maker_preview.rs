//! Isolated OpenGL 3.3 shader preview, rendered behind Maker's egui panels.
use std::ffi::CString;

const VERT: &str = "#version 330 core\nconst vec2 p[3] = vec2[3](vec2(-1,-1),vec2(3,-1),vec2(-1,3));\nout vec2 uv; void main(){vec2 q=p[gl_VertexID]; uv=q*.5+.5; gl_Position=vec4(q,0,1);}";
const FRAG: &str = r#"#version 330 core
in vec2 uv; out vec4 fragColor;
uniform float u_time; uniform float u_frequency; uniform vec3 u_color; uniform vec2 u_resolution;
void main() {
    vec2 p = (gl_FragCoord.xy - .5*u_resolution) / max(u_resolution.y,1.);
    float v = sin(p.x*16.*u_frequency + u_time) * cos(p.y*12.*u_frequency-u_time*.7);
    float glow = .5 + .5*v;
    vec3 col = mix(vec3(.015,.02,.04), u_color, pow(glow, 1.5));
    fragColor = vec4(col,1.);
}"#;

pub struct Preview { program: u32, vao: u32 }
impl Preview {
    pub fn new() -> Result<Self,String> {
        unsafe {
            let vs=compile(gl::VERTEX_SHADER,VERT)?;
            let fs=match compile(gl::FRAGMENT_SHADER,FRAG) { Ok(s)=>s, Err(e)=>{gl::DeleteShader(vs);return Err(e)} };
            let program=gl::CreateProgram(); gl::AttachShader(program,vs);gl::AttachShader(program,fs);gl::LinkProgram(program);
            gl::DeleteShader(vs);gl::DeleteShader(fs);
            let mut ok=0;gl::GetProgramiv(program,gl::LINK_STATUS,&mut ok);
            if ok==0 { let error=program_log(program);gl::DeleteProgram(program);return Err(error); }
            let mut vao=0;gl::GenVertexArrays(1,&mut vao);
            Ok(Self{program,vao})
        }
    }
    pub fn render(&mut self,w:i32,h:i32,t:f32,freq:f32,color:[f32;3])->Result<(),String>{
        unsafe {
            gl::Viewport(0,0,w,h);gl::Disable(gl::SCISSOR_TEST);gl::Disable(gl::DEPTH_TEST);
            gl::ClearColor(.015,.02,.04,1.);gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::UseProgram(self.program);
            let loc=|name:&str|gl::GetUniformLocation(self.program,CString::new(name).unwrap().as_ptr());
            gl::Uniform1f(loc("u_time"),t);gl::Uniform1f(loc("u_frequency"),freq);
            gl::Uniform3f(loc("u_color"),color[0],color[1],color[2]);gl::Uniform2f(loc("u_resolution"),w as f32,h as f32);
            gl::BindVertexArray(self.vao);gl::DrawArrays(gl::TRIANGLES,0,3);gl::BindVertexArray(0);gl::UseProgram(0);
        }
        Ok(())
    }
    pub fn destroy(&mut self){unsafe{gl::DeleteVertexArrays(1,&self.vao);gl::DeleteProgram(self.program)}}
}
unsafe fn compile(kind:u32,source:&str)->Result<u32,String>{
    unsafe {
        let shader=gl::CreateShader(kind);let c=CString::new(source).map_err(|e|e.to_string())?;
        gl::ShaderSource(shader,1,&c.as_ptr(),std::ptr::null());gl::CompileShader(shader);
        let mut ok=0;gl::GetShaderiv(shader,gl::COMPILE_STATUS,&mut ok);
        if ok==0 {let mut len=0;gl::GetShaderiv(shader,gl::INFO_LOG_LENGTH,&mut len);let mut buf=vec![0u8;len.max(1) as usize];gl::GetShaderInfoLog(shader,len,std::ptr::null_mut(),buf.as_mut_ptr() as *mut _);gl::DeleteShader(shader);return Err(String::from_utf8_lossy(&buf).into_owned());}
        Ok(shader)
    }
}
unsafe fn program_log(program:u32)->String{unsafe{let mut len=0;gl::GetProgramiv(program,gl::INFO_LOG_LENGTH,&mut len);let mut buf=vec![0u8;len.max(1) as usize];gl::GetProgramInfoLog(program,len,std::ptr::null_mut(),buf.as_mut_ptr() as *mut _);String::from_utf8_lossy(&buf).into_owned()}}
