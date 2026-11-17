use serde_json::{json, Map, Value};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

const INDEX_SCHEMA: &str = "phoenix-ui-media-index-v1";
const SOURCES_SCHEMA: &str = "phoenix-ui-media-sources-v1";
static SERIAL: AtomicU64 = AtomicU64::new(0);

pub(super) fn resolve_area(cwd: &Path, dir: &str) -> Result<PathBuf, String> {
    resolve(cwd, dir, true)
}

pub(super) fn resolve_readonly(cwd: &Path, dir: &str) -> Result<PathBuf, String> {
    resolve(cwd, dir, false)
}

fn resolve(cwd: &Path, dir: &str, create: bool) -> Result<PathBuf, String> {
    let base = normalize(&cwd.join(".phoenix-ui"));
    let area = normalize(&cwd.join(dir));
    let outside = || {
        format!("素材只取到 .phoenix-ui/ 下的素材区，只用于设计稿与方案预览，不进入产品：{} 在 .phoenix-ui/ 之外", area.display())
    };
    if !area.starts_with(&base) {
        return Err(outside());
    }
    // Check existing ancestors before mkdir: an escaping symlink must not create
    // directories outside the area. The physical root is anchored to the canonicalized
    // project's own `.phoenix-ui`, so a symlinked root cannot redirect the area into
    // another directory inside the project (e.g. `public/`).
    let real_cwd = cwd.canonicalize().map_err(|e| e.to_string())?;
    let root = real_cwd.join(".phoenix-ui");
    let mut parent = area.as_path();
    while !parent.exists() {
        parent = parent.parent().ok_or_else(outside)?;
    }
    let real_parent = parent.canonicalize().map_err(|e| e.to_string())?;
    if !real_parent.starts_with(&root) && base.exists() {
        // The only allowed exception is a media root that does not exist yet:
        // then nothing under `base` exists and the nearest existing ancestor is
        // the project directory itself. Once the root or any path under it
        // exists, its real location must already be inside the anchored root,
        // so a link jumping back to the project (`.phoenix-ui -> .` or an
        // up-jump inside the root) is rejected before any directory is created
        // and before a readonly area is accepted.
        return Err(outside());
    }
    if !create {
        return Ok(area);
    }
    fs::create_dir_all(&area).map_err(|e| e.to_string())?;
    let real_area = area.canonicalize().map_err(|e| e.to_string())?;
    if !real_area.starts_with(&root) {
        return Err(outside());
    }
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(area.join(".gitignore"))
    {
        Ok(mut f) => f.write_all(b"*\n").map_err(|e| e.to_string())?,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e.to_string()),
    }
    Ok(area)
}

fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            c => out.push(c.as_os_str()),
        }
    }
    out
}

