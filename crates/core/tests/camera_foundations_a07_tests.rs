//! Testes de integração e regressão para Camera Foundations (A07).
//!
//! Cobre:
//! - Ownership da câmera em EditorSession/Application, desacoplada do DocumentSession/ProjectState
//! - Presets existentes (cardinais, isométricos e perspectiva)
//! - Preset pseudo-isométrico 2:1 clássico (26,565° ortográfico com preservação de framing)
//! - Despacho de comandos SetViewPresetCmd sem poluição de Undo ou dirty document
//! - Navegação contextual (NavigationAnchor::ViewTarget vs Cursor3D, pan/frame sem mutação de cursor)
//! - Rejeição de valores não-finitos (NaN/Inf) e estabilidade de matrizes

use glam::Vec3;
use petunia_core::camera::{Camera, NavigationAnchor, Projection, ViewPreset};
use petunia_core::command::SetViewPresetCmd;
use petunia_core::state::{AppState, ViewportSession};
use petunia_project::ProjectChanges;

#[test]
fn test_a07_camera_ownership_in_editor_session_isolated_from_document() {
    let mut state = AppState::default();
    let initial_rev = state.project.project.revision_id();
    let initial_undo = state.project.undo.depth();

    // Câmera reside na EditorSession e na ViewportSession projetada
    assert_eq!(state.session.camera.proj, Projection::Perspective);
    let vp: ViewportSession = state.session.viewport_session();
    assert_eq!(vp.camera.proj, Projection::Perspective);

    // Mutações de câmera (orbit, pan, zoom)
    state.session.camera.orbit(30.0, 15.0);
    state.session.camera.pan(10.0, -5.0);
    state.session.camera.zoom(2.0);

    // Documento e histórico permanecem rigorosamente intocados
    assert_eq!(
        state.project.project.revision_id(),
        initial_rev,
        "Mutação de câmera não deve alterar revisão do documento"
    );
    assert_eq!(
        state.project.undo.depth(),
        initial_undo,
        "Mutação de câmera não deve poluir pilha de Undo"
    );
    assert!(
        !state.is_dirty(),
        "Mutação de câmera não deve marcar documento como dirty"
    );
}

#[test]
fn test_a07_pseudo_isometric_2to1_and_existing_presets() {
    let mut camera = Camera::default();
    let initial_visible_height = camera.visible_height();

    // 1. Preset Pseudo-Isométrico 2:1 (26,565°)
    camera.set_preset(ViewPreset::PseudoIsometric2To1);
    assert_eq!(camera.proj, Projection::Ortho);
    assert_eq!(camera.view_preset(), Some(ViewPreset::PseudoIsometric2To1));

    let (name, yaw_deg, pitch_deg) = camera.nominal_view();
    assert_eq!(name, "Pseudo-Isometric 2:1");
    assert!((yaw_deg - 45.0).abs() < 1e-3);
    assert!(
        (pitch_deg - 26.565).abs() < 1e-3,
        "Pitch do pseudo-isométrico 2:1 deve ser exatamente 26,565°"
    );

    // Preservação estrita de framing
    assert!(
        (camera.visible_height() - initial_visible_height).abs() < 1e-4,
        "Framing da câmera deve ser preservado ao comutar para pseudo-isométrico"
    );

    // 2. Distinção clara contra isométrico verdadeiro (35,264°)
    let mut iso_camera = Camera::default();
    iso_camera.set_preset(ViewPreset::IsometricNE);
    let (_, _, iso_pitch) = iso_camera.nominal_view();
    assert!(
        (iso_pitch - 35.264).abs() < 0.1,
        "Isométrico verdadeiro usa pitch de ~35,264°"
    );
    assert!(
        (iso_pitch - pitch_deg).abs() > 8.0,
        "Pseudo-isométrico e isométrico verdadeiro devem ser tratados como presets distintos"
    );

    // 3. Presets cardinais e perspectiva
    for (preset, expected_ortho) in [
        (ViewPreset::Front, true),
        (ViewPreset::Back, true),
        (ViewPreset::Left, true),
        (ViewPreset::Right, true),
        (ViewPreset::Top, true),
        (ViewPreset::Bottom, true),
        (ViewPreset::IsometricNW, true),
        (ViewPreset::IsometricSE, true),
        (ViewPreset::IsometricSW, true),
        (ViewPreset::Persp, false),
    ] {
        camera.set_preset(preset);
        assert_eq!(
            camera.proj == Projection::Ortho,
            expected_ortho,
            "Preset {preset:?} falhou no tipo de projeção esperado"
        );
        assert!(
            camera.view_proj().is_finite(),
            "Matriz view_proj deve ser finita para {preset:?}"
        );
    }
}

