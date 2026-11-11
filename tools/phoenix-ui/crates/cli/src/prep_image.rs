use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::codecs::{jpeg::JpegEncoder, webp::WebPEncoder};
use image::imageops::{self, FilterType};
use image::{
    DynamicImage, ImageDecoder, ImageEncoder, ImageFormat, ImageReader, Rgba, Rgba32FImage,
    RgbaImage,
};
use impeccable_common::Io;
use impeccable_context::{jsp, read_provenance, write_provenance, Carried};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub const USAGE: &str =
    "用法：phoenix-ui prep-image <图片或目录>… (--width <显示宽> | --widths <像素宽,…>) [选项]

本地处理静态 PNG、JPEG、WebP，不联网、不改原图。输入须先用 embed-prompt 写入来源。
目录只取直接子文件；目录输入必须给 --out，且输出目录不能是任一输入目录。

  --width <显示宽>    CSS 像素宽，导出 2 倍宽；1–16384，不足时不放大
  --widths <宽,…>     额外的像素宽，各 1–16384；可与 --width 同给
  --sizes <值>       img 的 sizes；默认 (max-width: Wpx) 100vw, Wpx；无 --width 时为 100vw
  --crop <宽:高>     居中裁切，接受 16:9、16/9、16x9、1.5；比例 0.01–100
  --trim             先裁全透明边，再按比例裁切，最后缩放
  --quality <1–100>  JPEG 质量，默认 82
  --out <目录>       默认各原图所在目录；重跑覆盖同名输出，禁止覆盖本批原图或输出冲突
  --json             输出 JSON 报告（像素尺寸、字节数、SHA-256、来源与说明）
  --help             显示帮助

不透明图出 JPEG，透明图出无损 WebP；不输出有损 WebP 或 AVIF。
输出携带原图来源；WebP 的 <图片>.json 旁注须与图片一起移动。
每张图有两个及以上输出时给出按宽度升序的 srcset/sizes 片段；路径相对当前目录并 URL 编码。
损坏、动画或缺少来源的图片单独失败，其他输入照常处理。
退出码：0 全部成功；1 参数错误或没有输入，不写文件；2 个别图片失败。
";

struct Options {
    inputs: Vec<PathBuf>,
    widths: BTreeSet<u32>,
    sizes: String,
    crop: Option<f64>,
    trim: bool,
    quality: u8,
    out: Option<PathBuf>,
    json: bool,
}

const VALUED: &[&str] = &[
    "--width",
    "--widths",
    "--sizes",
    "--crop",
    "--quality",
    "--out",
];
const FLAGS: &[&str] = &["--trim", "--json", "--help"];

