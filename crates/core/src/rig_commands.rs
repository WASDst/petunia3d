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
use crate::context::{CommandContext, EditorView};
use petunia_project::{
    AnimationAsset, AnimationClip, BakeOptions, ConnectionKind, IkChain, IkSolver, Interpolation,
    JointId, Keyframe, LiveRigData, MotionGenerator, MotionRecipe, MotionStyle, PartId, RigError,
    RigPreset, RigRole, RootMode, Skeleton, SkinBindMethod, SkinData, StructuralShapeGraph,
    Transform3D, VertexSkinWeight, auto_fit_humanoid, compute_auto_skin_weights,
    compute_blended_skin_weights, compute_skin_weights, default_blend_radius,
    fit_skeleton_to_bounds, mesh_bounds, remap_skin_weights, update_structural_rig_localized,
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
    fn can_execute(&self, _state: &EditorView<'_>) -> Result<(), &'static str> {
        if !self.scale.is_finite() || self.scale <= 0.0 {
            return Err("Rig scale must be a positive number");
        }
        Ok(())
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state
            .document
            .project
            .add_skeleton(self.kind.build(self.scale));
        state.set_status("Rig added");
        Ok(())
    }
}

/// Por que o modelo ativo não pode ser ligado a uma criatura.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitBlocked {
    NoCreature,
    NoModel,
    Locked,
}

/// Verifica, sem efeitos, se dá para ajustar `skeleton_id` ao asset ativo.
pub fn fit_availability(
    project: &petunia_project::Project,
    skeleton_id: Option<Uuid>,
) -> Result<(), FitBlocked> {
    if skeleton_id.is_none_or(|id| project.get_skeleton(id).is_none()) {
        return Err(FitBlocked::NoCreature);
    }
    match project.active() {
        Some(a) if !a.mesh.verts.is_empty() => {
            if a.locked {
                Err(FitBlocked::Locked)
            } else {
                Ok(())
            }
        }
        _ => Err(FitBlocked::NoModel),
    }
}

/// **Fit to model**: leva o rig da criatura ao volume do modelo ativo e o liga à
/// malha por skin (pesos que seguem o osso mais próximo e só misturam perto das
/// articulações). O rig mantém o id, então Motions, papéis e cadeias de IK
/// seguem valendo. Um passo de Undo; refazer com o mesmo resultado não cria
/// histórico. O documento continua guardando a malha em repouso.
#[derive(Debug, Clone)]
pub struct FitRigToActiveAssetCmd {
    pub skeleton_id: Uuid,
}

impl Command for FitRigToActiveAssetCmd {
    fn label(&self) -> &'static str {
        "fit rig to model"
    }
    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        match fit_availability(&state.document.project, Some(self.skeleton_id)) {
            Ok(()) => Ok(()),
            Err(FitBlocked::NoCreature) => Err("Rig not found"),
            Err(FitBlocked::NoModel) => Err("The active model has no points"),
            Err(FitBlocked::Locked) => Err("The active model is locked"),
        }
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let project = &mut state.document.project;
        let mesh = project
            .active()
            .map(|a| a.mesh.clone())
            .ok_or(CommandError::NoActiveAsset)?;
        let (lo, hi) =
            mesh_bounds(&mesh).ok_or_else(|| exec_err("The model has no finite points"))?;
        let original = project
            .get_skeleton(self.skeleton_id)
            .cloned()
            .ok_or_else(|| exec_err("Rig not found"))?;
        let mut skeleton = original.clone();
        if !fit_skeleton_to_bounds(&mut skeleton, lo, hi) {
            return Err(exec_err("The rig has no bones to fit"));
        }
        let skin = compute_blended_skin_weights(&mesh, &skeleton, default_blend_radius(&skeleton));
        let unchanged = skeleton == original
            && project.active().is_some_and(|a| {
                a.skeleton_id == Some(self.skeleton_id) && a.skin_data.as_ref() == Some(&skin)
            });
        if unchanged {
            return Err(CommandError::NoChange("rig already fits the model".into()));
        }
        if let Some(target) = project.get_skeleton_mut(self.skeleton_id) {
            *target = skeleton;
        }
        if let Some(asset) = project.active_mut() {
            asset.skeleton_id = Some(self.skeleton_id);
            asset.skin_data = Some(skin);
        }
        state.set_status("Rig fitted to model");
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
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        match state.document.active_mesh() {
            None => Err("No active mesh"),
            Some(m) if m.verts.is_empty() => Err("The active mesh has no points"),
            Some(_) => Ok(()),
        }
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let mesh = state
            .document
            .active_mesh()
            .cloned()
            .ok_or(CommandError::NoActiveAsset)?;
        let skeleton = auto_fit_humanoid(&mesh);
        let skin = compute_auto_skin_weights(&mesh, &skeleton);
        skin.validate(mesh.verts.len(), skeleton.bones.len())
            .map_err(exec_err)?;
        let skeleton_id = skeleton.id;
        state.document.project.add_skeleton(skeleton);
        let asset = state
            .document
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
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        state
            .document
            .project
            .get_skeleton(self.skeleton_id)
            .map(|_| ())
            .ok_or("Rig not found")
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.document.project.remove_skeleton(self.skeleton_id);
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
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let skeleton = state
            .document
            .project
            .get_skeleton(self.skeleton_id)
            .ok_or("Rig not found")?;
        if skeleton.get_bone(self.bone_id).is_none() {
            return Err("Bone not found");
        }
        if let Some(holder) = state
            .document
            .project
            .rig_roles_of(self.skeleton_id)
            .and_then(|m| m.bone_of(self.role))
            && holder != self.bone_id
        {
            return Err("That role already belongs to another bone");
        }
        Ok(())
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        match state
            .document
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
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        state
            .document
            .project
            .rig_roles_of(self.skeleton_id)
            .and_then(|m| m.role_of(self.bone_id))
            .map(|_| ())
            .ok_or("The bone has no role")
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        if state
            .document
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
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        state
            .document
            .project
            .get_skeleton(self.skeleton_id)
            .map(|_| ())
            .ok_or("Rig not found")
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        if state.document.project.infer_rig_roles(self.skeleton_id) {
            state.set_status("Rig roles detected");
            Ok(())
        } else {
            Err(CommandError::NoChange(
                "roles already match the names".into(),
            ))
        }
    }
}

fn check_chain(project: &petunia_project::Project, chain: &IkChain) -> Result<(), &'static str> {
    let skeleton = project
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
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state
            .document
            .project
            .ik_chains
            .iter()
            .any(|c| c.id == self.chain.id)
        {
            return Err("IK chain id already exists");
        }
        check_chain(&state.document.project, &self.chain)
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.document.project.ik_chains.push(self.chain.clone());
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
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        state
            .document
            .project
            .ik_chains
            .iter()
            .any(|c| c.id == self.chain_id)
            .then_some(())
            .ok_or("IK chain not found")
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state
            .document
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
    fn updated(&self, project: &petunia_project::Project) -> Result<IkChain, &'static str> {
        let mut chain = project
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
        check_chain(project, &chain)?;
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
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        self.updated(&state.document.project).map(|_| ())
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let mut chain = self.updated(&state.document.project).map_err(exec_err)?;
        let slot = state
            .document
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

/// Aplica a pose de uma cadeia IK aos ossos do esqueleto em uma transação atômica de Undo (Task H15).
#[derive(Debug, Clone)]
pub struct ApplyIkPoseCmd {
    pub skeleton_id: Uuid,
    pub chain_id: Uuid,
    pub bone_rotations: Vec<(u32, [f32; 4])>,
}

