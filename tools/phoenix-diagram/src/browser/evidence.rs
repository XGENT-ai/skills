//! Exclusive screenshot staging and descriptor-backed input identities.
use super::cdp::{Error, OwnedDirectory};
use super::{Screenshot, Theme, Viewport, DESKTOP_VIEWPORTS};
use crate::check::FileIdentity;
use base64::Engine;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
struct PhysicalFile {
    dev: u64,
    ino: u64,
    links: u64,
    mode: u32,
}
impl PhysicalFile {
    fn of(stat: &fs::Metadata) -> Self {
        Self {
            dev: stat.dev(),
            ino: stat.ino(),
            links: stat.nlink(),
            mode: stat.mode(),
        }
    }
    fn directory(stat: &fs::Metadata) -> Self {
        let mut result = Self::of(stat);
        result.links = 0;
        result
    }
}

pub(super) struct BoundRead {
    path: PathBuf,
    parent: PathBuf,
    parent_id: PhysicalFile,
    identity: FileIdentity,
    physical: PhysicalFile,
    _file: File,
    code: &'static str,
}

impl BoundRead {
    pub fn capture(
        path: &Path,
        expected: &FileIdentity,
        code: &'static str,
    ) -> Result<Self, Error> {
        let failure =
            |message: String| Error::new(code, "binding", format!("{}: {message}", path.display()));
        let parent = path
            .parent()
            .ok_or_else(|| failure("missing parent directory".into()))?;
        let canonical_parent = fs::canonicalize(parent).map_err(|e| failure(e.to_string()))?;
        let parent_id = PhysicalFile::directory(
            &fs::metadata(&canonical_parent).map_err(|e| failure(e.to_string()))?,
        );
        let mut file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)
            .map_err(|e| failure(e.to_string()))?;
        let before = file.metadata().map_err(|e| failure(e.to_string()))?;
        if !before.is_file() || before.ino() == 0 {
            return Err(failure(
                "expected an ordinary file with a physical identity".into(),
            ));
        }
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .map_err(|e| failure(e.to_string()))?;
        let after = file.metadata().map_err(|e| failure(e.to_string()))?;
        if PhysicalFile::of(&before) != PhysicalFile::of(&after)
            || before.len() != after.len()
            || before.mtime() != after.mtime()
            || before.mtime_nsec() != after.mtime_nsec()
        {
            return Err(failure("file changed during its read".into()));
        }
        let identity = FileIdentity::of(&contents);
        if identity != *expected {
            return Err(failure(
                "bytes or SHA-256 differ from the bound delivery".into(),
            ));
        }
        let physical = PhysicalFile::of(&after);
        let entry = fs::symlink_metadata(path).map_err(|e| failure(e.to_string()))?;
        if !entry.is_file()
            || entry.file_type().is_symlink()
            || PhysicalFile::of(&entry) != physical
            || fs::canonicalize(parent).map_err(|e| failure(e.to_string()))? != canonical_parent
        {
            return Err(failure("the file slot or parent changed".into()));
        }
        Ok(Self {
            path: path.to_owned(),
            parent: canonical_parent,
            parent_id,
            identity,
            physical,
            _file: file,
            code,
        })
    }
    pub fn verify(&self) -> Result<(), Error> {
        let current = Self::capture(&self.path, &self.identity, self.code)?;
        if current.physical != self.physical
            || current.parent != self.parent
            || current.parent_id != self.parent_id
        {
            return Err(Error::new(
                self.code,
                "binding",
                format!(
                    "The physical file or parent changed: {}",
                    self.path.display()
                ),
            ));
        }
        Ok(())
    }
}

struct Pending {
    stage: PathBuf,
    destination: PathBuf,
    identity: FileIdentity,
    physical: PhysicalFile,
    viewport: Viewport,
    theme: Theme,
    full_page: bool,
    pixel_width: u32,
    pixel_height: u32,
    published: bool,
}

pub(super) struct Evidence {
    directory: PathBuf,
    directory_id: PhysicalFile,
    created_directory: bool,
    stage: OwnedDirectory,
    targets: Vec<(Viewport, Theme, PathBuf)>,
    entries: Vec<Pending>,
    done: bool,
}

