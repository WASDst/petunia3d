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

    /// Cria um novo material no projeto e o seleciona como ativo.
    pub fn create_material(&mut self) -> bool {
        let count = self.state.project.project.materials.len();
        let name = format!("Material {}", count + 1);
        self.state.checkpoint("create material");
        self.state
            .project
            .project
            .materials
            .push(petunia_project::Material::new(name));
        self.active_material_slot = count as i32;
        self.state.set_status(crate::tr::fill(
            &self
                .state
                .t_id(petunia_config::text_id::STATUS_CREATED_MATERIAL),
            &[("0", format!("{}", count + 1))],
        ));
        self.state.emit_material_changed();
        true
    }

    /// Duplica o material do slot indicado e o seleciona como ativo.
    pub fn duplicate_material(&mut self, slot: usize) -> bool {
        let Some(mat) = self.state.project.project.materials.get(slot).cloned() else {
            return false;
        };
        let mut dup = mat;
        dup.id = uuid::Uuid::new_v4();
        dup.name = format!("{} Copy", dup.name);
        self.state.checkpoint("duplicate material");
        self.state.project.project.materials.push(dup);
        let new_idx = self.state.project.project.materials.len() - 1;
        self.active_material_slot = new_idx as i32;
        self.state.set_status(crate::tr::fill(
            &self
                .state
                .t_id(petunia_config::text_id::STATUS_DUPLICATED_MATERIAL_TO_SLOT),
            &[("new_idx", format!("{new_idx}"))],
        ));
        self.state.emit_material_changed();
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
        let Ok(slot) = usize::try_from(slot) else {
            return false;
        };
        if self.state.project.project.materials.len() <= 1
            || !self.state.project.project.materials.get(slot).is_some()
        {
            return false;
        }
        let material_id = self.state.project.project.materials[slot].id;
        self.state.checkpoint("remove material");
        self.state.project.project.remove_material(material_id);
        self.active_material_slot = (slot as i32 - 1).max(0);
        self.state.emit_material_changed();
        true
    }

    pub fn set_active_material_base_color(&mut self, red: f32, green: f32, blue: f32) -> bool {
        if [red, green, blue].iter().any(|value| !value.is_finite()) {
            return false;
        }
        let slot = self.active_material_slot.max(0) as usize;
        if !self.state.project.project.materials.get(slot).is_some() {
            return false;
        }
        self.state.checkpoint("change material color");
        let Some(material) = self.state.project.project.materials.get_mut(slot) else {
            return false;
        };
        material.base_color[0] = red.clamp(0.0, 1.0);
        material.base_color[1] = green.clamp(0.0, 1.0);
        material.base_color[2] = blue.clamp(0.0, 1.0);
        material.base_color[3] = 1.0;
        self.state.emit_material_changed();
        true
    }

    pub fn set_active_material_scalar(&mut self, field: &str, value: f32) -> bool {
        if !value.is_finite() {
            return false;
        }
        let slot = self.active_material_slot.max(0) as usize;
        if !matches!(
            field,
            "roughness" | "metallic" | "normal-scale" | "emission-strength" | "alpha-cutoff"
        ) || !self.state.project.project.materials.get(slot).is_some()
        {
            return false;
        }
        self.state.checkpoint("change material");
        let Some(material) = self.state.project.project.materials.get_mut(slot) else {
            return false;
        };
        match field {
            "roughness" => material.roughness = value.clamp(0.0, 1.0),
            "metallic" => material.metallic = value.clamp(0.0, 1.0),
            "normal-scale" => material.normal_scale = value.clamp(0.0, 10.0),
            "emission-strength" => material.emission_strength = value.clamp(0.0, 10.0),
            "alpha-cutoff" => material.alpha_cutoff = value.clamp(0.0, 1.0),
            _ => return false,
        }
        material.validate();
        self.state.emit_material_changed();
        true
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
        let slot = self.active_material_slot.max(0) as usize;
        if !self.state.project.project.materials.get(slot).is_some() {
            return false;
        }
        self.state.checkpoint("change material profile");
        let Some(material) = self.state.project.project.materials.get_mut(slot) else {
            return false;
        };
        material.profile = profile;
        self.state.emit_material_changed();
        true
    }

    pub fn set_active_material_alpha_mode(&mut self, mode: i32) -> bool {
        let mode = match mode {
            0 => AlphaMode::Opaque,
            1 => AlphaMode::Mask,
            2 => AlphaMode::Blend,
            _ => return false,
        };
        let slot = self.active_material_slot.max(0) as usize;
        if !self.state.project.project.materials.get(slot).is_some() {
            return false;
        }
        self.state.checkpoint("change material alpha");
        let Some(material) = self.state.project.project.materials.get_mut(slot) else {
            return false;
        };
        material.alpha_mode = mode;
        self.state.emit_material_changed();
        true
    }

    pub fn create_albedo_texture(&mut self) -> bool {
        let slot = self.active_material_slot.max(0) as usize;
        let Some(material) = self.state.project.project.materials.get(slot) else {
            return false;
        };
        let color = material.base_color;
        self.state.checkpoint("create albedo texture");
        let Some(material) = self.state.project.project.materials.get_mut(slot) else {
            return false;
        };
        material.albedo_texture = Some(petunia_project::Canvas::new(
            256,
            256,
            [
                (color[0].clamp(0.0, 1.0) * 255.0) as u8,
                (color[1].clamp(0.0, 1.0) * 255.0) as u8,
                (color[2].clamp(0.0, 1.0) * 255.0) as u8,
                255,
            ],
        ));
        self.state.emit_project_changed(
            petunia_project::ProjectChanges::MATERIALS | petunia_project::ProjectChanges::TEXTURES,
        );
        true
    }

    pub fn clear_albedo_texture(&mut self) -> bool {
        let slot = self.active_material_slot.max(0) as usize;
        if !self
            .state
            .project
            .project
            .materials
            .get(slot)
            .is_some_and(|material| material.albedo_texture.is_some())
        {
            return false;
        }
        self.state.checkpoint("clear albedo texture");
        let Some(material) = self.state.project.project.materials.get_mut(slot) else {
            return false;
        };
        material.albedo_texture = None;
        self.state.emit_project_changed(
            petunia_project::ProjectChanges::MATERIALS | petunia_project::ProjectChanges::TEXTURES,
        );
        true
    }
}