#[test]
fn test_a07_camera_preset_command_dispatch() {
    let mut state = AppState::default();

    // Despacho do comando SetViewPresetCmd
    let report = state
        .dispatch_detailed(&SetViewPresetCmd(ViewPreset::PseudoIsometric2To1))
        .expect("SetViewPresetCmd deve executar com sucesso");

    assert_eq!(report.label, "view pseudo-isometric 2:1");
    assert_eq!(report.changes, ProjectChanges::NONE);
    assert_eq!(report.undo_depth, 0);
    assert_eq!(report.redo_depth, 0);
    assert!(report.effects.redraw);
    assert_eq!(
        state.session.camera.view_preset(),
        Some(ViewPreset::PseudoIsometric2To1)
    );

    // Despacho de comando de vista frontal
    let front_report = state
        .dispatch_detailed(&SetViewPresetCmd(ViewPreset::Front))
        .expect("SetViewPresetCmd(Front) deve executar com sucesso");

    assert_eq!(front_report.label, "view front");
    assert_eq!(front_report.undo_depth, 0);
    assert_eq!(state.session.camera.view_preset(), Some(ViewPreset::Front));
}

#[test]
fn test_a07_navigation_anchors_and_cursor_independence() {
    let mut state = AppState::default();

    // 1. Default é NavigationAnchor::ViewTarget
    assert_eq!(
        state.session.navigation_anchor,
        NavigationAnchor::ViewTarget
    );

    // 2. Cursor3D inicial e transição de modo de âncora
    state.session.cursor_3d = [5.0, 10.0, -2.0];
    let initial_cam_eye = state.session.camera.eye();
    let initial_cam_target = state.session.camera.target;

    // Trocar modo de navegação para Cursor3D não altera a câmera imediatamente
    state.session.navigation_anchor = NavigationAnchor::Cursor3D;
    assert_eq!(state.session.camera.eye(), initial_cam_eye);
    assert_eq!(state.session.camera.target, initial_cam_target);

    // 3. Pan e Frame não alteram a posição do cursor 3D
    state.session.camera.pan(50.0, 50.0);
    assert_eq!(
        state.session.cursor_3d,
        [5.0, 10.0, -2.0],
        "Pan não deve mover o cursor 3D"
    );

    state.session.camera.frame(Vec3::new(100.0, 0.0, 0.0), 20.0);
    assert_eq!(
        state.session.cursor_3d,
        [5.0, 10.0, -2.0],
        "Frame não deve mover o cursor 3D"
    );
}

#[test]
fn test_a07_camera_numeric_limits_and_nan_inf_rejection() {
    let mut camera = Camera::default();
    let valid_height = camera.visible_height();

    // Rejeição de NaN e Inf em set_visible_height
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -5.0, 0.0] {
        assert!(
            !camera.set_visible_height(invalid),
            "Valor inválido {invalid} deve ser rejeitado"
        );
        assert_eq!(
            camera.visible_height(),
            valid_height,
            "Altura visível anterior deve ser preservada ao rejeitar {invalid}"
        );
    }

    assert!(camera.view_proj().is_finite());
}
