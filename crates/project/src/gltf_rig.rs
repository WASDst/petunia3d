//! glTF: skin, joints e animação de ossos (cap. 45 F0, P3D-135/138).
//!
//! Lado de **exportação** usado por [`crate::export::export_gltf`]: monta os nós
//! de joint, o `skin` (com `inverseBindMatrices`) e as `animations` a partir dos
//! `Skeleton`/`AnimationAsset` do projeto. O lado de importação vive na mesma
//! convenção (nós de joint com TRS local; canais `translation`/`rotation`/`scale`).
//!
//! Convenções (AN-16): o `local_transform` de cada osso é o **repouso local** do
//! joint; keyframes são valores locais absolutos. Os metadados do Petunia
//! (`bone_id`, `tail`, fps, loop) viajam em `extras.petunia` para round-trip.

use crate::export::ExportError;
use crate::rig::Skeleton;
use crate::{Asset, Project};
use serde_json::{Value, json};
use std::collections::HashMap;
use uuid::Uuid;

/// (bufferView, count, type, componentType, min/max).
pub(crate) type Acc = (usize, usize, String, usize, Option<(Vec<f32>, Vec<f32>)>);

pub(crate) const COMPONENT_F32: usize = 5126;
pub(crate) const COMPONENT_U16: usize = 5123;
pub(crate) const TARGET_ARRAY_BUFFER: u32 = 34962;

/// Acumula o chunk BIN, os bufferViews e os accessors de um GLB.
#[derive(Default)]
pub(crate) struct BinBuilder {
    pub bin: Vec<u8>,
    /// `(byteOffset, byteLength, target)`; `target == 0` omite o campo.
    pub views: Vec<(usize, usize, u32)>,
    pub accs: Vec<Acc>,
}

impl BinBuilder {
    pub fn align4(&mut self) {
        while !self.bin.len().is_multiple_of(4) {
            self.bin.push(0);
        }
    }

    pub fn push_view(&mut self, bytes: &[u8], target: u32) -> usize {
        self.align4();
        let off = self.bin.len();
        self.bin.extend_from_slice(bytes);
        self.views.push((off, bytes.len(), target));
        self.views.len() - 1
    }

    pub fn push_acc(
        &mut self,
        view: usize,
        count: usize,
        ty: &str,
        component: usize,
        minmax: Option<(Vec<f32>, Vec<f32>)>,
    ) -> usize {
        self.accs
            .push((view, count, ty.to_string(), component, minmax));
        self.accs.len() - 1
    }

    /// bufferView + accessor de floats (`ty` = SCALAR/VEC3/VEC4/MAT4).
    pub fn push_f32(
        &mut self,
        data: &[f32],
        ty: &str,
        components: usize,
        target: u32,
        minmax: Option<(Vec<f32>, Vec<f32>)>,
    ) -> usize {
        let view = self.push_view(bytemuck::cast_slice(data), target);
        self.push_acc(view, data.len() / components, ty, COMPONENT_F32, minmax)
    }
}

/// Pesos por vértice já resolvidos para índices de joint (`skin.joints`).
pub(crate) struct SkinBinding {
    pub skeleton_id: Uuid,
    pub joints: Vec<[u16; 4]>,
    pub weights: Vec<[f32; 4]>,
}

/// Motivo pelo qual um asset com pesos não é exportado com skin.
fn skin_problem<'a>(
    project: &'a Project,
    asset: &'a Asset,
    vert_count: usize,
) -> Result<Option<(&'a Skeleton, &'a crate::rig::SkinData)>, String> {
    let (Some(skeleton_id), Some(skin)) = (asset.skeleton_id, asset.skin_data.as_ref()) else {
        return Ok(None);
    };
    let Some(skeleton) = project.get_skeleton(skeleton_id) else {
        return Err(format!(
            "'{}': esqueleto ausente no projeto; exportado sem skin",
            asset.name
        ));
    };
    if skin.vertex_weights.len() != vert_count {
        return Err(format!(
            "'{}': a malha avaliada tem {vert_count} vértices e os pesos {}; \
             exportado sem skin (aplique os modifiers ou refaça o Auto-Skin)",
            asset.name,
            skin.vertex_weights.len()
        ));
    }
    if skeleton.bones.len() > usize::from(u16::MAX) {
        return Err(format!(
            "'{}': esqueleto com mais de 65535 ossos; exportado sem skin",
            asset.name
        ));
    }
    Ok(Some((skeleton, skin)))
}

