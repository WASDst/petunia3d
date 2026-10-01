//! Cabeçalho superior principal do Petunia3D (`Main Header`).
//! Menus do sistema (File, Edit, Render, Window, Help) e abas de Workspaces em pílulas arredondadas.

use egui::{Color32, Ui};
use petunia_core::{AppState, DocsTopic, Workspace};

use crate::UiAction;
use crate::adapters::toolbar::{PetuniaToolbarCluster, PetuniaToolbarId};
use crate::adapters::top_bar::{PetuniaTopBar, PetuniaTopBarSlot, PetuniaTopBarSpec};
use crate::foundation::typography::TextRole;
use crate::icon_registry::PetuniaIcon;
use crate::tokens;
use crate::widgets::{
    self, PetuniaMenuButton, PetuniaMenuCheckboxItem, PetuniaMenuItem, PetuniaMenuRadioItem,
    petunia_menu_separator,
};

/// Identificador da barra (raiz dos ids internos do adapter).
const TOP_BAR_ID: &str = "main-header";

/// Faixas da Top Bar (§25). O adapter mede e decide; aqui existe só a semântica
/// de produto: quais zonas existem, quais ações podem cair e o que a seta
/// esconde.
const SLOT_MENUS: PetuniaToolbarId = "header.menus";
const SLOT_WORKSPACES: PetuniaToolbarId = "header.workspaces";
const SLOT_ASSETS: PetuniaToolbarId = "header.assets";
const SLOT_SETTINGS: PetuniaToolbarId = "header.settings";
const SLOT_OVERFLOW: PetuniaToolbarId = "header.overflow";

/// Declaração da Top Bar: menus (navegação, nunca caem), pílulas de workspace
/// (centro geométrico) e ações globais (caem para a seta por prioridade).
///
/// `settings` tem rank maior que `assets`: com a barra estreita é o painel de
/// Assets que vai para a seta primeiro — configurações continuam a um clique.
static TOP_BAR: PetuniaTopBarSpec = PetuniaTopBarSpec {
    left: &[PetuniaToolbarCluster::pinned(SLOT_MENUS)],
    center: &[PetuniaToolbarCluster::pinned(SLOT_WORKSPACES)],
    right: &[
        PetuniaToolbarCluster::overflowable(SLOT_ASSETS, 20),
        PetuniaToolbarCluster::overflowable(SLOT_SETTINGS, 30),
    ],
    overflow: Some(SLOT_OVERFLOW),
};

/// Renderiza o cabeçalho superior completo da aplicação.
///
/// Três zonas medidas (§25): menus à esquerda, pílulas de workspace no **centro
/// geométrico** e ações globais à direita. Sem brand: o header é navegação, não
/// vitrine. Quem mede, decide o que cai e distribui é [`PetuniaTopBar`]; nenhuma
/// coluna, espaçador ou largura é calculada aqui.
pub fn draw(ui: &mut Ui, state: &mut AppState, action: &mut UiAction) {
    let header_resp = egui::Panel::top("main_header")
        .default_size(tokens::TOP_HEADER_HEIGHT)
        .size_range(tokens::TOP_HEADER_HEIGHT..=tokens::TOP_HEADER_MAX_HEIGHT)
        .resizable(true)
        .frame(
            egui::Frame::new()
                .fill(tokens::bg_header(state))
                .stroke(tokens::stroke_border_dyn(state))
                .inner_margin(egui::Margin::symmetric(8, 2)),
        )
        .show(ui, |ui| {
            ui.add_enabled_ui(!state.is_interacting(), |ui| {
                let bar = PetuniaTopBar::new(TOP_BAR_ID, &TOP_BAR);
                bar.show(ui, &mut |ui, slot| draw_slot(ui, state, action, slot));
            });
        });
    crate::regions::record(
        ui.ctx(),
        crate::regions::RegionSlot::Header,
        header_resp.response.rect,
    );
}

/// Desenho de uma faixa declarada em [`TOP_BAR`].
fn draw_slot(
    ui: &mut Ui,
    state: &mut AppState,
    action: &mut UiAction,
    slot: PetuniaTopBarSlot<'_>,
) {
    match slot.id {
        SLOT_MENUS => draw_menus(ui, state, action),
        SLOT_WORKSPACES => draw_workspace_pills(ui, state),
        SLOT_ASSETS => draw_asset_toggle(ui, state),
        SLOT_SETTINGS => draw_settings_toggle(ui, state),
        SLOT_OVERFLOW => draw_overflow_menu(ui, state, slot.hidden),
        _ => {}
    }
}

