//! Ciclo de vida do bridge: construção, despacho de `UiIntent` e recuperação.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub fn new(state: AppState, viewport: V) -> Self {
        let mut bridge = Self {
            state,
            viewport,
            overlays: OverlayStack::default(),
            scene_drawer_visible: false,
            asset_library_visible: false,
            asset_query: String::new(),
            asset_sort_by_name: false,
            asset_only_favorites: false,
            split: split_view::SplitView::default(),
            paint_pip: false,
            prefab_thumbs: Default::default(),
            inspector_layout: inspector_layout::InspectorLayoutMemory::default(),
            parts_query: String::new(),
            parts_selected_only: false,
            parts_sort_by_name: false,
            parts_row_height: 28.0,
            command_search_visible: false,
            settings_visible: false,
            drag: None,
            viewport_size: [1024.0, 768.0],
            paint_last: None,
            paint_sampler: petunia_core::StrokeSampler::default(),
            paint_stabilizer: petunia_core::PointStabilizer::default(),
            paint_2d_stabilizer: petunia_core::PointStabilizer::default(),
            brush_presets: Vec::new(),
            brush_presets_loaded: false,
            brush_presets_path_override: None,
            active_brush_preset: None,
            add_menu_open: false,
            tool_modal: None,
            pixel_ratio: 1.0,
            tool_session: petunia_core::ToolSession::default(),
            last_operation: None,
            parametric_tool: None,
            tool_gesture: None,
            tool_press_extend: false,
            tool_pointer_alt: false,
            tool_press_alternate: false,
            poly_pen_points: Vec::new(),
            poly_pen_mode: PolyPenMode::default(),
            paint_2d_raster_px: 0.0,
            poly_pen_strip: None,
            gizmo_axis_colors: GizmoAxisColors::default(),
            parametric_handle_hover: false,
            tool_press_parametric_handle: false,
            keyboard_tool_modal_active: false,
            tool_modal_value: 0.0,
            rename_draft: None,
            context_menu: None,
            menu_open: None,
            pivot_menu_open: false,
            shape_anchor: None,
            slice_anchor: None,
            last_viewport_click: None,
            loop_cut: None,
            loop_cut_hover_ring: None,
            loop_cut_hover_source: None,
            loop_cut_hover_cuts: 1,
            loop_cut_balanced: false,
            tool_preview: None,
            tool_preview_revision: 0,
            active_material_slot: 0,
            shading_popover_open: false,
            pointer_position: [512.0, 384.0],
            instant_transform: false,
            gizmo_hover: None,
            gizmo_drag: None,
            autosave: petunia_core::AutosaveService::default(),
            pending_recovery: None,
            paint_pixel_grid: true,
            paint_canvas_zoom: 1,
            paint_2d_last: None,
            surface_paint: Default::default(),
            draw_parameters: Default::default(),
            paint_2d_sampler: petunia_core::StrokeSampler::default(),
            paint_target_vertex: false,
            decal_gesture: None,
            decal_last_operation: None,
            decal_frame: 0,
            decal_playing: false,
            section_layouts: section_layout::default_section_layouts(),
            preferences: petunia_config::UserPreferences::default(),
            home_open: false,
            preferences_path_override: None,
            last_tool_press: None,
            slice_trim: false,
            micro_inspector_open: false,
            micro_inspector_pos: [512.0, 384.0],
            paint_show_uv_overlay: false,
            uv_show_texture: true,
            active_profile_id: None,
            profile_volume_mode: None,
            profile_drag_target: None,
            profile_selected_point: None,
            profile_selected_segment: None,
            region_selected: None,
            profile_hover_snap: None,
            region_hover: None,
            modeling_mode: ModelingMode::default(),
            region_planes_cache: std::cell::RefCell::new(None),
            shape_planes_cache: std::cell::RefCell::new(None),
            shape_builder: None,
            profile_last_anchor_click: None,
            previous_tool: "select".to_string(),
            scene_query_cache: std::cell::RefCell::new(None),
            snap_accel_cache: std::cell::RefCell::new(None),
            keymap_editor: keymap_edit::KeymapEditor::default(),
            profile_preview_asset_id: None,
            profile_edit_gesture: None,
            profile_transform_gesture: None,
            profile_dimension_gesture: None,
            profile_volume_original: None,
            reference_manager_open: false,
            reference_thumbnails: std::collections::HashMap::new(),
            clipboard: None,
            animate: animate::AnimateRuntime::default(),
            position: [
                NumericFieldState::new(0.0, None, None).with_steps(0.1, 0.01),
                NumericFieldState::new(0.0, None, None).with_steps(0.1, 0.01),
                NumericFieldState::new(0.0, None, None).with_steps(0.1, 0.01),
            ],
            rotation: [
                NumericFieldState::new(0.0, None, None).with_steps(1.0, 0.1),
                NumericFieldState::new(0.0, None, None).with_steps(1.0, 0.1),
                NumericFieldState::new(0.0, None, None).with_steps(1.0, 0.1),
            ],
            scale: [
                NumericFieldState::new(1.0, None, None).with_steps(0.1, 0.01),
                NumericFieldState::new(1.0, None, None).with_steps(0.1, 0.01),
                NumericFieldState::new(1.0, None, None).with_steps(0.1, 0.01),
            ],
            transform_initial: [0.0, 0.0, 0.0],
        };
        bridge.reset_transform_fields();
        bridge.sync_viewport_context();
        bridge
    }

    pub fn apply(&mut self, intent: UiIntent) {
        if matches!(
            &intent,
            UiIntent::SetWorkspace(_)
                | UiIntent::OpenProjectFrom(_)
                | UiIntent::SelectSceneAsset(_)
        ) {
            self.cancel_surface_paint();
        }
        if matches!(
            &intent,
            UiIntent::SetWorkspace(_)
                | UiIntent::SetSelectionDomain(_)
                | UiIntent::SetActiveTool(_)
                | UiIntent::OpenProjectFrom(_)
                | UiIntent::SelectSceneAsset(_)
        ) {
            self.cancel_active_operation();
        }
        match intent {
            UiIntent::SetWorkspace(workspace) => {
                let changed = self.state.workspace != workspace;
                self.state.switch_workspace(workspace);
                // De volta ao PAINT, o decalque mostra a variante escolhida
                // (o playhead do Animate não manda mais na textura).
                if changed && workspace == Workspace::Paint && !self.decal_playing {
                    self.show_decal_time(None);
                }
                // Preselection morta de outro workspace não pode vazar para
                // cá: o hover pertence ao domínio e ao modo onde nasceu.
                self.state.session.tools.hover = petunia_core::HoverTarget::None;
                if changed {
                    self.normalize_tool_for_workspace();
                }
            }
            UiIntent::Animate(intent) => self.apply_animate(intent),
            UiIntent::SaveProject => {
                if let Some(path) = self.state.project.project_path.clone() {
                    self.apply(UiIntent::SaveProjectTo(PathBuf::from(path)));
                } else {
                    self.state.set_status(
                        self.state
                            .t_id(petunia_config::text_id::STATUS_SAVE_AS_REQUIRED),
                    );
                }
            }
            UiIntent::OpenCommandSearch => {
                self.command_search_visible = true;
                self.overlays.push(OverlayEntry {
                    id: OverlayId::CommandPalette,
                    kind: OverlayKind::Modal,
                    pinned: false,
                    dismiss_on_escape: true,
                    dismiss_on_click_away: true,
                });
            }
            UiIntent::OpenSettings => {
                self.settings_visible = true;
                self.overlays.push(OverlayEntry {
                    id: OverlayId::Settings,
                    kind: OverlayKind::Modal,
                    pinned: false,
                    dismiss_on_escape: true,
                    dismiss_on_click_away: true,
                });
            }
            UiIntent::CloseSettings => {
                self.settings_visible = false;
                self.overlays.remove(OverlayId::Settings);
            }
            UiIntent::SetKeymap(profile_id) => {
                self.set_keymap_profile(&profile_id);
            }
            UiIntent::ToggleSceneDrawer => {
                self.scene_drawer_visible = !self.scene_drawer_visible;
                if self.scene_drawer_visible {
                    self.overlays.push(OverlayEntry {
                        id: OverlayId::SceneDrawer,
                        kind: OverlayKind::Drawer,
                        pinned: false,
                        dismiss_on_escape: true,
                        dismiss_on_click_away: true,
                    });
                } else {
                    self.overlays.remove(OverlayId::SceneDrawer);
                }
            }
            UiIntent::ExecuteCommand(id) => {
                self.execute_command(id);
            }
            UiIntent::DismissTopOverlay => {
                self.handle_escape();
            }
            UiIntent::ScrubTransform {
                kind,
                axis,
                delta,
                fine,
            } => {
                self.scrub_transform(kind, axis, delta, fine);
            }
            UiIntent::ViewportSelection(intent) => self.apply_viewport_selection(intent),
            UiIntent::ViewportGesture(gesture) => {
                self.apply_viewport_gesture(gesture);
            }
            UiIntent::SaveProjectTo(path) => {
                if let Err(err) = petunia_project::format::save(&self.state.project.project, &path)
                {
                    self.state.set_status(crate::tr::fill(
                        &self
                            .state
                            .t_id(petunia_config::text_id::STATUS_FAILED_TO_SAVE),
                        &[("err", format!("{err}"))],
                    ));
                } else {
                    self.state.project.project_path = Some(path.display().to_string());
                    self.preferences
                        .push_recent_project(&path.display().to_string());
                    self.state.mark_document_clean();
                    self.state.set_status(crate::tr::fill(
                        &self
                            .state
                            .t_id(petunia_config::text_id::STATUS_PROJECT_SAVED),
                        &[("0", format!("{}", path.display()))],
                    ));
                }
            }
            UiIntent::OpenProjectFrom(path) => match petunia_project::format::load(&path) {
                Ok(project) => {
                    self.state.project.palette = project.palette.clone();
                    self.state.project.project = project;
                    self.state.project.undo.clear();
                    self.state.project.project_path = Some(path.display().to_string());
                    self.preferences
                        .push_recent_project(&path.display().to_string());
                    self.home_open = false;
                    self.state.mark_document_clean();
                    self.state.set_status(crate::tr::fill(
                        &self
                            .state
                            .t_id(petunia_config::text_id::STATUS_PROJECT_OPENED),
                        &[("0", format!("{}", path.display()))],
                    ));
                }
                Err(err) => {
                    self.state.set_status(crate::tr::fill(
                        &self
                            .state
                            .t_id(petunia_config::text_id::STATUS_FAILED_TO_OPEN),
                        &[("err", format!("{err}"))],
                    ));
                }
            },
            UiIntent::ImportModelFrom(path) => {
                match petunia_core::ProjectService::import_file_pipeline(
                    &mut self.state,
                    &path,
                    &petunia_project::pipeline::ImportOptions::default(),
                ) {
                    Ok(names) => self.state.set_status(crate::tr::fill(
                        &self
                            .state
                            .t_id(petunia_config::text_id::STATUS_IMPORTED_ASSET_S),
                        &[("0", format!("{}", names.len()))],
                    )),
                    Err(error) => self.state.set_status(crate::tr::fill(
                        &self
                            .state
                            .t_id(petunia_config::text_id::STATUS_IMPORT_FAILED),
                        &[("error", format!("{error}"))],
                    )),
                }
            }
            UiIntent::ExportActiveObjTo(path) => {
                let index = self.state.project.active;
                match petunia_core::ProjectService::export_obj(&self.state, index, &path) {
                    Ok(()) => self.state.set_status(crate::tr::fill(
                        &self.state.t_id(petunia_config::text_id::STATUS_EXPORTED),
                        &[("0", format!("{}", path.display()))],
                    )),
                    Err(error) => self.state.set_status(crate::tr::fill(
                        &self
                            .state
                            .t_id(petunia_config::text_id::STATUS_EXPORT_FAILED),
                        &[("error", format!("{error}"))],
                    )),
                }
            }
            UiIntent::ExportSceneGlbTo(path) => {
                let indices = self.state.export_selected_indices();
                match petunia_core::ProjectService::export_glb_with_padding(
                    &self.state,
                    &indices,
                    &path,
                    self.draw_parameters.export_padding,
                ) {
                    Ok(()) => self.state.set_status(crate::tr::fill(
                        &self.state.t_id(petunia_config::text_id::STATUS_EXPORTED),
                        &[("0", format!("{}", path.display()))],
                    )),
                    Err(error) => self.state.set_status(crate::tr::fill(
                        &self
                            .state
                            .t_id(petunia_config::text_id::STATUS_EXPORT_FAILED),
                        &[("error", format!("{error}"))],
                    )),
                }
            }
            UiIntent::ImportSvgProfiles(path) => {
                self.import_svg_profiles(&path);
            }
            UiIntent::ImportDecalFrom(path) => {
                self.import_decal_image(&path);
            }
            UiIntent::AddDecalVariantFrom(path) => {
                self.add_decal_variant_from_path(&path);
            }
            UiIntent::ExportDecalFramesTo(dir) => {
                let message = match self.export_decal_frames(&dir) {
                    Ok(count) => crate::tr::fill(
                        &self
                            .state
                            .t_id(petunia_config::text_id::STATUS_DECAL_FRAMES_EXPORTED),
                        &[
                            ("count", count.to_string()),
                            ("dir", dir.display().to_string()),
                        ],
                    ),
                    Err(error) => crate::tr::fill(
                        &self
                            .state
                            .t_id(petunia_config::text_id::STATUS_DECAL_EXPORT_FAILED),
                        &[("error", error)],
                    ),
                };
                self.state.set_status(message);
            }
            UiIntent::SetDecalVariant(index) => {
                self.set_decal_variant(index);
            }
            UiIntent::RemoveDecalVariant => {
                self.remove_decal_variant();
            }
            UiIntent::SetDecalKey => {
                self.set_decal_key();
            }
            UiIntent::RemoveDecalKey => {
                self.remove_decal_key();
            }
            UiIntent::SetDecalFrame(frame) => {
                self.set_decal_frame(frame);
            }
            UiIntent::SetDecalTrackFps(fps) => {
                self.set_decal_track_fps(fps);
            }
            UiIntent::SetDecalTrackLength(frames) => {
                self.set_decal_track_length(frames);
            }
            UiIntent::ToggleDecalPlayback => {
                self.toggle_decal_playback();
            }
            UiIntent::ImportPalette(path) => {
                match petunia_module_paint::PaintModule::import_palette_file(&mut self.state, &path)
                {
                    Ok(count) => {
                        self.state.set_status(crate::tr::fill(
                            &self
                                .state
                                .t_id(petunia_config::text_id::STATUS_IMPORTED_PALETTE_COLORS),
                            &[("count", format!("{count}"))],
                        ));
                        self.state.mark_dirty();
                    }
                    Err(error) => {
                        self.state.set_status(crate::tr::fill(
                            &self
                                .state
                                .t_id(petunia_config::text_id::STATUS_PALETTE_IMPORT_FAILED),
                            &[("error", format!("{error}"))],
                        ));
                    }
                }
            }
            UiIntent::ExportPalette(path) => {
                match petunia_module_paint::PaintModule::export_palette_file(&self.state, &path) {
                    Ok(()) => {
                        self.state.set_status(
                            self.state.t_id(
                                petunia_config::text_id::STATUS_PALETTE_EXPORTED_SUCCESSFULLY,
                            ),
                        );
                    }
                    Err(error) => {
                        self.state.set_status(crate::tr::fill(
                            &self
                                .state
                                .t_id(petunia_config::text_id::STATUS_PALETTE_EXPORT_FAILED),
                            &[("error", format!("{error}"))],
                        ));
                    }
                }
            }
            UiIntent::Undo => {
                if self.cancel_surface_paint() {
                    return;
                }
                if self.cancel_active_operation() {
                    return;
                }
                if self.state.undo() {
                    self.state
                        .set_status(self.state.t_id(petunia_config::text_id::STATUS_UNDO_DONE));
                }
            }
            UiIntent::Redo => {
                if self.cancel_surface_paint() {
                    return;
                }
                if self.cancel_active_operation() {
                    return;
                }
                if self.state.redo() {
                    self.state
                        .set_status(self.state.t_id(petunia_config::text_id::STATUS_REDO_DONE));
                }
            }
            UiIntent::SetSelectionDomain(domain) => {
                self.state.set_selection_domain(domain);
                self.report_selection_domain();
            }
            UiIntent::CycleSelectionDomain => {
                self.state.cycle_selection_domain();
                self.report_selection_domain();
            }
            UiIntent::AddPrimitive(kind) => {
                self.state.set_selection_domain(SelectionDomain::Object);
                self.sync_viewport_context();
                self.state.begin_primitive(kind, None);
                self.state.set_status(crate::tr::fill(
                    &self.state.t_id(petunia_config::text_id::STATUS_ADDED),
                    &[("0", kind.default_name().to_string())],
                ));
            }
            UiIntent::FreezeActivePrimitive => {
                if self.state.freeze_active_primitive() {
                    self.state.set_status(
                        self.state.t_id(
                            petunia_config::text_id::STATUS_PRIMITIVE_FROZEN_TO_EDITABLE_MESH,
                        ),
                    );
                }
            }
            UiIntent::DeleteActiveAsset => {
                // `edit.delete` é contextual: em Object remove o asset ativo; em
                // Point/Edge/Face remove os sub-elementos selecionados.
                if let Err(error) = self.state.dispatch_command("edit.delete") {
                    self.state.set_status(error.to_string());
                }
            }
            UiIntent::SetPaintColor(color) => {
                if color.iter().all(|component| component.is_finite()) {
                    let color = color.map(|component| component.clamp(0.0, 1.0));
                    self.state.paint_color = color;
                    self.state.session.tools.paint_color = color;
                }
            }
            UiIntent::SetBrushSize(size) => {
                let size = size.clamp(0.01, 100.0);
                self.state.session.tools.paint_radius = size;
                // O pincel do canvas 2D usa o mesmo tamanho (px) do slider.
                self.state.session.tools.canvas_brush =
                    petunia_core::brush_size_px_from_slider(size)
                        .round()
                        .max(1.0) as u32;
            }
            UiIntent::SetBrushOpacity(opacity) => {
                self.state.session.tools.paint_strength = opacity.clamp(0.0, 1.0);
            }
            UiIntent::SetBrushHardness(hardness) => {
                self.state.session.tools.brush_hardness = hardness.clamp(0.0, 1.0);
            }
            UiIntent::TogglePaintSymmetryX => {
                self.state.session.tools.paint_symmetry_x =
                    !self.state.session.tools.paint_symmetry_x;
            }
            UiIntent::TogglePaintSymmetryY => {
                self.state.session.tools.paint_symmetry_y =
                    !self.state.session.tools.paint_symmetry_y;
            }
            UiIntent::TogglePaintSymmetryZ => {
                self.state.session.tools.paint_symmetry_z =
                    !self.state.session.tools.paint_symmetry_z;
            }
            UiIntent::SetPaintSymmetryX(val) => {
                self.state.session.tools.paint_symmetry_x = val;
            }
            UiIntent::SetPaintSymmetryY(val) => {
                self.state.session.tools.paint_symmetry_y = val;
            }
            UiIntent::SetPaintSymmetryZ(val) => {
                self.state.session.tools.paint_symmetry_z = val;
            }
            UiIntent::SetPaintTargetVertex(val) => {
                self.paint_target_vertex = val;
            }
            UiIntent::TogglePaintMaskSelection => {
                self.state.session.tools.paint_isolate_selection =
                    !self.state.session.tools.paint_isolate_selection;
            }
            UiIntent::SetPolyPenMode(id) => {
                if let Some(mode) = PolyPenMode::from_id(&id)
                    && mode != self.poly_pen_mode
                {
                    self.poly_pen_mode = mode;
                    self.state.session.tools.hover = petunia_core::HoverTarget::None;
                    self.state.mark_dirty();
                }
            }
            UiIntent::SetPaintMaskSelection(val) => {
                self.state.session.tools.paint_isolate_selection = val;
            }
            UiIntent::SetActiveTool(tool) => {
                if self.state.session.tools.active_tool != tool {
                    self.previous_tool = self.state.session.tools.active_tool.clone();
                }
                if tool != "poly_pen" {
                    self.poly_pen_points.clear();
                    if let Some(strip) = self.poly_pen_strip.take() {
                        self.state.cancel_poly_pen_strip(strip);
                    }
                }
                if self.state.session.tools.active_tool == "draw_profile" && tool != "draw_profile"
                {
                    self.profile_pointer_up();
                    if !self.draw_shapes_visible()
                        || !matches!(tool.as_str(), "move" | "rotate" | "scale" | "transform")
                    {
                        self.state.profile.clear();
                        self.active_profile_id = None;
                    }
                    self.profile_selected_point = None;
                    self.profile_drag_target = None;
                    self.profile_edit_gesture = None;
                }
                self.state.session.tools.active_tool = tool.clone();
                // Nome traduzido em `tools.<id>`; sem tradução, o id fica.
                let key = format!("tools.{tool}");
                let name = match self.state.t(&key) {
                    translated if translated == key => tool.clone(),
                    translated => translated,
                };
                let message = self
                    .state
                    .t_id(petunia_config::text_id::STATUS_ACTIVE_TOOL)
                    .replace("{tool}", &name);
                self.state.set_status(message);
                match tool.as_str() {
                    // Transformações são transacionais por arrasto: a sessão
                    // modal abre no pointer-down da viewport, não ao escolher a
                    // ferramenta.
                    "move" | "rotate" | "scale" | "transform" => {
                        self.state.session.tools.gizmo_mode = match tool.as_str() {
                            "rotate" => petunia_core::ModalKind::Rotate,
                            "scale" => petunia_core::ModalKind::Scale,
                            _ => petunia_core::ModalKind::Move,
                        };
                    }
                    "cut" => {
                        if let Some(mesh) = self.state.project.active_mesh().cloned() {
                            self.state.session.tools.cut_session =
                                Some(petunia_core::CutSession::new(mesh));
                            self.state.set_status(self.state.t_id(
                                petunia_config::text_id::STATUS_CUT_CHOOSE_TWO_EDGE_POINTS_IN_THE,
                            ));
                        }
                    }
                    "draw_profile" => {
                        self.active_profile_id = None;
                        self.profile_selected_point = None;
                        self.profile_drag_target = None;
                        self.profile_edit_gesture = None;
                        // A câmera nunca se move sozinha (capítulos 01 e 02).
                        petunia_module_model::profile_capture_current(&mut self.state);
                        let plane = self.workplane_display_name();
                        let message = self
                            .state
                            .t_id(petunia_config::text_id::TOOL_GRAMMAR_DRAW_READY)
                            .replace("{plane}", &plane);
                        self.state.set_status(message);
                    }
                    "loop_cut" => {
                        self.loop_cut_hover_ring = None;
                        self.loop_cut_hover_source = None;
                        self.loop_cut_hover_cuts = 1;
                        self.state.session.tools.hover = petunia_core::HoverTarget::None;
                        self.state.set_status(
                            self.state.t_id(
                                petunia_config::text_id::STATUS_LOOP_CUT_HOVER_A_QUAD_EDGE_RING,
                            ),
                        );
                    }
                    "slice" => {
                        self.state.set_status(self.state.t_id(
                            petunia_config::text_id::STATUS_SLICE_DRAG_IN_THE_VIEWPORT_TO_DEFINE,
                        ));
                    }
                    // No PAINT, Fill é o balde (age no clique, respeitando máscara e
                    // escopo). Só fora do PAINT ele pinta os vértices selecionados.
                    "fill" if self.state.workspace != Workspace::Paint => {
                        let count =
                            petunia_module_paint::PaintModule::fill_selection(&mut self.state);
                        self.state.set_status(crate::tr::fill(
                            &self
                                .state
                                .t_id(petunia_config::text_id::STATUS_FILLED_POINTS),
                            &[("count", format!("{count}"))],
                        ));
                    }
                    // Formas e pincéis do workspace PAINT compartilham o mesmo
                    // índice de tipo de pincel que o resto do domínio.
                    "line" | "rectangle" | "ellipse" => {
                        self.state.session.tools.paint_brush_kind =
                            petunia_core::kind_from_brush_type(match tool.as_str() {
                                "line" => petunia_core::BrushType::Line,
                                "ellipse" => petunia_core::BrushType::Ellipse,
                                _ => petunia_core::BrushType::Rectangle,
                            });
                        self.state.set_status(self.state.t_id(
                            petunia_config::text_id::STATUS_SHAPE_PRESS_ON_THE_SURFACE_TO_ANCHOR,
                        ));
                    }
                    "brush" | "eraser" | "picker" | "airbrush" | "pixel" | "smudge" | "blur"
                    | "dodge" | "burn" | "spray" | "clone" => {
                        self.state.session.tools.paint_brush_kind =
                            petunia_core::kind_from_brush_type(Self::brush_type_for_tool(&tool));
                    }
                    _ => {}
                }
            }
            UiIntent::SelectSceneAsset(id_str) => {
                if let Some(idx) = uuid::Uuid::parse_str(&id_str)
                    .ok()
                    .and_then(|id| self.state.project.assets.iter().position(|a| a.id == id))
                {
                    self.state.select_object(Some(idx), false);
                    self.state.mark_dirty();
                    self.reset_transform_fields();
                }
            }
            UiIntent::ToggleSceneAssetVisibility(id_str) => {
                if let Some(asset_index) = uuid::Uuid::parse_str(&id_str)
                    .ok()
                    .and_then(|id| self.state.project.assets.iter().position(|a| a.id == id))
                    && let Err(error) =
                        self.state
                            .dispatch(&petunia_core::ToggleVisibilityAssetCmd {
                                asset_index: Some(asset_index),
                            })
                {
                    self.state.set_status(error.to_string());
                }
            }
            UiIntent::ToggleSceneAssetLock(id_str) => {
                if let Some(asset_index) = uuid::Uuid::parse_str(&id_str)
                    .ok()
                    .and_then(|id| self.state.project.assets.iter().position(|a| a.id == id))
                    && let Err(error) = self.state.dispatch(&petunia_core::ToggleLockAssetCmd {
                        asset_index: Some(asset_index),
                    })
                {
                    self.state.set_status(error.to_string());
                }
            }
            UiIntent::MoveSceneAsset { id, delta } => {
                // Moves asset by delta in scene hierarchy / Move asset por delta na hierarquia da cena
                if let Some(uuid) = uuid::Uuid::parse_str(&id).ok()
                    && let Some(idx) = self.state.project.find(uuid)
                {
                    let len = self.state.project.assets.len();
                    let target = (idx as isize + delta as isize)
                        .clamp(0, (len.saturating_sub(1)) as isize)
                        as usize;
                    if target != idx {
                        let _ = self.state.dispatch(&petunia_core::ReorderAssetCmd {
                            from: idx,
                            to: target,
                        });
                    }
                }
            }
            UiIntent::ReorderSceneAsset { from, to } => {
                // Reorders asset explicitly from index to index / Reordena asset explicitamente entre índices
                let _ = self
                    .state
                    .dispatch(&petunia_core::ReorderAssetCmd { from, to });
            }
            UiIntent::ToggleIsolateActiveAsset => {
                // Toggles isolation of active asset / Alterna isolamento do asset ativo
                self.state.toggle_isolate();
            }
            UiIntent::SetTheme(theme_id) => {
                self.state.ui.active_theme_id = theme_id;
            }
            UiIntent::DuplicateActiveAsset => {
                let _ = self
                    .state
                    .dispatch(&petunia_core::DuplicateAssetCmd { asset_index: None });
                self.state.set_status(
                    self.state
                        .t_id(petunia_config::text_id::STATUS_OBJECT_DUPLICATED),
                );
            }
            UiIntent::SelectAll => {
                let _ = self.state.dispatch(&petunia_core::SelectAllCmd);
                self.state.set_status(
                    self.state
                        .t_id(petunia_config::text_id::STATUS_EVERYTHING_SELECTED),
                );
            }
            UiIntent::ClearSelection => {
                let _ = self.state.dispatch(&petunia_core::ClearSelectionCmd);
                self.state.set_status(
                    self.state
                        .t_id(petunia_config::text_id::STATUS_SELECTION_CLEARED),
                );
            }
            UiIntent::InvertSelection => {
                let _ = self.state.dispatch(&petunia_core::InvertSelectionCmd);
                self.state.set_status(
                    self.state
                        .t_id(petunia_config::text_id::STATUS_SELECTION_INVERTED),
                );
            }
            UiIntent::ToggleAssetLibrary => {
                self.asset_library_visible = !self.asset_library_visible;
                if self.asset_library_visible {
                    self.overlays.push(OverlayEntry {
                        id: OverlayId::AssetLibrary,
                        kind: OverlayKind::Drawer,
                        pinned: false,
                        dismiss_on_escape: true,
                        dismiss_on_click_away: true,
                    });
                } else {
                    self.overlays.remove(OverlayId::AssetLibrary);
                }
            }
            UiIntent::ResetCamera => {
                let _ = self.state.dispatch(&petunia_core::ResetCameraCmd);
                self.state.set_status(
                    self.state
                        .t_id(petunia_config::text_id::STATUS_CAMERA_RESET),
                );
            }
            UiIntent::ToggleProjection => {
                let _ = self.state.dispatch(&petunia_core::ToggleProjectionCmd);
                self.state.set_status(
                    if self.state.session.camera.proj == petunia_core::Projection::Ortho {
                        "Projeção: Ortográfica"
                    } else {
                        "Projeção: Perspectiva"
                    },
                );
            }
            UiIntent::SaveActiveAsAsset => {
                // "Save as asset" cria um prefab da biblioteca; a cena não muda.
                self.state.save_selection_as_prefab(None);
            }
            UiIntent::AssignMaterialSlot(slot) => {
                self.assign_material_slot(slot);
            }
            UiIntent::CreateMaterial => {
                self.create_material();
            }
            UiIntent::DuplicateMaterial(slot) => {
                self.duplicate_material(slot);
            }
            UiIntent::Paint2dStroke {
                norm_x,
                norm_y,
                phase,
            } => {
                self.paint_2d_stroke(norm_x, norm_y, phase);
            }
            UiIntent::TogglePaintPixelGrid => {
                self.paint_pixel_grid = !self.paint_pixel_grid;
            }
            UiIntent::SetPaintCanvasZoom(zoom) => {
                self.paint_canvas_zoom = zoom.clamp(1, 16);
            }
            UiIntent::ProjectFromReference => {
                self.project_from_reference();
            }
            UiIntent::BakeReference => {
                self.bake_reference();
            }
            UiIntent::ToggleFaceOrientation => {
                self.toggle_face_orientation();
            }
            UiIntent::ToggleUvChecker => {
                self.toggle_uv_checker();
            }
            UiIntent::ToggleProportionalEditing => {
                self.toggle_proportional_editing();
            }
            UiIntent::SetProportionalRadius(radius) => {
                self.set_proportional_radius(radius);
            }
            UiIntent::SetProportionalFalloff(falloff) => {
                self.set_proportional_falloff(&falloff);
            }
            UiIntent::ToggleSnapEnabled => {
                self.toggle_snap_enabled();
            }
            UiIntent::SetSnapTarget(target) => {
                self.set_snap_target(&target);
            }
            UiIntent::AddProfileRectangle { width, height } => {
                self.add_profile_rectangle(width, height);
            }
            UiIntent::AddProfileCircle { radius, segments } => {
                self.add_profile_circle(radius, segments);
            }
            UiIntent::AddDecalLayer => {
                self.add_decal_layer();
            }
            UiIntent::SetPaintTextureSide(side) => {
                petunia_module_paint::PaintModule::set_texture_side(&mut self.state, side);
            }
            UiIntent::SetDecalTransform {
                layer_id,
                center_u,
                center_v,
                scale_u,
                scale_v,
                rotation_deg,
            } => {
                // Dispatches command to update decal transformation (P3D-133)
                // Despacha comando para atualizar transformação do decalque (P3D-133)
                if let Ok(id) = uuid::Uuid::parse_str(&layer_id) {
                    let _ = self.state.dispatch(&petunia_core::SetDecalTransformCmd {
                        layer_id: id,
                        center_uv: [center_u, center_v],
                        scale_uv: [scale_u, scale_v],
                        rotation_rad: rotation_deg.to_radians(),
                    });
                }
            }
            UiIntent::BakeActiveDecal => {
                // Bakes active decal layer to static raster (P3D-160)
                // Rasteriza a camada decal ativa para raster estático (P3D-160)
                if let Some(active_layer) = self
                    .state
                    .project
                    .assets
                    .get(self.state.project.active)
                    .and_then(|asset| asset.paint_stack.as_ref())
                    .and_then(|stack| stack.active())
                {
                    let id = active_layer.id;
                    let _ = self
                        .state
                        .dispatch(&petunia_core::BakeDecalCmd { layer_id: id });
                }
            }
            UiIntent::SetSectionDocked { section, docked } => {
                self.set_section_docked(section, docked);
            }
            UiIntent::MoveSectionFloat { section, x, y } => {
                self.move_section_float(section, x, y);
            }
            UiIntent::SetSectionPinOpen { section, pin_open } => {
                self.set_section_pin_open(section, pin_open);
            }
            UiIntent::SetSectionPinnedAsset { section, asset } => {
                self.set_section_pinned_asset(section, asset);
            }
            UiIntent::SetOriginGeometry => {
                let _ = self.state.set_origin_geometry();
            }
            UiIntent::SetOriginBottom => {
                let _ = self.state.set_origin_bottom();
            }
            UiIntent::SetOriginCursor => {
                let _ = self.state.set_origin_cursor();
            }
            UiIntent::SetOriginSelection => {
                let _ = self.state.set_origin_selection();
            }
            UiIntent::SetGeometryToOrigin => {
                let _ = self.state.set_geometry_to_origin();
            }
            UiIntent::ToggleEditPivot => {
                self.state.toggle_edit_pivot();
            }
            UiIntent::SetColorblindAxes(enabled) => {
                let _ = self.set_colorblind_axes(enabled);
            }
            UiIntent::SetReducedMotion(enabled) => {
                let _ = self.set_reduced_motion(enabled);
            }
            UiIntent::SetDoubleTapIntervalMs(interval) => {
                let _ = self.set_double_tap_interval_ms(interval);
            }
            UiIntent::ToggleMicroInspector => {
                let _ = self.toggle_micro_inspector();
            }
            UiIntent::TogglePaintUvOverlay => {
                self.paint_show_uv_overlay = !self.paint_show_uv_overlay;
                self.state.mark_dirty();
            }
            UiIntent::ToggleUvShowTexture => {
                self.uv_show_texture = !self.uv_show_texture;
                self.state.mark_dirty();
            }
            UiIntent::ProfileSetWorkplaneGround => {
                self.reset_profile_for_workplane();
                self.state.profile.workplane_locked = true;
                petunia_module_model::profile_capture_ground(&mut self.state);
                self.announce_locked_workplane();
            }
            UiIntent::ProfileSetWorkplaneFace => {
                self.reset_profile_for_workplane();
                self.state.profile.workplane_locked = true;
                if petunia_module_model::profile_capture_face(&mut self.state) {
                    self.announce_locked_workplane();
                } else {
                    let message = self
                        .state
                        .t_id(petunia_config::text_id::TOOL_GRAMMAR_NO_FACE_SELECTED);
                    self.state.set_status(message);
                }
            }
            UiIntent::ProfileSetWorkplaneView => {
                self.reset_profile_for_workplane();
                self.state.profile.workplane_locked = true;
                petunia_module_model::profile_capture_view(&mut self.state);
                self.announce_locked_workplane();
            }
            UiIntent::ProfileSetWorkplaneSelection => {
                self.reset_profile_for_workplane();
                if petunia_module_model::profile_capture_from_selection(&mut self.state) {
                    self.state.profile.workplane_locked = true;
                    self.announce_locked_workplane();
                } else {
                    self.state.set_status(
                        self.state
                            .t_id(petunia_config::text_id::STATUS_SELECT_3_POINTS_OR_A_FACE_TO),
                    );
                }
            }
            UiIntent::ProfileSetWorkplaneAuto => {
                self.reset_profile_for_workplane();
                self.state.profile.workplane_locked = false;
                petunia_module_model::profile_capture_auto(&mut self.state);
                let message = self
                    .state
                    .t_id(petunia_config::text_id::TOOL_GRAMMAR_WORKPLANE_AUTO);
                self.state.set_status(message);
            }
            UiIntent::SetModelingMode(mode) => {
                if self.state.workspace != Workspace::Model {
                    self.state.switch_workspace(Workspace::Model);
                    self.state.session.tools.hover = petunia_core::HoverTarget::None;
                    self.normalize_tool_for_workspace();
                }
                if self.modeling_mode != mode {
                    self.modeling_mode = mode;
                    // A ferramenta ativa que não existe no trilho novo volta
                    // para Select; nada é convertido ou selecionado sozinho.
                    let tool = self.state.session.tools.active_tool.clone();
                    if !mode.offers_tool(&tool) {
                        self.apply(UiIntent::SetActiveTool("select".into()));
                    }
                }
                let message = self.state.t_id(match mode {
                    ModelingMode::Draw => petunia_config::text_id::WORKSPACE_DRAW_READY,
                    ModelingMode::Poly => petunia_config::text_id::WORKSPACE_POLY_READY,
                });
                self.state.set_status(message);
                self.state.mark_dirty();
            }
            UiIntent::ProfileLookAtPlane => {
                petunia_module_model::profile_align_camera_to_workplane(&mut self.state);
                self.state.mark_dirty();
            }
        }
        self.sync_viewport_context();
    }

    /// Um passo de autosave, respeitando intervalo e dirty state do domínio.
    ///
    /// Retorna `true` quando um snapshot foi gravado. Autosave nunca limpa o
    /// dirty state nem toca no arquivo oficial.
    pub fn autosave_tick(&mut self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let dirty = self.state.is_document_dirty();
        let path = self.state.project.project_path.clone();
        let path_ref = path.as_deref().map(std::path::Path::new);
        let revision = self.state.project.project.revision_id();
        matches!(
            self.autosave
                .tick(now, dirty, revision, &self.state.project.project, path_ref),
            Some(Ok(_))
        )
    }

    /// Carrega o snapshot de recuperação detectado no arranque.
    pub fn recover_pending(&mut self) -> bool {
        let Some(info) = self.pending_recovery.take() else {
            return false;
        };
        self.apply(UiIntent::OpenProjectFrom(info.snapshot_path));
        self.state.set_status(crate::tr::fill(
            &self
                .state
                .t_id(petunia_config::text_id::STATUS_RECOVERED_SNAPSHOT_OF),
            &[("0", info.project_name.to_string())],
        ));
        true
    }

    /// Mantém o projeto oficial e encerra o aviso de recuperação.
    pub fn keep_saved_project(&mut self) -> bool {
        self.pending_recovery.take().is_some()
    }

    /// Descarta os snapshots de recuperação e o marcador de sessão.
    pub fn discard_pending_recovery(&mut self) -> bool {
        if self.pending_recovery.take().is_none() {
            return false;
        }
        let path = self.state.project.project_path.clone();
        let path_ref = path.as_deref().map(std::path::Path::new);
        match petunia_core::AutosaveService::discard_recovery(path_ref) {
            Ok(()) => self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_RECOVERY_SNAPSHOTS_DISCARDED),
            ),
            Err(error) => self.state.set_status(crate::tr::fill(
                &self
                    .state
                    .t_id(petunia_config::text_id::STATUS_FAILED_TO_DISCARD_SNAPSHOTS),
                &[("error", format!("{error}"))],
            )),
        }
        true
    }
}
