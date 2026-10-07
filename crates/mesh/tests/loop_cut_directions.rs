use glam::Vec3;
use petunia_mesh::Mesh;
use petunia_mesh::boolean::{BooleanOp, boolean_meshes};
use petunia_mesh::loop_cut::LoopRing;

#[test]
fn boolean_results_keep_horizontal_and_vertical_section_directions() {
    for op in [
        BooleanOp::Union,
        BooleanOp::Difference,
        BooleanOp::Intersection,
    ] {
        let mut a = Mesh::cube(2.0);
        let mut b = Mesh::cube(1.5);
        for v in &mut b.verts {
            v.pos[0] += 0.8;
            v.pos[1] += 0.2;
        }
        a.triangulate();
        b.triangulate();
        let mut source = boolean_meshes(&a, &b, op).unwrap();
        for face in &mut source.faces {
            face.material_slot = Some(2);
        }
        let original = source.topology_fingerprint();
        for axis in [Vec3::X, Vec3::Y] {
            let seed = source
                .edges_unique()
                .into_iter()
                .find(|&(a, b)| {
                    let direction = (source.verts[b as usize].vec()
                        - source.verts[a as usize].vec())
                    .normalize_or_zero();
                    direction.dot(axis).abs() > 0.9999
                })
                .expect("axis-aligned boolean edge");
            let origin =
                (source.verts[seed.0 as usize].vec() + source.verts[seed.1 as usize].vec()) * 0.5;
            let ring = LoopRing::discover(&source, seed).unwrap();
            let preview = ring.preview(&source, 1, 0.0).unwrap();
            assert!(!preview.is_empty());
            for [a, b] in preview {
                assert!(
                    (a - origin).dot(axis).abs() < 1e-5 && (b - origin).dot(axis).abs() < 1e-5,
                    "{op:?}: section direction"
                );
            }
            let result = ring.apply(&source, 1, 0.0).unwrap();
            let topology = result.validate_topology();
            assert!(
                topology.is_closed && topology.is_manifold,
                "{op:?}: {topology:?}"
            );
            assert!(result.faces.iter().all(|face| face.material_slot == Some(2)
                && face.uv.iter().flatten().all(|v| v.is_finite())));
            assert!(!result.selected_edges.is_empty());
            assert_eq!(
                source.topology_fingerprint(),
                original,
                "source stays unchanged"
            );
        }
    }
}

#[test]
fn closed_primitives_support_both_edge_families_without_opening_the_surface() {
    for (name, source) in [
        ("cube", Mesh::cube(2.0)),
        ("cylinder", Mesh::cylinder(12, 1.0, 2.0)),
        ("cone", Mesh::cone(12, 1.0, 2.0)),
        ("sphere", Mesh::sphere_low(12, 6, 1.0)),
        ("torus", Mesh::torus(1.0, 0.3, 12, 8)),
    ] {
        for vertical in [false, true] {
            let seed = source
                .edges_unique()
                .into_iter()
                .find(|&(a, b)| {
                    let delta = source.verts[b as usize].vec() - source.verts[a as usize].vec();
                    if vertical {
                        delta.y.abs() > delta.x.abs().max(delta.z.abs()) + 1e-5
                    } else {
                        delta.y.abs() < 1e-5 && delta.length_squared() > 1e-6
                    }
                })
                .expect("primitive has both edge families");
            let ring = LoopRing::discover(&source, seed).unwrap();
            assert!(
                !ring.preview(&source, 1, 0.0).unwrap().is_empty(),
                "{name}: preview"
            );
            let result = ring.apply(&source, 1, 0.0).unwrap();
            let topology = result.validate_topology();
            assert!(
                topology.is_closed && topology.is_manifold,
                "{name}, vertical={vertical}: {topology:?}"
            );
            assert!(!result.selected_edges.is_empty());
            assert!(
                result
                    .faces
                    .iter()
                    .all(|face| face.verts.len() == face.uv.len()
                        && face.uv.iter().flatten().all(|v| v.is_finite()))
            );
        }
    }
}
