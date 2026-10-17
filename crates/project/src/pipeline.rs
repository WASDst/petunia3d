//! Pipeline de Importação, Exportação e Entrega Modular (Wave 8 — P3D-068 a P3D-072, P3D-124).
//!
//! Arquitetura desacoplada de I/O de formatos 3D:
//! - P3D-068: Export individual com validação e relatório.
//! - P3D-069: Export múltiplo orquestrado sobre o mesmo pipeline.
//! - P3D-070: Batch export com nomes determinísticos e tolerância a falhas parciais.
//! - P3D-071: Importadores modulares com declaração de capabilities.
//! - P3D-072: Exportadores modulares plugáveis e registráveis.
//! - P3D-124: Validação contra arquivos corrompidos e fixtures de round-trip.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use petunia_mesh::Mesh;

use crate::import_gltf;
use crate::import_obj::{self, ObjImportError};
use crate::package;
use crate::{Asset, Material, Project};

/// Formatos de arquivo 3D e container suportados pelo Petunia3D.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FileFormat {
    /// Wavefront OBJ (texto ascii).
    Obj,
    /// glTF 2.0 (JSON descritor).
    Gltf,
    /// glTF 2.0 Binário (.glb com buffers e materiais embutidos).
    Glb,
    /// Petunia Package Container (.pkg zip com manifest e anexos).
    Pkg,
}

impl FileFormat {
    pub const ALL: [Self; 4] = [Self::Obj, Self::Gltf, Self::Glb, Self::Pkg];

    pub fn label(self) -> &'static str {
        match self {
            Self::Obj => "Wavefront OBJ",
            Self::Gltf => "glTF 2.0 (JSON)",
            Self::Glb => "glTF 2.0 Binary (GLB)",
            Self::Pkg => "Petunia Package (.pkg)",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Obj => "obj",
            Self::Gltf => "gltf",
            Self::Glb => "glb",
            Self::Pkg => "pkg",
        }
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        let clean = ext.trim_start_matches('.').to_ascii_lowercase();
        match clean.as_str() {
            "obj" => Some(Self::Obj),
            "gltf" => Some(Self::Gltf),
            "glb" => Some(Self::Glb),
            "pkg" => Some(Self::Pkg),
            _ => None,
        }
    }

    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(Self::from_extension)
    }
}

/// Matriz de capacidades declaradas por cada formato suportado (P3D-071, P3D-072).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormatCapabilities {
    /// Suporte a propriedades de material PBR (base color, roughness, metallic, emission).
    pub supports_materials: bool,
    /// Suporte a texturas embutidas no arquivo.
    pub supports_textures: bool,
    /// Suporte a cores por vértice.
    pub supports_vertex_colors: bool,
    /// Suporte a múltiplas malhas/nós no mesmo arquivo.
    pub supports_multi_mesh: bool,
    /// Suporte a formato binário compacto (sem overhead de texto ASCII).
    pub supports_binary: bool,
}

/// Opções de exportação parametrizadas (P3D-068, P3D-072).
#[derive(Clone, Debug, PartialEq)]
pub struct ExportOptions {
    /// Se true, triangula todas as faces poligonais antes da emissão.
    pub triangulate: bool,
    /// Se true, inclui propriedades de materiais PBR suportadas pelo formato.
    pub export_materials: bool,
    /// Fator de escala uniforme aplicado às coordenadas geométricas.
    pub scale: f32,
    /// Política de sobrescrita caso o arquivo de destino já exista.
    pub overwrite: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            triangulate: true,
            export_materials: true,
            scale: 1.0,
            overwrite: true,
        }
    }
}

/// Opções de importação parametrizadas (P3D-071).
#[derive(Clone, Debug, PartialEq)]
pub struct ImportOptions {
    /// Se true, força a triangulação de malhas importadas.
    pub triangulate: bool,
    /// Se true, importa materiais associados quando disponíveis.
    pub import_materials: bool,
    /// Fator de escala uniforme aplicado aos vértices importados.
    pub scale: f32,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            triangulate: true,
            import_materials: true,
            scale: 1.0,
        }
    }
}

