use super::*;
use std::ffi::CString;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt};

pub(crate) fn uid() -> u32 {
    unsafe { libc::geteuid() }
}
fn c_name(name: &str) -> OutputResult<CString> {
    if name.is_empty() || name == "." || name == ".." || name.contains('/') {
        return Err(OutputError::new(
            ErrorKind::Invalid,
            "output/invalid-name",
            "Expected one ordinary path component",
        ));
    }
    CString::new(name).map_err(|_| {
        OutputError::new(
            ErrorKind::Invalid,
            "output/invalid-name",
            "NUL is not a path component",
        )
    })
}
pub(super) fn absolute(path: &Path) -> OutputResult<PathBuf> {
    let path = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|e| OutputError::io("output/current-directory", e))?
            .join(path)
    };
    if path.to_str().is_none() {
        return Err(OutputError::new(
            ErrorKind::Invalid,
            "output/non-utf8-path",
            "Recovery paths must have a UTF-8 representation",
        ));
    }
    Ok(path)
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Entry {
    pub device: u64,
    pub inode: u64,
    pub links: u64,
    pub mode: u32,
    pub uid: u32,
    pub kind: &'static str,
    pub bytes: u64,
}
#[derive(Debug)]
pub(crate) struct Directory {
    pub requested: PathBuf,
    pub real: PathBuf,
    pub file: File,
    pub device: u64,
    pub inode: u64,
}
impl Directory {
    pub fn for_path(path: &Path) -> OutputResult<(Arc<Self>, String)> {
        let requested = absolute(path)?;
        let name = requested
            .file_name()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                OutputError::new(
                    ErrorKind::Invalid,
                    "output/invalid-path",
                    "Output needs a UTF-8 file name",
                )
            })?
            .to_owned();
        c_name(&name)?;
        let parent = requested.parent().ok_or_else(|| {
            OutputError::new(
                ErrorKind::Invalid,
                "output/invalid-path",
                "Output needs an existing parent directory",
            )
        })?;
        Ok((Arc::new(Self::open(parent)?), name))
    }
    pub fn open(requested: &Path) -> OutputResult<Self> {
        let requested = absolute(requested)?;
        let real = fs::canonicalize(&requested)
            .map_err(|e| OutputError::io("output/parent-resolution", e))?;
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&real)
            .map_err(|e| OutputError::io("output/parent-open", e))?;
        let metadata = file
            .metadata()
            .map_err(|e| OutputError::io("output/parent-metadata", e))?;
        if !metadata.is_dir() || metadata.ino() == 0 {
            return Err(OutputError::conflict("output/parent-identity-unavailable"));
        }
        let result = Self {
            requested,
            real,
            file,
            device: metadata.dev(),
            inode: metadata.ino(),
        };
        result.verify()?;
        Ok(result)
    }
    pub fn verify(&self) -> OutputResult<()> {
        let current = fs::canonicalize(&self.requested)
            .map_err(|_| OutputError::conflict("output/parent-changed"))?;
        if current != self.real {
            return Err(OutputError::conflict("output/parent-changed"));
        }
        let path = fs::symlink_metadata(&self.real)
            .map_err(|_| OutputError::conflict("output/parent-changed"))?;
        let handle = self
            .file
            .metadata()
            .map_err(|e| OutputError::io("output/parent-metadata", e))?;
        if !path.is_dir()
            || !handle.is_dir()
            || path.dev() != self.device
            || path.ino() != self.inode
            || handle.dev() != self.device
            || handle.ino() != self.inode
        {
            return Err(OutputError::conflict("output/parent-changed"));
        }
        Ok(())
    }
    pub fn same_directory(&self, other: &Self) -> bool {
        self.device == other.device && self.inode == other.inode
    }
    pub fn entry(&self, name: &str) -> OutputResult<Option<Entry>> {
        let name = c_name(name)?;
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
            if error.kind() == std::io::ErrorKind::NotFound {
                return Ok(None);
            }
            return Err(OutputError::io("output/entry-metadata", error));
        }
        let metadata = unsafe { metadata.assume_init() };
        let kind = match metadata.st_mode & libc::S_IFMT {
            libc::S_IFREG => "file",
            libc::S_IFDIR => "directory",
            libc::S_IFLNK => "symbolic-link",
            libc::S_IFIFO => "fifo",
            libc::S_IFSOCK => "socket",
            _ => "special",
        };
        Ok(Some(Entry {
            device: metadata.st_dev as u64,
            inode: metadata.st_ino,
            links: metadata.st_nlink as u64,
            mode: (metadata.st_mode & 0o7777) as u32,
            uid: metadata.st_uid,
            kind,
            bytes: metadata.st_size as u64,
        }))
    }
    pub fn open_regular(&self, name: &str, write: bool) -> OutputResult<File> {
        let before = self
            .entry(name)?
            .ok_or_else(|| OutputError::conflict("output/file-absent"))?;
        if before.kind != "file" {
            return Err(OutputError::conflict("output/not-regular-file"));
        }
        let name = c_name(name)?;
        let flags = if write { libc::O_RDWR } else { libc::O_RDONLY };
        let descriptor = unsafe {
            libc::openat(
                self.file.as_raw_fd(),
                name.as_ptr(),
                flags | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            )
        };
        if descriptor < 0 {
            return Err(OutputError::io(
                "output/file-open",
                std::io::Error::last_os_error(),
            ));
        }
        let file = unsafe { File::from_raw_fd(descriptor) };
        let current = file
            .metadata()
            .map_err(|e| OutputError::io("output/file-metadata", e))?;
        if !current.is_file() || current.dev() != before.device || current.ino() != before.inode {
            return Err(OutputError::conflict("output/file-changed"));
        }
        Ok(file)
    }
    pub fn create_file(&self, name: &str, bytes: &[u8], mode: u32) -> OutputResult<File> {
        let name = c_name(name)?;
        let descriptor = unsafe {
            libc::openat(
                self.file.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        if descriptor < 0 {
            return Err(OutputError::io(
                "output/exclusive-create",
                std::io::Error::last_os_error(),
            ));
        }
        let mut file = unsafe { File::from_raw_fd(descriptor) };
        file.write_all(bytes)
            .map_err(|e| OutputError::io("output/stage-write", e))?;
        if unsafe { libc::fchmod(file.as_raw_fd(), mode as libc::mode_t) } != 0 {
            return Err(OutputError::io(
                "output/stage-mode",
                std::io::Error::last_os_error(),
            ));
        }
        file.sync_all()
            .map_err(|e| OutputError::io("output/file-sync", e))?;
        self.sync()?;
        Ok(file)
    }
    pub fn create_directory(&self, name: &str) -> OutputResult<Arc<Self>> {
        self.verify()?;
        let component = c_name(name)?;
        if unsafe { libc::mkdirat(self.file.as_raw_fd(), component.as_ptr(), 0o700) } != 0 {
            return Err(OutputError::io(
                "output/recovery-directory-create",
                std::io::Error::last_os_error(),
            ));
        }
        self.sync()?;
        let directory = Arc::new(Self::open(&self.real.join(name))?);
        let metadata = directory
            .file
            .metadata()
            .map_err(|e| OutputError::io("output/recovery-directory-metadata", e))?;
        if metadata.uid() != uid() || metadata.mode() & 0o7777 != 0o700 {
            return Err(OutputError::conflict(
                "output/recovery-directory-not-private",
            ));
        }
        Ok(directory)
    }
    pub fn sync(&self) -> OutputResult<()> {
        self.file
            .sync_all()
            .map_err(|e| OutputError::io("output/directory-sync", e))
    }
    pub fn unlink_private(&self, name: &str, directory: bool) -> OutputResult<()> {
        let name = c_name(name)?;
        if unsafe {
            libc::unlinkat(
                self.file.as_raw_fd(),
                name.as_ptr(),
                if directory { libc::AT_REMOVEDIR } else { 0 },
            )
        } != 0
        {
            let error = std::io::Error::last_os_error();
            if directory
                && [libc::ENOTEMPTY, libc::EEXIST].contains(&error.raw_os_error().unwrap_or(0))
            {
                return Err(OutputError::new(
                    ErrorKind::RecoveryRequired,
                    "output/private-directory-not-empty",
                    "Unknown private entries were preserved; cleanup is incomplete",
                ));
            }
            return Err(OutputError::io("output/owned-cleanup", error));
        }
        self.sync()
    }
}
fn stamp(metadata: &fs::Metadata) -> (u64, u64, u64, u32, u64, i64, i64, i64, i64) {
    (
        metadata.dev(),
        metadata.ino(),
        metadata.nlink(),
        metadata.mode(),
        metadata.len(),
        metadata.mtime(),
        metadata.mtime_nsec(),
        metadata.ctime(),
        metadata.ctime_nsec(),
    )
}
pub(super) fn snapshot(
    file: &File,
    parent: &Directory,
    name: &str,
    links: u64,
) -> OutputResult<(FileIdentity, Vec<u8>)> {
    let before = file
        .metadata()
        .map_err(|e| OutputError::io("output/file-metadata", e))?;
    let entry = parent
        .entry(name)?
        .ok_or_else(|| OutputError::conflict("output/file-name-changed"))?;
    if !before.is_file()
        || entry.kind != "file"
        || before.ino() == 0
        || entry.device != before.dev()
        || entry.inode != before.ino()
        || before.nlink() != links
        || entry.links != links
    {
        return Err(OutputError::conflict(
            "output/file-identity-or-links-changed",
        ));
    }
    let size = usize::try_from(before.len())
        .map_err(|_| OutputError::conflict("output/file-size-unavailable"))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size)
        .map_err(|e| OutputError::new(ErrorKind::Io, "output/file-allocation", e.to_string()))?;
    bytes.resize(size, 0);
    let mut offset = 0;
    while offset < size {
        let read = file
            .read_at(&mut bytes[offset..], offset as u64)
            .map_err(|e| OutputError::io("output/file-read", e))?;
        if read == 0 {
            return Err(OutputError::conflict("output/file-changed-during-read"));
        }
        offset += read;
    }
    let after = file
        .metadata()
        .map_err(|e| OutputError::io("output/file-metadata", e))?;
    let current = parent
        .entry(name)?
        .ok_or_else(|| OutputError::conflict("output/file-name-changed"))?;
    if stamp(&before) != stamp(&after)
        || current.kind != "file"
        || current.device != after.dev()
        || current.inode != after.ino()
        || current.links != links
        || current.mode != after.mode() & 0o7777
        || current.uid != after.uid()
    {
        return Err(OutputError::conflict("output/file-changed-during-read"));
    }
    Ok((
        FileIdentity {
            device: after.dev().to_string(),
            inode: after.ino().to_string(),
            links: after.nlink(),
            mode: after.mode() & 0o7777,
            uid: after.uid(),
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            bytes: after.len(),
        },
        bytes,
    ))
}
pub(super) fn link(
    source: &Directory,
    source_name: &str,
    target: &Directory,
    target_name: &str,
) -> OutputResult<()> {
    let source_name = c_name(source_name)?;
    let target_name = c_name(target_name)?;
    if unsafe {
        libc::linkat(
            source.file.as_raw_fd(),
            source_name.as_ptr(),
            target.file.as_raw_fd(),
            target_name.as_ptr(),
            0,
        )
    } != 0
    {
        let error = std::io::Error::last_os_error();
        return Err(if error.kind() == std::io::ErrorKind::AlreadyExists {
            OutputError::conflict("output/later-claimant")
        } else {
            OutputError::io("output/no-clobber-link", error)
        });
    }
    target.sync()?;
    source.sync()
}
pub(crate) fn rename_no_clobber(
    source: &Directory,
    source_name: &str,
    target: &Directory,
    target_name: &str,
) -> OutputResult<()> {
    let source_name = c_name(source_name)?;
    let target_name = c_name(target_name)?;
    #[cfg(target_os = "macos")]
    let result = unsafe {
        libc::renameatx_np(
            source.file.as_raw_fd(),
            source_name.as_ptr(),
            target.file.as_raw_fd(),
            target_name.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::renameat2(
            source.file.as_raw_fd(),
            source_name.as_ptr(),
            target.file.as_raw_fd(),
            target_name.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    let result = -1;
    if result != 0 {
        let error = std::io::Error::last_os_error();
        return Err(if error.kind() == std::io::ErrorKind::AlreadyExists {
            OutputError::conflict("output/later-claimant")
        } else {
            OutputError::io("output/no-clobber-move", error)
        });
    }
    source.sync()?;
    target.sync()
}
pub(crate) fn transaction_id() -> OutputResult<String> {
    let mut bytes = [0_u8; 16];
    File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .map_err(|e| OutputError::io("output/random-source", e))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
pub(crate) fn public_state(parent: &Directory, name: &str) -> PublicState {
    match parent.entry(name) {
        Ok(None) => PublicState::Absent,
        Ok(Some(entry)) if entry.kind == "file" => match parent
            .open_regular(name, false)
            .and_then(|f| snapshot(&f, parent, name, entry.links))
        {
            Ok((identity, _)) => PublicState::File { identity },
            Err(error) => PublicState::Unknown { reason: error.code },
        },
        Ok(Some(entry)) => PublicState::Untrusted {
            entry_type: entry.kind.into(),
        },
        Err(error) => PublicState::Unknown { reason: error.code },
    }
}

#[derive(Debug)]
pub(crate) struct SlotLock {
    pub parent: Arc<Directory>,
    pub target_name: String,
    binding: FileBinding,
}
impl Drop for SlotLock {
    fn drop(&mut self) {
        // Close alone can leave flock held by an unrelated child before exec.
        // This runs only after the final primary/sidecar Arc guard is dropped.
        let _ = self.binding.file.unlock();
    }
}
impl SlotLock {
    pub fn acquire(parent: Arc<Directory>, target_name: &str) -> OutputResult<Self> {
        Self::acquire_mode(parent, target_name, true)
    }
    pub fn acquire_existing(parent: Arc<Directory>, target_name: &str) -> OutputResult<Self> {
        Self::acquire_mode(parent, target_name, false)
    }
    fn acquire_mode(parent: Arc<Directory>, target_name: &str, create: bool) -> OutputResult<Self> {
        parent.verify()?;
        let digest = format!("{:x}", Sha256::digest(target_name.as_bytes()));
        let name = format!(".phoenix-diagram-lock-{digest}.lock");
        let bytes=serde_json::to_vec(&serde_json::json!({"schemaVersion":1,"tool":"phoenix-diagram","kind":"output-lock","name":target_name,"parentDevice":parent.device.to_string(),"parentInode":parent.inode.to_string()})).map_err(|e|OutputError::new(ErrorKind::Io,"output/lock-serialization",e.to_string()))?;
        let file = if let Some(entry) = parent.entry(&name)? {
            if entry.kind != "file"
                || entry.uid != uid()
                || entry.mode != 0o600
                || entry.links != 1
                || entry.bytes != bytes.len() as u64
            {
                return Err(OutputError::conflict("output/unknown-lock-file"));
            }
            parent.open_regular(&name, create)?
        } else if create {
            parent.create_file(&name, &bytes, 0o600)?
        } else {
            return Err(OutputError::conflict("output/existing-lock-absent"));
        };
        match file.try_lock() {
            Ok(()) => {}
            Err(std::fs::TryLockError::WouldBlock) => {
                return Err(OutputError::new(
                    ErrorKind::Busy,
                    "output/busy",
                    "Another process holds the output lock",
                ));
            }
            Err(std::fs::TryLockError::Error(error)) => {
                return Err(OutputError::io("output/lock", error));
            }
        }
        let (identity, content) = match snapshot(&file, &parent, &name, 1) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                let _ = file.unlock();
                return Err(error);
            }
        };
        if identity.mode != 0o600 || identity.uid != uid() || content != bytes {
            let _ = file.unlock();
            return Err(OutputError::conflict("output/unknown-lock-file"));
        }
        let binding = FileBinding {
            file,
            parent: parent.clone(),
            name: name.clone(),
            path: parent.real.join(name),
            identity,
            content,
        };
        Ok(Self {
            parent,
            target_name: target_name.into(),
            binding,
        })
    }
    pub fn verify(&self) -> OutputResult<()> {
        self.parent.verify()?;
        self.binding.verify_at(&self.parent, &self.binding.name, 1)
    }
}
