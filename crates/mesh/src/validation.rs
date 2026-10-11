//! Validação estrita e reparo explícito de malhas (website/docs/03-geometry/validation-repair-cleanup.md).
//!
//! Separar rigorosamente:
//! - Validação estrita sem mutação (`validate_invariants`): falhas indicam bug interno ou corrupção.
//! - Reparo externo (`repair_external_mesh`): higienização com relatório estruturado para dados não confiáveis.

use std::collections::HashSet;

use crate::Mesh;

/// Erros estruturais e de invariantes da malha.
#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum MeshValidationError {
    #[error("Vértice {index} possui coordenada não finita: {pos:?}")]
    NonFiniteVertexPosition { index: usize, pos: [f32; 3] },

    #[error("Vértice {index} possui cor não finita: {color:?}")]
    NonFiniteVertexColor { index: usize, color: [f32; 3] },

    #[error("Face {index} possui menos de 3 vértices: {count}")]
    DegenerateFaceVertexCount { index: usize, count: usize },

    #[error("Face {index} referencia vértice inexistente {vertex_index} (total: {total_verts})")]
    VertexIndexOutOfBounds {
        index: usize,
        vertex_index: u32,
        total_verts: usize,
    },

    #[error("Face {face_index} possui descompasso de UV: {verts_len} vértices vs {uv_len} UVs")]
    UvCountMismatch {
        face_index: usize,
        verts_len: usize,
        uv_len: usize,
    },

    #[error("Face {face_index}, corner {corner_index} possui coordenadas UV não finitas: {uv:?}")]
    NonFiniteUvCoordinate {
        face_index: usize,
        corner_index: usize,
        uv: [f32; 2],
    },

    #[error("Face {index} possui vértices duplicados: {verts:?}")]
    DuplicateFaceVertices { index: usize, verts: Vec<u32> },

    #[error("UV seam referencia vértice inexistente {vertex_index} (total: {total_verts})")]
    InvalidSeamVertex {
        vertex_index: u32,
        total_verts: usize,
    },

    #[error("UV seam é degenerada (vértice a == b == {vertex})")]
    DegenerateSeam { vertex: u32 },

    #[error("UV pin referencia índice de face inexistente {face_index} (total: {total_faces})")]
    InvalidPinFaceIndex {
        face_index: usize,
        total_faces: usize,
    },

    #[error(
        "UV pin referencia corner inexistente {corner_index} na face {face_index} (total: {total_corners})"
    )]
    InvalidPinCornerIndex {
        face_index: usize,
        corner_index: usize,
        total_corners: usize,
    },
}

/// Políticas de reparo na fronteira externa de I/O.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepairPolicy {
    /// Rejeita dados não conformes onde for possível, apenas descartando referências quebradas.
    StrictImport,
    /// Corrige NaNs para [0.0, 0.0, 0.0], gera UV padrão quando ausente e remove faces degeneradas.
    #[default]
    TolerantImport,
    /// Migração de projetos legados: ajusta atributos antigos para formatos novos.
    LegacyProjectMigration,
}

/// Relatório auditável de todas as modificações realizadas durante o reparo externo.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RepairReport {
    pub vertices_sanitized: usize,
    pub colors_sanitized: usize,
    pub faces_dropped: usize,
    pub uv_defaults_generated: usize,
    pub uv_seams_dropped: usize,
    pub uv_pins_dropped: usize,
    pub selected_edges_dropped: usize,
    pub warnings: Vec<String>,
}

impl RepairReport {
    /// Retorna `true` se nenhum reparo ou warning foi emitido.
    pub fn is_clean(&self) -> bool {
        self.vertices_sanitized == 0
            && self.colors_sanitized == 0
            && self.faces_dropped == 0
            && self.uv_defaults_generated == 0
            && self.uv_seams_dropped == 0
            && self.uv_pins_dropped == 0
            && self.selected_edges_dropped == 0
            && self.warnings.is_empty()
    }
}

