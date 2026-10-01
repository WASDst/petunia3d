//! Petunia3D UI (egui): header com pílulas de workspace, toolbar,
//! Asset Library, painéis por workspace, viewport com overlays e status.
//!
//! Regra de borrow: rótulos `state.t()` (owned String) são pré-calculados
//! antes dos closures; mutações via índices, nunca com iterator vivo.

use petunia_config::text_id;
use petunia_core::Projection;
use petunia_core::{AppState, ModuleRegistry, ProjectService, RefAxis, Workspace};
use petunia_module_model::ToolRegistry;
use petunia_project::export;

pub mod adapters;
pub mod annotation;
pub mod asset_browser;
pub mod asset_library_drawer;
pub mod camera_controls;
pub mod command_palette;
pub mod contextual_shelf;
mod cutting;
#[cfg(feature = "devtools")]
pub mod devtools;
pub mod file_dialog_service;
pub mod foundation;
/// Vitrine executável dos componentes (`cargo run -p petunia_ui --example component_gallery`).
/// Não é caminho de produto: nenhum painel do shell importa este módulo.
pub mod gallery;
pub mod gizmo;
#[cfg(feature = "help-markdown")]
pub mod help_markdown;
pub mod icon_provider;
pub mod icon_registry;
pub mod icons;
pub mod image_kit;
pub mod inspector_context;
pub mod inspector_widgets;
#[cfg(feature = "keymap-capture")]
pub mod key_capture;
pub mod main_header;
pub mod measurement;
#[cfg(test)]
mod modal_tests;
mod modal_viewport;
pub mod modeling_tool_properties;
pub mod modules_ui;
pub mod nav_gizmo;
pub mod outliner;
#[cfg(feature = "palette-autocomplete")]
pub mod palette_complete;
pub mod primitive_card;
pub mod properties_panel;
pub mod recovery_dialog;
pub mod reference_manager;
pub mod regions;
pub mod settings_modal;
/// Composição do shell por regiões (Wave 5b — §31, §47).
pub mod shell;
pub mod status_bar;
#[cfg(feature = "animation-workspace")]
pub mod timeline;
pub mod tokens;
pub mod tool_fields;
pub mod tool_properties_popover;
pub mod toolbar;
pub mod transform_gizmo_integration;
pub mod viewport_bar;
mod viewport_interaction;
pub mod widgets;
pub mod workspaces;

pub use recovery_dialog::{RecoveryAction, draw as draw_recovery_dialog};
pub use tokens::apply_theme_to_egui;

/// Canonical entrypoint for the right dock/panel as indexed in `ui-map.json`.
/// Ponto de entrada canônico para o dock/painel direito conforme indexado no `ui-map.json`.
pub fn right_panel(
    ui: &mut egui::Ui,
    state: &mut AppState,
    tools: &ToolRegistry,
    registry: &mut ModuleRegistry,
) {
    properties_panel::draw(ui, state, tools, registry);
}

/// Canonical entrypoint for the split dock as indexed in `ui-map.json`.
/// Ponto de entrada canônico para o dock dividido conforme indexado no `ui-map.json`.
pub fn draw_split_dock(
    ui: &mut egui::Ui,
    state: &mut AppState,
    tools: &ToolRegistry,
    registry: &mut ModuleRegistry,
) {
    properties_panel::draw(ui, state, tools, registry);
}

/// Canonical entrypoint for the animate workspace center strip as indexed in `ui-map.json`.
/// Ponto de entrada canônico para o centro do workspace Animate conforme indexado no `ui-map.json`.
pub fn animate_workspace_center(ui: &mut egui::Ui, state: &mut AppState) {
    let rect = ui.available_rect_before_wrap();
    viewport_3d(ui, state, rect);
}

pub fn rect_to_logical(r: egui::Rect) -> petunia_core::viewport::LogicalRect {
    petunia_core::viewport::LogicalRect::from_min_max([r.min.x, r.min.y], [r.max.x, r.max.y])
}

pub fn logical_to_rect(r: petunia_core::viewport::LogicalRect) -> egui::Rect {
    egui::Rect::from_min_max(
        egui::pos2(r.left(), r.top()),
        egui::pos2(r.right(), r.bottom()),
    )
}

