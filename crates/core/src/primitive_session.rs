//! Sessão de criação de primitivas (Wave 8 — §10, P3D-153/154).
//!
//! Uma inserção vira transação única de undo com cartão contextual de
//! parâmetros ("Last Operation"): os parâmetros regeneram a malha a partir do
//! descritor (nunca deformam o resultado anterior), `Confirm` encerra mantendo
//! um único checkpoint e `Cancel` (Esc) desfaz até o ponto de inserção.
//!
//! A sessão invalida sozinha quando outra operação assume o topo do undo
//! (qualquer edição topológica converte a primitiva em malha comum): a UI
//! esconde o cartão e o Esc posterior não remove nada.
use uuid::Uuid;

use super::selection::Selection;
use crate::command::PrimitiveKind;

pub use petunia_mesh::{CircleFill, PrimitiveDescriptor};

/// Extensão com métodos de alto nível para `PrimitiveDescriptor` (conversão com `PrimitiveKind` e `TextId`).
pub trait PrimitiveDescriptorExt {
    fn default_for(kind: PrimitiveKind) -> Self;
    fn kind(&self) -> PrimitiveKind;
    fn name_key(&self) -> petunia_config::TextId;
}

impl PrimitiveDescriptorExt for PrimitiveDescriptor {
    fn default_for(kind: PrimitiveKind) -> Self {
        match kind {
            PrimitiveKind::Cube => Self::Box {
                width: 1.0,
                height: 1.0,
                depth: 1.0,
            },
            PrimitiveKind::Plane => Self::Plane {
                width: 1.0,
                height: 1.0,
            },
            PrimitiveKind::Wedge => Self::Wedge {
                width: 1.0,
                height: 1.0,
                depth: 1.0,
            },
            PrimitiveKind::Cylinder => Self::Cylinder {
                radius: 1.0,
                height: 2.0,
                sides: 8,
                cap_top: true,
                cap_bottom: true,
            },
            PrimitiveKind::Cone => Self::Cone {
                bottom_radius: 1.0,
                top_radius: 0.0,
                height: 2.0,
                sides: 8,
                cap_bottom: true,
                cap_top: false,
            },
            PrimitiveKind::Circle => Self::Circle {
                radius: 1.0,
                vertices: 12,
                fill: CircleFill::Disc,
            },
            PrimitiveKind::Torus => Self::Torus {
                major_radius: 1.0,
                minor_radius: 0.3,
                major_segments: 12,
                minor_segments: 6,
            },
            PrimitiveKind::Sphere => Self::LowSphere {
                radius: 1.0,
                segments: 12,
                rings: 6,
            },
            PrimitiveKind::Icosphere => Self::Icosphere {
                radius: 1.0,
                subdiv: 1,
            },
            PrimitiveKind::Capsule => Self::Capsule {
                radius: 0.5,
                height: 2.0,
                radial_segments: 8,
                cap_segments: 2,
            },
        }
    }

    fn kind(&self) -> PrimitiveKind {
        match self {
            Self::Box { .. } => PrimitiveKind::Cube,
            Self::Plane { .. } => PrimitiveKind::Plane,
            Self::Wedge { .. } => PrimitiveKind::Wedge,
            Self::Cylinder { .. } => PrimitiveKind::Cylinder,
            Self::Cone { .. } => PrimitiveKind::Cone,
            Self::Circle { .. } => PrimitiveKind::Circle,
            Self::Torus { .. } => PrimitiveKind::Torus,
            Self::LowSphere { .. } => PrimitiveKind::Sphere,
            Self::Icosphere { .. } => PrimitiveKind::Icosphere,
            Self::Capsule { .. } => PrimitiveKind::Capsule,
        }
    }

    fn name_key(&self) -> petunia_config::TextId {
        use petunia_config::text_id as T;
        match self {
            Self::Box { .. } => T::PRIMS_CUBE,
            Self::Plane { .. } => T::PRIMS_PLANE,
            Self::Wedge { .. } => T::PRIMS_WEDGE,
            Self::Cylinder { .. } => T::PRIMS_CYLINDER,
            Self::Cone { .. } => T::PRIMS_CONE,
            Self::Circle { .. } => T::PRIMS_CIRCLE,
            Self::Torus { .. } => T::PRIMS_TORUS,
            Self::LowSphere { .. } => T::PRIMS_SPHERE,
            Self::Icosphere { .. } => T::PRIMS_ICOSPHERE,
            Self::Capsule { .. } => T::PRIMS_CAPSULE,
        }
    }
}

