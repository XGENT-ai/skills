use super::{
    index, poster,
    sources::{self, Endpoints},
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub(super) struct FetchOptions {
    pub maps: Vec<String>,
    pub resolution: String,
    pub video_size: String,
    pub max_mb: u64,
    pub download_timeout_ms: u64,
}

struct Job {
    item: Value,
    urls: Vec<String>,
    map: Option<String>,
    resolution: Option<String>,
    expected: Option<u64>,
}
struct TempFile(PathBuf);
impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
struct Stage(PathBuf);
impl Drop for Stage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
struct Download {
    file: TempFile,
    url: String,
    bytes: u64,
    digest: String,
    ext: &'static str,
    video: bool,
    width: u32,
    height: u32,
    warnings: Vec<String>,
}

pub(super) fn run(
    agent: &ureq::Agent,
    endpoints: &Endpoints,
    area: &Path,
    items: &[Value],
    options: &FetchOptions,
    env: &HashMap<String, String>,
) -> Result<(Vec<Value>, Vec<Value>), String> {
    index::read_sources(area)?;
    let mut jobs = Vec::new();
    let mut errors = Vec::new();
    for item in items {
        match plan(agent, endpoints, item, options) {
            Ok((ready, failed)) => {
                jobs.extend(ready);
                errors.extend(failed);
            }
            Err(message) => errors.push(json!({"ref":item["ref"],"message":message})),
        }
    }
    let mut files = Vec::new();
    let mut session: Option<Result<poster::Session, String>> = None;
    for group in jobs.chunks(3) {
        let downloaded = std::thread::scope(|scope| {
            let handles: Vec<_> = group
                .iter()
                .map(|job| scope.spawn(move || download(agent, area, job, options)))
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join())
                .collect::<Vec<_>>()
        });
        for (job, result) in group.iter().zip(downloaded) {
            let result = result
                .map_err(|_| "素材下载线程意外终止".to_string())
                .and_then(|r| r)
                .and_then(|download| finish(area, job, download, env, &mut session));
            match result {
                Ok(ready) => files.extend(ready),
                Err(message) => {
                    let mut error = json!({"ref":job.item["ref"],"message":message});
                    if let Some(map) = &job.map {
                        error["map"] = json!(map);
                    }
                    errors.push(error);
                }
            }
        }
    }
    if !files.is_empty() {
        index::save_sources(area, &files)?;
    }
    Ok((files, errors))
}

fn plan(
    agent: &ureq::Agent,
    endpoints: &Endpoints,
    item: &Value,
    options: &FetchOptions,
) -> Result<(Vec<Job>, Vec<Value>), String> {
    if item["provider"] == "polyhaven" && item["kind"] == "texture" {
        let source = item["sourceId"].as_str().ok_or("贴图缺少 sourceId")?;
        let files = sources::get_json(
            agent,
            &format!(
                "{}/files/{}",
                endpoints.polyhaven.trim_end_matches('/'),
                encode(source)
            ),
        )?;
        let mut jobs = Vec::new();
        let mut errors = Vec::new();
        for map in &options.maps {
            let key = match map.as_str() {
                "color" => "Diffuse",
                "roughness" => "Rough",
                "normal" => "nor_gl",
                _ => return Err(format!("未知贴图 {map}")),
            };
            let Some(entry) = sources::select_map(&files, &[key], &options.resolution) else {
                errors.push(
                    json!({"ref":item["ref"],"map":map,"message":format!("没有可用 {key} JPG")}),
                );
                continue;
            };
            let url = entry["url"].as_str().ok_or("贴图缺少下载地址")?;
            jobs.push(Job {
                item: item.clone(),
                urls: vec![url.to_string()],
                map: Some(map.clone()),
                resolution: entry["resolution"].as_str().map(str::to_string),
                expected: entry["size"].as_u64(),
            });
        }
        return Ok((jobs, errors));
    }
    let mut urls = Vec::new();
    if let Some(manifest) = item["manifestUrl"].as_str().filter(|s| !s.is_empty()) {
        let value = sources::get_json(agent, manifest)?;
        let rows = value
            .as_array()
            .or_else(|| value["collection"]["items"].as_array())
            .ok_or("素材清单不是数组")?;
        let video = item["video"] == true || item["kind"] == "video";
        let mut variants = if video {
            vec![
                options.video_size.as_str(),
                "medium",
                "small",
                "mobile",
                "preview",
                "orig",
            ]
        } else {
            vec!["orig", "large", "medium", "small"]
        };
        variants.dedup();
        let mut found: Vec<String> = rows
            .iter()
            .filter_map(|v| v.as_str().or_else(|| v["href"].as_str()))
            .filter(|url| {
                let path = url.split('?').next().unwrap_or(url).to_ascii_lowercase();
                if video {
                    path.ends_with(".mp4")
                } else {
                    [".jpg", ".jpeg", ".png", ".webp"]
                        .iter()
                        .any(|e| path.ends_with(e))
                }
            })
            .map(str::to_string)
            .collect();
        found.sort_by_key(|u| {
            variants
                .iter()
                .position(|v| u.contains(&format!("~{v}.")))
                .unwrap_or(usize::MAX)
        });
        urls.extend(found);
    } else if let Some(info) = item["iiifInfoUrl"].as_str().filter(|s| !s.is_empty()) {
        let base = info.strip_suffix("/info.json").unwrap_or(info);
        let largest = item["originalWidth"]
            .as_u64()
            .unwrap_or(0)
            .max(item["originalHeight"].as_u64().unwrap_or(0));
        let size = if largest <= 1600 {
            "full"
        } else {
            "!1600,1600"
        };
        urls.push(format!("{base}/full/{size}/0/default.jpg"));
    } else if let Some(url) = item["imageUrl"].as_str().filter(|s| !s.is_empty()) {
        urls.push(url.into());
    }
    if let Some(candidates) = item["downloadCandidates"].as_array() {
        for candidate in candidates.iter().filter_map(Value::as_str) {
            if !urls.iter().any(|u| u == candidate) {
                urls.push(candidate.to_string());
            }
        }
    }
    urls.truncate(3);
    if urls.is_empty() {
        return Err("没有可下载的图片或视频".into());
    }
    Ok((
        vec![Job {
            item: item.clone(),
            urls,
            map: None,
            resolution: None,
            expected: None,
        }],
        Vec::new(),
    ))
}

