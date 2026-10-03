//! Serviço canônico de ciclo de vida, persistência e I/O de projetos do Petunia3D.
//!
//! Isola completamente operações com disco, parsing de malhas e serialização
//! do ecossistema de interface de usuário (egui/rfd), permitindo execução pura
//! e headless em testes, CLI ou frontends alternativos.

use std::path::Path;

use petunia_mesh::Mesh;
use petunia_project::ProjectChanges;
use petunia_project::{
    BatchExportReport, DeliveryPipeline, ExportOptions, ExportReport, FileFormat, ImportOptions,
    PipelineError, export, format, palette,
};

use crate::state::AppState;
use crate::{AppEvent, RefAxis, ReferenceImage};

/// Erros estruturados ocorridos durante operações de I/O de projetos e assets.
#[derive(Debug, thiserror::Error)]
pub enum ProjectServiceError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Format error: {0}")]
    Format(String),

    #[error("Asset not found at index {0}")]
    AssetNotFound(usize),

    #[error("Export error: {0}")]
    Export(String),

    #[error("Pipeline error: {0}")]
    Pipeline(PipelineError),

    #[error("No valid colors found in palette file")]
    InvalidPalette,
}

impl From<PipelineError> for ProjectServiceError {
    fn from(err: PipelineError) -> Self {
        match err {
            PipelineError::AssetNotFound(idx) => ProjectServiceError::AssetNotFound(idx),
            PipelineError::Io(e) => ProjectServiceError::Io(e),
            other => ProjectServiceError::Pipeline(other),
        }
    }
}

/// Serviço puro de aplicação para carregamento, salvamento, importação e exportação de projetos e assets.
pub struct ProjectService;

impl ProjectService {
    /// Reinicia a sessão para um projeto vazio padrão (P3D-001 §New Project).
    pub fn new_project(state: &mut AppState) {
        state.project.reset();
        state.session.tools.uv_selected.clear();
        state.sync_selection();
        state.mark_document_clean();
        state.set_status("new project".to_string());
        state.events.emit(AppEvent::ProjectLoaded);
        state.mark_dirty();
    }

    /// Carrega um arquivo de projeto (.petunia) e sincroniza o estado da aplicação (P3D-001 §Open Project).
    pub fn load_project(state: &mut AppState, path: &Path) -> Result<(), ProjectServiceError> {
        let p = format::load(path).map_err(|e| ProjectServiceError::Format(e.to_string()))?;
        state.project.palette = p.palette.clone();
        state.project.project = p;
        // Arquivos carregados começam com revisões em 0; avançar evita o hash
        // de conteúdo por frame do fingerprint (ver `ProjectState::new`).
        state.project.project.bump_changes(ProjectChanges::ALL);
        state.project.undo.clear();
        state.session.tools.uv_selected.clear();
        state.project.project_path = Some(path.to_string_lossy().to_string());
        state.mark_document_clean();
        state.events.emit(AppEvent::ProjectLoaded);
        state.sync_selection();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled".to_string());
        state.project.recent_projects.add(path, &name, now);
        let _ = state.project.recent_projects.save();
        state.set_status(format!("open {}", path.display()));
        state.mark_dirty();
        Ok(())
    }

    /// Salva o estado atual do projeto no arquivo especificado (.petunia) usando escrita atômica segura (P3D-001 §Save).
    pub fn save_project(state: &mut AppState, path: &Path) -> Result<(), ProjectServiceError> {
        state.events.emit(AppEvent::ProjectSaving);
        state.project.project.palette = state.project.palette.clone();
        format::save(&state.project.project, path)
            .map_err(|e| ProjectServiceError::Format(e.to_string()))?;
        state.project.project_path = Some(path.to_string_lossy().to_string());
        state.mark_document_clean();
        state.events.emit(AppEvent::ProjectSaved);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled".to_string());
        state.project.recent_projects.add(path, &name, now);
        let _ = state.project.recent_projects.save();
        state.set_status(format!("saved {}", path.display()));
        state.mark_dirty();
        Ok(())
    }

