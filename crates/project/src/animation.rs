//! Petunia3D — Animation Clips, Keyframes, Tracks & Evaluation (P3D-067, P3D-136 a P3D-139)
//!
//! Subsistema desacoplado de animação: keyframing de transformações de ossos,
//! interpolações lineares e esféricas (Slerp), presets de esqueletos canônicos
//! (Humanoid, Quadruped, Multi-Leg), auto-rig heurístico, retargeting e biblioteca de assets.

use crate::rig::{RigError, Skeleton, SkinData, Transform3D, VertexSkinWeight};
use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Modo de interpolação entre keyframes sucessivos.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum Interpolation {
    /// Mantém o valor anterior até o próximo keyframe.
    Step,
    /// Interpolação linear contínua.
    #[default]
    Linear,
    /// Interpolação cúbica / esférica suave.
    Cubic,
}

/// Keyframe parametrizado no tempo (em segundos).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Keyframe<T> {
    pub time: f32,
    pub value: T,
    #[serde(default)]
    pub interpolation: Interpolation,
}

impl<T> Keyframe<T> {
    pub fn new(time: f32, value: T) -> Self {
        Self {
            time,
            value,
            interpolation: Interpolation::Linear,
        }
    }

    pub fn with_interp(time: f32, value: T, interpolation: Interpolation) -> Self {
        Self {
            time,
            value,
            interpolation,
        }
    }
}

/// Trilha de animação para um osso específico.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BoneTrack {
    pub bone_id: u32,
    pub bone_name: String,
    pub translations: Vec<Keyframe<[f32; 3]>>,
    pub rotations: Vec<Keyframe<[f32; 4]>>, // Quaternion [x, y, z, w]
    pub scales: Vec<Keyframe<[f32; 3]>>,
}

impl BoneTrack {
    pub fn new(bone_id: u32, bone_name: impl Into<String>) -> Self {
        Self {
            bone_id,
            bone_name: bone_name.into(),
            translations: Vec::new(),
            rotations: Vec::new(),
            scales: Vec::new(),
        }
    }

    pub fn add_translation(&mut self, time: f32, val: [f32; 3]) {
        if let Some(kf) = self
            .translations
            .iter_mut()
            .find(|k| (k.time - time).abs() < 1e-4)
        {
            kf.value = val;
        } else {
            self.translations.push(Keyframe::new(time, val));
            self.translations
                .sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
        }
    }

    pub fn add_rotation(&mut self, time: f32, quat: [f32; 4]) {
        if let Some(kf) = self
            .rotations
            .iter_mut()
            .find(|k| (k.time - time).abs() < 1e-4)
        {
            kf.value = quat;
        } else {
            self.rotations.push(Keyframe::new(time, quat));
            self.rotations
                .sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
        }
    }

    pub fn add_scale(&mut self, time: f32, val: [f32; 3]) {
        if let Some(kf) = self
            .scales
            .iter_mut()
            .find(|k| (k.time - time).abs() < 1e-4)
        {
            kf.value = val;
        } else {
            self.scales.push(Keyframe::new(time, val));
            self.scales
                .sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
        }
    }

    pub fn sample_translation(&self, time: f32) -> [f32; 3] {
        if self.translations.is_empty() {
            return [0.0, 0.0, 0.0];
        }
        if self.translations.len() == 1 || time <= self.translations[0].time {
            return self.translations[0].value;
        }
        let last = self.translations.last().unwrap();
        if time >= last.time {
            return last.value;
        }

        for i in 0..self.translations.len() - 1 {
            let k0 = &self.translations[i];
            let k1 = &self.translations[i + 1];
            if time >= k0.time && time <= k1.time {
                if k0.interpolation == Interpolation::Step {
                    return k0.value;
                }
                let dt = k1.time - k0.time;
                let factor = if dt > 1e-5 {
                    (time - k0.time) / dt
                } else {
                    0.0
                };
                let v0 = Vec3::from(k0.value);
                let v1 = Vec3::from(k1.value);
                let res = v0.lerp(v1, factor);
                return [res.x, res.y, res.z];
            }
        }
        self.translations[0].value
    }

