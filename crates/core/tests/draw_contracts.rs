use petunia_core::{AppState, Camera, WorkplaneMode};
use petunia_mesh::{
    Mesh,
    boolean::{SolidRegionClass, partition_solids},
};
use petunia_project::{PlanarShape, SplineResource, Workplane};
#[test]
fn viewport_session_keeps_plane_and_camera_independent() {
    let mut state = AppState::default();
    let first = state.session.viewport_session();
    let plane = Workplane {
        origin: [3., 4., 5.],
        right: [0., 1., 0.],
        up: [-1., 0., 0.],
        ..Default::default()
    };
    state
        .session
        .active_workplane
        .set(WorkplaneMode::Custom, plane)
        .unwrap();
    state.session.camera.align_to_workplane(plane);
    state.session.planar_saved_camera = Some(Camera::default());
    let second = state.session.viewport_session();
    state.session.set_viewport_session(first.clone());
    assert_ne!(state.session.active_workplane, second.active_workplane);
    assert_eq!(state.session.camera, first.camera);
    state.session.set_viewport_session(second.clone());
    assert_eq!(state.session.camera.right(), glam::Vec3::Y);
    assert_eq!(
        state.session.planar_saved_camera,
        second.planar_saved_camera
    );
}
#[test]
fn solid_partition_treats_empty_intersection_as_valid_and_preserves_components() {
    let a = Mesh::cube(2.);
    let mut b = Mesh::cube(2.);
    for vertex in &mut b.verts {
        vertex.pos[0] += 5.;
    }
    let partition = partition_solids(&a, &b).unwrap();
    assert_eq!(partition.regions.len(), 2);
    assert!(
        partition
            .regions
            .iter()
            .any(|r| r.class == SolidRegionClass::FirstOnly)
    );
    assert!(
        partition
            .regions
            .iter()
            .any(|r| r.class == SolidRegionClass::SecondOnly)
    );
    assert!(
        !partition
            .regions
            .iter()
            .any(|r| r.class == SolidRegionClass::Intersection)
    );
    assert!(partition_solids(&Mesh::plane(2.), &b).is_err());
}
#[test]
fn locked_shape_and_inner_contour_cannot_be_modified_or_opened() {
    let mut state = AppState::default();
    let outer =
        SplineResource::from_polyline("outer", &[[0., 0., 0.], [3., 0., 0.], [0., 3., 0.]], true);
    let inner = SplineResource::from_polyline(
        "inner",
        &[[0.2, 0.2, 0.], [0.4, 0.2, 0.], [0.2, 0.4, 0.]],
        true,
    );
    let mut shape = PlanarShape::new("shape", outer.id, Workplane::default());
    shape.hole_splines.push(inner.id);
    shape.locked = true;
    let id = shape.id;
    state.project.project.add_spline(outer.clone()).unwrap();
    state.project.project.add_spline(inner.clone()).unwrap();
    state.project.project.add_profile(shape).unwrap();
    let mut edited = outer;
    edited.points[0].position[0] = 1.;
    assert!(
        state
            .dispatch(&petunia_core::UpdateSplineCmd { spline: edited })
            .is_err()
    );
    assert!(
        state
            .dispatch(&petunia_core::DeleteProfileCmd { profile_id: id })
            .is_err()
    );
    state.project.project.get_profile_mut(id).unwrap().locked = false;
    let mut edited = inner;
    edited.closed = false;
    assert!(
        state
            .dispatch(&petunia_core::UpdateSplineCmd { spline: edited })
            .is_err()
    );
}

#[test]
fn primitive_source_is_authoritative_and_conversion_is_explicit() {
    let mut state = AppState::default();
    let count = state.project.assets.len();
    assert!(state.begin_primitive(petunia_core::command::PrimitiveKind::Cube, None));
    assert_eq!(state.project.assets.len(), count + 1);
    assert!(state.project.active().unwrap().geometry_source.is_some());
    assert!(state.project.active().unwrap().parametric.is_none());
    assert!(state.project.active().unwrap().mesh.verts.is_empty());
    assert!(
        !state
            .project
            .active()
            .unwrap()
            .source_mesh()
            .verts
            .is_empty()
    );
    assert!(state.confirm_primitive());
    let id = state.project.active().unwrap().id;
    assert!(state.freeze_active_primitive());
    assert_eq!(state.project.active().unwrap().id, id);
    assert!(state.project.active().unwrap().geometry_source.is_none());
    assert!(!state.project.active().unwrap().mesh.verts.is_empty());
    assert!(state.undo());
    assert!(state.project.active().unwrap().geometry_source.is_some());
}

