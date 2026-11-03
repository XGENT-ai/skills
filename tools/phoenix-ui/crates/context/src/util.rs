//! Small helpers shared by every verb: JS-flavoured string/number semantics,
//! best-effort fs reads, JSON conveniences.

use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::Path;

pub type Env = HashMap<String, String>;

/// JS `String.prototype.trim()`: WhiteSpace + LineTerminator, incl. U+FEFF.
pub fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
}

pub use impeccable_core::js::{number_to_string as js_number_to_string, to_fixed};
pub use impeccable_core::js_ext_a::utf16_len;

/// A JS number as a JSON value: integral values print without `.0`.
pub fn js_num(v: f64) -> Value {
    if v.is_finite() && v.fract() == 0.0 && v.abs() < 9007199254740992.0 {
        Value::from(v as i64)
    } else if v.is_finite() {
        Value::from(v)
    } else {
        Value::Null
    }
}

pub fn exists(p: &str) -> bool {
    Path::new(p).exists()
}

pub fn is_dir(p: &str) -> bool {
    Path::new(p).is_dir()
}

pub fn is_file(p: &str) -> bool {
    Path::new(p).is_file()
}

/// `fs.readFileSync(p, 'utf-8')` or null.
pub fn safe_read(p: &str) -> Option<String> {
    std::fs::read(p).ok().map(|b| String::from_utf8_lossy(&b).into_owned())
}

/// `JSON.parse(fs.readFileSync(p))` or null.
pub fn read_json(p: &str) -> Option<Value> {
    let text = safe_read(p)?;
    serde_json::from_str::<Value>(&text).ok()
}

/// `readdirSync` names in directory order (Node returns them sorted by the
/// OS; on macOS/Linux this is not guaranteed sorted, so callers that sort do
/// so explicitly). We sort by byte order for determinism where JS output
/// depends on order without sorting.
pub fn read_dir_names(p: &str) -> Option<Vec<String>> {
    let rd = std::fs::read_dir(p).ok()?;
    let mut names: Vec<String> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    Some(names)
}

pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub is_file: bool,
}

/// `readdirSync(p, { withFileTypes: true })`, sorted by name.
pub fn read_dir_entries(p: &str) -> Option<Vec<DirEntry>> {
    let rd = std::fs::read_dir(p).ok()?;
    let mut out: Vec<DirEntry> = rd
        .filter_map(|e| e.ok())
        .map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            // Node's Dirent reports the link type, not the target's; follow
            // symlinks like Node does when the entry is a symlink? Node's
            // withFileTypes reports isSymbolicLink() for links, so isDirectory()
            // is false for symlinked dirs. Mirror that.
            let ft = e.file_type().ok();
            let (is_dir, is_file) = match ft {
                Some(t) if t.is_symlink() => (false, false),
                Some(t) => (t.is_dir(), t.is_file()),
                None => (false, false),
            };
            DirEntry { name, is_dir, is_file }
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Some(out)
}

pub fn mtime_ms(p: &str) -> Option<f64> {
    let md = std::fs::metadata(p).ok()?;
    let t = md.modified().ok()?;
    let d = t.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(d.as_secs_f64() * 1000.0)
}

pub use impeccable_common::{iso_from_ms, iso_now, now_ms};

/// `JSON.stringify(v, null, 2)`
pub fn json_pretty(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|_| "null".into())
}

/// `JSON.stringify(v)`
pub fn json_compact(v: &Value) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "null".into())
}

pub fn obj() -> Map<String, Value> {
    Map::new()
}

pub fn opt_string(s: &Option<String>) -> Value {
    match s {
        Some(v) => Value::String(v.clone()),
        None => Value::Null,
    }
}

/// `os.homedir()` as Node computes it on posix: $HOME first.
pub fn homedir(env: &Env) -> String {
    if cfg!(windows) {
        // Node win32: USERPROFILE, then the process's own profile dir.
        if let Some(h) = env.get("USERPROFILE") {
            if !h.is_empty() {
                return h.clone();
            }
        }
        return std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string());
    }
    if let Some(h) = env.get("HOME") {
        if !h.is_empty() {
            return h.clone();
        }
    }
    if let Some(h) = env.get("USERPROFILE") {
        if !h.is_empty() {
            return h.clone();
        }
    }
    // Fallback: the process's real home.
    std::env::var("HOME").unwrap_or_else(|_| "/".to_string())
}

/// hook-lib `truthy()`: /^(1|true|yes|on)$/i on the trimmed value.
pub fn truthy_env(env: &Env, key: &str) -> bool {
    match env.get(key) {
        Some(v) => matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"),
        None => false,
    }
}

/// Node's ENOENT-style error message for a failed read.
pub fn node_read_error(p: &str, err: &std::io::Error) -> String {
    match err.kind() {
        std::io::ErrorKind::NotFound => format!("ENOENT: no such file or directory, open '{}'", p),
        std::io::ErrorKind::PermissionDenied => format!("EACCES: permission denied, open '{}'", p),
        _ => {
            if err.raw_os_error() == Some(21) {
                format!("EISDIR: illegal operation on a directory, read")
            } else {
                format!("{}", err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed() {
        assert_eq!(to_fixed(2.5, 0), "3");
        assert_eq!(to_fixed(0.1234, 3), "0.123");
        assert_eq!(to_fixed(1.0005, 3), "1.000"); // 1.0005 is below the tie in binary
        assert_eq!(to_fixed(22.5, 0), "23");
        assert_eq!(to_fixed(0.65, 3), "0.650");
        assert_eq!(to_fixed(359.99, 1), "360.0");
        assert_eq!(to_fixed(9.5, 0), "10");
        assert_eq!(to_fixed(99.999, 2), "100.00");
        assert_eq!(to_fixed(-99.999, 2), "-100.00");
        assert_eq!(to_fixed(-0.0001, 2), "-0.00");
        assert_eq!(to_fixed(-0.0, 2), "0.00");
    }
    #[test]
    fn iso() {
        assert_eq!(iso_from_ms(0.0), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso_from_ms(1778610600123.0), "2026-05-12T18:30:00.123Z");
    }
}
