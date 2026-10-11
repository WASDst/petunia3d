//! Contrato comum entre retângulo da UI, picking e os viewports das duas GPUs.

/// Retângulo no espaço de coordenadas lógicas de tela da UI.
/// Totalmente desacoplado de bibliotecas gráficas concretas (egui, Qt, Slint, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LogicalRect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

impl LogicalRect {
    pub const fn from_min_max(min: [f32; 2], max: [f32; 2]) -> Self {
        Self { min, max }
    }

    pub fn left(&self) -> f32 {
        self.min[0]
    }

    pub fn top(&self) -> f32 {
        self.min[1]
    }

    pub fn right(&self) -> f32 {
        self.max[0]
    }

    pub fn bottom(&self) -> f32 {
        self.max[1]
    }

    pub fn width(&self) -> f32 {
        self.max[0] - self.min[0]
    }

    pub fn height(&self) -> f32 {
        self.max[1] - self.min[1]
    }

    pub fn center(&self) -> [f32; 2] {
        [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
        ]
    }

    pub fn is_finite(&self) -> bool {
        self.min[0].is_finite()
            && self.min[1].is_finite()
            && self.max[0].is_finite()
            && self.max[1].is_finite()
    }

    pub fn is_positive(&self) -> bool {
        self.width() > 0.0 && self.height() > 0.0
    }

    pub fn contains(&self, p: [f32; 2]) -> bool {
        p[0] >= self.min[0] && p[0] <= self.max[0] && p[1] >= self.min[1] && p[1] <= self.max[1]
    }

    /// Converte coordenada de tela (pixels lógicos) para Normalized Device Coordinates (-1.0 a 1.0).
    pub fn screen_to_ndc(&self, p: [f32; 2]) -> [f32; 2] {
        let w = self.width().max(1.0);
        let h = self.height().max(1.0);
        [
            ((p[0] - self.left()) / w) * 2.0 - 1.0,
            1.0 - ((p[1] - self.top()) / h) * 2.0,
        ]
    }

    /// Converte Normalized Device Coordinates (-1.0 a 1.0) para coordenadas de tela (pixels lógicos).
    pub fn ndc_to_screen(&self, ndc: [f32; 2]) -> [f32; 2] {
        [
            self.center()[0] + ndc[0] * self.width() * 0.5,
            self.center()[1] - ndc[1] * self.height() * 0.5,
        ]
    }

    /// Projeta ponto 3D de mundo para coordenada 2D de tela dentro do retângulo lógico.
    pub fn project_point(
        &self,
        camera: &crate::camera::Camera,
        point: [f32; 3],
    ) -> Option<[f32; 2]> {
        let p3 = glam::Vec3::from(point);
        if camera.proj == crate::camera::Projection::Perspective {
            let fwd = camera.forward();
            let to_p = (p3 - camera.eye()).normalize_or_zero();
            if fwd.dot(to_p) <= 0.0 {
                return None;
            }
        }
        let ndc = camera.project_ndc(p3);
        if ndc.x.abs() > 2.0 || ndc.y.abs() > 2.0 {
            return None;
        }
        Some(self.ndc_to_screen([ndc.x, ndc.y]))
    }

    /// Dispara raio 3D (origem, direção) a partir de uma coordenada de tela.
    pub fn ray(
        &self,
        camera: &crate::camera::Camera,
        screen_pos: [f32; 2],
    ) -> (glam::Vec3, glam::Vec3) {
        let ndc = self.screen_to_ndc(screen_pos);
        camera.ray(ndc[0], ndc[1])
    }
}

/// Desprojeta a posição do cursor da tela para o espaço 3D, atingindo a superfície da malha
/// ou recaindo no plano horizontal do cursor 3D.
pub fn unproject_to_surface_or_cursor_plane(
    camera: &crate::camera::Camera,
    mesh: Option<&petunia_mesh::Mesh>,
    screen_pos: [f32; 2],
    viewport: LogicalRect,
    cursor_3d: [f32; 3],
    pixels_per_point: f32,
) -> [f32; 3] {
    let ndc = viewport.screen_to_ndc(screen_pos);

    // 1. Tenta atingir a superfície da malha
    if let Some(m) = mesh {
        let vp_pixels = glam::Vec2::new(viewport.width(), viewport.height()) * pixels_per_point;
        if let Some(hit) = crate::picking::pick_mesh(
            m,
            camera,
            vp_pixels,
            glam::Vec2::new(ndc[0], ndc[1]),
            crate::selection::SelectionDomain::Face,
            false,
        ) {
            let p = hit.position;
            return [p.x, p.y, p.z];
        }
    }

    // 2. Fallback: interseção com o plano horizontal do cursor 3D
    let (ray_origin, ray_dir) = camera.ray(ndc[0], ndc[1]);
    let plane_y = cursor_3d[1];
    if ray_dir.y.abs() > 1e-4 {
        let t = (plane_y - ray_origin.y) / ray_dir.y;
        if t > 0.0 {
            let hit = ray_origin + ray_dir * t;
            return [hit.x, hit.y, hit.z];
        }
    }

    let fallback = ray_origin + ray_dir * 5.0;
    [fallback.x, fallback.y, fallback.z]
}

