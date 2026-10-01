//! CPU fallback with the same selection, shading and depth contracts as WGPU.

use glam::{Mat4, Vec3, Vec4};
use petunia_core::{Camera, HoverTarget, SelectionDomain, Workspace};
use petunia_project::{Canvas, PoseOverride, Project, mesh_to_draw};
use petunia_render::{Shading, scene};
use std::sync::Arc;

use crate::{PetuniaViewport, ViewportRenderState};

#[derive(Clone, Copy)]
struct ScreenVertex {
    x: f32,
    y: f32,
    z: f32,
    inv_w: f32,
}

struct Surface<'a> {
    colors: [[f32; 3]; 3],
    uv: [[f32; 2]; 3],
    texture: Option<&'a Canvas>,
    tint: Option<([f32; 3], f32)>,
    opacity: f32,
    depth_write: bool,
}

pub struct Software3dViewport {
    pub width: u32,
    pub height: u32,
    pub workspace: Workspace,
    pub selection_domain: SelectionDomain,
    pub depth_buffer: Vec<f32>,
    pub color_buffer: Vec<u8>,
    /// Malhas deformadas por skin (preview de pose); o documento fica em repouso.
    pose: Option<Arc<PoseOverride>>,
}

impl Software3dViewport {
    pub fn new(width: u32, height: u32) -> Self {
        let (width, height) = (width.max(1), height.max(1));
        Self {
            width,
            height,
            workspace: Workspace::Model,
            selection_domain: SelectionDomain::Object,
            depth_buffer: vec![1.0; (width * height) as usize],
            color_buffer: vec![0; (width * height * 4) as usize],
            pose: None,
        }
    }

    fn project_clip(&self, p: Vec4) -> Option<ScreenVertex> {
        if !p.is_finite() || p.w <= 1.0e-5 || p.z < 0.0 || p.z > p.w {
            return None;
        }
        let inv_w = p.w.recip();
        Some(ScreenVertex {
            x: (p.x * inv_w + 1.0) * 0.5 * self.width as f32,
            y: (1.0 - p.y * inv_w) * 0.5 * self.height as f32,
            z: p.z * inv_w,
            inv_w,
        })
    }

    fn project_point(&self, p: Vec3, vp: &Mat4) -> Option<ScreenVertex> {
        self.project_clip(*vp * p.extend(1.0))
    }

    fn blend_pixel(&mut self, index: usize, color: [f32; 3], alpha: f32) {
        let offset = index * 4;
        let alpha = alpha.clamp(0.0, 1.0);
        for (channel, value) in color.iter().enumerate() {
            let old = self.color_buffer[offset + channel] as f32 / 255.0;
            self.color_buffer[offset + channel] =
                ((old * (1.0 - alpha) + value * alpha).clamp(0.0, 1.0) * 255.0).round() as u8;
        }
        self.color_buffer[offset + 3] = 255;
    }

