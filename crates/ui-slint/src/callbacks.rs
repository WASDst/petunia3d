// Slint UI callback registration and shell property synchronization.
// Registro de callbacks da UI Slint e sincronização de propriedades do shell.

use petunia_core::{SelectionDomain, Workspace};
use slint::ComponentHandle;
use std::sync::{Arc, Mutex};

use crate::overlay::OverlayId;
use crate::*;

/// Parâmetros numéricos expostos de um efeito de camada.
pub(crate) fn effect_params(
    effect: &petunia_project::paint_layers::PaintEffect,
) -> Vec<PaintEffectParam> {
    use petunia_project::paint_layers::PaintEffect;
    let param = |key: &str, label: &str, value: f32, min: f32, max: f32| PaintEffectParam {
        key: key.to_string(),
        label: label.to_string(),
        value,
        min,
        max,
    };
    match effect {
        PaintEffect::Pixelate { cell_size } => {
            vec![param(
                "cell_size",
                "Cell size",
                *cell_size as f32,
                1.0,
                64.0,
            )]
        }
        PaintEffect::Posterize { levels } => {
            vec![param("levels", "Levels", *levels as f32, 2.0, 32.0)]
        }
        PaintEffect::Invert => Vec::new(),
        PaintEffect::Grain { intensity, seed } => vec![
            param("intensity", "Intensity", *intensity, 0.0, 1.0),
            param("seed", "Seed", *seed as f32, 0.0, 9_999.0),
        ],
        PaintEffect::Levels {
            in_min,
            in_max,
            gamma,
            out_min,
            out_max,
        } => vec![
            param("in_min", "Input black", *in_min, 0.0, 1.0),
            param("in_max", "Input white", *in_max, 0.0, 1.0),
            param("gamma", "Gamma", *gamma, 0.1, 4.0),
            param("out_min", "Output black", *out_min, 0.0, 1.0),
            param("out_max", "Output white", *out_max, 0.0, 1.0),
        ],
        PaintEffect::BrightnessContrast {
            brightness,
            contrast,
        } => vec![
            param("brightness", "Brightness", *brightness, -1.0, 1.0),
            param("contrast", "Contrast", *contrast, -1.0, 1.0),
        ],
        PaintEffect::HueSaturation {
            hue_shift_deg,
            saturation,
        } => vec![
            param("hue_shift_deg", "Hue shift", *hue_shift_deg, -180.0, 180.0),
            param("saturation", "Saturation", *saturation, -1.0, 1.0),
        ],
    }
}

pub(crate) fn sync_overlay_models(
    window: &PetuniaSlintShell,
    selection: &SelectionOverlayModel,
    gizmo: &GizmoModel,
) {
    window.set_selection_overlay_visible(selection.visible);
    window.set_selection_outline_commands(selection.outline_commands.as_str().into());
    window.set_selection_active_outline_commands(selection.active_outline_commands.as_str().into());
    window.set_selection_point_commands(selection.point_commands.as_str().into());
    window.set_selection_unselected_outline_commands(
        selection.unselected_outline_commands.as_str().into(),
    );
    window.set_selection_unselected_point_commands(
        selection.unselected_point_commands.as_str().into(),
    );
    window.set_selection_overlay_accent(selection.accent);
    window.set_gizmo_visible(gizmo.visible);
    window.set_gizmo_origin_x(gizmo.origin_x);
    window.set_gizmo_origin_y(gizmo.origin_y);
    window.set_gizmo_x_commands(gizmo.x_commands.as_str().into());
    window.set_gizmo_y_commands(gizmo.y_commands.as_str().into());
    window.set_gizmo_z_commands(gizmo.z_commands.as_str().into());
    window.set_gizmo_x_end_x(gizmo.x_end[0]);
    window.set_gizmo_x_end_y(gizmo.x_end[1]);
    window.set_gizmo_y_end_x(gizmo.y_end[0]);
    window.set_gizmo_y_end_y(gizmo.y_end[1]);
    window.set_gizmo_z_end_x(gizmo.z_end[0]);
    window.set_gizmo_z_end_y(gizmo.z_end[1]);
    window.set_gizmo_x_label_x(gizmo.x_label[0]);
    window.set_gizmo_x_label_y(gizmo.x_label[1]);
    window.set_gizmo_y_label_x(gizmo.y_label[0]);
    window.set_gizmo_y_label_y(gizmo.y_label[1]);
    window.set_gizmo_z_label_x(gizmo.z_label[0]);
    window.set_gizmo_z_label_y(gizmo.z_label[1]);
    window.set_gizmo_x_arrow_commands(gizmo.x_arrow_commands.as_str().into());
    window.set_gizmo_y_arrow_commands(gizmo.y_arrow_commands.as_str().into());
    window.set_gizmo_z_arrow_commands(gizmo.z_arrow_commands.as_str().into());
    window.set_gizmo_x_scale_commands(gizmo.x_scale_commands.as_str().into());
    window.set_gizmo_y_scale_commands(gizmo.y_scale_commands.as_str().into());
    window.set_gizmo_z_scale_commands(gizmo.z_scale_commands.as_str().into());
    window.set_gizmo_x_rotate_commands(gizmo.x_rotate_commands.as_str().into());
    window.set_gizmo_y_rotate_commands(gizmo.y_rotate_commands.as_str().into());
    window.set_gizmo_z_rotate_commands(gizmo.z_rotate_commands.as_str().into());
    window.set_gizmo_plane_yz_commands(gizmo.plane_yz_commands.as_str().into());
    window.set_gizmo_plane_xz_commands(gizmo.plane_xz_commands.as_str().into());
    window.set_gizmo_plane_xy_commands(gizmo.plane_xy_commands.as_str().into());
    window.set_gizmo_view_roll_commands(gizmo.view_roll_commands.as_str().into());
    window.set_view_gizmo_x_commands(gizmo.view_x_commands.as_str().into());
    window.set_view_gizmo_y_commands(gizmo.view_y_commands.as_str().into());
    window.set_view_gizmo_z_commands(gizmo.view_z_commands.as_str().into());
    window.set_view_gizmo_x_end_x(gizmo.view_x_end[0]);
    window.set_view_gizmo_x_end_y(gizmo.view_x_end[1]);
    window.set_view_gizmo_y_end_x(gizmo.view_y_end[0]);
    window.set_view_gizmo_y_end_y(gizmo.view_y_end[1]);
    window.set_view_gizmo_z_end_x(gizmo.view_z_end[0]);
    window.set_view_gizmo_z_end_y(gizmo.view_z_end[1]);
    window.set_view_gizmo_origin_x(gizmo.view_origin_x);
    window.set_view_gizmo_origin_y(gizmo.view_origin_y);
    window.set_cursor_screen_x(gizmo.cursor_screen[0]);
    window.set_cursor_screen_y(gizmo.cursor_screen[1]);
    window.set_cursor_visible(gizmo.cursor_visible);
}

pub(crate) fn sync_viewport_overlays<V: PetuniaViewport>(
    window: &PetuniaSlintShell,
    bridge: &SlintUiBridge<V>,
) {
    puffin::profile_function!();
    let [width, height] = bridge.viewport_size;
    let selection = compute_selection_overlay(
        &bridge.state,
        width,
        height,
        bridge.viewport.draws_component_guides(),
    );
    let mut gizmo = compute_gizmo(&bridge.state, width, height);
    if bridge.viewport.draws_gizmo() {
        gizmo.clear_handle_geometry();
    }
    sync_overlay_models(window, &selection, &gizmo);
    // Cordão da ferramenta ativa: arrasto na viewport, modal de teclado
    // (Move/Rotate/Scale/Extrude/...) ou modal paramétrico com Tool Props.
    let link_active = bridge.drag.is_some()
        || bridge.tool_modal.is_some()
        || bridge.state.session.tools.modal.is_some();
    let drag_link = compute_drag_link(
        &bridge.state,
        width,
        height,
        bridge.pointer_position,
        link_active,
    );
    window.set_drag_link_commands(drag_link.as_str().into());
    let prop_circle = compute_proportional_circle(&bridge.state, width, height, link_active);
    window.set_proportional_circle_commands(prop_circle.as_str().into());

    let axis_guide = compute_axis_guide(&bridge.state, width, height);
    window.set_axis_guide_visible(axis_guide.visible);
    window.set_axis_guide_commands(axis_guide.commands.as_str().into());
    window.set_axis_guide_color(slint::Color::from_rgb_u8(
        axis_guide.color[0],
        axis_guide.color[1],
        axis_guide.color[2],
    ));
    window.set_axis_guide_label(axis_guide.label.as_str().into());
    window.set_axis_guide_label_x(axis_guide.label_x);
    window.set_axis_guide_label_y(axis_guide.label_y);

    let dimension = compute_dimension_annotation(&bridge.state, width, height);
    window.set_dimension_visible(dimension.visible);
    window.set_dimension_commands(dimension.commands.as_str().into());
    window.set_dimension_text(dimension.text.as_str().into());
    window.set_dimension_x(dimension.label_x);
    window.set_dimension_y(dimension.label_y);

    let snap_marker = bridge.snap_marker_model(width, height);
    window.set_snap_marker_visible(snap_marker.visible);
    window.set_snap_marker_x(snap_marker.x);
    window.set_snap_marker_y(snap_marker.y);
    window.set_snap_marker_label(snap_marker.label.as_str().into());
    window.set_snap_marker_round(snap_marker.round);

    let measure = compute_quick_measure(&bridge.state, width, height);
    window.set_measure_visible(measure.visible);
    window.set_measure_commands(measure.commands.as_str().into());
    window.set_measure_text(measure.text.as_str().into());
    window.set_measure_x(measure.label_x);
    window.set_measure_y(measure.label_y);
    window.set_measure_distance(measure.distance);
    window.set_measure_dx(measure.dx);
    window.set_measure_dy(measure.dy);
    window.set_measure_dz(measure.dz);
    window.set_measure_angle_deg(measure.angle_deg);
    window.set_measure_hud_text(measure.hud_text.as_str().into());
    let measure_tags: Vec<MeasureTag> = measure
        .tags
        .iter()
        .map(|tag| MeasureTag {
            text: tag.text.as_str().into(),
            x: tag.x,
            y: tag.y,
        })
        .collect();
    crate::refresh::set_model_if_changed(window.get_measure_tags(), measure_tags, |model| {
        window.set_measure_tags(model)
    });

    let world_axis_labels = compute_world_axis_labels(&bridge.state, width, height);
    let world_axis_tags: Vec<WorldAxisTag> = world_axis_labels
        .iter()
        .map(|tag| WorldAxisTag {
            text: tag.text.as_str().into(),
            x: tag.x,
            y: tag.y,
            tint: slint::Color::from_rgb_u8(tag.color[0], tag.color[1], tag.color[2]),
        })
        .collect();
    crate::refresh::set_model_if_changed(
        window.get_world_axis_labels(),
        world_axis_tags,
        |model| window.set_world_axis_labels(model),
    );

    window.set_micro_inspector_open(bridge.micro_inspector_open);
    window.set_micro_inspector_x(bridge.micro_inspector_pos[0]);
    window.set_micro_inspector_y(bridge.micro_inspector_pos[1]);

    let protractor = compute_protractor(&bridge.state, width, height, bridge.drag.as_ref());
    window.set_protractor_visible(protractor.visible);
    window.set_protractor_wedge_commands(protractor.wedge_commands.as_str().into());
    window.set_protractor_ticks_commands(protractor.ticks_commands.as_str().into());
    sync_draw_camera_overlays(window, bridge);
}

/// Overlays do DRAW/POLY que projetam pontos de mundo com a câmera corrente.
///
/// Orbitar, mover e dar zoom só passam por `sync_viewport_overlays`; sem refazer
/// estes traçados a região, o contorno e a prévia do perfil ficavam parados na
/// posição da câmera antiga (a "mancha" laranja desalinhada da malha).
pub(crate) fn sync_draw_camera_overlays<V: PetuniaViewport>(
    window: &PetuniaSlintShell,
    bridge: &SlintUiBridge<V>,
) {
    window.set_region_hover_commands(bridge.region_hover_commands().as_str().into());
    window.set_profile_selection_commands(bridge.profile_selection_commands().as_str().into());
    window.set_region_shapes_commands(bridge.region_shapes_commands().as_str().into());
    window.set_profile_outline_commands(bridge.profile_outline_commands().as_str().into());
    window.set_poly_pen_preview_commands(bridge.poly_pen_preview_commands().as_str().into());
    if bridge.state.session.tools.active_tool == "draw_profile" {
        window.set_profile_preview_commands(bridge.profile_preview_commands().as_str().into());
    }
    let (depth_commands, depth_pos, depth_label) = bridge.depth_handle_commands();
    window.set_depth_handle_commands(depth_commands.as_str().into());
    window.set_depth_handle_visible(depth_pos.is_some());
    if let Some([dx, dy]) = depth_pos {
        window.set_depth_handle_x(dx * 1.0);
        window.set_depth_handle_y(dy * 1.0);
    }
    window.set_depth_handle_label(depth_label.as_str().into());
    let (handle_commands, handle_pos) = bridge.parametric_handle_commands();
    window.set_parametric_handle_commands(handle_commands.as_str().into());
    window.set_parametric_handle_visible(handle_pos.is_some());
    window.set_parametric_handle_hover(bridge.parametric_handle_hover);
    if let Some([hx, hy]) = handle_pos {
        window.set_parametric_handle_x(hx * 1.0);
        window.set_parametric_handle_y(hy * 1.0);
    }
}

impl From<&crate::view_model::ShortcutsModel> for ShortcutsEntry {
    fn from(s: &crate::view_model::ShortcutsModel) -> Self {
        Self {
            undo: s.undo.as_str().into(),
            redo: s.redo.as_str().into(),
            save: s.save.as_str().into(),
            open: s.open.as_str().into(),
            search: s.search.as_str().into(),
            settings: s.settings.as_str().into(),
            parts: s.parts.as_str().into(),
            model_select: s.model_select.as_str().into(),
            model_lasso: s.model_lasso.as_str().into(),
            model_cursor: s.model_cursor.as_str().into(),
            model_measure: s.model_measure.as_str().into(),
            model_move: s.model_move.as_str().into(),
            model_rotate: s.model_rotate.as_str().into(),
            model_scale: s.model_scale.as_str().into(),
            model_transform: s.model_transform.as_str().into(),
            model_add_primitive: s.model_add_primitive.as_str().into(),
            model_duplicate: s.model_duplicate.as_str().into(),
            model_extrude: s.model_extrude.as_str().into(),
            model_push_pull: s.model_push_pull.as_str().into(),
            model_inset: s.model_inset.as_str().into(),
            model_bevel: s.model_bevel.as_str().into(),
            model_knife: s.model_knife.as_str().into(),
            model_loop_cut: s.model_loop_cut.as_str().into(),
            model_profile: s.model_profile.as_str().into(),
            model_subdivide: s.model_subdivide.as_str().into(),
            model_merge: s.model_merge.as_str().into(),
            model_slice: s.model_slice.as_str().into(),
            model_connect: s.model_connect.as_str().into(),
            model_spin: s.model_spin.as_str().into(),
            model_dissolve: s.model_dissolve.as_str().into(),
            model_delete: s.model_delete.as_str().into(),
            select_point: s.select_point.as_str().into(),
            select_edge: s.select_edge.as_str().into(),
            select_face: s.select_face.as_str().into(),
            select_object: s.select_object.as_str().into(),
            view_proj: s.view_proj.as_str().into(),
            view_frame: s.view_frame.as_str().into(),
            view_reset: s.view_reset.as_str().into(),
            view_wireframe: s.view_wireframe.as_str().into(),
            view_solid: s.view_solid.as_str().into(),
            view_material: s.view_material.as_str().into(),
            view_lit: s.view_lit.as_str().into(),
            view_xray: s.view_xray.as_str().into(),
            view_pivot: s.view_pivot.as_str().into(),
            view_snap: s.view_snap.as_str().into(),
            view_prop: s.view_prop.as_str().into(),
            paint_select: s.paint_select.as_str().into(),
            paint_isolate: s.paint_isolate.as_str().into(),
            paint_brush: s.paint_brush.as_str().into(),
            paint_airbrush: s.paint_airbrush.as_str().into(),
            paint_eraser: s.paint_eraser.as_str().into(),
            paint_picker: s.paint_picker.as_str().into(),
            paint_fill: s.paint_fill.as_str().into(),
            paint_line: s.paint_line.as_str().into(),
            paint_rectangle: s.paint_rectangle.as_str().into(),
            paint_ellipse: s.paint_ellipse.as_str().into(),
            paint_gradient_radial: s.paint_gradient_radial.as_str().into(),
            paint_gradient: s.paint_gradient.as_str().into(),
            uv_select: s.uv_select.as_str().into(),
            uv_unwrap: s.uv_unwrap.as_str().into(),
            uv_pack: s.uv_pack.as_str().into(),
            uv_project_ref: s.uv_project_ref.as_str().into(),
            uv_stitch: s.uv_stitch.as_str().into(),
            uv_relax: s.uv_relax.as_str().into(),
        }
    }
}

/// Escala do sistema capturada na primeira aplicação (bits de f32; 0 = ainda não lida).
static SYSTEM_SCALE_FACTOR_BITS: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(0);

/// Aplica a preferência de escala de UI (100–200 %) sobre a escala do sistema.
///
/// A escala do sistema é lida uma única vez; depois o fator efetivo é
/// `sistema × preferência`. Só despacha o evento quando o fator muda, então é
/// seguro chamar a cada sincronização de propriedades.
pub(crate) fn apply_ui_scale(window: &PetuniaSlintShell, preference: f32) {
    use std::sync::atomic::Ordering;
    let slint_window = window.window();
    let mut system = f32::from_bits(SYSTEM_SCALE_FACTOR_BITS.load(Ordering::Relaxed));
    if !(system.is_finite() && system > 0.0) {
        system = slint_window.scale_factor();
        if !(system.is_finite() && system > 0.0) {
            return;
        }
        SYSTEM_SCALE_FACTOR_BITS.store(system.to_bits(), Ordering::Relaxed);
    }
    let target = system * preference.clamp(1.0, 2.0);
    if (slint_window.scale_factor() - target).abs() > 0.001 {
        slint_window.dispatch_event(slint::platform::WindowEvent::ScaleFactorChanged {
            scale_factor: target,
        });
    }
}

fn to_prefab_item(item: &crate::view_model::PrefabItemModel) -> PrefabItem {
    let side = crate::thumbnail::THUMBNAIL_SIZE;
    let image = item.thumbnail.as_ref().map(|rgba| {
        slint::Image::from_rgba8(
            slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(rgba, side, side),
        )
    });
    PrefabItem {
        id: item.id.as_str().into(),
        name: item.name.as_str().into(),
        parts: item.parts as i32,
        tris: item.tris as i32,
        verts: item.verts as i32,
        favorite: item.favorite,
        instances: item.instances as i32,
        tags: item.tags.as_str().into(),
        has_thumbnail: image.is_some(),
        thumbnail: image.unwrap_or_default(),
    }
}

