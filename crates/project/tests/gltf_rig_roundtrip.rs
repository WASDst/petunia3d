//! Round-trip de skin, joints e animação em GLB (cap. 45 F0, AN-13; P3D-124).

use glam::Mat4;
use petunia_project::export::export_gltf;
use petunia_project::import_gltf::{import_glb_bytes, import_rig};
use petunia_project::pipeline::{DeliveryPipeline, ImportOptions};
use petunia_project::{
    AnimationAsset, AnimationLibrary, Project, RigPreset, Skeleton, Transform3D,
    compute_auto_skin_weights,
};
use std::collections::BTreeMap;

fn rigged_project() -> (Project, Skeleton) {
    let mut p = Project::new();
    let skel = RigPreset::humanoid(1.0);
    let mesh = p.assets[0].mesh.clone();
    p.assets[0].skin_data = Some(compute_auto_skin_weights(&mesh, &skel));
    p.assets[0].skeleton_id = Some(skel.id);
    p.animations.push(AnimationAsset::new(
        "Walk",
        AnimationLibrary::humanoid_walk(&skel),
    ));
    p.animations.push(AnimationAsset::new(
        "Idle",
        AnimationLibrary::humanoid_idle(&skel),
    ));
    p.add_skeleton(skel.clone());
    (p, skel)
}

fn parent_name(sk: &Skeleton, name: &str) -> Option<String> {
    let bone = sk.bones.iter().find(|b| b.name == name)?;
    bone.parent
        .and_then(|p| sk.get_bone(p))
        .map(|b| b.name.clone())
}

#[test]
fn skeleton_round_trips_names_hierarchy_heads_and_bind() {
    let (p, skel) = rigged_project();
    let glb = export_gltf(&p, &[0]).unwrap();
    let rig = import_rig(&glb, "t", 1.0).unwrap().expect("rig");
    assert!(rig.warnings.is_empty(), "{:?}", rig.warnings);
    assert_eq!(rig.skeletons.len(), 1);
    let got = &rig.skeletons[0];
    assert_eq!(got.bones.len(), skel.bones.len());
    for b in &skel.bones {
        let g = got.bones.iter().find(|x| x.name == b.name).expect(&b.name);
        for i in 0..3 {
            assert!((g.head[i] - b.head[i]).abs() < 1e-4, "{} head", b.name);
            assert!((g.tail[i] - b.tail[i]).abs() < 1e-4, "{} tail", b.name);
        }
        assert_eq!(parent_name(got, &b.name), parent_name(&skel, &b.name));
        let (gm, bm) = (
            Mat4::from_cols_array(&g.inverse_bind_matrix),
            Mat4::from_cols_array(&b.inverse_bind_matrix),
        );
        assert!(gm.abs_diff_eq(bm, 1e-4), "{} inverse bind", b.name);
    }
    // Repouso importado ⇒ skinning identidade.
    let rest: Vec<Transform3D> = got.bones.iter().map(|b| b.local_transform).collect();
    for m in got.compute_skinning_matrices(&rest).unwrap() {
        assert!(m.abs_diff_eq(Mat4::IDENTITY, 1e-4));
    }
}

#[test]
fn skin_weights_follow_the_exported_vertices() {
    let (p, skel) = rigged_project();
    let glb = export_gltf(&p, &[0]).unwrap();
    let meshes = import_glb_bytes(&glb, "t", false, 1.0).unwrap();
    let rig = import_rig(&glb, "t", 1.0).unwrap().unwrap();
    assert_eq!(rig.mesh_skins.len(), 1);
    let ms = &rig.mesh_skins[0];
    let (name, mesh) = &meshes[0];
    assert_eq!(&ms.mesh_name, name);
    assert_eq!(ms.skin.vertex_weights.len(), mesh.verts.len());
    assert_eq!(ms.skin.skeleton_id, rig.skeletons[0].id);
    ms.skin
        .validate(mesh.verts.len(), rig.skeletons[0].bones.len())
        .expect("pesos válidos");

    // Cada vértice importado tem os mesmos pesos (por nome de osso) que o vértice
    // original de mesma posição.
    let orig_mesh = &p.assets[0].mesh;
    let orig = p.assets[0].skin_data.as_ref().unwrap();
    let by_name = |sk: &Skeleton, vw: &petunia_project::VertexSkinWeight| {
        let mut m: BTreeMap<String, f32> = BTreeMap::new();
        for i in 0..4 {
            if vw.weights[i] > 1e-6 {
                let n = sk.get_bone(vw.bones[i]).unwrap().name.clone();
                *m.entry(n).or_default() += vw.weights[i];
            }
        }
        m
    };
    for (vi, v) in mesh.verts.iter().enumerate() {
        let oi = orig_mesh
            .verts
            .iter()
            .position(|o| o.pos == v.pos)
            .expect("posição original");
        let a = by_name(&skel, &orig.vertex_weights[oi]);
        let b = by_name(&rig.skeletons[0], &ms.skin.vertex_weights[vi]);
        assert_eq!(a.len(), b.len(), "vértice {vi}: {a:?} vs {b:?}");
        for (k, w) in &a {
            assert!((w - b[k]).abs() < 1e-4, "vértice {vi} osso {k}");
        }
    }
}

