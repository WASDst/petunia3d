//! Catálogo único de ferramentas por workspace (A08, Shell v2).
//!
//! O rail da GUI nunca mantém listas próprias: ele projeta este catálogo, que
//! vive no editor neutro (testável sem display). Um item é uma ferramenta
//! **persistente de ponteiro** (`active_tool`) ou o menu de criação; comandos
//! pontuais pertencem às ações contextuais, e parâmetros re-editáveis, às
//! operações (ver `website/docs/09-ui/shell-v2-proposal.md` §3).

use petunia_core::Workspace;

use crate::ModelingMode;

/// Agrupamento visual do rail; grupos consecutivos recebem um separador.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolGroup {
    Select,
    Transform,
    Create,
    Edit,
    Shape,
    Measure,
    Brush,
    Retouch,
    Shapes2d,
    Surface,
}

impl ToolGroup {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Select => "select",
            Self::Transform => "transform",
            Self::Create => "create",
            Self::Edit => "edit",
            Self::Shape => "shape",
            Self::Measure => "measure",
            Self::Brush => "brush",
            Self::Retouch => "retouch",
            Self::Shapes2d => "shapes2d",
            Self::Surface => "surface",
        }
    }
}

/// Como o item se comporta ao ser acionado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    /// Ativa `active_tool` (modo persistente de ponteiro).
    Mode,
    /// Abre o menu de criação de primitivas (não muda a ferramenta ativa).
    CreateMenu,
}

impl ToolKind {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Mode => "mode",
            Self::CreateMenu => "create_menu",
        }
    }
}

/// Descrição estática de uma ferramenta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolDescriptor {
    /// Id enviado em `SetActiveTool` (e chave de texto `tool.<id>`).
    pub id: &'static str,
    /// Rótulo em inglês (fallback quando não há tradução para `tool.<id>`).
    pub label: &'static str,
    /// Id do ícone no conjunto `petunia-outline` (flat, sem categoria).
    pub icon: &'static str,
    pub group: ToolGroup,
    pub kind: ToolKind,
    /// Ação do keymap cujo atalho aparece no tooltip (nunca fixo no widget).
    pub keymap_action: Option<&'static str>,
}

const fn tool(
    id: &'static str,
    label: &'static str,
    icon: &'static str,
    group: ToolGroup,
    keymap_action: Option<&'static str>,
) -> ToolDescriptor {
    ToolDescriptor {
        id,
        label,
        icon,
        group,
        kind: ToolKind::Mode,
        keymap_action,
    }
}

use ToolGroup as G;

const SELECT: ToolDescriptor = tool(
    "select",
    "Select",
    "select_box",
    G::Select,
    Some("model.tool_select"),
);
const LASSO: ToolDescriptor = tool(
    "lasso_select",
    "Lasso select",
    "select_lasso",
    G::Select,
    None,
);
const MOVE: ToolDescriptor = tool("move", "Move", "move", G::Transform, Some("model.move"));
const ROTATE: ToolDescriptor = tool(
    "rotate",
    "Rotate",
    "rotate",
    G::Transform,
    Some("model.rotate"),
);
const SCALE: ToolDescriptor = tool("scale", "Scale", "scale", G::Transform, Some("model.scale"));
const TRANSFORM: ToolDescriptor = tool(
    "transform",
    "Transform",
    "transform",
    G::Transform,
    Some("model.transform"),
);
const CREATE: ToolDescriptor = ToolDescriptor {
    id: "primitive",
    label: "Create primitive",
    icon: "add_primitive",
    group: G::Create,
    kind: ToolKind::CreateMenu,
    keymap_action: Some("model.primitives"),
};
const POLY_PEN: ToolDescriptor = tool("poly_pen", "Poly pen", "tool_poly_pen", G::Create, None);
const EXTRUDE: ToolDescriptor = tool(
    "extrude",
    "Extrude",
    "extrude",
    G::Edit,
    Some("model.extrude"),
);
const INSET: ToolDescriptor = tool("inset", "Inset", "inset", G::Edit, Some("model.inset"));
const BEVEL: ToolDescriptor = tool("bevel", "Bevel", "bevel", G::Edit, Some("model.bevel"));
const PUSH_PULL: ToolDescriptor = tool(
    "push_pull",
    "Push/Pull",
    "pushpull",
    G::Edit,
    Some("model.push_pull"),
);
const LOOP_CUT: ToolDescriptor = tool(
    "loop_cut",
    "Loop cut",
    "loop_cut",
    G::Edit,
    Some("model.loop_cut"),
);
const KNIFE: ToolDescriptor = tool("cut", "Knife", "knife", G::Edit, Some("model.knife"));
const SLICE: ToolDescriptor = tool("slice", "Slice", "slice", G::Edit, Some("model.slice"));
const DRAW_PROFILE: ToolDescriptor = tool(
    "draw_profile",
    "Draw profile",
    "draw_profile",
    G::Shape,
    Some("model.draw_profile"),
);
const SHAPE_BUILDER: ToolDescriptor = tool(
    "shape_builder",
    "Shape builder",
    "shape_layer",
    G::Shape,
    Some("model.shape_builder"),
);
const CURSOR: ToolDescriptor = tool("cursor", "3D cursor", "cursor_3d", G::Measure, None);
const MEASURE: ToolDescriptor = tool("measure", "Measure", "measure", G::Measure, None);

