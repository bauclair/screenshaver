//! Graph-to-GLSL compiler. Module code comes from validated JSON definitions.
use crate::maker_graph::{Graph,NodeId};
use std::collections::{HashMap,HashSet};
/// Restrict untrusted module metadata to a single safe GLSL comment line.
fn attribution_text(value:&str)->String{
 let normalized=value.replace("*/", "* /");
 normalized.chars().map(|ch|if ch.is_control(){' '}else{ch}).collect()
}
pub fn fragment_source(graph:&Graph)->Result<String,String>{
 graph.validate_renderable()?;
 let reachable=graph.output_reachable_ids();
 let out=graph.nodes.iter().find(|n|n.definition().terminal).ok_or("Missing output")?;
 // Keep #version first; include each contributing module instance and the terminal.
 let mut src=String::from("#version 330 core\n/*\n * Created using Screenshaver Maker\n * Maker modules used to create this shader:\n");
 for node in graph.nodes.iter().filter(|node|reachable.contains(&node.id)) {
  let module=node.definition();
  let name=attribution_text(&module.name);
  let creator=attribution_text(&module.created_by);
  src.push_str(&format!(" * {name} — Created by: {creator}"));
  if let Some(email)=&module.creator_email {
   src.push_str(&format!(" — Creator Email: {}",attribution_text(email)));
  }
  src.push('\n');
 }
 src.push_str(" */\nout vec4 fragColor;\nuniform vec2 u_resolution;\nuniform vec2 u_viewport_origin;\nuniform float u_time;\n");
 let mut emitted=HashSet::new();let mut helpers=HashSet::new();
  // Uniforms precede helpers because callable traversal functions read node parameters.
  for n in graph.nodes.iter().filter(|n|reachable.contains(&n.id)){for p in &n.definition().parameters{
   let ty=if p.ty=="color3"{"vec3"}else{"float"};
   src.push_str(&format!("uniform {ty} u_n{}_{};\n",n.id,p.id));
  }}
  src.push_str(r#"
float maker_twisted_ring(vec3 p,float radius,float w,float h,float twists,float fold){
 vec2 q=vec2(length(p.xz)-radius,p.y);float a=atan(p.x,p.z)*twists;
 mat2 r=mat2(cos(a),sin(a),-sin(a),cos(a));q=r*q;q.y=abs(q.y)-fold;
 vec2 d=abs(q)-vec2(w,h);return length(max(d,0.0))+min(max(d.x,d.y),0.0);
}
float maker_sequential_morph(float a,float b,float c,float d,float time,float hold){
 float t=mod(time,4.0);float f=clamp((fract(t)-hold)/max(1.0-hold,0.001),0.0,1.0);f=f*f*(3.0-2.0*f);
 return t<1.0?mix(a,b,f):t<2.0?mix(b,c,f):t<3.0?mix(c,d,f):mix(d,a,f);
}
vec3 maker_rotate_3d(vec3 p,float xy,float yz,float xz){
 mat2 a=mat2(cos(xy),sin(xy),-sin(xy),cos(xy));
 mat2 b=mat2(cos(yz),sin(yz),-sin(yz),cos(yz));
 mat2 c=mat2(cos(xz),sin(xz),-sin(xz),cos(xz));
 p.xy=a*p.xy;p.yz=b*p.yz;p.xz=c*p.xz;return p;
}
"#);
  // Helper functions are emitted once per distinct source.
  for n in graph.nodes.iter().filter(|n|reachable.contains(&n.id)){let m=n.definition();if !m.helpers.is_empty()&&helpers.insert(m.helpers.as_str()){src.push_str(&m.helpers);src.push('\n')}}
  // Node-specific domain-fold functions: GLSL requires a compile-time bounded loop.
  for n in graph.nodes.iter().filter(|n|reachable.contains(&n.id)) {
   if n.module_id=="screenshaver.geometry.domain_fold_3d" {
    let id=n.id;
    src.push_str(&format!(r#"vec3 maker_blue_fold_{id}(vec3 p){{
     float ax=u_n{id}_angle_xz;float ay=u_n{id}_angle_yz;
     mat2 rx=mat2(cos(ax),sin(ax),-sin(ax),cos(ax));
     mat2 ry=mat2(cos(ay),sin(ay),-sin(ay),cos(ay));
     for(int i=0;i<8;i++){{if(float(i)>=u_n{id}_iterations)break;
      p=abs(p)-vec3(u_n{id}_offset);p.xz=rx*p.xz;p.yz=ry*p.yz;
     }}return p;
    }}
"#));
   }
  }
  // Generate each callable node/port once, using a vec3 sampling position.
  // Function calls preserve distinct sample positions without recursive source duplication.
  let mut callable=CallableGenerator::new(graph);
  for n in graph.nodes.iter().filter(|n|reachable.contains(&n.id)) {
   match n.module_id.as_str() {
    "screenshaver.fields.traversal" => {
     if let Some(c)=graph.input_connection(n.id,0){callable.require(c.from,c.from_port,"float")?;}
    }
    "screenshaver.fields.temporal_frequency_layering_3d" => {
     let c=graph.input_connection(n.id,0).ok_or("Temporal Frequency Layering 3D requires a connected scalar field")?;
     callable.require(c.from,c.from_port,"float")?;
    }
     "screenshaver.fields.repeated_ring_accumulation" => {
      for index in 0..2 {
       let c=graph.input_connection(n.id,index).ok_or("Repeated Ring Accumulation requires connected Path X and Path Y fields")?;
       let source=graph.node(c.from).ok_or("Repeated ring path source missing")?;
       let (scalar,dimension)=source.callable_spec().ok_or("Repeated ring path must be a position-dependent scalar field")?;
       if dimension!="field3d" || c.from_port!=source.definition().outputs.len(){return Err("Repeated Ring Accumulation requires Field 3D callable outputs".into())}
       callable.require(c.from,scalar,"float")?;
      }
     }
    "screenshaver.fields.volume_density_ray_integrator" => {
     let c=graph.input_connection(n.id,0).ok_or("Volume Density Ray Integrator requires a connected density field")?;
     callable.require(c.from,c.from_port,"float")?;
    }
    "screenshaver.fields.sdf_raymarch" => {
     let c=graph.input_connection(n.id,0).ok_or("SDF Ray March requires a connected distance field")?;
     callable.require(c.from,c.from_port,"float")?;
     if let Some(c)=graph.input_connection(n.id,1){callable.require(c.from,c.from_port,"vec2")?;}
    }
     "screenshaver.lighting.secondary_sdf_reflection_trace" => {
      let c=graph.input_connection(n.id,0).ok_or("Secondary SDF Reflection Trace requires a connected 3D distance field")?;
      let source=graph.node(c.from).ok_or("Secondary reflection field source missing")?;
      let (scalar,dimension)=source.callable_spec().ok_or("Secondary reflection requires a position-dependent scalar SDF")?;
      if dimension!="field3d" || c.from_port!=source.definition().outputs.len(){return Err("Secondary reflection requires a field3d callable output".into())}
      callable.require(c.from,scalar,"float")?;
     }
    "screenshaver.lighting.layered_refractive_boundary_traversal" => {
     let c=graph.input_connection(n.id,0).ok_or("Layered Refractive Boundary Traversal requires a connected Distance Field")?;
     // The field3d disc is synthesized after the declared scalar outputs.
     let source=graph.node(c.from).ok_or("Layered Refractive Boundary Traversal field source missing")?;
     let (scalar,dimension)=source.callable_spec().ok_or("Layered Refractive Boundary Traversal requires a position-dependent scalar SDF")?;
     if dimension!="field3d" || c.from_port!=source.definition().outputs.len(){
      return Err("Layered Refractive Boundary Traversal requires a field3d callable output".into());
     }
     callable.require(c.from,scalar,"float")?;
     let c=graph.input_connection(n.id,1).ok_or("Layered Refractive Boundary Traversal requires a connected Background")?;
     callable.require(c.from,c.from_port,"vec3")?;
    }
    "screenshaver.lighting.refractive_surface" => {
     let c=graph.input_connection(n.id,0).ok_or("Refractive Surface Sampling requires a connected Distance Field")?;
     callable.require(c.from,c.from_port,"float")?;
     let c=graph.input_connection(n.id,1).ok_or("Refractive Surface Sampling requires a connected Background")?;
     callable.require(c.from,c.from_port,"vec3")?;
    }
    _=>{}
   }
  }
   // Ordinary field inputs carry function references, never sampled scalar values.
   for n in graph.nodes.iter().filter(|n|reachable.contains(&n.id)) {
    for (port,input) in n.definition().inputs.iter().enumerate() {
     if input.ty=="field2d"||input.ty=="field3d" {
      let Some(c)=graph.input_connection(n.id,port) else {
       if n.module_id=="screenshaver.test.universal_test" {continue}
       return Err(format!("{} requires {}",n.definition().name,input.name));
      };
      let source=graph.node(c.from).ok_or("Missing callable source")?;
      let port=source.output_port(c.from_port).ok_or("Callable source port missing")?;
      if port.ty!=input.ty{return Err("Callable field dimension or output mismatch".into())}
      if source.module_id=="screenshaver.test.universal_test" && (port.id=="producer2d" || port.id=="producer3d") {
       callable.require(c.from,c.from_port,&port.ty)?;
      }else{
       let (scalar,dimension)=source.callable_spec().ok_or("Callable source is not position-dependent")?;
       if dimension!=input.ty || c.from_port!=source.definition().outputs.len(){return Err("Callable field dimension or output mismatch".into())}
       callable.require(c.from,scalar,"float")?;
      }
     }
    }
   }
   src.push_str(&callable.source());
  // A connected scalar field becomes a position-dependent callable evaluation.
  // Unlike a precomputed scalar seed, this expression is evaluated inside each ray step.
  for n in graph.nodes.iter().filter(|n|reachable.contains(&n.id)){
   if n.module_id=="screenshaver.fields.traversal" {
    if let Some(c)=graph.input_connection(n.id,0){
     let field=callable.reference(c.from,c.from_port,"sample_position","float")?;
     // Both modes integrate bounded color/opacity along the connected field.
     // Density controls optical depth rather than unbounded additive brightness.
     // No reciprocal-distance amplification: a near-zero field must not white out.
     src.push_str(&format!(r#"vec3 maker_traverse_connected_{id}(vec3 origin,float depth,float density,float speed,float mode){{
      vec3 ray=normalize(vec3(origin.xy,1.25));
      float extent=max(depth,0.001);
      float step_size=extent/16.0;
      float optical_density=max(density,0.0);
      float t=u_time*speed;
      vec3 accumulated=vec3(0.0);
      float transmittance=1.0;
      for(int k=0;k<16;k++){{
       float z=(float(k)+0.5)*step_size;
       vec3 sample_position=vec3(origin.xy,-1.0)+ray*z;
       float field=({field});
       // Mode 0 emphasizes the field's zero crossing; mode 1 gives a
       // broader response to the same connected field, without singularities.
       float narrow=exp(-min(abs(field)*4.0,80.0));
       float broad=exp(-min(abs(field)*1.5,80.0));
       float occupancy=mix(narrow,broad,step(0.5,mode));
       float attenuation=1.0/(1.0+0.25*z*z);
       float opacity=1.0-exp(-min(optical_density*occupancy*attenuation*step_size,80.0));
       vec3 phase=vec3(0.0,2.1,4.2);
       vec3 tint=0.5+0.5*cos(phase+t*0.4+z*2.0+field*0.5);
       accumulated+=transmittance*opacity*tint;
       transmittance*=1.0-opacity;
      }}
      return accumulated;
     }}
"#,id=n.id));
    }
   }
  }
 // Generate per-node field resampling for temporal frequency layering.
 for n in graph.nodes.iter().filter(|n|reachable.contains(&n.id)) {
  if n.module_id=="screenshaver.fields.temporal_frequency_layering_3d" {
   let c=graph.input_connection(n.id,0).ok_or("Temporal Frequency Layering 3D requires a connected scalar field")?;
   let field=callable.reference(c.from,c.from_port,"sample_position","float")?;
   let id=n.id;
   src.push_str(&format!(r#"
float maker_temporal_layers_{id}(vec3 position,float layers,float spatialScale,float temporalScale,float decay,float speed){{
 vec3 spatial=position;
 float phase=u_time*speed;
 float amplitude=1.0;
 float total=0.0;
 float weight=0.0;
 int count=int(clamp(floor(layers+0.5),1.0,8.0));
 for(int k=0;k<8;k++){{
  if(k>=count)break;
  vec3 sample_position=spatial+vec3(0.0,0.0,phase);
  total+=({field})*amplitude;
  weight+=amplitude;
  spatial*=spatialScale;
  phase*=temporalScale;
  amplitude*=clamp(decay,0.0,1.0);
 }}
 return total/max(weight,1e-8);
}}
"#));
  }
 }
  // Independent bounded traversal of a connected, position-dependent scalar field.
  for n in graph.nodes.iter().filter(|n|reachable.contains(&n.id)) {
   if n.module_id=="screenshaver.fields.adaptive_procedural_ray_traversal" {
    let c=graph.input_connection(n.id,0).ok_or("Adaptive Procedural Ray Traversal requires a connected Field 3D")?;
    let source=graph.node(c.from).ok_or("Adaptive traversal field source missing")?;
    let (scalar,dimension)=source.callable_spec().ok_or("Adaptive traversal requires a position-dependent scalar Field 3D")?;
    if dimension!="field3d" || c.from_port!=source.definition().outputs.len() {
     return Err("Adaptive traversal requires a Field 3D callable output".into());
    }
    let field=callable.reference(c.from,scalar,"sample_position","float")?;
    let id=n.id;
    src.push_str(&format!(r#"
void maker_adaptive_traverse_{id}(vec3 origin,vec3 direction,out float intensity,out float travelled,out vec3 endPosition,out float transmission){{
 intensity=0.0;travelled=0.0;endPosition=origin;transmission=1.0;
 float dirLen=length(direction);
 if(dirLen<1e-8)return;
 vec3 ray=direction/dirLen;
 float extent=max(u_n{id}_max_distance,0.0);
 float low=max(u_n{id}_min_step,0.0001);
 float high=max(u_n{id}_max_step,low);
 float adapt=clamp(u_n{id}_adaptivity,0.0,1.0);
 int limit=int(clamp(floor(u_n{id}_max_steps+0.5),1.0,96.0));
 for(int k=0;k<96;k++){{
  if(k>=limit || travelled>=extent || transmission<0.001)break;
  vec3 sample_position=origin+ray*travelled;
  float sampleValue={field};
  if(isnan(sampleValue)||isinf(sampleValue))break;
  float response=exp(-min(abs(sampleValue)*max(u_n{id}_response_sharpness,0.0001),80.0));
  float desired=mix(high,mix(high,low,response),adapt);
  float stride=min(clamp(desired,low,high),extent-travelled);
  if(stride<=0.0)break;
  float alpha=1.0-exp(-min(max(u_n{id}_opacity_gain,0.0)*response*stride,80.0));
  intensity+=transmission*alpha;
  transmission*=1.0-alpha;
  travelled+=stride;
 }}
 endPosition=origin+ray*travelled;
}}
"#));
   }
  }
 // Project repeated ring outlines along a connected pair of callable scalar paths.
 for n in graph.nodes.iter().filter(|n|reachable.contains(&n.id)) {
  if n.module_id=="screenshaver.fields.repeated_ring_accumulation" {
   let mut fields=Vec::new();
   for index in 0..2 {
    let c=graph.input_connection(n.id,index).ok_or("Repeated Ring Accumulation requires Path X and Path Y")?;
    let source=graph.node(c.from).ok_or("Repeated ring path source missing")?;
    let (scalar,dimension)=source.callable_spec().ok_or("Repeated ring path must be a callable scalar field")?;
    if dimension!="field3d" || c.from_port!=source.definition().outputs.len(){return Err("Repeated ring path must be Field 3D".into())}
    fields.push(callable.reference(c.from,scalar,"sample_position","float")?);
   }
   let id=n.id;
   src.push_str(&format!(r#"
void maker_repeated_rings_{id}(vec2 uv,vec3 camera,out vec3 color,out float intensity,out vec2 lastCenter){{
 color=vec3(0.0);intensity=0.0;lastCenter=vec2(0.0);
 int count=int(clamp(floor(u_n{id}_ring_count+0.5),1.0,128.0));
 float spacing=max(u_n{id}_depth_spacing,0.0001);
 float focal=max(u_n{id}_focal_length,0.0001);
 float lineWidth=max(u_n{id}_line_width,0.00001);
 for(int k=0;k<128;k++){{
  if(k>=count)break;
  float depth=max(u_n{id}_start_depth,0.0001)+float(k)*spacing;
  float worldDepth=camera.z+depth;
  vec3 sample_position=vec3(0.0,0.0,worldDepth);
  float px={xfield};float py={yfield};
  if(isnan(px)||isinf(px)||isnan(py)||isinf(py))continue;
  vec2 projected=(vec2(px,py)-camera.xy)*focal/depth;
  float projectedRadius=max(u_n{id}_ring_radius,0.0001)*focal/depth;
  float edge=abs(length(uv-projected)-projectedRadius);
  float weight=exp(-min(edge/lineWidth,80.0))/(1.0+depth*depth);
  float phase=worldDepth*0.15;
  vec3 tint=0.5+0.5*cos(vec3(0.0,2.0943951,4.1887902)+phase);
  color+=tint*weight*max(u_n{id}_brightness,0.0);
  intensity+=weight*max(u_n{id}_brightness,0.0);
  lastCenter=projected;
 }}
 color=vec3(1.0)-exp(-max(color,vec3(0.0))*max(u_n{id}_exposure,0.0));
 intensity=1.0-exp(-max(intensity,0.0)*max(u_n{id}_exposure,0.0));
}}
"#,xfield=fields[0],yfield=fields[1]));
  }
 }
 // Generate a bounded Beer-Lambert volume integrator for each connected density field.
 for n in graph.nodes.iter().filter(|n|reachable.contains(&n.id)) {
  if n.module_id=="screenshaver.fields.volume_density_ray_integrator" {
   let c=graph.input_connection(n.id,0).ok_or("Volume Density Ray Integrator requires a connected density field")?;
   let field=callable.reference(c.from,c.from_port,"sample_position","float")?;
   let id=n.id;
   src.push_str(&format!(r#"
vec4 maker_volume_integrate_{id}(vec3 origin,vec3 direction,vec3 emission,float distance,float extinction,float gain,float samples){{
 float directionLength=length(direction);
 if(directionLength<1e-8)return vec4(0.0,0.0,0.0,1.0);
 vec3 ray=direction/directionLength;
 float extent=max(distance,0.0);
 int count=int(clamp(floor(samples+0.5),1.0,64.0));
 float stepSize=extent/float(count);
 float transmission=1.0;
 vec3 accumulated=vec3(0.0);
 for(int k=0;k<64;k++){{
  if(k>=count || transmission<0.001)break;
  vec3 sample_position=origin+ray*((float(k)+0.5)*stepSize);
  float rho=max({field},0.0);
  float alpha=1.0-exp(-min(rho*max(extinction,0.0)*stepSize,80.0));
  accumulated+=transmission*alpha*max(emission,vec3(0.0))*max(gain,0.0);
  transmission*=1.0-alpha;
 }}
 return vec4(accumulated,transmission);
}}
"#));
  }
 }
 // Generate an independent signed-distance ray marcher for each SDF node.
 for n in graph.nodes.iter().filter(|n|reachable.contains(&n.id)) {
  if n.module_id=="screenshaver.fields.sdf_raymarch" {
   let c=graph.input_connection(n.id,0).ok_or("SDF Ray March requires a connected distance field")?;
   let field=callable.reference(c.from,c.from_port,"sample_position","float")?;
   let camera=if let Some(c)=graph.input_connection(n.id,1){callable.reference(c.from,c.from_port,"vec3(0.0,0.0,camera_z)","vec2")?}else{"vec2(0.0)".into()};
   let id=n.id;
   src.push_str(&format!(r#"
float maker_sdf_{id}(vec3 sample_position){{return {field};}}
vec3 maker_sdf_normal_{id}(vec3 p,float e){{
 vec2 h=vec2(e,0.0);
 return normalize(vec3(maker_sdf_{id}(p+h.xyy)-maker_sdf_{id}(p-h.xyy),maker_sdf_{id}(p+h.yxy)-maker_sdf_{id}(p-h.yxy),maker_sdf_{id}(p+h.yyx)-maker_sdf_{id}(p-h.yyx)));
}}
vec3 maker_sdf_render_{id}(vec2 uv,float speed,float maxDistance,float epsilon,float stepScale,float fov,float glowFalloff,float glowGain,vec3 externalOrigin,vec3 externalDirection,float cameraSource,out vec3 surfaceNormal,out float surfaceAO,out float surfaceCurvature,out float rayGlow,out vec3 hitPosition,out vec3 incomingDirection,out float hitMask){{
 surfaceNormal=vec3(0.0,0.0,1.0); surfaceAO=1.0; surfaceCurvature=0.0; rayGlow=0.0; hitPosition=vec3(0.0); incomingDirection=vec3(0.0); hitMask=0.0;
 // Bounded, periodic camera travel: preserve the original view at t=0,
 // but never advance beyond origin-centered SDF geometry while looking +Z.
 // Negative Z keeps the camera on the approach side of the scene.
 float camera_z=-1.5*abs(sin(u_time*speed));
 vec2 camera_xy={camera};
 vec3 ro=vec3(camera_xy,camera_z);
 vec3 rd=normalize(vec3(uv*fov,1.0));
 // Preserve legacy behavior unless the user explicitly selects External Camera.
 // Guard invalid directions deterministically without producing NaNs.
 if(cameraSource>=0.5 && dot(externalDirection,externalDirection)>1e-12){{
  ro=externalOrigin;
  rd=normalize(externalDirection);
 }}
 incomingDirection=rd;
 float t=0.03;
 for(int i=0;i<128;i++){{
  vec3 p=ro+rd*t;
  float d=maker_sdf_{id}(p);
  rayGlow+=exp(-abs(d)*max(glowFalloff,0.001))*glowGain;
  if(abs(d)<epsilon){{
   float e=max(epsilon*3.0,0.003);
   vec3 nor=maker_sdf_normal_{id}(p,e);
   surfaceNormal=nor;
   hitPosition=p; hitMask=1.0;
   float occ=0.0;
   for(int j=1;j<=5;j++){{
    float h=0.045*float(j);
    occ+=max(h-maker_sdf_{id}(p+nor*h),0.0)/(h*float(j));
   }}
   surfaceAO=clamp(1.0-occ*0.55,0.0,1.0);
   float curvatureStep=max(e*3.0,0.02);
   vec3 dx=vec3(curvatureStep,0.0,0.0);
   vec3 dy=vec3(0.0,curvatureStep,0.0);
   vec3 dz=vec3(0.0,0.0,curvatureStep);
   float laplace=(maker_sdf_{id}(p+dx)+maker_sdf_{id}(p-dx)+maker_sdf_{id}(p+dy)+maker_sdf_{id}(p-dy)+maker_sdf_{id}(p+dz)+maker_sdf_{id}(p-dz)-6.0*maker_sdf_{id}(p))/(curvatureStep*curvatureStep);
   surfaceCurvature=clamp(laplace*0.025,-1.0,1.0);
   vec3 lightDir=normalize(vec3(0.55,0.85,-0.65));
   float diffuse=max(dot(nor,lightDir),0.0);
   float ambient=0.18+0.20*max(nor.y,0.0);
   float fog=exp(-t*0.024);
   return mix(vec3(0.012,0.018,0.035),vec3(0.48,0.65,0.88)*(ambient+0.85*diffuse),fog);
  }}
  if(t>maxDistance)break;
  t+=max(abs(d)*stepScale,epsilon*0.7);
 }}
 return vec3(0.012,0.018,0.035);
}}
"#));
  }
 }
 // Bounded single-path optical traversal: only emitted for an active optical node.
 for n in graph.nodes.iter().filter(|n|reachable.contains(&n.id)) {
  if n.module_id=="screenshaver.lighting.layered_refractive_boundary_traversal" {
   let field_conn=graph.input_connection(n.id,0).ok_or("Layered Refractive Boundary Traversal requires a connected Distance Field")?;
   let bg_conn=graph.input_connection(n.id,1).ok_or("Layered Refractive Boundary Traversal requires a connected Background")?;
   let source=graph.node(field_conn.from).ok_or("Layered Refractive Boundary Traversal field source missing")?;
   let (scalar,dimension)=source.callable_spec().ok_or("Layered Refractive Boundary Traversal requires a position-dependent scalar SDF")?;
   if dimension!="field3d" || field_conn.from_port!=source.definition().outputs.len(){
    return Err("Layered Refractive Boundary Traversal requires a field3d callable output".into());
   }
   let field=callable.reference(field_conn.from,scalar,"sample_position","float")?;
   let background=callable.reference(bg_conn.from,bg_conn.from_port,"sample_direction","vec3")?;
   let id=n.id;
   src.push_str(&format!(r#"
float maker_layer_sdf_{id}(vec3 sample_position){{return {field};}}
vec3 maker_layer_background_{id}(vec3 sample_direction){{return {background};}}
vec3 maker_layer_normal_{id}(vec3 p,float e){{
 vec3 dx=vec3(e,0.0,0.0),dy=vec3(0.0,e,0.0),dz=vec3(0.0,0.0,e);
 vec3 g=vec3(maker_layer_sdf_{id}(p+dx)-maker_layer_sdf_{id}(p-dx),
             maker_layer_sdf_{id}(p+dy)-maker_layer_sdf_{id}(p-dy),
             maker_layer_sdf_{id}(p+dz)-maker_layer_sdf_{id}(p-dz));
 return length(g)>1e-7?normalize(g):vec3(0.0,0.0,-1.0);
}}
vec3 maker_layer_render_{id}(vec2 uv){{
 vec3 ro=vec3(0.0,0.0,u_n{id}_camera_z);
 vec3 rd=normalize(vec3(uv,1.0));
 float eps=max(u_n{id}_epsilon,0.001);
 float maxDistance=max(u_n{id}_max_distance,2.0);
 float ior=max(u_n{id}_ior,1.0);
 int limit=int(clamp(floor(u_n{id}_max_boundaries+0.5),1.0,4.0));
 float travelled=0.0;
 float throughput=1.0;
 vec3 attenuation=vec3(1.0);
 // A single path, with a finite total SDF evaluation budget (4*48 march steps).
 for(int boundary=0;boundary<4;boundary++){{
  if(boundary>=limit || throughput<0.002 || travelled>=maxDistance)break;
  float t=0.0;
  bool hit=false;
  vec3 p=ro;
  for(int stepIndex=0;stepIndex<48;stepIndex++){{
   p=ro+rd*t;
   float d=maker_layer_sdf_{id}(p);
   if(abs(d)<=eps){{hit=true;break;}}
   // Safe signed-distance step, including travel inside the object.
   float stepLength=max(abs(d)*0.75,eps*0.75);
   if(travelled+t+stepLength>maxDistance)break;
   t+=stepLength;
  }}
  if(!hit){{
   float remaining=max(maxDistance-travelled,0.0);
   if(maker_layer_sdf_{id}(ro)<-eps)
    attenuation*=exp(-max(u_n{id}_absorption,0.0)*remaining*(vec3(1.0)-clamp(u_n{id}_tint,0.0,1.0)+vec3(0.05)));
   return clamp(maker_layer_background_{id}(rd)*attenuation*throughput,0.0,1.0);
  }}
  float startInside=maker_layer_sdf_{id}(ro);
  if(startInside<-eps)
   attenuation*=exp(-max(u_n{id}_absorption,0.0)*t*(vec3(1.0)-clamp(u_n{id}_tint,0.0,1.0)+vec3(0.05)));
  travelled+=t;
  vec3 outward=maker_layer_normal_{id}(p,max(eps*2.0,0.002));
  bool entering=dot(rd,outward)<0.0;
  vec3 normal=entering?outward:-outward;
  float eta=entering?1.0/ior:ior;
  float cosine=clamp(-dot(rd,normal),0.0,1.0);
  float f0=pow((ior-1.0)/(ior+1.0),2.0);
  float fresnel=f0+(1.0-f0)*pow(1.0-cosine,5.0);
  vec3 transmitted=refract(rd,normal,eta);
  bool totalInternal=dot(transmitted,transmitted)<1e-8;
  if(totalInternal){{rd=normalize(reflect(rd,normal));}}
  else{{rd=normalize(transmitted);throughput*=1.0-fresnel;}}
  // Offset beyond the interface to avoid self-intersections; keep this distance bounded.
  float offset=eps*3.0;
  ro=p+rd*offset;
  travelled+=offset;
 }}
 return clamp(maker_layer_background_{id}(rd)*attenuation*throughput,0.0,1.0);
}}
"#));
  }
 }
 // Optical sampling: evaluate the connected background at the actual ray direction.
 for n in graph.nodes.iter().filter(|n|reachable.contains(&n.id)) {
  if n.module_id=="screenshaver.lighting.refractive_surface" {
   let field_conn=graph.input_connection(n.id,0).ok_or("Refractive Surface Sampling requires a connected Distance Field")?;
   let bg_conn=graph.input_connection(n.id,1).ok_or("Refractive Surface Sampling requires a connected Background")?;
   let field=callable.reference(field_conn.from,field_conn.from_port,"sample_position","float")?;
   let background=callable.reference(bg_conn.from,bg_conn.from_port,"sample_direction","vec3")?;
   let id=n.id;
   src.push_str(&format!(r#"
float maker_optical_sdf_{id}(vec3 sample_position){{return {field};}}
vec3 maker_optical_background_{id}(vec3 sample_direction){{return {background};}}
vec3 maker_optical_render_{id}(vec2 uv){{
 vec3 ro=vec3(0.0,0.0,u_n{id}_camera_z);
 vec3 rd=normalize(vec3(uv,1.0));
 float t=0.0;
 for(int k=0;k<96;k++){{
  if(t>u_n{id}_max_distance)break;
  vec3 p=ro+rd*t;
  float d=maker_optical_sdf_{id}(p);
  if(abs(d)<u_n{id}_epsilon){{
   float e=max(u_n{id}_epsilon*2.0,0.002);
   vec3 dx=vec3(e,0.0,0.0),dy=vec3(0.0,e,0.0),dz=vec3(0.0,0.0,e);
   vec3 nor=normalize(vec3(
    maker_optical_sdf_{id}(p+dx)-maker_optical_sdf_{id}(p-dx),
    maker_optical_sdf_{id}(p+dy)-maker_optical_sdf_{id}(p-dy),
    maker_optical_sdf_{id}(p+dz)-maker_optical_sdf_{id}(p-dz)));
   vec3 refracted=refract(rd,nor,u_n{id}_refraction_ratio);
   if(dot(refracted,refracted)<0.00001)refracted=reflect(rd,nor);
   float fade=smoothstep(0.0,max(u_n{id}_fade_distance,0.001),t);
   return maker_optical_background_{id}(normalize(refracted))*fade;
  }}
  t+=max(abs(d)*u_n{id}_step_scale,u_n{id}_epsilon*0.7);
 }}
 return maker_optical_background_{id}(rd);
}}
"#));
  }
 }
 // One bounded, nonrecursive reflected SDF traversal per reachable reflection node.
 for n in graph.nodes.iter().filter(|n|reachable.contains(&n.id)) {
  if n.module_id=="screenshaver.lighting.secondary_sdf_reflection_trace" {
   let c=graph.input_connection(n.id,0).ok_or("Secondary reflection requires connected Distance Field")?;
   let source=graph.node(c.from).ok_or("Secondary reflection source missing")?;
   let (scalar,dimension)=source.callable_spec().ok_or("Secondary reflection requires callable SDF")?;
   if dimension!="field3d" || c.from_port!=source.definition().outputs.len(){return Err("Secondary reflection requires a field3d callable output".into())}
   let field=callable.reference(c.from,scalar,"sample_position","float")?;
   let id=n.id;
   src.push_str(&format!(r#"
float maker_reflection_sdf_{id}(vec3 sample_position){{return {field};}}
vec3 maker_reflection_trace_{id}(vec3 hitPosition,vec3 surfaceNormal,vec3 incomingDirection,float primaryHit,out float secondaryHit,out float secondaryDistance){{
 secondaryHit=0.0; secondaryDistance=-1.0;
 if(primaryHit<0.5 || dot(surfaceNormal,surfaceNormal)<1e-10 || dot(incomingDirection,incomingDirection)<1e-10)return u_n{id}_miss_color;
 vec3 n=normalize(surfaceNormal);
 vec3 incoming=normalize(incomingDirection);
 vec3 ray=normalize(reflect(incoming,n));
 float eps=max(u_n{id}_epsilon,0.0005);
 float bias=max(u_n{id}_surface_bias,eps*2.0);
 vec3 origin=hitPosition+ray*bias;
 float travelled=0.0;
 float extent=max(u_n{id}_max_distance,eps);
 int limit=int(clamp(floor(u_n{id}_max_steps+0.5),1.0,48.0));
 for(int i=0;i<48;i++){{
  if(i>=limit || travelled>extent)break;
  vec3 sample_position=origin+ray*travelled;
  float d=maker_reflection_sdf_{id}(sample_position);
  if(isnan(d)||isinf(d))break;
  if(abs(d)<=eps){{
   secondaryHit=1.0; secondaryDistance=travelled+bias;
   float shade=0.35+0.65*max(dot(normalize(vec3(0.45,0.8,-0.35)),n),0.0);
   float fade=exp(-max(u_n{id}_attenuation,0.0)*secondaryDistance);
   return mix(u_n{id}_miss_color,u_n{id}_hit_color*shade,fade)*clamp(u_n{id}_strength,0.0,1.0);
  }}
  travelled+=max(abs(d)*0.75,eps*0.75);
 }}
 return u_n{id}_miss_color;
}}
"#));
  }
 }
 src.push_str("void main(){\n  vec2 maker_uv=(gl_FragCoord.xy-u_viewport_origin-0.5*u_resolution)/max(u_resolution.y,1.0);\n");
 if graph.input_connection(out.id,0).is_some(){
  let c=graph.input_connection(out.id,0).unwrap();emit(graph,c.from,&reachable,&mut emitted,&mut src)?;
  src.push_str(&format!("  fragColor=vec4(clamp(v{}_p{},0.0,1.0),1.0);\n",c.from,c.from_port));
 }else{src.push_str("  fragColor=vec4(0.015,0.02,0.04,1.0);\n")}
 src.push_str("}\n");Ok(src)
}
fn substitute(template:&str,bindings:&std::collections::HashMap<String,String>)->Result<String,String>{
 let mut result=String::new();let mut chars=template.chars().peekable();
 while let Some(ch)=chars.next(){
  if ch=='{'{
   let mut key=String::new();let mut closed=false;
   for c in chars.by_ref(){if c=='}'{closed=true;break}key.push(c)}
   if !closed{return Err("Unclosed GLSL placeholder".into())}
   result.push_str(bindings.get(&key).ok_or_else(||format!("Unknown GLSL placeholder: {key}"))?);
  }else if ch=='}'{return Err("Unexpected GLSL closing brace".into())}
  else{result.push(ch)}
 }
 Ok(result)
}
// Select the first connected diagnostic input in a fixed priority order.
// No calibration parameters or runtime switching are required.
fn universal_preview_expression(graph:&Graph,id:NodeId)->String {
 for port in [6usize,5,3,2,1,0,4] {
  if graph.input_connection(id,port).is_some() {
   let c=graph.input_connection(id,port).expect("connected diagnostic input");
   let value=if port==5 || port==6 {
    let function=if port==5 {format!("maker_field2d_{}_{}",c.from,c.from_port)}
     else {format!("maker_callable_{}_{}",c.from,graph.node(c.from).and_then(|n|n.callable_spec()).map(|x|x.0).unwrap_or(c.from_port))};
    let position=if port==5 {format!("vec2(u_n{id}_uv_x,u_n{id}_uv_y)")}
     else {format!("vec3(u_n{id}_sample_x,u_n{id}_sample_y,u_n{id}_sample_z)")};
    format!("{function}({position})")
   } else {format!("v{}_p{}",c.from,c.from_port)};
   return match port {
    5 | 6 => format!("maker_universal_signed({value})"),
    3 => value,
    2 => format!("clamp(0.5+0.5*({value}),0.0,1.0)"),
    1 => format!("clamp(vec3(0.5+0.5*({value}),0.5),0.0,1.0)"),
    0 => format!("vec3(clamp({value},0.0,1.0))"),
    _ => format!("maker_universal_signed({value})"),
   };
  }
 }
 "maker_universal_color(maker_uv)".into()
}
fn emit(graph:&Graph,id:NodeId,reachable:&HashSet<NodeId>,emitted:&mut HashSet<NodeId>,src:&mut String)->Result<(),String>{
 if emitted.contains(&id){return Ok(())}
 let node=graph.node(id).ok_or("Missing upstream node")?;let m=node.definition();
 if m.terminal{return Err("Output cannot be an upstream source".into())}
 for port in 0..m.inputs.len(){if m.inputs[port].ty.starts_with("field") || (m.id=="screenshaver.fields.traversal" && port==0){continue}if let Some(c)=graph.input_connection(id,port){emit(graph,c.from,reachable,emitted,src)?}}
 let mut vars=std::collections::HashMap::new();
 for (port,p) in m.inputs.iter().enumerate(){
  let expression=if p.ty.starts_with("field") {
    let Some(c)=graph.input_connection(id,port) else {
     if m.id=="screenshaver.test.universal_test" {
      vars.insert(p.id.clone(),p.default.clone());
      continue;
     }
     return Err(format!("{} requires callable input {}",m.name,p.name));
    };
    if p.ty=="field2d" {format!("maker_field2d_{}_{}",c.from,c.from_port)}
    else {format!("maker_callable_{}_{}",c.from,if graph.node(c.from).is_some_and(|n|n.module_id=="screenshaver.test.universal_test") {c.from_port} else {graph.node(c.from).and_then(|n|n.callable_spec()).ok_or("Invalid callable source")?.0})}
   } else {graph.input_connection(id,port).map(|c|format!("v{}_p{}",c.from,c.from_port)).unwrap_or_else(||p.default.clone())};
  if expression.is_empty(){return Err(format!("Missing input default for {}:{}",m.id,p.id))}
  vars.insert(p.id.clone(),expression);
 }
 for p in &m.parameters{vars.insert(p.id.clone(),format!("u_n{}_{}",id,p.id));}
 if m.id=="screenshaver.color.angular_wave_background" && graph.input_connection(id,0).is_none(){vars.insert("direction".into(),"normalize(vec3(maker_uv,1.0))".into());}
 if m.id=="screenshaver.lighting.layered_refractive_boundary_traversal" {
  src.push_str(&format!("  vec3 v{}_p0=maker_layer_render_{}(maker_uv);\n",id,id));
  emitted.insert(id);return Ok(());
 }
 if m.id=="screenshaver.lighting.refractive_surface" {
  src.push_str(&format!("  vec3 v{}_p0=maker_optical_render_{}(maker_uv);\n",id,id));
  emitted.insert(id);return Ok(());
 }
 if m.id=="screenshaver.fields.temporal_frequency_layering_3d" {
  src.push_str(&format!("  float v{}_p0=maker_temporal_layers_{}({},u_n{}_layers,u_n{}_spatial_scale,u_n{}_temporal_scale,u_n{}_amplitude_decay,u_n{}_speed);\n",id,id,vars["position"],id,id,id,id,id));
  emitted.insert(id);return Ok(());
 }
  if m.id=="screenshaver.fields.adaptive_procedural_ray_traversal" {
   src.push_str(&format!("  float v{}_p0; float v{}_p1; vec3 v{}_p2; float v{}_p3;\n",id,id,id,id));
   src.push_str(&format!("  maker_adaptive_traverse_{}({},{},v{}_p0,v{}_p1,v{}_p2,v{}_p3);\n",id,vars["origin"],vars["direction"],id,id,id,id));
   emitted.insert(id);return Ok(());
  }
  if m.id=="screenshaver.fields.repeated_ring_accumulation" {
   src.push_str(&format!("  vec3 v{}_p0; float v{}_p1; vec2 v{}_p2; maker_repeated_rings_{}({},{},v{}_p0,v{}_p1,v{}_p2);\n",id,id,id,id,vars["uv"],vars["camera"],id,id,id));
   emitted.insert(id);return Ok(());
  }
 if m.id=="screenshaver.fields.volume_density_ray_integrator" {
  src.push_str(&format!("  vec4 v{}_volume=maker_volume_integrate_{}({},{},{},u_n{}_distance,u_n{}_extinction,u_n{}_emission_gain,u_n{}_samples);\n",id,id,vars["origin"],vars["direction"],vars["emission"],id,id,id,id));
  src.push_str(&format!("  vec3 v{}_p0=v{}_volume.rgb; float v{}_p1=1.0-v{}_volume.a; float v{}_p2=v{}_volume.a;\n",id,id,id,id,id,id));
  emitted.insert(id);return Ok(());
 }
 if m.id=="screenshaver.lighting.secondary_sdf_reflection_trace" {
  src.push_str(&format!("  float v{}_p1; float v{}_p2; vec3 v{}_p0=maker_reflection_trace_{}({},{},{},{},v{}_p1,v{}_p2);\n",id,id,id,id,vars["hit_position"],vars["surface_normal"],vars["incoming_direction"],vars["hit_mask"],id,id));
  emitted.insert(id);return Ok(());
 }
 if m.id=="screenshaver.fields.sdf_raymarch" {
  src.push_str(&format!("  vec3 v{}_p0; vec3 v{}_p1; float v{}_p2; float v{}_p3; float v{}_p4; vec3 v{}_p5; vec3 v{}_p6; float v{}_p7;\n",id,id,id,id,id,id,id,id));
  src.push_str(&format!("  v{}_p0=maker_sdf_render_{}(maker_uv,u_n{}_speed,u_n{}_max_distance,u_n{}_epsilon,u_n{}_step_scale,u_n{}_fov,u_n{}_glow_falloff,u_n{}_glow_gain,{},{},u_n{}_camera_source,v{}_p1,v{}_p2,v{}_p3,v{}_p4,v{}_p5,v{}_p6,v{}_p7);\n",id,id,id,id,id,id,id,id,id,vars["ray_origin"],vars["ray_direction"],id,id,id,id,id,id,id,id));
  emitted.insert(id);return Ok(());
 }
 let mut expressions=Vec::new();
 for (port,output) in m.outputs.iter().enumerate(){
  if output.ty.starts_with("field"){continue}
  // Keep every output consumed by a reachable node. This intentionally retains
  // conservative dependencies for specialized consumers and callable contexts.
  if !graph.connections.iter().any(|c|c.from==id && c.from_port==port && reachable.contains(&c.to)){continue}
  let expression=if m.id=="screenshaver.test.universal_test" && output.id=="preview_color" { universal_preview_expression(graph,id) } else if m.id=="screenshaver.fields.traversal" && graph.input_connection(id,0).is_some(){format!("maker_traverse_connected_{}({},{},{},{},{})",id,vars["position"],vars["depth"],vars["density"],vars["speed"],vars["mode"])}else{substitute(&m.glsl_outputs[port],&vars)?};
  expressions.push((port,output.ty.as_str(),expression));
 }
 // Only explicitly audited pure helpers are eligible for common-result reuse.
 // Never hoist arbitrary community expressions or cross sampling contexts.
 let helper=match m.id.as_str(){
  "screenshaver.coordinates.iterative_coordinate_fold_2d" if m.glsl_outputs.len()==5 && m.glsl_outputs.iter().all(|e| matches!(e.as_str(), "maker_iterative_coordinate_fold_2d({position},{time},{center},{iterations},{iteration_scale},{rotation},{speed},{fold_mode},{period},{symmetry},{sectors},{twist},{frequency},{response},{width},{exponent},{gain}).folded" | "maker_iterative_coordinate_fold_2d({position},{time},{center},{iterations},{iteration_scale},{rotation},{speed},{fold_mode},{period},{symmetry},{sectors},{twist},{frequency},{response},{width},{exponent},{gain}).final_radius" | "maker_iterative_coordinate_fold_2d({position},{time},{center},{iterations},{iteration_scale},{rotation},{speed},{fold_mode},{period},{symmetry},{sectors},{twist},{frequency},{response},{width},{exponent},{gain}).minimum_radius" | "maker_iterative_coordinate_fold_2d({position},{time},{center},{iterations},{iteration_scale},{rotation},{speed},{fold_mode},{period},{symmetry},{sectors},{twist},{frequency},{response},{width},{exponent},{gain}).mean_radius" | "maker_iterative_coordinate_fold_2d({position},{time},{center},{iterations},{iteration_scale},{rotation},{speed},{fold_mode},{period},{symmetry},{sectors},{twist},{frequency},{response},{width},{exponent},{gain}).accumulated_response"))=>Some(("maker_iterative_coordinate_fold_2d","MakerFold2DResult")),
  "screenshaver.fields.euclidean_voronoi_boundary_field_2d" if m.glsl_outputs.len()==3 && m.glsl_outputs.iter().all(|e| matches!(e.as_str(), "maker_evb_sample({position},{density},{jitter}).x" | "maker_evb_sample({position},{density},{jitter}).y" | "(1.0-smoothstep(max({width},0.0001),max({width},0.0001)+max({softness},0.0001),maker_evb_sample({position},{density},{jitter}).y))"))=>Some(("maker_evb_sample","vec2")),
  "screenshaver.geometry.spherical_orbital_path_3d" if m.glsl_outputs.len()==6 && m.glsl_outputs.iter().all(|e| matches!(e.as_str(), "maker_orbit({time}+{time_offset},{center},{radius}+{radius_modulation},{inclination},{orbital_speed},{precession_speed},{wobble_amplitude},{wobble_frequency},{initial_phase}).p" | "maker_orbit({time}+{time_offset},{center},{radius}+{radius_modulation},{inclination},{orbital_speed},{precession_speed},{wobble_amplitude},{wobble_frequency},{initial_phase}).radial" | "maker_orbit({time}+{time_offset},{center},{radius}+{radius_modulation},{inclination},{orbital_speed},{precession_speed},{wobble_amplitude},{wobble_frequency},{initial_phase}).tangent" | "maker_orbit({time}+{time_offset},{center},{radius}+{radius_modulation},{inclination},{orbital_speed},{precession_speed},{wobble_amplitude},{wobble_frequency},{initial_phase}).velocity" | "maker_orbit({time}+{time_offset},{center},{radius}+{radius_modulation},{inclination},{orbital_speed},{precession_speed},{wobble_amplitude},{wobble_frequency},{initial_phase}).lateral" | "maker_orbit({time}+{time_offset},{center},{radius}+{radius_modulation},{inclination},{orbital_speed},{precession_speed},{wobble_amplitude},{wobble_frequency},{initial_phase}).speed"))=>Some(("maker_orbit","MakerOrbit")),
  _=>None,
 };
 if let Some((function,ty))=helper {
  let temporary=format!("maker_shared_{id}");
  if let Some(call)=reuse_audited_call(&mut expressions,function,&temporary){
   src.push_str(&format!("  {ty} {temporary}={call};\n"));
  }
 }
 for (port,ty,expression) in expressions{
  src.push_str(&format!("  {ty} v{id}_p{port}={expression};\n"));
 }
 emitted.insert(id);Ok(())
}

// Locate a complete balanced call, not merely a prefix ending at the first ')'.
fn audited_call(expression:&str,function:&str)->Option<String>{
 let prefix=format!("{function}(");
 let start=expression.find(&prefix)?;
 let mut depth=0usize;
 for (offset,ch) in expression[start+function.len()..].char_indices(){
  match ch {
   '('=>depth+=1,
   ')'=>{depth=depth.checked_sub(1)?;if depth==0{return Some(expression[start..start+function.len()+offset+1].to_owned())}},
   _=>{},
  }
 }
 None
}
fn reuse_audited_call(expressions:&mut [(usize,&str,String)],function:&str,temporary:&str)->Option<String>{
 let mut calls=Vec::new();
 for (_,_,expression) in expressions.iter(){
  if let Some(call)=audited_call(expression,function){calls.push(call)}
 }
 let first=calls.first()?.clone();
 if calls.len()<2 || calls.iter().any(|call|call!=&first){return None}
 for (_,_,expression) in expressions.iter_mut(){*expression=expression.replace(&first,temporary);}
 Some(first)
}
#[cfg(test)]
mod output_reuse_tests {
 use super::*;
 #[test] fn identical_calls_reused(){
  let mut e=vec![(0,"float","sample(vec2(0.0),x).x".to_owned()),(1,"float","1.0-sample(vec2(0.0),x).y".to_owned())];
  assert_eq!(reuse_audited_call(&mut e,"sample","shared").as_deref(),Some("sample(vec2(0.0),x)"));
  assert_eq!(e[0].2,"shared.x");assert_eq!(e[1].2,"1.0-shared.y");
 }
 #[test] fn distinct_calls_preserved(){
  let mut e=vec![(0,"float","sample(a).x".to_owned()),(1,"float","sample(b).y".to_owned())];
  let original=e.clone();assert!(reuse_audited_call(&mut e,"sample","shared").is_none());assert_eq!(e,original);
 }
 #[test] fn single_call_preserved(){
  let mut e=vec![(0,"float","sample(a).x".to_owned())];
  assert!(reuse_audited_call(&mut e,"sample","shared").is_none());assert_eq!(e[0].2,"sample(a).x");
 }
}

// A callable is one GLSL function per node output.  All functions take the sampling
// position explicitly, allowing domain transformations to evaluate at different
// coordinates without copying the entire upstream expression tree.
struct CallableGenerator<'a> {
 graph:&'a Graph,
 definitions:Vec<String>,
 signatures:Vec<String>,
 generated:HashSet<(NodeId,usize)>,
 active:HashSet<(NodeId,usize)>,
}
impl<'a> CallableGenerator<'a> {
 fn new(graph:&'a Graph)->Self { Self {graph,definitions:Vec::new(),signatures:Vec::new(),generated:HashSet::new(),active:HashSet::new()} }
 fn name(id:NodeId,port:usize)->String {format!("maker_callable_{id}_{port}")}
 fn reference(&self,id:NodeId,port:usize,position:&str,expected:&str)->Result<String,String>{
  let n=self.graph.node(id).ok_or("Callable source missing")?;
  let ty=&n.definition().outputs.get(port).ok_or("Callable output missing")?.ty;
  if ty!=expected{return Err(format!("Callable source requires {expected}, got {ty}"))}
  if !self.generated.contains(&(id,port)){return Err(format!("Callable source not generated: {id}:{port}"))}
  Ok(format!("{}({position})",Self::name(id,port)))
 }
 fn require(&mut self,id:NodeId,port:usize,expected:&str)->Result<(),String>{
  let n=self.graph.node(id).ok_or("Callable source missing")?;
  let m=n.definition();
  let ty=m.outputs.get(port).ok_or("Callable output missing")?.ty.clone();
  if m.id=="screenshaver.test.universal_test" && (ty=="field2d" || ty=="field3d") {
   if ty!=expected{return Err(format!("Callable source requires {expected}, got {ty}"))}
   if self.generated.insert((id,port)) {
    let function=if ty=="field2d" {format!("maker_field2d_{id}_{port}")}else{Self::name(id,port)};
    let arg=if ty=="field2d" {"vec2"}else{"vec3"};
    let fallback=if ty=="field2d" {"maker_universal_field2d_fallback"}else{"maker_universal_field3d_fallback"};
    self.signatures.push(format!("float {function}({arg} maker_position);\n"));
    self.definitions.push(format!("float {function}({arg} maker_position){{return {fallback}(maker_position);}}\n"));
   }
   return Ok(())
  }
  if ty!=expected{return Err(format!("Callable source requires {expected}, got {ty}"))}
  if self.generated.contains(&(id,port)){return Ok(())}
  if !self.active.insert((id,port)){return Err("Callable field cycle".into())}
  let result=(||->Result<String,String>{
   if m.terminal{return Err("Output terminal cannot be sampled".into())}
   let mut vars=HashMap::new();
   let position="maker_position";
   let domain_position=if m.id=="screenshaver.geometry.domain_repeat_3d" {
    format!("(mod(({position})+0.5*vec3(u_n{id}_span_x,u_n{id}_span_y,u_n{id}_span_z),vec3(u_n{id}_span_x,u_n{id}_span_y,u_n{id}_span_z))-0.5*vec3(u_n{id}_span_x,u_n{id}_span_y,u_n{id}_span_z))")
   }else if m.id=="screenshaver.geometry.domain_fold_3d"{
    format!("maker_blue_fold_{id}({position})")
   }else if m.id=="screenshaver.geometry.rotation_3d"{
    format!("maker_rotate_3d({position},u_n{id}_xy+u_time*u_n{id}_speed,u_n{id}_yz+u_time*u_n{id}_speed,u_n{id}_xz+u_time*u_n{id}_speed)")
   }else{position.to_owned()};
   for (index,input) in m.inputs.iter().enumerate(){
    let is_domain=(m.id=="screenshaver.geometry.domain_repeat_3d" || m.id=="screenshaver.geometry.domain_fold_3d" || m.id=="screenshaver.geometry.rotation_3d") && input.id=="field";
    let value=if is_domain {
     let c=self.graph.input_connection(id,index).ok_or_else(||format!("{} requires a connected field",m.name))?;
     self.require(c.from,c.from_port,&input.ty)?;
     format!("{}({domain_position})",Self::name(c.from,c.from_port))
    }else if input.semantic_role=="position3d" && input.ty=="vec3" && self.graph.input_connection(id,index).is_none(){position.to_owned()}
    else if (input.semantic_role=="position2d"||input.semantic_role=="uv") && input.ty=="vec2" && self.graph.input_connection(id,index).is_none(){format!("({position}).xy")}
    else if input.id=="position" && input.ty=="vec3" && self.graph.input_connection(id,index).is_none(){position.to_owned()}
    else if m.id=="screenshaver.color.angular_wave_background" && input.id=="direction" && self.graph.input_connection(id,index).is_none(){position.to_owned()}
    else if m.id=="screenshaver.camera.sinusoidal_path" && input.id=="z" && self.graph.input_connection(id,index).is_none(){format!("({position}).z")}
    else if let Some(c)=self.graph.input_connection(id,index){
     self.require(c.from,c.from_port,&input.ty)?;
     format!("{}({position})",Self::name(c.from,c.from_port))
    }else{input.default.clone()};
    if value.is_empty(){return Err(format!("Missing callable input {}:{}",m.id,input.id))}
    vars.insert(input.id.clone(),value);
   }
   for parameter in &m.parameters{vars.insert(parameter.id.clone(),format!("u_n{}_{}",id,parameter.id));}
   substitute(&m.glsl_outputs[port],&vars)
  })();
  self.active.remove(&(id,port));
  let expression=result?;
  let name=Self::name(id,port);
  self.signatures.push(format!("{ty} {name}(vec3 maker_position);\n"));
  self.definitions.push(format!("{ty} {name}(vec3 maker_position){{vec2 maker_uv=maker_position.xy;return {expression};}}\n"));
  self.generated.insert((id,port));
  if self.graph.node(id).is_some_and(|n|n.callable_spec()==Some((port,"field2d"))) {
   let wrapper=format!("maker_field2d_{id}_{}",m.outputs.len());
   self.signatures.push(format!("float {wrapper}(vec2 p);\n"));
   self.definitions.push(format!("float {wrapper}(vec2 p){{return {name}(vec3(p,0.0));}}\n"));
  }
  Ok(())
 }
 fn source(&self)->String{
  let mut result=String::new();
  for signature in &self.signatures{result.push_str(signature)}
  for definition in &self.definitions{result.push_str(definition)}
  result
 }
}
