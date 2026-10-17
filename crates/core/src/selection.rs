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

/// Referência tipada e estável a uma entidade selecionada (SEL-002, SEL-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SelectionRef {
    Object(Uuid),
    Vertex { object: Uuid, vertex: u32 },
    Edge { object: Uuid, edge: (u32, u32) },
    Face { object: Uuid, face: usize },
    Shape { object: Uuid, shape: Uuid },
}

impl SelectionRef {
    pub fn object(id: Uuid) -> Self {
        Self::Object(id)
    }

    pub fn vertex(object: Uuid, vertex: u32) -> Self {
        Self::Vertex { object, vertex }
    }

    pub fn edge(object: Uuid, a: u32, b: u32) -> Self {
        Self::Edge {
            object,
            edge: (a.min(b), a.max(b)),
        }
    }

    pub fn face(object: Uuid, face: usize) -> Self {
        Self::Face { object, face }
    }

    pub fn shape(object: Uuid, shape: Uuid) -> Self {
        Self::Shape { object, shape }
    }

    pub fn object_id(&self) -> Uuid {
        match *self {
            Self::Object(id) => id,
            Self::Vertex { object, .. } => object,
            Self::Edge { object, .. } => object,
            Self::Face { object, .. } => object,
            Self::Shape { object, .. } => object,
        }
    }

    pub fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }

    pub fn is_component(&self) -> bool {
        matches!(
            self,
            Self::Vertex { .. } | Self::Edge { .. } | Self::Face { .. }
        )
    }

    pub fn is_shape(&self) -> bool {
        matches!(self, Self::Shape { .. })
    }

    pub fn domain(&self) -> SelectionDomain {
        match self {
            Self::Object(_) => SelectionDomain::Object,
            Self::Vertex { .. } => SelectionDomain::Vertex,
            Self::Edge { .. } => SelectionDomain::Edge,
            Self::Face { .. } => SelectionDomain::Face,
            Self::Shape { .. } => SelectionDomain::Object,
        }
    }
}

/// Filtros contextuais de seleção (SEL-002).
///
/// Permitem habilitar ou desabilitar categorias de entidades para picking
/// e seleção sem obrigar a troca manual entre modos globais.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionFilters {
    pub objects: bool,
    pub vertices: bool,
    pub edges: bool,
    pub faces: bool,
    pub shapes: bool,
}

impl Default for SelectionFilters {
    fn default() -> Self {
        Self::from_domain(SelectionDomain::Object)
    }
}

impl SelectionFilters {
    pub const fn all() -> Self {
        Self {
            objects: true,
            vertices: true,
            edges: true,
            faces: true,
            shapes: true,
        }
    }

    pub const fn objects_only() -> Self {
        Self {
            objects: true,
            vertices: false,
            edges: false,
            faces: false,
            shapes: false,
        }
    }

    pub const fn components_only() -> Self {
        Self {
            objects: false,
            vertices: true,
            edges: true,
            faces: true,
            shapes: false,
        }
    }

    pub const fn from_domain(domain: SelectionDomain) -> Self {
        match domain {
            SelectionDomain::Object => Self::objects_only(),
            SelectionDomain::Vertex => Self {
                objects: false,
                vertices: true,
                edges: false,
                faces: false,
                shapes: false,
            },
            SelectionDomain::Edge => Self {
                objects: false,
                vertices: false,
                edges: true,
                faces: false,
                shapes: false,
            },
            SelectionDomain::Face => Self {
                objects: false,
                vertices: false,
                edges: false,
                faces: true,
                shapes: false,
            },
        }
    }

    pub fn allows(&self, item: &SelectionRef) -> bool {
        match item {
            SelectionRef::Object(_) => self.objects,
            SelectionRef::Vertex { .. } => self.vertices,
            SelectionRef::Edge { .. } => self.edges,
            SelectionRef::Face { .. } => self.faces,
            SelectionRef::Shape { .. } => self.shapes,
        }
    }

    pub fn allows_domain(&self, domain: SelectionDomain) -> bool {
        match domain {
            SelectionDomain::Object => self.objects,
            SelectionDomain::Vertex => self.vertices,
            SelectionDomain::Edge => self.edges,
            SelectionDomain::Face => self.faces,
        }
    }
}

/// Elemento ativo da seleção (SEL-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActiveElement {
    pub item: SelectionRef,
}

impl ActiveElement {
    pub fn new(item: SelectionRef) -> Self {
        Self { item }
    }
}

