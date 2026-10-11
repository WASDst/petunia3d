use petunia_mesh::{Face, Mesh, Vertex};
use std::collections::HashSet;

#[test]
fn quad_cut_count_is_uniform_not_recursive() {
    let mut mesh = Mesh::plane(2.0);
    mesh.select_all();
    mesh.subdivide_selected_cuts(2);

    // Two cuts create three segments per original edge: 3 x 3 = 9 quads.
    assert_eq!(mesh.faces.len(), 9);
    assert_eq!(mesh.verts.len(), 16);
    assert!(mesh.faces.iter().all(|face| face.verts.len() == 4));
    assert!(mesh.faces.iter().all(|face| face.selected));
    assert!(
        mesh.faces
            .iter()
            .all(|face| face.uv.len() == face.verts.len())
    );
}

#[test]
fn triangle_cut_count_produces_n_squared_triangles() {
    let mut face = Face::with_uv(vec![0, 1, 2], vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
    face.selected = true;
    let mut mesh = Mesh {
        verts: vec![
            Vertex::new(0.0, 0.0, 0.0),
            Vertex::new(3.0, 0.0, 0.0),
            Vertex::new(0.0, 3.0, 0.0),
        ],
        faces: vec![face],
        selected_edges: HashSet::new(),
        uv_seams: HashSet::new(),
        uv_pinned: HashSet::new(),
        sharp_edges: Default::default(),
    };

    mesh.subdivide_selected_cuts(2);

    // Three segments per edge create 3^2 triangles and 10 lattice vertices.
    assert_eq!(mesh.faces.len(), 9);
    assert_eq!(mesh.verts.len(), 10);
    assert!(mesh.faces.iter().all(|face| face.verts.len() == 3));
    assert!(mesh.faces.iter().all(|face| face.uv.len() == 3));
}

#[test]
fn adjacent_selected_quads_share_edge_split_vertices() {
    let left = Face::with_uv(
        vec![0, 1, 4, 3],
        vec![[0.0, 0.0], [0.5, 0.0], [0.5, 1.0], [0.0, 1.0]],
    );
    let right = Face::with_uv(
        vec![1, 2, 5, 4],
        vec![[0.5, 0.0], [1.0, 0.0], [1.0, 1.0], [0.5, 1.0]],
    );
    let mut mesh = Mesh {
        verts: vec![
            Vertex::new(0.0, 0.0, 0.0),
            Vertex::new(1.0, 0.0, 0.0),
            Vertex::new(2.0, 0.0, 0.0),
            Vertex::new(0.0, 1.0, 0.0),
            Vertex::new(1.0, 1.0, 0.0),
            Vertex::new(2.0, 1.0, 0.0),
        ],
        faces: vec![left, right],
        selected_edges: HashSet::new(),
        uv_seams: HashSet::new(),
        uv_pinned: HashSet::new(),
        sharp_edges: Default::default(),
    };
    mesh.select_all();

    mesh.subdivide_selected_cuts(2);

    assert_eq!(mesh.faces.len(), 18);
    let shared_edge_vertices = mesh
        .verts
        .iter()
        .filter(|vertex| {
            (vertex.pos[0] - 1.0).abs() < 1.0e-5
                && vertex.pos[1] >= -1.0e-5
                && vertex.pos[1] <= 1.0 + 1.0e-5
                && vertex.pos[2].abs() < 1.0e-5
        })
        .count();
    // endpoints + two inserted cuts; duplicate boundary vertices would make this 6.
    assert_eq!(shared_edge_vertices, 4);
    assert!(
        mesh.faces
            .iter()
            .all(|face| face.uv.len() == face.verts.len())
    );
}
