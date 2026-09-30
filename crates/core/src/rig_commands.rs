//! Comandos transacionais de rig, papéis, IK e clipes (P3D-169, cap. 45 F0).
//!
//! Seguem o contrato do `CommandDispatcher`: `can_execute` sem efeitos
//! colaterais, `execute` só muta o `Project`, e um comando que não altera nada
//! devolve `CommandError::NoChange` (sem histórico). O dispatcher grava o
//! checkpoint de Undo; nenhum comando aqui chama `checkpoint`.
//!
//! `changes()` é `NONE`: rig, papéis e clipes não invalidam caches de GPU (o
//! renderer ainda não consome skinning). Quando consumir, um domínio próprio
//! entra em `ProjectChanges`.
//!
//! Registro no catálogo (`canonical()`): comandos parametrizados seguem o mesmo
//! caminho dos comandos de Spline (tipados, via `AppState::dispatch`); expô-los
//! à palette/MCP depende da decisão D-21 (catálogo de comandos parametrizados).

use crate::command::{Command, CommandError};
use crate::state::AppState;
use petunia_project::{
    AnimationAsset, AnimationClip, BakeOptions, IkChain, IkSolver, Interpolation, Keyframe,
    MotionGenerator, MotionRecipe, MotionStyle, RigPreset, RigRole, RootMode, Transform3D,
    auto_fit_humanoid, compute_auto_skin_weights,
};
use uuid::Uuid;

fn exec_err(e: impl std::fmt::Display) -> CommandError {
    CommandError::Execution(e.to_string())
}

fn finite3(v: &[f32; 3]) -> bool {
    v.iter().all(|x| x.is_finite())
}

/// Preset de esqueleto canônico (P3D-136).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RigPresetKind {
    Humanoid,
    Quadruped,
    /// Aranha/escorpião/inseto com este número de pernas (4–16).
    MultiLeg(u8),
    /// Cadeia longa de coluna e cauda, sem pernas (12 segmentos).
    Serpent,
    Fish,
    Bird,
}

impl RigPresetKind {
    /// Presets oferecidos ao usuário, na ordem do seletor de criaturas.
    pub const CREATURES: [RigPresetKind; 6] = [
        RigPresetKind::Humanoid,
        RigPresetKind::Quadruped,
        RigPresetKind::MultiLeg(8),
        RigPresetKind::Serpent,
        RigPresetKind::Fish,
        RigPresetKind::Bird,
    ];

    /// Identificador estável (chave de `TextId`).
    pub fn id(self) -> &'static str {
        match self {
            RigPresetKind::Humanoid => "humanoid",
            RigPresetKind::Quadruped => "quadruped",
            RigPresetKind::MultiLeg(_) => "multi_leg",
            RigPresetKind::Serpent => "serpent",
            RigPresetKind::Fish => "fish",
            RigPresetKind::Bird => "bird",
        }
    }

    /// Constrói o esqueleto do preset.
    pub fn build(self, scale: f32) -> petunia_project::Skeleton {
        match self {
            RigPresetKind::Humanoid => RigPreset::humanoid(scale),
            RigPresetKind::Quadruped => RigPreset::quadruped(scale),
            RigPresetKind::MultiLeg(n) => RigPreset::multi_leg(usize::from(n), scale),
            RigPresetKind::Serpent => RigPreset::serpent(12, scale),
            RigPresetKind::Fish => RigPreset::fish(scale),
            RigPresetKind::Bird => RigPreset::bird(scale),
        }
    }
}

/// Adiciona um esqueleto de preset ao projeto; os papéis são inferidos.
#[derive(Debug, Clone)]
pub struct AddRigPresetCmd {
    pub kind: RigPresetKind,
    pub scale: f32,
}

