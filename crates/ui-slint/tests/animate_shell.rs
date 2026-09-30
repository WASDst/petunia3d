//! Workspace Animate no shell declarativo real (cap. 45 F2): cliques, arrasto e
//! teclado sobre os controles Slint de verdade, ligados ao bridge e ao domínio.
//! Cobre o que o bridge isolado não vê: hit testing, foco, acessibilidade e a
//! atualização de modelos no lugar.
//!
//! Sem captura de tela: estes testes garantem comportamento e estrutura
//! acessível, não estética. A aceitação visual do Animate continua manual.

use std::sync::{Arc, Mutex};

use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use petunia_core::{AppState, RigPresetKind};
use petunia_project::MotionGenerator;
use petunia_ui_slint::animate::{AnimateIntent, sync_animate_playhead, sync_animate_properties};
use petunia_ui_slint::{PetuniaSlintShell, PlaceholderViewport, SlintUiBridge, UiIntent};
use slint::platform::PointerEventButton;
use slint::{ComponentHandle, LogicalPosition, LogicalSize, Model};

type Bridge = SlintUiBridge<PlaceholderViewport>;

struct Rig {
    shell: PetuniaSlintShell,
    bridge: Arc<Mutex<Bridge>>,
}

impl Rig {
    fn new() -> Self {
        i_slint_backend_testing::init_no_event_loop();
        let shell = PetuniaSlintShell::new().expect("Slint shell");
        shell.window().set_size(LogicalSize::new(1280.0, 800.0));
        shell.show().expect("headless window");
        let bridge = Arc::new(Mutex::new(SlintUiBridge::new(
            AppState::default(),
            PlaceholderViewport::default(),
        )));
        // Mesma ligação de `connect_animate_callbacks`, sem o sync do shell
        // inteiro (que reescreveria `active-workspace` sem a feature).
        let weak = shell.as_weak();
        let handle = Arc::clone(&bridge);
        shell.on_animate_action(move |action, arg, value| {
            let Some(intent) = AnimateIntent::parse(action.as_str(), arg.as_str(), value) else {
                return;
            };
            let gesture = matches!(
                intent,
                AnimateIntent::ParamPreview { .. } | AnimateIntent::Seek(_)
            );
            let mut bridge = handle.lock().unwrap();
            bridge.apply(UiIntent::Animate(intent));
            let shell = weak.unwrap();
            let vm = bridge.animate_view_model();
            if gesture {
                sync_animate_playhead(&shell, &vm);
            } else {
                sync_animate_properties(&shell, &vm);
            }
        });
        let rig = Self { shell, bridge };
        rig.shell.set_active_workspace("ANIMATE".into());
        rig.shell.set_animate_available(true);
        rig.sync();
        rig
    }

    fn sync(&self) {
        let vm = self.bridge.lock().unwrap().animate_view_model();
        sync_animate_properties(&self.shell, &vm);
        // Deixa as transições do flyout terminarem antes de medir posições.
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(400));
    }

    fn intent(&self, intent: AnimateIntent) {
        self.bridge.lock().unwrap().apply(UiIntent::Animate(intent));
        self.sync();
    }

    fn depth(&self) -> usize {
        self.bridge.lock().unwrap().state.project.undo.depth().0
    }

    fn motions(&self) -> usize {
        self.bridge
            .lock()
            .unwrap()
            .state
            .project
            .project
            .motions
            .len()
    }

    fn button(&self, label: &str) -> ElementHandle {
        ElementHandle::find_by_accessible_label(&self.shell, label)
            .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
            .unwrap_or_else(|| panic!("botão acessível '{label}' não encontrado"))
    }

    fn slider(&self, label: &str) -> ElementHandle {
        ElementHandle::find_by_accessible_label(&self.shell, label)
            .find(|e| e.accessible_role() == Some(AccessibleRole::Slider))
            .unwrap_or_else(|| panic!("slider acessível '{label}' não encontrado"))
    }

    fn param(&self, key: &str) -> f32 {
        let model = self.shell.get_animate_params();
        (0..model.row_count())
            .filter_map(|i| model.row_data(i))
            .find(|p| p.key == key)
            .unwrap_or_else(|| panic!("parâmetro {key} ausente"))
            .value
    }
}

/// Endereço do modelo concreto por trás de um `ModelRc` (identidade).
fn model_addr<T: 'static>(model: &slint::ModelRc<T>) -> usize {
    model.as_any() as *const dyn std::any::Any as *const () as usize
}

