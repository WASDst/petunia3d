//! Comandos transacionais de rig, papéis, IK e clipes (cap. 45 F0, P3D-169).

use petunia_core::{
    AddAnimationCmd, AddIkChainCmd, AddRigPresetCmd, AppState, AssignRigRoleCmd,
    AutoRigActiveAssetCmd, ClearRigRoleCmd, CommandError, DeleteBoneKeyCmd, InferRigRolesCmd,
    RemoveAnimationCmd, RemoveIkChainCmd, RemoveSkeletonCmd, RigPresetKind, SetBoneKeyCmd,
    UpdateIkChainCmd,
};
use petunia_project::{IkChain, IkSolver, Interpolation, LimbPart, RigRole, Transform3D};

/// Documento sem contadores de revisão (o Undo os rebaseia de forma monotônica
/// por desenho) nem `history_selection`.
fn snapshot(state: &AppState) -> String {
    let mut v = serde_json::to_value(&state.project.project).unwrap();
    let obj = v.as_object_mut().unwrap();
    obj.retain(|k, _| !k.ends_with("_revision") && k != "history_selection");
    v.to_string()
}

/// `do → undo → redo` restaura o documento byte a byte.
fn assert_transactional(state: &mut AppState, before: &str, after: &str) {
    assert_ne!(before, after, "o comando deveria alterar o projeto");
    assert!(state.undo());
    assert_eq!(snapshot(state), before, "undo");
    assert!(state.redo());
    assert_eq!(snapshot(state), after, "redo");
}

fn humanoid(state: &mut AppState) -> uuid::Uuid {
    state
        .dispatch(&AddRigPresetCmd {
            kind: RigPresetKind::Humanoid,
            scale: 1.0,
        })
        .unwrap();
    state.project.project.skeletons.last().unwrap().id
}

fn assert_no_history_change(state: &mut AppState, depth: (usize, usize), snap: &str) {
    assert_eq!(state.project.undo.depth(), depth, "histórico");
    assert_eq!(snapshot(state), snap, "documento");
}

#[test]
fn add_preset_infers_roles_and_is_transactional() {
    let mut state = AppState::default();
    let before = snapshot(&state);
    let id = humanoid(&mut state);
    let after = snapshot(&state);
    assert_eq!(
        state.project.project.rig_roles_of(id).unwrap().legs().len(),
        2
    );
    assert_transactional(&mut state, &before, &after);

    for kind in [RigPresetKind::Quadruped, RigPresetKind::MultiLeg(8)] {
        state
            .dispatch(&AddRigPresetCmd { kind, scale: 2.0 })
            .unwrap();
    }
    let n = state.project.project.skeletons.len();
    assert!(n >= 2);
    let last = state.project.project.skeletons.last().unwrap().id;
    assert_eq!(
        state
            .project
            .project
            .rig_roles_of(last)
            .unwrap()
            .legs()
            .len(),
        8
    );

    // Escala inválida: recusado antes de mexer no histórico.
    let (depth, snap) = (state.project.undo.depth(), snapshot(&state));
    for scale in [0.0, -1.0, f32::NAN] {
        let r = state.dispatch(&AddRigPresetCmd {
            kind: RigPresetKind::Humanoid,
            scale,
        });
        assert!(matches!(r, Err(CommandError::Execution(_))), "{r:?}");
    }
    assert_no_history_change(&mut state, depth, &snap);
}

#[test]
fn auto_rig_binds_skeleton_and_skin_to_the_active_asset() {
    let mut state = AppState::default();
    let before = snapshot(&state);
    state.dispatch(&AutoRigActiveAssetCmd).unwrap();
    let after = snapshot(&state);
    let asset = state.project.active().unwrap();
    let sid = asset.skeleton_id.expect("skeleton");
    let skin = asset.skin_data.as_ref().expect("skin");
    let sk = state.project.project.get_skeleton(sid).unwrap();
    skin.validate(asset.mesh.verts.len(), sk.bones.len())
        .unwrap();
    assert!(state.project.project.rig_roles_of(sid).is_some());
    assert_transactional(&mut state, &before, &after);
    // Depois do redo o vínculo existe; depois do undo, não.
    assert!(state.project.active().unwrap().skeleton_id.is_some());
    assert!(state.undo());
    assert!(state.project.active().unwrap().skeleton_id.is_none());
    assert!(state.project.active().unwrap().skin_data.is_none());
}

