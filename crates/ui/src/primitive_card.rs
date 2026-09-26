//! Cartão contextual Last Operation (Wave 8 + gauntlet V1 — §10/§22).
//!
//! Após inserir uma primitiva, um cartão ancorado perto dela permite ajustar
//! parâmetros com regeneração ao vivo. A inserção inteira é UMA transação de
//! undo: confirmar encerra, Reset restaura defaults, Esc cancela e remove.
//!
//! Posicionamento: canto projetado do bounding box, preso à viewport por
//! `constrain_to` (sem estimativa de tamanho). Enter/Esc valem com o ponteiro
//! sobre o cartão (Esc global nunca apaga a sessão por acidente); clique na
//! viewport confirma.

use egui::{Key, Pos2, Rect, Ui, pos2, vec2};
use petunia_core::{AppState, CircleFill, PrimitiveDescriptor, PrimitiveDescriptorExt};
use petunia_mesh::primitives::primitive_audit;

use crate::tokens;
use petunia_config::{TextId, text_id};

/// Desenha o cartão da sessão de criação ativa (nada se inválida/ausente).
pub fn draw_primitive_card(ui: &mut Ui, state: &mut AppState, viewport_rect: Rect) -> Option<Rect> {
    if !state.primitive_session_valid() {
        state.finalize_primitive_session();
        return None;
    }
    let session = state.session.primitive_session.clone()?;
    let descriptor = session.descriptor;
    let title = state.t_id(descriptor.name_key());
    let anchor = creation_anchor(state, viewport_rect, session.asset_id);
    let max_card_width = (viewport_rect.width() - 24.0).clamp(96.0, 300.0);
    let max_card_height = (viewport_rect.height() - 24.0).clamp(96.0, 520.0);

    let mut next: Option<PrimitiveDescriptor> = None;
    let mut confirm = false;
    let mut cancel = false;
    let win = egui::Window::new(title)
        .id(egui::Id::new("primitive_last_op"))
        .default_pos(anchor)
        .default_width(max_card_width.min(260.0))
        .max_width(max_card_width)
        .max_height(max_card_height)
        .constrain_to(viewport_rect)
        .collapsible(false)
        .resizable(false)
        .frame(
            egui::Frame::new()
                .fill(tokens::bg_panel(state))
                .stroke(tokens::stroke_border_dyn(state))
                .inner_margin(egui::Margin::symmetric(10, 8)),
        )
        .show(ui.ctx(), |ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
            // Parâmetros por espécie (tabela completa abaixo).
            if let Some(rebuilt) = draw_species_params(ui, state, descriptor) {
                next = Some(rebuilt);
            }
            // Estatísticas leves (§25): reforçam a identidade low-poly.
            draw_creation_stats(ui, state, session.asset_id);
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button(state.t_id(text_id::PRIMS_RESET)).clicked() {
                    next = Some(PrimitiveDescriptor::default_for(descriptor.kind()));
                }
                if ui.button(state.t_id(text_id::PRIMS_CONFIRM)).clicked() {
                    confirm = true;
                }
                if ui.button(state.t_id(text_id::PRIMS_CANCEL)).clicked() {
                    cancel = true;
                }
            });
            ui.label(
                egui::RichText::new(state.t_id(text_id::PRIMS_CONFIRM_HINT))
                    .size(10.0)
                    .color(tokens::TEXT_MUTED),
            );
        });

    // Enter/Esc só com o ponteiro sobre o cartão (segurança, §10.5).
    let hovered = win.as_ref().is_some_and(|w| w.response.hovered());
    match card_key_action(
        hovered,
        ui.ctx().input(|i| i.key_pressed(Key::Enter)),
        ui.ctx().input(|i| i.key_pressed(Key::Escape)),
    ) {
        Some(CardKeyAction::Confirm) => confirm = true,
        Some(CardKeyAction::Cancel) => cancel = true,
        None => {}
    }

    if let Some(descriptor) = next {
        state.update_primitive(descriptor);
    }
    if cancel {
        state.cancel_primitive();
    } else if confirm {
        state.confirm_primitive();
    }

    win.map(|window| window.response.rect.intersect(viewport_rect))
}

