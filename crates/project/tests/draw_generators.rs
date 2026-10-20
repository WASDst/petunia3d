use petunia_mesh::Mesh;
use petunia_project::{
    Asset, GeometryGenerator, GeometryGeneratorKind, PlanarShape, Project, RevolutionAxis,
    SplineResource, Workplane,
};

fn shape_fixture(holes: bool) -> (Project, uuid::Uuid) {
    let mut project = Project::new();
    project.assets.clear();
    let outer = SplineResource::from_polyline(
        "outer",
        &[[-2., -2., 0.], [2., -2., 0.], [2., 2., 0.], [-2., 2., 0.]],
        true,
    );
    let mut shape = PlanarShape::new("frame", outer.id, Workplane::default());
    if holes {
        let hole = SplineResource::from_polyline(
            "inner",
            &[[-1., -1., 0.], [1., -1., 0.], [1., 1., 0.], [-1., 1., 0.]],
            true,
        );
        shape.hole_splines.push(hole.id);
        project.add_spline(hole).unwrap();
    }
    project.add_spline(outer).unwrap();
    let id = shape.id;
    project.add_profile(shape).unwrap();
    (project, id)
}
fn extrude(id: uuid::Uuid, distance: f32) -> GeometryGenerator {
    GeometryGenerator::new(GeometryGeneratorKind::Extrude {
        shape_object: id,
        distance,
        cap_start: true,
        cap_end: true,
    })
}
#[test]
fn extrusion_preserves_holes_signed_height_and_rejects_non_finite() {
    let (project, id) = shape_fixture(true);
    let mesh = extrude(id, -3.).evaluate(&project).unwrap();
    assert!(
        mesh.verts
            .iter()
            .all(|v| v.pos[2] >= -3.001 && v.pos[2] <= 0.001)
    );
    assert!(
        mesh.verts
            .iter()
            .any(|v| (v.pos[0].abs() - 1.).abs() < 1e-4)
    );
    mesh.validate_invariants().unwrap();
    assert!(extrude(id, f32::NAN).evaluate(&project).is_err());
    assert!(extrude(id, 0.).evaluate(&project).is_err());
    let mut open = project.clone();
    let outer = open.get_profile(id).unwrap().spline_id;
    open.get_spline_mut(outer).unwrap().closed = false;
    assert!(extrude(id, 2.).evaluate(&open).is_err());
}
#[test]
fn source_and_contours_survive_zip_without_authoring_cache() {
    let (mut project, id) = shape_fixture(true);
    let mut asset = Asset::new("generated", Mesh::default());
    asset.geometry_source = Some(extrude(id, 2.));
    let object = asset.id;
    project.assets.push(asset);
    assert!(project.refresh_generated_geometry());
    assert!(!project.assets[0].source_mesh().verts.is_empty());
    assert!(project.assets[0].mesh.verts.is_empty());
    let json = serde_json::to_value(&project).unwrap();
    assert!(json["assets"][0].get("generated_mesh").is_none());
    let encoded = petunia_project::format::encode_zip(&project).unwrap();
    let mut loaded = petunia_project::format::load_bytes(&encoded).unwrap();
    assert_eq!(loaded.assets[0].id, object);
    assert_eq!(loaded.get_profile(id).unwrap().hole_splines.len(), 1);
    assert!(!loaded.assets[0].source_mesh().verts.is_empty());
    let outer = loaded.get_profile(id).unwrap().spline_id;
    let spline = loaded.get_spline_mut(outer).unwrap();
    spline.points[0].position[0] -= 1.;
    spline.revision += 1;
    assert!(loaded.refresh_generated_geometry());
    let duplicate = loaded.assets[0].duplicate();
    assert_ne!(
        duplicate.geometry_source.as_ref().unwrap().id,
        loaded.assets[0].geometry_source.as_ref().unwrap().id
    );
    loaded.profiles.clear();
    assert!(loaded.refresh_generated_geometry());
    assert!(loaded.assets[0].source_mesh().verts.is_empty());
    assert!(loaded.assets[0].generator_diagnostic.is_some());
}
#[test]
fn sweep_uses_explicit_path_and_saved_workplane_including_holes() {
    let (mut project, id) = shape_fixture(true);
    let path =
        SplineResource::from_polyline("path", &[[0., 0., 0.], [0., 4., 0.], [0., 8., 0.]], false);
    let path_id = path.id;
    let mut path_shape = PlanarShape::new("path plane", path_id, Workplane::default());
    path_shape.workplane.origin = [20., 0., 0.];
    project.add_spline(path).unwrap();
    project.add_profile(path_shape).unwrap();
    let generator = GeometryGenerator::new(GeometryGeneratorKind::Sweep {
        shape_object: id,
        path_object: path_id,
        cap_start: true,
        cap_end: true,
    });
    let mesh = generator.evaluate(&project).unwrap();
    assert!(mesh.verts.iter().all(|v| v.pos[0] > 15.));
    mesh.validate_invariants().unwrap();
    let bad = GeometryGenerator::new(GeometryGeneratorKind::Sweep {
        shape_object: id,
        path_object: uuid::Uuid::new_v4(),
        cap_start: true,
        cap_end: true,
    });
    assert!(bad.evaluate(&project).is_err());
}
#[test]
fn revolve_accepts_open_curve_and_explicit_axis() {
    let mut project = Project::new();
    let spline =
        SplineResource::from_polyline("lathe", &[[1., 0., 0.], [2., 1., 0.], [1., 2., 0.]], false);
    let shape = PlanarShape::new("lathe", spline.id, Workplane::default());
    let id = shape.id;
    project.add_spline(spline).unwrap();
    project.add_profile(shape).unwrap();
    for axis in [RevolutionAxis::Horizontal, RevolutionAxis::Vertical] {
        let generator = GeometryGenerator::new(GeometryGeneratorKind::Revolve {
            profile_object: id,
            axis,
            angle: 180.,
            segments: 16,
            cap_start: true,
            cap_end: true,
        });
        let mesh = generator.evaluate(&project).unwrap();
        assert!(!mesh.faces.is_empty());
        mesh.validate_invariants().unwrap();
    }
}

