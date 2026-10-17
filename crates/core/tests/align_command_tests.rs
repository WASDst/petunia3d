//! Testes de integração e regressão para comandos transacionais de alinhamento e distribuição no Core.

use glam::Vec3;
use petunia_core::AppState;
use petunia_core::align_commands::{AlignObjectsCmd, DistributeObjectsCmd};
use petunia_mesh::{Mesh, Vertex};
use petunia_project::{AlignAxis, AlignMode, AlignReference, Asset};

fn make_box_asset(name: &str, min: Vec3, max: Vec3) -> Asset {
    let mut mesh = Mesh::default();
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
fn test_align_objects_command_with_undo_redo() {
    let mut state = AppState::new("pt-BR");
    state.project.assets.clear();

    // Cria objeto A (ativo): centrado em X = 10 (min: 9, max: 11)
    let a = make_box_asset("A", Vec3::new(9.0, 0.0, 0.0), Vec3::new(11.0, 2.0, 2.0));
    let a_id = a.id;
    state.project.assets.push(a);

    // Cria objeto B: centrado em X = 0 (min: -2, max: 2)
    let b = make_box_asset("B", Vec3::new(-2.0, 0.0, 0.0), Vec3::new(2.0, 2.0, 2.0));
    let b_id = b.id;
    state.project.assets.push(b);

    // Configura seleção: A é o ativo, e ambos estão na seleção
    state.project.active = 0;
    state.session.selection.assets = vec![a_id, b_id];

    // Executa comando de alinhar centros em X
    let cmd = AlignObjectsCmd::new(
        AlignAxis::X,
        AlignMode::Center,
        AlignReference::ActiveObject,
    );
    let res = state.in_context(|ctx| ctx.dispatch(&cmd));
    assert!(
        res.is_ok(),
        "Execução do comando AlignObjectsCmd falhou: {:?}",
        res
    );

    // Verifica que B foi transladado para centro X = 10 (delta +10)
    let b_bounds = state.project.assets[1].world_bounds().expect("bounds de B");
    assert!((b_bounds.center().x - 10.0).abs() < 1e-4);

    // Verifica que A (âncora ativa) não se moveu
    let a_bounds = state.project.assets[0].world_bounds().expect("bounds de A");
    assert!((a_bounds.center().x - 10.0).abs() < 1e-4);

    // Testa Desfazer (Undo)
    assert!(state.undo());
    let b_restored = state.project.assets[1]
        .world_bounds()
        .expect("bounds de B pós-undo");
    assert!(
        (b_restored.center().x - 0.0).abs() < 1e-4,
        "Undo deve restaurar centro original de B"
    );

    // Testa Refazer (Redo)
    assert!(state.redo());
    let b_redone = state.project.assets[1]
        .world_bounds()
        .expect("bounds de B pós-redo");
    assert!(
        (b_redone.center().x - 10.0).abs() < 1e-4,
        "Redo deve reaplicar alinhamento de B"
    );
}

#[test]
fn test_distribute_objects_command_with_undo() {
    let mut state = AppState::new("pt-BR");
    state.project.assets.clear();

    let c0 = make_box_asset("C0", Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 2.0, 2.0));
    let c1 = make_box_asset("C1", Vec3::new(3.0, 0.0, 0.0), Vec3::new(5.0, 2.0, 2.0));
    let c2 = make_box_asset("C2", Vec3::new(8.0, 0.0, 0.0), Vec3::new(10.0, 2.0, 2.0));

    let ids = vec![c0.id, c1.id, c2.id];
    state.project.assets.extend([c0, c1, c2]);
    state.project.active = 0;
    state.session.selection.assets = ids;

    let cmd = DistributeObjectsCmd::on_x();
    let res = state.in_context(|ctx| ctx.dispatch(&cmd));
    assert!(res.is_ok(), "DistributeObjectsCmd falhou: {:?}", res);

    // C1 deve ter sido movido para min = 4.0 (gap igual a 2.0)
    let c1_bounds = state.project.assets[1]
        .world_bounds()
        .expect("bounds de C1");
    assert!((c1_bounds.min.x - 4.0).abs() < 1e-4);

    // Desfazer deve restaurar C1 para min = 3.0
    assert!(state.undo());
    let c1_undo = state.project.assets[1]
        .world_bounds()
        .expect("bounds de C1 pós-undo");
    assert!((c1_undo.min.x - 3.0).abs() < 1e-4);
}

#[test]
fn test_align_command_validation_guard() {
    let mut state = AppState::new("pt-BR");
    state.project.assets.clear();

    // Apenas 1 objeto selecionado com referência ActiveObject deve ser rejeitado em can_execute
    let a = make_box_asset("A", Vec3::ZERO, Vec3::ONE);
    state.session.selection.assets = vec![a.id];
    state.project.assets.push(a);
    state.project.active = 0;

    let cmd = AlignObjectsCmd::center_x();
    let res = state.in_context(|ctx| ctx.dispatch(&cmd));
    assert!(
        res.is_err(),
        "Deve falhar can_execute com menos de 2 objetos"
    );
}
