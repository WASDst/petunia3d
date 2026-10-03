//! Regiões de perfis e Push/Pull de região (ADR 007, Onda 4; capítulo 02).
//!
//! Os perfis persistentes do documento são agrupados por plano; em cada plano,
//! o arranjo planar ([`petunia_mesh::arrangement`]) dá as regiões fechadas —
//! inclusive as formadas por perfis que se cruzam. Arrastar uma região com
//! Push/Pull:
//! - sobre uma face coplanar da malha ativa: grava a região na face (imprint)
//!   e extruda a face nova — para fora soma, para dentro corta;
//! - no vazio: cria uma folha fechada num asset novo e extruda o topo.
//!
//! Em ambos os casos o gesto inteiro é **uma** entrada de Undo e `Esc`
//! restaura exatamente o documento anterior (prelúdio do modal).

use glam::{DVec3, Vec3};
use petunia_mesh::arrangement::{Polyline2, Region2, planar_regions, region_at};
use petunia_mesh::imprint::{ImprintError, imprint_region, region_sheet};

use crate::AppState;
use crate::modal::{ModalError, ModalKind};

/// Tolerância de tessellação dos perfis (unidades de mundo).
const PROFILE_TOLERANCE: f64 = 1.0e-3;

/// Regiões fechadas agrupadas por plano.
pub type RegionPlanes = Vec<(RegionPlane, Vec<Region2>)>;

/// Plano de um grupo de perfis coplanares.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegionPlane {
    pub origin: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub normal: Vec3,
}

impl RegionPlane {
    pub fn to_world(self, p: [f64; 2]) -> Vec3 {
        self.origin + self.right * p[0] as f32 + self.up * p[1] as f32
    }

    pub fn to_plane(self, p: Vec3) -> [f64; 2] {
        let d = p - self.origin;
        [f64::from(d.dot(self.right)), f64::from(d.dot(self.up))]
    }
}

/// Perfis coplanares e suas polilinhas no espaço do plano.
pub(crate) struct ProfileGroup {
    pub plane: RegionPlane,
    pub profiles: Vec<uuid::Uuid>,
    pub lines: Vec<Polyline2>,
    pub shapes: Vec<Option<Region2>>,
}

impl ProfileGroup {
    pub(crate) fn regions(&self) -> Vec<Region2> {
        let mut boundaries = self.lines.clone();
        for shape in self.shapes.iter().flatten() {
            for hole in &shape.holes {
                boundaries.push(Polyline2 {
                    points: hole.clone(),
                    closed: true,
                });
            }
        }
        let cells = planar_regions(&boundaries);
        let shapes: Vec<_> = self.shapes.iter().flatten().cloned().collect();
        petunia_mesh::shape_ops::occupied_regions(&cells, &shapes)
    }
}

/// Região sob o cursor.
#[derive(Debug, Clone, PartialEq)]
pub struct RegionHit {
    pub plane: RegionPlane,
    pub region: Region2,
    /// Ponto do raio no plano (mundo).
    pub point: Vec3,
    /// Distância ao longo do raio.
    pub depth: f32,
}

impl RegionHit {
    /// Contorno externo em mundo.
    pub fn outer_world(&self) -> Vec<Vec3> {
        self.region
            .outer
            .iter()
            .map(|p| self.plane.to_world(*p))
            .collect()
    }
}

