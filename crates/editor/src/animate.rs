//! Bridge do workspace Animate (cap. 45 F2, P3D-169/170).
//!
//! O markup Slint só emite **ações semânticas** ([`AnimateIntent`]) e só lê um
//! [`AnimateViewModel`] pronto: toda regra vive em `petunia_core`
//! (`animate_session`) e nos comandos transacionais de Motion/Rig. Nada aqui
//! conhece widgets; nada no domínio conhece este módulo (AGENTS.md §0.1).
//!
//! Regras de UX que este módulo garante (cap. 45, "camadas 1–2"):
//!
//! - **Um clique e já se move**: escolher um Motion cria, seleciona e toca.
//! - **Sem histórico por arrasto**: sliders emitem `ParamPreview` (draft ao
//!   vivo) e um único `ParamCommit` ao soltar.
//! - **Nada indisponível some**: Motions que o rig não suporta aparecem
//!   desabilitados com o motivo em linguagem simples.
//! - **Sem texto fixo**: rótulos vêm de `TextId` (chrome) e de chaves
//!   `animate.<grupo>.<id>` (geradores, estilos, parâmetros), cobertas por
//!   teste nos dois locales.
//!
//! O módulo compila sempre; só o **workspace** (a pill ANIMATE) fica atrás da
//! feature `animation-workspace`. Assim a suíte padrão exercita o bridge.

use std::cell::RefCell;
use std::sync::Arc;
use std::time::Instant;

use petunia_config::text_id as T;
use petunia_core::{
    AnimatePreview, CommandError, FitBlocked, MotionUnavailable, PosePreview, RigPresetKind,
    fit_availability, motion_catalog,
};
use petunia_project::{
    BlendMode, CurveChannel, Interpolation, MotionGenerator, MotionRecipe, MotionStyle,
    PoseOverride, RootMode,
};

use crate::{PetuniaViewport, SlintUiBridge, project_preview_segment, project_world_point};

/// Fração máxima de um passo de tempo (evita salto após travada da UI).
const MAX_TICK_SECONDS: f32 = 0.1;

/// Estilos oferecidos como chips (Custom é só o estado "editado à mão").
pub const STYLE_CHOICES: [MotionStyle; 4] = [
    MotionStyle::Cartoon,
    MotionStyle::Heavy,
    MotionStyle::Stiff,
    MotionStyle::Floaty,
];

/// Estado de runtime do Animate no shell (não é documento).
#[derive(Default)]
pub struct AnimateRuntime {
    /// Cache do avaliador. `RefCell` porque `view_model(&self)` precisa
    /// amostrar a pose sem exigir `&mut` do bridge inteiro.
    pub(crate) preview: RefCell<AnimatePreview>,
    last_tick: Option<Instant>,
    /// Última malha deformada entregue ao renderer e sua revisão (só avança
    /// quando a geometria muda, para o renderer não reenviar buffers à toa).
    pose: Option<Arc<PoseOverride>>,
    pose_revision: u64,
}

impl AnimateRuntime {
    /// Quantas vezes o avaliador foi reconstruído (testes e telemetria).
    pub fn evaluator_rebuilds(&self) -> u64 {
        self.preview.borrow().rebuilds()
    }
}

/// Ação semântica do Animate. O markup emite `(ação, argumento, valor)` e
/// [`AnimateIntent::parse`] traduz; não há segunda tabela de ações.
#[derive(Debug, Clone, PartialEq)]
pub enum AnimateIntent {
    AddCreature(RigPresetKind),
    SelectCreature(String),
    AddMotion(MotionGenerator),
    SelectMotion(String),
    RemoveMotion,
    DuplicateMotion,
    /// Valor provisório enquanto o slider é arrastado.
    ParamPreview {
        key: String,
        value: f32,
    },
    /// Soltar o slider: um único passo de Undo.
    ParamCommit,
    /// Escolha discreta (padrão de passo): imediata.
    ParamSet {
        key: String,
        value: f32,
    },
    SetStyle(MotionStyle),
    SetStepped(bool),
    SetRootMotion(bool),
    TogglePlay,
    /// Posição do playhead como fração do ciclo (0..=1).
    Seek(f32),
    /// `keep = true` mantém o Motion vivo além do clipe gerado.
    ApplyNow {
        keep: bool,
    },
    ToggleAdvanced,
    ToggleBones,
    /// Ajusta a criatura em foco ao modelo ativo e os liga por skin.
    FitToModel,
    /// Seleciona um osso pelo ID ou desseleciona se `None`.
    SelectBone(Option<u32>),
    /// Reseta pose do osso (rotação e escala de volta ao repouso).
    ResetBonePose(Option<u32>),
    /// Liga/desliga modo IK no posing manual.
    SetIkMode(bool),
    /// Grava keyframe na pose atual do osso selecionado no playhead.
    InsertKeyframe,
    /// Remove keyframe do osso no tempo atual ou informado.
    DeleteKeyframe(Option<f32>),
    /// Altera a interpolação do keyframe.
    SetKeyframeInterp(Interpolation),
    /// Limpa a trilha do osso selecionado.
    ClearBoneTrack,
    /// Seleciona um keyframe específico no tempo e osso informados.
    SelectKeyframe {
        bone_id: u32,
        time: f32,
    },
    /// Avança ou recua frames.
    StepFrame(i32),
    /// Pula para o início (t=0.0).
    JumpToStart,
    /// Pula para o fim do ciclo/duração.
    JumpToEnd,
    /// Alterna modo de curvas no drawer.
    SetCurveMode(bool),
    /// Define o canal da curva ativa.
    SetCurveChannel(CurveChannel),
    /// Adiciona camada de animação ou tweak.
    AddMotionLayer {
        name: String,
        is_tweak: bool,
    },
    /// Remove camada de animação.
    RemoveMotionLayer(uuid::Uuid),
    /// Define peso da camada de animação.
    SetMotionLayerWeight {
        id: uuid::Uuid,
        weight: f32,
    },
    /// Define modo de blend da camada de animação.
    SetMotionLayerBlendMode {
        id: uuid::Uuid,
        additive: bool,
    },
    /// Alterna ativação da camada de animação.
    ToggleMotionLayerEnabled(uuid::Uuid),
    /// Alterna Onion Skinning no viewport.
    ToggleOnionSkin(bool),
    /// Alterna Motion Trails no viewport.
    ToggleMotionTrails(bool),
}

impl AnimateIntent {
    /// Traduz a tripla emitida pelo markup. `None` para ação desconhecida ou
    /// argumento inválido (a UI nunca derruba o bridge).
    pub fn parse(action: &str, arg: &str, value: f32) -> Option<Self> {
        Some(match action {
            "add-creature" => Self::AddCreature(
                RigPresetKind::CREATURES
                    .into_iter()
                    .find(|k| k.id() == arg)?,
            ),
            "select-creature" => Self::SelectCreature(arg.to_string()),
            "add-motion" => {
                Self::AddMotion(MotionGenerator::ALL.into_iter().find(|g| g.id() == arg)?)
            }
            "select-motion" => Self::SelectMotion(arg.to_string()),
            "remove-motion" => Self::RemoveMotion,
            "duplicate-motion" => Self::DuplicateMotion,
            "param-preview" if value.is_finite() => Self::ParamPreview {
                key: arg.to_string(),
                value,
            },
            "param-commit" => Self::ParamCommit,
            "param-set" if value.is_finite() => Self::ParamSet {
                key: arg.to_string(),
                value,
            },
            "set-style" => Self::SetStyle(
                STYLE_CHOICES
                    .into_iter()
                    .chain([MotionStyle::Custom])
                    .find(|s| s.id() == arg)?,
            ),
            "set-stepped" => Self::SetStepped(value > 0.5),
            "set-root-motion" => Self::SetRootMotion(value > 0.5),
            "toggle-play" => Self::TogglePlay,
            "seek" if value.is_finite() => Self::Seek(value.clamp(0.0, 1.0)),
            "apply-now" => Self::ApplyNow {
                keep: arg == "keep",
            },
            "toggle-advanced" => Self::ToggleAdvanced,
            "toggle-bones" => Self::ToggleBones,
            "fit-to-model" => Self::FitToModel,
            "select-bone" => {
                let id = if arg.is_empty() {
                    None
                } else {
                    arg.parse::<u32>().ok()
                };
                Self::SelectBone(id)
            }
            "reset-bone" => {
                let id = if arg.is_empty() {
                    None
                } else {
                    arg.parse::<u32>().ok()
                };
                Self::ResetBonePose(id)
            }
            "set-ik-mode" => Self::SetIkMode(value > 0.5),
            "insert-keyframe" => Self::InsertKeyframe,
            "delete-keyframe" => Self::DeleteKeyframe(if value.is_finite() && value >= 0.0 {
                Some(value)
            } else {
                None
            }),
            "set-keyframe-interp" => {
                let interp = match arg {
                    "step" => Interpolation::Step,
                    "cubic" => Interpolation::Cubic,
                    _ => Interpolation::Linear,
                };
                Self::SetKeyframeInterp(interp)
            }
            "clear-bone-track" => Self::ClearBoneTrack,
            "select-keyframe" => {
                let bone_id = arg.parse::<u32>().ok().unwrap_or(0);
                Self::SelectKeyframe {
                    bone_id,
                    time: value,
                }
            }
            "step-frame" => Self::StepFrame(value as i32),
            "jump-start" => Self::JumpToStart,
            "jump-end" => Self::JumpToEnd,
            "set-curve-mode" => Self::SetCurveMode(value > 0.5),
            "set-curve-channel" => {
                let channel = match arg {
                    "trans_y" => CurveChannel::TranslationY,
                    "trans_z" => CurveChannel::TranslationZ,
                    "rot_x" => CurveChannel::RotationX,
                    "rot_y" => CurveChannel::RotationY,
                    "rot_z" => CurveChannel::RotationZ,
                    "scale" => CurveChannel::ScaleX,
                    _ => CurveChannel::TranslationX,
                };
                Self::SetCurveChannel(channel)
            }
            "add-motion-layer" => Self::AddMotionLayer {
                name: if arg.is_empty() {
                    "Layer".into()
                } else {
                    arg.into()
                },
                is_tweak: false,
            },
            "add-tweak-layer" => Self::AddMotionLayer {
                name: if arg.is_empty() {
                    "Tweak".into()
                } else {
                    arg.into()
                },
                is_tweak: true,
            },
            "remove-motion-layer" => Self::RemoveMotionLayer(parse_uuid(arg).ok()?),
            "set-layer-weight" => Self::SetMotionLayerWeight {
                id: parse_uuid(arg).ok()?,
                weight: value,
            },
            "set-layer-blend-mode" => Self::SetMotionLayerBlendMode {
                id: parse_uuid(arg).ok()?,
                additive: value > 0.5,
            },
            "toggle-layer-enabled" => Self::ToggleMotionLayerEnabled(parse_uuid(arg).ok()?),
            "toggle-onion-skin" => Self::ToggleOnionSkin(value > 0.5),
            "toggle-motion-trails" => Self::ToggleMotionTrails(value > 0.5),
            _ => return None,
        })
    }
}

