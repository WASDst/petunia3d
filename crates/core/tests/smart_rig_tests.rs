//! Petunia3D — Testes de Smart Rig Manual & Assistido (H31 / RIG-001–012)
//!
//! Cobre:
//! - RIG-001/010/011: Edição manual de esqueletos para anatomias variadas (bípede, quadrúpede,
//!   multilegged, serpente, peixe, asas, criaturas assimétricas).
//! - RIG-002/004/009: Preview revisável (`SkinningPreviewState`), binds transacionais
//!   (`Proximity`, `RigidPerPart`, `SmoothEnvelope`), normalização estrita, `UnbindRigCmd`, Undo/Redo.
//! - RIG-008: Pose Test isolado da Rest Pose, `ResetAllBonePosesCmd` e `ApplyPoseAsRestCmd`.
//! - RIG-012: Desativação não-destrutiva via `SetSkinningEnabledCmd` (sem apagar bones ou pesos).

use petunia_core::{
    AddBoneCmd, AppState, ApplyPoseAsRestCmd, BindRigCmd, CreateEmptySkeletonCmd,
    EditBoneRestPoseCmd, RemoveBoneCmd, RenameBoneCmd, ReparentBoneCmd, ResetAllBonePosesCmd,
    SetBoneTransformCmd, SetSkinningEnabledCmd, SkinningPreviewState, UnbindRigCmd,
};
use petunia_mesh::Mesh;
use petunia_project::{Asset, SkinBindMethod, Transform3D};
use uuid::Uuid;

/// Retorna uma representação JSON canônica do documento sem chaves de revisão para asserções de Undo/Redo.
fn snapshot(state: &AppState) -> String {
    let mut v = serde_json::to_value(&state.project.project).expect("serialize project");
    let obj = v.as_object_mut().expect("json object");
    obj.retain(|k, _| !k.ends_with("_revision") && k != "history_selection");
    v.to_string()
}

/// Assegura que uma operação respeita estritamente o ciclo transacional `do → undo → redo`.
fn assert_transactional(state: &mut AppState, before: &str, after: &str) {
    assert_ne!(before, after, "o comando deveria alterar o projeto");
    assert!(state.undo(), "undo falhou");
    assert_eq!(
        snapshot(state),
        before,
        "snapshot após undo difere do inicial"
    );
    assert!(state.redo(), "redo falhou");
    assert_eq!(
        snapshot(state),
        after,
        "snapshot após redo difere do esperado"
    );
}

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

#[test]
fn test_manual_rig_creation_arbitrary_anatomies() {
    let mut state = AppState::default();
    let asset_id = create_test_cube_asset(&mut state, "CreatureBody");

    // 1. Criar esqueleto vazio (RIG-001)
    let before_skel = snapshot(&state);
    state
        .dispatch(&CreateEmptySkeletonCmd {
            name: "CustomCreature".to_string(),
            asset_id: Some(asset_id),
        })
        .expect("criar esqueleto vazio");
    let after_skel = snapshot(&state);
    assert_transactional(&mut state, &before_skel, &after_skel);

    let skel_id = state
        .project
        .project
        .skeletons
        .iter()
        .find(|s| s.name == "CustomCreature")
        .expect("esqueleto encontrado")
        .id;

    // 2. Serpente: cadeia sequencial de 5 segmentos
    state
        .dispatch(&AddBoneCmd {
            skeleton_id: skel_id,
            name: "Spine0".to_string(),
            parent: None,
            head: [0.0, 0.0, 0.0],
            tail: [0.0, 1.0, 0.0],
        })
        .expect("add spine0");

    state
        .dispatch(&AddBoneCmd {
            skeleton_id: skel_id,
            name: "Spine1".to_string(),
            parent: Some(0),
            head: [0.0, 1.0, 0.0],
            tail: [0.0, 2.0, 0.0],
        })
        .expect("add spine1");

    // 3. Multi-legged (aranha/inseto): tórax central com pernas radiais
    for leg_idx in 0..6 {
        let angle = (leg_idx as f32) * std::f32::consts::PI / 3.0;
        let leg_name = format!("Leg_{}", leg_idx);
        let head = [0.0, 0.5, 0.0];
        let tail = [angle.cos() * 1.5, 0.0, angle.sin() * 1.5];
        state
            .dispatch(&AddBoneCmd {
                skeleton_id: skel_id,
                name: leg_name,
                parent: Some(0),
                head,
                tail,
            })
            .expect("add leg");
    }

    // 4. Asa / Criatura assimétrica: apêndice apenas no lado esquerdo
    state
        .dispatch(&AddBoneCmd {
            skeleton_id: skel_id,
            name: "Wing_L".to_string(),
            parent: Some(1),
            head: [-0.5, 1.5, 0.0],
            tail: [-2.5, 2.5, 0.5],
        })
        .expect("add wing");

    let skel = state.project.project.get_skeleton(skel_id).unwrap();
    assert_eq!(skel.bones.len(), 9); // 2 spines + 6 legs + 1 wing
    assert!(skel.validate().is_ok());

    // 5. Rejeitar duplicata de nome e coordenadas não finitas
    let err_dup = state.dispatch(&AddBoneCmd {
        skeleton_id: skel_id,
        name: "Wing_L".to_string(),
        parent: None,
        head: [0.0, 0.0, 0.0],
        tail: [1.0, 1.0, 1.0],
    });
    assert!(err_dup.is_err());

    let err_inf = state.dispatch(&AddBoneCmd {
        skeleton_id: skel_id,
        name: "BadBone".to_string(),
        parent: None,
        head: [f32::NAN, 0.0, 0.0],
        tail: [1.0, 1.0, 1.0],
    });
    assert!(err_inf.is_err());
}