impl Command for AddRigPresetCmd {
    fn label(&self) -> &'static str {
        "add rig preset"
    }
    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }
    fn can_execute(&self, _state: &AppState) -> Result<(), &'static str> {
        if !self.scale.is_finite() || self.scale <= 0.0 {
            return Err("Rig scale must be a positive number");
        }
        Ok(())
    }
    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        state
            .project
            .project
            .add_skeleton(self.kind.build(self.scale));
        state.set_status("Rig added");
        Ok(())
    }
}

/// Ajusta um esqueleto humanoide à malha do asset ativo e calcula os pesos de
/// skin (Auto-Rig heurístico, P3D-137). O resultado é editável.
#[derive(Debug, Clone, Default)]
pub struct AutoRigActiveAssetCmd;

impl Command for AutoRigActiveAssetCmd {
    fn label(&self) -> &'static str {
        "auto rig"
    }
    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }
    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        match state.project.active_mesh() {
            None => Err("No active mesh"),
            Some(m) if m.verts.is_empty() => Err("The active mesh has no points"),
            Some(_) => Ok(()),
        }
    }
    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let mesh = state
            .project
            .active_mesh()
            .cloned()
            .ok_or(CommandError::NoActiveAsset)?;
        let skeleton = auto_fit_humanoid(&mesh);
        let skin = compute_auto_skin_weights(&mesh, &skeleton);
        skin.validate(mesh.verts.len(), skeleton.bones.len())
            .map_err(exec_err)?;
        let skeleton_id = skeleton.id;
        state.project.project.add_skeleton(skeleton);
        let asset = state
            .project
            .active_mut()
            .ok_or(CommandError::NoActiveAsset)?;
        asset.skeleton_id = Some(skeleton_id);
        asset.skin_data = Some(skin);
        state.set_status("Auto-rig applied");
        Ok(())
    }
}

/// Remove um esqueleto (e os papéis, cadeias de IK e vínculos de skin dele).
#[derive(Debug, Clone)]
pub struct RemoveSkeletonCmd {
    pub skeleton_id: Uuid,
}

impl Command for RemoveSkeletonCmd {
    fn label(&self) -> &'static str {
        "remove rig"
    }
    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }
    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        state
            .project
            .project
            .get_skeleton(self.skeleton_id)
            .map(|_| ())
            .ok_or("Rig not found")
    }
    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        state.project.project.remove_skeleton(self.skeleton_id);
        state.set_status("Rig removed");
        Ok(())
    }
}

/// Atribui um papel a um osso.
#[derive(Debug, Clone)]
pub struct AssignRigRoleCmd {
    pub skeleton_id: Uuid,
    pub bone_id: u32,
    pub role: RigRole,
}

impl Command for AssignRigRoleCmd {
    fn label(&self) -> &'static str {
        "assign rig role"
    }
    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }
    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let skeleton = state
            .project
            .project
            .get_skeleton(self.skeleton_id)
            .ok_or("Rig not found")?;
        if skeleton.get_bone(self.bone_id).is_none() {
            return Err("Bone not found");
        }
        if let Some(holder) = state
            .project
            .project
            .rig_roles_of(self.skeleton_id)
            .and_then(|m| m.bone_of(self.role))
            && holder != self.bone_id
        {
            return Err("That role already belongs to another bone");
        }
        Ok(())
    }
    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        match state
            .project
            .project
            .assign_rig_role(self.skeleton_id, self.bone_id, self.role)
        {
            Ok(true) => Ok(()),
            Ok(false) => Err(CommandError::NoChange("role already assigned".into())),
            Err(e) => Err(exec_err(e)),
        }
    }
}

/// Remove o papel de um osso.
#[derive(Debug, Clone)]
pub struct ClearRigRoleCmd {
    pub skeleton_id: Uuid,
    pub bone_id: u32,
}

impl Command for ClearRigRoleCmd {
    fn label(&self) -> &'static str {
        "clear rig role"
    }
    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }
    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        state
            .project
            .project
            .rig_roles_of(self.skeleton_id)
            .and_then(|m| m.role_of(self.bone_id))
            .map(|_| ())
            .ok_or("The bone has no role")
    }
    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        if state
            .project
            .project
            .clear_rig_role(self.skeleton_id, self.bone_id)
        {
            Ok(())
        } else {
            Err(CommandError::NoChange("bone has no role".into()))
        }
    }
}