/// Linha de lista (criaturas, catálogo de Motions, Motions do projeto, estilos).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnimateRow {
    pub id: String,
    pub label: String,
    /// Descrição curta ou, se indisponível, o motivo.
    pub detail: String,
    /// Id de ícone estável (o markup mapeia para o desenho).
    pub icon: String,
    pub available: bool,
    pub selected: bool,
}

/// Um controle de parâmetro do Motion.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnimateParam {
    pub key: String,
    pub label: String,
    pub tip: String,
    pub minimum: f32,
    pub maximum: f32,
    pub value: f32,
    pub value_label: String,
    /// Pertence à camada "Advanced".
    pub advanced: bool,
    /// Não vazio ⇒ escolha discreta (chips) em vez de slider.
    pub choices: Vec<String>,
    pub choice_index: i32,
}

/// Textos fixos do chrome do Animate, já resolvidos no idioma ativo.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnimateTexts {
    pub workspace_label: String,
    pub workspace_tip: String,
    pub title_picker: String,
    pub title_creature: String,
    pub title_motion: String,
    pub title_style: String,
    pub add_creature: String,
    pub empty_creature: String,
    pub empty_motion: String,
    pub rig_error: String,
    pub advanced: String,
    pub show_bones: String,
    pub stepped: String,
    pub stepped_tip: String,
    pub root_motion: String,
    pub root_motion_tip: String,
    pub apply_now: String,
    pub apply_now_tip: String,
    pub keep_live: String,
    pub keep_live_tip: String,
    pub duplicate: String,
    pub remove: String,
    pub playhead: String,
    pub play: String,
    pub pause: String,
    pub play_tip: String,
    pub fit_model: String,
    pub linked_model: String,
    pub title_bone: String,
    pub reset_bone: String,
    pub ik_mode: String,
    pub insert_keyframe: String,
    pub delete_keyframe: String,
    pub interp_linear: String,
    pub interp_step: String,
    pub interp_cubic: String,
    pub clear_track: String,
    pub title_timeline: String,
    pub title_curves: String,
    pub title_dope_sheet: String,
    pub channel_trans_x: String,
    pub channel_trans_y: String,
    pub channel_trans_z: String,
    pub channel_rot_x: String,
    pub channel_rot_y: String,
    pub channel_rot_z: String,
    pub channel_scale: String,
    pub title_layers: String,
    pub add_layer: String,
    pub add_tweak: String,
    pub blend_override: String,
    pub blend_additive: String,
    pub layer_weight: String,
    pub onion_skin: String,
    pub motion_trails: String,
}

/// Um keyframe visualizado na Dope Sheet da Timeline.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnimateKeyframeModel {
    pub time: f32,
    pub frame: i32,
    pub normalized_pos: f32,
    pub selected: bool,
    pub interp_kind: i32,
}

/// Trilha de um osso visualizada na Dope Sheet da Timeline.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnimateTrackModel {
    pub bone_id: u32,
    pub bone_name: String,
    pub selected: bool,
    pub keyframes: Vec<AnimateKeyframeModel>,
}

/// Tudo que o painel Animate lê. Snapshot barato: nenhuma estrutura do core
/// vaza para o markup.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnimateViewModel {
    /// A pill ANIMATE existe (feature `animation-workspace`).
    pub available: bool,
    pub has_creature: bool,
    pub has_motion: bool,
    /// O Motion selecionado não roda neste rig.
    pub rig_error: bool,
    pub texts: AnimateTexts,
    pub creatures: Vec<AnimateRow>,
    pub creature_presets: Vec<AnimateRow>,
    pub catalog: Vec<AnimateRow>,
    pub motions: Vec<AnimateRow>,
    pub styles: Vec<AnimateRow>,
    pub params: Vec<AnimateParam>,
    pub playing: bool,
    /// Posição do playhead como fração do ciclo.
    pub progress: f32,
    pub time_label: String,
    pub show_advanced: bool,
    pub show_bones: bool,
    pub stepped: bool,
    pub root_motion: bool,
    pub root_motion_supported: bool,
    /// Ossos posados como comandos de `Path` em pixels da viewport.
    pub bone_commands: String,
    /// Articulações como cruzes pequenas (mesmo sistema de coordenadas).
    pub joint_commands: String,
    /// ID do osso atualmente selecionado para posing manual.
    pub selected_bone: Option<u32>,
    /// Nome amigável do osso selecionado.
    pub selected_bone_name: String,
    /// Contorno e destaque do osso selecionado como comandos de `Path`.
    pub selected_bone_commands: String,
    /// Modo IK ativado para posing.
    pub ik_mode: bool,
    /// Dá para ajustar a criatura ao modelo ativo (Fit to model)?
    pub can_fit: bool,
    /// Dica do botão: o que ele faz, ou por que está desabilitado.
    pub fit_tip: String,
    /// Nomes dos modelos já ligados à criatura em foco (separados por vírgula).
    pub linked_models: String,
    /// Pode inserir keyframe (tem criatura e osso selecionado).
    pub can_keyframe: bool,
    /// Existe keyframe na posição atual do playhead para o osso ativo.
    pub has_keyframe_at_playhead: bool,
    /// Rótulo descritivo do frame atual.
    pub frame_label: String,
    /// Typed frame coordinates for native consumers (24 fps).
    pub current_frame: i32,
    pub total_frames: i32,
    /// Tipo de interpolação ativa (0 = Step, 1 = Linear, 2 = Cubic).
    pub current_interp: i32,
    /// Pistas de ossos e keyframes para a Dope Sheet da Timeline.
    pub tracks: Vec<AnimateTrackModel>,
    /// Caminho SVG polyline da curva do canal ativo no Curve Editor (Fatia 3).
    pub curve_path: String,
    /// Pontos de controle na curva ativa (Fatia 3).
    pub curve_points: Vec<AnimateKeyframeModel>,
    /// Canal de curva ativo (0..2 Pos, 3..5 Rot, 6 Scale).
    pub active_curve_channel: i32,
    /// Camadas de animação e tweak layers (Fatia 4).
    pub layers: Vec<AnimateLayerModel>,
    /// Onion Skinning ativo.
    pub onion_skin_enabled: bool,
    /// Motion Trails ativo.
    pub motion_trails_enabled: bool,
    /// Comandos de Path dos esqueletos fantasmas (Fatia 5).
    pub ghost_commands: String,
    /// Comandos de Path do rastro de movimento 3D projetado (Fatia 5).
    pub motion_trail_commands: String,
}

/// Camada de movimento na pilha (Fatia 4).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnimateLayerModel {
    pub id: String,
    pub name: String,
    pub weight: f32,
    pub is_additive: bool,
    pub enabled: bool,
    pub is_tweak: bool,
    pub selected: bool,
}

/// Id de ícone de cada gerador.
pub fn generator_icon(generator: MotionGenerator) -> &'static str {
    match generator {
        MotionGenerator::BipedCycle => "footprints",
        MotionGenerator::Gait => "paw-print",
        MotionGenerator::Serpentine => "waves-horizontal",
        MotionGenerator::IdleBreath => "wind",
    }
}

/// Id de ícone de cada preset de criatura.
pub fn creature_icon(kind: RigPresetKind) -> &'static str {
    match kind {
        RigPresetKind::Humanoid => "person-standing",
        RigPresetKind::Quadruped => "paw-print",
        RigPresetKind::MultiLeg(_) => "bug",
        RigPresetKind::Serpent => "waves-horizontal",
        RigPresetKind::Fish => "fish",
        RigPresetKind::Bird => "bird",
    }
}

fn unavailable_text(reason: MotionUnavailable) -> petunia_config::TextId {
    match reason {
        MotionUnavailable::NoRig => T::ANIMATE_NO_RIG,
        MotionUnavailable::NeedsLegs { .. } => T::ANIMATE_NEEDS_LEGS,
        MotionUnavailable::NeedsChain { .. } => T::ANIMATE_NEEDS_CHAIN,
        MotionUnavailable::NeedsBody => T::ANIMATE_NEEDS_BODY,
    }
}

