//! Qt adapter for the existing editor controller. The controller owns the sole
//! Application/Document/Undo stack; Qt owns widgets, focus and GL contexts.
use petunia_application::Application;
use petunia_core::{SelectionDomain, Workspace};
use petunia_editor::{ModelingMode, SlintUiBridge, TransformKind, UiIntent};
use serde_json::{Value, json};
use std::{
    ffi::{CStr, CString, c_char},
    ops::{Deref, DerefMut},
    path::Path,
};

/// Modo do gizmo de decalque escolhido no painel PAINT. `Auto` mantém a
/// gramática das alças (o acerto decide entre pincel e transformação); os
/// outros modos enviesam o arrasto da viewport para a transformação pedida.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DecalGizmoMode {
    #[default]
    Auto,
    Move,
    Rotate,
    Scale,
}

impl DecalGizmoMode {
    fn parse(id: &str) -> Option<Self> {
        match id {
            "auto" => Some(Self::Auto),
            "move" => Some(Self::Move),
            "rotate" => Some(Self::Rotate),
            "scale" => Some(Self::Scale),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Move => "move",
            Self::Rotate => "rotate",
            Self::Scale => "scale",
        }
    }

    /// Modificadores que a gramática de decalque usa para escolher o gesto
    /// (`is_shift` escala, `is_ctrl` gira) e que repetimos ao arrastar.
    fn modifiers(self) -> (bool, bool) {
        match self {
            Self::Scale => (true, false),
            Self::Rotate => (false, true),
            Self::Auto | Self::Move => (false, false),
        }
    }
}

