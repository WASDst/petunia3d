//! i18n via TOML — melhor encaixe para Rust:
//! - `serde + toml` é idiomático, tipado e com comentários (#),
//! - menos verboso que JSON, sem as armadilhas do YAML (spec gigante),
//! - um arquivo por idioma em `locales/*.toml`, fácil de traduzir.
//!
//! Formato: tabelas viram prefixo com ponto. Ex.:
//! ```toml
//! [ui]
//! title = "Simple3D"
//! [tools]
//! select = "Selecionar"
//! ```
//! vira chaves `ui.title`, `tools.select` via `t()`.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{LazyLock, RwLock};

static AVAILABLE_LOCALES: LazyLock<RwLock<Vec<String>>> =
    LazyLock::new(|| RwLock::new(discover_available_locales()));

pub struct I18n {
    pub lang: String,
    map: HashMap<String, String>,
    fallback: HashMap<String, String>,
}

impl I18n {
    /// Lista de idiomas já descoberta em cache. O filesystem não é tocado no
    /// hot path de menus egui; o watcher chama [`Self::refresh_available`].
    pub fn available() -> Vec<String> {
        match AVAILABLE_LOCALES.read() {
            Ok(locales) => locales.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    /// Reescaneia os arquivos de locale. Deve ser chamado somente em eventos de
    /// mudança do watcher ou por ações explícitas de atualização.
    pub fn refresh_available() {
        let locales = discover_available_locales();
        match AVAILABLE_LOCALES.write() {
            Ok(mut cached) => *cached = locales,
            Err(poisoned) => *poisoned.into_inner() = locales,
        }
    }

    pub fn load(lang: &str) -> Self {
        let fallback = load_file("en");
        let map = if lang == "en" {
            fallback.clone()
        } else {
            let m = load_file(lang);
            if m.is_empty() { fallback.clone() } else { m }
        };
        Self {
            lang: lang.to_string(),
            map,
            fallback,
        }
    }

    pub fn set_lang(&mut self, lang: &str) {
        *self = Self::load(lang);
    }

    /// Traduz `chave`. Cai para inglês e depois para a própria chave.
    /// Retorna `String` (não `&str`) de propósito: evita borrow persistente
    /// de `app` dentro dos closures do egui.
    pub fn t(&self, key: &str) -> String {
        self.map
            .get(key)
            .or_else(|| self.fallback.get(key))
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    /// Traduz um [`TextId`] tipado (Wave 7).
    pub fn t_id(&self, id: TextId) -> String {
        self.t(id.key())
    }

    /// Constrói de um texto TOML (testes e pseudo-locale, sem disco).
    pub fn parse(text: &str) -> HashMap<String, String> {
        let v: toml::Value = toml::from_str(text).unwrap_or(toml::Value::Table(Default::default()));
        let mut map = HashMap::new();
        flatten(&v, String::new(), &mut map);
        map
    }

    /// Pseudo-locale de teste (Wave 7 — §12.4): expande ~40% e acentua para
    /// expor clipping, larguras fixas e overflow de shelf/dropdowns.
    /// Uso exclusivo em testes; nunca embarca.
    pub fn pseudo_from(base: &Self) -> Self {
        let map = base
            .map
            .iter()
            .map(|(k, v)| (k.clone(), pseudo_transform(v)))
            .collect();
        let fallback = base
            .fallback
            .iter()
            .map(|(k, v)| (k.clone(), pseudo_transform(v)))
            .collect();
        Self {
            lang: "pseudo".to_string(),
            map,
            fallback,
        }
    }

    /// Chaves do mapa principal (auditoria de paridade).
    pub fn keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.map.keys().cloned().collect();
        keys.sort();
        keys
    }
}

/// Expansão pseudo-locale: prefixo/sufixo visíveis + alongamento + acentos.
fn pseudo_transform(text: &str) -> String {
    const ACCENTS: &[char] = &['é', 'ñ', 'ü', 'ß', 'ç', 'ø'];
    let mut out = String::with_capacity(text.len() * 2 + 4);
    out.push('⟦');
    for (i, ch) in text.chars().enumerate() {
        out.push(ch);
        if ch.is_ascii_alphabetic() && i % 2 == 0 {
            out.push(ch);
        }
        if ch == ' ' {
            out.push(ACCENTS[i % ACCENTS.len()]);
        }
    }
    out.push('⟧');
    out
}

/// Identificador de tradução tipado (Wave 7 — §12.2).
///
/// Evita chaves stringly-typed espalhadas: o catálogo vive em [`text_id`] e o
/// teste de paridade garante resolução em todos os locales embutidos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextId(pub &'static str);

impl TextId {
    pub const fn new(key: &'static str) -> Self {
        Self(key)
    }

    pub fn key(self) -> &'static str {
        self.0
    }
}

/// Catálogo de `TextId` das superfícies migradas (Waves 2–7).
///
/// Chaves pré-existentes em formato string continuam válidas (compatível);
/// superfícies novas usam estas constantes.
pub mod text_id {
    use super::TextId;

    pub const UI_OUTLINER: TextId = TextId::new("ui.outliner");
    pub const UI_PROPERTIES: TextId = TextId::new("ui.properties");
    pub const UI_COLLAPSE: TextId = TextId::new("ui.collapse");
    pub const UI_EXPAND: TextId = TextId::new("ui.expand");
    pub const UI_DOCK_SPLIT_HINT: TextId = TextId::new("ui.dock_split_hint");
    pub const UI_MORE: TextId = TextId::new("ui.more");
    pub const UI_TOOLS_MENU: TextId = TextId::new("ui.tools_menu");
    pub const UI_DUPLICATE: TextId = TextId::new("ui.duplicate");
    pub const UI_REFS: TextId = TextId::new("ui.refs");
    pub const UI_ASSETS: TextId = TextId::new("ui.assets");
    pub const UI_CLOSE: TextId = TextId::new("ui.close");
    pub const UI_FLOATING_INSPECTOR: TextId = TextId::new("ui.floating_inspector");
    pub const UI_REDOCK: TextId = TextId::new("ui.redock");
    pub const UI_AT_3D_CURSOR: TextId = TextId::new("ui.at_3d_cursor");

