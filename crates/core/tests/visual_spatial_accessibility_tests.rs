//! Testes sintéticos de percepção espacial, assistência visual e acessibilidade posterior (H33 / VSP-001/002/004, SEL-007).
//!
//! Cobre:
//! 1. VSP-001: Perfis de assistência visual (Essential, Assisted, Analytical); Assisted como default; isolamento de keymap e documento.
//! 2. VSP-002: Modos Clay (Soft, Form, Technical, MatCap) e ancoragem de luz (CameraRelative vs WorldFixed); isolamento de materiais de exportação e Undo.
//! 3. VSP-004: Companion View desacoplada com câmera independente, trigger modes e sem roubo de foco; desacoplamento estrito X-Ray vs Select Through.
//! 4. SEL-007: Universal Gizmo com feedback semântico e acessível (não-dependência de cor, leitura numérica, anúncios para leitores de tela e hit targets >= 24dp).
//! 5. Imunidade de Undo/Redo: nenhuma operação de assistência visual polui o histórico.

use petunia_core::*;
use petunia_mesh::Mesh;
use petunia_project::Asset;

fn setup_test_state() -> AppState {
    let mut state = AppState::default();
    let mesh = Mesh::cube(2.0);
    let asset = Asset::new("Cube", mesh);
    state.project.project.assets.push(asset);
    state.project.project.active = 0;
    state
}

#[test]
fn test_vsp001_visual_assistance_profiles_and_invariants() {
    let mut state = setup_test_state();

    // 1. Default do fluxo iniciante deve ser Assisted (VSP-001)
    assert_eq!(
        state.visual_assistance_profile(),
        VisualAssistanceProfile::Assisted
    );
    assert!(
        state
            .visual_assistance_profile()
            .guidance_overlays_enabled()
    );
    assert!(
        !state
            .visual_assistance_profile()
            .dimension_annotations_enabled()
    );
    assert!(!state.visual_assistance_profile().spatial_frames_enabled());
    assert!(state.visual_assistance_profile().hint_badges_enabled());

    // Captura snapshot de ferramentas, keymap e documento
    let active_tool_before = state.active_tool.clone();
    let revision_before = state.project.project.revision_id();
    let undo_depth_before = state.project.undo.depth();

    // 2. Transição para Essential (viewport limpa para modeladores experientes)
    state.set_visual_assistance_profile(VisualAssistanceProfile::Essential);
    assert_eq!(
        state.visual_assistance_profile(),
        VisualAssistanceProfile::Essential
    );
    assert!(
        !state
            .visual_assistance_profile()
            .guidance_overlays_enabled()
    );
    assert!(
        !state
            .visual_assistance_profile()
            .dimension_annotations_enabled()
    );
    assert!(!state.visual_assistance_profile().spatial_frames_enabled());
    assert!(!state.visual_assistance_profile().hint_badges_enabled());

    // 3. Transição para Analytical (auxílios analíticos de engenharia e inspeção)
    state.set_visual_assistance_profile(VisualAssistanceProfile::Analytical);
    assert_eq!(
        state.visual_assistance_profile(),
        VisualAssistanceProfile::Analytical
    );
    assert!(
        state
            .visual_assistance_profile()
            .guidance_overlays_enabled()
    );
    assert!(
        state
            .visual_assistance_profile()
            .dimension_annotations_enabled()
    );
    assert!(state.visual_assistance_profile().spatial_frames_enabled());
    assert!(!state.visual_assistance_profile().hint_badges_enabled());

    // 4. Invariante VSP-001: nenhum perfil altera atalhos, ferramentas ou documento
    assert_eq!(state.active_tool, active_tool_before);
    assert_eq!(state.project.project.revision_id(), revision_before);
    assert_eq!(state.project.undo.depth(), undo_depth_before);
}

