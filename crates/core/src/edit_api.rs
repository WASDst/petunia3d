//! Pontos de entrada canônicos para edições que a UI hoje fazia direto na malha
//! (website/docs/09-ui/slint-rescue-plan.md §11 "Proibir Slint callback →
//! mesh.faces[...]"; website/docs/03-geometry/draw-unification-proposal.md
//! "Gesto → ToolSession, não mutar direto da UI").
//!
//! - Seleção de componentes não entra no Undo: métodos da Application
//!   ([`CommandContext::apply_component_selection`],
//!   [`CommandContext::select_hover_if_empty`]).
//! - Mutações do documento são Commands (um Undo cada, validação, restauração
//!   em erro pelo dispatcher): [`ApplyMeshEditCmd`], [`UvSeamCmd`],
//!   [`UvPinCmd`], [`AssignMaterialSlotCmd`], [`NudgeCmd`].

use std::collections::HashSet;

use petunia_mesh::{FaceCornerIndex, Mesh, edge_key};
use petunia_project::ProjectChanges;
use uuid::Uuid;

use crate::command::{Command, CommandError};
use crate::context::{CommandContext, EditorView};
use crate::state::HoverTarget;

/// Conjunto de componentes de uma seleção.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComponentSet {
    Vertices(Vec<u32>),
    Edges(Vec<(u32, u32)>),
    Faces(Vec<usize>),
}

impl ComponentSet {
    /// Componente sob o cursor, quando houver.
    pub fn from_hover(hover: HoverTarget) -> Option<Self> {
        match hover {
            HoverTarget::Vertex(v) => Some(Self::Vertices(vec![v])),
            HoverTarget::Edge(a, b) => Some(Self::Edges(vec![(a, b)])),
            HoverTarget::Face(f) => Some(Self::Faces(vec![f])),
            HoverTarget::None | HoverTarget::Object(_) => None,
        }
    }

    /// Índices que não existem na malha viram `StaleReference`.
    fn validate(&self, mesh: &Mesh) -> Result<(), CommandError> {
        let stale = |what: String| Err(CommandError::StaleReference(what));
        match self {
            Self::Vertices(v) => match v.iter().find(|&&i| i as usize >= mesh.verts.len()) {
                Some(i) => stale(format!("vertex {i}")),
                None => Ok(()),
            },
            Self::Edges(e) => {
                let existing: HashSet<(u32, u32)> = mesh.edges_unique().into_iter().collect();
                match e
                    .iter()
                    .find(|&&(a, b)| !existing.contains(&edge_key(a, b)))
                {
                    Some((a, b)) => stale(format!("edge {a}-{b}")),
                    None => Ok(()),
                }
            }
            Self::Faces(f) => match f.iter().find(|&&i| i >= mesh.faces.len()) {
                Some(i) => stale(format!("face {i}")),
                None => Ok(()),
            },
        }
    }
}

/// Como um conjunto se combina com a seleção atual.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectionOp {
    /// Substitui toda a seleção de componentes.
    #[default]
    Replace,
    Add,
    /// Alterna cada item (Shift-clique).
    Toggle,
    Subtract,
}

fn combine<T: PartialEq + Copy>(current: &mut Vec<T>, items: &[T], op: SelectionOp) {
    match op {
        SelectionOp::Replace => {
            current.clear();
            for &item in items {
                if !current.contains(&item) {
                    current.push(item);
                }
            }
        }
        SelectionOp::Add => {
            for &item in items {
                if !current.contains(&item) {
                    current.push(item);
                }
            }
        }
        SelectionOp::Subtract => current.retain(|x| !items.contains(x)),
        SelectionOp::Toggle => {
            for &item in items {
                if let Some(pos) = current.iter().position(|x| *x == item) {
                    current.remove(pos);
                } else {
                    current.push(item);
                }
            }
        }
    }
}