    // Chrome do shell Slint: títulos de painel, botões e dicas de status.
    pub const UI_PARTS: TextId = TextId::new("ui.parts");
    pub const UI_COLLAPSE_INSPECTOR: TextId = TextId::new("ui.collapse_inspector");
    pub const UI_EXPAND_INSPECTOR: TextId = TextId::new("ui.expand_inspector");
    pub const UI_TAB_PARTS: TextId = TextId::new("ui.tab_parts");
    pub const UI_TAB_TRANSFORM: TextId = TextId::new("ui.tab_transform");
    pub const UI_TAB_MATERIAL: TextId = TextId::new("ui.tab_material");
    pub const UI_TAB_OBJECT: TextId = TextId::new("ui.tab_object");
    pub const UI_TAB_MODIFIERS: TextId = TextId::new("ui.tab_modifiers");
    pub const UI_OBJECT_NAME: TextId = TextId::new("ui.object_name");
    pub const UI_OBJECT_VISIBILITY: TextId = TextId::new("ui.object_visibility");
    pub const UI_OBJECT_LOCK: TextId = TextId::new("ui.object_lock");
    pub const UI_OBJECT_NO_SELECTION: TextId = TextId::new("ui.object_no_selection");
    pub const UI_RESIZE_PANEL_WIDTH: TextId = TextId::new("ui.resize_panel_width");
    pub const UI_SECTION_DOCK: TextId = TextId::new("ui.section_dock");
    pub const UI_SECTION_DRAG: TextId = TextId::new("ui.section_drag");
    pub const UI_SECTION_PIN_OPEN: TextId = TextId::new("ui.section_pin_open");
    pub const UI_SECTION_PIN_ASSET: TextId = TextId::new("ui.section_pin_asset");
    pub const UI_SECTION_UNPIN_ASSET: TextId = TextId::new("ui.section_unpin_asset");
    pub const UI_TOOL_CARD: TextId = TextId::new("ui.tool_card");
    pub const UI_TOOL_OPTIONS: TextId = TextId::new("ui.tool_options");
    pub const UI_TOOL_OPTIONS_EXPAND: TextId = TextId::new("ui.tool_options_expand");
    pub const UI_TOOL_OPTIONS_COLLAPSE: TextId = TextId::new("ui.tool_options_collapse");
    pub const UI_NO_TOOL_PARAMETERS: TextId = TextId::new("ui.no_tool_parameters");
    pub const UI_LAST_OPERATION: TextId = TextId::new("ui.last_operation");
    pub const UI_STATS_VERTS: TextId = TextId::new("ui.stats_verts");
    pub const UI_STATS_FACES: TextId = TextId::new("ui.stats_faces");
    pub const UI_STATS_TRIS: TextId = TextId::new("ui.stats_tris");
    pub const UI_STATS_SELECTION: TextId = TextId::new("ui.stats_selection");
    pub const UI_QUICK_ACTIONS: TextId = TextId::new("ui.quick_actions");
    pub const UI_QUICK_ACTION_CUSTOMIZE: TextId = TextId::new("ui.quick_action_customize");
    pub const UI_QUICK_ACTION_ADD: TextId = TextId::new("ui.quick_action_add");
    pub const UI_QUICK_ACTION_REMOVE: TextId = TextId::new("ui.quick_action_remove");
    pub const UI_QUICK_ACTION_RESET: TextId = TextId::new("ui.quick_action_reset");
    pub const UI_QUICK_ACTION_DONE: TextId = TextId::new("ui.quick_action_done");
    pub const UI_ACTION_SUBDIVIDE: TextId = TextId::new("ui.action_subdivide");
    pub const UI_ACTION_FUSE: TextId = TextId::new("ui.action_fuse");
    pub const UI_ACTION_CUT: TextId = TextId::new("ui.action_cut");
    pub const UI_ACTION_INTERSECT: TextId = TextId::new("ui.action_intersect");
    pub const UI_ACTION_JOIN: TextId = TextId::new("ui.action_join");
    pub const UI_ACTION_MERGE: TextId = TextId::new("ui.action_merge");
    pub const UI_ACTION_SLICE: TextId = TextId::new("ui.action_slice");
    pub const UI_ACTION_LOOP_CUT: TextId = TextId::new("ui.action_loop_cut");
    pub const UI_MATERIAL_EDITOR: TextId = TextId::new("ui.material_editor");
    pub const UI_MATERIAL_NAME: TextId = TextId::new("ui.material_name");
    pub const UI_MATERIAL_BASE_COLOR: TextId = TextId::new("ui.material_base_color");
    pub const UI_MATERIAL_ASSIGN: TextId = TextId::new("ui.material_assign");
    pub const UI_MATERIAL_NEW: TextId = TextId::new("ui.material_new");
    pub const UI_MATERIAL_DUPLICATE: TextId = TextId::new("ui.material_duplicate");
    pub const UI_MATERIAL_REMOVE: TextId = TextId::new("ui.material_remove");
    pub const UI_MATERIAL_NO_MATERIAL: TextId = TextId::new("ui.material_no_material");
    pub const UI_MATERIAL_NO_SELECTION: TextId = TextId::new("ui.material_no_selection");
    pub const UI_MATERIAL_PROFILE: TextId = TextId::new("ui.material_profile");
    pub const UI_MATERIAL_ROUGHNESS: TextId = TextId::new("ui.material_roughness");
    pub const UI_MATERIAL_METALLIC: TextId = TextId::new("ui.material_metallic");
    pub const UI_MATERIAL_NORMAL_SCALE: TextId = TextId::new("ui.material_normal_scale");
    pub const UI_MATERIAL_EMISSION: TextId = TextId::new("ui.material_emission");
    pub const UI_MATERIAL_EMISSION_STRENGTH: TextId = TextId::new("ui.material_emission_strength");
    pub const UI_MATERIAL_ALPHA_MODE: TextId = TextId::new("ui.material_alpha_mode");
    pub const UI_MATERIAL_ALPHA_CUTOFF: TextId = TextId::new("ui.material_alpha_cutoff");
    pub const UI_MATERIAL_TEXTURE_ALBEDO: TextId = TextId::new("ui.material_texture_albedo");
    pub const UI_MATERIAL_NO_TEXTURE: TextId = TextId::new("ui.material_no_texture");
    pub const UI_MATERIAL_CREATE_TEXTURE: TextId = TextId::new("ui.material_create_texture");
    pub const UI_MATERIAL_CLEAR_TEXTURE: TextId = TextId::new("ui.material_clear_texture");
    pub const UI_MATERIAL_PROFILE_PBR: TextId = TextId::new("ui.material_profile_pbr");
    pub const UI_MATERIAL_PROFILE_UNLIT: TextId = TextId::new("ui.material_profile_unlit");
    pub const UI_MATERIAL_PROFILE_TOON: TextId = TextId::new("ui.material_profile_toon");
    pub const UI_MATERIAL_PROFILE_GLASS: TextId = TextId::new("ui.material_profile_glass");
    pub const UI_MATERIAL_PROFILE_EMISSIVE: TextId = TextId::new("ui.material_profile_emissive");
    pub const UI_MATERIAL_ALPHA_OPAQUE: TextId = TextId::new("ui.material_alpha_opaque");
    pub const UI_MATERIAL_ALPHA_MASK: TextId = TextId::new("ui.material_alpha_mask");
    pub const UI_MATERIAL_ALPHA_BLEND: TextId = TextId::new("ui.material_alpha_blend");
    pub const UI_MATERIAL_ADVANCED: TextId = TextId::new("ui.material_advanced");
    pub const UI_MODIFIERS_EMPTY: TextId = TextId::new("ui.modifiers_empty");
    pub const UI_MODIFIER_MIRROR: TextId = TextId::new("ui.modifier_mirror");
    pub const UI_MODIFIER_SYMMETRY: TextId = TextId::new("ui.modifier_symmetry");
    pub const UI_MODIFIER_APPLY: TextId = TextId::new("ui.modifier_apply");
    pub const UI_MODIFIER_DIRECTION: TextId = TextId::new("ui.modifier_direction");
    pub const UI_MODIFIER_POSITIVE_TO_NEGATIVE: TextId =
        TextId::new("ui.modifier_positive_to_negative");
    pub const UI_MODIFIER_NEGATIVE_TO_POSITIVE: TextId =
        TextId::new("ui.modifier_negative_to_positive");
    pub const UI_MODIFIER_ADD_MIRROR: TextId = TextId::new("ui.modifier_add_mirror");
    pub const UI_MODIFIER_ADD_SYMMETRY: TextId = TextId::new("ui.modifier_add_symmetry");
    pub const UI_MODIFIER_AXIS: TextId = TextId::new("ui.modifier_axis");
    pub const UI_MODIFIER_REMOVE: TextId = TextId::new("ui.modifier_remove");
    pub const UI_MODIFIER_MOVE_UP: TextId = TextId::new("ui.modifier_move_up");
    pub const UI_MODIFIER_MOVE_DOWN: TextId = TextId::new("ui.modifier_move_down");
    pub const UI_PARTS_COLLECTION: TextId = TextId::new("ui.parts_collection");
    pub const UI_PARTS_ANNOTATIONS: TextId = TextId::new("ui.parts_annotations");
    pub const UI_PARTS_MEASUREMENTS: TextId = TextId::new("ui.parts_measurements");
    pub const UI_PARTS_ISOLATE: TextId = TextId::new("ui.parts_isolate");
    pub const UI_PARTS_EXIT_ISOLATE: TextId = TextId::new("ui.parts_exit_isolate");
    pub const UI_PARTS_NEW_COLLECTION: TextId = TextId::new("ui.parts_new_collection");
    pub const UI_PROJECT_ASSET_LIBRARY: TextId = TextId::new("ui.project_asset_library");
    pub const UI_SAVE_ACTIVE_AS_ASSET: TextId = TextId::new("ui.save_active_as_asset");
    pub const UI_STATUS_HINT: TextId = TextId::new("ui.status_hint");
    pub const UI_UNWRAP_MESH: TextId = TextId::new("ui.unwrap_mesh");
    pub const UI_PACK_ISLANDS: TextId = TextId::new("ui.pack_islands");
    pub const UI_ACTIVE_BRUSH_COLOR: TextId = TextId::new("ui.active_brush_color");
    pub const UI_ALBEDO_BASE_COLOR: TextId = TextId::new("ui.albedo_base_color");
    pub const UI_THEME: TextId = TextId::new("ui.theme");
    pub const UI_PLACE_IN_SCENE: TextId = TextId::new("ui.place_in_scene");
    pub const UI_VERTICAL_TOOL_DRAG: TextId = TextId::new("ui.vertical_tool_drag");
    pub const UI_INVERT_VERTICAL_DRAG: TextId = TextId::new("ui.invert_vertical_drag");
    pub const UI_VERTICAL_DRAG_INVERTED: TextId = TextId::new("ui.vertical_drag_inverted");
    pub const UI_VERTICAL_DRAG_NORMAL: TextId = TextId::new("ui.vertical_drag_normal");
    pub const UI_SEARCH_ASSETS: TextId = TextId::new("ui.search_assets");
    pub const UI_SEARCH_PARTS: TextId = TextId::new("ui.search_parts");
    pub const UI_INSPECTOR: TextId = TextId::new("ui.inspector");
    pub const UI_NUMERIC_FIELD_HINT: TextId = TextId::new("ui.numeric_field_hint");
    pub const TOOLS_SELECT: TextId = TextId::new("tools.select");
    pub const TOOLS_ROTATE: TextId = TextId::new("tools.rotate");
    pub const TOOLS_SCALE: TextId = TextId::new("tools.scale");
    pub const TOOLS_TRANSFORM: TextId = TextId::new("tools.transform");
    pub const TOOLS_SELECT_LASSO: TextId = TextId::new("tools.select_lasso");
    pub const TRANSFORM_POSITION: TextId = TextId::new("transform.position");
    pub const TOOLS_LOOP_CUT: TextId = TextId::new("tools.loop_cut");
    pub const TOOLS_SLICE: TextId = TextId::new("tools.slice");
    pub const TOOLS_PUSH_PULL: TextId = TextId::new("tools.push_pull");
    pub const TOOLS_DRAW_PROFILE: TextId = TextId::new("tools.draw_profile");
    pub const TOOLS_PIVOT: TextId = TextId::new("tools.pivot");
    pub const UI_PIVOT_HINT: TextId = TextId::new("ui.pivot_hint");
    pub const UI_LOOP_CUT_HINT: TextId = TextId::new("ui.loop_cut_hint");
    pub const UI_SLICE_HINT: TextId = TextId::new("ui.slice_hint");
    pub const UI_PUSH_PULL_HINT: TextId = TextId::new("ui.push_pull_hint");
    pub const UI_PROFILE_HINT: TextId = TextId::new("ui.profile_hint");
    pub const UI_PROFILE_DEPTH: TextId = TextId::new("ui.profile_depth");
    pub const UI_PROFILE_POINTS: TextId = TextId::new("ui.profile_points");
    pub const UI_PROFILE_CLOSE: TextId = TextId::new("ui.profile_close");
    pub const UI_PROFILE_GENERATE: TextId = TextId::new("ui.profile_generate");
    pub const UI_PROFILE_REVOLVE: TextId = TextId::new("ui.profile_revolve");
    pub const UI_PROFILE_CUTS: TextId = TextId::new("ui.profile_cuts");
    pub const UI_PROFILE_PRESETS: TextId = TextId::new("ui.profile_presets");
    pub const UI_PROFILE_ADD_RECT: TextId = TextId::new("ui.profile_add_rect");
    pub const UI_PROFILE_ADD_CIRCLE: TextId = TextId::new("ui.profile_add_circle");
    pub const UI_PROFILE_CANVAS_HINT: TextId = TextId::new("ui.profile_canvas_hint");
    pub const UI_PROFILE_WALL_THICKNESS: TextId = TextId::new("ui.profile_wall_thickness");
    pub const UI_PROFILE_SMOOTH_CURVES: TextId = TextId::new("ui.profile_smooth_curves");
    pub const UI_PROFILE_SHARP_CORNERS: TextId = TextId::new("ui.profile_sharp_corners");
    pub const UI_PROFILE_SMOOTHNESS: TextId = TextId::new("ui.profile_smoothness");
    pub const UI_PROFILE_SWEEP: TextId = TextId::new("ui.profile_sweep");
    pub const UI_DECAL_TRANSFORM: TextId = TextId::new("ui.decal_transform");
    pub const UI_DECAL_POSITION: TextId = TextId::new("ui.decal_position");
    pub const UI_DECAL_SCALE: TextId = TextId::new("ui.decal_scale");
    pub const UI_DECAL_ROTATION: TextId = TextId::new("ui.decal_rotation");
    pub const UI_DECAL_BAKE: TextId = TextId::new("ui.decal_bake");
    pub const UI_DECAL_HINT: TextId = TextId::new("ui.decal_hint");
    pub const UI_PRIMITIVE_PARAMETRIC: TextId = TextId::new("ui.primitive_parametric");
    pub const UI_PRIMITIVE_FREEZE: TextId = TextId::new("ui.primitive_freeze");
    pub const UI_PRIMITIVE_FREEZE_HINT: TextId = TextId::new("ui.primitive_freeze_hint");
    pub const UI_PRIMITIVE_FROZEN_STATUS: TextId = TextId::new("ui.primitive_frozen_status");
    pub const UI_MODEL_SELECT_HINT: TextId = TextId::new("ui.model_select_hint");
    pub const UI_MODEL_POSITION_HINT: TextId = TextId::new("ui.model_position_hint");
    pub const UI_MODEL_ROTATE_HINT: TextId = TextId::new("ui.model_rotate_hint");
    pub const UI_MODEL_SCALE_HINT: TextId = TextId::new("ui.model_scale_hint");
    pub const UI_MODEL_TRANSFORM_HINT: TextId = TextId::new("ui.model_transform_hint");
    pub const UI_MODEL_LASSO_HINT: TextId = TextId::new("ui.model_lasso_hint");
    pub const UI_HIDE_PART: TextId = TextId::new("ui.hide_part");
    pub const UI_SHOW_PART: TextId = TextId::new("ui.show_part");
    pub const UI_LOCK_PART: TextId = TextId::new("ui.lock_part");
    pub const UI_UNLOCK_PART: TextId = TextId::new("ui.unlock_part");
    pub const UI_SELECTED_PARTS_ONLY: TextId = TextId::new("ui.selected_parts_only");
    pub const UI_SORT_PARTS: TextId = TextId::new("ui.sort_parts");
    pub const UI_PARTS_ROW_SIZE: TextId = TextId::new("ui.parts_row_size");
    pub const UI_SORT_ASSETS: TextId = TextId::new("ui.sort_assets");
    pub const UI_THUMBNAIL_SIZE: TextId = TextId::new("ui.thumbnail_size");
    pub const UI_SELECTION_COLOR: TextId = TextId::new("ui.selection_color");
    pub const UI_HIGHLIGHT_THICKNESS: TextId = TextId::new("ui.highlight_thickness");
    pub const UI_VIEW_WIREFRAME: TextId = TextId::new("ui.view_wireframe");
    pub const UI_VIEW_WIREFRAME_HINT: TextId = TextId::new("ui.view_wireframe_hint");
    pub const UI_VIEW_SOLID: TextId = TextId::new("ui.view_solid");
    pub const UI_VIEW_SOLID_HINT: TextId = TextId::new("ui.view_solid_hint");
    pub const UI_VIEW_MATERIAL: TextId = TextId::new("ui.view_material");
    pub const UI_VIEW_MATERIAL_HINT: TextId = TextId::new("ui.view_material_hint");
    pub const UI_VIEW_LIT: TextId = TextId::new("ui.view_lit");
    pub const UI_VIEW_LIT_HINT: TextId = TextId::new("ui.view_lit_hint");
    pub const UI_MORE_MODEL_TOOLS: TextId = TextId::new("ui.more_model_tools");
    pub const UI_XRAY_OPACITY: TextId = TextId::new("ui.xray_opacity");
    pub const UI_WIRE_OVERLAY: TextId = TextId::new("ui.wire_overlay");
    pub const UI_WIRE_OVERLAY_HINT: TextId = TextId::new("ui.wire_overlay_hint");
    pub const UI_SELECTION_COLOR_INVALID: TextId = TextId::new("ui.selection_color_invalid");
    pub const UI_SELECTION_COLOR_LOW_CONTRAST: TextId =
        TextId::new("ui.selection_color_low_contrast");
    pub const UI_PREFERENCES_SAVE_FAILED: TextId = TextId::new("ui.preferences_save_failed");
    pub const PIVOT_MEDIAN: TextId = TextId::new("pivot.median");
    pub const PIVOT_BOUNDS: TextId = TextId::new("pivot.bounds");
    pub const PIVOT_CURSOR: TextId = TextId::new("pivot.cursor");
    pub const PIVOT_INDIVIDUAL: TextId = TextId::new("pivot.individual");
    pub const PIVOT_ORIGIN_TO_GEOMETRY: TextId = TextId::new("pivot.origin_to_geometry");
    pub const PIVOT_ORIGIN_TO_BOTTOM: TextId = TextId::new("pivot.origin_to_bottom");
    pub const PIVOT_ORIGIN_TO_CURSOR: TextId = TextId::new("pivot.origin_to_cursor");
    pub const PIVOT_ORIGIN_TO_SELECTION: TextId = TextId::new("pivot.origin_to_selection");
    pub const PIVOT_GEOMETRY_TO_ORIGIN: TextId = TextId::new("pivot.geometry_to_origin");
    pub const PIVOT_EDIT_PIVOT: TextId = TextId::new("pivot.edit_pivot");
    pub const PIVOT_EDIT_PIVOT_HINT: TextId = TextId::new("pivot.edit_pivot_hint");
    pub const PREFERENCES_COLORBLIND_AXES: TextId = TextId::new("preferences.colorblind_axes");
    pub const PREFERENCES_REDUCED_MOTION: TextId = TextId::new("preferences.reduced_motion");
    pub const PREFERENCES_DOUBLE_TAP_INTERVAL: TextId =
        TextId::new("preferences.double_tap_interval");
    pub const PREFERENCES_MULTISELECTION_MEASURE: TextId =
        TextId::new("preferences.multiselection_measure_tag");

