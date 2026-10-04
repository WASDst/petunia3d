//! Petunia3D app: núcleo `Core` + backends wgpu/OpenGL + render-on-demand.
//! Módulos concretos vivem aqui; eventos são despachados a eles por frame.

pub mod diagnostics;
pub mod eframe_host;
pub mod watch;

use std::sync::Arc;

use petunia_core::{
    AppState, ClearSelectionCmd, DeleteSelectionCmd, DuplicateSelectionCmd, EditMode,
    InvertSelectionCmd, ModuleRegistry, SelectAllCmd, SelectMode,
};
use petunia_module_assets::AssetsModule;
use petunia_module_model::ToolRegistry;
use petunia_module_paint::PaintModule;
use petunia_module_uv::UvModule;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode as WKey, PhysicalKey};
use winit::window::{Window, WindowId};

use petunia_core::events::AppEvent;

// ---------------------------------------------------------------- Core

pub struct Core {
    pub state: AppState,
    pub tools: ToolRegistry,
    pub registry: ModuleRegistry,
    pub autosave: petunia_core::AutosaveService,
    pub recent_projects: petunia_core::RecentProjects,
    pub pending_recovery: Option<petunia_core::RecoveryInfo>,
    pub watch_service: Option<crate::watch::WatchService>,
    pub mmb_down: bool,
    pub shift_down: bool,
    pub ctrl_down: bool,
    pub alt_down: bool,
    pub last_mouse: Option<(f64, f64)>,
    pub save_requested: bool,
}

impl Default for Core {
    fn default() -> Self {
        Self::new()
    }
}

impl Core {
    pub fn new() -> Self {
        let lang = std::env::var("PETUNIA_LANG")
            .or_else(|_| std::env::var("SIMPLE3D_LANG"))
            .unwrap_or_else(|_| "pt-BR".to_string());
        let lang = if ["en", "pt-BR"].contains(&lang.as_str()) {
            lang
        } else {
            "pt-BR".to_string()
        };

        let pending_recovery = petunia_core::AutosaveService::detect_recovery(None);
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = petunia_core::AutosaveService::create_session_lock(None, "Untitled", now_secs);

        let watch_service = if std::path::Path::new("assets").is_dir() {
            crate::watch::WatchService::watch(&[std::path::Path::new("assets")]).ok()
        } else {
            None
        };

        Self {
            state: AppState::new(&lang),
            tools: ToolRegistry::with_defaults(),
            registry: {
                let mut r = ModuleRegistry::new();
                r.register(PaintModule::new());
                r.register(UvModule::new());
                r.register(AssetsModule::new());
                r
            },
            autosave: petunia_core::AutosaveService::default(),
            recent_projects: petunia_core::RecentProjects::default(),
            pending_recovery,
            watch_service,
            mmb_down: false,
            shift_down: false,
            ctrl_down: false,
            alt_down: false,
            last_mouse: None,
            save_requested: false,
        }
    }

    /// Executa o tick periódico do autosave (P3D-002).
    pub fn tick_autosave(&mut self) {
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let is_dirty = self.state.is_document_dirty();
        let proj_path = self
            .state
            .project
            .project_path
            .as_deref()
            .map(std::path::Path::new);
        if let Some(res) =
            self.autosave
                .tick(now_secs, is_dirty, &self.state.project.project, proj_path)
        {
            match res {
                Ok(path) => {
                    let filename = path
                        .file_name()
                        .and_then(|f| f.to_str())
                        .unwrap_or("snapshot");
                    self.state.set_status(format!("autosave: {filename}"));
                }
                Err(err) => {
                    self.state.set_status(format!("autosave err: {err}"));
                }
            }
        }
    }

    /// Executa o tick do WatchService (P1-12), recarregando configurações, temas e locales alterados no disco.
    pub fn tick_watcher(&mut self) {
        let Some(watcher) = &self.watch_service else {
            return;
        };
        let messages = watcher.drain_coalesced();
        for msg in messages {
            if let Some(path) = &msg.path {
                let path_str = path.to_string_lossy();
                if path_str.contains("locales") {
                    self.state.ui.i18n = petunia_config::i18n::I18n::load(&self.state.ui.i18n.lang);
                    self.state
                        .set_status(format!("reloaded translations: {}", path.display()));
                    self.state.mark_dirty();
                } else if path_str.contains("themes") {
                    self.state
                        .set_status(format!("reloaded theme: {}", path.display()));
                    self.state.mark_dirty();
                } else if path_str.contains("keybinds") {
                    self.state.ui.keybinds = petunia_config::keybinds::Keybinds::load_profile(
                        &self.state.ui.active_keymap_id,
                    );
                    self.state
                        .set_status(format!("reloaded keybinds: {}", path.display()));
                    self.state.mark_dirty();
                } else {
                    self.state
                        .set_status(format!("file changed: {}", path.display()));
                    self.state.mark_dirty();
                }
            }
        }
    }

    /// Despacha eventos acumulados aos módulos via registry.
    pub fn dispatch_events(&mut self) {
        for ev in self.state.events.drain() {
            match &ev {
                petunia_core::AppEvent::RequestImportPalette => {
                    petunia_ui::file_dialog_service::import_palette_in_canvas();
                }
                petunia_core::AppEvent::RequestExportPalette => {
                    petunia_ui::file_dialog_service::export_palette_in_canvas("palette.gpl");
                }
                _ => {}
            }
            self.registry.dispatch(&ev, &mut self.state);
        }
    }

    /// Presets de diagnóstico headless (screenshots): PETUNIA_DEMO=1 monta
    /// cena rica; WS/VIEW/SHADE/TOOL/PROFILE ajustam o estado inicial.
    pub fn apply_dev_presets(&mut self) {
        use petunia_core::{RefAxis, ViewPreset, Workspace};
        if std::env::var_os("PETUNIA_DEMO").is_some() {
            self.build_demo_scene();
        }
        if let Ok(ws) = std::env::var("PETUNIA_WS") {
            self.state.workspace = match ws.to_lowercase().as_str() {
                "paint" | "pintura" => Workspace::Paint,
                "uv" => Workspace::Uv,
                _ => Workspace::Model,
            };
        }
        if let Ok(v) = std::env::var("PETUNIA_VIEW") {
            self.state
                .camera
                .set_preset(match v.to_lowercase().as_str() {
                    "front" => ViewPreset::Front,
                    "back" => ViewPreset::Back,
                    "left" => ViewPreset::Left,
                    "right" => ViewPreset::Right,
                    "top" => ViewPreset::Top,
                    "bottom" => ViewPreset::Bottom,
                    _ => ViewPreset::Persp,
                });
        }
        if let Ok(s) = std::env::var("PETUNIA_SHADE") {
            match s.to_lowercase().as_str() {
                "wire" => self.state.shading = petunia_render::Shading::Wireframe,
                "material" | "textured" => {
                    self.state.shading = petunia_render::Shading::MaterialPreview;
                    self.state.textured = true;
                }
                "rendered" => self.state.shading = petunia_render::Shading::Rendered,
                "solid" => self.state.shading = petunia_render::Shading::Solid,
                _ => {}
            }
        }
        if let Ok(t) = std::env::var("PETUNIA_TOOL")
            && self.tools.get(&t).is_some()
        {
            self.state.active_tool = t;
        }
        if std::env::var_os("PETUNIA_PROFILE").is_some() {
            self.state.camera.set_preset(ViewPreset::Front);
            petunia_module_model::draw_profile::profile_capture_frame(&mut self.state);
            self.state.profile.points = vec![[-1.5, -1.5], [1.5, -1.5], [1.5, 1.5], [-1.5, 1.5]];
            self.state.profile.closed = true;
            self.state.active_tool = "draw_profile".to_string();
            let _ = RefAxis::Front;
        }
        self.state.mark_dirty();
    }