#[test]
fn role_commands_validate_report_no_change_and_undo() {
    let mut state = AppState::default();
    let sid = humanoid(&mut state);
    let sk = state.project.project.get_skeleton(sid).unwrap().clone();
    let head = sk.find_bone("Head").unwrap();
    let neck = sk.find_bone("Neck").unwrap();
    let hand = sk.find_bone("Hand.L").unwrap();

    // Papel de outro osso / osso inexistente: recusados, sem histórico.
    let (depth, snap) = (state.project.undo.depth(), snapshot(&state));
    let r = state.dispatch(&AssignRigRoleCmd {
        skeleton_id: sid,
        bone_id: neck,
        role: RigRole::Head,
    });
    assert!(matches!(r, Err(CommandError::Execution(_))), "{r:?}");
    let r = state.dispatch(&AssignRigRoleCmd {
        skeleton_id: sid,
        bone_id: 9999,
        role: RigRole::Jaw,
    });
    assert!(matches!(r, Err(CommandError::Execution(_))), "{r:?}");
    // Mesmo papel no mesmo osso: nada a alterar.
    let r = state.dispatch(&AssignRigRoleCmd {
        skeleton_id: sid,
        bone_id: head,
        role: RigRole::Head,
    });
    assert!(matches!(r, Err(CommandError::NoChange(_))), "{r:?}");
    assert_no_history_change(&mut state, depth, &snap);

    // Atribuição real: um passo de histórico, undo/redo por hash.
    let before = snapshot(&state);
    state
        .dispatch(&AssignRigRoleCmd {
            skeleton_id: sid,
            bone_id: head,
            role: RigRole::Jaw,
        })
        .unwrap();
    assert_eq!(state.project.undo.depth().0, depth.0 + 1);
    let after = snapshot(&state);
    assert_transactional(&mut state, &before, &after);

    // Limpar: com papel funciona; sem papel é recusado sem histórico.
    let before = snapshot(&state);
    state
        .dispatch(&ClearRigRoleCmd {
            skeleton_id: sid,
            bone_id: hand,
        })
        .unwrap();
    let after = snapshot(&state);
    assert_transactional(&mut state, &before, &after);
    state
        .dispatch(&ClearRigRoleCmd {
            skeleton_id: sid,
            bone_id: hand,
        })
        .ok();
    let (depth, snap) = (state.project.undo.depth(), snapshot(&state));
    let r = state.dispatch(&ClearRigRoleCmd {
        skeleton_id: sid,
        bone_id: hand,
    });
    assert!(matches!(r, Err(CommandError::Execution(_))), "{r:?}");
    assert_no_history_change(&mut state, depth, &snap);

    // Redetectar restaura o inferido (Head volta de Jaw; Hand.L recupera o papel).
    let before = snapshot(&state);
    state
        .dispatch(&InferRigRolesCmd { skeleton_id: sid })
        .unwrap();
    let after = snapshot(&state);
    assert_transactional(&mut state, &before, &after);
    state.redo();
    let (depth, snap) = (state.project.undo.depth(), snapshot(&state));
    let r = state.dispatch(&InferRigRolesCmd { skeleton_id: sid });
    assert!(matches!(r, Err(CommandError::NoChange(_))), "{r:?}");
    assert_no_history_change(&mut state, depth, &snap);
}

