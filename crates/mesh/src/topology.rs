//! Explicit topology-result contract for modeling operations.
//!
//! Operations return this so selection, UV, paint attachments and renderer
//! revisions can remap without guessing.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// Proveniência ou histórico de origem de um elemento após mutações topológicas (G04).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElementProvenance {
    /// Elemento preservado de antes da operação (id prévio mantido ou reindexado).
    Preserved(u32),
    /// Elemento resultante da fusão de dois ou mais elementos (ex.: weld, dissolve).
    Merged(Vec<u32>),
    /// Elemento derivado da divisão de um elemento fonte (ex.: knife, bisect, loopcut).
    SplitFrom(u32),
    /// Elemento novo gerado do zero pela operação (ex.: novas faces de extrusão, tampas).
    Created,
}

/// Old → new identity after a merge/split. Missing entries were deleted.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementRemap {
    pub vertices: Vec<(u32, Option<u32>)>,
    pub faces: Vec<(usize, Option<usize>)>,
    #[serde(default)]
    pub vertex_provenance: HashMap<u32, ElementProvenance>,
    #[serde(default)]
    pub face_provenance: HashMap<usize, ElementProvenance>,
}

impl ElementRemap {
    pub fn new(vertices: Vec<(u32, Option<u32>)>, faces: Vec<(usize, Option<usize>)>) -> Self {
        Self {
            vertices,
            faces,
            vertex_provenance: HashMap::new(),
            face_provenance: HashMap::new(),
        }
    }

    pub fn identity() -> Self {
        Self::default()
    }

    pub fn is_identity(&self) -> bool {
        self.vertices.is_empty()
            && self.faces.is_empty()
            && self.vertex_provenance.is_empty()
            && self.face_provenance.is_empty()
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

    /// Mapeia uma aresta `(a, b)` para sua nova identidade canonicalizada `(min, max)`.
    /// Retorna `None` se algum dos vértices foi excluído ou se a aresta colapsou (`na == nb`).
    pub fn map_edge(&self, (a, b): (u32, u32)) -> Option<(u32, u32)> {
        let na = self.map_vertex(a)?;
        let nb = self.map_vertex(b)?;
        if na == nb {
            None
        } else {
            Some((na.min(nb), na.max(nb)))
        }
    }

    /// Mapeia um canto de face `(face, corner_idx)`.
    /// Retorna `None` se a face foi excluída ou se `corner_idx` exceder os vértices da nova face.
    pub fn map_corner(
        &self,
        old_face: usize,
        corner: usize,
        new_face_vertex_count: impl Fn(usize) -> Option<usize>,
    ) -> Option<(usize, usize)> {
        let new_face = self.map_face(old_face)?;
        let count = new_face_vertex_count(new_face)?;
        if corner < count {
            Some((new_face, corner))
        } else {
            None
        }
    }

    pub fn vertex_origin(&self, new_vert: u32) -> Option<&ElementProvenance> {
        self.vertex_provenance.get(&new_vert)
    }

    pub fn face_origin(&self, new_face: usize) -> Option<&ElementProvenance> {
        self.face_provenance.get(&new_face)
    }

    pub fn record_merged_vertices(&mut self, new_id: u32, old_ids: Vec<u32>) {
        for &old in &old_ids {
            self.vertices.push((old, Some(new_id)));
        }
        self.vertex_provenance
            .insert(new_id, ElementProvenance::Merged(old_ids));
    }

    pub fn record_split_vertex(&mut self, new_id: u32, source_id: u32) {
        self.vertex_provenance
            .insert(new_id, ElementProvenance::SplitFrom(source_id));
    }

    pub fn record_created_vertex(&mut self, new_id: u32) {
        self.vertex_provenance
            .insert(new_id, ElementProvenance::Created);
    }

    pub fn record_merged_faces(&mut self, new_id: usize, old_ids: Vec<usize>) {
        for &old in &old_ids {
            self.faces.push((old, Some(new_id)));
        }
        self.face_provenance.insert(
            new_id,
            ElementProvenance::Merged(old_ids.into_iter().map(|f| f as u32).collect()),
        );
    }

    pub fn record_split_face(&mut self, new_id: usize, source_id: usize) {
        self.face_provenance
            .insert(new_id, ElementProvenance::SplitFrom(source_id as u32));
    }

    pub fn record_created_face(&mut self, new_id: usize) {
        self.face_provenance
            .insert(new_id, ElementProvenance::Created);
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
    #[serde(default)]
    pub sharp_edges_touched: Vec<(u32, u32)>,
    #[serde(default)]
    pub material_slots_touched: Vec<usize>,
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
            sharp_edges_touched: Vec::new(),
            material_slots_touched: Vec::new(),
        }
    }

