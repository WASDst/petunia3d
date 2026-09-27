//! Seleção e workspaces (pílulas da UI).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SelectMode {
    #[default]
    Vertex,
    Edge,
    Face,
}

/// Domínio unificado de interação e seleção (P3D-015 / P3D-016).
/// Elimina a divisão artificial entre Object Mode e Edit Mode na UX.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SelectionDomain {
    #[default]
    Object,
    Vertex,
    Edge,
    Face,
}

impl SelectionDomain {
    pub fn is_component(&self) -> bool {
        matches!(self, Self::Vertex | Self::Edge | Self::Face)
    }

    pub fn as_select_mode(&self) -> Option<SelectMode> {
        match self {
            Self::Vertex => Some(SelectMode::Vertex),
            Self::Edge => Some(SelectMode::Edge),
            Self::Face => Some(SelectMode::Face),
            Self::Object => None,
        }
    }

    pub fn from_select_mode(mode: SelectMode) -> Self {
        match mode {
            SelectMode::Vertex => Self::Vertex,
            SelectMode::Edge => Self::Edge,
            SelectMode::Face => Self::Face,
        }
    }

    pub fn key(&self) -> &'static str {
        match self {
            Self::Object => "modes.object",
            Self::Vertex => "modes.vertex",
            Self::Edge => "modes.edge",
            Self::Face => "modes.face",
        }
    }

    pub fn shortcut(&self) -> &'static str {
        match self {
            Self::Object => "0",
            Self::Vertex => "1",
            Self::Edge => "2",
            Self::Face => "3",
        }
    }
}

/// Workspaces do Petunia3D.
///
/// A V1 congela `MODEL / PAINT / UV` (capítulo 36 do Livro Vivo). O workspace de
/// animação pertence a P3D-066 (prioridade P3) e **não** faz parte da baseline
/// congelada: ele só é compilado com a feature `animation-workspace`, para que o
/// build de V1 nunca exponha um workspace fora do escopo aprovado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Workspace {
    #[default]
    Model,
    Paint,
    Uv,
    #[cfg(feature = "animation-workspace")]
    Animate,
}

impl Workspace {
    /// Workspaces compilados neste build, na ordem das pílulas do header.
    #[cfg(not(feature = "animation-workspace"))]
    pub const fn all() -> [Workspace; 3] {
        [Workspace::Model, Workspace::Paint, Workspace::Uv]
    }

    /// Workspaces compilados neste build, na ordem das pílulas do header.
    #[cfg(feature = "animation-workspace")]
    pub const fn all() -> [Workspace; 4] {
        [
            Workspace::Model,
            Workspace::Paint,
            Workspace::Uv,
            Workspace::Animate,
        ]
    }

    /// Quantidade de workspaces deste build (tamanho do array de memória de UI).
    pub const COUNT: usize = Self::all().len();

    /// Chave de tradução do rótulo do workspace.
    pub fn key(&self) -> &'static str {
        match self {
            Workspace::Model => "ws.model",
            Workspace::Paint => "ws.paint",
            Workspace::Uv => "ws.uv",
            #[cfg(feature = "animation-workspace")]
            Workspace::Animate => "ws.animate",
        }
    }
}

/// Seleção atual (sincronizada entre viewport e UV via eventos).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    pub asset: Option<Uuid>,
    /// Ordered object selection; `asset` is the active object.
    pub assets: Vec<Uuid>,
    pub verts: Vec<u32>,
    pub faces: Vec<usize>,
    pub edges: Vec<(u32, u32)>,
}

impl Selection {
    pub fn clear(&mut self) {
        self.verts.clear();
        self.faces.clear();
        self.edges.clear();
    }
    pub fn is_empty(&self) -> bool {
        self.verts.is_empty() && self.faces.is_empty() && self.edges.is_empty()
    }
}