    /// Salva o projeto em um novo destino de arquivo e atualiza o caminho ativo (P3D-001 §Save As).
    pub fn save_as_project(
        state: &mut AppState,
        new_path: &Path,
    ) -> Result<(), ProjectServiceError> {
        Self::save_project(state, new_path)
    }

    /// Recupera projeto a partir de snapshot de autosave preservando dirty state e sem sobrescrever arquivo principal (P3D-002 §Recovery).
    pub fn recover_from_snapshot(
        state: &mut AppState,
        snapshot_path: &Path,
        original_path: Option<&Path>,
    ) -> Result<(), ProjectServiceError> {
        let p =
            format::load(snapshot_path).map_err(|e| ProjectServiceError::Format(e.to_string()))?;
        state.project.palette = p.palette.clone();
        state.project.project = p;
        // Arquivos carregados começam com revisões em 0; avançar evita o hash
        // de conteúdo por frame do fingerprint (ver `ProjectState::new`).
        state.project.project.bump_changes(ProjectChanges::ALL);
        state.project.undo.clear();
        state.session.tools.uv_selected.clear();
        state.project.project_path = original_path.map(|p| p.to_string_lossy().to_string());
        // Ao recuperar, o documento entra como modificado/dirty (P3D-002)
        state.mark_document_dirty();
        state.events.emit(AppEvent::ProjectLoaded);
        state.sync_selection();
        state.set_status("recovered from autosave snapshot".to_string());
        state.mark_dirty();
        Ok(())
    }

    /// Importa uma malha Wavefront OBJ do disco e a anexa como novo asset do projeto.
    pub fn import_obj(state: &mut AppState, path: &Path) -> Result<String, ProjectServiceError> {
        let text = std::fs::read_to_string(path)?;
        let mesh = Mesh::from_obj(&text);
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("mesh")
            .to_string();

        state.checkpoint("import obj");
        state.project.add(&name, mesh);
        state.sync_selection();
        state.emit_mesh_changed();
        state.set_status(format!("import {}", path.display()));
        state.mark_dirty();
        Ok(name)
    }

    /// Exporta um asset do projeto para um arquivo Wavefront OBJ via pipeline unificado.
    pub fn export_obj(
        state: &AppState,
        asset_idx: usize,
        path: &Path,
    ) -> Result<(), ProjectServiceError> {
        let pipeline = DeliveryPipeline::new();
        let options = ExportOptions {
            triangulate: false,
            export_materials: false,
            scale: 1.0,
            overwrite: true,
        };
        pipeline.export_single_asset(&state.project, asset_idx, path, &options)?;
        Ok(())
    }

    /// Exporta múltiplos assets para arquivos OBJ individuais dentro de um diretório via pipeline.
    pub fn export_all_obj_to_dir(
        state: &AppState,
        asset_indices: &[usize],
        dir: &Path,
    ) -> Result<usize, ProjectServiceError> {
        let pipeline = DeliveryPipeline::new();
        let options = ExportOptions {
            triangulate: false,
            export_materials: false,
            scale: 1.0,
            overwrite: true,
        };
        let report = pipeline.export_multiple_assets(
            &state.project,
            asset_indices,
            dir,
            FileFormat::Obj,
            &options,
        )?;
        Ok(report.succeeded.len())
    }