/// Ação global: alterna o painel de Assets do projeto.
fn draw_asset_toggle(ui: &mut Ui, state: &mut AppState) {
    if widgets::petunia_action_button(ui, Some(PetuniaIcon::Folder), "Assets", false)
        .on_hover_text("Alternar Painel de Assets do Projeto")
        .clicked()
    {
        state.ui.show_asset_browser = !state.ui.show_asset_browser;
        state.mark_dirty();
    }
}

/// Ação global: preferências (tema, ícones, idioma, teclas).
fn draw_settings_toggle(ui: &mut Ui, state: &mut AppState) {
    if widgets::petunia_action_button(ui, Some(PetuniaIcon::Settings), "Config", false)
        .on_hover_text("Preferências e Configurações (Tema, Ícones, Idioma, Teclas)")
        .clicked()
    {
        state.ui.show_settings = !state.ui.show_settings;
        state.mark_dirty();
    }
}

/// Seta de acesso: ações que não couberam na barra continuam operáveis.
fn draw_overflow_menu(ui: &mut Ui, state: &mut AppState, hidden: &[PetuniaToolbarId]) {
    PetuniaMenuButton::chevron_only()
        .tooltip(&state.t("ui.more_actions"))
        .show(ui, |ui| {
            ui.set_min_width(180.0);
            if hidden.contains(&SLOT_ASSETS)
                && PetuniaMenuCheckboxItem::new("Assets", state.ui.show_asset_browser)
                    .show(ui)
                    .clicked()
            {
                state.ui.show_asset_browser = !state.ui.show_asset_browser;
                state.mark_dirty();
            }
            if hidden.contains(&SLOT_SETTINGS)
                && PetuniaMenuItem::new(&state.t("menu.preferences"))
                    .show(ui)
                    .clicked()
            {
                state.ui.show_settings = !state.ui.show_settings;
                state.mark_dirty();
                ui.close();
            }
        });
}

