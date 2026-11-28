//! CPU-only canonical, resumable iterative-deepening graph discovery.
//! Each rooted graph has one canonical parent: delete the greatest removable edge.
//! This is deliberately separate from production Maker discovery and GPU work.
use super::*;
use std::collections::{HashSet,BTreeMap,VecDeque};
use std::time::{Duration,Instant};
use std::path::PathBuf;

type Edge=(NodeId,usize,NodeId,usize);
fn setting(name:&str,default:usize,min:usize,max:usize)->usize{
    std::env::var(name).ok().and_then(|v|v.parse().ok()).unwrap_or(default).clamp(min,max)
}
// UI-selected production limit; the command-line test harness retains its env override.
static PRODUCTION_WIRE_LIMIT:std::sync::atomic::AtomicUsize=std::sync::atomic::AtomicUsize::new(5);
fn wire_limit()->usize{PRODUCTION_WIRE_LIMIT.load(std::sync::atomic::Ordering::Relaxed)}
pub(super) fn set_production_wire_limit(limit:usize){
    PRODUCTION_WIRE_LIMIT.store(limit.clamp(3,25),std::sync::atomic::Ordering::Relaxed);
}
pub(super) fn seed()->u64{std::env::var("SCREENSHAVER_MAKER_TEST_SEED").ok().and_then(|s|s.parse().ok()).unwrap_or(0xBADC0FFEE)}
fn mix(mut x:u64)->u64{x^=x>>30;x=x.wrapping_mul(0xbf58476d1ce4e5b9);x^=x>>27;x=x.wrapping_mul(0x94d049bb133111eb);x^(x>>31)}
pub(super) fn edge_rank(e:Edge,seed:u64)->u64{
    let mut x=mix(seed^e.0);
    x=mix(x^(e.1 as u64));x=mix(x^e.2);mix(x^(e.3 as u64))
}
fn graph_edges(g:&Graph)->Vec<Edge>{connection_key(g)}
fn catalog_graph(scale:usize)->Result<Graph,String>{
    let mut graph=Graph::blank();
    let mut ids:Vec<String>=maker_library::get().modules.values().filter(|m|!m.terminal).map(|m|m.id.clone()).collect();
    ids.sort();
    if ids.is_empty(){return Err("Maker module catalog is empty".into())}
    for _ in 0..scale {for id in &ids {let n=graph.nodes.len();graph.add(id,[(n%16) as f32*30.0,(n/16) as f32*30.0])?;}}
    Ok(graph)
}
// A graph is rooted if every connected edge contributes to the Output Color graph.
fn rooted(g:&Graph,output:NodeId)->bool{
    let mut reached=HashSet::from([output]);
    let mut frontier=vec![output];
    while let Some(id)=frontier.pop(){
        for c in g.connections.iter().filter(|c|c.to==id){if reached.insert(c.from){frontier.push(c.from)}}
    }
    g.connections.iter().all(|c|reached.contains(&c.from)&&reached.contains(&c.to))
}
// A canonical parent is obtained by deleting the greatest removable edge.
// A removable edge is one whose deletion preserves rootedness.  The resulting
// parent is independent of the order in which the edges were originally added.
// An edge is removable when ALL REMAINING edges stay Output Color-reachable.
// The deleted edge may orphan a leaf source node; that is allowed. No Graph
// clone is needed, but the rootedness test must match rooted() exactly.
// Preserve the v2 canonical-parent definition and edge tuple ordering.
// The <=64-node fast path represents backward reachability with a u64.
// For each possible deleted edge, propagate reachable nodes from Output Color.
// A deletion is legal iff BOTH endpoints of every REMAINING edge are reachable.
// This intentionally allows the deleted edge to orphan its source node.
// The general HashSet implementation is retained for larger catalogs.
fn is_canonical_child(child:&Graph,added:Edge,output:NodeId)->bool{
    let edges=&child.connections;
    if !edges.iter().any(|c|(c.from,c.from_port,c.to,c.to_port)>added){return true}
    if child.nodes.len()>64 {return is_canonical_child_general(child,added,output)}

    // Dense node positions, not NodeId values, are used as bit positions.
    let ids:Vec<NodeId>=child.nodes.iter().map(|n|n.id).collect();
    let Some(output_index)=ids.iter().position(|id|*id==output) else {return false};
    let mut packed=Vec::with_capacity(edges.len());
    let mut incoming=[0u64;64];
    for c in edges {
        let Some(from)=ids.iter().position(|id|*id==c.from) else {return false};
        let Some(to)=ids.iter().position(|id|*id==c.to) else {return false};
        packed.push((from,to));
        incoming[to]|=1u64<<from;
    }

    // For deletion of edge i, all remaining edge endpoints must be reachable.
    // Prefix/suffix unions calculate that required set in O(1) per deletion.
    let mut prefix=vec![0u64;packed.len()+1];
    for (i,&(from,to)) in packed.iter().enumerate(){
        prefix[i+1]=prefix[i]|(1u64<<from)|(1u64<<to);
    }
    let mut suffix=vec![0u64;packed.len()+1];
    for i in (0..packed.len()).rev(){
        let (from,to)=packed[i];
        suffix[i]=suffix[i+1]|(1u64<<from)|(1u64<<to);
    }
    for (excluded,c) in edges.iter().enumerate(){
        if (c.from,c.from_port,c.to,c.to_port)<=added {continue}
        let (_,excluded_to)=packed[excluded];
        // Recompute only the target's predecessor mask, preserving parallel
        // connections between the same pair of nodes on different ports.
        let mut target_predecessors=0u64;
        for (i,&(from,to)) in packed.iter().enumerate(){
            if i!=excluded && to==excluded_to {target_predecessors|=1u64<<from;}
        }
        let required=prefix[excluded]|suffix[excluded+1];
        let mut reached=1u64<<output_index;
        let mut frontier=reached;
        while frontier!=0 {
            let to=frontier.trailing_zeros() as usize;
            frontier&=frontier-1;
            let predecessors=if to==excluded_to {target_predecessors} else {incoming[to]};
            let newly_reached=predecessors & !reached;
            reached|=newly_reached;
            frontier|=newly_reached;
            if reached&required==required {break}
        }
        if reached&required==required {return false}
        // The excluded source may be orphaned; do not require it reachable.
    }
    true
}
fn is_canonical_child_general(child:&Graph,added:Edge,output:NodeId)->bool{
    let edges=&child.connections;
    let mut reachable=HashSet::<NodeId>::with_capacity(child.nodes.len());
    let mut stack=Vec::<NodeId>::with_capacity(child.nodes.len());
    for excluded in edges.iter().filter(|c|(c.from,c.from_port,c.to,c.to_port)>added){
        reachable.clear();stack.clear();
        reachable.insert(output);stack.push(output);
        while let Some(to)=stack.pop(){
            for c in edges {
                if std::ptr::eq(c,excluded){continue}
                if c.to==to && reachable.insert(c.from){stack.push(c.from)}
            }
        }
        if edges.iter().filter(|c|!std::ptr::eq(*c,excluded))
            .all(|c|reachable.contains(&c.from)&&reachable.contains(&c.to)) {return false}
    }
    true
}

