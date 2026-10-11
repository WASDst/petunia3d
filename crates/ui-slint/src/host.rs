//! `DesktopHost` do shell Slint (website/docs/02-application/architecture-decision.md
//! e commands-events-services.md, "DesktopHost").
//!
//! Tudo que é plataforma fica aqui: backend/renderer do Slint, janela, contexto
//! OpenGL do viewport, variáveis de ambiente, relógio do sistema, preferências em
//! disco, marcador de sessão/recuperação e os timers do loop. O estado do editor
//! (`AppState`) é construído e configurado com valores já resolvidos; ele não lê
//! ambiente, relógio nem filesystem por conta própria.

use std::sync::{Arc, Mutex};

use crate::*;

/// Configuração de plataforma lida uma única vez no arranque.
pub struct DesktopHost {
    preferences: petunia_config::UserPreferences,
    /// Segundos desde a época Unix no arranque (marcador de sessão).
    started_at_secs: u64,
    /// `PETUNIA_VIEWPORT_SOFTWARE`: força o viewport por software.
    force_software_viewport: bool,
}

impl DesktopHost {
    /// Lê preferências, relógio e ambiente (o único ponto que faz isso).
    pub fn from_platform() -> Self {
        Self {
            preferences: petunia_config::UserPreferences::load(),
            started_at_secs: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            force_software_viewport: std::env::var_os("PETUNIA_VIEWPORT_SOFTWARE").is_some(),
        }
    }

    /// Aplica as preferências do usuário ao estado do editor. Puro: só copia e
    /// valida valores já carregados (contraste da seleção, espessura, perfis).
    pub fn apply_preferences(state: &mut AppState, preferences: &petunia_config::UserPreferences) {
        state.ui.invert_vertical_drag = preferences.invert_vertical_drag;
        state.ui.colorblind_axes = preferences.colorblind_axes;
        state.ui.reduced_motion = preferences.reduced_motion;
        state.ui.multiselection_measure_tag = preferences.multiselection_measure_tag;
        state.ui.selection_rgb = if selection_color_has_contrast(preferences.selection_rgb) {
            preferences.selection_rgb
        } else {
            petunia_config::UserPreferences::default().selection_rgb
        };
        state.ui.selection_thickness = preferences.selection_thickness.clamp(1.0, 6.0);
        state.ui.model_quick_actions = preferences.model_quick_actions.clone();
        if !preferences.theme_id.is_empty() {
            state.ui.active_theme_id = preferences.theme_id.clone();
        }
        if !preferences.active_keymap_id.is_empty() {
            state.ui.active_keymap_id = preferences.active_keymap_id.clone();
            state.ui.keybinds =
                petunia_config::Keybinds::load_profile(&preferences.active_keymap_id);
        }
    }

    /// Viewport OpenGL no contexto do Slint/FemtoVG, ou software quando não há
    /// OpenGL (ou o usuário pede).
    fn create_viewport(&self, window: &PetuniaSlintShell) -> Box<dyn PetuniaViewport> {
        let shared = viewport_gl::SharedGl::default();
        if !self.force_software_viewport && viewport_gl::install_notifier(window, &shared).is_ok() {
            println!("Viewport backend: OpenGL (Slint/FemtoVG shared context)");
            Box::new(viewport_gl::GlViewport::new(shared, window, 1024, 768))
        } else {
            println!("Viewport backend: software compatibility path");
            Box::new(Software3dViewport::new(1024, 768))
        }
    }

