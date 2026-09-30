//! Geradores de movimento (P3D-170): matemática de `pose(t)`.
//!
//! Convenções: a **frente** e a **lateral** do corpo são derivadas do próprio rig
//! ([`Frame`]); "up" é +Y. Ângulos positivos seguem a regra da mão direita em
//! torno do eixo indicado. Os geradores partem da pose de repouso e só alteram
//! rotações locais — exceto a translação do osso raiz (bounce, sway, root
//! motion). Comprimentos de osso nunca mudam (as pernas usam o IK do P3D-169).

use crate::ik::{IkChain, IkSolver, chain_end_position, solve_chain};
use crate::motion::{MotionError, MotionGenerator, MotionRecipe};
use crate::rig::{Skeleton, Transform3D};
use crate::rig_roles::{ArmPart, LegChain, RigRole, RigRoleMap};
use glam::{Mat4, Quat, Vec3};
use std::collections::HashMap;
use std::f32::consts::{PI, TAU};

/// Eixos do corpo no espaço do esqueleto.
#[derive(Clone, Copy, Debug)]
struct Frame {
    fwd: Vec3,
    up: Vec3,
    lat: Vec3,
}

fn horiz(v: Vec3) -> Vec3 {
    Vec3::new(v.x, 0.0, v.z)
}

fn head_of(sk: &Skeleton, roles: &RigRoleMap, role: RigRole) -> Option<Vec3> {
    roles
        .bone_of(role)
        .and_then(|id| sk.get_bone(id))
        .map(|b| Vec3::from(b.head))
}

/// Frente do corpo: base→cabeça (horizontal), senão a direção dos pés, senão o
/// eixo do osso raiz, senão +Z. Lateral = `up × frente` (esquerda).
fn detect_frame(sk: &Skeleton, roles: &RigRoleMap) -> Frame {
    let up = Vec3::Y;
    let base = head_of(sk, roles, RigRole::Hips)
        .or_else(|| head_of(sk, roles, RigRole::Root))
        .or_else(|| head_of(sk, roles, RigRole::Spine(0)));
    let mut fwd = None;
    if let (Some(b), Some(h)) = (base, head_of(sk, roles, RigRole::Head)) {
        let d = h - b;
        let hv = horiz(d);
        if hv.length() > 1e-4 && hv.length() > 0.25 * d.y.abs() {
            fwd = hv.try_normalize();
        }
    }
    if fwd.is_none() {
        let dirs: Vec3 = roles
            .legs()
            .iter()
            .filter_map(|l| l.foot)
            .filter_map(|id| sk.get_bone(id))
            .map(|b| horiz(Vec3::from(b.tail) - Vec3::from(b.head)))
            .sum();
        if dirs.length() > 1e-4 {
            fwd = dirs.try_normalize();
        }
    }
    if fwd.is_none()
        && let Some(root) = sk.bones.iter().find(|b| b.parent.is_none())
    {
        fwd = horiz(Vec3::from(root.tail) - Vec3::from(root.head)).try_normalize();
    }
    let fwd = fwd.unwrap_or(Vec3::Z);
    Frame {
        fwd,
        up,
        lat: up.cross(fwd).normalize_or_zero(),
    }
}

/// Dados de repouso e utilitários de posar ossos em eixos do mundo.
pub(crate) struct Basis {
    sk: Skeleton,
    frame: Frame,
    rest_pose: Vec<Transform3D>,
    rest_rot: Vec<Quat>,
    rest_head: Vec<Vec3>,
    index: HashMap<u32, usize>,
    root: u32,
}

impl Basis {
    fn new(sk: &Skeleton, roles: &RigRoleMap) -> Self {
        let rest_rot = sk
            .bones
            .iter()
            .map(|b| {
                Mat4::from_cols_array(&b.inverse_bind_matrix)
                    .inverse()
                    .to_scale_rotation_translation()
                    .1
                    .normalize()
            })
            .collect();
        Self {
            frame: detect_frame(sk, roles),
            rest_pose: sk.bones.iter().map(|b| b.local_transform).collect(),
            rest_rot,
            rest_head: sk.bones.iter().map(|b| Vec3::from(b.head)).collect(),
            index: sk
                .bones
                .iter()
                .enumerate()
                .map(|(i, b)| (b.id, i))
                .collect(),
            root: sk
                .bones
                .iter()
                .find(|b| b.parent.is_none())
                .map_or(0, |b| b.id),
            sk: sk.clone(),
        }
    }

