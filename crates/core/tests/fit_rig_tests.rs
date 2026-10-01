//! Fit to model e malha deformada (cap. 45, AN-19): comando transacional,
//! disponibilidade e o override de pose que o renderer consome.

use petunia_core::{
    AnimatePreview, AppState, CommandError, FitBlocked, FitRigToActiveAssetCmd, Mesh,
    RigPresetKind, fit_availability,
};
use petunia_project::{MotionGenerator, mesh_bounds, skeleton_bounds};

fn snapshot(state: &AppState) -> String {
    let mut v = serde_json::to_value(&state.project.project).unwrap();
    v.as_object_mut()
        .unwrap()
        .retain(|k, _| !k.ends_with("_revision") && k != "history_selection");
    v.to_string()
}

/// Modelo low-poly "de personagem": caixa alta e fina, já como asset ativo.
fn add_hero(state: &mut AppState) {
    let mut mesh = Mesh::cube(1.0);
    for v in &mut mesh.verts {
        v.pos = [
            if v.pos[0] >= 0.0 { 0.4 } else { -0.4 },
            if v.pos[1] >= 0.0 { 1.9 } else { 0.0 },
            if v.pos[2] >= 0.0 { 0.2 } else { -0.2 },
        ];
    }
    state.project.project.add("Hero", mesh);
}

fn state_with_hero_and_rig() -> AppState {
    let mut state = AppState::default();
    add_hero(&mut state);
    state.animate_add_rig(RigPresetKind::Humanoid).unwrap();
    state
}

#[test]
fn fit_binds_the_active_model_to_the_creature_in_one_undo_step() {
    let mut state = state_with_hero_and_rig();
    let sid = state.session.animate.skeleton.unwrap();
    let before = snapshot(&state);
    let depth = state.project.undo.depth();

    state.animate_fit_to_model().unwrap();
    let after = snapshot(&state);
    assert_ne!(before, after);
    assert_eq!(state.project.undo.depth().0, depth.0 + 1);

    let asset = state.project.project.active().unwrap();
    assert_eq!(asset.skeleton_id, Some(sid));
    let skin = asset.skin_data.as_ref().expect("skin ligada");
    assert_eq!(skin.vertex_weights.len(), asset.mesh.verts.len());
    // O rig agora ocupa o volume do modelo.
    let (mlo, mhi) = mesh_bounds(&asset.mesh).unwrap();
    let (slo, shi) = skeleton_bounds(state.project.project.get_skeleton(sid).unwrap()).unwrap();
    for axis in 0..3 {
        assert!(
            slo[axis] >= mlo[axis] - 1e-3 && shi[axis] <= mhi[axis] + 1e-3,
            "eixo {axis}"
        );
    }
    assert!(
        (shi.y - mhi.y).abs() < 1e-3,
        "a cabeça chega ao topo do modelo"
    );

    // Do → undo → redo idêntico por hash.
    assert!(state.undo());
    assert_eq!(snapshot(&state), before);
    assert!(state.redo());
    assert_eq!(snapshot(&state), after);
}

#[test]
fn fitting_twice_is_a_no_op_without_history() {
    let mut state = state_with_hero_and_rig();
    state.animate_fit_to_model().unwrap();
    let (depth, snap) = (state.project.undo.depth(), snapshot(&state));
    let err = state.animate_fit_to_model().unwrap_err();
    assert!(matches!(err, CommandError::NoChange(_)), "{err:?}");
    assert_eq!(state.project.undo.depth(), depth);
    assert_eq!(snapshot(&state), snap);
}