#[test]
fn smart_grid_and_increment_are_distinct_and_mesh_snap_has_provenance() {
    use petunia_core::inference::{
        ScreenSnapQuery, SnapAccel, SnapMask, SnapReference, SnapSource, snap_screen_with,
    };
    assert!(SnapMask::for_target(petunia_core::SnapTarget::Smart).points);
    let grid = SnapMask::for_target(petunia_core::SnapTarget::Grid);
    assert!(grid.grid);
    assert!(!grid.points && !grid.edges && !grid.faces);
    assert!(!SnapMask::for_target(petunia_core::SnapTarget::Increment).grid);
    let mut mesh = Mesh::default();
    mesh.verts.push(petunia_mesh::Vertex::new(0., 0., 0.));
    let id = uuid::Uuid::new_v4();
    let accel = SnapAccel::build_scoped(
        &[SnapSource {
            mesh: &mesh,
            moving: &[],
        }],
        &[(id, true)],
    );
    let mut camera = Camera::default();
    camera.set_preset(petunia_core::ViewPreset::Front);
    camera.aspect = 1.;
    let query = ScreenSnapQuery {
        camera: &camera,
        viewport_pixels: glam::Vec2::splat(800.),
        cursor_pixels: glam::Vec2::splat(400.),
        mesh: None,
        moving: &[],
        anchor: None,
        grid: None,
        radius_pixels: 12.,
        mask: SnapMask::ALL,
        xray: false,
        reference: None,
        angle_step_degrees: Some(15.),
    };
    let hit = snap_screen_with(&query, &accel).unwrap();
    assert_eq!(
        hit.reference,
        Some(SnapReference::MeshVertex {
            object: id,
            vertex: 0,
            stable: true
        })
    );
}

#[test]
fn solid_region_commit_preserves_untouched_parametric_operand_and_undo() {
    use petunia_core::{CommitSolidShapeBuilderCmd, SolidShapeBuilderSession, SolidShapeIntent};
    use petunia_project::{Asset, GeometryGenerator, GeometryGeneratorKind};
    let mut state = AppState::default();
    let first = state.project.assets[0].id;
    let mut second = Asset::new("source", Mesh::default());
    second.geometry_source = Some(GeometryGenerator::new(GeometryGeneratorKind::Primitive(
        petunia_mesh::PrimitiveDescriptor::Box {
            width: 2.,
            height: 2.,
            depth: 2.,
        },
    )));
    second.transform.translation = [8., 0., 0.];
    let second_id = second.id;
    state.project.project.assets.push(second);
    state.project.project.refresh_generated_geometry();
    let source = state.project.project.assets[1].geometry_source.clone();
    let mut session =
        SolidShapeBuilderSession::begin(&state.project.project, first, second_id).unwrap();
    session.intent = SolidShapeIntent::Remove;
    let index = session
        .regions
        .iter()
        .position(|r| r.class == SolidRegionClass::FirstOnly)
        .unwrap();
    session.selected.insert(index);
    state
        .dispatch(&CommitSolidShapeBuilderCmd { session })
        .unwrap();
    assert_eq!(state.project.assets.len(), 1);
    assert_eq!(state.project.assets[0].id, second_id);
    assert_eq!(state.project.assets[0].geometry_source, source);
    assert!(state.undo());
    assert_eq!(state.project.assets.len(), 2);
    assert_eq!(state.project.assets[1].geometry_source, source);
}

#[test]
fn revolve_closed_profile_and_overlapping_solid_partition_are_closed() {
    let revolved = Mesh::revolve_profile(
        &[[1., -1.], [2., -1.], [2., 1.], [1., 1.]],
        24,
        360.,
        true,
        [true, true],
    )
    .unwrap();
    let mut far = Mesh::cube(1.);
    for vertex in &mut far.verts {
        vertex.pos[0] += 10.;
    }
    assert_eq!(partition_solids(&revolved, &far).unwrap().regions.len(), 2);
    let a = Mesh::cube(2.);
    let mut b = a.clone();
    for vertex in &mut b.verts {
        vertex.pos[0] += 1.;
    }
    let partition = partition_solids(&a, &b).unwrap();
    assert_eq!(partition.regions.len(), 3);
    assert!(
        partition
            .regions
            .iter()
            .any(|r| r.class == SolidRegionClass::Intersection)
    );
}

#[test]
fn spatial_curve_snap_keeps_depth_and_point_identity() {
    use petunia_core::inference::{
        ScreenSnapQuery, SnapAccel, SnapMask, SnapReference, snap_screen_with,
    };
    let spline = SplineResource::from_polyline("3D path", &[[0., 0., 2.], [1., 0., 2.]], false);
    let mut accel = SnapAccel::default();
    accel.append_spatial_spline(spline.id, &spline);
    let mut camera = Camera::default();
    camera.set_preset(petunia_core::ViewPreset::Front);
    camera.aspect = 1.;
    let query = ScreenSnapQuery {
        camera: &camera,
        viewport_pixels: glam::Vec2::splat(800.),
        cursor_pixels: glam::Vec2::splat(400.),
        mesh: None,
        moving: &[],
        anchor: None,
        grid: None,
        radius_pixels: 12.,
        mask: SnapMask::ALL,
        xray: false,
        reference: None,
        angle_step_degrees: None,
    };
    let hit = snap_screen_with(&query, &accel).unwrap();
    assert_eq!(hit.point.z, 2.);
    assert_eq!(
        hit.reference,
        Some(SnapReference::SplinePoint {
            object: spline.id,
            spline: spline.id,
            point: spline.points[0].id
        })
    );
}