/// Relatório de sucesso de uma exportação individual (P3D-068).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportReport {
    pub asset_name: String,
    pub output_path: PathBuf,
    pub format: FileFormat,
    pub bytes_written: usize,
    pub warnings: Vec<String>,
}

/// Relatório agregado de operações de exportação em lote (P3D-069, P3D-070).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BatchExportReport {
    pub succeeded: Vec<ExportReport>,
    pub failed: Vec<(String, String)>,
    pub warnings: Vec<String>,
    pub total_bytes: usize,
}

impl BatchExportReport {
    pub fn all_succeeded(&self) -> bool {
        self.failed.is_empty()
    }
}

/// Dados importados de um arquivo externo protegidos por validação (P3D-071).
#[derive(Clone, Debug, Default)]
pub struct ImportPayload {
    pub meshes: Vec<(String, Mesh)>,
    pub materials: Vec<Material>,
    /// Índice em `materials` para cada malha (dominante), paralelo a `meshes`.
    /// `Face.material_slot` dentro de cada malha usa o mesmo espaço de índice.
    pub mesh_materials: Vec<Option<usize>>,
    pub warnings: Vec<String>,
    /// Esqueletos, pesos de skin e clipes do arquivo (glTF/GLB).
    pub rig: Option<crate::gltf_rig::ImportedRig>,
}

/// Erros estruturados ocorridos no pipeline de importação e exportação.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Formato não suportado: {0}")]
    UnsupportedFormat(String),

    #[error("Asset não encontrado no índice {0}")]
    AssetNotFound(usize),

    #[error("Erro de validação de malha ou projeto: {0}")]
    Validation(String),

    #[error("Erro de exportação: {0}")]
    Export(String),

    #[error("Erro de importação: {0}")]
    Import(String),

    #[error("Arquivo de destino já existe e a política proíbe sobrescrita: {0}")]
    AlreadyExists(PathBuf),
}

/// Trait abstrata para adaptadores de exportação modular (P3D-072).
pub trait FormatExporter: Send + Sync {
    fn format(&self) -> FileFormat;
    fn capabilities(&self) -> FormatCapabilities;
    fn export_asset(
        &self,
        asset: &Asset,
        project: &Project,
        options: &ExportOptions,
    ) -> Result<Vec<u8>, PipelineError>;
    fn export_project(
        &self,
        project: &Project,
        indices: &[usize],
        options: &ExportOptions,
    ) -> Result<Vec<u8>, PipelineError>;

    /// Perdas declaradas antes de exportar: nunca prometer um efeito que o
    /// formato não representa (website/docs/12-interchange, "Política de
    /// conversão e perda").
    fn warnings(&self, options: &ExportOptions) -> Vec<String> {
        let mut out = Vec::new();
        if options.export_materials && !self.capabilities().supports_materials {
            out.push(format!(
                "{}: materials are not exported by this format",
                self.format().label()
            ));
        }
        out
    }
}

/// Maior arquivo aceito por um import (antes de qualquer parse).
pub const MAX_IMPORT_FILE_BYTES: u64 = 256 * 1024 * 1024;

/// Trait abstrata para adaptadores de importação modular (P3D-071).
pub trait FormatImporter: Send + Sync {
    fn format(&self) -> FileFormat;
    fn capabilities(&self) -> FormatCapabilities;
    fn import_bytes(
        &self,
        data: &[u8],
        name_hint: &str,
        options: &ImportOptions,
    ) -> Result<ImportPayload, PipelineError>;
}

// -----------------------------------------------------------------------------
// ADAPTADORES CONCRETOS DE FORMATOS (OBJ, GLB, GLTF, PKG)
// -----------------------------------------------------------------------------

/// Adaptador exportador para Wavefront OBJ.
#[derive(Default)]
pub struct ObjExporter;

