//! Ações contextuais da seleção (A08, Shell v2).
//!
//! A barra contextual da GUI projeta esta tabela: **até 4 ações em destaque** e
//! um "Mais" (newdocs UI-001 §4; SEL-007). A habilitação (e o motivo, quando
//! desabilitada) vem do catálogo de comandos publicado — a UI nunca filtra em
//! silêncio nem decide capacidades por conta própria.

use petunia_core::{SelectionDomain, Workspace};

use crate::ModelingMode;

/// Como a ação é executada pelo host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    /// `CommandDispatcher`: `request("command", {id: target})`.
    Command,
    /// Ativa uma ferramenta persistente: `request("tool", {id: target})`.
    Tool,
    /// Ação do adaptador do editor: `request(target)`.
    Request,
}

impl ActionKind {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::Tool => "tool",
            Self::Request => "request",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextAction {
    /// Id estável (chave de texto `action.<id>` e de teste).
    pub id: &'static str,
    /// Rótulo em inglês (fallback de tradução).
    pub label: &'static str,
    pub icon: &'static str,
    pub kind: ActionKind,
    /// Id do comando, da ferramenta ou da ação do adaptador.
    pub target: &'static str,
    pub keymap_action: Option<&'static str>,
    /// Em destaque na barra (no máximo [`MAX_PRIMARY`]); o resto vai em "Mais".
    pub primary: bool,
}

/// Limite de ações em destaque (newdocs: 3–4 + Mais).
pub const MAX_PRIMARY: usize = 4;

/// Grupos do menu "Mais", na ordem em que aparecem. O menu mostra um cabeçalho
/// por grupo em vez de uma lista única e longa.
pub const GROUP_ORDER: [&str; 8] = [
    "object", "scene", "form", "topology", "shape", "uv", "utility", "danger",
];

/// Grupo de uma ação (agrupamento do "Mais"; a barra em destaque ignora).
pub fn group_of(id: &str) -> &'static str {
    match id {
        "group" | "ungroup" | "isolate" => "scene",
        "extrude" | "inset" | "bevel" | "push_pull" | "make_face" | "flip_normals"
        | "extrude_volume" => "form",
        "loop_cut" | "knife" | "slice" | "subdivide" | "merge" | "connect" | "dissolve" => {
            "topology"
        }
        "close_profile" | "unite" | "subtract" | "intersect" | "exclude" | "shape_builder" => {
            "shape"
        }
        "unwrap" | "pack" | "relax" | "stitch" | "project_view" | "project_reference"
        | "project_planar" | "project_cube" | "equalize_density" | "split" => "uv",
        "measure" | "clear_measures" => "utility",
        "delete" => "danger",
        _ => "object",
    }
}

/// Alvo (`Request`) tratado só pela GUI: abre o painel de alinhar e distribuir.
pub const UI_ALIGN: &str = "ui.align";

const fn act(
    id: &'static str,
    label: &'static str,
    icon: &'static str,
    kind: ActionKind,
    target: &'static str,
    keymap_action: Option<&'static str>,
    primary: bool,
) -> ContextAction {
    ContextAction {
        id,
        label,
        icon,
        kind,
        target,
        keymap_action,
        primary,
    }
}

use ActionKind::{Command, Request, Tool};

const DELETE: ContextAction = act(
    "delete",
    "Delete",
    "trash",
    Command,
    "edit.delete",
    Some("model.delete"),
    false,
);
const DISSOLVE: ContextAction = act(
    "dissolve",
    "Dissolve",
    "dissolve",
    Command,
    "model.dissolve",
    Some("model.dissolve"),
    false,
);
const MAKE_FACE: ContextAction = act(
    "make_face",
    "Make face",
    "make_face",
    Command,
    "model.make_face",
    Some("model.make_face"),
    false,
);
const SUBDIVIDE: ContextAction = act(
    "subdivide",
    "Subdivide",
    "subdivide",
    Command,
    "model.subdivide",
    Some("model.subdivide"),
    false,
);
const KNIFE: ContextAction = act(
    "knife",
    "Knife",
    "knife",
    Tool,
    "cut",
    Some("model.knife"),
    false,
);

