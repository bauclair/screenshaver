//! Screenshaver Maker: typed node-graph workspace (Batch 03).
use std::{sync::Arc, time::Instant};
use sdl2::{event::Event, mouse::MouseButton};

use crate::maker_graph::{Graph, NodeId};
use crate::maker_compiler::ExportFormat;
#[path = "maker_routing.rs"]
mod maker_routing;
use maker_routing::{Box2, PortPair, Routes};
use std::collections::{BTreeMap,HashMap};
use serde_json::Value;
use crate::maker_module::{Module, Trust};
use crate::maker_library;

const MAKER_BRANDING_IMAGE: &[u8] = include_bytes!("../assets/screenshaver-splash.png");

const MAKER_MENU_ICON_RGBA: [u8; 1936] = [
    62, 1, 0, 192, 142, 3, 0, 198, 142, 3, 0, 198, 142, 3, 0, 198, 142, 3, 0, 198, 142, 3, 0, 198, 142, 3, 0, 198, 142, 3, 0, 198, 142, 3, 0, 198, 142, 3, 0, 198, 142, 3, 0, 198, 142, 3, 0, 198, 142, 3, 0, 198, 142, 3, 0, 198, 142, 3, 0, 198, 142, 3, 0, 198, 142, 3, 0, 198, 142, 3, 0, 198, 130, 3, 0, 202, 53, 1, 0, 229, 40, 1, 0, 235, 37, 0, 0, 214, 142, 18, 0, 198, 255, 32, 0, 255, 255, 32, 0, 255, 255, 32, 0, 255, 255, 32, 0, 255, 255, 32, 0, 255, 255, 32, 0, 255, 255, 32, 0, 255, 255, 32, 0, 255, 255, 32, 0, 255, 255, 32, 0, 255, 255, 32, 0, 255, 255, 32, 0, 255, 255, 32, 0, 255, 255, 32, 0, 255, 255, 32, 0, 255, 255, 32, 0, 255, 229, 29, 0, 255, 39, 5, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 40, 5, 0, 235, 142, 33, 0, 198, 255, 60, 0, 255, 255, 60, 0, 255, 255, 60, 0, 255, 255, 60, 0, 255, 255, 60, 0, 255, 255, 60, 0, 255, 255, 60, 0, 255, 255, 60, 0, 255, 255, 60, 0, 255, 255, 60, 0, 255, 255, 60, 0, 255, 255, 60, 0, 255, 255, 60, 0, 255, 255, 60, 0, 255, 255, 60, 0, 255, 229, 54, 0, 255, 39, 9, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 77, 19, 0, 220, 142, 48, 0, 198, 255, 87, 0, 255, 255, 87, 0, 255, 255, 87, 0, 255, 255, 87, 0, 255, 255, 87, 0, 255, 255, 87, 0, 255, 255, 87, 0, 255, 255, 87, 0, 255, 255, 87, 0, 255, 255, 87, 0, 255, 255, 87, 0, 255, 255, 87, 0, 255, 255, 87, 0, 255, 255, 87, 0, 255, 232, 79, 0, 255, 42, 14, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 99, 34, 0, 255, 140, 48, 0, 198, 142, 64, 0, 198, 255, 115, 0, 255, 255, 115, 0, 255, 255, 115, 0, 255, 255, 115, 0, 255, 255, 115, 0, 255, 255, 115, 0, 255, 255, 115, 0, 255, 255, 115, 0, 255, 255, 115, 0, 255, 255, 115, 0, 255, 255, 115, 0, 255, 255, 115, 0, 255, 255, 115, 0, 255, 232, 105, 0, 255, 42, 19, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 99, 45, 0, 255, 253, 114, 0, 255, 142, 64, 0, 198, 142, 79, 0, 198, 255, 141, 0, 255, 255, 141, 0, 255, 255, 141, 0, 255, 255, 141, 0, 255, 255, 141, 0, 255, 255, 141, 0, 255, 255, 141, 0, 255, 255, 141, 0, 255, 255, 141, 0, 255, 255, 141, 0, 255, 255, 141, 0, 255, 255, 141, 0, 255, 232, 128, 0, 255, 42, 23, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 99, 55, 0, 255, 253, 140, 0, 255, 255, 141, 0, 255, 142, 79, 0, 198, 142, 94, 0, 198, 255, 168, 0, 255, 255, 168, 0, 255, 255, 168, 0, 255, 255, 168, 0, 255, 255, 168, 0, 255, 255, 168, 0, 255, 255, 168, 0, 255, 255, 168, 0, 255, 255, 168, 0, 255, 255, 168, 0, 255, 255, 168, 0, 255, 232, 153, 0, 255, 42, 28, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 99, 65, 0, 255, 253, 167, 0, 255, 255, 168, 0, 255, 255, 168, 0, 255, 142, 94, 0, 198, 142, 108, 0, 198, 255, 194, 0, 255, 255, 194, 0, 255, 255, 194, 0, 255, 255, 194, 0, 255, 255, 194, 0, 255, 255, 194, 0, 255, 255, 194, 0, 255, 255, 194, 0, 255, 255, 194, 0, 255, 255, 194, 0, 255, 232, 177, 0, 255, 42, 32, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 99, 75, 0, 255, 253, 192, 0, 255, 255, 194, 0, 255, 255, 194, 0, 255, 255, 194, 0, 255, 142, 108, 0, 198, 142, 124, 0, 198, 255, 221, 0, 255, 255, 221, 0, 255, 255, 221, 0, 255, 255, 221, 0, 255, 255, 221, 0, 255, 255, 221, 0, 255, 255, 221, 0, 255, 255, 221, 0, 255, 255, 221, 0, 255, 232, 201, 0, 255, 42, 36, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 99, 86, 0, 255, 253, 219, 0, 255, 255, 221, 0, 255, 255, 221, 0, 255, 255, 221, 0, 255, 255, 221, 0, 255, 142, 124, 0, 198, 138, 134, 0, 198, 248, 241, 0, 255, 248, 241, 0, 255, 248, 241, 0, 255, 248, 241, 0, 255, 248, 241, 0, 255, 248, 241, 0, 255, 248, 241, 0, 255, 248, 241, 0, 255, 226, 219, 0, 255, 41, 40, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 67, 65, 0, 255, 246, 239, 0, 255, 248, 241, 0, 255, 248, 241, 0, 255, 248, 241, 0, 255, 248, 241, 0, 255, 248, 241, 0, 255, 138, 134, 0, 198, 118, 133, 0, 198, 213, 238, 0, 255, 213, 238, 0, 255, 213, 238, 0, 255, 213, 238, 0, 255, 213, 238, 0, 255, 213, 238, 0, 255, 213, 238, 0, 255, 194, 217, 0, 255, 35, 39, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 6, 7, 0, 255, 150, 168, 0, 255, 213, 238, 0, 255, 213, 238, 0, 255, 213, 238, 0, 255, 213, 238, 0, 255, 213, 238, 0, 255, 118, 133, 0, 198, 99, 131, 0, 198, 178, 235, 0, 255, 178, 235, 0, 255, 178, 235, 0, 255, 178, 235, 0, 255, 178, 235, 0, 255, 178, 235, 0, 255, 162, 214, 0, 255, 29, 39, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 4, 6, 0, 255, 124, 163, 0, 255, 178, 235, 0, 255, 178, 235, 0, 255, 178, 235, 0, 255, 178, 235, 0, 255, 99, 131, 0, 198, 80, 129, 0, 198, 143, 231, 0, 255, 143, 231, 0, 255, 143, 231, 0, 255, 143, 231, 0, 255, 143, 231, 0, 255, 130, 210, 0, 255, 24, 38, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 56, 90, 0, 255, 38, 61, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 3, 5, 0, 255, 99, 160, 0, 255, 143, 231, 0, 255, 143, 231, 0, 255, 143, 231, 0, 255, 80, 129, 0, 198, 59, 126, 0, 198, 108, 228, 0, 255, 108, 228, 0, 255, 108, 228, 0, 255, 108, 228, 0, 255, 98, 207, 0, 255, 18, 38, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 42, 89, 0, 255, 107, 226, 0, 255, 88, 186, 0, 255, 1, 3, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 3, 5, 0, 255, 75, 158, 0, 255, 108, 228, 0, 255, 108, 228, 0, 255, 59, 126, 0, 198, 40, 125, 0, 198, 73, 224, 0, 255, 73, 224, 0, 255, 73, 224, 0, 255, 66, 204, 0, 255, 12, 37, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 28, 87, 0, 255, 72, 222, 0, 255, 73, 224, 0, 255, 73, 224, 0, 255, 28, 87, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 2, 5, 0, 255, 51, 155, 0, 255, 73, 224, 0, 255, 40, 125, 0, 198, 28, 118, 15, 198, 52, 214, 29, 255, 52, 214, 29, 255, 47, 195, 26, 255, 9, 35, 5, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 20, 83, 11, 255, 52, 212, 29, 255, 52, 214, 29, 255, 52, 214, 29, 255, 52, 214, 29, 255, 48, 196, 26, 255, 3, 12, 2, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 1, 5, 1, 255, 36, 149, 20, 255, 28, 118, 15, 198, 18, 113, 35, 198, 33, 203, 62, 255, 30, 185, 56, 255, 5, 33, 10, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 13, 79, 24, 255, 33, 201, 62, 255, 33, 203, 62, 255, 33, 203, 62, 255, 33, 203, 62, 255, 33, 203, 62, 255, 33, 203, 62, 255, 17, 107, 33, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 1, 5, 1, 255, 15, 85, 27, 209, 8, 107, 53, 198, 13, 175, 86, 255, 2, 32, 16, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 5, 75, 37, 255, 14, 190, 94, 255, 14, 192, 95, 255, 14, 192, 95, 255, 14, 192, 95, 255, 14, 192, 95, 255, 14, 192, 95, 255, 14, 192, 95, 255, 14, 186, 92, 255, 2, 24, 12, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 2, 31, 16, 235, 0, 90, 67, 201, 0, 29, 21, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 68, 50, 255, 0, 175, 129, 255, 0, 176, 130, 255, 0, 176, 130, 255, 0, 176, 130, 255, 0, 176, 130, 255, 0, 176, 130, 255, 0, 176, 130, 255, 0, 176, 130, 255, 0, 176, 130, 255, 0, 117, 87, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 28, 21, 235, 0, 31, 37, 229, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 58, 68, 255, 0, 143, 169, 255, 0, 144, 170, 255, 0, 144, 170, 255, 0, 144, 170, 255, 0, 144, 170, 255, 0, 144, 170, 255, 0, 144, 170, 255, 0, 144, 170, 255, 0, 144, 170, 255, 0, 144, 170, 255, 0, 144, 170, 255, 0, 35, 41, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 23, 27, 235, 0, 17, 34, 235, 0, 0, 0, 255, 0, 0, 0, 255, 0, 45, 84, 255, 0, 113, 208, 255, 0, 113, 209, 255, 0, 113, 209, 255, 0, 113, 209, 255, 0, 113, 209, 255, 0, 113, 209, 255, 0, 113, 209, 255, 0, 113, 209, 255, 0, 113, 209, 255, 0, 113, 209, 255, 0, 113, 209, 255, 0, 113, 209, 255, 0, 91, 169, 255, 0, 1, 2, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 17, 34, 235, 0, 15, 45, 206, 0, 13, 40, 235, 0, 24, 74, 221, 0, 45, 138, 198, 0, 45, 138, 198, 0, 45, 138, 198, 0, 45, 138, 198, 0, 45, 138, 198, 0, 45, 138, 198, 0, 45, 138, 198, 0, 45, 138, 198, 0, 45, 138, 198, 0, 45, 138, 198, 0, 45, 138, 198, 0, 45, 138, 198, 0, 45, 138, 198, 0, 45, 138, 198, 0, 23, 72, 222, 0, 13, 40, 235, 0, 13, 40, 235, 0, 13, 40, 235, 0, 10, 30, 220
];



// Maker's last *run* range belongs to application state, not a .smprj or
// the canonical discovery checkpoint. Preserve every unrelated Maker JSON field.
const MAKER_DEFAULT_MIN_WIRES:usize=2;
const MAKER_DEFAULT_MAX_WIRES:usize=5;

fn maker_state_path()->Option<std::path::PathBuf>{
    Some(std::path::PathBuf::from(std::env::var_os("HOME")?)
        .join(".config/screenshaver/maker/state.json"))
}

// Preview height is application UI state, never part of a .smprj graph.
fn maker_saved_preview_height()->Option<f32>{
    let path=maker_state_path()?;
    let contents=std::fs::read_to_string(path).ok()?;
    let root:serde_json::Value=serde_json::from_str(&contents).ok()?;
    let height=root.pointer("/maker/ui/live_preview_height")?.as_f64()? as f32;
    (height.is_finite() && (120.0..=10000.0).contains(&height)).then_some(height)
}

fn maker_save_preview_height(height:f32)->Result<(),String>{
    if !height.is_finite() || height<120.0 {return Err("Invalid preview height".into());}
    let path=maker_state_path().ok_or("HOME is not set")?;
    let parent=path.parent().ok_or("Invalid Maker state path")?;
    std::fs::create_dir_all(parent).map_err(|e|e.to_string())?;
    let mut root=match std::fs::read_to_string(&path){
        Ok(contents)=>serde_json::from_str::<serde_json::Value>(&contents)
            .map_err(|e|format!("Maker state JSON is invalid; not overwriting: {e}"))?,
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>serde_json::json!({}),
        Err(e)=>return Err(e.to_string()),
    };
    let maker=root.as_object_mut().ok_or("Maker state root is not an object")?
        .entry("maker").or_insert_with(||serde_json::json!({}));
    let ui=maker.as_object_mut().ok_or("Maker state 'maker' is not an object")?
        .entry("ui").or_insert_with(||serde_json::json!({}));
    ui.as_object_mut().ok_or("Maker state 'maker.ui' is not an object")?
        .insert("live_preview_height".into(),serde_json::json!(height));
    let bytes=serde_json::to_vec_pretty(&root).map_err(|e|e.to_string())?;
    use std::io::Write;
    let mut temporary=None;
    for attempt in 0..16u32 {
        let candidate=parent.join(format!(".state.json.tmp-{}-{attempt}.tmp",std::process::id()));
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&candidate){
            Ok(file)=>{temporary=Some((candidate,file));break},
            Err(e) if e.kind()==std::io::ErrorKind::AlreadyExists=>continue,
            Err(e)=>return Err(e.to_string()),
        }
    }
    let (temp_path,mut file)=temporary.ok_or("Unable to allocate Maker state temporary file")?;
    let result=(||->Result<(),String>{
        file.write_all(&bytes).map_err(|e|e.to_string())?;
        file.sync_all().map_err(|e|e.to_string())?;
        drop(file);
        std::fs::rename(&temp_path,&path).map_err(|e|e.to_string())?;
        Ok(())
    })();
    if result.is_err(){let _=std::fs::remove_file(&temp_path);}
    result
}

fn maker_last_discovery_settings()->(usize,usize,usize,usize){
    let fallback=(MAKER_DEFAULT_MIN_WIRES,MAKER_DEFAULT_MAX_WIRES,DEFAULT_MAX_ENUMERATIONS,DEFAULT_MAX_CANDIDATES);
    let Some(path)=maker_state_path() else{return fallback};
    let Ok(contents)=std::fs::read_to_string(&path) else{return fallback};
    let Ok(root)=serde_json::from_str::<serde_json::Value>(&contents) else{
        crate::maker::operational_log(format_args!("[MAKER STATE] Invalid Maker state JSON; using wire defaults"));
        return fallback;
    };
    let Some(range)=root.pointer("/maker/discovery") else{return fallback};
    let (Some(min),Some(max))=(range["min_wire_connections"].as_u64(),range["max_wire_connections"].as_u64()) else{return fallback};
    if !(2..=25).contains(&min) || !(2..=25).contains(&max) || min>max {return fallback}
    // Validate against the actual selectable values; older state files without
    // these keys retain the existing defaults independently.
    let enumerations=range["max_enumerations"].as_u64()
        .and_then(|n|usize::try_from(n).ok())
        .filter(|n|[10_000,25_000,50_000,100_000].contains(n))
        .unwrap_or(DEFAULT_MAX_ENUMERATIONS);
    let candidates=range["max_glsl_candidates"].as_u64()
        .and_then(|n|usize::try_from(n).ok())
        .filter(|n|[100,250,500,1_000].contains(n))
        .unwrap_or(DEFAULT_MAX_CANDIDATES);
    (min as usize,max as usize,enumerations,candidates)
}

fn maker_save_discovery_settings(min:usize,max:usize,enumerations:usize,candidates:usize)->Result<(),String>{
    if !(2..=25).contains(&min) || !(2..=25).contains(&max) || min>max {
        return Err("Invalid Maker wire-connection range".into());
    }
    if ![10_000,25_000,50_000,100_000].contains(&enumerations)
        || ![100,250,500,1_000].contains(&candidates) {
        return Err("Invalid Maker discovery limits".into());
    }
    let path=maker_state_path().ok_or("HOME is not set")?;
    let parent=path.parent().ok_or("Invalid Maker state path")?;
    std::fs::create_dir_all(parent).map_err(|e|e.to_string())?;
    // Read at write time to retain any other Maker state fields.
    // Never overwrite malformed existing Maker state JSON.
    let mut root=match std::fs::read_to_string(&path){
        Ok(contents)=>serde_json::from_str::<serde_json::Value>(&contents)
            .map_err(|e|format!("Maker state JSON is invalid; not overwriting: {e}"))?,
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>serde_json::json!({}),
        Err(e)=>return Err(e.to_string()),
    };
    let object=root.as_object_mut().ok_or("Maker state JSON root is not an object")?;
    let maker=object.entry("maker").or_insert_with(||serde_json::json!({}));
    let maker=maker.as_object_mut().ok_or("Maker state 'maker' is not an object")?;
    let discovery=maker.entry("discovery").or_insert_with(||serde_json::json!({}));
    let discovery=discovery.as_object_mut().ok_or("Maker state 'maker.discovery' is not an object")?;
    discovery.insert("min_wire_connections".into(),serde_json::json!(min));
    discovery.insert("max_wire_connections".into(),serde_json::json!(max));
    discovery.insert("max_enumerations".into(),serde_json::json!(enumerations));
    discovery.insert("max_glsl_candidates".into(),serde_json::json!(candidates));
    let bytes=serde_json::to_vec_pretty(&root).map_err(|e|e.to_string())?;
    use std::io::Write;
    // Unique create_new temp avoids clobbering another writer's temporary file.
    let mut temporary=None;
    for attempt in 0..16u32 {
        let candidate=parent.join(format!(".state.json.tmp-{}-{attempt}.tmp",std::process::id()));
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&candidate){
            Ok(file)=>{temporary=Some((candidate,file));break},
            Err(e) if e.kind()==std::io::ErrorKind::AlreadyExists=>continue,
            Err(e)=>return Err(e.to_string()),
        }
    }
    let (temp_path,mut file)=temporary.ok_or("Unable to allocate Maker state temporary file")?;
    let result=(||->Result<(),String>{
        file.write_all(&bytes).map_err(|e|e.to_string())?;
        file.sync_all().map_err(|e|e.to_string())?;
        drop(file);
        std::fs::rename(&temp_path,&path).map_err(|e|e.to_string())?;
        Ok(())
    })();
    if result.is_err(){let _=std::fs::remove_file(&temp_path);}
    result
}

// Recent projects are stored alongside discovery preferences, not in .smprj files.
const MAKER_RECENT_PROJECT_LIMIT:usize=8;
fn maker_recent_projects()->Vec<std::path::PathBuf>{
    let Some(path)=maker_state_path() else{return Vec::new()};
    let Ok(data)=std::fs::read_to_string(path) else{return Vec::new()};
    let Ok(root)=serde_json::from_str::<serde_json::Value>(&data) else{return Vec::new()};
    let mut paths=Vec::new();
    if let Some(items)=root.pointer("/maker/recent_projects").and_then(|v|v.as_array()){
        for item in items {
            if let Some(path)=item.as_str(){
                let path=std::path::PathBuf::from(path);
                if path.is_absolute() && !paths.contains(&path){paths.push(path);}
                if paths.len()==MAKER_RECENT_PROJECT_LIMIT{break;}
            }
        }
    }
    paths
}
fn maker_record_recent_project(path:&std::path::Path)->Result<Vec<std::path::PathBuf>,String>{
    let path=maker_project_path(path)?;
    // Canonicalize when possible so the same project cannot appear under aliases.
    let path=std::fs::canonicalize(&path).unwrap_or(path);
    let mut recent=maker_recent_projects();
    recent.retain(|other|other!=&path);
    recent.insert(0,path);
    recent.truncate(MAKER_RECENT_PROJECT_LIMIT);
    let state=maker_state_path().ok_or("HOME is not set")?;
    let parent=state.parent().ok_or("Invalid Maker state path")?;
    std::fs::create_dir_all(parent).map_err(|e|e.to_string())?;
    let mut root=match std::fs::read_to_string(&state){
        Ok(contents)=>serde_json::from_str::<serde_json::Value>(&contents)
            .map_err(|e|format!("Maker state JSON is invalid; not overwriting: {e}"))?,
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>serde_json::json!({}),
        Err(e)=>return Err(e.to_string()),
    };
    let object=root.as_object_mut().ok_or("Maker state JSON root is not an object")?;
    let maker=object.entry("maker").or_insert_with(||serde_json::json!({}));
    let maker=maker.as_object_mut().ok_or("Maker state 'maker' is not an object")?;
    maker.insert("recent_projects".into(),serde_json::json!(recent));
    let bytes=serde_json::to_vec_pretty(&root).map_err(|e|e.to_string())?;
    use std::io::Write;
    let mut temporary=None;
    for attempt in 0..16u32 {
        let candidate=parent.join(format!(".state.json.tmp-{}-{attempt}.tmp",std::process::id()));
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&candidate){
            Ok(file)=>{temporary=Some((candidate,file));break},
            Err(e) if e.kind()==std::io::ErrorKind::AlreadyExists=>continue,
            Err(e)=>return Err(e.to_string()),
        }
    }
    let (temp_path,mut file)=temporary.ok_or("Unable to allocate Maker state temporary file")?;
    let result=(||->Result<(),String>{
        file.write_all(&bytes).map_err(|e|e.to_string())?;
        file.sync_all().map_err(|e|e.to_string())?;
        drop(file);
        std::fs::rename(&temp_path,&state).map_err(|e|e.to_string())?;
        Ok(())
    })();
    if result.is_err(){let _=std::fs::remove_file(&temp_path);}
    result.map(|()|recent)
}

/// Thumbnail-only visual descriptors; computed once after rendering.
#[derive(Clone, Copy, Default)]
struct VisualMetrics { interest:f32, complexity:f32, contrast:f32, entropy:f32, vibrancy:f32 }

fn visual_metrics(rgba:&[u8], width:usize, height:usize)->VisualMetrics {
    if width<3 || height<3 || rgba.len()<width.saturating_mul(height).saturating_mul(4) {return VisualMetrics::default();}
    let n=width*height;
    let mut lum=vec![0.0f32;n];
    let mut sat_sum=0.0f32;
    let mut histogram=[0usize;32];
    let mut sum=0.0f32;
    for (i,px) in rgba.chunks_exact(4).take(n).enumerate() {
        let r=px[0] as f32/255.0;let g=px[1] as f32/255.0;let b=px[2] as f32/255.0;
        let y=0.2126*r+0.7152*g+0.0722*b;
        lum[i]=y;sum+=y;
        histogram[((y*32.0) as usize).min(31)]+=1;
        let hi=r.max(g).max(b);let lo=r.min(g).min(b);
        sat_sum+=if hi>0.001 {(hi-lo)/hi} else {0.0};
    }
    let mean=sum/n as f32;
    let variance=lum.iter().map(|v|(v-mean)*(v-mean)).sum::<f32>()/n as f32;
    let contrast=(variance.sqrt()*2.0).clamp(0.0,1.0);
    let mut edge_sum=0.0f32;let mut edge_bins=[0usize;16];let mut edges=0usize;
    for y in 1..height-1 {for x in 1..width-1 {
        let i=y*width+x;
        let dx=(lum[i+1]-lum[i-1]).abs()*0.5;
        let dy=(lum[i+width]-lum[i-width]).abs()*0.5;
        let e=(dx*dx+dy*dy).sqrt();
        edge_sum+=e;edge_bins[((e*16.0) as usize).min(15)]+=1;edges+=1;
    }}
    // Soft edge strength avoids rewarding single-pixel aliasing as much as raw edge counts.
    let complexity=(edge_sum/edges.max(1) as f32*6.0).clamp(0.0,1.0);
    let entropy_of=|bins:&[usize],total:usize|->f32 {
        if total==0{return 0.0;}
        let mut e=0.0f32;
        for &count in bins {if count>0 {let p=count as f32/total as f32;e-=p*p.log2();}}
        (e/(bins.len() as f32).log2()).clamp(0.0,1.0)
    };
    let entropy=(entropy_of(&histogram,n)*0.6+entropy_of(&edge_bins,edges)*0.4).clamp(0.0,1.0);
    let vibrancy=(sat_sum/n as f32*contrast.sqrt()).clamp(0.0,1.0);
    let interest=0.35*complexity+0.30*contrast+0.20*entropy+0.15*vibrancy;
    VisualMetrics{interest,complexity,contrast,entropy,vibrancy}
}

fn candidate_score(candidate:&Candidate, sort:u8)->f32 {
    match sort {
        1=>candidate.visual.complexity,
        2=>candidate.visual.contrast,
        3=>candidate.visual.entropy,
        4=>candidate.visual.vibrancy,
        5=>0.0,
        6=>candidate.motion.unwrap_or(-1.0),
        7=>candidate.connections.len() as f32,
        8=>if candidate.uses_callable_field {1.0} else {0.0},
        _=>candidate.visual.interest * (if candidate.connection_candidate {
            match candidate.motion {Some(m) if m<0.003=>0.20,Some(m)=>0.75+0.25*(m*20.0).min(1.0),None=>0.65}
        } else {1.0}),
    }
}

#[derive(Clone)]
pub struct CandidateAnalysis {
    pub name: String,
    pub lines: Vec<String>,
}

