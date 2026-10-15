//! Câmera orbital (perspectiva) + vistas ortográficas (spec §8.1).
//! Ortográfica é necessária para Draw Profile sobre referências.

use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Projection {
    #[default]
    Perspective,
    Ortho,
}

/// Navigation anchor is session state, independent from transform pivots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NavigationAnchor {
    #[default]
    ViewTarget,
    Cursor3D,
}

/// Vista nomeada (header do viewport).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewPreset {
    Persp,
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
    IsometricNE,
    IsometricNW,
    IsometricSE,
    IsometricSW,
    /// Projeção pseudo-isométrica clássica de games 2:1 (26,565°).
    PseudoIsometric2To1,
}

impl ViewPreset {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Persp => "Perspective",
            Self::Front => "Front",
            Self::Back => "Back",
            Self::Left => "Left",
            Self::Right => "Right",
            Self::Top => "Top",
            Self::Bottom => "Bottom",
            Self::IsometricNE => "Isometric NE",
            Self::IsometricNW => "Isometric NW",
            Self::IsometricSE => "Isometric SE",
            Self::IsometricSW => "Isometric SW",
            Self::PseudoIsometric2To1 => "Pseudo-Isometric 2:1",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Camera {
    pub target: Vec3,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub fov_y: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
    pub proj: Projection,
    /// meia-altura do frustum ortográfico.
    pub ortho_half_h: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            distance: 8.0,
            yaw: 0.7,
            pitch: 0.5,
            fov_y: 45.0_f32.to_radians(),
            aspect: 16.0 / 9.0,
            near: 0.05,
            far: 200.0,
            proj: Projection::Perspective,
            ortho_half_h: 4.0,
        }
    }
}

impl Camera {
    pub fn eye(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        self.target + Vec3::new(sy * cp, sp, cy * cp) * self.distance
    }

    pub fn forward(&self) -> Vec3 {
        (self.target - self.eye()).normalize_or_zero()
    }
    pub fn right(&self) -> Vec3 {
        let (sin, cos) = self.yaw.sin_cos();
        Vec3::new(cos, 0.0, -sin)
    }
    pub fn up(&self) -> Vec3 {
        self.right().cross(self.forward()).normalize_or_zero()
    }

    pub fn view(&self) -> Mat4 {
        let eye = self.eye();
        Mat4::look_at_rh(eye, self.target, self.up())
    }

    pub fn proj(&self) -> Mat4 {
        match self.proj {
            Projection::Perspective => {
                Mat4::perspective_rh(self.fov_y, self.aspect.max(0.01), self.near, self.far)
            }
            Projection::Ortho => {
                let h = self.ortho_half_h.max(0.05);
                let w = h * self.aspect.max(0.01);
                Mat4::orthographic_rh(-w, w, -h, h, self.near, self.far)
            }
        }
    }

    pub fn view_proj(&self) -> Mat4 {
        self.proj() * self.view()
    }

    /// Raio do cursor (ndc x/y em [-1,1]) para pick/posicionamento.
    /// Funciona em perspectiva e ortográfica.
    pub fn ray(&self, nx: f32, ny: f32) -> (Vec3, Vec3) {
        let fwd = self.forward();
        match self.proj {
            Projection::Perspective => {
                let tan = (self.fov_y * 0.5).tan();
                let dir =
                    (fwd + self.right() * nx * tan * self.aspect.max(0.01) + self.up() * ny * tan)
                        .normalize_or_zero();
                (self.eye(), dir)
            }
            Projection::Ortho => {
                let h = self.ortho_half_h.max(0.05);
                let origin =
                    self.eye() + self.right() * nx * h * self.aspect.max(0.01) + self.up() * ny * h;
                (origin, fwd)
            }
        }
    }

    /// Projeta ponto 3D p/ NDC (p/ overlay 2D do perfil).
    pub fn project_ndc(&self, p: Vec3) -> Vec3 {
        let v = self.view_proj() * p.extend(1.0);
        if v.w.abs() < 1e-9 {
            Vec3::ZERO
        } else {
            Vec3::new(v.x / v.w, v.y / v.w, v.z / v.w)
        }
    }