// Deliberately slow reference: reconstruct each possible parent and check
// rootedness using the original graph predicate, not the v3 shortcut.
fn reference_parent(child:&Graph,output:NodeId)->Option<Edge>{
    let mut edges=graph_edges(child);
    edges.sort_unstable_by(|a,b|b.cmp(a));
    for edge in edges {
        let mut parent=child.clone();
        parent.connections.retain(|c|(c.from,c.from_port,c.to,c.to_port)!=edge);
        if rooted(&parent,output){return Some(edge)}
    }
    None
}
fn path_from_frames(frames:&[Frame])->Vec<Edge>{
    frames.windows(2).filter_map(|pair|{
        let parent=pair[0].edges.clone();
        pair[1].edges.clone().into_iter().find(|e|!parent.contains(e))
    }).collect()
}
fn diagnostic_file(label:&str, payload:&serde_json::Value)->Result<PathBuf,String>{
    let path=std::env::temp_dir().join(format!("screenshaver-maker-canonical-{label}.json"));
    std::fs::write(&path,serde_json::to_vec_pretty(payload).map_err(|e|e.to_string())?)
        .map_err(|e|e.to_string())?;
    Ok(path)
}

// Compact rooted state: at most 25 edges. No workspace node copies.
fn reached(edges:&[Edge],root:NodeId,skip:Option<usize>,forward:bool)->Vec<NodeId>{
    let mut seen=vec![root];let mut i=0;
    while i<seen.len(){let n=seen[i];i+=1;
        for (j,e) in edges.iter().enumerate(){if skip==Some(j){continue}
            let (a,b)=if forward {(e.0,e.2)}else{(e.2,e.0)};
            if a==n && !seen.contains(&b){seen.push(b);}
        }
    }seen
}
fn removable(edges:&[Edge],output:NodeId,i:usize)->bool{
    let r=reached(edges,output,Some(i),false);
    edges.iter().enumerate().all(|(j,e)|j==i || (r.contains(&e.0)&&r.contains(&e.2)))
}
fn compact_parent(edges:&[Edge],output:NodeId)->Option<Edge>{
    edges.iter().enumerate().filter(|(i,_)|removable(edges,output,*i)).map(|(_,e)|*e).max()
}
fn candidates(edges:&[Edge],output:NodeId,index:&ConnectionFeasibility,_seed:u64)->Vec<Edge>{
    let reachable=reached(edges,output,None,false);let mut choices=Vec::new();
    for (i,&node) in index.node_ids.iter().enumerate(){
        if !reachable.contains(&node){continue}
        let forbidden=reached(edges,node,None,true);
        for &p in &index.node_inputs[i]{
            if edges.iter().any(|e|e.2==node && e.3==index.inputs[p].port){continue}
            for &id in &index.incoming[p]{if !forbidden.contains(&index.edges[id].key.0){choices.push(id);}}
        }
    }
    choices.sort_unstable_by_key(|&id|(index.edges[id].rank,index.edges[id].key));
    choices.into_iter().map(|id|index.edges[id].key).collect()
}
fn parent_threshold(edges:&[Edge],output:NodeId)->Option<(Edge,Vec<NodeId>)>{
    let edge=compact_parent(edges,output)?;
    Some((edge,reached(edges,output,edges.iter().position(|e|*e==edge),false)))
}
fn materialize(base:&Graph,edges:&[Edge])->Result<Graph,String>{
    let mut g=base.clone();g.connections.clear();
    for &(a,b,c,d) in edges {g.connect(a,b,c,d)?;} Ok(g)
}
struct Frame {edges:Vec<Edge>, choices:Vec<Edge>, next:usize, threshold:Option<(Edge,Vec<NodeId>)>}
struct Explorer {
    base:Graph, output:NodeId, feasibility:ConnectionFeasibility, required:std::collections::HashMap<NodeId,Vec<usize>>, base_valid:bool,
    frames:Vec<Frame>, depth_limit:usize, min_depth:usize, max_depth:usize, seed:u64, examined:u64, valid:u64, rejected_parent:u64,
    reference_checks:u64, reference_mismatches:u64, reference_enabled:bool, prefilter_enabled:bool, prefiltered:u64, traversal_digest:u64,
}
impl Explorer {
    fn new(base:Graph,seed:u64,reference_enabled:bool,prefilter_enabled:bool)->Result<Self,String>{
        Self::new_with_min(base,seed,reference_enabled,prefilter_enabled,1)
    }
    fn new_with_min(base:Graph,seed:u64,reference_enabled:bool,prefilter_enabled:bool,min_depth:usize)->Result<Self,String>{
        let output=base.nodes.iter().find(|n|n.definition().terminal).ok_or("Missing output node")?.id;
        let started=Instant::now();
        let feasibility=ConnectionFeasibility::build(&base,output,seed);
        eprintln!("[MAKER ICE] index_build_ms={:.2}",started.elapsed().as_secs_f64()*1000.0);
        Self::new_indexed(base,seed,reference_enabled,prefilter_enabled,min_depth,feasibility)
    }
    fn new_indexed(base:Graph,seed:u64,reference_enabled:bool,prefilter_enabled:bool,min_depth:usize,feasibility:ConnectionFeasibility)->Result<Self,String>{
        let output=base.nodes.iter().find(|n|n.definition().terminal).ok_or("Missing output node")?.id;
        let root=Vec::new();
        let choices=candidates(&root,output,&feasibility,seed);
        let cap=wire_limit();
        let max_depth=if cap==0 {feasibility.reachable_input_count()} else {feasibility.reachable_input_count().min(cap)};
        if min_depth==0 || min_depth>max_depth {return Err(format!("Invalid minimum wire depth {min_depth} for maximum {max_depth}"));}
        let mut required=std::collections::HashMap::new();for n in &base.nodes {let ports:Vec<usize>=(0..n.input_count()).filter(|&p|base.input_required(n.id,p)).collect();required.insert(n.id,ports);}
        let base_valid={let mut g=base.clone();g.connections.clear();g.validate().is_ok()};
        Ok(Self{base,output,feasibility,required,base_valid,frames:vec![Frame{edges:root,choices,next:0,threshold:None}],depth_limit:min_depth,min_depth,max_depth,seed,examined:0,valid:0,rejected_parent:0,reference_checks:0,reference_mismatches:0,reference_enabled,prefilter_enabled,prefiltered:0,traversal_digest:0x6a09e667f3bcc909})
    }
    fn step(&mut self)->Result<Option<Vec<Edge>>,String>{
        loop {
            if self.frames.is_empty() {
                if self.depth_limit>=self.max_depth {return Ok(None)}
                self.depth_limit+=1;
                let root=Vec::new();
                let choices=candidates(&root,self.output,&self.feasibility,self.seed);
                self.frames.push(Frame{edges:root,choices,next:0,threshold:None});
                continue;
            }
            let Some(frame)=self.frames.last_mut() else {return Ok(None)};
            // Expand only through the current iterative-deepening horizon.
            // Graph size is never used as an admission or gallery ranking criterion.
            if frame.edges.len()>=self.depth_limit {self.frames.pop();continue;}
            if frame.next>=frame.choices.len(){self.frames.pop();continue}
            let edge=frame.choices[frame.next];frame.next+=1;
            let predicted_rejection=self.prefilter_enabled && frame.threshold.as_ref().is_some_and(|(e,reached)|*e>edge && reached.contains(&edge.2));
            if predicted_rejection && !self.reference_enabled {
                self.prefiltered+=1;
                self.rejected_parent+=1;
                continue;
            }
            let mut child=frame.edges.clone();child.push(edge);
            let optimized=!child.iter().enumerate().any(|(i,e)|*e>edge && removable(&child,self.output,i));
            // Run the slow oracle for every attempted successor, including
            // rejected ones. This catches false negatives as well as duplicates.
            if self.reference_enabled {
            let reference=reference_parent(&materialize(&self.base,&child)?,self.output);
            self.reference_checks+=1;
            if optimized!=(reference==Some(edge)) || (predicted_rejection && reference==Some(edge)) {
                self.reference_mismatches+=1;
                let payload=serde_json::json!({
                    "reason":"canonical parent disagreement or invalid parent prefilter",
                    "predicted_rejection":predicted_rejection,
                    "attempt":self.reference_checks,
                    "added_edge":edge,"optimized_accepts":optimized,
                    "reference_parent_edge":reference,
                    "parent_edges":child.clone().into_iter().filter(|e|*e!=edge).collect::<Vec<_>>(),
                    "child_edges":child.clone(),
                    "path_to_parent":path_from_frames(&self.frames),
                    "seed":self.seed
                });
                let path=diagnostic_file("parent-mismatch",&payload)?;
                eprintln!("[MAKER CANONICAL DIAGNOSTIC] parent mismatch: {}",path.display());
                return Err(format!("Canonical parent disagreement; diagnostic: {}",path.display()));
            }
            }
            if predicted_rejection {self.prefiltered+=1;}
            if !optimized {self.rejected_parent+=1;continue}
            if child.len()==self.depth_limit {self.examined+=1;}
            if child.len()==self.depth_limit && child.iter().any(|e|e.2==self.output && e.3==0) && std::iter::once(self.output).chain(child.iter().map(|e|e.0)).all(|id|self.required.get(&id).is_some_and(|ports|ports.iter().all(|&p|child.iter().any(|e|e.2==id && e.3==p)))) && self.base_valid{self.valid+=1}
            let mut key=child.clone();key.sort_unstable();
            if child.len()!=self.depth_limit {
                let choices=candidates(&child,self.output,&self.feasibility,self.seed);
                let threshold=if self.prefilter_enabled {parent_threshold(&child,self.output)} else {None};
                self.frames.push(Frame{edges:child,choices,next:0,threshold});
                continue;
            }
            // Stable, order-sensitive fingerprint of every emitted graph, independent
            // of session and process boundaries. Not a cryptographic proof.
            self.traversal_digest=mix(self.traversal_digest ^ (key.len() as u64));
            for &(from,from_port,to,to_port) in &key {
                self.traversal_digest=mix(self.traversal_digest ^ mix(from));
                self.traversal_digest=mix(self.traversal_digest ^ mix(from_port as u64));
                self.traversal_digest=mix(self.traversal_digest ^ mix(to));
                self.traversal_digest=mix(self.traversal_digest ^ mix(to_port as u64));
            }
            return Ok(Some(key))
        }
    }
    fn checkpoint(&self,scale:usize)->Result<usize,String>{self.checkpoint_at(scale,&checkpoint_path())}
    fn checkpoint_at(&self,scale:usize,path:&std::path::Path)->Result<usize,String>{
        let tmp=path.with_extension("tmp");
        let frames:Vec<_>=self.frames.iter().map(|f| {
            let mut edges = f.edges.clone();
            edges.sort_unstable();
            serde_json::json!({"edges": edges, "next": f.next})
        }).collect();
        let bytes=serde_json::to_vec(&serde_json::json!({"version":17,"wire_limit":wire_limit(),"min_wire_limit":self.min_depth,"depth_limit":self.depth_limit,"scale":scale,"seed":self.seed,
            "catalog":catalog_fingerprint(&self.base),"examined":self.examined,"valid":self.valid,
            "rejected_parent":self.rejected_parent,"traversal_digest":self.traversal_digest,"prefiltered":self.prefiltered,"prefilter_enabled":self.prefilter_enabled,"reference_checks":self.reference_checks,
            "reference_mismatches":self.reference_mismatches,"frames":frames})).map_err(|e|e.to_string())?;
        std::fs::write(&tmp,&bytes).map_err(|e|e.to_string())?;
        std::fs::rename(&tmp,&path).map_err(|e|e.to_string())?;
        Ok(bytes.len())
    }
    fn restore(base:Graph,scale:usize,seed:u64,reference_enabled:bool,prefilter_enabled:bool)->Result<Self,String>{
        Self::restore_at(base,scale,seed,reference_enabled,prefilter_enabled,1,&checkpoint_path(),None)
    }
    fn restore_at(base:Graph,scale:usize,seed:u64,reference_enabled:bool,prefilter_enabled:bool,min_depth:usize,path:&std::path::Path,index:Option<ConnectionFeasibility>)->Result<Self,String>{
        let bytes=std::fs::read(path).map_err(|e|e.to_string())?;
        let v:serde_json::Value=serde_json::from_slice(&bytes).map_err(|e|e.to_string())?;
        if v["version"].as_u64()!=Some(17)||v["wire_limit"].as_u64()!=Some(wire_limit() as u64)||v["min_wire_limit"].as_u64()!=Some(min_depth as u64)||v["scale"].as_u64()!=Some(scale as u64)
            ||v["seed"].as_u64()!=Some(seed)||v["catalog"].as_u64()!=Some(catalog_fingerprint(&base))
            ||v["prefilter_enabled"].as_bool()!=Some(prefilter_enabled){
            return Err("Checkpoint version, catalog, scale or seed mismatch".into())
        }
        let mut result=match index {Some(index)=>Self::new_indexed(base,seed,reference_enabled,prefilter_enabled,min_depth,index)?,None=>Self::new_with_min(base,seed,reference_enabled,prefilter_enabled,min_depth)?};
        result.depth_limit=v["depth_limit"].as_u64().ok_or("Missing depth limit")? as usize;
        if result.depth_limit<result.min_depth || result.depth_limit>result.max_depth {return Err("Checkpoint depth limit out of bounds".into())}
        result.frames.clear();
        for item in v["frames"].as_array().ok_or("Missing frames")? {
            let mut g=result.base.clone();g.connections.clear();
            for edge in item["edges"].as_array().ok_or("Missing edges")?{
                let a=edge.as_array().ok_or("Invalid edge")?;
                if a.len()!=4{return Err("Invalid edge width".into())}
                let e=(a[0].as_u64().ok_or("Invalid from")?,a[1].as_u64().ok_or("Invalid from port")? as usize,
                    a[2].as_u64().ok_or("Invalid to")?,a[3].as_u64().ok_or("Invalid to port")? as usize);
                g.connect(e.0,e.1,e.2,e.3)?;
            }
            let edges=graph_edges(&g);
            let choices=candidates(&edges,result.output,&result.feasibility,seed);
            let next=item["next"].as_u64().ok_or("Missing cursor")? as usize;
            if next>choices.len(){return Err("Checkpoint cursor out of bounds".into())}
            let threshold=if prefilter_enabled {parent_threshold(&edges,result.output)} else {None};
            result.frames.push(Frame{edges,choices,next,threshold});
        }
        result.examined=v["examined"].as_u64().ok_or("Missing examined")?;
        result.valid=v["valid"].as_u64().ok_or("Missing valid")?;
        result.rejected_parent=v["rejected_parent"].as_u64().ok_or("Missing rejected_parent")?;
        result.prefiltered=v["prefiltered"].as_u64().ok_or("Missing prefiltered")?;
        result.traversal_digest=v["traversal_digest"].as_u64().ok_or("Missing traversal_digest")?;
        result.reference_checks=v["reference_checks"].as_u64().ok_or("Missing reference_checks")?;
        result.reference_mismatches=v["reference_mismatches"].as_u64().ok_or("Missing reference_mismatches")?;
        Ok(result)
    }
}
fn catalog_fingerprint(g:&Graph)->u64{
    let mut hash=0x6a09e667f3bcc909u64;
    for n in &g.nodes {hash=mix(hash^n.id);for b in n.module_id.bytes(){hash=mix(hash^(b as u64));}}
    // Include full source definitions of all registered official and community
    // modules, not only the graph's node IDs. A port/GLSL/parameter edit changes
    // the catalog fingerprint even when its module ID is unchanged.
    mix(hash ^ maker_library::get().catalog_fingerprint)
}
fn checkpoint_path()->PathBuf{std::env::temp_dir().join("screenshaver-maker-discovery-canonical-v17.json")}

