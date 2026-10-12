//! Preferências, tema, idioma, Home, recuperação e keymap.

use crate::callbacks::*;

pub(super) fn sync_settings(window: &PetuniaSlintShell, vm: &ShellViewModel) {
    window.set_recovery_open(vm.recovery_open);
    window.set_recovery_title(vm.recovery_title.as_str().into());
    window.set_recovery_body(vm.recovery_body.as_str().into());
    window.set_recovery_detail(vm.recovery_detail.as_str().into());
    window.set_recovery_recover(vm.recovery_recover.as_str().into());
    window.set_recovery_keep(vm.recovery_keep.as_str().into());
    window.set_recovery_discard(vm.recovery_discard.as_str().into());
    let theme_entries: Vec<ThemeEntry> = vm
        .themes
        .iter()
        .map(|theme| ThemeEntry {
            id: theme.id.as_str().into(),
            name: theme.name.as_str().into(),
            active: theme.active,
        })
        .collect();
    window.set_themes(theme_entries.as_slice().into());
    if vm.keymap_revision >= 0 && i64::from(window.get_keymap_revision()) != vm.keymap_revision {
        window.set_keymap_revision(vm.keymap_revision as i32);
        let profiles: Vec<KeymapProfileEntry> = vm
            .keymap_snapshot
            .profiles
            .iter()
            .map(|p| KeymapProfileEntry {
                id: p.id.as_str().into(),
                name: p.name.as_str().into(),
                custom: p.custom,
                active: p.active,
            })
            .collect();
        crate::refresh::set_model_if_changed(window.get_keymap_profiles(), profiles, |model| {
            window.set_keymap_profiles(model)
        });
        let rows: Vec<KeymapActionEntry> = vm
            .keymap_snapshot
            .rows
            .iter()
            .map(|r| KeymapActionEntry {
                action: r.action.as_str().into(),
                label: r.label.as_str().into(),
                shortcut: r.shortcut.as_str().into(),
                conflict: r.conflict,
                capturing: r.capturing,
            })
            .collect();
        crate::refresh::set_model_if_changed(window.get_keymap_rows(), rows, |model| {
            window.set_keymap_rows(model)
        });
    }
    window.set_tool_activation(vm.tool_activation.as_str().into());
    window.set_invert_vertical_drag(vm.invert_vertical_drag);
    window.set_colorblind_axes(vm.colorblind_axes);
    window.set_reduced_motion(vm.reduced_motion);
    window
        .global::<crate::Motion>()
        .set_reduced(vm.reduced_motion);
    window.set_double_tap_interval_ms(vm.double_tap_interval_ms);
    window.set_click_move_click(vm.click_move_click);
    window.set_show_tool_labels(vm.show_tool_labels);
    window.set_workplane_prefer_ground(vm.workplane_prefer_ground);
    window.set_studio_light_follows_camera(vm.studio_light_follows_camera);
    window.set_viewport_matcap(vm.viewport_matcap);
    window.set_viewport_ambient_occlusion(vm.viewport_ambient_occlusion);
    window.set_multiselection_measure_tag(vm.multiselection_measure_tag);
    window.set_active_language(vm.active_language.as_str().into());
    crate::tr::install(window, &vm.active_language);
    window.set_ui_scale(vm.ui_scale);
    apply_ui_scale(window, vm.ui_scale);
    window.set_icon_theme(vm.icon_theme.as_str().into());
    window
        .global::<crate::PetuniaIcons>()
        .set_filled(vm.icon_theme == "filled");
    window.set_settings_visible(vm.settings_visible);
    window.set_command_search_visible(vm.command_search_visible);
    window.set_active_keymap_id(vm.active_keymap_id.as_str().into());
    window.set_pointer_precision_mask(vm.pointer_precision_mask);
    window.set_pointer_snap_mask(vm.pointer_snap_mask);
    window.set_pointer_extend_mask(vm.pointer_extend_mask);
    window.set_pointer_subtract_mask(vm.pointer_subtract_mask);
    window.set_pointer_pan_mask(vm.pointer_pan_mask);
    window.set_pointer_cursor_place_mask(vm.pointer_cursor_place_mask);
    window.set_pointer_adjust_mask(vm.pointer_adjust_mask);
    window.set_pointer_loop_mask(vm.pointer_loop_mask);
    window.set_pointer_orbit_left_mask(vm.pointer_orbit_left_mask);
    window.set_pointer_paint_straight_mask(vm.pointer_paint_straight_mask);
    window.set_pointer_clone_source_mask(vm.pointer_clone_source_mask);
    window.set_pointer_decal_scale_mask(vm.pointer_decal_scale_mask);
    window.set_pointer_decal_rotate_mask(vm.pointer_decal_rotate_mask);

    theme::apply_theme(window, &vm.current_theme);
    if let Some(rgb) = vm.accent_rgb {
        theme::apply_accent(window, rgb);
    }
    window.set_accent_color_hex(vm.accent_color_hex.as_str().into());
    window.set_home_open(vm.home_open);
    window.set_show_home_on_start(vm.show_home_on_start);
    let recents: Vec<crate::RecentProject> = vm
        .recent_projects
        .iter()
        .map(|(name, path)| crate::RecentProject {
            name: name.as_str().into(),
            path: path.as_str().into(),
        })
        .collect();
    window.set_recent_projects(std::rc::Rc::new(slint::VecModel::from(recents)).into());
    let [
        title,
        subtitle,
        new,
        open,
        recover,
        recent,
        no_recent,
        settings,
        show_on_start,
    ] = vm.home_texts.clone();
    window.set_home_texts(crate::HomeTexts {
        title: title.into(),
        subtitle: subtitle.into(),
        new: new.into(),
        open: open.into(),
        recover: recover.into(),
        recent: recent.into(),
        no_recent: no_recent.into(),
        settings: settings.into(),
        show_on_start: show_on_start.into(),
    });
}
