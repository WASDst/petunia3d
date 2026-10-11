//! Outliner, partes, prefabs, renomear e menus de contexto.

use crate::callbacks::*;

pub(super) fn sync_outliner(window: &PetuniaSlintShell, vm: &ShellViewModel) {
    window.set_asset_library_visible(vm.asset_library_visible);
    window.set_isolate_active(vm.isolate_active);

    window.set_active_object_title(vm.active_object_title.as_str().into());
    window.set_active_object_details(vm.active_object_details.as_str().into());
    window.set_object_has_selection(vm.object_has_selection);
    window.set_object_id(vm.object_id.as_str().into());
    window.set_object_name(vm.object_name.as_str().into());
    window.set_object_visible(vm.object_visible);
    window.set_object_locked(vm.object_locked);
    window.set_object_verts(vm.object_verts);
    window.set_object_faces(vm.object_faces);
    window.set_object_tris(vm.object_tris);
    window.set_object_selection(vm.object_selection.as_str().into());
    window.set_object_material(vm.object_material.as_str().into());
    window.set_object_modifier_count(vm.object_modifier_count);
    window.set_scene_stats(vm.scene_stats.as_str().into());
    window.set_current_theme(vm.current_theme.as_str().into());

    let to_scene_item = |item: &SceneItemModel| SceneItem {
        id: item.id.as_str().into(),
        name: item.name.as_str().into(),
        visible: item.visible,
        locked: item.locked,
        selected: item.selected,
        active: item.active,
        verts: item.verts as i32,
        tris: item.tris as i32,
        has_custom_color: item.has_custom_color,
        custom_color: slint::Color::from_rgb_u8(
            item.custom_color_rgb[0],
            item.custom_color_rgb[1],
            item.custom_color_rgb[2],
        ),
    };
    let scene_items: Vec<SceneItem> = vm.scene_items.iter().map(to_scene_item).collect();
    crate::refresh::set_model_if_changed(window.get_scene_items(), scene_items, |model| {
        window.set_scene_items(model)
    });
    let parts_items: Vec<SceneItem> = vm.parts_items.iter().map(to_scene_item).collect();
    crate::refresh::set_model_if_changed(window.get_parts_items(), parts_items, |model| {
        window.set_parts_items(model)
    });
    window.set_parts_query(vm.parts_query.as_str().into());
    window.set_parts_selected_only(vm.parts_selected_only);
    window.set_parts_sort_by_name(vm.parts_sort_by_name);
    window.set_parts_row_height(vm.parts_row_height);
    sync_inspector_pane_layout(window, vm.inspector_pane_layout);
    let prefab_items: Vec<PrefabItem> = vm.prefab_items.iter().map(to_prefab_item).collect();
    crate::refresh::set_model_if_changed(window.get_prefab_items(), prefab_items, |model| {
        window.set_prefab_items(model)
    });
    window.set_asset_only_favorites(vm.asset_only_favorites);
    window.set_split_enabled(vm.split_enabled);
    window.set_paint_pip(vm.paint_pip);
    window.set_split_preset(vm.split_preset.as_str().into());
    window.set_asset_query(vm.asset_query.as_str().into());
    window.set_asset_sort_by_name(vm.asset_sort_by_name);
    window.set_asset_thumbnail_size(vm.asset_thumbnail_size);
    window.set_show_face_orientation(vm.show_face_orientation);
    window.set_show_uv_checker(vm.show_uv_checker);
    window.set_add_menu_open(vm.add_menu_open);
    window.set_rename_active(vm.rename_active);
    window.set_rename_value(vm.rename_value.as_str().into());
    window.set_context_menu_open(vm.context_menu_open);
    window.set_context_menu_x(vm.context_menu_x);
    window.set_context_menu_y(vm.context_menu_y);
    window.set_context_menu_title(vm.context_menu_title.as_str().into());
    window.set_context_menu_mode(vm.context_menu_mode.as_str().into());
    window.set_context_menu_visible(vm.context_menu_visible);
    window.set_context_menu_locked(vm.context_menu_locked);
    window.set_context_menu_isolated(vm.context_menu_isolated);
    window.set_context_menu_can_move_up(vm.context_menu_can_move_up);
    window.set_context_menu_can_move_down(vm.context_menu_can_move_down);
    window.set_boolean_operand_name(vm.boolean_operand_name.as_str().into());
    window.set_boolean_ready(vm.boolean_ready);
    window.set_boolean_keep_parts(vm.boolean_keep_parts);
    window.set_boolean_cleanup(vm.boolean_cleanup);
    window.set_menu_open(vm.menu_open.as_str().into());
    window.set_pivot_menu_open(vm.pivot_menu_open);
    let to_entries = |items: &[MenuEntryModel]| -> Vec<MenuEntry> {
        items
            .iter()
            .map(|item| MenuEntry {
                id: item.id.as_str().into(),
                label: item.label.as_str().into(),
                shortcut: item.shortcut.as_str().into(),
                separator: item.separator,
                disabled: item.disabled,
                checked: item.checked,
            })
            .collect()
    };
    window.set_menu_file_items(to_entries(&vm.menu_file_items).as_slice().into());
    window.set_menu_edit_items(to_entries(&vm.menu_edit_items).as_slice().into());
    window.set_menu_view_items(to_entries(&vm.menu_view_items).as_slice().into());
    window.set_menu_window_items(to_entries(&vm.menu_window_items).as_slice().into());
    window.set_selection_color(slint::Color::from_rgb_u8(
        vm.selection_rgb[0],
        vm.selection_rgb[1],
        vm.selection_rgb[2],
    ));
    window.set_active_selection_color(slint::Color::from_rgb_u8(
        vm.active_selection_rgb[0],
        vm.active_selection_rgb[1],
        vm.active_selection_rgb[2],
    ));
    window.set_selection_color_hex(vm.selection_color_hex.as_str().into());
    window.set_selection_thickness(vm.selection_thickness);
    window.set_inspector_width(vm.inspector_width);
    window.set_asset_library_height(vm.asset_library_height);
    window.set_scene_drawer_visible(vm.scene_drawer_visible);
}