    fn idx(&self, bone: u32) -> usize {
        self.index[&bone]
    }

    fn parent_rest_rot(&self, i: usize) -> Quat {
        self.sk.bones[i]
            .parent
            .and_then(|p| self.index.get(&p).copied())
            .map_or(Quat::IDENTITY, |pi| self.rest_rot[pi])
    }

    /// Gira o osso por `angle` em torno de `world_axis` (eixo do mundo na pose
    /// de repouso), somando à pose atual. O eixo é expresso no referencial de
    /// repouso do pai e aplicado por pré-multiplicação: rotações sucessivas no
    /// mesmo osso mantêm o eixo fixo e o movimento do pai é herdado.
    fn swing(&self, pose: &mut [Transform3D], bone: u32, world_axis: Vec3, angle: f32) {
        if angle == 0.0 {
            return;
        }
        let i = self.idx(bone);
        let axis = (self.parent_rest_rot(i).inverse() * world_axis).normalize_or_zero();
        if axis == Vec3::ZERO {
            return;
        }
        let q = Quat::from_axis_angle(axis, angle) * Quat::from_array(pose[i].rotation).normalize();
        pose[i].rotation = q.normalize().to_array();
    }

    /// Soma um deslocamento em coordenadas do mundo à translação local do osso.
    fn offset(&self, pose: &mut [Transform3D], bone: u32, world: Vec3) {
        let i = self.idx(bone);
        let d = self.parent_rest_rot(i).inverse() * world;
        let t = &mut pose[i].translation;
        t[0] += d.x;
        t[1] += d.y;
        t[2] += d.z;
    }

    /// Fixa a rotação mundial do osso (a partir da pose atual do pai).
    fn set_world_rotation(&self, pose: &mut [Transform3D], world: &[Mat4], bone: u32, want: Quat) {
        let i = self.idx(bone);
        let parent_rot = self.sk.bones[i]
            .parent
            .and_then(|p| self.index.get(&p).copied())
            .map_or(Quat::IDENTITY, |pi| {
                world[pi].to_scale_rotation_translation().1.normalize()
            });
        pose[i].rotation = (parent_rot.inverse() * want).normalize().to_array();
    }

    fn size(&self) -> f32 {
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for h in &self.rest_head {
            lo = lo.min(*h);
            hi = hi.max(*h);
        }
        (hi - lo).max_element().max(0.1)
    }
}

fn phase_of(t: f32, cycle: f32) -> f32 {
    let x = t / cycle;
    x - x.floor()
}

/// Ruído determinístico em [-1, 1] por (seed, índice).
fn jitter(seed: u32, i: u32) -> f32 {
    let mut x = seed.wrapping_mul(0x9E37_79B1) ^ i.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x as f32 / u32::MAX as f32) * 2.0 - 1.0
}

fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Easing entre linear (smoothness 0) e smoothstep (1).
fn ease(u: f32, smooth: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u + (smoothstep(u) - u) * smooth.clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------
// Plano preparado
// ---------------------------------------------------------------------------

pub(crate) struct Prepared {
    basis: Basis,
    plan: Plan,
}

enum Plan {
    Loco(Box<Locomotion>),
    Serpentine(Serpentine),
    Idle(Idle),
}

impl Prepared {
    pub(crate) fn new(
        recipe: &MotionRecipe,
        sk: &Skeleton,
        roles: &RigRoleMap,
    ) -> Result<Self, MotionError> {
        let basis = Basis::new(sk, roles);
        let plan = match recipe.generator {
            MotionGenerator::BipedCycle | MotionGenerator::Gait => {
                Plan::Loco(Box::new(Locomotion::new(recipe, &basis, roles)?))
            }
            MotionGenerator::Serpentine => Plan::Serpentine(Serpentine::new(recipe, &basis, roles)),
            MotionGenerator::IdleBreath => Plan::Idle(Idle::new(recipe, &basis, roles)),
        };
        Ok(Self { basis, plan })
    }

    pub(crate) fn cycle_seconds(&self) -> f32 {
        match &self.plan {
            Plan::Loco(l) => l.cycle,
            Plan::Serpentine(s) => s.cycle,
            Plan::Idle(i) => i.cycle,
        }
    }

    pub(crate) fn pose_at(&self, t: f32, root_motion: bool) -> Vec<Transform3D> {
        let mut pose = self.basis.rest_pose.clone();
        match &self.plan {
            Plan::Loco(l) => l.apply(&self.basis, &mut pose, t, root_motion),
            Plan::Serpentine(s) => s.apply(&self.basis, &mut pose, t),
            Plan::Idle(i) => i.apply(&self.basis, &mut pose, t),
        }
        pose
    }
}

// ---------------------------------------------------------------------------
// Locomoção: ciclo bípede e gait de N pernas
// ---------------------------------------------------------------------------

struct Leg {
    chain: IkChain,
    foot: Option<u32>,
    rest_end: Vec3,
    rest_knee: Vec3,
    len: f32,
    pole_dir: Option<Vec3>,
    phase: f32,
}

struct Arm {
    upper: u32,
    lower: Option<u32>,
    phase: f32,
}

struct Locomotion {
    biped: bool,
    legs: Vec<Leg>,
    arms: Vec<Arm>,
    spine: Vec<u32>,
    chest: Option<u32>,
    tail: Vec<u32>,
    cycle: f32,
    duty: f32,
    stride_len: f32,
    step_h: f32,
    crouch: f32,
    bounce: f32,
    sway: f32,
    speed_v: f32,
    energy: f32,
    smooth: f32,
    lean: f32,
    run: f32,
    arm_swing: f32,
    spine_wave: f32,
    tail_sway: f32,
}

impl Locomotion {
    fn new(recipe: &MotionRecipe, b: &Basis, roles: &RigRoleMap) -> Result<Self, MotionError> {
        let biped = recipe.generator == MotionGenerator::BipedCycle;
        let frame = b.frame;
        let rest = &b.rest_pose;
        let energy = recipe.param("energy");
        let smooth = recipe.param("smoothness");
        let weight = recipe.param("weight");
        let stride = recipe.param("stride");
        let speed = recipe.param("speed");
        let variation = recipe.param("variation");
        let run = if biped {
            recipe.param("run_blend")
        } else {
            0.0
        };

        let leg_defs: Vec<LegChain> = roles.legs();
        let n = leg_defs.len();
        // Pernas: geometria de repouso e pole.
        struct Raw {
            chain: IkChain,
            foot: Option<u32>,
            limb: u8,
            hip: Vec3,
            knee: Vec3,
            rest_end: Vec3,
            len: f32,
            pole_dir: Option<Vec3>,
            bent: bool,
        }
        let mut legs: Vec<Raw> = Vec::with_capacity(n);
        for l in &leg_defs {
            let mut chain = IkChain::two_bone_leg(b.sk.id, format!("leg_{}", l.limb), l);
            chain.solver = IkSolver::TwoBone { soft: 0.0 };
            let rest_end = chain_end_position(&b.sk, rest, &chain)?;
            let hip = b.rest_head[b.idx(l.thigh)];
            let knee = b.rest_head[b.idx(l.shin)];
            let len = (knee - hip).length() + (rest_end - knee).length();
            let line = (rest_end - hip).try_normalize().unwrap_or(-Vec3::Y);
            let off = (knee - hip) - line * (knee - hip).dot(line);
            let bent = off.length() > 0.05 * len;
            legs.push(Raw {
                chain,
                foot: l.foot,
                limb: l.limb,
                hip,
                knee,
                rest_end,
                len,
                pole_dir: if bent { off.try_normalize() } else { None },
                bent,
            });
        }

        // Frente/lado de cada perna para padrões de passo e pole padrão.
        let center = legs.iter().map(|l| l.hip).sum::<Vec3>() / n as f32;
        let fa: Vec<f32> = legs
            .iter()
            .map(|l| (l.rest_end - center).dot(frame.fwd))
            .collect();
        let left: Vec<bool> = legs
            .iter()
            .map(|l| (l.hip - center).dot(frame.lat) >= 0.0)
            .collect();
        let mean_fa = fa.iter().sum::<f32>() / n as f32;

        // Pares por lado, da frente para trás (rank 0 = mais à frente).
        let mut rank = vec![0usize; n];
        let mut pairs = 1usize;
        for side in [true, false] {
            let mut ids: Vec<usize> = (0..n).filter(|&i| left[i] == side).collect();
            ids.sort_by(|&a, &c| fa[c].total_cmp(&fa[a]));
            pairs = pairs.max(ids.len());
            for (r, &i) in ids.iter().enumerate() {
                rank[i] = r;
            }
        }

        let duty = if biped {
            (0.62 - 0.24 * run).clamp(0.3, 0.9)
        } else {
            recipe.param("duty")
        };
        let pattern = if biped {
            0
        } else {
            recipe.param("pattern") as u8
        };
        let pattern = match pattern {
            0 if n <= 2 => 1,
            0 if n <= 4 => {
                if duty >= 0.6 {
                    2
                } else {
                    1
                }
            }
            0 => 1,
            p => p,
        };

        let mean_len = legs.iter().map(|l| l.len).sum::<f32>() / n as f32;
        let mut out = Vec::with_capacity(n);
        let any_bent = legs.iter().any(|l| l.bent);
        let hips: Vec<Vec3> = legs.iter().map(|l| l.hip).collect();
        for (k, l) in legs.into_iter().enumerate() {
            let Raw {
                chain,
                foot,
                limb,
                knee,
                rest_end,
                len,
                pole_dir,
                ..
            } = l;
            let side_bit = usize::from(!left[k]);
            let base = match pattern {
                2 => (((pairs - 1 - rank[k]) as f32) / pairs as f32) * 0.5,
                3 => ((pairs - 1 - rank[k]) as f32) / pairs as f32,
                _ => {
                    if (rank[k] + side_bit).is_multiple_of(2) {
                        0.0
                    } else {
                        0.5
                    }
                }
            };
            let phase = (base
                + if pattern == 1 {
                    0.0
                } else {
                    side_bit as f32 * 0.5
                }
                + variation * 0.03 * jitter(recipe.seed, limb as u32))
            .rem_euclid(1.0);
            // Pole padrão quando o repouso não tem dobra: bípede/dianteiras para a
            // frente, traseiras para trás, muitas pernas para cima e para fora.
            let pole_dir = pole_dir.or_else(|| {
                Some(if n <= 2 || (n <= 4 && fa[k] >= mean_fa) {
                    frame.fwd
                } else if n <= 4 {
                    -frame.fwd
                } else {
                    let out_dir = if left[k] { frame.lat } else { -frame.lat };
                    (frame.up * 0.7 + out_dir * 0.7).normalize_or_zero()
                })
            });
            out.push(Leg {
                chain,
                foot,
                rest_end,
                rest_knee: knee,
                len,
                pole_dir,
                phase,
            });
        }
        let legs = out;

        // Crouch (joelhos levemente dobrados) só quando as pernas estão esticadas.
        let crouch = if any_bent { 0.0 } else { 0.10 * mean_len };

        let bounce_scale = if n <= 4 { 0.02 } else { 0.006 };
        let bounce = bounce_scale * mean_len * (0.5 + 0.5 * weight) * energy;
        let sway = if n <= 4 { 0.03 } else { 0.008 } * mean_len * weight * energy;
        let step_h =
            recipe.param("step_height") * 0.25 * mean_len * (1.0 + 0.5 * run) * energy.max(0.15);

        // Alcance: o passo é limitado para que todos os alvos sejam alcançáveis.
        let base_stride = if biped {
            0.55 + 0.4 * run
        } else if n <= 4 {
            0.6
        } else {
            0.5
        };
        let desired = stride * base_stride * mean_len;
        let mut reach = f32::MAX;
        for (k, l) in legs.iter().enumerate() {
            let hip = hips[k] + frame.up * (bounce - crouch);
            let v = l.rest_end - hip;
            let fa_i = v.dot(frame.fwd).abs();
            let lat_i = v.dot(frame.lat);
            let vert = v.dot(frame.up);
            let room = (0.97 * l.len).powi(2) - lat_i * lat_i - vert * vert;
            let half = if room > 0.0 { room.sqrt() - fa_i } else { 0.0 };
            reach = reach.min(2.0 * half.max(0.0));
        }
        let stride_len = desired.min(reach).max(0.0);

        let cycle = if biped {
            (1.1 - 0.42 * run) / speed
        } else if n <= 4 {
            1.2 / speed
        } else {
            0.9 / speed
        };
        let speed_v = stride_len / (duty * cycle);

        // Braços (bípedes/asas não entram): pares com a perna do mesmo membro.
        let mut arms = Vec::new();
        let mut limbs: Vec<u8> = roles
            .entries()
            .iter()
            .filter_map(|e| match e.role {
                RigRole::Arm { limb, .. } => Some(limb),
                _ => None,
            })
            .collect();
        limbs.sort_unstable();
        limbs.dedup();
        for limb in limbs {
            let Some(upper) = roles.bone_of(RigRole::Arm {
                limb,
                part: ArmPart::Upper,
            }) else {
                continue;
            };
            let lower = roles.bone_of(RigRole::Arm {
                limb,
                part: ArmPart::Lower,
            });
            let partner = leg_defs
                .iter()
                .position(|l| l.limb == limb)
                .map(|i| legs[i].phase)
                .unwrap_or(f32::from(limb % 2) * 0.5);
            arms.push(Arm {
                upper,
                lower,
                phase: (partner + 0.5).rem_euclid(1.0),
            });
        }

        Ok(Self {
            biped,
            legs,
            arms,
            spine: roles.spine_chain(),
            chest: roles.bone_of(RigRole::Chest),
            tail: roles.tail_chain(),
            cycle,
            duty,
            stride_len,
            step_h,
            crouch,
            bounce,
            sway,
            speed_v,
            energy,
            smooth,
            lean: recipe.param("lean"),
            run,
            arm_swing: if biped {
                recipe.param("arm_swing")
            } else {
                0.0
            },
            spine_wave: if biped {
                0.0
            } else {
                recipe.param("spine_wave")
            },
            tail_sway: if biped {
                0.5
            } else {
                recipe.param("tail_sway")
            },
        })
    }

    fn apply(&self, b: &Basis, pose: &mut [Transform3D], t: f32, root_motion: bool) {
        let phi = phase_of(t, self.cycle);
        let Frame { fwd, up, lat } = b.frame;
        let e = self.energy;
        let d = self.duty;
        let root_off = if root_motion {
            fwd * self.speed_v * t
        } else {
            Vec3::ZERO
        };

        // Corpo: bounce (2x por ciclo), sway lateral e crouch.
        let bob = self.bounce * (2.0 * TAU * (phi - d * 0.5)).cos();
        let side = self.sway * (TAU * (phi - d * 0.5)).cos();
        b.offset(
            pose,
            b.root,
            up * (bob - self.crouch) + lat * side + root_off,
        );

        // Tronco: inclinação (lean) e torção.
        let torso: Vec<u32> = self.spine.iter().copied().chain(self.chest).collect();
        if !torso.is_empty() {
            let pitch = (self.lean * 0.25 + self.run * 0.12) * e.max(0.3);
            for &bone in &torso {
                b.swing(pose, bone, lat, pitch / torso.len() as f32);
            }
            if self.biped {
                let yaw = 0.10 * e * (0.5 + 0.5 * self.stride_len.min(1.0));
                let twist = yaw * (TAU * phi).cos();
                b.swing(pose, b.root, up, -0.6 * twist);
                for &bone in &torso {
                    b.swing(pose, bone, up, twist / torso.len() as f32);
                }
            }
        }
        // Ondulação da coluna e cauda (gait).
        for (k, &bone) in self.spine.iter().enumerate() {
            let a = 0.08 * e * self.spine_wave * (TAU * (phi - 0.1 * k as f32)).sin();
            b.swing(pose, bone, up, a);
        }
        for (k, &bone) in self.tail.iter().enumerate() {
            let a = 0.20 * e * self.tail_sway * (TAU * (phi - 0.15 * (k as f32 + 1.0))).sin();
            b.swing(pose, bone, up, a);
        }

        // Pernas: alvos no referencial do chão + IK de dois ossos.
        let hips_shift = up * (bob - self.crouch) + lat * side + root_off;
        let mut phases = Vec::with_capacity(self.legs.len());
        for leg in &self.legs {
            let u = (phi + leg.phase).rem_euclid(1.0);
            let s = self.stride_len;
            let (z, y) = if u < d {
                (s * 0.5 - s * (u / d), 0.0)
            } else {
                let q = (u - d) / (1.0 - d);
                (
                    -s * 0.5 + s * ease(q, self.smooth),
                    self.step_h * (PI * q).sin(),
                )
            };
            let target = leg.rest_end + fwd * z + up * y + root_off;
            let mut chain = leg.chain.clone();
            chain.pole = leg
                .pole_dir
                .map(|p| (leg.rest_knee + p * leg.len + hips_shift).to_array());
            let _ = solve_chain(&b.sk, pose, &chain, target);
            phases.push(u);
        }
        // Pés: achatados no chão, com toe-off e toe-up.
        let world = b.sk.world_pose_matrices(pose).unwrap_or_default();
        if world.len() == pose.len() {
            for (leg, &u) in self.legs.iter().zip(&phases) {
                let Some(foot) = leg.foot else { continue };
                let pitch = foot_pitch(u, d);
                let want = Quat::from_axis_angle(lat, pitch) * b.rest_rot[b.idx(foot)];
                b.set_world_rotation(pose, &world, foot, want);
            }
        }

        // Braços: contralaterais às pernas; cotovelo dobra para a frente.
        for arm in &self.arms {
            let a = 0.4
                * self.arm_swing
                * e
                * (0.6 + 0.4 * self.stride_len.min(1.5))
                * (1.0 + 0.6 * self.run);
            let swing = a * (TAU * (phi + arm.phase)).cos();
            b.swing(pose, arm.upper, lat, -swing);
            if let Some(lower) = arm.lower {
                let flex = ((0.15 + 0.5 * self.run) * self.arm_swing.min(1.5)
                    + 0.15 * (swing / a.max(1e-3)).max(0.0) * self.arm_swing.min(1.0))
                    * e.min(1.0);
                b.swing(pose, lower, lat, -flex);
            }
        }
    }
}

/// Inclinação do pé em torno da lateral (positivo = ponta para baixo).
fn foot_pitch(u: f32, duty: f32) -> f32 {
    if u < duty {
        let heel = -0.15 * (1.0 - (u / 0.1).clamp(0.0, 1.0));
        let toe = ((u - (duty - 0.18)) / 0.18).clamp(0.0, 1.0);
        heel + 0.35 * toe * toe
    } else {
        let q = (u - duty) / (1.0 - duty);
        0.35 * (1.0 - (q * 2.0).min(1.0)) - 0.15 * ((q - 0.6) / 0.4).clamp(0.0, 1.0)
    }
}

// ---------------------------------------------------------------------------
// Onda serpentina (serpente, peixe, cauda)
// ---------------------------------------------------------------------------

struct Serpentine {
    /// Ossos da cabeça para a cauda.
    chain: Vec<u32>,
    cycle: f32,
    amp: f32,
    wavelength: f32,
    tail_boost: f32,
    head_hold: f32,
    smooth: f32,
    variation: f32,
    seed: u32,
}

impl Serpentine {
    fn new(recipe: &MotionRecipe, b: &Basis, roles: &RigRoleMap) -> Self {
        let mut ids: Vec<u32> = roles.spine_chain();
        ids.extend(roles.tail_chain());
        for role in [RigRole::Head, RigRole::Root] {
            if let Some(id) = roles.bone_of(role)
                && !ids.contains(&id)
            {
                ids.push(id);
            }
        }
        // Da cabeça (mais à frente) para a cauda.
        ids.sort_by(|&a, &c| {
            let (pa, pc) = (
                b.rest_head[b.idx(a)].dot(b.frame.fwd),
                b.rest_head[b.idx(c)].dot(b.frame.fwd),
            );
            pc.total_cmp(&pa)
        });
        let n = ids.len().max(1) as f32;
        Self {
            chain: ids,
            cycle: 2.0 / recipe.param("speed"),
            amp: 0.30 * recipe.param("energy") * (8.0 / n).sqrt(),
            wavelength: recipe.param("wavelength"),
            tail_boost: recipe.param("tail_boost"),
            head_hold: recipe.param("head_hold"),
            smooth: recipe.param("smoothness"),
            variation: recipe.param("variation"),
            seed: recipe.seed,
        }
    }

    fn apply(&self, b: &Basis, pose: &mut [Transform3D], t: f32) {
        let phi = phase_of(t, self.cycle);
        let n = self.chain.len();
        for (k, &bone) in self.chain.iter().enumerate() {
            let u = if n > 1 {
                k as f32 / (n - 1) as f32
            } else {
                0.0
            };
            let boost = 1.0 + (0.4 + 0.6 * u - 1.0) * self.tail_boost;
            let head = 1.0 - self.head_hold * (1.0 - (k as f32 / 3.0).min(1.0));
            let x = TAU * (phi - k as f32 / self.wavelength)
                + self.variation * 0.3 * jitter(self.seed, k as u32);
            let s = x.sin();
            let shaped = s.signum() * s.abs().powf(0.6) * (1.0 - self.smooth) + s * self.smooth;
            b.swing(pose, bone, b.frame.up, self.amp * boost * head * shaped);
        }
    }
}

// ---------------------------------------------------------------------------
// Respirar / balançar
// ---------------------------------------------------------------------------

struct Idle {
    chest: Option<u32>,
    spine: Vec<u32>,
    head: Option<u32>,
    arms: Vec<(u32, f32)>,
    cycle: f32,
    energy: f32,
    weight: f32,
    sway: f32,
    size: f32,
    variation: f32,
    seed: u32,
}

impl Idle {
    fn new(recipe: &MotionRecipe, b: &Basis, roles: &RigRoleMap) -> Self {
        let mut arms = Vec::new();
        for e in roles.entries() {
            if let RigRole::Arm {
                limb,
                part: ArmPart::Upper,
            } = e.role
            {
                arms.push((e.bone_id, if limb % 2 == 0 { 1.0 } else { -1.0 }));
            }
        }
        Self {
            chest: roles.bone_of(RigRole::Chest),
            spine: roles.spine_chain(),
            head: roles.bone_of(RigRole::Head),
            arms,
            cycle: 3.5 / recipe.param("speed"),
            energy: recipe.param("energy"),
            weight: recipe.param("weight"),
            sway: recipe.param("sway"),
            size: b.size(),
            variation: recipe.param("variation"),
            seed: recipe.seed,
        }
    }

    fn apply(&self, b: &Basis, pose: &mut [Transform3D], t: f32) {
        let phi = phase_of(t, self.cycle);
        let Frame { fwd, up, lat } = b.frame;
        let e = self.energy;
        let breath = (TAU * phi).sin();
        // Respiração: tórax sobe e inclina levemente; coluna acompanha.
        if let Some(c) = self.chest {
            b.offset(pose, c, up * (0.006 * self.size * e * breath));
            b.swing(pose, c, lat, -0.03 * e * breath);
        }
        for &s in &self.spine {
            b.swing(pose, s, lat, -0.01 * e * breath);
        }
        if let Some(h) = self.head {
            let j = self.variation * 0.2 * jitter(self.seed, 1);
            b.swing(pose, h, lat, 0.015 * e * (TAU * (phi + j)).sin());
        }
        for &(arm, sign) in &self.arms {
            b.swing(pose, arm, fwd, sign * 0.02 * e * breath);
        }
        // Peso: o corpo afunda um pouco na inspiração e desloca-se de lado.
        b.offset(
            pose,
            b.root,
            up * (-0.004 * self.size * e * self.weight * breath)
                + lat * (0.006 * self.size * e * self.sway * self.weight * (TAU * phi).sin()),
        );
    }
}