    /// Exporta os assets indicados para um arquivo glTF Binário (.glb).
    pub fn export_glb(
        state: &AppState,
        asset_indices: &[usize],
        path: &Path,
    ) -> Result<(), ProjectServiceError> {
        let bytes = export::export_gltf(&state.project, asset_indices)
            .map_err(|e| ProjectServiceError::Export(e.to_string()))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, bytes)?;
        Ok(())
    }

    pub fn export_glb_with_padding(
        state: &AppState,
        asset_indices: &[usize],
        path: &Path,
        padding: u32,
    ) -> Result<(), ProjectServiceError> {
        let bytes = export::export_gltf_with_padding(&state.project, asset_indices, padding)
            .map_err(|e| ProjectServiceError::Export(e.to_string()))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, bytes)?;
        Ok(())
    }

    /// Exporta um asset individual usando o pipeline unificado com validação e opções parametrizadas (P3D-068).
    pub fn export_asset_pipeline(
        state: &AppState,
        asset_idx: usize,
        path: &Path,
        options: &ExportOptions,
    ) -> Result<ExportReport, ProjectServiceError> {
        let pipeline = DeliveryPipeline::new();
        let report = pipeline.export_single_asset(&state.project, asset_idx, path, options)?;
        Ok(report)
    }

    /// Exporta múltiplos assets selecionados para um diretório comum (P3D-069).
    pub fn export_multiple_pipeline(
        state: &AppState,
        indices: &[usize],
        dir: &Path,
        format: FileFormat,
        options: &ExportOptions,
    ) -> Result<BatchExportReport, ProjectServiceError> {
        let pipeline = DeliveryPipeline::new();
        let report =
            pipeline.export_multiple_assets(&state.project, indices, dir, format, options)?;
        Ok(report)
    }

    /// Batch export determinístico de todos os assets do projeto para um diretório (P3D-070).
    pub fn batch_export_pipeline(
        state: &AppState,
        dir: &Path,
        format: FileFormat,
        options: &ExportOptions,
    ) -> Result<BatchExportReport, ProjectServiceError> {
        let pipeline = DeliveryPipeline::new();
        let report = pipeline.batch_export(&state.project, dir, format, options)?;
        Ok(report)
    }

    /// Importa malhas e materiais de um arquivo suportado (OBJ, glTF, PKG) via pipeline modular (P3D-071).
    pub fn import_file_pipeline(
        state: &mut AppState,
        path: &Path,
        options: &ImportOptions,
    ) -> Result<Vec<String>, ProjectServiceError> {
        let pipeline = DeliveryPipeline::new();
        let payload = pipeline.import_file(path, options)?;

        let mut imported_names = Vec::new();
        if !payload.meshes.is_empty() {
            state.checkpoint("import via pipeline");
            let first_new_asset = state.project.assets.len();
            // Materiais primeiro: preserva IDs e permite vincular assets.
            let base_mat = state.project.project.materials.len();
            let mut mat_ids: Vec<uuid::Uuid> = Vec::with_capacity(payload.materials.len());
            for mat in payload.materials {
                let id = mat.id;
                state.project.add_material(mat);
                mat_ids.push(id);
            }
            for (mi, (name, mut mesh)) in payload.meshes.into_iter().enumerate() {
                // Remapeia slots locais (índice em payload.materials) para
                // índices do projeto (base + local).
                for face in &mut mesh.faces {
                    if let Some(slot) = face.material_slot
                        && slot < mat_ids.len()
                    {
                        face.material_slot = Some(base_mat + slot);
                    } else if face.material_slot.is_some() {
                        face.material_slot = None;
                    }
                }
                let dominant = payload
                    .mesh_materials
                    .get(mi)
                    .copied()
                    .flatten()
                    .filter(|&i| i < mat_ids.len());
                // Dados do material antes do borrow mutável do asset.
                let linked: Option<(uuid::Uuid, Option<petunia_project::Canvas>, [f32; 3])> =
                    dominant.map(|local| {
                        let id = mat_ids[local];
                        let found = state.project.project.get_material(id);
                        let canvas = found.and_then(|m| m.albedo_texture.clone());
                        let base = found.map_or([0.75, 0.75, 0.78], |m| {
                            [m.base_color[0], m.base_color[1], m.base_color[2]]
                        });
                        (id, canvas, base)
                    });
                state.project.add(&name, mesh);
                if let Some((id, canvas, base)) = linked
                    && let Some(asset) = state.project.assets.last_mut()
                {
                    asset.material_id = Some(id);
                    // Espelha albedo no asset para viewports legados que
                    // leem `Asset.texture` (export também aceita ambos).
                    if let Some(canvas) = canvas {
                        asset.texture = Some(canvas);
                    }
                    asset.base_color = base;
                }
                imported_names.push(name);
            }
            let mut warnings = payload.warnings;
            if let Some(rig) = payload.rig {
                warnings.extend(state.project.add_imported_rig(rig, first_new_asset));
            }
            state.sync_selection();
            state.emit_mesh_changed();
            let note = if warnings.is_empty() {
                String::new()
            } else {
                format!(" ({} warning(s): {})", warnings.len(), warnings[0])
            };
            state.set_status(format!(
                "imported {} assets from {}{note}",
                imported_names.len(),
                path.display()
            ));
            state.mark_dirty();
        }
        Ok(imported_names)
    }

    /// Importa paleta de cores a partir de um arquivo .hex ou .gpl.
    pub fn import_palette(state: &mut AppState, path: &Path) -> Result<usize, ProjectServiceError> {
        let text = std::fs::read_to_string(path)?;
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let colors = if ext.eq_ignore_ascii_case("gpl") {
            palette::import_gpl(&text)
        } else {
            palette::import_hex(&text)
        };

        if colors.is_empty() {
            return Err(ProjectServiceError::InvalidPalette);
        }

        let count = colors.len();
        state.project.palette = colors.clone();
        state.project.project.palette = colors;
        state.set_status(format!("imported {count} colors"));
        state.mark_dirty();
        Ok(count)
    }

    /// Exporta uma paleta de cores no formato GIMP Palette (.gpl).
    pub fn export_palette(
        palette: &[[f32; 3]],
        title: &str,
        path: &Path,
    ) -> Result<(), ProjectServiceError> {
        let gpl = palette::export_gpl(title, palette);
        std::fs::write(path, gpl)?;
        Ok(())
    }

    /// Adiciona uma imagem de referência à cena 3D.
    pub fn add_reference_image(
        state: &mut AppState,
        name: String,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    ) {
        state
            .project
            .refs
            .push(ReferenceImage::from_rgba(name.clone(), width, height, rgba));
        state.set_status(format!("ref {name}"));
        state.mark_dirty();
    }

    /// Define ou substitui a imagem de referência associada a um determinado slot ortográfico (P3D-013).
    pub fn set_reference_slot(
        state: &mut AppState,
        axis: RefAxis,
        name: String,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    ) {
        if let Some(existing) = state.project.refs.iter_mut().find(|r| r.axis == axis) {
            existing.name = name.clone();
            existing.width = width;
            existing.height = height;
            existing.rgba = rgba;
            existing.bump_revision();
        } else {
            let mut img = ReferenceImage::from_rgba(name.clone(), width, height, rgba);
            img.axis = axis;
            state.project.refs.push(img);
        }
        state.set_status(format!("Slot {axis:?} atualizado com '{name}'"));
        state.mark_dirty();
    }

    /// Remove uma imagem de referência pelo índice.
    pub fn remove_reference(state: &mut AppState, index: usize) -> bool {
        if index < state.project.refs.len() {
            let r = state.project.refs.remove(index);
            state.set_status(format!("Referência '{}' removida", r.name));
            state.mark_dirty();
            true
        } else {
            false
        }
    }

    /// Remove todas as imagens de referência da cena.
    pub fn clear_references(state: &mut AppState) {
        state.project.refs.clear();
        state.set_status("Todas as referências foram removidas");
        state.mark_dirty();
    }
}

/// Sanitiza strings para uso seguro como nomes de arquivo.
pub fn sanitize_filename(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "asset".to_string()
    } else {
        sanitized
    }
}
