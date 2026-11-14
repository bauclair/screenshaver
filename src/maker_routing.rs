//! Deterministic, bounded, UI-independent orthogonal routing.
//! All coordinates are egui logical points. No graph connectivity is modified.
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
use std::cmp::Ordering;
use crate::maker_graph::{Graph, NodeId, Connection};

pub type ConnectionKey = (NodeId, usize, NodeId, usize);
pub fn key(c: &Connection) -> ConnectionKey {(c.from,c.from_port,c.to,c.to_port)}
pub type Routes = BTreeMap<ConnectionKey, Vec<[f32;2]>>;

#[derive(Clone,Copy,Debug)]
pub struct Box2 {pub min:[f32;2],pub max:[f32;2]}
impl Box2 {
 pub fn inflated(self, m:f32)->Self {Self{min:[self.min[0]-m,self.min[1]-m],max:[self.max[0]+m,self.max[1]+m]}}
 pub fn overlaps(self, other:Self)->bool {self.min[0]<other.max[0] && self.max[0]>other.min[0] && self.min[1]<other.max[1] && self.max[1]>other.min[1]}
}
#[derive(Clone,Copy,Debug)]
pub struct PortPair {pub from:[f32;2],pub to:[f32;2]}
#[derive(Clone,Debug)]
pub struct Layout {pub positions:BTreeMap<NodeId,[f32;2]>,pub routes:Routes}

fn clear_segment(a:[f32;2],b:[f32;2],boxes:&[(NodeId,Box2)], allowed:Option<NodeId>)->bool {
 let eps=0.01;
 for &(id,r) in boxes {
  if Some(id)==allowed {continue}
  if (a[0]-b[0]).abs()<eps {
   if a[0]>r.min[0] && a[0]<r.max[0] && a[1].max(b[1])>r.min[1] && a[1].min(b[1])<r.max[1] {return false}
  } else if (a[1]-b[1]).abs()<eps {
   if a[1]>r.min[1] && a[1]<r.max[1] && a[0].max(b[0])>r.min[0] && a[0].min(b[0])<r.max[0] {return false}
  } else {return false}
 }
 true
}
#[derive(Clone,Copy,Eq,PartialEq)]
struct Queue {cost:i64,index:usize,dir:u8}
impl Ord for Queue {fn cmp(&self,other:&Self)->Ordering {other.cost.cmp(&self.cost).then_with(||other.index.cmp(&self.index)).then_with(||other.dir.cmp(&self.dir))}}
impl PartialOrd for Queue {fn partial_cmp(&self,other:&Self)->Option<Ordering>{Some(self.cmp(other))}}

/// Orthogonal visibility-grid shortest path with a small bend penalty.
/// Grid coordinates are taken from ports and inflated obstacle edges.
pub fn route(start:[f32;2],end:[f32;2],boxes:&[(NodeId,Box2)],from:NodeId,to:NodeId)->Result<Vec<[f32;2]>,String>{
 // The connector centers are on the module sidewalls. Mandatory horizontal
 // stubs make the connection unambiguous and force perpendicular entry/exit.
 const PREFERRED_STUB:f32=24.0;
 const MIN_STUB:f32=4.0;
 let source=boxes.iter().find(|(id,_)|*id==from).ok_or("Missing source module rectangle")?.1;
 let target=boxes.iter().find(|(id,_)|*id==to).ok_or("Missing destination module rectangle")?.1;
 if (start[0]-source.max[0]).abs()>2.0 || (end[0]-target.min[0]).abs()>2.0 {
  return Err("Connector center is not aligned with its module sidewall".into());
 }
 // Use the longest unobstructed stub up to the preferred length. Nodes in
 // the initial demo are closer together than two fixed 24-point stubs permit.
 // Clearance is a preference; perpendicular connection geometry is mandatory.
 let stub=|port:[f32;2],sign:f32,owner:NodeId| -> Option<[f32;2]> {
  let mut length=PREFERRED_STUB;
  while length>=MIN_STUB {
   let tip=[port[0]+sign*length,port[1]];
   if clear_segment(port,tip,boxes,Some(owner)) {return Some(tip)}
   length-=1.0;
  }
  None
 };
 let exit=stub(start,1.0,from).ok_or("Insufficient horizontal clearance at source connector")?;
 let entry=stub(end,-1.0,to).ok_or("Insufficient horizontal clearance at destination connector")?;
 let core=route_core(exit,entry,boxes,from,to)?;
 let mut result=Vec::with_capacity(core.len()+2);
 result.push(start);result.extend(core);result.push(end);
 // Avoid duplicate points while retaining the required connector stubs.
 result.dedup_by(|a,b|(a[0]-b[0]).abs()<0.05&&(a[1]-b[1]).abs()<0.05);
 Ok(result)
}

