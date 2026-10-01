//! Geradores de movimento procedural (P3D-170, cap. 45 F1).

use glam::{Quat, Vec3};
use petunia_project::{
    BakeOptions, IkChain, MotionError, MotionEvaluator, MotionGenerator, MotionRecipe, MotionStyle,
    Project, RigPreset, RigRoleMap, RootMode, Skeleton, Transform3D, bake_motion,
    chain_end_position,
};

type Rig = (Skeleton, RigRoleMap);

fn rig(sk: Skeleton) -> Rig {
    let roles = RigRoleMap::infer(&sk);
    (sk, roles)
}

fn recipe(rig: &Rig, g: MotionGenerator) -> MotionRecipe {
    MotionRecipe::new(rig.0.id, g, g.id())
}

fn eval(rig: &Rig, r: &MotionRecipe) -> MotionEvaluator {
    MotionEvaluator::new(r, &rig.0, &rig.1).expect("evaluator")
}

fn combos() -> Vec<(&'static str, Rig, Vec<MotionGenerator>)> {
    use MotionGenerator::*;
    vec![
        (
            "humanoid",
            rig(RigPreset::humanoid(1.0)),
            vec![BipedCycle, Gait, IdleBreath],
        ),
        (
            "bird",
            rig(RigPreset::bird(1.0)),
            vec![BipedCycle, IdleBreath],
        ),
        (
            "quadruped",
            rig(RigPreset::quadruped(1.0)),
            vec![Gait, IdleBreath],
        ),
        (
            "spider",
            rig(RigPreset::multi_leg(8, 1.0)),
            vec![Gait, IdleBreath],
        ),
        (
            "snake",
            rig(RigPreset::serpent(14, 1.0)),
            vec![Serpentine, IdleBreath],
        ),
        ("fish", rig(RigPreset::fish(1.0)), vec![Serpentine]),
    ]
}

fn same(a: &[Transform3D], b: &[Transform3D], tol: f32) -> bool {
    a.iter().zip(b).all(|(x, y)| {
        (Vec3::from(x.translation) - Vec3::from(y.translation)).length() < tol
            && Quat::from_array(x.rotation)
                .dot(Quat::from_array(y.rotation))
                .abs()
                > 1.0 - tol
            && (Vec3::from(x.scale) - Vec3::from(y.scale)).length() < tol
    })
}

fn leg_chains(rig: &Rig) -> Vec<IkChain> {
    rig.1
        .legs()
        .iter()
        .map(|l| IkChain::two_bone_leg(rig.0.id, "leg", l))
        .collect()
}

#[test]
fn params_are_clamped_typed_and_reported() {
    let r0 = rig(RigPreset::humanoid(1.0));
    let mut r = recipe(&r0, MotionGenerator::BipedCycle);
    assert_eq!(r.param("speed"), 1.0);
    assert_eq!(r.param("nope"), 0.0);
    assert!(r.set_param("speed", 99.0).unwrap());
    assert_eq!(r.param("speed"), 3.0);
    assert!(
        !r.set_param("speed", 3.0).unwrap(),
        "mesmo valor: nada muda"
    );
    assert_eq!(
        r.set_param("nope", 1.0),
        Err(MotionError::UnknownParam("nope".into()))
    );
    assert!(matches!(
        r.set_param("energy", f32::NAN),
        Err(MotionError::NotFinite(_))
    ));
    let rev = r.revision;
    assert!(r.set_param("energy", 1.5).unwrap());
    assert_eq!(r.revision, rev + 1);

    // Choices viram inteiros (Gait.pattern).
    let g0 = rig(RigPreset::quadruped(1.0));
    let mut g = recipe(&g0, MotionGenerator::Gait);
    g.set_param("pattern", 1.7).unwrap();
    assert_eq!(g.param("pattern"), 2.0);

    // Style preenche parâmetros conhecidos e ignora os desconhecidos.
    let mut r = recipe(&r0, MotionGenerator::BipedCycle);
    assert!(r.apply_style(MotionStyle::Cartoon));
    assert_eq!((r.param("energy"), r.style), (1.4, MotionStyle::Cartoon));
    assert!(!r.apply_style(MotionStyle::Cartoon));
    let mut s = recipe(
        &rig(RigPreset::serpent(12, 1.0)),
        MotionGenerator::Serpentine,
    );
    s.apply_style(MotionStyle::Cartoon);
    assert_eq!(s.param("energy"), 1.4);
    // Editar à mão volta o Style para Custom.
    r.set_param("speed", 1.2).unwrap();
    assert_eq!(r.style, MotionStyle::Custom);
}