impl Command for ApplyIkPoseCmd {
    fn label(&self) -> &'static str {
        "apply IK pose"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let skel = state
            .document
            .project
            .get_skeleton(self.skeleton_id)
            .ok_or("Skeleton not found")?;
        if self.bone_rotations.is_empty() {
            return Err("No bone rotations in IK pose");
        }
        for (bone_id, rot) in &self.bone_rotations {
            if skel.get_bone(*bone_id).is_none() {
                return Err("Bone in IK chain not found");
            }
            if !rot.iter().all(|x| x.is_finite()) {
                return Err("Rotation quaternion is not finite");
            }
            if rot.iter().map(|x| x * x).sum::<f32>() < 1e-8 {
                return Err("Rotation quaternion is zero");
            }
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let skel = state
            .document
            .project
            .skeletons
            .iter_mut()
            .find(|s| s.id == self.skeleton_id)
            .ok_or_else(|| exec_err("Skeleton not found"))?;
        for (bone_id, rot) in &self.bone_rotations {
            if let Some(bone) = skel.get_bone_mut(*bone_id) {
                bone.local_transform.rotation = *rot;
            }
        }
        state.set_status("IK pose applied");
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
    fn can_execute(&self, _state: &EditorView<'_>) -> Result<(), &'static str> {
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
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let mut clip = AnimationClip::new(self.name.trim(), self.duration);
        clip.fps = self.fps;
        clip.looping = self.looping;
        state
            .document
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
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        state
            .document
            .project
            .get_animation(self.animation_id)
            .map(|_| ())
            .ok_or("Animation not found")
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state
            .document
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
    fn bone_name(&self, project: &petunia_project::Project) -> Option<String> {
        project
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
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let anim = state
            .document
            .project
            .get_animation(self.animation_id)
            .ok_or("Animation not found")?;
        if self.bone_name(&state.document.project).is_none() {
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
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let name = self
            .bone_name(&state.document.project)
            .ok_or_else(|| exec_err("Bone not found"))?;
        let anim = state
            .document
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
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let anim = state
            .document
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
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let anim = state
            .document
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

fn motion_exists(project: &petunia_project::Project, id: Uuid) -> Result<(), &'static str> {
    project.get_motion(id).map(|_| ()).ok_or("Motion not found")
}

fn motion_mut(
    project: &mut petunia_project::Project,
    id: Uuid,
) -> Result<&mut MotionRecipe, CommandError> {
    project
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
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        state
            .document
            .project
            .get_skeleton(self.skeleton_id)
            .map(|_| ())
            .ok_or("Rig not found")
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let roles = state
            .document
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
        state.document.project.motions.push(recipe);
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
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        motion_exists(&state.document.project, self.motion_id)?;
        if !self.value.is_finite() {
            return Err("Parameter value is not finite");
        }
        Ok(())
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let recipe = motion_mut(&mut state.document.project, self.motion_id)?;
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
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        motion_exists(&state.document.project, self.motion_id)
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        if motion_mut(&mut state.document.project, self.motion_id)?.apply_style(self.style) {
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
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        motion_exists(&state.document.project, self.motion_id)?;
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
                .document
                .project
                .get_motion(self.motion_id)
                .is_some_and(|m| m.generator.supports_root_motion());
            if !gen_ok {
                return Err("This motion does not support root motion");
            }
        }
        Ok(())
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let recipe = motion_mut(&mut state.document.project, self.motion_id)?;
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
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        motion_exists(&state.document.project, self.motion_id)
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state
            .document
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
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        motion_exists(&state.document.project, self.motion_id)
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let mut copy = state
            .document
            .project
            .get_motion(self.motion_id)
            .cloned()
            .ok_or_else(|| exec_err("Motion not found"))?;
        copy.id = Uuid::new_v4();
        copy.name = format!("{} copy", copy.name);
        copy.revision = 0;
        state.document.project.motions.push(copy);
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
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        motion_exists(&state.document.project, self.motion_id)
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let result = state
            .document
            .project
            .bake_motion(self.motion_id, &self.options)
            .map_err(exec_err)?;
        let recipe = state
            .document
            .project
            .get_motion(self.motion_id)
            .cloned()
            .ok_or_else(|| exec_err("Motion not found"))?;
        let mut asset = AnimationAsset::new(recipe.name.clone(), result.clip);
        asset.preset = Some(recipe.generator.id().to_string());
        asset.tags = vec!["motion".into(), recipe.generator.id().into()];
        state.document.project.add_animation(asset);
        if !self.keep_motion {
            state
                .document
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

/// Define a transformação local de um osso no esqueleto com histórico de Undo.
#[derive(Debug, Clone)]
pub struct SetBoneTransformCmd {
    pub skeleton_id: Uuid,
    pub bone_id: u32,
    pub transform: Transform3D,
}

impl Command for SetBoneTransformCmd {
    fn label(&self) -> &'static str {
        "set bone transform"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let skel = state
            .document
            .project
            .get_skeleton(self.skeleton_id)
            .ok_or("Skeleton not found")?;
        if skel.get_bone(self.bone_id).is_none() {
            return Err("Bone not found");
        }
        let t = &self.transform;
        if !finite3(&t.translation)
            || !finite3(&t.scale)
            || !t.rotation.iter().all(|x| x.is_finite())
        {
            return Err("Transform values are not finite");
        }
        let q = t.rotation;
        if q.iter().map(|x| x * x).sum::<f32>() < 1e-8 {
            return Err("Rotation quaternion is zero");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let skel = state
            .document
            .project
            .skeletons
            .iter_mut()
            .find(|s| s.id == self.skeleton_id)
            .ok_or_else(|| exec_err("Skeleton not found"))?;
        let bone = skel
            .get_bone_mut(self.bone_id)
            .ok_or_else(|| exec_err("Bone not found"))?;
        if bone.local_transform == self.transform {
            return Err(CommandError::NoChange(
                "Bone transform already has those values".into(),
            ));
        }
        bone.local_transform = self.transform;
        state.set_status("Bone transform set");
        Ok(())
    }
}

/// Restaura a transformação de um osso para a identidade / rest pose.
#[derive(Debug, Clone)]
pub struct ResetBonePoseCmd {
    pub skeleton_id: Uuid,
    pub bone_id: u32,
}

impl Command for ResetBonePoseCmd {
    fn label(&self) -> &'static str {
        "reset bone pose"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let skel = state
            .document
            .project
            .get_skeleton(self.skeleton_id)
            .ok_or("Skeleton not found")?;
        if skel.get_bone(self.bone_id).is_none() {
            return Err("Bone not found");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let skel = state
            .document
            .project
            .skeletons
            .iter_mut()
            .find(|s| s.id == self.skeleton_id)
            .ok_or_else(|| exec_err("Skeleton not found"))?;
        let bone = skel
            .get_bone_mut(self.bone_id)
            .ok_or_else(|| exec_err("Bone not found"))?;
        let identity_rot = [0.0, 0.0, 0.0, 1.0];
        let identity_scale = [1.0, 1.0, 1.0];
        if bone.local_transform.rotation == identity_rot
            && bone.local_transform.scale == identity_scale
        {
            return Err(CommandError::NoChange(
                "Bone is already in rest pose".into(),
            ));
        }
        bone.local_transform.rotation = identity_rot;
        bone.local_transform.scale = identity_scale;
        state.set_status("Bone reset to rest pose");
        Ok(())
    }
}

/// Shared validation for explicit animation authoring targets.
fn validate_keyframe_target(
    state: &EditorView<'_>,
    skeleton_id: Uuid,
    bone_id: u32,
    clip_id: Option<Uuid>,
) -> Result<(), &'static str> {
    let skeleton = state
        .document
        .project
        .get_skeleton(skeleton_id)
        .ok_or("Skeleton not found")?;
    if skeleton.get_bone(bone_id).is_none() {
        return Err("Bone not found");
    }
    if let Some(id) = clip_id {
        let asset = state
            .document
            .project
            .get_animation(id)
            .ok_or("Animation clip not found")?;
        if !asset.tags.contains(&format!("skeleton:{skeleton_id}")) {
            return Err("Animation clip belongs to a different skeleton");
        }
    }
    Ok(())
}

/// Author into an explicit clip; None creates the first clip only.
#[derive(Debug, Clone)]
pub struct InsertBoneKeyframeCmd {
    pub skeleton_id: Uuid,
    pub clip_id: Option<Uuid>,
    pub bone_id: u32,
    pub time: f32,
    pub transform: Transform3D,
    pub interpolation: Interpolation,
}

impl Command for InsertBoneKeyframeCmd {
    fn label(&self) -> &'static str {
        "insert bone keyframe"
    }
    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        validate_keyframe_target(state, self.skeleton_id, self.bone_id, self.clip_id)?;
        if self.clip_id.is_none()
            && state
                .document
                .project
                .get_animation_for_skeleton(self.skeleton_id)
                .is_some()
        {
            return Err("Select an animation clip explicitly");
        }
        if !self.time.is_finite() || self.time < 0.0 {
            return Err("Invalid keyframe time");
        }
        if !finite3(&self.transform.translation)
            || !finite3(&self.transform.scale)
            || !self.transform.rotation.iter().all(|v| v.is_finite())
            || !glam::Quat::from_array(self.transform.rotation)
                .length_squared()
                .is_finite()
            || glam::Quat::from_array(self.transform.rotation).length_squared() <= f32::EPSILON
        {
            return Err("Invalid keyframe transform");
        }
        Ok(())
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        self.can_execute(&state.view()).map_err(exec_err)?;
        let bone_name = state
            .document
            .project
            .get_skeleton(self.skeleton_id)
            .and_then(|s| s.get_bone(self.bone_id))
            .ok_or_else(|| exec_err("Bone not found"))?
            .name
            .clone();
        let clip = if let Some(id) = self.clip_id {
            &mut state
                .document
                .project
                .get_animation_mut(id)
                .ok_or_else(|| exec_err("Animation clip not found"))?
                .clip
        } else {
            state.document.project.get_or_create_animation_clip(
                self.skeleton_id,
                "Action",
                (self.time + 1.0).max(2.0),
            )
        };
        if self.time > clip.duration {
            clip.duration = (self.time + 0.5).max(clip.duration);
        }
        let track = clip.get_or_create_track(self.bone_id, &bone_name);
        track.add_translation(self.time, self.transform.translation);
        track.add_rotation(self.time, self.transform.rotation);
        track.add_scale(self.time, self.transform.scale);
        track.set_keyframe_interpolation(self.time, self.interpolation, 1e-4);
        state.set_status(format!(
            "Keyframe inserted for {bone_name} at {:.4}s",
            self.time
        ));
        Ok(())
    }
}

/// Delete one authored key time, never every key in the search window.
#[derive(Debug, Clone)]
pub struct DeleteBoneKeyframeCmd {
    pub skeleton_id: Uuid,
    pub clip_id: Uuid,
    pub bone_id: u32,
    pub time: f32,
    pub tolerance: f32,
}

fn nearest_key_time(track: &petunia_project::BoneTrack, time: f32, tolerance: f32) -> Option<f32> {
    track
        .keyframe_times()
        .into_iter()
        .filter(|t| t.is_finite() && (*t - time).abs() <= tolerance)
        .min_by(|a, b| {
            (a - time)
                .abs()
                .total_cmp(&(b - time).abs())
                .then_with(|| a.total_cmp(b))
        })
}

impl Command for DeleteBoneKeyframeCmd {
    fn label(&self) -> &'static str {
        "delete bone keyframe"
    }
    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        validate_keyframe_target(state, self.skeleton_id, self.bone_id, Some(self.clip_id))?;
        if !self.time.is_finite()
            || self.time < 0.0
            || !self.tolerance.is_finite()
            || self.tolerance < 0.0
        {
            return Err("Invalid keyframe time or tolerance");
        }
        Ok(())
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        self.can_execute(&state.view()).map_err(exec_err)?;
        let track = state
            .document
            .project
            .get_animation_mut(self.clip_id)
            .and_then(|a| a.clip.get_track_mut(self.bone_id))
            .ok_or_else(|| CommandError::NoChange("No keyframe track".into()))?;
        let time = nearest_key_time(track, self.time, self.tolerance)
            .ok_or_else(|| CommandError::NoChange("No keyframe found at that time".into()))?;
        track.remove_keyframe_at(time, 0.0);
        state.set_status(format!("Keyframe deleted at {time:.4}s"));
        Ok(())
    }
}

/// Change interpolation only on the explicitly targeted clip and key time.
#[derive(Debug, Clone)]
pub struct SetBoneKeyframeInterpolationCmd {
    pub skeleton_id: Uuid,
    pub clip_id: Uuid,
    pub bone_id: u32,
    pub time: f32,
    pub interpolation: Interpolation,
}
impl Command for SetBoneKeyframeInterpolationCmd {
    fn label(&self) -> &'static str {
        "set bone keyframe interpolation"
    }
    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        validate_keyframe_target(state, self.skeleton_id, self.bone_id, Some(self.clip_id))?;
        if !self.time.is_finite() || self.time < 0.0 {
            return Err("Invalid keyframe time");
        }
        Ok(())
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        self.can_execute(&state.view()).map_err(exec_err)?;
        let track = state
            .document
            .project
            .get_animation_mut(self.clip_id)
            .and_then(|a| a.clip.get_track_mut(self.bone_id))
            .ok_or_else(|| CommandError::NoChange("No keyframe track".into()))?;
        let time = nearest_key_time(track, self.time, 1e-4)
            .ok_or_else(|| CommandError::NoChange("No keyframe found at that time".into()))?;
        track.set_keyframe_interpolation(time, self.interpolation, 0.0);
        state.set_status("Keyframe interpolation updated");
        Ok(())
    }
}

