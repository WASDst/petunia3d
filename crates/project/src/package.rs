//! Versioned project package container (`.petunia-pkg`, P1-02, `zip`).
//!
//! A package bundles exactly one project document plus declared attachments:
//!
//! ```text
//! manifest.json      — PackageManifest (name, format version, file table)
//! project.petunia    — canonical project bytes (see format::save)
//! assets/<name>      — declared attachments only (thumbnails, refs)
//! ```
//!
//! Rules enforced here, not by callers:
//!
//! - format version gate (`PACKAGE_VERSION`, migration point for v2);
//! - contained paths only (no absolute paths, no `..` traversal on extract);
//! - size limits (uncompressed cap + entry count cap, zip-bomb class);
//! - deterministic manifest serialization (sorted file table);
//! - atomic write through [`crate::io_atomic::atomic_write`];
//! - cleanup on cancel/failure (temp staging via [`crate::io_atomic::TempScope`]).

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Project;
use crate::format::{self, ProjectError};
use crate::io_atomic::{TempScope, atomic_write};

/// Package format version. Bump with a migration in [`open_package`].
pub const PACKAGE_VERSION: u32 = 1;
/// Manifest file name inside the container.
pub const MANIFEST_NAME: &str = "manifest.json";
/// Canonical project document name inside the container.
pub const PROJECT_DOC_NAME: &str = "project.petunia";
/// Max total uncompressed bytes across entries (64 MiB).
pub const MAX_UNCOMPRESSED_BYTES: u64 = 64 * 1024 * 1024;
/// Max number of entries (manifest + project + attachments).
pub const MAX_ENTRIES: usize = 1 + 1 + 256;

/// One file tracked by the manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackagedFile {
    /// Container-relative path with `/` separators (never absolute).
    pub path: String,
    /// Uncompressed size in bytes.
    pub bytes: u64,
}

/// Package manifest: identity + format version + sorted file table.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageManifest {
    /// Display name of the package.
    pub name: String,
    /// Container format version (see [`PACKAGE_VERSION`]).
    pub format_version: u32,
    /// Sorted by path for deterministic serialization.
    pub files: Vec<PackagedFile>,
}

impl PackageManifest {
    fn new(name: &str, mut files: Vec<PackagedFile>) -> Self {
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Self {
            name: name.to_string(),
            format_version: PACKAGE_VERSION,
            files,
        }
    }
}

/// Package errors with stable, user-presentable messages.
#[derive(Debug, thiserror::Error)]
pub enum PackageError {
    /// IO failure with path context.
    #[error("package IO '{path}': {detail}")]
    Io {
        /// Offending path.
        path: String,
        /// Cause.
        detail: String,
    },
    /// Structural violation (bad manifest, traversal, limits, version).
    #[error("package invalid: {0}")]
    Invalid(String),
    /// Underlying project document failure.
    #[error("project document: {0}")]
    Project(#[from] ProjectError),
}

/// Declared attachment bundled into a package.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attachment {
    /// File name inside `assets/` (no directories, no traversal).
    pub name: String,
    /// Raw bytes.
    pub bytes: Vec<u8>,
}

impl Attachment {
    /// Validates the attachment name (flat file name, bounded size).
    pub fn new(name: &str, bytes: Vec<u8>) -> Result<Self, PackageError> {
        if name.is_empty() || name.len() > 128 {
            return Err(PackageError::Invalid(format!(
                "bad attachment name '{name}'"
            )));
        }
        if name.contains(['/', '\\']) || name == "." || name == ".." {
            return Err(PackageError::Invalid(format!(
                "attachment name escapes assets/ '{name}'"
            )));
        }
        if bytes.len() as u64 > MAX_UNCOMPRESSED_BYTES {
            return Err(PackageError::Invalid(format!(
                "attachment '{name}' exceeds size limit"
            )));
        }
        Ok(Self {
            name: name.to_string(),
            bytes,
        })
    }

    fn container_path(&self) -> String {
        format!("assets/{}", self.name)
    }
}

