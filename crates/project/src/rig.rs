//! Petunia3D — Skeleton & Rig Core (P3D-135)
//!
//! Núcleo desacoplado de esqueletos, ossos, hierarquias, bind poses e pesos de skinning.
//! 100% livre de dependências de interface gráfica.

use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use uuid::Uuid;

/// Erros estruturados do subsistema de Skeleton & Rigging.
#[derive(Debug, Error, PartialEq)]
pub enum RigError {
    #[error("Osso com ID {0} não encontrado")]
    BoneNotFound(u32),

    #[error("Osso com o nome '{0}' já existe no esqueleto")]
    BoneNameAlreadyExists(String),

    #[error(
        "Ciclo detectado na hierarquia: osso {bone_id} não pode ser filho do osso {target_parent}"
    )]
    HierarchyCycleDetected { bone_id: u32, target_parent: u32 },

    #[error("Pai inválido {0}: não existe no esqueleto")]
    InvalidParent(u32),

    #[error("Pesos de skinning inválidos no vértice {vertex_index}: {reason}")]
    InvalidWeights { vertex_index: usize, reason: String },

    #[error("Contagem incompatível de vértices: malha possui {expected}, skin data possui {found}")]
    VertexCountMismatch { expected: usize, found: usize },

    #[error(
        "Contagem incompatível de matrizes de skinning: esqueleto possui {expected}, fornecidas {found}"
    )]
    MatrixCountMismatch { expected: usize, found: usize },
}

/// Transformação tridimensional desacoplada: translação, rotação (quaternion) e escala.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Transform3D {
    pub translation: [f32; 3],
    /// Quaternion no formato `[x, y, z, w]`.
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

impl Default for Transform3D {
    fn default() -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0], // Identidade
            scale: [1.0, 1.0, 1.0],
        }
    }
}

impl Transform3D {
    pub fn new(translation: [f32; 3], rotation: [f32; 4], scale: [f32; 3]) -> Self {
        Self {
            translation,
            rotation,
            scale,
        }
    }

    pub fn from_translation(t: [f32; 3]) -> Self {
        Self {
            translation: t,
            ..Default::default()
        }
    }

    pub fn from_euler_degrees(pitch: f32, yaw: f32, roll: f32) -> Self {
        let q = Quat::from_euler(
            glam::EulerRot::XYZ,
            pitch.to_radians(),
            yaw.to_radians(),
            roll.to_radians(),
        );
        Self {
            rotation: [q.x, q.y, q.z, q.w],
            ..Default::default()
        }
    }

    pub fn to_mat4(&self) -> Mat4 {
        let t = Vec3::from(self.translation);
        let r = Quat::from_xyzw(
            self.rotation[0],
            self.rotation[1],
            self.rotation[2],
            self.rotation[3],
        )
        .normalize();
        let s = Vec3::from(self.scale);
        Mat4::from_scale_rotation_translation(s, r, t)
    }

    pub fn from_mat4(m: Mat4) -> Self {
        let (s, r, t) = m.to_scale_rotation_translation();
        Self {
            translation: [t.x, t.y, t.z],
            rotation: [r.x, r.y, r.z, r.w],
            scale: [s.x, s.y, s.z],
        }
    }

    /// Interpolação linear de translação/escala e slerp de rotação.
    pub fn lerp(&self, other: &Self, factor: f32) -> Self {
        let factor = factor.clamp(0.0, 1.0);
        let t0 = Vec3::from(self.translation);
        let t1 = Vec3::from(other.translation);
        let t = t0.lerp(t1, factor);

        let q0 = Quat::from_xyzw(
            self.rotation[0],
            self.rotation[1],
            self.rotation[2],
            self.rotation[3],
        );
        let q1 = Quat::from_xyzw(
            other.rotation[0],
            other.rotation[1],
            other.rotation[2],
            other.rotation[3],
        );
        let r = q0.slerp(q1, factor).normalize();

        let s0 = Vec3::from(self.scale);
        let s1 = Vec3::from(other.scale);
        let s = s0.lerp(s1, factor);

        Self {
            translation: [t.x, t.y, t.z],
            rotation: [r.x, r.y, r.z, r.w],
            scale: [s.x, s.y, s.z],
        }
    }
}