/// Build only bounded, finite authoring data. The descriptor builders retain
/// their existing numeric clamps; non-finite inputs must never be persisted.
pub(crate) fn build_primitive(
    descriptor: PrimitiveDescriptor,
) -> Result<petunia_mesh::Mesh, &'static str> {
    use PrimitiveDescriptor::*;
    let finite = match descriptor {
        Box {
            width,
            height,
            depth,
        }
        | Wedge {
            width,
            height,
            depth,
        } => [width, height, depth].into_iter().all(f32::is_finite),
        Plane { width, height } => width.is_finite() && height.is_finite(),
        Cylinder { radius, height, .. } | Capsule { radius, height, .. } => {
            radius.is_finite() && height.is_finite()
        }
        Cone {
            bottom_radius,
            top_radius,
            height,
            ..
        } => [bottom_radius, top_radius, height]
            .into_iter()
            .all(f32::is_finite),
        Torus {
            major_radius,
            minor_radius,
            ..
        } => major_radius.is_finite() && minor_radius.is_finite(),
        Circle { radius, .. } | LowSphere { radius, .. } | Icosphere { radius, .. } => {
            radius.is_finite()
        }
    };
    if !finite {
        return Err("Primitive parameters must be finite");
    }
    let mesh = descriptor.build();
    if mesh.verts.is_empty() || mesh.validate_invariants().is_err() {
        return Err("Invalid primitive geometry");
    }
    Ok(mesh)
}

/// Prepare before checkpoint/publication. New primitives regenerate in their
/// object frame. Legacy translation-only sources keep their old representation;
/// an untracked baked rotation/scale cannot be inferred from accumulated Euler
/// metadata, so refuse rather than silently erase it.
pub(crate) fn regenerate_primitive(
    asset: &petunia_project::Asset,
    descriptor: PrimitiveDescriptor,
) -> Result<petunia_mesh::Mesh, &'static str> {
    let source = asset.parametric.ok_or("Object is no longer parametric")?;
    if asset.locked {
        return Err("Primitive is locked");
    }
    if asset.skeleton_id.is_some() || asset.skin_data.is_some() {
        return Err("Rigged primitive regeneration requires bind compensation");
    }
    let mut mesh = build_primitive(descriptor)?;
    let legacy = asset.has_legacy_baked_transform();
    let offset = if legacy {
        asset
            .origin
            .map(glam::Vec3::from)
            .unwrap_or_else(|| asset.mesh.center())
    } else {
        glam::Vec3::ZERO
    };
    let original = build_primitive(source)?;
    if !offset.is_finite()
        || original.verts.len() != asset.mesh.verts.len()
        || original.faces.len() != asset.mesh.faces.len()
        || original
            .verts
            .iter()
            .zip(&asset.mesh.verts)
            .any(|(a, b)| !b.vec().is_finite() || (a.vec() + offset).distance(b.vec()) > 1.0e-4)
        || original
            .faces
            .iter()
            .zip(&asset.mesh.faces)
            .any(|(a, b)| a.verts != b.verts)
    {
        return Err(
            "Primitive has baked or free geometry edits; Make Editable preserves its surface",
        );
    }
    if legacy {
        for vertex in &mut mesh.verts {
            vertex.pos = (vertex.vec() + offset).to_array();
        }
    }
    let matrix = asset.world_matrix();
    if !asset.transform.is_finite()
        || !matrix.is_finite()
        || !matrix.inverse().is_finite()
        || mesh
            .verts
            .iter()
            .any(|v| !v.vec().is_finite() || !matrix.transform_point3(v.vec()).is_finite())
    {
        return Err("Primitive object frame must be finite and invertible");
    }
    Ok(mesh)
}

/// Criação em andamento: alvo + parâmetros + contexto de reversão.
#[derive(Debug, Clone)]
pub struct PrimitiveCreationSession {
    /// Asset criado no `begin` (id estável; índice pode mudar).
    pub asset_id: Uuid,
    /// Parâmetros vivos (cada edição regenera a malha).
    pub descriptor: PrimitiveDescriptor,
    /// Rótulo do checkpoint de inserção (validade = ainda no topo do undo).
    pub undo_label: String,
    /// Seleção da sessão antes do `begin` (restauração do `cancel`).
    pub original_selection: Selection,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::PrimitiveKind;
    use crate::state::AppState;