/// Redetecta os papéis pelos nomes dos ossos (descarta correções manuais).
#[derive(Debug, Clone)]
pub struct InferRigRolesCmd {
    pub skeleton_id: Uuid,
}

impl Command for InferRigRolesCmd {
    fn label(&self) -> &'static str {
        "detect rig roles"
    }
    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }
    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        state
            .project
            .project
            .get_skeleton(self.skeleton_id)
            .map(|_| ())
            .ok_or("Rig not found")
    }
    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        if state.project.project.infer_rig_roles(self.skeleton_id) {
            state.set_status("Rig roles detected");
            Ok(())
        } else {
            Err(CommandError::NoChange(
                "roles already match the names".into(),
            ))
        }
    }
}

fn check_chain(state: &AppState, chain: &IkChain) -> Result<(), &'static str> {
    let skeleton = state
        .project
        .project
        .get_skeleton(chain.skeleton_id)
        .ok_or("Rig not found")?;
    if !chain.weight.is_finite() || !(0.0..=1.0).contains(&chain.weight) {
        return Err("IK weight must be between 0 and 1");
    }
    if chain.pole.as_ref().is_some_and(|p| !finite3(p)) {
        return Err("IK pole is not finite");
    }
    match chain.solver {
        IkSolver::TwoBone { soft } if !(0.0..=0.5).contains(&soft) => {
            return Err("Soft IK must be between 0 and 0.5");
        }
        IkSolver::Fabrik { tolerance, .. } if !tolerance.is_finite() || tolerance <= 0.0 => {
            return Err("IK tolerance must be positive");
        }
        _ => {}
    }
    chain
        .validate(skeleton)
        .map_err(|_| "IK chain needs existing, connected bones for its solver")
}

/// Adiciona uma cadeia de IK.
#[derive(Debug, Clone)]
pub struct AddIkChainCmd {
    pub chain: IkChain,
}

impl Command for AddIkChainCmd {
    fn label(&self) -> &'static str {
        "add IK chain"
    }
    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }
    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state
            .project
            .project
            .ik_chains
            .iter()
            .any(|c| c.id == self.chain.id)
        {
            return Err("IK chain id already exists");
        }
        check_chain(state, &self.chain)
    }
    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        state.project.project.ik_chains.push(self.chain.clone());
        state.set_status("IK chain added");
        Ok(())
    }
}

/// Remove uma cadeia de IK.
#[derive(Debug, Clone)]
pub struct RemoveIkChainCmd {
    pub chain_id: Uuid,
}

impl Command for RemoveIkChainCmd {
    fn label(&self) -> &'static str {
        "remove IK chain"
    }
    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }
    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        state
            .project
            .project
            .ik_chains
            .iter()
            .any(|c| c.id == self.chain_id)
            .then_some(())
            .ok_or("IK chain not found")
    }
    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        state
            .project
            .project
            .ik_chains
            .retain(|c| c.id != self.chain_id);
        state.set_status("IK chain removed");
        Ok(())
    }
}

/// Edita solver, pole e peso de uma cadeia de IK. Campos `None` não mudam;
/// `pole: Some(None)` remove o pole.
#[derive(Debug, Clone, Default)]
pub struct UpdateIkChainCmd {
    pub chain_id: Uuid,
    pub solver: Option<IkSolver>,
    pub pole: Option<Option<[f32; 3]>>,
    pub weight: Option<f32>,
}

impl UpdateIkChainCmd {
    fn updated(&self, state: &AppState) -> Result<IkChain, &'static str> {
        let mut chain = state
            .project
            .project
            .ik_chains
            .iter()
            .find(|c| c.id == self.chain_id)
            .cloned()
            .ok_or("IK chain not found")?;
        if let Some(s) = self.solver {
            chain.solver = s;
        }
        if let Some(p) = self.pole {
            chain.pole = p;
        }
        if let Some(w) = self.weight {
            chain.weight = w;
        }
        check_chain(state, &chain)?;
        Ok(chain)
    }
}

