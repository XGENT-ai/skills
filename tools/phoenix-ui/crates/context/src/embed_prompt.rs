//! JS: embed-prompt.mjs -> `impeccable embed-prompt`

use crate::jsp;
use crate::util::{exists, iso_now, json_pretty, read_dir_names, utf16_len};
use impeccable_common::Io;
use serde_json::{Map, Value};
use std::io::Read;

const KEYWORD: &[u8] = b"impeccable:prompt";

fn png_chunk(ty: &[u8], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(12 + data.len());
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(ty);
    out.extend_from_slice(data);
    let mut crc_in = ty.to_vec();
    crc_in.extend_from_slice(data);
    out.extend_from_slice(&impeccable_comp::crc32(&crc_in).to_be_bytes());
    out
}

fn u32be(b: &[u8], off: usize) -> u32 {
    u32::from_be_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

/// JS: embed-prompt.mjs#parsePng(buffer) -> { chunks, prompt } (fix #641).
/// Walks every PNG chunk, recording each chunk's byte range and whether it is
/// our keyword chunk, and extracts the first embedded prompt.
struct PngChunk {
    offset: usize,
    ty: [u8; 4],
    prompt_chunk: bool,
    end: usize,
}

fn parse_png(b: &[u8]) -> (Vec<PngChunk>, Option<String>) {
    let mut chunks: Vec<PngChunk> = Vec::new();
    let mut prompt: Option<String> = None;
    let mut off = 8usize;
    while off + 12 <= b.len() {
        let len = u32be(b, off) as usize;
        let mut ty = [0u8; 4];
        ty.copy_from_slice(&b[off + 4..off + 8]);
        let data_end = (off + 8 + len).min(b.len());
        let data = &b[(off + 8).min(b.len())..data_end];
        let nul = data.iter().position(|x| *x == 0);
        let prompt_chunk = (&ty == b"tEXt" || &ty == b"zTXt") && nul.map(|n| &data[..n] == KEYWORD).unwrap_or(false);
        if prompt.is_none() && prompt_chunk {
            let n = nul.unwrap();
            prompt = if &ty == b"tEXt" {
                Some(String::from_utf8_lossy(&data[n + 1..]).into_owned())
            } else {
                let comp = &data[(n + 2).min(data.len())..];
                let mut d = flate2::read::ZlibDecoder::new(comp);
                let mut out = Vec::new();
                match d.read_to_end(&mut out) {
                    Ok(_) => Some(String::from_utf8_lossy(&out).into_owned()),
                    Err(_) => None, // JS: inflateSync throws -> uncaught; treat as none
                }
            };
        }
        let end = (off + 12 + len).min(b.len());
        chunks.push(PngChunk { offset: off, ty, prompt_chunk, end });
        off += 12 + len;
    }
    (chunks, prompt)
}

fn read_jpeg_com(b: &[u8]) -> Option<String> {
    let mut off = 2usize;
    while off + 4 <= b.len() && b[off] == 0xff {
        let marker = b[off + 1];
        if marker == 0xda {
            break;
        }
        let len = u16::from_be_bytes([b[off + 2], b[off + 3]]) as usize;
        if marker == 0xfe {
            let end = (off + 2 + len).min(b.len());
            let start = (off + 4).min(end);
            let text = String::from_utf8_lossy(&b[start..end]).into_owned();
            let prefix = format!("{}\0", String::from_utf8_lossy(KEYWORD));
            if let Some(rest) = text.strip_prefix(&prefix) {
                return Some(rest.to_string());
            }
        }
        off += 2 + len;
    }
    None
}

fn is_png(b: &[u8]) -> bool {
    b.len() > 8 && u32be(b, 0) == 0x89504e47
}
fn is_jpeg(b: &[u8]) -> bool {
    b.len() > 3 && b[0] == 0xff && b[1] == 0xd8
}

/// JS: embed-prompt.mjs#imageType(buffer) (fix #641).
#[derive(PartialEq)]
enum ImageType {
    Png,
    Jpeg,
}

fn image_type(b: &[u8]) -> Option<ImageType> {
    if is_png(b) {
        Some(ImageType::Png)
    } else if is_jpeg(b) {
        Some(ImageType::Jpeg)
    } else {
        None
    }
}

/// JS: embed-prompt.mjs#readPrompt(imagePath, buffer) (fix #641).
fn read_prompt(image_path: &str, b: &[u8]) -> Option<String> {
    let mut prompt = match image_type(b) {
        Some(ImageType::Png) => parse_png(b).1,
        Some(ImageType::Jpeg) => read_jpeg_com(b),
        None => None,
    };
    if prompt.is_none() {
        prompt = sidecar_prompt(image_path);
    }
    prompt
}

fn sidecar_prompt(image_path: &str) -> Option<String> {
    let sc = format!("{}.json", image_path);
    if !exists(&sc) {
        return None;
    }
    let text = std::fs::read_to_string(&sc).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    match v.get("prompt") {
        Some(Value::Null) | None => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(other) => Some(crate::critique_storage::js_string_value(other)),
    }
}

/// Reads embedded provenance, then the `<path>.json` sidecar used by `embed-prompt`.
pub fn read_provenance(image_path: &str) -> Option<String> {
    let b = std::fs::read(image_path).ok()?;
    read_prompt(image_path, &b)
}

fn walk(p: &str, is_root: bool, rasters: &mut Vec<String>) -> Result<(), String> {
    let md = std::fs::metadata(p).map_err(|e| e.to_string())?;
    if md.is_dir() {
        let trimmed = p.trim_end_matches('/');
        let base = trimmed.rsplit('/').next().unwrap_or("");
        if !is_root && (base == "node_modules" || base.starts_with('.')) {
            return Ok(());
        }
        let Some(entries) = read_dir_names(p) else { return Ok(()) };
        for e in entries {
            walk(&format!("{}/{}", trimmed, e), false, rasters)?;
        }
    } else if is_raster(p) {
        rasters.push(p.to_string());
    }
    Ok(())
}

fn is_raster(p: &str) -> bool {
    let lower = p.to_ascii_lowercase();
    lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.ends_with(".webp")
}

/// How provenance is carried by an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Carried {
    PngText,
    JpegCom,
    Sidecar,
}

