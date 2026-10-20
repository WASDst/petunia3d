//! Transactional creation, parameter editing and explicit conversion of generated objects.
use crate::{Command, CommandContext, CommandError, EditorView};
use petunia_project::{Asset, GeometryGenerator, ProjectChanges};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CreateGeometryGeneratorCmd {
    pub object_id: Uuid,
    pub name: String,
    pub generator: GeometryGenerator,
    pub source_revision: [u64; 2],
}

#[derive(Debug, Clone)]
pub struct UpdateGeometryGeneratorCmd {
    pub object_id: Uuid,
    pub generator: GeometryGenerator,
    pub expected_revision: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct MakeGeneratedEditableCmd {
    pub object_id: Uuid,
    pub expected_revision: u64,
}

impl Command for CreateGeometryGeneratorCmd {
    fn label(&self) -> &'static str {
        "create shape generator"
    }
    fn changes(&self) -> ProjectChanges {
        ProjectChanges::GEOMETRY | ProjectChanges::PROCEDURAL | ProjectChanges::SELECTION
    }
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let project = &state.document.project;
        if self.name.trim().is_empty() || project.assets.iter().any(|a| a.id == self.object_id) {
            return Err("Invalid or duplicate generated object");
        }
        if [project.spline_revision, project.procedural_revision] != self.source_revision {
            return Err("The shape changed; restart the volume preview");
        }
        if project
            .assets
            .iter()
            .filter_map(|a| a.geometry_source.as_ref())
            .any(|g| g.id == self.generator.id)
        {
            return Err("Generator identifier already exists");
        }
        self.generator
            .evaluate(project)
            .map(|_| ())
            .map_err(|_| "Generator source or parameters are invalid")
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let mesh = self
            .generator
            .evaluate(&state.document.project)
            .map_err(|e| CommandError::Execution(e.to_string()))?;
        let mut asset = Asset::new(&self.name, petunia_mesh::Mesh::default());
        asset.id = self.object_id;
        asset.geometry_source = Some(self.generator.clone());
        asset.generated_mesh = mesh.into();
        state.document.project.assets.push(asset);
        state.document.project.active = state.document.project.assets.len().saturating_sub(1);
        state.editor.active_shape = None;
        state.editor.planar_selection = Default::default();
        state.editor.selection.assets = vec![self.object_id];
        state.editor.selection.asset = Some(self.object_id);
        state.document.project.history_selection = vec![self.object_id];
        Ok(())
    }
}

impl Command for UpdateGeometryGeneratorCmd {
    fn label(&self) -> &'static str {
        "edit shape generator"
    }
    fn changes(&self) -> ProjectChanges {
        ProjectChanges::GEOMETRY | ProjectChanges::PROCEDURAL
    }
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let project = &state.document.project;
        let asset = project
            .assets
            .iter()
            .find(|a| a.id == self.object_id)
            .ok_or("Generated object not found")?;
        if asset.locked {
            return Err("Generated object is locked");
        }
        let source = asset
            .geometry_source
            .as_ref()
            .ok_or("Object is no longer parametric")?;
        if source.id != self.generator.id || source.revision != self.expected_revision {
            return Err("Generator revision is obsolete");
        }
        if source.kind == self.generator.kind {
            return Err("Generator parameters are unchanged");
        }
        self.generator
            .evaluate(project)
            .map(|_| ())
            .map_err(|_| "Generator source or parameters are invalid")
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let mesh = self
            .generator
            .evaluate(&state.document.project)
            .map_err(|e| CommandError::Execution(e.to_string()))?;
        let asset = state
            .document
            .project
            .assets
            .iter_mut()
            .find(|a| a.id == self.object_id)
            .ok_or(CommandError::NoActiveAsset)?;
        let mut source = self.generator.clone();
        source.revision = self.expected_revision.wrapping_add(1);
        asset.geometry_source = Some(source);
        asset.generated_mesh = mesh.into();
        asset.generator_diagnostic = None;
        Ok(())
    }
}

