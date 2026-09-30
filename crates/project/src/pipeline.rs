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

use crate::export::{self, ExportError};
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
}

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
        _project: &Project,
        options: &ExportOptions,
    ) -> Result<Vec<u8>, PipelineError> {
        let mut mesh = asset.evaluated_mesh();
        if options.triangulate {
            mesh.triangulate();
        }
        if (options.scale - 1.0).abs() > 1e-5 && options.scale.is_finite() && options.scale > 0.0 {
            for v in &mut mesh.verts {
                v.pos = [
                    v.pos[0] * options.scale,
                    v.pos[1] * options.scale,
                    v.pos[2] * options.scale,
                ];
            }
        }
        let text = mesh.to_obj();
        Ok(text.into_bytes())
    }

    fn export_project(
        &self,
        project: &Project,
        indices: &[usize],
        options: &ExportOptions,
    ) -> Result<Vec<u8>, PipelineError> {
        let mut merged = Mesh::default();
        for &idx in indices {
            if let Some(asset) = project.assets.get(idx) {
                let mut m = asset.evaluated_mesh();
                if options.triangulate {
                    m.triangulate();
                }
                merged.join(&m);
            }
        }
        if (options.scale - 1.0).abs() > 1e-5 && options.scale.is_finite() && options.scale > 0.0 {
            for v in &mut merged.verts {
                v.pos = [
                    v.pos[0] * options.scale,
                    v.pos[1] * options.scale,
                    v.pos[2] * options.scale,
                ];
            }
        }
        let text = merged.to_obj();
        Ok(text.into_bytes())
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
        _options: &ExportOptions,
    ) -> Result<Vec<u8>, PipelineError> {
        let pos = project.assets.iter().position(|a| a.id == asset.id);
        let idx = pos.unwrap_or(0);
        export::export_gltf(project, &[idx]).map_err(|e| match e {
            ExportError::Empty => PipelineError::Export("Nada para exportar".into()),
            ExportError::Other(s) => PipelineError::Export(s),
            ExportError::Io(s) => PipelineError::Export(s),
        })
    }

    fn export_project(
        &self,
        project: &Project,
        indices: &[usize],
        _options: &ExportOptions,
    ) -> Result<Vec<u8>, PipelineError> {
        export::export_gltf(project, indices).map_err(|e| match e {
            ExportError::Empty => PipelineError::Export("Nada para exportar".into()),
            ExportError::Other(s) => PipelineError::Export(s),
            ExportError::Io(s) => PipelineError::Export(s),
        })
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
        let meshes =
            import_gltf::import_glb_bytes(data, name_hint, options.triangulate, options.scale)
                .map_err(|e| PipelineError::Import(e.to_string()))?;
        let rig = import_gltf::import_rig(data, name_hint, options.scale)
            .map_err(|e| PipelineError::Import(e.to_string()))?;
        let warnings = rig.as_ref().map_or_else(Vec::new, |r| r.warnings.clone());
        Ok(ImportPayload {
            meshes,
            materials: Vec::new(),
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
        _options: &ImportOptions,
    ) -> Result<ImportPayload, PipelineError> {
        let summary = import_gltf::parse_gltf_json(data)
            .map_err(|e| PipelineError::Import(format!("glTF JSON validation: {e}")))?;

        Ok(ImportPayload {
            meshes: Vec::new(),
            materials: Vec::new(),
            warnings: vec![format!(
                "glTF 2.0 validado ({name_hint}): {} cenas, {} nós, {} malhas",
                summary.scenes, summary.nodes, summary.meshes
            )],
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

        let meshes = proj.assets.into_iter().map(|a| (a.name, a.mesh)).collect();
        Ok(ImportPayload {
            meshes,
            materials: proj.materials,
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

        let bytes = exporter.export_asset(asset, project, options)?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, &bytes)?;

        Ok(ExportReport {
            asset_name: asset.name.clone(),
            output_path: path.to_path_buf(),
            format,
            bytes_written: bytes.len(),
            warnings: Vec::new(),
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

        for &idx in indices {
            if let Some(asset) = project.assets.get(idx) {
                let sanitized = sanitize_name(&asset.name);
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

        let importer = self
            .importer(format)
            .ok_or_else(|| PipelineError::UnsupportedFormat(format.label().to_string()))?;

        let data = std::fs::read(path)?;
        let name_hint = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("imported");

        importer.import_bytes(&data, name_hint, options)
    }
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
