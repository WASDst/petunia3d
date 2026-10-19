//! Imagens de referência: um passe atrás da geometria (depth test) e outro
//! X-Ray por cima (sem depth test). Texturas sobem por revisão.

use glow::HasContext;

use super::GlRenderer;
use super::buffers::float_attributes;
use crate::frame::RenderFrameInput;
use crate::scene::QUAD_UVS_TOP_LEFT;

/// Dois triângulos (0,1,2 e 0,2,3) com posição+UV por vértice.
fn quad_vertices(quad: &[[f32; 3]; 4]) -> [f32; 30] {
    let mut corners = [0.0f32; 20];
    for (index, (position, uv)) in quad.iter().zip(QUAD_UVS_TOP_LEFT).enumerate() {
        corners[index * 5..index * 5 + 3].copy_from_slice(position);
        corners[index * 5 + 3..index * 5 + 5].copy_from_slice(&uv);
    }
    let corner = |index: usize| &corners[index * 5..index * 5 + 5];
    let mut out = [0.0f32; 30];
    for (slot, source) in [0usize, 1, 2, 0, 2, 3].into_iter().enumerate() {
        out[slot * 5..slot * 5 + 5].copy_from_slice(corner(source));
    }
    out
}

impl GlRenderer {
    pub(super) unsafe fn draw_references(&mut self, frame: &RenderFrameInput<'_>, xray_pass: bool) {
        puffin::profile_function!();
        if !frame.settings.show_overlays
            || !frame
                .references
                .iter()
                .any(|image| image.visible && image.xray == xray_pass)
        {
            return;
        }
        let gl = std::sync::Arc::clone(&self.gl);
        let mut data = Vec::new();
        let mut draws = Vec::new();
        for (index, image) in frame.references.iter().enumerate() {
            if !image.visible || image.xray != xray_pass {
                continue;
            }
            let Some(slot) = self.reference_textures.get(index) else {
                continue;
            };
            draws.push((slot.texture, image.opacity, (data.len() / 5) as i32));
            data.extend_from_slice(&quad_vertices(&image.quad));
        }
        if draws.is_empty()
            || !self.references.upload(
                &gl,
                glow::ARRAY_BUFFER,
                &data,
                (data.len() / 5) as i32,
                None,
                &mut self.stats.buffer_creations,
            )
        {
            return;
        }
        let program = &self.programs.reference;
        program.bind(&gl);
        program.mat4(&gl, "uViewProj", &frame.camera.view_proj);
        program.i32(&gl, "uTexture", 0);
        float_attributes(&gl, &[3, 2]);
        gl.enable(glow::BLEND);
        super::buffers::alpha_blend(&gl);
        gl.depth_mask(false);
        if xray_pass {
            gl.disable(glow::DEPTH_TEST);
        } else {
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LEQUAL);
        }
        gl.active_texture(glow::TEXTURE0);
        for (texture, opacity, first) in draws {
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            program.f32(&gl, "uOpacity", opacity);
            gl.draw_arrays(glow::TRIANGLES, first, 6);
        }
        gl.disable(glow::BLEND);
        gl.enable(glow::DEPTH_TEST);
        gl.depth_func(glow::LESS);
        gl.depth_mask(true);
        gl.bind_texture(glow::TEXTURE_2D, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quad_expands_to_two_triangles_sharing_the_diagonal() {
        let quad = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        let v = quad_vertices(&quad);
        assert_eq!(&v[0..3], &quad[0]);
        assert_eq!(&v[15..18], &quad[0]);
        assert_eq!(&v[10..13], &quad[2]);
        assert_eq!(&v[20..23], &quad[2]);
        assert_eq!(&v[25..28], &quad[3]);
    }
}