    // Diálogo de recuperação de autosave (P3D-002).
    pub const UI_RECOVERY_TITLE: TextId = TextId::new("ui.recovery_title");
    pub const UI_RECOVERY_BODY: TextId = TextId::new("ui.recovery_body");
    pub const UI_RECOVERY_RECOVER: TextId = TextId::new("ui.recovery_recover");
    pub const UI_RECOVERY_KEEP: TextId = TextId::new("ui.recovery_keep");
    pub const UI_RECOVERY_DISCARD: TextId = TextId::new("ui.recovery_discard");
    pub const ACTIONS_APPLY: TextId = TextId::new("actions.apply");
    pub const ACTIONS_CANCEL: TextId = TextId::new("actions.cancel");
    pub const ACTIONS_DELETE: TextId = TextId::new("actions.delete");
    pub const ACTIONS_DUPLICATE: TextId = TextId::new("actions.duplicate");

    // Barra de menus do shell e itens que ela publica. As chaves de locale já
    // existiam; o que faltava era o vínculo tipado que o shell consome.
    pub const MENU_FILE: TextId = TextId::new("menu.file");
    pub const MENU_EDIT: TextId = TextId::new("menu.edit");
    pub const MENU_VIEW: TextId = TextId::new("menu.view");
    pub const MENU_WINDOW: TextId = TextId::new("menu.window");
    pub const MENU_COMMAND_PALETTE: TextId = TextId::new("menu.command_palette");
    pub const MENU_PREFERENCES: TextId = TextId::new("menu.preferences");
    pub const FILE_NEW: TextId = TextId::new("file.new");
    pub const FILE_OPEN_PROJECT: TextId = TextId::new("file.open_project");
    pub const FILE_SAVE: TextId = TextId::new("file.save");
    pub const FILE_SAVE_AS: TextId = TextId::new("file.save_as");
    pub const FILE_IMPORT_OBJ: TextId = TextId::new("file.import_obj");
    pub const EDIT_UNDO: TextId = TextId::new("edit.undo");
    pub const EDIT_REDO: TextId = TextId::new("edit.redo");
    pub const VIEW_FRAME: TextId = TextId::new("view.frame");
    pub const VIEW_FRAME_ALL: TextId = TextId::new("view.frame_all");
    pub const VIEW_TOGGLE_PROJECTION: TextId = TextId::new("camera.projection");
    pub const VIEW_RESET_CAMERA: TextId = TextId::new("camera.reset");
    pub const VIEW_TOGGLE_WIREFRAME: TextId = TextId::new("shading.wire");
    pub const VIEW_TOGGLE_SPLIT: TextId = TextId::new("view.toggle_split");