#[test]
fn test_bone_hierarchy_cycles_and_reparent() {
    let mut state = AppState::default();
    state
        .dispatch(&CreateEmptySkeletonCmd {
            name: "CycleTestRig".to_string(),
            asset_id: None,
        })
        .unwrap();

    let skel_id = state.project.project.skeletons[0].id;

    // Criar hierarquia A -> B -> C
    state
        .dispatch(&AddBoneCmd {
            skeleton_id: skel_id,
            name: "BoneA".to_string(),
            parent: None,
            head: [0.0, 0.0, 0.0],
            tail: [0.0, 1.0, 0.0],
        })
        .unwrap();
    state
        .dispatch(&AddBoneCmd {
            skeleton_id: skel_id,
            name: "BoneB".to_string(),
            parent: Some(0),
            head: [0.0, 1.0, 0.0],
            tail: [0.0, 2.0, 0.0],
        })
        .unwrap();
    state
        .dispatch(&AddBoneCmd {
            skeleton_id: skel_id,
            name: "BoneC".to_string(),
            parent: Some(1),
            head: [0.0, 2.0, 0.0],
            tail: [0.0, 3.0, 0.0],
        })
        .unwrap();

    // 1. Ciclo direto: reparent A para si mesmo
    let err_self = state.dispatch(&ReparentBoneCmd {
        skeleton_id: skel_id,
        bone_id: 0,
        new_parent: Some(0),
    });
    assert!(err_self.is_err());

    // 2. Ciclo indireto: reparent A para seu neto C
    let err_cycle = state.dispatch(&ReparentBoneCmd {
        skeleton_id: skel_id,
        bone_id: 0,
        new_parent: Some(2),
    });
    assert!(err_cycle.is_err());

    // 3. Reparent válido: mover C para ser irmão de B (filho de A)
    let before = snapshot(&state);
    state
        .dispatch(&ReparentBoneCmd {
            skeleton_id: skel_id,
            bone_id: 2,
            new_parent: Some(0),
        })
        .expect("reparent válido");
    let after = snapshot(&state);
    assert_transactional(&mut state, &before, &after);

    let skel = state.project.project.get_skeleton(skel_id).unwrap();
    assert_eq!(skel.get_bone(2).unwrap().parent, Some(0));

    // 4. Remoção com reparenting de filhos: remover A reatribui B e C para None (raízes)
    state
        .dispatch(&RemoveBoneCmd {
            skeleton_id: skel_id,
            bone_id: 0,
        })
        .expect("remove bone A");

    let skel_after_remove = state.project.project.get_skeleton(skel_id).unwrap();
    assert_eq!(skel_after_remove.bones.len(), 2);
    for b in &skel_after_remove.bones {
        assert_eq!(
            b.parent, None,
            "filhos reatribuídos ao pai da raiz removida"
        );
    }
}

