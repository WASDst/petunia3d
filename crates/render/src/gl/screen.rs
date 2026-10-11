//! Overlay de tela (gizmo de transformação e ferramentas): último passe, sem
//! depth test, em tamanho constante de pixels. O hit test continua na UI sobre
//! a mesma geometria lógica, nunca por leitura de framebuffer.

use glow::HasContext;

use super::GlRenderer;
use super::buffers::float_attributes;
use crate::frame::RenderFrameInput;
use crate::overlays::tessellate_screen_overlay;

impl GlRenderer {
    pub(super) unsafe fn draw_screen_overlay(&mut self, frame: &RenderFrameInput<'_>) {
        if frame.screen_overlay.is_empty() {
            return;
        }
        let gl = std::sync::Arc::clone(&self.gl);
        let vertices = tessellate_screen_overlay(frame.screen_overlay, frame.settings.pixel_ratio);
        if vertices.is_empty() {
            return;
        }
        let mut data = Vec::with_capacity(vertices.len() * 6);
        for vertex in &vertices {
            data.extend_from_slice(&vertex.position);
            data.extend_from_slice(&vertex.color);
        }
        if !self.screen.upload(
            &gl,
            glow::ARRAY_BUFFER,
            &data,
            vertices.len() as i32,
            None,
            &mut self.stats.buffer_creations,
        ) {
            return;
        }
        let program = &self.programs.screen;
        program.bind(&gl);
        program.vec2(&gl, "uViewportSize", Self::viewport_size(frame));
        float_attributes(&gl, &[2, 4]);
        gl.disable(glow::DEPTH_TEST);
        gl.depth_mask(false);
        gl.enable(glow::BLEND);
        super::buffers::alpha_blend(&gl);
        gl.draw_arrays(glow::TRIANGLES, 0, vertices.len() as i32);
        gl.disable(glow::BLEND);
        gl.enable(glow::DEPTH_TEST);
        gl.depth_mask(true);
        self.report.screen_overlay_vertices = vertices.len() as u32;
    }
}