impl FormatExporter for ObjExporter {
    fn format(&self) -> FileFormat {
        FileFormat::Obj
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            supports_materials: false,
            supports_textures: false,
            supports_vertex_colors: false,
            supports_multi_mesh: false,
            supports_binary: false,
        }
    }

    fn export_asset(
        &self,
        asset: &Asset,
        project: &Project,
        options: &ExportOptions,
    ) -> Result<Vec<u8>, PipelineError> {
        let pos = project.assets.iter().position(|a| a.id == asset.id);
        let idx = pos.unwrap_or(0);
        let model = crate::export_model::ExportModel::from_project(project, &[idx], options)?;
        Ok(model.to_obj()?.into_bytes())
    }

    fn export_project(
        &self,
        project: &Project,
        indices: &[usize],
        options: &ExportOptions,
    ) -> Result<Vec<u8>, PipelineError> {
        let model = crate::export_model::ExportModel::from_project(project, indices, options)?;
        Ok(model.to_obj()?.into_bytes())
    }
}

/// Adaptador importador para Wavefront OBJ (P3D-071 via tobj).
#[derive(Default)]
pub struct ObjImporter;

impl FormatImporter for ObjImporter {
    fn format(&self) -> FileFormat {
        FileFormat::Obj
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            supports_materials: false,
            supports_textures: false,
            supports_vertex_colors: false,
            supports_multi_mesh: false,
            supports_binary: false,
        }
    }

    fn import_bytes(
        &self,
        data: &[u8],
        name_hint: &str,
        options: &ImportOptions,
    ) -> Result<ImportPayload, PipelineError> {
        let mut mesh = import_obj::import_obj_bytes(data).map_err(|e| match e {
            ObjImportError::Parse(s) => PipelineError::Import(format!("OBJ parse: {s}")),
            ObjImportError::Validation(s) => {
                PipelineError::Validation(format!("OBJ validation: {s}"))
            }
            ObjImportError::TooLarge => {
                PipelineError::Validation("OBJ excede limites máximos de segurança".into())
            }
        })?;

        if (options.scale - 1.0).abs() > 1e-5 && options.scale.is_finite() && options.scale > 0.0 {
            for v in &mut mesh.verts {
                v.pos = [
                    v.pos[0] * options.scale,
                    v.pos[1] * options.scale,
                    v.pos[2] * options.scale,
                ];
            }
        }

        Ok(ImportPayload {
            meshes: vec![(name_hint.to_string(), mesh)],
            materials: Vec::new(),
            mesh_materials: vec![None],
            warnings: Vec::new(),
            rig: None,
        })
    }
}

/// Adaptador exportador para glTF 2.0 Binário (GLB com canais PBR — P3D-050, P3D-072).
#[derive(Default)]
pub struct GlbExporter;

impl FormatExporter for GlbExporter {
    fn format(&self) -> FileFormat {
        FileFormat::Glb
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            supports_materials: true,
            supports_textures: true,
            supports_vertex_colors: true,
            supports_multi_mesh: true,
            supports_binary: true,
        }
    }

    fn export_asset(
        &self,
        asset: &Asset,
        project: &Project,
        options: &ExportOptions,
    ) -> Result<Vec<u8>, PipelineError> {
        let pos = project.assets.iter().position(|a| a.id == asset.id);
        let idx = pos.unwrap_or(0);
        let mut opts = options.clone();
        opts.scale = 1.0; // GLB units stay in meters
        let model = crate::export_model::ExportModel::from_project(project, &[idx], &opts)?;
        model.to_glb(project)
    }

    fn export_project(
        &self,
        project: &Project,
        indices: &[usize],
        options: &ExportOptions,
    ) -> Result<Vec<u8>, PipelineError> {
        let mut opts = options.clone();
        opts.scale = 1.0; // GLB units stay in meters
        let model = crate::export_model::ExportModel::from_project(project, indices, &opts)?;
        model.to_glb(project)
    }

    fn warnings(&self, options: &ExportOptions) -> Vec<String> {
        let mut out = Vec::new();
        if (options.scale - 1.0).abs() > 1e-5 {
            out.push(format!(
                "GLB: scale {} is not applied (units stay in meters)",
                options.scale
            ));
        }
        out
    }
}

