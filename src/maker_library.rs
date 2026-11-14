//! Built-ins are compiled into the binary. Community files cannot impersonate them.
use crate::maker_module::{Module,Trust};
use std::{collections::BTreeMap,fs,path::Path,sync::OnceLock};
static LIBRARY:OnceLock<Library>=OnceLock::new();
pub struct Library { pub modules:BTreeMap<String,Module>, pub errors:Vec<String> }
pub fn get()->&'static Library{LIBRARY.get().expect("Maker library must initialize before graph creation")}
pub fn initialize()->Result<(),String>{LIBRARY.set(Library::load()).map_err(|_|"Maker library already initialized".into())}
impl Library {
 fn load()->Self{
    let mut result=Self{modules:BTreeMap::new(),errors:Vec::new()};
    let official=[
      ("coordinates/uv_coordinates.smmod",include_str!("../assets/maker/modules/coordinates/uv_coordinates.smmod")),
      ("noise/value_noise_2d.smmod",include_str!("../assets/maker/modules/noise/value_noise_2d.smmod")),
      ("math/multiply.smmod",include_str!("../assets/maker/modules/math/multiply.smmod")),
      ("color/cosine_palette.smmod",include_str!("../assets/maker/modules/color/cosine_palette.smmod")),
      ("output/output_color.smmod",include_str!("../assets/maker/modules/output/output_color.smmod")),
      ("fields/morphing_field.smmod",include_str!("../assets/maker/modules/fields/morphing_field.smmod")),
      ("fields/field_traversal.smmod",include_str!("../assets/maker/modules/fields/field_traversal.smmod")),
      ("color/color_accumulation.smmod",include_str!("../assets/maker/modules/color/color_accumulation.smmod")),
    ];
    for (path,src) in official {result.register(path,src,Trust::Official)}
    if let Some(home)=std::env::var_os("HOME"){
        let directory=Path::new(&home).join(".config/screenshaver/maker/modules");
        result.scan(&directory,0);
    }
    result
 }
 fn register(&mut self,path:&str,src:&str,trust:Trust){
    match Module::parse(src,trust){Ok(module)=>{
        if self.modules.contains_key(&module.id){self.errors.push(format!("{path}: duplicate module ID '{}'",module.id));}
        else{self.modules.insert(module.id.clone(),module);}
    },Err(e)=>self.errors.push(format!("{path}: {e}"))}
 }
 fn scan(&mut self,dir:&Path,depth:usize){
    if depth>8{return}
    let Ok(entries)=fs::read_dir(dir)else{return};
    let mut paths:Vec<_>=entries.filter_map(Result::ok).map(|e|e.path()).collect();paths.sort();
    for path in paths{
        if path.is_dir(){self.scan(&path,depth+1)}
        else if path.extension().is_some_and(|s|s=="smmod"){
            match fs::read_to_string(&path){Ok(s)=>self.register(&path.display().to_string(),&s,Trust::Community),Err(e)=>self.errors.push(format!("{}: {e}",path.display()))}
        }
    }
 }
 pub fn module(&self,id:&str)->Option<&Module>{self.modules.get(id)}
}
