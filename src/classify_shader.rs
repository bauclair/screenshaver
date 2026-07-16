#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderKind {
    ShaderToy,
    Isf,
    NativeGLSL,
}

pub fn classify_shader(
    source: &str,
) -> ShaderKind {

    if crate::parse_isf::looks_like_isf(
        source
    ) {
        ShaderKind::Isf

    } else if looks_like_shadertoy(
        source
    ) {
        ShaderKind::ShaderToy

    } else {
        ShaderKind::NativeGLSL
    }
}

fn looks_like_shadertoy(
    source: &str,
) -> bool {

    source.contains("mainImage")
}