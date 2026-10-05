//! Medição opcional dos caminhos por evento da viewport.
//!
//! `PETUNIA_FRAME_TIMING=1` registra a duração de cada etapa (view model,
//! frame, sincronização, hover, gesto) e imprime p50/p95/máximo a cada 120
//! amostras por etapa. Desligado, o custo é uma leitura de `OnceLock`.
//! Os mesmos trechos têm escopos `puffin` (ligados com
//! `PETUNIA_PROFILE=1`), compatíveis com os renderers WGPU/GL.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

const WINDOW: usize = 120;

fn timing_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        let profile = std::env::var_os("PETUNIA_PROFILE").is_some_and(|v| v != "0");
        if profile {
            puffin::set_scopes_on(true);
        }
        std::env::var_os("PETUNIA_FRAME_TIMING").is_some_and(|v| v != "0")
    })
}

thread_local! {
    static SAMPLES: RefCell<HashMap<&'static str, Vec<Duration>>> = RefCell::new(HashMap::new());
}

/// Mede `work` sob o nome `stage` quando a medição está ligada.
pub(crate) fn measure<T>(stage: &'static str, work: impl FnOnce() -> T) -> T {
    if !timing_enabled() {
        return work();
    }
    let start = Instant::now();
    let value = work();
    record(stage, start.elapsed());
    value
}

fn record(stage: &'static str, elapsed: Duration) {
    SAMPLES.with(|samples| {
        let mut samples = samples.borrow_mut();
        let entry = samples.entry(stage).or_default();
        entry.push(elapsed);
        if entry.len() < WINDOW {
            return;
        }
        entry.sort_unstable();
        let at = |q: f32| entry[((entry.len() - 1) as f32 * q) as usize];
        eprintln!(
            "[petunia timing] {stage}: p50 {:.2} ms · p95 {:.2} ms · max {:.2} ms ({} amostras)",
            at(0.5).as_secs_f64() * 1e3,
            at(0.95).as_secs_f64() * 1e3,
            at(1.0).as_secs_f64() * 1e3,
            entry.len()
        );
        entry.clear();
    });
}

/// Fecha um quadro do `puffin` (no-op com os escopos desligados).
pub(crate) fn end_frame() {
    if puffin::are_scopes_on() {
        puffin::GlobalProfiler::lock().new_frame();
    }
}