fn route_core(start:[f32;2],end:[f32;2],boxes:&[(NodeId,Box2)],from:NodeId,to:NodeId)->Result<Vec<[f32;2]>,String>{
 let mut xs=vec![start[0],end[0]];let mut ys=vec![start[1],end[1]];
 for &(_,b) in boxes {let b=b.inflated(13.0);xs.extend([b.min[0],b.max[0]]);ys.extend([b.min[1],b.max[1]]);}
 xs.sort_by(f32::total_cmp);xs.dedup_by(|a,b|(*a-*b).abs()<0.05);
 ys.sort_by(f32::total_cmp);ys.dedup_by(|a,b|(*a-*b).abs()<0.05);
 let w=xs.len();let h=ys.len();if w*h>25000{return Err("Routing grid exceeds safety limit".into())}
 let at=|p:[f32;2]| -> usize {let x=xs.iter().position(|x|(*x-p[0]).abs()<0.05).unwrap();let y=ys.iter().position(|y|(*y-p[1]).abs()<0.05).unwrap();y*w+x};
 let src=at(start);let dst=at(end);let count=w*h*3;
 let mut best=vec![i64::MAX;count];let mut prev=vec![None;count];let mut heap=BinaryHeap::new();
 best[src*3]=0;heap.push(Queue{cost:0,index:src,dir:0});
 let mut goal=None;
 while let Some(q)=heap.pop(){let state=q.index*3+q.dir as usize;if q.cost!=best[state]{continue}if q.index==dst{goal=Some(state);break}
  let x=q.index%w;let y=q.index/w;
  for (nx,ny,dir) in [(x.wrapping_sub(1),y,1u8),(x+1,y,1),(x,y.wrapping_sub(1),2),(x,y+1,2)] {
   if nx>=w||ny>=h{continue}
   let p=[xs[x],ys[y]];let next=[xs[nx],ys[ny]];
   if !clear_segment(p,next,boxes,None){continue}
   if q.index==src && next[0]<p[0]-0.05 {continue}
   let ni=ny*w+nx;
   if ni==dst && p[0]>next[0]+0.05 {continue}let state2=ni*3+dir as usize;
   let distance=((p[0]-next[0]).abs()+(p[1]-next[1]).abs())*100.0;
   let bend=if q.dir!=0 && q.dir!=dir {1200} else {0};
   let cost=q.cost+distance.round() as i64+bend;
   if cost<best[state2]{best[state2]=cost;prev[state2]=Some(state);heap.push(Queue{cost,index:ni,dir});}
  }
 }
 let mut state=goal.ok_or_else(||format!("No unobstructed route from node {from} to {to}"))?;
 let mut path=Vec::new();loop {let i=state/3;path.push([xs[i%w],ys[i/w]]);if let Some(p)=prev[state]{state=p}else{break}}
 path.reverse();
 // Remove collinear intermediate points, retaining endpoints.
 let mut compact:Vec<[f32;2]>=Vec::new();for p in path {compact.push(p);while compact.len()>=3 {let n=compact.len();let a=compact[n-3];let b=compact[n-2];let c=compact[n-1];if (a[0]-b[0]).abs()<0.05&&(b[0]-c[0]).abs()<0.05 || (a[1]-b[1]).abs()<0.05&&(b[1]-c[1]).abs()<0.05 {compact.remove(n-2);}else{break}}}
 Ok(compact)
}