#[derive(Clone)]
struct Candidate {
    name: String,
    parameters: HashMap<NodeId, HashMap<String,Value>>,
    modules: HashMap<NodeId,String>,
    connections: Vec<crate::maker_graph::Connection>,
    texture: egui::TextureHandle,
    visual: VisualMetrics,
    motion: Option<f32>,
    motion_failed: bool,
    connection_candidate: bool,
    uses_callable_field: bool,
}
#[derive(Clone)]
pub struct CandidateJob {
    pub graph: Graph,
    pub name: String,
    pub source_revision: u64,
    pub connection_candidate: bool,
}
// One independent color draw per editable color3 parameter, per GPU candidate.
// The values live in the candidate graph, not in shader-frame randomness.
fn randomize_candidate_colors(graph:&mut Graph, mut state:u64){
    if graph.nodes.iter().any(|n|n.unresolved()){return;}
    fn next(state:&mut u64)->f64 {
        *state=state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z=*state;
        z=(z^(z>>30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z=(z^(z>>27)).wrapping_mul(0x94D049BB133111EB);
        ((z^(z>>31))>>11) as f64 * (1.0/9007199254740992.0)
    }
    for node in &mut graph.nodes {
        let ids:Vec<String>=node.definition().parameters.iter()
            .filter(|p|p.ty=="color3").map(|p|p.id.clone()).collect();
        for id in ids {
            if node.module_id=="screenshaver.lighting.layered_refractive_boundary_traversal" && id=="tint" {continue;}
            node.parameters.insert(id,serde_json::json!([
                next(&mut state),next(&mut state),next(&mut state)
            ]));
        }
    }
}

fn is_speed_parameter(id:&str)->bool {
    // Animation speed is user-controlled and never part of candidate discovery.
    id.eq_ignore_ascii_case("speed") || id.eq_ignore_ascii_case("animation_speed")
}
// Decimal multiplication avoids overflow without introducing a bigint dependency.
fn multiply_decimal(value:&mut Vec<u8>, factor:usize) {
    let mut carry=0u128;
    for digit in value.iter_mut() {
        let next=(*digit as u128)*(factor as u128)+carry;
        *digit=(next%10) as u8;
        carry=next/10;
    }
    while carry>0 {value.push((carry%10) as u8);carry/=10;}
}
fn decimal_string(value:&[u8])->String {
    let digits:String=value.iter().rev().map(|d|char::from(b'0'+*d)).collect();
    let mut result=String::new();
    for (i,c) in digits.chars().enumerate() {
        if i>0 && (digits.len()-i)%3==0 {result.push(',');}
        result.push(c);
    }
    result
}
fn parameter_permutation_count(graph:&Graph)->String {
    if graph.nodes.iter().any(|n|n.unresolved()){return "Unavailable while modules are unresolved".into();}
    let mut total=vec![1u8];
    for node in &graph.nodes {
        for param in &node.definition().parameters {
            if is_speed_parameter(&param.id) {continue;}
            let choices=match param.ty.as_str() {
                "float" => {
                    let lo=param.min as f64;let hi=param.max as f64;
                    if !lo.is_finite() || !hi.is_finite() {1}
                    else if lo==hi {1} else {3}
                }
                "choice" => param.options.len().max(1),
                _ => 1,
            };
            multiply_decimal(&mut total,choices);
        }
    }
    decimal_string(&total)
}
// CPU-only eligibility check. This deliberately does not generate GLSL or touch OpenGL.
// Required inputs must be connected on every node that contributes to Output Color;
// disconnected modules do not affect the rendered shader.

// CPU-only estimate of expanded evaluation work. Saturating arithmetic prevents
// large fan-out graphs from overflowing the score. This is a qualification
// heuristic, not a claim about actual GPU instructions or compilation time.
pub(crate) fn graph_compile_cost(graph:&Graph)->u64 {
    if graph.output_reachable_ids().iter().any(|id|graph.node(*id).is_some_and(|n|n.unresolved())){return u64::MAX;}
    use std::collections::{HashMap,HashSet};
    fn cost(graph:&Graph,id:NodeId,cache:&mut HashMap<NodeId,u64>,visiting:&mut HashSet<NodeId>)->u64 {
        if let Some(&v)=cache.get(&id){return v}
        if !visiting.insert(id){return u64::MAX/4} // defensive cycle guard
        let Some(node)=graph.node(id) else {return u64::MAX/4};
        let input=|port:usize,cache:&mut HashMap<NodeId,u64>,visiting:&mut HashSet<NodeId>| -> u64 {
            graph.input_connection(id,port).map(|c|cost(graph,c.from,cache,visiting)).unwrap_or(0)
        };
        let kind=node.module_id.as_str();
        let mut inputs=Vec::new();
        for port in 0..node.input_count(){inputs.push(input(port,cache,visiting));}
        let total=inputs.iter().fold(0u64,|s,&v|s.saturating_add(v));
        // Ray marchers repeatedly sample their first input, including normal,
        // AO and curvature estimates. Optical marchers also resample background.
        // Conservative weighted factors approximate optimizer expansion pressure.
        let value=match kind {
            "screenshaver.fields.sdf_raymarch"=>
                300u64.saturating_add(inputs.first().copied().unwrap_or(0).saturating_mul(128*8))
                    .saturating_add(inputs.get(1).copied().unwrap_or(0)),
            "screenshaver.lighting.refractive_surface"=>
                250u64.saturating_add(inputs.first().copied().unwrap_or(0).saturating_mul(96*5))
                    .saturating_add(inputs.get(1).copied().unwrap_or(0).saturating_mul(3)),
            "screenshaver.lighting.layered_refractive_boundary_traversal"=>
                300u64.saturating_add(inputs.first().copied().unwrap_or(0).saturating_mul(4*48+4*9))
                    .saturating_add(inputs.get(1).copied().unwrap_or(0).saturating_mul(5)),
            "screenshaver.fields.traversal"=>
                60u64.saturating_add(inputs.first().copied().unwrap_or(0).saturating_mul(20))
                    .saturating_add(total.saturating_sub(inputs.first().copied().unwrap_or(0))),
            "screenshaver.geometry.domain_fold_3d"=>20u64.saturating_add(total.saturating_mul(8)),
            _=>8u64.saturating_add(total),
        };
        visiting.remove(&id);cache.insert(id,value);value
    }
    let Some(out)=graph.nodes.iter().find(|n|n.definition().terminal) else{return u64::MAX};
    cost(graph,out.id,&mut HashMap::new(),&mut HashSet::new())
}
// Initially calibrated as a conservative circuit-protection threshold.
// This may exclude valid complex shaders; rejected graphs are not invalid.
pub(crate) const MAX_COMPILE_COST:u64=100_000;
// The compiler emits each callable node/port once; this estimate models source
// generation rather than multiplying the cost of every call site by loop bounds.
// Keep the historical expanded-work estimate separately as a runtime risk guard.
const MAX_ESTIMATED_RUNTIME_WORK:u64=50_000_000;
const MAX_CONNECTION_GPU_JOBS:usize=32; // bounded per discovery batch
const DEFAULT_MAX_CANDIDATES:usize=250;
pub(crate) fn graph_source_cost(graph:&Graph)->u64 {
    if graph.output_reachable_ids().iter().any(|id|graph.node(*id).is_some_and(|n|n.unresolved())){return u64::MAX;}
    let reachable=graph.output_reachable_ids();
    let mut score=0u64;
    for node in graph.nodes.iter().filter(|n|reachable.contains(&n.id)) {
        let definition=node.definition();
        score=score.saturating_add(16)
            .saturating_add(definition.glsl_outputs.iter().map(|s|s.len() as u64).sum::<u64>())
            .saturating_add(definition.helpers.len() as u64);
        score=score.saturating_add(match node.module_id.as_str() {
            "screenshaver.fields.sdf_raymarch"=>8_000,
            "screenshaver.lighting.refractive_surface"=>5_000,
            "screenshaver.lighting.layered_refractive_boundary_traversal"=>8_000,
            "screenshaver.fields.traversal"=>3_000,
            "screenshaver.geometry.domain_fold_3d"=>1_000,
            _=>0,
        });
    }
    score
}


// Single admission policy used by both CPU discovery and the isolated GPU worker.
// These estimates are conservative screening heuristics, not proof of GPU safety.
// Initial structural screening profiles, based on the 206-module catalog.
// Relative units only: no claim of a hardware-independent execution bound.
fn safety_profile(id:&str)->Option<(u64,u64)> {
    Some(match id {

        "screenshaver.fields.iterative_orbit_correlation_field_3d" => (32768, 1),
        "screenshaver.geometry.radial_branched_distance_field_2d" => (262144, 1),
        "screenshaver.coordinates.complex_plane_transformation_3d" => (1024, 1),
        "screenshaver.fields.periodic_cell_envelope_2d" => (1024, 1),
        "screenshaver.fields.nonlinear_harmonic_feedback_field_2d" => (131072, 1),
        "screenshaver.math.differentiable_absolute_response" => (256, 1),
        "screenshaver.noise.simplex_gradient_noise_3d" => (4096, 1),
        "screenshaver.math.wavelength_dependent_refractive_index" => (512, 1),
        "screenshaver.color.xyz_linear_rgb_conversion" => (384, 1),
        "screenshaver.fields.ordered_bayer_sampling_field_2d" => (512, 1),
        "screenshaver.coordinates.radially_modulated_rotation_2d" => (2048, 1),
        "screenshaver.fields.orthogonal_product_flare_field_2d" => (1024, 1),
        "screenshaver.geometry.differential_harmonic_path_3d" => (1536, 1),
        "screenshaver.geometry.concave_star_polygon_distance_field_2d" => (65536, 1),
        "screenshaver.compositing.alpha_aware_rgb_compositor" => (512, 1),
        "screenshaver.geometry.generalized_annular_cross_section_field_3d" => (1536, 1),
        "screenshaver.math.logistic_sigmoid_response" => (256, 1),
        "screenshaver.coordinates.recursive_rectangular_subdivision_field_2d" => (32768, 1),
        "screenshaver.fields.analytical_bokeh_response_3d" => (1024, 1),
        "screenshaver.fields.length_gated_segment_field_2d" => (512, 1),
        "screenshaver.noise.procedural_b_spline_profile_1d" => (2048, 1),
        "screenshaver.fields.height_profile_boundary_field_2d" => (384, 1),
        "screenshaver.compositing.material_aware_field_selector" => (256, 1),
        "screenshaver.coordinates.polygonal_revolution_coordinates_3d" => (1536, 1),
        "screenshaver.geometry.generalized_polyhedral_projection_field_3d" => (2048, 1),
        "screenshaver.geometry.analytical_minkowski_tube_intersection_3d" => (2048, 1),
        "screenshaver.fields.four_dimensional_cyclic_trigonometric_field" => (1024, 1),
        "screenshaver.geometry.convex_faced_box_field_3d" => (1024, 1),
        "screenshaver.coordinates.probabilistic_hierarchical_grid_coordinates_2d" => (8192, 1),
        "screenshaver.fields.truchet_tile_distance_field_2d" => (8192, 1),
        "screenshaver.lighting.ggx_microfacet_lighting" => (2048, 1),
        "screenshaver.coordinates.iterative_vector_reflection_field_2d" => (65536, 1),
        "screenshaver.geometry.ray_disc_intersection_3d" => (2048, 1),
        "screenshaver.coordinates.complex_arithmetic_transformation_2d" => (2048, 1),
        "screenshaver.fields.harmonic_interval_array_field_2d" => (65536, 1),
        "screenshaver.coordinates.axially_modulated_coordinate_warp_2d" => (2048, 1),
        "screenshaver.geometry.superformula_field_2d" => (2048, 1),
        "screenshaver.geometry.superformula_surface_field_3d" => (4096, 1),
        "screenshaver.geometry.superquadric_surface_field_3d" => (2048, 1),
        "screenshaver.geometry.trochoidal_curve_field_2d" => (524288, 1),
        "screenshaver.fields.mandelbox_distance_estimation_field_3d" => (16384, 1),
        "screenshaver.fields.quaternion_julia_field_3d" => (32768, 1),
        "screenshaver.fields.apollonian_circle_packing_field_2d" => (262144, 1),
        "screenshaver.fields.lyapunov_fractal_field_2d" => (65536, 1),
        "screenshaver.fields.newton_convergence_fractal_field_2d" => (262144, 1),
        "screenshaver.color.complex_domain_coloring" => (1024, 1),
        "screenshaver.geometry.torus_knot_link_field_3d" => (4194304, 1),
        "screenshaver.coordinates.hopf_fibration_coordinate_field_3d" => (2048, 1),
        "screenshaver.geometry.algebraic_implicit_surface_field_3d" => (2048, 1),
        "screenshaver.fields.spherical_harmonic_field_3d" => (8192, 1),
        "screenshaver.math.bessel_function_field" => (32768, 1),
        "screenshaver.fields.zernike_polynomial_field_2d" => (4096, 1),
        "screenshaver.fields.standing_wave_nodal_field_2d" => (2048, 1),
        "screenshaver.geometry.cassini_oval_field_2d" => (512, 1),
        "screenshaver.fields.triangular_lattice_cellular_field_2d" => (32768, 1),
        "screenshaver.fields.quadratic_complex_escape_time_field_2d" => (262144, 1),
        "screenshaver.coordinates.strip_indexed_coordinate_displacement_2d" => (2048, 1),
        "screenshaver.coordinates.iterative_complex_rational_warp_2d" => (65536, 1),
        "screenshaver.geometry.multiscale_polygonal_contour_field_2d" => (131072, 1),
        "screenshaver.noise.fractal_triangle_wave_noise_2d" => (32768, 1),
        "screenshaver.noise.fractal_simplex_noise_3d" => (65536, 1),
        "screenshaver.coordinates.product_harmonic_coordinate_warp_2d" => (8192, 1),
        "screenshaver.coordinates.vortex_street_advection_2d" => (262144, 1),
        "screenshaver.coordinates.voxel_cell_coordinates_3d" => (512, 1),
        "screenshaver.fields.surface_gradient_optical_response_2d" => (512, 1),
        "screenshaver.fields.envelope_modulated_wave_ribbon_2d" => (2048, 1),
        "screenshaver.fields.radial_wave_interference_field_2d" => (65536, 1),
        "screenshaver.fields.multiscale_power_fold_field_3d" => (8192, 1),
        "screenshaver.fields.iterative_trigonometric_proximity_field_2d" => (131072, 1),
        "screenshaver.fields.multiscale_randomized_filament_field_2d" => (65536, 1),
        "screenshaver.coordinates.logarithmic_spherical_coordinates_3d" => (1024, 1),
        "screenshaver.fields.multiscale_harmonic_recurrence_field_3d" => (16384, 1),
        "screenshaver.geometry.animated_smooth_sphere_union_field_3d" => (32768, 1),
        "screenshaver.coordinates.cross_product_trigonometric_warp_3d" => (2048, 1),
        "screenshaver.fields.logarithmic_frequency_sweep_field_2d" => (2048, 1),
        "screenshaver.noise.localized_directional_kernel_noise_2d" => (32768, 1),
        "screenshaver.geometry.depth_coupled_harmonic_surface_field_3d" => (4096, 1),
        "screenshaver.lighting.directional_softmax_environment_lighting" => (4096, 1),
        "screenshaver.lighting.thin_film_interference_rgb_response" => (1024, 1),
        "screenshaver.fields.spherical_power_fractal_field_3d" => (32768, 1),
        "screenshaver.geometry.indexed_spatial_ring_field_3d" => (32768, 1),
        "screenshaver.coordinates.partial_revolution_coordinates_3d" => (2048, 1),
        "screenshaver.fields.rotated_harmonic_fractal_field_2d" => (16384, 1),
        "screenshaver.coordinates.radially_modulated_affine_warp_2d" => (1024, 1),
        "screenshaver.coordinates.expanding_comoving_coordinates_3d" => (512, 1),
        "screenshaver.fields.modulated_spherical_shell_density_field_3d" => (2048, 1),
        "screenshaver.coordinates.toroidal_helical_coordinate_warp_3d" => (2048, 1),
        "screenshaver.noise.simplex_gradient_noise_2d" => (2048, 1),
        "screenshaver.color.yiq_hue_rotation" => (512, 1),
        "screenshaver.animation.cell_indexed_temporal_oscillator_2d" => (2048, 1),
        "screenshaver.coordinates.bipolar_distance_coordinates_2d" => (1024, 1),
        "screenshaver.coordinates.ring_indexed_angular_coordinates_2d" => (2048, 1),
        "screenshaver.coordinates.penrose_tiling_coordinate_field_2d" => (524288, 1),
        "screenshaver.coordinates.ammann_beenker_tiling_field_2d" => (524288, 1),
        "screenshaver.coordinates.hyperbolic_tessellation_field_2d" => (32768, 1),
        "screenshaver.coordinates.archimedean_tiling_coordinates_2d" => (8192, 1),
        "screenshaver.fields.phyllotactic_point_distribution_field" => (262144, 1),
        "screenshaver.geometry.damped_harmonic_curve_field_2d" => (524288, 1),
        "screenshaver.geometry.classical_polar_curve_field_2d" => (524288, 1),
        "screenshaver.geometry.curvature_based_curve_field_2d" => (4194304, 1),
        "screenshaver.geometry.bezier_curve_field_2d" => (524288, 1),
        "screenshaver.geometry.recursive_space_filling_curve_field_2d" => (2097152, 1),
        "screenshaver.math.orthogonal_polynomial_response" => (4096, 1),
        "screenshaver.fields.analytical_diffraction_response" => (32768, 1),
        "screenshaver.fields.gaussian_beam_mode_field" => (8192, 1),
        "screenshaver.fields.hydrogenic_orbital_field_3d" => (8192, 1),
        "screenshaver.fields.analytical_soliton_wave_field" => (2048, 1),
        "screenshaver.math.jacobi_elliptic_function_response" => (262144, 1),
        "screenshaver.math.fresnel_integral_response" => (65536, 1),
        "screenshaver.coordinates.joukowski_coordinate_transformation_2d" => (512, 1),
        "screenshaver.coordinates.generalized_curvilinear_coordinates_2d" => (2048, 1),
        "screenshaver.coordinates.generalized_curvilinear_coordinates_3d" => (2048, 1),
        "screenshaver.color.oklab_oklch_color_conversion" => (1024, 1),
        "screenshaver.color.cie_lab_lch_color_conversion" => (1024, 1),
        "screenshaver.geometry.triply_periodic_surface_field_3d" => (2048, 1),
        "screenshaver.geometry.dupin_cyclide_field_3d" => (1024, 1),
        "screenshaver.geometry.clifford_torus_projection_field" => (1024, 1),
        "screenshaver.geometry.four_dimensional_rotation_transformation" => (1024, 1),
        "screenshaver.geometry.projected_hypercube_wireframe_field_2d" => (524288, 1),
        "screenshaver.fields.variable_radius_disc_chain_field_2d" => (16384, 1),
        "screenshaver.coordinates.reflection_axis_distance_coordinates_2d" => (2048, 1),
        "screenshaver.coordinates.component_wise_coordinate_transformation_2d" => (4096, 1),
        "screenshaver.geometry.axial_harmonic_tube_field_3d" => (8192, 1),
        "screenshaver.coordinates.radial_wave_coordinate_displacement_3d" => (4096, 1),
        "screenshaver.geometry.orthogonal_beam_lattice_field_3d" => (4096, 1),
        "screenshaver.fields.rotational_orbit_trap_field_2d" => (65536, 1),
        "screenshaver.geometry.radial_tapered_primitive_array_field_3d" => (65536, 1),
        "screenshaver.geometry.polyhedral_edge_network_field_3d" => (524288, 1),
        "screenshaver.coordinates.equirectangular_spherical_coordinates" => (2048, 1),
        "screenshaver.coordinates.rgb_to_vector_transformation_2d" => (1024, 1),
        "screenshaver.coordinates.iterative_polar_feedback_warp_2d" => (65536, 1),
        "screenshaver.coordinates.hyperbolic_model_conversion_3d" => (4096, 1),
        "screenshaver.coordinates.hyperbolic_translation_3d" => (8192, 1),
        "screenshaver.fields.gradient_compensated_contour_field" => (2048, 1),
        "screenshaver.geometry.rotational_segment_network_field_2d" => (262144, 1),
        "screenshaver.geometry.projected_polyhedral_array_field_2d" => (4194304, 1),
        "screenshaver.geometry.analytical_sphere_projection_2d" => (4096, 1),
        "screenshaver.fields.indexed_angular_radial_superposition_field_2d" => (131072, 1),
        "screenshaver.fields.radially_modulated_filament_field_3d" => (262144, 1),
        "screenshaver.fields.bilinear_reciprocal_response_field_2d" => (4096, 1),
        "screenshaver.geometry.temporal_harmonic_segment_field_2d" => (524288, 1),
        "screenshaver.geometry.phase_modulated_orbital_grid_field_2d" => (65536, 1),
        "screenshaver.fields.scalar_band_envelope" => (2048, 1),
        "screenshaver.fields.directional_quantization_field_3d" => (2048, 1),
        "screenshaver.fields.angular_modulated_radial_band_field_2d" => (4096, 1),
        "screenshaver.fields.harmonic_point_distribution_field_2d" => (262144, 1),
        "screenshaver.color.rgb_vector_rotation" => (512, 1),
        "screenshaver.geometry.periodic_hyperspherical_shell_field" => (1024, 1),
        "screenshaver.fields.traveling_segment_pulse_field_2d" => (1024, 1),
        "screenshaver.compositing.rgb_intensity_to_alpha_decomposition" => (256, 1),
        "screenshaver.coordinates.orthographic_fisheye_ray_projection" => (1024, 1),
        "screenshaver.geometry.symmetric_diagonal_line_field_3d" => (2048, 1),
        "screenshaver.coordinates.harmonic_revolution_coordinates_3d" => (2048, 1),
        "screenshaver.coordinates.rotating_polygon_face_coordinates_2d" => (8192, 1),
        "screenshaver.math.rational_sigmoid_response" => (256, 1),        "screenshaver.animation.beat_phase_envelope" => (8, 1),
        "screenshaver.animation.cyclic_scalar_spline" => (40, 1),
        "screenshaver.animation.cyclic_three_state_blend" => (27, 1),
        "screenshaver.animation.easing_curve" => (256, 1),
        "screenshaver.animation.interpolated_procedural_sequence" => (32, 1),
        "screenshaver.animation.looping_time_controller" => (8, 1),
        "screenshaver.animation.modulated_travel_time" => (9, 1),
        "screenshaver.animation.particle_lifetime_envelope" => (16, 1),
        "screenshaver.animation.phased_step_cycle" => (38, 1),
        "screenshaver.animation.ping_pong_oscillator" => (11, 1),
        "screenshaver.animation.staggered_emission_phase" => (9, 1),
        "screenshaver.camera.look_at_camera_direction" => (18, 1),
        "screenshaver.camera.sinusoidal_path" => (8, 1),
        "screenshaver.color.alternating_3d_cell_color" => (10, 1),
        "screenshaver.color.angular_wave_background" => (18, 1),
        "screenshaver.color.cosine_palette" => (15, 1),
        "screenshaver.color.distance_hue_mapping" => (8, 1),
        "screenshaver.color.field_accumulation" => (8, 1),
        "screenshaver.color.filmic_tonemapping" => (8, 1),
        "screenshaver.color.linear_rgb_gradient" => (9, 1),
        "screenshaver.color.multi_stop_rgb_gradient" => (1168, 1),
        "screenshaver.color.primary_spectrum_palette" => (12, 1),
        "screenshaver.color.procedural_sine_rgb" => (25, 1),
        "screenshaver.color.radial_edge_hue_blend" => (9, 1),
        "screenshaver.color.rgb_channel_remap" => (8, 1),
        "screenshaver.color.rgb_power_colorizer" => (10, 1),
        "screenshaver.color.rgb_to_alpha" => (8, 1),
        "screenshaver.color.smooth_hsv_color" => (8, 1),
        "screenshaver.color.thermal_emission_palette" => (16, 1),
        "screenshaver.color.three_color_cycling_palette" => (8, 1),
        "screenshaver.color.weighted_rgb_colorizer" => (9, 1),
        "screenshaver.compositing.masked_color_blend" => (8, 1),
        "screenshaver.color.interwoven_color_compositor" => (128, 1),
        "screenshaver.compositing.rgb_color_compositor" => (14, 1),
        "screenshaver.compositing.scalar_field_compositor" => (21, 1),
        "screenshaver.compositing.screen_color_blend" => (8, 1),
        "screenshaver.coordinates.axial_rational_compression_3d" => (17, 1),
        "screenshaver.coordinates.axial_twist_and_pulse_warp" => (35, 1),
        "screenshaver.coordinates.axis_plane_projection_3d" => (8, 1),
        "screenshaver.coordinates.concentric_ring_coordinates_2d" => (33, 1),
        "screenshaver.coordinates.coordinate_feedback_warp" => (600, 1),
        "screenshaver.coordinates.coordinate_rotation_3d" => (17, 1),
        "screenshaver.coordinates.cross_axis_sine_warp" => (32768, 1),
        "screenshaver.coordinates.cylindrical_direction_projection" => (53, 1),
        "screenshaver.coordinates.deep_fbm_domain_warp" => (264, 1),
        "screenshaver.coordinates.depth_gated_axis_warp" => (11, 1),
        "screenshaver.coordinates.directional_sine_perturbation_3d" => (13, 1),
        "screenshaver.coordinates.geodesic_triangle_coordinates_3d" => (33, 1),
        "screenshaver.coordinates.geometric_sequence_coordinates" => (37, 1),
        "screenshaver.coordinates.hexagonal_grid_coordinates_2d" => (46, 1),
        "screenshaver.coordinates.icosahedral_face_coordinates_3d" => (1020, 1),
        "screenshaver.coordinates.iterative_inversion_fold_3d" => (6144, 1),
        "screenshaver.coordinates.iterative_rotational_fold_3d" => (8192, 1),
        "screenshaver.coordinates.iterative_sinusoidal_wave_fold" => (112, 1),
        "screenshaver.coordinates.kaleidoscopic_sector_fold" => (512, 1),
        "screenshaver.coordinates.iterative_coordinate_fold_2d" => (15360, 1),
        "screenshaver.coordinates.lattice_interpolation_warp_3d" => (8, 1),
        "screenshaver.coordinates.log_polar_mapping" => (8, 1),
        "screenshaver.coordinates.multi_scale_flow_warp" => (13, 1),
        "screenshaver.coordinates.organic_coordinate_mutation_3d" => (8, 1),
        "screenshaver.coordinates.periodic_mirror_tile_3d" => (8, 1),
        "screenshaver.coordinates.periodic_vector_wrap_3d" => (27, 1),
        "screenshaver.coordinates.polar_coordinate_mapping_2d" => (48, 1),
        "screenshaver.coordinates.inverse_bilinear_mapping_2d" => (2048, 1),
        "screenshaver.coordinates.radial_vortex_warp" => (8, 1),
        "screenshaver.coordinates.randomized_cell_transform_2d" => (2048, 1),
        "screenshaver.coordinates.rectangular_grid_coordinates_2d" => (30, 1),
        "screenshaver.coordinates.sinusoidal_tunnel_path_3d" => (26, 1),
        "screenshaver.coordinates.spatially_modulated_cell_displacement" => (128, 1),
        "screenshaver.coordinates.spherical_direction" => (8, 1),
        "screenshaver.coordinates.staggered_grid_coordinates_2d" => (50, 1),
        "screenshaver.coordinates.traveling_axial_coordinate_3d" => (13, 1),
        "screenshaver.coordinates.tunnel_wall_uv_mapping" => (8, 1),
        "screenshaver.coordinates.uv" => (8, 1),
        "screenshaver.fields.2nd_order_distance_extractor" => (8, 1),
        "screenshaver.fields.adaptive_procedural_ray_traversal" => (768, 96),
        "screenshaver.fields.animated_layered_particle_field" => (224, 1),
        "screenshaver.fields.animated_spatial_network_field_2d" => (44820, 1),
        "screenshaver.fields.axial_field_displacement_3d" => (13, 2),
        "screenshaver.fields.chebyshev_voronoi_edge_2d" => (4096, 1),
        "screenshaver.fields.cross_shape_field_2d" => (21, 1),
        "screenshaver.fields.directional_field_derivative_3d" => (18, 3),
        "screenshaver.fields.euclidean_voronoi_boundary_field_2d" => (32768, 1),
        "screenshaver.fields.exponential_edge_attenuation_2d" => (10, 1),
        "screenshaver.fields.fibonacci_spherical_lattice_field_3d" => (376740, 1),
        "screenshaver.fields.four_wave_plasma_field" => (8, 1),
        "screenshaver.fields.fractal_orbit_variation_field" => (992, 1),
        "screenshaver.fields.geodesic_dual_cell_field_3d" => (180, 1),
        "screenshaver.fields.gradient_noise_3d" => (248, 1),
        "screenshaver.fields.harmonic_polar_curve_field_2d" => (4096, 1),
        "screenshaver.fields.harmonic_radial_curve_accumulation" => (1472, 1),
        "screenshaver.fields.icosahedral_truncated_face_field_3d" => (18, 1),
        "screenshaver.fields.independent_floating_particle_field_2d" => (12544, 1),
        "screenshaver.fields.iterative_plasma_field" => (640, 1),
        "screenshaver.fields.iterative_radial_field" => (104, 1),
        "screenshaver.fields.iterative_spiral_field" => (576, 1),
        "screenshaver.fields.logarithmic_radial_field_3d" => (8, 1),
        "screenshaver.fields.morphing" => (288, 1),
        "screenshaver.fields.neon_filament_extractor" => (8, 1),
        "screenshaver.fields.procedural_lattice_lines" => (16, 1),
        "screenshaver.coordinates.vector_alignment_rotation_3d" => (96, 1),
        "screenshaver.lighting.rgb_beer_lambert_transmittance" => (48, 1),
        "screenshaver.lighting.dielectric_fresnel_response" => (144, 1),
        "screenshaver.fields.soft_line_distance_mask_2d" => (24, 1),
        "screenshaver.fields.procedural_radial_ray_field" => (131072, 1),
        "screenshaver.fields.procedural_spectrum_envelope" => (12, 1),
        "screenshaver.fields.propagating_radial_wavefront_2d" => (27, 1),
        "screenshaver.fields.radial_panel_field_3d" => (28, 1),
        "screenshaver.fields.randomized_bokeh_disc_field" => (192, 1),
        "screenshaver.fields.repeated_ring_accumulation" => (1024, 128),
        "screenshaver.fields.parametric_ring_accumulation_2d" => (6144, 1),
        "screenshaver.fields.stellar_ray_accumulation_3d" => (12288, 1),
        "screenshaver.fields.rotated_harmonic_field_3d" => (1824, 1),
        "screenshaver.fields.scalar_ribbon_mask" => (8, 1),
        "screenshaver.fields.scale_compensated_fractal_orbit_field_3d" => (576, 1),
        "screenshaver.fields.sdf_raymarch" => (8192, 1024),
        "screenshaver.fields.second_order_cellular_field_3d" => (65536, 1),
        "screenshaver.fields.segmented_radial_ring" => (16, 1),
        "screenshaver.fields.sinusoidal_ribbon_field" => (10, 1),
        "screenshaver.fields.soft_interference_field" => (12, 1),
        "screenshaver.fields.soft_shape_mask_2d" => (41, 1),
        "screenshaver.fields.sparse_random_cell_field_2d" => (22, 1),
        "screenshaver.fields.spherical_great_circle_field_3d" => (23, 1),
        "screenshaver.fields.spiral_wave_field_2d" => (26, 1),
        "screenshaver.fields.tapered_trail_field_2d" => (55, 1),
        "screenshaver.fields.temporal_frequency_layering_3d" => (64, 8),
        "screenshaver.fields.traversal" => (496, 16),
        "screenshaver.fields.trigonometric_interference_field_3d" => (58, 1),
        "screenshaver.fields.tunnel" => (16, 1),
        "screenshaver.fields.vignette_mask" => (8, 1),
        "screenshaver.fields.volume_density_ray_integrator" => (512, 64),
        "screenshaver.geometry.3d_cellular_tile_field" => (8, 1),
        "screenshaver.geometry.animated_polygon_sector_mask" => (39, 1),
        "screenshaver.geometry.animated_radial_shell_field" => (13, 1),
        "screenshaver.geometry.bilinear_surface_intersection" => (124, 1),
        "screenshaver.geometry.circular_arc_distance_2d" => (36, 1),
        "screenshaver.geometry.cyclic_spline_path_3d" => (49, 1),
        "screenshaver.geometry.damped_ballistic_motion_2d" => (11, 1),
        "screenshaver.geometry.domain_fold_3d" => (64, 1),
        "screenshaver.geometry.domain_repeat_3d" => (8, 1),
        "screenshaver.geometry.ellipsoid_distance_field_3d" => (8, 1),
        "screenshaver.geometry.fibonacci_sphere_point_distribution" => (13, 1),
        "screenshaver.geometry.finite_cube_cluster_field" => (1600, 1),
        "screenshaver.geometry.geometric_ray_sample_distribution" => (6400, 1),
        "screenshaver.geometry.grid_ray_step_3d" => (17, 1),
        "screenshaver.geometry.gyroid_surface_field_3d" => (14, 1),
        "screenshaver.geometry.height_field_normal_2d" => (19, 4),
        "screenshaver.geometry.menger_sponge_field_3d" => (145, 1),
        "screenshaver.geometry.multi_corridor_tunnel_field" => (13, 1),
        "screenshaver.geometry.periodic_slice_field_3d" => (11, 1),
        "screenshaver.geometry.perspective_ring_projection" => (9, 1),
        "screenshaver.geometry.point_segment_distance_2d" => (512, 1),
        "screenshaver.geometry.procedural_sphere_distribution" => (11, 1),
        "screenshaver.geometry.ray_box_intersection_3d" => (69, 1),
        "screenshaver.geometry.ray_plane_intersection" => (8, 1),
        "screenshaver.geometry.ray_sphere_intersection" => (17, 1),
        "screenshaver.geometry.ray_convex_polyhedron_intersection_3d" => (12000, 1),
        "screenshaver.geometry.hyperbolic_distance_3d" => (120, 1),
        "screenshaver.geometry.rectangular_tunnel_intersection" => (27, 1),
        "screenshaver.geometry.rotation_3d" => (8, 1),
        "screenshaver.geometry.sdf_box" => (8, 1),
        "screenshaver.geometry.sdf_capsule" => (17, 1),
        "screenshaver.geometry.sdf_combine" => (8, 1),
        "screenshaver.geometry.sdf_icosahedron" => (3040, 1),
        "screenshaver.geometry.sdf_morph" => (8, 1),
        "screenshaver.geometry.sdf_normal_estimation" => (18, 6),
        "screenshaver.geometry.sdf_octahedron" => (8, 1),
        "screenshaver.geometry.sdf_rounded_hexagonal_prism" => (204, 1),
        "screenshaver.geometry.sdf_sequential_morph" => (512, 1),
        "screenshaver.geometry.sdf_sphere" => (8, 1),
        "screenshaver.geometry.sdf_torus" => (8, 1),
        "screenshaver.geometry.sdf_triangular_prism" => (8, 1),
        "screenshaver.geometry.sdf_twisted_ring" => (8, 1),
        "screenshaver.geometry.spherical_aperture_mask" => (1024, 1),
        "screenshaver.geometry.spherical_inversion_3d" => (8, 1),
        "screenshaver.geometry.spherical_orbital_path_3d" => (55, 1),
        "screenshaver.fields.modular_cell_classification_2d" => (320, 1),
        "screenshaver.compositing.dominant_weighted_color_selector" => (192, 1),
        "screenshaver.geometry.harmonic_motion_path_2d" => (256, 1),
        "screenshaver.geometry.triangle_edge_mask" => (15, 1),
        "screenshaver.geometry.two_bone_joint_solver_3d" => (46, 1),
        "screenshaver.geometry.warped_plane_field" => (8, 1),
        "screenshaver.lighting.ambient_occlusion_shading" => (8, 1),
        "screenshaver.lighting.analytical_box_soft_shadow" => (408, 1),
        "screenshaver.lighting.beer_lambert_transmittance" => (8, 1),
        "screenshaver.lighting.curvature_shading" => (8, 1),
        "screenshaver.lighting.directional_light" => (8, 1),
        "screenshaver.lighting.distance_attenuated_point_light" => (22, 1),
        "screenshaver.lighting.phong_blinn_phong_specular_response" => (320, 1),
        "screenshaver.lighting.polynomial_distance_attenuation" => (64, 1),
        "screenshaver.geometry.ray_to_segment_distance_3d" => (768, 1),
        "screenshaver.geometry.analytical_spherical_lens_3d" => (2048, 1),
        "screenshaver.geometry.dielectric_refraction_direction_3d" => (512, 1),
        "screenshaver.noise.gradient_noise_2d" => (512, 1),
        "screenshaver.coordinates.vector_coordinate_displacement_2d" => (96, 1),
        "screenshaver.coordinates.cylindrical_cell_rotation_3d" => (640, 1),
        "screenshaver.coordinates.cylindrical_cell_coordinates_3d" => (768, 1),
        "screenshaver.noise.gradient_attenuated_fractal_noise_2d" => (20480, 1),
        "screenshaver.lighting.reciprocal_angular_light_response" => (384, 1),
        "screenshaver.color.rgb_transfer_function" => (192, 1),
        "screenshaver.compositing.normalized_weighted_rgb_blend" => (384, 1),
        "screenshaver.geometry.ray_finite_cylinder_intersection_3d" => (4096, 1),
        "screenshaver.geometry.three_point_surface_normal_3d" => (768, 1),
        "screenshaver.coordinates.mobius_coordinate_transformation_2d" => (1536, 1),
        "screenshaver.coordinates.logarithmic_spiral_coordinate_mapping_2d" => (1536, 1),
        "screenshaver.fields.anisotropic_radial_glow_field_2d" => (512, 1),
        "screenshaver.coordinates.stereographic_sphere_projection" => (1536, 1),
        "screenshaver.coordinates.hyperbolic_mobius_coordinate_transformation_3d" => (2048, 1),
        "screenshaver.coordinates.radial_power_warp_2d" => (4096, 1),
        "screenshaver.coordinates.toroidal_coordinate_mapping_3d" => (1024, 1),
        "screenshaver.coordinates.modulated_periodic_coordinates_2d" => (512, 1),
        "screenshaver.fields.animated_metaball_field_2d" => (65536, 1),
        "screenshaver.fields.periodic_scalar_distance_field" => (320, 1),
        "screenshaver.noise.rotating_gradient_noise_2d" => (2048, 1),
        "screenshaver.geometry.point_to_ray_distance_3d" => (256, 1),
        "screenshaver.lighting.distance_fog" => (8, 1),
        "screenshaver.lighting.distance_glow_falloff" => (12, 1),
        "screenshaver.lighting.fresnel_rim" => (8, 1),
        "screenshaver.lighting.henyey_greenstein_scattering" => (27, 1),
        "screenshaver.lighting.lattice_crossing_highlight" => (8, 1),
        "screenshaver.lighting.layered_refractive_boundary_traversal" => (2048, 256),
        "screenshaver.lighting.luminous_edge_extraction" => (8, 1),
        "screenshaver.lighting.procedural_environment_reflection" => (16, 1),
        "screenshaver.lighting.radial_core_glow" => (8, 1),
        "screenshaver.lighting.ray_glow" => (8, 1),
        "screenshaver.lighting.refractive_surface" => (3840, 480),
        "screenshaver.lighting.secondary_sdf_reflection_trace" => (512, 64),
        "screenshaver.lighting.spherical_surface_shading_2d" => (23, 1),
        "screenshaver.math.animated_vertical_oscillator" => (8, 1),
        "screenshaver.math.cosine_range_oscillator" => (8, 1),
        "screenshaver.math.coupled_trigonometric_field_3d" => (24, 1),
        "screenshaver.math.geometric_norm_blend_3d" => (22, 1),
        "screenshaver.math.multiply" => (8, 1),
        "screenshaver.math.periodic_pulse_gate" => (8, 1),
        "screenshaver.math.rotation2d" => (8, 1),
        "screenshaver.math.scalar_response_shaping" => (512, 1),
        "screenshaver.math.logarithmic_exponential_response" => (192, 1),
        "screenshaver.math.cell_indexed_vector_modulation_3d" => (384, 1),
        "screenshaver.fields.repeated_harmonic_line_field_2d" => (262144, 1),
        "screenshaver.coordinates.cyclic_exponential_coordinate_scaling_2d" => (512, 1),
        "screenshaver.geometry.euclidean_box_distance_field_3d" => (512, 1),
        "screenshaver.coordinates.symmetric_primitive_array_coordinates_3d" => (2048, 1),
        "screenshaver.geometry.triangle_boundary_distance_field_2d" => (4096, 1),
        "screenshaver.coordinates.crystallographic_symmetry_coordinates_2d" => (8192, 1),
        "screenshaver.coordinates.circle_inversion_coordinates_2d" => (2048, 1),
        "screenshaver.noise.scalar_value_noise_1d" => (768, 1),
        "screenshaver.geometry.convex_polygon_distance_field_2d" => (65536, 1),
        "screenshaver.lighting.angular_difference_response" => (1024, 1),
        "screenshaver.coordinates.perspective_plane_coordinates_2d" => (1024, 1),
        "screenshaver.fields.depth_dependent_tapered_mask_2d" => (1024, 1),
        "screenshaver.geometry.toothed_radial_distance_field_2d" => (2048, 1),
        "screenshaver.coordinates.radial_normalization_warp_2d" => (1024, 1),
        "screenshaver.fields.super_gaussian_attenuation_field_2d" => (1024, 1),
        "screenshaver.geometry.circular_rounded_csg_operator" => (512, 1),
        "screenshaver.geometry.three_point_oriented_plane_field_3d" => (1024, 1),
        "screenshaver.fields.iterative_inversion_orbit_field_2d" => (262144, 1),
        "screenshaver.coordinates.independently_scrolling_cell_coordinates_2d" => (1536, 1),
        "screenshaver.coordinates.arc_length_radial_cell_coordinates_3d" => (2048, 1),
        "screenshaver.geometry.rounded_box_distance_field_3d" => (1024, 1),
        "screenshaver.color.parametric_harmonic_rgb_palette" => (768, 1),
        "screenshaver.geometry.finite_capped_cylinder_distance_field_3d" => (512, 1),
        "screenshaver.geometry.finite_cone_distance_field_3d" => (2048, 1),
        "screenshaver.fields.rotating_angular_sweep_field_2d" => (2048, 1),
        "screenshaver.fields.directional_wave_superposition_field_2d" => (32768, 1),
        "screenshaver.compositing.saturation_preserving_rgb_interpolation" => (1024, 1),
        "screenshaver.fields.multiscale_gyroid_density_field_3d" => (65536, 1),
        "screenshaver.coordinates.oblique_periodic_lattice_coordinates_2d" => (1024, 1),
        "screenshaver.noise.configurable_lattice_value_noise_3d" => (8192, 1),
        "screenshaver.coordinates.iterative_inverse_tangent_fold_2d" => (32768, 1),
        "screenshaver.coordinates.multiscale_cyclic_vector_warp_3d" => (16384, 1),
        "screenshaver.color.hyperbolic_tangent_rgb_response" => (512, 1),
        "screenshaver.coordinates.triplanar_projection_coordinates_3d" => (2048, 1),
        "screenshaver.math.threshold_field_remapping" => (512, 1),
        "screenshaver.coordinates.polygonal_polar_coordinates_2d" => (2048, 1),
        "screenshaver.fields.radial_density_envelope_2d" => (1024, 1),
        "screenshaver.coordinates.cylindrical_surface_coordinates_3d" => (2048, 1),
        "screenshaver.fields.periodic_cellular_distance_field_2d" => (524288, 1),
        "screenshaver.fields.periodic_vortex_vector_field_2d" => (2048, 1),
        "screenshaver.noise.flow_modulated_fractal_noise_2d" => (65536, 1),
        "screenshaver.noise.turbulent_ridged_fractal_noise_2d" => (16384, 1),
        "screenshaver.coordinates.multi_harmonic_coordinate_displacement_2d" => (4096, 1),
        "screenshaver.fields.tapered_gaussian_ribbon_field_2d" => (1536, 1),
        "screenshaver.lighting.multi_lobe_angular_response" => (1024, 1),
        "screenshaver.fields.multioctave_directional_wave_height_field_2d" => (32768, 1),
        "screenshaver.math.spatial_phase_oscillator" => (18, 1),
        "screenshaver.math.vector_magnitude_2d" => (8, 1),
        "screenshaver.noise.dot3d" => (17, 1),
        "screenshaver.noise.fbm_2d" => (8192, 1),
        "screenshaver.noise.fbm_3d" => (2048, 1),
        "screenshaver.noise.layered_temporal_noise" => (8192, 1),
        "screenshaver.noise.triangle3d" => (8, 1),
        "screenshaver.noise.triangular_lattice_field" => (26, 1),
        "screenshaver.noise.value2d" => (13, 1),
        "screenshaver.output.color" => (8, 1),
        "screenshaver.patterns.hexagonal_grid_pattern" => (96, 1),
        "screenshaver.test.universal_test" => (25, 8),
        _ => return None,
    })
}
// Provisional split of the existing 15,360-unit maximum. Keep a fixed
// allowance and preserve the old maximum exactly at 24 iterations.
fn fold_iteration_work(value:Option<f64>)->u64 {
    let count=value.map(|v|v as f32).filter(|v|v.is_finite())
        .map(|v|(v+0.5).floor().clamp(1.0,24.0) as u64).unwrap_or(24);
    768+608*count
}
fn node_safety_profile(n:&crate::maker_graph::Node)->Option<(u64,u64)> {
    let (local,multiplier)=safety_profile(&n.module_id)?;
    let local=if n.module_id=="screenshaver.coordinates.iterative_coordinate_fold_2d" {
        fold_iteration_work(n.parameters.get("iterations").and_then(|v|v.as_f64()))
    }else{local};
    Some((local,multiplier))
}
// Walk once in dependency order, then propagate evaluation multiplicities
// from the terminal. Ordinary graphs share main() locals. For any callable
// or amplified graph retain the historical per-edge conservative accounting.
pub(crate) fn safety_work(graph:&Graph)->Result<(u64,Vec<(NodeId,u64,u64)>),String> {
    safety_work_mode(graph,false)
}
// Only live preview uses the audited edge correction. Worker admission stays
// conservative until separate discovery regression testing is completed.
fn safety_work_mode(graph:&Graph,live:bool)->Result<(u64,Vec<(NodeId,u64,u64)>),String> {
    fn visit(graph:&Graph,id:NodeId,seen:&mut std::collections::HashSet<NodeId>,
        active:&mut std::collections::HashSet<NodeId>,order:&mut Vec<NodeId>)->Result<(),String>{
        if seen.contains(&id){return Ok(())}
        if !active.insert(id){return Err("Rendering deferred: cyclic safety dependency".into())}
        if graph.node(id).is_none(){return Err("Rendering deferred: missing safety dependency".into())}
        for c in graph.connections.iter().filter(|c|c.to==id){visit(graph,c.from,seen,active,order)?;}
        active.remove(&id);seen.insert(id);order.push(id);Ok(())
    }
    let out=graph.nodes.iter().find(|n|n.definition().terminal).ok_or("Missing output")?.id;
    let mut order=Vec::new();
    visit(graph,out,&mut std::collections::HashSet::new(),&mut std::collections::HashSet::new(),&mut order)?;
    let mut profiles=HashMap::new();
    let mut amplified=false;
    for &id in &order {
        let n=graph.node(id).ok_or("Missing safety dependency")?;
        let profile=node_safety_profile(n).ok_or_else(||format!("Rendering deferred: unknown safety profile for {} (node {})",n.display_name(),id))?;
        amplified |= profile.1!=1 || n.definition().inputs.iter().any(|p|p.ty.starts_with("field"));
        profiles.insert(id,profile);
    }
    let mut counts=HashMap::<NodeId,u64>::new();counts.insert(out,1);
    let mut total=0u64;let mut details=Vec::new();
    for &id in order.iter().rev(){
        let (local,multiplier)=profiles[&id];
        let count=counts.get(&id).copied().unwrap_or(0);
        let contribution=local.saturating_mul(count);
        total=total.saturating_add(contribution);details.push((id,count,contribution));
        for c in graph.connections.iter().filter(|c|c.to==id){
            let entry=counts.entry(c.from).or_insert(0);
            // Field Traversal's position is a value argument, evaluated once
            // per invocation. Its seed remains sampled inside the 16-step loop.
            // Other consumers retain their existing conservative multipliers.
            let edge_multiplier=if live && graph.node(id).is_some_and(|n|
                n.module_id=="screenshaver.fields.traversal") && c.to_port==1 {1}else{multiplier};
            *entry=if amplified {entry.saturating_add(count.saturating_mul(edge_multiplier))}else{1};
        }
    }
    details.sort_by(|a,b|b.2.cmp(&a.2).then_with(||a.0.cmp(&b.0)));
    Ok((total,details))
}
// Provisional conservative threshold; calibration requires real GPU measurements.
const MAX_FRAME_WORK:u64=500_000_000;
// Expanded live-preview allowance is limited to known ordinary, non-amplified
// official modules. Discovery and callable graphs retain the original ceiling.
pub(crate) fn adaptive_preview_work(graph:&Graph)->Result<(u64,u64),String>{
    graph.validate_renderable()?;
    if graph_source_cost(graph)>MAX_COMPILE_COST{return Err("Rendering deferred: source complexity budget".into())}
    let (work,_)=safety_work_mode(graph,true)?;
    let reachable=graph.output_reachable_ids();
    let ordinary=graph.nodes.iter().filter(|n|reachable.contains(&n.id)).all(|n|
        n.module_id.starts_with("screenshaver.") && n.definition().category!="Test" &&
        !n.definition().inputs.iter().any(|p|p.ty.starts_with("field")) &&
        safety_profile(&n.module_id).is_some_and(|(_,m)|m==1));
    Ok((work,if ordinary{5_000_000_000}else{MAX_FRAME_WORK}))
}
pub(crate) fn rendering_admission(graph:&Graph,w:i32,h:i32)->Result<(),String>{
    rendering_admission_impl(graph,w,h,MAX_FRAME_WORK,false)
}
pub(crate) fn rendering_admission_with_limit(graph:&Graph,w:i32,h:i32,limit:u64)->Result<(),String>{
    rendering_admission_impl(graph,w,h,limit,true)
}
fn rendering_admission_impl(graph:&Graph,w:i32,h:i32,limit:u64,live:bool)->Result<(),String>{
    graph.validate_renderable()?;
    if w<=0 || h<=0{return Err("Preview dimensions unavailable".into())}
    if graph_source_cost(graph)>MAX_COMPILE_COST{return Err("Rendering deferred: source complexity budget".into())}
    let (work,details)=safety_work_mode(graph,live)?;
    let frame=work.saturating_mul(w as u64).saturating_mul(h as u64);
    if frame>limit{
        let breakdown=details.iter().take(4).map(|(id,count,cost)|{
            let name=graph.node(*id).map(|n|n.display_name()).unwrap_or("Unknown");
            format!("{name} #{id}: {cost} (x{count})")
        }).collect::<Vec<_>>().join("; ");
        return Err(format!("Rendering deferred: estimated frame work {frame} exceeds {limit} ({w}x{h}; {work} units/pixel); top contributors: {breakdown}; reduce complexity or preview size"))
    }
    Ok(())
}

pub(crate) fn candidate_admission(graph:&Graph)->Result<(),String> {
    rendering_admission(graph,192,108)?;
    graph.validate_renderable()?;
    if graph.output_reachable_ids().iter().any(|id| graph.node(*id).is_some_and(|n| n.definition().category.eq_ignore_ascii_case("Test"))) { return Err("Test modules are excluded from shader candidate enumeration".into()); }
    let source_cost=graph_source_cost(graph);
    if source_cost>MAX_COMPILE_COST {
        return Err(format!("Source complexity deferred: estimated cost {source_cost} exceeds {MAX_COMPILE_COST}"));
    }
    let runtime_work=graph_compile_cost(graph);
    if runtime_work>MAX_ESTIMATED_RUNTIME_WORK {
        return Err(format!("Runtime complexity deferred: estimated work {runtime_work} exceeds {MAX_ESTIMATED_RUNTIME_WORK}"));
    }
    Ok(())
}


fn graph_preflight(graph:&Graph, _output:NodeId)->Result<(),String> {
    graph.validate_renderable()
}

// Bound graph snapshots independently in discovery.
const MAX_PENDING_RENDER_JOBS: usize = 96;
fn candidate_graphs(graph:&Graph)->Vec<(String,Graph)> {
    if graph.nodes.iter().any(|n|n.unresolved()){return Vec::new()}
    if graph.nodes.iter().any(|n| n.definition().category.eq_ignore_ascii_case("Test")) { return Vec::new(); }
    const MAX_CANDIDATES:usize=1_000;
    // Deterministic mixed-radix enumeration. Each numeric slider has exactly
    // three sample positions: minimum, arithmetic midpoint, maximum.
    // Equal values (degenerate ranges) are deduplicated.
    // Speed is intentionally excluded from candidate discovery.
    let base=graph.clone();
    let optical_id=base.nodes.iter().find(|n|n.module_id=="screenshaver.lighting.layered_refractive_boundary_traversal"
        && base.output_reachable_ids().contains(&n.id)).map(|n|n.id);
    // A single bounded optical axis avoids a 3^N combinatorial explosion.
    // Each preset is an actual graph parameter set, rendered by the normal GPU worker.
    let optical_presets:[(&str,f64,f64,f64,[f64;3]);8]=[
        ("neutral",1.45,0.15,2.0,[1.0,1.0,1.0]),
        ("clear",1.15,0.0,2.0,[1.0,1.0,1.0]),
        ("glass",1.45,0.15,4.0,[0.65,0.85,1.0]),
        ("strong",1.8,0.15,4.0,[1.0,1.0,1.0]),
        ("blue",1.45,0.6,4.0,[0.65,0.85,1.0]),
        ("amber",1.45,0.6,4.0,[1.0,0.78,0.45]),
        ("dense",1.8,0.6,2.0,[0.65,0.85,1.0]),
        ("subtle",1.15,0.15,4.0,[1.0,0.78,0.45]),
    ];
    let mut dimensions:Vec<(NodeId,String,String,Vec<Value>)>=Vec::new();
    if let Some(id)=optical_id {
        dimensions.push((id,"__optical_preset".into(),"Optical preset".into(),
            (0..optical_presets.len()).map(|i|serde_json::json!(i)).collect()));
    }
    for node in &base.nodes {
        for param in &node.definition().parameters {
            if is_speed_parameter(&param.id) {continue;}
            if Some(node.id)==optical_id && matches!(param.id.as_str(),"ior"|"absorption"|"tint"|"max_boundaries") {continue;}
            let values:Vec<Value>=match param.ty.as_str() {
                "float" => {
                    let lo=param.min as f64;
                    let hi=param.max as f64;
                    let mid=lo+(hi-lo)*0.5;
                    let mut samples=Vec::new();
                    for v in [lo,mid,hi] {
                        if v.is_finite() && !samples.iter().any(|x:&Value|x.as_f64()==Some(v)) {
                            samples.push(serde_json::json!(v));
                        }
                    }
                    samples
                }
                "choice" => (0..param.options.len()).map(|i|serde_json::json!(i as f64)).collect(),
                _ => Vec::new(),
            };
            if values.len()>1 {
                dimensions.push((node.id,param.id.clone(),param.name.clone(),values));
            }
        }
    }
    let mut result=Vec::new();
    let mut digits=vec![0usize;dimensions.len()];
    loop {
        let mut proposed=base.clone();
        let mut labels=Vec::new();
        for ((id,param_id,param_name,values),digit) in dimensions.iter().zip(&digits) {
            let value=&values[*digit];
            if param_id=="__optical_preset" {
                let index=value.as_u64().unwrap_or(0) as usize;
                let (name,ior,absorption,boundaries,tint)=optical_presets[index];
                if let Some(node)=proposed.node_mut(*id){
                    node.parameters.insert("ior".into(),serde_json::json!(ior));
                    node.parameters.insert("absorption".into(),serde_json::json!(absorption));
                    node.parameters.insert("max_boundaries".into(),serde_json::json!(boundaries));
                    node.parameters.insert("tint".into(),serde_json::json!(tint));
                }
                labels.push(format!("Optical #{}={}",id,name));
                continue;
            }
            if let Some(node)=proposed.node_mut(*id) {
                node.parameters.insert(param_id.clone(),value.clone());
            }
            let shown=if value.is_number(){format!("{:.3}",value.as_f64().unwrap_or(0.0))}else{value.to_string()};
            labels.push(format!("{} #{}={}",param_name,id,shown));
        }
        proposed.revision=graph.revision.wrapping_add(1);
        result.push((if labels.is_empty(){"Current parameters".to_owned()}else{labels.join(", ")},proposed));
        if result.len()>=MAX_CANDIDATES || dimensions.is_empty(){break;}
        let mut carry=true;
        for (digit,(_,_,_,values)) in digits.iter_mut().zip(&dimensions) {
            *digit+=1;
            if *digit<values.len(){carry=false;break;}
            *digit=0;
        }
        if carry{break;}
    }
    result
}
/// Six separately timed stages. CPU stages never call GLSL or OpenGL.
const DEFAULT_MAX_ENUMERATIONS:usize=50_000;
const CANDIDATES_EVALUATED:usize=1_000;
const CANDIDATES_RETAINED:usize=500;
const MAX_RAW_GRAPHS:usize=12_000;
const CPU_STAGE_BATCH:usize=256;

#[derive(Clone,Copy,PartialEq,Eq)]
enum DiscoveryStage {Feasibility,Enumerate,Validate,Deduplicate,Qualify,Ready}
impl DiscoveryStage {
    fn name(self)->&'static str {match self {Self::Feasibility=>"0 Connection feasibility",Self::Enumerate=>"1 Enumeration",Self::Validate=>"2 Structural validation",Self::Deduplicate=>"3 Deduplication",Self::Qualify=>"4 Static qualification",Self::Ready=>"CPU stages complete"}}
}
fn discovery_cpu_seconds()->f64 {
    #[cfg(unix)] unsafe {
        let mut usage:libc::rusage=std::mem::zeroed();
        if libc::getrusage(libc::RUSAGE_SELF,&mut usage)==0 {
            return usage.ru_utime.tv_sec as f64+usage.ru_utime.tv_usec as f64/1e6
                +usage.ru_stime.tv_sec as f64+usage.ru_stime.tv_usec as f64/1e6;
        }
    }
    0.0
}
fn discovery_rss_kib()->usize {
    #[cfg(target_os="linux")]
    if let Ok(status)=std::fs::read_to_string("/proc/self/status") {
        if let Some(line)=status.lines().find(|line|line.starts_with("VmRSS:")) {
            return line.split_whitespace().nth(1).and_then(|v|v.parse().ok()).unwrap_or(0);
        }
    }
    0
}
fn discovery_threads()->usize {
    let available=std::thread::available_parallelism().map(|n|n.get()).unwrap_or(1);
    std::env::var("SCREENSHAVER_MAKER_CPU_WORKERS").ok()
        .and_then(|v|v.parse::<usize>().ok()).unwrap_or((available/2).max(1)).clamp(1,available)
}
fn parallel_map<T:Sync,R:Send,F:Fn(&T)->R+Sync>(items:&[T],workers:usize,f:F)->Vec<R>{
    if items.is_empty(){return Vec::new()}
    let count=workers.min(items.len());
    if count==1{return items.iter().map(f).collect()}
    let chunk=items.len().div_ceil(count);
    std::thread::scope(|scope|{
        let f=&f;
        let handles:Vec<_>=items.chunks(chunk).map(|part|scope.spawn(move ||part.iter().map(f).collect::<Vec<R>>())).collect();
        let mut output=Vec::with_capacity(items.len());
        for handle in handles {output.extend(handle.join().expect("Maker CPU qualification worker panicked"));}
        output
    })
}
fn connection_key(graph:&Graph)->Vec<(NodeId,usize,NodeId,usize)> {
    let mut key:Vec<_>=graph.connections.iter().map(|c|(c.from,c.from_port,c.to,c.to_port)).collect();
    key.sort_unstable();key
}
// Semantic quality is a downstream ranking preference, NOT a graph validity rule.
// No work is added to the canonical-parent traversal or its checkpoint cursors.
fn semantic_pair_penalty(from:&str,to:&str)->u32 {
    if from==to && !from.is_empty() {return 0}
    if from.is_empty() || to.is_empty() {return 2}
    // Coordinate transforms and mask/intensity conversions are useful artistic connections.
    if matches!((from,to),("uv","position2d")|("position2d","uv")
        |("offset2d","position2d")|("position2d","offset2d")
        |("direction2d","position2d")|("position2d","direction2d")
        |("position3d","direction3d")|("direction3d","position3d")
        |("mask","intensity")|("intensity","mask")
        |("distance","curvature")|("curvature","distance")
        |("occlusion","mask")|("mask","occlusion")) {return 1}
    // Normal/color and position/color mismatches are especially likely to be unproductive.
    if (from=="color" && matches!(to,"normal3d"|"position3d"|"direction3d"))
        || (to=="color" && matches!(from,"normal3d"|"position3d"|"direction3d")) {return 5}
    3
}
fn graph_semantic_penalty(graph:&Graph)->u32 {
    graph.connections.iter().fold(0u32,|score,c| {
        let Some(source)=graph.node(c.from) else {return score};
        let Some(target)=graph.node(c.to) else {return score};
        let Some(from)=source.output_port(c.from_port) else {return score};
        let Some(to)=target.input_port(c.to_port) else {return score};
        score.saturating_add(semantic_pair_penalty(&from.semantic_role,&to.semantic_role))
    })
}

// Discovery preference only: a normal evaluated at the default origin is the
// same normal for every pixel. This is legal GLSL, but a poor callable thumbnail.
// Do not alter graph validity, module defaults, or manually authored graphs.
fn callable_sampling_penalty(graph:&Graph)->u32 {
    if !graph_has_callable_edge(graph) {return 0;}
    let reachable=graph.output_reachable_ids();
    let mut penalty=0u32;
    for node in &graph.nodes {
        if !reachable.contains(&node.id) {continue;}
        if node.module_id!="screenshaver.geometry.sdf_normal_estimation" {continue;}
        let definition=node.definition();
        let field=definition.inputs.iter().position(|p|p.ty=="field3d");
        let position=definition.inputs.iter().position(|p|p.ty=="vec3" &&
            (p.semantic_role=="position3d" || p.id=="position"));
        if let (Some(field),Some(position))=(field,position) {
            if graph.input_connection(node.id,field).is_some() &&
                graph.input_connection(node.id,position).is_none() {
                // Strong preference for spatially supplied normals, not a rejection.
                penalty=penalty.saturating_add(40);
            }
        }
    }
    penalty
}

// A deliberately conservative structural hint, not a proof of pixel motion.
// Only a time-dependent module on the reachable output path counts; declaring
// u_time in the shader header alone is not evidence of animation.
fn graph_time_dependency_hint(graph:&Graph)->bool {
    let reachable=graph.output_reachable_ids();
    graph.nodes.iter().any(|n|reachable.contains(&n.id) && {
        let m=n.definition();
        m.glsl_outputs.iter().any(|expression|expression.contains("u_time") || expression.contains("{time}")) ||
        m.parameters.iter().any(|p|p.id=="speed" &&
            m.glsl_outputs.iter().any(|expression|expression.contains("{speed}"))) ||
        matches!(n.module_id.as_str(),"screenshaver.geometry.rotation_3d"|
            "screenshaver.fields.sdf_raymarch"|"screenshaver.fields.traversal")
    })
}

fn graph_discovery_penalty(graph:&Graph)->u32 {
    let base=graph_semantic_penalty(graph).saturating_add(callable_sampling_penalty(graph));
    // This small tie preference never invalidates static shaders.
    if graph_has_callable_edge(graph) && !graph_time_dependency_hint(graph) {
        base.saturating_add(2)
    } else {base}
}

// Stable, connection-count-neutral priority for GPU admission. The hash
// uses connection identities, never the number of connections or a complexity
// score. It makes selection reproducible across runs and worker counts.
fn discovery_priority(graph:&Graph)->u64 {
    let mut h=0xcbf29ce484222325u64;
    for (from,from_port,to,to_port) in connection_key(graph) {
        for value in [from,from_port as u64,to,to_port as u64] {
            for byte in value.to_le_bytes() {
                h=(h^(byte as u64)).wrapping_mul(0x100000001b3);
            }
        }
    }
    // Avalanche to remove ordering patterns from low hash bits.
    h^=h>>30;h=h.wrapping_mul(0xbf58476d1ce4e5b9);
    h^=h>>27;h=h.wrapping_mul(0x94d049bb133111eb);h^(h>>31)
}

// Stable composition fingerprint for temporary candidate project names. This is
// deliberately separate from discovery_priority: discovery ranking must not change.
// The fingerprint excludes editor positions, expansion flags and revision counters.
fn candidate_composition_fingerprint(graph:&Graph)->String {
    let mut nodes:Vec<_>=graph.nodes.iter().map(|n|{
        (n.id,n.module_id.as_str(),&n.parameters)
    }).collect();
    nodes.sort_by_key(|n|n.0);
    let mut edges:Vec<_>=graph.connections.iter().map(|c|{
        (c.from,c.from_port,c.to,c.to_port)
    }).collect();
    edges.sort_unstable();
    let payload=serde_json::to_vec(&(nodes,edges)).unwrap_or_default();
    let mut hash=0xcbf29ce484222325u64;
    for byte in payload {hash=(hash^(byte as u64)).wrapping_mul(0x100000001b3);}
    // 64-bit avalanche, identical to the existing discovery priority finalizer.
    hash^=hash>>30;hash=hash.wrapping_mul(0xbf58476d1ce4e5b9);
    hash^=hash>>27;hash=hash.wrapping_mul(0x94d049bb133111eb);
    format!("{:016X}",hash^(hash>>31))
}

// Downstream-only graph diversity: compare complete typed edge identities.
// Never used by the canonical enumerator or its checkpoint cursor.
fn shared_connection_count(a:&[(NodeId,usize,NodeId,usize)],b:&[(NodeId,usize,NodeId,usize)])->usize {
    let (mut i,mut j,mut shared)=(0,0,0);
    while i<a.len() && j<b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less=>i+=1,
            std::cmp::Ordering::Greater=>j+=1,
            std::cmp::Ordering::Equal=>{shared+=1;i+=1;j+=1;}
        }
    }
    shared
}

