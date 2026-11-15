//! Built-ins are compiled into the binary. Community files cannot impersonate them.
use crate::maker_module::{Module,Trust};
use std::{collections::BTreeMap,fs,path::Path,sync::OnceLock};
static LIBRARY:OnceLock<Library>=OnceLock::new();
pub struct Library { pub modules:BTreeMap<String,Module>, pub categories:BTreeMap<String,String>, pub errors:Vec<String> }
pub fn get()->&'static Library{LIBRARY.get().expect("Maker library must initialize before graph creation")}
pub fn initialize()->Result<(),String>{LIBRARY.set(Library::load()).map_err(|_|"Maker library already initialized".into())}
impl Library {
 fn load()->Self{
    let mut result=Self{modules:BTreeMap::new(),categories:BTreeMap::new(),errors:Vec::new()};
    let official=[
      ("coordinates/uv_coordinates.smmod",include_str!("../assets/maker/modules/coordinates/uv_coordinates.smmod")),
      ("noise/value_noise_2d.smmod",include_str!("../assets/maker/modules/noise/value_noise_2d.smmod")),
      ("math/multiply.smmod",include_str!("../assets/maker/modules/math/multiply.smmod")),
      ("color/cosine_palette.smmod",include_str!("../assets/maker/modules/color/cosine_palette.smmod")),
      ("output/output_color.smmod",include_str!("../assets/maker/modules/output/output_color.smmod")),
      ("fields/morphing_field.smmod",include_str!("../assets/maker/modules/fields/morphing_field.smmod")),
      ("fields/field_traversal.smmod",include_str!("../assets/maker/modules/fields/field_traversal.smmod")),
      ("color/color_accumulation.smmod",include_str!("../assets/maker/modules/color/color_accumulation.smmod")),
      ("noise/dot_noise_3d.smmod",include_str!("../assets/maker/modules/noise/dot_noise_3d.smmod")),
      ("math/rotation_2d.smmod",include_str!("../assets/maker/modules/math/rotation_2d.smmod")),
      ("color/aces_tonemap.smmod",include_str!("../assets/maker/modules/color/aces_tonemap.smmod")),
      ("camera/sinusoidal_path.smmod",include_str!("../assets/maker/modules/camera/sinusoidal_path.smmod")),
      ("geometry/tunnel_field.smmod",include_str!("../assets/maker/modules/geometry/tunnel_field.smmod")),
      ("noise/triangle_noise_3d.smmod",include_str!("../assets/maker/modules/noise/triangle_noise_3d.smmod")),
      ("fields/sdf_raymarch.smmod",include_str!("../assets/maker/modules/fields/sdf_raymarch.smmod")),
      ("lighting/directional_light.smmod",include_str!("../assets/maker/modules/lighting/directional_light.smmod")),
      ("lighting/fresnel_rim.smmod",include_str!("../assets/maker/modules/lighting/fresnel_rim.smmod")),
      ("color/contour_shading.smmod",include_str!("../assets/maker/modules/color/contour_shading.smmod")),
      ("lighting/ambient_occlusion_shading.smmod",include_str!("../assets/maker/modules/lighting/ambient_occlusion_shading.smmod")),
      ("lighting/curvature_shading.smmod",include_str!("../assets/maker/modules/lighting/curvature_shading.smmod")),
      ("lighting/ray_glow.smmod",include_str!("../assets/maker/modules/lighting/ray_glow.smmod")),
      ("color/angular_wave_background.smmod",include_str!("../assets/maker/modules/color/angular_wave_background.smmod")),
      ("color/rgb_channel_remap.smmod",include_str!("../assets/maker/modules/color/rgb_channel_remap.smmod")),
      ("lighting/refractive_surface.smmod",include_str!("../assets/maker/modules/lighting/refractive_surface.smmod")),
      ("geometry/sdf_torus.smmod",include_str!("../assets/maker/modules/geometry/sdf_torus.smmod")),
      ("geometry/sdf_triangular_prism.smmod",include_str!("../assets/maker/modules/geometry/sdf_triangular_prism.smmod")),
      ("geometry/sdf_twisted_ring.smmod",include_str!("../assets/maker/modules/geometry/sdf_twisted_ring.smmod")),
      ("geometry/sdf_sequential_morph.smmod",include_str!("../assets/maker/modules/geometry/sdf_sequential_morph.smmod")),
      ("geometry/rotation_3d.smmod",include_str!("../assets/maker/modules/geometry/rotation_3d.smmod")),
      ("geometry/sdf_sphere.smmod",include_str!("../assets/maker/modules/geometry/sdf_sphere.smmod")),
      ("geometry/sdf_box.smmod",include_str!("../assets/maker/modules/geometry/sdf_box.smmod")),
      ("geometry/sdf_octahedron.smmod",include_str!("../assets/maker/modules/geometry/sdf_octahedron.smmod")),
      ("geometry/sdf_combine.smmod",include_str!("../assets/maker/modules/geometry/sdf_combine.smmod")),
      ("geometry/sdf_morph.smmod",include_str!("../assets/maker/modules/geometry/sdf_morph.smmod")),
      ("geometry/domain_repeat_3d.smmod",include_str!("../assets/maker/modules/geometry/domain_repeat_3d.smmod")),
      ("geometry/domain_fold_3d.smmod",include_str!("../assets/maker/modules/geometry/domain_fold_3d.smmod")),
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
        else{
            let category=path.replace('\\',"/");
            let category=category.split('/').rev().nth(1).unwrap_or("Other");
            let category=if category=="modules" {"Other"} else {category};
            self.categories.insert(module.id.clone(),category.to_owned());
            self.modules.insert(module.id.clone(),module);
        }
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
