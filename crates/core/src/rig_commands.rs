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
    AnimationAsset, AnimationClip, IkChain, IkSolver, Interpolation, Keyframe, RigPreset, RigRole,
    Transform3D, auto_fit_humanoid, compute_auto_skin_weights,
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
        let skeleton = match self.kind {
            RigPresetKind::Humanoid => RigPreset::humanoid(self.scale),
            RigPresetKind::Quadruped => RigPreset::quadruped(self.scale),
            RigPresetKind::MultiLeg(n) => RigPreset::multi_leg(usize::from(n), self.scale),
        };
        state.project.project.add_skeleton(skeleton);
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