fn download(
    agent: &ureq::Agent,
    area: &Path,
    job: &Job,
    options: &FetchOptions,
) -> Result<Download, String> {
    let mut failures = Vec::new();
    for (n, url) in job.urls.iter().enumerate() {
        match download_one(agent, area, url, job.expected, options) {
            Ok(mut result) => {
                if n > 0 {
                    result.warnings.push(format!("使用备选下载地址：{url}"));
                }
                return Ok(result);
            }
            Err(error) => failures.push(error),
        }
    }
    Err(failures.join("；"))
}

fn download_one(
    agent: &ureq::Agent,
    area: &Path,
    url: &str,
    expected: Option<u64>,
    options: &FetchOptions,
) -> Result<Download, String> {
    let url = sources::normalize_url(url)?;
    let response = agent
        .get(&url)
        .timeout(Duration::from_millis(options.download_timeout_ms))
        .call()
        .map_err(|e| format!("下载失败：{e}"))?;
    sources::normalize_url(response.get_url())?;
    let limit = options.max_mb * 1024 * 1024;
    let length = response
        .header("Content-Length")
        .and_then(|n| n.parse::<u64>().ok());
    if length.is_some_and(|n| n > limit) {
        return Err(format!("文件超过 --max-mb {}", options.max_mb));
    }
    let mime = response
        .header("Content-Type")
        .unwrap_or("")
        .to_ascii_lowercase();
    if mime.contains("text/html") || mime.contains("application/json") {
        return Err("响应是 HTML/JSON 错误页，不是素材".into());
    }
    let encoded = response.header("Content-Encoding").is_some();
    let file = TempFile(area.join(index::temporary_name()));
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&file.0)
        .map_err(|e| e.to_string())?;
    let mut input = response.into_reader();
    let mut hash = Sha256::new();
    let mut bytes = 0;
    let mut head = Vec::new();
    let mut buffer = [0_u8; 65536];
    loop {
        let n = input
            .read(&mut buffer)
            .map_err(|e| format!("下载中断：{e}"))?;
        if n == 0 {
            break;
        }
        bytes += n as u64;
        if bytes > limit {
            return Err(format!("文件超过 --max-mb {}", options.max_mb));
        }
        if head.len() < 32 {
            head.extend_from_slice(&buffer[..n.min(32 - head.len())]);
        }
        hash.update(&buffer[..n]);
        output.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
    }
    drop(output);
    if bytes == 0 || (!encoded && length.is_some_and(|n| n != bytes)) {
        return Err("文件为空或传输不完整".into());
    }
    if expected.is_some_and(|n| n != bytes) {
        return Err(format!(
            "文件大小不符：{bytes}，预期 {}",
            expected.unwrap_or(0)
        ));
    }
    let (ext, video) = sniff(&head)?;
    let (width, height) = if video {
        (0, 0)
    } else {
        let image = image::ImageReader::open(&file.0)
            .map_err(|e| e.to_string())?
            .with_guessed_format()
            .map_err(|e| e.to_string())?
            .decode()
            .map_err(|e| format!("图片无法完整解码：{e}"))?;
        (image.width(), image.height())
    };
    Ok(Download {
        file,
        url,
        bytes,
        digest: format!("{:x}", hash.finalize()),
        ext,
        video,
        width,
        height,
        warnings: Vec::new(),
    })
}