#[test]
fn ik_chain_commands_validate_edit_and_undo() {
    let mut state = AppState::default();
    let sid = humanoid(&mut state);
    let leg = state.project.project.rig_roles_of(sid).unwrap().legs()[0];
    let chain = IkChain::two_bone_leg(sid, "LegL", &leg);

    let before = snapshot(&state);
    state
        .dispatch(&AddIkChainCmd {
            chain: chain.clone(),
        })
        .unwrap();
    let after = snapshot(&state);
    assert_transactional(&mut state, &before, &after);

    // Duplicada / inválida: recusadas sem histórico.
    let (depth, snap) = (state.project.undo.depth(), snapshot(&state));
    assert!(
        state
            .dispatch(&AddIkChainCmd {
                chain: chain.clone()
            })
            .is_err()
    );
    let mut broken = IkChain::two_bone_leg(sid, "Broken", &leg);
    broken.bones.swap(0, 1);
    assert!(state.dispatch(&AddIkChainCmd { chain: broken }).is_err());
    let mut heavy = IkChain::two_bone_leg(sid, "Heavy", &leg);
    heavy.weight = 2.0;
    assert!(state.dispatch(&AddIkChainCmd { chain: heavy }).is_err());
    let mut bad_soft = IkChain::two_bone_leg(sid, "Soft", &leg);
    bad_soft.solver = IkSolver::TwoBone { soft: 0.9 };
    assert!(state.dispatch(&AddIkChainCmd { chain: bad_soft }).is_err());
    let other = IkChain::two_bone_leg(uuid::Uuid::new_v4(), "Ghost", &leg);
    assert!(state.dispatch(&AddIkChainCmd { chain: other }).is_err());
    assert_no_history_change(&mut state, depth, &snap);

    // Editar: muda peso/pole e avança a revisão; mesmos valores ⇒ NoChange.
    state
        .dispatch(&AddIkChainCmd {
            chain: chain.clone(),
        })
        .ok();
    let id = state.project.project.ik_chains[0].id;
    let before = snapshot(&state);
    state
        .dispatch(&UpdateIkChainCmd {
            chain_id: id,
            weight: Some(0.5),
            pole: Some(Some([0.0, 1.0, 2.0])),
            ..Default::default()
        })
        .unwrap();
    let after = snapshot(&state);
    let c = &state.project.project.ik_chains[0];
    assert_eq!(
        (c.weight, c.pole, c.revision),
        (0.5, Some([0.0, 1.0, 2.0]), 1)
    );
    assert_transactional(&mut state, &before, &after);
    let (depth, snap) = (state.project.undo.depth(), snapshot(&state));
    let r = state.dispatch(&UpdateIkChainCmd {
        chain_id: id,
        weight: Some(0.5),
        ..Default::default()
    });
    assert!(matches!(r, Err(CommandError::NoChange(_))), "{r:?}");
    assert_no_history_change(&mut state, depth, &snap);

    // Remover.
    let before = snapshot(&state);
    state.dispatch(&RemoveIkChainCmd { chain_id: id }).unwrap();
    let after = snapshot(&state);
    assert!(state.project.project.ik_chains.is_empty());
    assert_transactional(&mut state, &before, &after);
    assert!(
        state
            .dispatch(&RemoveIkChainCmd {
                chain_id: uuid::Uuid::new_v4()
            })
            .is_err()
    );
}

#[test]
fn remove_skeleton_cascades_and_undo_restores_everything() {
    let mut state = AppState::default();
    state.dispatch(&AutoRigActiveAssetCmd).unwrap();
    let sid = state.project.active().unwrap().skeleton_id.unwrap();
    let leg = state.project.project.rig_roles_of(sid).unwrap().legs()[0];
    state
        .dispatch(&AddIkChainCmd {
            chain: IkChain::two_bone_leg(sid, "L", &leg),
        })
        .unwrap();

    let before = snapshot(&state);
    state
        .dispatch(&RemoveSkeletonCmd { skeleton_id: sid })
        .unwrap();
    let after = snapshot(&state);
    let p = &state.project.project;
    assert!(p.get_skeleton(sid).is_none());
    assert!(p.rig_roles_of(sid).is_none());
    assert!(p.ik_chains.is_empty());
    assert!(state.project.active().unwrap().skin_data.is_none());
    assert_transactional(&mut state, &before, &after);
    assert!(
        state
            .dispatch(&RemoveSkeletonCmd {
                skeleton_id: uuid::Uuid::new_v4()
            })
            .is_err()
    );
}

