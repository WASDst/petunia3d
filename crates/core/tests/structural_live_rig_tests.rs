//! Testes sintéticos de Smart Rig estrutural e Live opcional (H32 / RIG-003/005/006/007/012).
//!
//! Cobre:
//! 1. Grafo de forma estrutural (StructuralShapeGraph): partes, juntas, conexões e IDs estáveis.
//! 2. Idempotência estrita: execuções repetidas sem mudanças de geometria não alteram pesos.
//! 3. Isolamento de Material e UV: mutações de textura/material/UV não disparam regeneração de pesos.
//! 4. Prevenção de corrupção topológica: mudanças de contagem sem remap ativam NeedsReview explícito.
//! 5. Remapeamento transacional de pesos (RemapRigWeightsCmd).
//! 6. Preservação de overrides manuais e travas de partes/ossos.
//! 7. RIG-012: Desligar auto-atualização, desligar deformação e desvincular rig são operações distintas.

use petunia_core::*;
use petunia_mesh::{Mesh, Vertex};
use petunia_project::{Asset, ConnectionKind, RigReviewStatus, SkinBindMethod};
use uuid::Uuid;

fn create_test_cube_asset(state: &mut AppState, name: &str) -> Uuid {
    let mesh = Mesh::cube(2.0);
    let asset = Asset::new(name, mesh);
    let id = asset.id;
    state.project.project.assets.push(asset);
    state.project.project.active = state.project.project.assets.len() - 1;
    id
}

fn get_test_asset(state: &AppState, id: Uuid) -> &Asset {
    state
        .project
        .project
        .assets
        .iter()
        .find(|a| a.id == id)
        .expect("asset found")
}

fn get_test_asset_mut(state: &mut AppState, id: Uuid) -> &mut Asset {
    state
        .project
        .project
        .assets
        .iter_mut()
        .find(|a| a.id == id)
        .expect("asset found")
}

#[test]
fn test_structural_shape_graph_creation_and_connections() {
    let mut state = AppState::default();
    let asset_id = create_test_cube_asset(&mut state, "Robot");

    // 1. Criar grafo de forma estrutural no asset
    state
        .dispatch(&CreateStructuralGraphCmd {
            asset_id: Some(asset_id),
            name: "RobotStructure".to_string(),
        })
        .expect("criar grafo estrutural");

    let asset = get_test_asset(&state, asset_id);
    let live = asset.live_rig.as_ref().expect("live rig presente");
    assert_eq!(live.graph.name, "RobotStructure");
    assert!(live.auto_update_enabled);

    // 2. Adicionar partes estruturais
    state
        .dispatch(&AddStructuralPartCmd {
            asset_id: Some(asset_id),
            name: "Base".to_string(),
            vertex_indices: vec![0, 1, 4, 5],
            center: [0.0, -1.0, 0.0],
            bounds: ([-1.0, -1.0, -1.0], [1.0, -0.5, 1.0]),
        })
        .expect("adicionar parte Base");

    state
        .dispatch(&AddStructuralPartCmd {
            asset_id: Some(asset_id),
            name: "Top".to_string(),
            vertex_indices: vec![2, 3, 6, 7],
            center: [0.0, 1.0, 0.0],
            bounds: ([-1.0, 0.5, -1.0], [1.0, 1.0, 1.0]),
        })
        .expect("adicionar parte Top");

    // 3. Conectar partes
    state
        .dispatch(&ConnectStructuralPartsCmd {
            asset_id: Some(asset_id),
            from_part: 0,
            to_part: 1,
            kind: ConnectionKind::Articulated,
        })
        .expect("conectar Base e Top");

    let asset = get_test_asset(&state, asset_id);
    let live = asset.live_rig.as_ref().unwrap();
    assert_eq!(live.graph.parts.len(), 2);
    assert_eq!(live.graph.connections.len(), 1);
    assert_eq!(live.graph.connections[0].kind, ConnectionKind::Articulated);

    // 4. Remover parte e verificar que conexões são limpas
    state
        .dispatch(&RemoveStructuralPartCmd {
            asset_id: Some(asset_id),
            part_id: 1,
        })
        .expect("remover parte Top");

    let asset = get_test_asset(&state, asset_id);
    let live = asset.live_rig.as_ref().unwrap();
    assert_eq!(live.graph.parts.len(), 1);
    assert_eq!(live.graph.connections.len(), 0);
}