fn sniff(head: &[u8]) -> Result<(&'static str, bool), String> {
    if head.starts_with(&[0xff, 0xd8, 0xff]) {
        return Ok(("jpg", false));
    }
    if head.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Ok(("png", false));
    }
    if head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a") {
        return Ok(("gif", false));
    }
    if head.starts_with(b"RIFF") && head.get(8..12) == Some(b"WEBP") {
        return Ok(("webp", false));
    }
    if head.get(4..8) == Some(b"ftyp") {
        return Ok((
            if head.get(8..10) == Some(b"qt") {
                "mov"
            } else {
                "mp4"
            },
            true,
        ));
    }
    if head.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]) {
        return Ok(("webm", true));
    }
    Err("响应不是支持的图片或视频；可能是错误页或损坏的文件".into())
}

fn finish(
    area: &Path,
    job: &Job,
    mut download: Download,
    env: &HashMap<String, String>,
    session: &mut Option<Result<poster::Session, String>>,
) -> Result<Vec<Value>, String> {
    let reference = job.item["ref"].as_str().ok_or("素材缺少编号")?;
    let source = job.item["sourceId"].as_str().ok_or("素材缺少 sourceId")?;
    let mut stem = format!("{}-{}", safe_name(reference), safe_name(source));
    if let (Some(map), Some(resolution)) = (&job.map, &job.resolution) {
        stem.push_str(&format!("-{map}-{resolution}"));
    }
    let name = format!("{stem}.{}", download.ext);
    let mut poster_image = None;
    let mut stage = None;
    if download.video {
        let session = session
            .get_or_insert_with(|| poster::Session::new(env, area))
            .as_mut()
            .map_err(|e| e.clone())?;
        // The static server rejects dot files, so decode an uncommitted file in a private staging directory.
        let folder = format!(
            "media-stage-{}",
            index::temporary_name().trim_start_matches('.')
        );
        let dir = Stage(area.join(&folder));
        fs::create_dir(&dir.0).map_err(|e| e.to_string())?;
        let file = format!("video.{}", download.ext);
        let target = dir.0.join(&file);
        fs::rename(&download.file.0, &target).map_err(|e| e.to_string())?;
        download.file.0 = target;
        let png = session.first_frame(&format!("{folder}/{file}"))?;
        let image = image::load_from_memory(&png).map_err(|e| format!("首帧无法解码：{e}"))?;
        download.width = image.width();
        download.height = image.height();
        poster_image = Some(png);
        stage = Some(dir);
    }
    if download.width.max(download.height) < 1000 {
        download.warnings.push("长边不足 1000 像素".into());
    }
    if job.item["use"] == "reference" {
        download.warnings.push("仅参考：只作生成或设计参考".into());
    }
    let record = record(job, &download, &name);
    let mut records = vec![record.clone()];
    if let Some(png) = poster_image {
        let poster_name = format!("{stem}-poster.png");
        index::atomic_write(area, &poster_name, &png)?;
        if let Err(error) = fs::rename(&download.file.0, area.join(&name)) {
            let _ = fs::remove_file(area.join(&poster_name));
            return Err(error.to_string());
        }
        let mut derived = record;
        derived["file"] = json!(poster_name);
        derived["bytes"] = json!(png.len());
        derived["sha256"] = json!(format!("{:x}", Sha256::digest(&png)));
        derived["derivedFrom"] = json!(name);
        records.push(derived);
    } else {
        fs::rename(&download.file.0, area.join(&name)).map_err(|e| e.to_string())?;
    }
    drop(stage);
    Ok(records)
}

