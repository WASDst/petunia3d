//! Gramática única de ferramenta (constituição 11, ADR 007).
//!
//! Uma máquina de estados para todas as ferramentas de operação, independente
//! de toolkit. O frontend traduz eventos físicos em entradas neutras
//! (`press`, `move_to`, `release`, `key`, `text`) e executa os [`ToolEffect`]s
//! devolvidos. O mesmo código atende:
//!
//! - arrastar pela alça ou em qualquer lugar do viewport ("haul");
//! - clicar-mover-clicar (`Latched`), alternativa sem arrasto (WCAG 2.2, 2.5.7)
//!   e também o modal estilo Blender iniciado por teclado;
//! - valor digitado durante o gesto, que vence o mouse.
//!
//! [`LastOperation`] guarda o gesto confirmado para ajuste posterior dentro da
//! mesma entrada de Undo.

use glam::Vec3;

use crate::AppState;
use crate::modal::{ModalConstraint, ModalError, ModalKind};

/// Limiar padrão, em pixels lógicos, entre clique e arrasto.
pub const DEFAULT_DRAG_THRESHOLD_PX: f32 = 4.0;
/// Limites aceitos para o limiar configurável.
pub const DRAG_THRESHOLD_RANGE: (f32, f32) = (2.0, 16.0);
const NUMERIC_BUFFER_LIMIT: usize = 32;

/// O que estava sob o ponteiro quando o botão principal foi pressionado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressTarget {
    /// Uma alça da ferramenta (índice definido pelo frontend).
    Handle(u8),
    /// Qualquer outro ponto do viewport: o arrasto ajusta o valor principal.
    Surface,
}

/// Fase atual do gesto.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolPhase {
    Idle,
    /// Botão pressionado, ainda abaixo do limiar de arrasto.
    Pressed {
        anchor: [f32; 2],
        target: PressTarget,
    },
    /// Arrasto com o botão pressionado.
    Dragging {
        anchor: [f32; 2],
        target: PressTarget,
    },
    /// Segue o ponteiro sem botão pressionado; o próximo clique confirma.
    Latched {
        anchor: [f32; 2],
        target: PressTarget,
    },
}

/// Teclas semânticas da ferramenta. O keymap traduz as teclas físicas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKey {
    Confirm,
    Cancel,
    Backspace,
}

/// Ação que o frontend deve executar em resposta a uma entrada.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolEffect {
    Nothing,
    /// Clique sem movimento: selecionar (ou adicionar ponto) nesta posição.
    Click {
        at: [f32; 2],
    },
    /// Começou um gesto: abrir a operação e aplicar `current`.
    BeginDrag {
        anchor: [f32; 2],
        current: [f32; 2],
        target: PressTarget,
    },
    /// Novo ponto do gesto em andamento.
    UpdateDrag {
        anchor: [f32; 2],
        current: [f32; 2],
    },
    /// Confirmar o gesto como uma transação.
    Commit,
    /// Restaurar exatamente o estado anterior ao gesto.
    Cancel,
    /// Valor digitado durante o gesto.
    TypedValue(f32),
    /// Texto digitado esvaziado: o ponteiro volta a controlar o valor.
    TypedCleared,
    /// `Esc` sem gesto: sair da ferramenta (escada: ferramenta → Select).
    ExitTool,
    /// Botão secundário fora de gesto: abrir o menu de contexto.
    OpenContextMenu {
        at: [f32; 2],
    },
}

/// Máquina de estados da gramática única.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolSession {
    phase: ToolPhase,
    drag_threshold_px: f32,
    click_move_click: bool,
    numeric: String,
}

impl Default for ToolSession {
    fn default() -> Self {
        Self::new(DEFAULT_DRAG_THRESHOLD_PX, false)
    }
}

impl ToolSession {
    pub fn new(drag_threshold_px: f32, click_move_click: bool) -> Self {
        Self {
            phase: ToolPhase::Idle,
            drag_threshold_px: clamp_threshold(drag_threshold_px),
            click_move_click,
            numeric: String::new(),
        }
    }

    pub fn phase(&self) -> ToolPhase {
        self.phase
    }