/// Clear a bone track only in the explicit animation clip.
#[derive(Debug, Clone)]
pub struct ClearBoneTrackCmd {
    pub skeleton_id: Uuid,
    pub clip_id: Uuid,
    pub bone_id: u32,
}
impl Command for ClearBoneTrackCmd {
    fn label(&self) -> &'static str {
        "clear bone track"
    }
    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        validate_keyframe_target(state, self.skeleton_id, self.bone_id, Some(self.clip_id))
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        self.can_execute(&state.view()).map_err(exec_err)?;
        let track = state
            .document
            .project
            .get_animation_mut(self.clip_id)
            .and_then(|a| a.clip.get_track_mut(self.bone_id))
            .filter(|t| {
                !t.translations.is_empty() || !t.rotations.is_empty() || !t.scales.is_empty()
            })
            .ok_or_else(|| CommandError::NoChange("Bone track was already empty".into()))?;
        track.clear();
        state.set_status("Bone track cleared");
        Ok(())
    }
}

/// Adiciona uma camada de animação à pilha (Fatia 4).
#[derive(Debug, Clone)]
pub struct AddMotionLayerCmd {
    pub layer: petunia_project::MotionLayer,
}

impl Command for AddMotionLayerCmd {
    fn label(&self) -> &'static str {
        "add motion layer"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, _state: &EditorView<'_>) -> Result<(), &'static str> {
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.document.project.add_motion_layer(self.layer.clone());
        state.set_status("Motion layer added");
        Ok(())
    }
}

/// Remove uma camada de animação da pilha (Fatia 4).
#[derive(Debug, Clone)]
pub struct RemoveMotionLayerCmd {
    pub layer_id: Uuid,
}

impl Command for RemoveMotionLayerCmd {
    fn label(&self) -> &'static str {
        "remove motion layer"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if !state
            .document
            .project
            .motion_layers
            .iter()
            .any(|l| l.id == self.layer_id)
        {
            return Err("Motion layer not found");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        if !state.document.project.remove_motion_layer(self.layer_id) {
            return Err(CommandError::NoChange("Motion layer not found".into()));
        }
        state.set_status("Motion layer removed");
        Ok(())
    }
}

/// Define o peso de blending de uma camada de animação (Fatia 4).
#[derive(Debug, Clone)]
pub struct SetMotionLayerWeightCmd {
    pub layer_id: Uuid,
    pub weight: f32,
}

impl Command for SetMotionLayerWeightCmd {
    fn label(&self) -> &'static str {
        "set motion layer weight"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if !self.weight.is_finite() {
            return Err("Weight must be finite");
        }
        if !state
            .document
            .project
            .motion_layers
            .iter()
            .any(|l| l.id == self.layer_id)
        {
            return Err("Motion layer not found");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let layer = state
            .document
            .project
            .get_motion_layer_mut(self.layer_id)
            .ok_or_else(|| exec_err("Motion layer not found"))?;
        let clamped = self.weight.clamp(0.0, 1.0);
        if (layer.blend_weight - clamped).abs() < 1e-4 {
            return Err(CommandError::NoChange("Weight already set".into()));
        }
        layer.blend_weight = clamped;
        state.set_status("Layer weight updated");
        Ok(())
    }
}

/// Define o modo de blending (Override/Additive) de uma camada de animação (Fatia 4).
#[derive(Debug, Clone)]
pub struct SetMotionLayerBlendModeCmd {
    pub layer_id: Uuid,
    pub blend_mode: petunia_project::BlendMode,
}

