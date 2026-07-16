//! Apply metadata-defined shader input values to a linked OpenGL program.

use std::ffi::CString;

pub fn apply(program: u32, inputs: &[crate::isf_types::ShaderInput]) {
    for input in inputs {
        let Ok(name) = CString::new(input.name.as_str()) else {
            continue;
        };

        let location = unsafe {
            gl::GetUniformLocation(program, name.as_ptr())
        };

        if location == -1 {
            continue;
        }

        unsafe {
            match input.value {
                crate::isf_types::ShaderInputValue::Float(value) => {
                    gl::Uniform1f(location, value);
                }
                crate::isf_types::ShaderInputValue::Bool(value) => {
                    gl::Uniform1i(location, if value { 1 } else { 0 });
                }
                crate::isf_types::ShaderInputValue::Integer(value) => {
                    gl::Uniform1i(location, value);
                }
                crate::isf_types::ShaderInputValue::Point2D(value) => {
                    gl::Uniform2f(location, value[0], value[1]);
                }
                crate::isf_types::ShaderInputValue::Color(value) => {
                    gl::Uniform4f(location, value[0], value[1], value[2], value[3]);
                }
            }
        }
    }
}
