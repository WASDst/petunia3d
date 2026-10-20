//! Snapshot intermediário avaliado e agnóstico ao formato de destino (I01 / I02).
//!
//! Conforme definido em `website/docs/12-interchange/import-export-game-ready.md`:
//! - Exportar é compilar uma cópia do authoring, nunca destrutivamente converter o Document.
//! - Um único pipeline atende Single, Multiple e Batch Export.
//! - Gera tangentes ortonormais determinísticas (estilo MikkTSpace com handedness +/- 1.0).
//! - Preserva seams e sharp edges via vertex splitting derivado por corner (pos, UV, normal).
//! - Emite GLB binário conforme Khronos glTF 2.0 e Wavefront OBJ geométrico.

use std::collections::HashMap;
use uuid::Uuid;

use crate::export::{encode_png, json_str};
use crate::gltf_rig::{
    BinBuilder, COMPONENT_U16, RigDoc, SkinBinding, TARGET_ARRAY_BUFFER, bind_skin, build_rig_doc,
};
use crate::pipeline::{ExportOptions, PipelineError};
use crate::snapshot::DocumentSnapshot;
use crate::{Asset, Project};

/// Snapshot intermediário avaliado agnóstico ao formato de destino.
#[derive(Clone, Debug, PartialEq)]
pub struct ExportModel {
    pub meshes: Vec<ExportMesh>,
    pub materials: Vec<ExportMaterial>,
    pub metadata: ExportMetadata,
    pub warnings: Vec<String>,
}

/// Malha compilada para exportação com atributos alinhados por vértice.
#[derive(Clone, Debug, PartialEq)]
pub struct ExportMesh {
    pub id: Uuid,
    pub name: String,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    /// Vetor 4D de tangente: XYZ ortonormalizado e W representando handedness (+1.0 ou -1.0).
    pub tangents: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
    pub material_index: Option<usize>,
    pub skin: Option<SkinBinding>,
    pub joints: Vec<u16>,
    pub weights: Vec<f32>,
    pub decal_sets: Option<String>,
}

/// Material PBR compilado pronto para emissão em formatos de jogo.
#[derive(Clone, Debug, PartialEq)]
pub struct ExportMaterial {
    pub id: Option<Uuid>,
    pub name: String,
    pub base_color: [f32; 4],
    pub roughness: f32,
    pub metallic: f32,
    pub emissive: [f32; 3],
    pub albedo_texture_png: Option<Vec<u8>>,
}

/// Metadados de exportação com declaração de eixos e unidades.
#[derive(Clone, Debug, PartialEq)]
pub struct ExportMetadata {
    pub generator: String,
    pub units: String,
    pub coordinate_system: String,
    pub source_objects: Vec<Uuid>,
}

impl Default for ExportMetadata {
    fn default() -> Self {
        Self {
            generator: "Petunia3D".to_string(),
            units: "meters".to_string(),
            coordinate_system: "Y-up, right-handed".to_string(),
            source_objects: Vec::new(),
        }
    }
}

impl ExportModel {
    /// Compila um `ExportModel` a partir de um `DocumentSnapshot` imutável.
    pub fn from_snapshot(
        snapshot: &DocumentSnapshot,
        indices: &[usize],
        options: &ExportOptions,
    ) -> Result<Self, PipelineError> {
        Self::from_project(snapshot.document(), indices, options)
    }

