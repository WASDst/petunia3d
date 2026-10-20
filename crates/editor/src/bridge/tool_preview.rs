//! Prévia e commit das sessões de ferramenta com resultado de malha.
//!
//! Contrato de `website/docs/03-geometry/draw-unification-proposal.md` (tabela de
//! donos): **prévia e overlays** viajam pelo contrato de render — nunca como malha
//! transitória gravada no Document — e o **gesto** vive na Tool Session. A
//! gramática de `direct-modeling-semantics.md` ("Undo / ToolSession"):
//!
//! ```text
//! begin → preview from fixed source → updates → commit = one Undo → Esc = exact restoration
//! ```
//!
//! Com a prévia fora do documento, Esc é restauração exata por construção (o
//! documento nunca mudou) e o commit é o único ponto que escreve: uma entrada de
//! Undo, e só se a fonte da sessão ainda for o modelo atual.

use std::sync::Arc;

use petunia_project::PoseOverride;

use crate::*;

/// Revisões de prévia de ferramenta ficam num espaço próprio, para nunca
/// coincidir com as revisões da pose do Animate no cache do renderer.
const TOOL_PREVIEW_REVISION_BIT: u64 = 1 << 63;

/// A malha do documento ainda é a fonte da sessão (mesma topologia e posições)?
/// Undo, Redo ou outra edição durante a ferramenta invalidam a prévia.
pub(crate) fn same_geometry(a: &petunia_core::Mesh, b: &petunia_core::Mesh) -> bool {
    a.verts.len() == b.verts.len()
        && a.faces.len() == b.faces.len()
        && a.verts.iter().zip(&b.verts).all(|(x, y)| {
            x.pos
                .iter()
                .zip(&y.pos)
                .all(|(p, q)| p.to_bits() == q.to_bits())
        })
        && a.faces
            .iter()
            .zip(&b.faces)
            .all(|(x, y)| x.verts == y.verts)
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Mostra `mesh` no lugar da malha ativa, só no viewport.
    pub fn show_tool_preview(&mut self, mesh: petunia_core::Mesh) -> bool {
        let Some(id) = self.state.project.active().map(|asset| asset.id) else {
            return false;
        };
        self.tool_preview_revision = self.tool_preview_revision.wrapping_add(1);
        let mut preview = PoseOverride::new(TOOL_PREVIEW_REVISION_BIT | self.tool_preview_revision);
        preview.insert(id, mesh);
        self.tool_preview = Some(Arc::new(preview));
        self.state.mark_dirty();
        true
    }

    /// Mostra `mesh` como objeto novo, só no viewport (prévia de criação: o
    /// objeto nasce no commit, por `AddMeshAssetCmd`). `id` estável durante a
    /// sessão deixa o renderer reaproveitar o objeto entre quadros.
    pub(crate) fn show_new_object_preview(
        &mut self,
        id: uuid::Uuid,
        mesh: petunia_core::Mesh,
    ) -> bool {
        self.tool_preview_revision = self.tool_preview_revision.wrapping_add(1);
        let mut preview = PoseOverride::new(TOOL_PREVIEW_REVISION_BIT | self.tool_preview_revision);
        preview.insert_transient(id, mesh);
        self.tool_preview = Some(Arc::new(preview));
        self.state.mark_dirty();
        true
    }

    /// Malha da prévia de objeto novo, se houver (HUD e testes).
    pub fn new_object_preview_mesh(&self) -> Option<&petunia_core::Mesh> {
        let preview = self.tool_preview.as_ref()?;
        preview.transient().first().map(|(_, mesh)| mesh)
    }

    /// Remove a prévia; `true` se havia uma.
    pub(crate) fn clear_tool_preview(&mut self) -> bool {
        let had = self.tool_preview.take().is_some();
        if had {
            self.state.mark_dirty();
        }
        had
    }

    /// Malha mostrada pela prévia de ferramenta, se houver (HUD e testes).
    pub fn tool_preview_mesh(&self) -> Option<&petunia_core::Mesh> {
        let preview = self.tool_preview.as_ref()?;
        let id = self.state.project.active()?.id;
        preview.mesh_for(id)
    }

    /// Feedback de modelagem da operação ativa ou prévia em andamento (H28 / MF-001–008).
    pub fn current_modeling_feedback(&self) -> Option<petunia_core::OperationFeedback> {
        self.state.current_modeling_feedback()
    }

    /// Prévia a desenhar neste frame: a da ferramenta vence a pose do Animate.
    pub(crate) fn draw_override(&self) -> Option<Arc<PoseOverride>> {
        self.tool_preview.clone()
    }

    /// Ponto único de commit das ferramentas de malha: despacha
    /// `ApplyMeshEditCmd` (uma entrada de Undo, invariantes validadas, objeto
    /// conferido por id). Recusa quando o documento já não é `source` — a
    /// prévia foi calculada sobre outro modelo (Undo/edição durante a ferramenta).
    pub(crate) fn commit_tool_result(
        &mut self,
        label: &'static str,
        source: &petunia_core::Mesh,
        result: petunia_core::Mesh,
    ) -> bool {
        self.clear_tool_preview();
        let current_is_source = self
            .state
            .project
            .active_mesh()
            .is_some_and(|mesh| same_geometry(mesh, source));
        let Some(object) = self.state.project.active().map(|asset| asset.id) else {
            return false;
        };
        if !current_is_source {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_TOOL_SOURCE_CHANGED),
            );
            return false;
        }
        let command = petunia_core::ApplyMeshEditCmd {
            label,
            object,
            expected_revision: None,
            mesh: result,
        };
        match self.state.dispatch(&command) {
            Ok(_) => {
                self.state.mark_dirty();
                true
            }
            Err(error) => {
                self.state.set_status(format!("{label}: {error}"));
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_geometry_tracks_positions_and_topology() {
        let cube = petunia_core::Mesh::cube(2.0);
        assert!(same_geometry(&cube, &cube.clone()));
        let mut moved = cube.clone();
        moved.verts[0].pos[0] += 0.001;
        assert!(!same_geometry(&cube, &moved));
        let mut fewer = cube.clone();
        fewer.faces.pop();
        assert!(!same_geometry(&cube, &fewer));
    }
}