impl Command for SetMotionLayerBlendModeCmd {
    fn label(&self) -> &'static str {
        "set motion layer blend mode"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if !state
            .document
            .project
            .motion_layers
            .iter()
            .any(|l| l.id == self.layer_id)
        {
            return Err("Motion layer not found");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let layer = state
            .document
            .project
            .get_motion_layer_mut(self.layer_id)
            .ok_or_else(|| exec_err("Motion layer not found"))?;
        if layer.blend_mode == self.blend_mode {
            return Err(CommandError::NoChange("Blend mode already set".into()));
        }
        layer.blend_mode = self.blend_mode;
        state.set_status("Layer blend mode updated");
        Ok(())
    }
}

/// Alterna a ativação (enable/disable) de uma camada de animação (Fatia 4).
#[derive(Debug, Clone)]
pub struct ToggleMotionLayerEnabledCmd {
    pub layer_id: Uuid,
}

impl Command for ToggleMotionLayerEnabledCmd {
    fn label(&self) -> &'static str {
        "toggle motion layer enabled"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if !state
            .document
            .project
            .motion_layers
            .iter()
            .any(|l| l.id == self.layer_id)
        {
            return Err("Motion layer not found");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let layer = state
            .document
            .project
            .get_motion_layer_mut(self.layer_id)
            .ok_or_else(|| exec_err("Motion layer not found"))?;
        layer.enabled = !layer.enabled;
        state.set_status("Layer enabled toggled");
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// P3D-170 / H31: Smart Rig Manual & Assistido (RIG-001/002/004/008/009/010/011)
// ---------------------------------------------------------------------------

fn find_asset(
    project: &petunia_project::Project,
    id: Option<Uuid>,
) -> Option<&petunia_project::Asset> {
    match id {
        Some(aid) => project.assets.iter().find(|a| a.id == aid),
        None => project.active(),
    }
}

fn find_asset_mut(
    project: &mut petunia_project::Project,
    id: Option<Uuid>,
) -> Option<&mut petunia_project::Asset> {
    match id {
        Some(aid) => project.assets.iter_mut().find(|a| a.id == aid),
        None => project.active_mut(),
    }
}

/// Cria um novo esqueleto vazio e o registra no projeto (RIG-001/010/011).
#[derive(Debug, Clone)]
pub struct CreateEmptySkeletonCmd {
    pub name: String,
    pub asset_id: Option<Uuid>,
}

impl Command for CreateEmptySkeletonCmd {
    fn label(&self) -> &'static str {
        "create empty skeleton"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err("Skeleton name cannot be empty");
        }
        if state
            .document
            .project
            .skeletons
            .iter()
            .any(|s| s.name == name)
        {
            return Err("A skeleton with this name already exists");
        }
        if let Some(aid) = self.asset_id
            && state.document.project.assets.iter().all(|a| a.id != aid)
        {
            return Err("Target asset not found");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let name = self.name.trim();
        let skeleton = Skeleton::new(name);
        let skel_id = skeleton.id;
        state.document.project.add_skeleton(skeleton);

        if let Some(asset) = find_asset_mut(&mut state.document.project, self.asset_id) {
            asset.skeleton_id = Some(skel_id);
        }

        state.set_status(format!("Created skeleton '{name}'"));
        Ok(())
    }
}

/// Adiciona manualmente um osso com posições head e tail ao esqueleto (RIG-001/010/011).
#[derive(Debug, Clone)]
pub struct AddBoneCmd {
    pub skeleton_id: Uuid,
    pub name: String,
    pub parent: Option<u32>,
    pub head: [f32; 3],
    pub tail: [f32; 3],
}

impl Command for AddBoneCmd {
    fn label(&self) -> &'static str {
        "add bone"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let skel = state
            .document
            .project
            .get_skeleton(self.skeleton_id)
            .ok_or("Skeleton not found")?;
        let name = self.name.trim();
        if name.is_empty() {
            return Err("Bone name cannot be empty");
        }
        if skel.bones.iter().any(|b| b.name == name) {
            return Err("A bone with this name already exists in the skeleton");
        }
        if let Some(pid) = self.parent
            && skel.get_bone(pid).is_none()
        {
            return Err("Parent bone not found in skeleton");
        }
        if !finite3(&self.head) || !finite3(&self.tail) {
            return Err("Bone coordinates must be finite numbers");
        }
        if glam::Vec3::from(self.head).distance(glam::Vec3::from(self.tail)) < 1e-6 {
            return Err("Bone length must be greater than zero");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let skel = state
            .document
            .project
            .skeletons
            .iter_mut()
            .find(|s| s.id == self.skeleton_id)
            .ok_or_else(|| exec_err("Skeleton not found"))?;

        let name = self.name.trim();
        let bone_id = skel
            .add_bone(name, self.parent, self.head, self.tail)
            .map_err(exec_err)?;

        state.set_status(format!("Added bone '{name}' (id: {bone_id})"));
        Ok(())
    }
}

/// Remove um osso do esqueleto, reatribuindo os filhos ao pai do osso removido (RIG-001/010).
#[derive(Debug, Clone)]
pub struct RemoveBoneCmd {
    pub skeleton_id: Uuid,
    pub bone_id: u32,
}

impl Command for RemoveBoneCmd {
    fn label(&self) -> &'static str {
        "remove bone"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let skel = state
            .document
            .project
            .get_skeleton(self.skeleton_id)
            .ok_or("Skeleton not found")?;
        if skel.get_bone(self.bone_id).is_none() {
            return Err("Bone not found");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let skel = state
            .document
            .project
            .skeletons
            .iter_mut()
            .find(|s| s.id == self.skeleton_id)
            .ok_or_else(|| exec_err("Skeleton not found"))?;

        skel.remove_bone(self.bone_id).map_err(exec_err)?;
        state.set_status(format!("Removed bone id: {}", self.bone_id));
        Ok(())
    }
}

/// Reatribui a hierarquia de um osso com detecção estrita de ciclos (RIG-001/010).
#[derive(Debug, Clone)]
pub struct ReparentBoneCmd {
    pub skeleton_id: Uuid,
    pub bone_id: u32,
    pub new_parent: Option<u32>,
}

impl Command for ReparentBoneCmd {
    fn label(&self) -> &'static str {
        "reparent bone"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let skel = state
            .document
            .project
            .get_skeleton(self.skeleton_id)
            .ok_or("Skeleton not found")?;
        let bone = skel.get_bone(self.bone_id).ok_or("Bone not found")?;
        if bone.parent == self.new_parent {
            return Err("Bone already has this parent");
        }
        if let Some(target) = self.new_parent {
            if target == self.bone_id {
                return Err("Cannot reparent bone to itself");
            }
            if skel.get_bone(target).is_none() {
                return Err("Target parent bone not found");
            }
            let mut curr = Some(target);
            while let Some(pid) = curr {
                if pid == self.bone_id {
                    return Err(
                        "Hierarchy cycle detected: target parent is a descendant of the bone",
                    );
                }
                curr = skel.get_bone(pid).and_then(|b| b.parent);
            }
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let skel = state
            .document
            .project
            .skeletons
            .iter_mut()
            .find(|s| s.id == self.skeleton_id)
            .ok_or_else(|| exec_err("Skeleton not found"))?;

        skel.reparent_bone(self.bone_id, self.new_parent)
            .map_err(exec_err)?;
        state.set_status(format!("Reparented bone id: {}", self.bone_id));
        Ok(())
    }
}

/// Edita as coordenadas head e tail de um osso no Rest Pose (RIG-001/008).
#[derive(Debug, Clone)]
pub struct EditBoneRestPoseCmd {
    pub skeleton_id: Uuid,
    pub bone_id: u32,
    pub head: [f32; 3],
    pub tail: [f32; 3],
}

impl Command for EditBoneRestPoseCmd {
    fn label(&self) -> &'static str {
        "edit bone rest pose"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let skel = state
            .document
            .project
            .get_skeleton(self.skeleton_id)
            .ok_or("Skeleton not found")?;
        if skel.get_bone(self.bone_id).is_none() {
            return Err("Bone not found");
        }
        if !finite3(&self.head) || !finite3(&self.tail) {
            return Err("Bone coordinates must be finite numbers");
        }
        if glam::Vec3::from(self.head).distance(glam::Vec3::from(self.tail)) < 1e-6 {
            return Err("Bone length must be greater than zero");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let skel = state
            .document
            .project
            .skeletons
            .iter_mut()
            .find(|s| s.id == self.skeleton_id)
            .ok_or_else(|| exec_err("Skeleton not found"))?;

        let bone = skel
            .get_bone(self.bone_id)
            .ok_or_else(|| exec_err("Bone not found"))?;

        if bone.head == self.head && bone.tail == self.tail {
            return Err(CommandError::NoChange(
                "Bone rest pose already has these coordinates".into(),
            ));
        }

        skel.edit_bone_rest_pose(self.bone_id, self.head, self.tail)
            .map_err(exec_err)?;

        state.set_status(format!("Edited rest pose of bone id: {}", self.bone_id));
        Ok(())
    }
}

/// Renomeia um osso existente garantindo unicidade no esqueleto (RIG-001/010).
#[derive(Debug, Clone)]
pub struct RenameBoneCmd {
    pub skeleton_id: Uuid,
    pub bone_id: u32,
    pub new_name: String,
}

impl Command for RenameBoneCmd {
    fn label(&self) -> &'static str {
        "rename bone"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let skel = state
            .document
            .project
            .get_skeleton(self.skeleton_id)
            .ok_or("Skeleton not found")?;
        let bone = skel.get_bone(self.bone_id).ok_or("Bone not found")?;
        let name = self.new_name.trim();
        if name.is_empty() {
            return Err("Bone name cannot be empty");
        }
        if skel
            .bones
            .iter()
            .any(|b| b.id != self.bone_id && b.name == name)
        {
            return Err("A bone with this name already exists in the skeleton");
        }
        if bone.name == name {
            return Err("Bone already has this name");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let skel = state
            .document
            .project
            .skeletons
            .iter_mut()
            .find(|s| s.id == self.skeleton_id)
            .ok_or_else(|| exec_err("Skeleton not found"))?;

        let name = self.new_name.trim();
        skel.rename_bone(self.bone_id, name).map_err(exec_err)?;
        state.set_status(format!("Renamed bone to '{name}'"));
        Ok(())
    }
}

/// Vincula uma malha a um esqueleto usando o método configurado (RIG-002/004/009).
/// Suporta Proximity, RigidPerPart e SmoothEnvelope, com limite de influências e normalização.
#[derive(Debug, Clone)]
pub struct BindRigCmd {
    pub skeleton_id: Uuid,
    pub asset_id: Option<Uuid>,
    pub method: SkinBindMethod,
    pub max_influences: usize,
    pub custom_skin: Option<SkinData>,
}

impl BindRigCmd {
    pub fn new(
        skeleton_id: Uuid,
        asset_id: Option<Uuid>,
        method: SkinBindMethod,
        max_influences: usize,
    ) -> Self {
        Self {
            skeleton_id,
            asset_id,
            method,
            max_influences,
            custom_skin: None,
        }
    }
}

impl Command for BindRigCmd {
    fn label(&self) -> &'static str {
        "bind rig"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let skel = state
            .document
            .project
            .get_skeleton(self.skeleton_id)
            .ok_or("Skeleton not found")?;
        if skel.bones.is_empty() {
            return Err("Skeleton has no bones to bind");
        }
        let asset =
            find_asset(&state.document.project, self.asset_id).ok_or("Target asset not found")?;
        if asset.locked {
            return Err("Asset is locked");
        }
        if asset.mesh.verts.is_empty() {
            return Err("Asset mesh has no vertices");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let skel = state
            .document
            .project
            .get_skeleton(self.skeleton_id)
            .cloned()
            .ok_or_else(|| exec_err("Skeleton not found"))?;

        let asset = find_asset(&state.document.project, self.asset_id)
            .ok_or_else(|| exec_err("Asset not found"))?;

        let mesh = asset.mesh.clone();

        let skin_data = match &self.custom_skin {
            Some(custom) => {
                custom
                    .validate(mesh.verts.len(), skel.bones.len())
                    .map_err(exec_err)?;
                custom.clone()
            }
            None => compute_skin_weights(&mesh, &skel, self.method, self.max_influences)
                .map_err(exec_err)?,
        };

        let asset_name = {
            let asset_mut = find_asset_mut(&mut state.document.project, self.asset_id)
                .ok_or_else(|| exec_err("Asset not found"))?;

            asset_mut.skeleton_id = Some(self.skeleton_id);
            asset_mut.skin_data = Some(skin_data);
            asset_mut.skinning_enabled = true;
            asset_mut.name.clone()
        };

        state.set_status(format!("Bound rig to asset '{asset_name}'"));
        Ok(())
    }
}

/// Desvincula o esqueleto e remove os dados de skinning do asset (RIG-002/009).
#[derive(Debug, Clone, Default)]
pub struct UnbindRigCmd {
    pub asset_id: Option<Uuid>,
}

impl Command for UnbindRigCmd {
    fn label(&self) -> &'static str {
        "unbind rig"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let asset =
            find_asset(&state.document.project, self.asset_id).ok_or("Target asset not found")?;
        if asset.skeleton_id.is_none() && asset.skin_data.is_none() {
            return Err("Asset is not bound to any rig");
        }
        if asset.locked {
            return Err("Asset is locked");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let asset_name = {
            let asset = find_asset_mut(&mut state.document.project, self.asset_id)
                .ok_or_else(|| exec_err("Asset not found"))?;

            if asset.skeleton_id.is_none() && asset.skin_data.is_none() {
                return Err(CommandError::NoChange("Asset is not bound to a rig".into()));
            }

            asset.skeleton_id = None;
            asset.skin_data = None;
            asset.name.clone()
        };

        state.set_status(format!("Unbound rig from asset '{asset_name}'"));
        Ok(())
    }
}

/// Restaura todos os ossos do esqueleto para a pose neutra (Rest Pose) (RIG-008).
#[derive(Debug, Clone)]
pub struct ResetAllBonePosesCmd {
    pub skeleton_id: Uuid,
}

impl Command for ResetAllBonePosesCmd {
    fn label(&self) -> &'static str {
        "reset all bone poses"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let skel = state
            .document
            .project
            .get_skeleton(self.skeleton_id)
            .ok_or("Skeleton not found")?;
        if skel.bones.is_empty() {
            return Err("Skeleton has no bones");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let skel = state
            .document
            .project
            .skeletons
            .iter_mut()
            .find(|s| s.id == self.skeleton_id)
            .ok_or_else(|| exec_err("Skeleton not found"))?;

        let identity_rot = [0.0, 0.0, 0.0, 1.0];
        let identity_scale = [1.0, 1.0, 1.0];

        let mut changed = false;
        for bone in &mut skel.bones {
            if bone.local_transform.rotation != identity_rot
                || bone.local_transform.scale != identity_scale
            {
                bone.local_transform.rotation = identity_rot;
                bone.local_transform.scale = identity_scale;
                changed = true;
            }
        }

        if !changed {
            return Err(CommandError::NoChange(
                "All bones are already in rest pose".into(),
            ));
        }

        state.set_status("All bones reset to rest pose");
        Ok(())
    }
}

/// Aplica a pose atual do esqueleto como novo Rest Pose (RIG-008).
/// Atualiza as posições de repouso head e tail dos ossos, recalcula as matrizes
/// de bind e, se fornecido asset com skin_data, deforma a malha base para a nova pose.
#[derive(Debug, Clone)]
pub struct ApplyPoseAsRestCmd {
    pub skeleton_id: Uuid,
    pub asset_id: Option<Uuid>,
}

impl Command for ApplyPoseAsRestCmd {
    fn label(&self) -> &'static str {
        "apply pose as rest"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let skel = state
            .document
            .project
            .get_skeleton(self.skeleton_id)
            .ok_or("Skeleton not found")?;
        if skel.bones.is_empty() {
            return Err("Skeleton has no bones");
        }
        if let Some(aid) = self.asset_id {
            let asset = state
                .document
                .project
                .assets
                .iter()
                .find(|a| a.id == aid)
                .ok_or("Asset not found")?;
            if asset.locked {
                return Err("Asset is locked");
            }
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let skel = state
            .document
            .project
            .skeletons
            .iter_mut()
            .find(|s| s.id == self.skeleton_id)
            .ok_or_else(|| exec_err("Skeleton not found"))?;

        let pose_transforms: Vec<Transform3D> =
            skel.bones.iter().map(|b| b.local_transform).collect();
        let world_matrices = skel
            .world_pose_matrices(&pose_transforms)
            .map_err(exec_err)?;
        let skinning_matrices = skel
            .compute_skinning_matrices(&pose_transforms)
            .map_err(exec_err)?;

        // Verificar se houve alguma alteração em relação ao rest pose
        let identity_rot = [0.0, 0.0, 0.0, 1.0];
        let identity_scale = [1.0, 1.0, 1.0];
        let is_identity = skel.bones.iter().all(|b| {
            b.local_transform.rotation == identity_rot && b.local_transform.scale == identity_scale
        });
        if is_identity {
            return Err(CommandError::NoChange(
                "Skeleton is already in rest pose".into(),
            ));
        }

        // Para cada osso, calcular novas coordenadas head e tail no espaço do esqueleto
        for (i, b) in skel.bones.iter_mut().enumerate() {
            let wm = world_matrices[i];
            let orig_tail = glam::Vec3::from(b.tail);
            let inv_bind = glam::Mat4::from_cols_array(&b.inverse_bind_matrix);
            let local_tail = inv_bind.transform_point3(orig_tail);

            let new_head = wm.transform_point3(glam::Vec3::ZERO);
            let new_tail = wm.transform_point3(local_tail);

            b.head = [new_head.x, new_head.y, new_head.z];
            b.tail = [new_tail.x, new_tail.y, new_tail.z];
            b.local_transform.rotation = identity_rot;
            b.local_transform.scale = identity_scale;
        }

        skel.compute_bind_pose_matrices();

        // Se houver asset associado, deformar a malha base para corresponder à nova rest pose
        if let Some(asset) = find_asset_mut(&mut state.document.project, self.asset_id)
            && let Some(skin_data) = &asset.skin_data
            && skin_data.skeleton_id == self.skeleton_id
            && asset.skinning_enabled
        {
            let orig_mesh = asset.mesh.clone();
            let deformed_mesh = skin_data.deform_mesh(&orig_mesh, &skinning_matrices);
            asset.mesh = deformed_mesh.into();
        }

        state.set_status("Current pose applied as new rest pose");
        Ok(())
    }
}

/// Habilita ou desabilita a deformação de skinning no asset sem apagar dados (RIG-012).
#[derive(Debug, Clone)]
pub struct SetSkinningEnabledCmd {
    pub asset_id: Option<Uuid>,
    pub enabled: bool,
}

impl Command for SetSkinningEnabledCmd {
    fn label(&self) -> &'static str {
        "set skinning enabled"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let asset =
            find_asset(&state.document.project, self.asset_id).ok_or("Target asset not found")?;
        if asset.locked {
            return Err("Asset is locked");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let asset_name = {
            let asset = find_asset_mut(&mut state.document.project, self.asset_id)
                .ok_or_else(|| exec_err("Asset not found"))?;

            if asset.skinning_enabled == self.enabled {
                return Err(CommandError::NoChange(format!(
                    "Skinning is already {}",
                    if self.enabled { "enabled" } else { "disabled" }
                )));
            }

            asset.skinning_enabled = self.enabled;
            asset.name.clone()
        };

        state.set_status(format!(
            "Skinning {} for asset '{asset_name}'",
            if self.enabled { "enabled" } else { "disabled" }
        ));
        Ok(())
    }
}

/// Estado de pré-visualização de skinning antes da confirmação do bind (RIG-002/004/009).
/// Permite inspecionar e revisar pesos sem poluir ou alterar o documento prematuramente.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SkinningPreviewState {
    pub skeleton_id: Uuid,
    pub asset_id: Uuid,
    pub method: SkinBindMethod,
    pub max_influences: usize,
    pub skin_data: SkinData,
    pub is_valid: bool,
    pub diagnostic: Option<String>,
}