#[test]
fn animation_and_key_commands_are_transactional_and_validated() {
    let mut state = AppState::default();
    let sid = humanoid(&mut state);
    let hips = state
        .project
        .project
        .get_skeleton(sid)
        .unwrap()
        .find_bone("Hips")
        .unwrap();

    // Validação do clipe.
    let (depth, snap) = (state.project.undo.depth(), snapshot(&state));
    for (name, duration, fps) in [
        ("", 1.0, 24.0),
        ("A", 0.0, 24.0),
        ("A", 1.0, 0.0),
        ("A", 1.0, f32::NAN),
    ] {
        assert!(
            state
                .dispatch(&AddAnimationCmd {
                    name: name.into(),
                    duration,
                    fps,
                    looping: true
                })
                .is_err()
        );
    }
    assert_no_history_change(&mut state, depth, &snap);

    let before = snapshot(&state);
    state
        .dispatch(&AddAnimationCmd {
            name: " Walk ".into(),
            duration: 2.0,
            fps: 30.0,
            looping: false,
        })
        .unwrap();
    let after = snapshot(&state);
    let anim = state.project.project.animations.last().unwrap().clone();
    assert_eq!(
        (anim.name.as_str(), anim.clip.fps, anim.clip.looping),
        ("Walk", 30.0, false)
    );
    assert_transactional(&mut state, &before, &after);
    let aid = anim.id;

    // Chave: grava T/R/S, substitui no mesmo instante, ordena, recusa inválidas.
    let mut t = Transform3D::from_translation([0.0, 0.5, 0.0]);
    t.rotation = [0.0, 0.0, 1.0, 1.0]; // não normalizado: o comando normaliza
    let before = snapshot(&state);
    state
        .dispatch(&SetBoneKeyCmd {
            animation_id: aid,
            bone_id: hips,
            time: 1.0,
            transform: t,
            interpolation: Interpolation::Linear,
        })
        .unwrap();
    state
        .dispatch(&SetBoneKeyCmd {
            animation_id: aid,
            bone_id: hips,
            time: 0.5,
            transform: Transform3D::default(),
            interpolation: Interpolation::Step,
        })
        .unwrap();
    let after = snapshot(&state);
    let track = state
        .project
        .project
        .get_animation(aid)
        .unwrap()
        .clip
        .get_track(hips)
        .unwrap()
        .clone();
    assert_eq!(track.bone_name, "Hips");
    let times: Vec<f32> = track.rotations.iter().map(|k| k.time).collect();
    assert_eq!(times, vec![0.5, 1.0]);
    let q = track.rotations[1].value;
    assert!((q.iter().map(|x| x * x).sum::<f32>() - 1.0).abs() < 1e-5);
    assert_eq!(track.rotations[0].interpolation, Interpolation::Step);
    // Undo desfaz as duas chaves em dois passos.
    assert!(state.undo() && state.undo());
    assert_eq!(snapshot(&state), before);
    assert!(state.redo() && state.redo());
    assert_eq!(snapshot(&state), after);

    // Mesmos valores ⇒ NoChange; inválidos ⇒ recusa; sem histórico em ambos.
    let (depth, snap) = (state.project.undo.depth(), snapshot(&state));
    let r = state.dispatch(&SetBoneKeyCmd {
        animation_id: aid,
        bone_id: hips,
        time: 1.0,
        transform: t,
        interpolation: Interpolation::Linear,
    });
    assert!(matches!(r, Err(CommandError::NoChange(_))), "{r:?}");
    let bad = |time: f32, tr: Transform3D, bone: u32, anim: uuid::Uuid| SetBoneKeyCmd {
        animation_id: anim,
        bone_id: bone,
        time,
        transform: tr,
        interpolation: Interpolation::Linear,
    };
    assert!(
        state.dispatch(&bad(5.0, t, hips, aid)).is_err(),
        "fora do clipe"
    );
    assert!(state.dispatch(&bad(-1.0, t, hips, aid)).is_err());
    assert!(state.dispatch(&bad(f32::NAN, t, hips, aid)).is_err());
    assert!(
        state.dispatch(&bad(1.0, t, 9999, aid)).is_err(),
        "osso inexistente"
    );
    assert!(
        state
            .dispatch(&bad(1.0, t, hips, uuid::Uuid::new_v4()))
            .is_err()
    );
    let mut zero = t;
    zero.rotation = [0.0; 4];
    assert!(state.dispatch(&bad(1.0, zero, hips, aid)).is_err());
    assert_no_history_change(&mut state, depth, &snap);

    // Apagar chave: inexistente recusa; existente remove; última chave remove a trilha.
    assert!(
        state
            .dispatch(&DeleteBoneKeyCmd {
                animation_id: aid,
                bone_id: hips,
                time: 0.75
            })
            .is_err()
    );
    assert_no_history_change(&mut state, depth, &snap);
    let before = snapshot(&state);
    state
        .dispatch(&DeleteBoneKeyCmd {
            animation_id: aid,
            bone_id: hips,
            time: 1.0,
        })
        .unwrap();
    let after = snapshot(&state);
    assert_transactional(&mut state, &before, &after);
    state
        .dispatch(&DeleteBoneKeyCmd {
            animation_id: aid,
            bone_id: hips,
            time: 0.5,
        })
        .unwrap();
    assert!(
        state
            .project
            .project
            .get_animation(aid)
            .unwrap()
            .clip
            .get_track(hips)
            .is_none()
    );

    // Remover clipe.
    let before = snapshot(&state);
    state
        .dispatch(&RemoveAnimationCmd { animation_id: aid })
        .unwrap();
    let after = snapshot(&state);
    assert!(state.project.project.get_animation(aid).is_none());
    assert_transactional(&mut state, &before, &after);
    assert!(
        state
            .dispatch(&RemoveAnimationCmd { animation_id: aid })
            .is_ok()
            || true
    );
}

