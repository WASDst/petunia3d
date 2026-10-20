//! Testes de conformidade do feedback comum de operações (Task H28 / MF-001 a MF-008).
//!
//! Valida:
//! - MF-001: Previews reversíveis não-destrutivas, snapshot congelado e Undo único.
//! - MF-002: Extrude (distância signed e ghost), Inset (limite anti-colapso) e Bevel (limite anti-overlap).
//! - MF-003: Boolean com operandos abertos/não-manifold (incompatibilidade explícita, sem auto-reparo).
//! - MF-004: Mirror/Symmetry (plano discreto, detecção de assimetria artística sem imposição de simetria).
//! - MF-005: Pincel de escultura / deformação (footprint, falloff e campo de influência).
//! - MF-006: Avisos contextuais não-bloqueantes sem interrupção de fluxo.
//! - MF-007: Incompatibilidades na prévia (remesh com skin bind exige rebind; seleção parcial reporta alvos ignorados).
//! - MF-008: Consultas de feedback são 100% puras e não alteram ferramentas, atalhos ou documento.

use glam::Vec3;
use petunia_core::modeling_feedback::{ActiveDimension, WarningSeverity};
use petunia_core::{AppState, EditMode, ModalKind};
use petunia_mesh::{Mesh, Vertex};
use petunia_project::Asset;

#[test]
fn test_mf001_non_destructive_reversible_preview_and_undo() {
    let mut state = AppState::default();
    let initial_undo = state.project.undo.depth().0;
    let original_faces = state.project.active_mesh().unwrap().faces.len();

    // Inicia modal de Extrude sobre a face 0
    state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    state.sync_selection();
    state
        .begin_modal(ModalKind::Extrude)
        .expect("begin extrude");

    // Aplica prévia com deslocamento de 1.5 metros
    state
        .update_modal(Vec3::Z * 1.5, 1.5)
        .expect("update modal");

    // Inspeciona feedback durante a prévia
    let fb = state.current_modeling_feedback().expect("feedback ativo");
    assert_eq!(fb.operation_id, "Extrude");
    assert_eq!(fb.active_dimension, ActiveDimension::Distance);
    assert_eq!(fb.numeric_value, 1.5);

    // MF-001: Ghost da malha de origem congelada corresponde à malha antes da extrusão
    let ghost = fb.ghost_mesh.expect("ghost mesh presente");
    assert_eq!(ghost.faces.len(), original_faces);

    // Malha de prévia atual já contém as novas paredes da extrusão
    let preview = fb.preview_mesh.expect("preview mesh presente");
    assert!(preview.faces.len() > original_faces);

    // Cancelar restaura exatamente o snapshot original sem gravar Undo
    state.cancel_modal();
    assert_eq!(state.project.undo.depth().0, initial_undo);
    assert_eq!(
        state.project.active_mesh().unwrap().faces.len(),
        original_faces
    );

    // Nova operação com commit: grava exatamente 1 entrada de Undo
    state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    state.sync_selection();
    state.begin_modal(ModalKind::Extrude).unwrap();
    state.update_modal(Vec3::Z * 2.0, 2.0).unwrap();
    state.commit_modal();
    assert_eq!(state.project.undo.depth().0, initial_undo + 1);
    assert!(state.undo());
    assert_eq!(
        state.project.active_mesh().unwrap().faces.len(),
        original_faces
    );
}