/// Serializes a versioned package container into an in-memory byte buffer.
pub fn save_package_bytes(
    project: &Project,
    name: &str,
    attachments: &[Attachment],
) -> Result<(PackageManifest, Vec<u8>), PackageError> {
    if attachments.len() + 2 > MAX_ENTRIES {
        return Err(PackageError::Invalid(
            "too many package entries".to_string(),
        ));
    }
    let scope = TempScope::new("petunia-pkg-").map_err(|e| PackageError::Io {
        path: "memory".to_string(),
        detail: e.to_string(),
    })?;
    let stage = scope.path().join("stage");
    std::fs::create_dir(&stage).map_err(|e| PackageError::Io {
        path: "stage".to_string(),
        detail: e.to_string(),
    })?;

    let project_path = stage.join(PROJECT_DOC_NAME);
    format::save(&crate::Project::clone(project), &project_path)?;
    let mut files = vec![PackagedFile {
        path: PROJECT_DOC_NAME.to_string(),
        bytes: std::fs::metadata(&project_path)
            .map_err(|e| PackageError::Io {
                path: PROJECT_DOC_NAME.to_string(),
                detail: e.to_string(),
            })?
            .len(),
    }];
    let assets_dir = stage.join("assets");
    if !attachments.is_empty() {
        std::fs::create_dir(&assets_dir).map_err(|e| PackageError::Io {
            path: "assets".to_string(),
            detail: e.to_string(),
        })?;
    }
    for attachment in attachments {
        let dest = assets_dir.join(&attachment.name);
        std::fs::write(&dest, &attachment.bytes).map_err(|e| PackageError::Io {
            path: attachment.container_path(),
            detail: e.to_string(),
        })?;
        files.push(PackagedFile {
            path: attachment.container_path(),
            bytes: attachment.bytes.len() as u64,
        });
    }
    let mut manifest = PackageManifest::new(name, files);
    manifest.files.push(PackagedFile {
        path: MANIFEST_NAME.to_string(),
        bytes: 0,
    });
    manifest.files.sort_by(|a, b| a.path.cmp(&b.path));
    // Fixpoint: the manifest lists its own byte length, which shifts the
    // length itself. Converges in ≤2 rounds for realistic sizes.
    let mut manifest_json = String::new();
    for _ in 0..4 {
        manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| PackageError::Invalid(format!("manifest serialization: {e}")))?;
        let entry = manifest
            .files
            .iter_mut()
            .find(|f| f.path == MANIFEST_NAME)
            .ok_or_else(|| PackageError::Invalid("manifest missing self-entry".into()))?;
        if entry.bytes == manifest_json.len() as u64 {
            break;
        }
        entry.bytes = manifest_json.len() as u64;
    }
    std::fs::write(stage.join(MANIFEST_NAME), manifest_json.as_bytes()).map_err(|e| {
        PackageError::Io {
            path: MANIFEST_NAME.to_string(),
            detail: e.to_string(),
        }
    })?;

    let staging_zip = scope.path().join("package.zip");
    zip_dir(&stage, &staging_zip)?;
    let bytes = std::fs::read(&staging_zip).map_err(|e| PackageError::Io {
        path: "package.zip".to_string(),
        detail: e.to_string(),
    })?;
    Ok((manifest, bytes))
}

/// Writes a versioned package atomically to `target`.
pub fn save_package(
    project: &Project,
    name: &str,
    attachments: &[Attachment],
    target: &Path,
) -> Result<PackageManifest, PackageError> {
    let (manifest, bytes) = save_package_bytes(project, name, attachments)?;
    atomic_write(target, &bytes).map_err(|e| PackageError::Io {
        path: target.display().to_string(),
        detail: e.to_string(),
    })?;
    Ok(manifest)
}

/// Opens a package: validates container, manifest, version and limits, then
/// parses the project document. Returns the project, manifest and attachments.
pub fn open_package(
    path: &Path,
) -> Result<(Project, PackageManifest, Vec<Attachment>), PackageError> {
    let bytes = std::fs::read(path).map_err(|e| PackageError::Io {
        path: path.display().to_string(),
        detail: e.to_string(),
    })?;
    open_package_bytes(&bytes)
}

/// Opens a package directly from an in-memory byte slice.
pub fn open_package_bytes(
    bytes: &[u8],
) -> Result<(Project, PackageManifest, Vec<Attachment>), PackageError> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| PackageError::Invalid(format!("not a package container: {e}")))?;
    if archive.len() > MAX_ENTRIES {
        return Err(PackageError::Invalid(
            "too many package entries".to_string(),
        ));
    }
    let mut total: u64 = 0;
    for i in 0..archive.len() {
        let file = archive
            .by_index(i)
            .map_err(|e| PackageError::Invalid(format!("unreadable entry: {e}")))?;
        total = total.saturating_add(file.size());
    }
    if total > MAX_UNCOMPRESSED_BYTES {
        return Err(PackageError::Invalid(
            "package exceeds size limit".to_string(),
        ));
    }
    let manifest: PackageManifest = {
        let mut entry = archive
            .by_name(MANIFEST_NAME)
            .map_err(|_| PackageError::Invalid("missing manifest.json".to_string()))?;
        let mut json = String::new();
        entry
            .read_to_string(&mut json)
            .map_err(|e| PackageError::Invalid(format!("manifest unreadable: {e}")))?;
        serde_json::from_str(&json)
            .map_err(|e| PackageError::Invalid(format!("manifest malformed: {e}")))?
    };
    if manifest.format_version != PACKAGE_VERSION {
        return Err(PackageError::Invalid(format!(
            "unsupported package version {} (max {PACKAGE_VERSION})",
            manifest.format_version
        )));
    }
    let project = {
        let mut entry = archive
            .by_name(PROJECT_DOC_NAME)
            .map_err(|_| PackageError::Invalid("missing project.petunia".to_string()))?;
        let mut doc = Vec::new();
        entry
            .read_to_end(&mut doc)
            .map_err(|e| PackageError::Invalid(format!("project document unreadable: {e}")))?;
        drop(entry);
        format::load_bytes(&doc)?
    };
    let mut attachments = Vec::new();
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| PackageError::Invalid(format!("unreadable entry: {e}")))?;
        let name = file.name().to_string();
        if name == MANIFEST_NAME || name == PROJECT_DOC_NAME || name.ends_with('/') {
            continue;
        }
        let enclosed = PathBuf::from(&name);
        if enclosed.is_absolute() || name.contains("..") {
            return Err(PackageError::Invalid(format!(
                "entry escapes container: '{name}'"
            )));
        }
        let Some(rest) = name.strip_prefix("assets/") else {
            return Err(PackageError::Invalid(format!("unexpected entry: '{name}'")));
        };
        if rest.contains('/') {
            return Err(PackageError::Invalid(format!("nested entry: '{name}'")));
        }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|e| PackageError::Invalid(format!("attachment unreadable: {e}")))?;
        attachments.push(Attachment::new(rest, bytes)?);
    }
    attachments.sort_by(|a, b| a.name.cmp(&b.name));
    Ok((project, manifest, attachments))
}