    pub fn sample_rotation(&self, time: f32) -> [f32; 4] {
        if self.rotations.is_empty() {
            return [0.0, 0.0, 0.0, 1.0];
        }
        if self.rotations.len() == 1 || time <= self.rotations[0].time {
            return self.rotations[0].value;
        }
        let last = self.rotations.last().unwrap();
        if time >= last.time {
            return last.value;
        }

        for i in 0..self.rotations.len() - 1 {
            let k0 = &self.rotations[i];
            let k1 = &self.rotations[i + 1];
            if time >= k0.time && time <= k1.time {
                if k0.interpolation == Interpolation::Step {
                    return k0.value;
                }
                let dt = k1.time - k0.time;
                let factor = if dt > 1e-5 {
                    (time - k0.time) / dt
                } else {
                    0.0
                };
                let q0 = Quat::from_xyzw(k0.value[0], k0.value[1], k0.value[2], k0.value[3]);
                let q1 = Quat::from_xyzw(k1.value[0], k1.value[1], k1.value[2], k1.value[3]);
                let res = q0.slerp(q1, factor).normalize();
                return [res.x, res.y, res.z, res.w];
            }
        }
        self.rotations[0].value
    }

    pub fn sample_scale(&self, time: f32) -> [f32; 3] {
        if self.scales.is_empty() {
            return [1.0, 1.0, 1.0];
        }
        if self.scales.len() == 1 || time <= self.scales[0].time {
            return self.scales[0].value;
        }
        let last = self.scales.last().unwrap();
        if time >= last.time {
            return last.value;
        }

        for i in 0..self.scales.len() - 1 {
            let k0 = &self.scales[i];
            let k1 = &self.scales[i + 1];
            if time >= k0.time && time <= k1.time {
                if k0.interpolation == Interpolation::Step {
                    return k0.value;
                }
                let dt = k1.time - k0.time;
                let factor = if dt > 1e-5 {
                    (time - k0.time) / dt
                } else {
                    0.0
                };
                let v0 = Vec3::from(k0.value);
                let v1 = Vec3::from(k1.value);
                let res = v0.lerp(v1, factor);
                return [res.x, res.y, res.z];
            }
        }
        self.scales[0].value
    }

    /// Amostra a transformação com **identidade** nos canais sem keyframes.
    /// Para posar um osso use [`Self::sample_transform_over`], que preserva o repouso.
    pub fn sample_transform(&self, time: f32) -> Transform3D {
        Transform3D {
            translation: self.sample_translation(time),
            rotation: self.sample_rotation(time),
            scale: self.sample_scale(time),
        }
    }

    /// Amostra a transformação local: canais **sem keyframes** herdam o repouso
    /// (`rest`, normalmente `Bone::local_transform`). Uma trilha só de rotação não
    /// desloca o osso do pai (AN-16). Keyframes são valores locais absolutos, a
    /// mesma convenção dos canais de animação do glTF.
    pub fn sample_transform_over(&self, time: f32, rest: &Transform3D) -> Transform3D {
        Transform3D {
            translation: if self.translations.is_empty() {
                rest.translation
            } else {
                self.sample_translation(time)
            },
            rotation: if self.rotations.is_empty() {
                rest.rotation
            } else {
                self.sample_rotation(time)
            },
            scale: if self.scales.is_empty() {
                rest.scale
            } else {
                self.sample_scale(time)
            },
        }
    }
}

/// Clipe de animação contendo trilhas para múltiplos ossos.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AnimationClip {
    pub id: Uuid,
    pub name: String,
    pub duration: f32, // Em segundos
    #[serde(default = "default_fps")]
    pub fps: f32,
    #[serde(default = "default_looping")]
    pub looping: bool,
    pub tracks: Vec<BoneTrack>,
}

fn default_fps() -> f32 {
    24.0
}

fn default_looping() -> bool {
    true
}