/// Campos por espécie. Retorna o descritor reconstruído quando algo mudou.
fn draw_species_params(
    ui: &mut Ui,
    state: &mut AppState,
    descriptor: PrimitiveDescriptor,
) -> Option<PrimitiveDescriptor> {
    match descriptor {
        PrimitiveDescriptor::Box {
            width,
            height,
            depth,
        } => {
            let (mut width, mut height, mut depth) = (width, height, depth);
            let mut changed = false;
            changed |= param_row(
                ui,
                state,
                text_id::PRIMS_WIDTH,
                None,
                &mut width,
                0.05..=100.0,
                0.05,
            );
            changed |= param_row(
                ui,
                state,
                text_id::PRIMS_HEIGHT,
                None,
                &mut height,
                0.05..=100.0,
                0.05,
            );
            changed |= param_row(
                ui,
                state,
                text_id::PRIMS_DEPTH,
                None,
                &mut depth,
                0.05..=100.0,
                0.05,
            );
            changed.then_some(PrimitiveDescriptor::Box {
                width,
                height,
                depth,
            })
        }
        PrimitiveDescriptor::Plane { width, height } => {
            let (mut width, mut height) = (width, height);
            let mut changed = false;
            changed |= param_row(
                ui,
                state,
                text_id::PRIMS_WIDTH,
                None,
                &mut width,
                0.05..=100.0,
                0.05,
            );
            changed |= param_row(
                ui,
                state,
                text_id::PRIMS_DEPTH,
                None,
                &mut height,
                0.05..=100.0,
                0.05,
            );
            changed.then_some(PrimitiveDescriptor::Plane { width, height })
        }
        PrimitiveDescriptor::Wedge {
            width,
            height,
            depth,
        } => {
            let (mut width, mut height, mut depth) = (width, height, depth);
            let mut changed = false;
            changed |= param_row(
                ui,
                state,
                text_id::PRIMS_WIDTH,
                None,
                &mut width,
                0.05..=100.0,
                0.05,
            );
            changed |= param_row(
                ui,
                state,
                text_id::PRIMS_HEIGHT,
                None,
                &mut height,
                0.05..=100.0,
                0.05,
            );
            changed |= param_row(
                ui,
                state,
                text_id::PRIMS_DEPTH,
                None,
                &mut depth,
                0.05..=100.0,
                0.05,
            );
            changed.then_some(PrimitiveDescriptor::Wedge {
                width,
                height,
                depth,
            })
        }
        PrimitiveDescriptor::Cylinder {
            radius,
            height,
            sides,
            cap_top,
            cap_bottom,
        } => {
            let (mut radius, mut height, mut sides) = (radius, height, sides);
            let (mut cap_top, mut cap_bottom) = (cap_top, cap_bottom);
            let mut changed = false;
            changed |= param_row(
                ui,
                state,
                text_id::PRIMS_RADIUS,
                None,
                &mut radius,
                0.05..=100.0,
                0.05,
            );
            changed |= param_row(
                ui,
                state,
                text_id::PRIMS_HEIGHT,
                None,
                &mut height,
                0.05..=100.0,
                0.05,
            );
            changed |= param_row_u32(
                ui,
                state,
                text_id::PRIMS_SIDES,
                Some(text_id::PRIMS_TIP_SIDES),
                &mut sides,
                3..=32,
                1.0,
            );
            changed |= param_caps(ui, state, &mut cap_top, &mut cap_bottom);
            changed.then_some(PrimitiveDescriptor::Cylinder {
                radius,
                height,
                sides,
                cap_top,
                cap_bottom,
            })
        }
        PrimitiveDescriptor::Cone {
            bottom_radius,
            top_radius,
            height,
            sides,
            cap_bottom,
            cap_top,
        } => {
            let (mut bottom_radius, mut top_radius, mut height, mut sides) =
                (bottom_radius, top_radius, height, sides);
            let (mut cap_bottom, mut cap_top) = (cap_bottom, cap_top);
            let mut changed = false;
            changed |= param_row(
                ui,
                state,
                text_id::PRIMS_BOTTOM_RADIUS,
                None,
                &mut bottom_radius,
                0.0..=100.0,
                0.05,
            );
            changed |= param_row(
                ui,
                state,
                text_id::PRIMS_TOP_RADIUS,
                Some(text_id::PRIMS_TIP_TOP_RADIUS),
                &mut top_radius,
                0.0..=100.0,
                0.05,
            );
            changed |= param_row(
                ui,
                state,
                text_id::PRIMS_HEIGHT,
                None,
                &mut height,
                0.05..=100.0,
                0.05,
            );
            changed |= param_row_u32(
                ui,
                state,
                text_id::PRIMS_SIDES,
                Some(text_id::PRIMS_TIP_SIDES),
                &mut sides,
                3..=32,
                1.0,
            );
            changed |= param_caps(ui, state, &mut cap_top, &mut cap_bottom);
            changed.then_some(PrimitiveDescriptor::Cone {
                bottom_radius,
                top_radius,
                height,
                sides,
                cap_bottom,
                cap_top,
            })
        }
        PrimitiveDescriptor::Circle {
            radius,
            vertices,
            fill,
        } => {
            let (mut radius, mut vertices, mut fill) = (radius, vertices, fill);
            let mut changed = false;
            changed |= param_row(
                ui,
                state,
                text_id::PRIMS_RADIUS,
                None,
                &mut radius,
                0.05..=100.0,
                0.05,
            );
            changed |= param_row_u32(
                ui,
                state,
                text_id::PRIMS_VERTICES,
                Some(text_id::PRIMS_TIP_VERTICES),
                &mut vertices,
                3..=64,
                1.0,
            );
            changed |= param_fill(ui, state, &mut fill);
            changed.then_some(PrimitiveDescriptor::Circle {
                radius,
                vertices,
                fill,
            })
        }
        PrimitiveDescriptor::Torus {
            major_radius,
            minor_radius,
            major_segments,
            minor_segments,
        } => {
            let (mut major_radius, mut minor_radius, mut major_segments, mut minor_segments) =
                (major_radius, minor_radius, major_segments, minor_segments);
            let mut changed = false;
            changed |= param_row(
                ui,
                state,
                text_id::PRIMS_MAJOR_RADIUS,
                Some(text_id::PRIMS_TIP_MAJOR_RADIUS),
                &mut major_radius,
                0.05..=100.0,
                0.05,
            );
            changed |= param_row(
                ui,
                state,
                text_id::PRIMS_MINOR_RADIUS,
                Some(text_id::PRIMS_TIP_MINOR_RADIUS),
                &mut minor_radius,
                0.01..=100.0,
                0.02,
            );
            changed |= param_row_u32(
                ui,
                state,
                text_id::PRIMS_SEGMENTS,
                Some(text_id::PRIMS_TIP_SEGMENTS),
                &mut major_segments,
                3..=64,
                1.0,
            );
            changed |= param_row_u32(
                ui,
                state,
                text_id::PRIMS_RINGS,
                Some(text_id::PRIMS_TIP_SEGMENTS),
                &mut minor_segments,
                3..=32,
                1.0,
            );
            changed.then_some(PrimitiveDescriptor::Torus {
                major_radius,
                minor_radius,
                major_segments,
                minor_segments,
            })
        }
        PrimitiveDescriptor::LowSphere {
            radius,
            segments,
            rings,
        } => {
            let (mut radius, mut segments, mut rings) = (radius, segments, rings);
            let mut changed = false;
            changed |= param_row(
                ui,
                state,
                text_id::PRIMS_RADIUS,
                None,
                &mut radius,
                0.05..=100.0,
                0.05,
            );
            changed |= param_row_u32(
                ui,
                state,
                text_id::PRIMS_SEGMENTS,
                Some(text_id::PRIMS_TIP_SEGMENTS),
                &mut segments,
                3..=32,
                1.0,
            );
            changed |= param_row_u32(
                ui,
                state,
                text_id::PRIMS_RINGS,
                Some(text_id::PRIMS_TIP_RINGS),
                &mut rings,
                2..=24,
                1.0,
            );
            changed.then_some(PrimitiveDescriptor::LowSphere {
                radius,
                segments,
                rings,
            })
        }
        PrimitiveDescriptor::Icosphere { radius, subdiv } => {
            let (mut radius, mut subdiv) = (radius, subdiv);
            let mut changed = false;
            changed |= param_row(
                ui,
                state,
                text_id::PRIMS_RADIUS,
                None,
                &mut radius,
                0.05..=100.0,
                0.05,
            );
            changed |= param_row_u32(
                ui,
                state,
                text_id::PRIMS_SUBDIV,
                Some(text_id::PRIMS_TIP_SUBDIV),
                &mut subdiv,
                0..=3,
                1.0,
            );
            changed.then_some(PrimitiveDescriptor::Icosphere { radius, subdiv })
        }
        PrimitiveDescriptor::Capsule {
            radius,
            height,
            radial_segments,
            cap_segments,
        } => {
            let (mut radius, mut height, mut radial_segments, mut cap_segments) =
                (radius, height, radial_segments, cap_segments);
            let mut changed = false;
            changed |= param_row(
                ui,
                state,
                text_id::PRIMS_RADIUS,
                None,
                &mut radius,
                0.05..=100.0,
                0.05,
            );
            changed |= param_row(
                ui,
                state,
                text_id::PRIMS_BODY_LENGTH,
                Some(text_id::PRIMS_TIP_BODY_LENGTH),
                &mut height,
                0.05..=100.0,
                0.05,
            );
            changed |= param_row_u32(
                ui,
                state,
                text_id::PRIMS_SEGMENTS,
                Some(text_id::PRIMS_TIP_SEGMENTS),
                &mut radial_segments,
                3..=32,
                1.0,
            );
            changed |= param_row_u32(
                ui,
                state,
                text_id::PRIMS_RINGS,
                Some(text_id::PRIMS_TIP_RINGS),
                &mut cap_segments,
                1..=8,
                1.0,
            );
            changed.then_some(PrimitiveDescriptor::Capsule {
                radius,
                height,
                radial_segments,
                cap_segments,
            })
        }
    }
}