#[test]
fn rig_contract_errors_are_structured_and_readable() {
    use petunia_project::RoleContractError::*;
    let quad = rig(RigPreset::quadruped(1.0));
    // Quadrúpede não tem Hips: o ciclo bípede recusa com o papel que falta.
    let err = MotionEvaluator::new(
        &recipe(&quad, MotionGenerator::BipedCycle),
        &quad.0,
        &quad.1,
    )
    .err()
    .unwrap();
    assert!(matches!(err, MotionError::Rig(MissingRole(_))), "{err:?}");
    let snake = rig(RigPreset::serpent(12, 1.0));
    let err = MotionEvaluator::new(&recipe(&snake, MotionGenerator::Gait), &snake.0, &snake.1)
        .err()
        .unwrap();
    assert!(
        matches!(
            err,
            MotionError::Rig(NotEnoughLegs {
                needed: 2,
                found: 0
            })
        ),
        "{err:?}"
    );
    assert!(err.to_string().contains("2 ou mais pernas"), "{err}");
    let human = rig(RigPreset::humanoid(1.0));
    let err = MotionEvaluator::new(
        &recipe(&human, MotionGenerator::Serpentine),
        &human.0,
        &human.1,
    )
    .err()
    .unwrap();
    assert!(
        matches!(err, MotionError::Rig(ChainTooShort { .. })),
        "{err:?}"
    );
    // Receita de outro esqueleto.
    let r = MotionRecipe::new(uuid::Uuid::new_v4(), MotionGenerator::Gait, "x");
    assert_eq!(
        MotionEvaluator::new(&r, &quad.0, &quad.1).err(),
        Some(MotionError::SkeletonMismatch)
    );
}

#[test]
fn every_generator_closes_its_loop_and_is_deterministic() {
    for (name, rig, gens) in combos() {
        for g in gens {
            let mut r = recipe(&rig, g);
            r.seed = 7;
            r.set_param("variation", 0.5).unwrap();
            let ev = eval(&rig, &r);
            let t = ev.cycle_seconds();
            assert!(t.is_finite() && t > 0.1, "{name} {g:?}: ciclo {t}");
            assert!(
                same(&ev.pose_at(0.0), &ev.pose_at(t), 1e-3),
                "{name} {g:?}: pose(0) != pose(T)"
            );
            assert!(
                same(&ev.pose_at(t * 0.37), &ev.pose_at(t * 0.37 + t), 1e-3),
                "{name} {g:?}"
            );
            // Determinismo: outra instância, mesma receita ⇒ mesma pose.
            let ev2 = eval(&rig, &r);
            assert_eq!(ev.pose_at(0.41 * t), ev2.pose_at(0.41 * t), "{name} {g:?}");
        }
    }
}

#[test]
fn seed_and_variation_change_the_pose_only_together() {
    let r0 = rig(RigPreset::multi_leg(8, 1.0));
    let mut a = recipe(&r0, MotionGenerator::Gait);
    let mut b = a.clone();
    b.seed = 99;
    let t = eval(&r0, &a).cycle_seconds() * 0.3;
    // Sem `variation` a seed é irrelevante.
    assert_eq!(eval(&r0, &a).pose_at(t), eval(&r0, &b).pose_at(t));
    a.set_param("variation", 1.0).unwrap();
    b.set_param("variation", 1.0).unwrap();
    assert_ne!(eval(&r0, &a).pose_at(t), eval(&r0, &b).pose_at(t));
}

#[test]
fn locomotion_and_wave_only_rotate_bones_except_the_root() {
    for (name, rig, gens) in combos() {
        for g in gens
            .into_iter()
            .filter(|g| *g != MotionGenerator::IdleBreath)
        {
            let r = recipe(&rig, g);
            let ev = eval(&rig, &r);
            let root = rig.0.bones.iter().position(|b| b.parent.is_none()).unwrap();
            for k in 0..12 {
                let pose = ev.pose_at(ev.cycle_seconds() * k as f32 / 12.0);
                for (i, (p, b)) in pose.iter().zip(&rig.0.bones).enumerate() {
                    if i == root {
                        continue;
                    }
                    assert_eq!(
                        p.translation, b.local_transform.translation,
                        "{name} {g:?} {}",
                        b.name
                    );
                    assert_eq!(p.scale, b.local_transform.scale, "{name} {g:?} {}", b.name);
                }
            }
        }
    }
}

