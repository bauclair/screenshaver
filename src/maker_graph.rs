//! UI-independent graph of JSON-defined modules.
use std::collections::{HashMap,HashSet};
use serde_json::Value;
use crate::maker_library;
pub type NodeId=u64;
#[derive(Clone,Debug)]
pub struct Node {pub id:NodeId,pub module_id:String,pub pos:[f32;2],pub expanded:bool,pub parameters:HashMap<String,Value>}
impl Node {
 fn new(id:NodeId,module_id:&str,pos:[f32;2])->Result<Self,String>{
  let m=maker_library::get().module(module_id).ok_or_else(||format!("Unknown module: {module_id}"))?;
  Ok(Self{id,module_id:module_id.into(),pos,expanded:true,parameters:m.parameters.iter().map(|p|(p.id.clone(),p.default.clone())).collect()})
 }
 pub fn definition(&self)->&'static crate::maker_module::Module{maker_library::get().module(&self.module_id).expect("Registered node module")}
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Connection {pub from:NodeId,pub from_port:usize,pub to:NodeId,pub to_port:usize}
#[derive(Clone,Debug)]
pub struct Graph {pub nodes:Vec<Node>,pub connections:Vec<Connection>,next_id:NodeId,pub revision:u64}
impl Graph {
 pub fn demo()->Self{
  let mut g=Self{nodes:vec![],connections:vec![],next_id:1,revision:0};
  let uv=g.add("screenshaver.coordinates.uv",[20.,70.]).unwrap();
  let noise=g.add("screenshaver.noise.value2d",[250.,65.]).unwrap();
  let palette=g.add("screenshaver.color.cosine_palette",[485.,85.]).unwrap();
  let output=g.add("screenshaver.output.color",[725.,110.]).unwrap();
  g.connect(uv,0,noise,0).unwrap();g.connect(noise,0,palette,0).unwrap();g.connect(palette,0,output,0).unwrap();g
 }
 pub fn add(&mut self,module_id:&str,pos:[f32;2])->Result<NodeId,String>{
  let def=maker_library::get().module(module_id).ok_or("Unknown module")?;
  if def.terminal && self.nodes.iter().any(|n|n.definition().terminal){return Err("Only one output terminal is allowed".into())}
  let id=self.next_id;let node=Node::new(id,module_id,pos)?;self.next_id+=1;self.nodes.push(node);self.revision+=1;Ok(id)
 }
 pub fn node(&self,id:NodeId)->Option<&Node>{self.nodes.iter().find(|n|n.id==id)}
 pub fn node_mut(&mut self,id:NodeId)->Option<&mut Node>{self.nodes.iter_mut().find(|n|n.id==id)}
 pub fn remove(&mut self,id:NodeId)->Result<(),String>{
  let n=self.node(id).ok_or("Node not found")?;
  if n.definition().terminal{return Err("Output Color is required".into())}
  self.nodes.retain(|n|n.id!=id);self.connections.retain(|c|c.from!=id&&c.to!=id);self.revision+=1;Ok(())
 }
 pub fn input_connection(&self,id:NodeId,port:usize)->Option<Connection>{self.connections.iter().copied().find(|c|c.to==id&&c.to_port==port)}
 pub fn disconnect_input(&mut self,id:NodeId,port:usize){let n=self.connections.len();self.connections.retain(|c|!(c.to==id&&c.to_port==port));if n!=self.connections.len(){self.revision+=1}}
 pub fn check_connection(&self,from:NodeId,from_port:usize,to:NodeId,to_port:usize)->Result<(),String>{
  if from==to{return Err("A node cannot connect to itself".into())}
  let a=self.node(from).ok_or("Source node missing")?.definition();let b=self.node(to).ok_or("Target node missing")?.definition();
  let ap=a.outputs.get(from_port).ok_or("Source port missing")?;let bp=b.inputs.get(to_port).ok_or("Target port missing")?;
  if ap.ty!=bp.ty{return Err(format!("{} output '{}' ({}) cannot connect to {} input '{}' ({}): incompatible types",a.name,ap.name,ap.ty,b.name,bp.name,bp.ty))}
  if self.reaches(to,from,&mut HashSet::new()){return Err("Connection would create a circular dependency".into())}
  Ok(())
 }
 pub fn connect(&mut self,from:NodeId,from_port:usize,to:NodeId,to_port:usize)->Result<(),String>{
  self.check_connection(from,from_port,to,to_port)?;
  let c=Connection{from,from_port,to,to_port};if self.input_connection(to,to_port)==Some(c){return Ok(())}
  self.disconnect_input(to,to_port);self.connections.push(c);self.revision+=1;Ok(())
 }
 fn reaches(&self,at:NodeId,target:NodeId,visited:&mut HashSet<NodeId>)->bool{
  if at==target{return true}if !visited.insert(at){return false}
  self.connections.iter().filter(|c|c.from==at).any(|c|self.reaches(c.to,target,visited))
 }
 pub fn validate(&self)->Result<(),String>{
  if self.nodes.iter().filter(|n|n.definition().terminal).count()!=1{return Err("Exactly one output terminal required".into())}
  let mut dest=HashSet::new();
  for c in &self.connections{
   let a=self.node(c.from).ok_or("Dangling source")?.definition();let b=self.node(c.to).ok_or("Dangling target")?.definition();
   let at=a.outputs.get(c.from_port).ok_or("Bad output port")?;let bt=b.inputs.get(c.to_port).ok_or("Bad input port")?;
   if at.ty!=bt.ty{return Err("Invalid connection types".into())}
   if !dest.insert((c.to,c.to_port)){return Err("Multiple sources for an input".into())}
  }
  for n in &self.nodes{if self.connections.iter().filter(|c|c.from==n.id).any(|c|self.reaches(c.to,n.id,&mut HashSet::new())){return Err("Cycle detected".into())}}
  Ok(())
 }
}
