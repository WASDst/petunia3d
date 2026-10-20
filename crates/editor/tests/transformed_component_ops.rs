//! Vértices, arestas e faces em objetos com transformação não uniforme (escala
//! esticada + rotação + translação, como a coluna do relato de uso): seleção por
//! clique na tela, operações básicas, Undo exato e movimento em deltas de mundo.

use glam::{Mat4, Quat, Vec3};
use petunia_core::{AppState, PrimitiveKind, SelectionDomain};
use petunia_editor::viewport_intents::{NormalizedViewportPoint, ViewportSelectionIntent};
use petunia_editor::{PlaceholderViewport, SlintUiBridge, UiIntent};
use petunia_project::ObjectTransform;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

/// Cubo esticado em Y, girado e deslocado: a "coluna" do relato.
fn pillar() -> SlintUiBridge<PlaceholderViewport> {
    let mut b = SlintUiBridge::new(AppState::new("en"), PlaceholderViewport::default());
    b.resize_viewport(WIDTH, HEIGHT);
    b.apply(UiIntent::AddPrimitive(PrimitiveKind::Cube));
    b.state.confirm_primitive();
    let matrix = Mat4::from_scale_rotation_translation(
        Vec3::new(0.44, 2.75, 0.49),
        Quat::from_euler(
            glam::EulerRot::XYZ,
            10f32.to_radians(),
            35f32.to_radians(),
            (-20f32).to_radians(),
        ),
        Vec3::new(-0.17, 0.42, -0.16),
    );
    let transform = ObjectTransform::from_mat4(matrix).expect("TRS válido");
    b.state
        .project
        .active_mut()
        .expect("objeto ativo")
        .set_object_transform(transform);
    b
}

fn world_positions(b: &SlintUiBridge<PlaceholderViewport>) -> Vec<Vec3> {
    b.state.project.active().expect("ativo").world_positions()
}

/// Posição normalizada (0..1) de um ponto de mundo na vista.
fn screen_of(b: &SlintUiBridge<PlaceholderViewport>, world: Vec3) -> NormalizedViewportPoint {
    let ndc = b.state.session.camera.project_ndc(world);
    NormalizedViewportPoint::new((ndc.x + 1.0) * 0.5, (1.0 - ndc.y) * 0.5).expect("ponto finito")
}

fn click(b: &mut SlintUiBridge<PlaceholderViewport>, world: Vec3) {
    let at = screen_of(b, world);
    b.apply_viewport_selection(ViewportSelectionIntent::Point {
        at,
        extend: false,
        loop_select: false,
    });
}

/// Vértices voltados para a câmera (mais perto que o centro do objeto).
fn front_vertices(b: &SlintUiBridge<PlaceholderViewport>) -> Vec<usize> {
    let camera = &b.state.session.camera;
    let positions = world_positions(b);
    let center = positions.iter().copied().sum::<Vec3>() / positions.len() as f32;
    let depth = |p: Vec3| (p - camera.eye()).dot(camera.forward());
    positions
        .iter()
        .enumerate()
        .filter(|(_, p)| depth(**p) < depth(center) - 0.05)
        .map(|(i, _)| i)
        .collect()
}

fn selected_vertices(b: &SlintUiBridge<PlaceholderViewport>) -> Vec<usize> {
    b.state
        .project
        .active_mesh()
        .expect("malha")
        .verts
        .iter()
        .enumerate()
        .filter(|(_, v)| v.selected)
        .map(|(i, _)| i)
        .collect()
}

#[test]
fn clicking_a_vertex_selects_exactly_that_vertex_on_a_transformed_object() {
    let mut b = pillar();
    b.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex));
    let positions = world_positions(&b);
    let front = front_vertices(&b);
    assert!(
        front.len() >= 3,
        "a vista enxerga vários vértices: {front:?}"
    );
    for index in front {
        click(&mut b, positions[index]);
        assert_eq!(
            selected_vertices(&b),
            vec![index],
            "clique no vértice {index} (mundo {:?})",
            positions[index]
        );
    }
}

#[test]
fn clicking_an_edge_midpoint_selects_that_edge_on_a_transformed_object() {
    let mut b = pillar();
    b.apply(UiIntent::SetSelectionDomain(SelectionDomain::Edge));
    let positions = world_positions(&b);
    let front: std::collections::HashSet<usize> = front_vertices(&b).into_iter().collect();
    let edges: Vec<(u32, u32)> = b.state.project.active_mesh().expect("malha").edges_unique();
    let mut tested = 0;
    for (a, c) in edges {
        if !front.contains(&(a as usize)) || !front.contains(&(c as usize)) {
            continue;
        }
        let middle = (positions[a as usize] + positions[c as usize]) * 0.5;
        click(&mut b, middle);
        let mesh = b.state.project.active_mesh().expect("malha");
        assert!(
            mesh.selected_edges.contains(&(a, c)) || mesh.selected_edges.contains(&(c, a)),
            "clique no meio da aresta ({a}, {c}): {:?}",
            mesh.selected_edges
        );
        tested += 1;
    }
    assert!(tested >= 2, "arestas da frente testadas: {tested}");
}

