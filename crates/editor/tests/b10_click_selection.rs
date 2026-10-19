//! B10 (ADR-C16): o clique de verdade (`tool_pointer_ex`, px lógicos da vista)
//! em um objeto movido e girado escolhe o componente que se vê sob o ponteiro.
//! A verdade geométrica vem das normais das faces no mundo (o cubo é convexo).

use glam::{Mat3, Quat, Vec2, Vec3};
use petunia_core::{AppState, SelectionDomain};
use petunia_editor::{PlaceholderViewport, SlintUiBridge, UiIntent};

const W: f32 = 800.0;
const H: f32 = 600.0;

fn bridge(domain: SelectionDomain) -> SlintUiBridge<PlaceholderViewport> {
    bridge_with(domain, true)
}

fn bridge_with(domain: SelectionDomain, transformed: bool) -> SlintUiBridge<PlaceholderViewport> {
    let mut b = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    b.resize_viewport(W as u32, H as u32);
    b.apply(UiIntent::SetActiveTool("select".into()));
    b.apply(UiIntent::SetSelectionDomain(domain));
    if !transformed {
        return b;
    }
    let asset = b.state.project.active_mut().expect("cubo inicial");
    asset.set_object_transform(petunia_project::ObjectTransform::from_trs(
        Vec3::new(1.5, 0.3, -0.7),
        Quat::from_euler(glam::EulerRot::XYZ, 0.4, 0.9, -0.2),
        Vec3::new(1.2, 0.8, 1.0),
    ));
    b.state
        .emit_project_changed(petunia_project::ProjectChanges::TRANSFORMS);
    b
}

fn to_pixels(b: &SlintUiBridge<PlaceholderViewport>, world: Vec3) -> Option<Vec2> {
    let clip = b.state.session.camera.view_proj() * world.extend(1.0);
    if clip.w <= 0.0 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    Some(Vec2::new((ndc.x * 0.5 + 0.5) * W, (0.5 - ndc.y * 0.5) * H))
}

/// Faces de frente para a câmera, no mundo.
fn front_faces(b: &SlintUiBridge<PlaceholderViewport>) -> Vec<bool> {
    let asset = b.state.project.active().expect("cubo");
    let world = asset.world_matrix();
    let normal_matrix = Mat3::from_mat4(world).inverse().transpose();
    let mesh: &petunia_core::Mesh = &asset.mesh;
    let eye = b.state.session.camera.eye();
    (0..mesh.faces.len())
        .map(|f| {
            let face = &mesh.faces[f];
            let center = face
                .verts
                .iter()
                .map(|&v| world.transform_point3(mesh.verts[v as usize].vec()))
                .sum::<Vec3>()
                / face.verts.len() as f32;
            (normal_matrix * mesh.face_normal(f)).dot(eye - center) > 1.0e-4
        })
        .collect()
}

fn click(b: &mut SlintUiBridge<PlaceholderViewport>, at: Vec2) {
    b.tool_pointer_ex(0, at.x, at.y, false, false, false);
    b.tool_pointer_ex(2, at.x, at.y, false, false, false);
}

fn orbit_views(b: &mut SlintUiBridge<PlaceholderViewport>, view: usize) {
    let camera = &mut b.state.session.camera;
    camera.target = Vec3::new(1.5, 0.3, -0.7);
    camera.distance = 7.0;
    camera.yaw = [0.7, 2.4, -1.1, 3.6][view];
    camera.pitch = [0.5, 0.25, -0.45, 0.9][view];
}