fn zip_dir(stage: &Path, out: &Path) -> Result<(), PackageError> {
    let file = std::fs::File::create(out).map_err(|e| PackageError::Io {
        path: out.display().to_string(),
        detail: e.to_string(),
    })?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let mut entries: Vec<PathBuf> = Vec::new();
    collect_files(stage, &mut entries).map_err(|e| PackageError::Io {
        path: stage.display().to_string(),
        detail: e.to_string(),
    })?;
    entries.sort();
    for entry in entries {
        let rel = entry.strip_prefix(stage).map_err(|e| PackageError::Io {
            path: entry.display().to_string(),
            detail: e.to_string(),
        })?;
        let name = rel.to_string_lossy().replace('\\', "/");
        zip.start_file(name, options)
            .map_err(|e| PackageError::Io {
                path: out.display().to_string(),
                detail: e.to_string(),
            })?;
        let bytes = std::fs::read(&entry).map_err(|e| PackageError::Io {
            path: entry.display().to_string(),
            detail: e.to_string(),
        })?;
        zip.write_all(&bytes).map_err(|e| PackageError::Io {
            path: entry.display().to_string(),
            detail: e.to_string(),
        })?;
    }
    zip.finish().map_err(|e| PackageError::Io {
        path: out.display().to_string(),
        detail: e.to_string(),
    })?;
    Ok(())
}

fn collect_files(dir: &Path, acc: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_files(&path, acc)?;
        } else {
            acc.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use petunia_mesh::Mesh;

    fn sample_project() -> Project {
        let mut project = Project::new();
        project.add("PkgCube", Mesh::cube(1.0));
        project
    }

    #[test]
    fn roundtrip_with_attachments() {
        let dir = TempScope::new("petunia-pkg-test-").unwrap();
        let target = dir.path().join("scene.pkg");
        let attachments = vec![
            Attachment::new("thumb.png", vec![1, 2, 3, 4]).unwrap(),
            Attachment::new("ref.json", br#"{"a":1}"#.to_vec()).unwrap(),
        ];
        let manifest = save_package(&sample_project(), "demo", &attachments, &target).unwrap();
        assert_eq!(manifest.format_version, PACKAGE_VERSION);
        assert_eq!(manifest.files.len(), 4);
        // Deterministic: sorted file table.
        let paths: Vec<_> = manifest.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(
            paths,
            [
                "assets/ref.json",
                "assets/thumb.png",
                "manifest.json",
                "project.petunia"
            ]
        );

        let (project, manifest_back, attachments_back) = open_package(&target).unwrap();
        assert_eq!(manifest_back, manifest);
        assert!(project.assets.iter().any(|a| a.name == "PkgCube"));
        let mut expected = attachments.clone();
        expected.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(attachments_back, expected);
    }

    #[test]
    fn traversal_names_are_rejected() {
        assert!(Attachment::new("../evil", vec![]).is_err());
        assert!(Attachment::new("a/b", vec![]).is_err());
        assert!(Attachment::new("", vec![]).is_err());
    }

    #[test]
    fn garbage_bytes_are_rejected() {
        assert!(matches!(
            open_package_bytes(b"not a zip at all"),
            Err(PackageError::Invalid(_))
        ));
        assert!(matches!(
            open_package_bytes(&[]),
            Err(PackageError::Invalid(_))
        ));
    }

    #[test]
    fn foreign_zip_without_manifest_is_rejected() {
        let dir = TempScope::new("petunia-pkg-foreign-").unwrap();
        let path = dir.path().join("foreign.zip");
        {
            let file = std::fs::File::create(&path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            zip.start_file("hello.txt", zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"hi").unwrap();
            zip.finish().unwrap();
        }
        let bytes = std::fs::read(&path).unwrap();
        assert!(matches!(
            open_package_bytes(&bytes),
            Err(PackageError::Invalid(_))
        ));
    }
}