fn center(e: &ElementHandle) -> LogicalPosition {
    let (p, s) = (e.absolute_position(), e.size());
    LogicalPosition::new(p.x + s.width * 0.5, p.y + s.height * 0.5)
}

/// Ponto dentro do trilho (a faixa de baixo do slider) a `fraction` da largura.
fn track_point(e: &ElementHandle, fraction: f32) -> LogicalPosition {
    let (p, s) = (e.absolute_position(), e.size());
    LogicalPosition::new(p.x + s.width * fraction, p.y + s.height - 11.0)
}

#[test]
fn the_animate_pill_only_exists_when_the_workspace_is_available() {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    shell.window().set_size(LogicalSize::new(1280.0, 800.0));
    shell.show().unwrap();
    let mut state = AppState::default();
    state.ui.i18n.set_lang("en");
    let bridge: Bridge = SlintUiBridge::new(state, PlaceholderViewport::default());
    sync_animate_properties(&shell, &bridge.animate_view_model());

    let pill = |shell: &PetuniaSlintShell| {
        ElementHandle::find_by_accessible_label(shell, "ANIMATE")
            .find(|e| e.accessible_role() == Some(AccessibleRole::Tab))
    };
    shell.set_animate_available(false);
    assert!(
        pill(&shell).is_none(),
        "sem o workspace não há pill (ADR 006)"
    );
    shell.set_animate_available(true);
    let pill = pill(&shell).expect("pill ANIMATE acessível");
    let clicked = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
    let sink = std::rc::Rc::clone(&clicked);
    shell.on_workspace_changed(move |ws| *sink.borrow_mut() = ws.to_string());
    pill.mock_single_click(PointerEventButton::Left);
    assert_eq!(clicked.borrow().as_str(), "ANIMATE");
}

#[test]
fn picking_a_motion_creates_it_and_the_panel_follows() {
    let rig = Rig::new();
    rig.intent(AnimateIntent::AddCreature(RigPresetKind::Humanoid));
    assert_eq!(rig.motions(), 0);
    assert!(!rig.shell.get_animate_has_motion());

    // O mesmo Motion aparece no cartão do Inspector e no botão da bandeja.
    let labelled: Vec<_> = ElementHandle::find_by_accessible_label(&rig.shell, "Walk / Run")
        .filter(|e| e.accessible_role() == Some(AccessibleRole::Button))
        .collect();
    assert_eq!(labelled.len(), 2, "cartão do painel + botão da bandeja");

    labelled[0].mock_single_click(PointerEventButton::Left);
    assert_eq!(rig.motions(), 1);
    assert!(rig.shell.get_animate_has_motion());
    assert!(rig.shell.get_animate_playing(), "um clique e já se move");
    assert!(
        rig.shell.get_animate_params().row_count() >= 6,
        "os sliders universais aparecem"
    );
    assert!(!rig.shell.get_animate_bone_commands().is_empty());
}

#[test]
fn motions_the_rig_cannot_do_are_disabled_and_explain_why() {
    let rig = Rig::new();
    rig.intent(AnimateIntent::AddCreature(RigPresetKind::Serpent));
    let walk = ElementHandle::find_by_accessible_label(&rig.shell, "Walk / Run")
        .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
        .expect("Walk / Run continua visível");
    assert_eq!(walk.accessible_enabled(), Some(false));
    assert_eq!(
        walk.accessible_description().as_deref(),
        Some("Needs at least two legs")
    );
    walk.mock_single_click(PointerEventButton::Left);
    assert_eq!(rig.motions(), 0, "clicar no que não serve não faz nada");

    let slither = rig.button("Slither / Swim");
    assert_eq!(slither.accessible_enabled(), Some(true));
}