    /// Cena demo: vaso (revolve) + cápsula + referência procedural +
    /// canvas com círculo + textura ligada.
    fn build_demo_scene(&mut self) {
        use petunia_mesh::Mesh;
        let s = &mut self.state;
        if let Ok(mut vase) = Mesh::revolve(
            &[[0.0, 0.0], [0.8, 0.1], [1.0, 1.0], [0.6, 2.0], [0.0, 2.2]],
            14,
        ) {
            for v in &mut vase.verts {
                v.pos[0] -= 3.2;
            }
            s.project.add("Vase", vase);
        }
        let mut cap = Mesh::capsule(10, 0.5, 2.2);
        for v in &mut cap.verts {
            v.pos[0] += 3.2;
        }
        s.project.add("Capsule", cap);
        // referência procedural (gradiente + mira) — exercita o pipeline
        let (w, h) = (256u32, 256u32);
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                rgba[i] = (x as f32 / w as f32 * 255.0) as u8;
                rgba[i + 1] = (y as f32 / h as f32 * 255.0) as u8;
                rgba[i + 2] = 128;
                rgba[i + 3] = 255;
            }
        }
        for k in 0..w {
            let i = ((128 * w + k) * 4) as usize;
            rgba[i] = 255;
            rgba[i + 1] = 0;
            rgba[i + 2] = 0;
        }
        for k in 0..h {
            let i = ((k * w + 128) * 4) as usize;
            rgba[i] = 255;
            rgba[i + 1] = 0;
            rgba[i + 2] = 0;
        }
        let mut r = petunia_core::ReferenceImage::from_rgba("demo-ref".into(), w, h, rgba);
        r.axis = petunia_core::RefAxis::Front;
        s.project.refs.push(r);
        // canvas com círculo no CUBO (asset 0, central) + preview texturizado
        s.project.active = 0;
        PaintModule::ensure_canvas(s);
        if let Some(o) = s.project.active_mut()
            && let Some(cv) = o.texture.as_mut()
        {
            for y in 0..cv.h {
                for x in 0..cv.w {
                    let dx = x as i32 - 128;
                    let dy = y as i32 - 128;
                    if dx * dx + dy * dy < 60 * 60 {
                        cv.set(x, y, [220, 40, 40, 255]);
                    }
                }
            }
        }
        s.textured = true;
        s.render.canvas_dirty = true;
        s.sync_selection();
        s.mark_dirty();
    }

    pub fn on_mouse_input(&mut self, pressed: bool, button: MouseButton) {
        if button == MouseButton::Middle {
            self.mmb_down = pressed;
        }
        self.state.mark_dirty();
    }

    pub fn on_cursor_moved(&mut self, x: f64, y: f64) {
        if self.mmb_down
            && let Some((lx, ly)) = self.last_mouse
        {
            let dx = (x - lx) as f32;
            let dy = (y - ly) as f32;
            if self.shift_down {
                self.state.camera.pan(dx, dy);
            } else {
                self.state.camera.orbit(dx, dy);
            }
        }
        self.last_mouse = Some((x, y));
    }

    pub fn on_wheel(&mut self, y: f32) {
        self.state.camera.zoom(y);
        self.state.mark_dirty();
    }

    pub fn on_modifiers(&mut self, shift: bool, ctrl: bool, alt: bool) {
        self.shift_down = shift;
        self.ctrl_down = ctrl;
        self.alt_down = alt;
    }

    fn mods(&self) -> petunia_config::keybinds::Mods2 {
        petunia_config::keybinds::Mods2 {
            ctrl: self.ctrl_down,
            shift: self.shift_down,
            alt: self.alt_down,
        }
    }

    /// Teclado via keybinds configuráveis (assets/keybinds/*.toml).
    pub fn on_key(&mut self, physical: PhysicalKey) {
        if self.state.is_interacting() {
            return; // A sessão modal consome teclado no viewport egui.
        }
        if let PhysicalKey::Code(key) = physical {
            use petunia_core::ViewPreset;
            let preset = match (key, self.ctrl_down) {
                (WKey::Numpad1, false) => Some(ViewPreset::Front),
                (WKey::Numpad1, true) => Some(ViewPreset::Back),
                (WKey::Numpad3, false) => Some(ViewPreset::Right),
                (WKey::Numpad3, true) => Some(ViewPreset::Left),
                (WKey::Numpad7, false) => Some(ViewPreset::Top),
                (WKey::Numpad7, true) => Some(ViewPreset::Bottom),
                _ => None,
            };
            if let Some(preset) = preset {
                self.state.camera_frame = None;
                self.state.camera.set_preset(preset);
                self.state.mark_dirty();
                return;
            }
            if key == WKey::Numpad5 {
                self.state.camera_frame = None;
                self.state.camera.toggle_projection();
                self.state.mark_dirty();
                return;
            }
            if key == WKey::Numpad9 {
                self.state.camera_frame = None;
                self.state.camera.opposite_view();
                self.state.mark_dirty();
                return;
            }
            if key == WKey::NumpadDecimal {
                petunia_ui::frame_selection(&mut self.state);
                return;
            }
        }
        // Escape: cancela último ponto do perfil
        if physical == PhysicalKey::Code(WKey::Escape) {
            if self.state.active_tool == "draw_profile" && !self.state.profile.points.is_empty() {
                self.state.profile.points.pop();
                self.state.profile.closed = false;
                self.state.mark_dirty();
            }
            return;
        }
        if self.state.workspace == petunia_core::Workspace::Paint
            && matches!(
                physical,
                PhysicalKey::Code(WKey::KeyF | WKey::KeyG | WKey::KeyB)
            )
            && !self.ctrl_down
        {
            return; // Brush radius and eyedropper are contextual viewport input.
        }
        // Tab: alterna entre modo Objeto e última seleção de componente (P3D-015); Shift+Tab: Snap
        if physical == PhysicalKey::Code(WKey::Tab) && !self.ctrl_down {
            if self.shift_down {
                self.state.snap_enabled = !self.state.snap_enabled;
                self.state.snap_settings.enabled = self.state.snap_enabled;
                self.state.mark_dirty();
            } else {
                self.state.cycle_selection_domain();
            }
            return;
        }
        // Tecla 0: Seleção de Objeto
        if physical == PhysicalKey::Code(WKey::Digit0) && !self.ctrl_down && !self.shift_down {
            self.state.set_edit_mode(EditMode::Object);
            self.state.active_tool = "select".into();
            self.state.mark_dirty();
            return;
        }
        // Alt+Z: modo Raio-X
        if physical == PhysicalKey::Code(WKey::KeyZ) && self.alt_down {
            self.state.show_xray = !self.state.show_xray;
            self.state.mark_dirty();
            return;
        }
        // NumpadDivide / Slash: Isolar objeto ativo (Local View)
        if matches!(
            physical,
            PhysicalKey::Code(WKey::NumpadDivide | WKey::Slash)
        ) && !self.ctrl_down
        {
            self.state.toggle_isolate();
            return;
        }
        let mods = self.mods();
        let ck = to_config_key(physical);
        // A ordem do mapa de atalhos é aleatória: com teclas compartilhadas
        // entre contextos (E = Extrude no modelo, borracha na pintura), `find`
        // escolhia uma ação diferente a cada execução.
        let context = if self.state.edit_mode() == EditMode::TexturePaint {
            "paint"
        } else {
            "model"
        };
        let action = ck
            .and_then(|k| self.state.ui.keybinds.find_in_context(k, mods, context))
            .unwrap_or("");
        let action = action.to_string();
        match action.as_str() {
            "global.undo" => {
                self.state.undo();
            }
            "global.redo" => {
                self.state.redo();
            }
            "global.save_project" => {
                self.save_requested = true;
            }
            "global.toggle_wireframe" | "view.toggle_wireframe" => {
                self.state.shading = match self.state.shading {
                    petunia_render::Shading::Wireframe => petunia_render::Shading::Solid,
                    _ => petunia_render::Shading::Wireframe,
                };
                self.state.mark_dirty();
            }
            "global.help" => {
                self.state.ui.show_help = !self.state.ui.show_help;
                self.state.mark_dirty();
            }
            "global.command_palette" | "window.command_palette" => {
                self.state.ui.show_command_palette = !self.state.ui.show_command_palette;
                if self.state.ui.show_command_palette {
                    self.state.ui.command_palette_query.clear();
                    self.state.ui.command_palette_selected_index = 0;
                }
                self.state.mark_dirty();
            }
            "global.settings" => {
                self.state.ui.show_settings = !self.state.ui.show_settings;
                self.state.mark_dirty();
            }
            "global.toggle_projection" | "view.toggle_projection" => {
                self.state.camera_frame = None;
                self.state.camera.toggle_projection();
                self.state.mark_dirty();
            }
            "global.reset_camera" | "view.reset_camera" => {
                self.state.camera_frame = None;
                self.state.camera.reset();
                self.state.mark_dirty();
            }
            "global.cycle_mode" => {
                // Tab nunca entra num "modo" separado: alterna o domínio de seleção (P3D-015).
                match self.state.selection_domain() {
                    petunia_core::SelectionDomain::Object => {
                        self.state.set_edit_mode(EditMode::Edit);
                    }
                    _ => self.state.set_edit_mode(EditMode::Object),
                }
                self.state.mark_dirty();
            }
            "model.select_vertex" => self.set_tool("select", Some(SelectMode::Vertex)),
            "model.select_edge" => self.set_tool("select", Some(SelectMode::Edge)),
            "model.select_face" => self.set_tool("select", Some(SelectMode::Face)),
            "model.select_object" => {
                self.state.set_edit_mode(EditMode::Object);
                self.state.active_tool = "select".into();
                self.state.mark_dirty();
            }
            "model.frame_selection" => petunia_ui::frame_selection(&mut self.state),
            "model.transform" => self.set_tool("transform", None),
            "model.move" | "model.rotate" | "model.scale" => {
                let (tool_id, kind) = match action.as_str() {
                    "model.move" => ("move", petunia_core::modal::ModalKind::Move),
                    "model.rotate" => ("rotate", petunia_core::modal::ModalKind::Rotate),
                    _ => ("scale", petunia_core::modal::ModalKind::Scale),
                };
                self.state.pending_modal = Some(kind);
                self.state.active_tool = tool_id.into();
                self.state.mark_dirty();
            }
            "model.primitives" => self.set_tool("primitives", None),
            "model.draw_profile" => self.set_tool("draw_profile", None),
            "model.merge" => self.set_tool("merge", None),
            "model.inset" => self.set_tool("inset", None),
            "model.bevel" => self.set_tool("bevel", None),
            "model.subdivide" => self.set_tool("subdivide", None),
            "model.push_pull" => self.set_tool("pushpull", None),
            "model.extrude" => {
                self.set_tool("extrude", None);
            }
            "model.extrude_individual" => {
                let dist = if self.state.extrude_dist == 0.0 {
                    0.5
                } else {
                    self.state.extrude_dist
                };
                let _ = self
                    .state
                    .dispatch(&petunia_core::ExtrudeIndividualCmd { dist });
            }
            "model.flip_diagonal" => {
                let _ = self.state.dispatch(&petunia_core::FlipDiagonalCmd);
            }
            "model.revolve" => self.set_tool("revolve", None),
            "model.delete" => {
                let _ = self.state.dispatch(&DeleteSelectionCmd);
            }
            "model.duplicate" => {
                let _ = self.state.dispatch(&DuplicateSelectionCmd);
            }
            "model.slice" => self.set_tool("slice", None),
            "model.knife" | "model.loop_cut" => {
                self.state.active_tool = if action == "model.knife" {
                    "knife"
                } else {
                    "loop_cut"
                }
                .into();
                self.state.mark_dirty();
            }
            "model.connect" => self.set_tool("connect", None),
            "model.dissolve" => self.set_tool("dissolve", None),
            "model.select_all" => {
                let _ = self.state.dispatch(&SelectAllCmd);
            }
            "model.deselect_all" => {
                let _ = self.state.dispatch(&ClearSelectionCmd);
            }
            "model.invert_selection" => {
                let _ = self.state.dispatch(&InvertSelectionCmd);
            }
            "model.select_linked" => {
                let _ = self.state.dispatch(&petunia_core::SelectLinkedCmd);
            }
            "paint.paint" => self.set_tool("paint", None),
            "paint.size_decrease"
            | "paint.size_increase"
            | "paint.hardness_decrease"
            | "paint.hardness_increase"
                if self.state.workspace == petunia_core::Workspace::Paint
                    || self.state.active_tool == "paint" =>
            {
                let size_step = 1.0;
                let hardness_step = 0.05;
                match action.as_str() {
                    "paint.size_decrease" => {
                        self.state.canvas_brush = self
                            .state
                            .canvas_brush
                            .saturating_sub(size_step as u32)
                            .max(1);
                    }
                    "paint.size_increase" => {
                        self.state.canvas_brush = self
                            .state
                            .canvas_brush
                            .saturating_add(size_step as u32)
                            .min(512);
                    }
                    "paint.hardness_decrease" => {
                        self.state.brush_hardness =
                            (self.state.brush_hardness - hardness_step).clamp(0.0, 1.0);
                    }
                    "paint.hardness_increase" => {
                        self.state.brush_hardness =
                            (self.state.brush_hardness + hardness_step).clamp(0.0, 1.0);
                    }
                    _ => {}
                }
                self.state.mark_dirty();
            }
            _ => {}
        }
    }

    fn set_tool(&mut self, id: &str, select: Option<SelectMode>) {
        self.state.active_tool = id.to_string();
        if let Some(sm) = select {
            self.state.set_edit_mode(EditMode::Edit);
            self.state.select_mode = sm;
            match sm {
                SelectMode::Edge => {
                    if let Some(m) = self.state.project.active_mesh_mut() {
                        m.sync_edge_selection_from_verts();
                    }
                }
                SelectMode::Face => {
                    if let Some(m) = self.state.project.active_mesh_mut() {
                        m.sync_face_selection_from_verts();
                    }
                }
                SelectMode::Vertex => {}
            }
        }
        if id == "paint" {
            self.state.set_edit_mode(EditMode::TexturePaint);
        }
        if let Some(t) = self.tools.get(id) {
            t.on_activate(&mut self.state);
        }
        self.state
            .events
            .emit(AppEvent::ToolActivated(id.to_string()));
        self.state.mark_dirty();
    }
}

