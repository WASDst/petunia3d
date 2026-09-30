//! Sessão do workspace Animate (cap. 45 F2): seleção coerente, catálogo por
//! rig, draft de slider com um único passo de Undo, cache do preview por
//! conteúdo e Apply Now — tudo headless, sem toolkit.

use petunia_core::{
    AnimatePreview, AppState, CommandError, MotionUnavailable, RigPresetKind, motion_catalog,
};
use petunia_project::{MotionGenerator, MotionStyle, RootMode};

fn state_with(kind: RigPresetKind) -> AppState {
    let mut state = AppState::default();
    state.animate_add_rig(kind).unwrap();
    state
}

fn depth(state: &AppState) -> (usize, usize) {
    state.project.undo.depth()
}

#[test]
fn catalog_marks_what_each_rig_can_do_and_lists_available_first() {
    let snake = state_with(RigPresetKind::Serpent);
    let sid = snake.session.animate.skeleton;
    let catalog = motion_catalog(&snake.project.project, sid);
    assert_eq!(catalog.len(), MotionGenerator::ALL.len());

    let entry = |g: MotionGenerator| catalog.iter().find(|e| e.generator == g).unwrap();
    assert!(entry(MotionGenerator::Serpentine).is_available());
    assert!(entry(MotionGenerator::IdleBreath).is_available());
    assert_eq!(
        entry(MotionGenerator::BipedCycle).unavailable,
        Some(MotionUnavailable::NeedsLegs {
            needed: 2,
            found: 0
        })
    );
    assert!(!entry(MotionGenerator::Gait).is_available());

    // Disponíveis vêm antes dos indisponíveis.
    let first_unavailable = catalog.iter().position(|e| !e.is_available()).unwrap();
    assert!(
        catalog[first_unavailable..]
            .iter()
            .all(|e| !e.is_available())
    );

    let human = state_with(RigPresetKind::Humanoid);
    let human_cat = motion_catalog(&human.project.project, human.session.animate.skeleton);
    let h = |g: MotionGenerator| human_cat.iter().find(|e| e.generator == g).unwrap();
    assert!(h(MotionGenerator::BipedCycle).is_available());
    assert!(matches!(
        h(MotionGenerator::Serpentine).unavailable,
        Some(MotionUnavailable::NeedsChain { needed: 3, .. })
    ));

    // Sem criatura, tudo pede um rig.
    let empty = AppState::default();
    assert!(
        motion_catalog(&empty.project.project, None)
            .iter()
            .all(|e| e.unavailable == Some(MotionUnavailable::NoRig))
    );
}

#[test]
fn every_creature_preset_offers_at_least_one_motion() {
    for kind in RigPresetKind::CREATURES {
        let state = state_with(kind);
        let catalog = motion_catalog(&state.project.project, state.session.animate.skeleton);
        assert!(
            catalog.iter().any(|e| e.is_available()),
            "{} sem nenhum Motion disponível",
            kind.id()
        );
    }
}

#[test]
fn resolve_follows_the_project_through_undo_and_removal() {
    let mut state = AppState::default();
    state.animate_resolve();
    assert!(state.session.animate.skeleton.is_none());

    let sid = state.animate_add_rig(RigPresetKind::Humanoid).unwrap();
    assert_eq!(state.session.animate.skeleton, Some(sid));

    let mid = state
        .animate_add_motion(MotionGenerator::BipedCycle)
        .unwrap();
    assert_eq!(state.session.animate.motion, Some(mid));
    assert!(state.session.animate.playing, "o Motion novo já toca");

    // Undo do motion: a sessão não pode apontar para um id que sumiu.
    assert!(state.undo());
    state.animate_resolve();
    assert_eq!(state.session.animate.motion, None);
    assert!(!state.session.animate.playing);

    // Undo do rig: a criatura some e a sessão se limpa.
    assert!(state.undo());
    state.animate_resolve();
    assert_eq!(state.session.animate.skeleton, None);

    // Redo devolve; resolve reencontra a criatura.
    assert!(state.redo());
    state.animate_resolve();
    assert_eq!(state.session.animate.skeleton, Some(sid));
}