#[test]
fn animations_round_trip_keys_and_clip_metadata() {
    let (mut p, skel) = rigged_project();
    p.animations[0].clip.fps = 30.0;
    p.animations[0].clip.looping = false;
    let glb = export_gltf(&p, &[0]).unwrap();
    let rig = import_rig(&glb, "t", 1.0).unwrap().unwrap();
    assert_eq!(rig.animations.len(), 2);
    let sk = &rig.skeletons[0];

    let walk = rig
        .animations
        .iter()
        .find(|a| a.name == "Humanoid_Walk")
        .expect("walk");
    assert_eq!(walk.clip.fps, 30.0);
    assert!(!walk.clip.looping);
    assert!((walk.clip.duration - 1.0).abs() < 1e-6);
    assert_eq!(walk.clip.tracks.len(), 2);
    for tr in &walk.clip.tracks {
        let src = p.animations[0]
            .clip
            .tracks
            .iter()
            .find(|t| t.bone_name == tr.bone_name)
            .unwrap();
        assert_eq!(sk.get_bone(tr.bone_id).unwrap().name, tr.bone_name);
        assert_eq!(tr.rotations.len(), src.rotations.len());
        for (a, b) in tr.rotations.iter().zip(&src.rotations) {
            assert!((a.time - b.time).abs() < 1e-6);
            for i in 0..4 {
                assert!((a.value[i] - b.value[i]).abs() < 1e-5);
            }
        }
    }

    // Mesma pose no mesmo instante, osso a osso (por nome).
    let orig_clip = &p.animations[0].clip;
    for t in [0.0f32, 0.2, 0.5, 0.8] {
        let a = orig_clip.sample_pose(&skel, t);
        let b = walk.clip.sample_pose(sk, t);
        for (bone, ta) in skel.bones.iter().zip(&a) {
            let tb = &b[sk.bones.iter().position(|x| x.name == bone.name).unwrap()];
            for i in 0..3 {
                assert!((ta.translation[i] - tb.translation[i]).abs() < 1e-4);
            }
            let dot: f32 = (0..4).map(|i| ta.rotation[i] * tb.rotation[i]).sum();
            assert!(dot.abs() > 0.9999, "{} t={t}", bone.name);
        }
    }
}

#[test]
fn rest_rotations_survive_the_round_trip() {
    let (mut p, mut skel) = rigged_project();
    let id = skel.find_bone("UpperArm.L").unwrap();
    let q = glam::Quat::from_rotation_z(0.6);
    skel.get_bone_mut(id).unwrap().local_transform.rotation = [q.x, q.y, q.z, q.w];
    skel.compute_bind_pose_matrices();
    p.skeletons.clear();
    p.assets[0].skeleton_id = Some(skel.id);
    p.animations.clear();
    p.add_skeleton(skel.clone());

    let glb = export_gltf(&p, &[0]).unwrap();
    let rig = import_rig(&glb, "t", 1.0).unwrap().unwrap();
    let got = &rig.skeletons[0];
    let g = got.bones.iter().find(|b| b.name == "UpperArm.L").unwrap();
    let dot: f32 = (0..4)
        .map(|i| {
            g.local_transform.rotation[i] * skel.get_bone(id).unwrap().local_transform.rotation[i]
        })
        .sum();
    assert!(dot.abs() > 0.99999);
    let rest: Vec<Transform3D> = got.bones.iter().map(|b| b.local_transform).collect();
    for m in got.compute_skinning_matrices(&rest).unwrap() {
        assert!(m.abs_diff_eq(Mat4::IDENTITY, 1e-4));
    }
    for b in &skel.bones {
        let gb = got.bones.iter().find(|x| x.name == b.name).unwrap();
        assert!(
            Mat4::from_cols_array(&gb.inverse_bind_matrix)
                .abs_diff_eq(Mat4::from_cols_array(&b.inverse_bind_matrix), 1e-4),
            "{}",
            b.name
        );
    }
}

