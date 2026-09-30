//! Malha deformada por skin e ajuste de rig ao modelo (cap. 45, AN-19).

use glam::Vec3;
use petunia_mesh::Mesh;
use petunia_project::{
    MotionGenerator, MotionRecipe, PoseOverride, Project, RigPreset, Skeleton, Transform3D,
    compute_blended_skin_weights, default_blend_radius, fit_skeleton_to_bounds, mesh_bounds,
    posed_meshes, skeleton_bounds, skin_mesh,
};

fn rest_pose(skeleton: &Skeleton) -> Vec<Transform3D> {
    skeleton.bones.iter().map(|b| b.local_transform).collect()
}

/// Caixa alta e fina, do tamanho aproximado de um personagem low-poly.
fn tall_box(min: [f32; 3], max: [f32; 3]) -> Mesh {
    let mut mesh = Mesh::cube(1.0);
    for v in &mut mesh.verts {
        for axis in 0..3 {
            let t = if v.pos[axis] >= 0.0 { 1.0 } else { 0.0 };
            v.pos[axis] = min[axis] + (max[axis] - min[axis]) * t;
        }
    }
    mesh
}

/// Duas ossos empilhados: raiz 0..1 e filho 1..2, com o id do filho ≠ índice.
fn two_bones() -> Skeleton {
    let mut s = Skeleton::new("two");
    let root = s
        .add_bone("Root", None, [0.0, 0.0, 0.0], [0.0, 1.0, 0.0])
        .unwrap();
    s.add_bone("Tip", Some(root), [0.0, 1.0, 0.0], [0.0, 2.0, 0.0])
        .unwrap();
    // ID que não é índice: prova o mapeamento id → índice (AN-17).
    s.bones[1].id = 42;
    s.compute_bind_pose_matrices();
    s
}

fn stick_mesh() -> Mesh {
    tall_box([-0.25, 0.0, -0.25], [0.25, 2.0, 0.25])
}

fn weights_by_height(mesh: &Mesh, skeleton: &Skeleton) -> petunia_project::SkinData {
    let mut skin = petunia_project::SkinData::new(skeleton.id, mesh.verts.len());
    for (i, v) in mesh.verts.iter().enumerate() {
        let id = if v.pos[1] > 0.5 { 42 } else { 0 };
        skin.vertex_weights[i] =
            petunia_project::VertexSkinWeight::new([id, 0, 0, 0], [1.0, 0.0, 0.0, 0.0]);
    }
    skin
}

#[test]
fn rest_pose_leaves_the_mesh_untouched() {
    let s = two_bones();
    let mesh = stick_mesh();
    let skin = weights_by_height(&mesh, &s);
    let m = s.compute_skinning_matrices(&rest_pose(&s)).unwrap();
    let out = skin_mesh(&mesh, &skin, &s, &m).unwrap();
    for (a, b) in mesh.verts.iter().zip(&out.verts) {
        assert!(Vec3::from(a.pos).distance(Vec3::from(b.pos)) < 1e-5);
    }
}

#[test]
fn weights_reference_bone_ids_not_indices() {
    let s = two_bones();
    let mesh = stick_mesh();
    let skin = weights_by_height(&mesh, &s);
    let mut pose = rest_pose(&s);
    pose[1].translation[2] += 1.0; // só o osso 42 anda 1 em Z
    let m = s.compute_skinning_matrices(&pose).unwrap();
    let out = skin_mesh(&mesh, &skin, &s, &m).unwrap();
    for (a, b) in mesh.verts.iter().zip(&out.verts) {
        let moved = b.pos[2] - a.pos[2];
        if a.pos[1] > 0.5 {
            assert!((moved - 1.0).abs() < 1e-5, "vértice do osso 42 anda");
        } else {
            assert!(moved.abs() < 1e-5, "vértice da raiz fica");
        }
    }
}