/// Ação de teclado do cartão (§10.5/§58): Enter confirma, Esc cancela —
/// somente com o ponteiro sobre o cartão (pura, testável).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardKeyAction {
    Confirm,
    Cancel,
}

pub fn card_key_action(
    card_hovered: bool,
    enter_pressed: bool,
    escape_pressed: bool,
) -> Option<CardKeyAction> {
    if !card_hovered {
        return None;
    }
    if escape_pressed {
        Some(CardKeyAction::Cancel)
    } else if enter_pressed {
        Some(CardKeyAction::Confirm)
    } else {
        None
    }
}

/// Linha de parâmetro float com largura fixa e tooltip opcional.
fn param_row(
    ui: &mut Ui,
    state: &AppState,
    label: TextId,
    tip: Option<TextId>,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    speed: f32,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let label_resp = ui.label(
            egui::RichText::new(state.t_id(label))
                .size(11.0)
                .color(tokens::TEXT_SECONDARY),
        );
        if let Some(tip) = tip {
            label_resp.on_hover_text(state.t_id(tip));
        }
        if ui
            .add_sized(
                vec2(ui.available_width().clamp(40.0, 110.0), 20.0),
                egui::DragValue::new(value).range(range).speed(speed),
            )
            .changed()
        {
            changed = true;
        }
    });
    changed
}

