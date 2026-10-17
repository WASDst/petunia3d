//! Gates e testes de conformidade para Selection Única e Domínios Tipados (A05).
//!
//! Website/docs/01-core/camera-and-selection.md:
//! 1. SelectionDomain autoridade única para o domínio de interação (Object, Vertex, Edge, Face).
//! 2. SelectMode mantido como adaptador compatível com transição segura.
//! 3. IDs tipados e seguros (VertexIndex, FaceIndex, EdgeKey).
//! 4. Remapeamento topológico resiliente (Selection::remap(&ElementRemap)).
//! 5. Prioridade e autoridade única da Application sobre a seleção (não Mesh).
//! 6. Preservação de integridade em Undo/Redo e poda graciosa de referências obsoletas (Never Crash).

use petunia_core::{
    AddPrimitiveCmd, AppState, EdgeKey, FaceIndex, SelectAllCmd, SelectMode, Selection,
    SelectionDomain, SelectionFilters, SelectionRef, VertexIndex,
};
use petunia_mesh::ElementRemap;

// ----------------------------------------------------------------------------
// Gate 1: SelectionDomain como autoridade única e equivalência com SelectMode
// ----------------------------------------------------------------------------
#[test]
fn test_gate1_selection_domain_authority_and_select_mode_equivalence() {
    let mut state = AppState::new("en");

    // Autoridade padrão é Object
    assert_eq!(state.selection_domain(), SelectionDomain::Object);
    assert!(!state.selection_domain().is_component());

    // Transição para Vertex
    state.set_selection_domain(SelectionDomain::Vertex);
    assert_eq!(state.selection_domain(), SelectionDomain::Vertex);
    assert!(state.selection_domain().is_component());
    assert_eq!(
        state.selection_domain().as_select_mode(),
        Some(SelectMode::Vertex)
    );
    assert_eq!(
        SelectionDomain::from(SelectMode::Vertex),
        SelectionDomain::Vertex
    );

    // Transição para Edge
    state.set_selection_domain(SelectionDomain::Edge);
    assert_eq!(state.selection_domain(), SelectionDomain::Edge);
    assert_eq!(
        state.selection_domain().as_select_mode(),
        Some(SelectMode::Edge)
    );
    assert_eq!(
        SelectionDomain::from(SelectMode::Edge),
        SelectionDomain::Edge
    );

    // Transição para Face
    state.set_selection_domain(SelectionDomain::Face);
    assert_eq!(state.selection_domain(), SelectionDomain::Face);
    assert_eq!(
        state.selection_domain().as_select_mode(),
        Some(SelectMode::Face)
    );
    assert_eq!(
        SelectionDomain::from(SelectMode::Face),
        SelectionDomain::Face
    );

    // Retorno para Object (não é modo de componente no SelectMode legado)
    state.set_selection_domain(SelectionDomain::Object);
    assert_eq!(state.selection_domain(), SelectionDomain::Object);
    assert_eq!(state.selection_domain().as_select_mode(), None);
}

// ----------------------------------------------------------------------------
// Gate 2: IDs tipados e newtypes seguros (VertexIndex, FaceIndex, EdgeKey)
// ----------------------------------------------------------------------------
#[test]
fn test_gate2_typed_ids_prevent_primitive_index_confusion() {
    let mut sel = Selection::default();

    // Vértices usando VertexIndex
    let vi = VertexIndex(42);
    sel.select_vertex(vi);
    assert!(sel.contains_vertex_index(vi));
    assert!(sel.contains_vertex(42));
    assert!(!sel.contains_vertex_index(VertexIndex(43)));

    sel.deselect_vertex(vi);
    assert!(!sel.contains_vertex_index(vi));

    // Faces usando FaceIndex
    let fi = FaceIndex(7);
    sel.select_face(fi);
    assert!(sel.contains_face_index(fi));
    assert!(sel.contains_face(7));
    assert!(!sel.contains_face_index(FaceIndex(8)));

    sel.deselect_face(fi);
    assert!(!sel.contains_face_index(fi));

    // Arestas usando EdgeKey (ordem normalizada a < b)
    let ek = EdgeKey::new(10, 5);
    sel.select_edge(ek);
    assert!(sel.contains_edge_key(ek));
    assert!(sel.contains_edge_key(EdgeKey::new(5, 10)));
    assert!(sel.contains_edge((5, 10)));
    assert!(sel.contains_edge((10, 5)));

    sel.deselect_edge(ek);
    assert!(!sel.contains_edge_key(ek));
}

