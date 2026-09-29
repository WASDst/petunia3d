// Floating and pinned state of Inspector sections.
// Fase 1 delivers the state model only: pure transitions, persistence mapping
// and bridge helpers live here; Slint markup arrives in Fase 2.
// Estado flutuante e fixado das seções do Inspector.
// A Fase 1 entrega só o modelo de estado: transições puras, mapeamento de
// persistência e helpers do bridge vivem aqui; o markup Slint chega na Fase 2.

use petunia_config::{InspectorSectionId, SectionLayout, UserPreferences};

/// Index into [`SectionLayouts`], in canonical Inspector order.
/// Índice em [`SectionLayouts`], na ordem canônica do Inspector.
pub const fn section_index(id: InspectorSectionId) -> usize {
    match id {
        InspectorSectionId::Parts => 0,
        InspectorSectionId::Transform => 1,
        InspectorSectionId::Material => 2,
        InspectorSectionId::Object => 3,
        InspectorSectionId::Modifiers => 4,
        InspectorSectionId::QuickActions => 5,
    }
}

/// Runtime layouts for all six sections; index with [`section_index`].
/// Layouts em tempo de execução das seis seções; indexar com [`section_index`].
pub type SectionLayouts = [SectionLayout; 6];

/// All sections docked: the pre-float baseline behavior.
/// Todas as seções ancoradas: o comportamento baseline pré-flutuação.
pub fn default_section_layouts() -> SectionLayouts {
    SectionLayouts::default()
}

/// Overlay persisted preferences onto defaults; missing entries stay default.
/// Sobrepõe as preferências persistidas aos padrões; entradas ausentes mantêm o padrão.
pub fn restore_section_layouts(preferences: &UserPreferences) -> SectionLayouts {
    InspectorSectionId::all().map(|id| preferences.section_layout(id))
}

/// Resolve a persisted section id; unknown ids are rejected (fail-safe).
/// Slint only ever sends known ids; anything else is ignored, never panics.
/// Resolve um id persistido de seção; ids desconhecidos são rejeitados (fail-safe).
/// O Slint só envia ids conhecidos; o resto é ignorado, nunca pânico.
pub fn section_id_from_str(id: &str) -> Option<InspectorSectionId> {
    InspectorSectionId::all()
        .into_iter()
        .find(|known| known.as_str() == id)
}

/// Dock (`true`) or float (`false`) one section.
/// Ancora (`true`) ou flutua (`false`) uma seção.
pub fn set_docked(layouts: &mut SectionLayouts, id: InspectorSectionId, docked: bool) {
    layouts[section_index(id)].docked = docked;
}

/// Abre (`true`) ou fecha (`false`) uma seção (ex.: recolhe na régua de pílulas).
pub fn set_open(layouts: &mut SectionLayouts, id: InspectorSectionId, open: bool) {
    layouts[section_index(id)].open = open;
}

/// Alterna estado aberto/fechado de uma seção.
pub fn toggle_open(layouts: &mut SectionLayouts, id: InspectorSectionId) -> bool {
    let idx = section_index(id);
    layouts[idx].open = !layouts[idx].open;
    layouts[idx].open
}

/// Move a floating card; coordinates go through [`SectionLayout::sanitized`].
/// Move um card flutuante; coordenadas passam por [`SectionLayout::sanitized`].
pub fn move_floating(layouts: &mut SectionLayouts, id: InspectorSectionId, x: f32, y: f32) {
    let current = &layouts[section_index(id)];
    layouts[section_index(id)] = SectionLayout {
        x,
        y,
        ..current.clone()
    }
    .sanitized();
}

/// Pin a section open: it ignores collapse-all and panel collapse.
/// Fixa uma seção aberta: ela ignora recolher-tudo e recolhimento do painel.
pub fn set_pin_open(layouts: &mut SectionLayouts, id: InspectorSectionId, pin_open: bool) {
    layouts[section_index(id)].pin_open = pin_open;
}

/// Pin a section to an asset UUID text (`None` follows the selection).
/// Validation (length, emptiness) goes through [`SectionLayout::sanitized`].
/// Fixa uma seção ao texto de UUID de um asset (`None` segue a seleção).
/// Validação (tamanho, vazio) passa por [`SectionLayout::sanitized`].
pub fn set_pinned_asset(
    layouts: &mut SectionLayouts,
    id: InspectorSectionId,
    asset: Option<String>,
) {
    let current = &layouts[section_index(id)];
    layouts[section_index(id)] = SectionLayout {
        pinned_asset: asset,
        ..current.clone()
    }
    .sanitized();
}