#[test]
fn import_scale_scales_heads_and_translations() {
    let (p, skel) = rigged_project();
    let glb = export_gltf(&p, &[0]).unwrap();
    let rig = import_rig(&glb, "t", 2.0).unwrap().unwrap();
    let hips = rig.skeletons[0]
        .bones
        .iter()
        .find(|b| b.name == "Hips")
        .unwrap();
    let src = skel.bones.iter().find(|b| b.name == "Hips").unwrap();
    assert!((hips.head[1] - src.head[1] * 2.0).abs() < 1e-4);
    let idle = rig
        .animations
        .iter()
        .find(|a| a.name == "Humanoid_Idle")
        .unwrap();
    let chest = rig.skeletons[0].find_bone("Chest").unwrap();
    let src_chest = skel.find_bone("Chest").unwrap();
    let a = idle.clip.get_track(chest).unwrap().translations[1].value[1];
    let b = p.animations[1]
        .clip
        .get_track(src_chest)
        .unwrap()
        .translations[1]
        .value[1];
    assert!((a - b * 2.0).abs() < 1e-4);
}

#[test]
fn glb_without_rig_imports_no_rig() {
    let p = Project::new();
    let glb = export_gltf(&p, &[0]).unwrap();
    assert!(import_rig(&glb, "t", 1.0).unwrap().is_none());
}

#[test]
fn pipeline_import_attaches_skeleton_weights_and_clips_to_the_project() {
    let (p, _) = rigged_project();
    let glb = export_gltf(&p, &[0]).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hero.glb");
    std::fs::write(&path, &glb).unwrap();

    let payload = DeliveryPipeline::new()
        .import_file(&path, &ImportOptions::default())
        .expect("import");
    assert_eq!(payload.meshes.len(), 1);
    let mut target = Project::new();
    let first = target.assets.len();
    for (name, mesh) in payload.meshes {
        target.add(&name, mesh);
    }
    let warnings = target.add_imported_rig(payload.rig.expect("rig"), first);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(target.skeletons.len(), 1);
    assert_eq!(target.animations.len(), 2);
    let asset = target.assets.last().unwrap();
    assert_eq!(asset.skeleton_id, Some(target.skeletons[0].id));
    let skin = asset.skin_data.as_ref().expect("skin");
    skin.validate(asset.mesh.verts.len(), target.skeletons[0].bones.len())
        .expect("pesos válidos");

    // E o resultado volta a ser exportável (re-export do rig importado).
    let again = export_gltf(&target, &[target.assets.len() - 1]).unwrap();
    let rig2 = import_rig(&again, "t", 1.0).unwrap().unwrap();
    assert_eq!(
        rig2.skeletons[0].bones.len(),
        target.skeletons[0].bones.len()
    );
    assert_eq!(rig2.animations.len(), 2);
}