    /// Há um gesto em andamento (arrastando ou seguindo o ponteiro).
    pub fn is_gesture_active(&self) -> bool {
        matches!(
            self.phase,
            ToolPhase::Dragging { .. } | ToolPhase::Latched { .. }
        )
    }

    pub fn is_latched(&self) -> bool {
        matches!(self.phase, ToolPhase::Latched { .. })
    }

    pub fn drag_threshold_px(&self) -> f32 {
        self.drag_threshold_px
    }

    pub fn set_drag_threshold_px(&mut self, threshold: f32) {
        self.drag_threshold_px = clamp_threshold(threshold);
    }

    pub fn click_move_click(&self) -> bool {
        self.click_move_click
    }

    pub fn set_click_move_click(&mut self, enabled: bool) {
        self.click_move_click = enabled;
    }

    /// Texto digitado no gesto atual (vazio quando o ponteiro controla).
    pub fn numeric_text(&self) -> &str {
        &self.numeric
    }

    /// Botão principal pressionado.
    pub fn press(&mut self, at: [f32; 2], target: PressTarget) -> ToolEffect {
        if !is_finite_point(at) {
            return ToolEffect::Nothing;
        }
        match self.phase {
            ToolPhase::Idle => {
                self.numeric.clear();
                self.phase = ToolPhase::Pressed { anchor: at, target };
                ToolEffect::Nothing
            }
            // No modo clicar-mover-clicar, o clique seguinte confirma.
            ToolPhase::Latched { .. } => self.finish(ToolEffect::Commit),
            ToolPhase::Pressed { .. } | ToolPhase::Dragging { .. } => ToolEffect::Nothing,
        }
    }

    /// Ponteiro moveu (com ou sem botão pressionado).
    pub fn move_to(&mut self, at: [f32; 2]) -> ToolEffect {
        if !is_finite_point(at) {
            return ToolEffect::Nothing;
        }
        match self.phase {
            ToolPhase::Pressed { anchor, target } => {
                let distance = (at[0] - anchor[0]).hypot(at[1] - anchor[1]);
                if distance < self.drag_threshold_px {
                    return ToolEffect::Nothing;
                }
                self.phase = ToolPhase::Dragging { anchor, target };
                ToolEffect::BeginDrag {
                    anchor,
                    current: at,
                    target,
                }
            }
            ToolPhase::Dragging { anchor, .. } | ToolPhase::Latched { anchor, .. } => {
                if self.numeric.is_empty() {
                    ToolEffect::UpdateDrag {
                        anchor,
                        current: at,
                    }
                } else {
                    // O valor digitado vence o mouse (constituição 11).
                    ToolEffect::Nothing
                }
            }
            ToolPhase::Idle => ToolEffect::Nothing,
        }
    }

    /// Botão principal solto.
    pub fn release(&mut self, at: [f32; 2]) -> ToolEffect {
        match self.phase {
            ToolPhase::Pressed { anchor, target } => {
                if self.click_move_click && matches!(target, PressTarget::Handle(_)) {
                    // Clique numa alça sem arrastar: a alça passa a seguir o
                    // ponteiro até o próximo clique (WCAG 2.5.7).
                    self.phase = ToolPhase::Latched { anchor, target };
                    ToolEffect::BeginDrag {
                        anchor,
                        current: if is_finite_point(at) { at } else { anchor },
                        target,
                    }
                } else {
                    self.phase = ToolPhase::Idle;
                    ToolEffect::Click {
                        at: if is_finite_point(at) { at } else { anchor },
                    }
                }
            }
            ToolPhase::Dragging { .. } => self.finish(ToolEffect::Commit),
            ToolPhase::Latched { .. } | ToolPhase::Idle => ToolEffect::Nothing,
        }
    }

    /// Botão secundário. Nunca cancela: durante um gesto é ignorado.
    pub fn secondary(&mut self, at: [f32; 2]) -> ToolEffect {
        if self.is_gesture_active() || matches!(self.phase, ToolPhase::Pressed { .. }) {
            ToolEffect::Nothing
        } else {
            ToolEffect::OpenContextMenu { at }
        }
    }

