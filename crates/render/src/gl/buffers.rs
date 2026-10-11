//! Buffers de vértices persistentes por revisão e layout de atributos.

use glow::{HasContext, NativeBuffer};

/// Atributos de vértice que algum passe usa (os demais ficam desligados).
const MAX_ATTRIBUTES: u32 = 4;

/// VBO reaproveitado entre frames: só reenvia quando a revisão muda.
#[derive(Default)]
pub(super) struct StreamBuffer {
    buffer: Option<NativeBuffer>,
    /// Vértices (ou índices) enviados.
    pub(super) count: i32,
    revision: Option<u64>,
}

impl StreamBuffer {
    /// O conteúdo na GPU corresponde a `revision`.
    pub(super) fn is_current(&self, revision: u64) -> bool {
        self.buffer.is_some() && self.revision == Some(revision)
    }

    /// Envia `data` (`count` vértices) e marca a revisão; `None` = sempre reenviar.
    /// Fica ligado em `target` ao voltar. `false` se a GPU recusar o buffer.
    pub(super) unsafe fn upload<T: bytemuck::Pod>(
        &mut self,
        gl: &glow::Context,
        target: u32,
        data: &[T],
        count: i32,
        revision: Option<u64>,
        creations: &mut u64,
    ) -> bool {
        // SAFETY: caller keeps the context current on this thread.
        unsafe {
            if self.buffer.is_none() {
                match gl.create_buffer() {
                    Ok(buffer) => {
                        self.buffer = Some(buffer);
                        *creations += 1;
                    }
                    Err(error) => {
                        eprintln!("petunia3d: OpenGL buffer: {error}");
                        return false;
                    }
                }
            }
            gl.bind_buffer(target, self.buffer);
            gl.buffer_data_u8_slice(target, bytemuck::cast_slice(data), glow::DYNAMIC_DRAW);
        }
        self.count = count;
        self.revision = revision;
        true
    }

    pub(super) unsafe fn bind(&self, gl: &glow::Context, target: u32) -> bool {
        // SAFETY: caller keeps the context current on this thread.
        unsafe { gl.bind_buffer(target, self.buffer) };
        self.buffer.is_some()
    }

    pub(super) unsafe fn delete(&mut self, gl: &glow::Context) {
        if let Some(buffer) = self.buffer.take() {
            // SAFETY: caller keeps the context that created the buffer current.
            unsafe { gl.delete_buffer(buffer) };
        }
        self.count = 0;
        self.revision = None;
    }
}

/// Liga os atributos float `sizes` (componentes) intercalados no VBO atual e
/// desliga os demais (nenhum ponteiro velho fica ativo entre passes).
pub(super) unsafe fn float_attributes(gl: &glow::Context, sizes: &[i32]) {
    let stride: i32 = sizes.iter().sum::<i32>() * 4;
    let mut offset = 0;
    // SAFETY: caller keeps the context current with the matching VBO bound.
    unsafe {
        for (index, size) in sizes.iter().enumerate() {
            gl.enable_vertex_attrib_array(index as u32);
            gl.vertex_attrib_pointer_f32(index as u32, *size, glow::FLOAT, false, stride, offset);
            offset += size * 4;
        }
        for index in sizes.len() as u32..MAX_ATTRIBUTES {
            gl.disable_vertex_attrib_array(index);
        }
    }
}

/// Desliga todos os atributos (passes de tela cheia sem vértices).
pub(super) unsafe fn no_attributes(gl: &glow::Context) {
    // SAFETY: caller keeps the context current.
    unsafe {
        for index in 0..MAX_ATTRIBUTES {
            gl.disable_vertex_attrib_array(index);
        }
    }
}

/// Blend "over" para a cor; o alpha do destino continua 1. O alvo do viewport
/// é composto pelo Slint como textura: alpha < 1 deixaria o shell aparecer
/// através de X-Ray, overlays e gizmo.
pub(super) unsafe fn alpha_blend(gl: &glow::Context) {
    // SAFETY: caller keeps the context current.
    unsafe {
        gl.blend_func_separate(
            glow::SRC_ALPHA,
            glow::ONE_MINUS_SRC_ALPHA,
            glow::ONE,
            glow::ONE_MINUS_SRC_ALPHA,
        );
    }
}

/// Seis cantos (ponta, lado) da faixa de uma aresta larga: dois triângulos.
pub(super) const BAND_CORNERS: [(f32, f32); 6] = [
    (0.0, -1.0),
    (0.0, 1.0),
    (1.0, 1.0),
    (0.0, -1.0),
    (1.0, 1.0),
    (1.0, -1.0),
];

/// Floats por vértice de faixa: a(3) + b(3) + cor(4) + canto(3).
pub(super) const BAND_FLOATS: usize = 13;

/// Acrescenta a faixa do segmento `a`–`b` (seis vértices) a `out`.
pub(super) fn push_band(out: &mut Vec<f32>, a: [f32; 3], b: [f32; 3], color: [f32; 4], width: f32) {
    for (end, side) in BAND_CORNERS {
        out.extend_from_slice(&a);
        out.extend_from_slice(&b);
        out.extend_from_slice(&color);
        out.extend_from_slice(&[end, side, width]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_band_is_two_triangles_covering_both_ends_and_sides() {
        let mut data = Vec::new();
        push_band(&mut data, [0.0; 3], [1.0, 0.0, 0.0], [1.0; 4], 2.0);
        assert_eq!(data.len(), 6 * BAND_FLOATS);
        let corners: Vec<(f32, f32)> = data.chunks(BAND_FLOATS).map(|v| (v[10], v[11])).collect();
        assert_eq!(corners, BAND_CORNERS.to_vec());
        assert!(data.chunks(BAND_FLOATS).all(|v| v[12] == 2.0));
    }
}