impl SkinningPreviewState {
    pub fn compute(
        asset_id: Uuid,
        mesh: &petunia_mesh::Mesh,
        skeleton: &Skeleton,
        method: SkinBindMethod,
        max_influences: usize,
    ) -> Self {
        match compute_skin_weights(mesh, skeleton, method, max_influences) {
            Ok(skin_data) => Self {
                skeleton_id: skeleton.id,
                asset_id,
                method,
                max_influences,
                skin_data,
                is_valid: true,
                diagnostic: None,
            },
            Err(e) => Self {
                skeleton_id: skeleton.id,
                asset_id,
                method,
                max_influences,
                skin_data: SkinData::new(skeleton.id, mesh.verts.len()),
                is_valid: false,
                diagnostic: Some(e.to_string()),
            },
        }
    }

    /// Permite editar manualmente o peso de um vértice durante a pré-visualização revisável.
    pub fn override_vertex_weight(
        &mut self,
        vertex_idx: usize,
        bones: [u32; 4],
        weights: [f32; 4],
    ) -> Result<(), RigError> {
        if vertex_idx >= self.skin_data.vertex_weights.len() {
            return Err(RigError::InvalidWeights {
                vertex_index: vertex_idx,
                reason: "Índice de vértice fora dos limites".to_string(),
            });
        }
        let mut vw = VertexSkinWeight::new(bones, weights);
        vw.normalize();
        self.skin_data.vertex_weights[vertex_idx] = vw;
        Ok(())
    }

