//! Transactional viewport previews. Each update starts from the original mesh;
//! only commit records history, and cancel restores the exact project snapshot.

use glam::{EulerRot, Quat, Vec3};
use petunia_mesh::Mesh;
use petunia_project::{Project, ProjectChanges};

use crate::{AppState, EditMode, Selection, SnapKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalKind {
    Move,
    Rotate,
    Scale,
    Extrude,
    ExtrudeIndividual,
    Inset,
    Bevel,
    PushPull,
}

impl ModalKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Move => "Move",
            Self::Rotate => "Rotate",
            Self::Scale => "Scale",
            Self::Extrude => "Extrude",
            Self::ExtrudeIndividual => "Extrude Individual",
            Self::Inset => "Inset (factor)",
            Self::Bevel => "Bevel",
            Self::PushPull => "Push/Pull",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModalConstraint {
    #[default]
    Free,
    Axis(usize),
    /// Plane perpendicular to the given axis (0 = YZ, 1 = XZ, 2 = XY).
    Plane(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalError {
    AlreadyActive,
    NoActiveMesh,
    NoSelection,
    FacesRequired,
    EdgesRequired,
    InvalidInput,
    InvalidMesh,
    NoActiveOperation,
    UnsupportedTopology,
    ActiveLocked,
}

impl std::fmt::Display for ModalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::AlreadyActive => "Confirm or cancel the active operation first",
            Self::NoActiveMesh => "No active mesh",
            Self::NoSelection => "Select geometry first",
            Self::FacesRequired => "Select a face first",
            Self::EdgesRequired => "Select an edge first",
            Self::InvalidInput => "Invalid or out-of-range transform value",
            Self::InvalidMesh => "Operation rejected: invalid mesh",
            Self::NoActiveOperation => "No active operation",
            Self::UnsupportedTopology => {
                "Bevel supports one manifold edge with simple corners; select a supported edge"
            }
            Self::ActiveLocked => "Active object is locked",
        })
    }
}

impl std::error::Error for ModalError {}

pub struct ModalOp {
    pub kind: ModalKind,
    pub constraint: ModalConstraint,
    pub pivot: Vec3,
    pub normal: Vec3,
    /// Distance, degrees, scale factor, or inset fraction (not meters).
    pub value: f32,
    /// Absolute XYZ delta, Euler XYZ degrees, or per-axis scale factors.
    pub components: Vec3,
    /// Frozen per-point pivots for the Individual Origins policy.
    pivots: Option<Vec<Vec3>>,
    individual_origins: bool,
    original: Project,
    selection: Selection,
    source: Mesh,
    changed: bool,
    /// Alvo em que o último update encaixou de fato (não apenas "snap ligado").
    snap: Option<SnapKind>,
    /// Estado de antes de um prelúdio (imprint de região, forma nova) feito
    /// fora do histórico: o gesto inteiro vira uma entrada de Undo e Esc
    /// restaura exatamente este estado (constituição 11).
    before_prelude: Option<Box<(Project, Selection)>>,
    /// A malha ativa é uma folha de região solta: extrudar para o lado
    /// negativo inverte todas as faces para o sólido continuar voltado para fora.
    flip_when_negative: bool,
}

impl ModalOp {
    /// A prévia difere do estado original (confirmar criará uma entrada de Undo).
    pub(crate) fn changed(&self) -> bool {
        self.changed
    }

    /// Verdadeiro somente quando o último update foi atraído por um alvo de snap.
    pub fn snapped(&self) -> bool {
        self.snap.is_some()
    }

    /// Tipo do alvo que encaixou no último update, para o rótulo do HUD.
    pub fn snap_kind(&self) -> Option<SnapKind> {
        self.snap
    }

    /// Vértices da malha de origem que se movem com a operação: não servem de
    /// alvo de snap (senão o ponto encaixaria em si mesmo).
    /// Malha de origem congelada no início da operação (sem a prévia).
    pub fn source_mesh(&self) -> &Mesh {
        &self.source
    }

    pub fn moving_vertices(&self) -> Vec<bool> {
        self.source.verts.iter().map(|v| v.selected).collect()
    }

    /// Ponto de mundo que representa o valor atual, quando ele é uma posição.
    ///
    /// Move usa o deslocamento absoluto; Extrude/Push-Pull, a distância ao longo
    /// da normal. Rotate, Scale, Inset e Bevel não têm um ponto de mundo
    /// correspondente (graus, fator ou fração), então retornam `None`.
    pub fn current_point(&self) -> Option<Vec3> {
        match self.kind {
            ModalKind::Move => Some(self.pivot + self.components),
            ModalKind::Extrude | ModalKind::ExtrudeIndividual | ModalKind::PushPull => {
                Some(self.pivot + self.normal * self.value)
            }
            ModalKind::Rotate | ModalKind::Scale | ModalKind::Inset | ModalKind::Bevel => None,
        }
    }
}

fn valid_mesh(mesh: &Mesh) -> bool {
    mesh.verts.iter().all(|v| v.vec().is_finite())
        && mesh.faces.iter().all(|f| {
            f.verts.len() >= 3
                && f.uv.len() == f.verts.len()
                && f.verts.iter().all(|&v| (v as usize) < mesh.verts.len())
                && f.uv.iter().flatten().all(|v| v.is_finite())
        })
        && mesh.selected_edges.iter().all(|&(a, b)| {
            a != b && (a as usize) < mesh.verts.len() && (b as usize) < mesh.verts.len()
        })
}

fn same_geometry(a: &Mesh, b: &Mesh) -> bool {
    a.verts.len() == b.verts.len()
        && a.faces.len() == b.faces.len()
        && a.verts
            .iter()
            .zip(&b.verts)
            .all(|(a, b)| a.pos == b.pos && a.color == b.color)
        && a.faces
            .iter()
            .zip(&b.faces)
            .all(|(a, b)| a.verts == b.verts && a.uv == b.uv)
}

fn axis(index: usize) -> Vec3 {
    match index {
        0 => Vec3::X,
        1 => Vec3::Y,
        _ => Vec3::Z,
    }
}

/// Connected selected elements share an individual pivot. Loose selected points
/// retain their own position; an object uses its complete geometric center.
fn individual_pivots(mesh: &Mesh, object: bool) -> Vec<Vec3> {
    if object {
        let center =
            mesh.verts.iter().map(|v| v.vec()).sum::<Vec3>() / mesh.verts.len().max(1) as f32;
        return vec![center; mesh.verts.len()];
    }
    let mut neighbors = vec![Vec::new(); mesh.verts.len()];
    for (a, b) in mesh.edges_unique() {
        let (a, b) = (a as usize, b as usize);
        if mesh.verts[a].selected && mesh.verts[b].selected {
            neighbors[a].push(b);
            neighbors[b].push(a);
        }
    }
    let mut pivots: Vec<_> = mesh.verts.iter().map(|v| v.vec()).collect();
    let mut visited = vec![false; mesh.verts.len()];
    for seed in 0..mesh.verts.len() {
        if visited[seed] || !mesh.verts[seed].selected {
            continue;
        }
        let mut stack = vec![seed];
        let mut group = Vec::new();
        while let Some(index) = stack.pop() {
            if visited[index] {
                continue;
            }
            visited[index] = true;
            group.push(index);
            stack.extend(neighbors[index].iter().copied().filter(|&i| !visited[i]));
        }
        let center = group.iter().map(|&i| mesh.verts[i].vec()).sum::<Vec3>() / group.len() as f32;
        for index in group {
            pivots[index] = center;
        }
    }
    pivots
}

