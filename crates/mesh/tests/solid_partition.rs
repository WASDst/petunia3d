use petunia_mesh::{
    Mesh,
    boolean::{BooleanOp, BooleanSolid, PreparedBooleanOperands, boolean_solids},
};

fn cube(offset: [f32; 3]) -> Mesh {
    let mut mesh = Mesh::cube(2.0);
    for v in &mut mesh.verts {
        v.pos = (v.vec() + glam::Vec3::from_array(offset)).to_array();
    }
    mesh.triangulate();
    mesh
}

#[test]
fn disjoint_intersection_and_identical_difference_are_successful_empty_solids() {
    assert!(matches!(
        boolean_solids(
            &cube([0.0; 3]),
            &cube([5.0, 0.0, 0.0]),
            BooleanOp::Intersection
        )
        .unwrap(),
        BooleanSolid::Empty
    ));
    assert!(matches!(
        boolean_solids(&cube([0.0; 3]), &cube([0.0; 3]), BooleanOp::Difference).unwrap(),
        BooleanSolid::Empty
    ));
}

#[test]
fn prepared_partition_handles_overlap_containment_and_disjoint_components() {
    for offset in [[0.0; 3], [1.0, 0.0, 0.0], [5.0, 0.0, 0.0]] {
        let operands = PreparedBooleanOperands::new(&cube([0.0; 3]), &cube(offset)).unwrap();
        let partition = operands.partition().unwrap();
        if offset == [0.0; 3] {
            assert!(matches!(partition.first_only, BooleanSolid::Empty));
            assert!(matches!(partition.second_only, BooleanSolid::Empty));
            assert_eq!(partition.overlap.connected_regions().unwrap().len(), 1);
        } else if offset[0] == 5.0 {
            assert!(matches!(partition.overlap, BooleanSolid::Empty));
            assert_eq!(
                operands
                    .evaluate(BooleanOp::Union)
                    .unwrap()
                    .connected_regions()
                    .unwrap()
                    .len(),
                2
            );
        } else {
            assert_eq!(partition.first_only.connected_regions().unwrap().len(), 1);
            assert_eq!(partition.overlap.connected_regions().unwrap().len(), 1);
            assert_eq!(partition.second_only.connected_regions().unwrap().len(), 1);
        }
    }
}

#[test]
fn non_manifold_and_non_finite_operands_are_rejected_before_partition() {
    let valid = cube([0.0; 3]);
    let mut open = valid.clone();
    open.faces.pop();
    assert!(PreparedBooleanOperands::new(&open, &valid).is_err());
    let mut corrupt = valid.clone();
    corrupt.verts[0].pos[0] = f32::INFINITY;
    assert!(PreparedBooleanOperands::new(&corrupt, &valid).is_err());
}
