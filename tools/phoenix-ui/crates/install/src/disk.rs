use crate::{sha256, Error, ErrorKind, ExpectedFile, Permissions, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct State {
    pub hash: String,
    pub permissions: Permissions,
}

pub(crate) struct Snapshot {
    pub state: State,
    pub bytes: Vec<u8>,
}

pub(crate) fn relative(path: &str) -> Result<()> {
    if path.is_empty()
        || path.contains(['\\', ':', '\0'])
        || path
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == ".." || p.ends_with(['.', ' ']))
    {
        return Err(Error::new(
            ErrorKind::InvalidPlan,
            format!("unsafe project path: {path}"),
        ));
    }
    Ok(())
}

pub(crate) fn checked(root: &Path, relative_path: &str) -> Result<PathBuf> {
    relative(relative_path)?;
    let mut path = root.to_path_buf();
    let parts: Vec<_> = relative_path.split('/').collect();
    for (i, component) in parts.iter().enumerate() {
        path.push(component);
        match fs::symlink_metadata(&path) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    return Err(Error::conflict(format!(
                        "symlink is not writable ownership: {}",
                        path.display()
                    )));
                }
                if i + 1 < parts.len() && !meta.is_dir() {
                    return Err(Error::conflict(format!(
                        "path ancestor is not a directory: {}",
                        path.display()
                    )));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(Error::io(&path, e)),
        }
    }
    Ok(path)
}

fn options() -> fs::OpenOptions {
    let mut options = fs::OpenOptions::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    options
}

pub(crate) fn read(root: &Path, path: &str) -> Result<Option<Snapshot>> {
    let full = checked(root, path)?;
    let mut file = match options().read(true).open(&full) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(Error::io(&full, e)),
    };
    let meta = file.metadata().map_err(|e| Error::io(&full, e))?;
    if !meta.is_file() {
        return Err(Error::conflict(format!(
            "not a regular file: {}",
            full.display()
        )));
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|e| Error::io(&full, e))?;
    Ok(Some(Snapshot {
        state: State {
            hash: sha256(&bytes),
            permissions: permissions(&meta),
        },
        bytes,
    }))
}

pub(crate) fn permissions(meta: &fs::Metadata) -> Permissions {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        Permissions {
            unix_mode: meta.permissions().mode() & 0o7777,
            readonly: false,
        }
    }
    #[cfg(not(unix))]
    {
        Permissions {
            unix_mode: 0,
            readonly: meta.permissions().readonly(),
        }
    }
}

fn set_permissions(file: &fs::File, requested: &Permissions) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(requested.unix_mode))
    }
    #[cfg(not(unix))]
    {
        let mut permissions = file.metadata()?.permissions();
        permissions.set_readonly(requested.readonly);
        file.set_permissions(permissions)
    }
}

pub(crate) fn expected(
    path: &str,
    expected: &ExpectedFile,
    snapshot: Option<&Snapshot>,
) -> Result<()> {
    let matches = match (expected, snapshot) {
        (ExpectedFile::Absent, None) => true,
        (ExpectedFile::Hash(hash), Some(snapshot)) => *hash == snapshot.state.hash,
        _ => false,
    };
    if matches {
        Ok(())
    } else {
        Err(Error::conflict(format!(
            "{path}: expected {expected:?}, current {:?}",
            snapshot.map(|s| &s.state.hash)
        )))
    }
}

pub(crate) fn writable(root: &Path, path: &str) -> Result<()> {
    let full = checked(root, path)?;
    let mut parent = full
        .parent()
        .ok_or_else(|| Error::conflict("missing parent"))?;
    while !parent.exists() {
        parent = parent
            .parent()
            .ok_or_else(|| Error::conflict("missing project ancestor"))?;
    }
    let meta = fs::metadata(parent).map_err(|e| Error::io(parent, e))?;
    if !meta.is_dir() || meta.permissions().readonly() {
        return Err(Error::conflict(format!(
            "directory is not writable: {}",
            parent.display()
        )));
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let name = std::ffi::CString::new(parent.as_os_str().as_bytes())
            .map_err(|_| Error::conflict("NUL in project root"))?;
        // No file creation or content change is needed for this permission check.
        if unsafe { libc::access(name.as_ptr(), libc::W_OK | libc::X_OK) } != 0 {
            return Err(Error::io(parent, std::io::Error::last_os_error()));
        }
    }
    Ok(())
}

