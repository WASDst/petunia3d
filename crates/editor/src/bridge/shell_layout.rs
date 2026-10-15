//! Coordenação do layout/sections do shell, sem picking nem mutações geométricas.
use crate::overlay::{OverlayEntry, OverlayId, OverlayKind};
use crate::{PetuniaViewport, SectionStateModel, SlintUiBridge, inspector_layout, section_layout};

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
    /// O botão "»" esconde a coluna inteira; as seções abertas do acordeão
    /// ficam como estavam para quando a coluna voltar.
    pub fn collapse_inspector(&mut self) {
        self.inspector_visible = false;
        self.state.mark_dirty();
    }

    /// Alterna estado aberto/fechado de uma seção do Inspector (persistido).
    pub fn toggle_section_open(&mut self, section: petunia_config::InspectorSectionId) -> bool {
        let open = section_layout::toggle_open(&mut self.section_layouts, section);
        self.persist_section_layouts();
        self.state.mark_dirty();
        open
    }
}