impl Command for UpdateIkChainCmd {
    fn label(&self) -> &'static str {
        "edit IK chain"
    }
    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }
    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        self.updated(state).map(|_| ())
    }
    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let mut chain = self.updated(state).map_err(exec_err)?;
        let slot = state
            .project
            .project
            .ik_chains
            .iter_mut()
            .find(|c| c.id == self.chain_id)
            .ok_or_else(|| exec_err("IK chain not found"))?;
        chain.revision = slot.revision;
        if *slot == chain {
            return Err(CommandError::NoChange(
                "IK chain already has those values".into(),
            ));
        }
        chain.revision += 1;
        *slot = chain;
        Ok(())
    }
}

/// Cria um clipe vazio na biblioteca de animações do projeto.
#[derive(Debug, Clone)]
pub struct AddAnimationCmd {
    pub name: String,
    pub duration: f32,
    pub fps: f32,
    pub looping: bool,
}

impl Command for AddAnimationCmd {
    fn label(&self) -> &'static str {
        "add animation"
    }
    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }
    fn can_execute(&self, _state: &AppState) -> Result<(), &'static str> {
        if self.name.trim().is_empty() {
            return Err("Animation name is empty");
        }
        if !self.duration.is_finite() || self.duration <= 0.0 {
            return Err("Animation duration must be positive");
        }
        if !self.fps.is_finite() || !(1.0..=240.0).contains(&self.fps) {
            return Err("Animation fps must be between 1 and 240");
        }
        Ok(())
    }
    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let mut clip = AnimationClip::new(self.name.trim(), self.duration);
        clip.fps = self.fps;
        clip.looping = self.looping;
        state
            .project
            .project
            .add_animation(AnimationAsset::new(self.name.trim(), clip));
        state.set_status("Animation added");
        Ok(())
    }
}

/// Remove um clipe.
#[derive(Debug, Clone)]
pub struct RemoveAnimationCmd {
    pub animation_id: Uuid,
}

impl Command for RemoveAnimationCmd {
    fn label(&self) -> &'static str {
        "remove animation"
    }
    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }
    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        state
            .project
            .project
            .get_animation(self.animation_id)
            .map(|_| ())
            .ok_or("Animation not found")
    }
    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        state
            .project
            .project
            .animations
            .retain(|a| a.id != self.animation_id);
        state.set_status("Animation removed");
        Ok(())
    }
}

const KEY_TIME_EPS: f32 = 1e-4;

/// Grava (ou substitui) a pose local de um osso num instante do clipe: chaves de
/// translação, rotação e escala (valores locais absolutos, AN-16).
#[derive(Debug, Clone)]
pub struct SetBoneKeyCmd {
    pub animation_id: Uuid,
    pub bone_id: u32,
    pub time: f32,
    pub transform: Transform3D,
    pub interpolation: Interpolation,
}

impl SetBoneKeyCmd {
    fn bone_name(&self, state: &AppState) -> Option<String> {
        state
            .project
            .project
            .skeletons
            .iter()
            .find_map(|s| s.get_bone(self.bone_id))
            .map(|b| b.name.clone())
    }
}

/// Insere ou substitui a chave em `time` (tolerância `KEY_TIME_EPS`), mantendo
/// as chaves ordenadas por tempo.
fn upsert_key<T: Copy>(keys: &mut Vec<Keyframe<T>>, time: f32, value: T, mode: Interpolation) {
    match keys
        .iter()
        .position(|k| (k.time - time).abs() < KEY_TIME_EPS)
    {
        Some(i) => keys[i] = Keyframe::with_interp(keys[i].time, value, mode),
        None => {
            keys.push(Keyframe::with_interp(time, value, mode));
            keys.sort_by(|a, b| a.time.total_cmp(&b.time));
        }
    }
}