/// Resolve os pesos do asset para `JOINTS_0`/`WEIGHTS_0`. `None` quando o asset
/// não tem skin exportável (veja [`skin_export_note`] para o motivo).
pub(crate) fn bind_skin(
    project: &Project,
    asset: &Asset,
    vert_count: usize,
) -> Option<SkinBinding> {
    let (skeleton, skin) = skin_problem(project, asset, vert_count).ok().flatten()?;
    let mut joints = Vec::with_capacity(vert_count);
    let mut weights = Vec::with_capacity(vert_count);
    for vw in &skin.vertex_weights {
        let mut j = [0u16; 4];
        let mut w = [0f32; 4];
        let mut k = 0;
        for i in 0..4 {
            let weight = vw.weights[i];
            if !(weight.is_finite() && weight > 1e-6) {
                continue;
            }
            // `bones` guarda IDs de osso (produzidos pelo Auto-Skin); o glTF pede a
            // posição no `skin.joints`.
            let Some(idx) = skeleton.bone_index(vw.bones[i]) else {
                continue;
            };
            j[k] = idx as u16;
            w[k] = weight;
            k += 1;
        }
        let sum: f32 = w.iter().sum();
        if sum > 1e-6 {
            for x in &mut w {
                *x /= sum;
            }
        } else {
            // Vértice sem influência válida: preso à raiz para não "voar".
            j = [0; 4];
            w = [1.0, 0.0, 0.0, 0.0];
        }
        joints.push(j);
        weights.push(w);
    }
    Some(SkinBinding {
        skeleton_id: skeleton.id,
        joints,
        weights,
    })
}

/// Aviso legível quando um asset com pesos é exportado **sem** skin.
pub fn skin_export_note(project: &Project, asset: &Asset) -> Option<String> {
    let verts = asset.evaluated_mesh().verts.len();
    skin_problem(project, asset, verts).err()
}

/// Nós de joint, skins e animações prontos para inserir no JSON do GLB.
#[derive(Default)]
pub(crate) struct RigDoc {
    /// skeleton id → índice em `skins`.
    pub skin_of: HashMap<Uuid, usize>,
    /// Nós de joint (índices a partir de `first_node`).
    pub nodes: Vec<Value>,
    /// Raízes dos esqueletos, para a `scene`.
    pub scene_roots: Vec<usize>,
    pub skins: Vec<Value>,
    pub animations: Vec<Value>,
}

fn finite(values: &[f32], what: &str) -> Result<(), ExportError> {
    if values.iter().all(|x| x.is_finite()) {
        Ok(())
    } else {
        Err(ExportError::Other(format!("{what} com NaN/inf")))
    }
}

fn normalized(q: [f32; 4]) -> [f32; 4] {
    let len = q.iter().map(|x| x * x).sum::<f32>().sqrt();
    if len > 1e-6 {
        [q[0] / len, q[1] / len, q[2] / len, q[3] / len]
    } else {
        [0.0, 0.0, 0.0, 1.0]
    }
}

