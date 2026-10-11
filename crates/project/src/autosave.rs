//! Serviço puro de Autosave, Session Lock e Detecção de Recuperação (P3D-002).
//!
//! Contrato canônico:
//! 1. `Save explícito` -> projeto oficial (`project.petunia`);
//! 2. `Autosave` -> snapshots rotativos de recuperação (`.petunia/autosave/autosave-*.petunia`);
//! 3. Autosave **nunca** sobrescreve o arquivo oficial;
//! 4. Autosave **nunca** limpa o `dirty state` do documento;
//! 5. Retenção de $N$ snapshots (descarta o mais antigo ao exceder limite);
//! 6. Marcador de sessão (`session.lock`) para detecção determinística de unclean shutdown / crash;
//! 7. Recuperação segura: o usuário escolhe Recover, Inspect ou Discard por geração
//!    ([`AutosaveService::list_recovery_generations`]), sem sobrescrita silenciosa;
//!    documentos sem arquivo têm diretório por `project_id`, e cada snapshot leva
//!    a revisão no nome;
//! 8. 100% puro e desacoplado do egui (acionado por timestamps/relógio).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::Project;
use crate::format::{self, ProjectError};

/// Configurações do sistema de Autosave e Recuperação (P3D-002 §Configurações).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutosaveConfig {
    /// Habilita ou desabilita o autosave automático.
    pub enabled: bool,
    /// Intervalo entre snapshots em segundos (ex: 60s, 120s, 300s).
    pub interval_secs: u64,
    /// Quantidade máxima de snapshots retidos por projeto.
    pub keep_n: usize,
    /// Executa snapshot apenas quando o documento possuir modificações não salvas (dirty).
    pub only_when_dirty: bool,
}

impl Default for AutosaveConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: 120, // 2 minutos
            keep_n: 5,
            only_when_dirty: true,
        }
    }
}

/// Metadados gravados no marcador de sessão (`session.lock`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionLockInfo {
    pub pid: u32,
    pub start_time: u64,
    pub project_name: String,
    pub project_path: Option<String>,
}

/// Informações diagnósticas para apresentação da decisão de recuperação (Recovery Dialog).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryInfo {
    pub snapshot_path: PathBuf,
    pub project_name: String,
    pub snapshot_time: u64,
    pub main_project_path: Option<PathBuf>,
    pub is_newer_than_main: bool,
}

/// Uma geração de recuperação no disco (website/docs/11-project/project-persistence.md,
/// "Autosave e Crash Recovery"): o que o diálogo Recover/Inspect/Discard mostra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryCandidate {
    pub snapshot_path: PathBuf,
    /// Segundos Unix gravados no nome do snapshot.
    pub timestamp: u64,
    pub seq: u64,
    /// Revisão do documento no snapshot; `None` em snapshots de versões antigas.
    pub revision: Option<u64>,
    /// Projeto salvo de origem; `None` para documento sem arquivo.
    pub project_path: Option<PathBuf>,
}

/// Serviço de autosave e retenção de snapshots.
#[derive(Debug, Clone)]
pub struct AutosaveService {
    pub config: AutosaveConfig,
    pub last_autosave_time: u64,
    /// Revisão do documento no último snapshot gravado com sucesso: ciclos sem
    /// mudança nova são coalescidos mesmo que o documento continue dirty.
    last_snapshot_revision: Option<u64>,
    seq: u64,
}

/// Mecanismo de persistência do autosave (infraestrutura): *como e onde* gravar.
/// A política (`AutosaveService::decide`) só decide *quando*.
pub trait AutosaveStorage {
    fn write_snapshot(
        &self,
        project: &Project,
        project_path: Option<&Path>,
        timestamp: u64,
        seq: u64,
        revision: u64,
        keep_n: usize,
    ) -> Result<PathBuf, ProjectError>;
}

/// Armazenamento em disco: arquivos rotativos com escrita atômica.
#[derive(Debug, Clone, Copy, Default)]
pub struct FileAutosaveStorage;

