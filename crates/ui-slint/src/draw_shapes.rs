//! DRAW — formas persistentes sempre visíveis, nós inseridos depois de fechar e
//! conversão ponto reto ↔ curva (cap. 02, ADR 007).
//!
//! O documento já guarda cada perfil como `ProfileResource` + `SplineResource`.
//! Este módulo só decide **como o bridge os mostra e os edita**:
//!
//! - **Sempre visíveis** (`region_shapes_commands`, `profile_outline_commands`):
//!   as regiões fechadas aparecem com uma tinta leve e os perfis que não estão
//!   em edição mostram o contorno. Sem isso, clicar fora da forma fazia o perfil
//!   sumir da viewport.
//! - **Reativar** (`profile_hit_inactive`): clicar no contorno de um perfil
//!   inativo o torna o perfil em edição.
//! - **Inserir nó** (`profile_segment_hit`, `insert_profile_node`): clicar na
//!   aresta do perfil ativo (aberto ou fechado) divide o segmento sem mudar a
//!   forma e já permite arrastar o novo ponto.
//! - **Reto ↔ curva** (`profile_set_point_curved`): vale para um ponto ou para
//!   todos, em qualquer forma (polígono, retângulo, círculo).

use glam::Vec3;

use crate::projection::project_world_point;
use crate::{ModelingMode, PetuniaViewport, ProfileEditGesture, ProfileHitTarget, SlintUiBridge};
use petunia_core::Workspace;

/// Raio (px) para acertar uma aresta ou o contorno de um perfil.
pub(crate) const SHAPE_HIT_RADIUS_PX: f32 = 10.0;
/// Fator das alças ao curvar um ponto (fração da distância aos vizinhos).
const CURVE_FACTOR: f64 = 0.25;
/// Tolerância de tesselação das linhas persistentes.
const OUTLINE_TOLERANCE: f64 = 0.01;
/// Teto de pontos por quadro nos overlays de formas.
const MAX_OVERLAY_POINTS: usize = 40_000;

/// Polilinha de mundo de um perfil do documento.
pub(crate) struct ProfileLine {
    pub id: uuid::Uuid,
    pub world: Vec<Vec3>,
    pub closed: bool,
    pub hole: bool,
}

/// Regiões e contornos derivados do documento, recalculados por revisão.
pub(crate) struct ShapeCache {
    pub revision: [u64; 11],
    pub planes: petunia_core::RegionPlanes,
    pub lines: Vec<ProfileLine>,
}

/// Aresta do perfil ativo sob o cursor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SegmentHit {
    pub segment: usize,
    pub parameter: f64,
    pub distance: f32,
}

