//! Sessão do workspace Animate (P3D-169/170, cap. 45 F2): estado **UI-neutro**
//! do que o usuário está vendo e fazendo — qual criatura, qual Motion, onde
//! está o playhead — mais a pré-visualização de pose e as ações que a UI pode
//! disparar. Nenhum tipo de toolkit passa por aqui (AGENTS.md §3).
//!
//! Divisão de responsabilidades:
//!
//! - [`AnimateSession`]: dados simples (seleção, tempo, play, *draft* de
//!   parâmetro). Vive em [`crate::EditorSession`]; **não** é persistido.
//! - [`AnimatePreview`]: cache do avaliador e amostragem da pose. Vive no host
//!   (o shell), porque o avaliador não é dado de documento.
//! - `impl AppState`: ações (`animate_*`) que traduzem a intenção do usuário em
//!   comandos transacionais (`AddMotionCmd`, `SetMotionParamCmd`, …). Editar o
//!   Motion **sempre** passa pelo dispatcher (Undo/Redo, single-writer).
//!
//! ## Arrastar um slider sem inundar o histórico
//!
//! O dispatcher grava um checkpoint por comando. Um slider gera dezenas de
//! valores por segundo, então o arrasto é um *draft*: [`AnimateSession`] guarda
//! `(motion, chave, valor)` e o preview avalia a receita **com** o draft, ao
//! vivo e sem tocar no projeto. Ao soltar, um único `SetMotionParamCmd`
//! confirma o valor (um passo de Undo). Cancelar descarta o draft.

use crate::command::CommandError;
use crate::rig_commands::{
    AddMotionCmd, AddRigPresetCmd, ApplyMotionNowCmd, DuplicateMotionCmd, FitRigToActiveAssetCmd,
    RemoveMotionCmd, RigPresetKind, SetMotionParamCmd, SetMotionStyleCmd, UpdateMotionCmd,
};
use crate::state::AppState;
use glam::Vec3;
use petunia_project::{
    BakeOptions, MotionError, MotionEvaluator, MotionGenerator, MotionRecipe, MotionStyle,
    PoseOverride, Project, RigRoleMap, RoleContractError, RootMode, Skeleton, Transform3D,
    posed_meshes,
};
use uuid::Uuid;

/// Escala padrão dos rigs criados pelo seletor de criaturas.
pub const DEFAULT_RIG_SCALE: f32 = 1.0;

/// Valor provisório de um parâmetro durante o arrasto de um slider.
#[derive(Debug, Clone, PartialEq)]
struct ParamDraft {
    motion: Uuid,
    key: String,
    value: f32,
}

/// Estado da sessão do Animate. Não é persistido no `.petunia`.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimateSession {
    /// Criatura (esqueleto) em foco.
    pub skeleton: Option<Uuid>,
    /// Motion selecionado (sempre pertence a `skeleton`).
    pub motion: Option<Uuid>,
    /// Playhead em segundos, dentro de `[0, ciclo)`.
    pub time: f32,
    pub playing: bool,
    /// Mostra os parâmetros da camada "Advanced".
    pub show_advanced: bool,
    /// Desenha o esqueleto posado no viewport.
    pub show_skeleton: bool,
    draft: Option<ParamDraft>,
}

impl Default for AnimateSession {
    fn default() -> Self {
        Self {
            skeleton: None,
            motion: None,
            time: 0.0,
            playing: false,
            show_advanced: false,
            show_skeleton: true,
            draft: None,
        }
    }
}

impl AnimateSession {
    /// Parâmetro em edição (arrasto de slider), se houver: `(chave, valor)`.
    pub fn draft(&self) -> Option<(&str, f32)> {
        self.draft.as_ref().map(|d| (d.key.as_str(), d.value))
    }

    /// Valor efetivo de um parâmetro: o draft enquanto o slider está sendo
    /// arrastado, senão o valor gravado na receita.
    pub fn effective_param(&self, recipe: &MotionRecipe, key: &str) -> f32 {
        match &self.draft {
            Some(d) if d.motion == recipe.id && d.key == key => d.value,
            _ => recipe.param(key),
        }
    }