    /// Compila um `ExportModel` a partir de um `Project` em cópia estrita.
    pub fn from_project(
        project: &Project,
        indices: &[usize],
        options: &ExportOptions,
    ) -> Result<Self, PipelineError> {
        let picked: Vec<&Asset> = indices
            .iter()
            .filter_map(|&i| project.assets.get(i))
            .collect();

        if picked.is_empty() {
            return Err(PipelineError::Validation(
                "Nenhum asset selecionado para exportação".into(),
            ));
        }

        let mut out_meshes = Vec::with_capacity(picked.len());
        let mut out_materials = Vec::new();
        let mut material_id_map: HashMap<Uuid, usize> = HashMap::new();
        let mut warnings = Vec::new();
        let mut source_objects = Vec::with_capacity(picked.len());

        for asset in picked {
            source_objects.push(asset.id);

            // Validação Game-Ready: Severity::Error bloqueia a compilação
            let diagnostics = crate::game_ready::validate_asset(asset);
            if crate::game_ready::has_errors(&diagnostics) {
                let errs: Vec<String> = diagnostics
                    .iter()
                    .filter(|d| d.severity == crate::game_ready::Severity::Error)
                    .map(|d| format!("{}: {}", d.code, d.message))
                    .collect();
                return Err(PipelineError::Validation(format!(
                    "Asset '{}' rejeitado pelo Game-Ready Validator: {}",
                    asset.name,
                    errs.join("; ")
                )));
            }

            for d in diagnostics
                .iter()
                .filter(|d| d.severity == crate::game_ready::Severity::Warning)
            {
                warnings.push(format!(
                    "Asset '{}': {} - {}",
                    asset.name, d.code, d.message
                ));
            }

            // Material mapping
            let mat_idx = if options.export_materials {
                if let Some(mat) = asset.material(project) {
                    if let Some(&existing_idx) = material_id_map.get(&mat.id) {
                        Some(existing_idx)
                    } else {
                        let idx = out_materials.len();
                        material_id_map.insert(mat.id, idx);
                        let texture_png = asset
                            .texture
                            .as_ref()
                            .or(mat.albedo_texture.as_ref())
                            .and_then(encode_png);

                        out_materials.push(ExportMaterial {
                            id: Some(mat.id),
                            name: mat.name.clone(),
                            base_color: mat.base_color,
                            roughness: mat.roughness.clamp(0.0, 1.0),
                            metallic: mat.metallic.clamp(0.0, 1.0),
                            emissive: [
                                mat.emission_color[0] * mat.emission_strength,
                                mat.emission_color[1] * mat.emission_strength,
                                mat.emission_color[2] * mat.emission_strength,
                            ],
                            albedo_texture_png: texture_png,
                        });
                        Some(idx)
                    }
                } else {
                    let texture_png = asset.texture.as_ref().and_then(encode_png);
                    let idx = out_materials.len();
                    out_materials.push(ExportMaterial {
                        id: None,
                        name: format!("{}_mat", asset.name),
                        base_color: [
                            asset.base_color[0],
                            asset.base_color[1],
                            asset.base_color[2],
                            1.0,
                        ],
                        roughness: 0.9,
                        metallic: 0.0,
                        emissive: [0.0, 0.0, 0.0],
                        albedo_texture_png: texture_png,
                    });
                    Some(idx)
                }
            } else {
                None
            };

            // Avaliação geométrica em cópia
            let evaluated = asset.world_mesh().into_owned();
            let mut triangulated = evaluated.clone();
            if options.triangulate {
                triangulated.triangulate();
            }

            // Aplicar escala uniforme nas posições se solicitado
            let scale = if options.scale.is_finite() && options.scale > 0.0 {
                options.scale
            } else {
                1.0
            };
            if (scale - 1.0).abs() > 1e-5 {
                for v in &mut triangulated.verts {
                    v.pos = [v.pos[0] * scale, v.pos[1] * scale, v.pos[2] * scale];
                }
            }

            // Validar coordenadas finitas antes de gerar buffers
            if !triangulated.verts.iter().all(|v| {
                v.pos.iter().all(|x| x.is_finite()) && v.color.iter().all(|x| x.is_finite())
            }) {
                return Err(PipelineError::Validation(format!(
                    "Asset '{}' contém vértices com coordenadas não-finitas (NaN/Inf)",
                    asset.name
                )));
            }

            // Corners sombreados canônicos com preservação de FaceCorner UVs e sharp normals
            let corners = triangulated.shaded_corners(asset.surface_shading);
            if corners.is_empty() || corners.len() % 3 != 0 {
                return Err(PipelineError::Validation(format!(
                    "Asset '{}' sem geometria triangular válida",
                    asset.name
                )));
            }

            let num_tris = corners.len() / 3;
            let mut corner_tangents: Vec<[f32; 4]> = Vec::with_capacity(corners.len());

            // Cálculo determinístico de tangentes estilo MikkTSpace com handedness
            for t in 0..num_tris {
                let c0 = &corners[t * 3];
                let c1 = &corners[t * 3 + 1];
                let c2 = &corners[t * 3 + 2];

                let p0 = glam::Vec3::from(c0.position);
                let p1 = glam::Vec3::from(c1.position);
                let p2 = glam::Vec3::from(c2.position);

                let e1 = p1 - p0;
                let e2 = p2 - p0;

                let du1 = c1.uv[0] - c0.uv[0];
                let dv1 = c1.uv[1] - c0.uv[1];
                let du2 = c2.uv[0] - c0.uv[0];
                let dv2 = c2.uv[1] - c0.uv[1];

                let det = du1 * dv2 - du2 * dv1;
                let (t_raw, b_raw) = if det.abs() > 1e-8 && det.is_finite() {
                    let inv = 1.0 / det;
                    let tan = (e1 * dv2 - e2 * dv1) * inv;
                    let bit = (e2 * du1 - e1 * du2) * inv;
                    (tan, bit)
                } else {
                    let tan = if e1.length_squared() > 1e-6 {
                        e1.normalize()
                    } else {
                        glam::Vec3::X
                    };
                    (tan, glam::Vec3::Y)
                };

                for c in [c0, c1, c2] {
                    let n = glam::Vec3::from(c.normal).normalize_or_zero();
                    let proj = t_raw - n * n.dot(t_raw);
                    let t_ortho = if proj.length_squared() > 1e-6 {
                        proj.normalize()
                    } else {
                        let arb = if n.x.abs() > 0.9 {
                            glam::Vec3::Y
                        } else {
                            glam::Vec3::X
                        };
                        (arb - n * n.dot(arb)).normalize_or_zero()
                    };

                    let handedness = if n.cross(t_ortho).dot(b_raw) < 0.0 {
                        -1.0f32
                    } else {
                        1.0f32
                    };

                    corner_tangents.push([t_ortho.x, t_ortho.y, t_ortho.z, handedness]);
                }
            }

            // Split de vértices em corners para evitar sangramento de seams/UVs e sharp edges
            let skin = bind_skin(project, asset, triangulated.verts.len());
            let mut positions: Vec<[f32; 3]> = Vec::new();
            let mut normals: Vec<[f32; 3]> = Vec::new();
            let mut uvs: Vec<[f32; 2]> = Vec::new();
            let mut tangents: Vec<[f32; 4]> = Vec::new();
            let mut indices_out: Vec<u32> = Vec::new();
            let mut joints: Vec<u16> = Vec::new();
            let mut weights: Vec<f32> = Vec::new();

            let mut lut: HashMap<(u32, [u32; 2], [u32; 3]), u32> = HashMap::new();

            for (ci, corner) in corners.iter().enumerate() {
                let key = (
                    corner.vertex,
                    [corner.uv[0].to_bits(), corner.uv[1].to_bits()],
                    [
                        corner.normal[0].to_bits(),
                        corner.normal[1].to_bits(),
                        corner.normal[2].to_bits(),
                    ],
                );
                let vi = corner.vertex;
                let tan = corner_tangents
                    .get(ci)
                    .copied()
                    .unwrap_or([1.0, 0.0, 0.0, 1.0]);

                let id = *lut.entry(key).or_insert_with(|| {
                    let id = positions.len() as u32;
                    positions.push(corner.position);
                    normals.push(corner.normal);
                    uvs.push(corner.uv);
                    tangents.push(tan);
                    if let Some(s) = &skin {
                        if let Some(j) = s.joints.get(vi as usize) {
                            joints.extend_from_slice(j);
                        } else {
                            joints.extend_from_slice(&[0, 0, 0, 0]);
                        }
                        if let Some(w) = s.weights.get(vi as usize) {
                            weights.extend_from_slice(w);
                        } else {
                            weights.extend_from_slice(&[1.0, 0.0, 0.0, 0.0]);
                        }
                    }
                    id
                });
                indices_out.push(id);
            }

            let decal_sets = crate::export::export_report(project, &[0], false)
                .into_iter()
                .next();
            let _ = decal_sets;

            out_meshes.push(ExportMesh {
                id: asset.id,
                name: asset.name.clone(),
                positions,
                normals,
                uvs,
                tangents,
                indices: indices_out,
                material_index: mat_idx,
                skin,
                joints,
                weights,
                decal_sets: None,
            });
        }

        Ok(Self {
            meshes: out_meshes,
            materials: out_materials,
            metadata: ExportMetadata {
                generator: "Petunia3D".to_string(),
                units: "meters".to_string(),
                coordinate_system: "Y-up, right-handed".to_string(),
                source_objects,
            },
            warnings,
        })
    }

