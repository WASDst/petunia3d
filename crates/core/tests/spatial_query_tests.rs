//! Testes unitários para Dimensões e Comparação Espacial (VSP-003 e VSP-004 / Task H29).
//!
//! Cobre:
//! - VSP-003: Volume Frame (World vs ObjectLocal vs Structural; Evaluated vs Source; qualidade;
//!   formatação sem inventar cm/m em cenas genéricas; ScaleToDimensionCmd com prevenção contra divisão por zero e Undo/Redo).
//! - VSP-004: Consultas espaciais distintas (EuclideanDistance, AxisDelta, CameraDepth, SurfaceClearance,
//!   ContactCheck, VolumetricIntersection via SAT 15 eixos).
//! - VSP-004: Companion View ortográfica independente, opt-in, sem roubo de foco.
//! - VSP-004: Invariante X-Ray vs Select Through (X-Ray visual nunca altera Select Through).

use std::f32::consts::PI;

use glam::{Quat, Vec3};
use petunia_core::command::Command;
use petunia_core::modal::ModalKind;
use petunia_core::spatial_query::{
    GeometryRepresentation, ScaleToDimensionCmd, SpatialFrame, SpatialFrameKind,
    SpatialQueryInspector, SpatialQueryKind, VolumeFrame,
};
use petunia_core::state::AppState;
use petunia_mesh::Mesh;
use petunia_project::{Asset, ObjectTransform, Project};

#[test]
fn test_vsp003_volume_frame_world_vs_object_local_invariance() {
    let mut project = Project::new();
    let cube_mesh = Mesh::cube(2.0); // Cubo 2x2x2 -> volume 8.0

    // Rotaciona o cubo por 45° em torno do eixo Z
    let rot = Quat::from_rotation_z(PI * 0.25);
    let mut asset = Asset::new("Rotated Cube", cube_mesh);
    let asset_id = asset.id;
    asset.transform = ObjectTransform::from_trs(Vec3::new(10.0, 5.0, 0.0), rot, Vec3::ONE);
    project.assets.push(asset);

    // 1. Frame World: AABB alinhada ao mundo
    let world_vf = SpatialQueryInspector::compute_volume_frame(
        &project,
        asset_id,
        SpatialFrameKind::World,
        GeometryRepresentation::Evaluated,
    )
    .expect("Volume frame World deve ser calculado");

    // Para cubo 2x2 rotacionado por 45°, a extensão em X e Y no mundo é 2.0 * sqrt(2) ≈ 2.828
    let expected_world_extent = 2.0 * 2.0f32.sqrt();
    assert!(
        (world_vf.size[0] - expected_world_extent).abs() < 1e-3,
        "World X deve ser ~2.828, obtido {}",
        world_vf.size[0]
    );
    assert!(
        (world_vf.size[1] - expected_world_extent).abs() < 1e-3,
        "World Y deve ser ~2.828, obtido {}",
        world_vf.size[1]
    );
    assert!(
        (world_vf.size[2] - 2.0).abs() < 1e-3,
        "World Z deve ser 2.0, obtido {}",
        world_vf.size[2]
    );
    let world_volume = world_vf.volume;
    assert!(
        world_volume > 15.0,
        "Volume no frame World cresce devido à rotação (esperado ~16.0, obtido {world_volume})"
    );

    // 2. Frame ObjectLocal: OBB alinhado ao objeto
    let local_vf = SpatialQueryInspector::compute_volume_frame(
        &project,
        asset_id,
        SpatialFrameKind::ObjectLocal,
        GeometryRepresentation::Evaluated,
    )
    .expect("Volume frame ObjectLocal deve ser calculado");

    // No frame do objeto, as dimensões devem ser exatamente 2.0 x 2.0 x 2.0 com volume 8.0
    assert!(
        (local_vf.size[0] - 2.0).abs() < 1e-3,
        "Local X deve ser exatamente 2.0, obtido {}",
        local_vf.size[0]
    );
    assert!(
        (local_vf.size[1] - 2.0).abs() < 1e-3,
        "Local Y deve ser exatamente 2.0, obtido {}",
        local_vf.size[1]
    );
    assert!(
        (local_vf.size[2] - 2.0).abs() < 1e-3,
        "Local Z deve ser exatamente 2.0, obtido {}",
        local_vf.size[2]
    );
    assert!(
        (local_vf.volume - 8.0).abs() < 1e-3,
        "Volume no frame local deve ser 8.0, obtido {}",
        local_vf.volume
    );

    // 3. Invariante VSP-003 comprovado: World e Local NÃO são equivalentes
    assert_ne!(
        world_vf.size, local_vf.size,
        "Invariante VSP-003: Volume Frame World e ObjectLocal não são redutíveis à mesma caixa"
    );
    assert!(
        world_vf.volume > local_vf.volume + 5.0,
        "AABB no mundo é significativamente maior que OBB local"
    );

    // 4. Frame Structural: PCA derivado dos vértices
    let struct_vf = SpatialQueryInspector::compute_volume_frame(
        &project,
        asset_id,
        SpatialFrameKind::Structural,
        GeometryRepresentation::Evaluated,
    )
    .expect("Volume frame Structural deve ser calculado");

    assert!(
        (struct_vf.volume - 8.0).abs() < 0.1,
        "Frame estrutural deve recuperar o volume mínimo do cubo (esperado ~8.0, obtido {})",
        struct_vf.volume
    );
}