    /// Estado de render inicial (antes do primeiro sync do bridge).
    fn initial_render_state(&self, state: &AppState) -> ViewportRenderState {
        ViewportRenderState {
            shading: state.session.shading,
            xray: state.session.show_xray,
            show_triangulation: state.session.show_triangulation,
            textured: state.session.textured,
            show_wireframe_overlay: state.session.show_wireframe_overlay,
            show_face_orientation: state.session.show_face_orientation,
            show_uv_checker: state.session.show_uv_checker,
            selection_domain: state.selection_domain(),
            xray_opacity: state.session.xray_opacity,
            selection_rgb: state.ui.selection_rgb,
            selection_thickness: state.ui.selection_thickness,
            show_grid: state.session.show_grid,
            hover: state.session.tools.hover,
            boolean_operand: state.session.tools.boolean_operand,
            studio_light_follows_camera: true,
            matcap: self.preferences.viewport_matcap,
            ambient_occlusion: self.preferences.viewport_ambient_occlusion,
            edge_mode: ModelingMode::default().edge_mode(),
            workplane: None,
        }
    }

    /// Marcador de sessão e detecção de encerramento sujo (P3D-002).
    fn start_session(&self, bridge: &mut SlintUiBridge<Box<dyn PetuniaViewport>>) {
        bridge.pending_recovery = petunia_core::AutosaveService::detect_recovery(None);
        let project_name = bridge
            .state
            .project
            .project_path
            .clone()
            .unwrap_or_else(|| "Untitled".to_string());
        let path = bridge.state.project.project_path.clone();
        if let Err(error) = petunia_core::AutosaveService::create_session_lock(
            path.as_deref().map(std::path::Path::new),
            &project_name,
            self.started_at_secs,
        ) {
            eprintln!("petunia3d: falha ao gravar marcador de sessão: {error}");
        }
    }

    /// Cria janela, viewport e bridge e roda o loop até a janela fechar.
    pub fn run(self) -> Result<(), slint::PlatformError> {
        // OpenGL via FemtoVG: o contexto é do Slint. Se não houver, o padrão do
        // Slint decide e o viewport cai para software.
        let _ = slint::BackendSelector::new()
            .backend_name("winit".into())
            .renderer_name("femtovg".into())
            .select();

        println!("Petunia3D - Slint production frontend");
        let window = PetuniaSlintShell::new()?;
        let mut state = AppState::default();
        Self::apply_preferences(&mut state, &self.preferences);

        let mut viewport = self.create_viewport(&window);
        let render_state = self.initial_render_state(&state);
        if let Some(frame) = viewport.render_frame(
            &state.project,
            &state.project.refs,
            &state.session.camera,
            render_state,
        ) {
            window.set_viewport_image(frame);
        }
        window.set_has_gpu_viewport(true);

        let mut startup_bridge = SlintUiBridge::new(state, viewport);
        // Layouts de seção persistem por módulo: restaura no estado runtime e
        // semeia o cache para mutações futuras persistirem tudo.
        startup_bridge.restore_section_layouts(&self.preferences);
        startup_bridge.home_open = self.preferences.show_home_on_start
            && startup_bridge.state.project.project_path.is_none();
        self.start_session(&mut startup_bridge);
        #[allow(clippy::arc_with_non_send_sync)]
        let bridge = Arc::new(Mutex::new(startup_bridge));

        connect_callbacks(&window, Arc::clone(&bridge));
        let vm = bridge
            .lock()
            .expect("Slint bridge mutex poisoned during startup")
            .view_model();
        sync_window_properties(&window, &vm);
        schedule_first_frame(&window, &bridge);
        let timers = start_timers(&window, &bridge);

        println!("Petunia3D window ready");
        let result = window.run();
        drop(timers);
        let path = bridge
            .lock()
            .map(|bridge| bridge.state.project.project_path.clone())
            .unwrap_or(None);
        petunia_core::AutosaveService::remove_session_lock(
            path.as_deref().map(std::path::Path::new),
        );
        result
    }
}

/// Executa o frontend de produção: viewport OpenGL 3.3 no contexto do
/// Slint/FemtoVG, ou o caminho de compatibilidade por software.
pub fn run() -> Result<(), slint::PlatformError> {
    DesktopHost::from_platform().run()
}

type SharedBridge = Arc<Mutex<SlintUiBridge<Box<dyn PetuniaViewport>>>>;