static REF_TEXTURES: std::sync::Mutex<
    Option<std::collections::HashMap<String, egui::TextureHandle>>,
> = std::sync::Mutex::new(None);

/// Textura egui de uma imagem de referência (Wave 5 — §9.5).
///
/// Chave inclui dimensões + tamanho do buffer (troca de imagem reaproveita o
/// slot sem exibir pixels obsoletos) e entradas de refs removidas são podadas
/// para não vazar VRAM de UI.
pub fn get_ref_texture(
    ctx: &egui::Context,
    img: &petunia_core::ReferenceImage,
    live: &[petunia_core::ReferenceImage],
) -> egui::TextureHandle {
    let key = format!(
        "{}|{}x{}#{}",
        img.name,
        img.width,
        img.height,
        img.rgba.len()
    );
    let mut lock = REF_TEXTURES.lock().unwrap();
    let map = lock.get_or_insert_with(std::collections::HashMap::new);
    let live_keys: std::collections::HashSet<String> = live
        .iter()
        .map(|r| format!("{}|{}x{}#{}", r.name, r.width, r.height, r.rgba.len()))
        .collect();
    map.retain(|k, _| live_keys.contains(k));
    if let Some(handle) = map.get(&key) {
        return handle.clone();
    }
    let color_img = egui::ColorImage::from_rgba_unmultiplied(
        [img.width as usize, img.height as usize],
        &img.rgba,
    );
    let handle = ctx.load_texture(&key, color_img, egui::TextureOptions::LINEAR);
    map.insert(key, handle.clone());
    handle
}

pub struct UiAction {
    pub quit: bool,
}

impl UiAction {
    pub fn none() -> Self {
        Self { quit: false }
    }
}

pub fn draw(
    ui: &mut egui::Ui,
    state: &mut AppState,
    tools: &ToolRegistry,
    registry: &mut ModuleRegistry,
    action: &mut UiAction,
) {
    let theme_id_key = egui::Id::new("petunia_applied_theme_id");
    let needs_theme_update = ui.ctx().data(|d| {
        d.get_temp::<String>(theme_id_key)
            .map(|id| id != state.ui.active_theme_id)
            .unwrap_or(true)
    });
    if needs_theme_update {
        let theme_reg = petunia_config::ThemeRegistry::global();
        if let Some(t) = theme_reg.get_theme(&state.ui.active_theme_id) {
            tokens::apply_theme_to_egui(t, ui.ctx());
            ui.ctx().data_mut(|d| {
                d.insert_temp(theme_id_key, state.ui.active_theme_id.clone());
            });
        }
    }

    icon_registry::IconRegistry::ensure_fonts(ui.ctx());
    ui.ctx().data_mut(|d| {
        d.insert_temp(
            egui::Id::new("petunia_active_icon_pack"),
            state.ui.active_icon_pack_id.clone(),
        );
    });
    // Wave 2: reseta as regiões do shell; cada painel registra a sua ao desenhar.
    regions::reset(ui.ctx());

    // O chrome (header, status e barra de assets) é de largura total e continua
    // em painéis próprios; o macro-layout (paleta · centro · dock · faixa
    // inferior) é resolvido pela árvore do `PetuniaLayoutAdapter` (Wave 5b).
    main_header::draw(ui, state, action);
    status_bar::draw(ui, state, tools);
    asset_browser::draw(ui, state);
    shell::draw(ui, state, tools, registry);
    let ctx = ui.ctx().clone();
    asset_library_drawer::draw(&ctx, state);
    settings_modal::draw(&ctx, state);
    command_palette::draw(&ctx, state);
    reference_manager::draw(&ctx, state);
    file_dialog_service::draw(&ctx, state);
}