#[test]
fn dragging_a_slider_with_the_pointer_commits_exactly_one_undo_step() {
    let rig = Rig::new();
    rig.intent(AnimateIntent::AddCreature(RigPresetKind::Humanoid));
    rig.intent(AnimateIntent::AddMotion(MotionGenerator::BipedCycle));
    let depth = rig.depth();

    let energy = rig.slider("Energy");
    assert_eq!(energy.accessible_value_minimum(), Some(0.0));
    assert_eq!(energy.accessible_value_maximum(), Some(2.0));
    let model_before = rig.shell.get_animate_params();

    energy.mock_drag(track_point(&energy, 0.85), PointerEventButton::Left);
    // Soltar o mouse confirma.
    assert_eq!(
        rig.depth(),
        depth + 1,
        "o arrasto inteiro é um passo de Undo"
    );
    let value = rig.param("energy");
    assert!(value > 1.3, "o valor acompanhou o ponteiro: {value}");

    // O mesmo controle continua vivo: o modelo foi atualizado no lugar.
    assert_eq!(
        model_addr(&model_before),
        model_addr(&rig.shell.get_animate_params()),
        "os sliders não são recriados a cada ajuste"
    );

    assert!(rig.bridge.lock().unwrap().state.undo());
    rig.sync();
    assert!(
        (rig.param("energy") - 1.0).abs() < 1e-4,
        "um Undo volta ao padrão"
    );
}

#[test]
fn keyboard_steps_a_slider_and_confirms_each_step() {
    let rig = Rig::new();
    rig.intent(AnimateIntent::AddCreature(RigPresetKind::Humanoid));
    rig.intent(AnimateIntent::AddMotion(MotionGenerator::BipedCycle));
    let depth = rig.depth();
    let before = rig.param("stride");

    rig.slider("Stride").invoke_accessible_increment_action();
    rig.sync();
    assert_eq!(
        rig.depth(),
        depth + 1,
        "cada passo do teclado confirma na hora"
    );
    assert!(rig.param("stride") > before);

    rig.slider("Stride").invoke_accessible_decrement_action();
    rig.sync();
    assert_eq!(rig.depth(), depth + 2);
    assert!((rig.param("stride") - before).abs() < 1e-3);
}

#[test]
fn transport_plays_pauses_scrubs_and_applies_now() {
    let rig = Rig::new();
    rig.intent(AnimateIntent::AddCreature(RigPresetKind::Quadruped));
    rig.intent(AnimateIntent::AddMotion(MotionGenerator::Gait));
    assert!(rig.shell.get_animate_playing());

    // Pausar pelo botão real (o rótulo acompanha o estado).
    rig.button("Pause")
        .mock_single_click(PointerEventButton::Left);
    assert!(!rig.shell.get_animate_playing());
    let play = rig.button("Play");
    play.mock_single_click(PointerEventButton::Left);
    assert!(rig.shell.get_animate_playing());

    // Scrub: arrastar o playhead pausa e posiciona.
    let playhead = rig.slider("Playhead");
    playhead.mock_drag(track_point(&playhead, 0.5), PointerEventButton::Left);
    assert!(!rig.shell.get_animate_playing(), "scrub pausa");
    let progress = rig.shell.get_animate_progress();
    assert!((0.3..=0.7).contains(&progress), "playhead ~50%: {progress}");

    // Apply Now: converte em clipe editável e remove o Motion vivo.
    let clips = rig
        .bridge
        .lock()
        .unwrap()
        .state
        .project
        .project
        .animations
        .len();
    rig.button("Bake a copy")
        .mock_single_click(PointerEventButton::Left);
    let after_copy = rig
        .bridge
        .lock()
        .unwrap()
        .state
        .project
        .project
        .animations
        .len();
    let status = rig.bridge.lock().unwrap().state.ui.status.clone();
    assert_eq!(after_copy, clips + 1, "status: {status}");
    assert_eq!(rig.motions(), 1, "a cópia mantém o Motion vivo");
    rig.button("Apply Now")
        .mock_single_click(PointerEventButton::Left);
    assert_eq!(rig.motions(), 0, "Apply Now converte");
    assert!(!rig.shell.get_animate_has_motion());
}

#[test]
fn advanced_controls_appear_on_demand_and_choices_are_immediate() {
    let rig = Rig::new();
    rig.intent(AnimateIntent::AddCreature(RigPresetKind::Quadruped));
    rig.intent(AnimateIntent::AddMotion(MotionGenerator::Gait));
    assert!(
        ElementHandle::find_by_accessible_label(&rig.shell, "Arm swing")
            .next()
            .is_none()
            && ElementHandle::find_by_accessible_label(&rig.shell, "Step height")
                .next()
                .is_none(),
        "Advanced começa recolhido"
    );
    rig.button("Advanced")
        .mock_single_click(PointerEventButton::Left);
    assert!(rig.shell.get_animate_show_advanced());
    rig.slider("Step height");
    rig.button("Wave")
        .mock_single_click(PointerEventButton::Left);
    assert_eq!(rig.param("pattern"), 3.0, "a escolha é imediata");
}

