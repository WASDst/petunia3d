use glam::Vec3;
use petunia_core::modeling_feedback::MeasurementQuality;
use petunia_core::section_analysis::{
    ClearSectionPlaneCmd, FlipSectionPlaneCmd, LoopKind, OffsetSectionPlaneCmd, SectionAnchor,
    SectionInspector, SectionPlane, SetSectionPlaneCmd, ThicknessProbeQuery, WallType,
};
use petunia_core::state::AppState;
use petunia_mesh::Mesh;
use petunia_project::ObjectTransform;
use uuid::Uuid;

/// Cria uma malha de cubo canônico unitário/básico centrado na origem (-1.0 a +1.0 nos eixos X, Y, Z).
fn create_test_cube() -> Mesh {
    Mesh::cube(2.0)
}

/// Cria um plano de malha aberta quad (4 vértices, 1 quad) no plano XZ de -1 a +1.
fn create_open_quad_mesh() -> Mesh {
    Mesh::plane(2.0)
}

/// Cria uma malha aninhada com dois cubos concêntricos (outer 4x4x4, inner 2x2x2).
fn create_nested_cubes_mesh() -> Mesh {
    let mut mesh = Mesh::cube(4.0);
    let inner = Mesh::cube(2.0);
    let base_idx = mesh.verts.len() as u32;
    for v in inner.verts {
        mesh.verts.push(v);
    }
    for f in inner.faces {
        let new_verts = f.verts.into_iter().map(|idx| idx + base_idx).collect();
        mesh.faces.push(petunia_mesh::Face::new(new_verts));
    }
    mesh
}

#[test]
fn test_cube_closed_section_and_synthetic_cap() {
    let mesh = create_test_cube();
    let plane = SectionPlane::xz(0.0); // Corta em Y = 0
    let asset_id = Uuid::new_v4();

    let res = SectionInspector::intersect_mesh_plane(asset_id, &mesh, None, &plane);

    assert_eq!(res.closed_contours_count, 1);
    assert_eq!(res.open_contours_count, 0);
    assert_eq!(res.holes_count, 0);
    assert_eq!(res.quality, MeasurementQuality::Exact);

    // O corte de um cubo 2x2x2 em Y=0 produz um quadrado 2x2
    // Perímetro = 2 * 4 = 8.0
    // Área = 2 * 2 = 4.0
    assert!((res.total_perimeter - 8.0).abs() < 1e-3);
    assert!((res.total_area - 4.0).abs() < 1e-3);

    // Invariante VSP-005: SyntheticCap gerada para visualização DEVE ter is_selectable == false
    assert!(res.synthetic_cap.is_some());
    let cap = res.synthetic_cap.unwrap();
    assert!(
        !cap.is_selectable,
        "Synthetic cap must NEVER be selectable!"
    );
    assert!(!cap.triangles.is_empty());
}

#[test]
fn test_open_mesh_produces_open_contour() {
    let mesh = create_open_quad_mesh();
    // Corta o plano XY em X = 0 (plano YZ)
    let plane = SectionPlane::yz(0.0);
    let asset_id = Uuid::new_v4();

    let res = SectionInspector::intersect_mesh_plane(asset_id, &mesh, None, &plane);

    // Como o quad é uma superfície aberta, o corte é um segmento aberto de Y=-1 a Y=+1
    assert_eq!(res.open_contours_count, 1);
    assert_eq!(res.closed_contours_count, 0);
    assert_eq!(res.contours[0].kind, LoopKind::Open);
    assert!(res.contours[0].area.is_none());
    assert!((res.total_perimeter - 2.0).abs() < 1e-3);
    // Malha aberta sem loops fechados não gera synthetic cap
    assert!(res.synthetic_cap.is_none());
}

#[test]
fn test_nested_contours_hole_detection() {
    let mesh = create_nested_cubes_mesh();
    let plane = SectionPlane::xz(0.0);
    let asset_id = Uuid::new_v4();

    let res = SectionInspector::intersect_mesh_plane(asset_id, &mesh, None, &plane);

    assert_eq!(res.closed_contours_count, 2);
    assert_eq!(res.open_contours_count, 0);
    // Deve detectar exatamente 1 furo (o cubo interno dentro do externo)
    assert_eq!(res.holes_count, 1);

    let hole_contour = res.contours.iter().find(|c| c.is_hole);
    let outer_contour = res.contours.iter().find(|c| !c.is_hole);

    assert!(
        hole_contour.is_some(),
        "Inner contour must be detected as hole"
    );
    assert!(outer_contour.is_some(), "Outer contour must not be hole");

    let hole_area = hole_contour.unwrap().area.unwrap();
    let outer_area = outer_contour.unwrap().area.unwrap();

    // Área do cubo interno 2x2 = 4.0; externo 4x4 = 16.0
    assert!((hole_area - 4.0).abs() < 1e-3);
    assert!((outer_area - 16.0).abs() < 1e-3);
}