pub(crate) fn mkdir(root: &Path, path: &str, private: bool) -> Result<()> {
    let mut current = String::new();
    for component in path.split('/') {
        if !current.is_empty() {
            current.push('/');
        }
        current.push_str(component);
        let full = checked(root, &current)?;
        if full.exists() {
            if !fs::metadata(&full)
                .map_err(|e| Error::io(&full, e))?
                .is_dir()
            {
                return Err(Error::conflict(format!(
                    "not a directory: {}",
                    full.display()
                )));
            }
            continue;
        }
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(if private { 0o700 } else { 0o755 });
        }
        #[cfg(not(unix))]
        let _ = private;
        match builder.create(&full) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                checked(root, &current)?;
            }
            Err(e) => return Err(Error::io(&full, e)),
        }
    }
    Ok(())
}

/// The lock's inode is never removed. Kernel ownership, not PID text or a stale
/// filename, decides exclusion and releases the lock after a killed process.
pub(crate) struct Lock(#[allow(dead_code)] fs::File);

pub(crate) fn lock(root: &Path) -> Result<Lock> {
    mkdir(root, ".phoenix-ui/install", true)?;
    let path = checked(root, ".phoenix-ui/install/install.lock")?;
    let mut options = options();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(&path).map_err(|e| Error::io(&path, e))?;
    match file.try_lock() {
        Ok(()) => Ok(Lock(file)),
        Err(fs::TryLockError::WouldBlock) => Err(Error::new(
            ErrorKind::Busy,
            "project installation is locked by another process",
        )),
        Err(fs::TryLockError::Error(e)) => Err(Error::io(&path, e)),
    }
}

pub(crate) fn create(
    root: &Path,
    path: &str,
    bytes: &[u8],
    permissions: &Permissions,
) -> Result<()> {
    let full = checked(root, path)?;
    let mut options = options();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .write(true)
        .create_new(true)
        .open(&full)
        .map_err(|e| Error::io(&full, e))?;
    let result = file
        .write_all(bytes)
        .and_then(|()| set_permissions(&file, permissions))
        .and_then(|()| file.sync_all());
    drop(file);
    if result.is_err() {
        let _ = fs::remove_file(&full);
    }
    result.map_err(|e| Error::io(&full, e))
}

pub(crate) fn replace(
    root: &Path,
    path: &str,
    expected: Option<&State>,
    bytes: Option<&[u8]>,
    permissions: &Permissions,
    exact_stage: Option<&str>,
    before_cas: &mut dyn FnMut(),
) -> Result<()> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let full = checked(root, path)?;
    writable(root, path)?;
    let parent = full
        .parent()
        .ok_or_else(|| Error::conflict("missing parent"))?;
    let parent_relative = parent
        .strip_prefix(root)
        .map_err(|_| Error::conflict("parent escapes project"))?
        .to_string_lossy()
        .replace('\\', "/");
    if !parent_relative.is_empty() {
        mkdir(root, &parent_relative, false)?;
    }
    let temp_relative = exact_stage.map(str::to_string).unwrap_or_else(|| {
        format!(
            "{}{}.phoenix-stage-{}-{}",
            parent_relative,
            if parent_relative.is_empty() { "" } else { "/" },
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )
    });
    if checked(root, &temp_relative)?.parent() != Some(parent) {
        return Err(Error::conflict("stage must share the target directory"));
    }
    let stage = if let Some(bytes) = bytes {
        create(root, &temp_relative, bytes, permissions)?;
        let staged =
            read(root, &temp_relative)?.ok_or_else(|| Error::conflict("stage disappeared"))?;
        if staged.state
            != (State {
                hash: sha256(bytes),
                permissions: permissions.native(),
            })
        {
            return Err(Error::conflict("staging bytes or permissions differ"));
        }
        Some((checked(root, &temp_relative)?, staged.state))
    } else {
        None
    };
    let result = (|| {
        before_cas();
        let current = read(root, path)?;
        if current.as_ref().map(|s| &s.state) != expected {
            return Err(Error::conflict(format!(
                "{path}: changed before atomic replacement"
            )));
        }
        checked(root, path)?;
        if let Some((stage, expected_stage)) = &stage {
            if read(root, &temp_relative)?.as_ref().map(|s| &s.state) != Some(expected_stage) {
                return Err(Error::conflict("stage changed before atomic replacement"));
            }
            fs::rename(stage, &full).map_err(|e| Error::io(&full, e))?;
        } else if current.is_some() {
            fs::remove_file(&full).map_err(|e| Error::io(&full, e))?;
        }
        Ok(())
    })();
    if result.is_err() {
        if let Some((stage, expected_stage)) = &stage {
            if read(root, &temp_relative)
                .ok()
                .flatten()
                .is_some_and(|s| s.state == *expected_stage)
            {
                let _ = fs::remove_file(stage);
            }
        }
    }
    result
}