pub struct QtEditor {
    pub controller: SlintUiBridge<QtViewportCapture>,
    tablet_input: Option<TabletInput>,
    session_lease: Option<SessionLease>,
    /// Pegada do pincel 3D sob o último ponto consultado: achatamento e giro
    /// do anel, como o Slint publica em `brush-cursor-squash/tilt`.
    brush_footprint: Option<[f32; 2]>,
    /// Alça do decalque sob o ponteiro (20 mover, 21 escalar, 22 girar).
    decal_hover_handle: Option<u8>,
    decal_gizmo: DecalGizmoMode,
}
impl Default for QtEditor {
    fn default() -> Self {
        Self::new(Application::default())
    }
}
impl QtEditor {
    pub fn new(mut application: Application) -> Self {
        let prefs = petunia_config::UserPreferences::load();
        petunia_editor::DesktopHost::apply_preferences(&mut application.state, &prefs);
        let mut controller =
            SlintUiBridge::from_application(application, QtViewportCapture::default());
        controller.preferences = prefs;
        controller.restore_section_layouts(&controller.preferences.clone());
        controller.all_brush_presets();
        controller.pending_recovery = if SessionLease::active_owner() {
            None
        } else {
            petunia_core::AutosaveService::detect_recovery(None)
        };
        Self {
            controller,
            tablet_input: None,
            session_lease: None,
            brush_footprint: None,
            decal_hover_handle: None,
            decal_gizmo: DecalGizmoMode::Auto,
        }
    }
    pub fn execute_registered_command(
        &mut self,
        id: &str,
    ) -> Result<(), petunia_core::CommandError> {
        if id == "file.recover_last_session" {
            self.recover()
                .map_err(petunia_core::CommandError::Execution)?;
            return Ok(());
        }
        if id == "file.discard_recovery" {
            self.discard_recovery()
                .map_err(petunia_core::CommandError::Execution)?;
            return Ok(());
        }
        self.controller.execute_core_command(id)
    }
    pub fn undo(&mut self) -> bool {
        let available = self.controller.view_model().can_undo;
        self.controller.apply(UiIntent::Undo);
        available
    }
    pub fn redo(&mut self) -> bool {
        let available = self.controller.view_model().can_redo;
        self.controller.apply(UiIntent::Redo);
        available
    }
    pub fn execute_command(
        &mut self,
        command: &dyn petunia_core::Command,
    ) -> Result<(), petunia_core::CommandError> {
        self.controller.state.dispatch(command)
    }
    pub fn snapshot(&self) -> Value {
        let vm = self.controller.view_model();
        let mut v = crate::editor_snapshot::snapshot(&vm);
        v["uv_selection_stamp"] = json!(uv_selection_stamp(&self.state.session.uv_selection));
        v["document_revision"] = json!(self.state.project.project.revision_id().to_string());
        v["active_asset_id"] = json!(self.state.project.active().map(|a| a.id.to_string()));
        v["selected_count"] = json!(self.state.session.selection.assets.len());
        v["project_path"] = json!(self.state.project.project_path);
        // Native per-axis forms consume the same slot projection as Slint.
        for slot in v["reference_slots"].as_array().cloned().unwrap_or_default() {
            if let Some(axis) = slot["axis"].as_str() {
                v[format!("reference_{}", axis.to_lowercase())] = slot.clone();
            }
        }
        let mut reference_axes = std::collections::HashSet::new();
        for r in &self.state.project.refs {
            let axis = if r.axis == petunia_project::RefAxis::Side {
                "right".to_owned()
            } else {
                format!("{:?}", r.axis).to_lowercase()
            };
            if !reference_axes.insert(axis.clone()) {
                continue;
            }
            let key = format!("reference_{axis}");
            v[&key]["revision"] = json!(r.revision);
        }
        v["draw"] = self.controller.draw_workspace_snapshot();
        v["recovery_available"] = json!(self.controller.pending_recovery.is_some());
        // Feedback do pincel e do gizmo de decalque (mesma projeção do Slint).
        let (brush_squash, brush_tilt) = self
            .brush_footprint
            .map_or((1.0, 0.0), |[squash, tilt]| (squash, tilt));
        v["brush_squash"] = json!(brush_squash);
        v["brush_tilt"] = json!(brush_tilt);
        v["brush_flow"] = json!(vm.brush_panel.flow);
        // Sobe a cada mutação de pixels de textura/canvas: o host nativo usa
        // essa revisão para reler o raster em vez de redesenhar às cegas.
        v["texture_revision"] = json!(self.state.project.project.texture_revision);
        v["decal_gizmo_mode"] = json!(self.decal_gizmo.as_str());
        v["decal_hover_handle"] = match self.decal_hover_handle {
            Some(id) if id == petunia_editor::DECAL_HANDLE_ROTATE => json!("rotate"),
            Some(id) if id == petunia_editor::DECAL_HANDLE_SCALE => json!("scale"),
            Some(id) if id == petunia_editor::DECAL_HANDLE_MOVE => json!("move"),
            _ => Value::Null,
        };
        v["decal_handles"] = self.controller.decal_handles().map_or(Value::Null, |h| {
            json!({
                "center": h.center,
                "corners": h.corners,
                "top": h.top,
                "rotate": h.rotate,
            })
        });
        if let Some(r) = &self.controller.pending_recovery {
            v["recovery"] = json!({"name":r.project_name,"path":r.snapshot_path.display().to_string(),"newer":r.is_newer_than_main});
        }
        v["primitive_parameters"] = json!(match vm.primitive_kind.as_str() {
            "box" | "cube" => vec!["width", "height", "depth"],
            "plane" => vec!["width", "height"],
            "sphere" => vec!["radius", "subdiv"],
            "cylinder" => vec!["radius", "height", "sides", "cap_top", "cap_bottom"],
            "cone" => vec![
                "radius",
                "radius_b",
                "height",
                "sides",
                "cap_top",
                "cap_bottom"
            ],
            "circle" => vec!["radius", "sides", "fill_disc"],
            "torus" => vec!["radius", "radius_b", "sides", "rings"],
            _ => vec![],
        });
        v["material_options"]=json!(vm.material_slots.iter().enumerate().map(|(i,name)|json!({"id":i.to_string(),"name":name,"active":i as i32==vm.active_material_slot})).collect::<Vec<_>>());
        v["brush_presets"]=json!(vm.brush_panel.presets.iter().enumerate().map(|(i,name)|json!({"id":i.to_string(),"name":name,"active":i as i32==vm.brush_panel.active_preset})).collect::<Vec<_>>());
        v["decal_variants"]=json!(vm.decal_variant_names.iter().enumerate().map(|(i,name)|json!({"id":i.to_string(),"name":name,"active":i as i32==vm.decal_variant_index})).collect::<Vec<_>>());
        v["keymap_profiles"] = json!(
            self.controller
                .keymap_profile_rows()
                .iter()
                .map(|p| json!({"id":p.id,"name":p.name,"active":p.active,"custom":p.custom}))
                .collect::<Vec<_>>()
        );
        v["keymap_actions"]=json!(vm.keymap_snapshot.rows.iter().map(|r|json!({"id":r.action,"name":format!("{} · {}{}",r.label,r.shortcut,if r.conflict{" ⚠"}else{""}),"capturing":r.capturing})).collect::<Vec<_>>());
        v["uv_selection_mode"] = json!(match self.state.session.uv_selection.mode {
            petunia_core::UvSelectionMode::Corner => "vertex",
            petunia_core::UvSelectionMode::Edge => "edge",
            petunia_core::UvSelectionMode::Face => "face",
            petunia_core::UvSelectionMode::Island => "island",
        });
        v["uv_selection_sync"] = json!(self.state.session.uv_selection.sync);
        v["preferences"] = json!(self.controller.preferences);
        v["available_languages"] = json!(
            petunia_config::I18n::available()
                .iter()
                .map(|id| {
                    let name = match id.as_str() {
                        "en" | "en-US" => "English",
                        "pt-BR" => "Português (Brasil)",
                        other => other,
                    };
                    let active = id == self.state.ui.i18n.lang()
                        || (id == "en" && self.state.ui.i18n.lang() == "en-US");
                    json!({"id":id,"name":name,"active":active})
                })
                .collect::<Vec<_>>()
        );
        v["animate"]["curve_mode"] = json!(self.state.session.animate.curve_mode);
        v["animate"]["curve_channel"] = json!(
            [
                "trans_x", "trans_y", "trans_z", "rot_x", "rot_y", "rot_z", "scale"
            ][vm.animate.active_curve_channel.clamp(0, 6) as usize]
        );
        v["animate"]["interpolation"] =
            json!(["step", "linear", "cubic"][vm.animate.current_interp.clamp(0, 2) as usize]);
        for slot in &vm.reference_slots {
            v[format!("reference_{}", slot.axis)] = json!({"visible":slot.visible,"locked":slot.locked,"opacity":slot.opacity,"size":slot.size,"offset":slot.offset,"rotation":slot.rotation,"name":slot.image_name,"has_image":slot.has_image});
        }
        v["animate"]["bones"]=json!(self.state.session.animate.skeleton.and_then(|id|self.state.project.project.get_skeleton(id)).map(|s|s.bones.iter().map(|bone|json!({"id":bone.id.to_string(),"name":bone.name,"selected":vm.animate.selected_bone==Some(bone.id)})).collect::<Vec<_>>()).unwrap_or_default());
        if let Some(bone) = self
            .state
            .session
            .animate
            .skeleton
            .and_then(|id| self.state.project.project.get_skeleton(id))
            .and_then(|s| {
                s.bones
                    .iter()
                    .find(|b| Some(b.id) == vm.animate.selected_bone)
            })
        {
            v["bone_translation"] = json!(bone.local_transform.translation);
            let (x, y, z) =
                glam::Quat::from_array(bone.local_transform.rotation).to_euler(glam::EulerRot::XYZ);
            v["bone_rotation"] = json!([x.to_degrees(), y.to_degrees(), z.to_degrees()]);
            v["bone_scale"] = json!(bone.local_transform.scale);
        }
        v["commands"]=json!(self.controller.search_commands("").iter().map(|c|json!({"id":c.id,"label":c.label,"enabled":c.is_available,"reason":c.disabled_reason})).collect::<Vec<_>>());
        let registry = petunia_config::theme::ThemeRegistry::global();
        if let Some(theme) = registry.get_theme(&vm.current_theme) {
            v["theme_colors"] = json!(theme.colors);
        }
        v
    }
    /// Load validates the complete file before replacing the session. Selection,
    /// previews and history never retain identifiers from the previous document.
    pub fn open(&mut self, path: &Path) -> Result<(), String> {
        let mut project = petunia_project::format::load(path).map_err(|e| e.to_string())?;
        // Persistent image revisions cannot identify textures from another session.
        for reference in &mut project.references {
            reference.bump_revision();
        }
        let prefs = self.controller.preferences.clone();
        let mut app = Application::default();
        app.state.project.palette = project.palette.clone();
        app.state.project.project = project;
        app.state
            .project
            .project
            .rebase_revisions_after_restore(self.state.project.project.revision_clock());
        app.state.project.project_path = Some(path.display().to_string());
        app.state.project.undo.clear();
        app.state.session.selection.clear();
        app.state.select_object(None, false);
        app.state.mark_document_clean();
        petunia_editor::DesktopHost::apply_preferences(&mut app.state, &prefs);
        self.controller = SlintUiBridge::from_application(app, QtViewportCapture::default());
        self.controller.preferences = prefs;
        self.controller
            .preferences
            .push_recent_project(&path.display().to_string());
        self.controller
            .restore_section_layouts(&self.controller.preferences.clone());
        self.controller
            .state
            .set_status(format!("Projeto aberto: {}", path.display()));
        Ok(())
    }
    pub fn new_document(&mut self) {
        let prefs = self.controller.preferences.clone();
        let mut app = Application::default();
        app.state.project.reset();
        app.state.session.selection.clear();
        app.state.select_object(None, false);
        app.state.mark_document_clean();
        petunia_editor::DesktopHost::apply_preferences(&mut app.state, &prefs);
        self.controller = SlintUiBridge::from_application(app, QtViewportCapture::default());
        self.controller.preferences = prefs;
        self.controller
            .restore_section_layouts(&self.controller.preferences.clone());
    }
    fn recover(&mut self) -> Result<bool, String> {
        let info = self
            .controller
            .pending_recovery
            .clone()
            .ok_or("Nenhum snapshot disponível")?;
        let recent = self.controller.preferences.recent_projects.clone();
        self.open(&info.snapshot_path)?;
        self.controller.preferences.recent_projects = recent;
        self.state.project.project_path = info.main_project_path.map(|p| p.display().to_string());
        self.state.mark_dirty();
        self.controller.pending_recovery = None;
        Ok(true)
    }
    fn discard_recovery(&mut self) -> Result<bool, String> {
        let Some(info) = self.controller.pending_recovery.clone() else {
            return Ok(false);
        };
        // Preserve the lease owned by this host and other sessions' markers.
        std::fs::remove_file(&info.snapshot_path).map_err(|e| e.to_string())?;
        self.controller.pending_recovery = None;
        Ok(true)
    }
    pub fn request(&mut self, request: &Value) -> Result<bool, String> {
        let action = string(request, "action")?;
        // Native host operations with fallible results, then typed editor intents.
        match action {
            "session_start" => {
                if self.session_lease.is_none() {
                    self.session_lease = Some(SessionLease::claim()?)
                }
                return Ok(true);
            }
            "session_end" => {
                self.session_lease.take();
                return Ok(true);
            }
            "recover_pending" => return self.recover(),
            "discard_pending_recovery" => return self.discard_recovery(),
            // Feedback do pincel 3D: o anel do cursor segue a superfície
            // inclinada (achatamento e giro). Fora do objeto volta a círculo.
            "brush_footprint" => {
                let x = number(request, "x")?;
                let y = number(request, "y")?;
                self.brush_footprint = if x.is_finite() && y.is_finite() {
                    self.controller
                        .brush_footprint_at(x, y)
                        .map(|(squash, tilt)| [squash, tilt])
                } else {
                    None
                };
                return Ok(self.brush_footprint.is_some());
            }
            // Alça do decalque sob o ponteiro, para o realce do gizmo.
            "decal_hover" => {
                let x = number(request, "x")?;
                let y = number(request, "y")?;
                self.decal_hover_handle = if x.is_finite() && y.is_finite() {
                    self.controller.decal_handle_at(x, y)
                } else {
                    None
                };
                return Ok(self.decal_hover_handle.is_some());
            }
            "decal_gizmo_mode" => {
                self.decal_gizmo = DecalGizmoMode::parse(string(request, "id")?)
                    .ok_or("Modo do gizmo inválido")?;
                return Ok(true);
            }
            "export_obj" => {
                let index = self.state.project.active;
                petunia_core::ProjectService::export_obj(
                    &self.state,
                    index,
                    Path::new(string(request, "path")?),
                )
                .map_err(|e| e.to_string())?;
                return Ok(true);
            }
            "export_glb" => {
                petunia_core::ProjectService::export_glb_with_padding(
                    &self.state,
                    &self.state.export_selected_indices(),
                    Path::new(string(request, "path")?),
                    0,
                )
                .map_err(|e| e.to_string())?;
                return Ok(true);
            }
            "tablet_input" => {
                self.tablet_input = Some(TabletInput {
                    pressure: number(request, "pressure")?.clamp(0., 1.),
                    tilt_x: number(request, "tilt_x")?.clamp(-90., 90.),
                    tilt_y: number(request, "tilt_y")?.clamp(-90., 90.),
                    eraser: boolean(request, "eraser")?,
                });
                return Ok(true);
            }
            "tablet_reset" => {
                self.tablet_input = None;
                return Ok(true);
            }
            "project.new" | "file.new" => {
                self.new_document();
                return Ok(true);
            }
            "project.new_empty" | "file.new_empty" => {
                self.new_document();
                self.state.project.assets.clear();
                self.state.project.active = usize::MAX;
                self.state.session.selection.clear();
                self.state.select_object(None, false);
                self.state.project.bump_topology();
                self.state.mark_document_clean();
                return Ok(true);
            }
            "project.open" => {
                self.open(Path::new(string(request, "path")?))?;
                return Ok(true);
            }
            "project.save" => {
                let path = Path::new(string(request, "path")?);
                petunia_core::ProjectService::save_project(&mut self.state, path)
                    .map_err(|e| e.to_string())?;
                self.controller
                    .preferences
                    .push_recent_project(&path.display().to_string());
                return Ok(true);
            }
            "command" => {
                self.execute_registered_command(string(request, "id")?)
                    .map_err(|e| e.to_string())?;
                return Ok(true);
            }
            _ => {}
        }
        let owns_session = self.session_lease.is_some();
        let tablet = self.tablet_input;
        let decal_gizmo = self.decal_gizmo;
        let b = &mut self.controller;
        match action {
            "set_extension_parameter" => {
                return Ok(
                    b.set_extension_parameter(string(request, "key")?, string(request, "text")?)
                );
            }
            "paint_2d_select" => {
                return Ok(b.paint_2d_select(
                    number(request, "x0")?,
                    number(request, "y0")?,
                    number(request, "x1")?,
                    number(request, "y1")?,
                    boolean(request, "extend")?,
                    boolean(request, "subtract")?,
                    boolean(request, "island")?,
                ));
            }
            "gizmo_begin" => {
                let x = number(request, "x")?;
                let y = number(request, "y")?;
                return Ok(b.gizmo_handle_at(x, y).is_some() && b.begin_gizmo_drag(x, y));
            }
            "uv_drag_commit" => {
                if string(request, "revision")? != b.state.project.project.revision_id().to_string()
                    || b.state
                        .project
                        .active()
                        .map(|a| a.id.to_string())
                        .as_deref()
                        != Some(string(request, "asset")?)
                {
                    return Err("O documento mudou durante o gesto UV".into());
                }
                if string(request, "selection")?
                    != uv_selection_stamp(&b.state.session.uv_selection)
                {
                    return Err("A seleção mudou durante o gesto UV".into());
                }
                return Ok(b.uv_move_selected(number(request, "du")?, number(request, "dv")?));
            }
            "select" => {
                return Ok(b.select_scene_item(
                    string(request, "id")?,
                    request["extend"].as_bool().unwrap_or(false),
                ));
            }
            "scene_selection" => {
                let profile = request["active"]
                    .as_str()
                    .filter(|s| s.starts_with("profile:"));
                if let Some(profile) = profile {
                    return Ok(b.select_scene_item(profile, false));
                }
                let ids = request["ids"].as_array().ok_or("Seleção inválida")?;
                if ids.len() > 10000 {
                    return Err("Seleção grande demais".into());
                }
                b.state.select_object(None, false);
                for id in ids {
                    if let Some(index) = id
                        .as_str()
                        .and_then(|s| uuid::Uuid::parse_str(s).ok())
                        .and_then(|id| b.state.project.assets.iter().position(|a| a.id == id))
                    {
                        b.state.select_object(Some(index), true);
                        b.state.session.active_shape = None;
                    }
                }
                if let Some(active) = request["active"]
                    .as_str()
                    .and_then(|s| uuid::Uuid::parse_str(s).ok())
                    .and_then(|id| b.state.project.assets.iter().position(|a| a.id == id))
                    && b.state
                        .session
                        .selection
                        .assets
                        .contains(&b.state.project.assets[active].id)
                {
                    b.state.project.active = active;
                }
                b.state.sync_selection();
                b.reset_transform_fields();
                return Ok(true);
            }
            "section_state" => {
                use petunia_config::InspectorSectionId as S;
                let section = match string(request, "section")? {
                    "parts" => S::Parts,
                    "transform" => S::Transform,
                    "material" => S::Material,
                    "object" => S::Object,
                    "modifiers" => S::Modifiers,
                    "quick-actions" | "quick_actions" => S::QuickActions,
                    _ => return Err("Seção inválida".into()),
                };
                match string(request, "field")? {
                    "docked" => b.apply(UiIntent::SetSectionDocked {
                        section,
                        docked: boolean(request, "value")?,
                    }),
                    "pin_open" => b.apply(UiIntent::SetSectionPinOpen {
                        section,
                        pin_open: boolean(request, "value")?,
                    }),
                    "pinned_asset" => {
                        let asset = if boolean(request, "value")? {
                            b.state.project.active().map(|a| a.id.to_string())
                        } else {
                            None
                        };
                        b.apply(UiIntent::SetSectionPinnedAsset { section, asset })
                    }
                    _ => return Err("Layout inválido".into()),
                };
                return Ok(true);
            }
            "part_selection_color" => {
                let color = if request["color"].is_null() {
                    None
                } else {
                    let rgb = request["color"].as_array().ok_or("Cor inválida")?;
                    if rgb.len() != 3 {
                        return Err("Cor inválida".into());
                    }
                    let mut value = [0; 3];
                    for (i, c) in rgb.iter().enumerate() {
                        value[i] = c.as_u64().filter(|v| *v <= 255).ok_or("Cor inválida")? as u8;
                    }
                    Some(value)
                };
                return Ok(b.set_part_selection_color(string(request, "id")?, color));
            }
            "consume_host_requests" => {
                use petunia_editor::overlay::OverlayId as O;
                b.command_search_visible = false;
                b.settings_visible = false;
                b.reference_manager_open = false;
                b.asset_library_visible = false;
                b.add_menu_open = false;
                for id in [
                    O::CommandPalette,
                    O::Settings,
                    O::ReferenceManager,
                    O::AssetLibrary,
                    O::AddMenu,
                ] {
                    b.overlays.remove(id);
                }
                return Ok(true);
            }
            "scene_context" => {
                b.open_context_menu(string(request, "id")?, 0., 0.);
                return Ok(b.context_menu_action(string(request, "target")?));
            }
            "rename" => {
                let id =
                    uuid::Uuid::parse_str(string(request, "id")?).map_err(|e| e.to_string())?;
                let index = b
                    .state
                    .project
                    .assets
                    .iter()
                    .position(|a| a.id == id)
                    .ok_or("Objeto ausente")?;
                if b.state.project.assets[index].locked {
                    return Err("Objeto bloqueado".into());
                }
                b.apply(UiIntent::SelectSceneAsset(id.to_string()));
                b.begin_rename();
                return Ok(b.commit_rename(string(request, "name")?));
            }
            "scene_move" => {
                b.apply(UiIntent::MoveSceneAsset {
                    id: string(request, "id")?.into(),
                    delta: integer(request, "delta")? as i32,
                });
                return Ok(true);
            }
            "viewport_select" => {
                use petunia_editor::viewport_intents::{
                    NormalizedViewportPoint, ViewportSelectionIntent,
                };
                b.apply_viewport_selection(ViewportSelectionIntent::Point {
                    at: NormalizedViewportPoint::new(number(request, "x")?, number(request, "y")?)
                        .ok_or("Ponto inválido")?,
                    extend: request["extend"].as_bool().unwrap_or(false),
                    loop_select: request["loop"].as_bool().unwrap_or(false),
                });
                b.reset_transform_fields();
                return Ok(true);
            }
            "region_select" => {
                use petunia_editor::viewport_intents::{
                    NormalizedViewportPoint as P, RegionSelectionMode as M, ViewportLassoPolygon,
                    ViewportSelectionIntent as I,
                };
                let mode = M::from_flags(
                    request["extend"].as_bool().unwrap_or(false),
                    request["subtract"].as_bool().unwrap_or(false),
                );
                let intent = if request["lasso"].as_bool().unwrap_or(false) {
                    I::Lasso {
                        polygon: ViewportLassoPolygon::from_slint_path(string(request, "path")?)
                            .ok_or("Laço inválido")?,
                        mode,
                    }
                } else {
                    I::Box {
                        from: P::new(number(request, "x0")?, number(request, "y0")?)
                            .ok_or("Origem inválida")?,
                        to: P::new(number(request, "x1")?, number(request, "y1")?)
                            .ok_or("Destino inválido")?,
                        mode,
                    }
                };
                b.apply_viewport_selection(intent);
                b.reset_transform_fields();
                return Ok(true);
            }
            "place_cursor_3d" => {
                let norm_x = number(request, "x").or_else(|_| number(request, "norm_x"))?;
                let norm_y = number(request, "y").or_else(|_| number(request, "norm_y"))?;
                return Ok(b.place_cursor_3d(norm_x, norm_y));
            }
            "keymap_select" => return Ok(false),
            "preferences_save" => {
                let p = &mut b.preferences;
                let state = &b.state;
                p.invert_vertical_drag = state.ui.invert_vertical_drag;
                p.colorblind_axes = state.ui.colorblind_axes;
                p.reduced_motion = state.ui.reduced_motion;
                p.multiselection_measure_tag = state.ui.multiselection_measure_tag;
                p.selection_rgb = state.ui.selection_rgb;
                p.selection_thickness = state.ui.selection_thickness;
                p.model_quick_actions = state.ui.model_quick_actions.clone();
                p.active_keymap_id = state.ui.active_keymap_id.clone();
                p.theme_id = state.ui.active_theme_id.clone();
                p.save().map_err(|e| e.to_string())?;
                return Ok(true);
            }
            "material_slot" | "material_profile" | "material_alpha" | "apply_brush_preset"
            | "decal_variant" => {
                let index = request["index"]
                    .as_i64()
                    .or_else(|| request["index"].as_str().and_then(|s| s.parse().ok()));
                let index = if let Some(i) = index {
                    i
                } else {
                    match request["index"].as_str().unwrap_or("") {
                        "Pbr" | "Opaque" | "pbr" | "opaque" => 0,
                        "Unlit" | "Mask" | "unlit" | "mask" => 1,
                        "Toon" | "Blend" | "toon" | "blend" => 2,
                        "Glass" | "glass" => 3,
                        "Emissive" | "emissive" => 4,
                        _ => return Err("Opção inválida".into()),
                    }
                };
                if index < 0 {
                    return Err("Índice inválido".into());
                }
                return Ok(match action {
                    "material_slot" => b.select_material_slot(index as i32),
                    "material_profile" => b.set_active_material_profile(index as i32),
                    "material_alpha" => b.set_active_material_alpha_mode(index as i32),
                    "apply_brush_preset" => b.apply_brush_preset(index as usize),
                    _ => {
                        b.apply(UiIntent::SetDecalVariant(index as usize));
                        true
                    }
                });
            }
            "material_assign" => {
                let slot = b.active_material_slot.max(0) as usize;
                return Ok(b.assign_material_slot(slot));
            }
            "material_duplicate" => {
                let slot = b.active_material_slot.max(0) as usize;
                return Ok(b.duplicate_material(slot));
            }
            "material_remove" => {
                let slot = b.active_material_slot;
                return Ok(b.remove_material(slot));
            }
            "brush_preset_remove" => {
                let index = b.active_brush_preset.ok_or("Selecione um preset")?;
                return Ok(b.delete_brush_preset(index));
            }
            "profile_dimension_gesture" => {
                b.profile_dimension_gesture(
                    i32::try_from(integer(request, "phase")?).map_err(|e| e.to_string())?,
                );
                return Ok(true);
            }
            "draw_generator_gesture" => {
                return Ok(b.generator_parameter_gesture(
                    i32::try_from(integer(request, "phase")?).map_err(|e| e.to_string())?,
                ));
            }
            "draw_generator_parameter" => {
                return Ok(b.edit_selected_generator_parameter(
                    string(request, "field")?,
                    number(request, "value")?,
                ));
            }
            "draw_rectangle_tool" => {
                b.apply(UiIntent::SetActiveTool("draw_rectangle".into()));
                return Ok(true);
            }
            "draw_circle_tool" => {
                b.apply(UiIntent::SetActiveTool("draw_circle".into()));
                return Ok(true);
            }
            "profile_dimension" => {
                let vm = b.view_model();
                let value = number(request, "value")?;
                let field = string(request, "field")?;
                return Ok(b.set_profile_dimensions(
                    if field == "width" {
                        value
                    } else {
                        vm.profile_width
                    },
                    if field == "height" {
                        value
                    } else {
                        vm.profile_height
                    },
                    if field == "segments" {
                        value.clamp(3., 4096.) as usize
                    } else {
                        vm.profile_segments.max(3) as usize
                    },
                ));
            }
            "draw_curve_finish" => return Ok(b.finish_draw_curve()),
            "draw_curve_new" => return Ok(b.start_new_draw_curve()),
            "draw_solid_begin" => return Ok(b.begin_solid_shape_builder()),
            "draw_solid_intent" => return Ok(b.set_solid_shape_intent(string(request, "id")?)),
            "draw_solid_toggle" => {
                return Ok(b.toggle_solid_region_id(
                    string(request, "id")?
                        .parse()
                        .map_err(|_| "Invalid region id".to_string())?,
                ));
            }
            "draw_solid_cycle" => return Ok(b.cycle_solid_region()),
            "draw_solid_xray" => return Ok(b.toggle_solid_xray()),
            "draw_solid_commit" => return Ok(b.commit_solid_shape_builder()),
            "draw_solid_cancel" => return Ok(b.cancel_solid_shape_builder()),
            "draw_shape_flag" => return Ok(b.toggle_selected_shape_flag(string(request, "field")?)),
            "draw_contour" => {
                let id =
                    uuid::Uuid::parse_str(string(request, "id")?).map_err(|e| e.to_string())?;
                return Ok(b.select_draw_contour(id));
            }
            "draw_edit_context" => return Ok(b.set_draw_edit_context(string(request, "id")?)),
            "draw_generator_distance" => {
                return Ok(b.edit_selected_generator_distance(number(request, "depth")?));
            }
            "profile_workplane" => return Ok(b.set_draw_workplane(string(request, "id")?)),
            "draw_shape" => {
                let id =
                    uuid::Uuid::parse_str(string(request, "id")?).map_err(|e| e.to_string())?;
                return Ok(b.select_draw_shape(id));
            }
            "draw_sweep_path" => {
                let id =
                    uuid::Uuid::parse_str(string(request, "id")?).map_err(|e| e.to_string())?;
                return Ok(b.set_profile_sweep_path(id));
            }
            "draw_shape_name" => return Ok(b.rename_draw_shape(string(request, "name")?)),
            "draw_point" => {
                return Ok(b.select_draw_point(
                    uuid::Uuid::parse_str(string(request, "id")?).map_err(|e| e.to_string())?,
                ));
            }
            "draw_point_parameter" => {
                return Ok(b.set_draw_point_parameter(
                    string(request, "field")?,
                    number(request, "value")?,
                ));
            }
            "draw_point_mode" => return Ok(b.set_draw_point_mode(string(request, "id")?)),
            "draw_shape_transform" => {
                return Ok(
                    b.transform_draw_shape(string(request, "field")?, number(request, "value")?)
                );
            }
            "draw_angle_step" => return Ok(b.set_draw_angle_step(number(request, "value")?)),
            "draw_inference_lock" => return Ok(b.toggle_draw_inference()),
            "draw_plane_component" => {
                return Ok(b.set_draw_plane_component(
                    string(request, "field")?,
                    number(request, "value")?,
                ));
            }
            "draw_volume_option" => {
                return Ok(
                    b.set_draw_volume_option(string(request, "field")?, string(request, "value")?)
                );
            }
            "draw_offset_inner" => {
                return Ok(b.set_profile_wall_thickness(number(request, "distance")?));
            }
            "draw_make_editable" => return Ok(b.make_selected_generated_editable()),
            "uv_selection_sync" => {
                b.state.set_uv_sync(boolean(request, "value")?);
                return Ok(true);
            }
            "uv_selection_mode" => {
                b.state.session.uv_selection.mode = match string(request, "id")? {
                    "vertex" => petunia_core::UvSelectionMode::Corner,
                    "edge" => petunia_core::UvSelectionMode::Edge,
                    "face" => petunia_core::UvSelectionMode::Face,
                    "island" => petunia_core::UvSelectionMode::Island,
                    _ => return Err("Seleção UV inválida".into()),
                };
                return Ok(true);
            }
            "decal_transform_field" => {
                let d = b.active_decal().ok_or("Selecione um decalque")?;
                let v = number(request, "value")?;
                let field = string(request, "field")?;
                return Ok(b.set_active_decal_transform(
                    [
                        if field == "center_u" {
                            v
                        } else {
                            d.center_uv[0]
                        },
                        if field == "center_v" {
                            v
                        } else {
                            d.center_uv[1]
                        },
                    ],
                    [
                        if field == "scale_u" { v } else { d.scale_uv[0] },
                        if field == "scale_v" { v } else { d.scale_uv[1] },
                    ],
                    if field == "rotation_deg" {
                        v.to_radians()
                    } else {
                        d.rotation_rad
                    },
                ));
            }
            "bone_transform" => {
                let id = b
                    .state
                    .session
                    .animate
                    .selected_bone
                    .ok_or("Selecione um osso")?;
                let sid = b
                    .state
                    .session
                    .animate
                    .skeleton
                    .ok_or("Selecione uma criatura")?;
                let bone = b
                    .state
                    .project
                    .project
                    .get_skeleton(sid)
                    .and_then(|s| s.bones.iter().find(|x| x.id == id))
                    .ok_or("Osso ausente")?;
                let mut t = bone.local_transform;
                let axis = integer(request, "axis")?;
                if !(0..3).contains(&axis) {
                    return Err("Eixo inválido".into());
                }
                let value = number(request, "value")?;
                match string(request, "kind")? {
                    "translation" => t.translation[axis as usize] = value,
                    "scale" => t.scale[axis as usize] = value,
                    "rotation" => {
                        let (x, y, z) =
                            glam::Quat::from_array(t.rotation).to_euler(glam::EulerRot::XYZ);
                        let mut a = [x, y, z];
                        a[axis as usize] = value.to_radians();
                        t.rotation =
                            glam::Quat::from_euler(glam::EulerRot::XYZ, a[0], a[1], a[2]).to_array()
                    }
                    _ => return Err("Transform de osso inválido".into()),
                }
                b.state
                    .animate_set_bone_transform(id, t)
                    .map_err(|e| e.to_string())?;
                return Ok(true);
            }
            _ => {}
        }
        let intent = match action {
            "workspace" => UiIntent::SetWorkspace(match string(request, "id")? {
                "DRAW" | "POLY" | "MODEL" => Workspace::Model,
                "PAINT" => Workspace::Paint,
                "UV" => Workspace::Uv,
                "ANIMATE" => Workspace::Animate,
                _ => return Err("Workspace desconhecido".into()),
            }),
            "modeling_mode" => UiIntent::SetModelingMode(match string(request, "id")? {
                "DRAW" => ModelingMode::Draw,
                "POLY" => ModelingMode::Poly,
                _ => return Err("Modo desconhecido".into()),
            }),
            "tool" => UiIntent::SetActiveTool(string(request, "id")?.into()),
            "undo" => UiIntent::Undo,
            "redo" => UiIntent::Redo,

            "visibility" => UiIntent::ToggleSceneAssetVisibility(string(request, "id")?.into()),
            "lock" => UiIntent::ToggleSceneAssetLock(string(request, "id")?.into()),
            "select_all" => UiIntent::SelectAll,
            "clear_selection" => UiIntent::ClearSelection,
            "invert_selection" => UiIntent::InvertSelection,
            "duplicate" => UiIntent::DuplicateActiveAsset,
            "delete" => UiIntent::DeleteActiveAsset,
            "domain" => UiIntent::SetSelectionDomain(match string(request, "id")? {
                "object" => SelectionDomain::Object,
                "vertex" => SelectionDomain::Vertex,
                "edge" => SelectionDomain::Edge,
                "face" => SelectionDomain::Face,
                _ => return Err("Domínio desconhecido".into()),
            }),
            "theme" => UiIntent::SetTheme(string(request, "id")?.into()),
            "paint_color" => UiIntent::SetPaintColor([
                number(request, "r")?,
                number(request, "g")?,
                number(request, "b")?,
            ]),
            "brush_size" => UiIntent::SetBrushSize(number(request, "value")?),
            "brush_opacity" => UiIntent::SetBrushOpacity(number(request, "value")?),
            "brush_hardness" => UiIntent::SetBrushHardness(number(request, "value")?),
            "paint_target" => UiIntent::SetPaintTargetVertex(boolean(request, "value")?),
            "paint_mask" => UiIntent::SetPaintMaskSelection(boolean(request, "value")?),
            "paint_texture_side" => {
                UiIntent::SetPaintTextureSide(integer(request, "value")?.clamp(16, 8192) as u32)
            }
            "paint_grid" => UiIntent::TogglePaintPixelGrid,
            "paint_uv_overlay" => UiIntent::TogglePaintUvOverlay,
            "paint_canvas_zoom" => UiIntent::SetPaintCanvasZoom(integer(request, "value")? as i32),
            "snap" => UiIntent::ToggleSnapEnabled,
            "proportional" => UiIntent::ToggleProportionalEditing,
            "proportional_radius" => UiIntent::SetProportionalRadius(number(request, "value")?),
            "proportional_falloff" => {
                UiIntent::SetProportionalFalloff(string(request, "id")?.into())
            }
            "snap_target" => UiIntent::SetSnapTarget(string(request, "id")?.into()),
            "face_orientation" => UiIntent::ToggleFaceOrientation,
            "uv_checker" => UiIntent::ToggleUvChecker,
            "uv_texture" => UiIntent::ToggleUvShowTexture,
            "planar_navigation" => UiIntent::TogglePlanarNavigation,
            "poly_pen_mode" => UiIntent::SetPolyPenMode(string(request, "id")?.into()),
            "profile_rectangle" => UiIntent::AddProfileRectangle {
                width: number(request, "width")?,
                height: number(request, "height")?,
            },
            "profile_circle" => UiIntent::AddProfileCircle {
                radius: number(request, "radius")?,
                segments: integer(request, "segments")?.clamp(3, 4096) as usize,
            },
            "workplane_auto" => UiIntent::ProfileSetWorkplaneAuto,
            "workplane_ground" => UiIntent::ProfileSetWorkplaneGround,
            "workplane_face" => UiIntent::ProfileSetWorkplaneFace,
            "workplane_view" => UiIntent::ProfileSetWorkplaneView,
            "workplane_selection" => UiIntent::ProfileSetWorkplaneSelection,
            "workplane_look" => UiIntent::ProfileLookAtPlane,
            "freeze_primitive" => UiIntent::FreezeActivePrimitive,
            "origin_geometry" => UiIntent::SetOriginGeometry,
            "origin_bottom" => UiIntent::SetOriginBottom,
            "origin_cursor" => UiIntent::SetOriginCursor,
            "origin_selection" => UiIntent::SetOriginSelection,
            "geometry_origin" => UiIntent::SetGeometryToOrigin,
            "edit_pivot" => UiIntent::ToggleEditPivot,
            "isolate" => UiIntent::ToggleIsolateActiveAsset,
            "save_asset" => UiIntent::SaveActiveAsAsset,
            "import_model" => UiIntent::ImportModelFrom(string(request, "path")?.into()),
            "export_obj" => UiIntent::ExportActiveObjTo(string(request, "path")?.into()),
            "export_glb" => UiIntent::ExportSceneGlbTo(string(request, "path")?.into()),
            "import_svg" => UiIntent::ImportSvgProfiles(string(request, "path")?.into()),
            "import_palette" => UiIntent::ImportPalette(string(request, "path")?.into()),
            "export_palette" => UiIntent::ExportPalette(string(request, "path")?.into()),
            "import_decal" => UiIntent::ImportDecalFrom(string(request, "path")?.into()),
            "decal_variant_add" => UiIntent::AddDecalVariantFrom(string(request, "path")?.into()),
            "decal_variant" => {
                UiIntent::SetDecalVariant(integer(request, "index")?.max(0) as usize)
            }
            "decal_variant_remove" => UiIntent::RemoveDecalVariant,
            "decal_key_add" => UiIntent::SetDecalKey,
            "decal_key_remove" => UiIntent::RemoveDecalKey,
            "decal_frame" => UiIntent::SetDecalFrame(integer(request, "value")? as i32),
            "decal_fps" => UiIntent::SetDecalTrackFps(number(request, "value")?),
            "decal_length" => UiIntent::SetDecalTrackLength(integer(request, "value")? as i32),
            "decal_play" => UiIntent::ToggleDecalPlayback,
            "bake_active_decal" => UiIntent::BakeActiveDecal,
            "decal_export" => UiIntent::ExportDecalFramesTo(string(request, "path")?.into()),
            "animate" => UiIntent::Animate(
                petunia_editor::animate::AnimateIntent::parse(
                    string(request, "id")?,
                    request["arg"].as_str().unwrap_or(""),
                    request["value"]
                        .as_f64()
                        .or_else(|| request["value"].as_bool().map(|v| if v { 1. } else { 0. }))
                        .or_else(|| request["value"].as_str().and_then(|s| s.parse().ok()))
                        .unwrap_or(0.) as f32,
                )
                .ok_or("Ação de animação inválida")?,
            ),
            "transform" => {
                let kind = match string(request, "kind")? {
                    "position" => TransformKind::Position,
                    "rotation" => TransformKind::Rotation,
                    "scale" => TransformKind::Scale,
                    _ => return Err("Transform inválido".into()),
                };
                let axis = integer(request, "axis")?;
                if !(0..=2).contains(&axis) {
                    return Err("Eixo inválido".into());
                }
                b.commit_transform_text(kind, axis as usize, string(request, "text")?)
                    .map_err(|e| format!("{e:?}"))?;
                return Ok(true);
            }
            "paint_stroke" => {
                let x = number(request, "x")?;
                let y = number(request, "y")?;
                let phase = integer(request, "phase")? as i32;
                // A textura aparece no canvas 2D com uma escala própria: sem
                // ela o tamanho do pincel (px de tela) não vira texels e o
                // carimbo não bate com o anel do cursor.
                if let Some(raster) = request["raster_px"].as_f64()
                    && raster.is_finite()
                    && raster > 1.
                {
                    b.paint_2d_raster_px = (raster as f32).min(32768.);
                }
                return Ok(with_tablet(b, tablet, |b| b.paint_2d_stroke(x, y, phase)));
            }
            "pointer" => {
                let phase = integer(request, "phase")? as i32;
                if !(0..=3).contains(&phase) {
                    return Err("Fase inválida".into());
                }
                let x = number(request, "x")?;
                let y = number(request, "y")?;
                b.pointer_position = [x, y];
                let shift = request["shift"].as_bool().unwrap_or(false);
                let ctrl = request["ctrl"].as_bool().unwrap_or(false);
                let alt = request["alt"].as_bool().unwrap_or(false);
                // Gizmo do painel PAINT: com um modo explícito e um decalque
                // ativo, o arrasto transforma o decalque em vez de pintar (ou
                // de seguir a gramática das alças, que é o modo Automático).
                if decal_gizmo != DecalGizmoMode::Auto
                    && b.state.workspace == Workspace::Paint
                    && b.state.session.tools.active_tool != "select"
                    && b.active_layer_is_decal()
                {
                    let (scale, rotate) = decal_gizmo.modifiers();
                    return Ok(with_tablet(b, tablet, |b| match phase {
                        0 => b.decal_drag_begin(x, y, scale, rotate),
                        1 => b.decal_drag_to(x, y, scale, rotate),
                        2 => b.decal_drag_end(),
                        _ => b.cancel_decal_drag(),
                    }));
                }
                if b.state.workspace == Workspace::Paint
                    && b.grammar_tool().is_none()
                    && b.state.session.tools.active_tool != "select"
                {
                    return Ok(with_tablet(b, tablet, |b| match phase {
                        0 => {
                            if b.is_shape_tool() {
                                b.begin_paint_shape_at(x, y)
                            } else {
                                b.begin_paint_stroke_with_modifiers(x, y, shift, ctrl)
                            }
                        }
                        1 => b.paint_stroke_to_with_modifiers(x, y, shift, ctrl),
                        2 => {
                            if b.is_shape_tool() {
                                b.end_paint_shape_at(x, y)
                            } else {
                                b.end_paint_stroke_at(x, y)
                            }
                        }
                        _ => b.cancel_paint_stroke() | b.cancel_paint_shape(),
                    }));
                }
                return Ok(b.tool_pointer_ex(phase, x, y, shift, ctrl, alt));
            }
            "hover" => {
                return Ok(b.hover_component(number(request, "x")?, number(request, "y")?));
            }
            "shortcut" => {
                return Ok(b.route_shortcut(
                    string(request, "text")?,
                    boolean(request, "ctrl")?,
                    boolean(request, "shift")?,
                    boolean(request, "alt")?,
                ));
            }
            "tick" => {
                let dt = number(request, "dt")?.clamp(0., 0.1);
                let previous_status = b.state.ui.status.clone();
                let mut changed = if b.state.workspace == Workspace::Animate {
                    b.animate_advance(dt)
                } else {
                    false
                };
                changed |= b.decal_playback_tick(dt);
                changed |= with_tablet(b, tablet, |b| b.airbrush_tick());
                if owns_session {
                    changed |= b.autosave_tick();
                }
                return Ok(changed || b.state.ui.status != previous_status);
            }
            other => return dispatch_controller(b, other, request),
        };
        b.apply(intent);
        if action == "workspace" {
            match request["id"].as_str() {
                Some("DRAW") => b.apply(UiIntent::SetModelingMode(ModelingMode::Draw)),
                Some("POLY") => b.apply(UiIntent::SetModelingMode(ModelingMode::Poly)),
                _ => {}
            }
        }
        Ok(true)
    }
}
impl Deref for QtEditor {
    type Target = Application;
    fn deref(&self) -> &Application {
        self.controller.application()
    }
}
impl DerefMut for QtEditor {
    fn deref_mut(&mut self) -> &mut Application {
        self.controller.application_mut()
    }
}
fn string<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    v[key]
        .as_str()
        .filter(|s| s.len() <= 1024 * 1024)
        .ok_or_else(|| format!("Texto inválido: {key}"))
}
fn number(v: &Value, key: &str) -> Result<f32, String> {
    v[key]
        .as_f64()
        .map(|v| v as f32)
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("Número inválido: {key}"))
}
fn integer(v: &Value, key: &str) -> Result<i64, String> {
    v[key]
        .as_i64()
        .or_else(|| {
            v[key]
                .as_f64()
                .filter(|n| n.is_finite() && n.fract() == 0. && n.abs() <= 1_000_000.)
                .map(|n| n as i64)
        })
        .or_else(|| v[key].as_str().and_then(|s| s.parse().ok()))
        .filter(|v| (-1_000_000..=1_000_000).contains(v))
        .ok_or_else(|| format!("Inteiro inválido: {key}"))
}
fn boolean(v: &Value, key: &str) -> Result<bool, String> {
    v[key]
        .as_bool()
        .ok_or_else(|| format!("Booleano inválido: {key}"))
}