    pub const UV_TITLE: TextId = TextId::new("uv.title");
    pub const UV_SELECTED: TextId = TextId::new("uv.selected");
    pub const UV_FACES: TextId = TextId::new("uv.faces");
    pub const UV_PREVIEW_3D: TextId = TextId::new("uv.preview_3d");
    pub const UV_HINT: TextId = TextId::new("uv.hint");

    pub const ANIMATE_HUMANOID: TextId = TextId::new("animate.humanoid");
    pub const ANIMATE_AUTO_RIG: TextId = TextId::new("animate.auto_rig");
    pub const ANIMATE_PLAY: TextId = TextId::new("animate.play");
    pub const ANIMATE_PAUSE: TextId = TextId::new("animate.pause");
    pub const ANIMATE_FIRST_FRAME: TextId = TextId::new("animate.first_frame");
    pub const ANIMATE_LAST_FRAME: TextId = TextId::new("animate.last_frame");
    pub const ANIMATE_FRAME: TextId = TextId::new("animate.frame");
    pub const ANIMATE_TIP_FIRST: TextId = TextId::new("animate.tip_first");
    pub const ANIMATE_TIP_PREV: TextId = TextId::new("animate.tip_prev");
    pub const ANIMATE_TIP_PLAY: TextId = TextId::new("animate.tip_play");
    pub const ANIMATE_TIP_NEXT: TextId = TextId::new("animate.tip_next");
    pub const ANIMATE_TIP_LAST: TextId = TextId::new("animate.tip_last");

