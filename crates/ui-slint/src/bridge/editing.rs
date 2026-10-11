//! Edição de seleção: apagar/dissolver, copiar, colar, duplicar e inserir assets.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Exclui ou dissolve elementos selecionados ou o ativo ativo se em Object Mode.
    pub fn delete_or_dissolve_selection(&mut self) -> bool {
        if self.state.session.tools.active_tool == "draw_profile"
            && self.profile_selected_point.is_some()
        {
            return self.delete_selected_profile_point();
        }

        // Se houver um perfil ativo em DRAW sem ponto selecionado, apaga a forma 2D inteira!
        if let Some(profile_id) = self.active_profile_id
            && self
                .state
                .dispatch(&petunia_core::DeleteProfileCmd { profile_id })
                .is_ok()
        {
            self.active_profile_id = None;
            self.profile_selected_point = None;
            self.profile_drag_target = None;
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_SHAPE_DELETED),
            );
            self.state.mark_dirty();
            return true;
        }

        if self.state.selection_domain() != SelectionDomain::Object {
            // Sem componentes selecionados, age sobre o que está sob o cursor
            // (seleção pelo core, nunca escrevendo flags da malha daqui).
            self.state.sync_selection();
            self.state.select_hover_if_empty();
            let has_selection_now = self.state.project.active_mesh().is_some_and(|mesh| {
                mesh.verts.iter().any(|v| v.selected)
                    || mesh.faces.iter().any(|f| f.selected)
                    || !mesh.selected_edges.is_empty()
            });
            if !has_selection_now {
                return false;
            }
            self.state.sync_selection();
            if self.state.selection_domain() == SelectionDomain::Face
                || self
                    .state
                    .project
                    .active_mesh()
                    .is_some_and(|m| m.faces.iter().any(|f| f.selected))
            {
                self.state
                    .dispatch(&petunia_core::DeleteSelectionCmd)
                    .is_ok()
            } else {
                self.state
                    .dispatch(&petunia_core::DeleteOrDissolveSelectionCmd)
                    .is_ok()
            }
        } else {
            self.apply(UiIntent::DeleteActiveAsset);
            true
        }
    }

    /// Copia para a área de transferência a geometria selecionada (ou ativo em Object Mode).
    pub fn copy_selection(&mut self) -> bool {
        if self.state.selection_domain() != SelectionDomain::Object {
            let Some(mesh) = self.state.project.active_mesh() else {
                return false;
            };

            let sel_faces: Vec<&petunia_mesh::Face> =
                mesh.faces.iter().filter(|f| f.selected).collect();
            if !sel_faces.is_empty() {
                let mut vert_map: std::collections::HashMap<u32, u32> =
                    std::collections::HashMap::new();
                let mut new_verts = Vec::new();
                for f in &sel_faces {
                    for &v in &f.verts {
                        if !vert_map.contains_key(&v)
                            && let Some(src_v) = mesh.verts.get(v as usize)
                        {
                            let new_idx = new_verts.len() as u32;
                            vert_map.insert(v, new_idx);
                            new_verts.push(src_v.clone());
                        }
                    }
                }
                let mut new_faces = Vec::new();
                for f in &sel_faces {
                    let mut remapped_verts = Vec::new();
                    for &v in &f.verts {
                        if let Some(&new_v) = vert_map.get(&v) {
                            remapped_verts.push(new_v);
                        }
                    }
                    let mut new_face = (*f).clone();
                    new_face.verts = remapped_verts;
                    new_faces.push(new_face);
                }
                let sub_mesh = petunia_mesh::Mesh {
                    verts: new_verts,
                    faces: new_faces,
                    ..Default::default()
                };
                self.clipboard = Some(GeometryClipboard::Mesh(sub_mesh));
                self.state.set_status(
                    self.state
                        .t_id(petunia_config::text_id::STATUS_COPIED_SELECTED_FACES_TO_CLIPBOARD),
                );
                return true;
            }

            if !mesh.selected_edges.is_empty() {
                let mut vert_indices = std::collections::HashSet::new();
                for &(a, b) in &mesh.selected_edges {
                    vert_indices.insert(a);
                    vert_indices.insert(b);
                }
                let mut vert_map: std::collections::HashMap<u32, u32> =
                    std::collections::HashMap::new();
                let mut new_verts = Vec::new();
                for &v in &vert_indices {
                    if let Some(src_v) = mesh.verts.get(v as usize) {
                        let new_idx = new_verts.len() as u32;
                        vert_map.insert(v, new_idx);
                        new_verts.push(src_v.clone());
                    }
                }
                let mut new_faces = Vec::new();
                for f in &mesh.faces {
                    if f.verts.iter().all(|v| vert_indices.contains(v)) {
                        let mut remapped = Vec::new();
                        for &v in &f.verts {
                            if let Some(&new_v) = vert_map.get(&v) {
                                remapped.push(new_v);
                            }
                        }
                        let mut new_f = f.clone();
                        new_f.verts = remapped;
                        new_faces.push(new_f);
                    }
                }
                let mut new_selected_edges = std::collections::HashSet::new();
                for &(a, b) in &mesh.selected_edges {
                    if let (Some(&na), Some(&nb)) = (vert_map.get(&a), vert_map.get(&b)) {
                        new_selected_edges.insert((na.min(nb), na.max(nb)));
                    }
                }
                let sub_mesh = petunia_mesh::Mesh {
                    verts: new_verts,
                    faces: new_faces,
                    selected_edges: new_selected_edges,
                    ..Default::default()
                };
                self.clipboard = Some(GeometryClipboard::Mesh(sub_mesh));
                self.state.set_status(
                    self.state
                        .t_id(petunia_config::text_id::STATUS_COPIED_SELECTED_EDGES_TO_CLIPBOARD),
                );
                return true;
            }

            let sel_verts: Vec<(usize, &petunia_mesh::Vertex)> = mesh
                .verts
                .iter()
                .enumerate()
                .filter(|(_, v)| v.selected)
                .collect();
            if !sel_verts.is_empty() {
                let vert_indices: std::collections::HashSet<u32> =
                    sel_verts.iter().map(|(i, _)| *i as u32).collect();
                let mut vert_map: std::collections::HashMap<u32, u32> =
                    std::collections::HashMap::new();
                let mut new_verts = Vec::new();
                for (i, v) in &sel_verts {
                    let new_idx = new_verts.len() as u32;
                    vert_map.insert(*i as u32, new_idx);
                    new_verts.push((*v).clone());
                }
                let mut new_faces = Vec::new();
                for f in &mesh.faces {
                    if f.verts.iter().all(|v| vert_indices.contains(v)) {
                        let mut remapped = Vec::new();
                        for &v in &f.verts {
                            if let Some(&new_v) = vert_map.get(&v) {
                                remapped.push(new_v);
                            }
                        }
                        let mut new_f = f.clone();
                        new_f.verts = remapped;
                        new_faces.push(new_f);
                    }
                }
                let sub_mesh = petunia_mesh::Mesh {
                    verts: new_verts,
                    faces: new_faces,
                    ..Default::default()
                };
                self.clipboard = Some(GeometryClipboard::Mesh(sub_mesh));
                self.state.set_status(
                    self.state.t_id(
                        petunia_config::text_id::STATUS_COPIED_SELECTED_VERTICES_TO_CLIPBOARD,
                    ),
                );
                return true;
            }

            match self.state.hover {
                petunia_core::HoverTarget::Face(f_idx) => {
                    if let Some(f) = mesh.faces.get(f_idx) {
                        let mut new_verts = Vec::new();
                        let mut remapped = Vec::new();
                        for (new_i, &v) in f.verts.iter().enumerate() {
                            if let Some(src_v) = mesh.verts.get(v as usize) {
                                new_verts.push(src_v.clone());
                                remapped.push(new_i as u32);
                            }
                        }
                        let mut new_f = f.clone();
                        new_f.verts = remapped;
                        let sub_mesh = petunia_mesh::Mesh {
                            verts: new_verts,
                            faces: vec![new_f],
                            ..Default::default()
                        };
                        self.clipboard = Some(GeometryClipboard::Mesh(sub_mesh));
                        self.state.set_status(self.state.t_id(
                            petunia_config::text_id::STATUS_COPIED_HOVERED_FACE_TO_CLIPBOARD,
                        ));
                        return true;
                    }
                }
                petunia_core::HoverTarget::Edge(a, b) => {
                    if let (Some(va), Some(vb)) =
                        (mesh.verts.get(a as usize), mesh.verts.get(b as usize))
                    {
                        let mut new_edges = std::collections::HashSet::new();
                        new_edges.insert((0, 1));
                        let sub_mesh = petunia_mesh::Mesh {
                            verts: vec![va.clone(), vb.clone()],
                            selected_edges: new_edges,
                            ..Default::default()
                        };
                        self.clipboard = Some(GeometryClipboard::Mesh(sub_mesh));
                        self.state.set_status(self.state.t_id(
                            petunia_config::text_id::STATUS_COPIED_HOVERED_EDGE_TO_CLIPBOARD,
                        ));
                        return true;
                    }
                }
                petunia_core::HoverTarget::Vertex(v_idx) => {
                    if let Some(v) = mesh.verts.get(v_idx as usize) {
                        let sub_mesh = petunia_mesh::Mesh {
                            verts: vec![v.clone()],
                            ..Default::default()
                        };
                        self.clipboard = Some(GeometryClipboard::Mesh(sub_mesh));
                        self.state.set_status(self.state.t_id(
                            petunia_config::text_id::STATUS_COPIED_HOVERED_VERTEX_TO_CLIPBOARD,
                        ));
                        return true;
                    }
                }
                _ => {}
            }

            false
        } else {
            let Some(asset) = self.state.project.active() else {
                return false;
            };
            self.clipboard = Some(GeometryClipboard::Asset(Box::new(asset.clone())));
            self.state.set_status(crate::tr::fill(
                &self
                    .state
                    .t_id(petunia_config::text_id::STATUS_COPIED_TO_CLIPBOARD),
                &[("0", asset.name.to_string())],
            ));
            true
        }
    }

    /// Cola a geometria da área de transferência como um novo objeto separado.
    pub fn paste_clipboard(&mut self) -> bool {
        let Some(clipboard_data) = self.clipboard.clone() else {
            return false;
        };
        match clipboard_data {
            GeometryClipboard::Mesh(mut mesh) => {
                let base_name = self
                    .state
                    .project
                    .active()
                    .map(|a| a.name.clone())
                    .unwrap_or_else(|| "part".to_string());
                let new_name = format!("{base_name}_part");

                mesh.select_all();

                self.state
                    .checkpoint(&format!("paste geometry: {new_name}"));
                self.state.project.add(&new_name, mesh);
                let new_index = self.state.project.assets.len() - 1;
                self.state.set_selection_domain(SelectionDomain::Object);
                self.state.select_object(Some(new_index), false);
                self.state.emit_mesh_changed();
                self.state.mark_dirty();
                self.state.set_status(crate::tr::fill(
                    &self
                        .state
                        .t_id(petunia_config::text_id::STATUS_PASTED_SEPARATE_OBJECT),
                    &[("new_name", new_name.to_string())],
                ));
                true
            }
            GeometryClipboard::Asset(asset) => {
                let mut copy = asset.duplicate();
                copy.name = format!("{}_copy", asset.name);
                for v in &mut copy.mesh.verts {
                    v.pos[0] += 0.5;
                }
                let copy_name = copy.name.clone();
                self.state.checkpoint(&format!("paste asset: {copy_name}"));
                self.state.project.assets.push(copy);
                let new_index = self.state.project.assets.len() - 1;
                self.state.set_selection_domain(SelectionDomain::Object);
                self.state.select_object(Some(new_index), false);
                self.state.emit_mesh_changed();
                self.state.mark_dirty();
                self.state.set_status(crate::tr::fill(
                    &self
                        .state
                        .t_id(petunia_config::text_id::STATUS_PASTED_OBJECT),
                    &[("copy_name", copy_name.to_string())],
                ));
                true
            }
        }
    }

    /// Duplica a seleção: elementos na malha em Edit Mode, ou ativo em Object Mode.
    pub fn duplicate_selection(&mut self) -> bool {
        if self.state.selection_domain() != SelectionDomain::Object {
            self.state.sync_selection();
            self.state.select_hover_if_empty();
            self.state
                .dispatch(&petunia_core::DuplicateSelectionCmd)
                .is_ok()
        } else {
            self.state
                .dispatch(&petunia_core::DuplicateAssetCmd { asset_index: None })
                .is_ok()
        }
    }

    /// Instancia uma cópia do asset da biblioteca no cursor 3D.
    ///
    /// É o caminho que o comando canônico `model.instantiate_asset` não tinha:
    /// ele exige um `asset_id`, então a palette não consegue disparar sozinha.
    pub fn place_asset(&mut self, id: &str) -> bool {
        let Ok(asset) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.state.set_selection_domain(SelectionDomain::Object);
        self.sync_viewport_context();
        if !self.state.instantiate_asset_by_id(asset, None) {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_ASSET_NOT_FOUND_IN_PROJECT_LIBRARY),
            );
            return false;
        }
        self.state.mark_dirty();
        true
    }
}