fn to_config_key(p: PhysicalKey) -> Option<petunia_config::keybinds::winit_keys::KeyCode> {
    use petunia_config::keybinds::winit_keys::KeyCode as C;
    let c = match p {
        PhysicalKey::Code(WKey::KeyA) => C::KeyA,
        PhysicalKey::Code(WKey::KeyB) => C::KeyB,
        PhysicalKey::Code(WKey::KeyC) => C::KeyC,
        PhysicalKey::Code(WKey::KeyD) => C::KeyD,
        PhysicalKey::Code(WKey::KeyE) => C::KeyE,
        PhysicalKey::Code(WKey::KeyF) => C::KeyF,
        PhysicalKey::Code(WKey::KeyG) => C::KeyG,
        PhysicalKey::Code(WKey::KeyH) => C::KeyH,
        PhysicalKey::Code(WKey::KeyI) => C::KeyI,
        PhysicalKey::Code(WKey::KeyJ) => C::KeyJ,
        PhysicalKey::Code(WKey::KeyK) => C::KeyK,
        PhysicalKey::Code(WKey::KeyL) => C::KeyL,
        PhysicalKey::Code(WKey::KeyM) => C::KeyM,
        PhysicalKey::Code(WKey::KeyN) => C::KeyN,
        PhysicalKey::Code(WKey::KeyO) => C::KeyO,
        PhysicalKey::Code(WKey::KeyP) => C::KeyP,
        PhysicalKey::Code(WKey::KeyQ) => C::KeyQ,
        PhysicalKey::Code(WKey::KeyR) => C::KeyR,
        PhysicalKey::Code(WKey::KeyS) => C::KeyS,
        PhysicalKey::Code(WKey::KeyT) => C::KeyT,
        PhysicalKey::Code(WKey::KeyU) => C::KeyU,
        PhysicalKey::Code(WKey::KeyV) => C::KeyV,
        PhysicalKey::Code(WKey::KeyW) => C::KeyW,
        PhysicalKey::Code(WKey::KeyX) => C::KeyX,
        PhysicalKey::Code(WKey::KeyY) => C::KeyY,
        PhysicalKey::Code(WKey::KeyZ) => C::KeyZ,
        PhysicalKey::Code(WKey::Digit0) => C::Digit0,
        PhysicalKey::Code(WKey::Digit1) => C::Digit1,
        PhysicalKey::Code(WKey::Digit2) => C::Digit2,
        PhysicalKey::Code(WKey::Digit3) => C::Digit3,
        PhysicalKey::Code(WKey::Digit4) => C::Digit4,
        PhysicalKey::Code(WKey::Digit5) => C::Digit5,
        PhysicalKey::Code(WKey::Digit6) => C::Digit6,
        PhysicalKey::Code(WKey::Digit7) => C::Digit7,
        PhysicalKey::Code(WKey::Digit8) => C::Digit8,
        PhysicalKey::Code(WKey::Digit9) => C::Digit9,
        PhysicalKey::Code(WKey::F1) => C::F1,
        PhysicalKey::Code(WKey::F2) => C::F2,
        PhysicalKey::Code(WKey::F3) => C::F3,
        PhysicalKey::Code(WKey::F4) => C::F4,
        PhysicalKey::Code(WKey::F5) => C::F5,
        PhysicalKey::Code(WKey::F6) => C::F6,
        PhysicalKey::Code(WKey::F7) => C::F7,
        PhysicalKey::Code(WKey::F8) => C::F8,
        PhysicalKey::Code(WKey::F9) => C::F9,
        PhysicalKey::Code(WKey::F10) => C::F10,
        PhysicalKey::Code(WKey::F11) => C::F11,
        PhysicalKey::Code(WKey::F12) => C::F12,
        PhysicalKey::Code(WKey::Tab) => C::Tab,
        PhysicalKey::Code(WKey::Space) => C::Space,
        PhysicalKey::Code(WKey::Delete) => C::Delete,
        PhysicalKey::Code(WKey::Backspace) => C::Backspace,
        PhysicalKey::Code(WKey::Home) => C::Home,
        PhysicalKey::Code(WKey::End) => C::End,
        PhysicalKey::Code(WKey::Escape) => C::Escape,
        PhysicalKey::Code(WKey::Enter) => C::Enter,
        PhysicalKey::Code(WKey::BracketLeft) => C::BracketLeft,
        PhysicalKey::Code(WKey::BracketRight) => C::BracketRight,
        _ => return None,
    };
    Some(c)
}

/// Pick no viewport: draw-profile / paint (+Alt eyedropper) / seleção.
pub fn handle_pick(core: &mut Core, nx: f32, ny: f32) {
    if core.state.active_tool == "draw_profile" {
        petunia_module_model::draw_profile::profile_add_point(&mut core.state, nx, ny);
        return;
    }
    let (origin, dir) = core.state.camera.ray(nx, ny);
    let paint_mode =
        core.state.workspace == petunia_core::Workspace::Paint || core.state.active_tool == "paint";
    if paint_mode && core.alt_down {
        if let Some((vi, _)) = core.state.pick_vertex(origin, dir) {
            PaintModule::eyedrop_vertex(&mut core.state, vi);
            core.state.set_status(core.state.t("paint.picked"));
        }
        return;
    }
    if paint_mode {
        // ponto real na superfície (face) cai melhor que vértice p/ pintar
        let p = core
            .state
            .project
            .assets
            .get(core.state.project.active)
            .and_then(|o| o.mesh.ray_hit(origin, dir))
            .map(|(_, _, p)| p)
            .or_else(|| core.state.pick_vertex(origin, dir).map(|(_, p)| p));
        if let Some(p) = p {
            core.state.paint_at(p);
        }
        return;
    }
    use petunia_core::picking::{PickComponent, pick_mesh};
    let viewport = core
        .state
        .ui
        .viewport_rect
        .map(|r| glam::Vec2::new(r.width(), r.height()))
        .unwrap_or(glam::Vec2::new(800.0, 600.0))
        * core.state.ui.viewport_pixels_per_point;

    if core.state.session.edit_mode() == EditMode::Object {
        let is_wire =
            core.state.shading == petunia_render::Shading::Wireframe || core.state.show_xray;
        let mut closest_hit: Option<(usize, f32)> = None;
        for (idx, asset) in core.state.project.assets.iter().enumerate() {
            if !asset.visible || asset.locked {
                continue;
            }
            if let Some(hit) = pick_mesh(
                &asset.mesh,
                &core.state.camera,
                viewport,
                glam::Vec2::new(nx, ny),
                SelectMode::Face,
                is_wire,
            ) {
                let dist = (hit.position - core.state.camera.eye()).length();
                if closest_hit.is_none_or(|(_, min_dist)| dist < min_dist) {
                    closest_hit = Some((idx, dist));
                }
            }
        }

        if let Some((best_idx, _)) = closest_hit {
            if !core.shift_down {
                for (idx, asset) in core.state.project.assets.iter_mut().enumerate() {
                    if idx != best_idx {
                        asset.mesh.deselect_all();
                    }
                }
                core.state.project.active = best_idx;
                if let Some(mesh) = core.state.project.active_mesh_mut() {
                    mesh.select_all();
                }
            } else {
                core.state.project.active = best_idx;
                if let Some(mesh) = core.state.project.active_mesh_mut() {
                    let any_selected = mesh.verts.iter().any(|v| v.selected);
                    if any_selected {
                        mesh.deselect_all();
                    } else {
                        mesh.select_all();
                    }
                }
            }
            core.state.mark_dirty();
        } else if !core.shift_down {
            for asset in &mut core.state.project.assets {
                asset.mesh.deselect_all();
            }
            core.state.mark_dirty();
        }
        core.state.sync_selection();
        return;
    }

    let mode = core.state.select_mode;
    let hit = core.state.project.active_mesh().and_then(|mesh| {
        pick_mesh(
            mesh,
            &core.state.camera,
            viewport,
            glam::Vec2::new(nx, ny),
            mode,
            core.state.shading == petunia_render::Shading::Wireframe || core.state.show_xray,
        )
    });
    if let Some(mesh) = core.state.project.active_mesh_mut() {
        // Shift toggles; a plain click replaces selection without ambiguous fallbacks.
        let was_selected = hit.is_some_and(|h| match h.component {
            PickComponent::Vertex(i) => mesh.verts.get(i).is_some_and(|v| v.selected),
            PickComponent::Edge(a, b) => mesh.selected_edges.contains(&(a.min(b), a.max(b))),
            PickComponent::Face(i) => mesh.faces.get(i).is_some_and(|f| f.selected),
        });
        if !core.shift_down {
            mesh.deselect_all();
        }
        let selected = !core.shift_down || !was_selected;
        if let Some(hit) = hit {
            match hit.component {
                PickComponent::Vertex(i) => {
                    if let Some(v) = mesh.verts.get_mut(i) {
                        v.selected = selected;
                    }
                    mesh.sync_face_selection_from_verts();
                }
                PickComponent::Edge(a, b) => {
                    let edge = (a.min(b), a.max(b));
                    if selected {
                        mesh.selected_edges.insert(edge);
                    } else {
                        mesh.selected_edges.remove(&edge);
                    }
                    for v in &mut mesh.verts {
                        v.selected = false;
                    }
                    for &(a, b) in &mesh.selected_edges {
                        for i in [a, b] {
                            if let Some(v) = mesh.verts.get_mut(i as usize) {
                                v.selected = true;
                            }
                        }
                    }
                    mesh.sync_face_selection_from_verts();
                }
                PickComponent::Face(i) => {
                    if let Some(f) = mesh.faces.get_mut(i) {
                        f.selected = selected;
                    }
                    mesh.sync_vert_selection_from_faces();
                }
            }
        }
    }
    core.state.sync_selection();
}