/// Linha de parâmetro inteiro com largura fixa e tooltip opcional.
fn param_row_u32(
    ui: &mut Ui,
    state: &AppState,
    label: TextId,
    tip: Option<TextId>,
    value: &mut u32,
    range: std::ops::RangeInclusive<u32>,
    speed: f32,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let label_resp = ui.label(
            egui::RichText::new(state.t_id(label))
                .size(11.0)
                .color(tokens::TEXT_SECONDARY),
        );
        if let Some(tip) = tip {
            label_resp.on_hover_text(state.t_id(tip));
        }
        let mut v = *value as i64;
        if ui
            .add_sized(
                vec2(ui.available_width().clamp(40.0, 110.0), 20.0),
                egui::DragValue::new(&mut v)
                    .range(*range.start() as i64..=*range.end() as i64)
                    .speed(speed),
            )
            .changed()
        {
            *value = (v as u32).clamp(*range.start(), *range.end());
            changed = true;
        }
    });
    changed
}

/// Dropdown de tampas (Both/Top/Bottom/None) com largura local (§63).
fn param_caps(
    ui: &mut Ui,
    state: &mut AppState,
    cap_top: &mut bool,
    cap_bottom: &mut bool,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(state.t_id(text_id::PRIMS_CAP))
                .size(11.0)
                .color(tokens::TEXT_SECONDARY),
        )
        .on_hover_text(state.t_id(text_id::PRIMS_TIP_CAPS));
        let current = match (*cap_top, *cap_bottom) {
            (true, true) => text_id::PRIMS_CAP_BOTH,
            (true, false) => text_id::PRIMS_CAP_TOP,
            (false, true) => text_id::PRIMS_CAP_BOTTOM,
            (false, false) => text_id::PRIMS_CAP_NONE,
        };
        egui::ComboBox::from_id_salt("primitive_cap_combo")
            .width(ui.available_width().clamp(64.0, 160.0))
            .selected_text(state.t_id(current))
            .show_ui(ui, |ui| {
                for (id, top, bottom) in [
                    (text_id::PRIMS_CAP_BOTH, true, true),
                    (text_id::PRIMS_CAP_TOP, true, false),
                    (text_id::PRIMS_CAP_BOTTOM, false, true),
                    (text_id::PRIMS_CAP_NONE, false, false),
                ] {
                    if ui
                        .selectable_value(
                            &mut (*cap_top, *cap_bottom),
                            (top, bottom),
                            state.t_id(id),
                        )
                        .clicked()
                    {
                        changed = true;
                    }
                }
            });
    });
    changed
}

