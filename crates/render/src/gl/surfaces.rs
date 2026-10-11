//! Superfícies: um VBO/IBO por objeto, reenviado só quando a revisão muda; a
//! mesma geometria alimenta a máscara do contorno e o pré-passe de AO.

use glam::{Mat3, Mat4};
use glow::{HasContext, NativeTexture};
use uuid::Uuid;

use super::buffers::float_attributes;
use super::program::Program;
use super::{GlRenderer, ObjectSlot};
use crate::Shading;
use crate::frame::{IDENTITY, RenderFrameInput, RenderMesh, RenderObject};

/// Floats por vértice: posição(3) + normal(3) + cor(3) + UV(2).
const STRIDE: usize = 11;

fn interleave(mesh: &RenderMesh) -> Vec<f32> {
    let mut data = Vec::with_capacity(mesh.vertices.len() * STRIDE);
    for vertex in &mesh.vertices {
        data.extend_from_slice(&vertex.position);
        data.extend_from_slice(&vertex.normal);
        data.extend_from_slice(&vertex.color[..3]);
        data.extend_from_slice(&vertex.texture_coordinates);
    }
    data
}

/// Inversa-transposta da parte linear do modelo (normais sob escala não uniforme).
fn normal_matrix(transform: &[f32; 16]) -> [f32; 9] {
    if *transform == IDENTITY {
        return [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    }
    Mat3::from_mat4(Mat4::from_cols_array(transform))
        .inverse()
        .transpose()
        .to_cols_array()
}

impl GlRenderer {
    /// Poda objetos que saíram do frame e reenvia superfícies/arestas cujas
    /// revisões mudaram. Câmera, seleção e matriz rígida nunca reenviam.
    pub(super) unsafe fn sync_objects(&mut self, frame: &RenderFrameInput<'_>) {
        puffin::profile_function!();
        let gl = std::sync::Arc::clone(&self.gl);
        let live: std::collections::HashSet<Uuid> = frame
            .objects
            .iter()
            .map(|object| object.object_id)
            .collect();
        self.objects.retain(|id, slot| {
            let keep = live.contains(id);
            if !keep {
                slot.vertices.delete(&gl);
                slot.indices.delete(&gl);
                slot.edges.delete(&gl);
            }
            keep
        });
        for object in frame.objects {
            let slot = self.objects.entry(object.object_id).or_default();
            if slot.vertices.is_current(object.surface_revision) {
                self.stats.skipped_uploads += 1;
            } else {
                let data = interleave(&object.mesh);
                let ok = slot.vertices.upload(
                    &gl,
                    glow::ARRAY_BUFFER,
                    &data,
                    object.mesh.vertices.len() as i32,
                    Some(object.surface_revision),
                    &mut self.stats.buffer_creations,
                ) && slot.indices.upload(
                    &gl,
                    glow::ELEMENT_ARRAY_BUFFER,
                    &object.mesh.indices,
                    object.mesh.indices.len() as i32,
                    Some(object.surface_revision),
                    &mut self.stats.buffer_creations,
                );
                if ok {
                    self.stats.surface_uploads += 1;
                }
            }
            if slot.edges.is_current(object.edge_revision) {
                self.stats.skipped_uploads += 1;
            } else {
                let data = super::edges::edge_bands(object);
                if slot.edges.upload(
                    &gl,
                    glow::ARRAY_BUFFER,
                    &data,
                    (data.len() / super::buffers::BAND_FLOATS) as i32,
                    Some(object.edge_revision),
                    &mut self.stats.buffer_creations,
                ) {
                    self.stats.edge_uploads += 1;
                }
            }
        }
    }

    /// Liga VBO/IBO do objeto com o layout completo; `None` se não há o que desenhar.
    unsafe fn bind_surface(&self, object: &RenderObject, position_only: bool) -> Option<i32> {
        let slot: &ObjectSlot = self.objects.get(&object.object_id)?;
        if slot.indices.count == 0
            || !slot.vertices.bind(&self.gl, glow::ARRAY_BUFFER)
            || !slot.indices.bind(&self.gl, glow::ELEMENT_ARRAY_BUFFER)
        {
            return None;
        }
        if position_only {
            // Mesmo stride de 11 floats; só a posição é lida.
            self.gl.enable_vertex_attrib_array(0);
            self.gl
                .vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, (STRIDE * 4) as i32, 0);
            for index in 1..4 {
                self.gl.disable_vertex_attrib_array(index);
            }
        } else {
            float_attributes(&self.gl, &[3, 3, 3, 2]);
        }
        Some(slot.indices.count)
    }

    /// Passes 4–5: superfícies opacas, ou translúcidas sem depth write em X-Ray.
    pub(super) unsafe fn draw_surfaces(
        &mut self,
        frame: &RenderFrameInput<'_>,
        ambient_occlusion: Option<NativeTexture>,
    ) {
        puffin::profile_function!();
        let settings = &frame.settings;
        if settings.shading == Shading::Wireframe {
            return; // wireframe sai só nas arestas
        }
        let gl = std::sync::Arc::clone(&self.gl);
        let program = &self.programs.surface;
        program.bind(&gl);
        gl.enable(glow::DEPTH_TEST);
        if settings.xray {
            gl.enable(glow::BLEND);
            super::buffers::alpha_blend(&gl);
            gl.depth_mask(false);
            gl.depth_func(glow::LEQUAL);
        } else {
            gl.disable(glow::BLEND);
            gl.depth_mask(true);
            gl.depth_func(glow::LESS);
        }
        let camera = &frame.camera;
        program.mat4(&gl, "uViewProj", &camera.view_proj);
        program.f32(
            &gl,
            "uOpacity",
            if settings.xray {
                settings.xray_opacity.clamp(0.1, 0.9)
            } else {
                1.0
            },
        );
        program.vec3(&gl, "uLightDir", settings.light.direction);
        program.f32(&gl, "uAmbient", settings.light.ambient);
        program.f32(&gl, "uDiffuse", settings.light.diffuse);
        program.bool(
            &gl,
            "uMatcap",
            settings.matcap && settings.shading == Shading::Solid,
        );
        program.vec3(&gl, "uViewRight", camera.right);
        program.vec3(&gl, "uViewUp", camera.up);
        program.bool(&gl, "uFaceOrientation", settings.show_face_orientation);
        program.bool(&gl, "uUvChecker", settings.show_uv_checker);
        program.i32(&gl, "uTexture", 0);
        program.i32(&gl, "uAoTexture", 1);
        let viewport = frame.viewport;
        program.vec4(
            &gl,
            "uViewportRect",
            [
                viewport.x as f32,
                viewport.gl_y(frame.framebuffer_height) as f32,
                viewport.width.max(1) as f32,
                viewport.height.max(1) as f32,
            ],
        );
        program.bool(&gl, "uAmbientOcclusion", ambient_occlusion.is_some());
        gl.active_texture(glow::TEXTURE1);
        gl.bind_texture(
            glow::TEXTURE_2D,
            Some(ambient_occlusion.unwrap_or(self.white)),
        );
        gl.active_texture(glow::TEXTURE0);
        for object in frame.objects {
            let texture = object
                .texture
                .and_then(|id| self.object_textures.get(&id))
                .map(|slot| slot.texture);
            let Some(count) = self.bind_surface(object, false) else {
                continue;
            };
            let program = &self.programs.surface;
            program.bool(&gl, "uTextured", texture.is_some());
            gl.bind_texture(glow::TEXTURE_2D, Some(texture.unwrap_or(self.white)));
            program.mat4(&gl, "uModel", &object.transform);
            program.mat3(&gl, "uNormalMatrix", &normal_matrix(&object.transform));
            gl.draw_elements(glow::TRIANGLES, count, glow::UNSIGNED_INT, 0);
        }
        gl.disable(glow::BLEND);
        gl.depth_mask(true);
        gl.depth_func(glow::LESS);
        gl.active_texture(glow::TEXTURE1);
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, None);
    }

    /// Desenha as superfícies de `filter` só com posição, no programa ligado
    /// (máscara do contorno ou pré-passe de profundidade).
    pub(super) unsafe fn draw_positions(
        &self,
        frame: &RenderFrameInput<'_>,
        program: &Program,
        mut each: impl FnMut(&RenderObject) -> bool,
    ) -> bool {
        let mut drew = false;
        for object in frame.objects {
            if !each(object) {
                continue;
            }
            let Some(count) = self.bind_surface(object, true) else {
                continue;
            };
            program.mat4(&self.gl, "uModel", &object.transform);
            self.gl
                .draw_elements(glow::TRIANGLES, count, glow::UNSIGNED_INT, 0);
            drew = true;
        }
        drew
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::RenderVertex;

    fn vertex(x: f32) -> RenderVertex {
        RenderVertex {
            position: [x, 0.0, 0.0],
            normal: [0.0, 0.0, 1.0],
            color: [0.1, 0.2, 0.3, 1.0],
            texture_coordinates: [0.5, 0.25],
        }
    }

    #[test]
    fn interleave_matches_the_declared_stride() {
        let mesh = RenderMesh {
            vertices: vec![vertex(0.0), vertex(1.0)],
            indices: vec![0, 1, 0],
        };
        let data = interleave(&mesh);
        assert_eq!(data.len(), 2 * STRIDE);
        assert_eq!(&data[9..11], &[0.5, 0.25]);
        assert_eq!(&data[6..9], &[0.1, 0.2, 0.3]);
    }

    #[test]
    fn normal_matrix_undoes_non_uniform_scale() {
        let transform = Mat4::from_scale(glam::Vec3::new(2.0, 1.0, 1.0)).to_cols_array();
        let matrix = Mat3::from_cols_array(&normal_matrix(&transform));
        let normal = matrix * glam::Vec3::X;
        assert!((normal.x - 0.5).abs() < 1e-6);
        assert_eq!(normal_matrix(&IDENTITY), Mat3::IDENTITY.to_cols_array());
    }
}