    pub const PAINT_RADIUS: TextId = TextId::new("paint.radius");
    pub const PAINT_COLOR: TextId = TextId::new("paint.color");

    pub const SETTINGS_INTERFACE: TextId = TextId::new("settings.interface");
    pub const SETTINGS_IMPORT_EXPORT: TextId = TextId::new("settings.import_export");
    pub const SETTINGS_SHOW_SHELF: TextId = TextId::new("settings.show_shelf");
    pub const SETTINGS_RESET_WORKSPACE: TextId = TextId::new("settings.reset_workspace");
    pub const SETTINGS_RESET_ALL: TextId = TextId::new("settings.reset_all_layouts");
    pub const SETTINGS_EXPORT_GLB: TextId = TextId::new("settings.export_glb");
    pub const SETTINGS_EXPORT_GLB_HINT: TextId = TextId::new("settings.export_glb_hint");

    pub const REFS_VISIBLE: TextId = TextId::new("refs.visible");
    pub const REFS_LOCK: TextId = TextId::new("refs.lock");
    pub const REFS_CLICK_TO_LOAD: TextId = TextId::new("refs.click_to_load");
    pub const REFS_NO_IMAGE: TextId = TextId::new("refs.no_image");
    pub const REFS_REPLACE: TextId = TextId::new("refs.replace");
    pub const REFS_REMOVE: TextId = TextId::new("refs.remove");
    pub const REFS_ALIGN_VIEW: TextId = TextId::new("refs.align_view");
    pub const REFS_RESET_DEFAULT: TextId = TextId::new("refs.reset_default");
    pub const REFS_FINE_TUNE: TextId = TextId::new("refs.fine_tune");
    pub const REFS_LOADED: TextId = TextId::new("refs.loaded");
    pub const REFS_OPACITY: TextId = TextId::new("refs.opacity");
    pub const REFS_SIZE: TextId = TextId::new("refs.size");
    pub const REFS_OFFSET: TextId = TextId::new("refs.offset");
    pub const REFS_ROTATION: TextId = TextId::new("refs.rotation");