#[test]
fn clicking_a_vertex_on_a_transformed_object_picks_what_is_seen() {
    let mut failures = Vec::new();
    for view in 0..4 {
        let mut b = bridge(SelectionDomain::Vertex);
        orbit_views(&mut b, view);
        let front = front_faces(&b);
        let asset = b.state.project.active().expect("cubo");
        let world = asset.world_matrix();
        let mesh: petunia_core::Mesh = (*asset.mesh).clone();
        for (index, vertex) in mesh.verts.iter().enumerate() {
            let visible = mesh
                .faces
                .iter()
                .enumerate()
                .any(|(f, face)| face.verts.contains(&(index as u32)) && front[f]);
            let Some(at) = to_pixels(&b, world.transform_point3(vertex.vec())) else {
                continue;
            };
            click(&mut b, at);
            let after = b.state.project.active_mesh().expect("malha");
            let picked: Vec<usize> = after
                .verts
                .iter()
                .enumerate()
                .filter(|(_, v)| v.selected)
                .map(|(i, _)| i)
                .collect();
            let ok = if visible {
                picked == vec![index]
            } else {
                !picked.contains(&index)
            };
            if !ok {
                failures.push(format!("vista {view}: vértice {index} visível={visible} selecionados={picked:?} em {at:?}"));
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
fn clicking_a_face_on_a_transformed_object_picks_the_front_face() {
    let mut failures = Vec::new();
    for view in 0..4 {
        let mut b = bridge(SelectionDomain::Face);
        orbit_views(&mut b, view);
        let front = front_faces(&b);
        let asset = b.state.project.active().expect("cubo");
        let world = asset.world_matrix();
        let mesh: petunia_core::Mesh = (*asset.mesh).clone();
        for (f, face) in mesh.faces.iter().enumerate() {
            if !front[f] {
                continue;
            }
            let center = face
                .verts
                .iter()
                .map(|&v| world.transform_point3(mesh.verts[v as usize].vec()))
                .sum::<Vec3>()
                / face.verts.len() as f32;
            let Some(at) = to_pixels(&b, center) else {
                continue;
            };
            // Sem seleção não há gizmo: o clique é só da face sob o ponteiro.
            let _ = b.state.dispatch_command("select.none");
            assert!(b.gizmo_handle_at(at.x, at.y).is_none(), "gizmo sem seleção");
            click(&mut b, at);
            let after = b.state.project.active_mesh().expect("malha");
            let picked: Vec<usize> = after
                .faces
                .iter()
                .enumerate()
                .filter(|(_, face)| face.selected)
                .map(|(i, _)| i)
                .collect();
            if picked != vec![f] {
                failures.push(format!(
                    "vista {view}: face {f} selecionadas={picked:?} em {at:?}"
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

/// Controle: o mesmo clique no cubo sem transformação (separa defeito do teste).
#[test]
fn clicking_a_vertex_on_an_untransformed_object_picks_what_is_seen() {
    let mut failures = Vec::new();
    for view in 0..4 {
        let mut b = bridge_with(SelectionDomain::Vertex, false);
        orbit_views(&mut b, view);
        b.state.session.camera.target = Vec3::ZERO;
        let front = front_faces(&b);
        let mesh: petunia_core::Mesh = (*b.state.project.active().expect("cubo").mesh).clone();
        for (index, vertex) in mesh.verts.iter().enumerate() {
            let visible = mesh
                .faces
                .iter()
                .enumerate()
                .any(|(f, face)| face.verts.contains(&(index as u32)) && front[f]);
            let Some(at) = to_pixels(&b, vertex.vec()) else {
                continue;
            };
            click(&mut b, at);
            let picked: Vec<usize> = b
                .state
                .project
                .active_mesh()
                .expect("malha")
                .verts
                .iter()
                .enumerate()
                .filter(|(_, v)| v.selected)
                .map(|(i, _)| i)
                .collect();
            let ok = if visible {
                picked == vec![index]
            } else {
                !picked.contains(&index)
            };
            if !ok {
                failures.push(format!(
                    "vista {view}: vértice {index} visível={visible} selecionados={picked:?}"
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

/// Com uma face selecionada, clicar sobre uma alça do gizmo é da alça: a seleção
/// não muda (nem some). Fora das alças, o clique troca de face.
#[test]
fn clicking_on_a_gizmo_handle_keeps_the_face_selection() {
    let mut b = bridge(SelectionDomain::Face);
    orbit_views(&mut b, 1);
    let front = front_faces(&b);
    let first = front.iter().position(|f| *f).expect("face de frente");
    let asset = b.state.project.active().expect("cubo").clone();
    let world = asset.world_matrix();
    let face_center = |f: usize| {
        let face = &asset.mesh.faces[f];
        face.verts
            .iter()
            .map(|&v| world.transform_point3(asset.mesh.verts[v as usize].vec()))
            .sum::<Vec3>()
            / face.verts.len() as f32
    };
    let first_at = to_pixels(&b, face_center(first)).expect("na vista");
    click(&mut b, first_at);
    let gizmo = b.gizmo_handle_at(0.0, 0.0);
    assert!(gizmo.is_none());
    // Procura um ponto sobre uma alça varrendo a vista.
    let mut handle_point = None;
    'scan: for y in (0..H as i32).step_by(4) {
        for x in (0..W as i32).step_by(4) {
            if b.gizmo_handle_at(x as f32, y as f32).is_some() {
                handle_point = Some(Vec2::new(x as f32, y as f32));
                break 'scan;
            }
        }
    }
    let at = handle_point.expect("o gizmo aparece com a face selecionada");
    click(&mut b, at);
    let selected: Vec<usize> = b
        .state
        .project
        .active_mesh()
        .expect("malha")
        .faces
        .iter()
        .enumerate()
        .filter(|(_, f)| f.selected)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        selected,
        vec![first],
        "clique na alça não troca nem limpa a seleção"
    );
}

fn any_handle(b: &SlintUiBridge<PlaceholderViewport>) -> usize {
    let mut hits = 0;
    for y in (0..H as i32).step_by(6) {
        for x in (0..W as i32).step_by(6) {
            if b.gizmo_handle_at(x as f32, y as f32).is_some() {
                hits += 1;
            }
        }
    }
    hits
}

/// B11: as Preferências escolhem quais alças o gizmo do Selecionar mostra; alça
/// escondida também não recebe clique. Sem nenhuma, não há gizmo.
#[test]
fn select_gizmo_shows_only_the_chosen_handle_families() {
    let mut b = bridge(SelectionDomain::Object);
    orbit_views(&mut b, 0);
    let all = any_handle(&b);
    assert!(all > 0, "gizmo padrão com as três famílias");
    assert!(b.set_select_gizmo_handle(1, false));
    let without_rotate = any_handle(&b);
    assert!(
        without_rotate < all,
        "sem os anéis há menos área de alça: {without_rotate} < {all}"
    );
    assert!(b.set_select_gizmo_handle(0, false));
    assert!(b.set_select_gizmo_handle(2, false));
    assert_eq!(any_handle(&b), 0, "nenhuma família: nenhum gizmo");
    assert!(
        !b.set_select_gizmo_handle(7, true),
        "família hostil é recusada"
    );
    b.apply(UiIntent::SetActiveTool("rotate".into()));
    assert!(any_handle(&b) > 0, "Girar continua com a própria alça");
}

/// B15: Local alinha o frame do gizmo e das restrições aos eixos do objeto
/// girado; Global volta aos eixos do mundo. A troca não mexe no documento.
#[test]
fn local_orientation_follows_the_rotated_object_axes() {
    let mut b = bridge(SelectionDomain::Object);
    let rotation = b.state.project.active().expect("cubo").transform.rotation();
    let revision = b.state.project.project.revision_id();
    b.state
        .set_transform_orientation(petunia_core::TransformOrientation::Local);
    let frame = b
        .state
        .calculate_transform_frame(petunia_core::TransformOrientation::Local);
    assert!(
        (frame.x_axis - rotation * Vec3::X).length() < 1e-4,
        "{frame:?}"
    );
    let global = b
        .state
        .calculate_transform_frame(petunia_core::TransformOrientation::Global);
    assert!((global.x_axis - Vec3::X).length() < 1e-5);
    assert_eq!(
        b.state.project.project.revision_id(),
        revision,
        "orientação é sessão, não documento"
    );
    assert!(petunia_core::TransformOrientation::from_offered_id("cursor").is_none());
}
