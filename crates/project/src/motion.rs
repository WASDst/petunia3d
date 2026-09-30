//! Procedural Motion Generators (P3D-170, cap. 45 F1): dados, parâmetros,
//! Styles, contrato de rig e **Apply Now** (bake com redução de keys).
//!
//! Um [`MotionRecipe`] é dado serializável avaliado como função pura:
//!
//! ```text
//! pose(t) = Generator(rig, roles, params, style, seed, t)
//! ```
//!
//! Mesmo `(receita, rig, t)` ⇒ mesma pose (determinismo por hash). Os geradores
//! falam em **papéis de osso** ([`RigRole`]) e em número de pernas, nunca em
//! espécies. A matemática vive em [`crate::motion_gen`]; este módulo é a
//! superfície pública: parâmetros com faixa, Styles, [`MotionEvaluator`] e
//! [`bake_motion`].

use crate::animation::{AnimationClip, BoneTrack, Interpolation, Keyframe};
use crate::ik::IkError;
use crate::motion_gen::Prepared;
use crate::rig::{Skeleton, Transform3D};
use crate::rig_roles::{RigRoleMap, RoleContractError};
use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Gerador de movimento (P3D-170). Novos geradores entram no fim.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MotionGenerator {
    /// Andar e correr de bípedes (`run_blend` 0 = walk, 1 = run).
    BipedCycle,
    /// Gait de N pernas: quadrúpedes, aranhas, escorpiões, insetos.
    Gait,
    /// Onda serpentina: serpente, peixe, cauda, tentáculo.
    Serpentine,
    /// Respirar e balançar.
    IdleBreath,
}

/// Especificação de um parâmetro: faixa, padrão e se pertence ao painel Advanced.
/// `choices` não vazio ⇒ o valor é o índice (inteiro) da escolha.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParamSpec {
    pub key: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub advanced: bool,
    pub choices: &'static [&'static str],
}

const fn p(key: &'static str, min: f32, max: f32, default: f32, advanced: bool) -> ParamSpec {
    ParamSpec {
        key,
        min,
        max,
        default,
        advanced,
        choices: &[],
    }
}

// Controles universais (camada 1 da UX do Animate).
const SPEED: ParamSpec = p("speed", 0.25, 3.0, 1.0, false);
const ENERGY: ParamSpec = p("energy", 0.0, 2.0, 1.0, false);
const WEIGHT: ParamSpec = p("weight", 0.0, 2.0, 1.0, false);
const STRIDE: ParamSpec = p("stride", 0.3, 2.0, 1.0, false);
const LEAN: ParamSpec = p("lean", -1.0, 1.0, 0.0, false);
const SMOOTHNESS: ParamSpec = p("smoothness", 0.0, 1.0, 0.5, false);
const VARIATION: ParamSpec = p("variation", 0.0, 1.0, 0.0, true);
const STEP_HEIGHT: ParamSpec = p("step_height", 0.0, 1.0, 0.5, true);

const BIPED_SPECS: &[ParamSpec] = &[
    SPEED,
    ENERGY,
    WEIGHT,
    STRIDE,
    LEAN,
    SMOOTHNESS,
    p("run_blend", 0.0, 1.0, 0.0, false),
    p("arm_swing", 0.0, 2.0, 1.0, true),
    STEP_HEIGHT,
    VARIATION,
];

const GAIT_SPECS: &[ParamSpec] = &[
    SPEED,
    ENERGY,
    WEIGHT,
    STRIDE,
    LEAN,
    SMOOTHNESS,
    ParamSpec {
        key: "pattern",
        min: 0.0,
        max: 3.0,
        default: 0.0,
        advanced: true,
        choices: &["auto", "alternate", "lateral", "wave"],
    },
    p("duty", 0.3, 0.9, 0.6, true),
    STEP_HEIGHT,
    p("spine_wave", 0.0, 2.0, 1.0, true),
    p("tail_sway", 0.0, 2.0, 1.0, true),
    VARIATION,
];

const SERPENTINE_SPECS: &[ParamSpec] = &[
    SPEED,
    ENERGY,
    SMOOTHNESS,
    p("wavelength", 1.0, 8.0, 3.5, true),
    p("tail_boost", 0.0, 1.0, 0.5, true),
    p("head_hold", 0.0, 1.0, 0.5, true),
    VARIATION,
];