    /// Receita com o draft aplicado (usada pelo preview; nunca é gravada).
    pub fn effective_recipe(&self, recipe: &MotionRecipe) -> MotionRecipe {
        let mut out = recipe.clone();
        if let Some(d) = &self.draft
            && d.motion == recipe.id
        {
            let _ = out.set_param(&d.key, d.value);
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Catálogo de Motions
// ---------------------------------------------------------------------------

/// Por que um Motion não está disponível para o rig atual. A UI traduz cada
/// caso por `TextId`; o domínio não carrega texto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionUnavailable {
    /// Ainda não há criatura/rig.
    NoRig,
    /// Faltam pernas completas (coxa + canela).
    NeedsLegs { needed: usize, found: usize },
    /// Falta uma cadeia de coluna/cauda longa o bastante.
    NeedsChain { needed: usize, found: usize },
    /// Falta um osso de corpo (Hips/Spine/Chest/Root).
    NeedsBody,
}

impl From<&RoleContractError> for MotionUnavailable {
    fn from(error: &RoleContractError) -> Self {
        match *error {
            RoleContractError::NotEnoughLegs { needed, found } => Self::NeedsLegs { needed, found },
            RoleContractError::ChainTooShort { needed, found, .. } => {
                Self::NeedsChain { needed, found }
            }
            RoleContractError::MissingRole(_) => Self::NeedsBody,
        }
    }
}

/// Uma linha do seletor de Motions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotionCatalogEntry {
    pub generator: MotionGenerator,
    /// `None` = disponível para este rig.
    pub unavailable: Option<MotionUnavailable>,
}

impl MotionCatalogEntry {
    pub fn is_available(&self) -> bool {
        self.unavailable.is_none()
    }
}

/// Todos os geradores, marcando quais este rig suporta. Disponíveis primeiro,
/// mantendo a ordem canônica dentro de cada grupo.
pub fn motion_catalog(project: &Project, skeleton: Option<Uuid>) -> Vec<MotionCatalogEntry> {
    let roles = skeleton.and_then(|id| project.rig_roles_of(id));
    let has_rig = skeleton.is_some_and(|id| project.get_skeleton(id).is_some());
    let mut entries: Vec<MotionCatalogEntry> = MotionGenerator::ALL
        .iter()
        .map(|&generator| {
            let unavailable = if !has_rig {
                Some(MotionUnavailable::NoRig)
            } else {
                let empty;
                let map = match roles {
                    Some(map) => map,
                    None => {
                        empty = RigRoleMap::new(skeleton.unwrap_or_default());
                        &empty
                    }
                };
                generator
                    .check_rig(map)
                    .err()
                    .map(|e| MotionUnavailable::from(&e))
            };
            MotionCatalogEntry {
                generator,
                unavailable,
            }
        })
        .collect();
    entries.sort_by_key(|e| !e.is_available());
    entries
}

// ---------------------------------------------------------------------------
// Pré-visualização de pose
// ---------------------------------------------------------------------------

/// Um osso posado, em coordenadas do mundo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreviewBone {
    pub head: [f32; 3],
    pub tail: [f32; 3],
    /// Índice do pai dentro de [`PosePreview::bones`].
    pub parent: Option<usize>,
}

/// Esqueleto posado num instante: o que o viewport desenha.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PosePreview {
    pub bones: Vec<PreviewBone>,
}

impl PosePreview {
    /// Posa o esqueleto com `local` (uma transformação por osso).
    fn from_pose(skeleton: &Skeleton, local: &[Transform3D]) -> Option<Self> {
        let skin = skeleton.compute_skinning_matrices(local).ok()?;
        let index_of = |id: u32| skeleton.bones.iter().position(|b| b.id == id);
        let bones = skeleton
            .bones
            .iter()
            .zip(&skin)
            .map(|(bone, m)| {
                let head = m.transform_point3(Vec3::from(bone.head));
                let tail = m.transform_point3(Vec3::from(bone.tail));
                PreviewBone {
                    head: head.to_array(),
                    tail: tail.to_array(),
                    parent: bone.parent.and_then(index_of),
                }
            })
            .collect();
        Some(Self { bones })
    }