    /// Cria o comando transacional de bind a partir da pré-visualização revisada.
    pub fn commit_cmd(&self) -> BindRigCmd {
        BindRigCmd {
            skeleton_id: self.skeleton_id,
            asset_id: Some(self.asset_id),
            method: self.method,
            max_influences: self.max_influences,
            custom_skin: Some(self.skin_data.clone()),
        }
    }
}

// ---------------------------------------------------------------------------
// H32: Smart Rig Estrutural & Live Opcional (RIG-003/005/006/007/012)
// ---------------------------------------------------------------------------

/// Habilita ou desabilita a auto-atualização do Live Rig sem apagar dados (RIG-012).
#[derive(Debug, Clone)]
pub struct SetLiveAutoUpdateCmd {
    pub asset_id: Option<Uuid>,
    pub enabled: bool,
}

impl Command for SetLiveAutoUpdateCmd {
    fn label(&self) -> &'static str {
        "set live auto-update"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let asset = find_asset(&state.document.project, self.asset_id).ok_or("Asset not found")?;
        if asset.locked {
            return Err("Asset is locked");
        }
        if asset.live_rig.is_none() {
            return Err("Asset does not have a live rig configured");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let asset_name = {
            let asset = find_asset_mut(&mut state.document.project, self.asset_id)
                .ok_or_else(|| exec_err("Asset not found"))?;

            let live = asset
                .live_rig
                .as_mut()
                .ok_or_else(|| exec_err("Live rig not found"))?;

            if live.auto_update_enabled == self.enabled {
                return Err(CommandError::NoChange(format!(
                    "Live auto-update is already {}",
                    if self.enabled { "enabled" } else { "disabled" }
                )));
            }

            live.auto_update_enabled = self.enabled;
            asset.name.clone()
        };

        state.set_status(format!(
            "Live auto-update {} for asset '{asset_name}'",
            if self.enabled { "enabled" } else { "disabled" }
        ));
        Ok(())
    }
}

/// Cria e inicializa um Grafo de Forma Estrutural (StructuralShapeGraph) no asset (RIG-003).
#[derive(Debug, Clone)]
pub struct CreateStructuralGraphCmd {
    pub asset_id: Option<Uuid>,
    pub name: String,
}

impl Command for CreateStructuralGraphCmd {
    fn label(&self) -> &'static str {
        "create structural shape graph"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let asset = find_asset(&state.document.project, self.asset_id).ok_or("Asset not found")?;
        if asset.locked {
            return Err("Asset is locked");
        }
        if self.name.trim().is_empty() {
            return Err("Graph name cannot be empty");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let name = self.name.trim().to_string();
        let asset_name = {
            let asset = find_asset_mut(&mut state.document.project, self.asset_id)
                .ok_or_else(|| exec_err("Asset not found"))?;

            let graph = StructuralShapeGraph::new(&name);
            asset.live_rig = Some(LiveRigData::new(graph));
            asset.name.clone()
        };

        state.set_status(format!(
            "Created structural shape graph '{name}' for '{asset_name}'"
        ));
        Ok(())
    }
}

/// Adiciona uma parte anatômica/estrutural ao grafo de forma (RIG-003/005).
#[derive(Debug, Clone)]
pub struct AddStructuralPartCmd {
    pub asset_id: Option<Uuid>,
    pub name: String,
    pub vertex_indices: Vec<usize>,
    pub center: [f32; 3],
    pub bounds: ([f32; 3], [f32; 3]),
}