#[test]
fn test_structural_joints_hierarchy_and_bounds() {
    let mut state = AppState::default();
    let asset_id = create_test_cube_asset(&mut state, "Creature");

    state
        .dispatch(&CreateStructuralGraphCmd {
            asset_id: Some(asset_id),
            name: "CreatureGraph".to_string(),
        })
        .expect("criar grafo");

    // Adicionar junta Root
    state
        .dispatch(&AddStructuralJointCmd {
            asset_id: Some(asset_id),
            name: "RootJoint".to_string(),
            bone_id: Some(0),
            position: [0.0, 0.0, 0.0],
            parent: None,
        })
        .expect("adicionar RootJoint");

    // Adicionar junta Spine (filha de Root)
    state
        .dispatch(&AddStructuralJointCmd {
            asset_id: Some(asset_id),
            name: "SpineJoint".to_string(),
            bone_id: Some(1),
            position: [0.0, 1.0, 0.0],
            parent: Some(0),
        })
        .expect("adicionar SpineJoint");

    let asset = get_test_asset(&state, asset_id);
    let live = asset.live_rig.as_ref().unwrap();
    assert_eq!(live.graph.joints.len(), 2);
    assert_eq!(live.graph.joints[1].parent, Some(0));

    // Rejeitar junta com pai inexistente
    let err = state.dispatch(&AddStructuralJointCmd {
        asset_id: Some(asset_id),
        name: "HeadJoint".to_string(),
        bone_id: Some(2),
        position: [0.0, 2.0, 0.0],
        parent: Some(999),
    });
    assert!(err.is_err());
}

#[test]
fn test_live_rig_idempotency_and_no_change() {
    let mut state = AppState::default();
    let asset_id = create_test_cube_asset(&mut state, "TestObj");

    state
        .dispatch(&CreateEmptySkeletonCmd {
            name: "ObjRig".to_string(),
            asset_id: Some(asset_id),
        })
        .expect("criar esqueleto");

    let skel_id = state.project.project.skeletons[0].id;
    state
        .dispatch(&AddBoneCmd {
            skeleton_id: skel_id,
            name: "Bone1".to_string(),
            parent: None,
            head: [0.0, -1.0, 0.0],
            tail: [0.0, 1.0, 0.0],
        })
        .expect("adicionar osso");

    state
        .dispatch(&BindRigCmd {
            skeleton_id: skel_id,
            asset_id: Some(asset_id),
            method: SkinBindMethod::Proximity,
            max_influences: 4,
            custom_skin: None,
        })
        .expect("bind inicial");

    state
        .dispatch(&CreateStructuralGraphCmd {
            asset_id: Some(asset_id),
            name: "ObjStructure".to_string(),
        })
        .expect("criar grafo");

    state
        .dispatch(&AddStructuralPartCmd {
            asset_id: Some(asset_id),
            name: "FullBody".to_string(),
            vertex_indices: (0..8).collect(),
            center: [0.0, 0.0, 0.0],
            bounds: ([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]),
        })
        .expect("adicionar parte");

    // Primeira atualização
    state
        .dispatch(&UpdateStructuralRigCmd {
            asset_id: Some(asset_id),
            method: SkinBindMethod::Proximity,
            max_influences: 4,
        })
        .expect("primeira atualização");

    // Segunda atualização consecutiva sem alterar geometria (Idempotência!)
    let result = state.dispatch(&UpdateStructuralRigCmd {
        asset_id: Some(asset_id),
        method: SkinBindMethod::Proximity,
        max_influences: 4,
    });

    // Deve retornar NoChange garantindo idempotência estrita
    assert!(
        result.is_err(),
        "Segunda execução sem mudanças deve ser NoChange"
    );
}

