//! Editable Tool Properties driving the same transaction as viewport transforms.
use egui::{Key, Modifiers};
use glam::Vec3;
use petunia_core::{AppState, ModalConstraint, ModalKind};

const SESSION_ID: &str = "tool.property_fields";

#[derive(Clone)]
struct FieldSession {
    kind: ModalKind,
    values: [String; 3],
    active: bool,
    error: Option<String>,
}

impl FieldSession {
    fn new(kind: ModalKind, state: &AppState) -> Self {
        let values = match kind {
            ModalKind::Scale => [1.0; 3],
            ModalKind::Extrude => [state.extrude_dist, 0.0, 0.0],
            ModalKind::Inset => [state.inset_factor, 0.0, 0.0],
            ModalKind::Bevel => [state.bevel_amount, 0.0, 0.0],
            ModalKind::PushPull => [state.push_dist, 0.0, 0.0],
            _ => [0.0; 3],
        };
        Self {
            kind,
            values: values.map(|value| value.to_string()),
            active: false,
            error: None,
        }
    }

    fn parsed(&self, state: &AppState) -> Result<Vec3, String> {
        let count = if transform(self.kind) { 3 } else { 1 };
        let mut values = [0.0; 3];
        for (index, slot) in values.iter_mut().enumerate().take(count) {
            *slot = self.values[index]
                .trim()
                .replace(',', ".")
                .parse::<f32>()
                .ok()
                .filter(|value| value.is_finite())
                .ok_or_else(|| state.t("tool_properties.error_number"))?;
        }
        Ok(Vec3::from_array(values))
    }

    fn preview(&mut self, state: &mut AppState) -> Result<(), String> {
        if !self.active {
            state.pending_modal = None;
            state
                .begin_modal(self.kind)
                .map_err(|error| error.to_string())?;
            self.active = true;
        }
        let values = self.parsed(state)?;
        if transform(self.kind) {
            state.update_modal_components(values)
        } else {
            state
                .set_modal_constraint(ModalConstraint::Free)
                .map_err(|error| error.to_string())?;
            state.update_modal(Vec3::ZERO, values.x)
        }
        .map_err(|error| error.to_string())
    }
}

fn transform(kind: ModalKind) -> bool {
    matches!(kind, ModalKind::Move | ModalKind::Rotate | ModalKind::Scale)
}

pub fn kind_label(state: &AppState, kind: ModalKind) -> String {
    state.t(match kind {
        ModalKind::Move => "tools.move",
        ModalKind::Rotate => "tools.rotate",
        ModalKind::Scale => "tools.scale",
        ModalKind::Extrude => "tools.extrude",
        ModalKind::ExtrudeIndividual => "tools.extrude",
        ModalKind::Inset => "tools.inset",
        ModalKind::Bevel => "tools.bevel",
        ModalKind::PushPull => "tools.pushpull",
    })
}

/// Viewport input yields while the property editor owns the active transaction.
pub fn owns_modal(ctx: &egui::Context) -> bool {
    ctx.data_mut(|data| data.get_temp::<FieldSession>(egui::Id::new(SESSION_ID)))
        .is_some_and(|session| session.active)
}

fn selected_kind(state: &AppState) -> Option<ModalKind> {
    state
        .modal
        .as_ref()
        .map(|modal| modal.kind)
        .or(state.pending_modal)
        .or(match state.active_tool.as_str() {
            "move" => Some(ModalKind::Move),
            "rotate" => Some(ModalKind::Rotate),
            "scale" => Some(ModalKind::Scale),
            "extrude" => Some(ModalKind::Extrude),
            "inset" => Some(ModalKind::Inset),
            "bevel" => Some(ModalKind::Bevel),
            "pushpull" => Some(ModalKind::PushPull),
            // Universal Transform is intentionally not an implicit alias for
            // whichever gizmo mode happened to be selected previously.
            _ => None,
        })
}

fn field_id(ui: &egui::Ui, kind: ModalKind, index: usize) -> egui::Id {
    ui.id().with((SESSION_ID, kind.label(), index))
}

fn activate_transform_kind(state: &mut AppState, session: &mut FieldSession, kind: ModalKind) {
    let active_tool = match kind {
        ModalKind::Move => "move",
        ModalKind::Rotate => "rotate",
        ModalKind::Scale => "scale",
        _ => return,
    };
    state.active_tool = active_tool.to_owned();
    state.gizmo_mode = kind;
    state.pending_modal = Some(kind);
    *session = FieldSession::new(kind, state);
    state.mark_dirty();
}

