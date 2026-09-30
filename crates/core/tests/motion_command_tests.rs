//! Comandos de Motion procedural (cap. 45 F1, P3D-170): transacionais e validados.

use petunia_core::{
    AddMotionCmd, AddRigPresetCmd, AppState, ApplyMotionNowCmd, AutoRigActiveAssetCmd,
    CommandError, DuplicateMotionCmd, RemoveMotionCmd, RigPresetKind, SetMotionParamCmd,
    SetMotionStyleCmd, UpdateMotionCmd,
};
use petunia_project::{BakeOptions, MotionGenerator, MotionStyle, RootMode};

fn snapshot(state: &AppState) -> String {
    let mut v = serde_json::to_value(&state.project.project).unwrap();
    v.as_object_mut()
        .unwrap()
        .retain(|k, _| !k.ends_with("_revision") && k != "history_selection");
    v.to_string()
}

fn assert_transactional(state: &mut AppState, before: &str, after: &str) {
    assert_ne!(before, after, "o comando deveria alterar o projeto");
    assert!(state.undo());
    assert_eq!(snapshot(state), before, "undo");
    assert!(state.redo());
    assert_eq!(snapshot(state), after, "redo");
}

fn assert_untouched(state: &AppState, depth: (usize, usize), snap: &str) {
    assert_eq!(state.project.undo.depth(), depth);
    assert_eq!(snapshot(state), snap);
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

fn add_walk(state: &mut AppState, sid: uuid::Uuid) -> uuid::Uuid {
    state
        .dispatch(&AddMotionCmd {
            skeleton_id: sid,
            generator: MotionGenerator::BipedCycle,
            name: Some(" Walk ".into()),
            style: MotionStyle::Custom,
        })
        .unwrap();
    state.project.project.motions.last().unwrap().id
}

#[test]
fn add_motion_checks_the_rig_contract_and_is_transactional() {
    let mut state = AppState::default();
    let sid = humanoid(&mut state);
    let before = snapshot(&state);
    let id = add_walk(&mut state, sid);
    let after = snapshot(&state);
    let m = state.project.project.get_motion(id).unwrap();
    assert_eq!(
        (m.name.as_str(), m.generator),
        ("Walk", MotionGenerator::BipedCycle)
    );
    assert_transactional(&mut state, &before, &after);
    state.redo();

    // Estilo inicial e nome padrão.
    state
        .dispatch(&AddMotionCmd {
            skeleton_id: sid,
            generator: MotionGenerator::IdleBreath,
            name: None,
            style: MotionStyle::Floaty,
        })
        .unwrap();
    let idle = state.project.project.motions.last().unwrap();
    assert_eq!(
        (idle.name.as_str(), idle.style),
        ("idle_breath", MotionStyle::Floaty)
    );

    // Rig que não cumpre o contrato: erro legível, sem histórico.
    let (depth, snap) = (state.project.undo.depth(), snapshot(&state));
    let err = state
        .dispatch(&AddMotionCmd {
            skeleton_id: sid,
            generator: MotionGenerator::Serpentine,
            name: None,
            style: MotionStyle::Custom,
        })
        .unwrap_err();
    assert!(
        matches!(&err, CommandError::Execution(m) if m.contains("coluna e cauda")),
        "{err:?}"
    );
    assert!(
        state
            .dispatch(&AddMotionCmd {
                skeleton_id: uuid::Uuid::new_v4(),
                generator: MotionGenerator::Gait,
                name: None,
                style: MotionStyle::Custom,
            })
            .is_err()
    );
    assert_untouched(&state, depth, &snap);
}

#[test]
fn parameter_and_style_commands_clamp_report_no_change_and_undo() {
    let mut state = AppState::default();
    let sid = humanoid(&mut state);
    let id = add_walk(&mut state, sid);

    let before = snapshot(&state);
    state
        .dispatch(&SetMotionParamCmd {
            motion_id: id,
            key: "speed".into(),
            value: 99.0,
        })
        .unwrap();
    let after = snapshot(&state);
    assert_eq!(
        state.project.project.get_motion(id).unwrap().param("speed"),
        3.0
    );
    assert_transactional(&mut state, &before, &after);

    let (depth, snap) = (state.project.undo.depth(), snapshot(&state));
    // Depois do redo `speed` já vale 3.0: 99 é limitado a 3.0 e nada muda.
    assert!(matches!(
        state.dispatch(&SetMotionParamCmd {
            motion_id: id,
            key: "speed".into(),
            value: 99.0
        }),
        Err(CommandError::NoChange(_))
    ));
    for (key, value) in [("nope", 1.0), ("energy", f32::NAN)] {
        assert!(
            state
                .dispatch(&SetMotionParamCmd {
                    motion_id: id,
                    key: key.into(),
                    value
                })
                .is_err()
        );
    }
    assert!(
        state
            .dispatch(&SetMotionParamCmd {
                motion_id: uuid::Uuid::new_v4(),
                key: "speed".into(),
                value: 1.0
            })
            .is_err()
    );
    assert_untouched(&state, depth, &snap);

    let before = snapshot(&state);
    state
        .dispatch(&SetMotionStyleCmd {
            motion_id: id,
            style: MotionStyle::Cartoon,
        })
        .unwrap();
    let after = snapshot(&state);
    let m = state.project.project.get_motion(id).unwrap();
    assert_eq!((m.style, m.param("energy")), (MotionStyle::Cartoon, 1.4));
    assert_transactional(&mut state, &before, &after);
    let (depth, snap) = (state.project.undo.depth(), snapshot(&state));
    assert!(matches!(
        state.dispatch(&SetMotionStyleCmd {
            motion_id: id,
            style: MotionStyle::Cartoon
        }),
        Err(CommandError::NoChange(_))
    ));
    assert_untouched(&state, depth, &snap);
}

#[test]
fn update_duplicate_and_remove_motion() {
    let mut state = AppState::default();
    let sid = humanoid(&mut state);
    let id = add_walk(&mut state, sid);

    let before = snapshot(&state);
    state
        .dispatch(&UpdateMotionCmd {
            motion_id: id,
            name: Some(" Marcha ".into()),
            seed: Some(42),
            root_mode: Some(RootMode::RootMotion),
            step_fps: Some(Some(12.0)),
        })
        .unwrap();
    let after = snapshot(&state);
    let m = state.project.project.get_motion(id).unwrap();
    assert_eq!(
        (m.name.as_str(), m.seed, m.root_mode, m.step_fps),
        ("Marcha", 42, RootMode::RootMotion, Some(12.0))
    );
    assert_transactional(&mut state, &before, &after);

    // Inválidos e NoChange: sem histórico.
    let (depth, snap) = (state.project.undo.depth(), snapshot(&state));
    for cmd in [
        UpdateMotionCmd {
            motion_id: id,
            name: Some("  ".into()),
            ..Default::default()
        },
        UpdateMotionCmd {
            motion_id: id,
            step_fps: Some(Some(0.0)),
            ..Default::default()
        },
        UpdateMotionCmd {
            motion_id: id,
            step_fps: Some(Some(f32::NAN)),
            ..Default::default()
        },
    ] {
        assert!(state.dispatch(&cmd).is_err());
    }
    assert!(
        matches!(
            state.dispatch(&UpdateMotionCmd {
                motion_id: id,
                seed: Some(42),
                ..Default::default()
            }),
            Err(CommandError::NoChange(_))
        ),
        "seed 42 já está no valor"
    );
    assert!(matches!(
        state.dispatch(&UpdateMotionCmd {
            motion_id: id,
            seed: Some(0),
            ..Default::default()
        }),
        Ok(())
    ));
    let _ = (depth, snap);

    // Root motion só onde o gerador suporta.
    let snake = {
        state
            .dispatch(&AddRigPresetCmd {
                kind: RigPresetKind::Humanoid,
                scale: 1.0,
            })
            .unwrap();
        let s = petunia_project::RigPreset::serpent(12, 1.0);
        let sid = s.id;
        state.project.project.add_skeleton(s);
        sid
    };
    state
        .dispatch(&AddMotionCmd {
            skeleton_id: snake,
            generator: MotionGenerator::Serpentine,
            name: None,
            style: MotionStyle::Custom,
        })
        .unwrap();
    let sm = state.project.project.motions.last().unwrap().id;
    assert!(
        state
            .dispatch(&UpdateMotionCmd {
                motion_id: sm,
                root_mode: Some(RootMode::RootMotion),
                ..Default::default()
            })
            .is_err()
    );

    // Duplicar e remover.
    let before = snapshot(&state);
    state
        .dispatch(&DuplicateMotionCmd { motion_id: id })
        .unwrap();
    let after = snapshot(&state);
    let copy = state.project.project.motions.last().unwrap().clone();
    assert_ne!(copy.id, id);
    assert!(copy.name.ends_with(" copy"));
    assert_transactional(&mut state, &before, &after);
    state.redo();
    let before = snapshot(&state);
    state
        .dispatch(&RemoveMotionCmd { motion_id: copy.id })
        .unwrap();
    let after = snapshot(&state);
    assert!(state.project.project.get_motion(copy.id).is_none());
    assert_transactional(&mut state, &before, &after);
    assert!(
        state
            .dispatch(&RemoveMotionCmd {
                motion_id: uuid::Uuid::new_v4()
            })
            .is_err()
    );
}

#[test]
fn apply_now_creates_an_editable_clip_and_reports_the_keys() {
    let mut state = AppState::default();
    let sid = humanoid(&mut state);
    let id = add_walk(&mut state, sid);

    // Keep Live: o clipe é criado e o Motion continua vivo.
    let before = snapshot(&state);
    state
        .dispatch(&ApplyMotionNowCmd {
            motion_id: id,
            options: BakeOptions::default(),
            keep_motion: true,
        })
        .unwrap();
    let after = snapshot(&state);
    let p = &state.project.project;
    assert!(p.get_motion(id).is_some());
    let anim = p.animations.last().unwrap();
    assert_eq!(anim.name, "Walk");
    assert_eq!(anim.preset.as_deref(), Some("biped_cycle"));
    assert!(anim.clip.tracks.len() >= 4);
    assert!(state.ui.status.contains("keys"), "{}", state.ui.status);
    assert_transactional(&mut state, &before, &after);

    // Apply Now (converte): o Motion vivo some e sobra o clipe.
    state.redo();
    let before = snapshot(&state);
    state
        .dispatch(&ApplyMotionNowCmd {
            motion_id: id,
            options: BakeOptions {
                cycles: 2,
                ..BakeOptions::default()
            },
            keep_motion: false,
        })
        .unwrap();
    let after = snapshot(&state);
    assert!(state.project.project.get_motion(id).is_none());
    assert_eq!(state.project.project.animations.len(), 2);
    assert_transactional(&mut state, &before, &after);

    // fps inválido / Motion inexistente: recusado, sem histórico.
    let (depth, snap) = (state.project.undo.depth(), snapshot(&state));
    for fps in [0.0, 500.0, f32::NAN] {
        assert!(
            state
                .dispatch(&ApplyMotionNowCmd {
                    motion_id: id,
                    options: BakeOptions {
                        fps,
                        ..BakeOptions::default()
                    },
                    keep_motion: true
                })
                .is_err()
        );
    }
    assert!(
        state
            .dispatch(&ApplyMotionNowCmd {
                motion_id: uuid::Uuid::new_v4(),
                options: BakeOptions::default(),
                keep_motion: true
            })
            .is_err()
    );
    assert_untouched(&state, depth, &snap);
}

#[test]
fn motion_commands_do_not_touch_render_revisions() {
    let mut state = AppState::default();
    let sid = humanoid(&mut state); // aquece a sincronização inicial da seleção
    let clock = state.project.project.revision_clock();
    let id = add_walk(&mut state, sid);
    state
        .dispatch(&SetMotionParamCmd {
            motion_id: id,
            key: "energy".into(),
            value: 1.5,
        })
        .unwrap();
    state
        .dispatch(&ApplyMotionNowCmd {
            motion_id: id,
            options: BakeOptions::default(),
            keep_motion: true,
        })
        .unwrap();
    assert_eq!(state.project.project.revision_clock(), clock);
}

#[test]
fn auto_rig_to_walking_glb_in_a_handful_of_commands() {
    // O "teste do iniciante" do cap. 45, em comandos: Auto-Rig → Motion → exportar.
    let mut state = AppState::default();
    state.dispatch(&AutoRigActiveAssetCmd).unwrap();
    let sid = state.project.active().unwrap().skeleton_id.unwrap();
    let id = add_walk(&mut state, sid);
    state
        .dispatch(&SetMotionStyleCmd {
            motion_id: id,
            style: MotionStyle::Cartoon,
        })
        .unwrap();
    let glb = petunia_project::export::export_gltf(&state.project.project, &[0]).unwrap();
    let rig = petunia_project::import_gltf::import_rig(&glb, "hero", 1.0)
        .unwrap()
        .unwrap();
    assert_eq!(rig.animations.len(), 1);
    assert_eq!(rig.animations[0].name, "Walk");
    assert!(rig.animations[0].clip.tracks.len() >= 4);
    assert_eq!(rig.mesh_skins.len(), 1, "malha com pesos de skin");
}
