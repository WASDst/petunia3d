//! Explicit topology-result contract for modeling operations.
//!
//! Operations return this so selection, UV, paint attachments and renderer
//! revisions can remap without guessing.

use serde::{Deserialize, Serialize};

/// Which derived domains a topology/geometry edit invalidated.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirtyDomains {
    pub topology: bool,
    pub positions: bool,
    pub selection: bool,
    pub uv: bool,
    pub paint_attachments: bool,
}

impl DirtyDomains {
    pub fn topology_edit() -> Self {
        Self {
            topology: true,
            positions: true,
            selection: true,
            uv: true,
            paint_attachments: true,
        }
    }

    pub fn positions_only() -> Self {
        Self {
            topology: false,
            positions: true,
            selection: false,
            uv: false,
            paint_attachments: false,
        }
    }
}

/// Old → new identity after a merge/split. Missing entries were deleted.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementRemap {
    pub vertices: Vec<(u32, Option<u32>)>,
    pub faces: Vec<(usize, Option<usize>)>,
}

impl ElementRemap {
    pub fn identity() -> Self {
        Self::default()
    }

    pub fn map_vertex(&self, old: u32) -> Option<u32> {
        match self.vertices.iter().find(|(k, _)| *k == old) {
            Some((_, v)) => *v,
            None => Some(old),
        }
    }

    pub fn map_face(&self, old: usize) -> Option<usize> {
        match self.faces.iter().find(|(k, _)| *k == old) {
            Some((_, v)) => *v,
            None => Some(old),
        }
    }
}

/// Description of a modeling operation's effect on authoring topology.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyResult {
    pub verts_added: usize,
    pub faces_added: usize,
    pub verts_removed: usize,
    pub faces_removed: usize,
    pub remap: ElementRemap,
    pub uv_faces_touched: Vec<usize>,
    pub invalidated: DirtyDomains,
}

impl TopologyResult {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn from_counts(
        verts_before: usize,
        faces_before: usize,
        verts_after: usize,
        faces_after: usize,
    ) -> Self {
        Self {
            verts_added: verts_after.saturating_sub(verts_before),
            faces_added: faces_after.saturating_sub(faces_before),
            verts_removed: verts_before.saturating_sub(verts_after),
            faces_removed: faces_before.saturating_sub(faces_after),
            remap: ElementRemap::identity(),
            uv_faces_touched: Vec::new(),
            invalidated: DirtyDomains::topology_edit(),
        }
    }
}