    fn line(
        &mut self,
        a: ScreenVertex,
        b: ScreenVertex,
        color: [f32; 3],
        alpha: f32,
        through: bool,
    ) {
        // Clip to the visible rectangle before stepping. A near-plane edge
        // must not make the CPU walk millions of off-screen pixels.
        let delta = [b.x - a.x, b.y - a.y];
        let mut lo: f32 = 0.0;
        let mut hi: f32 = 1.0;
        for (p, q) in [
            (-delta[0], a.x),
            (delta[0], self.width as f32 - 1.0 - a.x),
            (-delta[1], a.y),
            (delta[1], self.height as f32 - 1.0 - a.y),
        ] {
            if p.abs() < 1.0e-6 {
                if q < 0.0 {
                    return;
                }
            } else if p < 0.0 {
                lo = lo.max(q / p);
            } else {
                hi = hi.min(q / p);
            }
        }
        if lo > hi {
            return;
        }
        let steps = ((delta[0].abs().max(delta[1].abs()) * (hi - lo)).ceil() as u32).max(1);
        for step in 0..=steps {
            let t = lo + (hi - lo) * step as f32 / steps as f32;
            let x = (a.x + delta[0] * t).round() as i32;
            let y = (a.y + delta[1] * t).round() as i32;
            if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
                continue;
            }
            let index = y as usize * self.width as usize + x as usize;
            let z = a.z + (b.z - a.z) * t;
            if through || z <= self.depth_buffer[index] + 2.0e-4 {
                self.blend_pixel(index, color, alpha);
            }
        }
    }

    fn world_line(&mut self, a: Vec3, b: Vec3, vp: &Mat4, color: [f32; 3], through: bool) {
        let mut a = *vp * a.extend(1.0);
        let mut b = *vp * b.extend(1.0);
        // Homogeneous clipping at the near plane preserves grid lines that
        // cross the camera, instead of dropping the entire segment.
        if a.z < 0.0 && b.z < 0.0 {
            return;
        }
        if a.z < 0.0 {
            a = a.lerp(b, (-a.z / (b.z - a.z)).clamp(0.0, 1.0));
        }
        if b.z < 0.0 {
            b = b.lerp(a, (-b.z / (a.z - b.z)).clamp(0.0, 1.0));
        }
        if let (Some(a), Some(b)) = (self.project_clip(a), self.project_clip(b)) {
            self.line(a, b, color, 1.0, through);
        }
    }

    fn world_line_width(
        &mut self,
        a: Vec3,
        b: Vec3,
        vp: &Mat4,
        color: [f32; 3],
        through: bool,
        width_px: f32,
    ) {
        let (Some(a), Some(b)) = (self.project_point(a, vp), self.project_point(b, vp)) else {
            return;
        };
        let delta = [b.x - a.x, b.y - a.y];
        let length = delta[0].hypot(delta[1]).max(1.0);
        let normal = [-delta[1] / length, delta[0] / length];
        let width = width_px.round().clamp(1.0, 6.0) as i32;
        for index in 0..width {
            let offset = index as f32 - (width - 1) as f32 * 0.5;
            let mut a = a;
            let mut b = b;
            a.x += normal[0] * offset;
            a.y += normal[1] * offset;
            b.x += normal[0] * offset;
            b.y += normal[1] * offset;
            self.line(a, b, color, 1.0, through);
        }
    }

    fn triangle(&mut self, p: [ScreenVertex; 3], surface: &Surface<'_>) {
        let [a, b, c] = p;
        let area = (b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y);
        if area.abs() < 1.0e-5 {
            return;
        }
        let min_x = a.x.min(b.x).min(c.x).floor().max(0.0) as u32;
        let max_x = a.x.max(b.x).max(c.x).ceil().min(self.width as f32 - 1.0) as u32;
        let min_y = a.y.min(b.y).min(c.y).floor().max(0.0) as u32;
        let max_y = a.y.max(b.y).max(c.y).ceil().min(self.height as f32 - 1.0) as u32;
        if min_x >= self.width || min_y >= self.height {
            return;
        }
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                let w0 = ((b.x - fx) * (c.y - fy) - (c.x - fx) * (b.y - fy)) / area;
                let w1 = ((c.x - fx) * (a.y - fy) - (a.x - fx) * (c.y - fy)) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }
                let z = w0 * a.z + w1 * b.z + w2 * c.z;
                let index = (y * self.width + x) as usize;
                if z >= self.depth_buffer[index] || !(0.0..=1.0).contains(&z) {
                    continue;
                }
                let weights = [w0 * a.inv_w, w1 * b.inv_w, w2 * c.inv_w];
                let sum = weights.iter().sum::<f32>();
                if sum <= 0.0 {
                    continue;
                }
                let weights = weights.map(|w| w / sum);
                let mut color = [0.0; 3];
                for (channel, value) in color.iter_mut().enumerate() {
                    *value = (0..3)
                        .map(|i| surface.colors[i][channel] * weights[i])
                        .sum();
                }
                if let Some(texture) = surface.texture {
                    let uv: [f32; 2] = std::array::from_fn(|axis| {
                        (0..3).map(|i| surface.uv[i][axis] * weights[i]).sum()
                    });
                    let tx = (uv[0].rem_euclid(1.0) * texture.w as f32) as u32;
                    let ty = ((1.0 - uv[1].rem_euclid(1.0)) * texture.h as f32) as u32;
                    if let Some(pixel) = texture.get(tx.min(texture.w - 1), ty.min(texture.h - 1)) {
                        for channel in 0..3 {
                            color[channel] *= pixel[channel] as f32 / 255.0;
                        }
                    }
                }
                if let Some((tint, strength)) = surface.tint {
                    for channel in 0..3 {
                        color[channel] =
                            color[channel] * (1.0 - strength) + tint[channel] * strength;
                    }
                }
                self.blend_pixel(index, color, surface.opacity);
                if surface.depth_write {
                    self.depth_buffer[index] = z;
                }
            }
        }
    }

    fn marker(&mut self, p: ScreenVertex, radius: i32, color: [f32; 3], through: bool) {
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy > radius * radius {
                    continue;
                }
                let (x, y) = (p.x.round() as i32 + dx, p.y.round() as i32 + dy);
                if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
                    continue;
                }
                let index = y as usize * self.width as usize + x as usize;
                if through || p.z <= self.depth_buffer[index] + 2.0e-4 {
                    self.blend_pixel(index, color, 1.0);
                }
            }
        }
    }
}