/// Formata um valor de slider: inteiros sem casas, o resto com duas.
fn value_label(value: f32) -> String {
    if (value - value.round()).abs() < 0.005 {
        format!("{}", value.round() as i64)
    } else {
        format!("{value:.2}")
    }
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// O Animate é o workspace ativo (sempre `false` sem a feature).
    pub fn animate_workspace_active(&self) -> bool {
        #[cfg(feature = "animation-workspace")]
        {
            self.state.session.workspace == petunia_core::Workspace::Animate
        }
        #[cfg(not(feature = "animation-workspace"))]
        {
            false
        }
    }

    fn animate_texts(&self) -> AnimateTexts {
        let t = |id| self.state.t_id(id);
        AnimateTexts {
            workspace_label: self.state.t("ws.animate"),
            workspace_tip: t(T::ANIMATE_WORKSPACE_TIP),
            title_picker: t(T::ANIMATE_TITLE_PICKER),
            title_creature: t(T::ANIMATE_TITLE_CREATURE),
            title_motion: t(T::ANIMATE_TITLE_MOTION),
            title_style: t(T::ANIMATE_TITLE_STYLE),
            add_creature: t(T::ANIMATE_ADD_CREATURE),
            empty_creature: t(T::ANIMATE_EMPTY_CREATURE),
            empty_motion: t(T::ANIMATE_EMPTY_MOTION),
            rig_error: t(T::ANIMATE_RIG_ERROR),
            advanced: t(T::ANIMATE_ADVANCED),
            show_bones: t(T::ANIMATE_SHOW_BONES),
            stepped: t(T::ANIMATE_STEPPED),
            stepped_tip: t(T::ANIMATE_STEPPED_TIP),
            root_motion: t(T::ANIMATE_ROOT_MOTION),
            root_motion_tip: t(T::ANIMATE_ROOT_MOTION_TIP),
            apply_now: t(T::ANIMATE_APPLY_NOW),
            apply_now_tip: t(T::ANIMATE_APPLY_NOW_TIP),
            keep_live: t(T::ANIMATE_KEEP_LIVE),
            keep_live_tip: t(T::ANIMATE_KEEP_LIVE_TIP),
            duplicate: t(T::ANIMATE_DUPLICATE),
            remove: t(T::ANIMATE_REMOVE),
            playhead: t(T::ANIMATE_PLAYHEAD),
            play: t(T::ANIMATE_PLAY),
            pause: t(T::ANIMATE_PAUSE),
            play_tip: t(T::ANIMATE_TIP_PLAY),
            fit_model: t(T::ANIMATE_FIT_MODEL),
            linked_model: t(T::ANIMATE_LINKED_MODEL),
            title_bone: t(T::ANIMATE_TITLE_BONE),
            reset_bone: t(T::ANIMATE_RESET_BONE),
            ik_mode: t(T::ANIMATE_IK_MODE),
            insert_keyframe: t(T::ANIMATE_INSERT_KEYFRAME),
            delete_keyframe: t(T::ANIMATE_DELETE_KEYFRAME),
            interp_linear: t(T::ANIMATE_INTERP_LINEAR),
            interp_step: t(T::ANIMATE_INTERP_STEP),
            interp_cubic: t(T::ANIMATE_INTERP_CUBIC),
            clear_track: t(T::ANIMATE_CLEAR_TRACK),
            title_timeline: t(T::ANIMATE_TITLE_TIMELINE),
            title_curves: t(T::ANIMATE_TITLE_CURVES),
            title_dope_sheet: t(T::ANIMATE_TITLE_DOPE_SHEET),
            channel_trans_x: t(T::ANIMATE_CHANNEL_TRANS_X),
            channel_trans_y: t(T::ANIMATE_CHANNEL_TRANS_Y),
            channel_trans_z: t(T::ANIMATE_CHANNEL_TRANS_Z),
            channel_rot_x: t(T::ANIMATE_CHANNEL_ROT_X),
            channel_rot_y: t(T::ANIMATE_CHANNEL_ROT_Y),
            channel_rot_z: t(T::ANIMATE_CHANNEL_ROT_Z),
            channel_scale: t(T::ANIMATE_CHANNEL_SCALE),
            title_layers: t(T::ANIMATE_TITLE_LAYERS),
            add_layer: t(T::ANIMATE_ADD_LAYER),
            add_tweak: t(T::ANIMATE_ADD_TWEAK),
            blend_override: t(T::ANIMATE_BLEND_OVERRIDE),
            blend_additive: t(T::ANIMATE_BLEND_ADDITIVE),
            layer_weight: t(T::ANIMATE_LAYER_WEIGHT),
            onion_skin: t(T::ANIMATE_ONION_SKIN),
            motion_trails: t(T::ANIMATE_MOTION_TRAILS),
        }
    }

    fn animate_param_rows(&self, recipe: &MotionRecipe) -> Vec<AnimateParam> {
        let session = &self.state.session.animate;
        recipe
            .generator
            .specs()
            .iter()
            .map(|spec| {
                let value = session.effective_param(recipe, spec.key);
                let choices: Vec<String> = spec
                    .choices
                    .iter()
                    .map(|c| self.state.t(&format!("animate.choice.{}_{c}", spec.key)))
                    .collect();
                let choice_index = if choices.is_empty() {
                    0
                } else {
                    (value.round() as i32).clamp(0, choices.len() as i32 - 1)
                };
                AnimateParam {
                    key: spec.key.to_string(),
                    label: self.state.t(&format!("animate.param.{}", spec.key)),
                    tip: self.state.t(&format!("animate.param_tip.{}", spec.key)),
                    minimum: spec.min,
                    maximum: spec.max,
                    value,
                    value_label: choices
                        .get(choice_index as usize)
                        .cloned()
                        .unwrap_or_else(|| value_label(value)),
                    advanced: spec.advanced,
                    choices,
                    choice_index,
                }
            })
            .collect()
    }

    /// Snapshot do painel Animate. Barato o bastante para o tick de reprodução:
    /// só a amostragem da pose usa o cache do avaliador.
    #[allow(clippy::collapsible_if)]
    pub fn animate_view_model(&self) -> AnimateViewModel {
        let st = &self.state;
        let project = &st.project.project;
        let session = &st.session.animate;
        let recipe = session.motion.and_then(|id| project.get_motion(id));

        let creature_presets = RigPresetKind::CREATURES
            .into_iter()
            .map(|kind| AnimateRow {
                id: kind.id().to_string(),
                label: st.t(&format!("animate.creature.{}", kind.id())),
                icon: creature_icon(kind).to_string(),
                available: true,
                ..Default::default()
            })
            .collect();
        let creatures = project
            .skeletons
            .iter()
            .map(|s| AnimateRow {
                id: s.id.to_string(),
                label: s.name.clone(),
                available: true,
                selected: session.skeleton == Some(s.id),
                ..Default::default()
            })
            .collect();
        let catalog = motion_catalog(project, session.skeleton)
            .into_iter()
            .map(|entry| {
                let id = entry.generator.id();
                AnimateRow {
                    id: id.to_string(),
                    label: st.t(&format!("animate.motion.{id}")),
                    detail: match entry.unavailable {
                        None => st.t(&format!("animate.motion_tip.{id}")),
                        Some(reason) => st.t_id(unavailable_text(reason)),
                    },
                    icon: generator_icon(entry.generator).to_string(),
                    available: entry.is_available(),
                    selected: recipe.is_some_and(|r| r.generator == entry.generator),
                }
            })
            .collect();
        let motions = project
            .motions
            .iter()
            .filter(|m| Some(m.skeleton_id) == session.skeleton)
            .map(|m| AnimateRow {
                id: m.id.to_string(),
                label: m.name.clone(),
                detail: st.t(&format!("animate.motion.{}", m.generator.id())),
                icon: generator_icon(m.generator).to_string(),
                available: true,
                selected: session.motion == Some(m.id),
            })
            .collect();
        let styles = STYLE_CHOICES
            .into_iter()
            .map(|style| AnimateRow {
                id: style.id().to_string(),
                label: st.t(&format!("animate.style.{}", style.id())),
                available: recipe.is_some(),
                selected: recipe.is_some_and(|r| r.style == style),
                ..Default::default()
            })
            .collect();

        let (cycle, rig_error, bones) = {
            let mut preview = self.animate.preview.borrow_mut();
            let cycle = preview.cycle_seconds(project, session);
            let rig_error = recipe.is_some() && preview.error().is_some();
            let pose = if session.show_skeleton {
                preview.pose(project, session)
            } else {
                None
            };
            (cycle, rig_error, pose)
        };
        let (bone_commands, joint_commands, selected_bone_commands) = bones
            .as_ref()
            .map(|pose| self.animate_pose_commands(pose))
            .unwrap_or_default();
        let progress = match cycle {
            Some(c) if c > f32::EPSILON => (session.time / c).clamp(0.0, 1.0),
            _ => 0.0,
        };

        let selected_bone = session.selected_bone;
        let ik_mode = session.ik_mode;
        let selected_bone_name = selected_bone
            .and_then(|bone_id| {
                session.skeleton.and_then(|skel_id| {
                    project
                        .skeletons
                        .iter()
                        .find(|s| s.id == skel_id)
                        .and_then(|s| s.bones.iter().find(|b| b.id == bone_id))
                        .map(|b| b.name.clone())
                })
            })
            .unwrap_or_default();

        let fit = fit_availability(project, session.skeleton);
        let fit_tip = st.t_id(match fit {
            Ok(()) => T::ANIMATE_FIT_MODEL_TIP,
            Err(FitBlocked::NoCreature) => T::ANIMATE_NO_RIG,
            Err(FitBlocked::NoModel) => T::ANIMATE_FIT_NEEDS_MODEL,
            Err(FitBlocked::Locked) => T::ANIMATE_FIT_LOCKED,
        });
        let linked_models = session.skeleton.map_or_else(String::new, |id| {
            project
                .assets
                .iter()
                .filter(|a| a.skin_data.as_ref().is_some_and(|s| s.skeleton_id == id))
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        });

        let fps = self.state.session.animate.timeline_frame_rate();
        let duration = cycle.unwrap_or(2.0).max(0.1);
        let current_frame = (session.time * fps).round() as i32;
        let total_frames = (duration * fps).round() as i32;
        let frame_label = format!(
            "Frame {} / {} ({:.2}s)",
            current_frame, total_frames, session.time
        );
        let can_keyframe = session.skeleton.is_some() && session.selected_bone.is_some();

        let active_asset = session.resolved_clip(project).ok().flatten();

        let mut has_keyframe_at_playhead = false;
        let mut current_interp = 1;

        let mut tracks = Vec::new();
        if let Some(skel_id) = session.skeleton {
            if let Some(skel) = project.get_skeleton(skel_id) {
                if let Some(asset) = active_asset {
                    if let Some(sel_bone) = session.selected_bone {
                        if let Some(tr) = asset.clip.get_track(sel_bone) {
                            if tr.has_keyframe_at(session.time, 0.05) {
                                has_keyframe_at_playhead = true;
                                if tr.translations.iter().any(|k| {
                                    (k.time - session.time).abs() < 0.05
                                        && k.interpolation == Interpolation::Step
                                }) || tr.rotations.iter().any(|k| {
                                    (k.time - session.time).abs() < 0.05
                                        && k.interpolation == Interpolation::Step
                                }) {
                                    current_interp = 0;
                                } else if tr.translations.iter().any(|k| {
                                    (k.time - session.time).abs() < 0.05
                                        && k.interpolation == Interpolation::Cubic
                                }) || tr.rotations.iter().any(|k| {
                                    (k.time - session.time).abs() < 0.05
                                        && k.interpolation == Interpolation::Cubic
                                }) {
                                    current_interp = 2;
                                }
                            }
                        }
                    }
                    for bone in &skel.bones {
                        let is_selected_bone = session.selected_bone == Some(bone.id);
                        let tr_opt = asset.clip.get_track(bone.id);
                        if tr_opt.is_some() || is_selected_bone {
                            let keyframes = tr_opt.map_or_else(Vec::new, |tr| {
                                tr.keyframe_times()
                                    .into_iter()
                                    .map(|t| {
                                        let interp_kind = if tr.translations.iter().any(|k| {
                                            (k.time - t).abs() < 1e-4
                                                && k.interpolation == Interpolation::Step
                                        }) || tr.rotations.iter().any(|k| {
                                            (k.time - t).abs() < 1e-4
                                                && k.interpolation == Interpolation::Step
                                        }) {
                                            0
                                        } else if tr.translations.iter().any(|k| {
                                            (k.time - t).abs() < 1e-4
                                                && k.interpolation == Interpolation::Cubic
                                        }) || tr.rotations.iter().any(|k| {
                                            (k.time - t).abs() < 1e-4
                                                && k.interpolation == Interpolation::Cubic
                                        }) {
                                            2
                                        } else {
                                            1
                                        };
                                        AnimateKeyframeModel {
                                            time: t,
                                            frame: (t * fps).round() as i32,
                                            normalized_pos: if duration > 1e-4 {
                                                (t / duration).clamp(0.0, 1.0)
                                            } else {
                                                0.0
                                            },
                                            selected: session.selected_keyframe
                                                == Some((bone.id, t))
                                                || (is_selected_bone
                                                    && (session.time - t).abs() < 0.05),
                                            interp_kind,
                                        }
                                    })
                                    .collect()
                            });
                            tracks.push(AnimateTrackModel {
                                bone_id: bone.id,
                                bone_name: bone.name.clone(),
                                selected: is_selected_bone,
                                keyframes,
                            });
                        }
                    }
                } else if let Some(bone_id) = session.selected_bone {
                    if let Some(bone) = skel.get_bone(bone_id) {
                        tracks.push(AnimateTrackModel {
                            bone_id: bone.id,
                            bone_name: bone.name.clone(),
                            selected: true,
                            keyframes: Vec::new(),
                        });
                    }
                }
            }
        }

        // Editor de Curvas (Fatia 3)
        let active_curve_channel_idx = match session.active_curve_channel {
            CurveChannel::TranslationX => 0,
            CurveChannel::TranslationY => 1,
            CurveChannel::TranslationZ => 2,
            CurveChannel::RotationX => 3,
            CurveChannel::RotationY => 4,
            CurveChannel::RotationZ => 5,
            _ => 6,
        };
        let mut curve_path = String::new();
        let mut curve_points = Vec::new();

        if let Some(sel_bone) = session.selected_bone {
            if let Some(asset) = active_asset {
                if let Some(tr) = asset.clip.get_track(sel_bone) {
                    let pts = tr.sample_curve(session.active_curve_channel, 0.0, duration, 32);
                    if pts.len() >= 2 {
                        let mut min_val = f32::INFINITY;
                        let mut max_val = f32::NEG_INFINITY;
                        for p in &pts {
                            min_val = min_val.min(p.value);
                            max_val = max_val.max(p.value);
                        }
                        let span = if (max_val - min_val).abs() > 1e-4 {
                            max_val - min_val
                        } else {
                            1.0
                        };
                        use std::fmt::Write as _;
                        for (i, p) in pts.iter().enumerate() {
                            let norm_x = if duration > 1e-4 {
                                p.time / duration
                            } else {
                                0.0
                            };
                            let norm_y = ((p.value - min_val) / span).clamp(0.0, 1.0);
                            let px = norm_x * 400.0;
                            let py = 100.0 - norm_y * 80.0;
                            if i == 0 {
                                let _ = write!(curve_path, "M {:.1} {:.1}", px, py);
                            } else {
                                let _ = write!(curve_path, " L {:.1} {:.1}", px, py);
                            }
                        }
                    }
                    for t in tr.keyframe_times() {
                        curve_points.push(AnimateKeyframeModel {
                            time: t,
                            frame: (t * fps).round() as i32,
                            normalized_pos: if duration > 1e-4 {
                                (t / duration).clamp(0.0, 1.0)
                            } else {
                                0.0
                            },
                            selected: session.selected_keyframe == Some((sel_bone, t)),
                            interp_kind: 1,
                        });
                    }
                }
            }
        }

        // Camadas de animação e blending (Fatia 4)
        let layers: Vec<AnimateLayerModel> = project
            .motion_layers
            .iter()
            .map(|l| AnimateLayerModel {
                id: l.id.to_string(),
                name: l.name.clone(),
                weight: l.blend_weight,
                is_additive: l.blend_mode == BlendMode::Additive,
                enabled: l.enabled,
                is_tweak: l.is_tweak,
                selected: session.selected_layer == Some(l.id),
            })
            .collect();

        // Fantasmas e Rastros de movimento no Viewport (Fatia 5)
        let mut ghost_commands = String::new();
        if session.onion_skin_enabled {
            let ghosts = self.animate.preview.borrow_mut().ghost_poses(
                project,
                session,
                &[-2, -1, 1, 2],
                fps,
            );
            for (_df, ghost_pose) in ghosts {
                for bone in &ghost_pose.bones {
                    let (head, tail) = (glam::Vec3::from(bone.head), glam::Vec3::from(bone.tail));
                    project_preview_segment(
                        &self.state.session.camera,
                        self.viewport_size,
                        head,
                        tail,
                        &mut ghost_commands,
                    );
                }
            }
        }

        let mut motion_trail_commands = String::new();
        if session.motion_trails_enabled
            && let Some(sel_bone) = session.selected_bone
        {
            let trail_pts = self
                .animate
                .preview
                .borrow_mut()
                .motion_trail(project, session, sel_bone, 24);
            let camera = &self.state.session.camera;
            let mut first = true;
            use std::fmt::Write as _;
            for pt in trail_pts {
                if let Some([x, y]) =
                    project_world_point(camera, self.viewport_size, glam::Vec3::from(pt))
                {
                    if first {
                        let _ = write!(motion_trail_commands, "M {:.1} {:.1}", x, y);
                        first = false;
                    } else {
                        let _ = write!(motion_trail_commands, " L {:.1} {:.1}", x, y);
                    }
                }
            }
        }

        AnimateViewModel {
            available: cfg!(feature = "animation-workspace"),
            has_creature: session.skeleton.is_some(),
            has_motion: recipe.is_some(),
            rig_error,
            texts: self.animate_texts(),
            creatures,
            creature_presets,
            catalog,
            motions,
            styles,
            params: recipe.map_or_else(Vec::new, |r| self.animate_param_rows(r)),
            playing: session.playing,
            progress,
            time_label: cycle.map_or_else(String::new, |c| {
                format!("{:.2} / {:.2}", session.time.min(c), c)
            }),
            show_advanced: session.show_advanced,
            show_bones: session.show_skeleton,
            stepped: recipe.is_some_and(|r| r.step_fps.is_some()),
            root_motion: recipe.is_some_and(|r| r.root_mode == RootMode::RootMotion),
            root_motion_supported: recipe.is_some_and(|r| r.generator.supports_root_motion()),
            bone_commands,
            joint_commands,
            selected_bone,
            selected_bone_name,
            selected_bone_commands,
            ik_mode,
            can_fit: fit.is_ok(),
            fit_tip,
            linked_models,
            can_keyframe,
            has_keyframe_at_playhead,
            frame_label,
            current_frame,
            total_frames,
            current_interp,
            tracks,
            curve_path,
            curve_points,
            active_curve_channel: active_curve_channel_idx,
            layers,
            onion_skin_enabled: session.onion_skin_enabled,
            motion_trails_enabled: session.motion_trails_enabled,
            ghost_commands,
            motion_trail_commands,
        }
    }

    /// Projeta o esqueleto posado para pixels da viewport: um segmento por osso
    /// e uma cruz por articulação. Reusa a projeção do bridge, sem GPU.
    pub(crate) fn animate_pose_commands(&self, pose: &PosePreview) -> (String, String, String) {
        use std::fmt::Write as _;
        let camera = &self.state.session.camera;
        let mut bones = String::new();
        let mut joints = String::new();
        let mut selected_bone_commands = String::new();
        let selected_id = self.state.session.animate.selected_bone;
        for bone in &pose.bones {
            let (head, tail) = (glam::Vec3::from(bone.head), glam::Vec3::from(bone.tail));
            project_preview_segment(camera, self.viewport_size, head, tail, &mut bones);
            if selected_id == Some(bone.id) {
                project_preview_segment(
                    camera,
                    self.viewport_size,
                    head,
                    tail,
                    &mut selected_bone_commands,
                );
                if let Some([x, y]) = project_world_point(camera, self.viewport_size, head) {
                    let _ = write!(
                        selected_bone_commands,
                        "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z ",
                        x,
                        y - 6.0,
                        x + 6.0,
                        y,
                        x,
                        y + 6.0,
                        x - 6.0,
                        y
                    );
                }
            }
            // Cruz de 3 px na articulação, pelos mesmos cortes de câmera.
            if let Some([x, y]) = project_world_point(camera, self.viewport_size, head) {
                let _ = write!(
                    joints,
                    "M {:.2} {:.2} L {:.2} {:.2} M {:.2} {:.2} L {:.2} {:.2} ",
                    x - 3.0,
                    y,
                    x + 3.0,
                    y,
                    x,
                    y - 3.0,
                    x,
                    y + 3.0
                );
            }
        }
        (bones, joints, selected_bone_commands)
    }

    /// Há um modelo do projeto ligado (por skin) à criatura em foco? Só então
    /// vale re-renderizar o viewport a cada quadro de animação.
    pub fn animate_has_posed_model(&self) -> bool {
        let project = &self.state.project.project;
        let Some(skeleton) = self.state.session.animate.skeleton else {
            return false;
        };
        project.assets.iter().any(|a| {
            a.visible
                && !a.has_enabled_modifiers()
                && a.skin_data
                    .as_ref()
                    .is_some_and(|s| s.skeleton_id == skeleton)
        })
    }

    /// Malhas deformadas pela pose atual, para o renderer desenhar no lugar do
    /// repouso. Fora do workspace Animate (ou sem modelo ligado) é `None` e o
    /// viewport mostra o documento como está. O documento nunca é alterado.
    pub fn animate_pose_override(&mut self) -> Option<Arc<PoseOverride>> {
        if !self.animate_workspace_active() {
            self.animate.pose = None;
            return None;
        }
        self.animate_pose_override_now()
    }

    /// Como [`Self::animate_pose_override`], sem olhar o workspace ativo.
    pub(crate) fn animate_pose_override_now(&mut self) -> Option<Arc<PoseOverride>> {
        let next = self.animate.pose_revision + 1;
        let fresh = self.animate.preview.get_mut().pose_override(
            &self.state.project.project,
            &self.state.session.animate,
            next,
        );
        let Some(fresh) = fresh else {
            self.animate.pose = None;
            return None;
        };
        if let Some(previous) = &self.animate.pose
            && previous.same_geometry(&fresh)
        {
            return Some(Arc::clone(previous));
        }
        self.animate.pose_revision = next;
        let shared = Arc::new(fresh);
        self.animate.pose = Some(Arc::clone(&shared));
        Some(shared)
    }

    /// Avança o playhead em `dt` segundos. `true` se a pose mudou ou se uma
    /// trilha de variantes de decalque trocou de variante (cap. 39: o
    /// Animate referencia `variant_index`).
    pub fn animate_advance(&mut self, dt: f32) -> bool {
        self.state.animate_resolve();
        // Projeta uma vez a sessão para preservar borrows disjuntos de
        // documento e transporte através do adapter da Application.
        let state: &mut petunia_core::AppState = &mut self.state;
        let project = &state.project.project;
        let posed = self
            .animate
            .preview
            .get_mut()
            .advance(project, &mut state.session.animate, dt);
        let time = self.state.session.animate.time;
        let decals = self.show_decal_time(Some(time));
        posed || decals
    }

    /// Tique do temporizador da UI (~30 Hz). Só anda com o Animate ativo e
    /// tocando; fora dele zera o relógio para não acumular `dt`.
    pub fn animate_tick(&mut self) -> bool {
        if !self.animate_workspace_active() || !self.state.session.animate.playing {
            self.animate.last_tick = None;
            return false;
        }
        let now = Instant::now();
        let dt = self
            .animate
            .last_tick
            .replace(now)
            .map_or(0.0, |previous| now.duration_since(previous).as_secs_f32())
            .min(MAX_TICK_SECONDS);
        self.animate_advance(dt)
    }

    /// Aplica uma ação do Animate. Erros de comando viram mensagem de status.
    pub fn apply_animate(&mut self, intent: AnimateIntent) {
        let result = self.run_animate(intent);
        match result {
            Ok(()) => {}
            // "Nada mudou" não é erro para o usuário.
            Err(CommandError::NoChange(_)) => {}
            Err(error) => self.state.set_status(error.to_string()),
        }
        self.state.mark_dirty();
    }

    fn run_animate(&mut self, intent: AnimateIntent) -> Result<(), CommandError> {
        use AnimateIntent as A;
        self.state.animate_resolve();
        let no_change = |changed: bool| {
            if changed {
                Ok(())
            } else {
                Err(CommandError::NoChange(String::new()))
            }
        };
        match intent {
            A::AddCreature(kind) => self.state.animate_add_rig(kind).map(|_| ()),
            A::SelectCreature(id) => {
                let id = parse_uuid(&id)?;
                no_change(self.state.animate_select_skeleton(id))
            }
            A::AddMotion(generator) => {
                if self.state.session.animate.skeleton.is_none() {
                    return Err(CommandError::Execution(self.state.t_id(T::ANIMATE_NO_RIG)));
                }
                self.state.animate_add_motion(generator).map(|_| ())
            }
            A::SelectMotion(id) => {
                let id = parse_uuid(&id)?;
                no_change(self.state.animate_select_motion(id))
            }
            A::RemoveMotion => self.state.animate_remove_motion(),
            A::DuplicateMotion => self.state.animate_duplicate_motion().map(|_| ()),
            A::ParamPreview { key, value } => {
                no_change(self.state.animate_param_preview(&key, value))
            }
            A::ParamCommit => self.state.animate_param_commit().and_then(no_change),
            A::ParamSet { key, value } => self
                .state
                .animate_set_param(&key, value)
                .and_then(no_change),
            A::SetStyle(style) => self.state.animate_set_style(style).and_then(no_change),
            A::SetStepped(on) => self
                .state
                .animate_set_stepped(on.then_some(12.0))
                .and_then(no_change),
            A::SetRootMotion(on) => self
                .state
                .animate_set_root_mode(if on {
                    RootMode::RootMotion
                } else {
                    RootMode::InPlace
                })
                .and_then(no_change),
            A::TogglePlay => {
                self.state.animate_toggle_play();
                Ok(())
            }
            A::Seek(progress) => {
                let cycle = self
                    .animate
                    .preview
                    .get_mut()
                    .cycle_seconds(&self.state.project.project, &self.state.session.animate)
                    .unwrap_or(0.0);
                self.state.animate_seek(progress * cycle);
                let time = self.state.session.animate.time;
                self.show_decal_time(Some(time));
                Ok(())
            }
            A::ApplyNow { keep } => self.state.animate_apply_now(keep),
            A::ToggleAdvanced => {
                let a = &mut self.state.session.animate;
                a.show_advanced = !a.show_advanced;
                Ok(())
            }
            A::FitToModel => self.state.animate_fit_to_model(),
            A::ToggleBones => {
                let a = &mut self.state.session.animate;
                a.show_skeleton = !a.show_skeleton;
                Ok(())
            }
            A::SelectBone(id) => {
                self.state.animate_select_bone(id);
                Ok(())
            }
            A::ResetBonePose(id) => {
                let target = id.or(self.state.session.animate.selected_bone);
                if let Some(bone_id) = target {
                    self.state.animate_reset_bone_pose(bone_id)
                } else {
                    Ok(())
                }
            }
            A::SetIkMode(ik) => {
                self.state.animate_set_ik_mode(ik);
                Ok(())
            }
            A::InsertKeyframe => self.state.animate_insert_keyframe(),
            A::DeleteKeyframe(time) => self.state.animate_delete_keyframe(time),
            A::SetKeyframeInterp(interp) => self.state.animate_set_keyframe_interp(interp),
            A::ClearBoneTrack => self.state.animate_clear_bone_track(),
            A::SelectKeyframe { bone_id, time } => {
                self.state.session.animate.selected_bone = Some(bone_id);
                self.state.session.animate.selected_keyframe = Some((bone_id, time));
                self.state.animate_seek(time);
                Ok(())
            }
            A::StepFrame(delta) => {
                let fps = self.state.session.animate.timeline_frame_rate();
                let dt = (delta as f32) / fps;
                let new_time = (self.state.session.animate.time + dt).max(0.0);
                self.state.animate_seek(new_time);
                Ok(())
            }
            A::JumpToStart => {
                self.state.animate_seek(0.0);
                Ok(())
            }
            A::JumpToEnd => {
                let cycle = self
                    .animate
                    .preview
                    .get_mut()
                    .cycle_seconds(&self.state.project.project, &self.state.session.animate)
                    .unwrap_or(0.0);
                self.state.animate_seek(cycle);
                Ok(())
            }
            A::SetCurveMode(active) => {
                self.state.animate_set_curve_mode(active);
                Ok(())
            }
            A::SetCurveChannel(channel) => {
                self.state.animate_set_curve_channel(channel);
                Ok(())
            }
            A::AddMotionLayer { name, is_tweak } => self
                .state
                .animate_add_motion_layer(&name, is_tweak)
                .map(|_| ()),
            A::RemoveMotionLayer(id) => self.state.animate_remove_motion_layer(id),
            A::SetMotionLayerWeight { id, weight } => {
                self.state.animate_set_motion_layer_weight(id, weight)
            }
            A::SetMotionLayerBlendMode { id, additive } => {
                let mode = if additive {
                    petunia_project::BlendMode::Additive
                } else {
                    petunia_project::BlendMode::Override
                };
                self.state.animate_set_motion_layer_blend_mode(id, mode)
            }
            A::ToggleMotionLayerEnabled(id) => self.state.animate_toggle_motion_layer_enabled(id),
            A::ToggleOnionSkin(enabled) => {
                self.state.animate_set_onion_skin(enabled);
                Ok(())
            }
            A::ToggleMotionTrails(enabled) => {
                self.state.animate_set_motion_trails(enabled);
                Ok(())
            }
        }
    }
}