impl Command for AddStructuralPartCmd {
    fn label(&self) -> &'static str {
        "add structural part"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let asset = find_asset(&state.document.project, self.asset_id).ok_or("Asset not found")?;
        if asset.locked {
            return Err("Asset is locked");
        }
        let live = asset
            .live_rig
            .as_ref()
            .ok_or("Asset has no structural shape graph")?;

        let name = self.name.trim();
        if name.is_empty() {
            return Err("Part name cannot be empty");
        }
        if live.graph.parts.iter().any(|p| p.name == name) {
            return Err("A structural part with this name already exists");
        }
        if !finite3(&self.center) || !finite3(&self.bounds.0) || !finite3(&self.bounds.1) {
            return Err("Coordinates and bounds must be finite numbers");
        }
        let n_verts = asset.mesh.verts.len();
        if self.vertex_indices.iter().any(|&i| i >= n_verts) {
            return Err("Vertex index out of mesh bounds");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let asset = find_asset_mut(&mut state.document.project, self.asset_id)
            .ok_or_else(|| exec_err("Asset not found"))?;

        let live = asset
            .live_rig
            .as_mut()
            .ok_or_else(|| exec_err("Asset has no structural shape graph"))?;

        let name = self.name.trim();
        let part_id = live
            .graph
            .add_part(name, self.vertex_indices.clone(), self.center, self.bounds)
            .map_err(exec_err)?;

        state.set_status(format!("Added structural part '{name}' (id: {part_id})"));
        Ok(())
    }
}

/// Remove uma parte estrutural do grafo (RIG-003).
#[derive(Debug, Clone)]
pub struct RemoveStructuralPartCmd {
    pub asset_id: Option<Uuid>,
    pub part_id: PartId,
}

impl Command for RemoveStructuralPartCmd {
    fn label(&self) -> &'static str {
        "remove structural part"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let asset = find_asset(&state.document.project, self.asset_id).ok_or("Asset not found")?;
        if asset.locked {
            return Err("Asset is locked");
        }
        let live = asset
            .live_rig
            .as_ref()
            .ok_or("Asset has no structural shape graph")?;

        if live.graph.get_part(self.part_id).is_none() {
            return Err("Structural part not found");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let asset = find_asset_mut(&mut state.document.project, self.asset_id)
            .ok_or_else(|| exec_err("Asset not found"))?;

        let live = asset
            .live_rig
            .as_mut()
            .ok_or_else(|| exec_err("Asset has no structural shape graph"))?;

        live.graph.remove_part(self.part_id).map_err(exec_err)?;
        state.set_status(format!("Removed structural part id: {}", self.part_id));
        Ok(())
    }
}

/// Adiciona uma junta ao grafo de forma estrutural (RIG-003/005).
#[derive(Debug, Clone)]
pub struct AddStructuralJointCmd {
    pub asset_id: Option<Uuid>,
    pub name: String,
    pub bone_id: Option<u32>,
    pub position: [f32; 3],
    pub parent: Option<JointId>,
}

impl Command for AddStructuralJointCmd {
    fn label(&self) -> &'static str {
        "add structural joint"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let asset = find_asset(&state.document.project, self.asset_id).ok_or("Asset not found")?;
        if asset.locked {
            return Err("Asset is locked");
        }
        let live = asset
            .live_rig
            .as_ref()
            .ok_or("Asset has no structural shape graph")?;

        let name = self.name.trim();
        if name.is_empty() {
            return Err("Joint name cannot be empty");
        }
        if live.graph.joints.iter().any(|j| j.name == name) {
            return Err("A structural joint with this name already exists");
        }
        if !finite3(&self.position) {
            return Err("Position coordinates must be finite numbers");
        }
        if let Some(pid) = self.parent
            && live.graph.get_joint(pid).is_none()
        {
            return Err("Parent joint not found");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let asset = find_asset_mut(&mut state.document.project, self.asset_id)
            .ok_or_else(|| exec_err("Asset not found"))?;

        let live = asset
            .live_rig
            .as_mut()
            .ok_or_else(|| exec_err("Asset has no structural shape graph"))?;

        let name = self.name.trim();
        let joint_id = live
            .graph
            .add_joint(name, self.bone_id, self.position, self.parent)
            .map_err(exec_err)?;

        state.set_status(format!("Added structural joint '{name}' (id: {joint_id})"));
        Ok(())
    }
}

/// Conecta duas partes estruturais no grafo (RIG-003).
#[derive(Debug, Clone)]
pub struct ConnectStructuralPartsCmd {
    pub asset_id: Option<Uuid>,
    pub from_part: PartId,
    pub to_part: PartId,
    pub kind: ConnectionKind,
}

impl Command for ConnectStructuralPartsCmd {
    fn label(&self) -> &'static str {
        "connect structural parts"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let asset = find_asset(&state.document.project, self.asset_id).ok_or("Asset not found")?;
        if asset.locked {
            return Err("Asset is locked");
        }
        let live = asset
            .live_rig
            .as_ref()
            .ok_or("Asset has no structural shape graph")?;

        if self.from_part == self.to_part {
            return Err("Cannot connect a part to itself");
        }
        if live.graph.get_part(self.from_part).is_none() {
            return Err("Source part not found");
        }
        if live.graph.get_part(self.to_part).is_none() {
            return Err("Target part not found");
        }
        if live.graph.connections.iter().any(|c| {
            (c.from_part == self.from_part && c.to_part == self.to_part)
                || (c.from_part == self.to_part && c.to_part == self.from_part)
        }) {
            return Err("Connection between these parts already exists");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let asset = find_asset_mut(&mut state.document.project, self.asset_id)
            .ok_or_else(|| exec_err("Asset not found"))?;

        let live = asset
            .live_rig
            .as_mut()
            .ok_or_else(|| exec_err("Asset has no structural shape graph"))?;

        let conn_id = live
            .graph
            .connect_parts(self.from_part, self.to_part, self.kind)
            .map_err(exec_err)?;

        state.set_status(format!(
            "Connected parts {} and {} (connection id: {conn_id})",
            self.from_part, self.to_part
        ));
        Ok(())
    }
}

/// Define o estado de bloqueio (lock) de uma parte estrutural (RIG-005/006).
#[derive(Debug, Clone)]
pub struct SetStructuralPartLockedCmd {
    pub asset_id: Option<Uuid>,
    pub part_id: PartId,
    pub locked: bool,
}

impl Command for SetStructuralPartLockedCmd {
    fn label(&self) -> &'static str {
        "set structural part locked"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let asset = find_asset(&state.document.project, self.asset_id).ok_or("Asset not found")?;
        if asset.locked {
            return Err("Asset is locked");
        }
        let live = asset
            .live_rig
            .as_ref()
            .ok_or("Asset has no structural shape graph")?;

        if live.graph.get_part(self.part_id).is_none() {
            return Err("Structural part not found");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let asset = find_asset_mut(&mut state.document.project, self.asset_id)
            .ok_or_else(|| exec_err("Asset not found"))?;

        let live = asset
            .live_rig
            .as_mut()
            .ok_or_else(|| exec_err("Asset has no structural shape graph"))?;

        let part = live
            .graph
            .get_part_mut(self.part_id)
            .ok_or_else(|| exec_err("Part not found"))?;

        if part.locked == self.locked {
            return Err(CommandError::NoChange(format!(
                "Part {} is already {}",
                self.part_id,
                if self.locked { "locked" } else { "unlocked" }
            )));
        }

        part.locked = self.locked;
        state.set_status(format!(
            "Part {} {}",
            self.part_id,
            if self.locked { "locked" } else { "unlocked" }
        ));
        Ok(())
    }
}

/// Define o estado de bloqueio (lock) de um osso no Live Rig (RIG-005/006).
#[derive(Debug, Clone)]
pub struct SetStructuralBoneLockedCmd {
    pub asset_id: Option<Uuid>,
    pub bone_id: u32,
    pub locked: bool,
}

