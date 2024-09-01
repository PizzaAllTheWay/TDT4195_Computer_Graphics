use gl;
use std::{
    ptr,
    str,
    ffi::CString,
    path::Path,
};

pub struct Shader {
    pub program_id: u32,
}

pub struct ShaderBuilder {
    program_id: u32,
    shaders: Vec::<u32>,
}

#[allow(dead_code)]
pub enum ShaderType {
    Vertex,
    Fragment,
    TessellationControl,
    TessellationEvaluation,
    Geometry,
}

impl Shader {
    // Make sure the shader is active before calling this
    pub unsafe fn get_uniform_location(&self, name: &str) -> i32 {
        let name_cstr = CString::new(name).expect("CString::new failed");
        gl::GetUniformLocation(self.program_id, name_cstr.as_ptr())
    }

    pub unsafe fn activate(&self) {
        gl::UseProgram(self.program_id);
    }

    // * Custom method to edit shader color
    /// The power of ChatGPT and my final brain cell X)
    /// Sets a vec3 uniform in the shader program.
    /// 
    /// # Parameters
    /// - `name`: The name of the uniform variable in the shader.
    /// - `value`: A reference to an array of 3 floats representing the vec3 value to be set.
    ///
    /// # Safety
    /// This method is unsafe because it interacts with the raw OpenGL API, which assumes
    /// that you are passing valid data and operating in a valid OpenGL context.
    pub unsafe fn set_uniform_vec3(&self, name: &str, value: &[f32; 3]) {
        // Convert the uniform name from a Rust string to a C-compatible string.
        // This is necessary because OpenGL functions expect C strings.
        let name_cstr = CString::new(name).expect("CString::new failed");

        // Get the location of the uniform variable in the shader program.
        // This location is necessary to update the value of the uniform.
        let uniform_location = gl::GetUniformLocation(self.program_id, name_cstr.as_ptr());

        // Check if the uniform location is valid (i.e., not -1).
        // If the location is valid, set the value of the uniform using `glUniform3fv`.
        if uniform_location != -1 {
            // `glUniform3fv` is used to set the value of a vec3 uniform variable in the shader.
            // Parameters:
            // - uniform_location: The location of the uniform variable.
            // - 1: The number of vec3 values to set (in this case, just 1).
            // - value.as_ptr(): A pointer to the array of 3 floats representing the vec3 value.
            gl::Uniform3fv(uniform_location, 1, value.as_ptr());
        } else {
            // If the uniform location is invalid (i.e., the uniform was not found),
            // print a warning message to the console.
            println!("Warning: uniform '{}' not found in shader!", name);
        }
    }

    // * Custom method to set a float uniform in the shader program
    /// Sets a float uniform in the shader program.
    /// 
    /// # Parameters
    /// - `name`: The name of the uniform variable in the shader.
    /// - `value`: The float value to be set.
    ///
    /// # Safety
    /// This method is unsafe because it interacts with the raw OpenGL API, which assumes
    /// that you are passing valid data and operating in a valid OpenGL context.
    pub unsafe fn set_uniform_float(&self, name: &str, value: f32) {
        let name_cstr = CString::new(name).expect("CString::new failed");
        let uniform_location = gl::GetUniformLocation(self.program_id, name_cstr.as_ptr());

        if uniform_location != -1 {
            gl::Uniform1f(uniform_location, value);
        } else {
            println!("Warning: uniform '{}' not found in shader!", name);
        }
    }
}

impl Into<gl::types::GLenum> for ShaderType {
    fn into(self) -> gl::types::GLenum {
        match self {
            ShaderType::Vertex                  => { gl::VERTEX_SHADER          },
            ShaderType::Fragment                => { gl::FRAGMENT_SHADER        },
            ShaderType::TessellationControl     => { gl::TESS_CONTROL_SHADER    },
            ShaderType::TessellationEvaluation  => { gl::TESS_EVALUATION_SHADER } ,
            ShaderType::Geometry                => { gl::GEOMETRY_SHADER        },
        }
    }
}

impl ShaderType {
    fn from_ext(ext: &std::ffi::OsStr) -> Result<ShaderType, String> {
        match ext.to_str().expect("Failed to read extension") {
            "vert" => { Ok(ShaderType::Vertex) },
            "frag" => { Ok(ShaderType::Fragment) },
            "tcs"  => { Ok(ShaderType::TessellationControl) },
            "tes"  => { Ok(ShaderType::TessellationEvaluation) },
            "geom" => { Ok(ShaderType::Geometry) },
            e => { Err(e.to_string()) },
        }
    }
}

impl ShaderBuilder {
    pub unsafe fn new() -> ShaderBuilder {
        ShaderBuilder {
            program_id: gl::CreateProgram(),
            shaders: vec![],
        }
    }

    pub unsafe fn attach_file(self, shader_path: &str) -> ShaderBuilder {
        let path = Path::new(shader_path);
        if let Some(extension) = path.extension() {
            let shader_type = ShaderType::from_ext(extension)
                .expect("Failed to parse file extension.");
            let shader_src = std::fs::read_to_string(path)
                .expect(&format!("Failed to read shader source. {}", shader_path));
            self.compile_shader(&shader_src, shader_type)
        } else {
            panic!("Failed to read extension of file with path: {}", shader_path);
        }
    }

    pub unsafe fn compile_shader(mut self, shader_src: &str, shader_type: ShaderType) -> ShaderBuilder {
        let shader = gl::CreateShader(shader_type.into());
        let c_str_shader = CString::new(shader_src.as_bytes()).unwrap();
        gl::ShaderSource(shader, 1, &c_str_shader.as_ptr(), ptr::null());
        gl::CompileShader(shader);

        if !self.check_shader_errors(shader) {
            panic!("Shader failed to compile.");
        }

        self.shaders.push(shader);

        self
    }

    unsafe fn check_shader_errors(&self, shader_id: u32) -> bool {
        let mut success = i32::from(gl::FALSE);
        let mut info_log = Vec::with_capacity(512);
        info_log.set_len(512 - 1);
        gl::GetShaderiv(shader_id, gl::COMPILE_STATUS, &mut success);
        if success != i32::from(gl::TRUE) {
            gl::GetShaderInfoLog(
                shader_id,
                512,
                ptr::null_mut(),
                info_log.as_mut_ptr() as *mut gl::types::GLchar,
            );
            println!("ERROR::Shader Compilation Failed!\n{}", String::from_utf8_lossy(&info_log));
            return false;
        }
        true
    }

    unsafe fn check_linker_errors(&self) -> bool {
        let mut success = i32::from(gl::FALSE);
        let mut info_log = Vec::with_capacity(512);
        info_log.set_len(512 - 1);
        gl::GetProgramiv(self.program_id, gl::LINK_STATUS, &mut success);
        if success != i32::from(gl::TRUE) {
            gl::GetProgramInfoLog(
                self.program_id,
                512,
                ptr::null_mut(),
                info_log.as_mut_ptr() as *mut gl::types::GLchar,
            );
            println!("ERROR::SHADER::PROGRAM::COMPILATION_FAILED\n{}", String::from_utf8_lossy(&info_log));
            return false;
        }
        true
    }

    #[must_use = "The shader program is useless if not stored in a variable."]
    pub unsafe fn link(self) -> Shader {
        for &shader in &self.shaders {
            gl::AttachShader(self.program_id, shader);
        }
        gl::LinkProgram(self.program_id);

        // todo:: use this to make safer abstraction
        self.check_linker_errors();

        for &shader in &self.shaders {
            gl::DeleteShader(shader);
        }

        Shader {
            program_id: self.program_id
        }
    }
}