const IDLE_SPECS: &[ParamSpec] = &[
    SPEED,
    ENERGY,
    WEIGHT,
    SMOOTHNESS,
    p("sway", 0.0, 2.0, 1.0, true),
    VARIATION,
];

impl MotionGenerator {
    pub const ALL: [MotionGenerator; 4] = [
        MotionGenerator::BipedCycle,
        MotionGenerator::Gait,
        MotionGenerator::Serpentine,
        MotionGenerator::IdleBreath,
    ];

    /// Identificador estável (chave de `TextId` e de palette).
    pub fn id(self) -> &'static str {
        match self {
            MotionGenerator::BipedCycle => "biped_cycle",
            MotionGenerator::Gait => "gait",
            MotionGenerator::Serpentine => "serpentine",
            MotionGenerator::IdleBreath => "idle_breath",
        }
    }

    pub fn specs(self) -> &'static [ParamSpec] {
        match self {
            MotionGenerator::BipedCycle => BIPED_SPECS,
            MotionGenerator::Gait => GAIT_SPECS,
            MotionGenerator::Serpentine => SERPENTINE_SPECS,
            MotionGenerator::IdleBreath => IDLE_SPECS,
        }
    }

    /// Roda em loop perfeito no modo In Place.
    pub fn is_cyclic(self) -> bool {
        true
    }

    /// Suporta `RootMode::RootMotion` (deslocamento do corpo).
    pub fn supports_root_motion(self) -> bool {
        matches!(self, MotionGenerator::BipedCycle | MotionGenerator::Gait)
    }

    /// Verifica o contrato de rig (P3D-169): erro estruturado e legível.
    pub fn check_rig(self, roles: &RigRoleMap) -> Result<(), RoleContractError> {
        use crate::rig_roles::{RigRequirement as R, RigRole};
        match self {
            MotionGenerator::BipedCycle => {
                // Pernas primeiro: "precisa de pernas" é o motivo mais útil para
                // quem tenta andar com uma serpente ou um peixe.
                roles.require(&R::Legs { min: 2 })?;
                roles.require(&R::Roles(vec![RigRole::Hips]))
            }
            MotionGenerator::Gait => roles.require(&R::Legs { min: 2 }),
            MotionGenerator::Serpentine => {
                let found = roles.spine_chain().len() + roles.tail_chain().len();
                if found < 3 {
                    return Err(RoleContractError::ChainTooShort {
                        chain: "coluna e cauda",
                        needed: 3,
                        found,
                    });
                }
                Ok(())
            }
            MotionGenerator::IdleBreath => {
                let any = [
                    RigRole::Chest,
                    RigRole::Spine(0),
                    RigRole::Hips,
                    RigRole::Root,
                ]
                .iter()
                .any(|r| roles.bone_of(*r).is_some());
                if any {
                    Ok(())
                } else {
                    Err(RoleContractError::MissingRole(RigRole::Chest))
                }
            }
        }
    }
}

/// Preset de parâmetros (P3D-170). Apenas preenche valores; nunca lógica.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MotionStyle {
    #[default]
    Custom,
    Cartoon,
    Heavy,
    Stiff,
    Floaty,
}

impl MotionStyle {
    pub fn id(self) -> &'static str {
        match self {
            MotionStyle::Custom => "custom",
            MotionStyle::Cartoon => "cartoon",
            MotionStyle::Heavy => "heavy",
            MotionStyle::Stiff => "stiff",
            MotionStyle::Floaty => "floaty",
        }
    }

    /// Valores aplicados; chaves que o gerador não tem são ignoradas.
    pub fn overrides(self) -> &'static [(&'static str, f32)] {
        match self {
            MotionStyle::Custom => &[],
            MotionStyle::Cartoon => &[
                ("energy", 1.4),
                ("weight", 0.8),
                ("smoothness", 0.85),
                ("step_height", 0.8),
                ("arm_swing", 1.4),
                ("stride", 1.15),
            ],
            MotionStyle::Heavy => &[
                ("weight", 1.7),
                ("energy", 0.85),
                ("speed", 0.8),
                ("smoothness", 0.6),
                ("step_height", 0.3),
                ("stride", 0.85),
            ],
            MotionStyle::Stiff => &[
                ("smoothness", 0.1),
                ("energy", 0.8),
                ("arm_swing", 0.5),
                ("spine_wave", 0.3),
                ("tail_sway", 0.3),
            ],
            MotionStyle::Floaty => &[
                ("weight", 0.3),
                ("smoothness", 1.0),
                ("speed", 0.85),
                ("step_height", 0.9),
                ("energy", 1.1),
            ],
        }
    }
}