    /// Entrada por teclado no gesto que segue o ponteiro (modal estilo
    /// Blender ou preferência "arrastar sem segurar").
    pub fn begin_latched(&mut self, at: [f32; 2], target: PressTarget) -> ToolEffect {
        if !is_finite_point(at) || self.is_gesture_active() {
            return ToolEffect::Nothing;
        }
        self.numeric.clear();
        self.phase = ToolPhase::Latched { anchor: at, target };
        ToolEffect::BeginDrag {
            anchor: at,
            current: at,
            target,
        }
    }

    pub fn key(&mut self, key: ToolKey) -> ToolEffect {
        match key {
            ToolKey::Cancel => match self.phase {
                ToolPhase::Dragging { .. } | ToolPhase::Latched { .. } => {
                    self.finish(ToolEffect::Cancel)
                }
                ToolPhase::Pressed { .. } => {
                    self.phase = ToolPhase::Idle;
                    ToolEffect::Nothing
                }
                ToolPhase::Idle => ToolEffect::ExitTool,
            },
            ToolKey::Confirm => {
                if self.is_gesture_active() {
                    self.finish(ToolEffect::Commit)
                } else {
                    ToolEffect::Nothing
                }
            }
            ToolKey::Backspace => {
                if !self.is_gesture_active() || self.numeric.pop().is_none() {
                    return ToolEffect::Nothing;
                }
                match parse_numeric(&self.numeric) {
                    Some(value) => ToolEffect::TypedValue(value),
                    None if self.numeric.is_empty() => ToolEffect::TypedCleared,
                    None => ToolEffect::Nothing,
                }
            }
        }
    }

    /// Caractere digitado. Só é aceito durante um gesto.
    pub fn text(&mut self, c: char) -> ToolEffect {
        if !self.is_gesture_active()
            || !(c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '+'))
            || self.numeric.len() >= NUMERIC_BUFFER_LIMIT
        {
            return ToolEffect::Nothing;
        }
        self.numeric.push(if c == ',' { '.' } else { c });
        match parse_numeric(&self.numeric) {
            Some(value) => ToolEffect::TypedValue(value),
            None => ToolEffect::Nothing,
        }
    }

    /// Buffer numérico único de todas as ferramentas (constituição 11): o
    /// frontend pode acrescentar expressões (`10 + 5`, `*2`) além dos dígitos
    /// aceitos por [`Self::text`], com ou sem gesto ativo. Retorna se aceitou.
    pub fn push_numeric(&mut self, text: &str) -> bool {
        let text = text.replace(',', ".");
        if text.chars().any(char::is_control)
            || self.numeric.len() + text.len() > NUMERIC_BUFFER_LIMIT
        {
            return false;
        }
        self.numeric.push_str(&text);
        true
    }

    /// Apaga o último caractere do buffer numérico.
    pub fn pop_numeric(&mut self) -> bool {
        self.numeric.pop().is_some()
    }

    /// Esvazia o buffer numérico sem mexer na fase do gesto.
    pub fn clear_numeric(&mut self) {
        self.numeric.clear();
    }

    /// Encerra o gesto sem efeito de documento (troca de ferramenta, reset).
    pub fn reset(&mut self) {
        self.phase = ToolPhase::Idle;
        self.numeric.clear();
    }

    fn finish(&mut self, effect: ToolEffect) -> ToolEffect {
        self.phase = ToolPhase::Idle;
        // O buffer nunca vaza para a operação seguinte.
        self.numeric.clear();
        effect
    }
}

fn clamp_threshold(threshold: f32) -> f32 {
    if threshold.is_finite() {
        threshold.clamp(DRAG_THRESHOLD_RANGE.0, DRAG_THRESHOLD_RANGE.1)
    } else {
        DEFAULT_DRAG_THRESHOLD_PX
    }
}

fn is_finite_point(point: [f32; 2]) -> bool {
    point[0].is_finite() && point[1].is_finite()
}

fn parse_numeric(text: &str) -> Option<f32> {
    let value = text.parse::<f32>().ok()?;
    value.is_finite().then_some(value)
}