/// Monta joints, skins e animações. `skeleton_ids` é a ordem dos skins;
/// `first_node` é o índice do primeiro nó de joint (após os nós de mesh).
pub(crate) fn build_rig_doc(
    project: &Project,
    skeleton_ids: &[Uuid],
    first_node: usize,
    bb: &mut BinBuilder,
) -> Result<RigDoc, ExportError> {
    let mut doc = RigDoc::default();
    let skeletons: Vec<&Skeleton> = skeleton_ids
        .iter()
        .filter_map(|id| project.get_skeleton(*id))
        .collect();

    // Índice do nó de cada osso, por esqueleto.
    let mut bases = Vec::with_capacity(skeletons.len());
    let mut next = first_node;
    for sk in &skeletons {
        bases.push(next);
        next += sk.bones.len();
    }

    for (si, sk) in skeletons.iter().enumerate() {
        let base = bases[si];
        let ids: HashMap<u32, usize> = sk
            .bones
            .iter()
            .enumerate()
            .map(|(i, b)| (b.id, i))
            .collect();
        let mut ibm: Vec<f32> = Vec::with_capacity(sk.bones.len() * 16);
        let mut first_root: Option<usize> = None;
        for (bi, bone) in sk.bones.iter().enumerate() {
            let t = bone.local_transform;
            finite(&t.translation, "joint (translação)")?;
            finite(&t.rotation, "joint (rotação)")?;
            finite(&t.scale, "joint (escala)")?;
            finite(&bone.tail, "osso (tail)")?;
            finite(&bone.inverse_bind_matrix, "osso (inverse bind)")?;
            ibm.extend_from_slice(&bone.inverse_bind_matrix);

            let children: Vec<usize> = sk
                .bones
                .iter()
                .enumerate()
                .filter(|(_, c)| c.parent == Some(bone.id))
                .map(|(ci, _)| base + ci)
                .collect();
            let is_root = bone.parent.is_none_or(|p| !ids.contains_key(&p));
            if is_root {
                doc.scene_roots.push(base + bi);
                first_root.get_or_insert(base + bi);
            }
            let mut node = json!({
                "name": bone.name,
                "translation": t.translation,
                "rotation": normalized(t.rotation),
                "scale": t.scale,
                "extras": { "petunia": { "bone_id": bone.id, "tail": bone.tail } },
            });
            if !children.is_empty() {
                node["children"] = json!(children);
            }
            doc.nodes.push(node);
        }
        let ibm_acc = bb.push_f32(&ibm, "MAT4", 16, 0, None);
        let joints: Vec<usize> = (0..sk.bones.len()).map(|i| base + i).collect();
        let mut skin = json!({
            "name": sk.name,
            "joints": joints,
            "inverseBindMatrices": ibm_acc,
            "extras": { "petunia": { "skeleton_id": sk.id } },
        });
        if let Some(root) = first_root {
            skin["skeleton"] = json!(root);
        }
        doc.skin_of.insert(sk.id, doc.skins.len());
        doc.skins.push(skin);
    }

    doc.animations = build_animations(project, &skeletons, &bases, bb)?;
    Ok(doc)
}

fn build_animations(
    project: &Project,
    skeletons: &[&Skeleton],
    bases: &[usize],
    bb: &mut BinBuilder,
) -> Result<Vec<Value>, ExportError> {
    let mut out = Vec::new();
    // Clipes não guardam o esqueleto: com um único esqueleto casa por ID de osso;
    // com vários exige ID **e** nome iguais.
    let strict = skeletons.len() > 1;

    for anim in &project.animations {
        let clip = &anim.clip;
        let mut samplers: Vec<Value> = Vec::new();
        let mut channels: Vec<Value> = Vec::new();
        let mut time_cache: HashMap<Vec<u32>, usize> = HashMap::new();

        for (si, sk) in skeletons.iter().enumerate() {
            for track in &clip.tracks {
                let Some(bi) = sk.bone_index(track.bone_id) else {
                    continue;
                };
                if strict && sk.bones[bi].name != track.bone_name {
                    continue;
                }
                let node = bases[si] + bi;
                let mut add = |path: &str,
                               times: Vec<f32>,
                               values: Vec<f32>,
                               ty: &str,
                               comps: usize,
                               step: bool|
                 -> Result<(), ExportError> {
                    if times.is_empty() {
                        return Ok(());
                    }
                    finite(&times, "keyframe (tempo)")?;
                    finite(&values, "keyframe (valor)")?;
                    if times.windows(2).any(|w| w[1] <= w[0]) || times[0] < 0.0 {
                        return Err(ExportError::Other(format!(
                            "clipe '{}': tempos de keyframe não crescentes no osso '{}'",
                            clip.name, track.bone_name
                        )));
                    }
                    let key: Vec<u32> = times.iter().map(|t| t.to_bits()).collect();
                    let input = *time_cache.entry(key).or_insert_with(|| {
                        let mm = (vec![times[0]], vec![times[times.len() - 1]]);
                        bb.push_f32(&times, "SCALAR", 1, 0, Some(mm))
                    });
                    let output = bb.push_f32(&values, ty, comps, 0, None);
                    samplers.push(json!({
                        "input": input,
                        "output": output,
                        "interpolation": if step { "STEP" } else { "LINEAR" },
                    }));
                    channels.push(json!({
                        "sampler": samplers.len() - 1,
                        "target": { "node": node, "path": path },
                    }));
                    Ok(())
                };

                let tr = &track.translations;
                add(
                    "translation",
                    tr.iter().map(|k| k.time).collect(),
                    tr.iter().flat_map(|k| k.value).collect(),
                    "VEC3",
                    3,
                    all_step(tr.iter().map(|k| k.interpolation)),
                )?;
                let ro = &track.rotations;
                add(
                    "rotation",
                    ro.iter().map(|k| k.time).collect(),
                    ro.iter().flat_map(|k| normalized(k.value)).collect(),
                    "VEC4",
                    4,
                    all_step(ro.iter().map(|k| k.interpolation)),
                )?;
                let sc = &track.scales;
                add(
                    "scale",
                    sc.iter().map(|k| k.time).collect(),
                    sc.iter().flat_map(|k| k.value).collect(),
                    "VEC3",
                    3,
                    all_step(sc.iter().map(|k| k.interpolation)),
                )?;
            }
        }
        if channels.is_empty() {
            continue;
        }
        out.push(json!({
            "name": anim.name,
            "samplers": samplers,
            "channels": channels,
            "extras": { "petunia": {
                "animation_id": anim.id,
                "clip_id": clip.id,
                "clip_name": clip.name,
                "duration": clip.duration,
                "fps": clip.fps,
                "looping": clip.looping,
            } },
        }));
    }
    Ok(out)
}