impl AppState {
    pub fn begin_modal(&mut self, kind: ModalKind) -> Result<(), ModalError> {
        if self.modal.is_some() || self.mesh_preview.is_some() || self.paint_stroke.is_some() {
            return Err(ModalError::AlreadyActive);
        }
        if self.is_active_locked() {
            return Err(ModalError::ActiveLocked);
        }
        self.sync_selection();
        let mesh = self.project.active_mesh().ok_or(ModalError::NoActiveMesh)?;
        if !valid_mesh(mesh) {
            return Err(ModalError::InvalidMesh);
        }
        let mut source = mesh.clone();
        let transform = matches!(kind, ModalKind::Move | ModalKind::Rotate | ModalKind::Scale);
        if transform {
            self.gizmo_mode = kind;
        }
        if transform && self.edit_mode() == EditMode::Object {
            source.select_all();
        } else {
            // Edições topológicas ou de sub-elementos congelam a primitiva paramétrica em malha B-Rep estática.
            self.freeze_active_primitive();

            // Face and edge selection must transform their vertices too.
            for face in &source.faces {
                if face.selected {
                    for &vi in &face.verts {
                        source.verts[vi as usize].selected = true;
                    }
                }
            }
            for &(a, b) in &source.selected_edges {
                source.verts[a as usize].selected = true;
                source.verts[b as usize].selected = true;
            }
        }
        if !source.has_selection() || source.verts.is_empty() {
            return Err(ModalError::NoSelection);
        }
        if matches!(
            kind,
            ModalKind::Extrude
                | ModalKind::ExtrudeIndividual
                | ModalKind::Inset
                | ModalKind::PushPull
        ) {
            if source.selected_face_count() == 0 {
                source.sync_face_selection_from_verts();
            }
            if source.selected_face_count() == 0 {
                return Err(ModalError::FacesRequired);
            }
        }
        if kind == ModalKind::Bevel && source.selected_edges.is_empty() {
            let sel_verts = source.verts.iter().filter(|v| v.selected).count();
            if sel_verts == 0 {
                source.sync_edge_selection_from_verts();
                if source.selected_edges.is_empty() {
                    return Err(ModalError::EdgesRequired);
                }
            }
        }
        let mut normal = Vec3::ZERO;
        for (fi, face) in source.faces.iter().enumerate() {
            if face.selected {
                normal += source.face_normal(fi);
            }
        }
        normal = normal.normalize_or_zero();
        if normal == Vec3::ZERO || transform {
            normal = self.camera.forward().normalize_or_zero();
        }
        let initial_constraint = match self.locked_axes {
            [true, false, false] => ModalConstraint::Axis(0),
            [false, true, false] => ModalConstraint::Axis(1),
            [false, false, true] => ModalConstraint::Axis(2),
            [false, true, true] => ModalConstraint::Plane(0),
            [true, false, true] => ModalConstraint::Plane(1),
            [true, true, false] => ModalConstraint::Plane(2),
            _ => ModalConstraint::Free,
        };
        let pivot = self.calculate_pivot(self.session.pivot_point);
        let individual_origins =
            transform && self.session.pivot_point == crate::PivotPoint::IndividualOrigins;
        let pivots = individual_origins
            .then(|| individual_pivots(&source, self.edit_mode() == EditMode::Object));
        self.modal = Some(ModalOp {
            kind,
            pivots,
            individual_origins,
            constraint: initial_constraint,
            pivot,
            normal,
            value: if kind == ModalKind::Scale { 1.0 } else { 0.0 },
            components: if kind == ModalKind::Scale {
                Vec3::ONE
            } else {
                Vec3::ZERO
            },
            original: self.project.project.clone(),
            selection: self.session.selection.clone(),
            source,
            changed: false,
            snap: None,
            before_prelude: None,
            flip_when_negative: false,
        });
        self.pending_modal = None;
        self.mark_dirty();
        Ok(())
    }

    /// Inicia `kind` depois de um prelúdio já aplicado ao documento fora do
    /// histórico (ex.: imprint de uma região numa face). `before` é o estado
    /// anterior ao prelúdio: confirmar grava uma única entrada de Undo a partir
    /// dele; cancelar (ou confirmar sem mudança) volta exatamente a ele. Em
    /// erro, o documento também volta a `before`.
    pub fn begin_modal_after_prelude(
        &mut self,
        kind: ModalKind,
        before: Project,
        before_selection: Selection,
    ) -> Result<(), ModalError> {
        match self.begin_modal(kind) {
            Ok(()) => {
                if let Some(modal) = self.modal.as_mut() {
                    modal.before_prelude = Some(Box::new((before, before_selection)));
                }
                Ok(())
            }
            Err(error) => {
                if !matches!(error, ModalError::AlreadyActive) {
                    self.restore_before_prelude(before, before_selection);
                }
                Err(error)
            }
        }
    }

    fn restore_before_prelude(&mut self, before: Project, selection: Selection) {
        let revision_clock = self.project.project.revision_clock();
        self.project.project = before;
        self.project
            .project
            .rebase_revisions_after_restore(revision_clock);
        self.session.selection = selection;
        self.events.emit(crate::AppEvent::SelectionChanged(
            self.session.selection.clone(),
        ));
        self.emit_project_changed(ProjectChanges::ALL);
    }

    pub(crate) fn modal_flips_when_negative(&self) -> bool {
        self.modal
            .as_ref()
            .is_some_and(|modal| modal.flip_when_negative)
    }

    /// Marca a operação ativa como extrusão de uma folha de região solta.
    pub(crate) fn set_modal_flip_when_negative(&mut self) {
        if let Some(modal) = self.modal.as_mut() {
            modal.flip_when_negative = true;
        }
    }

    /// Estado posterior ao prelúdio (para reaplicar a "Última operação").
    pub(crate) fn modal_prelude_state(&self) -> Option<(Project, Selection)> {
        let modal = self.modal.as_ref()?;
        modal
            .before_prelude
            .as_ref()
            .map(|_| (modal.original.clone(), modal.selection.clone()))
    }

