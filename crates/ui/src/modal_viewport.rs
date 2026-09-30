//! Adaptador de eventos egui para a transação modal do domínio.
use egui::{Color32, Key, PointerButton, Pos2, Rect, Vec2};
use glam::Vec3;
use petunia_core::modal::{ModalConstraint, ModalKind, PointerSession};
use petunia_core::{AppState, Projection};

pub fn start_handle(
    _ctx: &egui::Context,
    state: &mut AppState,
    kind: ModalKind,
    constraint: ModalConstraint,
    anchor: Pos2,
) {
    match state.begin_modal(kind) {
        Ok(()) => {
            if let Err(error) = state.set_modal_constraint(constraint) {
                state.set_status(error.to_string());
            }
            state.pointer_session = Some(PointerSession::new([anchor.x, anchor.y], true));
        }
        Err(error) => state.set_status(error.to_string()),
    }
}

/// Retorna true enquanto a sessão possui o viewport (inclusive no frame de término).
pub fn draw(
    ctx: &egui::Context,
    state: &mut AppState,
    rect: Rect,
    painter: &egui::Painter,
) -> bool {
    if crate::tool_fields::owns_modal(ctx) {
        return state.modal.is_some();
    }
    let mut started = false;
    if let Some(kind) = state.pending_modal {
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            state.pending_modal = None;
            return true;
        }
        if let Some(pos) = ctx.pointer_hover_pos().filter(|pos| rect.contains(*pos)) {
            state.pending_modal = None;
            match state.begin_modal(kind) {
                Ok(()) => {
                    state.pointer_session = Some(PointerSession::new([pos.x, pos.y], false));
                    started = true;
                }
                Err(error) => state.set_status(error.to_string()),
            }
        } else {
            painter.text(
                rect.center_top() + egui::vec2(0.0, 24.0),
                egui::Align2::CENTER_TOP,
                "Mova o cursor para o viewport · Esc: cancelar",
                egui::FontId::proportional(14.0),
                egui::Color32::WHITE,
            );
            return true;
        }
    }
    let Some(op) = state.modal.as_ref() else {
        return started;
    };
    let (kind, pivot, normal) = (op.kind, op.pivot, op.normal);
    if matches!(kind, ModalKind::Move | ModalKind::Rotate | ModalKind::Scale) {
        crate::gizmo::draw_gizmo(
            painter,
            &state.camera,
            rect,
            pivot,
            match kind {
                ModalKind::Rotate => crate::gizmo::GizmoKind::Rotate,
                ModalKind::Scale => crate::gizmo::GizmoKind::Scale,
                _ => crate::gizmo::GizmoKind::Translate,
            },
            ctx.pointer_hover_pos(),
        );
    }
    let mut pointer = state
        .pointer_session
        .take()
        .unwrap_or_else(|| PointerSession::new([rect.center().x, rect.center().y], false));
    let anchor = Pos2::new(pointer.anchor[0], pointer.anchor[1]);
    let pos = ctx
        .pointer_hover_pos()
        .unwrap_or(Pos2::new(pointer.last_pos[0], pointer.last_pos[1]));
    pointer.last_pos = [pos.x, pos.y];
    ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
    let cancel = ctx.input(|i| {
        i.key_pressed(Key::Escape) || i.pointer.button_pressed(PointerButton::Secondary)
    });
    if cancel {
        state.cancel_modal();
        return true;
    }
    let mut changed = started || ctx.input(|i| !i.events.is_empty());
    for (key, axis) in [(Key::X, 0), (Key::Y, 1), (Key::Z, 2)] {
        if ctx.input(|i| i.key_pressed(key)) {
            let constraint = if ctx.input(|i| i.modifiers.shift) {
                ModalConstraint::Plane(axis)
            } else {
                ModalConstraint::Axis(axis)
            };
            let current = state.modal.as_ref().map(|op| op.constraint);
            let next = if current == Some(constraint) {
                ModalConstraint::Free
            } else {
                constraint
            };
            if let Err(error) = state.set_modal_constraint(next) {
                state.set_status(error.to_string());
            }
            state.locked_axes = match next {
                ModalConstraint::Axis(0) => [true, false, false],
                ModalConstraint::Axis(1) => [false, true, false],
                ModalConstraint::Axis(2) => [false, false, true],
                ModalConstraint::Plane(0) => [false, true, true],
                ModalConstraint::Plane(1) => [true, false, true],
                ModalConstraint::Plane(2) => [true, true, false],
                _ => [false, false, false],
            };
            changed = true;
        }
    }
    ctx.input(|i| {
        for event in &i.events {
            if matches!(
                event,
                egui::Event::Key {
                    key: Key::Backspace,
                    pressed: true,
                    ..
                }
            ) {
                pointer.numeric.pop();
            }
            if let egui::Event::Text(text) = event
                && !i.modifiers.command
                && !i.modifiers.ctrl
                && text
                    .chars()
                    .all(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '+'))
                && pointer.numeric.len() + text.len() <= 64
            {
                pointer.numeric.push_str(&text.replace(',', "."));
            }
        }
    });
    let constraint = state
        .modal
        .as_ref()
        .map(|op| op.constraint)
        .unwrap_or(ModalConstraint::Free);
    let delta = pos - anchor;
    let units = world_per_pixel(state, rect, pivot);
    let translation = (state.camera.right() * delta.x - state.camera.up() * delta.y) * units;
    let axis = match constraint {
        ModalConstraint::Axis(axis) | ModalConstraint::Plane(axis) => [Vec3::X, Vec3::Y, Vec3::Z]
            .get(axis)
            .copied()
            .unwrap_or(Vec3::X),
        _ => normal,
    };
    let translation = match constraint {
        ModalConstraint::Plane(axis) => {
            let normal = [Vec3::X, Vec3::Y, Vec3::Z][axis];
            match (
                plane_point(state, rect, anchor, pivot, normal),
                plane_point(state, rect, pos, pivot, normal),
            ) {
                (Some(start), Some(end)) => end - start,
                _ => translation - normal * translation.dot(normal),
            }
        }
        _ => translation,
    };
    let mut value = match kind {
        ModalKind::Move if !matches!(constraint, ModalConstraint::Axis(_)) => translation.length(),
        ModalKind::Move
        | ModalKind::Extrude
        | ModalKind::ExtrudeIndividual
        | ModalKind::PushPull => projected_distance(state, rect, pivot, axis, delta, units),
        ModalKind::Rotate => {
            match (
                plane_point(state, rect, anchor, pivot, axis),
                plane_point(state, rect, pos, pivot, axis),
            ) {
                (Some(a), Some(b))
                    if (a - pivot).length() > units * 12.0
                        && (b - pivot).length() > units * 12.0 =>
                {
                    let a = (a - pivot).normalize();
                    let b = (b - pivot).normalize();
                    axis.dot(a.cross(b)).atan2(a.dot(b)).to_degrees()
                }
                _ => delta.x * 0.5,
            }
        }
        ModalKind::Scale => {
            if matches!(constraint, ModalConstraint::Axis(_)) {
                1.0 + projected_distance(state, rect, pivot, axis, delta, units) / (80.0 * units)
            } else {
                1.0 + delta.x * 0.01
            }
        }
        ModalKind::Inset => delta.x * 0.005,
        ModalKind::Bevel => delta.x * units,
    };
    let numeric = pointer.parse_numeric();
    let valid_number = pointer.numeric.is_empty() || numeric.is_some();
    if let Some(number) = numeric {
        value = number;
    }
    let snap = ctx.input(|i| i.modifiers.ctrl) || state.snap_enabled;
    let grid_step = state.snap_settings.grid_spacing.max(0.001);
    let translation = if snap && numeric.is_none() {
        value = snap_value(
            value,
            if kind == ModalKind::Rotate {
                15.0
            } else {
                grid_step
            },
        );
        (translation / grid_step).round() * grid_step
    } else {
        translation
    };
    let translation = if numeric.is_some()
        && !matches!(constraint, ModalConstraint::Axis(_))
        && kind == ModalKind::Move
    {
        let fallback = match constraint {
            ModalConstraint::Plane(axis) => [Vec3::Y, Vec3::Z, Vec3::X][axis],
            _ => state.camera.right(),
        };
        translation.try_normalize().unwrap_or(fallback) * value
    } else {
        translation
    };
    let mut valid_preview = valid_number && pointer.valid_preview;
    if changed && valid_number {
        valid_preview = true;
        if let Err(error) = state.update_modal(translation, value) {
            state.set_status(error.to_string());
            valid_preview = false;
        }
    }
    pointer.valid_preview = valid_preview;
    let confirm = !started
        && ctx.input(|i| {
            i.key_pressed(Key::Enter)
                || (rect.contains(pos)
                    && if pointer.drag_handle {
                        i.pointer.button_released(PointerButton::Primary)
                    } else {
                        i.pointer.button_pressed(PointerButton::Primary)
                    })
        });
    if confirm && valid_preview {
        state.commit_modal();
        return true;
    }
    draw_axis_guide_lines(painter, state, rect, pivot, constraint);

    let tool_feedback = state.current_tool_feedback();
    if let Some(ref fb) = tool_feedback
        && let Some((origin, current)) = fb.guide_line
        && let (Some(p0), Some(p1)) = (
            screen_point(&state.camera, rect, origin),
            screen_point(&state.camera, rect, current),
        )
    {
        let col = if fb.is_snapped {
            crate::tokens::ACCENT_AMBER
        } else {
            Color32::from_rgba_premultiplied(100, 200, 255, 180)
        };
        painter.line_segment([p0, p1], egui::Stroke::new(1.5_f32, col));
        if fb.is_snapped {
            painter.circle_filled(p1, 4.0, crate::tokens::ACCENT_AMBER);
        }
    }

    painter.line_segment(
        [anchor, pos],
        egui::Stroke::new(1.0_f32, egui::Color32::LIGHT_BLUE),
    );
    painter.circle_stroke(
        anchor,
        4.0,
        egui::Stroke::new(1.0_f32, egui::Color32::WHITE),
    );
    let unit = match kind {
        ModalKind::Rotate => "°",
        ModalKind::Scale => "×",
        ModalKind::Inset => " fator",
        _ => " m",
    };

    draw_modal_hud(
        painter,
        rect,
        ModalHudInfo {
            pos,
            kind,
            value,
            unit,
            pointer_numeric: &pointer.numeric,
            constraint,
            valid_preview,
            feedback: tool_feedback.as_ref(),
        },
    );
    state.pointer_session = Some(pointer);
    true
}

