//! Malha deformada por skin e ajuste de um rig ao modelo (cap. 45, AN-19).
//!
//! Três peças, todas **puras** (sem UI, sem GPU, sem tocar o documento):
//!
//! - [`PoseOverride`]: as malhas deformadas de um instante. É dado transitório
//!   do renderer — nunca é serializado nem entra em Undo. O documento continua
//!   guardando a malha **em repouso**; o preview substitui a malha só na hora de
//!   desenhar.
//! - [`posed_meshes`]: Linear Blend Skinning com **id de osso → índice**
//!   (AN-17: os pesos guardam ids, e ids não são índices depois de remover ossos).
//! - [`fit_skeleton_to_bounds`] + [`compute_blended_skin_weights`]: levam um
//!   rig de preset (qualquer criatura) ao volume de um modelo low-poly e o
//!   ligam à malha com pesos que seguem o osso mais próximo e só misturam perto
//!   das articulações — o que costuma ser o certo para low-poly.

use crate::rig::{Skeleton, SkinData, Transform3D, VertexSkinWeight};
use crate::{Asset, Project};
use glam::{Mat4, Vec3};
use petunia_mesh::Mesh;
use std::collections::HashMap;
use uuid::Uuid;

/// Fração da maior dimensão do rig usada como raio de mistura dos pesos.
pub const DEFAULT_BLEND_FRACTION: f32 = 0.06;

/// Malhas deformadas por asset num instante da pose.
///
/// `revision` identifica o quadro: o renderer só reconstrói buffers quando ela
/// muda, então quem produz o override incrementa a revisão a cada pose nova.
#[derive(Clone, Debug, Default)]
pub struct PoseOverride {
    pub revision: u64,
    meshes: HashMap<Uuid, Mesh>,
}

impl PoseOverride {
    pub fn new(revision: u64) -> Self {
        Self {
            revision,
            meshes: HashMap::new(),
        }
    }

    pub fn insert(&mut self, asset: Uuid, mesh: Mesh) {
        self.meshes.insert(asset, mesh);
    }

    /// Malha deformada do asset, se ele participa da pose.
    pub fn mesh_for(&self, asset: Uuid) -> Option<&Mesh> {
        self.meshes.get(&asset)
    }

    pub fn is_empty(&self) -> bool {
        self.meshes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.meshes.len()
    }

    /// Mesmos assets e mesmas posições de vértice (ignora `revision`). Quem
    /// produz o override usa isto para só avançar a revisão — e o renderer só
    /// reconstruir buffers — quando a pose realmente mudou.
    pub fn same_geometry(&self, other: &PoseOverride) -> bool {
        self.meshes.len() == other.meshes.len()
            && self.meshes.iter().all(|(id, a)| {
                other.meshes.get(id).is_some_and(|b| {
                    a.verts.len() == b.verts.len()
                        && a.verts.iter().zip(&b.verts).all(|(x, y)| x.pos == y.pos)
                })
            })
    }
}

/// Malha a desenhar para `asset`: a deformada, quando há; senão a avaliada
/// (com modifiers). Único ponto de decisão para os dois renderers.
pub fn mesh_to_draw<'a>(
    pose: Option<&'a PoseOverride>,
    asset: &'a Asset,
) -> std::borrow::Cow<'a, Mesh> {
    match pose.and_then(|p| p.mesh_for(asset.id)) {
        Some(mesh) => std::borrow::Cow::Borrowed(mesh),
        None => asset.evaluated_mesh_ref(),
    }
}