/// GLB mínimo "externo": 2 joints sob um nó `Armature` transladado, malha de 3
/// vértices com JOINTS_0 em UNSIGNED_BYTE, animação CUBICSPLINE de rotação e um
/// canal que aponta para um nó fora do esqueleto.
fn foreign_glb() -> Vec<u8> {
    fn f32s(v: &[f32]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_le_bytes()).collect()
    }
    let mut bin: Vec<u8> = Vec::new();
    let mut views: Vec<(usize, usize)> = Vec::new();
    let mut push = |bytes: Vec<u8>| -> usize {
        while !bin.len().is_multiple_of(4) {
            bin.push(0);
        }
        views.push((bin.len(), bytes.len()));
        bin.extend_from_slice(&bytes);
        views.len() - 1
    };
    // 0 posições, 1 joints (u8 x4), 2 weights, 3 IBM, 4 tempos, 5 rotações cubic, 6 translação
    let _pos = push(f32s(&[0.0, 1.0, 0.0, 0.0, 2.0, 0.0, 1.0, 1.5, 0.0]));
    let _joints = push(vec![0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0]);
    let _weights = push(f32s(&[
        1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.5, 0.5, 0.0, 0.0,
    ]));
    let _ibm = push({
        let a = Mat4::from_translation(glam::Vec3::new(5.0, 1.0, 0.0)).inverse();
        let b = Mat4::from_translation(glam::Vec3::new(5.0, 2.0, 0.0)).inverse();
        f32s(&[a.to_cols_array(), b.to_cols_array()].concat())
    });
    let _times = push(f32s(&[0.0, 1.0]));
    let q = |z: f32| {
        let q = glam::Quat::from_rotation_z(z);
        [q.x, q.y, q.z, q.w]
    };
    let mut cubic = Vec::new();
    for z in [0.0f32, 0.9] {
        cubic.extend_from_slice(&[0.0, 0.0, 0.0, 0.0]); // in-tangent
        cubic.extend_from_slice(&q(z)); // valor
        cubic.extend_from_slice(&[0.0, 0.0, 0.0, 0.0]); // out-tangent
    }
    let _rot = push(f32s(&cubic));
    let _trans = push(f32s(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0]));

    let v = |i: usize| views[i];
    let json = format!(
        r#"{{"asset":{{"version":"2.0"}},"scene":0,"scenes":[{{"nodes":[0,3]}}],
"nodes":[
 {{"name":"Armature","translation":[5,0,0],"children":[1]}},
 {{"name":"Root","translation":[0,1,0],"children":[2]}},
 {{"name":"Tip","translation":[0,1,0]}},
 {{"name":"Body","mesh":0,"skin":0}}],
"meshes":[{{"name":"Body","primitives":[{{"attributes":{{"POSITION":0,"JOINTS_0":1,"WEIGHTS_0":2}}}}]}}],
"skins":[{{"name":"Rig","joints":[1,2],"inverseBindMatrices":3}}],
"animations":[{{"name":"Bend",
 "samplers":[{{"input":4,"output":5,"interpolation":"CUBICSPLINE"}},{{"input":4,"output":6,"interpolation":"LINEAR"}}],
 "channels":[{{"sampler":0,"target":{{"node":2,"path":"rotation"}}}},{{"sampler":1,"target":{{"node":3,"path":"translation"}}}}]}}],
"accessors":[
 {{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,1,0],"max":[1,2,0]}},
 {{"bufferView":1,"componentType":5121,"count":3,"type":"VEC4"}},
 {{"bufferView":2,"componentType":5126,"count":3,"type":"VEC4"}},
 {{"bufferView":3,"componentType":5126,"count":2,"type":"MAT4"}},
 {{"bufferView":4,"componentType":5126,"count":2,"type":"SCALAR","min":[0],"max":[1]}},
 {{"bufferView":5,"componentType":5126,"count":6,"type":"VEC4"}},
 {{"bufferView":6,"componentType":5126,"count":2,"type":"VEC3"}}],
"bufferViews":[
 {{"buffer":0,"byteOffset":{},"byteLength":{}}},{{"buffer":0,"byteOffset":{},"byteLength":{}}},
 {{"buffer":0,"byteOffset":{},"byteLength":{}}},{{"buffer":0,"byteOffset":{},"byteLength":{}}},
 {{"buffer":0,"byteOffset":{},"byteLength":{}}},{{"buffer":0,"byteOffset":{},"byteLength":{}}},
 {{"buffer":0,"byteOffset":{},"byteLength":{}}}],
"buffers":[{{"byteLength":{}}}]}}"#,
        v(0).0,
        v(0).1,
        v(1).0,
        v(1).1,
        v(2).0,
        v(2).1,
        v(3).0,
        v(3).1,
        v(4).0,
        v(4).1,
        v(5).0,
        v(5).1,
        v(6).0,
        v(6).1,
        {
            while !bin.len().is_multiple_of(4) {
                bin.push(0);
            }
            bin.len()
        }
    );
    let mut json = json.into_bytes();
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }
    let total = 12 + 8 + json.len() + 8 + bin.len();
    let mut out = Vec::new();
    out.extend_from_slice(&0x46546C67u32.to_le_bytes());
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x4E4F534Au32.to_le_bytes());
    out.extend_from_slice(&json);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x004E4942u32.to_le_bytes());
    out.extend_from_slice(&bin);
    out
}

