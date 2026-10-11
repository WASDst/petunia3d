//! Generic UV unwrap fallback provider (`xatlas-rs-v2`, P1-06 / G01).
//!
//! Petunia native deterministic/projection workflows (`project_planar` and
//! friends) remain first-class. This provider is the generic fallback for
//! arbitrary meshes. No xatlas type ever becomes the canonical project
//! representation (website/docs/07-uv/architecture-decision.md, "Auto Unwrap").
//!
//! Fluxo (a topologia autoral nunca muda):
//!
//! ```text
//! authoring faces
//! → triangulação derivada canônica (Mesh::face_triangle_corners)
//! → proveniência triângulo-canto → FaceCorner original
//! → xatlas
//! → UV de volta aos FaceCorners originais
//! → seams derivadas das fronteiras de chart
//! ```
//!
//! Todas as UVs são calculadas antes de qualquer escrita: em erro a malha
//! fica intacta.
//!
//! Provider note: `xatlas-rs` 0.1.3 ships Windows-only pre-generated bindings
//! (unusable on Linux) and its `generate_bindings` path fails against modern
//! libclang, so this boundary uses `xatlas-rs-v2` 0.1.4 — the maintained
//! evolution from the same upstream repository. Não usamos o modo polígono do
//! xatlas (`face_vertex_count`): ele não mantém os triângulos de um polígono no
//! mesmo chart e, ao remontar, fica com o primeiro chart sem avisar.

use crate::Mesh;

/// xatlas input limits (hostile-mesh class).
const MAX_VERTS: usize = 200_000;
const MAX_TRIS: usize = 400_000;

/// Fallback unwrap failure. Em qualquer variante a malha não foi alterada.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum UnwrapError {
    /// Nothing to unwrap (empty mesh).
    #[error("mesh is empty")]
    Empty,
    /// Mesh exceeds provider limits.
    #[error("mesh exceeds xatlas fallback limits")]
    TooLarge,
    /// Non-finite coordinates or bad indices.
    #[error("mesh has invalid geometry")]
    InvalidGeometry,
    /// Provider produced no usable UVs.
    #[error("xatlas produced no UVs")]
    NoOutput,
    /// Os triângulos derivados de uma face autoral caíram em charts
    /// diferentes (ou foram ignorados pelo provider) e não há UV coerente
    /// para todos os cantos da face.
    #[error("face {face} cannot receive one coherent UV chart")]
    FaceSplitAcrossCharts { face: usize },
}

/// Um triângulo derivado de uma face autoral e o que o xatlas devolveu para ele.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TriangleSample {
    /// Chart do xatlas; `None` quando o provider ignorou o triângulo
    /// (por exemplo, área abaixo do epsilon dele).
    pub chart: Option<u32>,
    /// `(índice do canto na face autoral, UV normalizada)` para os três cantos.
    pub corners: [(usize, [f32; 2]); 3],
}

/// A face autoral não pode receber uma UV coerente a partir dos seus triângulos.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FaceUvConflict;