/// Osso ou articulação (Joint) dentro de um esqueleto.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Bone {
    pub id: u32,
    pub name: String,
    pub parent: Option<u32>,
    /// Posição inicial da articulação no espaço do esqueleto (Rest Pose).
    pub head: [f32; 3],
    /// Posição final da articulação no espaço do esqueleto (Rest Pose).
    pub tail: [f32; 3],
    /// Transformação local em relação ao pai.
    pub local_transform: Transform3D,
    /// Matriz Inversa de Bind (4x4 em formato column-major linear).
    pub inverse_bind_matrix: [f32; 16],
}

impl Bone {
    pub fn new(
        id: u32,
        name: impl Into<String>,
        parent: Option<u32>,
        head: [f32; 3],
        tail: [f32; 3],
    ) -> Self {
        Self {
            id,
            name: name.into(),
            parent,
            head,
            tail,
            local_transform: Transform3D::default(),
            inverse_bind_matrix: Mat4::IDENTITY.to_cols_array(),
        }
    }

    /// Comprimento do osso da cabeça à ponta.
    pub fn length(&self) -> f32 {
        Vec3::from(self.head).distance(Vec3::from(self.tail))
    }

    /// Direção unitária do osso.
    pub fn direction(&self) -> [f32; 3] {
        let dir = (Vec3::from(self.tail) - Vec3::from(self.head)).normalize_or_zero();
        [dir.x, dir.y, dir.z]
    }
}

/// Esqueleto completo com hierarquia acíclica de ossos.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Skeleton {
    pub id: Uuid,
    pub name: String,
    pub bones: Vec<Bone>,
    next_bone_id: u32,
}

impl Default for Skeleton {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Armature".to_string(),
            bones: Vec::new(),
            next_bone_id: 0,
        }
    }
}