// Pick from a bounded, semantically ranked shortlist. A repeated edge costs
// two penalty units; semantic compatibility remains a preference, not a rule.
fn select_diverse_gpu_indices(graphs:&[CompactCandidate],ranked:&[(u32,u64,usize)],limit:usize)->Vec<usize>{
    const DIVERSITY_POOL:usize=1024;
    let pool=&ranked[..ranked.len().min(DIVERSITY_POOL)];
    let keys:Vec<_>=pool.iter().map(|(_,_,i)|graphs[*i].key()).collect();
    let mut chosen=Vec::<usize>::new();
    let mut used=vec![false;pool.len()];
    for _ in 0..limit.min(pool.len()) {
        let mut best:Option<(u32,u64,usize,usize)>=None;
        for (position,&(semantic,hash,index)) in pool.iter().enumerate(){
            if used[position]{continue}
            let overlap=chosen.iter().map(|&selected|shared_connection_count(&keys[position],&keys[selected])).max().unwrap_or(0);
            let penalty=semantic.saturating_add((overlap as u32).saturating_mul(2));
            let candidate=(penalty,hash,index,position);
            if best.is_none_or(|previous|candidate<previous){best=Some(candidate);}
        }
        if let Some((_,_,_,position))=best {used[position]=true;chosen.push(position);}
    }
    chosen.into_iter().map(|position|pool[position].2).collect()
}

// A callable is counted only when a real graph edge uses a field-typed port.
fn graph_has_callable_edge(graph:&Graph)->bool {
    graph.connections.iter().any(|c| {
        graph.node(c.from).and_then(|n|n.output_port(c.from_port))
            .is_some_and(|p|p.ty=="field2d" || p.ty=="field3d")
    })
}

// Deterministic reservoir admission: every output-connected graph encountered
// has equal probability of occupying one of the bounded raw slots. This avoids
// keeping only the first 12,000 states of a depth-first traversal.
fn reservoir_slot(ordinal:u64,capacity:usize)->Option<usize>{
    if ordinal<=capacity as u64 {return Some((ordinal-1) as usize)}
    let mut x=ordinal^0x9e3779b97f4a7c15;
    x^=x>>30;x=x.wrapping_mul(0xbf58476d1ce4e5b9);
    x^=x>>27;x=x.wrapping_mul(0x94d049bb133111eb);x^=x>>31;
    let index=((x as u128*ordinal as u128)>>64) as u64;
    (index<capacity as u64).then_some(index as usize)
}

/// Stage 0: immutable port-compatibility index for this exact node inventory.
/// Potential reachability is intentionally optimistic: dynamic cycles are checked
/// during enumeration, and no partially wired graph is rejected as incomplete.
struct ConnectionFeasibility {
    sources:std::collections::BTreeMap<(NodeId,usize),Vec<(NodeId,usize)>>,
    raw_pairs:usize,
    typed_pairs:usize,
    feasible_pairs:usize,
    reachable_nodes:usize,
    excluded_nodes:usize,
}
impl ConnectionFeasibility {
    fn build(graph:&Graph,output:NodeId)->Self {
        let mut index=Self{sources:Default::default(),raw_pairs:0,typed_pairs:0,
            feasible_pairs:0,reachable_nodes:0,excluded_nodes:0};
        let mut typed=std::collections::BTreeMap::<(NodeId,usize),Vec<(NodeId,usize)>>::new();
        for target in &graph.nodes {
            for tp in 0..target.input_count(){
                let Some(input)=target.input_port(tp) else {continue};
                let mut sources=Vec::new();
                for source in &graph.nodes {
                    if source.definition().terminal || source.id==target.id {continue}
                    for sp in 0..source.output_count(){
                        let Some(out)=source.output_port(sp) else {continue};
                        index.raw_pairs+=1;
                        if out.ty==input.ty {sources.push((source.id,sp));index.typed_pairs+=1;}
                    }
                }
                typed.insert((target.id,tp),sources);
            }
        }
        // A fixed-point over possible predecessors, not currently connected edges.
        // Only modules potentially feeding the terminal can contribute to a result.
        let mut reachable=std::collections::HashSet::from([output]);
        loop {
            let mut changed=false;
            for (&(to,_),sources) in &typed {
                if !reachable.contains(&to){continue}
                for &(from,_) in sources {changed|=reachable.insert(from);}
            }
            if !changed{break}
        }
        index.reachable_nodes=reachable.len();
        index.excluded_nodes=graph.nodes.len().saturating_sub(reachable.len());
        for (target,sources) in typed {
            if !reachable.contains(&target.0){continue}
            let sources:Vec<_>=sources.into_iter().filter(|(from,_)|reachable.contains(from)).collect();
            index.feasible_pairs+=sources.len();
            index.sources.insert(target,sources);
        }
        index
    }
}
#[derive(Clone)]
struct CompactCandidate {
    connections: Vec<crate::maker_graph::Connection>,
    revision: u64,
    callable: bool,
}
impl CompactCandidate {
    fn capture(graph: Graph) -> Self {
        let callable=graph_has_callable_edge(&graph);
        Self{connections:graph.connections,revision:graph.revision,callable}
    }
    fn materialize(&self,base:&Graph)->Graph {
        let mut graph=base.clone();
        graph.connections=self.connections.clone();graph.revision=self.revision;graph
    }
    fn key(&self)->Vec<(NodeId,usize,NodeId,usize)> {
        let mut key:Vec<_>=self.connections.iter().map(|c|(c.from,c.from_port,c.to,c.to_port)).collect();
        key.sort_unstable();key
    }
}
struct ConnectionSearch {
    base:Option<Graph>,
    canonical:Option<maker_discovery::ProductionEnumeration>,
    enumeration_start:u64,
    max_enumerations:usize,
    min_wire_connections:usize,
    max_wire_connections:usize,
    discovery_error:Option<String>,
    seen:std::collections::HashSet<Vec<(NodeId,usize,NodeId,usize)>>,
    raw:Vec<CompactCandidate>,
    callable_raw:Vec<CompactCandidate>,
    callable_seen:u64,
    callable_qualified:usize,
    callable_admissible:usize,
    qualified:Vec<CompactCandidate>,
    scores:Vec<(u32,u64,usize)>,
    raw_seen:u64,
    selected_jobs:Vec<CandidateJob>,
    gpu_started:bool,
    output:NodeId,
    examined:usize,
    generated:usize,
    incomplete:usize,
    duplicates:usize,
    stage:DiscoveryStage,
    cursor:usize,
    workers:usize,
    started:Instant,
    cpu_started:f64,
    reports:Vec<String>,
    peak_rss_kib:usize,
    deferred_complexity:usize,
    min_runtime_cost:u64,
    max_runtime_cost:u64,
    min_source_cost:u64,
    max_source_cost:u64,
}
impl ConnectionSearch {
    fn new(graph:&Graph,max_enumerations:usize,min_wire_connections:usize,max_wire_connections:usize)->Option<Self>{
        let output=graph.nodes.iter().find(|n|n.definition().terminal)?.id;
        let mut base=graph.clone();
        let test_ids:std::collections::HashSet<NodeId>=base.nodes.iter()
            .filter(|n|n.definition().category.eq_ignore_ascii_case("Test"))
            .map(|n|n.id).collect();
        base.nodes.retain(|n|!test_ids.contains(&n.id));
        base.connections.retain(|c|!test_ids.contains(&c.from)&&!test_ids.contains(&c.to));
        base.connections.clear();base.revision=graph.revision.wrapping_add(1);
        Some(Self{base:Some(base),canonical:None,enumeration_start:0,max_enumerations,min_wire_connections,max_wire_connections,discovery_error:None,seen:Default::default(),raw:Vec::new(),callable_raw:Vec::new(),callable_seen:0,callable_qualified:0,callable_admissible:0,qualified:Vec::new(),scores:Vec::new(),raw_seen:0,selected_jobs:Vec::new(),gpu_started:false,output,
            examined:0,generated:0,incomplete:0,duplicates:0,stage:DiscoveryStage::Feasibility,
            cursor:0,workers:discovery_threads(),started:Instant::now(),cpu_started:discovery_cpu_seconds(),
            reports:Vec::new(),peak_rss_kib:discovery_rss_kib(),deferred_complexity:0,min_runtime_cost:u64::MAX,max_runtime_cost:0,min_source_cost:u64::MAX,max_source_cost:0})
    }
    fn finished(&self)->bool{self.stage==DiscoveryStage::Ready}
    fn transition(&mut self,next:DiscoveryStage,input:usize,output:usize){
        self.peak_rss_kib=self.peak_rss_kib.max(discovery_rss_kib());
        let wall=self.started.elapsed().as_secs_f64();
        let cpu=(discovery_cpu_seconds()-self.cpu_started).max(0.0);
        let line=format!("{}: input={} output={} wall={:.3}s cpu={:.3}s peak_observed_rss={} KiB workers={}",
            self.stage.name(),input,output,wall,cpu,self.peak_rss_kib,self.workers);
        crate::maker::operational_log(format_args!("[MAKER DISCOVERY STAGE] {line}"));self.reports.push(line);
        self.stage=next;self.cursor=0;self.started=Instant::now();
        self.cpu_started=discovery_cpu_seconds();self.peak_rss_kib=discovery_rss_kib();
    }
    fn advance(&mut self,source_revision:u64,_jobs:&mut Vec<CandidateJob>){
        match self.stage {
            DiscoveryStage::Feasibility=>{
                let Some(base)=self.base.clone() else {return};
                let index=ConnectionFeasibility::build(&base,self.output);
                crate::maker::operational_log(format_args!("[MAKER FEASIBILITY] raw_pairs={} type_compatible={} feasible={} reachable_nodes={} excluded_nodes={}",
                    index.raw_pairs,index.typed_pairs,index.feasible_pairs,index.reachable_nodes,index.excluded_nodes));
                let input=index.raw_pairs;let output=index.feasible_pairs;
                match maker_discovery::ProductionEnumeration::open(base,self.min_wire_connections,self.max_wire_connections) {
                    Ok((canonical,resumed))=>{
                        self.enumeration_start=canonical.examined();
                        crate::maker::operational_log(format_args!("[MAKER DISCOVERY] canonical enumeration {} from state={} digest={:016x}",
                            if resumed {"resumed"} else {"started"},self.enumeration_start,canonical.digest()));
                        self.canonical=Some(canonical);
                        self.transition(DiscoveryStage::Enumerate,input,output);
                    }
                    Err(error)=>{
                        self.discovery_error=Some(format!("Unable to start canonical enumeration: {error}. Use Reset Enumeration to discard an incompatible checkpoint."));
                        self.transition(DiscoveryStage::Ready,input,0);
                    }
                }
            }
            DiscoveryStage::Enumerate=>{
                let deadline=Instant::now()+std::time::Duration::from_millis(8);
                let mut batch=0usize;
                let mut exhausted=false;
                while self.examined<self.max_enumerations
                    && batch<128 && Instant::now()<deadline {
                    let next=match self.canonical.as_mut() {
                        Some(enumerator)=>enumerator.next_graph(),
                        None=>Err("Canonical enumerator is missing".into()),
                    };
                    let graph=match next {
                        Ok(Some(graph))=>graph,
                        Ok(None)=>{exhausted=true;break},
                        Err(error)=>{
                            self.discovery_error=Some(format!("Canonical enumeration stopped: {error}"));
                            exhausted=true;break;
                        }
                    };
                    batch+=1;
                    self.examined+=1;
                    // Preserve Stage 2's structural validation boundary.
                    if (self.min_wire_connections..=self.max_wire_connections).contains(&graph.connections.len()) && graph.input_connection(self.output,0).is_some() {
                        self.raw_seen+=1;
                        let graph=CompactCandidate::capture(graph);
                        if graph.callable {
                            self.callable_seen+=1;
                            // Dedicated reservoir prevents noncallable graphs from evicting every callable.
                            const CALLABLE_RESERVOIR:usize=2_000;
                            if let Some(slot)=reservoir_slot(self.callable_seen,CALLABLE_RESERVOIR) {
                                if slot==self.callable_raw.len(){self.callable_raw.push(graph.clone());}
                                else {self.callable_raw[slot]=graph.clone();}
                            }
                        }
                        if let Some(slot)=reservoir_slot(self.raw_seen,MAX_RAW_GRAPHS){
                            if slot==self.raw.len(){self.raw.push(graph)}
                            else {self.raw[slot]=graph}
                        }
                    }
                }
                if let Some(enumerator)=self.canonical.as_ref(){
                    self.duplicates=0;
                    if let Err(error)=enumerator.save(){
                        self.discovery_error=Some(format!("Cannot persist discovery checkpoint: {error}"));
                        exhausted=true;
                    }
                }
                if exhausted || self.examined>=self.max_enumerations {
                    // Merge callable snapshots before validation, deduplicating in the normal stage.
                    let callable_stored=self.callable_raw.len();
                    self.raw.append(&mut self.callable_raw);
                    crate::maker::operational_log(format_args!("[MAKER CALLABLE ENUMERATION] emitted={} output_connected={} callable_seen={} callable_stored={}",
                        self.examined,self.raw_seen,self.callable_seen,callable_stored));
                    let input=self.examined;let output=self.raw.len();
                    self.canonical=None;
                    self.transition(DiscoveryStage::Validate,input,output);
                }
            }
            DiscoveryStage::Validate=>{
                let end=(self.cursor+CPU_STAGE_BATCH).min(self.raw.len());
                let base=self.base.as_ref().expect("Discovery base retained");
                let checks=parallel_map(&self.raw[self.cursor..end],self.workers,|g|graph_preflight(&g.materialize(base),self.output).is_ok());
                // Compact in place only at the end of the stage; preserve deterministic order.
                for (i,valid) in checks.into_iter().enumerate(){if !valid{self.incomplete+=1;}else{
                    if self.raw[self.cursor+i].callable {self.callable_qualified+=1;}
                    self.qualified.push(self.raw[self.cursor+i].clone());
                }}
                self.cursor=end;
                if end==self.raw.len(){let input=self.raw.len();let output=self.qualified.len();
                    self.raw.clear();self.transition(DiscoveryStage::Deduplicate,input,output);}
            }
            DiscoveryStage::Deduplicate=>{
                let end=(self.cursor+CPU_STAGE_BATCH).min(self.qualified.len());
                let keys=parallel_map(&self.qualified[self.cursor..end],self.workers,CompactCandidate::key);
                for (i,key) in keys.into_iter().enumerate(){
                    if self.seen.insert(key){self.raw.push(self.qualified[self.cursor+i].clone());}
                    else{self.duplicates+=1;}
                }
                self.cursor=end;
                if end==self.qualified.len(){let input=self.qualified.len();let output=self.raw.len();
                    self.qualified.clear();self.seen.clear();self.transition(DiscoveryStage::Qualify,input,output);}
            }
            DiscoveryStage::Qualify=>{
                let end=(self.cursor+CPU_STAGE_BATCH).min(self.raw.len());
                let base=self.base.as_ref().expect("Discovery base retained");
                let scores=parallel_map(&self.raw[self.cursor..end],self.workers,|candidate| {
                    let graph=&candidate.materialize(base);
                    (graph_discovery_penalty(graph),discovery_priority(graph),graph_source_cost(graph),graph_compile_cost(graph),candidate_admission(graph).is_ok())
                });
                for (i,(semantic_penalty,score,source_cost,runtime_cost,admitted)) in scores.into_iter().enumerate(){
                    self.min_source_cost=self.min_source_cost.min(source_cost);
                    self.max_source_cost=self.max_source_cost.max(source_cost);
                    self.min_runtime_cost=self.min_runtime_cost.min(runtime_cost);
                    self.max_runtime_cost=self.max_runtime_cost.max(runtime_cost);
                    if admitted {
                        if self.raw[self.cursor+i].callable {self.callable_admissible+=1;}
                        self.scores.push((semantic_penalty,score,self.cursor+i));
                    } else {self.deferred_complexity+=1;}
                }
                self.cursor=end;
                if end==self.raw.len(){
                    crate::maker::operational_log(format_args!("[MAKER DISCOVERY COMPLEXITY] qualified={} deferred={} max_source_cost={} max_runtime_work={} source_range={}..{} runtime_range={}..{}",
                        self.scores.len(),self.deferred_complexity,MAX_COMPILE_COST,MAX_ESTIMATED_RUNTIME_WORK,
                        if self.min_source_cost==u64::MAX {0}else{self.min_source_cost},self.max_source_cost,
                        if self.min_runtime_cost==u64::MAX {0}else{self.min_runtime_cost},self.max_runtime_cost));
                    let constant_normal_candidates=self.scores.iter().filter(|(_,_,index)|
                        callable_sampling_penalty(&self.raw[*index].materialize(base))>0).count();
                    let callable_time_hints=self.scores.iter().filter(|(_,_,index)|
                        self.raw[*index].callable && graph_time_dependency_hint(&self.raw[*index].materialize(base))).count();
                    crate::maker::operational_log(format_args!("[MAKER CALLABLE QUALITY] admissible_constant_normal={} admissible_time_hint={} (time hint is not measured motion)",
                        constant_normal_candidates,callable_time_hints));
                    // Preserve the semantic score and hash ordering as a deterministic
                    // shortlist, then select distinct graph structures for GPU work.
                    self.scores.sort_unstable_by(|a,b|a.0.cmp(&b.0).then_with(||a.1.cmp(&b.1)).then_with(||a.2.cmp(&b.2)));
                    // Preserve the original GPU budget. Reserve up to eight slots for callable
                    // graphs, selected from their own semantic-ranked pool rather than the global top 1024.
                    let callable_ranked:Vec<_>=self.scores.iter().copied()
                        .filter(|(_,_,i)|self.raw[*i].callable).collect();
                    let mut chosen=select_diverse_gpu_indices(&self.raw,&callable_ranked,8);
                    let regular=select_diverse_gpu_indices(&self.raw,&self.scores,MAX_CONNECTION_GPU_JOBS);
                    for index in regular {
                        if chosen.len()>=MAX_CONNECTION_GPU_JOBS {break;}
                        if !chosen.contains(&index) {chosen.push(index);}
                    }
                    let callable_selected=chosen.iter().filter(|&&i|self.raw[i].callable).count();
                    crate::maker::operational_log(format_args!("[MAKER CALLABLE PIPELINE] enumerated={} preflight_passed={} admission_passed={} gpu_selected={}",
                        self.callable_seen,self.callable_qualified,self.callable_admissible,callable_selected));
                    let selected=chosen.len();
                    let mut counts=std::collections::BTreeMap::<usize,usize>::new();
                    let mut overlap_sum=0usize;
                    let selected_keys:Vec<_>=chosen.iter().map(|&i|self.raw[i].key()).collect();
                    for (position,&index) in chosen.iter().enumerate(){
                        *counts.entry(self.raw[index].connections.len()).or_default()+=1;
                        if position>0 {overlap_sum+=selected_keys[..position].iter()
                            .map(|key|shared_connection_count(key,&selected_keys[position])).max().unwrap_or(0);}
                    }
                    crate::maker::operational_log(format_args!("[MAKER DISCOVERY SAMPLE] output_connected={} reservoir={} admissible={} gpu_selected={} selected_wire_counts={:?} max_prior_overlap_mean={:.2}",
                        self.raw_seen,self.raw.len(),self.scores.len(),selected,counts,
                        overlap_sum as f32/(selected.saturating_sub(1).max(1) as f32)));
                    // Reverse push order because next_candidate_job() pops from the end.
                    for &index in chosen.iter().rev(){
                        let graph=self.raw[index].materialize(base);
                        let name=format!("Connections: {} wires",graph.connections.len());
                        self.selected_jobs.push(CandidateJob{graph,name,
                            source_revision,connection_candidate:true});
                    }
                    self.generated=selected;
                    let input=self.raw.len();self.raw.clear();self.qualified.clear();self.scores.clear();
                    self.transition(DiscoveryStage::Ready,input,selected);
                }
            }
            DiscoveryStage::Ready=>{}
        }
    }
}


fn apply_connection_candidate(graph:&mut Graph,candidate:&Candidate,base:&Graph)->Result<(),String>{
    if graph.nodes.len()!=base.nodes.len() {return Err("Modules changed; rediscover connections".into())}
    for node in &graph.nodes {
        if base.node(node.id).is_none_or(|original|original.module_id!=node.module_id){
            return Err("Modules changed; rediscover connections".into())
        }
    }
    let mut proposed=graph.clone();
    proposed.connections.clear();
    for c in &candidate.connections {proposed.connect(c.from,c.from_port,c.to,c.to_port)?;}
    proposed.validate()?;
    proposed.revision=graph.revision.wrapping_add(1);
    *graph=proposed;
    Ok(())
}