/// Como o corpo se desloca.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RootMode {
    /// O corpo fica no lugar (padrão; ciclo fecha em loop perfeito).
    #[default]
    InPlace,
    /// O osso raiz avança na direção da frente conforme a velocidade da passada.
    RootMotion,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MotionParam {
    pub key: String,
    pub value: f32,
}

/// Erros de receita, avaliação e bake.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MotionError {
    #[error("parâmetro desconhecido para este movimento: '{0}'")]
    UnknownParam(String),
    #[error("valor não finito para '{0}'")]
    NotFinite(String),
    #[error("fps inválido (use 1 a 240)")]
    InvalidFps,
    #[error("a receita pertence a outro esqueleto")]
    SkeletonMismatch,
    #[error(transparent)]
    Rig(#[from] RoleContractError),
    #[error(transparent)]
    Ik(#[from] IkError),
    #[error("{0}")]
    Bake(String),
}

/// Receita de movimento: dado serializável (P3D-170).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MotionRecipe {
    pub id: Uuid,
    pub name: String,
    pub skeleton_id: Uuid,
    pub generator: MotionGenerator,
    /// Apenas valores diferentes do padrão precisam existir; ausente = padrão.
    pub params: Vec<MotionParam>,
    pub style: MotionStyle,
    /// Quantização temporal (estética retrô): `pose(t)` segura o último passo.
    pub step_fps: Option<f32>,
    /// Variação determinística (com o parâmetro `variation`).
    pub seed: u32,
    pub root_mode: RootMode,
    #[serde(default)]
    pub revision: u64,
}

impl MotionRecipe {
    pub fn new(skeleton_id: Uuid, generator: MotionGenerator, name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            skeleton_id,
            generator,
            params: Vec::new(),
            style: MotionStyle::Custom,
            step_fps: None,
            seed: 0,
            root_mode: RootMode::InPlace,
            revision: 0,
        }
    }

    fn spec(&self, key: &str) -> Option<&'static ParamSpec> {
        self.generator.specs().iter().find(|s| s.key == key)
    }

    /// Valor efetivo (o guardado, ou o padrão; `0.0` para chave desconhecida).
    pub fn param(&self, key: &str) -> f32 {
        self.params
            .iter()
            .find(|p| p.key == key)
            .map(|p| p.value)
            .or_else(|| self.spec(key).map(|s| s.default))
            .unwrap_or(0.0)
    }

    /// Define um parâmetro (limitado à faixa; escolhas viram inteiros).
    /// `Ok(true)` se o valor efetivo mudou.
    pub fn set_param(&mut self, key: &str, value: f32) -> Result<bool, MotionError> {
        let spec = self
            .spec(key)
            .ok_or_else(|| MotionError::UnknownParam(key.to_string()))?;
        if !value.is_finite() {
            return Err(MotionError::NotFinite(key.to_string()));
        }
        let mut v = value.clamp(spec.min, spec.max);
        if !spec.choices.is_empty() {
            v = v.round();
        }
        let before = self.param(key);
        match self.params.iter_mut().find(|p| p.key == key) {
            Some(p) => p.value = v,
            None => self.params.push(MotionParam {
                key: key.to_string(),
                value: v,
            }),
        }
        let changed = (self.param(key) - before).abs() > f32::EPSILON;
        if changed {
            self.style = MotionStyle::Custom;
            self.revision += 1;
        }
        Ok(changed)
    }

    /// Aplica um Style (preenche parâmetros). `true` se algo mudou.
    pub fn apply_style(&mut self, style: MotionStyle) -> bool {
        let mut changed = self.style != style;
        for (key, value) in style.overrides() {
            if self.spec(key).is_some()
                && let Ok(c) = self.set_param(key, *value)
            {
                changed |= c;
            }
        }
        self.style = style;
        if changed {
            self.revision += 1;
        }
        changed
    }
}