impl AnimationClip {
    pub fn new(name: impl Into<String>, duration: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            duration: duration.max(0.1),
            fps: 24.0,
            looping: true,
            tracks: Vec::new(),
        }
    }

    pub fn get_track(&self, bone_id: u32) -> Option<&BoneTrack> {
        self.tracks.iter().find(|t| t.bone_id == bone_id)
    }

    pub fn get_track_mut(&mut self, bone_id: u32) -> Option<&mut BoneTrack> {
        self.tracks.iter_mut().find(|t| t.bone_id == bone_id)
    }

    pub fn get_or_create_track(&mut self, bone_id: u32, bone_name: &str) -> &mut BoneTrack {
        if let Some(pos) = self.tracks.iter().position(|t| t.bone_id == bone_id) {
            &mut self.tracks[pos]
        } else {
            self.tracks.push(BoneTrack::new(bone_id, bone_name));
            self.tracks.last_mut().unwrap()
        }
    }

    pub fn total_frames(&self) -> i32 {
        (self.duration * self.fps).round() as i32
    }

    pub fn frame_to_time(&self, frame: i32) -> f32 {
        (frame as f32 / self.fps).clamp(0.0, self.duration)
    }

    pub fn time_to_frame(&self, time: f32) -> i32 {
        (time * self.fps).round() as i32
    }

    /// Amostra a pose local para todos os ossos do esqueleto no instante de tempo `time`.
    pub fn sample_pose(&self, skeleton: &Skeleton, mut time: f32) -> Vec<Transform3D> {
        if self.looping && self.duration > 1e-4 {
            time = time.rem_euclid(self.duration);
        } else {
            time = time.clamp(0.0, self.duration);
        }

        skeleton
            .bones
            .iter()
            .map(|b| {
                if let Some(track) = self.get_track(b.id) {
                    track.sample_transform_over(time, &b.local_transform)
                } else {
                    b.local_transform
                }
            })
            .collect()
    }

    /// Amostra as matrizes de skinning no instante de tempo `time`.
    pub fn sample_skinning_matrices(
        &self,
        skeleton: &Skeleton,
        time: f32,
    ) -> Result<Vec<Mat4>, RigError> {
        let pose = self.sample_pose(skeleton, time);
        skeleton.compute_skinning_matrices(&pose)
    }
}

// ---------------------------------------------------------------------------
// P3D-136: Rig Presets (Humanoid, Quadruped e Multi-Leg)
// ---------------------------------------------------------------------------

pub struct RigPreset;

impl RigPreset {
    /// Gera um esqueleto canônico Humanoid bípede proporcional.
    pub fn humanoid(scale: f32) -> Skeleton {
        let mut skel = Skeleton::new("Humanoid_Rig");
        let s = scale.max(0.1);

        // Espinha e Cabeça
        let hips = skel
            .add_bone("Hips", None, [0.0, 1.0 * s, 0.0], [0.0, 1.2 * s, 0.0])
            .unwrap();
        let spine = skel
            .add_bone(
                "Spine",
                Some(hips),
                [0.0, 1.2 * s, 0.0],
                [0.0, 1.4 * s, 0.0],
            )
            .unwrap();
        let chest = skel
            .add_bone(
                "Chest",
                Some(spine),
                [0.0, 1.4 * s, 0.0],
                [0.0, 1.6 * s, 0.0],
            )
            .unwrap();
        let neck = skel
            .add_bone(
                "Neck",
                Some(chest),
                [0.0, 1.6 * s, 0.0],
                [0.0, 1.7 * s, 0.0],
            )
            .unwrap();
        let _head = skel
            .add_bone(
                "Head",
                Some(neck),
                [0.0, 1.7 * s, 0.0],
                [0.0, 1.95 * s, 0.0],
            )
            .unwrap();

        // Braço Esquerdo
        let sh_l = skel
            .add_bone(
                "Shoulder.L",
                Some(chest),
                [0.1 * s, 1.55 * s, 0.0],
                [0.25 * s, 1.55 * s, 0.0],
            )
            .unwrap();
        let arm_l = skel
            .add_bone(
                "UpperArm.L",
                Some(sh_l),
                [0.25 * s, 1.55 * s, 0.0],
                [0.55 * s, 1.55 * s, 0.0],
            )
            .unwrap();
        let farm_l = skel
            .add_bone(
                "LowerArm.L",
                Some(arm_l),
                [0.55 * s, 1.55 * s, 0.0],
                [0.85 * s, 1.55 * s, 0.0],
            )
            .unwrap();
        let _hand_l = skel
            .add_bone(
                "Hand.L",
                Some(farm_l),
                [0.85 * s, 1.55 * s, 0.0],
                [1.0 * s, 1.55 * s, 0.0],
            )
            .unwrap();

        // Braço Direito
        let sh_r = skel
            .add_bone(
                "Shoulder.R",
                Some(chest),
                [-0.1 * s, 1.55 * s, 0.0],
                [-0.25 * s, 1.55 * s, 0.0],
            )
            .unwrap();
        let arm_r = skel
            .add_bone(
                "UpperArm.R",
                Some(sh_r),
                [-0.25 * s, 1.55 * s, 0.0],
                [-0.55 * s, 1.55 * s, 0.0],
            )
            .unwrap();
        let farm_r = skel
            .add_bone(
                "LowerArm.R",
                Some(arm_r),
                [-0.55 * s, 1.55 * s, 0.0],
                [-0.85 * s, 1.55 * s, 0.0],
            )
            .unwrap();
        let _hand_r = skel
            .add_bone(
                "Hand.R",
                Some(farm_r),
                [-0.85 * s, 1.55 * s, 0.0],
                [-s, 1.55 * s, 0.0],
            )
            .unwrap();

        // Perna Esquerda
        let uleg_l = skel
            .add_bone(
                "UpperLeg.L",
                Some(hips),
                [0.15 * s, 1.0 * s, 0.0],
                [0.15 * s, 0.5 * s, 0.0],
            )
            .unwrap();
        let lleg_l = skel
            .add_bone(
                "LowerLeg.L",
                Some(uleg_l),
                [0.15 * s, 0.5 * s, 0.0],
                [0.15 * s, 0.1 * s, 0.0],
            )
            .unwrap();
        let _foot_l = skel
            .add_bone(
                "Foot.L",
                Some(lleg_l),
                [0.15 * s, 0.1 * s, 0.0],
                [0.15 * s, 0.0, 0.2 * s],
            )
            .unwrap();

        // Perna Direita
        let uleg_r = skel
            .add_bone(
                "UpperLeg.R",
                Some(hips),
                [-0.15 * s, 1.0 * s, 0.0],
                [-0.15 * s, 0.5 * s, 0.0],
            )
            .unwrap();
        let lleg_r = skel
            .add_bone(
                "LowerLeg.R",
                Some(uleg_r),
                [-0.15 * s, 0.5 * s, 0.0],
                [-0.15 * s, 0.1 * s, 0.0],
            )
            .unwrap();
        let _foot_r = skel
            .add_bone(
                "Foot.R",
                Some(lleg_r),
                [-0.15 * s, 0.1 * s, 0.0],
                [-0.15 * s, 0.0, 0.2 * s],
            )
            .unwrap();

        skel
    }