    /// Pose de repouso: cada osso na sua posição original.
    pub fn rest(skeleton: &Skeleton) -> Self {
        let local: Vec<Transform3D> = skeleton.bones.iter().map(|b| b.local_transform).collect();
        Self::from_pose(skeleton, &local).unwrap_or_default()
    }

    /// Caixa envolvente (mín, máx) de todas as articulações; `None` sem ossos.
    pub fn bounds(&self) -> Option<([f32; 3], [f32; 3])> {
        let mut points = self.bones.iter().flat_map(|b| [b.head, b.tail]);
        let first = points.next()?;
        let (mut lo, mut hi) = (first, first);
        for p in points {
            for i in 0..3 {
                lo[i] = lo[i].min(p[i]);
                hi[i] = hi[i].max(p[i]);
            }
        }
        Some((lo, hi))
    }
}

/// Insumos que determinam o avaliador; se algum mudar, ele é reconstruído.
/// Comparação por **conteúdo** (não por `revision`): Undo pode voltar a uma
/// revisão já vista com outros valores, e um cache por número serviria pose velha.
#[derive(Debug, Clone, PartialEq)]
struct EvaluatorInputs {
    recipe: MotionRecipe,
    roles: RigRoleMap,
    skeleton: Skeleton,
}

/// Cache do avaliador + amostragem de pose. Pertence ao host (shell), não ao
/// documento.
#[derive(Default)]
pub struct AnimatePreview {
    inputs: Option<EvaluatorInputs>,
    evaluator: Option<Result<MotionEvaluator, MotionError>>,
    rebuilds: u64,
}

impl AnimatePreview {
    /// Quantas vezes o avaliador foi reconstruído (telemetria e testes).
    pub fn rebuilds(&self) -> u64 {
        self.rebuilds
    }

    /// Erro do avaliador atual (rig incompatível com o Motion), se houver.
    pub fn error(&self) -> Option<&MotionError> {
        self.evaluator.as_ref().and_then(|r| r.as_ref().err())
    }

    fn refresh<'p>(
        &mut self,
        project: &'p Project,
        session: &AnimateSession,
    ) -> Option<&'p Skeleton> {
        let motion = project.get_motion(session.motion?)?;
        let recipe = session.effective_recipe(motion);
        let skeleton = project.get_skeleton(recipe.skeleton_id)?;
        let roles = project
            .rig_roles_of(skeleton.id)
            .cloned()
            .unwrap_or_else(|| RigRoleMap::new(skeleton.id));
        let inputs = EvaluatorInputs {
            recipe,
            roles,
            skeleton: skeleton.clone(),
        };
        if self.inputs.as_ref() != Some(&inputs) {
            self.evaluator = Some(MotionEvaluator::new(
                &inputs.recipe,
                &inputs.skeleton,
                &inputs.roles,
            ));
            self.inputs = Some(inputs);
            self.rebuilds += 1;
        }
        Some(skeleton)
    }

    /// Duração do ciclo do Motion selecionado, em segundos.
    pub fn cycle_seconds(&mut self, project: &Project, session: &AnimateSession) -> Option<f32> {
        self.refresh(project, session)?;
        match self.evaluator.as_ref()? {
            Ok(e) => Some(e.cycle_seconds()),
            Err(_) => None,
        }
    }

    /// Pose do Motion no tempo da sessão; sem Motion, a pose de repouso da
    /// criatura em foco; sem criatura, `None`.
    pub fn pose(&mut self, project: &Project, session: &AnimateSession) -> Option<PosePreview> {
        if session.motion.is_some() {
            let skeleton = self.refresh(project, session)?;
            if let Some(Ok(evaluator)) = self.evaluator.as_ref() {
                let local = evaluator.pose_at(session.time);
                return PosePreview::from_pose(skeleton, &local);
            }
            // Erro de contrato: mostra o repouso; a UI lê `error()`.
            return Some(PosePreview::rest(skeleton));
        }
        let skeleton = project.get_skeleton(session.skeleton?)?;
        Some(PosePreview::rest(skeleton))
    }

    /// Pose **local** (uma transformação por osso) da criatura em foco: a do
    /// Motion no tempo da sessão ou, sem Motion (ou com rig incompatível), o
    /// repouso. Devolve também o id do esqueleto.
    pub fn local_pose(
        &mut self,
        project: &Project,
        session: &AnimateSession,
    ) -> Option<(Uuid, Vec<Transform3D>)> {
        if session.motion.is_some()
            && let Some(skeleton) = self.refresh(project, session)
            && let Some(Ok(evaluator)) = self.evaluator.as_ref()
        {
            return Some((skeleton.id, evaluator.pose_at(session.time)));
        }
        let skeleton = project.get_skeleton(session.skeleton?)?;
        Some((
            skeleton.id,
            skeleton.bones.iter().map(|b| b.local_transform).collect(),
        ))
    }

    /// Malhas deformadas pela pose atual, para o renderer desenhar no lugar das
    /// malhas em repouso. `None` quando nenhum modelo está ligado à criatura em
    /// foco (nada a substituir). O documento não é tocado.
    pub fn pose_override(
        &mut self,
        project: &Project,
        session: &AnimateSession,
        revision: u64,
    ) -> Option<PoseOverride> {
        let (skeleton_id, local) = self.local_pose(project, session)?;
        let over = posed_meshes(project, skeleton_id, &local, revision);
        (!over.is_empty()).then_some(over)
    }

    /// Avança o playhead em `dt` segundos, dando a volta no ciclo. `true` se o
    /// tempo mudou (o shell só redesenha nesse caso).
    pub fn advance(&mut self, project: &Project, session: &mut AnimateSession, dt: f32) -> bool {
        if !session.playing || !dt.is_finite() || dt <= 0.0 {
            return false;
        }
        let Some(cycle) = self.cycle_seconds(project, session) else {
            return false;
        };
        let next = wrap_time(session.time + dt, cycle);
        let changed = (next - session.time).abs() > f32::EPSILON;
        session.time = next;
        changed
    }
}

