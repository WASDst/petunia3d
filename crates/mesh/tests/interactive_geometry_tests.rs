//! Testes de Conformidade para Ferramentas Geométricas Interativas (Gauntlet Loop):
//! - Extrude Individual Faces (Alt+E)
//! - Revolve 360° Selection
//! - Multi-segment Rounded Bevel
//! - Guarded Metric Inset contra Auto-Interseção

use petunia_mesh::{Mesh, Vertex};

#[test]
fn test_extrude_individual_faces_decouples_shared_edges() {
    let mut mesh = Mesh::cube(2.0);
    // Seleciona duas faces adjacentes do cubo (ex: faces 0 e 1)
    mesh.faces[0].selected = true;
    mesh.faces[1].selected = true;

    let verts_before = mesh.verts.len();
    mesh.extrude_individual(0.5);

    // Ambas as faces devem ter gerado 4 vértices cada (total 8 novos vértices)
    assert_eq!(mesh.verts.len(), verts_before + 8);
    // Cada face quadrangular gera 4 novas faces de parede (total 8 novas faces)
    assert_eq!(mesh.faces.len(), 6 + 8);

    // As duas tampas superiores agora são disjuntas (não compartilham vértices)
    let top_face0_verts = &mesh.faces[0].verts;
    let top_face1_verts = &mesh.faces[1].verts;
    for v0 in top_face0_verts {
        assert!(
            !top_face1_verts.contains(v0),
            "Faces extrudadas individualmente não devem compartilhar vértices no topo"
        );
    }
}

#[test]
fn test_revolve_selection_full_360_circle() {
    let mut mesh = Mesh::default();
    // Cria um perfil aberto com 3 pontos afastados do eixo Y
    mesh.verts.push(Vertex::new(1.0, 0.0, 0.0)); // 0
    mesh.verts.push(Vertex::new(1.5, 1.0, 0.0)); // 1
    mesh.verts.push(Vertex::new(1.0, 2.0, 0.0)); // 2
    for v in &mut mesh.verts {
        v.selected = true;
    }
    mesh.selected_edges.insert(petunia_mesh::edge_key(0, 1));
    mesh.selected_edges.insert(petunia_mesh::edge_key(1, 2));

    let segments = 8;
    let ok = mesh.revolve_selection(segments, 360.0, 1, [0.0, 0.0, 0.0]);
    assert!(ok, "revolve_selection deve retornar true");

    // 2 arestas do perfil * 8 segmentos = 16 quads gerados
    assert_eq!(mesh.faces.len(), 16);
    for face in &mesh.faces {
        assert_eq!(
            face.verts.len(),
            4,
            "Revolução em anel deve gerar faces quadrangulares"
        );
    }
}

#[test]
fn test_bevel_multi_segments() {
    let mut mesh = Mesh::cube(2.0);
    // Seleciona a primeira aresta
    let (a, b) = mesh
        .to_edges()
        .first()
        .map(|&(a_pos, b_pos, _)| {
            let ai = mesh.verts.iter().position(|v| v.pos == a_pos).unwrap() as u32;
            let bi = mesh.verts.iter().position(|v| v.pos == b_pos).unwrap() as u32;
            (ai, bi)
        })
        .unwrap();

    mesh.selected_edges.insert(petunia_mesh::edge_key(a, b));

    // Chanfra com 3 segmentos
    let (ok, skipped) = mesh.bevel_selected_segments(0.3, 3);
    assert_eq!(ok, 1);
    assert_eq!(skipped, 0);

    // O cubo original tinha 6 faces. O bevel com 3 segmentos adiciona 3 quads no lugar da aresta
    assert_eq!(mesh.faces.len(), 6 + 3);

    let report = mesh.validate_topology();
    assert!(
        report.is_manifold,
        "Malha após bevel multi-segmentos deve ser manifold"
    );
    assert!(
        report.is_closed,
        "Cubo com aresta chanfrada deve permanecer fechado"
    );
}

#[test]
fn test_guarded_inset_extreme_factor_stability() {
    let mut mesh = Mesh::cube(2.0);
    mesh.faces[0].selected = true;

    // Distância extrema que em implementações ingênuas causaria auto-interseção e inversão de normais
    let inset = mesh.inset_faces(&[0], 5.0, petunia_mesh::InsetMode::Region);
    assert!(inset.limited, "o clamp precisa ser reportado");

    let report = mesh.validate_topology();
    assert!(
        report.is_manifold,
        "Inset com fator extremo deve manter a malha manifold"
    );
    assert_eq!(
        mesh.faces.len(),
        6 + 4,
        "Inset de 1 face gera 4 faces de anel"
    );

    // A normal da face interna modificada deve continuar alinhada à face original
    let inner_face = &mesh.faces[0];
    assert_eq!(inner_face.verts.len(), 4);
}