/// Effective open state: pinned-open sections ignore collapse-all.
/// The open query is a closure over [`InspectorSectionId`] (not a positional
/// array) so a caller can never silently pass flags in the wrong order.
/// Estado aberto efetivo: seções com pin aberto ignoram recolher-tudo.
/// A consulta de aberto é um closure sobre [`InspectorSectionId`] (não um array
/// posicional) para que o chamador nunca passe flags na ordem errada em silêncio.
pub fn is_effectively_open(
    layouts: &SectionLayouts,
    is_open: impl Fn(InspectorSectionId) -> bool,
    id: InspectorSectionId,
) -> bool {
    is_open(id) || layouts[section_index(id)].pin_open
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PlaceholderViewport, SlintUiBridge, UiIntent};
    use petunia_core::AppState;

    fn indexed() -> SectionLayouts {
        default_section_layouts()
    }

    fn hermetic_bridge(name: &str) -> (SlintUiBridge<PlaceholderViewport>, std::path::PathBuf) {
        let path =
            std::env::temp_dir().join(format!("petunia-sections-{name}-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.preferences_path_override = Some(path.clone());
        (bridge, path)
    }

    #[test]
    fn section_index_covers_all_six_sections_distinctly() {
        let mut seen = [false; 6];
        for id in InspectorSectionId::all() {
            let index = section_index(id);
            assert!(index < 6);
            assert!(!seen[index], "duplicate index for {id:?}");
            seen[index] = true;
        }
        assert!(seen.iter().all(|used| *used));
    }

    #[test]
    fn defaults_keep_every_section_docked() {
        let layouts = indexed();
        for id in InspectorSectionId::all() {
            assert!(
                layouts[section_index(id)].docked,
                "{id:?} must start docked"
            );
            assert_eq!(layouts[section_index(id)], SectionLayout::default());
        }
    }

    #[test]
    fn restore_overlays_persisted_entries_onto_defaults() {
        let mut preferences = UserPreferences::default();
        preferences.set_section_layout(
            InspectorSectionId::Material,
            SectionLayout {
                docked: false,
                x: 200.0,
                y: 100.0,
                pin_open: false,
                pinned_asset: None,
                open: true,
            },
        );
        let layouts = restore_section_layouts(&preferences);
        assert!(!layouts[section_index(InspectorSectionId::Material)].docked);
        assert!(layouts[section_index(InspectorSectionId::Parts)].docked);
    }

    #[test]
    fn move_clamps_coordinates_through_sanitized() {
        let mut layouts = indexed();
        move_floating(&mut layouts, InspectorSectionId::Object, -20.0, 99999.0);
        let moved = &layouts[section_index(InspectorSectionId::Object)];
        assert_eq!((moved.x, moved.y), (0.0, 8192.0));
        move_floating(&mut layouts, InspectorSectionId::Object, f32::NAN, 50.0);
        let moved = &layouts[section_index(InspectorSectionId::Object)];
        assert_eq!((moved.x, moved.y), (12.0, 50.0));
    }

    #[test]
    fn drag_writes_to_disk_once_on_commit() {
        let (mut bridge, path) = hermetic_bridge("drag-commit");
        let events = 500usize;
        let start = std::time::Instant::now();
        for step in 0..events {
            bridge.move_section_float(
                InspectorSectionId::Material,
                40.0 + (step % 200) as f32,
                80.0,
            );
        }
        let drag_elapsed = start.elapsed();

        assert!(
            !path.exists(),
            "arrastar não pode gravar preferências antes do commit"
        );
        let layout = bridge.section_layouts[section_index(InspectorSectionId::Material)].clone();
        assert_eq!(layout.x, 40.0 + ((events - 1) % 200) as f32);
        assert_eq!(layout.y, 80.0);

        bridge.commit_section_float();
        let persisted = UserPreferences::load_from_path(&path).expect("preferências gravadas");
        let saved = persisted
            .section_layout(InspectorSectionId::Material)
            .clone();
        assert_eq!((saved.x, saved.y), (layout.x, layout.y));

        eprintln!(
            "PROBE drag path: {events} eventos em {drag_elapsed:?} = {:.2} us/evento (antes: 133.5 us/evento com I/O)",
            drag_elapsed.as_secs_f64() * 1e6 / events as f64
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn pin_open_survives_collapse_all() {
        let mut layouts = indexed();
        let collapsed = |_: InspectorSectionId| false;
        assert!(!is_effectively_open(
            &layouts,
            collapsed,
            InspectorSectionId::Parts
        ));
        set_pin_open(&mut layouts, InspectorSectionId::Parts, true);
        assert!(is_effectively_open(
            &layouts,
            collapsed,
            InspectorSectionId::Parts
        ));
        assert!(!is_effectively_open(
            &layouts,
            collapsed,
            InspectorSectionId::Transform
        ));
        let all_open = |_: InspectorSectionId| true;
        assert!(is_effectively_open(
            &layouts,
            all_open,
            InspectorSectionId::Transform
        ));
    }

    #[test]
    fn section_view_model_exposes_canonical_order() {
        let bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let vm = bridge.view_model();
        let ids: Vec<&str> = vm
            .section_states
            .iter()
            .map(|state| state.id.as_str())
            .collect();
        assert_eq!(
            ids,
            [
                "parts",
                "transform",
                "material",
                "object",
                "modifiers",
                "quick_actions"
            ]
        );
        assert!(vm.section_states.iter().all(|state| !state.pin_open));
    }

    #[test]
    fn section_intents_mutate_state_and_persist_hermetically() {
        let (mut bridge, path) = hermetic_bridge("intents");
        bridge.apply(UiIntent::SetSectionDocked {
            section: InspectorSectionId::Material,
            docked: false,
        });
        bridge.apply(UiIntent::MoveSectionFloat {
            section: InspectorSectionId::Material,
            x: 200.0,
            y: -30.0,
        });
        bridge.apply(UiIntent::SetSectionPinOpen {
            section: InspectorSectionId::Material,
            pin_open: true,
        });
        bridge.apply(UiIntent::SetSectionPinnedAsset {
            section: InspectorSectionId::Material,
            asset: Some("01234567-89ab-cdef-0123-456789abcdef".to_string()),
        });
        let material = &bridge.section_layouts[section_index(InspectorSectionId::Material)];
        assert!(!material.docked);
        assert_eq!((material.x, material.y), (200.0, 0.0));
        assert!(material.pin_open);
        assert!(material.pinned_asset.is_some());
        let reloaded = UserPreferences::load_from_path(&path).unwrap();
        assert_eq!(
            reloaded.section_layout(InspectorSectionId::Material),
            *material
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn section_persist_cost_stays_flat_per_mutation() {
        // Evidence for keeping auto-persist on every intent (including drag
        // moves): 200 atomic saves must stay far below any interactive budget.
        // The bound is deliberately generous to avoid flaky timing asserts.
        // Evidência para manter auto-persist em todo intent (incluindo arrastos):
        // 200 gravações atômicas devem ficar longe de qualquer orçamento interativo.
        // O limite é propositalmente folgado para não criar assert flaky de tempo.
        let (mut bridge, path) = hermetic_bridge("cost");
        let start = std::time::Instant::now();
        for index in 0..200 {
            bridge.apply(UiIntent::MoveSectionFloat {
                section: InspectorSectionId::Material,
                x: index as f32,
                y: index as f32,
            });
        }
        let elapsed = start.elapsed();
        let per_save_us = elapsed.as_micros() / 200;
        eprintln!("section persist: 200 saves in {elapsed:?} ({per_save_us}µs/save)");
        assert!(
            elapsed.as_secs() < 10,
            "persist regressed: 200 saves took {elapsed:?}"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn toggle_all_sections_skips_pinned_open() {
        let (mut bridge, _path) = hermetic_bridge("toggle-all");
        assert_eq!(
            bridge.toggle_all_sections([true; 6]),
            [false, false, false, false, false, false]
        );
        bridge.apply(UiIntent::SetSectionPinOpen {
            section: InspectorSectionId::Parts,
            pin_open: true,
        });
        assert_eq!(
            bridge.toggle_all_sections([true; 6]),
            [true, false, false, false, false, false]
        );
        assert_eq!(
            bridge.toggle_all_sections([false, true, false, true, false, true]),
            [false, true, true, true, true, true]
        );
        let _ = std::fs::remove_file(&_path);
    }

    #[test]
    fn unknown_section_ids_are_rejected() {
        assert_eq!(section_id_from_str("nope"), None);
        assert_eq!(section_id_from_str(""), None);
        for id in InspectorSectionId::all() {
            assert_eq!(section_id_from_str(id.as_str()), Some(id));
        }
    }

    #[test]
    fn settings_persist_keeps_section_layouts() {
        // Regression guard for the wipe vector: settings saves must carry the
        // section layouts instead of resetting them to empty defaults.
        // Guarda de regressão do vetor de wipe: saves de settings devem carregar
        // os layouts em vez de zerá-los para os padrões vazios.
        let (mut bridge, path) = hermetic_bridge("settings-wipe");
        bridge.apply(UiIntent::SetSectionDocked {
            section: InspectorSectionId::Material,
            docked: false,
        });
        bridge.state.ui.invert_vertical_drag = true;
        crate::callbacks::persist_user_preferences(&mut bridge);
        let reloaded = UserPreferences::load_from_path(&path).unwrap();
        assert!(!reloaded.section_layout(InspectorSectionId::Material).docked);
        assert!(reloaded.invert_vertical_drag);
        let _ = std::fs::remove_file(&path);
    }

    fn two_asset_bridge() -> (SlintUiBridge<PlaceholderViewport>, String, String) {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.state.project.assets[0].name = "First".to_string();
        bridge
            .state
            .project
            .add("Second", petunia_core::Mesh::cube(1.0));
        let first = bridge.state.project.assets[0].id.to_string();
        let second = bridge.state.project.assets[1].id.to_string();
        (bridge, first, second)
    }

    #[test]
    fn pinned_object_section_shows_pinned_asset_with_active_fallback() {
        // Note: `add` makes the new asset active, so Second is active here.
        // Nota: `add` ativa o asset novo, então Second está ativo aqui.
        let (mut bridge, first, second) = two_asset_bridge();
        assert_eq!(bridge.view_model().object_name, "Second");
        bridge.apply(UiIntent::SetSectionPinnedAsset {
            section: InspectorSectionId::Object,
            asset: Some(first.clone()),
        });
        let vm = bridge.view_model();
        assert_eq!(vm.object_name, "First");
        assert_eq!(vm.object_id, first);
        bridge.apply(UiIntent::SetSectionPinnedAsset {
            section: InspectorSectionId::Object,
            asset: Some("not-a-uuid".to_string()),
        });
        let vm = bridge.view_model();
        assert_eq!(vm.object_name, "Second");
        assert_eq!(vm.object_id, second);
    }

    #[test]
    fn pinned_modifiers_act_on_displayed_stack() {
        // Second is active; the modifier lives on First (non-active), so only
        // owner search can reach it. / Second está ativo; o modifier vive em
        // First (não-ativo), então só a busca por dono o alcança.
        let (mut bridge, first, _second) = two_asset_bridge();
        bridge.state.project.assets[0]
            .modifiers
            .push(petunia_project::ModifierInstance::mirror(0, 0.001));
        let modifier_id = bridge.state.project.assets[0].modifiers[0].id.to_string();
        bridge.apply(UiIntent::SetSectionPinnedAsset {
            section: InspectorSectionId::Modifiers,
            asset: Some(first),
        });
        assert_eq!(bridge.view_model().modifier_rows.len(), 1);
        assert!(bridge.set_modifier_enabled(&modifier_id, false));
        assert!(!bridge.state.project.assets[0].modifiers[0].enabled);
    }

    #[test]
    fn pinned_material_resolves_asset_material() {
        // Slot 0 holds Other; Second owns PinnedMat. Display must follow the
        // pin, not the active slot. / Slot 0 tem Other; Second detém PinnedMat.
        // A exibição deve seguir o pin, não o slot ativo.
        let (mut bridge, _first, second) = two_asset_bridge();
        bridge
            .state
            .project
            .project
            .add_material(petunia_project::Material::new("Other".to_string()));
        let material_id = bridge
            .state
            .project
            .project
            .add_material(petunia_project::Material::new("PinnedMat".to_string()));
        bridge.state.project.assets[1].material_id = Some(material_id);
        bridge.active_material_slot = 0;
        bridge.apply(UiIntent::SetSectionPinnedAsset {
            section: InspectorSectionId::Material,
            asset: Some(second),
        });
        assert_eq!(bridge.view_model().material_name, "PinnedMat");
    }

    #[test]
    fn add_modifier_targets_pinned_asset() {
        // Pinned to First (non-active): creation must land there, not on Second.
        // Fixado em First (não-ativo): a criação deve ir para lá, não Second.
        let (mut bridge, first, _second) = two_asset_bridge();
        bridge.apply(UiIntent::SetSectionPinnedAsset {
            section: InspectorSectionId::Modifiers,
            asset: Some(first),
        });
        assert!(bridge.add_modifier("mirror"));
        assert_eq!(bridge.state.project.assets[0].modifiers.len(), 1);
        assert!(bridge.state.project.assets[1].modifiers.is_empty());
    }

    #[test]
    fn restore_section_layouts_loads_persisted_entries() {
        let (mut bridge, path) = hermetic_bridge("restore");
        let mut preferences = UserPreferences::default();
        preferences.set_section_layout(
            InspectorSectionId::Object,
            SectionLayout {
                docked: false,
                ..SectionLayout::default()
            },
        );
        bridge.restore_section_layouts(&preferences);
        assert!(!bridge.section_layouts[section_index(InspectorSectionId::Object)].docked);
        assert!(bridge.section_layouts[section_index(InspectorSectionId::Parts)].docked);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn pinned_asset_rejects_oversized_ids() {
        let mut layouts = indexed();
        set_pinned_asset(
            &mut layouts,
            InspectorSectionId::Transform,
            Some("x".repeat(65)),
        );
        assert_eq!(
            layouts[section_index(InspectorSectionId::Transform)].pinned_asset,
            None
        );
        set_pinned_asset(
            &mut layouts,
            InspectorSectionId::Transform,
            Some("01234567-89ab-cdef-0123-456789abcdef".to_string()),
        );
        assert!(
            layouts[section_index(InspectorSectionId::Transform)]
                .pinned_asset
                .is_some()
        );
    }
}