// Production adapter: the qualified traversal remains the single implementation.
// Its checkpoint is intentionally separate from the /tmp test harness.
pub(super) struct ProductionEnumeration { explorer:Explorer, path:PathBuf }
impl ProductionEnumeration {
    fn path(min_wires:usize,max_wires:usize)->Result<PathBuf,String>{
        let home=std::env::var_os("HOME").ok_or("HOME is not set")?;
        Ok(PathBuf::from(home).join(format!(".config/screenshaver/maker/discovery-canonical-v17-minwires-{min_wires}-maxwires-{max_wires}.json")))
    }
    pub(super) fn reset_checkpoint_for_range(min_wires:usize,max_wires:usize)->Result<(),String>{
        if !(2..=25).contains(&min_wires) || !(3..=25).contains(&max_wires) || min_wires>max_wires {
            return Err(format!("Invalid wire range: {min_wires}..={max_wires}"));
        }
        let path=Self::path(min_wires,max_wires)?;
        match std::fs::remove_file(path){Ok(())=>Ok(()),Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(()),Err(e)=>Err(e.to_string())}
    }
    pub(super) fn open(base:Graph,min_wires:usize,max_wires:usize,index:ConnectionFeasibility)->Result<(Self,bool),String>{
        if !(2..=25).contains(&min_wires) || !(3..=25).contains(&max_wires) || min_wires>max_wires {
            return Err(format!("Invalid wire range: {min_wires}..={max_wires}"));
        }
        if wire_limit()!=max_wires {return Err("Production wire limit differs from selected maximum".into());}
        let path=Self::path(min_wires,max_wires)?;
        let seed=seed();
        // Inspect only the catalog identity before restoring. Other mismatches,
        // malformed JSON, unreadable files, and corrupt traversal frames remain
        // errors rather than being silently interpreted as catalog changes.
        let mut resumed=false;
        let mut catalog_changed=false;
        if path.exists() {
            let bytes=std::fs::read(&path).map_err(|e|format!("Cannot read enumeration checkpoint: {e}"))?;
            let checkpoint:serde_json::Value=serde_json::from_slice(&bytes)
                .map_err(|e|format!("Invalid enumeration checkpoint JSON: {e}"))?;
            let stored=checkpoint["catalog"].as_u64()
                .ok_or("Enumeration checkpoint is missing a valid catalog fingerprint")?;
            if stored!=catalog_fingerprint(&base) {
                // Only the selected wire-limit checkpoint will be overwritten on
                // the next save. Other wire-limit checkpoints remain untouched.
                catalog_changed=true;
                eprintln!("[MAKER DISCOVERY] module catalog changed; starting fresh enumeration for min_wires={} max_wires={}",min_wires,max_wires);
            } else {
                resumed=true;
            }
        }
        let explorer=if catalog_changed || !resumed {Explorer::new_indexed(base,seed,false,true,min_wires,index)?}
            else {Explorer::restore_at(base,1,seed,false,true,min_wires,&path,Some(index))?};
        Ok((Self{explorer,path},resumed && !catalog_changed))
    }
    // The canonical engine emits sorted, validated structural edge tuples.
    pub(super) fn next_edges(&mut self)->Result<Option<Vec<(NodeId,usize,NodeId,usize)>>,String>{
        self.explorer.step()
    }
    pub(super) fn save(&self)->Result<(),String>{
        if let Some(dir)=self.path.parent(){std::fs::create_dir_all(dir).map_err(|e|e.to_string())?;}
        self.explorer.checkpoint_at(1,&self.path).map(|_|())
    }
    pub(super) fn examined(&self)->u64{self.explorer.examined}
    pub(super) fn valid(&self)->u64{self.explorer.valid}
    pub(super) fn rejected(&self)->u64{self.explorer.rejected_parent}
    pub(super) fn digest(&self)->u64{self.explorer.traversal_digest}
    pub(super) fn depth_limit(&self)->usize{self.explorer.depth_limit}
}