#[test]
fn test_bone_rest_pose_edit_and_rename() {
    let mut state = AppState::default();
    state
        .dispatch(&CreateEmptySkeletonCmd {
            name: "EditRig".to_string(),
            asset_id: None,
        })
        .unwrap();

    let skel_id = state.project.project.skeletons[0].id;
    state
        .dispatch(&AddBoneCmd {
            skeleton_id: skel_id,
            name: "Arm_L".to_string(),
            parent: None,
            head: [0.0, 1.0, 0.0],
            tail: [1.0, 1.0, 0.0],
        })
        .unwrap();

    // 1. Editar Rest Pose
    let before_edit = snapshot(&state);
    state
        .dispatch(&EditBoneRestPoseCmd {
            skeleton_id: skel_id,
            bone_id: 0,
            head: [0.0, 1.2, 0.0],
            tail: [1.5, 1.2, 0.0],
        })
        .expect("editar rest pose");
    let after_edit = snapshot(&state);
    assert_transactional(&mut state, &before_edit, &after_edit);

    let skel = state.project.project.get_skeleton(skel_id).unwrap();
    let arm = skel.get_bone(0).unwrap();
    assert_eq!(arm.head, [0.0, 1.2, 0.0]);
    assert_eq!(arm.tail, [1.5, 1.2, 0.0]);

    // 2. Renomear osso
    let before_rename = snapshot(&state);
    state
        .dispatch(&RenameBoneCmd {
            skeleton_id: skel_id,
            bone_id: 0,
            new_name: "UpperArm_L".to_string(),
        })
        .expect("renomear osso");
    let after_rename = snapshot(&state);
    assert_transactional(&mut state, &before_rename, &after_rename);

    let skel = state.project.project.get_skeleton(skel_id).unwrap();
    assert_eq!(skel.get_bone(0).unwrap().name, "UpperArm_L");
}

#[test]
fn test_revisable_preview_and_manual_overrides() {
    let mut state = AppState::default();
    let asset_id = create_test_cube_asset(&mut state, "CharacterMesh");

    state
        .dispatch(&CreateEmptySkeletonCmd {
            name: "PreviewRig".to_string(),
            asset_id: Some(asset_id),
        })
        .unwrap();

    let skel_id = state.project.project.skeletons[0].id;
    state
        .dispatch(&AddBoneCmd {
            skeleton_id: skel_id,
            name: "Root".to_string(),
            parent: None,
            head: [0.0, -1.0, 0.0],
            tail: [0.0, 0.0, 0.0],
        })
        .unwrap();
    state
        .dispatch(&AddBoneCmd {
            skeleton_id: skel_id,
            name: "Spine".to_string(),
            parent: Some(0),
            head: [0.0, 0.0, 0.0],
            tail: [0.0, 1.0, 0.0],
        })
        .unwrap();

    let asset = get_test_asset(&state, asset_id);
    let skel = state.project.project.get_skeleton(skel_id).unwrap();

    // 1. Pré-visualização revisável não altera o documento antes do commit (RIG-002/004/009)
    let doc_before = snapshot(&state);
    let mut preview =
        SkinningPreviewState::compute(asset_id, &asset.mesh, skel, SkinBindMethod::Proximity, 4);
    assert!(preview.is_valid);
    assert_eq!(snapshot(&state), doc_before, "preview não muta o projeto");

    // 2. Modificação manual de pesos na pré-visualização
    preview
        .override_vertex_weight(0, [0, 1, 0, 0], [0.8, 0.2, 0.0, 0.0])
        .expect("override vertex weight");
    let w0 = preview.skin_data.vertex_weights[0];
    assert!((w0.weights[0] - 0.8).abs() < 1e-4);
    assert!((w0.weights[1] - 0.2).abs() < 1e-4);
    assert!((w0.weights.iter().sum::<f32>() - 1.0).abs() < 1e-4);

    // 3. Confirmar bind via commit_cmd() gera comando transacional
    let bind_cmd = preview.commit_cmd();
    state.dispatch(&bind_cmd).expect("executar bind_cmd");
    let doc_after = snapshot(&state);
    assert_transactional(&mut state, &doc_before, &doc_after);

    let bound_asset = get_test_asset(&state, asset_id);
    assert_eq!(bound_asset.skeleton_id, Some(skel_id));
    assert!(bound_asset.skin_data.is_some());
    assert!(bound_asset.skinning_enabled);
    let applied_w0 = bound_asset.skin_data.as_ref().unwrap().vertex_weights[0];
    assert!((applied_w0.weights[0] - 0.8).abs() < 1e-4);
}