#[test]
fn rotating_a_bone_swings_its_vertices_about_the_joint() {
    let s = two_bones();
    let mesh = stick_mesh();
    let skin = weights_by_height(&mesh, &s);
    let mut pose = rest_pose(&s);
    let q = glam::Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
    pose[1].rotation = q.to_array();
    let m = s.compute_skinning_matrices(&pose).unwrap();
    let out = skin_mesh(&mesh, &skin, &s, &m).unwrap();
    let joint = Vec3::new(0.0, 1.0, 0.0);
    for (a, b) in mesh.verts.iter().zip(&out.verts) {
        if a.pos[1] > 0.5 {
            let (da, db) = (
                Vec3::from(a.pos).distance(joint),
                Vec3::from(b.pos).distance(joint),
            );
            assert!(
                (da - db).abs() < 1e-4,
                "rotação rígida preserva a distância à junta"
            );
        }
    }
    // A ponta de cima (y = 2) passa a apontar para ±Z: nada mais está em y = 2.
    let top_after = out.verts.iter().map(|v| v.pos[1]).fold(f32::MIN, f32::max);
    assert!(top_after < 1.4, "a metade de cima tombou: {top_after}");
}

#[test]
fn skinning_refuses_mismatched_data_instead_of_deforming_partially() {
    let s = two_bones();
    let mesh = stick_mesh();
    let m = s.compute_skinning_matrices(&rest_pose(&s)).unwrap();
    let mut skin = weights_by_height(&mesh, &s);
    skin.vertex_weights.pop();
    assert!(
        skin_mesh(&mesh, &skin, &s, &m).is_none(),
        "contagem de vértices"
    );
    let mut other = weights_by_height(&mesh, &s);
    other.skeleton_id = uuid::Uuid::new_v4();
    assert!(
        skin_mesh(&mesh, &other, &s, &m).is_none(),
        "outro esqueleto"
    );
    let ok = weights_by_height(&mesh, &s);
    assert!(
        skin_mesh(&mesh, &ok, &s, &m[..1]).is_none(),
        "matrizes faltando"
    );
}

#[test]
fn posed_meshes_only_include_bound_visible_unmodified_assets() {
    let mut project = Project::default();
    let skeleton = two_bones();
    let sid = skeleton.id;
    project.add_skeleton(skeleton);
    let mesh = stick_mesh();
    let skin = weights_by_height(&mesh, project.get_skeleton(sid).unwrap());

    project.add("Bound", mesh.clone());
    let bound_id = project.assets.last().unwrap().id;
    project.assets.last_mut().unwrap().skin_data = Some(skin.clone());
    project.add("Hidden", mesh.clone());
    project.assets.last_mut().unwrap().skin_data = Some(skin.clone());
    project.assets.last_mut().unwrap().visible = false;
    project.add("Unbound", mesh);

    let skeleton = project.get_skeleton(sid).unwrap();
    let pose = rest_pose(skeleton);
    let over = posed_meshes(&project, sid, &pose, 7);
    assert_eq!(over.revision, 7);
    assert_eq!(over.len(), 1);
    assert!(over.mesh_for(bound_id).is_some());
    assert!(posed_meshes(&project, uuid::Uuid::new_v4(), &pose, 1).is_empty());
    assert!(
        posed_meshes(&project, sid, &pose[..1], 1).is_empty(),
        "pose incompleta"
    );
}

#[test]
fn every_creature_fits_the_target_volume_and_rest_skinning_is_identity() {
    let creatures: Vec<(&str, Skeleton)> = vec![
        ("humanoid", RigPreset::humanoid(1.0)),
        ("quadruped", RigPreset::quadruped(1.0)),
        ("spider", RigPreset::multi_leg(8, 1.0)),
        ("serpent", RigPreset::serpent(12, 1.0)),
        ("fish", RigPreset::fish(1.0)),
        ("bird", RigPreset::bird(1.0)),
    ];
    let (lo, hi) = (Vec3::new(-1.5, 0.2, -0.9), Vec3::new(2.5, 3.0, 0.9));
    for (name, mut skeleton) in creatures {
        assert!(fit_skeleton_to_bounds(&mut skeleton, lo, hi), "{name}");
        let (a, b) = skeleton_bounds(&skeleton).unwrap();
        // Cada eixo ocupa exatamente o alvo (eixos degenerados centralizam).
        for axis in 0..3 {
            let inside = a[axis] >= lo[axis] - 1e-3 && b[axis] <= hi[axis] + 1e-3;
            assert!(inside, "{name} eixo {axis}: {a:?} {b:?}");
        }
        // Repouso continua sendo identidade depois do fit (bind recalculado).
        let m = skeleton
            .compute_skinning_matrices(&rest_pose(&skeleton))
            .unwrap();
        for mat in m {
            assert!(
                mat.abs_diff_eq(glam::Mat4::IDENTITY, 1e-4),
                "{name}: skinning de repouso não é identidade"
            );
        }
    }
}

