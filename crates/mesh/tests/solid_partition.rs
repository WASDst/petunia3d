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

#[test]
fn decompose_regions_and_region_plan_unite_remove_extract() {
    use petunia_mesh::boolean::{RegionPlan, SolidRegionClass, SurfaceOrigin};

    let mut a = cube([0.0; 3]);
    for f in &mut a.faces {
        f.material_slot = Some(1);
    }
    let mut b = cube([1.0, 0.0, 0.0]);
    for f in &mut b.faces {
        f.material_slot = Some(2);
    }

    let operands = PreparedBooleanOperands::new(&a, &b).unwrap();
    let regions = operands.decompose_regions(&a, &b).unwrap();
    assert_eq!(regions.len(), 3);

    // Verify region classes and provenance
    let first = regions
        .iter()
        .find(|r| r.class == SolidRegionClass::FirstOnly)
        .unwrap();
    let second = regions
        .iter()
        .find(|r| r.class == SolidRegionClass::SecondOnly)
        .unwrap();
    let overlap = regions
        .iter()
        .find(|r| r.class == SolidRegionClass::Overlap)
        .unwrap();

    assert!(
        first
            .provenance
            .iter()
            .any(|p| matches!(p, SurfaceOrigin::First(_)))
    );
    assert!(
        second
            .provenance
            .iter()
            .any(|p| matches!(p, SurfaceOrigin::Second(_)))
    );
    assert!(
        overlap
            .provenance
            .iter()
            .any(|p| matches!(p, SurfaceOrigin::First(_) | SurfaceOrigin::Second(_)))
    );

    // Verify material slot inheritance
    assert!(first.mesh.faces.iter().any(|f| f.material_slot == Some(1)));
    assert!(second.mesh.faces.iter().any(|f| f.material_slot == Some(2)));

    // 1. Initial plan: Group 0 owns {first, overlap}, Group 1 owns {second, overlap}
    let mut plan = RegionPlan::initial(&regions);
    assert_eq!(plan.groups.len(), 2);
    assert!(plan.groups[0].contains(&first.id) && plan.groups[0].contains(&overlap.id));
    assert!(plan.groups[1].contains(&second.id) && plan.groups[1].contains(&overlap.id));

    // 2. Remove overlap
    plan.remove(&[overlap.id]);
    assert!(!plan.groups[0].contains(&overlap.id));
    assert!(!plan.groups[1].contains(&overlap.id));
    let committed_remove = plan.commit(&regions).unwrap();
    assert_eq!(committed_remove.len(), 2);
    assert!(committed_remove[0].is_some(), "A-only survives");
    assert!(committed_remove[1].is_some(), "B-only survives");

    // 3. Extract overlap
    let mut plan_extract = RegionPlan::initial(&regions);
    let extracted_group = plan_extract.extract(&[overlap.id]);
    assert_eq!(extracted_group, 2);
    let committed_extract = plan_extract.commit(&regions).unwrap();
    assert_eq!(committed_extract.len(), 3);
    assert!(committed_extract[0].is_some(), "A-only survives");
    assert!(committed_extract[1].is_some(), "B-only survives");
    assert!(
        committed_extract[2].is_some(),
        "Overlap extracted to group 2"
    );

    // 4. Unite all into single output
    let mut plan_unite = RegionPlan::initial(&regions);
    plan_unite.unite_all(&regions);
    assert_eq!(plan_unite.groups.len(), 1);
    let committed_unite = plan_unite.commit(&regions).unwrap();
    assert_eq!(committed_unite.len(), 1);
    assert!(
        committed_unite[0].is_some(),
        "Unite all produces single mesh"
    );
}