/// Dá a volta em `[0, cycle)`.
fn wrap_time(t: f32, cycle: f32) -> f32 {
    if cycle <= f32::EPSILON || !t.is_finite() {
        return 0.0;
    }
    t.rem_euclid(cycle)
}

// ---------------------------------------------------------------------------
// Ações (a UI só chama estas; toda mutação passa pelo dispatcher)
// ---------------------------------------------------------------------------

fn no_rig() -> CommandError {
    CommandError::Execution("No rig selected".into())
}

impl AppState {
    /// Mantém a seleção da sessão coerente com o projeto (após Undo/Redo,
    /// importação, remoção…): criatura existente, Motion da criatura, draft do
    /// Motion selecionado. Idempotente e barata; o shell chama a cada frame.
    pub fn animate_resolve(&mut self) {
        let project = &self.project.project;
        let a = &mut self.session.animate;
        if a.skeleton
            .is_none_or(|id| project.get_skeleton(id).is_none())
        {
            a.skeleton = project.skeletons.first().map(|s| s.id);
        }
        let motion_ok = a.motion.is_some_and(|id| {
            project
                .get_motion(id)
                .is_some_and(|m| Some(m.skeleton_id) == a.skeleton)
        });
        if !motion_ok {
            a.motion = project
                .motions
                .iter()
                .find(|m| Some(m.skeleton_id) == a.skeleton)
                .map(|m| m.id);
        }
        if a.motion.is_none() {
            a.playing = false;
            a.time = 0.0;
        }
        if a.draft.as_ref().is_some_and(|d| Some(d.motion) != a.motion) {
            a.draft = None;
        }
    }

    /// Foca outra criatura. `false` se o id não existe.
    pub fn animate_select_skeleton(&mut self, id: Uuid) -> bool {
        if self.project.project.get_skeleton(id).is_none() {
            return false;
        }
        let a = &mut self.session.animate;
        if a.skeleton != Some(id) {
            a.skeleton = Some(id);
            a.motion = None;
            a.draft = None;
            a.time = 0.0;
        }
        self.animate_resolve();
        true
    }