pub(super) struct Lock(PathBuf);
impl Lock {
    pub(super) fn acquire(area: &Path) -> Result<Self, String> {
        let file = area.join("index.lock");
        let start = Instant::now();
        loop {
            match OpenOptions::new().write(true).create_new(true).open(&file) {
                Ok(_) => return Ok(Self(file)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    if start.elapsed() >= Duration::from_secs(5) {
                        return Err(format!(
                            "另一个 media 命令正在写索引；确认没有命令在运行后删除 {}",
                            file.display()
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(e) => return Err(format!("{}: {e}", file.display())),
            }
        }
    }
}
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

pub(super) fn temporary_name() -> String {
    format!(
        ".{}-{}-{}.part",
        std::process::id(),
        impeccable_common::now_ms(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    )
}

pub(super) fn atomic_write(area: &Path, name: &str, bytes: &[u8]) -> Result<(), String> {
    let temp = area.join(temporary_name());
    let result = (|| {
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        f.write_all(bytes).map_err(|e| e.to_string())?;
        fs::rename(&temp, area.join(name)).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn read(area: &Path, name: &str, schema: &str) -> Result<Option<Value>, String> {
    let path = area.join(name);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let data: Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("{} 无法读取：{e}", path.display()))?;
    if data["schema"] != schema {
        return Err(format!("{} 的 schema 不受支持", path.display()));
    }
    Ok(Some(data))
}

pub(super) fn read_index(area: &Path) -> Result<Map<String, Value>, String> {
    let Some(data) = read(area, "index.json", INDEX_SCHEMA)? else {
        return Ok(Map::new());
    };
    let items = data["items"]
        .as_object()
        .ok_or("index.json 的 items 应为对象")?;
    for (key, item) in items {
        if item["ref"] != *key
            || item["sourceId"].as_str().is_none()
            || prefix(item["provider"].as_str().unwrap_or("")).is_none()
        {
            return Err(format!("index.json 的条目 {key} 无效"));
        }
    }
    Ok(items.clone())
}

fn prefix(provider: &str) -> Option<char> {
    Some(match provider {
        "polyhaven" => 'T',
        "vam" => 'V',
        "loc" => 'L',
        "wellcome" => 'W',
        "nasa" => 'N',
        "commons" => 'C',
        "met" => 'M',
        "aic" => 'A',
        "cleveland" => 'K',
        "japan" => 'J',
        "rijks" => 'R',
        _ => return None,
    })
}

pub(super) fn remember(area: &Path, items: Vec<Value>) -> Result<Vec<Value>, String> {
    let _lock = Lock::acquire(area)?;
    let mut saved = read_index(area)?;
    let mut result = Vec::new();
    for mut item in items {
        let provider = item["provider"].as_str().ok_or("条目缺少 provider")?;
        let source = item["sourceId"].as_str().ok_or("条目缺少 sourceId")?;
        let prefix = prefix(provider).ok_or_else(|| format!("未知信源 {provider}"))?;
        let existing = saved
            .iter()
            .find(|(_, v)| v["provider"] == provider && v["sourceId"] == source)
            .map(|(k, _)| k.clone());
        let reference = match existing {
            Some(reference) => reference,
            None => {
                let highest = saved
                    .keys()
                    .filter_map(|k| k.strip_prefix(prefix)?.parse::<u64>().ok())
                    .max()
                    .unwrap_or(0);
                format!(
                    "{prefix}{}",
                    highest.checked_add(1).ok_or("素材编号已用尽")?
                )
            }
        };
        item["ref"] = json!(reference);
        item["cachedAt"] = json!(impeccable_common::iso_now());
        saved.insert(reference, item.clone());
        result.push(item);
    }
    atomic_write(
        area,
        "index.json",
        &serde_json::to_vec_pretty(&json!({"schema":INDEX_SCHEMA,"items":saved}))
            .map_err(|e| e.to_string())?,
    )?;
    Ok(result)
}

pub(super) fn read_sources(area: &Path) -> Result<Vec<Value>, String> {
    let Some(data) = read(area, "sources.json", SOURCES_SCHEMA)? else {
        return Ok(Vec::new());
    };
    data["files"]
        .as_array()
        .cloned()
        .ok_or_else(|| "sources.json 的 files 应为数组".into())
}

pub(super) fn save_sources(area: &Path, files: &[Value]) -> Result<(), String> {
    let _lock = Lock::acquire(area)?;
    let mut saved = read_sources(area)?;
    for item in files {
        let mut record = item.clone();
        if let Some(obj) = record.as_object_mut() {
            obj.remove("warnings");
        }
        if let Some(old) = saved.iter_mut().find(|v| v["file"] == record["file"]) {
            *old = record;
        } else {
            saved.push(record);
        }
    }
    atomic_write(
        area,
        "sources.json",
        &serde_json::to_vec_pretty(&json!({"schema":SOURCES_SCHEMA,"files":saved}))
            .map_err(|e| e.to_string())?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, thread};
    fn project() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "phoenix-media-index-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }
    #[test]
    fn area_rejects_escape_and_preserves_existing_ignore() {
        let cwd = project();
        assert!(resolve_area(&cwd, "../assets").is_err());
        assert!(!cwd.join(".phoenix-ui").exists());
        let area = resolve_area(&cwd, ".phoenix-ui/media").unwrap();
        assert_eq!(fs::read_to_string(area.join(".gitignore")).unwrap(), "*\n");
        fs::write(area.join(".gitignore"), "custom\n").unwrap();
        resolve_area(&cwd, ".phoenix-ui/media").unwrap();
        assert_eq!(
            fs::read_to_string(area.join(".gitignore")).unwrap(),
            "custom\n"
        );
        #[cfg(unix)]
        {
            fs::create_dir_all(cwd.join("public")).unwrap();
            std::os::unix::fs::symlink(cwd.join("public"), cwd.join(".phoenix-ui/escape")).unwrap();
            assert!(resolve_area(&cwd, ".phoenix-ui/escape/new").is_err());
            assert!(!cwd.join("public/new").exists());
        }
        fs::remove_dir_all(cwd).unwrap();
    }
    #[test]
    #[cfg(unix)]
    fn area_rejects_root_symlink_to_product_directory() {
        let cwd = project();
        fs::create_dir_all(cwd.join("public")).unwrap();
        std::os::unix::fs::symlink(cwd.join("public"), cwd.join(".phoenix-ui")).unwrap();
        let err = resolve_area(&cwd, ".phoenix-ui/media").unwrap_err();
        assert!(err.contains("在 .phoenix-ui/ 之外"));
        assert!(!cwd.join("public/media").exists());
        assert!(resolve_readonly(&cwd, ".phoenix-ui/media").is_err());
        fs::remove_dir_all(cwd).unwrap();
    }
    #[test]
    #[cfg(unix)]
    fn area_rejects_existing_links_to_project_ancestors() {
        let cwd = project();
        // root link back to the project itself
        std::os::unix::fs::symlink(".", cwd.join(".phoenix-ui")).unwrap();
        assert!(resolve_area(&cwd, ".phoenix-ui/public/media").is_err());
        assert!(resolve_readonly(&cwd, ".phoenix-ui/public/media").is_err());
        assert!(!cwd.join("public/media").exists());
        fs::remove_file(cwd.join(".phoenix-ui")).unwrap();
        // first-time initialization of a fresh root still works
        resolve_area(&cwd, ".phoenix-ui/media").unwrap();
        // an up-jump link inside the existing root
        std::os::unix::fs::symlink("..", cwd.join(".phoenix-ui/escape")).unwrap();
        assert!(resolve_area(&cwd, ".phoenix-ui/escape/public/media").is_err());
        assert!(resolve_readonly(&cwd, ".phoenix-ui/escape/public/media").is_err());
        assert!(!cwd.join("public").exists());
        fs::remove_dir_all(cwd).unwrap();
    }
    #[test]
    fn concurrent_remember_preserves_identity_and_refuses_corrupt_index() {
        let cwd = project();
        let area = resolve_area(&cwd, ".phoenix-ui/media").unwrap();
        thread::scope(|scope| {
            for n in 0..4 {
                let area = &area;
                scope.spawn(move || {
                    remember(
                        area,
                        vec![
                            json!({"provider":"commons", "sourceId": n.to_string(), "title":"map"}),
                        ],
                    )
                    .unwrap();
                });
            }
        });
        let value: Value =
            serde_json::from_slice(&fs::read(area.join("index.json")).unwrap()).unwrap();
        assert_eq!(value["items"].as_object().unwrap().len(), 4);
        let old = value["items"]
            .as_object()
            .unwrap()
            .values()
            .find(|v| v["sourceId"] == "0")
            .unwrap();
        let next = remember(
            &area,
            vec![json!({"provider":"commons", "sourceId":"0", "title":"updated"})],
        )
        .unwrap();
        assert_eq!(next[0]["ref"], old["ref"]);
        assert_eq!(next[0]["title"], "updated");
        fs::write(area.join("index.json"), "broken").unwrap();
        assert!(remember(&area, vec![]).is_err());
        assert_eq!(
            fs::read_to_string(area.join("index.json")).unwrap(),
            "broken"
        );
        fs::remove_dir_all(cwd).unwrap();
    }
}
