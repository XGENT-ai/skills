use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions, TryLockError},
    io::Write,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Debug)]
pub struct StoreError {
    pub code: u8,
    pub message: String,
}

pub(crate) fn error(code: u8, message: impl ToString) -> StoreError {
    StoreError {
        code,
        message: message.to_string(),
    }
}

pub(crate) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Header {
    schema_version: u32,
    revision: u64,
}

pub(crate) fn snapshot(bytes: Vec<u8>, code: u8) -> Result<Snapshot, StoreError> {
    let header: Header = serde_json::from_slice(&bytes).map_err(|e| error(code, e))?;
    if header.schema_version != 1 || header.revision == 0 {
        return Err(error(code, "Unsupported schema or invalid revision"));
    }
    Ok(Snapshot {
        revision: header.revision,
        hash: hash(&bytes),
        bytes,
    })
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub revision: u64,
    pub hash: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug)]
pub struct CycleLock {
    root: PathBuf,
    lock_path: PathBuf,
    _lock: Option<File>,
}

impl CycleLock {
    /// The index identifies the cycle lock; writes remain bounded by the project.
    /// This allows the fixed plan record to live outside the report directory.
    pub fn acquire_project(project: &Path, index: &Path) -> Result<Self, StoreError> {
        let text = index
            .to_str()
            .ok_or_else(|| error(2, "Cycle index must have a UTF-8 project-relative path"))?;
        if text
            .split(['/', '\\'])
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
        {
            return Err(error(2, "Cycle index must use a canonical relative path"));
        }
        let root = fs::canonicalize(project).map_err(|e| error(4, e))?;
        Self::acquire_at(root, index)
    }

    pub fn acquire(index: &Path) -> Result<Self, StoreError> {
        let parent = index
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent).map_err(|e| error(4, e))?;
        let root = fs::canonicalize(parent).map_err(|e| error(4, e))?;
        let name = index
            .file_name()
            .ok_or_else(|| error(2, "Cycle index needs a file name"))?;
        Self::acquire_at(root, Path::new(name))
    }

    fn acquire_at(root: PathBuf, index: &Path) -> Result<Self, StoreError> {
        Self::resolve_path(&root, index)?;
        let relative_lock = index.with_extension("lock");
        let lock_path = Self::resolve_path(&root, &relative_lock)?;
        fs::create_dir_all(lock_path.parent().unwrap()).map_err(|e| error(4, e))?;
        let lock_path = Self::resolve_path(&root, &relative_lock)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|e| error(4, e))?;
        lock.try_lock().map_err(|e| match e {
            TryLockError::WouldBlock => error(3, "Another writer holds this cycle lock"),
            TryLockError::Error(e) => error(4, e),
        })?;
        Ok(Self {
            root,
            lock_path,
            _lock: Some(lock),
        })
    }

    /// A lock-free view for read-only status: nothing is created, locked or
    /// recovered, so concurrent writers are reported rather than blocked.
    pub(crate) fn observe(project: &Path, index: &Path) -> Result<Self, StoreError> {
        let root = fs::canonicalize(project).map_err(|e| error(4, e))?;
        Self::resolve_path(&root, index)?;
        let lock_path = Self::resolve_path(&root, &index.with_extension("lock"))?;
        Ok(Self {
            root,
            lock_path,
            _lock: None,
        })
    }

    pub(crate) fn publication_path(&self) -> PathBuf {
        self.lock_path
            .strip_prefix(&self.root)
            .unwrap()
            .with_extension("publish.json")
    }

    fn resolve_path(root: &Path, relative: &Path) -> Result<PathBuf, StoreError> {
        if relative.as_os_str().is_empty() {
            return Err(error(2, "State path is empty"));
        }
        let mut path = root.to_owned();
        for component in relative.components() {
            let Component::Normal(name) = component else {
                return Err(error(2, "State paths must stay inside the cycle directory"));
            };
            path.push(name);
            match fs::symlink_metadata(&path) {
                Ok(m) if m.file_type().is_symlink() => {
                    return Err(error(2, "State paths cannot follow symbolic links"))
                }
                Ok(_) => (),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(error(4, e)),
            }
        }
        Ok(path)
    }

    pub(crate) fn path(&self, relative: &Path) -> Result<PathBuf, StoreError> {
        let path = Self::resolve_path(&self.root, relative)?;
        if path == self.lock_path {
            return Err(error(2, "Cannot replace the cycle lock"));
        }
        Ok(path)
    }

    pub fn read(&self, relative: &Path) -> Result<Option<Snapshot>, StoreError> {
        match fs::read(self.path(relative)?) {
            Ok(bytes) => snapshot(bytes, 4).map(Some),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(error(4, e)),
        }
    }

    pub(crate) fn verify_sources(&self, sources: &[(PathBuf, String)]) -> Result<(), StoreError> {
        for (path, expected_hash) in sources {
            let bytes = fs::read(self.path(path)?).map_err(|e| error(3, e))?;
            if hash(&bytes) != *expected_hash {
                return Err(error(3, "A frozen source changed"));
            }
        }
        Ok(())
    }

    pub(crate) fn compare(
        &self,
        relative: &Path,
        expected: Option<&Snapshot>,
        sources: &[(PathBuf, String)],
    ) -> Result<(), StoreError> {
        if let Some(expected) = expected {
            let verified = snapshot(expected.bytes.clone(), 2)?;
            if verified.revision != expected.revision || verified.hash != expected.hash {
                return Err(error(2, "Expected snapshot does not match its bytes"));
            }
        }
        let current = self.read(relative)?;
        if current.as_ref().map(|s| (s.revision, &s.hash))
            != expected.map(|s| (s.revision, &s.hash))
        {
            return Err(error(3, "State revision or source hash changed"));
        }
        self.verify_sources(sources)
    }

    pub fn replace(
        &self,
        relative: &Path,
        expected: Option<&Snapshot>,
        bytes: &[u8],
        sources: &[(PathBuf, String)],
    ) -> Result<Snapshot, StoreError> {
        if self.publication_pending()? {
            return Err(error(3, "Recover the pending publication before writing"));
        }
        if let Some(old) = expected {
            if crate::publication::is_finalized(&old.bytes, 4)? {
                return Err(error(3, "Finalized round facts are immutable"));
            }
        }
        let next = snapshot(bytes.to_vec(), 2)?;
        let revision = expected
            .map_or(Some(1), |s| s.revision.checked_add(1))
            .ok_or_else(|| error(2, "Revision exceeds the storage limit"))?;
        if next.revision != revision {
            return Err(error(2, "Write must advance exactly one revision"));
        }
        self.write_atomic(relative, bytes, || {
            self.compare(relative, expected, sources)
        })?;
        Ok(next)
    }

    pub(crate) fn write_atomic(
        &self,
        relative: &Path,
        bytes: &[u8],
        compare: impl Fn() -> Result<(), StoreError>,
    ) -> Result<(), StoreError> {
        compare()?;
        let destination = self.path(relative)?;
        let parent = destination.parent().unwrap();
        fs::create_dir_all(parent).map_err(|e| error(4, e))?;
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let temporary = parent.join(format!(
            ".review-{}-{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .map_err(|e| error(4, e))?;
            if let Ok(metadata) = fs::metadata(&destination) {
                file.set_permissions(metadata.permissions())
                    .map_err(|e| error(4, e))?;
            }
            file.write_all(bytes)
                .and_then(|()| file.sync_all())
                .map_err(|e| error(4, e))?;
            drop(file);
            compare()?;
            fs::rename(&temporary, self.path(relative)?).map_err(|e| error(4, e))?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}
