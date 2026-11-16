//! Graph-to-GLSL compiler. Module code comes from validated JSON definitions.
use crate::maker_graph::{Graph,NodeId};
use std::collections::HashSet;
pub fn fragment_source(graph:&Graph)->Result<String,String>{
 graph.validate()?;
 let out=graph.nodes.iter().find(|n|n.definition().terminal).ok_or("Missing output")?;
 let mut src=String::from("#version 330 core\nout vec4 fragColor;\nuniform vec2 u_resolution;\nuniform float u_time;\n");
 let mut emitted=HashSet::new();let mut helpers=HashSet::new();
  // Uniforms precede helpers because callable traversal functions read node parameters.
  for n in &graph.nodes{for p in &n.definition().parameters{
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
  for n in &graph.nodes{let m=n.definition();if !m.helpers.is_empty()&&helpers.insert(m.helpers.as_str()){src.push_str(&m.helpers);src.push('\n')}}
  // Node-specific domain-fold functions: GLSL requires a compile-time bounded loop.
  for n in &graph.nodes {
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
  // A connected scalar field becomes a position-dependent callable evaluation.
  // Unlike a precomputed scalar seed, this expression is evaluated inside each ray step.
  for n in &graph.nodes{
   if n.module_id=="screenshaver.fields.traversal" {
    if let Some(c)=graph.input_connection(n.id,0){
     let field=callable_field(graph,c.from,c.from_port,"sample_position")?;
     src.push_str(&format!("vec3 maker_traverse_connected_{}(vec3 origin,float depth,float density,float speed,float mode){{vec3 ray=normalize(vec3(origin.xy,1.25));vec3 sum=vec3(0.0);float t=u_time*speed;for(int k=0;k<10;k++){{float z=(float(k)+0.5)*depth/10.0;vec3 sample_position=vec3(origin.xy,-1.0)+ray*z;float field=({field});float weight=density*exp(-abs(field)*4.0)/(1.0+0.25*z*z);vec3 phase=vec3(0.0,2.1,4.2);sum+=weight*(0.5+0.5*sin(t*0.4+z*2.0+phase));}}vec3 p=vec3(0.0,0.0,-1.0-0.5*sin(u_time*(speed/0.5)*0.1));vec3 d=normalize(vec3(origin.xy,1.0));vec3 light=vec3(0.0);for(int k=0;k<10;k++){{vec3 sample_position=p;float step_distance=max(abs(({field})),0.02);p+=d*step_distance*(depth/2.5);light+=density*(1.0+1.5*sin(float(k)+length(p.xy*0.1)+2.0+vec3(3.0,1.5,0.5)))/step_distance;}}return mix(sum/10.0,light,step(0.5,mode));}}\n",n.id));
    }
   }
  }
 // Generate an independent signed-distance ray marcher for each SDF node.
 for n in &graph.nodes {
  if n.module_id=="screenshaver.fields.sdf_raymarch" {
   let c=graph.input_connection(n.id,0).ok_or("SDF Ray March requires a connected distance field")?;
   let field=callable_expression(graph,c.from,c.from_port,"sample_position","float")?;
   let camera=if let Some(c)=graph.input_connection(n.id,1){callable_expression(graph,c.from,c.from_port,"vec3(0.0,0.0,camera_z)","vec2")?}else{"vec2(0.0)".into()};
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
 for n in &graph.nodes {
  if n.module_id=="screenshaver.lighting.refractive_surface" {
   let field_conn=graph.input_connection(n.id,0).ok_or("Refractive Surface Sampling requires a connected Distance Field")?;
   let bg_conn=graph.input_connection(n.id,1).ok_or("Refractive Surface Sampling requires a connected Background")?;
   let field=callable_expression(graph,field_conn.from,field_conn.from_port,"sample_position","float")?;
   let background=callable_expression(graph,bg_conn.from,bg_conn.from_port,"sample_direction","vec3")?;
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
  let expression=if m.id=="screenshaver.fields.traversal" && graph.input_connection(id,0).is_some(){format!("maker_traverse_connected_{}({},{},{},{},{})",id,vars["position"],vars["depth"],vars["density"],vars["speed"],vars["mode"])}else{substitute(&m.glsl_outputs[port],&vars)?};
  src.push_str(&format!("  {} v{}_p{}={};\n",output.ty,id,port,expression));
 }
 emitted.insert(id);Ok(())
}

// Build a field expression that can be evaluated at an arbitrary 3D sample position.
// The first implementation deliberately supports only position-driven scalar field modules.
// Unsupported graphs produce a clear error instead of silently using a frozen seed.
fn callable_field(graph:&Graph,id:NodeId,port:usize,position:&str)->Result<String,String>{
 callable_expression(graph,id,port,position,"float")
}
fn callable_expression(graph:&Graph,id:NodeId,port:usize,position:&str,expected:&str)->Result<String,String>{
 fn expand(graph:&Graph,id:NodeId,port:usize,position:&str,active:&mut HashSet<NodeId>)->Result<String,String>{
  if !active.insert(id){return Err("Callable field cycle".into())}
  let result=(||{
   let n=graph.node(id).ok_or("Callable source missing")?;
   let m=n.definition();
   if m.terminal{return Err("Output terminal cannot be sampled".into())}
   let mut vars=std::collections::HashMap::new();
   // Domain operators re-evaluate their upstream field at transformed coordinates.
   // This is essential: transforming an already-computed distance has no geometric effect.
   let domain_position=if m.id=="screenshaver.geometry.domain_repeat_3d" {
    format!("(mod(({position})+0.5*vec3(u_n{id}_span_x,u_n{id}_span_y,u_n{id}_span_z),vec3(u_n{id}_span_x,u_n{id}_span_y,u_n{id}_span_z))-0.5*vec3(u_n{id}_span_x,u_n{id}_span_y,u_n{id}_span_z))")
   } else if m.id=="screenshaver.geometry.domain_fold_3d" {
    format!("maker_blue_fold_{id}({position})")
   } else if m.id=="screenshaver.geometry.rotation_3d" {
    format!("maker_rotate_3d({position},u_n{id}_xy+u_time*u_n{id}_speed,u_n{id}_yz+u_time*u_n{id}_speed,u_n{id}_xz+u_time*u_n{id}_speed)")
   } else {position.to_owned()};
   for (index,input) in m.inputs.iter().enumerate(){
    let value=if (m.id=="screenshaver.geometry.domain_repeat_3d" || m.id=="screenshaver.geometry.domain_fold_3d" || m.id=="screenshaver.geometry.rotation_3d") && input.id=="field" {
     let c=graph.input_connection(id,index).ok_or_else(||format!("{} requires a connected field",m.name))?;
     expand(graph,c.from,c.from_port,&domain_position,active)?
    }else if input.id=="position" && input.ty=="vec3" && graph.input_connection(id,index).is_none(){position.to_owned()}
    else if m.id=="screenshaver.color.angular_wave_background" && input.id=="direction" && graph.input_connection(id,index).is_none(){position.to_owned()}
    else if m.id=="screenshaver.camera.sinusoidal_path" && input.id=="z" && graph.input_connection(id,index).is_none(){format!("({position}).z")}
    else if let Some(c)=graph.input_connection(id,index){
     let from=graph.node(c.from).ok_or("Callable input source missing")?;
     if from.definition().outputs[c.from_port].ty!=input.ty{return Err("Callable input type mismatch".into())}
     expand(graph,c.from,c.from_port,position,active)?
    }else{input.default.clone()};
    if value.is_empty(){return Err(format!("Missing callable input {}:{}",m.id,input.id))}
    vars.insert(input.id.clone(),value);
   }
   for parameter in &m.parameters{vars.insert(parameter.id.clone(),format!("u_n{}_{}",id,parameter.id));}
   if m.id=="screenshaver.fields.morphing" {return Ok(format!("mix(maker_morph_field({position},u_n{id}_scale,u_n{id}_strength,u_n{id}_speed),maker_original_distance({position},u_n{id}_scale,u_n{id}_strength,u_n{id}_speed),step(0.5,u_n{id}_mode))"))}
   substitute(&m.glsl_outputs[port],&vars)
  })();
  active.remove(&id);result
 }
 let n=graph.node(id).ok_or("Callable source missing")?;
 let ty=&n.definition().outputs.get(port).ok_or("Callable output missing")?.ty;
 if ty!=expected{return Err(format!("Callable source requires {expected}, got {ty}"))}
 expand(graph,id,port,position,&mut HashSet::new())
}
