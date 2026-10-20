//! Normals, SurfaceShading e Sharp Edges — G18 Gates Tests.
//! Website doc: website/docs/03-geometry/normals-shading-sharp-edges.md
//!
//! Gates obrigatórios:
//! 1. Cylinder Smooth com cap boundaries Sharp mantém tampas planas (+Y/-Y) e laterais suaves (XZ).
//! 2. Uneven face areas com zero divergência entre Render e Export (corner-angle weighting).
//! 3. glTF vertex split em hard-normal boundary (split na aresta sharp; compartilhado na suave).
//! 4. OBJ normal indices por corner (índices distintos na aresta sharp; índice compartilhado na suave).

use glam::Vec3;
use petunia_mesh::shading::{SurfaceShading, compute_corner_normals};
use petunia_mesh::{Face, Mesh, Vertex, edge_key};
use petunia_project::export::{export_gltf, export_obj};
use petunia_project::{Asset, Project};

fn close_vec3(a: [f32; 3], b: [f32; 3], tol: f32) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() <= tol)
}

#[test]
fn test_g18_gate_cylinder_smooth_with_cap_boundaries_sharp() {
    let sides = 16;
    let mut cyl = Mesh::cylinder(sides, 1.0, 2.0);

    // O gerador radial_frustum marca as bordas das tampas como Sharp.
    // 16 arestas no anel inferior + 16 arestas no anel superior = 32 arestas sharp.
    assert_eq!(
        cyl.sharp_edges.len(),
        (sides * 2) as usize,
        "Cylinder deve gerar bordas de topo e fundo como Sharp edges"
    );

    let normals_sharp = compute_corner_normals(&cyl, SurfaceShading::Smooth);

    // 1. As faces das tampas (topo: normal +Y [0, 1, 0]; fundo: normal -Y [0, -1, 0])
    // devem ter corners exatamente perpendiculares (planos), sem curvatura das laterais.
    for (fi, face) in cyl.faces.iter().enumerate() {
        let fnorm = cyl.face_normal(fi);
        if fnorm.y > 0.99 {
            // Tampa do topo (+Y)
            for corner in 0..face.verts.len() {
                let n = normals_sharp.corner(fi, corner);
                assert!(
                    close_vec3(n, [0.0, 1.0, 0.0], 1e-4),
                    "Corner na tampa do topo deve ter normal [0, 1, 0], obteve {n:?}"
                );
            }
        } else if fnorm.y < -0.99 {
            // Tampa do fundo (-Y)
            for corner in 0..face.verts.len() {
                let n = normals_sharp.corner(fi, corner);
                assert!(
                    close_vec3(n, [0.0, -1.0, 0.0], 1e-4),
                    "Corner na tampa do fundo deve ter normal [0, -1, 0], obteve {n:?}"
                );
            }
        } else {
            // Faces laterais: normais no plano XZ, Y deve ser zero
            for corner in 0..face.verts.len() {
                let n = normals_sharp.corner(fi, corner);
                assert!(
                    n[1].abs() < 1e-4,
                    "Corner na lateral do cilindro não deve vazar componente Y ({}), obteve {n:?}",
                    n[1]
                );
                // Direção radial: vetor da posição no plano XZ
                let vi = face.verts[corner] as usize;
                let pos = cyl.verts[vi].pos;
                let radial = Vec3::new(pos[0], 0.0, pos[2]).normalize();
                let corner_v = Vec3::from_slice(&n);
                assert!(
                    radial.dot(corner_v) > 0.95,
                    "Corner normal {corner_v} deve apontar na direção radial {radial}"
                );
            }
        }
    }

    // 2. Se removermos as arestas sharp, as laterais passam a misturar com as tampas
    let sharp_backup = cyl.sharp_edges.clone();
    cyl.sharp_edges.clear();
    let normals_unsharp = compute_corner_normals(&cyl, SurfaceShading::Smooth);

    // Na malha unsharp, os corners superiores da lateral vazam Y positivo para a lateral
    let mut found_bleeding = false;
    for (fi, face) in cyl.faces.iter().enumerate() {
        let fnorm = cyl.face_normal(fi);
        if fnorm.y.abs() < 0.1 {
            // Face lateral
            for corner in 0..face.verts.len() {
                let n = normals_unsharp.corner(fi, corner);
                if n[1].abs() > 0.1 {
                    found_bleeding = true;
                }
            }
        }
    }
    assert!(
        found_bleeding,
        "Sem sharp edges, as normais das laterais devem misturar com as tampas"
    );

    // Restaurar sharp edges
    cyl.sharp_edges = sharp_backup;
}