fn parse_uuid(text: &str) -> Result<uuid::Uuid, CommandError> {
    uuid::Uuid::parse_str(text).map_err(|_| CommandError::Execution("Invalid id".into()))
}

#[cfg(test)]
mod parse_tests {
    use super::*;

    #[test]
    fn parse_maps_every_action_and_rejects_garbage() {
        assert_eq!(
            AnimateIntent::parse("add-creature", "bird", 0.0),
            Some(AnimateIntent::AddCreature(RigPresetKind::Bird))
        );
        assert_eq!(AnimateIntent::parse("add-creature", "dragon", 0.0), None);
        assert_eq!(
            AnimateIntent::parse("add-motion", "gait", 0.0),
            Some(AnimateIntent::AddMotion(MotionGenerator::Gait))
        );
        assert_eq!(AnimateIntent::parse("add-motion", "moonwalk", 0.0), None);
        assert_eq!(
            AnimateIntent::parse("param-preview", "energy", 1.5),
            Some(AnimateIntent::ParamPreview {
                key: "energy".into(),
                value: 1.5
            })
        );
        assert_eq!(
            AnimateIntent::parse("param-preview", "energy", f32::NAN),
            None,
            "valor não finito nunca chega ao domínio"
        );
        assert_eq!(
            AnimateIntent::parse("seek", "", 7.0),
            Some(AnimateIntent::Seek(1.0)),
            "o playhead é limitado a 0..=1"
        );
        assert_eq!(
            AnimateIntent::parse("set-style", "floaty", 0.0),
            Some(AnimateIntent::SetStyle(MotionStyle::Floaty))
        );
        assert_eq!(
            AnimateIntent::parse("apply-now", "keep", 0.0),
            Some(AnimateIntent::ApplyNow { keep: true })
        );
        assert_eq!(
            AnimateIntent::parse("apply-now", "convert", 0.0),
            Some(AnimateIntent::ApplyNow { keep: false })
        );
        assert_eq!(
            AnimateIntent::parse("fit-to-model", "", 0.0),
            Some(AnimateIntent::FitToModel)
        );
        assert_eq!(
            AnimateIntent::parse("set-curve-mode", "", 1.0),
            Some(AnimateIntent::SetCurveMode(true))
        );
        assert_eq!(
            AnimateIntent::parse("set-curve-channel", "rot_x", 0.0),
            Some(AnimateIntent::SetCurveChannel(CurveChannel::RotationX))
        );
        assert_eq!(
            AnimateIntent::parse("add-motion-layer", "Action", 0.0),
            Some(AnimateIntent::AddMotionLayer {
                name: "Action".into(),
                is_tweak: false
            })
        );
        assert_eq!(
            AnimateIntent::parse("add-tweak-layer", "", 0.0),
            Some(AnimateIntent::AddMotionLayer {
                name: "Tweak".into(),
                is_tweak: true
            })
        );
        assert_eq!(
            AnimateIntent::parse("toggle-onion-skin", "", 1.0),
            Some(AnimateIntent::ToggleOnionSkin(true))
        );
        assert_eq!(
            AnimateIntent::parse("toggle-motion-trails", "", 1.0),
            Some(AnimateIntent::ToggleMotionTrails(true))
        );
        assert_eq!(AnimateIntent::parse("explode", "", 0.0), None);
    }

