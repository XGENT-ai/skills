//! Executable-mode preservation for locally distributed launchers and engines.
//! Engine acquisition belongs to the explicit Node distribution command.

use crate::providers::Sys;
use crate::util::{self, jsp};
use impeccable_common::Io;

/// Normalize this build's target tag. Asset availability is established by
/// the explicit distribution manifest, not by this platform mapping.
pub fn platform_tag() -> Option<(&'static str, &'static str)> {
    let os = if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        return None;
    };
    let arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else if cfg!(target_arch = "x86_64") {
        "x64"
    } else {
        return None;
    };
    Some((os, arch))
}

/// Where the binary for `(os, arch)` lives inside a skill directory.
pub fn binary_path(skill_dir: &str, os: &str, arch: &str) -> String {
    let name = if os == "windows" {
        "phoenix-ui.exe"
    } else {
        "phoenix-ui"
    };
    jsp::join(&[skill_dir, "scripts", "bin", &format!("{os}-{arch}"), name])
}

/// Preserve executability after copying a verified local payload. Never fetch.
pub fn ensure_executable_scripts(skill_dir: &str) {
    let launcher = jsp::join(&[skill_dir, "scripts", "phoenix-ui"]);
    if util::is_file(&launcher) && !util::is_symlink(&launcher) {
        let _ = util::set_executable(&launcher);
    }
}

/// Kept as the existing copy/refresh integration point; no network fallback.
pub fn install_engine_binaries(_sys: &Sys, _io: &mut Io, skill_dirs: &[String]) {
    for skill_dir in skill_dirs {
        ensure_executable_scripts(skill_dir);
    }
}
