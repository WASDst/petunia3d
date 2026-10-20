//! Comandos de construção sobre superfícies (H27).
//!
//! Reúne alinhamento de plano de trabalho à face, imprint de perfis com furos na superfície
//! e Push/Pull contextual (com separação entre boundary caps e regiões internas embedadas).
//! Superfícies curvas são tratadas com mensagens informativas e rejeição graciosa sem panics.

use uuid::Uuid;

use crate::command::{Command, CommandContext, CommandError, EditorView};

/// Alinha o plano de trabalho do desenho (ProfileWorkplane) à face de uma malha autoral.
///
/// Rejeita faces inexistentes, degeneradas ou curvas/não-planares com erro informativo
/// em conformidade com o contrato H27 ("superfícies curvas como extensão posterior").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlignWorkplaneToFaceCmd {
    pub target: Option<Uuid>,
    pub face_index: usize,
}

impl AlignWorkplaneToFaceCmd {
    pub const fn new(face_index: usize) -> Self {
        Self {
            target: None,
            face_index,
        }
    }

    pub const fn for_asset(target: Uuid, face_index: usize) -> Self {
        Self {
            target: Some(target),
            face_index,
        }
    }
}

impl Command for AlignWorkplaneToFaceCmd {
    fn label(&self) -> &'static str {
        "align workplane to face"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let asset = match self.target {
            Some(id) => state.document.assets.iter().find(|a| a.id == id),
            None => state.document.active(),
        };
        let Some(asset) = asset else {
            return Err("Target asset not found");
        };
        let Some(face) = asset.mesh.faces.get(self.face_index) else {
            return Err("Face index out of bounds");
        };
        if face.verts.len() < 3 {
            return Err("Face has fewer than 3 vertices");
        }
        let normal = asset.mesh.face_normal(self.face_index);
        if !normal.is_finite() || normal.length_squared() < 1e-6 {
            return Err("Face has degenerate or non-finite normal");
        }
        let first = asset.mesh.verts[face.verts[0] as usize].vec();
        let n = normal.normalize();
        let non_planar = face.verts.iter().any(|&vi| {
            let p = asset.mesh.verts[vi as usize].vec();
            (p - first).dot(n).abs() > 1e-4
        });
        if non_planar {
            return Err(
                "Face is curved or non-planar; curved surface construction is deferred to a subsequent extension",
            );
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.align_workplane_to_face(self.target, self.face_index)?;
        Ok(())
    }
}

/// Alinha o plano de trabalho do desenho às faces selecionadas da malha ativa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AlignWorkplaneToSelectionCmd;

impl Command for AlignWorkplaneToSelectionCmd {
    fn label(&self) -> &'static str {
        "align workplane to selection"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let Some(asset) = state.document.active() else {
            return Err("No active asset");
        };
        let count = asset.mesh.faces.iter().filter(|f| f.selected).count();
        if count == 0 {
            return Err("No face selected for workplane alignment");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.align_workplane_to_selection()?;
        Ok(())
    }
}

/// Grava um perfil (contorno e furos opcionais) sobre a superfície coplanar da malha (Imprint).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImprintProfileCmd {
    pub profile_id: Uuid,
    pub target: Option<Uuid>,
}

impl ImprintProfileCmd {
    pub const fn new(profile_id: Uuid) -> Self {
        Self {
            profile_id,
            target: None,
        }
    }

    pub const fn for_asset(profile_id: Uuid, target: Uuid) -> Self {
        Self {
            profile_id,
            target: Some(target),
        }
    }
}

impl Command for ImprintProfileCmd {
    fn label(&self) -> &'static str {
        "imprint profile on surface"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let asset = match self.target {
            Some(id) => state.document.assets.iter().find(|a| a.id == id),
            None => state.document.active(),
        };
        let Some(asset) = asset else {
            return Err("Target asset not found");
        };
        if asset.locked {
            return Err("Target asset is locked");
        }
        let Some(profile) = state.document.project.get_profile(self.profile_id) else {
            return Err("Referenced profile not found in project");
        };
        let Some(spline) = state.document.project.get_spline(profile.spline_id) else {
            return Err("Referenced spline not found for profile");
        };
        if spline.points.len() < 3 {
            return Err("Profile spline has fewer than 3 points");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.imprint_profile_on_surface(self.profile_id, self.target)?;
        Ok(())
    }
}

/// Executa Push/Pull contextual:
/// - Se as faces selecionadas formam um contorno/cap externo do sólido, move a superfície
///   ao longo da normal esticando paredes vizinhas sem criar anéis supérfluos;
/// - Se as faces selecionadas estão contidas/imprinted dentro de faces coplanares, cria
///   extrusão com paredes laterais (rebaixo ou ressalto).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContextualPushPullCmd {
    pub target: Option<Uuid>,
    pub distance: f32,
}

impl ContextualPushPullCmd {
    pub const fn new(distance: f32) -> Self {
        Self {
            target: None,
            distance,
        }
    }

    pub const fn for_asset(target: Uuid, distance: f32) -> Self {
        Self {
            target: Some(target),
            distance,
        }
    }
}

impl Command for ContextualPushPullCmd {
    fn label(&self) -> &'static str {
        "contextual push/pull"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if !self.distance.is_finite() || self.distance.abs() < 1e-6 {
            return Err("Push/pull distance must be non-zero and finite");
        }
        let asset = match self.target {
            Some(id) => state.document.assets.iter().find(|a| a.id == id),
            None => state.document.active(),
        };
        let Some(asset) = asset else {
            return Err("Target asset not found");
        };
        if asset.locked {
            return Err("Target asset is locked");
        }
        let sel_count = asset.mesh.faces.iter().filter(|f| f.selected).count();
        if sel_count == 0 {
            return Err("No face selected for push/pull");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.contextual_push_pull(self.target, self.distance)?;
        Ok(())
    }
}
