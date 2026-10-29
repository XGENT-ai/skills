//! Explicit maintenance-only process checkpoints. Normal commands never enable them.
use std::ffi::OsStr;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

pub(crate) fn checkpoint(phase: &str, target: &Path, recovery_dir: &Path) -> io::Result<()> {
    let acceptance = std::env::var_os("PHOENIX_DIAGRAM_ACCEPTANCE");
    let directory = std::env::var_os("PHOENIX_DIAGRAM_CHECKPOINT_DIR");
    let requested = std::env::var_os("PHOENIX_DIAGRAM_CHECKPOINT_PHASE");
    checkpoint_with(
        phase,
        target,
        recovery_dir,
        acceptance.as_deref(),
        directory.as_deref(),
        requested.as_deref(),
        || {
            std::thread::sleep(std::time::Duration::from_secs(30));
            Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Acceptance checkpoint was not interrupted within 30 seconds",
            ))
        },
    )
}

pub(crate) fn preview_worker_checkpoint(generation: u64, input_sha256: &str) -> io::Result<()> {
    let requested = std::env::var_os("PHOENIX_DIAGRAM_CHECKPOINT_PHASE");
    if std::env::var_os("PHOENIX_DIAGRAM_ACCEPTANCE").as_deref() != Some(OsStr::new("1"))
        || !matches!(
            requested.as_deref().and_then(OsStr::to_str),
            Some("preview-worker-ready" | "preview-worker-ignore-term")
        )
    {
        return Ok(());
    }
    let Some(directory) =
        std::env::var_os("PHOENIX_DIAGRAM_CHECKPOINT_DIR").filter(|value| !value.is_empty())
    else {
        return Ok(());
    };
    let directory = Path::new(&directory);
    let (handle, identity) = checkpoint_directory(directory)?;
    let ignore_term = requested.as_deref() == Some(OsStr::new("preview-worker-ignore-term"));
    let previous = if ignore_term {
        let handler = unsafe { libc::signal(libc::SIGTERM, libc::SIG_IGN) };
        if handler == libc::SIG_ERR {
            return Err(io::Error::last_os_error());
        }
        Some(handler)
    } else {
        None
    };
    let result = write_marker(
        directory,
        &handle,
        &identity,
        serde_json::json!({"schemaVersion":1,"kind":"preview-worker-checkpoint","pid":std::process::id(),"phase":if ignore_term {"preview-worker-ignore-term"} else {"preview-worker-ready"},"generation":generation,"inputSha256":input_sha256,"ignoreSigterm":ignore_term}),
        || {
            std::thread::sleep(std::time::Duration::from_secs(30));
            Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Preview worker checkpoint was not interrupted within 30 seconds",
            ))
        },
    );
    if let Some(handler) = previous {
        unsafe {
            libc::signal(libc::SIGTERM, handler);
        }
    }
    result
}

fn checkpoint_with(
    phase: &str,
    target: &Path,
    recovery_dir: &Path,
    acceptance: Option<&OsStr>,
    directory: Option<&OsStr>,
    requested: Option<&OsStr>,
    ready: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    if acceptance != Some(OsStr::new("1")) {
        return Ok(());
    }
    let (Some(directory), Some(requested)) = (
        directory.filter(|value| !value.is_empty()),
        requested.filter(|value| !value.is_empty()),
    ) else {
        return Ok(());
    };
    let requested = requested
        .to_str()
        .filter(|value| {
            matches!(
                *value,
                "after-retire"
                    | "after-recovery-link"
                    | "after-witness-quarantine"
                    | "compare-before-second-member"
                    | "compare-before-committed"
                    | "compare-committed"
                    | "preview-worker-ready"
                    | "preview-worker-ignore-term"
            )
        })
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Unknown acceptance checkpoint phase",
            )
        })?;
    let directory = Path::new(directory);
    let (handle, identity) = checkpoint_directory(directory)?;
    let phase = match phase {
        "AfterRetire" | "after-retire" => "after-retire",
        "AfterRecoveryLink" | "after-recovery-link" => "after-recovery-link",
        "AfterWitnessQuarantine" | "after-witness-quarantine" => "after-witness-quarantine",
        "compare-before-second-member" => "compare-before-second-member",
        "compare-before-committed" => "compare-before-committed",
        "compare-committed" => "compare-committed",
        _ => return Ok(()),
    };
    if phase != requested {
        return Ok(());
    }
    write_marker(
        directory,
        &handle,
        &identity,
        serde_json::json!({"schemaVersion":1,"pid":std::process::id(),"phase":phase,"target":target,"recoveryDirectory":recovery_dir}),
        ready,
    )
}