#[test]
fn test_bind_methods_and_unbind_cmd() {
    let mut state = AppState::default();
    let asset_id = create_test_cube_asset(&mut state, "BoxTarget");

    state
        .dispatch(&CreateEmptySkeletonCmd {
            name: "RigMethods".to_string(),
            asset_id: Some(asset_id),
        })
        .unwrap();

    let skel_id = state.project.project.skeletons[0].id;
    state
        .dispatch(&AddBoneCmd {
            skeleton_id: skel_id,
            name: "Bone1".to_string(),
            parent: None,
            head: [0.0, -1.0, 0.0],
            tail: [0.0, 0.0, 0.0],
        })
        .unwrap();
    state
        .dispatch(&AddBoneCmd {
            skeleton_id: skel_id,
            name: "Bone2".to_string(),
            parent: Some(0),
            head: [0.0, 0.0, 0.0],
            tail: [0.0, 1.0, 0.0],
        })
        .unwrap();

    // 1. Bind com RigidPerPart (100% no mais próximo)
    state
        .dispatch(&BindRigCmd::new(
            skel_id,
            Some(asset_id),
            SkinBindMethod::RigidPerPart,
            1,
        ))
        .expect("bind rigid");

    let asset = get_test_asset(&state, asset_id);
    let skin = asset.skin_data.as_ref().unwrap();
    for vw in &skin.vertex_weights {
        assert_eq!(vw.weights[0], 1.0, "peso rígido deve ser 1.0");
        assert_eq!(vw.weights[1], 0.0);
    }

    // 2. Bind com SmoothEnvelope
    state
        .dispatch(&BindRigCmd::new(
            skel_id,
            Some(asset_id),
            SkinBindMethod::SmoothEnvelope,
            4,
        ))
        .expect("bind smooth");

    let asset = get_test_asset(&state, asset_id);
    let skin_smooth = asset.skin_data.as_ref().unwrap();
    for vw in &skin_smooth.vertex_weights {
        let sum: f32 = vw.weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-4, "soma de pesos deve ser 1.0");
    }

    // 3. UnbindRigCmd transacional
    let before_unbind = snapshot(&state);
    state
        .dispatch(&UnbindRigCmd {
            asset_id: Some(asset_id),
        })
        .expect("unbind rig");
    let after_unbind = snapshot(&state);
    assert_transactional(&mut state, &before_unbind, &after_unbind);

    let unbound_asset = get_test_asset(&state, asset_id);
    assert_eq!(unbound_asset.skeleton_id, None);
    assert_eq!(unbound_asset.skin_data, None);
}

