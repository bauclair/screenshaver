//! Built-ins are compiled into the binary. Community files cannot impersonate them.
use crate::maker_module::{Module,Trust};
use std::{collections::BTreeMap,fs,path::Path,sync::OnceLock};
static LIBRARY:OnceLock<Library>=OnceLock::new();
pub struct Library { pub modules:BTreeMap<String,Module>, pub categories:BTreeMap<String,String>, pub errors:Vec<String>, pub catalog_fingerprint:u64 }
pub fn get()->&'static Library{LIBRARY.get().expect("Maker library must initialize before graph creation")}
pub fn initialize()->Result<(),String>{LIBRARY.set(Library::load()).map_err(|_|"Maker library already initialized".into())}
impl Library {
 fn load()->Self{
    let mut result=Self{modules:BTreeMap::new(),categories:BTreeMap::new(),errors:Vec::new(),catalog_fingerprint:0xcbf29ce484222325};
    let official=[
      ("color/thermal_emission_palette.smmod",include_str!("../assets/maker/modules/color/thermal_emission_palette.smmod")),
      ("fields/temporal_frequency_layering_3d.smmod",include_str!("../assets/maker/modules/fields/temporal_frequency_layering_3d.smmod")),
      ("lighting/spherical_surface_shading_2d.smmod",include_str!("../assets/maker/modules/lighting/spherical_surface_shading_2d.smmod")),
      ("fields/animated_layered_particle_field.smmod",include_str!("../assets/maker/modules/fields/animated_layered_particle_field.smmod")),
      ("test/universal_test.smmod",include_str!("../assets/maker/modules/test/universal_test.smmod")),
      ("coordinates/uv_coordinates.smmod",include_str!("../assets/maker/modules/coordinates/uv_coordinates.smmod")),
      ("coordinates/spherical_direction.smmod",include_str!("../assets/maker/modules/coordinates/spherical_direction.smmod")),
      ("coordinates/log_polar_mapping.smmod",include_str!("../assets/maker/modules/coordinates/log_polar_mapping.smmod")),
      ("fields/sinusoidal_ribbon_field.smmod",include_str!("../assets/maker/modules/fields/sinusoidal_ribbon_field.smmod")),
      ("fields/iterative_spiral_field.smmod",include_str!("../assets/maker/modules/fields/iterative_spiral_field.smmod")),
      ("fields/iterative_radial_field.smmod",include_str!("../assets/maker/modules/fields/iterative_radial_field.smmod")),
      ("noise/value_noise_2d.smmod",include_str!("../assets/maker/modules/noise/value_noise_2d.smmod")),
      ("math/multiply.smmod",include_str!("../assets/maker/modules/math/multiply.smmod")),
      ("color/cosine_palette.smmod",include_str!("../assets/maker/modules/color/cosine_palette.smmod")),
      ("color/smooth_hsv_color.smmod",include_str!("../assets/maker/modules/color/smooth_hsv_color.smmod")),
      ("color/distance_hue_mapping.smmod",include_str!("../assets/maker/modules/color/distance_hue_mapping.smmod")),
      ("coordinates/kaleidoscopic_sector_fold.smmod",include_str!("../assets/maker/modules/coordinates/kaleidoscopic_sector_fold.smmod")),
      ("coordinates/radial_vortex_warp.smmod",include_str!("../assets/maker/modules/coordinates/radial_vortex_warp.smmod")),
      ("fields/procedural_lattice_lines.smmod",include_str!("../assets/maker/modules/fields/procedural_lattice_lines.smmod")),
      ("fields/soft_interference_field.smmod",include_str!("../assets/maker/modules/fields/soft_interference_field.smmod")),
      ("color/primary_spectrum_palette.smmod",include_str!("../assets/maker/modules/color/primary_spectrum_palette.smmod")),
      ("output/output_color.smmod",include_str!("../assets/maker/modules/output/output_color.smmod")),
      ("fields/morphing_field.smmod",include_str!("../assets/maker/modules/fields/morphing_field.smmod")),
      ("fields/field_traversal.smmod",include_str!("../assets/maker/modules/fields/field_traversal.smmod")),
      ("color/color_accumulation.smmod",include_str!("../assets/maker/modules/color/color_accumulation.smmod")),
      ("noise/dot_noise_3d.smmod",include_str!("../assets/maker/modules/noise/dot_noise_3d.smmod")),
      ("math/rotation_2d.smmod",include_str!("../assets/maker/modules/math/rotation_2d.smmod")),
      ("color/filmic_tonemapping.smmod",include_str!("../assets/maker/modules/color/filmic_tonemapping.smmod")),
      ("camera/sinusoidal_path.smmod",include_str!("../assets/maker/modules/camera/sinusoidal_path.smmod")),
      ("geometry/tunnel_field.smmod",include_str!("../assets/maker/modules/geometry/tunnel_field.smmod")),
      ("noise/triangle_noise_3d.smmod",include_str!("../assets/maker/modules/noise/triangle_noise_3d.smmod")),
      ("fields/sdf_raymarch.smmod",include_str!("../assets/maker/modules/fields/sdf_raymarch.smmod")),
      ("lighting/directional_light.smmod",include_str!("../assets/maker/modules/lighting/directional_light.smmod")),
      ("lighting/fresnel_rim.smmod",include_str!("../assets/maker/modules/lighting/fresnel_rim.smmod")),
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
      ("fields/iterative_plasma_field.smmod",include_str!("../assets/maker/modules/fields/iterative_plasma_field.smmod")),
      ("coordinates/coordinate_feedback_warp.smmod",include_str!("../assets/maker/modules/coordinates/coordinate_feedback_warp.smmod")),
      ("color/weighted_rgb_colorizer.smmod",include_str!("../assets/maker/modules/color/weighted_rgb_colorizer.smmod")),
      ("coordinates/cross_axis_sine_warp.smmod",include_str!("../assets/maker/modules/coordinates/cross_axis_sine_warp.smmod")),
      ("coordinates/depth_gated_axis_warp.smmod",include_str!("../assets/maker/modules/coordinates/depth_gated_axis_warp.smmod")),
      ("math/cosine_range_oscillator.smmod",include_str!("../assets/maker/modules/math/cosine_range_oscillator.smmod")),
      ("color/procedural_sine_rgb.smmod",include_str!("../assets/maker/modules/color/procedural_sine_rgb.smmod")),
      ("fields/vignette_mask.smmod",include_str!("../assets/maker/modules/fields/vignette_mask.smmod")),
      ("color/rgb_to_alpha.smmod",include_str!("../assets/maker/modules/color/rgb_to_alpha.smmod")),
      ("noise/fbm_2d.smmod",include_str!("../assets/maker/modules/noise/fbm_2d.smmod")),
      ("coordinates/deep_fbm_domain_warp.smmod",include_str!("../assets/maker/modules/coordinates/deep_fbm_domain_warp.smmod")),
      ("fields/four_wave_plasma_field.smmod",include_str!("../assets/maker/modules/fields/four_wave_plasma_field.smmod")),
      ("fields/neon_filament_extractor.smmod",include_str!("../assets/maker/modules/fields/neon_filament_extractor.smmod")),
      ("color/three_color_cycling_palette.smmod",include_str!("../assets/maker/modules/color/three_color_cycling_palette.smmod")),
      ("fields/2nd_order_distance_extractor.smmod",include_str!("../assets/maker/modules/fields/2nd_order_distance_extractor.smmod")),
      ("geometry/3d_cellular_tile_field.smmod",include_str!("../assets/maker/modules/geometry/3d_cellular_tile_field.smmod")),
      ("geometry/warped_plane_field.smmod",include_str!("../assets/maker/modules/geometry/warped_plane_field.smmod")),
      ("lighting/procedural_environment_reflection.smmod",include_str!("../assets/maker/modules/lighting/procedural_environment_reflection.smmod")),
      ("lighting/distance_fog.smmod",include_str!("../assets/maker/modules/lighting/distance_fog.smmod")),
      ("lighting/beer_lambert_transmittance.smmod",include_str!("../assets/maker/modules/lighting/beer_lambert_transmittance.smmod")),
      ("fields/volume_density_ray_integrator.smmod",include_str!("../assets/maker/modules/fields/volume_density_ray_integrator.smmod")),
      ("geometry/ray_box_intersection_3d.smmod",include_str!("../assets/maker/modules/geometry/ray_box_intersection_3d.smmod")),
      ("geometry/periodic_slice_field_3d.smmod",include_str!("../assets/maker/modules/geometry/periodic_slice_field_3d.smmod")),
      ("geometry/spherical_inversion_3d.smmod",include_str!("../assets/maker/modules/geometry/spherical_inversion_3d.smmod")),
      ("geometry/ray_sphere_intersection.smmod",include_str!("../assets/maker/modules/geometry/ray_sphere_intersection.smmod")),
      ("geometry/ray_plane_intersection.smmod",include_str!("../assets/maker/modules/geometry/ray_plane_intersection.smmod")),
      ("geometry/procedural_sphere_distribution.smmod",include_str!("../assets/maker/modules/geometry/procedural_sphere_distribution.smmod")),
      ("math/animated_vertical_oscillator.smmod",include_str!("../assets/maker/modules/math/animated_vertical_oscillator.smmod")),
      ("lighting/distance_attenuated_point_light.smmod",include_str!("../assets/maker/modules/lighting/distance_attenuated_point_light.smmod")),
      ("geometry/grid_ray_step_3d.smmod",include_str!("../assets/maker/modules/geometry/grid_ray_step_3d.smmod")),
      ("noise/triangular_lattice_field.smmod",include_str!("../assets/maker/modules/noise/triangular_lattice_field.smmod")),
      ("geometry/triangle_edge_mask.smmod",include_str!("../assets/maker/modules/geometry/triangle_edge_mask.smmod")),
      ("math/periodic_pulse_gate.smmod",include_str!("../assets/maker/modules/math/periodic_pulse_gate.smmod")),
      ("fields/scalar_ribbon_mask.smmod",include_str!("../assets/maker/modules/fields/scalar_ribbon_mask.smmod")),
      ("fields/soft_shape_mask_2d.smmod",include_str!("../assets/maker/modules/fields/soft_shape_mask_2d.smmod")),
      ("lighting/luminous_edge_extraction.smmod",include_str!("../assets/maker/modules/lighting/luminous_edge_extraction.smmod")),
      ("geometry/rectangular_tunnel_intersection.smmod",include_str!("../assets/maker/modules/geometry/rectangular_tunnel_intersection.smmod")),
      ("coordinates/multi_scale_flow_warp.smmod",include_str!("../assets/maker/modules/coordinates/multi_scale_flow_warp.smmod")),
      ("coordinates/tunnel_wall_uv_mapping.smmod",include_str!("../assets/maker/modules/coordinates/tunnel_wall_uv_mapping.smmod")),
      ("lighting/lattice_crossing_highlight.smmod",include_str!("../assets/maker/modules/lighting/lattice_crossing_highlight.smmod")),
      ("geometry/gyroid_surface_field_3d.smmod",include_str!("../assets/maker/modules/geometry/gyroid_surface_field_3d.smmod")),
      ("fields/second_order_cellular_field_3d.smmod",include_str!("../assets/maker/modules/fields/second_order_cellular_field_3d.smmod")),
      ("fields/chebyshev_voronoi_edge_2d.smmod",include_str!("../assets/maker/modules/fields/chebyshev_voronoi_edge_2d.smmod")),
      ("coordinates/organic_coordinate_mutation_3d.smmod",include_str!("../assets/maker/modules/coordinates/organic_coordinate_mutation_3d.smmod")),
      ("coordinates/sinusoidal_tunnel_path_3d.smmod",include_str!("../assets/maker/modules/coordinates/sinusoidal_tunnel_path_3d.smmod")),
      ("coordinates/iterative_inversion_fold_3d.smmod",include_str!("../assets/maker/modules/coordinates/iterative_inversion_fold_3d.smmod")),
      ("fields/fractal_orbit_variation_field.smmod",include_str!("../assets/maker/modules/fields/fractal_orbit_variation_field.smmod")),
      ("coordinates/periodic_mirror_tile_3d.smmod",include_str!("../assets/maker/modules/coordinates/periodic_mirror_tile_3d.smmod")),
      ("animation/ping_pong_oscillator.smmod",include_str!("../assets/maker/modules/animation/ping_pong_oscillator.smmod")),
      ("animation/looping_time_controller.smmod",include_str!("../assets/maker/modules/animation/looping_time_controller.smmod")),
      ("animation/easing_curve.smmod",include_str!("../assets/maker/modules/animation/easing_curve.smmod")),
      ("compositing/masked_color_blend.smmod",include_str!("../assets/maker/modules/compositing/masked_color_blend.smmod")),
      ("compositing/screen_color_blend.smmod",include_str!("../assets/maker/modules/compositing/screen_color_blend.smmod")),
      ("compositing/scalar_field_compositor.smmod",include_str!("../assets/maker/modules/compositing/scalar_field_compositor.smmod")),
      ("coordinates/staggered_grid_coordinates_2d.smmod",include_str!("../assets/maker/modules/coordinates/staggered_grid_coordinates_2d.smmod")),
      ("fields/randomized_bokeh_disc_field.smmod",include_str!("../assets/maker/modules/fields/randomized_bokeh_disc_field.smmod")),
      ("geometry/sdf_normal_estimation.smmod",include_str!("../assets/maker/modules/geometry/sdf_normal_estimation.smmod")),
       ("geometry/point_segment_distance_2d.smmod",include_str!("../assets/maker/modules/geometry/point_segment_distance_2d.smmod")),
      ("lighting/distance_glow_falloff.smmod",include_str!("../assets/maker/modules/lighting/distance_glow_falloff.smmod")),
      ("geometry/height_field_normal_2d.smmod",include_str!("../assets/maker/modules/geometry/height_field_normal_2d.smmod")),
       ("math/vector_magnitude_2d.smmod",include_str!("../assets/maker/modules/math/vector_magnitude_2d.smmod")),
      ];
    for (path,src) in official {result.register(path,src,Trust::Official)}
    if let Some(home)=std::env::var_os("HOME"){
        let directory=Path::new(&home).join(".config/screenshaver/maker/modules");
        result.scan(&directory,0);
    }
    result
 }
 fn register(&mut self,path:&str,src:&str,trust:Trust){
    match Module::parse(src,trust){Ok(mut module)=>{
        // Assign conservative roles to official ports without altering JSON source
        // or catalog fingerprints. Explicit semantic_role always takes precedence.
        if trust==Trust::Official {
            for port in module.inputs.iter_mut().chain(module.outputs.iter_mut()) {
                if port.semantic_role.is_empty() {
                    port.semantic_role=match (port.ty.as_str(),port.id.as_str()) {
                        ("vec2","uv")=>"uv", ("vec2","position"|"polar"|"path")=>"position2d",
                        ("vec2","offset"|"camera_offset")=>"offset2d",
                        ("vec2","vector"|"rotated")=>"direction2d",
                        ("vec3","color"|"mapped"|"base"|"albedo"|"background"|"edge_color"|"fog_color"|"light_color"|"rim_color"|"light")=>"color",
                        ("vec3","normal"|"plane_normal")=>"normal3d",
                        ("vec3","direction"|"light_direction"|"view_direction"|"incident")=>"direction3d",
                        ("vec3","position"|"origin"|"center"|"plane_point"|"light_position"|"cell")=>"position3d",
                        ("float","time")=>"time", ("float","distance")=>"distance",
                        ("float","curvature")=>"curvature", ("float","occlusion"|"ao")=>"occlusion",
                        ("float","mask"|"alpha")=>"mask", ("float","intensity"|"glow"|"pulse")=>"intensity",
                        ("float","angle")=>"angle", _=>"",
                    }.to_owned();
                }
            }
        }
        if self.modules.contains_key(&module.id){self.errors.push(format!("{path}: duplicate module ID '{}'",module.id));}
        else{
            // Local modules always appear under Third-Party, regardless of
            // their filesystem subdirectory or self-declared functional category.
            // Trust is assigned by the loader, never by module JSON.
            let category=if trust==Trust::Community {
                "Third-Party".to_owned()
            }else{
                let category=path.replace('\\',"/");
                let category=category.split('/').rev().nth(1).unwrap_or("Other");
                (if category=="modules" {"Other"} else {category}).to_owned()
            };
            self.categories.insert(module.id.clone(),category);
            // Fingerprint only successfully registered modules. Hash the complete
            // definition (including ports, defaults, GLSL and helpers), not just IDs.
            // This stable FNV-1a stream is independent of process-randomized hashers.
            for byte in (module.id.len() as u64).to_le_bytes().iter().copied()
                .chain(module.id.bytes())
                .chain((src.len() as u64).to_le_bytes().iter().copied())
                .chain(src.bytes()) {
                self.catalog_fingerprint^=u64::from(byte);
                self.catalog_fingerprint=self.catalog_fingerprint.wrapping_mul(0x100000001b3);
            }
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
