//! Materiais do projeto por comando: criar, duplicar, remover e editar.
//!
//! A UI emite a intenção; o dispatcher valida, grava uma entrada de Undo e
//! emite `ProjectChanges::MATERIALS` (website/docs/02-application/
//! commands-events-services.md). Materiais são endereçados por id: um alvo
//! que deixou de existir falha em vez de editar outro material.

use petunia_project::{AlphaMode, Canvas, Material, ProjectChanges, ShaderProfile};
use uuid::Uuid;

use crate::command::{Command, CommandError};
use crate::context::{CommandContext, EditorView};

fn material_index(document: &petunia_project::Project, id: Uuid) -> Option<usize> {
    document
        .materials
        .iter()
        .position(|material| material.id == id)
}

/// Novo material com `name`, ao fim da lista.
#[derive(Clone, Debug, PartialEq)]
pub struct CreateMaterialCmd {
    pub name: String,
}

impl Default for CreateMaterialCmd {
    fn default() -> Self {
        Self {
            name: "Material".to_string(),
        }
    }
}

impl Command for CreateMaterialCmd {
    fn label(&self) -> &'static str {
        "create material"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::MATERIALS
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state
            .document
            .materials
            .push(Material::new(self.name.clone()));
        Ok(())
    }
}

/// Cópia do material `material` (id novo, nome "<nome> Copy"), ao fim da lista.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DuplicateMaterialCmd {
    pub material: Uuid,
}

impl Command for DuplicateMaterialCmd {
    fn label(&self) -> &'static str {
        "duplicate material"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::MATERIALS
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        material_index(state.document, self.material)
            .map(|_| ())
            .ok_or("material not found")
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let index = material_index(state.document, self.material)
            .ok_or_else(|| CommandError::StaleReference(self.material.to_string()))?;
        let mut copy = state.document.materials[index].clone();
        copy.id = Uuid::new_v4();
        copy.name = format!("{} Copy", copy.name);
        state.document.materials.push(copy);
        Ok(())
    }
}

/// Remove o material `material`; o projeto sempre mantém ao menos um.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RemoveMaterialCmd {
    pub material: Uuid,
}

impl Command for RemoveMaterialCmd {
    fn label(&self) -> &'static str {
        "remove material"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::MATERIALS
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.document.materials.len() <= 1 {
            return Err("the project keeps at least one material");
        }
        material_index(state.document, self.material)
            .map(|_| ())
            .ok_or("material not found")
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        material_index(state.document, self.material)
            .ok_or_else(|| CommandError::StaleReference(self.material.to_string()))?;
        state.document.remove_material(self.material);
        Ok(())
    }
}

/// Uma edição de propriedade de material. Escalares são limitados à faixa
/// válida, como no Inspector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MaterialEdit {
    /// RGB `0..1`; o alfa da cor base fica em 1.
    BaseColor([f32; 3]),
    Roughness(f32),
    Metallic(f32),
    NormalScale(f32),
    EmissionStrength(f32),
    AlphaCutoff(f32),
    Profile(ShaderProfile),
    AlphaMode(AlphaMode),
    /// Textura albedo 256 × 256 preenchida com a cor base atual.
    CreateAlbedo,
    ClearAlbedo,
}

impl MaterialEdit {
    fn label(self) -> &'static str {
        match self {
            Self::BaseColor(_) => "change material color",
            Self::Profile(_) => "change material profile",
            Self::AlphaMode(_) => "change material alpha",
            Self::CreateAlbedo => "create albedo texture",
            Self::ClearAlbedo => "clear albedo texture",
            _ => "change material",
        }
    }

    fn is_finite(self) -> bool {
        match self {
            Self::BaseColor(rgb) => rgb.iter().all(|value| value.is_finite()),
            Self::Roughness(value)
            | Self::Metallic(value)
            | Self::NormalScale(value)
            | Self::EmissionStrength(value)
            | Self::AlphaCutoff(value) => value.is_finite(),
            _ => true,
        }
    }
}

/// Aplica `edit` ao material `material`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EditMaterialCmd {
    pub material: Uuid,
    pub edit: MaterialEdit,
}

impl Default for EditMaterialCmd {
    fn default() -> Self {
        Self {
            material: Uuid::nil(),
            edit: MaterialEdit::Roughness(0.5),
        }
    }
}