#[test]
fn test_vsp003_formatted_dimensions_without_inventing_units() {
    let mut vf = VolumeFrame {
        frame: SpatialFrame::world(),
        representation: GeometryRepresentation::Evaluated,
        quality: petunia_core::modeling_feedback::MeasurementQuality::Exact,
        center: [0.0, 0.0, 0.0],
        min: [-1.0, -1.0, -1.0],
        max: [1.0, 1.0, 1.0],
        size: [2.0, 3.5, 4.125],
        volume: 28.875,
        unit: String::new(), // Cena genérica sem unidade configurada
        vertex_count: 8,
        revision: 1,
    };

    // VSP-003: Não inventar cm/m em cenas genéricas
    assert_eq!(vf.formatted_dimensions(), "2.000 × 3.500 × 4.125");

    // Quando uma unidade é explicitamente fornecida
    vf.unit = "m".to_string();
    assert_eq!(vf.formatted_dimensions(), "2.000 × 3.500 × 4.125 m");
}

#[test]
fn test_vsp003_scale_to_dimension_division_by_zero_guard() {
    let mut state = AppState::default();
    let plane_mesh = Mesh::plane(4.0); // Plano 4x4 em Z=0 (espessura Z = 0.0)
    let asset = Asset::new("Flat Plane", plane_mesh);
    let asset_id = asset.id;
    state.project.project.assets.push(asset);

    // Tentativa de escalar no eixo Y (axis = 1), onde a dimensão do plano é zero!
    let cmd = ScaleToDimensionCmd::new(
        asset_id,
        1, // Y axis
        5.0,
        false,
        SpatialFrameKind::ObjectLocal,
    );

    // Deve rejeitar graciosamente sem pânico
    let direct_result = state.in_context(|ctx| cmd.execute(ctx));
    assert!(
        direct_result.is_err(),
        "Escalar objeto com dimensão zero deve retornar erro e evitar divisão por zero"
    );
    let err_msg = format!("{:?}", direct_result.unwrap_err());
    assert!(
        err_msg.contains("division by zero avoided"),
        "Mensagem de erro deve explicitar prevenção de divisão por zero: {err_msg}"
    );

    // Tentativa com dimensão alvo negativa ou zero
    let invalid_cmd = ScaleToDimensionCmd::new(asset_id, 0, -2.0, false, SpatialFrameKind::World);
    let invalid_res = state.in_context(|ctx| invalid_cmd.execute(ctx));
    assert!(
        invalid_res.is_err(),
        "Dimensão alvo negativa deve ser rejeitada"
    );

    let zero_cmd = ScaleToDimensionCmd::new(asset_id, 0, 0.0, false, SpatialFrameKind::World);
    let zero_res = state.in_context(|ctx| zero_cmd.execute(ctx));
    assert!(zero_res.is_err(), "Dimensão alvo zero deve ser rejeitada");
}