const PAINT_SELECT: ToolDescriptor = tool(
    "select",
    "Select",
    "select_box",
    G::Select,
    Some("paint.select"),
);
const BRUSH: ToolDescriptor = tool(
    "brush",
    "Brush",
    "paint_brush",
    G::Brush,
    Some("paint.paint"),
);
const PIXEL: ToolDescriptor = tool("pixel", "Pixel", "paint_pixel", G::Brush, None);
const AIRBRUSH: ToolDescriptor = tool(
    "airbrush",
    "Airbrush",
    "paint_airbrush",
    G::Brush,
    Some("paint.airbrush"),
);
const SPRAY: ToolDescriptor = tool("spray", "Spray", "paint_spray", G::Brush, None);
const ERASER: ToolDescriptor = tool(
    "eraser",
    "Eraser",
    "paint_eraser",
    G::Brush,
    Some("paint.eraser"),
);
const SMUDGE: ToolDescriptor = tool("smudge", "Smudge", "paint_smudge", G::Retouch, None);
const BLUR: ToolDescriptor = tool("blur", "Blur", "paint_blur", G::Retouch, None);
const DODGE: ToolDescriptor = tool("dodge", "Dodge", "paint_dodge", G::Retouch, None);
const BURN: ToolDescriptor = tool("burn", "Burn", "paint_burn", G::Retouch, None);
const CLONE: ToolDescriptor = tool("clone", "Clone", "paint_clone", G::Retouch, None);
const PICKER: ToolDescriptor = tool(
    "picker",
    "Color picker",
    "paint_picker",
    G::Retouch,
    Some("paint.color_picker"),
);
const FILL: ToolDescriptor = tool(
    "fill",
    "Fill",
    "paint_fill",
    G::Shapes2d,
    Some("paint.fill"),
);
const GRADIENT: ToolDescriptor = tool(
    "gradient",
    "Gradient",
    "paint_gradient",
    G::Shapes2d,
    Some("paint.gradient"),
);
const GRADIENT_RADIAL: ToolDescriptor = tool(
    "gradient_radial",
    "Radial gradient",
    "paint_gradient_radial",
    G::Shapes2d,
    None,
);
const LINE: ToolDescriptor = tool(
    "line",
    "Line",
    "paint_line",
    G::Shapes2d,
    Some("paint.line"),
);
const RECTANGLE: ToolDescriptor = tool(
    "rectangle",
    "Rectangle",
    "paint_rect",
    G::Shapes2d,
    Some("paint.rectangle"),
);
const ELLIPSE: ToolDescriptor = tool("ellipse", "Ellipse", "shape_circle", G::Shapes2d, None);
const PATH_PAINT: ToolDescriptor = tool("path_paint", "Path paint", "paint_path", G::Surface, None);
const PROJECTION: ToolDescriptor = tool(
    "projection",
    "Projection",
    "paint_projection",
    G::Surface,
    None,
);

/// O rail da esquerda só tem transformar, 3D Cursor e criação (primitivas /
/// perfis). Operações (extrude, inset, bevel, push/pull, loop cut, knife,
/// slice, shape builder, medir) são contextuais: vivem na barra flutuante
/// (`context_actions`), organizadas por grupo.
const DRAW_TOOLS: &[ToolDescriptor] =
    &[SELECT, MOVE, ROTATE, SCALE, TRANSFORM, DRAW_PROFILE, CURSOR];
const POLY_TOOLS: &[ToolDescriptor] = &[
    SELECT, LASSO, MOVE, ROTATE, SCALE, TRANSFORM, CREATE, POLY_PEN, CURSOR,
];
const PAINT_TOOLS: &[ToolDescriptor] = &[
    PAINT_SELECT,
    BRUSH,
    PIXEL,
    AIRBRUSH,
    SPRAY,
    ERASER,
    SMUDGE,
    BLUR,
    DODGE,
    BURN,
    CLONE,
    PICKER,
    FILL,
    GRADIENT,
    GRADIENT_RADIAL,
    LINE,
    RECTANGLE,
    ELLIPSE,
    PATH_PAINT,
    PROJECTION,
];
const SELECT_ONLY: &[ToolDescriptor] = &[SELECT];

/// Ferramentas de operação do POLY: ativadas pela barra contextual.
const POLY_OPERATIONS: &[ToolDescriptor] = &[
    EXTRUDE, INSET, BEVEL, PUSH_PULL, LOOP_CUT, KNIFE, SLICE, MEASURE,
];
/// Ferramentas de operação do DRAW: ativadas pela barra contextual.
const DRAW_OPERATIONS: &[ToolDescriptor] = &[SHAPE_BUILDER, PUSH_PULL, MEASURE];