/// Conjunto resolvido de alvos para transformação desambiguada (SEL-006).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolvedTransformTargets {
    /// IDs de objetos que sofrem transformação de nó/TRS.
    /// Exclui nós filhos quando o pai também está selecionado.
    pub objects: Vec<Uuid>,
    /// Vértices por malha (UUID do objeto -> lista ordenada e deduplicada de índices de vértices).
    /// Objetos que já estão em `objects` NÃO têm seus vértices duplicados aqui (dominância de objeto).
    pub mesh_vertices: std::collections::BTreeMap<Uuid, Vec<u32>>,
    /// Shapes/perfis paramétricos a transformar (object_id, shape_id).
    pub shapes: Vec<(Uuid, Uuid)>,
    /// Alvos pertencentes a objetos com lock ativo que foram omitidos da transformação.
    pub locked_targets: Vec<SelectionRef>,
    /// Elemento ativo resolvido.
    pub active_element: Option<SelectionRef>,
}

impl ResolvedTransformTargets {
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty() && self.mesh_vertices.is_empty() && self.shapes.is_empty()
    }

    /// Retorna verdadeiro se o objeto especificado sofre transformação completa de nó.
    pub fn transforms_object(&self, id: Uuid) -> bool {
        self.objects.contains(&id)
    }

    /// Retorna os vértices afetados para um objeto específico (se houver).
    pub fn vertices_for_object(&self, id: Uuid) -> Option<&[u32]> {
        self.mesh_vertices.get(&id).map(|v| v.as_slice())
    }
}

/// Motor de resolução e desambiguação de alvos de seleção (SEL-006).
pub struct SelectionResolver;

impl SelectionResolver {
    /// Filtra objetos raiz na hierarquia para evitar transformar pai e filho duas vezes.
    pub fn filter_hierarchy_roots(
        objects: &[Uuid],
        parent_lookup: impl Fn(Uuid) -> Option<Uuid>,
    ) -> Vec<Uuid> {
        let mut roots = Vec::new();
        for &obj in objects {
            let mut curr = obj;
            let mut has_selected_ancestor = false;
            let mut depth = 0;
            while let Some(parent) = parent_lookup(curr) {
                if depth >= 64 {
                    break;
                }
                depth += 1;
                if parent == obj {
                    break;
                }
                if objects.contains(&parent) {
                    has_selected_ancestor = true;
                    break;
                }
                curr = parent;
            }
            if !has_selected_ancestor && !roots.contains(&obj) {
                roots.push(obj);
            }
        }
        if roots.is_empty() && !objects.is_empty() {
            roots.push(objects[0]);
        }
        roots
    }