const ALIGN: ContextAction = act(
    "align",
    "Align",
    "bounding_box",
    Request,
    UI_ALIGN,
    None,
    true,
);
const GROUP: ContextAction = act(
    "group",
    "Group",
    "collection",
    Command,
    "scene.group",
    None,
    false,
);
const UNGROUP: ContextAction = act(
    "ungroup",
    "Ungroup",
    "hierarchy",
    Command,
    "scene.ungroup",
    None,
    false,
);
const ISOLATE: ContextAction = act(
    "isolate",
    "Isolate",
    "isolate",
    Request,
    "isolate",
    Some("view.isolate"),
    false,
);
const EXTRUDE_TOOL: ContextAction = act(
    "extrude",
    "Extrude",
    "extrude",
    Tool,
    "extrude",
    Some("model.extrude"),
    false,
);
const INSET_TOOL: ContextAction = act(
    "inset",
    "Inset",
    "inset",
    Tool,
    "inset",
    Some("model.inset"),
    false,
);
const BEVEL_TOOL: ContextAction = act(
    "bevel",
    "Bevel",
    "bevel",
    Tool,
    "bevel",
    Some("model.bevel"),
    false,
);
const PUSH_PULL_TOOL: ContextAction = act(
    "push_pull",
    "Push/Pull",
    "pushpull",
    Tool,
    "push_pull",
    Some("model.push_pull"),
    false,
);
const LOOP_CUT_TOOL: ContextAction = act(
    "loop_cut",
    "Loop cut",
    "loop_cut",
    Tool,
    "loop_cut",
    Some("model.loop_cut"),
    false,
);
const SLICE_TOOL: ContextAction = act(
    "slice",
    "Slice",
    "slice",
    Tool,
    "slice",
    Some("model.slice"),
    false,
);
const MEASURE_TOOL: ContextAction = act(
    "measure", "Measure", "measure", Tool, "measure", None, false,
);
/// Remove as medidas gravadas pela ferramenta Medir (desabilitada sem medidas).
const CLEAR_MEASURES: ContextAction = act(
    "clear_measures",
    "Clear measures",
    "close",
    Command,
    "measure.clear",
    None,
    false,
);
const SHAPE_BUILDER_TOOL: ContextAction = act(
    "shape_builder",
    "Shape builder",
    "shape_layer",
    Tool,
    "shape_builder",
    Some("model.shape_builder"),
    false,
);