    pub fn set_modal_constraint(&mut self, constraint: ModalConstraint) -> Result<(), ModalError> {
        if matches!(constraint, ModalConstraint::Axis(i) | ModalConstraint::Plane(i) if i > 2) {
            return Err(ModalError::InvalidInput);
        }
        let modal = self.modal.as_mut().ok_or(ModalError::NoActiveOperation)?;
        modal.constraint = constraint;
        self.locked_axes = match constraint {
            ModalConstraint::Axis(0) => [true, false, false],
            ModalConstraint::Axis(1) => [false, true, false],
            ModalConstraint::Axis(2) => [false, false, true],
            ModalConstraint::Plane(0) => [false, true, true],
            ModalConstraint::Plane(1) => [true, false, true],
            ModalConstraint::Plane(2) => [true, true, false],
            _ => [false, false, false],
        };
        self.mark_dirty();
        Ok(())
    }

    /// `translation` is an absolute world-space delta from the start, `value`
    /// is the absolute scalar (including exact numeric input), never a frame delta.
    /// The core never snaps: snapping is one screen-space pass by the caller
    /// ([`crate::inference`]), reported through [`Self::update_modal_snapped`].
    pub fn update_modal(&mut self, translation: Vec3, value: f32) -> Result<(), ModalError> {
        self.update_modal_snapped(translation, value, None)
    }

    /// Same as [`Self::update_modal`], recording which snap target the caller
    /// already applied to `translation` (P3D-040: one snap pass per gesture).
    pub fn update_modal_snapped(
        &mut self,
        translation: Vec3,
        value: f32,
        snap: Option<SnapKind>,
    ) -> Result<(), ModalError> {
        if !translation.is_finite() || !value.is_finite() || value.abs() > 1.0e6 {
            return Err(ModalError::InvalidInput);
        }
        let modal = self.modal.as_ref().ok_or(ModalError::NoActiveOperation)?;
        if modal.kind == ModalKind::Scale && value.abs() < 1.0e-6 {
            return Err(ModalError::InvalidInput);
        }
        if modal.kind == ModalKind::Inset && !(0.0..=0.95).contains(&value) {
            return Err(ModalError::InvalidInput);
        }
        if modal.kind == ModalKind::Bevel && value < 0.0 {
            return Err(ModalError::InvalidInput);
        }
        let mut mesh = modal.source.clone();
        let mut components = Vec3::ZERO;
        let direction = match modal.constraint {
            ModalConstraint::Axis(i) => axis(i),
            ModalConstraint::Plane(i) => {
                (modal.normal - axis(i) * modal.normal[i]).normalize_or_zero()
            }
            ModalConstraint::Free => modal.normal,
        };
        let use_proportional = self.proportional_editing
            && matches!(
                modal.kind,
                ModalKind::Move | ModalKind::Rotate | ModalKind::Scale
            );
        match modal.kind {
            ModalKind::Move => {
                let delta = match modal.constraint {
                    ModalConstraint::Free => translation,
                    ModalConstraint::Axis(i) => {
                        if self.snap_enabled {
                            axis(i) * translation[i]
                        } else {
                            axis(i) * value
                        }
                    }
                    ModalConstraint::Plane(i) => translation - axis(i) * translation[i],
                };
                components = delta;
                if self.session.edit_pivot {
                    // In Edit Pivot mode, geometry stays stationary; only origin moves.
                } else if use_proportional {
                    for vertex in &mut mesh.verts {
                        if vertex.selected {
                            vertex.pos = (vertex.vec() + delta).to_array();
                        } else {
                            let dist = (vertex.vec() - modal.pivot).length();
                            let weight = crate::proportional::calculate_falloff_weight(
                                dist,
                                self.session.proportional_settings.radius,
                                self.session.proportional_settings.falloff,
                            );
                            if weight > 0.0 {
                                vertex.pos = (vertex.vec() + delta * weight).to_array();
                            }
                        }
                    }
                } else {
                    mesh.translate_selected(delta.to_array());
                }
            }
            ModalKind::Rotate => {
                let normal = match modal.constraint {
                    ModalConstraint::Axis(i) | ModalConstraint::Plane(i) => axis(i),
                    ModalConstraint::Free => modal.normal,
                };
                let rotation = Quat::from_axis_angle(normal, value.to_radians());
                let (x, y, z) = rotation.to_euler(EulerRot::XYZ);
                components = Vec3::new(x.to_degrees(), y.to_degrees(), z.to_degrees());
                for (index, vertex) in mesh.verts.iter_mut().enumerate() {
                    let pivot = modal
                        .pivots
                        .as_ref()
                        .map_or(modal.pivot, |pivots| pivots[index]);
                    if vertex.selected {
                        vertex.pos = (pivot + rotation * (vertex.vec() - pivot)).to_array();
                    } else if use_proportional {
                        let dist = (vertex.vec() - modal.pivot).length();
                        let weight = crate::proportional::calculate_falloff_weight(
                            dist,
                            self.session.proportional_settings.radius,
                            self.session.proportional_settings.falloff,
                        );
                        if weight > 0.0 {
                            let rotated = pivot + rotation * (vertex.vec() - pivot);
                            vertex.pos = vertex.vec().lerp(rotated, weight).to_array();
                        }
                    }
                }
            }
            ModalKind::Scale => {
                let factors = match modal.constraint {
                    ModalConstraint::Free => Vec3::splat(value),
                    ModalConstraint::Axis(i) => Vec3::ONE + axis(i) * (value - 1.0),
                    ModalConstraint::Plane(i) => Vec3::splat(value) + axis(i) * (1.0 - value),
                };
                components = factors;
                for (index, vertex) in mesh.verts.iter_mut().enumerate() {
                    let pivot = modal
                        .pivots
                        .as_ref()
                        .map_or(modal.pivot, |pivots| pivots[index]);
                    if vertex.selected {
                        vertex.pos = (pivot + (vertex.vec() - pivot) * factors).to_array();
                    } else if use_proportional {
                        let dist = (vertex.vec() - modal.pivot).length();
                        let weight = crate::proportional::calculate_falloff_weight(
                            dist,
                            self.session.proportional_settings.radius,
                            self.session.proportional_settings.falloff,
                        );
                        if weight > 0.0 {
                            let scaled = pivot + (vertex.vec() - pivot) * factors;
                            vertex.pos = vertex.vec().lerp(scaled, weight).to_array();
                        }
                    }
                }
            }
            ModalKind::Extrude if value != 0.0 => {
                // Build topology once per absolute preview, then constrain the new cap.
                mesh.extrude_selected(0.0);
                for vertex in mesh.verts.iter_mut().filter(|v| v.selected) {
                    vertex.pos = (vertex.vec() + direction * value).to_array();
                }
                if modal.flip_when_negative && value < 0.0 {
                    for face in &mut mesh.faces {
                        face.verts.reverse();
                        face.uv.reverse();
                    }
                }
            }
            ModalKind::ExtrudeIndividual if value != 0.0 => mesh.extrude_individual(value),
            ModalKind::Inset if value != 0.0 => mesh.inset_selected(value),
            ModalKind::Bevel if value != 0.0 => {
                let segs = self.tools.bevel_segments.clamp(1, 4);
                let clamp = self.tools.bevel_clamp_overlap;
                let affect_verts = self.tools.bevel_affect_vertices;
                let has_verts = mesh.verts.iter().any(|v| v.selected);
                let (applied, skipped) = if affect_verts && has_verts {
                    mesh.bevel_selected_vertex(value, clamp)
                } else {
                    mesh.bevel_selected_full(value, segs, clamp)
                };
                if applied == 0 || skipped > 0 {
                    return Err(ModalError::UnsupportedTopology);
                }
            }
            ModalKind::PushPull => mesh.translate_selected((direction * value).to_array()),
            _ => {}
        }
        self.publish_modal_mesh(mesh, value, components)?;
        if let Some(modal) = self.modal.as_mut() {
            modal.snap = snap;
        }
        Ok(())
    }

