//! IK mínimo (P3D-169, cap. 45 F0): two-bone analítico com soft IK, FABRIK e
//! look-at. Solvers **puros e determinísticos** (sem alocação por chamada além
//! dos vetores de junta) e a aplicação sobre a pose de um `Skeleton`.
//!
//! Escopo deliberado: não é um stack geral de constraints. Um [`IkChain`] é um
//! dado (ossos contíguos, solver, pole, peso) que geradores de movimento e
//! comandos consomem.
//!
//! Convenções: posições no espaço do esqueleto; a solução preserva o
//! comprimento dos segmentos e a **torção** atual de cada osso (só gira o
//! "swing" até a nova direção).

use crate::rig::{RigError, Skeleton, Transform3D};
use crate::rig_roles::LegChain;
use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const EPS: f32 = 1e-6;
const REACH_TOLERANCE: f32 = 1e-3;

/// Solver de uma cadeia.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum IkSolver {
    /// Analítico para dois segmentos (pernas e braços). `soft` é a fração
    /// (0..=0.5) do comprimento total em que a extensão é suavizada.
    TwoBone { soft: f32 },
    /// Cadeias longas (cauda, tentáculo, serpente). Limites angulares ficam
    /// como evolução futura.
    Fabrik { iterations: u8, tolerance: f32 },
    /// Aponta o primeiro osso da cadeia (cabeça, olhar, mira).
    LookAt,
}

/// Cadeia de IK: ossos contíguos (cada um filho do anterior), da raiz à ponta.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IkChain {
    pub id: Uuid,
    pub name: String,
    pub skeleton_id: Uuid,
    pub solver: IkSolver,
    /// IDs de osso, da raiz à ponta.
    pub bones: Vec<u32>,
    /// Ponto no espaço do esqueleto para onde o joelho/cotovelo aponta.
    pub pole: Option<[f32; 3]>,
    /// Mistura 0..=1 entre a pose atual e a solução.
    pub weight: f32,
    #[serde(default)]
    pub revision: u64,
}