const OBJECT: &[ContextAction] = &[
    act(
        "duplicate",
        "Duplicate",
        "duplicate",
        Command,
        "model.duplicate",
        Some("model.duplicate"),
        true,
    ),
    ALIGN,
    act(
        "join",
        "Join",
        "boolean_join",
        Command,
        "model.join",
        None,
        true,
    ),
    act(
        "separate",
        "Separate",
        "separate",
        Command,
        "model.separate_selection",
        None,
        true,
    ),
    GROUP,
    UNGROUP,
    ISOLATE,
    act(
        "origin_geometry",
        "Origin to geometry",
        "origin",
        Command,
        "model.origin_geometry",
        None,
        false,
    ),
    act(
        "shade_smooth",
        "Shade smooth",
        "shade_smooth",
        Command,
        "model.shade_smooth",
        None,
        false,
    ),
    act(
        "shade_flat",
        "Shade flat",
        "shade_flat",
        Command,
        "model.shade_flat",
        None,
        false,
    ),
    EXTRUDE_TOOL,
    INSET_TOOL,
    BEVEL_TOOL,
    PUSH_PULL_TOOL,
    LOOP_CUT_TOOL,
    KNIFE,
    SLICE_TOOL,
    MEASURE_TOOL,
    CLEAR_MEASURES,
    DELETE,
];
const VERTEX: &[ContextAction] = &[
    act(
        "extrude",
        "Extrude",
        "extrude",
        Tool,
        "extrude",
        Some("model.extrude"),
        true,
    ),
    act(
        "bevel",
        "Bevel",
        "bevel",
        Tool,
        "bevel",
        Some("model.bevel"),
        true,
    ),
    act(
        "merge",
        "Merge",
        "merge",
        Command,
        "model.merge",
        Some("model.merge"),
        true,
    ),
    act(
        "connect",
        "Connect",
        "connect",
        Command,
        "model.connect",
        Some("model.connect"),
        false,
    ),
    MAKE_FACE,
    DISSOLVE,
    LOOP_CUT_TOOL,
    KNIFE,
    SLICE_TOOL,
    PUSH_PULL_TOOL,
    MEASURE_TOOL,
    CLEAR_MEASURES,
    DELETE,
];
const EDGE: &[ContextAction] = &[
    act(
        "extrude",
        "Extrude",
        "extrude",
        Tool,
        "extrude",
        Some("model.extrude"),
        true,
    ),
    act(
        "bevel",
        "Bevel",
        "bevel",
        Tool,
        "bevel",
        Some("model.bevel"),
        true,
    ),
    act(
        "loop_cut",
        "Loop cut",
        "loop_cut",
        Tool,
        "loop_cut",
        Some("model.loop_cut"),
        true,
    ),
    SUBDIVIDE,
    KNIFE,
    MAKE_FACE,
    DISSOLVE,
    PUSH_PULL_TOOL,
    SLICE_TOOL,
    MEASURE_TOOL,
    CLEAR_MEASURES,
    DELETE,
];
const FACE: &[ContextAction] = &[
    act(
        "extrude",
        "Extrude",
        "extrude",
        Tool,
        "extrude",
        Some("model.extrude"),
        true,
    ),
    act(
        "push_pull",
        "Push/Pull",
        "pushpull",
        Tool,
        "push_pull",
        Some("model.push_pull"),
        true,
    ),
    act(
        "inset",
        "Inset",
        "inset",
        Tool,
        "inset",
        Some("model.inset"),
        true,
    ),
    act(
        "bevel",
        "Bevel",
        "bevel",
        Tool,
        "bevel",
        Some("model.bevel"),
        false,
    ),
    KNIFE,
    SUBDIVIDE,
    act(
        "flip_normals",
        "Flip normals",
        "flip_normals",
        Command,
        "model.flip_normals",
        None,
        false,
    ),
    DISSOLVE,
    LOOP_CUT_TOOL,
    SLICE_TOOL,
    MEASURE_TOOL,
    CLEAR_MEASURES,
    DELETE,
];
const DRAW: &[ContextAction] = &[
    act(
        "close_profile",
        "Close profile",
        "profile_closed",
        Request,
        "close_profile",
        None,
        true,
    ),
    act(
        "extrude_volume",
        "Extrude volume",
        "extrude",
        Command,
        "model.extrude",
        Some("model.extrude"),
        true,
    ),
    act(
        "unite",
        "Unite",
        "boolean_fuse",
        Command,
        "draw.unite",
        None,
        false,
    ),
    act(
        "subtract",
        "Subtract",
        "boolean_cut",
        Command,
        "draw.subtract",
        None,
        false,
    ),
    act(
        "intersect",
        "Intersect",
        "boolean_intersect",
        Command,
        "draw.intersect",
        None,
        false,
    ),
    act(
        "exclude",
        "Exclude",
        "shape_square_dashed",
        Command,
        "draw.exclude",
        None,
        false,
    ),
    SHAPE_BUILDER_TOOL,
    PUSH_PULL_TOOL,
    MEASURE_TOOL,
    CLEAR_MEASURES,
];
const UV: &[ContextAction] = &[
    act(
        "unwrap",
        "Unwrap",
        "unwrap_auto",
        Command,
        "uv.unwrap_auto",
        None,
        true,
    ),
    act(
        "pack",
        "Pack islands",
        "uv_pack",
        Command,
        "uv.pack_islands",
        None,
        true,
    ),
    act(
        "relax", "Relax", "uv_relax", Command, "uv.relax", None, true,
    ),
    act(
        "stitch",
        "Stitch",
        "uv_stitch",
        Command,
        "uv.stitch",
        None,
        true,
    ),
    act(
        "project_view",
        "Project from view",
        "project_from_view",
        Command,
        "uv.project_view",
        None,
        false,
    ),
    act(
        "project_reference",
        "Project reference",
        "uv_project_reference",
        Command,
        "uv.project_reference",
        None,
        false,
    ),
    act(
        "project_planar",
        "Planar projection",
        "uv_project_view",
        Command,
        "uv.project_planar",
        None,
        false,
    ),
    act(
        "project_cube",
        "Cube projection",
        "uv_project_view",
        Command,
        "uv.project_cube",
        None,
        false,
    ),
    act(
        "equalize_density",
        "Equalize texel density",
        "uv_scale",
        Command,
        "uv.equalize_texel_density",
        None,
        false,
    ),
    act(
        "split",
        "Split island",
        "slice",
        Command,
        "uv.split",
        None,
        false,
    ),
];
const NONE: &[ContextAction] = &[];