    pub fn set_preset(&mut self, p: ViewPreset) {
        self.set_projection(if p == ViewPreset::Persp {
            Projection::Perspective
        } else {
            Projection::Ortho
        });
        match p {
            ViewPreset::Persp => {
                self.yaw = 0.7;
                self.pitch = 0.5;
            }
            ViewPreset::Front => {
                self.yaw = 0.0;
                self.pitch = 0.0;
            }
            ViewPreset::Back => {
                self.yaw = std::f32::consts::PI;
                self.pitch = 0.0;
            }
            ViewPreset::Right => {
                self.yaw = std::f32::consts::FRAC_PI_2;
                self.pitch = 0.0;
            }
            ViewPreset::Left => {
                self.yaw = -std::f32::consts::FRAC_PI_2;
                self.pitch = 0.0;
            }
            ViewPreset::Top => {
                self.yaw = 0.0;
                self.pitch = std::f32::consts::FRAC_PI_2;
            }
            ViewPreset::Bottom => {
                self.yaw = 0.0;
                self.pitch = -std::f32::consts::FRAC_PI_2;
            }
            ViewPreset::IsometricNE => {
                self.yaw = std::f32::consts::FRAC_PI_4;
                self.pitch = 0.6154797; // arcsin(tan(30°))
            }
            ViewPreset::IsometricNW => {
                self.yaw = std::f32::consts::FRAC_PI_4 * 3.0;
                self.pitch = 0.6154797;
            }
            ViewPreset::IsometricSE => {
                self.yaw = -std::f32::consts::FRAC_PI_4;
                self.pitch = 0.6154797;
            }
            ViewPreset::IsometricSW => {
                self.yaw = -std::f32::consts::FRAC_PI_4 * 3.0;
                self.pitch = 0.6154797;
            }
            ViewPreset::PseudoIsometric2To1 => {
                self.yaw = std::f32::consts::FRAC_PI_4;
                self.pitch = 26.565_f32.to_radians();
            }
        }
    }