fn all_step(mut it: impl Iterator<Item = crate::animation::Interpolation>) -> bool {
    let mut any = false;
    let all = it.all(|i| {
        any = true;
        i == crate::animation::Interpolation::Step
    });
    any && all
}

// ---------------------------------------------------------------------------
// Importação
// ---------------------------------------------------------------------------

use crate::animation::{AnimationAsset, AnimationClip, Interpolation, Keyframe};
use crate::import_gltf::GltfImportError;
use crate::rig::{SkinData, VertexSkinWeight};
use glam::{Mat4, Quat, Vec3};

/// Pesos de skin de uma malha importada, casados pelo nome da malha.
#[derive(Clone, Debug)]
pub struct ImportedMeshSkin {
    /// Nome idêntico ao devolvido por `import_glb_bytes` para a mesma malha.
    pub mesh_name: String,
    /// Índice em [`ImportedRig::skeletons`].
    pub skeleton_index: usize,
    /// `skeleton_id` é o do esqueleto importado; `bones` guarda IDs de osso.
    pub skin: SkinData,
}

/// Esqueletos, pesos e clipes lidos de um GLB.
#[derive(Clone, Debug, Default)]
pub struct ImportedRig {
    pub skeletons: Vec<Skeleton>,
    pub mesh_skins: Vec<ImportedMeshSkin>,
    pub animations: Vec<AnimationAsset>,
    /// Limitações encontradas (nunca silenciosas).
    pub warnings: Vec<String>,
}

const MAX_JOINTS: usize = 4096;

fn extras_petunia(raw: &Option<Box<serde_json::value::RawValue>>) -> Option<Value> {
    let raw = raw.as_ref()?;
    let v: Value = serde_json::from_str(raw.get()).ok()?;
    v.get("petunia").cloned()
}

fn vec3_from(v: &Value) -> Option<Vec3> {
    let a = v.as_array()?;
    Some(Vec3::new(
        a.first()?.as_f64()? as f32,
        a.get(1)?.as_f64()? as f32,
        a.get(2)?.as_f64()? as f32,
    ))
}

