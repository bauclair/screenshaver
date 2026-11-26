// TEMPORARY development diagnostics. Remove this file and its callers before release.
use std::io::Write;
use std::path::PathBuf;
fn directory()->std::io::Result<PathBuf>{
    let dir=PathBuf::from(std::env::var_os("HOME").ok_or_else(||std::io::Error::other("HOME missing"))?).join(".config/screenshaver");
    std::fs::create_dir_all(&dir)?;Ok(dir)
}
pub(super) fn event(stage:&str,durable:bool)->std::io::Result<()> {
    use std::os::fd::AsRawFd;
    unsafe extern "C" {fn flock(fd:i32,operation:i32)->i32;}
    let dir=directory()?;
    let lock=std::fs::OpenOptions::new().create(true).write(true).open(dir.join(".maker-trace.lock"))?;
    if unsafe{flock(lock.as_raw_fd(),2|4)}!=0{return Err(std::io::Error::last_os_error())}
    let path=dir.join("maker_render_trace.log");
    if std::fs::metadata(&path).map(|m|m.len()).unwrap_or(0)>8*1024*1024 {std::fs::rename(&path,dir.join("maker_render_trace.log.1"))?;}
    let mut f=std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(f,"unix_ms={} pid={} {}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis(),std::process::id(),stage)?;
    if durable {f.sync_all()?;std::fs::File::open(dir)?.sync_all()?;}
    Ok(())
}
fn report_failure(stage:&str,error:&str){
    use std::sync::atomic::{AtomicU64,Ordering};
    static DROPPED:AtomicU64=AtomicU64::new(0);
    static LAST:AtomicU64=AtomicU64::new(0);
    DROPPED.fetch_add(1,Ordering::Relaxed);
    let now=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
    let last=LAST.load(Ordering::Relaxed);
    if now.saturating_sub(last)>=10 && LAST.compare_exchange(last,now,Ordering::Relaxed,Ordering::Relaxed).is_ok(){
        let count=DROPPED.swap(0,Ordering::Relaxed);
        crate::maker::operational_log(format_args!("[MAKER TRACE] dropped={count} last_stage={stage}: {error}"));
    }
}
pub(super) fn mark(stage:&str,durable:bool){if let Err(e)=event(stage,durable){report_failure(stage,&e.to_string());}}
pub(super) fn snapshot_best_effort(graph:&crate::maker_graph::Graph,source:&str,w:i32,h:i32,t:f32,work:u64){
    if let Err(e)=snapshot(graph,source,w,h,t,work){report_failure("snapshot",&e);}
}
pub(super) fn snapshot(graph:&crate::maker_graph::Graph,source:&str,w:i32,h:i32,t:f32,work:u64)->Result<(),String>{
    (||->std::io::Result<()> {
        let dir=directory()?;
        let path=dir.join("maker_last_render.json");
        let tmp=dir.join(format!(".maker-last-{}.tmp",std::process::id()));
        let hash=source.bytes().fold(14695981039346656037u64,|h,b|(h^b as u64).wrapping_mul(1099511628211));
        let record=serde_json::json!({"pid":std::process::id(),"unix_ms":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64,"width":w,"height":h,"time":t,"work_per_pixel":work,"source_fingerprint":format!("{hash:016x}"),"graph":crate::maker::graph_to_wire(graph),"glsl":source});
        let mut f=std::fs::File::create(&tmp)?;serde_json::to_writer(&mut f,&record).map_err(std::io::Error::other)?;f.sync_all()?;
        std::fs::rename(tmp,path)?;std::fs::File::open(dir)?.sync_all()?;Ok(())
    })().map_err(|e|format!("Temporary render snapshot failed: {e}"))
}
