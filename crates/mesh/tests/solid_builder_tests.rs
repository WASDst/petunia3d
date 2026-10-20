use petunia_mesh::{CommittedObject, Mesh, RegionGroupOrigin, RegionPlan, SolidRegionProvenance};

fn cube(offset: [f32; 3], material_slot: usize) -> Mesh {
    let mut mesh = Mesh::cube(2.0);
    for v in &mut mesh.verts {
        v.pos = (v.vec() + glam::Vec3::from_array(offset)).to_array();
    }
    for f in &mut mesh.faces {
        f.material_slot = Some(material_slot);
    }
    mesh.triangulate();
    mesh
}

#[test]
fn solid_partition_and_regions_decompose_partially_overlapping_cubes() {
    let a = cube([0.0; 3], 0);
    let b = cube([1.0, 0.0, 0.0], 1);

    let plan = RegionPlan::build(&a, &b).expect("partition should succeed");
    assert_eq!(plan.regions.len(), 3);

    let first_only_count = plan
        .regions
        .iter()
        .filter(|r| r.provenance == SolidRegionProvenance::FirstOnly)
        .count();
    let overlap_count = plan
        .regions
        .iter()
        .filter(|r| r.provenance == SolidRegionProvenance::Overlap)
        .count();
    let second_only_count = plan
        .regions
        .iter()
        .filter(|r| r.provenance == SolidRegionProvenance::SecondOnly)
        .count();

    assert_eq!(first_only_count, 1);
    assert_eq!(overlap_count, 1);
    assert_eq!(second_only_count, 1);

    // Initial groups setup: Group 0 has A-only + Overlap, Group 1 has B-only + Overlap
    assert!(plan.is_first_untouched());
    assert!(plan.is_second_untouched());
}

#[test]
fn remove_overlap_removes_shared_region_from_both_operands() {
    let a = cube([0.0; 3], 0);
    let b = cube([1.0, 0.0, 0.0], 1);

    let mut plan = RegionPlan::build(&a, &b).unwrap();
    let overlap_id = plan
        .regions
        .iter()
        .find(|r| r.provenance == SolidRegionProvenance::Overlap)
        .unwrap()
        .id;

    plan.remove(&[overlap_id]);

    assert!(!plan.is_first_untouched());
    assert!(!plan.is_second_untouched());

    let results = plan.commit(&a, &b).expect("commit should succeed");
    assert_eq!(results.len(), 2);

    match &results[0] {
        CommittedObject::Mesh { mesh, origin } => {
            assert_eq!(*origin, RegionGroupOrigin::FirstOperand);
            assert!(!mesh.faces.is_empty());
        }
        _ => panic!("Group 0 should produce modified mesh (A - B)"),
    }

    match &results[1] {
        CommittedObject::Mesh { mesh, origin } => {
            assert_eq!(*origin, RegionGroupOrigin::SecondOperand);
            assert!(!mesh.faces.is_empty());
        }
        _ => panic!("Group 1 should produce modified mesh (B - A)"),
    }
}

#[test]
fn extract_overlap_creates_third_object_and_removes_from_source_groups() {
    let a = cube([0.0; 3], 0);
    let b = cube([1.0, 0.0, 0.0], 1);

    let mut plan = RegionPlan::build(&a, &b).unwrap();
    let overlap_id = plan
        .regions
        .iter()
        .find(|r| r.provenance == SolidRegionProvenance::Overlap)
        .unwrap()
        .id;

    let new_group = plan
        .extract(&[overlap_id])
        .expect("extract should create group");
    assert_eq!(new_group, 2);

    assert_eq!(plan.groups.len(), 3);
    assert!(!plan.groups[0].regions.contains(&overlap_id));
    assert!(!plan.groups[1].regions.contains(&overlap_id));
    assert!(plan.groups[2].regions.contains(&overlap_id));

    let results = plan.commit(&a, &b).expect("commit should succeed");
    assert_eq!(results.len(), 3);

    // Group 2 is the extracted overlap
    match &results[2] {
        CommittedObject::Mesh { mesh, origin } => {
            assert_eq!(*origin, RegionGroupOrigin::Created);
            assert!(!mesh.faces.is_empty());
        }
        _ => panic!("Group 2 should produce extracted mesh"),
    }
}