/// Barra de contexto da viewport (`ViewportBar`, §46).
///
/// Vive dentro do paine central (Wave 5b): o `Panel::top` é relativo ao `Ui` do
/// tile, então a barra nunca atravessa o dock nem a paleta.
pub(crate) fn viewport_bar_panel(ui: &mut egui::Ui, state: &mut AppState) {
    let bar = egui::Panel::top("viewport_context_bar")
        .default_size(tokens::VIEWPORT_BAR_HEIGHT)
        .size_range(tokens::VIEWPORT_BAR_HEIGHT..=tokens::VIEWPORT_BAR_MAX_HEIGHT)
        .resizable(true)
        .frame(
            egui::Frame::new()
                .fill(tokens::bg_panel_header(state))
                .stroke(tokens::stroke_border_dyn(state))
                .inner_margin(egui::Margin::symmetric(6, 2)),
        )
        .show(ui, |ui| {
            ui.add_enabled_ui(!state.is_interacting(), |ui| {
                viewport_bar::draw(ui, state);
            });
        });
    regions::record(
        ui.ctx(),
        regions::RegionSlot::ViewportToolbar,
        bar.response.rect,
    );
}

pub fn new_project(state: &mut AppState) {
    ProjectService::new_project(state);
}

pub fn open_project_dialog(_state: &mut AppState) {
    file_dialog_service::open_project_in_canvas();
}

pub fn save_project_dialog(state: &mut AppState, save_as: bool) {
    if !save_as && let Some(ref path) = state.project.project_path {
        let p = std::path::PathBuf::from(path);
        if let Err(e) = ProjectService::save_project(state, &p) {
            state.set_status(format!("save err: {e}"));
        }
    } else {
        file_dialog_service::save_project_as_in_canvas();
    }
}

pub fn import_obj_dialog(_state: &mut AppState) {
    file_dialog_service::import_obj_in_canvas();
}

pub fn import_gltf_dialog(_state: &mut AppState) {
    file_dialog_service::import_gltf_in_canvas();
}

pub fn frame_selection(state: &mut AppState) {
    if let Some(o) = state.project.assets.get(state.project.active) {
        let mut c = glam::Vec3::ZERO;
        let mut n = 0;
        let mut r: f32 = 0.0;
        for v in &o.mesh.verts {
            if v.selected {
                c += v.vec();
                n += 1;
            }
        }
        if n == 0 {
            for v in &o.mesh.verts {
                c += v.vec();
            }
            n = o.mesh.verts.len().max(1);
        }
        c /= n as f32;
        let has_selection = o.mesh.verts.iter().any(|v| v.selected);
        for v in &o.mesh.verts {
            if !has_selection || v.selected {
                r = r.max((v.vec() - c).length());
            }
        }
        let mut goal = state.camera.clone();
        goal.frame(c, r.max(0.05));
        state.camera_frame = Some((state.camera.clone(), goal, 0.0));
        state.mark_dirty();
    }
}