#[test]
fn test_vsp003_scale_to_dimension_execution_and_undo() {
    let mut state = AppState::default();
    let cube_mesh = Mesh::cube(2.0); // Cubo 2x2x2
    let asset = Asset::new("Scale Target", cube_mesh);
    let asset_id = asset.id;
    state.project.project.assets.push(asset);

    // Escala eixo X de 2.0 para 6.0 (ratio = 3.0)
    let res = state.scale_to_dimension(
        asset_id,
        0, // X
        6.0,
        false, // não uniforme
        SpatialFrameKind::ObjectLocal,
    );
    assert!(
        res.is_ok(),
        "Escala para dimensão deve ter sucesso: {:?}",
        res
    );

    // Verifica que a nova dimensão em X é 6.0
    let vf = state
        .query_volume_frame(
            asset_id,
            SpatialFrameKind::ObjectLocal,
            GeometryRepresentation::Evaluated,
        )
        .expect("Volume frame");
    assert!(
        (vf.size[0] - 6.0).abs() < 1e-3,
        "Dimensão X deve ser 6.0, obtido {}",
        vf.size[0]
    );
    assert!(
        (vf.size[1] - 2.0).abs() < 1e-3,
        "Dimensão Y deve permanecer 2.0"
    );
    assert!(
        (vf.size[2] - 2.0).abs() < 1e-3,
        "Dimensão Z deve permanecer 2.0"
    );

    // Testa Undo: deve restaurar a dimensão para 2.0
    let undo_res = state.dispatch_command("edit.undo");
    assert!(undo_res.is_ok(), "Undo deve executar com sucesso");
    let vf_undone = state
        .query_volume_frame(
            asset_id,
            SpatialFrameKind::ObjectLocal,
            GeometryRepresentation::Evaluated,
        )
        .expect("Volume frame após undo");
    assert!(
        (vf_undone.size[0] - 2.0).abs() < 1e-3,
        "Dimensão X restaurada para 2.0"
    );

    // Testa Redo: deve reaplicar para 6.0
    let redo_res = state.dispatch_command("edit.redo");
    assert!(redo_res.is_ok(), "Redo deve executar com sucesso");
    let vf_redone = state
        .query_volume_frame(
            asset_id,
            SpatialFrameKind::ObjectLocal,
            GeometryRepresentation::Evaluated,
        )
        .expect("Volume frame após redo");
    assert!(
        (vf_redone.size[0] - 6.0).abs() < 1e-3,
        "Dimensão X refeita para 6.0"
    );
}