/// Rust allocates; Qt frees using the paired function. Snapshot size is queried
/// once and never truncated into a fixed native buffer.
///
/// # Safety
/// A non-null input must point to a valid, null-terminated C string for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_qt_editor_request(input: *const c_char) -> *mut c_char {
    crate::render::opengl_viewport::VIEWPORT_RENDERER.with(|renderer| {

    let response = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut editor = renderer.try_borrow_mut()
            .map_err(|_| "Editor indisponível".to_string())?;
        if !input.is_null() {
            let input = unsafe { CStr::from_ptr(input) }
                .to_str()
                .map_err(|e| e.to_string())?;
            if input.len() > 1024 * 1024 {
                return Err("Pedido grande demais".into());
            }
            let request: Value = serde_json::from_str(input).map_err(|e| e.to_string())?;
            if request["action"] == "translations" {
                return Ok(
                    serde_json::json!({"ok":true,"translations":crate::localization::pack(&editor.app.state.ui.i18n)}),
                );
            }
            if request["action"] != "snapshot" {
                match editor.app.request(&request) {
                    Ok(changed) => {
                        if request["action"] == "tick" && !changed {
                            return Ok(json!({"ok":true,"changed":false}));
                        }
                        let mut vm = editor.app.snapshot();
                        vm["ok"] = json!(true);
                        vm["changed"] = json!(changed);
                        return Ok(vm);
                    }
                    Err(error) => {
                        editor.app.state.set_status(error.clone());
                        let mut vm = editor.app.snapshot();
                        vm["ok"] = json!(false);
                        vm["error"] = json!(error);
                        return Ok(vm);
                    }
                }
            }
        }
        let mut vm = editor.app.snapshot();
        vm["ok"] = json!(true);
        Ok::<_, String>(vm)
    }));
    let value = match response {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => json!({"ok":false,"error":e}),
        Err(_) => json!({"ok":false,"error":"Falha interna do editor"}),
    };
    CString::new(value.to_string()).map_or(std::ptr::null_mut(), CString::into_raw)

    }).unwrap_or(std::ptr::null_mut())
}
#[unsafe(no_mangle)]
/// # Safety
/// A non-null value must be an allocation returned by petunia_qt_editor_request,
/// and must not have been freed previously.
pub unsafe extern "C" fn petunia_qt_editor_free(value: *mut c_char) {
    if !value.is_null() {
        drop(unsafe { CString::from_raw(value) })
    }
}