pub fn refs_section(ui: &mut egui::Ui, state: &mut AppState) {
    let l_refs = state.t_id(text_id::UI_REFS);
    let l_load = state.t("refs.load");
    let l_offset = state.t_id(text_id::REFS_OFFSET);
    let l_size = state.t_id(text_id::REFS_SIZE);
    let l_opacity = state.t_id(text_id::REFS_OPACITY);
    let l_rotation = state.t_id(text_id::REFS_ROTATION);
    let l_xray = state.t("refs.xray");
    let l_front = state.t(RefAxis::Front.key());
    let l_back = state.t(RefAxis::Back.key());
    let l_left = state.t(RefAxis::Left.key());
    let l_right = state.t(RefAxis::Right.key());
    let l_top = state.t(RefAxis::Top.key());
    let l_bottom = state.t(RefAxis::Bottom.key());
    let ctx = ui.ctx().clone();
    egui::CollapsingHeader::new(l_refs)
        .default_open(false)
        .show(ui, |ui| {
            if ui.button(l_load).clicked()
                && let Some(path) = file_dialog_service::pick_image_file()
            {
                match load_image_rgba(&path) {
                    Ok((w, h, rgba)) => {
                        let name = path
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("ref")
                            .to_string();
                        ProjectService::add_reference_image(state, name, w, h, rgba);
                    }
                    Err(e) => state.set_status(format!("ref err: {e}")),
                }
            }
            let mut rm: Option<usize> = None;
            let n = state.project.refs.len();
            for i in 0..n {
                let (tex_id, aspect) = {
                    let r = &state.project.refs[i];
                    let tex = get_ref_texture(&ctx, r, &state.project.refs);
                    (Some(tex.id()), r.height as f32 / r.width.max(1) as f32)
                };
                {
                    let r = &mut state.project.refs[i];
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut r.visible, "");
                        let mut lock = r.locked;
                        if ui.checkbox(&mut lock, "Lock").changed() {
                            r.locked = lock;
                        }
                        ui.label(&r.name);
                        if ui.small_button("×").clicked() {
                            rm = Some(i);
                        }
                    });
                }
                if let Some(tid) = tex_id {
                    ui.image((tid, egui::vec2(230.0, 230.0 * aspect)));
                }
                {
                    let r = &mut state.project.refs[i];
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(r.axis == RefAxis::Front, format!("F {l_front}"))
                            .clicked()
                        {
                            r.axis = RefAxis::Front;
                        }
                        if ui
                            .selectable_label(r.axis == RefAxis::Back, format!("B {l_back}"))
                            .clicked()
                        {
                            r.axis = RefAxis::Back;
                        }
                        if ui
                            .selectable_label(r.axis == RefAxis::Left, format!("L {l_left}"))
                            .clicked()
                        {
                            r.axis = RefAxis::Left;
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(
                                r.axis == RefAxis::Right || r.axis == RefAxis::Side,
                                format!("R {l_right}"),
                            )
                            .clicked()
                        {
                            r.axis = RefAxis::Right;
                        }
                        if ui
                            .selectable_label(r.axis == RefAxis::Top, format!("T {l_top}"))
                            .clicked()
                        {
                            r.axis = RefAxis::Top;
                        }
                        if ui
                            .selectable_label(r.axis == RefAxis::Bottom, format!("D {l_bottom}"))
                            .clicked()
                        {
                            r.axis = RefAxis::Bottom;
                        }
                    });
                    let mut ch = false;
                    ch |= ui
                        .add(egui::Slider::new(&mut r.offset, -10.0..=10.0).text(&l_offset))
                        .changed();
                    ch |= ui
                        .add(egui::Slider::new(&mut r.size, 0.5..=12.0).text(&l_size))
                        .changed();
                    ch |= ui
                        .add(egui::Slider::new(&mut r.opacity, 0.05..=1.0).text(&l_opacity))
                        .changed();
                    ch |= ui
                        .add(
                            egui::Slider::new(&mut r.rotation, -180.0..=180.0)
                                .suffix("°")
                                .text(&l_rotation),
                        )
                        .changed();
                    ch |= ui.checkbox(&mut r.xray, &l_xray).changed();
                    if ch {
                        state.mark_dirty();
                    }
                }
                ui.separator();
            }
            if let Some(i) = rm {
                state.project.refs.remove(i);
                state.mark_dirty();
            }
        });
}

pub fn export_section(ui: &mut egui::Ui, state: &mut AppState) {
    let l_exp = state.t("export.title");
    let l_fmt = state.t("export.format");
    let l_report = state.t("export.report");
    let l_go = state.t("export.go");
    egui::CollapsingHeader::new(l_exp)
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(l_fmt);
                if ui
                    .selectable_label(!state.project.export_gltf, "OBJ")
                    .clicked()
                {
                    state.project.export_gltf = false;
                }
                if ui
                    .selectable_label(state.project.export_gltf, "GLB")
                    .clicked()
                {
                    state.project.export_gltf = true;
                }
            });
            // multi-select
            let mut sel_ids = state.project.export_selected.clone();
            if sel_ids.is_empty() {
                sel_ids = state.project.assets.iter().map(|a| a.id).collect();
            }
            let mut new_sel_ids = Vec::new();
            let mut new_sel_indices = Vec::new();
            let asset_items: Vec<(usize, uuid::Uuid, String)> = state
                .project
                .assets
                .iter()
                .enumerate()
                .map(|(i, a)| (i, a.id, a.name.clone()))
                .collect();
            let mut changed = false;
            for (i, asset_id, name) in asset_items {
                let mut on = sel_ids.contains(&asset_id);
                if ui.checkbox(&mut on, &name).changed() {
                    changed = true;
                }
                if on {
                    new_sel_ids.push(asset_id);
                    new_sel_indices.push(i);
                }
            }
            if changed {
                state.mark_dirty();
            }
            state.project.export_selected = new_sel_ids;
            ui.separator();
            ui.label(l_report);
            for line in
                export::export_report(&state.project, &new_sel_indices, state.project.export_gltf)
            {
                ui.small(line);
            }
            if ui.button(l_go).clicked() {
                export_dialog(state, &new_sel_indices);
            }
        });
}

