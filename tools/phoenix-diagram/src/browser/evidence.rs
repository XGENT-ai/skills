//! Screenshot files for the image review.
use super::cdp::Error;
use super::{Screenshot, Theme, Viewport, DESKTOP_VIEWPORTS};
use crate::check::FileIdentity;
use base64::Engine;
use std::path::{Path, PathBuf};

/// The review looks at the smallest and largest desktop sizes in both themes.
pub(super) fn requires_capture(viewport: Viewport, _theme: Theme) -> bool {
    viewport == DESKTOP_VIEWPORTS[0] || viewport == DESKTOP_VIEWPORTS[3]
}

pub(super) struct Evidence {
    directory: PathBuf,
    stem: String,
    pub screenshots: Vec<Screenshot>,
}

impl Evidence {
    /// Screenshots go to `out_dir`, or beside the HTML, named after it and the
    /// comparison view when there is one.
    pub fn new(artifact: &Path, out_dir: Option<&Path>, view: Option<&str>) -> Result<Self, Error> {
        let directory = out_dir
            .map(Path::to_owned)
            .or_else(|| artifact.parent().map(Path::to_owned))
            .ok_or_else(|| {
                Error::new(
                    "browser/evidence-path",
                    "evidence",
                    "No screenshot directory",
                )
            })?;
        let directory = std::fs::create_dir_all(&directory)
            .and_then(|_| std::fs::canonicalize(&directory))
            .map_err(|e| Error::new("browser/evidence-path", "evidence", e.to_string()))?;
        let name = artifact.file_name().unwrap_or_default().to_string_lossy();
        let mut stem = name.strip_suffix(".html").unwrap_or(&name).to_owned();
        if let Some(view) = view {
            stem = format!("{stem}.{view}");
        }
        Ok(Self {
            directory,
            stem,
            screenshots: Vec::new(),
        })
    }

    pub fn write_png(
        &mut self,
        viewport: Viewport,
        theme: Theme,
        full_page: bool,
        encoded: &str,
    ) -> Result<(), Error> {
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
        let path = self.directory.join(format!(
            "{}.{}x{}.{}.png",
            self.stem,
            viewport.width,
            viewport.height,
            theme.as_str()
        ));
        crate::output::write(&path, &bytes)
            .map_err(|e| Error::new("browser/evidence-write", "capture", e.to_string()))?;
        self.screenshots.push(Screenshot {
            path,
            identity: FileIdentity::of(&bytes),
            viewport,
            width,
            height,
            theme,
            full_page,
        });
        Ok(())
    }
}