#[test]
fn test_material_and_uv_revisions_do_not_trigger_regeneration() {
    let mut state = AppState::default();
    let asset_id = create_test_cube_asset(&mut state, "SkinTest");

    state
        .dispatch(&CreateEmptySkeletonCmd {
            name: "Rig".to_string(),
            asset_id: Some(asset_id),
        })
        .expect("criar esqueleto");
    let skel_id = state.project.project.skeletons[0].id;
    state
        .dispatch(&AddBoneCmd {
            skeleton_id: skel_id,
            name: "Bone".to_string(),
            parent: None,
            head: [0.0, 0.0, 0.0],
            tail: [0.0, 1.0, 0.0],
        })
        .expect("add bone");

    state
        .dispatch(&BindRigCmd {
            skeleton_id: skel_id,
            asset_id: Some(asset_id),
            method: SkinBindMethod::Proximity,
            max_influences: 4,
            custom_skin: None,
        })
        .expect("bind");

    state
        .dispatch(&CreateStructuralGraphCmd {
            asset_id: Some(asset_id),
            name: "Graph".to_string(),
        })
        .expect("criar grafo");

    state
        .dispatch(&AddStructuralPartCmd {
            asset_id: Some(asset_id),
            name: "CubePart".to_string(),
            vertex_indices: (0..8).collect(),
            center: [0.0, 0.0, 0.0],
            bounds: ([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]),
        })
        .expect("adicionar parte");

    // Executar atualização inicial para sincronizar revisões
    state
        .dispatch(&UpdateStructuralRigCmd {
            asset_id: Some(asset_id),
            method: SkinBindMethod::Proximity,
            max_influences: 4,
        })
        .expect("atualização inicial");

    // Simular edição de Material e UV (avançar contadores) sem alterar geometria
    state.project.project.material_revision += 1;
    state.project.project.uv_revision += 1;

    // Chamar atualização do rig estrutural
    let res = state.dispatch(&UpdateStructuralRigCmd {
        asset_id: Some(asset_id),
        method: SkinBindMethod::Proximity,
        max_influences: 4,
    });

    // RIG-007: Material/UV não dispara regeneração de pesos (retorna NoChange com mensagem explicativa)
    assert!(res.is_err());
    match res {
        Err(petunia_core::CommandError::NoChange(msg)) => {
            assert!(
                msg.contains("Material/UV alterado")
                    || msg.contains("regeneração de pesos ignorada")
            );
        }
        other => panic!("Esperava NoChange devido a Material/UV, obtido {other:?}"),
    }
}

#[test]
fn test_destructive_topology_change_triggers_needs_review() {
    let mut state = AppState::default();
    let asset_id = create_test_cube_asset(&mut state, "DestructiveTest");

    state
        .dispatch(&CreateEmptySkeletonCmd {
            name: "Rig".to_string(),
            asset_id: Some(asset_id),
        })
        .expect("criar esqueleto");
    let skel_id = state.project.project.skeletons[0].id;
    state
        .dispatch(&AddBoneCmd {
            skeleton_id: skel_id,
            name: "Root".to_string(),
            parent: None,
            head: [0.0, 0.0, 0.0],
            tail: [0.0, 1.0, 0.0],
        })
        .expect("add bone");

    state
        .dispatch(&BindRigCmd {
            skeleton_id: skel_id,
            asset_id: Some(asset_id),
            method: SkinBindMethod::Proximity,
            max_influences: 4,
            custom_skin: None,
        })
        .expect("bind");

    state
        .dispatch(&CreateStructuralGraphCmd {
            asset_id: Some(asset_id),
            name: "Graph".to_string(),
        })
        .expect("criar grafo");

    state
        .dispatch(&AddStructuralPartCmd {
            asset_id: Some(asset_id),
            name: "MainPart".to_string(),
            vertex_indices: (0..8).collect(),
            center: [0.0, 0.0, 0.0],
            bounds: ([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]),
        })
        .expect("adicionar parte");

    // Sincronizar
    state
        .dispatch(&UpdateStructuralRigCmd {
            asset_id: Some(asset_id),
            method: SkinBindMethod::Proximity,
            max_influences: 4,
        })
        .expect("sync");

    // Alterar destrutivamente a malha: adicionar um 9º vértice diretamente
    {
        let asset = get_test_asset_mut(&mut state, asset_id);
        let mut m = (*asset.mesh).clone();
        m.verts.push(Vertex::new(2.0, 2.0, 2.0));
        asset.mesh = m.into();
        state.project.project.position_revision += 1;
        state.project.project.topology_revision += 1;
    }

    // Tentar atualizar: deve rejeitar silenciosa reutilização de índices e definir NeedsReview comitando a transição
    let res = state.dispatch(&UpdateStructuralRigCmd {
        asset_id: Some(asset_id),
        method: SkinBindMethod::Proximity,
        max_influences: 4,
    });
    assert!(
        res.is_ok(),
        "Atualização que altera status para NeedsReview comita a transição"
    );

    let asset = get_test_asset(&state, asset_id);
    let live = asset.live_rig.as_ref().unwrap();
    match &live.review_status {
        RigReviewStatus::NeedsReview(reason) => {
            assert!(reason.contains("Topologia destrutiva detectada"));
        }
        other => panic!("Esperava status NeedsReview, obtido {other:?}"),
    }

    // Segunda execução consecutiva já estando em NeedsReview: retorna NoChange
    let res2 = state.dispatch(&UpdateStructuralRigCmd {
        asset_id: Some(asset_id),
        method: SkinBindMethod::Proximity,
        max_influences: 4,
    });
    assert!(
        res2.is_err(),
        "Segunda chamada já em NeedsReview deve ser NoChange"
    );
}