#[test]
fn bezier_snap_refines_in_screen_pixels_at_extreme_zoom() {
    use petunia_core::inference::{
        ScreenSnapQuery, SnapAccel, SnapMask, SnapReference, snap_screen_with,
    };
    let mut spline = SplineResource::from_polyline("Bezier", &[[0., 0., 0.], [1., 0., 0.]], false);
    spline.interpolation = petunia_project::SplineInterpolation::CubicBezier;
    spline.points[0].handle_out = [0., 1., 0.];
    spline.points[1].handle_in = [0., -1., 0.];
    let expected =
        glam::Vec3::from_array(spline.evaluate_segment(0, 0.25).unwrap().map(|v| v as f32));
    let mut accel = SnapAccel::default();
    accel.append_spatial_spline(spline.id, &spline);
    for half_height in [4.0, 0.1, 0.001] {
        let mut camera = Camera::default();
        camera.set_preset(petunia_core::ViewPreset::Front);
        camera.aspect = 1.;
        camera.target = expected;
        camera.ortho_half_h = half_height;
        let query = ScreenSnapQuery {
            camera: &camera,
            viewport_pixels: glam::Vec2::splat(800.),
            cursor_pixels: glam::Vec2::splat(400.),
            mesh: None,
            moving: &[],
            anchor: None,
            grid: None,
            radius_pixels: 12.,
            mask: SnapMask {
                points: false,
                edges: true,
                axes: false,
                faces: false,
                grid: false,
            },
            xray: false,
            reference: None,
            angle_step_degrees: None,
        };
        let hit = snap_screen_with(&query, &accel).unwrap();
        assert!(
            hit.pixel_distance < 0.5,
            "zoom {half_height}: {} pixels",
            hit.pixel_distance
        );
        assert!(hit.point.distance(expected) < 1.0e-5);
        assert!(
            matches!(hit.reference,Some(SnapReference::SplineSegment{parameter,..}) if (parameter-0.25).abs()<1.0e-4)
        );
    }
}

#[test]
fn solid_commit_keeps_source_materials_paint_uv_and_roundtrip_without_new_materials() {
    use petunia_core::{CommitSolidShapeBuilderCmd, SolidShapeBuilderSession};
    use petunia_project::{Asset, Canvas, Material};
    let mut state = AppState::default();
    state.project.project.assets.clear();
    let red = Material::new("red");
    let red_id = red.id;
    let blue = Material::new("blue");
    let blue_id = blue.id;
    state.project.project.materials = vec![red, blue];
    let mut a = Asset::new("A", Mesh::cube(2.));
    a.material_id = Some(red_id);
    a.texture = Some(Canvas::new(2, 2, [255, 0, 0, 255]));
    let mut b = Asset::new("B", Mesh::cube(2.));
    b.material_id = Some(blue_id);
    b.texture = Some(Canvas::new(2, 2, [0, 0, 255, 255]));
    b.transform.translation = [5., 0., 0.];
    let ids = [a.id, b.id];
    let uv_a = a.mesh.faces[0].uv.clone();
    state.project.project.assets = vec![a, b];
    let mut session =
        SolidShapeBuilderSession::begin(&state.project.project, ids[0], ids[1]).unwrap();
    session.selected.extend(0..session.regions.len());
    state
        .dispatch(&CommitSolidShapeBuilderCmd { session })
        .unwrap();
    assert_eq!(state.project.project.materials.len(), 2);
    assert_eq!(state.project.project.assets.len(), 1);
    let result = &state.project.project.assets[0];
    assert_eq!(result.surface_materials.len(), 2);
    assert_eq!(result.surface_materials[0].material_id, Some(red_id));
    assert_eq!(result.surface_materials[1].material_id, Some(blue_id));
    assert!(result.surface_materials.iter().all(|b| b.texture.is_some()));
    assert!(result.mesh.faces.iter().any(|f| f.material_slot == Some(0)));
    assert!(result.mesh.faces.iter().any(|f| f.material_slot == Some(1)));
    assert!(result.mesh.faces.iter().any(|f| f.uv == uv_a));
    let json = serde_json::to_vec(&state.project.project).unwrap();
    let restored: petunia_project::Project = serde_json::from_slice(&json).unwrap();
    assert_eq!(
        restored.assets[0].surface_materials,
        result.surface_materials
    );
    assert!(state.undo());
    assert_eq!(state.project.project.assets.len(), 2);
    assert!(state.redo());
    assert_eq!(state.project.project.assets[0].surface_materials.len(), 2);
}