#[test]
fn test_vsp002_clay_shading_modes_and_lighting_anchors() {
    let mut state = setup_test_state();

    // 1. Estado inicial: Clay desativado, iluminação CameraRelative
    assert_eq!(state.clay_mode(), ClayMode::Disabled);
    assert_eq!(state.clay_lighting(), LightingAnchor::CameraRelative);
    assert!(!state.clay_mode().is_enabled());

    let revision_before = state.project.project.revision_id();
    let undo_depth_before = state.project.undo.depth();
    let material_before = state.project.project.assets[0].material_id;

    // 2. Modos Clay de trabalho (VSP-002): Soft, Form, Technical, MatCap
    state.set_clay_mode(ClayMode::Soft);
    assert_eq!(state.clay_mode(), ClayMode::Soft);
    assert!(state.clay_mode().is_enabled());

    state.set_clay_mode(ClayMode::Form);
    assert_eq!(state.clay_mode(), ClayMode::Form);
    assert_eq!(state.clay_mode().label(), "Clay Form");

    state.set_clay_mode(ClayMode::Technical);
    assert_eq!(state.clay_mode(), ClayMode::Technical);

    state.set_clay_mode(ClayMode::MatCap);
    assert_eq!(state.clay_mode(), ClayMode::MatCap);

    // 3. Ancoragem de iluminação de modelagem: WorldFixed vs CameraRelative
    state.set_clay_lighting(LightingAnchor::WorldFixed);
    assert_eq!(state.clay_lighting(), LightingAnchor::WorldFixed);
    assert_eq!(state.clay_lighting().label(), "World Fixed");

    state.set_clay_lighting(LightingAnchor::CameraRelative);
    assert_eq!(state.clay_lighting(), LightingAnchor::CameraRelative);

    // 4. Invariante VSP-002: modos de trabalho são isolados dos materiais do projeto e do Undo
    assert_eq!(state.project.project.revision_id(), revision_before);
    assert_eq!(state.project.undo.depth(), undo_depth_before);

    // Material do ativo permanece intacto
    let asset = &state.project.project.assets[0];
    assert_eq!(asset.material_id, material_before);
}

#[test]
fn test_vsp004_companion_view_independence_and_no_focus_stealing() {
    let mut state = setup_test_state();

    // 1. Estado inicial
    assert!(!state.is_companion_view_open());
    let companion = state.companion_view_state();
    assert!(companion.is_orthographic);
    assert!(!companion.steals_focus);
    assert_eq!(companion.trigger_mode, CompanionViewTrigger::Manual);

    let main_camera_before = state.session.camera.clone();

    // 2. Abertura manual da Companion View
    state.open_companion_view();
    assert!(state.is_companion_view_open());
    assert!(!state.companion_view_state().steals_focus);

    // Alteração na câmera da companion view não toca na câmera principal
    state.session.companion_view.camera_position = [10.0, 20.0, 30.0];
    state.session.companion_view.ortho_scale = 12.0;
    assert_eq!(state.session.camera, main_camera_before);

    // 3. Trigger modes: Manual, Suggested, OptInAuto
    state.set_companion_view_trigger(CompanionViewTrigger::Suggested);
    assert_eq!(
        state.companion_view_state().trigger_mode,
        CompanionViewTrigger::Suggested
    );

    state.set_companion_view_trigger(CompanionViewTrigger::OptInAuto);
    assert_eq!(
        state.companion_view_state().trigger_mode,
        CompanionViewTrigger::OptInAuto
    );

    // 4. Toggle e fechamento
    state.toggle_companion_view();
    assert!(!state.is_companion_view_open());

    state.close_companion_view();
    assert!(!state.is_companion_view_open());
}

#[test]
fn test_vsp004_xray_and_select_through_strict_decoupling() {
    let mut state = setup_test_state();

    // Estado inicial: ambos desativados
    assert!(!state.session.show_xray);
    assert!(!state.session.select_through);

    // Ativar X-Ray NÃO ativa Select Through
    state.set_show_xray(true);
    assert!(state.session.show_xray);
    assert!(!state.session.select_through);

    // Ativar Select Through NÃO altera X-Ray
    state.set_select_through(true);
    assert!(state.session.show_xray);
    assert!(state.session.select_through);

    // Desativar X-Ray NÃO desativa Select Through
    state.set_show_xray(false);
    assert!(!state.session.show_xray);
    assert!(state.session.select_through);

    // Desativar Select Through
    state.set_select_through(false);
    assert!(!state.session.show_xray);
    assert!(!state.session.select_through);
}

#[test]
fn test_sel007_accessible_gizmo_feedback_idle() {
    let state = setup_test_state();

    let feedback = state.gizmo_accessibility_feedback();

    assert_eq!(feedback.mode, "Idle");
    assert_eq!(feedback.active_axis, "None");
    assert_eq!(feedback.frame_orientation, "Global");
    assert_eq!(feedback.numeric_readout, "0.000");
    assert!(feedback.high_contrast_compliant);
    assert!(feedback.minimum_hit_target_dp >= 24.0);
    assert!(
        feedback
            .screen_reader_announcement
            .contains("Universal Gizmo pronto")
    );
}