#[test]
fn test_section_plane_anchors_and_normal_flip() {
    let mut plane =
        SectionPlane::new(SectionAnchor::World, [0.0, 1.0, 0.0], [0.0, 1.0, 0.0]).unwrap();

    let transform = ObjectTransform {
        translation: [10.0, 5.0, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: [1.0, 1.0, 1.0],
    };

    // 1. World Anchor
    let (p_world, n_world) = plane.effective_world_plane(Some(&transform));
    assert_eq!(p_world, Vec3::new(0.0, 1.0, 0.0));
    assert_eq!(n_world, Vec3::new(0.0, 1.0, 0.0));

    // 2. Object Anchor
    plane.anchor = SectionAnchor::Object;
    let (p_obj, n_obj) = plane.effective_world_plane(Some(&transform));
    // Origem local [0, 1, 0] no objeto em [10, 5, 0] vira [10, 6, 0]
    assert_eq!(p_obj, Vec3::new(10.0, 6.0, 0.0));
    assert_eq!(n_obj, Vec3::new(0.0, 1.0, 0.0));

    // 3. Normal Flip
    plane.flip();
    assert!(plane.flipped);
    let (p_flipped, n_flipped) = plane.effective_world_plane(Some(&transform));
    assert_eq!(n_flipped, Vec3::new(0.0, -1.0, 0.0));
    assert_eq!(p_flipped, Vec3::new(10.0, 6.0, 0.0));

    // 4. Offset numérico
    plane.offset = 2.5;
    let (p_offset, _) = plane.effective_world_plane(Some(&transform));
    // p_obj + (-normal * 2.5) = [10, 6 - 2.5, 0] = [10, 3.5, 0]
    assert_eq!(p_offset, Vec3::new(10.0, 3.5, 0.0));
}

#[test]
fn test_thickness_probe_parallel_wall() {
    let mesh = create_test_cube(); // Cubo de -1 a +1 (espessura 2.0 ao longo dos eixos)
    let asset_id = Uuid::new_v4();

    // Sonda partindo da face X = -1.0 em direção a +X
    let query = ThicknessProbeQuery {
        asset_id,
        start_point: [-1.0, 0.0, 0.0],
        direction: Some([1.0, 0.0, 0.0]),
        max_distance: 10.0,
    };

    let res = SectionInspector::probe_thickness(asset_id, &mesh, None, &query);

    assert_eq!(res.wall_type, WallType::ParallelWall);
    assert_eq!(res.quality, MeasurementQuality::Exact);
    assert!(res.thickness.is_some());
    assert!((res.thickness.unwrap() - 2.0).abs() < 1e-4);

    let exit = res.exit_point.unwrap();
    assert!((exit[0] - 1.0).abs() < 1e-4);
    assert!((exit[1] - 0.0).abs() < 1e-4);
    assert!((exit[2] - 0.0).abs() < 1e-4);
}

#[test]
fn test_thickness_probe_no_opposing_wall() {
    let mesh = create_test_cube();
    let asset_id = Uuid::new_v4();

    // Raio apontando para longe do cubo (-X a partir de -1.0)
    let query = ThicknessProbeQuery {
        asset_id,
        start_point: [-1.0, 0.0, 0.0],
        direction: Some([-1.0, 0.0, 0.0]),
        max_distance: 10.0,
    };

    let res = SectionInspector::probe_thickness(asset_id, &mesh, None, &query);

    assert_eq!(res.wall_type, WallType::NoOpposingWall);
    assert_eq!(res.quality, MeasurementQuality::Unavailable);
    assert!(res.thickness.is_none());
    assert!(res.exit_point.is_none());
}

#[test]
fn test_organic_centerline_parallel_transport_zero_flips() {
    // Curva 3D com formato de S e ponto de inflexão
    let points = [
        [0.0, 0.0, 0.0],
        [1.0, 1.0, 0.2],
        [2.0, 2.0, 0.0],
        [3.0, 1.0, -0.2],
        [4.0, 0.0, 0.0],
        [5.0, -1.0, 0.2],
        [6.0, 0.0, 0.0],
    ];

    let centerline = SectionInspector::build_organic_centerline(&points, &[]).unwrap();

    assert_eq!(centerline.trunk_stations.len(), points.len());

    // Verifica que em todas as estações:
    // 1. Tangente, Normal e Binormal são ortonormais (|t|=1, |n|=1, |b|=1, t.n=0, t.b=0, n.b=0)
    // 2. Transição da normal entre estações consecutivas não tem inversão abrupta (|n_i . n_{i+1}| > 0.8)
    for i in 0..centerline.trunk_stations.len() {
        let st = &centerline.trunk_stations[i];
        let t = Vec3::from(st.tangent);
        let n = Vec3::from(st.normal);
        let b = Vec3::from(st.binormal);

        assert!(
            (t.length() - 1.0).abs() < 1e-4,
            "Tangent must be unit vector at station {i}"
        );
        assert!(
            (n.length() - 1.0).abs() < 1e-4,
            "Normal must be unit vector at station {i}"
        );
        assert!(
            (b.length() - 1.0).abs() < 1e-4,
            "Binormal must be unit vector at station {i}"
        );

        assert!(
            t.dot(n).abs() < 1e-4,
            "Tangent and Normal must be orthogonal at station {i}"
        );
        assert!(
            t.dot(b).abs() < 1e-4,
            "Tangent and Binormal must be orthogonal at station {i}"
        );
        assert!(
            n.dot(b).abs() < 1e-4,
            "Normal and Binormal must be orthogonal at station {i}"
        );

        if i > 0 {
            let prev_st = &centerline.trunk_stations[i - 1];
            let prev_n = Vec3::from(prev_st.normal);
            let dot_norm = n.dot(prev_n);
            // Invariante VSP-005: Parallel Transport Frame (Bishop Frame) garante zero flips de normal
            assert!(
                dot_norm > 0.7,
                "Normal inverted or flipped between station {} and {}: dot = {}",
                i - 1,
                i,
                dot_norm
            );
        }
    }
}

#[test]
fn test_organic_centerline_branches() {
    let trunk_pts = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [3.0, 0.0, 0.0],
    ];

    // Ramo saindo da estação 2
    let branch_pts = [[2.0, 0.0, 0.0], [2.0, 1.0, 0.5], [2.0, 2.0, 1.0]];

    let branches = vec![(2usize, branch_pts.to_vec())];
    let centerline = SectionInspector::build_organic_centerline(&trunk_pts, &branches).unwrap();

    assert_eq!(centerline.branches.len(), 1);
    let branch = &centerline.branches[0];
    assert_eq!(branch.parent_station_index, 2);
    assert_eq!(branch.stations.len(), 3);

    // Estações do ramo mantêm ortonormalidade
    for st in &branch.stations {
        let t = Vec3::from(st.tangent);
        let n = Vec3::from(st.normal);
        let b = Vec3::from(st.binormal);
        assert!((t.length() - 1.0).abs() < 1e-4);
        assert!((n.length() - 1.0).abs() < 1e-4);
        assert!((b.length() - 1.0).abs() < 1e-4);
    }
}