fn snap_value(value: f32, step: f32) -> f32 {
    (value / step).round() * step
}

fn world_per_pixel(state: &AppState, rect: Rect, pivot: Vec3) -> f32 {
    let half_height = match state.camera.proj {
        Projection::Ortho => state.camera.ortho_half_h,
        Projection::Perspective => {
            (pivot - state.camera.eye())
                .dot(state.camera.forward())
                .max(0.01)
                * (state.camera.fov_y * 0.5).tan()
        }
    };
    2.0 * half_height / rect.height().max(1.0)
}

fn projected_distance(
    state: &AppState,
    rect: Rect,
    pivot: Vec3,
    axis: Vec3,
    delta: Vec2,
    units: f32,
) -> f32 {
    let a = state.camera.project_ndc(pivot);
    let b = state.camera.project_ndc(pivot + axis);
    let projected = Vec2::new(
        (b.x - a.x) * rect.width() * 0.5,
        (a.y - b.y) * rect.height() * 0.5,
    );
    if projected.length_sq() < 16.0 {
        delta.x * units
    } else {
        delta.dot(projected) / projected.length_sq()
    }
}

pub(crate) fn plane_point(
    state: &AppState,
    rect: Rect,
    pos: Pos2,
    pivot: Vec3,
    normal: Vec3,
) -> Option<Vec3> {
    let nx = (pos.x - rect.left()) / rect.width() * 2.0 - 1.0;
    let ny = 1.0 - (pos.y - rect.top()) / rect.height() * 2.0;
    let (origin, direction) = state.camera.ray(nx, ny);
    let denominator = direction.dot(normal);
    if denominator.abs() < 1e-5 {
        return None;
    }
    let distance = (pivot - origin).dot(normal) / denominator;
    let point = origin + direction * distance;
    (distance >= 0.0 && point.is_finite()).then_some(point)
}