    pub const PRIMS_CUBE: TextId = TextId::new("prims.cube");
    pub const PRIMS_PLANE: TextId = TextId::new("prims.plane");
    pub const PRIMS_CYLINDER: TextId = TextId::new("prims.cylinder");
    pub const PRIMS_SPHERE: TextId = TextId::new("prims.sphere");
    pub const PRIMS_CONE: TextId = TextId::new("prims.cone");
    pub const PRIMS_CAPSULE: TextId = TextId::new("prims.capsule");
    pub const PRIMS_SIZE: TextId = TextId::new("prims.size");
    pub const PRIMS_RADIUS: TextId = TextId::new("prims.radius");
    pub const PRIMS_SEGMENTS: TextId = TextId::new("prims.segments");
    pub const PRIMS_RINGS: TextId = TextId::new("prims.rings");
    pub const PRIMS_HEIGHT: TextId = TextId::new("prims.height");
    pub const PRIMS_SIDES: TextId = TextId::new("prims.sides");
    pub const PRIMS_WIDTH: TextId = TextId::new("prims.width");
    pub const PRIMS_CONFIRM: TextId = TextId::new("prims.confirm");
    pub const PRIMS_CANCEL: TextId = TextId::new("prims.cancel");
    pub const PRIMS_REOPEN: TextId = TextId::new("prims.reopen");
    pub const PRIMS_CONFIRM_HINT: TextId = TextId::new("prims.confirm_hint");
    pub const PRIMS_WEDGE: TextId = TextId::new("prims.wedge");
    pub const PRIMS_CIRCLE: TextId = TextId::new("prims.circle");
    pub const PRIMS_TORUS: TextId = TextId::new("prims.torus");
    pub const PRIMS_ICOSPHERE: TextId = TextId::new("prims.icosphere");
    pub const PRIMS_GROUP_BASIC: TextId = TextId::new("prims.group_basic");
    pub const PRIMS_GROUP_ROUND: TextId = TextId::new("prims.group_round");
    pub const PRIMS_GROUP_ORGANIC: TextId = TextId::new("prims.group_organic");
    pub const PRIMS_DEPTH: TextId = TextId::new("prims.depth");
    pub const PRIMS_TOP_RADIUS: TextId = TextId::new("prims.top_radius");
    pub const PRIMS_BOTTOM_RADIUS: TextId = TextId::new("prims.bottom_radius");
    pub const PRIMS_MAJOR_RADIUS: TextId = TextId::new("prims.major_radius");
    pub const PRIMS_MINOR_RADIUS: TextId = TextId::new("prims.minor_radius");
    pub const PRIMS_VERTICES: TextId = TextId::new("prims.vertices");
    pub const PRIMS_FILL: TextId = TextId::new("prims.fill");
    pub const PRIMS_CAP: TextId = TextId::new("prims.cap");
    pub const PRIMS_SUBDIV: TextId = TextId::new("prims.subdivision");
    pub const PRIMS_BODY_LENGTH: TextId = TextId::new("prims.body_length");
    pub const PRIMS_RESET: TextId = TextId::new("prims.reset");
    pub const PRIMS_TRIS: TextId = TextId::new("prims.tris");
    pub const PRIMS_CAP_BOTH: TextId = TextId::new("prims.cap_both");
    pub const PRIMS_CAP_TOP: TextId = TextId::new("prims.cap_top_only");
    pub const PRIMS_CAP_BOTTOM: TextId = TextId::new("prims.cap_bottom_only");
    pub const PRIMS_CAP_NONE: TextId = TextId::new("prims.cap_none");
    pub const PRIMS_FILL_NONE: TextId = TextId::new("prims.fill_none");
    pub const PRIMS_FILL_DISC: TextId = TextId::new("prims.fill_disc");
    pub const PRIMS_TIP_SIDES: TextId = TextId::new("prims.tip_sides");
    pub const PRIMS_TIP_TOP_RADIUS: TextId = TextId::new("prims.tip_top_radius");
    pub const PRIMS_TIP_SUBDIV: TextId = TextId::new("prims.tip_subdiv");
    pub const PRIMS_TIP_MAJOR_RADIUS: TextId = TextId::new("prims.tip_major_radius");
    pub const PRIMS_TIP_MINOR_RADIUS: TextId = TextId::new("prims.tip_minor_radius");
    pub const PRIMS_TIP_SEGMENTS: TextId = TextId::new("prims.tip_segments");
    pub const PRIMS_TIP_RINGS: TextId = TextId::new("prims.tip_rings");
    pub const PRIMS_TIP_VERTICES: TextId = TextId::new("prims.tip_vertices");
    pub const PRIMS_TIP_FILL: TextId = TextId::new("prims.tip_fill");
    pub const PRIMS_TIP_CAPS: TextId = TextId::new("prims.tip_caps");
    pub const PRIMS_TIP_BODY_LENGTH: TextId = TextId::new("prims.tip_body_length");