fn parse_args(args: &[String], cwd: &Path) -> Result<Option<Options>, String> {
    let mut values = HashMap::new();
    let mut inputs = Vec::new();
    let mut trim = false;
    let mut json_output = false;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        i += 1;
        if arg == "--help" || arg == "-h" {
            return Ok(None);
        }
        if arg == "--" {
            inputs.extend(args[i..].iter().map(|s| cwd.join(s)));
            break;
        }
        if !arg.starts_with('-') {
            inputs.push(cwd.join(arg));
            continue;
        }
        let (name, inline) = arg
            .split_once('=')
            .map_or((arg.as_str(), None), |(n, v)| (n, Some(v)));
        if name == "--trim" || name == "--json" {
            if inline.is_some() {
                return Err(format!("{name} 不带值"));
            }
            if name == "--trim" {
                trim = true;
            } else {
                json_output = true;
            }
        } else if VALUED.contains(&name) {
            let value = match inline {
                Some(value) => value,
                None => match args.get(i) {
                    Some(value) if !value.starts_with("--") => {
                        i += 1;
                        value
                    }
                    _ => return Err(format!("{name} 需要一个值")),
                },
            };
            if values.insert(name, value).is_some() {
                return Err(format!("{name} 只能给一次"));
            }
        } else {
            let candidates: Vec<_> = VALUED.iter().chain(FLAGS).copied().collect();
            let hint = crate::capture::plan::nearest(name, &candidates)
                .map(|s| format!("，是否想写 {s}？"))
                .unwrap_or_default();
            return Err(format!("未知选项 {name}{hint}"));
        }
    }
    if inputs.is_empty() {
        return Err("至少需要一张图片或一个目录".into());
    }
    let width = values
        .get("--width")
        .map(|v| number(v, "--width", 16384))
        .transpose()?;
    let mut widths = BTreeSet::new();
    if let Some(width) = width {
        widths.insert(width * 2);
    }
    if let Some(list) = values.get("--widths") {
        for value in list.split(',') {
            widths.insert(number(value, "--widths", 16384)?);
        }
    }
    if widths.is_empty() {
        return Err("至少给 --width 或 --widths 之一".into());
    }
    let crop = values
        .get("--crop")
        .map(|value| {
            let ratio = if let Some((w, h)) = value.split_once([':', '/', 'x']) {
                w.parse::<f64>()
                    .ok()
                    .zip(h.parse::<f64>().ok())
                    .map(|(w, h)| w / h)
            } else {
                value.parse().ok()
            };
            ratio
                .filter(|r| r.is_finite() && (0.01..=100.0).contains(r))
                .ok_or_else(|| "--crop 比例应为 0.01–100".to_string())
        })
        .transpose()?;
    let quality = values
        .get("--quality")
        .map(|v| number(v, "--quality", 100))
        .transpose()?
        .unwrap_or(82) as u8;
    let out = values.get("--out").map(|v| cwd.join(v));
    for path in &inputs {
        if path.is_dir() {
            let target = out.as_ref().ok_or("目录输入必须给 --out")?;
            if path_identity(path)? == path_identity(target)? {
                return Err("--out 不能是输入目录".into());
            }
        }
    }
    let sizes = values
        .get("--sizes")
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            width
                .map(|w| format!("(max-width: {w}px) 100vw, {w}px"))
                .unwrap_or_else(|| "100vw".into())
        });
    Ok(Some(Options {
        inputs,
        widths,
        sizes,
        crop,
        trim,
        quality,
        out,
        json: json_output,
    }))
}

fn number(value: &str, option: &str, max: u32) -> Result<u32, String> {
    value
        .parse::<u32>()
        .ok()
        .filter(|n| (1..=max).contains(n))
        .ok_or_else(|| format!("{option} 应为 1–{max} 的整数，收到 {value}"))
}

fn path_identity(path: &Path) -> Result<PathBuf, String> {
    let normalized = PathBuf::from(jsp::resolve("/", &[&path.to_string_lossy()]));
    if normalized.exists() {
        return normalized.canonicalize().map_err(|e| e.to_string());
    }
    let parent = normalized
        .parent()
        .ok_or_else(|| format!("无效路径：{}", path.display()))?;
    let name = normalized
        .file_name()
        .ok_or_else(|| format!("无效路径：{}", path.display()))?;
    Ok(path_identity(parent)?.join(name))
}

fn shown(cwd: &Path, path: &Path) -> String {
    let root = cwd.to_string_lossy();
    jsp::relative(&root, &root, &path.to_string_lossy())
}

fn image_kind(bytes: &[u8]) -> Result<ImageFormat, String> {
    let kind = image::guess_format(bytes).map_err(|e| format!("不是静态 PNG/JPEG/WebP：{e}"))?;
    if !matches!(
        kind,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP
    ) {
        return Err("只支持静态 PNG/JPEG/WebP".into());
    }
    if kind == ImageFormat::Png {
        let mut offset = 8usize;
        while offset + 12 <= bytes.len() {
            if &bytes[offset + 4..offset + 8] == b"acTL" {
                return Err("动画图片不支持".into());
            }
            let length = u32::from_be_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]) as usize;
            offset += length + 12;
        }
    } else if kind == ImageFormat::WebP {
        let mut offset = 12usize;
        while offset + 8 <= bytes.len() {
            if matches!(&bytes[offset..offset + 4], b"ANIM" | b"ANMF") {
                return Err("动画图片不支持".into());
            }
            let length = u32::from_le_bytes([
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]) as usize;
            offset += 8 + length + length % 2;
        }
    }
    Ok(kind)
}

struct Prepared {
    input: PathBuf,
    pixels: RgbaImage,
    icc: Option<Vec<u8>>,
    provenance: String,
    original: Value,
    crop: Value,
    notes: Vec<String>,
    outputs: Vec<(PathBuf, u32, u32)>,
    alpha: bool,
}