fn dispatch_controller(
    b: &mut SlintUiBridge<QtViewportCapture>,
    action: &str,
    v: &Value,
) -> Result<bool, String> {
    Ok(match action {
        "activate_keymap_profile" => b.activate_keymap_profile(string(v, "id")?),
        "add_decal_layer" => b.add_decal_layer(),
        "add_modifier" => b.add_modifier(string(v, "kind")?),
        "add_paint_effect_layer" => b.add_paint_effect_layer(string(v, "kind")?),
        "add_paint_group" => b.add_paint_group(),
        "add_paint_layer" => b.add_paint_layer(),
        "add_profile_circle" => b.add_profile_circle(
            number(v, "radius")?,
            integer(v, "segments")?.max(0) as usize,
        ),
        "add_profile_rectangle" => {
            b.add_profile_rectangle(number(v, "width")?, number(v, "height")?)
        }
        "adjust_brush_hardness" => {
            b.adjust_brush_hardness(number(v, "delta")?);
            true
        }
        "adjust_last_operation" => b.adjust_last_operation(number(v, "value")?),
        "adjust_loop_cut_count_from_input" => {
            b.adjust_loop_cut_count_from_input(integer(v, "cuts")?.max(0) as usize)
        }
        "adjust_loop_cut_hover_count" => b.adjust_loop_cut_hover_count(integer(v, "delta")? as i32),
        "adjust_proportional_radius" => b.adjust_proportional_radius(number(v, "delta")?),
        "airbrush_tick" => b.airbrush_tick(),
        "align_reference_view" => b.align_reference_view(string(v, "axis_str")?),
        "apply_brush_preset" => b.apply_brush_preset(integer(v, "index")?.max(0) as usize),
        "apply_modifier" => b.apply_modifier(string(v, "id")?),
        "assign_material_slot" => b.assign_material_slot(integer(v, "slot")?.max(0) as usize),
        "autosave_tick" => b.autosave_tick(),
        "bake_reference" => b.bake_reference(),
        "begin_gizmo_drag" => b.begin_gizmo_drag(number(v, "x")?, number(v, "y")?),
        "begin_keymap_capture" => b.begin_keymap_capture(string(v, "target")?),
        "begin_loop_cut" => b.begin_loop_cut(),
        "begin_paint_shape_at" => b.begin_paint_shape_at(number(v, "x")?, number(v, "y")?),
        "begin_paint_stroke_at" => b.begin_paint_stroke_at(number(v, "x")?, number(v, "y")?),
        "begin_paint_stroke_with_modifiers" => b.begin_paint_stroke_with_modifiers(
            number(v, "x")?,
            number(v, "y")?,
            boolean(v, "is_shift")?,
            boolean(v, "is_ctrl")?,
        ),
        "begin_rename" => b.begin_rename(),
        "begin_slice" => b.begin_slice(number(v, "x")?, number(v, "y")?),
        "boolean_op" => b.boolean_op(string(v, "id")?),
        "cancel_decal_drag" => b.cancel_decal_drag(),
        "cancel_keymap_capture" => b.cancel_keymap_capture(),
        "cancel_knife" => b.cancel_knife(),
        "cancel_loop_cut" => b.cancel_loop_cut(),
        "cancel_paint_shape" => b.cancel_paint_shape(),
        "cancel_paint_stroke" => b.cancel_paint_stroke(),
        "cancel_primitive" => b.cancel_primitive(),
        "cancel_profile_volume" => b.cancel_profile_volume(),
        "cancel_rename" => b.cancel_rename(),
        "cancel_slice" => b.cancel_slice(),
        "cancel_tool_modal" => b.cancel_tool_modal(),
        "cancel_transform" => b.cancel_transform(),
        "cancel_viewport_transform" => b.cancel_viewport_transform(),
        "capture_keymap_key" => b.capture_keymap_key(
            string(v, "text")?,
            boolean(v, "ctrl")?,
            boolean(v, "shift")?,
            boolean(v, "alt")?,
        ),
        "clear_albedo_texture" => b.clear_albedo_texture(),
        "clear_all_references" => b.clear_all_references(),
        "clear_all_uv_pins" => b.clear_all_uv_pins(),
        "clear_all_uv_seams" => b.clear_all_uv_seams(),
        "clear_boolean_operand" => b.clear_boolean_operand(),
        "clear_hover" => b.clear_hover(),
        "clear_keymap_binding" => b.clear_keymap_binding(string(v, "target")?),
        "close_context_menu" => b.close_context_menu(),
        "close_menu" => b.close_menu(),
        "close_profile" => b.close_profile(),
        "close_reference_manager" => b.close_reference_manager(),
        "collapse_inspector" => {
            b.collapse_inspector();
            true
        }
        "commit_knife" => b.commit_knife(),
        "commit_last_operation_text" => b.commit_last_operation_text(string(v, "text")?),
        "commit_loop_cut" => b.commit_loop_cut(),
        "commit_profile_volume" => b.commit_profile_volume(),
        "commit_rename" => b.commit_rename(string(v, "name")?),
        "commit_slice" => b.commit_slice(),
        "commit_tool_modal" => b.commit_tool_modal(),
        "commit_transform" => b.commit_transform(),
        "confirm_primitive" => b.confirm_primitive(),
        "context_menu_action" => b.context_menu_action(string(v, "target")?),
        "copy_selection" => b.copy_selection(),
        "create_albedo_texture" => b.create_albedo_texture(),
        "create_keymap_profile" => b.create_keymap_profile(string(v, "name")?),
        "create_material" => b.create_material(),
        "decal_drag_begin" => b.decal_drag_begin(
            number(v, "x")?,
            number(v, "y")?,
            boolean(v, "is_shift")?,
            boolean(v, "is_ctrl")?,
        ),
        "decal_drag_end" => b.decal_drag_end(),
        "decal_drag_to" => b.decal_drag_to(
            number(v, "x")?,
            number(v, "y")?,
            boolean(v, "_is_shift")?,
            boolean(v, "_is_ctrl")?,
        ),
        "delete_brush_preset" => b.delete_brush_preset(integer(v, "index")?.max(0) as usize),
        "delete_keymap_profile" => b.delete_keymap_profile(string(v, "id")?),
        "delete_or_dissolve_selection" => b.delete_or_dissolve_selection(),
        "delete_prefab" => b.delete_prefab(string(v, "id")?),
        "discard_pending_recovery" => b.discard_pending_recovery(),
        "duplicate_material" => b.duplicate_material(integer(v, "slot")?.max(0) as usize),
        "duplicate_selection" => b.duplicate_selection(),
        "end_gizmo_drag" => b.end_gizmo_drag(),
        "end_paint_shape_at" => b.end_paint_shape_at(number(v, "x")?, number(v, "y")?),
        "end_paint_stroke_at" => b.end_paint_stroke_at(number(v, "x")?, number(v, "y")?),
        "end_viewport_transform" => b.end_viewport_transform(),
        "ensure_editable_keymap" => b.ensure_editable_keymap(),
        "enter_profile_volume" => b.enter_profile_volume(string(v, "mode_str")?),
        "execute_quick_action" => b.execute_quick_action(string(v, "id")?),
        "frame_camera_on_uv" => b.frame_camera_on_uv(number(v, "u")?, number(v, "v")?),
        "frame_cursor" => b.frame_cursor(),
        "freeze_active_primitive" => b.freeze_active_primitive(),
        "generate_profile_extrude" => b.generate_profile_extrude(),
        "generate_profile_revolve" => b.generate_profile_revolve(),
        "generate_profile_sweep" => b.generate_profile_sweep(),
        "handle_click_away" => b.handle_click_away(),
        "handle_escape" => b.handle_escape(),
        "hover_component" => {
            b.hover_component(number(v, "normalized_x")?, number(v, "normalized_y")?)
        }
        "hover_gizmo" => b.hover_gizmo(number(v, "x")?, number(v, "y")?),
        "import_decal_image" => b.import_decal_image(Path::new(string(v, "path")?)),
        "join_operand" => b.join_operand(),
        "keep_saved_project" => b.keep_saved_project(),
        "knife_click" => b.knife_click(number(v, "normalized_x")?, number(v, "normalized_y")?),
        "menu_item_invoked" => b.menu_item_invoked(string(v, "id")?),
        "merge_down_paint_layer" => b.merge_down_paint_layer(string(v, "id")?),
        "move_modifier" => b.move_modifier(string(v, "id")?, integer(v, "direction")? as i32),
        "move_paint_layer" => b.move_paint_layer(string(v, "id")?, integer(v, "delta")? as i32),
        "nudge_selection" => {
            b.nudge_selection(number(v, "dx")?, number(v, "dy")?, number(v, "dz")?)
        }
        "open_context_menu" => {
            b.open_context_menu(string(v, "id")?, number(v, "x")?, number(v, "y")?)
        }
        "open_menu" => b.open_menu(string(v, "id")?),
        "open_reference_manager" => b.open_reference_manager(),
        "open_viewport_context_menu" => {
            b.open_viewport_context_menu(number(v, "x")?, number(v, "y")?)
        }
        "orbit_viewport" => b.orbit_viewport(number(v, "dx")?, number(v, "dy")?),
        "paint_2d_stroke" => b.paint_2d_stroke(
            number(v, "norm_x")?,
            number(v, "norm_y")?,
            integer(v, "phase")? as i32,
        ),
        "paint_stroke_to" => b.paint_stroke_to(number(v, "x")?, number(v, "y")?),
        "paint_stroke_to_with_modifiers" => b.paint_stroke_to_with_modifiers(
            number(v, "x")?,
            number(v, "y")?,
            boolean(v, "is_shift")?,
            boolean(v, "is_ctrl")?,
        ),
        "paste_clipboard" => b.paste_clipboard(),
        "place_asset" => b.place_asset(string(v, "id")?),
        "place_cursor_3d" => b.place_cursor_3d(number(v, "norm_x")?, number(v, "norm_y")?),
        "place_loop_cut_hover" => b.place_loop_cut_hover(),
        "place_prefab" => b.place_prefab(string(v, "id")?),
        "profile_clear_curves" => b.profile_clear_curves(),
        "profile_smooth_curves" => b.profile_smooth_curves(),
        "profile_update_drag_handle" => b.profile_update_drag_handle(
            number(v, "norm_x")?,
            number(v, "norm_y")?,
            boolean(v, "break_tangent")?,
        ),
        "project_from_reference" => b.project_from_reference(),
        "recover_pending" => b.recover_pending(),
        "remove_material" => b.remove_material(integer(v, "slot")? as i32),
        "remove_modifier" => b.remove_modifier(string(v, "id")?),
        "remove_paint_layer" => b.remove_paint_layer(string(v, "id")?),
        "remove_reference_slot" => b.remove_reference_slot(string(v, "axis_str")?),
        "reset_cursor_3d" => b.reset_cursor_3d(),
        "reset_keymap_binding" => b.reset_keymap_binding(string(v, "target")?),
        "reset_quick_actions" => b.reset_quick_actions(),
        "reset_transform_fields" => {
            b.reset_transform_fields();
            true
        }
        "route_shortcut" => b.route_shortcut(
            string(v, "text")?,
            boolean(v, "ctrl")?,
            boolean(v, "shift")?,
            boolean(v, "alt")?,
        ),
        "save_brush_preset" => b.save_brush_preset(string(v, "name")?),
        "scroll_loop_cut_count" => b.scroll_loop_cut_count(number(v, "delta_y")?),
        "scrub_loop_cut" => b.scrub_loop_cut(number(v, "delta_x")?, boolean(v, "fine")?),
        "scrub_loop_cut_2d" => b.scrub_loop_cut_2d(
            number(v, "delta_x")?,
            number(v, "delta_y")?,
            boolean(v, "fine")?,
        ),
        "scrub_tool_modal" => b.scrub_tool_modal(number(v, "delta_y")?, boolean(v, "fine")?),
        "select_material_slot" => b.select_material_slot(integer(v, "slot")? as i32),
        "select_viewport" => {
            b.select_viewport(
                number(v, "normalized_x")?,
                number(v, "normalized_y")?,
                boolean(v, "extend")?,
            );
            true
        }
        "select_viewport_ext" => {
            b.select_viewport_ext(
                number(v, "normalized_x")?,
                number(v, "normalized_y")?,
                boolean(v, "extend")?,
                boolean(v, "loop_select")?,
            );
            true
        }
        "set_accent_color_hex" => b.set_accent_color_hex(string(v, "value")?),
        "set_active_material_alpha_mode" => {
            b.set_active_material_alpha_mode(integer(v, "mode")? as i32)
        }
        "set_active_material_base_color" => b.set_active_material_base_color(
            number(v, "red")?,
            number(v, "green")?,
            number(v, "blue")?,
        ),
        "set_active_material_profile" => {
            b.set_active_material_profile(integer(v, "profile")? as i32)
        }
        "set_active_material_scalar" => b.set_active_material_scalar(
            &string(v, "field")?.replace('_', "-"),
            number(v, "value")?,
        ),
        "set_add_menu_open" => {
            b.set_add_menu_open(boolean(v, "open")?);
            true
        }
        "set_asset_library_height" => b.set_asset_library_height(number(v, "height")?),
        "set_asset_only_favorites" => b.set_asset_only_favorites(boolean(v, "only")?),
        "set_asset_query" => b.set_asset_query(string(v, "query")?),
        "set_asset_sort_by_name" => b.set_asset_sort_by_name(boolean(v, "sort_by_name")?),
        "set_asset_thumbnail_size" => b.set_asset_thumbnail_size(number(v, "size")?),
        "set_boolean_cleanup" => b.set_boolean_cleanup(boolean(v, "cleanup")?),
        "set_boolean_keep_parts" => b.set_boolean_keep_parts(boolean(v, "keep")?),
        "set_boolean_operand" => b.set_boolean_operand(string(v, "id")?),
        "set_brush_blend" => b.set_brush_blend(string(v, "blend")?),
        "set_brush_hardness" => b.set_brush_hardness(number(v, "val")?),
        "set_brush_lock" => b.set_brush_lock(string(v, "lock")?),
        "set_brush_param" => b.set_brush_param(string(v, "name")?, number(v, "value")?),
        "set_brush_projection" => b.set_brush_projection(string(v, "projection")?),
        "set_brush_tip" => b.set_brush_tip(string(v, "tip")?),
        "set_click_move_click" => b.set_click_move_click(boolean(v, "enabled")?),
        "set_colorblind_axes" => b.set_colorblind_axes(boolean(v, "enabled")?),
        "set_cursor_3d_coord" => {
            b.set_cursor_3d_coord(integer(v, "index")?.max(0) as usize, number(v, "val")?)
        }
        "set_double_tap_interval_ms" => {
            b.set_double_tap_interval_ms(integer(v, "interval_ms")?.max(0) as u64)
        }
        "set_drag_threshold_px" => b.set_drag_threshold_px(number(v, "pixels")?),
        "set_fill_scope" => b.set_fill_scope(string(v, "scope")?),
        "set_geometry_to_origin" => b.set_geometry_to_origin(),
        "set_icon_theme" => b.set_icon_theme(string(v, "theme")?),
        "set_inspector_structure_ratio" => b.set_inspector_structure_ratio(number(v, "ratio")?),
        "set_inspector_width" => b.set_inspector_width(number(v, "width")?),
        "set_invert_vertical_drag" => b.set_invert_vertical_drag(boolean(v, "invert")?),
        "set_zoom_sensitivity" => true,
        "set_keymap_profile" => b.set_keymap_profile(string(v, "profile_id")?),
        "set_language" => b.set_language(string(v, "lang")?),
        "set_loop_cut_balanced" => b.set_loop_cut_balanced(boolean(v, "balanced")?),
        "set_loop_cut_count" => b.set_loop_cut_count(integer(v, "cuts")?.max(0) as usize),
        "set_loop_cut_slide" => b.set_loop_cut_slide(number(v, "slide")?),
        "set_modifier_axis" => b.set_modifier_axis(string(v, "id")?, integer(v, "axis")? as i32),
        "set_modifier_direction" => {
            b.set_modifier_direction(string(v, "id")?, boolean(v, "direction_value")?)
        }
        "set_modifier_enabled" => b.set_modifier_enabled(string(v, "id")?, boolean(v, "enabled")?),
        "set_multiselection_measure_tag" => {
            b.set_multiselection_measure_tag(boolean(v, "enabled")?)
        }
        "set_origin_bottom" => b.set_origin_bottom(),
        "set_origin_cursor" => b.set_origin_cursor(),
        "set_origin_geometry" => b.set_origin_geometry(),
        "set_origin_selection" => b.set_origin_selection(),
        "set_paint_effect_param" => {
            b.set_paint_effect_param(string(v, "key")?, number(v, "value")?)
        }
        "set_paint_layer_active" => b.set_paint_layer_active(string(v, "id")?),
        "set_paint_layer_blend" => {
            b.set_paint_layer_blend(string(v, "id")?, integer(v, "blend")? as i32)
        }
        "set_paint_layer_opacity" => {
            b.set_paint_layer_opacity(string(v, "id")?, number(v, "opacity")?)
        }
        "set_paint_symmetry_x" => {
            b.set_paint_symmetry_x(boolean(v, "val")?);
            true
        }
        "set_paint_symmetry_y" => {
            b.set_paint_symmetry_y(boolean(v, "val")?);
            true
        }
        "set_paint_symmetry_z" => {
            b.set_paint_symmetry_z(boolean(v, "val")?);
            true
        }
        "set_parts_query" => b.set_parts_query(string(v, "query")?),
        "set_parts_row_height" => b.set_parts_row_height(number(v, "size")?),
        "set_parts_selected_only" => b.set_parts_selected_only(boolean(v, "enabled")?),
        "set_parts_sort_by_name" => b.set_parts_sort_by_name(boolean(v, "enabled")?),
        "set_pivot_menu_open" => b.set_pivot_menu_open(boolean(v, "open")?),
        "set_pivot_point" => b.set_pivot_point(string(v, "id")?),
        "set_prefab_favorite" => b.set_prefab_favorite(string(v, "id")?, boolean(v, "favorite")?),
        "set_profile_depth" => b.set_profile_depth(number(v, "depth")?),
        "set_profile_dimensions" => b.set_profile_dimensions(
            number(v, "width")?,
            number(v, "height")?,
            integer(v, "segments")?.max(0) as usize,
        ),
        "set_profile_revolve_angle" => b.set_profile_revolve_angle(number(v, "angle_deg")?),
        "set_profile_smoothness" => b.set_profile_smoothness(number(v, "smoothness")?),
        "set_profile_wall_thickness" => b.set_profile_wall_thickness(number(v, "thickness")?),
        "set_proportional_falloff" => b.set_proportional_falloff(string(v, "falloff_str")?),
        "set_proportional_radius" => b.set_proportional_radius(number(v, "radius")?),
        "set_reduced_motion" => b.set_reduced_motion(boolean(v, "enabled")?),
        "set_reference_param" => b.set_reference_param(
            string(v, "axis_str")?,
            string(v, "param")?,
            number(v, "val")?,
        ),
        "set_selection_color_hex" => b.set_selection_color_hex(string(v, "value")?),
        "set_selection_thickness" => b.set_selection_thickness(number(v, "thickness")?),
        "set_shading_mode" => b.set_shading_mode(string(v, "id")?),
        "set_show_home_on_start" => b.set_show_home_on_start(boolean(v, "enabled")?),
        "set_show_tool_labels" => b.set_show_tool_labels(boolean(v, "enabled")?),
        "set_slice_trim" => {
            b.set_slice_trim(boolean(v, "trim")?);
            true
        }
        "set_snap_radius_px" => b.set_snap_radius_px(number(v, "pixels")?),
        "set_snap_target" => b.set_snap_target(string(v, "target_str")?),
        "set_split_preset" => b.set_split_preset(string(v, "id")?),
        "set_studio_light_follows_camera" => {
            b.set_studio_light_follows_camera(boolean(v, "follows")?)
        }
        "set_tool_activation" => b.set_tool_activation(string(v, "id")?),
        "set_tool_modal_value" => b.set_tool_modal_value(number(v, "value")?),
        "set_ui_scale" => b.set_ui_scale(number(v, "scale")?),
        "set_viewport_ambient_occlusion" => {
            b.set_viewport_ambient_occlusion(boolean(v, "enabled")?)
        }
        "set_viewport_matcap" => b.set_viewport_matcap(boolean(v, "enabled")?),
        "set_workplane_prefer_ground" => b.set_workplane_prefer_ground(boolean(v, "enabled")?),
        "set_xray_opacity" => b.set_xray_opacity(number(v, "opacity")?),
        "snap_view_to_axis" => b.snap_view_to_axis(string(v, "axis")?),
        "switch_extrude_mode" => b.switch_extrude_mode(),
        "toggle_bevel_affect_vertices" => b.toggle_bevel_affect_vertices(),
        "toggle_bevel_clamp_overlap" => b.toggle_bevel_clamp_overlap(),
        "toggle_edit_pivot" => b.toggle_edit_pivot(),
        "toggle_face_orientation" => b.toggle_face_orientation(),
        "toggle_loop_cut_balanced" => b.toggle_loop_cut_balanced(),
        "toggle_menu" => b.toggle_menu(string(v, "id")?),
        "toggle_micro_inspector" => b.toggle_micro_inspector(),
        "toggle_paint_layer_lock" => b.toggle_paint_layer_lock(string(v, "id")?),
        "toggle_paint_layer_visibility" => b.toggle_paint_layer_visibility(string(v, "id")?),
        "toggle_paint_pip" => b.toggle_paint_pip(),
        "toggle_paint_symmetry_x" => b.toggle_paint_symmetry_x(),
        "toggle_paint_symmetry_y" => b.toggle_paint_symmetry_y(),
        "toggle_paint_symmetry_z" => b.toggle_paint_symmetry_z(),
        "toggle_proportional_editing" => b.toggle_proportional_editing(),
        "toggle_quick_action" => b.toggle_quick_action(string(v, "id")?),
        "toggle_reference_lock" => b.toggle_reference_lock(string(v, "axis_str")?),
        "toggle_reference_manager" => b.toggle_reference_manager(),
        "toggle_reference_visible" => b.toggle_reference_visible(string(v, "axis_str")?),
        "toggle_selected_uv_pins" => b.toggle_selected_uv_pins(),
        "toggle_selected_uv_seams" => b.toggle_selected_uv_seams(),
        "toggle_snap_enabled" => b.toggle_snap_enabled(),
        "toggle_split_view" => b.toggle_split_view(),
        "toggle_uv_checker" => b.toggle_uv_checker(),
        "toggle_view_opposite" => b.toggle_view_opposite(),
        "update_loop_cut_hover" => b.update_loop_cut_hover(number(v, "x")?, number(v, "y")?),
        "update_prefab" => b.update_prefab(string(v, "id")?),
        "update_primitive_param_bool" => {
            b.update_primitive_param_bool(string(v, "param")?, boolean(v, "value")?)
        }
        "update_primitive_param_float" => {
            b.update_primitive_param_float(string(v, "param")?, number(v, "value")?)
        }
        "update_slice" => b.update_slice(number(v, "x")?, number(v, "y")?),
        "update_slice_modified" => {
            b.update_slice_modified(number(v, "x")?, number(v, "y")?, boolean(v, "snap")?)
        }
        "update_viewport_slice" => b.update_viewport_slice(number(v, "x")?, number(v, "y")?),
        "update_viewport_slice_modified" => {
            b.update_viewport_slice_modified(number(v, "x")?, number(v, "y")?, boolean(v, "snap")?)
        }
        "update_viewport_transform" => {
            b.update_viewport_transform(number(v, "x")?, number(v, "y")?)
        }
        "update_viewport_transform_modified" => b.update_viewport_transform_modified(
            number(v, "x")?,
            number(v, "y")?,
            boolean(v, "fine")?,
            boolean(v, "snap")?,
        ),
        "uv_editor_click" => {
            b.uv_editor_click(number(v, "u")?, number(v, "v")?, boolean(v, "extend")?)
        }
        "uv_equalize_texel_density" => b.uv_equalize_texel_density(),
        "uv_move_selected" => b.uv_move_selected(number(v, "du")?, number(v, "dv")?),
        "uv_rotate_selected" => b.uv_rotate_selected(number(v, "degrees")?),
        "uv_scale_selected" => b.uv_scale_selected(number(v, "factor")?),
        "viewport_context_triage" => b.viewport_context_triage(number(v, "x")?, number(v, "y")?),
        "viewport_ctrl_scroll" => b.viewport_ctrl_scroll(number(v, "delta")?),
        _ => return Err(format!("Ação desconhecida: {action}")),
    })
}

