//! Integration tests for contextual selection, heterogeneous targets, and topology resolution (SEL-002, SEL-006).

use petunia_core::{
    ActiveElement, AppState, Selection, SelectionDomain, SelectionFilters, SelectionRef,
    SelectionResolver,
};
use petunia_mesh::{Face, Mesh, Vertex};
use uuid::Uuid;

fn sample_quad_mesh() -> Mesh {
    Mesh {
        verts: vec![
            Vertex::new(0.0, 0.0, 0.0),
            Vertex::new(1.0, 0.0, 0.0),
            Vertex::new(1.0, 1.0, 0.0),
            Vertex::new(0.0, 1.0, 0.0),
            Vertex::new(2.0, 0.0, 0.0),
            Vertex::new(2.0, 1.0, 0.0),
        ],
        faces: vec![
            Face::new(vec![0, 1, 2, 3]), // Quad 0: verts 0, 1, 2, 3
            Face::new(vec![1, 4, 5, 2]), // Quad 1: verts 1, 4, 5, 2 (shares edge 1-2 with Quad 0)
        ],
        ..Default::default()
    }
}

#[test]
fn shared_components_resolve_vertices_without_duplication() {
    let mesh = sample_quad_mesh();
    let obj_id = Uuid::new_v4();

    // Select Quad 0 (0,1,2,3), Quad 1 (1,4,5,2), shared edge (1,2), and vertex 2
    let items = vec![
        SelectionRef::face(obj_id, 0),
        SelectionRef::face(obj_id, 1),
        SelectionRef::edge(obj_id, 1, 2),
        SelectionRef::vertex(obj_id, 2),
    ];

    let targets = SelectionResolver::resolve(
        &items,
        |id| if id == obj_id { Some(&mesh) } else { None },
        |_| false,
        |_| None,
    );

    assert!(targets.objects.is_empty());
    let vertices = targets
        .vertices_for_object(obj_id)
        .expect("Resolved vertices for obj");
    // All unique vertices: 0, 1, 2, 3, 4, 5 exactly once, sorted
    assert_eq!(vertices, &[0, 1, 2, 3, 4, 5]);
    assert_eq!(targets.locked_targets.len(), 0);
    assert_eq!(
        targets.active_element,
        Some(SelectionRef::vertex(obj_id, 2))
    );
}

#[test]
fn object_selection_dominates_over_own_subcomponents() {
    let mesh = sample_quad_mesh();
    let obj_id = Uuid::new_v4();

    // Select the entire object AND a face and vertex of that same object
    let items = vec![
        SelectionRef::object(obj_id),
        SelectionRef::face(obj_id, 0),
        SelectionRef::vertex(obj_id, 4),
    ];

    let targets = SelectionResolver::resolve(
        &items,
        |id| if id == obj_id { Some(&mesh) } else { None },
        |_| false,
        |_| None,
    );

    // Object is in the object transformation list
    assert_eq!(targets.objects, vec![obj_id]);
    // Components of the object are NOT in the mesh_vertices list, preventing double transform
    assert!(!targets.mesh_vertices.contains_key(&obj_id));
}

#[test]
fn hierarchy_avoids_double_transformation_for_parent_and_child() {
    let parent = Uuid::new_v4();
    let child1 = Uuid::new_v4();
    let child2 = Uuid::new_v4();
    let grandchild = Uuid::new_v4();
    let independent = Uuid::new_v4();

    let items = vec![
        SelectionRef::object(parent),
        SelectionRef::object(child1),
        SelectionRef::object(child2),
        SelectionRef::object(grandchild),
        SelectionRef::object(independent),
    ];

    let parent_lookup = |id| {
        if id == child1 || id == child2 {
            Some(parent)
        } else if id == grandchild {
            Some(child1)
        } else {
            None
        }
    };

    let targets = SelectionResolver::resolve(&items, |_| None, |_| false, parent_lookup);

    // Only the parent and the independent object should be root targets
    assert_eq!(targets.objects, vec![parent, independent]);
}

#[test]
fn hierarchy_cycle_protection_terminates_safely() {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();

    // Mutual parenting cycle: a -> b -> a
    let parent_lookup = |id| {
        if id == a {
            Some(b)
        } else if id == b {
            Some(a)
        } else {
            None
        }
    };

    let roots = SelectionResolver::filter_hierarchy_roots(&[a, b], parent_lookup);
    // Should terminate gracefully without hanging or crashing
    assert!(!roots.is_empty());
}

#[test]
fn locked_assets_are_recorded_and_excluded_from_transformation() {
    let locked_id = Uuid::new_v4();
    let active_id = Uuid::new_v4();

    let items = vec![
        SelectionRef::object(locked_id),
        SelectionRef::vertex(locked_id, 0),
        SelectionRef::object(active_id),
    ];

    let targets = SelectionResolver::resolve(&items, |_| None, |id| id == locked_id, |_| None);

    assert_eq!(targets.objects, vec![active_id]);
    assert_eq!(targets.locked_targets.len(), 2);
    assert!(
        targets
            .locked_targets
            .contains(&SelectionRef::object(locked_id))
    );
    assert!(
        targets
            .locked_targets
            .contains(&SelectionRef::vertex(locked_id, 0))
    );
}

#[test]
fn mixed_heterogeneous_selection_partitions_correctly() {
    let mesh_a = sample_quad_mesh();
    let obj_a = Uuid::new_v4();
    let obj_b = Uuid::new_v4();
    let shape_id = Uuid::new_v4();

    let items = vec![
        SelectionRef::face(obj_a, 0),
        SelectionRef::object(obj_b),
        SelectionRef::shape(obj_a, shape_id),
    ];

    let targets = SelectionResolver::resolve(
        &items,
        |id| if id == obj_a { Some(&mesh_a) } else { None },
        |_| false,
        |_| None,
    );

    assert_eq!(targets.objects, vec![obj_b]);
    assert_eq!(targets.shapes, vec![(obj_a, shape_id)]);
    let a_verts = targets.vertices_for_object(obj_a).unwrap();
    assert_eq!(a_verts, &[0, 1, 2, 3]);
}