impl CommandContext<'_> {
    /// Combina `set` com a seleção de componentes do objeto ativo. Índices
    /// inexistentes falham com `StaleReference` sem mudar nada. `Replace`
    /// limpa os outros domínios. As flags legadas da malha são espelhadas
    /// (faces/edges marcam seus vértices) e a seleção é sincronizada.
    pub fn apply_component_selection(
        &mut self,
        set: &ComponentSet,
        op: SelectionOp,
    ) -> Result<(), CommandError> {
        let Some(mesh) = self.document.active_mesh() else {
            return Err(CommandError::NoActiveAsset);
        };
        set.validate(mesh)?;
        let selection = &mut self.editor.selection;
        if op == SelectionOp::Replace {
            selection.verts.clear();
            selection.edges.clear();
            selection.faces.clear();
        }
        match set {
            ComponentSet::Vertices(v) => combine(&mut selection.verts, v, op),
            ComponentSet::Edges(e) => {
                let keys: Vec<(u32, u32)> = e.iter().map(|&(a, b)| edge_key(a, b)).collect();
                selection.edges = selection
                    .edges
                    .iter()
                    .map(|&(a, b)| edge_key(a, b))
                    .collect();
                combine(&mut selection.edges, &keys, op);
            }
            ComponentSet::Faces(f) => combine(&mut selection.faces, f, op),
        }
        let selection = self.editor.selection.clone();
        if let Some(mesh) = self.document.active_mesh_mut() {
            selection.apply_to_mesh(mesh);
            // Kernels leem flags de vértice: faces e edges marcam os seus.
            for &f in &selection.faces {
                for &v in &mesh.faces[f].verts {
                    mesh.verts[v as usize].selected = true;
                }
            }
            for &(a, b) in &selection.edges {
                mesh.verts[a as usize].selected = true;
                mesh.verts[b as usize].selected = true;
            }
        }
        self.sync_selection();
        Ok(())
    }

    /// Fallback de Delete/Duplicate: sem componentes selecionados, seleciona o
    /// componente sob o cursor. Devolve se a seleção mudou.
    pub fn select_hover_if_empty(&mut self) -> bool {
        let selection = &self.editor.selection;
        if !selection.verts.is_empty() || !selection.edges.is_empty() || !selection.faces.is_empty()
        {
            return false;
        }
        let Some(set) = ComponentSet::from_hover(self.editor.tools.hover) else {
            return false;
        };
        self.apply_component_selection(&set, SelectionOp::Replace)
            .is_ok()
    }
}

/// Commit do resultado de uma ferramenta (Loop Cut, Slice, Knife…): troca a
/// malha autoral de `object` por `mesh` como **uma** entrada de Undo
/// rotulada `label`. Recusa objeto inexistente (`StaleReference`), documento
/// em outra revisão e malha que viola invariantes; congela a primitiva
/// paramétrica do objeto; seleção vem das flags da malha resultante.
#[derive(Clone, Debug)]
pub struct ApplyMeshEditCmd {
    pub label: &'static str,
    pub object: Uuid,
    /// Revisão do documento quando o preview começou.
    pub expected_revision: Option<u64>,
    pub mesh: Mesh,
}

impl Command for ApplyMeshEditCmd {
    fn label(&self) -> &'static str {
        self.label
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::GEOMETRY | ProjectChanges::UVS | ProjectChanges::SELECTION
    }

    /// A revisão é conferida antes do dispatcher congelar a primitiva (o
    /// congelamento muda a revisão e daria um falso "desatualizado").
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        match self.expected_revision {
            Some(expected) if expected != state.document.revision_id() => {
                Err("document changed since the tool preview started")
            }
            _ => Ok(()),
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let index = state
            .document
            .find(self.object)
            .ok_or_else(|| CommandError::StaleReference(self.object.to_string()))?;
        self.mesh
            .validate_invariants()
            .map_err(|e| CommandError::Execution(format!("invalid tool result: {e}")))?;
        state.document.active = index;
        state.freeze_active_primitive_for_command();
        let asset = state
            .document
            .active_mut()
            .ok_or(CommandError::NoActiveAsset)?;
        asset.mesh = self.mesh.clone().into();
        Ok(())
    }
}