/// Geometria de tela de uma operação, em pixels lógicos, usada para traduzir
/// o arrasto no valor principal seguindo o cursor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragFrame {
    /// Pivô projetado na tela.
    pub pivot_px: [f32; 2],
    /// Deslocamento em tela de uma unidade de mundo ao longo da normal, se a
    /// normal não estiver quase alinhada à direção da vista.
    pub normal_px_per_unit: Option<[f32; 2]>,
    /// Unidades de mundo por pixel na profundidade do pivô.
    pub world_per_pixel: f32,
}

/// Menor comprimento em tela (px) de uma unidade de normal para usar a
/// projeção; abaixo disso a normal aponta quase para a câmera.
const MIN_NORMAL_PX_PER_UNIT: f32 = 6.0;

/// Valor principal de uma operação paramétrica a partir do arrasto, seguindo
/// o cursor e respeitando o zoom. `start_value` é o valor no início do gesto.
pub fn drag_value(
    kind: ModalKind,
    frame: DragFrame,
    anchor: [f32; 2],
    current: [f32; 2],
    start_value: f32,
) -> f32 {
    let delta = [current[0] - anchor[0], current[1] - anchor[1]];
    let world_per_pixel = if frame.world_per_pixel.is_finite() && frame.world_per_pixel > 0.0 {
        frame.world_per_pixel
    } else {
        0.01
    };
    match kind {
        ModalKind::Extrude | ModalKind::ExtrudeIndividual | ModalKind::PushPull => {
            match frame.normal_px_per_unit {
                Some(normal) if normal[0].hypot(normal[1]) >= MIN_NORMAL_PX_PER_UNIT => {
                    let length_sq = normal[0] * normal[0] + normal[1] * normal[1];
                    start_value + (delta[0] * normal[0] + delta[1] * normal[1]) / length_sq
                }
                // Normal apontando para a câmera: arrastar para cima aumenta.
                _ => start_value - delta[1] * world_per_pixel,
            }
        }
        ModalKind::Inset => {
            // Distância de mundo: aproximar o cursor do pivô aumenta o inset
            // (o limite anti-interseção é aplicado pela geometria).
            let (away, _) = away_from_pivot(frame.pivot_px, anchor, delta);
            (start_value - away * world_per_pixel).max(0.0)
        }
        ModalKind::Bevel => {
            // Distância de mundo: afastar o cursor do pivô aumenta.
            let (away, _) = away_from_pivot(frame.pivot_px, anchor, delta);
            (start_value + away * world_per_pixel).max(0.0)
        }
        ModalKind::Scale => {
            let start = distance(frame.pivot_px, anchor).max(8.0);
            let now = distance(frame.pivot_px, current);
            start_value * (now / start).max(0.01)
        }
        ModalKind::Move | ModalKind::Rotate => start_value,
    }
}

/// Componente do deslocamento que se afasta do pivô e a distância inicial.
fn away_from_pivot(pivot: [f32; 2], anchor: [f32; 2], delta: [f32; 2]) -> (f32, f32) {
    let reach = distance(pivot, anchor);
    let direction = if reach > 1.0e-3 {
        [
            (anchor[0] - pivot[0]) / reach,
            (anchor[1] - pivot[1]) / reach,
        ]
    } else {
        [0.0, -1.0]
    };
    (delta[0] * direction[0] + delta[1] * direction[1], reach)
}

fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// Gesto confirmado, ajustável até outra mudança no documento.
#[derive(Debug, Clone, PartialEq)]
pub struct LastOperation {
    pub kind: ModalKind,
    pub constraint: ModalConstraint,
    /// Valor principal: distância, graus, fator ou fração.
    pub value: f32,
    pub components: Vec3,
    normal: Vec3,
    history_depth: (usize, usize),
    revision_clock: [u64; 11],
    /// Documento logo após o prelúdio (imprint/forma nova), para reaplicar o
    /// gesto sem refazer a detecção de região.
    prelude: Option<PreludeState>,
    flip_when_negative: bool,
}

/// Documento após o prelúdio de um gesto; comparado por identidade.
#[derive(Debug, Clone)]
struct PreludeState(std::sync::Arc<(petunia_project::Project, crate::Selection)>);

impl PartialEq for PreludeState {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}