#[test]
fn styles_are_chips_that_only_fill_parameters() {
    let rig = Rig::new();
    rig.intent(AnimateIntent::AddCreature(RigPresetKind::Humanoid));
    assert!(
        ElementHandle::find_by_accessible_label(&rig.shell, "Heavy")
            .next()
            .is_none(),
        "Styles só existem com um Motion escolhido"
    );
    rig.intent(AnimateIntent::AddMotion(MotionGenerator::BipedCycle));
    let heavy = rig.button("Heavy");
    assert_eq!(heavy.accessible_enabled(), Some(true));
    let weight_before = rig.param("weight");
    heavy.mock_single_click(PointerEventButton::Left);
    assert!(rig.param("weight") > weight_before, "Heavy aumenta o peso");
    assert_eq!(rig.button("Heavy").accessible_checked(), Some(true));
}

#[test]
fn the_universal_sliders_fit_the_panel_without_scrolling() {
    // O objetivo do Animate é "poucos sliders, sem procurar": na janela mínima
    // do baseline (cap. 36) os seis controles universais precisam estar à vista.
    let rig = Rig::new();
    rig.intent(AnimateIntent::AddCreature(RigPresetKind::Humanoid));
    rig.intent(AnimateIntent::AddMotion(MotionGenerator::BipedCycle));
    for label in ["Speed", "Energy", "Weight", "Stride", "Lean", "Smoothness"] {
        let slider = rig.slider(label);
        let bottom = slider.absolute_position().y + slider.size().height;
        assert!(bottom <= 800.0, "{label} fora da janela (y={bottom})");
    }
}

#[test]
fn controls_are_reachable_by_keyboard() {
    let rig = Rig::new();
    rig.intent(AnimateIntent::AddCreature(RigPresetKind::Humanoid));
    rig.intent(AnimateIntent::AddMotion(MotionGenerator::BipedCycle));
    // Todo controle interativo é botão/slider acessível (focável) — nenhum é
    // só uma área de toque muda. Enter/Espaço ativam o que tem foco.
    for label in [
        "Walk / Run",
        "Heavy",
        "Advanced",
        "Pause",
        "Apply Now",
        "Bake a copy",
    ] {
        let e = rig.button(label);
        assert_eq!(e.accessible_enabled(), Some(true), "{label}");
        assert!(e.size().width > 0.0 && e.size().height > 0.0, "{label}");
    }
    for label in ["Energy", "Playhead"] {
        assert!(rig.slider(label).accessible_value().is_some(), "{label}");
    }
    let apply = rig.button("Apply Now");
    let c = center(&apply);
    assert!(c.x > 0.0 && c.y > 0.0);
}

/// Caminho de produção completo: workspace real, callback real
/// (`connect_animate_callbacks`) e o sync do shell inteiro.
#[cfg(feature = "animation-workspace")]
#[test]
fn the_production_callback_path_keeps_the_workspace_and_shows_the_pill() {
    use petunia_core::Workspace;
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    shell.window().set_size(LogicalSize::new(1280.0, 800.0));
    shell.show().unwrap();
    let mut bridge: Bridge =
        SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Animate));
    bridge.apply(UiIntent::Animate(AnimateIntent::AddCreature(
        RigPresetKind::Humanoid,
    )));
    let bridge = Arc::new(Mutex::new(bridge));
    petunia_ui_slint::animate::connect_animate_callbacks(&shell, Arc::clone(&bridge));
    shell.set_active_workspace("ANIMATE".into());
    sync_animate_properties(&shell, &bridge.lock().unwrap().animate_view_model());
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(400));
    assert!(shell.get_animate_available(), "com a feature a pill existe");

    let walk = ElementHandle::find_by_accessible_label(&shell, "Walk / Run")
        .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
        .expect("Motion Walk / Run");
    walk.mock_single_click(PointerEventButton::Left);
    assert_eq!(
        bridge.lock().unwrap().state.project.project.motions.len(),
        1
    );
    assert_eq!(
        shell.get_active_workspace(),
        "ANIMATE",
        "o sync completo do shell mantém o workspace"
    );
    assert!(shell.get_animate_has_motion() && shell.get_animate_playing());
}