/// Unwraps `mesh` in place with the xatlas fallback, preserving the authoring
/// topology (quads and n-gons stay quads and n-gons). Faces whose derived
/// triangulation is empty (degenerate) keep their current UVs. Returns the
/// number of charts.
pub fn unwrap_fallback(mesh: &mut Mesh) -> Result<usize, UnwrapError> {
    if mesh.verts.is_empty() || mesh.faces.is_empty() {
        return Err(UnwrapError::Empty);
    }
    if mesh.verts.len() > MAX_VERTS {
        return Err(UnwrapError::TooLarge);
    }
    let vertex_count = mesh.verts.len();
    if mesh
        .verts
        .iter()
        .any(|v| !v.pos.iter().all(|x| x.is_finite()))
        || mesh
            .faces
            .iter()
            .any(|f| f.verts.iter().any(|&i| i as usize >= vertex_count))
    {
        return Err(UnwrapError::InvalidGeometry);
    }

    // Triangulação derivada + proveniência: `sources[t]` é a face e os cantos
    // autorais do triângulo `t`; nada disso é escrito de volta na malha.
    let mut indices: Vec<u32> = Vec::new();
    let mut sources: Vec<(usize, [usize; 3])> = Vec::new();
    for fi in 0..mesh.faces.len() {
        let face = &mesh.faces[fi];
        for corners in mesh.face_triangle_corners(fi) {
            indices.extend(corners.map(|c| face.verts[c]));
            sources.push((fi, corners));
        }
    }
    if sources.is_empty() {
        return Err(UnwrapError::InvalidGeometry);
    }
    if sources.len() > MAX_TRIS {
        return Err(UnwrapError::TooLarge);
    }

    let mut positions = Vec::with_capacity(vertex_count * 3);
    for v in &mesh.verts {
        positions.extend_from_slice(&v.pos);
    }
    let decl = xatlas_rs_v2::MeshDecl {
        vertex_position_data: xatlas_rs_v2::MeshData::Contiguous(&positions),
        index_data: Some(xatlas_rs_v2::IndexData::U32(&indices)),
        face_count: sources.len() as u32,
        ..Default::default()
    };
    // Declared after `decl` so it drops first (it borrows the buffers).
    let mut atlas = xatlas_rs_v2::Xatlas::new();
    atlas
        .add_mesh(&decl)
        .map_err(|_| UnwrapError::InvalidGeometry)?;
    atlas.generate(
        &xatlas_rs_v2::ChartOptions::default(),
        &xatlas_rs_v2::PackOptions {
            padding: 2,
            bilinear: true,
            resolution: 1024,
            ..Default::default()
        },
    );
    let (atlas_w, atlas_h) = (atlas.width().max(1) as f32, atlas.height().max(1) as f32);
    let meshes = atlas.meshes();
    let output = meshes.first().ok_or(UnwrapError::NoOutput)?;
    if output.index_array.len() != sources.len() * 3 {
        return Err(UnwrapError::NoOutput);
    }

    // Amostras por face, na ordem em que os triângulos foram emitidos.
    // As UVs saem em texels do atlas, então normalizamos.
    let mut samples: Vec<Vec<TriangleSample>> = vec![Vec::new(); mesh.faces.len()];
    for (tri, (fi, corners)) in output.index_array.as_chunks::<3>().0.iter().zip(&sources) {
        let mut chart = None;
        let mut out = [(0usize, [0.0f32; 2]); 3];
        for (k, &oi) in tri.iter().enumerate() {
            let vert = output
                .vertex_array
                .get(oi as usize)
                .ok_or(UnwrapError::NoOutput)?;
            if !vert.uv.iter().all(|u| u.is_finite()) {
                return Err(UnwrapError::NoOutput);
            }
            chart = u32::try_from(vert.chart_index).ok();
            out[k] = (
                corners[k],
                [
                    (vert.uv[0] / atlas_w).clamp(0.0, 1.0),
                    (vert.uv[1] / atlas_h).clamp(0.0, 1.0),
                ],
            );
        }
        samples[*fi].push(TriangleSample {
            chart,
            corners: out,
        });
    }

    let mut new_uv: Vec<Option<Vec<[f32; 2]>>> = Vec::with_capacity(mesh.faces.len());
    for (fi, face_samples) in samples.iter().enumerate() {
        if face_samples.is_empty() {
            // Face degenerada: sem triangulação derivada, mantém a UV atual.
            new_uv.push(None);
            continue;
        }
        let uv = resolve_face_uvs(mesh.faces[fi].verts.len(), face_samples)
            .map_err(|FaceUvConflict| UnwrapError::FaceSplitAcrossCharts { face: fi })?;
        new_uv.push(Some(uv));
    }

    // Commit: só agora a malha muda (UV por FaceCorner + seams de chart).
    for (face, uv) in mesh.faces.iter_mut().zip(new_uv) {
        if let Some(uv) = uv {
            face.uv = uv;
        }
    }
    let chart_seams = mesh.uv_split_edges();
    mesh.uv_seams.extend(chart_seams);
    Ok(output.chart_array.len())
}