/// Collect the shared renderer contract without creating a Slint window or
/// rasterizing a second viewport. Qt consumes these inputs in its GL context.
#[derive(Default)]
pub struct QtViewportCapture {
    pub render_state: petunia_editor::ViewportRenderState,
    pub pose: Option<std::sync::Arc<petunia_project::PoseOverride>>,
    pub texture_updates: Vec<petunia_core::TextureUpdate>,
    pub outlined: Vec<uuid::Uuid>,
    pub active: Option<uuid::Uuid>,
    pub rigid: Vec<(uuid::Uuid, glam::Mat4)>,
}
impl petunia_editor::PetuniaViewport for QtViewportCapture {
    fn resize(&mut self, _: u32, _: u32) {}
    fn update(&mut self, _: f32) {}
    fn set_workspace(&mut self, _: Workspace) {}
    fn set_selection_domain(&mut self, _: SelectionDomain) {}
    fn draws_component_guides(&self) -> bool {
        true
    }
    fn uses_physical_pixels(&self) -> bool {
        true
    }
    fn queue_texture_updates(&mut self, updates: Vec<petunia_core::TextureUpdate>) {
        self.texture_updates.extend(updates)
    }
    fn set_pose_override(&mut self, pose: Option<std::sync::Arc<petunia_project::PoseOverride>>) {
        self.pose = pose
    }
    fn set_outlined_objects(&mut self, selected: &[uuid::Uuid], active: Option<uuid::Uuid>) {
        self.outlined = selected.to_vec();
        self.active = active
    }
    fn set_rigid_previews(&mut self, previews: &[(uuid::Uuid, glam::Mat4)]) {
        self.rigid = previews.to_vec()
    }
    fn render_frame(
        &mut self,
        _: &petunia_project::Project,
        _: &[petunia_core::ReferenceImage],
        _: &petunia_core::Camera,
        state: petunia_editor::ViewportRenderState,
    ) {
        self.render_state = state;
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// C string inputs `axis` and `name` must be non-null and null-terminated.
/// `pixels` must point to a buffer of length at least `len` bytes.
pub unsafe extern "C" fn petunia_qt_reference_load(
    axis: *const c_char,
    name: *const c_char,
    width: u32,
    height: u32,
    pixels: *const u8,
    len: usize,
) -> bool {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            if axis.is_null()
                || name.is_null()
                || pixels.is_null()
                || width == 0
                || height == 0
                || width > 4096
                || height > 4096
            {
                return false;
            }
            let Some(expected) = (width as usize)
                .checked_mul(height as usize)
                .and_then(|n| n.checked_mul(4))
            else {
                return false;
            };
            if expected != len {
                return false;
            }
            let Ok(axis) = unsafe { CStr::from_ptr(axis) }.to_str() else {
                return false;
            };
            let Ok(name) = unsafe { CStr::from_ptr(name) }.to_str() else {
                return false;
            };
            let Ok(mut host) = renderer.try_borrow_mut() else {
                return false;
            };
            host.app.controller.load_reference_slot(
                axis,
                name.into(),
                width,
                height,
                unsafe { std::slice::from_raw_parts(pixels, len) }.to_vec(),
            )
        })
        .unwrap_or(false)
}