// ---------------------------------------------------------------------------
// Backend wgpu
// ---------------------------------------------------------------------------

struct WgpuGfx {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    renderer3d: petunia_render_wgpu::Renderer,
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
}

/// Target de display seguro para o egui-winit.
/// Retorna `Unavailable` para display_handle, prevenindo a inicialização da thread
/// worker instável do `smithay-clipboard` no Wayland que causa double-free/segfault em `wl_proxy_destroy`,
/// enquanto delega o clipboard de forma 100% segura para o `arboard`.
struct SafeDisplayTarget;

impl winit::raw_window_handle::HasDisplayHandle for SafeDisplayTarget {
    fn display_handle(
        &self,
    ) -> Result<winit::raw_window_handle::DisplayHandle<'_>, winit::raw_window_handle::HandleError>
    {
        Err(winit::raw_window_handle::HandleError::Unavailable)
    }
}

struct WgpuApp {
    core: Core,
    gfx: Option<WgpuGfx>,
    fps_acc: f32,
    fps_n: u32,
}

impl WgpuApp {
    fn new() -> Self {
        Self {
            core: Core::new(),
            gfx: None,
            fps_acc: 0.0,
            fps_n: 0,
        }
    }

    fn init_gfx(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Petunia3D")
                        .with_inner_size(PhysicalSize::new(1280, 800)),
                )
                .map_err(|error| format!("wgpu window: {error}"))?,
        );
        let size = window.inner_size();
        let mut instance_desc = wgpu::InstanceDescriptor::new_without_display_handle();
        instance_desc.backends = wgpu::Backends::all();
        let instance = wgpu::Instance::new(instance_desc);
        let surface = instance
            .create_surface(Arc::clone(&window))
            .map_err(|error| format!("wgpu surface: {error}"))?;
        let adapter = pollster::block_on(pick_adapter(&instance, &surface))?;
        let info = adapter.get_info();
        self.core.state.render.backend_name = format!("wgpu {:?} {}", info.backend, info.name);
        eprintln!(
            "petunia3d: GPU: {} ({:?} via {:?}, driver {})",
            info.name, info.device_type, info.backend, info.driver_info
        );
        let (device, queue) = pollster::block_on(pick_device(&adapter))?;
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .or_else(|| caps.formats.first().copied())
            .ok_or_else(|| "wgpu: surface exposes no texture format".to_owned())?;
        let alpha_mode = caps
            .alpha_modes
            .first()
            .copied()
            .ok_or_else(|| "wgpu: surface exposes no alpha mode".to_owned())?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        surface.configure(&device, &config);

        let renderer3d = petunia_render_wgpu::Renderer::new(&device, format);

        let egui_ctx = egui::Context::default();
        petunia_ui::apply_theme_to_egui(&petunia_config::Theme::load(), &egui_ctx);
        petunia_ui::icon_registry::IconRegistry::ensure_fonts(&egui_ctx);
        petunia_ui::image_kit::install_image_loaders(&egui_ctx);
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &SafeDisplayTarget,
            None,
            None,
            None,
        );
        let egui_renderer =
            egui_wgpu::Renderer::new(&device, format, egui_wgpu::RendererOptions::default());

        self.core.apply_dev_presets();
        self.gfx = Some(WgpuGfx {
            window,
            surface,
            device,
            queue,
            config,
            renderer3d,
            egui_ctx,
            egui_state,
            egui_renderer,
        });
        self.core.state.mark_dirty();
        Ok(())
    }

    fn redraw(&mut self) {
        let t0 = std::time::Instant::now();
        let Some(gfx) = self.gfx.as_mut() else { return };

        if let Some(rect) = self.core.state.ui.viewport_rect {
            if rect.width() > 1.0 && rect.height() > 1.0 {
                self.core.state.camera.aspect = rect.width() / rect.height();
            }
        } else {
            let s = gfx.window.inner_size();
            self.core.state.camera.aspect = s.width as f32 / s.height.max(1) as f32;
        }

        let raw_input = gfx.egui_state.take_egui_input(&gfx.window);
        let mut quit = false;
        let mut full_output = gfx.egui_ctx.run_ui(raw_input, |ui| {
            let mut act = petunia_ui::UiAction::none();
            petunia_ui::draw(
                ui,
                &mut self.core.state,
                &self.core.tools,
                &mut self.core.registry,
                &mut act,
            );
            if let Some(ref info) = self.core.pending_recovery
                && let Some(rec_act) =
                    petunia_ui::draw_recovery_dialog(ui.ctx(), &mut self.core.state, info)
            {
                match rec_act {
                    petunia_ui::RecoveryAction::Recover | petunia_ui::RecoveryAction::OpenSaved => {
                        self.core.pending_recovery = None;
                    }
                    petunia_ui::RecoveryAction::Discard => {
                        let _ = petunia_core::AutosaveService::discard_recovery(
                            info.main_project_path.as_deref(),
                        );
                        self.core.pending_recovery = None;
                    }
                }
            }
            quit = act.quit;
        });
        gfx.egui_state
            .handle_platform_output(&gfx.window, full_output.platform_output.clone());
        self.core.dispatch_events();
        self.core.tick_autosave();
        self.core.tick_watcher();

        if let Some((nx, ny)) = self.core.state.ui.pending_pick.take() {
            handle_pick(&mut self.core, nx, ny);
        }
        if self.core.save_requested {
            self.core.save_requested = false;
            petunia_ui::save_project_dialog(&mut self.core.state, false);
        }

        gfx.renderer3d.update(
            &gfx.device,
            &gfx.queue,
            &self.core.state.project,
            &self.core.state.project.refs,
            &self.core.state.camera,
            self.core.state.shading,
            self.core.state.show_xray,
            self.core.state.show_triangulation,
            self.core.state.textured,
            self.core.state.show_wireframe_overlay,
            self.core.state.show_face_orientation,
            self.core.state.show_uv_checker,
            self.core.state.selection_domain(),
            self.core.state.session.tools.hover,
        );
        gfx.renderer3d
            .set_xray_opacity(self.core.state.session.xray_opacity);
        gfx.renderer3d
            .set_overlays(self.core.state.show_overlays, self.core.state.show_grid);
        gfx.renderer3d
            .upload_ref_pixels(&gfx.queue, &self.core.state.project.refs);

        let paint_jobs = gfx
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        let screen_desc = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [gfx.config.width, gfx.config.height],
            pixels_per_point: full_output.pixels_per_point,
        };

        let frame = match gfx.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                gfx.surface.configure(&gfx.device, &gfx.config);
                full_output.textures_delta.clear();
                return;
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                full_output.textures_delta.clear();
                return;
            }
        };
        let view = frame.texture.create_view(&Default::default());

        gfx.renderer3d
            .resize(&gfx.device, gfx.config.width, gfx.config.height);

        let mut encoder = gfx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("petunia-encoder"),
            });

        {
            let Some(depth) = gfx.renderer3d.depth_view() else {
                eprintln!("petunia3d: render skipped: depth buffer unavailable");
                full_output.textures_delta.clear();
                return;
            };
            let bg = petunia_config::ThemeRegistry::global()
                .get_theme(&self.core.state.ui.active_theme_id)
                .map(|t| {
                    t.colors
                        .get_token_color(petunia_config::ThemeToken::BgCanvas)
                        .to_rgba_f32()
                })
                .unwrap_or([0.117, 0.117, 0.133, 1.0]);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("petunia-3d"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: bg[0] as f64,
                            g: bg[1] as f64,
                            b: bg[2] as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if let Some(viewport) = petunia_core::viewport::PhysicalViewport::from_logical(
                self.core.state.ui.viewport_rect,
                self.core.state.ui.viewport_pixels_per_point,
                gfx.config.width,
                gfx.config.height,
            ) {
                pass.set_viewport(
                    viewport.x as f32,
                    viewport.y as f32,
                    viewport.width as f32,
                    viewport.height as f32,
                    0.0,
                    1.0,
                );
                pass.set_scissor_rect(viewport.x, viewport.y, viewport.width, viewport.height);
                gfx.renderer3d
                    .render(&mut pass, &self.core.state.project.refs);
            }
        }

        for (id, deltas) in &full_output.textures_delta.set {
            for delta in deltas {
                gfx.egui_renderer
                    .update_texture(&gfx.device, &gfx.queue, *id, delta);
            }
        }
        let user_cmds = gfx.egui_renderer.update_buffers(
            &gfx.device,
            &gfx.queue,
            &mut encoder,
            &paint_jobs,
            &screen_desc,
        );
        {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("petunia-egui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            gfx.egui_renderer
                .render(&mut pass.forget_lifetime(), &paint_jobs, &screen_desc);
        }
        for id in &full_output.textures_delta.free {
            gfx.egui_renderer.free_texture(id);
        }
        full_output.textures_delta.clear();

        gfx.queue
            .submit(user_cmds.into_iter().chain([encoder.finish()]));
        gfx.queue.present(frame);

        self.update_stats(t0);
        if quit {
            let proj_path = self
                .core
                .state
                .project
                .project_path
                .as_deref()
                .map(std::path::Path::new);
            petunia_core::AutosaveService::remove_session_lock(proj_path);
            std::process::exit(0);
        }
    }

    fn update_stats(&mut self, t0: std::time::Instant) {
        let ms = t0.elapsed().as_secs_f32() * 1000.0;
        self.fps_acc += ms;
        self.fps_n += 1;
        let (v, t) = self.core.state.project.totals();
        // estimativa honesta de draws: malha+arestas por asset + refs + grid
        let draws = self
            .core
            .state
            .project
            .assets
            .iter()
            .filter(|a| a.visible)
            .count()
            * 2
            + self
                .core
                .state
                .project
                .refs
                .iter()
                .filter(|r| r.visible)
                .count()
            + 1;
        self.core.state.render.stats.tris = t;
        self.core.state.render.stats.verts = v;
        self.core.state.render.stats.draws = draws;
        if self.fps_acc >= 500.0 && self.fps_n > 0 {
            self.core.state.render.stats.fps = 1000.0 * self.fps_n as f32 / self.fps_acc;
            self.core.state.render.stats.frame_ms = self.fps_acc / self.fps_n as f32;
            self.fps_acc = 0.0;
            self.fps_n = 0;
        }
    }
}

