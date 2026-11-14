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
  // Helper functions are emitted once per distinct source.
  for n in &graph.nodes{let m=n.definition();if !m.helpers.is_empty()&&helpers.insert(m.helpers.as_str()){src.push_str(&m.helpers);src.push('\n')}}
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
 let n=graph.node(id).ok_or("Callable field source missing")?;
 if port!=0{return Err("Callable field requires the first scalar output".into())}
 if graph.input_connection(id,0).is_some(){return Err("Callable field position must be controlled by traversal; disconnect the field's Position input".into())}
 match n.module_id.as_str(){
  "screenshaver.fields.morphing"=>Ok(format!("mix(maker_morph_field({position},u_n{}_scale,u_n{}_strength,u_n{}_speed),maker_original_distance({position},u_n{}_scale,u_n{}_strength,u_n{}_speed),step(0.5,u_n{}_mode))",id,id,id,id,id,id,id)),
  "screenshaver.noise.dot3d"=>Ok(format!("maker_dot_noise_3d({position})")),
  _=>Err(format!("Module '{}' does not support callable field evaluation yet",n.module_id)),
 }
}