impl PetuniaViewport for Software3dViewport {
    fn resize(&mut self, width: u32, height: u32) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.depth_buffer
            .resize((self.width * self.height) as usize, 1.0);
        self.color_buffer
            .resize((self.width * self.height * 4) as usize, 0);
    }

    fn update(&mut self, _dt_seconds: f32) {}
    fn set_workspace(&mut self, workspace: Workspace) {
        self.workspace = workspace;
    }
    fn set_selection_domain(&mut self, domain: SelectionDomain) {
        self.selection_domain = domain;
    }
    fn draws_component_guides(&self) -> bool {
        true
    }

    fn set_pose_override(&mut self, pose: Option<Arc<PoseOverride>>) {
        self.pose = pose;
    }

    fn render_frame(
        &mut self,
        project: &Project,
        _refs: &[petunia_core::ReferenceImage],
        camera: &Camera,
        state: ViewportRenderState,
    ) -> Option<slint::Image> {
        self.depth_buffer.fill(1.0);
        for pixel in self.color_buffer.as_chunks_mut::<4>().0 {
            pixel.copy_from_slice(&[24, 25, 28, 255]);
        }
        let vp = camera.view_proj();
        let through = state.xray || state.shading == Shading::Wireframe;
        if state.show_grid {
            let step = 10.0f32.powf((camera.visible_height().max(0.001) / 20.0).log10().floor());
            for (a, b, color) in scene::grid_lines_custom(step * 50.0, step, 0.8, false, 30.0) {
                self.world_line(Vec3::from_array(a), Vec3::from_array(b), &vp, color, true);
            }
        }
        let pose = self.pose.clone();
        let meshes: Vec<_> = project
            .assets
            .iter()
            .enumerate()
            .filter(|(_, a)| a.visible)
            .map(|(index, asset)| (index, asset, mesh_to_draw(pose.as_deref(), asset)))
            .collect();
        let scene_light = if state.shading.uses_scene_light() {
            project.active_light()
        } else {
            None
        };
        let light_dir = scene_light.map_or(
            if state.studio_light_follows_camera {
                Vec3::from_array(scene::studio_light_for_camera(camera))
            } else {
                Vec3::from_array(scene::LIGHT_DIR).normalize()
            },
            |light| Vec3::from_array(light.normalized_direction()),
        );
        let ambient = if scene_light.is_some() {
            scene::LIGHT_AMBIENT * 0.35
        } else {
            scene::LIGHT_AMBIENT
        };
        let diffuse =
            scene::LIGHT_DIFFUSE * scene_light.map_or(1.0, |light| light.intensity.clamp(0.0, 8.0));
        let light_color = scene_light.map_or([1.0; 3], |light| light.color);

        if state.shading.fills_faces() {
            // Sort transparency globally, so overlapping objects do not depend
            // on their order in the Parts list.
            let mut faces: Vec<_> = meshes
                .iter()
                .enumerate()
                .flat_map(|(mi, (_, _, mesh))| {
                    mesh.faces.iter().enumerate().map(move |(fi, _)| (mi, fi))
                })
                .collect();
            if state.xray {
                faces.sort_by(|&(ma, fa), &(mb, fb)| {
                    let depth = |mi: usize, fi: usize| {
                        let mesh = &meshes[mi].2;
                        let face = &mesh.faces[fi];
                        let center = face
                            .verts
                            .iter()
                            .map(|&v| mesh.verts[v as usize].vec())
                            .sum::<Vec3>()
                            / face.verts.len().max(1) as f32;
                        (center - camera.eye()).length_squared()
                    };
                    depth(mb, fb).total_cmp(&depth(ma, fa))
                });
            }
            for (mi, fi) in faces {
                let (asset_index, asset, mesh) = &meshes[mi];
                let face = &mesh.faces[fi];
                let material = asset.material(project);
                let texture = if state.shading.samples_material() {
                    material
                        .and_then(|m| m.albedo_texture.as_ref())
                        .or(asset.texture.as_ref())
                } else {
                    None
                };
                let base = if state.show_face_orientation {
                    let normal = mesh.face_normal(fi);
                    let to_cam = (camera.eye() - mesh.face_centroid(fi)).normalize_or_zero();
                    if normal.dot(to_cam) >= 0.0 {
                        [0.2, 0.45, 0.95]
                    } else {
                        [0.95, 0.2, 0.2]
                    }
                } else if state.show_uv_checker {
                    [0.85, 0.85, 0.85]
                } else if state.shading.samples_material() {
                    material.map_or(asset.base_color, |m| {
                        [m.base_color[0], m.base_color[1], m.base_color[2]]
                    })
                } else {
                    [0.72, 0.74, 0.78]
                };
                let is_boolean_operand = state.boolean_operand == Some(asset.id);
                let tint = if is_boolean_operand {
                    Some(([0.71, 0.55, 1.0], 0.35))
                } else if *asset_index == project.active
                    && state.selection_domain == SelectionDomain::Face
                {
                    if face.selected {
                        Some((state.selection_rgb.map(|value| value as f32 / 255.0), 0.32))
                    } else if state.hover == HoverTarget::Face(fi) {
                        Some(([0.49, 0.86, 1.0], 0.2))
                    } else {
                        None
                    }
                } else {
                    None
                };
                let lambert = mesh.face_normal(fi).dot(light_dir).max(0.0);
                for tri in mesh.face_triangle_corners(fi) {
                    let positions = tri.map(|i| mesh.verts[face.verts[i] as usize].vec());
                    let [Some(a), Some(b), Some(c)] = positions.map(|p| self.project_point(p, &vp))
                    else {
                        continue;
                    };
                    let colors = tri.map(|i| {
                        let uv_coord = face.uv.get(i).copied().unwrap_or_default();
                        let (checker_r, checker_g, checker_b) = if state.show_uv_checker {
                            let u_cell = (uv_coord[0] * 16.0).floor() as i32;
                            let v_cell = (uv_coord[1] * 16.0).floor() as i32;
                            let is_even = (u_cell + v_cell).rem_euclid(2) == 0;
                            let u_macro = u_cell.rem_euclid(8) == 0;
                            let v_macro = v_cell.rem_euclid(8) == 0;
                            if is_even {
                                if u_macro {
                                    (1.0, 0.65, 0.55)
                                } else if v_macro {
                                    (0.55, 0.80, 1.0)
                                } else {
                                    (1.0, 1.0, 1.0)
                                }
                            } else {
                                if u_macro {
                                    (0.45, 0.22, 0.20)
                                } else if v_macro {
                                    (0.20, 0.30, 0.45)
                                } else {
                                    (0.3, 0.3, 0.3)
                                }
                            }
                        } else {
                            (1.0, 1.0, 1.0)
                        };
                        std::array::from_fn(|channel| {
                            let vertex = &mesh.verts[face.verts[i] as usize];
                            let paint = if state.shading.samples_material() && material.is_none() {
                                vertex.color[channel] / [0.75, 0.75, 0.78][channel]
                            } else {
                                1.0
                            };
                            let emission = if state.shading.samples_material() {
                                material.map_or(0.0, |m| {
                                    m.emission_color[channel] * m.emission_strength
                                })
                            } else {
                                0.0
                            };
                            let ch_mult = match channel {
                                0 => checker_r,
                                1 => checker_g,
                                2 => checker_b,
                                _ => 1.0,
                            };
                            base[channel]
                                * ch_mult
                                * paint
                                * (ambient + diffuse * lambert * light_color[channel])
                                + emission
                        })
                    });
                    self.triangle(
                        [a, b, c],
                        &Surface {
                            colors,
                            uv: tri.map(|i| face.uv.get(i).copied().unwrap_or_default()),
                            texture: if state.show_uv_checker { None } else { texture },
                            tint,
                            opacity: if state.xray {
                                state.xray_opacity.clamp(0.1, 0.9)
                            } else {
                                1.0
                            },
                            depth_write: !state.xray,
                        },
                    );
                }
            }
        }

        // All opaque surfaces must be in the depth buffer before components.
        for (asset_index, asset, mesh) in &meshes {
            let active = *asset_index == project.active;
            let is_boolean_operand = state.boolean_operand == Some(asset.id);
            if state.show_wireframe_overlay
                || state.show_triangulation
                || state.shading == Shading::Wireframe
                || active && state.selection_domain == SelectionDomain::Edge
                || is_boolean_operand
            {
                for (a, b) in mesh.edges_unique() {
                    let selected = active
                        && state.selection_domain == SelectionDomain::Edge
                        && mesh.selected_edges.contains(&(a, b));
                    let is_seam = active
                        && (mesh.uv_seams.contains(&(a, b)) || mesh.uv_seams.contains(&(b, a)));
                    let hover = active && state.hover == HoverTarget::Edge(a, b);
                    let color = if is_boolean_operand {
                        [0.71, 0.55, 1.0]
                    } else if selected {
                        state.selection_rgb.map(|value| value as f32 / 255.0)
                    } else if is_seam {
                        [0.96, 0.48, 0.12]
                    } else if hover {
                        [0.49, 0.86, 1.0]
                    } else {
                        [0.32, 0.35, 0.40]
                    };
                    if selected || hover || is_seam || is_boolean_operand {
                        self.world_line_width(
                            mesh.verts[a as usize].vec(),
                            mesh.verts[b as usize].vec(),
                            &vp,
                            color,
                            through,
                            if is_boolean_operand {
                                1.8
                            } else if hover || is_seam {
                                state.selection_thickness * 1.35
                            } else {
                                state.selection_thickness
                            },
                        );
                    } else {
                        self.world_line(
                            mesh.verts[a as usize].vec(),
                            mesh.verts[b as usize].vec(),
                            &vp,
                            color,
                            through,
                        );
                    }
                }
                if state.show_triangulation {
                    for (a, b) in mesh.triangulation_wireframe() {
                        self.world_line(
                            Vec3::from_array(a),
                            Vec3::from_array(b),
                            &vp,
                            [0.23, 0.26, 0.30],
                            through,
                        );
                    }
                }
            }
            if active && state.selection_domain == SelectionDomain::Vertex {
                for (index, vertex) in mesh.verts.iter().enumerate() {
                    let Some(p) = self.project_point(vertex.vec(), &vp) else {
                        continue;
                    };
                    let (radius, color) = if state.hover == HoverTarget::Vertex(index as u32) {
                        (
                            (state.selection_thickness * 2.5).min(7.0).round() as i32,
                            [0.49, 0.86, 1.0],
                        )
                    } else if vertex.selected {
                        (
                            (state.selection_thickness * 2.0).min(6.0).round() as i32,
                            state.selection_rgb.map(|value| value as f32 / 255.0),
                        )
                    } else {
                        (
                            (state.selection_thickness * 1.25).min(5.0).round() as i32,
                            [0.62, 0.66, 0.74],
                        )
                    };
                    self.marker(p, radius, color, through);
                }
            }
        }
        let mut pixels =
            slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(self.width, self.height);
        pixels.make_mut_bytes().copy_from_slice(&self.color_buffer);
        Some(slint::Image::from_rgba8(pixels))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn software_viewport_initializes_and_renders_frame() {
        let mut viewport = Software3dViewport::new(320, 240);
        let project = Project::default();
        let camera = Camera::default();

        let image = viewport.render_frame(&project, &[], &camera, ViewportRenderState::default());
        assert!(image.is_some());
        let img = image.expect("image");
        assert_eq!(img.size().width, 320);
        assert_eq!(img.size().height, 240);
    }

    #[test]
    fn software_pose_override_replaces_the_mesh_only_while_drawing() {
        let mut viewport = Software3dViewport::new(240, 180);
        let mut project = Project::default();
        project.add("Hero", petunia_core::Mesh::cube(2.0));
        let asset_id = project.assets.last().unwrap().id;
        let camera = Camera::default();
        let state = ViewportRenderState {
            show_grid: false,
            ..ViewportRenderState::default()
        };
        viewport.render_frame(&project, &[], &camera, state);
        let rest = viewport.color_buffer.clone();

        // A mesma malha, deslocada: o documento não muda, o desenho sim.
        let mut moved = project.assets.last().unwrap().mesh.clone();
        for v in &mut moved.verts {
            v.pos[0] += 1.5;
        }
        let mut pose = PoseOverride::new(1);
        pose.insert(asset_id, moved);
        viewport.set_pose_override(Some(Arc::new(pose)));
        viewport.render_frame(&project, &[], &camera, state);
        assert_ne!(viewport.color_buffer, rest, "a pose muda os pixels");
        assert_eq!(
            project.assets.last().unwrap().mesh.verts[0].pos,
            petunia_core::Mesh::cube(2.0).verts[0].pos,
            "o documento segue em repouso"
        );

        // Sem override, volta exatamente ao repouso.
        viewport.set_pose_override(None);
        viewport.render_frame(&project, &[], &camera, state);
        assert_eq!(viewport.color_buffer, rest);
    }

    #[test]
    fn software_wireframe_overlay_toggle_changes_pixels() {
        // Caracterização: o toggle do overlay precisa ter efeito visível.
        let mut viewport = Software3dViewport::new(320, 240);
        let mut project = Project::default();
        project.add("Cube", petunia_core::Mesh::cube(2.0));
        let camera = Camera::default();
        let base = ViewportRenderState {
            show_grid: false,
            ..ViewportRenderState::default()
        };
        viewport.render_frame(&project, &[], &camera, base);
        let with_overlay = viewport.color_buffer.clone();
        assert!(base.show_wireframe_overlay, "default tem overlay");

        let off = ViewportRenderState {
            show_wireframe_overlay: false,
            ..base
        };
        viewport.render_frame(&project, &[], &camera, off);
        assert_ne!(
            viewport.color_buffer, with_overlay,
            "desligar o overlay muda os pixels"
        );
    }

    #[test]
    fn software_viewport_resizes_correctly() {
        let mut viewport = Software3dViewport::new(100, 100);
        viewport.resize(200, 150);
        assert_eq!(viewport.width, 200);
        assert_eq!(viewport.height, 150);

        let project = Project::default();
        let camera = Camera::default();
        let image = viewport
            .render_frame(&project, &[], &camera, ViewportRenderState::default())
            .expect("frame");
        assert_eq!(image.size().width, 200);
        assert_eq!(image.size().height, 150);
    }

    #[test]
    fn software_viewport_renders_with_selection_domains() {
        let mut viewport = Software3dViewport::new(160, 120);
        let mut project = Project::default();
        if let Some(v) = project.active_mesh_mut().and_then(|m| m.verts.first_mut()) {
            v.selected = true;
        }
        let camera = Camera::default();

        viewport.set_selection_domain(SelectionDomain::Vertex);
        let frame_v = viewport.render_frame(&project, &[], &camera, ViewportRenderState::default());
        assert!(frame_v.is_some());

        viewport.set_selection_domain(SelectionDomain::Edge);
        let frame_e = viewport.render_frame(&project, &[], &camera, ViewportRenderState::default());
        assert!(frame_e.is_some());

        viewport.set_selection_domain(SelectionDomain::Face);
        let frame_f = viewport.render_frame(&project, &[], &camera, ViewportRenderState::default());
        assert!(frame_f.is_some());
    }

    #[test]
    fn software_selection_style_changes_pixels() {
        let mut viewport = Software3dViewport::new(320, 240);
        let mut project = Project::default();
        project.add("Cube", petunia_core::Mesh::cube(2.0));
        for vertex in &mut project.active_mesh_mut().unwrap().verts {
            vertex.selected = true;
        }
        let camera = Camera::default();
        let mut style = ViewportRenderState {
            selection_domain: SelectionDomain::Vertex,
            selection_rgb: [255, 64, 32],
            selection_thickness: 2.0,
            ..ViewportRenderState::default()
        };
        viewport.render_frame(&project, &[], &camera, style);
        let red = viewport.color_buffer.clone();
        style.selection_rgb = [32, 160, 255];
        viewport.render_frame(&project, &[], &camera, style);
        assert_ne!(viewport.color_buffer, red);
        let blue = viewport.color_buffer.clone();
        style.selection_thickness = 5.0;
        viewport.render_frame(&project, &[], &camera, style);
        assert_ne!(viewport.color_buffer, blue);
    }

    #[test]
    fn software_studio_light_follows_the_camera() {
        let mut project = Project::default();
        project.add("Cube", petunia_core::Mesh::cube(2.0));
        let luminance_from = |preset, follows| {
            let mut viewport = Software3dViewport::new(160, 120);
            let mut camera = Camera::default();
            camera.set_preset(preset);
            camera.target = Vec3::ZERO;
            let state = ViewportRenderState {
                show_grid: false,
                show_wireframe_overlay: false,
                studio_light_follows_camera: follows,
                ..ViewportRenderState::default()
            };
            viewport.render_frame(&project, &[], &camera, state);
            let offset = ((60 * 160 + 80) * 4) as usize;
            let pixel = &viewport.color_buffer[offset..offset + 3];
            0.2126 * f32::from(pixel[0])
                + 0.7152 * f32::from(pixel[1])
                + 0.0722 * f32::from(pixel[2])
        };
        let front = luminance_from(petunia_core::ViewPreset::Front, true);
        let back = luminance_from(petunia_core::ViewPreset::Back, true);
        assert!(
            (front - back).abs() < 6.0,
            "frente {front:.1} × trás {back:.1}"
        );
        let fixed_front = luminance_from(petunia_core::ViewPreset::Front, false);
        let fixed_back = luminance_from(petunia_core::ViewPreset::Back, false);
        assert!(
            fixed_front - fixed_back > 30.0,
            "{fixed_front:.1} × {fixed_back:.1}"
        );
    }

    #[test]
    fn software_xray_opacity_changes_visible_pixels() {
        let mut viewport = Software3dViewport::new(320, 240);
        let mut project = Project::default();
        project.add("Cube", petunia_core::Mesh::cube(2.0));
        let camera = Camera::default();
        let mut state = ViewportRenderState {
            show_grid: false,
            xray: true,
            xray_opacity: 0.15,
            ..ViewportRenderState::default()
        };
        viewport.render_frame(&project, &[], &camera, state);
        let transparent = viewport.color_buffer.clone();
        state.xray_opacity = 0.85;
        viewport.render_frame(&project, &[], &camera, state);
        assert_ne!(viewport.color_buffer, transparent);
        let mostly_opaque = viewport.color_buffer.clone();
        state.xray = false;
        viewport.render_frame(&project, &[], &camera, state);
        assert_ne!(viewport.color_buffer, mostly_opaque);
    }
}