    /// Absolute XYZ property fields; rotation uses Euler XYZ angles in degrees.
    /// Each call rebuilds from the same transaction snapshot, avoiding drift.
    pub fn update_modal_components(&mut self, components: Vec3) -> Result<(), ModalError> {
        if !components.is_finite() || components.abs().max_element() > 1.0e6 {
            return Err(ModalError::InvalidInput);
        }
        let modal = self.modal.as_ref().ok_or(ModalError::NoActiveOperation)?;
        if !matches!(
            modal.kind,
            ModalKind::Move | ModalKind::Rotate | ModalKind::Scale
        ) || (modal.kind == ModalKind::Scale && components.abs().min_element() < 1.0e-6)
        {
            return Err(ModalError::InvalidInput);
        }
        let mut mesh = modal.source.clone();
        let rotation = Quat::from_euler(
            EulerRot::XYZ,
            components.x.to_radians(),
            components.y.to_radians(),
            components.z.to_radians(),
        );
        for (index, vertex) in mesh
            .verts
            .iter_mut()
            .enumerate()
            .filter(|(_, v)| v.selected)
        {
            let pivot = modal
                .pivots
                .as_ref()
                .map_or(modal.pivot, |pivots| pivots[index]);
            vertex.pos = match modal.kind {
                ModalKind::Move => {
                    if self.session.edit_pivot {
                        vertex.vec()
                    } else {
                        vertex.vec() + components
                    }
                }
                ModalKind::Rotate => pivot + rotation * (vertex.vec() - pivot),
                ModalKind::Scale => pivot + components * (vertex.vec() - pivot),
                _ => return Err(ModalError::InvalidInput),
            }
            .to_array();
        }
        let value = if modal.kind == ModalKind::Scale {
            components.x
        } else {
            components.length()
        };
        self.publish_modal_mesh(mesh, value, components)?;
        if let Some(modal) = self.modal.as_mut() {
            modal.constraint = ModalConstraint::Free;
        }
        Ok(())
    }

    fn publish_modal_mesh(
        &mut self,
        mut mesh: Mesh,
        value: f32,
        components: Vec3,
    ) -> Result<(), ModalError> {
        if !valid_mesh(&mesh) {
            return Err(ModalError::InvalidMesh);
        }
        let (
            modal_kind,
            modal_pivot,
            is_edit_pivot,
            is_object_mode,
            active_orig,
            active_orig_pos,
            active_orig_rot,
            active_orig_scale,
            others,
            mut changed,
        ) = {
            let modal = self.modal.as_ref().ok_or(ModalError::NoActiveOperation)?;
            let changed = !same_geometry(&mesh, &modal.source);
            let modal_kind = modal.kind;
            let modal_pivot = modal.pivot;
            let is_edit_pivot = self.session.edit_pivot;
            let is_object_mode = self.edit_mode() == EditMode::Object;
            let active_id = modal.original.active().map(|asset| asset.id);
            let active_orig = modal.original.active().and_then(|a| a.origin);
            let active_orig_pos = modal.original.active().map_or([0.0; 3], |a| a.position);
            let active_orig_rot = modal.original.active().map_or([0.0; 3], |a| a.rotation);
            let active_orig_scale = modal.original.active().map_or([1.0; 3], |a| a.scale);
            let selection_assets = &modal.selection.assets;
            let individual_origins = modal.individual_origins;

            let others: Vec<_> = if is_object_mode
                && matches!(
                    modal_kind,
                    ModalKind::Move | ModalKind::Rotate | ModalKind::Scale
                ) {
                let rotation = Quat::from_euler(
                    EulerRot::XYZ,
                    components.x.to_radians(),
                    components.y.to_radians(),
                    components.z.to_radians(),
                );
                modal
                    .original
                    .assets
                    .iter()
                    .filter(|asset| {
                        !asset.locked
                            && Some(asset.id) != active_id
                            && selection_assets.contains(&asset.id)
                    })
                    .map(|asset| {
                        let mut other = asset.mesh.clone();
                        let pivot = if individual_origins {
                            other.verts.iter().map(|v| v.vec()).sum::<Vec3>()
                                / other.verts.len().max(1) as f32
                        } else {
                            modal_pivot
                        };
                        for vertex in &mut other.verts {
                            vertex.pos = match modal_kind {
                                ModalKind::Move => vertex.vec() + components,
                                ModalKind::Rotate => pivot + rotation * (vertex.vec() - pivot),
                                ModalKind::Scale => pivot + components * (vertex.vec() - pivot),
                                _ => vertex.vec(),
                            }
                            .to_array();
                        }
                        let other_orig = asset.origin.map(|orig| {
                            let orig_vec = Vec3::from(orig);
                            match modal_kind {
                                ModalKind::Move => (orig_vec + components).to_array(),
                                ModalKind::Rotate => {
                                    (modal_pivot + rotation * (orig_vec - modal_pivot)).to_array()
                                }
                                ModalKind::Scale => {
                                    (modal_pivot + components * (orig_vec - modal_pivot)).to_array()
                                }
                                _ => orig,
                            }
                        });
                        let other_pos = match modal_kind {
                            ModalKind::Move => Some([
                                asset.position[0] + components.x,
                                asset.position[1] + components.y,
                                asset.position[2] + components.z,
                            ]),
                            _ => Some(asset.position),
                        };
                        let other_rot = match modal_kind {
                            ModalKind::Rotate => Some([
                                asset.rotation[0] + components.x,
                                asset.rotation[1] + components.y,
                                asset.rotation[2] + components.z,
                            ]),
                            _ => Some(asset.rotation),
                        };
                        let other_scale = match modal_kind {
                            ModalKind::Scale => Some([
                                asset.scale[0] * components.x,
                                asset.scale[1] * components.y,
                                asset.scale[2] * components.z,
                            ]),
                            _ => Some(asset.scale),
                        };
                        let is_changed = !same_geometry(&other, &asset.mesh);
                        (
                            asset.id,
                            other,
                            is_changed,
                            other_orig,
                            other_pos,
                            other_rot,
                            other_scale,
                        )
                    })
                    .collect()
            } else {
                Vec::new()
            };

            // Preserve selection flags for object transforms and identity previews.
            if (!changed
                || (is_object_mode
                    && matches!(
                        modal_kind,
                        ModalKind::Move | ModalKind::Rotate | ModalKind::Scale
                    )))
                && let Some(original) = modal.original.active_mesh()
            {
                for (vertex, source) in mesh.verts.iter_mut().zip(&original.verts) {
                    vertex.selected = source.selected;
                }
                for (face, source) in mesh.faces.iter_mut().zip(&original.faces) {
                    face.selected = source.selected;
                }
                mesh.selected_edges = original.selected_edges.clone();
            }

            (
                modal_kind,
                modal_pivot,
                is_edit_pivot,
                is_object_mode,
                active_orig,
                active_orig_pos,
                active_orig_rot,
                active_orig_scale,
                others,
                changed,
            )
        };

        if others
            .iter()
            .any(|(_, mesh, _, _, _, _, _)| !valid_mesh(mesh))
        {
            return Err(ModalError::InvalidMesh);
        }
        changed |= others
            .iter()
            .any(|(_, _, is_changed, _, _, _, _)| *is_changed);

        let active = self.project.active_mut().ok_or(ModalError::NoActiveMesh)?;
        active.mesh = mesh;
        if is_edit_pivot && modal_kind == ModalKind::Move {
            active.origin = Some((modal_pivot + components).to_array());
            changed = true;
        } else if is_object_mode && let Some(orig) = active_orig {
            let orig_vec = Vec3::from(orig);
            active.origin = Some(match modal_kind {
                ModalKind::Move => (orig_vec + components).to_array(),
                ModalKind::Rotate => {
                    let rotation = Quat::from_euler(
                        EulerRot::XYZ,
                        components.x.to_radians(),
                        components.y.to_radians(),
                        components.z.to_radians(),
                    );
                    (modal_pivot + rotation * (orig_vec - modal_pivot)).to_array()
                }
                ModalKind::Scale => {
                    (modal_pivot + components * (orig_vec - modal_pivot)).to_array()
                }
                _ => orig,
            });
        }
        if is_object_mode && !is_edit_pivot {
            match modal_kind {
                ModalKind::Move => {
                    active.position = [
                        active_orig_pos[0] + components.x,
                        active_orig_pos[1] + components.y,
                        active_orig_pos[2] + components.z,
                    ];
                }
                ModalKind::Rotate => {
                    active.rotation = [
                        active_orig_rot[0] + components.x,
                        active_orig_rot[1] + components.y,
                        active_orig_rot[2] + components.z,
                    ];
                }
                ModalKind::Scale => {
                    active.scale = [
                        active_orig_scale[0] * components.x,
                        active_orig_scale[1] * components.y,
                        active_orig_scale[2] * components.z,
                    ];
                }
                _ => {}
            }
        }
        for (id, other_mesh, _, other_orig, other_pos, other_rot, other_scale) in others {
            if let Some(asset) = self.project.assets.iter_mut().find(|asset| asset.id == id) {
                asset.mesh = other_mesh;
                if is_object_mode && other_orig.is_some() {
                    asset.origin = other_orig;
                }
                if is_object_mode && !is_edit_pivot {
                    if let Some(p) = other_pos {
                        asset.position = p;
                    }
                    if let Some(r) = other_rot {
                        asset.rotation = r;
                    }
                    if let Some(s) = other_scale {
                        asset.scale = s;
                    }
                }
            }
        }
        if let Some(modal) = self.modal.as_mut() {
            modal.value = value;
            modal.components = components;
            modal.changed = changed;
        }
        self.sync_selection();
        let mut changes = match modal_kind {
            ModalKind::Move | ModalKind::Rotate | ModalKind::Scale => ProjectChanges::POSITIONS,
            _ => ProjectChanges::GEOMETRY,
        };
        if is_object_mode || is_edit_pivot {
            changes |= ProjectChanges::TRANSFORMS;
        }
        self.emit_project_changed(changes);
        Ok(())
    }