#[test]
fn foreign_glb_imports_with_honest_warnings() {
    let glb = foreign_glb();
    let rig = import_rig(&glb, "ext", 1.0).unwrap().expect("rig");
    let all = rig.warnings.join(" | ");
    assert!(all.contains("ancestral"), "{all}");
    assert!(all.contains("CUBICSPLINE"), "{all}");
    assert!(all.contains("ignorado(s)"), "{all}");

    // Esqueleto no espaço da malha (bind = inversa da IBM), hierarquia preservada.
    let sk = &rig.skeletons[0];
    assert_eq!(sk.name, "Rig");
    let root = sk.bones.iter().find(|b| b.name == "Root").unwrap();
    let tip = sk.bones.iter().find(|b| b.name == "Tip").unwrap();
    assert_eq!(root.head, [5.0, 1.0, 0.0]);
    assert_eq!(tip.head, [5.0, 2.0, 0.0]);
    assert_eq!(tip.parent, Some(root.id));
    assert_eq!(root.tail, tip.head);

    // Pesos: JOINTS_0 em u8 mapeados para IDs de osso.
    let ms = &rig.mesh_skins[0];
    assert_eq!(ms.mesh_name, "Body");
    let w = &ms.skin.vertex_weights;
    assert_eq!(w.len(), 3);
    assert_eq!((w[0].bones[0], w[0].weights[0]), (root.id, 1.0));
    assert_eq!((w[1].bones[0], w[1].weights[0]), (tip.id, 1.0));
    assert!((w[2].weights[0] - 0.5).abs() < 1e-6 && (w[2].weights[1] - 0.5).abs() < 1e-6);

    // CUBICSPLINE: usa o valor central de cada chave e marca a interpolação.
    let clip = &rig.animations[0].clip;
    assert_eq!(rig.animations[0].name, "Bend");
    let track = clip.get_track(tip.id).expect("trilha do Tip");
    assert_eq!(track.rotations.len(), 2);
    assert!(
        track
            .rotations
            .iter()
            .all(|k| k.interpolation == petunia_project::Interpolation::Cubic)
    );
    let expected = glam::Quat::from_rotation_z(0.9);
    assert!((track.rotations[1].value[2] - expected.z).abs() < 1e-5);
    assert!((clip.duration - 1.0).abs() < 1e-6);
    // O canal do nó fora do esqueleto não vira trilha.
    assert_eq!(clip.tracks.len(), 1);
}

#[test]
fn roles_round_trip_through_gltf_and_are_inferred_for_foreign_rigs() {
    use petunia_project::{ArmPart, RigRole};
    let (mut p, skel) = rigged_project();
    // Correção manual: a mão esquerda vira acessório com movimento secundário.
    let hand = skel.find_bone("Hand.L").unwrap();
    p.assign_rig_role(skel.id, hand, RigRole::Wiggle(0))
        .unwrap();
    let glb = export_gltf(&p, &[0]).unwrap();
    let rig = import_rig(&glb, "t", 1.0).unwrap().unwrap();
    let sk = &rig.skeletons[0];
    let roles = &rig.roles[0];
    assert_eq!(roles.skeleton_id, sk.id);
    let hand2 = sk.find_bone("Hand.L").unwrap();
    assert_eq!(
        roles.role_of(hand2),
        Some(RigRole::Wiggle(0)),
        "papel manual"
    );
    let hand_r = sk.find_bone("Hand.R").unwrap();
    assert_eq!(
        roles.role_of(hand_r),
        Some(RigRole::Arm {
            limb: 1,
            part: ArmPart::Hand
        })
    );
    assert_eq!(roles.legs().len(), 2);

    // Entra no projeto com os papéis.
    let mut target = Project::new();
    let first = target.assets.len();
    let meshes = import_glb_bytes(&glb, "t", false, 1.0).unwrap();
    for (name, mesh) in meshes {
        target.add(&name, mesh);
    }
    target.add_imported_rig(rig, first);
    let sid = target.skeletons[0].id;
    assert_eq!(target.rig_roles_of(sid).unwrap().legs().len(), 2);

    // GLB externo (sem extras de papéis): inferência por nome; "Root" é raiz.
    let foreign = import_rig(&foreign_glb(), "ext", 1.0).unwrap().unwrap();
    let fsk = &foreign.skeletons[0];
    let root = fsk.find_bone("Root").unwrap();
    assert_eq!(foreign.roles[0].role_of(root), Some(RigRole::Root));
    assert_eq!(
        foreign.roles[0].role_of(fsk.find_bone("Tip").unwrap()),
        None
    );
}
