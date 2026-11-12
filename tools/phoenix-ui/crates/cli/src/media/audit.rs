use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use super::index;

pub(super) fn run(area: &Path, cwd: &Path, paths: &[String]) -> Result<Vec<Value>, String> {
    let records = index::read_sources(area)?;
    if records.is_empty() {
        return Ok(Vec::new());
    }
    let area_name = impeccable_common::jsp::relative(
        &cwd.to_string_lossy(),
        &cwd.to_string_lossy(),
        &area.to_string_lossy(),
    );
    let roots: Vec<PathBuf> = if paths.is_empty() {
        vec![cwd.to_path_buf()]
    } else {
        paths
            .iter()
            .map(|p| {
                PathBuf::from(impeccable_common::jsp::normalize(
                    &cwd.join(p).to_string_lossy(),
                ))
            })
            .collect()
    };
    let mut files = Vec::new();
    for root in roots {
        visit(&root, cwd, &mut files)?;
    }
    files.sort();
    files.dedup();
    let mut findings = Vec::new();
    for file in files {
        let bytes = fs::metadata(&file)
            .map_err(|e| format!("{}: {e}", file.display()))?
            .len();
        let name = impeccable_common::jsp::relative(
            &cwd.to_string_lossy(),
            &cwd.to_string_lossy(),
            &file.to_string_lossy(),
        );
        let same_size: Vec<&Value> = records
            .iter()
            .filter(|r| r["bytes"].as_u64() == Some(bytes))
            .collect();
        if !same_size.is_empty() {
            let mut source = File::open(&file).map_err(|e| e.to_string())?;
            let mut hash = Sha256::new();
            let mut buffer = [0_u8; 65536];
            loop {
                let n = source.read(&mut buffer).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                hash.update(&buffer[..n]);
            }
            let digest = format!("{:x}", hash.finalize());
            for record in same_size {
                if record["sha256"] == digest {
                    findings.push(json!({"kind":"copy","file":name,"ref":record["ref"],"match":record["file"]}));
                }
            }
        }
        if bytes > 2 * 1024 * 1024 {
            continue;
        }
        let data = fs::read(&file).map_err(|e| e.to_string())?;
        let Ok(text) = std::str::from_utf8(&data) else {
            continue;
        };
        for (line, content) in text.lines().enumerate() {
            if content.contains(&area_name) {
                let matched: Vec<&Value> = records
                    .iter()
                    .filter(|r| r["file"].as_str().is_some_and(|f| content.contains(f)))
                    .collect();
                let references = if matched.is_empty() {
                    records.iter().collect()
                } else {
                    matched
                };
                for record in references {
                    findings.push(json!({"kind":"reference","file":name,"line":line+1,"ref":record["ref"],"match":area_name}));
                }
            }
            for record in &records {
                let mut matches = Vec::new();
                for key in [
                    "pageUrl",
                    "downloadUrl",
                    "licenseUrl",
                    "license",
                    "attribution",
                ] {
                    if let Some(value) = record[key].as_str().filter(|s| !s.is_empty()) {
                        if content.contains(value) && !matches.contains(&value) {
                            matches.push(value);
                        }
                    }
                }
                for value in matches {
                    findings.push(json!({"kind":"credit","file":name,"line":line+1,"ref":record["ref"],"match":value}));
                }
            }
        }
    }
    Ok(findings)
}

fn visit(path: &Path, cwd: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if path.starts_with(cwd.join(".phoenix-ui")) {
        return Ok(());
    }
    let meta = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if meta.file_type().is_symlink() {
        return Ok(());
    }
    if meta.is_dir() {
        if path
            .file_name()
            .is_some_and(|s| s == ".git" || s == "node_modules")
        {
            return Ok(());
        }
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            visit(&entry.map_err(|e| e.to_string())?.path(), cwd, files)?;
        }
    } else if meta.is_file() {
        files.push(path.to_path_buf());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::fs;
    #[test]
    fn finds_copies_references_and_even_short_credits_then_clears() {
        let cwd = std::env::temp_dir().join(format!(
            "phoenix-media-audit-{}-{}",
            std::process::id(),
            impeccable_common::now_ms()
        ));
        let area = super::super::index::resolve_area(&cwd, ".phoenix-ui/media");
        assert!(area.is_err());
        fs::create_dir_all(&cwd).unwrap();
        let area = super::super::index::resolve_area(&cwd, ".phoenix-ui/media").unwrap();
        assert!(run(&area, &cwd, &[]).unwrap().is_empty());
        fs::write(area.join("T1-test.png"), b"asset bytes").unwrap();
        let records = vec![
            json!({"file":"T1-test.png","ref":"T1","bytes":11,"sha256":format!("{:x}",Sha256::digest(b"asset bytes")),"pageUrl":"https://source.test/item","downloadUrl":"https://source.test/image","licenseUrl":"https://source.test/rights","license":"CC0","attribution":"AB"}),
        ];
        super::super::index::save_sources(&area, &records).unwrap();
        fs::write(cwd.join("hero.png"), b"asset bytes").unwrap();
        fs::write(
            cwd.join("index.html"),
            "<img src=\".phoenix-ui/media/T1-test.png\">\nAB\nCC0\nhttps://source.test/rights\n",
        )
        .unwrap();
        fs::create_dir_all(cwd.join("node_modules")).unwrap();
        fs::write(cwd.join("node_modules/copied.png"), b"asset bytes").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(cwd.join("hero.png"), cwd.join("linked.png")).unwrap();
        let findings = run(&area, &cwd, &[]).unwrap();
        assert!(!findings
            .iter()
            .any(|f| f["file"].as_str().unwrap().contains("node_modules")
                || f["file"] == "linked.png"));
        assert!(findings
            .iter()
            .any(|f| f["kind"] == "copy" && f["file"] == "hero.png"));
        assert!(findings
            .iter()
            .any(|f| f["kind"] == "reference" && f["line"] == 1));
        for (line, text) in [(2, "AB"), (3, "CC0"), (4, "https://source.test/rights")] {
            assert!(findings
                .iter()
                .any(|f| f["kind"] == "credit" && f["line"] == line && f["match"] == text));
        }
        let before = fs::read(area.join("sources.json")).unwrap();
        fs::remove_file(cwd.join("hero.png")).unwrap();
        fs::remove_file(cwd.join("index.html")).unwrap();
        assert!(run(&area, &cwd, &[]).unwrap().is_empty());
        assert_eq!(before, fs::read(area.join("sources.json")).unwrap());
        fs::remove_dir_all(cwd).unwrap();
    }
}