pub(crate) fn screen_point(camera: &petunia_core::Camera, rect: Rect, point: Vec3) -> Option<Pos2> {
    let clip = camera.view_proj() * point.extend(1.0);
    if !clip.is_finite() || clip.w <= 0.0 {
        return None;
    }
    Some(Pos2::new(
        rect.center().x + clip.x / clip.w * rect.width() * 0.5,
        rect.center().y - clip.y / clip.w * rect.height() * 0.5,
    ))
}

/// Desenha linhas-guia 3D infinitas no viewport através do pivô na cor canônica do eixo ou plano travado.
pub fn draw_axis_guide_lines(
    painter: &egui::Painter,
    state: &AppState,
    rect: Rect,
    pivot: Vec3,
    constraint: ModalConstraint,
) {
    if !rect.is_positive() {
        return;
    }

    let draw_single_axis = |axis_vec: Vec3, color: Color32| {
        let Some(p0) = screen_point(&state.camera, rect, pivot) else {
            return;
        };
        let Some(p1) = screen_point(&state.camera, rect, pivot + axis_vec * 2.0) else {
            return;
        };
        let delta = p1 - p0;
        if delta.length_sq() < 1e-4 {
            return;
        }
        let dir = delta.normalized();
        let start = p0 - dir * 4000.0;
        let end = p0 + dir * 4000.0;

        let glow = Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 45);
        painter
            .with_clip_rect(rect)
            .line_segment([start, end], egui::Stroke::new(6.0_f32, glow));
        painter
            .with_clip_rect(rect)
            .line_segment([start, end], egui::Stroke::new(2.0_f32, color));
    };

    match constraint {
        ModalConstraint::Axis(0) => draw_single_axis(Vec3::X, crate::tokens::AXIS_X),
        ModalConstraint::Axis(1) => draw_single_axis(Vec3::Y, crate::tokens::AXIS_Y),
        ModalConstraint::Axis(2) => draw_single_axis(Vec3::Z, crate::tokens::AXIS_Z),
        ModalConstraint::Plane(0) => {
            draw_single_axis(Vec3::Y, crate::tokens::AXIS_Y);
            draw_single_axis(Vec3::Z, crate::tokens::AXIS_Z);
            draw_plane_shade(painter, state, rect, pivot, Vec3::Y, Vec3::Z);
        }
        ModalConstraint::Plane(1) => {
            draw_single_axis(Vec3::X, crate::tokens::AXIS_X);
            draw_single_axis(Vec3::Z, crate::tokens::AXIS_Z);
            draw_plane_shade(painter, state, rect, pivot, Vec3::X, Vec3::Z);
        }
        ModalConstraint::Plane(2) => {
            draw_single_axis(Vec3::X, crate::tokens::AXIS_X);
            draw_single_axis(Vec3::Y, crate::tokens::AXIS_Y);
            draw_plane_shade(painter, state, rect, pivot, Vec3::X, Vec3::Y);
        }
        _ => {}
    }
}

