//! Maker's UI-independent, typed, directed acyclic graph.
use std::collections::{HashMap, HashSet};

pub type NodeId = u64;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortType { Float, Uv2, Color3 }
impl PortType { pub fn name(self) -> &'static str { match self { Self::Float=>"Float", Self::Uv2=>"UV2", Self::Color3=>"Color3" } } }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind { Uv, Noise, Multiply, Palette, Output }
impl Kind {
    pub const LIBRARY: [Kind; 5] = [Kind::Uv, Kind::Noise, Kind::Multiply, Kind::Palette, Kind::Output];
    pub fn title(self) -> &'static str { match self { Self::Uv=>"UV Coordinates",Self::Noise=>"Value Noise 2D",Self::Multiply=>"Multiply",Self::Palette=>"Cosine Palette",Self::Output=>"Output Color" } }
    pub fn inputs(self) -> &'static [(&'static str,PortType)] { match self {
        Self::Uv=>&[],Self::Noise=>&[("UV",PortType::Uv2)],Self::Multiply=>&[("Value",PortType::Float)],Self::Palette=>&[("Value",PortType::Float)],Self::Output=>&[("Color",PortType::Color3)]
    } }
    pub fn outputs(self) -> &'static [(&'static str,PortType)] { match self {
        Self::Uv=>&[("UV",PortType::Uv2)],Self::Noise|Self::Multiply=>&[("Value",PortType::Float)],Self::Palette=>&[("Color",PortType::Color3)],Self::Output=>&[]
    } }
}
#[derive(Clone, Debug)]
pub struct Node { pub id: NodeId, pub kind: Kind, pub pos: [f32;2], pub expanded: bool, pub frequency: f32, pub color: [f32;3], pub scale: f32, pub factor: f32 }
impl Node { fn new(id: NodeId, kind: Kind, pos:[f32;2]) -> Self { Self {id,kind,pos,expanded:true,frequency:1.0,color:[0.25,0.65,1.0],scale:4.0,factor:1.0} } }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Connection { pub from: NodeId, pub from_port: usize, pub to: NodeId, pub to_port: usize }
#[derive(Clone, Debug)]
pub struct Graph { pub nodes: Vec<Node>, pub connections: Vec<Connection>, next_id: NodeId, pub revision: u64 }
impl Graph {
    pub fn demo() -> Self {
        let mut g=Self { nodes:vec![],connections:vec![],next_id:1,revision:0 };
        let uv=g.add(Kind::Uv,[20.0,70.0]); let noise=g.add(Kind::Noise,[250.0,65.0]);
        let palette=g.add(Kind::Palette,[485.0,85.0]); let output=g.add(Kind::Output,[725.0,110.0]);
        g.connect(uv,0,noise,0).unwrap(); g.connect(noise,0,palette,0).unwrap(); g.connect(palette,0,output,0).unwrap(); g
    }
    pub fn add(&mut self, kind:Kind, pos:[f32;2])->NodeId {
        let id=self.next_id;self.next_id+=1;self.nodes.push(Node::new(id,kind,pos));self.revision+=1;id
    }
    pub fn node(&self,id:NodeId)->Option<&Node>{self.nodes.iter().find(|n|n.id==id)}
    pub fn node_mut(&mut self,id:NodeId)->Option<&mut Node>{self.nodes.iter_mut().find(|n|n.id==id)}
    pub fn remove(&mut self,id:NodeId)->Result<(),String>{
        let n=self.node(id).ok_or("Node not found")?;
        if n.kind==Kind::Output {return Err("Output Color is required".into())}
        self.nodes.retain(|n|n.id!=id);self.connections.retain(|c|c.from!=id&&c.to!=id);self.revision+=1;Ok(())
    }
    pub fn input_connection(&self,id:NodeId,port:usize)->Option<Connection>{self.connections.iter().copied().find(|c|c.to==id&&c.to_port==port)}
    pub fn disconnect_input(&mut self,id:NodeId,port:usize){let n=self.connections.len();self.connections.retain(|c|!(c.to==id&&c.to_port==port));if n!=self.connections.len(){self.revision+=1}}
    pub fn connect(&mut self,from:NodeId,from_port:usize,to:NodeId,to_port:usize)->Result<(),String>{
        if from==to{return Err("Self-connections are not allowed".into())}
        let a=self.node(from).ok_or("Source node not found")?;let b=self.node(to).ok_or("Destination node not found")?;
        let src=a.kind.outputs().get(from_port).ok_or("Source port not found")?.1;
        let dst=b.kind.inputs().get(to_port).ok_or("Destination port not found")?.1;
        if src!=dst {return Err(format!("Port mismatch: {} cannot connect to {}",src.name(),dst.name()))}
        // A new edge from -> to is cyclic iff to already reaches from.
        let mut visited=HashSet::new();if self.reaches(to,from,&mut visited){return Err("Connection would create a cycle".into())}
        self.disconnect_input(to,to_port);
        self.connections.push(Connection{from,from_port,to,to_port});self.revision+=1;Ok(())
    }
    fn reaches(&self,at:NodeId,target:NodeId,visited:&mut HashSet<NodeId>)->bool{
        if at==target{return true}if !visited.insert(at){return false}
        self.connections.iter().filter(|c|c.from==at).any(|c|self.reaches(c.to,target,visited))
    }
    pub fn validate(&self)->Result<(),String>{
        let ids:HashMap<_,_>=self.nodes.iter().map(|n|(n.id,n.kind)).collect();
        if self.nodes.iter().filter(|n|n.kind==Kind::Output).count()!=1{return Err("Exactly one Output Color node is required".into())}
        let mut destinations=HashSet::new();
        for c in &self.connections {
            let a=ids.get(&c.from).ok_or("Dangling source")?;let b=ids.get(&c.to).ok_or("Dangling target")?;
            if a.outputs().get(c.from_port).map(|p|p.1)!=b.inputs().get(c.to_port).map(|p|p.1){return Err("Invalid connection types".into())}
            if !destinations.insert((c.to,c.to_port)){return Err("Multiple sources for an input".into())}
        }
        for n in &self.nodes {let mut visited=HashSet::new();if self.connections.iter().filter(|c|c.from==n.id).any(|c|self.reaches(c.to,n.id,&mut visited)){return Err("Cycle detected".into())}}
        Ok(())
    }
}
#[cfg(test)]mod tests{use super::*;
 #[test]fn valid_demo(){let g=Graph::demo();assert!(g.validate().is_ok());}
 #[test]fn rejects_type_mismatch(){let mut g=Graph::demo();let uv=g.nodes[0].id;let out=g.nodes[3].id;assert!(g.connect(uv,0,out,0).is_err());}
 #[test]fn rejects_cycle(){let mut g=Graph::demo();let a=g.add(Kind::Multiply,[0.0,0.0]);let b=g.add(Kind::Multiply,[0.0,0.0]);g.connect(a,0,b,0).unwrap();assert!(g.connect(b,0,a,0).is_err());}
}
