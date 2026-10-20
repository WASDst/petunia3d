//! Transformações interativas (teclado, Inspector e viewport) e fatias de viewport.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Atualiza o plano de corte enquanto o ponteiro se move.
    pub fn update_viewport_slice(&mut self, x: f32, y: f32) -> bool {
        self.update_viewport_slice_modified(x, y, false)
    }

    /// Atualiza o plano de corte com suporte a snap angular (ex.: 15° ao segurar Ctrl/snap).
    pub fn update_viewport_slice_modified(&mut self, x: f32, y: f32, snap: bool) -> bool {
        if self.slice_anchor.is_none() {
            return false;
        }
        self.update_slice_modified(x, y, snap)
    }

    /// Inicia uma transformação modal por arrasto na viewport.
    pub fn begin_viewport_transform(&mut self, kind: TransformKind, x: f32, y: f32) -> bool {
        self.tool_session.clear_numeric();
        self.instant_transform = false;
        let modal_kind = match kind {
            TransformKind::Position => petunia_core::ModalKind::Move,
            TransformKind::Rotation => petunia_core::ModalKind::Rotate,
            TransformKind::Scale => petunia_core::ModalKind::Scale,
        };
        match self.state.begin_modal(modal_kind) {
            Ok(()) => {
                self.start_viewport_drag(kind, x, y);
                true
            }
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    /// Arrasto de viewport sobre uma operação já aberta no core.
    pub(crate) fn start_viewport_drag(&mut self, kind: TransformKind, x: f32, y: f32) {
        self.tool_session.clear_numeric();
        self.instant_transform = false;
        self.reset_transform_fields();
        self.drag = Some(ViewportDrag {
            kind,
            start: [x, y],
            viewport: self.viewport_size,
            last_pointer: [x, y],
            virtual_pointer: [x, y],
            rotation_angle: 0.0,
            last_angle: 0.0,
        });
    }

    /// Atualiza a transformação a partir do deslocamento absoluto do ponteiro.
    pub fn update_viewport_transform(&mut self, x: f32, y: f32) -> bool {
        self.update_viewport_transform_modified(x, y, false, false)
    }

    pub fn update_viewport_transform_modified(
        &mut self,
        x: f32,
        y: f32,
        fine: bool,
        snap: bool,
    ) -> bool {
        self.pointer_position = [x, y];
        if !self.tool_session.numeric_text().is_empty() {
            return false;
        }
        if !x.is_finite() || !y.is_finite() {
            return false;
        }
        let Some(mut drag) = self.drag else {
            return false;
        };
        let precision = if fine { 0.1 } else { 1.0 };
        for (axis, pointer) in [x, y].into_iter().enumerate() {
            let direction = if axis == 1 && self.state.ui.invert_vertical_drag {
                -1.0
            } else {
                1.0
            };
            drag.virtual_pointer[axis] +=
                (pointer - drag.last_pointer[axis]) * precision * direction;
            drag.last_pointer[axis] = pointer;
        }
        if self.state.workspace.is_animate() {
            let Some(bone_id) = self.state.session.animate.selected_bone else {
                return false;
            };
            let Some(skel_id) = self.state.session.animate.skeleton else {
                return false;
            };
            let Some(skeleton) = self
                .state
                .project
                .skeletons
                .iter()
                .find(|s| s.id == skel_id)
            else {
                return false;
            };
            let Some(bone) = skeleton.bones.iter().find(|b| b.id == bone_id) else {
                return false;
            };

            let camera = &self.state.session.camera;
            let viewport = glam::Vec2::from_array(drag.viewport);
            let start = glam::Vec2::from_array(drag.start);
            let current = glam::Vec2::from_array(drag.virtual_pointer);
            let delta = current - start;
            let pivot = self
                .state
                .animate_selected_bone_pivot()
                .unwrap_or(glam::Vec3::from(bone.head));

            let handle = self.gizmo_drag.unwrap_or(GizmoHandle::Center);
            let axis_vec = match handle {
                GizmoHandle::X => glam::Vec3::X,
                GizmoHandle::Y => glam::Vec3::Y,
                GizmoHandle::Z => glam::Vec3::Z,
                _ => -camera.forward(),
            };

            let mut transform = bone.local_transform;

            match drag.kind {
                TransformKind::Rotation => {
                    let normal = axis_vec;
                    let angle = petunia_core::transform_projection::rotation_angle(
                        camera, viewport, start, current, pivot, normal,
                    )
                    .unwrap_or(delta.x * 0.5);
                    let change = (angle - drag.last_angle + 180.0).rem_euclid(360.0) - 180.0;
                    drag.rotation_angle += change;
                    drag.last_angle = angle;
                    let angle_deg = if snap {
                        (drag.rotation_angle / 15.0).round() * 15.0
                    } else {
                        drag.rotation_angle
                    };
                    let delta_rot =
                        glam::Quat::from_axis_angle(normal.normalize(), angle_deg.to_radians());
                    let initial_rot = glam::Quat::from_array(bone.local_transform.rotation);
                    let new_rot = (delta_rot * initial_rot).normalize();
                    transform.rotation = new_rot.to_array();
                }
                TransformKind::Position => {
                    let d = match handle {
                        GizmoHandle::X | GizmoHandle::Y | GizmoHandle::Z => {
                            let val = petunia_core::transform_projection::axis_delta(
                                camera, viewport, start, current, pivot, axis_vec,
                            )
                            .unwrap_or(delta.x * 0.01);
                            axis_vec * val
                        }
                        _ => petunia_core::transform_projection::plane_delta(
                            camera,
                            viewport,
                            start,
                            current,
                            pivot,
                            -camera.forward(),
                        )
                        .unwrap_or(glam::Vec3::ZERO),
                    };
                    let initial_pos = glam::Vec3::from(bone.local_transform.translation);
                    transform.translation = (initial_pos + d).to_array();
                }
                TransformKind::Scale => {
                    let factor = (1.0 + delta.x * 0.01).max(0.01);
                    let initial_scale = glam::Vec3::from(bone.local_transform.scale);
                    transform.scale = (initial_scale * factor).to_array();
                }
            }

            self.drag = Some(drag);
            self.state
                .session
                .animate
                .set_bone_draft(bone_id, transform);
            self.state.mark_dirty();
            return true;
        }
        let Some(modal) = self.state.session.tools.modal.as_ref() else {
            return false;
        };
        let (constraint, pivot, normal) = (modal.constraint, modal.pivot, modal.normal);
        let camera = &self.state.session.camera;
        let viewport = glam::Vec2::from_array(drag.viewport);
        let start = glam::Vec2::from_array(drag.start);
        let current = glam::Vec2::from_array(drag.virtual_pointer);
        let delta = current - start;
        let axis = |index| modal.frame.axis(index);
        use petunia_core::{ModalConstraint, transform_projection as projection};
        let result = match drag.kind {
            TransformKind::Position => {
                let (mut translation, mut scalar) = match constraint {
                    ModalConstraint::Axis(index) => {
                        let value = projection::axis_delta(
                            camera,
                            viewport,
                            start,
                            current,
                            pivot,
                            axis(index),
                        )
                        .unwrap_or_else(|| {
                            self.screen_delta_on_axis(index, delta.x, delta.y, drag.viewport)
                        });
                        (axis(index) * value, value)
                    }
                    ModalConstraint::Plane(index) => (
                        projection::plane_delta(
                            camera,
                            viewport,
                            start,
                            current,
                            pivot,
                            axis(index),
                        )
                        .unwrap_or(glam::Vec3::ZERO),
                        0.0,
                    ),
                    ModalConstraint::Free => (
                        projection::plane_delta(
                            camera,
                            viewport,
                            start,
                            current,
                            pivot,
                            camera.forward(),
                        )
                        .unwrap_or(glam::Vec3::ZERO),
                        0.0,
                    ),
                };
                let mut snapped_kind = None;
                if snap || self.state.session.snap_enabled {
                    let target = self.state.session.snap_settings.target;
                    let mut mask = petunia_core::SnapMask::for_target(target);
                    // Move não tem plano de grade inequívoco em 3D: a grade vira
                    // passo relativo ao pivô abaixo, como antes.
                    mask.grid = false;
                    let hit = self.screen_snap(
                        current,
                        mask,
                        Some(petunia_core::SnapAnchor::world(pivot)),
                        None,
                    );
                    if let Some(hit) = hit {
                        translation = hit.point - pivot;
                        match constraint {
                            ModalConstraint::Axis(index) => {
                                let a = axis(index);
                                translation = a * translation.dot(a);
                                scalar = translation.dot(a);
                            }
                            ModalConstraint::Plane(index) => {
                                let a = axis(index);
                                translation = translation - a * translation.dot(a);
                            }
                            ModalConstraint::Free => {}
                        }
                        snapped_kind = Some(hit.kind);
                    } else if snap
                        && matches!(
                            target,
                            petunia_core::SnapTarget::Grid | petunia_core::SnapTarget::Increment
                        )
                    {
                        // Passo da grade só com Ctrl: o snap padrão é contextual.
                        let step = self.state.session.snap_settings.grid_spacing.max(0.001);
                        translation = (translation / step).round() * step;
                        scalar = (scalar / step).round() * step;
                    }
                }
                self.state
                    .update_modal_snapped(translation, scalar, snapped_kind)
            }
            TransformKind::Rotation => {
                let normal = match constraint {
                    ModalConstraint::Axis(i) | ModalConstraint::Plane(i) => axis(i),
                    ModalConstraint::Free => normal,
                };
                let angle =
                    projection::rotation_angle(camera, viewport, start, current, pivot, normal)
                        .unwrap_or(delta.x * 0.5);
                let change = (angle - drag.last_angle + 180.0).rem_euclid(360.0) - 180.0;
                drag.rotation_angle += change;
                drag.last_angle = angle;
                // Passos de 15° só com Ctrl: o snap contextual (ligado por padrão,
                // cap. 01) encaixa em alvos, não arredonda todo gesto.
                let angle = if snap {
                    (drag.rotation_angle / 15.0).round() * 15.0
                } else {
                    drag.rotation_angle
                };
                self.state.update_modal(glam::Vec3::ZERO, angle)
            }
            TransformKind::Scale => {
                let origin = projection::project_pixel(camera, viewport, pivot).unwrap_or(start);
                let a = start - origin;
                let b = current - origin;
                let mut factor = if a.length() >= 8.0 {
                    b.dot(a) / a.length_squared()
                } else {
                    1.0 + delta.x * 0.005
                };
                if snap {
                    factor = (factor * 10.0).round() / 10.0;
                }
                if factor.abs() < 0.001 {
                    factor = if factor < 0.0 { -0.001 } else { 0.001 };
                }
                self.state.update_modal(glam::Vec3::ZERO, factor)
            }
        };
        self.drag = Some(drag);
        match result {
            Ok(()) => {
                let components = self
                    .state
                    .session
                    .tools
                    .modal
                    .as_ref()
                    .map(|modal| modal.components);
                if let Some(components) = components {
                    let fields = match drag.kind {
                        TransformKind::Position => &mut self.position,
                        TransformKind::Rotation => &mut self.rotation,
                        TransformKind::Scale => &mut self.scale,
                    };
                    fields[0].set_value(components.x);
                    fields[1].set_value(components.y);
                    fields[2].set_value(components.z);
                }
                self.state.mark_dirty();
                true
            }
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    /// Confirma a transformação por arrasto como uma única operação de undo.
    pub fn end_viewport_transform(&mut self) -> bool {
        self.release_latched_session();
        if self.drag.take().is_none() {
            return false;
        }
        if self.state.workspace.is_animate() {
            if let Some(draft) = self.state.session.animate.bone_draft().copied() {
                let _ = self
                    .state
                    .animate_set_bone_transform(draft.bone_id, draft.transform);
                self.state.session.animate.clear_bone_draft();
            }
            self.gizmo_drag = None;
            self.reset_transform_fields();
            self.state.mark_dirty();
            return true;
        }
        self.commit_transform();
        self.reset_transform_fields();
        true
    }

    pub fn cancel_viewport_transform(&mut self) -> bool {
        self.release_latched_session();
        if self.drag.take().is_none() {
            return false;
        }
        if self.state.workspace.is_animate() {
            self.state.session.animate.clear_bone_draft();
            self.gizmo_drag = None;
            self.state.mark_dirty();
            return true;
        }
        self.cancel_transform()
    }

    /// Operação aberta por teclado encerrada por outro caminho (Enter, valor
    /// digitado, barra): a sessão travada que seguia o ponteiro termina junto.
    pub(crate) fn release_latched_session(&mut self) {
        if self.tool_session.is_latched() {
            self.tool_session.reset();
            self.tool_gesture = None;
        }
    }

    pub(crate) fn begin_keyboard_transform(&mut self, kind: TransformKind) {
        let tool = match kind {
            TransformKind::Position => "move",
            TransformKind::Rotation => "rotate",
            TransformKind::Scale => "scale",
        };
        let now = std::time::Instant::now();
        let interval_ms = self.preferences.double_tap_interval_ms;

        let is_double_tap = if interval_ms > 0 {
            if let Some((ref last_tool, _, last_time)) = self.last_tool_press {
                last_tool == tool
                    && now.duration_since(last_time).as_millis() <= interval_ms as u128
            } else {
                false
            }
        } else {
            self.state.session.tools.active_tool == tool
        };

        let is_instant_activation = self.state.session.tools.tool_activation
            == petunia_core::ToolActivation::Instant
            || self.state.ui.active_keymap_id.starts_with("blender")
            || self.preferences.active_keymap_id.starts_with("blender")
            || is_double_tap;

        if is_instant_activation {
            let double_tap_origin = if is_double_tap {
                self.last_tool_press
                    .as_ref()
                    .map(|(_, origin, _)| origin.clone())
            } else {
                None
            };
            self.last_tool_press = None;
            if self.state.session.tools.modal.is_some() {
                self.cancel_transform();
            }
            if let Some(origin) = double_tap_origin {
                self.state.session.tools.temporary_modal_origin_tool = Some(origin);
                self.state.session.tools.active_tool = tool.to_string();
            } else {
                self.state.begin_temporary_transform(tool);
            }
            let [x, y] = self.pointer_position;
            if self.begin_viewport_transform(kind, x, y) {
                self.instant_transform = true;
                // Gramática única: o modal por atalho é uma sessão travada.
                let _ = self
                    .tool_session
                    .begin_latched([x, y], petunia_core::PressTarget::Surface);
                self.tool_gesture = Some(ToolGesture::Transform { gizmo: false });
                let label = match kind {
                    TransformKind::Position => "Mover",
                    TransformKind::Rotation => "Rotacionar",
                    TransformKind::Scale => "Escalar",
                };
                self.state.set_status(crate::tr::fill(
                    &self
                        .state
                        .t_id(petunia_config::text_id::STATUS_FREE_MODE_MOVE_THE_MOUSE_CLICK_OR),
                    &[("0", label.to_string())],
                ));
            } else {
                self.state.restore_origin_tool();
            }
        } else {
            let origin_tool = self.state.session.tools.active_tool.clone();
            self.apply(UiIntent::SetActiveTool(tool.into()));
            self.last_tool_press = Some((tool.to_string(), origin_tool, now));
            self.instant_transform = false;
            let label = match kind {
                TransformKind::Position => "Mover",
                TransformKind::Rotation => "Rotacionar",
                TransformKind::Scale => "Escalar",
            };
            self.state.set_status(crate::tr::fill(
                &self
                    .state
                    .t_id(petunia_config::text_id::STATUS_TOOL_ACTIVE_DRAG_THE_GIZMO_TYPE_A),
                &[("0", label.to_string())],
            ));
        }
    }

    pub fn nudge_selection(&mut self, dx: f32, dy: f32, dz: f32) -> bool {
        if self.state.workspace == Workspace::Uv {
            return self.uv_move_selected(dx, dy);
        }
        if self.state.workspace != Workspace::Model {
            return false;
        }
        let delta = glam::Vec3::new(dx, dy, dz);
        if delta.length_squared() < 1e-8 {
            return false;
        }
        let command = petunia_core::NudgeCmd {
            delta: delta.to_array(),
        };
        if let Err(error) = self.state.dispatch(&command) {
            // Sem seleção em Edição o core recusa em vez de mover a malha inteira;
            // o status explica a tecla que não fez nada.
            self.state.set_status(error.to_string());
            return false;
        }
        self.state.set_status(crate::tr::fill(
            &self.state.t_id(petunia_config::text_id::STATUS_NUDGE),
            &[
                ("dx", format!("{dx:+.2}")),
                ("dy", format!("{dy:+.2}")),
                ("dz", format!("{dz:+.2}")),
            ],
        ));
        true
    }

    pub fn scrub_transform(
        &mut self,
        kind: TransformKind,
        axis: usize,
        delta: f32,
        fine: bool,
    ) -> f32 {
        if self.state.session.tools.modal.is_none()
            && let Err(error) = self.begin_transform(kind)
        {
            self.state.set_status(error.to_string());
            return self.transform_value(kind, axis.min(2));
        }
        let axis = axis.min(2);
        let field = match kind {
            TransformKind::Position => &mut self.position[axis],
            TransformKind::Rotation => &mut self.rotation[axis],
            TransformKind::Scale => &mut self.scale[axis],
        };
        let new_val = field.scrub(delta, fine);
        let current = match kind {
            TransformKind::Position => self.position.map(|f| f.value()),
            TransformKind::Rotation => self.rotation.map(|f| f.value()),
            TransformKind::Scale => self.scale.map(|f| f.value()),
        };
        let delta_components = match kind {
            TransformKind::Position | TransformKind::Rotation => [
                current[0] - self.transform_initial[0],
                current[1] - self.transform_initial[1],
                current[2] - self.transform_initial[2],
            ],
            TransformKind::Scale => [
                if self.transform_initial[0].abs() > 1e-5 {
                    current[0] / self.transform_initial[0]
                } else {
                    current[0]
                },
                if self.transform_initial[1].abs() > 1e-5 {
                    current[1] / self.transform_initial[1]
                } else {
                    current[1]
                },
                if self.transform_initial[2].abs() > 1e-5 {
                    current[2] / self.transform_initial[2]
                } else {
                    current[2]
                },
            ],
        };
        if let Err(error) = self
            .state
            .update_modal_components(glam::Vec3::from_array(delta_components))
        {
            self.state.set_status(error.to_string());
        }
        new_val
    }

    pub fn begin_transform(&mut self, kind: TransformKind) -> Result<(), petunia_core::ModalError> {
        let modal_kind = match kind {
            TransformKind::Position => petunia_core::ModalKind::Move,
            TransformKind::Rotation => petunia_core::ModalKind::Rotate,
            TransformKind::Scale => petunia_core::ModalKind::Scale,
        };
        self.state.begin_modal(modal_kind)?;
        self.transform_initial = if let Some(asset) = self.state.project.active() {
            match kind {
                TransformKind::Position => asset.position,
                TransformKind::Rotation => asset.rotation,
                TransformKind::Scale => asset.scale,
            }
        } else {
            match kind {
                TransformKind::Scale => [1.0, 1.0, 1.0],
                _ => [0.0, 0.0, 0.0],
            }
        };
        Ok(())
    }

    pub fn commit_transform(&mut self) -> bool {
        self.release_latched_session();
        let had_operation = self.state.session.tools.modal.is_some();
        self.last_operation = self.state.commit_modal_gesture();
        self.reset_transform_fields();
        had_operation
    }

    pub fn commit_transform_text(
        &mut self,
        kind: TransformKind,
        axis: usize,
        text: &str,
    ) -> Result<f32, numeric::NumericInputError> {
        let current_base = match kind {
            TransformKind::Position => self.position[axis.min(2)].value(),
            TransformKind::Rotation => self.rotation[axis.min(2)].value(),
            TransformKind::Scale => self.scale[axis.min(2)].value(),
        };
        // Reject malformed text before opening a transaction.
        let value = numeric::parse_numeric_with_base(text, current_base)?;
        // Um único objeto: os campos mostram o TRS **local**, então o valor digitado
        // define o TRS local (um comando, um Undo). O gesto modal compõe no espaço do
        // mundo e, num objeto girado, trocaria eixos ou cisalharia a escala.
        if let Some(index) = self.single_object_transform_target() {
            let mut current = match kind {
                TransformKind::Position => self.position.map(|f| f.value()),
                TransformKind::Rotation => self.rotation.map(|f| f.value()),
                TransformKind::Scale => self.scale.map(|f| f.value()),
            };
            current[axis.min(2)] = value;
            let cmd = petunia_core::SetAssetTransformCmd {
                asset_index: Some(index),
                translation: (kind == TransformKind::Position).then_some(current),
                rotation_euler_deg: (kind == TransformKind::Rotation).then_some(current),
                scale: (kind == TransformKind::Scale).then_some(current),
            };
            if let Err(error) = self.state.dispatch(&cmd) {
                self.state.set_status(error.to_string());
                self.reset_transform_fields();
                return Err(numeric::NumericInputError::Invalid);
            }
            self.state.mark_dirty();
            self.reset_transform_fields();
            return Ok(value);
        }
        let kind_modal = match kind {
            TransformKind::Position => petunia_core::ModalKind::Move,
            TransformKind::Rotation => petunia_core::ModalKind::Rotate,
            TransformKind::Scale => petunia_core::ModalKind::Scale,
        };
        if self
            .state
            .session
            .tools
            .modal
            .as_ref()
            .is_some_and(|modal| modal.kind != kind_modal)
        {
            return Err(numeric::NumericInputError::Invalid);
        }
        let started = self.state.session.tools.modal.is_none();
        if started && let Err(error) = self.begin_transform(kind) {
            self.state.set_status(error.to_string());
            return Err(numeric::NumericInputError::Invalid);
        }
        let mut current = match kind {
            TransformKind::Position => self.position.map(|f| f.value()),
            TransformKind::Rotation => self.rotation.map(|f| f.value()),
            TransformKind::Scale => self.scale.map(|f| f.value()),
        };
        current[axis.min(2)] = value;
        let delta_components = match kind {
            TransformKind::Position | TransformKind::Rotation => [
                current[0] - self.transform_initial[0],
                current[1] - self.transform_initial[1],
                current[2] - self.transform_initial[2],
            ],
            TransformKind::Scale => [
                if self.transform_initial[0].abs() > 1e-5 {
                    current[0] / self.transform_initial[0]
                } else {
                    current[0]
                },
                if self.transform_initial[1].abs() > 1e-5 {
                    current[1] / self.transform_initial[1]
                } else {
                    current[1]
                },
                if self.transform_initial[2].abs() > 1e-5 {
                    current[2] / self.transform_initial[2]
                } else {
                    current[2]
                },
            ],
        };
        if let Err(error) = self
            .state
            .update_modal_components(glam::Vec3::from_array(delta_components))
        {
            self.state.set_status(error.to_string());
            if started {
                self.cancel_transform();
            }
            return Err(numeric::NumericInputError::Invalid);
        }
        let fields = match kind {
            TransformKind::Position => &mut self.position,
            TransformKind::Rotation => &mut self.rotation,
            TransformKind::Scale => &mut self.scale,
        };
        for (field, val) in fields.iter_mut().zip(current) {
            field.set_value(val);
        }
        self.state.commit_modal();
        self.reset_transform_fields();
        Ok(value)
    }

    /// Índice do objeto cujo TRS os campos editam diretamente: modo objeto, sem
    /// gesto em curso, sem edição de pivô e exatamente o objeto ativo selecionado.
    fn single_object_transform_target(&self) -> Option<usize> {
        if self.state.session.tools.modal.is_some()
            || self.state.session.edit_pivot
            || self.state.selection_domain() != petunia_core::SelectionDomain::Object
        {
            return None;
        }
        let active = self.state.project.active()?;
        let selected = &self.state.session.selection.assets;
        (selected.len() == 1 && selected.first() == Some(&active.id))
            .then_some(self.state.project.active)
    }

    pub fn cancel_transform(&mut self) -> bool {
        self.release_latched_session();
        let cancelled = self.state.cancel_modal();
        self.reset_transform_fields();
        cancelled
    }

    pub fn reset_transform_fields(&mut self) {
        self.tool_session.clear_numeric();
        self.instant_transform = false;
        self.gizmo_drag = None;
        if let Some(asset) = self.state.project.active() {
            for (field, &val) in self.position.iter_mut().zip(&asset.position) {
                field.set_value(val);
            }
            for (field, &val) in self.rotation.iter_mut().zip(&asset.rotation) {
                field.set_value(val);
            }
            for (field, &val) in self.scale.iter_mut().zip(&asset.scale) {
                field.set_value(val);
            }
        } else {
            for field in &mut self.position {
                field.set_value(0.0);
            }
            for field in &mut self.rotation {
                field.set_value(0.0);
            }
            for field in &mut self.scale {
                field.set_value(1.0);
            }
        }
    }

    pub(crate) fn transform_value(&self, kind: TransformKind, axis: usize) -> f32 {
        match kind {
            TransformKind::Position => self.position[axis].value(),
            TransformKind::Rotation => self.rotation[axis].value(),
            TransformKind::Scale => self.scale[axis].value(),
        }
    }
}

#[cfg(test)]
mod typed_transform_tests {
    use super::*;

    fn bridge() -> SlintUiBridge<PlaceholderViewport> {
        let mut b = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        b.resize_viewport(800, 600);
        b.apply(UiIntent::SetActiveTool("select".into()));
        b.state.dispatch(&petunia_core::SelectAllCmd).unwrap();
        b
    }

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        a.iter().zip(&b).all(|(x, y)| (x - y).abs() < 1e-3)
    }

    /// Regressão (U19): os campos da seção Transform mostram a escala **local**;
    /// digitar nela num objeto já girado deve definir a escala local, sem
    /// cisalhamento nem troca de eixos.
    #[test]
    fn typed_scale_on_rotated_object_sets_local_scale() {
        let mut b = bridge();
        b.commit_transform_text(TransformKind::Rotation, 2, "90")
            .unwrap();
        for (axis, text) in ["2", "3", "0.5"].iter().enumerate() {
            b.commit_transform_text(TransformKind::Scale, axis, text)
                .unwrap();
        }
        let asset = b.state.project.active().unwrap();
        assert!(
            close(asset.scale, [2.0, 3.0, 0.5]),
            "escala local {:?}",
            asset.scale
        );
        assert!(
            close(asset.rotation, [0.0, 0.0, 90.0]),
            "rotação {:?}",
            asset.rotation
        );
        // Os campos exibem o que o documento guarda.
        assert!(close(b.scale.map(|f| f.value()), [2.0, 3.0, 0.5]));
        // Cada valor digitado é um passo de Undo.
        b.apply(UiIntent::Undo);
        assert!(close(
            b.state.project.active().unwrap().scale,
            [2.0, 3.0, 1.0]
        ));
    }

    #[test]
    fn typed_rotation_sets_each_local_euler_component() {
        let mut b = bridge();
        b.commit_transform_text(TransformKind::Rotation, 0, "30")
            .unwrap();
        b.commit_transform_text(TransformKind::Rotation, 2, "90")
            .unwrap();
        let rotation = b.state.project.active().unwrap().rotation;
        assert!(close(rotation, [30.0, 0.0, 90.0]), "rotação {rotation:?}");
    }
}
