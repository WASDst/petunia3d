//! Snapshot imutável autoral do Documento para serialização, carregamento em staging
//! e isolamento entre Application e Infrastructure (P02 / website/docs/11-project/project-persistence.md).
//!
//! Um [`DocumentSnapshot`] representa o estado estático e consistente de um documento
//! autoral (`Project`), desacoplado de estados de sessão, janelas, câmeras transitórias,
//! histórico ou preferências de UI.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::format::{self, LoadMode, LoadReport, ProjectError};
use crate::{Asset, Project};

/// Snapshot autoral desacoplado do ciclo de vida da interface e da sessão.
///
/// Garante que apenas dados autorais transitem pela camada de I/O, preservando
/// invariantes geométricos, IDs persistentes de assets e recursos, e permitindo
/// staged loads atômicos que nunca corrompem a sessão aberta em caso de falha.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DocumentSnapshot {
    /// Documento autoral encapsulado no snapshot.
    pub document: Project,
}

impl DocumentSnapshot {
    /// Cria um novo snapshot a partir de um documento.
    pub fn new(document: Project) -> Self {
        Self { document }
    }

    /// Captura um snapshot imutável do documento fornecido.
    pub fn from_document(document: &Project) -> Self {
        Self {
            document: document.clone(),
        }
    }

    /// Consome o snapshot e retorna o documento autoral.
    pub fn into_document(self) -> Project {
        self.document
    }

    /// Retorna uma referência ao documento autoral contido no snapshot.
    pub fn document(&self) -> &Project {
        &self.document
    }

    /// Retorna uma referência mutável ao documento autoral.
    pub fn document_mut(&mut self) -> &mut Project {
        &mut self.document
    }

    /// Retorna o identificador persistente do projeto.
    pub fn project_id(&self) -> uuid::Uuid {
        self.document.id
    }

    /// Retorna o nome autoral do projeto.
    pub fn name(&self) -> &str {
        &self.document.name
    }

    /// Retorna a lista de assets presentes no snapshot.
    pub fn assets(&self) -> &[Asset] {
        &self.document.assets
    }

    /// Valida os invariantes geométricos de todas as malhas do documento autoral.
    ///
    /// Se qualquer malha contiver índices de vértice fora dos limites, polígonos
    /// degenerados ou coordenadas não-finitas (`NaN`/`Inf`), retorna um erro
    /// explícito rejeitando o snapshot.
    pub fn validate_invariants(&self) -> Result<(), ProjectError> {
        for asset in &self.document.assets {
            if let Err(e) = petunia_mesh::validate_invariants(&asset.mesh) {
                return Err(ProjectError::RepairRequired(format!(
                    "malha '{}' ({}) com invariantes violados: {e}",
                    asset.name, asset.id
                )));
            }
        }
        Ok(())
    }

    /// Codifica o snapshot no container ZIP canônico V1 (.petunia).
    pub fn encode_zip(&self) -> Result<Vec<u8>, ProjectError> {
        format::encode_zip(&self.document)
    }

    /// Decodifica um snapshot a partir de um buffer de bytes (staged load).
    ///
    /// Aplica as políticas de limites e reparo sem mutar qualquer estado global.
    pub fn decode_bytes(bytes: &[u8], mode: LoadMode) -> Result<(Self, LoadReport), ProjectError> {
        let (document, report) = format::load_bytes_with_report(bytes, mode)?;
        Ok((Self { document }, report))
    }

    /// Salva o snapshot de forma atômica no caminho especificado via escrita em arquivo
    /// temporário no mesmo diretório seguida de substituição atômica.
    pub fn save_atomic(&self, path: &Path) -> Result<(), ProjectError> {
        format::save_atomic(&self.document, path)
    }

    /// Carrega um snapshot de forma atômica a partir do caminho de arquivo em disco
    /// aplicando o modo de carregamento especificado (Strict vs LegacyRepair).
    pub fn load_atomic(path: &Path, mode: LoadMode) -> Result<(Self, LoadReport), ProjectError> {
        let (document, report) = format::load_with_report(path, mode)?;
        Ok((Self { document }, report))
    }
}

impl From<Project> for DocumentSnapshot {
    fn from(document: Project) -> Self {
        Self::new(document)
    }
}

impl From<DocumentSnapshot> for Project {
    fn from(snapshot: DocumentSnapshot) -> Self {
        snapshot.into_document()
    }
}