#[test]
fn ik_keeps_bone_lengths_during_a_walk() {
    let r0 = rig(RigPreset::humanoid(1.0));
    let mut r = recipe(&r0, MotionGenerator::BipedCycle);
    r.set_param("run_blend", 1.0).unwrap();
    let ev = eval(&r0, &r);
    let sk = &r0.0;
    for k in 0..16 {
        let pose = ev.pose_at(ev.cycle_seconds() * k as f32 / 16.0);
        let world = sk.world_pose_matrices(&pose).unwrap();
        for l in r0.1.legs() {
            let head = |id: u32| world[sk.bone_index(id).unwrap()].w_axis.truncate();
            let rest = |id: u32| Vec3::from(sk.get_bone(id).unwrap().head);
            let (t, s, f) = (l.thigh, l.shin, l.foot.unwrap());
            for (a, b) in [(t, s), (s, f)] {
                let (dl, rl) = ((head(a) - head(b)).length(), (rest(a) - rest(b)).length());
                assert!((dl - rl).abs() < 1e-3, "comprimento {dl} vs {rl}");
            }
        }
    }
}

/// Posição do efetuador de cada perna ao longo de dois ciclos.
fn foot_tracks(rig: &Rig, ev: &MotionEvaluator, samples: usize) -> Vec<Vec<Vec3>> {
    let chains = leg_chains(rig);
    let t = ev.cycle_seconds();
    chains
        .iter()
        .map(|c| {
            (0..samples)
                .map(|i| {
                    let pose = ev.pose_at(2.0 * t * i as f32 / samples as f32);
                    chain_end_position(&rig.0, &pose, c).unwrap()
                })
                .collect()
        })
        .collect()
}