/// Lê skeletons, pesos e animações. `None` quando o arquivo não tem skin nem
/// animação de nós. `scale` multiplica posições e translações (como a malha).
pub fn import_rig(
    data: &[u8],
    name_hint: &str,
    scale: f32,
) -> Result<Option<ImportedRig>, GltfImportError> {
    let gltf = gltf::Gltf::from_slice(data).map_err(|e| GltfImportError::Parse(e.to_string()))?;
    if gltf.skins().len() == 0 && gltf.animations().len() == 0 {
        return Ok(None);
    }
    let blob = gltf.blob.as_deref();
    let get = |b: gltf::Buffer| if b.index() == 0 { blob } else { None };
    let mut rig = ImportedRig::default();

    // Mundo de cada nó (cena padrão ou nós sem pai).
    let n_nodes = gltf.nodes().len();
    let mut parent: Vec<Option<usize>> = vec![None; n_nodes];
    for n in gltf.nodes() {
        for c in n.children() {
            parent[c.index()] = Some(n.index());
        }
    }
    let mut world = vec![Mat4::IDENTITY; n_nodes];
    fn fill(node: gltf::Node, up: Mat4, out: &mut [Mat4]) {
        let w = up * Mat4::from_cols_array_2d(&node.transform().matrix());
        out[node.index()] = w;
        for c in node.children() {
            fill(c, w, out);
        }
    }
    for n in gltf.nodes().filter(|n| parent[n.index()].is_none()) {
        fill(n, Mat4::IDENTITY, &mut world);
    }

    // (skin gltf -> índice do esqueleto), (nó -> (esqueleto, osso)).
    let mut skin_to_skeleton: HashMap<usize, usize> = HashMap::new();
    let mut joint_to_bone: Vec<Vec<u32>> = Vec::new();
    let mut node_bone: HashMap<usize, (usize, u32)> = HashMap::new();

    for (k, skin) in gltf.skins().enumerate() {
        let joints: Vec<gltf::Node> = skin.joints().collect();
        if joints.is_empty() || joints.len() > MAX_JOINTS {
            return Err(GltfImportError::Validation(format!(
                "skin {k}: número de joints inválido ({})",
                joints.len()
            )));
        }
        let joint_index: HashMap<usize, usize> = joints
            .iter()
            .enumerate()
            .map(|(j, n)| (n.index(), j))
            .collect();
        let ibm: Option<Vec<Mat4>> = skin
            .reader(get)
            .read_inverse_bind_matrices()
            .map(|it| it.map(|m| Mat4::from_cols_array_2d(&m)).collect::<Vec<_>>());
        if let Some(ibm) = &ibm
            && ibm.len() < joints.len()
        {
            return Err(GltfImportError::Validation(format!(
                "skin {k}: inverseBindMatrices insuficientes"
            )));
        }

        // Bind mundial de cada joint: inversa da IBM (espaço da malha) ou repouso do nó.
        let bind: Vec<Mat4> = (0..joints.len())
            .map(|j| match &ibm {
                Some(ibm) => ibm[j].inverse(),
                None => world[joints[j].index()],
            })
            .collect();
        if ibm.is_some()
            && joints
                .iter()
                .zip(&bind)
                .any(|(n, b)| !world[n.index()].abs_diff_eq(*b, 1e-3))
        {
            rig.warnings.push(format!(
                "skin {k}: a pose de repouso dos nós difere da pose de bind; \
                 a pose de bind (inverseBindMatrices) foi usada como repouso"
            ));
        }
        let mut scaled_warned = false;
        let mut world_rot = Vec::with_capacity(joints.len());
        let mut heads = Vec::with_capacity(joints.len());
        for b in &bind {
            let (s, r, t) = b.to_scale_rotation_translation();
            if !scaled_warned && (s - Vec3::ONE).abs().max_element() > 1e-3 {
                scaled_warned = true;
                rig.warnings.push(format!(
                    "skin {k}: escala de repouso diferente de 1 ignorada (rigs com escala no bind não são suportados)"
                ));
            }
            world_rot.push(r);
            heads.push(t * scale);
        }
        let parent_joint: Vec<Option<usize>> = joints
            .iter()
            .map(|n| {
                parent[n.index()]
                    .and_then(|p| joint_index.get(&p).copied())
                    .filter(|&p| p != joint_index[&n.index()])
            })
            .collect();
        for (j, n) in joints.iter().enumerate() {
            if let Some(p) = parent[n.index()]
                && parent_joint[j].is_none()
                && !world[p].abs_diff_eq(Mat4::IDENTITY, 1e-4)
            {
                rig.warnings.push(format!(
                    "skin {k}: o joint '{}' tem ancestral fora do esqueleto com transformação; \
                     ela é ignorada nas animações da raiz",
                    n.name().unwrap_or("?")
                ));
                break;
            }
        }

        // Nomes únicos e ordem topológica (pais antes dos filhos).
        let mut used: std::collections::HashSet<String> = Default::default();
        let names: Vec<String> = joints
            .iter()
            .enumerate()
            .map(|(j, n)| {
                let base = n
                    .name()
                    .filter(|s| !s.is_empty())
                    .map_or_else(|| format!("Bone_{j}"), str::to_string);
                let mut name = base.clone();
                let mut i = 1;
                while !used.insert(name.clone()) {
                    name = format!("{base}.{i:03}");
                    i += 1;
                }
                name
            })
            .collect();

        let mut skeleton = Skeleton::new(
            skin.name()
                .filter(|s| !s.is_empty())
                .map_or_else(|| format!("{name_hint}_rig{k}"), str::to_string),
        );
        let mut bone_of: Vec<Option<u32>> = vec![None; joints.len()];
        let mut remaining = joints.len();
        while remaining > 0 {
            let mut progressed = false;
            for j in 0..joints.len() {
                if bone_of[j].is_some() {
                    continue;
                }
                let parent_bone = match parent_joint[j] {
                    None => None,
                    Some(p) => match bone_of[p] {
                        Some(id) => Some(id),
                        None => continue,
                    },
                };
                // Tail: extras do Petunia, senão o primeiro filho, senão um segmento curto.
                let extras = extras_petunia(joints[j].extras());
                let tail = extras
                    .as_ref()
                    .and_then(|e| e.get("tail"))
                    .and_then(vec3_from)
                    .map(|t| t * scale)
                    .or_else(|| {
                        (0..joints.len())
                            .find(|&c| parent_joint[c] == Some(j))
                            .map(|c| heads[c])
                    })
                    .unwrap_or_else(|| heads[j] + Vec3::Y * 0.1 * scale.max(1e-3));
                let id = skeleton
                    .add_bone(
                        names[j].clone(),
                        parent_bone,
                        heads[j].to_array(),
                        tail.to_array(),
                    )
                    .map_err(|e| GltfImportError::Validation(e.to_string()))?;
                bone_of[j] = Some(id);
                remaining -= 1;
                progressed = true;
            }
            if !progressed {
                return Err(GltfImportError::Validation(format!(
                    "skin {k}: hierarquia de joints cíclica"
                )));
            }
        }
        // Rotação local de repouso = rot_mundial(pai)⁻¹ · rot_mundial.
        for j in 0..joints.len() {
            let id = bone_of[j].expect("resolvido");
            let parent_rot = parent_joint[j].map_or(Quat::IDENTITY, |p| world_rot[p]);
            let q = (parent_rot.inverse() * world_rot[j]).normalize();
            if let Some(b) = skeleton.get_bone_mut(id) {
                b.local_transform.rotation = [q.x, q.y, q.z, q.w];
            }
        }
        skeleton.compute_bind_pose_matrices();

        let ids: Vec<u32> = bone_of.iter().map(|b| b.expect("resolvido")).collect();
        for (j, n) in joints.iter().enumerate() {
            node_bone.insert(n.index(), (rig.skeletons.len(), ids[j]));
        }
        skin_to_skeleton.insert(skin.index(), rig.skeletons.len());
        joint_to_bone.push(ids);
        rig.skeletons.push(skeleton);
    }

    import_mesh_skins(
        &gltf,
        name_hint,
        &get,
        &skin_to_skeleton,
        &joint_to_bone,
        &mut rig,
    );
    import_animations(&gltf, &get, scale, &node_bone, &mut rig);
    Ok(Some(rig))
}