    #[test]
    fn value_labels_are_short() {
        assert_eq!(value_label(1.0), "1");
        assert_eq!(value_label(0.5), "0.50");
        assert_eq!(value_label(2.999), "3");
        assert_eq!(value_label(0.254), "0.25");
    }
}

#[cfg(test)]
mod bridge_tests {
    use super::*;
    use crate::{PlaceholderViewport, UiIntent};
    use petunia_core::AppState;

    const EN: &str = include_str!("../../../assets/locales/en.toml");
    const PT: &str = include_str!("../../../assets/locales/pt-BR.toml");

    type Bridge = SlintUiBridge<PlaceholderViewport>;

    fn bridge() -> Bridge {
        SlintUiBridge::new(AppState::default(), PlaceholderViewport::default())
    }

    fn act(bridge: &mut Bridge, intent: AnimateIntent) {
        bridge.apply(UiIntent::Animate(intent));
    }

    fn with_walk() -> Bridge {
        let mut b = bridge();
        act(&mut b, AnimateIntent::AddCreature(RigPresetKind::Humanoid));
        act(
            &mut b,
            AnimateIntent::AddMotion(MotionGenerator::BipedCycle),
        );
        b
    }

    fn undo_depth(b: &Bridge) -> usize {
        b.state.project.undo.depth().0
    }