fn prepare(input: &Path, opts: &Options) -> Result<Prepared, String> {
    let bytes = fs::read(input).map_err(|e| e.to_string())?;
    image_kind(&bytes)?;
    let reader = ImageReader::new(Cursor::new(&bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut decoder = reader
        .into_decoder()
        .map_err(|e| format!("图片解码失败：{e}"))?;
    let mut limits = image::Limits::default();
    limits
        .reserve(decoder.total_bytes())
        .map_err(|e| e.to_string())?;
    let orientation = decoder.orientation().map_err(|e| e.to_string())?;
    let icc = decoder.icc_profile().map_err(|e| e.to_string())?;
    let mut decoded =
        DynamicImage::from_decoder(decoder).map_err(|e| format!("图片解码失败：{e}"))?;
    decoded.apply_orientation(orientation);
    let mut pixels = decoded.into_rgba8();
    let (w, h) = pixels.dimensions();
    let alpha = pixels.pixels().any(|p| p[3] < 255);
    let original = json!({"width":w,"height":h,"bytes":bytes.len(),"alpha":alpha,"sha256":format!("{:x}",Sha256::digest(&bytes)),"provenance":true});
    let provenance = read_provenance(&input.to_string_lossy())
        .ok_or("先嵌入来源再处理：phoenix-ui embed-prompt <图片> --prompt <来源>")?;
    let mut x = 0;
    let mut y = 0;
    let mut cw = w;
    let mut ch = h;
    let mut notes = Vec::new();
    if opts.trim {
        if !alpha {
            notes.push("图片没有透明像素，未裁透明边".into());
        } else {
            let mut left = w;
            let mut top = h;
            let mut right = 0;
            let mut bottom = 0;
            for (px, py, pixel) in pixels.enumerate_pixels() {
                if pixel[3] > 0 {
                    left = left.min(px);
                    top = top.min(py);
                    right = right.max(px + 1);
                    bottom = bottom.max(py + 1);
                }
            }
            if left == w {
                return Err("整张图片全透明，无法裁边".into());
            }
            x = left;
            y = top;
            cw = right - left;
            ch = bottom - top;
        }
    }
    if let Some(ratio) = opts.crop {
        let width = cw.min((ch as f64 * ratio).round().max(1.0) as u32);
        let height = ch.min((cw as f64 / ratio).round().max(1.0) as u32);
        x += (cw - width) / 2;
        y += (ch - height) / 2;
        cw = width;
        ch = height;
    }
    pixels = imageops::crop_imm(&pixels, x, y, cw, ch).to_image();
    let alpha = pixels.pixels().any(|p| p[3] < 255);
    let widths: BTreeSet<u32> = opts.widths.iter().map(|n| (*n).min(cw)).collect();
    if opts.widths.iter().any(|n| *n > cw) {
        notes.push(format!("原图宽 {cw}，未放大"));
    }
    let directory = opts
        .out
        .as_deref()
        .unwrap_or_else(|| input.parent().unwrap_or(Path::new(".")));
    let stem = input.file_stem().ok_or("输入没有文件名")?.to_string_lossy();
    let extension = if alpha { "webp" } else { "jpg" };
    let mut outputs = Vec::new();
    for width in widths {
        let height = ((ch as f64 * width as f64 / cw as f64).round() as u32).max(1);
        outputs.push((
            path_identity(&directory.join(format!("{stem}-{width}.{extension}")))?,
            width,
            height,
        ));
    }
    Ok(Prepared {
        input: input.into(),
        pixels,
        icc,
        provenance,
        original,
        crop: json!({"x":x,"y":y,"width":cw,"height":ch}),
        notes,
        outputs,
        alpha,
    })
}

fn resize(pixels: &RgbaImage, width: u32, height: u32, alpha: bool) -> RgbaImage {
    if !alpha {
        return imageops::resize(pixels, width, height, FilterType::Lanczos3);
    }
    // Filtering straight RGB lets invisible dark pixels contaminate translucent edges.
    let premultiplied = Rgba32FImage::from_fn(pixels.width(), pixels.height(), |x, y| {
        let p = pixels.get_pixel(x, y);
        let a = p[3] as f32 / 255.0;
        Rgba([
            p[0] as f32 / 255.0 * a,
            p[1] as f32 / 255.0 * a,
            p[2] as f32 / 255.0 * a,
            a,
        ])
    });
    let resized = imageops::resize(&premultiplied, width, height, FilterType::Lanczos3);
    RgbaImage::from_fn(width, height, |x, y| {
        let p = resized.get_pixel(x, y);
        let alpha = p[3].clamp(0.0, 1.0);
        if alpha <= 0.0 {
            return Rgba([0, 0, 0, 0]);
        }
        Rgba([
            (p[0] / p[3] * 255.0).round().clamp(0.0, 255.0) as u8,
            (p[1] / p[3] * 255.0).round().clamp(0.0, 255.0) as u8,
            (p[2] / p[3] * 255.0).round().clamp(0.0, 255.0) as u8,
            (alpha * 255.0).round() as u8,
        ])
    })
}

fn output_url(path: &str) -> String {
    path.as_bytes()
        .iter()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~/".contains(b) {
                (*b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\'', "&#39;")
}

fn deliver(prepared: Prepared, opts: &Options, cwd: &Path) -> Result<Value, String> {
    let mut outputs = Vec::new();
    for (path, width, height) in &prepared.outputs {
        let pixels = resize(&prepared.pixels, *width, *height, prepared.alpha);
        let mut bytes = Vec::new();
        if prepared.alpha {
            let mut encoder = WebPEncoder::new_lossless(&mut bytes);
            if let Some(icc) = &prepared.icc {
                encoder
                    .set_icc_profile(icc.clone())
                    .map_err(|e| e.to_string())?;
            }
            encoder
                .write_image(
                    pixels.as_raw(),
                    *width,
                    *height,
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(|e| e.to_string())?;
        } else {
            let pixels = DynamicImage::ImageRgba8(pixels).into_rgb8();
            let mut encoder = JpegEncoder::new_with_quality(&mut bytes, opts.quality);
            if let Some(icc) = &prepared.icc {
                encoder
                    .set_icc_profile(icc.clone())
                    .map_err(|e| e.to_string())?;
            }
            encoder
                .write_image(
                    pixels.as_raw(),
                    *width,
                    *height,
                    image::ExtendedColorType::Rgb8,
                )
                .map_err(|e| e.to_string())?;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(path, &bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        let carrier = write_provenance(&path.to_string_lossy(), &prepared.provenance)?;
        let length = fs::metadata(path).map_err(|e| e.to_string())?.len();
        outputs.push(json!({"file":shown(cwd,path),"width":width,"height":height,"bytes":length,"format":if prepared.alpha {"webp"} else {"jpeg"},"provenance":match carrier {Carried::JpegCom => "jpeg-com",Carried::Sidecar => "sidecar",Carried::PngText => "png-text"}}));
    }
    let after = fs::read(&prepared.input).map_err(|e| e.to_string())?;
    if prepared.original["sha256"] != format!("{:x}", Sha256::digest(after)) {
        return Err("原图 SHA-256 已变化".into());
    }
    let mut record = json!({"input":shown(cwd,&prepared.input),"original":prepared.original,"crop":prepared.crop,"outputs":outputs,"notes":prepared.notes});
    if outputs.len() > 1 {
        if let Some(last) = outputs.last() {
            let srcset = outputs
                .iter()
                .map(|o| {
                    format!(
                        "{} {}w",
                        output_url(o["file"].as_str().unwrap_or_default()),
                        o["width"]
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            record["html"] = json!(format!(
                "<img src=\"{}\" srcset=\"{}\" sizes=\"{}\" width=\"{}\" height=\"{}\" alt=\"\">",
                output_url(last["file"].as_str().unwrap_or_default()),
                srcset,
                html_escape(&opts.sizes),
                last["width"],
                last["height"]
            ));
        }
    }
    Ok(record)
}

pub fn run(args: &[String], io: &mut Io) -> i32 {
    let opts = match parse_args(args, &io.cwd) {
        Ok(Some(opts)) => opts,
        Ok(None) => {
            io.out(USAGE);
            return 0;
        }
        Err(error) => {
            io.err(&format!("prep-image: {error}\n"));
            return 1;
        }
    };
    let cwd = match io.cwd.canonicalize() {
        Ok(cwd) => cwd,
        Err(error) => {
            io.err(&format!("prep-image: 当前目录不可读：{error}\n"));
            return 1;
        }
    };
    let mut inputs = BTreeSet::new();
    let mut errors = Vec::new();
    for input in &opts.inputs {
        let collected = (|| -> Result<Vec<PathBuf>, String> {
            if input.is_dir() {
                let mut paths = Vec::new();
                for entry in fs::read_dir(input).map_err(|e| e.to_string())? {
                    let path = entry.map_err(|e| e.to_string())?.path();
                    let extension = path
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    if path.is_file()
                        && matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp")
                    {
                        paths.push(path_identity(&path)?);
                    }
                }
                if paths.is_empty() {
                    return Err("目录没有 PNG/JPEG/WebP 图片".into());
                }
                Ok(paths)
            } else {
                Ok(vec![path_identity(input)?])
            }
        })();
        match collected {
            Ok(paths) => inputs.extend(paths),
            Err(message) => errors.push(json!({"input":shown(&cwd,input),"message":message})),
        }
    }
    let mut prepared = Vec::new();
    for input in &inputs {
        match prepare(input, &opts) {
            Ok(p) => prepared.push(p),
            Err(message) => errors.push(json!({"input":shown(&cwd,input),"message":message})),
        }
    }
    // Resolve the entire batch before writing so conflicts cannot destroy later inputs.
    let mut conflicts: HashMap<PathBuf, String> = HashMap::new();
    let mut owners: HashMap<PathBuf, PathBuf> = HashMap::new();
    for image in &prepared {
        for (path, _, _) in &image.outputs {
            let mut paths = vec![path.clone()];
            if image.alpha || image.provenance.len() + b"impeccable:prompt".len() + 3 > 0xffff {
                paths.push(PathBuf::from(format!("{}.json", path.display())));
            }
            for output in paths {
                if inputs.contains(&output) {
                    let message = format!(
                        "输出 {} 会覆盖输入 {}",
                        shown(&cwd, &output),
                        shown(&cwd, &output)
                    );
                    conflicts.insert(image.input.clone(), message.clone());
                    conflicts.insert(output.clone(), message);
                }
                if let Some(previous) = owners.insert(output.clone(), image.input.clone()) {
                    if previous != image.input {
                        let message = format!(
                            "{} 与 {} 的输出冲突：{}",
                            shown(&cwd, &previous),
                            shown(&cwd, &image.input),
                            shown(&cwd, &output)
                        );
                        conflicts.insert(previous, message.clone());
                        conflicts.insert(image.input.clone(), message);
                    }
                }
            }
        }
    }
    let mut records = Vec::new();
    for image in prepared {
        let input = shown(&cwd, &image.input);
        let result = match conflicts.remove(&image.input) {
            Some(message) => Err(message),
            None => deliver(image, &opts, &cwd),
        };
        match result {
            Ok(record) => records.push(record),
            Err(message) => errors.push(json!({"input":input,"message":message})),
        }
    }
    let code = if errors.is_empty() { 0 } else { 2 };
    let report = json!({"schema":"phoenix-ui-prep-image-v1","files":records,"errors":errors});
    if opts.json {
        io.out(&format!(
            "{}\n",
            impeccable_context::util::json_pretty(&report)
        ));
    } else {
        for record in &records {
            if let Some(outputs) = record["outputs"].as_array() {
                for o in outputs {
                    io.out(&format!(
                        "{}  {}x{}  {} B\n",
                        o["file"].as_str().unwrap_or_default(),
                        o["width"],
                        o["height"],
                        o["bytes"]
                    ));
                }
            }
            if let Some(notes) = record["notes"].as_array() {
                for note in notes {
                    io.out(&format!(
                        "{}: {}\n",
                        record["input"].as_str().unwrap_or_default(),
                        note.as_str().unwrap_or_default()
                    ));
                }
            }
            if let Some(html) = record["html"].as_str() {
                io.out(&format!("{html}\n"));
            }
        }
        for error in errors {
            io.err(&format!(
                "FAILED {}: {}\n",
                error["input"].as_str().unwrap_or_default(),
                error["message"].as_str().unwrap_or_default()
            ));
        }
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};
    use impeccable_context::{read_provenance, write_provenance};
    use serde_json::Value;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Project(PathBuf);
    impl Project {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "phoenix-prep-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn image(&self, name: &str, pixels: RgbaImage, source: bool) -> PathBuf {
            let path = self.0.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            pixels
                .save_with_format(&path, image::ImageFormat::Png)
                .unwrap();
            if source {
                write_provenance(path.to_str().unwrap(), "project-owned source").unwrap();
            }
            path
        }
        fn run(&self, args: &[&str]) -> (i32, Value, String) {
            let (mut io, output) = Io::captured("", self.0.clone(), HashMap::new());
            let mut argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
            argv.push("--json".into());
            let code = run(&argv, &mut io);
            let report = serde_json::from_slice(&output.stdout.borrow()).unwrap_or(Value::Null);
            let stderr = String::from_utf8_lossy(&output.stderr.borrow()).into_owned();
            (code, report, stderr)
        }
    }
    impl Drop for Project {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn solid(w: u32, h: u32) -> RgbaImage {
        RgbaImage::from_pixel(w, h, Rgba([80, 120, 180, 255]))
    }
    fn output_image(root: &Path, output: &Value) -> image::DynamicImage {
        image::open(root.join(output["file"].as_str().unwrap())).unwrap()
    }

    #[test]
    fn rejects_invalid_arguments_and_directory_output_before_writing() {
        let p = Project::new();
        p.image("in/a.png", solid(10, 10), true);
        for args in [
            vec![],
            vec!["in/a.png"],
            vec!["in/a.png", "--width", "0"],
            vec!["in/a.png", "--crop", "NaN", "--width", "1"],
            vec!["in", "--width", "1"],
            vec!["in", "--width", "1", "--out", "in"],
        ] {
            let (code, _, _) = p.run(&args);
            assert_eq!(code, 1, "{args:?}");
        }
        assert_eq!(std::fs::read_dir(p.0.join("in")).unwrap().count(), 1);
    }

    #[test]
    fn produces_sorted_widths_without_upscaling_and_keeps_sources() {
        let p = Project::new();
        let original = p.image("hero & 图.png", solid(80, 50), true);
        let before = std::fs::read(&original).unwrap();
        let (code, result, err) = p.run(&[
            "hero & 图.png",
            "--width",
            "60",
            "--widths",
            "20,80,20",
            "--out",
            "public/img",
        ]);
        assert_eq!(code, 0, "{err} {result}");
        let record = &result["files"][0];
        let outputs = record["outputs"].as_array().unwrap();
        assert_eq!(
            outputs
                .iter()
                .map(|x| x["width"].as_u64().unwrap())
                .collect::<Vec<_>>(),
            vec![20, 80]
        );
        assert!(record["notes"].to_string().contains("未放大"));
        assert_eq!(outputs[1]["height"], 50);
        assert_eq!(outputs[0]["format"], "jpeg");
        assert_eq!(output_image(&p.0, &outputs[0]).width(), 20);
        let html = record["html"].as_str().unwrap();
        assert!(
            html.contains(
                "hero%20%26%20%E5%9B%BE-20.jpg 20w, public/img/hero%20%26%20%E5%9B%BE-80.jpg 80w"
            ),
            "{html}"
        );
        assert!(html.contains("sizes=\"(max-width: 60px) 100vw, 60px\""));
        for output in outputs {
            assert_eq!(
                read_provenance(p.0.join(output["file"].as_str().unwrap()).to_str().unwrap())
                    .as_deref(),
                Some("project-owned source")
            );
        }
        assert_eq!(std::fs::read(original).unwrap(), before);
        let (code, result, _) = p.run(&["hero & 图.png", "--widths", "20,40", "--out", "other"]);
        assert_eq!(code, 0);
        assert!(result["files"][0]["html"]
            .as_str()
            .unwrap()
            .contains("sizes=\"100vw\""));
    }

    #[test]
    fn trims_then_crops_and_preserves_transparent_pixels() {
        let p = Project::new();
        let mut pixels = RgbaImage::from_pixel(20, 12, Rgba([0, 0, 0, 0]));
        for y in 2..10 {
            for x in 4..16 {
                pixels.put_pixel(x, y, Rgba([255, 70, 30, 128]));
            }
        }
        p.image("padded.png", pixels, true);
        let (code, result, _) = p.run(&["padded.png", "--widths", "6", "--trim", "--crop", "1"]);
        assert_eq!(code, 0, "{result}");
        let record = &result["files"][0];
        assert_eq!(
            record["crop"],
            serde_json::json!({"x":6,"y":2,"width":8,"height":8})
        );
        let output = &record["outputs"][0];
        assert_eq!(output["format"], "webp");
        let image = output_image(&p.0, output).into_rgba8();
        assert_eq!(image.dimensions(), (6, 6));
        assert!(image.pixels().all(|p| p[3] == 128 && p[0] >= 254));
        assert!(record.get("html").is_none());
        p.image("transparent.png", RgbaImage::new(3, 3), true);
        let (code, result, _) = p.run(&["transparent.png", "--trim", "--width", "1"]);
        assert_eq!(code, 2);
        assert!(result["errors"][0]["message"]
            .as_str()
            .unwrap()
            .contains("全透明"));
        p.image("opaque.png", solid(10, 5), true);
        let (_, result, _) = p.run(&["opaque.png", "--trim", "--widths", "5"]);
        assert!(result["files"][0]["notes"].to_string().contains("没有透明"));
        assert_eq!(result["files"][0]["outputs"][0]["format"], "jpeg");
    }

    #[test]
    fn crop_geometry_and_output_height_stay_nonzero() {
        let p = Project::new();
        for (name, w, h, ratio, expected) in [
            ("wide.png", 20, 10, "1", (10, 10)),
            ("tall.png", 10, 20, "2", (10, 5)),
            ("same.png", 20, 10, "2", (20, 10)),
            ("thin.png", 2, 2, "0.01", (1, 2)),
            ("flat.png", 2, 2, "100", (2, 1)),
        ] {
            p.image(name, solid(w, h), true);
            let (code, result, _) = p.run(&[name, "--crop", ratio, "--widths", "1"]);
            assert_eq!(code, 0, "{result}");
            let r = &result["files"][0];
            assert_eq!(
                (
                    r["crop"]["width"].as_u64().unwrap(),
                    r["crop"]["height"].as_u64().unwrap()
                ),
                expected
            );
            assert!(r["outputs"][0]["height"].as_u64().unwrap() >= 1);
        }
    }

    #[test]
    fn refuses_input_and_batch_output_collisions() {
        let p = Project::new();
        let a = p.image("hero.png", solid(20, 20), true);
        let b = p.image("hero-10.jpg", solid(20, 20), true);
        let before = std::fs::read(&b).unwrap();
        let (code, result, _) = p.run(&["hero.png", "hero-10.jpg", "--widths", "10"]);
        assert_eq!(code, 2, "{result}");
        assert_eq!(std::fs::read(&b).unwrap(), before);
        assert!(!p.0.join("hero-10-10.jpg").exists());
        assert!(a.exists());
        p.image("one/same.png", solid(20, 20), true);
        p.image("two/same.png", solid(20, 20), true);
        let (code, result, _) = p.run(&[
            "one/same.png",
            "two/same.png",
            "--widths",
            "10",
            "--out",
            "out",
        ]);
        assert_eq!(code, 2, "{result}");
        assert_eq!(result["errors"].as_array().unwrap().len(), 2);
        assert!(!p.0.join("out/same-10.jpg").exists());
    }

    #[test]
    fn bad_missing_source_and_animated_inputs_fail_independently() {
        let p = Project::new();
        p.image("good.png", solid(10, 10), true);
        p.image("no-source.png", solid(10, 10), false);
        std::fs::write(p.0.join("broken.png"), "broken").unwrap();
        let apng = p.image("animated.png", solid(10, 10), true);
        let mut bytes = std::fs::read(&apng).unwrap();
        bytes.splice(
            33..33,
            [
                0, 0, 0, 8, b'a', b'c', b'T', b'L', 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
        );
        std::fs::write(&apng, bytes).unwrap();
        let webp = p.0.join("animated.webp");
        std::fs::write(&webp, b"RIFF\x0c\0\0\0WEBPANIM\0\0\0\0").unwrap();
        write_provenance(webp.to_str().unwrap(), "animation").unwrap();
        let (code, result, _) = p.run(&[
            "good.png",
            "no-source.png",
            "broken.png",
            "animated.png",
            "animated.webp",
            "--widths",
            "5",
            "--out",
            "out",
        ]);
        assert_eq!(code, 2, "{result}");
        assert_eq!(result["files"].as_array().unwrap().len(), 1);
        assert_eq!(result["errors"].as_array().unwrap().len(), 4);
        assert_eq!(
            result["errors"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| r["message"].as_str().unwrap().contains("动画"))
                .count(),
            2
        );
        assert!(!p.0.join("out/no-source-5.jpg").exists());
    }
}
