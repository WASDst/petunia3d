//! Poly Pen no core (ADR 007, Onda 5): comandos transacionais usados pela
//! ferramenta, sem conhecer teclas nem toolkit.
//!
//! - [`AppState::poly_pen_add_polygon`]: polígono desenhado ponto a ponto
//!   (1 Undo);
//! - [`AppState::poly_pen_melt_point`]: derrete um ponto (1 Undo);
//! - [`AppState::begin_poly_pen_edge_extrude`]: prelúdio que extruda a aresta
//!   e abre um Move dos pontos novos — o gesto inteiro é 1 Undo e `Esc`
//!   restaura exatamente.

use glam::Vec3;
use petunia_mesh::poly_pen::{PenPoint, PolyPenError};
use petunia_project::ProjectChanges;

use crate::modal::{ModalError, ModalKind};
use crate::{AppState, SelectionDomain};

/// Por que um comando do Poly Pen foi recusado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PolyPenCommandError {
    #[error("{0}")]
    Geometry(#[from] PolyPenError),
    #[error("{0}")]
    Modal(#[from] ModalError),
    #[error("there is no active object")]
    NoActiveMesh,
    #[error("the active object is locked")]
    Locked,
}

impl AppState {
    fn poly_pen_ready(&mut self) -> Result<(), PolyPenCommandError> {
        if self.modal.is_some() {
            return Err(ModalError::AlreadyActive.into());
        }
        if self.project.active_mesh().is_none() {
            return Err(PolyPenCommandError::NoActiveMesh);
        }
        if self.is_active_locked() {
            return Err(PolyPenCommandError::Locked);
        }
        Ok(())
    }

    /// Direção da superfície para a câmera, para orientar polígonos soltos.
    fn poly_pen_viewer(&self) -> Vec3 {
        -self.session.camera.forward()
    }

    /// Adiciona o polígono desenhado e seleciona a face nova.
    pub fn poly_pen_add_polygon(
        &mut self,
        points: &[PenPoint],
    ) -> Result<usize, PolyPenCommandError> {
        self.poly_pen_ready()?;
        let viewer = self.poly_pen_viewer();
        // Validar numa cópia: erro não toca o documento nem o histórico.
        let mut mesh = self
            .project
            .active_mesh()
            .ok_or(PolyPenCommandError::NoActiveMesh)?
            .clone();
        let face = mesh.add_pen_polygon(points, viewer)?;
        mesh.deselect_all();
        mesh.faces[face].selected = true;
        mesh.sync_vert_selection_from_faces();
        self.freeze_active_primitive_for_command();
        self.project.checkpoint("Poly Pen");
        if let Some(active) = self.project.project.active_mesh_mut() {
            *active = mesh;
        }
        self.set_selection_domain(SelectionDomain::Face);
        self.sync_selection();
        self.emit_project_changed(ProjectChanges::GEOMETRY | ProjectChanges::SELECTION);
        self.mark_dirty();
        Ok(face)
    }

    /// Derrete o ponto (une as faces ao redor), como o Ctrl-clique do C4D.
    pub fn poly_pen_melt_point(&mut self, point: u32) -> Result<(), PolyPenCommandError> {
        self.poly_pen_ready()?;
        let mut mesh = self
            .project
            .active_mesh()
            .ok_or(PolyPenCommandError::NoActiveMesh)?
            .clone();
        if point as usize >= mesh.verts.len() {
            return Err(PolyPenError::MissingPoint.into());
        }
        let before = (mesh.verts.len(), mesh.faces.len());
        mesh.dissolve_vertices(&[point]);
        if (mesh.verts.len(), mesh.faces.len()) == before {
            return Err(PolyPenError::Degenerate.into());
        }
        self.freeze_active_primitive_for_command();
        self.project.checkpoint("Poly Pen: Melt");
        if let Some(active) = self.project.project.active_mesh_mut() {
            *active = mesh;
        }
        self.sync_selection();
        self.emit_project_changed(ProjectChanges::GEOMETRY | ProjectChanges::SELECTION);
        self.mark_dirty();
        Ok(())
    }

    /// Extruda a aresta de borda `a–b` e abre um Move da aresta nova.
    pub fn begin_poly_pen_edge_extrude(
        &mut self,
        a: u32,
        b: u32,
    ) -> Result<(), PolyPenCommandError> {
        self.poly_pen_ready()?;
        let before = self.project.project.clone();
        let before_selection = self.session.selection.clone();
        let mut mesh = self
            .project
            .active_mesh()
            .ok_or(PolyPenCommandError::NoActiveMesh)?
            .clone();
        mesh.extrude_edge(a, b)?;
        // Domínio antes da malha nova: a conversão de seleção não pode
        // estender a aresta para as faces vizinhas.
        self.set_selection_domain(SelectionDomain::Edge);
        self.freeze_active_primitive_for_command();
        if let Some(active) = self.project.project.active_mesh_mut() {
            *active = mesh;
        }
        self.emit_project_changed(ProjectChanges::GEOMETRY | ProjectChanges::SELECTION);
        self.begin_modal_after_prelude(ModalKind::Move, before, before_selection)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ViewPreset;

    fn front_state() -> AppState {
        let mut state = AppState::default();
        state.session.camera.set_preset(ViewPreset::Front);
        state
    }

    /// Aresta de borda do cubo padrão? O cubo é fechado: cria uma borda
    /// apagando uma face para os testes de extrusão.
    fn open_box_state() -> (AppState, u32, u32) {
        let mut state = front_state();
        let mesh = state.project.project.active_mesh_mut().unwrap();
        let removed = mesh.faces.remove(0);
        let (a, b) = (removed.verts[0], removed.verts[1]);
        (state, a, b)
    }

    #[test]
    fn drawn_polygon_is_one_undo_and_selected() {
        let mut state = front_state();
        let (verts, faces) = {
            let mesh = state.project.active_mesh().unwrap();
            (mesh.verts.len(), mesh.faces.len())
        };
        let depth = state.project.undo.depth().0;
        let points = [
            PenPoint::New([3.0, 0.0, 0.0]),
            PenPoint::New([4.0, 0.0, 0.0]),
            PenPoint::New([4.0, 1.0, 0.0]),
        ];
        let face = state.poly_pen_add_polygon(&points).unwrap();
        let mesh = state.project.active_mesh().unwrap();
        assert_eq!(mesh.verts.len(), verts + 3);
        assert_eq!(mesh.faces.len(), faces + 1);
        assert!(mesh.faces[face].selected);
        assert!(
            mesh.face_normal(face).z > 0.9,
            "voltado para a câmera Front"
        );
        assert_eq!(state.project.undo.depth().0, depth + 1);
        assert!(state.undo());
        assert_eq!(state.project.active_mesh().unwrap().faces.len(), faces);
    }

    #[test]
    fn refused_polygon_leaves_history_untouched() {
        let mut state = front_state();
        let depth = state.project.undo.depth();
        let points = [
            PenPoint::Existing(0),
            PenPoint::Existing(0),
            PenPoint::Existing(1),
        ];
        assert!(state.poly_pen_add_polygon(&points).is_err());
        assert_eq!(state.project.undo.depth(), depth);
    }

    #[test]
    fn edge_extrude_gesture_is_one_undo_and_escape_restores() {
        let (mut state, a, b) = open_box_state();
        let faces = state.project.active_mesh().unwrap().faces.len();
        let depth = state.project.undo.depth().0;

        state.begin_poly_pen_edge_extrude(a, b).unwrap();
        state.update_modal(Vec3::new(0.0, 0.0, 0.5), 0.5).unwrap();
        assert!(state.commit_modal());
        let mesh = state.project.active_mesh().unwrap();
        assert_eq!(mesh.faces.len(), faces + 1);
        assert_eq!(state.project.undo.depth().0, depth + 1);

        let (mut state, a, b) = open_box_state();
        let before = state.project.active_mesh().unwrap().clone();
        state.begin_poly_pen_edge_extrude(a, b).unwrap();
        state.update_modal(Vec3::X, 1.0).unwrap();
        assert!(state.cancel_modal());
        let after = state.project.active_mesh().unwrap();
        assert_eq!(after.faces.len(), before.faces.len());
        assert_eq!(after.verts.len(), before.verts.len());
    }

    #[test]
    fn closed_box_edges_are_not_border_edges() {
        let mut state = front_state();
        let depth = state.project.undo.depth();
        let face = state.project.active_mesh().unwrap().faces[0].verts.clone();
        assert!(matches!(
            state.begin_poly_pen_edge_extrude(face[0], face[1]),
            Err(PolyPenCommandError::Geometry(PolyPenError::NotBorderEdge))
        ));
        assert_eq!(state.project.undo.depth(), depth);
        assert!(state.modal.is_none());
    }
}