pub fn export_active_or_all(state: &mut AppState, glb: bool) {
    let sel = state.project.export_selected_indices();
    state.project.export_gltf = glb;
    export_dialog(state, &sel);
}

fn export_dialog(state: &mut AppState, sel: &[usize]) {
    if sel.is_empty() {
        state.set_status(state.t("export.empty"));
        return;
    }
    if state.project.export_gltf {
        if let Some(path) = file_dialog_service::pick_export_glb_file("assets.glb") {
            match ProjectService::export_glb(state, sel, &path) {
                Ok(()) => state.set_status(format!("export {}", path.display())),
                Err(e) => state.set_status(format!("export err: {e}")),
            }
        }
    } else if let Some(dir) = file_dialog_service::pick_folder() {
        match ProjectService::export_all_obj_to_dir(state, sel, &dir) {
            Ok(n) => state.set_status(format!("export: {n} OBJ")),
            Err(e) => state.set_status(format!("export err: {e}")),
        }
    }
}

// ------------------------------------------------------------- viewport

/// Centro do workspace: viewport 3D ou editor UV + prévia.
///
/// Wave 5b: a faixa inferior (Timeline) saiu daqui — ela é o paine `Bottom` da
/// árvore do shell, e a viewport fica com toda a altura restante do paine.
///
/// FUNDO TRANSPARENTE: o 3D é desenhado por baixo (wgpu/GL) e o egui compõe por
/// cima. Um fill opaco aqui ESCONDE a cena inteira.
pub(crate) fn viewport(ui: &mut egui::Ui, state: &mut AppState) {
    puffin::profile_function!();
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(egui::Color32::TRANSPARENT))
        .show(ui, |ui| {
            let rect = ui.available_rect_before_wrap();
            viewport_3d(ui, state, rect);
        });
}

// O centro do PAINT (viewport 3D + tela 2D lado a lado) **não** é composto
// aqui: ele é o paine `PetuniaPane::PaintCanvas` do `adapters::tile_layout`
// (§31), que é dono da divisória, dos mínimos das duas superfícies e da largura
// persistida. O produto só declara o perfil do workspace
// (`workspaces::profile_for(..).paints_on_canvas()`) e desenha cada paine.

/// Centro UV legado mantido apenas para compatibilidade de código antigo; não é
/// alcançável pela UI V1.
///
/// Janela larga: lado a lado. Janela estreita (<760px): alternador, nunca os
/// dois esmagados. A prévia 3D registra o `viewport_rect` que posiciona a
/// superfície GPU — nenhum acoplamento novo com o app.
#[allow(dead_code)]
fn uv_workspace_center(ui: &mut egui::Ui, state: &mut AppState) {
    puffin::profile_function!();
    let total = ui.available_rect_before_wrap();
    if total.width() < 760.0 {
        ui.horizontal(|ui| {
            let editor_label = state.t_id(text_id::UV_TITLE);
            let preview_label = state.t_id(text_id::UV_PREVIEW_3D);
            if ui
                .selectable_label(!state.ui.uv_show_preview, editor_label)
                .clicked()
            {
                state.ui.uv_show_preview = false;
                state.mark_dirty();
            }
            if ui
                .selectable_label(state.ui.uv_show_preview, preview_label)
                .clicked()
            {
                state.ui.uv_show_preview = true;
                state.mark_dirty();
            }
        });
        ui.separator();
        if state.ui.uv_show_preview {
            viewport_3d(ui, state, ui.available_rect_before_wrap());
        } else {
            let pane = ui.available_rect_before_wrap();
            regions::record(ui.ctx(), regions::RegionSlot::UvEditor, pane);
            egui::ScrollArea::vertical()
                .id_salt("uv_editor_narrow_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    modules_ui::uv_ui::draw_uv_panel(ui, state);
                });
        }
    } else {
        let gap = ui.spacing().item_spacing.x;
        let left_w = (total.width() * 0.45).clamp(320.0, 560.0);
        let left = ui.allocate_ui_with_layout(
            egui::vec2(left_w, total.height()),
            egui::Layout::top_down_justified(egui::Align::LEFT),
            |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("uv_editor_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        modules_ui::uv_ui::draw_uv_panel(ui, state);
                    });
            },
        );
        regions::record(ui.ctx(), regions::RegionSlot::UvEditor, left.response.rect);
        let right_rect = egui::Rect::from_min_max(
            egui::pos2(left.response.rect.max.x + gap, total.min.y),
            total.max,
        );
        viewport_3d(ui, state, right_rect);
    }
}