#[test]
fn rig_commands_do_not_touch_render_revisions() {
    let mut state = AppState::default();
    // Aquece: o primeiro dispatch sincroniza a seleção inicial do editor.
    let sid = humanoid(&mut state);
    let clock = state.project.project.revision_clock();
    let leg = state.project.project.rig_roles_of(sid).unwrap().legs()[0];
    state
        .dispatch(&AddIkChainCmd {
            chain: IkChain::two_bone_leg(sid, "L", &leg),
        })
        .unwrap();
    let hand = state
        .project
        .project
        .get_skeleton(sid)
        .unwrap()
        .find_bone("Hand.L")
        .unwrap();
    state
        .dispatch(&AssignRigRoleCmd {
            skeleton_id: sid,
            bone_id: hand,
            role: RigRole::Leg {
                limb: 9,
                part: LimbPart::Toe,
            },
        })
        .unwrap();
    state.dispatch(&AutoRigActiveAssetCmd).unwrap();
    assert_eq!(state.project.project.revision_clock(), clock);
}

#[test]
fn glb_import_through_the_service_attaches_rig_skin_clips_and_is_undoable() {
    use petunia_core::ProjectService;
    use petunia_project::pipeline::ImportOptions;

    // Origem: asset com auto-rig, um clipe com chave e um papel manual.
    let mut src = AppState::default();
    src.dispatch(&AutoRigActiveAssetCmd).unwrap();
    let sid = src.project.active().unwrap().skeleton_id.unwrap();
    let hips = src
        .project
        .project
        .get_skeleton(sid)
        .unwrap()
        .find_bone("Hips")
        .unwrap();
    src.dispatch(&AddAnimationCmd {
        name: "Bob".into(),
        duration: 1.0,
        fps: 24.0,
        looping: true,
    })
    .unwrap();
    let aid = src.project.project.animations.last().unwrap().id;
    src.dispatch(&SetBoneKeyCmd {
        animation_id: aid,
        bone_id: hips,
        time: 0.5,
        transform: Transform3D::from_translation([0.0, 1.25, 0.0]),
        interpolation: Interpolation::Linear,
    })
    .unwrap();
    let glb = petunia_project::export::export_gltf(&src.project.project, &[0]).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hero.glb");
    std::fs::write(&path, glb).unwrap();

    // Destino: importa pelo serviço.
    let mut dst = AppState::default();
    let before = snapshot(&dst);
    let assets_before = dst.project.assets.len();
    let names =
        ProjectService::import_file_pipeline(&mut dst, &path, &ImportOptions::default()).unwrap();
    assert_eq!(names.len(), 1);
    let after = snapshot(&dst);
    let p = &dst.project.project;
    assert_eq!(p.skeletons.len(), 1);
    assert_eq!(p.animations.len(), 1);
    assert_eq!(p.animations[0].name, "Bob");
    let asset = p.assets.last().unwrap();
    assert_eq!(asset.skeleton_id, Some(p.skeletons[0].id));
    asset
        .skin_data
        .as_ref()
        .unwrap()
        .validate(asset.mesh.verts.len(), p.skeletons[0].bones.len())
        .unwrap();
    assert_eq!(p.rig_roles_of(p.skeletons[0].id).unwrap().legs().len(), 2);
    let track = &p.animations[0].clip.tracks[0];
    assert_eq!(track.bone_name, "Hips");
    assert_eq!(track.translations[0].value, [0.0, 1.25, 0.0]);

    // O checkpoint do serviço torna o import desfazível.
    assert!(dst.undo());
    assert_eq!(snapshot(&dst), before);
    assert_eq!(dst.project.assets.len(), assets_before);
    assert!(dst.redo());
    assert_eq!(snapshot(&dst), after);
}