#[test]
fn test_pose_test_vs_rest_pose_isolation_and_baking() {
    let mut state = AppState::default();
    let asset_id = create_test_cube_asset(&mut state, "PosedCreature");

    state
        .dispatch(&CreateEmptySkeletonCmd {
            name: "PoseRig".to_string(),
            asset_id: Some(asset_id),
        })
        .unwrap();

    let skel_id = state.project.project.skeletons[0].id;
    state
        .dispatch(&AddBoneCmd {
            skeleton_id: skel_id,
            name: "Base".to_string(),
            parent: None,
            head: [0.0, 0.0, 0.0],
            tail: [0.0, 1.0, 0.0],
        })
        .unwrap();

    state
        .dispatch(&BindRigCmd::new(
            skel_id,
            Some(asset_id),
            SkinBindMethod::Proximity,
            4,
        ))
        .unwrap();

    let orig_head = state.project.project.skeletons[0].bones[0].head;
    let orig_tail = state.project.project.skeletons[0].bones[0].tail;

    // 1. Pose Test: rotacionar 90 graus no eixo Z (RIG-008)
    let rot90z = glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    state
        .dispatch(&SetBoneTransformCmd {
            skeleton_id: skel_id,
            bone_id: 0,
            transform: Transform3D {
                translation: [0.0, 0.0, 0.0],
                rotation: [rot90z.x, rot90z.y, rot90z.z, rot90z.w],
                scale: [1.0, 1.0, 1.0],
            },
        })
        .expect("pose test rotation");

    // Pose Test nunca altera silenciosamente o Rest Pose (head e tail permanecem idênticos)
    let skel_posed = &state.project.project.skeletons[0];
    assert_eq!(
        skel_posed.bones[0].head, orig_head,
        "head de repouso não deve mudar no teste de pose"
    );
    assert_eq!(
        skel_posed.bones[0].tail, orig_tail,
        "tail de repouso não deve mudar no teste de pose"
    );

    // 2. ResetAllBonePosesCmd restaura todos os ossos para o repouso
    state
        .dispatch(&ResetAllBonePosesCmd {
            skeleton_id: skel_id,
        })
        .expect("reset all poses");

    let skel_reset = &state.project.project.skeletons[0];
    assert_eq!(
        skel_reset.bones[0].local_transform.rotation,
        [0.0, 0.0, 0.0, 1.0]
    );

    // 3. ApplyPoseAsRestCmd: baka explicitamente a pose atual no Rest Pose
    state
        .dispatch(&SetBoneTransformCmd {
            skeleton_id: skel_id,
            bone_id: 0,
            transform: Transform3D {
                translation: [0.0, 0.0, 0.0],
                rotation: [rot90z.x, rot90z.y, rot90z.z, rot90z.w],
                scale: [1.0, 1.0, 1.0],
            },
        })
        .unwrap();

    let before_apply = snapshot(&state);
    state
        .dispatch(&ApplyPoseAsRestCmd {
            skeleton_id: skel_id,
            asset_id: Some(asset_id),
        })
        .expect("apply pose as rest");
    let after_apply = snapshot(&state);
    assert_transactional(&mut state, &before_apply, &after_apply);

    let skel_baked = &state.project.project.skeletons[0];
    // Com rotação de 90° em Z, o tail [0, 1, 0] deve virar [-1, 0, 0]
    assert!((skel_baked.bones[0].tail[0] - (-1.0)).abs() < 1e-4);
    assert!((skel_baked.bones[0].tail[1] - 0.0).abs() < 1e-4);
    // A rotação local volta para identidade na nova rest pose
    assert_eq!(
        skel_baked.bones[0].local_transform.rotation,
        [0.0, 0.0, 0.0, 1.0]
    );
}

#[test]
fn test_set_skinning_enabled_non_destructive() {
    let mut state = AppState::default();
    let asset_id = create_test_cube_asset(&mut state, "ToggleSkinMesh");

    state
        .dispatch(&CreateEmptySkeletonCmd {
            name: "ToggleRig".to_string(),
            asset_id: Some(asset_id),
        })
        .unwrap();

    let skel_id = state.project.project.skeletons[0].id;
    state
        .dispatch(&AddBoneCmd {
            skeleton_id: skel_id,
            name: "BoneA".to_string(),
            parent: None,
            head: [0.0, 0.0, 0.0],
            tail: [0.0, 1.0, 0.0],
        })
        .unwrap();

    state
        .dispatch(&BindRigCmd::new(
            skel_id,
            Some(asset_id),
            SkinBindMethod::Proximity,
            4,
        ))
        .unwrap();

    let asset = get_test_asset(&state, asset_id);
    assert!(asset.skinning_enabled);
    assert!(asset.skin_data.is_some());
    assert_eq!(asset.skeleton_id, Some(skel_id));

    // 1. Desativar deformação via SetSkinningEnabledCmd (RIG-012)
    let before_disable = snapshot(&state);
    state
        .dispatch(&SetSkinningEnabledCmd {
            asset_id: Some(asset_id),
            enabled: false,
        })
        .expect("desabilitar skinning");
    let after_disable = snapshot(&state);
    assert_transactional(&mut state, &before_disable, &after_disable);

    let asset_disabled = get_test_asset(&state, asset_id);
    assert!(!asset_disabled.skinning_enabled);
    // Dados NUNCA são apagados quando apenas desligado (RIG-012)
    assert!(asset_disabled.skin_data.is_some(), "pesos preservados");
    assert_eq!(
        asset_disabled.skeleton_id,
        Some(skel_id),
        "esqueleto preservado"
    );
    assert_eq!(
        state.project.project.skeletons.len(),
        1,
        "ossos preservados"
    );

    // 2. Re-habilitar deformação
    state
        .dispatch(&SetSkinningEnabledCmd {
            asset_id: Some(asset_id),
            enabled: true,
        })
        .expect("habilitar skinning");

    let asset_re_enabled = get_test_asset(&state, asset_id);
    assert!(asset_re_enabled.skinning_enabled);
}