/// Menor distância de `p` ao segmento `a-b` e a fração (0..1) do pé da perpendicular.
pub(crate) fn point_segment_distance(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> (f32, f32) {
    let ab = [b[0] - a[0], b[1] - a[1]];
    let len2 = ab[0] * ab[0] + ab[1] * ab[1];
    let t = if len2 < 1e-9 {
        0.0
    } else {
        (((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1]) / len2).clamp(0.0, 1.0)
    };
    let foot = [a[0] + ab[0] * t, a[1] + ab[1] * t];
    ((p[0] - foot[0]).hypot(p[1] - foot[1]), t)
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Sólo o DRAW mostra as formas persistentes (POLY trabalha em componentes).
    fn draw_shapes_visible(&self) -> bool {
        self.state.workspace == Workspace::Model && self.modeling_mode == ModelingMode::Draw
    }

    /// Regiões e contornos em cache; recalcula só quando o documento muda.
    pub(crate) fn shape_cache(&self) -> std::cell::Ref<'_, ShapeCache> {
        // Só perfis e splines importam: pintar (cor/textura) não recalcula o arranjo.
        let clock = self.state.project.project.revision_clock();
        let mut revision = [0u64; 11];
        revision[0] = self.state.project.project.profiles.len() as u64;
        revision[9] = clock[9];
        revision[10] = clock[10];
        {
            let mut slot = self.region_planes_cache.borrow_mut();
            if slot.as_ref().is_none_or(|cache| cache.revision != revision) {
                *slot = Some(self.build_shape_cache(revision));
            }
        }
        std::cell::Ref::map(self.region_planes_cache.borrow(), |slot| {
            slot.as_ref().expect("cache preenchido acima")
        })
    }

    fn build_shape_cache(&self, revision: [u64; 11]) -> ShapeCache {
        let project = &self.state.project.project;
        let mut lines = Vec::new();
        for profile in &project.profiles {
            let Some(spline) = project.get_spline(profile.spline_id) else {
                continue;
            };
            let Ok(table) = spline.arc_length_table(OUTLINE_TOLERANCE) else {
                continue;
            };
            let wp = profile.workplane;
            let [origin, right, up] =
                [wp.origin, wp.right, wp.up].map(|v| glam::DVec3::from_array(v).as_vec3());
            if !(origin.is_finite() && right.is_finite() && up.is_finite()) {
                continue;
            }
            let to_world = |p: [f64; 2]| origin + right * p[0] as f32 + up * p[1] as f32;
            lines.push(ProfileLine {
                id: profile.id,
                world: table
                    .polyline()
                    .into_iter()
                    .map(|p| to_world([p[0], p[1]]))
                    .collect(),
                closed: spline.closed,
                hole: false,
            });
            for hole in &profile.holes {
                lines.push(ProfileLine {
                    id: profile.id,
                    world: hole.iter().copied().map(to_world).collect(),
                    closed: true,
                    hole: true,
                });
            }
        }
        ShapeCache {
            revision,
            planes: self.state.profile_region_planes(),
            lines,
        }
    }

    /// Escreve um anel projetado e recortado no plano próximo; `false` se nada aparece.
    fn write_ring(
        &self,
        points: impl Iterator<Item = Vec3>,
        close: bool,
        out: &mut String,
    ) -> bool {
        crate::projection::write_clipped_path(
            &self.state.session.camera,
            self.viewport_size,
            points,
            close,
            out,
        )
    }

    /// Todas as regiões fechadas (tinta leve), para a forma nunca se perder.
    pub(crate) fn region_shapes_commands(&self) -> String {
        if !self.draw_shapes_visible() || self.viewport_size[0] <= 1.0 {
            return String::new();
        }
        let cache = self.shape_cache();
        let mut out = String::new();
        let mut budget = MAX_OVERLAY_POINTS;
        for (plane, regions) in &cache.planes {
            for region in regions {
                for ring in std::iter::once(&region.outer).chain(region.holes.iter()) {
                    if ring.len() > budget {
                        return out;
                    }
                    budget -= ring.len();
                    self.write_ring(ring.iter().map(|p| plane.to_world(*p)), true, &mut out);
                }
            }
        }
        out
    }

    /// Contorno dos perfis que não estão em edição (abertos ou fechados).
    pub(crate) fn profile_outline_commands(&self) -> String {
        if !self.draw_shapes_visible() || self.viewport_size[0] <= 1.0 {
            return String::new();
        }
        let cache = self.shape_cache();
        let mut out = String::new();
        let mut budget = MAX_OVERLAY_POINTS;
        for line in &cache.lines {
            if Some(line.id) == self.active_profile_id && !line.hole {
                continue;
            }
            if line.world.len() > budget {
                break;
            }
            budget -= line.world.len();
            self.write_ring(line.world.iter().copied(), line.closed, &mut out);
        }
        out
    }

    /// Comandos e posição de tela da Depth Handle sobre a forma fechada ativa em DRAW.
    pub(crate) fn depth_handle_commands(&self) -> (String, Option<[f32; 2]>, String) {
        if !self.draw_shapes_visible() || self.viewport_size[0] <= 1.0 {
            return (String::new(), None, String::new());
        }
        let Some((profile, spline)) = self.active_profile_resources() else {
            return (String::new(), None, String::new());
        };
        if !spline.closed || spline.points.len() < 3 {
            return (String::new(), None, String::new());
        }
        let wp = profile.workplane;
        let [origin, right, up, normal] =
            [wp.origin, wp.right, wp.up, wp.normal].map(|v| glam::DVec3::from_array(v).as_vec3());
        let count = spline.points.len() as f32;
        let sum: (f64, f64) = spline.points.iter().fold((0.0, 0.0), |acc, p| {
            (acc.0 + p.position[0], acc.1 + p.position[1])
        });
        let (cx, cy) = (sum.0 as f32 / count, sum.1 as f32 / count);
        let center_world = origin + right * cx + up * cy;
        let depth = (self.state.profile.depth as f32).max(0.1);
        let top_world = center_world + normal * depth;

        let camera = &self.state.session.camera;
        let Some(base_screen) = project_world_point(camera, self.viewport_size, center_world)
        else {
            return (String::new(), None, String::new());
        };
        let Some(top_screen) = project_world_point(camera, self.viewport_size, top_world) else {
            return (String::new(), None, String::new());
        };

        let (bx, by) = (base_screen[0], base_screen[1]);
        let (tx, ty) = (top_screen[0], top_screen[1]);
        let mut out = format!("M {bx:.1} {by:.1} L {tx:.1} {ty:.1} ");
        out.push_str(&format!(
            "M {bx:.1} {by:.1} m -4 0 a 4 4 0 1 0 8 0 a 4 4 0 1 0 -8 0 "
        ));
        out.push_str(&format!(
            "M {tx:.1} {:.1} L {:.1} {ty:.1} L {tx:.1} {:.1} L {:.1} {ty:.1} Z",
            ty - 6.0,
            tx + 6.0,
            ty + 6.0,
            tx - 6.0
        ));

        let label = format!("{:.2} m", depth);
        (out, Some(top_screen), label)
    }

    /// Perfil inativo cujo contorno passa perto do cursor (px da viewport).
    pub(crate) fn profile_hit_inactive(&self, screen: [f32; 2]) -> Option<uuid::Uuid> {
        let cache = self.shape_cache();
        let camera = &self.state.session.camera;
        let mut best: Option<(uuid::Uuid, f32)> = None;
        for line in &cache.lines {
            if Some(line.id) == self.active_profile_id {
                continue;
            }
            let projected: Vec<Option<[f32; 2]>> = line
                .world
                .iter()
                .map(|p| project_world_point(camera, self.viewport_size, *p))
                .collect();
            let mut pairs: Vec<(usize, usize)> = (1..projected.len()).map(|i| (i - 1, i)).collect();
            if line.closed && projected.len() > 2 {
                pairs.push((projected.len() - 1, 0));
            }
            for (i, j) in pairs {
                if let (Some(a), Some(b)) = (projected[i], projected[j]) {
                    let (distance, _) = point_segment_distance(screen, a, b);
                    if distance <= SHAPE_HIT_RADIUS_PX
                        && best.is_none_or(|(_, current)| distance < current)
                    {
                        best = Some((line.id, distance));
                    }
                }
            }
        }
        best.map(|(id, _)| id)
    }

    /// Torna `profile_id` o perfil em edição (plano de trabalho incluído).
    pub(crate) fn activate_profile(&mut self, profile_id: uuid::Uuid) -> bool {
        if self.profile_volume_mode.is_some() {
            return false;
        }
        let Some(profile) = self.state.project.project.get_profile(profile_id) else {
            return false;
        };
        let wp = profile.workplane;
        self.state.profile.origin = wp.origin.map(|v| v as f32);
        self.state.profile.right = wp.right.map(|v| v as f32);
        self.state.profile.up = wp.up.map(|v| v as f32);
        self.state.profile.normal = wp.normal.map(|v| v as f32);
        self.state.profile.workplane_locked = true;
        self.active_profile_id = Some(profile_id);
        self.profile_selected_point = None;
        self.profile_drag_target = None;
        self.profile_edit_gesture = None;
        self.state.mark_dirty();
        true
    }

    /// Aresta do perfil ativo sob o cursor (px da viewport).
    pub(crate) fn profile_segment_hit(&self, screen: [f32; 2]) -> Option<SegmentHit> {
        const SAMPLES: usize = 24;
        let (profile, spline) = self.active_profile_resources()?;
        let wp = profile.workplane;
        let [origin, right, up] =
            [wp.origin, wp.right, wp.up].map(|v| glam::DVec3::from_array(v).as_vec3());
        let camera = &self.state.session.camera;
        let mut best: Option<SegmentHit> = None;
        for segment in 0..spline.segment_count() {
            let mut previous: Option<([f32; 2], f64)> = None;
            for k in 0..=SAMPLES {
                let t = k as f64 / SAMPLES as f64;
                let Ok(p) = spline.evaluate_segment(segment, t) else {
                    previous = None;
                    continue;
                };
                let world = origin + right * p[0] as f32 + up * p[1] as f32;
                let here = project_world_point(camera, self.viewport_size, world);
                if let (Some((a, ta)), Some(b)) = (previous, here) {
                    let (distance, frac) = point_segment_distance(screen, a, b);
                    if distance <= SHAPE_HIT_RADIUS_PX
                        && best.is_none_or(|hit| distance < hit.distance)
                    {
                        best = Some(SegmentHit {
                            segment,
                            parameter: ta + (t - ta) * f64::from(frac),
                            distance,
                        });
                    }
                }
                previous = here.map(|s| (s, t));
            }
        }
        best
    }

    /// Divide o segmento sem mudar a forma e deixa o novo ponto pronto para arrastar.
    pub(crate) fn insert_profile_node(&mut self, hit: SegmentHit) -> Option<uuid::Uuid> {
        if self.active_profile_point_count() >= 512 {
            self.state.set_status("max 512 pts");
            return None;
        }
        let (_, spline) = self.active_profile_resources()?;
        let mut updated = spline.clone();
        let id = updated.split_segment(hit.segment, hit.parameter).ok()?;
        if let Err(error) = self
            .state
            .dispatch(&petunia_core::UpdateSplineCmd { spline: updated })
        {
            self.state.set_status(error.to_string());
            return None;
        }
        let (_, spline) = self.active_profile_resources()?;
        let gesture = ProfileEditGesture {
            spline_id: spline.id,
            point_id: id,
            point_before: spline.point(id).cloned(),
            revision_before: spline.revision,
        };
        self.profile_selected_point = Some(id);
        self.profile_drag_target = Some(ProfileHitTarget::Anchor(id));
        self.profile_edit_gesture = Some(gesture);
        if self.profile_volume_mode.is_some() {
            self.update_profile_volume_preview();
        }
        self.state.set_status("Point added");
        Some(id)
    }

    /// Reto (`false`) ou curvo (`true`) para um ponto ou para todos (`None`).
    pub(crate) fn profile_set_point_curved(
        &mut self,
        point: Option<uuid::Uuid>,
        curved: bool,
    ) -> bool {
        let Some((_, spline)) = self.active_profile_resources() else {
            return false;
        };
        let mut updated = spline.clone();
        let ids: Vec<uuid::Uuid> = match point {
            Some(id) => vec![id],
            None => updated.points.iter().map(|p| p.id).collect(),
        };
        let mut changed = false;
        for id in ids {
            changed |= updated.set_point_curved(id, curved, CURVE_FACTOR).is_ok();
        }
        if !changed {
            return false;
        }
        if !curved
            && updated
                .points
                .iter()
                .all(|p| p.handle_in == [0.0; 3] && p.handle_out == [0.0; 3])
        {
            updated.interpolation = petunia_core::SplineInterpolation::Polyline;
        }
        if let Err(error) = self
            .state
            .dispatch(&petunia_core::UpdateSplineCmd { spline: updated })
        {
            self.state.set_status(error.to_string());
            return false;
        }
        self.state.set_status(match (point.is_some(), curved) {
            (true, true) => "Point curved",
            (true, false) => "Point straightened",
            (false, true) => "Profile curves smoothed (Cubic Bézier)",
            (false, false) => "Profile corners sharpened",
        });
        if self.profile_volume_mode.is_some() {
            self.update_profile_volume_preview();
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_to_segment_reports_foot_fraction() {
        let (d, t) = point_segment_distance([5.0, 3.0], [0.0, 0.0], [10.0, 0.0]);
        assert!((d - 3.0).abs() < 1e-5);
        assert!((t - 0.5).abs() < 1e-5);
        let (d, t) = point_segment_distance([-4.0, 0.0], [0.0, 0.0], [10.0, 0.0]);
        assert!((d - 4.0).abs() < 1e-5 && t == 0.0);
    }
}