fn viewport_3d(ui: &mut egui::Ui, state: &mut AppState, rect: egui::Rect) {
    puffin::profile_function!();
    {
        let ctx = ui.ctx().clone();
        regions::record(&ctx, regions::RegionSlot::Viewport, rect);
        state.ui.viewport_rect = Some(rect_to_logical(rect));
        state.ui.viewport_pixels_per_point = ctx.pixels_per_point();
        state.camera.aspect = rect.width() / rect.height().max(1.0);
        let p = ui.painter_at(rect);
        // mira central
        p.circle_stroke(
            rect.center(),
            4.0,
            egui::Stroke::new(1.0_f32, egui::Color32::from_gray(120)),
        );
        // overlay do Draw Profile
        if !state.profile.points.is_empty() {
            draw_profile_overlay(&p, rect, state);
        }
        // indicador de vista ortográfica
        if state.camera.proj == Projection::Ortho {
            p.text(
                rect.min + egui::vec2(8.0, 8.0),
                egui::Align2::LEFT_TOP,
                "ORTHO",
                egui::FontId::monospace(11.0),
                egui::Color32::LIGHT_BLUE,
            );
        }
        let resp = ui.allocate_rect(rect, egui::Sense::click_and_drag());
        // No PAINT a interação desenha a pincelada e reivindica a cena. O cartão
        // da ferramenta não é cena, é chrome do usuário: nasce **antes** da
        // interação para (a) continuar visível durante o traço e (b) publicar o
        // retângulo medido que impede a pincelada de começar sob os controles.
        let paint_viewport = state.workspace == Workspace::Paint;
        let paint_chrome = if paint_viewport {
            tool_properties_popover::draw(ui, state, rect)
        } else {
            None
        };
        if let Some(chrome) = paint_chrome {
            regions::record(&ctx, regions::RegionSlot::ToolProperties, chrome);
        }
        if viewport_interaction::draw(&ctx, state, rect, &p, &resp) && !paint_viewport {
            return;
        }

        // Barra contextual horizontal flutuante na base da viewport.
        // Posicionada dentro do `rect` da viewport (nunca da tela global).
        // Respeita a preferência `show_shelf` (Settings → Interface).
        let shelf_rect = if state.ui.show_shelf {
            contextual_shelf::draw(ui, state, rect)
        } else {
            None
        };
        if let Some(shelf) = shelf_rect {
            regions::record(&ctx, regions::RegionSlot::Shelf, shelf);
        }
        // Viewport-local interactive chrome. Every surface returns its actual
        // rect, which becomes both QA evidence and a hit-test exclusion zone.
        let had_primitive_session = state.session.primitive_session.is_some();
        let primitive_card_rect = primitive_card::draw_primitive_card(ui, state, rect);
        if let Some(card_rect) = primitive_card_rect {
            regions::record(&ctx, regions::RegionSlot::PrimitiveCard, card_rect);
        }
        // Avoid flashing a second contextual surface on the same frame that a
        // primitive card confirms/cancels itself.
        let tool_properties_rect = if paint_viewport {
            paint_chrome
        } else if had_primitive_session {
            None
        } else {
            tool_properties_popover::draw(ui, state, rect)
        };
        if let Some(tool_rect) = tool_properties_rect {
            regions::record(&ctx, regions::RegionSlot::ToolProperties, tool_rect);
        }

        let pointer_on_viewport_chrome = ui.input(|i| {
            i.pointer
                .interact_pos()
                .or(i.pointer.hover_pos())
                .is_some_and(|pos| {
                    [shelf_rect, primitive_card_rect, tool_properties_rect]
                        .into_iter()
                        .flatten()
                        .any(|overlay| overlay.contains(pos))
                })
        });

        if pointer_on_viewport_chrome {
            // Critical Wave 3 invariant: buttons, fields and scroll gestures in
            // viewport chrome must never become box-select starts or GPU picks.
            state.ui.box_select_start = None;
            return;
        }
        if state.active_tool == "select_box"
            && resp.drag_started_by(egui::PointerButton::Primary)
            && let Some(pos) = resp.interact_pointer_pos()
        {
            state.ui.box_select_start = Some([pos.x, pos.y]);
        }
        if state.active_tool == "select_box"
            && resp.dragged_by(egui::PointerButton::Primary)
            && let (Some(start), Some(curr)) =
                (state.ui.box_select_start, resp.interact_pointer_pos())
        {
            let r = egui::Rect::from_two_pos(egui::pos2(start[0], start[1]), curr);
            p.rect_filled(
                r,
                0.0,
                egui::Color32::from_rgba_unmultiplied(255, 160, 40, 40),
            );
            p.rect_stroke(
                r,
                0.0,
                egui::Stroke::new(1.0f32, egui::Color32::from_rgb(255, 160, 40)),
                egui::StrokeKind::Outside,
            );
            state.mark_dirty();
        }
        if state.active_tool == "select_box"
            && resp.drag_stopped_by(egui::PointerButton::Primary)
            && let (Some(start), Some(curr)) = (
                state.ui.box_select_start.take(),
                resp.interact_pointer_pos(),
            )
        {
            let dx = (curr.x - start[0]).abs();
            let dy = (curr.y - start[1]).abs();
            if dx > 8.0 || dy > 8.0 {
                let to_ndc = |pos: egui::Pos2| -> [f32; 2] {
                    let nx = ((pos.x - rect.min.x) / rect.width().max(1.0)) * 2.0 - 1.0;
                    let ny = 1.0 - ((pos.y - rect.min.y) / rect.height().max(1.0)) * 2.0;
                    [nx, ny]
                };
                let p0 = to_ndc(egui::pos2(start[0], start[1]));
                let p1 = to_ndc(curr);
                let shift = ui.input(|i| i.modifiers.shift);
                let vp = state.session.camera.view_proj().to_cols_array();
                let _ = state.dispatch(&petunia_core::BoxSelectCmd {
                    p0,
                    p1,
                    view_proj: vp,
                    add: shift,
                });
            }
        }
        if resp.clicked() {
            // Clique fora confirma a criação ativa (Wave 8 — §10.5).
            if had_primitive_session {
                state.confirm_primitive();
                state.ui.box_select_start = None;
            } else if let Some(pos) = resp.interact_pointer_pos() {
                let nx = ((pos.x - rect.min.x) / rect.width().max(1.0)) * 2.0 - 1.0;
                let ny = 1.0 - ((pos.y - rect.min.y) / rect.height().max(1.0)) * 2.0;
                state.ui.pending_pick = Some((nx, ny));
                state.mark_dirty();
            }
        }
    }
}

