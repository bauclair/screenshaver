//! Data-only Maker module definitions. Trust is assigned by the loader, never JSON.
use serde_json::Value;

#[derive(Clone,Debug,PartialEq,Eq)]
pub enum Trust { Official, Community }
#[derive(Clone,Debug)]
pub struct Port { pub id:String, pub name:String, pub ty:String, pub default:String }
#[derive(Clone,Debug)]
pub struct Parameter { pub id:String, pub name:String, pub ty:String, pub default:Value, pub min:f32, pub max:f32 }
#[derive(Clone,Debug)]
pub struct Module {
    pub id:String, pub name:String, pub category:String, pub created_by:String,
    pub description:String, pub uses:String, pub notes:String, pub header_color:[u8;3],
    pub inputs:Vec<Port>, pub outputs:Vec<Port>, pub parameters:Vec<Parameter>,
    pub glsl:String, pub glsl_outputs:Vec<String>, pub helpers:String, pub terminal:bool, pub trust:Trust,
}
fn required<'a>(v:&'a Value,key:&str)->Result<&'a str,String>{v.get(key).and_then(Value::as_str).filter(|s|!s.trim().is_empty()).ok_or_else(||format!("Missing or empty '{key}'"))}
fn ports(v:&Value,key:&str)->Result<Vec<Port>,String>{
    let list=v.get(key).and_then(Value::as_array).ok_or_else(||format!("Missing {key} array"))?;
    list.iter().map(|p|{
        let ty=required(p,"type")?.to_string();
        if !matches!(ty.as_str(),"float"|"vec2"|"vec3"){return Err(format!("Unsupported port type: {ty}"))}
        Ok(Port{id:required(p,"id")?.into(),name:required(p,"name")?.into(),ty,default:p.get("default").and_then(Value::as_str).unwrap_or("").into()})
    }).collect()
}
impl Module {
 pub fn parse(src:&str,trust:Trust)->Result<Self,String>{
    let v:Value=serde_json::from_str(src).map_err(|e|e.to_string())?;
    if v.get("schema_version").and_then(Value::as_u64)!=Some(1){return Err("Unsupported schema_version".into())}
    let id=required(&v,"id")?.to_string();
    if !id.chars().all(|c|c.is_ascii_lowercase()||c.is_ascii_digit()||c=='.'||c=='_'||c=='-') || !id.contains('.') {return Err("Invalid module ID".into())}
    if trust==Trust::Community && (id=="screenshaver"||id.starts_with("screenshaver.")){return Err("Reserved Screenshaver module namespace".into())}
    let color=required(&v,"header_color")?;
    if color.len()!=7||!color.starts_with('#'){return Err("header_color must be #RRGGBB".into())}
    let rgb=|i|u8::from_str_radix(&color[i..i+2],16).map_err(|_|"Invalid header_color".to_string());
    let inputs=ports(&v,"inputs")?;let outputs=ports(&v,"outputs")?;
    let valid_ident=|s:&str|{let mut chars=s.chars();chars.next().is_some_and(|c|c.is_ascii_alphabetic()||c=='_')&&chars.all(|c|c.is_ascii_alphanumeric()||c=='_')};
    if inputs.iter().chain(outputs.iter()).any(|p|!valid_ident(&p.id)){return Err("Invalid port identifier".into())}
    let mut parameters=Vec::new();
    for p in v.get("parameters").and_then(Value::as_array).ok_or("Missing parameters array")?{
        let ty=required(p,"type")?.to_string();
        if !matches!(ty.as_str(),"float"|"color3"){return Err(format!("Unsupported parameter type: {ty}"))}
        let default=p.get("default").ok_or("Missing parameter default")?.clone();
        if ty=="float" && default.as_f64().is_none(){return Err("Float default must be numeric".into())}
        if ty=="color3" && !default.as_array().is_some_and(|a|a.len()==3&&a.iter().all(|x|x.as_f64().is_some())){return Err("color3 default must be RGB array".into())}
        let min=p.get("min").and_then(Value::as_f64).unwrap_or(0.0) as f32;
        let max=p.get("max").and_then(Value::as_f64).unwrap_or(1.0) as f32;
        if min>max{return Err("Parameter min exceeds max".into())}
        let param_id=required(p,"id")?;
        if !valid_ident(param_id){return Err("Invalid parameter identifier".into())}
        parameters.push(Parameter{id:param_id.into(),name:required(p,"name")?.into(),ty,default,min,max});
    }
    let unique=|items:Vec<&str>|{let mut s=std::collections::HashSet::new();items.into_iter().all(|x|s.insert(x.to_owned()))};
    if !unique(inputs.iter().map(|p|p.id.as_str()).collect())||!unique(outputs.iter().map(|p|p.id.as_str()).collect())||!unique(parameters.iter().map(|p|p.id.as_str()).collect()){return Err("Duplicate port/parameter identifier".into())}
    let terminal=v.get("terminal").and_then(Value::as_bool).unwrap_or(false);
    if terminal && (!outputs.is_empty()||inputs.len()!=1||inputs[0].ty!="vec3"){return Err("Terminal must have one vec3 input and no outputs".into())}
    if !terminal && (outputs.is_empty() || outputs.len()>8){return Err("Nonterminal modules require 1 to 8 outputs".into())}
     if inputs.len()>8{return Err("Modules support up to 8 inputs".into())}
     let glsl_outputs=if terminal {Vec::new()} else if outputs.len()==1 {vec![required(&v,"glsl")?.to_string()]} else {
         let expressions=v.get("glsl_outputs").and_then(Value::as_object).ok_or("Multiple outputs require glsl_outputs object")?;
         outputs.iter().map(|port|expressions.get(&port.id).and_then(Value::as_str).filter(|s|!s.trim().is_empty()).map(str::to_owned).ok_or_else(||format!("Missing GLSL for output {}",port.id))).collect::<Result<Vec<_>,_>>()?
     };
    let glsl=if terminal {required(&v,"glsl")?.to_string()} else {glsl_outputs[0].clone()};
    if src.len()>131072{return Err("Module definition exceeds 128 KiB".into())}
    if glsl_outputs.iter().chain(std::iter::once(&glsl)).any(|s|s.contains("#version")||s.contains("#include")||s.contains("void main")) {return Err("GLSL module may not redefine shader entry point".into())}
    Ok(Self{id,name:required(&v,"name")?.into(),category:required(&v,"category")?.into(),created_by:required(&v,"created_by")?.into(),
        description:required(&v,"description")?.into(),uses:required(&v,"uses")?.into(),notes:required(&v,"notes")?.into(),
        header_color:[rgb(1)?,rgb(3)?,rgb(5)?],inputs,outputs,parameters,glsl,glsl_outputs,
        helpers:v.get("helpers").and_then(Value::as_str).unwrap_or("").into(),terminal,trust})
 }
}