fn import_mesh_skins<'s, F>(
    gltf: &gltf::Gltf,
    name_hint: &str,
    get: &F,
    skin_to_skeleton: &HashMap<usize, usize>,
    joint_to_bone: &[Vec<u32>],
    rig: &mut ImportedRig,
) where
    F: for<'b> Fn(gltf::Buffer<'b>) -> Option<&'s [u8]>,
{
    let mut done: std::collections::HashSet<usize> = Default::default();
    for node in gltf.nodes() {
        let (Some(mesh), Some(skin)) = (node.mesh(), node.skin()) else {
            continue;
        };
        let Some(&skel_i) = skin_to_skeleton.get(&skin.index()) else {
            continue;
        };
        if !done.insert(mesh.index()) {
            continue;
        }
        let bone_ids = &joint_to_bone[skel_i];
        let root = rig.skeletons[skel_i].bones.first().map_or(0, |b| b.id);
        let mut weights_out: Vec<VertexSkinWeight> = Vec::new();
        let mut missing_attrs = false;
        for prim in mesh.primitives() {
            let reader = prim.reader(get);
            let Some(positions) = reader.read_positions() else {
                continue; // mesma regra de `import_glb_bytes`
            };
            let n = positions.len();
            let joints: Option<Vec<[u16; 4]>> =
                reader.read_joints(0).map(|j| j.into_u16().collect());
            let weights: Option<Vec<[f32; 4]>> =
                reader.read_weights(0).map(|w| w.into_f32().collect());
            if reader.read_joints(1).is_some() {
                rig.warnings.push(format!(
                    "malha '{}': mais de 4 influências por vértice; apenas as 4 primeiras foram usadas",
                    mesh.name().unwrap_or("?")
                ));
            }
            for vi in 0..n {
                let (Some(j), Some(w)) = (
                    joints.as_ref().and_then(|j| j.get(vi)),
                    weights.as_ref().and_then(|w| w.get(vi)),
                ) else {
                    missing_attrs = true;
                    weights_out.push(VertexSkinWeight::new([root, 0, 0, 0], [1.0, 0.0, 0.0, 0.0]));
                    continue;
                };
                let mut bones = [0u32; 4];
                let mut ws = [0f32; 4];
                let mut k = 0;
                for i in 0..4 {
                    let wi = w[i];
                    if !(wi.is_finite() && wi > 1e-6) {
                        continue;
                    }
                    let Some(&id) = bone_ids.get(usize::from(j[i])) else {
                        continue;
                    };
                    bones[k] = id;
                    ws[k] = wi;
                    k += 1;
                }
                let sum: f32 = ws.iter().sum();
                if sum > 1e-6 {
                    for x in &mut ws {
                        *x /= sum;
                    }
                } else {
                    bones = [root, 0, 0, 0];
                    ws = [1.0, 0.0, 0.0, 0.0];
                }
                weights_out.push(VertexSkinWeight::new(bones, ws));
            }
        }
        if weights_out.is_empty() {
            continue;
        }
        if missing_attrs {
            rig.warnings.push(format!(
                "malha '{}': skin sem JOINTS_0/WEIGHTS_0 em algum primitivo; vértices presos à raiz",
                mesh.name().unwrap_or("?")
            ));
        }
        rig.mesh_skins.push(ImportedMeshSkin {
            mesh_name: mesh
                .name()
                .map(str::to_string)
                .unwrap_or_else(|| format!("{name_hint}_{}", mesh.index())),
            skeleton_index: skel_i,
            skin: SkinData {
                skeleton_id: rig.skeletons[skel_i].id,
                vertex_weights: weights_out,
            },
        });
    }
}