// Exhaustive independent queue oracle on a deliberately small inventory.
// The oracle uses no canonical-parent pruning and deduplicates by sorted edges.
// Its successor generator is intentionally separate from candidates().
fn small_catalog_graph(count:usize)->Result<Graph,String>{
    let mut ids:Vec<String>=maker_library::get().modules.values().filter(|m|!m.terminal)
        .map(|m|m.id.clone()).collect();
    ids.sort();
    let offset=setting("SCREENSHAVER_MAKER_TEST_SMALL_OFFSET",0,0,ids.len().saturating_sub(1));
    let mut g=Graph::blank();
    for id in ids.into_iter().skip(offset).take(count) {
        let n=g.nodes.len();g.add(&id,[(n%16) as f32*30.0,(n/16) as f32*30.0])?;
    }
    Ok(g)
}
fn exhaustive_oracle(base:&Graph,limit:usize)->Result<HashSet<Vec<Edge>>,String>{
    let output=base.nodes.iter().find(|n|n.definition().terminal).ok_or("Missing output")?.id;
    let mut root=base.clone();root.connections.clear();
    let mut seen=HashSet::new();
    let mut queue=VecDeque::from([root]);
    let mut queued=HashSet::from([Vec::<Edge>::new()]);
    while let Some(graph)=queue.pop_front(){
        let key=graph_edges(&graph);
        if !seen.insert(key){continue}
        if seen.len()>limit{return Err(format!("Small-catalog oracle exceeded {limit} states; reduce SCREENSHAVER_MAKER_TEST_SMALL_MODULES"))}
        if graph.connections.len()>=wire_limit(){continue}
        let mut reachable=HashSet::from([output]);
        let mut stack=vec![output];
        while let Some(to)=stack.pop(){
            for c in &graph.connections {if c.to==to && reachable.insert(c.from){stack.push(c.from)}}
        }
        for target in &graph.nodes {
            if !reachable.contains(&target.id){continue}
            for port in 0..target.input_count(){
                if graph.input_connection(target.id,port).is_some(){continue}
                for source in &graph.nodes {
                    if source.definition().terminal || source.id==target.id{continue}
                    for source_port in 0..source.output_count(){
                        if graph.check_connection(source.id,source_port,target.id,port).is_err(){continue}
                        let mut next=graph.clone();
                        if next.connect(source.id,source_port,target.id,port).is_ok(){
                            let key=graph_edges(&next);
                            if queued.insert(key){queue.push_back(next)}
                        }
                    }
                }
            }
        }
    }
    Ok(seen)
}
fn exhaustive_crosscheck(seed:u64)->Result<(),String>{
    let modules=setting("SCREENSHAVER_MAKER_TEST_SMALL_MODULES",4,1,12);
    let limit=setting("SCREENSHAVER_MAKER_TEST_SMALL_LIMIT",100_000,10,2_000_000);
    let base=small_catalog_graph(modules)?;
    let oracle=exhaustive_oracle(&base,limit)?;
    let mut explorer=Explorer::new(base,seed,true,true)?;
    let mut actual=HashSet::from([Vec::<Edge>::new()]);
    while let Some(key)=explorer.step()? {
        if !actual.insert(key){return Err("Small-catalog canonical traversal repeated a state".into())}
        if actual.len()>limit{return Err("Small-catalog canonical traversal exceeded state limit".into())}
    }
    let missing=oracle.difference(&actual).next().cloned();
    let extra=actual.difference(&oracle).next().cloned();
    println!("[MAKER CANONICAL EXHAUSTIVE] modules={} offset={} oracle_states={} canonical_states={} missing={} extra={} reference_checks={} mismatches={}",
        modules,setting("SCREENSHAVER_MAKER_TEST_SMALL_OFFSET",0,0,maker_library::get().modules.len()),oracle.len(),actual.len(),oracle.len()-oracle.intersection(&actual).count(),
        actual.len()-oracle.intersection(&actual).count(),explorer.reference_checks,explorer.reference_mismatches);
    if missing.is_some()||extra.is_some(){
        let path=diagnostic_file("exhaustive-mismatch",&serde_json::json!({"modules":modules,"missing_example":missing,"extra_example":extra,"oracle_states":oracle.len(),"canonical_states":actual.len()}))?;
        return Err(format!("Exhaustive completeness disagreement: {}",path.display()))
    }
    Ok(())
}