    pub fn touches_uv(&self) -> bool {
        self.invalidated.uv || !self.uv_faces_touched.is_empty()
    }

    pub fn touches_sharp(&self) -> bool {
        !self.sharp_edges_touched.is_empty()
    }

    pub fn touches_material(&self) -> bool {
        !self.material_slots_touched.is_empty()
    }

    pub fn affects_selection(&self) -> bool {
        self.invalidated.selection
            || self.verts_added > 0
            || self.verts_removed > 0
            || self.faces_added > 0
            || self.faces_removed > 0
            || !self.remap.is_identity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_remap_identity_and_fallbacks() {
        let remap = ElementRemap::identity();
        assert!(remap.is_identity());
        assert_eq!(remap.map_vertex(5), Some(5));
        assert_eq!(remap.map_face(3), Some(3));
        assert_eq!(remap.map_edge((2, 7)), Some((2, 7)));
        assert_eq!(remap.map_edge((7, 2)), Some((2, 7)));
    }

    #[test]
    fn element_remap_edge_mapping_and_collapse() {
        let mut remap = ElementRemap::default();
        // Vértices 0 e 1 soldados no novo vértice 0; vértice 3 deletado
        remap.vertices.push((0, Some(0)));
        remap.vertices.push((1, Some(0)));
        remap.vertices.push((3, None));

        // Aresta (0, 1) colapsou
        assert_eq!(remap.map_edge((0, 1)), None);
        // Aresta com vértice deletado colapsa para None
        assert_eq!(remap.map_edge((2, 3)), None);
        // Aresta (1, 4) vira (0, 4)
        assert_eq!(remap.map_edge((1, 4)), Some((0, 4)));
    }

    #[test]
    fn element_remap_corner_mapping() {
        let mut remap = ElementRemap::default();
        remap.faces.push((0, Some(2)));
        remap.faces.push((1, None)); // deletada

        // Face 2 tem 4 vértices (quad)
        let count_fn = |f: usize| if f == 2 { Some(4) } else { None };

        assert_eq!(remap.map_corner(0, 2, count_fn), Some((2, 2)));
        // Corner além do limite da face é rejeitado
        assert_eq!(remap.map_corner(0, 5, count_fn), None);
        // Face deletada é rejeitada
        assert_eq!(remap.map_corner(1, 0, count_fn), None);
    }

    #[test]
    fn element_provenance_tracking() {
        let mut remap = ElementRemap::default();
        remap.record_merged_vertices(10, vec![1, 2, 3]);
        remap.record_split_vertex(11, 4);
        remap.record_created_vertex(12);

        assert_eq!(
            remap.vertex_origin(10),
            Some(&ElementProvenance::Merged(vec![1, 2, 3]))
        );
        assert_eq!(
            remap.vertex_origin(11),
            Some(&ElementProvenance::SplitFrom(4))
        );
        assert_eq!(remap.vertex_origin(12), Some(&ElementProvenance::Created));
        assert_eq!(remap.vertex_origin(99), None);
    }

    #[test]
    fn topology_result_domain_flags_and_query_helpers() {
        let mut result = TopologyResult::from_counts(4, 1, 8, 5);
        assert!(result.affects_selection());
        assert!(result.touches_uv());
        assert!(!result.touches_sharp());
        assert!(!result.touches_material());

        result.sharp_edges_touched.push((0, 1));
        result.material_slots_touched.push(0);
        assert!(result.touches_sharp());
        assert!(result.touches_material());
    }

    #[test]
    fn serde_roundtrip_preserves_topology_result_and_provenance() {
        let mut remap = ElementRemap::default();
        remap.record_merged_vertices(0, vec![10, 11]);
        let mut tr = TopologyResult::from_counts(10, 2, 9, 2);
        tr.remap = remap;
        tr.sharp_edges_touched = vec![(0, 2)];
        tr.material_slots_touched = vec![1];

        let json = serde_json::to_string(&tr).expect("serialização json");
        let decoded: TopologyResult = serde_json::from_str(&json).expect("desserialização json");
        assert_eq!(tr, decoded);
    }
}