/// Returns true when the selected tool has canonical editable numeric properties.
pub fn draw(ui: &mut egui::Ui, state: &mut AppState) -> bool {
    let ctx = ui.ctx().clone();
    let id = egui::Id::new(SESSION_ID);
    let Some(mut kind) = selected_kind(state) else {
        ctx.data_mut(|data| data.remove::<FieldSession>(id));
        return false;
    };
    let mut session = ctx
        .data_mut(|data| data.get_temp::<FieldSession>(id))
        .filter(|session| session.kind == kind)
        .unwrap_or_else(|| FieldSession::new(kind, state));
    if session.active && state.modal.is_none() {
        session = FieldSession::new(kind, state);
    }
    let blocked = state.mesh_preview.is_some()
        || state.paint_stroke.is_some()
        || (state.modal.is_some() && !session.active);

    if transform(kind) {
        ui.add_enabled_ui(!session.active && !blocked, |ui| {
            ui.horizontal(|ui| {
                for (candidate, label_id) in [
                    (ModalKind::Move, "tools.move"),
                    (ModalKind::Rotate, "tools.rotate"),
                    (ModalKind::Scale, "tools.scale"),
                ] {
                    if ui
                        .selectable_label(kind == candidate, state.t(label_id))
                        .clicked()
                    {
                        kind = candidate;
                        activate_transform_kind(state, &mut session, kind);
                    }
                }
            });
        });
    }

    if blocked {
        if let Some(modal) = &state.modal {
            session.values = if transform(kind) {
                modal.components.to_array()
            } else {
                [modal.value, 0.0, 0.0]
            }
            .map(|value| format!("{value:.4}"));
        }
        ui.small(state.t("tool_properties.blocked"));
    }

    let (label, unit) = match kind {
        ModalKind::Move => (state.t("tool_properties.displacement"), "m"),
        ModalKind::Rotate => (state.t("tool_properties.rotation_xyz"), "°"),
        ModalKind::Scale => (state.t("tool_properties.scale_xyz"), "×"),
        ModalKind::Extrude => (state.t("tool_properties.extrude_distance"), "m"),
        ModalKind::ExtrudeIndividual => (state.t("tool_properties.extrude_distance"), "m"),
        ModalKind::Inset => (state.t("tool_properties.inset_factor"), "0–0.95"),
        ModalKind::Bevel => (state.t("tool_properties.bevel_width"), "m"),
        ModalKind::PushPull => (state.t("tool_properties.pushpull_distance"), "m"),
    };
    ui.label(label);

    let mut changed = false;
    let mut focused = false;
    let mut apply = false;
    let mut cancel = false;
    ui.add_enabled_ui(!blocked, |ui| {
        egui::Grid::new(ui.id().with((SESSION_ID, "grid")))
            .num_columns(3)
            .show(ui, |ui| {
                for index in 0..if transform(kind) { 3 } else { 1 } {
                    ui.label(if transform(kind) {
                        ["X", "Y", "Z"][index].to_owned()
                    } else {
                        state.t("tool_properties.value")
                    });
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut session.values[index])
                            .id(field_id(ui, kind, index))
                            .desired_width(100.0)
                            .char_limit(64),
                    );
                    changed |= response.changed();
                    focused |= response.has_focus() || response.lost_focus();
                    ui.label(unit);
                    ui.end_row();
                }
            });
        if changed {
            session.error = session.preview(state).err();
            state.mark_dirty();
        }
        ui.horizontal(|ui| {
            apply = ui
                .add_enabled(
                    session.parsed(state).is_ok(),
                    egui::Button::new(state.t("actions.apply")),
                )
                .clicked();
            cancel = ui
                .add_enabled(
                    session.active || state.pending_modal.is_some(),
                    egui::Button::new(state.t("actions.cancel")),
                )
                .clicked();
        });
        if focused || session.active {
            apply |= ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter));
            cancel |= ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape));
        }
    });

    if cancel {
        state.cancel_modal();
        session = FieldSession::new(kind, state);
    } else if apply {
        match session.preview(state) {
            Ok(()) => {
                if let Ok(values) = session.parsed(state) {
                    match kind {
                        ModalKind::Extrude => state.extrude_dist = values.x,
                        ModalKind::Inset => state.inset_factor = values.x,
                        ModalKind::Bevel => state.bevel_amount = values.x,
                        ModalKind::PushPull => state.push_dist = values.x,
                        _ => {}
                    }
                }
                state.commit_modal();
                session = FieldSession::new(kind, state);
            }
            Err(error) => session.error = Some(error),
        }
    }

    if let Some(error) = &session.error {
        ui.colored_label(egui::Color32::LIGHT_RED, error);
    }
    ui.small(state.t("tool_properties.preview_hint"));
    if kind == ModalKind::Rotate {
        ui.small(state.t("tool_properties.rotation_hint"));
    }
    if kind == ModalKind::Bevel {
        ui.small(state.t("tool_properties.bevel_hint"));
    }
    ctx.data_mut(|data| data.insert_temp(id, session));
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_children_map_explicitly_and_universal_does_not_alias_stale_mode() {
        let mut state = AppState::new("en");
        state.gizmo_mode = ModalKind::Scale;
        state.active_tool = "transform".into();
        assert_eq!(selected_kind(&state), None);
        state.active_tool = "move".into();
        assert_eq!(selected_kind(&state), Some(ModalKind::Move));
        state.active_tool = "rotate".into();
        assert_eq!(selected_kind(&state), Some(ModalKind::Rotate));
        state.active_tool = "scale".into();
        assert_eq!(selected_kind(&state), Some(ModalKind::Scale));
    }

    #[test]
    fn modal_kind_labels_are_localized() {
        let state = AppState::new("pt-BR");
        assert_eq!(kind_label(&state, ModalKind::Move), "Mover");
        // Vocabulário de usuário (cap. 13): Round Edge; Bevel é termo técnico.
        assert_eq!(kind_label(&state, ModalKind::Bevel), "Round Edge");
    }
}