/// Linear Blend Skinning das **posições** de `base`.
///
/// `skinning` tem uma matriz por osso, na ordem de `skeleton.bones`
/// ([`Skeleton::compute_skinning_matrices`]). `None` se os pesos não casam com a
/// malha ou pertencem a outro esqueleto (sem deformação parcial).
pub fn skin_mesh(
    base: &Mesh,
    skin: &SkinData,
    skeleton: &Skeleton,
    skinning: &[Mat4],
) -> Option<Mesh> {
    if skin.skeleton_id != skeleton.id
        || skin.vertex_weights.len() != base.verts.len()
        || skinning.len() != skeleton.bones.len()
    {
        return None;
    }
    let index_of: HashMap<u32, usize> = skeleton
        .bones
        .iter()
        .enumerate()
        .map(|(i, b)| (b.id, i))
        .collect();
    let mut out = base.clone();
    for (vertex, weights) in out.verts.iter_mut().zip(&skin.vertex_weights) {
        let rest = Vec3::from(vertex.pos);
        let mut posed = Vec3::ZERO;
        let mut total = 0.0;
        for k in 0..4 {
            let w = weights.weights[k];
            if w <= 1e-6 {
                continue;
            }
            let Some(&bone) = index_of.get(&weights.bones[k]) else {
                continue;
            };
            posed += skinning[bone].transform_point3(rest) * w;
            total += w;
        }
        // Vértice sem influência válida fica em repouso, em vez de colapsar.
        vertex.pos = if total > 1e-6 {
            (posed / total).to_array()
        } else {
            vertex.pos
        };
    }
    Some(out)
}

/// Deforma todos os assets visíveis ligados a `skeleton_id` pela pose `local`
/// (uma transformação local por osso, ordem de `skeleton.bones`).
///
/// Assets com modifiers ativos ficam de fora: os pesos pertencem à malha-base e
/// a topologia avaliada pode ser outra (o preview mostra o asset em repouso, sem
/// deformação parcial).
pub fn posed_meshes(
    project: &Project,
    skeleton_id: Uuid,
    local: &[Transform3D],
    revision: u64,
) -> PoseOverride {
    let mut out = PoseOverride::new(revision);
    let Some(skeleton) = project.get_skeleton(skeleton_id) else {
        return out;
    };
    let Ok(skinning) = skeleton.compute_skinning_matrices(local) else {
        return out;
    };
    for asset in &project.assets {
        if !asset.visible || asset.has_enabled_modifiers() {
            continue;
        }
        let Some(skin) = asset.skin_data.as_ref() else {
            continue;
        };
        if let Some(mesh) = skin_mesh(&asset.mesh, skin, skeleton, &skinning) {
            out.insert(asset.id, mesh);
        }
    }
    out
}

/// Caixa envolvente das articulações do esqueleto (cabeças e caudas).
pub fn skeleton_bounds(skeleton: &Skeleton) -> Option<(Vec3, Vec3)> {
    bounds_of(
        skeleton
            .bones
            .iter()
            .flat_map(|b| [Vec3::from(b.head), Vec3::from(b.tail)]),
    )
}

/// Caixa envolvente dos vértices da malha.
pub fn mesh_bounds(mesh: &Mesh) -> Option<(Vec3, Vec3)> {
    bounds_of(mesh.verts.iter().map(|v| Vec3::from(v.pos)))
}

fn bounds_of(points: impl Iterator<Item = Vec3>) -> Option<(Vec3, Vec3)> {
    let mut iter = points.filter(|p| p.is_finite());
    let first = iter.next()?;
    Some(iter.fold((first, first), |(lo, hi), p| (lo.min(p), hi.max(p))))
}

/// Uma dimensão é "alongada" quando passa disto vezes a outra.
const ELONGATION: f32 = 1.25;