/// Avalia uma receita sobre um rig: `pose_at(t)` é pura e determinística.
pub struct MotionEvaluator {
    prepared: Prepared,
    step_fps: Option<f32>,
    root_motion: bool,
    cycle: f32,
}

impl MotionEvaluator {
    /// Valida o contrato de rig e pré-calcula o plano (pernas, cadeias, eixos).
    pub fn new(
        recipe: &MotionRecipe,
        skeleton: &Skeleton,
        roles: &RigRoleMap,
    ) -> Result<Self, MotionError> {
        if skeleton.id != recipe.skeleton_id || roles.skeleton_id != skeleton.id {
            return Err(MotionError::SkeletonMismatch);
        }
        recipe.generator.check_rig(roles)?;
        let prepared = Prepared::new(recipe, skeleton, roles)?;
        let cycle = prepared.cycle_seconds();
        Ok(Self {
            prepared,
            step_fps: recipe.step_fps.filter(|f| f.is_finite() && *f > 0.0),
            root_motion: recipe.root_mode == RootMode::RootMotion
                && recipe.generator.supports_root_motion(),
            cycle,
        })
    }

    /// Duração de um ciclo em segundos.
    pub fn cycle_seconds(&self) -> f32 {
        self.cycle
    }

    pub fn root_motion(&self) -> bool {
        self.root_motion
    }

    /// Pose local de todos os ossos (ordem de `skeleton.bones`) no instante `t`.
    pub fn pose_at(&self, t: f32) -> Vec<Transform3D> {
        let t = if t.is_finite() { t } else { 0.0 };
        let t = match self.step_fps {
            Some(fps) => ((t * fps) + 1e-4).floor() / fps,
            None => t,
        };
        self.prepared.pose_at(t, self.root_motion)
    }
}

// ---------------------------------------------------------------------------
// Apply Now: bake com redução de keys
// ---------------------------------------------------------------------------

/// Tolerâncias da redução de keys (por canal).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BakeTolerance {
    pub translation: f32,
    pub rotation_deg: f32,
    pub scale: f32,
}

