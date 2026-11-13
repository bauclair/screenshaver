//! Deterministic graph-to-GLSL 330 core compilation. Only reachable nodes are emitted.
use crate::maker_graph::{Graph,Kind,NodeId};
use std::collections::HashSet;

pub fn fragment_source(graph:&Graph)->Result<String,String>{
    graph.validate()?;
    let out=graph.nodes.iter().find(|n|n.kind==Kind::Output).ok_or("Missing output")?;
    let mut source=String::from(r#"#version 330 core
out vec4 fragColor;
uniform vec2 u_resolution;
uniform float u_time;
float maker_hash(vec2 p) { return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453123); }
float maker_noise(vec2 p) {
    vec2 i=floor(p),f=fract(p);f=f*f*(3.0-2.0*f);
    return mix(mix(maker_hash(i),maker_hash(i+vec2(1,0)),f.x),
               mix(maker_hash(i+vec2(0,1)),maker_hash(i+vec2(1,1)),f.x),f.y);
}
"#);
    // Uniforms are declared for every node, but the expression tree only evaluates reachable nodes.
    for n in &graph.nodes {match n.kind {
        Kind::Noise=>source.push_str(&format!("uniform float u_n{}_scale;\n",n.id)),
        Kind::Multiply=>source.push_str(&format!("uniform float u_n{}_factor;\n",n.id)),
        Kind::Palette=>source.push_str(&format!("uniform float u_n{}_frequency;\nuniform vec3 u_n{}_color;\n",n.id,n.id)),
        _=>{}
    }}
    source.push_str("void main(){\n  vec2 maker_uv=(gl_FragCoord.xy-0.5*u_resolution)/max(u_resolution.y,1.0);\n");
    let mut emitted=HashSet::new();
    if let Some(c)=graph.input_connection(out.id,0){
        emit(graph,c.from,&mut emitted,&mut source)?;
        source.push_str(&format!("  fragColor=vec4(clamp(v{},0.0,1.0),1.0);\n",c.from));
    }else{source.push_str("  fragColor=vec4(0.015,0.02,0.04,1.0);\n");}
    source.push_str("}\n");Ok(source)
}
fn emit(graph:&Graph,id:NodeId,emitted:&mut HashSet<NodeId>,src:&mut String)->Result<(),String>{
    if emitted.contains(&id){return Ok(())}
    let node=graph.node(id).ok_or("Missing upstream node")?;
    for port in 0..node.kind.inputs().len(){if let Some(c)=graph.input_connection(id,port){emit(graph,c.from,emitted,src)?;}}
    let input=|port:usize,default:&str|->String{graph.input_connection(id,port).map(|c|format!("v{}",c.from)).unwrap_or_else(||default.into())};
    match node.kind{
        Kind::Uv=>src.push_str(&format!("  vec2 v{}=maker_uv;\n",id)),
        Kind::Noise=>src.push_str(&format!("  float v{}=maker_noise({}*u_n{}_scale+vec2(u_time*0.17,-u_time*0.11));\n",id,input(0,"maker_uv"),id)),
        Kind::Multiply=>src.push_str(&format!("  float v{}={}*u_n{}_factor;\n",id,input(0,"0.5"),id)),
        Kind::Palette=>src.push_str(&format!("  vec3 v{}=0.5+0.5*cos(6.2831853*({}*u_n{}_frequency+vec3(0.0,0.33,0.67))+u_time*0.3);\n  v{}*=u_n{}_color;\n",id,input(0,"0.5"),id,id,id)),
        Kind::Output=>return Err("Output cannot be an upstream source".into()),
    }
    emitted.insert(id);Ok(())
}
#[cfg(test)]mod tests{use super::*;#[test]fn emits_graph(){let g=Graph::demo();let s=fragment_source(&g).unwrap();assert!(s.contains("maker_noise"));assert!(s.contains("fragColor=vec4(clamp"));}}
