//! B10 (ADR-C16): a seleção de vértice/aresta no Sólido precisa ser fiel ao que
//! se vê. Para um cubo (convexo) a verdade geométrica é simples: um componente é
//! visível se alguma face vizinha está de frente para a câmera. Clicar na
//! posição projetada de um componente visível escolhe exatamente ele; clicar
//! sobre a posição de um oculto nunca o escolhe.

use glam::{Vec2, Vec3};
use petunia_core::picking::{PickComponent, pick_mesh_in_scene};
use petunia_core::viewport_query::ViewportSceneQuery;
use petunia_core::{Camera, Projection, SelectionDomain};
use petunia_mesh::Mesh;

const VIEWPORT: Vec2 = Vec2::new(800.0, 600.0);

fn cameras() -> Vec<(String, Camera)> {
    let mut out = Vec::new();
    for (yaw, pitch) in [
        (0.7, 0.5),
        (2.3, 0.35),
        (-0.9, -0.4),
        (0.15, 1.1),
        (3.9, 0.05),
    ] {
        for proj in [Projection::Perspective, Projection::Ortho] {
            let camera = Camera {
                yaw,
                pitch,
                distance: 7.0,
                aspect: VIEWPORT.x / VIEWPORT.y,
                proj,
                ortho_half_h: 2.5,
                ..Camera::default()
            };
            out.push((format!("yaw={yaw} pitch={pitch} {proj:?}"), camera));
        }
    }
    out
}

/// Direção do ponto para o olho (perspectiva) ou contrária à vista (ortográfica).
fn to_eye(camera: &Camera, point: Vec3) -> Vec3 {
    match camera.proj {
        Projection::Perspective => camera.eye() - point,
        Projection::Ortho => -camera.forward(),
    }
}

fn front_facing(mesh: &Mesh, face: usize, camera: &Camera) -> bool {
    let Some(f) = mesh.faces.get(face) else {
        return false;
    };
    let center = f
        .verts
        .iter()
        .map(|&v| mesh.verts[v as usize].vec())
        .sum::<Vec3>()
        / f.verts.len() as f32;
    mesh.face_normal(face).dot(to_eye(camera, center)) > 1.0e-4
}

fn ndc(camera: &Camera, point: Vec3) -> Vec2 {
    let clip = camera.view_proj() * point.extend(1.0);
    Vec2::new(clip.x / clip.w, clip.y / clip.w)
}

fn scene(mesh: &Mesh) -> (petunia_project::Project, Mesh) {
    let mut project = petunia_project::Project::new();
    project.assets = vec![petunia_project::Asset::new("cube", mesh.clone())];
    project.active = 0;
    (project, mesh.clone())
}

#[test]
fn clicking_a_visible_vertex_picks_it_and_a_hidden_one_is_never_picked() {
    let (project, mesh) = scene(&Mesh::cube(2.0));
    let query = ViewportSceneQuery::new(&project);
    let mut failures = Vec::new();
    for (label, camera) in cameras() {
        for (index, vertex) in mesh.verts.iter().enumerate() {
            let visible = (0..mesh.faces.len())
                .filter(|&f| mesh.faces[f].verts.contains(&(index as u32)))
                .any(|f| front_facing(&mesh, f, &camera));
            let hit = pick_mesh_in_scene(
                &mesh,
                0,
                &query,
                &camera,
                VIEWPORT,
                ndc(&camera, vertex.vec()),
                SelectionDomain::Vertex,
                false,
            )
            .map(|h| h.component);
            let picked_it = hit == Some(PickComponent::Vertex(index));
            if visible != picked_it {
                failures.push(format!(
                    "{label}: vértice {index} visível={visible} escolhido={hit:?}"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} falhas:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn clicking_a_visible_edge_picks_it_and_a_hidden_one_is_never_picked() {
    let (project, mesh) = scene(&Mesh::cube(2.0));
    let query = ViewportSceneQuery::new(&project);
    let mut failures = Vec::new();
    for (label, camera) in cameras() {
        for (a, b) in mesh.edges_unique() {
            let faces: Vec<usize> = (0..mesh.faces.len())
                .filter(|&f| {
                    let v = &mesh.faces[f].verts;
                    v.contains(&a) && v.contains(&b)
                })
                .collect();
            let visible = faces.iter().any(|&f| front_facing(&mesh, f, &camera));
            let mid = (mesh.verts[a as usize].vec() + mesh.verts[b as usize].vec()) * 0.5;
            let hit = pick_mesh_in_scene(
                &mesh,
                0,
                &query,
                &camera,
                VIEWPORT,
                ndc(&camera, mid),
                SelectionDomain::Edge,
                false,
            )
            .map(|h| h.component);
            let picked_it = matches!(hit, Some(PickComponent::Edge(x, y)) if (x, y) == (a, b) || (y, x) == (a, b));
            if visible != picked_it {
                failures.push(format!(
                    "{label}: aresta {a}-{b} visível={visible} escolhida={hit:?}"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} falhas:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