    pub fn commit_modal(&mut self) -> bool {
        let Some(modal) = self.modal.take() else {
            return false;
        };
        let mut restored_changes = None;
        if let Some(before) = modal.before_prelude {
            let (before, selection) = *before;
            if modal.changed {
                self.project.undo.checkpoint_sized(
                    modal.kind.label(),
                    &before,
                    before.estimated_bytes(),
                );
                self.mark_document_dirty();
            } else {
                self.restore_before_prelude(before, selection);
            }
        } else if modal.changed {
            self.project.undo.checkpoint_sized(
                modal.kind.label(),
                &modal.original,
                modal.original.estimated_bytes(),
            );
            self.mark_document_dirty();
        } else {
            let revision_clock = self.project.project.revision_clock();
            let previewed = revision_clock != modal.original.revision_clock();
            self.project.project = modal.original;
            self.session.selection = modal.selection;
            if previewed {
                self.project
                    .project
                    .rebase_revisions_after_restore(revision_clock);
                restored_changes = Some(match modal.kind {
                    ModalKind::Move | ModalKind::Rotate | ModalKind::Scale => {
                        ProjectChanges::POSITIONS | ProjectChanges::TRANSFORMS
                    }
                    _ => ProjectChanges::GEOMETRY,
                });
            }
        }
        self.pointer_session = None;
        self.pending_modal = None;
        self.locked_axes = [false; 3];
        if let Some(changes) = restored_changes {
            self.emit_project_changed(changes);
        }
        true
    }

    pub fn cancel_modal(&mut self) -> bool {
        self.pointer_session = None;
        self.pending_modal = None;
        let Some(modal) = self.modal.take() else {
            return false;
        };
        if let Some(before) = modal.before_prelude {
            let (before, selection) = *before;
            self.restore_before_prelude(before, selection);
            self.locked_axes = [false; 3];
            return true;
        }
        let revision_clock = self.project.project.revision_clock();
        self.project.project = modal.original;
        self.project
            .project
            .rebase_revisions_after_restore(revision_clock);
        self.session.selection = modal.selection;
        self.locked_axes = [false; 3];
        self.events.emit(crate::AppEvent::SelectionChanged(
            self.session.selection.clone(),
        ));
        let changes = match modal.kind {
            ModalKind::Move | ModalKind::Rotate | ModalKind::Scale => {
                ProjectChanges::POSITIONS | ProjectChanges::TRANSFORMS
            }
            _ => ProjectChanges::GEOMETRY,
        };
        self.emit_project_changed(changes);
        true
    }
}

/// Sessão interativa de transformação modal por ponteiro e teclado numérico.
#[derive(Debug, Clone)]
pub struct PointerSession {
    /// Posição 2D de tela inicial do clique/arrasto (pixels lógicos).
    pub anchor: [f32; 2],
    /// Buffer de entrada de texto numérico (ex: "1.5", "-45").
    pub numeric: String,
    /// Flag indicando se a sessão foi iniciada via arraste de gizmo.
    pub drag_handle: bool,
    /// Flag indicando se a pré-visualização atual é válida.
    pub valid_preview: bool,
    /// Última posição 2D de tela registrada.
    pub last_pos: [f32; 2],
}