#[test]
fn test_vsp004_all_six_spatial_queries() {
    let mut project = Project::new();
    project.assets.clear();

    // Asset A: Cubo 2x2x2 em X = 0 (de -1 a +1)
    let mesh_a = Mesh::cube(2.0);
    let mut asset_a = Asset::new("Cube A", mesh_a);
    let id_a = asset_a.id;
    asset_a.transform =
        ObjectTransform::from_trs(Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE);
    project.assets.push(asset_a);

    // Asset B: Cubo 2x2x2 em X = 5 (de +4 a +6) -> Clearance entre A e B é exatamente 3.0 (de +1 a +4)
    let mesh_b = Mesh::cube(2.0);
    let mut asset_b = Asset::new("Cube B", mesh_b);
    let id_b = asset_b.id;
    asset_b.transform =
        ObjectTransform::from_trs(Vec3::new(5.0, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE);
    project.assets.push(asset_b);

    // 1. EuclideanDistance
    let q1 = SpatialQueryKind::EuclideanDistance {
        from: [0.0, 0.0, 0.0],
        to: [3.0, 4.0, 0.0],
    };
    let r1 = SpatialQueryInspector::execute_query(&project, q1).expect("Query 1");
    assert!(
        (r1.distance - 5.0).abs() < 1e-4,
        "Distância euclidiana deve ser 5.0"
    );
    assert_eq!(r1.delta, Some([3.0, 4.0, 0.0]));

    // 2. AxisDelta em frame canônico
    let q2 = SpatialQueryKind::AxisDelta {
        from: [1.0, 2.0, 3.0],
        to: [4.0, 6.0, 3.0],
        frame: SpatialFrame::world(),
    };
    let r2 = SpatialQueryInspector::execute_query(&project, q2).expect("Query 2");
    assert_eq!(r2.delta, Some([3.0, 4.0, 0.0]));
    assert!((r2.distance - 5.0).abs() < 1e-4);

    // 3. CameraDepth
    let q3 = SpatialQueryKind::CameraDepth {
        point: [0.0, 0.0, -10.0],
        camera_pos: [0.0, 0.0, 0.0],
        camera_forward: [0.0, 0.0, -1.0],
    };
    let r3 = SpatialQueryInspector::execute_query(&project, q3).expect("Query 3");
    assert!(
        (r3.distance - 10.0).abs() < 1e-4,
        "Profundidade de câmera deve ser 10.0"
    );

    // 4. SurfaceClearance entre Cubo A (max X = 1) e Cubo B (min X = 4)
    let q4 = SpatialQueryKind::SurfaceClearance {
        object_a: id_a,
        object_b: id_b,
    };
    let r4 = SpatialQueryInspector::execute_query(&project, q4).expect("Query 4");
    assert!(
        (r4.distance - 3.0).abs() < 1e-3,
        "Clearance entre Cubo A e Cubo B deve ser 3.0, obtido {}",
        r4.distance
    );
    assert!(!r4.is_in_contact);
    assert!(!r4.is_intersecting);
    let (pt_a, pt_b) = r4.closest_points.expect("Pontos mais próximos");
    assert!(
        (pt_a[0] - 1.0).abs() < 1e-3,
        "Ponto mais próximo de A deve estar em X=1"
    );
    assert!(
        (pt_b[0] - 4.0).abs() < 1e-3,
        "Ponto mais próximo de B deve estar em X=4"
    );

    // 5. ContactCheck com tolerância
    // Com tolerância 2.0 (menor que distância 3.0) -> Sem contato
    let q5_no = SpatialQueryKind::ContactCheck {
        object_a: id_a,
        object_b: id_b,
        tolerance: 2.0,
    };
    let r5_no = SpatialQueryInspector::execute_query(&project, q5_no).expect("Query 5 no contact");
    assert!(
        !r5_no.is_in_contact,
        "Com tolerância 2.0 < 3.0 não deve haver contato"
    );

    // Com tolerância 3.5 (maior que distância 3.0) -> Em contato
    let q5_yes = SpatialQueryKind::ContactCheck {
        object_a: id_a,
        object_b: id_b,
        tolerance: 3.5,
    };
    let r5_yes = SpatialQueryInspector::execute_query(&project, q5_yes).expect("Query 5 contact");
    assert!(
        r5_yes.is_in_contact,
        "Com tolerância 3.5 >= 3.0 deve haver contato"
    );

    // 6. VolumetricIntersection via SAT
    // A e B separados por 3.0 -> Não intersectam
    let q6_disjoint = SpatialQueryKind::VolumetricIntersection {
        object_a: id_a,
        object_b: id_b,
    };
    let r6_disjoint =
        SpatialQueryInspector::execute_query(&project, q6_disjoint).expect("Query 6 disjoint");
    assert!(!r6_disjoint.is_intersecting);
    assert_eq!(r6_disjoint.penetration_depth, None);

    // Move Cubo B para sobrepor Cubo A (X = 1.0, centro em 1.0, de 0.0 a 2.0)
    // Cubo A vai de -1 a +1. Sobreposição de 0.0 a 1.0 -> penetração = 1.0
    project
        .assets
        .iter_mut()
        .find(|a| a.id == id_b)
        .unwrap()
        .transform = ObjectTransform::from_trs(Vec3::new(1.0, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE);

    let q6_overlap = SpatialQueryKind::VolumetricIntersection {
        object_a: id_a,
        object_b: id_b,
    };
    let r6_overlap =
        SpatialQueryInspector::execute_query(&project, q6_overlap).expect("Query 6 overlap");
    assert!(
        r6_overlap.is_intersecting,
        "Objetos sobrepostos devem intersectar"
    );
    assert!(r6_overlap.penetration_depth.is_some());
    let pen = r6_overlap.penetration_depth.unwrap();
    assert!(
        (pen - 1.0).abs() < 1e-3,
        "Profundidade de penetração deve ser 1.0, obtido {pen}"
    );
}

#[test]
fn test_vsp004_companion_view_independence_and_no_focus_stealing() {
    let mut state = AppState::default();

    // Estado inicial: Companion View desativada
    assert!(!state.is_companion_view_open());
    assert!(state.companion_view_state().is_orthographic);

    // Câmera principal da cena
    let initial_cam_pos = state.camera.eye();

    // Abre Companion View opt-in
    state.open_companion_view();
    assert!(state.is_companion_view_open());

    // Câmera principal não é alterada (Companion View é independente)
    assert_eq!(state.camera.eye(), initial_cam_pos);

    // Inicia uma operação modal (ex: Grab/Move)
    state.begin_modal(ModalKind::Move).expect("begin modal");
    assert!(state.is_interacting());

    // Alternar Companion View NÃO rouba foco nem cancela a operação modal ativa
    state.toggle_companion_view();
    assert!(!state.is_companion_view_open());
    assert!(
        state.is_interacting(),
        "Alternar Companion View não deve cancelar ferramenta modal"
    );

    state.toggle_companion_view();
    assert!(state.is_companion_view_open());
    assert!(
        state.is_interacting(),
        "Reabrir Companion View não deve cancelar ferramenta modal"
    );
}

#[test]
fn test_vsp004_xray_vs_select_through_strict_invariance() {
    let mut state = AppState::default();

    // Estado inicial: ambos false
    assert!(!state.show_xray);
    assert!(!state.select_through);

    // Ativar X-Ray NÃO ativa Select Through
    state.set_show_xray(true);
    assert!(state.show_xray, "X-Ray deve estar ativado");
    assert!(
        !state.select_through,
        "Invariante VSP-004: Ativar X-Ray NUNCA ativa Select Through"
    );

    // Ativar Select Through explicitamente
    state.set_select_through(true);
    assert!(
        state.select_through,
        "Select Through explicitamente ativado"
    );
    assert!(state.show_xray, "X-Ray continua ativo");

    // Desativar X-Ray NÃO desativa Select Through (são ortogonais)
    state.set_show_xray(false);
    assert!(!state.show_xray, "X-Ray desativado");
    assert!(
        state.select_through,
        "Invariante VSP-004: Desativar X-Ray NÃO altera Select Through configurado pelo usuário"
    );

    // Desativar Select Through
    state.set_select_through(false);
    assert!(!state.show_xray);
    assert!(!state.select_through);
}

#[test]
fn test_vsp003_axial_symmetry_anchor_alignment_and_quality() {
    let cylinder_mesh = Mesh::cylinder(32, 1.0, 6.0); // Raio 1.0, altura 6.0 em Y
    let transform = ObjectTransform::default();

    let (frame, quality) = SpatialFrame::structural_with_quality(&cylinder_mesh, &transform);

    // 1. Simetria axial detectada e tratada com Anchor Alignment
    assert_eq!(
        quality,
        petunia_core::modeling_feedback::MeasurementQuality::Approximate,
        "Cilindro com simetria axial deve reportar qualidade Approximate"
    );

    // 2. O eixo longitudinal dominante deve estar alinhado com o eixo Y ([0, 1, 0])
    let dominant_axis = Vec3::from(frame.axes[0]);
    assert!(
        (dominant_axis.y.abs() - 1.0).abs() < 1e-2,
        "Eixo dominante deve alinhar com Y: {:?}",
        dominant_axis
    );

    // 3. Os eixos transversais devem estar perfeitamente ortogonalizados e ancorados
    let axis_1 = Vec3::from(frame.axes[1]);
    let axis_2 = Vec3::from(frame.axes[2]);

    assert!(
        dominant_axis.dot(axis_1).abs() < 1e-3,
        "Eixos devem ser mutuamente ortogonais"
    );
    assert!(
        dominant_axis.dot(axis_2).abs() < 1e-3,
        "Eixos devem ser mutuamente ortogonais"
    );
    assert!(
        axis_1.dot(axis_2).abs() < 1e-3,
        "Eixos transversais devem ser mutuamente ortogonais"
    );
}

#[test]
fn test_vsp004_clearance_acceleration_culling_and_early_exit() {
    let mut project = Project::new();
    project.assets.clear();

    // Asset A: Cubo em [0, 0, 0] (tamanho 2x2x2 -> X vai de -1 a +1)
    let mesh_a = Mesh::cube(2.0);
    let mut asset_a = Asset::new("Cube A", mesh_a);
    let id_a = asset_a.id;
    asset_a.transform =
        ObjectTransform::from_trs(Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE);
    project.assets.push(asset_a);

    // Asset B: Cubo em [20, 0, 0] (tamanho 2x2x2 -> X vai de 19 a 21)
    // Separação real de 18.0 unidades
    let mesh_b = Mesh::cube(2.0);
    let mut asset_b = Asset::new("Cube B", mesh_b);
    let id_b = asset_b.id;
    asset_b.transform =
        ObjectTransform::from_trs(Vec3::new(20.0, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE);
    project.assets.push(asset_b);

    // Teste 1: ContactCheck com tolerância 2.0 (muito menor que a separação de 18.0)
    // Dispara a Camada 0 (rejeição em O(1) via AABB culling)
    let q_contact_far = SpatialQueryKind::ContactCheck {
        object_a: id_a,
        object_b: id_b,
        tolerance: 2.0,
    };
    let res_far =
        SpatialQueryInspector::execute_query(&project, q_contact_far).expect("Query Contact Far");
    assert!(
        !res_far.is_in_contact,
        "Objetos distantes não devem estar em contato"
    );
    assert!(
        res_far.distance >= 18.0,
        "Distância deve ser pelo menos 18.0"
    );

    // Teste 2: Move Cubo B para colidir/tangenciar Cubo A (X = 2.0001 -> separação ~0.0001)
    project
        .assets
        .iter_mut()
        .find(|a| a.id == id_b)
        .unwrap()
        .transform =
        ObjectTransform::from_trs(Vec3::new(2.0001, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE);

    // ContactCheck com tolerância 0.1 -> deve detectar contato imediatamente via Camada 2 (Early Exit)
    let q_contact_touch = SpatialQueryKind::ContactCheck {
        object_a: id_a,
        object_b: id_b,
        tolerance: 0.1,
    };
    let res_touch = SpatialQueryInspector::execute_query(&project, q_contact_touch)
        .expect("Query Contact Touch");
    assert!(
        res_touch.is_in_contact,
        "Objetos muito próximos devem ser detectados em contato"
    );
    assert!(res_touch.distance < 0.01, "Distância deve ser minúscula");

    // Teste 3: SurfaceClearance exato
    let q_clearance = SpatialQueryKind::SurfaceClearance {
        object_a: id_a,
        object_b: id_b,
    };
    let res_clearance =
        SpatialQueryInspector::execute_query(&project, q_clearance).expect("Query Clearance");
    assert!(
        (res_clearance.distance - 0.0001).abs() < 1e-3,
        "Clearance exato deve ser ~0.0001"
    );
}

#[test]
fn test_vsp003_surface_inertia_tessellation_invariance() {
    // Malha A: Caixa canônica 2.0 x 4.0 x 6.0 centrada em [0, 0, 0]
    let box_clean = Mesh::box_dim(2.0, 4.0, 6.0);
    let transform = ObjectTransform::default();

    let frame_clean = SpatialFrame::structural(&box_clean, &transform);
    assert!(
        Vec3::from(frame_clean.origin).length() < 1e-4,
        "Centroide da caixa canônica deve ser [0, 0, 0]"
    );

    // Malha B: Mesma caixa, mas a face +Z (quad [4, 5, 6, 7] em Z=3.0) é subdividida
    // adicionando um vértice central em (0, 0, 3) e 4 triângulos.
    // Em um PCA ingênuo por vértices, o centroide seria puxado para Z positivo (3/9 = 0.333).
    // Com o cálculo por integral de superfície (área), o centroide e os eixos permanecem invariantes.
    let mut box_subdivided = Mesh::box_dim(2.0, 4.0, 6.0);
    // Adiciona vértice no centro da face +Z
    box_subdivided
        .verts
        .push(petunia_mesh::Vertex::new(0.0, 0.0, 3.0));
    // Remove o quad face[1]
    box_subdivided.faces.remove(1);
    // Adiciona 4 triângulos cobrindo a mesma face
    box_subdivided
        .faces
        .push(petunia_mesh::Face::new(vec![4, 5, 8]));
    box_subdivided
        .faces
        .push(petunia_mesh::Face::new(vec![5, 6, 8]));
    box_subdivided
        .faces
        .push(petunia_mesh::Face::new(vec![6, 7, 8]));
    box_subdivided
        .faces
        .push(petunia_mesh::Face::new(vec![7, 4, 8]));

    let frame_subdivided = SpatialFrame::structural(&box_subdivided, &transform);

    assert!(
        (frame_subdivided.origin[2] - 0.0).abs() < 1e-3,
        "Centroide Z deve permanecer 0.0 após subdivisão de face: obtido {}",
        frame_subdivided.origin[2]
    );

    // Os eixos principais de inércia devem ser idênticos (alinhados com Z, Y, X)
    for i in 0..3 {
        let axis_clean = Vec3::from(frame_clean.axes[i]);
        let axis_sub = Vec3::from(frame_subdivided.axes[i]);
        assert!(
            axis_clean.dot(axis_sub).abs() > 0.99,
            "Eixo {} deve permanecer alinhado (invariância à tesselação): {:?} vs {:?}",
            i,
            axis_clean,
            axis_sub
        );
    }
}