impl AutosaveStorage for FileAutosaveStorage {
    fn write_snapshot(
        &self,
        project: &Project,
        project_path: Option<&Path>,
        timestamp: u64,
        seq: u64,
        revision: u64,
        keep_n: usize,
    ) -> Result<PathBuf, ProjectError> {
        AutosaveService::perform_snapshot(project, project_path, timestamp, seq, revision, keep_n)
    }
}

impl Default for AutosaveService {
    fn default() -> Self {
        Self::new(AutosaveConfig::default())
    }
}

impl AutosaveService {
    pub fn new(config: AutosaveConfig) -> Self {
        Self {
            config,
            last_autosave_time: 0,
            last_snapshot_revision: None,
            seq: 0,
        }
    }

    /// Determina o diretório canônico de autosave para o projeto atual.
    pub fn autosave_dir(project_path: Option<&Path>) -> PathBuf {
        if let Some(path) = project_path {
            let parent = path.parent().unwrap_or_else(|| Path::new("."));
            parent.join(".petunia").join("autosave")
        } else {
            std::env::temp_dir()
                .join("petunia3d")
                .join("untitled_project")
                .join(".petunia")
                .join("autosave")
        }
    }

    /// Diretório de autosave de um documento: ao lado do arquivo salvo, ou, sem
    /// arquivo, um diretório temporário **por `project_id`** — dois documentos
    /// sem nome nunca podam as gerações um do outro.
    pub fn autosave_dir_for(project_path: Option<&Path>, project_id: uuid::Uuid) -> PathBuf {
        match project_path {
            Some(_) => Self::autosave_dir(project_path),
            None => Self::untitled_root()
                .join(format!("untitled-{project_id}"))
                .join(".petunia")
                .join("autosave"),
        }
    }

    fn untitled_root() -> PathBuf {
        std::env::temp_dir().join("petunia3d")
    }

