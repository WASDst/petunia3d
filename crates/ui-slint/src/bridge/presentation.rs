//! Projeção do estado para o ViewModel do shell.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub fn view_model(&self) -> ShellViewModel {
        puffin::profile_function!();
        let mut vm = ShellViewModel::from_state(&self.state);
        self.project_scene_state(&mut vm);
        self.project_viewport_hud(&mut vm);
        self.project_labels(&mut vm);
        self.project_menus_and_canvas(&mut vm);
        self.project_paint_uv(&mut vm);
        self.project_modeling(&mut vm);
        self.project_settings(&mut vm);
        self.project_tool_options(&mut vm);
        vm
    }

    /// Estado de cena, Inspector, partes, prefabs e seleção.
    fn project_scene_state(&self, vm: &mut ShellViewModel) {
        vm.paint_target_vertex = self.paint_target_vertex;
        vm.paint_mask_selection = self.state.session.tools.paint_isolate_selection;
        vm.position = [
            self.position[0].value(),
            self.position[1].value(),
            self.position[2].value(),
        ];
        vm.rotation = [
            self.rotation[0].value(),
            self.rotation[1].value(),
            self.rotation[2].value(),
        ];
        vm.scale = [
            self.scale[0].value(),
            self.scale[1].value(),
            self.scale[2].value(),
        ];
        vm.animate = self.animate_view_model();
        vm.is_orthographic = self.state.session.camera.proj == petunia_core::Projection::Ortho;
        vm.is_wireframe = self.state.session.show_wireframe_overlay;
        vm.asset_library_visible = self.asset_library_visible;
        vm.parts_query = self.parts_query.clone();
        vm.parts_selected_only = self.parts_selected_only;
        vm.parts_sort_by_name = self.parts_sort_by_name;
        vm.parts_row_height = self.parts_row_height;
        vm.inspector_pane_layout = self.inspector_pane_layout();

        for profile in &self.state.project.project.profiles {
            let pts = self
                .state
                .project
                .project
                .get_spline(profile.spline_id)
                .map_or(0, |s| s.points.len());
            let is_active = self.active_profile_id == Some(profile.id);
            vm.scene_items.push(SceneItemModel {
                id: format!("profile:{}", profile.id),
                name: profile.name.clone(),
                visible: true,
                locked: false,
                selected: is_active,
                active: is_active,
                verts: pts,
                tris: 0,
                has_custom_color: false,
                custom_color_rgb: self.state.ui.selection_rgb,
            });
        }

        let parts_query = self.parts_query.trim().to_lowercase();
        vm.parts_items = vm
            .scene_items
            .iter()
            .filter(|item| {
                (parts_query.is_empty() || item.name.to_lowercase().contains(&parts_query))
                    && (!self.parts_selected_only || item.selected || item.active)
            })
            .cloned()
            .collect();
        if self.parts_sort_by_name {
            vm.parts_items.sort_by(|a, b| {
                a.name
                    .to_lowercase()
                    .cmp(&b.name.to_lowercase())
                    .then_with(|| a.id.cmp(&b.id))
            });
        }
        vm.asset_query = self.asset_query.clone();
        vm.asset_sort_by_name = self.asset_sort_by_name;
        vm.asset_thumbnail_size = self.state.ui.asset_thumbnail_size;
        vm.asset_only_favorites = self.asset_only_favorites;
        vm.split_enabled = self.split.enabled;
        vm.paint_pip = self.paint_pip;
        vm.split_preset = self.split.preset_id().to_string();
        vm.prefab_items = self.prefab_item_models();
        vm.gizmo = compute_gizmo(&self.state, self.viewport_size[0], self.viewport_size[1]);
        if self.viewport.draws_gizmo() {
            vm.gizmo.clear_handle_geometry();
        }
        vm.selection_overlay = compute_selection_overlay(
            &self.state,
            self.viewport_size[0],
            self.viewport_size[1],
            self.viewport.draws_component_guides(),
        );
        vm.add_menu_open = self.add_menu_open;
        if let Some(draft) = &self.rename_draft {
            vm.rename_active = true;
            vm.rename_value = draft.clone();
        }
        vm.shading_popover_open = self.shading_popover_open;
        vm.section_states = self.section_state_models();
    }

    /// Gizmo, HUD de operação, guias, medidas e overlays da viewport.
    fn project_viewport_hud(&self, vm: &mut ShellViewModel) {
        vm.transform_instant_active =
            self.instant_transform && self.state.session.tools.modal.is_some();
        vm.gizmo_hover_axis = self
            .gizmo_hover
            .and_then(|h| h.axis())
            .map_or(-1, |a| a as i32);
        vm.gizmo_active_axis = self
            .gizmo_drag
            .and_then(|h| h.axis())
            .map_or(-1, |a| a as i32);
        vm.gizmo_has_hover = self.gizmo_hover.is_some();
        vm.gizmo_center_active = matches!(self.gizmo_drag, Some(GizmoHandle::Center));
        vm.gizmo_center_hover = matches!(self.gizmo_hover, Some(GizmoHandle::Center));
        vm.gizmo_constraint_axes = match self
            .state
            .session
            .tools
            .modal
            .as_ref()
            .map(|op| op.constraint)
        {
            Some(petunia_core::ModalConstraint::Axis(axis)) => [axis as i32, -1],
            Some(petunia_core::ModalConstraint::Plane(excluded)) => {
                [((excluded + 1) % 3) as i32, ((excluded + 2) % 3) as i32]
            }
            _ => [-1, -1],
        };
        let link_active = self.drag.is_some()
            || self.tool_modal.is_some()
            || self.state.session.tools.modal.is_some();
        vm.drag_link_commands = compute_drag_link(
            &self.state,
            self.viewport_size[0],
            self.viewport_size[1],
            self.pointer_position,
            link_active,
        );
        vm.proportional_circle_commands = compute_proportional_circle(
            &self.state,
            self.viewport_size[0],
            self.viewport_size[1],
            link_active,
        );
        if let Some(anchor) = self.slice_anchor {
            let dx = self.pointer_position[0] - anchor[0];
            let dy = self.pointer_position[1] - anchor[1];
            let len = dx.hypot(dy);
            if len > 2.0 {
                let dir_x = dx / len;
                let dir_y = dy / len;
                let p1 = [anchor[0] - dir_x * 2000.0, anchor[1] - dir_y * 2000.0];
                let p2 = [anchor[0] + dir_x * 2000.0, anchor[1] + dir_y * 2000.0];
                vm.drag_link_commands =
                    format!("M {:.2} {:.2} L {:.2} {:.2} ", p1[0], p1[1], p2[0], p2[1]);
            }
        }
        vm.hover_label = self.state.session.tools.hover.label();
        self.fill_operation_hud(vm);

        if vm.operation_hud_active {
            vm.hud_pill_visible = true;
            vm.hud_pill_title = vm.operation_hud_title.clone();
            vm.hud_pill_badge = if self.is_instant_tool_mode() {
                self.state
                    .t_id(petunia_config::text_id::HUD_BADGE_FREE_MODE)
            } else if !vm.operation_hud_subject.is_empty() {
                vm.operation_hud_subject.clone()
            } else {
                self.state.t_id(petunia_config::text_id::HUD_BADGE_TOOL)
            };
            vm.hud_pill_value = vm.operation_hud_lines.join("   ");
            vm.hud_pill_hint = vm.operation_hud_hint.clone();
            if self.pointer_position[0] > 0.0 && self.pointer_position[1] > 0.0 {
                vm.hud_pill_x = (self.pointer_position[0] + 16.0)
                    .clamp(16.0, (self.viewport_size[0] - 260.0).max(16.0));
                vm.hud_pill_y = (self.pointer_position[1] + 16.0)
                    .clamp(16.0, (self.viewport_size[1] - 90.0).max(16.0));
            } else {
                vm.hud_pill_x = 24.0;
                vm.hud_pill_y = 64.0;
            }
        }

        let axis_guide =
            compute_axis_guide(&self.state, self.viewport_size[0], self.viewport_size[1]);
        vm.axis_guide_visible = axis_guide.visible;
        vm.axis_guide_commands = axis_guide.commands;
        vm.axis_guide_color = axis_guide.color;
        vm.axis_guide_label = axis_guide.label;
        vm.axis_guide_label_x = axis_guide.label_x;
        vm.axis_guide_label_y = axis_guide.label_y;

        vm.world_axis_labels =
            compute_world_axis_labels(&self.state, self.viewport_size[0], self.viewport_size[1]);

        let dimension =
            compute_dimension_annotation(&self.state, self.viewport_size[0], self.viewport_size[1]);
        vm.dimension_visible = dimension.visible;
        vm.dimension_commands = dimension.commands;
        vm.dimension_text = dimension.text;
        vm.dimension_x = dimension.label_x;
        vm.dimension_y = dimension.label_y;

        let measure =
            compute_quick_measure(&self.state, self.viewport_size[0], self.viewport_size[1]);
        vm.measure_visible = measure.visible;
        vm.measure_commands = measure.commands;
        vm.measure_text = measure.text;
        vm.measure_x = measure.label_x;
        vm.measure_y = measure.label_y;
        vm.measure_distance = measure.distance;
        vm.measure_dx = measure.dx;
        vm.measure_dy = measure.dy;
        vm.measure_dz = measure.dz;
        vm.measure_angle_deg = measure.angle_deg;
        vm.measure_hud_text = measure.hud_text;
        vm.measure_tags = measure.tags;

        vm.micro_inspector_open = self.micro_inspector_open;
        vm.micro_inspector_x = self.micro_inspector_pos[0];
        vm.micro_inspector_y = self.micro_inspector_pos[1];

        let snap_marker = self.snap_marker_model(self.viewport_size[0], self.viewport_size[1]);
        vm.snap_marker_visible = snap_marker.visible;
        vm.snap_marker_x = snap_marker.x;
        vm.snap_marker_y = snap_marker.y;
        vm.snap_marker_label = snap_marker.label;
        vm.snap_marker_round = snap_marker.round;

        let protractor = compute_protractor(
            &self.state,
            self.viewport_size[0],
            self.viewport_size[1],
            self.drag.as_ref(),
        );
        vm.protractor_visible = protractor.visible;
        vm.protractor_wedge_commands = protractor.wedge_commands;
        vm.protractor_ticks_commands = protractor.ticks_commands;

        if let Some(menu) = self.context_menu {
            vm.context_menu_open = true;
            vm.context_menu_x = menu.x;
            vm.context_menu_y = menu.y;
            if menu.viewport {
                vm.context_menu_mode = "viewport".to_string();
                vm.context_menu_title = self.state.t_id(petunia_config::text_id::HUD_VIEWPORT_MENU);
            } else if let Some(asset) = self
                .state
                .project
                .assets
                .iter()
                .find(|asset| asset.id == menu.asset)
            {
                vm.context_menu_title = asset.name.clone();
                vm.context_menu_visible = asset.visible;
                vm.context_menu_locked = asset.locked;
                vm.context_menu_isolated = self.state.session.isolate_active;
                if let Some(idx) = self.state.project.find(menu.asset) {
                    vm.context_menu_can_move_up = idx > 0;
                    vm.context_menu_can_move_down = idx + 1 < self.state.project.assets.len();
                }
            }
        }
        if let Some(operand) = self.state.session.tools.boolean_operand {
            if let Some(asset) = self
                .state
                .project
                .assets
                .iter()
                .find(|asset| asset.id == operand)
            {
                vm.boolean_operand_name = asset.name.clone();
            } else {
                vm.boolean_operand_name = "missing".to_string();
            }
        }
        vm.boolean_keep_parts = self.state.session.tools.boolean_keep_parts;
        vm.boolean_cleanup = self.state.session.tools.boolean_cleanup;
        vm.boolean_ready = self.state.session.tools.boolean_operand.is_some()
            && self.state.project.active_mesh().is_some();
        if let Some(kind) = self.menu_open {
            vm.menu_open = kind.id().to_string();
        }
        vm.pivot_menu_open = self.pivot_menu_open;
    }

    /// Rótulos e dicas traduzidos.
    fn project_labels(&self, vm: &mut ShellViewModel) {
        let translated = |id: petunia_config::TextId| self.state.t_id(id);
        vm.label_parts = translated(petunia_config::text_id::UI_PARTS);
        vm.label_project_asset_library =
            translated(petunia_config::text_id::UI_PROJECT_ASSET_LIBRARY);
        vm.label_save_active_as_asset =
            translated(petunia_config::text_id::UI_SAVE_ACTIVE_AS_ASSET);
        vm.label_status_hint = translated(petunia_config::text_id::UI_STATUS_HINT);
        vm.label_unwrap_mesh = translated(petunia_config::text_id::UI_UNWRAP_MESH);
        vm.label_pack_islands = translated(petunia_config::text_id::UI_PACK_ISLANDS);
        vm.label_active_brush_color = translated(petunia_config::text_id::UI_ACTIVE_BRUSH_COLOR);
        vm.label_albedo_base_color = translated(petunia_config::text_id::UI_ALBEDO_BASE_COLOR);
        vm.label_theme = translated(petunia_config::text_id::UI_THEME);
        vm.label_place_in_scene = translated(petunia_config::text_id::UI_PLACE_IN_SCENE);
        vm.label_vertical_tool_drag = translated(petunia_config::text_id::UI_VERTICAL_TOOL_DRAG);
        vm.label_invert_vertical_drag =
            translated(petunia_config::text_id::UI_INVERT_VERTICAL_DRAG);
        vm.label_search_assets = translated(petunia_config::text_id::UI_SEARCH_ASSETS);
        vm.label_search_parts = translated(petunia_config::text_id::UI_SEARCH_PARTS);
        vm.label_inspector = translated(petunia_config::text_id::UI_INSPECTOR);
        vm.label_expand_inspector = translated(petunia_config::text_id::UI_EXPAND_INSPECTOR);
        vm.label_collapse_inspector = translated(petunia_config::text_id::UI_COLLAPSE_INSPECTOR);
        vm.label_resize_panel_width = translated(petunia_config::text_id::UI_RESIZE_PANEL_WIDTH);
        vm.label_section_dock = translated(petunia_config::text_id::UI_SECTION_DOCK);
        vm.label_section_drag = translated(petunia_config::text_id::UI_SECTION_DRAG);
        vm.label_section_pin_open = translated(petunia_config::text_id::UI_SECTION_PIN_OPEN);
        vm.label_section_pin_asset = translated(petunia_config::text_id::UI_SECTION_PIN_ASSET);
        vm.label_section_unpin_asset = translated(petunia_config::text_id::UI_SECTION_UNPIN_ASSET);
        vm.label_object_name = translated(petunia_config::text_id::UI_OBJECT_NAME);
        vm.label_object_visibility = translated(petunia_config::text_id::UI_OBJECT_VISIBILITY);
        vm.label_object_lock = translated(petunia_config::text_id::UI_OBJECT_LOCK);
        vm.label_object_no_selection = translated(petunia_config::text_id::UI_OBJECT_NO_SELECTION);
        vm.label_stats_faces = translated(petunia_config::text_id::UI_STATS_FACES);
        vm.label_stats_verts = translated(petunia_config::text_id::UI_STATS_VERTS);
        vm.label_stats_tris = translated(petunia_config::text_id::UI_STATS_TRIS);
        vm.label_stats_selection = translated(petunia_config::text_id::UI_STATS_SELECTION);
        vm.label_tool_options = translated(petunia_config::text_id::UI_TOOL_OPTIONS);
        vm.label_tool_options_expand = translated(petunia_config::text_id::UI_TOOL_OPTIONS_EXPAND);
        vm.label_tool_options_collapse =
            translated(petunia_config::text_id::UI_TOOL_OPTIONS_COLLAPSE);
        vm.label_quick_actions = translated(petunia_config::text_id::UI_QUICK_ACTIONS);
        vm.label_quick_action_customize =
            translated(petunia_config::text_id::UI_QUICK_ACTION_CUSTOMIZE);
        vm.label_quick_action_add = translated(petunia_config::text_id::UI_QUICK_ACTION_ADD);
        vm.label_quick_action_remove = translated(petunia_config::text_id::UI_QUICK_ACTION_REMOVE);
        vm.label_quick_action_reset = translated(petunia_config::text_id::UI_QUICK_ACTION_RESET);
        vm.label_quick_action_done = translated(petunia_config::text_id::UI_QUICK_ACTION_DONE);
        vm.label_action_subdivide = translated(petunia_config::text_id::UI_ACTION_SUBDIVIDE);
        vm.label_action_fuse = translated(petunia_config::text_id::UI_ACTION_FUSE);
        vm.label_action_cut = translated(petunia_config::text_id::UI_ACTION_CUT);
        vm.label_action_intersect = translated(petunia_config::text_id::UI_ACTION_INTERSECT);
        vm.label_action_join = translated(petunia_config::text_id::UI_ACTION_JOIN);
        vm.label_action_merge = translated(petunia_config::text_id::UI_ACTION_MERGE);
        vm.label_action_slice = translated(petunia_config::text_id::UI_ACTION_SLICE);
        vm.label_action_loop_cut = translated(petunia_config::text_id::UI_ACTION_LOOP_CUT);
        vm.label_material_base_color = translated(petunia_config::text_id::UI_MATERIAL_BASE_COLOR);
        vm.label_material_profile = translated(petunia_config::text_id::UI_MATERIAL_PROFILE);
        vm.label_material_roughness = translated(petunia_config::text_id::UI_MATERIAL_ROUGHNESS);
        vm.label_material_metallic = translated(petunia_config::text_id::UI_MATERIAL_METALLIC);
        vm.label_material_normal_scale =
            translated(petunia_config::text_id::UI_MATERIAL_NORMAL_SCALE);
        vm.label_material_advanced = translated(petunia_config::text_id::UI_MATERIAL_ADVANCED);
        vm.label_material_assign = translated(petunia_config::text_id::UI_MATERIAL_ASSIGN);
        vm.label_material_new = translated(petunia_config::text_id::UI_MATERIAL_NEW);
        vm.label_material_duplicate = translated(petunia_config::text_id::UI_MATERIAL_DUPLICATE);
        vm.label_material_remove = translated(petunia_config::text_id::UI_MATERIAL_REMOVE);
        vm.label_material_no_material =
            translated(petunia_config::text_id::UI_MATERIAL_NO_MATERIAL);
        vm.label_material_no_selection =
            translated(petunia_config::text_id::UI_MATERIAL_NO_SELECTION);
        vm.label_material_emission_strength =
            translated(petunia_config::text_id::UI_MATERIAL_EMISSION_STRENGTH);
        vm.label_material_alpha_cutoff =
            translated(petunia_config::text_id::UI_MATERIAL_ALPHA_CUTOFF);
        vm.label_material_texture_albedo =
            translated(petunia_config::text_id::UI_MATERIAL_TEXTURE_ALBEDO);
        vm.label_material_no_texture = translated(petunia_config::text_id::UI_MATERIAL_NO_TEXTURE);
        vm.label_material_create_texture =
            translated(petunia_config::text_id::UI_MATERIAL_CREATE_TEXTURE);
        vm.label_material_clear_texture =
            translated(petunia_config::text_id::UI_MATERIAL_CLEAR_TEXTURE);
        vm.label_material_profile_pbr =
            translated(petunia_config::text_id::UI_MATERIAL_PROFILE_PBR);
        vm.label_material_profile_unlit =
            translated(petunia_config::text_id::UI_MATERIAL_PROFILE_UNLIT);
        vm.label_material_profile_toon =
            translated(petunia_config::text_id::UI_MATERIAL_PROFILE_TOON);
        vm.label_material_profile_glass =
            translated(petunia_config::text_id::UI_MATERIAL_PROFILE_GLASS);
        vm.label_material_profile_emissive =
            translated(petunia_config::text_id::UI_MATERIAL_PROFILE_EMISSIVE);
        vm.label_material_alpha_opaque =
            translated(petunia_config::text_id::UI_MATERIAL_ALPHA_OPAQUE);
        vm.label_material_alpha_mask = translated(petunia_config::text_id::UI_MATERIAL_ALPHA_MASK);
        vm.label_material_alpha_blend =
            translated(petunia_config::text_id::UI_MATERIAL_ALPHA_BLEND);
        vm.label_modifier_mirror = translated(petunia_config::text_id::UI_MODIFIER_MIRROR);
        vm.label_modifier_symmetry = translated(petunia_config::text_id::UI_MODIFIER_SYMMETRY);
        vm.label_modifiers_empty = translated(petunia_config::text_id::UI_MODIFIERS_EMPTY);
        vm.label_modifier_apply = translated(petunia_config::text_id::UI_MODIFIER_APPLY);
        vm.label_modifier_axis = translated(petunia_config::text_id::UI_MODIFIER_AXIS);
        vm.label_modifier_add_mirror = translated(petunia_config::text_id::UI_MODIFIER_ADD_MIRROR);
        vm.label_modifier_add_symmetry =
            translated(petunia_config::text_id::UI_MODIFIER_ADD_SYMMETRY);
        vm.label_modifier_remove = translated(petunia_config::text_id::UI_MODIFIER_REMOVE);
        vm.label_modifier_move_up = translated(petunia_config::text_id::UI_MODIFIER_MOVE_UP);
        vm.label_modifier_move_down = translated(petunia_config::text_id::UI_MODIFIER_MOVE_DOWN);
        vm.label_modifier_direction = translated(petunia_config::text_id::UI_MODIFIER_DIRECTION);
        vm.label_modifier_positive_to_negative =
            translated(petunia_config::text_id::UI_MODIFIER_POSITIVE_TO_NEGATIVE);
        vm.label_modifier_negative_to_positive =
            translated(petunia_config::text_id::UI_MODIFIER_NEGATIVE_TO_POSITIVE);
        vm.label_tab_parts = translated(petunia_config::text_id::UI_TAB_PARTS);
        vm.label_tab_transform = translated(petunia_config::text_id::UI_TAB_TRANSFORM);
        vm.label_tab_material = translated(petunia_config::text_id::UI_TAB_MATERIAL);
        vm.label_tab_modifiers = translated(petunia_config::text_id::UI_TAB_MODIFIERS);
        vm.material_slots = self
            .state
            .project
            .project
            .materials
            .iter()
            .map(|m| m.name.clone())
            .collect();
        vm.active_material_slot = self
            .active_material_slot
            .clamp(0, vm.material_slots.len().saturating_sub(1) as i32);
        vm.label_tab_object = translated(petunia_config::text_id::UI_TAB_OBJECT);
        vm.label_numeric_field_hint = translated(petunia_config::text_id::UI_NUMERIC_FIELD_HINT);
        vm.label_model_select = translated(petunia_config::text_id::TOOLS_SELECT);
        vm.label_model_position = translated(petunia_config::text_id::TRANSFORM_POSITION);
        vm.label_model_rotate = translated(petunia_config::text_id::TOOLS_ROTATE);
        vm.label_model_scale = translated(petunia_config::text_id::TOOLS_SCALE);
        vm.label_model_transform = translated(petunia_config::text_id::TOOLS_TRANSFORM);
        vm.label_model_lasso = translated(petunia_config::text_id::TOOLS_SELECT_LASSO);
        vm.hint_model_select = translated(petunia_config::text_id::UI_MODEL_SELECT_HINT);
        vm.hint_model_position = translated(petunia_config::text_id::UI_MODEL_POSITION_HINT);
        vm.hint_model_rotate = translated(petunia_config::text_id::UI_MODEL_ROTATE_HINT);
        vm.hint_model_scale = translated(petunia_config::text_id::UI_MODEL_SCALE_HINT);
        vm.hint_model_transform = translated(petunia_config::text_id::UI_MODEL_TRANSFORM_HINT);
        vm.hint_model_lasso = translated(petunia_config::text_id::UI_MODEL_LASSO_HINT);
        vm.label_model_loop_cut = translated(petunia_config::text_id::TOOLS_LOOP_CUT);
        vm.hint_model_loop_cut = translated(petunia_config::text_id::UI_LOOP_CUT_HINT);
        vm.label_model_slice = translated(petunia_config::text_id::TOOLS_SLICE);
        vm.hint_model_slice = translated(petunia_config::text_id::UI_SLICE_HINT);
        vm.label_model_push_pull = translated(petunia_config::text_id::TOOLS_PUSH_PULL);
        vm.label_poly_pen = translated(petunia_config::text_id::TOOLS_POLY_PEN);
        vm.label_poly_pen_hint = translated(petunia_config::text_id::TOOLS_POLY_PEN_HINT);
        vm.poly_pen_mode = self.poly_pen_mode.id().to_string();
        vm.label_poly_pen_mode = translated(petunia_config::text_id::TOOLS_POLY_PEN_MODE);
        vm.label_poly_pen_mode_auto = translated(petunia_config::text_id::TOOLS_POLY_PEN_MODE_AUTO);
        vm.label_poly_pen_mode_points =
            translated(petunia_config::text_id::TOOLS_POLY_PEN_MODE_POINTS);
        vm.label_poly_pen_mode_edges =
            translated(petunia_config::text_id::TOOLS_POLY_PEN_MODE_EDGES);
        vm.label_poly_pen_mode_polygons =
            translated(petunia_config::text_id::TOOLS_POLY_PEN_MODE_POLYGONS);
        let alternate = self
            .state
            .ui
            .keybinds
            .pointer_modifier(petunia_config::keybinds::POINTER_ALTERNATE)
            .to_modifier_string();
        vm.label_poly_pen_mode_hint = crate::tr::fill(
            &translated(self.poly_pen_mode.hint_id()),
            &[("alternate", alternate)],
        );
        vm.poly_pen_preview_commands = self.poly_pen_preview_commands();
        vm.hint_model_push_pull = translated(petunia_config::text_id::UI_PUSH_PULL_HINT);
        vm.label_model_profile = translated(petunia_config::text_id::TOOLS_DRAW_PROFILE);
        vm.hint_model_profile = translated(petunia_config::text_id::UI_PROFILE_HINT);
        vm.label_model_pivot = translated(petunia_config::text_id::TOOLS_PIVOT);
        vm.hint_model_pivot = translated(petunia_config::text_id::UI_PIVOT_HINT);
        vm.label_profile_depth = translated(petunia_config::text_id::UI_PROFILE_DEPTH);
        vm.label_profile_points = translated(petunia_config::text_id::UI_PROFILE_POINTS);
        vm.label_profile_close = translated(petunia_config::text_id::UI_PROFILE_CLOSE);
        vm.label_view_gizmo = translated(petunia_config::text_id::UI_VIEW_GIZMO);
        vm.label_view_gizmo_hint = translated(petunia_config::text_id::UI_VIEW_GIZMO_HINT);
        vm.label_profile_plane = translated(petunia_config::text_id::UI_PROFILE_PLANE);
        vm.label_profile_plane_auto = translated(petunia_config::text_id::UI_PROFILE_PLANE_AUTO);
        vm.label_profile_plane_ground =
            translated(petunia_config::text_id::UI_PROFILE_PLANE_GROUND);
        vm.label_profile_plane_face = translated(petunia_config::text_id::UI_PROFILE_PLANE_FACE);
        vm.label_profile_plane_view = translated(petunia_config::text_id::UI_PROFILE_PLANE_VIEW);
        vm.label_profile_look_at_plane =
            translated(petunia_config::text_id::UI_PROFILE_LOOK_AT_PLANE);
        vm.label_profile_generate = translated(petunia_config::text_id::UI_PROFILE_GENERATE);
        vm.label_profile_revolve = translated(petunia_config::text_id::UI_PROFILE_REVOLVE);
        vm.label_profile_sweep = translated(petunia_config::text_id::UI_PROFILE_SWEEP);
        vm.label_profile_cuts = translated(petunia_config::text_id::UI_PROFILE_CUTS);
        vm.label_profile_presets = translated(petunia_config::text_id::UI_PROFILE_PRESETS);
        vm.label_profile_add_rect = translated(petunia_config::text_id::UI_PROFILE_ADD_RECT);
        vm.label_profile_add_circle = translated(petunia_config::text_id::UI_PROFILE_ADD_CIRCLE);
        vm.label_profile_canvas_hint = translated(petunia_config::text_id::UI_PROFILE_CANVAS_HINT);
        vm.label_profile_wall_thickness =
            translated(petunia_config::text_id::UI_PROFILE_WALL_THICKNESS);
        vm.label_profile_smooth_curves =
            translated(petunia_config::text_id::UI_PROFILE_SMOOTH_CURVES);
        vm.label_profile_sharp_corners =
            translated(petunia_config::text_id::UI_PROFILE_SHARP_CORNERS);
        vm.label_profile_smoothness = translated(petunia_config::text_id::UI_PROFILE_SMOOTHNESS);
        vm.label_decal_transform = translated(petunia_config::text_id::UI_DECAL_TRANSFORM);
        vm.label_decal_position = translated(petunia_config::text_id::UI_DECAL_POSITION);
        vm.label_decal_scale = translated(petunia_config::text_id::UI_DECAL_SCALE);
        vm.label_decal_width = translated(petunia_config::text_id::UI_DECAL_WIDTH);
        vm.label_decal_rotation = translated(petunia_config::text_id::UI_DECAL_ROTATION);
        vm.label_decal_bake = translated(petunia_config::text_id::UI_DECAL_BAKE);
        vm.label_decal_hint = translated(petunia_config::text_id::UI_DECAL_HINT);
        vm.label_parametric_primitive =
            translated(petunia_config::text_id::UI_PRIMITIVE_PARAMETRIC);
        vm.label_freeze_primitive = translated(petunia_config::text_id::UI_PRIMITIVE_FREEZE);
        vm.label_freeze_primitive_hint =
            translated(petunia_config::text_id::UI_PRIMITIVE_FREEZE_HINT);
        vm.label_hide_part = translated(petunia_config::text_id::UI_HIDE_PART);
        vm.label_show_part = translated(petunia_config::text_id::UI_SHOW_PART);
        vm.label_lock_part = translated(petunia_config::text_id::UI_LOCK_PART);
        vm.label_unlock_part = translated(petunia_config::text_id::UI_UNLOCK_PART);
        vm.label_selected_parts_only = translated(petunia_config::text_id::UI_SELECTED_PARTS_ONLY);
        vm.label_sort_parts = translated(petunia_config::text_id::UI_SORT_PARTS);
        vm.label_parts_row_size = translated(petunia_config::text_id::UI_PARTS_ROW_SIZE);
        vm.label_sort_assets = translated(petunia_config::text_id::UI_SORT_ASSETS);
        vm.label_thumbnail_size = translated(petunia_config::text_id::UI_THUMBNAIL_SIZE);
        vm.label_selection_color = translated(petunia_config::text_id::UI_SELECTION_COLOR);
        vm.label_highlight_thickness = translated(petunia_config::text_id::UI_HIGHLIGHT_THICKNESS);
        vm.label_view_wireframe = translated(petunia_config::text_id::UI_VIEW_WIREFRAME);
        vm.label_view_wireframe_hint = translated(petunia_config::text_id::UI_VIEW_WIREFRAME_HINT);
        vm.label_view_solid = translated(petunia_config::text_id::UI_VIEW_SOLID);
        vm.label_view_solid_hint = translated(petunia_config::text_id::UI_VIEW_SOLID_HINT);
        vm.label_view_material = translated(petunia_config::text_id::UI_VIEW_MATERIAL);
        vm.label_view_material_hint = translated(petunia_config::text_id::UI_VIEW_MATERIAL_HINT);
        vm.label_view_lit = translated(petunia_config::text_id::UI_VIEW_LIT);
        vm.label_view_lit_hint = translated(petunia_config::text_id::UI_VIEW_LIT_HINT);
        vm.label_more_model_tools = translated(petunia_config::text_id::UI_MORE_MODEL_TOOLS);
        vm.label_xray_opacity = translated(petunia_config::text_id::UI_XRAY_OPACITY);
        vm.label_wire_overlay = translated(petunia_config::text_id::UI_WIRE_OVERLAY);
        vm.label_wire_overlay_hint = translated(petunia_config::text_id::UI_WIRE_OVERLAY_HINT);
        if let Some(info) = &self.pending_recovery {
            vm.recovery_open = true;
            vm.recovery_title = translated(petunia_config::text_id::UI_RECOVERY_TITLE);
            vm.recovery_body = translated(petunia_config::text_id::UI_RECOVERY_BODY);
            vm.recovery_detail = format!(
                "{}  ·  snapshot {}  ·  {}",
                info.project_name,
                info.snapshot_time,
                info.snapshot_path.display()
            );
            vm.recovery_recover = translated(petunia_config::text_id::UI_RECOVERY_RECOVER);
            vm.recovery_keep = translated(petunia_config::text_id::UI_RECOVERY_KEEP);
            vm.recovery_discard = translated(petunia_config::text_id::UI_RECOVERY_DISCARD);
        }
        vm.label_asset_library = translated(petunia_config::text_id::UI_ASSETS);
        vm.label_preferences = translated(petunia_config::text_id::MENU_PREFERENCES);
        // A linha de rodapé das preferências informa o keymap e o idioma REAIS em uso.
        vm.shell_info = format!(
            "{}: {}  ·  {}: {}",
            translated(petunia_config::text_id::UI_THEME),
            self.state.ui.active_theme_id,
            "Keymap",
            self.state.ui.active_keymap_id,
        );
        vm.label_apply = translated(petunia_config::text_id::ACTIONS_APPLY);
        vm.label_cancel = translated(petunia_config::text_id::ACTIONS_CANCEL);
        vm.label_delete = translated(petunia_config::text_id::ACTIONS_DELETE);
        vm.label_duplicate = translated(petunia_config::text_id::ACTIONS_DUPLICATE);
        vm.themes = petunia_config::theme::ThemeRegistry::global()
            .available()
            .iter()
            .map(|manifest| ThemeEntryModel {
                id: manifest.id.clone(),
                name: manifest.name.clone(),
                active: manifest.id == self.state.ui.active_theme_id,
            })
            .collect();
    }

    /// Menus do Header, decals e canvas PAINT.
    fn project_menus_and_canvas(&self, vm: &mut ShellViewModel) {
        let translated = |id: petunia_config::TextId| self.state.t_id(id);
        for kind in MenuKind::ALL {
            let entries: Vec<MenuEntryModel> = kind
                .items()
                .iter()
                .map(|(id, label, shortcut)| MenuEntryModel {
                    id: (*id).to_string(),
                    label: if *id == MENU_SEPARATOR {
                        String::new()
                    } else {
                        translated(*label)
                    },
                    shortcut: (*shortcut).to_string(),
                    separator: *id == MENU_SEPARATOR,
                    disabled: match *id {
                        "edit.undo" => !vm.can_undo,
                        "edit.redo" => !vm.can_redo,
                        _ => false,
                    },
                    checked: match *id {
                        // O item alterna o MODO de shading Wireframe (não o
                        // overlay): o check reflete o shading ativo.
                        "view.toggle_wireframe" => vm.shading_mode == "wireframe",
                        "view.toggle_projection" => vm.is_orthographic,
                        "view.toggle_split" => self.split.enabled,
                        "view.reference_images" => self.reference_manager_open,
                        _ => false,
                    },
                })
                .collect();
            match kind {
                MenuKind::File => {
                    vm.menu_file_label = translated(kind.title());
                    vm.menu_file_items = entries;
                }
                MenuKind::Edit => {
                    vm.menu_edit_label = translated(kind.title());
                    vm.menu_edit_items = entries;
                }
                MenuKind::View => {
                    vm.menu_view_label = translated(kind.title());
                    vm.menu_view_items = entries;
                }
                MenuKind::Window => {
                    vm.menu_window_label = translated(kind.title());
                    vm.menu_window_items = entries;
                }
            }
        }
        if let Some(effect) = self
            .state
            .project
            .assets
            .get(self.state.project.active)
            .and_then(|asset| asset.paint_stack.as_ref())
            .and_then(|stack| stack.active())
            .and_then(|layer| match &layer.kind {
                petunia_project::paint_layers::LayerKind::Effect(effect) => Some(*effect),
                _ => None,
            })
        {
            use petunia_project::paint_layers::PaintEffect;
            vm.paint_effect_kind = match &effect {
                PaintEffect::Pixelate { .. } => "Pixelate",
                PaintEffect::Posterize { .. } => "Posterize",
                PaintEffect::Invert => "Invert",
                PaintEffect::Grain { .. } => "Grain",
                PaintEffect::Levels { .. } => "Levels",
                PaintEffect::BrightnessContrast { .. } => "BrightnessContrast",
                PaintEffect::HueSaturation { .. } => "HueSaturation",
            }
            .to_string();
            vm.paint_effect_params = effect_params(&effect);
        }
        if let Some(decal) = self
            .state
            .project
            .assets
            .get(self.state.project.active)
            .and_then(|asset| asset.paint_stack.as_ref())
            .and_then(|stack| stack.active())
            .and_then(|layer| match &layer.kind {
                petunia_project::paint_layers::LayerKind::Decal(decal) => Some(decal.clone()),
                _ => None,
            })
        {
            // Populates decal transformation fields (P3D-133)
            // Preenche campos de transformação do decalque (P3D-133)
            vm.active_layer_is_decal = true;
            vm.decal_center_u = decal.center_uv[0];
            vm.decal_center_v = decal.center_uv[1];
            vm.decal_scale_u = decal.scale_uv[0];
            vm.decal_scale_v = decal.scale_uv[1];
            if let Some(anchor) = decal.anchor {
                vm.decal_surface = true;
                vm.decal_scale_u = anchor.width;
                vm.decal_scale_v = decal.surface_height(anchor.width);
            }
            vm.decal_rotation_deg = decal.rotation_rad.to_degrees();
            vm.decal_preview_commands = self.decal_preview_commands();
            let base = self
                .state
                .project
                .active()
                .and_then(|asset| asset.paint_stack.as_ref())
                .and_then(|stack| stack.active())
                .map_or_else(String::new, |layer| layer.name.clone());
            vm.decal_variant_names = (0..decal.variant_count())
                .map(|index| decal.variant_name(index, &base).to_owned())
                .collect();
            vm.decal_variant_index = decal.variant_index as i32;
            let track = decal.track.clone().unwrap_or_default();
            vm.decal_frame = self.decal_frame as i32;
            vm.decal_track_length = track.length_frames as i32;
            vm.decal_track_fps = track.fps;
            vm.decal_has_keys = !track.keys.is_empty();
            vm.decal_frame_keyed = track.keys.iter().any(|key| key.frame == self.decal_frame);
            vm.decal_keys_text = track
                .keys
                .iter()
                .map(|key| {
                    format!(
                        "{}: {}",
                        key.frame,
                        decal.variant_name(key.variant as usize, &base)
                    )
                })
                .collect::<Vec<_>>()
                .join(" · ");
            vm.decal_playing = self.decal_playing;
        }
        if let Some((width, height)) = self.paint_canvas_dimensions() {
            vm.paint_canvas_size = format!("{width} × {height}");
            vm.paint_texture_side = if width == height { width as i32 } else { 0 };
            vm.paint_canvas_revision = self.state.project.assets[self.state.project.active]
                .paint_stack
                .as_ref()
                .map(|stack| {
                    stack
                        .layers
                        .iter()
                        .filter_map(|layer| layer.canvas())
                        .map(|canvas| canvas.w as i32 * canvas.h as i32)
                        .sum::<i32>()
                        + stack.layers.len() as i32
                })
                .unwrap_or(0);
        }
    }

    /// Opções de PAINT e editor UV.
    fn project_paint_uv(&self, vm: &mut ShellViewModel) {
        vm.paint_fill_scope = format!("{:?}", self.state.session.tools.fill_scope);
        vm.paint_projection = format!("{:?}", self.state.session.tools.brush_projection);
        vm.paint_lock = format!("{:?}", self.state.session.tools.brush_lock);
        vm.paint_pixel_grid = self.paint_pixel_grid;
        vm.paint_canvas_zoom = self.paint_canvas_zoom;
        vm.paint_canvas_grid_commands = self.paint_canvas_grid_commands();
        let path_preview_active = self.state.workspace == Workspace::Paint
            && self.state.session.tools.active_tool == "path_paint";
        vm.paint_show_uv_overlay = self.paint_show_uv_overlay || path_preview_active;
        vm.uv_show_texture = self.uv_show_texture;
        vm.uv_editor = if cfg!(test)
            || self.state.workspace == Workspace::Uv
            || (self.state.workspace == Workspace::Paint
                && (self.paint_show_uv_overlay || path_preview_active))
        {
            self.build_uv_editor()
        } else {
            UvEditorModel::default()
        };
        if path_preview_active {
            vm.uv_editor
                .layout_commands
                .push_str(&self.surface_path_uv_preview());
        }
        if let Some(stack) = self
            .state
            .project
            .assets
            .get(self.state.project.active)
            .and_then(|asset| asset.paint_stack.as_ref())
        {
            vm.paint_layers = stack
                .layers
                .iter()
                .enumerate()
                .map(|(index, layer)| PaintLayerModel {
                    id: layer.id.to_string(),
                    name: layer.name.clone(),
                    visible: layer.visible,
                    locked: layer.locked,
                    opacity: layer.opacity,
                    active: index == stack.active_layer,
                    is_group: layer.is_group,
                    kind_label: match layer.kind {
                        petunia_project::paint_layers::LayerKind::Raster(_) => "Raster",
                        petunia_project::paint_layers::LayerKind::Decal(_) => "Decal",
                        petunia_project::paint_layers::LayerKind::Effect(_) => "Effect",
                    }
                    .to_string(),
                })
                .collect();
            let (width, height) = self
                .state
                .project
                .assets
                .get(self.state.project.active)
                .and_then(|asset| asset.paint_stack.as_ref())
                .and_then(|stack| stack.active().and_then(|layer| layer.canvas()))
                .map(|canvas| (canvas.w, canvas.h))
                .unwrap_or((0, 0));
            vm.paint_layer_count =
                format!("{} layer(s)  ·  {width} × {height}", stack.layers.len());
        }
        if let Some(session) = &self.loop_cut {
            vm.loop_cut_active = true;
            vm.loop_cut_slide = session.slide;
            vm.loop_cut_cuts = session.cuts as i32;
            vm.loop_cut_balanced = session.balanced;
            let preview_res = if session.balanced {
                session
                    .ring
                    .preview_balanced(&session.source, session.cuts, session.slide)
            } else {
                session
                    .ring
                    .preview(&session.source, session.cuts, session.slide)
            };
            if let Ok(segments) = preview_res {
                let mut commands = String::new();
                for [a, b] in segments {
                    project_preview_segment(
                        &self.state.session.camera,
                        self.viewport_size,
                        a,
                        b,
                        &mut commands,
                    );
                }
                vm.loop_cut_preview_commands = commands;
            }
        } else if self.state.session.tools.active_tool == "loop_cut" {
            vm.loop_cut_armed = true;
            vm.loop_cut_cuts = self.loop_cut_hover_cuts as i32;
            vm.loop_cut_balanced = self.loop_cut_balanced;
            vm.loop_cut_preview_commands = self.loop_cut_hover_preview_commands();
        }
    }

    /// Perfil DRAW, POLY, Loop Cut e ferramentas de modelagem.
    fn project_modeling(&self, vm: &mut ShellViewModel) {
        vm.profile_active = self.state.session.workspace == Workspace::Model
            && self.modeling_mode == ModelingMode::Draw
            && (self.state.session.tools.active_tool == "draw_profile"
                || self.active_profile_id.is_some());
        vm.profile_point_count = self.active_profile_point_count() as i32;
        vm.profile_closed = self.active_profile_closed();
        if let Some((profile, spline)) = self.active_profile_resources()
            && let Some((_, x, y)) = profile.primitive_frame(spline)
        {
            vm.profile_parametric = true;
            vm.profile_ellipse =
                profile.primitive == Some(petunia_project::profile::ProfilePrimitive::Ellipse);
            vm.profile_width = (x.length() * 2.0) as f32;
            vm.profile_height = (y.length() * 2.0) as f32;
            vm.profile_segments = spline.points.len() as i32;
        }
        vm.profile_depth = self.state.profile.depth;
        vm.profile_wall_thickness = self
            .active_profile_resources()
            .map_or(self.state.profile.wall_thickness, |(profile, _)| {
                profile.wall_thickness as f32
            });
        vm.profile_smoothness = self.state.profile.curve_smoothness;
        vm.profile_has_curves = self.active_profile_has_curves();
        if vm.profile_active {
            vm.profile_preview_commands = self.profile_preview_commands();
        }
        vm.profile_workplane =
            petunia_module_model::profile_workplane_label(&self.state).to_string();
        vm.profile_workplane_locked = self.state.profile.workplane_locked;
        vm.region_hover_commands = self.region_hover_commands();
        vm.profile_selection_commands = self.profile_selection_commands();
        vm.region_shapes_commands = self.region_shapes_commands();
        vm.keymap_capture_action = self.keymap_editor.capturing.clone().unwrap_or_default();
        vm.brush_panel = self.brush_panel_model();
        if self.settings_visible {
            let (revision, snapshot) = self.keymap_snapshot();
            vm.keymap_revision = i64::from(revision);
            vm.keymap_snapshot = snapshot;
        }
        vm.profile_outline_commands = self.profile_outline_commands();
        vm.modeling_mode = self.modeling_mode.id().to_string();
        vm.label_workspace_draw_title = self
            .state
            .t_id(petunia_config::text_id::WORKSPACE_DRAW_TITLE);
        vm.label_workspace_draw_description = self
            .state
            .t_id(petunia_config::text_id::WORKSPACE_DRAW_DESCRIPTION);
        vm.label_workspace_poly_title = self
            .state
            .t_id(petunia_config::text_id::WORKSPACE_POLY_TITLE);
        vm.label_workspace_poly_description = self
            .state
            .t_id(petunia_config::text_id::WORKSPACE_POLY_DESCRIPTION);
        vm.profile_volume_mode = match self.profile_volume_mode {
            Some(petunia_module_model::ProfileVolumeMode::Extrude) => "extrude".to_string(),
            Some(petunia_module_model::ProfileVolumeMode::Revolve) => "revolve".to_string(),
            Some(petunia_module_model::ProfileVolumeMode::Sweep) => "sweep".to_string(),
            None => "none".to_string(),
        };
        vm.profile_revolve_angle = if self.state.profile.revolve_angle <= 0.0 {
            360.0
        } else {
            self.state.profile.revolve_angle
        };
        vm.slice_trim = self.slice_trim;
        vm.bevel_clamp_overlap = self.state.tools.bevel_clamp_overlap;
        vm.bevel_affect_vertices = self.state.tools.bevel_affect_vertices;
        if let Some(anchor) = self.slice_anchor {
            let mut cmd = format!(
                "M {:.2} {:.2} L {:.2} {:.2}",
                anchor[0], anchor[1], self.pointer_position[0], self.pointer_position[1]
            );
            let r = 5.0;
            cmd.push_str(&format!(
                " M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z",
                anchor[0] - r,
                anchor[1] - r,
                anchor[0] + r,
                anchor[1] - r,
                anchor[0] + r,
                anchor[1] + r,
                anchor[0] - r,
                anchor[1] + r,
            ));
            cmd.push_str(&format!(
                " M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z",
                self.pointer_position[0],
                self.pointer_position[1] - r,
                self.pointer_position[0] + r,
                self.pointer_position[1],
                self.pointer_position[0],
                self.pointer_position[1] + r,
                self.pointer_position[0] - r,
                self.pointer_position[1],
            ));
            if self.slice_trim {
                let dx = self.pointer_position[0] - anchor[0];
                let dy = self.pointer_position[1] - anchor[1];
                let len = dx.hypot(dy);
                if len > 4.0 {
                    let dir_x = dx / len;
                    let dir_y = dy / len;
                    let nx = dir_y;
                    let ny = -dir_x;
                    let mid_x = (anchor[0] + self.pointer_position[0]) * 0.5;
                    let mid_y = (anchor[1] + self.pointer_position[1]) * 0.5;
                    let arrow_x = mid_x + nx * 24.0;
                    let arrow_y = mid_y + ny * 24.0;
                    let w1_x = arrow_x - nx * 7.0 + dir_x * 5.0;
                    let w1_y = arrow_y - ny * 7.0 + dir_y * 5.0;
                    let w2_x = arrow_x - nx * 7.0 - dir_x * 5.0;
                    let w2_y = arrow_y - ny * 7.0 - dir_y * 5.0;
                    cmd.push_str(&format!(
                        " M {:.2} {:.2} L {:.2} {:.2} M {:.2} {:.2} L {:.2} {:.2} M {:.2} {:.2} L {:.2} {:.2}",
                        mid_x, mid_y, arrow_x, arrow_y,
                        arrow_x, arrow_y, w1_x, w1_y,
                        arrow_x, arrow_y, w2_x, w2_y
                    ));
                }
            }
            vm.operation_preview_commands = cmd.clone();
            vm.slice_preview_visible = true;
            vm.slice_preview_commands = cmd;
        }
        let parameters = self.draw_parameters;
        vm.draw_extension_values = [
            parameters.radius as f32,
            parameters.length as f32,
            parameters.angle as f32,
            parameters.sides as f32,
            parameters.tolerance as f32,
        ];
        vm.export_padding = parameters.export_padding as f32;
        vm.trace_threshold = parameters.trace_threshold as f32;
        if self.state.workspace == Workspace::Paint
            && let Some(mesh) = self.state.project.active_mesh()
        {
            let (w, h) = self.paint_canvas_dimensions().unwrap_or((256, 256));
            let islands = mesh.uv_islands();
            let tiny = islands
                .iter()
                .filter(|island| {
                    (island.max[0] - island.min[0]) * (w as f32) < 2.0
                        || (island.max[1] - island.min[1]) * (h as f32) < 2.0
                })
                .count();
            vm.uv_stats.push_str(
                &self
                    .state
                    .t_id(petunia_config::TextId::new("surface.uv_details"))
                    .replace("{density}", &format!("{:.1}", mesh.texel_density(w, false)))
                    .replace("{tiny}", &tiny.to_string())
                    .replace("{padding}", &parameters.export_padding.to_string()),
            );
        }
        vm.stencil_image = self.stencil_preview_image();
        let surface_preview = self.surface_paint_preview();
        if !surface_preview.is_empty() {
            vm.operation_preview_commands = surface_preview;
        }
    }

    /// Preferências, Home, tema e idioma.
    fn project_settings(&self, vm: &mut ShellViewModel) {
        let translated = |id: petunia_config::TextId| self.state.t_id(id);
        vm.tool_activation = self.state.session.tools.tool_activation.id().to_string();
        vm.keyboard_tool_modal_active =
            self.keyboard_tool_modal_active && self.tool_modal.is_some();
        vm.is_instant_tool_mode = self.is_instant_tool_mode();
        vm.invert_vertical_drag = self.state.ui.invert_vertical_drag;
        vm.colorblind_axes = self.state.ui.colorblind_axes;
        vm.reduced_motion = self.state.ui.reduced_motion;
        vm.multiselection_measure_tag = self.state.ui.multiselection_measure_tag;
        vm.double_tap_interval_ms = self.preferences.double_tap_interval_ms as i32;
        vm.tool_grammar_active = self.tool_grammar_active();
        vm.tool_gesture_latched = self.tool_session.is_latched();
        vm.drag_threshold_px = self.preferences.drag_threshold_px;
        vm.snap_radius_px = self.preferences.snap_radius_px;
        vm.label_snap_radius = translated(petunia_config::text_id::PREFERENCES_SNAP_RADIUS);
        vm.click_move_click = self.preferences.click_move_click;
        vm.show_tool_labels = self.preferences.show_tool_labels;
        vm.home_open = self.home_open;
        vm.show_home_on_start = self.preferences.show_home_on_start;
        vm.recent_projects = self
            .preferences
            .recent_projects
            .iter()
            .map(|path| {
                let name = std::path::Path::new(path)
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.clone());
                (name, path.clone())
            })
            .collect();
        {
            use petunia_config::text_id as T;
            vm.home_texts = [
                T::HOME_TITLE,
                T::HOME_SUBTITLE,
                T::HOME_NEW,
                T::HOME_OPEN,
                T::HOME_RECOVER,
                T::HOME_RECENT,
                T::HOME_NO_RECENT,
                T::HOME_SETTINGS,
                T::HOME_SHOW_ON_START,
            ]
            .map(translated);
        }
        vm.accent_rgb = self.preferences.accent_rgb;
        let [r, g, b] = self.effective_accent_rgb();
        vm.accent_color_hex = format!("#{r:02X}{g:02X}{b:02X}");
        vm.label_accent_color = translated(petunia_config::text_id::UI_ACCENT_COLOR);
        vm.label_accent_color_hint = translated(petunia_config::text_id::UI_ACCENT_COLOR_HINT);
        vm.label_show_tool_labels =
            translated(petunia_config::text_id::PREFERENCES_SHOW_TOOL_LABELS);
        vm.label_show_tool_labels_hint =
            translated(petunia_config::text_id::PREFERENCES_SHOW_TOOL_LABELS_HINT);
        vm.workplane_prefer_ground = self.preferences.workplane_prefer_ground;
        vm.studio_light_follows_camera = self.preferences.studio_light_follows_camera;
        vm.viewport_matcap = self.preferences.viewport_matcap;
        vm.viewport_ambient_occlusion = self.preferences.viewport_ambient_occlusion;
        vm.label_viewport_matcap = translated(petunia_config::text_id::PREFERENCES_VIEWPORT_MATCAP);
        vm.label_viewport_matcap_hint =
            translated(petunia_config::text_id::PREFERENCES_VIEWPORT_MATCAP_HINT);
        vm.label_viewport_ambient_occlusion =
            translated(petunia_config::text_id::PREFERENCES_VIEWPORT_AMBIENT_OCCLUSION);
        vm.label_viewport_ambient_occlusion_hint =
            translated(petunia_config::text_id::PREFERENCES_VIEWPORT_AMBIENT_OCCLUSION_HINT);
        vm.gizmo_gpu = self.viewport.draws_gizmo();
        vm.label_studio_light_follows_camera =
            translated(petunia_config::text_id::PREFERENCES_STUDIO_LIGHT_FOLLOWS_CAMERA);
        vm.label_studio_light_follows_camera_hint =
            translated(petunia_config::text_id::PREFERENCES_STUDIO_LIGHT_FOLLOWS_CAMERA_HINT);
        vm.label_workplane_prefer_ground =
            translated(petunia_config::text_id::PREFERENCES_WORKPLANE_PREFER_GROUND);
        vm.label_workplane_prefer_ground_hint =
            translated(petunia_config::text_id::PREFERENCES_WORKPLANE_PREFER_GROUND_HINT);
        vm.label_drag_threshold = translated(petunia_config::text_id::PREFERENCES_DRAG_THRESHOLD);
        vm.label_click_move_click =
            translated(petunia_config::text_id::PREFERENCES_CLICK_MOVE_CLICK);
        vm.label_click_move_click_hint =
            translated(petunia_config::text_id::PREFERENCES_CLICK_MOVE_CLICK_HINT);
        vm.label_last_operation = translated(petunia_config::text_id::TOOL_GRAMMAR_LAST_OPERATION);
        vm.label_last_operation_hint =
            translated(petunia_config::text_id::TOOL_GRAMMAR_ADJUST_HINT);
        if self.last_operation_adjustable()
            && let Some(last) = &self.last_operation
        {
            use petunia_core::ModalKind as Kind;
            vm.last_operation_active = true;
            vm.last_operation_title = match last.kind {
                Kind::Move => self.state.t("tools.move"),
                Kind::Rotate => self.state.t_id(petunia_config::text_id::TOOLS_ROTATE),
                Kind::Scale => self.state.t_id(petunia_config::text_id::TOOLS_SCALE),
                Kind::Extrude | Kind::ExtrudeIndividual => self.state.t("tools.extrude"),
                Kind::Inset => self.state.t("tools.inset"),
                Kind::Bevel => self.state.t("tools.bevel"),
                Kind::PushPull => self.state.t_id(petunia_config::text_id::TOOLS_PUSH_PULL),
            };
            vm.last_operation_value = last.primary_value();
            let (step, unit) = match last.kind {
                Kind::Rotate => (5.0, "°"),
                Kind::Scale => (0.1, "×"),
                Kind::Inset => (0.01, ""),
                Kind::Bevel => (0.01, "m"),
                Kind::Move | Kind::Extrude | Kind::ExtrudeIndividual | Kind::PushPull => (0.1, "m"),
            };
            vm.last_operation_step = step;
            vm.last_operation_unit = unit.to_string();
        } else if self.decal_last_operation_adjustable()
            && let Some(last) = &self.decal_last_operation
        {
            let (field, step, unit) = match last.param {
                DecalParam::Width => (petunia_config::text_id::UI_DECAL_WIDTH, 0.05, "m"),
                DecalParam::Rotation => (petunia_config::text_id::UI_DECAL_ROTATION, 5.0, "°"),
            };
            vm.last_operation_active = true;
            vm.last_operation_title = format!(
                "{} · {}",
                self.state.t_id(petunia_config::text_id::TOOLS_DECAL),
                self.state.t_id(field)
            );
            vm.last_operation_value = last.value;
            vm.last_operation_step = step;
            vm.last_operation_unit = unit.to_string();
        }
        vm.label_colorblind_axes = translated(petunia_config::text_id::PREFERENCES_COLORBLIND_AXES);
        vm.label_reduced_motion = translated(petunia_config::text_id::PREFERENCES_REDUCED_MOTION);
        vm.label_double_tap_interval =
            translated(petunia_config::text_id::PREFERENCES_DOUBLE_TAP_INTERVAL);
        vm.label_multiselection_measure_tag =
            translated(petunia_config::text_id::PREFERENCES_MULTISELECTION_MEASURE);
        vm.active_language = self.state.ui.i18n.lang.clone();
        vm.ui_scale = self.preferences.ui_scale;
        vm.icon_theme = self.preferences.icon_theme.clone();
        if let Some(kind) = self.tool_modal {
            let (minimum, maximum) = kind.bounds();
            vm.tool_modal_active = true;
            vm.tool_modal_id = kind.id().to_string();
            vm.tool_modal_title = self.state.t(kind.title_key());
            vm.tool_modal_label = self.state.t_id(kind.label_id());
            vm.tool_modal_value = self.tool_modal_value;
            vm.tool_modal_step = kind.step();
            vm.tool_modal_min = minimum;
            vm.tool_modal_max = maximum;
        }
    }

    /// Opções de ferramenta, materiais, modificadores e quick actions.
    fn project_tool_options(&self, vm: &mut ShellViewModel) {
        vm.tool_options_active = self.tool_modal.is_some()
            || self.loop_cut.is_some()
            || self.state.session.tools.active_tool == "loop_cut"
            || self.state.session.tools.active_tool == "draw_profile"
            || (self.state.workspace == Workspace::Model
                && !matches!(
                    self.state.session.tools.active_tool.as_str(),
                    "select" | "box_select" | "lasso_select"
                ));
        vm.tool_options_title = if let Some(kind) = self.tool_modal {
            self.state.t(kind.title_key())
        } else if self.loop_cut.is_some() || self.state.session.tools.active_tool == "loop_cut" {
            self.state.t_id(petunia_config::text_id::TOOLS_LOOP_CUT)
        } else if self.state.session.tools.active_tool == "draw_profile" {
            self.state.t_id(petunia_config::text_id::TOOLS_DRAW_PROFILE)
        } else {
            match self.state.session.tools.active_tool.as_str() {
                "scale" => self.state.t_id(petunia_config::text_id::TOOLS_SCALE),
                "rotate" => self.state.t_id(petunia_config::text_id::TOOLS_ROTATE),
                "move" => self.state.t_id(petunia_config::text_id::TOOLS_TRANSFORM),
                "slice" => self.state.t_id(petunia_config::text_id::TOOLS_SLICE),
                "knife" => self.state.t("tools.knife"),
                "extrude" => self.state.t("tools.extrude"),
                "extrude_individual" => self.state.t("tools.extrude_individual"),
                "inset" => self.state.t("tools.inset"),
                "bevel" => self.state.t("tools.bevel"),
                "push_pull" => self.state.t_id(petunia_config::text_id::TOOLS_PUSH_PULL),
                "poly_pen" => self.state.t_id(petunia_config::text_id::TOOLS_POLY_PEN),
                "cursor" | "cursor_3d" => "3D Cursor".to_string(),
                "measure" => self.state.t("tools.measure"),
                other => {
                    let key = format!("tools.{other}");
                    let val = self.state.t(&key);
                    if val != key {
                        val
                    } else if !other.is_empty() {
                        let mut chars = other.chars();
                        match chars.next() {
                            Some(first) => {
                                first.to_uppercase().collect::<String>() + chars.as_str()
                            }
                            None => "Tool Options".to_string(),
                        }
                    } else {
                        "Tool Options".to_string()
                    }
                }
            }
        };
        vm.tool_options_hint = if self.tool_modal.is_some() {
            self.state
                .t_id(petunia_config::text_id::UI_NUMERIC_FIELD_HINT)
        } else {
            self.state
                .t_id(petunia_config::text_id::UI_NO_TOOL_PARAMETERS)
        };
        if let Some(asset) = self.section_asset(petunia_config::InspectorSectionId::Object) {
            vm.object_has_selection = true;
            vm.object_id = asset.id.to_string();
            vm.object_name = asset.name.clone();
            vm.object_visible = asset.visible;
            vm.object_locked = asset.locked;
            vm.object_verts = asset.mesh.verts.len() as i32;
            vm.object_faces = asset.mesh.faces.len() as i32;
            vm.object_tris = asset.mesh.tri_count() as i32;
            vm.object_selection = vm.selection_summary.clone();
            vm.object_material = asset
                .material(&self.state.project.project)
                .map(|material| material.name.clone())
                .unwrap_or_else(|| {
                    self.state
                        .t_id(petunia_config::text_id::UI_MATERIAL_NO_MATERIAL)
                });
            vm.object_modifier_count = asset.modifiers.len() as i32;
        }
        // Display follows the pinned asset's material when the Material section
        // is pinned and resolvable; slot actions stay selection-contextual.
        // Exibição segue o material do asset fixado quando resolvível; ações de
        // slot seguem selection-context.
        let material_slot = self
            .section_asset(petunia_config::InspectorSectionId::Material)
            .and_then(|asset| asset.material_id)
            .and_then(|id| {
                self.state
                    .project
                    .project
                    .materials
                    .iter()
                    .position(|material| material.id == id)
            })
            .unwrap_or_else(|| vm.active_material_slot.max(0) as usize);
        if let Some(material) = self.state.project.project.materials.get(material_slot) {
            vm.material_has_selection = true;
            vm.material_id = material.id.to_string();
            vm.material_name = material.name.clone();
            vm.material_profile = match material.profile {
                petunia_project::ShaderProfile::Pbr => "pbr",
                petunia_project::ShaderProfile::Unlit => "unlit",
                petunia_project::ShaderProfile::Toon => "toon",
                petunia_project::ShaderProfile::Glass => "glass",
                petunia_project::ShaderProfile::Emissive => "emissive",
            }
            .to_string();
            vm.material_profile_label = material.profile.label().to_string();
            vm.material_base_color = [
                material.base_color[0],
                material.base_color[1],
                material.base_color[2],
            ];
            vm.material_roughness = material.roughness;
            vm.material_metallic = material.metallic;
            vm.material_normal_scale = material.normal_scale;
            vm.material_emission = material.emission_color;
            vm.material_emission_strength = material.emission_strength;
            vm.material_alpha_mode = match material.alpha_mode {
                petunia_project::AlphaMode::Opaque => "opaque",
                petunia_project::AlphaMode::Mask => "mask",
                petunia_project::AlphaMode::Blend => "blend",
            }
            .to_string();
            vm.material_alpha_cutoff = material.alpha_cutoff;
            vm.material_has_albedo = material.albedo_texture.is_some();
            vm.material_albedo_label = material
                .albedo_texture
                .as_ref()
                .map(|texture| format!("{} × {}", texture.w, texture.h))
                .unwrap_or_else(|| {
                    self.state
                        .t_id(petunia_config::text_id::UI_MATERIAL_NO_TEXTURE)
                });
        }
        vm.material_palette = self.state.project.palette.clone();
        let pinned_ids = self.state.ui.model_quick_action_ids();
        let quick_label = |id: &str| match id {
            "model.subdivide" => self
                .state
                .t_id(petunia_config::text_id::UI_ACTION_SUBDIVIDE),
            "model.fuse" => self.state.t_id(petunia_config::text_id::UI_ACTION_FUSE),
            "model.cut" => self.state.t_id(petunia_config::text_id::UI_ACTION_CUT),
            "model.intersect" => self
                .state
                .t_id(petunia_config::text_id::UI_ACTION_INTERSECT),
            "model.join" => self.state.t_id(petunia_config::text_id::UI_ACTION_JOIN),
            "model.merge" => self.state.t_id(petunia_config::text_id::UI_ACTION_MERGE),
            "model.slice" => self.state.t_id(petunia_config::text_id::UI_ACTION_SLICE),
            "model.loop_cut" => self.state.t_id(petunia_config::text_id::UI_ACTION_LOOP_CUT),
            _ => id.to_string(),
        };
        vm.quick_actions = pinned_ids
            .iter()
            .map(|id| QuickActionModel {
                id: id.clone(),
                label: quick_label(id),
                enabled: self.state.commands.can_execute(id, &self.state).is_ok()
                    || id == "model.slice",
                active: self.state.session.tools.active_tool == id.as_str(),
                pinned: true,
            })
            .collect();
        vm.quick_action_candidates = petunia_core::state::UiState::MODEL_QUICK_ACTION_CANDIDATES
            .iter()
            .map(|id| QuickActionModel {
                id: (*id).to_string(),
                label: quick_label(id),
                enabled: self.state.commands.can_execute(id, &self.state).is_ok()
                    || *id == "model.slice",
                active: self.state.session.tools.active_tool == *id,
                pinned: pinned_ids.iter().any(|pinned| pinned == id),
            })
            .collect();
        if let Some(asset) = self.section_asset(petunia_config::InspectorSectionId::Modifiers) {
            vm.modifier_rows = asset
                .modifiers
                .iter()
                .enumerate()
                .map(|(index, modifier)| {
                    let (kind, axis, positive_to_negative) = match modifier.kind {
                        petunia_project::ModifierKind::Mirror { axis, .. } => {
                            ("mirror", axis, true)
                        }
                        petunia_project::ModifierKind::Symmetry {
                            axis,
                            positive_to_negative,
                            ..
                        } => ("symmetry", axis, positive_to_negative),
                    };
                    let title = if kind == "mirror" {
                        self.state.t_id(petunia_config::text_id::UI_MODIFIER_MIRROR)
                    } else {
                        self.state
                            .t_id(petunia_config::text_id::UI_MODIFIER_SYMMETRY)
                    };
                    ModifierRowModel {
                        id: modifier.id.to_string(),
                        subtitle: format!(
                            "{} {}",
                            self.state.t_id(petunia_config::text_id::UI_MODIFIER_AXIS),
                            ["X", "Y", "Z"][axis.min(2)]
                        ),
                        title,
                        enabled: modifier.enabled,
                        kind: kind.to_string(),
                        axis: axis as i32,
                        positive_to_negative,
                        can_move_up: index > 0,
                        can_move_down: index + 1 < asset.modifiers.len(),
                    }
                })
                .collect();
        }
        vm.settings_visible = self.settings_visible;
        vm.command_search_visible = self.command_search_visible;
        vm.overlay_top_id = self
            .overlays
            .top()
            .map(|entry| entry.id.presentation_id().to_owned())
            .unwrap_or_default();
        vm.scene_drawer_visible = self.scene_drawer_visible;
        vm.reference_manager_open = self.reference_manager_open;
        for slot in &mut vm.reference_slots {
            if let Some((w, h, rgba)) = self.reference_thumbnails.get(&slot.axis) {
                slot.thumbnail = files::create_thumbnail_image(*w, *h, rgba);
            }
        }
        vm.active_keymap_id = self.state.ui.active_keymap_id.clone();
    }

    pub(crate) fn sync_viewport_context(&mut self) {
        self.viewport.set_workspace(self.state.workspace);
        self.viewport
            .set_selection_domain(self.state.selection_domain());
    }

    pub(crate) fn hide_overlay(&mut self, id: OverlayId) {
        match id {
            OverlayId::CommandPalette => self.command_search_visible = false,
            OverlayId::Settings => self.settings_visible = false,
            OverlayId::SceneDrawer => self.scene_drawer_visible = false,
            OverlayId::AssetLibrary => self.asset_library_visible = false,
            OverlayId::OutlinerContextMenu => self.context_menu = None,
            OverlayId::ContextMenu => self.context_menu = None,
            OverlayId::MenuBar => self.menu_open = None,
            OverlayId::PivotMenu => self.pivot_menu_open = false,
            OverlayId::MicroInspector => self.micro_inspector_open = false,
            OverlayId::ReferenceManager => self.reference_manager_open = false,
            OverlayId::AddMenu => self.add_menu_open = false,
        }
    }
}