pub(crate) fn sync_window_properties(window: &PetuniaSlintShell, vm: &ShellViewModel) {
    puffin::profile_function!();
    window.set_active_workspace(vm.workspace_label().into());
    crate::animate::sync_animate_properties(window, &vm.animate);
    window.set_saved(vm.saved);
    window.set_can_undo(vm.can_undo);
    window.set_can_redo(vm.can_redo);
    window.set_shortcuts((&vm.shortcuts).into());
    window.set_status_severity(vm.status_severity);
    window.set_status_message(vm.status_message.as_str().into());
    window.set_active_tool(vm.active_tool.as_str().into());
    let domain_str = match vm.selection_domain {
        SelectionDomain::Object => "OBJECT",
        SelectionDomain::Vertex => "POINT",
        SelectionDomain::Edge => "EDGE",
        SelectionDomain::Face => "FACE",
    };
    window.set_selection_domain(domain_str.into());
    window.set_is_orthographic(vm.is_orthographic);
    window.set_is_wireframe(vm.is_wireframe);
    window.set_shading_mode(vm.shading_mode.as_str().into());
    window.set_show_xray(vm.show_xray);
    window.set_shading_popover_open(vm.shading_popover_open);
    window.set_transform_instant_active(vm.transform_instant_active);
    window.set_gizmo_hover_axis(vm.gizmo_hover_axis);
    window.set_gizmo_active_axis(vm.gizmo_active_axis);
    window.set_gizmo_constraint_axis_a(vm.gizmo_constraint_axes[0]);
    window.set_gizmo_constraint_axis_b(vm.gizmo_constraint_axes[1]);
    window.set_gizmo_has_hover(vm.gizmo_has_hover);
    window.set_gizmo_center_active(vm.gizmo_center_active);
    window.set_gizmo_center_hover(vm.gizmo_center_hover);
    window.set_operation_preview_commands(vm.operation_preview_commands.as_str().into());
    window.set_drag_link_commands(vm.drag_link_commands.as_str().into());
    window.set_proportional_circle_commands(vm.proportional_circle_commands.as_str().into());
    window.set_operation_hud_active(vm.operation_hud_active);
    window.set_operation_hud_title(vm.operation_hud_title.as_str().into());
    window.set_operation_hud_subject(vm.operation_hud_subject.as_str().into());
    window.set_operation_hud_hint(vm.operation_hud_hint.as_str().into());
    window.set_context_hint(vm.context_hint.as_str().into());
    window.set_hover_label(vm.hover_label.as_str().into());
    window.set_selection_summary(vm.selection_summary.as_str().into());
    let hud_lines: Vec<slint::SharedString> = vm
        .operation_hud_lines
        .iter()
        .map(|line| line.as_str().into())
        .collect();
    window.set_operation_hud_lines(hud_lines.as_slice().into());
    window.set_xray_opacity(vm.xray_opacity);
    window.set_asset_library_visible(vm.asset_library_visible);
    window.set_paint_color(slint::Color::from_argb_f32(
        1.0,
        vm.paint_color[0],
        vm.paint_color[1],
        vm.paint_color[2],
    ));
    window.set_brush_size(vm.brush_size);
    window.set_brush_opacity(vm.brush_opacity);
    window.set_brush_hardness(vm.brush_hardness);
    window.set_paint_symmetry_x(vm.paint_symmetry_x);
    window.set_paint_symmetry_y(vm.paint_symmetry_y);
    window.set_paint_symmetry_z(vm.paint_symmetry_z);
    window.set_paint_target_vertex(vm.paint_target_vertex);
    window.set_paint_mask_selection(vm.paint_mask_selection);
    window.set_isolate_active(vm.isolate_active);

    window.set_pos_x(vm.position[0]);
    window.set_pos_y(vm.position[1]);
    window.set_pos_z(vm.position[2]);
    window.set_rot_x(vm.rotation[0]);
    window.set_rot_y(vm.rotation[1]);
    window.set_rot_z(vm.rotation[2]);
    window.set_scale_x(vm.scale[0]);
    window.set_scale_y(vm.scale[1]);
    window.set_scale_z(vm.scale[2]);
    window.set_cursor_x(vm.cursor_3d[0]);
    window.set_cursor_y(vm.cursor_3d[1]);
    window.set_cursor_z(vm.cursor_3d[2]);

    window.set_active_object_title(vm.active_object_title.as_str().into());
    window.set_active_object_details(vm.active_object_details.as_str().into());
    window.set_active_material_name(vm.active_material_name.as_str().into());
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
    window.set_uv_stats(vm.uv_stats.as_str().into());
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
    window.set_extension_radius(vm.draw_extension_values[0]);
    window.set_extension_length(vm.draw_extension_values[1]);
    window.set_extension_angle(vm.draw_extension_values[2]);
    window.set_extension_sides(vm.draw_extension_values[3]);
    window.set_extension_tolerance(vm.draw_extension_values[4]);
    window.set_export_padding(vm.export_padding);
    window.set_trace_threshold(vm.trace_threshold);
    window.set_stencil_visible(vm.stencil_image.is_some());
    window.set_stencil_image(vm.stencil_image.clone().unwrap_or_default());
    window.set_proportional_editing(vm.proportional_editing);
    window.set_proportional_radius(vm.proportional_radius);
    window.set_proportional_falloff(vm.proportional_falloff.as_str().into());
    window.set_snap_enabled(vm.snap_enabled);
    window.set_snap_target(vm.snap_target.as_str().into());

    sync_overlay_models(window, &vm.selection_overlay, &vm.gizmo);
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
    window.set_menu_file_label(vm.menu_file_label.as_str().into());
    window.set_menu_edit_label(vm.menu_edit_label.as_str().into());
    window.set_menu_view_label(vm.menu_view_label.as_str().into());
    window.set_menu_window_label(vm.menu_window_label.as_str().into());
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
    window.set_label_parts(vm.label_parts.as_str().into());
    window.set_label_project_asset_library(vm.label_project_asset_library.as_str().into());
    window.set_label_save_active_as_asset(vm.label_save_active_as_asset.as_str().into());
    window.set_label_status_hint(vm.label_status_hint.as_str().into());
    window.set_label_unwrap_mesh(vm.label_unwrap_mesh.as_str().into());
    window.set_label_pack_islands(vm.label_pack_islands.as_str().into());
    window.set_label_active_brush_color(vm.label_active_brush_color.as_str().into());
    window.set_label_albedo_base_color(vm.label_albedo_base_color.as_str().into());
    window.set_label_theme(vm.label_theme.as_str().into());
    window.set_label_place_in_scene(vm.label_place_in_scene.as_str().into());
    window.set_label_vertical_tool_drag(vm.label_vertical_tool_drag.as_str().into());
    window.set_label_invert_vertical_drag(vm.label_invert_vertical_drag.as_str().into());
    window.set_label_search_assets(vm.label_search_assets.as_str().into());
    window.set_label_search_parts(vm.label_search_parts.as_str().into());
    window.set_label_inspector(vm.label_inspector.as_str().into());
    window.set_label_resize_panel_width(vm.label_resize_panel_width.as_str().into());
    window.set_label_section_pin_open(vm.label_section_pin_open.as_str().into());
    window.set_label_expand_inspector(vm.label_expand_inspector.as_str().into());
    window.set_label_collapse_inspector(vm.label_collapse_inspector.as_str().into());
    window.set_label_tab_parts(vm.label_tab_parts.as_str().into());
    window.set_label_tab_transform(vm.label_tab_transform.as_str().into());
    window.set_label_tab_material(vm.label_tab_material.as_str().into());
    window.set_label_tab_object(vm.label_tab_object.as_str().into());
    window.set_label_tab_modifiers(vm.label_tab_modifiers.as_str().into());
    window.set_label_object_name(vm.label_object_name.as_str().into());
    window.set_label_object_visibility(vm.label_object_visibility.as_str().into());
    window.set_label_object_lock(vm.label_object_lock.as_str().into());
    window.set_label_object_no_selection(vm.label_object_no_selection.as_str().into());
    window.set_label_stats_faces(vm.label_stats_faces.as_str().into());
    window.set_label_stats_verts(vm.label_stats_verts.as_str().into());
    window.set_label_stats_tris(vm.label_stats_tris.as_str().into());
    window.set_label_stats_selection(vm.label_stats_selection.as_str().into());
    window.set_label_tool_options(vm.label_tool_options.as_str().into());
    window.set_label_tool_options_expand(vm.label_tool_options_expand.as_str().into());
    window.set_label_tool_options_collapse(vm.label_tool_options_collapse.as_str().into());
    window.set_label_quick_actions(vm.label_quick_actions.as_str().into());
    window.set_label_quick_action_customize(vm.label_quick_action_customize.as_str().into());
    window.set_label_quick_action_add(vm.label_quick_action_add.as_str().into());
    window.set_label_quick_action_remove(vm.label_quick_action_remove.as_str().into());
    window.set_label_quick_action_reset(vm.label_quick_action_reset.as_str().into());
    window.set_label_quick_action_done(vm.label_quick_action_done.as_str().into());
    window.set_label_action_subdivide(vm.label_action_subdivide.as_str().into());
    window.set_label_action_fuse(vm.label_action_fuse.as_str().into());
    window.set_label_action_cut(vm.label_action_cut.as_str().into());
    window.set_label_action_intersect(vm.label_action_intersect.as_str().into());
    window.set_label_action_join(vm.label_action_join.as_str().into());
    window.set_label_action_merge(vm.label_action_merge.as_str().into());
    window.set_label_action_slice(vm.label_action_slice.as_str().into());
    window.set_label_action_loop_cut(vm.label_action_loop_cut.as_str().into());
    window.set_label_material_base_color(vm.label_material_base_color.as_str().into());
    window.set_label_material_profile(vm.label_material_profile.as_str().into());
    window.set_label_material_roughness(vm.label_material_roughness.as_str().into());
    window.set_label_material_metallic(vm.label_material_metallic.as_str().into());
    window.set_label_material_normal_scale(vm.label_material_normal_scale.as_str().into());
    window.set_label_material_advanced(vm.label_material_advanced.as_str().into());
    window.set_label_material_assign(vm.label_material_assign.as_str().into());
    window.set_label_material_new(vm.label_material_new.as_str().into());
    window.set_label_material_duplicate(vm.label_material_duplicate.as_str().into());
    window.set_label_material_remove(vm.label_material_remove.as_str().into());
    window.set_label_material_no_material(vm.label_material_no_material.as_str().into());
    window.set_label_material_no_selection(vm.label_material_no_selection.as_str().into());
    window
        .set_label_material_emission_strength(vm.label_material_emission_strength.as_str().into());
    window.set_label_material_alpha_cutoff(vm.label_material_alpha_cutoff.as_str().into());
    window.set_label_material_texture_albedo(vm.label_material_texture_albedo.as_str().into());
    window.set_label_material_no_texture(vm.label_material_no_texture.as_str().into());
    window.set_label_material_create_texture(vm.label_material_create_texture.as_str().into());
    window.set_label_material_clear_texture(vm.label_material_clear_texture.as_str().into());
    window.set_label_material_profile_pbr(vm.label_material_profile_pbr.as_str().into());
    window.set_label_material_profile_unlit(vm.label_material_profile_unlit.as_str().into());
    window.set_label_material_profile_toon(vm.label_material_profile_toon.as_str().into());
    window.set_label_material_profile_glass(vm.label_material_profile_glass.as_str().into());
    window.set_label_material_profile_emissive(vm.label_material_profile_emissive.as_str().into());
    window.set_label_material_alpha_opaque(vm.label_material_alpha_opaque.as_str().into());
    window.set_label_material_alpha_mask(vm.label_material_alpha_mask.as_str().into());
    window.set_label_material_alpha_blend(vm.label_material_alpha_blend.as_str().into());
    window.set_label_modifier_mirror(vm.label_modifier_mirror.as_str().into());
    window.set_label_modifier_symmetry(vm.label_modifier_symmetry.as_str().into());
    window.set_label_modifiers_empty(vm.label_modifiers_empty.as_str().into());
    window.set_label_modifier_apply(vm.label_modifier_apply.as_str().into());
    window.set_label_modifier_axis(vm.label_modifier_axis.as_str().into());
    window.set_label_modifier_add_mirror(vm.label_modifier_add_mirror.as_str().into());
    window.set_label_modifier_add_symmetry(vm.label_modifier_add_symmetry.as_str().into());
    window.set_label_modifier_remove(vm.label_modifier_remove.as_str().into());
    window.set_label_modifier_move_up(vm.label_modifier_move_up.as_str().into());
    window.set_label_modifier_move_down(vm.label_modifier_move_down.as_str().into());
    window.set_label_modifier_direction(vm.label_modifier_direction.as_str().into());
    window.set_label_modifier_positive_to_negative(
        vm.label_modifier_positive_to_negative.as_str().into(),
    );
    window.set_label_modifier_negative_to_positive(
        vm.label_modifier_negative_to_positive.as_str().into(),
    );
    window.set_label_numeric_field_hint(vm.label_numeric_field_hint.as_str().into());
    window.set_label_model_select(vm.label_model_select.as_str().into());
    window.set_label_model_position(vm.label_model_position.as_str().into());
    window.set_label_model_rotate(vm.label_model_rotate.as_str().into());
    window.set_label_model_scale(vm.label_model_scale.as_str().into());
    window.set_label_model_transform(vm.label_model_transform.as_str().into());
    window.set_label_model_lasso(vm.label_model_lasso.as_str().into());
    window.set_label_model_loop_cut(vm.label_model_loop_cut.as_str().into());
    window.set_label_model_slice(vm.label_model_slice.as_str().into());
    window.set_label_model_push_pull(vm.label_model_push_pull.as_str().into());
    window.set_label_model_profile(vm.label_model_profile.as_str().into());
    window.set_label_model_pivot(vm.label_model_pivot.as_str().into());
    window.set_label_profile_depth(vm.label_profile_depth.as_str().into());
    window.set_label_profile_points(vm.label_profile_points.as_str().into());
    window.set_label_profile_close(vm.label_profile_close.as_str().into());
    window.set_label_view_gizmo(vm.label_view_gizmo.as_str().into());
    window.set_label_view_gizmo_hint(vm.label_view_gizmo_hint.as_str().into());
    window.set_label_profile_plane(vm.label_profile_plane.as_str().into());
    window.set_label_profile_plane_auto(vm.label_profile_plane_auto.as_str().into());
    window.set_label_profile_plane_ground(vm.label_profile_plane_ground.as_str().into());
    window.set_label_profile_plane_face(vm.label_profile_plane_face.as_str().into());
    window.set_label_profile_plane_view(vm.label_profile_plane_view.as_str().into());
    window.set_label_profile_look_at_plane(vm.label_profile_look_at_plane.as_str().into());
    window.set_profile_workplane_locked(vm.profile_workplane_locked);
    window.set_label_profile_generate(vm.label_profile_generate.as_str().into());
    window.set_label_profile_revolve(vm.label_profile_revolve.as_str().into());
    window.set_label_profile_sweep(vm.label_profile_sweep.as_str().into());
    window.set_label_profile_cuts(vm.label_profile_cuts.as_str().into());
    window.set_label_profile_presets(vm.label_profile_presets.as_str().into());
    window.set_label_profile_add_rect(vm.label_profile_add_rect.as_str().into());
    window.set_label_profile_add_circle(vm.label_profile_add_circle.as_str().into());
    window.set_label_profile_canvas_hint(vm.label_profile_canvas_hint.as_str().into());
    window.set_label_profile_wall_thickness(vm.label_profile_wall_thickness.as_str().into());
    window.set_label_profile_smooth_curves(vm.label_profile_smooth_curves.as_str().into());
    window.set_label_profile_sharp_corners(vm.label_profile_sharp_corners.as_str().into());
    window.set_label_profile_smoothness(vm.label_profile_smoothness.as_str().into());
    window.set_label_decal_transform(vm.label_decal_transform.as_str().into());
    window.set_label_decal_position(vm.label_decal_position.as_str().into());
    window.set_label_decal_scale(vm.label_decal_scale.as_str().into());
    window.set_label_decal_width(vm.label_decal_width.as_str().into());
    window.set_label_decal_rotation(vm.label_decal_rotation.as_str().into());
    window.set_label_decal_bake(vm.label_decal_bake.as_str().into());
    window.set_label_decal_hint(vm.label_decal_hint.as_str().into());
    window.set_decal_preview_commands(vm.decal_preview_commands.as_str().into());
    window.set_hint_model_select(vm.hint_model_select.as_str().into());
    window.set_hint_model_position(vm.hint_model_position.as_str().into());
    window.set_hint_model_rotate(vm.hint_model_rotate.as_str().into());
    window.set_hint_model_scale(vm.hint_model_scale.as_str().into());
    window.set_hint_model_transform(vm.hint_model_transform.as_str().into());
    window.set_hint_model_lasso(vm.hint_model_lasso.as_str().into());
    window.set_hint_model_loop_cut(vm.hint_model_loop_cut.as_str().into());
    window.set_hint_model_slice(vm.hint_model_slice.as_str().into());
    window.set_hint_model_push_pull(vm.hint_model_push_pull.as_str().into());
    window.set_hint_model_profile(vm.hint_model_profile.as_str().into());
    window.set_hint_model_pivot(vm.hint_model_pivot.as_str().into());
    window.set_label_hide_part(vm.label_hide_part.as_str().into());
    window.set_label_show_part(vm.label_show_part.as_str().into());
    window.set_label_lock_part(vm.label_lock_part.as_str().into());
    window.set_label_unlock_part(vm.label_unlock_part.as_str().into());
    window.set_label_selected_parts_only(vm.label_selected_parts_only.as_str().into());
    window.set_label_sort_parts(vm.label_sort_parts.as_str().into());
    window.set_label_parts_row_size(vm.label_parts_row_size.as_str().into());
    window.set_label_sort_assets(vm.label_sort_assets.as_str().into());
    window.set_label_thumbnail_size(vm.label_thumbnail_size.as_str().into());
    window.set_label_selection_color(vm.label_selection_color.as_str().into());
    window.set_label_highlight_thickness(vm.label_highlight_thickness.as_str().into());
    window.set_label_view_wireframe(vm.label_view_wireframe.as_str().into());
    window.set_label_view_wireframe_hint(vm.label_view_wireframe_hint.as_str().into());
    window.set_label_view_solid(vm.label_view_solid.as_str().into());
    window.set_label_view_solid_hint(vm.label_view_solid_hint.as_str().into());
    window.set_label_view_material(vm.label_view_material.as_str().into());
    window.set_label_view_material_hint(vm.label_view_material_hint.as_str().into());
    window.set_label_view_lit(vm.label_view_lit.as_str().into());
    window.set_label_view_lit_hint(vm.label_view_lit_hint.as_str().into());
    window.set_label_more_model_tools(vm.label_more_model_tools.as_str().into());
    window.set_label_xray_opacity(vm.label_xray_opacity.as_str().into());
    window.set_label_wire_overlay(vm.label_wire_overlay.as_str().into());
    window.set_label_wire_overlay_hint(vm.label_wire_overlay_hint.as_str().into());
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
    window.set_recovery_open(vm.recovery_open);
    window.set_recovery_title(vm.recovery_title.as_str().into());
    window.set_recovery_body(vm.recovery_body.as_str().into());
    window.set_recovery_detail(vm.recovery_detail.as_str().into());
    window.set_recovery_recover(vm.recovery_recover.as_str().into());
    window.set_recovery_keep(vm.recovery_keep.as_str().into());
    window.set_recovery_discard(vm.recovery_discard.as_str().into());
    window.set_label_asset_library(vm.label_asset_library.as_str().into());
    window.set_label_preferences(vm.label_preferences.as_str().into());
    window.set_shell_info(vm.shell_info.as_str().into());
    window.set_label_apply(vm.label_apply.as_str().into());
    window.set_label_cancel(vm.label_cancel.as_str().into());
    window.set_label_delete(vm.label_delete.as_str().into());
    window.set_label_duplicate(vm.label_duplicate.as_str().into());
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
    window.set_inspector_width(vm.inspector_width);
    window.set_asset_library_height(vm.asset_library_height);
    window.set_reference_manager_visible(vm.reference_manager_open);
    window.set_reference_total_count(vm.reference_total_count);
    let ref_slots: Vec<ReferenceSlotData> = vm
        .reference_slots
        .iter()
        .map(|s| ReferenceSlotData {
            axis: s.axis.as_str().into(),
            axis_tag: s.axis_tag.as_str().into(),
            title: s.title.as_str().into(),
            has_image: s.has_image,
            image_name: s.image_name.as_str().into(),
            dimensions: s.dimensions.as_str().into(),
            visible: s.visible,
            locked: s.locked,
            opacity: s.opacity,
            size: s.size,
            offset: s.offset,
            rotation: s.rotation,
            fine_open: s.fine_open,
            thumbnail: s.thumbnail.clone().unwrap_or_default(),
        })
        .collect();
    crate::refresh::set_model_if_changed(window.get_reference_slots(), ref_slots, |model| {
        window.set_reference_slots(model)
    });
    window.set_uv_layout_commands(vm.uv_editor.layout_commands.as_str().into());
    window.set_uv_seam_commands(vm.uv_editor.seam_commands.as_str().into());
    window.set_uv_pinned_commands(vm.uv_editor.pinned_commands.as_str().into());
    window.set_uv_selected_commands(vm.uv_editor.selected_commands.as_str().into());
    window.set_uv_island_count(vm.uv_editor.island_count as i32);
    window.set_uv_face_count(vm.uv_editor.face_count as i32);
    window.set_uv_selected_face(vm.uv_editor.selected_face);
    window.set_uv_selected_count(vm.uv_editor.uv_selected_count as i32);
    window.set_uv_layout_truncated(vm.uv_editor.truncated);
    window.set_paint_layer_count(vm.paint_layer_count.as_str().into());
    window.set_paint_effect_kind(vm.paint_effect_kind.as_str().into());
    let effect_params: Vec<PaintEffectParamEntry> = vm
        .paint_effect_params
        .iter()
        .map(|param| PaintEffectParamEntry {
            key: param.key.as_str().into(),
            label: param.label.as_str().into(),
            value: param.value,
            minimum: param.min,
            maximum: param.max,
        })
        .collect();
    window.set_paint_effect_params(effect_params.as_slice().into());
    window.set_paint_canvas_size(vm.paint_canvas_size.as_str().into());
    window.set_paint_canvas_revision(vm.paint_canvas_revision);
    window.set_paint_fill_scope(vm.paint_fill_scope.as_str().into());
    window.set_paint_projection(vm.paint_projection.as_str().into());
    window.set_paint_lock(vm.paint_lock.as_str().into());
    let layer_entries: Vec<PaintLayerEntry> = vm
        .paint_layers
        .iter()
        .map(|layer| PaintLayerEntry {
            id: layer.id.as_str().into(),
            name: layer.name.as_str().into(),
            visible: layer.visible,
            locked: layer.locked,
            opacity: layer.opacity,
            active: layer.active,
            is_group: layer.is_group,
            kind_label: layer.kind_label.as_str().into(),
        })
        .collect();
    window.set_paint_layers(layer_entries.as_slice().into());
    window.set_active_layer_is_decal(vm.active_layer_is_decal);
    window.set_decal_surface(vm.decal_surface);
    window.set_paint_texture_side(vm.paint_texture_side);
    window.set_decal_center_u(vm.decal_center_u);
    window.set_decal_center_v(vm.decal_center_v);
    window.set_decal_scale_u(vm.decal_scale_u);
    window.set_decal_scale_v(vm.decal_scale_v);
    window.set_decal_rotation_deg(vm.decal_rotation_deg);
    window.set_loop_cut_active(vm.loop_cut_active);
    window.set_loop_cut_slide(vm.loop_cut_slide);
    window.set_loop_cut_cuts(vm.loop_cut_cuts);
    window.set_loop_cut_preview_commands(vm.loop_cut_preview_commands.as_str().into());
    window.set_loop_cut_armed(vm.loop_cut_armed);
    window.set_loop_cut_balanced(vm.loop_cut_balanced);
    window.set_pivot_id(vm.pivot_id.as_str().into());
    window.set_pivot_label(vm.pivot_label.as_str().into());
    window.set_pivot_median_label(vm.pivot_median_label.as_str().into());
    window.set_pivot_bounds_label(vm.pivot_bounds_label.as_str().into());
    window.set_pivot_cursor_label(vm.pivot_cursor_label.as_str().into());
    window.set_pivot_individual_label(vm.pivot_individual_label.as_str().into());
    window.set_edit_pivot(vm.edit_pivot);
    window.set_label_origin_to_geometry(vm.label_origin_to_geometry.as_str().into());
    window.set_label_origin_to_bottom(vm.label_origin_to_bottom.as_str().into());
    window.set_label_origin_to_cursor(vm.label_origin_to_cursor.as_str().into());
    window.set_label_origin_to_selection(vm.label_origin_to_selection.as_str().into());
    window.set_label_geometry_to_origin(vm.label_geometry_to_origin.as_str().into());
    window.set_label_edit_pivot(vm.label_edit_pivot.as_str().into());
    window.set_hint_edit_pivot(vm.hint_edit_pivot.as_str().into());
    window.set_profile_active(vm.profile_active);
    window.set_profile_point_count(vm.profile_point_count);
    window.set_profile_closed(vm.profile_closed);
    window.set_profile_preview_commands(vm.profile_preview_commands.as_str().into());
    window.set_region_hover_commands(vm.region_hover_commands.as_str().into());
    window.set_profile_selection_commands(vm.profile_selection_commands.as_str().into());
    window.set_region_shapes_commands(vm.region_shapes_commands.as_str().into());
    window.set_keymap_capture_action(vm.keymap_capture_action.as_str().into());
    {
        let b = &vm.brush_panel;
        window.set_brush_flow(b.flow);
        window.set_brush_spacing(b.spacing);
        window.set_brush_smoothing(b.smoothing);
        window.set_brush_size_jitter(b.size_jitter);
        window.set_brush_opacity_jitter(b.opacity_jitter);
        window.set_brush_scatter(b.scatter);
        window.set_brush_spray_density(b.spray_density);
        window.set_brush_angle(b.angle);
        window.set_brush_roundness(b.roundness);
        window.set_brush_tip(b.tip.as_str().into());
        window.set_brush_blend(b.blend.as_str().into());
        window.set_brush_preset_index(b.active_preset);
        window.set_clone_source_set(b.clone_source_set);
        let names: Vec<slint::SharedString> = b.presets.iter().map(|n| n.as_str().into()).collect();
        let current: Vec<slint::SharedString> =
            slint::Model::iter(&window.get_brush_presets()).collect();
        if current != names {
            crate::refresh::set_model_if_changed(window.get_brush_presets(), names, |model| {
                window.set_brush_presets(model)
            });
        }
    }
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
    window.set_profile_outline_commands(vm.profile_outline_commands.as_str().into());
    window.set_poly_pen_preview_commands(vm.poly_pen_preview_commands.as_str().into());
    window.set_label_poly_pen(vm.label_poly_pen.as_str().into());
    window.set_label_poly_pen_hint(vm.label_poly_pen_hint.as_str().into());
    window.set_poly_pen_mode(vm.poly_pen_mode.as_str().into());
    window.set_label_poly_pen_mode(vm.label_poly_pen_mode.as_str().into());
    window.set_label_poly_pen_mode_auto(vm.label_poly_pen_mode_auto.as_str().into());
    window.set_label_poly_pen_mode_points(vm.label_poly_pen_mode_points.as_str().into());
    window.set_label_poly_pen_mode_edges(vm.label_poly_pen_mode_edges.as_str().into());
    window.set_label_poly_pen_mode_polygons(vm.label_poly_pen_mode_polygons.as_str().into());
    window.set_label_poly_pen_mode_hint(vm.label_poly_pen_mode_hint.as_str().into());
    window.set_modeling_mode(vm.modeling_mode.as_str().into());
    window.set_label_workspace_draw_title(vm.label_workspace_draw_title.as_str().into());
    window
        .set_label_workspace_draw_description(vm.label_workspace_draw_description.as_str().into());
    window.set_label_workspace_poly_title(vm.label_workspace_poly_title.as_str().into());
    window
        .set_label_workspace_poly_description(vm.label_workspace_poly_description.as_str().into());
    window.set_profile_parametric(vm.profile_parametric);
    window.set_profile_ellipse(vm.profile_ellipse);
    window.set_profile_width(vm.profile_width);
    window.set_profile_height(vm.profile_height);
    window.set_profile_segments(vm.profile_segments);
    window.set_profile_depth(vm.profile_depth);
    window.set_profile_wall_thickness(vm.profile_wall_thickness);
    window.set_profile_smoothness(vm.profile_smoothness);
    window.set_profile_has_curves(vm.profile_has_curves);
    window.set_profile_workplane(vm.profile_workplane.as_str().into());
    window.set_profile_volume_mode(vm.profile_volume_mode.as_str().into());
    window.set_profile_revolve_angle(vm.profile_revolve_angle);
    window.set_paint_show_uv(vm.paint_show_uv_overlay);
    window.set_uv_show_texture(vm.uv_show_texture);
    window.set_tool_activation(vm.tool_activation.as_str().into());
    window.set_keyboard_tool_modal_active(vm.keyboard_tool_modal_active);
    window.set_is_instant_tool_mode(vm.is_instant_tool_mode);
    window.set_invert_vertical_drag(vm.invert_vertical_drag);
    window.set_colorblind_axes(vm.colorblind_axes);
    window.set_reduced_motion(vm.reduced_motion);
    window
        .global::<crate::Motion>()
        .set_reduced(vm.reduced_motion);
    window.set_double_tap_interval_ms(vm.double_tap_interval_ms);
    window.set_tool_grammar_active(vm.tool_grammar_active);
    window.set_tool_gesture_latched(vm.tool_gesture_latched);
    window.set_last_operation_active(vm.last_operation_active);
    window.set_last_operation_title(vm.last_operation_title.as_str().into());
    window.set_last_operation_value(vm.last_operation_value);
    window.set_last_operation_step(vm.last_operation_step);
    window.set_last_operation_unit(vm.last_operation_unit.as_str().into());
    window.set_label_last_operation(vm.label_last_operation.as_str().into());
    window.set_label_last_operation_hint(vm.label_last_operation_hint.as_str().into());
    window.set_drag_threshold_px(vm.drag_threshold_px);
    window.set_snap_radius_px(vm.snap_radius_px);
    window.set_label_snap_radius(vm.label_snap_radius.as_str().into());
    window.set_click_move_click(vm.click_move_click);
    window.set_workplane_prefer_ground(vm.workplane_prefer_ground);
    window.set_studio_light_follows_camera(vm.studio_light_follows_camera);
    window.set_viewport_matcap(vm.viewport_matcap);
    window.set_viewport_ambient_occlusion(vm.viewport_ambient_occlusion);
    window.set_label_viewport_matcap(vm.label_viewport_matcap.as_str().into());
    window.set_label_viewport_matcap_hint(vm.label_viewport_matcap_hint.as_str().into());
    window
        .set_label_viewport_ambient_occlusion(vm.label_viewport_ambient_occlusion.as_str().into());
    window.set_label_viewport_ambient_occlusion_hint(
        vm.label_viewport_ambient_occlusion_hint.as_str().into(),
    );
    window.set_gizmo_gpu(vm.gizmo_gpu);
    window.set_label_studio_light_follows_camera(
        vm.label_studio_light_follows_camera.as_str().into(),
    );
    window.set_label_studio_light_follows_camera_hint(
        vm.label_studio_light_follows_camera_hint.as_str().into(),
    );
    window.set_label_workplane_prefer_ground(vm.label_workplane_prefer_ground.as_str().into());
    window.set_label_workplane_prefer_ground_hint(
        vm.label_workplane_prefer_ground_hint.as_str().into(),
    );
    window.set_label_drag_threshold(vm.label_drag_threshold.as_str().into());
    window.set_label_click_move_click(vm.label_click_move_click.as_str().into());
    window.set_label_click_move_click_hint(vm.label_click_move_click_hint.as_str().into());
    window.set_multiselection_measure_tag(vm.multiselection_measure_tag);
    window.set_label_colorblind_axes(vm.label_colorblind_axes.as_str().into());
    window.set_label_reduced_motion(vm.label_reduced_motion.as_str().into());
    window.set_label_double_tap_interval(vm.label_double_tap_interval.as_str().into());
    window
        .set_label_multiselection_measure_tag(vm.label_multiselection_measure_tag.as_str().into());
    window.set_active_language(vm.active_language.as_str().into());
    crate::tr::install(window, &vm.active_language);
    window.set_ui_scale(vm.ui_scale);
    apply_ui_scale(window, vm.ui_scale);
    window.set_icon_theme(vm.icon_theme.as_str().into());
    window
        .global::<crate::PetuniaIcons>()
        .set_filled(vm.icon_theme == "filled");
    window.set_tool_modal_active(vm.tool_modal_active);
    window.set_tool_modal_id(vm.tool_modal_id.as_str().into());
    window.set_tool_options_active(vm.tool_options_active);
    window.set_tool_options_title(vm.tool_options_title.as_str().into());
    window.set_tool_options_hint(vm.tool_options_hint.as_str().into());
    window.set_tool_modal_title(vm.tool_modal_title.as_str().into());
    window.set_tool_modal_label(vm.tool_modal_label.as_str().into());
    window.set_tool_modal_value(vm.tool_modal_value);
    window.set_tool_modal_step(vm.tool_modal_step);
    window.set_tool_modal_min(vm.tool_modal_min);
    window.set_tool_modal_max(vm.tool_modal_max);
    window.set_hud_pill_visible(vm.hud_pill_visible);
    window.set_hud_pill_x(vm.hud_pill_x);
    window.set_hud_pill_y(vm.hud_pill_y);
    window.set_hud_pill_title(vm.hud_pill_title.as_str().into());
    window.set_hud_pill_value(vm.hud_pill_value.as_str().into());
    window.set_hud_pill_badge(vm.hud_pill_badge.as_str().into());
    window.set_hud_pill_hint(vm.hud_pill_hint.as_str().into());
    window.set_axis_guide_visible(vm.axis_guide_visible);
    window.set_axis_guide_commands(vm.axis_guide_commands.as_str().into());
    window.set_axis_guide_color(slint::Color::from_rgb_u8(
        vm.axis_guide_color[0],
        vm.axis_guide_color[1],
        vm.axis_guide_color[2],
    ));
    window.set_axis_guide_label(vm.axis_guide_label.as_str().into());
    window.set_axis_guide_label_x(vm.axis_guide_label_x);
    window.set_axis_guide_label_y(vm.axis_guide_label_y);
    window.set_dimension_visible(vm.dimension_visible);
    window.set_dimension_commands(vm.dimension_commands.as_str().into());
    window.set_dimension_text(vm.dimension_text.as_str().into());
    window.set_dimension_x(vm.dimension_x);
    window.set_dimension_y(vm.dimension_y);
    window.set_snap_marker_visible(vm.snap_marker_visible);
    window.set_snap_marker_x(vm.snap_marker_x);
    window.set_snap_marker_y(vm.snap_marker_y);
    window.set_snap_marker_label(vm.snap_marker_label.as_str().into());
    window.set_snap_marker_round(vm.snap_marker_round);

    window.set_measure_visible(vm.measure_visible);
    window.set_measure_commands(vm.measure_commands.as_str().into());
    window.set_measure_text(vm.measure_text.as_str().into());
    window.set_measure_x(vm.measure_x);
    window.set_measure_y(vm.measure_y);
    window.set_measure_distance(vm.measure_distance);
    window.set_measure_dx(vm.measure_dx);
    window.set_measure_dy(vm.measure_dy);
    window.set_measure_dz(vm.measure_dz);
    window.set_measure_angle_deg(vm.measure_angle_deg);
    window.set_measure_hud_text(vm.measure_hud_text.as_str().into());
    let measure_tags: Vec<MeasureTag> = vm
        .measure_tags
        .iter()
        .map(|tag| MeasureTag {
            text: tag.text.as_str().into(),
            x: tag.x,
            y: tag.y,
        })
        .collect();
    crate::refresh::set_model_if_changed(window.get_measure_tags(), measure_tags, |model| {
        window.set_measure_tags(model)
    });

    let world_axis_tags: Vec<WorldAxisTag> = vm
        .world_axis_labels
        .iter()
        .map(|tag| WorldAxisTag {
            text: tag.text.as_str().into(),
            x: tag.x,
            y: tag.y,
            tint: slint::Color::from_rgb_u8(tag.color[0], tag.color[1], tag.color[2]),
        })
        .collect();
    crate::refresh::set_model_if_changed(
        window.get_world_axis_labels(),
        world_axis_tags,
        |model| window.set_world_axis_labels(model),
    );

    window.set_micro_inspector_open(vm.micro_inspector_open);
    window.set_micro_inspector_x(vm.micro_inspector_x);
    window.set_micro_inspector_y(vm.micro_inspector_y);

    let material_slots: Vec<slint::SharedString> = vm
        .material_slots
        .iter()
        .map(|slot| slot.as_str().into())
        .collect();
    window.set_material_slots(material_slots.as_slice().into());
    window.set_active_material_slot(vm.active_material_slot);
    window.set_material_has_selection(vm.material_has_selection);
    window.set_material_id(vm.material_id.as_str().into());
    window.set_material_name(vm.material_name.as_str().into());
    window.set_material_profile(vm.material_profile.as_str().into());
    window.set_material_profile_label(vm.material_profile_label.as_str().into());
    window.set_material_base_color(slint::Color::from_argb_f32(
        1.0,
        vm.material_base_color[0],
        vm.material_base_color[1],
        vm.material_base_color[2],
    ));
    let material_palette: Vec<slint::Color> = vm
        .material_palette
        .iter()
        .map(|color| slint::Color::from_argb_f32(1.0, color[0], color[1], color[2]))
        .collect();
    window.set_material_palette(material_palette.as_slice().into());
    window.set_material_roughness(vm.material_roughness);
    window.set_material_metallic(vm.material_metallic);
    window.set_material_normal_scale(vm.material_normal_scale);
    window.set_material_emission(slint::Color::from_argb_f32(
        1.0,
        vm.material_emission[0],
        vm.material_emission[1],
        vm.material_emission[2],
    ));
    window.set_material_emission_strength(vm.material_emission_strength);
    window.set_material_alpha_mode(vm.material_alpha_mode.as_str().into());
    window.set_material_alpha_cutoff(vm.material_alpha_cutoff);
    window.set_material_has_albedo(vm.material_has_albedo);
    window.set_material_albedo_label(vm.material_albedo_label.as_str().into());

    let quick_actions: Vec<QuickActionEntry> = vm
        .quick_actions
        .iter()
        .map(|action| QuickActionEntry {
            id: action.id.as_str().into(),
            label: action.label.as_str().into(),
            enabled: action.enabled,
            pinned: action.pinned,
        })
        .collect();
    crate::refresh::set_model_if_changed(window.get_quick_actions(), quick_actions, |model| {
        window.set_quick_actions(model)
    });
    let quick_action_candidates: Vec<QuickActionEntry> = vm
        .quick_action_candidates
        .iter()
        .map(|action| QuickActionEntry {
            id: action.id.as_str().into(),
            label: action.label.as_str().into(),
            enabled: action.enabled,
            pinned: action.pinned,
        })
        .collect();
    crate::refresh::set_model_if_changed(
        window.get_quick_action_candidates(),
        quick_action_candidates,
        |model| window.set_quick_action_candidates(model),
    );
    let modifier_rows: Vec<ModifierEntry> = vm
        .modifier_rows
        .iter()
        .map(|modifier| ModifierEntry {
            id: modifier.id.as_str().into(),
            title: modifier.title.as_str().into(),
            subtitle: modifier.subtitle.as_str().into(),
            enabled: modifier.enabled,
            kind: modifier.kind.as_str().into(),
            axis: modifier.axis,
            positive_to_negative: modifier.positive_to_negative,
            can_move_up: modifier.can_move_up,
            can_move_down: modifier.can_move_down,
        })
        .collect();
    crate::refresh::set_model_if_changed(window.get_modifier_rows(), modifier_rows, |model| {
        window.set_modifier_rows(model)
    });
    let section_states: Vec<SectionState> = vm
        .section_states
        .iter()
        .map(|state| SectionState {
            id: state.id.as_str().into(),
            pin_open: state.pin_open,
            open: state.open,
        })
        .collect();
    crate::refresh::set_model_if_changed(window.get_section_states(), section_states, |model| {
        window.set_section_states(model)
    });
    for state in &vm.section_states {
        match state.id.as_str() {
            "parts" => {
                window.set_model_parts_open(state.open);
            }
            "transform" => {
                window.set_model_transform_open(state.open);
            }
            "material" => {
                window.set_model_material_open(state.open);
            }
            "object" => {
                window.set_model_object_open(state.open);
            }
            "modifiers" => {
                window.set_model_modifiers_open(state.open);
            }
            "quick_actions" => {
                window.set_quick_actions_section_open(state.open);
            }
            _ => {}
        }
    }
    window.set_paint_pixel_grid(vm.paint_pixel_grid);
    window.set_paint_canvas_zoom(vm.paint_canvas_zoom);
    window.set_paint_canvas_grid_commands(vm.paint_canvas_grid_commands.as_str().into());

    window.set_primitive_active(vm.primitive_active);
    window.set_primitive_kind(vm.primitive_kind.as_str().into());
    window.set_primitive_title(vm.primitive_title.as_str().into());
    window.set_primitive_radius(vm.primitive_radius);
    window.set_primitive_radius_b(vm.primitive_radius_b);
    window.set_primitive_height(vm.primitive_height);
    window.set_primitive_depth(vm.primitive_depth);
    window.set_primitive_width(vm.primitive_width);
    window.set_primitive_sides(vm.primitive_sides);
    window.set_primitive_rings(vm.primitive_rings);
    window.set_primitive_subdiv(vm.primitive_subdiv);
    window.set_primitive_cap_top(vm.primitive_cap_top);
    window.set_primitive_cap_bottom(vm.primitive_cap_bottom);
    window.set_primitive_fill_disc(vm.primitive_fill_disc);
    window.set_active_asset_is_parametric(vm.active_asset_is_parametric);
    window.set_label_parametric_primitive(vm.label_parametric_primitive.as_str().into());
    window.set_label_freeze_primitive(vm.label_freeze_primitive.as_str().into());
    window.set_label_freeze_primitive_hint(vm.label_freeze_primitive_hint.as_str().into());
    window.set_slice_trim(vm.slice_trim);
    window.set_bevel_clamp_overlap(vm.bevel_clamp_overlap);
    window.set_bevel_affect_vertices(vm.bevel_affect_vertices);
    window.set_protractor_visible(vm.protractor_visible);
    window.set_protractor_wedge_commands(vm.protractor_wedge_commands.as_str().into());
    window.set_protractor_ticks_commands(vm.protractor_ticks_commands.as_str().into());
    window.set_settings_visible(vm.settings_visible);
    window.set_command_search_visible(vm.command_search_visible);
    window.set_scene_drawer_visible(vm.scene_drawer_visible);
    window.set_active_keymap_id(vm.active_keymap_id.as_str().into());

    theme::apply_theme(window, &vm.current_theme);
}

