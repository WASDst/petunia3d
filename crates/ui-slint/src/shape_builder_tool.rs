//! Shape Builder no bridge: arrastar sobre faces do DRAW para fundi-las, Ctrl +
//! arrastar para apagá-las, clique para extrair uma face (ver `petunia_core::shape_builder`).
//!
//! O gesto segue a gramática única (ADR 007): o `ToolSession` decide clique ×
//! arrasto, e `Esc` cancela sem tocar no documento. Durante o arrasto as faces
//! tocadas ficam destacadas no mesmo overlay da região em hover; o documento só
//! muda ao soltar, numa única entrada de Undo.

use glam::{Vec2, Vec3};
use petunia_core::{PathfinderOp, RegionHit, RegionPlane, ShapeEdit, ShapeEditCmd, ShapePlane};
use petunia_mesh::arrangement::{Region2, region_at};

use crate::{ModelingMode, PetuniaViewport, SlintUiBridge, ToolGesture};
use petunia_core::Workspace;

/// Passo (px) entre amostras ao longo do caminho do ponteiro: pequeno o bastante
/// para não pular faces estreitas quando o mouse anda depressa.
const SAMPLE_STEP_PX: f32 = 3.0;
/// Teto de amostras por gesto.
const MAX_SAMPLES: usize = 4_000;

/// Gesto em andamento.
#[derive(Debug, Clone)]
pub(crate) struct ShapeBuilderGesture {
    pub plane: Option<RegionPlane>,
    pub samples: Vec<Vec3>,
    /// Faces tocadas (para o destaque), no plano do gesto.
    pub touched: Vec<Region2>,
    pub last_pixel: [f32; 2],
    pub delete: bool,
}

/// Face sob o cursor no plano editável.
pub(crate) struct ShapeHit {
    pub(crate) plane: RegionPlane,
    pub(crate) region: Region2,
    pub(crate) point: Vec3,
    pub(crate) depth: f32,
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Só o DRAW oferece o Shape Builder.
    fn shape_builder_available(&self) -> bool {
        self.state.workspace == Workspace::Model
            && self.modeling_mode == ModelingMode::Draw
            && self.profile_volume_mode.is_none()
            && self.state.modal.is_none()
            && self.state.mesh_preview.is_none()
    }