#[test]
fn a_motion_the_rig_cannot_do_is_refused_without_history() {
    let mut state = state_with(RigPresetKind::Humanoid);
    let before = depth(&state);
    let err = state
        .animate_add_motion(MotionGenerator::Serpentine)
        .unwrap_err();
    assert!(matches!(err, CommandError::Execution(_)), "{err:?}");
    assert_eq!(depth(&state), before);
    assert!(state.project.project.motions.is_empty());
    assert!(state.session.animate.motion.is_none());

    // Sem criatura em foco, também recusa.
    let mut empty = AppState::default();
    assert!(
        empty
            .animate_add_motion(MotionGenerator::IdleBreath)
            .is_err()
    );
}

#[test]
fn slider_drag_previews_live_and_commits_one_undo_step() {
    let mut state = state_with(RigPresetKind::Humanoid);
    let mid = state
        .animate_add_motion(MotionGenerator::BipedCycle)
        .unwrap();
    let before_depth = depth(&state);
    let stored = state
        .project
        .project
        .get_motion(mid)
        .unwrap()
        .param("energy");

    // Arrasto: 30 valores, nenhum toca o projeto ou o histórico.
    for i in 0..30 {
        assert!(state.animate_param_preview("energy", 0.5 + i as f32 * 0.04));
    }
    assert_eq!(depth(&state), before_depth);
    assert_eq!(
        state
            .project
            .project
            .get_motion(mid)
            .unwrap()
            .param("energy"),
        stored
    );
    let recipe = state.project.project.get_motion(mid).unwrap();
    let live = state.session.animate.effective_param(recipe, "energy");
    assert!((live - 1.66).abs() < 1e-4, "o draft vale para a UI: {live}");

    // Soltar: um único passo.
    assert!(state.animate_param_commit().unwrap());
    assert_eq!(depth(&state).0, before_depth.0 + 1);
    let committed = state
        .project
        .project
        .get_motion(mid)
        .unwrap()
        .param("energy");
    assert!((committed - 1.66).abs() < 1e-4);
    assert!(state.session.animate.draft().is_none());

    // Um Undo volta ao valor original.
    assert!(state.undo());
    assert_eq!(
        state
            .project
            .project
            .get_motion(mid)
            .unwrap()
            .param("energy"),
        stored
    );

    // Commit sem draft é no-op; valor igual ao gravado não cria passo.
    assert!(!state.animate_param_commit().unwrap());
    let d = depth(&state);
    state.animate_param_preview("energy", stored);
    assert!(!state.animate_param_commit().unwrap());
    assert_eq!(depth(&state), d);
}

#[test]
fn draft_rejects_unknown_keys_and_cancel_discards_it() {
    let mut state = state_with(RigPresetKind::Humanoid);
    assert!(
        !state.animate_param_preview("energy", 1.0),
        "sem Motion não há draft"
    );
    let mid = state
        .animate_add_motion(MotionGenerator::BipedCycle)
        .unwrap();
    assert!(
        !state.animate_param_preview("wavelength", 2.0),
        "chave de outro gerador"
    );
    assert!(!state.animate_param_preview("energy", f32::NAN));
    assert!(state.animate_param_preview("energy", 1.5));
    state.animate_param_cancel();
    assert!(state.session.animate.draft().is_none());
    let recipe = state.project.project.get_motion(mid).unwrap();
    assert_eq!(state.session.animate.effective_param(recipe, "energy"), 1.0);

    // Trocar de Motion descarta o draft do anterior.
    let other = state.animate_duplicate_motion().unwrap();
    assert_ne!(other, mid);
    state.animate_param_preview("energy", 1.9);
    state.animate_select_motion(mid);
    state.animate_resolve();
    assert!(state.session.animate.draft().is_none());
}

