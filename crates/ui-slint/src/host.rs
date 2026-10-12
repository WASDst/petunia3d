//! `DesktopHost` do shell Slint (website/docs/02-application/architecture-decision.md
//! e commands-events-services.md, "DesktopHost").
//!
//! Tudo que é plataforma fica aqui: backend/renderer do Slint, janela, contexto
//! OpenGL do viewport, variáveis de ambiente, relógio do sistema, preferências em
//! disco, marcador de sessão/recuperação e os timers do loop. O estado do editor
//! é possuído pela Application e configurado com valores já resolvidos; ele não lê
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
        // OpenGL via FemtoVG: o contexto é do Slint. Sem pedido explícito o
        // Slint cria OpenGL ES (2.0 pedido; Mesa entrega ES 3.2) e o renderer,
        // que exige OpenGL 3.3 Core, caía para o viewport de CPU — a causa da
        // lentidão observada em 2026-10-11. Pede-se OpenGL de desktop 3.3
        // (perfil Core é o padrão de EGL/WGL para ≥ 3.2); se o driver recusar,
        // o Slint usa os atributos padrão e o viewport cai para software.
        let strict = slint::BackendSelector::new()
            .backend_name("winit".into())
            .renderer_name("femtovg".into())
            .require_opengl_with_version(3, 3)
            .select();
        if let Err(error) = strict {
            eprintln!(
                "petunia3d: OpenGL 3.3 indisponível no seletor ({error}); usando o padrão do Slint"
            );
            let _ = slint::BackendSelector::new()
                .backend_name("winit".into())
                .renderer_name("femtovg".into())
                .select();
        }

        println!("Petunia3D - Slint production frontend");
        let window = PetuniaSlintShell::new()?;
        let mut application = petunia_application::Application {
            // Catálogo configurado em disco é responsabilidade do host; a
            // Application recém-construída só contém o catálogo nativo puro.
            tools: petunia_module_model::ToolRegistry::with_defaults(),
            ..Default::default()
        };
        // O construtor só usa as traduções embutidas; o host decide ler
        // `assets/locales` do disco (sobreposição sem recompilar).
        let lang = application.state.ui.i18n.lang().to_string();
        application.state.ui.i18n.set_lang(&lang);
        Self::apply_preferences(&mut application.state, &self.preferences);

        let mut viewport = self.create_viewport(&window);
        let render_state = self.initial_render_state(&application.state);
        if let Some(frame) = viewport.render_frame(
            &application.state.project,
            &application.state.project.refs,
            &application.state.session.camera,
            render_state,
        ) {
            window.set_viewport_image(frame);
        }
        window.set_has_gpu_viewport(true);

        let mut startup_bridge = SlintUiBridge::from_application(application, viewport);
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
        schedule_startup_commands(&window, &bridge);
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