#[test]
fn test_g18_gate_uneven_face_areas_zero_render_export_difference() {
    // Malha com duas faces incidentes no vértice 0, porém com áreas desiguais em proporção 1:100.
    // Triângulo 0: cantos (0,0,0), (0.1, 0, 0), (0, 0.1, 0). Área: 0.005. Ângulo no vértice 0: 90 graus (pi/2).
    // Triângulo 1: cantos (0,0,0), (0, 0.1, 0), (0, 0, 10.0). Área: 0.5 (100x maior). Ângulo no vértice 0: 90 graus (pi/2).
    let mut mesh = Mesh {
        verts: vec![
            Vertex::new(0.0, 0.0, 0.0),  // 0
            Vertex::new(0.1, 0.0, 0.0),  // 1
            Vertex::new(0.0, 0.1, 0.0),  // 2
            Vertex::new(0.0, 0.0, 10.0), // 3
        ],
        faces: vec![
            Face::new(vec![0, 1, 2]), // Face normal: [0, 0, 1]
            Face::new(vec![0, 2, 3]), // Face normal: [1, 0, 0]
        ],
        ..Default::default()
    };
    mesh.layout_uv_charts(0.01);

    // Como os ângulos nos cantos incidentes no vértice 0 são ambos 90° (pi/2),
    // o cálculo por corner-angle dá peso IGUAL a ambas as faces, independentemente da área!
    let corner_normals = compute_corner_normals(&mesh, SurfaceShading::Smooth);

    let n0_f0 = corner_normals.corner(0, 0); // Vértice 0 na face 0
    let n0_f1 = corner_normals.corner(1, 0); // Vértice 0 na face 1

    // Ambos devem ser idênticos e apontar exatamente no bissetor (1/sqrt(2), 0, 1/sqrt(2))
    let bisector = 1.0 / 2.0f32.sqrt();
    assert!(
        close_vec3(n0_f0, [bisector, 0.0, bisector], 1e-4),
        "Normal corner face 0 deve ser bissetora [{bisector}, 0, {bisector}], obteve {n0_f0:?}"
    );
    assert!(
        close_vec3(n0_f1, [bisector, 0.0, bisector], 1e-4),
        "Normal corner face 1 deve ser bissetora [{bisector}, 0, {bisector}], obteve {n0_f1:?}"
    );

    // Agora conferir paridade exata Render vs Export:
    // Render: Mesh::to_triangles_shaded
    let render_tris = mesh.to_triangles_shaded(SurfaceShading::Smooth);
    // Export: Mesh::shaded_corners
    let export_corners = mesh.shaded_corners(SurfaceShading::Smooth);

    assert_eq!(render_tris.len(), export_corners.len());
    for (render_tri, export_corner) in render_tris.iter().zip(&export_corners) {
        assert_eq!(
            render_tri.0, export_corner.position,
            "Posições devem ser idênticas"
        );
        assert_eq!(
            render_tri.1, export_corner.normal,
            "Normais Render e Export devem ser estritamente idênticas"
        );
        assert_eq!(
            render_tri.2, export_corner.color,
            "Cores devem ser idênticas"
        );
        assert_eq!(render_tri.3, export_corner.uv, "UVs devem ser idênticas");
    }
}