    /// Gera um esqueleto canônico Quadruped (4 patas).
    pub fn quadruped(scale: f32) -> Skeleton {
        let mut skel = Skeleton::new("Quadruped_Rig");
        let s = scale.max(0.1);

        let root = skel
            .add_bone("Root", None, [0.0, 0.8 * s, 0.0], [0.0, 0.8 * s, 0.5 * s])
            .unwrap();
        let spine = skel
            .add_bone(
                "Spine",
                Some(root),
                [0.0, 0.8 * s, 0.5 * s],
                [0.0, 0.85 * s, 1.0 * s],
            )
            .unwrap();
        let neck = skel
            .add_bone(
                "Neck",
                Some(spine),
                [0.0, 0.85 * s, 1.0 * s],
                [0.0, 1.1 * s, 1.3 * s],
            )
            .unwrap();
        let _head = skel
            .add_bone(
                "Head",
                Some(neck),
                [0.0, 1.1 * s, 1.3 * s],
                [0.0, 1.2 * s, 1.6 * s],
            )
            .unwrap();

        let _tail = skel
            .add_bone(
                "Tail",
                Some(root),
                [0.0, 0.8 * s, 0.0],
                [0.0, 0.6 * s, -0.6 * s],
            )
            .unwrap();

        // 4 Patas
        let leg_fl = skel
            .add_bone(
                "LegFront.L",
                Some(spine),
                [0.25 * s, 0.8 * s, 0.9 * s],
                [0.25 * s, 0.4 * s, 0.9 * s],
            )
            .unwrap();
        let _paw_fl = skel
            .add_bone(
                "PawFront.L",
                Some(leg_fl),
                [0.25 * s, 0.4 * s, 0.9 * s],
                [0.25 * s, 0.0, 0.9 * s],
            )
            .unwrap();

        let leg_fr = skel
            .add_bone(
                "LegFront.R",
                Some(spine),
                [-0.25 * s, 0.8 * s, 0.9 * s],
                [-0.25 * s, 0.4 * s, 0.9 * s],
            )
            .unwrap();
        let _paw_fr = skel
            .add_bone(
                "PawFront.R",
                Some(leg_fr),
                [-0.25 * s, 0.4 * s, 0.9 * s],
                [-0.25 * s, 0.0, 0.9 * s],
            )
            .unwrap();

        let leg_bl = skel
            .add_bone(
                "LegBack.L",
                Some(root),
                [0.25 * s, 0.8 * s, 0.1 * s],
                [0.25 * s, 0.4 * s, 0.1 * s],
            )
            .unwrap();
        let _paw_bl = skel
            .add_bone(
                "PawBack.L",
                Some(leg_bl),
                [0.25 * s, 0.4 * s, 0.1 * s],
                [0.25 * s, 0.0, 0.1 * s],
            )
            .unwrap();

        let leg_br = skel
            .add_bone(
                "LegBack.R",
                Some(root),
                [-0.25 * s, 0.8 * s, 0.1 * s],
                [-0.25 * s, 0.4 * s, 0.1 * s],
            )
            .unwrap();
        let _paw_br = skel
            .add_bone(
                "PawBack.R",
                Some(leg_br),
                [-0.25 * s, 0.4 * s, 0.1 * s],
                [-0.25 * s, 0.0, 0.1 * s],
            )
            .unwrap();

        skel
    }

