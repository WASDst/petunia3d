//! Testes unitários para o módulo de alinhamento e distribuição 3D de objetos.

use glam::Vec3;
use petunia_mesh::{Mesh, Vertex};
use petunia_project::{
    AlignAxis, AlignError, AlignMode, AlignReference, Asset, BoundingBox3D,
    compute_alignment_deltas, compute_distribution_deltas,
};
use uuid::Uuid;

fn make_box_asset(name: &str, min: Vec3, max: Vec3) -> Asset {
    let mut mesh = Mesh::default();
    // 8 vértices de uma caixa
    let corners = [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(min.x, max.y, max.z),
    ];
    for c in corners {
        mesh.verts.push(Vertex::new(c.x, c.y, c.z));
    }
    Asset::new(name, mesh)
}

#[test]
fn test_bounding_box_3d_basics() {
    let bb1 = BoundingBox3D::new(Vec3::new(-1.0, -2.0, -3.0), Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(bb1.center(), Vec3::ZERO);
    assert_eq!(bb1.size(), Vec3::new(2.0, 4.0, 6.0));
    assert!(bb1.is_finite());

    let bb2 = BoundingBox3D::new(Vec3::new(2.0, 0.0, 0.0), Vec3::new(4.0, 1.0, 1.0));
    assert!(!bb1.intersects(&bb2));

    let union = bb1.union(&bb2);
    assert_eq!(union.min, Vec3::new(-1.0, -2.0, -3.0));
    assert_eq!(union.max, Vec3::new(4.0, 2.0, 3.0));

    // Caixas com sobreposição
    let bb3 = BoundingBox3D::new(Vec3::new(0.5, 0.5, 0.5), Vec3::new(2.0, 2.0, 2.0));
    assert!(bb1.intersects(&bb3));
}

#[test]
fn test_align_to_active_object_center_x() {
    // Objeto A (ativo): centrado em x = 10, tamanho 2 (min: 9, max: 11)
    let a = make_box_asset("A", Vec3::new(9.0, 0.0, 0.0), Vec3::new(11.0, 2.0, 2.0));
    // Objeto B: centrado em x = 0, tamanho 4 (min: -2, max: 2)
    let b = make_box_asset("B", Vec3::new(-2.0, 0.0, 0.0), Vec3::new(2.0, 2.0, 2.0));

    let deltas = compute_alignment_deltas(
        &[&a, &b],
        Some(a.id),
        AlignAxis::X,
        AlignMode::Center,
        AlignReference::ActiveObject,
    )
    .expect("cálculo de alinhamento com sucesso");

    assert_eq!(deltas.len(), 2);

    let delta_a = deltas.iter().find(|(id, _)| *id == a.id).unwrap().1;
    let delta_b = deltas.iter().find(|(id, _)| *id == b.id).unwrap().1;

    // Objeto ativo deve permanecer estritamente imóvel
    assert_eq!(delta_a, Vec3::ZERO);
    // Objeto B deve se mover +10 em X para alinhar seu centro (0 -> 10)
    assert_eq!(delta_b, Vec3::new(10.0, 0.0, 0.0));
}

#[test]
fn test_align_to_active_object_min_y() {
    // Objeto A (ativo): base em Y = 5.0
    let a = make_box_asset("A", Vec3::new(0.0, 5.0, 0.0), Vec3::new(2.0, 7.0, 2.0));
    // Objeto B: base em Y = -1.0
    let b = make_box_asset("B", Vec3::new(0.0, -1.0, 0.0), Vec3::new(2.0, 3.0, 2.0));

    let deltas = compute_alignment_deltas(
        &[&a, &b],
        Some(a.id),
        AlignAxis::Y,
        AlignMode::Min,
        AlignReference::ActiveObject,
    )
    .expect("alinhamento de base Y");

    let delta_a = deltas.iter().find(|(id, _)| *id == a.id).unwrap().1;
    let delta_b = deltas.iter().find(|(id, _)| *id == b.id).unwrap().1;

    assert_eq!(delta_a, Vec3::ZERO);
    // B deve subir +6 em Y (-1 + 6 = 5)
    assert_eq!(delta_b, Vec3::new(0.0, 6.0, 0.0));
}

#[test]
fn test_align_to_active_object_max_z() {
    // Objeto A (ativo): max Z = 8.0
    let a = make_box_asset("A", Vec3::new(0.0, 0.0, 6.0), Vec3::new(2.0, 2.0, 8.0));
    // Objeto B: max Z = 2.0
    let b = make_box_asset("B", Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 2.0, 2.0));

    let deltas = compute_alignment_deltas(
        &[&a, &b],
        Some(a.id),
        AlignAxis::Z,
        AlignMode::Max,
        AlignReference::ActiveObject,
    )
    .expect("alinhamento de topo Z");

    let delta_b = deltas.iter().find(|(id, _)| *id == b.id).unwrap().1;
    assert_eq!(delta_b, Vec3::new(0.0, 0.0, 6.0));
}