/// Objeto novo com a malha de uma prévia de criação (volume de perfil etc.):
/// a prévia nunca entra no documento, só este comando. Uma entrada de Undo;
/// o objeto novo vira o ativo. A malha já está no mundo (transformação
/// identidade).
#[derive(Clone, Debug)]
pub struct AddMeshAssetCmd {
    pub label: &'static str,
    pub name: String,
    pub mesh: Mesh,
}

impl Command for AddMeshAssetCmd {
    fn label(&self) -> &'static str {
        self.label
    }

    fn can_execute(&self, _state: &EditorView<'_>) -> Result<(), &'static str> {
        if self.mesh.faces.is_empty() {
            Err("generated mesh has no faces")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        self.mesh
            .validate_invariants()
            .map_err(|e| CommandError::Execution(format!("invalid generated mesh: {e}")))?;
        state.document.add(&self.name, self.mesh.clone());
        Ok(())
    }
}

/// Insere um objeto já montado (colar objeto: cópia com material,
/// modificadores e pintura). Uma entrada de Undo; o objeto novo vira o ativo.
#[derive(Clone, Debug)]
pub struct InsertAssetCmd {
    pub label: &'static str,
    pub asset: Box<petunia_project::Asset>,
}

impl Command for InsertAssetCmd {
    fn label(&self) -> &'static str {
        self.label
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state
            .document
            .assets
            .iter()
            .any(|asset| asset.id == self.asset.id)
        {
            Err("an object with this id already exists")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.document.assets.push((*self.asset).clone());
        state.document.active = state.document.assets.len() - 1;
        Ok(())
    }
}

/// Operação sobre seams UV.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeamOp {
    Mark,
    Clear,
    Toggle,
    /// Remove todas as seams do objeto (ignora `edges`).
    ClearAll,
}

/// Seams UV do objeto ativo. `edges = None`: arestas selecionadas; sem
/// arestas, as bordas das faces selecionadas.
#[derive(Clone, Debug)]
pub struct UvSeamCmd {
    pub op: SeamOp,
    pub edges: Option<Vec<(u32, u32)>>,
}

impl Command for UvSeamCmd {
    fn label(&self) -> &'static str {
        match self.op {
            SeamOp::Mark => "mark uv seam",
            SeamOp::Clear => "clear uv seam",
            SeamOp::Toggle => "toggle uv seam",
            SeamOp::ClearAll => "clear all uv seams",
        }
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::UVS
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        if self.op == SeamOp::ClearAll {
            if mesh.uv_seams.is_empty() {
                return Err(CommandError::NoChange("there are no seams to clear".into()));
            }
            mesh.uv_seams.clear();
            state.set_status("Cleared all UV seams");
            return Ok(());
        }
        let edges: Vec<(u32, u32)> = match &self.edges {
            Some(edges) => edges.iter().map(|&(a, b)| edge_key(a, b)).collect(),
            None if !mesh.selected_edges.is_empty() => {
                let mut e: Vec<_> = mesh.selected_edges.iter().copied().collect();
                e.sort_unstable();
                e
            }
            None => {
                let mut e: Vec<(u32, u32)> = mesh
                    .faces
                    .iter()
                    .filter(|f| f.selected)
                    .flat_map(|f| {
                        let n = f.verts.len();
                        (0..n).map(move |k| edge_key(f.verts[k], f.verts[(k + 1) % n]))
                    })
                    .collect();
                e.sort_unstable();
                e.dedup();
                e
            }
        };
        if edges.is_empty() {
            return Err(CommandError::EmptySelection);
        }
        let existing: HashSet<(u32, u32)> = mesh.edges_unique().into_iter().collect();
        if let Some((a, b)) = edges.iter().find(|e| !existing.contains(e)) {
            return Err(CommandError::StaleReference(format!("edge {a}-{b}")));
        }
        for &(a, b) in &edges {
            match self.op {
                SeamOp::Mark => mesh.mark_seam(a, b),
                SeamOp::Clear => mesh.clear_seam(a, b),
                SeamOp::Toggle => mesh.toggle_seam(a, b),
                SeamOp::ClearAll => {}
            }
        }
        let total = mesh.uv_seams.len();
        state.set_status(format!(
            "UV seams on {} edge(s) · {total} total",
            edges.len()
        ));
        Ok(())
    }
}