    /// Seleciona um Motion (e a criatura dele). `false` se não existe.
    pub fn animate_select_motion(&mut self, id: Uuid) -> bool {
        let Some(skeleton_id) = self.project.project.get_motion(id).map(|m| m.skeleton_id) else {
            return false;
        };
        let a = &mut self.session.animate;
        a.skeleton = Some(skeleton_id);
        if a.motion != Some(id) {
            a.motion = Some(id);
            a.draft = None;
            a.time = 0.0;
        }
        true
    }

    /// Cria a criatura do preset e a foca. Um passo de Undo.
    pub fn animate_add_rig(&mut self, kind: RigPresetKind) -> Result<Uuid, CommandError> {
        self.dispatch(&AddRigPresetCmd {
            kind,
            scale: DEFAULT_RIG_SCALE,
        })?;
        let id = self
            .project
            .project
            .skeletons
            .last()
            .map(|s| s.id)
            .ok_or_else(no_rig)?;
        self.animate_select_skeleton(id);
        Ok(id)
    }

    /// Cria um Motion para a criatura em foco, seleciona e já toca: quem nunca
    /// animou vê o resultado no mesmo gesto. Um passo de Undo. O erro
    /// estruturado do rig chega intacto quando o gerador não serve.
    pub fn animate_add_motion(&mut self, generator: MotionGenerator) -> Result<Uuid, CommandError> {
        let skeleton_id = self.session.animate.skeleton.ok_or_else(no_rig)?;
        self.dispatch(&AddMotionCmd {
            skeleton_id,
            generator,
            name: None,
            style: MotionStyle::Custom,
        })?;
        let id = self
            .project
            .project
            .motions
            .last()
            .map(|m| m.id)
            .ok_or_else(no_rig)?;
        self.animate_select_motion(id);
        self.session.animate.playing = true;
        Ok(id)
    }

    /// Duplica o Motion selecionado e seleciona a cópia.
    pub fn animate_duplicate_motion(&mut self) -> Result<Uuid, CommandError> {
        let motion_id = self.session.animate.motion.ok_or_else(no_rig)?;
        self.dispatch(&DuplicateMotionCmd { motion_id })?;
        let id = self
            .project
            .project
            .motions
            .last()
            .map(|m| m.id)
            .ok_or_else(no_rig)?;
        self.animate_select_motion(id);
        Ok(id)
    }

    /// Remove o Motion selecionado e passa ao próximo da criatura.
    pub fn animate_remove_motion(&mut self) -> Result<(), CommandError> {
        let motion_id = self.session.animate.motion.ok_or_else(no_rig)?;
        self.dispatch(&RemoveMotionCmd { motion_id })?;
        self.session.animate.motion = None;
        self.animate_resolve();
        Ok(())
    }

    /// Valor provisório durante o arrasto de um slider: o preview reage ao
    /// vivo e o histórico não é tocado. `false` se não há Motion ou a chave é
    /// desconhecida.
    pub fn animate_param_preview(&mut self, key: &str, value: f32) -> bool {
        let Some(motion_id) = self.session.animate.motion else {
            return false;
        };
        let Some(recipe) = self.project.project.get_motion(motion_id) else {
            return false;
        };
        if !value.is_finite() || !recipe.generator.specs().iter().any(|s| s.key == key) {
            return false;
        }
        self.session.animate.draft = Some(ParamDraft {
            motion: motion_id,
            key: key.to_string(),
            value,
        });
        true
    }