#[test]
fn test_mf002_extrude_inset_bevel_bounds_and_limits() {
    let mut state = AppState::default();

    // 1. Extrude negativo: emite aviso informativo de cavidade interna
    state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    state.sync_selection();
    state.begin_modal(ModalKind::Extrude).unwrap();
    state.update_modal(Vec3::Z * -1.0, -1.0).unwrap();
    let fb_extrude = state.current_modeling_feedback().unwrap();
    assert_eq!(fb_extrude.numeric_value, -1.0);
    assert!(
        fb_extrude
            .warnings
            .iter()
            .any(|w| w.code == "EXTRUDE_NEGATIVE")
    );
    state.cancel_modal();

    // 2. Inset: quando o offset atinge o limite do inradius da face, emite aviso anti-colapso
    state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    state.sync_selection();
    state.begin_modal(ModalKind::Inset).unwrap();
    // Um cubo 2x2 tem inradius de face 1.0; um offset de 1.0 atinge/ultrapassa o colapso
    state.update_modal(Vec3::ZERO, 1.0).unwrap();
    let fb_inset = state.current_modeling_feedback().unwrap();
    assert_eq!(fb_inset.active_dimension, ActiveDimension::Offset);
    assert!(
        fb_inset
            .warnings
            .iter()
            .any(|w| w.code == "INSET_COLLAPSE_LIMIT"),
        "Inset deve avisar proximidade do limite de colapso de arestas"
    );
    assert!(
        fb_inset.markers.iter().any(|m| matches!(
            m,
            petunia_core::modeling_feedback::FeedbackMarker::LimitBoundary { .. }
        )),
        "Deve exibir marcador de limite de colapso"
    );
    state.cancel_modal();

    // 3. Bevel: quando o raio excede metade do comprimento da aresta, emite aviso de overlap
    state.project.active_mesh_mut().unwrap().selected_edges =
        std::collections::HashSet::from([(0, 1)]);
    state.sync_selection();
    state.begin_modal(ModalKind::Bevel).unwrap();
    state.update_modal(Vec3::ZERO, 1.2).unwrap();
    let fb_bevel = state.current_modeling_feedback().unwrap();
    assert!(
        fb_bevel
            .warnings
            .iter()
            .any(|w| w.code == "BEVEL_OVERLAP_LIMIT"),
        "Bevel deve avisar overlap ao exceder metade da aresta"
    );
    state.cancel_modal();
}

#[test]
fn test_mf003_boolean_open_operands_and_ghost_separation() {
    let mut state = AppState::default();
    let cube_id = state.project.assets[0].id;

    // Cria um plano aberto (não-fechado / non-manifold)
    let mut open_mesh = Mesh::default();
    open_mesh.verts.push(Vertex::new(-1.0, -1.0, 0.0));
    open_mesh.verts.push(Vertex::new(1.0, -1.0, 0.0));
    open_mesh.verts.push(Vertex::new(1.0, 1.0, 0.0));
    open_mesh.faces.push(petunia_mesh::Face::new(vec![0, 1, 2]));
    let open_asset = Asset::new("OpenPlane", open_mesh);
    let open_id = open_asset.id;
    state.project.assets.push(open_asset);

    // MF-003 / MF-T04: Preflight de booleana com operando aberto
    let fb = state.preflight_boolean_feedback(cube_id, open_id);

    // Operandos são diferenciados e ghost está presente
    assert!(fb.ghost_mesh.is_some());
    // Incompatibilidade explícita identificada antes do commit
    assert!(
        !fb.commit_enabled,
        "Commit deve ser bloqueado para operandos abertos"
    );
    assert!(
        fb.warnings
            .iter()
            .any(|w| w.code == "OPEN_BOOLEAN_OPERAND" && w.severity == WarningSeverity::Error),
        "Deve acusar erro explícito de operando aberto sem autocorreção oculta"
    );
}

#[test]
fn test_mf004_mirror_asymmetry_detection_without_forced_correction() {
    let mut state = AppState::default();

    // Cria um modelo assimétrico (um torso com saliência apenas em X positivo)
    let mut asym_mesh = Mesh::cube(1.0);
    asym_mesh.verts.push(Vertex::new(1.2, 0.0, 0.0)); // Vértice assimétrico
    let asym_asset = Asset::new("AsymmetricModel", asym_mesh);
    let asym_id = asym_asset.id;
    state.project.assets.push(asym_asset);

    // MF-004 / MF-T05: Preflight do espelhamento pelo plano YZ (normal X)
    let fb = state.preflight_mirror_feedback(asym_id, [1.0, 0.0, 0.0], 0.0);

    // Deve exibir o plano discreto de espelhamento
    assert!(fb.markers.iter().any(|m| matches!(
        m,
        petunia_core::modeling_feedback::FeedbackMarker::PlaneDisc { .. }
    )));

    // Detecta assimetria artística existente mas NÃO força simetria
    assert!(
        fb.warnings
            .iter()
            .any(|w| w.code == "ASYMMETRIC_SOURCE_DETECTED"),
        "Deve notificar assimetria artística sem correção forçada"
    );
    assert!(fb.commit_enabled);
}