fn apply_candidate(graph:&mut Graph,candidate:&Candidate)->Result<(),String>{
    if graph.nodes.len()!=candidate.parameters.len(){return Err("Candidate graph no longer matches project".into())}
    if graph.connections!=candidate.connections{return Err("Project connections changed; regenerate candidates".into())}
    for node in &graph.nodes {
        if !candidate.parameters.contains_key(&node.id) || candidate.modules.get(&node.id)!=Some(&node.module_id){return Err("Project modules changed; regenerate candidates".into())}
    }
    for node in &mut graph.nodes {
        let mut next=candidate.parameters[&node.id].clone();
        // Candidate snapshots must not change animation speed, even if the
        // user adjusted it after candidate discovery.
        for (key,value) in &node.parameters {
            if is_speed_parameter(key) {next.insert(key.clone(),value.clone());}
        }
        node.parameters=next;
    }
    graph.revision+=1;
    Ok(())
}
// Maker projects are JSON documents with the .smprj extension.
fn maker_projects_dir()->Result<std::path::PathBuf,String>{
    let home=std::env::var_os("HOME").ok_or("HOME is not set")?;
    Ok(std::path::PathBuf::from(home).join(".config/screenshaver/maker/projects"))
}
// An explicit filesystem path is used for both load and save. The project
// directory is a starting location, not a restriction on where files may live.
fn maker_project_path(path:&std::path::Path)->Result<std::path::PathBuf,String>{
    let name=path.file_name().and_then(|n|n.to_str()).ok_or("Enter a project filename")?;
    if name.is_empty() || name=="." || name==".." || name.chars().any(|c|c.is_control()) {
        return Err("Invalid project filename".into());
    }
    let mut result=path.to_path_buf();
    if result.extension().is_none(){result.set_extension("smprj");}
    if result.extension().and_then(|s|s.to_str())!=Some("smprj") {
        return Err("Maker projects must use the .smprj extension".into());
    }
    Ok(result)
}

#[derive(Clone)]
struct MakerProjectBrowser {
    save:bool,
    directory:std::path::PathBuf,
    filename:String,
    overwrite:Option<std::path::PathBuf>,
    error:Option<String>,
}
impl MakerProjectBrowser {
    fn open(save:bool,current:Option<&std::path::Path>,name:&str)->Self{
        let directory=maker_projects_dir().ok()
            .unwrap_or_else(||std::path::PathBuf::from("."));
        if save {let _=std::fs::create_dir_all(&directory);}
        let filename=current.and_then(|p|p.file_name()).and_then(|n|n.to_str())
            .map(str::to_owned).unwrap_or_else(||format!("{}.smprj",name.trim_end_matches(".smprj")));
        Self{save,directory,filename,overwrite:None,error:None}
    }
    fn change_directory(&mut self,path:std::path::PathBuf){
        match std::fs::canonicalize(&path){
            Ok(p) if p.is_dir()=>{self.directory=p;self.overwrite=None;self.error=None;},
            Ok(_)=>self.error=Some("Not a directory".into()),
            Err(e)=>self.error=Some(format!("Cannot open directory: {e}")),
        }
    }
    fn selected_path(&self)->Result<std::path::PathBuf,String>{
        let name=self.filename.trim();
        if name.is_empty() || name=="." || name==".." || name.contains('/') || name.contains('\\') {
            return Err("Enter a filename without path separators".into());
        }
        maker_project_path(&self.directory.join(name))
    }
}

fn maker_save_project(graph:&Graph,path:&std::path::Path)->Result<(),String>{
    use std::io::Write;
    let path=maker_project_path(path)?;
    let dir=path.parent().ok_or("Invalid projects directory")?;
    std::fs::create_dir_all(dir).map_err(|e|e.to_string())?;
    let data=serde_json::to_vec_pretty(&serde_json::json!({
        "format":"screenshaver-maker-project", "version":1,
        "graph":crate::maker::graph_to_wire(graph)
    })).map_err(|e|e.to_string())?;
    // Stage beside the destination and rename only after a complete write.
    let tmp=path.with_extension(format!("smprj.tmp.{}",std::process::id()));
    let result=(||->Result<(),String>{
        let mut file=std::fs::File::create(&tmp).map_err(|e|e.to_string())?;
        file.write_all(&data).map_err(|e|e.to_string())?;
        file.sync_all().map_err(|e|e.to_string())?;
        std::fs::rename(&tmp,&path).map_err(|e|e.to_string())
    })();
    if result.is_err(){let _=std::fs::remove_file(&tmp);}
    result
}
fn maker_load_project(path:&std::path::Path)->Result<Graph,String>{
    let path=maker_project_path(path)?;
    let bytes=std::fs::read(&path).map_err(|e|e.to_string())?;
    let value:serde_json::Value=serde_json::from_slice(&bytes).map_err(|e|e.to_string())?;
    if value["format"].as_str()!=Some("screenshaver-maker-project") || value["version"].as_u64()!=Some(1){
        return Err("Unsupported Maker project format or version".into());
    }
    let wire=&value["graph"];
    let mut graph=crate::maker::graph_from_wire(wire)?;
    // graph_from_wire reuses the mandatory output terminal. Restore its saved position.
    if let Some(saved)=wire["nodes"].as_array().and_then(|nodes|nodes.iter().find(|n|n["module_id"]=="screenshaver.output.color")) {
        if let Some(output)=graph.nodes.iter_mut().find(|n|n.module_id=="screenshaver.output.color") {
            for axis in 0..2 {if let Some(v)=saved["pos"][axis].as_f64(){output.pos[axis]=v as f32;}}
            output.expanded=saved["expanded"].as_bool().unwrap_or(true);
            if let Some(params)=saved["parameters"].as_object(){output.parameters=params.iter().map(|(k,v)|(k.clone(),v.clone())).collect();}
        }
    }
    graph.validate()?;
    Ok(graph)
}
fn maker_browser_entries(directory:&std::path::Path)->Result<Vec<(String,bool)>,String>{
    let mut entries=Vec::new();
    for item in std::fs::read_dir(directory).map_err(|e|e.to_string())? {
        let item=item.map_err(|e|e.to_string())?;
        let kind=item.file_type().map_err(|e|e.to_string())?;
        let is_dir=kind.is_dir();
        let name=item.file_name().to_string_lossy().into_owned();
        if is_dir || (kind.is_file() && item.path().extension().and_then(|s|s.to_str())==Some("smprj")) {
            entries.push((name,is_dir));
        }
    }
    entries.sort_by(|a,b|b.1.cmp(&a.1).then_with(||a.0.to_lowercase().cmp(&b.0.to_lowercase())).then_with(||a.0.cmp(&b.0)));
    Ok(entries)
}


// A tab owns its entire editable workspace, including discovery and undo history.
// Inactive tabs retain their GPU thumbnail texture handles and canonical DFS checkpoints.
struct MakerProjectTab {
    graph: Graph,
    project_name: String,
    thumbnail_workspace: bool,
    thumbnail_hash: Option<String>,
    project_path: Option<std::path::PathBuf>,
    saved_graph_snapshot: serde_json::Value,
    selected: Option<NodeId>,
    parameter_popup: Option<NodeId>,
    pending_source: Option<(NodeId, usize)>,
    dragging_wire: Option<(NodeId, usize)>,
    preview_active: bool,
    routes: Routes,
    canvas_pan: egui::Vec2,
    canvas_zoom: f32,
    pending_zoom: Option<f32>,
    pending_zoom_to_fit: bool,
    pinpoint_next: HashMap<String, NodeId>,
    pinpoint_target: Option<NodeId>,
    pinpoint_pulse: Option<(NodeId, Instant)>,
    pinpoint_boxes: BTreeMap<NodeId, Box2>,
    workspace_min: [f32;2],
    workspace_max: [f32;2],
    routing_undo: Vec<(Graph,Routes)>,
    routing_redo: Vec<(Graph,Routes)>,
    candidate_mode: u8,
    candidate_sort: u8,
    max_wire_connections: usize,
    min_wire_connections: usize,
    max_enumerations: usize,
    max_candidates: usize,
    candidates: Vec<Candidate>,
    candidate_jobs: Vec<CandidateJob>,
    candidate_epoch: u64,
    candidate_base: Option<Graph>,
    candidate_selected: Option<usize>,
    candidate_undo: Vec<Graph>,
    orphan_undo: Vec<Graph>,
    orphan_redo: Vec<Graph>,
    orphan_restore_pending: bool,
    candidate_notice: String,
    connection_search: Option<ConnectionSearch>,
    reset_enumeration_confirm: bool,
    candidate_evaluated: usize,
    candidate_glsl_capable: usize,
    candidate_rejections: usize,
    candidate_uniform: usize,
    candidate_glsl_errors: usize,
    candidate_render_errors: usize,
    analysis_request: Option<usize>,
    analysis: Option<CandidateAnalysis>,
    pending_glsl_export: Option<(usize,bool)>,

}
impl MakerProjectTab {
    fn blank(epoch:u64)->Self {
        let graph=Graph::blank();
        let saved_graph_snapshot=serde_json::json!(crate::maker::graph_to_wire(&graph));
        let (min_wire_connections,max_wire_connections,max_enumerations,max_candidates)=maker_last_discovery_settings();
        Self {
            graph: graph,
            project_name: "Untitled".into(),
            thumbnail_workspace: false,
            thumbnail_hash: None,
            project_path: None,
            saved_graph_snapshot: saved_graph_snapshot,
            selected: None,
            parameter_popup: None,
            pending_source: None,
            dragging_wire: None,
            preview_active: true,
            routes: Routes::new(),
            canvas_pan: egui::Vec2::ZERO,
            canvas_zoom: 1.0,
            pending_zoom: None,
            pending_zoom_to_fit: true,
            pinpoint_next: HashMap::new(),
            pinpoint_target: None,
            pinpoint_pulse: None,
            pinpoint_boxes: BTreeMap::new(),
            workspace_min: [-1000.0,-1000.0],
            workspace_max: [2000.0,2000.0],
            routing_undo: Vec::new(),
            routing_redo: Vec::new(),
            candidate_mode: 2,
            candidate_sort: 0,
            max_wire_connections: max_wire_connections,
            min_wire_connections: min_wire_connections,
            max_enumerations: max_enumerations,
            max_candidates: max_candidates,
            candidates: Vec::new(),
            candidate_jobs: Vec::new(),
            candidate_epoch: epoch,
            candidate_base: None,
            candidate_selected: None,
            candidate_undo: Vec::new(),
            orphan_undo: Vec::new(),
            orphan_redo: Vec::new(),
            orphan_restore_pending: false,
            candidate_notice: String::new(),
            connection_search: None,
            reset_enumeration_confirm: false,
            candidate_evaluated: 0,
            candidate_glsl_capable: 0,
            candidate_rejections: 0,
            candidate_uniform: 0,
            candidate_glsl_errors: 0,
            candidate_render_errors: 0,
            analysis_request: None,
            analysis: None,
            pending_glsl_export: None,
        }
    }
}

pub struct MakerUi {
    inactive_tabs: Vec<MakerProjectTab>,
    close_all_running: bool,
    active_tab: usize,
    project_tab_scroll_x: f32,
    project_open: bool,
    next_tab_epoch: u64,
    context: egui::Context,
    painter: egui_glow::Painter,
    events: Vec<egui::Event>,
    pointer: egui::Pos2,
    modifiers: egui::Modifiers,
    start: Instant,
    graph: Graph,
    project_name: String,
    thumbnail_workspace: bool,
    thumbnail_hash: Option<String>,
    project_dialog: Option<MakerProjectBrowser>,
    examine_module: Option<String>,
    glsl_export_dialog: Option<(String,String,bool,String,bool,ExportFormat)>,
    last_glsl_curator: String,
    project_path: Option<std::path::PathBuf>,
    recent_projects: Vec<std::path::PathBuf>,
    saved_graph_snapshot: serde_json::Value,
    unsaved_action: Option<&'static str>,
    after_save_action: Option<&'static str>,
    discard_approved: Option<&'static str>,
    selected: Option<NodeId>,
    parameter_popup: Option<NodeId>,
    pending_source: Option<(NodeId, usize)>,
    dragging_wire: Option<(NodeId, usize)>,

    status: String,
    menu_icon: egui::TextureHandle,
    branding_texture: egui::TextureHandle,
    branding_aspect_ratio: f32,
    open: bool,
    preview_rect: egui::Rect,
    preview_height: Option<f32>,
    preview_active: bool,
    routes: Routes,
    canvas_pan: egui::Vec2,
    canvas_zoom: f32,
    pending_zoom: Option<f32>,
    pending_zoom_to_fit: bool,
    pending_menu_command: Option<&'static str>,
    pending_tools_action: Option<&'static str>,
    canvas_rect: egui::Rect,
    parameter_wheel_rect: egui::Rect,
    pinpoint_next: HashMap<String, NodeId>,
    pinpoint_target: Option<NodeId>,
    pinpoint_pulse: Option<(NodeId, Instant)>,
    pinpoint_boxes: BTreeMap<NodeId, Box2>,
    workspace_min: [f32;2],
    workspace_max: [f32;2],
    routing_undo: Vec<(Graph,Routes)>,
    routing_redo: Vec<(Graph,Routes)>,
    candidate_mode: u8,
    candidate_sort: u8,
    max_wire_connections: usize,
    min_wire_connections: usize,
    max_enumerations: usize,
    max_candidates: usize,
    candidates: Vec<Candidate>,
    candidate_jobs: Vec<CandidateJob>,
    candidate_epoch: u64,
    candidate_base: Option<Graph>,
    candidate_selected: Option<usize>,
    candidate_undo: Vec<Graph>,
    orphan_undo: Vec<Graph>,
    orphan_redo: Vec<Graph>,
    delete_project_confirm: bool,
    orphan_restore_pending: bool,
    candidate_notice: String,
    library_expanded: HashMap<String,bool>,
    library_search: String,
    connection_search: Option<ConnectionSearch>,
    reset_enumeration_confirm:bool,
    candidate_evaluated: usize,
    candidate_glsl_capable: usize,
    candidate_rejections: usize,
    candidate_uniform: usize,
    candidate_glsl_errors: usize,
    candidate_render_errors: usize,
    analysis_request: Option<usize>,
    analysis: Option<CandidateAnalysis>,
    pending_clipboard: Option<String>,
    pending_clipboard_is_glsl: bool,
    pending_clipboard_is_module: bool,
    pending_glsl_export: Option<(usize,bool)>,

}
impl MakerUi {
    // The active workspace is stored directly in MakerUi for compatibility with the
    // existing graph/editor code; every inactive workspace is parked in its tab.
    fn exchange_tab(&mut self, tab:&mut MakerProjectTab) {
        std::mem::swap(&mut self.graph, &mut tab.graph);
        std::mem::swap(&mut self.project_name, &mut tab.project_name);
        std::mem::swap(&mut self.thumbnail_workspace, &mut tab.thumbnail_workspace);
        std::mem::swap(&mut self.thumbnail_hash, &mut tab.thumbnail_hash);
        std::mem::swap(&mut self.project_path, &mut tab.project_path);
        std::mem::swap(&mut self.saved_graph_snapshot, &mut tab.saved_graph_snapshot);
        std::mem::swap(&mut self.selected, &mut tab.selected);
        std::mem::swap(&mut self.parameter_popup, &mut tab.parameter_popup);
        std::mem::swap(&mut self.pending_source, &mut tab.pending_source);
        std::mem::swap(&mut self.dragging_wire, &mut tab.dragging_wire);
        std::mem::swap(&mut self.preview_active, &mut tab.preview_active);
        std::mem::swap(&mut self.routes, &mut tab.routes);
        std::mem::swap(&mut self.canvas_pan, &mut tab.canvas_pan);
        std::mem::swap(&mut self.canvas_zoom, &mut tab.canvas_zoom);
        std::mem::swap(&mut self.pending_zoom, &mut tab.pending_zoom);
        std::mem::swap(&mut self.pending_zoom_to_fit, &mut tab.pending_zoom_to_fit);
        std::mem::swap(&mut self.pinpoint_next, &mut tab.pinpoint_next);
        std::mem::swap(&mut self.pinpoint_target, &mut tab.pinpoint_target);
        std::mem::swap(&mut self.pinpoint_pulse, &mut tab.pinpoint_pulse);
        std::mem::swap(&mut self.pinpoint_boxes, &mut tab.pinpoint_boxes);
        std::mem::swap(&mut self.workspace_min, &mut tab.workspace_min);
        std::mem::swap(&mut self.workspace_max, &mut tab.workspace_max);
        std::mem::swap(&mut self.routing_undo, &mut tab.routing_undo);
        std::mem::swap(&mut self.routing_redo, &mut tab.routing_redo);
        std::mem::swap(&mut self.candidate_mode, &mut tab.candidate_mode);
        std::mem::swap(&mut self.candidate_sort, &mut tab.candidate_sort);
        std::mem::swap(&mut self.max_wire_connections, &mut tab.max_wire_connections);
        std::mem::swap(&mut self.min_wire_connections, &mut tab.min_wire_connections);
        std::mem::swap(&mut self.max_enumerations, &mut tab.max_enumerations);
        std::mem::swap(&mut self.max_candidates, &mut tab.max_candidates);
        std::mem::swap(&mut self.candidates, &mut tab.candidates);
        std::mem::swap(&mut self.candidate_jobs, &mut tab.candidate_jobs);
        std::mem::swap(&mut self.candidate_epoch, &mut tab.candidate_epoch);
        std::mem::swap(&mut self.candidate_base, &mut tab.candidate_base);
        std::mem::swap(&mut self.candidate_selected, &mut tab.candidate_selected);
        std::mem::swap(&mut self.candidate_undo, &mut tab.candidate_undo);
        std::mem::swap(&mut self.orphan_undo, &mut tab.orphan_undo);
        std::mem::swap(&mut self.orphan_redo, &mut tab.orphan_redo);
        std::mem::swap(&mut self.orphan_restore_pending, &mut tab.orphan_restore_pending);
        std::mem::swap(&mut self.candidate_notice, &mut tab.candidate_notice);
        std::mem::swap(&mut self.connection_search, &mut tab.connection_search);
        std::mem::swap(&mut self.reset_enumeration_confirm, &mut tab.reset_enumeration_confirm);
        std::mem::swap(&mut self.candidate_evaluated, &mut tab.candidate_evaluated);
        std::mem::swap(&mut self.candidate_glsl_capable, &mut tab.candidate_glsl_capable);
        std::mem::swap(&mut self.candidate_rejections, &mut tab.candidate_rejections);
        std::mem::swap(&mut self.candidate_uniform, &mut tab.candidate_uniform);
        std::mem::swap(&mut self.candidate_glsl_errors, &mut tab.candidate_glsl_errors);
        std::mem::swap(&mut self.candidate_render_errors, &mut tab.candidate_render_errors);
        std::mem::swap(&mut self.analysis_request, &mut tab.analysis_request);
        std::mem::swap(&mut self.analysis, &mut tab.analysis);
        std::mem::swap(&mut self.pending_glsl_export, &mut tab.pending_glsl_export);
    }
    fn switch_project_tab(&mut self, target:usize) {
        // inactive_tabs contains every tab EXCEPT the active one, in display order.
        // The last tab is a real tab, never a sentinel for creating a new one.
        if !self.project_open || target==self.active_tab || target>self.inactive_tabs.len(){return;}
        let old=self.active_tab;
        let target_slot=if target<old {target} else {target-1};
        let mut selected=self.inactive_tabs.remove(target_slot);
        let mut parked=MakerProjectTab::blank(0);
        self.exchange_tab(&mut parked);
        self.exchange_tab(&mut selected);
        // Removing a tab before the former active tab shifts its slot left.
        // Reinsert at the former active tab's position in the reduced list.
        let parked_slot=if target<old {old-1} else {old};
        self.inactive_tabs.insert(parked_slot,parked);
        self.active_tab=target;
        self.project_dialog=None;
        self.unsaved_action=None;
        self.after_save_action=None;
        self.discard_approved=None;
        self.pending_menu_command=None;
        self.preview_rect=egui::Rect::NOTHING;
        self.context.request_repaint();
    }
    // Move a newly saved Thumbnail into the ordinary tab sequence without
    // reloading its graph or renderer. The active project remains active.
    fn move_saved_thumbnail_to_end(&mut self) {
        if !self.project_open || self.active_tab == self.inactive_tabs.len() { return; }
        let old = self.active_tab;
        let mut active = MakerProjectTab::blank(0);
        self.exchange_tab(&mut active);
        self.inactive_tabs.insert(old, active);
        let mut selected = self.inactive_tabs.remove(old);
        self.exchange_tab(&mut selected);
        self.active_tab = self.inactive_tabs.len();
        self.context.request_repaint();
    }
    fn create_project_tab(&mut self){
        // The startup/last-closed workspace is not a project and must not be parked.
        if self.project_open {
            let mut parked=MakerProjectTab::blank(0);
            self.exchange_tab(&mut parked);
            self.inactive_tabs.insert(self.active_tab,parked);
        }
        let mut fresh=MakerProjectTab::blank(self.next_tab_epoch<<32);
        self.next_tab_epoch=self.next_tab_epoch.wrapping_add(1);
        self.exchange_tab(&mut fresh);
        self.active_tab=self.inactive_tabs.len();
        self.project_open=true;
        self.project_dialog=None;
        self.unsaved_action=None;
        self.after_save_action=None;
        self.discard_approved=None;
        self.pending_menu_command=None;
        self.status="New project tab created".into();
        self.context.request_repaint();
    }
    // Only one unsaved Thumbnail workspace exists. Discovery results are copied
    // from the source project when opening it, so browsing remains possible
    // after switching tabs without mutating the source graph.
    fn show_candidate_thumbnail(&mut self, mut candidate_graph: Graph) {
        let hash = candidate_composition_fingerprint(&candidate_graph);
        let source_candidates = self.candidates.clone();
        let source_base = self.candidate_base.clone();
        let source_mode = self.candidate_mode;
        let source_sort = self.candidate_sort;
        let source_notice = self.candidate_notice.clone();
        let source_epoch = self.candidate_epoch;
        if !self.project_open || !self.thumbnail_workspace {
            if let Some(index) = self.inactive_tabs.iter().position(|tab|tab.thumbnail_workspace) {
                self.switch_project_tab(if index < self.active_tab {index} else {index+1});
            } else {
                self.create_project_tab();
            }
        }
        // An existing Thumbnail retains its own candidate browser. When the
        // click originated elsewhere, refresh the browser from that source.
        if source_epoch != self.candidate_epoch {
            self.candidates = source_candidates;
            self.candidate_base = source_base;
            self.candidate_mode = source_mode;
            self.candidate_sort = source_sort;
            self.candidate_notice = source_notice;
            self.candidate_selected = None;
            self.candidate_jobs.clear();
            self.connection_search = None;
        }
        // The live Preview caches its GL program by graph revision. Candidate
        // graphs often carry the same revision, so deriving the next revision
        // from the incoming candidate can leave the previous shader running.
        // Advance from the graph currently shown instead.
        candidate_graph.revision = self.graph.revision.wrapping_add(1);
        self.graph = candidate_graph;
        self.project_name = "Thumbnail".into();
        self.thumbnail_workspace = true;
        self.thumbnail_hash = Some(hash);
        self.project_path = None;
        self.selected = None;
        self.parameter_popup = None;
        self.pending_source = None;
        self.dragging_wire = None;
        self.routes.clear();
        self.routing_undo.clear();
        self.routing_redo.clear();
        self.candidate_undo.clear();
        self.canvas_pan = egui::Vec2::ZERO;
        self.canvas_zoom = 1.0;
        self.pending_zoom_to_fit = true;
        self.preview_active = true;
        self.preview_rect = egui::Rect::NOTHING;
        self.status = "Thumbnail loaded; click another candidate to replace it, or save to keep it".into();
        self.context.request_repaint();
    }
    fn advance_close_all(&mut self) {
        if !self.close_all_running { return; }
        if !self.project_open {
            self.close_all_running = false;
            self.status = "All projects closed".into();
            return;
        }
        if self.unsaved_action.is_some() || self.project_dialog.is_some()
            || self.after_save_action.is_some() { return; }
        if self.discard_approved == Some("close_all") {
            self.discard_approved = None;
            self.close_active_project_tab();
            return;
        }
        let dirty = serde_json::json!(crate::maker::graph_to_wire(&self.graph))
            != self.saved_graph_snapshot;
        if dirty && !self.thumbnail_workspace {
            self.unsaved_action = Some("close_all");
        } else {
            self.close_active_project_tab();
        }
    }
    fn close_active_project_tab(&mut self){
        if !self.project_open {return;}
        if self.inactive_tabs.is_empty(){
            let mut empty=MakerProjectTab::blank(self.next_tab_epoch<<32);
            self.next_tab_epoch=self.next_tab_epoch.wrapping_add(1);
            self.exchange_tab(&mut empty);
            self.project_open=false;
            self.active_tab=0;
            self.preview_active=false;
            self.status="No project open".into();
        }else{
            let old=self.active_tab;
            let next=if old>0 {old-1} else {0};
            let slot=if old>0 {old-1} else {0};
            let mut tab=self.inactive_tabs.remove(slot);
            self.exchange_tab(&mut tab);
            self.active_tab=next;
            self.status="Project closed".into();
        }
        self.project_dialog=None;
        self.unsaved_action=None;
        self.after_save_action=None;
        self.discard_approved=None;
        self.pending_menu_command=None;
        self.context.request_repaint();
    }
    fn open_project_in_tab(&mut self,path:std::path::PathBuf,mut graph:Graph){
        if self.project_open && self.project_path.as_ref()==Some(&path){return;}
        if let Some(index)=self.inactive_tabs.iter().position(|t|t.project_path.as_ref()==Some(&path)){
            let target=if index<self.active_tab {index} else {index+1};
            self.switch_project_tab(target);
            return;
        }
        self.create_project_tab();
        graph.revision=graph.revision.wrapping_add(1);
        self.graph=graph;
        self.project_name=path.file_stem().and_then(|s|s.to_str()).unwrap_or("Untitled").into();
        self.project_path=Some(path);
        self.saved_graph_snapshot=serde_json::json!(crate::maker::graph_to_wire(&self.graph));
        self.preview_active=true;
        let unresolved:Vec<_>=self.graph.nodes.iter().filter(|n|n.unresolved()).collect();
        self.status=if unresolved.is_empty(){format!("Opened project {}",self.project_name)}else{
            let invalid=unresolved.iter().filter(|n|n.invalid_reason.is_some()).count();
            format!("Project opened with {} unresolved modules (Missing {}, Invalid {})",unresolved.len(),unresolved.len()-invalid,invalid)
        };
    }
    pub fn with_candidate_epoch<R>(&mut self,epoch:u64,f:impl FnOnce(&mut Self)->R)->Option<R>{
        if self.candidate_epoch==epoch {return Some(f(self));}
        let index=self.inactive_tabs.iter().position(|t|t.candidate_epoch==epoch)?;
        let mut tab=self.inactive_tabs.remove(index);
        self.exchange_tab(&mut tab);
        let result=f(self);
        self.exchange_tab(&mut tab);
        self.inactive_tabs.insert(index,tab);
        Some(result)
    }
    pub fn contains_candidate_epoch(&self,epoch:u64)->bool{
        self.candidate_epoch==epoch || self.inactive_tabs.iter().any(|t|t.candidate_epoch==epoch)
    }