/// Adaptador importador para glTF 2.0 Binário (GLB).
#[derive(Default)]
pub struct GlbImporter;

impl FormatImporter for GlbImporter {
    fn format(&self) -> FileFormat {
        FileFormat::Glb
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            supports_materials: true,
            supports_textures: true,
            supports_vertex_colors: true,
            supports_multi_mesh: true,
            supports_binary: true,
        }
    }

    fn import_bytes(
        &self,
        data: &[u8],
        name_hint: &str,
        options: &ImportOptions,
    ) -> Result<ImportPayload, PipelineError> {
        let scene = import_gltf::import_scene_bytes(
            data,
            None,
            name_hint,
            options.triangulate,
            options.scale,
            options.import_materials,
        )
        .map_err(|e| PipelineError::Import(e.to_string()))?;
        let rig = import_gltf::import_rig(data, name_hint, options.scale)
            .map_err(|e| PipelineError::Import(e.to_string()))?;
        let mut warnings = scene.warnings.clone();
        if let Some(r) = rig.as_ref() {
            warnings.extend(r.warnings.iter().cloned());
        }
        Ok(ImportPayload {
            meshes: scene.meshes,
            materials: scene.materials,
            mesh_materials: scene.mesh_materials,
            warnings,
            rig,
        })
    }
}

/// Adaptador importador para glTF 2.0 (validação estrutural de schema — P3D-071).
#[derive(Default)]
pub struct GltfImporter;

impl FormatImporter for GltfImporter {
    fn format(&self) -> FileFormat {
        FileFormat::Gltf
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            supports_materials: true,
            supports_textures: true,
            supports_vertex_colors: true,
            supports_multi_mesh: true,
            supports_binary: false,
        }
    }

    fn import_bytes(
        &self,
        data: &[u8],
        name_hint: &str,
        options: &ImportOptions,
    ) -> Result<ImportPayload, PipelineError> {
        // `.gltf` com buffers/imagens embutidos (`data:` URIs). Arquivos com
        // referências externas usam `DeliveryPipeline::import_file`, que passa
        // o diretório base.
        let scene = import_gltf::import_scene_bytes(
            data,
            None,
            name_hint,
            options.triangulate,
            options.scale,
            options.import_materials,
        )
        .map_err(|e| PipelineError::Import(e.to_string()))?;
        Ok(ImportPayload {
            meshes: scene.meshes,
            materials: scene.materials,
            mesh_materials: scene.mesh_materials,
            warnings: scene.warnings,
            rig: None,
        })
    }
}

/// Adaptador exportador para Petunia Package (.pkg).
#[derive(Default)]
pub struct PkgExporter;

impl FormatExporter for PkgExporter {
    fn format(&self) -> FileFormat {
        FileFormat::Pkg
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            supports_materials: true,
            supports_textures: true,
            supports_vertex_colors: true,
            supports_multi_mesh: true,
            supports_binary: true,
        }
    }

    fn export_asset(
        &self,
        asset: &Asset,
        project: &Project,
        options: &ExportOptions,
    ) -> Result<Vec<u8>, PipelineError> {
        let mut isolated = Project::new();
        isolated.assets = vec![asset.clone()];
        if let Some(mat) = asset.material(project) {
            isolated.materials = vec![mat.clone()];
        }
        self.export_project(&isolated, &[0], options)
    }

    fn export_project(
        &self,
        project: &Project,
        _indices: &[usize],
        _options: &ExportOptions,
    ) -> Result<Vec<u8>, PipelineError> {
        let (_, bytes) = package::save_package_bytes(project, "export", &[])
            .map_err(|e| PipelineError::Export(e.to_string()))?;
        Ok(bytes)
    }
}

/// Adaptador importador para Petunia Package (.pkg).
#[derive(Default)]
pub struct PkgImporter;