async fn pick_adapter(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface<'_>,
) -> Result<wgpu::Adapter, String> {
    let attempts = [
        (
            "high-performance",
            wgpu::PowerPreference::HighPerformance,
            true,
            false,
        ),
        ("low-power", wgpu::PowerPreference::LowPower, true, false),
        ("qualquer", wgpu::PowerPreference::None, true, false),
        ("software", wgpu::PowerPreference::None, true, true),
    ];
    for (name, power, with_surface, fallback) in attempts {
        let req = wgpu::RequestAdapterOptions {
            power_preference: power,
            compatible_surface: if with_surface { Some(surface) } else { None },
            force_fallback_adapter: fallback,
            apply_limit_buckets: false,
        };
        match instance.request_adapter(&req).await {
            Ok(a) => {
                if !with_surface && !a.is_surface_supported(surface) {
                    let i = a.get_info();
                    eprintln!(
                        "petunia3d: adapter {:?} ({}) sem suporte à janela, tentando próximo",
                        i.backend, i.name
                    );
                    continue;
                }
                eprintln!("petunia3d: adapter via tentativa '{name}'");
                return Ok(a);
            }
            Err(e) => eprintln!("petunia3d: tentativa '{name}' falhou: {e:?}"),
        }
    }
    Err("No compatible wgpu adapter. Use SIMPLE3D_BACKEND=gl or PETUNIA_BACKEND=gl.".to_owned())
}

async fn pick_device(adapter: &wgpu::Adapter) -> Result<(wgpu::Device, wgpu::Queue), String> {
    let mk = |limits: wgpu::Limits| wgpu::DeviceDescriptor {
        label: Some("petunia3d"),
        required_features: wgpu::Features::empty(),
        required_limits: limits,
        experimental_features: Default::default(),
        memory_hints: Default::default(),
        trace: Default::default(),
    };
    match adapter.request_device(&mk(wgpu::Limits::default())).await {
        Ok(d) => Ok(d),
        Err(e) => {
            eprintln!("petunia3d: limits padrão recusados ({e}), usando downlevel");
            adapter
                .request_device(&mk(wgpu::Limits::downlevel_defaults()))
                .await
                .map_err(|error| {
                    format!("wgpu device creation failed with downlevel limits: {error}")
                })
        }
    }
}

impl ApplicationHandler for WgpuApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gfx.is_none() {
            if let Err(error) = self.init_gfx(event_loop) {
                self.core.state.set_status(error);
                event_loop.exit();
                return;
            }
            if let Some(g) = self.gfx.as_ref() {
                g.window.request_redraw();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(gfx) = self.gfx.as_mut() else { return };

        // Intercepta a tecla Tab antes do egui para garantir alternância instantânea de modo (P3D-015)
        if let WindowEvent::KeyboardInput {
            event:
                winit::event::KeyEvent {
                    state: ElementState::Pressed,
                    physical_key: PhysicalKey::Code(WKey::Tab),
                    ..
                },
            ..
        } = &event
        {
            self.core.on_key(PhysicalKey::Code(WKey::Tab));
            gfx.window.request_redraw();
            return;
        }

        let is_shortcut_key = matches!(
            &event,
            WindowEvent::KeyboardInput {
                event: winit::event::KeyEvent {
                    state: ElementState::Pressed,
                    physical_key: PhysicalKey::Code(
                        WKey::Tab
                            | WKey::Digit0
                            | WKey::Digit1
                            | WKey::Digit2
                            | WKey::Digit3
                            | WKey::Digit4
                            | WKey::Delete
                            | WKey::Backspace
                            | WKey::KeyD
                            | WKey::KeyX
                            | WKey::KeyA
                            | WKey::KeyG
                            | WKey::KeyR
                            | WKey::KeyS
                            | WKey::KeyZ
                    ),
                    ..
                },
                ..
            }
        );

        let resp = gfx.egui_state.on_window_event(&gfx.window, &event);
        if resp.repaint {
            gfx.window.request_redraw();
        }
        if resp.consumed && (!is_shortcut_key || gfx.egui_ctx.egui_wants_keyboard_input()) {
            if matches!(event, WindowEvent::RedrawRequested) {
                self.redraw();
            }
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                self.gfx = None;
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                let PhysicalSize { width, height } = size;
                if width > 0 && height > 0 {
                    gfx.config.width = width;
                    gfx.config.height = height;
                    gfx.surface.configure(&gfx.device, &gfx.config);
                }
                gfx.window.request_redraw();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.core
                    .on_mouse_input(state == ElementState::Pressed, button);
                gfx.window.request_redraw();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let y = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y * 40.0,
                    winit::event::MouseScrollDelta::PixelDelta(p) => p.y as f32,
                };
                let _ = y; // O viewport processa scroll com foco e modalidade.
                gfx.window.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                // egui-winit already reports whether pointer movement needs repaint.
                // Avoid a second unconditional full 3D + UI frame on every event.
                self.core.last_mouse = Some((position.x, position.y));
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    self.core.on_key(event.physical_key);
                    gfx.window.request_redraw();
                }
            }
            WindowEvent::ModifiersChanged(m) => {
                let s = m.state();
                self.core
                    .on_modifiers(s.shift_key(), s.control_key(), s.alt_key());
            }
            WindowEvent::RedrawRequested => self.redraw(),
            _ => {}
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.gfx = None;
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _id: winit::event::DeviceId,
        _event: DeviceEvent,
    ) {
    }

    /// Render-on-demand (§33): só redesenha se algo mudou.
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let spin = std::env::var_os("SIMPLE3D_SPIN").is_some();
        if (spin || self.core.state.consume_dirty())
            && let Some(g) = self.gfx.as_ref()
        {
            g.window.request_redraw();
        }
    }
}

// ---------------------------------------------------------------------------
// Backend OpenGL puro (glow + glutin)
// ---------------------------------------------------------------------------

struct GlGfx {
    gl_window: petunia_render_gl::GlWindow,
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_painter: egui_glow::Painter,
    gl_renderer: petunia_render_gl::GlRenderer,
}

struct GlApp {
    core: Core,
    gfx: Option<GlGfx>,
    fps_acc: f32,
    fps_n: u32,
    shot_path: Option<String>,
    shot_frames: u32,
    shot_taken: u32,
}

impl GlApp {
    fn new() -> Self {
        Self {
            core: Core::new(),
            gfx: None,
            fps_acc: 0.0,
            fps_n: 0,
            shot_path: std::env::var("PETUNIA_SCREENSHOT").ok(),
            shot_frames: std::env::var("PETUNIA_SHOT_FRAMES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            shot_taken: 0,
        }
    }

    fn init_gfx(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        // Todo conhecimento GL vive em render-gl (§27); aqui só UI + loop.
        // PETUNIA_SHOT_SIZE=WxH fixa a janela no modo screenshot.
        let attrs = if let Some((w, h)) = std::env::var("PETUNIA_SHOT_SIZE").ok().and_then(|s| {
            let (w, h) = s.split_once('x')?;
            Some((w.parse::<u32>().ok()?, h.parse::<u32>().ok()?))
        }) {
            Window::default_attributes()
                .with_title("Petunia3D (OpenGL)")
                .with_inner_size(winit::dpi::LogicalSize::new(w, h))
        } else {
            Window::default_attributes()
                .with_title("Petunia3D (OpenGL)")
                .with_inner_size(PhysicalSize::new(1280, 800))
        };
        let gl_window = petunia_render_gl::GlWindow::create(event_loop, attrs)?;
        let caps = gl_window.caps();
        self.core.state.render.backend_name =
            format!("OpenGL {} ({})", caps.gl_version, caps.renderer);
        eprintln!("petunia3d: OpenGL: {} | {}", caps.gl_version, caps.renderer);

        let egui_ctx = egui::Context::default();
        petunia_ui::apply_theme_to_egui(&petunia_config::Theme::load(), &egui_ctx);
        petunia_ui::icon_registry::IconRegistry::ensure_fonts(&egui_ctx);
        petunia_ui::image_kit::install_image_loaders(&egui_ctx);
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &SafeDisplayTarget,
            None,
            None,
            None,
        );
        let egui_painter = egui_glow::Painter::new(Arc::clone(gl_window.gl()), "", None, false)
            .map_err(|error| format!("OpenGL UI painter: {error}"))?;
        let gl_renderer = petunia_render_gl::GlRenderer::new(Arc::clone(gl_window.gl()))?;

        self.core.apply_dev_presets();
        self.gfx = Some(GlGfx {
            gl_window,
            egui_ctx,
            egui_state,
            egui_painter,
            gl_renderer,
        });
        self.core.state.mark_dirty();
        Ok(())
    }