impl Command for SetBoneKeyCmd {
    fn label(&self) -> &'static str {
        "set bone key"
    }
    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }
    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let anim = state
            .project
            .project
            .get_animation(self.animation_id)
            .ok_or("Animation not found")?;
        if self.bone_name(state).is_none() {
            return Err("Bone not found in any rig");
        }
        let t = &self.transform;
        if !self.time.is_finite()
            || !finite3(&t.translation)
            || !finite3(&t.scale)
            || !t.rotation.iter().all(|x| x.is_finite())
        {
            return Err("Key values are not finite");
        }
        if self.time < 0.0 || self.time > anim.clip.duration + KEY_TIME_EPS {
            return Err("Key time is outside the animation");
        }
        let q = t.rotation;
        if q.iter().map(|x| x * x).sum::<f32>() < 1e-8 {
            return Err("Key rotation is zero");
        }
        Ok(())
    }
    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let name = self
            .bone_name(state)
            .ok_or_else(|| exec_err("Bone not found"))?;
        let anim = state
            .project
            .project
            .animations
            .iter_mut()
            .find(|a| a.id == self.animation_id)
            .ok_or_else(|| exec_err("Animation not found"))?;
        let before = anim.clip.get_track(self.bone_id).cloned();
        let time = self.time.clamp(0.0, anim.clip.duration);
        let r = self.transform.rotation;
        let len = r.iter().map(|x| x * x).sum::<f32>().sqrt();
        let rot = [r[0] / len, r[1] / len, r[2] / len, r[3] / len];
        let track = anim.clip.get_or_create_track(self.bone_id, &name);
        upsert_key(
            &mut track.translations,
            time,
            self.transform.translation,
            self.interpolation,
        );
        upsert_key(&mut track.rotations, time, rot, self.interpolation);
        upsert_key(
            &mut track.scales,
            time,
            self.transform.scale,
            self.interpolation,
        );
        if before.as_ref() == anim.clip.get_track(self.bone_id) {
            return Err(CommandError::NoChange(
                "key already has those values".into(),
            ));
        }
        state.set_status("Key set");
        Ok(())
    }
}

/// Remove as chaves de um osso num instante do clipe (todos os canais).
#[derive(Debug, Clone)]
pub struct DeleteBoneKeyCmd {
    pub animation_id: Uuid,
    pub bone_id: u32,
    pub time: f32,
}

impl Command for DeleteBoneKeyCmd {
    fn label(&self) -> &'static str {
        "delete bone key"
    }
    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }
    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let anim = state
            .project
            .project
            .get_animation(self.animation_id)
            .ok_or("Animation not found")?;
        let track = anim
            .clip
            .get_track(self.bone_id)
            .ok_or("The bone has no keys")?;
        let has = |t: f32| (t - self.time).abs() < KEY_TIME_EPS;
        if track.translations.iter().any(|k| has(k.time))
            || track.rotations.iter().any(|k| has(k.time))
            || track.scales.iter().any(|k| has(k.time))
        {
            Ok(())
        } else {
            Err("There is no key at that time")
        }
    }
    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let anim = state
            .project
            .project
            .animations
            .iter_mut()
            .find(|a| a.id == self.animation_id)
            .ok_or_else(|| exec_err("Animation not found"))?;
        let track = anim
            .clip
            .get_track_mut(self.bone_id)
            .ok_or_else(|| exec_err("The bone has no keys"))?;
        let t = self.time;
        let before = track.translations.len() + track.rotations.len() + track.scales.len();
        track
            .translations
            .retain(|k| (k.time - t).abs() >= KEY_TIME_EPS);
        track
            .rotations
            .retain(|k| (k.time - t).abs() >= KEY_TIME_EPS);
        track.scales.retain(|k| (k.time - t).abs() >= KEY_TIME_EPS);
        let after = track.translations.len() + track.rotations.len() + track.scales.len();
        if after == before {
            return Err(CommandError::NoChange("no key at that time".into()));
        }
        if after == 0 {
            let id = self.bone_id;
            anim.clip.tracks.retain(|t| t.bone_id != id);
        }
        state.set_status("Key deleted");
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Movimento procedural (P3D-170)
// ---------------------------------------------------------------------------

