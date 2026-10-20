//! Explicit origin editing. Cursor and transform-center policy remain session data.
use crate::{Command, CommandContext, CommandDispatcher, CommandError, EditorView};
use glam::Vec3;
use petunia_project::{Asset, ProjectChanges};
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub enum OriginPreset {
    Geometry,
    Bottom,
    Cursor,
    Selection,
    GeometryToOrigin,
    /// Origem do mundo (0, 0, 0).
    World,
    /// Um ponto de mundo escolhido pelo usuário (ex.: a posição sob o mouse).
    Point([f32; 3]),
}

#[derive(Debug, Clone)]
pub struct SetObjectOriginCmd {
    pub object: Uuid,
    pub world_point: [f32; 3],
    pub revision: [u64; 11],
}

#[derive(Debug, Clone, Copy)]
pub struct OriginPresetCmd(pub OriginPreset);

impl Command for SetObjectOriginCmd {
    fn freezes_active_primitive(&self) -> bool {
        false
    }

    fn label(&self) -> &'static str {
        "Set Object Origin"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::POSITIONS | ProjectChanges::TRANSFORMS
    }

    fn can_execute(&self, view: &EditorView<'_>) -> Result<(), &'static str> {
        let asset = view
            .document
            .assets
            .iter()
            .find(|a| a.id == self.object)
            .ok_or("Origin target no longer exists")?;
        validate_asset(asset)
    }

    fn execute(&self, ctx: &mut CommandContext<'_>) -> Result<(), CommandError> {
        if ctx.document.project.revision_clock() != self.revision {
            return Err(CommandError::StaleReference("Origin source changed".into()));
        }
        let asset = ctx
            .document
            .assets
            .iter_mut()
            .find(|a| a.id == self.object)
            .ok_or_else(|| CommandError::StaleReference(self.object.to_string()))?;
        if asset.origin.is_none() && asset.transform.translation == self.world_point {
            return Err(CommandError::NoChange("Origin unchanged".into()));
        }
        asset
            .rebase_origin_world(Vec3::from(self.world_point))
            .map_err(|e| CommandError::Execution(e.to_string()))
    }
}

impl Command for OriginPresetCmd {
    fn freezes_active_primitive(&self) -> bool {
        false
    }

    fn label(&self) -> &'static str {
        match self.0 {
            OriginPreset::Geometry => "Set Origin to Geometry",
            OriginPreset::Bottom => "Set Origin to Bottom",
            OriginPreset::Cursor => "Set Origin to 3D Cursor",
            OriginPreset::Selection => "Set Origin to Selection",
            OriginPreset::GeometryToOrigin => "Geometry to Origin",
            OriginPreset::World => "Set Origin to World Origin",
            OriginPreset::Point(_) => "Set Origin to Point",
        }
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::POSITIONS | ProjectChanges::TRANSFORMS
    }

    fn can_execute(&self, view: &EditorView<'_>) -> Result<(), &'static str> {
        validate_asset(view.document.active().ok_or("No active asset")?)
    }

    fn execute(&self, ctx: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let asset = ctx.document.active().ok_or(CommandError::NoActiveAsset)?;
        let point = match self.0 {
            OriginPreset::Cursor => Vec3::from(ctx.editor.cursor_3d),
            OriginPreset::Bottom => {
                let (min, max) = asset.world_positions().into_iter().fold(
                    (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
                    |(min, max), p| (min.min(p), max.max(p)),
                );
                let center = min * 0.5 + max * 0.5;
                Vec3::new(center.x, min.y, center.z)
            }
            OriginPreset::World => Vec3::ZERO,
            OriginPreset::Point(point) => {
                let point = Vec3::from(point);
                if !point.is_finite() {
                    return Err(CommandError::Execution("Non-finite origin point".into()));
                }
                point
            }
            OriginPreset::Geometry => {
                let center = mean(asset.mesh.verts.iter().map(|v| v.vec()))?;
                asset.to_world_point(center)
            }
            OriginPreset::Selection => selection_center_world(asset)?,
            OriginPreset::GeometryToOrigin => {
                let center = mean(asset.mesh.verts.iter().map(|v| v.vec()))?;
                let delta = asset.origin.map_or(Vec3::ZERO, Vec3::from) - center;
                if delta == Vec3::ZERO {
                    return Err(CommandError::NoChange("Geometry already centered".into()));
                }
                let mut mesh = (*asset.mesh).clone();
                for vertex in &mut mesh.verts {
                    let next = vertex.vec() + delta;
                    if !next.is_finite() || !asset.to_world_point(next).is_finite() {
                        return Err(CommandError::Execution(
                            "Geometry translation overflow".into(),
                        ));
                    }
                    vertex.pos = next.to_array();
                }
                let active = ctx
                    .document
                    .active_mut()
                    .ok_or(CommandError::NoActiveAsset)?;
                active.mesh = mesh.into();
                return Ok(());
            }
        };
        SetObjectOriginCmd {
            object: asset.id,
            world_point: point.to_array(),
            revision: ctx.document.project.revision_clock(),
        }
        .execute(ctx)
    }
}

impl CommandContext<'_> {
    pub fn set_origin_geometry(&mut self) -> Result<(), &'static str> {
        self.origin_preset(OriginPreset::Geometry)
    }

    pub fn set_origin_bottom(&mut self) -> Result<(), &'static str> {
        self.origin_preset(OriginPreset::Bottom)
    }

    pub fn set_origin_cursor(&mut self) -> Result<(), &'static str> {
        self.origin_preset(OriginPreset::Cursor)
    }

    pub fn set_origin_selection(&mut self) -> Result<(), &'static str> {
        self.origin_preset(OriginPreset::Selection)
    }

    pub fn set_origin_world(&mut self) -> Result<(), &'static str> {
        self.origin_preset(OriginPreset::World)
    }

    pub fn set_origin_point(&mut self, point: Vec3) -> Result<(), &'static str> {
        self.origin_preset(OriginPreset::Point(point.to_array()))
    }

    pub fn set_geometry_to_origin(&mut self) -> Result<(), &'static str> {
        self.origin_preset(OriginPreset::GeometryToOrigin)
    }

    fn origin_preset(&mut self, preset: OriginPreset) -> Result<(), &'static str> {
        let cmd = OriginPresetCmd(preset);
        match CommandDispatcher::run(self, &cmd) {
            Ok(()) => {
                self.set_status(cmd.label());
                Ok(())
            }
            Err(error) => {
                self.set_status(error.to_string());
                Err("Origin operation rejected; see status for details")
            }
        }
    }
}