    #[test]
    fn descriptor_builds_valid_meshes() {
        for kind in [
            PrimitiveKind::Cube,
            PrimitiveKind::Sphere,
            PrimitiveKind::Cylinder,
            PrimitiveKind::Plane,
            PrimitiveKind::Cone,
            PrimitiveKind::Capsule,
        ] {
            let mesh = PrimitiveDescriptor::default_for(kind).build();
            assert!(!mesh.verts.is_empty(), "{kind:?}");
            assert!(!mesh.faces.is_empty(), "{kind:?}");
            // Dados internos de authoring passam por validação estrita, nunca auto-repair.
            mesh.validate_invariants()
                .unwrap_or_else(|e| panic!("{kind:?} Inválida: {e}"));
            assert!(!mesh.faces.is_empty(), "{kind:?} após validação estrita");
        }
    }

    #[test]
    fn descriptor_roundtrips_kind() {
        for kind in [
            PrimitiveKind::Cube,
            PrimitiveKind::Sphere,
            PrimitiveKind::Cylinder,
            PrimitiveKind::Plane,
            PrimitiveKind::Cone,
            PrimitiveKind::Capsule,
            PrimitiveKind::Wedge,
            PrimitiveKind::Circle,
            PrimitiveKind::Torus,
            PrimitiveKind::Icosphere,
        ] {
            assert_eq!(PrimitiveDescriptor::default_for(kind).kind(), kind);
        }
    }

    #[test]
    fn confirm_keeps_single_checkpoint() {
        let mut state = AppState::new("en");
        let depth_before = state.project.undo.depth().0;
        assert!(state.begin_primitive(PrimitiveKind::Cube, Some("Box".to_string())));
        assert!(state.primitive_session_valid());
        let asset_id = state.session.primitive_session.as_ref().unwrap().asset_id;
        // Regenerações não empilham undo.
        assert!(state.update_primitive(PrimitiveDescriptor::Box {
            width: 4.0,
            height: 4.0,
            depth: 4.0,
        }));
        assert!(state.update_primitive(PrimitiveDescriptor::Box {
            width: 5.0,
            height: 5.0,
            depth: 5.0,
        }));
        assert_eq!(state.project.undo.depth().0, depth_before + 1);
        assert!(state.confirm_primitive());
        assert!(!state.primitive_session_valid());
        assert!(state.project.assets.iter().any(|a| a.id == asset_id));
        // Um único undo remove a criação inteira.
        assert!(state.undo());
        assert!(!state.project.assets.iter().any(|a| a.id == asset_id));
    }

    #[test]
    fn cancel_removes_asset_and_restores_selection() {
        let mut state = AppState::new("en");
        let before = state.project.assets.len();
        // Marca uma seleção prévia para verificar restauração.
        if let Some(first) = state.project.assets.first() {
            let id = first.id;
            state.session.selection.asset = Some(id);
        }
        assert!(state.begin_primitive(PrimitiveKind::Sphere, None));
        assert_eq!(state.project.assets.len(), before + 1);
        assert!(state.cancel_primitive());
        assert_eq!(state.project.assets.len(), before);
        assert!(!state.primitive_session_valid());
    }

    #[test]
    fn intervening_mesh_edit_requires_explicit_conversion() {
        use crate::command::SubdivideSelectionCmd;
        let mut state = AppState::new("en");
        assert!(state.begin_primitive(PrimitiveKind::Cube, None));
        // Generated geometry cannot be mutated or implicitly baked.
        let depth = state.project.undo.depth();
        assert!(state.project.active_mesh_mut().is_none());
        assert!(state.dispatch(&SubdivideSelectionCmd).is_err());
        assert!(state.project.active().unwrap().geometry_source.is_some());
        assert!(state.primitive_session_valid());
        assert_eq!(state.project.undo.depth(), depth);
        // Explicit conversion starts its own reversible operation.
        assert!(state.freeze_active_primitive());
        state.project.active_mesh_mut().unwrap().faces[0].selected = true;
        assert!(state.dispatch(&SubdivideSelectionCmd).is_ok());
        assert!(!state.primitive_session_valid());
        // Cancel posterior não remove nada.
        let count = state.project.assets.len();
        assert!(!state.cancel_primitive());
        assert_eq!(state.project.assets.len(), count);
    }