/// O primeiro layout pode ocorrer antes da instalação dos callbacks de resize:
/// usa as dimensões reais antes do primeiro frame interativo.
fn schedule_first_frame(window: &PetuniaSlintShell, bridge: &SharedBridge) {
    let initial_bridge = Arc::clone(bridge);
    let initial_window = window.as_weak();
    slint::Timer::single_shot(std::time::Duration::ZERO, move || {
        if let Some(window) = initial_window.upgrade()
            && let Ok(mut bridge) = initial_bridge.lock()
        {
            bridge.resize_viewport_scaled(
                window.get_viewport_region_width().round().max(1.0) as u32,
                window.get_viewport_region_height().round().max(1.0) as u32,
                window.window().scale_factor(),
            );
            sync_window_properties(&window, &bridge.view_model());
            if let Some(frame) = bridge.render_viewport() {
                window.set_viewport_image(frame);
            }
        }
    });
}

/// Timers do loop: só oferecem oportunidades; o domínio decide se age
/// (autosave, airbrush, vista dividida e reprodução do Animate).
fn start_timers(window: &PetuniaSlintShell, bridge: &SharedBridge) -> Vec<slint::Timer> {
    let repeated = |every: std::time::Duration, tick: Box<dyn FnMut()>| {
        let timer = slint::Timer::default();
        timer.start(slint::TimerMode::Repeated, every, tick);
        timer
    };
    let mut timers = Vec::new();

    // Autosave a cada 30 s; o intervalo real (120 s) e o dirty state são do domínio.
    let autosave = Arc::clone(bridge);
    timers.push(repeated(
        std::time::Duration::from_secs(30),
        Box::new(move || {
            if let Ok(mut bridge) = autosave.lock() {
                bridge.autosave_tick();
            }
        }),
    ));

    // Acúmulo contínuo de tinta para o Airbrush (P3D-056).
    let airbrush = Arc::clone(bridge);
    let airbrush_window = window.as_weak();
    timers.push(repeated(
        std::time::Duration::from_millis(50),
        Box::new(move || {
            if let Ok(mut bridge) = airbrush.lock()
                && bridge.airbrush_tick()
                && let Some(window) = airbrush_window.upgrade()
            {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(canvas_img) = bridge.render_paint_canvas() {
                    window.set_paint_canvas_image(canvas_img);
                }
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }),
    ));

    // Vista dividida: re-renderiza (só se algo mudou) enquanto aberta.
    let split = Arc::clone(bridge);
    let split_window = window.as_weak();
    timers.push(repeated(
        std::time::Duration::from_millis(80),
        Box::new(move || {
            if let Ok(mut bridge) = split.lock()
                && bridge.split.enabled
                && let Some(frame) = bridge.render_split_view()
                && let Some(window) = split_window.upgrade()
            {
                window.set_split_view_image(frame);
            }
        }),
    ));

    // Reprodução do Animate (~30 Hz): só com o workspace ativo e um Motion tocando.
    let animate_bridge = Arc::clone(bridge);
    let animate_window = window.as_weak();
    timers.push(repeated(
        std::time::Duration::from_millis(33),
        Box::new(move || {
            if let Ok(mut bridge) = animate_bridge.lock()
                && bridge.animate_tick()
                && let Some(window) = animate_window.upgrade()
            {
                animate::sync_animate_playhead(&window, &bridge.animate_view_model());
                animate::refresh_posed_viewport(&mut bridge, &window);
            }
        }),
    ));
    timers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_are_applied_and_validated_without_a_window() {
        let mut state = AppState::default();
        let preferences = petunia_config::UserPreferences {
            selection_thickness: 40.0,
            reduced_motion: true,
            ..petunia_config::UserPreferences::default()
        };
        DesktopHost::apply_preferences(&mut state, &preferences);
        assert_eq!(state.ui.selection_thickness, 6.0, "espessura limitada");
        assert!(state.ui.reduced_motion);
        assert!(selection_color_has_contrast(state.ui.selection_rgb));
    }
}