#[test]
fn app_state_selection_sync_preserves_contextual_items_and_filters() {
    let mut state = AppState::new("en");
    assert_eq!(state.selection_domain(), SelectionDomain::Object);
    assert_eq!(
        state.session.selection.filters,
        SelectionFilters::from_domain(SelectionDomain::Object)
    );

    // Switch selection domain to Edge
    state.set_selection_domain(SelectionDomain::Edge);
    assert_eq!(state.selection_domain(), SelectionDomain::Edge);
    assert_eq!(
        state.session.selection.filters,
        SelectionFilters::from_domain(SelectionDomain::Edge)
    );
    assert!(state.session.selection.filters.edges);
    assert!(!state.session.selection.filters.objects);

    // Switch to contextual selection with all filters enabled
    state.session.selection.filters = SelectionFilters::all();
    assert!(
        state
            .session
            .selection
            .filters
            .allows_domain(SelectionDomain::Object)
    );
    assert!(
        state
            .session
            .selection
            .filters
            .allows_domain(SelectionDomain::Vertex)
    );
    assert!(
        state
            .session
            .selection
            .filters
            .allows_domain(SelectionDomain::Edge)
    );
    assert!(
        state
            .session
            .selection
            .filters
            .allows_domain(SelectionDomain::Face)
    );
}

#[test]
fn selection_ref_ordering_and_active_element_tracking() {
    let mut sel = Selection {
        filters: SelectionFilters::all(),
        ..Default::default()
    };
    let obj = Uuid::new_v4();
    let f0 = SelectionRef::face(obj, 0);
    let e0 = SelectionRef::edge(obj, 1, 2);
    let v0 = SelectionRef::vertex(obj, 3);

    sel.select_item(f0);
    assert_eq!(sel.active_element, Some(ActiveElement::new(f0)));

    sel.select_item(e0);
    assert_eq!(sel.active_element, Some(ActiveElement::new(e0)));

    sel.select_item(v0);
    assert_eq!(sel.active_element, Some(ActiveElement::new(v0)));

    // Deselect the middle item
    sel.deselect_item(&e0);
    assert_eq!(sel.items, vec![f0, v0]);
    assert_eq!(sel.active_element, Some(ActiveElement::new(v0)));

    // Deselect the active element: previous becomes active
    sel.deselect_item(&v0);
    assert_eq!(sel.items, vec![f0]);
    assert_eq!(sel.active_element, Some(ActiveElement::new(f0)));
}

#[test]
fn mesh_selection_syncs_to_contextual_items_and_active_element() {
    let mut state = AppState::default();
    let asset_id = state.project.assets[0].id;

    // Mutate mesh selection directly on active asset
    let mesh = state.project.active_mesh_mut().expect("active mesh");
    mesh.faces[0].selected = true;
    mesh.selected_edges.insert((0, 1));
    mesh.verts[2].selected = true;

    state.sync_selection();

    let items = &state.session.selection.items;
    assert!(items.contains(&SelectionRef::face(asset_id, 0)));
    assert!(items.contains(&SelectionRef::edge(asset_id, 0, 1)));
    assert!(items.contains(&SelectionRef::vertex(asset_id, 2)));
    assert!(state.session.selection.active_element.is_some());

    // Deselect a face directly on the mesh
    let mesh = state.project.active_mesh_mut().expect("active mesh");
    mesh.faces[0].selected = false;

    state.sync_selection();

    let items = &state.session.selection.items;
    assert!(!items.contains(&SelectionRef::face(asset_id, 0)));
    assert!(items.contains(&SelectionRef::edge(asset_id, 0, 1)));
    assert!(items.contains(&SelectionRef::vertex(asset_id, 2)));
}

#[test]
fn undo_redo_reconciles_contextual_selection_without_stale_references() {
    let mut state = AppState::default();
    let initial_id = state.project.assets[0].id;

    // Add a new cube primitive via command
    let add_cube = petunia_core::AddPrimitiveCmd::cube(true);
    state.dispatch(&add_cube).expect("add cube succeeds");
    assert_eq!(state.project.assets.len(), 2);
    let new_id = state.project.assets[1].id;

    // Enable contextual selection filters so vertices/edges/faces can be selected
    state.session.selection.filters = SelectionFilters::all();

    // Select the new object contextually
    state
        .session
        .selection
        .select_item(SelectionRef::object(new_id));
    state
        .session
        .selection
        .select_item(SelectionRef::vertex(new_id, 0));
    assert_eq!(
        state.session.selection.active_element,
        Some(ActiveElement::new(SelectionRef::vertex(new_id, 0)))
    );

    // Undo the primitive creation
    assert!(state.undo());
    assert_eq!(state.project.assets.len(), 1);

    // Contextual selection should prune the deleted asset's references
    assert!(
        !state
            .session
            .selection
            .items
            .contains(&SelectionRef::object(new_id))
    );
    assert!(
        !state
            .session
            .selection
            .items
            .contains(&SelectionRef::vertex(new_id, 0))
    );
    assert_eq!(
        state.session.selection.items,
        vec![SelectionRef::object(initial_id)]
    );
    assert_eq!(
        state.session.selection.active_element,
        Some(ActiveElement::new(SelectionRef::object(initial_id)))
    );

    // Redo restores the object
    assert!(state.redo());
    assert_eq!(state.project.assets.len(), 2);
}