    fn redraw(&mut self) {
        let t0 = std::time::Instant::now();
        let Some(g) = self.gfx.as_mut() else { return };

        if let Some(rect) = self.core.state.ui.viewport_rect {
            if rect.width() > 1.0 && rect.height() > 1.0 {
                self.core.state.camera.aspect = rect.width() / rect.height();
            }
        } else {
            let s = g.gl_window.window().inner_size();
            self.core.state.camera.aspect = s.width as f32 / s.height.max(1) as f32;
        }
        let (w, h) = g.gl_window.size();

        let raw_input = g.egui_state.take_egui_input(g.gl_window.window());
        let mut quit = false;
        let mut full_output = g.egui_ctx.run_ui(raw_input, |ui| {
            let mut act = petunia_ui::UiAction::none();
            petunia_ui::draw(
                ui,
                &mut self.core.state,
                &self.core.tools,
                &mut self.core.registry,
                &mut act,
            );
            if let Some(ref info) = self.core.pending_recovery
                && let Some(rec_act) =
                    petunia_ui::draw_recovery_dialog(ui.ctx(), &mut self.core.state, info)
            {
                match rec_act {
                    petunia_ui::RecoveryAction::Recover | petunia_ui::RecoveryAction::OpenSaved => {
                        self.core.pending_recovery = None;
                    }
                    petunia_ui::RecoveryAction::Discard => {
                        let _ = petunia_core::AutosaveService::discard_recovery(
                            info.main_project_path.as_deref(),
                        );
                        self.core.pending_recovery = None;
                    }
                }
            }
            quit = act.quit;
        });
        g.egui_state
            .handle_platform_output(g.gl_window.window(), full_output.platform_output.clone());
        self.core.dispatch_events();
        self.core.tick_autosave();
        self.core.tick_watcher();

        if let Some((nx, ny)) = self.core.state.ui.pending_pick.take() {
            handle_pick(&mut self.core, nx, ny);
        }
        if self.core.save_requested {
            self.core.save_requested = false;
            petunia_ui::save_project_dialog(&mut self.core.state, false);
        }

        let paint_jobs = g
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        let t_draw = std::time::Instant::now();
        g.gl_renderer.draw(&self.core.state, w, h);
        let t_paint = std::time::Instant::now();
        g.egui_painter.paint_and_update_textures(
            [w, h],
            full_output.pixels_per_point,
            &paint_jobs,
            &mut full_output.textures_delta,
        );
        let t_swap = std::time::Instant::now();
        // modo screenshot: captura o framebuffer e sai (verificação headless)
        if let Some(path) = self.shot_path.clone() {
            self.shot_taken += 1;
            if self.shot_taken >= self.shot_frames {
                capture_screenshot(g, &path, w, h);
                std::process::exit(0);
            }
        }
        g.gl_window.swap();
        let t_end = std::time::Instant::now();
        if std::env::var_os("SIMPLE3D_PHASES").is_some() && self.fps_n < 5 {
            eprintln!(
                "phases ui={:.1}ms draw={:.1}ms paint={:.1}ms swap={:.1}ms",
                (t_draw - t0).as_secs_f32() * 1000.0,
                (t_paint - t_draw).as_secs_f32() * 1000.0,
                (t_swap - t_paint).as_secs_f32() * 1000.0,
                (t_end - t_swap).as_secs_f32() * 1000.0,
            );
        }

        let ms = t0.elapsed().as_secs_f32() * 1000.0;
        self.fps_acc += ms;
        self.fps_n += 1;
        let (v, t) = self.core.state.project.totals();
        self.core.state.render.stats.tris = t;
        self.core.state.render.stats.verts = v;
        self.core.state.render.stats.draws = self
            .core
            .state
            .project
            .assets
            .iter()
            .filter(|a| a.visible)
            .count()
            * 2
            + self
                .core
                .state
                .project
                .refs
                .iter()
                .filter(|r| r.visible)
                .count()
            + 1;
        if self.fps_acc >= 500.0 && self.fps_n > 0 {
            self.core.state.render.stats.fps = 1000.0 * self.fps_n as f32 / self.fps_acc;
            self.core.state.render.stats.frame_ms = self.fps_acc / self.fps_n as f32;
            self.fps_acc = 0.0;
            self.fps_n = 0;
        }
        if std::env::var_os("SIMPLE3D_STATS").is_some() {
            eprintln!(
                "petunia3d: {:.0} fps {:.1} ms",
                self.core.state.render.stats.fps, self.core.state.render.stats.frame_ms
            );
        }

        if quit {
            let proj_path = self
                .core
                .state
                .project
                .project_path
                .as_deref()
                .map(std::path::Path::new);
            petunia_core::AutosaveService::remove_session_lock(proj_path);
            std::process::exit(0);
        }
    }
}

impl ApplicationHandler for GlApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gfx.is_none() {
            if let Err(error) = self.init_gfx(event_loop) {
                self.core.state.set_status(error);
                event_loop.exit();
                return;
            }
            if let Some(g) = self.gfx.as_ref() {
                g.gl_window.window().request_redraw();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(g) = self.gfx.as_mut() else { return };

        // Intercepta a tecla Tab antes do egui para garantir alternância instantânea de modo (P3D-015)
        if let WindowEvent::KeyboardInput {
            event:
                winit::event::KeyEvent {
                    state: ElementState::Pressed,
                    physical_key: PhysicalKey::Code(WKey::Tab),
                    ..
                },
            ..
        } = &event
        {
            self.core.on_key(PhysicalKey::Code(WKey::Tab));
            g.gl_window.window().request_redraw();
            return;
        }

        let is_shortcut_key = matches!(
            &event,
            WindowEvent::KeyboardInput {
                event: winit::event::KeyEvent {
                    state: ElementState::Pressed,
                    physical_key: PhysicalKey::Code(
                        WKey::Tab
                            | WKey::Digit0
                            | WKey::Digit1
                            | WKey::Digit2
                            | WKey::Digit3
                            | WKey::Digit4
                            | WKey::Delete
                            | WKey::Backspace
                            | WKey::KeyD
                            | WKey::KeyX
                            | WKey::KeyA
                            | WKey::KeyG
                            | WKey::KeyR
                            | WKey::KeyS
                            | WKey::KeyZ
                    ),
                    ..
                },
                ..
            }
        );

        let resp = g.egui_state.on_window_event(g.gl_window.window(), &event);
        if resp.repaint {
            g.gl_window.window().request_redraw();
        }
        if resp.consumed && (!is_shortcut_key || g.egui_ctx.egui_wants_keyboard_input()) {
            if matches!(event, WindowEvent::RedrawRequested) {
                self.redraw();
            }
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                self.gfx = None;
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                g.gl_window.resize(size.width, size.height);
                g.gl_window.window().request_redraw();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.core
                    .on_mouse_input(state == ElementState::Pressed, button);
                g.gl_window.window().request_redraw();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let y = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y * 40.0,
                    winit::event::MouseScrollDelta::PixelDelta(p) => p.y as f32,
                };
                let _ = y; // O viewport processa scroll com foco e modalidade.
                g.gl_window.window().request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                // Same policy as WGPU: egui decides whether pointer movement needs
                // repaint, avoiding an unconditional full render on every event.
                self.core.last_mouse = Some((position.x, position.y));
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    self.core.on_key(event.physical_key);
                    g.gl_window.window().request_redraw();
                }
            }
            WindowEvent::ModifiersChanged(m) => {
                let s = m.state();
                self.core
                    .on_modifiers(s.shift_key(), s.control_key(), s.alt_key());
            }
            WindowEvent::RedrawRequested => self.redraw(),
            _ => {}
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.gfx = None;
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _id: winit::event::DeviceId,
        _event: DeviceEvent,
    ) {
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let spin = std::env::var_os("SIMPLE3D_SPIN").is_some() || self.shot_path.is_some();
        if (spin || self.core.state.consume_dirty())
            && let Some(g) = self.gfx.as_ref()
        {
            g.gl_window.window().request_redraw();
        }
    }
}

// ---------------------------------------------------------------------------

/// Captura o framebuffer atual em PNG (glReadPixels + flip vertical).
fn capture_screenshot(g: &GlGfx, path: &str, w: u32, h: u32) {
    use egui_glow::glow::{HasContext as _, PixelPackData};
    let gl = g.gl_window.gl();
    let mut buf = vec![0u8; (w * h * 4) as usize];
    unsafe {
        gl.read_pixels(
            0,
            0,
            w as i32,
            h as i32,
            egui_glow::glow::RGBA,
            egui_glow::glow::UNSIGNED_BYTE,
            PixelPackData::Slice(Some(&mut buf)),
        );
    }
    match image::RgbaImage::from_raw(w, h, buf) {
        Some(img) => {
            let img = image::imageops::flip_vertical(&img);
            match img.save(path) {
                Ok(()) => eprintln!("petunia3d: screenshot {path} ({w}x{h})"),
                Err(e) => eprintln!("petunia3d: screenshot err: {e}"),
            }
        }
        None => eprintln!("petunia3d: screenshot: buffer inválido"),
    }
}

