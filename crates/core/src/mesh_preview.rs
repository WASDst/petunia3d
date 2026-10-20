//! Transação de ferramentas de corte com vários estágios de interação.
use crate::Selection;
use crate::context::{CommandContext, EditorView};
use petunia_mesh::Mesh;
use petunia_project::Project;

pub struct MeshPreview {
    pub(crate) original: Project,
    pub(crate) selection: Selection,
    pub(crate) label: &'static str,
    pub(crate) changed: bool,
}

impl CommandContext<'_> {
    pub fn begin_mesh_preview(&mut self, label: &'static str) -> bool {
        if self.is_interacting() {
            return false;
        }
        self.mesh_preview = Some(MeshPreview {
            original: self.document.project.clone(),
            selection: self.editor.selection.clone(),
            label,
            changed: false,
        });
        self.mark_dirty();
        true
    }

    pub fn preview_mesh(&mut self, mesh: Mesh) {
        if let Some(preview) = self.editor.tools.mesh_preview.as_mut() {
            if let Some(active) = self.document.active_mesh_mut() {
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
        let Some(preview) = self.editor.tools.mesh_preview.take() else {
            return;
        };
        let restored = cancel || !preview.changed;
        if restored {
            let revision_clock = self.document.project.revision_clock();
            self.document.project = preview.original;
            self.document
                .project
                .rebase_revisions_after_restore(revision_clock);
            self.editor.selection = preview.selection;
        } else {
            self.document
                .checkpoint_snapshot(preview.label, &preview.original);
        }
        self.editor.tools.cut_session = None;
        self.sync_selection();
        if restored {
            self.emit_mesh_changed();
        }
    }
}

impl<'a> EditorView<'a> {
    pub fn is_interacting(self) -> bool {
        self.uv_transform.is_some()
            || self.modal.is_some()
            || self.pending_modal.is_some()
            || self.mesh_preview.is_some()
            || self.paint_stroke.is_some()
    }
}
