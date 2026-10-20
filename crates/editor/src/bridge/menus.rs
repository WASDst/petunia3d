//! Menus do Header, menu de contexto e popovers associados.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub fn set_pivot_menu_open(&mut self, open: bool) -> bool {
        self.shading_popover_open = false;
        if open {
            self.close_menu();
            self.close_context_menu();
            self.overlays.push(OverlayEntry {
                id: OverlayId::PivotMenu,
                kind: OverlayKind::Popover,
                pinned: false,
                dismiss_on_escape: true,
                dismiss_on_click_away: true,
            });
            self.pivot_menu_open = true;
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_CHOOSE_A_TRANSFORM_PIVOT),
            );
        } else if self.overlays.remove(OverlayId::PivotMenu).is_some() {
            self.pivot_menu_open = false;
        }
        self.pivot_menu_open == open
    }

    /// Abre, troca ou fecha um menu da barra superior.
    pub fn toggle_menu(&mut self, id: &str) -> bool {
        let next = MenuKind::ALL.into_iter().find(|kind| kind.id() == id);
        self.menu_open = match (self.menu_open, next) {
            (Some(current), Some(next)) if current == next => None,
            (_, next) => next,
        };
        match self.menu_open {
            Some(_) => self.overlays.push(OverlayEntry {
                id: OverlayId::MenuBar,
                kind: OverlayKind::Popover,
                pinned: false,
                dismiss_on_escape: true,
                dismiss_on_click_away: true,
            }),
            None => {
                self.overlays.remove(OverlayId::MenuBar);
            }
        }
        self.state.mark_dirty();
        self.menu_open.is_some()
    }

    /// Abre diretamente um menu ou troca para ele (rollover entre menus do topo).
    pub fn open_menu(&mut self, id: &str) -> bool {
        let next = MenuKind::ALL.into_iter().find(|kind| kind.id() == id);
        if self.menu_open == next {
            return false;
        }
        self.menu_open = next;
        match self.menu_open {
            Some(_) => self.overlays.push(OverlayEntry {
                id: OverlayId::MenuBar,
                kind: OverlayKind::Popover,
                pinned: false,
                dismiss_on_escape: true,
                dismiss_on_click_away: true,
            }),
            None => {
                self.overlays.remove(OverlayId::MenuBar);
            }
        }
        self.state.mark_dirty();
        self.menu_open.is_some()
    }

    pub fn close_menu(&mut self) -> bool {
        self.overlays.remove(OverlayId::MenuBar);
        self.menu_open.take().is_some()
    }

    /// Executa um item de menu pelo id canônico que ele publica.
    pub fn menu_item_invoked(&mut self, id: &str) -> bool {
        self.close_menu();
        match id {
            // Itens de arquivo que abrem diálogo são despachados pelo mesmo
            // caminho assíncrono do `command-file-action`; aqui só o que resolve
            // de imediato.
            "file.new" => self.execute_core_command("file.new").is_ok(),
            "edit.undo" => {
                self.apply(UiIntent::Undo);
                true
            }
            "edit.redo" => {
                self.apply(UiIntent::Redo);
                true
            }
            "edit.duplicate" => {
                self.apply(UiIntent::DuplicateActiveAsset);
                true
            }
            "view.toggle_split" => self.toggle_split_view(),
            "view.reference_images" => self.toggle_reference_manager(),
            "window.command_palette" => {
                self.apply(UiIntent::OpenCommandSearch);
                true
            }
            "window.settings" => {
                self.apply(UiIntent::OpenSettings);
                true
            }
            other => self.execute_core_command(other).is_ok(),
        }
    }

    /// Abre o menu de contexto do Outliner sobre a linha de `id`.
    ///
    /// O alvo é selecionado antes de abrir: as ações do menu operam sobre ele e
    /// um clique-direito em linha não selecionada precisa agir no que o usuário
    /// apontou, não no que estava ativo.
    pub fn open_context_menu(&mut self, id: &str, x: f32, y: f32) -> bool {
        let (is_profile, asset_id) = if let Some(p_str) = id.strip_prefix("profile:") {
            (true, uuid::Uuid::parse_str(p_str).ok())
        } else {
            (false, uuid::Uuid::parse_str(id).ok())
        };
        let Some(asset) = asset_id else {
            return false;
        };
        if is_profile {
            if !self
                .state
                .project
                .project
                .profiles
                .iter()
                .any(|p| p.id == asset)
            {
                return false;
            }
            self.activate_profile(asset);
        } else {
            if !self.state.project.assets.iter().any(|a| a.id == asset) {
                return false;
            }
            self.select_asset_by_id(asset);
        }
        self.context_menu = Some(ContextMenuState {
            x,
            y,
            asset,
            viewport: false,
            edge: None,
            face: None,
        });
        self.overlays.push(OverlayEntry {
            id: OverlayId::OutlinerContextMenu,
            kind: OverlayKind::ContextMenu,
            pinned: false,
            dismiss_on_escape: true,
            dismiss_on_click_away: true,
        });
        self.state.mark_dirty();
        true
    }

    /// Menu da viewport (botão direito no 3D): só verbetes de seleção, sem
    /// alvo de asset. Id de overlay próprio para o Escape LIFO fechar o menu
    /// certo quando Outliner e viewport competem.
    pub fn open_viewport_context_menu(&mut self, x: f32, y: f32) -> bool {
        // Os loops partem do que o cursor aponta: a aresta (ou a face) é capturada
        // agora, porque o menu é um overlay e o hover some quando ele abre.
        let (edge, face) = self.context_loop_seeds(x, y);
        self.context_menu = Some(ContextMenuState {
            x,
            y,
            asset: uuid::Uuid::nil(),
            viewport: true,
            edge,
            face,
        });
        self.overlays.push(OverlayEntry {
            id: OverlayId::ContextMenu,
            kind: OverlayKind::ContextMenu,
            pinned: false,
            dismiss_on_escape: true,
            dismiss_on_click_away: true,
        });
        self.state.mark_dirty();
        true
    }

    /// Aresta e face sob o cursor (px da viewport) para os loops do menu de contexto.
    pub(crate) fn context_loop_seeds(&self, x: f32, y: f32) -> (Option<(u32, u32)>, Option<usize>) {
        let [width, height] = self.viewport_size;
        if self.state.workspace == Workspace::Paint
            || !self.state.selection_domain().is_component()
            || width <= 1.0
            || height <= 1.0
        {
            return (None, None);
        }
        let (nx, ny) = (x / width, y / height);
        let edge = match self.pick_target_for_domain(SelectionDomain::Edge, nx, ny) {
            petunia_core::HoverTarget::Edge(a, b) => Some((a, b)),
            _ => None,
        };
        let face = match self.pick_target_for_domain(SelectionDomain::Face, nx, ny) {
            petunia_core::HoverTarget::Face(face) => Some(face),
            _ => None,
        };
        (edge, face)
    }

    /// Edge Loop / Edge Ring do menu de contexto. Parte da aresta apontada no
    /// clique direito; sem aresta sob o cursor, expande as arestas já
    /// selecionadas. `ring` escolhe Edge Ring em vez de Edge Loop.
    pub fn select_loop_from_context(&mut self, seed: Option<(u32, u32)>, ring: bool) -> bool {
        // O domínio muda antes de selecionar: a conversão entre domínios reescreve
        // a seleção e apagaria o loop recém-escolhido.
        if self.state.selection_domain() != SelectionDomain::Edge {
            self.state.set_selection_domain(SelectionDomain::Edge);
            self.sync_viewport_context();
        }
        let has_edges = self
            .state
            .project
            .active_mesh()
            .is_some_and(|mesh| !mesh.selected_edges.is_empty());
        let count = match seed {
            Some(edge) => self.select_loop_through_core(|mesh| {
                if ring {
                    mesh.select_edge_ring(edge, false)
                } else {
                    mesh.select_edge_loop(edge, false)
                }
            }),
            None if has_edges => self.select_loop_through_core(|mesh| {
                mesh.expand_selection_to_edge_loops(ring);
                mesh.selected_edges.len()
            }),
            None => 0,
        };
        if count == 0 {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_RIGHT_CLICK_ON_AN_EDGE_TO_SELECT),
            );
            return false;
        }
        self.state.set_status(crate::tr::fill(
            &self
                .state
                .t_id(petunia_config::text_id::STATUS_SELECTED_EDGE_EDGES),
            &[
                ("0", (if ring { "ring" } else { "loop" }).to_string()),
                ("count", format!("{count}")),
            ],
        ));
        self.state.sync_selection();
        self.state.mark_dirty();
        true
    }

    /// Face Loop do menu de contexto, pela face (e aresta mais próxima) apontada.
    pub fn select_face_loop_from_context(
        &mut self,
        face: Option<usize>,
        edge: Option<(u32, u32)>,
    ) -> bool {
        let Some(face) = face else {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_RIGHT_CLICK_ON_A_FACE_TO_SELECT),
            );
            return false;
        };
        if self.state.selection_domain() != SelectionDomain::Face {
            self.state.set_selection_domain(SelectionDomain::Face);
            self.sync_viewport_context();
        }
        if self.state.project.active_mesh().is_none() {
            return false;
        }
        let count = self.select_loop_through_core(|mesh| mesh.select_face_loop(face, edge, false));
        self.state.set_status(crate::tr::fill(
            &self
                .state
                .t_id(petunia_config::text_id::STATUS_SELECTED_FACE_LOOP_FACES),
            &[("count", format!("{count}"))],
        ));
        self.state.sync_selection();
        self.state.mark_dirty();
        true
    }

    /// Botão direito na viewport: abre o menu de contexto e nunca cancela
    /// (constituição 11, ADR 007). Durante um gesto ou operação aberta ele é
    /// ignorado; cancelar é `Esc`. Retorna verdadeiro quando abriu o menu.
    pub fn viewport_context_triage(&mut self, x: f32, y: f32) -> bool {
        let _ = self.cancel_rename();
        if self.tool_session.is_gesture_active()
            || self.drag.is_some()
            || self.tool_modal.is_some()
            || self.state.session.tools.modal.is_some()
        {
            return false;
        }
        self.open_viewport_context_menu(x, y)
    }

    pub fn close_context_menu(&mut self) -> bool {
        self.overlays.remove(OverlayId::OutlinerContextMenu);
        self.overlays.remove(OverlayId::ContextMenu);
        self.context_menu.take().is_some()
    }

    /// Executa uma ação do menu de contexto sobre o alvo apontado.
    pub fn context_menu_action(&mut self, action: &str) -> bool {
        let Some(menu) = self.context_menu else {
            return false;
        };
        // Modo viewport: só verbetes de seleção, sem alvo de asset.
        if menu.viewport {
            self.close_context_menu();
            return match action {
                "select_all" => {
                    self.apply(UiIntent::SelectAll);
                    true
                }
                "clear_selection" => {
                    self.apply(UiIntent::ClearSelection);
                    true
                }
                "invert_selection" => {
                    self.apply(UiIntent::InvertSelection);
                    true
                }
                "frame" => {
                    let _ = self.state.dispatch_command("view.frame_selection");
                    true
                }
                "delete" => {
                    if self.state.session.tools.active_tool == "draw_profile" {
                        self.delete_selected_profile_point()
                    } else if self.state.selection_domain().is_component() {
                        let _ = self.execute_core_command("model.delete");
                        true
                    } else {
                        self.apply(UiIntent::DeleteActiveAsset);
                        true
                    }
                }
                "duplicate" => {
                    self.apply(UiIntent::DuplicateActiveAsset);
                    true
                }
                "extrude" => {
                    if self.state.session.tools.active_tool == "draw_profile"
                        && (self.active_profile_closed() || self.active_profile_point_count() >= 3)
                    {
                        if self.profile_volume_mode
                            == Some(petunia_module_model::ProfileVolumeMode::Extrude)
                        {
                            self.commit_profile_volume();
                        } else {
                            self.enter_profile_volume("extrude");
                        }
                    } else {
                        self.execute_shortcut_tool("model.extrude");
                    }
                    true
                }
                "inset" => {
                    self.execute_shortcut_tool("model.inset");
                    true
                }
                "bevel" => {
                    self.execute_shortcut_tool("model.bevel");
                    true
                }
                "subdivide" => {
                    let _ = self.execute_core_command("model.subdivide");
                    true
                }
                "loop_cut" => {
                    self.apply(UiIntent::SetActiveTool("loop_cut".to_string()));
                    true
                }
                "connect" => {
                    let _ = self.execute_core_command("model.connect");
                    true
                }
                "make_face" => {
                    let _ = self.execute_core_command("model.make_face");
                    true
                }
                "merge" => {
                    let _ = self.execute_core_command("model.merge");
                    true
                }
                "dissolve" => {
                    let _ = self.execute_core_command("model.dissolve");
                    true
                }
                "slice" => {
                    self.apply(UiIntent::SetActiveTool("slice".to_string()));
                    true
                }
                "separate_selection" => {
                    let _ = self.execute_core_command("model.separate_selection");
                    true
                }
                "workplane_from_selection" => {
                    self.apply(UiIntent::ProfileSetWorkplaneSelection);
                    true
                }
                "flip_normals" => {
                    let _ = self.execute_core_command("model.flip_normals");
                    true
                }
                "draw_unite" => self.run_pathfinder(petunia_core::PathfinderOp::Unite),
                "draw_subtract" => self.run_pathfinder(petunia_core::PathfinderOp::Subtract),
                "draw_intersect" => self.run_pathfinder(petunia_core::PathfinderOp::Intersect),
                "draw_exclude" => self.run_pathfinder(petunia_core::PathfinderOp::Exclude),
                "edge_loop" => self.select_loop_from_context(menu.edge, false),
                "edge_ring" => self.select_loop_from_context(menu.edge, true),
                "face_loop" => self.select_face_loop_from_context(menu.face, menu.edge),
                "mark_seam" => self.toggle_selected_uv_seams(),
                "shade_smooth" => self.execute_core_command("model.shade_smooth").is_ok(),
                "shade_flat" => self.execute_core_command("model.shade_flat").is_ok(),
                "origin_to_geometry" => self.set_origin_geometry(),
                "origin_to_bottom" => self.set_origin_bottom(),
                "origin_to_cursor" => self.set_origin_cursor(),
                "origin_to_selection" => self.set_origin_selection(),
                "geometry_to_origin" => self.set_geometry_to_origin(),
                "toggle_edit_pivot" => self.toggle_edit_pivot(),
                "clear_location" => {
                    let _ = self.execute_core_command("model.clear_location");
                    self.reset_transform_fields();
                    true
                }
                "clear_rotation" => {
                    let _ = self.execute_core_command("model.clear_rotation");
                    self.reset_transform_fields();
                    true
                }
                "clear_scale" => {
                    let _ = self.execute_core_command("model.clear_scale");
                    self.reset_transform_fields();
                    true
                }
                "clear_all_transforms" => {
                    let _ = self.execute_core_command("model.clear_all_transforms");
                    self.reset_transform_fields();
                    true
                }
                _ => false,
            };
        }
        let id = menu.asset.to_string();
        self.close_context_menu();
        match action {
            "rename" => self.begin_rename(),
            "duplicate" => {
                self.select_asset_by_id(menu.asset);
                self.apply(UiIntent::DuplicateActiveAsset);
                true
            }
            "visibility" => {
                self.toggle_asset_visibility(&id);
                true
            }
            "lock" => {
                self.toggle_asset_lock(&id);
                true
            }
            "isolate" => {
                // Toggles isolation for the targeted asset / Alterna isolamento para o asset alvo
                self.select_asset_by_id(menu.asset);
                self.state.toggle_isolate();
                true
            }
            "move_up" => {
                // Moves target asset up in the scene list / Move o asset alvo para cima na lista da cena
                if let Some(idx) = self.state.project.find(menu.asset)
                    && idx > 0
                {
                    let _ = self.state.dispatch(&petunia_core::ReorderAssetCmd {
                        from: idx,
                        to: idx - 1,
                    });
                    return true;
                }
                false
            }
            "move_down" => {
                // Moves target asset down in the scene list / Move o asset alvo para baixo na lista da cena
                if let Some(idx) = self.state.project.find(menu.asset)
                    && idx + 1 < self.state.project.assets.len()
                {
                    let _ = self.state.dispatch(&petunia_core::ReorderAssetCmd {
                        from: idx,
                        to: idx + 1,
                    });
                    return true;
                }
                false
            }
            "frame" => {
                self.select_asset_by_id(menu.asset);
                let _ = self.state.dispatch_command("view.frame_selection");
                true
            }
            "origin_to_geometry" => {
                self.select_asset_by_id(menu.asset);
                self.set_origin_geometry()
            }
            "origin_to_bottom" => {
                self.select_asset_by_id(menu.asset);
                self.set_origin_bottom()
            }
            "origin_to_cursor" => {
                self.select_asset_by_id(menu.asset);
                self.set_origin_cursor()
            }
            "origin_to_selection" => {
                self.select_asset_by_id(menu.asset);
                self.set_origin_selection()
            }
            "geometry_to_origin" => {
                self.select_asset_by_id(menu.asset);
                self.set_geometry_to_origin()
            }
            "toggle_edit_pivot" => {
                self.select_asset_by_id(menu.asset);
                self.toggle_edit_pivot()
            }
            "clear_location" => {
                self.select_asset_by_id(menu.asset);
                let _ = self.execute_core_command("model.clear_location");
                self.reset_transform_fields();
                true
            }
            "clear_rotation" => {
                self.select_asset_by_id(menu.asset);
                let _ = self.execute_core_command("model.clear_rotation");
                self.reset_transform_fields();
                true
            }
            "clear_scale" => {
                self.select_asset_by_id(menu.asset);
                let _ = self.execute_core_command("model.clear_scale");
                self.reset_transform_fields();
                true
            }
            "clear_all_transforms" => {
                self.select_asset_by_id(menu.asset);
                let _ = self.execute_core_command("model.clear_all_transforms");
                self.reset_transform_fields();
                true
            }
            "delete" => {
                if let Some(profile_id) = self.state.session.active_shape {
                    let _ = self
                        .state
                        .dispatch(&petunia_core::DeleteProfileCmd { profile_id });
                    self.state.session.active_shape = None;
                    self.state.session.planar_selection.point = None;
                    self.profile_drag_target = None;
                    self.state.set_status(
                        self.state
                            .t_id(petunia_config::text_id::STATUS_SHAPE_DELETED),
                    );
                    self.state.mark_dirty();
                    true
                } else if self
                    .state
                    .project
                    .project
                    .profiles
                    .iter()
                    .any(|p| p.id == menu.asset)
                {
                    let pid = menu.asset;
                    let _ = self
                        .state
                        .dispatch(&petunia_core::DeleteProfileCmd { profile_id: pid });
                    if self.state.session.active_shape == Some(pid) {
                        self.state.session.active_shape = None;
                    }
                    self.state.set_status(
                        self.state
                            .t_id(petunia_config::text_id::STATUS_SHAPE_DELETED),
                    );
                    self.state.mark_dirty();
                    true
                } else {
                    self.select_asset_by_id(menu.asset);
                    self.apply(UiIntent::DeleteActiveAsset);
                    true
                }
            }
            _ => false,
        }
    }
}
