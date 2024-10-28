
#[derive(Debug)]
pub struct ShaderProgramError(String);


pub struct TextShader {
    pub id: u32
}

impl TextShader {
    pub fn new(vertex_code: &str, fragment_code: &str) -> Result<Self, ShaderProgramError> {
        unsafe {
            let vertex_shader = gl::CreateShader(gl::VERTEX_SHADER);
            gl::ShaderSource(vertex_shader, 1, &vertex_code.as_bytes().as_ptr().cast(), &vertex_code.len().try_into().unwrap());
            gl::CompileShader(vertex_shader);
            check_shader_compile_errors(vertex_shader, "VERTEX")?;

            let fragment_shader = gl::CreateShader(gl::FRAGMENT_SHADER);
            gl::ShaderSource(fragment_shader, 1, &fragment_code.as_bytes().as_ptr().cast(), &fragment_code.len().try_into().unwrap());
            gl::CompileShader(fragment_shader);
            check_shader_compile_errors(fragment_shader, "FRAGMENT")?;

            let id = gl::CreateProgram();
            gl::AttachShader(id, vertex_shader);
            gl::AttachShader(id, fragment_shader);
            gl::LinkProgram(id);
            check_program_link_errors(id)?;

            // already linked to program, no need anymore
            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);
            
            Ok(Self { id })
        }
    }

    pub fn use_program(&self) {
        unsafe { gl::UseProgram(self.id) };
    }

}



pub struct RectShader {
    pub id: u32
}

impl RectShader {
    pub fn new(vertex_code: &str, fragment_code: &str) -> Result<Self, ShaderProgramError> {
        unsafe {
            let vertex_shader = gl::CreateShader(gl::VERTEX_SHADER);
            gl::ShaderSource(vertex_shader, 1, &vertex_code.as_bytes().as_ptr().cast(), &vertex_code.len().try_into().unwrap());
            gl::CompileShader(vertex_shader);
            check_shader_compile_errors(vertex_shader, "VERTEX")?;

            let fragment_shader = gl::CreateShader(gl::FRAGMENT_SHADER);
            gl::ShaderSource(fragment_shader, 1, &fragment_code.as_bytes().as_ptr().cast(), &fragment_code.len().try_into().unwrap());
            gl::CompileShader(fragment_shader);
            check_shader_compile_errors(fragment_shader, "FRAGMENT")?;

            let id = gl::CreateProgram();
            gl::AttachShader(id, vertex_shader);
            gl::AttachShader(id, fragment_shader);
            gl::LinkProgram(id);
            check_program_link_errors(id)?;

            // already linked to program, no need anymore
            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);
            
            Ok(Self { id })
        }
    }

    pub fn use_program(&self) {
        unsafe { gl::UseProgram(self.id) };
    }
}


unsafe fn check_shader_compile_errors(shader: gl::types::GLuint, shader_type: &str) -> Result<(), ShaderProgramError>{
    let mut success = 0;
    let mut log_len = 0i32;
    let mut info_log: Vec<u8> = Vec::with_capacity(1024);
    gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
    if success == 0 {
        gl::GetShaderInfoLog(shader, 1024, &mut log_len, info_log.as_mut_ptr().cast());
        info_log.set_len(log_len as usize);
        let e = format!("SHADER COMPILATION ERROR of type: {}\n{}", shader_type, String::from_utf8_lossy(&info_log));
        return Err(ShaderProgramError(e))
    }

    Ok(())
}

unsafe fn check_program_link_errors(program: u32) -> Result<(), ShaderProgramError> {
    let mut success = 0;
    let mut log_len = 0i32;
    let mut info_log: Vec<u8> = Vec::with_capacity(1024);
    gl::GetProgramiv(program, gl::LINK_STATUS, &mut success);
    if success == 0 {
        gl::GetProgramInfoLog(program, 1024, &mut log_len, info_log.as_mut_ptr().cast());
        info_log.set_len(log_len as usize);
        let e = format!("SHADER COMPILATION ERROR:\n{}", String::from_utf8_lossy(&info_log));
        return Err(ShaderProgramError(e))
    }

    Ok(())
}