impl Command for EditMaterialCmd {
    fn label(&self) -> &'static str {
        self.edit.label()
    }

    fn changes(&self) -> ProjectChanges {
        match self.edit {
            MaterialEdit::CreateAlbedo | MaterialEdit::ClearAlbedo => {
                ProjectChanges::MATERIALS | ProjectChanges::TEXTURES
            }
            _ => ProjectChanges::MATERIALS,
        }
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if !self.edit.is_finite() {
            return Err("material value must be finite");
        }
        let index = material_index(state.document, self.material).ok_or("material not found")?;
        if self.edit == MaterialEdit::ClearAlbedo
            && state.document.materials[index].albedo_texture.is_none()
        {
            return Err("material has no albedo texture");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let index = material_index(state.document, self.material)
            .ok_or_else(|| CommandError::StaleReference(self.material.to_string()))?;
        let material = &mut state.document.materials[index];
        match self.edit {
            MaterialEdit::BaseColor([red, green, blue]) => {
                material.base_color = [
                    red.clamp(0.0, 1.0),
                    green.clamp(0.0, 1.0),
                    blue.clamp(0.0, 1.0),
                    1.0,
                ];
            }
            MaterialEdit::Roughness(value) => material.roughness = value.clamp(0.0, 1.0),
            MaterialEdit::Metallic(value) => material.metallic = value.clamp(0.0, 1.0),
            MaterialEdit::NormalScale(value) => material.normal_scale = value.clamp(0.0, 10.0),
            MaterialEdit::EmissionStrength(value) => {
                material.emission_strength = value.clamp(0.0, 10.0);
            }
            MaterialEdit::AlphaCutoff(value) => material.alpha_cutoff = value.clamp(0.0, 1.0),
            MaterialEdit::Profile(profile) => material.profile = profile,
            MaterialEdit::AlphaMode(mode) => material.alpha_mode = mode,
            MaterialEdit::CreateAlbedo => {
                let [red, green, blue, _] = material.base_color;
                let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0) as u8;
                material.albedo_texture = Some(Canvas::new(
                    256,
                    256,
                    [byte(red), byte(green), byte(blue), 255],
                ));
            }
            MaterialEdit::ClearAlbedo => material.albedo_texture = None,
        }
        material.validate();
        Ok(())
    }
}

/// Altera múltiplas propriedades de um material em uma única transação atômica.
/// Cada campo com `None` preserva o valor atual.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SetMaterialPropertiesCmd {
    pub material: Uuid,
    pub base_color: Option<[f32; 3]>,
    pub roughness: Option<f32>,
    pub metallic: Option<f32>,
    pub normal_scale: Option<f32>,
    pub emission_color: Option<[f32; 3]>,
    pub emission_strength: Option<f32>,
    pub alpha_cutoff: Option<f32>,
    pub profile: Option<ShaderProfile>,
    pub alpha_mode: Option<AlphaMode>,
}

impl Command for SetMaterialPropertiesCmd {
    fn label(&self) -> &'static str {
        "set material properties"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::MATERIALS
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if self
            .base_color
            .is_some_and(|rgb| !rgb.iter().all(|v| v.is_finite()))
        {
            return Err("material base color must be finite");
        }
        if self.roughness.is_some_and(|v| !v.is_finite()) {
            return Err("material roughness must be finite");
        }
        if self.metallic.is_some_and(|v| !v.is_finite()) {
            return Err("material metallic must be finite");
        }
        if self.normal_scale.is_some_and(|v| !v.is_finite()) {
            return Err("material normal scale must be finite");
        }
        if self
            .emission_color
            .is_some_and(|rgb| !rgb.iter().all(|v| v.is_finite()))
        {
            return Err("material emission color must be finite");
        }
        if self.emission_strength.is_some_and(|v| !v.is_finite()) {
            return Err("material emission strength must be finite");
        }
        if self.alpha_cutoff.is_some_and(|v| !v.is_finite()) {
            return Err("material alpha cutoff must be finite");
        }
        material_index(state.document, self.material)
            .map(|_| ())
            .ok_or("material not found")
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let index = material_index(state.document, self.material)
            .ok_or_else(|| CommandError::StaleReference(self.material.to_string()))?;
        let material = &mut state.document.materials[index];
        let mut changed = false;

        if let Some([red, green, blue]) = self.base_color {
            let new_color = [
                red.clamp(0.0, 1.0),
                green.clamp(0.0, 1.0),
                blue.clamp(0.0, 1.0),
                1.0,
            ];
            if material.base_color != new_color {
                material.base_color = new_color;
                changed = true;
            }
        }
        if let Some(value) = self.roughness {
            let clamped = value.clamp(0.0, 1.0);
            if (material.roughness - clamped).abs() > f32::EPSILON {
                material.roughness = clamped;
                changed = true;
            }
        }
        if let Some(value) = self.metallic {
            let clamped = value.clamp(0.0, 1.0);
            if (material.metallic - clamped).abs() > f32::EPSILON {
                material.metallic = clamped;
                changed = true;
            }
        }
        if let Some(value) = self.normal_scale {
            let clamped = value.clamp(0.0, 10.0);
            if (material.normal_scale - clamped).abs() > f32::EPSILON {
                material.normal_scale = clamped;
                changed = true;
            }
        }
        if let Some([red, green, blue]) = self.emission_color {
            let new_emission = [
                red.clamp(0.0, 1.0),
                green.clamp(0.0, 1.0),
                blue.clamp(0.0, 1.0),
            ];
            if material.emission_color != new_emission {
                material.emission_color = new_emission;
                changed = true;
            }
        }
        if let Some(value) = self.emission_strength {
            let clamped = value.clamp(0.0, 10.0);
            if (material.emission_strength - clamped).abs() > f32::EPSILON {
                material.emission_strength = clamped;
                changed = true;
            }
        }
        if let Some(value) = self.alpha_cutoff {
            let clamped = value.clamp(0.0, 1.0);
            if (material.alpha_cutoff - clamped).abs() > f32::EPSILON {
                material.alpha_cutoff = clamped;
                changed = true;
            }
        }
        if let Some(profile) = self.profile
            && material.profile != profile
        {
            material.profile = profile;
            changed = true;
        }
        if let Some(alpha_mode) = self.alpha_mode
            && material.alpha_mode != alpha_mode
        {
            material.alpha_mode = alpha_mode;
            changed = true;
        }

        if !changed {
            return Err(CommandError::NoChange(
                "no material properties modified".into(),
            ));
        }

        material.validate();
        Ok(())
    }
}