/// Ferramentas do contexto atual, na ordem do rail.
pub fn tools_for(workspace: Workspace, mode: ModelingMode) -> &'static [ToolDescriptor] {
    match workspace {
        Workspace::Model => match mode {
            ModelingMode::Draw => DRAW_TOOLS,
            ModelingMode::Poly => POLY_TOOLS,
        },
        Workspace::Paint => PAINT_TOOLS,
        _ => SELECT_ONLY,
    }
}

/// Ferramentas de operação do contexto (extrude, inset, bevel, push/pull, loop
/// cut, knife, slice, shape builder, medir). Não aparecem no rail: a barra
/// contextual as oferece, organizadas por grupo, sobre a seleção atual.
pub fn operation_tools(workspace: Workspace, mode: ModelingMode) -> &'static [ToolDescriptor] {
    match (workspace, mode) {
        (Workspace::Model, ModelingMode::Draw) => DRAW_OPERATIONS,
        (Workspace::Model, ModelingMode::Poly) => POLY_OPERATIONS,
        _ => &[],
    }
}

/// Procura uma ferramenta pelo id no contexto (rail ou operação contextual).
pub fn find(workspace: Workspace, mode: ModelingMode, id: &str) -> Option<&'static ToolDescriptor> {
    tools_for(workspace, mode)
        .iter()
        .chain(operation_tools(workspace, mode))
        .find(|tool| tool.id == id)
}

/// Todos os contextos cobertos (para testes e verificações estáticas).
pub fn all_contexts() -> Vec<(Workspace, ModelingMode)> {
    #[allow(unused_mut)]
    let mut contexts = vec![
        (Workspace::Model, ModelingMode::Draw),
        (Workspace::Model, ModelingMode::Poly),
        (Workspace::Paint, ModelingMode::Poly),
        (Workspace::Uv, ModelingMode::Poly),
    ];
    #[cfg(feature = "animation-workspace")]
    contexts.push((Workspace::Animate, ModelingMode::Poly));
    contexts
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_are_unique_per_context_and_have_icons_and_labels() {
        for (workspace, mode) in all_contexts() {
            let mut seen = HashSet::new();
            for tool in tools_for(workspace, mode) {
                assert!(
                    seen.insert(tool.id),
                    "{:?}/{:?}: id repetido `{}`",
                    workspace,
                    mode,
                    tool.id
                );
                assert!(!tool.label.is_empty(), "`{}` sem rótulo", tool.id);
                assert!(!tool.icon.is_empty(), "`{}` sem ícone", tool.id);
            }
        }
    }

    #[test]
    fn groups_are_contiguous_so_the_rail_can_separate_them() {
        for (workspace, mode) in all_contexts() {
            let mut closed = HashSet::new();
            let mut current: Option<ToolGroup> = None;
            for tool in tools_for(workspace, mode) {
                if current != Some(tool.group) {
                    if let Some(previous) = current {
                        closed.insert(previous);
                    }
                    assert!(
                        !closed.contains(&tool.group),
                        "{:?}/{:?}: grupo `{}` reaparece depois de fechado",
                        workspace,
                        mode,
                        tool.group.id()
                    );
                    current = Some(tool.group);
                }
            }
        }
    }

    #[test]
    fn every_workspace_exposes_selection_first() {
        for (workspace, mode) in all_contexts() {
            assert_eq!(tools_for(workspace, mode)[0].id, "select");
        }
    }

    #[test]
    fn the_rail_holds_only_transform_cursor_and_creation_tools() {
        const OPERATIONS: [&str; 9] = [
            "extrude",
            "inset",
            "bevel",
            "push_pull",
            "loop_cut",
            "cut",
            "slice",
            "shape_builder",
            "measure",
        ];
        for mode in [ModelingMode::Draw, ModelingMode::Poly] {
            let ids: Vec<_> = tools_for(Workspace::Model, mode)
                .iter()
                .map(|tool| tool.id)
                .collect();
            for id in OPERATIONS {
                assert!(
                    !ids.contains(&id),
                    "{mode:?}: a operação `{id}` pertence à barra contextual, não ao rail"
                );
            }
            assert!(ids.contains(&"cursor"), "{mode:?} mantém o 3D Cursor");
            assert!(ids.contains(&"transform"), "{mode:?} mantém Transformar");
        }
        let poly: Vec<_> = tools_for(Workspace::Model, ModelingMode::Poly)
            .iter()
            .map(|tool| tool.id)
            .collect();
        assert!(poly.contains(&"primitive"), "POLY cria primitivas");
    }

    #[test]
    fn create_menu_is_the_only_non_mode_item() {
        for (workspace, mode) in all_contexts() {
            for tool in tools_for(workspace, mode) {
                if tool.kind == ToolKind::CreateMenu {
                    assert_eq!(tool.id, "primitive");
                }
            }
        }
    }
}
