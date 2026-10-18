//! Typed projection of shared editor DTOs to the Qt transport.
use serde_json::{Map, Value, json};
fn animatekeyframemodel(vm: &petunia_editor::animate::AnimateKeyframeModel) -> Value {
    let mut obj = Map::new();
    obj.insert("time".into(), json!(&vm.time));
    obj.insert("frame".into(), json!(&vm.frame));
    obj.insert("normalized_pos".into(), json!(&vm.normalized_pos));
    obj.insert("selected".into(), json!(&vm.selected));
    obj.insert("interp_kind".into(), json!(&vm.interp_kind));
    Value::Object(obj)
}
fn animatelayermodel(vm: &petunia_editor::animate::AnimateLayerModel) -> Value {
    let mut obj = Map::new();
    obj.insert("id".into(), json!(&vm.id));
    obj.insert("name".into(), json!(&vm.name));
    obj.insert("weight".into(), json!(&vm.weight));
    obj.insert("is_additive".into(), json!(&vm.is_additive));
    obj.insert("enabled".into(), json!(&vm.enabled));
    obj.insert("is_tweak".into(), json!(&vm.is_tweak));
    obj.insert("selected".into(), json!(&vm.selected));
    Value::Object(obj)
}
fn animateparam(vm: &petunia_editor::animate::AnimateParam) -> Value {
    let mut obj = Map::new();
    obj.insert("key".into(), json!(&vm.key));
    obj.insert("label".into(), json!(&vm.label));
    obj.insert("tip".into(), json!(&vm.tip));
    obj.insert("minimum".into(), json!(&vm.minimum));
    obj.insert("maximum".into(), json!(&vm.maximum));
    obj.insert("value".into(), json!(&vm.value));
    obj.insert("value_label".into(), json!(&vm.value_label));
    obj.insert("advanced".into(), json!(&vm.advanced));
    obj.insert("choices".into(), json!(&vm.choices));
    obj.insert("choice_index".into(), json!(&vm.choice_index));
    Value::Object(obj)
}
fn animaterow(vm: &petunia_editor::animate::AnimateRow) -> Value {
    let mut obj = Map::new();
    obj.insert("id".into(), json!(&vm.id));
    obj.insert("label".into(), json!(&vm.label));
    obj.insert("detail".into(), json!(&vm.detail));
    obj.insert("icon".into(), json!(&vm.icon));
    obj.insert("available".into(), json!(&vm.available));
    obj.insert("selected".into(), json!(&vm.selected));
    Value::Object(obj)
}
fn animatetexts(vm: &petunia_editor::animate::AnimateTexts) -> Value {
    let mut obj = Map::new();
    obj.insert("workspace_label".into(), json!(&vm.workspace_label));
    obj.insert("workspace_tip".into(), json!(&vm.workspace_tip));
    obj.insert("title_picker".into(), json!(&vm.title_picker));
    obj.insert("title_creature".into(), json!(&vm.title_creature));
    obj.insert("title_motion".into(), json!(&vm.title_motion));
    obj.insert("title_style".into(), json!(&vm.title_style));
    obj.insert("add_creature".into(), json!(&vm.add_creature));
    obj.insert("empty_creature".into(), json!(&vm.empty_creature));
    obj.insert("empty_motion".into(), json!(&vm.empty_motion));
    obj.insert("rig_error".into(), json!(&vm.rig_error));
    obj.insert("advanced".into(), json!(&vm.advanced));
    obj.insert("show_bones".into(), json!(&vm.show_bones));
    obj.insert("stepped".into(), json!(&vm.stepped));
    obj.insert("stepped_tip".into(), json!(&vm.stepped_tip));
    obj.insert("root_motion".into(), json!(&vm.root_motion));
    obj.insert("root_motion_tip".into(), json!(&vm.root_motion_tip));
    obj.insert("apply_now".into(), json!(&vm.apply_now));
    obj.insert("apply_now_tip".into(), json!(&vm.apply_now_tip));
    obj.insert("keep_live".into(), json!(&vm.keep_live));
    obj.insert("keep_live_tip".into(), json!(&vm.keep_live_tip));
    obj.insert("duplicate".into(), json!(&vm.duplicate));
    obj.insert("remove".into(), json!(&vm.remove));
    obj.insert("playhead".into(), json!(&vm.playhead));
    obj.insert("play".into(), json!(&vm.play));
    obj.insert("pause".into(), json!(&vm.pause));
    obj.insert("play_tip".into(), json!(&vm.play_tip));
    obj.insert("fit_model".into(), json!(&vm.fit_model));
    obj.insert("linked_model".into(), json!(&vm.linked_model));
    obj.insert("title_bone".into(), json!(&vm.title_bone));
    obj.insert("reset_bone".into(), json!(&vm.reset_bone));
    obj.insert("ik_mode".into(), json!(&vm.ik_mode));
    obj.insert("insert_keyframe".into(), json!(&vm.insert_keyframe));
    obj.insert("delete_keyframe".into(), json!(&vm.delete_keyframe));
    obj.insert("interp_linear".into(), json!(&vm.interp_linear));
    obj.insert("interp_step".into(), json!(&vm.interp_step));
    obj.insert("interp_cubic".into(), json!(&vm.interp_cubic));
    obj.insert("clear_track".into(), json!(&vm.clear_track));
    obj.insert("title_timeline".into(), json!(&vm.title_timeline));
    obj.insert("title_curves".into(), json!(&vm.title_curves));
    obj.insert("title_dope_sheet".into(), json!(&vm.title_dope_sheet));
    obj.insert("channel_trans_x".into(), json!(&vm.channel_trans_x));
    obj.insert("channel_trans_y".into(), json!(&vm.channel_trans_y));
    obj.insert("channel_trans_z".into(), json!(&vm.channel_trans_z));
    obj.insert("channel_rot_x".into(), json!(&vm.channel_rot_x));
    obj.insert("channel_rot_y".into(), json!(&vm.channel_rot_y));
    obj.insert("channel_rot_z".into(), json!(&vm.channel_rot_z));
    obj.insert("channel_scale".into(), json!(&vm.channel_scale));
    obj.insert("title_layers".into(), json!(&vm.title_layers));
    obj.insert("add_layer".into(), json!(&vm.add_layer));
    obj.insert("add_tweak".into(), json!(&vm.add_tweak));
    obj.insert("blend_override".into(), json!(&vm.blend_override));
    obj.insert("blend_additive".into(), json!(&vm.blend_additive));
    obj.insert("layer_weight".into(), json!(&vm.layer_weight));
    obj.insert("onion_skin".into(), json!(&vm.onion_skin));
    obj.insert("motion_trails".into(), json!(&vm.motion_trails));
    Value::Object(obj)
}
fn animatetrackmodel(vm: &petunia_editor::animate::AnimateTrackModel) -> Value {
    let mut obj = Map::new();
    obj.insert("bone_id".into(), json!(&vm.bone_id));
    obj.insert("bone_name".into(), json!(&vm.bone_name));
    obj.insert("selected".into(), json!(&vm.selected));
    obj.insert(
        "keyframes".into(),
        Value::Array(vm.keyframes.iter().map(animatekeyframemodel).collect()),
    );
    Value::Object(obj)
}
fn animateviewmodel(vm: &petunia_editor::animate::AnimateViewModel) -> Value {
    let mut obj = Map::new();
    obj.insert("available".into(), json!(&vm.available));
    obj.insert("has_creature".into(), json!(&vm.has_creature));
    obj.insert("has_motion".into(), json!(&vm.has_motion));
    obj.insert("rig_error".into(), json!(&vm.rig_error));
    obj.insert("texts".into(), animatetexts(&vm.texts));
    obj.insert(
        "creatures".into(),
        Value::Array(vm.creatures.iter().map(animaterow).collect()),
    );
    obj.insert(
        "creature_presets".into(),
        Value::Array(vm.creature_presets.iter().map(animaterow).collect()),
    );
    obj.insert(
        "catalog".into(),
        Value::Array(vm.catalog.iter().map(animaterow).collect()),
    );
    obj.insert(
        "motions".into(),
        Value::Array(vm.motions.iter().map(animaterow).collect()),
    );
    obj.insert(
        "styles".into(),
        Value::Array(vm.styles.iter().map(animaterow).collect()),
    );
    obj.insert(
        "params".into(),
        Value::Array(vm.params.iter().map(animateparam).collect()),
    );
    obj.insert("playing".into(), json!(&vm.playing));
    obj.insert("progress".into(), json!(&vm.progress));
    obj.insert("time_label".into(), json!(&vm.time_label));
    obj.insert("show_advanced".into(), json!(&vm.show_advanced));
    obj.insert("show_bones".into(), json!(&vm.show_bones));
    obj.insert("stepped".into(), json!(&vm.stepped));
    obj.insert("root_motion".into(), json!(&vm.root_motion));
    obj.insert(
        "root_motion_supported".into(),
        json!(&vm.root_motion_supported),
    );
    obj.insert("bone_commands".into(), json!(&vm.bone_commands));
    obj.insert("joint_commands".into(), json!(&vm.joint_commands));
    obj.insert("selected_bone".into(), json!(&vm.selected_bone));
    obj.insert("selected_bone_name".into(), json!(&vm.selected_bone_name));
    obj.insert(
        "selected_bone_commands".into(),
        json!(&vm.selected_bone_commands),
    );
    obj.insert("ik_mode".into(), json!(&vm.ik_mode));
    obj.insert("can_fit".into(), json!(&vm.can_fit));
    obj.insert("fit_tip".into(), json!(&vm.fit_tip));
    obj.insert("linked_models".into(), json!(&vm.linked_models));
    obj.insert("can_keyframe".into(), json!(&vm.can_keyframe));
    obj.insert(
        "has_keyframe_at_playhead".into(),
        json!(&vm.has_keyframe_at_playhead),
    );
    obj.insert("frame_label".into(), json!(&vm.frame_label));
    obj.insert("current_frame".into(), json!(vm.current_frame));
    obj.insert("total_frames".into(), json!(vm.total_frames));
    obj.insert("current_interp".into(), json!(&vm.current_interp));
    obj.insert(
        "tracks".into(),
        Value::Array(vm.tracks.iter().map(animatetrackmodel).collect()),
    );
    obj.insert("curve_path".into(), json!(&vm.curve_path));
    obj.insert(
        "curve_points".into(),
        Value::Array(vm.curve_points.iter().map(animatekeyframemodel).collect()),
    );
    obj.insert(
        "active_curve_channel".into(),
        json!(&vm.active_curve_channel),
    );
    obj.insert(
        "layers".into(),
        Value::Array(vm.layers.iter().map(animatelayermodel).collect()),
    );
    obj.insert("onion_skin_enabled".into(), json!(&vm.onion_skin_enabled));
    obj.insert(
        "motion_trails_enabled".into(),
        json!(&vm.motion_trails_enabled),
    );
    obj.insert("ghost_commands".into(), json!(&vm.ghost_commands));
    obj.insert(
        "motion_trail_commands".into(),
        json!(&vm.motion_trail_commands),
    );
    Value::Object(obj)
}
fn gizmomodel(vm: &petunia_editor::GizmoModel) -> Value {
    let mut obj = Map::new();
    obj.insert("visible".into(), json!(&vm.visible));
    obj.insert("origin_x".into(), json!(&vm.origin_x));
    obj.insert("origin_y".into(), json!(&vm.origin_y));
    obj.insert("x_commands".into(), json!(&vm.x_commands));
    obj.insert("y_commands".into(), json!(&vm.y_commands));
    obj.insert("z_commands".into(), json!(&vm.z_commands));
    obj.insert("x_end".into(), json!(&vm.x_end));
    obj.insert("y_end".into(), json!(&vm.y_end));
    obj.insert("z_end".into(), json!(&vm.z_end));
    obj.insert("x_label".into(), json!(&vm.x_label));
    obj.insert("y_label".into(), json!(&vm.y_label));
    obj.insert("z_label".into(), json!(&vm.z_label));
    obj.insert("x_arrow_commands".into(), json!(&vm.x_arrow_commands));
    obj.insert("y_arrow_commands".into(), json!(&vm.y_arrow_commands));
    obj.insert("z_arrow_commands".into(), json!(&vm.z_arrow_commands));
    obj.insert("x_scale_commands".into(), json!(&vm.x_scale_commands));
    obj.insert("y_scale_commands".into(), json!(&vm.y_scale_commands));
    obj.insert("z_scale_commands".into(), json!(&vm.z_scale_commands));
    obj.insert("x_rotate_commands".into(), json!(&vm.x_rotate_commands));
    obj.insert("y_rotate_commands".into(), json!(&vm.y_rotate_commands));
    obj.insert("z_rotate_commands".into(), json!(&vm.z_rotate_commands));
    obj.insert("plane_yz_commands".into(), json!(&vm.plane_yz_commands));
    obj.insert("plane_xz_commands".into(), json!(&vm.plane_xz_commands));
    obj.insert("plane_xy_commands".into(), json!(&vm.plane_xy_commands));
    obj.insert("view_roll_commands".into(), json!(&vm.view_roll_commands));
    obj.insert("view_x_commands".into(), json!(&vm.view_x_commands));
    obj.insert("view_y_commands".into(), json!(&vm.view_y_commands));
    obj.insert("view_z_commands".into(), json!(&vm.view_z_commands));
    obj.insert("view_x_end".into(), json!(&vm.view_x_end));
    obj.insert("view_y_end".into(), json!(&vm.view_y_end));
    obj.insert("view_z_end".into(), json!(&vm.view_z_end));
    obj.insert("view_origin_x".into(), json!(&vm.view_origin_x));
    obj.insert("view_origin_y".into(), json!(&vm.view_origin_y));
    obj.insert("cursor_screen".into(), json!(&vm.cursor_screen));
    obj.insert("cursor_visible".into(), json!(&vm.cursor_visible));
    obj.insert("pivot_visible".into(), json!(&vm.pivot_visible));
    obj.insert("pivot_screen".into(), json!(&vm.pivot_screen));
    obj.insert(
        "origin_marker_visible".into(),
        json!(&vm.origin_marker_visible),
    );
    obj.insert(
        "origin_marker_screen".into(),
        json!(&vm.origin_marker_screen),
    );
    obj.insert("pivot_editing".into(), json!(&vm.pivot_editing));
    obj.insert("x_ring_front".into(), json!(&vm.x_ring_front));
    obj.insert("y_ring_front".into(), json!(&vm.y_ring_front));
    obj.insert("z_ring_front".into(), json!(&vm.z_ring_front));
    obj.insert("thickness".into(), json!(&vm.thickness));
    obj.insert("size".into(), json!(&vm.size));
    Value::Object(obj)
}
fn measuretagmodel(vm: &petunia_editor::projection::MeasureTagModel) -> Value {
    let mut obj = Map::new();
    obj.insert("text".into(), json!(&vm.text));
    obj.insert("x".into(), json!(&vm.x));
    obj.insert("y".into(), json!(&vm.y));
    Value::Object(obj)
}
fn menuentrymodel(vm: &petunia_editor::MenuEntryModel) -> Value {
    let mut obj = Map::new();
    obj.insert("id".into(), json!(&vm.id));
    obj.insert("label".into(), json!(&vm.label));
    obj.insert("shortcut".into(), json!(&vm.shortcut));
    obj.insert("separator".into(), json!(&vm.separator));
    obj.insert("disabled".into(), json!(&vm.disabled));
    obj.insert("checked".into(), json!(&vm.checked));
    Value::Object(obj)
}
fn modifierrowmodel(vm: &petunia_editor::ModifierRowModel) -> Value {
    let mut obj = Map::new();
    obj.insert("id".into(), json!(&vm.id));
    obj.insert("title".into(), json!(&vm.title));
    obj.insert("subtitle".into(), json!(&vm.subtitle));
    obj.insert("enabled".into(), json!(&vm.enabled));
    obj.insert("kind".into(), json!(&vm.kind));
    obj.insert("axis".into(), json!(&vm.axis));
    obj.insert(
        "positive_to_negative".into(),
        json!(&vm.positive_to_negative),
    );
    obj.insert("can_move_up".into(), json!(&vm.can_move_up));
    obj.insert("can_move_down".into(), json!(&vm.can_move_down));
    Value::Object(obj)
}
fn painteffectparam(vm: &petunia_editor::PaintEffectParam) -> Value {
    let mut obj = Map::new();
    obj.insert("key".into(), json!(&vm.key));
    obj.insert("label".into(), json!(&vm.label));
    obj.insert("value".into(), json!(&vm.value));
    obj.insert("min".into(), json!(&vm.min));
    obj.insert("max".into(), json!(&vm.max));
    Value::Object(obj)
}
fn paintlayermodel(vm: &petunia_editor::PaintLayerModel) -> Value {
    let mut obj = Map::new();
    obj.insert("id".into(), json!(&vm.id));
    obj.insert("name".into(), json!(&vm.name));
    obj.insert("visible".into(), json!(&vm.visible));
    obj.insert("locked".into(), json!(&vm.locked));
    obj.insert("opacity".into(), json!(&vm.opacity));
    obj.insert("active".into(), json!(&vm.active));
    obj.insert("is_group".into(), json!(&vm.is_group));
    obj.insert("kind_label".into(), json!(&vm.kind_label));
    obj.insert("kind".into(), json!(&vm.kind));
    obj.insert("blend".into(), json!(&vm.blend));
    obj.insert("depth".into(), json!(&vm.depth));
    Value::Object(obj)
}
fn prefabitemmodel(vm: &petunia_editor::PrefabItemModel) -> Value {
    let mut obj = Map::new();
    obj.insert("id".into(), json!(&vm.id));
    obj.insert("name".into(), json!(&vm.name));
    obj.insert("parts".into(), json!(&vm.parts));
    obj.insert("tris".into(), json!(&vm.tris));
    obj.insert("verts".into(), json!(&vm.verts));
    obj.insert("favorite".into(), json!(&vm.favorite));
    obj.insert("instances".into(), json!(&vm.instances));
    obj.insert("tags".into(), json!(&vm.tags));
    obj.insert("has_thumbnail".into(), json!(vm.thumbnail.is_some()));
    Value::Object(obj)
}
fn quickactionmodel(vm: &petunia_editor::QuickActionModel) -> Value {
    let mut obj = Map::new();
    obj.insert("id".into(), json!(&vm.id));
    obj.insert("label".into(), json!(&vm.label));
    obj.insert("enabled".into(), json!(&vm.enabled));
    obj.insert("disabled_reason".into(), json!(&vm.disabled_reason));
    obj.insert("active".into(), json!(&vm.active));
    obj.insert("pinned".into(), json!(&vm.pinned));
    Value::Object(obj)
}
fn referenceslotviewmodel(vm: &petunia_editor::ReferenceSlotViewModel) -> Value {
    let mut obj = Map::new();
    obj.insert("axis".into(), json!(&vm.axis));
    obj.insert("axis_tag".into(), json!(&vm.axis_tag));
    obj.insert("title".into(), json!(&vm.title));
    obj.insert("has_image".into(), json!(&vm.has_image));
    obj.insert("image_name".into(), json!(&vm.image_name));
    obj.insert("dimensions".into(), json!(&vm.dimensions));
    obj.insert("visible".into(), json!(&vm.visible));
    obj.insert("locked".into(), json!(&vm.locked));
    obj.insert("opacity".into(), json!(&vm.opacity));
    obj.insert("size".into(), json!(&vm.size));
    obj.insert("offset".into(), json!(&vm.offset));
    obj.insert("rotation".into(), json!(&vm.rotation));
    obj.insert("fine_open".into(), json!(&vm.fine_open));
    Value::Object(obj)
}
fn sceneitemmodel(vm: &petunia_editor::SceneItemModel) -> Value {
    let mut obj = Map::new();
    obj.insert("id".into(), json!(&vm.id));
    obj.insert("name".into(), json!(&vm.name));
    obj.insert("visible".into(), json!(&vm.visible));
    obj.insert("locked".into(), json!(&vm.locked));
    obj.insert("selected".into(), json!(&vm.selected));
    obj.insert("active".into(), json!(&vm.active));
    obj.insert("verts".into(), json!(&vm.verts));
    obj.insert("tris".into(), json!(&vm.tris));
    obj.insert("has_custom_color".into(), json!(&vm.has_custom_color));
    obj.insert("custom_color_rgb".into(), json!(&vm.custom_color_rgb));
    obj.insert("collection".into(), json!(&vm.collection));
    Value::Object(obj)
}
fn scenegroupmodel(vm: &petunia_editor::SceneGroupModel) -> Value {
    let mut obj = Map::new();
    obj.insert("name".into(), json!(&vm.name));
    obj.insert("count".into(), json!(&vm.count));
    obj.insert("visible".into(), json!(&vm.visible));
    obj.insert("locked".into(), json!(&vm.locked));
    obj.insert("selected".into(), json!(&vm.selected));
    Value::Object(obj)
}
fn sectionstatemodel(vm: &petunia_editor::SectionStateModel) -> Value {
    let mut obj = Map::new();
    obj.insert("id".into(), json!(&vm.id));
    obj.insert("pin_open".into(), json!(&vm.pin_open));
    obj.insert("open".into(), json!(&vm.open));
    Value::Object(obj)
}
fn shellviewmodel(vm: &petunia_editor::ShellViewModel) -> Value {
    let mut obj = Map::new();
    obj.insert("workspace".into(), json!(format!("{:?}", vm.workspace)));
    obj.insert(
        "selection_domain".into(),
        json!(format!("{:?}", vm.selection_domain)),
    );
    obj.insert("inspector_visible".into(), json!(&vm.inspector_visible));
    obj.insert("planar_navigation".into(), json!(&vm.planar_navigation));
    obj.insert("shortcuts".into(), shortcutsmodel(&vm.shortcuts));
    obj.insert("saved".into(), json!(&vm.saved));
    obj.insert("can_undo".into(), json!(&vm.can_undo));
    obj.insert("can_redo".into(), json!(&vm.can_redo));
    obj.insert("active_tool".into(), json!(&vm.active_tool));
    obj.insert("paint_color".into(), json!(&vm.paint_color));
    obj.insert("brush_size".into(), json!(&vm.brush_size));
    obj.insert("brush_opacity".into(), json!(&vm.brush_opacity));
    obj.insert("brush_hardness".into(), json!(&vm.brush_hardness));
    obj.insert("paint_symmetry_x".into(), json!(&vm.paint_symmetry_x));
    obj.insert("paint_symmetry_y".into(), json!(&vm.paint_symmetry_y));
    obj.insert("paint_symmetry_z".into(), json!(&vm.paint_symmetry_z));
    obj.insert("paint_target_vertex".into(), json!(&vm.paint_target_vertex));
    obj.insert(
        "paint_mask_selection".into(),
        json!(&vm.paint_mask_selection),
    );
    obj.insert("isolate_active".into(), json!(&vm.isolate_active));
    obj.insert("status_message".into(), json!(&vm.status_message));
    obj.insert("status_severity".into(), json!(&vm.status_severity));
    obj.insert("selection_summary".into(), json!(&vm.selection_summary));
    obj.insert("position".into(), json!(&vm.position));
    obj.insert("rotation".into(), json!(&vm.rotation));
    obj.insert("scale".into(), json!(&vm.scale));
    obj.insert("cursor_3d".into(), json!(&vm.cursor_3d));
    obj.insert(
        "scene_items".into(),
        Value::Array(vm.scene_items.iter().map(sceneitemmodel).collect()),
    );
    obj.insert(
        "scene_groups".into(),
        Value::Array(vm.scene_groups.iter().map(scenegroupmodel).collect()),
    );
    obj.insert(
        "parts_items".into(),
        Value::Array(vm.parts_items.iter().map(sceneitemmodel).collect()),
    );
    obj.insert("parts_query".into(), json!(&vm.parts_query));
    obj.insert("parts_selected_only".into(), json!(&vm.parts_selected_only));
    obj.insert("parts_sort_by_name".into(), json!(&vm.parts_sort_by_name));
    obj.insert("parts_row_height".into(), json!(&vm.parts_row_height));
    obj.insert(
        "prefab_items".into(),
        Value::Array(vm.prefab_items.iter().map(prefabitemmodel).collect()),
    );
    obj.insert(
        "asset_only_favorites".into(),
        json!(&vm.asset_only_favorites),
    );
    obj.insert("split_enabled".into(), json!(&vm.split_enabled));
    obj.insert("paint_pip".into(), json!(&vm.paint_pip));
    obj.insert("split_preset".into(), json!(&vm.split_preset));
    obj.insert("asset_query".into(), json!(&vm.asset_query));
    obj.insert("asset_sort_by_name".into(), json!(&vm.asset_sort_by_name));
    obj.insert(
        "asset_thumbnail_size".into(),
        json!(&vm.asset_thumbnail_size),
    );
    obj.insert("active_object_title".into(), json!(&vm.active_object_title));
    obj.insert(
        "active_object_details".into(),
        json!(&vm.active_object_details),
    );
    obj.insert(
        "active_material_name".into(),
        json!(&vm.active_material_name),
    );
    obj.insert("scene_stats".into(), json!(&vm.scene_stats));
    obj.insert("uv_stats".into(), json!(&vm.uv_stats));
    obj.insert("current_theme".into(), json!(&vm.current_theme));
    let theme_colors = petunia_config::ThemeRegistry::global()
        .get_theme(&vm.current_theme)
        .map(|t| &t.colors)
        .cloned()
        .unwrap_or_default();
    obj.insert(
        "theme_colors".into(),
        json!({
            "bg_canvas": theme_colors.bg_canvas,
            "bg_header": theme_colors.bg_header,
            "bg_panel": theme_colors.bg_panel,
            "bg_panel_header": theme_colors.bg_panel_header,
            "bg_surface": theme_colors.bg_surface,
            "bg_surface_hover": theme_colors.bg_surface_hover,
            "bg_surface_active": theme_colors.bg_surface_active,
            "text_primary": theme_colors.text_primary,
            "text_secondary": theme_colors.text_secondary,
            "text_muted": theme_colors.text_muted,
            "text_active": theme_colors.text_active,
            "accent_blue": theme_colors.accent_blue,
            "accent_orange": theme_colors.accent_orange,
            "accent_hover": theme_colors.accent_hover,
            "accent_border": theme_colors.accent_border,
            "border_subtle": theme_colors.border_subtle,
            "border_strong": theme_colors.border_strong,
            "border_focus": theme_colors.border_focus,
            "status_info": theme_colors.status_info,
            "status_warning": theme_colors.status_warning,
            "status_error": theme_colors.status_error,
            "status_success": theme_colors.status_success,
        }),
    );
    obj.insert("icon_theme".into(), json!(&vm.icon_theme));
    obj.insert("is_orthographic".into(), json!(&vm.is_orthographic));
    obj.insert("is_wireframe".into(), json!(&vm.is_wireframe));
    obj.insert("shading_mode".into(), json!(&vm.shading_mode));
    obj.insert("xray_opacity".into(), json!(&vm.xray_opacity));
    obj.insert("selection_rgb".into(), json!(&vm.selection_rgb));
    obj.insert(
        "active_selection_rgb".into(),
        json!(&vm.active_selection_rgb),
    );
    obj.insert("selection_thickness".into(), json!(&vm.selection_thickness));
    obj.insert("gizmo_size".into(), json!(&vm.gizmo_size));
    obj.insert("gizmo_thickness".into(), json!(&vm.gizmo_thickness));
    obj.insert("selection_color_hex".into(), json!(&vm.selection_color_hex));
    obj.insert("show_xray".into(), json!(&vm.show_xray));
    obj.insert(
        "show_face_orientation".into(),
        json!(&vm.show_face_orientation),
    );
    obj.insert("show_uv_checker".into(), json!(&vm.show_uv_checker));
    obj.insert(
        "draw_extension_values".into(),
        json!(&vm.draw_extension_values),
    );
    obj.insert("export_padding".into(), json!(&vm.export_padding));
    obj.insert("trace_threshold".into(), json!(&vm.trace_threshold));
    obj.insert(
        "proportional_editing".into(),
        json!(&vm.proportional_editing),
    );
    obj.insert("proportional_radius".into(), json!(&vm.proportional_radius));
    obj.insert(
        "proportional_falloff".into(),
        json!(&vm.proportional_falloff),
    );
    obj.insert("snap_enabled".into(), json!(&vm.snap_enabled));
    obj.insert("snap_target".into(), json!(&vm.snap_target));
    obj.insert(
        "shading_popover_open".into(),
        json!(&vm.shading_popover_open),
    );
    obj.insert(
        "operation_hud_active".into(),
        json!(&vm.operation_hud_active),
    );
    obj.insert("operation_hud_title".into(), json!(&vm.operation_hud_title));
    obj.insert("operation_hud_lines".into(), json!(&vm.operation_hud_lines));
    obj.insert("operation_hud_hint".into(), json!(&vm.operation_hud_hint));
    obj.insert(
        "operation_hud_subject".into(),
        json!(&vm.operation_hud_subject),
    );
    obj.insert("context_hint".into(), json!(&vm.context_hint));
    obj.insert("hud_pill_visible".into(), json!(&vm.hud_pill_visible));
    obj.insert("hud_pill_x".into(), json!(&vm.hud_pill_x));
    obj.insert("hud_pill_y".into(), json!(&vm.hud_pill_y));
    obj.insert("hud_pill_title".into(), json!(&vm.hud_pill_title));
    obj.insert("hud_pill_value".into(), json!(&vm.hud_pill_value));
    obj.insert("hud_pill_badge".into(), json!(&vm.hud_pill_badge));
    obj.insert("hud_pill_hint".into(), json!(&vm.hud_pill_hint));
    obj.insert("axis_guide_visible".into(), json!(&vm.axis_guide_visible));
    obj.insert("axis_guide_commands".into(), json!(&vm.axis_guide_commands));
    obj.insert("axis_guide_color".into(), json!(&vm.axis_guide_color));
    obj.insert("axis_guide_label".into(), json!(&vm.axis_guide_label));
    obj.insert("axis_guide_label_x".into(), json!(&vm.axis_guide_label_x));
    obj.insert("axis_guide_label_y".into(), json!(&vm.axis_guide_label_y));
    obj.insert(
        "world_axis_labels".into(),
        Value::Array(
            vm.world_axis_labels
                .iter()
                .map(worldaxislabelmodel)
                .collect(),
        ),
    );
    obj.insert("dimension_visible".into(), json!(&vm.dimension_visible));
    obj.insert("dimension_commands".into(), json!(&vm.dimension_commands));
    obj.insert("dimension_text".into(), json!(&vm.dimension_text));
    obj.insert("dimension_x".into(), json!(&vm.dimension_x));
    obj.insert("dimension_y".into(), json!(&vm.dimension_y));
    obj.insert("measure_visible".into(), json!(&vm.measure_visible));
    obj.insert("measure_commands".into(), json!(&vm.measure_commands));
    obj.insert("measure_text".into(), json!(&vm.measure_text));
    obj.insert("measure_x".into(), json!(&vm.measure_x));
    obj.insert("measure_y".into(), json!(&vm.measure_y));
    obj.insert("measure_distance".into(), json!(&vm.measure_distance));
    obj.insert("measure_dx".into(), json!(&vm.measure_dx));
    obj.insert("measure_dy".into(), json!(&vm.measure_dy));
    obj.insert("measure_dz".into(), json!(&vm.measure_dz));
    obj.insert("measure_angle_deg".into(), json!(&vm.measure_angle_deg));
    obj.insert("measure_hud_text".into(), json!(&vm.measure_hud_text));
    obj.insert(
        "measure_tags".into(),
        Value::Array(vm.measure_tags.iter().map(measuretagmodel).collect()),
    );
    obj.insert(
        "micro_inspector_open".into(),
        json!(&vm.micro_inspector_open),
    );
    obj.insert("micro_inspector_x".into(), json!(&vm.micro_inspector_x));
    obj.insert("micro_inspector_y".into(), json!(&vm.micro_inspector_y));
    obj.insert("snap_marker_visible".into(), json!(&vm.snap_marker_visible));
    obj.insert("snap_marker_x".into(), json!(&vm.snap_marker_x));
    obj.insert("snap_marker_y".into(), json!(&vm.snap_marker_y));
    obj.insert("snap_marker_label".into(), json!(&vm.snap_marker_label));
    obj.insert("snap_marker_round".into(), json!(&vm.snap_marker_round));
    obj.insert(
        "operation_preview_commands".into(),
        json!(&vm.operation_preview_commands),
    );
    obj.insert("drag_link_commands".into(), json!(&vm.drag_link_commands));
    obj.insert(
        "proportional_circle_commands".into(),
        json!(&vm.proportional_circle_commands),
    );
    obj.insert("hover_label".into(), json!(&vm.hover_label));
    obj.insert(
        "transform_instant_active".into(),
        json!(&vm.transform_instant_active),
    );
    obj.insert("gizmo_hover_axis".into(), json!(&vm.gizmo_hover_axis));
    obj.insert("gizmo_active_axis".into(), json!(&vm.gizmo_active_axis));
    obj.insert(
        "gizmo_constraint_axes".into(),
        json!(&vm.gizmo_constraint_axes),
    );
    obj.insert("gizmo_has_hover".into(), json!(&vm.gizmo_has_hover));
    obj.insert("gizmo_center_active".into(), json!(&vm.gizmo_center_active));
    obj.insert("gizmo_center_hover".into(), json!(&vm.gizmo_center_hover));
    obj.insert(
        "asset_library_visible".into(),
        json!(&vm.asset_library_visible),
    );
    obj.insert("gizmo".into(), gizmomodel(&vm.gizmo));
    obj.insert("cursor_visible".into(), json!(&vm.gizmo.cursor_visible));
    obj.insert("cursor_screen".into(), json!(&vm.gizmo.cursor_screen));
    obj.insert("add_menu_open".into(), json!(&vm.add_menu_open));
    obj.insert("inspector_width".into(), json!(&vm.inspector_width));
    obj.insert(
        "asset_library_height".into(),
        json!(&vm.asset_library_height),
    );
    obj.insert("rename_active".into(), json!(&vm.rename_active));
    obj.insert("rename_value".into(), json!(&vm.rename_value));
    obj.insert("context_menu_open".into(), json!(&vm.context_menu_open));
    obj.insert("context_menu_x".into(), json!(&vm.context_menu_x));
    obj.insert("context_menu_y".into(), json!(&vm.context_menu_y));
    obj.insert("context_menu_title".into(), json!(&vm.context_menu_title));
    obj.insert("context_menu_mode".into(), json!(&vm.context_menu_mode));
    obj.insert(
        "context_menu_visible".into(),
        json!(&vm.context_menu_visible),
    );
    obj.insert("context_menu_locked".into(), json!(&vm.context_menu_locked));
    obj.insert(
        "context_menu_isolated".into(),
        json!(&vm.context_menu_isolated),
    );
    obj.insert(
        "context_menu_can_move_up".into(),
        json!(&vm.context_menu_can_move_up),
    );
    obj.insert(
        "context_menu_can_move_down".into(),
        json!(&vm.context_menu_can_move_down),
    );
    obj.insert(
        "reference_manager_open".into(),
        json!(&vm.reference_manager_open),
    );
    obj.insert(
        "reference_slots".into(),
        Value::Array(
            vm.reference_slots
                .iter()
                .map(referenceslotviewmodel)
                .collect(),
        ),
    );
    obj.insert(
        "reference_total_count".into(),
        json!(&vm.reference_total_count),
    );
    obj.insert("primitive_active".into(), json!(&vm.primitive_active));
    obj.insert(
        "active_asset_is_parametric".into(),
        json!(&vm.active_asset_is_parametric),
    );
    obj.insert("primitive_kind".into(), json!(&vm.primitive_kind));
    obj.insert("primitive_title".into(), json!(&vm.primitive_title));
    obj.insert("primitive_radius".into(), json!(&vm.primitive_radius));
    obj.insert("primitive_radius_b".into(), json!(&vm.primitive_radius_b));
    obj.insert("primitive_height".into(), json!(&vm.primitive_height));
    obj.insert("primitive_depth".into(), json!(&vm.primitive_depth));
    obj.insert("primitive_width".into(), json!(&vm.primitive_width));
    obj.insert("primitive_sides".into(), json!(&vm.primitive_sides));
    obj.insert("primitive_rings".into(), json!(&vm.primitive_rings));
    obj.insert("primitive_subdiv".into(), json!(&vm.primitive_subdiv));
    obj.insert("primitive_cap_top".into(), json!(&vm.primitive_cap_top));
    obj.insert(
        "primitive_cap_bottom".into(),
        json!(&vm.primitive_cap_bottom),
    );
    obj.insert("primitive_fill_disc".into(), json!(&vm.primitive_fill_disc));
    obj.insert(
        "label_parametric_primitive".into(),
        json!(&vm.label_parametric_primitive),
    );
    obj.insert(
        "label_freeze_primitive".into(),
        json!(&vm.label_freeze_primitive),
    );
    obj.insert(
        "label_freeze_primitive_hint".into(),
        json!(&vm.label_freeze_primitive_hint),
    );
    obj.insert(
        "slice_preview_visible".into(),
        json!(&vm.slice_preview_visible),
    );
    obj.insert(
        "slice_preview_commands".into(),
        json!(&vm.slice_preview_commands),
    );
    obj.insert("slice_trim".into(), json!(&vm.slice_trim));
    obj.insert("bevel_clamp_overlap".into(), json!(&vm.bevel_clamp_overlap));
    obj.insert(
        "bevel_affect_vertices".into(),
        json!(&vm.bevel_affect_vertices),
    );
    obj.insert("protractor_visible".into(), json!(&vm.protractor_visible));
    obj.insert(
        "protractor_wedge_commands".into(),
        json!(&vm.protractor_wedge_commands),
    );
    obj.insert(
        "protractor_ticks_commands".into(),
        json!(&vm.protractor_ticks_commands),
    );
    obj.insert("menu_open".into(), json!(&vm.menu_open));
    obj.insert("menu_file_label".into(), json!(&vm.menu_file_label));
    obj.insert("menu_edit_label".into(), json!(&vm.menu_edit_label));
    obj.insert("menu_view_label".into(), json!(&vm.menu_view_label));
    obj.insert("menu_window_label".into(), json!(&vm.menu_window_label));
    obj.insert(
        "menu_file_items".into(),
        Value::Array(vm.menu_file_items.iter().map(menuentrymodel).collect()),
    );
    obj.insert(
        "menu_edit_items".into(),
        Value::Array(vm.menu_edit_items.iter().map(menuentrymodel).collect()),
    );
    obj.insert(
        "menu_view_items".into(),
        Value::Array(vm.menu_view_items.iter().map(menuentrymodel).collect()),
    );
    obj.insert(
        "menu_window_items".into(),
        Value::Array(vm.menu_window_items.iter().map(menuentrymodel).collect()),
    );
    obj.insert("label_parts".into(), json!(&vm.label_parts));
    obj.insert(
        "label_project_asset_library".into(),
        json!(&vm.label_project_asset_library),
    );
    obj.insert(
        "label_save_active_as_asset".into(),
        json!(&vm.label_save_active_as_asset),
    );
    obj.insert("label_status_hint".into(), json!(&vm.label_status_hint));
    obj.insert("label_unwrap_mesh".into(), json!(&vm.label_unwrap_mesh));
    obj.insert("label_pack_islands".into(), json!(&vm.label_pack_islands));
    obj.insert(
        "label_active_brush_color".into(),
        json!(&vm.label_active_brush_color),
    );
    obj.insert(
        "label_albedo_base_color".into(),
        json!(&vm.label_albedo_base_color),
    );
    obj.insert("label_theme".into(), json!(&vm.label_theme));
    obj.insert(
        "label_place_in_scene".into(),
        json!(&vm.label_place_in_scene),
    );
    obj.insert(
        "label_vertical_tool_drag".into(),
        json!(&vm.label_vertical_tool_drag),
    );
    obj.insert(
        "label_invert_vertical_drag".into(),
        json!(&vm.label_invert_vertical_drag),
    );
    obj.insert(
        "label_colorblind_axes".into(),
        json!(&vm.label_colorblind_axes),
    );
    obj.insert(
        "label_reduced_motion".into(),
        json!(&vm.label_reduced_motion),
    );
    obj.insert(
        "label_double_tap_interval".into(),
        json!(&vm.label_double_tap_interval),
    );
    obj.insert(
        "label_multiselection_measure_tag".into(),
        json!(&vm.label_multiselection_measure_tag),
    );
    obj.insert("label_search_assets".into(), json!(&vm.label_search_assets));
    obj.insert("label_search_parts".into(), json!(&vm.label_search_parts));
    obj.insert("label_inspector".into(), json!(&vm.label_inspector));
    obj.insert(
        "label_expand_inspector".into(),
        json!(&vm.label_expand_inspector),
    );
    obj.insert(
        "label_collapse_inspector".into(),
        json!(&vm.label_collapse_inspector),
    );
    obj.insert(
        "label_resize_panel_width".into(),
        json!(&vm.label_resize_panel_width),
    );
    obj.insert("label_section_dock".into(), json!(&vm.label_section_dock));
    obj.insert("label_section_drag".into(), json!(&vm.label_section_drag));
    obj.insert(
        "label_section_pin_open".into(),
        json!(&vm.label_section_pin_open),
    );
    obj.insert(
        "label_section_pin_asset".into(),
        json!(&vm.label_section_pin_asset),
    );
    obj.insert(
        "label_section_unpin_asset".into(),
        json!(&vm.label_section_unpin_asset),
    );
    obj.insert("label_object_name".into(), json!(&vm.label_object_name));
    obj.insert(
        "label_object_visibility".into(),
        json!(&vm.label_object_visibility),
    );
    obj.insert("label_object_lock".into(), json!(&vm.label_object_lock));
    obj.insert(
        "label_object_no_selection".into(),
        json!(&vm.label_object_no_selection),
    );
    obj.insert("label_stats_faces".into(), json!(&vm.label_stats_faces));
    obj.insert("label_stats_verts".into(), json!(&vm.label_stats_verts));
    obj.insert("label_stats_tris".into(), json!(&vm.label_stats_tris));
    obj.insert(
        "label_stats_selection".into(),
        json!(&vm.label_stats_selection),
    );
    obj.insert("label_tool_options".into(), json!(&vm.label_tool_options));
    obj.insert(
        "label_tool_options_expand".into(),
        json!(&vm.label_tool_options_expand),
    );
    obj.insert(
        "label_tool_options_collapse".into(),
        json!(&vm.label_tool_options_collapse),
    );
    obj.insert("label_quick_actions".into(), json!(&vm.label_quick_actions));
    obj.insert(
        "label_quick_action_customize".into(),
        json!(&vm.label_quick_action_customize),
    );
    obj.insert(
        "label_quick_action_add".into(),
        json!(&vm.label_quick_action_add),
    );
    obj.insert(
        "label_quick_action_remove".into(),
        json!(&vm.label_quick_action_remove),
    );
    obj.insert(
        "label_quick_action_reset".into(),
        json!(&vm.label_quick_action_reset),
    );
    obj.insert(
        "label_quick_action_done".into(),
        json!(&vm.label_quick_action_done),
    );
    obj.insert(
        "label_action_subdivide".into(),
        json!(&vm.label_action_subdivide),
    );
    obj.insert("label_action_join".into(), json!(&vm.label_action_join));
    obj.insert("label_action_merge".into(), json!(&vm.label_action_merge));
    obj.insert("label_action_slice".into(), json!(&vm.label_action_slice));
    obj.insert(
        "label_action_loop_cut".into(),
        json!(&vm.label_action_loop_cut),
    );
    obj.insert(
        "label_material_base_color".into(),
        json!(&vm.label_material_base_color),
    );
    obj.insert(
        "label_material_profile".into(),
        json!(&vm.label_material_profile),
    );
    obj.insert(
        "label_material_roughness".into(),
        json!(&vm.label_material_roughness),
    );
    obj.insert(
        "label_material_metallic".into(),
        json!(&vm.label_material_metallic),
    );
    obj.insert(
        "label_material_normal_scale".into(),
        json!(&vm.label_material_normal_scale),
    );
    obj.insert(
        "label_material_advanced".into(),
        json!(&vm.label_material_advanced),
    );
    obj.insert(
        "label_material_assign".into(),
        json!(&vm.label_material_assign),
    );
    obj.insert("label_material_new".into(), json!(&vm.label_material_new));
    obj.insert(
        "label_material_duplicate".into(),
        json!(&vm.label_material_duplicate),
    );
    obj.insert(
        "label_material_remove".into(),
        json!(&vm.label_material_remove),
    );
    obj.insert(
        "label_material_no_material".into(),
        json!(&vm.label_material_no_material),
    );
    obj.insert(
        "label_material_no_selection".into(),
        json!(&vm.label_material_no_selection),
    );
    obj.insert(
        "label_material_emission_strength".into(),
        json!(&vm.label_material_emission_strength),
    );
    obj.insert(
        "label_material_alpha_cutoff".into(),
        json!(&vm.label_material_alpha_cutoff),
    );
    obj.insert(
        "label_material_texture_albedo".into(),
        json!(&vm.label_material_texture_albedo),
    );
    obj.insert(
        "label_material_no_texture".into(),
        json!(&vm.label_material_no_texture),
    );
    obj.insert(
        "label_material_create_texture".into(),
        json!(&vm.label_material_create_texture),
    );
    obj.insert(
        "label_material_clear_texture".into(),
        json!(&vm.label_material_clear_texture),
    );
    obj.insert(
        "label_material_profile_pbr".into(),
        json!(&vm.label_material_profile_pbr),
    );
    obj.insert(
        "label_material_profile_unlit".into(),
        json!(&vm.label_material_profile_unlit),
    );
    obj.insert(
        "label_material_profile_toon".into(),
        json!(&vm.label_material_profile_toon),
    );
    obj.insert(
        "label_material_profile_glass".into(),
        json!(&vm.label_material_profile_glass),
    );
    obj.insert(
        "label_material_profile_emissive".into(),
        json!(&vm.label_material_profile_emissive),
    );
    obj.insert(
        "label_material_alpha_opaque".into(),
        json!(&vm.label_material_alpha_opaque),
    );
    obj.insert(
        "label_material_alpha_mask".into(),
        json!(&vm.label_material_alpha_mask),
    );
    obj.insert(
        "label_material_alpha_blend".into(),
        json!(&vm.label_material_alpha_blend),
    );
    obj.insert(
        "label_modifier_mirror".into(),
        json!(&vm.label_modifier_mirror),
    );
    obj.insert(
        "label_modifier_symmetry".into(),
        json!(&vm.label_modifier_symmetry),
    );
    obj.insert(
        "label_modifiers_empty".into(),
        json!(&vm.label_modifiers_empty),
    );
    obj.insert(
        "label_modifier_apply".into(),
        json!(&vm.label_modifier_apply),
    );
    obj.insert("label_modifier_axis".into(), json!(&vm.label_modifier_axis));
    obj.insert(
        "label_modifier_add_mirror".into(),
        json!(&vm.label_modifier_add_mirror),
    );
    obj.insert(
        "label_modifier_add_symmetry".into(),
        json!(&vm.label_modifier_add_symmetry),
    );
    obj.insert(
        "label_modifier_remove".into(),
        json!(&vm.label_modifier_remove),
    );
    obj.insert(
        "label_modifier_move_up".into(),
        json!(&vm.label_modifier_move_up),
    );
    obj.insert(
        "label_modifier_move_down".into(),
        json!(&vm.label_modifier_move_down),
    );
    obj.insert(
        "label_modifier_direction".into(),
        json!(&vm.label_modifier_direction),
    );
    obj.insert(
        "label_modifier_positive_to_negative".into(),
        json!(&vm.label_modifier_positive_to_negative),
    );
    obj.insert(
        "label_modifier_negative_to_positive".into(),
        json!(&vm.label_modifier_negative_to_positive),
    );
    obj.insert("label_tab_parts".into(), json!(&vm.label_tab_parts));
    obj.insert("label_tab_transform".into(), json!(&vm.label_tab_transform));
    obj.insert("label_tab_material".into(), json!(&vm.label_tab_material));
    obj.insert("label_tab_object".into(), json!(&vm.label_tab_object));
    obj.insert("label_tab_modifiers".into(), json!(&vm.label_tab_modifiers));
    obj.insert(
        "label_numeric_field_hint".into(),
        json!(&vm.label_numeric_field_hint),
    );
    obj.insert("label_model_select".into(), json!(&vm.label_model_select));
    obj.insert(
        "label_model_position".into(),
        json!(&vm.label_model_position),
    );
    obj.insert("label_model_rotate".into(), json!(&vm.label_model_rotate));
    obj.insert("label_model_scale".into(), json!(&vm.label_model_scale));
    obj.insert(
        "label_model_transform".into(),
        json!(&vm.label_model_transform),
    );
    obj.insert("label_model_lasso".into(), json!(&vm.label_model_lasso));
    obj.insert(
        "label_model_loop_cut".into(),
        json!(&vm.label_model_loop_cut),
    );
    obj.insert("hint_model_loop_cut".into(), json!(&vm.hint_model_loop_cut));
    obj.insert("label_model_slice".into(), json!(&vm.label_model_slice));
    obj.insert("hint_model_slice".into(), json!(&vm.hint_model_slice));
    obj.insert(
        "label_model_push_pull".into(),
        json!(&vm.label_model_push_pull),
    );
    obj.insert(
        "hint_model_push_pull".into(),
        json!(&vm.hint_model_push_pull),
    );
    obj.insert("label_model_profile".into(), json!(&vm.label_model_profile));
    obj.insert("hint_model_profile".into(), json!(&vm.hint_model_profile));
    obj.insert("label_model_pivot".into(), json!(&vm.label_model_pivot));
    obj.insert("hint_model_pivot".into(), json!(&vm.hint_model_pivot));
    obj.insert("label_profile_depth".into(), json!(&vm.label_profile_depth));
    obj.insert(
        "label_profile_points".into(),
        json!(&vm.label_profile_points),
    );
    obj.insert("label_profile_close".into(), json!(&vm.label_profile_close));
    obj.insert("label_view_gizmo".into(), json!(&vm.label_view_gizmo));
    obj.insert(
        "label_view_gizmo_hint".into(),
        json!(&vm.label_view_gizmo_hint),
    );
    obj.insert("label_profile_plane".into(), json!(&vm.label_profile_plane));
    obj.insert(
        "label_profile_plane_auto".into(),
        json!(&vm.label_profile_plane_auto),
    );
    obj.insert(
        "label_profile_plane_ground".into(),
        json!(&vm.label_profile_plane_ground),
    );
    obj.insert(
        "label_profile_plane_face".into(),
        json!(&vm.label_profile_plane_face),
    );
    obj.insert(
        "label_profile_plane_view".into(),
        json!(&vm.label_profile_plane_view),
    );
    obj.insert(
        "label_profile_look_at_plane".into(),
        json!(&vm.label_profile_look_at_plane),
    );
    obj.insert(
        "profile_workplane_locked".into(),
        json!(&vm.profile_workplane_locked),
    );
    obj.insert(
        "region_hover_commands".into(),
        json!(&vm.region_hover_commands),
    );
    obj.insert(
        "profile_selection_commands".into(),
        json!(&vm.profile_selection_commands),
    );
    obj.insert("brush_panel".into(), brushpanelmodel(&vm.brush_panel));
    obj.insert("keymap_revision".into(), json!(&vm.keymap_revision));
    obj.insert(
        "keymap_snapshot".into(),
        keymapsnapshot(&vm.keymap_snapshot),
    );
    obj.insert(
        "keymap_capture_action".into(),
        json!(&vm.keymap_capture_action),
    );
    obj.insert(
        "region_shapes_commands".into(),
        json!(&vm.region_shapes_commands),
    );
    obj.insert(
        "profile_outline_commands".into(),
        json!(&vm.profile_outline_commands),
    );
    obj.insert(
        "poly_pen_preview_commands".into(),
        json!(&vm.poly_pen_preview_commands),
    );
    obj.insert("label_poly_pen".into(), json!(&vm.label_poly_pen));
    obj.insert("label_poly_pen_hint".into(), json!(&vm.label_poly_pen_hint));
    obj.insert("poly_pen_mode".into(), json!(&vm.poly_pen_mode));
    obj.insert("label_poly_pen_mode".into(), json!(&vm.label_poly_pen_mode));
    obj.insert(
        "label_poly_pen_mode_auto".into(),
        json!(&vm.label_poly_pen_mode_auto),
    );
    obj.insert(
        "label_poly_pen_mode_points".into(),
        json!(&vm.label_poly_pen_mode_points),
    );
    obj.insert(
        "label_poly_pen_mode_edges".into(),
        json!(&vm.label_poly_pen_mode_edges),
    );
    obj.insert(
        "label_poly_pen_mode_polygons".into(),
        json!(&vm.label_poly_pen_mode_polygons),
    );
    obj.insert(
        "label_poly_pen_mode_hint".into(),
        json!(&vm.label_poly_pen_mode_hint),
    );
    obj.insert("modeling_mode".into(), json!(&vm.modeling_mode));
    obj.insert(
        "label_workspace_draw_title".into(),
        json!(&vm.label_workspace_draw_title),
    );
    obj.insert(
        "label_workspace_draw_description".into(),
        json!(&vm.label_workspace_draw_description),
    );
    obj.insert(
        "label_workspace_poly_title".into(),
        json!(&vm.label_workspace_poly_title),
    );
    obj.insert(
        "label_workspace_poly_description".into(),
        json!(&vm.label_workspace_poly_description),
    );
    obj.insert(
        "label_profile_generate".into(),
        json!(&vm.label_profile_generate),
    );
    obj.insert(
        "label_profile_revolve".into(),
        json!(&vm.label_profile_revolve),
    );
    obj.insert("label_profile_sweep".into(), json!(&vm.label_profile_sweep));
    obj.insert("label_profile_cuts".into(), json!(&vm.label_profile_cuts));
    obj.insert(
        "label_profile_presets".into(),
        json!(&vm.label_profile_presets),
    );
    obj.insert(
        "label_profile_add_rect".into(),
        json!(&vm.label_profile_add_rect),
    );
    obj.insert(
        "label_profile_add_circle".into(),
        json!(&vm.label_profile_add_circle),
    );
    obj.insert(
        "label_profile_canvas_hint".into(),
        json!(&vm.label_profile_canvas_hint),
    );
    obj.insert(
        "label_profile_wall_thickness".into(),
        json!(&vm.label_profile_wall_thickness),
    );
    obj.insert(
        "label_profile_smooth_curves".into(),
        json!(&vm.label_profile_smooth_curves),
    );
    obj.insert(
        "label_profile_sharp_corners".into(),
        json!(&vm.label_profile_sharp_corners),
    );
    obj.insert(
        "label_profile_smoothness".into(),
        json!(&vm.label_profile_smoothness),
    );
    obj.insert("hint_model_select".into(), json!(&vm.hint_model_select));
    obj.insert("hint_model_position".into(), json!(&vm.hint_model_position));
    obj.insert("hint_model_rotate".into(), json!(&vm.hint_model_rotate));
    obj.insert("hint_model_scale".into(), json!(&vm.hint_model_scale));
    obj.insert(
        "hint_model_transform".into(),
        json!(&vm.hint_model_transform),
    );
    obj.insert("hint_model_lasso".into(), json!(&vm.hint_model_lasso));
    obj.insert("label_hide_part".into(), json!(&vm.label_hide_part));
    obj.insert("label_show_part".into(), json!(&vm.label_show_part));
    obj.insert("label_lock_part".into(), json!(&vm.label_lock_part));
    obj.insert("label_unlock_part".into(), json!(&vm.label_unlock_part));
    obj.insert(
        "label_selected_parts_only".into(),
        json!(&vm.label_selected_parts_only),
    );
    obj.insert("label_sort_parts".into(), json!(&vm.label_sort_parts));
    obj.insert(
        "label_parts_row_size".into(),
        json!(&vm.label_parts_row_size),
    );
    obj.insert("label_sort_assets".into(), json!(&vm.label_sort_assets));
    obj.insert(
        "label_thumbnail_size".into(),
        json!(&vm.label_thumbnail_size),
    );
    obj.insert(
        "label_selection_color".into(),
        json!(&vm.label_selection_color),
    );
    obj.insert(
        "label_highlight_thickness".into(),
        json!(&vm.label_highlight_thickness),
    );
    obj.insert(
        "label_view_wireframe".into(),
        json!(&vm.label_view_wireframe),
    );
    obj.insert(
        "label_view_wireframe_hint".into(),
        json!(&vm.label_view_wireframe_hint),
    );
    obj.insert("label_view_solid".into(), json!(&vm.label_view_solid));
    obj.insert(
        "label_view_solid_hint".into(),
        json!(&vm.label_view_solid_hint),
    );
    obj.insert("label_view_material".into(), json!(&vm.label_view_material));
    obj.insert(
        "label_view_material_hint".into(),
        json!(&vm.label_view_material_hint),
    );
    obj.insert("label_view_lit".into(), json!(&vm.label_view_lit));
    obj.insert("label_view_lit_hint".into(), json!(&vm.label_view_lit_hint));
    obj.insert(
        "label_more_model_tools".into(),
        json!(&vm.label_more_model_tools),
    );
    obj.insert("label_xray_opacity".into(), json!(&vm.label_xray_opacity));
    obj.insert("label_wire_overlay".into(), json!(&vm.label_wire_overlay));
    obj.insert(
        "label_wire_overlay_hint".into(),
        json!(&vm.label_wire_overlay_hint),
    );
    obj.insert("recovery_open".into(), json!(&vm.recovery_open));
    obj.insert("recovery_title".into(), json!(&vm.recovery_title));
    obj.insert("recovery_body".into(), json!(&vm.recovery_body));
    obj.insert("recovery_detail".into(), json!(&vm.recovery_detail));
    obj.insert("recovery_recover".into(), json!(&vm.recovery_recover));
    obj.insert("recovery_keep".into(), json!(&vm.recovery_keep));
    obj.insert("recovery_discard".into(), json!(&vm.recovery_discard));
    obj.insert("label_asset_library".into(), json!(&vm.label_asset_library));
    obj.insert("label_preferences".into(), json!(&vm.label_preferences));
    obj.insert("shell_info".into(), json!(&vm.shell_info));
    obj.insert("label_apply".into(), json!(&vm.label_apply));
    obj.insert("label_cancel".into(), json!(&vm.label_cancel));
    obj.insert("label_delete".into(), json!(&vm.label_delete));
    obj.insert("label_duplicate".into(), json!(&vm.label_duplicate));
    obj.insert(
        "themes".into(),
        Value::Array(vm.themes.iter().map(themeentrymodel).collect()),
    );
    obj.insert("uv_editor".into(), uveditormodel(&vm.uv_editor));
    obj.insert(
        "paint_layers".into(),
        Value::Array(vm.paint_layers.iter().map(paintlayermodel).collect()),
    );
    obj.insert("paint_layer_count".into(), json!(&vm.paint_layer_count));
    obj.insert("paint_effect_kind".into(), json!(&vm.paint_effect_kind));
    obj.insert(
        "paint_effect_params".into(),
        Value::Array(
            vm.paint_effect_params
                .iter()
                .map(painteffectparam)
                .collect(),
        ),
    );
    obj.insert("paint_canvas_size".into(), json!(&vm.paint_canvas_size));
    obj.insert(
        "paint_canvas_revision".into(),
        json!(&vm.paint_canvas_revision),
    );
    obj.insert("paint_pixel_grid".into(), json!(&vm.paint_pixel_grid));
    obj.insert("paint_canvas_zoom".into(), json!(&vm.paint_canvas_zoom));
    obj.insert(
        "paint_canvas_grid_commands".into(),
        json!(&vm.paint_canvas_grid_commands),
    );
    obj.insert("paint_fill_scope".into(), json!(&vm.paint_fill_scope));
    obj.insert("paint_projection".into(), json!(&vm.paint_projection));
    obj.insert("paint_lock".into(), json!(&vm.paint_lock));
    obj.insert(
        "active_layer_is_decal".into(),
        json!(&vm.active_layer_is_decal),
    );
    obj.insert("decal_surface".into(), json!(&vm.decal_surface));
    obj.insert("paint_texture_side".into(), json!(&vm.paint_texture_side));
    obj.insert("decal_center_u".into(), json!(&vm.decal_center_u));
    obj.insert("decal_center_v".into(), json!(&vm.decal_center_v));
    obj.insert("decal_scale_u".into(), json!(&vm.decal_scale_u));
    obj.insert("decal_scale_v".into(), json!(&vm.decal_scale_v));
    obj.insert("decal_rotation_deg".into(), json!(&vm.decal_rotation_deg));
    obj.insert(
        "label_decal_transform".into(),
        json!(&vm.label_decal_transform),
    );
    obj.insert(
        "label_decal_position".into(),
        json!(&vm.label_decal_position),
    );
    obj.insert("label_decal_scale".into(), json!(&vm.label_decal_scale));
    obj.insert("label_decal_width".into(), json!(&vm.label_decal_width));
    obj.insert(
        "label_decal_rotation".into(),
        json!(&vm.label_decal_rotation),
    );
    obj.insert("label_decal_bake".into(), json!(&vm.label_decal_bake));
    obj.insert("label_decal_hint".into(), json!(&vm.label_decal_hint));
    obj.insert(
        "decal_preview_commands".into(),
        json!(&vm.decal_preview_commands),
    );
    obj.insert("decal_variant_names".into(), json!(&vm.decal_variant_names));
    obj.insert("decal_variant_index".into(), json!(&vm.decal_variant_index));
    obj.insert("decal_frame".into(), json!(&vm.decal_frame));
    obj.insert("decal_track_length".into(), json!(&vm.decal_track_length));
    obj.insert("decal_track_fps".into(), json!(&vm.decal_track_fps));
    obj.insert("decal_keys_text".into(), json!(&vm.decal_keys_text));
    obj.insert("decal_has_keys".into(), json!(&vm.decal_has_keys));
    obj.insert("decal_frame_keyed".into(), json!(&vm.decal_frame_keyed));
    obj.insert("decal_playing".into(), json!(&vm.decal_playing));
    obj.insert("loop_cut_active".into(), json!(&vm.loop_cut_active));
    obj.insert("loop_cut_slide".into(), json!(&vm.loop_cut_slide));
    obj.insert("loop_cut_cuts".into(), json!(&vm.loop_cut_cuts));
    obj.insert(
        "loop_cut_preview_commands".into(),
        json!(&vm.loop_cut_preview_commands),
    );
    obj.insert("loop_cut_armed".into(), json!(&vm.loop_cut_armed));
    obj.insert("loop_cut_balanced".into(), json!(&vm.loop_cut_balanced));
    obj.insert("profile_active".into(), json!(&vm.profile_active));
    obj.insert("pivot_id".into(), json!(&vm.pivot_id));
    obj.insert("pivot_label".into(), json!(&vm.pivot_label));
    obj.insert("pivot_menu_open".into(), json!(&vm.pivot_menu_open));
    obj.insert("pivot_median_label".into(), json!(&vm.pivot_median_label));
    obj.insert("pivot_bounds_label".into(), json!(&vm.pivot_bounds_label));
    obj.insert("pivot_cursor_label".into(), json!(&vm.pivot_cursor_label));
    obj.insert(
        "pivot_individual_label".into(),
        json!(&vm.pivot_individual_label),
    );
    obj.insert("edit_pivot".into(), json!(&vm.edit_pivot));
    obj.insert(
        "label_origin_to_geometry".into(),
        json!(&vm.label_origin_to_geometry),
    );
    obj.insert(
        "label_origin_to_bottom".into(),
        json!(&vm.label_origin_to_bottom),
    );
    obj.insert(
        "label_origin_to_cursor".into(),
        json!(&vm.label_origin_to_cursor),
    );
    obj.insert(
        "label_origin_to_selection".into(),
        json!(&vm.label_origin_to_selection),
    );
    obj.insert(
        "label_geometry_to_origin".into(),
        json!(&vm.label_geometry_to_origin),
    );
    obj.insert("label_edit_pivot".into(), json!(&vm.label_edit_pivot));
    obj.insert("hint_edit_pivot".into(), json!(&vm.hint_edit_pivot));
    obj.insert("profile_point_count".into(), json!(&vm.profile_point_count));
    obj.insert("profile_closed".into(), json!(&vm.profile_closed));
    obj.insert(
        "profile_preview_commands".into(),
        json!(&vm.profile_preview_commands),
    );
    obj.insert("profile_parametric".into(), json!(&vm.profile_parametric));
    obj.insert("profile_ellipse".into(), json!(&vm.profile_ellipse));
    obj.insert("profile_width".into(), json!(&vm.profile_width));
    obj.insert("profile_height".into(), json!(&vm.profile_height));
    obj.insert("profile_segments".into(), json!(&vm.profile_segments));
    obj.insert("profile_depth".into(), json!(&vm.profile_depth));
    obj.insert(
        "profile_wall_thickness".into(),
        json!(&vm.profile_wall_thickness),
    );
    obj.insert("profile_smoothness".into(), json!(&vm.profile_smoothness));
    obj.insert("profile_has_curves".into(), json!(&vm.profile_has_curves));
    obj.insert("profile_workplane".into(), json!(&vm.profile_workplane));
    obj.insert("profile_volume_mode".into(), json!(&vm.profile_volume_mode));
    obj.insert(
        "profile_revolve_angle".into(),
        json!(&vm.profile_revolve_angle),
    );
    obj.insert(
        "paint_show_uv_overlay".into(),
        json!(&vm.paint_show_uv_overlay),
    );
    obj.insert("uv_show_texture".into(), json!(&vm.uv_show_texture));
    obj.insert("tool_activation".into(), json!(&vm.tool_activation));
    obj.insert(
        "keyboard_tool_modal_active".into(),
        json!(&vm.keyboard_tool_modal_active),
    );
    obj.insert(
        "is_instant_tool_mode".into(),
        json!(&vm.is_instant_tool_mode),
    );
    obj.insert(
        "invert_vertical_drag".into(),
        json!(&vm.invert_vertical_drag),
    );
    obj.insert("zoom_sensitivity".into(), json!(1.0));
    obj.insert("colorblind_axes".into(), json!(&vm.colorblind_axes));
    obj.insert("reduced_motion".into(), json!(&vm.reduced_motion));
    obj.insert(
        "double_tap_interval_ms".into(),
        json!(&vm.double_tap_interval_ms),
    );
    obj.insert("tool_grammar_active".into(), json!(&vm.tool_grammar_active));
    obj.insert(
        "tool_gesture_latched".into(),
        json!(&vm.tool_gesture_latched),
    );
    obj.insert(
        "last_operation_active".into(),
        json!(&vm.last_operation_active),
    );
    obj.insert(
        "last_operation_title".into(),
        json!(&vm.last_operation_title),
    );
    obj.insert(
        "last_operation_value".into(),
        json!(&vm.last_operation_value),
    );
    obj.insert("last_operation_step".into(), json!(&vm.last_operation_step));
    obj.insert("last_operation_unit".into(), json!(&vm.last_operation_unit));
    obj.insert(
        "label_last_operation".into(),
        json!(&vm.label_last_operation),
    );
    obj.insert(
        "label_last_operation_hint".into(),
        json!(&vm.label_last_operation_hint),
    );
    obj.insert("drag_threshold_px".into(), json!(&vm.drag_threshold_px));
    obj.insert("snap_radius_px".into(), json!(&vm.snap_radius_px));
    obj.insert("label_snap_radius".into(), json!(&vm.label_snap_radius));
    obj.insert("click_move_click".into(), json!(&vm.click_move_click));
    obj.insert("show_tool_labels".into(), json!(&vm.show_tool_labels));
    obj.insert(
        "label_show_tool_labels".into(),
        json!(&vm.label_show_tool_labels),
    );
    obj.insert(
        "label_show_tool_labels_hint".into(),
        json!(&vm.label_show_tool_labels_hint),
    );
    obj.insert("accent_rgb".into(), json!(&vm.accent_rgb));
    obj.insert("home_open".into(), json!(&vm.home_open));
    obj.insert("show_home_on_start".into(), json!(&vm.show_home_on_start));
    obj.insert("recent_projects".into(), json!(&vm.recent_projects));
    obj.insert("home_texts".into(), json!(&vm.home_texts));
    obj.insert("accent_color_hex".into(), json!(&vm.accent_color_hex));
    obj.insert("label_accent_color".into(), json!(&vm.label_accent_color));
    obj.insert(
        "label_accent_color_hint".into(),
        json!(&vm.label_accent_color_hint),
    );
    obj.insert(
        "workplane_prefer_ground".into(),
        json!(&vm.workplane_prefer_ground),
    );
    obj.insert(
        "studio_light_follows_camera".into(),
        json!(&vm.studio_light_follows_camera),
    );
    obj.insert("viewport_matcap".into(), json!(&vm.viewport_matcap));
    obj.insert(
        "viewport_ambient_occlusion".into(),
        json!(&vm.viewport_ambient_occlusion),
    );
    obj.insert(
        "label_viewport_matcap".into(),
        json!(&vm.label_viewport_matcap),
    );
    obj.insert(
        "label_viewport_matcap_hint".into(),
        json!(&vm.label_viewport_matcap_hint),
    );
    obj.insert(
        "label_viewport_ambient_occlusion".into(),
        json!(&vm.label_viewport_ambient_occlusion),
    );
    obj.insert(
        "label_viewport_ambient_occlusion_hint".into(),
        json!(&vm.label_viewport_ambient_occlusion_hint),
    );
    obj.insert("gizmo_gpu".into(), json!(&vm.gizmo_gpu));
    obj.insert(
        "label_studio_light_follows_camera".into(),
        json!(&vm.label_studio_light_follows_camera),
    );
    obj.insert(
        "label_studio_light_follows_camera_hint".into(),
        json!(&vm.label_studio_light_follows_camera_hint),
    );
    obj.insert(
        "label_workplane_prefer_ground".into(),
        json!(&vm.label_workplane_prefer_ground),
    );
    obj.insert(
        "label_workplane_prefer_ground_hint".into(),
        json!(&vm.label_workplane_prefer_ground_hint),
    );
    obj.insert(
        "label_drag_threshold".into(),
        json!(&vm.label_drag_threshold),
    );
    obj.insert(
        "label_click_move_click".into(),
        json!(&vm.label_click_move_click),
    );
    obj.insert(
        "label_click_move_click_hint".into(),
        json!(&vm.label_click_move_click_hint),
    );
    obj.insert(
        "multiselection_measure_tag".into(),
        json!(&vm.multiselection_measure_tag),
    );
    obj.insert("active_language".into(), json!(&vm.active_language));
    obj.insert("ui_scale".into(), json!(&vm.ui_scale));
    obj.insert("tool_modal_active".into(), json!(&vm.tool_modal_active));
    obj.insert("tool_modal_id".into(), json!(&vm.tool_modal_id));
    obj.insert("tool_modal_title".into(), json!(&vm.tool_modal_title));
    obj.insert("tool_modal_label".into(), json!(&vm.tool_modal_label));
    obj.insert("tool_modal_value".into(), json!(&vm.tool_modal_value));
    obj.insert("tool_modal_step".into(), json!(&vm.tool_modal_step));
    obj.insert("tool_modal_min".into(), json!(&vm.tool_modal_min));
    obj.insert("tool_modal_max".into(), json!(&vm.tool_modal_max));
    obj.insert("tool_options_active".into(), json!(&vm.tool_options_active));
    obj.insert("tool_options_title".into(), json!(&vm.tool_options_title));
    obj.insert("tool_options_hint".into(), json!(&vm.tool_options_hint));
    obj.insert(
        "tool_options_accessible_label".into(),
        json!(&vm.tool_options_accessible_label),
    );
    obj.insert(
        "object_has_selection".into(),
        json!(&vm.object_has_selection),
    );
    obj.insert("object_id".into(), json!(&vm.object_id));
    obj.insert("object_name".into(), json!(&vm.object_name));
    obj.insert("object_visible".into(), json!(&vm.object_visible));
    obj.insert("object_locked".into(), json!(&vm.object_locked));
    obj.insert("object_verts".into(), json!(&vm.object_verts));
    obj.insert("object_faces".into(), json!(&vm.object_faces));
    obj.insert("object_tris".into(), json!(&vm.object_tris));
    obj.insert("object_selection".into(), json!(&vm.object_selection));
    obj.insert("object_material".into(), json!(&vm.object_material));
    obj.insert(
        "object_modifier_count".into(),
        json!(&vm.object_modifier_count),
    );
    obj.insert(
        "material_has_selection".into(),
        json!(&vm.material_has_selection),
    );
    obj.insert("material_id".into(), json!(&vm.material_id));
    obj.insert("material_name".into(), json!(&vm.material_name));
    obj.insert("material_profile".into(), json!(&vm.material_profile));
    obj.insert(
        "material_profile_label".into(),
        json!(&vm.material_profile_label),
    );
    obj.insert(
        "material_profile_ids".into(),
        json!(&vm.material_profile_ids),
    );
    obj.insert(
        "material_profile_labels".into(),
        json!(&vm.material_profile_labels),
    );
    obj.insert("material_base_color".into(), json!(&vm.material_base_color));
    obj.insert("material_palette".into(), json!(&vm.material_palette));
    obj.insert("material_roughness".into(), json!(&vm.material_roughness));
    obj.insert("material_metallic".into(), json!(&vm.material_metallic));
    obj.insert(
        "material_normal_scale".into(),
        json!(&vm.material_normal_scale),
    );
    obj.insert("material_emission".into(), json!(&vm.material_emission));
    obj.insert(
        "material_emission_strength".into(),
        json!(&vm.material_emission_strength),
    );
    obj.insert("material_alpha_mode".into(), json!(&vm.material_alpha_mode));
    obj.insert(
        "material_alpha_label".into(),
        json!(&vm.material_alpha_label),
    );
    obj.insert("material_alpha_ids".into(), json!(&vm.material_alpha_ids));
    obj.insert(
        "material_alpha_labels".into(),
        json!(&vm.material_alpha_labels),
    );
    obj.insert(
        "material_alpha_cutoff".into(),
        json!(&vm.material_alpha_cutoff),
    );
    obj.insert("material_has_albedo".into(), json!(&vm.material_has_albedo));
    obj.insert(
        "material_albedo_label".into(),
        json!(&vm.material_albedo_label),
    );
    obj.insert(
        "quick_actions".into(),
        Value::Array(vm.quick_actions.iter().map(quickactionmodel).collect()),
    );
    obj.insert(
        "quick_action_candidates".into(),
        Value::Array(
            vm.quick_action_candidates
                .iter()
                .map(quickactionmodel)
                .collect(),
        ),
    );
    obj.insert(
        "modifier_rows".into(),
        Value::Array(vm.modifier_rows.iter().map(modifierrowmodel).collect()),
    );
    obj.insert(
        "section_states".into(),
        Value::Array(vm.section_states.iter().map(sectionstatemodel).collect()),
    );
    obj.insert("material_slots".into(), json!(&vm.material_slots));
    obj.insert(
        "active_material_slot".into(),
        json!(&vm.active_material_slot),
    );
    obj.insert("settings_visible".into(), json!(&vm.settings_visible));
    obj.insert(
        "command_search_visible".into(),
        json!(&vm.command_search_visible),
    );
    obj.insert("overlay_top_id".into(), json!(&vm.overlay_top_id));
    obj.insert(
        "scene_drawer_visible".into(),
        json!(&vm.scene_drawer_visible),
    );
    obj.insert("active_keymap_id".into(), json!(&vm.active_keymap_id));
    obj.insert(
        "pointer_precision_mask".into(),
        json!(&vm.pointer_precision_mask),
    );
    obj.insert("pointer_snap_mask".into(), json!(&vm.pointer_snap_mask));
    obj.insert("pointer_extend_mask".into(), json!(&vm.pointer_extend_mask));
    obj.insert(
        "pointer_subtract_mask".into(),
        json!(&vm.pointer_subtract_mask),
    );
    obj.insert("pointer_pan_mask".into(), json!(&vm.pointer_pan_mask));
    obj.insert(
        "pointer_cursor_place_mask".into(),
        json!(&vm.pointer_cursor_place_mask),
    );
    obj.insert("pointer_adjust_mask".into(), json!(&vm.pointer_adjust_mask));
    obj.insert("pointer_loop_mask".into(), json!(&vm.pointer_loop_mask));
    obj.insert(
        "pointer_orbit_left_mask".into(),
        json!(&vm.pointer_orbit_left_mask),
    );
    obj.insert(
        "pointer_paint_straight_mask".into(),
        json!(&vm.pointer_paint_straight_mask),
    );
    obj.insert(
        "pointer_clone_source_mask".into(),
        json!(&vm.pointer_clone_source_mask),
    );
    obj.insert(
        "pointer_decal_scale_mask".into(),
        json!(&vm.pointer_decal_scale_mask),
    );
    obj.insert(
        "pointer_decal_rotate_mask".into(),
        json!(&vm.pointer_decal_rotate_mask),
    );
    obj.insert("animate".into(), animateviewmodel(&vm.animate));
    Value::Object(obj)
}
fn shortcutsmodel(vm: &petunia_editor::ShortcutsModel) -> Value {
    let mut obj = Map::new();
    obj.insert("undo".into(), json!(&vm.undo));
    obj.insert("redo".into(), json!(&vm.redo));
    obj.insert("save".into(), json!(&vm.save));
    obj.insert("open".into(), json!(&vm.open));
    obj.insert("search".into(), json!(&vm.search));
    obj.insert("settings".into(), json!(&vm.settings));
    obj.insert("parts".into(), json!(&vm.parts));
    obj.insert("model_select".into(), json!(&vm.model_select));
    obj.insert("model_lasso".into(), json!(&vm.model_lasso));
    obj.insert("model_cursor".into(), json!(&vm.model_cursor));
    obj.insert("model_measure".into(), json!(&vm.model_measure));
    obj.insert("model_move".into(), json!(&vm.model_move));
    obj.insert("model_rotate".into(), json!(&vm.model_rotate));
    obj.insert("model_scale".into(), json!(&vm.model_scale));
    obj.insert("model_transform".into(), json!(&vm.model_transform));
    obj.insert("model_add_primitive".into(), json!(&vm.model_add_primitive));
    obj.insert("model_duplicate".into(), json!(&vm.model_duplicate));
    obj.insert("model_extrude".into(), json!(&vm.model_extrude));
    obj.insert("model_push_pull".into(), json!(&vm.model_push_pull));
    obj.insert("model_inset".into(), json!(&vm.model_inset));
    obj.insert("model_bevel".into(), json!(&vm.model_bevel));
    obj.insert("model_knife".into(), json!(&vm.model_knife));
    obj.insert("model_loop_cut".into(), json!(&vm.model_loop_cut));
    obj.insert("model_profile".into(), json!(&vm.model_profile));
    obj.insert("model_subdivide".into(), json!(&vm.model_subdivide));
    obj.insert("model_merge".into(), json!(&vm.model_merge));
    obj.insert("model_slice".into(), json!(&vm.model_slice));
    obj.insert("model_connect".into(), json!(&vm.model_connect));
    obj.insert("model_spin".into(), json!(&vm.model_spin));
    obj.insert("model_dissolve".into(), json!(&vm.model_dissolve));
    obj.insert("model_delete".into(), json!(&vm.model_delete));
    obj.insert("select_point".into(), json!(&vm.select_point));
    obj.insert("select_edge".into(), json!(&vm.select_edge));
    obj.insert("select_face".into(), json!(&vm.select_face));
    obj.insert("select_object".into(), json!(&vm.select_object));
    obj.insert("view_proj".into(), json!(&vm.view_proj));
    obj.insert("view_frame".into(), json!(&vm.view_frame));
    obj.insert("view_reset".into(), json!(&vm.view_reset));
    obj.insert("view_wireframe".into(), json!(&vm.view_wireframe));
    obj.insert("view_solid".into(), json!(&vm.view_solid));
    obj.insert("view_material".into(), json!(&vm.view_material));
    obj.insert("view_lit".into(), json!(&vm.view_lit));
    obj.insert("view_xray".into(), json!(&vm.view_xray));
    obj.insert("view_pivot".into(), json!(&vm.view_pivot));
    obj.insert("view_snap".into(), json!(&vm.view_snap));
    obj.insert("view_prop".into(), json!(&vm.view_prop));
    obj.insert("paint_select".into(), json!(&vm.paint_select));
    obj.insert("paint_isolate".into(), json!(&vm.paint_isolate));
    obj.insert("paint_brush".into(), json!(&vm.paint_brush));
    obj.insert("paint_airbrush".into(), json!(&vm.paint_airbrush));
    obj.insert("paint_eraser".into(), json!(&vm.paint_eraser));
    obj.insert("paint_picker".into(), json!(&vm.paint_picker));
    obj.insert("paint_fill".into(), json!(&vm.paint_fill));
    obj.insert("paint_line".into(), json!(&vm.paint_line));
    obj.insert("paint_rectangle".into(), json!(&vm.paint_rectangle));
    obj.insert("paint_ellipse".into(), json!(&vm.paint_ellipse));
    obj.insert(
        "paint_gradient_radial".into(),
        json!(&vm.paint_gradient_radial),
    );
    obj.insert("paint_gradient".into(), json!(&vm.paint_gradient));
    obj.insert("uv_select".into(), json!(&vm.uv_select));
    obj.insert("uv_unwrap".into(), json!(&vm.uv_unwrap));
    obj.insert("uv_pack".into(), json!(&vm.uv_pack));
    obj.insert("uv_project_ref".into(), json!(&vm.uv_project_ref));
    obj.insert("uv_stitch".into(), json!(&vm.uv_stitch));
    obj.insert("uv_relax".into(), json!(&vm.uv_relax));
    Value::Object(obj)
}
fn themeentrymodel(vm: &petunia_editor::ThemeEntryModel) -> Value {
    let mut obj = Map::new();
    obj.insert("id".into(), json!(&vm.id));
    obj.insert("name".into(), json!(&vm.name));
    obj.insert("active".into(), json!(&vm.active));
    Value::Object(obj)
}
fn uveditormodel(vm: &petunia_editor::UvEditorModel) -> Value {
    let mut obj = Map::new();
    obj.insert("layout_commands".into(), json!(&vm.layout_commands));
    obj.insert("seam_commands".into(), json!(&vm.seam_commands));
    obj.insert("selected_commands".into(), json!(&vm.selected_commands));
    obj.insert("pinned_commands".into(), json!(&vm.pinned_commands));
    obj.insert("pinned_count".into(), json!(&vm.pinned_count));
    obj.insert("island_count".into(), json!(&vm.island_count));
    obj.insert("face_count".into(), json!(&vm.face_count));
    obj.insert("selected_face".into(), json!(&vm.selected_face));
    obj.insert("uv_selected_count".into(), json!(&vm.uv_selected_count));
    obj.insert("truncated".into(), json!(&vm.truncated));
    obj.insert(
        "islands".into(),
        Value::Array(vm.islands.iter().map(uvislandrowmodel).collect()),
    );
    Value::Object(obj)
}
fn uvislandrowmodel(vm: &petunia_editor::UvIslandRowModel) -> Value {
    let mut obj = Map::new();
    obj.insert("index".into(), json!(&vm.index));
    obj.insert("face_count".into(), json!(&vm.face_count));
    obj.insert("area".into(), json!(&vm.area));
    obj.insert("selected".into(), json!(&vm.selected));
    Value::Object(obj)
}
fn worldaxislabelmodel(vm: &petunia_editor::projection::WorldAxisLabelModel) -> Value {
    let mut obj = Map::new();
    obj.insert("text".into(), json!(&vm.text));
    obj.insert("x".into(), json!(&vm.x));
    obj.insert("y".into(), json!(&vm.y));
    obj.insert("color".into(), json!(&vm.color));
    Value::Object(obj)
}
pub(crate) fn snapshot(vm: &petunia_editor::ShellViewModel) -> Value {
    shellviewmodel(vm)
}