#[test]
fn unsupported_primitive_parameters_are_rejected_without_silent_clamps() {
    let project = Project::new();
    for descriptor in [
        petunia_mesh::PrimitiveDescriptor::Plane {
            width: 0.001,
            height: 1.,
        },
        petunia_mesh::PrimitiveDescriptor::Icosphere {
            radius: 1.,
            subdiv: 4,
        },
        petunia_mesh::PrimitiveDescriptor::LowSphere {
            radius: 1.,
            segments: 100,
            rings: 4,
        },
    ] {
        assert!(
            GeometryGenerator::new(GeometryGeneratorKind::Primitive(descriptor))
                .evaluate(&project)
                .is_err()
        );
    }
}

#[test]
fn invalid_contours_keep_sources_and_never_generate_a_repaired_solid() {
    let (mut project, id) = shape_fixture(true);
    let inner = project.get_profile(id).unwrap().hole_splines[0];
    for point in &mut project.get_spline_mut(inner).unwrap().points {
        point.position[0] += 10.;
    }
    assert!(extrude(id, 2.).evaluate(&project).is_err());
    let (mut project, id) = shape_fixture(false);
    let outer = project.get_profile(id).unwrap().spline_id;
    project.get_spline_mut(outer).unwrap().points.swap(1, 2);
    assert!(extrude(id, 2.).evaluate(&project).is_err());
}

#[test]
fn mixed_surface_bindings_export_distinct_materials_and_textures_without_mutating_source() {
    use petunia_project::{Canvas, Material, SurfaceMaterial};
    let mut project = Project::new();
    let red = Material::new("red");
    let blue = Material::new("blue");
    let mut asset = Asset::new("mixed", Mesh::cube(2.));
    asset.surface_materials = vec![
        SurfaceMaterial {
            material_id: Some(red.id),
            base_color: [1.; 3],
            texture: Some(Canvas::new(2, 2, [255, 0, 0, 255])),
        },
        SurfaceMaterial {
            material_id: Some(blue.id),
            base_color: [1.; 3],
            texture: Some(Canvas::new(2, 2, [0, 0, 255, 255])),
        },
    ];
    for (i, f) in asset.mesh.faces.iter_mut().enumerate() {
        f.material_slot = Some(i % 2);
    }
    project.materials = vec![red, blue];
    project.assets = vec![asset];
    let before = serde_json::to_vec(&project).unwrap();
    let bytes = petunia_project::export::export_gltf(&project, &[0]).unwrap();
    let document = gltf::Gltf::from_slice(&bytes).unwrap();
    assert_eq!(document.meshes().count(), 2);
    assert_eq!(document.images().count(), 2);
    assert_eq!(before, serde_json::to_vec(&project).unwrap());
}
