//! Transação de ferramentas de corte com vários estágios de interação.
use crate::{AppState, Selection};
use petunia_mesh::Mesh;
use petunia_project::Project;

pub struct MeshPreview {
    original: Project,
    selection: Selection,
    label: &'static str,
    changed: bool,
}

impl AppState {
    pub fn is_interacting(&self) -> bool {
        self.modal.is_some()
            || self.pending_modal.is_some()
            || self.mesh_preview.is_some()
            || self.paint_stroke.is_some()
    }
    pub fn begin_mesh_preview(&mut self, label: &'static str) -> bool {
        if self.is_interacting() {
            return false;
        }
        self.mesh_preview = Some(MeshPreview {
            original: self.project.project.clone(),
            selection: self.session.selection.clone(),
            label,
            changed: false,
        });
        self.mark_dirty();
        true
    }
    pub fn preview_mesh(&mut self, mesh: Mesh) {
        if let Some(preview) = self.session.tools.mesh_preview.as_mut() {
            if let Some(active) = self.project.active_mesh_mut() {
                preview.changed = preview.original.active_mesh().is_some_and(|original| {
                    original.verts.len() != mesh.verts.len()
                        || original.faces.len() != mesh.faces.len()
                        || original
                            .verts
                            .iter()
                            .zip(&mesh.verts)
                            .any(|(a, b)| a.pos != b.pos || a.color != b.color)
                        || original
                            .faces
                            .iter()
                            .zip(&mesh.faces)
                            .any(|(a, b)| a.verts != b.verts || a.uv != b.uv)
                });
                *active = mesh;
            }
            self.sync_selection();
            self.emit_mesh_changed();
        }
    }
    pub fn finish_mesh_preview(&mut self, cancel: bool) {
        let Some(preview) = self.session.tools.mesh_preview.take() else {
            return;
        };
        let restored = cancel || !preview.changed;
        if restored {
            let revision_clock = self.project.project.revision_clock();
            self.project.project = preview.original;
            self.project
                .project
                .rebase_revisions_after_restore(revision_clock);
            self.session.selection = preview.selection;
        } else {
            self.project
                .checkpoint_snapshot(preview.label, &preview.original);
        }
        self.session.tools.cut_session = None;
        self.sync_selection();
        if restored {
            self.emit_mesh_changed();
        }
    }
}