impl Skeleton {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            bones: Vec::new(),
            next_bone_id: 0,
        }
    }

    /// Adiciona um novo osso ao esqueleto.
    pub fn add_bone(
        &mut self,
        name: impl Into<String>,
        parent: Option<u32>,
        head: [f32; 3],
        tail: [f32; 3],
    ) -> Result<u32, RigError> {
        let name_str = name.into();
        if self.bones.iter().any(|b| b.name == name_str) {
            return Err(RigError::BoneNameAlreadyExists(name_str));
        }
        if let Some(p) = parent.filter(|&p| !self.bones.iter().any(|b| b.id == p)) {
            return Err(RigError::InvalidParent(p));
        }

        let bone_id = self.next_bone_id;
        self.next_bone_id += 1;

        let bone = Bone::new(bone_id, name_str, parent, head, tail);
        self.bones.push(bone);
        self.compute_bind_pose_matrices();
        Ok(bone_id)
    }

    /// Busca um osso por ID.
    pub fn get_bone(&self, id: u32) -> Option<&Bone> {
        self.bones.iter().find(|b| b.id == id)
    }

    /// Busca mutável de um osso por ID.
    pub fn get_bone_mut(&mut self, id: u32) -> Option<&mut Bone> {
        self.bones.iter_mut().find(|b| b.id == id)
    }

    /// Busca o índice do osso no vetor interno por ID.
    pub fn bone_index(&self, id: u32) -> Option<usize> {
        self.bones.iter().position(|b| b.id == id)
    }

    /// Busca ID do osso pelo nome.
    pub fn find_bone(&self, name: &str) -> Option<u32> {
        self.bones.iter().find(|b| b.name == name).map(|b| b.id)
    }

    /// Retorna os IDs dos filhos diretos de um determinado osso.
    pub fn children_of(&self, parent_id: u32) -> Vec<u32> {
        self.bones
            .iter()
            .filter(|b| b.parent == Some(parent_id))
            .map(|b| b.id)
            .collect()
    }

    /// Remove um osso e reatribui seus filhos ao pai do osso removido.
    pub fn remove_bone(&mut self, bone_id: u32) -> Result<(), RigError> {
        let idx = self
            .bones
            .iter()
            .position(|b| b.id == bone_id)
            .ok_or(RigError::BoneNotFound(bone_id))?;

        let old_parent = self.bones[idx].parent;
        self.bones.remove(idx);

        for b in &mut self.bones {
            if b.parent == Some(bone_id) {
                b.parent = old_parent;
            }
        }

        self.compute_bind_pose_matrices();
        Ok(())
    }

    /// Reatribui o pai de um osso, prevenindo estritamente ciclos na hierarquia.
    pub fn reparent_bone(&mut self, bone_id: u32, new_parent: Option<u32>) -> Result<(), RigError> {
        if !self.bones.iter().any(|b| b.id == bone_id) {
            return Err(RigError::BoneNotFound(bone_id));
        }

        if let Some(target) = new_parent {
            if target == bone_id {
                return Err(RigError::HierarchyCycleDetected {
                    bone_id,
                    target_parent: target,
                });
            }
            if !self.bones.iter().any(|b| b.id == target) {
                return Err(RigError::InvalidParent(target));
            }

            // Verificar se target é descendente de bone_id
            let mut curr = Some(target);
            while let Some(p_id) = curr {
                if p_id == bone_id {
                    return Err(RigError::HierarchyCycleDetected {
                        bone_id,
                        target_parent: target,
                    });
                }
                curr = self
                    .bones
                    .iter()
                    .find(|b| b.id == p_id)
                    .and_then(|b| b.parent);
            }
        }

        if let Some(b) = self.bones.iter_mut().find(|b| b.id == bone_id) {
            b.parent = new_parent;
        }

        self.compute_bind_pose_matrices();
        Ok(())
    }

    /// Valida a integridade do esqueleto: unicidade de nomes, integridade de pais e ausência de ciclos.
    pub fn validate(&self) -> Result<(), RigError> {
        let mut names = HashSet::new();
        for b in &self.bones {
            if !names.insert(&b.name) {
                return Err(RigError::BoneNameAlreadyExists(b.name.clone()));
            }
        }

        for b in &self.bones {
            if let Some(p) = b.parent {
                if !self.bones.iter().any(|other| other.id == p) {
                    return Err(RigError::InvalidParent(p));
                }

                // Detecção de ciclos via subida na árvore
                let mut visited = HashSet::new();
                visited.insert(b.id);
                let mut curr = Some(p);
                while let Some(curr_id) = curr {
                    if !visited.insert(curr_id) {
                        return Err(RigError::HierarchyCycleDetected {
                            bone_id: b.id,
                            target_parent: p,
                        });
                    }
                    curr = self
                        .bones
                        .iter()
                        .find(|item| item.id == curr_id)
                        .and_then(|item| item.parent);
                }
            }
        }

        Ok(())
    }

    /// Recalcula a pose de repouso a partir de `head` (posição **absoluta** no
    /// espaço do esqueleto) e da rotação local de repouso de cada osso
    /// (`local_transform.rotation`, identidade por padrão):
    ///
    /// - matriz de repouso mundial `W = translate(head) * R_mundial`, com
    ///   `R_mundial = R_mundial(pai) * rotação_local`;
    /// - `inverse_bind_matrix = W⁻¹`;
    /// - `local_transform.translation` = offset de repouso **no referencial do
    ///   pai** (`R_mundial(pai)⁻¹ * (head − head_do_pai)`), a mesma convenção dos
    ///   nós de joint do glTF. Rotação e escala locais são preservadas
    ///   (a escala de repouso é assumida 1).
    ///
    /// Assim, uma pose igual ao `local_transform` de cada osso produz matrizes de
    /// skinning identidade (AN-16), inclusive com rotações de repouso (rigs
    /// externos). Chamado a cada mudança estrutural.
    pub fn compute_bind_pose_matrices(&mut self) {
        let n = self.bones.len();
        let index: HashMap<u32, usize> = self
            .bones
            .iter()
            .enumerate()
            .map(|(i, b)| (b.id, i))
            .collect();
        let mut world_rot: Vec<Option<Quat>> = vec![None; n];

        // Pais antes dos filhos; ossos com pai ausente/cíclico viram raízes.
        let mut remaining = n;
        let mut progressed = true;
        while remaining > 0 {
            if !progressed {
                // Ciclo defensivo: resolve o primeiro pendente como raiz.
                if let Some(i) = world_rot.iter().position(Option::is_none) {
                    self.bones[i].parent = None;
                }
            }
            progressed = false;
            for i in 0..n {
                if world_rot[i].is_some() {
                    continue;
                }
                let parent = self.bones[i]
                    .parent
                    .and_then(|pid| index.get(&pid).copied());
                let (parent_rot, parent_head) = match parent {
                    None => (Quat::IDENTITY, Vec3::ZERO),
                    Some(pi) => match world_rot[pi] {
                        Some(r) => (r, Vec3::from(self.bones[pi].head)),
                        None => continue,
                    },
                };
                let head = Vec3::from(self.bones[i].head);
                let raw = self.bones[i].local_transform.rotation;
                let local_rot = Quat::from_xyzw(raw[0], raw[1], raw[2], raw[3]);
                let local_rot = if local_rot.length_squared() > 1e-12 {
                    local_rot.normalize()
                } else {
                    Quat::IDENTITY
                };
                let rot = parent_rot * local_rot;
                let offset = parent_rot.inverse() * (head - parent_head);
                let b = &mut self.bones[i];
                b.local_transform.translation = [offset.x, offset.y, offset.z];
                let world = Mat4::from_rotation_translation(rot, head);
                b.inverse_bind_matrix = world.inverse().to_cols_array();
                world_rot[i] = Some(rot);
                remaining -= 1;
                progressed = true;
            }
        }
    }

    /// Calcula as matrizes de skinning finais para uma pose fornecida.
    /// `pose_transforms` deve possuir uma transformação para cada osso (na mesma ordem de `self.bones`).
    pub fn compute_skinning_matrices(
        &self,
        pose_transforms: &[Transform3D],
    ) -> Result<Vec<Mat4>, RigError> {
        if pose_transforms.len() != self.bones.len() {
            return Err(RigError::MatrixCountMismatch {
                expected: self.bones.len(),
                found: pose_transforms.len(),
            });
        }

        let mut world_pose_matrices = HashMap::new();
        let mut resolved = HashSet::new();
        let total = self.bones.len();

        for _ in 0..total {
            for (i, b) in self.bones.iter().enumerate() {
                if resolved.contains(&b.id) {
                    continue;
                }

                let parent_mat = match b.parent {
                    None => Mat4::IDENTITY,
                    Some(pid) => match world_pose_matrices.get(&pid) {
                        Some(&m) => m,
                        None => continue,
                    },
                };

                let local_mat = pose_transforms[i].to_mat4();
                let world_pose = parent_mat * local_mat;

                world_pose_matrices.insert(b.id, world_pose);
                resolved.insert(b.id);
            }
        }

        let mut skinning_matrices = Vec::with_capacity(self.bones.len());
        for b in &self.bones {
            let world_pose = world_pose_matrices
                .get(&b.id)
                .copied()
                .unwrap_or(Mat4::IDENTITY);
            let inv_bind = Mat4::from_cols_array(&b.inverse_bind_matrix);
            let skinning_mat = world_pose * inv_bind;
            skinning_matrices.push(skinning_mat);
        }

        Ok(skinning_matrices)
    }
}

