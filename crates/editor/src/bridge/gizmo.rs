//! Gizmos de transformação e handles paramétricos na viewport.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Only expose handles backed by a transformable target. Profile adapters
    /// remain explicit until heterogeneous target resolution is implemented.
    pub(crate) fn projected_gizmo(&self) -> GizmoModel {
        let mut gizmo = compute_gizmo(&self.state, self.viewport_size[0], self.viewport_size[1]);
        let unsupported_profile = self.state.workspace == Workspace::Model
            && self.state.active_tool == "select"
            && self.active_profile_resources().is_some();
        let unavailable_asset = self.state.workspace == Workspace::Model
            && self
                .state
                .project
                .active()
                .is_none_or(|asset| asset.locked || !asset.visible);
        if unsupported_profile || unavailable_asset {
            gizmo.visible = false;
            gizmo.clear_handle_geometry();
        }
        gizmo
    }

    /// Handle do gizmo sob o cursor (preselection, sem clique).
    /// Handle do gizmo sob um ponto de tela, dentro de um raio de tolerância.
    ///
    /// O teste é em espaço de tela porque o gizmo tem tamanho fixo em pixels:
    /// o alvo do mouse precisa ser generoso (12 px) mesmo com a haste fina.
    pub fn gizmo_handle_at(&self, x: f32, y: f32) -> Option<GizmoHandle> {
        self.gizmo_target_at(x, y).map(|target| target.handle)
    }

    pub(crate) fn gizmo_target_at(&self, x: f32, y: f32) -> Option<GizmoTarget> {
        let gizmo = self.projected_gizmo();
        if !gizmo.visible {
            return None;
        }
        let active = if self.state.workspace.is_animate() {
            match self.state.session.tools.active_tool.as_str() {
                "move" => "move",
                "scale" => "scale",
                _ => "rotate",
            }
        } else if self.state.session.tools.active_tool == "select" {
            "transform"
        } else {
            self.state.session.tools.active_tool.as_str()
        };
        let dist_from_origin = (x - gizmo.origin_x).hypot(y - gizmo.origin_y);
        // Desambiguação do gizmo universal: zona morta do centro protege contra cliques acidentais
        if active == "transform" && dist_from_origin < 12.0 {
            return None;
        }

        // Centro (Screen-space translation para Move, Uniform Scale para Scale, Trackball para Rotate)
        if dist_from_origin <= 12.0 {
            let kind = match active {
                "scale" => TransformKind::Scale,
                "rotate" => TransformKind::Rotation,
                _ => TransformKind::Position,
            };
            return Some(GizmoTarget {
                handle: GizmoHandle::Center,
                kind,
            });
        }

        // View Roll para a ferramenta Rotate (anel perimetral externo a ~113px)
        if active == "rotate"
            && (dist_from_origin - projection::GIZMO_VIEW_ROLL_RADIUS).abs()
                <= projection::GIZMO_RING_HIT_HALF_WIDTH
        {
            return Some(GizmoTarget {
                handle: GizmoHandle::Center,
                kind: TransformKind::Rotation,
            });
        }

        // Quadrantes de planos (Move e Scale): YZ (normal 0), XZ (normal 1), XY (normal 2)
        if matches!(active, "move" | "scale") {
            let plane_commands = [
                (0u8, &gizmo.plane_yz_commands),
                (1u8, &gizmo.plane_xz_commands),
                (2u8, &gizmo.plane_xy_commands),
            ];
            for (normal, cmd) in plane_commands {
                let numbers: Vec<f32> = cmd
                    .split_whitespace()
                    .filter_map(|token| token.parse::<f32>().ok())
                    .collect();
                if numbers.len() >= 8 {
                    let cx = (numbers[0] + numbers[2] + numbers[4] + numbers[6]) * 0.25;
                    let cy = (numbers[1] + numbers[3] + numbers[5] + numbers[7]) * 0.25;
                    if (x - cx).hypot(y - cy) <= 10.0 {
                        let kind = if active == "scale" {
                            TransformKind::Scale
                        } else {
                            TransformKind::Position
                        };
                        return Some(GizmoTarget {
                            handle: GizmoHandle::Plane(normal),
                            kind,
                        });
                    }
                }
            }
        }

        let families = [
            (
                TransformKind::Scale,
                9.0,
                [
                    &gizmo.x_scale_commands,
                    &gizmo.y_scale_commands,
                    &gizmo.z_scale_commands,
                ],
            ),
            (
                TransformKind::Rotation,
                7.0,
                [
                    &gizmo.x_rotate_commands,
                    &gizmo.y_rotate_commands,
                    &gizmo.z_rotate_commands,
                ],
            ),
            (
                match active {
                    "rotate" => TransformKind::Rotation,
                    "scale" => TransformKind::Scale,
                    _ => TransformKind::Position,
                },
                12.0,
                [&gizmo.x_commands, &gizmo.y_commands, &gizmo.z_commands],
            ),
        ];
        for (kind, hit_radius, commands_by_axis) in families {
            let mut family_best: Option<(GizmoTarget, f32)> = None;
            for (axis, commands) in commands_by_axis.into_iter().enumerate() {
                let handle = [GizmoHandle::X, GizmoHandle::Y, GizmoHandle::Z][axis];
                let numbers: Vec<f32> = commands
                    .split_whitespace()
                    .filter_map(|token| token.parse::<f32>().ok())
                    .collect();
                for segment in numbers.as_chunks::<2>().0.windows(2) {
                    let a = [segment[0][0], segment[0][1]];
                    let b = [segment[1][0], segment[1][1]];
                    if (a[0] - b[0]).hypot(a[1] - b[1]) < 0.25 {
                        continue;
                    }
                    let distance = point_segment_distance([x, y], a, b);
                    if distance <= hit_radius
                        && family_best.is_none_or(|(_, current)| distance < current)
                    {
                        family_best = Some((GizmoTarget { handle, kind }, distance));
                    }
                }
            }
            if let Some((target, _)) = family_best {
                return Some(target);
            }
        }
        None
    }

    /// Inicia o arrasto no handle do gizmo, restringindo a transformação ao eixo ou plano.
    pub fn begin_gizmo_drag(&mut self, x: f32, y: f32) -> bool {
        if self.state.primitive_session_valid() {
            self.state.finalize_primitive_session();
        }
        let Some(target) = self.gizmo_target_at(x, y) else {
            return false;
        };
        let handle = target.handle;
        let kind = target.kind;
        if self.state.workspace.is_animate() {
            let Some(_bone_id) = self.state.session.animate.selected_bone else {
                return false;
            };
            self.start_viewport_drag(kind, x, y);
            self.gizmo_drag = Some(handle);
            self.state.set_status(format!(
                "{} · {}",
                match kind {
                    TransformKind::Position => "Move Bone",
                    TransformKind::Rotation => "Rotate Bone",
                    TransformKind::Scale => "Scale Bone",
                },
                match handle {
                    GizmoHandle::X => "X axis",
                    GizmoHandle::Y => "Y axis",
                    GizmoHandle::Z => "Z axis",
                    GizmoHandle::Center => "Free rotation",
                    _ => "Plane",
                }
            ));
            return true;
        }
        if !self.begin_viewport_transform(kind, x, y) {
            return false;
        }
        // A restrição é do domínio: o preview já sai no eixo/plano certo.
        let constraint = match handle {
            GizmoHandle::X => petunia_core::ModalConstraint::Axis(0),
            GizmoHandle::Y => petunia_core::ModalConstraint::Axis(1),
            GizmoHandle::Z => petunia_core::ModalConstraint::Axis(2),
            GizmoHandle::Center => petunia_core::ModalConstraint::Free,
            GizmoHandle::Plane(normal) => petunia_core::ModalConstraint::Plane(normal as usize),
        };
        let _ = self.state.set_modal_constraint(constraint);
        self.gizmo_drag = Some(handle);
        self.state.set_status(format!(
            "{} · {}",
            match kind {
                TransformKind::Position => "Move",
                TransformKind::Rotation => "Rotate",
                TransformKind::Scale => "Scale",
            },
            match handle {
                GizmoHandle::X => "X axis",
                GizmoHandle::Y => "Y axis",
                GizmoHandle::Z => "Z axis",
                GizmoHandle::Center => match kind {
                    TransformKind::Position => "Screen plane",
                    TransformKind::Scale => "Uniform",
                    TransformKind::Rotation => "View roll",
                },
                GizmoHandle::Plane(0) => "YZ plane",
                GizmoHandle::Plane(1) => "XZ plane",
                GizmoHandle::Plane(2) => "XY plane",
                GizmoHandle::Plane(_) => "Plane",
            }
        ));
        true
    }

    /// Encerra o arrasto do gizmo.
    pub fn end_gizmo_drag(&mut self) -> bool {
        if self.gizmo_drag.take().is_none() {
            return false;
        }
        self.end_viewport_transform();
        self.gizmo_hover = None;
        true
    }

    /// Alça visível da ferramenta paramétrica ativa (Extrude, Inset, Round
    /// Edge, Push/Pull): `(base, alça)` em px da viewport. A base é o pivô
    /// (no Extrude, o topo da extrusão em prévia); a alça fica a
    /// [`projection::PARAMETRIC_HANDLE_OFFSET_PX`] na direção do valor e segue
    /// o valor durante o gesto.
    pub(crate) fn parametric_handle(&self) -> Option<([f32; 2], [f32; 2])> {
        let Some(GrammarTool::Parametric(tool_kind)) = self.grammar_tool() else {
            return None;
        };
        if self.state.workspace != Workspace::Model || tool_kind == ToolModalKind::ScaleSelection {
            return None;
        }
        let kind = self.tool_modal.unwrap_or(tool_kind);
        let viewport = self.viewport_size;
        if viewport[0] <= 1.0 || viewport[1] <= 1.0 {
            return None;
        }
        let camera = &self.state.session.camera;
        let (pivot, normal, value) = match self.state.session.tools.modal.as_ref() {
            Some(modal) => (modal.pivot, modal.normal, self.tool_modal_value),
            None => {
                let mesh = self.state.project.active_mesh()?;
                let mut normal = glam::Vec3::ZERO;
                for (index, face) in mesh.faces.iter().enumerate() {
                    if face.selected {
                        normal += mesh.face_normal(index);
                    }
                }
                let ready = if kind == ToolModalKind::Bevel {
                    !mesh.selected_edges.is_empty()
                } else {
                    normal != glam::Vec3::ZERO
                };
                if !ready {
                    return None;
                }
                let pivot = self.state.calculate_pivot(self.state.session.pivot_point);
                (pivot, normal.normalize_or_zero(), 0.0)
            }
        };
        let offset = projection::PARAMETRIC_HANDLE_OFFSET_PX;
        let pivot_px = project_world_point(camera, viewport, pivot)?;
        let screen_per_world = project_world_point(camera, viewport, pivot + camera.right())
            .map(|right| (right[0] - pivot_px[0]).hypot(right[1] - pivot_px[1]))
            .filter(|pixels| pixels.is_finite() && *pixels > 1.0e-3)?;
        let value = if value.is_finite() { value } else { 0.0 };
        match kind {
            ToolModalKind::Extrude | ToolModalKind::ExtrudeIndividual | ToolModalKind::PushPull => {
                let base = project_world_point(camera, viewport, pivot + normal * value)?;
                let tip = project_world_point(camera, viewport, pivot + normal * (value + 1.0));
                let handle = match tip
                    .map(|tip| glam::Vec2::new(tip[0] - base[0], tip[1] - base[1]))
                    .filter(|d| d.length() >= 6.0)
                {
                    Some(direction) => glam::Vec2::from(base) + direction.normalize() * offset,
                    // Normal para a câmera: o arrasto vertical controla o valor
                    // (`drag_value`), e a alça sobe com ele.
                    None => {
                        glam::Vec2::from(base)
                            - glam::Vec2::Y * (offset + value.max(0.0) * screen_per_world)
                    }
                };
                Some((base, handle.to_array()))
            }
            // Inset cresce ao aproximar do pivô; Round Edge, ao afastar.
            ToolModalKind::Inset => {
                let x = (offset - value * screen_per_world)
                    .max(projection::PARAMETRIC_HANDLE_RADIUS_PX);
                Some((pivot_px, [pivot_px[0] + x, pivot_px[1]]))
            }
            ToolModalKind::Bevel => {
                let x = offset + value * screen_per_world;
                Some((pivot_px, [pivot_px[0] + x, pivot_px[1]]))
            }
            ToolModalKind::ScaleSelection => None,
        }
    }

    /// Gizmo de transformação como formas de tela para o passo GPU. A
    /// geometria é a mesma de [`compute_gizmo`] (que também serve ao hit
    /// test), com a hierarquia de estados do shell: o eixo ativo/travado
    /// domina, o eixo sob o cursor clareia e os demais esmaecem no arrasto.
    pub(crate) fn gizmo_overlay_shapes(&self) -> Vec<petunia_render::OverlayShape> {
        use petunia_render::OverlayShape;
        let gizmo = self.projected_gizmo();
        if !gizmo.visible || self.state.workspace != Workspace::Model {
            return Vec::new();
        }
        let theme_color = |token: petunia_config::ThemeToken| -> [f32; 4] {
            let registry = petunia_config::ThemeRegistry::global();
            let c = registry
                .get_theme(&self.state.ui.active_theme_id)
                .or_else(|| registry.get_theme("petunia-dark"))
                .map_or(petunia_config::theme::ColorRgba::WHITE, |theme| {
                    theme.colors.get_token_color(token)
                });
            c.0.map(|channel| channel as f32 / 255.0)
        };
        let accent = theme_color(petunia_config::ThemeToken::AccentBlue);
        let center = theme_color(petunia_config::ThemeToken::TextPrimary);
        let canvas = theme_color(petunia_config::ThemeToken::BgCanvas);
        let muted = theme_color(petunia_config::ThemeToken::TextSecondary);
        let hover = self.gizmo_hover.and_then(|h| h.axis()).map(|a| a as i32);
        let active = self.gizmo_drag.and_then(|h| h.axis()).map(|a| a as i32);
        let constraint = match self
            .state
            .session
            .tools
            .modal
            .as_ref()
            .map(|op| op.constraint)
        {
            Some(petunia_core::ModalConstraint::Axis(axis)) => [axis as i32, -1],
            Some(petunia_core::ModalConstraint::Plane(excluded)) => {
                [((excluded + 1) % 3) as i32, ((excluded + 2) % 3) as i32]
            }
            _ => [-1, -1],
        };
        let fade = |c: [f32; 4], amount: f32| [c[0], c[1], c[2], c[3] * (1.0 - amount)];
        let color = |axis: i32| -> [f32; 4] {
            let base = self.gizmo_axis_colors.0[axis as usize];
            if constraint.contains(&axis) {
                base
            } else if constraint[0] >= 0 || active.is_some_and(|a| a != axis) {
                fade(base, 0.72)
            } else if active == Some(axis) {
                base
            } else if hover == Some(axis) {
                let lift = |v: f32| v + (1.0 - v) * 0.35;
                [lift(base[0]), lift(base[1]), lift(base[2]), base[3]]
            } else {
                base
            }
        };
        let stroke_width = |axis: i32| -> f32 {
            if constraint.contains(&axis) || active == Some(axis) {
                4.0
            } else if constraint[0] >= 0 {
                2.0
            } else if hover == Some(axis) {
                3.5
            } else {
                2.5
            }
        };
        let mut shapes = Vec::new();
        let mut push = |commands: &str, fill: Option<[f32; 4]>, stroke: Option<([f32; 4], f32)>| {
            for (points, closed) in path_polylines(commands) {
                shapes.push(OverlayShape {
                    points,
                    closed,
                    fill: fill.filter(|_| closed),
                    stroke,
                });
            }
        };
        let axes = [
            (
                &gizmo.x_commands,
                &gizmo.x_arrow_commands,
                &gizmo.x_scale_commands,
                &gizmo.x_rotate_commands,
                &gizmo.plane_yz_commands,
            ),
            (
                &gizmo.y_commands,
                &gizmo.y_arrow_commands,
                &gizmo.y_scale_commands,
                &gizmo.y_rotate_commands,
                &gizmo.plane_xz_commands,
            ),
            (
                &gizmo.z_commands,
                &gizmo.z_arrow_commands,
                &gizmo.z_scale_commands,
                &gizmo.z_rotate_commands,
                &gizmo.plane_xy_commands,
            ),
        ];
        // Planos primeiro (translúcidos, por baixo das hastes).
        for (axis, (_, _, _, _, plane)) in axes.iter().enumerate() {
            let c = color(axis as i32);
            push(plane, Some(fade(c, 0.65)), Some((c, 1.0)));
        }
        for (axis, (rod, arrow, scale, rotate, _)) in axes.iter().enumerate() {
            let c = color(axis as i32);
            push(rotate, None, Some((fade(c, 0.32), 1.5)));
            push(rod, None, Some((c, stroke_width(axis as i32))));
            push(arrow, Some(c), None);
            push(scale, Some(c), Some((canvas, 1.0)));
        }
        let center_on = matches!(self.gizmo_drag, Some(GizmoHandle::Center));
        let center_hover = matches!(self.gizmo_hover, Some(GizmoHandle::Center));
        let ring = if center_on || center_hover {
            accent
        } else {
            fade(muted, 0.4)
        };
        push(&gizmo.view_roll_commands, None, Some((ring, 1.5)));
        let dot = if center_on {
            accent
        } else if center_hover {
            fade(accent, 0.2)
        } else {
            center
        };
        let circle: Vec<[f32; 2]> = (0..20)
            .map(|i| {
                let angle = i as f32 * std::f32::consts::TAU / 20.0;
                [
                    gizmo.origin_x + 6.0 * angle.cos(),
                    gizmo.origin_y + 6.0 * angle.sin(),
                ]
            })
            .collect();
        shapes.push(OverlayShape {
            points: circle,
            closed: true,
            fill: Some(dot),
            stroke: Some((canvas, 1.5)),
        });
        shapes
    }

    /// O ponto (px) está no alvo da alça paramétrica?
    pub(crate) fn parametric_handle_at(&self, pixel: [f32; 2]) -> bool {
        self.parametric_handle().is_some_and(|(_, handle)| {
            (pixel[0] - handle[0]).hypot(pixel[1] - handle[1])
                <= projection::PARAMETRIC_HANDLE_HIT_RADIUS_PX
        })
    }

    /// Haste da base até a alça (px), para o overlay.
    pub fn parametric_handle_commands(&self) -> (String, Option<[f32; 2]>) {
        match self.parametric_handle() {
            Some(([bx, by], [hx, hy])) => (
                format!(
                    "M {bx:.1} {by:.1} L {hx:.1} {hy:.1} M {bx:.1} {by:.1} m -3 0 a 3 3 0 1 0 6 0 a 3 3 0 1 0 -6 0"
                ),
                Some([hx, hy]),
            ),
            None => (String::new(), None),
        }
    }
}