#[test]
fn test_section_plane_commands_dispatch() {
    let mut app = AppState::new("en");

    assert!(app.active_section_plane().is_none());

    // 1. SetSectionPlaneCmd
    let plane = SectionPlane::xz(1.5).with_offset(0.2);
    let set_cmd = SetSectionPlaneCmd::new(plane.clone());
    petunia_core::command::CommandDispatcher::dispatch(&mut app, &set_cmd)
        .expect("Dispatch set_section_plane should succeed");

    assert!(app.active_section_plane().is_some());
    let active = app.active_section_plane().unwrap();
    assert_eq!(active.origin, [0.0, 1.5, 0.0]);
    assert_eq!(active.offset, 0.2);
    assert!(!active.flipped);

    // 2. OffsetSectionPlaneCmd
    let offset_cmd = OffsetSectionPlaneCmd::new(0.3);
    petunia_core::command::CommandDispatcher::dispatch(&mut app, &offset_cmd)
        .expect("Dispatch offset_section_plane should succeed");
    assert!((app.active_section_plane().unwrap().offset - 0.5).abs() < 1e-5);

    // 3. FlipSectionPlaneCmd
    petunia_core::command::CommandDispatcher::dispatch(&mut app, &FlipSectionPlaneCmd)
        .expect("Dispatch flip_section_plane should succeed");
    assert!(app.active_section_plane().unwrap().flipped);

    // 4. ClearSectionPlaneCmd
    petunia_core::command::CommandDispatcher::dispatch(&mut app, &ClearSectionPlaneCmd)
        .expect("Dispatch clear_section_plane should succeed");
    assert!(app.active_section_plane().is_none());

    // Invariante: Comandos de inspeção visual nunca marcam o documento como sujo (não destrutivos)
    assert!(!app.is_document_dirty());
}