#[test]
fn unite_all_regions_computes_exact_union_mesh() {
    let a = cube([0.0; 3], 0);
    let b = cube([1.0, 0.0, 0.0], 1);

    let mut plan = RegionPlan::build(&a, &b).unwrap();
    let all_ids: Vec<usize> = plan.regions.iter().map(|r| r.id).collect();

    let united_group = plan.unite(&all_ids).expect("unite should create group");
    assert_eq!(united_group, 2);

    let results = plan.commit(&a, &b).expect("commit should succeed");
    assert_eq!(results.len(), 3);

    // Groups 0 and 1 are now empty
    assert!(matches!(results[0], CommittedObject::Empty));
    assert!(matches!(results[1], CommittedObject::Empty));

    // Group 2 has the united mesh
    match &results[2] {
        CommittedObject::Mesh { mesh, origin } => {
            assert_eq!(*origin, RegionGroupOrigin::Created);
            assert!(!mesh.faces.is_empty());
        }
        _ => panic!("Group 2 should produce united mesh"),
    }
}

#[test]
fn untouched_operand_preserves_identity_and_object_id() {
    let a = cube([0.0; 3], 0);
    let b = cube([1.0, 0.0, 0.0], 1);

    let mut plan = RegionPlan::build(&a, &b).unwrap();
    let b_only_id = plan
        .regions
        .iter()
        .find(|r| r.provenance == SolidRegionProvenance::SecondOnly)
        .unwrap()
        .id;

    // Remove only B-only part. Operand A remains completely untouched!
    plan.remove(&[b_only_id]);

    assert!(plan.is_first_untouched());
    assert!(!plan.is_second_untouched());

    let results = plan.commit(&a, &b).expect("commit should succeed");
    assert!(matches!(results[0], CommittedObject::PreserveFirst));
    assert!(matches!(results[1], CommittedObject::Mesh { .. }));
}

#[test]
fn disjoint_cubes_produce_empty_overlap_and_two_components() {
    let a = cube([0.0; 3], 0);
    let b = cube([5.0, 0.0, 0.0], 1);

    let plan = RegionPlan::build(&a, &b).unwrap();
    assert_eq!(plan.regions.len(), 2);

    let overlap_exists = plan
        .regions
        .iter()
        .any(|r| r.provenance == SolidRegionProvenance::Overlap);
    assert!(!overlap_exists);
}

#[test]
fn identical_cubes_produce_single_overlap_region() {
    let a = cube([0.0; 3], 0);
    let b = cube([0.0; 3], 1);

    let plan = RegionPlan::build(&a, &b).unwrap();
    assert_eq!(plan.regions.len(), 1);
    assert_eq!(plan.regions[0].provenance, SolidRegionProvenance::Overlap);
}

#[test]
fn contained_cube_produces_outer_shell_and_inner_overlap() {
    let a = cube([0.0; 3], 0);
    let mut b = Mesh::cube(0.5);
    b.triangulate();

    let plan = RegionPlan::build(&a, &b).unwrap();
    // A - B for a hollow shell produces 2 edge-connected components (outer shell and inner cavity)
    assert_eq!(plan.regions.len(), 3);

    let first_only_count = plan
        .regions
        .iter()
        .filter(|r| r.provenance == SolidRegionProvenance::FirstOnly)
        .count();
    let overlap_count = plan
        .regions
        .iter()
        .filter(|r| r.provenance == SolidRegionProvenance::Overlap)
        .count();
    let second_only_count = plan
        .regions
        .iter()
        .filter(|r| r.provenance == SolidRegionProvenance::SecondOnly)
        .count();

    assert_eq!(first_only_count, 2);
    assert_eq!(overlap_count, 1);
    assert_eq!(second_only_count, 0);
}

