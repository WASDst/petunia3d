//! Snap, edição proporcional e marcadores de snap.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Marcador de snap do quadro: o da operação em curso ou, no Draw, o da
    /// pré-seleção.
    pub(crate) fn snap_marker_model(&self, width: f32, height: f32) -> projection::SnapMarkerModel {
        let marker = compute_snap_marker(&self.state, width, height);
        if marker.visible || self.state.session.tools.active_tool != "draw_profile" {
            return marker;
        }
        self.profile_hover_snap.map_or(marker, |hit| {
            projection::snap_marker_at(&self.state, hit.point, Some(hit.kind), width, height)
        })
    }

    /// Uma única passada de snap em espaço de tela (P3D-040, ADR 007 Onda 3).
    ///
    /// `cursor` em px lógicos da viewport. Durante uma operação, a malha de
    /// origem congelada é o alvo e os vértices que se movem são ignorados;
    /// fora dela, a malha ativa inteira.
    pub(crate) fn screen_snap(
        &self,
        cursor: glam::Vec2,
        mask: petunia_core::SnapMask,
        anchor: Option<petunia_core::SnapAnchor>,
        grid: Option<petunia_core::SnapGrid>,
    ) -> Option<petunia_core::ScreenSnapHit> {
        self.screen_snap_guided(cursor, mask, anchor, grid, None, None)
    }

    /// Passada de snap com guias extras a partir da âncora: paralela e
    /// perpendicular a `reference` e ângulos em passos de `angle_step` graus.
    ///
    /// Todos os objetos visíveis são alvos (snap entre objetos). Durante uma
    /// operação, a malha ativa entra pela origem congelada sem os vértices
    /// que se movem.
    pub(crate) fn screen_snap_guided(
        &self,
        cursor: glam::Vec2,
        mask: petunia_core::SnapMask,
        anchor: Option<petunia_core::SnapAnchor>,
        grid: Option<petunia_core::SnapGrid>,
        reference: Option<glam::Vec3>,
        angle_step: Option<f32>,
    ) -> Option<petunia_core::ScreenSnapHit> {
        let accel = self.snap_accel();
        let session = &self.state.session;
        petunia_core::snap_screen_with(
            &petunia_core::ScreenSnapQuery {
                camera: &session.camera,
                viewport_pixels: glam::Vec2::from_array(self.viewport_size),
                cursor_pixels: cursor,
                mesh: None,
                moving: &[],
                anchor,
                grid,
                radius_pixels: session.snap_settings.radius_pixels,
                mask,
                xray: session.selects_through(),
                reference,
                angle_step_degrees: angle_step,
            },
            &accel,
        )
    }

    /// Geometria de snap em cache: por gesto durante uma operação (a origem
    /// congelada não muda) e por revisão de geometria fora dela.
    pub(crate) fn snap_accel(&self) -> std::sync::Arc<petunia_core::SnapAccel> {
        use std::hash::{Hash, Hasher};
        let modal = self.state.session.tools.modal.as_ref();
        let project = &self.state.project.project;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        match modal {
            Some(modal) => {
                (1u8, modal.id(), project.active, project.assets.len()).hash(&mut hasher)
            }
            None => (2u8, self.scene_geometry_key(), project.active).hash(&mut hasher),
        }
        (
            project.spline_revision,
            project.procedural_revision,
            self.profile_authoring_preview.is_some(),
        )
            .hash(&mut hasher);
        let key = hasher.finish();
        if let Some((cached, accel)) = self.snap_accel_cache.borrow().as_ref()
            && *cached == key
        {
            return std::sync::Arc::clone(accel);
        }
        puffin::profile_scope!("snap_accel_rebuild");
        let moving = modal
            .map(|modal| modal.moving_vertices())
            .unwrap_or_default();
        let meshes: Vec<(usize, std::borrow::Cow<'_, petunia_core::Mesh>)> = project
            .assets
            .iter()
            .enumerate()
            .filter(|(_, asset)| {
                asset.visible && !modal.is_some_and(|m| m.excludes_snap_object(asset.id))
            })
            .map(|(index, asset)| {
                let asset = modal
                    .and_then(|m| m.snap_source_asset(asset.id))
                    .unwrap_or(asset);
                // `SnapSource` não carrega matriz: o snap é de mundo, então as
                // malhas entram no mundo (índices preservados para `moving`).
                let mesh = match modal {
                    Some(modal) if index == project.active => {
                        if asset.transform.is_identity() {
                            std::borrow::Cow::Borrowed(modal.source_mesh())
                        } else {
                            let mut source = modal.source_mesh().clone();
                            source.transform_affine(asset.world_matrix());
                            std::borrow::Cow::Owned(source)
                        }
                    }
                    _ => asset.world_mesh(),
                };
                (index, mesh)
            })
            .collect();
        let sources: Vec<petunia_core::SnapSource<'_>> = meshes
            .iter()
            .map(|(index, mesh)| petunia_core::SnapSource {
                mesh,
                moving: if *index == project.active {
                    &moving
                } else {
                    &[]
                },
            })
            .collect();
        let accel = std::sync::Arc::new(crate::perf::measure("snap_accel_rebuild", || {
            let identities: Vec<_> = meshes
                .iter()
                .filter_map(|(index, _)| {
                    project.assets.get(*index).map(|a| {
                        (
                            a.id,
                            a.geometry_source.is_none() && !a.has_enabled_modifiers(),
                        )
                    })
                })
                .collect();
            let mut accel = petunia_core::SnapAccel::build_scoped(&sources, &identities);
            for shape in project.profiles.iter().filter(|s| s.visible) {
                if self
                    .profile_authoring_preview
                    .as_ref()
                    .is_some_and(|d| d.profile.id == shape.id)
                {
                    continue;
                }
                for id in std::iter::once(&shape.spline_id).chain(&shape.hole_splines) {
                    if let Some(spline) = project.get_spline(*id) {
                        accel.append_planar_spline(shape.id, spline, shape.workplane);
                    }
                }
            }
            for spline in project.splines.iter().filter(|spline| {
                !project.profiles.iter().any(|shape| {
                    shape.spline_id == spline.id || shape.hole_splines.contains(&spline.id)
                })
            }) {
                if let Ok(resolved) = project.resolved_spline(spline.id) {
                    accel.append_spatial_spline(spline.id, &resolved);
                }
            }
            accel
        }));
        *self.snap_accel_cache.borrow_mut() = Some((key, std::sync::Arc::clone(&accel)));
        accel
    }

    pub fn toggle_proportional_editing(&mut self) -> bool {
        self.state.session.proportional_editing = !self.state.session.proportional_editing;
        let enabled = self.state.session.proportional_editing;
        self.state.render.mark_dirty();
        self.state.set_status(if enabled {
            "Edição proporcional ativada"
        } else {
            "Edição proporcional desativada"
        });
        enabled
    }

    pub fn set_proportional_radius(&mut self, radius: f32) -> bool {
        if !radius.is_finite() {
            return false;
        }
        let r = radius.clamp(0.01, 100.0);
        self.state.session.proportional_settings.radius = r;
        self.state.render.mark_dirty();
        true
    }

    pub fn adjust_proportional_radius(&mut self, delta: f32) -> bool {
        if !delta.is_finite() {
            return false;
        }
        let current = self.state.session.proportional_settings.radius;
        let new_radius = (current + delta).clamp(0.05, 100.0);
        self.set_proportional_radius(new_radius);
        self.state.set_status(crate::tr::fill(
            &self
                .state
                .t_id(petunia_config::text_id::STATUS_PROPORTIONAL_RADIUS),
            &[("0", format!("{:.2}", new_radius))],
        ));
        true
    }

    pub fn set_proportional_falloff(&mut self, falloff_str: &str) -> bool {
        use petunia_core::ProportionalFalloff;
        let falloff = match falloff_str.to_lowercase().as_str() {
            "smooth" => ProportionalFalloff::Smooth,
            "linear" => ProportionalFalloff::Linear,
            "sphere" => ProportionalFalloff::Sphere,
            "sharp" => ProportionalFalloff::Sharp,
            "constant" => ProportionalFalloff::Constant,
            _ => ProportionalFalloff::Smooth,
        };
        self.state.session.proportional_settings.falloff = falloff;
        self.state.render.mark_dirty();
        true
    }

    pub fn toggle_snap_enabled(&mut self) -> bool {
        self.state.session.snap_settings.enabled = !self.state.session.snap_settings.enabled;
        let enabled = self.state.session.snap_settings.enabled;
        self.state.render.mark_dirty();
        self.state.set_status(if enabled {
            "Snap magnético ativado"
        } else {
            "Snap magnético desativado"
        });
        enabled
    }

    pub fn set_snap_target(&mut self, target_str: &str) -> bool {
        use petunia_core::SnapTarget;
        let target = match target_str.to_lowercase().as_str() {
            "smart" => SnapTarget::Smart,
            "grid" => SnapTarget::Grid,
            "increment" => SnapTarget::Increment,
            "vertex" | "point" => SnapTarget::Vertex,
            "edge" => SnapTarget::Edge,
            "face" => SnapTarget::Face,
            _ => return false,
        };
        self.state.session.snap_settings.target = target;
        self.state.render.mark_dirty();
        self.state.set_status(crate::tr::fill(
            &self.state.t_id(petunia_config::text_id::STATUS_SNAP_TARGET),
            &[("0", target.label().to_string())],
        ));
        true
    }
}

