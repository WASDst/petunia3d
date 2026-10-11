//! Atomic file writes and scoped temp files (`tempfile`, P0-15).
//!
//! Project saves, exports and recovery snapshots must never leave a
//! half-written file behind: bytes go to a temp file in the *same* directory
//! (same filesystem, so `rename` is atomic) and only then replace the target.
//! Cancellation or crash paths clean up via [`TempScope`].

use std::path::{Path, PathBuf};

/// IO errors with the offending path attached for diagnostics.
#[derive(Debug, thiserror::Error)]
pub enum AtomicIoError {
    /// Creating or writing the temp file failed.
    #[error("temp file for '{path}': {detail}")]
    Temp {
        /// Target path the write was meant for.
        path: String,
        /// Underlying cause.
        detail: String,
    },
    /// Persisting (renaming) the temp file failed.
    #[error("atomic persist of '{path}': {detail}")]
    Persist {
        /// Target path the write was meant for.
        path: String,
        /// Underlying cause.
        detail: String,
    },
}

/// Writes `bytes` atomically: temp file in the target directory + rename.
/// Creates parent directories when missing.
pub fn atomic_write(target: &Path, bytes: &[u8]) -> Result<(), AtomicIoError> {
    let path_str = target.display().to_string();
    if let Some(parent) = target.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| AtomicIoError::Temp {
            path: path_str.clone(),
            detail: e.to_string(),
        })?;
    }
    let dir = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let mut temp = tempfile::NamedTempFile::new_in(&dir).map_err(|e| AtomicIoError::Temp {
        path: path_str.clone(),
        detail: e.to_string(),
    })?;
    use std::io::Write as _;
    // Dados no disco antes do rename: sem `sync_all`, um crash logo após o
    // rename pode deixar o arquivo novo vazio/truncado no lugar do anterior.
    temp.write_all(bytes)
        .and_then(|()| temp.flush())
        .and_then(|()| temp.as_file().sync_all())
        .map_err(|e| AtomicIoError::Temp {
            path: path_str.clone(),
            detail: e.to_string(),
        })?;
    temp.persist(target).map_err(|e| AtomicIoError::Persist {
        path: path_str,
        detail: e.to_string(),
    })?;
    sync_directory(&dir);
    Ok(())
}

/// Torna o rename durável (entrada de diretório) onde o SO permite. Melhor
/// esforço: falhar aqui não desfaz um replace já concluído.
fn sync_directory(dir: &Path) {
    #[cfg(unix)]
    if let Ok(handle) = std::fs::File::open(dir) {
        let _ = handle.sync_all();
    }
    #[cfg(not(unix))]
    let _ = dir;
}

/// Scoped temp directory, deleted on drop. For export staging, recovery
/// scratch and filesystem tests.
#[derive(Debug)]
pub struct TempScope {
    dir: tempfile::TempDir,
}

impl TempScope {
    /// Creates a scope under the system temp dir.
    pub fn new(prefix: &str) -> Result<Self, AtomicIoError> {
        tempfile::Builder::new()
            .prefix(prefix)
            .tempdir()
            .map(|dir| Self { dir })
            .map_err(|e| AtomicIoError::Temp {
                path: prefix.to_string(),
                detail: e.to_string(),
            })
    }

    /// Path of the scoped directory.
    pub fn path(&self) -> &Path {
        self.dir.path()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_roundtrips_and_replaces() {
        let scope = TempScope::new("petunia-io-").unwrap();
        let target = scope.path().join("project.petunia");
        atomic_write(&target, b"v1").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"v1");
        atomic_write(&target, b"v2-longer-payload").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"v2-longer-payload");
    }

    #[test]
    fn temp_scope_cleans_up_on_drop() {
        let path: PathBuf;
        {
            let scope = TempScope::new("petunia-scope-").unwrap();
            path = scope.path().join("scratch.bin");
            atomic_write(&path, b"tmp").unwrap();
            assert!(path.exists());
        }
        assert!(!path.exists());
    }

    #[test]
    fn atomic_write_creates_missing_parents() {
        let scope = TempScope::new("petunia-parents-").unwrap();
        let target = scope.path().join("a").join("b").join("save.petunia");
        atomic_write(&target, b"nested").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"nested");
    }
}
