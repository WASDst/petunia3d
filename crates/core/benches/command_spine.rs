//! Comparative command-spine benchmarks. No wall-clock asserts.

use criterion::{Criterion, criterion_group, criterion_main};
use petunia_core::{
    AddPrimitiveCmd, AppState, ExtrudeSelectedCmd, PrimitiveKind, ProjectService, SelectAllCmd,
};

fn primed() -> AppState {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);
    state.set_edit_mode(petunia_core::EditMode::Edit);
    state.dispatch(&SelectAllCmd).unwrap();
    state
}

fn bench_dispatch(c: &mut Criterion) {
    c.bench_function("command/add_primitive", |b| {
        b.iter(|| {
            let mut state = AppState::new("en");
            ProjectService::new_project(&mut state);
            std::hint::black_box(state.dispatch(&AddPrimitiveCmd::new(PrimitiveKind::Cube)))
        })
    });
    c.bench_function("command/extrude", |b| {
        b.iter(|| {
            let mut state = primed();
            std::hint::black_box(state.dispatch(&ExtrudeSelectedCmd {
                dist: 0.5,
                ..Default::default()
            }))
        })
    });
    c.bench_function("command/checkpoint_metrics", |b| {
        b.iter(|| {
            let mut state = primed();
            state
                .dispatch(&ExtrudeSelectedCmd {
                    dist: 0.25,
                    ..Default::default()
                })
                .unwrap();
            std::hint::black_box(state.project.history_metrics())
        })
    });
}

criterion_group!(benches, bench_dispatch);
criterion_main!(benches);