fn brushpanelmodel(vm: &petunia_editor::BrushPanelModel) -> Value {
    let mut obj = Map::new();
    obj.insert("flow".into(), json!(&vm.flow));
    obj.insert("spacing".into(), json!(&vm.spacing));
    obj.insert("smoothing".into(), json!(&vm.smoothing));
    obj.insert("size_jitter".into(), json!(&vm.size_jitter));
    obj.insert("opacity_jitter".into(), json!(&vm.opacity_jitter));
    obj.insert("scatter".into(), json!(&vm.scatter));
    obj.insert("spray_density".into(), json!(&vm.spray_density));
    obj.insert("angle".into(), json!(&vm.angle));
    obj.insert("roundness".into(), json!(&vm.roundness));
    obj.insert("tip".into(), json!(&vm.tip));
    obj.insert("blend".into(), json!(&vm.blend));
    obj.insert("presets".into(), json!(&vm.presets));
    obj.insert("active_preset".into(), json!(&vm.active_preset));
    obj.insert("clone_source_set".into(), json!(&vm.clone_source_set));
    Value::Object(obj)
}
fn keymapactionrow(vm: &petunia_editor::keymap_edit::KeymapActionRow) -> Value {
    let mut obj = Map::new();
    obj.insert("action".into(), json!(&vm.action));
    obj.insert("label".into(), json!(&vm.label));
    obj.insert("shortcut".into(), json!(&vm.shortcut));
    obj.insert("conflict".into(), json!(&vm.conflict));
    obj.insert("capturing".into(), json!(&vm.capturing));
    Value::Object(obj)
}
fn keymapprofilerow(vm: &petunia_editor::keymap_edit::KeymapProfileRow) -> Value {
    let mut obj = Map::new();
    obj.insert("id".into(), json!(&vm.id));
    obj.insert("name".into(), json!(&vm.name));
    obj.insert("custom".into(), json!(&vm.custom));
    obj.insert("active".into(), json!(&vm.active));
    Value::Object(obj)
}
fn keymapsnapshot(vm: &petunia_editor::keymap_edit::KeymapSnapshot) -> Value {
    let mut obj = Map::new();
    obj.insert(
        "profiles".into(),
        Value::Array(vm.profiles.iter().map(keymapprofilerow).collect()),
    );
    obj.insert(
        "rows".into(),
        Value::Array(vm.rows.iter().map(keymapactionrow).collect()),
    );
    Value::Object(obj)
}
