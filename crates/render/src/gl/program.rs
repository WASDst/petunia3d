//! Programas GLSL: compilação, link com limpeza em caso de erro e uniforms por nome.

use glow::{HasContext, NativeProgram, NativeUniformLocation};

/// Programa ligado com as localizações dos uniforms declarados.
pub(super) struct Program {
    pub(super) id: NativeProgram,
    locations: Vec<(&'static str, Option<NativeUniformLocation>)>,
}

impl Program {
    /// # Safety
    /// O contexto GL precisa estar corrente na thread de renderização.
    pub(super) unsafe fn new(
        gl: &glow::Context,
        label: &str,
        vertex_source: &str,
        fragment_source: &str,
        uniforms: &[&'static str],
    ) -> Result<Self, String> {
        // SAFETY: delegated to the caller (current context on this thread).
        unsafe {
            let id = link(gl, vertex_source, fragment_source)
                .map_err(|error| format!("{label}: {error}"))?;
            let locations = uniforms
                .iter()
                .map(|name| (*name, gl.get_uniform_location(id, name)))
                .collect();
            Ok(Self { id, locations })
        }
    }

    fn location(&self, name: &str) -> Option<&NativeUniformLocation> {
        self.locations
            .iter()
            .find(|(candidate, _)| *candidate == name)
            .and_then(|(_, location)| location.as_ref())
    }

    pub(super) unsafe fn bind(&self, gl: &glow::Context) {
        // SAFETY: caller keeps the context current.
        unsafe { gl.use_program(Some(self.id)) }
    }

    pub(super) unsafe fn mat4(&self, gl: &glow::Context, name: &str, value: &[f32; 16]) {
        // SAFETY: caller keeps the context current and this program bound.
        unsafe { gl.uniform_matrix_4_f32_slice(self.location(name), false, value) }
    }

    pub(super) unsafe fn mat3(&self, gl: &glow::Context, name: &str, value: &[f32; 9]) {
        // SAFETY: caller keeps the context current and this program bound.
        unsafe { gl.uniform_matrix_3_f32_slice(self.location(name), false, value) }
    }

    pub(super) unsafe fn f32(&self, gl: &glow::Context, name: &str, value: f32) {
        // SAFETY: caller keeps the context current and this program bound.
        unsafe { gl.uniform_1_f32(self.location(name), value) }
    }

    pub(super) unsafe fn i32(&self, gl: &glow::Context, name: &str, value: i32) {
        // SAFETY: caller keeps the context current and this program bound.
        unsafe { gl.uniform_1_i32(self.location(name), value) }
    }

    pub(super) unsafe fn bool(&self, gl: &glow::Context, name: &str, value: bool) {
        // SAFETY: caller keeps the context current and this program bound.
        unsafe { gl.uniform_1_i32(self.location(name), i32::from(value)) }
    }

    pub(super) unsafe fn vec2(&self, gl: &glow::Context, name: &str, value: [f32; 2]) {
        // SAFETY: caller keeps the context current and this program bound.
        unsafe { gl.uniform_2_f32(self.location(name), value[0], value[1]) }
    }

    pub(super) unsafe fn vec3(&self, gl: &glow::Context, name: &str, value: [f32; 3]) {
        // SAFETY: caller keeps the context current and this program bound.
        unsafe { gl.uniform_3_f32(self.location(name), value[0], value[1], value[2]) }
    }

    pub(super) unsafe fn vec4(&self, gl: &glow::Context, name: &str, value: [f32; 4]) {
        // SAFETY: caller keeps the context current and this program bound.
        unsafe { gl.uniform_4_f32(self.location(name), value[0], value[1], value[2], value[3]) }
    }

    pub(super) unsafe fn delete(&self, gl: &glow::Context) {
        // SAFETY: caller keeps the context that created the program current.
        unsafe { gl.delete_program(self.id) }
    }
}

unsafe fn link(
    gl: &glow::Context,
    vertex_source: &str,
    fragment_source: &str,
) -> Result<NativeProgram, String> {
    // SAFETY: delegated to the caller (current context on this thread).
    unsafe {
        let vertex = compile_shader(gl, glow::VERTEX_SHADER, vertex_source)?;
        let fragment = match compile_shader(gl, glow::FRAGMENT_SHADER, fragment_source) {
            Ok(shader) => shader,
            Err(error) => {
                gl.delete_shader(vertex);
                return Err(error);
            }
        };
        let program = match gl.create_program() {
            Ok(program) => program,
            Err(error) => {
                gl.delete_shader(vertex);
                gl.delete_shader(fragment);
                return Err(format!("OpenGL program allocation: {error}"));
            }
        };
        gl.attach_shader(program, vertex);
        gl.attach_shader(program, fragment);
        gl.link_program(program);
        let linked = gl.get_program_link_status(program);
        let log = if linked {
            String::new()
        } else {
            gl.get_program_info_log(program)
        };
        gl.detach_shader(program, vertex);
        gl.detach_shader(program, fragment);
        gl.delete_shader(vertex);
        gl.delete_shader(fragment);
        if !linked {
            gl.delete_program(program);
            return Err(format!("OpenGL shader link: {log}"));
        }
        Ok(program)
    }
}

unsafe fn compile_shader(
    gl: &glow::Context,
    stage: u32,
    source: &str,
) -> Result<glow::NativeShader, String> {
    // SAFETY: stage is a GL shader constant and the context is current on this thread.
    unsafe {
        let shader = gl
            .create_shader(stage)
            .map_err(|error| format!("OpenGL shader allocation: {error}"))?;
        gl.shader_source(shader, source);
        gl.compile_shader(shader);
        if !gl.get_shader_compile_status(shader) {
            let log = gl.get_shader_info_log(shader);
            gl.delete_shader(shader);
            let kind = if stage == glow::VERTEX_SHADER {
                "vertex"
            } else {
                "fragment"
            };
            return Err(format!("OpenGL {kind} shader compilation: {log}"));
        }
        Ok(shader)
    }
}