/// Peso de influência de ossos para um único vértice (até 4 ossos por vértice, padrão de games).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct VertexSkinWeight {
    pub bones: [u32; 4],
    pub weights: [f32; 4],
}

impl Default for VertexSkinWeight {
    fn default() -> Self {
        Self {
            bones: [0; 4],
            weights: [1.0, 0.0, 0.0, 0.0],
        }
    }
}

impl VertexSkinWeight {
    pub fn new(bones: [u32; 4], weights: [f32; 4]) -> Self {
        let mut v = Self { bones, weights };
        v.normalize();
        v
    }

    /// Normaliza os pesos para garantir que a soma seja exatamente 1.0.
    pub fn normalize(&mut self) {
        let sum: f32 = self.weights.iter().sum();
        if sum > 1e-6 {
            let inv = 1.0 / sum;
            for w in &mut self.weights {
                *w *= inv;
            }
        } else {
            self.weights = [1.0, 0.0, 0.0, 0.0];
        }
    }
}

/// Dados de deformação de malha (Skinning) associados a um esqueleto.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SkinData {
    pub skeleton_id: Uuid,
    pub vertex_weights: Vec<VertexSkinWeight>,
}

impl SkinData {
    pub fn new(skeleton_id: Uuid, count: usize) -> Self {
        Self {
            skeleton_id,
            vertex_weights: vec![VertexSkinWeight::default(); count],
        }
    }