    #[test]
    fn reopen_creates_new_session_from_last() {
        let mut state = AppState::new("en");
        assert!(state.begin_primitive(PrimitiveKind::Cylinder, Some("Tube".to_string())));
        assert!(state.update_primitive(PrimitiveDescriptor::Cylinder {
            radius: 2.0,
            height: 3.0,
            sides: 8,
            cap_top: true,
            cap_bottom: false,
        }));
        assert!(state.confirm_primitive());
        let before = state.project.assets.len();
        assert!(state.reopen_last_primitive());
        assert!(state.primitive_session_valid());
        assert_eq!(state.project.assets.len(), before + 1);
        let desc = state.session.primitive_session.as_ref().unwrap().descriptor;
        assert_eq!(
            desc,
            PrimitiveDescriptor::Cylinder {
                radius: 2.0,
                height: 3.0,
                sides: 8,
                cap_top: true,
                cap_bottom: false,
            }
        );
    }

    #[test]
    fn reset_restores_defaults() {
        let mut state = AppState::new("en");
        assert!(state.begin_primitive(PrimitiveKind::Torus, None));
        assert!(state.update_primitive(PrimitiveDescriptor::Torus {
            major_radius: 5.0,
            minor_radius: 2.0,
            major_segments: 32,
            minor_segments: 16,
        }));
        // Reset = regenerar a partir do default da espécie.
        let kind = state
            .session
            .primitive_session
            .as_ref()
            .unwrap()
            .descriptor
            .kind();
        assert!(state.update_primitive(PrimitiveDescriptor::default_for(kind)));
        let id = state.session.primitive_session.as_ref().unwrap().asset_id;
        let asset = state.project.assets.iter().find(|a| a.id == id).unwrap();
        let fresh = PrimitiveDescriptor::default_for(kind).build();
        assert_eq!(asset.source_mesh().verts.len(), fresh.verts.len());
        assert!(state.confirm_primitive());
    }

    #[test]
    fn workspace_switch_finalizes_session_keeping_mesh() {
        let mut state = AppState::new("en");
        assert!(state.begin_primitive(PrimitiveKind::Torus, None));
        let count = state.project.assets.len();
        state.switch_workspace(crate::selection::Workspace::Uv);
        assert!(state.session.primitive_session.is_none());
        assert_eq!(state.project.assets.len(), count);
        // Virou malha comum: reabertura explícita ainda disponível.
        assert!(state.session.last_primitive.is_some());
    }

    #[test]
    fn all_ten_kinds_begin_update_confirm() {
        use PrimitiveKind as K;
        for kind in [
            K::Cube,
            K::Plane,
            K::Wedge,
            K::Cylinder,
            K::Cone,
            K::Circle,
            K::Torus,
            K::Sphere,
            K::Icosphere,
            K::Capsule,
        ] {
            let mut state = AppState::new("en");
            assert!(state.begin_primitive(kind, None), "{kind:?} begin");
            assert!(state.primitive_session_valid(), "{kind:?} valid");
            let depth = state.project.undo.depth().0;
            let desc = PrimitiveDescriptor::default_for(kind);
            assert!(state.update_primitive(desc), "{kind:?} update");
            assert_eq!(state.project.undo.depth().0, depth);
            assert!(state.confirm_primitive(), "{kind:?} confirm");
            assert_eq!(state.session.last_primitive, Some(desc));
        }
    }

    #[test]
    fn plane_scales_to_rectangle() {
        let mesh = PrimitiveDescriptor::Plane {
            width: 4.0,
            height: 2.0,
        }
        .build();
        let (mut min_x, mut max_x) = (f32::MAX, f32::MIN);
        let (mut min_z, mut max_z) = (f32::MAX, f32::MIN);
        for v in &mesh.verts {
            min_x = min_x.min(v.pos[0]);
            max_x = max_x.max(v.pos[0]);
            min_z = min_z.min(v.pos[2]);
            max_z = max_z.max(v.pos[2]);
        }
        assert!((max_x - min_x - 4.0).abs() < 1e-4);
        assert!((max_z - min_z - 2.0).abs() < 1e-4);
    }
}