fn motion_exists(state: &AppState, id: Uuid) -> Result<(), &'static str> {
    state
        .project
        .project
        .get_motion(id)
        .map(|_| ())
        .ok_or("Motion not found")
}

fn motion_mut(state: &mut AppState, id: Uuid) -> Result<&mut MotionRecipe, CommandError> {
    state
        .project
        .project
        .motions
        .iter_mut()
        .find(|m| m.id == id)
        .ok_or_else(|| exec_err("Motion not found"))
}

/// Cria um Motion para o esqueleto. O contrato de rig (papéis) é verificado na
/// execução: o erro diz o que falta no rig e nada é criado.
#[derive(Debug, Clone)]
pub struct AddMotionCmd {
    pub skeleton_id: Uuid,
    pub generator: MotionGenerator,
    /// `None` usa o nome do gerador.
    pub name: Option<String>,
    pub style: MotionStyle,
}

impl Command for AddMotionCmd {
    fn label(&self) -> &'static str {
        "add motion"
    }
    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }
    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        state
            .project
            .project
            .get_skeleton(self.skeleton_id)
            .map(|_| ())
            .ok_or("Rig not found")
    }
    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let roles = state
            .project
            .project
            .rig_roles_of(self.skeleton_id)
            .cloned()
            .unwrap_or_else(|| petunia_project::RigRoleMap::new(self.skeleton_id));
        self.generator.check_rig(&roles).map_err(exec_err)?;
        let name = self
            .name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .unwrap_or(self.generator.id());
        let mut recipe = MotionRecipe::new(self.skeleton_id, self.generator, name);
        recipe.apply_style(self.style);
        state.project.project.motions.push(recipe);
        state.set_status("Motion added");
        Ok(())
    }
}

/// Ajusta um parâmetro do Motion (limitado à faixa do parâmetro).
#[derive(Debug, Clone)]
pub struct SetMotionParamCmd {
    pub motion_id: Uuid,
    pub key: String,
    pub value: f32,
}

impl Command for SetMotionParamCmd {
    fn label(&self) -> &'static str {
        "set motion parameter"
    }
    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }
    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        motion_exists(state, self.motion_id)?;
        if !self.value.is_finite() {
            return Err("Parameter value is not finite");
        }
        Ok(())
    }
    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let recipe = motion_mut(state, self.motion_id)?;
        if recipe.set_param(&self.key, self.value).map_err(exec_err)? {
            Ok(())
        } else {
            Err(CommandError::NoChange(
                "parameter already has that value".into(),
            ))
        }
    }
}

/// Aplica um Style (preenche parâmetros).
#[derive(Debug, Clone)]
pub struct SetMotionStyleCmd {
    pub motion_id: Uuid,
    pub style: MotionStyle,
}

impl Command for SetMotionStyleCmd {
    fn label(&self) -> &'static str {
        "set motion style"
    }
    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }
    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        motion_exists(state, self.motion_id)
    }
    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        if motion_mut(state, self.motion_id)?.apply_style(self.style) {
            Ok(())
        } else {
            Err(CommandError::NoChange(
                "motion already has that style".into(),
            ))
        }
    }
}

/// Edita nome, seed, modo de raiz e Stepped. Campos `None` não mudam;
/// `step_fps: Some(None)` desliga o Stepped.
#[derive(Debug, Clone, Default)]
pub struct UpdateMotionCmd {
    pub motion_id: Uuid,
    pub name: Option<String>,
    pub seed: Option<u32>,
    pub root_mode: Option<RootMode>,
    pub step_fps: Option<Option<f32>>,
}

