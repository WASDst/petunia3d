//! UV previews are derived from a fixed source and never mutate the document.
use crate::{AppState, Command, CommandContext, CommandError, EditorView, UvSelectionState};
use petunia_mesh::{FaceCornerIndex, Mesh, UvTransform};
use petunia_project::ProjectChanges;
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UvTransformKind {
    Move,
    Rotate,
    Scale,
}

#[derive(Clone)]
pub struct UvTransformSession {
    pub kind: UvTransformKind,
    asset_id: Uuid,
    revision: [u64; 11],
    source: Mesh,
    preview: Mesh,
    selection: UvSelectionState,
    corners: HashSet<FaceCornerIndex>,
}

impl UvTransformSession {
    pub fn preview_mesh(&self) -> &Mesh {
        &self.preview
    }

    fn validate(&self, view: &EditorView<'_>) -> Result<(), CommandError> {
        let asset = view.document.active().ok_or(CommandError::NoActiveAsset)?;
        if asset.id != self.asset_id || view.document.project.revision_clock() != self.revision {
            return Err(CommandError::StaleReference(
                "UV transform source changed".into(),
            ));
        }
        if asset.locked {
            return Err(CommandError::Execution("Asset is locked".into()));
        }
        if view.editor.uv_selection != self.selection {
            return Err(CommandError::StaleReference(
                "UV selection changed during the gesture".into(),
            ));
        }
        if asset.mesh.faces.len() != self.source.faces.len()
            || asset.mesh.verts.len() != self.source.verts.len()
            || asset
                .mesh
                .faces
                .iter()
                .zip(&self.source.faces)
                .any(|(a, b)| a.verts != b.verts || a.uv != b.uv)
        {
            return Err(CommandError::StaleReference(
                "UV topology or coordinates changed".into(),
            ));
        }
        Ok(())
    }
}

impl AppState {
    pub fn begin_uv_transform(&mut self, kind: UvTransformKind) -> Result<(), CommandError> {
        if self.is_interacting() {
            return Err(CommandError::Execution(
                "Confirm or cancel the active operation first".into(),
            ));
        }
        let asset = self.project.active().ok_or(CommandError::NoActiveAsset)?;
        if asset.locked {
            return Err(CommandError::Execution("Asset is locked".into()));
        }
        let corners = self.session.uv_selection.corner_set();
        if corners.is_empty() {
            return Err(CommandError::EmptySelection);
        }
        for corner in &corners {
            let uv = asset
                .mesh
                .faces
                .get(corner.face)
                .and_then(|f| f.uv.get(corner.corner))
                .ok_or_else(|| CommandError::StaleReference("UV corner no longer exists".into()))?;
            if !uv.iter().all(|v| v.is_finite()) {
                return Err(CommandError::Execution("Non-finite source UV".into()));
            }
        }
        let session = UvTransformSession {
            kind,
            asset_id: asset.id,
            revision: self.project.project.revision_clock(),
            source: (*asset.mesh).clone(),
            preview: (*asset.mesh).clone(),
            selection: self.session.uv_selection.clone(),
            corners,
        };
        self.session.tools.uv_transform = Some(session);
        self.mark_dirty();
        Ok(())
    }

    /// Absolute transform from the initial source. Repeating an update cannot drift.
    pub fn update_uv_transform(&mut self, transform: UvTransform) -> Result<(), CommandError> {
        if !transform.is_finite() {
            return Err(CommandError::Execution("Non-finite UV transform".into()));
        }
        let session = self
            .session
            .tools
            .uv_transform
            .as_ref()
            .ok_or_else(|| CommandError::Execution("No UV transform active".into()))?;
        session.validate(&self.view())?;
        if !matches!(
            (session.kind, transform),
            (UvTransformKind::Move, UvTransform::Translate(_))
                | (UvTransformKind::Rotate, UvTransform::Rotate(_))
                | (UvTransformKind::Scale, UvTransform::Scale(_))
        ) {
            return Err(CommandError::Execution(
                "UV transform kind does not match the active gesture".into(),
            ));
        }
        let mut preview = session.source.clone();
        preview.transform_uv_corners(&session.corners, transform);
        if preview
            .faces
            .iter()
            .flat_map(|f| &f.uv)
            .flatten()
            .any(|v| !v.is_finite())
        {
            return Err(CommandError::Execution("UV transform overflow".into()));
        }
        if let Some(session) = self.session.tools.uv_transform.as_mut() {
            session.preview = preview;
        }
        self.mark_dirty();
        Ok(())
    }

    pub fn cancel_uv_transform(&mut self) -> bool {
        let cancelled = self.session.tools.uv_transform.take().is_some();
        if cancelled {
            self.mark_dirty();
        }
        cancelled
    }

    pub fn commit_uv_transform(&mut self) -> Result<(), CommandError> {
        let session = self
            .session
            .tools
            .uv_transform
            .as_ref()
            .ok_or_else(|| CommandError::Execution("No UV transform active".into()))?;
        session.validate(&self.view())?;
        let session = self
            .session
            .tools
            .uv_transform
            .take()
            .ok_or_else(|| CommandError::Execution("No UV transform active".into()))?;
        self.dispatch(&CommitUvTransformCmd { session })
    }
}

struct CommitUvTransformCmd {
    session: UvTransformSession,
}
impl Command for CommitUvTransformCmd {
    fn label(&self) -> &'static str {
        match self.session.kind {
            UvTransformKind::Move => "move uv",
            UvTransformKind::Rotate => "rotate uv",
            UvTransformKind::Scale => "scale uv",
        }
    }
    fn changes(&self) -> ProjectChanges {
        ProjectChanges::UVS
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        self.session.validate(&state.view())?;
        if self
            .session
            .source
            .faces
            .iter()
            .zip(&self.session.preview.faces)
            .all(|(a, b)| a.uv == b.uv)
        {
            return Err(CommandError::NoChange("UV preview is unchanged".into()));
        }
        let mesh = state
            .document
            .active_mesh_mut()
            .ok_or(CommandError::NoActiveAsset)?;
        for corner in &self.session.corners {
            let uv = self
                .session
                .preview
                .faces
                .get(corner.face)
                .and_then(|f| f.uv.get(corner.corner))
                .ok_or_else(|| CommandError::StaleReference("UV preview corner missing".into()))?;
            let target = mesh
                .faces
                .get_mut(corner.face)
                .and_then(|f| f.uv.get_mut(corner.corner))
                .ok_or_else(|| CommandError::StaleReference("UV corner missing".into()))?;
            *target = *uv;
        }
        Ok(())
    }
}
