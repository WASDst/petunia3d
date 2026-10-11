//! Arestas de topologia/feição em largura constante de pixels (faixas
//! expandidas no vertex shader; nunca `glLineWidth`).

use glow::HasContext;

use super::GlRenderer;
use super::buffers::{BAND_FLOATS, float_attributes, push_band};
use crate::frame::{RenderFrameInput, RenderObject};

/// Profundidade puxada para a câmera (clip em [0, 1]): as arestas não brigam
/// com as faces coplanares.
pub(super) const EDGE_DEPTH_BIAS: f32 = 0.0001;

/// Faixas das arestas do objeto: largura = multiplicador da largura base.
pub(super) fn edge_bands(object: &RenderObject) -> Vec<f32> {
    let mut data = Vec::with_capacity(object.edges.len() * 6 * BAND_FLOATS);
    for edge in &object.edges {
        let [r, g, b] = edge.color;
        push_band(&mut data, edge.a, edge.b, [r, g, b, 1.0], edge.width);
    }
    data
}

impl GlRenderer {
    /// Passe 6. Com X-Ray as arestas atravessam a geometria (sem depth test).
    pub(super) unsafe fn draw_edges(&mut self, frame: &RenderFrameInput<'_>) {
        puffin::profile_function!();
        let gl = &self.gl;
        let program = &self.programs.wide_line;
        program.bind(gl);
        program.mat4(gl, "uViewProj", &frame.camera.view_proj);
        program.vec2(gl, "uViewportSize", Self::viewport_size(frame));
        program.f32(
            gl,
            "uWidthScale",
            frame.settings.line_width_px.clamp(1.0, 8.0) * frame.settings.pixel_ratio,
        );
        program.f32(gl, "uDepthBias", EDGE_DEPTH_BIAS);
        gl.disable(glow::BLEND);
        gl.depth_mask(false);
        if frame.settings.xray {
            gl.disable(glow::DEPTH_TEST);
        } else {
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LEQUAL);
        }
        for object in frame.objects {
            let Some(slot) = self.objects.get(&object.object_id) else {
                continue;
            };
            if slot.edges.count == 0 || !slot.edges.bind(gl, glow::ARRAY_BUFFER) {
                continue;
            }
            float_attributes(gl, &[3, 3, 4, 3]);
            program.mat4(gl, "uModel", &object.transform);
            gl.draw_arrays(glow::TRIANGLES, 0, slot.edges.count);
        }
        gl.enable(glow::DEPTH_TEST);
        gl.depth_func(glow::LESS);
        gl.depth_mask(true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{IDENTITY, RenderEdge, RenderMesh};

    #[test]
    fn each_edge_becomes_six_band_vertices_with_its_width() {
        let object = RenderObject {
            object_id: uuid::Uuid::nil(),
            surface_revision: 1,
            edge_revision: 1,
            mesh: RenderMesh::default(),
            texture: None,
            edges: vec![RenderEdge {
                a: [0.0; 3],
                b: [1.0, 0.0, 0.0],
                color: [0.1, 0.2, 0.3],
                width: 0.67,
            }],
            transform: IDENTITY,
        };
        let data = edge_bands(&object);
        assert_eq!(data.len(), 6 * BAND_FLOATS);
        assert_eq!(&data[6..10], &[0.1, 0.2, 0.3, 1.0]);
        assert_eq!(data[12], 0.67);
    }
}
