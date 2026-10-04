//! Consultas por evento da viewport (`criterion`): picking, oclusão, seleção
//! por caixa e snap numa malha de ~50 mil triângulos — o orçamento do
//! capítulo 46 (≤ 16,7 ms por quadro até 50 mil triângulos).

use criterion::{Criterion, criterion_group, criterion_main};
use glam::{Vec2, Vec3};
use petunia_core::viewport_query::ViewportSceneQuery;
use petunia_core::{AppState, ScreenSnapQuery, SelectionDomain, SnapMask, snap_screen};
use petunia_mesh::Mesh;

/// Esfera de 180 × 140 quads (~50 mil triângulos) enquadrada pela câmera.
fn dense_state() -> AppState {
    let mut state = AppState::default();
    state.project.assets.clear();
    state.project.assets.push(petunia_project::Asset::new(
        "Dense",
        Mesh::sphere_low(180, 140, 1.0),
    ));
    state.project.active = 0;
    state.frame_all();
    state
}

fn bench_scene_query(c: &mut Criterion) {
    let state = dense_state();
    c.bench_function("viewport/scene_query_build_50k", |b| {
        b.iter(|| std::hint::black_box(ViewportSceneQuery::new(&state.project.project)))
    });
    let scene = ViewportSceneQuery::new(&state.project.project);
    let camera = state.session.camera.clone();
    c.bench_function("viewport/nearest_object_50k", |b| {
        b.iter(|| std::hint::black_box(scene.nearest_object(&camera, [0.05, 0.02])))
    });
    let points: Vec<Vec3> = state.project.project.assets[0]
        .mesh
        .verts
        .iter()
        .step_by(97)
        .map(|v| v.vec())
        .collect();
    c.bench_function("viewport/point_visible_256_of_50k", |b| {
        b.iter(|| {
            points
                .iter()
                .take(256)
                .filter(|p| scene.point_visible(&camera, **p))
                .count()
        })
    });
}

fn bench_box_select(c: &mut Criterion) {
    let mut state = dense_state();
    state.set_selection_domain(SelectionDomain::Vertex);
    c.bench_function("viewport/box_select_vertices_50k", |b| {
        b.iter(|| {
            state.select_viewport_box([-0.5, -0.5], [0.5, 0.5], false, false);
            std::hint::black_box(state.project.project.assets[0].mesh.verts.len())
        })
    });
}

fn bench_snap(c: &mut Criterion) {
    let state = dense_state();
    let mesh = &state.project.project.assets[0].mesh;
    let camera = state.session.camera.clone();
    c.bench_function("snap/screen_snap_50k", |b| {
        b.iter(|| {
            std::hint::black_box(snap_screen(&ScreenSnapQuery {
                camera: &camera,
                viewport_pixels: Vec2::new(1280.0, 800.0),
                cursor_pixels: Vec2::new(660.0, 410.0),
                mesh: Some(mesh),
                moving: &[],
                anchor: None,
                grid: None,
                radius_pixels: 12.0,
                mask: SnapMask::ALL,
                xray: false,
                reference: None,
                angle_step_degrees: None,
            }))
        })
    });
}

criterion_group!(benches, bench_scene_query, bench_box_select, bench_snap);
criterion_main!(benches);