/// Operação sobre pins UV.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinOp {
    Pin,
    Unpin,
    /// Algum canto fixado no alvo → desfixa todos; senão fixa todos.
    Toggle,
    /// Remove todos os pins do objeto (ignora `corners`).
    ClearAll,
}

/// Pins UV do objeto ativo. `corners = None`: cantos das faces selecionadas
/// no UV Editor e das faces selecionadas em 3D.
#[derive(Clone, Debug)]
pub struct UvPinCmd {
    pub op: PinOp,
    pub corners: Option<Vec<FaceCornerIndex>>,
}

impl Command for UvPinCmd {
    fn label(&self) -> &'static str {
        match self.op {
            PinOp::Pin => "pin uv",
            PinOp::Unpin => "unpin uv",
            PinOp::Toggle => "toggle uv pin",
            PinOp::ClearAll => "clear all uv pins",
        }
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::UVS
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let uv_faces = match state.document.active_mesh() {
            Some(mesh) => state.editor.tools.uv_selection.faces(mesh),
            None => return Err(CommandError::NoActiveAsset),
        };
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        if self.op == PinOp::ClearAll {
            if mesh.uv_pinned.is_empty() {
                return Err(CommandError::NoChange("there are no pins to clear".into()));
            }
            mesh.clear_all_pins();
            state.set_status("Cleared all UV pins");
            return Ok(());
        }
        let corners: Vec<FaceCornerIndex> = match &self.corners {
            Some(c) => c.clone(),
            None => mesh
                .faces
                .iter()
                .enumerate()
                .filter(|(fi, f)| f.selected || uv_faces.contains(fi))
                .flat_map(|(fi, f)| (0..f.uv.len()).map(move |c| FaceCornerIndex::new(fi, c)))
                .collect(),
        };
        if corners.is_empty() {
            return Err(CommandError::EmptySelection);
        }
        if let Some(bad) = corners.iter().find(|c| {
            mesh.faces
                .get(c.face)
                .is_none_or(|f| c.corner >= f.uv.len())
        }) {
            return Err(CommandError::StaleReference(format!(
                "corner {}:{}",
                bad.face, bad.corner
            )));
        }
        let pin = match self.op {
            PinOp::Pin => true,
            PinOp::Unpin | PinOp::ClearAll => false,
            PinOp::Toggle => !corners.iter().any(|c| mesh.is_uv_pinned(c.face, c.corner)),
        };
        for c in &corners {
            if pin {
                mesh.pin_uv(c.face, c.corner);
            } else {
                mesh.unpin_uv(c.face, c.corner);
            }
        }
        let total = mesh.uv_pinned.len();
        state.set_status(format!(
            "{} {} UV corner(s) · {total} pinned",
            if pin { "Pinned" } else { "Unpinned" },
            corners.len()
        ));
        Ok(())
    }
}

/// Atribui o material `material_index` (índice em `Project::materials`) ao
/// objeto ativo e, quando houver faces alvo, grava o slot nelas.
/// `faces = None`: faces selecionadas.
#[derive(Clone, Debug)]
pub struct AssignMaterialSlotCmd {
    pub material_index: usize,
    pub faces: Option<Vec<usize>>,
}