fn draw_menus(ui: &mut Ui, state: &mut AppState, action: &mut UiAction) {
    let visuals = ui.visuals_mut();
    visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
    visuals.widgets.hovered.weak_bg_fill = tokens::BG_SURFACE_HOVER;
    visuals.widgets.active.weak_bg_fill = tokens::ACCENT_BLUE;

    ui.style_mut().text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::proportional(TextRole::Pill.size()),
    );

    // 1. FILE MENU
    ui.menu_button(state.t("menu.file"), |ui| {
        let new_sc = state.ui.keybinds.format_shortcut("global.new_project");
        if PetuniaMenuItem::new(&state.t("file.new"))
            .shortcut(new_sc.as_deref().or(Some("Ctrl+N")))
            .show(ui)
            .clicked()
        {
            crate::new_project(state);
            ui.close();
        }

        let open_sc = state.ui.keybinds.format_shortcut("global.open_project");
        if PetuniaMenuItem::new(&state.t("file.open_project"))
            .shortcut(open_sc.as_deref().or(Some("Ctrl+O")))
            .show(ui)
            .clicked()
        {
            crate::open_project_dialog(state);
            ui.close();
        }

        let recent_projects = state.project.recent_projects.projects().to_vec();
        ui.menu_button(state.t("menu.recent_projects"), |ui| {
            if recent_projects.is_empty() {
                PetuniaMenuItem::new("No Recent Projects")
                    .enabled(false)
                    .show(ui);
            } else {
                let mut path_to_open = None;
                for entry in &recent_projects {
                    let name = if !entry.name.is_empty() {
                        &entry.name
                    } else {
                        "Untitled"
                    };
                    if PetuniaMenuItem::new(name).show(ui).clicked() {
                        path_to_open = Some(entry.path.clone());
                        ui.close();
                    }
                }
                if let Some(path) = path_to_open
                    && let Err(e) = state.open_project(&path)
                {
                    state.set_status(format!("Failed to open project: {}", e));
                }
            }
        });

        petunia_menu_separator(ui);

        let save_sc = state.ui.keybinds.format_shortcut("global.save_project");
        if PetuniaMenuItem::new(&state.t("file.save"))
            .shortcut(save_sc.as_deref().or(Some("Ctrl+S")))
            .show(ui)
            .clicked()
        {
            crate::save_project_dialog(state, false);
            ui.close();
        }

        let save_as_sc = state.ui.keybinds.format_shortcut("global.save_project_as");
        if PetuniaMenuItem::new(&state.t("file.save_as"))
            .shortcut(save_as_sc.as_deref().or(Some("Ctrl+Shift+S")))
            .show(ui)
            .clicked()
        {
            crate::save_project_dialog(state, true);
            ui.close();
        }

        if PetuniaMenuItem::new("Save Active Model as Asset")
            .show(ui)
            .clicked()
        {
            state.save_active_as_asset();
            ui.close();
        }

        petunia_menu_separator(ui);

        if PetuniaMenuItem::new("Import OBJ (.obj)...")
            .show(ui)
            .clicked()
        {
            crate::import_obj_dialog(state);
            ui.close();
        }

        if PetuniaMenuItem::new("Import glTF / GLB (.gltf, .glb)...")
            .show(ui)
            .clicked()
        {
            crate::import_gltf_dialog(state);
            ui.close();
        }

        if PetuniaMenuItem::new("Export OBJ (.obj)...")
            .show(ui)
            .clicked()
        {
            crate::export_active_or_all(state, false);
            ui.close();
        }

        if PetuniaMenuItem::new("Export GLB (.glb)...")
            .show(ui)
            .clicked()
        {
            crate::export_active_or_all(state, true);
            ui.close();
        }

        petunia_menu_separator(ui);

        let quit_sc = state.ui.keybinds.format_shortcut("global.quit");
        if PetuniaMenuItem::new(&state.t("file.quit"))
            .shortcut(quit_sc.as_deref().or(Some("Ctrl+Q")))
            .show(ui)
            .clicked()
        {
            action.quit = true;
            ui.close();
        }
    });

    // 2. EDIT MENU
    ui.menu_button(state.t("menu.edit"), |ui| {
        let undo_sc = state.ui.keybinds.format_shortcut("global.undo");
        if PetuniaMenuItem::new(&state.t("edit.undo"))
            .shortcut(undo_sc.as_deref().or(Some("Ctrl+Z")))
            .enabled(state.project.undo.can_undo())
            .show(ui)
            .clicked()
        {
            state.undo();
            ui.close();
        }

        let redo_sc = state.ui.keybinds.format_shortcut("global.redo");
        if PetuniaMenuItem::new(&state.t("edit.redo"))
            .shortcut(redo_sc.as_deref().or(Some("Ctrl+Y")))
            .enabled(state.project.undo.can_redo())
            .show(ui)
            .clicked()
        {
            state.redo();
            ui.close();
        }

        petunia_menu_separator(ui);

        let cp_sc = state.ui.keybinds.format_shortcut("global.command_palette");
        if PetuniaMenuItem::new(&state.t("menu.command_palette"))
            .shortcut(cp_sc.as_deref().or(Some("Ctrl+P")))
            .show(ui)
            .clicked()
        {
            state.ui.show_command_palette = !state.ui.show_command_palette;
            if state.ui.show_command_palette {
                state.ui.command_palette_query.clear();
                state.ui.command_palette_selected_index = 0;
            }
            state.mark_dirty();
            ui.close();
        }

        let pref_sc = state.ui.keybinds.format_shortcut("global.settings");
        if PetuniaMenuItem::new(&state.t("menu.preferences"))
            .shortcut(pref_sc.as_deref().or(Some("Ctrl+,")))
            .show(ui)
            .clicked()
        {
            state.ui.show_settings = !state.ui.show_settings;
            state.mark_dirty();
            ui.close();
        }
    });

    // 3. WINDOW MENU
    ui.menu_button(state.t("menu.window"), |ui| {
        if PetuniaMenuCheckboxItem::new("Performance HUD", state.ui.show_perf)
            .show(ui)
            .clicked()
        {
            state.ui.show_perf = !state.ui.show_perf;
            state.mark_dirty();
        }

        if PetuniaMenuCheckboxItem::new("Asset Browser", state.ui.show_asset_browser)
            .show(ui)
            .clicked()
        {
            state.ui.show_asset_browser = !state.ui.show_asset_browser;
            state.mark_dirty();
        }

        if PetuniaMenuCheckboxItem::new("Asset Library Drawer", state.ui.show_asset_library)
            .show(ui)
            .clicked()
        {
            state.ui.show_asset_library = !state.ui.show_asset_library;
            state.mark_dirty();
        }

        if PetuniaMenuCheckboxItem::new("Reference Set Manager", state.ui.show_reference_manager)
            .show(ui)
            .clicked()
        {
            state.ui.show_reference_manager = !state.ui.show_reference_manager;
            state.mark_dirty();
        }

        petunia_menu_separator(ui);

        // Language Submenu
        ui.menu_button(state.t("ui.language"), |ui| {
            for lang in petunia_config::I18n::available() {
                let is_active = state.ui.i18n.lang == lang;
                if PetuniaMenuRadioItem::new(&lang, is_active)
                    .show(ui)
                    .clicked()
                {
                    state.ui.i18n.set_lang(&lang);
                    state.mark_dirty();
                    ui.close();
                }
            }
        });

        // Theme Submenu — lista derivada do registro de temas (oficiais V1 +
        // packs externos do usuário). Nada de IDs hardcoded: um pack novo ou um
        // tema renomeado aparece aqui automaticamente (P3D-085).
        ui.menu_button("Theme", |ui| {
            let registry = petunia_config::theme::ThemeRegistry::global();
            for manifest in registry.available() {
                let theme_id = manifest.id.as_str();
                let is_active = state.ui.active_theme_id == theme_id;
                if PetuniaMenuRadioItem::new(&manifest.name, is_active)
                    .show(ui)
                    .clicked()
                {
                    state.ui.active_theme_id = theme_id.to_string();
                    state.mark_dirty();
                    ui.close();
                }
            }
        });
    });

    // 4. HELP MENU
    ui.menu_button(state.t("menu.help"), |ui| {
        let topics = [
            ("Quick Start Guide", DocsTopic::GettingStarted),
            ("Extrude & Modeling", DocsTopic::Extrude),
            ("Keyboard Shortcuts", DocsTopic::Keymaps),
            ("Themes & Styling", DocsTopic::Themes),
            ("Navigation Controls", DocsTopic::Navigation),
        ];

        for (label, topic) in topics {
            if PetuniaMenuItem::new(label).show(ui).clicked() {
                ui.ctx()
                    .open_url(egui::OpenUrl::new_tab(topic.canonical_url()));
                ui.close();
            }
        }

        petunia_menu_separator(ui);

        if PetuniaMenuItem::new("About Petunia3D...")
            .show(ui)
            .clicked()
        {
            state.ui.show_help = true;
            state.mark_dirty();
            ui.close();
        }
    });
}