fn draw_profile_overlay(p: &egui::Painter, rect: egui::Rect, state: &AppState) {
    let to_screen = |w: glam::Vec3| {
        let ndc = state.camera.project_ndc(w);
        egui::pos2(
            rect.min.x + (ndc.x * 0.5 + 0.5) * rect.width(),
            rect.min.y + (1.0 - (ndc.y * 0.5 + 0.5)) * rect.height(),
        )
    };
    let pts: Vec<egui::Pos2> = (0..state.profile.points.len())
        .map(|i| to_screen(state.profile.to_3d(i)))
        .collect();
    for w in pts.windows(2) {
        p.line_segment(
            [w[0], w[1]],
            egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(255, 170, 60)),
        );
    }
    if state.profile.closed && pts.len() >= 3 {
        p.line_segment(
            [pts[pts.len() - 1], pts[0]],
            egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(255, 170, 60)),
        );
        // preview da triangulação
        if let Ok(tris) = petunia_mesh::triangulate::ear_clip(&state.profile.points) {
            for t in tris {
                for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                    if a < pts.len() && b < pts.len() {
                        p.line_segment(
                            [pts[a], pts[b]],
                            egui::Stroke::new(0.75_f32, egui::Color32::from_rgb(90, 200, 255)),
                        );
                    }
                }
            }
        }
    }
    for q in &pts {
        p.circle_filled(*q, 3.5, egui::Color32::from_rgb(255, 170, 60));
    }
}