    /// Orbits in the current projection; orthographic views stay orthographic.
    pub fn orbit(&mut self, dx: f32, dy: f32) {
        if !dx.is_finite() || !dy.is_finite() {
            return;
        }
        self.yaw -= dx * 0.008;
        self.pitch = (self.pitch + dy * 0.008)
            .clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);
    }

    /// Rotate the existing eye and target around a world anchor, without a
    /// preliminary reframe. Changing the anchor alone never moves the camera.
    pub fn orbit_about(&mut self, anchor: Vec3, dx: f32, dy: f32) -> bool {
        if !anchor.is_finite() || !dx.is_finite() || !dy.is_finite() {
            return false;
        }
        let before = glam::Mat3::from_cols(self.right(), self.up(), -self.forward());
        let offset = self.target - anchor;
        let mut next = self.clone();
        next.orbit(dx, dy);
        let after = glam::Mat3::from_cols(next.right(), next.up(), -next.forward());
        let target = anchor + after * before.transpose() * offset;
        if !target.is_finite() {
            return false;
        }
        next.target = target;
        *self = next;
        true
    }

    /// Keep the world anchor at its projected position while changing zoom.
    pub fn zoom_about(&mut self, anchor: Vec3, delta: f32) -> bool {
        if !anchor.is_finite() || !delta.is_finite() {
            return false;
        }
        let before = self.visible_height();
        if !before.is_finite() || before <= 0.0 {
            return false;
        }
        let mut next = self.clone();
        next.zoom(delta);
        let ratio = next.visible_height() / before;
        next.target = anchor + (self.target - anchor) * ratio;
        if !next.target.is_finite() || !next.distance.is_finite() {
            return false;
        }
        *self = next;
        true
    }

    pub fn pan(&mut self, dx: f32, dy: f32) {
        let s = if self.proj == Projection::Ortho {
            self.ortho_half_h * 0.002
        } else {
            self.distance * 0.0016
        };
        let offset = (-self.right() * dx + self.up() * dy) * s;
        self.target += offset;
    }

    /// Height in world units at the focal plane, independent of projection.
    pub fn visible_height(&self) -> f32 {
        match self.proj {
            Projection::Ortho => self.ortho_half_h * 2.0,
            Projection::Perspective => self.distance * (self.fov_y * 0.5).tan() * 2.0,
        }
    }

    /// Changes framing without changing the viewing direction or focus point.
    /// Returns false for non-finite or non-positive input.
    pub fn set_visible_height(&mut self, height: f32) -> bool {
        if !height.is_finite() || height <= 0.0 {
            return false;
        }
        let half_height = height.clamp(0.1, 100_000.0) * 0.5;
        match self.proj {
            Projection::Ortho => self.ortho_half_h = half_height,
            Projection::Perspective => {
                self.distance = half_height / (self.fov_y * 0.5).tan();
                self.far = self.far.max(self.distance * 4.0 + self.near);
            }
        }
        true
    }

    /// Switches projection while preserving the image scale at the focal plane.
    pub fn set_projection(&mut self, projection: Projection) {
        if self.proj == projection {
            return;
        }
        let height = self.visible_height();
        self.proj = projection;
        self.set_visible_height(height);
    }

    pub fn toggle_projection(&mut self) {
        self.set_projection(match self.proj {
            Projection::Perspective => Projection::Ortho,
            Projection::Ortho => Projection::Perspective,
        });
    }

    /// Returns an axis-aligned or isometric view preset if current forward matches closely.
    pub fn view_preset(&self) -> Option<ViewPreset> {
        let forward = self.forward();
        let iso_ne_fwd = -Vec3::new(1.0, 1.0, 1.0).normalize();
        let iso_nw_fwd = -Vec3::new(1.0, 1.0, -1.0).normalize();
        let iso_se_fwd = -Vec3::new(-1.0, 1.0, 1.0).normalize();
        let iso_sw_fwd = -Vec3::new(-1.0, 1.0, -1.0).normalize();

        let pseudo_iso_pitch = 26.565_f32.to_radians();
        let (pseudo_sp, pseudo_cp) = pseudo_iso_pitch.sin_cos();
        let (pseudo_sy, pseudo_cy) = std::f32::consts::FRAC_PI_4.sin_cos();
        let pseudo_iso_fwd =
            -Vec3::new(pseudo_sy * pseudo_cp, pseudo_sp, pseudo_cy * pseudo_cp).normalize_or_zero();

        [
            (Vec3::NEG_Z, ViewPreset::Front),
            (Vec3::Z, ViewPreset::Back),
            (Vec3::NEG_X, ViewPreset::Right),
            (Vec3::X, ViewPreset::Left),
            (Vec3::NEG_Y, ViewPreset::Top),
            (Vec3::Y, ViewPreset::Bottom),
            (iso_ne_fwd, ViewPreset::IsometricNE),
            (iso_nw_fwd, ViewPreset::IsometricNW),
            (iso_se_fwd, ViewPreset::IsometricSE),
            (iso_sw_fwd, ViewPreset::IsometricSW),
            (pseudo_iso_fwd, ViewPreset::PseudoIsometric2To1),
        ]
        .into_iter()
        .find_map(|(direction, preset)| {
            (forward.distance_squared(direction) < 1e-4).then_some(preset)
        })
    }

    /// Identifica a vista nominal atual (ex: "Front Ortho", "Isometric NE", "User Persp")
    /// e retorna os ângulos yaw e pitch em graus no intervalo [-180, 180].
    pub fn nominal_view(&self) -> (&'static str, f32, f32) {
        let yaw_deg = (self.yaw.to_degrees() + 180.0).rem_euclid(360.0) - 180.0;
        let pitch_deg = self.pitch.to_degrees();

        if self.proj == Projection::Perspective {
            return ("User Persp", yaw_deg, pitch_deg);
        }

        const ANGULAR_EPS: f32 = 1.0;

        if pitch_deg.abs() < ANGULAR_EPS {
            if yaw_deg.abs() < ANGULAR_EPS {
                ("Front Ortho", yaw_deg, pitch_deg)
            } else if (yaw_deg.abs() - 180.0).abs() < ANGULAR_EPS {
                ("Back Ortho", yaw_deg, pitch_deg)
            } else if (yaw_deg - 90.0).abs() < ANGULAR_EPS {
                ("Right Ortho", yaw_deg, pitch_deg)
            } else if (yaw_deg + 90.0).abs() < ANGULAR_EPS {
                ("Left Ortho", yaw_deg, pitch_deg)
            } else {
                ("User Ortho", yaw_deg, pitch_deg)
            }
        } else if (pitch_deg - 90.0).abs() < ANGULAR_EPS {
            ("Top Ortho", yaw_deg, pitch_deg)
        } else if (pitch_deg + 90.0).abs() < ANGULAR_EPS {
            ("Bottom Ortho", yaw_deg, pitch_deg)
        } else if (pitch_deg - 35.264).abs() < 1.5 {
            if (yaw_deg - 45.0).abs() < 1.5 {
                ("Isometric NE", yaw_deg, pitch_deg)
            } else if (yaw_deg - 135.0).abs() < 1.5 {
                ("Isometric NW", yaw_deg, pitch_deg)
            } else if (yaw_deg + 45.0).abs() < 1.5 {
                ("Isometric SE", yaw_deg, pitch_deg)
            } else if (yaw_deg + 135.0).abs() < 1.5 {
                ("Isometric SW", yaw_deg, pitch_deg)
            } else {
                ("User Ortho", yaw_deg, pitch_deg)
            }
        } else if (pitch_deg - 26.565).abs() < 1.5 && (yaw_deg - 45.0).abs() < 1.5 {
            ("Pseudo-Isometric 2:1", yaw_deg, pitch_deg)
        } else {
            ("User Ortho", yaw_deg, pitch_deg)
        }
    }

    pub fn opposite_view(&mut self) {
        self.yaw = (self.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU);
        self.pitch = -self.pitch;
    }

    pub fn zoom(&mut self, delta: f32) {
        if !delta.is_finite() {
            return;
        }
        let factor = (-delta * 0.0015).clamp(-10.0, 10.0).exp();
        self.set_visible_height(self.visible_height() * factor);
    }

    /// Resets focus and zoom while retaining the current view/projection and viewport.
    pub fn reset(&mut self) {
        let (proj, yaw, pitch, aspect) = (self.proj, self.yaw, self.pitch, self.aspect);
        *self = Self {
            proj,
            yaw,
            pitch,
            aspect,
            ..Self::default()
        };
    }

    /// Enquadra a seleção (frame selection) — aproxima target/distância.
    pub fn frame(&mut self, center: Vec3, radius: f32) {
        if !center.is_finite() || !radius.is_finite() || radius < 0.0 {
            return;
        }
        self.target = center;
        let r = radius.max(0.05);
        if self.proj == Projection::Ortho {
            self.ortho_half_h = (r * 1.4 / self.aspect.clamp(0.01, 1.0)).max(0.05);
            self.distance = self.distance.max(r * 2.0 + self.near);
        } else {
            let half_vertical = self.fov_y * 0.5;
            let half_horizontal = (half_vertical.tan() * self.aspect.max(0.01)).atan();
            let half_angle = half_vertical.min(half_horizontal);
            self.distance = (r * 1.1 / half_angle.sin().max(0.01)).max(self.near * 2.0);
        }
        self.far = self.far.max(self.distance + r * 2.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_toggle_preserves_focal_plane_scale_and_roundtrips() {
        for distance in [0.3, 8.0, 80.0] {
            let mut camera = Camera {
                distance,
                target: Vec3::new(2.0, 3.0, 4.0),
                ..Camera::default()
            };
            let point = camera.target + camera.right() * 0.2 + camera.up() * 0.1;
            let before = camera.project_ndc(point).truncate();
            camera.toggle_projection();
            assert_eq!(camera.proj, Projection::Ortho);
            assert!(camera.project_ndc(point).truncate().distance(before) < 1e-5);
            camera.toggle_projection();
            assert!((camera.distance - distance).abs() < 1e-5);
            assert!(camera.project_ndc(point).truncate().distance(before) < 1e-5);
        }
    }

    #[test]
    fn orthographic_orbit_preserves_projection_scale_and_parallel_rays() {
        let mut camera = Camera::default();
        camera.set_preset(ViewPreset::Front);
        let height = camera.visible_height();
        camera.orbit(25.0, -12.0);
        assert_eq!(camera.proj, Projection::Ortho);
        assert_eq!(camera.visible_height(), height);
        let (a, dir_a) = camera.ray(-0.4, 0.2);
        let (b, dir_b) = camera.ray(0.7, -0.3);
        assert!(a.distance(b) > 0.1);
        assert!(dir_a.distance(dir_b) < 1e-6);
    }

    #[test]
    fn pole_navigation_keeps_a_continuous_screen_basis() {
        let mut camera = Camera::default();
        camera.set_preset(ViewPreset::Top);
        let right = camera.right();
        let up = camera.up();
        camera.orbit(0.0, -4.0);
        assert!(camera.right().dot(right) > 0.99);
        assert!(camera.up().dot(up) > 0.99);
        assert!(camera.view_proj().is_finite());
    }

    #[test]
    fn scale_input_rejects_invalid_values_and_reset_retains_view() {
        let mut camera = Camera {
            aspect: 0.4,
            ..Camera::default()
        };
        camera.set_preset(ViewPreset::Top);
        camera.set_visible_height(12.0);
        for value in [f32::NAN, f32::INFINITY, -1.0, 0.0] {
            assert!(!camera.set_visible_height(value));
            assert_eq!(camera.visible_height(), 12.0);
        }
        camera.target = Vec3::splat(9.0);
        camera.reset();
        assert_eq!(camera.target, Vec3::ZERO);
        assert_eq!(camera.aspect, 0.4);
        assert_eq!(camera.proj, Projection::Ortho);
        assert_eq!(camera.view_preset(), Some(ViewPreset::Top));
    }

    #[test]
    fn large_frame_updates_depth_range_in_both_projections() {
        for projection in [Projection::Perspective, Projection::Ortho] {
            let mut camera = Camera {
                proj: projection,
                ..Camera::default()
            };
            let center = Vec3::new(500.0, 100.0, -30.0);
            camera.frame(center, 500.0);
            for direction in [
                Vec3::X,
                Vec3::NEG_X,
                Vec3::Y,
                Vec3::NEG_Y,
                Vec3::Z,
                Vec3::NEG_Z,
            ] {
                let projected = camera.project_ndc(center + direction * 500.0);
                assert!(projected.is_finite());
                assert!((0.0..=1.0).contains(&projected.z));
            }
        }
    }

    #[test]
    fn rays_match_projected_cursor_in_all_presets() {
        for preset in [
            ViewPreset::Persp,
            ViewPreset::Front,
            ViewPreset::Back,
            ViewPreset::Left,
            ViewPreset::Right,
            ViewPreset::Top,
            ViewPreset::Bottom,
        ] {
            let mut camera = Camera::default();
            camera.set_preset(preset);
            for (nx, ny) in [(0.25, -0.3), (-0.7, 0.6)] {
                let (origin, direction) = camera.ray(nx, ny);
                let projected = camera.project_ndc(origin + direction * 4.0);
                assert!((projected.x - nx).abs() < 1e-4, "{preset:?}: {projected:?}");
                assert!((projected.y - ny).abs() < 1e-4, "{preset:?}: {projected:?}");
            }
        }
    }

    #[test]
    fn pan_follows_screen_basis_in_top_and_bottom_views() {
        for preset in [ViewPreset::Top, ViewPreset::Bottom, ViewPreset::Persp] {
            let mut camera = Camera::default();
            camera.set_preset(preset);
            let before = camera.project_ndc(Vec3::ZERO);
            camera.pan(20.0, 0.0);
            let horizontal = camera.project_ndc(Vec3::ZERO);
            assert!(horizontal.x > before.x);
            assert!((horizontal.y - before.y).abs() < 1e-4);
            camera.pan(0.0, 20.0);
            let vertical = camera.project_ndc(Vec3::ZERO);
            assert!(vertical.y < horizontal.y);
            assert!((vertical.x - horizontal.x).abs() < 1e-4);
        }
    }

    #[test]
    fn frame_keeps_selection_inside_portrait_viewport() {
        for projection in [Projection::Perspective, Projection::Ortho] {
            let mut camera = Camera {
                aspect: 0.35,
                proj: projection,
                yaw: 0.0,
                pitch: 0.0,
                ..Camera::default()
            };
            camera.frame(Vec3::ZERO, 2.0);
            for corner in [
                Vec3::X * 2.0,
                Vec3::NEG_X * 2.0,
                Vec3::Y * 2.0,
                Vec3::NEG_Y * 2.0,
            ] {
                let projected = camera.project_ndc(corner);
                assert!(
                    projected.x.abs() <= 1.0 && projected.y.abs() <= 1.0,
                    "{projection:?}: {projected:?}"
                );
            }
        }
    }

    #[test]
    fn isometric_presets_and_nominal_views() {
        let presets = [
            (ViewPreset::Front, "Front Ortho"),
            (ViewPreset::Back, "Back Ortho"),
            (ViewPreset::Left, "Left Ortho"),
            (ViewPreset::Right, "Right Ortho"),
            (ViewPreset::Top, "Top Ortho"),
            (ViewPreset::Bottom, "Bottom Ortho"),
            (ViewPreset::IsometricNE, "Isometric NE"),
            (ViewPreset::IsometricNW, "Isometric NW"),
            (ViewPreset::IsometricSE, "Isometric SE"),
            (ViewPreset::IsometricSW, "Isometric SW"),
            (ViewPreset::PseudoIsometric2To1, "Pseudo-Isometric 2:1"),
        ];

        for (preset, expected_name) in presets {
            let mut camera = Camera::default();
            camera.set_preset(preset);
            assert_eq!(camera.proj, Projection::Ortho);
            let detected_preset = camera.view_preset();
            assert_eq!(detected_preset, Some(preset), "Preset {preset:?} mismatch");
            let (name, _, _) = camera.nominal_view();
            assert_eq!(name, expected_name, "Nominal view mismatch for {preset:?}");
        }

        let mut persp_camera = Camera::default();
        persp_camera.set_preset(ViewPreset::Persp);
        assert_eq!(persp_camera.proj, Projection::Perspective);
        let (name, _, _) = persp_camera.nominal_view();
        assert_eq!(name, "User Persp");
    }

    #[test]
    fn pseudo_isometric_2to1_framing_and_projection() {
        let mut camera = Camera::default();
        let initial_visible_h = camera.visible_height();
        camera.set_preset(ViewPreset::PseudoIsometric2To1);

        assert_eq!(camera.proj, Projection::Ortho);
        assert_eq!(camera.view_preset(), Some(ViewPreset::PseudoIsometric2To1));

        let (name, yaw_deg, pitch_deg) = camera.nominal_view();
        assert_eq!(name, "Pseudo-Isometric 2:1");
        assert!((yaw_deg - 45.0).abs() < 1e-3);
        assert!((pitch_deg - 26.565).abs() < 1e-3);

        // Framing preservation: visible height should remain identical
        assert!((camera.visible_height() - initial_visible_h).abs() < 1e-4);
    }
}