#[test]
fn test_sel007_accessible_gizmo_feedback_modal_translation_and_axes() {
    let mut state = setup_test_state();

    // Inicia gesto modal de Move (Translate)
    let res = state.begin_modal(petunia_core::ModalKind::Move);
    assert!(res.is_ok(), "iniciar modal Move");

    // Por padrão o modal é livre (Screen / Free)
    let feedback_free = state.gizmo_accessibility_feedback();
    assert_eq!(feedback_free.mode, "Move");
    assert_eq!(feedback_free.active_axis, "Screen / Free");
    assert!(feedback_free.high_contrast_compliant);
    assert!(feedback_free.minimum_hit_target_dp >= 24.0);
    assert!(
        feedback_free
            .screen_reader_announcement
            .contains("Gesto ativo: Move")
    );

    // Restringe ao eixo X (ModalConstraint::Axis(0))
    if let Some(op) = state.modal.as_mut() {
        op.constraint = ModalConstraint::Axis(0);
        op.value = 1.75;
        op.components = glam::Vec3::new(1.75, 0.0, 0.0);
    }
    let feedback_x = state.gizmo_accessibility_feedback();
    assert_eq!(feedback_x.active_axis, "X");
    assert!(feedback_x.numeric_readout.contains("+1.750 m"));
    assert!(feedback_x.delta_text.contains("dx: +1.750"));
    assert!(feedback_x.screen_reader_announcement.contains("eixo X"));
    assert!(feedback_x.screen_reader_announcement.contains("+1.750"));

    // Restringe ao plano YZ (ModalConstraint::Plane(0))
    if let Some(op) = state.modal.as_mut() {
        op.constraint = ModalConstraint::Plane(0);
        op.value = 2.50;
        op.components = glam::Vec3::new(0.0, 1.5, 2.0);
    }
    let feedback_yz = state.gizmo_accessibility_feedback();
    assert_eq!(feedback_yz.active_axis, "YZ Plane");
    assert!(feedback_yz.screen_reader_announcement.contains("plano YZ"));

    // Cancela o modal limpa o estado
    state.cancel_modal();
    assert!(state.modal.is_none());
}

#[test]
fn test_sel007_accessible_gizmo_feedback_rotation_and_scale() {
    let mut state = setup_test_state();

    // 1. Rotação em torno do eixo Z
    state
        .begin_modal(petunia_core::ModalKind::Rotate)
        .expect("begin rotate");
    if let Some(op) = state.modal.as_mut() {
        op.constraint = ModalConstraint::Axis(2);
        op.value = 45.0;
    }
    let feedback_rot = state.gizmo_accessibility_feedback();
    assert_eq!(feedback_rot.mode, "Rotate");
    assert_eq!(feedback_rot.active_axis, "Z");
    assert!(feedback_rot.numeric_readout.contains("45.0°"));
    assert!(
        feedback_rot
            .screen_reader_announcement
            .contains("rotação de 45.0 graus")
    );
    state.cancel_modal();

    // 2. Escala
    state
        .begin_modal(petunia_core::ModalKind::Scale)
        .expect("begin scale");
    if let Some(op) = state.modal.as_mut() {
        op.constraint = ModalConstraint::Axis(1);
        op.value = 2.0;
        op.components = glam::Vec3::new(1.0, 2.0, 1.0);
    }
    let feedback_scale = state.gizmo_accessibility_feedback();
    assert_eq!(feedback_scale.mode, "Scale");
    assert_eq!(feedback_scale.active_axis, "Y");
    assert!(feedback_scale.numeric_readout.contains("2.000x"));
    assert!(
        feedback_scale
            .screen_reader_announcement
            .contains("fator de escala 2.000")
    );
    state.cancel_modal();
}

#[test]
fn test_vsp_and_accessibility_zero_undo_pollution() {
    let mut state = setup_test_state();

    let initial_undo = state.project.undo.depth();

    // Executa sequência completa de ajustes visuais e de acessibilidade
    state.set_visual_assistance_profile(VisualAssistanceProfile::Analytical);
    state.set_clay_mode(ClayMode::Form);
    state.set_clay_lighting(LightingAnchor::WorldFixed);
    state.open_companion_view();
    state.set_companion_view_trigger(CompanionViewTrigger::OptInAuto);
    state.toggle_companion_view();
    state.set_show_xray(true);
    state.set_select_through(true);
    let _ = state.gizmo_accessibility_feedback();

    // Invariante: absolutamente nenhuma entrada de Undo foi gerada
    assert_eq!(state.project.undo.depth(), initial_undo);
    assert!(!state.project.undo.can_undo());
}
