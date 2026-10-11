//! Overlays de componente (seleção/preselection de vértices, arestas e faces)
//! e o workplane. Buffers próprios: trocar seleção nunca reenvia a malha.

use glow::HasContext;

use super::GlRenderer;
use super::buffers::{BAND_FLOATS, float_attributes, push_band};
use crate::frame::{OverlayFaceVertex, OverlayLine, OverlayPoint, RenderFrameInput};
use crate::overlays::workplane_geometry;

/// Cantos do quad de um ponto (dois triângulos).
const POINT_CORNERS: [[f32; 2]; 6] = [
    [-1.0, -1.0],
    [1.0, -1.0],
    [1.0, 1.0],
    [-1.0, -1.0],
    [1.0, 1.0],
    [-1.0, 1.0],
];
const POINT_FLOATS: usize = 10;
const FACE_FLOATS: usize = 7;

pub(super) fn face_data(faces: &[OverlayFaceVertex]) -> Vec<f32> {
    let mut data = Vec::with_capacity(faces.len() * FACE_FLOATS);
    for vertex in faces {
        data.extend_from_slice(&vertex.position);
        data.extend_from_slice(&vertex.color);
    }
    data
}

pub(super) fn line_data(lines: &[OverlayLine]) -> Vec<f32> {
    let mut data = Vec::with_capacity(lines.len() * 6 * BAND_FLOATS);
    for line in lines {
        push_band(&mut data, line.a, line.b, line.color, line.width_px);
    }
    data
}

pub(super) fn point_data(points: &[OverlayPoint]) -> Vec<f32> {
    let mut data = Vec::with_capacity(points.len() * 6 * POINT_FLOATS);
    for point in points {
        for corner in POINT_CORNERS {
            data.extend_from_slice(&point.position);
            data.extend_from_slice(&point.color);
            data.extend_from_slice(&[corner[0], corner[1], point.radius_px]);
        }
    }
    data
}