    /// Gera um esqueleto canônico Multi-Leg (para aranhas, escorpiões, centopéias).
    pub fn multi_leg(legs_count: usize, scale: f32) -> Skeleton {
        let mut skel = Skeleton::new("MultiLeg_Rig");
        let s = scale.max(0.1);
        let count = legs_count.clamp(4, 16);

        let thorax = skel
            .add_bone("Thorax", None, [0.0, 0.4 * s, 0.0], [0.0, 0.4 * s, 0.8 * s])
            .unwrap();

        let half = count / 2;
        let step = (0.8 * s) / (half as f32);

        for i in 0..half {
            let z = (i as f32) * step + 0.1 * s;

            // Perna Esquerda
            let coxa_l = skel
                .add_bone(
                    format!("Leg_{i}_Coxa.L"),
                    Some(thorax),
                    [0.2 * s, 0.4 * s, z],
                    [0.6 * s, 0.5 * s, z],
                )
                .unwrap();
            let _tibia_l = skel
                .add_bone(
                    format!("Leg_{i}_Tibia.L"),
                    Some(coxa_l),
                    [0.6 * s, 0.5 * s, z],
                    [1.0 * s, 0.0, z],
                )
                .unwrap();

            // Perna Direita
            let coxa_r = skel
                .add_bone(
                    format!("Leg_{i}_Coxa.R"),
                    Some(thorax),
                    [-0.2 * s, 0.4 * s, z],
                    [-0.6 * s, 0.5 * s, z],
                )
                .unwrap();
            let _tibia_r = skel
                .add_bone(
                    format!("Leg_{i}_Tibia.R"),
                    Some(coxa_r),
                    [-0.6 * s, 0.5 * s, z],
                    [-s, 0.0, z],
                )
                .unwrap();
        }

        skel
    }
}

// ---------------------------------------------------------------------------
// P3D-137: Auto-Rigging & Auto-Skinning
// ---------------------------------------------------------------------------

/// Ajusta proporcionalmente um esqueleto humanoide às dimensões (Bounding Box) de uma malha.
pub fn auto_fit_humanoid(mesh: &petunia_mesh::Mesh) -> Skeleton {
    if mesh.verts.is_empty() {
        return RigPreset::humanoid(1.0);
    }

    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);

    for v in &mesh.verts {
        let p = Vec3::from(v.pos);
        min = min.min(p);
        max = max.max(p);
    }

    let height = (max.y - min.y).max(0.2);
    let scale = height / 1.95; // 1.95 é a altura padrão do preset humanoide
    let center_x = (min.x + max.x) * 0.5;
    let base_y = min.y;
    let center_z = (min.z + max.z) * 0.5;

    let mut skel = RigPreset::humanoid(scale);

    // Ajustar offsets para coincidir com a base e centro da malha
    for b in &mut skel.bones {
        b.head[0] += center_x;
        b.head[1] += base_y;
        b.head[2] += center_z;

        b.tail[0] += center_x;
        b.tail[1] += base_y;
        b.tail[2] += center_z;
    }

    skel.compute_bind_pose_matrices();
    skel
}

/// Calcula a distância euclidiana mínima de um ponto $P$ a um segmento de linha $AB$.
fn distance_to_segment(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_squared();
    if len_sq < 1e-6 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    let projection = a + ab * t;
    p.distance(projection)
}