    pub fn new(video:&sdl2::VideoSubsystem)->Result<Self,String>{
        let gl=unsafe{glow::Context::from_loader_function(|s|video.gl_get_proc_address(s) as *const _)};
        let painter=egui_glow::Painter::new(Arc::new(gl),"",None,false).map_err(|e|e.to_string())?;
        let graph=Graph::blank();let selected=None;
        let saved_graph_snapshot=serde_json::json!(crate::maker::graph_to_wire(&graph));
        let recent_projects=maker_recent_projects();
        let (saved_min_wires,saved_max_wires,saved_max_enumerations,saved_max_candidates)=maker_last_discovery_settings();
        let context=egui::Context::default();
        let menu_icon=context.load_texture(
            "screenshaver_maker_menu_icon",
            egui::ColorImage::from_rgba_unmultiplied([22,22], &MAKER_MENU_ICON_RGBA),
            egui::TextureOptions::LINEAR,
        );
        // Reuse the exact embedded Control Center logo and preserve its aspect ratio.
        let branding_image = image::load_from_memory(MAKER_BRANDING_IMAGE)
            .map_err(|error| format!("Unable to decode Screenshaver branding image: {error}"))?
            .to_rgba8();
        let (branding_width, branding_height) = branding_image.dimensions();
        if branding_width == 0 || branding_height == 0 {
            return Err("Screenshaver branding image has invalid dimensions".into());
        }
        let branding_aspect_ratio = branding_width as f32 / branding_height as f32;
        let branding_texture = context.load_texture(
            "screenshaver_maker_branding",
            egui::ColorImage::from_rgba_unmultiplied(
                [branding_width as usize, branding_height as usize], branding_image.as_raw()),
            egui::TextureOptions::LINEAR,
        );
        Ok(Self{inactive_tabs:Vec::new(),close_all_running:false,active_tab:0,project_tab_scroll_x:0.0,project_open:true,next_tab_epoch:1,context,painter,events:Vec::new(),pointer:egui::Pos2::ZERO,
            modifiers:egui::Modifiers::default(),start:Instant::now(),graph,project_name:"Untitled".into(),thumbnail_workspace:false,thumbnail_hash:None,project_dialog:None,examine_module:None,glsl_export_dialog:None,last_glsl_curator:String::new(),project_path:None,recent_projects,saved_graph_snapshot,unsaved_action:None,after_save_action:None,discard_approved:None,selected,parameter_popup:None,pending_source:None,dragging_wire:None,
            status:"Click an output port, then a compatible input port".into(),menu_icon,branding_texture,branding_aspect_ratio,open:true,
            preview_rect:egui::Rect::from_min_size(egui::pos2(210.0,600.0),egui::vec2(950.0,210.0)),
            preview_height:maker_saved_preview_height(),preview_active:true,routes:Routes::new(),canvas_pan:egui::Vec2::ZERO,canvas_zoom:1.0,pending_zoom:None,pending_zoom_to_fit:false,pending_menu_command:None,pending_tools_action:None,canvas_rect:egui::Rect::NOTHING,parameter_wheel_rect:egui::Rect::NOTHING,pinpoint_next:HashMap::new(),pinpoint_target:None,pinpoint_pulse:None,pinpoint_boxes:BTreeMap::new(),workspace_min:[-1000.0,-1000.0],workspace_max:[2000.0,2000.0],routing_undo:Vec::new(),routing_redo:Vec::new(),candidate_mode:2,candidate_sort:0,max_wire_connections:saved_max_wires,min_wire_connections:saved_min_wires,max_enumerations:saved_max_enumerations,max_candidates:saved_max_candidates,
            candidates:Vec::new(),candidate_jobs:Vec::new(),candidate_epoch:0,candidate_base:None,
            candidate_selected:None,candidate_undo:Vec::new(),orphan_undo:Vec::new(),orphan_redo:Vec::new(),delete_project_confirm:false,orphan_restore_pending:false,candidate_notice:String::new(),library_expanded:HashMap::new(),library_search:String::new(),connection_search:None,reset_enumeration_confirm:false,candidate_evaluated:0,candidate_glsl_capable:0,candidate_rejections:0,candidate_uniform:0,candidate_glsl_errors:0,candidate_render_errors:0,analysis_request:None,analysis:None,pending_clipboard:None,pending_clipboard_is_glsl:false,pending_clipboard_is_module:false,pending_glsl_export:None})
    }
    /// Returns the exact candidate reconstruction, independent of the currently applied graph.
    /// The boolean requests saving to disk rather than copying to the clipboard.
    pub fn take_glsl_export(&mut self)->Option<(String,bool)> {
        let (index,save)=self.pending_glsl_export.take()?;
        let graph=match self.reconstructed_candidate(index) {
            Ok(graph)=>graph,
            Err(error)=>{self.candidate_notice=format!("Candidate reconstruction failed: {error}");return None;}
        };
        match crate::maker_compiler::fragment_source(&graph) {
            Ok(source)=>Some((source,save)),
            Err(error)=>{self.candidate_notice=format!("GLSL generation failed: {error}");None}
        }
    }
    pub fn glsl_export_result(&mut self,message:String){self.candidate_notice=message;}
    pub fn take_tools_action(&mut self)->Option<&'static str>{self.pending_tools_action.take()}
    pub fn tools_status(&mut self,message:String){self.status=message;}
    pub fn take_clipboard_request(&mut self)->Option<String>{self.pending_clipboard.take()}
    pub fn clipboard_result(&mut self,result:Result<(),String>){
        let is_glsl=std::mem::take(&mut self.pending_clipboard_is_glsl);
        let is_module=std::mem::take(&mut self.pending_clipboard_is_module);
        let message=match (is_glsl,is_module,result) {
            (_,true,Ok(()))=>"Module JSON copied to system clipboard".to_owned(),
            (_,true,Err(e))=>format!("Module JSON clipboard copy failed: {e}"),
            (true,false,Ok(()))=>"Active project GLSL copied to system clipboard".to_owned(),
            (true,false,Err(e))=>format!("GLSL clipboard export failed: {e}"),
            (false,false,Ok(()))=>"Analysis report copied to system clipboard".to_owned(),
            (false,false,Err(e))=>format!("Clipboard copy failed: {e}"),
        };
        self.status=message.clone();
        self.candidate_notice=message;
    }
    pub fn graph(&self)->&Graph{&self.graph}
    pub fn candidate_epoch(&self)->u64 {self.candidate_epoch}
    pub fn next_candidate_job(&mut self)->Option<CandidateJob>{
        if self.candidate_base.is_none(){return None}
        if self.candidates.len()>=CANDIDATES_RETAINED && self.connection_search.is_none() && self.candidate_jobs.first().is_some_and(|job|!job.connection_candidate) {self.candidate_jobs.clear();return None}
        if self.connection_search.is_some() && self.candidate_glsl_capable>=self.max_candidates {self.candidate_jobs.clear();return None}
        let mut job=self.candidate_jobs.pop();
        if let Some(candidate)=job.as_mut(){
            self.candidate_evaluated+=1;
            let entropy=std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d|d.as_nanos() as u64).unwrap_or(0);
            randomize_candidate_colors(&mut candidate.graph,
                entropy ^ (self.candidate_evaluated as u64).wrapping_mul(0xD6E8FEB86659FD93));
        }
        job
    }
    pub fn receive_candidate(&mut self,job:CandidateJob,rgba:Vec<u8>,width:usize,height:usize){
        if self.candidate_base.as_ref().is_none_or(|g|g.revision!=job.source_revision){return}
        let visual=visual_metrics(&rgba,width,height);
        if job.connection_candidate && (visual.contrast<0.018 || visual.entropy<0.055 || visual.interest<0.045) {
            self.candidate_rejections+=1;
            self.candidate_uniform+=1;
            crate::maker::operational_log(format_args!("[MAKER VISUAL REJECT] {} contrast={:.4} entropy={:.4} interest={:.4}",
                job.name,visual.contrast,visual.entropy,visual.interest));
            return;
        }
        // Mark only candidates with an actual callable-field wire, not merely
        // modules that advertise an always-available callable output.
        let uses_callable_field=job.graph.connections.iter().any(|connection| {
            job.graph.node(connection.from)
                .and_then(|node|node.output_port(connection.from_port))
                .is_some_and(|port|matches!(port.ty.as_str(),"field2d"|"field3d"))
        });
        let parameters=job.graph.nodes.iter().map(|n|(n.id,n.parameters.clone())).collect();
        let modules=job.graph.nodes.iter().map(|n|(n.id,n.module_id.clone())).collect();
        // Parameter discovery retains its existing first-500 behavior.
        // Connection discovery keeps evaluating, replacing weak/near-duplicate entries.
        let candidate=Candidate{name:job.name,parameters,modules,connections:job.graph.connections.clone(),
            texture:self.context.load_texture(format!("maker_candidate_{}",self.candidate_evaluated),
                egui::ColorImage::from_rgba_unmultiplied([width,height],&rgba),egui::TextureOptions::LINEAR),
            visual,motion:None,motion_failed:false,connection_candidate:job.connection_candidate,uses_callable_field};
        if self.candidates.len()<CANDIDATES_RETAINED {self.candidates.push(candidate);return}
        if !job.connection_candidate {return}
        // A compact, normalized descriptor gives diversity a role without adding
        // an image-processing dependency or retaining every thumbnail in memory.
        let mut closest=None;
        let mut lowest=None;
        for (index,existing) in self.candidates.iter().enumerate(){
            if !existing.connection_candidate || self.candidate_selected==Some(index){continue}
            let value=existing.visual.interest;
            if lowest.is_none_or(|(_,score)|value<score){lowest=Some((index,value));}
            // Compare inexpensive perceptual metrics for already retained entries.
            let distance=(visual.complexity-existing.visual.complexity).abs()
                +(visual.contrast-existing.visual.contrast).abs()
                +(visual.entropy-existing.visual.entropy).abs()
                +(visual.vibrancy-existing.visual.vibrancy).abs();
            if closest.is_none_or(|(_,d)|distance<d){closest=Some((index,distance));}
        }
        let victim=match (closest,lowest){
            (Some((index,d)),_) if d<0.12 => Some(index),
            (_,Some((index,_)))=>Some(index),
            _=>None,
        };
        if let Some(index)=victim {
            let old=&self.candidates[index];
            let similarity=(visual.complexity-old.visual.complexity).abs()
                +(visual.contrast-old.visual.contrast).abs()
                +(visual.entropy-old.visual.entropy).abs()
                +(visual.vibrancy-old.visual.vibrancy).abs();
            // Novelty can compensate for a modest interest deficit; near duplicates
            // must strictly improve on their predecessor.
            let improvement=if similarity<0.12 {0.005}else{-0.04};
            if visual.interest>old.visual.interest+improvement {self.candidates[index]=candidate;}
        }
    }
    fn reconstructed_candidate(&self,index:usize)->Result<Graph,String>{
        let base=self.candidate_base.as_ref().ok_or("Discovery snapshot missing")?;
        let candidate=self.candidates.get(index).ok_or("Candidate no longer retained")?;
        let mut graph=base.clone();
        if candidate.connection_candidate {
            graph.connections.clear();
            for c in &candidate.connections {
                graph.connect(c.from,c.from_port,c.to,c.to_port)?;
            }
        }
        // Both connection and parameter candidates retain their exact sampled colors.
        for node in &mut graph.nodes {
            if let Some(parameters)=candidate.parameters.get(&node.id){
                if candidate.connection_candidate {
                    for (id,value) in parameters {
                        if node.definition().parameters.iter().any(|p|p.id==*id && p.ty=="color3") {
                            node.parameters.insert(id.clone(),value.clone());
                        }
                    }
                } else {node.parameters=parameters.clone();}
            }
        }
        Ok(graph)
    }
    pub fn next_analysis_job(&mut self)->Option<(usize,Graph,String)>{
        let index=self.analysis_request.take()?;
        let name=self.candidates.get(index)?.name.clone();
        match self.reconstructed_candidate(index){
            Ok(graph)=>Some((index,graph,name)),
            Err(error)=>{self.analysis=Some(CandidateAnalysis{name,lines:vec![format!("Graph reconstruction failed: {error}")]});None}
        }
    }
    pub fn receive_analysis(&mut self,index:usize,analysis:CandidateAnalysis){
        if index<self.candidates.len() {self.analysis=Some(analysis);}
    }

    /// Reconstruct one retained candidate for GPU motion qualification.
    /// Connection candidates are measured regardless of the selected sort.
    pub fn next_motion_job(&self)->Option<(usize,Graph)> {
        let base=self.candidate_base.as_ref()?;
        for (index,candidate) in self.candidates.iter().enumerate() {
            if candidate.motion.is_some() || candidate.motion_failed {continue}
            // Connection candidates are qualified automatically; parameter
            // candidates retain their historical on-demand motion behavior.
            if !candidate.connection_candidate && self.candidate_sort!=6 {continue}
            let mut graph=base.clone();
            if candidate.connection_candidate {
                graph.connections.clear();
                let mut valid=true;
                for c in &candidate.connections {
                    if graph.connect(c.from,c.from_port,c.to,c.to_port).is_err(){valid=false;break}
                }
                if !valid {continue}
            } else {
                for node in &mut graph.nodes {
                    if let Some(parameters)=candidate.parameters.get(&node.id) {
                        node.parameters=parameters.clone();
                    }
                }
            }
            return Some((index,graph));
        }
        None
    }
    pub fn receive_motion_score(&mut self,index:usize,result:Result<f32,String>){
        if let Some(candidate)=self.candidates.get_mut(index){
            match result {
                Ok(score)=>{
                    candidate.motion=Some(score.clamp(0.0,1.0));
                    if candidate.connection_candidate {
                        crate::maker::operational_log(format_args!("[MAKER TEMPORAL QUALIFY] {} motion={:.5} status={}",
                            candidate.name,score,if score<0.003 {"static"} else {"animated"}));
                    }
                },
                Err(error)=>{candidate.motion_failed=true;crate::maker::operational_log(format_args!("[MAKER MOTION] {}: {}",candidate.name,error));}
            }
        }
    }

    pub fn candidate_glsl_capable(&mut self){self.candidate_glsl_capable+=1;}
    pub fn candidate_rejected_uniform(&mut self){self.candidate_rejections+=1;self.candidate_uniform+=1;}
    pub fn candidate_rejected_glsl(&mut self){self.candidate_rejections+=1;self.candidate_glsl_errors+=1;}
    pub fn candidate_rejected_render(&mut self){self.candidate_rejections+=1;self.candidate_render_errors+=1;}

    pub fn preview_rect(&self)->egui::Rect{self.preview_rect}
    pub fn preview_active(&self)->bool{self.preview_active}
    pub fn handle_event(&mut self, event: &Event) {
        // SDL mouse x/y are window-space logical coordinates, already in egui points.
        // Dividing by the drawable/window pixel ratio offsets hit testing on HiDPI.

        match event {
            Event::MouseMotion { x, y, .. } => {
                let next=egui::pos2(*x as f32,*y as f32);
                self.pointer = next;
                self.events.push(egui::Event::PointerMoved(self.pointer));
            }
            Event::MouseButtonDown { mouse_btn, x, y, .. } | Event::MouseButtonUp { mouse_btn, x, y, .. } => {
                let pressed = matches!(event, Event::MouseButtonDown { .. });
                let button = match mouse_btn { MouseButton::Left => egui::PointerButton::Primary,
                    MouseButton::Right => egui::PointerButton::Secondary,
                    MouseButton::Middle => egui::PointerButton::Middle, _ => return };
                self.pointer = egui::pos2(*x as f32, *y as f32);
                // Mango owns the middle mouse button. The graph navigates via
                // scrollbars and node-drag edge auto-pan, not mouse-button panning.
                if *mouse_btn == MouseButton::Middle { return; }
                self.events.push(egui::Event::PointerButton { pos: self.pointer, button, pressed, modifiers: self.modifiers });
            }
            Event::MouseWheel { x, y, .. } => {
                if self.canvas_rect.contains(self.pointer) && self.examine_module.is_none()
                    && self.glsl_export_dialog.is_none()
                    && !(self.parameter_popup.is_some() && self.parameter_wheel_rect.contains(self.pointer)) {
                    let delta = if self.modifiers.shift { *y as f32 + *x as f32 }
                                else { *y as f32 };
                    let axis=if self.modifiers.shift {0} else {1};
                    let proposed=if axis==0 {self.canvas_pan.x+delta*48.0}
                        else {self.canvas_pan.y+delta*48.0};
                    let visible=if axis==0 {self.canvas_rect.width()/self.canvas_zoom}
                        else {self.canvas_rect.height()/self.canvas_zoom};
                    let low=self.workspace_min[axis];
                    let upper=(self.workspace_max[axis]-visible).max(low);
                    // Wheel navigation respects the current finite workspace;
                    // no wheel input should expand its dimensions.
                    if upper>low {
                        let position=(-proposed/self.canvas_zoom).clamp(low,upper);
                        if axis==0 {self.canvas_pan.x=-position*self.canvas_zoom;}
                        else {self.canvas_pan.y=-position*self.canvas_zoom;}
                    }
                } else {
                    self.events.push(egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Line,
                        delta: egui::vec2(*x as f32, *y as f32), modifiers: self.modifiers,
                    });
                }
            }
            Event::Window { win_event: sdl2::event::WindowEvent::FocusLost, .. } => {}
            Event::TextInput { text, .. } => self.events.push(egui::Event::Text(text.clone())),
            Event::KeyDown { keycode: Some(key), keymod, repeat, .. } |
            Event::KeyUp { keycode: Some(key), keymod, repeat, .. } => {
                use sdl2::keyboard::{Keycode, Mod};
                self.modifiers = egui::Modifiers {
                    alt: keymod.intersects(Mod::LALTMOD | Mod::RALTMOD),
                    ctrl: keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD),
                    shift: keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD),
                    mac_cmd: false, command: keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD),
                };
                if *repeat { return; }
                if matches!(event,Event::KeyDown{..}) && self.modifiers.ctrl {
                    match *key {
                        Keycode::Equals | Keycode::Plus | Keycode::KpPlus => self.pending_zoom=Some(self.canvas_zoom*1.2),
                        Keycode::Minus | Keycode::KpMinus => self.pending_zoom=Some(self.canvas_zoom/1.2),
                        Keycode::Num0 | Keycode::Kp0 => self.pending_zoom=Some(1.0),
                        Keycode::F if self.modifiers.shift => self.pending_zoom_to_fit=true,
                        Keycode::N if !self.modifiers.shift => self.pending_menu_command=Some("new"),
                        Keycode::O if !self.modifiers.shift => self.pending_menu_command=Some("open"),
                        Keycode::S if self.modifiers.shift => self.pending_menu_command=Some("save_as"),
                        Keycode::S => self.pending_menu_command=Some("save"),
                        Keycode::C if self.modifiers.shift => self.pending_menu_command=Some("close_all"),
                        Keycode::C => self.pending_menu_command=Some("close"),
                        Keycode::X if self.modifiers.shift => self.pending_menu_command=Some("export_glsl_clipboard"),
                        Keycode::X => self.pending_menu_command=Some("export_glsl"),
                        Keycode::D if self.modifiers.shift => self.pending_menu_command=Some("delete"),
                        Keycode::Comma if self.modifiers.shift => self.pending_menu_command=Some("optimize"),
                        Keycode::Period if self.modifiers.shift => self.pending_menu_command=Some("remove_orphans"),
                        Keycode::Q if !self.modifiers.shift => self.pending_menu_command=Some("exit"),
                        _=>{}
                    }
                }
                let mapped = match *key {
                    Keycode::Tab => Some(egui::Key::Tab), Keycode::Return | Keycode::KpEnter => Some(egui::Key::Enter),
                    Keycode::Escape => Some(egui::Key::Escape), Keycode::Backspace => Some(egui::Key::Backspace),
                    Keycode::Delete => Some(egui::Key::Delete), Keycode::Space => Some(egui::Key::Space),
                    Keycode::Left => Some(egui::Key::ArrowLeft), Keycode::Right => Some(egui::Key::ArrowRight),
                    Keycode::Up => Some(egui::Key::ArrowUp), Keycode::Down => Some(egui::Key::ArrowDown), _ => None,
                };
                if let Some(key) = mapped { self.events.push(egui::Event::Key {
                    key, physical_key: None, pressed: matches!(event, Event::KeyDown { .. }),
                    repeat: false, modifiers: self.modifiers }); }
            }
            _ => {}
        }
    }
    pub fn frame(&mut self,dw:u32,dh:u32,ww:u32,wh:u32,scale:f32,compile_error:Option<&str>,resolution_warning:Option<&str>)->bool{
        // CPU-only exploration is incremental. OpenGL thumbnails remain on the caller's GL thread.
        if let Some(search)=self.connection_search.as_mut(){
            if let Some(base)=self.candidate_base.as_ref(){
                search.advance(base.revision,&mut self.candidate_jobs);
            }else{self.connection_search=None;}
        }
        // Each bounded canonical DFS batch is followed by bounded GPU work.
        // Resume the saved canonical checkpoint only after every reply from the
        // previous batch has been accounted for by the UI thread.
        if self.connection_search.as_ref().is_some_and(|s|s.finished()) && self.candidate_jobs.is_empty()
            && self.candidate_evaluated==self.candidate_glsl_capable+self.candidate_glsl_errors+self.candidate_render_errors {
            if let Some(mut search)=self.connection_search.take(){
                if search.discovery_error.is_some(){
                    self.candidate_notice=search.discovery_error.take().unwrap();
                } else if !search.gpu_started {
                    // The UI below schedules this batch's jobs. Do not advance yet.
                    self.connection_search=Some(search);
                } else if self.candidate_glsl_capable>=self.max_candidates {
                    self.candidate_notice=format!("GLSL target reached: {} / {} capable; {} thumbnails retained",self.candidate_glsl_capable,self.max_candidates,self.candidates.len());
                    crate::maker::operational_log(format_args!("[MAKER DISCOVERY TARGET] reached capable={} retained={}",self.candidate_glsl_capable,self.candidates.len()));
                } else if search.examined<search.max_enumerations {
                    self.candidate_notice=format!("Canonical enumeration exhausted after {} GLSL-capable candidates",self.candidate_glsl_capable);
                    crate::maker::operational_log(format_args!("[MAKER DISCOVERY TARGET] exhausted capable={}",self.candidate_glsl_capable));
                } else if search.generated==0 && search.examined==0 {
                    self.candidate_notice="Discovery stopped: no progress in last batch".into();
                } else if let Some(base)=self.candidate_base.as_ref(){
                    self.candidate_notice=format!("GLSL-capable: {} / {}; resuming canonical checkpoint",self.candidate_glsl_capable,self.max_candidates);
                    self.connection_search=ConnectionSearch::new(base,self.max_enumerations,self.min_wire_connections,self.max_wire_connections);
                }
            }
        }
        let scale=scale.max(0.1);
        let mut input=egui::RawInput::default();
        // SDL reports mouse positions in WINDOW coordinates. egui points use the
        // same coordinate space; pixels_per_point only scales rasterization.
        // Do not derive the logical viewport from drawable pixels.
        input.screen_rect=Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(ww as f32,wh as f32)));
        input.viewports.insert(egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(scale),..Default::default()});
        input.time=Some(self.start.elapsed().as_secs_f64());input.modifiers=self.modifiers;
        input.events=std::mem::take(&mut self.events);
        let old_parameters:Vec<_>=self.graph.nodes.iter().map(|n|(n.id,n.parameters.clone())).collect();
        let tab_labels:Vec<String> = (0..if self.project_open {self.inactive_tabs.len()+1} else {0}).map(|i| {
            if i==self.active_tab {self.project_name.clone()}
            else {let slot=if i<self.active_tab{i}else{i-1};self.inactive_tabs[slot].project_name.clone()}
        }).collect();
        let thumbnail_tab_index = if self.project_open && self.thumbnail_workspace {
            Some(self.active_tab)
        } else {
            self.inactive_tabs.iter().position(|tab| tab.thumbnail_workspace)
                .map(|index| if index < self.active_tab {index} else {index+1})
        };
        let active_tab_index=self.active_tab;
        let project_tab_scroll_x=&mut self.project_tab_scroll_x;
        let has_active_project=self.project_open;
        let mut saved_thumbnail_this_frame=false;
        let mut requested_tab=None;
        let mut requested_new_tab=false;
        let mut requested_start_close_all=false;
        let mut requested_candidate_tab:Option<Graph>=None;
        let mut requested_close_tab=false;
        let mut requested_close_tab_index:Option<usize>=None;
        let mut requested_open_project:Option<(std::path::PathBuf,Graph)>=None;
        let project_name=&mut self.project_name;
        let thumbnail_workspace=&mut self.thumbnail_workspace;
        let thumbnail_hash=&mut self.thumbnail_hash;
        let project_dialog=&mut self.project_dialog;
        let examine_module=&mut self.examine_module;
        let glsl_export_dialog=&mut self.glsl_export_dialog;
        let last_glsl_curator=&mut self.last_glsl_curator;
        let project_path=&mut self.project_path;
        let recent_projects=&mut self.recent_projects;
        let saved_graph_snapshot=&mut self.saved_graph_snapshot;
        let unsaved_action=&mut self.unsaved_action;
        let close_all_running=&mut self.close_all_running;
        let after_save_action=&mut self.after_save_action;
        let discard_approved=&mut self.discard_approved;
        let graph=&mut self.graph;
        let selected=&mut self.selected;
        let parameter_popup=&mut self.parameter_popup;
        let parameter_wheel_rect=&mut self.parameter_wheel_rect;
        let pending=&mut self.pending_source;
        let dragging_wire=&mut self.dragging_wire;
        let menu_icon=&self.menu_icon;
        let branding_texture=&self.branding_texture;
        let branding_aspect_ratio=self.branding_aspect_ratio;
        let status=&mut self.status;
        let open=&mut self.open;
        let preview_rect=&mut self.preview_rect;
        let preview_height=&mut self.preview_height;
        let preview_active=&mut self.preview_active;
        let routes=&mut self.routes;
        let canvas_pan=&mut self.canvas_pan;
        let canvas_zoom=&mut self.canvas_zoom;
        let pending_zoom=&mut self.pending_zoom;
        let pending_zoom_to_fit=&mut self.pending_zoom_to_fit;
        let pending_menu_command=&mut self.pending_menu_command;
        let pending_tools_action=&mut self.pending_tools_action;
        let canvas_rect=&mut self.canvas_rect;
        let pinpoint_next=&mut self.pinpoint_next;
        let pinpoint_target=&mut self.pinpoint_target;
        let pinpoint_pulse=&mut self.pinpoint_pulse;
        let pinpoint_boxes=&mut self.pinpoint_boxes;
        let pointer_position=self.pointer;
        let workspace_min=&mut self.workspace_min;
        let workspace_max=&mut self.workspace_max;
        let routing_undo=&mut self.routing_undo;
        let routing_redo=&mut self.routing_redo;
        let candidate_mode=&mut self.candidate_mode;
        let candidate_sort=&mut self.candidate_sort;
        let max_wire_connections=&mut self.max_wire_connections;
        let min_wire_connections=&mut self.min_wire_connections;
        let max_enumerations=&mut self.max_enumerations;
        let max_candidates=&mut self.max_candidates;
        let candidates=&mut self.candidates;
        let candidate_jobs=&mut self.candidate_jobs;
        let candidate_epoch=&mut self.candidate_epoch;
        let candidate_base=&mut self.candidate_base;
        let candidate_selected=&mut self.candidate_selected;
        let candidate_undo=&mut self.candidate_undo;
        let orphan_undo=&mut self.orphan_undo;
        let orphan_redo=&mut self.orphan_redo;
        let delete_project_confirm=&mut self.delete_project_confirm;
        let orphan_restore_pending=&mut self.orphan_restore_pending;
        let candidate_notice=&mut self.candidate_notice;
        let analysis_request=&mut self.analysis_request;
        let analysis=&mut self.analysis;
        let pending_clipboard=&mut self.pending_clipboard;
        let pending_clipboard_is_glsl=&mut self.pending_clipboard_is_glsl;
        let pending_clipboard_is_module=&mut self.pending_clipboard_is_module;
        let pending_glsl_export=&mut self.pending_glsl_export;
        let candidate_uniform=&mut self.candidate_uniform;
        let candidate_glsl_errors=&mut self.candidate_glsl_errors;
        let candidate_render_errors=&mut self.candidate_render_errors;
        let connection_search=&mut self.connection_search;
        let candidate_glsl_capable=&mut self.candidate_glsl_capable;
        let reset_enumeration_confirm=&mut self.reset_enumeration_confirm;
        let candidate_evaluated=&mut self.candidate_evaluated;
        let candidate_rejections=&mut self.candidate_rejections;
        let library_expanded=&mut self.library_expanded;
        let library_search=&mut self.library_search;
        let mut optimize_requested=false;
        let mut fit_requested=false;
        let mut editor_anchor: Option<(NodeId, egui::Pos2)> = None;
        let output=self.context.run(input,|ctx|{
            if *reset_enumeration_confirm {
                egui::Window::new("Reset Enumeration?").collapsible(false).resizable(false).show(ctx,|ui|{
                    ui.label(format!("Reset enumeration for {}–{} wire connections?", *min_wire_connections,*max_wire_connections));
                    ui.label("Only the checkpoint for this wire range will be discarded.");
                    ui.label("Saved projects, exported shaders and module definitions will not be deleted.");
                    ui.horizontal(|ui|{
                        if ui.button("Cancel").clicked(){*reset_enumeration_confirm=false;}
                        if ui.button("Reset Enumeration").clicked(){
                            // The enumeration is advanced synchronously on the UI thread.
                            // Epoch invalidation discards replies from older GPU requests.
                            *connection_search=None;
                            *candidate_epoch=candidate_epoch.wrapping_add(1);
                            candidate_jobs.clear();
                            *candidate_base=None;
                            *candidate_selected=None;
                            *candidate_evaluated=0;
                            *candidate_glsl_capable=0;
                            *candidate_rejections=0;
                            *candidate_uniform=0;
                            *candidate_glsl_errors=0;
                            *candidate_render_errors=0;
                            *analysis_request=None;
                            *analysis=None;
                            match maker_discovery::ProductionEnumeration::reset_checkpoint_for_range(*min_wire_connections,*max_wire_connections){
                                Ok(())=>*candidate_notice=format!("Enumeration reset for {}–{} wire connections. Click Start Discovery to begin again; other checkpoints and thumbnails are preserved.", *min_wire_connections,*max_wire_connections),
                                Err(error)=>*candidate_notice=format!("Enumeration reset failed: {error}"),
                            }
                            *reset_enumeration_confirm=false;
                        }
                    });
                });
            }
            if let Some(browser)=project_dialog.as_mut() {
                let mut close=false;
                let mut completed:Option<(std::path::PathBuf,Option<Graph>)>=None;
                egui::Window::new(if browser.save{"Save Maker Project"}else{"Open Maker Project"})
                    .id(egui::Id::new("maker_project_file_browser"))
                    // The Inspector and scrollbar widgets intentionally use Foreground
                    // layers to prevent node hit-test interception. The project browser
                    // must share that order and be raised after those controls, or they
                    // will be painted over its window and receive its pointer events.
                    .order(egui::Order::Foreground)
                    .collapsible(false).resizable(true).default_width(650.0).min_width(480.0)
                    .show(ctx,|ui|{
                        ui.label("Browse to a folder, select a .smprj project, or enter a filename.");
                        ui.horizontal(|ui|{
                            if ui.button("Home").clicked(){if let Ok(p)=maker_projects_dir(){browser.change_directory(p);}}
                            if ui.button("Up").clicked(){if let Some(p)=browser.directory.parent(){browser.change_directory(p.to_path_buf());}}
                            if ui.button("Refresh").clicked(){browser.error=None;}
                        });
                        ui.horizontal(|ui|{
                            ui.label("Folder:");
                            let mut typed=browser.directory.to_string_lossy().into_owned();
                            let response=ui.add(egui::TextEdit::singleline(&mut typed).desired_width(f32::INFINITY));
                            let enter=response.lost_focus() && ui.input(|i|i.key_pressed(egui::Key::Enter));
                            if ui.button("Go").clicked() || enter {
                                browser.change_directory(std::path::PathBuf::from(typed));
                            }
                        });
                        ui.separator();
                        let mut open_on_double_click = false;
                        match maker_browser_entries(&browser.directory){
                            Ok(entries)=>{
                                egui::ScrollArea::vertical().max_height(310.0).show(ui,|ui|{
                                    for (name,is_dir) in entries {
                                        let label=if is_dir{format!("[Folder] {name}")}else{name.clone()};
                                        let selected=!is_dir && browser.filename==name;
                                        let response=ui.selectable_label(selected,label);
                                        if response.clicked(){
                                            if is_dir{browser.change_directory(browser.directory.join(&name));}
                                            else{browser.filename=name.clone();browser.overwrite=None;browser.error=None;}
                                        }
                                        if response.double_clicked() && !is_dir {
                                            browser.filename=name;
                                            browser.overwrite=None;
                                            browser.error=None;
                                            // Opening a file uses the same validation and loading path as Open.
                                            // Save dialogs keep their explicit confirmation/overwrite workflow.
                                            if !browser.save { open_on_double_click = true; }
                                        }
                                    }
                                });
                            }
                            Err(e)=>{ui.colored_label(egui::Color32::LIGHT_RED,format!("Cannot list folder: {e}"));}
                        }
                        ui.separator();
                        ui.horizontal(|ui|{
                            ui.label("File name:");
                            ui.add(egui::TextEdit::singleline(&mut browser.filename).desired_width(400.0));
                        });
                        if let Some(path)=browser.overwrite.clone(){
                            ui.colored_label(egui::Color32::YELLOW,format!("Overwrite existing project {}?",path.display()));
                            ui.horizontal(|ui|{
                                if ui.button("Confirm Overwrite").clicked(){
                                    match maker_save_project(graph,&path){
                                        Ok(())=>completed=Some((path,None)),
                                        Err(e)=>browser.error=Some(format!("Save failed: {e}")),
                                    }
                                }
                                if ui.button("Keep Existing").clicked(){browser.overwrite=None;}
                            });
                        }else{
                            ui.horizontal(|ui|{
                                let open_button_clicked = ui.button(if browser.save{"Save"}else{"Open"}).clicked();
                                if open_button_clicked || open_on_double_click {
                                    match browser.selected_path(){
                                        Ok(path)=>{
                                            if browser.save {
                                                if path.exists(){browser.overwrite=Some(path);}
                                                else {match maker_save_project(graph,&path){
                                                    Ok(())=>completed=Some((path,None)),
                                                    Err(e)=>browser.error=Some(format!("Save failed: {e}")),
                                                }}
                                            }else{
                                                match maker_load_project(&path){
                                                    Ok(loaded)=>completed=Some((path,Some(loaded))),
                                                    Err(e)=>browser.error=Some(format!("Open failed: {e}")),
                                                }
                                            }
                                        }
                                        Err(e)=>browser.error=Some(e),
                                    }
                                }
                                if ui.button("Cancel").clicked(){close=true;}
                            });
                        }
                        if let Some(e)=&browser.error{ui.colored_label(egui::Color32::LIGHT_RED,e);}
                    });
                let did_complete=completed.is_some();
                if let Some((path,loaded))=completed {
                    let was_open=loaded.is_some();
                    if let Some(loaded)=loaded {
                        requested_open_project=Some((path.clone(),loaded));
                        *status=format!("Opening project {}",path.display());
                    }else{
                        *saved_graph_snapshot=serde_json::json!(crate::maker::graph_to_wire(graph));
                        if *thumbnail_workspace { saved_thumbnail_this_frame=true; }
                        *thumbnail_workspace=false;
                        *thumbnail_hash=None;
                        *status=format!("Saved project {}",path.display());
                        if let Some(action)=after_save_action.take(){*pending_menu_command=Some(action);}
                    }
                    if !was_open {*project_name=path.file_stem().and_then(|s|s.to_str()).unwrap_or("Untitled").to_owned();}
                    if was_open {
                        match maker_record_recent_project(&path){
                            Ok(updated)=>*recent_projects=updated,
                            Err(e)=>*status=format!("Opened project {}, but recent-project history could not be saved: {e}",path.display()),
                        }
                    }
                    if !was_open {*project_path=Some(path);}
                    close=true;
                }
                if close{
                    if !did_complete { if *after_save_action==Some("close_all") {*close_all_running=false;} *after_save_action=None; }
                    *project_dialog=None;
                }
            }
            // Size the dock from the longest displayed module label, including the
            // "+ " prefix. All category buttons share this single width.
            let library_button_width = {
                let font_id = egui::TextStyle::Button.resolve(&ctx.style());
                let painter = ctx.debug_painter();
                maker_library::get().modules.values()
                    .map(|module| {
                        let label = format!("+ {}", module.name);
                        painter.layout_no_wrap(label, font_id.clone(), egui::Color32::WHITE).size().x
                    })
                    .fold(0.0_f32, f32::max)
                    + 24.0 // Button horizontal padding.
            };
            // Category indentation, panel margins and a little breathing room.
            let pinpoint_column_width = 25.0_f32;
            let library_dock_width = library_button_width + pinpoint_column_width + 48.0;
            if *delete_project_confirm {
                egui::Window::new("Delete Maker Project").collapsible(false).resizable(false).show(ctx,|ui|{
                    if let Some(path)=project_path.clone(){
                        ui.label(format!("Permanently delete project file {}?",path.display()));
                        ui.label("The current graph will remain open.");
                        ui.horizontal(|ui|{
                            if ui.button("Delete Project").clicked(){
                                match std::fs::remove_file(&path) {
                                    Ok(())=>{*status=format!("Deleted project {}",path.display());*project_path=None;*project_name="Untitled".into();},
                                    Err(e)=>*status=format!("Delete failed: {e}"),
                                }
                                *delete_project_confirm=false;
                            }
                            if ui.button("Cancel").clicked(){*delete_project_confirm=false;}
                        });
                    }else{*delete_project_confirm=false;}
                });
            }
            egui::TopBottomPanel::bottom("maker_footer").show(ctx,|ui|{
                ui.horizontal_wrapped(|ui|{
                    ui.label(format!("{}.smprj",project_name));ui.separator();
                    ui.label("Batch 07 · field traversal + anchored parameter editor");ui.separator();
                    ui.label(format!("{} nodes / {} connections",graph.nodes.len(),graph.connections.len()));ui.separator();
                    ui.monospace(format!("Zoom: {:.0}%", *canvas_zoom * 100.0))
                        .on_hover_text("Current Node Graph viewport zoom (100% = actual graph scale)");
                    ui.separator();
                    // Graph coordinates use logical graph pixels: they are stable
                    // across viewport zoom and SDL display scaling. The cursor
                    // is reported only while it is inside the graph canvas.
                    if canvas_rect.is_positive() && canvas_rect.contains(pointer_position) {
                        let graph_position=(pointer_position-canvas_rect.min-*canvas_pan)/ *canvas_zoom;
                        ui.monospace(format!("Cursor X: {:.0}  Y: {:.0} px",graph_position.x,graph_position.y));
                    } else {
                        ui.monospace("Cursor X: —  Y: — px");
                    }
                    ui.separator();
                    // Workspace extents, not the bounding box of placed nodes:
                    // users can scroll into empty space and extend these limits.
                    let graph_width=(workspace_max[0]-workspace_min[0]).max(0.0);
                    let graph_height=(workspace_max[1]-workspace_min[1]).max(0.0);
                    ui.monospace(format!("Graph X: {:.0}  Y: {:.0} px",graph_width,graph_height))
                        .on_hover_text("Current expandable graph workspace width × height in logical graph pixels; expands as you navigate");
                    ui.separator();
                    if let Some(e)=compile_error{ui.colored_label(egui::Color32::LIGHT_RED,format!("GLSL: {}",e));}
                    else if let Some(warning)=resolution_warning{ui.colored_label(egui::Color32::from_rgb(255,190,70),warning);}
                    else{ui.label(status.as_str());}
                });
            });
            egui::SidePanel::left("maker_library")
                .resizable(false)
                .default_width(library_dock_width)
                .min_width(library_dock_width)
                .max_width(library_dock_width)
                .show(ctx,|ui|{
                ui.heading("Screenshaver Maker");
                ui.label(egui::RichText::new("Module Library").heading());
                ui.separator();
                ui.horizontal(|ui|{
                    ui.small(format!("{} modules", maker_library::get().modules.len()));
                    // Bulk addition excludes Universal Test and never duplicates modules
                    // already represented in the graph. Availability follows graph state.
                    let modules:Vec<String>=maker_library::get().modules.values()
                        .filter(|m|!m.terminal && m.id!="screenshaver.test.universal_test")
                        .filter(|m|!graph.nodes.iter().any(|n|n.module_id==m.id))
                        .map(|m|m.id.clone()).collect();
                    let can_add_all=!modules.is_empty();
                    if ui.add_enabled(can_add_all,egui::Button::new("+ Add All Modules"))
                        .on_hover_text("Add missing modules (excluding Universal Test) without connecting wires")
                        .clicked(){
                    let mut added=0usize;
                    for module_id in modules {
                        let index=graph.nodes.len().saturating_sub(1);
                        let col=index%4;
                        let row=index/4;
                        if graph.add(&module_id,[40.0+col as f32*265.0,40.0+row as f32*200.0]).is_ok(){added+=1;}
                    }
                    *selected=None;*parameter_popup=None;*pending=None;*dragging_wire=None;
                    routes.clear();routing_undo.clear();routing_redo.clear();orphan_undo.clear();orphan_redo.clear();*orphan_restore_pending=false;
                    candidates.clear();*candidate_epoch=candidate_epoch.wrapping_add(1);candidate_jobs.clear();*connection_search=None;*candidate_base=None;
                    candidate_undo.clear();*candidate_selected=None;candidate_notice.clear();
                    *status=format!("Added {added} modules without connections");
                    }
                });
                // Search is a temporary view: category expansion state is never changed
                // by entering, editing, or clearing the query.
                ui.horizontal(|ui| {
                    let available = (ui.available_width() - 24.0).max(80.0);
                    ui.add_sized([available, ui.spacing().interact_size.y],
                        egui::TextEdit::singleline(library_search).hint_text("Search modules..."));
                    if ui.add_enabled(!library_search.is_empty(), egui::Button::new("✕"))
                        .on_hover_text("Clear search").clicked() {
                        library_search.clear();
                    }
                });
                let search_term=library_search.trim().to_lowercase();
                let searching=!search_term.is_empty();
                ui.add_enabled_ui(!searching, |ui| {
                ui.horizontal(|ui|{
                    if ui.small_button("Expand All").clicked(){
                        for category in maker_library::get().categories.values(){
                            library_expanded.insert(category.clone(),true);
                        }
                    }
                    if ui.small_button("Contract All").clicked(){
                        for category in maker_library::get().categories.values(){
                            library_expanded.insert(category.clone(),false);
                        }
                    }
                });
                }); // Preserve expansion settings while search is active.
                ui.small("Add nodes to the graph:");
                // Keep the category list scrollable within the library panel.
                // Without a ScrollArea, lower categories can be clipped and
                // their buttons cannot receive pointer input.
                egui::ScrollArea::vertical()
                    .id_source("maker_library_categories_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                // Group by the actual directory of each .smmod definition.
                // Sort categories and visible module names independently.
                let library=maker_library::get();
                let mut grouped:BTreeMap<String,Vec<&Module>>=BTreeMap::new();
                for module in library.modules.values(){
                    if searching && !module.name.to_lowercase().contains(&search_term) {continue}
                    if module.terminal && graph.nodes.iter().any(|n|n.definition().terminal){continue}
                    let category=library.categories.get(&module.id).map(String::as_str).unwrap_or("Other");
                    grouped.entry(category.to_owned()).or_default().push(module);
                }
                // Project-local unresolved definitions remain discoverable even when their
                // .smmod file is missing or invalid. They are not addable modules.
                for node in &graph.nodes {
                    if !node.unresolved(){continue}
                    if searching && !node.display_name().to_lowercase().contains(&search_term)
                        && !node.module_id.to_lowercase().contains(&search_term){continue}
                    let category="Unresolved".to_owned();
                    let list=grouped.entry(category).or_default();
                    if !list.iter().any(|m|m.id==node.module_id){list.push(node.definition());}
                }
                // Advertise community module support even when no local modules are installed.
                grouped.entry("third-party".to_owned()).or_default();
                // Size all category headers to the longest category caption,
                // independent of the module-button and dock widths.
                let category_header_font = egui::TextStyle::Button.resolve(ui.style());
                let category_header_width = grouped.keys().map(|category| {
                    let display = category.split(|c: char| c == '_' || c == '-' || c == ' ')
                        .filter(|part| !part.is_empty())
                        .map(|part| {
                            let mut chars = part.chars();
                            match chars.next() {
                                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                                None => String::new(),
                            }
                        }).collect::<Vec<_>>().join(" ");
                    ["▶", "▼"].iter().map(|arrow| {
                        ui.painter().layout_no_wrap(
                            format!("{} {}", arrow, display),
                            category_header_font.clone(),
                            egui::Color32::WHITE,
                        ).size().x
                    }).fold(0.0_f32, f32::max)
                }).fold(0.0_f32, f32::max) + 24.0;
                for (category,modules) in &mut grouped {
                    modules.sort_by(|a,b|a.name.to_lowercase().cmp(&b.name.to_lowercase()).then_with(||a.id.cmp(&b.id)));
                    // Display capitalization does not change category paths or IDs.
                    let display=category.split(|c:char|c=='_' || c=='-' || c==' ')
                        .filter(|part|!part.is_empty())
                        .map(|part|{
                            let mut chars=part.chars();
                            match chars.next(){Some(first)=>format!("{}{}",first.to_uppercase(),chars.as_str()),None=>String::new()}
                        }).collect::<Vec<_>>().join(" ");
                    let stored_expanded=library_expanded.entry(category.clone()).or_insert(false);
                    let expanded=searching || *stored_expanded;
                    // The category header and its module buttons share one palette entry.
                    let category_color = if category == "Unresolved" {
                        egui::Color32::from_rgb(235, 20, 35)
                    } else {
                        maker_category_color(category)
                    };
                    let header_text = format!("{} {}", if expanded {"▼"} else {"▶"}, display);
                    // Use an empty native button and paint its caption at a fixed left inset.
                    // This avoids egui's default centered Button text alignment while
                    // preserving the uniform header width and native button interaction.
                    let header_response = ui.add_sized(
                        [category_header_width, ui.spacing().interact_size.y],
                        egui::Button::new("").fill(category_color),
                    );
                    ui.painter().text(
                        header_response.rect.left_center() + egui::vec2(8.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        header_text,
                        egui::TextStyle::Button.resolve(ui.style()),
                        egui::Color32::WHITE,
                    );
                    if header_response.clicked() && !searching {*stored_expanded=!*stored_expanded;}
                    if expanded {ui.indent(("maker_library_category",category.as_str()),|ui|{
                        if category == "third-party" && modules.is_empty() {
                            ui.small("No third-party modules installed");
                        }
                        for module in modules.iter().copied(){
                    // Each category owns its own aligned pinpoint column. The
                    // indicator is interactive only when this type is in the graph.
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        let occurrences: Vec<NodeId> = graph.nodes.iter()
                            .filter(|node| node.module_id == module.id)
                            .map(|node| node.id).collect();
                        let is_unresolved=graph.nodes.iter().any(|n|n.module_id==module.id&&n.unresolved());
                        if occurrences.is_empty() {
                            ui.allocate_exact_size(
                                egui::vec2(pinpoint_column_width, ui.spacing().interact_size.y),
                                egui::Sense::hover());
                        } else {
                            // Draw the pinpoint indicator ourselves instead of relying on
                            // emoji fonts, whose glyph colors vary by platform/theme.
                            let (indicator_rect, check) = ui.allocate_exact_size(
                                egui::vec2(pinpoint_column_width, ui.spacing().interact_size.y),
                                egui::Sense::click(),
                            );
                            let check = check.on_hover_text(format!(
                                "Locate {} in graph ({} instance{})",
                                module.name, occurrences.len(), if occurrences.len()==1 {""} else {"s"}
                            ));
                            let side = 16.0_f32.min(indicator_rect.width()).min(indicator_rect.height());
                            let square = egui::Rect::from_center_size(
                                indicator_rect.center(), egui::vec2(side, side),
                            );
                            let indicator_color=if is_unresolved {
                                if check.hovered(){egui::Color32::from_rgb(225,62,62)}else{egui::Color32::from_rgb(190,35,35)}
                            }else if check.hovered(){egui::Color32::from_rgb(34,185,68)}
                            else{egui::Color32::from_rgb(24,155,52)};
                            ui.painter().rect_filled(square, 3.0, indicator_color);
                            let p = |x: f32, y: f32| {
                                square.min + egui::vec2(x * side, y * side)
                            };
                            let stroke = egui::Stroke::new(2.0, egui::Color32::WHITE);
                            if is_unresolved {
                                ui.painter().line_segment([p(0.27,0.27),p(0.73,0.73)],stroke);
                                ui.painter().line_segment([p(0.73,0.27),p(0.27,0.73)],stroke);
                            }else{
                                ui.painter().line_segment([p(0.22, 0.52), p(0.43, 0.73)], stroke);
                                ui.painter().line_segment([p(0.43, 0.73), p(0.80, 0.29)], stroke);
                            }
                            if check.clicked() {
                                let previous = pinpoint_next.get(&module.id).copied();
                                let target = previous.and_then(|id| occurrences.iter().copied().find(|next| *next > id))
                                    .unwrap_or(occurrences[0]);
                                pinpoint_next.insert(module.id.clone(), target);
                                *pinpoint_target = Some(target);
                                *pinpoint_pulse = Some((target, Instant::now()));
                                *selected = Some(target);
                                *status = format!("Located {} #{} ({} instance{})",
                                    module.name, target, occurrences.len(), if occurrences.len()==1 {""} else {"s"});
                            }
                        }
                        // Reuse the node-header palette so library and graph match.
                        let add_response = ui.add_sized(
                            [library_button_width, ui.spacing().interact_size.y],
                            egui::Button::new("").fill(node_header_color(module)),
                        ).on_hover_ui(|ui| {
                            ui.set_max_width(350.0);
                            if is_unresolved {
                                ui.strong(format!("{} (unknown)",module.name));
                                ui.label(format!("Module ID: {}",module.id));
                                ui.colored_label(egui::Color32::LIGHT_RED,"Module definition missing or invalid; saved graph data retained");
                            }else{module_library_tooltip(ui, module);}
                        });
                        ui.painter().text(
                            add_response.rect.left_center() + egui::vec2(8.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            format!("{} {}",if is_unresolved {"!"}else{"+"},module.name),
                            egui::TextStyle::Button.resolve(ui.style()),
                            egui::Color32::WHITE,
                        );
                    if add_response.clicked() && !is_unresolved{
                        let count=graph.nodes.len();
                        match graph.add(&module.id,[80.0+(count%3) as f32*45.0,180.0+(count%4) as f32*42.0]){
                            Ok(id)=>{*selected=Some(id);*status=format!("Added {} #{}",module.name,id);},
                            Err(e)=>*status=e,
                        }
                    }
                    }); // module row
                        }
                    });}
                }
                    }); // category scroll area
                ui.separator();ui.small("Drag from an output port to a compatible input, or click output then input. Outputs support fan-out; each input has one source.");
                if let Some((id,port))=*pending{ui.label(format!("Source: node #{id}, port {port}"));if ui.button("Cancel connection").clicked(){*pending=None;*dragging_wire=None;}}
            });
            if let Some(action)=*unsaved_action {
                egui::Window::new("Unsaved Project Changes")
                    .id(egui::Id::new("maker_unsaved_changes"))
                    .order(egui::Order::Foreground)
                    .collapsible(false).resizable(false).default_width(390.0)
                    .anchor(egui::Align2::CENTER_CENTER,egui::Vec2::ZERO)
                    .show(ctx,|ui|{
                        ui.label(format!("Save changes to {}.smprj before continuing?",project_name));
                        ui.label("Discard will lose changes made since the last save.");
                        ui.horizontal(|ui|{
                            if ui.button("Save").clicked(){
                                *unsaved_action=None;
                                *after_save_action=Some(action);
                                if project_path.is_some(){*pending_menu_command=Some("save");}
                                else {*project_dialog=Some(MakerProjectBrowser::open(true,None,thumbnail_hash.as_deref().unwrap_or(project_name)));}
                            }
                            if ui.button("Discard").clicked(){
                                *unsaved_action=None;
                                // Close All owns its own sequential state machine.
                                // Do not requeue it through the File command handler:
                                // that handler consumes discard_approved before
                                // advance_close_all() can close this project.
                                if action != "close_all" {
                                    *pending_menu_command=Some(action);
                                }
                                // One-shot bypass: the graph remains dirty until the
                                // chosen action actually completes.
                                *discard_approved=Some(action);
                            }
                            if ui.button("Cancel").clicked(){*unsaved_action=None; if action=="close_all" {*close_all_running=false;} }
                        });
                    });
            }
            if let Some((directory,filename,confirm_overwrite,curator,clipboard,export_format))=glsl_export_dialog.as_mut(){
                let mut close_export=false;
                egui::Window::new(if *clipboard {"Export GLSL Code"} else {"Export GLSL File"})
                    .id(egui::Id::new("maker_export_glsl"))
                    .order(egui::Order::Foreground)
                    .collapsible(false).resizable(false).default_width(440.0)
                    .anchor(egui::Align2::CENTER_CENTER,egui::Vec2::ZERO)
                    .show(ctx,|ui|{
                        ui.label("Export the current Node Graph as a GLSL shader:");
                        let previous_format=*export_format;
                        ui.label("Export format:");
                        for target in ExportFormat::ALL {
                            ui.radio_value(export_format, target, target.label());
                        }
                        if previous_format!=*export_format {
                            *confirm_overwrite=false;
                            if !*clipboard {
                                let path=std::path::Path::new(filename.as_str());
                                *filename=path.with_extension(export_format.extension()).to_string_lossy().into_owned();
                            }
                        }
                        if *export_format!=ExportFormat::Native {
                            ui.label("External host compatibility depends on GLSL features; not yet runtime verified.");
                        }
                        if !*clipboard {
                        ui.label("Export directory:");
                        if ui.add(egui::TextEdit::singleline(directory).desired_width(410.0)).changed(){
                            *confirm_overwrite=false;
                        }
                        ui.label("Filename:");
                        if ui.add(egui::TextEdit::singleline(filename).desired_width(410.0)).changed(){
                            *confirm_overwrite=false;
                        }
                        }
                        ui.label("Curated by:");
                        ui.add(egui::TextEdit::singleline(curator).desired_width(410.0));
                        ui.label("Optional composition attribution.");
                        if *confirm_overwrite {
                            ui.colored_label(egui::Color32::YELLOW,"File exists. Replace it?");
                        }
                        ui.horizontal(|ui|{
                            if ui.button(if *clipboard {"Copy GLSL"} else if *confirm_overwrite {"Confirm Overwrite"} else {"Export"}).clicked(){
                                if *clipboard {
                                    match crate::maker_compiler::fragment_source_for_export(graph,curator,*export_format){
                                        Ok(source)=>{
                                            *pending_clipboard=Some(source);
                                            *pending_clipboard_is_glsl=true;
                                            *last_glsl_curator=curator.clone();
                                            *status="Copying active project GLSL to clipboard".into();
                                            close_export=true;
                                        }
                                        Err(e)=>*status=format!("GLSL generation failed: {e}"),
                                    }
                                }else{
                                let name=filename.trim();
                                let dir=directory.trim();
                                let valid_name=!name.is_empty()
                                    && name!="." && name!=".."
                                    && !name.contains('/') && !name.contains('\\')
                                    && std::path::Path::new(name).extension().and_then(|v|v.to_str())==Some(export_format.extension());
                                let path=std::path::PathBuf::from(dir).join(name);
                                if dir.is_empty() {
                                    *status="Export directory cannot be empty".into();
                                }else if !valid_name {
                                    *status=format!("Export filename must be a .{} filename without directory separators",export_format.extension());
                                }else if path.exists() && !*confirm_overwrite {
                                    *confirm_overwrite=true;
                                }else{
                                    match crate::maker_compiler::fragment_source_for_export(graph,curator,*export_format){
                                        Ok(source)=>{
                                            let result=(||->Result<(),String>{
                                                let parent=path.parent().ok_or("Invalid export directory")?;
                                                std::fs::create_dir_all(parent).map_err(|e|e.to_string())?;
                                                std::fs::write(&path,source.as_bytes()).map_err(|e|e.to_string())
                                            })();
                                            match result {
                                                Ok(())=>{*last_glsl_curator=curator.clone();*status=format!("Exported GLSL to {}",path.display());close_export=true;},
                                                Err(e)=>*status=format!("GLSL export failed: {e}"),
                                            }
                                        }
                                        Err(e)=>*status=format!("GLSL generation failed: {e}"),
                                    }
                                }
                            }
                            }
                            if ui.button("Cancel").clicked(){close_export=true;}
                        });
                    });
                if close_export {*glsl_export_dialog=None;}
            }
            // The dock is laid out first. The top menu panel therefore occupies
            // exactly the remaining region to its right, with no cached coordinates.
            egui::TopBottomPanel::top("maker_header")
                .frame(egui::Frame::none()
                    .fill(egui::Color32::from_gray(110))
                    .inner_margin(egui::Margin::symmetric(8, 3)))
                .show(ctx, |ui| {
                let mut command = pending_menu_command.take();
                // Menu controls must not share the Background hit-test order
                // with graph Areas. A node positioned mostly outside the canvas
                // can otherwise overlap the header's interaction region even
                // though its painted body is clipped to the canvas.
                //
                // Render only the compact menu bar in a Foreground Area. Do NOT
                // promote the entire TopBottomPanel: doing so paints its panel
                // background over the graph and dims node bodies.
                let menu_origin = ui.cursor().min;
                let menu_area = egui::Area::new(egui::Id::new("maker_header_menu_interaction"))
                    .order(egui::Order::Foreground)
                    .movable(false)
                    .fixed_pos(menu_origin)
                    .show(ctx, |menu_ui| {
                egui::menu::bar(menu_ui, |ui| {
                    // Display the native 22x22 artwork without scaling; the menu
                    // retains its existing compact header layout.
                    ui.add(egui::Image::new((menu_icon.id(), egui::vec2(22.0, 22.0))));
                    ui.add_space(4.0);
                    ui.menu_button("File", |ui| {
                        for (label, action, enabled) in [
                            ("New Project    Ctrl+N", "new", true),
                            ("Open Project    Ctrl+O", "open", true),
                        ] {
                            if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
                                command=Some(action);ui.close_menu();
                            }
                        }
                        ui.menu_button("Open Recent Project", |ui| {
                            if recent_projects.is_empty(){ui.add_enabled(false,egui::Label::new("No recent projects"));}
                            for (index,path) in recent_projects.iter().enumerate(){
                                let label=format!("{}  {}",index+1,path.file_name().and_then(|n|n.to_str()).unwrap_or("Project"));
                                if ui.button(label).on_hover_text(path.display().to_string()).clicked(){
                                    command=Some(match index {0=>"recent_0",1=>"recent_1",2=>"recent_2",3=>"recent_3",4=>"recent_4",5=>"recent_5",6=>"recent_6",_=>"recent_7"});
                                    ui.close_menu();
                                }
                            }
                        });
                        for (label, action, enabled) in [
                            ("Save Project    Ctrl+S", "save", true),
                            ("Save Project As    Ctrl+Shift+S", "save_as", true),
                            ("Close Project    Ctrl+C", "close", has_active_project),
                            ("Close All Projects    Ctrl+Shift+C", "close_all", has_active_project),
                            ("Delete Project    Ctrl+Shift+D", "delete", project_path.is_some()),
                            ("Export GLSL File    Ctrl+X", "export_glsl", true),
                            ("Export GLSL Code    Ctrl+Shift+X", "export_glsl_clipboard", true),
                            ("Exit    Ctrl+Q", "exit", true),
                        ] {
                            if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
                                command = Some(action); ui.close_menu();
                            }
                        }
                    });
                    ui.menu_button("Edit", |ui| {
                        for (label, action, enabled) in [
                            ("Remove Orphan Modules    Ctrl+>", "remove_orphans", !graph.orphan_ids().is_empty()),
                            ("Undo Remove", "undo_remove", !orphan_undo.is_empty()),
                            ("Redo Remove", "redo_remove", !orphan_redo.is_empty()),
                            ("Optimize Routing    Ctrl+<", "optimize", true),
                            ("Undo Routing", "undo_routing", !routing_undo.is_empty()),
                            ("Redo Routing", "redo_routing", !routing_redo.is_empty()),
                            ("Center Graph", "center", true),
                            ("Clear Graph", "clear", true),
                        ] {
                            if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
                                command = Some(action); ui.close_menu();
                            }
                        }
                    });
                    ui.menu_button("Tools", |ui| {
                        for (label, action) in [
                            ("Start Screenshaver", "start_screenshaver"),
                            ("Open Effects Manager", "open_control_center"),
                        ] {
                            if ui.button(label).clicked() {
                                command = Some(action);
                                ui.close_menu();
                            }
                        }
                    });
                    ui.menu_button("View", |ui| {
                        for (label, action) in [
                            ("Zoom In    Ctrl++", "zoom_in"),
                            ("Zoom Out    Ctrl+-", "zoom_out"),
                            ("Reset Zoom (100%)    Ctrl+0", "zoom_reset"),
                            ("Zoom to Fit    Ctrl+Shift+F", "zoom_fit"),
                        ] {
                            if ui.button(label).clicked() {
                                command = Some(action); ui.close_menu();
                            }
                        }
                    });
                });
                });
                // Keep the original header layout: the foreground menu is
                // independently rendered, so reserve its measured height in
                // the panel before laying out the project tabs beneath it.
                ui.allocate_space(egui::vec2(ui.available_width(), menu_area.response.rect.height()));
                ui.separator();
                ui.horizontal(|ui| {
                    // Keyboard focus belongs to the project rail, not the graph or text editors.
                    // Clicking a tab opts into Left/Right navigation until focus moves elsewhere.
                    let rail_focus_id = ui.make_persistent_id("maker_project_tab_keyboard_focus");
                    let rail_focused = ui.memory(|memory| memory.has_focus(rail_focus_id));
                    if rail_focused && has_active_project && requested_tab.is_none() {
                        let direction = ui.input(|input| {
                            if input.key_pressed(egui::Key::ArrowLeft) { -1_i32 }
                            else if input.key_pressed(egui::Key::ArrowRight) { 1_i32 }
                            else { 0_i32 }
                        });
                        if direction != 0 {
                            let next = if direction < 0 {
                                active_tab_index.checked_sub(1)
                            } else if active_tab_index + 1 < tab_labels.len() {
                                Some(active_tab_index + 1)
                            } else { None };
                            if let Some(index) = next {
                                // A grayed-out Thumbnail is not part of tab_labels.
                                requested_tab = Some(index);
                            }
                        }
                    }
                    // Fixed left-hand slot: never participates in horizontal scrolling.
                    if let Some(index) = thumbnail_tab_index {
                        if ui.selectable_label(index == active_tab_index, "Thumbnail").clicked() {
                            requested_tab = Some(index);
                            ui.memory_mut(|memory| memory.request_focus(rail_focus_id));
                        }
                    } else {
                        ui.add_enabled(false, egui::SelectableLabel::new(false, "Thumbnail"));
                    }
                    // The Thumbnail workspace is fixed outside the scrolling tab rail.
                    // Give it the same independent close target as ordinary projects.
                    if let Some(index) = thumbnail_tab_index {
                        if ui.small_button("×").on_hover_text("Close Thumbnail").clicked() {
                            requested_close_tab_index = Some(index);
                        }
                    }
                    ui.separator();
                    // The scroll area clips the ordinary tabs but never paints a scrollbar
                    // over their labels. Arrow buttons navigate the same scroll offset.
                    let rail_width = (ui.available_width() - 78.0).max(48.0);
                    let step = 140.0_f32;
                    let left = ui.add_enabled(*project_tab_scroll_x > 0.5, egui::Button::new("◀"))
                        .on_hover_text("Scroll project tabs left");
                    if left.clicked() {
                        *project_tab_scroll_x = (*project_tab_scroll_x - step).max(0.0);
                    }
                    let mut max_scroll = 0.0_f32;
                    ui.allocate_ui_with_layout(
                        egui::vec2(rail_width, ui.spacing().interact_size.y),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            let output = egui::ScrollArea::horizontal()
                                .id_source("maker_project_tab_scroll")
                                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                                .horizontal_scroll_offset(*project_tab_scroll_x)
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        for (index, label) in tab_labels.iter().enumerate() {
                                            if Some(index) == thumbnail_tab_index { continue; }
                                            let title = if label.is_empty() { "Untitled" } else { label.as_str() };
                                            let response = ui.selectable_label(index == active_tab_index, title);
                                            if response.clicked() {
                                                requested_tab = Some(index);
                                                ui.memory_mut(|memory| memory.request_focus(rail_focus_id));
                                            }
                                            // A separate hit target closes this tab without treating the X as a selection click.
                                            // The existing close command handles dirty-state confirmation and Save/Discard/Cancel.
                                            if ui.small_button("×")
                                                .on_hover_text(format!("Close {title}"))
                                                .clicked() {
                                                requested_close_tab_index = Some(index);
                                            }
                                            if index == active_tab_index && !ui.clip_rect().contains_rect(response.rect) {
                                                response.scroll_to_me(Some(egui::Align::Center));
                                            }
                                        }
                                        if !has_active_project { ui.label("No project open"); }
                                    });
                                });
                            max_scroll = (output.content_size.x - output.inner_rect.width()).max(0.0);
                            *project_tab_scroll_x = output.state.offset.x.clamp(0.0, max_scroll);
                        },
                    );
                    if ui.add_enabled(*project_tab_scroll_x < max_scroll - 0.5, egui::Button::new("▶"))
                        .on_hover_text("Scroll project tabs right").clicked() {
                        *project_tab_scroll_x = (*project_tab_scroll_x + step).min(max_scroll);
                    }
                    if ui.small_button("+").on_hover_text("New Project Tab").clicked() {
                        requested_new_tab = true;
                    }
                });
                // While the confirmation is open, do not accept a second command.
                if unsaved_action.is_some(){command=None;}
                // Only graph-replacing or closing operations need the unsaved-work guard.
                // Compare persisted graph data, not revision counters (which also change
                // for transient operations and do not reliably represent file changes).
                if let Some(action)=command {
                    let destructive=matches!(action,"close"|"exit"|"clear") || action.starts_with("recent_");
                    let approved=*discard_approved==Some(action);
                    if approved {*discard_approved=None;}
                    if destructive && !approved && !*thumbnail_workspace && serde_json::json!(crate::maker::graph_to_wire(graph))!=*saved_graph_snapshot {
                        *unsaved_action=Some(action);
                        command=None;
                    }
                }
                match command {
                    Some("start_screenshaver")=>{*pending_tools_action=Some("start_screenshaver");}
                    Some("open_control_center")=>{*pending_tools_action=Some("open_control_center");}
                    Some("new")=>{requested_new_tab=true;}
                    Some("close_all")=>{requested_start_close_all=true;}
                    Some("close")=>{requested_close_tab=true;}
                    Some("open")=>*project_dialog=Some(MakerProjectBrowser::open(false,None,"Untitled")),
                    Some(action) if action.starts_with("recent_")=>{
                        if let Some(index)=action.strip_prefix("recent_").and_then(|n|n.parse::<usize>().ok()){
                            if let Some(path)=recent_projects.get(index).cloned(){
                                match maker_load_project(&path){
                                    Ok(loaded)=>{
                                        requested_open_project=Some((path.clone(),loaded));
                                        *status=format!("Opening recent project {}",path.display());
                                        match maker_record_recent_project(&path){
                                            Ok(updated)=>*recent_projects=updated,
                                            Err(e)=>*status=format!("Opened project, but recent history update failed: {e}"),
                                        }
                                    }
                                    Err(e)=>*status=format!("Unable to open recent project {}: {e}",path.display()),
                                }
                            }
                        }
                    }
                    Some("save_as")=>*project_dialog=Some(MakerProjectBrowser::open(true,None,thumbnail_hash.as_deref().unwrap_or(project_name))),
                    Some("save")=>{
                        if let Some(path)=project_path.clone(){
                            match maker_save_project(graph,&path){
                                Ok(())=>{
                                    *saved_graph_snapshot=serde_json::json!(crate::maker::graph_to_wire(graph));
                                    *thumbnail_workspace=false;
                                    *thumbnail_hash=None;
                                    *status=format!("Saved project {}",path.display());
                                    if let Some(action)=after_save_action.take(){*pending_menu_command=Some(action);}
                                },
                                Err(e)=>{if *after_save_action==Some("close_all") {*close_all_running=false;} *after_save_action=None;*status=format!("Save failed: {e}")},
                            }
                        }else{*project_dialog=Some(MakerProjectBrowser::open(true,None,thumbnail_hash.as_deref().unwrap_or(project_name)));}
                    }
                    Some("export_glsl_clipboard")=>{
                        *glsl_export_dialog=Some((String::new(),String::new(),false,last_glsl_curator.clone(),true,ExportFormat::Native));
                    }
                    Some("export_glsl")=>{
                        let default_dir=std::env::var_os("HOME")
                            .map(|home|std::path::PathBuf::from(home).join(".config/screenshaver/shaders"))
                            .unwrap_or_else(||std::path::PathBuf::from("."));
                        let filename=format!("{}.glsl",project_name.trim_end_matches(".smprj"));
                        *glsl_export_dialog=Some((default_dir.to_string_lossy().into_owned(),filename,false,last_glsl_curator.clone(),false,ExportFormat::Native));
                    }
                    Some("delete")=>*delete_project_confirm=true,
                    Some("exit")=>*open=false,
                    Some("clear")=>{
                        let revision=graph.revision;*graph=Graph::blank();graph.revision=revision.wrapping_add(1);
                        *selected=None;*parameter_popup=None;*pending=None;*dragging_wire=None;
                        routes.clear();routing_undo.clear();routing_redo.clear();orphan_undo.clear();orphan_redo.clear();*orphan_restore_pending=false;
                        candidates.clear();*candidate_epoch=candidate_epoch.wrapping_add(1);candidate_jobs.clear();*connection_search=None;
                        *candidate_base=None;candidate_undo.clear();*candidate_selected=None;
                        *status="Graph cleared; Output Color retained".into();
                    }
                    Some("remove_orphans")=>{
                        let previous=graph.clone();
                        let removed=graph.remove_orphans();
                        if !removed.is_empty(){
                            orphan_undo.push(previous);
                            if orphan_undo.len()>32{orphan_undo.remove(0);}
                            if selected.is_some_and(|id|removed.contains(&id)){*selected=None;}
                            if parameter_popup.is_some_and(|id|removed.contains(&id)){*parameter_popup=None;}
                            *pending=None;*dragging_wire=None;
                            routes.clear();routing_undo.clear();routing_redo.clear();
                            // Keep existing thumbnails and their discovery snapshot. The most
                            // recent undo entry is the exact pre-removal graph for reactivation.
                            *orphan_restore_pending=true;
                            *candidate_epoch=candidate_epoch.wrapping_add(1);candidate_jobs.clear();*connection_search=None;
                            *candidate_selected=None;
                            *status=format!("Removed {} orphan module{}; thumbnails retained (click to restore)",removed.len(),if removed.len()==1{""}else{"s"});
                        }
                        orphan_redo.clear();
                    }
                    Some("undo_remove")=>{
                        let before=graph.clone();
                        if let Some(mut previous)=orphan_undo.pop(){
                            previous.revision=graph.revision.wrapping_add(1);
                            *graph=previous;
                            routes.clear();routing_undo.clear();routing_redo.clear();
                            *orphan_restore_pending=false;
                            *candidate_epoch=candidate_epoch.wrapping_add(1);candidate_jobs.clear();*connection_search=None;
                            *candidate_selected=None;
                            *pending=None;*dragging_wire=None;
                            *status="Restored removed orphan modules; thumbnails retained".into();
                        }
                        orphan_redo.push(before);
                    }
                    Some("redo_remove")=>{
                        if let Some(mut next)=orphan_redo.pop(){
                            orphan_undo.push(graph.clone());next.revision=graph.revision.wrapping_add(1);*graph=next;
                            routes.clear();routing_undo.clear();routing_redo.clear();
                            *orphan_restore_pending=false;*candidate_epoch=candidate_epoch.wrapping_add(1);
                            candidate_jobs.clear();*connection_search=None;*candidate_selected=None;
                            *pending=None;*dragging_wire=None;*status="Reapplied orphan removal".into();
                        }
                    }
                    Some("optimize")=>optimize_requested=true,
                    Some("center")=>fit_requested=true,
                    Some("zoom_fit")=>*pending_zoom_to_fit=true,
                    Some("zoom_in")=>*pending_zoom=Some(*canvas_zoom*1.2),
                    Some("zoom_out")=>*pending_zoom=Some(*canvas_zoom/1.2),
                    Some("zoom_reset")=>*pending_zoom=Some(1.0),
                    Some("undo_routing")=>{
                        if let Some((previous,old_routes))=routing_undo.pop(){
                            routing_redo.push((graph.clone(),routes.clone()));
                            *graph=previous;*routes=old_routes;graph.revision+=1;
                            *status="Restored previous routing layout".into();
                        }
                    }
                    Some("redo_routing")=>{
                        if let Some((next,next_routes))=routing_redo.pop(){
                            routing_undo.push((graph.clone(),routes.clone()));
                            *graph=next;*routes=next_routes;graph.revision+=1;
                            *status="Reapplied routing layout".into();
                        }
                    }
                    _=>{}
                }
            });
            egui::SidePanel::right("maker_inspector").resizable(true).default_width(260.0).min_width(200.0).show(ctx,|ui|{
                // Keep panel sizing/resizing on its original layer, but register
                // Inspector controls above independent graph-node Areas.
                ui.with_layer_id(egui::LayerId::new(
                    if project_dialog.is_some() || unsaved_action.is_some() || glsl_export_dialog.is_some() { egui::Order::Background } else { egui::Order::Middle },
                    egui::Id::new("maker_inspector_interaction")), |ui| {
                // Same 130 logical-point logo width as the Control Center header.
                // Keep it at the top of the dock, above Shader Discovery.
                let logo_width = 130.0_f32;
                let logo_height = logo_width / branding_aspect_ratio.max(0.001);
                ui.vertical_centered(|ui| {
                    ui.add(egui::Image::new((branding_texture.id(), egui::vec2(logo_width, logo_height))));
                    ui.heading("Shader Discovery");
                });
                ui.separator();
                ui.horizontal(|ui|{
                    ui.selectable_value(candidate_mode,2,"Connections");
                    ui.selectable_value(candidate_mode,1,"Parameters");
                    ui.selectable_value(candidate_mode,0,"Properties");
                });
                ui.separator();
                if *candidate_mode != 0 {
                    if *candidate_mode==2 {
                        // Connection feasibility and structural validity are reported by discovery itself.
                    } else {
                        ui.label(format!("Potential parameter combinations: {}",parameter_permutation_count(graph)))
                            .on_hover_text("Exact number of sampled parameter combinations before the discovery cap; animation speed is excluded.");
                    }
                    if *candidate_mode==2 {
                        ui.add_enabled_ui(connection_search.is_none() && candidate_jobs.is_empty(),|ui|{
                            egui::ComboBox::from_label("Max. enumerations")
                                .selected_text(max_enumerations.to_string())
                                .show_ui(ui,|ui|{
                                    for count in [10_000usize,25_000,50_000,100_000] {
                                        ui.selectable_value(max_enumerations,count,count.to_string());
                                    }
                                });
                            egui::ComboBox::from_label("Max. GLSL candidates")
                                .selected_text(max_candidates.to_string())
                                .show_ui(ui,|ui|{
                                    for count in [100usize,250,500,1_000] {
                                        ui.selectable_value(max_candidates,count,count.to_string());
                                    }
                                });
                            egui::ComboBox::from_label("Max. wire connections")
                                .selected_text(max_wire_connections.to_string())
                                .show_ui(ui,|ui|{
                                    for count in 3..=25 {
                                        ui.selectable_value(max_wire_connections,count,count.to_string());
                                    }
                                });
                            egui::ComboBox::from_label("Min. wire connections")
                                .selected_text(min_wire_connections.to_string())
                                .show_ui(ui,|ui|{
                                    for count in 2..=24 {
                                        ui.add_enabled_ui(count<=*max_wire_connections,|ui|{
                                            ui.selectable_value(min_wire_connections,count,count.to_string());
                                        });
                                    }
                                });
                            if *min_wire_connections>*max_wire_connections {
                                *min_wire_connections=*max_wire_connections;
                            }
                        });
                    }
                    egui::ComboBox::from_label("Sort thumbnails")
                        .selected_text(match *candidate_sort {
                            1=>"Visual Complexity",2=>"Perceptual Contrast",3=>"Spatial Entropy",
                            4=>"Color Vibrancy",5=>"Discovery Order",6=>"Motion Activity",7=>"Wire Connections",8=>"Callable Fields",_=>"Visual Interest",
                        })
                        .show_ui(ui,|ui|{
                            ui.selectable_value(candidate_sort,0,"Visual Interest");
                            ui.selectable_value(candidate_sort,1,"Visual Complexity");
                            ui.selectable_value(candidate_sort,2,"Perceptual Contrast");
                            ui.selectable_value(candidate_sort,3,"Spatial Entropy");
                            ui.selectable_value(candidate_sort,4,"Color Vibrancy");
                            ui.selectable_value(candidate_sort,5,"Discovery Order");
                            ui.selectable_value(candidate_sort,6,"Motion Activity");
                            ui.selectable_value(candidate_sort,7,"Wire Connections (most first)");
                            ui.selectable_value(candidate_sort,8,"Callable Fields");
                        });
                    if *candidate_sort==6 {
                        let scored=candidates.iter().filter(|c|c.motion.is_some() || c.motion_failed).count();
                        ui.small(format!("Motion analyzed: {scored} / {} retained (one candidate per frame)",candidates.len()));
                    }
                    if *candidate_mode==2 {
                        if ui.button("Reset Enumeration").on_hover_text("Reset only the checkpoint for the selected minimum/maximum wire range. Other ranges and retained thumbnails are preserved.").clicked(){
                            *reset_enumeration_confirm=true;
                        }
                    }
                    ui.add_space(14.0);
                    let start_discovery_clicked = ui.horizontal(|ui| {
                        let button_width = 210.0;
                        ui.add_space(((ui.available_width() - button_width) * 0.5).max(0.0));
                        ui.add_sized(
                            [button_width, 42.0],
                            egui::Button::new(
                                egui::RichText::new(if *candidate_mode == 2 { "Start Discovery" } else { "Discover parameters" })
                                    .size(18.0)
                                    .strong(),
                            ),
                        ).clicked()
                    }).inner;
                    if start_discovery_clicked {
                        candidates.clear();*candidate_epoch=candidate_epoch.wrapping_add(1);candidate_jobs.clear();candidate_undo.clear();*analysis_request=None;*analysis=None;*candidate_selected=None;
                        *candidate_base=Some(graph.clone());
                        *connection_search=None;*candidate_evaluated=0;*candidate_glsl_capable=0;*candidate_rejections=0;*candidate_uniform=0;*candidate_glsl_errors=0;*candidate_render_errors=0;
                        if *candidate_mode==2 {
                            if let Err(error)=maker_save_discovery_settings(*min_wire_connections,*max_wire_connections,*max_enumerations,*max_candidates){
                                crate::maker::operational_log(format_args!("[MAKER STATE] Unable to save last discovery settings: {error}"));
                            }
                            maker_discovery::set_production_wire_limit(*max_wire_connections);
                            *connection_search=ConnectionSearch::new(graph,*max_enumerations,*min_wire_connections,*max_wire_connections);
                            *candidate_notice="Canonical discovery (CPU only); compatible checkpoint resumes automatically...".into();
                        }else{
                            let generated=candidate_graphs(graph);
                            let count=generated.len();
                            *candidate_jobs=generated.into_iter().rev().map(|(name,graph)|CandidateJob{
                                graph,name,source_revision:candidate_base.as_ref().unwrap().revision,
                                connection_candidate:false,
                            }).collect();
                            *candidate_notice=format!("Evaluating {count} parameter combinations...");
                        }
                    }
                    if connection_search.is_some() || !candidate_jobs.is_empty(){
                        if ui.button("Cancel discovery").clicked(){
                            *connection_search=None;*candidate_epoch=candidate_epoch.wrapping_add(1);candidate_jobs.clear();
                            *candidate_notice="Discovery cancelled; completed thumbnails preserved".into();
                        }
                    }
                    if let Some(search)=connection_search.as_ref(){
                        // The canonical checkpoint count is cumulative, while the
                        // progress bar describes only this bounded CPU batch.
                        // The selected batch size is not the lifetime graph limit.
                        let batch_size=search.max_enumerations as u64;
                        let total=search.enumeration_start.saturating_add(search.examined as u64);
                        let batch_number=search.enumeration_start / batch_size + 1;
                        let batch_progress=(search.examined as u64).min(batch_size);
                        ui.label(format!("Total states examined: {}",total));
                        if let Some(enumerator)=search.canonical.as_ref(){
                            ui.small(format!("Structurally valid configurations encountered: {}",enumerator.valid()));
                        }
                        ui.add(egui::ProgressBar::new(batch_progress as f32 / batch_size as f32)
                            .show_percentage()
                            .text(format!("Current discovery batch {}: {} / {} states",batch_number,batch_progress,batch_size)));
                        ui.small(format!("Stage: {} · raw={} · validated={} · rejected={} · duplicates={} · CPU workers={}",
                            search.stage.name(),search.raw.len(),search.qualified.len(),search.incomplete,search.duplicates,search.workers));
                        if let Some(error)=&search.discovery_error {ui.colored_label(egui::Color32::RED,error);}
                        for report in &search.reports {ui.small(report);}
                    }
                    if let Some(search)=connection_search.as_mut(){
                        if search.finished() && !search.gpu_started && search.discovery_error.is_none() {
                            // Automatically begin the next GPU batch. The 32-job cap
                            // limits a batch, not the lifetime discovery target.
                            if search.generated>0 && *candidate_glsl_capable<*max_candidates {
                                *candidate_jobs=std::mem::take(&mut search.selected_jobs);
                                search.gpu_started=true;
                                *candidate_notice=format!("GPU batch queued: {} graphs; GLSL-capable {} / {}",candidate_jobs.len(),*candidate_glsl_capable,*max_candidates);
                            } else if search.generated==0 {
                                search.gpu_started=true;
                            }

                            ui.small(format!("CPU discovery complete: {} selected graphs (raw storage ceiling: {}). No GLSL generated.",search.generated,MAX_RAW_GRAPHS));
                            ui.small(format!("Qualified: {} · deferred for complexity: {} · source score: {}..{} · runtime estimate: {}..{}",
                                search.generated,search.deferred_complexity,
                                if search.min_source_cost==u64::MAX {0}else{search.min_source_cost},search.max_source_cost,
                                if search.min_runtime_cost==u64::MAX {0}else{search.min_runtime_cost},search.max_runtime_cost));
                            if search.generated==0 {
                                ui.colored_label(egui::Color32::YELLOW,"No candidates passed the source-size and runtime-risk guards; GPU evaluation is unavailable for this batch.");
                            }
                        }
                    }
                    if *candidate_evaluated>0 || !candidate_jobs.is_empty(){
                        let total=(*candidate_evaluated+candidate_jobs.len()).max(1);
                        ui.add(egui::ProgressBar::new(*candidate_evaluated as f32/total as f32)
                            .text(format!("Evaluated: {} · rejected: {} · retained: {}",*candidate_evaluated,*candidate_rejections,candidates.len())));
                        ui.small(format!("GLSL-capable: {} / {} · rejected: {} uniform · {} GLSL · {} render",*candidate_glsl_capable,*max_candidates,*candidate_uniform,*candidate_glsl_errors,*candidate_render_errors));
                    }
                    if !candidate_notice.is_empty(){ui.small(candidate_notice.as_str());}
                    ui.label(format!("{} visible · {} pending",
                        candidates.iter().filter(|c|c.connection_candidate==(*candidate_mode==2)).count(),
                        candidate_jobs.len()));
                    if let Some(report)=analysis.as_ref(){
                        ui.separator();ui.strong(format!("Analysis: {}",report.name));
                        let report_text=report.lines.join("\n");
                        if ui.small_button("Copy report").clicked(){*pending_clipboard=Some(report_text);}
                        egui::ScrollArea::vertical().max_height(190.0).show(ui,|ui|{
                            for line in &report.lines {ui.small(line);}
                        });
                    } else if analysis_request.is_some(){ui.small("Analyzing candidate...");}
                    // The Inspector controls use a dedicated foreground layer to
                    // protect them from graph-node hit testing. In egui 0.33 that
                    // layer can prevent the nested ScrollArea from claiming wheel
                    // input through its usual hover test. Route wheel movement by
                    // the candidate viewport's actual geometry instead, without
                    // changing any Inspector, node, scrollbar or dialog layers.
                    let candidate_viewport=ui.available_rect_before_wrap().intersect(ui.clip_rect());
                    let candidate_wheel=if project_dialog.is_none() && examine_module.is_none()
                        && ui.ctx().input(|i| i.pointer.hover_pos()
                            .is_some_and(|p| candidate_viewport.contains(p)))
                    {
                        ui.input_mut(|i| {
                            // Candidate thumbnails are relatively tall; amplify only
                            // their Inspector wheel movement, leaving graph zoom and
                            // all other scroll regions unchanged.
                            const CANDIDATE_WHEEL_MULTIPLIER: f32 = 3.0;
                            let delta=i.smooth_scroll_delta.y * CANDIDATE_WHEEL_MULTIPLIER;
                            // Do not also let the ScrollArea consume this event:
                            // it would otherwise be applied twice on frames where
                            // its built-in hover detection succeeds.
                            i.smooth_scroll_delta.y=0.0;
                            delta
                        })
                    } else {0.0};
                    egui::ScrollArea::vertical().show(ui,|ui|{
                        if candidate_wheel!=0.0 {
                            ui.scroll_with_delta(egui::vec2(0.0,candidate_wheel));
                        }
                        let mut display_order:Vec<usize>=(0..candidates.len()).collect();
                        // Stable ties and discovery-order mode preserve original arrival order.
                        if *candidate_sort!=5 {
                            display_order.sort_by(|&a,&b|candidate_score(&candidates[b],*candidate_sort)
                                .total_cmp(&candidate_score(&candidates[a],*candidate_sort))
                                .then_with(||a.cmp(&b)));
                        }
                        for index in display_order {
                            let candidate=&mut candidates[index];
                            if candidate.connection_candidate!=(*candidate_mode==2){continue}
                            ui.push_id(("maker_candidate_card", index), |ui| {
                            ui.group(|ui|{
                                // Allocate the image and its hit target together in the
                                // scroll area's own coordinate space. Painting through the
                                // same response rect prevents scroll-dependent vertical drift.
                                let thumbnail_size=egui::vec2(ui.available_width().min(240.0),135.0);
                                // Keep layout allocation separate from the uniquely identified
                                // click response. Candidate arrival and asynchronous motion scores
                                // can change card order; positional auto-IDs would otherwise be
                                // reused for a different candidate on subsequent frames.
                                let (thumbnail_rect,_layout_response)=ui.allocate_exact_size(
                                    thumbnail_size,egui::Sense::hover());
                                let thumbnail_response=ui.interact(
                                    thumbnail_rect,ui.id().with("thumbnail_click"),egui::Sense::click());
                                ui.painter().image(candidate.texture.id(),thumbnail_rect,
                                    egui::Rect::from_min_max(egui::Pos2::ZERO,egui::pos2(1.0,1.0)),
                                    egui::Color32::WHITE);
                                if *candidate_selected==Some(index) {
                                    // Selection is drawn inside the same rectangle, without
                                    // introducing a second layout or interactive widget.
                                    let border=egui::Color32::from_rgb(57,255,20);
                                    for offset in [0.0_f32,1.0,2.0] {
                                        let r=thumbnail_rect.shrink(offset);
                                        ui.painter().line_segment([r.left_top(),r.right_top()],egui::Stroke::new(1.0,border));
                                        ui.painter().line_segment([r.right_top(),r.right_bottom()],egui::Stroke::new(1.0,border));
                                        ui.painter().line_segment([r.right_bottom(),r.left_bottom()],egui::Stroke::new(1.0,border));
                                        ui.painter().line_segment([r.left_bottom(),r.left_top()],egui::Stroke::new(1.0,border));
                                    }
                                }
                                // Single-click loads the candidate into the one reusable
                                // Thumbnail tab. Do not modify the source project in-place.
                                if thumbnail_response.clicked() {
                                    *candidate_selected=Some(index);
                                    if let Some(base)=candidate_base.as_ref() {
                                        let mut proposed=base.clone();
                                        let result=if candidate.connection_candidate {
                                            proposed.connections.clear();
                                            let mut result=Ok(());
                                            for c in &candidate.connections {
                                                if let Err(error)=proposed.connect(c.from,c.from_port,c.to,c.to_port) {
                                                    result=Err(error);break;
                                                }
                                            }
                                            if result.is_ok() {
                                                for node in &mut proposed.nodes {
                                                    if let Some(parameters)=candidate.parameters.get(&node.id) {
                                                        let color_ids:Vec<_>=node.definition().parameters.iter()
                                                            .filter(|p|p.ty=="color3").map(|p|p.id.clone()).collect();
                                                        for id in color_ids {
                                                            if let Some(value)=parameters.get(&id) {
                                                                node.parameters.insert(id,value.clone());
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            result
                                        } else {
                                            for node in &mut proposed.nodes {
                                                if let Some(parameters)=candidate.parameters.get(&node.id) {
                                                    node.parameters=parameters.clone();
                                                }
                                            }
                                            Ok(())
                                        };
                                        match result {
                                            Ok(())=>requested_candidate_tab=Some(proposed),
                                            Err(error)=>*status=format!("Cannot open thumbnail: {error}"),
                                        }
                                    } else {*status="Discovery snapshot missing".into();}
                                }
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x=5.0;
                                    ui.small(&candidate.name);
                                    if candidate.uses_callable_field {
                                        let (rect,_) = ui.allocate_exact_size(
                                            egui::vec2(12.0,12.0),egui::Sense::hover());
                                        ui.painter().circle_filled(rect.center(),5.0,
                                            egui::Color32::from_rgb(224,64,251));
                                    }
                                });
                                if candidate.connection_candidate && *candidate_sort!=6 {
                                    ui.small(match candidate.motion {
                                        Some(score) if score<0.003=>"Temporal: static / near-static".to_owned(),
                                        Some(score)=>format!("Temporal: animated ({:.1}%)",score*100.0),
                                        None if candidate.motion_failed=>"Temporal: unavailable".to_owned(),
                                        None=>"Temporal: pending".to_owned(),
                                    });
                                }
                                if *candidate_sort==6 {
                                    ui.small(match candidate.motion {
                                        Some(score)=>format!("Motion activity: {:.1}%",score*100.0),
                                        None if candidate.motion_failed=>"Motion activity: unavailable".to_owned(),
                                        None=>"Motion activity: pending".to_owned(),
                                    });
                                } else if *candidate_sort==7 {
                                    ui.small(format!("Wire connections: {}",candidate.connections.len()));
                                } else if *candidate_sort==8 {
                                    ui.small(if candidate.uses_callable_field {"Callable field: connected"} else {"Callable field: none"});
                                } else if *candidate_sort!=5 {
                                    ui.small(format!("{}: {:.1}%",match *candidate_sort {
                                        1=>"Complexity",2=>"Contrast",3=>"Entropy",4=>"Vibrancy",_=>"Visual interest",
                                    },candidate_score(candidate,*candidate_sort)*100.0));
                                }
                                ui.horizontal(|ui|{
                                    if ui.small_button("Analyze").clicked(){*analysis_request=Some(index);*analysis=None;}
                                });
                            });
                            }); // Stable candidate-specific egui ID scope.
                        }
                    });
                } else if let Some(id)=*selected{
                    if let Some(n)=graph.node_mut(id){
                        ui.strong(format!("{}{} #{}",n.display_name(),if n.unresolved(){" (unknown)"}else{""},id));ui.separator();
                        ui.label(n.definition().description.as_str());
                        if !n.definition().parameters.is_empty() {
                            let response=ui.button("Open parameter editor");
                            if response.clicked(){*parameter_popup=if *parameter_popup==Some(id){None}else{Some(id)};}
                            
                        }
                    }
                    // The required Output Color terminal cannot be removed.
                    if graph.node(id).is_some_and(|node| !node.definition().terminal) {
                        if ui.button("Remove selected module").clicked(){match graph.remove(id){Ok(())=>{*selected=None;*pending=None;*status=format!("Removed module #{id}")},Err(e)=>*status=e}}
                    }
                }else{ui.label("Select a node to edit its parameters.");}
                ui.separator();ui.label("Graph status");
                match graph.validate(){Ok(())=>{ui.small("Typed connections valid; no cycles");},Err(e)=>{ui.colored_label(egui::Color32::LIGHT_RED,e);}}
                });
            });
            // Keep the preview panel allocated at its existing height even when
            // no project is open. Only its shader rendering is disabled.
            // Explicitly own the preview height. A default_height() hint alone is
            // not sufficient: egui may restore a previously cached panel size.
            // The saved value changes only when the divider is dragged.
            let available_height = ctx.available_rect().height();
            let minimum_graph_height = 150.0_f32;
            let minimum_preview_height = 120.0_f32;
            let maximum_preview_height = (available_height - minimum_graph_height)
                .max(minimum_preview_height);
            let desired_height = preview_height.get_or_insert_with(|| available_height * 0.5);
            let shown_height = desired_height.clamp(
                minimum_preview_height.min(maximum_preview_height),
                maximum_preview_height,
            );
            egui::TopBottomPanel::bottom("maker_live_preview")
                .resizable(false)
                .exact_height(shown_height)
                .frame(egui::Frame::default().fill(egui::Color32::from_rgba_unmultiplied(10,14,20,42)))
                .show(ctx,|ui|{
                    // A dedicated draggable divider, instead of relying on
                    // TopBottomPanel's internally remembered resize state.
                    let (divider_rect, divider_response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 9.0), egui::Sense::drag());
                    ui.painter().line_segment(
                        [egui::pos2(divider_rect.left(),divider_rect.center().y),
                         egui::pos2(divider_rect.right(),divider_rect.center().y)],
                        egui::Stroke::new(2.0,egui::Color32::from_gray(105)));
                    if divider_response.hovered() || divider_response.dragged() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                    }
                    if divider_response.dragged() {
                        let dy=ui.input(|i|i.pointer.delta().y);
                        *desired_height = (*desired_height - dy).clamp(
                            minimum_preview_height.min(maximum_preview_height),
                            maximum_preview_height,
                        );
                    }
                    // Persist only when the user releases the resize divider;
                    // never rewrite state.json on every rendered frame.
                    if divider_response.drag_stopped() {
                        if let Err(error)=maker_save_preview_height(*desired_height) {
                            crate::maker::operational_log(format_args!("[MAKER STATE] Could not save preview height: {error}"));
                        }
                    }
                    ui.strong("Live Shader Preview");
                    if *preview_active {
                        ui.small("Graph-generated GLSL 330 core · preview retains last valid program on errors");
                    }
                    // The renderer must not draw into the reserved, blank panel
                    // after Close Project. Its geometry remains unchanged.
                    *preview_rect = if *preview_active {
                        ui.available_rect_before_wrap()
                    } else {
                        egui::Rect::NOTHING
                    };
                });
            egui::CentralPanel::default().frame(egui::Frame::default().fill(egui::Color32::from_rgba_unmultiplied(12,16,25,228))).show(ctx,|ui|{
                ui.strong("Node Graph");ui.small("Drag from output to input to wire · click-to-connect also works · drag node headers to move");ui.separator();
                let available=ui.available_rect_before_wrap();
                // Reserve the bottom and right strips for graph navigation.
                let canvas=egui::Rect::from_min_max(available.min,
                    egui::pos2((available.max.x-15.0).max(available.min.x+1.0),
                               (available.max.y-15.0).max(available.min.y+1.0)));
                *canvas_rect=canvas;
                // Pan in graph coordinates so the chosen instance is centered
                // independently of the current zoom. Use its last measured frame
                // when available, otherwise a standard node-size estimate.
                if let Some(target) = pinpoint_target.take() {
                    if let Some(node) = graph.node(target) {
                        // Center from persistent graph-space coordinates, never
                        // from an Area rectangle measured at an earlier zoom.
                        // A node may be entirely offscreen when pinpointed.
                        let port_extent=staggered_port_height(node);
                        let node_height = 110.0_f32.max(55.0 + port_extent);
                        let center = egui::vec2(
                            node.pos[0] + 93.0,
                            node.pos[1] + node_height * 0.5,
                        );
                        *canvas_pan = canvas.size()*0.5-center* *canvas_zoom;
                    }
                }
                if let Some(requested)=pending_zoom.take() {
                    let next=requested.clamp(0.4,2.0);
                    if (next-*canvas_zoom).abs()>0.0001 {
                        let anchor=canvas.center();
                        let graph_at_center=(anchor-canvas.min-*canvas_pan)/ *canvas_zoom;
                        *canvas_zoom=next;
                        *canvas_pan=anchor-canvas.min-graph_at_center*next;
                        // Invalidate routes together with the viewport transform.
                        routes.clear();
                    }
                }
                if *pending_zoom_to_fit {
                    *pending_zoom_to_fit=false;
                    if !graph.nodes.is_empty() {
                        // Graph-space bounds: node frames and all routed wire points.
                        // A fit changes only zoom and pan, never graph coordinates.
                        let mut left=f32::INFINITY;
                        let mut top=f32::INFINITY;
                        let mut right=f32::NEG_INFINITY;
                        let mut bottom=f32::NEG_INFINITY;
                        for n in &graph.nodes {
                            let port_extent=staggered_port_height(n);
                            let width=if n.module_id=="screenshaver.test.universal_test" {256.0_f32} else {186.0_f32};
                            let height=110.0_f32.max(55.0+port_extent);
                            left=left.min(n.pos[0]);top=top.min(n.pos[1]);
                            right=right.max(n.pos[0]+width);
                            bottom=bottom.max(n.pos[1]+height);
                        }
                        for path in routes.values() {
                            for p in path {
                                left=left.min(p[0]);top=top.min(p[1]);
                                right=right.max(p[0]);bottom=bottom.max(p[1]);
                            }
                        }
                        if left.is_finite() && right.is_finite() &&
                           canvas.width()>64.0 && canvas.height()>64.0 {
                            let width=(right-left).max(1.0);
                            let height=(bottom-top).max(1.0);
                            // 32 screen pixels of clearance on each side.
                            let fit_x=(canvas.width()-64.0)/width;
                            let fit_y=(canvas.height()-64.0)/height;
                            *canvas_zoom=fit_x.min(fit_y).clamp(0.4,1.0);
                            routes.clear();
                            *canvas_pan=egui::vec2(
                                canvas.width()*0.5-(left+right)*0.5* *canvas_zoom,
                                canvas.height()*0.5-(top+bottom)*0.5* *canvas_zoom);
                        }
                    }
                }
                if fit_requested && !graph.nodes.is_empty() {
                    // Center the *complete* existing layout, including routed
                    // wires. This is a translation only: graph coordinates and
                    // the user's routing Undo history remain untouched.
                    let mut left=f32::INFINITY;
                    let mut top=f32::INFINITY;
                    let mut right=f32::NEG_INFINITY;
                    let mut bottom=f32::NEG_INFINITY;
                    for n in &graph.nodes {
                        let port_extent=staggered_port_height(n);
                        // Compact module's measured frame includes egui margins.
                        let width=if n.module_id=="screenshaver.test.universal_test" {256.0_f32} else {186.0_f32};
                        let height=(110.0_f32).max(55.0+port_extent);
                        left=left.min(n.pos[0]);top=top.min(n.pos[1]);
                        right=right.max(n.pos[0]+width);
                        bottom=bottom.max(n.pos[1]+height);
                    }
                    for path in routes.values() {
                        for p in path {
                            left=left.min(p[0]);top=top.min(p[1]);
                            right=right.max(p[0]);bottom=bottom.max(p[1]);
                        }
                    }
                    if left.is_finite() && right.is_finite() {
                        *canvas_pan=egui::vec2(
                            (canvas.width()-(right-left)* *canvas_zoom)*0.5-left* *canvas_zoom,
                            (canvas.height()-(bottom-top)* *canvas_zoom)*0.5-top* *canvas_zoom);
                    }
                }
                // Areas are independent egui layers. Clipping their painting to
                // the canvas does not necessarily prevent their hit-test regions
                // from capturing input over the toolbar or docks. Evaluate the
                // pointer against the actual canvas each frame, not a cached rect.
                let graph_pointer_active=ctx.input(|i| {
                    i.pointer.hover_pos().is_some_and(|p| canvas.contains(p))
                });
                let origin=canvas.min+*canvas_pan;

                let mut input_ports=std::collections::HashMap::new();
                let mut input_hit_rects=std::collections::HashMap::new();
                let mut output_ports=std::collections::HashMap::new();
                let mut module_boxes:BTreeMap<NodeId,Box2>=BTreeMap::new();
                let mut active_node_drag=false;
                let ids:Vec<NodeId>=graph.nodes.iter().map(|n|n.id).collect();
                let mut context_remove_node: Option<NodeId> = None;
                let mut context_examine_module: Option<String> = None;
                for id in ids{
                    // Copy the immutable node fields before entering the UI closure.
                    // The closure also needs mutable access to the graph.
                    let Some(node) = graph.node(id) else { continue };
                    let module = node.definition();
                    let is_unresolved = node.unresolved();
                    let unresolved_display_name = if is_unresolved { Some(node.display_name().to_owned()) } else { None };
                    let pos = node.pos;
                    let expanded = node.expanded;
                    let area=egui::Area::new(egui::Id::new(("maker_node",id)))
                        .order(egui::Order::Middle).movable(false)
                        // Graph nodes may travel beyond the viewport when scrolling.
                        // Area's default screen constraint otherwise pins offscreen
                        // rows to the upper screen edge, stacking them together.
                        // The node UI remains clipped to the graph canvas below.
                        .constrain(false)
                        // Never let a scrolled-out Area intercept toolbar/dock
                        // pointer events. Partially visible nodes remain fully
                        // interactive while the pointer is inside the canvas.
                        .interactable(graph_pointer_active)
                        // The position must follow pan/zoom every frame. current_pos
                        // only initializes an Area and may reuse a stored location.
                        .fixed_pos(origin+egui::vec2(pos[0],pos[1])* *canvas_zoom)
                        .show(ctx,|ui|{
                            ui.set_clip_rect(ui.clip_rect().intersect(canvas));
                            // Scale only graph node widgets, not the Maker toolbar or library.
                            let mut style=(**ui.style()).clone();
                            for font in style.text_styles.values_mut(){font.size*=*canvas_zoom;}
                            style.spacing.item_spacing*=*canvas_zoom;
                            style.spacing.button_padding*=*canvas_zoom;
                            ui.set_style(style);
                            let frame_response=egui::Frame::window(ui.style()).show(ui,|ui|{
                                // Fixed standard width and height; only port count may grow height.
                                let node_width = (if module.id=="screenshaver.test.universal_test" {240.0} else {170.0}) * *canvas_zoom;
                                let port_pitch = 25.0 * *canvas_zoom + ui.spacing().item_spacing.y;
                                let output_offset = port_pitch * 0.5;
                                let port_height = graph.node(id).map(|n| n.port_rows() as f32 * 25.0 * *canvas_zoom).unwrap_or(0.0);
                                let header_height = 27.0 * *canvas_zoom;
                                ui.set_min_width(node_width);
                                ui.set_max_width(node_width);
                                let header_color = if is_unresolved {egui::Color32::from_rgb(85,85,85)}else{node_header_color(module)};
                                let title = if is_unresolved{format!("{} (unknown) #{}",unresolved_display_name.as_deref().unwrap_or(&module.name),id)}
                                    else{format!("{} #{}",module.name.as_str(),id)};
                                let (header_rect, header_response) = ui.allocate_exact_size(
                                    egui::vec2(node_width, header_height), egui::Sense::click_and_drag(),
                                );
                                ui.painter().rect_filled(header_rect, 3.0, header_color);
                                // Measure the rendered text, not the character count: glyph widths
                                // and graph zoom both affect whether the title fits the fixed header.
                                let title_font=egui::TextStyle::Button.resolve(ui.style());
                                let title_left_padding=7.0 * *canvas_zoom;
                                let title_right_padding=7.0 * *canvas_zoom;
                                let available_title_width=(header_rect.width()
                                    -title_left_padding-title_right_padding).max(0.0);
                                let measure_title=|text:&str| {
                                    ui.painter().layout_no_wrap(
                                        text.to_owned(),title_font.clone(),egui::Color32::WHITE,
                                    ).size().x
                                };
                                let display_title=if measure_title(&title)<=available_title_width {
                                    title.clone()
                                } else {
                                    let suffix="...";
                                    let mut shortened=suffix.to_owned();
                                    // Iterate on UTF-8 character boundaries; never split a glyph's bytes.
                                    if measure_title(suffix)<=available_title_width {
                                        for end in title.char_indices().map(|(i,_)|i).skip(1)
                                            .chain(std::iter::once(title.len())) {
                                            let candidate=format!("{}{}",&title[..end],suffix);
                                            if measure_title(&candidate)>available_title_width {break;}
                                            shortened=candidate;
                                        }
                                    }
                                    shortened
                                };
                                ui.painter().text(header_rect.left_center()
                                        +egui::vec2(title_left_padding,0.0),
                                    egui::Align2::LEFT_CENTER,display_title,title_font,
                                    egui::Color32::WHITE);
                                let header_response=header_response.on_hover_text(
                                    format!("{}\nCreated by: {}{}{}\nDrag title bar to move the module",title,module.created_by,module.creator_email.as_ref().map(|email|format!("\nCreator Email: {email}")).unwrap_or_default(),module.attributed_to.iter().map(|name|format!("\nAttributed to: {name}")).collect::<String>())
                                );
                                if header_response.clicked() {
                                    *selected=Some(id);
                                    if *parameter_popup==Some(id) { *parameter_popup=None; }
                                }
                                if header_response.dragged(){*selected=Some(id);active_node_drag=true;}
                                if header_response.dragged(){
                                    let delta=ui.ctx().input(|i|i.pointer.delta());
                                    if let Some(n)=graph.node_mut(id){n.pos[0]+=delta.x/ *canvas_zoom;n.pos[1]+=delta.y/ *canvas_zoom;}
                                }
                                // All editable values live exclusively in the attached editor.
                                // The compact node exposes only a Parameters link.
                                if !module.parameters.is_empty() {
                                    let response = ui.small_button("⚙ Parameters ↗")
                                        .on_hover_text("Open parameter editor");
                                    if response.clicked() {
                                        *parameter_popup = if *parameter_popup == Some(id) { None } else { Some(id) };
                                        *selected = Some(id);
                                    }
                                }
                                // Keep a compact, consistent minimum body height;
                                // extra connector rows may expand it as required.
                                let used = ui.min_rect().height();
                                let target = (110.0_f32* *canvas_zoom - port_height - 16.0* *canvas_zoom).max(0.0);
                                if used < target { ui.add_space(target - used); }
                                // Ports occupy the vertical left/right edges of the node.
                                // They remain visible when the node is collapsed.
                                let inputs = &module.inputs;
                                let mut outputs:Vec<crate::maker_module::Port>=(0..graph.node(id).map(|n|n.output_count()).unwrap_or(0))
                                    .filter_map(|index|graph.node(id).and_then(|n|n.output_port(index))).collect();
                                // Callable outputs are permanent typed ports on eligible modules.
                                // Display Universal Test's Preview Color last without changing
                                // stable output indices in saved .smprj graph connections.
                                let mut output_order:Vec<usize>=(0..outputs.len()).collect();
                                if module.id=="screenshaver.test.universal_test" {
                                    if let Some(preview_index)=outputs.iter().position(|p|p.id=="preview_color") {
                                        output_order.retain(|&i|i!=preview_index);
                                        output_order.push(preview_index);
                                    }
                                }
                                let rows = inputs.len().max(outputs.len());
                                let port_color = port_color;
                                // Port rows share the compact module body width.
                                let body_width = node_width;
                                for row in 0..rows {
                                    let (row_rect, _) = ui.allocate_exact_size(
                                        egui::vec2(body_width, 25.0* *canvas_zoom), egui::Sense::hover());
                                    if let Some(port) = inputs.get(row) {
                                        let name=&port.name;let ty=port.ty.as_str();
                                        let center = egui::pos2(row_rect.left(), row_rect.center().y);
                                        let hit = egui::Rect::from_center_size(center, egui::vec2(22.0* *canvas_zoom, 22.0* *canvas_zoom));
                                        let response = ui.interact(hit, ui.id().with((id, "input", row)), egui::Sense::click_and_drag());
                                        input_ports.insert((id,row), center);
                                        input_hit_rects.insert((id,row), hit);
                                        let _color = port_color(ty);
                                        // Connector disk is painted on this node's own layer after its frame.
                                        ui.painter().text(center + egui::vec2(11.0* *canvas_zoom, 0.0), egui::Align2::LEFT_CENTER,
                                            name, egui::TextStyle::Small.resolve(ui.style()), ui.visuals().text_color());
                                        let response = response.on_hover_text(port_hint(graph,id,row,*pending,*dragging_wire));
                                        if response.clicked() && dragging_wire.is_none() {
                                            if let Some((from, from_port)) = pending.take() {
                                                match graph.connect(from,from_port,id,row) {
                                                    Ok(()) => *status = format!("Connected #{from}:{from_port} to #{id}:{row}"),
                                                    Err(e) => *status = format!("Connection rejected: {e}"),
                                                }
                                            } else {
                                                graph.disconnect_input(id,row);
                                                *status = format!("Disconnected input #{id}:{row}");
                                            }
                                            *selected = Some(id);
                                        }
                                    }
                                    if let Some(&output_index) = output_order.get(row) {
                                        let port=&outputs[output_index];
                                        let name=&port.name;let ty=port.ty.as_str();
                                        let center = egui::pos2(row_rect.right(), row_rect.center().y + output_offset);
                                        let hit = egui::Rect::from_center_size(center, egui::vec2(22.0* *canvas_zoom, 22.0* *canvas_zoom));
                                        let response = ui.interact(hit, ui.id().with((id, "output", output_index)), egui::Sense::click_and_drag());
                                        output_ports.insert((id,output_index),center);
                                        // Connector disk is painted on this node's own layer after its frame.
                                        ui.painter().text(center - egui::vec2(11.0* *canvas_zoom,0.0), egui::Align2::RIGHT_CENTER,
                                            name, egui::TextStyle::Small.resolve(ui.style()),ui.visuals().text_color());
                                        let response = response.on_hover_text(format!(
                                            "{} output ({})\nMultiple destinations allowed. Click or drag to an input.",name,ty));
                                        if response.drag_started() {
                                            *dragging_wire = Some((id,output_index));
                                            *pending = None;
                                            *selected = Some(id);
                                            *status = format!("Drag {} output to a compatible input",ty);
                                        }
                                        if response.clicked() {
                                            let active = *pending == Some((id,output_index));
                                            *pending = if active {None} else {Some((id,output_index))};
                                            *selected = Some(id);
                                            *status = if active {"Connection cancelled".into()} else {format!("Select a {} input",ty)};
                                        }
                                    }
                                }
                                // Reserve the staggered last output row inside the frame.
                                if !outputs.is_empty() && outputs.len() >= inputs.len() {
                                    ui.add_space(output_offset);
                                }
                            });
                        });
                    // The module's context menu is separate from its drag, port,
                    // and parameter interactions. Defer mutation until all nodes
                    // have been rendered so iteration and connector layout remain valid.
                    area.response.context_menu(|ui| {
                        if ui.button("Examine Module Code").clicked() {
                            context_examine_module = Some(module.id.clone());
                            ui.close_menu();
                        }
                        if !module.terminal && ui.button("Remove Module").clicked() {
                            context_remove_node = Some(id);
                            ui.close_menu();
                        }
                    });
                    if let Some((pulse_id, started)) = pinpoint_pulse.as_ref() {
                        if *pulse_id == id {
                            let elapsed = started.elapsed().as_secs_f32();
                            if elapsed < 0.72 {
                                // Three bright 120 ms flashes, each followed by 120 ms off.
                                if (elapsed / 0.12).floor() as u32 % 2 == 0 {
                                    let painter = ctx.layer_painter(egui::LayerId::new(
                                        egui::Order::Foreground, egui::Id::new("maker_pinpoint_pulse")));
                                    let border = area.response.rect.expand(2.0);
                                    let stroke = egui::Stroke::new(3.0,
                                        if graph.node(id).is_some_and(|n|n.unresolved()) {
                                            egui::Color32::from_rgb(0xff,0x38,0x38)
                                        }else{egui::Color32::from_rgb(0x39,0xff,0x14)});
                                    painter.line_segment([border.left_top(),border.right_top()],stroke);
                                    painter.line_segment([border.right_top(),border.right_bottom()],stroke);
                                    painter.line_segment([border.right_bottom(),border.left_bottom()],stroke);
                                    painter.line_segment([border.left_bottom(),border.left_top()],stroke);
                                }
                                ctx.request_repaint();
                            }
                        }
                    }
                    module_boxes.insert(id,Box2{min:[(area.response.rect.min.x-origin.x)/ *canvas_zoom,(area.response.rect.min.y-origin.y)/ *canvas_zoom],
                        max:[(area.response.rect.max.x-origin.x)/ *canvas_zoom,(area.response.rect.max.y-origin.y)/ *canvas_zoom]});
                    pinpoint_boxes.insert(id, Box2 {
                        min: [(area.response.rect.min.x-origin.x)/ *canvas_zoom,
                              (area.response.rect.min.y-origin.y)/ *canvas_zoom],
                        max: [(area.response.rect.max.x-origin.x)/ *canvas_zoom,
                              (area.response.rect.max.y-origin.y)/ *canvas_zoom],
                    });
                    // Use the measured outer module rectangle as the single
                    // authoritative left/right sidewall for connector centers.
                    // Port-row rects are inset by egui Frame margins; using their
                    // x coordinates for routing caused all wires to fail validation.
                    let wall = area.response.rect;
                    for (&(port_node, _), center) in input_ports.iter_mut() {
                        if port_node == id { center.x = wall.left(); }
                    }
                    for (&(port_node, _), center) in output_ports.iter_mut() {
                        if port_node == id { center.x = wall.right(); }
                    }
                    // Keep drop targeting centered on the same visible connector.
                    for (&(port_node, port_row), hit) in input_hit_rects.iter_mut() {
                        if port_node == id {
                            if let Some(center) = input_ports.get(&(port_node, port_row)) {
                                *hit = egui::Rect::from_center_size(*center, egui::vec2(22.0* *canvas_zoom, 22.0* *canvas_zoom));
                            }
                        }
                    }
                    // Draw on the node's own layer so later Areas still occlude
                    // earlier nodes. The disc center straddles the actual border.
                    let disc_painter=ctx.layer_painter(area.response.layer_id).with_clip_rect(canvas);
                    for (&(owner,row),&center) in &input_ports {
                        if owner==id {
                            if let Some(port)=module.inputs.get(row) {
                                disc_painter.circle_filled(center,5.0* *canvas_zoom,port_color(&port.ty));
                            }
                        }
                    }
                    for (&(owner,row),&center) in &output_ports {
                        if owner==id {
                            if let Some(port)=graph.node(id).and_then(|n|n.output_port(row)) {
                                disc_painter.circle_filled(center,5.0* *canvas_zoom,port_color(&port.ty));
                            }
                        }
                    }
                    if let Some((from,from_port))=(*dragging_wire).or(*pending) {
                        for (&(owner,to_port),&center) in &input_ports {
                            if owner==id {
                                let compatible=graph.check_connection(from,from_port,id,to_port).is_ok();
                                disc_painter.circle_stroke(center,10.0* *canvas_zoom,
                                    egui::Stroke::new(1.5* *canvas_zoom,
                                        if compatible {egui::Color32::GREEN}else{egui::Color32::RED}));
                            }
                        }
                    }
                    // Anchor the editor to the *bottom of the whole module*, not
                    // to the Parameters button or title bar.
                    if *parameter_popup == Some(id) {
                        editor_anchor = Some((id, area.response.rect.left_bottom()));
                    }
                    // A click on the module background also dismisses its editor;
                    // child controls and wiring ports retain their own interactions.
                    if area.response.clicked() && *parameter_popup == Some(id) {
                        *parameter_popup = None;
                    }
                }
                if let Some(module_id) = context_examine_module {
                    *examine_module = Some(module_id);
                }
                if let Some(id) = context_remove_node {
                    match graph.remove(id) {
                        Ok(()) => {
                            if *selected == Some(id) { *selected = None; }
                            if *parameter_popup == Some(id) { *parameter_popup = None; }
                            *pending = None;
                            *dragging_wire = None;
                            *status = format!("Removed module #{id}");
                        }
                        Err(error) => *status = error,
                    }
                }
                // Resolve a wire drop after collecting all port rectangles. This also
                // supports dropping on a port in a node drawn earlier in the frame.
                let pointer_pos = ctx.input(|i| i.pointer.interact_pos());
                let pointer_released = ctx.input(|i| i.pointer.any_released());
                if let Some((from, from_port)) = *dragging_wire {
                    if pointer_released {
                        let target = pointer_pos.and_then(|p| {
                            input_ports.iter().find_map(|(&(id, port), &center)| {
                                // The visible port is a small button, not just its center.
                                input_hit_rects.get(&(id,port)).is_some_and(|hit|hit.contains(p))
                                    .then_some((id,port))
                            })
                        });
                        if let Some((to, to_port)) = target {
                            match graph.connect(from, from_port, to, to_port) {
                                Ok(()) => *status = format!("Connected #{from}:{from_port} → #{to}:{to_port}"),
                                Err(error) => *status = format!("Connection rejected: {error}"),
                            }
                            *selected = Some(to);
                        } else {
                            *status = "Wire drag cancelled (release over an input port)".into();
                        }
                        *dragging_wire = None;
                    }
                }
                // Measure actual port locations after drawing modules; never guess
                // their offsets from the JSON module definitions.
                let measured_ports: BTreeMap<maker_routing::ConnectionKey,PortPair> = {
                    graph.connections.iter().filter_map(|c|{
                        let a=output_ports.get(&(c.from,c.from_port))?;
                        let b=input_ports.get(&(c.to,c.to_port))?;
                        Some((maker_routing::key(c),PortPair{from:[(a.x-origin.x)/ *canvas_zoom,(a.y-origin.y)/ *canvas_zoom],to:[(b.x-origin.x)/ *canvas_zoom,(b.y-origin.y)/ *canvas_zoom]}))
                    }).collect()
                };
                let boxes_now:Vec<_>=module_boxes.iter().map(|(&id,&rect)|(id,rect)).collect();
                if optimize_requested {
                    let original=graph.clone();
                    let sizes:BTreeMap<_,_>=module_boxes.iter().map(|(&id,b)|(id,[b.max[0]-b.min[0],b.max[1]-b.min[1]])).collect();
                    // Evaluate several deterministic row widths using the real router.
                    // The footprint includes every routed wire, not just module boxes.
                    let baseline=maker_routing::compact_positions(graph,&sizes);
                    let mut ordered:Vec<_>=baseline.iter().map(|(&id,&p)|(id,p)).collect();
                    ordered.sort_by(|a,b|a.1[0].total_cmp(&b.1[0]).then_with(||a.1[1].total_cmp(&b.1[1])).then_with(||a.0.cmp(&b.0)));
                    let ids:Vec<NodeId>=ordered.into_iter().map(|(id,_)|id).collect();
                    let margin=24.0_f32;
                    let mut best:Option<(f64,Graph,Routes)>=None;
                    let mut rejected=0usize;
                    for columns in 1..=ids.len().min(12) {
                        for &gap_x in &[60.0_f32,90.0,130.0] {
                            for &gap_y in &[70.0_f32,110.0,160.0] {
                                let mut candidate=BTreeMap::new();
                                let mut y=margin;
                                for row_ids in ids.chunks(columns) {
                                    let mut x=margin;
                                    let mut row_height=0.0_f32;
                                    for &id in row_ids {
                                        let size=sizes[&id];
                                        candidate.insert(id,[x,y]);
                                        x+=size[0]+gap_x;
                                        row_height=row_height.max(size[1]);
                                    }
                                    y+=row_height+gap_y;
                                }
                                let candidate_boxes:Vec<_>=boxes_now.iter().map(|&(id,b)|{
                                    let old=graph.node(id).map(|n|n.pos).unwrap_or([0.0,0.0]);
                                    let new=candidate.get(&id).copied().unwrap_or(old);
                                    let dx=new[0]-old[0];let dy=new[1]-old[1];
                                    (id,Box2{min:[b.min[0]+dx,b.min[1]+dy],max:[b.max[0]+dx,b.max[1]+dy]})
                                }).collect();
                                let shifted:BTreeMap<_,_>=measured_ports.iter().map(|(&k,p)|{
                                    let shift=|id:NodeId,pt:[f32;2]|{
                                        let old=graph.node(id).map(|n|n.pos).unwrap_or([0.0,0.0]);
                                        let new=candidate.get(&id).copied().unwrap_or(old);
                                        [pt[0]+new[0]-old[0],pt[1]+new[1]-old[1]]
                                    };
                                    (k,PortPair{from:shift(k.0,p.from),to:shift(k.2,p.to)})
                                }).collect();
                                let mut proposed=graph.clone();
                                for n in &mut proposed.nodes {if let Some(&pos)=candidate.get(&n.id){n.pos=pos;}}
                                let Ok(mut new_routes)=maker_routing::route_all_separated(&proposed,&shifted,&candidate_boxes,*canvas_zoom) else {rejected+=1;continue};
                                let mut min=[f32::INFINITY;2];let mut max=[f32::NEG_INFINITY;2];
                                for &(_,b) in &candidate_boxes {
                                    for axis in 0..2 {min[axis]=min[axis].min(b.min[axis]);max[axis]=max[axis].max(b.max[axis]);}
                                }
                                let mut wire_length=0.0_f64;
                                let mut bends=0usize;
                                for path in new_routes.values(){
                                    for &pt in path {for axis in 0..2 {min[axis]=min[axis].min(pt[axis]);max[axis]=max[axis].max(pt[axis]);}}
                                    for seg in path.windows(2) {wire_length+=((seg[1][0]-seg[0][0]).abs()+(seg[1][1]-seg[0][1]).abs()) as f64;}
                                    bends+=path.len().saturating_sub(2);
                                }
                                let w=(max[0]-min[0]).max(1.0);let h=(max[1]-min[1]).max(1.0);
                                let fits=w+2.0*margin<=canvas.width()/ *canvas_zoom && h+2.0*margin<=canvas.height()/ *canvas_zoom;
                                let ratio=(w.max(h)/w.min(h)) as f64;
                                let area=(w as f64)*(h as f64);
                                // Fit first, then favor a square compact footprint;
                                // routing length and bends break near ties.
                                let score=(if fits {0.0} else {1.0e12})
                                    +area*(1.0+0.6*(ratio-1.0))
                                    +wire_length*8.0+(bends as f64)*300.0;
                                // Translate both modules and routes so the entire
                                // geometry (including returning wires) has a margin.
                                let dx=margin-min[0];let dy=margin-min[1];
                                for n in &mut proposed.nodes {n.pos[0]+=dx;n.pos[1]+=dy;}
                                for path in new_routes.values_mut(){for pt in path {pt[0]+=dx;pt[1]+=dy;}}
                                if best.as_ref().is_none_or(|(value,_,_)|score<*value) {
                                    best=Some((score,proposed,new_routes));
                                }
                            }
                        }
                    }
                    if let Some((_,proposed,new_routes))=best {
                        routing_undo.push((original,routes.clone()));
                        if routing_undo.len()>32{routing_undo.remove(0);}
                        routing_redo.clear();
                        *graph=proposed;*routes=new_routes;graph.revision+=1;
                        // Refit the virtual workspace to the optimized layout.
                        // The routing operation must never inherit previously
                        // expanded scrollbar ranges or grow them on its own.
                        let mut bounds_min=[f32::INFINITY;2];
                        let mut bounds_max=[f32::NEG_INFINITY;2];
                        for node in &graph.nodes {
                            let port_extent=staggered_port_height(node);
                            let size=[186.0,110.0_f32.max(55.0+port_extent)];
                            for axis in 0..2 {
                                bounds_min[axis]=bounds_min[axis].min(node.pos[axis]);
                                bounds_max[axis]=bounds_max[axis].max(node.pos[axis]+size[axis]);
                            }
                        }
                        // Include routed wires so no connector is clipped.
                        for path in routes.values() {
                            for point in path {
                                for axis in 0..2 {
                                    bounds_min[axis]=bounds_min[axis].min(point[axis]);
                                    bounds_max[axis]=bounds_max[axis].max(point[axis]);
                                }
                            }
                        }
                        // Normalize the actual final layout, not merely the candidate
                        // layout: node and routed-wire bounds must begin at +24 px.
                        // This keeps the optimized workspace origin at (0, 0),
                        // regardless of negative coordinates before optimization.
                        let margin=24.0_f32;
                        let offset=[margin-bounds_min[0],margin-bounds_min[1]];
                        for node in &mut graph.nodes {
                            node.pos[0]+=offset[0];
                            node.pos[1]+=offset[1];
                        }
                        for path in routes.values_mut() {
                            for point in path {
                                point[0]+=offset[0];
                                point[1]+=offset[1];
                            }
                        }
                        for axis in 0..2 {
                            let visible=if axis==0 {canvas.width()/ *canvas_zoom}
                                else {canvas.height()/ *canvas_zoom};
                            // At minimum show the full layout plus a 24 px margin
                            // on both sides; if it fits, show the viewport width.
                            let content_size=bounds_max[axis]-bounds_min[axis]+2.0*margin;
                            workspace_min[axis]=0.0;
                            workspace_max[axis]=content_size.max(visible);
                        }
                        // Center the normalized contents within the available view,
                        // while the subsequent common clamp enforces scroll bounds.
                        let cx=(margin+(bounds_max[0]-bounds_min[0])*0.5);
                        let cy=(margin+(bounds_max[1]-bounds_min[1])*0.5);
                        *canvas_pan=egui::vec2(canvas.width()*0.5-cx* *canvas_zoom,
                            canvas.height()*0.5-cy* *canvas_zoom);
                        *status="Optimized routing and fitted graph workspace to its contents (Undo available)".into();
                    } else {
                        *status=format!("Routing unchanged: no valid candidate ({rejected} tested)");
                    }
                } else {
                    // Repair only geometry when modules are moved or wired. Never
                    // rearrange modules except via the explicit toolbar action.
                    // A blocked connection must never erase routes for other wires.
                    // Recompute from the measured port geometry on every frame,
                    // including the first one, without requiring module movement.
                    let separated=maker_routing::route_all_separated(graph,&measured_ports,&boxes_now,*canvas_zoom);
                    match separated {
                        Ok(updated)=>*routes=updated,
                        Err(separation_error)=>{
                            // The all-graph validator rejects layouts containing
                            // overlapping module rectangles, even when individual
                            // connections have perfectly valid routes. Candidate
                            // selection must never depend on Optimize Routing to
                            // make the first set of wires visible.
                            let mut repaired=Routes::new();
                            let mut failures=Vec::new();
                            for connection in &graph.connections {
                                let k=maker_routing::key(connection);
                                match measured_ports.get(&k) {
                                    Some(pair)=>match maker_routing::route(
                                        pair.from,pair.to,&boxes_now,connection.from,connection.to) {
                                        Ok(path)=>{repaired.insert(k,path);},
                                        Err(error)=>failures.push(format!("{k:?}: {error}")),
                                    },
                                    None=>failures.push(format!("{k:?}: port geometry missing")),
                                }
                            }
                            // Retain a previously computed path only for the same
                            // connection; never carry a route over to a different
                            // candidate connection key.
                            for connection in &graph.connections {
                                let k=maker_routing::key(connection);
                                if !repaired.contains_key(&k) {
                                    if let Some(path)=routes.get(&k) {
                                        repaired.insert(k,path.clone());
                                    }
                                }
                            }
                            let visible=repaired.len();
                            *routes=repaired;
                            if !failures.is_empty() {
                                *status=format!("Routing: {visible}/{} wires visible; {} failed. First: {}. Lane routing: {}",
                                    graph.connections.len(),failures.len(),failures[0],separation_error);
                            }
                        }
                    }
                }
                let painter=ctx.layer_painter(egui::LayerId::new(egui::Order::Background,egui::Id::new("maker_wire_layer"))).with_clip_rect(canvas);
                let wire_paths: Vec<(Vec<egui::Pos2>,egui::Color32)> = graph.connections.iter().filter_map(|c| {
                    let a=*output_ports.get(&(c.from,c.from_port))?;
                    let b=*input_ports.get(&(c.to,c.to_port))?;
                    let ty=graph.node(c.from)?.output_port(c.from_port)?.ty;
                    let color=port_color(&ty);
                    // Never draw a fallback path that could cross a module or
                    // appear to connect anywhere other than its actual port.
                    let path:Vec<egui::Pos2>=routes.get(&maker_routing::key(c))?
                        .iter().map(|v|egui::pos2(v[0]* *canvas_zoom+origin.x,v[1]* *canvas_zoom+origin.y)).collect();
                    Some((path,color))
                }).collect();
                for (index,(path,color)) in wire_paths.iter().enumerate() {
                    let bridged = bridge_crossings(index, path, &wire_paths);
                    painter.add(egui::Shape::line(bridged,egui::Stroke::new(2.5,*color)));
                }
                // Compatibility rings are drawn in a separate foreground pass below.
                // Drawing them on the Middle wire layer hides them behind node bodies.
                // Live wire follows the pointer while the source port is dragged.
                if let Some((id, port)) = *dragging_wire {
                    if let (Some(&start), Some(end)) = (output_ports.get(&(id, port)), pointer_pos) {
                        painter.add(egui::Shape::line(
                            orthogonal_wire(start, end),
                            egui::Stroke::new(2.5, egui::Color32::from_rgb(255, 196, 92)),
                        ));
                    }
                }
                // Edge-triggered auto-pan during an actual node-header drag.
                // Move the viewport and the dragged node by equal screen-space
                // amounts: the node stays under the pointer without jumping.
                if active_node_drag {
                    if let Some(pointer)=ctx.input(|i|i.pointer.interact_pos()) {
                        let zone=32.0_f32;
                        let edge_speed=|distance:f32| -> f32 {
                            if distance>=zone {0.0} else {
                                let strength=((zone-distance)/zone).clamp(0.0,1.0);
                                100.0*strength+500.0*strength*strength
                            }
                        };
                        let dx=edge_speed(pointer.x-canvas.left())
                            -edge_speed(canvas.right()-pointer.x);
                        let dy=edge_speed(pointer.y-canvas.top())
                            -edge_speed(canvas.bottom()-pointer.y);
                        let dt=ctx.input(|i|i.stable_dt).clamp(0.0,0.05);
                        let movement=egui::vec2(dx,dy)*dt;
                        if movement.length_sq()>0.0 {
                            *canvas_pan+=movement;
                            if let Some(id)=*selected {
                                if let Some(n)=graph.node_mut(id) {
                                    n.pos[0]-=movement.x/ *canvas_zoom;
                                    n.pos[1]-=movement.y/ *canvas_zoom;
                                }
                            }
                            ctx.request_repaint();
                        }
                    }
                }
                // Keep the scrollable workspace large enough for every module,
                // including rows created by Add All, candidate restoration, and
                // automatic layout. Previously only node dragging expanded it,
                // so the vertical thumb could reach its end above the last rows.
                // Bounds are monotonic: ordinary scrolling must not grow them.
                const WORKSPACE_CLEARANCE: f32 = 64.0;
                for node in &graph.nodes {
                    let port_extent=staggered_port_height(node);
                    let height=110.0_f32.max(55.0+port_extent);
                    workspace_min[0]=workspace_min[0].min(node.pos[0]-WORKSPACE_CLEARANCE);
                    workspace_min[1]=workspace_min[1].min(node.pos[1]-WORKSPACE_CLEARANCE);
                    workspace_max[0]=workspace_max[0].max(node.pos[0]+186.0+WORKSPACE_CLEARANCE);
                    workspace_max[1]=workspace_max[1].max(node.pos[1]+height+WORKSPACE_CLEARANCE);
                }
                // Wire detours can extend beyond module frames, and must also
                // be reachable with the scrollbars at every supported zoom.
                for path in routes.values() {
                    for point in path {
                        workspace_min[0]=workspace_min[0].min(point[0]-WORKSPACE_CLEARANCE);
                        workspace_min[1]=workspace_min[1].min(point[1]-WORKSPACE_CLEARANCE);
                        workspace_max[0]=workspace_max[0].max(point[0]+WORKSPACE_CLEARANCE);
                        workspace_max[1]=workspace_max[1].max(point[1]+WORKSPACE_CLEARANCE);
                    }
                }
                // Workspace boundaries are stable under scrollbar movement.
                // Only a node being actively dragged may grow the workspace
                // beyond the actual graph content.
                let view_size=[canvas.width()/ *canvas_zoom,canvas.height()/ *canvas_zoom];
                if active_node_drag {
                    for node in &graph.nodes {
                        let port_extent=staggered_port_height(node);
                        let height=110.0_f32.max(55.0+port_extent);
                        workspace_min[0]=workspace_min[0].min(node.pos[0]-48.0);
                        workspace_min[1]=workspace_min[1].min(node.pos[1]-48.0);
                        workspace_max[0]=workspace_max[0].max(node.pos[0]+186.0+48.0);
                        workspace_max[1]=workspace_max[1].max(node.pos[1]+height+48.0);
                    }
                    // Grow in the direction of an edge-triggered drag only.
                    // Do not expand merely because the viewport is near an edge.
                    if let Some(pointer)=ctx.input(|i|i.pointer.interact_pos()) {
                        let edge=32.0;
                        let view_min=[-canvas_pan.x/ *canvas_zoom,-canvas_pan.y/ *canvas_zoom];
                        for axis in 0..2 {
                            let (coord,lo,hi)=if axis==0 {(pointer.x,canvas.left(),canvas.right())}
                                else {(pointer.y,canvas.top(),canvas.bottom())};
                            if coord<=lo+edge && view_min[axis]<=workspace_min[axis]+view_size[axis]*0.2 {
                                workspace_min[axis]-=view_size[axis].max(250.0);
                            }
                            if coord>=hi-edge && view_min[axis]+view_size[axis]>=workspace_max[axis]-view_size[axis]*0.2 {
                                workspace_max[axis]+=view_size[axis].max(250.0);
                            }
                        }
                    }
                }
                // Keep the visible region inside the finite current workspace.
                // This also handles zooming out beyond the current workspace size.
                for axis in 0..2 {
                    let extent=workspace_max[axis]-workspace_min[axis];
                    if extent<view_size[axis] {
                        let center=(workspace_min[axis]+workspace_max[axis])*0.5;
                        workspace_min[axis]=center-view_size[axis]*0.5;
                        workspace_max[axis]=center+view_size[axis]*0.5;
                    }
                    // Do not clamp the viewport here. Center Graph and
                    // Optimize Routing intentionally position graph content
                    // independently of scrollbar limits, particularly when
                    // zoomed in. The scrollbar interaction itself clamps its
                    // own requested position. A per-frame clamp here caused
                    // one-frame centering followed by a snap to the corner.
                }
                // Scrollbar widgets must win hit testing over graph-node Areas.
                // Keep their geometry and pan calculations unchanged; only their
                // interaction/paint layer is elevated above node Areas.
                ui.with_layer_id(egui::LayerId::new(
                    if project_dialog.is_some() || unsaved_action.is_some() || glsl_export_dialog.is_some() { egui::Order::Background } else { egui::Order::Foreground },
                    egui::Id::new("maker_graph_scrollbar_interaction")), |ui| {
                    graph_scrollbar(ui,canvas,available,true,canvas_pan,*canvas_zoom,
                        workspace_min[0],workspace_max[0],glsl_export_dialog.is_none());
                    graph_scrollbar(ui,canvas,available,false,canvas_pan,*canvas_zoom,
                        workspace_min[1],workspace_max[1],glsl_export_dialog.is_none());
                });
                ui.allocate_rect(available,egui::Sense::hover());
            });
            // Node Areas use Middle order so their painted bodies are above the
            // Background CentralPanel. They are non-interactable whenever the
            // pointer is outside the graph canvas, including the header region.
            // Do not reorder the header or node layers after widget interaction:
            // that changes paint order without updating already-computed responses.
            // Module-attached editor is a foreground Area, not a Popup. Its
            // lifetime is independent of nested color-picker popups.
            // The contents are only interactive while the editor is open.
            let editor_id = egui::Id::new("maker_attached_parameter_editor");
            let is_open = parameter_popup.is_some();
            let progress = ctx.animate_bool_with_time(editor_id, is_open, 0.18);
            if let Some(id) = *parameter_popup {
                if graph.node(id).is_none() { *parameter_popup = None; }
            }
            *parameter_wheel_rect = egui::Rect::NOTHING;
            if let Some(id) = *parameter_popup {
                if let Some((anchor_id, anchor_pos)) = editor_anchor {
                    if anchor_id == id {
                        let bounds = *canvas_rect;
                        let width = 360.0_f32.min((bounds.width() - 16.0).max(200.0));
                        let max_height = (bounds.height() - 16.0).max(120.0);
                        let target_height = 410.0_f32.min(max_height);
                        // The panel unfolds directly below the module. Keep its
                        // horizontal origin aligned with the module's left edge.
                        let x = anchor_pos.x.clamp(bounds.left() + 8.0,
                            (bounds.right() - width - 8.0).max(bounds.left() + 8.0));
                        // Preserve the bottom-edge attachment. If the module is
                        // near the bottom, reduce the panel's visible height and
                        // scroll its controls rather than overlay the module.
                        let y = anchor_pos.y;
                        let room_below = (bounds.bottom() - y - 8.0).max(0.0);
                        let visible_height = target_height.min(room_below);
                        let parameter_area = egui::Area::new(egui::Id::new(("maker_parameter_area", id)))
                            .order(egui::Order::Foreground)
                            .movable(false)
                            .interactable(ctx.input(|i| {
                                i.pointer.hover_pos().is_some_and(|p| bounds.contains(p))
                            }))
                            .fixed_pos(egui::pos2(x, y))
                            .show(ctx, |ui| {
                                ui.set_clip_rect(ui.clip_rect().intersect(bounds));
                                ui.set_width(width);
                                ui.set_max_height(visible_height);
                                // Clip only the opening animation. Once expanded,
                                // nested widgets use their normal independent layers.
                                if progress < 1.0 || visible_height < target_height {
                                    let clip = egui::Rect::from_min_size(
                                        ui.min_rect().min, egui::vec2(width, visible_height * progress));
                                    ui.set_clip_rect(ui.clip_rect().intersect(clip));
                                }
                                egui::Frame::window(ui.style()).show(ui, |ui| {
                                    ui.set_width(width - 24.0);
                                    ui.horizontal(|ui| {
                                        ui.strong(format!("{} #{} — Parameters",
                                            graph.node(id).unwrap().definition().name, id));
                                        if ui.button("✕").on_hover_text("Close parameter editor").clicked() {
                                            *parameter_popup = None;
                                        }
                                    });
                                    ui.separator();
                                    egui::ScrollArea::vertical()
                                        .max_height((visible_height - 64.0).max(24.0))
                                        .show(ui, |ui| {
                                            if *parameter_popup == Some(id) {
                                                let exploration = graph.connections.iter().any(|c| {
                                                    (c.from == id && graph.node(c.to).is_some_and(|n| n.module_id == "screenshaver.test.universal_test")) ||
                                                    (c.to == id && graph.node(c.from).is_some_and(|n| n.module_id == "screenshaver.test.universal_test"))
                                                }) && graph.node(id).is_some_and(|n| n.module_id != "screenshaver.test.universal_test");
                                                if let Some(node) = graph.node_mut(id) { edit_parameters(ui, node, exploration); }

                                            }
                                        });
                                });
                            });
                        *parameter_wheel_rect = parameter_area.response.rect.intersect(bounds);
                    }
                }
            }
            if parameter_popup.is_some() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                *parameter_popup = None;
            }
            // The code viewer is read-only and uses the original JSON loaded by
            // Maker (including bundled official definitions), not reconstructed GLSL.
            // A foreground shield prevents clicks reaching the graph and docks.
            if let Some(module_id) = examine_module.clone() {
                let library = maker_library::get();
                let module_name = library.module(&module_id)
                    .map(|module| module.name.as_str()).unwrap_or(module_id.as_str());
                let source = library.source(&module_id).unwrap_or("Module source unavailable");
                let screen = ctx.screen_rect();
                egui::Area::new(egui::Id::new("maker_examine_code_shield"))
                    .order(egui::Order::Foreground)
                    .fixed_pos(screen.min)
                    .interactable(true)
                    .show(ctx, |ui| {
                        ui.allocate_rect(egui::Rect::from_min_size(ui.min_rect().min, screen.size()), egui::Sense::click());
                    });
                let mut window_open = true;
                let mut close_button = false;
                egui::Window::new(format!("Examine Module Code — {module_name}"))
                    .id(egui::Id::new("maker_examine_module_code"))
                    .order(egui::Order::Foreground)
                    .open(&mut window_open)
                    .collapsible(false)
                    .resizable(true)
                    .movable(true)
                    .default_size(egui::vec2(780.0, 570.0))
                    .show(ctx, |ui| {
                        ui.label(format!("Module ID: {module_id}"));
                        ui.separator();
                        // The modal shield and the graph's SDL wheel handler
                        // can otherwise keep this ScrollArea from receiving wheel
                        // input. Route wheel movement to the viewer viewport,
                        // consuming it once so the underlying graph cannot pan.
                        let code_viewport = ui.available_rect_before_wrap().intersect(ui.clip_rect());
                        let code_wheel = if ui.ctx().input(|i| i.pointer.hover_pos()
                            .is_some_and(|p| code_viewport.contains(p))) {
                            ui.input_mut(|i| {
                                let delta = i.smooth_scroll_delta;
                                i.smooth_scroll_delta = egui::Vec2::ZERO;
                                delta
                            })
                        } else { egui::Vec2::ZERO };
                        egui::ScrollArea::both().auto_shrink([false, false])
                            .max_height((ui.available_height() - 38.0).max(120.0))
                            .show(ui, |ui| {
                                if code_wheel != egui::Vec2::ZERO {
                                    ui.scroll_with_delta(code_wheel);
                                }
                                ui.add(egui::Label::new(egui::RichText::new(source).monospace()).selectable(true));
                            });
                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.button("Copy to Clipboard").clicked() {
                                // Maker owns the SDL2 event loop; egui copy_text alone
                                // does not update the native clipboard. Queue the exact
                                // .smmod source for Maker's existing SDL2 clipboard writer.
                                *pending_clipboard=Some(source.to_owned());
                                *pending_clipboard_is_glsl=false;
                                *pending_clipboard_is_module=true;
                                *status="Copying module JSON to system clipboard".into();
                            }
                            if ui.button("Close").clicked() { close_button = true; }
                        });
                    });
                if !window_open || close_button || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                    *examine_module = None;
                } else {
                    ctx.move_to_top(egui::LayerId::new(
                        egui::Order::Foreground, egui::Id::new("maker_examine_module_code")));
                }
            }
            // Keep Inspector controls above graph-node Areas within Middle order.
            // ComboBox popups use Foreground order, so they must remain above
            // Inspector controls rather than being occluded by them.
            if project_dialog.is_none() && unsaved_action.is_none() && glsl_export_dialog.is_none() {
                ctx.move_to_top(egui::LayerId::new(
                    egui::Order::Middle, egui::Id::new("maker_inspector_interaction")));
            }
            // Windows are created before panels in this UI pass. Raise the project
            // browser after the menu, Inspector, scrollbars and graph have all been
            // registered. This changes only the browser's z-order, not the node
            // paint order or the foreground protection of underlying controls.
            if project_dialog.is_some() {
                ctx.move_to_top(egui::LayerId::new(
                    egui::Order::Foreground,
                    egui::Id::new("maker_project_file_browser"),
                ));
            }
            // Export shares the foreground order of the project browser.
            if glsl_export_dialog.is_some() {
                ctx.move_to_top(egui::LayerId::new(
                    egui::Order::Foreground, egui::Id::new("maker_export_glsl")));
            }
            // The Unsaved Changes confirmation is modal as well. Raise it above
            // the foreground menu and restore Inspector/scrollbar priority when
            // it closes (the layer choice above is evaluated each frame).
            if unsaved_action.is_some() {
                ctx.move_to_top(egui::LayerId::new(
                    egui::Order::Foreground,
                    egui::Id::new("maker_unsaved_changes"),
                ));
            }

        });
        if graph.nodes.iter().any(|n|old_parameters.iter().find(|(id,_)|*id==n.id).is_some_and(|(_,p)|*p!=n.parameters)){graph.revision+=1;}
        let shapes=self.context.tessellate(output.shapes,output.pixels_per_point);
        self.painter.paint_and_update_textures([dw,dh],output.pixels_per_point,&shapes,&output.textures_delta);
        if saved_thumbnail_this_frame { self.move_saved_thumbnail_to_end(); }
        // Resolve a tab X through the established close command, including its
        // unsaved-changes confirmation. Switching is internal and never saves or
        // discards the target graph; it simply makes that tab the close target.
        if let Some(index)=requested_close_tab_index {
            if self.project_open && index<=self.inactive_tabs.len()
                && self.unsaved_action.is_none() && !self.close_all_running {
                if index!=self.active_tab {self.switch_project_tab(index);}
                self.pending_menu_command=Some("close");
                self.context.request_repaint();
            }
            return self.open;
        }
        if requested_start_close_all {self.close_all_running=true;}
        if let Some(graph)=requested_candidate_tab {self.show_candidate_thumbnail(graph);}
        else if requested_close_tab {self.close_active_project_tab();}
        else if let Some((path,graph))=requested_open_project {self.open_project_in_tab(path,graph);}
        else if requested_new_tab {self.create_project_tab();}
        else if let Some(index)=requested_tab {self.switch_project_tab(index);}
        self.advance_close_all();
        self.open
    }
    /// Request a graceful application shutdown through the existing per-project
    /// Save / Discard / Cancel workflow. A canceled confirmation leaves the
    /// editor and its project tabs intact.
    pub fn request_application_close(&mut self) {
        if !self.close_all_running {
            self.close_all_running = true;
            self.context.request_repaint();
        }
    }

    /// True only after all projects have been closed by the existing workflow.
    /// In particular, canceling a confirmation does not satisfy this condition.
    /// The close-all operation ended without closing every project (Cancel).
    pub fn application_close_cancelled(&self) -> bool {
        !self.close_all_running && self.project_open
    }

    pub fn application_close_ready(&self) -> bool {
        !self.project_open && !self.close_all_running
    }

    pub fn destroy(&mut self){self.painter.destroy();}
}
fn edit_parameters(ui:&mut egui::Ui,node:&mut crate::maker_graph::Node,exploration:bool){
    let module=node.definition();
    if module.parameters.is_empty(){ui.small(module.description.as_str());}
    if exploration {ui.small("Test exploration: expanded numeric range (3×), Shift for fine adjustment.");}
    for p in &module.parameters {
        let Some(value)=node.parameters.get_mut(&p.id) else {continue};
        match p.ty.as_str(){
            "choice"=>{
                let mut selected=value.as_f64().unwrap_or(0.0) as usize;
                selected=selected.min(p.options.len().saturating_sub(1));
                ui.label(&p.name);
                egui::ComboBox::from_id_salt((node.id,&p.id,"choice"))
                    .selected_text(p.options.get(selected).map(String::as_str).unwrap_or("Select"))
                    .show_ui(ui,|ui|{
                        for (index,label) in p.options.iter().enumerate(){
                            ui.selectable_value(&mut selected,index,label);
                        }
                    });
                if value.as_f64()!=Some(selected as f64){*value=serde_json::json!(selected as f64);}
            }
            "float"=>{
                // Match Control Center's draw_fine_slider: one authoritative
                // pointer-driven value calculation, with Shift = 10x finer.
                let previous=value.as_f64().unwrap_or(0.0) as f32;
                let span=(p.max-p.min).max(0.0);
                let low=if exploration && p.min<0.0 {p.min-span} else {p.min};
                let high=if exploration {p.max+2.0*span} else {p.max};
                ui.label(&p.name);
                let desired_size=egui::vec2(ui.available_width().max(80.0),ui.spacing().interact_size.y);
                let (rect,response)=ui.allocate_exact_size(desired_size,egui::Sense::click_and_drag());
                let state_id=response.id.with("maker_fine_slider_anchor");
                let shift=ui.input(|i|i.modifiers.shift);
                let pointer=response.interact_pointer_pos();
                if response.drag_started() {
                    if let Some(pos)=pointer {
                        ui.ctx().data_mut(|d|d.insert_temp(state_id,(previous,pos.x,shift)));
                    }
                }
                let mut next=previous;
                if response.dragged() {
                    if let Some(pos)=pointer {
                        let mut state=ui.ctx().data_mut(|d|d.get_temp::<(f32,f32,bool)>(state_id));
                        if state.is_none(){state=Some((previous,pos.x,shift));}
                        if let Some((mut anchor,mut anchor_x,mut was_shift))=state {
                            if was_shift!=shift {
                                anchor=previous;
                                anchor_x=pos.x;
                                was_shift=shift;
                            }
                            let sensitivity=if shift {0.1} else {1.0};
                            next=(anchor+(pos.x-anchor_x)/rect.width().max(1.0)
                                *(high-low)*sensitivity).clamp(low,high);
                            ui.ctx().data_mut(|d|d.insert_temp(state_id,(anchor,anchor_x,was_shift)));
                        }
                    }
                } else if response.clicked() {
                    if let Some(pos)=pointer {
                        let fraction=((pos.x-rect.left())/rect.width().max(1.0)).clamp(0.0,1.0);
                        next=low+fraction*(high-low);
                    }
                } else {
                    ui.ctx().data_mut(|d|d.remove::<(f32,f32,bool)>(state_id));
                }
                if (next-previous).abs()>f32::EPSILON {*value=serde_json::json!(next);}
                let fraction=((next-low)/(high-low).max(f32::EPSILON)).clamp(0.0,1.0);
                let visuals=if response.dragged(){&ui.visuals().widgets.active}
                    else if response.hovered(){&ui.visuals().widgets.hovered}
                    else {&ui.visuals().widgets.inactive};
                let track=egui::Rect::from_center_size(rect.center(),egui::vec2(rect.width(),4.0));
                ui.painter().rect_filled(track,2.0,visuals.bg_fill);
                let knob=egui::pos2(rect.left()+fraction*rect.width(),rect.center().y);
                ui.painter().circle_filled(knob,7.0,visuals.fg_stroke.color);
                ui.label(format!("{next:.2}"));
            }
            "color3"=>{
                if let Some(a)=value.as_array(){
                    if a.len()==3{
                        let mut rgb=[a[0].as_f64().unwrap_or(0.0) as f32,a[1].as_f64().unwrap_or(0.0) as f32,a[2].as_f64().unwrap_or(0.0) as f32];
                        ui.label(&p.name);
                        if ui.color_edit_button_rgb(&mut rgb).changed(){*value=serde_json::json!(rgb);}
                    }
                }
            }
            _=>{}
        }
    }
}