pub(crate) fn persist_user_preferences<V: PetuniaViewport>(bridge: &mut SlintUiBridge<V>) {
    // Single source of truth: live UI state refreshes the cache (which owns
    // the section layouts), then the whole cache is saved. Building a fresh
    // struct here would silently wipe persisted section layouts.
    // Fonte única da verdade: o estado vivo atualiza o cache (que detém os
    // layouts), depois o cache inteiro é salvo. Construir struct nova aqui
    // apagaria silenciosamente os layouts persistidos.
    bridge.sync_preferences_from_state();
    let preferences = bridge.preferences.clone();
    let path = bridge
        .preferences_path_override
        .clone()
        .unwrap_or_else(petunia_config::UserPreferences::default_path);
    if let Err(error) = preferences.save_to_path(&path) {
        let message = bridge
            .state
            .t_id(petunia_config::text_id::UI_PREFERENCES_SAVE_FAILED);
        bridge.state.set_status(format!("{message}: {error}"));
    }
}

/// Mantém o destaque configurável legível sobre o canvas escuro oficial.
pub(crate) fn selection_color_has_contrast(rgb: [u8; 3]) -> bool {
    let luminance = |channels: [u8; 3]| {
        let linear = channels.map(|channel| {
            let value = channel as f32 / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        });
        linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722
    };
    let backdrop = luminance([16, 17, 20]);
    let foreground = luminance(rgb);
    // WCAG 3:1 para indicadores não textuais.
    (foreground + 0.05) / (backdrop + 0.05) >= 3.0
}