/// Computa automaticamente os pesos de skinning para uma malha a partir da proximidade dos ossos do esqueleto.
pub fn compute_auto_skin_weights(mesh: &petunia_mesh::Mesh, skeleton: &Skeleton) -> SkinData {
    let mut skin = SkinData::new(skeleton.id, mesh.verts.len());
    if skeleton.bones.is_empty() || mesh.verts.is_empty() {
        return skin;
    }

    for (v_idx, v) in mesh.verts.iter().enumerate() {
        let p = Vec3::from(v.pos);

        // Avaliar distância para todos os ossos
        let mut bone_distances: Vec<(u32, f32)> = skeleton
            .bones
            .iter()
            .map(|b| {
                let d = distance_to_segment(p, Vec3::from(b.head), Vec3::from(b.tail));
                (b.id, d)
            })
            .collect();

        // Ordenar por menor distância
        bone_distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        // Selecionar os 4 ossos mais próximos
        let mut bones = [0u32; 4];
        let mut weights = [0.0f32; 4];

        for i in 0..4.min(bone_distances.len()) {
            let (b_id, dist) = bone_distances[i];
            bones[i] = b_id;
            // Queda quadrática suave inversamente proporcional à distância
            weights[i] = 1.0 / (dist.powi(2) + 0.05);
        }

        let mut vw = VertexSkinWeight { bones, weights };
        vw.normalize();
        skin.vertex_weights[v_idx] = vw;
    }

    skin
}

// ---------------------------------------------------------------------------
// P3D-138: Animation Retargeting
// ---------------------------------------------------------------------------

/// Perfil de mapeamento semântico de ossos entre convenções de rigs externos e internos.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RetargetProfile {
    pub name: String,
    pub bone_map: HashMap<String, String>, // Mapeia `source_name` -> `target_name`
}

impl RetargetProfile {
    /// Perfil canônico para rigs estilo Mixamo.
    pub fn mixamo_standard() -> Self {
        let mut map = HashMap::new();
        map.insert("mixamorig:Hips".to_string(), "Hips".to_string());
        map.insert("mixamorig:Spine".to_string(), "Spine".to_string());
        map.insert("mixamorig:Spine1".to_string(), "Chest".to_string());
        map.insert("mixamorig:Neck".to_string(), "Neck".to_string());
        map.insert("mixamorig:Head".to_string(), "Head".to_string());
        map.insert(
            "mixamorig:LeftShoulder".to_string(),
            "Shoulder.L".to_string(),
        );
        map.insert("mixamorig:LeftArm".to_string(), "UpperArm.L".to_string());
        map.insert(
            "mixamorig:LeftForeArm".to_string(),
            "LowerArm.L".to_string(),
        );
        map.insert("mixamorig:LeftHand".to_string(), "Hand.L".to_string());
        map.insert(
            "mixamorig:RightShoulder".to_string(),
            "Shoulder.R".to_string(),
        );
        map.insert("mixamorig:RightArm".to_string(), "UpperArm.R".to_string());
        map.insert(
            "mixamorig:RightForeArm".to_string(),
            "LowerArm.R".to_string(),
        );
        map.insert("mixamorig:RightHand".to_string(), "Hand.R".to_string());
        map.insert("mixamorig:LeftUpLeg".to_string(), "UpperLeg.L".to_string());
        map.insert("mixamorig:LeftLeg".to_string(), "LowerLeg.L".to_string());
        map.insert("mixamorig:LeftFoot".to_string(), "Foot.L".to_string());
        map.insert("mixamorig:RightUpLeg".to_string(), "UpperLeg.R".to_string());
        map.insert("mixamorig:RightLeg".to_string(), "LowerLeg.R".to_string());
        map.insert("mixamorig:RightFoot".to_string(), "Foot.R".to_string());

        Self {
            name: "Mixamo to Petunia Humanoid".to_string(),
            bone_map: map,
        }
    }

    /// Retargeteia um AnimationClip para o esqueleto de destino.
    pub fn retarget_clip(
        &self,
        source_clip: &AnimationClip,
        target_skeleton: &Skeleton,
    ) -> AnimationClip {
        let mut target_clip = AnimationClip::new(
            format!("{}_retargeted", source_clip.name),
            source_clip.duration,
        );
        target_clip.fps = source_clip.fps;
        target_clip.looping = source_clip.looping;

        for src_track in &source_clip.tracks {
            // Verificar mapeamento direto ou com prefixo mapeado
            let target_name = self
                .bone_map
                .get(&src_track.bone_name)
                .cloned()
                .unwrap_or_else(|| src_track.bone_name.clone());

            if let Some(target_bone_id) = target_skeleton.find_bone(&target_name) {
                let mut new_track = BoneTrack::new(target_bone_id, target_name);
                new_track.translations = src_track.translations.clone();
                new_track.rotations = src_track.rotations.clone();
                new_track.scales = src_track.scales.clone();
                target_clip.tracks.push(new_track);
            }
        }

        target_clip
    }
}