impl LastOperation {
    /// Valor exibido no card: para Move, a distância percorrida.
    pub fn primary_value(&self) -> f32 {
        match (self.kind, self.constraint) {
            (ModalKind::Move, ModalConstraint::Axis(index)) => self.components[index.min(2)],
            (ModalKind::Move, _) => self.components.length(),
            _ => self.value,
        }
    }

    /// Deslocamento e escalar para reaplicar a operação com `value`.
    fn inputs_for(&self, value: f32) -> Result<(Vec3, f32), ModalError> {
        match self.kind {
            ModalKind::Move => match self.constraint {
                ModalConstraint::Axis(index) => {
                    let mut axis = Vec3::ZERO;
                    axis[index.min(2)] = 1.0;
                    Ok((axis * value, value))
                }
                _ => {
                    let direction = self.components.normalize_or_zero();
                    if direction == Vec3::ZERO {
                        return Err(ModalError::InvalidInput);
                    }
                    Ok((direction * value, value))
                }
            },
            _ => Ok((Vec3::ZERO, value)),
        }
    }
}

impl AppState {
    /// Confirma a operação modal ativa como um gesto e devolve a "Última
    /// operação" ajustável. `None` quando nada mudou (sem entrada de Undo).
    pub fn commit_modal_gesture(&mut self) -> Option<LastOperation> {
        let flip_when_negative = self.modal_flips_when_negative();
        let prelude = self
            .modal_prelude_state()
            .map(|state| PreludeState(std::sync::Arc::new(state)));
        let modal = self.modal.as_ref()?;
        let changed = modal.changed();
        let (kind, constraint, value, components, normal) = (
            modal.kind,
            modal.constraint,
            modal.value,
            modal.components,
            modal.normal,
        );
        self.commit_modal();
        changed.then(|| LastOperation {
            kind,
            constraint,
            value,
            components,
            normal,
            history_depth: self.project.undo.depth(),
            revision_clock: self.project.project.revision_clock(),
            prelude,
            flip_when_negative,
        })
    }

    /// A última operação ainda é o topo do histórico e nada mudou desde então.
    pub fn last_operation_is_current(&self, last: &LastOperation) -> bool {
        self.modal.is_none()
            && self.project.undo.depth() == last.history_depth
            && self.project.project.revision_clock() == last.revision_clock
    }

    /// Reaplica a última operação com um novo valor, substituindo a entrada de
    /// Undo do gesto (a profundidade do histórico não muda). Em erro, o
    /// documento volta exatamente ao resultado anterior.
    pub fn adjust_last_operation(
        &mut self,
        last: &LastOperation,
        value: f32,
    ) -> Result<Option<LastOperation>, ModalError> {
        if !value.is_finite() {
            return Err(ModalError::InvalidInput);
        }
        if !self.last_operation_is_current(last) {
            return Err(ModalError::NoActiveOperation);
        }
        let (translation, scalar) = last.inputs_for(value)?;
        if !self.undo() {
            return Err(ModalError::NoActiveOperation);
        }
        let applied = self.reapply(last, translation, scalar);
        match applied {
            Ok(()) => Ok(self.commit_modal_gesture()),
            Err(error) => {
                self.cancel_modal();
                // Devolve o resultado confirmado anteriormente.
                self.redo();
                Err(error)
            }
        }
    }