/// Monta a UV de cada canto de uma face autoral a partir das amostras dos seus
/// triângulos derivados. Cada canto aparece em pelo menos uma amostra quando a
/// triangulação é válida; um canto compartilhado por dois triângulos do mesmo
/// chart recebe a mesma UV do xatlas.
///
/// Política estrita (recusa segura): todos os triângulos válidos precisam estar
/// no mesmo chart e todo canto precisa receber UV. Escolher um chart
/// "dominante" esticaria a face entre regiões distantes do atlas sem avisar.
pub(crate) fn resolve_face_uvs(
    corner_count: usize,
    samples: &[TriangleSample],
) -> Result<Vec<[f32; 2]>, FaceUvConflict> {
    let mut charts = samples.iter().filter_map(|s| s.chart);
    let chart = charts.next().ok_or(FaceUvConflict)?;
    if charts.any(|c| c != chart) {
        return Err(FaceUvConflict);
    }
    let mut uv: Vec<Option<[f32; 2]>> = vec![None; corner_count];
    for sample in samples.iter().filter(|s| s.chart == Some(chart)) {
        for &(corner, value) in &sample.corners {
            *uv.get_mut(corner).ok_or(FaceUvConflict)? = Some(value);
        }
    }
    uv.into_iter().map(|c| c.ok_or(FaceUvConflict)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(chart: Option<u32>, corners: [(usize, [f32; 2]); 3]) -> TriangleSample {
        TriangleSample { chart, corners }
    }

    #[test]
    fn cube_unwrap_preserves_quads_and_writes_finite_corner_uvs() {
        let mut mesh = Mesh::cube(2.0);
        let face_sizes: Vec<usize> = mesh.faces.iter().map(|f| f.verts.len()).collect();
        let verts_before = mesh.verts.len();

        let charts = unwrap_fallback(&mut mesh).unwrap();

        assert!(charts >= 1);
        assert_eq!(mesh.verts.len(), verts_before);
        assert_eq!(
            mesh.faces.iter().map(|f| f.verts.len()).collect::<Vec<_>>(),
            face_sizes
        );
        for face in &mesh.faces {
            assert_eq!(face.uv.len(), face.verts.len());
            assert!(face.uv.iter().flatten().all(|u| u.is_finite()));
        }
    }

    #[test]
    fn unwrap_marks_chart_boundaries_and_keeps_manual_seams() {
        let mut mesh = Mesh::cube(2.0);
        let manual = crate::edge_key(mesh.faces[0].verts[0], mesh.faces[0].verts[1]);
        mesh.uv_seams.insert(manual);

        unwrap_fallback(&mut mesh).unwrap();

        // Um cubo fechado não abre num único chart plano: há fronteiras.
        assert!(mesh.uv_seams.contains(&manual));
        assert!(mesh.uv_seams.len() > 1);
        assert!(mesh.uv_split_edges().is_subset(&mesh.uv_seams));
    }

    #[test]
    fn empty_and_degenerate_inputs_are_rejected_without_mutation() {
        assert_eq!(
            unwrap_fallback(&mut Mesh::default()).unwrap_err(),
            UnwrapError::Empty
        );
        let mut bad = Mesh::cube(1.0);
        bad.verts[0].pos[0] = f32::NAN;
        let snapshot = |m: &Mesh| -> Vec<(Vec<u32>, Vec<[f32; 2]>)> {
            m.faces
                .iter()
                .map(|f| (f.verts.clone(), f.uv.clone()))
                .collect()
        };
        let before = snapshot(&bad);
        assert_eq!(
            unwrap_fallback(&mut bad).unwrap_err(),
            UnwrapError::InvalidGeometry
        );
        assert_eq!(snapshot(&bad), before);
    }

    #[test]
    fn resolve_accepts_a_quad_whose_triangles_share_one_chart() {
        let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let samples = [
            sample(Some(3), [(0, uv[0]), (1, uv[1]), (2, uv[2])]),
            sample(Some(3), [(0, uv[0]), (2, uv[2]), (3, uv[3])]),
        ];
        assert_eq!(resolve_face_uvs(4, &samples), Ok(uv.to_vec()));
    }

    #[test]
    fn resolve_refuses_a_face_split_across_charts() {
        let samples = [
            sample(Some(0), [(0, [0.0, 0.0]), (1, [1.0, 0.0]), (2, [1.0, 1.0])]),
            sample(Some(1), [(0, [0.5, 0.5]), (2, [0.9, 0.9]), (3, [0.5, 0.9])]),
        ];
        assert_eq!(resolve_face_uvs(4, &samples), Err(FaceUvConflict));
    }

    #[test]
    fn resolve_never_returns_a_corner_without_uv() {
        // Canto 3 só existe num triângulo ignorado pelo provider.
        let samples = [
            sample(Some(0), [(0, [0.0, 0.0]), (1, [1.0, 0.0]), (2, [1.0, 1.0])]),
            sample(None, [(0, [0.0, 0.0]), (2, [0.0, 0.0]), (3, [0.0, 0.0])]),
        ];
        assert_eq!(resolve_face_uvs(4, &samples), Err(FaceUvConflict));
    }
}