// ---------------------------------------------------------------------------
// P3D-139: Animation Asset Library
// ---------------------------------------------------------------------------

/// Ativo de animação persistente para biblioteca reutilizável do projeto.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AnimationAsset {
    pub id: Uuid,
    pub name: String,
    pub clip: AnimationClip,
    pub tags: Vec<String>,
    pub preset: Option<String>,
}

impl AnimationAsset {
    pub fn new(name: impl Into<String>, clip: AnimationClip) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            clip,
            tags: Vec::new(),
            preset: None,
        }
    }

    pub fn with_preset(mut self, preset: impl Into<String>) -> Self {
        self.preset = Some(preset.into());
        self
    }
}

/// Biblioteca nativa com presets úteis prontos para uso.
pub struct AnimationLibrary;

impl AnimationLibrary {
    /// Gera clipe canônico de Idle respiratório para humanoides.
    pub fn humanoid_idle(skeleton: &Skeleton) -> AnimationClip {
        let mut clip = AnimationClip::new("Humanoid_Idle", 2.0);
        let chest_id = skeleton.find_bone("Chest").unwrap_or(0);

        // Keyframes são locais absolutos: o respirar é um delta sobre o repouso.
        let rest = skeleton
            .get_bone(chest_id)
            .map_or([0.0; 3], |b| b.local_transform.translation);
        let up = [rest[0], rest[1] + 0.03, rest[2]];
        let track = clip.get_or_create_track(chest_id, "Chest");
        track.add_translation(0.0, rest);
        track.add_translation(1.0, up);
        track.add_translation(2.0, rest);

        track.add_scale(0.0, [1.0, 1.0, 1.0]);
        track.add_scale(1.0, [1.04, 1.02, 1.04]);
        track.add_scale(2.0, [1.0, 1.0, 1.0]);

        clip
    }