    /// Todos os ids do catálogo (cobertura de tradução).
    pub const ALL: &[TextId] = &[
        UI_OUTLINER,
        UI_PROPERTIES,
        UI_COLLAPSE,
        UI_EXPAND,
        UI_DOCK_SPLIT_HINT,
        UI_MORE,
        UI_TOOLS_MENU,
        UI_DUPLICATE,
        UI_REFS,
        UI_ASSETS,
        UI_CLOSE,
        UI_FLOATING_INSPECTOR,
        UI_REDOCK,
        UI_AT_3D_CURSOR,
        UI_PARTS,
        UI_COLLAPSE_INSPECTOR,
        UI_EXPAND_INSPECTOR,
        UI_TAB_PARTS,
        UI_TAB_TRANSFORM,
        UI_TAB_MATERIAL,
        UI_TAB_OBJECT,
        UI_TAB_MODIFIERS,
        UI_OBJECT_NAME,
        UI_OBJECT_VISIBILITY,
        UI_OBJECT_LOCK,
        UI_OBJECT_NO_SELECTION,
        UI_RESIZE_PANEL_WIDTH,
        UI_SECTION_DOCK,
        UI_SECTION_DRAG,
        UI_SECTION_PIN_OPEN,
        UI_SECTION_PIN_ASSET,
        UI_SECTION_UNPIN_ASSET,
        UI_TOOL_CARD,
        UI_TOOL_OPTIONS,
        UI_TOOL_OPTIONS_EXPAND,
        UI_TOOL_OPTIONS_COLLAPSE,
        UI_NO_TOOL_PARAMETERS,
        UI_LAST_OPERATION,
        UI_STATS_VERTS,
        UI_STATS_FACES,
        UI_STATS_TRIS,
        UI_STATS_SELECTION,
        UI_QUICK_ACTIONS,
        UI_QUICK_ACTION_CUSTOMIZE,
        UI_QUICK_ACTION_ADD,
        UI_QUICK_ACTION_REMOVE,
        UI_QUICK_ACTION_RESET,
        UI_QUICK_ACTION_DONE,
        UI_ACTION_SUBDIVIDE,
        UI_ACTION_FUSE,
        UI_ACTION_CUT,
        UI_ACTION_INTERSECT,
        UI_ACTION_JOIN,
        UI_ACTION_MERGE,
        UI_ACTION_SLICE,
        UI_ACTION_LOOP_CUT,
        UI_MATERIAL_EDITOR,
        UI_MATERIAL_NAME,
        UI_MATERIAL_BASE_COLOR,
        UI_MATERIAL_ASSIGN,
        UI_MATERIAL_NEW,
        UI_MATERIAL_DUPLICATE,
        UI_MATERIAL_REMOVE,
        UI_MATERIAL_NO_MATERIAL,
        UI_MATERIAL_NO_SELECTION,
        UI_MATERIAL_PROFILE,
        UI_MATERIAL_ROUGHNESS,
        UI_MATERIAL_METALLIC,
        UI_MATERIAL_NORMAL_SCALE,
        UI_MATERIAL_EMISSION,
        UI_MATERIAL_EMISSION_STRENGTH,
        UI_MATERIAL_ALPHA_MODE,
        UI_MATERIAL_ALPHA_CUTOFF,
        UI_MATERIAL_TEXTURE_ALBEDO,
        UI_MATERIAL_NO_TEXTURE,
        UI_MATERIAL_CREATE_TEXTURE,
        UI_MATERIAL_CLEAR_TEXTURE,
        UI_MATERIAL_PROFILE_PBR,
        UI_MATERIAL_PROFILE_UNLIT,
        UI_MATERIAL_PROFILE_TOON,
        UI_MATERIAL_PROFILE_GLASS,
        UI_MATERIAL_PROFILE_EMISSIVE,
        UI_MATERIAL_ALPHA_OPAQUE,
        UI_MATERIAL_ALPHA_MASK,
        UI_MATERIAL_ALPHA_BLEND,
        UI_MATERIAL_ADVANCED,
        UI_MODIFIERS_EMPTY,
        UI_MODIFIER_MIRROR,
        UI_MODIFIER_SYMMETRY,
        UI_MODIFIER_APPLY,
        UI_MODIFIER_DIRECTION,
        UI_MODIFIER_POSITIVE_TO_NEGATIVE,
        UI_MODIFIER_NEGATIVE_TO_POSITIVE,
        UI_MODIFIER_ADD_MIRROR,
        UI_MODIFIER_ADD_SYMMETRY,
        UI_MODIFIER_AXIS,
        UI_MODIFIER_REMOVE,
        UI_MODIFIER_MOVE_UP,
        UI_MODIFIER_MOVE_DOWN,
        UI_PARTS_COLLECTION,
        UI_PARTS_ANNOTATIONS,
        UI_PARTS_MEASUREMENTS,
        UI_PARTS_ISOLATE,
        UI_PARTS_EXIT_ISOLATE,
        UI_PARTS_NEW_COLLECTION,
        UI_INSPECTOR,
        UI_NUMERIC_FIELD_HINT,
        TOOLS_SELECT,
        TOOLS_ROTATE,
        TOOLS_SCALE,
        TOOLS_TRANSFORM,
        TOOLS_SELECT_LASSO,
        TRANSFORM_POSITION,
        TOOLS_LOOP_CUT,
        TOOLS_SLICE,
        TOOLS_PUSH_PULL,
        TOOLS_DRAW_PROFILE,
        TOOLS_PIVOT,
        UI_PIVOT_HINT,
        UI_LOOP_CUT_HINT,
        UI_SLICE_HINT,
        UI_PUSH_PULL_HINT,
        UI_PROFILE_HINT,
        UI_PROFILE_DEPTH,
        UI_PROFILE_POINTS,
        UI_PROFILE_CLOSE,
        UI_PROFILE_GENERATE,
        UI_PROFILE_REVOLVE,
        UI_PROFILE_CUTS,
        UI_PROFILE_PRESETS,
        UI_PROFILE_ADD_RECT,
        UI_PROFILE_ADD_CIRCLE,
        UI_PROFILE_CANVAS_HINT,
        UI_PROFILE_WALL_THICKNESS,
        UI_PROFILE_SMOOTH_CURVES,
        UI_PROFILE_SHARP_CORNERS,
        UI_PROFILE_SMOOTHNESS,
        UI_PROFILE_SWEEP,
        UI_DECAL_TRANSFORM,
        UI_DECAL_POSITION,
        UI_DECAL_SCALE,
        UI_DECAL_ROTATION,
        UI_DECAL_BAKE,
        UI_DECAL_HINT,
        UI_PRIMITIVE_PARAMETRIC,
        UI_PRIMITIVE_FREEZE,
        UI_PRIMITIVE_FREEZE_HINT,
        UI_PRIMITIVE_FROZEN_STATUS,
        UI_MODEL_SELECT_HINT,
        UI_MODEL_POSITION_HINT,
        UI_MODEL_ROTATE_HINT,
        UI_MODEL_SCALE_HINT,
        UI_MODEL_TRANSFORM_HINT,
        UI_MODEL_LASSO_HINT,
        UI_HIDE_PART,
        UI_SHOW_PART,
        UI_LOCK_PART,
        UI_UNLOCK_PART,
        UI_PROJECT_ASSET_LIBRARY,
        UI_SAVE_ACTIVE_AS_ASSET,
        UI_STATUS_HINT,
        UI_WIRE_OVERLAY,
        UI_WIRE_OVERLAY_HINT,
        UI_VIEW_WIREFRAME_HINT,
        UI_VIEW_SOLID_HINT,
        UI_VIEW_MATERIAL_HINT,
        UI_VIEW_LIT_HINT,
        UI_UNWRAP_MESH,
        UI_PACK_ISLANDS,
        UI_ACTIVE_BRUSH_COLOR,
        UI_ALBEDO_BASE_COLOR,
        UI_THEME,
        UI_PLACE_IN_SCENE,
        UI_RECOVERY_TITLE,
        UI_RECOVERY_BODY,
        UI_RECOVERY_RECOVER,
        UI_RECOVERY_KEEP,
        UI_RECOVERY_DISCARD,
        ACTIONS_APPLY,
        ACTIONS_CANCEL,
        ACTIONS_DELETE,
        ACTIONS_DUPLICATE,
        MENU_FILE,
        MENU_EDIT,
        MENU_VIEW,
        MENU_WINDOW,
        MENU_COMMAND_PALETTE,
        MENU_PREFERENCES,
        PREFERENCES_COLORBLIND_AXES,
        PREFERENCES_REDUCED_MOTION,
        PREFERENCES_DOUBLE_TAP_INTERVAL,
        PREFERENCES_MULTISELECTION_MEASURE,
        FILE_NEW,
        FILE_OPEN_PROJECT,
        FILE_SAVE,
        FILE_SAVE_AS,
        FILE_IMPORT_OBJ,
        EDIT_UNDO,
        EDIT_REDO,
        VIEW_FRAME,
        VIEW_FRAME_ALL,
        VIEW_TOGGLE_PROJECTION,
        VIEW_RESET_CAMERA,
        VIEW_TOGGLE_WIREFRAME,
        VIEW_TOGGLE_SPLIT,
        UV_TITLE,
        UV_SELECTED,
        UV_FACES,
        UV_PREVIEW_3D,
        UV_HINT,
        ANIMATE_HUMANOID,
        ANIMATE_AUTO_RIG,
        ANIMATE_PLAY,
        ANIMATE_PAUSE,
        ANIMATE_FIRST_FRAME,
        ANIMATE_LAST_FRAME,
        ANIMATE_FRAME,
        ANIMATE_TIP_FIRST,
        ANIMATE_TIP_PREV,
        ANIMATE_TIP_PLAY,
        ANIMATE_TIP_NEXT,
        ANIMATE_TIP_LAST,
        PAINT_RADIUS,
        PAINT_COLOR,
        SETTINGS_INTERFACE,
        SETTINGS_IMPORT_EXPORT,
        SETTINGS_SHOW_SHELF,
        SETTINGS_RESET_WORKSPACE,
        SETTINGS_RESET_ALL,
        SETTINGS_EXPORT_GLB,
        SETTINGS_EXPORT_GLB_HINT,
        REFS_VISIBLE,
        REFS_LOCK,
        REFS_CLICK_TO_LOAD,
        REFS_NO_IMAGE,
        REFS_REPLACE,
        REFS_REMOVE,
        REFS_ALIGN_VIEW,
        REFS_RESET_DEFAULT,
        REFS_FINE_TUNE,
        REFS_LOADED,
        REFS_OPACITY,
        REFS_SIZE,
        REFS_OFFSET,
        REFS_ROTATION,
        PRIMS_SIZE,
        PRIMS_RADIUS,
        PRIMS_SEGMENTS,
        PRIMS_RINGS,
        PRIMS_HEIGHT,
        PRIMS_SIDES,
        PRIMS_WIDTH,
        PRIMS_CONFIRM,
        PRIMS_CANCEL,
        PRIMS_REOPEN,
        PRIMS_CONFIRM_HINT,
        PRIMS_CUBE,
        PRIMS_PLANE,
        PRIMS_CYLINDER,
        PRIMS_SPHERE,
        PRIMS_CONE,
        PRIMS_CAPSULE,
        PRIMS_WEDGE,
        PRIMS_CIRCLE,
        PRIMS_TORUS,
        PRIMS_ICOSPHERE,
        PRIMS_GROUP_BASIC,
        PRIMS_GROUP_ROUND,
        PRIMS_GROUP_ORGANIC,
        PRIMS_DEPTH,
        PRIMS_TOP_RADIUS,
        PRIMS_BOTTOM_RADIUS,
        PRIMS_MAJOR_RADIUS,
        PRIMS_MINOR_RADIUS,
        PRIMS_VERTICES,
        PRIMS_FILL,
        PRIMS_CAP,
        PRIMS_SUBDIV,
        PRIMS_BODY_LENGTH,
        PRIMS_RESET,
        PRIMS_TRIS,
        PRIMS_CAP_BOTH,
        PRIMS_CAP_TOP,
        PRIMS_CAP_BOTTOM,
        PRIMS_CAP_NONE,
        PRIMS_FILL_NONE,
        PRIMS_FILL_DISC,
        PRIMS_TIP_SIDES,
        PRIMS_TIP_TOP_RADIUS,
        PRIMS_TIP_SUBDIV,
        PRIMS_TIP_MAJOR_RADIUS,
        PRIMS_TIP_MINOR_RADIUS,
        PRIMS_TIP_SEGMENTS,
        PRIMS_TIP_RINGS,
        PRIMS_TIP_VERTICES,
        PRIMS_TIP_FILL,
        PRIMS_TIP_CAPS,
        PRIMS_TIP_BODY_LENGTH,
        PIVOT_MEDIAN,
        PIVOT_BOUNDS,
        PIVOT_CURSOR,
        PIVOT_INDIVIDUAL,
        PIVOT_ORIGIN_TO_GEOMETRY,
        PIVOT_ORIGIN_TO_BOTTOM,
        PIVOT_ORIGIN_TO_CURSOR,
        PIVOT_ORIGIN_TO_SELECTION,
        PIVOT_GEOMETRY_TO_ORIGIN,
        PIVOT_EDIT_PIVOT,
        PIVOT_EDIT_PIVOT_HINT,
    ];
}

fn locales_dir() -> PathBuf {
    // 1. ./assets/locales (dev, executando da raiz)
    // 2. ao lado do executável (instalado)
    for cand in ["assets/locales", "locales"] {
        let p = PathBuf::from(cand);
        if p.exists() {
            return p;
        }
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        for cand in ["assets/locales", "locales", "../../assets/locales"] {
            let p = dir.join(cand);
            if p.exists() {
                return p;
            }
        }
    }
    PathBuf::from("assets/locales")
}

fn discover_available_locales() -> Vec<String> {
    let mut out = vec!["en".to_string()];
    if let Ok(entries) = fs::read_dir(locales_dir()) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "toml")
                && let Some(stem) = path.file_stem().and_then(|stem| stem.to_str())
                && stem != "en"
                && !out.iter().any(|locale| locale == stem)
            {
                out.push(stem.to_string());
            }
        }
    }
    out.sort();
    out
}

