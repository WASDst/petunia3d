//! Detecção de capacidades no startup e verificação de erros em debug.
//! Mensagens técnicas vão para o log, nunca para o usuário final.

use glow::HasContext;

use crate::GpuCaps;

/// Lê versão, GLSL, vendor, renderer e limites do contexto corrente.
///
/// # Safety
/// O contexto GL precisa estar corrente na thread de renderização.
pub unsafe fn detect_caps(gl: &glow::Context) -> GpuCaps {
    // SAFETY: delegated to the caller.
    unsafe {
        let version = gl.version();
        GpuCaps {
            gl_version: gl.get_parameter_string(glow::VERSION),
            glsl_version: gl.get_parameter_string(glow::SHADING_LANGUAGE_VERSION),
            vendor: gl.get_parameter_string(glow::VENDOR),
            renderer: gl.get_parameter_string(glow::RENDERER),
            max_texture_size: gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE),
            max_samples: gl.get_parameter_i32(glow::MAX_SAMPLES).max(1),
            major: version.major,
            minor: version.minor,
            embedded: version.is_embedded,
        }
    }
}

/// Em builds de debug, registra erros GL pendentes (com limite por frame).
pub(super) unsafe fn check_errors(gl: &glow::Context, label: &str) {
    if !cfg!(debug_assertions) {
        return;
    }
    // SAFETY: caller keeps the context current.
    unsafe {
        for _ in 0..8 {
            let error = gl.get_error();
            if error == glow::NO_ERROR {
                return;
            }
            eprintln!("petunia3d: OpenGL error {error:#x} after {label}");
        }
    }
}