/// Dropdown de preenchimento do círculo (None/Disc) com largura local.
fn param_fill(ui: &mut Ui, state: &mut AppState, fill: &mut CircleFill) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(state.t_id(text_id::PRIMS_FILL))
                .size(11.0)
                .color(tokens::TEXT_SECONDARY),
        )
        .on_hover_text(state.t_id(text_id::PRIMS_TIP_FILL));
        egui::ComboBox::from_id_salt("primitive_fill_combo")
            .width(ui.available_width().clamp(64.0, 160.0))
            .selected_text(match fill {
                CircleFill::None => state.t_id(text_id::PRIMS_FILL_NONE),
                CircleFill::Disc => state.t_id(text_id::PRIMS_FILL_DISC),
            })
            .show_ui(ui, |ui| {
                if ui
                    .selectable_value(fill, CircleFill::None, state.t_id(text_id::PRIMS_FILL_NONE))
                    .clicked()
                {
                    changed = true;
                }
                if ui
                    .selectable_value(fill, CircleFill::Disc, state.t_id(text_id::PRIMS_FILL_DISC))
                    .clicked()
                {
                    changed = true;
                }
            });
    });
    changed
}

/// Estatísticas leves da criação (§25).
fn draw_creation_stats(ui: &mut Ui, state: &AppState, asset_id: uuid::Uuid) {
    if let Some(asset) = state.project.assets.iter().find(|a| a.id == asset_id) {
        let audit = primitive_audit(&asset.mesh);
        ui.label(
            egui::RichText::new(format!(
                "{} {} · {} {} · {} {}",
                audit.verts,
                state.t("props.verts"),
                audit.faces,
                state.t("props.faces"),
                audit.tris,
                state.t_id(text_id::PRIMS_TRIS),
            ))
            .size(10.5)
            .color(tokens::TEXT_MUTED),
        );
    }
}

