//! Um traço pinta ao longo de todo o caminho, não só no primeiro dab.
//!
//! Os testes do bridge conferiam Undo e dirty state, mas nenhum contava
//! pixels: a QA no app real (2026-10-11) mostrou um traço de 40 atualizações
//! alterando só a área de um dab.

use glam::Vec3;
use petunia_core::AppState;
use petunia_module_paint::PaintModule;

fn texture(state: &AppState) -> Vec<u8> {
    state.project.assets[state.project.active]
        .texture
        .as_ref()
        .map(|canvas| canvas.pixels.to_vec())
        .unwrap_or_default()
}

fn changed(before: &[u8], after: &[u8]) -> usize {
    if before.len() != after.len() {
        return after.chunks(4).filter(|p| *p != &after[..4]).count();
    }
    before
        .chunks(4)
        .zip(after.chunks(4))
        .filter(|(a, b)| a != b)
        .count()
}

/// Pontos em linha reta sobre a face `face`, do centro rumo a um vértice.
fn hits_along_face(state: &AppState, face: usize, count: usize) -> Vec<(usize, Vec3)> {
    let mesh = state.project.active_mesh().expect("cubo padrão");
    let points: Vec<Vec3> = mesh.faces[face]
        .verts
        .iter()
        .map(|&v| mesh.verts[v as usize].vec())
        .collect();
    let center = points.iter().copied().sum::<Vec3>() / points.len() as f32;
    let corner = points[0];
    (0..count)
        .map(|i| {
            let t = i as f32 / count as f32 * 0.8;
            (face, center + (corner - center) * t)
        })
        .collect()
}

#[test]
fn every_call_of_a_stroke_lays_paint() {
    let mut state = AppState::new("en");
    state.paint_color = [1.0, 0.0, 0.0];
    let mut settings = state.brush_settings();
    settings.size_px = 12.8;
    let hits = hits_along_face(&state, 1, 24);

    state.begin_paint_stroke();
    let start = texture(&state);
    assert!(PaintModule::paint_mesh_3d_batch_with_settings(
        &mut state,
        &hits[..1],
        settings,
        false
    ));
    let first = texture(&state);
    let first_dab = changed(&start, &first);
    assert!(first_dab > 0, "o primeiro dab pinta");

    // As chamadas seguintes (uma por atualização do ponteiro) continuam o traço.
    for chunk in hits[1..].chunks(4) {
        PaintModule::paint_mesh_3d_batch_with_settings(&mut state, chunk, settings, false);
    }
    state.finish_paint_stroke(false);
    let painted = changed(&start, &texture(&state));
    assert!(
        painted > first_dab * 4,
        "o traço inteiro pinta: {painted} pixels, primeiro dab {first_dab}"
    );
}
