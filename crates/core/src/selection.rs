//! Seleção e workspaces (pílulas da UI).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use petunia_mesh::{EdgeKey, FaceIndex, VertexIndex};

/// Domínio unificado de interação e seleção (P3D-015 / P3D-016).
///
/// Autoridade única para o domínio de interação no Core (website/docs/01-core/camera-and-selection.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SelectionDomain {
    #[default]
    Object,
    Vertex,
    Edge,
    Face,
}

/// Modo de seleção de componentes (legado).
///
/// Redundante com `SelectionDomain`. Mantido temporariamente para compatibilidade
/// com cascas e testes legados; novos consumidores devem usar exclusivamente `SelectionDomain`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SelectMode {
    #[default]
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
}

impl From<SelectMode> for SelectionDomain {
    fn from(mode: SelectMode) -> Self {
        Self::from_select_mode(mode)
    }
}

impl SelectionDomain {
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

    /// Retorna verdadeiro se o workspace ativo é ANIMATE.
    pub fn is_animate(&self) -> bool {
        #[cfg(feature = "animation-workspace")]
        {
            *self == Workspace::Animate
        }
        #[cfg(not(feature = "animation-workspace"))]
        {
            false
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

/// Estado de seleção da sessão do editor (website/docs/02-application/state-separation.md).
pub type SelectionState = Selection;

impl Selection {
    pub fn clear(&mut self) {
        self.verts.clear();
        self.faces.clear();
        self.edges.clear();
    }
    pub fn is_empty(&self) -> bool {
        self.verts.is_empty() && self.faces.is_empty() && self.edges.is_empty()
    }

    pub fn contains_vertex(&self, v: u32) -> bool {
        self.verts.contains(&v)
    }

    pub fn contains_face(&self, f: usize) -> bool {
        self.faces.contains(&f)
    }

    pub fn contains_edge(&self, edge: (u32, u32)) -> bool {
        let key = if edge.0 < edge.1 {
            edge
        } else {
            (edge.1, edge.0)
        };
        self.edges.iter().any(|&e| {
            let ek = if e.0 < e.1 { e } else { (e.1, e.0) };
            ek == key
        })
    }

    pub fn contains_vertex_index(&self, vi: VertexIndex) -> bool {
        self.contains_vertex(vi.0)
    }

    pub fn contains_face_index(&self, fi: FaceIndex) -> bool {
        self.contains_face(fi.0)
    }

    pub fn contains_edge_key(&self, ek: EdgeKey) -> bool {
        self.contains_edge(ek.to_pair())
    }

    pub fn select_vertex(&mut self, vi: impl Into<VertexIndex>) {
        let v = vi.into().0;
        if !self.verts.contains(&v) {
            self.verts.push(v);
        }
    }

    pub fn deselect_vertex(&mut self, vi: impl Into<VertexIndex>) {
        let v = vi.into().0;
        self.verts.retain(|&x| x != v);
    }

    pub fn select_face(&mut self, fi: impl Into<FaceIndex>) {
        let f = fi.into().0;
        if !self.faces.contains(&f) {
            self.faces.push(f);
        }
    }

    pub fn deselect_face(&mut self, fi: impl Into<FaceIndex>) {
        let f = fi.into().0;
        self.faces.retain(|&x| x != f);
    }

    pub fn select_edge(&mut self, ek: impl Into<EdgeKey>) {
        let pair = ek.into().to_pair();
        if !self.contains_edge(pair) {
            self.edges.push(pair);
        }
    }

    pub fn deselect_edge(&mut self, ek: impl Into<EdgeKey>) {
        let pair = ek.into().to_pair();
        let key = if pair.0 < pair.1 {
            pair
        } else {
            (pair.1, pair.0)
        };
        self.edges.retain(|&e| {
            let ek = if e.0 < e.1 { e } else { (e.1, e.0) };
            ek != key
        });
    }

    /// Aplica a seleção atual diretamente sobre a malha.
    pub fn apply_to_mesh(&self, mesh: &mut petunia_mesh::Mesh) {
        for v in &mut mesh.verts {
            v.selected = false;
        }
        for f in &mut mesh.faces {
            f.selected = false;
        }
        mesh.selected_edges.clear();

        for &v in &self.verts {
            if let Some(vert) = mesh.verts.get_mut(v as usize) {
                vert.selected = true;
            }
        }
        for &f in &self.faces {
            if let Some(face) = mesh.faces.get_mut(f) {
                face.selected = true;
            }
        }
        for &e in &self.edges {
            mesh.selected_edges.insert((e.0.min(e.1), e.0.max(e.1)));
        }
    }

    /// Atualiza a seleção de componentes usando um `ElementRemap` topológico
    /// (website/docs/01-core/camera-and-selection.md).
    pub fn remap(&mut self, remap: &petunia_mesh::ElementRemap) {
        let mut new_verts = Vec::with_capacity(self.verts.len());
        for &v in &self.verts {
            if let Some(nv) = remap.map_vertex(v) {
                new_verts.push(nv);
            }
        }
        self.verts = new_verts;

        let mut new_faces = Vec::with_capacity(self.faces.len());
        for &f in &self.faces {
            if let Some(nf) = remap.map_face(f) {
                new_faces.push(nf);
            }
        }
        self.faces = new_faces;

        let mut new_edges = Vec::with_capacity(self.edges.len());
        for &(a, b) in &self.edges {
            if let (Some(na), Some(nb)) = (remap.map_vertex(a), remap.map_vertex(b))
                && na != nb
            {
                new_edges.push((na.min(nb), na.max(nb)));
            }
        }
        self.edges = new_edges;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_domain_from_select_mode_equivalence() {
        assert_eq!(
            SelectionDomain::from(SelectMode::Vertex),
            SelectionDomain::Vertex
        );
        assert_eq!(
            SelectionDomain::from(SelectMode::Edge),
            SelectionDomain::Edge
        );
        assert_eq!(
            SelectionDomain::from(SelectMode::Face),
            SelectionDomain::Face
        );
    }

    #[test]
    fn selection_membership_and_topological_remap() {
        let mut sel = Selection {
            asset: None,
            assets: Vec::new(),
            verts: vec![0, 1, 3],
            faces: vec![0, 2],
            edges: vec![(0, 1), (1, 3)],
        };

        assert!(sel.contains_vertex(1));
        assert!(!sel.contains_vertex(2));
        assert!(sel.contains_face(0));
        assert!(!sel.contains_face(1));
        assert!(sel.contains_edge((1, 0))); // edge key normalizada

        // Simula remap onde 0 -> 0, 1 -> fundido com 0 (0), 3 -> 1
        // Face 0 sobrevive como 0, face 2 colapsa (None)
        let remap = petunia_mesh::ElementRemap {
            vertices: vec![(0, Some(0)), (1, Some(0)), (3, Some(1))],
            faces: vec![(0, Some(0)), (2, None)],
        };

        sel.remap(&remap);

        assert_eq!(sel.verts, vec![0, 0, 1]);
        assert_eq!(sel.faces, vec![0]);
        // Edge (0, 1) colapsou para (0, 0) e foi descartada; Edge (1, 3) virou (0, 1)
        assert_eq!(sel.edges, vec![(0, 1)]);
    }

    #[test]
    fn selection_typed_ids_and_apply_to_mesh() {
        let mut sel = Selection::default();
        sel.select_vertex(VertexIndex(2));
        sel.select_face(FaceIndex(1));
        sel.select_edge(EdgeKey::new(1, 3));

        assert!(sel.contains_vertex_index(VertexIndex(2)));
        assert!(!sel.contains_vertex_index(VertexIndex(0)));
        assert!(sel.contains_face_index(FaceIndex(1)));
        assert!(!sel.contains_face_index(FaceIndex(0)));
        assert!(sel.contains_edge_key(EdgeKey::new(3, 1)));

        let mut mesh = petunia_mesh::Mesh {
            verts: vec![
                petunia_mesh::Vertex::new(0.0, 0.0, 0.0),
                petunia_mesh::Vertex::new(1.0, 0.0, 0.0),
                petunia_mesh::Vertex::new(1.0, 1.0, 0.0),
                petunia_mesh::Vertex::new(0.0, 1.0, 0.0),
            ],
            faces: vec![
                petunia_mesh::Face::new(vec![0, 1, 2]),
                petunia_mesh::Face::new(vec![0, 2, 3]),
            ],
            ..Default::default()
        };

        sel.apply_to_mesh(&mut mesh);
        assert!(!mesh.verts[0].selected);
        assert!(!mesh.verts[1].selected);
        assert!(mesh.verts[2].selected);
        assert!(!mesh.verts[3].selected);
        assert!(!mesh.faces[0].selected);
        assert!(mesh.faces[1].selected);
        assert!(mesh.selected_edges.contains(&(1, 3)));

        sel.deselect_vertex(VertexIndex(2));
        sel.deselect_face(FaceIndex(1));
        sel.deselect_edge(EdgeKey::new(1, 3));
        assert!(!sel.contains_vertex_index(VertexIndex(2)));
        assert!(!sel.contains_face_index(FaceIndex(1)));
        assert!(!sel.contains_edge_key(EdgeKey::new(1, 3)));
    }
}