#[derive(Clone, Copy)]
struct TabletInput {
    pressure: f32,
    tilt_x: f32,
    tilt_y: f32,
    eraser: bool,
}
fn with_tablet(
    b: &mut SlintUiBridge<QtViewportCapture>,
    input: Option<TabletInput>,
    paint: impl FnOnce(&mut SlintUiBridge<QtViewportCapture>) -> bool,
) -> bool {
    let Some(input) = input else { return paint(b) };
    let previous = (
        b.state.session.tools.paint_strength,
        b.state.session.tools.brush_flow,
        b.state.session.tools.paint_brush_kind,
        b.state.session.tools.brush_style,
        b.state.session.tools.active_tool.clone(),
    );
    b.state.session.tools.paint_strength *= input.pressure;
    b.state.session.tools.brush_flow *= input.pressure;
    let tilt = input.tilt_x.hypot(input.tilt_y).min(85.);
    if tilt > 1. {
        b.state.session.tools.brush_style.roundness *= tilt.to_radians().cos().clamp(0.1, 1.);
        b.state.session.tools.brush_style.angle_deg = input.tilt_y.atan2(input.tilt_x).to_degrees();
    }
    if input.eraser {
        b.state.session.tools.paint_brush_kind =
            petunia_core::kind_from_brush_type(petunia_core::BrushType::Eraser);
        b.state.session.tools.active_tool = "eraser".into();
    }
    let changed = paint(b);
    b.state.session.tools.paint_strength = previous.0;
    b.state.session.tools.brush_flow = previous.1;
    b.state.session.tools.paint_brush_kind = previous.2;
    b.state.session.tools.brush_style = previous.3;
    b.state.session.tools.active_tool = previous.4;
    changed
}