impl Command for MakeGeneratedEditableCmd {
    fn label(&self) -> &'static str {
        "make generated object editable"
    }
    fn changes(&self) -> ProjectChanges {
        ProjectChanges::GEOMETRY | ProjectChanges::PROCEDURAL
    }
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let asset = state
            .document
            .project
            .assets
            .iter()
            .find(|a| a.id == self.object_id)
            .ok_or("Generated object not found")?;
        if asset.locked || asset.skeleton_id.is_some() || asset.skin_data.is_some() {
            return Err("Object cannot be converted in this context");
        }
        let source = asset
            .geometry_source
            .as_ref()
            .ok_or("Object is already editable")?;
        if source.revision != self.expected_revision {
            return Err("Generator revision is obsolete");
        }
        source
            .evaluate(&state.document.project)
            .map(|_| ())
            .map_err(|_| "Cannot convert an invalid generator")
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let project = &state.document.project;
        let asset = project
            .assets
            .iter()
            .find(|a| a.id == self.object_id)
            .ok_or(CommandError::NoActiveAsset)?;
        let source = asset
            .geometry_source
            .as_ref()
            .ok_or_else(|| CommandError::Execution("Object is already editable".into()))?;
        let mut evaluated = asset.clone();
        evaluated.generated_mesh = source
            .evaluate(project)
            .map_err(|e| CommandError::Execution(e.to_string()))?
            .into();
        let mesh = evaluated.evaluated_mesh();
        let asset = state
            .document
            .project
            .assets
            .iter_mut()
            .find(|a| a.id == self.object_id)
            .ok_or(CommandError::NoActiveAsset)?;
        asset.mesh = mesh.into();
        asset.geometry_source = None;
        asset.generated_mesh = petunia_mesh::Mesh::default().into();
        asset.generator_diagnostic = None;
        asset.modifiers.clear();
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CreateRegionGeneratorCmd {
    pub expected_document_revision: u64,
    pub object_id: Uuid,
    pub spline: petunia_project::SplineResource,
    pub shape: petunia_project::PlanarShape,
    pub generator: GeometryGenerator,
    pub source_revision: [u64; 2],
}

impl Command for CreateRegionGeneratorCmd {
    fn label(&self) -> &'static str {
        "extrude planar region"
    }
    fn changes(&self) -> ProjectChanges {
        ProjectChanges::GEOMETRY
            | ProjectChanges::SPLINES
            | ProjectChanges::PROCEDURAL
            | ProjectChanges::SELECTION
    }
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let project = &state.document.project;
        if self.expected_document_revision != project.revision_id() {
            return Err("Region geometry changed; restart the gesture");
        }
        if self.source_revision != [project.spline_revision, project.procedural_revision] {
            return Err("Region source changed; restart the gesture");
        }
        crate::CreateProfileCmd {
            spline: self.spline.clone(),
            profile: self.shape.clone(),
        }
        .can_execute(state)?;
        if project.assets.iter().any(|a| a.id == self.object_id) {
            return Err("Generated object already exists");
        }
        let mut candidate = project.clone();
        candidate
            .add_spline(self.spline.clone())
            .map_err(|_| "Invalid region spline")?;
        candidate
            .add_profile(self.shape.clone())
            .map_err(|_| "Invalid region shape")?;
        candidate
            .promote_planar_contours()
            .map_err(|_| "Invalid region holes")?;
        self.generator
            .evaluate(&candidate)
            .map(|_| ())
            .map_err(|_| "Invalid region extrusion")
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.document.project.add_spline(self.spline.clone())?;
        state.document.project.add_profile(self.shape.clone())?;
        state.document.project.promote_planar_contours()?;
        CreateGeometryGeneratorCmd {
            object_id: self.object_id,
            name: "Region Extrusion".into(),
            generator: self.generator.clone(),
            source_revision: self.source_revision,
        }
        .execute(state)
    }
}