impl PointerSession {
    pub fn new(anchor: [f32; 2], drag_handle: bool) -> Self {
        Self {
            anchor,
            numeric: String::new(),
            drag_handle,
            valid_preview: true,
            last_pos: anchor,
        }
    }

    pub fn push_char(&mut self, c: char) -> bool {
        if (c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '+')) && self.numeric.len() < 64 {
            self.numeric.push(if c == ',' { '.' } else { c });
            true
        } else {
            false
        }
    }

    pub fn pop_char(&mut self) -> Option<char> {
        self.numeric.pop()
    }

    pub fn parse_numeric(&self) -> Option<f32> {
        self.numeric.parse::<f32>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selected_face() -> AppState {
        let mut state = AppState::new("en");
        state.set_edit_mode(EditMode::Edit);
        let mesh = state.project.active_mesh_mut().unwrap();
        mesh.deselect_all();
        mesh.faces[0].selected = true;
        mesh.sync_vert_selection_from_faces();
        state.sync_selection();
        state
    }

    // Compare every mesh attribute, including UVs, colors and selection flags.
    fn assert_exact_mesh(actual: &Mesh, expected: &Mesh) {
        assert!(same_geometry(actual, expected));
        assert_eq!(actual.selected_edges, expected.selected_edges);
        assert_eq!(
            actual.verts.iter().map(|v| v.selected).collect::<Vec<_>>(),
            expected
                .verts
                .iter()
                .map(|v| v.selected)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            actual.faces.iter().map(|f| f.selected).collect::<Vec<_>>(),
            expected
                .faces
                .iter()
                .map(|f| f.selected)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn numeric_xyz_fields_rebuild_absolute_transform_and_commit_once() {
        for (kind, values) in [
            (ModalKind::Move, Vec3::new(1.25, -2.0, 0.5)),
            (ModalKind::Rotate, Vec3::new(90.0, 0.0, 0.0)),
            (ModalKind::Scale, Vec3::new(2.0, 0.5, -1.0)),
        ] {
            let mut state = selected_face();
            let original = state.project.active_mesh().unwrap().clone();
            state.begin_modal(kind).unwrap();
            let pivot = state.modal.as_ref().unwrap().pivot;
            state.update_modal_components(Vec3::splat(0.25)).unwrap();
            state.update_modal_components(values).unwrap();
            for (before, after) in original
                .verts
                .iter()
                .zip(&state.project.active_mesh().unwrap().verts)
            {
                let relative = before.vec() - pivot;
                let expected = if before.selected {
                    match kind {
                        ModalKind::Move => before.vec() + values,
                        ModalKind::Rotate => pivot + Vec3::new(relative.x, -relative.z, relative.y),
                        ModalKind::Scale => pivot + relative * values,
                        _ => unreachable!(),
                    }
                } else {
                    before.vec()
                };
                assert!((after.vec() - expected).length() < 1.0e-5, "{kind:?}");
            }
            assert_eq!(state.modal.as_ref().unwrap().components, values);
            assert_eq!(state.project.undo.depth(), (0, 0));
            state.commit_modal();
            assert_eq!(state.project.undo.depth(), (1, 0));
            state.undo();
            assert_exact_mesh(state.project.active_mesh().unwrap(), &original);
        }
    }

    #[test]
    fn invalid_numeric_axis_preserves_last_valid_preview() {
        let mut state = selected_face();
        state.begin_modal(ModalKind::Scale).unwrap();
        state
            .update_modal_components(Vec3::new(1.0, 2.0, 3.0))
            .unwrap();
        let preview = state.project.active_mesh().unwrap().clone();
        for invalid in [
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::splat(f32::NAN),
            Vec3::splat(1.0e7),
        ] {
            assert_eq!(
                state.update_modal_components(invalid),
                Err(ModalError::InvalidInput)
            );
            assert_exact_mesh(state.project.active_mesh().unwrap(), &preview);
        }
        assert_eq!(state.project.undo.depth(), (0, 0));
    }

    #[test]
    fn cancel_restores_every_operation_without_touching_history() {
        for kind in [
            ModalKind::Move,
            ModalKind::Rotate,
            ModalKind::Scale,
            ModalKind::Extrude,
            ModalKind::Inset,
            ModalKind::Bevel,
            ModalKind::PushPull,
        ] {
            let mut state = selected_face();
            if kind == ModalKind::Bevel {
                state
                    .project
                    .active_mesh_mut()
                    .unwrap()
                    .selected_edges
                    .insert((0, 1));
            }
            state.checkpoint("prior");
            let before = state.project.clone();
            state.begin_modal(kind).unwrap();
            state.update_modal(Vec3::new(0.4, 0.2, 0.0), 0.25).unwrap();
            state.update_modal(Vec3::new(0.8, 0.4, 0.0), 0.5).unwrap();
            assert!(state.cancel_modal());
            assert_eq!(state.project.undo.depth(), (1, 0), "{kind:?}");
            assert_exact_mesh(
                state.project.active_mesh().unwrap(),
                before.active_mesh().unwrap(),
            );
        }
    }

    #[test]
    fn modal_bevel_supports_single_vertex_selection() {
        let mut state = AppState::default();
        let mesh = state.project.active_mesh_mut().unwrap();
        mesh.selected_edges.clear();
        for f in &mut mesh.faces {
            f.selected = false;
        }
        for v in &mut mesh.verts {
            v.selected = false;
        }
        mesh.verts[0].selected = true;

        assert!(state.begin_modal(ModalKind::Bevel).is_ok());
        assert!(state.update_modal(Vec3::ZERO, 0.2).is_ok());
        assert!(state.commit_modal());

        let result = state.project.active_mesh().unwrap();
        let report = result.validate_topology();
        assert!(report.is_manifold && report.is_closed);
        assert_eq!((result.verts.len(), result.faces.len()), (10, 7));
    }

    #[test]
    fn modal_bevel_supports_multiple_vertices_batch() {
        let mut state = AppState::default();
        let mesh = state.project.active_mesh_mut().unwrap();
        mesh.selected_edges.clear();
        for f in &mut mesh.faces {
            f.selected = false;
        }
        for v in &mut mesh.verts {
            v.selected = true; // todos os 8 vértices
        }

        assert!(state.begin_modal(ModalKind::Bevel).is_ok());
        assert!(state.update_modal(Vec3::ZERO, 0.2).is_ok());
        assert!(state.commit_modal());

        let result = state.project.active_mesh().unwrap();
        let report = result.validate_topology();
        assert!(report.is_manifold && report.is_closed);
        assert_eq!((result.verts.len(), result.faces.len()), (24, 14));
    }

    #[test]
    fn multiple_previews_commit_one_checkpoint_and_roundtrip() {
        let mut state = selected_face();
        let original = state.project.active_mesh().unwrap().clone();
        state.begin_modal(ModalKind::Move).unwrap();
        for value in [0.2, 0.3, 0.8] {
            state.update_modal(Vec3::X * value, value).unwrap();
            assert_eq!(state.project.undo.depth(), (0, 0));
        }
        let result = state.project.active_mesh().unwrap().clone();
        for (before, after) in original.verts.iter().zip(&result.verts) {
            let expected = before.vec()
                + if before.selected {
                    Vec3::X * 0.8
                } else {
                    Vec3::ZERO
                };
            assert!((after.vec() - expected).length() < 1.0e-6);
        }
        assert!(state.commit_modal());
        assert_eq!(state.project.undo.depth(), (1, 0));
        assert!(state.undo());
        assert_exact_mesh(state.project.active_mesh().unwrap(), &original);
        assert!(state.redo());
        assert_exact_mesh(state.project.active_mesh().unwrap(), &result);
    }

    #[test]
    fn topology_undo_and_redo_resynchronize_cached_selection() {
        let mut state = selected_face();
        let original_verts = state.selection.verts.clone();
        state.begin_modal(ModalKind::Extrude).unwrap();
        state.update_modal(Vec3::ZERO, 0.5).unwrap();
        let extruded_verts = state.selection.verts.clone();
        assert_ne!(extruded_verts, original_verts);
        state.commit_modal();
        state.undo();
        assert_eq!(state.selection.verts, original_verts);
        state.redo();
        assert_eq!(state.selection.verts, extruded_verts);
    }

    #[test]
    fn cancel_preserves_redo_and_undo_cancels_active_preview() {
        let mut state = selected_face();
        state.checkpoint("prior");
        state
            .project
            .active_mesh_mut()
            .unwrap()
            .translate_selected([1.0, 0.0, 0.0]);
        state.undo();
        let original = state.project.active_mesh().unwrap().clone();
        state.begin_modal(ModalKind::Extrude).unwrap();
        state.update_modal(Vec3::ZERO, 1.0).unwrap();
        assert!(state.undo());
        assert_eq!(state.project.undo.depth(), (0, 1));
        assert_exact_mesh(state.project.active_mesh().unwrap(), &original);
    }

    #[test]
    fn identity_after_preview_does_not_create_history_or_geometry() {
        for kind in [
            ModalKind::Move,
            ModalKind::Extrude,
            ModalKind::Inset,
            ModalKind::Scale,
        ] {
            let mut state = selected_face();
            let original = state.project.active_mesh().unwrap().clone();
            state.begin_modal(kind).unwrap();
            state.update_modal(Vec3::X, 0.2).unwrap();
            state
                .update_modal(Vec3::ZERO, if kind == ModalKind::Scale { 1.0 } else { 0.0 })
                .unwrap();
            state.commit_modal();
            assert_eq!(state.project.undo.depth(), (0, 0));
            assert_exact_mesh(state.project.active_mesh().unwrap(), &original);
        }
    }

    #[test]
    fn axes_and_planes_constrain_absolute_translation() {
        let mut state = selected_face();
        let original = state.project.active_mesh().unwrap().clone();
        state.begin_modal(ModalKind::Move).unwrap();
        state
            .set_modal_constraint(ModalConstraint::Axis(2))
            .unwrap();
        state.update_modal(Vec3::splat(9.0), -1.25).unwrap();
        let vertex = original.verts.iter().position(|v| v.selected).unwrap();
        assert_eq!(
            state.project.active_mesh().unwrap().verts[vertex].vec(),
            original.verts[vertex].vec() - Vec3::Z * 1.25
        );
        state
            .set_modal_constraint(ModalConstraint::Plane(1))
            .unwrap();
        state.update_modal(Vec3::new(1.0, 2.0, 3.0), 0.0).unwrap();
        assert_eq!(
            state.project.active_mesh().unwrap().verts[vertex].vec(),
            original.verts[vertex].vec() + Vec3::new(1.0, 0.0, 3.0)
        );
    }

    #[test]
    fn rotation_and_plane_scale_preserve_pivot_and_constrained_coordinates() {
        let mut state = selected_face();
        let original = state.project.active_mesh().unwrap().clone();
        state.begin_modal(ModalKind::Rotate).unwrap();
        let pivot = state.modal.as_ref().unwrap().pivot;
        state
            .set_modal_constraint(ModalConstraint::Axis(1))
            .unwrap();
        state.update_modal(Vec3::ZERO, 90.0).unwrap();
        for (before, after) in original
            .verts
            .iter()
            .zip(&state.project.active_mesh().unwrap().verts)
        {
            if before.selected {
                let relative = before.vec() - pivot;
                let expected = pivot + Vec3::new(relative.z, relative.y, -relative.x);
                assert!((after.vec() - expected).length() < 1.0e-5);
            } else {
                assert_eq!(after.pos, before.pos);
            }
        }
        state.cancel_modal();
        state.begin_modal(ModalKind::Scale).unwrap();
        state
            .set_modal_constraint(ModalConstraint::Plane(2))
            .unwrap();
        state.update_modal(Vec3::ZERO, 2.0).unwrap();
        for (before, after) in original
            .verts
            .iter()
            .zip(&state.project.active_mesh().unwrap().verts)
        {
            if before.selected {
                assert_eq!(
                    after.vec(),
                    pivot + (before.vec() - pivot) * Vec3::new(2.0, 2.0, 1.0)
                );
            } else {
                assert_eq!(after.pos, before.pos);
            }
        }
    }

    #[test]
    fn invalid_inputs_leave_last_preview_untouched() {
        let mut state = selected_face();
        state.begin_modal(ModalKind::Move).unwrap();
        state.update_modal(Vec3::X, 1.0).unwrap();
        let preview = state.project.active_mesh().unwrap().clone();
        assert_eq!(
            state.update_modal(Vec3::ZERO, f32::NAN),
            Err(ModalError::InvalidInput)
        );
        assert_eq!(
            state.update_modal(Vec3::splat(f32::INFINITY), 1.0),
            Err(ModalError::InvalidInput)
        );
        assert_eq!(
            state.set_modal_constraint(ModalConstraint::Axis(99)),
            Err(ModalError::InvalidInput)
        );
        assert_exact_mesh(state.project.active_mesh().unwrap(), &preview);
        assert_eq!(state.project.undo.depth(), (0, 0));
    }

    #[test]
    fn corrupted_mesh_rejected_before_indexing() {
        let mut state = selected_face();
        state.project.active_mesh_mut().unwrap().faces[0].verts[0] = u32::MAX;
        assert_eq!(
            state.begin_modal(ModalKind::Extrude),
            Err(ModalError::InvalidMesh)
        );
        assert!(state.modal.is_none());
    }

    #[test]
    fn empty_selection_rejected_in_edit_mode_and_object_mode_transforms_whole_mesh() {
        let mut state = AppState::new("en");
        state.project.active_mesh_mut().unwrap().deselect_all();
        state.set_edit_mode(EditMode::Edit);
        assert_eq!(
            state.begin_modal(ModalKind::Move),
            Err(ModalError::NoSelection)
        );
        state.set_edit_mode(EditMode::Object);
        let original = state.project.active_mesh().unwrap().clone();
        state.begin_modal(ModalKind::Move).unwrap();
        state.update_modal(Vec3::Y, 1.0).unwrap();
        state.commit_modal();
        for (before, after) in original
            .verts
            .iter()
            .zip(&state.project.active_mesh().unwrap().verts)
        {
            assert_eq!(after.vec(), before.vec() + Vec3::Y);
            assert!(!after.selected);
        }
    }

    #[test]
    fn locked_asset_rejects_begin_modal() {
        let mut state = AppState::new("en");
        state.toggle_lock_active();
        assert!(state.is_active_locked());
        assert_eq!(
            state.begin_modal(ModalKind::Move),
            Err(ModalError::ActiveLocked)
        );
    }

    #[test]
    fn axis_lock_initialization_and_toggling() {
        let mut state = selected_face();

        // 1. Fora do modal: toggle e labels
        assert_eq!(state.locked_axes, [false; 3]);
        assert!(!state.is_axis_locked(0));
        assert!(state.active_axis_constraint_label().is_none());

        state.toggle_axis_lock(0);
        assert!(state.is_axis_locked(0));
        assert_eq!(
            state.active_axis_constraint_label(),
            Some(("Eixo X", [235, 75, 75]))
        );

        // 2. Herança para o modal
        state.begin_modal(ModalKind::Move).unwrap();
        assert_eq!(
            state.modal.as_ref().map(|m| m.constraint),
            Some(ModalConstraint::Axis(0))
        );
        assert!(state.is_axis_locked(0));

        // 3. Alternância dentro do modal
        state.toggle_axis_lock(1); // ativa eixo Y
        assert_eq!(
            state.modal.as_ref().map(|m| m.constraint),
            Some(ModalConstraint::Axis(1))
        );
        assert!(!state.is_axis_locked(0));
        assert!(state.is_axis_locked(1));
        assert_eq!(
            state.active_axis_constraint_label(),
            Some(("Eixo Y", [85, 195, 100]))
        );

        // 4. Commit reseta o travamento
        state.commit_modal();
        assert_eq!(state.locked_axes, [false; 3]);
        assert!(!state.is_axis_locked(1));
        assert!(state.active_axis_constraint_label().is_none());

        // 5. Cancelamento também reseta o travamento
        state.locked_axes = [true, false, true]; // Plano XZ pré-configurado
        assert_eq!(
            state.active_axis_constraint_label(),
            Some(("Plano XZ", [142, 68, 173]))
        );
        state.begin_modal(ModalKind::Scale).unwrap();
        assert_eq!(
            state.modal.as_ref().map(|m| m.constraint),
            Some(ModalConstraint::Plane(1))
        );
        state.cancel_modal();
        assert_eq!(state.locked_axes, [false; 3]);
    }

    #[test]
    fn feedback_reports_snap_only_when_the_caller_snapped() {
        let mut state = selected_face();
        state.snap_enabled = true;
        state.session.snap_settings.enabled = true;
        state.begin_modal(ModalKind::Move).unwrap();
        let pivot = state.modal.as_ref().unwrap().pivot;
        let vertex = state.modal.as_ref().unwrap().source.verts[0].vec();

        // Snap ligado não encaixa nada no core: uma única passada, feita por
        // quem conhece a tela (P3D-040). O ponto sobre um vértice fica livre.
        state
            .update_modal(vertex - pivot, (vertex - pivot).length())
            .unwrap();
        let feedback = state.current_tool_feedback().unwrap();
        assert!(!feedback.is_snapped, "snap ligado não significa encaixe");
        assert_eq!(feedback.snap_kind, None);

        state
            .update_modal_snapped(
                vertex - pivot,
                (vertex - pivot).length(),
                Some(SnapKind::Point),
            )
            .unwrap();
        let feedback = state.current_tool_feedback().unwrap();
        assert!(feedback.is_snapped);
        assert_eq!(feedback.snap_kind, Some(SnapKind::Point));
        assert!((feedback.current - vertex).length() < 1.0e-4);

        // O próximo update sem encaixe limpa o rótulo.
        state.update_modal(Vec3::new(0.0, 0.0, 3.0), 3.0).unwrap();
        assert!(!state.current_tool_feedback().unwrap().is_snapped);
    }

    #[test]
    fn rotation_and_scale_have_no_world_point_or_guide_line() {
        let mut state = selected_face();
        state.begin_modal(ModalKind::Rotate).unwrap();
        state.update_modal(Vec3::ZERO, 45.0).unwrap();
        assert!(state.modal.as_ref().unwrap().current_point().is_none());
        let feedback = state.current_tool_feedback().unwrap();
        assert!(
            feedback.guide_line.is_none(),
            "graus não viram posição de mundo"
        );
        state.cancel_modal();

        state.begin_modal(ModalKind::Move).unwrap();
        state.update_modal(Vec3::new(1.0, 0.0, 0.0), 1.0).unwrap();
        let modal = state.modal.as_ref().unwrap();
        assert_eq!(modal.current_point(), Some(modal.pivot + Vec3::X));
    }

    #[test]
    fn test_proportional_editing_move_influences_unselected_vertices() {
        let mut state = selected_face();
        state.proportional_editing = true;
        state.session.proportional_settings.radius = 3.0;
        state.session.proportional_settings.falloff =
            crate::proportional::ProportionalFalloff::Linear;

        state.begin_modal(ModalKind::Move).unwrap();
        state.update_modal(Vec3::new(0.0, 1.0, 0.0), 1.0).unwrap();

        let mesh = state.project.active_mesh().unwrap();
        let selected_count = mesh.verts.iter().filter(|v| v.selected).count();
        let unselected_moved_count = mesh
            .verts
            .iter()
            .filter(|v| !v.selected && v.pos[1] > -1.0)
            .count();

        assert_eq!(selected_count, 4);
        assert!(unselected_moved_count > 0);
        state.commit_modal();
    }

    #[test]
    fn test_modal_tool_feedback_generation() {
        let mut state = selected_face();
        assert!(state.current_tool_feedback().is_none());

        state.begin_modal(ModalKind::Move).unwrap();
        state.update_modal(Vec3::new(1.0, 0.0, 0.0), 1.0).unwrap();

        let fb = state.current_tool_feedback().expect("feedback generated");
        assert!(fb.guide_line.is_some());
        assert!(fb.delta_text.contains("1.00"));
        assert!(!fb.is_snapped);

        state.cancel_modal();
        assert!(state.current_tool_feedback().is_none());
    }
}