    /// Normaliza todos os pesos de vértices.
    pub fn normalize(&mut self) {
        for w in &mut self.vertex_weights {
            w.normalize();
        }
    }

    /// Valida a integridade dos pesos em relação à malha e ao esqueleto.
    pub fn validate(&self, mesh_vertex_count: usize, bone_count: usize) -> Result<(), RigError> {
        if self.vertex_weights.len() != mesh_vertex_count {
            return Err(RigError::VertexCountMismatch {
                expected: mesh_vertex_count,
                found: self.vertex_weights.len(),
            });
        }

        for (idx, vw) in self.vertex_weights.iter().enumerate() {
            let sum: f32 = vw.weights.iter().sum();
            if (sum - 1.0).abs() > 1e-3 {
                return Err(RigError::InvalidWeights {
                    vertex_index: idx,
                    reason: format!("Soma dos pesos é {sum}, esperava 1.0"),
                });
            }
            for (b_idx, &bone_id) in vw.bones.iter().enumerate() {
                if vw.weights[b_idx] > 0.0 && bone_id as usize >= bone_count {
                    return Err(RigError::InvalidWeights {
                        vertex_index: idx,
                        reason: format!(
                            "Índice de osso {bone_id} fora dos limites (máximo: {})",
                            bone_count - 1
                        ),
                    });
                }
            }
        }

        Ok(())
    }

    /// Deforma um vértice e normal utilizando Linear Blend Skinning (LBS).
    pub fn deform_vertex(
        &self,
        vertex_idx: usize,
        pos: [f32; 3],
        normal: [f32; 3],
        skinning_matrices: &[Mat4],
    ) -> ([f32; 3], [f32; 3]) {
        let vw = match self.vertex_weights.get(vertex_idx) {
            Some(w) => w,
            None => return (pos, normal),
        };

        let p = Vec3::from(pos);
        let n = Vec3::from(normal);

        let mut out_pos = Vec3::ZERO;
        let mut out_norm = Vec3::ZERO;

        for i in 0..4 {
            let weight = vw.weights[i];
            if weight <= 1e-6 {
                continue;
            }
            let bone_idx = vw.bones[i] as usize;
            let mat = match skinning_matrices.get(bone_idx) {
                Some(&m) => m,
                None => Mat4::IDENTITY,
            };

            out_pos += mat.transform_point3(p) * weight;
            out_norm += mat.transform_vector3(n) * weight;
        }

        let final_norm = out_norm.normalize_or_zero();
        (
            [out_pos.x, out_pos.y, out_pos.z],
            [final_norm.x, final_norm.y, final_norm.z],
        )
    }

