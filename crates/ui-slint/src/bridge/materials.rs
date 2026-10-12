//! Materiais, slots e textura albedo.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Atribui o material ao objeto e às faces selecionadas, quando existirem,
    /// por `AssignMaterialSlotCmd` (uma entrada de Undo).
    pub fn assign_material_slot(&mut self, slot: usize) -> bool {
        if self.state.project.active == usize::MAX
            || self.state.project.project.materials.get(slot).is_none()
        {
            return false;
        }
        let selected_faces = self.state.project.active_mesh().map_or(0, |mesh| {
            mesh.faces.iter().filter(|face| face.selected).count()
        });
        let command = petunia_core::AssignMaterialSlotCmd {
            material_index: slot,
            faces: None,
        };
        if self.state.dispatch(&command).is_err() {
            return false;
        }
        let message = if selected_faces == 0 {
            format!("Assigned material slot {slot} to object")
        } else {
            format!("Assigned material slot {slot} to object and {selected_faces} face(s)")
        };
        self.state.set_status(message);
        true
    }

    /// Despacha um comando de material; erro vira status (nada muda).
    fn dispatch_material(&mut self, command: &dyn petunia_core::Command) -> bool {
        match self.state.dispatch(command) {
            Ok(()) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    /// Id do material no slot ativo do Inspector.
    fn active_material_id(&self) -> Option<uuid::Uuid> {
        let slot = usize::try_from(self.active_material_slot.max(0)).ok()?;
        self.state.project.project.materials.get(slot).map(|m| m.id)
    }

    /// Edita o material do slot ativo por `EditMaterialCmd`.
    fn edit_active_material(&mut self, edit: petunia_core::MaterialEdit) -> bool {
        let Some(material) = self.active_material_id() else {
            return false;
        };
        self.dispatch_material(&petunia_core::EditMaterialCmd { material, edit })
    }

    /// Cria um novo material no projeto e o seleciona como ativo.
    pub fn create_material(&mut self) -> bool {
        let count = self.state.project.project.materials.len();
        let name = format!("Material {}", count + 1);
        if !self.dispatch_material(&petunia_core::CreateMaterialCmd { name }) {
            return false;
        }
        self.active_material_slot = count as i32;
        self.state.set_status(crate::tr::fill(
            &self
                .state
                .t_id(petunia_config::text_id::STATUS_CREATED_MATERIAL),
            &[("0", format!("{}", count + 1))],
        ));
        true
    }

    /// Duplica o material do slot indicado e o seleciona como ativo.
    pub fn duplicate_material(&mut self, slot: usize) -> bool {
        let Some(material) = self.state.project.project.materials.get(slot).map(|m| m.id) else {
            return false;
        };
        if !self.dispatch_material(&petunia_core::DuplicateMaterialCmd { material }) {
            return false;
        }
        let new_idx = self.state.project.project.materials.len() - 1;
        self.active_material_slot = new_idx as i32;
        self.state.set_status(crate::tr::fill(
            &self
                .state
                .t_id(petunia_config::text_id::STATUS_DUPLICATED_MATERIAL_TO_SLOT),
            &[("new_idx", format!("{new_idx}"))],
        ));
        true
    }

    pub fn select_material_slot(&mut self, slot: i32) -> bool {
        let Ok(slot) = usize::try_from(slot) else {
            return false;
        };
        if slot >= self.state.project.project.materials.len() {
            return false;
        }
        self.active_material_slot = slot as i32;
        true
    }

    pub fn remove_material(&mut self, slot: i32) -> bool {
        let Ok(index) = usize::try_from(slot) else {
            return false;
        };
        // O comando também recusa remover o último material; aqui a recusa é
        // silenciosa, como antes (o botão fica desabilitado).
        if self.state.project.project.materials.len() <= 1 {
            return false;
        }
        let Some(material) = self
            .state
            .project
            .project
            .materials
            .get(index)
            .map(|m| m.id)
        else {
            return false;
        };
        if !self.dispatch_material(&petunia_core::RemoveMaterialCmd { material }) {
            return false;
        }
        self.active_material_slot = (slot - 1).max(0);
        true
    }

    pub fn set_active_material_base_color(&mut self, red: f32, green: f32, blue: f32) -> bool {
        if [red, green, blue].iter().any(|value| !value.is_finite()) {
            return false;
        }
        self.edit_active_material(petunia_core::MaterialEdit::BaseColor([red, green, blue]))
    }

    pub fn set_active_material_scalar(&mut self, field: &str, value: f32) -> bool {
        use petunia_core::MaterialEdit as Edit;
        if !value.is_finite() {
            return false;
        }
        let edit = match field {
            "roughness" => Edit::Roughness(value),
            "metallic" => Edit::Metallic(value),
            "normal-scale" => Edit::NormalScale(value),
            "emission-strength" => Edit::EmissionStrength(value),
            "alpha-cutoff" => Edit::AlphaCutoff(value),
            _ => return false,
        };
        self.edit_active_material(edit)
    }

    pub fn set_active_material_profile(&mut self, profile: i32) -> bool {
        let profile = match profile {
            0 => ShaderProfile::Pbr,
            1 => ShaderProfile::Unlit,
            2 => ShaderProfile::Toon,
            3 => ShaderProfile::Glass,
            4 => ShaderProfile::Emissive,
            _ => return false,
        };
        self.edit_active_material(petunia_core::MaterialEdit::Profile(profile))
    }

    pub fn set_active_material_alpha_mode(&mut self, mode: i32) -> bool {
        let mode = match mode {
            0 => AlphaMode::Opaque,
            1 => AlphaMode::Mask,
            2 => AlphaMode::Blend,
            _ => return false,
        };
        self.edit_active_material(petunia_core::MaterialEdit::AlphaMode(mode))
    }

    pub fn create_albedo_texture(&mut self) -> bool {
        self.edit_active_material(petunia_core::MaterialEdit::CreateAlbedo)
    }

    pub fn clear_albedo_texture(&mut self) -> bool {
        let has_texture = self
            .active_material_id()
            .and_then(|id| {
                self.state
                    .project
                    .project
                    .materials
                    .iter()
                    .find(|m| m.id == id)
            })
            .is_some_and(|material| material.albedo_texture.is_some());
        if !has_texture {
            return false;
        }
        self.edit_active_material(petunia_core::MaterialEdit::ClearAlbedo)
    }
}