fn port_color(ty:&str)->egui::Color32{
    match ty {
        "float"=>egui::Color32::from_rgb(104,195,237),
        "vec2"=>egui::Color32::from_rgb(123,207,152),
        "vec3"=>egui::Color32::from_rgb(229,173,96),
        "field2d"|"field3d"=>egui::Color32::from_rgb(224,64,251),
        _=>egui::Color32::GRAY,
    }
}

/// Right-angle routing with two horizontal runs and a vertical middle segment.
fn orthogonal_wire(a: egui::Pos2,b: egui::Pos2)->Vec<egui::Pos2>{
    let mid = if b.x >= a.x + 32.0 { (a.x+b.x)*0.5 } else { a.x+24.0 };
    vec![a,egui::pos2(mid,a.y),egui::pos2(mid,b.y),b]
}

/// Add a semicircular jump on the vertical segment where another wire's
/// horizontal segment crosses it. This is presentation-only, never connectivity.
fn bridge_crossings(index:usize,path:&[egui::Pos2],all:&[(Vec<egui::Pos2>,egui::Color32)])->Vec<egui::Pos2>{
    if path.len()<2{return path.to_vec()}
    let mut result=vec![path[0]];
    for segment in path.windows(2){
        let a=segment[0];let b=segment[1];
        if (a.x-b.x).abs()>0.1 || (a.y-b.y).abs()<18.0 {result.push(b);continue}
        let direction=if b.y>=a.y{1.0}else{-1.0};
        let mut crossings=Vec::new();
        for (other_index,(other,_)) in all.iter().enumerate(){
            if index==other_index {continue}
            for horizontal in other.windows(2){
                let c=horizontal[0];let d=horizontal[1];
                if (c.y-d.y).abs()>0.1{continue}
                if a.x>c.x.min(d.x)+7.0 && a.x<c.x.max(d.x)-7.0 &&
                    c.y>a.y.min(b.y)+9.0 && c.y<a.y.max(b.y)-9.0 {crossings.push(c.y);}
            }
        }
        crossings.sort_by(|x,y|if direction>0.0{x.total_cmp(y)}else{y.total_cmp(x)});
        crossings.dedup_by(|x,y|(*x-*y).abs()<13.0);
        let mut last=a.y;
        for y in crossings {
            if (y-last)*direction<12.0 || (b.y-y)*direction<9.0{continue}
            let radius=6.0;
            result.push(egui::pos2(a.x,y-radius*direction));
            for step in 1..=12{
                let theta=std::f32::consts::PI*(step as f32/12.0);
                result.push(egui::pos2(a.x+radius*theta.sin(),y-radius*direction*theta.cos()));
            }
            last=y+radius*direction;
        }
        result.push(b);
    }
    result
}