impl Default for BakeTolerance {
    fn default() -> Self {
        Self {
            translation: 1e-3,
            rotation_deg: 0.25,
            scale: 1e-3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BakeOptions {
    /// Frames por segundo de amostragem (1..=240).
    pub fps: f32,
    /// Ciclos no clipe (1..=8).
    pub cycles: u8,
    pub tolerance: BakeTolerance,
}

impl Default for BakeOptions {
    fn default() -> Self {
        Self {
            fps: 30.0,
            cycles: 1,
            tolerance: BakeTolerance::default(),
        }
    }
}

/// Clipe baked e relatório do que mudou (nunca silencioso, P3D-160).
#[derive(Clone, Debug, PartialEq)]
pub struct BakeResult {
    pub clip: AnimationClip,
    /// Frames amostrados (incluindo o final).
    pub frames: usize,
    /// Chaves antes e depois da redução.
    pub keys_before: usize,
    pub keys_after: usize,
    /// Ossos com pelo menos um canal animado.
    pub animated_bones: usize,
}

fn quat(v: [f32; 4]) -> Quat {
    Quat::from_array(v).normalize()
}

/// Índices mantidos por Ramer–Douglas–Peucker; `err(a, b, j)` mede o desvio do
/// ponto `j` em relação à interpolação entre as chaves `a` e `b`.
fn reduce(n: usize, tol: f32, err: &dyn Fn(usize, usize, usize) -> f32) -> Vec<usize> {
    if n <= 2 {
        return (0..n).collect();
    }
    let mut keep = vec![false; n];
    keep[0] = true;
    keep[n - 1] = true;
    let mut stack = vec![(0usize, n - 1)];
    while let Some((a, b)) = stack.pop() {
        if b <= a + 1 {
            continue;
        }
        let (mut worst, mut idx) = (0.0f32, a);
        for j in a + 1..b {
            let e = err(a, b, j);
            if e > worst {
                worst = e;
                idx = j;
            }
        }
        if worst > tol {
            keep[idx] = true;
            stack.push((a, idx));
            stack.push((idx, b));
        }
    }
    (0..n).filter(|&i| keep[i]).collect()
}

/// Converte a receita em `BoneTrack`s editáveis: amostra um número inteiro de
/// ciclos e reduz as chaves por tolerância. Canais que ficam no repouso não
/// geram chaves (o clipe herda o repouso, AN-16).
pub fn bake_motion(
    recipe: &MotionRecipe,
    skeleton: &Skeleton,
    roles: &RigRoleMap,
    options: &BakeOptions,
) -> Result<BakeResult, MotionError> {
    if !options.fps.is_finite() || !(1.0..=240.0).contains(&options.fps) {
        return Err(MotionError::InvalidFps);
    }
    let ev = MotionEvaluator::new(recipe, skeleton, roles)?;
    let cycles = f32::from(options.cycles.clamp(1, 8));
    let duration = ev.cycle_seconds() * cycles;
    let n = ((duration * options.fps).round() as usize).max(2);
    if n > 20_000 {
        return Err(MotionError::Bake(
            "clipe longo demais para o fps pedido".into(),
        ));
    }
    let times: Vec<f32> = (0..=n).map(|i| duration * i as f32 / n as f32).collect();
    let samples: Vec<Vec<Transform3D>> = times.iter().map(|&t| ev.pose_at(t)).collect();

    let tol_r = options.tolerance.rotation_deg.max(0.0).to_radians();
    let (tol_t, tol_s) = (options.tolerance.translation, options.tolerance.scale);
    let mut clip = AnimationClip::new(recipe.name.clone(), duration);
    clip.fps = n as f32 / duration;
    clip.looping = !ev.root_motion();
    let (mut before, mut after, mut animated) = (0usize, 0usize, 0usize);

    for (bi, bone) in skeleton.bones.iter().enumerate() {
        let rest = bone.local_transform;
        let mut track = BoneTrack::new(bone.id, bone.name.clone());

        // Translação
        let tr: Vec<Vec3> = samples
            .iter()
            .map(|s| Vec3::from(s[bi].translation))
            .collect();
        if tr
            .iter()
            .any(|v| (*v - Vec3::from(rest.translation)).length() > 1e-6)
        {
            before += tr.len();
            let kept = reduce(tr.len(), tol_t, &|a, b, j| {
                let u = (times[j] - times[a]) / (times[b] - times[a]);
                (tr[a].lerp(tr[b], u) - tr[j]).length()
            });
            after += kept.len();
            track.translations = kept
                .into_iter()
                .map(|i| Keyframe::with_interp(times[i], tr[i].to_array(), Interpolation::Linear))
                .collect();
        }
        // Rotação (continuidade de sinal do quaternion)
        let mut ro: Vec<Quat> = samples.iter().map(|s| quat(s[bi].rotation)).collect();
        for i in 1..ro.len() {
            if ro[i - 1].dot(ro[i]) < 0.0 {
                ro[i] = -ro[i];
            }
        }
        let rest_q = quat(rest.rotation);
        if ro.iter().any(|q| q.dot(rest_q).abs() < 1.0 - 1e-8) {
            before += ro.len();
            let kept = reduce(ro.len(), tol_r, &|a, b, j| {
                let u = (times[j] - times[a]) / (times[b] - times[a]);
                ro[a].slerp(ro[b], u).angle_between(ro[j])
            });
            after += kept.len();
            track.rotations = kept
                .into_iter()
                .map(|i| Keyframe::with_interp(times[i], ro[i].to_array(), Interpolation::Linear))
                .collect();
        }
        // Escala
        let sc: Vec<Vec3> = samples.iter().map(|s| Vec3::from(s[bi].scale)).collect();
        if sc
            .iter()
            .any(|v| (*v - Vec3::from(rest.scale)).length() > 1e-6)
        {
            before += sc.len();
            let kept = reduce(sc.len(), tol_s, &|a, b, j| {
                let u = (times[j] - times[a]) / (times[b] - times[a]);
                (sc[a].lerp(sc[b], u) - sc[j]).length()
            });
            after += kept.len();
            track.scales = kept
                .into_iter()
                .map(|i| Keyframe::with_interp(times[i], sc[i].to_array(), Interpolation::Linear))
                .collect();
        }

        if !(track.translations.is_empty() && track.rotations.is_empty() && track.scales.is_empty())
        {
            animated += 1;
            clip.tracks.push(track);
        }
    }

    Ok(BakeResult {
        clip,
        frames: n + 1,
        keys_before: before,
        keys_after: after,
        animated_bones: animated,
    })
}