impl Command for AssignMaterialSlotCmd {
    fn label(&self) -> &'static str {
        "assign material"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::MATERIALS
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(material_id) = state
            .document
            .materials
            .get(self.material_index)
            .map(|m| m.id)
        else {
            return Err(CommandError::StaleReference(format!(
                "material {}",
                self.material_index
            )));
        };
        let Some(asset) = state.document.active_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        let faces: Vec<usize> = match &self.faces {
            Some(f) => f.clone(),
            None => asset
                .mesh
                .faces
                .iter()
                .enumerate()
                .filter_map(|(i, f)| f.selected.then_some(i))
                .collect(),
        };
        if let Some(bad) = faces.iter().find(|&&f| f >= asset.mesh.faces.len()) {
            return Err(CommandError::StaleReference(format!("face {bad}")));
        }
        asset.material_id = Some(material_id);
        if !faces.is_empty() {
            let mesh = &mut *asset.mesh;
            for &f in &faces {
                mesh.faces[f].material_slot = Some(self.material_index);
            }
        }
        state.set_status(if faces.is_empty() {
            format!("Assigned material slot {} to object", self.material_index)
        } else {
            format!(
                "Assigned material slot {} to object and {} face(s)",
                self.material_index,
                faces.len()
            )
        });
        Ok(())
    }
}

/// Desloca a seleção por `delta` (setas do teclado). Edit Mode: vértices
/// selecionados. Object Mode: o objeto inteiro — hoje ainda movendo vértices;
/// passa a mover só o transform quando a autoridade de transform migrar (G10).
#[derive(Clone, Debug)]
pub struct NudgeCmd {
    pub delta: [f32; 3],
}