impl IkChain {
    pub fn new(
        skeleton_id: Uuid,
        name: impl Into<String>,
        solver: IkSolver,
        bones: Vec<u32>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            skeleton_id,
            solver,
            bones,
            pole: None,
            weight: 1.0,
            revision: 0,
        }
    }

    /// Perna com IK de dois ossos (coxa, canela e, se houver, o pé).
    pub fn two_bone_leg(skeleton_id: Uuid, name: impl Into<String>, leg: &LegChain) -> Self {
        let mut bones = vec![leg.thigh, leg.shin];
        bones.extend(leg.foot);
        Self::new(skeleton_id, name, IkSolver::TwoBone { soft: 0.05 }, bones)
    }

    /// Valida ossos existentes, contíguos e o mínimo do solver.
    pub fn validate(&self, skeleton: &Skeleton) -> Result<(), IkError> {
        if skeleton.id != self.skeleton_id {
            return Err(IkError::WrongSkeleton);
        }
        let needed = match self.solver {
            IkSolver::TwoBone { .. } => 2,
            IkSolver::Fabrik { .. } | IkSolver::LookAt => 1,
        };
        if self.bones.len() < needed {
            return Err(IkError::ChainTooShort {
                needed,
                found: self.bones.len(),
            });
        }
        for (i, &id) in self.bones.iter().enumerate() {
            let bone = skeleton.get_bone(id).ok_or(IkError::BoneNotFound(id))?;
            if i > 0 && bone.parent != Some(self.bones[i - 1]) {
                return Err(IkError::BrokenChain { bone: id });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum IkError {
    #[error("osso {0} não existe no esqueleto")]
    BoneNotFound(u32),
    #[error("a cadeia precisa de {needed} osso(s) ou mais; tem {found}")]
    ChainTooShort { needed: usize, found: usize },
    #[error("o osso {bone} não é filho do osso anterior da cadeia")]
    BrokenChain { bone: u32 },
    #[error("a cadeia pertence a outro esqueleto")]
    WrongSkeleton,
    #[error(transparent)]
    Rig(#[from] RigError),
}

/// Resultado de uma solução.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IkOutcome {
    /// A ponta alcançou o alvo (dentro de 1 mm de unidade do esqueleto).
    pub reached: bool,
    /// Distância final entre a ponta e o alvo.
    pub error: f32,
}

// ---------------------------------------------------------------------------
// Solvers puros
// ---------------------------------------------------------------------------

/// Vetor unitário ortogonal a `dir`, determinístico (fallback quando não há pole).
fn any_perpendicular(dir: Vec3) -> Vec3 {
    let axis = if dir.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    (axis - dir * axis.dot(dir)).normalize_or_zero()
}

/// IK analítico de dois ossos. Devolve as novas posições `(mid, end)`; a raiz
/// fica fixa e os comprimentos `|mid−root|` e `|end−mid|` são preservados.
///
/// - `pole`: ponto para onde o joelho/cotovelo aponta; sem ele usa o lado em que
///   `mid` já está e, se a perna estiver esticada, um perpendicular determinístico.
/// - `soft`: fração (0..=0.5) do comprimento total em que a extensão é suavizada
///   exponencialmente (sem "estalo" ao esticar).
///
/// Alvos inalcançáveis são limitados à extensão máxima; alvos perto demais, à
/// dobra mínima `|l1−l2|`.
pub fn solve_two_bone(
    root: Vec3,
    mid: Vec3,
    end: Vec3,
    target: Vec3,
    pole: Option<Vec3>,
    soft: f32,
) -> (Vec3, Vec3) {
    let l1 = (mid - root).length();
    let l2 = (end - mid).length();
    if l1 < EPS || l2 < EPS || !target.is_finite() {
        return (mid, end);
    }
    let max = l1 + l2;
    let min = (l1 - l2).abs();

    let to_target = target - root;
    let dist = to_target.length();
    let dir = if dist > EPS {
        to_target / dist
    } else {
        (end - root).try_normalize().unwrap_or(Vec3::Y)
    };

    // Soft IK: suaviza a aproximação do alcance máximo.
    let ds = soft.clamp(0.0, 0.5) * max;
    let mut d = dist;
    if ds > EPS {
        let da = max - ds;
        if d > da {
            d = da + ds * (1.0 - ((da - d) / ds).exp());
        }
    }
    let d = d.clamp(min.max(EPS), max * (1.0 - 1e-6));

    let cos_a = ((l1 * l1 + d * d - l2 * l2) / (2.0 * l1 * d)).clamp(-1.0, 1.0);
    let sin_a = (1.0 - cos_a * cos_a).sqrt();

    // Direção da dobra: componente do pole (ou do joelho atual) ⟂ ao eixo.
    let perp = |v: Vec3| (v - dir * v.dot(dir)).try_normalize();
    let bend = pole
        .and_then(|p| perp(p - root))
        .or_else(|| perp(mid - root))
        .unwrap_or_else(|| any_perpendicular(dir));

    (
        root + dir * (l1 * cos_a) + bend * (l1 * sin_a),
        root + dir * d,
    )
}

/// FABRIK (Aristidou & Lasenby, 2011) com a raiz fixa. `joints[0]` é a raiz;
/// os comprimentos entre juntas são preservados. Devolve as novas juntas e se
/// a ponta chegou ao alvo. Alvo fora de alcance ⇒ cadeia esticada em sua direção.
pub fn solve_fabrik(
    joints: &[Vec3],
    target: Vec3,
    iterations: u8,
    tolerance: f32,
) -> (Vec<Vec3>, bool) {
    let n = joints.len();
    if n < 2 || !target.is_finite() {
        return (joints.to_vec(), false);
    }
    let lengths: Vec<f32> = joints.windows(2).map(|w| (w[1] - w[0]).length()).collect();
    let total: f32 = lengths.iter().sum();
    let root = joints[0];
    let mut p = joints.to_vec();
    let tol = tolerance.max(1e-6);

    let to_target = target - root;
    if to_target.length() >= total {
        let dir = to_target.try_normalize().unwrap_or(Vec3::Y);
        for i in 0..n - 1 {
            p[i + 1] = p[i] + dir * lengths[i];
        }
        let reached = (p[n - 1] - target).length() <= tol;
        return (p, reached);
    }

    for _ in 0..iterations.max(1) {
        if (p[n - 1] - target).length() <= tol {
            break;
        }
        // Passada para trás: da ponta até a raiz.
        p[n - 1] = target;
        for i in (0..n - 1).rev() {
            let dir = (p[i] - p[i + 1]).try_normalize().unwrap_or(Vec3::Y);
            p[i] = p[i + 1] + dir * lengths[i];
        }
        // Passada para frente: da raiz até a ponta.
        p[0] = root;
        for i in 0..n - 1 {
            let dir = (p[i + 1] - p[i]).try_normalize().unwrap_or(Vec3::Y);
            p[i + 1] = p[i] + dir * lengths[i];
        }
    }
    let reached = (p[n - 1] - target).length() <= tol;
    (p, reached)
}

// ---------------------------------------------------------------------------
// Aplicação sobre a pose do esqueleto
// ---------------------------------------------------------------------------

fn world_rotation(m: &Mat4) -> Quat {
    m.to_scale_rotation_translation().1.normalize()
}

/// Posição mundial da ponta (`tail`) do osso `idx` na pose `world`.
fn posed_tail(skeleton: &Skeleton, world: &[Mat4], idx: usize) -> Vec3 {
    let bone = &skeleton.bones[idx];
    let ibm = Mat4::from_cols_array(&bone.inverse_bind_matrix);
    let local = ibm.transform_vector3(Vec3::from(bone.tail) - Vec3::from(bone.head));
    world[idx].transform_point3(local)
}

/// Resolve `chain` para que sua ponta alcance `target` (espaço do esqueleto),
/// alterando **somente as rotações locais** dos ossos da cadeia em `pose`
/// (uma entrada por osso, na ordem de `skeleton.bones`).
pub fn solve_chain(
    skeleton: &Skeleton,
    pose: &mut [Transform3D],
    chain: &IkChain,
    target: Vec3,
) -> Result<IkOutcome, IkError> {
    chain.validate(skeleton)?;
    let idx: Vec<usize> = chain
        .bones
        .iter()
        .map(|&id| skeleton.bone_index(id).ok_or(IkError::BoneNotFound(id)))
        .collect::<Result<_, _>>()?;
    let world = skeleton.world_pose_matrices(pose)?;
    let head = |i: usize| world[idx[i]].w_axis.truncate();
    let weight = chain.weight.clamp(0.0, 1.0);

    // Juntas atuais e novas (segmento i vai de juntas[i] a juntas[i+1]).
    let (cur, new, reached): (Vec<Vec3>, Vec<Vec3>, Option<bool>) = match chain.solver {
        IkSolver::TwoBone { soft } => {
            let end_cur = if idx.len() >= 3 {
                head(2)
            } else {
                posed_tail(skeleton, &world, idx[1])
            };
            let (mid, end) = solve_two_bone(
                head(0),
                head(1),
                end_cur,
                target,
                chain.pole.map(Vec3::from),
                soft,
            );
            (
                vec![head(0), head(1), end_cur],
                vec![head(0), mid, end],
                None,
            )
        }
        IkSolver::Fabrik {
            iterations,
            tolerance,
        } => {
            let mut cur: Vec<Vec3> = (0..idx.len()).map(head).collect();
            cur.push(posed_tail(
                skeleton,
                &world,
                *idx.last().expect("cadeia validada"),
            ));
            let (new, reached) = solve_fabrik(&cur, target, iterations, tolerance);
            (cur, new, Some(reached))
        }
        IkSolver::LookAt => {
            let root = head(0);
            let tip = posed_tail(skeleton, &world, idx[0]);
            let len = (tip - root).length();
            let dir = (target - root).try_normalize().unwrap_or(Vec3::Y);
            (vec![root, tip], vec![root, root + dir * len], None)
        }
    };

    // Rotaciona cada osso do swing atual até a nova direção do segmento.
    let segments = new.len() - 1;
    let first = idx[0];
    let mut parent_rot = skeleton.bones[first]
        .parent
        .and_then(|pid| skeleton.bone_index(pid))
        .map_or(Quat::IDENTITY, |pi| world_rotation(&world[pi]));
    for s in 0..segments.min(idx.len()) {
        let cur_dir = (cur[s + 1] - cur[s]).try_normalize();
        let new_dir = (new[s + 1] - new[s]).try_normalize();
        let bone_i = idx[s];
        let cur_world = world_rotation(&world[bone_i]);
        let cur_local = Quat::from_array(pose[bone_i].rotation).normalize();
        let solved_world = match (cur_dir, new_dir) {
            (Some(a), Some(b)) => Quat::from_rotation_arc(a, b) * cur_world,
            _ => cur_world,
        };
        let solved_local = (parent_rot.inverse() * solved_world).normalize();
        let blended = if weight >= 1.0 {
            solved_local
        } else {
            cur_local.slerp(solved_local, weight).normalize()
        };
        pose[bone_i].rotation = blended.to_array();
        parent_rot = (parent_rot * blended).normalize();
    }

    let end_new = *new.last().expect("juntas");
    let error = (end_new - target).length();
    // `reached`/`error` descrevem a solução do solver; o `weight` < 1 mistura a
    // pose atual e não entra nessa medida.
    let reached = reached.unwrap_or(error <= REACH_TOLERANCE);
    Ok(IkOutcome { reached, error })
}

/// Posição mundial atual da ponta da cadeia (efetuador) numa pose.
pub fn chain_end_position(
    skeleton: &Skeleton,
    pose: &[Transform3D],
    chain: &IkChain,
) -> Result<Vec3, IkError> {
    chain.validate(skeleton)?;
    let world = skeleton.world_pose_matrices(pose)?;
    let last = *chain.bones.last().expect("cadeia validada");
    let li = skeleton
        .bone_index(last)
        .ok_or(IkError::BoneNotFound(last))?;
    Ok(match chain.solver {
        IkSolver::TwoBone { .. } if chain.bones.len() >= 3 => world[li].w_axis.truncate(),
        _ => posed_tail(skeleton, &world, li),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RigPreset, RigRoleMap};

    fn v(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3::new(x, y, z)
    }

    fn assert_lengths(a: &[Vec3], b: &[Vec3]) {
        for (w0, w1) in a.windows(2).zip(b.windows(2)) {
            let (l0, l1) = ((w0[1] - w0[0]).length(), (w1[1] - w1[0]).length());
            assert!((l0 - l1).abs() < 1e-4, "comprimento {l0} → {l1}");
        }
    }

    #[test]
    fn two_bone_reaches_reachable_targets_and_keeps_lengths() {
        let (root, mid, end) = (v(0.0, 2.0, 0.0), v(0.0, 1.0, 0.1), v(0.0, 0.0, 0.0));
        for target in [
            v(0.3, 0.6, 0.2),
            v(-0.5, 0.8, 0.4),
            v(0.0, 0.5, 0.0),
            v(0.9, 1.5, -0.3),
        ] {
            let (m, e) = solve_two_bone(root, mid, end, target, None, 0.0);
            assert!((e - target).length() < 1e-4, "alvo {target:?} → {e:?}");
            assert_lengths(&[root, mid, end], &[root, m, e]);
        }
    }

    #[test]
    fn two_bone_clamps_unreachable_and_too_close_targets_without_nan() {
        let (root, mid, end) = (v(0.0, 2.0, 0.0), v(0.0, 1.0, 0.0), v(0.0, 0.0, 0.0));
        // Longe demais: estica na direção do alvo sem passar do alcance.
        let (m, e) = solve_two_bone(root, mid, end, v(0.0, -10.0, 0.0), None, 0.0);
        assert!(m.is_finite() && e.is_finite());
        assert!((e - root).length() <= 2.0 + 1e-4);
        assert!(e.y < 0.0 + 1e-3 && e.y > -1e-3, "{e:?}");
        assert_lengths(&[root, mid, end], &[root, m, e]);
        // Exatamente no root e no root deslocado: não explode.
        for t in [root, root + v(0.0, 1e-9, 0.0)] {
            let (m, e) = solve_two_bone(root, mid, end, t, None, 0.1);
            assert!(m.is_finite() && e.is_finite());
            assert_lengths(&[root, mid, end], &[root, m, e]);
        }
        // Alvo não finito: mantém a pose.
        assert_eq!(
            solve_two_bone(root, mid, end, v(f32::NAN, 0.0, 0.0), None, 0.0),
            (mid, end)
        );
    }

    #[test]
    fn two_bone_pole_chooses_the_bend_side() {
        let (root, mid, end) = (v(0.0, 2.0, 0.0), v(0.0, 1.0, 0.0), v(0.0, 0.0, 0.0));
        let target = v(0.0, 0.7, 0.0);
        let (m_front, _) = solve_two_bone(root, mid, end, target, Some(v(0.0, 1.0, 5.0)), 0.0);
        let (m_back, _) = solve_two_bone(root, mid, end, target, Some(v(0.0, 1.0, -5.0)), 0.0);
        assert!(m_front.z > 0.1, "{m_front:?}");
        assert!(m_back.z < -0.1, "{m_back:?}");
        // Sem pole e perna esticada: perpendicular determinístico e finito.
        let a = solve_two_bone(root, v(0.0, 1.0, 0.0), end, v(0.0, 0.0, 0.0), None, 0.0);
        let b = solve_two_bone(root, v(0.0, 1.0, 0.0), end, v(0.0, 0.0, 0.0), None, 0.0);
        assert_eq!(a, b);
        assert!(a.0.is_finite());
    }

    #[test]
    fn soft_ik_is_continuous_and_never_overextends() {
        let (root, mid, end) = (v(0.0, 2.0, 0.0), v(0.0, 1.0, 0.1), v(0.0, 0.0, 0.0));
        let reach = 2.0f32;
        let mut prev: Option<f32> = None;
        for i in 0..=120 {
            let dist = reach * (0.7 + 0.5 * i as f32 / 120.0); // 70%..120% do alcance
            let (_, e) = solve_two_bone(root, mid, end, root + v(0.0, -dist, 0.0), None, 0.2);
            let d = (e - root).length();
            assert!(d < reach, "passou do alcance: {d}");
            if let Some(p) = prev {
                assert!(d + 1e-6 >= p, "não monotônico");
                assert!(d - p < 0.02, "salto entre amostras: {p} → {d}");
            }
            prev = Some(d);
        }
        // Sem soft o alvo dentro do alcance é atingido exatamente.
        let t = root + v(0.0, -1.9, 0.0);
        let (_, hard) = solve_two_bone(root, mid, end, t, None, 0.0);
        assert!((hard - t).length() < 1e-3);
    }

    #[test]
    fn fabrik_converges_preserves_lengths_and_root() {
        let joints: Vec<Vec3> = (0..6).map(|i| v(0.0, i as f32 * 0.5, 0.0)).collect();
        let target = v(1.2, 1.4, 0.6);
        let (p, reached) = solve_fabrik(&joints, target, 30, 1e-3);
        assert!(reached, "{:?}", p.last());
        assert_eq!(p[0], joints[0]);
        assert_lengths(&joints, &p);

        // Fora de alcance: estica na direção do alvo.
        let far = v(0.0, 50.0, 0.0);
        let (p, reached) = solve_fabrik(&joints, far, 30, 1e-3);
        assert!(!reached);
        assert!((p[5] - v(0.0, 2.5, 0.0)).length() < 1e-4);
        assert_lengths(&joints, &p);
        // Determinismo.
        assert_eq!(
            solve_fabrik(&joints, target, 30, 1e-3),
            solve_fabrik(&joints, target, 30, 1e-3)
        );
    }

    /// Cadeia da perna esquerda do humanoide como `IkChain` de 3 ossos.
    fn leg(sk: &Skeleton) -> IkChain {
        let roles = RigRoleMap::infer(sk);
        let l = roles.legs()[0];
        IkChain::two_bone_leg(sk.id, "LegL", &l)
    }

    fn rest_pose(sk: &Skeleton) -> Vec<Transform3D> {
        sk.bones.iter().map(|b| b.local_transform).collect()
    }

    fn head_of(sk: &Skeleton, pose: &[Transform3D], name: &str) -> Vec3 {
        let w = sk.world_pose_matrices(pose).unwrap();
        w[sk.bone_index(sk.find_bone(name).unwrap()).unwrap()]
            .w_axis
            .truncate()
    }

    #[test]
    fn leg_chain_lifts_the_foot_to_the_target_and_only_touches_the_chain() {
        let sk = RigPreset::humanoid(1.0);
        let chain = leg(&sk);
        let mut pose = rest_pose(&sk);
        let foot0 = chain_end_position(&sk, &pose, &chain).unwrap();
        let target = foot0 + v(0.05, 0.25, 0.15);

        let before = pose.clone();
        let out = solve_chain(&sk, &mut pose, &chain, target).unwrap();
        assert!(out.reached, "erro {}", out.error);

        // O pé (cabeça do osso Foot.L) chegou ao alvo.
        let foot = head_of(&sk, &pose, "Foot.L");
        assert!((foot - target).length() < 2e-3, "{foot:?} vs {target:?}");
        // Comprimentos preservados; quadril não se move.
        let (hip, knee) = (
            head_of(&sk, &pose, "UpperLeg.L"),
            head_of(&sk, &pose, "LowerLeg.L"),
        );
        let (hip0, knee0, foot0h) = (
            head_of(&sk, &before, "UpperLeg.L"),
            head_of(&sk, &before, "LowerLeg.L"),
            head_of(&sk, &before, "Foot.L"),
        );
        assert_lengths(&[hip0, knee0, foot0h], &[hip, knee, foot]);
        assert!((hip - hip0).length() < 1e-5);

        // Apenas as rotações dos ossos da cadeia (coxa e canela) mudaram.
        let changed: Vec<&str> = sk
            .bones
            .iter()
            .enumerate()
            .filter(|(i, _)| pose[*i] != before[*i])
            .map(|(_, b)| b.name.as_str())
            .collect();
        assert_eq!(changed, vec!["UpperLeg.L", "LowerLeg.L"]);
        for i in 0..pose.len() {
            assert_eq!(pose[i].translation, before[i].translation);
        }
    }

    #[test]
    fn chain_solutions_are_deterministic_and_respect_weight() {
        let sk = RigPreset::humanoid(1.0);
        let chain = leg(&sk);
        let target = chain_end_position(&sk, &rest_pose(&sk), &chain).unwrap() + v(0.0, 0.2, 0.1);

        let (mut a, mut b) = (rest_pose(&sk), rest_pose(&sk));
        solve_chain(&sk, &mut a, &chain, target).unwrap();
        solve_chain(&sk, &mut b, &chain, target).unwrap();
        assert_eq!(a, b);

        // weight 0 = pose intacta.
        let mut none = rest_pose(&sk);
        let mut c0 = chain.clone();
        c0.weight = 0.0;
        solve_chain(&sk, &mut none, &c0, target).unwrap();
        assert_eq!(none, rest_pose(&sk));

        // weight 0.5 = rotação intermediária entre repouso e solução.
        let mut half = rest_pose(&sk);
        let mut c5 = chain.clone();
        c5.weight = 0.5;
        solve_chain(&sk, &mut half, &c5, target).unwrap();
        let i = sk.bone_index(chain.bones[0]).unwrap();
        let rest_q = Quat::from_array(rest_pose(&sk)[i].rotation);
        let full_q = Quat::from_array(a[i].rotation);
        let half_q = Quat::from_array(half[i].rotation);
        let (total, part) = (rest_q.angle_between(full_q), rest_q.angle_between(half_q));
        assert!((part - total * 0.5).abs() < 1e-3, "{part} vs {total}");
    }

    #[test]
    fn fabrik_and_look_at_chains_drive_the_skeleton() {
        // Cauda de 3 ossos.
        let mut sk = Skeleton::new("Tail");
        let root = sk
            .add_bone("Root", None, [0.0, 1.0, 0.0], [0.0, 1.0, 0.5])
            .unwrap();
        let t0 = sk
            .add_bone("Tail_0", Some(root), [0.0, 1.0, 0.5], [0.0, 1.0, 1.0])
            .unwrap();
        let t1 = sk
            .add_bone("Tail_1", Some(t0), [0.0, 1.0, 1.0], [0.0, 1.0, 1.5])
            .unwrap();
        let t2 = sk
            .add_bone("Tail_2", Some(t1), [0.0, 1.0, 1.5], [0.0, 1.0, 2.0])
            .unwrap();
        let chain = IkChain::new(
            sk.id,
            "Tail",
            IkSolver::Fabrik {
                iterations: 20,
                tolerance: 1e-3,
            },
            vec![t0, t1, t2],
        );
        let mut pose = rest_pose(&sk);
        let target = v(0.6, 1.4, 1.5);
        let out = solve_chain(&sk, &mut pose, &chain, target).unwrap();
        assert!(out.reached, "erro {}", out.error);
        let end = chain_end_position(&sk, &pose, &chain).unwrap();
        assert!((end - target).length() < 5e-3, "{end:?}");
        assert_eq!(pose[sk.bone_index(root).unwrap()], rest_pose(&sk)[0]);

        // Look-at: o eixo do osso aponta para o alvo.
        let look = IkChain::new(sk.id, "Look", IkSolver::LookAt, vec![t0]);
        let mut pose = rest_pose(&sk);
        let target = v(3.0, 1.0, 0.5);
        solve_chain(&sk, &mut pose, &look, target).unwrap();
        let w = sk.world_pose_matrices(&pose).unwrap();
        let i = sk.bone_index(t0).unwrap();
        let ibm = Mat4::from_cols_array(&sk.bones[i].inverse_bind_matrix);
        let axis_local =
            ibm.transform_vector3(Vec3::from(sk.bones[i].tail) - Vec3::from(sk.bones[i].head));
        let axis = w[i].transform_vector3(axis_local).normalize();
        let want = (target - w[i].w_axis.truncate()).normalize();
        assert!(axis.dot(want) > 0.9999, "{axis:?} vs {want:?}");
    }

    #[test]
    fn chain_validation_reports_structured_errors() {
        let sk = RigPreset::humanoid(1.0);
        let good = leg(&sk);
        assert!(good.validate(&sk).is_ok());
        let mut broken = good.clone();
        broken.bones.swap(0, 1);
        assert!(matches!(
            broken.validate(&sk),
            Err(IkError::BrokenChain { .. })
        ));
        let mut short = good.clone();
        short.bones.truncate(1);
        assert_eq!(
            short.validate(&sk),
            Err(IkError::ChainTooShort {
                needed: 2,
                found: 1
            })
        );
        let mut ghost = good.clone();
        ghost.bones.push(999);
        assert_eq!(ghost.validate(&sk), Err(IkError::BoneNotFound(999)));
        let other = RigPreset::quadruped(1.0);
        assert_eq!(good.validate(&other), Err(IkError::WrongSkeleton));
        let mut pose = rest_pose(&sk);
        assert!(solve_chain(&other, &mut pose, &good, Vec3::ZERO).is_err());
    }

    #[test]
    fn ik_chains_persist_and_are_pruned_by_the_project() {
        let mut p = crate::Project::new();
        let sk = RigPreset::humanoid(1.0);
        let chain = leg(&sk);
        let sid = sk.id;
        p.add_skeleton(sk);
        p.ik_chains.push(chain.clone());
        let back: crate::Project =
            serde_json::from_value(serde_json::to_value(&p).unwrap()).unwrap();
        assert_eq!(back.ik_chains, vec![chain.clone()]);
        // Osso da cadeia removido: `validate` descarta a cadeia.
        let shin = chain.bones[1];
        p.skeletons[0].remove_bone(shin).unwrap();
        p.validate();
        assert!(p.ik_chains.is_empty());
        p.ik_chains.push(chain);
        p.remove_skeleton(sid);
        assert!(p.ik_chains.is_empty());
    }
}