#[cfg(test)]
mod origin_frame_tests {
    use super::*;
    use glam::{Quat, Vec2, Vec3};
    use petunia_core::{EditMode, SnapMask, SnapTarget, modal::ModalKind};

    #[test]
    fn origin_drag_snaps_to_stationary_surface_using_frozen_frame_on_first_query() {
        let mut bridge = SlintUiBridge::new(AppState::new("en"), Software3dViewport::new(800, 600));
        bridge.resize_viewport(800, 600);
        bridge.state.set_edit_mode(EditMode::Object);
        bridge
            .state
            .project
            .active_mut()
            .unwrap()
            .set_object_transform(petunia_project::ObjectTransform::from_trs(
                Vec3::new(0.4, 0.2, 0.0),
                Quat::from_rotation_y(0.3),
                Vec3::splat(0.8),
            ));
        let points = bridge.state.project.active().unwrap().world_positions();
        bridge.state.session.snap_settings.enabled = true;
        bridge.state.session.snap_enabled = true;
        bridge.state.toggle_edit_pivot();
        bridge.state.begin_modal(ModalKind::Move).unwrap();
        bridge
            .state
            .update_modal(Vec3::new(3.0, 2.0, 1.0), 0.0)
            .unwrap();
        // Cache is first built after the origin/mesh have already been rebased.
        let matrix = bridge.state.session.camera.view_proj();
        let hits = points
            .into_iter()
            .filter(|point| {
                let clip = matrix * point.extend(1.0);
                let ndc = clip.truncate() / clip.w;
                let pixel = Vec2::new((ndc.x + 1.0) * 400.0, (1.0 - ndc.y) * 300.0);
                bridge
                    .screen_snap(pixel, SnapMask::for_target(SnapTarget::Vertex), None, None)
                    .is_some_and(|hit| hit.point.distance(*point) < 1e-4)
            })
            .count();
        assert!(
            hits >= 2,
            "stationary corners must remain usable snap targets"
        );
    }

    #[test]
    fn moving_object_group_does_not_snap_to_its_other_selected_objects() {
        let mut bridge = SlintUiBridge::new(AppState::new("en"), Software3dViewport::new(800, 600));
        bridge.resize_viewport(800, 600);
        bridge.state.set_edit_mode(EditMode::Object);
        let first = bridge.state.project.active().unwrap().id;
        let mut other = bridge.state.project.active().unwrap().clone();
        other.id = uuid::Uuid::new_v4();
        let second = other.id;
        let points = other.world_positions();
        bridge.state.project.assets.push(other);
        bridge.state.session.selection.assets = vec![first, second];
        bridge.state.session.snap_settings.enabled = true;
        bridge.state.session.snap_enabled = true;
        bridge.state.begin_modal(ModalKind::Move).unwrap();
        let matrix = bridge.state.session.camera.view_proj();
        for point in points {
            let clip = matrix * point.extend(1.0);
            let ndc = clip.truncate() / clip.w;
            let pixel = Vec2::new((ndc.x + 1.0) * 400.0, (1.0 - ndc.y) * 300.0);
            assert!(
                bridge
                    .screen_snap(pixel, SnapMask::for_target(SnapTarget::Vertex), None, None)
                    .is_none()
            );
        }
    }
}