    /// Resolve alvos de transformação a partir de itens selecionados,
    /// com resolução de dominância de objeto, expansão deduplicada de faces/arestas,
    /// prevenção de dupla transformação na hierarquia e respeito a bloqueios.
    pub fn resolve<'a>(
        items: impl IntoIterator<Item = &'a SelectionRef>,
        mesh_lookup: impl Fn(Uuid) -> Option<&'a petunia_mesh::Mesh>,
        locked_lookup: impl Fn(Uuid) -> bool,
        parent_lookup: impl Fn(Uuid) -> Option<Uuid>,
    ) -> ResolvedTransformTargets {
        let mut raw_objects = Vec::new();
        let mut component_items: Vec<&SelectionRef> = Vec::new();
        let mut shapes = Vec::new();
        let mut locked_targets = Vec::new();
        let mut last_valid_item = None;

        for item in items {
            let obj_id = item.object_id();
            if locked_lookup(obj_id) {
                if !locked_targets.contains(item) {
                    locked_targets.push(*item);
                }
                continue;
            }
            last_valid_item = Some(*item);
            match item {
                SelectionRef::Object(id) => {
                    if !raw_objects.contains(id) {
                        raw_objects.push(*id);
                    }
                }
                SelectionRef::Shape { object, shape } => {
                    let pair = (*object, *shape);
                    if !shapes.contains(&pair) {
                        shapes.push(pair);
                    }
                }
                SelectionRef::Vertex { .. }
                | SelectionRef::Edge { .. }
                | SelectionRef::Face { .. } => {
                    component_items.push(item);
                }
            }
        }

        let resolved_objects = Self::filter_hierarchy_roots(&raw_objects, parent_lookup);
        let mut mesh_vertices = std::collections::BTreeMap::new();

        for item in component_items {
            let obj_id = item.object_id();
            // Dominância de objeto: Se o objeto inteiro já está selecionado para transformação de nó,
            // seus componentes NÃO sofrem uma segunda transformação no nível de vértice.
            if raw_objects.contains(&obj_id) {
                continue;
            }

            let entry = mesh_vertices
                .entry(obj_id)
                .or_insert_with(std::collections::BTreeSet::new);

            match item {
                SelectionRef::Vertex { vertex, .. } => {
                    entry.insert(*vertex);
                }
                SelectionRef::Edge { edge, .. } => {
                    entry.insert(edge.0);
                    entry.insert(edge.1);
                }
                SelectionRef::Face { face, .. } => {
                    if let Some(f) = mesh_lookup(obj_id).and_then(|m| m.faces.get(*face)) {
                        for &v in &f.verts {
                            entry.insert(v);
                        }
                    }
                }
                _ => {}
            }
        }

        let sorted_mesh_vertices = mesh_vertices
            .into_iter()
            .map(|(id, set)| (id, set.into_iter().collect::<Vec<u32>>()))
            .collect();

        ResolvedTransformTargets {
            objects: resolved_objects,
            mesh_vertices: sorted_mesh_vertices,
            shapes,
            locked_targets,
            active_element: last_valid_item,
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
    /// Itens contextuais selecionados com ordem de seleção preservada (SEL-002, SEL-006).
    pub items: Vec<SelectionRef>,
    /// Elemento ativo tipado (SEL-006).
    pub active_element: Option<ActiveElement>,
    /// Filtros contextuais ativos (SEL-002).
    pub filters: SelectionFilters,
}

/// Estado de seleção da sessão do editor (website/docs/02-application/state-separation.md).
pub type SelectionState = Selection;

impl Selection {
    pub fn clear(&mut self) {
        self.verts.clear();
        self.faces.clear();
        self.edges.clear();
        self.items.clear();
        self.active_element = None;
    }
    pub fn is_empty(&self) -> bool {
        self.verts.is_empty() && self.faces.is_empty() && self.edges.is_empty()
    }
    pub fn is_totally_empty(&self) -> bool {
        self.is_empty() && self.assets.is_empty() && self.items.is_empty()
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

    /// Sincroniza campos legados a partir da lista ordenada de `items`.
    pub fn sync_legacy_fields(&mut self) {
        let mut assets = Vec::new();
        let mut verts = Vec::new();
        let mut faces = Vec::new();
        let mut edges = Vec::new();
        let mut active_asset = None;

        for item in &self.items {
            match item {
                SelectionRef::Object(id) => {
                    if !assets.contains(id) {
                        assets.push(*id);
                    }
                    active_asset = Some(*id);
                }
                SelectionRef::Vertex { object, vertex } => {
                    active_asset = Some(*object);
                    if !verts.contains(vertex) {
                        verts.push(*vertex);
                    }
                }
                SelectionRef::Edge { object, edge } => {
                    active_asset = Some(*object);
                    let k = if edge.0 < edge.1 {
                        *edge
                    } else {
                        (edge.1, edge.0)
                    };
                    if !edges.contains(&k) {
                        edges.push(k);
                    }
                }
                SelectionRef::Face { object, face } => {
                    active_asset = Some(*object);
                    if !faces.contains(face) {
                        faces.push(*face);
                    }
                }
                SelectionRef::Shape { object, .. } => {
                    active_asset = Some(*object);
                }
            }
        }

        self.assets = assets;
        if self.asset.is_none()
            || (active_asset.is_some()
                && !self
                    .assets
                    .contains(&self.asset.unwrap_or(uuid::Uuid::nil())))
        {
            self.asset = active_asset;
        }
        self.verts = verts;
        self.faces = faces;
        self.edges = edges;
        self.active_element = self.items.last().copied().map(ActiveElement::new);
    }

    /// Reconstrói a lista de `items` a partir dos campos legados (`assets`, `verts`, `faces`, `edges`).
    pub fn sync_from_legacy_fields(&mut self) {
        let mut items = Vec::new();
        for &id in &self.assets {
            items.push(SelectionRef::Object(id));
        }
        if let Some(active) = self.asset {
            for &v in &self.verts {
                items.push(SelectionRef::vertex(active, v));
            }
            for &(a, b) in &self.edges {
                items.push(SelectionRef::edge(active, a, b));
            }
            for &f in &self.faces {
                items.push(SelectionRef::face(active, f));
            }
        }
        self.items = items;
        self.active_element = self.items.last().copied().map(ActiveElement::new);
    }

    /// Seleciona um item contextual respeitando os filtros ativos.
    pub fn select_item(&mut self, item: SelectionRef) -> bool {
        if !self.filters.allows(&item) {
            return false;
        }
        if !self.items.contains(&item) {
            self.items.push(item);
            self.sync_legacy_fields();
            true
        } else {
            false
        }
    }

    /// Remove um item da seleção.
    pub fn deselect_item(&mut self, item: &SelectionRef) -> bool {
        let before_len = self.items.len();
        self.items.retain(|i| i != item);
        if self.items.len() != before_len {
            self.sync_legacy_fields();
            true
        } else {
            false
        }
    }

    /// Alterna a presença de um item na seleção.
    pub fn toggle_item(&mut self, item: SelectionRef) -> bool {
        if self.items.contains(&item) {
            self.deselect_item(&item)
        } else {
            self.select_item(item)
        }
    }

    /// Resolve alvos de transformação a partir dos itens selecionados.
    pub fn resolve_transform_targets<'a>(
        &'a self,
        mesh_lookup: impl Fn(Uuid) -> Option<&'a petunia_mesh::Mesh>,
        locked_lookup: impl Fn(Uuid) -> bool,
        parent_lookup: impl Fn(Uuid) -> Option<Uuid>,
    ) -> ResolvedTransformTargets {
        SelectionResolver::resolve(&self.items, mesh_lookup, locked_lookup, parent_lookup)
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
            ..Default::default()
        };

        assert!(sel.contains_vertex(1));
        assert!(!sel.contains_vertex(2));
        assert!(sel.contains_face(0));
        assert!(!sel.contains_face(1));
        assert!(sel.contains_edge((1, 0))); // edge key normalizada

        // Simula remap onde 0 -> 0, 1 -> fundido com 0 (0), 3 -> 1
        // Face 0 sobrevive como 0, face 2 colapsa (None)
        let remap = petunia_mesh::ElementRemap::new(
            vec![(0, Some(0)), (1, Some(0)), (3, Some(1))],
            vec![(0, Some(0)), (2, None)],
        );

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

    #[test]
    fn selection_ref_identity_and_domains() {
        let obj = Uuid::new_v4();
        let shape = Uuid::new_v4();

        let r_obj = SelectionRef::object(obj);
        let r_v = SelectionRef::vertex(obj, 5);
        let r_e = SelectionRef::edge(obj, 10, 4); // normaliza para (4, 10)
        let r_f = SelectionRef::face(obj, 2);
        let r_s = SelectionRef::shape(obj, shape);

        assert_eq!(r_obj.object_id(), obj);
        assert!(r_obj.is_object());
        assert!(!r_obj.is_component());
        assert_eq!(r_obj.domain(), SelectionDomain::Object);

        assert_eq!(r_v.object_id(), obj);
        assert!(r_v.is_component());
        assert_eq!(r_v.domain(), SelectionDomain::Vertex);

        assert_eq!(r_e.object_id(), obj);
        assert!(r_e.is_component());
        assert_eq!(r_e.domain(), SelectionDomain::Edge);
        if let SelectionRef::Edge { edge, .. } = r_e {
            assert_eq!(edge, (4, 10));
        } else {
            panic!("Expected edge");
        }

        assert_eq!(r_f.object_id(), obj);
        assert!(r_f.is_component());
        assert_eq!(r_f.domain(), SelectionDomain::Face);

        assert_eq!(r_s.object_id(), obj);
        assert!(r_s.is_shape());
    }

    #[test]
    fn selection_filters_allow_logic() {
        let obj = Uuid::new_v4();
        let r_obj = SelectionRef::object(obj);
        let r_v = SelectionRef::vertex(obj, 1);
        let r_e = SelectionRef::edge(obj, 1, 2);
        let r_f = SelectionRef::face(obj, 0);

        let all = SelectionFilters::all();
        assert!(all.allows(&r_obj));
        assert!(all.allows(&r_v));
        assert!(all.allows(&r_e));
        assert!(all.allows(&r_f));

        let obj_only = SelectionFilters::objects_only();
        assert!(obj_only.allows(&r_obj));
        assert!(!obj_only.allows(&r_v));
        assert!(!obj_only.allows(&r_e));
        assert!(!obj_only.allows(&r_f));

        let comp_only = SelectionFilters::components_only();
        assert!(!comp_only.allows(&r_obj));
        assert!(comp_only.allows(&r_v));
        assert!(comp_only.allows(&r_e));
        assert!(comp_only.allows(&r_f));

        let edge_only = SelectionFilters::from_domain(SelectionDomain::Edge);
        assert!(!edge_only.allows(&r_obj));
        assert!(!edge_only.allows(&r_v));
        assert!(edge_only.allows(&r_e));
        assert!(!edge_only.allows(&r_f));
    }

    #[test]
    fn selection_resolver_shared_vertex_deduplication() {
        let obj = Uuid::new_v4();
        let mesh = petunia_mesh::Mesh {
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

        // Seleciona face 0 (vértices 0, 1, 2) e face 1 (vértices 0, 2, 3) e aresta (1, 2)
        let items = vec![
            SelectionRef::face(obj, 0),
            SelectionRef::face(obj, 1),
            SelectionRef::edge(obj, 1, 2),
            SelectionRef::vertex(obj, 2),
        ];

        let targets = SelectionResolver::resolve(
            &items,
            |id| if id == obj { Some(&mesh) } else { None },
            |_| false,
            |_| None,
        );

        assert!(targets.objects.is_empty());
        let verts = targets
            .vertices_for_object(obj)
            .expect("Mesh vertices resolved");
        // Vértices 0, 1, 2, 3 devem aparecer exatamente UMA vez cada, ordenados
        assert_eq!(verts, &[0, 1, 2, 3]);
    }

    #[test]
    fn selection_resolver_object_dominance_over_components() {
        let obj = Uuid::new_v4();
        let mesh = petunia_mesh::Mesh {
            verts: vec![
                petunia_mesh::Vertex::new(0.0, 0.0, 0.0),
                petunia_mesh::Vertex::new(1.0, 0.0, 0.0),
            ],
            faces: vec![],
            ..Default::default()
        };

        // Objeto E componentes do mesmo objeto selecionados
        let items = vec![
            SelectionRef::object(obj),
            SelectionRef::vertex(obj, 0),
            SelectionRef::vertex(obj, 1),
        ];

        let targets = SelectionResolver::resolve(
            &items,
            |id| if id == obj { Some(&mesh) } else { None },
            |_| false,
            |_| None,
        );

        // Objeto inteiro domina na transformação; vértices não entram na lista de transform
        assert_eq!(targets.objects, vec![obj]);
        assert!(!targets.mesh_vertices.contains_key(&obj));
    }

    #[test]
    fn selection_resolver_parent_child_hierarchy_no_double_transform() {
        let parent = Uuid::new_v4();
        let child = Uuid::new_v4();
        let grandchild = Uuid::new_v4();
        let unrelated = Uuid::new_v4();

        let items = vec![
            SelectionRef::object(parent),
            SelectionRef::object(child),
            SelectionRef::object(grandchild),
            SelectionRef::object(unrelated),
        ];

        let parent_lookup = |id| {
            if id == child {
                Some(parent)
            } else if id == grandchild {
                Some(child)
            } else {
                None
            }
        };

        let targets = SelectionResolver::resolve(&items, |_| None, |_| false, parent_lookup);

        // Filho e neto herdam o transform do pai no grafo; apenas o pai e o não relacionado entram na raiz
        assert_eq!(targets.objects, vec![parent, unrelated]);
    }

    #[test]
    fn selection_resolver_respects_locked_objects() {
        let locked_obj = Uuid::new_v4();
        let open_obj = Uuid::new_v4();

        let items = vec![
            SelectionRef::object(locked_obj),
            SelectionRef::vertex(locked_obj, 1),
            SelectionRef::object(open_obj),
        ];

        let targets = SelectionResolver::resolve(&items, |_| None, |id| id == locked_obj, |_| None);

        assert_eq!(targets.objects, vec![open_obj]);
        assert_eq!(targets.locked_targets.len(), 2);
        assert!(
            targets
                .locked_targets
                .contains(&SelectionRef::object(locked_obj))
        );
        assert!(
            targets
                .locked_targets
                .contains(&SelectionRef::vertex(locked_obj, 1))
        );
    }

    #[test]
    fn selection_item_helpers_and_active_element() {
        let mut sel = Selection {
            filters: SelectionFilters::all(),
            ..Default::default()
        };
        let obj = Uuid::new_v4();
        let v1 = SelectionRef::vertex(obj, 1);
        let v2 = SelectionRef::vertex(obj, 2);

        assert!(sel.select_item(v1));
        assert_eq!(sel.items, vec![v1]);
        assert_eq!(sel.active_element.map(|a| a.item), Some(v1));
        assert_eq!(sel.verts, vec![1]);

        assert!(sel.select_item(v2));
        assert_eq!(sel.items, vec![v1, v2]);
        assert_eq!(sel.active_element.map(|a| a.item), Some(v2));
        assert_eq!(sel.verts, vec![1, 2]);

        assert!(sel.deselect_item(&v2));
        assert_eq!(sel.items, vec![v1]);
        assert_eq!(sel.active_element.map(|a| a.item), Some(v1));

        sel.clear();
        assert!(sel.items.is_empty());
        assert!(sel.active_element.is_none());
        assert!(sel.verts.is_empty());
    }
}