    #[test]
    fn empty_project_guides_the_user_towards_a_creature() {
        let b = bridge();
        let vm = b.animate_view_model();
        assert!(!vm.has_creature && !vm.has_motion);
        assert_eq!(vm.creature_presets.len(), RigPresetKind::CREATURES.len());
        assert!(vm.creature_presets.iter().all(|r| r.available));
        // Nada de Motion sem criatura: todos aparecem, desabilitados, com motivo.
        assert_eq!(vm.catalog.len(), MotionGenerator::ALL.len());
        assert!(vm.catalog.iter().all(|r| !r.available));
        assert!(
            vm.catalog
                .iter()
                .all(|r| r.detail == b.state.t_id(T::ANIMATE_NO_RIG))
        );
        assert!(vm.params.is_empty() && vm.bone_commands.is_empty());
        assert_eq!(vm.available, cfg!(feature = "animation-workspace"));
    }

    #[test]
    fn adding_a_creature_selects_it_and_explains_unavailable_motions() {
        let mut b = bridge();
        act(&mut b, AnimateIntent::AddCreature(RigPresetKind::Serpent));
        let vm = b.animate_view_model();
        assert!(vm.has_creature && !vm.has_motion);
        assert_eq!(vm.creatures.len(), 1);
        assert!(vm.creatures[0].selected);

        let row = |id: &str| vm.catalog.iter().find(|r| r.id == id).unwrap().clone();
        assert!(row("serpentine").available);
        assert!(row("idle_breath").available);
        let walk = row("biped_cycle");
        assert!(!walk.available);
        assert_eq!(walk.detail, b.state.t_id(T::ANIMATE_NEEDS_LEGS));
        // O que serve vem antes do que não serve.
        let first_off = vm.catalog.iter().position(|r| !r.available).unwrap();
        assert!(vm.catalog[first_off..].iter().all(|r| !r.available));
        // Styles só fazem sentido com um Motion.
        assert!(vm.styles.iter().all(|s| !s.available));
    }

    #[test]
    fn choosing_a_motion_creates_selects_and_plays_with_readable_sliders() {
        let b = with_walk();
        let vm = b.animate_view_model();
        assert!(vm.has_motion && vm.playing);
        assert_eq!(vm.motions.len(), 1);
        assert!(vm.motions[0].selected);
        assert!(
            vm.catalog
                .iter()
                .any(|r| r.id == "biped_cycle" && r.selected)
        );

        // Camada 1: os controles universais existem e não são "advanced".
        let basic: Vec<&str> = vm
            .params
            .iter()
            .filter(|p| !p.advanced)
            .map(|p| p.key.as_str())
            .collect();
        for key in ["speed", "energy", "weight", "stride", "lean", "smoothness"] {
            assert!(basic.contains(&key), "falta o slider universal {key}");
        }
        assert!(
            vm.params.iter().any(|p| p.advanced),
            "existe camada Advanced"
        );
        for p in &vm.params {
            assert!(
                !p.label.is_empty() && !p.label.starts_with("animate."),
                "{}",
                p.key
            );
            assert!(
                !p.tip.is_empty() && !p.tip.starts_with("animate."),
                "{}",
                p.key
            );
            assert!(p.minimum < p.maximum);
            assert!((p.minimum..=p.maximum).contains(&p.value));
        }
        assert!(vm.styles.iter().all(|s| s.available));
    }

    #[test]
    fn slider_gesture_previews_live_and_commits_one_undo_step() {
        let mut b = with_walk();
        let depth = undo_depth(&b);
        let energy = |b: &Bridge| {
            b.animate_view_model()
                .params
                .iter()
                .find(|p| p.key == "energy")
                .unwrap()
                .value
        };
        let before = energy(&b);

        for i in 0..25 {
            act(
                &mut b,
                AnimateIntent::ParamPreview {
                    key: "energy".into(),
                    value: 0.2 + i as f32 * 0.05,
                },
            );
        }
        assert_eq!(undo_depth(&b), depth, "arrastar não toca o histórico");
        assert!(
            (energy(&b) - 1.4).abs() < 1e-4,
            "o painel mostra o draft ao vivo"
        );

        act(&mut b, AnimateIntent::ParamCommit);
        assert_eq!(undo_depth(&b), depth + 1, "soltar cria um único passo");
        assert!((energy(&b) - 1.4).abs() < 1e-4);

        assert!(b.state.undo());
        assert!((energy(&b) - before).abs() < 1e-4);
    }

    #[test]
    fn style_chips_stepped_and_apply_now_go_through_commands() {
        let mut b = with_walk();
        act(&mut b, AnimateIntent::SetStyle(MotionStyle::Heavy));
        let vm = b.animate_view_model();
        assert!(vm.styles.iter().any(|s| s.id == "heavy" && s.selected));

        act(&mut b, AnimateIntent::SetStepped(true));
        assert!(b.animate_view_model().stepped);
        act(&mut b, AnimateIntent::SetStepped(false));
        assert!(!b.animate_view_model().stepped);

        assert!(b.animate_view_model().root_motion_supported);
        act(&mut b, AnimateIntent::SetRootMotion(true));
        assert!(b.animate_view_model().root_motion);

        let clips = b.state.project.project.animations.len();
        act(&mut b, AnimateIntent::ApplyNow { keep: true });
        assert_eq!(b.state.project.project.animations.len(), clips + 1);
        assert!(
            b.animate_view_model().has_motion,
            "Keep Live mantém o Motion"
        );
        act(&mut b, AnimateIntent::ApplyNow { keep: false });
        assert!(
            !b.animate_view_model().has_motion,
            "converter remove o Motion vivo"
        );
    }