/// Centro (em mundo) dos elementos selecionados do objeto, seja qual for o
/// domínio: vértices marcados, vértices das faces selecionadas e extremos das
/// arestas selecionadas. Uma aresta dá o ponto médio, uma face o centroide e
/// vários elementos a média dos vértices envolvidos.
pub fn selection_center_world(asset: &Asset) -> Result<Vec3, CommandError> {
    let mesh = &asset.mesh;
    let mut marked = vec![false; mesh.verts.len()];
    let mut mark = |index: usize| {
        if let Some(slot) = marked.get_mut(index) {
            *slot = true;
        }
    };
    for (index, vertex) in mesh.verts.iter().enumerate() {
        if vertex.selected {
            mark(index);
        }
    }
    for face in mesh.faces.iter().filter(|face| face.selected) {
        for &vertex in &face.verts {
            mark(vertex as usize);
        }
    }
    for &(a, b) in &mesh.selected_edges {
        mark(a as usize);
        mark(b as usize);
    }
    let center = mean(
        mesh.verts
            .iter()
            .zip(&marked)
            .filter(|(_, marked)| **marked)
            .map(|(vertex, _)| vertex.vec()),
    )?;
    let world = asset.to_world_point(center);
    if world.is_finite() {
        Ok(world)
    } else {
        Err(CommandError::Execution("Non-finite geometry center".into()))
    }
}

/// Motivo pelo qual a origem do objeto não pode ser movida, ou `None` se pode.
pub fn origin_refusal(asset: &Asset) -> Option<&'static str> {
    validate_asset(asset).err()
}

fn validate_asset(asset: &Asset) -> Result<(), &'static str> {
    use petunia_project::origin::OriginError;
    asset.validate_origin_rebase().map_err(|e| match e {
        OriginError::Locked => "Asset is locked",
        OriginError::Empty => "Asset mesh is empty",
        OriginError::InvalidFrame => "Invalid object frame or geometry",
        OriginError::Parametric => "Use Make Editable before changing a parametric origin",
        OriginError::Modifiers => "Apply or remove modifiers explicitly before changing origin",
        OriginError::Rigged => "Rigged origin rebase requires bind compensation",
    })
}

fn mean(points: impl Iterator<Item = Vec3>) -> Result<Vec3, CommandError> {
    let mut sum = glam::DVec3::ZERO;
    let mut count = 0_u64;
    for point in points {
        sum += point.as_dvec3();
        count = count.saturating_add(1);
    }
    if count == 0 {
        return Err(CommandError::EmptySelection);
    }
    let center = (sum / count as f64).as_vec3();
    if !center.is_finite() {
        return Err(CommandError::Execution("Non-finite geometry center".into()));
    }
    Ok(center)
}