#[test]
fn fit_rejects_empty_or_invalid_targets() {
    let mut s = RigPreset::humanoid(1.0);
    assert!(
        !fit_skeleton_to_bounds(&mut s, Vec3::ONE, Vec3::ZERO),
        "min > max"
    );
    assert!(!fit_skeleton_to_bounds(
        &mut s,
        Vec3::splat(f32::NAN),
        Vec3::ONE
    ));
    let mut empty = Skeleton::new("empty");
    assert!(!fit_skeleton_to_bounds(&mut empty, Vec3::ZERO, Vec3::ONE));
}

#[test]
fn a_creature_modelled_along_the_other_axis_is_turned_a_quarter() {
    let mut quad = RigPreset::quadruped(1.0);
    let (lo, hi) = skeleton_bounds(&quad).unwrap();
    let size = hi - lo;
    let along_z = size.z > size.x;
    let head = |s: &Skeleton| {
        let h = s
            .bones
            .iter()
            .find(|b| b.name.contains("Head"))
            .expect("Head");
        let hips = s
            .bones
            .iter()
            .find(|b| b.name.contains("Hips") || b.name.contains("Root"))
            .expect("Hips");
        (Vec3::from(h.head) - Vec3::from(hips.head)).abs()
    };
    let before = head(&quad);
    // Alvo alongado no eixo oposto ao do preset.
    let target = if along_z {
        (Vec3::new(0.0, 0.0, 0.0), Vec3::new(4.0, 2.0, 1.5))
    } else {
        (Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.5, 2.0, 4.0))
    };
    assert!(fit_skeleton_to_bounds(&mut quad, target.0, target.1));
    let after = head(&quad);
    if along_z {
        assert!(
            before.z >= before.x && after.x > after.z,
            "girou: {before:?} → {after:?}"
        );
    } else {
        assert!(
            before.x >= before.z && after.z > after.x,
            "girou: {before:?} → {after:?}"
        );
    }

    // Mesma orientação: nada gira.
    let mut same = RigPreset::quadruped(1.0);
    let target = if along_z {
        (Vec3::ZERO, Vec3::new(1.5, 2.0, 4.0))
    } else {
        (Vec3::ZERO, Vec3::new(4.0, 2.0, 1.5))
    };
    fit_skeleton_to_bounds(&mut same, target.0, target.1);
    let kept = head(&same);
    if along_z {
        assert!(kept.z >= kept.x);
    } else {
        assert!(kept.x >= kept.z);
    }
}

#[test]
fn blended_weights_are_rigid_mid_bone_and_blend_at_the_joint() {
    let s = two_bones();
    let mesh = stick_mesh();
    let r = default_blend_radius(&s);
    assert!(r > 0.0);
    let skin = compute_blended_skin_weights(&mesh, &s, 0.3);
    assert_eq!(skin.vertex_weights.len(), mesh.verts.len());
    for (v, w) in mesh.verts.iter().zip(&skin.vertex_weights) {
        let sum: f32 = w.weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-4);
        assert!(w.weights.iter().all(|x| x.is_finite() && *x >= 0.0));
        // Longe da junta (y = 1) o vértice segue um osso só.
        if (v.pos[1] - 1.0).abs() > 0.6 {
            assert!(
                w.weights[0] > 0.999,
                "vértice em y={} devia ser rígido: {w:?}",
                v.pos[1]
            );
        }
    }
    // Um vértice exatamente na junta divide entre os dois ossos.
    let mut at_joint = Mesh::cube(0.1);
    for v in &mut at_joint.verts {
        v.pos = [0.0, 1.0, 0.0];
    }
    let joint = compute_blended_skin_weights(&at_joint, &s, 0.3);
    let w = joint.vertex_weights[0];
    assert!(
        (w.weights[0] - w.weights[1]).abs() < 1e-3 && w.weights[1] > 0.4,
        "{w:?}"
    );

    // Determinismo.
    assert_eq!(skin, compute_blended_skin_weights(&mesh, &s, 0.3));
}