    /// Diretórios onde procurar gerações: o do arquivo salvo, ou — sem arquivo —
    /// o diretório legado `untitled_project` e todos os `untitled-<id>`.
    fn recovery_dirs(project_path: Option<&Path>) -> Vec<PathBuf> {
        let mut dirs = vec![Self::autosave_dir(project_path)];
        if project_path.is_none()
            && let Ok(rd) = fs::read_dir(Self::untitled_root())
        {
            let mut per_id: Vec<PathBuf> = rd
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with("untitled-"))
                })
                .map(|p| p.join(".petunia").join("autosave"))
                .collect();
            per_id.sort();
            dirs.extend(per_id);
        }
        dirs
    }

    /// Lê `autosave-<timestamp>-<seq>[-r<revision>].petunia`.
    fn parse_snapshot_name(name: &str) -> Option<(u64, u64, Option<u64>)> {
        let stem = name.strip_prefix("autosave-")?.strip_suffix(".petunia")?;
        let mut parts = stem.split('-');
        let timestamp = parts.next()?.parse().ok()?;
        let seq = parts.next()?.parse().ok()?;
        let revision = match parts.next() {
            Some(r) => Some(r.strip_prefix('r')?.parse().ok()?),
            None => None,
        };
        parts.next().is_none().then_some((timestamp, seq, revision))
    }

    /// Todas as gerações de recuperação disponíveis, da mais nova para a mais
    /// antiga (empate: ordem de caminho). Nada é lido além de nomes de arquivo.
    pub fn list_recovery_generations(project_path: Option<&Path>) -> Vec<RecoveryCandidate> {
        let mut out = Vec::new();
        for dir in Self::recovery_dirs(project_path) {
            let Ok(rd) = fs::read_dir(&dir) else {
                continue;
            };
            for entry in rd.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let Some((timestamp, seq, revision)) = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .and_then(Self::parse_snapshot_name)
                else {
                    continue;
                };
                out.push(RecoveryCandidate {
                    snapshot_path: path,
                    timestamp,
                    seq,
                    revision,
                    project_path: project_path.map(Path::to_path_buf),
                });
            }
        }
        out.sort_by(|a, b| {
            (b.timestamp, b.seq)
                .cmp(&(a.timestamp, a.seq))
                .then_with(|| a.snapshot_path.cmp(&b.snapshot_path))
        });
        out
    }

    /// Discard de uma geração específica (as demais continuam recuperáveis).
    pub fn discard_generation(candidate: &RecoveryCandidate) -> Result<(), std::io::Error> {
        fs::remove_file(&candidate.snapshot_path)
    }

    /// Determina o caminho do marcador de sessão ativa (`session.lock`).
    pub fn session_lock_path(project_path: Option<&Path>) -> PathBuf {
        if let Some(path) = project_path {
            let parent = path.parent().unwrap_or_else(|| Path::new("."));
            parent.join(".petunia").join("session.lock")
        } else {
            std::env::temp_dir()
                .join("petunia3d")
                .join("untitled_project")
                .join(".petunia")
                .join("session.lock")
        }
    }

    /// Grava marcador de sessão ativa indicando que o projeto está aberto.
    pub fn create_session_lock(
        project_path: Option<&Path>,
        project_name: &str,
        start_time: u64,
    ) -> Result<PathBuf, std::io::Error> {
        let lock_path = Self::session_lock_path(project_path);
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let info = SessionLockInfo {
            pid: std::process::id(),
            start_time,
            project_name: project_name.to_string(),
            project_path: project_path.map(|p| p.to_string_lossy().to_string()),
        };
        let bytes = postcard::to_allocvec(&info)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        fs::write(&lock_path, bytes)?;
        Ok(lock_path)
    }

    /// Remove o marcador de sessão ativa em encerramento normal e limpo (clean shutdown).
    pub fn remove_session_lock(project_path: Option<&Path>) {
        let lock_path = Self::session_lock_path(project_path);
        let _ = fs::remove_file(lock_path);
    }

    /// Política pura: devolve o número de sequência do snapshot quando é hora de
    /// gravar, ou `None`. Não toca o filesystem nem o relógio (o tempo é injetado).
    ///
    /// Coalesce por revisão: sem mudança desde o último snapshot bem-sucedido não há
    /// o que gravar de novo.
    pub fn decide(&mut self, now_secs: u64, is_dirty: bool, revision: u64) -> Option<u64> {
        if !self.config.enabled {
            return None;
        }
        if self.config.only_when_dirty && !is_dirty {
            return None;
        }
        if self.last_snapshot_revision == Some(revision) {
            return None;
        }
        if now_secs
            < self
                .last_autosave_time
                .saturating_add(self.config.interval_secs)
        {
            return None;
        }
        self.last_autosave_time = now_secs;
        self.seq = self.seq.wrapping_add(1);
        Some(self.seq)
    }

    /// Passo completo com armazenamento injetado: política + gravação.
    /// Retorna `Some(Ok(path))` se o snapshot foi criado com sucesso.
    pub fn tick_with(
        &mut self,
        storage: &dyn AutosaveStorage,
        now_secs: u64,
        is_dirty: bool,
        revision: u64,
        project: &Project,
        project_path: Option<&Path>,
    ) -> Option<Result<PathBuf, ProjectError>> {
        let seq = self.decide(now_secs, is_dirty, revision)?;
        let result = storage.write_snapshot(
            project,
            project_path,
            now_secs,
            seq,
            revision,
            self.config.keep_n,
        );
        if result.is_ok() {
            self.last_snapshot_revision = Some(revision);
        }
        Some(result)
    }

    /// `tick_with` sobre o armazenamento em disco padrão.
    ///
    /// Serializa a partir do projeto emprestado; quem já tem um snapshot imutável
    /// versionado deve passá-lo no lugar do documento vivo.
    pub fn tick(
        &mut self,
        current_time_secs: u64,
        is_dirty: bool,
        revision: u64,
        project: &Project,
        project_path: Option<&Path>,
    ) -> Option<Result<PathBuf, ProjectError>> {
        self.tick_with(
            &FileAutosaveStorage,
            current_time_secs,
            is_dirty,
            revision,
            project,
            project_path,
        )
    }

    /// Captures an immutable snapshot suitable for a worker thread.
    /// At most one outstanding autosave should run per document (caller coalesces).
    pub fn capture_snapshot(project: &Project) -> Project {
        project.clone()
    }

    /// Executa gravação atômica do snapshot de recuperação e poda os mais antigos.
    pub fn perform_snapshot(
        project: &Project,
        project_path: Option<&Path>,
        timestamp: u64,
        seq: u64,
        revision: u64,
        keep_n: usize,
    ) -> Result<PathBuf, ProjectError> {
        let dir = Self::autosave_dir_for(project_path, project.id);
        fs::create_dir_all(&dir).map_err(|e| ProjectError::Io(e.to_string()))?;

        let filename = format!("autosave-{timestamp:010}-{seq:04}-r{revision}.petunia");
        let target_path = dir.join(filename);

        // Grava de forma atômica
        format::save_atomic(project, &target_path)?;

        // Aplica política de retenção (keep_n)
        let _ = Self::prune_retention(&dir, keep_n);

        Ok(target_path)
    }

    /// Remove snapshots mais antigos que excederem o limite `keep_n`.
    pub fn prune_retention(dir: &Path, keep_n: usize) -> Result<usize, std::io::Error> {
        if keep_n == 0 {
            return Ok(0);
        }

        let mut entries = Vec::new();
        if !dir.exists() {
            return Ok(0);
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file()
                && let Some(name) = path.file_name().and_then(|n| n.to_str())
                && name.starts_with("autosave-")
                && name.ends_with(".petunia")
            {
                entries.push(path);
            }
        }

        // Ordena por nome (que inclui timestamp zero-padded e sequence)
        entries.sort();

        let mut removed = 0;
        if entries.len() > keep_n {
            let excess = entries.len() - keep_n;
            for path in entries.iter().take(excess) {
                if fs::remove_file(path).is_ok() {
                    removed += 1;
                }
            }
        }

        Ok(removed)
    }

    /// Detecta se a sessão anterior encerrou de forma inesperada (unclean shutdown)
    /// e se existem snapshots válidos para recuperação.
    pub fn detect_recovery(project_path: Option<&Path>) -> Option<RecoveryInfo> {
        let lock_path = Self::session_lock_path(project_path);
        if !lock_path.exists() {
            // Encerramento limpo anterior ou primeiro início
            return None;
        }

        // Lê os metadados do lock
        let lock_info: Option<SessionLockInfo> = fs::read(&lock_path)
            .ok()
            .and_then(|b| postcard::from_bytes(&b).ok());

        let project_name = lock_info
            .as_ref()
            .map(|i| i.project_name.clone())
            .unwrap_or_else(|| "Untitled".to_string());

        let newest_snapshot = Self::list_recovery_generations(project_path)
            .into_iter()
            .next()?
            .snapshot_path;

        // Determina se o snapshot é mais recente que o arquivo principal (quando existir)
        let mut is_newer = true;
        let mut main_path_buf = None;

        if let Some(p) = project_path {
            main_path_buf = Some(p.to_path_buf());
            if p.exists()
                && let (Ok(snap_meta), Ok(main_meta)) =
                    (fs::metadata(&newest_snapshot), fs::metadata(p))
                && let (Ok(snap_time), Ok(main_time)) = (snap_meta.modified(), main_meta.modified())
            {
                is_newer = snap_time > main_time;
            }
        }

        let snap_time = fs::metadata(&newest_snapshot)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Some(RecoveryInfo {
            snapshot_path: newest_snapshot,
            project_name,
            snapshot_time: snap_time,
            main_project_path: main_path_buf,
            is_newer_than_main: is_newer,
        })
    }

    /// Descarta todos os snapshots de recuperação e remove o marcador de sessão.
    pub fn discard_recovery(project_path: Option<&Path>) -> Result<(), std::io::Error> {
        for dir in Self::recovery_dirs(project_path) {
            if dir.exists() {
                let _ = fs::remove_dir_all(&dir);
            }
        }
        Self::remove_session_lock(project_path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use petunia_mesh::Mesh;

    #[test]
    fn test_autosave_tick_only_triggers_when_dirty() {
        let mut service = AutosaveService::new(AutosaveConfig {
            enabled: true,
            interval_secs: 10,
            keep_n: 3,
            only_when_dirty: true,
        });

        let p = Project::new();
        let dir = std::env::temp_dir().join(format!("petunia_as_test_{}", uuid::Uuid::new_v4()));
        let proj_path = dir.join("proj.petunia");

        // Caso 1: Não dirty -> não deve disparar
        let res = service.tick(100, false, 1, &p, Some(&proj_path));
        assert!(res.is_none());

        // Caso 2: Dirty -> dispara snapshot
        let res = service.tick(100, true, 1, &p, Some(&proj_path));
        assert!(res.is_some());
        let snap_path = res.unwrap().unwrap();
        assert!(snap_path.exists());

        // Caso 3: Mesmo tempo ou intervalo não atingido -> não dispara
        let res = service.tick(105, true, 2, &p, Some(&proj_path));
        assert!(res.is_none());

        // Caso 4: Intervalo atingido -> dispara novo snapshot
        let res = service.tick(111, true, 2, &p, Some(&proj_path));
        assert!(res.is_some());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn autosave_coalesces_cycles_without_a_new_revision() {
        let mut service = AutosaveService::new(AutosaveConfig {
            enabled: true,
            interval_secs: 10,
            keep_n: 3,
            only_when_dirty: true,
        });
        let p = Project::new();
        let dir = std::env::temp_dir().join(format!("petunia_as_rev_{}", uuid::Uuid::new_v4()));
        let proj_path = dir.join("proj.petunia");
        assert!(service.tick(100, true, 7, &p, Some(&proj_path)).is_some());
        // Mesmo documento ainda dirty, mas a revisão não mudou: nada novo a gravar.
        assert!(service.tick(200, true, 7, &p, Some(&proj_path)).is_none());
        // Nova revisão -> novo snapshot.
        assert!(service.tick(300, true, 8, &p, Some(&proj_path)).is_some());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn failed_snapshots_are_retried_on_the_same_revision() {
        struct Failing;
        impl AutosaveStorage for Failing {
            fn write_snapshot(
                &self,
                _: &Project,
                _: Option<&Path>,
                _: u64,
                _: u64,
                _: u64,
                _: usize,
            ) -> Result<PathBuf, ProjectError> {
                Err(ProjectError::Io("disk full".into()))
            }
        }
        let mut service = AutosaveService::new(AutosaveConfig {
            enabled: true,
            interval_secs: 10,
            keep_n: 3,
            only_when_dirty: true,
        });
        let p = Project::new();
        assert!(matches!(
            service.tick_with(&Failing, 100, true, 1, &p, None),
            Some(Err(_))
        ));
        // A falha não consome a revisão: a próxima janela tenta de novo.
        assert!(matches!(
            service.tick_with(&Failing, 200, true, 1, &p, None),
            Some(Err(_))
        ));
    }

    #[test]
    fn test_autosave_retention_pruning() {
        let dir =
            std::env::temp_dir().join(format!("petunia_retention_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();

        let mut p = Project::new();
        p.add("TestMesh", Mesh::cube(1.0));

        let proj_path = dir.join("test.petunia");

        // Cria 7 snapshots com limite keep_n = 3
        for i in 1..=7 {
            let snap =
                AutosaveService::perform_snapshot(&p, Some(&proj_path), 1000 + i, i, i, 3).unwrap();
            assert!(snap.exists());
        }

        // Verifica que restaram exatamente 3 arquivos
        let as_dir = AutosaveService::autosave_dir(Some(&proj_path));
        let remaining: Vec<_> = fs::read_dir(&as_dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert_eq!(
            remaining.len(),
            3,
            "Retenção deve manter exatamente keep_n=3 arquivos"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_session_lock_and_recovery_detection() {
        let dir =
            std::env::temp_dir().join(format!("petunia_recovery_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let proj_path = dir.join("adventure.petunia");

        let mut p = Project::new();
        p.name = "Adventure".to_string();

        // 1. Início de sessão: cria lock
        AutosaveService::create_session_lock(Some(&proj_path), &p.name, 5000).unwrap();

        // 2. Autosave grava snapshot
        let snap_path =
            AutosaveService::perform_snapshot(&p, Some(&proj_path), 5010, 1, 42, 5).unwrap();
        assert!(snap_path.exists());

        // 3. Simula crash (lock permanece no disco sem clean shutdown)
        let rec = AutosaveService::detect_recovery(Some(&proj_path));
        assert!(rec.is_some(), "Crash marker deve detectar recuperação");
        let info = rec.unwrap();
        assert_eq!(info.project_name, "Adventure");
        assert_eq!(info.snapshot_path, snap_path);

        // 4. Descarte da recuperação limpa snapshots e lock
        AutosaveService::discard_recovery(Some(&proj_path)).unwrap();
        assert!(!AutosaveService::session_lock_path(Some(&proj_path)).exists());
        assert!(!snap_path.exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn generations_are_listed_newest_first_with_revision_and_discarded_one_by_one() {
        let dir =
            std::env::temp_dir().join(format!("petunia_generations_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let proj_path = dir.join("scene.petunia");
        let p = Project::new();
        for (ts, rev) in [(100, 3), (200, 7), (300, 9)] {
            AutosaveService::perform_snapshot(&p, Some(&proj_path), ts, ts, rev, 5).unwrap();
        }

        let generations = AutosaveService::list_recovery_generations(Some(&proj_path));
        let summary: Vec<_> = generations
            .iter()
            .map(|g| (g.timestamp, g.revision))
            .collect();
        assert_eq!(
            summary,
            vec![(300, Some(9)), (200, Some(7)), (100, Some(3))]
        );
        assert!(
            generations
                .iter()
                .all(|g| g.project_path.as_deref() == Some(&*proj_path))
        );

        AutosaveService::discard_generation(&generations[0]).unwrap();
        let left = AutosaveService::list_recovery_generations(Some(&proj_path));
        assert_eq!(left.len(), 2);
        assert_eq!(left[0].timestamp, 200);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn untitled_documents_keep_separate_generations_per_project_id() {
        let a = Project::new();
        let b = Project::new();
        assert_ne!(a.id, b.id);
        let dir_a = AutosaveService::autosave_dir_for(None, a.id);
        let dir_b = AutosaveService::autosave_dir_for(None, b.id);
        assert_ne!(dir_a, dir_b);

        // keep_n = 1 em A não poda a geração de B.
        let snap_b = AutosaveService::perform_snapshot(&b, None, 10, 1, 1, 1).unwrap();
        AutosaveService::perform_snapshot(&a, None, 20, 1, 1, 1).unwrap();
        AutosaveService::perform_snapshot(&a, None, 30, 2, 2, 1).unwrap();
        assert!(snap_b.exists());
        let _ = fs::remove_dir_all(dir_a.parent().unwrap().parent().unwrap());
        let _ = fs::remove_dir_all(dir_b.parent().unwrap().parent().unwrap());
    }

    #[test]
    fn snapshot_names_parse_with_and_without_revision() {
        assert_eq!(
            AutosaveService::parse_snapshot_name("autosave-0000000100-0002-r17.petunia"),
            Some((100, 2, Some(17)))
        );
        assert_eq!(
            AutosaveService::parse_snapshot_name("autosave-0000000100-0002.petunia"),
            Some((100, 2, None))
        );
        assert_eq!(
            AutosaveService::parse_snapshot_name("autosave-x.petunia"),
            None
        );
        assert_eq!(AutosaveService::parse_snapshot_name("notes.txt"), None);
    }
}