pub fn route_all(graph:&Graph,ports:&BTreeMap<ConnectionKey,PortPair>,boxes:&[(NodeId,Box2)])->Result<Routes,String>{
 let mut routes=Routes::new();for c in &graph.connections {let k=key(c);let pair=ports.get(&k).ok_or("Missing port geometry")?;routes.insert(k,route(pair.from,pair.to,boxes,c.from,c.to)?);}
 validate(graph,&routes,boxes)?;Ok(routes)
}
pub fn validate(graph:&Graph,routes:&Routes,boxes:&[(NodeId,Box2)])->Result<(),String>{
 graph.validate()?;
 for (i,&(_,a)) in boxes.iter().enumerate(){for &(_,b) in boxes.iter().skip(i+1){if a.overlaps(b){return Err("Overlapping module rectangles".into())}}}
 for c in &graph.connections {let path=routes.get(&key(c)).ok_or("Missing connection route")?;
  if path.len()<4{return Err("Incomplete connector stubs".into())}
  let first=path[0];let second=path[1];let last=path[path.len()-1];let before=path[path.len()-2];
  if (first[1]-second[1]).abs()>0.05 || second[0]<first[0]+3.95
    || (last[1]-before[1]).abs()>0.05 || last[0]<before[0]+3.95 {
   return Err("Route violates perpendicular connector entry/exit".into());
  }
  let src=boxes.iter().find(|(id,_)|*id==c.from).ok_or("Missing source module")?.1;
  let dst=boxes.iter().find(|(id,_)|*id==c.to).ok_or("Missing destination module")?.1;
  if first[0]>src.max[0]+2.0 || last[0]<dst.min[0]-2.0 {
   return Err("Route endpoint is not on its connector sidewall".into());
  }
  for (i,p) in path.windows(2).enumerate(){
   let allow=if i==0 {Some(c.from)} else if i==path.len()-2 {Some(c.to)} else {None};
   if !clear_segment(p[0],p[1],boxes,allow){return Err("Connection intersects module".into())}
  }
 }
 Ok(())
}

/// Compact deterministic layered placement. This is a bounded heuristic, not
/// a mathematical global minimum. No graph structure or parameters change.
pub fn compact_positions(graph:&Graph, node_sizes:&BTreeMap<NodeId,[f32;2]>)->BTreeMap<NodeId,[f32;2]>{
 let mut depth:BTreeMap<NodeId,usize>=graph.nodes.iter().map(|n|(n.id,0)).collect();
 for _ in 0..graph.nodes.len(){let mut changed=false;for c in &graph.connections {let next=depth[&c.from]+1;if next>depth[&c.to]{depth.insert(c.to,next);changed=true}}if !changed{break}}
 let mut layers:BTreeMap<usize,Vec<NodeId>>=BTreeMap::new();for (&id,&d) in &depth{layers.entry(d).or_default().push(id)}
 // Prefer predecessor barycenter while retaining stable ID tie-breaking.
 for (&d,ids) in layers.iter_mut(){if d==0{continue}ids.sort_by(|a,b|{
  let avg=|id:NodeId|{let parents:Vec<_>=graph.connections.iter().filter(|c|c.to==id).map(|c|c.from).collect();if parents.is_empty(){0.0}else{parents.iter().map(|p|*p as f64).sum::<f64>()/parents.len() as f64}};
  avg(*a).total_cmp(&avg(*b)).then(a.cmp(b))
 });}
 let mut result=BTreeMap::new();let mut x=24.0;
 for ids in layers.values(){let max_w=ids.iter().map(|id|node_sizes[id][0]).fold(0.0_f32,f32::max);let mut y=24.0;
  for id in ids {result.insert(*id,[x,y]);y+=node_sizes[id][1]+48.0;}
  x+=max_w+90.0;
 }
 result
}

/// Viewport-aware deterministic placement. A long dependency chain is folded
/// into rows rather than being forced into one horizontal strip. Graph-space
/// positions remain independent of viewport pan. Routing validates the result
/// before the UI commits it as one undoable operation.
pub fn compact_positions_in_view(
    graph: &Graph,
    node_sizes: &BTreeMap<NodeId, [f32; 2]>,
    width: f32,
    height: f32,
) -> BTreeMap<NodeId, [f32; 2]> {
    let baseline = compact_positions(graph, node_sizes);
    if baseline.is_empty() { return baseline; }
    let gap_x = 90.0;
    let gap_y = 65.0;
    let margin = 20.0;
    let max_width = (width - 2.0 * margin).max(240.0);
    let mut ordered: Vec<_> = baseline.iter().map(|(&id, &p)| (id, p)).collect();
    ordered.sort_by(|a, b| a.1[0].total_cmp(&b.1[0]).then_with(|| a.1[1].total_cmp(&b.1[1])).then_with(|| a.0.cmp(&b.0)));
    let mut out = BTreeMap::new();
    let mut x = margin;
    let mut y = margin;
    let mut row_height = 0.0_f32;
    for (id, _) in ordered {
        let size = node_sizes.get(&id).copied().unwrap_or([240.0, 160.0]);
        if x > margin && x + size[0] + margin > max_width + margin {
            x = margin;
            y += row_height + gap_y;
            row_height = 0.0;
        }
        out.insert(id, [x, y]);
        x += size[0] + gap_x;
        row_height = row_height.max(size[1]);
    }
    // The viewport is a preference, not a reason to overlap modules.
    // If the graph is taller than the viewport, mouse panning reveals it.
    let _ = height;
    out
}
