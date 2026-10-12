//! POLY pen: alvo, clique, split de aresta e preview.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Elemento sob o cursor para o Poly Pen, filtrado pelo modo (Auto:
    /// ponto > aresta > face), com as tolerâncias em pixels do picking de
    /// seleção. No modo Polygons só arestas de borda contam (início da pintura).
    pub(crate) fn poly_pen_target(&self, pixel: [f32; 2]) -> petunia_core::HoverTarget {
        let [width, height] = self.viewport_size;
        if width <= 1.0 || height <= 1.0 {
            return petunia_core::HoverTarget::None;
        }
        let (x, y) = (pixel[0] / width, pixel[1] / height);
        let polygons = self.poly_pen_mode == PolyPenMode::Polygons;
        self.poly_pen_mode
            .domains()
            .iter()
            .map(|&domain| self.pick_target_for_domain(domain, x, y))
            .find(|target| match *target {
                petunia_core::HoverTarget::Edge(a, b) if polygons => self.is_border_edge(a, b),
                _ => target.is_some(),
            })
            .unwrap_or_default()
    }

    pub(crate) fn is_border_edge(&self, a: u32, b: u32) -> bool {
        self.state
            .project
            .active_mesh()
            .is_some_and(|mesh| mesh.edge_faces(a, b).len() == 1)
    }

    /// Ponto no plano de desenho do Poly Pen sob o pixel: o plano de trabalho
    /// travado (cap. 05) ou, sem ele, o plano de frente para a câmera por
    /// `anchor`.
    pub(crate) fn poly_pen_plane_point(
        &self,
        pixel: [f32; 2],
        anchor: glam::Vec3,
    ) -> Option<glam::Vec3> {
        let [width, height] = self.viewport_size;
        if width <= 1.0 || height <= 1.0 {
            return None;
        }
        let camera = &self.state.session.camera;
        let ndc = glam::Vec2::new(pixel[0] / width * 2.0 - 1.0, 1.0 - pixel[1] / height * 2.0);
        let profile = &self.state.profile;
        let workplane_normal = glam::Vec3::from(profile.right).cross(glam::Vec3::from(profile.up));
        let (point, normal) =
            if profile.workplane_locked && workplane_normal.length_squared() > 1.0e-8 {
                (
                    glam::Vec3::from(profile.origin),
                    workplane_normal.normalize(),
                )
            } else {
                (anchor, camera.forward())
            };
        let (origin, direction) = camera.ray(ndc.x, ndc.y);
        let denominator = direction.dot(normal);
        if denominator.abs() < 1.0e-6 {
            return None;
        }
        let distance = (point - origin).dot(normal) / denominator;
        let hit = origin + direction * distance;
        (distance > 0.0 && hit.is_finite()).then_some(hit)
    }

    /// Ponto de mundo para um ponto novo do Poly Pen: snap (se ligado), face
    /// sob o cursor ou plano de frente para a câmera pelo último ponto.
    pub(crate) fn poly_pen_world_point(&self, pixel: [f32; 2]) -> Option<glam::Vec3> {
        let [width, height] = self.viewport_size;
        if width <= 1.0 || height <= 1.0 {
            return None;
        }
        // Tudo aqui é mundo: o core converte `PenPoint::New` para o local.
        let mesh = crate::projection::active_mesh_in_world(&self.state)?;
        let mesh = &*mesh;
        let last = self.poly_pen_points.last().map(|point| match *point {
            petunia_core::PenPoint::Existing(index) => mesh
                .verts
                .get(index as usize)
                .map_or(glam::Vec3::ZERO, |v| v.vec()),
            petunia_core::PenPoint::New(position) => glam::Vec3::from(position),
        });
        if self.state.session.snap_enabled {
            let mask = petunia_core::SnapMask::for_target(self.state.session.snap_settings.target);
            if let Some(hit) = self.screen_snap(
                glam::Vec2::from_array(pixel),
                mask,
                last.map(petunia_core::SnapAnchor::world),
                None,
            ) {
                return Some(hit.point);
            }
        }
        let camera = &self.state.session.camera;
        let ndc = glam::Vec2::new(pixel[0] / width * 2.0 - 1.0, 1.0 - pixel[1] / height * 2.0);
        if let Some(hit) = petunia_core::picking::pick_mesh(
            mesh,
            camera,
            glam::Vec2::new(width, height),
            ndc,
            petunia_core::SelectMode::Face,
            false,
        ) {
            return Some(hit.position);
        }
        let anchor = last.unwrap_or_else(|| mesh.center());
        self.poly_pen_plane_point(pixel, anchor)
    }

    /// Clique do Poly Pen: o modificador alternativo derrete o ponto; no modo
    /// Edges (ou no Auto, sem coleta) clicar numa aresta insere um ponto; nos
    /// demais, coleta um ponto do polígono (clicar no primeiro ponto fecha).
    /// Pontos existentes são reaproveitados em qualquer modo de desenho.
    pub(crate) fn poly_pen_click(&mut self, at: [f32; 2]) -> bool {
        let mode = self.poly_pen_mode;
        let target = self.poly_pen_target(at);
        if self.tool_press_alternate {
            if matches!(mode, PolyPenMode::Auto | PolyPenMode::Points)
                && let petunia_core::HoverTarget::Vertex(point) = target
            {
                if let Err(error) = self.state.poly_pen_melt_point(point) {
                    self.state.set_status(error.to_string());
                }
                return true;
            }
            return false;
        }
        let splits_edges = mode == PolyPenMode::Edges
            || (mode == PolyPenMode::Auto && self.poly_pen_points.is_empty());
        if splits_edges && let petunia_core::HoverTarget::Edge(a, b) = target {
            return self.poly_pen_split_edge_at(a, b, at);
        }
        if mode == PolyPenMode::Edges {
            return false;
        }
        let target = match mode {
            PolyPenMode::Polygons => {
                let [width, height] = self.viewport_size;
                self.pick_target_for_domain(SelectionDomain::Vertex, at[0] / width, at[1] / height)
            }
            _ => target,
        };
        let point = match target {
            petunia_core::HoverTarget::Vertex(index) => petunia_core::PenPoint::Existing(index),
            _ => match self.poly_pen_world_point(at) {
                Some(position) => petunia_core::PenPoint::New(position.to_array()),
                None => return false,
            },
        };
        if self.poly_pen_points.len() >= 3 && self.poly_pen_points.first() == Some(&point) {
            return self.poly_pen_close();
        }
        if self.poly_pen_points.contains(&point) {
            return false;
        }
        self.poly_pen_points.push(point);
        self.state.mark_dirty();
        true
    }

    /// Divide a aresta `a–b` no ponto mais próximo do clique (px da viewport).
    pub(crate) fn poly_pen_split_edge_at(&mut self, a: u32, b: u32, at: [f32; 2]) -> bool {
        // Projeção na tela: extremos no mundo.
        let (pa, pb) = match crate::projection::active_mesh_in_world(&self.state) {
            Some(mesh) => match (mesh.verts.get(a as usize), mesh.verts.get(b as usize)) {
                (Some(va), Some(vb)) => (va.vec(), vb.vec()),
                _ => return false,
            },
            None => return false,
        };
        let camera = &self.state.session.camera;
        let t = match (
            project_world_point(camera, self.viewport_size, pa),
            project_world_point(camera, self.viewport_size, pb),
        ) {
            (Some(sa), Some(sb)) => draw_shapes::point_segment_distance(at, sa, sb).1,
            _ => 0.5,
        };
        match self.state.poly_pen_split_edge(a, b, t) {
            Ok(_) => {
                self.sync_viewport_context();
                true
            }
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    /// Fecha o polígono coletado (Enter ou clique no primeiro ponto).
    pub(crate) fn poly_pen_close(&mut self) -> bool {
        let points = std::mem::take(&mut self.poly_pen_points);
        match self.state.poly_pen_add_polygon(&points) {
            Ok(_) => {
                self.sync_viewport_context();
                true
            }
            Err(error) => {
                // Mantém os pontos para o usuário corrigir.
                self.poly_pen_points = points;
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    /// Arrasto do Poly Pen: move o elemento sob o cursor (sem selecionar
    /// antes) ou, com o modificador alternativo sobre uma aresta de borda,
    /// extruda a aresta. Durante a coleta de pontos, arrastar não edita.
    pub(crate) fn begin_poly_pen_drag(&mut self, anchor: [f32; 2]) -> bool {
        if !self.poly_pen_points.is_empty() {
            return false;
        }
        let target = self.poly_pen_target(anchor);
        // Modo Polygons: arrastar a partir de uma aresta de borda pinta quads.
        if self.poly_pen_mode == PolyPenMode::Polygons
            && !self.tool_press_alternate
            && let petunia_core::HoverTarget::Edge(a, b) = target
        {
            return match self.state.begin_poly_pen_strip(a, b) {
                Ok(strip) => {
                    self.poly_pen_strip = Some(strip);
                    self.tool_gesture = Some(ToolGesture::PolyPenStrip);
                    true
                }
                Err(error) => {
                    self.state.set_status(error.to_string());
                    false
                }
            };
        }
        if self.tool_press_alternate
            && let petunia_core::HoverTarget::Edge(a, b) = target
        {
            if let Err(error) = self.state.begin_poly_pen_edge_extrude(a, b) {
                self.state.set_status(error.to_string());
                return false;
            }
            self.start_viewport_drag(TransformKind::Position, anchor[0], anchor[1]);
            self.tool_gesture = Some(ToolGesture::Transform { gizmo: false });
            self.sync_viewport_context();
            return true;
        }
        let domain = match target {
            petunia_core::HoverTarget::Vertex(_) => SelectionDomain::Vertex,
            petunia_core::HoverTarget::Edge(..) => SelectionDomain::Edge,
            petunia_core::HoverTarget::Face(_) => SelectionDomain::Face,
            _ => return false,
        };
        // O elemento arrastado passa a ser a seleção (visível), como no C4D.
        self.state.set_selection_domain(domain);
        let [width, height] = self.viewport_size;
        self.select_viewport_ext(anchor[0] / width, anchor[1] / height, false, false);
        self.sync_viewport_context();
        let begun = self.begin_viewport_transform(TransformKind::Position, anchor[0], anchor[1]);
        if begun {
            self.tool_gesture = Some(ToolGesture::Transform { gizmo: false });
        }
        begun
    }

    /// Contorno dos pontos coletados até o cursor, em px da viewport.
    pub(crate) fn poly_pen_preview_commands(&self) -> String {
        use std::fmt::Write as _;
        // Pontos existentes vão ao mundo; `PenPoint::New` já é do mundo.
        let Some(mesh) = crate::projection::active_mesh_in_world(&self.state) else {
            return String::new();
        };
        if self.poly_pen_points.is_empty() {
            return String::new();
        }
        let [width, height] = self.viewport_size;
        let matrix = self.state.session.camera.view_proj();
        let mut commands = String::new();
        for (index, point) in self.poly_pen_points.iter().enumerate() {
            let world = match *point {
                petunia_core::PenPoint::Existing(i) => match mesh.verts.get(i as usize) {
                    Some(vertex) => vertex.vec(),
                    None => return String::new(),
                },
                petunia_core::PenPoint::New(position) => glam::Vec3::from(position),
            };
            let clip = matrix * world.extend(1.0);
            if !clip.is_finite() || clip.w <= 0.05 {
                return String::new();
            }
            let x = (clip.x / clip.w * 0.5 + 0.5) * width;
            let y = (0.5 - clip.y / clip.w * 0.5) * height;
            let action = if index == 0 { 'M' } else { 'L' };
            let _ = write!(commands, "{action} {x:.2} {y:.2} ");
        }
        let [x, y] = self.pointer_position;
        let _ = write!(commands, "L {x:.2} {y:.2}");
        commands
    }

    /// Estende a pintura de faces até o cursor (no plano de desenho pela
    /// aresta da ponta).
    pub(crate) fn update_poly_pen_strip(&mut self, current: [f32; 2]) -> bool {
        let Some(mut strip) = self.poly_pen_strip.take() else {
            return false;
        };
        let (a, b) = strip.edge();
        // Âncora do plano no mundo: `poly_pen_strip_to` converte o alvo.
        let tip = crate::projection::active_mesh_in_world(&self.state).and_then(|mesh| {
            Some((mesh.verts.get(a as usize)?.vec() + mesh.verts.get(b as usize)?.vec()) * 0.5)
        });
        let changed = match tip.and_then(|tip| self.poly_pen_plane_point(current, tip)) {
            Some(target) => match self.state.poly_pen_strip_to(&mut strip, target) {
                Ok(added) => added > 0,
                Err(error) => {
                    self.state.set_status(error.to_string());
                    false
                }
            },
            None => false,
        };
        self.poly_pen_strip = Some(strip);
        if changed {
            self.sync_viewport_context();
        }
        changed
    }
}