/// Valida estritamente todos os invariantes da malha sem nenhuma mutação.
///
/// Falhas aqui indicam bugs em comandos internos ou dados inconsistentes de authoring.
pub fn validate_invariants(mesh: &Mesh) -> Result<(), MeshValidationError> {
    let total_verts = mesh.verts.len();

    // 1. Vértices
    for (i, v) in mesh.verts.iter().enumerate() {
        if !v.pos.iter().all(|c| c.is_finite()) {
            return Err(MeshValidationError::NonFiniteVertexPosition {
                index: i,
                pos: v.pos,
            });
        }
        if !v.color.iter().all(|c| c.is_finite()) {
            return Err(MeshValidationError::NonFiniteVertexColor {
                index: i,
                color: v.color,
            });
        }
    }

    // 2. Faces
    for (f_idx, f) in mesh.faces.iter().enumerate() {
        if f.verts.len() < 3 {
            return Err(MeshValidationError::DegenerateFaceVertexCount {
                index: f_idx,
                count: f.verts.len(),
            });
        }

        // UV vs Verts
        if f.uv.len() != f.verts.len() {
            return Err(MeshValidationError::UvCountMismatch {
                face_index: f_idx,
                verts_len: f.verts.len(),
                uv_len: f.uv.len(),
            });
        }

        // UV finitas
        for (corner_idx, uv) in f.uv.iter().enumerate() {
            if !uv.iter().all(|c| c.is_finite()) {
                return Err(MeshValidationError::NonFiniteUvCoordinate {
                    face_index: f_idx,
                    corner_index: corner_idx,
                    uv: *uv,
                });
            }
        }

        // Índices de vértices dentro dos limites
        for &vi in &f.verts {
            if (vi as usize) >= total_verts {
                return Err(MeshValidationError::VertexIndexOutOfBounds {
                    index: f_idx,
                    vertex_index: vi,
                    total_verts,
                });
            }
        }

        // Vértices duplicados dentro da mesma face
        let mut seen = HashSet::with_capacity(f.verts.len());
        for &vi in &f.verts {
            if !seen.insert(vi) {
                return Err(MeshValidationError::DuplicateFaceVertices {
                    index: f_idx,
                    verts: f.verts.clone(),
                });
            }
        }
    }

    // 3. Seams
    for &(a, b) in &mesh.uv_seams {
        if a == b {
            return Err(MeshValidationError::DegenerateSeam { vertex: a });
        }
        if (a as usize) >= total_verts {
            return Err(MeshValidationError::InvalidSeamVertex {
                vertex_index: a,
                total_verts,
            });
        }
        if (b as usize) >= total_verts {
            return Err(MeshValidationError::InvalidSeamVertex {
                vertex_index: b,
                total_verts,
            });
        }
    }

    // 4. Pins
    let total_faces = mesh.faces.len();
    for &(face_idx, corner_idx) in &mesh.uv_pinned {
        if face_idx >= total_faces {
            return Err(MeshValidationError::InvalidPinFaceIndex {
                face_index: face_idx,
                total_faces,
            });
        }
        let corners = mesh.faces[face_idx].verts.len();
        if corner_idx >= corners {
            return Err(MeshValidationError::InvalidPinCornerIndex {
                face_index: face_idx,
                corner_index: corner_idx,
                total_corners: corners,
            });
        }
    }

    Ok(())
}

