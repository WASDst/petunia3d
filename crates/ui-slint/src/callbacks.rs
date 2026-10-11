use crate::bridge::shell_layout::sync_inspector_pane_layout;
// Slint UI callback registration and shell property synchronization.
// Registro de callbacks da UI Slint e sincronização de propriedades do shell.

use petunia_core::{SelectionDomain, Workspace};
use slint::ComponentHandle;
use std::sync::{Arc, Mutex};

use crate::overlay::OverlayId;
use crate::*;
mod sync;
pub(crate) use sync::sync_window_properties;

mod commands;
mod materials;
mod menus;
mod modeling;
mod outliner;
mod paint;
mod references;
mod session;
mod settings;
mod transform_inputs;
mod uv;
mod view_options;

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
    connect_decal_callbacks(window, Arc::clone(&bridge));
    crate::bridge::viewport::connect_navigation_callbacks(window, Arc::clone(&bridge));
    crate::bridge::viewport::connect_selection_callbacks(window, Arc::clone(&bridge));
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
    crate::bridge::viewport::connect_hover_callbacks(
        window,
        Arc::clone(&bridge),
        std::rc::Rc::clone(&throttle),
    );
    crate::bridge::viewport::connect_transform_callbacks(
        window,
        Arc::clone(&bridge),
        std::rc::Rc::clone(&throttle),
    );
    crate::bridge::viewport::connect_tool_pointer_callbacks(
        window,
        Arc::clone(&bridge),
        std::rc::Rc::clone(&throttle),
    );
    crate::bridge::viewport::connect_split_view_callbacks(window, Arc::clone(&bridge));
    crate::bridge::model::connect_profile_callbacks(
        window,
        Arc::clone(&bridge),
        std::rc::Rc::clone(&throttle),
    );
    crate::bridge::paint::connect_viewport_paint_callbacks(
        window,
        Arc::clone(&bridge),
        std::rc::Rc::clone(&throttle),
    );
    crate::bridge::shell_layout::connect_callbacks(window, Arc::clone(&bridge));

    session::connect_session_callbacks(window, Arc::clone(&bridge));
    commands::connect_commands_callbacks(window, Arc::clone(&bridge));
    settings::connect_settings_callbacks(window, Arc::clone(&bridge));
    outliner::connect_outliner_callbacks(window, Arc::clone(&bridge));
    menus::connect_menus_callbacks(window, Arc::clone(&bridge));
    transform_inputs::connect_transform_inputs_callbacks(
        window,
        Arc::clone(&bridge),
        std::rc::Rc::clone(&throttle),
    );
    view_options::connect_view_options_callbacks(window, Arc::clone(&bridge));
    modeling::connect_modeling_callbacks(window, Arc::clone(&bridge));
    materials::connect_materials_callbacks(window, Arc::clone(&bridge));
    paint::connect_paint_callbacks(window, Arc::clone(&bridge));
    uv::connect_uv_callbacks(window, Arc::clone(&bridge));
    references::connect_references_callbacks(window, Arc::clone(&bridge));
}

/// Atualiza a janela depois de uma ação que pode mudar a textura.
fn refresh_after_texture_change<V: PetuniaViewport>(
    bridge: &mut SlintUiBridge<V>,
    window: &PetuniaSlintShell,
) {
    let vm = bridge.view_model();
    let frame = bridge.render_viewport();
    sync_window_properties(window, &vm);
    bridge.publish_canvas_image(window);
    if let Some(frame) = frame {
        window.set_viewport_image(frame);
    }
}

/// Decal Set (variantes e trilha), seleção pelo UV no canvas 2D.
fn connect_decal_callbacks<V: PetuniaViewport + 'static>(
    window: &PetuniaSlintShell,
    bridge: Arc<Mutex<SlintUiBridge<V>>>,
) {
    macro_rules! on_intent {
        ($setter:ident, || $intent:expr) => {
            on_intent!($setter, | | $intent)
        };
        ($setter:ident, |$($arg:ident),*| $intent:expr) => {{
            let bridge = Arc::clone(&bridge);
            let window_weak = window.as_weak();
            window.$setter(move |$($arg),*| {
                if let Ok(mut bridge) = bridge.lock() {
                    bridge.apply($intent);
                    if let Some(window) = window_weak.upgrade() {
                        refresh_after_texture_change(&mut bridge, &window);
                    }
                }
            });
        }};
    }
    on_intent!(
        on_decal_variant_selected,
        |index| UiIntent::SetDecalVariant(index.max(0) as usize)
    );
    on_intent!(on_decal_variant_remove_requested, || {
        UiIntent::RemoveDecalVariant
    });
    on_intent!(on_decal_key_set_requested, || UiIntent::SetDecalKey);
    on_intent!(on_decal_key_remove_requested, || UiIntent::RemoveDecalKey);
    on_intent!(on_decal_frame_changed, |frame| UiIntent::SetDecalFrame(
        frame
    ));
    on_intent!(on_decal_fps_changed, |fps| UiIntent::SetDecalTrackFps(fps));
    on_intent!(on_decal_length_changed, |frames| {
        UiIntent::SetDecalTrackLength(frames)
    });
    on_intent!(on_decal_play_toggled, || UiIntent::ToggleDecalPlayback);

    // Reprodução: o quadro anda a cada tique; a textura só é refeita quando
    // a variante mostrada muda.
    let tick_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_decal_playback_tick(move |dt| {
        if let Ok(mut bridge) = tick_bridge.lock() {
            let changed = bridge.decal_playback_tick(dt);
            if let Some(window) = window_weak.upgrade() {
                window.set_decal_frame(bridge.decal_frame as i32);
                if changed {
                    refresh_after_texture_change(&mut bridge, &window);
                }
            }
        }
    });

    let select_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_2d_select(move |x0, y0, x1, y1, extend, subtract, island| {
        if let Ok(mut bridge) = select_bridge.lock()
            && bridge.paint_2d_select(x0, y0, x1, y1, extend, subtract, island)
            && let Some(window) = window_weak.upgrade()
        {
            refresh_after_texture_change(&mut bridge, &window);
        }
    });
}