#[test]
fn preview_reflects_the_draft_and_caches_by_content() {
    let mut state = state_with(RigPresetKind::Humanoid);
    let mid = state
        .animate_add_motion(MotionGenerator::BipedCycle)
        .unwrap();
    let mut preview = AnimatePreview::default();
    state.session.animate.time = 0.31;

    let base = preview
        .pose(&state.project.project, &state.session.animate)
        .unwrap();
    assert_eq!(preview.rebuilds(), 1);
    // Mesmo tempo, mesmas entradas: nada é reconstruído.
    let again = preview
        .pose(&state.project.project, &state.session.animate)
        .unwrap();
    assert_eq!(again, base);
    assert_eq!(preview.rebuilds(), 1);

    // Mover o playhead não reconstrói o avaliador, mas muda a pose.
    state.session.animate.time = 0.52;
    let moved = preview
        .pose(&state.project.project, &state.session.animate)
        .unwrap();
    assert_ne!(moved, base);
    assert_eq!(preview.rebuilds(), 1);

    // O draft muda a pose ao vivo, sem tocar no projeto.
    state.session.animate.time = 0.31;
    state.animate_param_preview("stride", 2.0);
    let wide = preview
        .pose(&state.project.project, &state.session.animate)
        .unwrap();
    assert_ne!(wide, base, "o draft precisa alterar a pose");
    assert_eq!(preview.rebuilds(), 2);

    // Confirmar e desfazer: o cache serve a pose do valor certo (por conteúdo).
    state.animate_param_commit().unwrap();
    let committed = preview
        .pose(&state.project.project, &state.session.animate)
        .unwrap();
    assert_eq!(committed, wide);
    assert!(state.undo());
    let undone = preview
        .pose(&state.project.project, &state.session.animate)
        .unwrap();
    assert_eq!(
        undone, base,
        "depois do Undo, a pose volta ao valor original"
    );
    assert_eq!(
        state
            .project
            .project
            .get_motion(mid)
            .unwrap()
            .param("stride"),
        1.0
    );
}

#[test]
fn undo_then_a_different_edit_never_serves_a_stale_pose() {
    // Regressão do cache por `revision`: Undo volta a uma revisão já vista e
    // uma nova edição reutiliza o número, com outro valor.
    let mut state = state_with(RigPresetKind::Humanoid);
    state
        .animate_add_motion(MotionGenerator::BipedCycle)
        .unwrap();
    let mut preview = AnimatePreview::default();
    state.session.animate.time = 0.31;

    state.animate_set_param("stride", 1.8).unwrap();
    let a = preview
        .pose(&state.project.project, &state.session.animate)
        .unwrap();
    assert!(state.undo());
    state.animate_set_param("stride", 0.5).unwrap();
    let b = preview
        .pose(&state.project.project, &state.session.animate)
        .unwrap();
    assert_ne!(a, b);
}

#[test]
fn preview_without_motion_shows_the_rest_pose_and_bounds() {
    let state = state_with(RigPresetKind::Bird);
    let mut preview = AnimatePreview::default();
    let pose = preview
        .pose(&state.project.project, &state.session.animate)
        .unwrap();
    let skeleton = state
        .project
        .project
        .get_skeleton(state.session.animate.skeleton.unwrap())
        .unwrap();
    assert_eq!(pose.bones.len(), skeleton.bones.len());
    for (bone, posed) in skeleton.bones.iter().zip(&pose.bones) {
        for i in 0..3 {
            assert!((bone.head[i] - posed.head[i]).abs() < 1e-4, "{}", bone.name);
            assert!((bone.tail[i] - posed.tail[i]).abs() < 1e-4, "{}", bone.name);
        }
    }
    assert_eq!(
        pose.bones.iter().filter(|b| b.parent.is_none()).count(),
        skeleton.bones.iter().filter(|b| b.parent.is_none()).count()
    );
    let (lo, hi) = pose.bounds().unwrap();
    assert!((0..3).all(|i| lo[i] <= hi[i]));
    assert!(preview.error().is_none());

    // Sem criatura: nada a mostrar.
    let empty = AppState::default();
    assert!(
        AnimatePreview::default()
            .pose(&empty.project.project, &empty.session.animate)
            .is_none()
    );
}