    #[test]
    fn an_impossible_motion_is_refused_with_a_status_message() {
        let mut b = bridge();
        act(&mut b, AnimateIntent::AddCreature(RigPresetKind::Fish));
        let depth = undo_depth(&b);
        act(
            &mut b,
            AnimateIntent::AddMotion(MotionGenerator::BipedCycle),
        );
        assert_eq!(undo_depth(&b), depth);
        assert!(!b.animate_view_model().has_motion);
        assert!(!b.state.ui.status.is_empty(), "o usuário é avisado");
    }

    #[test]
    fn creatures_switch_and_selection_survives_undo() {
        let mut b = with_walk();
        act(&mut b, AnimateIntent::AddCreature(RigPresetKind::Bird));
        let vm = b.animate_view_model();
        assert_eq!(vm.creatures.len(), 2);
        assert!(vm.motions.is_empty(), "o pássaro ainda não tem Motion");
        let human = vm
            .creatures
            .iter()
            .find(|c| !c.selected)
            .unwrap()
            .id
            .clone();
        act(&mut b, AnimateIntent::SelectCreature(human));
        assert!(b.animate_view_model().has_motion);
        // Id inválido nunca derruba o bridge.
        act(&mut b, AnimateIntent::SelectCreature("não-é-uuid".into()));
        act(&mut b, AnimateIntent::SelectMotion("também-não".into()));

        // Undo do Motion: o painel se corrige sozinho.
        assert!(b.state.undo() || b.state.undo());
        let vm = b.animate_view_model();
        assert!(vm.motions.iter().all(|m| m.id.len() == 36));
    }

    #[test]
    fn playback_advances_wraps_and_pauses() {
        let mut b = with_walk();
        let t0 = b.animate_view_model().progress;
        assert!(b.animate_advance(0.1));
        assert!(b.animate_view_model().progress > t0);
        for _ in 0..400 {
            b.animate_advance(0.031);
            let p = b.animate_view_model().progress;
            assert!((0.0..=1.0).contains(&p), "progress={p}");
        }
        act(&mut b, AnimateIntent::TogglePlay);
        let p = b.animate_view_model().progress;
        assert!(!b.animate_advance(0.1), "pausado não anda");
        assert_eq!(b.animate_view_model().progress, p);

        // Scrub: posiciona e pausa, proporcional ao ciclo.
        act(&mut b, AnimateIntent::TogglePlay);
        act(&mut b, AnimateIntent::Seek(0.5));
        let vm = b.animate_view_model();
        assert!(!vm.playing);
        assert!((vm.progress - 0.5).abs() < 1e-3);
        assert!(!vm.time_label.is_empty());
    }

    #[test]
    fn the_tick_only_runs_inside_the_animate_workspace() {
        let mut b = with_walk();
        // Fora do workspace (ou sem a feature) o relógio não anda.
        if !b.animate_workspace_active() {
            assert!(!b.animate_tick());
            assert!(b.animate.last_tick.is_none());
        }
        #[cfg(feature = "animation-workspace")]
        {
            b.apply(UiIntent::SetWorkspace(petunia_core::Workspace::Animate));
            assert!(b.animate_workspace_active());
            // O primeiro tique só arma o relógio; o segundo já avança.
            b.animate_tick();
            std::thread::sleep(std::time::Duration::from_millis(20));
            assert!(
                b.animate_tick(),
                "com o Animate ativo e tocando, a pose anda"
            );
        }
    }

    #[test]
    fn posed_bones_are_projected_and_move_with_the_playhead() {
        let mut b = with_walk();
        b.viewport_size = [800.0, 600.0];
        act(&mut b, AnimateIntent::Seek(0.05));
        let a = b.animate_view_model();
        assert!(
            a.bone_commands.starts_with("M "),
            "ossos projetados na viewport"
        );
        assert!(a.joint_commands.starts_with("M "));
        act(&mut b, AnimateIntent::Seek(0.55));
        let c = b.animate_view_model();
        assert_ne!(a.bone_commands, c.bone_commands, "a pose muda no tempo");

        // Esconder os ossos limpa o overlay sem perder a criatura.
        act(&mut b, AnimateIntent::ToggleBones);
        let hidden = b.animate_view_model();
        assert!(hidden.bone_commands.is_empty() && hidden.joint_commands.is_empty());
        assert!(hidden.has_creature && !hidden.show_bones);
    }

    #[test]
    fn a_rig_without_a_motion_shows_its_rest_pose() {
        let mut b = bridge();
        b.viewport_size = [800.0, 600.0];
        act(&mut b, AnimateIntent::AddCreature(RigPresetKind::Quadruped));
        let vm = b.animate_view_model();
        assert!(!vm.has_motion);
        assert!(
            vm.bone_commands.starts_with("M "),
            "o esqueleto aparece de imediato"
        );
    }

    #[test]
    fn evaluator_is_cached_between_frames_and_playhead_moves() {
        let mut b = with_walk();
        b.viewport_size = [800.0, 600.0];
        let _ = b.animate_view_model();
        let rebuilds = b.animate.evaluator_rebuilds();
        for i in 0..30 {
            act(&mut b, AnimateIntent::Seek(i as f32 / 30.0));
            let _ = b.animate_view_model();
        }
        assert_eq!(
            b.animate.evaluator_rebuilds(),
            rebuilds,
            "mover o playhead não reconstrói o avaliador"
        );
    }

    /// Modelo low-poly ligado à criatura em foco (Fit to model), com Motion.
    fn with_walking_model() -> Bridge {
        let mut b = bridge();
        let mut mesh = petunia_core::Mesh::cube(1.0);
        for v in &mut mesh.verts {
            v.pos = [
                if v.pos[0] >= 0.0 { 0.4 } else { -0.4 },
                if v.pos[1] >= 0.0 { 1.9 } else { 0.0 },
                if v.pos[2] >= 0.0 { 0.2 } else { -0.2 },
            ];
        }
        b.state.project.project.add("Hero", mesh);
        act(&mut b, AnimateIntent::AddCreature(RigPresetKind::Humanoid));
        b.state.animate_fit_to_model().unwrap();
        act(
            &mut b,
            AnimateIntent::AddMotion(MotionGenerator::BipedCycle),
        );
        b
    }

    #[test]
    fn pose_override_exists_only_for_a_bound_model_and_only_when_it_changes() {
        // Sem modelo ligado não há nada a substituir.
        let mut plain = with_walk();
        assert!(!plain.animate_has_posed_model());
        assert!(plain.animate_pose_override_now().is_none());

        let mut b = with_walking_model();
        assert!(b.animate_has_posed_model());
        act(&mut b, AnimateIntent::Seek(0.1));
        let first = b.animate_pose_override_now().expect("modelo ligado");
        // Mesma pose ⇒ mesmo Arc e mesma revisão (o renderer não reenvia).
        let again = b.animate_pose_override_now().unwrap();
        assert!(Arc::ptr_eq(&first, &again));
        assert_eq!(first.revision, again.revision);

        act(&mut b, AnimateIntent::Seek(0.6));
        let moved = b.animate_pose_override_now().unwrap();
        assert!(moved.revision > first.revision, "pose nova ⇒ revisão nova");
        assert!(!moved.same_geometry(&first));

        // Esconder o modelo tira-o da pose.
        b.state.project.project.assets.last_mut().unwrap().visible = false;
        assert!(!b.animate_has_posed_model());
        assert!(b.animate_pose_override_now().is_none());
    }

    #[test]
    fn a_draft_slider_value_deforms_the_model_live() {
        let mut b = with_walking_model();
        act(&mut b, AnimateIntent::Seek(0.3));
        let before = b.animate_pose_override_now().unwrap();
        act(
            &mut b,
            AnimateIntent::ParamPreview {
                key: "stride".into(),
                value: 2.0,
            },
        );
        let live = b.animate_pose_override_now().unwrap();
        assert!(!live.same_geometry(&before), "o draft já muda a malha");
        let depth = b.state.project.undo.depth();
        act(&mut b, AnimateIntent::ParamCommit);
        assert_eq!(b.state.project.undo.depth().0, depth.0 + 1);
    }

    #[test]
    fn the_viewport_receives_the_pose_override_when_rendering() {
        #[derive(Default)]
        struct Recorder {
            last: Option<Option<Arc<PoseOverride>>>,
        }
        impl PetuniaViewport for Recorder {
            fn resize(&mut self, _: u32, _: u32) {}
            fn update(&mut self, _: f32) {}
            fn set_workspace(&mut self, _: petunia_core::Workspace) {}
            fn set_selection_domain(&mut self, _: petunia_core::SelectionDomain) {}
            fn set_pose_override(&mut self, pose: Option<Arc<PoseOverride>>) {
                self.last = Some(pose);
            }
        }
        let mut b = SlintUiBridge::new(AppState::default(), Recorder::default());
        b.render_viewport();
        assert_eq!(b.viewport.last.as_ref().map(Option::is_none), Some(true));

        #[cfg(feature = "animation-workspace")]
        {
            let mut mesh = petunia_core::Mesh::cube(1.0);
            for v in &mut mesh.verts {
                v.pos[1] += 1.0;
            }
            b.state.project.project.add("Hero", mesh);
            b.apply(UiIntent::SetWorkspace(petunia_core::Workspace::Animate));
            b.apply(UiIntent::Animate(AnimateIntent::AddCreature(
                RigPresetKind::Humanoid,
            )));
            b.state.animate_fit_to_model().unwrap();
            b.render_viewport();
            let sent = b.viewport.last.clone().flatten();
            assert!(
                sent.is_some_and(|p| !p.is_empty()),
                "no Animate o renderer recebe a pose"
            );
            // Saindo do Animate o viewport volta ao repouso do documento.
            b.apply(UiIntent::SetWorkspace(petunia_core::Workspace::Model));
            b.render_viewport();
            assert!(b.viewport.last.clone().flatten().is_none());
        }
    }