#[test]
fn test_mf005_sculpt_brush_footprint_and_influence() {
    let state = AppState::default();

    // MF-005 / MF-T06: Preflight de sculpt com raio 0.35 e força nominal
    let fb = state.preflight_sculpt_feedback(0.35, 0.8, [0.0, 0.5, 0.0], [0.0, 1.0, 0.0]);

    assert_eq!(fb.numeric_value, 0.35);
    assert_eq!(fb.active_dimension, ActiveDimension::Distance);

    // Deve conter marcador do footprint do pincel com falloff
    let has_footprint = fb.markers.iter().any(|m| match m {
        petunia_core::modeling_feedback::FeedbackMarker::BrushFootprint {
            radius,
            falloff_radius,
            ..
        } => *radius == 0.35 && *falloff_radius == 0.175,
        _ => false,
    });
    assert!(has_footprint, "Footprint e falloff do pincel corretos");
}

#[test]
fn test_mf006_contextual_warnings_do_not_interrupt_or_silently_repair() {
    let mut state = AppState::default();

    // Scale em Edit Mode (Face) próximo a zero: avisa degradação sem pânico ou reparo silencioso
    state.set_edit_mode(EditMode::Edit);
    state.set_selection_domain(petunia_core::SelectionDomain::Face);
    state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    state.sync_selection();
    state.begin_modal(ModalKind::Scale).unwrap();
    state.update_modal(Vec3::ZERO, 0.00001).unwrap();

    let fb = state.current_modeling_feedback().unwrap();
    assert!(
        fb.warnings
            .iter()
            .any(|w| w.code == "SCALE_NEAR_ZERO" && w.severity == WarningSeverity::Warning)
    );
    // Operação continua funcional sem crash
    state.cancel_modal();
}

#[test]
fn test_mf007_remesh_skin_bind_and_partial_selection_reports() {
    let mut state = AppState::default();

    // 1. MF-T07: Asset com rig/skin binding submetido a remesh
    let mut skinned_asset = Asset::new("CharacterTorso", Mesh::cube(1.0));
    skinned_asset.skeleton_id = Some(uuid::Uuid::new_v4());
    let skinned_id = skinned_asset.id;
    state.project.assets.push(skinned_asset);

    let fb_skin = state.preflight_remesh_skin_feedback(skinned_id);
    assert!(
        fb_skin
            .warnings
            .iter()
            .any(|w| w.code == "SKIN_REMAP_REQUIRED"),
        "Remesh em malha skinned exige aviso explícito de rebind"
    );

    // 2. MF-T08: Seleção parcial incompatível (1 objeto bloqueado + 1 com faces selecionadas)
    state.project.assets[1].mesh.faces[0].selected = true;
    state.sync_selection();
    state.session.selection.assets = vec![state.project.assets[0].id, skinned_id];
    state.project.assets[0].locked = true;

    let fb_partial = state.preflight_partial_selection_feedback("face_operation");
    assert!(fb_partial.is_partial());
    let report = fb_partial.partial_selection.unwrap();
    assert_eq!(report.ignored_targets, vec![state.project.assets[0].id]);
    assert_eq!(report.supported_targets, vec![skinned_id]);
    assert!(report.requires_explicit_choice);
}

#[test]
fn test_mf008_feedback_queries_are_pure_and_non_mutating() {
    let mut state = AppState::default();
    state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    state.sync_selection();

    let initial_tool = state.active_tool.clone();
    let initial_selection = state.session.selection.clone();
    let initial_faces = state.project.active_mesh().unwrap().faces.len();
    let initial_undo = state.project.undo.depth();

    // Inicia modal para ter feedback ativo
    state.begin_modal(ModalKind::Move).unwrap();
    state.update_modal(Vec3::X * 2.0, 2.0).unwrap();

    // Executa múltiplas consultas de feedback
    let fb1 = state.current_modeling_feedback();
    assert!(fb1.is_some());
    let fb2 = state.preflight_boolean_feedback(uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
    assert!(!fb2.commit_enabled);
    let fb3 = state.preflight_sculpt_feedback(0.5, 1.0, [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    assert_eq!(fb3.numeric_value, 0.5);

    // Cancela modal
    state.cancel_modal();

    // MF-008: Documento, seleção, ferramenta e histórico intocados
    assert_eq!(state.active_tool, initial_tool);
    assert_eq!(state.session.selection, initial_selection);
    assert_eq!(
        state.project.active_mesh().unwrap().faces.len(),
        initial_faces
    );
    assert_eq!(state.project.undo.depth(), initial_undo);
}