fn draw_workspace_pills(ui: &mut Ui, state: &mut AppState) {
    // Derivado do enum de workspaces compilados: a lista de pílulas nunca
    // diverge do contrato (V1 = MODEL / PAINT / UV).
    let canonical_workspaces: Vec<(Workspace, String)> = crate::workspaces::visible_workspaces()
        .iter()
        .copied()
        .map(|ws| (ws, state.t(ws.key())))
        .collect();

    for (ws, label) in canonical_workspaces {
        let is_active = state.workspace == ws;
        let (bg, fg) = if is_active {
            (tokens::ACCENT_BLUE, tokens::TEXT_ACTIVE)
        } else {
            (Color32::TRANSPARENT, tokens::TEXT_SECONDARY)
        };

        let button = egui::Button::new(
            egui::RichText::new(&label)
                .size(TextRole::Pill.size())
                .color(fg),
        )
        .fill(bg)
        .corner_radius(tokens::RADIUS_PILL);

        if ui.add(button).clicked() {
            // Transição com memória de layout por workspace (Wave 3).
            state.switch_workspace(ws);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main_header_renders_without_panic() {
        let ctx = egui::Context::default();
        let mut state = AppState::new("en");
        let mut action = UiAction::none();

        ctx.run_ui(egui::RawInput::default(), |ui| {
            draw(ui, &mut state, &mut action);
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn test_main_header_renders_at_different_heights() {
        let mut state = AppState::new("en");
        let mut action = UiAction::none();

        // Testa renderização com tela expandida e múltiplas dimensões
        for width in [800.0, 1280.0, 1920.0] {
            let ctx = egui::Context::default();
            let raw_input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 800.0),
                )),
                ..Default::default()
            };

            ctx.run_ui(raw_input, |ui| {
                draw(ui, &mut state, &mut action);
            })
            .textures_delta
            .clear();
        }
    }
}