/// Writes provenance using the same carriers as `embed-prompt`.
/// JPEG text exceeding one COM segment is stored in a sidecar instead.
/// Returns an error if the image cannot be read or its provenance cannot be written.
pub fn write_provenance(path: &str, text: &str) -> Result<Carried, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    write_text(path, &bytes, text, true)
}

fn write_text(
    path: &str,
    buf: &[u8],
    prompt: &str,
    long_jpeg_sidecar: bool,
) -> Result<Carried, String> {
    match image_type(buf) {
        Some(ImageType::Png) => {
            // JS-PARITY: embed-prompt.mjs#641 finds IEND by walking the PNG
            // chunks (not buf.indexOf('IEND')) and reuses parsePng's chunk list
            // and prompt to rebuild the body idempotently.
            let (chunks, existing) = parse_png(buf);
            let iend: i64 = chunks
                .iter()
                .find(|c| &c.ty == b"IEND")
                .map(|c| c.offset as i64)
                .unwrap_or(-1);
            if iend < 8 {
                return Err("embed-prompt: malformed PNG".into());
            }
            let iend = iend as usize;
            let mut text_data = KEYWORD.to_vec();
            text_data.push(0);
            text_data.extend_from_slice(prompt.as_bytes());
            let text_chunk = png_chunk(b"tEXt", &text_data);
            let out: Vec<u8> = if existing.is_some() {
                let mut body: Vec<u8> = Vec::new();
                for c in &chunks {
                    if c.offset < iend && !c.prompt_chunk {
                        body.extend_from_slice(&buf[c.offset..c.end]);
                    }
                }
                let mut o = buf[..8].to_vec();
                o.extend_from_slice(&body);
                o.extend_from_slice(&text_chunk);
                o.extend_from_slice(&png_chunk(b"IEND", &[]));
                o
            } else {
                let mut o = buf[..iend].to_vec();
                o.extend_from_slice(&text_chunk);
                o.extend_from_slice(&buf[iend..]);
                o
            };
            std::fs::write(path, out).map_err(|e| e.to_string())?;
            Ok(Carried::PngText)
        }
        Some(ImageType::Jpeg) => {
            let mut seg = KEYWORD.to_vec();
            seg.push(0);
            seg.extend_from_slice(prompt.as_bytes());
            if seg.len() + 2 > 0xffff {
                if long_jpeg_sidecar {
                    // Embedded text wins during reads, so remove an older COM provenance.
                    let mut output = buf[..2].to_vec();
                    let mut offset = 2;
                    while offset + 4 <= buf.len() && buf[offset] == 0xff && buf[offset + 1] != 0xda
                    {
                        let length =
                            u16::from_be_bytes([buf[offset + 2], buf[offset + 3]]) as usize;
                        let end = offset + 2 + length;
                        if length < 2 || end > buf.len() {
                            break;
                        }
                        if !(buf[offset + 1] == 0xfe
                            && buf[offset + 4..end].starts_with(&[KEYWORD, &[0]].concat()))
                        {
                            output.extend_from_slice(&buf[offset..end]);
                        }
                        offset = end;
                    }
                    output.extend_from_slice(&buf[offset..]);
                    write_sidecar(path, prompt)?;
                    std::fs::write(path, output).map_err(|e| e.to_string())?;
                    return Ok(Carried::Sidecar);
                }
                return Err("embed-prompt: prompt too long for a JPEG segment".into());
            }
            let mut com = vec![0xff, 0xfe];
            com.extend_from_slice(&((seg.len() + 2) as u16).to_be_bytes());
            com.extend_from_slice(&seg);
            let mut out = buf[..2].to_vec();
            out.extend_from_slice(&com);
            out.extend_from_slice(&buf[2..]);
            std::fs::write(path, out).map_err(|e| e.to_string())?;
            Ok(Carried::JpegCom)
        }
        None => write_sidecar(path, prompt),
    }
}