pub(super) fn run()->Result<(),String>{
    set_production_wire_limit(setting("SCREENSHAVER_MAKER_MAX_WIRES",5,3,25));
    if std::env::var("SCREENSHAVER_MAKER_TEST_EXHAUSTIVE").ok().as_deref()==Some("1") {return exhaustive_crosscheck(seed())}
    let budget=setting("SCREENSHAVER_MAKER_TEST_BUDGET",25_000,1,1_000_000);
    let runs=setting("SCREENSHAVER_MAKER_TEST_RUNS",5,1,100);
    let scale=setting("SCREENSHAVER_MAKER_TEST_SCALE",1,1,20);
    let limit=setting("SCREENSHAVER_MAKER_TEST_LIMIT_SECONDS",120,1,3600);
    let seed=seed();
    let verify=std::env::var("SCREENSHAVER_MAKER_TEST_VERIFY").ok().as_deref()==Some("1");
    let reference_enabled=std::env::var("SCREENSHAVER_MAKER_TEST_REFERENCE").ok().as_deref()==Some("1");
    let prefilter_enabled=std::env::var("SCREENSHAVER_MAKER_TEST_PREFILTER").ok().as_deref()!=Some("0");
    let base=catalog_graph(scale)?;
    let resume=std::env::var("SCREENSHAVER_MAKER_TEST_RESUME").ok().as_deref()==Some("1");
    let mut explorer=if resume {
        Explorer::restore(base.clone(),scale,seed,reference_enabled,prefilter_enabled)?
    } else {
        Explorer::new(base.clone(),seed,reference_enabled,prefilter_enabled)?
    };
    let mut independent=BTreeMap::<Vec<Edge>,Vec<Edge>>::new();
    if resume && verify {
        println!("[MAKER CANONICAL QUALIFICATION] WARNING: duplicate-history verifier covers only this process; traversal_digest covers the full resumed sequence");
    }
    println!("[MAKER CANONICAL LIMIT] max_wires={} (0 means unlimited)",wire_limit());
    println!("[MAKER CANONICAL TEST] catalog_nonterminal={} instances={} scale={} budget={} runs={} seed={} feasible_pairs={} verify={} reference={} prefilter={}",
        maker_library::get().modules.values().filter(|m|!m.terminal).count(),base.nodes.len()-1,scale,budget,runs,seed,explorer.feasibility.feasible_pairs,verify,reference_enabled,prefilter_enabled);
    println!("[MAKER CANONICAL QUALIFICATION] resume={} initial_examined={} initial_digest={:016x} nodes={} reachability_path={}",resume,explorer.examined,explorer.traversal_digest,base.nodes.len(),"compact-local");
    for session in 1..=runs {
        if session>1 {explorer=Explorer::restore(base.clone(),scale,seed,reference_enabled,prefilter_enabled)?;}
        let tick=Instant::now();let mut attempts=0;let mut repeats=0;
        while attempts<budget && tick.elapsed()<Duration::from_secs(limit as u64){
            let Some(key)=explorer.step()? else {break};
            attempts+=1;
            if verify {
                let current_path=path_from_frames(&explorer.frames);
                if let Some(first_path)=independent.get(&key) {
                    repeats+=1;
                    let payload=serde_json::json!({
                        "reason":"duplicate state", "session":session,
                        "examined":explorer.examined,"seed":seed,
                        "sorted_edges":key,"first_path":first_path,
                        "second_path":current_path,
                        "reference_parent":compact_parent(&explorer.frames.last().unwrap().edges,explorer.output),
                        "rejected_parent":explorer.rejected_parent,
                        "reference_checks":explorer.reference_checks
                    });
                    let path=diagnostic_file("first-duplicate",&payload)?;
                    eprintln!("[MAKER CANONICAL DIAGNOSTIC] first duplicate: {}",path.display());
                    return Err(format!("Canonical uniqueness failure at state {}; diagnostic: {}",explorer.examined,path.display()));
                }
                independent.insert(key,current_path);
            }
        }
        let enumeration_s=tick.elapsed().as_secs_f64();
        let save_start=Instant::now();let checkpoint_bytes=explorer.checkpoint(scale)?;
        println!("[MAKER CANONICAL TEST] session={} attempted={} total_examined={} valid_cumulative={} rejected_parent={} prefiltered={} independent_repeats={} reference_checks={} reference_mismatches={} depth={} checkpoint_bytes={} enumeration_s={:.3} checkpoint_save_ms={:.2} rss_kib={} traversal_digest={:016x}",
            session,attempts,explorer.examined,explorer.valid,explorer.rejected_parent,explorer.prefiltered,repeats,explorer.reference_checks,explorer.reference_mismatches,explorer.frames.len(),checkpoint_bytes,enumeration_s,save_start.elapsed().as_secs_f64()*1000.0,discovery_rss_kib(),explorer.traversal_digest);
        if repeats>0{return Err(format!("Canonical uniqueness failure: {repeats} repeated states"))}
        if attempts==0 {println!("[MAKER CANONICAL TEST] exhausted");break}
    }
    println!("[MAKER CANONICAL TEST] completed; production discovery unchanged; verifier memory is excluded only when VERIFY=0");
    Ok(())
}
