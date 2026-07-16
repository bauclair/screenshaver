//! Shared data structures for Interactive Shader Format metadata and runtime inputs.

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
pub struct IsfMetadata {
    #[serde(rename = "ISFVSN", default)]
    pub version: Option<Value>,
    #[serde(rename = "CREDIT", default)]
    pub credit: Option<String>,
    #[serde(rename = "DESCRIPTION", default)]
    pub description: Option<String>,
    #[serde(rename = "CATEGORIES", default)]
    pub categories: Vec<String>,
    #[serde(rename = "INPUTS", default)]
    pub inputs: Vec<IsfInputMetadata>,
    #[serde(rename = "PASSES", default)]
    pub passes: Vec<Value>,
    #[serde(rename = "IMPORTED", default)]
    pub imported: Value,
}

impl IsfMetadata {
    pub fn version_name(&self) -> String {
        match self.version.as_ref() {
            Some(Value::String(value)) => value.clone(),
            Some(Value::Number(value)) => value.to_string(),
            Some(value) => value.to_string(),
            None => "unspecified".to_string(),
        }
    }

    pub fn pass_count(&self) -> usize {
        if self.passes.is_empty() { 1 } else { self.passes.len() }
    }

    pub fn has_imported_resources(&self) -> bool {
        match &self.imported {
            Value::Null => false,
            Value::Array(values) => !values.is_empty(),
            Value::Object(values) => !values.is_empty(),
            _ => true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct IsfInputMetadata {
    #[serde(rename = "NAME")]
    pub name: String,
    #[serde(rename = "TYPE")]
    pub input_type: String,
    #[serde(rename = "DEFAULT", default)]
    pub default_value: Value,
    #[serde(rename = "MIN", default)]
    pub minimum: Value,
    #[serde(rename = "MAX", default)]
    pub maximum: Value,
    #[serde(rename = "VALUES", default)]
    pub values: Vec<Value>,
    #[serde(rename = "LABELS", default)]
    pub labels: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct IsfDocument {
    pub metadata: IsfMetadata,
    pub metadata_start: usize,
    pub metadata_end: usize,
    pub shader_source: String,
}

#[derive(Debug, Clone)]
pub struct ShaderInput {
    pub name: String,
    pub value: ShaderInputValue,
}

#[derive(Debug, Clone)]
pub enum ShaderInputValue {
    Float(f32),
    Bool(bool),
    Integer(i32),
    Point2D([f32; 2]),
    Color([f32; 4]),
}

impl ShaderInputValue {
    pub fn glsl_type(&self) -> &'static str {
        match self {
            Self::Float(_) => "float",
            Self::Bool(_) => "bool",
            Self::Integer(_) => "int",
            Self::Point2D(_) => "vec2",
            Self::Color(_) => "vec4",
        }
    }
}