impl Command for UpdateMotionCmd {
    fn label(&self) -> &'static str {
        "edit motion"
    }
    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }
    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        motion_exists(state, self.motion_id)?;
        if self.name.as_deref().is_some_and(|n| n.trim().is_empty()) {
            return Err("Motion name is empty");
        }
        if let Some(Some(fps)) = self.step_fps
            && (!fps.is_finite() || !(1.0..=60.0).contains(&fps))
        {
            return Err("Stepped fps must be between 1 and 60");
        }
        if self.root_mode == Some(RootMode::RootMotion) {
            let gen_ok = state
                .project
                .project
                .get_motion(self.motion_id)
                .is_some_and(|m| m.generator.supports_root_motion());
            if !gen_ok {
                return Err("This motion does not support root motion");
            }
        }
        Ok(())
    }
    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let recipe = motion_mut(state, self.motion_id)?;
        let before = recipe.clone();
        if let Some(n) = &self.name {
            recipe.name = n.trim().to_string();
        }
        if let Some(seed) = self.seed {
            recipe.seed = seed;
        }
        if let Some(mode) = self.root_mode {
            recipe.root_mode = mode;
        }
        if let Some(step) = self.step_fps {
            recipe.step_fps = step;
        }
        if *recipe == before {
            return Err(CommandError::NoChange(
                "motion already has those values".into(),
            ));
        }
        recipe.revision += 1;
        Ok(())
    }
}

/// Remove o Motion.
#[derive(Debug, Clone)]
pub struct RemoveMotionCmd {
    pub motion_id: Uuid,
}

impl Command for RemoveMotionCmd {
    fn label(&self) -> &'static str {
        "remove motion"
    }
    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }
    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        motion_exists(state, self.motion_id)
    }
    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        state
            .project
            .project
            .motions
            .retain(|m| m.id != self.motion_id);
        state.set_status("Motion removed");
        Ok(())
    }
}

/// Duplica o Motion (novo id, nome com " copy").
#[derive(Debug, Clone)]
pub struct DuplicateMotionCmd {
    pub motion_id: Uuid,
}

impl Command for DuplicateMotionCmd {
    fn label(&self) -> &'static str {
        "duplicate motion"
    }
    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }
    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        motion_exists(state, self.motion_id)
    }
    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let mut copy = state
            .project
            .project
            .get_motion(self.motion_id)
            .cloned()
            .ok_or_else(|| exec_err("Motion not found"))?;
        copy.id = Uuid::new_v4();
        copy.name = format!("{} copy", copy.name);
        copy.revision = 0;
        state.project.project.motions.push(copy);
        state.set_status("Motion duplicated");
        Ok(())
    }
}

/// **Apply Now** (P3D-160): converte o Motion em um clipe editável na biblioteca.
/// Com `keep_motion = false` o Motion vivo é removido; o resultado é um passo de
/// histórico e a mensagem relata quantas chaves foram geradas.
#[derive(Debug, Clone)]
pub struct ApplyMotionNowCmd {
    pub motion_id: Uuid,
    pub options: BakeOptions,
    pub keep_motion: bool,
}

impl Command for ApplyMotionNowCmd {
    fn label(&self) -> &'static str {
        "apply motion now"
    }
    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }
    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        motion_exists(state, self.motion_id)
    }
    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let result = state
            .project
            .project
            .bake_motion(self.motion_id, &self.options)
            .map_err(exec_err)?;
        let recipe = state
            .project
            .project
            .get_motion(self.motion_id)
            .cloned()
            .ok_or_else(|| exec_err("Motion not found"))?;
        let mut asset = AnimationAsset::new(recipe.name.clone(), result.clip);
        asset.preset = Some(recipe.generator.id().to_string());
        asset.tags = vec!["motion".into(), recipe.generator.id().into()];
        state.project.project.add_animation(asset);
        if !self.keep_motion {
            state
                .project
                .project
                .motions
                .retain(|m| m.id != self.motion_id);
        }
        state.set_status(format!(
            "Apply Now: {} keys (from {} samples) on {} bones",
            result.keys_after, result.keys_before, result.animated_bones
        ));
        Ok(())
    }
}