/// Smoke test scriptado (headless): exercita domínio + módulos + projeto
/// sem janela. Falha com `Err` legível. Uso: `petunia3d --smoke-test`.
pub fn smoke_test() -> anyhow::Result<()> {
    use anyhow::{Context as _, ensure};
    use petunia_mesh::Mesh;

    let mut core = Core::new();
    let s = &mut core.state;

    // primitivas
    s.checkpoint("add");
    s.project.add("Cube2", Mesh::cube(2.0));
    s.project.add("Capsule", Mesh::capsule(8, 0.5, 2.0));
    s.project.add("Sphere", Mesh::sphere_low(8, 5, 1.0));
    ensure!(s.project.assets.len() == 4, "assets após add");
    eprintln!("SMOKE primitives ok ({} assets)", s.project.assets.len());

    // seleção + ops com undo/redo
    s.project
        .active_mesh_mut()
        .context("mesh ativa")?
        .select_all();
    s.checkpoint("extrude");
    s.project.active_mesh_mut().unwrap().extrude_selected(0.4);
    s.checkpoint("inset");
    s.project.active_mesh_mut().unwrap().inset_selected(0.2);
    s.checkpoint("subdivide");
    s.project.active_mesh_mut().unwrap().subdivide_selected();
    // Bevel has a stricter supported topology than arbitrary subdivided spheres.
    let mut bevel_fixture = petunia_mesh::Mesh::cube(2.0);
    bevel_fixture.selected_edges.insert((0, 1));
    let (ok, skipped) = bevel_fixture.bevel_selected(0.05);
    ensure!(ok == 1 && skipped == 0, "bevel fixture");
    let (topology, defects) = petunia_mesh::HalfEdgeMesh::from_mesh(&bevel_fixture);
    ensure!(
        defects.is_empty() && topology.is_closed_manifold(),
        "bevel closed manifold"
    );
    s.checkpoint("mirror");
    s.project.active_mesh_mut().unwrap().mirror(0, 0.001);
    s.checkpoint("pushpull");
    s.project.active_mesh_mut().unwrap().push_pull(0.2);
    // novas operações de geometria da Gauntlet
    let m = s.project.active_mesh_mut().unwrap();
    m.slice_plane(glam::Vec3::ZERO, glam::Vec3::Y, true);
    m.flip_normals();
    m.recalculate_normals();
    let edge_for_dissolve = {
        m.faces.iter().find_map(|f| {
            let k = f.verts.len();
            (0..k).find_map(|i| {
                let a = f.verts[i];
                let b = f.verts[(i + 1) % k];
                if m.edge_faces(a, b).len() == 2 {
                    Some((a, b))
                } else {
                    None
                }
            })
        })
    };
    if let Some(edge) = edge_for_dissolve {
        m.selected_edges.insert(edge);
        m.dissolve_selected();
    }
    let rep = m.validate_topology();
    ensure!(
        rep.isolated_vertex_count == 0,
        "topology isolated vertices cleaned"
    );

    // sweep
    let mut sweep_mesh = petunia_mesh::Mesh::default();
    let sweep_prof = [[-0.2, -0.2], [0.2, -0.2], [0.2, 0.2], [-0.2, 0.2]];
    let path = [
        glam::vec3(0.0, 0.0, 0.0),
        glam::vec3(0.0, 1.0, 0.5),
        glam::vec3(1.0, 2.0, 1.0),
    ];
    sweep_mesh
        .sweep(&sweep_prof, &path, true)
        .map_err(|e| anyhow::anyhow!(e))?;
    ensure!(sweep_mesh.tri_count() > 0, "sweep gerou geometria");

    let (v, f) = s.project.totals();
    ensure!(v > 8 && f > 6, "malha cresceu v={v} f={f}");
    eprintln!("SMOKE ops ok (v={v} f={f})");

    ensure!(s.undo(), "undo 1");
    ensure!(s.undo(), "undo 2");
    ensure!(s.redo(), "redo 1");
    eprintln!("SMOKE undo/redo ok");

    // draw profile -> extrude + revolve (nível domínio)
    let square = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];
    let pm = Mesh::from_polygon(&square, 0.5).map_err(|e| anyhow::anyhow!(e))?;
    ensure!(pm.tri_count() >= 2, "polígono triangulou");
    s.project.add("Profile", pm);
    let prof = [[0.0, 0.0], [1.0, 0.0], [1.0, 2.0], [0.0, 2.0]];
    let rm = Mesh::revolve(&prof, 10).map_err(|e| anyhow::anyhow!(e))?;
    ensure!(!rm.faces.is_empty(), "revolve gerou faces");
    s.project.add("Revolved", rm);
    eprintln!("SMOKE profile ok");

    // uv + paint
    s.project.active_mesh_mut().unwrap().project_planar();
    UvModule::move_selected(s, 0.1, 0.0);
    ensure!(
        s.project
            .active_mesh_mut()
            .unwrap()
            .faces
            .iter()
            .all(|f| f.uv.len() == f.verts.len()),
        "uv invariante"
    );
    PaintModule::fill_selection(s);
    PaintModule::ensure_canvas(s);
    PaintModule::canvas_brush(s, 10, 10, false);
    PaintModule::canvas_fill(s);
    eprintln!("SMOKE uv/paint ok");

    // projeto save/load + export
    let dir = std::env::temp_dir();
    let pp = dir.join("petunia_smoke.petunia");
    petunia_project::format::save(&s.project, &pp).context("save")?;
    let q = petunia_project::format::load(&pp).context("load")?;
    ensure!(q.assets.len() == s.project.assets.len(), "roundtrip assets");
    let glb = petunia_project::export::export_gltf(&s.project, &[0]).context("glb")?;
    ensure!(glb.starts_with(b"glTF"), "magic glb");
    let gp = dir.join("petunia_smoke.glb");
    std::fs::write(&gp, &glb).context("write glb")?;
    eprintln!(
        "SMOKE project ok ({} assets, glb {} bytes)",
        q.assets.len(),
        glb.len()
    );
    let _ = std::fs::remove_file(&pp);
    let _ = std::fs::remove_file(&gp);

    // eventos fluem?
    s.emit_mesh_changed();
    core.dispatch_events();
    eprintln!("SMOKE OK");
    Ok(())
}

async fn probe_wgpu() -> bool {
    let mut instance_desc = wgpu::InstanceDescriptor::new_without_display_handle();
    instance_desc.backends = wgpu::Backends::all();
    let instance = wgpu::Instance::new(instance_desc);
    for power in [
        wgpu::PowerPreference::HighPerformance,
        wgpu::PowerPreference::LowPower,
    ] {
        if let Ok(adapter) = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: power,
                compatible_surface: None,
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
        {
            let info = adapter.get_info();
            let name_lower = info.name.to_lowercase();
            // Intel Gen 7 (Ivy Bridge / Bay Trail) lacks working wgpu surface presentation
            // in both Mesa Vulkan (no WSI) and GLES EGL (fails context creation).
            // Skip in probe so Petunia automatically falls back to native OpenGL (glow).
            if name_lower.contains("ivy bridge")
                || name_lower.contains("ivb")
                || name_lower.contains("bay trail")
            {
                continue;
            }
            return true;
        }
    }
    if let Ok(adapter) = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::None,
            compatible_surface: None,
            force_fallback_adapter: true,
            apply_limit_buckets: false,
        })
        .await
    {
        let info = adapter.get_info();
        let name_lower = info.name.to_lowercase();
        if name_lower.contains("ivy bridge")
            || name_lower.contains("ivb")
            || name_lower.contains("bay trail")
        {
            return false;
        }
        return true;
    }
    false
}

/// Ponto de entrada chamado pelo binário.
pub fn run() {
    if let Err(error) = run_event_loop() {
        eprintln!("petunia3d: {error}");
        std::process::exit(1);
    }
}

