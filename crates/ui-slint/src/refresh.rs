//! Refresh da janela em eventos de alta frequência (arrasto e hover).
//!
//! A sincronização completa — `view_model()`, cerca de 550 propriedades e as
//! listas do Inspector/Outliner — custava O(UI inteira) por movimento do
//! mouse. Em arrasto e hover só a viewport e seus overlays mudam a cada
//! evento; o restante da janela é sincronizado no máximo a cada
//! [`FULL_SYNC_INTERVAL`], com uma passada final garantida por timer para a
//! janela nunca ficar com um estado intermediário.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use slint::{ComponentHandle, Model, ModelRc, VecModel};

use crate::callbacks::{sync_viewport_overlays, sync_window_properties};
use crate::perf;
use crate::{PetuniaSlintShell, PetuniaViewport, SlintUiBridge};

/// Intervalo mínimo entre duas sincronizações completas durante um gesto.
pub(crate) const FULL_SYNC_INTERVAL: Duration = Duration::from_millis(50);

/// Estado do limitador, compartilhado pelas closures de eventos contínuos.
#[derive(Default)]
pub(crate) struct RefreshThrottle {
    last_full: Cell<Option<Instant>>,
    pending: Cell<bool>,
}

impl RefreshThrottle {
    fn due(&self) -> bool {
        self.last_full
            .get()
            .is_none_or(|at| at.elapsed() >= FULL_SYNC_INTERVAL)
    }

    fn remaining(&self) -> Duration {
        self.last_full.get().map_or(Duration::ZERO, |at| {
            FULL_SYNC_INTERVAL.saturating_sub(at.elapsed())
        })
    }
}

/// Sincronização completa: view model, propriedades, listas e frame.
pub(crate) fn refresh_full<V: PetuniaViewport>(
    window: &PetuniaSlintShell,
    bridge: &mut SlintUiBridge<V>,
    throttle: &RefreshThrottle,
) {
    let vm = perf::measure("view_model", || bridge.view_model());
    let frame = perf::measure("render_viewport", || bridge.render_viewport());
    perf::measure("sync_window_properties", || {
        sync_window_properties(window, &vm)
    });
    if let Some(frame) = frame {
        window.set_viewport_image(frame);
    }
    throttle.last_full.set(Some(Instant::now()));
    perf::end_frame();
}

/// Evento contínuo: frame e overlays agora; o resto da janela, limitado.
pub(crate) fn refresh_interactive<V: PetuniaViewport + 'static>(
    window: &PetuniaSlintShell,
    shared: &Arc<Mutex<SlintUiBridge<V>>>,
    bridge: &mut SlintUiBridge<V>,
    throttle: &Rc<RefreshThrottle>,
) {
    if throttle.due() {
        refresh_full(window, bridge, throttle);
        return;
    }
    if let Some(frame) = perf::measure("render_viewport", || bridge.render_viewport()) {
        window.set_viewport_image(frame);
    }
    perf::measure("sync_viewport_overlays", || {
        sync_viewport_overlays(window, bridge)
    });
    perf::end_frame();
    if throttle.pending.replace(true) {
        return;
    }
    let weak = window.as_weak();
    let shared = Arc::clone(shared);
    let deferred = Rc::clone(throttle);
    slint::Timer::single_shot(throttle.remaining(), move || {
        deferred.pending.set(false);
        let Some(window) = weak.upgrade() else {
            return;
        };
        if let Ok(mut bridge) = shared.lock() {
            refresh_full(&window, &mut bridge, &deferred);
        }
    });
}

/// Troca o modelo só quando o conteúdo mudou.
///
/// Um `VecModel` novo faz o Slint destruir e recriar todas as linhas da
/// lista (Outliner, Parts, tags de medida...), mesmo com dados idênticos.
pub(crate) fn set_model_if_changed<T>(
    current: ModelRc<T>,
    items: Vec<T>,
    set: impl FnOnce(ModelRc<T>),
) where
    T: Clone + PartialEq + 'static,
{
    let unchanged = current.row_count() == items.len()
        && items
            .iter()
            .enumerate()
            .all(|(row, item)| current.row_data(row).as_ref() == Some(item));
    if !unchanged {
        set(Rc::new(VecModel::from(items)).into());
    }
}