/// Leva o rig ao volume `[target_min, target_max]`: cada eixo dos limites atuais
/// é mapeado para o eixo do alvo (um eixo degenerado no rig apenas centraliza).
///
/// Se o rig é claramente alongado num eixo horizontal e o alvo no outro (um
/// quadrúpede modelado ao longo de X, por exemplo), o rig gira 90° em Y antes de
/// mapear — só quando todas as rotações de repouso são identidade, para não
/// contradizer os quadros dos ossos. Frente × costas não é inferível pelos
/// limites; a inversão fica para o usuário.
///
/// Recalcula a pose de bind. `false` se o rig não tem geometria finita.
pub fn fit_skeleton_to_bounds(skeleton: &mut Skeleton, target_min: Vec3, target_max: Vec3) -> bool {
    let Some((src_min, src_max)) = skeleton_bounds(skeleton) else {
        return false;
    };
    if !(target_min.is_finite() && target_max.is_finite()) || target_min.cmpgt(target_max).any() {
        return false;
    }
    let rest_is_identity = skeleton
        .bones
        .iter()
        .all(|b| b.local_transform.rotation == [0.0, 0.0, 0.0, 1.0]);
    let src_size = src_max - src_min;
    let dst_size = target_max - target_min;
    let src_center = (src_min + src_max) * 0.5;
    let turn = rest_is_identity && needs_quarter_turn(src_size, dst_size);

    let (mapped_min, mapped_size) = if turn {
        (
            Vec3::new(
                src_center.x - src_size.z * 0.5,
                src_min.y,
                src_center.z - src_size.x * 0.5,
            ),
            Vec3::new(src_size.z, src_size.y, src_size.x),
        )
    } else {
        (src_min, src_size)
    };
    let map = |p: Vec3| -> Vec3 {
        let p = if turn {
            // 90° em torno do centro do rig: (x, z) → (-(z - cz), (x - cx)).
            let d = p - src_center;
            Vec3::new(src_center.x - d.z, p.y, src_center.z + d.x)
        } else {
            p
        };
        let mut out = Vec3::ZERO;
        for axis in 0..3 {
            out[axis] = if mapped_size[axis] > 1e-6 {
                target_min[axis] + (p[axis] - mapped_min[axis]) / mapped_size[axis] * dst_size[axis]
            } else {
                target_min[axis] + dst_size[axis] * 0.5
            };
        }
        out
    };
    for bone in &mut skeleton.bones {
        bone.head = map(Vec3::from(bone.head)).to_array();
        bone.tail = map(Vec3::from(bone.tail)).to_array();
    }
    skeleton.compute_bind_pose_matrices();
    true
}

fn needs_quarter_turn(src: Vec3, dst: Vec3) -> bool {
    let src_long_x = src.x > src.z * ELONGATION;
    let src_long_z = src.z > src.x * ELONGATION;
    let dst_long_x = dst.x > dst.z * ELONGATION;
    let dst_long_z = dst.z > dst.x * ELONGATION;
    (src_long_x && dst_long_z) || (src_long_z && dst_long_x)
}

fn distance_to_segment(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_squared();
    if len_sq < 1e-9 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

/// Raio de mistura padrão para o esqueleto: [`DEFAULT_BLEND_FRACTION`] da maior
/// dimensão do rig.
pub fn default_blend_radius(skeleton: &Skeleton) -> f32 {
    skeleton_bounds(skeleton).map_or(0.05, |(lo, hi)| {
        ((hi - lo).max_element() * DEFAULT_BLEND_FRACTION).max(1e-3)
    })
}

/// Pesos de skin para low-poly: o osso mais próximo domina e os outros só entram
/// quando estão quase tão perto quanto ele (a menos de `blend_radius`). O peso do
/// osso *i* cai como `(1 − (dᵢ − d_min)/r)²`; ficam os 4 maiores, normalizados.
/// Determinístico (empates por id). Guarda **ids** de osso, como o Auto-Skin.
pub fn compute_blended_skin_weights(
    mesh: &Mesh,
    skeleton: &Skeleton,
    blend_radius: f32,
) -> SkinData {
    let mut skin = SkinData::new(skeleton.id, mesh.verts.len());
    if skeleton.bones.is_empty() {
        return skin;
    }
    let radius = blend_radius.max(1e-4);
    for (index, vertex) in mesh.verts.iter().enumerate() {
        let p = Vec3::from(vertex.pos);
        let mut ranked: Vec<(u32, f32)> = skeleton
            .bones
            .iter()
            .map(|b| {
                (
                    b.id,
                    distance_to_segment(p, Vec3::from(b.head), Vec3::from(b.tail)),
                )
            })
            .collect();
        ranked.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        let nearest = ranked[0].1;
        let mut bones = [0u32; 4];
        let mut weights = [0.0f32; 4];
        for (slot, (id, d)) in ranked.iter().take(4).enumerate() {
            let t = ((d - nearest) / radius).clamp(0.0, 1.0);
            bones[slot] = *id;
            weights[slot] = (1.0 - t) * (1.0 - t);
        }
        let mut vw = VertexSkinWeight { bones, weights };
        vw.normalize();
        skin.vertex_weights[index] = vw;
    }
    skin
}
