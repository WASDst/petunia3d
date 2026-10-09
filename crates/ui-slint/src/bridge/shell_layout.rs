//! Coordenação do layout/sections do shell, sem picking nem mutações geométricas.
use crate::callbacks::sync_window_properties;
use crate::overlay::{OverlayEntry, OverlayId, OverlayKind};
use crate::{
    PetuniaSlintShell, PetuniaViewport, SectionStateModel, SlintUiBridge, inspector_layout,
    section_layout,
};
use slint::ComponentHandle;
use std::sync::{Arc, Mutex};

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub fn set_add_menu_open(&mut self, open: bool) {
        self.add_menu_open = open;
        if open {
            self.overlays.push(OverlayEntry {
                id: OverlayId::AddMenu,
                kind: OverlayKind::Popover,
                pinned: false,
                dismiss_on_escape: true,
                dismiss_on_click_away: true,
            });
        } else {
            self.overlays.remove(OverlayId::AddMenu);
        }
    }
}

pub(crate) fn connect_callbacks<V: PetuniaViewport + 'static>(
    window: &PetuniaSlintShell,
    bridge: Arc<Mutex<SlintUiBridge<V>>>,
) {
    let parts_size_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_parts_row_height_changed(move |height| {
        if let Ok(mut bridge) = parts_size_bridge.lock() {
            bridge.set_parts_row_height(height);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let inspector_width_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_inspector_width_changed(move |width| {
        if let Ok(mut bridge) = inspector_width_bridge.lock()
            && bridge.set_inspector_width(width)
        {
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let asset_height_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_asset_library_height_changed(move |height| {
        if let Ok(mut bridge) = asset_height_bridge.lock()
            && bridge.set_asset_library_height(height)
        {
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let toggle_all_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_all_sections(move || {
        if let (Ok(bridge), Some(window)) = (toggle_all_bridge.lock(), window_weak.upgrade()) {
            let open = [
                window.get_model_parts_open(),
                window.get_model_transform_open(),
                window.get_model_material_open(),
                window.get_model_object_open(),
                window.get_model_modifiers_open(),
                window.get_quick_actions_section_open(),
            ];
            let next = bridge.toggle_all_sections(open);
            window.set_model_parts_open(next[0]);
            window.set_model_transform_open(next[1]);
            window.set_model_material_open(next[2]);
            window.set_model_object_open(next[3]);
            window.set_model_modifiers_open(next[4]);
            window.set_quick_actions_section_open(next[5]);
        }
    });
    let section_pin_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_section_pin_open_toggled(move |id| {
        if let (Ok(mut bridge), Some(window)) = (section_pin_bridge.lock(), window_weak.upgrade()) {
            if let Some(section) = crate::section_layout::section_id_from_str(id.as_str()) {
                let idx = crate::section_layout::section_index(section);
                let current_pin = bridge.section_layouts[idx].pin_open;
                bridge.set_section_pin_open(section, !current_pin);
                bridge.set_section_open(section, !current_pin);
            }
            sync_window_properties(&window, &bridge.view_model());
        }
    });

    let section_toggle_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_section_open_toggled(move |id| {
        if let (Ok(mut bridge), Some(window)) =
            (section_toggle_bridge.lock(), window_weak.upgrade())
        {
            if let Some(section) = crate::section_layout::section_id_from_str(id.as_str()) {
                bridge.toggle_section_open(section);
            }
            sync_window_properties(&window, &bridge.view_model());
        }
    });

    let collapse_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_inspector_collapse_requested(move || {
        if let (Ok(mut bridge), Some(window)) = (collapse_bridge.lock(), window_weak.upgrade()) {
            bridge.collapse_inspector();
            sync_window_properties(&window, &bridge.view_model());
        }
    });

    let layout_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_inspector_split_ratio_set(move |ratio| {
        if let (Ok(mut bridge), Some(window)) = (layout_bridge.lock(), window_weak.upgrade()) {
            if bridge.set_inspector_structure_ratio(ratio) {
                sync_inspector_pane_layout(&window, bridge.inspector_pane_layout());
            }
        }
    });

    let layout_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_inspector_pane_collapsed_set(move |pane, collapsed| {
        let Ok(pane) = crate::inspector_layout::InspectorPane::try_from(pane) else {
            return;
        };
        if let (Ok(mut bridge), Some(window)) = (layout_bridge.lock(), window_weak.upgrade()) {
            if bridge.set_inspector_pane_collapsed(pane, collapsed) {
                sync_inspector_pane_layout(&window, bridge.inspector_pane_layout());
            }
        }
    });

    let section_pill_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_section_pill_clicked(move |id| {
        if let (Ok(mut bridge), Some(window)) = (section_pill_bridge.lock(), window_weak.upgrade())
        {
            if let Some(section) = crate::section_layout::section_id_from_str(id.as_str()) {
                let idx = crate::section_layout::section_index(section);
                let current_open = bridge.section_layouts[idx].open;
                bridge.set_section_pin_open(section, !current_open);
                bridge.set_section_open(section, !current_open);
            }
            sync_window_properties(&window, &bridge.view_model());
        }
    });
}

/// Só projeções de apresentação: resize não reconstrói o view model de domínio por pixel.
pub(crate) fn sync_inspector_pane_layout(
    window: &PetuniaSlintShell,
    layout: crate::inspector_layout::InspectorPaneLayout,
) {
    window.set_inspector_structure_ratio(layout.structure_ratio);
    window.set_inspector_structure_collapsed(layout.structure_collapsed);
    window.set_inspector_properties_collapsed(layout.properties_collapsed);
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub fn inspector_pane_layout(&self) -> inspector_layout::InspectorPaneLayout {
        self.inspector_layout.get(self.state.workspace)
    }

    pub fn set_inspector_structure_ratio(&mut self, ratio: f32) -> bool {
        self.inspector_layout.set_ratio(self.state.workspace, ratio)
    }

    pub fn set_inspector_pane_collapsed(
        &mut self,
        pane: inspector_layout::InspectorPane,
        collapsed: bool,
    ) -> bool {
        self.inspector_layout
            .set_collapsed(self.state.workspace, pane, collapsed)
    }

    pub fn set_parts_row_height(&mut self, size: f32) -> bool {
        if !size.is_finite() {
            return false;
        }
        let size = size.clamp(28.0, 44.0);
        if (self.parts_row_height - size).abs() < f32::EPSILON {
            return false;
        }
        self.parts_row_height = size;
        true
    }

    /// Redimensiona o dock de contexto pelo divisor vertical.
    ///
    /// Layout é estado de apresentação: não marca o documento como alterado.
    pub fn set_inspector_width(&mut self, width: f32) -> bool {
        self.state.ui.set_right_width(width)
    }

    /// Redimensiona a Asset Library pelo divisor horizontal.
    pub fn set_asset_library_height(&mut self, height: f32) -> bool {
        self.state.ui.set_shell_asset_library_height(height)
    }

    /// Dock or float one Inspector section, then persist.
    /// Ancora ou flutua uma seção do Inspector e persiste.
    pub fn set_section_docked(
        &mut self,
        section: petunia_config::InspectorSectionId,
        docked: bool,
    ) -> bool {
        section_layout::set_docked(&mut self.section_layouts, section, docked);
        self.persist_section_layouts();
        true
    }

    /// Move a floating card in memory (coordinates sanitized), without I/O.
    ///
    /// A drag emits one event per pointer move; persisting here would write the
    /// preferences file dozens of times per second. The UI commits once on
    /// pointer release through [`Self::commit_section_float`].
    /// Move um card flutuante em memória (coordenadas sanitizadas), sem I/O.
    ///
    /// O arraste emite um evento por movimento do ponteiro; persistir aqui
    /// gravaria o arquivo dezenas de vezes por segundo. A UI confirma uma vez
    /// ao soltar o ponteiro via [`Self::commit_section_float`].
    pub fn move_section_float(
        &mut self,
        section: petunia_config::InspectorSectionId,
        x: f32,
        y: f32,
    ) -> bool {
        section_layout::move_floating(&mut self.section_layouts, section, x, y);
        true
    }

    /// Persist section layouts once, after a drag or any other live edit.
    /// Persiste os layouts uma vez, depois do arraste ou de outra edição viva.
    pub fn commit_section_float(&mut self) {
        self.persist_section_layouts();
    }

    /// Pin a section open (ignores collapse-all), then persist.
    /// Fixa uma seção aberta (ignora recolher-tudo) e persiste.
    pub fn set_section_pin_open(
        &mut self,
        section: petunia_config::InspectorSectionId,
        pin_open: bool,
    ) -> bool {
        section_layout::set_pin_open(&mut self.section_layouts, section, pin_open);
        self.persist_section_layouts();
        true
    }

    /// Pin a section to an asset (`None` follows selection), then persist.
    /// Fixa uma seção a um asset (`None` segue a seleção) e persiste.
    pub fn set_section_pinned_asset(
        &mut self,
        section: petunia_config::InspectorSectionId,
        asset: Option<String>,
    ) -> bool {
        section_layout::set_pinned_asset(&mut self.section_layouts, section, asset);
        self.persist_section_layouts();
        true
    }

    /// Collapse-all toggle honoring pinned-open sections: if every section is
    /// open, close the unpinned ones; otherwise open them. Pinned sections
    /// stay open either way. Takes and returns open flags in canonical order.
    /// Alternador de recolher-tudo respeitando pins: se tudo está aberto, fecha
    /// as não-fixadas; senão, abre-as. Fixadas seguem abertas. Recebe e devolve
    /// flags de aberto em ordem canônica.
    pub fn toggle_all_sections(&self, open: [bool; 6]) -> [bool; 6] {
        let close_all = open.iter().all(|flag| *flag);
        let mut next = open;
        for id in petunia_config::InspectorSectionId::all() {
            if !self.section_layouts[section_layout::section_index(id)].pin_open {
                next[section_layout::section_index(id)] = !close_all;
            }
        }
        next
    }

    /// Presentation snapshot of the six section layouts, in canonical order.
    /// Snapshot de apresentação dos seis layouts, em ordem canônica.
    pub fn section_state_models(&self) -> Vec<SectionStateModel> {
        petunia_config::InspectorSectionId::all()
            .iter()
            .map(|id| {
                let layout = &self.section_layouts[section_layout::section_index(*id)];
                SectionStateModel {
                    id: id.as_str().to_string(),
                    pin_open: layout.pin_open,
                    open: layout.open,
                }
            })
            .collect()
    }

    /// Abre ou fecha uma seção do Inspector (persistido).
    pub fn set_section_open(&mut self, section: petunia_config::InspectorSectionId, open: bool) {
        section_layout::set_open(&mut self.section_layouts, section, open);
        self.persist_section_layouts();
        self.state.mark_dirty();
    }

    /// "Recolher Inspector" (plano de UI F4, D1): fecha e solta o pin de todas
    /// as seções, devolvendo o shell ao trilho de pílulas. É um comando
    /// explícito do usuário, por isso ignora o pin — diferente do recolhimento
    /// automático do peek, que o pin protege (ADR 005 §5).
    pub fn collapse_inspector(&mut self) {
        for id in petunia_config::InspectorSectionId::all() {
            section_layout::set_open(&mut self.section_layouts, id, false);
            section_layout::set_pin_open(&mut self.section_layouts, id, false);
        }
        self.persist_section_layouts();
    }

    /// Alterna estado aberto/fechado de uma seção do Inspector (persistido).
    pub fn toggle_section_open(&mut self, section: petunia_config::InspectorSectionId) -> bool {
        let open = section_layout::toggle_open(&mut self.section_layouts, section);
        self.persist_section_layouts();
        self.state.mark_dirty();
        open
    }
}