impl Command for SetStructuralBoneLockedCmd {
    fn label(&self) -> &'static str {
        "set structural bone locked"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let asset = find_asset(&state.document.project, self.asset_id).ok_or("Asset not found")?;
        if asset.locked {
            return Err("Asset is locked");
        }
        if asset.live_rig.is_none() {
            return Err("Asset has no structural shape graph");
        }
        let skel_id = asset.skeleton_id.ok_or("Asset has no skeleton bound")?;
        let skel = state
            .document
            .project
            .get_skeleton(skel_id)
            .ok_or("Skeleton not found")?;
        if skel.get_bone(self.bone_id).is_none() {
            return Err("Bone not found in skeleton");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let asset = find_asset_mut(&mut state.document.project, self.asset_id)
            .ok_or_else(|| exec_err("Asset not found"))?;

        let live = asset
            .live_rig
            .as_mut()
            .ok_or_else(|| exec_err("Live rig not found"))?;

        let changed = if self.locked {
            live.locked_bones.insert(self.bone_id)
        } else {
            live.locked_bones.remove(&self.bone_id)
        };

        if !changed {
            return Err(CommandError::NoChange(format!(
                "Bone {} is already {}",
                self.bone_id,
                if self.locked { "locked" } else { "unlocked" }
            )));
        }

        state.set_status(format!(
            "Bone {} {}",
            self.bone_id,
            if self.locked { "locked" } else { "unlocked" }
        ));
        Ok(())
    }
}

/// Define um override manual de pesos para um vértice que DEVE ser preservado (RIG-005/006).
#[derive(Debug, Clone)]
pub struct SetVertexWeightOverrideCmd {
    pub asset_id: Option<Uuid>,
    pub vertex_index: usize,
    pub bones: [u32; 4],
    pub weights: [f32; 4],
}

impl Command for SetVertexWeightOverrideCmd {
    fn label(&self) -> &'static str {
        "set vertex weight override"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let asset = find_asset(&state.document.project, self.asset_id).ok_or("Asset not found")?;
        if asset.locked {
            return Err("Asset is locked");
        }
        if asset.live_rig.is_none() {
            return Err("Asset has no structural shape graph");
        }
        if self.vertex_index >= asset.mesh.verts.len() {
            return Err("Vertex index out of mesh bounds");
        }
        if !self.weights.iter().all(|w| w.is_finite() && *w >= 0.0) {
            return Err("Weights must be finite non-negative numbers");
        }
        if self.weights.iter().sum::<f32>() < 1e-6 {
            return Err("Weight sum cannot be zero");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let asset = find_asset_mut(&mut state.document.project, self.asset_id)
            .ok_or_else(|| exec_err("Asset not found"))?;

        let mut vw = VertexSkinWeight::new(self.bones, self.weights);
        vw.normalize();

        let live = asset
            .live_rig
            .as_mut()
            .ok_or_else(|| exec_err("Live rig not found"))?;

        live.overrides.insert(self.vertex_index, vw);

        if let Some(skin) = asset.skin_data.as_mut()
            && let Some(target) = skin.vertex_weights.get_mut(self.vertex_index)
        {
            *target = vw;
        }

        state.set_status(format!(
            "Set weight override for vertex {}",
            self.vertex_index
        ));
        Ok(())
    }
}

/// Remove o override manual de um vértice (RIG-005/006).
#[derive(Debug, Clone)]
pub struct ClearVertexWeightOverrideCmd {
    pub asset_id: Option<Uuid>,
    pub vertex_index: usize,
}

impl Command for ClearVertexWeightOverrideCmd {
    fn label(&self) -> &'static str {
        "clear vertex weight override"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let asset = find_asset(&state.document.project, self.asset_id).ok_or("Asset not found")?;
        if asset.locked {
            return Err("Asset is locked");
        }
        let live = asset
            .live_rig
            .as_ref()
            .ok_or("Asset has no structural shape graph")?;

        if !live.overrides.contains_key(&self.vertex_index) {
            return Err("Vertex does not have a weight override");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let asset = find_asset_mut(&mut state.document.project, self.asset_id)
            .ok_or_else(|| exec_err("Asset not found"))?;

        let live = asset
            .live_rig
            .as_mut()
            .ok_or_else(|| exec_err("Live rig not found"))?;

        if live.overrides.remove(&self.vertex_index).is_none() {
            return Err(CommandError::NoChange("No override found to clear".into()));
        }

        state.set_status(format!(
            "Cleared weight override for vertex {}",
            self.vertex_index
        ));
        Ok(())
    }
}

/// Executa a atualização localizada e idempotente do Smart Rig estrutural (RIG-003/005/006/007).
#[derive(Debug, Clone)]
pub struct UpdateStructuralRigCmd {
    pub asset_id: Option<Uuid>,
    pub method: SkinBindMethod,
    pub max_influences: usize,
}

impl Command for UpdateStructuralRigCmd {
    fn label(&self) -> &'static str {
        "update structural rig"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let asset = find_asset(&state.document.project, self.asset_id).ok_or("Asset not found")?;
        if asset.locked {
            return Err("Asset is locked");
        }
        if asset.live_rig.is_none() {
            return Err("Asset has no structural shape graph");
        }
        if asset.skeleton_id.is_none() {
            return Err("Asset has no skeleton bound");
        }
        if asset.skin_data.is_none() {
            return Err("Asset has no skin data");
        }
        if self.max_influences == 0 || self.max_influences > 4 {
            return Err("Max influences must be between 1 and 4");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let (skel_clone, mesh_clone, pos_rev, topo_rev, mat_rev, uv_rev) = {
            let asset = find_asset(&state.document.project, self.asset_id)
                .ok_or_else(|| exec_err("Asset not found"))?;
            let skel_id = asset
                .skeleton_id
                .ok_or_else(|| exec_err("Asset has no skeleton"))?;
            let skel = state
                .document
                .project
                .get_skeleton(skel_id)
                .ok_or_else(|| exec_err("Skeleton not found"))?;

            (
                skel.clone(),
                asset.mesh.clone(),
                state.document.project.position_revision,
                state.document.project.topology_revision,
                state.document.project.material_revision,
                state.document.project.uv_revision,
            )
        };

        let asset = find_asset_mut(&mut state.document.project, self.asset_id)
            .ok_or_else(|| exec_err("Asset not found"))?;

        let skin_data = asset
            .skin_data
            .as_mut()
            .ok_or_else(|| exec_err("Skin data not found"))?;

        let live_data = asset
            .live_rig
            .as_mut()
            .ok_or_else(|| exec_err("Live rig not found"))?;

        let report = update_structural_rig_localized(
            &mesh_clone,
            &skel_clone,
            skin_data,
            live_data,
            pos_rev,
            topo_rev,
            mat_rev,
            uv_rev,
            self.method,
            self.max_influences,
        )
        .map_err(exec_err)?;

        if !report.changed {
            return Err(CommandError::NoChange(report.message));
        }

        state.set_status(report.message);
        Ok(())
    }
}

/// Remapeia explicitamente os pesos e o grafo após alterações na topologia (RIG-005/007).
#[derive(Debug, Clone)]
pub struct RemapRigWeightsCmd {
    pub asset_id: Option<Uuid>,
    pub old_to_new_indices: Vec<Option<usize>>,
    pub new_vertex_count: usize,
}

impl Command for RemapRigWeightsCmd {
    fn label(&self) -> &'static str {
        "remap rig weights"
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let asset = find_asset(&state.document.project, self.asset_id).ok_or("Asset not found")?;
        if asset.locked {
            return Err("Asset is locked");
        }
        let skin = asset.skin_data.as_ref().ok_or("Asset has no skin data")?;
        if asset.live_rig.is_none() {
            return Err("Asset has no structural shape graph");
        }
        if self.old_to_new_indices.len() != skin.vertex_weights.len() {
            return Err("Remap table size does not match skin weights count");
        }
        if self.new_vertex_count != asset.mesh.verts.len() {
            return Err("New vertex count does not match mesh vertex count");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let asset = find_asset_mut(&mut state.document.project, self.asset_id)
            .ok_or_else(|| exec_err("Asset not found"))?;

        let skin_data = asset
            .skin_data
            .as_mut()
            .ok_or_else(|| exec_err("Skin data not found"))?;

        let live_data = asset
            .live_rig
            .as_mut()
            .ok_or_else(|| exec_err("Live rig not found"))?;

        remap_skin_weights(
            skin_data,
            live_data,
            &self.old_to_new_indices,
            self.new_vertex_count,
        )
        .map_err(exec_err)?;

        state.set_status(format!(
            "Successfully remapped rig weights to {} vertices",
            self.new_vertex_count
        ));
        Ok(())
    }
}
