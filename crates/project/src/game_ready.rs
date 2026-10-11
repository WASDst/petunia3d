//! Game-Ready Validator (website/docs/12-interchange/import-export-game-ready.md,
//! "Game-Ready Validator").
//!
//! Compõe validações que já existem — invariantes da malha (G02), topologia
//! half-edge e UV Health (V02) — em diagnósticos com **gravidade**, código
//! estável, alvo, mensagem simples e ação recomendada. Errors bloqueiam o export
//! daquele alvo; Warnings vão para o relatório; Info é transparência. Nada aqui
//! altera o Document: o validator não decide a estética do artista nem repara.

use uuid::Uuid;

use crate::{Asset, Project};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// Um diagnóstico acionável.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    /// Código estável (`mesh.invalid`, `uv.overlap`, …).
    pub code: &'static str,
    /// Objeto alvo.
    pub target: Uuid,
    pub target_name: String,
    pub message: String,
    /// Ação recomendada ao usuário (vazio para Info).
    pub action: &'static str,
}

/// Overlap acima disso vira Warning (mirrored/stacked UV são válidos, mas
/// pintura atingiria várias superfícies).
const OVERLAP_WARNING: f32 = 0.02;
/// Area Stretch máximo acima disso vira Warning.
const STRETCH_WARNING: f32 = 1.0;

/// Diagnósticos de um objeto, avaliado como será exportado (modifiers em cópia).
pub fn validate_asset(asset: &Asset) -> Vec<Diagnostic> {
    // Como será exportado: modifiers avaliados, em mundo.
    let mesh = asset.world_mesh().into_owned();
    let mut out = Vec::new();
    let mut push = |severity, code, message: String, action| {
        out.push(Diagnostic {
            severity,
            code,
            target: asset.id,
            target_name: asset.name.clone(),
            message,
            action,
        });
    };

    if mesh.faces.is_empty() {
        push(
            Severity::Error,
            "mesh.empty",
            "Object has no faces to export.".into(),
            "Add geometry or exclude the object from the export.",
        );
        return out;
    }
    if let Err(error) = mesh.validate_invariants() {
        push(
            Severity::Error,
            "mesh.invalid",
            error.to_string(),
            "Run Validation/Repair on the object before exporting.",
        );
        return out;
    }

    let topology = mesh.validate_topology();
    if !topology.is_manifold {
        push(
            Severity::Warning,
            "mesh.non_manifold",
            "Some edges are shared by more than two faces.".into(),
            "Check the highlighted edges; engines may shade or collide incorrectly.",
        );
    }
    if topology.isolated_vertex_count > 0 {
        push(
            Severity::Warning,
            "mesh.isolated_vertices",
            format!(
                "{} vertices are not used by any face.",
                topology.isolated_vertex_count
            ),
            "Use Cleanup > Orphan vertices to remove them.",
        );
    }

    let uv = mesh.uv_diagnostics();
    if uv.zero_area_faces > 0 {
        push(
            Severity::Warning,
            "uv.zero_area",
            format!("{} faces have no UV area.", uv.zero_area_faces),
            "Unwrap or project UVs for these faces.",
        );
    }
    if uv.overlap_ratio > OVERLAP_WARNING {
        push(
            Severity::Warning,
            "uv.overlap",
            format!(
                "{:.0}% of the covered texels are shared by more than one face.",
                uv.overlap_ratio * 100.0
            ),
            "Pack the UV islands, unless the overlap is intentional (mirrored/stacked).",
        );
    }
    if uv.max_stretch > STRETCH_WARNING {
        push(
            Severity::Warning,
            "uv.stretch",
            format!("UV area stretch reaches {:.0}%.", uv.max_stretch * 100.0),
            "Relax or re-unwrap the stretched islands.",
        );
    }
    if uv.out_of_range_corners > 0 {
        push(
            Severity::Info,
            "uv.out_of_range",
            format!("{} UV corners lie outside 0..1.", uv.out_of_range_corners),
            "",
        );
    }
    push(
        Severity::Info,
        "mesh.counts",
        format!(
            "{} vertices, {} triangles, {} UV islands.",
            mesh.vert_count(),
            mesh.tri_count(),
            uv.island_count
        ),
        "",
    );
    out
}

/// Diagnósticos dos objetos `indices` (índices inexistentes são ignorados),
/// ordenados por gravidade decrescente e depois pela ordem da cena.
pub fn validate_project(project: &Project, indices: &[usize]) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = indices
        .iter()
        .filter_map(|&i| project.assets.get(i))
        .flat_map(validate_asset)
        .collect();
    out.sort_by(|a, b| b.severity.cmp(&a.severity));
    out
}

/// `true` quando algum diagnóstico bloqueia o export.
pub fn has_errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(|d| d.severity == Severity::Error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use petunia_mesh::Mesh;

    fn project_with(mesh: Mesh) -> Project {
        let mut p = Project::new();
        p.assets.clear();
        p.add("Target", mesh);
        p
    }

    #[test]
    fn healthy_cube_has_no_errors_and_reports_counts() {
        let mut mesh = Mesh::cube(1.0);
        mesh.project_cube();
        let diagnostics = validate_project(&project_with(mesh), &[0]);
        assert!(!has_errors(&diagnostics));
        assert!(diagnostics.iter().any(|d| d.code == "mesh.counts"));
    }

    #[test]
    fn invalid_geometry_is_an_error_with_target_and_action() {
        let mut mesh = Mesh::cube(1.0);
        mesh.verts[0].pos[0] = f32::NAN;
        let p = project_with(mesh);
        let diagnostics = validate_project(&p, &[0]);
        let error = diagnostics
            .iter()
            .find(|d| d.code == "mesh.invalid")
            .unwrap();
        assert_eq!(error.severity, Severity::Error);
        assert_eq!(error.target, p.assets[0].id);
        assert!(!error.action.is_empty());
        assert!(has_errors(&diagnostics));
    }

    #[test]
    fn empty_objects_block_and_errors_sort_first() {
        let mut p = project_with(Mesh::cube(1.0));
        p.add("Empty", Mesh::default());
        let diagnostics = validate_project(&p, &[0, 1]);
        assert_eq!(diagnostics[0].code, "mesh.empty");
        assert_eq!(diagnostics[0].target_name, "Empty");
    }
}