/// QA visual sem injetar entrada: `PETUNIA_STARTUP_COMMANDS` executa, depois
/// do primeiro quadro, ids separados por vírgula — comandos (`window.asset_library`),
/// `workspace:PAINT|MODEL|UV`, `ui:add_menu`, `ui:no_home`, `ui:no_recovery` e
/// `ui:stroke` (traço de pincel cronometrado).
/// Sem a variável, nada muda.
fn schedule_startup_commands(window: &PetuniaSlintShell, bridge: &SharedBridge) {
    let Some(list) = std::env::var_os("PETUNIA_STARTUP_COMMANDS") else {
        return;
    };
    let list = list.to_string_lossy().into_owned();
    let startup_bridge = Arc::clone(bridge);
    let startup_window = window.as_weak();
    slint::Timer::single_shot(std::time::Duration::from_millis(400), move || {
        let Some(window) = startup_window.upgrade() else {
            return;
        };
        let Ok(mut bridge) = startup_bridge.lock() else {
            return;
        };
        for id in list.split(',').map(str::trim).filter(|id| !id.is_empty()) {
            match id {
                "ui:add_menu" => window.set_add_menu_open(true),
                // Executado depois, sem o lock (os callbacks o pegam).
                "ui:stroke" => {}
                "ui:no_home" => bridge.home_open = false,
                // Só oculta o aviso; os snapshots ficam no disco.
                "ui:no_recovery" => bridge.pending_recovery = None,
                _ if id.starts_with("workspace:") => {
                    let workspace = match &id["workspace:".len()..] {
                        "PAINT" => Some(petunia_core::Workspace::Paint),
                        "UV" => Some(petunia_core::Workspace::Uv),
                        "MODEL" => Some(petunia_core::Workspace::Model),
                        _ => None,
                    };
                    if let Some(workspace) = workspace {
                        bridge.apply(crate::UiIntent::SetWorkspace(workspace));
                    }
                }
                _ => match crate::commands::CommandId::from_id_str(id) {
                    Some(command) => bridge.execute_command(command),
                    None => {
                        if let Err(error) = bridge.execute_core_command(id) {
                            eprintln!("petunia3d: PETUNIA_STARTUP_COMMANDS {id}: {error}");
                        }
                    }
                },
            }
        }
        sync_window_properties(&window, &bridge.view_model());
        if let Some(frame) = bridge.render_viewport() {
            window.set_viewport_image(frame);
        }
        drop(bridge);
        if list.split(',').any(|id| id.trim() == "ui:stroke") {
            // Depois de o compositor dar o tamanho final à janela.
            let stroke_window = window.as_weak();
            let stroke_bridge = Arc::clone(&startup_bridge);
            slint::Timer::single_shot(std::time::Duration::from_millis(2500), move || {
                if let Some(window) = stroke_window.upgrade() {
                    run_startup_stroke(&window, &stroke_bridge);
                }
            });
        }
    });
}

/// Traço horizontal no centro da viewport pelos mesmos callbacks do
/// ponteiro (begin → updates → end), com o tempo médio por atualização e a
/// prova do resultado: o raio do meio acerta a malha? quantos pixels mudaram?
fn run_startup_stroke(window: &PetuniaSlintShell, bridge: &SharedBridge) {
    let Some([width, height]) = bridge.lock().ok().map(|bridge| bridge.viewport_size) else {
        return;
    };
    eprintln!(
        "petunia3d: QA viewport bridge={width}x{height} região={}x{}",
        window.get_viewport_region_width(),
        window.get_viewport_region_height()
    );
    let texture = |bridge: &SharedBridge| {
        let bridge = bridge.lock().ok()?;
        let asset = bridge
            .state
            .project
            .assets
            .get(bridge.state.project.active)?;
        asset.texture.as_ref().map(|canvas| canvas.pixels.to_vec())
    };
    let before = texture(bridge);
    let (y, from, to) = (height * 0.5, width * 0.3, width * 0.7);
    if let Ok(bridge) = bridge.lock() {
        let (origin, direction) = bridge.state.session.camera.ray(0.0, 0.0);
        eprintln!(
            "petunia3d: QA raio central: pick={:?} ferramenta={} viewport={width}x{height}",
            bridge.paint_pick(origin, direction).map(|(face, _)| face),
            bridge.state.session.tools.active_tool,
        );
    }
    let steps = 40;
    window.invoke_viewport_paint_begin(from, y, false, false, false, false);
    let started = std::time::Instant::now();
    for step in 1..=steps {
        let x = from + (to - from) * step as f32 / steps as f32;
        window.invoke_viewport_paint_update(x, y + (step as f32 * 0.3).sin() * 20.0);
    }
    let per_update = started.elapsed().as_secs_f64() * 1000.0 / f64::from(steps);
    window.invoke_viewport_paint_end(to, y);
    eprintln!("petunia3d: traço de QA {steps} atualizações, {per_update:.2} ms/atualização");
    let after = texture(bridge);
    let changed = match (&before, &after) {
        (Some(a), Some(b)) if a.len() == b.len() => {
            a.chunks(4).zip(b.chunks(4)).filter(|(p, q)| p != q).count()
        }
        // Sem textura antes: conta o que difere do fundo (primeiro pixel).
        (None, Some(b)) if b.len() >= 4 => b.chunks(4).filter(|p| *p != &b[..4]).count(),
        _ => usize::MAX,
    };
    eprintln!(
        "petunia3d: QA textura antes={:?} depois={:?} pixels alterados={changed}",
        before.as_ref().map(Vec::len),
        after.as_ref().map(Vec::len)
    );
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