#[test]
fn basic_component_operations_stay_valid_and_undo_exactly() {
    // Acumula as falhas para relatar todas de uma vez (um caso ruim não esconde os outros).
    let mut failures: Vec<String> = Vec::new();
    for (domain, command) in [
        (SelectionDomain::Vertex, "model.extrude"),
        (SelectionDomain::Vertex, "model.merge"),
        (SelectionDomain::Vertex, "model.dissolve"),
        (SelectionDomain::Vertex, "model.bevel"),
        (SelectionDomain::Edge, "model.extrude"),
        (SelectionDomain::Edge, "model.bevel"),
        (SelectionDomain::Edge, "model.subdivide"),
        (SelectionDomain::Edge, "model.dissolve"),
        (SelectionDomain::Face, "model.extrude"),
        (SelectionDomain::Face, "model.inset"),
        (SelectionDomain::Face, "model.subdivide"),
        (SelectionDomain::Face, "model.flip_normals"),
    ] {
        let mut b = pillar();
        b.apply(UiIntent::SetSelectionDomain(domain));
        {
            let mesh = b.state.project.active_mesh_mut().expect("malha");
            match domain {
                SelectionDomain::Vertex => {
                    for v in mesh.verts.iter_mut().take(2) {
                        v.selected = true;
                    }
                }
                SelectionDomain::Edge => {
                    let first: Vec<(u32, u32)> = mesh.edges_unique().into_iter().take(2).collect();
                    mesh.selected_edges.extend(first);
                }
                _ => {
                    if let Some(face) = mesh.faces.first_mut() {
                        face.selected = true;
                    }
                }
            }
        }
        let before_world = world_positions(&b);
        let before_faces = b.state.project.active_mesh().expect("malha").faces.len();
        let depth = b.state.project.undo.depth().0;
        let tag = format!("{domain:?}/{command}");
        match b.state.dispatch_command(command) {
            Ok(()) => {
                let asset = b.state.project.active().expect("ativo");
                let mesh = asset.mesh.to_owned_value();
                if !mesh.verts.iter().all(|v| v.vec().is_finite()) {
                    failures.push(format!("{tag}: vértices não finitos"));
                }
                if let Err(error) = mesh.validate_invariants() {
                    failures.push(format!(
                        "{tag}: malha inválida depois da operação: {error:?}"
                    ));
                }
                if b.state.project.undo.depth().0 != depth + 1 {
                    failures.push(format!("{tag}: esperado um único passo de Undo"));
                }
                if !b.state.undo() {
                    failures.push(format!("{tag}: Undo recusado"));
                    continue;
                }
                let restored = world_positions(&b);
                let exact = restored.len() == before_world.len()
                    && restored
                        .iter()
                        .zip(&before_world)
                        .all(|(after, before)| (*after - *before).length() < 1.0e-5);
                if !exact {
                    failures.push(format!("{tag}: Undo não devolve a geometria exata"));
                }
                let faces = b.state.project.active_mesh().expect("malha").faces.len();
                if faces != before_faces {
                    failures.push(format!("{tag}: faces após Undo {faces} != {before_faces}"));
                }
            }
            Err(error) => {
                // Recusar é válido (precondição); nunca pode alterar a malha nem o Undo.
                if b.state.project.undo.depth().0 != depth {
                    failures.push(format!("{tag}: recusa com efeito no Undo ({error})"));
                }
                let after = world_positions(&b);
                if !after
                    .iter()
                    .zip(&before_world)
                    .all(|(a, c)| (*a - *c).length() < 1.0e-6)
                {
                    failures.push(format!("{tag}: recusa alterou a geometria ({error})"));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "operações com falha:\n{}",
        failures.join("\n")
    );
}

#[test]
fn moving_selected_vertices_applies_the_world_delta_through_a_stretched_object() {
    let mut b = pillar();
    b.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex));
    let positions = world_positions(&b);
    let target = *front_vertices(&b).first().expect("vértice visível");
    click(&mut b, positions[target]);
    assert_eq!(selected_vertices(&b), vec![target]);

    let delta = Vec3::new(0.3, 0.0, 0.0);
    b.state
        .begin_modal(petunia_core::ModalKind::Move)
        .expect("movimento");
    b.state
        .update_modal(delta, delta.length())
        .expect("prévia do movimento");
    assert!(b.state.commit_modal());
    let after = world_positions(&b);
    for (i, (a, p)) in after.iter().zip(&positions).enumerate() {
        let expected = if i == target { *p + delta } else { *p };
        assert!(
            (*a - expected).length() < 1.0e-3,
            "vértice {i}: mundo {a:?}, esperado {expected:?}"
        );
    }
}