fn write_sidecar(path: &str, prompt: &str) -> Result<Carried, String> {
    let mut m = Map::new();
    m.insert("prompt".into(), Value::String(prompt.into()));
    m.insert("createdAt".into(), Value::String(iso_now()));
    std::fs::write(format!("{path}.json"), json_pretty(&Value::Object(m)))
        .map_err(|e| e.to_string())?;
    Ok(Carried::Sidecar)
}

pub fn run(args: &[String], io: &mut Io) -> i32 {
    let cwd = io.cwd.to_string_lossy().into_owned();
    let abs = |p: &str| -> String { jsp::resolve(&cwd, &[p]) };
    let file = args.iter().find(|a| !a.starts_with("--")).cloned();
    let read_mode = args.iter().any(|a| a == "--read");
    let scan_mode = args.iter().any(|a| a == "--scan");
    let arg_of = |name: &str| -> Option<String> {
        let i = args.iter().position(|a| a == name)?;
        args.get(i + 1).cloned()
    };

    if scan_mode {
        let targets: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
        if targets.is_empty() {
            io.err("embed-prompt: --scan needs at least one directory\n");
            return 1;
        }
        let mut rasters: Vec<String> = Vec::new();
        for t in &targets {
            if !exists(&abs(t)) {
                io.err(&format!("embed-prompt: no such path {}\n", t));
                return 1;
            }
            // JS walks with the path as given (relative to cwd); output uses that spelling.
            let saved = std::env::current_dir().ok();
            let _ = std::env::set_current_dir(&cwd);
            let r = walk(t, true, &mut rasters);
            if let Some(s) = saved {
                let _ = std::env::set_current_dir(s);
            }
            if let Err(e) = r {
                io.err(&format!("Error: {}\n", e));
                return 1;
            }
        }
        let mut missing = 0;
        for r in &rasters {
            if read_provenance(&abs(r)).is_none() {
                io.out(&format!("MISSING: {}\n", r));
                missing += 1;
            }
        }
        io.out(&format!(
            "SCAN: {} raster{}, {} missing\n",
            rasters.len(),
            if rasters.len() == 1 { "" } else { "s" },
            missing
        ));
        return if missing > 0 { 3 } else { 0 };
    }

    let Some(file) = file.filter(|f| exists(&abs(f))) else {
        io.err("embed-prompt: image file required\n");
        return 1;
    };
    let file_abs = abs(&file);
    let buf = std::fs::read(&file_abs).unwrap_or_default();
    let sidecar = format!("{}.json", file);

    if read_mode {
        let prompt = read_prompt(&file_abs, &buf);
        match prompt {
            None => {
                io.err("embed-prompt: no embedded prompt found\n");
                2
            }
            Some(p) => {
                io.out(&format!("{}\n", p));
                0
            }
        }
    } else {
        let prompt = match arg_of("--prompt") {
            Some(p) => Some(p),
            None => match arg_of("--prompt-file") {
                Some(pf) if !pf.is_empty() => match std::fs::read(abs(&pf)) {
                    Ok(b) => Some(String::from_utf8_lossy(&b).into_owned()),
                    Err(e) => {
                        io.err(&format!("Error: {}\n", crate::util::node_read_error(&pf, &e)));
                        return 1;
                    }
                },
                _ => None,
            },
        };
        let Some(prompt) = prompt.filter(|p| !p.is_empty()) else {
            io.err("embed-prompt: --prompt or --prompt-file required\n");
            return 1;
        };
        let plen = utf16_len(&prompt);
        match write_text(&file_abs, &buf, &prompt, false) {
            Ok(Carried::PngText) => {
                io.out(&format!("EMBEDDED: {} (png tEXt, {} chars)\n", file, plen));
                0
            }
            Ok(Carried::JpegCom) => {
                io.out(&format!("EMBEDDED: {} (jpeg COM, {} chars)\n", file, plen));
                0
            }
            Ok(Carried::Sidecar) => {
                io.out(&format!(
                    "EMBEDDED: {} (sidecar fallback for this format)\n",
                    sidecar
                ));
                0
            }
            Err(error) => {
                io.err(&format!("{error}\n"));
                1
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_roundtrips_and_long_jpeg_uses_sidecar() {
        let dir = std::env::temp_dir().join(format!("phoenix-provenance-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend(png_chunk(b"IEND", &[]));
        for (name, bytes, expected) in [
            ("image.png", png, Carried::PngText),
            ("image.jpg", vec![0xff, 0xd8, 0xff, 0xd9], Carried::JpegCom),
            ("image.webp", b"RIFF0000WEBP".to_vec(), Carried::Sidecar),
        ] {
            let path = dir.join(name).to_string_lossy().into_owned();
            std::fs::write(&path, bytes).unwrap();
            assert_eq!(
                write_provenance(&path, "source: 项目图片").unwrap(),
                expected
            );
            assert_eq!(read_provenance(&path).as_deref(), Some("source: 项目图片"));
        }
        let path = dir.join("long.jpg").to_string_lossy().into_owned();
        std::fs::write(&path, [0xff, 0xd8, 0xff, 0xd9]).unwrap();
        write_provenance(&path, "old embedded text").unwrap();
        let text = "源".repeat(30_000);
        assert_eq!(write_provenance(&path, &text).unwrap(), Carried::Sidecar);
        assert_eq!(read_provenance(&path), Some(text));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