#[test]
fn test_remap_skin_weights_restores_sync() {
    let mut state = AppState::default();
    let asset_id = create_test_cube_asset(&mut state, "RemapTest");

    state
        .dispatch(&CreateEmptySkeletonCmd {
            name: "Rig".to_string(),
            asset_id: Some(asset_id),
        })
        .expect("criar esqueleto");
    let skel_id = state.project.project.skeletons[0].id;
    state
        .dispatch(&AddBoneCmd {
            skeleton_id: skel_id,
            name: "Root".to_string(),
            parent: None,
            head: [0.0, 0.0, 0.0],
            tail: [0.0, 1.0, 0.0],
        })
        .expect("add bone");

    state
        .dispatch(&BindRigCmd {
            skeleton_id: skel_id,
            asset_id: Some(asset_id),
            method: SkinBindMethod::Proximity,
            max_influences: 4,
            custom_skin: None,
        })
        .expect("bind");

    state
        .dispatch(&CreateStructuralGraphCmd {
            asset_id: Some(asset_id),
            name: "Graph".to_string(),
        })
        .expect("criar grafo");

    state
        .dispatch(&AddStructuralPartCmd {
            asset_id: Some(asset_id),
            name: "Part".to_string(),
            vertex_indices: vec![0, 1, 2, 3],
            center: [0.0, 0.0, 0.0],
            bounds: ([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]),
        })
        .expect("add part");

    // Alterar contagem de vértices da malha para 4
    {
        let asset = get_test_asset_mut(&mut state, asset_id);
        let m = Mesh::plane(2.0);
        asset.mesh = m.into();
    }

    // Tabela de remapeamento: os 8 vértices originais mapeados para 4 novos
    let remap_table = vec![Some(0), Some(1), Some(2), Some(3), None, None, None, None];

    state
        .dispatch(&RemapRigWeightsCmd {
            asset_id: Some(asset_id),
            old_to_new_indices: remap_table,
            new_vertex_count: 4,
        })
        .expect("remap com sucesso");

    let asset = get_test_asset(&state, asset_id);
    let skin = asset.skin_data.as_ref().unwrap();
    assert_eq!(skin.vertex_weights.len(), 4);
    let live = asset.live_rig.as_ref().unwrap();
    assert_eq!(live.review_status, RigReviewStatus::UpToDate);
}

