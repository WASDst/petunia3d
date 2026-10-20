//! Seleção de ferramentas, gramática de ferramentas e adequação ao workspace.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// O modo Instant / Modo Livre com mouse está ativo?
    pub fn is_instant_tool_mode(&self) -> bool {
        self.keyboard_tool_modal_active
            || self.instant_transform
            || self.state.session.tools.tool_activation == petunia_core::ToolActivation::Instant
    }

    /// Ferramenta ativa que segue a gramática única, se houver.
    pub fn grammar_tool(&self) -> Option<GrammarTool> {
        if self.decal_tool_active() {
            return Some(GrammarTool::Decal);
        }
        if self.state.workspace == Workspace::Paint
            && matches!(
                self.state.session.tools.active_tool.as_str(),
                "path_paint" | "projection"
            )
        {
            return Some(GrammarTool::SurfacePaint);
        }
        if self.state.workspace != Workspace::Model {
            return None;
        }
        let parametric = |fallback: ToolModalKind| {
            let kind = self
                .tool_modal
                .or(self.parametric_tool)
                .filter(|kind| parametric_tool_id(*kind) == parametric_tool_id(fallback))
                .unwrap_or(fallback);
            Some(GrammarTool::Parametric(kind))
        };
        match self.state.session.tools.active_tool.as_str() {
            "select" | "box_select" => Some(GrammarTool::Select),
            "move" | "transform" => Some(GrammarTool::Transform(TransformKind::Position)),
            "rotate" => Some(GrammarTool::Transform(TransformKind::Rotation)),
            "scale" => Some(GrammarTool::Transform(TransformKind::Scale)),
            "extrude" => parametric(ToolModalKind::Extrude),
            "inset" => parametric(ToolModalKind::Inset),
            "bevel" => parametric(ToolModalKind::Bevel),
            "push_pull" => parametric(ToolModalKind::PushPull),
            "poly_pen" => Some(GrammarTool::PolyPen),
            "shape_builder" => Some(GrammarTool::ShapeBuilder),
            "draw_profile" => Some(GrammarTool::DrawProfile),
            "draw_rectangle" => Some(GrammarTool::PlanarPrimitive(PlanarPrimitiveKind::Rectangle)),
            "draw_circle" => Some(GrammarTool::PlanarPrimitive(PlanarPrimitiveKind::Circle)),
            "draw_ellipse" => Some(GrammarTool::PlanarPrimitive(PlanarPrimitiveKind::Ellipse)),
            "draw_polygon" => Some(GrammarTool::PlanarPrimitive(PlanarPrimitiveKind::Polygon)),
            "cut" => Some(GrammarTool::Knife),
            "loop_cut" => Some(GrammarTool::LoopCut),
            "slice" => Some(GrammarTool::Slice),
            _ => None,
        }
    }

    /// O botão principal na viewport é roteado pela gramática única.
    ///
    /// Operações abertas por teclado (modal estilo Blender) seguem o ponteiro
    /// numa sessão travada (`ToolSession::begin_latched`): o próximo clique
    /// confirma pelo mesmo caminho.
    pub fn tool_grammar_active(&self) -> bool {
        self.grammar_tool().is_some() && !self.keyboard_tool_modal_active && !self.instant_transform
    }

    /// Ativa uma ferramenta paramétrica persistente sem abrir operação.
    ///
    /// Em Object, troca de forma visível para o domínio exigido; nunca
    /// seleciona tudo por conta própria (constituição 11).
    pub fn activate_parametric_tool(&mut self, kind: ToolModalKind) {
        if self.tool_modal.is_some() {
            self.cancel_tool_modal();
        }
        self.tool_session.reset();
        self.tool_gesture = None;
        self.parametric_tool = Some(kind);
        self.state.session.tools.active_tool = parametric_tool_id(kind).to_string();
        let domain = if kind == ToolModalKind::Bevel {
            SelectionDomain::Edge
        } else {
            SelectionDomain::Face
        };
        if self.state.selection_domain() == SelectionDomain::Object {
            self.state.set_selection_domain(domain);
            self.sync_viewport_context();
        }
        let message = self
            .state
            .t_id(petunia_config::text_id::TOOL_GRAMMAR_READY)
            .replace("{tool}", &self.state.t(kind.title_key()));
        self.state.set_status(message);
        self.state.mark_dirty();
    }

    /// `Esc` sem gesto: a ferramenta volta para Select (escada do Esc).
    pub(crate) fn exit_grammar_tool(&mut self) -> bool {
        if self.grammar_tool().is_none() {
            return false;
        }
        self.tool_session.reset();
        self.tool_gesture = None;
        self.state.session.tools.active_tool = "select".to_string();
        self.state.mark_dirty();
        true
    }

    /// `Space`: volta para a ferramenta anterior (e vice-versa). Não faz nada
    /// durante uma sessão, um arrasto ou uma renomeação.
    pub(crate) fn swap_to_previous_tool(&mut self) -> bool {
        if self.state.session.tools.modal.is_some()
            || self.tool_modal.is_some()
            || self.loop_cut.is_some()
            || self.drag.is_some()
            || self.rename_draft.is_some()
            || self.tool_session.is_gesture_active()
        {
            return false;
        }
        let current = self.state.session.tools.active_tool.clone();
        let mut target = self.previous_tool.clone();
        if !self.tool_belongs_to_workspace(&target) {
            target = "select".to_string();
        }
        if target == current {
            return false;
        }
        self.apply(UiIntent::SetActiveTool(target));
        true
    }

    /// Ferramentas que só existem no workspace PAINT (pincéis, balde e formas).
    pub(crate) fn is_paint_only_tool(tool: &str) -> bool {
        matches!(
            tool,
            "path_paint"
                | "projection"
                | "brush"
                | "airbrush"
                | "eraser"
                | "picker"
                | "fill"
                | "line"
                | "rectangle"
                | "ellipse"
                | "gradient"
                | "gradient_radial"
                | "pixel"
                | "smudge"
                | "blur"
                | "dodge"
                | "burn"
                | "spray"
                | "clone"
        )
    }

    /// Ao trocar de workspace a ferramenta ativa precisa existir no novo: no
    /// PAINT vira o pincel (o clique pinta); fora dele, uma ferramenta de
    /// pintura volta para Select.
    pub(crate) fn normalize_tool_for_workspace(&mut self) {
        let tool = self.state.session.tools.active_tool.clone();
        if self.state.workspace == Workspace::Paint {
            if !self.tool_belongs_to_workspace(&tool) || tool == "select" {
                self.apply(UiIntent::SetActiveTool("brush".to_string()));
            }
        } else if Self::is_paint_only_tool(&tool) || !self.tool_belongs_to_workspace(&tool) {
            self.apply(UiIntent::SetActiveTool("select".to_string()));
        }
        self.previous_tool = "select".to_string();
    }

    /// A ferramenta existe no workspace atual (evita levar um pincel para o DRAW).
    pub(crate) fn tool_belongs_to_workspace(&self, tool: &str) -> bool {
        match self.state.workspace {
            Workspace::Model => {
                !Self::is_paint_only_tool(tool) && self.modeling_mode.offers_tool(tool)
            }
            Workspace::Paint => tool == "select" || Self::is_paint_only_tool(tool),
            Workspace::Uv => tool == "select" || tool == "uv_select",
            #[cfg(feature = "animation-workspace")]
            Workspace::Animate => tool == "select",
        }
    }
}