#[test]
fn every_motion_on_every_creature_poses_without_nan() {
    for kind in RigPresetKind::CREATURES {
        let mut state = state_with(kind);
        let catalog = motion_catalog(&state.project.project, state.session.animate.skeleton);
        for entry in catalog.iter().filter(|e| e.is_available()) {
            state.animate_add_motion(entry.generator).unwrap();
            let mut preview = AnimatePreview::default();
            for step in 0..=20 {
                state.session.animate.time = step as f32 * 0.07;
                let pose = preview
                    .pose(&state.project.project, &state.session.animate)
                    .unwrap();
                for b in &pose.bones {
                    assert!(
                        b.head.iter().chain(&b.tail).all(|v| v.is_finite()),
                        "{} / {:?} passo {step}",
                        kind.id(),
                        entry.generator
                    );
                }
            }
            assert!(preview.error().is_none());
        }
    }
}

#[test]
fn advance_wraps_the_cycle_and_respects_pause() {
    let mut state = state_with(RigPresetKind::Humanoid);
    state
        .animate_add_motion(MotionGenerator::BipedCycle)
        .unwrap();
    let mut preview = AnimatePreview::default();
    let cycle = preview
        .cycle_seconds(&state.project.project, &state.session.animate)
        .unwrap();
    assert!(cycle > 0.1);

    assert!(preview.advance(&state.project.project, &mut state.session.animate, 0.1));
    assert!((state.session.animate.time - 0.1).abs() < 1e-5);

    // Dá a volta no ciclo, nunca passa dele.
    for _ in 0..500 {
        preview.advance(&state.project.project, &mut state.session.animate, 0.037);
        let t = state.session.animate.time;
        assert!((0.0..cycle).contains(&t), "t={t} ciclo={cycle}");
    }

    // Pausado, não anda; dt inválido também não.
    state.animate_toggle_play();
    assert!(!state.session.animate.playing);
    let t = state.session.animate.time;
    assert!(!preview.advance(&state.project.project, &mut state.session.animate, 0.1));
    assert_eq!(state.session.animate.time, t);
    state.animate_toggle_play();
    assert!(!preview.advance(&state.project.project, &mut state.session.animate, f32::NAN));
    assert!(!preview.advance(&state.project.project, &mut state.session.animate, -1.0));
}

#[test]
fn seek_pauses_and_toggle_needs_a_motion() {
    let mut state = state_with(RigPresetKind::Humanoid);
    state.animate_toggle_play();
    assert!(!state.session.animate.playing, "sem Motion não toca");

    state
        .animate_add_motion(MotionGenerator::IdleBreath)
        .unwrap();
    assert!(state.session.animate.playing);
    state.animate_seek(0.4);
    assert!(!state.session.animate.playing);
    assert_eq!(state.session.animate.time, 0.4);
    state.animate_seek(f32::NAN);
    assert_eq!(state.session.animate.time, 0.0);
    state.animate_seek(-3.0);
    assert_eq!(state.session.animate.time, 0.0);
}

#[test]
fn style_stepped_and_root_mode_are_single_undo_steps() {
    let mut state = state_with(RigPresetKind::Quadruped);
    let mid = state.animate_add_motion(MotionGenerator::Gait).unwrap();

    let d = depth(&state).0;
    assert!(state.animate_set_style(MotionStyle::Heavy).unwrap());
    assert_eq!(depth(&state).0, d + 1);
    assert!(
        !state.animate_set_style(MotionStyle::Heavy).unwrap(),
        "igual = sem passo"
    );
    assert_eq!(depth(&state).0, d + 1);
    assert_eq!(
        state.project.project.get_motion(mid).unwrap().style,
        MotionStyle::Heavy
    );

    assert!(state.animate_set_stepped(Some(12.0)).unwrap());
    assert_eq!(
        state.project.project.get_motion(mid).unwrap().step_fps,
        Some(12.0)
    );
    assert!(state.animate_set_stepped(None).unwrap());
    assert!(
        state
            .project
            .project
            .get_motion(mid)
            .unwrap()
            .step_fps
            .is_none()
    );

    assert!(state.animate_set_root_mode(RootMode::RootMotion).unwrap());
    assert!(
        state.animate_set_stepped(Some(500.0)).is_err(),
        "fora da faixa"
    );
}