impl Evidence {
    pub fn reserve(artifact: &Path, out_dir: Option<&Path>) -> Result<Self, Error> {
        let requested = out_dir.or_else(|| artifact.parent()).ok_or_else(|| {
            Error::new(
                "browser/evidence-path",
                "evidence",
                "No screenshot directory",
            )
        })?;
        let created_directory = match fs::symlink_metadata(requested) {
            Ok(stat) if stat.is_dir() && !stat.file_type().is_symlink() => false,
            Ok(_) => {
                return Err(Error::new(
                    "browser/evidence-conflict",
                    "evidence",
                    format!(
                        "Screenshot directory is not an ordinary directory: {}",
                        requested.display()
                    ),
                ))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                fs::DirBuilder::new()
                    .mode(0o700)
                    .create(requested)
                    .map_err(|e| Error::new("browser/evidence-path", "evidence", e.to_string()))?;
                true
            }
            Err(e) => {
                return Err(Error::new(
                    "browser/evidence-path",
                    "evidence",
                    e.to_string(),
                ))
            }
        };
        let directory = fs::canonicalize(requested)
            .map_err(|e| Error::new("browser/evidence-path", "evidence", e.to_string()))?;
        let directory_id = PhysicalFile::directory(
            &fs::symlink_metadata(&directory)
                .map_err(|e| Error::new("browser/evidence-path", "evidence", e.to_string()))?,
        );
        let stem = crate::delivery::bounded_sidecar_stem(artifact);
        let mut targets = Vec::new();
        for viewport in [DESKTOP_VIEWPORTS[0], DESKTOP_VIEWPORTS[3]] {
            for theme in [Theme::Light, Theme::Dark] {
                let path = directory.join(format!(
                    "{stem}.visual-check.{}x{}.{}.png",
                    viewport.width,
                    viewport.height,
                    theme.as_str()
                ));
                match fs::symlink_metadata(&path) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    _ => {
                        return Err(Error::new(
                            "browser/evidence-conflict",
                            "evidence",
                            format!(
                                "Screenshot evidence already occupies {}; choose a fresh --out-dir",
                                path.display()
                            ),
                        ))
                    }
                }
                targets.push((viewport, theme, path));
            }
        }
        let stage = OwnedDirectory::create(&directory, "phoenix-diagram-browser-evidence-")?;
        Ok(Self {
            directory,
            directory_id,
            created_directory,
            stage,
            targets,
            entries: Vec::new(),
            done: false,
        })
    }
    fn verify_directory(&self) -> Result<(), Error> {
        let stat = fs::symlink_metadata(&self.directory)
            .map_err(|e| Error::new("browser/evidence-changed", "evidence", e.to_string()))?;
        if !stat.is_dir() || PhysicalFile::directory(&stat) != self.directory_id {
            return Err(Error::new(
                "browser/evidence-changed",
                "evidence",
                "Screenshot directory identity changed",
            ));
        }
        self.stage.verify()
    }
    pub fn requires_capture(&self, viewport: Viewport, theme: Theme) -> bool {
        self.targets
            .iter()
            .any(|(v, t, _)| *v == viewport && *t == theme)
    }
    pub fn stage_png(
        &mut self,
        viewport: Viewport,
        theme: Theme,
        full_page: bool,
        encoded: &str,
    ) -> Result<(), Error> {
        self.verify_directory()?;
        let destination = self
            .targets
            .iter()
            .find(|(v, t, _)| *v == viewport && *t == theme)
            .map(|(_, _, path)| path.clone())
            .ok_or_else(|| {
                Error::new(
                    "browser/capture",
                    "capture",
                    "Unexpected screenshot viewport",
                )
            })?;
        if self
            .entries
            .iter()
            .any(|entry| entry.destination == destination)
        {
            return Err(Error::new(
                "browser/capture",
                "capture",
                "Duplicate screenshot viewport",
            ));
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| {
                Error::new(
                    "browser/capture",
                    "capture",
                    format!("Invalid PNG base64: {e}"),
                )
            })?;
        if bytes.len() < 33 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" || &bytes[12..16] != b"IHDR" {
            return Err(Error::new(
                "browser/capture",
                "capture",
                "Chrome returned no PNG image",
            ));
        }
        let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
        if width != viewport.width
            || (!full_page && height != viewport.height)
            || full_page && height < viewport.height
        {
            return Err(Error::new(
                "browser/capture",
                "capture",
                format!(
                    "PNG dimensions {width}x{height} do not cover requested {}x{}",
                    viewport.width, viewport.height
                ),
            ));
        }
        let stage = self.stage.path.join(format!("{}.png", self.entries.len()));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&stage)
            .map_err(|e| Error::new("browser/evidence-write", "capture", e.to_string()))?;
        let physical = PhysicalFile::of(
            &file
                .metadata()
                .map_err(|e| Error::new("browser/evidence-write", "capture", e.to_string()))?,
        );
        let identity = FileIdentity::of(&bytes);
        self.entries.push(Pending {
            stage: stage.clone(),
            destination,
            identity,
            physical,
            viewport,
            theme,
            full_page,
            pixel_width: width,
            pixel_height: height,
            published: false,
        });
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| Error::new("browser/evidence-write", "capture", e.to_string()))?;
        Ok(())
    }
    pub fn publish(&mut self) -> Result<Vec<Screenshot>, Error> {
        if self.entries.len() != 4 {
            return Err(Error::new(
                "browser/capture-incomplete",
                "capture",
                "The four required screenshots were not all captured",
            ));
        }
        let mut screenshots = Vec::new();
        for index in 0..self.entries.len() {
            self.verify_directory()?;
            let entry = &mut self.entries[index];
            verify_owned(&entry.stage, &entry.physical, &entry.identity, 1)?;
            fs::hard_link(&entry.stage, &entry.destination).map_err(|e| {
                Error::new(
                    if e.kind() == std::io::ErrorKind::AlreadyExists {
                        "browser/evidence-conflict"
                    } else {
                        "browser/evidence-write"
                    },
                    "evidence",
                    format!(
                        "Could not exclusively publish {}: {e}",
                        entry.destination.display()
                    ),
                )
            })?;
            entry.published = true;
            verify_owned(&entry.destination, &entry.physical, &entry.identity, 2)?;
            verify_owned(&entry.stage, &entry.physical, &entry.identity, 2)?;
            fs::remove_file(&entry.stage)
                .map_err(|e| Error::new("browser/evidence-cleanup", "evidence", e.to_string()))?;
            verify_owned(&entry.destination, &entry.physical, &entry.identity, 1)?;
            screenshots.push(Screenshot {
                path: entry.destination.clone(),
                identity: entry.identity.clone(),
                viewport: entry.viewport,
                width: entry.pixel_width,
                height: entry.pixel_height,
                theme: entry.theme,
                full_page: entry.full_page,
            });
        }
        File::open(&self.directory)
            .and_then(|file| file.sync_all())
            .map_err(|e| Error::new("browser/evidence-write", "evidence", e.to_string()))?;
        for entry in &self.entries {
            verify_owned(&entry.destination, &entry.physical, &entry.identity, 1)?;
        }
        self.stage.remove_empty()?;
        self.done = true;
        Ok(screenshots)
    }
    pub fn cleanup(&mut self) -> Vec<Error> {
        if self.done {
            return Vec::new();
        }
        let mut errors = Vec::new();
        if let Err(error) = self.verify_directory() {
            return vec![error];
        }
        for entry in &self.entries {
            for path in [&entry.stage, &entry.destination] {
                if path == &entry.destination && !entry.published {
                    continue;
                }
                match fs::symlink_metadata(path) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                    Ok(stat)
                        if stat.is_file()
                            && !stat.file_type().is_symlink()
                            && stat.nlink() <= 2 =>
                    {
                        if let Err(error) =
                            verify_owned(path, &entry.physical, &entry.identity, stat.nlink())
                        {
                            errors.push(error);
                            continue;
                        }
                        if let Err(error) = fs::remove_file(path) {
                            errors.push(Error::new(
                                "browser/evidence-cleanup",
                                "cleanup",
                                format!("{}: {error}", path.display()),
                            ));
                        }
                    }
                    _ => errors.push(Error::new(
                        "browser/evidence-changed",
                        "cleanup",
                        format!(
                            "Uncertain screenshot entry was preserved: {}",
                            path.display()
                        ),
                    )),
                }
            }
        }
        if let Err(error) = self.stage.remove_empty() {
            errors.push(error);
        }
        if self.created_directory {
            if let Ok(stat) = fs::symlink_metadata(&self.directory) {
                if PhysicalFile::directory(&stat) == self.directory_id {
                    if let Err(error) = fs::remove_dir(&self.directory) {
                        errors.push(Error::new(
                            "browser/evidence-cleanup",
                            "cleanup",
                            format!(
                                "Screenshot directory was preserved at {}: {error}",
                                self.directory.display()
                            ),
                        ));
                    }
                }
            }
        }
        if errors.is_empty() {
            self.done = true;
        }
        errors
    }
}
impl Drop for Evidence {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

fn verify_owned(
    path: &Path,
    physical: &PhysicalFile,
    identity: &FileIdentity,
    links: u64,
) -> Result<(), Error> {
    let bound = BoundRead::capture(path, identity, "browser/evidence-changed")?;
    let mut expected = physical.clone();
    expected.links = links;
    if bound.physical != expected {
        return Err(Error::new(
            "browser/evidence-changed",
            "evidence",
            format!("Screenshot physical identity changed: {}", path.display()),
        ));
    }
    Ok(())
}
