// Apache-2.0 source: tools/phoenix-ui/crates/install/src/disk.rs.
// Phoenix diagram changes: descriptor-relative directory traversal and
// no-clobber moves; deletion is restricted to journaled private quarantine.
use super::{InstallError, InstallErrorKind, InstallResult};
use crate::output::FileIdentity;
use sha2::{Digest, Sha256};
use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub(super) const FILE_LIMIT: u64 = 256 * 1024 * 1024;
pub(super) const LOCK_BYTES: &[u8] = b"phoenix-diagram install lock v1\n";

fn error(kind: InstallErrorKind, code: &str, message: impl Into<String>) -> InstallError {
    InstallError {
        kind,
        code: code.into(),
        message: message.into(),
    }
}
pub(super) fn conflict(code: &str, message: impl Into<String>) -> InstallError {
    error(InstallErrorKind::Conflict, code, message)
}
fn io(code: &str, error: std::io::Error) -> InstallError {
    self::error(InstallErrorKind::Io, code, error.to_string())
}
fn component(value: &str) -> InstallResult<CString> {
    if value.is_empty() || matches!(value, "." | "..") || value.contains(['/', '\\', ':']) {
        return Err(error(
            InstallErrorKind::Invalid,
            "install/unsafe-path",
            value,
        ));
    }
    CString::new(value).map_err(|_| error(InstallErrorKind::Invalid, "install/unsafe-path", value))
}
pub(super) fn relative(path: &str) -> InstallResult<()> {
    if path.is_empty() || path.split('/').any(|part| part.ends_with(['.', ' '])) {
        return Err(error(
            InstallErrorKind::Invalid,
            "install/unsafe-path",
            path,
        ));
    }
    for part in path.split('/') {
        component(part)?;
    }
    Ok(())
}
pub(super) fn uid() -> u32 {
    unsafe { libc::geteuid() }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Physical {
    pub device: String,
    pub inode: String,
}
impl Physical {
    fn of(meta: &fs::Metadata) -> Self {
        Self {
            device: meta.dev().to_string(),
            inode: meta.ino().to_string(),
        }
    }
}
#[derive(Debug)]
pub(super) struct Directory {
    pub path: PathBuf,
    pub physical: Physical,
    file: File,
    parent: Option<Arc<Directory>>,
    name: Option<String>,
    requested: Option<PathBuf>,
    owner_required: bool,
}
impl Directory {
    pub fn open(path: &Path) -> InstallResult<Arc<Self>> {
        Self::open_root(path, true)
    }
    pub fn open_source(path: &Path) -> InstallResult<Arc<Self>> {
        Self::open_root(path, false)
    }
    fn open_root(path: &Path, owner_required: bool) -> InstallResult<Arc<Self>> {
        let requested = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|e| io("install/current-directory", e))?
                .join(path)
        };
        let before =
            fs::symlink_metadata(&requested).map_err(|e| io("install/root-metadata", e))?;
        if before.is_symlink() || !before.is_dir() || requested.to_str().is_none() {
            return Err(error(
                InstallErrorKind::Invalid,
                "install/root-not-directory",
                requested.display().to_string(),
            ));
        }
        let real = fs::canonicalize(&requested).map_err(|e| io("install/root-resolution", e))?;
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&real)
            .map_err(|e| io("install/root-open", e))?;
        let metadata = file
            .metadata()
            .map_err(|e| io("install/root-metadata", e))?;
        if !metadata.is_dir()
            || Physical::of(&before) != Physical::of(&metadata)
            || (owner_required && metadata.uid() != uid())
        {
            return Err(conflict("install/root-changed", real.display().to_string()));
        }
        let dir = Arc::new(Self {
            path: real,
            physical: Physical::of(&metadata),
            file,
            parent: None,
            name: None,
            requested: Some(requested),
            owner_required,
        });
        dir.verify()?;
        Ok(dir)
    }
    pub fn verify(&self) -> InstallResult<()> {
        if let (Some(parent), Some(name)) = (&self.parent, &self.name) {
            parent.verify()?;
            let entry = parent.entry(name)?.ok_or_else(|| {
                conflict("install/parent-changed", self.path.display().to_string())
            })?;
            if entry.kind != libc::S_IFDIR
                || entry.physical != self.physical
                || (self.owner_required && entry.uid != uid())
            {
                return Err(conflict(
                    "install/parent-changed",
                    self.path.display().to_string(),
                ));
            }
        } else if let Some(requested) = &self.requested {
            let meta = fs::symlink_metadata(requested)
                .map_err(|_| conflict("install/root-changed", requested.display().to_string()))?;
            if !meta.is_dir()
                || meta.is_symlink()
                || Physical::of(&meta) != self.physical
                || fs::canonicalize(requested).ok().as_ref() != Some(&self.path)
            {
                return Err(conflict(
                    "install/root-changed",
                    requested.display().to_string(),
                ));
            }
        }
        let meta = self
            .file
            .metadata()
            .map_err(|e| io("install/directory-metadata", e))?;
        if !meta.is_dir()
            || Physical::of(&meta) != self.physical
            || (self.owner_required && meta.uid() != uid())
        {
            return Err(conflict(
                "install/parent-changed",
                self.path.display().to_string(),
            ));
        }
        Ok(())
    }
    pub fn entry(&self, name: &str) -> InstallResult<Option<Entry>> {
        let name = component(name)?;
        let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
        let result = unsafe {
            libc::fstatat(
                self.file.as_raw_fd(),
                name.as_ptr(),
                metadata.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if result != 0 {
            let error = std::io::Error::last_os_error();
            return if error.kind() == std::io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(io("install/entry-metadata", error))
            };
        }
        let meta = unsafe { metadata.assume_init() };
        Ok(Some(Entry {
            physical: Physical {
                device: meta.st_dev.to_string(),
                inode: meta.st_ino.to_string(),
            },
            kind: meta.st_mode & libc::S_IFMT,
            uid: meta.st_uid,
            mode: (meta.st_mode & 0o7777) as u32,
        }))
    }
    pub fn child(self: &Arc<Self>, name: &str) -> InstallResult<Option<Arc<Self>>> {
        self.verify()?;
        let Some(before) = self.entry(name)? else {
            return Ok(None);
        };
        if before.kind != libc::S_IFDIR || (self.owner_required && before.uid != uid()) {
            return Err(conflict(
                "install/unsafe-parent",
                self.path.join(name).display().to_string(),
            ));
        }
        let c = component(name)?;
        let descriptor = unsafe {
            libc::openat(
                self.file.as_raw_fd(),
                c.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if descriptor < 0 {
            return Err(io(
                "install/directory-open",
                std::io::Error::last_os_error(),
            ));
        }
        let file = unsafe { File::from_raw_fd(descriptor) };
        let metadata = file
            .metadata()
            .map_err(|e| io("install/directory-metadata", e))?;
        if !metadata.is_dir() || Physical::of(&metadata) != before.physical {
            return Err(conflict(
                "install/parent-changed",
                self.path.join(name).display().to_string(),
            ));
        }
        let child = Arc::new(Self {
            path: self.path.join(name),
            physical: before.physical,
            file,
            parent: Some(self.clone()),
            name: Some(name.into()),
            requested: None,
            owner_required: self.owner_required,
        });
        child.verify()?;
        Ok(Some(child))
    }
    pub fn descend(self: &Arc<Self>, path: &str) -> InstallResult<Option<Arc<Self>>> {
        relative(path)?;
        let mut directory = self.clone();
        for part in path.split('/') {
            let Some(child) = directory.child(part)? else {
                return Ok(None);
            };
            directory = child;
        }
        Ok(Some(directory))
    }
    pub fn child_if_directory(self: &Arc<Self>, name: &str) -> InstallResult<Option<Arc<Self>>> {
        match self.entry(name)? {
            Some(entry) if entry.kind == libc::S_IFDIR => self.child(name),
            Some(entry) if entry.kind == libc::S_IFLNK => Err(conflict(
                "install/unsafe-parent",
                self.path.join(name).display().to_string(),
            )),
            _ => Ok(None),
        }
    }
    pub fn private(&self) -> InstallResult<()> {
        self.verify()?;
        let metadata = self
            .file
            .metadata()
            .map_err(|e| io("install/directory-metadata", e))?;
        if metadata.mode() & 0o7777 != 0o700 {
            return Err(conflict(
                "install/state-not-private",
                self.path.display().to_string(),
            ));
        }
        Ok(())
    }
    pub fn writable(&self) -> InstallResult<()> {
        self.verify()?;
        let metadata = self
            .file
            .metadata()
            .map_err(|e| io("install/directory-metadata", e))?;
        if metadata.mode() & 0o200 == 0 {
            return Err(conflict(
                "install/parent-not-writable",
                self.path.display().to_string(),
            ));
        }
        Ok(())
    }
    pub fn ensure(self: &Arc<Self>, path: &str, private: bool) -> InstallResult<Arc<Self>> {
        relative(path)?;
        let mut directory = self.clone();
        for part in path.split('/') {
            directory.verify()?;
            let next = match directory.child(part)? {
                Some(next) => next,
                None => {
                    let c = component(part)?;
                    if unsafe {
                        libc::mkdirat(
                            directory.file.as_raw_fd(),
                            c.as_ptr(),
                            if private { 0o700 } else { 0o755 },
                        )
                    } != 0
                    {
                        let error = std::io::Error::last_os_error();
                        if error.kind() != std::io::ErrorKind::AlreadyExists {
                            return Err(io("install/directory-create", error));
                        }
                    }
                    directory.sync()?;
                    directory
                        .child(part)?
                        .ok_or_else(|| conflict("install/parent-changed", part))?
                }
            };
            if private
                && next
                    .file
                    .metadata()
                    .map_err(|e| io("install/directory-metadata", e))?
                    .mode()
                    & 0o7777
                    != 0o700
            {
                return Err(conflict(
                    "install/state-not-private",
                    next.path.display().to_string(),
                ));
            }
            directory = next;
        }
        Ok(directory)
    }
    pub fn create_child(self: &Arc<Self>, name: &str, private: bool) -> InstallResult<Arc<Self>> {
        self.verify()?;
        let c = component(name)?;
        if unsafe {
            libc::mkdirat(
                self.file.as_raw_fd(),
                c.as_ptr(),
                if private { 0o700 } else { 0o755 },
            )
        } != 0
        {
            let error = std::io::Error::last_os_error();
            return Err(if error.kind() == std::io::ErrorKind::AlreadyExists {
                conflict(
                    "install/later-parent",
                    self.path.join(name).display().to_string(),
                )
            } else {
                io("install/directory-create", error)
            });
        }
        self.sync()?;
        self.child(name)?
            .ok_or_else(|| conflict("install/parent-changed", name))
    }
    pub fn parent(self: &Arc<Self>, path: &str) -> InstallResult<Option<(Arc<Self>, String)>> {
        relative(path)?;
        let (parent, name) = path.rsplit_once('/').unwrap_or(("", path));
        let directory = if parent.is_empty() {
            Some(self.clone())
        } else {
            self.descend(parent)?
        };
        Ok(directory.map(|dir| (dir, name.into())))
    }
    pub fn list(&self) -> InstallResult<Vec<String>> {
        self.verify()?;
        // Opening '.' through the retained descriptor avoids choosing a new path
        // if an ancestor is renamed between verification and enumeration.
        let descriptor = unsafe {
            libc::openat(
                self.file.as_raw_fd(),
                c".".as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        };
        if descriptor < 0 {
            return Err(io(
                "install/directory-list",
                std::io::Error::last_os_error(),
            ));
        }
        let stream = unsafe { libc::fdopendir(descriptor) };
        if stream.is_null() {
            unsafe {
                libc::close(descriptor);
            }
            return Err(io(
                "install/directory-list",
                std::io::Error::last_os_error(),
            ));
        }
        let mut names = vec![];
        let result = loop {
            #[cfg(target_os = "macos")]
            unsafe {
                *libc::__error() = 0;
            }
            #[cfg(target_os = "linux")]
            unsafe {
                *libc::__errno_location() = 0;
            }
            let entry = unsafe { libc::readdir(stream) };
            if entry.is_null() {
                let error = std::io::Error::last_os_error();
                break if error.raw_os_error() == Some(0) {
                    Ok(())
                } else {
                    Err(io("install/directory-list", error))
                };
            }
            let bytes = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
            if matches!(bytes, b"." | b"..") {
                continue;
            }
            match std::str::from_utf8(bytes) {
                Ok(name) if names.len() < 50_000 => names.push(name.to_owned()),
                _ => {
                    break Err(conflict(
                        "install/directory-inventory-incomplete",
                        self.path.display().to_string(),
                    ))
                }
            }
        };
        unsafe {
            libc::closedir(stream);
        }
        result?;
        self.verify()?;
        names.sort();
        Ok(names)
    }
    pub fn read(self: &Arc<Self>, name: &str, links: &[u64]) -> InstallResult<Option<Snapshot>> {
        self.verify()?;
        let Some(entry) = self.entry(name)? else {
            return Ok(None);
        };
        if entry.kind != libc::S_IFREG
            || (self.owner_required && entry.uid != uid())
            || entry.mode > 0o777
        {
            return Err(conflict(
                "install/unsafe-file",
                self.path.join(name).display().to_string(),
            ));
        }
        let c = component(name)?;
        let descriptor = unsafe {
            libc::openat(
                self.file.as_raw_fd(),
                c.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            )
        };
        if descriptor < 0 {
            return Err(io("install/file-open", std::io::Error::last_os_error()));
        }
        let mut file = unsafe { File::from_raw_fd(descriptor) };
        let before = file
            .metadata()
            .map_err(|e| io("install/file-metadata", e))?;
        if !before.is_file()
            || Physical::of(&before) != entry.physical
            || !links.contains(&before.nlink())
            || before.len() > FILE_LIMIT
        {
            return Err(conflict(
                "install/unsafe-file",
                self.path.join(name).display().to_string(),
            ));
        }
        let mut bytes = Vec::with_capacity(before.len() as usize);
        (&mut file)
            .take(FILE_LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| io("install/file-read", e))?;
        let after = file
            .metadata()
            .map_err(|e| io("install/file-metadata", e))?;
        if before.dev() != after.dev()
            || before.ino() != after.ino()
            || before.len() != after.len()
            || before.mode() != after.mode()
            || before.nlink() != after.nlink()
            || before.uid() != after.uid()
            || before.mtime() != after.mtime()
            || before.mtime_nsec() != after.mtime_nsec()
            || before.ctime() != after.ctime()
            || before.ctime_nsec() != after.ctime_nsec()
            || bytes.len() as u64 != after.len()
        {
            return Err(conflict(
                "install/file-changed",
                self.path.join(name).display().to_string(),
            ));
        }
        self.verify()?;
        let current = self
            .entry(name)?
            .ok_or_else(|| conflict("install/file-changed", name))?;
        if current.physical != entry.physical || current.kind != libc::S_IFREG {
            return Err(conflict("install/file-changed", name));
        }
        Ok(Some(Snapshot {
            identity: FileIdentity {
                device: after.dev().to_string(),
                inode: after.ino().to_string(),
                links: after.nlink(),
                mode: after.mode() & 0o7777,
                uid: after.uid(),
                sha256: format!("{:x}", Sha256::digest(&bytes)),
                bytes: after.len(),
            },
            bytes,
        }))
    }
    pub fn read_path(self: &Arc<Self>, path: &str) -> InstallResult<Option<Snapshot>> {
        match self.parent(path)? {
            Some((dir, name)) => dir.read(&name, &[1]),
            None => Ok(None),
        }
    }
    pub fn sync(&self) -> InstallResult<()> {
        self.file
            .sync_all()
            .map_err(|e| io("install/directory-sync", e))
    }
    pub fn lock(self: &Arc<Self>) -> InstallResult<Lock> {
        self.verify()?;
        if let Some(entry) = self.entry("install.lock")? {
            if entry.kind != libc::S_IFREG || entry.uid != uid() || entry.mode != 0o600 {
                return Err(conflict(
                    "install/unknown-lock",
                    self.path.display().to_string(),
                ));
            }
        }
        let descriptor = unsafe {
            libc::openat(
                self.file.as_raw_fd(),
                c"install.lock".as_ptr(),
                libc::O_RDWR
                    | libc::O_CREAT
                    | libc::O_NOFOLLOW
                    | libc::O_CLOEXEC
                    | libc::O_NONBLOCK,
                0o600,
            )
        };
        if descriptor < 0 {
            return Err(io("install/lock-open", std::io::Error::last_os_error()));
        }
        let mut file = unsafe { File::from_raw_fd(descriptor) };
        let metadata = file
            .metadata()
            .map_err(|e| io("install/lock-metadata", e))?;
        if !metadata.is_file()
            || metadata.nlink() != 1
            || metadata.uid() != uid()
            || metadata.mode() & 0o7777 != 0o600
        {
            return Err(conflict(
                "install/unknown-lock",
                self.path.display().to_string(),
            ));
        }
        match file.try_lock() {
            Ok(()) => {}
            Err(fs::TryLockError::WouldBlock) => {
                return Err(error(
                    InstallErrorKind::Busy,
                    "install/busy",
                    "Another diagram installation holds the project lock",
                ))
            }
            Err(fs::TryLockError::Error(error)) => return Err(io("install/lock", error)),
        }
        if metadata.len() == 0 {
            file.write_all(LOCK_BYTES)
                .map_err(|e| io("install/lock-write", e))?;
            file.sync_all().map_err(|e| io("install/lock-sync", e))?;
            self.sync()?;
        } else {
            let mut bytes = vec![];
            (&mut file)
                .take(1024)
                .read_to_end(&mut bytes)
                .map_err(|e| io("install/lock-read", e))?;
            if bytes != LOCK_BYTES {
                return Err(conflict(
                    "install/unknown-lock",
                    self.path.display().to_string(),
                ));
            }
        }
        let lock = Lock {
            file,
            parent: self.clone(),
            physical: Physical::of(&metadata),
        };
        lock.verify()?;
        Ok(lock)
    }
}
#[derive(Debug)]
pub(super) struct Entry {
    pub physical: Physical,
    pub kind: libc::mode_t,
    pub uid: u32,
    pub mode: u32,
}
#[derive(Clone, Debug)]
pub(super) struct Snapshot {
    pub identity: FileIdentity,
    pub bytes: Vec<u8>,
}
pub(super) struct Lock {
    file: File,
    parent: Arc<Directory>,
    physical: Physical,
}
impl Lock {
    pub fn verify(&self) -> InstallResult<()> {
        self.parent.verify()?;
        let meta = self
            .file
            .metadata()
            .map_err(|e| io("install/lock-metadata", e))?;
        let entry = self
            .parent
            .entry("install.lock")?
            .ok_or_else(|| conflict("install/lock-changed", "install.lock"))?;
        if Physical::of(&meta) != self.physical
            || entry.physical != self.physical
            || entry.kind != libc::S_IFREG
            || entry.mode != 0o600
            || meta.nlink() != 1
        {
            return Err(conflict("install/lock-changed", "install.lock"));
        }
        Ok(())
    }
}
pub(super) fn matches(expected: &FileIdentity, actual: &FileIdentity) -> bool {
    expected.device == actual.device
        && expected.inode == actual.inode
        && expected.mode == actual.mode
        && expected.uid == actual.uid
        && expected.sha256 == actual.sha256
        && expected.bytes == actual.bytes
}
pub(super) fn expected(
    dir: &Arc<Directory>,
    name: &str,
    identity: &FileIdentity,
    links: &[u64],
) -> InstallResult<Snapshot> {
    let snapshot = dir.read(name, links)?.ok_or_else(|| {
        conflict(
            "install/file-absent",
            dir.path.join(name).display().to_string(),
        )
    })?;
    if !matches(identity, &snapshot.identity) {
        return Err(conflict(
            "install/binding-changed",
            dir.path.join(name).display().to_string(),
        ));
    }
    Ok(snapshot)
}
pub(super) fn rename(
    source: &Directory,
    name: &str,
    target: &Directory,
    destination: &str,
) -> InstallResult<()> {
    source.verify()?;
    target.verify()?;
    let name = component(name)?;
    let destination = component(destination)?;
    #[cfg(target_os = "macos")]
    let result = unsafe {
        libc::renameatx_np(
            source.file.as_raw_fd(),
            name.as_ptr(),
            target.file.as_raw_fd(),
            destination.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::renameat2(
            source.file.as_raw_fd(),
            name.as_ptr(),
            target.file.as_raw_fd(),
            destination.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result != 0 {
        let error = std::io::Error::last_os_error();
        return Err(if error.kind() == std::io::ErrorKind::AlreadyExists {
            conflict("install/later-claimant", destination.to_string_lossy())
        } else {
            io("install/no-clobber-move", error)
        });
    }
    source.sync()?;
    target.sync()
}
pub(super) fn link(
    source: &Directory,
    name: &str,
    target: &Directory,
    destination: &str,
) -> InstallResult<()> {
    source.verify()?;
    target.verify()?;
    let name = component(name)?;
    let destination = component(destination)?;
    if unsafe {
        libc::linkat(
            source.file.as_raw_fd(),
            name.as_ptr(),
            target.file.as_raw_fd(),
            destination.as_ptr(),
            0,
        )
    } != 0
    {
        let error = std::io::Error::last_os_error();
        return Err(if error.kind() == std::io::ErrorKind::AlreadyExists {
            conflict("install/later-claimant", destination.to_string_lossy())
        } else {
            io("install/no-clobber-link", error)
        });
    }
    source.sync()?;
    target.sync()
}
/// Caller first persists both names and identity in its installation journal.
pub(super) fn remove_known(
    dir: &Arc<Directory>,
    name: &str,
    quarantine: &str,
    identity: &FileIdentity,
    links: &[u64],
) -> InstallResult<()> {
    if dir.entry(name)?.is_some() {
        expected(dir, name, identity, links)?;
        rename(dir, name, dir, quarantine)?;
        if let Err(error) = expected(dir, quarantine, identity, links) {
            let _ = rename(dir, quarantine, dir, name);
            return Err(error);
        }
    }
    if dir.entry(quarantine)?.is_some() {
        expected(dir, quarantine, identity, links)?;
        let quarantine = component(quarantine)?;
        if unsafe { libc::unlinkat(dir.file.as_raw_fd(), quarantine.as_ptr(), 0) } != 0 {
            return Err(io(
                "install/private-unlink",
                std::io::Error::last_os_error(),
            ));
        }
        dir.sync()?;
    }
    Ok(())
}

/// Only for an already-empty, journal-bound legacy directory. Both names are
/// persisted by migration before entering this conditional removal primitive.
pub(super) fn remove_empty_known(
    dir: &Arc<Directory>,
    name: &str,
    temporary: &str,
    physical: &Physical,
    mode: u32,
) -> InstallResult<()> {
    let verify = |name: &str| -> InstallResult<()> {
        let entry = dir
            .entry(name)?
            .ok_or_else(|| conflict("install/legacy-directory-absent", name))?;
        if entry.kind != libc::S_IFDIR
            || entry.physical != *physical
            || entry.uid != uid()
            || entry.mode != mode
        {
            return Err(conflict(
                "install/legacy-directory-changed",
                dir.path.join(name).display().to_string(),
            ));
        }
        let child = dir
            .child(name)?
            .ok_or_else(|| conflict("install/legacy-directory-absent", name))?;
        if !child.list()?.is_empty() {
            return Err(conflict(
                "install/legacy-directory-not-empty",
                child.path.display().to_string(),
            ));
        }
        Ok(())
    };
    if dir.entry(name)?.is_some() {
        verify(name)?;
        rename(dir, name, dir, temporary)?;
        if let Err(error) = verify(temporary) {
            let _ = rename(dir, temporary, dir, name);
            return Err(error);
        }
    }
    if dir.entry(temporary)?.is_some() {
        verify(temporary)?;
        let temporary = component(temporary)?;
        if unsafe { libc::unlinkat(dir.file.as_raw_fd(), temporary.as_ptr(), libc::AT_REMOVEDIR) }
            != 0
        {
            return Err(io("install/legacy-rmdir", std::io::Error::last_os_error()));
        }
        dir.sync()?;
    }
    Ok(())
}