struct SessionLease {
    path: std::path::PathBuf,
    bytes: Vec<u8>,
}
impl SessionLease {
    fn active_owner() -> bool {
        let path = petunia_core::AutosaveService::session_lock_path(None);
        let owner = std::fs::read(path).ok().and_then(|bytes| {
            postcard::from_bytes::<petunia_project::SessionLockInfo>(&bytes).ok()
        });
        owner.is_some_and(|o| Self::process_running(o.pid))
    }
    fn process_running(pid: u32) -> bool {
        #[cfg(target_os = "linux")]
        {
            std::path::Path::new("/proc").join(pid.to_string()).exists()
        }
        #[cfg(target_os = "windows")]
        {
            #[link(name = "kernel32")]
            unsafe extern "system" {
                fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut std::ffi::c_void;
                fn GetExitCodeProcess(handle: *mut std::ffi::c_void, code: *mut u32) -> i32;
                fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
                fn GetLastError() -> u32;
            }
            unsafe {
                let handle = OpenProcess(0x1000, 0, pid);
                if handle.is_null() {
                    return GetLastError() != 87;
                }
                let mut code = 259;
                let queried = GetExitCodeProcess(handle, &mut code);
                CloseHandle(handle);
                queried == 0 || code == 259
            }
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            let _ = pid;
            true
        }
    }
    fn claim() -> Result<Self, String> {
        use std::io::Write;
        let path = petunia_core::AutosaveService::session_lock_path(None);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        if let Ok(bytes) = std::fs::read(&path) {
            let owner = postcard::from_bytes::<petunia_project::SessionLockInfo>(&bytes).ok();
            let active = owner.is_some_and(|o| Self::process_running(o.pid));
            if active {
                return Err("Outra sessão utiliza os snapshots sem nome; salve seu projeto em um arquivo próprio".into());
            }
            if std::fs::read(&path).ok().as_deref() == Some(bytes.as_slice()) {
                std::fs::remove_file(&path).map_err(|e| e.to_string())?;
            }
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();
        let bytes = postcard::to_allocvec(&petunia_project::SessionLockInfo {
            pid: std::process::id(),
            start_time: now,
            project_name: "Petunia3D Qt".into(),
            project_path: None,
        })
        .map_err(|e| e.to_string())?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        Ok(Self { path, bytes })
    }
}
impl Drop for SessionLease {
    fn drop(&mut self) {
        if std::fs::read(&self.path).ok().as_deref() == Some(self.bytes.as_slice()) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// # Safety
/// C strings are valid and output has at least capacity writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_qt_editor_image(
    kind: *const c_char,
    id: *const c_char,
    output: *mut u8,
    capacity: usize,
    width: *mut u32,
    height: *mut u32,
) -> usize {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            if kind.is_null() || id.is_null() {
                return 0;
            }
            let (Ok(kind), Ok(id)) = (
                unsafe { CStr::from_ptr(kind) }.to_str(),
                unsafe { CStr::from_ptr(id) }.to_str(),
            ) else {
                return 0;
            };
            let Ok(host) = renderer.try_borrow_mut() else {
                return 0;
            };
            let data = match kind {
                "prefab" => host
                    .app
                    .controller
                    .view_model()
                    .prefab_items
                    .into_iter()
                    .find(|p| p.id == id)
                    .and_then(|p| p.thumbnail)
                    .map(|p| (128, 128, p.as_ref().clone())),
                "texture" => host
                    .app
                    .state
                    .project
                    .assets
                    .iter()
                    .find(|a| a.id.to_string() == id)
                    .and_then(|a| a.texture.as_ref())
                    .map(|t| (t.w, t.h, t.pixels.to_vec())),
                "reference" => host
                    .app
                    .state
                    .project
                    .refs
                    .iter()
                    .find(|r| {
                        format!("{:?}", r.axis).eq_ignore_ascii_case(id)
                            || (r.axis == petunia_project::RefAxis::Side && id == "right")
                    })
                    .map(|r| (r.width, r.height, (*r.rgba).clone())),
                _ => None,
            };
            let Some((w, h, bytes)) = data else { return 0 };
            let len = bytes.len();
            unsafe {
                if !width.is_null() {
                    *width = w
                }
                if !height.is_null() {
                    *height = h
                }
                if !output.is_null() {
                    if capacity < len {
                        return 0;
                    }
                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), output, len)
                }
            }
            len
        })
        .unwrap_or(0)
}

fn uv_selection_stamp(selection: &petunia_core::UvSelectionState) -> String {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    selection.mode.hash(&mut hash);
    selection.sync.hash(&mut hash);
    for corner in selection.corners() {
        corner.hash(&mut hash);
    }
    hash.finish().to_string()
}