pub fn load_image_rgba(path: &std::path::Path) -> Result<(u32, u32, Vec<u8>), String> {
    let img = image::open(path).map_err(|e| e.to_string())?;
    let mut rgba = img.to_rgba8();
    const MAX: u32 = 2048;
    if rgba.width() > MAX || rgba.height() > MAX {
        let (w, h) = (rgba.width(), rgba.height());
        let s = (MAX as f32 / w.max(h) as f32).min(1.0);
        rgba = image::imageops::resize(
            &rgba,
            ((w as f32 * s) as u32).max(1),
            ((h as f32 * s) as u32).max(1),
            image::imageops::FilterType::Lanczos3,
        );
    }
    Ok((rgba.width(), rgba.height(), rgba.into_raw()))
}

/// Diálogo para selecionar e adicionar uma imagem de referência à cena via in-canvas picker.
pub fn pick_and_add_reference_image(_state: &mut AppState) {
    file_dialog_service::open_reference_image_dialog(None);
}

#[cfg(test)]
mod paint_tests;

#[cfg(test)]
mod cutting_tests;

#[cfg(test)]
mod shell_layout_tests {
    //! Regressão do shell: nenhum painel lateral cobre a status bar em
    //! nenhuma resolução suportada (zonas rígidas da `status_bar`).
    use super::*;
    use petunia_core::{DockOrientation, DockSide};

    fn draw_shell(w: f32, h: f32) -> Option<regions::UiRegions> {
        draw_shell_with(w, h, true)
    }

    fn draw_shell_with(w: f32, h: f32, assets: bool) -> Option<regions::UiRegions> {
        draw_shell_full(w, h, assets, DockSide::Right, DockOrientation::Stacked)
    }

    fn draw_shell_full(
        w: f32,
        h: f32,
        assets: bool,
        side: DockSide,
        orient: DockOrientation,
    ) -> Option<regions::UiRegions> {
        let ctx = egui::Context::default();
        let mut state = AppState::new("en");
        state.ui.show_asset_browser = assets;
        state.ui.dock_side = side;
        state.ui.dock_orientation = orient;
        let tools = ToolRegistry::default();
        let mut registry = ModuleRegistry::new();
        let mut action = UiAction::none();
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(w, h),
            )),
            ..Default::default()
        };
        ctx.run_ui(raw, |ui| {
            draw(ui, &mut state, &tools, &mut registry, &mut action);
        })
        .textures_delta
        .clear();
        regions::load(&ctx)
    }

    #[test]
    fn shell_panels_never_cover_status_bar() {
        use DockOrientation::{SideBySide, Stacked};
        use DockSide::{Left, Right};
        for (side, orient) in [
            (Right, Stacked),
            (Left, Stacked),
            (Right, SideBySide),
            (Left, SideBySide),
        ] {
            let regions = draw_shell_full(1280.0, 800.0, true, side, orient).expect("regions");
            assert!(
                regions.status_overlaps().is_empty(),
                "{side:?}/{orient:?}: painéis sobre a status bar: {:?}",
                regions.status_overlaps()
            );
            assert!(
                regions.dock_sections_disjoint(),
                "{side:?}/{orient:?}: outliner/inspector sobrepostos"
            );
        }
        for (w, h) in [(1920.0, 1080.0), (1280.0, 800.0), (1024.0, 700.0)] {
            let regions = draw_shell(w, h).expect("shell registra regiões do frame corrente");
            eprintln!("{w}x{h} panes: {:?}", regions.panes());
            assert!(regions.status_bar.is_some(), "{w}x{h}: status bar ausente");
            assert!(
                regions.status_overlaps().is_empty(),
                "{w}x{h}: painéis sobre a status bar: {:?}",
                regions.status_overlaps()
            );
            assert!(
                regions.dock_sections_disjoint(),
                "{w}x{h}: outliner/inspector sobrepostos"
            );
            assert!(
                regions.shelf_within_viewport(),
                "{w}x{h}: shelf fora da viewport"
            );
            let vp = regions.viewport.expect("viewport presente");
            assert!(
                vp.width() > 200.0 && vp.height() > 100.0,
                "{w}x{h}: viewport esmagada ({vp:?})"
            );
        }
    }
}