/// Trechos contíguos em que o pé está no chão (y ≈ repouso).
fn stance_runs(track: &[Vec3], ground: f32) -> Vec<Vec<Vec3>> {
    let mut runs: Vec<Vec<Vec3>> = Vec::new();
    let mut cur: Vec<Vec3> = Vec::new();
    for p in track {
        if (p.y - ground).abs() < 1e-3 {
            cur.push(*p);
        } else if !cur.is_empty() {
            runs.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        runs.push(cur);
    }
    runs
}

#[test]
fn biped_feet_stay_planted_during_stance_with_root_motion() {
    let r0 = rig(RigPreset::humanoid(1.0));
    for run in [0.0, 1.0] {
        let mut r = recipe(&r0, MotionGenerator::BipedCycle);
        r.root_mode = RootMode::RootMotion;
        r.set_param("run_blend", run).unwrap();
        let ev = eval(&r0, &r);
        let tracks = foot_tracks(&r0, &ev, 400);
        for (leg, track) in tracks.iter().enumerate() {
            let ground = track.iter().map(|p| p.y).fold(f32::MAX, f32::min);
            assert!(ground > -0.01, "pé abaixo do chão: {ground}");
            // Nunca atravessa o chão; levanta no balanço.
            let peak = track.iter().map(|p| p.y).fold(f32::MIN, f32::max);
            assert!(
                peak - ground > 0.03,
                "perna {leg} não levanta o pé ({peak})"
            );
            let runs = stance_runs(track, ground);
            assert!(
                runs.len() >= 2,
                "perna {leg}: {} apoios em 2 ciclos",
                runs.len()
            );
            for run_pts in runs.iter().filter(|r| r.len() > 20) {
                let (lo, hi) = run_pts.iter().fold((Vec3::MAX, Vec3::MIN), |(lo, hi), p| {
                    (lo.min(*p), hi.max(*p))
                });
                assert!(
                    (hi - lo).length() < 0.015,
                    "perna {leg} run={run}: pé desliza {:?} durante o apoio",
                    hi - lo
                );
            }
        }
    }
}

#[test]
fn feet_travel_backwards_relative_to_the_body_in_place() {
    let r0 = rig(RigPreset::humanoid(1.0));
    let r = recipe(&r0, MotionGenerator::BipedCycle);
    let ev = eval(&r0, &r);
    let tracks = foot_tracks(&r0, &ev, 400);
    // Em posição, o apoio empurra o pé para trás (z decrescente ao longo do apoio).
    let ground = tracks[0].iter().map(|p| p.y).fold(f32::MAX, f32::min);
    let runs = stance_runs(&tracks[0], ground);
    let long = runs.iter().find(|r| r.len() > 20).expect("apoio");
    assert!(long.first().unwrap().z - long.last().unwrap().z > 0.05);
}

#[test]
fn root_motion_advances_the_body_and_scales_with_stride() {
    let r0 = rig(RigPreset::humanoid(1.0));
    let advance = |mode: RootMode, stride: f32| {
        let mut r = recipe(&r0, MotionGenerator::BipedCycle);
        r.root_mode = mode;
        r.set_param("stride", stride).unwrap();
        let ev = eval(&r0, &r);
        let t = ev.cycle_seconds();
        let root = r0.0.bones.iter().position(|b| b.parent.is_none()).unwrap();
        let (a, b) = (
            ev.pose_at(0.0)[root].translation,
            ev.pose_at(t)[root].translation,
        );
        Vec3::from(b) - Vec3::from(a)
    };
    assert!(advance(RootMode::InPlace, 1.0).length() < 1e-4);
    let (full, half) = (
        advance(RootMode::RootMotion, 1.0),
        advance(RootMode::RootMotion, 0.5),
    );
    assert!(full.z > 0.1, "{full:?}");
    assert!((full.z / half.z - 2.0).abs() < 0.05, "{full:?} {half:?}");
    // Geradores sem root motion ignoram o modo.
    let snake = rig(RigPreset::serpent(12, 1.0));
    let mut s = recipe(&snake, MotionGenerator::Serpentine);
    s.root_mode = RootMode::RootMotion;
    assert!(!eval(&snake, &s).root_motion());
}

#[test]
fn spider_uses_an_alternating_tetrapod_gait() {
    let r0 = rig(RigPreset::multi_leg(8, 1.0));
    let r = recipe(&r0, MotionGenerator::Gait);
    let ev = eval(&r0, &r);
    let tracks = foot_tracks(&r0, &ev, 400);
    let legs = r0.1.legs();
    assert_eq!(legs.len(), 8);
    let ground: Vec<f32> = tracks
        .iter()
        .map(|t| t.iter().map(|p| p.y).fold(f32::MAX, f32::min))
        .collect();
    let stance = |leg: usize, i: usize| (tracks[leg][i].y - ground[leg]).abs() < 1e-3;
    for i in 0..400 {
        let planted = (0..8).filter(|&l| stance(l, i)).count();
        assert!(planted >= 4, "amostra {i}: só {planted} pernas apoiadas");
        // Nunca duas pernas vizinhas do mesmo lado no balanço ao mesmo tempo.
        for a in 0..8 {
            for b in a + 1..8 {
                let (la, lb) = (legs[a].limb, legs[b].limb);
                if la % 2 == lb % 2 && (la / 2).abs_diff(lb / 2) == 1 {
                    assert!(
                        stance(a, i) || stance(b, i),
                        "pernas {la} e {lb} no ar juntas ({i})"
                    );
                }
            }
        }
    }
    // Cada perna levanta o pé.
    for (l, t) in tracks.iter().enumerate() {
        assert!(
            t.iter().map(|p| p.y).fold(f32::MIN, f32::max) - ground[l] > 0.01,
            "perna {l}"
        );
    }
}

#[test]
fn quadruped_walk_keeps_three_feet_down_and_trot_pairs_diagonals() {
    let r0 = rig(RigPreset::quadruped(1.0));
    let mut r = recipe(&r0, MotionGenerator::Gait);
    let planted = |r: &MotionRecipe| {
        let ev = eval(&r0, r);
        let tracks = foot_tracks(&r0, &ev, 300);
        let g: Vec<f32> = tracks
            .iter()
            .map(|t| t.iter().map(|p| p.y).fold(f32::MAX, f32::min))
            .collect();
        (0..300)
            .map(|i| {
                (0..4)
                    .filter(|&l| (tracks[l][i].y - g[l]).abs() < 1e-3)
                    .count()
            })
            .collect::<Vec<_>>()
    };
    // Auto com duty 0.6 = lateral sequence: 2 a 3 apoiadas, nunca menos de 2.
    let walk = planted(&r);
    assert!(
        walk.iter().all(|&c| c >= 2) && walk.contains(&3),
        "{walk:?}"
    );
    // Passo de caminhada com duty 0.8: sempre ≥ 3 pés no chão.
    r.set_param("duty", 0.8).unwrap();
    let slow = planted(&r);
    assert!(slow.iter().all(|&c| c >= 3), "{slow:?}");
    // Trote (duty 0.5): pares diagonais; sempre ≥ 2 apoiadas.
    r.set_param("duty", 0.5).unwrap();
    let trot = planted(&r);
    assert!(trot.iter().all(|&c| c >= 2), "{trot:?}");
}

#[test]
fn serpentine_wave_travels_from_head_to_tail_and_rests_at_zero_energy() {
    let r0 = rig(RigPreset::serpent(14, 1.0));
    let mut r = recipe(&r0, MotionGenerator::Serpentine);
    r.set_param("tail_boost", 0.0).unwrap();
    r.set_param("head_hold", 0.0).unwrap();
    r.set_param("smoothness", 1.0).unwrap();
    let ev = eval(&r0, &r);
    let sk = &r0.0;
    // Ossos da cabeça para a cauda (mesma ordem do gerador).
    let mut order: Vec<usize> = (0..sk.bones.len()).collect();
    order.sort_by(|&a, &b| sk.bones[b].head[2].total_cmp(&sk.bones[a].head[2]));
    let yaw = |pose: &[Transform3D], i: usize| {
        let q = Quat::from_array(pose[i].rotation);
        2.0 * q.y.atan2(q.w)
    };
    let (t_cycle, lambda) = (ev.cycle_seconds(), r.param("wavelength"));
    // (Root e Tail_0 empatam em z: índices 8 e 9 têm ordem ambígua.)
    for k in [1usize, 4, 6, 10] {
        for s in [0.05f32, 0.2, 0.6] {
            let t = s * t_cycle;
            let delayed = yaw(&ev.pose_at(t - k as f32 * t_cycle / lambda), order[0]);
            let here = yaw(&ev.pose_at(t), order[k]);
            assert!((delayed - here).abs() < 1e-4, "k={k}: {delayed} vs {here}");
        }
    }
    // Energia 0: repouso exato.
    r.set_param("energy", 0.0).unwrap();
    let ev0 = eval(&r0, &r);
    let rest: Vec<Transform3D> = sk.bones.iter().map(|b| b.local_transform).collect();
    assert!(same(&ev0.pose_at(0.7), &rest, 1e-6));
}

#[test]
fn idle_breathing_is_small_smooth_and_zero_at_zero_energy() {
    let r0 = rig(RigPreset::humanoid(1.0));
    let mut r = recipe(&r0, MotionGenerator::IdleBreath);
    let ev = eval(&r0, &r);
    let chest = r0.0.bone_index(r0.0.find_bone("Chest").unwrap()).unwrap();
    let rest_y = r0.0.bones[chest].local_transform.translation[1];
    let ys: Vec<f32> = (0..64)
        .map(|i| ev.pose_at(ev.cycle_seconds() * i as f32 / 64.0)[chest].translation[1] - rest_y)
        .collect();
    let (lo, hi) = ys
        .iter()
        .fold((f32::MAX, f32::MIN), |(l, h), y| (l.min(*y), h.max(*y)));
    assert!(
        lo < -0.005 && hi > 0.005 && hi < 0.05,
        "amplitude {lo}..{hi}"
    );
    r.set_param("energy", 0.0).unwrap();
    let rest: Vec<Transform3D> = r0.0.bones.iter().map(|b| b.local_transform).collect();
    assert!(same(&eval(&r0, &r).pose_at(1.3), &rest, 1e-6));
}

#[test]
fn stepped_style_quantizes_time_and_holds_the_pose() {
    let r0 = rig(RigPreset::humanoid(1.0));
    let mut r = recipe(&r0, MotionGenerator::BipedCycle);
    let smooth = eval(&r0, &r);
    r.step_fps = Some(12.0);
    let ev = eval(&r0, &r);
    assert_eq!(ev.pose_at(0.03), ev.pose_at(0.0));
    assert_eq!(ev.pose_at(0.09), ev.pose_at(1.0 / 12.0));
    assert_ne!(ev.pose_at(0.09), ev.pose_at(0.0));
    // Nos instantes de passo coincide com a versão contínua.
    assert!(same(
        &ev.pose_at(4.0 / 12.0),
        &smooth.pose_at(4.0 / 12.0),
        1e-4
    ));
}

#[test]
fn extreme_parameters_never_produce_nan_or_denormalized_rotations() {
    for (name, rig, gens) in combos() {
        for g in gens {
            let base = recipe(&rig, g);
            let mut variants = vec![base.clone()];
            for spec in g.specs() {
                for v in [spec.min, spec.max] {
                    let mut r = base.clone();
                    r.set_param(spec.key, v).unwrap();
                    variants.push(r);
                }
            }
            for style in [
                MotionStyle::Cartoon,
                MotionStyle::Heavy,
                MotionStyle::Stiff,
                MotionStyle::Floaty,
            ] {
                let mut r = base.clone();
                r.apply_style(style);
                variants.push(r);
            }
            for r in variants {
                let ev = eval(&rig, &r);
                for k in 0..10 {
                    for p in ev.pose_at(ev.cycle_seconds() * k as f32 / 10.0 + 0.013) {
                        assert!(
                            p.translation
                                .iter()
                                .chain(&p.scale)
                                .chain(&p.rotation)
                                .all(|x| x.is_finite()),
                            "{name} {g:?}"
                        );
                        let n: f32 = p.rotation.iter().map(|x| x * x).sum();
                        assert!((n - 1.0).abs() < 1e-3, "{name} {g:?}: |q|²={n}");
                    }
                }
            }
        }
    }
}

#[test]
fn bake_reduces_keys_closes_the_loop_and_matches_the_recipe() {
    let r0 = rig(RigPreset::humanoid(1.0));
    let r = recipe(&r0, MotionGenerator::BipedCycle);
    let ev = eval(&r0, &r);
    let opts = BakeOptions::default();
    let out = bake_motion(&r, &r0.0, &r0.1, &opts).unwrap();
    let clip = &out.clip;
    assert!((clip.duration - ev.cycle_seconds()).abs() < 1e-5);
    assert!(clip.looping);
    assert_eq!(clip.name, r.name);
    assert!(
        out.keys_after > 0 && out.keys_after < out.keys_before,
        "{out:?}"
    );
    assert!(out.animated_bones >= 4);
    // Bones que o gerador não anima não ganham trilha (herdam o repouso).
    assert!(clip.tracks.iter().all(|t| t.bone_name != "Head"));

    // Loop: primeira e última chave de cada canal coincidem.
    for tr in &clip.tracks {
        if let (Some(a), Some(b)) = (tr.rotations.first(), tr.rotations.last()) {
            assert!(
                Quat::from_array(a.value)
                    .dot(Quat::from_array(b.value))
                    .abs()
                    > 1.0 - 1e-5
            );
        }
    }

    // Equivalência: nos instantes de amostragem, dentro da tolerância declarada.
    let n = out.frames - 1;
    let tol_rot = opts.tolerance.rotation_deg.to_radians() * 1.05;
    for i in 0..=n {
        let t = clip.duration * i as f32 / n as f32;
        let (want, got) = (
            ev.pose_at(t),
            clip.sample_pose(&r0.0, t.min(clip.duration - 1e-4)),
        );
        for (w, g) in want.iter().zip(&got) {
            assert!(
                (Vec3::from(w.translation) - Vec3::from(g.translation)).length()
                    < opts.tolerance.translation * 1.05 + 1e-4
            );
            let ang = Quat::from_array(w.rotation).angle_between(Quat::from_array(g.rotation));
            assert!(ang < tol_rot + 1e-3, "frame {i}: {ang}");
        }
    }
    // Entre amostras o erro continua pequeno para uma curva suave.
    for k in 0..40 {
        let t = clip.duration * (k as f32 + 0.37) / 40.5;
        let (want, got) = (ev.pose_at(t), clip.sample_pose(&r0.0, t));
        for (w, g) in want.iter().zip(&got) {
            assert!(
                Quat::from_array(w.rotation).angle_between(Quat::from_array(g.rotation)) < 0.06
            );
        }
    }
}

#[test]
fn bake_options_and_root_motion_are_respected() {
    let r0 = rig(RigPreset::quadruped(1.0));
    let mut r = recipe(&r0, MotionGenerator::Gait);
    for bad in [0.0, 0.5, 241.0, f32::NAN] {
        let o = BakeOptions {
            fps: bad,
            ..BakeOptions::default()
        };
        assert_eq!(
            bake_motion(&r, &r0.0, &r0.1, &o).err(),
            Some(MotionError::InvalidFps)
        );
    }
    let two = bake_motion(
        &r,
        &r0.0,
        &r0.1,
        &BakeOptions {
            cycles: 2,
            ..BakeOptions::default()
        },
    )
    .unwrap();
    let one = bake_motion(&r, &r0.0, &r0.1, &BakeOptions::default()).unwrap();
    assert!((two.clip.duration / one.clip.duration - 2.0).abs() < 1e-4);
    r.root_mode = RootMode::RootMotion;
    assert!(
        !bake_motion(&r, &r0.0, &r0.1, &BakeOptions::default())
            .unwrap()
            .clip
            .looping
    );
    // Rig que não cumpre o contrato: erro claro, nada é gerado.
    let human = rig(RigPreset::humanoid(1.0));
    let bad = recipe(&human, MotionGenerator::Serpentine);
    assert!(matches!(
        bake_motion(&bad, &human.0, &human.1, &BakeOptions::default()),
        Err(MotionError::Rig(_))
    ));
}

#[test]
fn recipes_round_trip_through_serde() {
    let r0 = rig(RigPreset::humanoid(1.0));
    let mut r = recipe(&r0, MotionGenerator::BipedCycle);
    r.apply_style(MotionStyle::Heavy);
    r.step_fps = Some(15.0);
    r.seed = 5;
    let back: MotionRecipe = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    assert_eq!(back, r);
}

fn head_pos(sk: &Skeleton, pose: &[Transform3D], name: &str) -> Vec3 {
    let world = sk.world_pose_matrices(pose).unwrap();
    world[sk.bone_index(sk.find_bone(name).unwrap()).unwrap()]
        .w_axis
        .truncate()
}

fn correlation(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len() as f32;
    let (ma, mb) = (a.iter().sum::<f32>() / n, b.iter().sum::<f32>() / n);
    let cov: f32 = a.iter().zip(b).map(|(x, y)| (x - ma) * (y - mb)).sum();
    let (va, vb): (f32, f32) = (
        a.iter().map(|x| (x - ma).powi(2)).sum(),
        b.iter().map(|y| (y - mb).powi(2)).sum(),
    );
    cov / (va.sqrt() * vb.sqrt()).max(1e-9)
}

#[test]
fn biped_knees_bend_forward_arms_swing_against_legs_and_lean_tilts_forward() {
    let r0 = rig(RigPreset::humanoid(1.0));
    let sk = &r0.0;
    let r = recipe(&r0, MotionGenerator::BipedCycle);
    let ev = eval(&r0, &r);
    let n = 200;
    let poses: Vec<Vec<Transform3D>> = (0..n)
        .map(|i| ev.pose_at(ev.cycle_seconds() * i as f32 / n as f32))
        .collect();

    // Joelho à frente da linha quadril→tornozelo (frente = +Z neste preset).
    for pose in &poses {
        for side in ["L", "R"] {
            let (hip, knee, ankle) = (
                head_pos(sk, pose, &format!("UpperLeg.{side}")),
                head_pos(sk, pose, &format!("LowerLeg.{side}")),
                head_pos(sk, pose, &format!("Foot.{side}")),
            );
            let line = (ankle - hip).normalize();
            let bend = (knee - hip) - line * (knee - hip).dot(line);
            assert!(bend.z > -1e-4, "joelho {side} dobra para trás: {bend:?}");
        }
    }

    // Braço esquerdo balança contra a perna esquerda (e com a direita).
    let series =
        |f: &dyn Fn(&[Transform3D]) -> f32| poses.iter().map(|p| f(p)).collect::<Vec<f32>>();
    let foot_l = series(&|p| head_pos(sk, p, "Foot.L").z);
    let foot_r = series(&|p| head_pos(sk, p, "Foot.R").z);
    let elbow_l = series(&|p| head_pos(sk, p, "LowerArm.L").z);
    assert!(
        correlation(&elbow_l, &foot_l) < -0.6,
        "{}",
        correlation(&elbow_l, &foot_l)
    );
    assert!(
        correlation(&elbow_l, &foot_r) > 0.6,
        "{}",
        correlation(&elbow_l, &foot_r)
    );
    // As pernas estão em oposição de fase.
    assert!(correlation(&foot_l, &foot_r) < -0.6);

    // `lean` positivo inclina o tronco para a frente.
    let head_z = |lean: f32| {
        let mut r = recipe(&r0, MotionGenerator::BipedCycle);
        r.set_param("lean", lean).unwrap();
        head_pos(sk, &eval(&r0, &r).pose_at(0.0), "Head").z
    };
    assert!(head_z(1.0) > head_z(0.0) + 0.02 && head_z(-1.0) < head_z(0.0) - 0.02);
}

#[test]
fn energy_scales_the_motion_and_style_changes_it_measurably() {
    let r0 = rig(RigPreset::humanoid(1.0));
    let travel = |edit: &dyn Fn(&mut MotionRecipe)| {
        let mut r = recipe(&r0, MotionGenerator::BipedCycle);
        edit(&mut r);
        let ev = eval(&r0, &r);
        (0..60)
            .map(|i| {
                head_pos(
                    &r0.0,
                    &ev.pose_at(ev.cycle_seconds() * i as f32 / 60.0),
                    "Hand.L",
                )
                .z
            })
            .fold((f32::MAX, f32::MIN), |(lo, hi), z| (lo.min(z), hi.max(z)))
    };
    let span = |e: f32| {
        let (lo, hi) = travel(&|r: &mut MotionRecipe| {
            r.set_param("energy", e).unwrap();
        });
        hi - lo
    };
    assert!(
        span(1.5) > span(1.0) && span(1.0) > span(0.3),
        "{} {} {}",
        span(1.5),
        span(1.0),
        span(0.3)
    );
    let (lo, hi) = travel(&|r: &mut MotionRecipe| {
        r.apply_style(MotionStyle::Stiff);
    });
    let (lo0, hi0) = travel(&|_| {});
    assert!(hi - lo < hi0 - lo0, "Stiff balança menos os braços");
}

fn project_with_rig(sk: Skeleton) -> Project {
    let mut p = Project::new();
    p.assets[0].skeleton_id = Some(sk.id);
    p.assets[0].skin_data = Some(petunia_project::compute_auto_skin_weights(
        &p.assets[0].mesh,
        &sk,
    ));
    p.add_skeleton(sk);
    p
}

#[test]
fn project_stores_validates_and_prunes_motions() {
    let mut p = project_with_rig(RigPreset::humanoid(1.0));
    let sid = p.skeletons[0].id;
    let mut m = MotionRecipe::new(sid, MotionGenerator::BipedCycle, "Walk");
    m.apply_style(MotionStyle::Heavy);
    let id = m.id;
    p.motions.push(m.clone());
    p.motions.push(m.clone()); // id duplicado
    p.motions.push(MotionRecipe::new(
        uuid::Uuid::new_v4(),
        MotionGenerator::Gait,
        "Órfão",
    ));
    let back: Project = serde_json::from_value(serde_json::to_value(&p).unwrap()).unwrap();
    assert_eq!(back.motions.len(), 3);
    p.validate();
    assert_eq!(p.motions.len(), 1, "duplicado e órfão removidos");
    assert_eq!(p.get_motion(id), Some(&m));

    // JSON antigo sem `motions` continua abrindo.
    let mut old = serde_json::to_value(&p).unwrap();
    old.as_object_mut().unwrap().remove("motions");
    assert!(
        serde_json::from_value::<Project>(old)
            .unwrap()
            .motions
            .is_empty()
    );

    // Avaliador e bake pelos papéis do projeto.
    assert!(p.motion_evaluator(id).is_ok());
    let baked = p.bake_motion(id, &BakeOptions::default()).unwrap();
    assert!(baked.keys_after > 0);
    p.remove_skeleton(sid);
    assert!(p.motions.is_empty());
    assert!(p.bake_motion(id, &BakeOptions::default()).is_err());
}

#[test]
fn gltf_export_bakes_motions_and_reports_the_ones_that_cannot_be() {
    use petunia_project::export::{export_gltf, export_report};
    use petunia_project::import_gltf::import_rig;
    let mut p = project_with_rig(RigPreset::humanoid(1.0));
    let sid = p.skeletons[0].id;
    let walk = MotionRecipe::new(sid, MotionGenerator::BipedCycle, "Walk");
    let bad = MotionRecipe::new(sid, MotionGenerator::Serpentine, "Slither");
    let wid = walk.id;
    p.motions.push(walk);
    p.motions.push(bad);

    // Movimento inexportável: o GLB sai sem ele e o relatório avisa.
    let report = export_report(&p, &[0], true);
    assert!(
        report
            .iter()
            .any(|l| l.contains("aviso") && l.contains("Slither")),
        "{report:?}"
    );
    let glb = export_gltf(&p, &[0]).unwrap();
    let rig = import_rig(&glb, "t", 1.0).unwrap().unwrap();
    assert_eq!(rig.animations.len(), 1);
    let clip = &rig.animations[0].clip;
    assert_eq!(rig.animations[0].name, "Walk");

    // O clipe importado reproduz o gerador nos instantes de amostragem.
    let ev = p.motion_evaluator(wid).unwrap();
    let sk = &rig.skeletons[0];
    let src = &p.skeletons[0];
    let n = (clip.duration * clip.fps).round() as usize;
    for i in (0..n).step_by(3) {
        let t = clip.duration * i as f32 / n as f32;
        let want = ev.pose_at(t);
        let got = clip.sample_pose(sk, t.min(clip.duration - 1e-4));
        for (bone, w) in src.bones.iter().zip(&want) {
            let g = &got[sk.bone_index(sk.find_bone(&bone.name).unwrap()).unwrap()];
            let ang = Quat::from_array(w.rotation).angle_between(Quat::from_array(g.rotation));
            assert!(ang < 0.02, "{} frame {i}: {ang}", bone.name);
            assert!((Vec3::from(w.translation) - Vec3::from(g.translation)).length() < 5e-3);
        }
    }
}
