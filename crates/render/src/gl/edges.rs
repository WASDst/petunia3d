//! Arestas de topologia/feição em largura constante de pixels (faixas
//! expandidas no vertex shader; nunca `glLineWidth`).

use glow::HasContext;

use super::GlRenderer;
use super::buffers::{BAND_CORNERS, float_attributes};
use crate::Shading;
use crate::frame::{RenderFrameInput, RenderObject};

/// Profundidade puxada para a câmera (clip em [0, 1]): as arestas não brigam
/// com as faces coplanares.
pub(super) const EDGE_DEPTH_BIAS: f32 = 0.0001;

/// Floats por vértice da faixa de aresta: A, B, cor, canto e as duas normais.
pub(super) const EDGE_FLOATS: usize = 19;

/// Faixas das arestas do objeto: largura = multiplicador da largura base; as
/// normais das faces vizinhas permitem descartar as arestas de trás.
pub(super) fn edge_bands(object: &RenderObject) -> Vec<f32> {
    let mut data = Vec::with_capacity(object.edges.len() * 6 * EDGE_FLOATS);
    for edge in &object.edges {
        let [r, g, b] = edge.color;
        for (end, side) in BAND_CORNERS {
            data.extend_from_slice(&edge.a);
            data.extend_from_slice(&edge.b);
            data.extend_from_slice(&[r, g, b, 1.0]);
            data.extend_from_slice(&[end, side, edge.width]);
            data.extend_from_slice(&edge.normals[0]);
            data.extend_from_slice(&edge.normals[1]);
        }
    }
    data
}

impl GlRenderer {
    /// Passe 6. Com X-Ray as arestas atravessam a geometria (sem depth test).
    pub(super) unsafe fn draw_edges(&mut self, frame: &RenderFrameInput<'_>) {
        puffin::profile_function!();
        let gl = &self.gl;
        let program = &self.programs.edge_line;
        program.bind(gl);
        program.mat4(gl, "uViewProj", &frame.camera.view_proj);
        program.vec3(gl, "uEye", frame.camera.eye);
        program.vec3(gl, "uViewDir", frame.camera.forward);
        program.f32(
            gl,
            "uOrtho",
            if frame.camera.orthographic { 1.0 } else { 0.0 },
        );
        // Com Raio-X — e no Wireframe, que é Raio-X por definição — as arestas
        // de trás são justamente o que se quer ver.
        let see_through =
            frame.settings.xray || matches!(frame.settings.shading, Shading::Wireframe);
        program.f32(gl, "uCullBack", if see_through { 0.0 } else { 1.0 });
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
            float_attributes(gl, &[3, 3, 4, 3, 3, 3]);
            program.mat4(gl, "uModel", &object.transform);
            program.mat3(gl, "uNormalModel", &normal_matrix(&object.transform));
            gl.draw_arrays(glow::TRIANGLES, 0, slot.edges.count);
        }
        gl.enable(glow::DEPTH_TEST);
        gl.depth_func(glow::LESS);
        gl.depth_mask(true);
    }
}

/// Inversa transposta da parte 3×3 de `model` (normais sob escala não uniforme).
fn normal_matrix(model: &[f32; 16]) -> [f32; 9] {
    let m = glam::Mat3::from_mat4(glam::Mat4::from_cols_array(model));
    let inverse = if m.determinant().abs() > 1.0e-12 {
        m.inverse()
    } else {
        glam::Mat3::IDENTITY
    };
    inverse.transpose().to_cols_array()
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
                normals: [[0.0; 3]; 2],
                a: [0.0; 3],
                b: [1.0, 0.0, 0.0],
                color: [0.1, 0.2, 0.3],
                width: 0.67,
            }],
            transform: IDENTITY,
        };
        let data = edge_bands(&object);
        assert_eq!(data.len(), 6 * EDGE_FLOATS);
        assert_eq!(&data[6..10], &[0.1, 0.2, 0.3, 1.0]);
        assert_eq!(data[12], 0.67);
    }

    #[test]
    fn band_vertices_carry_both_neighbour_face_normals() {
        let object = RenderObject {
            object_id: uuid::Uuid::nil(),
            surface_revision: 1,
            edge_revision: 1,
            mesh: RenderMesh::default(),
            texture: None,
            edges: vec![RenderEdge {
                normals: [[0.0, 0.0, 1.0], [1.0, 0.0, 0.0]],
                a: [0.0; 3],
                b: [1.0, 0.0, 0.0],
                color: [0.0; 3],
                width: 1.0,
            }],
            transform: IDENTITY,
        };
        let data = edge_bands(&object);
        // Cada vértice da faixa repete as normais nos últimos 6 floats.
        let vertices = data.len() / EDGE_FLOATS;
        assert_eq!(vertices, 6);
        for vertex in 0..vertices {
            let base = vertex * EDGE_FLOATS;
            assert_eq!(&data[base + 13..base + 16], &[0.0, 0.0, 1.0]);
            assert_eq!(&data[base + 16..base + 19], &[1.0, 0.0, 0.0]);
        }
    }
}