/// Repara uma malha externa não confiável de acordo com a política especificada,
/// emitindo um relatório completo de todas as alterações feitas.
pub fn repair_external_mesh(mesh: &mut Mesh, policy: RepairPolicy) -> RepairReport {
    let mut report = RepairReport::default();

    // 1. Higienização de posições e cores de vértices
    for (i, v) in mesh.verts.iter_mut().enumerate() {
        if !v.pos.iter().all(|c| c.is_finite()) {
            match policy {
                RepairPolicy::StrictImport => {
                    report.warnings.push(format!(
                        "Vértice {i} possui coordenada não finita: {:?}",
                        v.pos
                    ));
                }
                RepairPolicy::TolerantImport | RepairPolicy::LegacyProjectMigration => {
                    v.pos = [0.0, 0.0, 0.0];
                    report.vertices_sanitized += 1;
                }
            }
        }
        if !v.color.iter().all(|c| c.is_finite()) {
            match policy {
                RepairPolicy::StrictImport => {
                    report
                        .warnings
                        .push(format!("Vértice {i} possui cor não finita: {:?}", v.color));
                }
                RepairPolicy::TolerantImport | RepairPolicy::LegacyProjectMigration => {
                    v.color = [0.75, 0.75, 0.78];
                    report.colors_sanitized += 1;
                }
            }
        }
    }

    let n = mesh.verts.len();

    // 2. Higienização de UVs por face
    for f in &mut mesh.faces {
        if f.uv.len() != f.verts.len() {
            f.uv = vec![[0.0, 0.0]; f.verts.len()];
            report.uv_defaults_generated += 1;
        } else {
            for uv in &mut f.uv {
                if !uv.iter().all(|c| c.is_finite()) {
                    *uv = [0.0, 0.0];
                    report.uv_defaults_generated += 1;
                }
            }
        }
    }

    // 3. Filtragem de faces estruturalmente válidas
    let before_faces = mesh.faces.len();
    mesh.faces.retain(|f| {
        let valid_len = f.verts.len() >= 3;
        let bounds_ok = f.verts.iter().all(|&vi| (vi as usize) < n);
        let no_duplicates = {
            let mut u = f.verts.clone();
            u.sort_unstable();
            u.dedup();
            u.len() >= 3 && u.len() == f.verts.len()
        };
        valid_len && bounds_ok && no_duplicates
    });
    report.faces_dropped = before_faces.saturating_sub(mesh.faces.len());

    // 4. Filtragem de arestas selecionadas
    let before_edges = mesh.selected_edges.len();
    mesh.selected_edges
        .retain(|&(a, b)| (a as usize) < n && (b as usize) < n && a != b);
    report.selected_edges_dropped = before_edges.saturating_sub(mesh.selected_edges.len());

    // 5. Filtragem de UV seams
    let before_seams = mesh.uv_seams.len();
    mesh.uv_seams
        .retain(|&(a, b)| (a as usize) < n && (b as usize) < n && a != b);
    report.uv_seams_dropped = before_seams.saturating_sub(mesh.uv_seams.len());

    // 5b. Arestas Sharp inválidas (vértice inexistente ou self-edge)
    mesh.sharp_edges
        .retain(|&(a, b)| (a as usize) < n && (b as usize) < n && a != b);

    // 6. Filtragem de UV pins
    let before_pins = mesh.uv_pinned.len();
    let num_faces = mesh.faces.len();
    mesh.uv_pinned.retain(|&(fi, ci)| {
        if fi < num_faces {
            ci < mesh.faces[fi].verts.len()
        } else {
            false
        }
    });
    report.uv_pins_dropped = before_pins.saturating_sub(mesh.uv_pinned.len());

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Face, Vertex};

    #[test]
    fn valid_mesh_passes_strict_invariants() {
        let mut mesh = Mesh::default();
        mesh.verts.push(Vertex::new(0.0, 0.0, 0.0));
        mesh.verts.push(Vertex::new(1.0, 0.0, 0.0));
        mesh.verts.push(Vertex::new(0.0, 1.0, 0.0));
        mesh.faces.push(Face::new(vec![0, 1, 2]));

        assert!(validate_invariants(&mesh).is_ok());
    }

    #[test]
    fn non_finite_vertex_rejected_by_strict_validation() {
        let mut mesh = Mesh::default();
        mesh.verts.push(Vertex::new(f32::NAN, 0.0, 0.0));
        mesh.verts.push(Vertex::new(1.0, 0.0, 0.0));
        mesh.verts.push(Vertex::new(0.0, 1.0, 0.0));
        mesh.faces.push(Face::new(vec![0, 1, 2]));

        match validate_invariants(&mesh) {
            Err(MeshValidationError::NonFiniteVertexPosition { index, .. }) => {
                assert_eq!(index, 0);
            }
            other => panic!("Esperava NonFiniteVertexPosition, veio {other:?}"),
        }
    }

    #[test]
    fn out_of_bounds_face_rejected_by_strict_validation() {
        let mut mesh = Mesh::default();
        mesh.verts.push(Vertex::new(0.0, 0.0, 0.0));
        mesh.verts.push(Vertex::new(1.0, 0.0, 0.0));
        mesh.faces.push(Face::new(vec![0, 1, 99])); // 99 não existe

        match validate_invariants(&mesh) {
            Err(MeshValidationError::VertexIndexOutOfBounds { vertex_index, .. }) => {
                assert_eq!(vertex_index, 99);
            }
            other => panic!("Esperava VertexIndexOutOfBounds, veio {other:?}"),
        }
    }

    #[test]
    fn duplicate_vertices_in_face_rejected_by_strict_validation() {
        let mut mesh = Mesh::default();
        mesh.verts.push(Vertex::new(0.0, 0.0, 0.0));
        mesh.verts.push(Vertex::new(1.0, 0.0, 0.0));
        mesh.verts.push(Vertex::new(0.0, 1.0, 0.0));
        mesh.faces.push(Face::new(vec![0, 1, 1])); // 1 duplicado

        match validate_invariants(&mesh) {
            Err(MeshValidationError::DuplicateFaceVertices { index, .. }) => {
                assert_eq!(index, 0);
            }
            other => panic!("Esperava DuplicateFaceVertices, veio {other:?}"),
        }
    }

    #[test]
    fn tolerant_repair_sanitizes_nan_and_drops_invalid_faces() {
        let mut mesh = Mesh::default();
        mesh.verts.push(Vertex::new(f32::NAN, 0.0, 0.0));
        mesh.verts.push(Vertex::new(1.0, 0.0, 0.0));
        mesh.verts.push(Vertex::new(0.0, 1.0, 0.0));
        mesh.faces.push(Face::new(vec![0, 1, 2]));
        mesh.faces.push(Face::new(vec![0, 1, 999])); // face inválida

        let report = repair_external_mesh(&mut mesh, RepairPolicy::TolerantImport);
        assert_eq!(report.vertices_sanitized, 1);
        assert_eq!(report.faces_dropped, 1);
        assert_eq!(mesh.faces.len(), 1);
        assert!(mesh.verts[0].pos[0].is_finite());
        assert_eq!(mesh.verts[0].pos, [0.0, 0.0, 0.0]);

        // Após reparo tolerante, a malha passa na validação estrita!
        assert!(validate_invariants(&mesh).is_ok());
    }
}