impl FormatImporter for PkgImporter {
    fn format(&self) -> FileFormat {
        FileFormat::Pkg
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            supports_materials: true,
            supports_textures: true,
            supports_vertex_colors: true,
            supports_multi_mesh: true,
            supports_binary: true,
        }
    }

    fn import_bytes(
        &self,
        data: &[u8],
        _name_hint: &str,
        _options: &ImportOptions,
    ) -> Result<ImportPayload, PipelineError> {
        let (proj, _, _) =
            package::open_package_bytes(data).map_err(|e| PipelineError::Import(e.to_string()))?;

        let mut meshes = Vec::with_capacity(proj.assets.len());
        let mut mesh_materials = Vec::with_capacity(proj.assets.len());
        for asset in &proj.assets {
            let idx = asset
                .material_id
                .and_then(|id| proj.materials.iter().position(|m| m.id == id));
            meshes.push((asset.name.clone(), asset.mesh.to_owned_value()));
            mesh_materials.push(idx);
        }
        Ok(ImportPayload {
            meshes,
            materials: proj.materials,
            mesh_materials,
            warnings: Vec::new(),
            rig: None,
        })
    }
}

// -----------------------------------------------------------------------------
// DELIVERY PIPELINE REGISTRY & ORCHESTRATOR
// -----------------------------------------------------------------------------

/// Pipeline unificado de entrega e conversão de formatos 3D (P3D-068 a P3D-072, P3D-124).
pub struct DeliveryPipeline {
    exporters: HashMap<FileFormat, Box<dyn FormatExporter>>,
    importers: HashMap<FileFormat, Box<dyn FormatImporter>>,
}

impl Default for DeliveryPipeline {
    fn default() -> Self {
        let mut pipe = Self {
            exporters: HashMap::new(),
            importers: HashMap::new(),
        };

        // Registra adaptadores canônicos padrão
        pipe.register_exporter(Box::new(ObjExporter));
        pipe.register_exporter(Box::new(GlbExporter));
        pipe.register_exporter(Box::new(PkgExporter));

        pipe.register_importer(Box::new(ObjImporter));
        pipe.register_importer(Box::new(GltfImporter));
        pipe.register_importer(Box::new(GlbImporter));
        pipe.register_importer(Box::new(PkgImporter));

        pipe
    }
}

impl DeliveryPipeline {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_exporter(&mut self, exporter: Box<dyn FormatExporter>) {
        self.exporters.insert(exporter.format(), exporter);
    }

    pub fn register_importer(&mut self, importer: Box<dyn FormatImporter>) {
        self.importers.insert(importer.format(), importer);
    }

    pub fn exporter(&self, format: FileFormat) -> Option<&dyn FormatExporter> {
        self.exporters.get(&format).map(|b| b.as_ref())
    }

    pub fn importer(&self, format: FileFormat) -> Option<&dyn FormatImporter> {
        self.importers.get(&format).map(|b| b.as_ref())
    }

    pub fn capabilities(&self, format: FileFormat) -> Option<FormatCapabilities> {
        self.exporters
            .get(&format)
            .map(|e| e.capabilities())
            .or_else(|| self.importers.get(&format).map(|i| i.capabilities()))
    }

    /// Honest capability matrix derived from registered adapters.
    pub fn capability_matrix(&self) -> Vec<(FileFormat, bool, bool, FormatCapabilities)> {
        FileFormat::ALL
            .into_iter()
            .map(|fmt| {
                let import = self.importers.contains_key(&fmt);
                let export = self.exporters.contains_key(&fmt);
                let caps = self
                    .exporters
                    .get(&fmt)
                    .map(|e| e.capabilities())
                    .or_else(|| self.importers.get(&fmt).map(|i| i.capabilities()))
                    .unwrap_or(FormatCapabilities {
                        supports_materials: false,
                        supports_textures: false,
                        supports_vertex_colors: false,
                        supports_multi_mesh: false,
                        supports_binary: false,
                    });
                (fmt, import, export, caps)
            })
            .collect()
    }

