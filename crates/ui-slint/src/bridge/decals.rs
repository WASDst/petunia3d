//! Decals: preview, âncora e arraste.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Gera comandos SVG de projeção da estampa de decalque sobre a superfície 3D (P3D-133).
    pub fn decal_preview_commands(&self) -> String {
        if self.state.workspace != Workspace::Paint {
            return String::new();
        }
        let Some(decal) = self.active_decal() else {
            return String::new();
        };
        let Some(mesh) = self.state.project.active_mesh() else {
            return String::new();
        };
        let viewport = [
            self.viewport_size[0].max(1.0),
            self.viewport_size[1].max(1.0),
        ];
        let matrix = self.state.session.camera.view_proj();
        let project = |point: glam::Vec3| -> Option<[f32; 2]> {
            let clip = matrix * point.extend(1.0);
            if !clip.is_finite() || clip.w <= 0.05 || clip.z < 0.0 || clip.z > clip.w {
                return None;
            }
            Some([
                (clip.x / clip.w * 0.5 + 0.5) * viewport[0],
                (0.5 - clip.y / clip.w * 0.5) * viewport[1],
            ])
        };

        let cu = decal.center_uv[0];
        let cv = decal.center_uv[1];
        let su = decal.scale_uv[0];
        let sv = decal.scale_uv[1];
        let rot = decal.rotation_rad;
        let cos_r = rot.cos();
        let sin_r = rot.sin();

        // Converte coordenadas locais da estampa [-0.5..0.5] para espaço UV
        let local_to_uv = |lx: f32, ly: f32| -> [f32; 2] {
            let du = lx * su * cos_r - ly * sv * sin_r;
            let dv = lx * su * sin_r + ly * sv * cos_r;
            [(cu + du).clamp(0.0, 1.0), (cv + dv).clamp(0.0, 1.0)]
        };

        // Âncora central e plano tangente de contingência
        let center_hit = mesh.uv_to_world([cu, cv]);
        let (c_pos, c_norm) = center_hit.unwrap_or((glam::Vec3::ZERO, glam::Vec3::Y));
        let up = if c_norm.y.abs() > 0.9 {
            glam::Vec3::X
        } else {
            glam::Vec3::Y
        };
        let tangent = c_norm.cross(up).normalize_or_zero();
        let bitangent = c_norm.cross(tangent).normalize_or_zero();

        // Decalque de superfície: o contorno é o retângulo do projetor.
        let surface = decal.anchor.map(|anchor| {
            let (right, up, normal) = anchor.frame(decal.rotation_rad);
            (
                glam::Vec3::from(anchor.point),
                right * anchor.width,
                up * decal.surface_height(anchor.width),
                normal,
            )
        });
        let sample_world_pos = |lx: f32, ly: f32| -> glam::Vec3 {
            if let Some((point, right, up, normal)) = surface {
                return point + right * lx - up * ly + normal * 0.003;
            }
            let uv = local_to_uv(lx, ly);
            if let Some((pos, norm)) = mesh.uv_to_world(uv) {
                pos + norm * 0.003
            } else {
                let du = lx * su * cos_r - ly * sv * sin_r;
                let dv = lx * su * sin_r + ly * sv * cos_r;
                c_pos + (tangent * du + bitangent * dv) * 2.0 + c_norm * 0.003
            }
        };

        use std::fmt::Write as _;
        let mut commands = String::new();

        // 1. Perímetro do decalque com subdivisões para conformação a superfícies curvas
        let mut perimeter_pts = Vec::new();
        const SUBDIVS: usize = 4;
        // Borda superior: (-0.5, -0.5) -> (0.5, -0.5)
        for i in 0..SUBDIVS {
            let t = i as f32 / SUBDIVS as f32;
            perimeter_pts.push((-0.5 + t, -0.5));
        }
        // Borda direita: (0.5, -0.5) -> (0.5, 0.5)
        for i in 0..SUBDIVS {
            let t = i as f32 / SUBDIVS as f32;
            perimeter_pts.push((0.5, -0.5 + t));
        }
        // Borda inferior: (0.5, 0.5) -> (-0.5, 0.5)
        for i in 0..SUBDIVS {
            let t = i as f32 / SUBDIVS as f32;
            perimeter_pts.push((0.5 - t, 0.5));
        }
        // Borda esquerda: (-0.5, 0.5) -> (-0.5, -0.5)
        for i in 0..SUBDIVS {
            let t = i as f32 / SUBDIVS as f32;
            perimeter_pts.push((-0.5, 0.5 - t));
        }

        let mut first = true;
        for (lx, ly) in perimeter_pts {
            let world_pos = sample_world_pos(lx, ly);
            if let Some(screen_pt) = project(world_pos) {
                if first {
                    let _ = write!(commands, "M {:.2} {:.2} ", screen_pt[0], screen_pt[1]);
                    first = false;
                } else {
                    let _ = write!(commands, "L {:.2} {:.2} ", screen_pt[0], screen_pt[1]);
                }
            }
        }
        if !first {
            commands.push_str("Z ");
        }

        // 2. Retículo / mira central
        let center_h0 = sample_world_pos(-0.08, 0.0);
        let center_h1 = sample_world_pos(0.08, 0.0);
        if let (Some(a), Some(b)) = (project(center_h0), project(center_h1)) {
            let _ = write!(
                commands,
                "M {:.2} {:.2} L {:.2} {:.2} ",
                a[0], a[1], b[0], b[1]
            );
        }
        let center_v0 = sample_world_pos(0.0, -0.08);
        let center_v1 = sample_world_pos(0.0, 0.08);
        if let (Some(a), Some(b)) = (project(center_v0), project(center_v1)) {
            let _ = write!(
                commands,
                "M {:.2} {:.2} L {:.2} {:.2} ",
                a[0], a[1], b[0], b[1]
            );
        }

        // 3. Indicador direcional de orientação (topo do decalque)
        let arrow_base = sample_world_pos(0.0, 0.0);
        let arrow_tip = sample_world_pos(0.0, -0.38);
        let arrow_left = sample_world_pos(-0.06, -0.28);
        let arrow_right = sample_world_pos(0.06, -0.28);
        if let (Some(base), Some(tip), Some(left), Some(right)) = (
            project(arrow_base),
            project(arrow_tip),
            project(arrow_left),
            project(arrow_right),
        ) {
            let _ = write!(
                commands,
                "M {:.2} {:.2} L {:.2} {:.2} M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} ",
                base[0],
                base[1],
                tip[0],
                tip[1],
                left[0],
                left[1],
                tip[0],
                tip[1],
                right[0],
                right[1]
            );
        }

        // 4. Alças da ferramenta (constituição 11): cantos escalam, a de cima gira.
        if self.decal_tool_active()
            && let Some(handles) = self.decal_handles()
        {
            const HALF: f32 = 5.0;
            const KNOB: f32 = 6.0;
            for [x, y] in handles.corners {
                let _ = write!(
                    commands,
                    "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z ",
                    x - HALF,
                    y - HALF,
                    x + HALF,
                    y - HALF,
                    x + HALF,
                    y + HALF,
                    x - HALF,
                    y + HALF
                );
            }
            let ([tx, ty], [rx, ry]) = (handles.top, handles.rotate);
            let _ = write!(commands, "M {tx:.2} {ty:.2} L {rx:.2} {ry:.2} ");
            for step in 0..=12 {
                let angle = step as f32 / 12.0 * std::f32::consts::TAU;
                let (px, py) = (rx + KNOB * angle.cos(), ry + KNOB * angle.sin());
                let verb = if step == 0 { 'M' } else { 'L' };
                let _ = write!(commands, "{verb} {px:.2} {py:.2} ");
            }
            commands.push_str("Z ");
        }

        commands
    }

    pub fn active_layer_is_decal(&self) -> bool {
        self.state
            .project
            .assets
            .get(self.state.project.active)
            .and_then(|asset| asset.paint_stack.as_ref())
            .and_then(|stack| stack.active())
            .map(|layer| {
                matches!(
                    layer.kind,
                    petunia_project::paint_layers::LayerKind::Decal(_)
                )
            })
            .unwrap_or(false)
    }

    pub fn active_decal(&self) -> Option<petunia_project::paint_layers::DecalLayer> {
        self.state
            .project
            .assets
            .get(self.state.project.active)
            .and_then(|asset| asset.paint_stack.as_ref())
            .and_then(|stack| stack.active())
            .and_then(|layer| match &layer.kind {
                petunia_project::paint_layers::LayerKind::Decal(decal) => Some(decal.clone()),
                _ => None,
            })
    }

    pub fn set_active_decal_transform(
        &mut self,
        center_uv: [f32; 2],
        scale_uv: [f32; 2],
        rotation_rad: f32,
    ) -> bool {
        let Some(mut next) = self.active_decal() else {
            return false;
        };
        next.center_uv = [center_uv[0].clamp(0.0, 1.0), center_uv[1].clamp(0.0, 1.0)];
        next.scale_uv = [scale_uv[0].clamp(0.01, 5.0), scale_uv[1].clamp(0.01, 5.0)];
        next.rotation_rad = rotation_rad;
        self.apply_decal_state(next);
        true
    }

    /// Fixa o decalque ativo na superfície (`anchor`) com a rotação dada.
    pub fn set_active_decal_anchor(
        &mut self,
        anchor: petunia_project::paint_layers::DecalAnchor,
        rotation_rad: f32,
    ) -> bool {
        if !(anchor.width.is_finite() && anchor.width > 0.0 && rotation_rad.is_finite()) {
            return false;
        }
        let Some(mut next) = self.active_decal() else {
            return false;
        };
        next.anchor = Some(anchor);
        next.rotation_rad = rotation_rad;
        self.apply_decal_state(next);
        true
    }

    /// Fixação de decalque no ponto da superfície sob `(x, y)` do viewport,
    /// com a imagem "em pé" segundo a câmera. `None` fora do objeto.
    pub fn decal_anchor_at(
        &self,
        x: f32,
        y: f32,
        width: f32,
    ) -> Option<petunia_project::paint_layers::DecalAnchor> {
        let width_px = self.viewport_size[0].max(1.0);
        let height_px = self.viewport_size[1].max(1.0);
        if !x.is_finite()
            || !y.is_finite()
            || !(0.0..width_px).contains(&x)
            || !(0.0..height_px).contains(&y)
        {
            return None;
        }
        let camera = &self.state.session.camera;
        let (origin, direction) = camera.ray(x / width_px * 2.0 - 1.0, 1.0 - y / height_px * 2.0);
        let (face, hit) = pick_face_hit(&self.state, origin, direction)?;
        let normal = self.state.project.active_mesh()?.face_normal(face);
        petunia_project::paint_layers::DecalAnchor::new(hit, normal, camera.up(), width)
    }

    /// Fixação de um decalque novo: o ponto do objeto sob o centro da vista
    /// ou, se o centro não cai no objeto, o ponto que a câmera vê na direção
    /// do centro do objeto. Largura = 30% do maior lado do objeto.
    pub fn new_decal_anchor(&self) -> Option<petunia_project::paint_layers::DecalAnchor> {
        let mesh = self.state.project.active_mesh()?;
        let (min, max) = mesh.verts.iter().map(|v| v.vec()).fold(
            (glam::Vec3::splat(f32::MAX), glam::Vec3::splat(f32::MIN)),
            |(lo, hi), p| (lo.min(p), hi.max(p)),
        );
        let extent = (max - min).max_element();
        if !(extent.is_finite() && extent > 0.0) {
            return None;
        }
        let width = extent * 0.3;
        let center = [self.viewport_size[0] * 0.5, self.viewport_size[1] * 0.5];
        self.decal_anchor_at(center[0], center[1], width)
            .or_else(|| {
                let camera = &self.state.session.camera;
                let eye = camera.eye();
                let direction = ((min + max) * 0.5 - eye).normalize_or_zero();
                let (face, hit) = pick_face_hit(&self.state, eye, direction)?;
                petunia_project::paint_layers::DecalAnchor::new(
                    hit,
                    mesh.face_normal(face),
                    camera.up(),
                    width,
                )
            })
    }

    /// Arrasto de decalque pelo caminho do traço: sem modificador move, Shift
    /// escala e Ctrl gira — o mesmo gesto da ferramenta de decalque.
    pub fn decal_drag_begin(&mut self, x: f32, y: f32, is_shift: bool, is_ctrl: bool) -> bool {
        let mode = if is_shift {
            DecalMode::Scale
        } else if is_ctrl {
            DecalMode::Rotate
        } else {
            DecalMode::Move
        };
        if !self.begin_decal_gesture([x, y], mode) {
            return false;
        }
        self.update_decal_gesture([x, y], false, false);
        true
    }

    pub fn decal_drag_to(&mut self, x: f32, y: f32, _is_shift: bool, _is_ctrl: bool) -> bool {
        self.decal_gesture.is_some() && self.update_decal_gesture([x, y], false, false)
    }

    pub fn decal_drag_end(&mut self) -> bool {
        self.commit_decal_gesture()
    }

    pub fn cancel_decal_drag(&mut self) -> bool {
        self.cancel_decal_gesture()
    }
}