#[test]
fn availability_explains_what_is_missing() {
    let mut state = AppState::default();
    add_hero(&mut state);
    let project = &state.project.project;
    assert_eq!(fit_availability(project, None), Err(FitBlocked::NoCreature));
    assert_eq!(
        fit_availability(project, Some(uuid::Uuid::new_v4())),
        Err(FitBlocked::NoCreature)
    );

    state.animate_add_rig(RigPresetKind::Bird).unwrap();
    let sid = state.session.animate.skeleton;
    assert_eq!(fit_availability(&state.project.project, sid), Ok(()));

    state.project.project.active_mut().unwrap().locked = true;
    assert_eq!(
        fit_availability(&state.project.project, sid),
        Err(FitBlocked::Locked)
    );
    let (depth, snap) = (state.project.undo.depth(), snapshot(&state));
    assert!(state.animate_fit_to_model().is_err());
    assert_eq!(state.project.undo.depth(), depth);
    assert_eq!(snapshot(&state), snap, "modelo travado não é alterado");

    state.project.project.active_mut().unwrap().locked = false;
    state
        .project
        .project
        .active_mut()
        .unwrap()
        .mesh
        .verts
        .clear();
    assert_eq!(
        fit_availability(&state.project.project, sid),
        Err(FitBlocked::NoModel)
    );
}

#[test]
fn fit_needs_a_creature_in_focus_and_an_existing_rig() {
    let mut state = AppState::default();
    add_hero(&mut state);
    assert!(
        state.animate_fit_to_model().is_err(),
        "sem criatura em foco"
    );
    let err = state
        .dispatch(&FitRigToActiveAssetCmd {
            skeleton_id: uuid::Uuid::new_v4(),
        })
        .unwrap_err();
    assert!(matches!(err, CommandError::Execution(_)));
}

#[test]
fn every_creature_can_be_fitted_to_the_same_model() {
    for kind in RigPresetKind::CREATURES {
        let mut state = AppState::default();
        add_hero(&mut state);
        state.animate_add_rig(kind).unwrap();
        state.animate_fit_to_model().unwrap();
        let asset = state.project.project.active().unwrap();
        let skin = asset.skin_data.as_ref().unwrap();
        for w in &skin.vertex_weights {
            let sum: f32 = w.weights.iter().sum();
            assert!((sum - 1.0).abs() < 1e-4, "{}", kind.id());
        }
    }
}

#[test]
fn motions_and_roles_survive_the_fit_and_the_pose_moves_the_model() {
    let mut state = state_with_hero_and_rig();
    let mid = state
        .animate_add_motion(MotionGenerator::BipedCycle)
        .unwrap();
    let mut preview = AnimatePreview::default();

    // Antes do fit nenhum modelo participa: nada a substituir no renderer.
    assert!(
        preview
            .pose_override(&state.project.project, &state.session.animate, 1)
            .is_none()
    );

    state.animate_fit_to_model().unwrap();
    assert!(
        state.project.project.get_motion(mid).is_some(),
        "o Motion segue"
    );
    assert!(
        state.project.project.motion_evaluator(mid).is_ok(),
        "papéis e rig continuam válidos"
    );

    let doc = snapshot(&state);
    state.session.animate.time = 0.05;
    let a = preview
        .pose_override(&state.project.project, &state.session.animate, 2)
        .expect("o modelo ligado entra na pose");
    state.session.animate.time = 0.35;
    let b = preview
        .pose_override(&state.project.project, &state.session.animate, 3)
        .unwrap();
    let asset_id = state.project.project.active().unwrap().id;
    let (ma, mb) = (a.mesh_for(asset_id).unwrap(), b.mesh_for(asset_id).unwrap());
    assert!(
        ma.verts.iter().zip(&mb.verts).any(|(x, y)| x.pos != y.pos),
        "a malha deformada muda no tempo"
    );
    assert_eq!((a.revision, b.revision), (2, 3));

    // O preview nunca toca o documento: a malha guardada segue em repouso.
    assert_eq!(snapshot(&state), doc);

    // Sem Motion, a pose de repouso reproduz o modelo (identidade).
    state.animate_remove_motion().unwrap();
    let rest = preview
        .pose_override(&state.project.project, &state.session.animate, 4)
        .expect("repouso também substitui (idêntico ao original)");
    let original = &state.project.project.active().unwrap().mesh;
    for (r, o) in rest
        .mesh_for(asset_id)
        .unwrap()
        .verts
        .iter()
        .zip(&original.verts)
    {
        for k in 0..3 {
            assert!((r.pos[k] - o.pos[k]).abs() < 1e-4);
        }
    }
}