/// Atomic replacement of a planar shape's authoring contours after a gesture.
#[derive(Debug, Clone)]
pub struct EditPlanarShapeCmd {
    pub profile: petunia_project::PlanarShape,
    pub spline: petunia_project::SplineResource,
    pub inner: Vec<petunia_project::SplineResource>,
    pub expected_profile_revision: u64,
    pub expected_spline_revision: u64,
}
impl Command for EditPlanarShapeCmd {
    fn label(&self) -> &'static str {
        "edit planar shape"
    }
    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SPLINES | ProjectChanges::PROCEDURAL
    }
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let project = &state.document.project;
        let before = project
            .get_profile(self.profile.id)
            .ok_or("Shape no longer exists")?;
        if before.locked {
            return Err("Shape is locked");
        }
        let spline = project
            .get_spline(before.spline_id)
            .ok_or("Shape contour no longer exists")?;
        if before.revision != self.expected_profile_revision
            || spline.revision != self.expected_spline_revision
        {
            return Err("Shape source changed; preview discarded");
        }
        if before.spline_id != self.spline.id || before.hole_splines != self.profile.hole_splines {
            return Err("Contour identity cannot change during a gesture");
        }
        if before == &self.profile
            && spline == &self.spline
            && self
                .inner
                .iter()
                .all(|s| project.get_spline(s.id) == Some(s))
        {
            return Err("Shape is unchanged");
        }
        self.profile
            .validate_authoring(&self.spline)
            .map_err(|_| "Invalid shape data")?;
        for contour in &self.inner {
            if !before.hole_splines.contains(&contour.id)
                || project
                    .get_spline(contour.id)
                    .is_none_or(|s| s.revision != contour.revision)
            {
                return Err("Inner contour changed; preview discarded");
            }
            if !contour.closed || contour.points.len() < 3 {
                return Err("Inner contours must remain closed");
            }
            let mut contour_shape = self.profile.clone();
            contour_shape.spline_id = contour.id;
            contour_shape
                .validate_authoring(contour)
                .map_err(|_| "Invalid inner contour")?;
        }
        Ok(())
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let project = &mut state.document.project;
        let mut spline = self.spline.clone();
        spline.revision = self.expected_spline_revision.wrapping_add(1);
        let spline_id = spline.id;
        *project
            .get_spline_mut(spline_id)
            .ok_or(CommandError::EmptySelection)? = spline;
        for contour in &self.inner {
            let mut next = contour.clone();
            next.revision = next.revision.wrapping_add(1);
            let contour_id = next.id;
            *project
                .get_spline_mut(contour_id)
                .ok_or(CommandError::EmptySelection)? = next;
        }
        let mut profile = self.profile.clone();
        profile.revision = self.expected_profile_revision.wrapping_add(1);
        let profile_id = profile.id;
        *project
            .get_profile_mut(profile_id)
            .ok_or(CommandError::EmptySelection)? = profile;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SetPlanarShapeFlagsCmd {
    pub shape: Uuid,
    pub expected_revision: u64,
    pub visible: bool,
    pub locked: bool,
}
impl Command for SetPlanarShapeFlagsCmd {
    fn label(&self) -> &'static str {
        "set shape visibility and lock"
    }
    fn changes(&self) -> ProjectChanges {
        ProjectChanges::PROCEDURAL
    }
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let shape = state
            .document
            .project
            .get_profile(self.shape)
            .ok_or("Shape no longer exists")?;
        if shape.revision != self.expected_revision {
            return Err("Shape state changed");
        }
        if shape.visible == self.visible && shape.locked == self.locked {
            return Err("Shape state is unchanged");
        }
        Ok(())
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let shape = state
            .document
            .project
            .get_profile_mut(self.shape)
            .ok_or(CommandError::EmptySelection)?;
        shape.visible = self.visible;
        shape.locked = self.locked;
        shape.revision = shape.revision.wrapping_add(1);
        Ok(())
    }
}