// ----------------------------------------------------------------------------
// Gate 3: Remap topológico resiliente (Selection::remap)
// ----------------------------------------------------------------------------
#[test]
fn test_gate3_topological_remapping_updates_selection_consistently() {
    let mut sel = Selection::default();
    sel.select_vertex(VertexIndex(0));
    sel.select_vertex(VertexIndex(1));
    sel.select_vertex(VertexIndex(2));

    sel.select_edge(EdgeKey::new(0, 1));
    sel.select_edge(EdgeKey::new(1, 2));

    sel.select_face(FaceIndex(0));
    sel.select_face(FaceIndex(1));

    // Simula fusão (weld/collapse) de 1 em 0 e remoção de face 1
    let remap = ElementRemap {
        vertices: vec![
            (0, Some(0)),
            (1, Some(0)), // Vértice 1 fundido no vértice 0
            (2, Some(1)), // Vértice 2 reindexado para 1
        ],
        faces: vec![
            (0, Some(0)),
            (1, None), // Face 1 foi colapsada/removida
        ],
    };

    sel.remap(&remap);

    // Vértices remapeados
    assert!(sel.contains_vertex_index(VertexIndex(0)));
    assert!(sel.contains_vertex_index(VertexIndex(1)));
    assert!(!sel.contains_vertex_index(VertexIndex(2)));

    // A aresta (0, 1) colapsou (ambos viraram 0) -> foi eliminada
    assert!(!sel.contains_edge((0, 0)));
    // A aresta (1, 2) virou (0, 1)
    assert!(sel.contains_edge_key(EdgeKey::new(0, 1)));

    // Faces remapeadas
    assert!(sel.contains_face_index(FaceIndex(0)));
    assert!(!sel.contains_face_index(FaceIndex(1)));
}

// ----------------------------------------------------------------------------
// Gate 4: Autoridade da Application e sincronização explícita com Mesh
// ----------------------------------------------------------------------------
#[test]
fn test_gate4_application_is_single_selection_authority_over_mesh() {
    let mut state = AppState::default();
    let add_cube = AddPrimitiveCmd::cube(true);
    state.dispatch(&add_cube).unwrap();

    // Seleciona todos os componentes no domínio Face via comando da aplicação
    state.set_selection_domain(SelectionDomain::Face);
    let select_all = SelectAllCmd;
    state.dispatch(&select_all).unwrap();

    // A seleção reside e é governada pela sessão da Application
    assert!(!state.session.selection.faces.is_empty());
    assert_eq!(state.session.selection.faces.len(), 6); // 6 faces do cubo

    // A malha pode ser sincronizada via apply_to_mesh para renderização/export
    let mesh = state.project.active_mesh_mut().unwrap();
    state.session.selection.apply_to_mesh(mesh);
    assert!(mesh.faces.iter().all(|f| f.selected));

    // Limpar seleção na Application reflete no estado
    state.session.selection.clear();
    assert!(state.session.selection.faces.is_empty());

    let mesh = state.project.active_mesh_mut().unwrap();
    state.session.selection.apply_to_mesh(mesh);
    assert!(mesh.faces.iter().all(|f| !f.selected));
}

// ----------------------------------------------------------------------------
// Gate 5: Preservação de seleção após Undo/Redo e poda de stale references
// ----------------------------------------------------------------------------
#[test]
fn test_gate5_undo_redo_preserves_selection_and_prunes_stale_references() {
    let mut state = AppState::default();
    let initial_obj = state.project.assets[0].id;

    state.dispatch(&AddPrimitiveCmd::cube(true)).unwrap();
    assert_eq!(state.project.assets.len(), 2);
    let second_obj = state.project.assets[1].id;

    state.session.selection.filters = SelectionFilters::all();
    state
        .session
        .selection
        .select_item(SelectionRef::object(second_obj));
    state
        .session
        .selection
        .select_item(SelectionRef::vertex(second_obj, 3));

    assert_eq!(state.session.selection.assets, vec![second_obj]);

    // Undo da criação do segundo objeto
    assert!(state.undo());
    assert_eq!(state.project.assets.len(), 1);

    // O objeto deletado foi removido da seleção automaticamente, sem stale pointer
    assert!(!state.session.selection.assets.contains(&second_obj));
    assert!(state.session.selection.assets.contains(&initial_obj));

    // Redo restaura o objeto
    assert!(state.redo());
    assert_eq!(state.project.assets.len(), 2);
}