/// Locales embutidos como reserva (Wave 7): binários instalados sem o diretório
/// de locales e testes executados fora da raiz nunca voltam a exibir chaves.
const EN_EMBEDDED: &str = include_str!("../../../assets/locales/en.toml");
const PT_BR_EMBEDDED: &str = include_str!("../../../assets/locales/pt-BR.toml");

fn embedded(lang: &str) -> &'static str {
    match lang {
        "pt-BR" => PT_BR_EMBEDDED,
        _ => EN_EMBEDDED,
    }
}

fn load_file(lang: &str) -> HashMap<String, String> {
    let path = locales_dir().join(format!("{lang}.toml"));
    let text = fs::read_to_string(&path).unwrap_or_default();
    if text.is_empty() {
        return I18n::parse(embedded(lang));
    }
    I18n::parse(&text)
}

fn flatten(v: &toml::Value, prefix: String, out: &mut HashMap<String, String>) {
    match v {
        toml::Value::Table(t) => {
            for (k, vv) in t {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten(vv, key, out);
            }
        }
        toml::Value::String(s) => {
            out.insert(prefix, s.clone());
        }
        other => {
            out.insert(prefix, other.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::text_id::ALL;
    use super::{I18n, pseudo_transform};

    const EN_TOML: &str = include_str!("../../../assets/locales/en.toml");
    const PT_TOML: &str = include_str!("../../../assets/locales/pt-BR.toml");

    fn keys_of(toml_text: &str) -> Vec<String> {
        let mut keys: Vec<String> = I18n::parse(toml_text).keys().cloned().collect();
        keys.sort();
        keys
    }

    #[test]
    fn locale_parity_en_ptbr() {
        // Paridade exata: CI falha com chave faltante ou órfã (Wave 7 — §12.3).
        let en = keys_of(EN_TOML);
        let pt = keys_of(PT_TOML);
        let missing: Vec<&String> = en.iter().filter(|k| !pt.contains(k)).collect();
        let orphan: Vec<&String> = pt.iter().filter(|k| !en.contains(k)).collect();
        assert!(missing.is_empty(), "pt-BR sem tradução: {missing:?}");
        assert!(orphan.is_empty(), "pt-BR com chaves órfãs: {orphan:?}");
        assert!(!en.is_empty());
    }

    #[test]
    fn text_id_catalog_resolves_in_both_locales() {
        let en = I18n::parse(EN_TOML);
        let pt = I18n::parse(PT_TOML);
        assert_eq!(en.len(), pt.len());
        for id in ALL {
            assert!(en.contains_key(id.key()), "en sem {}", id.key());
            assert!(pt.contains_key(id.key()), "pt-BR sem {}", id.key());
            assert!(!en[id.key()].is_empty() && !pt[id.key()].is_empty());
        }
    }

    #[test]
    fn pseudo_expands_and_marks() {
        let out = pseudo_transform("Settings");
        assert!(out.starts_with('⟦') && out.ends_with('⟧'));
        assert!(out.len() > "Settings".len());
        // Texto vazio não quebra.
        assert_eq!(pseudo_transform(""), "⟦⟧");
    }

    #[test]
    fn pseudo_locale_covers_catalog() {
        let base = I18n {
            lang: "en".to_string(),
            map: I18n::parse(EN_TOML),
            fallback: I18n::parse(EN_TOML),
        };
        let pseudo = I18n::pseudo_from(&base);
        assert_eq!(pseudo.lang, "pseudo");
        for id in ALL {
            let s = pseudo.t_id(*id);
            assert!(s.starts_with('⟦'), "{} sem marca pseudo", id.key());
        }
    }
}