#[test]
fn test_g18_gate_gltf_vertex_split_on_hard_normal_boundary() {
    // Duas faces formando uma dobra de 90° com aresta compartilhada (1, 2)
    // Face 0: (0, 1, 2, 3) no plano XY (normal [0, 0, 1])
    // Face 1: (1, 4, 5, 2) no plano YZ (normal [1, 0, 0])
    let mut mesh = Mesh {
        verts: vec![
            Vertex::new(-1.0, 0.0, 0.0), // 0
            Vertex::new(0.0, 0.0, 0.0),  // 1 (borda compartilhada)
            Vertex::new(0.0, 1.0, 0.0),  // 2 (borda compartilhada)
            Vertex::new(-1.0, 1.0, 0.0), // 3
            Vertex::new(0.0, 0.0, -1.0), // 4
            Vertex::new(0.0, 1.0, -1.0), // 5
        ],
        faces: vec![Face::new(vec![0, 1, 2, 3]), Face::new(vec![1, 4, 5, 2])],
        ..Default::default()
    };
    mesh.layout_uv_charts(0.01);

    // Caso A: Aresta compartilhada marcada como Sharp
    mesh.sharp_edges.insert(edge_key(1, 2));

    let mut project = Project::new();
    project.assets.clear();
    let mut asset = Asset::new("FoldedSheet", mesh.clone());
    asset.surface_shading = SurfaceShading::Smooth;
    project.assets.push(asset);

    let glb_bytes_sharp = export_gltf(&project, &[0]).expect("Export GLB sharp");
    let (doc_sharp, buffers_sharp, _) =
        gltf::import_slice(&glb_bytes_sharp).expect("Parse GLB sharp");
    let get_sharp = |b: gltf::Buffer| Some(&buffers_sharp[b.index()][..]);

    let mesh_sharp = doc_sharp.meshes().next().expect("Mesh sharp no glTF");
    let prim_sharp = mesh_sharp
        .primitives()
        .next()
        .expect("Primitive sharp no glTF");
    let reader_sharp = prim_sharp.reader(get_sharp);
    let positions_sharp: Vec<[f32; 3]> =
        reader_sharp.read_positions().expect("positions").collect();
    let normals_sharp: Vec<[f32; 3]> = reader_sharp.read_normals().expect("normals").collect();

    // Como a aresta (1, 2) é Sharp, os vértices 1 e 2 possuem normais divergentes em cada face:
    // Face 0: normal [0, 0, 1]
    // Face 1: normal [1, 0, 0]
    // Logo, o glTF divide os vértices ao longo da borda Sharp (vertex split)!
    // A malha triangulada tem 4 triângulos = 12 corners.
    // Sem duplicar na borda lisa seriam menos vértices, mas com split os vértices 1 e 2 são duplicados.
    assert!(
        positions_sharp.len() > 6,
        "Vértices devem ser divididos na aresta sharp no glTF (obtido {})",
        positions_sharp.len()
    );

    // Verificar que existem vértices na mesma posição com normais distintas
    let mut splits_found = 0;
    for i in 0..positions_sharp.len() {
        for j in (i + 1)..positions_sharp.len() {
            if close_vec3(positions_sharp[i], positions_sharp[j], 1e-4) {
                // Mesma posição (vértice 1 ou 2)
                let dot =
                    Vec3::from_slice(&normals_sharp[i]).dot(Vec3::from_slice(&normals_sharp[j]));
                if dot < 0.1 {
                    splits_found += 1;
                }
            }
        }
    }
    assert!(
        splits_found >= 2,
        "Deve haver split normal em pelo menos 2 vértices na aresta Sharp, encontrou {splits_found}"
    );

    // Caso B: Aresta suave (sem Sharp)
    mesh.sharp_edges.clear();
    project.assets[0].mesh = mesh.into();
    let glb_bytes_smooth = export_gltf(&project, &[0]).expect("Export GLB smooth");
    let (doc_smooth, buffers_smooth, _) =
        gltf::import_slice(&glb_bytes_smooth).expect("Parse GLB smooth");
    let get_smooth = |b: gltf::Buffer| Some(&buffers_smooth[b.index()][..]);
    let prim_smooth = doc_smooth
        .meshes()
        .next()
        .unwrap()
        .primitives()
        .next()
        .unwrap();
    let reader_smooth = prim_smooth.reader(get_smooth);
    let positions_smooth: Vec<[f32; 3]> = reader_smooth.read_positions().unwrap().collect();
    let normals_smooth: Vec<[f32; 3]> = reader_smooth.read_normals().unwrap().collect();

    // Na aresta suave, os vértices 1 e 2 compartilham a normal média e não são divididos por normal!
    // Se houver coincidência de UV ou chart único, há menos vértices que no caso sharp.
    // E nenhuma normal em posições coincidentes tem produto escalar ~0.
    for i in 0..positions_smooth.len() {
        for j in (i + 1)..positions_smooth.len() {
            if close_vec3(positions_smooth[i], positions_smooth[j], 1e-4) {
                let dot =
                    Vec3::from_slice(&normals_smooth[i]).dot(Vec3::from_slice(&normals_smooth[j]));
                assert!(
                    dot > 0.99,
                    "Na malha suave, vértices coincidentes não devem divergir na normal (dot={dot})"
                );
            }
        }
    }
}