    /// Gera clipe canônico de caminhada (Walk Cycle) para humanoides.
    pub fn humanoid_walk(skeleton: &Skeleton) -> AnimationClip {
        let mut clip = AnimationClip::new("Humanoid_Walk", 1.0);
        let uleg_l = skeleton.find_bone("UpperLeg.L").unwrap_or(0);
        let uleg_r = skeleton.find_bone("UpperLeg.R").unwrap_or(0);

        let q_forward = Quat::from_rotation_x(25.0f32.to_radians());
        let q_back = Quat::from_rotation_x(-25.0f32.to_radians());

        let track_l = clip.get_or_create_track(uleg_l, "UpperLeg.L");
        track_l.add_rotation(0.0, [q_forward.x, q_forward.y, q_forward.z, q_forward.w]);
        track_l.add_rotation(0.5, [q_back.x, q_back.y, q_back.z, q_back.w]);
        track_l.add_rotation(1.0, [q_forward.x, q_forward.y, q_forward.z, q_forward.w]);

        let track_r = clip.get_or_create_track(uleg_r, "UpperLeg.R");
        track_r.add_rotation(0.0, [q_back.x, q_back.y, q_back.z, q_back.w]);
        track_r.add_rotation(0.5, [q_forward.x, q_forward.y, q_forward.z, q_forward.w]);
        track_r.add_rotation(1.0, [q_back.x, q_back.y, q_back.z, q_back.w]);

        clip
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AN-16: trilha só de rotação não pode soltar o osso do pai.
    #[test]
    fn rotation_only_track_keeps_rest_offset_and_rest_frame_is_identity() {
        let skel = RigPreset::humanoid(1.0);
        let walk = AnimationLibrary::humanoid_walk(&skel);
        let pose = walk.sample_pose(&skel, 0.0);
        let uleg = skel.find_bone("UpperLeg.L").unwrap();
        let idx = skel.bone_index(uleg).unwrap();
        // Translação de repouso preservada; só a rotação é animada.
        assert_eq!(
            pose[idx].translation,
            skel.bones[idx].local_transform.translation
        );
        assert_ne!(pose[idx].rotation, skel.bones[idx].local_transform.rotation);

        // Ossos sem trilha ficam em repouso => skinning identidade.
        let empty = AnimationClip::new("Empty", 1.0);
        let mats = empty.sample_skinning_matrices(&skel, 0.5).unwrap();
        assert!(mats.iter().all(|m| m.abs_diff_eq(Mat4::IDENTITY, 1e-5)));

        // Idle respira em torno do repouso: frame 0 e 2 s == repouso do Chest.
        let idle = AnimationLibrary::humanoid_idle(&skel);
        let mats0 = idle.sample_skinning_matrices(&skel, 0.0).unwrap();
        assert!(mats0.iter().all(|m| m.abs_diff_eq(Mat4::IDENTITY, 1e-4)));
    }

    #[test]
    fn test_animation_track_sampling_and_interpolation() {
        let mut track = BoneTrack::new(0, "TestBone");
        track.add_translation(0.0, [0.0, 0.0, 0.0]);
        track.add_translation(2.0, [10.0, 0.0, 0.0]);

        let mid = track.sample_translation(1.0);
        assert!((mid[0] - 5.0).abs() < 1e-4);

        let before = track.sample_translation(-1.0);
        assert_eq!(before, [0.0, 0.0, 0.0]);

        let after = track.sample_translation(3.0);
        assert_eq!(after, [10.0, 0.0, 0.0]);
    }

    #[test]
    fn test_clip_sample_pose_and_matrices() {
        let skel = RigPreset::humanoid(1.0);
        let idle = AnimationLibrary::humanoid_idle(&skel);

        let pose_0 = idle.sample_pose(&skel, 0.0);
        assert_eq!(pose_0.len(), skel.bones.len());

        let matrices = idle.sample_skinning_matrices(&skel, 0.5).unwrap();
        assert_eq!(matrices.len(), skel.bones.len());
    }

    #[test]
    fn test_rig_presets_validity() {
        let humanoid = RigPreset::humanoid(1.0);
        assert!(humanoid.validate().is_ok());
        assert!(humanoid.find_bone("Hips").is_some());
        assert!(humanoid.find_bone("Head").is_some());

        let quadruped = RigPreset::quadruped(1.0);
        assert!(quadruped.validate().is_ok());
        assert!(quadruped.find_bone("Root").is_some());
        assert!(quadruped.find_bone("Tail").is_some());

        let spider = RigPreset::multi_leg(8, 1.0);
        assert!(spider.validate().is_ok());
        assert_eq!(spider.bones.len(), 17); // 1 tórax + 8 pares de coxa e tíbia (1 + 16 = 17)
    }

    #[test]
    fn test_auto_fit_humanoid_and_skin_weights() {
        let cube = petunia_mesh::Mesh::cube(2.0);
        let fitted_skel = auto_fit_humanoid(&cube);
        assert!(fitted_skel.validate().is_ok());

        let skin = compute_auto_skin_weights(&cube, &fitted_skel);
        assert_eq!(skin.vertex_weights.len(), cube.verts.len());
        assert!(
            skin.validate(cube.verts.len(), fitted_skel.bones.len())
                .is_ok()
        );

        // Deformação da malha não deve produzir NaNs
        let matrices = vec![Mat4::IDENTITY; fitted_skel.bones.len()];
        let deformed = skin.deform_mesh(&cube, &matrices);
        assert_eq!(deformed.verts.len(), cube.verts.len());
        for v in &deformed.verts {
            assert!(v.pos[0].is_finite());
            assert!(v.pos[1].is_finite());
            assert!(v.pos[2].is_finite());
        }
    }

    #[test]
    fn test_animation_retargeting() {
        let target_skel = RigPreset::humanoid(1.0);
        let profile = RetargetProfile::mixamo_standard();

        let mut mixamo_clip = AnimationClip::new("mixamo_walk", 1.0);
        let track = mixamo_clip.get_or_create_track(99, "mixamorig:Hips");
        track.add_translation(0.0, [0.0, 1.0, 0.0]);
        track.add_translation(1.0, [0.0, 1.1, 0.0]);

        let retargeted = profile.retarget_clip(&mixamo_clip, &target_skel);
        assert_eq!(retargeted.tracks.len(), 1);
        assert_eq!(retargeted.tracks[0].bone_name, "Hips");

        let target_hips_id = target_skel.find_bone("Hips").unwrap();
        assert_eq!(retargeted.tracks[0].bone_id, target_hips_id);
    }
}