fn run_event_loop() -> Result<(), String> {
    // Structured diagnostics sink (tracing, P0-10). RUST_LOG controls
    // verbosity without recompiling; production default stays quiet.
    diagnostics::init_diagnostics();
    // Logs: RUST_LOG=wgpu_hal=debug diagnostica backend que falhou.
    // Sem RUST_LOG, os ERRORs internos da sonda wgpu/EGL (normais em
    // máquina sem Vulkan) são silenciados para não assustar.
    {
        let mut b = env_logger::Builder::from_default_env();
        if std::env::var_os("RUST_LOG").is_none() {
            b.filter_module("wgpu_hal", log::LevelFilter::Off);
            b.filter_module("wgpu_core", log::LevelFilter::Off);
            b.filter_module("egui_glow", log::LevelFilter::Off);
        }
        let _ = b.try_init();
    }
    let forced = std::env::var("PETUNIA_BACKEND")
        .or_else(|_| std::env::var("SIMPLE3D_BACKEND"))
        .unwrap_or_default();
    let use_gl = match forced.as_str() {
        "gl" | "opengl" => true,
        "wgpu" => false,
        _ => {
            if forced.is_empty() {
                eprintln!("petunia3d: procurando GPU via wgpu...");
            } else {
                eprintln!("petunia3d: backend '{forced}' desconhecido, detectando...");
            }
            !pollster::block_on(probe_wgpu())
        }
    };
    let event_loop = EventLoop::new().map_err(|error| format!("Event loop: {error}"))?;
    // Render-on-demand: Wait + redraw explícito (sem loop contínuo em idle).
    // SIMPLE3D_SPIN=1 volta ao loop contínuo (diagnóstico/benchmark).
    if std::env::var_os("SIMPLE3D_SPIN").is_some() {
        event_loop.set_control_flow(ControlFlow::Poll);
    } else {
        event_loop.set_control_flow(ControlFlow::Wait);
    }
    if use_gl {
        eprintln!("petunia3d: backend OpenGL puro (glow).");
        let mut app = GlApp::new();
        event_loop
            .run_app(&mut app)
            .map_err(|error| format!("Application event loop: {error}"))?;
        if app.gfx.is_none() {
            return Err(app.core.state.ui.status.clone());
        }
    } else {
        eprintln!("petunia3d: backend wgpu.");
        let mut app = WgpuApp::new();
        event_loop
            .run_app(&mut app)
            .map_err(|error| format!("Application event loop: {error}"))?;
        if app.gfx.is_none() {
            return Err(app.core.state.ui.status.clone());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use petunia_core::SelectMode;

    /// NDC aproximado do centro da face frontal do cubo default.
    /// (câmera default: yaw 0.7/pitch 0.5 — o centro da tela atinge o cubo)

    #[test]
    fn pick_center_selects_face() {
        let mut core = Core::new();
        core.state.select_mode = SelectMode::Face;
        handle_pick(&mut core, 0.0, 0.0);
        let m = core.state.project.active_mesh_mut().unwrap();
        assert!(
            m.selected_face_count() >= 1,
            "clique no centro deve selecionar face"
        );
    }

    #[test]
    fn pick_empty_space_deselects() {
        let mut core = Core::new();
        core.state.project.active_mesh_mut().unwrap().select_all();
        // canto longe do cubo
        handle_pick(&mut core, 0.99, 0.99);
        let m = core.state.project.active_mesh_mut().unwrap();
        assert_eq!(m.selected_face_count(), 0);
        assert_eq!(m.selected_vert_count(), 0);
    }

    #[test]
    fn key_extrude_starts_preview_without_mutating() {
        use winit::keyboard::KeyCode as WKey;
        let mut core = Core::new();
        core.state.project.active_mesh_mut().unwrap().select_all();
        let before = core.state.project.active_mesh_mut().unwrap().faces.len();
        core.on_key(PhysicalKey::Code(WKey::KeyE));
        let after = core.state.project.active_mesh_mut().unwrap().faces.len();
        assert_eq!(after, before, "E apenas inicia interação");
        assert_eq!(
            core.state.pending_modal,
            Some(petunia_core::ModalKind::Extrude)
        );
        assert!(!core.state.project.undo.can_undo());
    }

    #[test]
    fn key_ctrl_z_undoes() {
        let mut core = Core::new();
        core.state.set_edit_mode(EditMode::Edit);
        let mesh = core.state.project.active_mesh_mut().unwrap();
        mesh.deselect_all();
        mesh.faces[0].selected = true;
        mesh.sync_vert_selection_from_faces();
        let before = mesh.faces.len();
        core.state
            .begin_modal(petunia_core::ModalKind::Extrude)
            .unwrap();
        core.state.update_modal(glam::Vec3::ZERO, 0.5).unwrap();
        core.state.commit_modal();
        assert!(core.state.project.active_mesh().unwrap().faces.len() > before);
        core.ctrl_down = true;
        core.on_key(PhysicalKey::Code(WKey::KeyZ));
        assert_eq!(
            core.state.project.active_mesh().unwrap().faces.len(),
            before
        );
    }

    #[test]
    fn draw_profile_click_adds_point_in_front_view() {
        use petunia_core::ViewPreset;
        let mut core = Core::new();
        core.state.camera.set_preset(ViewPreset::Front);
        core.set_tool("draw_profile", None);
        assert!(core.state.profile.points.is_empty());
        handle_pick(&mut core, 0.0, 0.0);
        assert_eq!(core.state.profile.points.len(), 1);
        handle_pick(&mut core, 0.5, 0.5);
        assert_eq!(core.state.profile.points.len(), 2);
    }

    #[test]
    fn test_tab_and_selection_keys() {
        use winit::keyboard::KeyCode as WKey;
        let mut core = Core::new();
        assert_eq!(core.state.edit_mode(), EditMode::Object);

        // Tab -> alterna para Edit
        core.on_key(PhysicalKey::Code(WKey::Tab));
        assert_eq!(core.state.edit_mode(), EditMode::Edit);

        // Tab -> alterna de volta para Object
        core.on_key(PhysicalKey::Code(WKey::Tab));
        assert_eq!(core.state.edit_mode(), EditMode::Object);

        // Tecla 1 -> alterna para Edit + Vertex
        core.on_key(PhysicalKey::Code(WKey::Digit1));
        assert_eq!(core.state.edit_mode(), EditMode::Edit);
        assert_eq!(core.state.select_mode, SelectMode::Vertex);

        // Tecla 2 -> alterna para Edit + Edge
        core.on_key(PhysicalKey::Code(WKey::Digit2));
        assert_eq!(core.state.edit_mode(), EditMode::Edit);
        assert_eq!(core.state.select_mode, SelectMode::Edge);

        // Tecla 3 -> alterna para Edit + Face
        core.on_key(PhysicalKey::Code(WKey::Digit3));
        assert_eq!(core.state.edit_mode(), EditMode::Edit);
        assert_eq!(core.state.select_mode, SelectMode::Face);

        // Tecla 0 -> alterna para Object
        core.on_key(PhysicalKey::Code(WKey::Digit0));
        assert_eq!(core.state.edit_mode(), EditMode::Object);
    }
}

#[cfg(test)]
mod camera_shortcut_tests {
    use super::*;
    use petunia_core::{Projection, ViewPreset};

    #[test]
    fn numpad_and_letter_toggle_preserve_scale_and_cancel_frame_animation() {
        let mut core = Core::new();
        core.state.ui.keybinds = petunia_config::keybinds::Keybinds::defaults();
        let height = core.state.camera.visible_height();
        for key in [WKey::Numpad5, WKey::KeyO] {
            let before = core.state.camera.proj;
            core.state.camera_frame =
                Some((core.state.camera.clone(), core.state.camera.clone(), 0.0));
            core.on_key(PhysicalKey::Code(key));
            assert_ne!(core.state.camera.proj, before);
            assert!((core.state.camera.visible_height() - height).abs() < 1e-5);
            assert!(core.state.camera_frame.is_none());
        }
    }

    #[test]
    fn numpad_with_ctrl_exposes_all_six_orthographic_views() {
        let mut core = Core::new();
        for (key, ctrl, view) in [
            (WKey::Numpad1, false, ViewPreset::Front),
            (WKey::Numpad1, true, ViewPreset::Back),
            (WKey::Numpad3, false, ViewPreset::Right),
            (WKey::Numpad3, true, ViewPreset::Left),
            (WKey::Numpad7, false, ViewPreset::Top),
            (WKey::Numpad7, true, ViewPreset::Bottom),
        ] {
            core.ctrl_down = ctrl;
            core.on_key(PhysicalKey::Code(key));
            assert_eq!(core.state.camera.proj, Projection::Ortho);
            assert_eq!(core.state.camera.view_preset(), Some(view));
        }
    }

    #[test]
    fn test_pick_multiple_objects_in_object_mode() {
        let mut core = Core::new();
        core.state.set_edit_mode(EditMode::Object);
        assert_eq!(core.state.project.assets.len(), 1);
        assert_eq!(core.state.project.active, 0);

        // Cria segundo objeto e posiciona seus vértices longe do primeiro
        let mut second_mesh = petunia_mesh::Mesh::cube(2.0);
        for v in &mut second_mesh.verts {
            v.pos[0] += 5.0; // Desloca para X = 5.0
        }
        core.state
            .project
            .assets
            .push(petunia_project::Asset::new("Cube.001", second_mesh));
        assert_eq!(core.state.project.assets.len(), 2);

        // Clica no centro (0, 0) onde está o primeiro objeto (em [0, 0, 0])
        handle_pick(&mut core, 0.0, 0.0);
        assert_eq!(core.state.project.active, 0);
        assert!(
            core.state.project.assets[0]
                .mesh
                .verts
                .iter()
                .any(|v| v.selected)
        );

        // Move a câmera para enquadrar o segundo objeto em [5, 0, 0]
        core.state.camera.target = glam::Vec3::new(5.0, 0.0, 0.0);

        // Agora o centro da tela (0, 0) aponta para o segundo objeto
        handle_pick(&mut core, 0.0, 0.0);
        assert_eq!(core.state.project.active, 1);
        assert!(
            core.state.project.assets[1]
                .mesh
                .verts
                .iter()
                .any(|v| v.selected)
        );
        assert!(
            !core.state.project.assets[0]
                .mesh
                .verts
                .iter()
                .any(|v| v.selected)
        );

        // Clica no vazio (-0.99, -0.99)
        handle_pick(&mut core, -0.99, -0.99);
        assert!(
            !core.state.project.assets[0]
                .mesh
                .verts
                .iter()
                .any(|v| v.selected)
        );
        assert!(
            !core.state.project.assets[1]
                .mesh
                .verts
                .iter()
                .any(|v| v.selected)
        );
    }

    #[test]
    fn test_delete_and_duplicate_keys_in_object_mode() {
        use winit::keyboard::KeyCode as WKey;
        let mut core = Core::new();
        core.state.set_edit_mode(EditMode::Object);
        core.state.ui.keybinds = petunia_config::keybinds::Keybinds::defaults();
        assert_eq!(core.state.project.assets.len(), 1);

        // Duplicação via Shift+D
        core.shift_down = true;
        core.on_key(PhysicalKey::Code(WKey::KeyD));
        core.shift_down = false;
        assert_eq!(
            core.state.project.assets.len(),
            2,
            "Shift+D deve duplicar o objeto ativo"
        );
        assert_eq!(core.state.project.active, 1);

        // Deleção via Delete
        core.on_key(PhysicalKey::Code(WKey::Delete));
        assert_eq!(
            core.state.project.assets.len(),
            1,
            "Delete deve remover o objeto ativo"
        );
    }

    #[test]
    fn test_tick_watcher_reloads_config_changes() {
        let temp_dir =
            std::env::temp_dir().join(format!("petunia-core-watch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let locales_dir = temp_dir.join("locales");
        std::fs::create_dir_all(&locales_dir).unwrap();

        let mut core = Core::new();
        core.watch_service = crate::watch::WatchService::watch(&[temp_dir.as_path()]).ok();
        assert!(core.watch_service.is_some());

        let test_file = locales_dir.join("test.toml");
        std::fs::write(&test_file, b"test_key = 'test_val'").unwrap();

        let mut saw_status = false;
        for _ in 0..100 {
            std::thread::sleep(std::time::Duration::from_millis(20));
            core.tick_watcher();
            if core.state.ui.status.contains("reloaded translations")
                || core.state.ui.status.contains("file changed")
            {
                saw_status = true;
                break;
            }
        }
        assert!(
            saw_status,
            "tick_watcher should have processed file activity and updated status"
        );
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