#[test]
fn blended_weights_survive_degenerate_bones() {
    let mut s = Skeleton::new("d");
    s.add_bone("A", None, [0.0; 3], [0.0; 3]).unwrap();
    s.add_bone("B", Some(0), [0.0, 1.0, 0.0], [0.0, 1.0, 0.0])
        .unwrap();
    let skin = compute_blended_skin_weights(&stick_mesh(), &s, 0.2);
    for w in &skin.vertex_weights {
        assert!(w.weights.iter().all(|x| x.is_finite()));
        assert!((w.weights.iter().sum::<f32>() - 1.0).abs() < 1e-4);
    }
    let none = compute_blended_skin_weights(&stick_mesh(), &Skeleton::new("empty"), 0.2);
    assert_eq!(none.vertex_weights.len(), stick_mesh().verts.len());
}

#[test]
fn a_walking_humanoid_deforms_its_model_without_exploding() {
    let mut project = Project::default();
    let mesh = tall_box([-0.4, 0.0, -0.2], [0.4, 1.9, 0.2]);
    let (lo, hi) = mesh_bounds(&mesh).unwrap();
    project.add("Hero", mesh.clone());
    let mut skeleton = RigPreset::humanoid(1.0);
    assert!(fit_skeleton_to_bounds(&mut skeleton, lo, hi));
    let skin = compute_blended_skin_weights(&mesh, &skeleton, default_blend_radius(&skeleton));
    let sid = skeleton.id;
    project.add_skeleton(skeleton);
    let asset = project.assets.last_mut().unwrap();
    asset.skin_data = Some(skin);
    asset.skeleton_id = Some(sid);
    let asset_id = asset.id;

    let recipe = MotionRecipe::new(sid, MotionGenerator::BipedCycle, "Walk");
    let motion_id = recipe.id;
    project.motions.push(recipe);
    let evaluator = project.motion_evaluator(motion_id).unwrap();

    let rest = posed_meshes(
        &project,
        sid,
        &rest_pose(project.get_skeleton(sid).unwrap()),
        0,
    );
    let rest_mesh = rest.mesh_for(asset_id).unwrap();
    let mut max_shift = 0.0f32;
    let extent = (hi - lo).length();
    for step in 0..12 {
        let t = step as f32 * evaluator.cycle_seconds() / 12.0;
        let over = posed_meshes(&project, sid, &evaluator.pose_at(t), step as u64);
        let posed = over.mesh_for(asset_id).expect("o modelo participa da pose");
        assert_eq!(posed.verts.len(), mesh.verts.len());
        for (p, r) in posed.verts.iter().zip(&rest_mesh.verts) {
            assert!(p.pos.iter().all(|v| v.is_finite()));
            let shift = Vec3::from(p.pos).distance(Vec3::from(r.pos));
            max_shift = max_shift.max(shift);
            assert!(shift < extent, "vértice voou {shift} (extensão {extent})");
        }
    }
    assert!(
        max_shift > 0.02,
        "a caminhada precisa mover o modelo: {max_shift}"
    );
}

#[test]
fn pose_override_is_plain_transient_data() {
    let mut o = PoseOverride::new(3);
    assert!(o.is_empty());
    let id = uuid::Uuid::new_v4();
    o.insert(id, Mesh::cube(1.0));
    assert_eq!((o.len(), o.revision), (1, 3));
    assert!(o.mesh_for(id).is_some() && o.mesh_for(uuid::Uuid::new_v4()).is_none());
    // A malha desenhada cai na avaliada quando o asset não está no override.
    let asset = petunia_project::Asset::new("A", Mesh::cube(1.0));
    let plain = petunia_project::mesh_to_draw(None, &asset);
    assert_eq!(plain.verts.len(), asset.mesh.verts.len());
    let other = petunia_project::mesh_to_draw(Some(&o), &asset);
    assert_eq!(other.verts.len(), asset.mesh.verts.len());
}