#[test]
fn test_g18_gate_obj_normal_indices_per_corner() {
    let mut mesh = Mesh {
        verts: vec![
            Vertex::new(-1.0, 0.0, 0.0), // 0
            Vertex::new(0.0, 0.0, 0.0),  // 1 (borda compartilhada)
            Vertex::new(0.0, 1.0, 0.0),  // 2 (borda compartilhada)
            Vertex::new(-1.0, 1.0, 0.0), // 3
            Vertex::new(0.0, 0.0, -1.0), // 4
            Vertex::new(0.0, 1.0, -1.0), // 5
        ],
        faces: vec![Face::new(vec![0, 1, 2, 3]), Face::new(vec![1, 4, 5, 2])],
        ..Default::default()
    };
    mesh.layout_uv_charts(0.01);

    // 1. Quando a aresta é Sharp:
    mesh.sharp_edges.insert(edge_key(1, 2));
    let obj_sharp = mesh.to_obj_with_normals(SurfaceShading::Smooth);

    // O OBJ gerado deve conter múltiplas linhas vn e as faces 'f v/vt/vn' devem
    // referenciar índices vn diferentes para os cantos em faces diferentes no vértice 1.
    let face_lines_sharp: Vec<&str> = obj_sharp.lines().filter(|l| l.starts_with('f')).collect();
    assert_eq!(face_lines_sharp.len(), 2, "Duas faces exportadas");

    // Extrair índices vn de cada canto da face
    let extract_vn_indices = |face_line: &str| -> Vec<usize> {
        face_line
            .split_whitespace()
            .skip(1)
            .map(|tok| {
                let parts: Vec<&str> = tok.split('/').collect();
                parts.last().unwrap().parse::<usize>().unwrap()
            })
            .collect()
    };

    let f0_vns = extract_vn_indices(face_lines_sharp[0]);
    let f1_vns = extract_vn_indices(face_lines_sharp[1]);

    // O vértice 1 é o corner 1 na Face 0 e o corner 0 na Face 1.
    // Como a aresta é sharp, eles NÃO devem compartilhar o mesmo índice de normal vn!
    let vn_f0_v1 = f0_vns[1];
    let vn_f1_v1 = f1_vns[0];
    assert_ne!(
        vn_f0_v1, vn_f1_v1,
        "No OBJ com aresta Sharp, as faces devem usar vn diferentes no mesmo vértice ({vn_f0_v1} vs {vn_f1_v1})"
    );

    // O vértice 2 é o corner 2 na Face 0 e o corner 3 na Face 1.
    let vn_f0_v2 = f0_vns[2];
    let vn_f1_v2 = f1_vns[3];
    assert_ne!(
        vn_f0_v2, vn_f1_v2,
        "No OBJ com aresta Sharp, as faces devem usar vn diferentes no vértice 2 ({vn_f0_v2} vs {vn_f1_v2})"
    );

    // 2. Quando a aresta é Smooth:
    mesh.sharp_edges.clear();
    let obj_smooth = mesh.to_obj_with_normals(SurfaceShading::Smooth);
    let face_lines_smooth: Vec<&str> = obj_smooth.lines().filter(|l| l.starts_with('f')).collect();

    let f0_vns_smooth = extract_vn_indices(face_lines_smooth[0]);
    let f1_vns_smooth = extract_vn_indices(face_lines_smooth[1]);

    // Na aresta suave, as faces compartilham exatamente o mesmo índice vn no vértice 1 e 2!
    assert_eq!(
        f0_vns_smooth[1], f1_vns_smooth[0],
        "No OBJ com aresta suave, as faces devem compartilhar o mesmo vn no vértice 1"
    );
    assert_eq!(
        f0_vns_smooth[2], f1_vns_smooth[3],
        "No OBJ com aresta suave, as faces devem compartilhar o mesmo vn no vértice 2"
    );

    // Também testar via wrapper de projeto `export_obj`
    let mut project = Project::new();
    project.assets.clear();
    let mut asset = Asset::new("ObjTest", mesh);
    asset.surface_shading = SurfaceShading::Smooth;
    project.assets.push(asset);

    let exported_obj_str = export_obj(&project.assets[0]);
    assert!(exported_obj_str.contains("vn "));
    assert!(exported_obj_str.contains("f "));
}