pub(crate) fn connect_callbacks<V: PetuniaViewport + 'static>(
    window: &PetuniaSlintShell,
    bridge: Arc<Mutex<SlintUiBridge<V>>>,
) {
    crate::animate::connect_animate_callbacks(window, Arc::clone(&bridge));
    // Cores dos eixos do gizmo GPU: as mesmas dos tokens do shell.
    if let Ok(mut bridge) = bridge.lock() {
        let tokens = window.global::<crate::DesignTokens>();
        let rgba = |c: slint::Color| {
            [
                c.red() as f32 / 255.0,
                c.green() as f32 / 255.0,
                c.blue() as f32 / 255.0,
                c.alpha() as f32 / 255.0,
            ]
        };
        bridge.gizmo_axis_colors = crate::GizmoAxisColors([
            rgba(tokens.get_axis_x()),
            rgba(tokens.get_axis_y()),
            rgba(tokens.get_axis_z()),
        ]);
    }
    // Arrasto e hover: viewport a cada evento, janela completa limitada.
    let throttle = std::rc::Rc::new(crate::refresh::RefreshThrottle::default());
    let shortcut_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_shortcut_requested(move |text, ctrl, shift, alt| {
        if let Ok(mut bridge) = shortcut_bridge.lock() {
            bridge.route_shortcut(text.as_str(), ctrl, shift, alt);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });
    let workspace_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_workspace_changed(move |workspace| {
        let mode = match workspace.as_str() {
            "DRAW" => Some(crate::ModelingMode::Draw),
            "POLY" => Some(crate::ModelingMode::Poly),
            _ => None,
        };
        if let Some(mode) = mode
            && let Ok(mut bridge) = workspace_bridge.lock()
        {
            bridge.apply(UiIntent::SetModelingMode(mode));
            let vm = bridge.view_model();
            let frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = frame {
                    window.set_viewport_image(frame);
                }
            }
            return;
        }
        let workspace = match workspace.as_str() {
            "MODEL" => Workspace::Model,
            "PAINT" => Workspace::Paint,
            "UV" => Workspace::Uv,
            #[cfg(feature = "animation-workspace")]
            "ANIMATE" => Workspace::Animate,
            _ => return,
        };
        if let Ok(mut bridge) = workspace_bridge.lock() {
            bridge.apply(UiIntent::SetWorkspace(workspace));
            let vm = bridge.view_model();
            let frame = bridge.render_viewport();
            let canvas_img = if workspace == Workspace::Paint || workspace == Workspace::Uv {
                bridge.render_paint_canvas()
            } else {
                None
            };
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(canvas_img) = canvas_img {
                    window.set_paint_canvas_image(canvas_img);
                }
                if let Some(frame) = frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let save_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_save_requested(move || {
        let existing = save_bridge
            .lock()
            .ok()
            .and_then(|bridge| bridge.state.project.project_path.clone());
        if let Some(path) = existing {
            if let Ok(mut bridge) = save_bridge.lock() {
                bridge.apply(UiIntent::SaveProjectTo(PathBuf::from(path)));
                let vm = bridge.view_model();
                if let Some(window) = window_weak.upgrade() {
                    window.set_saved(vm.saved);
                    window.set_status_severity(vm.status_severity);
                    window.set_status_message(vm.status_message.as_str().into());
                }
            }
            return;
        }
        let bridge = Arc::clone(&save_bridge);
        let window_weak = window_weak.clone();
        let _ = slint::spawn_local(async move {
            let service = files::FileDialogService::new();
            let Some(path) = service.save_project().await else {
                return;
            };
            if let Ok(mut bridge) = bridge.lock() {
                bridge.apply(UiIntent::SaveProjectTo(path));
                let vm = bridge.view_model();
                if let Some(window) = window_weak.upgrade() {
                    window.set_saved(vm.saved);
                    window.set_status_severity(vm.status_severity);
                    window.set_status_message(vm.status_message.as_str().into());
                }
            }
        });
    });

    let open_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_open_project_requested(move || {
        let bridge = Arc::clone(&open_bridge);
        let window_weak = window_weak.clone();
        let _ = slint::spawn_local(async move {
            let service = files::FileDialogService::new();
            let Some(path) = service.open_project().await else {
                return;
            };
            if let Ok(mut bridge) = bridge.lock() {
                bridge.apply(UiIntent::OpenProjectFrom(path));
                let vm = bridge.view_model();
                let new_frame = bridge.render_viewport();
                if let Some(window) = window_weak.upgrade() {
                    window.set_saved(vm.saved);
                    if let Some(frame) = new_frame {
                        window.set_viewport_image(frame);
                    }
                }
            }
        });
    });

    let file_command_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_command_file_action(move |id| {
        let bridge = Arc::clone(&file_command_bridge);
        let window_weak = window_weak.clone();
        let id = id.to_string();
        let _ = slint::spawn_local(async move {
            let service = files::FileDialogService::new();
            let path = match id.as_str() {
                "file.open" => service.open_project().await,
                "file.save" | "file.save_as" => service.save_project().await,
                "file.import_obj" => service.import_model().await,
                "file.export_obj" => service.export_obj().await,
                "file.export_glb" => service.export_glb().await,
                "palette.import" => service.import_palette().await,
                "palette.export" => service.export_palette().await,
                "paint.import_decal" => service.open_decal_image().await,
                "draw.import_svg" => service.open_svg().await,
                _ => None,
            };
            let Some(path) = path else {
                return;
            };
            if let Ok(mut bridge) = bridge.lock() {
                let intent = match id.as_str() {
                    "file.open" => UiIntent::OpenProjectFrom(path),
                    "file.import_obj" => UiIntent::ImportModelFrom(path),
                    "file.export_obj" => UiIntent::ExportActiveObjTo(path),
                    "file.export_glb" => UiIntent::ExportSceneGlbTo(path),
                    "palette.import" => UiIntent::ImportPalette(path),
                    "palette.export" => UiIntent::ExportPalette(path),
                    "paint.import_decal" => UiIntent::ImportDecalFrom(path),
                    "draw.import_svg" => UiIntent::ImportSvgProfiles(path),
                    _ => UiIntent::SaveProjectTo(path),
                };
                let needs_render = matches!(id.as_str(), "file.open" | "file.import_obj");
                bridge.apply(intent);
                bridge.command_search_visible = false;
                bridge.overlays.remove(OverlayId::CommandPalette);
                let vm = bridge.view_model();
                let new_frame = if needs_render {
                    bridge.render_viewport()
                } else {
                    None
                };
                if let Some(window) = window_weak.upgrade() {
                    window.set_command_search_visible(false);
                    sync_window_properties(&window, &vm);
                    if let Some(frame) = new_frame {
                        window.set_viewport_image(frame);
                    }
                }
            }
        });
    });

    let search_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_search_requested(move || {
        if let Ok(mut bridge) = search_bridge.lock() {
            bridge.apply(UiIntent::OpenCommandSearch);
            let items: Vec<CommandItem> = bridge
                .search_commands("")
                .into_iter()
                .map(|cmd| CommandItem {
                    id: cmd.id.into(),
                    label: cmd.label.into(),
                    shortcut: cmd.shortcut.unwrap_or_default().into(),
                    available: cmd.is_available,
                    disabled_reason: cmd.disabled_reason.unwrap_or_default().into(),
                })
                .collect();
            if let Some(window) = window_weak.upgrade() {
                window.set_command_search_visible(true);
                let model = std::rc::Rc::new(slint::VecModel::from(items));
                window.set_command_results(model.into());
            }
        }
    });

    let settings_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_settings_requested(move || {
        if let Ok(mut bridge) = settings_bridge.lock() {
            bridge.apply(UiIntent::OpenSettings);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                window.set_settings_visible(true);
                sync_window_properties(&window, &vm);
            }
        }
    });

    let settings_close_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_settings_closed(move || {
        if let Ok(mut bridge) = settings_close_bridge.lock() {
            persist_user_preferences(&mut bridge);
            bridge.apply(UiIntent::CloseSettings);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                window.set_settings_visible(false);
                sync_window_properties(&window, &vm);
            }
        }
    });

    let keymap_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_keymap_changed(move |keymap_id| {
        if let Ok(mut bridge) = keymap_bridge.lock() {
            if bridge.set_keymap_profile(keymap_id.as_str()) {
                persist_user_preferences(&mut bridge);
            }
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    // Editor de atalhos: criar/apagar perfil, capturar, restaurar e limpar.
    macro_rules! keymap_callback {
        ($register:ident, $persist:expr, |$bridge:ident $(, $arg:ident)*| $body:expr) => {{
            let handle = Arc::clone(&bridge);
            let window_weak = window.as_weak();
            window.$register(move |$($arg),*| {
                if let Ok(mut $bridge) = handle.lock() {
                    let changed: bool = $body;
                    if changed && $persist {
                        persist_user_preferences(&mut $bridge);
                    }
                    let vm = $bridge.view_model();
                    if let Some(window) = window_weak.upgrade() {
                        sync_window_properties(&window, &vm);
                    }
                }
            });
        }};
    }
    keymap_callback!(on_brush_param_changed, false, |b, name, value| {
        b.set_brush_param(name.as_str(), value)
    });
    keymap_callback!(on_brush_tip_changed, false, |b, tip| b
        .set_brush_tip(tip.as_str()));
    keymap_callback!(on_brush_blend_changed, false, |b, blend| b
        .set_brush_blend(blend.as_str()));
    keymap_callback!(on_brush_preset_applied, false, |b, index| b
        .apply_brush_preset(index.max(0) as usize));
    keymap_callback!(on_brush_preset_saved, false, |b| {
        let count = b.all_brush_presets().len();
        b.save_brush_preset(&format!("Custom {}", count + 1))
    });
    keymap_callback!(on_brush_preset_deleted, false, |b, index| b
        .delete_brush_preset(index.max(0) as usize));
    keymap_callback!(on_keymap_profile_create, true, |b| b
        .create_keymap_profile(""));
    keymap_callback!(on_keymap_profile_delete, true, |b, id| b
        .delete_keymap_profile(id.as_str()));
    keymap_callback!(on_keymap_capture_requested, true, |b, action| b
        .begin_keymap_capture(action.as_str()));
    keymap_callback!(on_keymap_capture_cancelled, true, |b| b
        .cancel_keymap_capture());
    keymap_callback!(on_keymap_capture_key, true, |b, key, ctrl, shift, alt| b
        .capture_keymap_key(key.as_str(), ctrl, shift, alt));
    keymap_callback!(on_keymap_binding_reset, true, |b, action| b
        .reset_keymap_binding(action.as_str()));
    keymap_callback!(on_keymap_binding_clear, true, |b, action| b
        .clear_keymap_binding(action.as_str()));

    let scene_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_scene_requested(move || {
        if let Ok(mut bridge) = scene_bridge.lock()
            && let Some(window) = window_weak.upgrade()
        {
            if window.get_compact_shell() {
                bridge.apply(UiIntent::ToggleSceneDrawer);
            } else {
                if bridge.scene_drawer_visible {
                    bridge.apply(UiIntent::ToggleSceneDrawer);
                }
                window.set_model_parts_open(true);
            }
            let vm = bridge.view_model();
            window.set_scene_drawer_visible(bridge.scene_drawer_visible);
            sync_window_properties(&window, &vm);
        }
    });

    let scene_pin_bridge = Arc::clone(&bridge);
    window.on_scene_pin_requested(move |pinned| {
        if let Ok(mut bridge) = scene_pin_bridge.lock() {
            bridge.overlays.set_pinned(OverlayId::SceneDrawer, pinned);
        }
    });

    let query_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_command_query_changed(move |query| {
        let items: Vec<CommandItem> = if let Ok(bridge) = query_bridge.lock() {
            bridge
                .search_commands(query.as_str())
                .into_iter()
                .map(|cmd| CommandItem {
                    id: cmd.id.into(),
                    label: cmd.label.into(),
                    shortcut: cmd.shortcut.unwrap_or_default().into(),
                    available: cmd.is_available,
                    disabled_reason: cmd.disabled_reason.unwrap_or_default().into(),
                })
                .collect()
        } else {
            Vec::new()
        };
        if let Some(window) = window_weak.upgrade() {
            let model = std::rc::Rc::new(slint::VecModel::from(items));
            window.set_command_results(model.into());
        }
    });

    let exec_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_command_executed(move |id_str| {
        if let Ok(mut bridge) = exec_bridge.lock() {
            if let Some(command) = CommandId::from_id_str(id_str.as_str()) {
                if command == CommandId::ToggleSceneDrawer
                    && let Some(window) = window_weak.upgrade()
                    && !window.get_compact_shell()
                {
                    if bridge.scene_drawer_visible {
                        bridge.apply(UiIntent::ToggleSceneDrawer);
                    }
                    window.set_model_parts_open(true);
                } else {
                    bridge.execute_command(command);
                }
            } else if let Err(error) = bridge.execute_core_command(id_str.as_str()) {
                bridge.state.set_status(error.to_string());
            }
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                window.set_command_search_visible(bridge.command_search_visible);
                window.set_settings_visible(bridge.settings_visible);
                window.set_scene_drawer_visible(bridge.scene_drawer_visible);
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let esc_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_escape_requested(move || {
        if let Ok(mut bridge) = esc_bridge.lock() {
            bridge.handle_escape();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let click_away_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_click_away_requested(move || {
        if let Ok(mut bridge) = click_away_bridge.lock() {
            bridge.handle_click_away();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let transform_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_transform_scrubbed(move |kind_str, axis, delta, fine| {
        let kind = match kind_str.as_str() {
            "pos" => TransformKind::Position,
            "rot" => TransformKind::Rotation,
            "scale" => TransformKind::Scale,
            _ => return,
        };
        if let Ok(mut bridge) = transform_bridge.lock() {
            let val = bridge.scrub_transform(kind, axis as usize, delta, fine);
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
                match (kind, axis) {
                    (TransformKind::Position, 0) => window.set_pos_x(val),
                    (TransformKind::Position, 1) => window.set_pos_y(val),
                    (TransformKind::Position, 2) => window.set_pos_z(val),
                    (TransformKind::Rotation, 0) => window.set_rot_x(val),
                    (TransformKind::Rotation, 1) => window.set_rot_y(val),
                    (TransformKind::Rotation, 2) => window.set_rot_z(val),
                    (TransformKind::Scale, 0) => window.set_scale_x(val),
                    (TransformKind::Scale, 1) => window.set_scale_y(val),
                    (TransformKind::Scale, 2) => window.set_scale_z(val),
                    _ => {}
                }
            }
        }
    });

    let box_bridge = Arc::clone(&bridge);
    let box_window = window.as_weak();
    window.on_viewport_box_select(move |x0, y0, x1, y1, add, subtract| {
        if let Ok(mut bridge) = box_bridge.lock() {
            bridge.state.select_viewport_box(
                [x0 * 2.0 - 1.0, 1.0 - y0 * 2.0],
                [x1 * 2.0 - 1.0, 1.0 - y1 * 2.0],
                add,
                subtract,
            );
            // Box mudo é box confuso: dizer o que entrou na seleção fecha o
            // ciclo de feedback do gesto (Blender mostra a contagem na barra).
            let summary = match bridge.state.selection_domain() {
                SelectionDomain::Object => {
                    let total = bridge.state.session.selection.assets.len();
                    if total == 0 {
                        "Box select: nothing in the region".to_string()
                    } else {
                        format!("Box select: {total} object(s)")
                    }
                }
                SelectionDomain::Vertex | SelectionDomain::Edge | SelectionDomain::Face => {
                    match bridge.state.project.active_mesh() {
                        Some(mesh) => {
                            let points = mesh.verts.iter().filter(|v| v.selected).count();
                            let faces = mesh.faces.iter().filter(|f| f.selected).count();
                            let edges = mesh.selected_edges.len();
                            match bridge.state.selection_domain() {
                                SelectionDomain::Vertex => {
                                    if points == 0 {
                                        "Box select: nothing in the region".to_string()
                                    } else {
                                        format!("Box select: {points} point(s)")
                                    }
                                }
                                SelectionDomain::Edge => {
                                    if edges == 0 {
                                        "Box select: nothing in the region".to_string()
                                    } else {
                                        format!("Box select: {edges} edge(s)")
                                    }
                                }
                                _ => {
                                    if faces == 0 {
                                        "Box select: nothing in the region".to_string()
                                    } else {
                                        format!("Box select: {faces} face(s)")
                                    }
                                }
                            }
                        }
                        None => "Box select: no active object".to_string(),
                    }
                }
            };
            bridge.state.set_status(summary);
            if let Some(window) = box_window.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let lasso_bridge = Arc::clone(&bridge);
    let lasso_window = window.as_weak();
    window.on_viewport_lasso_select(move |path, add, subtract| {
        let Some(polygon) = parse_lasso_path(path.as_str()) else {
            return;
        };
        if let Ok(mut bridge) = lasso_bridge.lock() {
            bridge.state.select_viewport_lasso(&polygon, add, subtract);
            let message = bridge
                .state
                .t_id(petunia_config::text_id::STATUS_LASSO_SELECTION_UPDATED);
            bridge.state.set_status(message);
            if let Some(window) = lasso_window.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let place_cursor_bridge = Arc::clone(&bridge);
    let place_cursor_window = window.as_weak();
    window.on_viewport_place_cursor(move |norm_x, norm_y| {
        if let Ok(mut bridge) = place_cursor_bridge.lock()
            && bridge.place_cursor_3d(norm_x, norm_y)
        {
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = place_cursor_window.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let cursor_coord_bridge = Arc::clone(&bridge);
    let cursor_coord_window = window.as_weak();
    window.on_cursor_coord_committed(move |axis_idx, val| {
        if let Ok(mut bridge) = cursor_coord_bridge.lock()
            && bridge.set_cursor_3d_coord(axis_idx as usize, val)
        {
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = cursor_coord_window.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let reset_cursor_bridge = Arc::clone(&bridge);
    let reset_cursor_window = window.as_weak();
    window.on_reset_cursor_requested(move || {
        if let Ok(mut bridge) = reset_cursor_bridge.lock()
            && bridge.reset_cursor_3d()
        {
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = reset_cursor_window.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let frame_cursor_bridge = Arc::clone(&bridge);
    let frame_cursor_window = window.as_weak();
    window.on_frame_cursor_requested(move || {
        if let Ok(mut bridge) = frame_cursor_bridge.lock()
            && bridge.frame_cursor()
        {
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = frame_cursor_window.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let transform_begin_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_transform_scrub_started(move |kind_str| {
        let kind = match kind_str.as_str() {
            "pos" => TransformKind::Position,
            "rot" => TransformKind::Rotation,
            "scale" => TransformKind::Scale,
            _ => return,
        };
        if let Ok(mut bridge) = transform_begin_bridge.lock()
            && let Err(error) = bridge.begin_transform(kind)
        {
            bridge.state.set_status(error.to_string());
            if let Some(window) = window_weak.upgrade() {
                window.set_status_severity(3);
                window.set_status_message(error.to_string().into());
            }
        }
    });

    let transform_finish_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_transform_scrub_finished(move || {
        if let Ok(mut bridge) = transform_finish_bridge.lock() {
            bridge.commit_transform();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
            let frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade()
                && let Some(frame) = frame
            {
                window.set_viewport_image(frame);
            }
        }
    });

    let transform_text_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_transform_text_committed(move |kind_str, axis, text| {
        let kind = match kind_str.as_str() {
            "pos" => TransformKind::Position,
            "rot" => TransformKind::Rotation,
            "scale" => TransformKind::Scale,
            _ => return false,
        };
        let Ok(mut bridge) = transform_text_bridge.lock() else {
            return false;
        };
        let result = bridge.commit_transform_text(kind, axis as usize, text.as_str());
        if let Err(error) = result {
            let message = crate::tr::fill(
                &bridge
                    .state
                    .t_id(petunia_config::text_id::STATUS_INVALID_NUMERIC_VALUE),
                &[("error", format!("{error:?}"))],
            );
            bridge.state.set_status(message);
        }
        if let Some(window) = window_weak.upgrade() {
            sync_window_properties(&window, &bridge.view_model());
            if let Some(frame) = bridge.render_viewport() {
                window.set_viewport_image(frame);
            }
        }
        result.is_ok()
    });

    let transform_cancel_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_transform_scrub_cancelled(move || {
        if let Ok(mut bridge) = transform_cancel_bridge.lock() {
            bridge.cancel_transform();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let orbit_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_orbit(move |dx, dy| {
        if let Ok(mut bridge) = orbit_bridge.lock() {
            bridge.orbit_viewport(dx, dy);
            let new_frame = bridge.render_viewport();
            if let (Some(window), Some(frame)) = (window_weak.upgrade(), new_frame) {
                sync_viewport_overlays(&window, &bridge);
                window.set_viewport_image(frame);
            }
        }
    });

    let pan_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_pan(move |dx, dy| {
        if let Ok(mut bridge) = pan_bridge.lock() {
            bridge.apply(UiIntent::ViewportGesture(ViewportGesture::Pan { dx, dy }));
            let new_frame = bridge.render_viewport();
            if let (Some(window), Some(frame)) = (window_weak.upgrade(), new_frame) {
                sync_viewport_overlays(&window, &bridge);
                window.set_viewport_image(frame);
            }
        }
    });

    let zoom_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_zoom(move |delta| {
        if let Ok(mut bridge) = zoom_bridge.lock() {
            bridge.apply(UiIntent::ViewportGesture(ViewportGesture::Zoom { delta }));
            let new_frame = bridge.render_viewport();
            if let (Some(window), Some(frame)) = (window_weak.upgrade(), new_frame) {
                sync_viewport_overlays(&window, &bridge);
                window.set_viewport_image(frame);
            }
        }
    });

    let resize_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_resized(move |width, height, physical_width, _physical_height| {
        // A razão vem do próprio Slint (px físicos / px lógicos), então também
        // acompanha troca de monitor e da preferência de UI scale.
        let ratio = if width > 0.5 {
            physical_width / width
        } else {
            1.0
        };
        let width = width.round().max(1.0) as u32;
        let height = height.round().max(1.0) as u32;
        if let Ok(mut bridge) = resize_bridge.lock() {
            bridge.resize_viewport_scaled(width, height, ratio);
            let new_frame = bridge.render_viewport();
            if let (Some(window), Some(frame)) = (window_weak.upgrade(), new_frame) {
                sync_viewport_overlays(&window, &bridge);
                window.set_viewport_image(frame);
            }
        }
    });

    let viewport_select_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_select(move |x, y, extend, loop_select| {
        if let Ok(mut bridge) = viewport_select_bridge.lock() {
            bridge.select_viewport_ext(x, y, extend, loop_select);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let viewport_profile_drag_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_profile_drag_handle(move |x, y, break_tangent| {
        if let Ok(mut bridge) = viewport_profile_drag_bridge.lock()
            && bridge.profile_update_drag_handle(x, y, break_tangent)
        {
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let profile_down_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_profile_pointer_down(move |x, y, alt, ctrl| {
        if let Ok(mut bridge) = profile_down_bridge.lock() {
            let hit = bridge.profile_pointer_down_ex(x, y, alt, ctrl);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
            hit
        } else {
            false
        }
    });

    let profile_move_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    let profile_move_throttle = std::rc::Rc::clone(&throttle);
    window.on_viewport_profile_pointer_move(move |x, y, alt| {
        if let Ok(mut bridge) = profile_move_bridge.lock()
            && bridge.profile_pointer_move(x, y, alt)
            && let Some(window) = window_weak.upgrade()
        {
            crate::refresh::refresh_interactive(
                &window,
                &profile_move_bridge,
                &mut *bridge,
                &profile_move_throttle,
            );
        }
    });

    let profile_up_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_profile_pointer_up(move || {
        if let Ok(mut bridge) = profile_up_bridge.lock() {
            bridge.profile_pointer_up();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let transform_begin_bridge = Arc::clone(&bridge);
    window.on_viewport_transform_begin(move |kind_str, x, y| {
        let kind = match kind_str.as_str() {
            "pos" => TransformKind::Position,
            "rot" => TransformKind::Rotation,
            "scale" => TransformKind::Scale,
            // O plano de corte reusa o mesmo canal de arrasto, mas com a própria
            // sessão: nada de transformar geometria.
            "slice" => {
                if let Ok(mut bridge) = transform_begin_bridge.lock() {
                    bridge.begin_slice(x, y);
                }
                return;
            }
            _ => return,
        };
        if let Ok(mut bridge) = transform_begin_bridge.lock() {
            bridge.begin_viewport_transform(kind, x, y);
        }
    });

    let transform_drag_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    let transform_drag_throttle = std::rc::Rc::clone(&throttle);
    window.on_viewport_transform_update(move |x, y, fine, snap| {
        if let Ok(mut bridge) = transform_drag_bridge.lock() {
            bridge.pointer_position = [x, y];
            if !bridge.update_viewport_slice_modified(x, y, snap) {
                bridge.update_viewport_transform_modified(x, y, fine, snap);
            }
            if let Some(window) = window_weak.upgrade() {
                crate::refresh::refresh_interactive(
                    &window,
                    &transform_drag_bridge,
                    &mut *bridge,
                    &transform_drag_throttle,
                );
            }
        }
    });

    let transform_end_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_transform_end(move || {
        if let Ok(mut bridge) = transform_end_bridge.lock() {
            bridge.end_viewport_transform();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let paint_begin_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_paint_begin(move |x, y, is_shift, is_ctrl| {
        if let Ok(mut bridge) = paint_begin_bridge.lock() {
            bridge.begin_paint_stroke_with_modifiers(x, y, is_shift, is_ctrl);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
                bridge.publish_canvas_image(&window);
            }
        }
    });

    let paint_update_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    let paint_update_throttle = std::rc::Rc::clone(&throttle);
    window.on_viewport_paint_update(move |x, y, is_shift, is_ctrl| {
        if let Ok(mut bridge) = paint_update_bridge.lock() {
            bridge.paint_stroke_to_with_modifiers(x, y, is_shift, is_ctrl);
            if let Some(window) = window_weak.upgrade() {
                crate::refresh::refresh_interactive(
                    &window,
                    &paint_update_bridge,
                    &mut *bridge,
                    &paint_update_throttle,
                );
                bridge.publish_canvas_image(&window);
            }
        }
    });

    let paint_end_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_paint_end(move |x, y| {
        if let Ok(mut bridge) = paint_end_bridge.lock() {
            bridge.end_paint_stroke_at(x, y);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let add_menu_bridge = Arc::clone(&bridge);
    window.on_add_menu_changed(move |open| {
        if let Ok(mut bridge) = add_menu_bridge.lock() {
            bridge.add_menu_open = open;
        }
    });

    let tool_hover_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    let tool_hover_throttle = std::rc::Rc::clone(&throttle);
    window.on_tool_modal_hovered(move |delta, x, y| {
        if let Ok(mut bridge) = tool_hover_bridge.lock()
            && bridge.is_instant_tool_mode()
            && bridge.tool_modal.is_some()
        {
            bridge.pointer_position = [x, y];
            bridge.scrub_tool_modal(delta, false);
            if let Some(window) = window_weak.upgrade() {
                crate::refresh::refresh_interactive(
                    &window,
                    &tool_hover_bridge,
                    &mut *bridge,
                    &tool_hover_throttle,
                );
            }
        }
    });

    let activation_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_tool_activation_set(move |id| {
        if let Ok(mut bridge) = activation_bridge.lock() {
            bridge.set_tool_activation(id.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let vertical_drag_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_invert_vertical_drag_set(move |invert| {
        if let Ok(mut bridge) = vertical_drag_bridge.lock() {
            if bridge.set_invert_vertical_drag(invert) {
                persist_user_preferences(&mut bridge);
            }
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let colorblind_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_colorblind_axes_set(move |enabled| {
        if let Ok(mut bridge) = colorblind_bridge.lock() {
            if bridge.set_colorblind_axes(enabled) {
                persist_user_preferences(&mut bridge);
            }
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                sync_viewport_overlays(&window, &bridge);
            }
        }
    });

    let reduced_motion_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_reduced_motion_set(move |enabled| {
        if let Ok(mut bridge) = reduced_motion_bridge.lock() {
            if bridge.set_reduced_motion(enabled) {
                persist_user_preferences(&mut bridge);
            }
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let double_tap_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_double_tap_interval_set(move |interval| {
        if let Ok(mut bridge) = double_tap_bridge.lock() {
            bridge.set_double_tap_interval_ms(interval.max(0) as u64);
            if let Some(window) = window_weak.upgrade() {
                window.set_double_tap_interval_ms(interval);
            }
        }
    });

    let double_tap_rel_bridge = Arc::clone(&bridge);
    window.on_double_tap_interval_released(move |_interval| {
        if let Ok(mut bridge) = double_tap_rel_bridge.lock() {
            persist_user_preferences(&mut bridge);
        }
    });

    let measure_tag_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_multiselection_measure_tag_set(move |enabled| {
        if let Ok(mut bridge) = measure_tag_bridge.lock() {
            if bridge.set_multiselection_measure_tag(enabled) {
                persist_user_preferences(&mut bridge);
            }
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                sync_viewport_overlays(&window, &bridge);
            }
        }
    });

    let selection_color_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_selection_color_set(move |hex| {
        if let Ok(mut bridge) = selection_color_bridge.lock() {
            if bridge.set_selection_color_hex(hex.as_str()) {
                persist_user_preferences(&mut bridge);
            }
            let vm = bridge.view_model();
            let frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let selection_thickness_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_selection_thickness_set(move |thickness| {
        if let Ok(mut bridge) = selection_thickness_bridge.lock() {
            bridge.set_selection_thickness(thickness);
            let frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                window.set_selection_thickness(thickness);
                if let Some(frame) = frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let selection_thickness_rel_bridge = Arc::clone(&bridge);
    window.on_selection_thickness_released(move |_thickness| {
        if let Ok(mut bridge) = selection_thickness_rel_bridge.lock() {
            persist_user_preferences(&mut bridge);
        }
    });

    let language_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_language_changed(move |lang| {
        if let Ok(mut bridge) = language_bridge.lock() {
            if bridge.set_language(lang.as_str()) {
                persist_user_preferences(&mut bridge);
            }
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let ui_scale_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_ui_scale_set(move |scale| {
        if let Ok(mut bridge) = ui_scale_bridge.lock() {
            if bridge.set_ui_scale(scale) {
                persist_user_preferences(&mut bridge);
            }
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let icon_theme_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_icon_theme_set(move |theme| {
        if let Ok(mut bridge) = icon_theme_bridge.lock() {
            if bridge.set_icon_theme(theme.as_str()) {
                persist_user_preferences(&mut bridge);
            }
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let parts_query_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_parts_query_changed(move |query| {
        if let Ok(mut bridge) = parts_query_bridge.lock() {
            bridge.set_parts_query(query.as_str());
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let parts_filter_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_parts_selected_only_changed(move |enabled| {
        if let Ok(mut bridge) = parts_filter_bridge.lock() {
            bridge.set_parts_selected_only(enabled);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let parts_sort_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_parts_sort_changed(move |enabled| {
        if let Ok(mut bridge) = parts_sort_bridge.lock() {
            bridge.set_parts_sort_by_name(enabled);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

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

    let asset_query_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_asset_query_changed(move |query| {
        if let Ok(mut bridge) = asset_query_bridge.lock() {
            bridge.set_asset_query(query.as_str());
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let asset_sort_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_asset_sort_changed(move |sort_by_name| {
        if let Ok(mut bridge) = asset_sort_bridge.lock() {
            bridge.set_asset_sort_by_name(sort_by_name);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let asset_size_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_asset_thumbnail_size_changed(move |size| {
        if let Ok(mut bridge) = asset_size_bridge.lock() {
            bridge.set_asset_thumbnail_size(size);
            if let Some(window) = window_weak.upgrade() {
                window.set_asset_thumbnail_size(size);
            }
        }
    });

    let tool_scrub_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_tool_modal_scrubbed(move |delta, fine, x, y| {
        if let Ok(mut bridge) = tool_scrub_bridge.lock() {
            bridge.pointer_position = [x, y];
            bridge.scrub_tool_modal(delta, fine);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let tool_text_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_tool_modal_text_committed(move |text| {
        let Ok(mut bridge) = tool_text_bridge.lock() else {
            return false;
        };
        let base = bridge.tool_modal_value;
        let accepted = match numeric::parse_numeric_with_base(text.as_str(), base) {
            Ok(value) => bridge.set_tool_modal_value(value),
            Err(error) => {
                let message = crate::tr::fill(
                    &bridge
                        .state
                        .t_id(petunia_config::text_id::STATUS_INVALID_VALUE),
                    &[("error", format!("{error:?}"))],
                );
                bridge.state.set_status(message);
                false
            }
        };
        if let Some(window) = window_weak.upgrade() {
            sync_window_properties(&window, &bridge.view_model());
            if let Some(frame) = bridge.render_viewport() {
                window.set_viewport_image(frame);
            }
        }
        accepted
    });

    let tool_apply_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_tool_modal_apply(move || {
        if let Ok(mut bridge) = tool_apply_bridge.lock() {
            bridge.commit_tool_modal();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let tool_cancel_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_tool_modal_cancel(move || {
        if let Ok(mut bridge) = tool_cancel_bridge.lock() {
            bridge.cancel_tool_modal();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let extrude_mode_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_tool_modal_toggle_extrude_mode(move || {
        if let Ok(mut bridge) = extrude_mode_bridge.lock()
            && bridge.switch_extrude_mode()
        {
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
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

    let rename_edit_bridge = Arc::clone(&bridge);
    window.on_rename_edited(move |text| {
        if let Ok(mut bridge) = rename_edit_bridge.lock()
            && let Some(draft) = bridge.rename_draft.as_mut()
        {
            *draft = text.to_string();
        }
    });

    let rename_commit_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_rename_committed(move |text| {
        if let Ok(mut bridge) = rename_commit_bridge.lock() {
            bridge.commit_rename(text.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let rename_cancel_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_rename_cancelled(move || {
        if let Ok(mut bridge) = rename_cancel_bridge.lock() {
            bridge.cancel_rename();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let context_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_scene_context_requested(move |id, x, y| {
        if let Ok(mut bridge) = context_bridge.lock() {
            bridge.open_context_menu(id.as_str(), x, y);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let viewport_context_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_context_requested(move |x, y| {
        if let Ok(mut bridge) = viewport_context_bridge.lock() {
            bridge.viewport_context_triage(x, y);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let context_action_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_context_menu_action(move |action| {
        if let Ok(mut bridge) = context_action_bridge.lock() {
            bridge.context_menu_action(action.as_str());
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let menu_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_menu_toggled(move |id| {
        if let Ok(mut bridge) = menu_bridge.lock() {
            bridge.toggle_menu(id.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let menu_open_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_menu_opened(move |id| {
        if let Ok(mut bridge) = menu_open_bridge.lock() {
            if !bridge.open_menu(id.as_str()) {
                return;
            }
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let menu_item_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_menu_item_invoked(move |id| {
        let id = id.to_string();
        // Diálogos de arquivo continuam no caminho assíncrono já testado.
        let opens_dialog = matches!(
            id.as_str(),
            "file.open" | "file.save" | "file.save_as" | "file.import_obj"
        );
        if let Ok(mut bridge) = menu_item_bridge.lock() {
            bridge.close_menu();
            if opens_dialog {
                let vm = bridge.view_model();
                if let Some(window) = window_weak.upgrade() {
                    sync_window_properties(&window, &vm);
                    window.invoke_command_file_action(id.as_str().into());
                }
                return;
            }
            bridge.menu_item_invoked(id.as_str());
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let keep_parts_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_boolean_keep_parts_set(move |keep| {
        if let Ok(mut bridge) = keep_parts_bridge.lock() {
            bridge.set_boolean_keep_parts(keep);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let cleanup_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_boolean_cleanup_set(move |cleanup| {
        if let Ok(mut bridge) = cleanup_bridge.lock() {
            bridge.set_boolean_cleanup(cleanup);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let join_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_join_requested(move || {
        if let Ok(mut bridge) = join_bridge.lock() {
            bridge.join_operand();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let effect_add_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_effect_layer_added(move |kind| {
        if let Ok(mut bridge) = effect_add_bridge.lock() {
            if !bridge.add_paint_effect_layer(kind.as_str()) {
                let message = crate::tr::fill(
                    &bridge
                        .state
                        .t_id(petunia_config::text_id::STATUS_UNKNOWN_EFFECT_LAYER),
                    &[("kind", format!("{kind}"))],
                );
                bridge.state.set_status(message);
            }
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
                bridge.publish_canvas_image(&window);
            }
        }
    });

    let effect_param_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_effect_param_set(move |key, value| {
        if let Ok(mut bridge) = effect_param_bridge.lock() {
            bridge.set_paint_effect_param(key.as_str(), value);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
                bridge.publish_canvas_image(&window);
            }
        }
    });

    // Gramática única (constituição 11): o botão principal das ferramentas
    // persistentes chega aqui como pressionar (0), mover (1), soltar (2) ou
    // cancelar (3). Movimentos sem efeito não redesenham nada.
    let tool_pointer_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    let tool_pointer_throttle = std::rc::Rc::clone(&throttle);
    window.on_tool_pointer(move |phase, x, y, shift, ctrl, alt| {
        if let Ok(mut bridge) = tool_pointer_bridge.lock() {
            if !crate::perf::measure("tool_pointer", || {
                bridge.tool_pointer_ex(phase, x, y, shift, ctrl, alt)
            }) {
                return;
            }
            if let Some(window) = window_weak.upgrade() {
                // Pressionar e soltar mudam o documento/a seleção: janela inteira.
                if phase == 1 {
                    crate::refresh::refresh_interactive(
                        &window,
                        &tool_pointer_bridge,
                        &mut *bridge,
                        &tool_pointer_throttle,
                    );
                } else {
                    crate::refresh::refresh_full(&window, &mut *bridge, &tool_pointer_throttle);
                }
            }
        }
    });

    let last_operation_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_last_operation_committed(move |text| {
        let Ok(mut bridge) = last_operation_bridge.lock() else {
            return false;
        };
        let accepted = bridge.commit_last_operation_text(text.as_str());
        let vm = bridge.view_model();
        let new_frame = bridge.render_viewport();
        if let Some(window) = window_weak.upgrade() {
            sync_window_properties(&window, &vm);
            if let Some(frame) = new_frame {
                window.set_viewport_image(frame);
            }
        }
        accepted
    });

    let ctrl_scroll_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_ctrl_scroll(move |delta| {
        if let Ok(mut bridge) = ctrl_scroll_bridge.lock()
            && bridge.viewport_ctrl_scroll(delta)
        {
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let drag_threshold_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_drag_threshold_set(move |pixels| {
        if let Ok(mut bridge) = drag_threshold_bridge.lock()
            && bridge.set_drag_threshold_px(pixels)
        {
            persist_user_preferences(&mut bridge);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let snap_radius_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_snap_radius_set(move |pixels| {
        if let Ok(mut bridge) = snap_radius_bridge.lock()
            && bridge.set_snap_radius_px(pixels)
        {
            persist_user_preferences(&mut bridge);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let click_move_click_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_click_move_click_set(move |enabled| {
        if let Ok(mut bridge) = click_move_click_bridge.lock()
            && bridge.set_click_move_click(enabled)
        {
            persist_user_preferences(&mut bridge);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let studio_light_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_studio_light_follows_camera_set(move |follows| {
        if let Ok(mut bridge) = studio_light_bridge.lock()
            && bridge.set_studio_light_follows_camera(follows)
        {
            persist_user_preferences(&mut bridge);
            let vm = bridge.view_model();
            let frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    for (kind, which) in [("matcap", 0u8), ("ao", 1u8)] {
        let _ = kind;
        let render_bridge = Arc::clone(&bridge);
        let window_weak = window.as_weak();
        let apply = move |enabled: bool| {
            if let Ok(mut bridge) = render_bridge.lock() {
                let changed = if which == 0 {
                    bridge.set_viewport_matcap(enabled)
                } else {
                    bridge.set_viewport_ambient_occlusion(enabled)
                };
                if changed {
                    persist_user_preferences(&mut bridge);
                    let vm = bridge.view_model();
                    let frame = bridge.render_viewport();
                    if let Some(window) = window_weak.upgrade() {
                        sync_window_properties(&window, &vm);
                        if let Some(frame) = frame {
                            window.set_viewport_image(frame);
                        }
                    }
                }
            }
        };
        if which == 0 {
            window.on_viewport_matcap_set(apply);
        } else {
            window.on_viewport_ambient_occlusion_set(apply);
        }
    }

    let workplane_ground_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_workplane_prefer_ground_set(move |enabled| {
        if let Ok(mut bridge) = workplane_ground_bridge.lock()
            && bridge.set_workplane_prefer_ground(enabled)
        {
            persist_user_preferences(&mut bridge);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let component_hover_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    let hover_throttle = std::rc::Rc::clone(&throttle);
    window.on_viewport_hover(move |x, y| {
        if let Ok(mut bridge) = component_hover_bridge.lock() {
            let changed = if bridge.state.session.tools.active_tool == "loop_cut" {
                let viewport_size = bridge.viewport_size;
                bridge.update_loop_cut_hover(x * viewport_size[0], y * viewport_size[1])
            } else {
                crate::perf::measure("hover_component", || bridge.hover_component(x, y))
            };
            // Mouse parado sobre o mesmo alvo: nada a redesenhar. Evita o
            // view model completo, a sincronização da janela e o frame GPU.
            if !changed {
                return;
            }
            if let Some(window) = window_weak.upgrade() {
                crate::refresh::refresh_interactive(
                    &window,
                    &component_hover_bridge,
                    &mut *bridge,
                    &hover_throttle,
                );
            }
        }
    });

    let hover_clear_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_hover_clear(move || {
        if let Ok(mut bridge) = hover_clear_bridge.lock()
            && bridge.clear_hover()
        {
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let gizmo_hover_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    let gizmo_hover_throttle = std::rc::Rc::clone(&throttle);
    window.on_gizmo_hover(move |x, y| {
        if let Ok(mut bridge) = gizmo_hover_bridge.lock()
            && bridge.hover_gizmo(x, y)
            && let Some(window) = window_weak.upgrade()
        {
            crate::refresh::refresh_interactive(
                &window,
                &gizmo_hover_bridge,
                &mut *bridge,
                &gizmo_hover_throttle,
            );
        }
    });

    let gizmo_begin_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_gizmo_drag_begin(move |x, y| {
        if let Ok(mut bridge) = gizmo_begin_bridge.lock() {
            bridge.begin_gizmo_drag(x, y);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let gizmo_end_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_gizmo_drag_end(move || {
        if let Ok(mut bridge) = gizmo_end_bridge.lock() {
            bridge.end_gizmo_drag();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let shading_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_shading_mode_set(move |id| {
        if let Ok(mut bridge) = shading_bridge.lock() {
            bridge.set_shading_mode(id.as_str());
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let xray_toggle_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_xray_requested(move || {
        if let Ok(mut bridge) = xray_toggle_bridge.lock() {
            if let Err(error) = bridge.execute_core_command("view.toggle_xray") {
                bridge.state.set_status(error.to_string());
            }
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let xray_opacity_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_xray_opacity_set(move |opacity| {
        if let Ok(mut bridge) = xray_opacity_bridge.lock() {
            bridge.set_xray_opacity(opacity);
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                window.set_xray_opacity(opacity);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let shading_popover_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_shading_popover_toggled(move |open| {
        if let Ok(mut bridge) = shading_popover_bridge.lock() {
            bridge.shading_popover_open = open;
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let face_orient_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_face_orientation(move || {
        if let Ok(mut bridge) = face_orient_bridge.lock() {
            bridge.toggle_face_orientation();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let uv_checker_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_uv_checker(move || {
        if let Ok(mut bridge) = uv_checker_bridge.lock() {
            bridge.toggle_uv_checker();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let prop_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_proportional_editing(move || {
        if let Ok(mut bridge) = prop_bridge.lock() {
            bridge.toggle_proportional_editing();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let prop_rad_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_set_proportional_radius(move |radius| {
        if let Ok(mut bridge) = prop_rad_bridge.lock() {
            bridge.set_proportional_radius(radius);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let prop_fall_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_set_proportional_falloff(move |falloff| {
        if let Ok(mut bridge) = prop_fall_bridge.lock() {
            bridge.set_proportional_falloff(falloff.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let prop_adj_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_adjust_proportional_radius(move |delta| {
        if let Ok(mut bridge) = prop_adj_bridge.lock() {
            bridge.adjust_proportional_radius(delta);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let snap_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_snap_enabled(move || {
        if let Ok(mut bridge) = snap_bridge.lock() {
            bridge.toggle_snap_enabled();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let snap_target_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_set_snap_target(move |target| {
        if let Ok(mut bridge) = snap_target_bridge.lock() {
            bridge.set_snap_target(target.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let prof_rect_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_add_profile_rectangle(move |width, height| {
        if let Ok(mut bridge) = prof_rect_bridge.lock() {
            bridge.add_profile_rectangle(width, height);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let prof_circle_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_add_profile_circle(move |radius, segments| {
        if let Ok(mut bridge) = prof_circle_bridge.lock() {
            bridge.add_profile_circle(radius, segments as usize);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let decal_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_decal_layer_added(move || {
        if let Ok(mut bridge) = decal_bridge.lock() {
            bridge.add_decal_layer();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let view_axis_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_view_axis_clicked(move |axis| {
        if let Ok(mut bridge) = view_axis_bridge.lock() {
            bridge.snap_view_to_axis(axis.as_str());
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let operand_clear_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_boolean_operand_cleared(move || {
        if let Ok(mut bridge) = operand_clear_bridge.lock() {
            bridge.clear_boolean_operand();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let operand_set_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_boolean_operand_set(move |id| {
        if let Ok(mut bridge) = operand_set_bridge.lock() {
            bridge.set_boolean_operand(id.as_str());
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let fill_scope_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_fill_scope_set(move |scope| {
        if let Ok(mut bridge) = fill_scope_bridge.lock() {
            bridge.set_fill_scope(scope.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let projection_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_projection_set(move |projection| {
        if let Ok(mut bridge) = projection_bridge.lock() {
            bridge.set_brush_projection(projection.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let brush_lock_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_lock_set(move |lock| {
        if let Ok(mut bridge) = brush_lock_bridge.lock() {
            bridge.set_brush_lock(lock.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let paint_stroke_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_2d_stroke(move |norm_x, norm_y, phase, shift, raster_px| {
        if let Ok(mut bridge) = paint_stroke_bridge.lock() {
            bridge.paint_2d_raster_px = raster_px;
            if !bridge.paint_2d_line(norm_x, norm_y, phase, shift) {
                bridge.apply(UiIntent::Paint2dStroke {
                    norm_x,
                    norm_y,
                    phase,
                });
            }
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(canvas_img) = bridge.render_paint_canvas() {
                    window.set_paint_canvas_image(canvas_img);
                }
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let pixel_grid_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_paint_pixel_grid(move || {
        if let Ok(mut bridge) = pixel_grid_bridge.lock() {
            bridge.apply(UiIntent::TogglePaintPixelGrid);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(canvas_img) = bridge.render_paint_canvas() {
                    window.set_paint_canvas_image(canvas_img);
                }
            }
        }
    });

    let canvas_zoom_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_set_paint_canvas_zoom(move |zoom| {
        if let Ok(mut bridge) = canvas_zoom_bridge.lock() {
            bridge.apply(UiIntent::SetPaintCanvasZoom(zoom));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(canvas_img) = bridge.render_paint_canvas() {
                    window.set_paint_canvas_image(canvas_img);
                }
            }
        }
    });

    let uv_click_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_uv_editor_clicked(move |u, v, extend| {
        if let Ok(mut bridge) = uv_click_bridge.lock() {
            bridge.uv_editor_click(u, v, extend);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let uv_move_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_uv_moved(move |du, dv| {
        if let Ok(mut bridge) = uv_move_bridge.lock() {
            bridge.uv_move_selected(du, dv);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let uv_scale_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_uv_scaled(move |factor| {
        if let Ok(mut bridge) = uv_scale_bridge.lock() {
            bridge.uv_scale_selected(factor);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let uv_rotate_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_uv_rotated(move |degrees| {
        if let Ok(mut bridge) = uv_rotate_bridge.lock() {
            bridge.uv_rotate_selected(degrees);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let uv_seam_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_uv_seam_toggled(move || {
        if let Ok(mut bridge) = uv_seam_bridge.lock() {
            bridge.toggle_selected_uv_seams();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let uv_clear_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_uv_seams_cleared(move || {
        if let Ok(mut bridge) = uv_clear_bridge.lock() {
            bridge.clear_all_uv_seams();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let uv_pin_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_uv_pins(move || {
        if let Ok(mut bridge) = uv_pin_bridge.lock() {
            bridge.toggle_selected_uv_pins();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let uv_clear_pins_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_clear_all_uv_pins(move || {
        if let Ok(mut bridge) = uv_clear_pins_bridge.lock() {
            bridge.clear_all_uv_pins();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let extension_bridge = Arc::clone(&bridge);
    let extension_window = window.as_weak();
    window.on_extension_parameter_set(move |key, text| {
        let Ok(mut bridge) = extension_bridge.lock() else {
            return false;
        };
        let changed = bridge.set_extension_parameter(key.as_str(), text.as_str());
        if let Some(window) = extension_window.upgrade() {
            sync_window_properties(&window, &bridge.view_model());
        }
        changed
    });
    let uv_equalize_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_uv_equalize_texel_density(move || {
        if let Ok(mut bridge) = uv_equalize_bridge.lock() {
            bridge.uv_equalize_texel_density();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    // Painel de camadas do PAINT: cada ação recompoe o raster canônico.
    let paint_layer_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_added(move || {
        if let Ok(mut bridge) = paint_layer_bridge.lock() {
            bridge.add_paint_layer();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let paint_group_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_group_added(move || {
        if let Ok(mut bridge) = paint_group_bridge.lock() {
            bridge.add_paint_group();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let paint_remove_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_removed(move |id| {
        if let Ok(mut bridge) = paint_remove_bridge.lock() {
            if !bridge.remove_paint_layer(id.as_str()) {
                let message = bridge
                    .state
                    .t_id(petunia_config::text_id::STATUS_THE_LAST_LAYER_CANNOT_BE_REMOVED);
                bridge.state.set_status(message);
            }
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let paint_merge_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_merged_down(move |id| {
        if let Ok(mut bridge) = paint_merge_bridge.lock() {
            if !bridge.merge_down_paint_layer(id.as_str()) {
                let message = bridge
                    .state
                    .t_id(petunia_config::text_id::STATUS_CANNOT_MERGE_DOWN_THIS_LAYER);
                bridge.state.set_status(message);
            }
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let paint_active_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_activated(move |id| {
        if let Ok(mut bridge) = paint_active_bridge.lock() {
            bridge.set_paint_layer_active(id.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let paint_vis_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_visibility_toggled(move |id| {
        if let Ok(mut bridge) = paint_vis_bridge.lock() {
            bridge.toggle_paint_layer_visibility(id.as_str());
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let paint_lock_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_lock_toggled(move |id| {
        if let Ok(mut bridge) = paint_lock_bridge.lock() {
            bridge.toggle_paint_layer_lock(id.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let paint_move_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_moved(move |id, delta| {
        if let Ok(mut bridge) = paint_move_bridge.lock() {
            bridge.move_paint_layer(id.as_str(), delta);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let paint_opacity_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_opacity_set(move |id, opacity| {
        if let Ok(mut bridge) = paint_opacity_bridge.lock() {
            bridge.set_paint_layer_opacity(id.as_str(), opacity);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let loop_scrub_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_loop_cut_scrubbed(move |delta_x, delta_y| {
        if let Ok(mut bridge) = loop_scrub_bridge.lock() {
            bridge.scrub_loop_cut_2d(delta_x, delta_y, false);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let loop_slide_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_loop_cut_slide_committed(move |text| {
        let Ok(mut bridge) = loop_slide_bridge.lock() else {
            return false;
        };
        let accepted = match numeric::parse_numeric(text.as_str()) {
            Ok(value) => {
                bridge
                    .loop_cut
                    .as_ref()
                    .is_some_and(|session| session.slide == value)
                    || bridge.set_loop_cut_slide(value)
            }
            Err(_) => {
                let message = bridge
                    .state
                    .t_id(petunia_config::text_id::STATUS_LOOP_CUT_SLIDE_MUST_BE_BETWEEN_1);
                bridge.state.set_status(message);
                false
            }
        };
        if let Some(window) = window_weak.upgrade() {
            sync_window_properties(&window, &bridge.view_model());
            if let Some(frame) = bridge.render_viewport() {
                window.set_viewport_image(frame);
            }
        }
        accepted
    });

    let loop_count_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_loop_cut_count_committed(move |text| {
        if let Ok(mut bridge) = loop_count_bridge.lock() {
            match text.trim().parse::<i32>() {
                Ok(cuts) => {
                    bridge.adjust_loop_cut_count_from_input(cuts.clamp(1, 32) as usize);
                }
                Err(_) => {
                    let message = bridge
                        .state
                        .t_id(petunia_config::text_id::STATUS_LOOP_CUT_CUTS_MUST_BE_A_WHOLE);
                    bridge.state.set_status(message);
                }
            }
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let pivot_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_pivot_set(move |id| {
        if let Ok(mut bridge) = pivot_bridge.lock() {
            bridge.set_pivot_point(id.as_str());
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let pivot_menu_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_pivot_menu_toggled(move |open| {
        if let Ok(mut bridge) = pivot_menu_bridge.lock() {
            bridge.set_pivot_menu_open(open);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let micro_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_micro_inspector_toggle(move || {
        if let Ok(mut bridge) = micro_bridge.lock() {
            bridge.toggle_micro_inspector();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let loop_place_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_loop_cut_place(move || {
        if let Ok(mut bridge) = loop_place_bridge.lock() {
            bridge.place_loop_cut_hover();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let profile_close_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_close(move || {
        if let Ok(mut bridge) = profile_close_bridge.lock() {
            bridge.close_profile();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let dimension_gesture_bridge = Arc::clone(&bridge);
    let dimension_gesture_window = window.as_weak();
    window.on_profile_dimension_gesture(move |phase| {
        if let Ok(mut bridge) = dimension_gesture_bridge.lock() {
            bridge.profile_dimension_gesture(phase);
            if let Some(window) = dimension_gesture_window.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });
    let dimensions_bridge = Arc::clone(&bridge);
    let dimensions_window = window.as_weak();
    window.on_profile_dimension_set(move |field, text| {
        let Ok(mut bridge) = dimensions_bridge.lock() else {
            return false;
        };
        let vm = bridge.view_model();
        let base = match field.as_str() {
            "width" => vm.profile_width,
            "height" => vm.profile_height,
            "segments" => vm.profile_segments as f32,
            _ => return false,
        };
        let Ok(value) = numeric::parse_numeric_with_base(text.as_str(), base) else {
            return false;
        };
        let (width, height, segments) = match field.as_str() {
            "width" => (value, vm.profile_height, vm.profile_segments),
            "height" => (vm.profile_width, value, vm.profile_segments),
            "segments" if (6.0..=512.0).contains(&value) && value.fract() == 0.0 => {
                (vm.profile_width, vm.profile_height, value as i32)
            }
            _ => return false,
        };
        let accepted = bridge.set_profile_dimensions(width, height, segments as usize);
        if let Some(window) = dimensions_window.upgrade() {
            sync_window_properties(&window, &bridge.view_model());
            if let Some(frame) = bridge.render_viewport() {
                window.set_viewport_image(frame);
            }
        }
        accepted
    });

    let profile_depth_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_depth_set(move |text| {
        let Ok(depth) = numeric::parse_numeric(text.as_str()) else {
            return false;
        };
        if let Ok(mut bridge) = profile_depth_bridge.lock() {
            let accepted = bridge.set_profile_depth(depth);
            if accepted && let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
            accepted
        } else {
            false
        }
    });

    let profile_enter_extrude_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_enter_extrude(move || {
        if let Ok(mut bridge) = profile_enter_extrude_bridge.lock() {
            bridge.enter_profile_volume("extrude");
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let profile_enter_revolve_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_enter_revolve(move || {
        if let Ok(mut bridge) = profile_enter_revolve_bridge.lock() {
            bridge.enter_profile_volume("revolve");
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let profile_enter_sweep_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_enter_sweep(move || {
        if let Ok(mut bridge) = profile_enter_sweep_bridge.lock() {
            bridge.enter_profile_volume("sweep");
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let profile_volume_confirm_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_volume_confirm(move || {
        if let Ok(mut bridge) = profile_volume_confirm_bridge.lock() {
            bridge.commit_profile_volume();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let profile_volume_cancel_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_volume_cancel(move || {
        if let Ok(mut bridge) = profile_volume_cancel_bridge.lock() {
            bridge.cancel_profile_volume();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let profile_revolve_angle_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_revolve_angle_set(move |text| {
        let Ok(angle) = numeric::parse_numeric(text.as_str()) else {
            return false;
        };
        if let Ok(mut bridge) = profile_revolve_angle_bridge.lock() {
            let accepted = bridge.set_profile_revolve_angle(angle);
            if accepted && let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
            accepted
        } else {
            false
        }
    });

    let profile_generate_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_generate(move || {
        if let Ok(mut bridge) = profile_generate_bridge.lock() {
            bridge.generate_profile_extrude();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let profile_revolve_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_revolve(move || {
        if let Ok(mut bridge) = profile_revolve_bridge.lock() {
            bridge.generate_profile_revolve();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let profile_sweep_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_sweep(move || {
        if let Ok(mut bridge) = profile_sweep_bridge.lock() {
            bridge.generate_profile_sweep();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let profile_wall_thickness_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_wall_thickness_set(move |text| {
        let Ok(thickness) = text.trim().parse::<f32>() else {
            return false;
        };
        if let Ok(mut bridge) = profile_wall_thickness_bridge.lock() {
            let accepted = bridge.set_profile_wall_thickness(thickness);
            if accepted && let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
            accepted
        } else {
            false
        }
    });

    let profile_smoothness_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_smoothness_set(move |text| {
        let Ok(smoothness) = text.trim().parse::<f32>() else {
            return false;
        };
        if let Ok(mut bridge) = profile_smoothness_bridge.lock() {
            let accepted = bridge.set_profile_smoothness(smoothness);
            if accepted && let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
            accepted
        } else {
            false
        }
    });

    let profile_smooth_curves_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_smooth_curves(move || {
        if let Ok(mut bridge) = profile_smooth_curves_bridge.lock() {
            bridge.profile_smooth_curves();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let profile_clear_curves_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_clear_curves(move || {
        if let Ok(mut bridge) = profile_clear_curves_bridge.lock() {
            bridge.profile_clear_curves();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let wp_ground_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_set_workplane_ground(move || {
        if let Ok(mut bridge) = wp_ground_bridge.lock() {
            bridge.apply(UiIntent::ProfileSetWorkplaneGround);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let wp_face_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_set_workplane_face(move || {
        if let Ok(mut bridge) = wp_face_bridge.lock() {
            bridge.apply(UiIntent::ProfileSetWorkplaneFace);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let wp_auto_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_set_workplane_auto(move || {
        if let Ok(mut bridge) = wp_auto_bridge.lock() {
            bridge.apply(UiIntent::ProfileSetWorkplaneAuto);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let wp_look_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_look_at_plane(move || {
        if let Ok(mut bridge) = wp_look_bridge.lock() {
            bridge.apply(UiIntent::ProfileLookAtPlane);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let wp_view_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_set_workplane_view(move || {
        if let Ok(mut bridge) = wp_view_bridge.lock() {
            bridge.apply(UiIntent::ProfileSetWorkplaneView);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let toggle_paint_uv_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_paint_show_uv(move || {
        if let Ok(mut bridge) = toggle_paint_uv_bridge.lock() {
            bridge.apply(UiIntent::TogglePaintUvOverlay);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let toggle_uv_tex_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_uv_show_texture(move || {
        if let Ok(mut bridge) = toggle_uv_tex_bridge.lock() {
            bridge.apply(UiIntent::ToggleUvShowTexture);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let loop_apply_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_loop_cut_apply(move || {
        if let Ok(mut bridge) = loop_apply_bridge.lock() {
            bridge.commit_loop_cut();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let loop_cancel_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_loop_cut_cancel(move || {
        if let Ok(mut bridge) = loop_cancel_bridge.lock() {
            bridge.cancel_loop_cut();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let loop_balanced_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_loop_cut_balanced(move || {
        if let Ok(mut bridge) = loop_balanced_bridge.lock() {
            bridge.toggle_loop_cut_balanced();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let prim_float_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_primitive_param_float(move |param, val| {
        if let Ok(mut bridge) = prim_float_bridge.lock()
            && bridge.update_primitive_param_float(param.as_str(), val)
        {
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let prim_bool_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_primitive_param_bool(move |param, val| {
        if let Ok(mut bridge) = prim_bool_bridge.lock()
            && bridge.update_primitive_param_bool(param.as_str(), val)
        {
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let prim_confirm_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_primitive_confirm(move || {
        if let Ok(mut bridge) = prim_confirm_bridge.lock() {
            bridge.confirm_primitive();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let prim_cancel_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_primitive_cancel(move || {
        if let Ok(mut bridge) = prim_cancel_bridge.lock() {
            bridge.cancel_primitive();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let freeze_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_freeze_primitive_requested(move || {
        if let Ok(mut bridge) = freeze_bridge.lock() {
            bridge.apply(UiIntent::FreezeActivePrimitive);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let slice_trim_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_slice_trim_set(move |trim| {
        if let Ok(mut bridge) = slice_trim_bridge.lock() {
            bridge.set_slice_trim(trim);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let slice_confirm_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_slice_confirm(move || {
        if let Ok(mut bridge) = slice_confirm_bridge.lock()
            && bridge.commit_slice()
        {
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let slice_cancel_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_slice_cancel(move || {
        if let Ok(mut bridge) = slice_cancel_bridge.lock()
            && bridge.cancel_slice()
        {
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let bevel_clamp_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_bevel_clamp_overlap(move || {
        if let Ok(mut bridge) = bevel_clamp_bridge.lock() {
            bridge.toggle_bevel_clamp_overlap();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let bevel_affect_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_bevel_affect_vertices(move || {
        if let Ok(mut bridge) = bevel_affect_bridge.lock() {
            bridge.toggle_bevel_affect_vertices();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let recover_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_recovery_recover_requested(move || {
        if let Ok(mut bridge) = recover_bridge.lock() {
            bridge.recover_pending();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let keep_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_recovery_keep_requested(move || {
        if let Ok(mut bridge) = keep_bridge.lock() {
            bridge.keep_saved_project();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let discard_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_recovery_discard_requested(move || {
        if let Ok(mut bridge) = discard_bridge.lock() {
            bridge.discard_pending_recovery();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let pip_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_pip_toggle_requested(move || {
        if let Ok(mut bridge) = pip_bridge.lock() {
            bridge.toggle_paint_pip();
            let vm = bridge.view_model();
            let frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let split_toggle_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_split_toggle_requested(move || {
        if let Ok(mut bridge) = split_toggle_bridge.lock() {
            bridge.toggle_split_view();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let split_preset_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_split_preset_requested(move |id| {
        if let Ok(mut bridge) = split_preset_bridge.lock() {
            bridge.set_split_preset(id.as_str());
            let vm = bridge.view_model();
            let frame = bridge.render_split_view();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = frame {
                    window.set_split_view_image(frame);
                }
            }
        }
    });

    let split_orbit_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_split_orbit(move |dx, dy| {
        if let Ok(mut bridge) = split_orbit_bridge.lock() {
            bridge.split.orbit(dx, dy);
            let vm = bridge.view_model();
            let frame = bridge.render_split_view();
            if let Some(window) = window_weak.upgrade() {
                window.set_split_preset(vm.split_preset.as_str().into());
                if let Some(frame) = frame {
                    window.set_split_view_image(frame);
                }
            }
        }
    });

    let split_pan_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_split_pan(move |dx, dy| {
        if let Ok(mut bridge) = split_pan_bridge.lock() {
            bridge.split.pan(dx, dy);
            let frame = bridge.render_split_view();
            if let (Some(window), Some(frame)) = (window_weak.upgrade(), frame) {
                window.set_split_view_image(frame);
            }
        }
    });

    let split_zoom_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_split_zoom(move |delta| {
        if let Ok(mut bridge) = split_zoom_bridge.lock() {
            bridge.split.zoom(delta);
            let frame = bridge.render_split_view();
            if let (Some(window), Some(frame)) = (window_weak.upgrade(), frame) {
                window.set_split_view_image(frame);
            }
        }
    });

    let split_resize_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_split_resized(move |width, height| {
        if let Ok(mut bridge) = split_resize_bridge.lock() {
            bridge.split.resize(
                width.round().max(1.0) as u32,
                height.round().max(1.0) as u32,
            );
            let frame = bridge.render_split_view();
            if let (Some(window), Some(frame)) = (window_weak.upgrade(), frame) {
                window.set_split_view_image(frame);
            }
        }
    });

    let place_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_prefab_place_requested(move |id| {
        if let Ok(mut bridge) = place_bridge.lock() {
            bridge.place_prefab(id.as_str());
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let prefab_delete_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_prefab_delete_requested(move |id| {
        if let Ok(mut bridge) = prefab_delete_bridge.lock() {
            bridge.delete_prefab(id.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let prefab_update_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_prefab_update_requested(move |id| {
        if let Ok(mut bridge) = prefab_update_bridge.lock() {
            bridge.update_prefab(id.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let prefab_favorite_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_prefab_favorite_requested(move |id, favorite| {
        if let Ok(mut bridge) = prefab_favorite_bridge.lock() {
            bridge.set_prefab_favorite(id.as_str(), favorite);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let only_favorites_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_asset_only_favorites_changed(move |only| {
        if let Ok(mut bridge) = only_favorites_bridge.lock() {
            bridge.set_asset_only_favorites(only);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let select_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_scene_select(move |id, extend| {
        if let Ok(mut bridge) = select_bridge.lock() {
            if let Some(profile_uuid_str) = id.as_str().strip_prefix("profile:") {
                if let Ok(profile_id) = uuid::Uuid::parse_str(profile_uuid_str) {
                    bridge.cancel_active_operation();
                    bridge.activate_profile(profile_id);
                    bridge.state.select_object(None, false);
                    bridge.reset_transform_fields();
                }
            } else if let Some(index) = bridge
                .state
                .project
                .assets
                .iter()
                .position(|a| a.id.to_string() == id.as_str())
            {
                bridge.cancel_active_operation();
                bridge.active_profile_id = None;
                bridge.state.select_object(Some(index), extend);
                bridge.reset_transform_fields();
            }
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let vis_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_scene_toggle_visibility(move |id| {
        if let Ok(mut bridge) = vis_bridge.lock() {
            bridge.apply(UiIntent::ToggleSceneAssetVisibility(
                id.as_str().to_string(),
            ));
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let lock_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_scene_toggle_lock(move |id| {
        if let Ok(mut bridge) = lock_bridge.lock() {
            bridge.apply(UiIntent::ToggleSceneAssetLock(id.as_str().to_string()));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let part_color_open_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_part_color_picker_open(move |id| {
        let id_str = id.to_string();
        if let Ok(bridge) = part_color_open_bridge.lock()
            && let Some(window) = window_weak.upgrade()
            && let Ok(asset_id) = uuid::Uuid::parse_str(&id_str)
            && let Some(asset) = bridge
                .state
                .project
                .project
                .assets
                .iter()
                .find(|a| a.id == asset_id)
        {
            let custom_color = asset.selection_overlay_color;
            let eff = custom_color.unwrap_or(bridge.state.ui.selection_rgb);
            let hex = format!("#{:02X}{:02X}{:02X}", eff[0], eff[1], eff[2]);
            window.set_part_color_popover_id(id_str.as_str().into());
            window.set_part_color_popover_name(asset.name.as_str().into());
            window.set_part_color_preview(slint::Color::from_rgb_u8(eff[0], eff[1], eff[2]));
            window.set_part_color_hex(hex.into());
            window.set_part_color_has_custom(custom_color.is_some());
            window.set_color_wheel_image(crate::color_wheel::generate_color_wheel_image(130));
            window.set_part_color_popover_open(true);
        }
    });

    let wheel_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_part_color_wheel_interacted(move |x, y| {
        if let Some(window) = window_weak.upgrade() {
            let id = window.get_part_color_popover_id().to_string();
            let dx = x - 65.0;
            let dy = y - 65.0;
            let rgb = crate::color_wheel::sample_wheel_color(dx, dy, 64.0);
            let hex = format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]);
            window.set_part_color_preview(slint::Color::from_rgb_u8(rgb[0], rgb[1], rgb[2]));
            window.set_part_color_hex(hex.into());
            window.set_part_color_has_custom(true);

            if let Ok(mut bridge) = wheel_bridge.lock() {
                bridge.set_part_selection_color(&id, Some(rgb));
                let vm = bridge.view_model();
                let new_frame = bridge.render_viewport();
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let hex_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_part_color_hex_entered(move |hex| {
        let clean = hex.trim().strip_prefix('#').unwrap_or(hex.trim());
        if let Ok(val) = u32::from_str_radix(clean, 16) {
            let rgb = match clean.len() {
                6 => [
                    ((val >> 16) & 0xFF) as u8,
                    ((val >> 8) & 0xFF) as u8,
                    (val & 0xFF) as u8,
                ],
                3 => [
                    (((val >> 8) & 0xF) * 17) as u8,
                    (((val >> 4) & 0xF) * 17) as u8,
                    ((val & 0xF) * 17) as u8,
                ],
                _ => return,
            };
            if let Some(window) = window_weak.upgrade() {
                let id = window.get_part_color_popover_id().to_string();
                window.set_part_color_preview(slint::Color::from_rgb_u8(rgb[0], rgb[1], rgb[2]));
                window.set_part_color_has_custom(true);
                if let Ok(mut bridge) = hex_bridge.lock() {
                    bridge.set_part_selection_color(&id, Some(rgb));
                    let vm = bridge.view_model();
                    let new_frame = bridge.render_viewport();
                    sync_window_properties(&window, &vm);
                    if let Some(frame) = new_frame {
                        window.set_viewport_image(frame);
                    }
                }
            }
        }
    });

    let preset_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_part_color_preset_clicked(move |color| {
        if let Some(window) = window_weak.upgrade() {
            let id = window.get_part_color_popover_id().to_string();
            let rgb = [color.red(), color.green(), color.blue()];
            let hex = format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]);
            window.set_part_color_preview(color);
            window.set_part_color_hex(hex.into());
            window.set_part_color_has_custom(true);

            if let Ok(mut bridge) = preset_bridge.lock() {
                bridge.set_part_selection_color(&id, Some(rgb));
                let vm = bridge.view_model();
                let new_frame = bridge.render_viewport();
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let reset_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_part_color_reset_requested(move || {
        if let Some(window) = window_weak.upgrade() {
            let id = window.get_part_color_popover_id().to_string();
            if let Ok(mut bridge) = reset_bridge.lock() {
                bridge.set_part_selection_color(&id, None);
                let default_rgb = bridge.state.ui.selection_rgb;
                let hex = format!(
                    "#{:02X}{:02X}{:02X}",
                    default_rgb[0], default_rgb[1], default_rgb[2]
                );
                window.set_part_color_preview(slint::Color::from_rgb_u8(
                    default_rgb[0],
                    default_rgb[1],
                    default_rgb[2],
                ));
                window.set_part_color_hex(hex.into());
                window.set_part_color_has_custom(false);

                let vm = bridge.view_model();
                let new_frame = bridge.render_viewport();
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let sep_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_separate_selection_requested(move || {
        if let Ok(mut bridge) = sep_bridge.lock() {
            let _ = bridge.execute_core_command("model.separate_selection");
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let move_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    // Reorders an asset in the scene hierarchy / Reordena um asset na hierarquia da cena
    window.on_scene_move(move |id, delta| {
        if let Ok(mut bridge) = move_bridge.lock() {
            bridge.apply(UiIntent::MoveSceneAsset {
                id: id.as_str().to_string(),
                delta,
            });
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let theme_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_theme_changed(move |theme_id| {
        if let Ok(mut bridge) = theme_bridge.lock() {
            bridge.apply(UiIntent::SetTheme(theme_id.as_str().to_string()));
            persist_user_preferences(&mut bridge);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let undo_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_undo_requested(move || {
        if let Ok(mut bridge) = undo_bridge.lock() {
            bridge.apply(UiIntent::Undo);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let redo_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_redo_requested(move || {
        if let Ok(mut bridge) = redo_bridge.lock() {
            bridge.apply(UiIntent::Redo);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let domain_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_selection_domain_changed(move |domain_str| {
        let domain = match domain_str.as_str() {
            "OBJECT" => SelectionDomain::Object,
            "POINT" => SelectionDomain::Vertex,
            "EDGE" => SelectionDomain::Edge,
            "FACE" => SelectionDomain::Face,
            _ => return,
        };
        if let Ok(mut bridge) = domain_bridge.lock() {
            bridge.apply(UiIntent::SetSelectionDomain(domain));
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let tool_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_active_tool_changed(move |tool| {
        if let Ok(mut bridge) = tool_bridge.lock() {
            bridge.apply(UiIntent::SetActiveTool(tool.as_str().to_string()));
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let primitive_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_add_primitive_requested(move |kind_str| {
        let Some(kind) = petunia_core::PrimitiveKind::parse(kind_str.as_str()).ok() else {
            if let Ok(mut bridge) = primitive_bridge.lock() {
                let message = crate::tr::fill(
                    &bridge
                        .state
                        .t_id(petunia_config::text_id::STATUS_UNKNOWN_PRIMITIVE),
                    &[("kind_str", format!("{kind_str}"))],
                );
                bridge.state.set_status(message);
            }
            return;
        };
        if let Ok(mut bridge) = primitive_bridge.lock() {
            bridge.apply(UiIntent::AddPrimitive(kind));
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let delete_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_delete_requested(move || {
        if let Ok(mut bridge) = delete_bridge.lock() {
            bridge.delete_or_dissolve_selection();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let color_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_color_changed(move |r, g, b| {
        if let Ok(mut bridge) = color_bridge.lock() {
            bridge.apply(UiIntent::SetPaintColor([r, g, b]));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let size_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_brush_size_changed(move |size| {
        if let Ok(mut bridge) = size_bridge.lock() {
            bridge.apply(UiIntent::SetBrushSize(size));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let opacity_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_brush_opacity_changed(move |opacity| {
        if let Ok(mut bridge) = opacity_bridge.lock() {
            bridge.apply(UiIntent::SetBrushOpacity(opacity));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let hardness_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_brush_hardness_changed(move |hardness| {
        if let Ok(mut bridge) = hardness_bridge.lock() {
            bridge.apply(UiIntent::SetBrushHardness(hardness));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let sym_x_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_symmetry_x_toggled(move || {
        if let Ok(mut bridge) = sym_x_bridge.lock() {
            bridge.apply(UiIntent::TogglePaintSymmetryX);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let sym_y_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_symmetry_y_toggled(move || {
        if let Ok(mut bridge) = sym_y_bridge.lock() {
            bridge.apply(UiIntent::TogglePaintSymmetryY);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let sym_z_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_symmetry_z_toggled(move || {
        if let Ok(mut bridge) = sym_z_bridge.lock() {
            bridge.apply(UiIntent::TogglePaintSymmetryZ);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let target_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_target_vertex_changed(move |val| {
        if let Ok(mut bridge) = target_bridge.lock() {
            bridge.apply(UiIntent::SetPaintTargetVertex(val));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let pen_mode_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_poly_pen_mode_selected(move |mode| {
        if let Ok(mut bridge) = pen_mode_bridge.lock() {
            bridge.apply(UiIntent::SetPolyPenMode(mode.as_str().to_string()));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let mask_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_mask_selection_toggled(move || {
        if let Ok(mut bridge) = mask_bridge.lock() {
            bridge.apply(UiIntent::TogglePaintMaskSelection);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let dup_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_duplicate_requested(move || {
        if let Ok(mut bridge) = dup_bridge.lock() {
            bridge.duplicate_selection();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let copy_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_copy_requested(move || {
        if let Ok(mut bridge) = copy_bridge.lock() {
            bridge.copy_selection();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let paste_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paste_requested(move || {
        if let Ok(mut bridge) = paste_bridge.lock() {
            bridge.paste_clipboard();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let asset_lib_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_asset_library_requested(move || {
        if let Ok(mut bridge) = asset_lib_bridge.lock() {
            bridge.apply(UiIntent::ToggleAssetLibrary);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                window.set_asset_library_visible(bridge.asset_library_visible);
                sync_window_properties(&window, &vm);
            }
        }
    });

    let proj_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_projection_requested(move || {
        if let Ok(mut bridge) = proj_bridge.lock() {
            bridge.apply(UiIntent::ToggleProjection);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let wire_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_wireframe_requested(move || {
        if let Ok(mut bridge) = wire_bridge.lock() {
            bridge.execute_command(CommandId::ToggleWireOverlay);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let reset_cam_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_reset_camera_requested(move || {
        if let Ok(mut bridge) = reset_cam_bridge.lock() {
            bridge.apply(UiIntent::ResetCamera);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let save_asset_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_save_active_as_asset_requested(move || {
        if let Ok(mut bridge) = save_asset_bridge.lock() {
            bridge.apply(UiIntent::SaveActiveAsAsset);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let assign_mat_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_assign_material_slot(move |slot| {
        if slot < 0 {
            return;
        }
        if let Ok(mut bridge) = assign_mat_bridge.lock() {
            bridge.apply(UiIntent::AssignMaterialSlot(slot as usize));
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let create_mat_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_create_material(move || {
        if let Ok(mut bridge) = create_mat_bridge.lock() {
            bridge.apply(UiIntent::CreateMaterial);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let dup_mat_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_duplicate_material(move |slot| {
        if slot < 0 {
            return;
        }
        if let Ok(mut bridge) = dup_mat_bridge.lock() {
            bridge.apply(UiIntent::DuplicateMaterial(slot as usize));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let material_slot_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_material_slot_selected(move |slot| {
        if let Ok(mut bridge) = material_slot_bridge.lock() {
            bridge.select_material_slot(slot);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let material_color_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_material_base_color_changed(move |red, green, blue| {
        if let Ok(mut bridge) = material_color_bridge.lock() {
            bridge.set_active_material_base_color(red, green, blue);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let material_scalar_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_material_scalar_committed(move |field, slot, text| {
        let Ok(mut bridge) = material_scalar_bridge.lock() else {
            return false;
        };
        let Ok(value) = numeric::parse_numeric(text.as_str()) else {
            return false;
        };
        bridge.select_material_slot(slot);
        let result = bridge.set_active_material_scalar(field.as_str(), value);
        if let Some(window) = window_weak.upgrade() {
            sync_window_properties(&window, &bridge.view_model());
            if let Some(frame) = bridge.render_viewport() {
                window.set_viewport_image(frame);
            }
        }
        result
    });

    let material_profile_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_material_profile_changed(move |profile| {
        if let Ok(mut bridge) = material_profile_bridge.lock() {
            bridge.set_active_material_profile(profile);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let material_alpha_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_material_alpha_mode_changed(move |mode| {
        if let Ok(mut bridge) = material_alpha_bridge.lock() {
            bridge.set_active_material_alpha_mode(mode);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let material_texture_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_material_albedo_create(move || {
        if let Ok(mut bridge) = material_texture_bridge.lock() {
            bridge.create_albedo_texture();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
    let material_texture_clear_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_material_albedo_clear(move || {
        if let Ok(mut bridge) = material_texture_clear_bridge.lock() {
            bridge.clear_albedo_texture();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let remove_material_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_remove_material(move |slot| {
        if let Ok(mut bridge) = remove_material_bridge.lock() {
            bridge.remove_material(slot);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let quick_execute_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_quick_action_executed(move |id| {
        if let Ok(mut bridge) = quick_execute_bridge.lock() {
            bridge.execute_quick_action(id.as_str());
            let vm = bridge.view_model();
            let frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let quick_action_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_quick_action_toggled(move |id| {
        if let Ok(mut bridge) = quick_action_bridge.lock() {
            bridge.toggle_quick_action(id.as_str());
            persist_user_preferences(&mut bridge);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
    let quick_reset_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_quick_actions_reset(move || {
        if let Ok(mut bridge) = quick_reset_bridge.lock() {
            bridge.reset_quick_actions();
            persist_user_preferences(&mut bridge);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let modifier_add_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_modifier_add(move |kind| {
        if let Ok(mut bridge) = modifier_add_bridge.lock() {
            bridge.add_modifier(kind.as_str());
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
    let modifier_toggle_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_modifier_toggle(move |id, enabled| {
        if let Ok(mut bridge) = modifier_toggle_bridge.lock() {
            bridge.set_modifier_enabled(id.as_str(), enabled);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
    let modifier_remove_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_modifier_remove(move |id| {
        if let Ok(mut bridge) = modifier_remove_bridge.lock() {
            bridge.remove_modifier(id.as_str());
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
    let modifier_move_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_modifier_move(move |id, direction| {
        if let Ok(mut bridge) = modifier_move_bridge.lock() {
            bridge.move_modifier(id.as_str(), direction);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
    let modifier_apply_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_modifier_apply(move |id| {
        if let Ok(mut bridge) = modifier_apply_bridge.lock() {
            bridge.apply_modifier(id.as_str());
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
    let modifier_axis_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_modifier_axis_changed(move |id, axis| {
        if let Ok(mut bridge) = modifier_axis_bridge.lock() {
            bridge.set_modifier_axis(id.as_str(), axis);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
    let modifier_direction_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_modifier_direction_changed(move |id, direction| {
        if let Ok(mut bridge) = modifier_direction_bridge.lock() {
            bridge.set_modifier_direction(id.as_str(), direction);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
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

    let decal_transform_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    // Handles decal transform changes (P3D-133) / Trata alterações na transformação do decalque (P3D-133)
    window.on_decal_transform_changed(move |id, cu, cv, su, sv, rot| {
        if let Ok(mut bridge) = decal_transform_bridge.lock() {
            let target_id = if id.is_empty() {
                bridge
                    .state
                    .project
                    .assets
                    .get(bridge.state.project.active)
                    .and_then(|asset| asset.paint_stack.as_ref())
                    .and_then(|stack| stack.active())
                    .map(|layer| layer.id.to_string())
                    .unwrap_or_default()
            } else {
                id.as_str().to_string()
            };
            if !target_id.is_empty() {
                bridge.apply(UiIntent::SetDecalTransform {
                    layer_id: target_id,
                    center_u: cu,
                    center_v: cv,
                    scale_u: su,
                    scale_v: sv,
                    rotation_deg: rot,
                });
                let vm = bridge.view_model();
                let new_frame = bridge.render_viewport();
                if let Some(window) = window_weak.upgrade() {
                    sync_window_properties(&window, &vm);
                    if let Some(frame) = new_frame {
                        window.set_viewport_image(frame);
                    }
                }
            }
        }
    });

    let texture_side_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_texture_side_selected(move |side| {
        if let Ok(mut bridge) = texture_side_bridge.lock() {
            bridge.apply(UiIntent::SetPaintTextureSide(side.max(0) as u32));
            let vm = bridge.view_model();
            let frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                bridge.publish_canvas_image(&window);
                if let Some(frame) = frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let bake_decal_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    // Bakes active decal layer to static raster (P3D-160) / Converte decalque ativo em raster estático (P3D-160)
    window.on_bake_active_decal(move || {
        if let Ok(mut bridge) = bake_decal_bridge.lock() {
            bridge.apply(UiIntent::BakeActiveDecal);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let ref_toggle_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_reference_manager_toggle(move || {
        if let Ok(mut bridge) = ref_toggle_bridge.lock() {
            bridge.toggle_reference_manager();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let ref_close_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_reference_manager_closed(move || {
        if let Ok(mut bridge) = ref_close_bridge.lock() {
            bridge.close_reference_manager();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let ref_pick_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_reference_slot_pick(move |axis| {
        let bridge = Arc::clone(&ref_pick_bridge);
        let window_weak = window_weak.clone();
        let axis = axis.to_string();
        let _ = slint::spawn_local(async move {
            let service = files::FileDialogService::new();
            let Some(path) = service.open_reference_image().await else {
                return;
            };
            let file_name = path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| "reference".to_string());
            let Ok((width, height, rgba)) = files::load_image_rgba(&path) else {
                return;
            };
            if let Ok(mut bridge) = bridge.lock() {
                bridge.load_reference_slot(&axis, file_name, width, height, rgba);
                let vm = bridge.view_model();
                let new_frame = bridge.render_viewport();
                if let Some(window) = window_weak.upgrade() {
                    sync_window_properties(&window, &vm);
                    if let Some(frame) = new_frame {
                        window.set_viewport_image(frame);
                    }
                }
            }
        });
    });

    let ref_remove_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_reference_slot_remove(move |axis| {
        if let Ok(mut bridge) = ref_remove_bridge.lock() {
            bridge.remove_reference_slot(axis.as_str());
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let ref_align_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_reference_slot_align(move |axis| {
        if let Ok(mut bridge) = ref_align_bridge.lock() {
            bridge.align_reference_view(axis.as_str());
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let ref_vis_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_reference_slot_toggle_visible(move |axis| {
        if let Ok(mut bridge) = ref_vis_bridge.lock() {
            bridge.toggle_reference_visible(axis.as_str());
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let ref_lock_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_reference_slot_toggle_lock(move |axis| {
        if let Ok(mut bridge) = ref_lock_bridge.lock() {
            bridge.toggle_reference_lock(axis.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let ref_param_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_reference_slot_set_param(move |axis, param, val| {
        if let Ok(mut bridge) = ref_param_bridge.lock() {
            bridge.set_reference_param(axis.as_str(), param.as_str(), val);
            let new_frame = bridge.render_viewport();
            if let (Some(window), Some(frame)) = (window_weak.upgrade(), new_frame) {
                window.set_viewport_image(frame);
            }
        }
    });

    let ref_clear_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_reference_clear_all(move || {
        if let Ok(mut bridge) = ref_clear_bridge.lock() {
            bridge.clear_all_references();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });
}
