//! Graph-to-GLSL compiler. Module code comes from validated JSON definitions.
use crate::maker_graph::{Graph,NodeId};
use std::collections::HashSet;
pub fn fragment_source(graph:&Graph)->Result<String,String>{
 graph.validate()?;
 let out=graph.nodes.iter().find(|n|n.definition().terminal).ok_or("Missing output")?;
 let mut src=String::from("#version 330 core\nout vec4 fragColor;\nuniform vec2 u_resolution;\nuniform float u_time;\n");
 let mut emitted=HashSet::new();let mut helpers=HashSet::new();
 // Helper functions are emitted before main and deduplicated by exact source.
 for n in &graph.nodes{let m=n.definition();if !m.helpers.is_empty()&&helpers.insert(m.helpers.as_str()){src.push_str(&m.helpers);src.push('\n')}}
 for n in &graph.nodes{for p in &n.definition().parameters{
  let ty=if p.ty=="color3"{"vec3"}else{"float"};
  src.push_str(&format!("uniform {ty} u_n{}_{};\n",n.id,p.id));
 }}
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
  let expression=substitute(&m.glsl_outputs[port],&vars)?;
  src.push_str(&format!("  {} v{}_p{}={};\n",output.ty,id,port,expression));
 }
 emitted.insert(id);Ok(())
}