fn import_animations<'s, F>(
    gltf: &gltf::Gltf,
    get: &F,
    scale: f32,
    node_bone: &HashMap<usize, (usize, u32)>,
    rig: &mut ImportedRig,
) where
    F: for<'b> Fn(gltf::Buffer<'b>) -> Option<&'s [u8]>,
{
    use gltf::animation::util::ReadOutputs;
    use gltf::animation::{Interpolation as GInterp, Property};

    for (ai, anim) in gltf.animations().enumerate() {
        let meta = extras_petunia(anim.extras());
        let mut clips: std::collections::BTreeMap<usize, AnimationClip> = Default::default();
        let mut ignored = 0usize;
        let mut cubic = false;
        let mut max_time = 0f32;

        for ch in anim.channels() {
            let Some(&(skel_i, bone_id)) = node_bone.get(&ch.target().node().index()) else {
                ignored += 1;
                continue;
            };
            let reader = ch.reader(get);
            let (Some(times), Some(outputs)) = (reader.read_inputs(), reader.read_outputs()) else {
                ignored += 1;
                continue;
            };
            let times: Vec<f32> = times.collect();
            let interp = ch.sampler().interpolation();
            let stride = if interp == GInterp::CubicSpline { 3 } else { 1 };
            cubic |= interp == GInterp::CubicSpline;
            let mode = match interp {
                GInterp::Step => Interpolation::Step,
                GInterp::Linear => Interpolation::Linear,
                GInterp::CubicSpline => Interpolation::Cubic,
            };
            // Com CUBICSPLINE cada chave tem [in-tangent, valor, out-tangent].
            let pick = |i: usize| if stride == 3 { i * 3 + 1 } else { i };

            let bone_name = rig.skeletons[skel_i]
                .get_bone(bone_id)
                .map(|b| b.name.clone())
                .unwrap_or_default();
            let clip = clips.entry(skel_i).or_insert_with(|| {
                let mut c = AnimationClip::new(anim.name().unwrap_or("Animation"), 0.1);
                c.looping = true;
                c
            });
            let track = clip.get_or_create_track(bone_id, &bone_name);
            max_time = times.iter().copied().fold(max_time, f32::max);
            match (ch.target().property(), outputs) {
                (Property::Translation, ReadOutputs::Translations(it)) => {
                    let vals: Vec<[f32; 3]> = it.collect();
                    for (i, &t) in times.iter().enumerate() {
                        if let Some(v) = vals.get(pick(i)) {
                            let v = [v[0] * scale, v[1] * scale, v[2] * scale];
                            track.translations.push(Keyframe::with_interp(t, v, mode));
                        }
                    }
                }
                (Property::Rotation, ReadOutputs::Rotations(it)) => {
                    let vals: Vec<[f32; 4]> = it.into_f32().collect();
                    for (i, &t) in times.iter().enumerate() {
                        if let Some(&q) = vals.get(pick(i)) {
                            track.rotations.push(Keyframe::with_interp(t, q, mode));
                        }
                    }
                }
                (Property::Scale, ReadOutputs::Scales(it)) => {
                    let vals: Vec<[f32; 3]> = it.collect();
                    for (i, &t) in times.iter().enumerate() {
                        if let Some(&v) = vals.get(pick(i)) {
                            track.scales.push(Keyframe::with_interp(t, v, mode));
                        }
                    }
                }
                _ => ignored += 1, // morph target weights etc.
            }
        }

        if ignored > 0 {
            rig.warnings.push(format!(
                "animação '{}': {ignored} canal(is) ignorado(s) (fora do esqueleto ou tipo não suportado)",
                anim.name().unwrap_or("?")
            ));
        }
        if cubic {
            rig.warnings.push(format!(
                "animação '{}': CUBICSPLINE convertido em chaves suaves (tangentes descartadas)",
                anim.name().unwrap_or("?")
            ));
        }
        let multiple = clips.len() > 1;
        for (skel_i, mut clip) in clips {
            let meta_f = |k: &str| meta.as_ref().and_then(|m| m.get(k)).and_then(Value::as_f64);
            clip.duration = meta_f("duration")
                .map_or(max_time, |d| d as f32)
                .max(max_time)
                .max(0.1);
            if let Some(fps) = meta_f("fps").filter(|f| f.is_finite() && *f > 0.0) {
                clip.fps = fps as f32;
            }
            if let Some(l) = meta
                .as_ref()
                .and_then(|m| m.get("looping"))
                .and_then(Value::as_bool)
            {
                clip.looping = l;
            }
            let base = meta
                .as_ref()
                .and_then(|m| m.get("clip_name"))
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| anim.name().map(str::to_string))
                .unwrap_or_else(|| format!("Animation_{ai}"));
            clip.name = if multiple {
                format!("{base}_{}", rig.skeletons[skel_i].name)
            } else {
                base
            };
            let asset_name = clip.name.clone();
            rig.animations.push(AnimationAsset::new(asset_name, clip));
        }
    }
}