fn draw_plane_shade(
    painter: &egui::Painter,
    state: &AppState,
    rect: Rect,
    pivot: Vec3,
    u: Vec3,
    v: Vec3,
) {
    let s = 1.2;
    if let (Some(q0), Some(q1), Some(q2), Some(q3)) = (
        screen_point(&state.camera, rect, pivot - u * s - v * s),
        screen_point(&state.camera, rect, pivot + u * s - v * s),
        screen_point(&state.camera, rect, pivot + u * s + v * s),
        screen_point(&state.camera, rect, pivot - u * s + v * s),
    ) {
        painter
            .with_clip_rect(rect)
            .add(egui::Shape::convex_polygon(
                vec![q0, q1, q2, q3],
                Color32::from_rgba_premultiplied(100, 180, 255, 30),
                egui::Stroke::new(
                    1.0_f32,
                    Color32::from_rgba_premultiplied(150, 210, 255, 100),
                ),
            ));
    }
}

struct ModalHudInfo<'a> {
    pos: Pos2,
    kind: ModalKind,
    value: f32,
    unit: &'a str,
    pointer_numeric: &'a str,
    constraint: ModalConstraint,
    valid_preview: bool,
    feedback: Option<&'a petunia_core::ToolFeedback>,
}

fn draw_modal_hud(painter: &egui::Painter, rect: Rect, info: ModalHudInfo<'_>) {
    let label = match info.kind {
        ModalKind::Move => "Mover",
        ModalKind::Rotate => "Rotacionar",
        ModalKind::Scale => "Escalar",
        ModalKind::Extrude => "Extrusão",
        ModalKind::ExtrudeIndividual => "Extrusão individual",
        ModalKind::Inset => "Inserção",
        ModalKind::Bevel => "Chanfro",
        ModalKind::PushPull => "Push/Pull",
    };
    let value_label = if info.pointer_numeric.is_empty() {
        format!("{:.3}", info.value)
    } else {
        info.pointer_numeric.to_string()
    };

    let (badge_text, badge_color) = match info.constraint {
        ModalConstraint::Axis(0) => ("AXIS X", crate::tokens::AXIS_X),
        ModalConstraint::Axis(1) => ("AXIS Y", crate::tokens::AXIS_Y),
        ModalConstraint::Axis(2) => ("AXIS Z", crate::tokens::AXIS_Z),
        ModalConstraint::Plane(0) => ("PLANE YZ (Shift+X)", Color32::from_rgb(30, 144, 180)),
        ModalConstraint::Plane(1) => ("PLANE XZ (Shift+Y)", Color32::from_rgb(142, 68, 173)),
        ModalConstraint::Plane(2) => ("PLANE XY (Shift+Z)", Color32::from_rgb(211, 84, 0)),
        _ => ("FREE", Color32::from_rgb(80, 85, 95)),
    };

    let snap_badge = if info.feedback.map(|f| f.is_snapped).unwrap_or(false) {
        " [SNAP]"
    } else {
        ""
    };

    let line1 = format!(
        "{label}: {value_label}{}   [{badge_text}]{snap_badge}",
        info.unit
    );
    let line2 = "X/Y/Z: travar eixo · Shift: plano · Ctrl: snap";
    let line3 = if info.valid_preview {
        "Enter/LMB: confirmar · Esc/RMB: cancelar"
    } else {
        "Valor inválido — ajuste ou cancele"
    };

    let text = format!("{line1}\n{line2}\n{line3}");
    let font_id = egui::FontId::monospace(11.5);
    let galley = painter.layout_no_wrap(text, font_id, Color32::WHITE);

    let hud_pos = Pos2::new(
        (info.pos.x + 18.0)
            .min(rect.right() - galley.size().x - 16.0)
            .max(rect.left() + 8.0),
        (info.pos.y + 18.0)
            .min(rect.bottom() - galley.size().y - 16.0)
            .max(rect.top() + 8.0),
    );

    let hud_rect = Rect::from_min_size(
        hud_pos - egui::vec2(8.0, 6.0),
        galley.size() + egui::vec2(16.0, 12.0),
    );

    painter.rect_filled(hud_rect, 6.0, Color32::from_black_alpha(225));
    painter.rect_stroke(
        hud_rect,
        6.0,
        egui::Stroke::new(1.5_f32, badge_color),
        egui::StrokeKind::Outside,
    );

    painter.galley(hud_pos, galley, Color32::WHITE);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_axis_guide_lines_all_constraints() {
        let ctx = egui::Context::default();
        let state = AppState::new("en");
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));
        let pivot = Vec3::new(0.0, 0.0, 0.0);

        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let painter = ui.painter();
                // Test all axis and plane constraints without panic
                for constraint in [
                    ModalConstraint::Free,
                    ModalConstraint::Axis(0),
                    ModalConstraint::Axis(1),
                    ModalConstraint::Axis(2),
                    ModalConstraint::Plane(0),
                    ModalConstraint::Plane(1),
                    ModalConstraint::Plane(2),
                ] {
                    draw_axis_guide_lines(painter, &state, rect, pivot, constraint);
                }
            });
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn test_draw_modal_hud_all_kinds_and_constraints() {
        let ctx = egui::Context::default();
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));
        let pos = Pos2::new(200.0, 200.0);

        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let painter = ui.painter();
                for kind in [
                    ModalKind::Move,
                    ModalKind::Rotate,
                    ModalKind::Scale,
                    ModalKind::Extrude,
                    ModalKind::Inset,
                    ModalKind::Bevel,
                    ModalKind::PushPull,
                ] {
                    let mock_fb = petunia_core::ToolFeedback {
                        origin: Vec3::ZERO,
                        current: Vec3::ONE,
                        guide_line: Some((Vec3::ZERO, Vec3::ONE)),
                        delta_text: "Δ 1.50 m".into(),
                        delta_value: 1.5,
                        axis_constraint: None,
                        plane_constraint: None,
                        is_snapped: true,
                        snap_kind: None,
                        status_hint: "LMB Confirm",
                    };

                    for constraint in [
                        ModalConstraint::Free,
                        ModalConstraint::Axis(0),
                        ModalConstraint::Axis(1),
                        ModalConstraint::Axis(2),
                        ModalConstraint::Plane(0),
                        ModalConstraint::Plane(1),
                        ModalConstraint::Plane(2),
                    ] {
                        draw_modal_hud(
                            painter,
                            rect,
                            ModalHudInfo {
                                pos,
                                kind,
                                value: 1.5,
                                unit: " m",
                                pointer_numeric: "",
                                constraint,
                                valid_preview: true,
                                feedback: None,
                            },
                        );
                        draw_modal_hud(
                            painter,
                            rect,
                            ModalHudInfo {
                                pos,
                                kind,
                                value: -0.5,
                                unit: "°",
                                pointer_numeric: "45.0",
                                constraint,
                                valid_preview: false,
                                feedback: Some(&mock_fb),
                            },
                        );
                    }
                }
            });
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn test_screen_point_valid_and_behind_camera() {
        let camera = petunia_core::Camera {
            aspect: 800.0 / 600.0,
            ..Default::default()
        };
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));

        // Target in front of camera should yield Some(point) inside or near viewport
        let pt_in_front = screen_point(&camera, rect, camera.target);
        assert!(pt_in_front.is_some());
        let p = pt_in_front.unwrap();
        assert!((p.x - 400.0).abs() < 10.0);
        assert!((p.y - 300.0).abs() < 10.0);

        // Point far behind camera should be None
        let behind = camera.eye() - camera.forward() * 10.0;
        let pt_behind = screen_point(&camera, rect, behind);
        assert!(pt_behind.is_none());
    }
}