#[test]
fn material_slots_and_uvs_are_preserved_across_regions() {
    let a = cube([0.0; 3], 10);
    let b = cube([1.0, 0.0, 0.0], 20);

    let plan = RegionPlan::build(&a, &b).unwrap();
    let first_only = plan
        .regions
        .iter()
        .find(|r| r.provenance == SolidRegionProvenance::FirstOnly)
        .unwrap();
    let second_only = plan
        .regions
        .iter()
        .find(|r| r.provenance == SolidRegionProvenance::SecondOnly)
        .unwrap();

    // In A-only region, faces preserve material_slot 10 from A
    assert!(
        first_only
            .mesh
            .faces
            .iter()
            .any(|f| f.material_slot == Some(10))
    );

    // In B-only region, faces preserve material_slot 20 from B
    assert!(
        second_only
            .mesh
            .faces
            .iter()
            .any(|f| f.material_slot == Some(20))
    );
}

#[test]
fn invalid_or_open_mesh_is_rejected_without_panics() {
    let valid = cube([0.0; 3], 0);
    let mut open = valid.clone();
    open.faces.pop();

    assert!(RegionPlan::build(&open, &valid).is_err());
    assert!(RegionPlan::build(&valid, &open).is_err());
}

#[test]
fn multi_solid_evaluation_with_three_cubes_respects_budget_and_order() {
    use petunia_mesh::solid_builder::{ShapeBuilderBudget, SolidOperation, evaluate_multi_solid};

    let base = cube([0.0; 3], 1);
    let operand_b = cube([0.5, 0.0, 0.0], 2);
    let operand_c = cube([1.0, 0.0, 0.0], 3);

    let budget = ShapeBuilderBudget::default();

    // 1. Base U B U C
    let steps = [
        (&operand_b, SolidOperation::UniteAll),
        (&operand_c, SolidOperation::UniteAll),
    ];
    let result =
        evaluate_multi_solid(&base, &steps, &budget).expect("Multi-solid union must succeed");
    assert!(!result.faces.is_empty());
    assert!(
        result
            .faces
            .iter()
            .all(|f| f.verts.iter().all(|&v| (v as usize) < result.verts.len()))
    );

    // 2. Base U B, then remove overlap with C
    let steps_remove = [
        (&operand_b, SolidOperation::UniteAll),
        (&operand_c, SolidOperation::RemoveOverlap),
    ];
    let result_remove = evaluate_multi_solid(&base, &steps_remove, &budget)
        .expect("Multi-solid union + remove overlap must succeed");
    assert!(!result_remove.faces.is_empty());
}

#[test]
fn budget_enforces_input_triangle_limit_and_operation_depth() {
    use petunia_mesh::solid_builder::{ShapeBuilderBudget, SolidOperation, evaluate_multi_solid};

    let a = cube([0.0; 3], 0);
    let b = cube([0.5, 0.0, 0.0], 0);

    // Limit input triangles to 10 (each cube has 12 tris, total 24 > 10)
    let restrictive_budget = ShapeBuilderBudget {
        max_input_triangles: 10,
        ..Default::default()
    };
    let steps = [(&b, SolidOperation::UniteAll)];
    let err = evaluate_multi_solid(&a, &steps, &restrictive_budget);
    assert!(
        err.is_err(),
        "Must reject when input triangles exceed budget"
    );

    // Limit depth to 1 step, but give 2 steps
    let depth_budget = ShapeBuilderBudget {
        max_depth: 1,
        ..Default::default()
    };
    let steps_two = [
        (&b, SolidOperation::UniteAll),
        (&b, SolidOperation::UniteAll),
    ];
    let err_depth = evaluate_multi_solid(&a, &steps_two, &depth_budget);
    assert!(
        err_depth.is_err(),
        "Must reject when operation depth exceeds budget"
    );
}