/// Âncora do cartão: canto projetado do bounding box (cai para o topo da
/// viewport se a primitiva estiver fora da vista).
fn creation_anchor(state: &AppState, viewport_rect: Rect, asset_id: uuid::Uuid) -> Pos2 {
    let to_screen = |w: glam::Vec3| {
        let ndc = state.camera.project_ndc(w);
        pos2(
            viewport_rect.min.x + (ndc.x * 0.5 + 0.5) * viewport_rect.width(),
            viewport_rect.min.y + (1.0 - (ndc.y * 0.5 + 0.5)) * viewport_rect.height(),
        )
    };
    if let Some(asset) = state.project.assets.iter().find(|a| a.id == asset_id) {
        let visible: Vec<Pos2> = asset
            .mesh
            .verts
            .iter()
            .map(|v| to_screen(v.vec()))
            .filter(|p| viewport_rect.expand(200.0).contains(*p))
            .collect();
        if !visible.is_empty() {
            let max_x = visible.iter().map(|p| p.x).fold(f32::MIN, f32::max);
            let min_y = visible.iter().map(|p| p.y).fold(f32::MAX, f32::min);
            return pos2(max_x + 12.0, min_y - 12.0);
        }
    }
    pos2(viewport_rect.center().x, viewport_rect.min.y + 40.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_renders_without_panic_during_session() {
        let ctx = egui::Context::default();
        let mut state = AppState::new("en");
        assert!(state.begin_primitive(petunia_core::PrimitiveKind::Cube, None));
        let viewport = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1280.0, 800.0));
        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let _ = draw_primitive_card(ui, &mut state, viewport);
            });
        })
        .textures_delta
        .clear();
        assert!(state.primitive_session_valid());
    }

    #[test]
    fn anchor_falls_back_when_asset_hidden() {
        let mut state = AppState::new("en");
        assert!(state.begin_primitive(petunia_core::PrimitiveKind::Cube, None));
        let viewport = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1280.0, 800.0));
        let id = state.session.primitive_session.as_ref().unwrap().asset_id;
        let anchor = creation_anchor(&state, viewport, id);
        assert!(viewport.contains(anchor) || anchor.y >= viewport.min.y);
    }

    #[test]
    fn card_keys_require_hover() {
        assert_eq!(card_key_action(false, true, true), None);
        assert_eq!(
            card_key_action(true, false, true),
            Some(CardKeyAction::Cancel)
        );
        assert_eq!(
            card_key_action(true, true, false),
            Some(CardKeyAction::Confirm)
        );
        assert_eq!(card_key_action(true, false, false), None);
        // Esc tem precedência sobre Enter simultâneo.
        assert_eq!(
            card_key_action(true, true, true),
            Some(CardKeyAction::Cancel)
        );
    }

    #[test]
    fn card_renders_every_species_without_panic() {
        use petunia_core::PrimitiveKind as K;
        for (i, kind) in [
            K::Cube,
            K::Plane,
            K::Wedge,
            K::Cylinder,
            K::Cone,
            K::Circle,
            K::Torus,
            K::Sphere,
            K::Icosphere,
            K::Capsule,
        ]
        .into_iter()
        .enumerate()
        {
            let ctx = egui::Context::default();
            // Alterna en/pt-BR: rótulos longos não podem quebrar o cartão.
            let mut state = AppState::new(if i % 2 == 0 { "en" } else { "pt-BR" });
            assert!(state.begin_primitive(kind, None), "{kind:?}");
            let viewport =
                egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1280.0, 800.0));
            ctx.run_ui(egui::RawInput::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let _ = draw_primitive_card(ui, &mut state, viewport);
                });
            })
            .textures_delta
            .clear();
            assert!(state.confirm_primitive(), "{kind:?}");
        }
    }
}