    /// P3D-068: Exporta um asset individual com validação, opções e geração de relatório.
    pub fn export_single_asset(
        &self,
        project: &Project,
        asset_idx: usize,
        path: &Path,
        options: &ExportOptions,
    ) -> Result<ExportReport, PipelineError> {
        let asset = project
            .assets
            .get(asset_idx)
            .ok_or(PipelineError::AssetNotFound(asset_idx))?;

        if path.exists() && !options.overwrite {
            return Err(PipelineError::AlreadyExists(path.to_path_buf()));
        }

        let format = FileFormat::from_path(path).ok_or_else(|| {
            PipelineError::UnsupportedFormat(
                path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
            )
        })?;

        let exporter = self
            .exporter(format)
            .ok_or_else(|| PipelineError::UnsupportedFormat(format.label().to_string()))?;

        // Game-Ready Validator: Error bloqueia este alvo; Warning vai ao relatório.
        let diagnostics = crate::game_ready::validate_asset(asset);
        if crate::game_ready::has_errors(&diagnostics) {
            let codes: Vec<String> = diagnostics
                .iter()
                .filter(|d| d.severity == crate::game_ready::Severity::Error)
                .map(|d| format!("{}: {}", d.code, d.message))
                .collect();
            return Err(PipelineError::Validation(codes.join("; ")));
        }
        let mut warnings = exporter.warnings(options);
        warnings.extend(
            diagnostics
                .iter()
                .filter(|d| d.severity == crate::game_ready::Severity::Warning)
                .map(|d| format!("{}: {}", d.code, d.message)),
        );

        let bytes = exporter.export_asset(asset, project, options)?;

        // Escrita atômica: uma falha no meio nunca deixa o destino truncado.
        crate::io_atomic::atomic_write(path, &bytes)
            .map_err(|e| PipelineError::Io(std::io::Error::other(e.to_string())))?;

        Ok(ExportReport {
            asset_name: asset.name.clone(),
            output_path: path.to_path_buf(),
            format,
            bytes_written: bytes.len(),
            warnings,
        })
    }

    /// Exporta a partir de um DocumentSnapshot com isolamento e integridade atômica (I01 / I02).
    pub fn export_snapshot(
        &self,
        snapshot: &crate::snapshot::DocumentSnapshot,
        indices: &[usize],
        path: &Path,
        options: &ExportOptions,
    ) -> Result<ExportReport, PipelineError> {
        if path.exists() && !options.overwrite {
            return Err(PipelineError::AlreadyExists(path.to_path_buf()));
        }

        let format = FileFormat::from_path(path).ok_or_else(|| {
            PipelineError::UnsupportedFormat(
                path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
            )
        })?;

        let exporter = self
            .exporter(format)
            .ok_or_else(|| PipelineError::UnsupportedFormat(format.label().to_string()))?;

        let export_model =
            crate::export_model::ExportModel::from_snapshot(snapshot, indices, options)?;
        let mut warnings = exporter.warnings(options);
        warnings.extend(export_model.warnings.clone());

        let bytes = match format {
            FileFormat::Glb => export_model.to_glb(snapshot.document())?,
            FileFormat::Obj => export_model.to_obj()?.into_bytes(),
            FileFormat::Pkg => {
                let isolated = snapshot.document().clone();
                exporter.export_project(&isolated, indices, options)?
            }
            FileFormat::Gltf => {
                return Err(PipelineError::UnsupportedFormat(
                    "glTF JSON direto não suportado para snapshot; use GLB".into(),
                ));
            }
        };

        crate::io_atomic::atomic_write(path, &bytes)
            .map_err(|e| PipelineError::Io(std::io::Error::other(e.to_string())))?;

        let asset_name = if indices.len() == 1 {
            indices
                .first()
                .and_then(|&i| snapshot.assets().get(i))
                .map(|a| a.name.clone())
                .unwrap_or_else(|| "asset".into())
        } else {
            snapshot.name().to_string()
        };

        Ok(ExportReport {
            asset_name,
            output_path: path.to_path_buf(),
            format,
            bytes_written: bytes.len(),
            warnings,
        })
    }