    /// Serializa o modelo compilado em formato glTF 2.0 Binary (GLB) game-ready.
    pub fn to_glb(&self, project: &Project) -> Result<Vec<u8>, PipelineError> {
        if self.meshes.is_empty() {
            return Err(PipelineError::Export(
                "Nada para exportar no ExportModel".into(),
            ));
        }

        let mut bb = BinBuilder::default();

        struct MeshAcc {
            pos: usize,
            nrm: usize,
            uv: usize,
            tan: usize,
            idx: usize,
            skin_attrs: Option<(usize, usize)>,
        }

        let mut mesh_accs: Vec<MeshAcc> = Vec::with_capacity(self.meshes.len());

        for m in &self.meshes {
            // Positions min/max
            let mut min_pos = vec![f32::MAX; 3];
            let mut max_pos = vec![f32::MIN; 3];
            let mut pos_flat = Vec::with_capacity(m.positions.len() * 3);
            for p in &m.positions {
                pos_flat.extend_from_slice(p);
                for c in 0..3 {
                    min_pos[c] = min_pos[c].min(p[c]);
                    max_pos[c] = max_pos[c].max(p[c]);
                }
            }
            let pos_acc = bb.push_f32(
                &pos_flat,
                "VEC3",
                3,
                TARGET_ARRAY_BUFFER,
                Some((min_pos, max_pos)),
            );

            // Normals
            let mut nrm_flat = Vec::with_capacity(m.normals.len() * 3);
            for n in &m.normals {
                nrm_flat.extend_from_slice(n);
            }
            let nrm_acc = bb.push_f32(&nrm_flat, "VEC3", 3, TARGET_ARRAY_BUFFER, None);

            // UVs
            let mut uv_flat = Vec::with_capacity(m.uvs.len() * 2);
            for uv in &m.uvs {
                uv_flat.extend_from_slice(uv);
            }
            let uv_acc = bb.push_f32(&uv_flat, "VEC2", 2, TARGET_ARRAY_BUFFER, None);

            // Tangents (VEC4)
            let mut tan_flat = Vec::with_capacity(m.tangents.len() * 4);
            for tan in &m.tangents {
                tan_flat.extend_from_slice(tan);
            }
            let tan_acc = bb.push_f32(&tan_flat, "VEC4", 4, TARGET_ARRAY_BUFFER, None);

            // Indices
            let idx_bytes: Vec<u8> = m.indices.iter().flat_map(|i| i.to_le_bytes()).collect();
            let idx_view = bb.push_view(&idx_bytes, 34963);
            let idx_acc = bb.push_acc(idx_view, m.indices.len(), "SCALAR", 5125, None);

            // Skin (JOINTS_0, WEIGHTS_0)
            let skin_attrs = if m.skin.is_some() && !m.joints.is_empty() {
                let jv = bb.push_view(bytemuck::cast_slice(&m.joints), TARGET_ARRAY_BUFFER);
                let ja = bb.push_acc(jv, m.joints.len() / 4, "VEC4", COMPONENT_U16, None);
                let wa = bb.push_f32(&m.weights, "VEC4", 4, TARGET_ARRAY_BUFFER, None);
                Some((ja, wa))
            } else {
                None
            };

            mesh_accs.push(MeshAcc {
                pos: pos_acc,
                nrm: nrm_acc,
                uv: uv_acc,
                tan: tan_acc,
                idx: idx_acc,
                skin_attrs,
            });
        }

        // Rig e skeletons
        let mut skeleton_ids: Vec<Uuid> = Vec::new();
        for m in &self.meshes {
            if let Some(skin) = &m.skin
                && !skeleton_ids.contains(&skin.skeleton_id)
            {
                skeleton_ids.push(skin.skeleton_id);
            }
        }
        let rig = if skeleton_ids.is_empty() {
            RigDoc::default()
        } else {
            build_rig_doc(project, &skeleton_ids, self.meshes.len(), &mut bb)
                .map_err(|e| PipelineError::Export(e.to_string()))?
        };

        // Texturas embutidas
        let mut image_views: Vec<usize> = Vec::new();
        let mut mat_image_view: Vec<Option<usize>> = Vec::with_capacity(self.materials.len());
        for mat in &self.materials {
            if let Some(png) = &mat.albedo_texture_png {
                let view_idx = bb.push_view(png, 0);
                image_views.push(view_idx);
                mat_image_view.push(Some(image_views.len() - 1));
            } else {
                mat_image_view.push(None);
            }
        }

        // Montagem do JSON estrutural glTF 2.0
        let mut j = String::from("{\"asset\":{\"version\":\"2.0\",\"generator\":\"Petunia3D\"},");
        j.push_str("\"scene\":0,\"scenes\":[{\"nodes\":[");
        let scene_nodes: Vec<usize> = (0..self.meshes.len())
            .chain(rig.scene_roots.iter().copied())
            .collect();
        for (i, n) in scene_nodes.iter().enumerate() {
            if i > 0 {
                j.push(',');
            }
            j.push_str(&n.to_string());
        }
        j.push_str("]}],\"nodes\":[");
        for (i, m) in self.meshes.iter().enumerate() {
            if i > 0 {
                j.push(',');
            }
            let skin_field = m
                .skin
                .as_ref()
                .and_then(|s| rig.skin_of.get(&s.skeleton_id))
                .map(|k| format!(",\"skin\":{k}"))
                .unwrap_or_default();
            j.push_str(&format!(
                "{{\"name\":{},\"mesh\":{i}{skin_field}}}",
                json_str(&m.name)
            ));
        }
        for node in &rig.nodes {
            j.push(',');
            j.push_str(&node.to_string());
        }
        j.push_str("],\"meshes\":[");
        for (i, m) in self.meshes.iter().enumerate() {
            if i > 0 {
                j.push(',');
            }
            let a = &mesh_accs[i];
            let skin_attrs = a
                .skin_attrs
                .map(|(jn, w)| format!(",\"JOINTS_0\":{jn},\"WEIGHTS_0\":{w}"))
                .unwrap_or_default();
            let mat_prop = m
                .material_index
                .map(|idx| format!(",\"material\":{idx}"))
                .unwrap_or_default();
            j.push_str(&format!(
                "{{\"name\":{},\"primitives\":[{{\"attributes\":{{\"POSITION\":{},\"NORMAL\":{},\"TEXCOORD_0\":{},\"TANGENT\":{}{skin_attrs}}},\"indices\":{}{mat_prop}}}]}}",
                json_str(&m.name), a.pos, a.nrm, a.uv, a.tan, a.idx
            ));
        }

        j.push_str("],\"materials\":[");
        for (i, mat) in self.materials.iter().enumerate() {
            if i > 0 {
                j.push(',');
            }
            let mut pbr = format!(
                "\"baseColorFactor\":[{:.4},{:.4},{:.4},{:.4}],\"metallicFactor\":{:.4},\"roughnessFactor\":{:.4}",
                mat.base_color[0],
                mat.base_color[1],
                mat.base_color[2],
                mat.base_color[3],
                mat.metallic,
                mat.roughness,
            );
            if let Some(img_idx) = mat_image_view.get(i).copied().flatten() {
                pbr.push_str(&format!(",\"baseColorTexture\":{{\"index\":{img_idx}}}"));
            }
            j.push_str(&format!(
                "{{\"name\":{},\"doubleSided\":true,\"pbrMetallicRoughness\":{{{pbr}}},\"emissiveFactor\":[{:.4},{:.4},{:.4}]}}",
                json_str(&mat.name),
                mat.emissive[0], mat.emissive[1], mat.emissive[2]
            ));
        }
        j.push(']');

        if !image_views.is_empty() {
            j.push_str(",\"images\":[");
            for (i, view_idx) in image_views.iter().enumerate() {
                if i > 0 {
                    j.push(',');
                }
                j.push_str(&format!(
                    "{{\"name\":\"petunia_tex_{i}\",\"bufferView\":{view_idx},\"mimeType\":\"image/png\"}}"
                ));
            }
            j.push_str("],\"textures\":[");
            for (i, _) in image_views.iter().enumerate() {
                if i > 0 {
                    j.push(',');
                }
                j.push_str(&format!("{{\"source\":{i}}}"));
            }
            j.push(']');
        }

        if !rig.skins.is_empty() {
            j.push_str(",\"skins\":");
            j.push_str(&serde_json::Value::Array(rig.skins.clone()).to_string());
        }
        if !rig.animations.is_empty() {
            j.push_str(",\"animations\":");
            j.push_str(&serde_json::Value::Array(rig.animations.clone()).to_string());
        }

        j.push_str(",\"accessors\":[");
        for (i, (v, count, ty, comp, mm)) in bb.accs.iter().enumerate() {
            if i > 0 {
                j.push(',');
            }
            j.push_str(&format!(
                "{{\"bufferView\":{v},\"componentType\":{comp},\"count\":{count},\"type\":\"{ty}\""
            ));
            if let Some((mn, mx)) = mm {
                let fmt = |v: &Vec<f32>| {
                    v.iter()
                        .map(|x| format!("{x:.6}"))
                        .collect::<Vec<_>>()
                        .join(",")
                };
                j.push_str(&format!(",\"min\":[{}],\"max\":[{}]", fmt(mn), fmt(mx)));
            }
            j.push('}');
        }

        j.push_str("],\"bufferViews\":[");
        for (i, (off, len, target)) in bb.views.iter().enumerate() {
            if i > 0 {
                j.push(',');
            }
            if *target == 0 {
                j.push_str(&format!(
                    "{{\"buffer\":0,\"byteOffset\":{off},\"byteLength\":{len}}}"
                ));
            } else {
                j.push_str(&format!(
                    "{{\"buffer\":0,\"byteOffset\":{off},\"byteLength\":{len},\"target\":{target}}}"
                ));
            }
        }
        bb.align4();
        j.push_str(&format!(
            "],\"buffers\":[{{\"byteLength\":{}}}]}}",
            bb.bin.len()
        ));

        let bin = std::mem::take(&mut bb.bin);
        let mut json = j.into_bytes();
        while !json.len().is_multiple_of(4) {
            json.push(b' ');
        }

        let total = 12 + 8 + json.len() + 8 + bin.len();
        let mut out = Vec::with_capacity(total);
        out.extend_from_slice(&0x46546C67u32.to_le_bytes()); // 'glTF'
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&(total as u32).to_le_bytes());
        out.extend_from_slice(&(json.len() as u32).to_le_bytes());
        out.extend_from_slice(&0x4E4F534Au32.to_le_bytes()); // 'JSON'
        out.extend_from_slice(&json);
        out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        out.extend_from_slice(&0x004E4942u32.to_le_bytes()); // 'BIN\0'
        out.extend_from_slice(&bin);

