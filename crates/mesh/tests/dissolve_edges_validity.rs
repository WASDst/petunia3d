//! Dissolver arestas nunca pode deixar uma face com vértice repetido: o caso
//! veio de uma coluna esticada em que duas arestas vizinhas eram dissolvidas.

use petunia_mesh::Mesh;

/// Todas as faces precisam seguir válidas e sem vértices repetidos.
fn assert_valid(mesh: &Mesh, context: &str) {
    assert!(
        mesh.validate_invariants().is_ok(),
        "{context}: {:?}",
        mesh.validate_invariants()
    );
    for (index, face) in mesh.faces.iter().enumerate() {
        let mut seen = std::collections::HashSet::new();
        assert!(
            face.verts.iter().all(|v| seen.insert(*v)),
            "{context}: a face {index} repete vértice: {:?}",
            face.verts
        );
    }
}

#[test]
fn dissolving_two_neighbouring_edges_keeps_every_face_simple() {
    let mut mesh = Mesh::cube(2.0);
    let first_two: Vec<(u32, u32)> = mesh.edges_unique().into_iter().take(2).collect();
    mesh.selected_edges.extend(first_two);
    mesh.dissolve_selected();
    assert_valid(&mesh, "duas arestas vizinhas");
}

#[test]
fn dissolving_any_edge_pair_of_a_cube_never_pinches_a_face() {
    let base = Mesh::cube(2.0);
    let edges = base.edges_unique();
    for i in 0..edges.len() {
        for j in (i + 1)..edges.len() {
            let mut mesh = base.clone();
            mesh.selected_edges.insert(edges[i]);
            mesh.selected_edges.insert(edges[j]);
            mesh.dissolve_selected();
            assert_valid(&mesh, &format!("arestas {:?} e {:?}", edges[i], edges[j]));
        }
    }
}

#[test]
fn dissolving_every_edge_of_a_cube_stays_valid() {
    let mut mesh = Mesh::cube(2.0);
    let all = mesh.edges_unique();
    mesh.selected_edges.extend(all);
    mesh.dissolve_selected();
    assert_valid(&mesh, "todas as arestas");
}

#[test]
fn dissolving_any_single_vertex_of_a_cube_stays_valid() {
    for vertex in 0..8usize {
        let mut mesh = Mesh::cube(2.0);
        mesh.verts[vertex].selected = true;
        mesh.dissolve_selected();
        assert_valid(&mesh, &format!("vértice {vertex}"));
    }
}

#[test]
fn dissolving_any_face_pair_or_triple_of_a_cube_stays_valid() {
    let base = Mesh::cube(2.0);
    let faces = base.faces.len();
    for i in 0..faces {
        for j in (i + 1)..faces {
            let mut mesh = base.clone();
            mesh.faces[i].selected = true;
            mesh.faces[j].selected = true;
            mesh.dissolve_selected();
            assert_valid(&mesh, &format!("faces {i} e {j}"));
            for k in (j + 1)..faces {
                let mut mesh = base.clone();
                for index in [i, j, k] {
                    mesh.faces[index].selected = true;
                }
                mesh.dissolve_selected();
                assert_valid(&mesh, &format!("faces {i}, {j} e {k}"));
            }
        }
    }
}

#[test]
fn dissolving_edges_of_a_subdivided_cube_stays_valid() {
    let mut base = Mesh::cube(2.0);
    for face in &mut base.faces {
        face.selected = true;
    }
    base.subdivide_selected();
    base.deselect_all();
    let edges = base.edges_unique();
    // Pares espaçados: cobre arestas vizinhas e distantes sem explodir o tempo do teste.
    for i in (0..edges.len()).step_by(3) {
        for j in (i + 1..edges.len()).step_by(5) {
            let mut mesh = base.clone();
            mesh.selected_edges.insert(edges[i]);
            mesh.selected_edges.insert(edges[j]);
            mesh.dissolve_selected();
            assert_valid(
                &mesh,
                &format!("subdividido: {:?} e {:?}", edges[i], edges[j]),
            );
        }
    }
}