#[test]
fn apply_now_keeps_the_motion_live_or_converts_it() {
    let mut state = state_with(RigPresetKind::Humanoid);
    let mid = state
        .animate_add_motion(MotionGenerator::BipedCycle)
        .unwrap();
    let clips = state.project.project.animations.len();

    // Keep Live: o clipe editável aparece e o Motion continua.
    let d = depth(&state).0;
    state.animate_apply_now(true).unwrap();
    assert_eq!(state.project.project.animations.len(), clips + 1);
    assert!(state.project.project.get_motion(mid).is_some());
    assert_eq!(state.session.animate.motion, Some(mid));
    assert_eq!(depth(&state).0, d + 1);

    // Converter: o Motion vivo sai e a sessão resolve para nada.
    state.animate_apply_now(false).unwrap();
    assert_eq!(state.project.project.animations.len(), clips + 2);
    assert!(state.project.project.get_motion(mid).is_none());
    assert_eq!(state.session.animate.motion, None);
    assert!(!state.session.animate.playing);

    // Um Undo devolve o Motion.
    assert!(state.undo());
    state.animate_resolve();
    assert_eq!(state.session.animate.motion, Some(mid));

    // Sem Motion selecionado, Apply Now recusa.
    let mut empty = AppState::default();
    assert!(empty.animate_apply_now(true).is_err());
}

#[test]
fn removing_and_duplicating_motions_keeps_selection_valid() {
    let mut state = state_with(RigPresetKind::Humanoid);
    let a = state
        .animate_add_motion(MotionGenerator::BipedCycle)
        .unwrap();
    let b = state.animate_duplicate_motion().unwrap();
    assert_ne!(a, b);
    assert_eq!(state.session.animate.motion, Some(b));

    state.animate_remove_motion().unwrap();
    assert_eq!(state.session.animate.motion, Some(a), "cai no que sobrou");
    state.animate_remove_motion().unwrap();
    assert_eq!(state.session.animate.motion, None);
    assert!(state.animate_remove_motion().is_err());
}

#[test]
fn switching_creature_selects_that_creatures_motion() {
    let mut state = AppState::default();
    let human = state.animate_add_rig(RigPresetKind::Humanoid).unwrap();
    let walk = state
        .animate_add_motion(MotionGenerator::BipedCycle)
        .unwrap();
    let snake = state.animate_add_rig(RigPresetKind::Serpent).unwrap();
    assert_eq!(state.session.animate.skeleton, Some(snake));
    assert_eq!(
        state.session.animate.motion, None,
        "a serpente ainda não tem Motion"
    );

    assert!(state.animate_select_skeleton(human));
    assert_eq!(state.session.animate.motion, Some(walk));
    assert!(!state.animate_select_skeleton(uuid::Uuid::new_v4()));

    // Selecionar um Motion foca a criatura dele.
    let slither = {
        state.animate_select_skeleton(snake);
        state
            .animate_add_motion(MotionGenerator::Serpentine)
            .unwrap()
    };
    state.animate_select_skeleton(human);
    assert!(state.animate_select_motion(slither));
    assert_eq!(state.session.animate.skeleton, Some(snake));
    assert!(!state.animate_select_motion(uuid::Uuid::new_v4()));
}

#[cfg(feature = "animation-workspace")]
#[test]
fn entering_the_animate_workspace_resolves_the_selection() {
    use petunia_core::Workspace;
    let mut state = AppState::default();
    state.animate_add_rig(RigPresetKind::Fish).unwrap();
    state.session.animate.skeleton = None;
    state.switch_workspace(Workspace::Animate);
    assert!(state.session.animate.skeleton.is_some());
}