    #[test]
    fn fit_to_model_explains_itself_and_links_the_model_once() {
        let mut b = bridge();
        let vm = b.animate_view_model();
        assert!(!vm.can_fit);
        assert_eq!(vm.fit_tip, b.state.t_id(T::ANIMATE_NO_RIG));

        act(&mut b, AnimateIntent::AddCreature(RigPresetKind::Quadruped));
        let vm = b.animate_view_model();
        assert!(vm.can_fit, "há criatura e o modelo padrão do projeto");
        assert_eq!(vm.fit_tip, b.state.t_id(T::ANIMATE_FIT_MODEL_TIP));
        assert_eq!(vm.linked_models, "");
        assert!(!b.animate_has_posed_model());

        let depth = undo_depth(&b);
        act(&mut b, AnimateIntent::FitToModel);
        assert_eq!(undo_depth(&b), depth + 1, "um passo de Undo");
        let model = b.state.project.project.active().unwrap().name.clone();
        let vm = b.animate_view_model();
        assert_eq!(vm.linked_models, model);
        assert!(b.animate_has_posed_model());

        // Repetir não cria histórico nem mensagem de erro.
        b.state.set_status("");
        act(&mut b, AnimateIntent::FitToModel);
        assert_eq!(undo_depth(&b), depth + 1);
        assert!(b.state.ui.status.is_empty());

        // Undo desfaz a ligação.
        assert!(b.state.undo());
        assert_eq!(b.animate_view_model().linked_models, "");
    }

    #[test]
    fn fit_to_model_reports_a_locked_or_empty_model() {
        let mut b = bridge();
        act(&mut b, AnimateIntent::AddCreature(RigPresetKind::Bird));
        b.state.project.project.active_mut().unwrap().locked = true;
        let vm = b.animate_view_model();
        assert!(!vm.can_fit);
        assert_eq!(vm.fit_tip, b.state.t_id(T::ANIMATE_FIT_LOCKED));
        let depth = undo_depth(&b);
        act(&mut b, AnimateIntent::FitToModel);
        assert_eq!(undo_depth(&b), depth);
        assert!(!b.state.ui.status.is_empty(), "o usuário é avisado");

        b.state.project.project.active_mut().unwrap().locked = false;
        b.state
            .project
            .project
            .active_mut()
            .unwrap()
            .mesh
            .verts
            .clear();
        assert_eq!(
            b.animate_view_model().fit_tip,
            b.state.t_id(T::ANIMATE_FIT_NEEDS_MODEL)
        );
    }

    #[test]
    fn every_animate_text_key_exists_in_both_locales() {
        let en = petunia_config::I18n::parse(EN);
        let pt = petunia_config::I18n::parse(PT);
        let mut keys: Vec<String> = Vec::new();
        for g in MotionGenerator::ALL {
            keys.push(format!("animate.motion.{}", g.id()));
            keys.push(format!("animate.motion_tip.{}", g.id()));
            for spec in g.specs() {
                keys.push(format!("animate.param.{}", spec.key));
                keys.push(format!("animate.param_tip.{}", spec.key));
                for c in spec.choices {
                    keys.push(format!("animate.choice.{}_{c}", spec.key));
                }
            }
        }
        for s in STYLE_CHOICES.into_iter().chain([MotionStyle::Custom]) {
            keys.push(format!("animate.style.{}", s.id()));
        }
        for k in RigPresetKind::CREATURES {
            keys.push(format!("animate.creature.{}", k.id()));
        }
        keys.push("ws.animate".into());
        for key in keys {
            for (name, table) in [("en", &en), ("pt-BR", &pt)] {
                let text = table
                    .get(&key)
                    .unwrap_or_else(|| panic!("{name} sem {key}"));
                assert!(!text.trim().is_empty(), "{name}: {key} vazio");
            }
        }
    }

    #[test]
    fn labels_render_in_the_active_language() {
        let mut b = with_walk();
        let en = b.animate_view_model();
        let walk = en.catalog.iter().find(|r| r.id == "biped_cycle").unwrap();
        assert_eq!(walk.label, "Walk / Run");

        b.state.ui.i18n.set_lang("pt-BR");
        let pt = b.animate_view_model();
        let walk = pt.catalog.iter().find(|r| r.id == "biped_cycle").unwrap();
        assert_eq!(walk.label, "Andar / Correr");
        assert_eq!(pt.texts.apply_now, "Aplicar agora");
        assert_ne!(pt.texts, en.texts);
    }

    #[test]
    fn timeline_keyframe_intents_update_view_model_and_tracks() {
        let mut b = with_walk();
        let vm = b.animate_view_model();
        assert!(!vm.can_keyframe, "sem osso selecionado");

        let skel_id = b.state.session.animate.skeleton.unwrap();
        let bone_id = b.state.project.project.get_skeleton(skel_id).unwrap().bones[0].id;
        act(&mut b, AnimateIntent::SelectBone(Some(bone_id)));
        let vm = b.animate_view_model();
        assert!(vm.can_keyframe);
        assert!(!vm.has_keyframe_at_playhead);

        act(&mut b, AnimateIntent::InsertKeyframe);
        let vm = b.animate_view_model();
        assert!(vm.has_keyframe_at_playhead);
        assert!(!vm.tracks.is_empty());
        let track = vm.tracks.iter().find(|t| t.bone_id == bone_id).unwrap();
        assert_eq!(track.keyframes.len(), 1);

        act(
            &mut b,
            AnimateIntent::SetKeyframeInterp(Interpolation::Cubic),
        );
        let vm = b.animate_view_model();
        assert_eq!(vm.current_interp, 2);

        act(&mut b, AnimateIntent::DeleteKeyframe(None));
        let vm = b.animate_view_model();
        assert!(!vm.has_keyframe_at_playhead);
    }

    #[test]
    fn curve_editor_intents_and_sampling() {
        let mut b = with_walk();
        let skel_id = b.state.session.animate.skeleton.unwrap();
        let bone_id = b.state.project.project.get_skeleton(skel_id).unwrap().bones[0].id;
        act(&mut b, AnimateIntent::SelectBone(Some(bone_id)));
        act(&mut b, AnimateIntent::InsertKeyframe);

        act(&mut b, AnimateIntent::SetCurveMode(true));
        act(
            &mut b,
            AnimateIntent::SetCurveChannel(CurveChannel::RotationX),
        );

        let vm = b.animate_view_model();
        assert_eq!(vm.active_curve_channel, 3);
        assert!(!vm.curve_path.is_empty(), "caminho de curva gerado");
        assert!(!vm.curve_points.is_empty(), "pontos de curva gerados");
    }

    #[test]
    fn motion_layers_and_tweak_blending_with_undo() {
        let mut b = with_walk();
        let depth_before = undo_depth(&b);

        act(
            &mut b,
            AnimateIntent::AddMotionLayer {
                name: "Secondary".into(),
                is_tweak: false,
            },
        );
        act(
            &mut b,
            AnimateIntent::AddMotionLayer {
                name: "TweakArm".into(),
                is_tweak: true,
            },
        );
        assert_eq!(
            undo_depth(&b),
            depth_before + 2,
            "dois passos de Undo adicionados"
        );

        let vm = b.animate_view_model();
        assert_eq!(vm.layers.len(), 2);
        assert!(!vm.layers[0].is_tweak);
        assert!(vm.layers[1].is_tweak);

        let layer_id = uuid::Uuid::parse_str(&vm.layers[0].id).unwrap();
        act(
            &mut b,
            AnimateIntent::SetMotionLayerWeight {
                id: layer_id,
                weight: 0.75,
            },
        );
        act(
            &mut b,
            AnimateIntent::SetMotionLayerBlendMode {
                id: layer_id,
                additive: true,
            },
        );
        act(&mut b, AnimateIntent::ToggleMotionLayerEnabled(layer_id));

        let vm = b.animate_view_model();
        let layer = vm
            .layers
            .iter()
            .find(|l| l.id == layer_id.to_string())
            .unwrap();
        assert_eq!(layer.weight, 0.75);
        assert!(layer.is_additive);
        assert!(!layer.enabled);

        assert!(b.state.undo(), "desfaz toggle");
        let vm = b.animate_view_model();
        assert!(
            vm.layers
                .iter()
                .find(|l| l.id == layer_id.to_string())
                .unwrap()
                .enabled
        );

        act(&mut b, AnimateIntent::RemoveMotionLayer(layer_id));
        let vm = b.animate_view_model();
        assert_eq!(vm.layers.len(), 1);
    }

    #[test]
    fn onion_skin_and_motion_trails_toggle() {
        let mut b = with_walk();
        let vm = b.animate_view_model();
        assert!(!vm.onion_skin_enabled);
        assert!(!vm.motion_trails_enabled);

        act(&mut b, AnimateIntent::ToggleOnionSkin(true));
        act(&mut b, AnimateIntent::ToggleMotionTrails(true));

        let vm = b.animate_view_model();
        assert!(vm.onion_skin_enabled);
        assert!(vm.motion_trails_enabled);

        let skel_id = b.state.session.animate.skeleton.unwrap();
        let bone_id = b.state.project.project.get_skeleton(skel_id).unwrap().bones[0].id;
        act(&mut b, AnimateIntent::SelectBone(Some(bone_id)));

        let vm = b.animate_view_model();
        assert!(!vm.ghost_commands.is_empty(), "fantasmas temporais gerados");
        assert!(
            !vm.motion_trail_commands.is_empty(),
            "rastro de movimento gerado"
        );
    }
}
