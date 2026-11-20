//! Graph-to-GLSL compiler. Module code comes from validated JSON definitions.
use crate::maker_graph::{Graph,NodeId};
use std::collections::{HashMap,HashSet};
/// Restrict untrusted module metadata to a single safe GLSL comment line.
fn attribution_text(value:&str)->String{
 let normalized=value.replace("*/", "* /");
 normalized.chars().map(|ch|if ch.is_control(){' '}else{ch}).collect()
}
pub fn fragment_source(graph:&Graph)->Result<String,String>{
 graph.validate_renderable()?;
 let reachable=graph.output_reachable_ids();
 let out=graph.nodes.iter().find(|n|n.definition().terminal).ok_or("Missing output")?;
 // Keep #version first; include each contributing module instance and the terminal.
 let mut src=String::from("#version 330 core\n/*\n * Created using Screenshaver Maker\n * Maker modules used to create this shader:\n");
 for node in graph.nodes.iter().filter(|node|reachable.contains(&node.id)) {
  let module=node.definition();
  let name=attribution_text(&module.name);
  let creator=attribution_text(&module.created_by);
  src.push_str(&format!(" * {name} — Created by: {creator}\n"));
 }
 src.push_str(" */\nout vec4 fragColor;\nuniform vec2 u_resolution;\nuniform float u_time;\n");
 let mut emitted=HashSet::new();let mut helpers=HashSet::new();
  // Uniforms precede helpers because callable traversal functions read node parameters.
  for n in graph.nodes.iter().filter(|n|reachable.contains(&n.id)){for p in &n.definition().parameters{
   let ty=if p.ty=="color3"{"vec3"}else{"float"};
   src.push_str(&format!("uniform {ty} u_n{}_{};\n",n.id,p.id));
  }}
  src.push_str(r#"
float maker_twisted_ring(vec3 p,float radius,float w,float h,float twists,float fold){
 vec2 q=vec2(length(p.xz)-radius,p.y);float a=atan(p.x,p.z)*twists;
 mat2 r=mat2(cos(a),sin(a),-sin(a),cos(a));q=r*q;q.y=abs(q.y)-fold;
 vec2 d=abs(q)-vec2(w,h);return length(max(d,0.0))+min(max(d.x,d.y),0.0);
}
float maker_sequential_morph(float a,float b,float c,float d,float time,float hold){
 float t=mod(time,4.0);float f=clamp((fract(t)-hold)/max(1.0-hold,0.001),0.0,1.0);f=f*f*(3.0-2.0*f);
 return t<1.0?mix(a,b,f):t<2.0?mix(b,c,f):t<3.0?mix(c,d,f):mix(d,a,f);
}
vec3 maker_rotate_3d(vec3 p,float xy,float yz,float xz){
 mat2 a=mat2(cos(xy),sin(xy),-sin(xy),cos(xy));
 mat2 b=mat2(cos(yz),sin(yz),-sin(yz),cos(yz));
 mat2 c=mat2(cos(xz),sin(xz),-sin(xz),cos(xz));
 p.xy=a*p.xy;p.yz=b*p.yz;p.xz=c*p.xz;return p;
}
"#);
  // Helper functions are emitted once per distinct source.
  for n in graph.nodes.iter().filter(|n|reachable.contains(&n.id)){let m=n.definition();if !m.helpers.is_empty()&&helpers.insert(m.helpers.as_str()){src.push_str(&m.helpers);src.push('\n')}}
  // Node-specific domain-fold functions: GLSL requires a compile-time bounded loop.
  for n in graph.nodes.iter().filter(|n|reachable.contains(&n.id)) {
   if n.module_id=="screenshaver.geometry.domain_fold_3d" {
    let id=n.id;
    src.push_str(&format!(r#"vec3 maker_blue_fold_{id}(vec3 p){{
     float ax=u_n{id}_angle_xz;float ay=u_n{id}_angle_yz;
     mat2 rx=mat2(cos(ax),sin(ax),-sin(ax),cos(ax));
     mat2 ry=mat2(cos(ay),sin(ay),-sin(ay),cos(ay));
     for(int i=0;i<8;i++){{if(float(i)>=u_n{id}_iterations)break;
      p=abs(p)-vec3(u_n{id}_offset);p.xz=rx*p.xz;p.yz=ry*p.yz;
     }}return p;
    }}
"#));
   }
  }
  // Generate each callable node/port once, using a vec3 sampling position.
  // Function calls preserve distinct sample positions without recursive source duplication.
  let mut callable=CallableGenerator::new(graph);
  for n in graph.nodes.iter().filter(|n|reachable.contains(&n.id)) {
   match n.module_id.as_str() {
    "screenshaver.fields.traversal" => {
     if let Some(c)=graph.input_connection(n.id,0){callable.require(c.from,c.from_port,"float")?;}
    }
    "screenshaver.fields.sdf_raymarch" => {
     let c=graph.input_connection(n.id,0).ok_or("SDF Ray March requires a connected distance field")?;
     callable.require(c.from,c.from_port,"float")?;
     if let Some(c)=graph.input_connection(n.id,1){callable.require(c.from,c.from_port,"vec2")?;}
    }
    "screenshaver.lighting.refractive_surface" => {
     let c=graph.input_connection(n.id,0).ok_or("Refractive Surface Sampling requires a connected Distance Field")?;
     callable.require(c.from,c.from_port,"float")?;
     let c=graph.input_connection(n.id,1).ok_or("Refractive Surface Sampling requires a connected Background")?;
     callable.require(c.from,c.from_port,"vec3")?;
    }
    _=>{}
   }
  }
  src.push_str(&callable.source());
  // A connected scalar field becomes a position-dependent callable evaluation.
  // Unlike a precomputed scalar seed, this expression is evaluated inside each ray step.
  for n in graph.nodes.iter().filter(|n|reachable.contains(&n.id)){
   if n.module_id=="screenshaver.fields.traversal" {
    if let Some(c)=graph.input_connection(n.id,0){
     let field=callable.reference(c.from,c.from_port,"sample_position","float")?;
     // Both modes integrate bounded color/opacity along the connected field.
     // Density controls optical depth rather than unbounded additive brightness.
     // No reciprocal-distance amplification: a near-zero field must not white out.
     src.push_str(&format!(r#"vec3 maker_traverse_connected_{id}(vec3 origin,float depth,float density,float speed,float mode){{
      vec3 ray=normalize(vec3(origin.xy,1.25));
      float extent=max(depth,0.001);
      float step_size=extent/16.0;
      float optical_density=max(density,0.0);
      float t=u_time*speed;
      vec3 accumulated=vec3(0.0);
      float transmittance=1.0;
      for(int k=0;k<16;k++){{
       float z=(float(k)+0.5)*step_size;
       vec3 sample_position=vec3(origin.xy,-1.0)+ray*z;
       float field=({field});
       // Mode 0 emphasizes the field's zero crossing; mode 1 gives a
       // broader response to the same connected field, without singularities.
       float narrow=exp(-min(abs(field)*4.0,80.0));
       float broad=exp(-min(abs(field)*1.5,80.0));
       float occupancy=mix(narrow,broad,step(0.5,mode));
       float attenuation=1.0/(1.0+0.25*z*z);
       float opacity=1.0-exp(-min(optical_density*occupancy*attenuation*step_size,80.0));
       vec3 phase=vec3(0.0,2.1,4.2);
       vec3 tint=0.5+0.5*cos(phase+t*0.4+z*2.0+field*0.5);
       accumulated+=transmittance*opacity*tint;
       transmittance*=1.0-opacity;
      }}
      return accumulated;
     }}
"#,id=n.id));
    }
   }
  }
 // Generate an independent signed-distance ray marcher for each SDF node.
 for n in graph.nodes.iter().filter(|n|reachable.contains(&n.id)) {
  if n.module_id=="screenshaver.fields.sdf_raymarch" {
   let c=graph.input_connection(n.id,0).ok_or("SDF Ray March requires a connected distance field")?;
   let field=callable.reference(c.from,c.from_port,"sample_position","float")?;
   let camera=if let Some(c)=graph.input_connection(n.id,1){callable.reference(c.from,c.from_port,"vec3(0.0,0.0,camera_z)","vec2")?}else{"vec2(0.0)".into()};
   let id=n.id;
   src.push_str(&format!(r#"
float maker_sdf_{id}(vec3 sample_position){{return {field};}}
vec3 maker_sdf_normal_{id}(vec3 p,float e){{
 vec2 h=vec2(e,0.0);
 return normalize(vec3(maker_sdf_{id}(p+h.xyy)-maker_sdf_{id}(p-h.xyy),maker_sdf_{id}(p+h.yxy)-maker_sdf_{id}(p-h.yxy),maker_sdf_{id}(p+h.yyx)-maker_sdf_{id}(p-h.yyx)));
}}
vec3 maker_sdf_render_{id}(vec2 uv,float speed,float maxDistance,float epsilon,float stepScale,float fov,float glowFalloff,float glowGain,out vec3 surfaceNormal,out float surfaceAO,out float surfaceCurvature,out float rayGlow){{
 surfaceNormal=vec3(0.0,0.0,1.0); surfaceAO=1.0; surfaceCurvature=0.0; rayGlow=0.0;
 // Bounded, periodic camera travel: preserve the original view at t=0,
 // but never advance beyond origin-centered SDF geometry while looking +Z.
 // Negative Z keeps the camera on the approach side of the scene.
 float camera_z=-1.5*abs(sin(u_time*speed));
 vec2 camera_xy={camera};
 vec3 ro=vec3(camera_xy,camera_z);
 vec3 rd=normalize(vec3(uv*fov,1.0));
 float t=0.03;
 for(int i=0;i<128;i++){{
  vec3 p=ro+rd*t;
  float d=maker_sdf_{id}(p);
  rayGlow+=exp(-abs(d)*max(glowFalloff,0.001))*glowGain;
  if(abs(d)<epsilon){{
   float e=max(epsilon*3.0,0.003);
   vec3 nor=maker_sdf_normal_{id}(p,e);
   surfaceNormal=nor;
   float occ=0.0;
   for(int j=1;j<=5;j++){{
    float h=0.045*float(j);
    occ+=max(h-maker_sdf_{id}(p+nor*h),0.0)/(h*float(j));
   }}
   surfaceAO=clamp(1.0-occ*0.55,0.0,1.0);
   float curvatureStep=max(e*3.0,0.02);
   vec3 dx=vec3(curvatureStep,0.0,0.0);
   vec3 dy=vec3(0.0,curvatureStep,0.0);
   vec3 dz=vec3(0.0,0.0,curvatureStep);
   float laplace=(maker_sdf_{id}(p+dx)+maker_sdf_{id}(p-dx)+maker_sdf_{id}(p+dy)+maker_sdf_{id}(p-dy)+maker_sdf_{id}(p+dz)+maker_sdf_{id}(p-dz)-6.0*maker_sdf_{id}(p))/(curvatureStep*curvatureStep);
   surfaceCurvature=clamp(laplace*0.025,-1.0,1.0);
   vec3 lightDir=normalize(vec3(0.55,0.85,-0.65));
   float diffuse=max(dot(nor,lightDir),0.0);
   float ambient=0.18+0.20*max(nor.y,0.0);
   float fog=exp(-t*0.024);
   return mix(vec3(0.012,0.018,0.035),vec3(0.48,0.65,0.88)*(ambient+0.85*diffuse),fog);
  }}
  if(t>maxDistance)break;
  t+=max(abs(d)*stepScale,epsilon*0.7);
 }}
 return vec3(0.012,0.018,0.035);
}}
"#));
  }
 }
 // Optical sampling: evaluate the connected background at the actual ray direction.
 for n in graph.nodes.iter().filter(|n|reachable.contains(&n.id)) {
  if n.module_id=="screenshaver.lighting.refractive_surface" {
   let field_conn=graph.input_connection(n.id,0).ok_or("Refractive Surface Sampling requires a connected Distance Field")?;
   let bg_conn=graph.input_connection(n.id,1).ok_or("Refractive Surface Sampling requires a connected Background")?;
   let field=callable.reference(field_conn.from,field_conn.from_port,"sample_position","float")?;
   let background=callable.reference(bg_conn.from,bg_conn.from_port,"sample_direction","vec3")?;
   let id=n.id;
   src.push_str(&format!(r#"
float maker_optical_sdf_{id}(vec3 sample_position){{return {field};}}
vec3 maker_optical_background_{id}(vec3 sample_direction){{return {background};}}
vec3 maker_optical_render_{id}(vec2 uv){{
 vec3 ro=vec3(0.0,0.0,u_n{id}_camera_z);
 vec3 rd=normalize(vec3(uv,1.0));
 float t=0.0;
 for(int k=0;k<96;k++){{
  if(t>u_n{id}_max_distance)break;
  vec3 p=ro+rd*t;
  float d=maker_optical_sdf_{id}(p);
  if(abs(d)<u_n{id}_epsilon){{
   float e=max(u_n{id}_epsilon*2.0,0.002);
   vec3 dx=vec3(e,0.0,0.0),dy=vec3(0.0,e,0.0),dz=vec3(0.0,0.0,e);
   vec3 nor=normalize(vec3(
    maker_optical_sdf_{id}(p+dx)-maker_optical_sdf_{id}(p-dx),
    maker_optical_sdf_{id}(p+dy)-maker_optical_sdf_{id}(p-dy),
    maker_optical_sdf_{id}(p+dz)-maker_optical_sdf_{id}(p-dz)));
   vec3 refracted=refract(rd,nor,u_n{id}_refraction_ratio);
   if(dot(refracted,refracted)<0.00001)refracted=reflect(rd,nor);
   float fade=smoothstep(0.0,max(u_n{id}_fade_distance,0.001),t);
   return maker_optical_background_{id}(normalize(refracted))*fade;
  }}
  t+=max(abs(d)*u_n{id}_step_scale,u_n{id}_epsilon*0.7);
 }}
 return maker_optical_background_{id}(rd);
}}
"#));
  }
 }
 src.push_str("void main(){\n  vec2 maker_uv=(gl_FragCoord.xy-0.5*u_resolution)/max(u_resolution.y,1.0);\n");
 if graph.input_connection(out.id,0).is_some(){
  let c=graph.input_connection(out.id,0).unwrap();emit(graph,c.from,&mut emitted,&mut src)?;
  src.push_str(&format!("  fragColor=vec4(clamp(v{}_p{},0.0,1.0),1.0);\n",c.from,c.from_port));
 }else{src.push_str("  fragColor=vec4(0.015,0.02,0.04,1.0);\n")}
 src.push_str("}\n");Ok(src)
}
fn substitute(template:&str,bindings:&std::collections::HashMap<String,String>)->Result<String,String>{
 let mut result=String::new();let mut chars=template.chars().peekable();
 while let Some(ch)=chars.next(){
  if ch=='{'{
   let mut key=String::new();let mut closed=false;
   for c in chars.by_ref(){if c=='}'{closed=true;break}key.push(c)}
   if !closed{return Err("Unclosed GLSL placeholder".into())}
   result.push_str(bindings.get(&key).ok_or_else(||format!("Unknown GLSL placeholder: {key}"))?);
  }else if ch=='}'{return Err("Unexpected GLSL closing brace".into())}
  else{result.push(ch)}
 }
 Ok(result)
}
// Select the first connected diagnostic input in a fixed priority order.
// No calibration parameters or runtime switching are required.
fn universal_preview_expression(graph:&Graph,id:NodeId)->String {
 for port in [3usize,2,1,0,4] {
  if graph.input_connection(id,port).is_some() {
   let c=graph.input_connection(id,port).expect("connected diagnostic input");
   let value=format!("v{}_p{}",c.from,c.from_port);
   return match port {
    3 => value,
    2 => format!("clamp(0.5+0.5*({value}),0.0,1.0)"),
    1 => format!("clamp(vec3(0.5+0.5*({value}),0.5),0.0,1.0)"),
    0 => format!("vec3(clamp({value},0.0,1.0))"),
    _ => format!("maker_universal_signed({value})"),
   };
  }
 }
 "maker_universal_color(maker_uv)".into()
}
fn emit(graph:&Graph,id:NodeId,emitted:&mut HashSet<NodeId>,src:&mut String)->Result<(),String>{
 if emitted.contains(&id){return Ok(())}
 let node=graph.node(id).ok_or("Missing upstream node")?;let m=node.definition();
 if m.terminal{return Err("Output cannot be an upstream source".into())}
 for port in 0..m.inputs.len(){if let Some(c)=graph.input_connection(id,port){emit(graph,c.from,emitted,src)?}}
 let mut vars=std::collections::HashMap::new();
 for (port,p) in m.inputs.iter().enumerate(){
  let expression=graph.input_connection(id,port).map(|c|format!("v{}_p{}",c.from,c.from_port)).unwrap_or_else(||p.default.clone());
  if expression.is_empty(){return Err(format!("Missing input default for {}:{}",m.id,p.id))}
  vars.insert(p.id.clone(),expression);
 }
 for p in &m.parameters{vars.insert(p.id.clone(),format!("u_n{}_{}",id,p.id));}
 if m.id=="screenshaver.color.angular_wave_background" && graph.input_connection(id,0).is_none(){vars.insert("direction".into(),"normalize(vec3(maker_uv,1.0))".into());}
 if m.id=="screenshaver.lighting.refractive_surface" {
  src.push_str(&format!("  vec3 v{}_p0=maker_optical_render_{}(maker_uv);\n",id,id));
  emitted.insert(id);return Ok(());
 }
 if m.id=="screenshaver.fields.sdf_raymarch" {
  src.push_str(&format!("  vec3 v{}_p0; vec3 v{}_p1; float v{}_p2; float v{}_p3; float v{}_p4;\n",id,id,id,id,id));
  src.push_str(&format!("  v{}_p0=maker_sdf_render_{}(maker_uv,u_n{}_speed,u_n{}_max_distance,u_n{}_epsilon,u_n{}_step_scale,u_n{}_fov,u_n{}_glow_falloff,u_n{}_glow_gain,v{}_p1,v{}_p2,v{}_p3,v{}_p4);\n",id,id,id,id,id,id,id,id,id,id,id,id,id));
  emitted.insert(id);return Ok(());
 }
 for (port,output) in m.outputs.iter().enumerate(){
  let expression=if m.id=="screenshaver.test.universal_test" && output.id=="preview_color" { universal_preview_expression(graph,id) } else if m.id=="screenshaver.fields.traversal" && graph.input_connection(id,0).is_some(){format!("maker_traverse_connected_{}({},{},{},{},{})",id,vars["position"],vars["depth"],vars["density"],vars["speed"],vars["mode"])}else{substitute(&m.glsl_outputs[port],&vars)?};
  src.push_str(&format!("  {} v{}_p{}={};\n",output.ty,id,port,expression));
 }
 emitted.insert(id);Ok(())
}

// A callable is one GLSL function per node output.  All functions take the sampling
// position explicitly, allowing domain transformations to evaluate at different
// coordinates without copying the entire upstream expression tree.
struct CallableGenerator<'a> {
 graph:&'a Graph,
 definitions:Vec<String>,
 signatures:Vec<String>,
 generated:HashSet<(NodeId,usize)>,
 active:HashSet<(NodeId,usize)>,
}
impl<'a> CallableGenerator<'a> {
 fn new(graph:&'a Graph)->Self { Self {graph,definitions:Vec::new(),signatures:Vec::new(),generated:HashSet::new(),active:HashSet::new()} }
 fn name(id:NodeId,port:usize)->String {format!("maker_callable_{id}_{port}")}
 fn reference(&self,id:NodeId,port:usize,position:&str,expected:&str)->Result<String,String>{
  let n=self.graph.node(id).ok_or("Callable source missing")?;
  let ty=&n.definition().outputs.get(port).ok_or("Callable output missing")?.ty;
  if ty!=expected{return Err(format!("Callable source requires {expected}, got {ty}"))}
  if !self.generated.contains(&(id,port)){return Err(format!("Callable source not generated: {id}:{port}"))}
  Ok(format!("{}({position})",Self::name(id,port)))
 }
 fn require(&mut self,id:NodeId,port:usize,expected:&str)->Result<(),String>{
  let n=self.graph.node(id).ok_or("Callable source missing")?;
  let m=n.definition();
  let ty=m.outputs.get(port).ok_or("Callable output missing")?.ty.clone();
  if ty!=expected{return Err(format!("Callable source requires {expected}, got {ty}"))}
  if self.generated.contains(&(id,port)){return Ok(())}
  if !self.active.insert((id,port)){return Err("Callable field cycle".into())}
  let result=(||->Result<String,String>{
   if m.terminal{return Err("Output terminal cannot be sampled".into())}
   let mut vars=HashMap::new();
   let position="maker_position";
   let domain_position=if m.id=="screenshaver.geometry.domain_repeat_3d" {
    format!("(mod(({position})+0.5*vec3(u_n{id}_span_x,u_n{id}_span_y,u_n{id}_span_z),vec3(u_n{id}_span_x,u_n{id}_span_y,u_n{id}_span_z))-0.5*vec3(u_n{id}_span_x,u_n{id}_span_y,u_n{id}_span_z))")
   }else if m.id=="screenshaver.geometry.domain_fold_3d"{
    format!("maker_blue_fold_{id}({position})")
   }else if m.id=="screenshaver.geometry.rotation_3d"{
    format!("maker_rotate_3d({position},u_n{id}_xy+u_time*u_n{id}_speed,u_n{id}_yz+u_time*u_n{id}_speed,u_n{id}_xz+u_time*u_n{id}_speed)")
   }else{position.to_owned()};
   for (index,input) in m.inputs.iter().enumerate(){
    let is_domain=(m.id=="screenshaver.geometry.domain_repeat_3d" || m.id=="screenshaver.geometry.domain_fold_3d" || m.id=="screenshaver.geometry.rotation_3d") && input.id=="field";
    let value=if is_domain {
     let c=self.graph.input_connection(id,index).ok_or_else(||format!("{} requires a connected field",m.name))?;
     self.require(c.from,c.from_port,&input.ty)?;
     format!("{}({domain_position})",Self::name(c.from,c.from_port))
    }else if input.id=="position" && input.ty=="vec3" && self.graph.input_connection(id,index).is_none(){position.to_owned()}
    else if m.id=="screenshaver.color.angular_wave_background" && input.id=="direction" && self.graph.input_connection(id,index).is_none(){position.to_owned()}
    else if m.id=="screenshaver.camera.sinusoidal_path" && input.id=="z" && self.graph.input_connection(id,index).is_none(){format!("({position}).z")}
    else if let Some(c)=self.graph.input_connection(id,index){
     self.require(c.from,c.from_port,&input.ty)?;
     format!("{}({position})",Self::name(c.from,c.from_port))
    }else{input.default.clone()};
    if value.is_empty(){return Err(format!("Missing callable input {}:{}",m.id,input.id))}
    vars.insert(input.id.clone(),value);
   }
   for parameter in &m.parameters{vars.insert(parameter.id.clone(),format!("u_n{}_{}",id,parameter.id));}
   substitute(&m.glsl_outputs[port],&vars)
  })();
  self.active.remove(&(id,port));
  let expression=result?;
  let name=Self::name(id,port);
  self.signatures.push(format!("{ty} {name}(vec3 maker_position);\n"));
  self.definitions.push(format!("{ty} {name}(vec3 maker_position){{vec2 maker_uv=maker_position.xy;return {expression};}}\n"));
  self.generated.insert((id,port));
  Ok(())
 }
 fn source(&self)->String{
  let mut result=String::new();
  for signature in &self.signatures{result.push_str(signature)}
  for definition in &self.definitions{result.push_str(definition)}
  result
 }
}