/// Explain the port contract and the exact validation outcome for the selected source.
fn port_hint(graph: &Graph, to: NodeId, to_port: usize,
    pending: Option<(NodeId, usize)>, dragging: Option<(NodeId, usize)>) -> String {
    let Some(node) = graph.node(to) else { return "Destination node missing".into() };
    let Some(port) = node.definition().inputs.get(to_port) else { return "Input port missing".into() };
    let name=&port.name;let ty=&port.ty;
    let occupied = graph.input_connection(to, to_port).is_some();
    let mut hint = format!("{} input ({})\nOne incoming connection; a new source replaces the old one.", name, ty);
    if occupied { hint.push_str("\nCurrently connected."); }
    if let Some((from, from_port)) = dragging.or(pending) {
        match graph.check_connection(from, from_port, to, to_port) {
            Ok(()) => hint.push_str("\nCompatible: connect here."),
            Err(reason) => hint.push_str(&format!("\nCannot connect: {reason}")),
        }
    } else {
        hint.push_str("\nClick a source output first; click this input without a source to disconnect.");
    }
    hint
}

/// Central Maker category palette. Change colors here, not in individual .smmod files.
/// These colors are used by category headers, library buttons and graph node titles.
fn maker_category_color(category: &str) -> egui::Color32 {
    let rgb = match category.to_ascii_lowercase().as_str() {
        "animation" => [153, 57, 102],
        "compositing" => [95, 78, 186],
        "camera" => [0, 119, 133],
        "color" => [207, 45, 157],
        "coordinates" => [35, 91, 177],
        "fields" => [38, 122, 73],
        "geometry" => [169, 79, 31],
        "lighting" => [132, 113, 0],
        "math" => [16, 143, 204],
        "noise" => [118, 54, 153],
        "output" => [190, 45, 31],
        "test" => [83, 125, 137],
        "other" => [83, 91, 99],
        "third-party" => [105, 95, 123],
        // Unrecognized categories have a stable fallback until assigned a color.
        _ => [83, 91, 99],
    };
    egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

// Graph-space port pitch includes egui's vertical item spacing at render time.
fn staggered_port_height(node: &crate::maker_graph::Node) -> f32 {
    let inputs = node.definition().inputs.len() as f32;
    let outputs = node.output_count() as f32;
    (inputs * 25.0).max(if outputs > 0.0 { outputs * 25.0 + 12.5 } else { 0.0 })
}

fn node_header_color(module:&Module)->egui::Color32 {
    // The terminal Output Color node uses a neutral dark-gray header.
    if module.id == "screenshaver.output.color" {
        return egui::Color32::from_rgb(65, 70, 76);
    }
    let category = maker_library::get().categories.get(&module.id)
        .map(String::as_str).unwrap_or("Other");
    maker_category_color(category)
}

fn module_library_tooltip(ui:&mut egui::Ui,module:&Module){
    ui.heading(&module.name);ui.small(&module.category);ui.separator();
    ui.label(&module.description);ui.add_space(5.0);
    ui.strong("Inputs");
    if module.inputs.is_empty(){ui.label("None");}
    for p in &module.inputs{ui.label(format!("{} ({})",p.name,p.ty));}
    ui.strong("Outputs");
    if module.outputs.is_empty(){ui.label("None (terminal node)");}
    for p in &module.outputs{ui.label(format!("{} ({})",p.name,p.ty));}
    ui.strong("Parameters");
    if module.parameters.is_empty(){ui.label("None");}
    for p in &module.parameters{ui.label(format!("{} ({})",p.name,p.ty));}
    ui.strong("Typical uses");ui.label(&module.uses);
    ui.strong("Behavioral notes");ui.label(&module.notes);
    ui.separator();ui.label(format!("Created by: {}",module.created_by));
    if let Some(email)=&module.creator_email {ui.label(format!("Creator Email: {email}"));}
    for name in &module.attributed_to {ui.label(format!("Attributed to: {name}"));}
    if module.trust==Trust::Community{
        ui.colored_label(egui::Color32::YELLOW,"This is a third-party module, not provided by Screenshaver");
    }else{ui.small("Official Screenshaver Library");}
}

#[path = "maker_discovery.rs"]
mod maker_discovery;
pub fn run_discovery_test()->Result<(),String>{maker_discovery::run()}

/// Scrollbar positions are graph-space values, not node-space mutations.
/// A drag on the track jumps toward the pointer; a drag on the thumb scrolls
/// proportionally to its available travel. The ranges grow as needed.
fn graph_scrollbar(ui:&mut egui::Ui,canvas:egui::Rect,available:egui::Rect,
    horizontal:bool,pan:&mut egui::Vec2,zoom:f32,low:f32,high:f32,interactive:bool) {
    let track=if horizontal {
        egui::Rect::from_min_max(egui::pos2(canvas.left(),canvas.bottom()+2.0),
            egui::pos2(canvas.right(),available.bottom()-2.0))
    } else {
        egui::Rect::from_min_max(egui::pos2(canvas.right()+2.0,canvas.top()),
            egui::pos2(available.right()-2.0,canvas.bottom()))
    };
    let length=if horizontal {track.width()} else {track.height()};
    if length<=4.0 {return;}
    let extent=(high-low).max(1.0);
    let visible=if horizontal {canvas.width()/zoom} else {canvas.height()/zoom};
    let scrollable=(extent-visible).max(0.0);
    let thumb_length=(length*(visible/extent)).clamp(24.0,length);
    let travel=(length-thumb_length).max(0.0);
    let position=if horizontal {-pan.x/zoom} else {-pan.y/zoom};
    let fraction=if scrollable>0.0 {((position-low)/scrollable).clamp(0.0,1.0)} else {0.0};
    let offset=fraction*travel;
    let thumb=if horizontal {
        egui::Rect::from_min_size(egui::pos2(track.left()+offset,track.top()),
            egui::vec2(thumb_length,track.height()))
    } else {
        egui::Rect::from_min_size(egui::pos2(track.left(),track.top()+offset),
            egui::vec2(track.width(),thumb_length))
    };
    ui.painter().rect_filled(track,3.0,egui::Color32::from_rgb(48,59,76));
    ui.painter().rect_filled(thumb,3.0,egui::Color32::from_rgb(139,161,190));
    if !interactive {return;}
    let axis=if horizontal {"horizontal"} else {"vertical"};
    let track_response=ui.interact(track,egui::Id::new(("maker_scroll_track",axis)),egui::Sense::click());
    let thumb_response=ui.interact(thumb,egui::Id::new(("maker_scroll_thumb",axis)),egui::Sense::click_and_drag());
    if thumb_response.dragged() && travel>0.0 {
        let delta=ui.input(|i|i.pointer.delta());
        let component=if horizontal {delta.x} else {delta.y};
        // A viewport-sized workspace can round to high-visible < low.
        let upper=(high-visible).max(low);
        let new_position=(position+component*scrollable/travel).clamp(low,upper);
        if horizontal {pan.x=-new_position*zoom;} else {pan.y=-new_position*zoom;}
    } else if track_response.clicked() && scrollable>0.0 {
        if let Some(pointer)=track_response.interact_pointer_pos() {
            let coord=if horizontal {pointer.x-track.left()} else {pointer.y-track.top()};
            let fraction=((coord-thumb_length*0.5)/travel.max(1.0)).clamp(0.0,1.0);
            let new_position=low+fraction*scrollable;
            if horizontal {pan.x=-new_position*zoom;} else {pan.y=-new_position*zoom;}
        }
    }
}

#[cfg(test)]
mod callable_quality_regression {
    use super::*;
    #[test]
    fn unconnected_normal_position_is_deprioritized_not_rejected() {
        crate::maker_library::initialize().expect("initialize Maker modules");
        let mut graph=Graph::blank();
        let sphere=graph.add("screenshaver.geometry.sdf_sphere",[0.0,0.0]).unwrap();
        let normal=graph.add("screenshaver.geometry.sdf_normal_estimation",[0.0,0.0]).unwrap();
        let source=graph.node(sphere).unwrap();
        let callable=source.definition().outputs.len();
        let normal_node=graph.node(normal).unwrap();
        let field=normal_node.definition().inputs.iter().position(|p|p.ty=="field3d").unwrap();
        graph.connect(sphere,callable,normal,field).unwrap();
        // Structural penalty is independent of whether the graph is renderable.
        // The normal still has its documented vec3(0.0) default.
        assert_eq!(callable_sampling_penalty(&graph),0); // not on output path
        let output=graph.nodes.iter().find(|n|n.definition().terminal).unwrap().id;
        graph.connect(normal,0,output,0).unwrap();
        assert_eq!(callable_sampling_penalty(&graph),40);
        assert!(graph.validate_renderable().is_ok());
    }
}

#[cfg(test)]
mod safety_estimator_regression {
    use super::*;
    #[test]
    fn fold_cost_preserves_maximum_and_tracks_glsl_rounding(){
        assert_eq!(fold_iteration_work(Some(6.0)),4416);
        assert_eq!(fold_iteration_work(Some(24.0)),15360);
        assert_eq!(fold_iteration_work(Some(5.5)),4416);
        assert_eq!(fold_iteration_work(Some(-1.0)),1376);
        assert_eq!(fold_iteration_work(Some(100.0)),15360);
        assert_eq!(fold_iteration_work(None),15360);
        assert_eq!(fold_iteration_work(Some(f64::NAN)),15360);
    }
}