impl GlRenderer {
    /// Estado dos overlays: blend, sem escrita de profundidade, depth test
    /// (ou através da geometria em X-Ray).
    unsafe fn overlay_state(&self, xray: bool) {
        let gl = &self.gl;
        gl.enable(glow::BLEND);
        super::buffers::alpha_blend(&gl);
        gl.depth_mask(false);
        if xray {
            gl.disable(glow::DEPTH_TEST);
        } else {
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LEQUAL);
        }
    }

    unsafe fn restore_after_overlay(&self) {
        let gl = &self.gl;
        gl.disable(glow::BLEND);
        gl.enable(glow::DEPTH_TEST);
        gl.depth_func(glow::LESS);
        gl.depth_mask(true);
    }

    /// Passe 7: preenchimentos, arestas e pontos da seleção do objeto ativo.
    pub(super) unsafe fn draw_components(&mut self, frame: &RenderFrameInput<'_>) {
        puffin::profile_function!();
        let gl = std::sync::Arc::clone(&self.gl);
        let components = frame.components;
        let revision = components.revision;
        if self.component_faces.is_current(revision) {
            self.stats.skipped_uploads += 1;
        } else {
            let faces = face_data(&components.faces);
            let lines = line_data(&components.lines);
            let points = point_data(&components.points);
            let creations = &mut self.stats.buffer_creations;
            self.component_faces.upload(
                &gl,
                glow::ARRAY_BUFFER,
                &faces,
                components.faces.len() as i32,
                Some(revision),
                creations,
            );
            self.component_lines.upload(
                &gl,
                glow::ARRAY_BUFFER,
                &lines,
                (lines.len() / BAND_FLOATS) as i32,
                Some(revision),
                creations,
            );
            self.component_points.upload(
                &gl,
                glow::ARRAY_BUFFER,
                &points,
                (points.len() / POINT_FLOATS) as i32,
                Some(revision),
                creations,
            );
            self.stats.component_uploads += 1;
        }
        if components.is_empty() {
            return;
        }
        self.overlay_state(frame.settings.xray);
        self.draw_faces_buffer(frame, false);
        self.draw_lines_buffer(frame, false);
        if self.component_points.count > 0 && self.component_points.bind(&gl, glow::ARRAY_BUFFER) {
            let program = &self.programs.point;
            program.bind(&gl);
            program.mat4(&gl, "uViewProj", &frame.camera.view_proj);
            program.vec2(&gl, "uViewportSize", Self::viewport_size(frame));
            program.f32(&gl, "uPxScale", frame.settings.pixel_ratio);
            float_attributes(&gl, &[3, 4, 3]);
            gl.draw_arrays(glow::TRIANGLES, 0, self.component_points.count);
        }
        self.restore_after_overlay();
    }

    unsafe fn draw_faces_buffer(&self, frame: &RenderFrameInput<'_>, workplane: bool) {
        let buffer = if workplane {
            &self.workplane_faces
        } else {
            &self.component_faces
        };
        if buffer.count == 0 || !buffer.bind(&self.gl, glow::ARRAY_BUFFER) {
            return;
        }
        let program = &self.programs.flat;
        program.bind(&self.gl);
        program.mat4(&self.gl, "uViewProj", &frame.camera.view_proj);
        float_attributes(&self.gl, &[3, 4]);
        self.gl.draw_arrays(glow::TRIANGLES, 0, buffer.count);
    }

    unsafe fn draw_lines_buffer(&self, frame: &RenderFrameInput<'_>, workplane: bool) {
        let buffer = if workplane {
            &self.workplane_lines
        } else {
            &self.component_lines
        };
        if buffer.count == 0 || !buffer.bind(&self.gl, glow::ARRAY_BUFFER) {
            return;
        }
        let program = &self.programs.wide_line;
        program.bind(&self.gl);
        program.mat4(&self.gl, "uViewProj", &frame.camera.view_proj);
        program.mat4(&self.gl, "uModel", &crate::frame::IDENTITY);
        program.vec2(&self.gl, "uViewportSize", Self::viewport_size(frame));
        // Larguras já em px lógicos.
        program.f32(&self.gl, "uWidthScale", frame.settings.pixel_ratio);
        program.f32(&self.gl, "uDepthBias", 0.0002);
        float_attributes(&self.gl, &[3, 3, 4, 3]);
        self.gl.draw_arrays(glow::TRIANGLES, 0, buffer.count);
    }

    /// Passe 8 (workplane): recorte translúcido com grade, tamanho estável em
    /// tela (depende da câmera, então é refeito por frame — poucos vértices).
    pub(super) unsafe fn draw_workplane(&mut self, frame: &RenderFrameInput<'_>) {
        let Some(plane) = frame.workplane else {
            return;
        };
        let gl = std::sync::Arc::clone(&self.gl);
        let (faces, lines) = workplane_geometry(plane, &frame.camera);
        let face_floats = face_data(&faces);
        let line_floats = line_data(&lines);
        let creations = &mut self.stats.buffer_creations;
        self.workplane_faces.upload(
            &gl,
            glow::ARRAY_BUFFER,
            &face_floats,
            faces.len() as i32,
            None,
            creations,
        );
        self.workplane_lines.upload(
            &gl,
            glow::ARRAY_BUFFER,
            &line_floats,
            (line_floats.len() / BAND_FLOATS) as i32,
            None,
            creations,
        );
        self.overlay_state(frame.settings.xray);
        self.draw_faces_buffer(frame, true);
        self.draw_lines_buffer(frame, true);
        self.restore_after_overlay();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_expand_to_quads_with_their_radius() {
        let data = point_data(&[OverlayPoint {
            position: [1.0, 2.0, 3.0],
            color: [1.0; 4],
            radius_px: 5.0,
        }]);
        assert_eq!(data.len(), 6 * POINT_FLOATS);
        assert!(
            data.chunks(POINT_FLOATS)
                .all(|v| v[9] == 5.0 && v[0] == 1.0)
        );
    }

    #[test]
    fn faces_keep_position_and_rgba() {
        let data = face_data(&[OverlayFaceVertex {
            position: [1.0, 2.0, 3.0],
            color: [0.1, 0.2, 0.3, 0.4],
        }]);
        assert_eq!(data, vec![1.0, 2.0, 3.0, 0.1, 0.2, 0.3, 0.4]);
    }
}