fn checkpoint_directory(directory: &Path) -> io::Result<(File, Metadata)> {
    let identity = fs::symlink_metadata(directory)?;
    private_metadata(&identity)?;
    let handle = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(directory)?;
    verify_directory(directory, &handle, &identity)?;
    Ok((handle, identity))
}

fn write_marker(
    directory: &Path,
    handle: &File,
    identity: &Metadata,
    payload: serde_json::Value,
    ready: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    let name = c"checkpoint.json";
    let fd = unsafe {
        libc::openat(
            handle.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let mut marker = unsafe { File::from_raw_fd(fd) };
    let created = marker.metadata()?;
    verify_directory(directory, handle, identity)?;
    let mut bytes = serde_json::to_vec(&payload)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    bytes.push(b'\n');
    marker.write_all(&bytes)?;
    marker.flush()?;
    marker.sync_all()?;
    handle.sync_all()?;
    marker.seek(SeekFrom::Start(0))?;
    let mut actual = Vec::new();
    (&mut marker)
        .take(bytes.len() as u64 + 1)
        .read_to_end(&mut actual)?;
    let current = marker.metadata()?;
    let mut entry = std::mem::MaybeUninit::<libc::stat>::uninit();
    if unsafe {
        libc::fstatat(
            handle.as_raw_fd(),
            name.as_ptr(),
            entry.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    let entry = unsafe { entry.assume_init() };
    if !current.is_file()
        || current.uid() != unsafe { libc::geteuid() }
        || current.mode() & 0o7777 != 0o600
        || current.nlink() != 1
        || current.dev() != created.dev()
        || current.ino() != created.ino()
        || entry.st_dev as u64 != created.dev()
        || entry.st_ino != created.ino()
        || entry.st_nlink != 1
        || entry.st_mode & libc::S_IFMT != libc::S_IFREG
        || actual != bytes
    {
        return Err(io::Error::other(
            "Acceptance checkpoint marker binding changed",
        ));
    }
    verify_directory(directory, handle, identity)?;
    ready()
}

fn private_metadata(metadata: &Metadata) -> io::Result<()> {
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o7777 != 0o700
        || metadata.ino() == 0
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Acceptance checkpoint directory must be an existing owned ordinary 0700 directory",
        ));
    }
    Ok(())
}

fn verify_directory(path: &Path, file: &File, identity: &Metadata) -> io::Result<()> {
    let current = fs::symlink_metadata(path)?;
    let descriptor = file.metadata()?;
    private_metadata(&current)?;
    private_metadata(&descriptor)?;
    if current.dev() != identity.dev()
        || current.ino() != identity.ino()
        || descriptor.dev() != identity.dev()
        || descriptor.ino() != identity.ino()
    {
        return Err(io::Error::other(
            "Acceptance checkpoint directory binding changed",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "phoenix-acceptance-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            Self(path)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn invoke(
        root: &Path,
        acceptance: Option<&OsStr>,
        directory: Option<&OsStr>,
        phase: Option<&OsStr>,
    ) -> io::Result<()> {
        checkpoint_with(
            "AfterRetire",
            &root.join("artifact.html"),
            &root.join("recovery"),
            acceptance,
            directory,
            phase,
            || {
                Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "test ready boundary",
                ))
            },
        )
    }

    #[test]
    fn an_incomplete_gate_performs_no_filesystem_or_wait_action() {
        let root = Temp::new();
        let missing = root.0.join("missing");
        for (acceptance, directory, phase) in [
            (
                None,
                Some(missing.as_os_str()),
                Some(OsStr::new("after-retire")),
            ),
            (
                Some(OsStr::new("0")),
                Some(missing.as_os_str()),
                Some(OsStr::new("unknown")),
            ),
            (
                Some(OsStr::new("1")),
                None,
                Some(OsStr::new("after-retire")),
            ),
            (Some(OsStr::new("1")), Some(missing.as_os_str()), None),
        ] {
            invoke(&root.0, acceptance, directory, phase).unwrap();
        }
        assert_eq!(fs::read_dir(&root.0).unwrap().count(), 0);
    }

    #[test]
    fn an_unknown_enabled_phase_is_rejected_without_creating_a_marker() {
        let root = Temp::new();
        let error = invoke(
            &root.0,
            Some(OsStr::new("1")),
            Some(root.0.as_os_str()),
            Some(OsStr::new("arbitrary-command")),
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(fs::read_dir(&root.0).unwrap().count(), 0);
    }

    #[test]
    fn unknown_directory_permissions_and_symlinks_fail_closed() {
        let root = Temp::new();
        let missing = root.0.join("missing");
        assert!(invoke(
            &root.0,
            Some(OsStr::new("1")),
            Some(missing.as_os_str()),
            Some(OsStr::new("after-retire"))
        )
        .is_err());
        let public = root.0.join("public");
        fs::create_dir(&public).unwrap();
        fs::set_permissions(&public, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(invoke(
            &root.0,
            Some(OsStr::new("1")),
            Some(public.as_os_str()),
            Some(OsStr::new("after-retire"))
        )
        .is_err());
        let alias = root.0.join("alias");
        std::os::unix::fs::symlink(&root.0, &alias).unwrap();
        assert!(invoke(
            &root.0,
            Some(OsStr::new("1")),
            Some(alias.as_os_str()),
            Some(OsStr::new("after-retire"))
        )
        .is_err());
        assert!(!root.0.join("checkpoint.json").exists());
    }

    #[test]
    fn comparison_phases_select_exact_durable_pair_boundaries() {
        for phase in [
            "compare-before-second-member",
            "compare-before-committed",
            "compare-committed",
        ] {
            let root = Temp::new();
            let mut paused = false;
            checkpoint_with(
                phase,
                &root.0.join("comparison.html"),
                &root.0.join("pair-journal"),
                Some(OsStr::new("1")),
                Some(root.0.as_os_str()),
                Some(OsStr::new(phase)),
                || {
                    paused = true;
                    Ok(())
                },
            )
            .unwrap();
            assert!(paused);
            let marker: serde_json::Value =
                serde_json::from_slice(&fs::read(root.0.join("checkpoint.json")).unwrap()).unwrap();
            assert_eq!(marker["phase"], phase);
            assert_eq!(
                marker["recoveryDirectory"],
                root.0.join("pair-journal").to_str().unwrap()
            );
        }
    }

    #[test]
    fn a_complete_selected_gate_writes_one_private_bound_marker_then_waits() {
        let root = Temp::new();
        let error = invoke(
            &root.0,
            Some(OsStr::new("1")),
            Some(root.0.as_os_str()),
            Some(OsStr::new("after-retire")),
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        let file = root.0.join("checkpoint.json");
        let metadata = fs::symlink_metadata(&file).unwrap();
        assert!(metadata.is_file());
        assert_eq!(metadata.mode() & 0o7777, 0o600);
        assert_eq!(metadata.nlink(), 1);
        let marker: serde_json::Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        assert_eq!(marker["schemaVersion"], 1);
        assert_eq!(marker["pid"], std::process::id());
        assert_eq!(marker["phase"], "after-retire");
        assert_eq!(
            marker["target"],
            root.0.join("artifact.html").to_str().unwrap()
        );
        assert_eq!(
            marker["recoveryDirectory"],
            root.0.join("recovery").to_str().unwrap()
        );
        let before = fs::read(&file).unwrap();
        assert_eq!(
            invoke(
                &root.0,
                Some(OsStr::new("1")),
                Some(root.0.as_os_str()),
                Some(OsStr::new("after-retire"))
            )
            .unwrap_err()
            .kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read(file).unwrap(), before);
    }
}
