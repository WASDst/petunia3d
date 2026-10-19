//! Shell v2 (ADR-C14): invariantes de layout, foco e estado por workspace, tema,
//! densidade e largura estreita. Roda offscreen e salva capturas PNG em
//! `target/shell-captures/` para inspeção visual ("olhar o resultado").
use petunia_ui_cxxqt::widgets::PetuniaWidgetsApp;
use serde_json::Value;
use std::{ffi::CString, path::PathBuf};

fn isolated_dirs() -> PathBuf {
    let root = std::env::temp_dir().join(format!("petunia-shell-v2-{}", std::process::id()));
    for name in ["config", "data", "cache"] {
        std::fs::create_dir_all(root.join(name)).expect("diretório isolado");
    }
    // Antes de criar o QApplication e o editor: nenhuma preferência real é lida ou gravada.
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", root.join("config"));
        std::env::set_var("XDG_DATA_HOME", root.join("data"));
        std::env::set_var("XDG_CACHE_HOME", root.join("cache"));
    }
    root
}

#[test]
fn shell_v2_regions_layout_focus_and_theme() {
    let root = isolated_dirs();
    let captures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/shell-captures");
    std::fs::create_dir_all(&captures).expect("diretório de capturas");
    let _app = PetuniaWidgetsApp::new();
    unsafe extern "C" {
        fn petunia_shell_harness(directory: *const std::ffi::c_char) -> *const std::ffi::c_char;
    }
    let dir = CString::new(captures.to_string_lossy().as_ref()).expect("caminho sem NUL");
    let raw = unsafe { std::ffi::CStr::from_ptr(petunia_shell_harness(dir.as_ptr())) }
        .to_str()
        .expect("relatório UTF-8")
        .to_owned();
    let report: Value = serde_json::from_str(&raw).expect("relatório JSON");
    // Evidência ao lado das capturas: detalhes também das verificações aprovadas.
    let _ = std::fs::write(captures.join("report.json"), &raw);
    let checks = report["checks"].as_array().expect("lista de verificações");
    assert!(
        checks.len() >= 40,
        "cobertura mínima do harness: {} verificações",
        checks.len()
    );
    // As verificações da GUI completa (F7, V03, U07) precisam ter rodado de fato.
    let names: Vec<&str> = checks.iter().filter_map(|c| c["name"].as_str()).collect();
    let missing: Vec<&str> = [
        "file:menu-offers-the-model-import",
        "paint:native-property-sections",
        "paint:panels-have-no-missing-bindings",
        "paint:layer-mask-added-by-the-panel",
        "paint:sub-effect-added-by-the-panel",
        "paint:sub-effect-removed-by-the-panel",
        "paint:gradient-layer-from-the-effects-panel",
        "paint:gradient-stop-added-by-the-panel",
        "paint:isolate-ghost-toggles-the-mode",
        "paint:isolate-ghost-restores",
        "surface:2d-hides-3d-view",
        "surface:3d-hides-canvas",
        "surface:mode-persists-per-workspace",
        "surface:split-shows-both",
        "uv:native-property-sections",
        "uv:structure-lists-islands",
        "uv:selecting-island-row-selects-uv",
        "animate:native-property-sections",
        "timeline:play-button-toggles-playback",
        "viewport:shift-f10-opens-context-menu",
        "viewport:context-menu-has-no-raw-ids",
        "viewport:context-menu-returns-focus",
        "viewport:gizmo-visible-for-the-camera-test",
        "viewport:gizmo-follows-the-orbit-mid-drag",
        "viewport:orbit-moved-the-gizmo",
        "viewport:gizmo-follows-the-pan-mid-drag",
        "viewport:pan-moved-the-gizmo",
        "viewport:gizmo-arrow-is-reachable-for-the-drag-test",
        "viewport:object-moved-by-the-gizmo-drag",
        "viewport:gizmo-follows-the-object-while-dragging",
        "gizmo:select-shows-move-rotate-and-scale-handles",
        "gizmo:transform-shows-move-rotate-and-scale-handles",
        "gizmo:move-rotate-and-scale-keep-their-own-handles",
        "nodes:trackpad-pans-both-axes",
        "nodes:tilted-wheel-pans-horizontally",
        "nodes:shift-wheel-pans-vertically",
        "nodes:wheel-zooms-under-the-cursor",
        "nodes:right-drag-pans-without-a-menu",
        "nodes:space-drag-pans",
        "nodes:legend-stays-fixed-while-panning",
        "nodes:box-start-is-empty-space",
        "nodes:box-selects-every-node",
        "nodes:click-on-empty-space-narrows-the-selection",
        "nodes:ctrl-click-adds-to-the-selection",
        "nodes:dragging-moves-the-whole-selection",
        "nodes:ctrl-a-selects-every-node",
        "nodes:ctrl-g-groups-the-selected-nodes",
        "nodes:rename-group-by-keyboard",
        "nodes:double-click-renames-and-escape-cancels",
        "nodes:ctrl-shift-g-ungroups-keeping-the-nodes",
        "navcube:opacity-preference",
        "measure:first-click-marks-and-preview-follows",
        "measure:second-click-stores-the-measurement",
        "measure:clear-removes-the-stored-measurements",
        "scrub:label-drag-adjusts-a-number-field",
        "scrub:value-drag-adjusts-a-number-field",
        "scrub:shift-multiplies-the-step",
        "scrub:click-without-drag-edits-the-field",
        "scrub:editing-field-is-not-scrubbed",
        "scrub:spin-boxes-scrub-by-label-and-value",
        "nodes:view-can-leave-the-graph",
        "nodes:f-frames-everything",
        "nodes:fit-button-frames-everything",
        "nodes:home-frames-everything",
        "uv:box-selects-every-island",
        "structure:double-click-renames-a-group",
        "uv:pressing-a-selected-island-keeps-the-multi-selection",
        "uv:dragging-moves-every-selected-island",
        "uv:click-without-drag-narrows-to-the-clicked-item",
        "uv:click-on-empty-space-clears",
        "paint:canvas-button-toggles-the-uv-wireframe",
        "paint:canvas-button-reflects-the-state",
        "paint:alt-down-reorders-layer",
        "paint:alt-up-restores-order",
        "paint:h-toggles-layer-visibility",
        "paint:delete-removes-layer-not-object",
        "uv:plus-minus-zoom",
        "uv:ctrl-arrow-pans-without-editing",
        "uv:arrow-nudges-selection",
        "uv:home-frames",
        // Refinamento de uso (ADR-C15).
        "rail:only-transform-cursor-and-create",
        "header:icon-only-with-tooltips",
        "structure:ctrl-g-groups-the-selection",
        "structure:drop-on-group-joins-it",
        "structure:drop-on-empty-area-leaves-the-group",
        "structure:drop-above-reorders",
        "structure:isolate-keeps-only-the-selection",
        "structure:leaving-isolation-restores-visibility",
        "structure:ctrl-shift-g-ungroups",
        "align:popover-has-grid-and-distribute",
        "align:enabled-with-three-objects",
        "align:disabled-with-one-object",
        "pivot:marker-published",
        "pivot:edit-mode-reflected",
        "gizmo:size-and-thickness-preferences",
        "navcube:face-hits-map-to-presets",
        "navcube:corner-hit-gives-its-direction",
        "navcube:edge-hit-gives-the-two-faces",
        "navcube:coincident-corner-prefers-the-nearer-one",
        "navcube:home-and-menu-hit",
        "navcube:paints-pixels",
        // Onda B (2026-10-19)
        "select-policy:defaults-to-automatic",
        "select-policy:header-offers-three-policies",
        "select-policy:automatic-selects-through-in-xray",
        "select-policy:visible-only-wins-over-xray",
        "select-policy:state-is-announced",
        "orientation:header-offers-global-local-normal-view",
        "orientation:local-reaches-the-session",
        "select-gizmo:rotate-handles-can-be-hidden",
        "panel:number-fields-use-the-dense-height",
        "color:brush-panel-is-compact",
        "color:create-palette-reaches-the-document",
        "color:plus-adds-the-color-to-the-palette",
        "color:rename-palette",
        "color:delete-palette",
        "color:hex-sets-the-color",
        "color:box-mode",
        "color:rail-swatch-visible-in-paint",
        "color:brush-form-embeds-the-color-panel",
        "brush:2d-cursor-maps-to-the-view",
        "brush:3d-cursor-maps-to-the-texture",
        "brush:2d-cursor-maps-back-to-the-same-view-point",
        "brush:mirror-clears",
        "color:rail-swatch-hidden-outside-paint",
        "dialog:frameless-with-own-title-bar",
        "dialog:title-bar-drag-moves",
        "dialog:right-edge-drag-resizes",
        "dialog:keyboard-moves-and-resizes",
        "dialog:geometry-is-remembered",
        "hud:visible-for-a-new-primitive",
        "hud:card-drags-inside-the-view",
        "hud:double-click-returns-to-the-corner",
        "grid:y-axis-hidden-by-default",
        "grid:header-button-reflects-the-grid",
        "grid:header-button-toggles",
        "grid:axis-checkbox-toggles-the-axis",
        "grid:size-and-tile-are-configurable",
        "grid:invalid-tile-is-refused",
        "lock:transform-fields-have-padlocks",
        "lock:request-locks-the-channel",
        "lock:field-shows-the-padlock-closed",
        "lock:typed-value-on-locked-axis-is-refused",
        "lock:clicking-the-padlock-unlocks",
        "lock:alt-l-asks-to-lock",
        "pie:opens-with-the-selection-domains",
        "pie:direction-picks-the-item",
        "pie:items-are-named-buttons",
        "pie:number-key-activates",
        "pie:closes-after-choosing",
        "pie:add-offers-eight-primitives",
        "pie:escape-cancels-without-effect",
        "pie:shortcut-in-the-viewport-opens-it",
        "pie:list-fallback-has-the-same-items",
        "preview:extrude-publishes-anchor-arrow-and-value",
        "preview:gone-after-cancel",
    ]
    .into_iter()
    .filter(|name| !names.contains(name))
    .collect();
    assert!(missing.is_empty(), "verificações ausentes: {missing:?}");
    let failed: Vec<String> = checks
        .iter()
        .filter(|c| c["pass"] != true)
        .map(|c| format!("{} {}", c["name"], c["detail"].as_str().unwrap_or("")))
        .collect();
    let _ = std::fs::remove_dir_all(root);
    assert!(
        failed.is_empty(),
        "verificações reprovadas:\n{}",
        failed.join("\n")
    );
}