impl Command for NudgeCmd {
    fn label(&self) -> &'static str {
        "nudge"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::POSITIONS
    }

    fn can_execute(&self, _state: &EditorView<'_>) -> Result<(), &'static str> {
        let d = glam::Vec3::from_array(self.delta);
        if !d.is_finite() {
            Err("nudge delta must be finite")
        } else if d.length_squared() < 1.0e-8 {
            Err("nudge delta is zero")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let edit = state.editor.edit_mode() == crate::EditMode::Edit;
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        if edit {
            if !mesh.verts.iter().any(|v| v.selected) {
                return Err(CommandError::EmptySelection);
            }
            mesh.translate_selected(self.delta);
        } else {
            for v in &mut mesh.verts {
                for (axis, d) in v.pos.iter_mut().zip(self.delta) {
                    *axis += d;
                }
            }
        }
        let [dx, dy, dz] = self.delta;
        state.set_status(format!("Nudge {dx:+.2}, {dy:+.2}, {dz:+.2}"));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppState;

    fn state() -> AppState {
        let mut state = AppState::new("en");
        state.set_edit_mode(crate::EditMode::Edit);
        state
    }

    #[test]
    fn component_selection_ops_combine_and_reject_stale_indices() {
        let mut s = state();
        s.apply_component_selection(&ComponentSet::Faces(vec![0, 1]), SelectionOp::Replace)
            .unwrap();
        s.apply_component_selection(&ComponentSet::Faces(vec![1, 2]), SelectionOp::Toggle)
            .unwrap();
        let mut faces = s.session.selection.faces.clone();
        faces.sort_unstable();
        assert_eq!(faces, vec![0, 2]);
        s.apply_component_selection(&ComponentSet::Faces(vec![0]), SelectionOp::Subtract)
            .unwrap();
        assert_eq!(s.session.selection.faces, vec![2]);
        assert!(matches!(
            s.apply_component_selection(&ComponentSet::Faces(vec![9999]), SelectionOp::Add),
            Err(CommandError::StaleReference(_))
        ));
        assert_eq!(s.session.selection.faces, vec![2]);
    }

    #[test]
    fn hover_fallback_only_when_nothing_is_selected() {
        let mut s = state();
        s.session.tools.hover = HoverTarget::Face(3);
        assert!(s.select_hover_if_empty());
        assert_eq!(s.session.selection.faces, vec![3]);
        s.session.tools.hover = HoverTarget::Face(4);
        assert!(!s.select_hover_if_empty());
    }

    #[test]
    fn tool_results_are_one_undo_entry_and_refuse_stale_objects() {
        let mut s = state();
        let object = s.project.active().unwrap().id;
        let mut mesh = s.project.active_mesh().unwrap().clone();
        mesh.translate_selected([0.0; 3]);
        let undo_before = s.project.undo.depth().0;
        s.dispatch(&ApplyMeshEditCmd {
            label: "loop cut",
            object,
            expected_revision: Some(s.project.project.revision_id()),
            mesh: mesh.clone(),
        })
        .unwrap();
        assert_eq!(s.project.undo.depth().0, undo_before + 1);
        assert_eq!(s.project.undo.undo_label(), Some("loop cut"));

        let stale = s.dispatch(&ApplyMeshEditCmd {
            label: "loop cut",
            object: Uuid::new_v4(),
            expected_revision: None,
            mesh: mesh.clone(),
        });
        assert!(matches!(stale, Err(CommandError::StaleReference(_))));
        let outdated = s.dispatch(&ApplyMeshEditCmd {
            label: "loop cut",
            object,
            expected_revision: Some(u64::MAX),
            mesh,
        });
        assert!(outdated.is_err());
        assert_eq!(s.project.undo.depth().0, undo_before + 1);
    }

    #[test]
    fn invalid_tool_meshes_are_rejected_without_change() {
        let mut s = state();
        let object = s.project.active().unwrap().id;
        let faces_before = s.project.active_mesh().unwrap().faces.len();
        let mut bad = s.project.active_mesh().unwrap().clone();
        bad.verts[0].pos[0] = f32::NAN;
        assert!(
            s.dispatch(&ApplyMeshEditCmd {
                label: "slice",
                object,
                expected_revision: None,
                mesh: bad,
            })
            .is_err()
        );
        assert!(s.project.active_mesh().unwrap().verts[0].pos[0].is_finite());
        assert_eq!(s.project.active_mesh().unwrap().faces.len(), faces_before);
    }

    #[test]
    fn seams_pins_material_and_nudge_are_commands() {
        let mut s = state();
        s.apply_component_selection(&ComponentSet::Faces(vec![0]), SelectionOp::Replace)
            .unwrap();
        s.dispatch(&UvSeamCmd {
            op: SeamOp::Mark,
            edges: None,
        })
        .unwrap();
        assert!(!s.project.active_mesh().unwrap().uv_seams.is_empty());
        s.dispatch(&UvSeamCmd {
            op: SeamOp::ClearAll,
            edges: None,
        })
        .unwrap();
        assert!(s.project.active_mesh().unwrap().uv_seams.is_empty());

        s.dispatch(&UvPinCmd {
            op: PinOp::Toggle,
            corners: None,
        })
        .unwrap();
        assert!(s.project.active_mesh().unwrap().is_uv_pinned(0, 0));
        assert!(matches!(
            s.dispatch(&UvPinCmd {
                op: PinOp::Pin,
                corners: Some(vec![FaceCornerIndex::new(9999, 0)]),
            }),
            Err(CommandError::StaleReference(_))
        ));

        let x_before = s.project.active_mesh().unwrap().verts
            [s.project.active_mesh().unwrap().faces[0].verts[0] as usize]
            .pos[0];
        s.dispatch(&NudgeCmd {
            delta: [0.5, 0.0, 0.0],
        })
        .unwrap();
        let mesh = s.project.active_mesh().unwrap();
        let x_after = mesh.verts[mesh.faces[0].verts[0] as usize].pos[0];
        assert!((x_after - x_before - 0.5).abs() < 1e-5);
        assert!(s.dispatch(&NudgeCmd { delta: [0.0; 3] }).is_err());
    }
}