#[test]
fn test_align_to_selection_bounds() {
    // Objeto A de x = 0 a 2 (centro = 1)
    let a = make_box_asset("A", Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 2.0, 2.0));
    // Objeto B de x = 8 a 10 (centro = 9)
    let b = make_box_asset("B", Vec3::new(8.0, 0.0, 0.0), Vec3::new(10.0, 2.0, 2.0));
    // Bounding box combinada: min X = 0, max X = 10, centro = 5.0

    let deltas = compute_alignment_deltas(
        &[&a, &b],
        None,
        AlignAxis::X,
        AlignMode::Center,
        AlignReference::SelectionBounds,
    )
    .expect("alinhamento com selection bounds");

    let delta_a = deltas.iter().find(|(id, _)| *id == a.id).unwrap().1;
    let delta_b = deltas.iter().find(|(id, _)| *id == b.id).unwrap().1;

    // A deve ir de 1 para 5 (+4)
    assert_eq!(delta_a, Vec3::new(4.0, 0.0, 0.0));
    // B deve ir de 9 para 5 (-4)
    assert_eq!(delta_b, Vec3::new(-4.0, 0.0, 0.0));
}

#[test]
fn test_align_to_world_origin() {
    // Objeto centrado em (5, 5, 5)
    let a = make_box_asset("A", Vec3::new(4.0, 4.0, 4.0), Vec3::new(6.0, 6.0, 6.0));

    let deltas = compute_alignment_deltas(
        &[&a],
        None,
        AlignAxis::X,
        AlignMode::Center,
        AlignReference::WorldOrigin,
    )
    .expect("alinhamento com origem");

    assert_eq!(deltas.len(), 1);
    assert_eq!(deltas[0].1, Vec3::new(-5.0, 0.0, 0.0));
}

#[test]
fn test_align_validation_errors() {
    let a = make_box_asset("A", Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));

    // Menos de 2 objetos para ActiveObject deve falhar
    let res = compute_alignment_deltas(
        &[&a],
        Some(a.id),
        AlignAxis::X,
        AlignMode::Center,
        AlignReference::ActiveObject,
    );
    assert!(matches!(res, Err(AlignError::InsufficientObjects { .. })));

    // Objeto ativo não encontrado
    let b = make_box_asset("B", Vec3::new(2.0, 0.0, 0.0), Vec3::new(3.0, 1.0, 1.0));
    let fake_id = Uuid::new_v4();
    let res2 = compute_alignment_deltas(
        &[&a, &b],
        Some(fake_id),
        AlignAxis::X,
        AlignMode::Center,
        AlignReference::ActiveObject,
    );
    assert!(matches!(res2, Err(AlignError::ActiveObjectNotFound(_))));

    // Objeto vazio
    let empty_mesh = Asset::new("Empty", Mesh::default());
    let res3 = compute_alignment_deltas(
        &[&a, &empty_mesh],
        Some(a.id),
        AlignAxis::X,
        AlignMode::Center,
        AlignReference::ActiveObject,
    );
    assert!(matches!(res3, Err(AlignError::InvalidBounds(_))));
}

#[test]
fn test_distribution_equal_gaps() {
    // Três cubos de tamanho 2.0 em X:
    // C0: min=0, max=2
    // C1: min=3, max=5 (desalinhado)
    // C2: min=8, max=10
    // Total span: 10 - 0 = 10. Tamanhos somados: 2 + 2 + 2 = 6.
    // Espaço livre: 10 - 6 = 4. Gaps = 2. Gap uniforme = 4 / 2 = 2.0.
    // Posições esperadas:
    // C0 min = 0 (fixo, delta 0)
    // C1 min = 0 + 2 (tamanho C0) + 2 (gap) = 4 (delta = 4 - 3 = +1)
    // C2 min = 8 (fixo, delta 0)
    let c0 = make_box_asset("C0", Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 2.0, 2.0));
    let c1 = make_box_asset("C1", Vec3::new(3.0, 0.0, 0.0), Vec3::new(5.0, 2.0, 2.0));
    let c2 = make_box_asset("C2", Vec3::new(8.0, 0.0, 0.0), Vec3::new(10.0, 2.0, 2.0));

    let deltas = compute_distribution_deltas(&[&c0, &c1, &c2], AlignAxis::X)
        .expect("distribuição calculada");

    let delta_c0 = deltas.iter().find(|(id, _)| *id == c0.id).unwrap().1;
    let delta_c1 = deltas.iter().find(|(id, _)| *id == c1.id).unwrap().1;
    let delta_c2 = deltas.iter().find(|(id, _)| *id == c2.id).unwrap().1;

    assert_eq!(delta_c0, Vec3::ZERO);
    assert_eq!(delta_c1, Vec3::new(1.0, 0.0, 0.0));
    assert_eq!(delta_c2, Vec3::ZERO);
}

#[test]
fn test_distribution_requires_minimum_three_objects() {
    let a = make_box_asset("A", Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
    let b = make_box_asset("B", Vec3::new(2.0, 0.0, 0.0), Vec3::new(3.0, 1.0, 1.0));

    let res = compute_distribution_deltas(&[&a, &b], AlignAxis::X);
    assert!(matches!(
        res,
        Err(AlignError::InsufficientObjects {
            required: 3,
            found: 2
        })
    ));
}