        Ok(out)
    }

    /// Serializa o modelo compilado em formato Wavefront OBJ.
    pub fn to_obj(&self) -> Result<String, PipelineError> {
        if self.meshes.is_empty() {
            return Err(PipelineError::Export(
                "Nada para exportar no ExportModel".into(),
            ));
        }

        let mut out = String::new();
        out.push_str("# Petunia3D Wavefront OBJ Export\n");
        out.push_str("# Units: meters (Y-up)\n\n");

        let mut base_idx = 1u32;

        for m in &self.meshes {
            out.push_str(&format!("o {}\n", m.name));

            for p in &m.positions {
                out.push_str(&format!("v {:.6} {:.6} {:.6}\n", p[0], p[1], p[2]));
            }

            for uv in &m.uvs {
                out.push_str(&format!("vt {:.6} {:.6}\n", uv[0], uv[1]));
            }

            for n in &m.normals {
                out.push_str(&format!("vn {:.6} {:.6} {:.6}\n", n[0], n[1], n[2]));
            }

            for tri in m.indices.chunks(3) {
                if tri.len() == 3 {
                    let i0 = base_idx + tri[0];
                    let i1 = base_idx + tri[1];
                    let i2 = base_idx + tri[2];
                    out.push_str(&format!("f {i0}/{i0}/{i0} {i1}/{i1}/{i1} {i2}/{i2}/{i2}\n"));
                }
            }

            base_idx += m.positions.len() as u32;
            out.push('\n');
        }

        Ok(out)
    }
}