    /// P3D-069: Exporta múltiplos assets selecionados para um diretório comum.
    pub fn export_multiple_assets(
        &self,
        project: &Project,
        indices: &[usize],
        dir: &Path,
        format: FileFormat,
        options: &ExportOptions,
    ) -> Result<BatchExportReport, PipelineError> {
        std::fs::create_dir_all(dir)?;
        let mut report = BatchExportReport::default();
        // Nomes únicos e determinísticos dentro do batch: dois objetos com o
        // mesmo nome nunca se sobrescrevem (`Cube`, `Cube_2`, …).
        let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();

        for &idx in indices {
            if let Some(asset) = project.assets.get(idx) {
                let sanitized = unique_name(&sanitize_name(&asset.name), &mut used);
                let target = dir.join(format!("{sanitized}.{}", format.extension()));
                match self.export_single_asset(project, idx, &target, options) {
                    Ok(rep) => {
                        report.total_bytes += rep.bytes_written;
                        report.succeeded.push(rep);
                    }
                    Err(e) => {
                        report.failed.push((asset.name.clone(), e.to_string()));
                    }
                }
            } else {
                report
                    .failed
                    .push((format!("index_{idx}"), "Asset não encontrado".into()));
            }
        }

        Ok(report)
    }

    /// P3D-070: Batch Export repetível de todos os assets do projeto para um diretório.
    pub fn batch_export(
        &self,
        project: &Project,
        dir: &Path,
        format: FileFormat,
        options: &ExportOptions,
    ) -> Result<BatchExportReport, PipelineError> {
        let all_indices: Vec<usize> = (0..project.assets.len()).collect();
        self.export_multiple_assets(project, &all_indices, dir, format, options)
    }

    /// P3D-071: Importa uma malha/modelo a partir de um arquivo do disco.
    pub fn import_file(
        &self,
        path: &Path,
        options: &ImportOptions,
    ) -> Result<ImportPayload, PipelineError> {
        let format = FileFormat::from_path(path).ok_or_else(|| {
            PipelineError::UnsupportedFormat(
                path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
            )
        })?;

        let size = std::fs::metadata(path)?.len();
        if size > MAX_IMPORT_FILE_BYTES {
            return Err(PipelineError::Validation(format!(
                "file has {size} bytes (max {MAX_IMPORT_FILE_BYTES})"
            )));
        }
        let data = std::fs::read(path)?;
        let name_hint = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("imported");

        // glTF/GLB resolve buffers e imagens externos relativos ao arquivo.
        if matches!(format, FileFormat::Gltf | FileFormat::Glb) {
            let base = path.parent();
            let scene = import_gltf::import_scene_bytes(
                &data,
                base,
                name_hint,
                options.triangulate,
                options.scale,
                options.import_materials,
            )
            .map_err(|e| match e {
                import_gltf::GltfImportError::TooLarge => {
                    PipelineError::Validation("glTF excede limites máximos de segurança".into())
                }
                other => PipelineError::Import(other.to_string()),
            })?;
            // Rig só existe em GLB autocontido (ou glTF embutido); externo
            // segue sem skin para não exigir rede/disco extra além do base.
            let rig = import_gltf::import_rig(&data, name_hint, options.scale)
                .map_err(|e| PipelineError::Import(e.to_string()))?;
            let mut warnings = scene.warnings;
            if let Some(r) = rig.as_ref() {
                warnings.extend(r.warnings.iter().cloned());
            }
            return Ok(ImportPayload {
                meshes: scene.meshes,
                materials: scene.materials,
                mesh_materials: scene.mesh_materials,
                warnings,
                rig,
            });
        }

        let importer = self
            .importer(format)
            .ok_or_else(|| PipelineError::UnsupportedFormat(format.label().to_string()))?;

        importer.import_bytes(&data, name_hint, options)
    }
}

/// `base`, ou `base_2`, `base_3`… — o primeiro ainda não usado (sem
/// diferenciar maiúsculas, por causa de filesystems case-insensitive).
fn unique_name(base: &str, used: &mut std::collections::HashSet<String>) -> String {
    let base = if base.is_empty() { "object" } else { base };
    let mut candidate = base.to_string();
    let mut n = 2;
    while !used.insert(candidate.to_lowercase()) {
        candidate = format!("{base}_{n}");
        n += 1;
    }
    candidate
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