/// Encontra ponto 3D com snapping magnético ao vértice mais próximo da malha na tela (raio em px),
/// ou recai no plano horizontal do cursor 3D.
pub fn unproject_cursor_or_vertex_snap(
    camera: &crate::camera::Camera,
    mesh: Option<&petunia_mesh::Mesh>,
    screen_pos: [f32; 2],
    viewport: LogicalRect,
    cursor_3d: [f32; 3],
    snap_radius_px: f32,
) -> [f32; 3] {
    // 1. Tenta snapping magnético para o vértice mais próximo na tela
    if let Some(m) = mesh {
        let max_dist_sq = snap_radius_px * snap_radius_px;
        let mut best_dist_sq = max_dist_sq;
        let mut best_pos = None;

        for v in &m.verts {
            if let Some(sp) = viewport.project_point(camera, v.pos) {
                let dx = sp[0] - screen_pos[0];
                let dy = sp[1] - screen_pos[1];
                let d_sq = dx * dx + dy * dy;
                if d_sq < best_dist_sq {
                    best_dist_sq = d_sq;
                    best_pos = Some(v.pos);
                }
            }
        }

        if let Some(pos) = best_pos {
            return pos;
        }
    }

    // 2. Fallback para o plano do cursor 3D
    let ndc = viewport.screen_to_ndc(screen_pos);
    let (ray_origin, ray_dir) = camera.ray(ndc[0], ndc[1]);
    let plane_y = cursor_3d[1];
    if ray_dir.y.abs() > 1e-4 {
        let t = (plane_y - ray_origin.y) / ray_dir.y;
        if t > 0.0 {
            let hit = ray_origin + ray_dir * t;
            return [hit.x, hit.y, hit.z];
        }
    }

    let fallback = ray_origin + ray_dir * 5.0;
    [fallback.x, fallback.y, fallback.z]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalViewport {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl PhysicalViewport {
    pub fn from_logical(
        rect: Option<LogicalRect>,
        pixels_per_point: f32,
        width: u32,
        height: u32,
    ) -> Option<Self> {
        if width == 0 || height == 0 || !pixels_per_point.is_finite() || pixels_per_point <= 0.0 {
            return None;
        }
        let Some(rect) = rect else {
            return Some(Self {
                x: 0,
                y: 0,
                width,
                height,
            });
        };
        if !rect.is_finite() || !rect.is_positive() {
            return None;
        }
        let x = (rect.left() * pixels_per_point)
            .round()
            .clamp(0.0, width as f32) as u32;
        let y = (rect.top() * pixels_per_point)
            .round()
            .clamp(0.0, height as f32) as u32;
        let right = (rect.right() * pixels_per_point)
            .round()
            .clamp(0.0, width as f32) as u32;
        let bottom = (rect.bottom() * pixels_per_point)
            .round()
            .clamp(0.0, height as f32) as u32;
        (right > x && bottom > y).then_some(Self {
            x,
            y,
            width: right.saturating_sub(x),
            height: bottom.saturating_sub(y),
        })
    }

    /// OpenGL começa no canto inferior esquerdo; egui e wgpu usam o superior.
    pub fn gl_y(self, framebuffer_height: u32) -> u32 {
        framebuffer_height.saturating_sub(self.y + self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panels_and_hidpi_map_to_same_pixels_for_both_backends() {
        let rect = LogicalRect::from_min_max([60.0, 24.0], [440.0, 710.0]);
        let viewport = PhysicalViewport::from_logical(Some(rect), 2.0, 1356, 1478).unwrap();
        assert_eq!(
            viewport,
            PhysicalViewport {
                x: 120,
                y: 48,
                width: 760,
                height: 1372
            }
        );
        assert_eq!(viewport.gl_y(1478), 58);
        let center = [rect.center()[0] * 2.0, rect.center()[1] * 2.0];
        assert_eq!(center[0], viewport.x as f32 + viewport.width as f32 * 0.5);
        assert_eq!(center[1], viewport.y as f32 + viewport.height as f32 * 0.5);
    }

    #[test]
    fn resize_clamps_gpu_rect_and_rejects_empty_viewport() {
        let rect = LogicalRect::from_min_max([60.0, 24.0], [440.0, 710.0]);
        assert_eq!(
            PhysicalViewport::from_logical(Some(rect), 1.0, 300, 400),
            Some(PhysicalViewport {
                x: 60,
                y: 24,
                width: 240,
                height: 376
            })
        );
        assert!(PhysicalViewport::from_logical(Some(rect), 1.0, 40, 400).is_none());
        assert!(PhysicalViewport::from_logical(Some(rect), f32::NAN, 400, 400).is_none());
    }

    #[test]
    fn projected_corners_and_interior_agree_with_egui_at_fractional_dpi() {
        let rect = LogicalRect::from_min_max([73.25, 31.75], [615.5, 469.25]);
        // Viewport deslocado e assimétrico detecta inversão Y escondida quando
        // a cena ocupa a janela toda. Tolera apenas arredondamento subpixel.
        for scale in [1.0, 1.5, 2.0] {
            let viewport = PhysicalViewport::from_logical(Some(rect), scale, 1600, 1200).unwrap();
            for (nx, ny) in [
                (-1.0, -1.0),
                (-1.0, 1.0),
                (1.0, -1.0),
                (1.0, 1.0),
                (0.0, 0.0),
                (0.6, -0.4),
            ] {
                let expected_x = (rect.center()[0] + nx * rect.width() * 0.5) * scale;
                let expected_y = (rect.center()[1] - ny * rect.height() * 0.5) * scale;
                let gpu_x = viewport.x as f32 + (nx + 1.0) * viewport.width as f32 * 0.5;
                let wgpu_y = viewport.y as f32 + (1.0 - ny) * viewport.height as f32 * 0.5;
                let gl_y = viewport.gl_y(1200) as f32 + (ny + 1.0) * viewport.height as f32 * 0.5;
                assert!((gpu_x - expected_x).abs() <= 0.5);
                assert!((wgpu_y - expected_y).abs() <= 0.5);
                assert!((1200.0 - gl_y - expected_y).abs() <= 0.5);
            }
        }
    }
}
