//! Whole-file output writes: readers see either the old file or the new one.
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Write bytes to a temporary sibling, then rename it over `path`.
pub fn write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    write_with_mode(path, bytes, 0o644)
}

pub fn write_with_mode(path: &Path, bytes: &[u8], mode: u32) -> std::io::Result<()> {
    let name = path
        .file_name()
        .ok_or_else(|| std::io::Error::other("output path must name a file"))?;
    let mut temporary = name.to_owned();
    temporary.push(format!(".{}.tmp", std::process::id()));
    let temporary = path.with_file_name(temporary);
    let written = (|| {
        let mut file = std::fs::File::create(&temporary)?;
        file.set_permissions(std::fs::Permissions::from_mode(mode))?;
        file.write_all(bytes)?;
        std::fs::rename(&temporary, path)
    })();
    if written.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    written
}