fn record(job: &Job, download: &Download, file: &str) -> Value {
    let mut value = json!({"file":file,"downloadUrl":download.url,"fetchedAt":impeccable_common::iso_now().chars().take(10).collect::<String>(),"width":download.width,"height":download.height,"bytes":download.bytes,"sha256":download.digest,"integrity":"decoded","warnings":download.warnings});
    for field in [
        "ref",
        "title",
        "provider",
        "pageUrl",
        "license",
        "licenseUrl",
        "attribution",
        "use",
    ] {
        value[field] = job.item[field].clone();
    }
    if let Some(map) = &job.map {
        value["map"] = json!(map);
        value["resolution"] = json!(job.resolution);
    }
    value
}

fn safe_name(value: &str) -> String {
    let mut out = String::new();
    let mut previous_dash = false;
    for c in value.chars() {
        if out.len() >= 60 {
            break;
        }
        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            out.push(c);
            previous_dash = false;
        } else if !previous_dash {
            out.push('-');
            previous_dash = true;
        }
    }
    if out.is_empty() {
        out.push('-');
    }
    out
}
fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use std::time::Duration;
    const PNG: &[u8] = include_bytes!("../../../../tests/fixtures/media/sample.png");
    const JPEG_BAD: &[u8] = include_bytes!("../../../../tests/fixtures/media/truncated.jpg");
    struct Server {
        url: String,
        stop: Arc<AtomicBool>,
        join: Option<std::thread::JoinHandle<()>>,
    }
    impl Server {
        fn new() -> Self {
            let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
            let url = format!("http://{}", server.server_addr());
            let host = url.clone();
            let stop = Arc::new(AtomicBool::new(false));
            let done = stop.clone();
            let join = std::thread::spawn(move || {
                while !done.load(Ordering::Relaxed) {
                    if let Some(request) = server.recv_timeout(Duration::from_millis(25)).unwrap() {
                        let route = request.url();
                        let (bytes, mime) = match route {
                            "/image" => (PNG.to_vec(),"image/png"),
                            "/broken" => (JPEG_BAD.to_vec(),"image/jpeg"),
                            "/html" => (b"<html>error</html>".to_vec(),"text/html"),
                            "/large" => (vec![b'x';1024*1024+1],"image/png"),
                            "/video" => (include_bytes!("../../../../tests/fixtures/media/sample.webm").to_vec(),"video/webm"),
                            "/files/paper" | "/files/paper-all" => (serde_json::to_vec(&json!({"Diffuse":{"1k":{"jpg":{"url":format!("{host}/image"),"size":PNG.len()}}},"Rough":{"1k":{"jpg":{"url":format!("{host}/image"),"size":PNG.len()}}},"nor_gl":{"1k":{"jpg":{"url":format!("{host}/image"),"size":if route.ends_with("paper-all"){PNG.len()}else{1}}}}})).unwrap(),"application/json"),
                            _ => (b"missing".to_vec(),"text/plain"),
                        };
                        let response = tiny_http::Response::from_data(bytes).with_header(
                            tiny_http::Header::from_bytes("Content-Type", mime).unwrap(),
                        );
                        let _ = request.respond(response);
                    }
                }
            });
            Self {
                url,
                stop,
                join: Some(join),
            }
        }
    }
    impl Drop for Server {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(join) = self.join.take() {
                join.join().unwrap();
            }
        }
    }
    fn item(reference: &str, url: &str) -> Value {
        json!({"ref":reference,"provider":"commons","sourceId":reference,"kind":"photo","title":"test","imageUrl":url,"pageUrl":"https://example.test/item","license":"CC0","licenseUrl":"","attribution":"test","use":"page"})
    }
    fn options() -> FetchOptions {
        FetchOptions {
            maps: vec!["color".into()],
            resolution: "1k".into(),
            video_size: "medium".into(),
            max_mb: 1,
            download_timeout_ms: 2000,
        }
    }
    fn area(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "phoenix-media-fetch-{name}-{}-{}",
            std::process::id(),
            impeccable_common::now_ms()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
    #[test]
    fn downloads_good_images_rejects_bad_content_and_cleans_parts() {
        let server = Server::new();
        let area = area("content");
        let agent = ureq::AgentBuilder::new().build();
        let mut good = item("C1", &format!("{}/image", server.url));
        good["sourceId"] = json!("../../bad title/图");
        let items = vec![
            good,
            item("C2", &format!("{}/broken", server.url)),
            item("C3", &format!("{}/html", server.url)),
            item("C4", &format!("{}/large", server.url)),
        ];
        let (files, errors) = run(
            &agent,
            &Endpoints::default(),
            &area,
            &items,
            &options(),
            &HashMap::new(),
        )
        .unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(errors.len(), 3);
        assert_eq!(files[0]["file"], "C1--bad-title-.png");
        assert_eq!(files[0]["width"], 64);
        assert_eq!(files[0]["height"], 48);
        assert_eq!(files[0]["integrity"], "decoded");
        assert_eq!(files[0]["bytes"], PNG.len());
        use sha2::{Digest, Sha256};
        assert_eq!(files[0]["sha256"], format!("{:x}", Sha256::digest(PNG)));
        assert_eq!(super::super::index::read_sources(&area).unwrap().len(), 1);
        assert!(!fs::read_dir(&area).unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".part")));
        fs::remove_dir_all(area).unwrap();
    }
    #[test]
    fn texture_maps_validate_expected_bytes_individually() {
        let server = Server::new();
        let area = area("maps");
        let endpoints = Endpoints {
            polyhaven: server.url.clone(),
            ..Default::default()
        };
        let mut options = options();
        options.maps = vec!["color".into(), "roughness".into(), "normal".into()];
        let mut texture = item("T1", "");
        texture["provider"] = json!("polyhaven");
        texture["kind"] = json!("texture");
        texture["sourceId"] = json!("paper");
        let mut all = texture.clone();
        all["sourceId"] = json!("paper-all");
        let (all_files, all_errors) = run(
            &ureq::AgentBuilder::new().build(),
            &endpoints,
            &area,
            &[all],
            &options,
            &HashMap::new(),
        )
        .unwrap();
        assert_eq!(all_files.len(), 3);
        assert!(all_errors.is_empty());
        let (files, errors) = run(
            &ureq::AgentBuilder::new().build(),
            &endpoints,
            &area,
            &[texture],
            &options,
            &HashMap::new(),
        )
        .unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0]["map"], "normal");
        assert!(files.iter().all(|f| f["resolution"] == "1k"));
        fs::remove_dir_all(area).unwrap();
    }
    #[test]
    fn missing_browser_never_leaves_video_or_manifest_entry() {
        let server = Server::new();
        let area = area("no-browser");
        let mut video = item("N1", &format!("{}/video", server.url));
        video["video"] = json!(true);
        let env = HashMap::from([("PHOENIX_UI_BROWSER".into(), "/nonexistent".into())]);
        let (files, errors) = run(
            &ureq::AgentBuilder::new().build(),
            &Endpoints::default(),
            &area,
            &[video],
            &options(),
            &env,
        )
        .unwrap();
        assert!(files.is_empty());
        assert_eq!(errors.len(), 1);
        assert!(errors[0]["message"]
            .as_str()
            .unwrap()
            .contains("浏览器不可用"));
        assert!(super::super::index::read_sources(&area).unwrap().is_empty());
        assert_eq!(fs::read_dir(&area).unwrap().count(), 0);
        fs::remove_dir_all(area).unwrap();
    }
    #[test]
    fn browser_decodes_webm_and_registers_video_and_poster_together() {
        let env = std::env::vars().collect();
        if impeccable_browser::discovery::find_browser(&env).is_err() {
            eprintln!("skip: browser unavailable");
            return;
        }
        let server = Server::new();
        let area = area("video");
        let mut video = item("N2", &format!("{}/video", server.url));
        video["video"] = json!(true);
        let (files, errors) = run(
            &ureq::AgentBuilder::new().build(),
            &Endpoints::default(),
            &area,
            &[video],
            &options(),
            &env,
        )
        .unwrap();
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(files.len(), 2);
        assert!(files
            .iter()
            .all(|f| f["width"] == 64 && f["height"] == 48 && f["integrity"] == "decoded"));
        let poster = files.iter().find(|f| f["derivedFrom"].is_string()).unwrap();
        let png = image::open(area.join(poster["file"].as_str().unwrap())).unwrap();
        assert_eq!((png.width(), png.height()), (64, 48));
        assert_eq!(super::super::index::read_sources(&area).unwrap().len(), 2);
        assert!(!fs::read_dir(&area)
            .unwrap()
            .any(|e| e.unwrap().file_type().unwrap().is_dir()));
        fs::remove_dir_all(area).unwrap();
    }
}