/// Por que o Push/Pull de região não começou.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RegionPushError {
    #[error("{0}")]
    Imprint(#[from] ImprintError),
    #[error("{0}")]
    Modal(#[from] ModalError),
    #[error("regions with holes cannot be pushed into a face yet")]
    HoleOnFace,
    #[error("the active object is locked")]
    Locked,
}

/// Região mais próxima atravessada pelo raio, entre planos já calculados
/// (permite ao frontend guardar os planos em cache por revisão).
pub fn region_at_ray(
    planes: &[(RegionPlane, Vec<Region2>)],
    origin: Vec3,
    direction: Vec3,
    perspective: bool,
) -> Option<RegionHit> {
    let mut best: Option<RegionHit> = None;
    for (plane, regions) in planes {
        let denominator = direction.dot(plane.normal);
        if denominator.abs() < 1.0e-6 {
            continue;
        }
        let depth = (plane.origin - origin).dot(plane.normal) / denominator;
        if depth < 0.0 && perspective {
            continue;
        }
        let point = origin + direction * depth;
        let Some(index) = region_at(regions, plane.to_plane(point)) else {
            continue;
        };
        if best.as_ref().is_none_or(|hit| depth < hit.depth) {
            best = Some(RegionHit {
                plane: *plane,
                region: regions[index].clone(),
                point,
                depth,
            });
        }
    }
    best
}

impl AppState {
    /// Planos de perfis com suas regiões fechadas (todas as do documento).
    pub fn profile_region_planes(&self) -> RegionPlanes {
        self.profile_groups(false)
            .into_iter()
            .map(|group| (group.plane, group.regions()))
            .filter(|(_, regions)| !regions.is_empty())
            .collect()
    }

    /// Perfis agrupados por plano. Com `editable_only`, os usados por um Path
    /// Generator (como perfil ou como caminho) ficam de fora: o Shape Builder não
    /// pode substituí-los sem quebrar o gerador.
    pub(crate) fn profile_groups(&self, editable_only: bool) -> Vec<ProfileGroup> {
        let project = &self.project.project;
        let in_use = |profile: &petunia_project::ProfileResource| {
            project.path_generators.iter().any(|generator| {
                generator.profile_id == profile.id || generator.path_id == profile.spline_id
            })
        };
        let mut groups: Vec<ProfileGroup> = Vec::new();
        for profile in &project.profiles {
            if editable_only && in_use(profile) {
                continue;
            }
            let Some(spline) = project.get_spline(profile.spline_id) else {
                continue;
            };
            let Ok(table) = spline.arc_length_table(PROFILE_TOLERANCE) else {
                continue;
            };
            let wp = profile.workplane;
            let [origin, right, up, normal] =
                [wp.origin, wp.right, wp.up, wp.normal].map(|v| DVec3::from_array(v).as_vec3());
            if !(origin.is_finite() && right.is_finite() && up.is_finite() && normal.is_finite())
                || normal.length_squared() < 0.5
            {
                continue;
            }
            let world: Vec<Vec3> = table
                .polyline()
                .into_iter()
                .map(|p| origin + right * p[0] as f32 + up * p[1] as f32)
                .collect();
            let normal = normal.normalize();
            let index = groups.iter().position(|group| {
                group.plane.normal.dot(normal).abs() > 0.9999
                    && (origin - group.plane.origin).dot(group.plane.normal).abs() < 1.0e-4
            });
            let index = index.unwrap_or_else(|| {
                groups.push(ProfileGroup {
                    plane: RegionPlane {
                        origin,
                        right: right.normalize_or_zero(),
                        up: up.normalize_or_zero(),
                        normal,
                    },
                    profiles: Vec::new(),
                    lines: Vec::new(),
                    shapes: Vec::new(),
                });
                groups.len() - 1
            });
            let plane = groups[index].plane;
            let outer: Vec<_> = world.iter().map(|p| plane.to_plane(*p)).collect();
            let mut holes: Vec<Vec<[f64; 2]>> = profile
                .holes
                .iter()
                .map(|hole| {
                    hole.iter()
                        .map(|p| plane.to_plane(origin + right * p[0] as f32 + up * p[1] as f32))
                        .collect()
                })
                .collect();
            for hole in &mut holes {
                if petunia_mesh::arrangement::signed_area(hole) > 0.0 {
                    hole.reverse();
                }
            }
            let mut shape_outer = outer;
            if shape_outer.len() > 1 && shape_outer.first() == shape_outer.last() {
                shape_outer.pop();
            }
            if petunia_mesh::arrangement::signed_area(&shape_outer) < 0.0 {
                shape_outer.reverse();
            }
            groups[index].shapes.push(spline.closed.then_some(Region2 {
                outer: shape_outer,
                holes,
            }));
            groups[index].profiles.push(profile.id);
            groups[index].lines.push(Polyline2 {
                points: world.iter().map(|p| plane.to_plane(*p)).collect(),
                closed: spline.closed,
            });
        }
        groups
    }

    /// Região de perfil mais próxima atravessada pelo raio do cursor.
    pub fn profile_region_at_ray(&self, origin: Vec3, direction: Vec3) -> Option<RegionHit> {
        region_at_ray(
            &self.profile_region_planes(),
            origin,
            direction,
            self.session.camera.proj == crate::Projection::Perspective,
        )
    }

    /// Face da malha ativa coplanar à região e que contém o ponto da região.
    pub fn region_host_face(&self, hit: &RegionHit) -> Option<usize> {
        let mesh = self.project.active_mesh()?;
        let tolerance = 1.0e-4_f32.max(hit.depth.abs() * 1.0e-5);
        (0..mesh.faces.len()).find(|&face| {
            let normal = mesh.face_normal(face).normalize_or_zero();
            let Some(&first) = mesh.faces[face].verts.first() else {
                return false;
            };
            let anchor = mesh.verts[first as usize].vec();
            if normal.dot(hit.plane.normal).abs() < 0.9999
                || (hit.point - anchor).dot(normal).abs() > tolerance
            {
                return false;
            }
            // O ponto da região está dentro da face (no plano da face).
            let u =
                (mesh.verts[mesh.faces[face].verts[1] as usize].vec() - anchor).normalize_or_zero();
            let v = normal.cross(u);
            let polygon: Vec<[f64; 2]> = mesh.faces[face]
                .verts
                .iter()
                .map(|&i| {
                    let d = mesh.verts[i as usize].vec() - anchor;
                    [f64::from(d.dot(u)), f64::from(d.dot(v))]
                })
                .collect();
            let d = hit.point - anchor;
            petunia_mesh::arrangement::point_in_polygon(
                &polygon,
                [f64::from(d.dot(u)), f64::from(d.dot(v))],
            )
        })
    }

    /// Prelúdio + Extrude: grava a região na face hospedeira (ou cria uma
    /// folha num asset novo) fora do histórico e abre a extrusão da face nova.
    /// Confirmar = 1 Undo; cancelar = documento exatamente como antes.
    pub fn begin_region_push_pull(&mut self, hit: &RegionHit) -> Result<(), RegionPushError> {
        if self.modal.is_some() {
            return Err(ModalError::AlreadyActive.into());
        }
        let before = self.project.project.clone();
        let before_selection = self.session.selection.clone();
        let host = self.region_host_face(hit);
        // Antes do prelúdio: trocar o domínio converte a seleção por vértices,
        // e o fundo da folha compartilha os vértices do topo.
        self.set_selection_domain(crate::SelectionDomain::Face);
        if host.is_some() {
            if self.is_active_locked() {
                return Err(RegionPushError::Locked);
            }
            if !hit.region.holes.is_empty() {
                return Err(RegionPushError::HoleOnFace);
            }
            // Congelar antes: a primitiva paramétrica seria regenerada.
            self.freeze_active_primitive_for_command();
        }
        let prelude = match host {
            Some(face) => {
                let mesh = self.project.active_mesh().ok_or(ModalError::NoActiveMesh)?;
                imprint_region(mesh, face, &hit.outer_world()).map(|(mesh, _)| {
                    if let Some(active) = self.project.project.active_mesh_mut() {
                        *active = mesh;
                    }
                })
            }
            None => region_sheet(
                &hit.region.outer,
                &hit.region.holes,
                hit.plane.origin,
                hit.plane.right,
                hit.plane.up,
            )
            .map(|mesh| {
                let name = self.t_id(petunia_config::text_id::DRAW_SHAPE_NAME);
                self.project.project.add(&name, mesh);
            }),
        };
        if let Err(error) = prelude {
            self.project.project = before;
            self.session.selection = before_selection;
            return Err(error.into());
        }
        self.emit_project_changed(petunia_project::ProjectChanges::ALL);
        self.begin_modal_after_prelude(ModalKind::Extrude, before, before_selection)?;
        if host.is_none() {
            self.set_modal_flip_when_negative();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ViewPreset;
    use petunia_project::{ProfileResource, ProfileWorkplane, SplineResource};

    fn state_with_profiles(profiles: &[(&[[f64; 2]], ProfileWorkplane)]) -> AppState {
        let mut state = AppState::default();
        for (points, workplane) in profiles {
            let points: Vec<[f64; 3]> = points.iter().map(|p| [p[0], p[1], 0.0]).collect();
            let spline = SplineResource::from_polyline("curve", &points, true);
            let profile = ProfileResource::new("profile", spline.id, *workplane);
            state.project.project.splines.push(spline);
            state.project.project.profiles.push(profile);
        }
        state.session.camera.set_preset(ViewPreset::Front);
        state
    }

    /// Plano da face frontal do cubo padrão (z = 1, normal +Z).
    fn front_face_plane() -> ProfileWorkplane {
        ProfileWorkplane {
            origin: [0.0, 0.0, 1.0],
            right: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            normal: [0.0, 0.0, 1.0],
        }
    }

    /// Plano vertical afastado do cubo (z = 5).
    fn free_plane() -> ProfileWorkplane {
        ProfileWorkplane {
            origin: [0.0, 0.0, 5.0],
            ..front_face_plane()
        }
    }

    const SQUARE: [[f64; 2]; 4] = [[-0.5, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]];
    const SHIFTED: [[f64; 2]; 4] = [[0.0, 0.0], [0.8, 0.0], [0.8, 0.8], [0.0, 0.8]];

    fn ray_at(state: &AppState, x: f32, y: f32) -> (Vec3, Vec3) {
        let ndc = state.session.camera.project_ndc(Vec3::new(x, y, 0.0));
        state.session.camera.ray(ndc.x, ndc.y)
    }

    #[test]
    fn crossing_profiles_expose_their_overlap_as_a_region() {
        let state = state_with_profiles(&[(&SQUARE, free_plane()), (&SHIFTED, free_plane())]);
        let planes = state.profile_region_planes();
        assert_eq!(planes.len(), 1, "perfis coplanares formam um grupo");
        assert_eq!(planes[0].1.len(), 3);
        let (origin, direction) = ray_at(&state, 0.25, 0.25);
        let hit = state.profile_region_at_ray(origin, direction).unwrap();
        assert!(
            (hit.region.area() - 0.25).abs() < 1e-6,
            "{}",
            hit.region.area()
        );
        assert!((hit.point.z - 5.0).abs() < 1e-4);
    }

    #[test]
    fn region_on_a_face_is_imprinted_and_extruded_as_one_undo() {
        let mut state = state_with_profiles(&[(&SQUARE, front_face_plane())]);
        let faces_before = state.project.active_mesh().unwrap().faces.len();
        let depth_before = state.project.undo.depth();
        let (origin, direction) = ray_at(&state, 0.0, 0.0);
        let hit = state.profile_region_at_ray(origin, direction).unwrap();
        assert!(state.region_host_face(&hit).is_some());

        state.begin_region_push_pull(&hit).unwrap();
        state.update_modal(Vec3::ZERO, -0.3).unwrap();
        assert!(state.commit_modal());
        let mesh = state.project.active_mesh().unwrap();
        // Face frontal virou anel (2) + face interna; mais 4 paredes do rebaixo.
        assert_eq!(mesh.faces.len(), faces_before + 2 + 4);
        assert!(mesh.verts.iter().any(|v| (v.pos[2] - 0.7).abs() < 1e-4));
        assert_eq!(
            state.project.undo.depth().0,
            depth_before.0 + 1,
            "um só Undo"
        );

        assert!(state.undo());
        assert_eq!(
            state.project.active_mesh().unwrap().faces.len(),
            faces_before
        );
    }

    #[test]
    fn cancel_restores_the_document_exactly() {
        let mut state = state_with_profiles(&[(&SQUARE, front_face_plane())]);
        let before = state.project.active_mesh().unwrap().clone();
        let assets = state.project.project.assets.len();
        let (origin, direction) = ray_at(&state, 0.0, 0.0);
        let hit = state.profile_region_at_ray(origin, direction).unwrap();
        state.begin_region_push_pull(&hit).unwrap();
        state.update_modal(Vec3::ZERO, 0.4).unwrap();
        assert!(state.cancel_modal());
        let after = state.project.active_mesh().unwrap();
        assert_eq!(after.faces.len(), before.faces.len());
        assert_eq!(after.verts.len(), before.verts.len());
        assert_eq!(state.project.project.assets.len(), assets);
    }

    #[test]
    fn free_region_becomes_a_closed_solid_in_a_new_asset() {
        let mut state = state_with_profiles(&[(&SQUARE, free_plane())]);
        let assets = state.project.project.assets.len();
        let (origin, direction) = ray_at(&state, 0.0, 0.0);
        let hit = state.profile_region_at_ray(origin, direction).unwrap();
        assert!(state.region_host_face(&hit).is_none());
        state.begin_region_push_pull(&hit).unwrap();
        state.update_modal(Vec3::ZERO, 0.5).unwrap();
        assert!(state.commit_modal());
        assert_eq!(state.project.project.assets.len(), assets + 1);
        let mesh = state.project.active_mesh().unwrap();
        assert_eq!(mesh.faces.len(), 6, "caixa fechada");

        // Para o lado negativo o sólido continua fechado e voltado para fora.
        let mut state = state_with_profiles(&[(&SQUARE, free_plane())]);
        let hit = state.profile_region_at_ray(origin, direction).unwrap();
        state.begin_region_push_pull(&hit).unwrap();
        state.update_modal(Vec3::ZERO, -0.5).unwrap();
        assert!(state.commit_modal());
        let mesh = state.project.active_mesh().unwrap();
        assert_eq!(mesh.faces.len(), 6);
        let center = mesh.verts.iter().map(|v| v.vec()).sum::<Vec3>() / mesh.verts.len() as f32;
        for face in 0..mesh.faces.len() {
            let first = mesh.verts[mesh.faces[face].verts[0] as usize].vec();
            assert!(
                mesh.face_normal(face).dot(first - center) > 0.0,
                "face {face} voltada para dentro"
            );
        }

        // Confirmar sem mover não deixa asset órfão.
        let mut state = state_with_profiles(&[(&SQUARE, free_plane())]);
        let hit = state.profile_region_at_ray(origin, direction).unwrap();
        state.begin_region_push_pull(&hit).unwrap();
        assert!(state.commit_modal());
        assert_eq!(state.project.project.assets.len(), assets);
    }

    #[test]
    fn last_operation_reapplies_the_region_push_in_the_same_undo() {
        let mut state = state_with_profiles(&[(&SQUARE, front_face_plane())]);
        let (origin, direction) = ray_at(&state, 0.0, 0.0);
        let hit = state.profile_region_at_ray(origin, direction).unwrap();
        state.begin_region_push_pull(&hit).unwrap();
        state.update_modal(Vec3::ZERO, 0.2).unwrap();
        let last = state.commit_modal_gesture().unwrap();
        let depth = state.project.undo.depth();
        let faces = state.project.active_mesh().unwrap().faces.len();

        let adjusted = state.adjust_last_operation(&last, 0.6).unwrap().unwrap();
        assert_eq!(state.project.undo.depth(), depth);
        let mesh = state.project.active_mesh().unwrap();
        assert_eq!(mesh.faces.len(), faces, "sem imprint duplicado");
        assert!(mesh.verts.iter().any(|v| (v.pos[2] - 1.6).abs() < 1e-4));
        assert!((adjusted.value - 0.6).abs() < 1e-6);
    }
}