    /// Aplica Linear Blend Skinning sobre todos os vértices de uma malha,
    /// retornando uma nova malha deformada para renderização.
    pub fn deform_mesh(
        &self,
        mesh: &petunia_mesh::Mesh,
        skinning_matrices: &[Mat4],
    ) -> petunia_mesh::Mesh {
        let mut deformed = mesh.clone();
        for (idx, v) in deformed.verts.iter_mut().enumerate() {
            let (new_pos, _) = self.deform_vertex(idx, v.pos, [0.0, 1.0, 0.0], skinning_matrices);
            v.pos = new_pos;
        }
        deformed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skeleton_creation_and_bone_hierarchy() {
        let mut skel = Skeleton::new("TestRig");
        let root = skel
            .add_bone("Root", None, [0.0, 0.0, 0.0], [0.0, 1.0, 0.0])
            .unwrap();
        let spine = skel
            .add_bone("Spine", Some(root), [0.0, 1.0, 0.0], [0.0, 2.0, 0.0])
            .unwrap();
        let head = skel
            .add_bone("Head", Some(spine), [0.0, 2.0, 0.0], [0.0, 2.5, 0.0])
            .unwrap();

        assert_eq!(skel.bones.len(), 3);
        assert_eq!(skel.children_of(root), vec![spine]);
        assert_eq!(skel.children_of(spine), vec![head]);
        assert_eq!(skel.find_bone("Head"), Some(head));

        assert!(skel.validate().is_ok());
    }

    #[test]
    fn test_duplicate_bone_name_rejected() {
        let mut skel = Skeleton::new("TestRig");
        skel.add_bone("Hips", None, [0.0, 0.0, 0.0], [0.0, 1.0, 0.0])
            .unwrap();
        let err = skel
            .add_bone("Hips", None, [0.0, 1.0, 0.0], [0.0, 2.0, 0.0])
            .unwrap_err();
        assert_eq!(err, RigError::BoneNameAlreadyExists("Hips".to_string()));
    }

    #[test]
    fn test_hierarchy_cycle_prevention() {
        let mut skel = Skeleton::new("TestRig");
        let a = skel
            .add_bone("A", None, [0.0, 0.0, 0.0], [0.0, 1.0, 0.0])
            .unwrap();
        let b = skel
            .add_bone("B", Some(a), [0.0, 1.0, 0.0], [0.0, 2.0, 0.0])
            .unwrap();
        let c = skel
            .add_bone("C", Some(b), [0.0, 2.0, 0.0], [0.0, 3.0, 0.0])
            .unwrap();

        // Tentar fazer A filho de C (criando ciclo C -> B -> A -> C) deve falhar
        let err = skel.reparent_bone(a, Some(c)).unwrap_err();
        assert_eq!(
            err,
            RigError::HierarchyCycleDetected {
                bone_id: a,
                target_parent: c
            }
        );

        // Tentar fazer A filho de si mesmo deve falhar
        let self_err = skel.reparent_bone(a, Some(a)).unwrap_err();
        assert_eq!(
            self_err,
            RigError::HierarchyCycleDetected {
                bone_id: a,
                target_parent: a
            }
        );
    }

    #[test]
    fn test_bone_removal_reparents_children() {
        let mut skel = Skeleton::new("TestRig");
        let root = skel
            .add_bone("Root", None, [0.0, 0.0, 0.0], [0.0, 1.0, 0.0])
            .unwrap();
        let middle = skel
            .add_bone("Middle", Some(root), [0.0, 1.0, 0.0], [0.0, 2.0, 0.0])
            .unwrap();
        let leaf = skel
            .add_bone("Leaf", Some(middle), [0.0, 2.0, 0.0], [0.0, 3.0, 0.0])
            .unwrap();

        assert_eq!(skel.get_bone(leaf).unwrap().parent, Some(middle));

        // Ao remover Middle, Leaf deve ser reatribuído a Root
        skel.remove_bone(middle).unwrap();
        assert_eq!(skel.bones.len(), 2);
        assert_eq!(skel.get_bone(leaf).unwrap().parent, Some(root));
        assert!(skel.validate().is_ok());
    }

    #[test]
    fn test_skin_weights_normalization_and_validation() {
        let mut skin = SkinData::new(Uuid::new_v4(), 2);
        skin.vertex_weights[0] = VertexSkinWeight {
            bones: [0, 1, 0, 0],
            weights: [0.5, 0.5, 0.0, 0.0],
        };
        skin.vertex_weights[1] = VertexSkinWeight {
            bones: [1, 0, 0, 0],
            weights: [2.0, 2.0, 0.0, 0.0], // Não-normalizado
        };

        skin.normalize();
        assert!((skin.vertex_weights[1].weights[0] - 0.5).abs() < 1e-5);
        assert!((skin.vertex_weights[1].weights[1] - 0.5).abs() < 1e-5);

        assert!(skin.validate(2, 2).is_ok());
        assert!(skin.validate(3, 2).is_err()); // Contagem incorreta de vértices
    }

    #[test]
    fn test_linear_blend_skinning_identity_pose() {
        let mut skel = Skeleton::new("TestRig");
        let root = skel
            .add_bone("Root", None, [0.0, 0.0, 0.0], [0.0, 1.0, 0.0])
            .unwrap();
        let pose = vec![Transform3D::default()];

        let skinning_matrices = skel.compute_skinning_matrices(&pose).unwrap();
        assert_eq!(skinning_matrices.len(), 1);

        let skin = SkinData {
            skeleton_id: skel.id,
            vertex_weights: vec![VertexSkinWeight::new([root, 0, 0, 0], [1.0, 0.0, 0.0, 0.0])],
        };

        let pt = [1.0, 2.0, 3.0];
        let norm = [0.0, 1.0, 0.0];
        let (deformed_pt, deformed_norm) = skin.deform_vertex(0, pt, norm, &skinning_matrices);

        assert!((deformed_pt[0] - pt[0]).abs() < 1e-4);
        assert!((deformed_pt[1] - pt[1]).abs() < 1e-4);
        assert!((deformed_pt[2] - pt[2]).abs() < 1e-4);
        assert!((deformed_norm[1] - 1.0).abs() < 1e-4);
    }

    /// AN-16: em repouso (pose = `local_transform` de cada osso) as matrizes de
    /// skinning precisam ser identidade, mesmo com hierarquia deslocada da origem.
    #[test]
    fn test_rest_pose_yields_identity_skinning_for_offset_hierarchy() {
        let mut skel = Skeleton::new("Chain");
        let hips = skel
            .add_bone("Hips", None, [0.0, 1.0, 0.0], [0.0, 1.2, 0.0])
            .unwrap();
        let spine = skel
            .add_bone("Spine", Some(hips), [0.0, 1.2, 0.0], [0.0, 1.4, 0.0])
            .unwrap();
        let _head = skel
            .add_bone("Head", Some(spine), [0.0, 1.4, 0.0], [0.0, 1.6, 0.0])
            .unwrap();

        let rest: Vec<Transform3D> = skel.bones.iter().map(|b| b.local_transform).collect();
        let mats = skel.compute_skinning_matrices(&rest).unwrap();
        for (bone, m) in skel.bones.iter().zip(&mats) {
            assert!(
                m.abs_diff_eq(Mat4::IDENTITY, 1e-5),
                "osso {} não é identidade em repouso: {m:?}",
                bone.name
            );
        }
    }

    #[test]
    fn test_rest_rotation_is_respected_and_rest_pose_stays_identity() {
        let mut skel = Skeleton::new("Rotated");
        let a = skel
            .add_bone("A", None, [0.0, 1.0, 0.0], [0.0, 2.0, 0.0])
            .unwrap();
        let b = skel
            .add_bone("B", Some(a), [0.0, 2.0, 0.0], [1.0, 2.0, 0.0])
            .unwrap();
        // Rotação de repouso de 90° em Z no osso A (como em rigs externos).
        let q = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
        skel.get_bone_mut(a).unwrap().local_transform.rotation = [q.x, q.y, q.z, q.w];
        skel.compute_bind_pose_matrices();

        // O offset de B é expresso no referencial (rotacionado) do pai.
        let tb = skel.get_bone(b).unwrap().local_transform.translation;
        assert!(
            (Vec3::from(tb) - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-5,
            "{tb:?}"
        );

        let rest: Vec<Transform3D> = skel.bones.iter().map(|b| b.local_transform).collect();
        for m in skel.compute_skinning_matrices(&rest).unwrap() {
            assert!(m.abs_diff_eq(Mat4::IDENTITY, 1e-5), "{m:?}");
        }
    }

    #[test]
    fn test_rest_local_translation_is_offset_from_parent_and_bind_is_absolute() {
        let mut skel = Skeleton::new("Chain");
        let a = skel
            .add_bone("A", None, [1.0, 2.0, 3.0], [1.0, 3.0, 3.0])
            .unwrap();
        let b = skel
            .add_bone("B", Some(a), [1.0, 3.0, 3.0], [1.0, 4.0, 3.0])
            .unwrap();
        assert_eq!(
            skel.get_bone(a).unwrap().local_transform.translation,
            [1.0, 2.0, 3.0]
        );
        assert_eq!(
            skel.get_bone(b).unwrap().local_transform.translation,
            [0.0, 1.0, 0.0]
        );
        // Bind mundial = translate(head) absoluto.
        let inv_b = Mat4::from_cols_array(&skel.get_bone(b).unwrap().inverse_bind_matrix);
        let world_b = inv_b.inverse();
        assert!(world_b.abs_diff_eq(Mat4::from_translation(Vec3::new(1.0, 3.0, 3.0)), 1e-5));
    }

    #[test]
    fn test_reparent_and_remove_keep_bind_and_recompute_rest_offsets() {
        let mut skel = Skeleton::new("Chain");
        let a = skel
            .add_bone("A", None, [0.0, 1.0, 0.0], [0.0, 2.0, 0.0])
            .unwrap();
        let b = skel
            .add_bone("B", Some(a), [0.0, 2.0, 0.0], [0.0, 3.0, 0.0])
            .unwrap();
        let c = skel
            .add_bone("C", Some(b), [0.0, 3.0, 0.0], [0.0, 4.0, 0.0])
            .unwrap();
        skel.remove_bone(b).unwrap();
        // C passa a filho de A; head absoluto permanece, offset local é recalculado.
        assert_eq!(skel.get_bone(c).unwrap().parent, Some(a));
        assert_eq!(
            skel.get_bone(c).unwrap().local_transform.translation,
            [0.0, 2.0, 0.0]
        );
        skel.reparent_bone(c, None).unwrap();
        assert_eq!(
            skel.get_bone(c).unwrap().local_transform.translation,
            [0.0, 3.0, 0.0]
        );
        let rest: Vec<Transform3D> = skel.bones.iter().map(|b| b.local_transform).collect();
        for m in skel.compute_skinning_matrices(&rest).unwrap() {
            assert!(m.abs_diff_eq(Mat4::IDENTITY, 1e-5));
        }
    }

    #[test]
    fn test_linear_blend_skinning_translation_pose() {
        let mut skel = Skeleton::new("TestRig");
        let root = skel
            .add_bone("Root", None, [0.0, 0.0, 0.0], [0.0, 1.0, 0.0])
            .unwrap();

        // Mover o osso +5.0 unidades no eixo X
        let pose = vec![Transform3D::from_translation([5.0, 0.0, 0.0])];

        let skinning_matrices = skel.compute_skinning_matrices(&pose).unwrap();
        let skin = SkinData {
            skeleton_id: skel.id,
            vertex_weights: vec![VertexSkinWeight::new([root, 0, 0, 0], [1.0, 0.0, 0.0, 0.0])],
        };

        let pt = [1.0, 2.0, 3.0];
        let norm = [0.0, 1.0, 0.0];
        let (deformed_pt, _) = skin.deform_vertex(0, pt, norm, &skinning_matrices);

        assert!((deformed_pt[0] - 6.0).abs() < 1e-4);
        assert!((deformed_pt[1] - 2.0).abs() < 1e-4);
        assert!((deformed_pt[2] - 3.0).abs() < 1e-4);
    }
}