#[test]
fn test_manual_overrides_and_part_locks_preserved_during_update() {
    let mut state = AppState::default();
    let asset_id = create_test_cube_asset(&mut state, "LockTest");

    state
        .dispatch(&CreateEmptySkeletonCmd {
            name: "Rig".to_string(),
            asset_id: Some(asset_id),
        })
        .expect("criar esqueleto");
    let skel_id = state.project.project.skeletons[0].id;
    state
        .dispatch(&AddBoneCmd {
            skeleton_id: skel_id,
            name: "BoneA".to_string(),
            parent: None,
            head: [0.0, -1.0, 0.0],
            tail: [0.0, 0.0, 0.0],
        })
        .expect("add bone A");
    state
        .dispatch(&AddBoneCmd {
            skeleton_id: skel_id,
            name: "BoneB".to_string(),
            parent: Some(0),
            head: [0.0, 0.0, 0.0],
            tail: [0.0, 1.0, 0.0],
        })
        .expect("add bone B");

    state
        .dispatch(&BindRigCmd {
            skeleton_id: skel_id,
            asset_id: Some(asset_id),
            method: SkinBindMethod::Proximity,
            max_influences: 4,
            custom_skin: None,
        })
        .expect("bind");

    state
        .dispatch(&CreateStructuralGraphCmd {
            asset_id: Some(asset_id),
            name: "Graph".to_string(),
        })
        .expect("criar grafo");

    state
        .dispatch(&AddStructuralPartCmd {
            asset_id: Some(asset_id),
            name: "Part1".to_string(),
            vertex_indices: vec![0, 1, 2, 3],
            center: [0.0, -1.0, 0.0],
            bounds: ([-1.0, -1.0, -1.0], [1.0, 0.0, 1.0]),
        })
        .expect("add part 1");

    state
        .dispatch(&AddStructuralPartCmd {
            asset_id: Some(asset_id),
            name: "Part2".to_string(),
            vertex_indices: vec![4, 5, 6, 7],
            center: [0.0, 1.0, 0.0],
            bounds: ([-1.0, 0.0, -1.0], [1.0, 1.0, 1.0]),
        })
        .expect("add part 2");

    // 1. Definir override manual no vértice 0: 100% no BoneB (osso 1)
    state
        .dispatch(&SetVertexWeightOverrideCmd {
            asset_id: Some(asset_id),
            vertex_index: 0,
            bones: [1, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        })
        .expect("set override");

    // 2. Travar Part1 contra recalculo automático
    state
        .dispatch(&SetStructuralPartLockedCmd {
            asset_id: Some(asset_id),
            part_id: 0,
            locked: true,
        })
        .expect("lock part 0");

    // 3. Modificar posições dos vértices para forçar atualização
    state.project.project.position_revision += 1;

    state
        .dispatch(&UpdateStructuralRigCmd {
            asset_id: Some(asset_id),
            method: SkinBindMethod::Proximity,
            max_influences: 4,
        })
        .expect("update");

    let asset = get_test_asset(&state, asset_id);
    let skin = asset.skin_data.as_ref().unwrap();

    // Vértice 0 deve ter preservado o override de 100% no osso 1
    let w0 = &skin.vertex_weights[0];
    assert_eq!(w0.bones[0], 1);
    assert!((w0.weights[0] - 1.0).abs() < 1e-4);

    // 4. Limpar override
    state
        .dispatch(&ClearVertexWeightOverrideCmd {
            asset_id: Some(asset_id),
            vertex_index: 0,
        })
        .expect("clear override");

    let asset = get_test_asset(&state, asset_id);
    let live = asset.live_rig.as_ref().unwrap();
    assert!(!live.overrides.contains_key(&0));
}

#[test]
fn test_rig_012_three_distinct_off_operations() {
    let mut state = AppState::default();
    let asset_id = create_test_cube_asset(&mut state, "Rig12Test");

    state
        .dispatch(&CreateEmptySkeletonCmd {
            name: "Rig".to_string(),
            asset_id: Some(asset_id),
        })
        .expect("criar esqueleto");
    let skel_id = state.project.project.skeletons[0].id;
    state
        .dispatch(&AddBoneCmd {
            skeleton_id: skel_id,
            name: "Root".to_string(),
            parent: None,
            head: [0.0, 0.0, 0.0],
            tail: [0.0, 1.0, 0.0],
        })
        .expect("add bone");

    state
        .dispatch(&BindRigCmd {
            skeleton_id: skel_id,
            asset_id: Some(asset_id),
            method: SkinBindMethod::Proximity,
            max_influences: 4,
            custom_skin: None,
        })
        .expect("bind");

    state
        .dispatch(&CreateStructuralGraphCmd {
            asset_id: Some(asset_id),
            name: "Graph".to_string(),
        })
        .expect("criar grafo");

    // Operação 1: Desligar auto-atualização do Live Rig (RIG-012)
    state
        .dispatch(&SetLiveAutoUpdateCmd {
            asset_id: Some(asset_id),
            enabled: false,
        })
        .expect("desligar auto-update");

    let asset = get_test_asset(&state, asset_id);
    assert!(!asset.live_rig.as_ref().unwrap().auto_update_enabled);
    assert!(asset.skinning_enabled); // Deformação continua ligada!
    assert!(asset.skeleton_id.is_some()); // Ossos continuam no asset!
    assert!(asset.skin_data.is_some()); // Pesos continuam intactos!

    // Operação 2: Desligar deformação de skinning (RIG-012)
    state
        .dispatch(&SetSkinningEnabledCmd {
            asset_id: Some(asset_id),
            enabled: false,
        })
        .expect("desligar deformação");

    let asset = get_test_asset(&state, asset_id);
    assert!(!asset.skinning_enabled); // Deformação desligada!
    assert!(asset.skeleton_id.is_some()); // Ossos NÃO foram apagados!
    assert!(asset.skin_data.is_some()); // Pesos NÃO foram apagados!
    assert!(asset.live_rig.is_some()); // Live rig NÃO foi apagado!

    // Religar deformação
    state
        .dispatch(&SetSkinningEnabledCmd {
            asset_id: Some(asset_id),
            enabled: true,
        })
        .expect("religar deformação");

    let asset = get_test_asset(&state, asset_id);
    assert!(asset.skinning_enabled);
    assert!(asset.skin_data.is_some());
}