    fn reapply(
        &mut self,
        last: &LastOperation,
        translation: Vec3,
        scalar: f32,
    ) -> Result<(), ModalError> {
        if let Some(prelude) = &last.prelude {
            let before = self.project.project.clone();
            let before_selection = self.session.selection.clone();
            let (after, selection) = prelude.0.as_ref().clone();
            self.project.project = after;
            self.session.selection = selection;
            self.begin_modal_after_prelude(last.kind, before, before_selection)?;
            if last.flip_when_negative {
                self.set_modal_flip_when_negative();
            }
        } else {
            self.begin_modal(last.kind)?;
        }
        if last.constraint != ModalConstraint::Free {
            self.set_modal_constraint(last.constraint)?;
        }
        if let Some(modal) = self.modal.as_mut() {
            // Mesma orientação do gesto original, mesmo se a câmera mudou.
            modal.normal = last.normal;
        }
        self.update_modal(translation, scalar)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EditMode;

    const A: [f32; 2] = [100.0, 100.0];

    #[test]
    fn click_below_threshold_selects_and_drag_above_begins() {
        let mut session = ToolSession::default();
        assert_eq!(session.press(A, PressTarget::Surface), ToolEffect::Nothing);
        assert_eq!(session.move_to([102.0, 101.0]), ToolEffect::Nothing);
        assert_eq!(
            session.release([102.0, 101.0]),
            ToolEffect::Click { at: [102.0, 101.0] }
        );
        assert_eq!(session.phase(), ToolPhase::Idle);

        session.press(A, PressTarget::Surface);
        assert_eq!(
            session.move_to([110.0, 100.0]),
            ToolEffect::BeginDrag {
                anchor: A,
                current: [110.0, 100.0],
                target: PressTarget::Surface
            }
        );
        assert_eq!(
            session.move_to([120.0, 100.0]),
            ToolEffect::UpdateDrag {
                anchor: A,
                current: [120.0, 100.0]
            }
        );
        assert_eq!(session.release([120.0, 100.0]), ToolEffect::Commit);
        assert_eq!(session.phase(), ToolPhase::Idle);
    }

    #[test]
    fn threshold_is_configurable_and_clamped() {
        let mut session = ToolSession::new(12.0, false);
        session.press(A, PressTarget::Surface);
        assert_eq!(session.move_to([108.0, 100.0]), ToolEffect::Nothing);
        assert!(matches!(
            session.move_to([113.0, 100.0]),
            ToolEffect::BeginDrag { .. }
        ));
        session.set_drag_threshold_px(500.0);
        assert_eq!(session.drag_threshold_px(), DRAG_THRESHOLD_RANGE.1);
        session.set_drag_threshold_px(f32::NAN);
        assert_eq!(session.drag_threshold_px(), DEFAULT_DRAG_THRESHOLD_PX);
    }

    #[test]
    fn click_move_click_on_a_handle_latches_and_next_click_commits() {
        let mut session = ToolSession::new(DEFAULT_DRAG_THRESHOLD_PX, true);
        session.press(A, PressTarget::Handle(2));
        assert_eq!(
            session.release(A),
            ToolEffect::BeginDrag {
                anchor: A,
                current: A,
                target: PressTarget::Handle(2)
            }
        );
        assert!(session.is_latched());
        assert!(matches!(
            session.move_to([160.0, 90.0]),
            ToolEffect::UpdateDrag { .. }
        ));
        assert_eq!(
            session.press([160.0, 90.0], PressTarget::Surface),
            ToolEffect::Commit
        );
        assert_eq!(session.phase(), ToolPhase::Idle);

        // Clique fora de alça continua selecionando.
        session.press(A, PressTarget::Surface);
        assert_eq!(session.release(A), ToolEffect::Click { at: A });
    }

    #[test]
    fn typed_value_wins_over_the_pointer_and_never_leaks() {
        let mut session = ToolSession::default();
        session.press(A, PressTarget::Surface);
        session.move_to([130.0, 100.0]);
        assert_eq!(session.text('2'), ToolEffect::TypedValue(2.0));
        assert_eq!(session.text(','), ToolEffect::TypedValue(2.0));
        assert_eq!(session.text('5'), ToolEffect::TypedValue(2.5));
        assert_eq!(session.numeric_text(), "2.5");
        assert_eq!(session.move_to([10.0, 10.0]), ToolEffect::Nothing);
        assert_eq!(session.key(ToolKey::Backspace), ToolEffect::TypedValue(2.0));
        assert_eq!(session.key(ToolKey::Backspace), ToolEffect::TypedValue(2.0));
        assert_eq!(session.key(ToolKey::Backspace), ToolEffect::TypedCleared);
        assert!(matches!(
            session.move_to([140.0, 100.0]),
            ToolEffect::UpdateDrag { .. }
        ));
        session.text('7');
        assert_eq!(session.key(ToolKey::Confirm), ToolEffect::Commit);
        assert_eq!(session.numeric_text(), "", "confirmar limpa o buffer");

        session.press(A, PressTarget::Surface);
        session.move_to([130.0, 100.0]);
        session.text('9');
        assert_eq!(session.key(ToolKey::Cancel), ToolEffect::Cancel);
        assert_eq!(session.numeric_text(), "", "cancelar limpa o buffer");
    }

    #[test]
    fn digits_outside_a_gesture_are_ignored() {
        let mut session = ToolSession::default();
        assert_eq!(session.text('3'), ToolEffect::Nothing);
        assert_eq!(session.numeric_text(), "");
    }

    #[test]
    fn secondary_button_never_cancels() {
        let mut session = ToolSession::default();
        assert_eq!(session.secondary(A), ToolEffect::OpenContextMenu { at: A });
        session.press(A, PressTarget::Surface);
        session.move_to([130.0, 100.0]);
        assert_eq!(session.secondary(A), ToolEffect::Nothing);
        assert!(session.is_gesture_active(), "o gesto continua");
    }

    #[test]
    fn escape_ladder_cancels_the_gesture_then_exits_the_tool() {
        let mut session = ToolSession::default();
        session.begin_latched(A, PressTarget::Surface);
        assert_eq!(session.key(ToolKey::Cancel), ToolEffect::Cancel);
        assert_eq!(session.key(ToolKey::Cancel), ToolEffect::ExitTool);
    }

    #[test]
    fn keyboard_latched_gesture_follows_the_pointer_and_click_confirms() {
        let mut session = ToolSession::default();
        assert!(matches!(
            session.begin_latched(A, PressTarget::Surface),
            ToolEffect::BeginDrag { .. }
        ));
        assert!(matches!(
            session.move_to([150.0, 120.0]),
            ToolEffect::UpdateDrag { .. }
        ));
        assert_eq!(
            session.press([150.0, 120.0], PressTarget::Surface),
            ToolEffect::Commit
        );
    }

    #[test]
    fn extrude_drag_follows_the_projected_normal_under_zoom() {
        // Uma unidade de normal ocupa 50 px para cima na tela.
        let frame = DragFrame {
            pivot_px: [400.0, 300.0],
            normal_px_per_unit: Some([0.0, -50.0]),
            world_per_pixel: 0.02,
        };
        let value = drag_value(
            ModalKind::Extrude,
            frame,
            [400.0, 300.0],
            [400.0, 200.0],
            0.0,
        );
        assert!(
            (value - 2.0).abs() < 1.0e-5,
            "100 px para cima = 2 unidades"
        );
        // Movimento perpendicular à normal não muda a profundidade.
        let sideways = drag_value(
            ModalKind::Extrude,
            frame,
            [400.0, 300.0],
            [500.0, 300.0],
            0.0,
        );
        assert!(sideways.abs() < 1.0e-5);
        // Com zoom 2×, a mesma distância em tela vale metade.
        let zoomed = DragFrame {
            normal_px_per_unit: Some([0.0, -100.0]),
            ..frame
        };
        let half = drag_value(
            ModalKind::Extrude,
            zoomed,
            [400.0, 300.0],
            [400.0, 200.0],
            0.0,
        );
        assert!((half - 1.0).abs() < 1.0e-5);
    }

    #[test]
    fn extrude_toward_the_camera_falls_back_to_vertical_drag() {
        let frame = DragFrame {
            pivot_px: [400.0, 300.0],
            normal_px_per_unit: Some([0.5, 0.5]),
            world_per_pixel: 0.01,
        };
        let value = drag_value(
            ModalKind::PushPull,
            frame,
            [400.0, 300.0],
            [400.0, 250.0],
            0.0,
        );
        assert!((value - 0.5).abs() < 1.0e-5);
    }

    #[test]
    fn inset_bevel_and_scale_have_bounded_screen_mappings() {
        let frame = DragFrame {
            pivot_px: [400.0, 300.0],
            normal_px_per_unit: None,
            world_per_pixel: 0.01,
        };
        let inset = drag_value(ModalKind::Inset, frame, [500.0, 300.0], [450.0, 300.0], 0.0);
        assert!(
            (inset - 0.5).abs() < 1.0e-5,
            "50 px × 0,01 m/px em direção ao pivô"
        );
        // Inset métrico: distância de mundo, sem teto de fração (o limite
        // anti-interseção é da geometria).
        let past_pivot = drag_value(ModalKind::Inset, frame, [500.0, 300.0], [300.0, 300.0], 0.0);
        assert!((past_pivot - 2.0).abs() < 1.0e-5);
        let outward = drag_value(ModalKind::Inset, frame, [500.0, 300.0], [600.0, 300.0], 0.0);
        assert_eq!(outward, 0.0);
        let bevel = drag_value(ModalKind::Bevel, frame, [500.0, 300.0], [550.0, 300.0], 0.0);
        assert!((bevel - 0.5).abs() < 1.0e-5);
        let bevel_in = drag_value(ModalKind::Bevel, frame, [500.0, 300.0], [300.0, 300.0], 0.0);
        assert_eq!(bevel_in, 0.0);
        let scale = drag_value(ModalKind::Scale, frame, [500.0, 300.0], [600.0, 300.0], 1.0);
        assert!((scale - 2.0).abs() < 1.0e-5);
    }

    fn selected_face_state() -> AppState {
        let mut state = AppState::new("en");
        state.set_edit_mode(EditMode::Edit);
        let mesh = state.project.active_mesh_mut().unwrap();
        mesh.deselect_all();
        mesh.faces[0].selected = true;
        mesh.sync_vert_selection_from_faces();
        state.sync_selection();
        state
    }

    #[test]
    fn a_confirmed_gesture_is_one_undo_entry_and_adjusting_it_keeps_the_depth() {
        let mut state = selected_face_state();
        state.begin_modal(ModalKind::Extrude).unwrap();
        state.update_modal(Vec3::ZERO, 0.5).unwrap();
        let last = state
            .commit_modal_gesture()
            .expect("gesto mudou o documento");
        assert_eq!(state.project.undo.depth(), (1, 0));
        let faces_after_first = state.project.active_mesh().unwrap().faces.len();

        let adjusted = state
            .adjust_last_operation(&last, 2.0)
            .expect("ajuste aceito")
            .expect("ainda muda o documento");
        assert_eq!(
            state.project.undo.depth(),
            (1, 0),
            "sem entrada extra de Undo"
        );
        assert!((adjusted.primary_value() - 2.0).abs() < 1.0e-5);
        assert_eq!(
            state.project.active_mesh().unwrap().faces.len(),
            faces_after_first,
            "reaplica a partir do estado original, não em cima do resultado"
        );

        // Um Undo volta ao estado anterior ao gesto.
        assert!(state.undo());
        assert_eq!(state.project.undo.depth(), (0, 1));
    }

    #[test]
    fn last_operation_expires_when_the_document_changes() {
        let mut state = selected_face_state();
        state.begin_modal(ModalKind::Extrude).unwrap();
        state.update_modal(Vec3::ZERO, 0.5).unwrap();
        let last = state.commit_modal_gesture().unwrap();
        assert!(state.last_operation_is_current(&last));

        state.begin_modal(ModalKind::Extrude).unwrap();
        state.update_modal(Vec3::ZERO, 0.25).unwrap();
        state.commit_modal_gesture().unwrap();
        assert!(!state.last_operation_is_current(&last));
        assert_eq!(
            state.adjust_last_operation(&last, 1.0),
            Err(ModalError::NoActiveOperation)
        );
    }

    #[test]
    fn invalid_adjustment_restores_the_previous_result() {
        let mut state = selected_face_state();
        state.begin_modal(ModalKind::Inset).unwrap();
        state.update_modal(Vec3::ZERO, 0.3).unwrap();
        let last = state.commit_modal_gesture().unwrap();
        let before = state.project.active_mesh().unwrap().clone();

        assert_eq!(
            state.adjust_last_operation(&last, -1.0),
            Err(ModalError::InvalidInput),
            "inset negativo é recusado"
        );
        assert_eq!(state.project.undo.depth(), (1, 0));
        let after = state.project.active_mesh().unwrap();
        assert_eq!(after.faces.len(), before.faces.len());
        assert_eq!(after.verts.len(), before.verts.len());
    }

    #[test]
    fn an_unchanged_gesture_creates_no_history() {
        let mut state = selected_face_state();
        state.begin_modal(ModalKind::Extrude).unwrap();
        assert!(state.commit_modal_gesture().is_none());
        assert_eq!(state.project.undo.depth(), (0, 0));
    }
}