    /// Confirma o draft (soltar o slider): um único `SetMotionParamCmd`.
    /// `Ok(false)` quando não havia draft ou o valor final é igual ao gravado.
    pub fn animate_param_commit(&mut self) -> Result<bool, CommandError> {
        let Some(draft) = self.session.animate.draft.take() else {
            return Ok(false);
        };
        match self.dispatch(&SetMotionParamCmd {
            motion_id: draft.motion,
            key: draft.key,
            value: draft.value,
        }) {
            Ok(()) => Ok(true),
            Err(CommandError::NoChange(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Descarta o draft (Esc durante o arrasto).
    pub fn animate_param_cancel(&mut self) {
        self.session.animate.draft = None;
    }

    /// Ajuste imediato (teclado, campo numérico): um passo de Undo.
    pub fn animate_set_param(&mut self, key: &str, value: f32) -> Result<bool, CommandError> {
        let motion_id = self.session.animate.motion.ok_or_else(no_rig)?;
        self.session.animate.draft = None;
        match self.dispatch(&SetMotionParamCmd {
            motion_id,
            key: key.to_string(),
            value,
        }) {
            Ok(()) => Ok(true),
            Err(CommandError::NoChange(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Aplica um Style ao Motion selecionado.
    pub fn animate_set_style(&mut self, style: MotionStyle) -> Result<bool, CommandError> {
        let motion_id = self.session.animate.motion.ok_or_else(no_rig)?;
        self.session.animate.draft = None;
        match self.dispatch(&SetMotionStyleCmd { motion_id, style }) {
            Ok(()) => Ok(true),
            Err(CommandError::NoChange(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// **Fit to model**: ajusta a criatura em foco ao modelo ativo e os liga por
    /// skin, para o viewport mostrar a malha se movendo. Um passo de Undo.
    pub fn animate_fit_to_model(&mut self) -> Result<(), CommandError> {
        let skeleton_id = self.session.animate.skeleton.ok_or_else(no_rig)?;
        self.dispatch(&FitRigToActiveAssetCmd { skeleton_id })
    }

    /// Liga/desliga o Stepped (`Some(fps)` liga; `None` desliga).
    pub fn animate_set_stepped(&mut self, fps: Option<f32>) -> Result<bool, CommandError> {
        self.animate_update_motion(|c| c.step_fps = Some(fps))
    }

    /// Alterna entre In Place e Root Motion.
    pub fn animate_set_root_mode(&mut self, mode: RootMode) -> Result<bool, CommandError> {
        self.animate_update_motion(|c| c.root_mode = Some(mode))
    }

    fn animate_update_motion(
        &mut self,
        fill: impl FnOnce(&mut UpdateMotionCmd),
    ) -> Result<bool, CommandError> {
        let motion_id = self.session.animate.motion.ok_or_else(no_rig)?;
        let mut cmd = UpdateMotionCmd {
            motion_id,
            ..Default::default()
        };
        fill(&mut cmd);
        match self.dispatch(&cmd) {
            Ok(()) => Ok(true),
            Err(CommandError::NoChange(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Alterna reprodução. Sem Motion, não toca.
    pub fn animate_toggle_play(&mut self) {
        let a = &mut self.session.animate;
        a.playing = a.motion.is_some() && !a.playing;
    }

    /// Posiciona o playhead (segundos) e pausa: arrastar a barra é "scrub".
    pub fn animate_seek(&mut self, time: f32) {
        let a = &mut self.session.animate;
        a.time = if time.is_finite() { time.max(0.0) } else { 0.0 };
        a.playing = false;
    }

    /// **Apply Now**: converte o Motion em clipe editável. Com
    /// `keep_motion = true` o Motion vivo continua (Keep Live). Um passo de
    /// Undo; a mensagem de status relata as chaves geradas.
    pub fn animate_apply_now(&mut self, keep_motion: bool) -> Result<(), CommandError> {
        let motion_id = self.session.animate.motion.ok_or_else(no_rig)?;
        self.session.animate.draft = None;
        self.dispatch(&ApplyMotionNowCmd {
            motion_id,
            options: BakeOptions::default(),
            keep_motion,
        })?;
        self.animate_resolve();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_time_stays_inside_the_cycle() {
        assert_eq!(wrap_time(0.0, 1.0), 0.0);
        assert!((wrap_time(2.25, 1.0) - 0.25).abs() < 1e-6);
        assert!((wrap_time(-0.25, 1.0) - 0.75).abs() < 1e-6);
        assert_eq!(wrap_time(1.0, 0.0), 0.0);
        assert_eq!(wrap_time(f32::NAN, 1.0), 0.0);
    }

    #[test]
    fn a_new_session_is_empty_and_paused() {
        let s = AnimateSession::default();
        assert!(s.skeleton.is_none() && s.motion.is_none());
        assert!(!s.playing);
        assert!(s.show_skeleton);
        assert!(s.draft().is_none());
    }
}