    /// Planos editáveis em cache; recalcula quando perfis, splines ou geradores mudam.
    pub(crate) fn shape_planes_cached(&self) -> std::cell::Ref<'_, Vec<ShapePlane>> {
        let project = &self.state.project.project;
        let clock = project.revision_clock();
        let key = [
            project.profiles.len() as u64,
            project.path_generators.len() as u64,
            clock[9],
            clock[10],
        ];
        {
            let mut slot = self.shape_planes_cache.borrow_mut();
            if slot.as_ref().is_none_or(|(cached, _)| *cached != key) {
                *slot = Some((key, self.state.shape_planes()));
            }
        }
        std::cell::Ref::map(self.shape_planes_cache.borrow(), |slot| {
            &slot.as_ref().expect("cache preenchido acima").1
        })
    }

    /// Face editável sob o ponto (px lógicos da viewport), se não estiver
    /// escondida atrás da malha ativa.
    pub(crate) fn shape_hit_at(&self, pixel: [f32; 2]) -> Option<ShapeHit> {
        let [width, height] = self.viewport_size;
        if width <= 1.0 || height <= 1.0 || !pixel[0].is_finite() || !pixel[1].is_finite() {
            return None;
        }
        let ndc = Vec2::new(pixel[0] / width * 2.0 - 1.0, 1.0 - pixel[1] / height * 2.0);
        let camera = &self.state.session.camera;
        let (origin, direction) = camera.ray(ndc.x, ndc.y);
        let perspective = camera.proj == petunia_core::Projection::Perspective;
        let mut best: Option<ShapeHit> = None;
        for shape in self.shape_planes_cached().iter() {
            let plane = shape.plane;
            let denominator = direction.dot(plane.normal);
            if denominator.abs() < 1.0e-6 {
                continue;
            }
            let depth = (plane.origin - origin).dot(plane.normal) / denominator;
            if depth < 0.0 && perspective {
                continue;
            }
            let point = origin + direction * depth;
            let Some(index) = region_at(&shape.regions, plane.to_plane(point)) else {
                continue;
            };
            if best.as_ref().is_none_or(|hit| depth < hit.depth) {
                best = Some(ShapeHit {
                    plane,
                    region: shape.regions[index].clone(),
                    point,
                    depth,
                });
            }
        }
        let hit = best?;
        let occluder = self.state.project.active_mesh().and_then(|mesh| {
            petunia_core::picking::pick_mesh(
                mesh,
                camera,
                Vec2::new(width, height),
                ndc,
                petunia_core::SelectMode::Face,
                false,
            )
        });
        let hidden = occluder.is_some_and(|occluder| {
            let depth = (occluder.position - origin).dot(direction);
            depth < hit.depth - 1.0e-3 * hit.depth.abs().max(1.0)
        });
        (!hidden).then_some(hit)
    }

    /// Hover do Shape Builder: destaca a face editável sob o cursor.
    pub(crate) fn hover_shape_builder(&mut self, pixel: [f32; 2]) -> bool {
        let previous = self.region_hover.take();
        self.region_hover = self.shape_hit_at(pixel).map(|hit| RegionHit {
            plane: hit.plane,
            region: hit.region,
            point: hit.point,
            depth: hit.depth,
        });
        previous != self.region_hover
    }

    /// Começa o arrasto: a primeira amostra é a face sob o ponto de partida.
    pub(crate) fn begin_shape_builder(&mut self, anchor: [f32; 2]) -> bool {
        if !self.shape_builder_available() {
            return false;
        }
        let mut gesture = ShapeBuilderGesture {
            plane: None,
            samples: Vec::new(),
            touched: Vec::new(),
            last_pixel: anchor,
            delete: self.tool_press_alternate,
        };
        self.shape_builder_sample(&mut gesture, anchor);
        self.shape_builder = Some(gesture);
        self.tool_gesture = Some(ToolGesture::ShapeBuilder);
        self.region_hover = None;
        self.state.set_status(if self.tool_press_alternate {
            "Shape Builder: release to delete the regions you passed over"
        } else {
            "Shape Builder: release to merge the regions you passed over (Ctrl deletes, Esc cancels)"
        });
        true
    }

    fn shape_builder_sample(&self, gesture: &mut ShapeBuilderGesture, pixel: [f32; 2]) {
        if gesture.samples.len() >= MAX_SAMPLES {
            return;
        }
        let Some(hit) = self.shape_hit_at(pixel) else {
            return;
        };
        match gesture.plane {
            None => gesture.plane = Some(hit.plane),
            Some(plane) if !same_plane(&plane, &hit.plane) => return,
            Some(_) => {}
        }
        gesture.samples.push(hit.point);
        if !gesture.touched.contains(&hit.region) {
            gesture.touched.push(hit.region);
        }
    }

    /// Estende o caminho até `current` amostrando a cada poucos pixels.
    pub(crate) fn update_shape_builder(&mut self, current: [f32; 2]) -> bool {
        let Some(mut gesture) = self.shape_builder.take() else {
            return false;
        };
        let (from, to) = (gesture.last_pixel, current);
        let distance = (to[0] - from[0]).hypot(to[1] - from[1]);
        let steps = (distance / SAMPLE_STEP_PX).ceil().max(1.0) as usize;
        let before = gesture.touched.len();
        for step in 1..=steps {
            let t = step as f32 / steps as f32;
            let pixel = [
                from[0] + (to[0] - from[0]) * t,
                from[1] + (to[1] - from[1]) * t,
            ];
            self.shape_builder_sample(&mut gesture, pixel);
        }
        gesture.last_pixel = current;
        let changed = gesture.touched.len() != before;
        self.shape_builder = Some(gesture);
        changed
    }

    fn finish_shape_builder(
        &mut self,
        samples: Vec<Vec3>,
        plane: Option<RegionPlane>,
        delete: bool,
    ) -> bool {
        if samples.is_empty() {
            self.state
                .set_status("Shape Builder: pass over the regions you want to change");
            return true;
        }
        let edit = if delete {
            ShapeEdit::Delete { samples }
        } else {
            ShapeEdit::Merge { samples }
        };
        let command = ShapeEditCmd { plane, edit };
        match self.state.dispatch(&command) {
            Ok(()) => {
                self.after_shape_edit();
                true
            }
            Err(error) => {
                self.state.set_status(error.to_string());
                true
            }
        }
    }

    /// Solta o gesto: aplica a fusão (ou a exclusão) numa única entrada de Undo.
    pub(crate) fn commit_shape_builder(&mut self) -> bool {
        let Some(gesture) = self.shape_builder.take() else {
            return false;
        };
        self.finish_shape_builder(gesture.samples, gesture.plane, gesture.delete)
    }

    /// Clique sem arrasto: extrai (ou, com Ctrl, apaga) a face sob o cursor.
    pub(crate) fn shape_builder_click(&mut self, at: [f32; 2]) -> bool {
        let Some(hit) = self.shape_hit_at(at) else {
            self.state
                .set_status("Shape Builder: click inside a closed shape");
            return true;
        };
        self.finish_shape_builder(vec![hit.point], Some(hit.plane), self.tool_press_alternate)
    }

    /// `Esc` no meio do arrasto: nada muda no documento.
    pub(crate) fn cancel_shape_builder(&mut self) -> bool {
        let had = self.shape_builder.take().is_some();
        if had {
            self.state.set_status("Shape Builder cancelled");
        }
        had
    }

    /// Pathfinder (Unite/Subtract/Intersect/Exclude) sobre os perfis fechados do plano.
    pub fn run_pathfinder(&mut self, op: PathfinderOp) -> bool {
        if !self.shape_builder_available() {
            return false;
        }
        let plane = self.active_profile_id.and_then(|id| {
            self.shape_planes_cached()
                .iter()
                .find(|shape| shape.profiles.contains(&id))
                .map(|shape| shape.plane)
        });
        let command = ShapeEditCmd {
            plane,
            edit: ShapeEdit::Pathfinder(op),
        };
        match self.state.dispatch(&command) {
            Ok(()) => {
                self.after_shape_edit();
                true
            }
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    /// Depois de editar formas: caches e perfil em edição precisam acompanhar o documento.
    fn after_shape_edit(&mut self) {
        *self.region_planes_cache.borrow_mut() = None;
        *self.shape_planes_cache.borrow_mut() = None;
        self.region_hover = None;
        if let Some(id) = self.active_profile_id
            && self.state.project.project.get_profile(id).is_none()
        {
            self.active_profile_id = None;
            self.profile_selected_point = None;
            self.profile_drag_target = None;
            self.profile_edit_gesture = None;
        }
        self.state.mark_dirty();
    }

    /// Faces tocadas no gesto (para o overlay), em mundo.
    pub(crate) fn shape_builder_touched_rings(&self) -> Vec<Vec<Vec3>> {
        let Some(gesture) = &self.shape_builder else {
            return Vec::new();
        };
        let Some(plane) = gesture.plane else {
            return Vec::new();
        };
        gesture
            .touched
            .iter()
            .flat_map(|region| std::iter::once(&region.outer).chain(region.holes.iter()))
            .map(|ring| ring.iter().map(|p| plane.to_world(*p)).collect())
            .collect()
    }
}

fn same_plane(a: &RegionPlane, b: &RegionPlane) -> bool {
    a.normal.dot(b.normal).abs() > 0.9999 && (a.origin - b.origin).dot(a.normal).abs() < 1.0e-4
}