/// Ações do contexto atual, na ordem da barra (destaque primeiro).
pub fn context_actions(
    workspace: Workspace,
    mode: ModelingMode,
    domain: SelectionDomain,
) -> &'static [ContextAction] {
    match workspace {
        Workspace::Model => match (mode, domain) {
            (ModelingMode::Draw, _) => DRAW,
            (_, SelectionDomain::Object) => OBJECT,
            (_, SelectionDomain::Vertex) => VERTEX,
            (_, SelectionDomain::Edge) => EDGE,
            (_, SelectionDomain::Face) => FACE,
        },
        Workspace::Uv => UV,
        _ => NONE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn all() -> Vec<(
        Workspace,
        ModelingMode,
        SelectionDomain,
        &'static [ContextAction],
    )> {
        let mut out = Vec::new();
        for mode in [ModelingMode::Draw, ModelingMode::Poly] {
            for domain in [
                SelectionDomain::Object,
                SelectionDomain::Vertex,
                SelectionDomain::Edge,
                SelectionDomain::Face,
            ] {
                out.push((
                    Workspace::Model,
                    mode,
                    domain,
                    context_actions(Workspace::Model, mode, domain),
                ));
            }
        }
        out.push((
            Workspace::Uv,
            ModelingMode::Poly,
            SelectionDomain::Face,
            context_actions(Workspace::Uv, ModelingMode::Poly, SelectionDomain::Face),
        ));
        out
    }

    #[test]
    fn at_most_four_primary_actions_per_context() {
        for (workspace, mode, domain, actions) in all() {
            let primary = actions.iter().filter(|action| action.primary).count();
            assert!(
                primary <= MAX_PRIMARY,
                "{workspace:?}/{mode:?}/{domain:?}: {primary} ações em destaque"
            );
        }
    }

    #[test]
    fn primary_actions_come_first_and_ids_are_unique() {
        for (workspace, mode, domain, actions) in all() {
            let mut seen = HashSet::new();
            let mut left_primary = false;
            for action in actions {
                assert!(
                    seen.insert(action.id),
                    "{workspace:?}/{mode:?}/{domain:?}: id repetido `{}`",
                    action.id
                );
                if !action.primary {
                    left_primary = true;
                } else {
                    assert!(
                        !left_primary,
                        "{workspace:?}/{mode:?}/{domain:?}: `{}` em destaque depois de item do Mais",
                        action.id
                    );
                }
                assert!(!action.target.is_empty() && !action.icon.is_empty());
            }
        }
    }

    #[test]
    fn command_targets_are_registered_commands() {
        let dispatcher = petunia_core::command::CommandDispatcher::canonical();
        for (_, _, _, actions) in all() {
            for action in actions {
                if action.kind == ActionKind::Command && !action.target.starts_with("draw.") {
                    assert!(
                        dispatcher.contains(action.target),
                        "`{}` não está registrado no CommandDispatcher",
                        action.target
                    );
                }
            }
        }
    }

    #[test]
    fn domains_expose_domain_specific_operations() {
        let ids = |domain| -> Vec<&'static str> {
            context_actions(Workspace::Model, ModelingMode::Poly, domain)
                .iter()
                .map(|action| action.id)
                .collect()
        };
        assert!(ids(SelectionDomain::Edge).contains(&"loop_cut"));
        assert!(ids(SelectionDomain::Face).contains(&"inset"));
        assert!(ids(SelectionDomain::Vertex).contains(&"merge"));
        assert!(ids(SelectionDomain::Object).contains(&"duplicate"));
    }

    #[test]
    fn every_action_belongs_to_a_known_menu_group() {
        for (workspace, mode, domain, actions) in all() {
            for action in actions {
                assert!(
                    GROUP_ORDER.contains(&group_of(action.id)),
                    "{workspace:?}/{mode:?}/{domain:?}: `{}` sem grupo conhecido",
                    action.id
                );
            }
        }
    }

    #[test]
    fn operations_removed_from_the_rail_are_reachable_here() {
        use crate::tool_catalog;
        for mode in [ModelingMode::Draw, ModelingMode::Poly] {
            let reachable: Vec<&str> = [
                SelectionDomain::Object,
                SelectionDomain::Vertex,
                SelectionDomain::Edge,
                SelectionDomain::Face,
            ]
            .iter()
            .flat_map(|domain| context_actions(Workspace::Model, mode, *domain))
            .filter(|action| action.kind == ActionKind::Tool)
            .map(|action| action.target)
            .collect();
            for tool in tool_catalog::operation_tools(Workspace::Model, mode) {
                assert!(
                    reachable.contains(&tool.id),
                    "{mode:?}: a ferramenta `{}` não tem ação contextual",
                    tool.id
                );
            }
        }
    }

    #[test]
    fn object_actions_offer_align_group_and_isolate() {
        let ids: Vec<&str> = context_actions(
            Workspace::Model,
            ModelingMode::Poly,
            SelectionDomain::Object,
        )
        .iter()
        .map(|action| action.id)
        .collect();
        for id in ["align", "group", "ungroup", "isolate"] {
            assert!(ids.contains(&id), "objeto sem `{id}`");
        }
    }
}
