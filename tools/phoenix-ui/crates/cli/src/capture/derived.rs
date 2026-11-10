//! Derived evidence: sequence frames, glance thumbnails, numbered marks,
//! state sheets and before/after stitches. Layouts that carry text are
//! composed in the same headless browser, so no image or font crate is
//! needed. Derived images help a reviewer read the state screenshots; they
//! never replace them.

use std::path::Path;

use base64::Engine as _;
use impeccable_browser::cdp::Browser;
use impeccable_common::Io;
use impeccable_comp::raster::{self, Image};

/// Largest composed side, so a composition stays one screenshot.
pub const MAX_SIDE: f64 = 16384.0;

pub const FRAMES_NOTE: &str = "序列图只表示状态变化，不代表正常速度手感已核对";

/// For `n` evenly spaced instants over `[t0, t1]`, the index of the latest
/// frame captured at or before each instant. An instant with no earlier
/// frame is `None`: a later frame is never shown as an earlier moment.
pub fn sample_frames(times: &[f64], t0: f64, t1: f64, n: usize) -> Vec<Option<usize>> {
    (0..n)
        .map(|i| {
            let at = if n > 1 {
                t0 + (t1 - t0) * i as f64 / (n - 1) as f64
            } else {
                t0
            };
            times
                .iter()
                .enumerate()
                .filter(|(_, t)| **t <= at)
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(i, _)| i)
        })
        .collect()
}

/// Glance thumbnails: 480 px wide for desktop-width viewports, 180 px for
/// phone widths.
pub fn thumb_width(viewport_css_width: u32) -> f64 {
    if viewport_css_width >= 600 {
        480.0
    } else {
        180.0
    }
}

/// The first viewport of a screenshot, scaled to `width`.
pub fn thumbnail(image: &Image, viewport: (usize, usize), width: f64) -> Image {
    let first = raster::crop(
        image,
        0.0,
        0.0,
        viewport.0.min(image.width) as f64,
        viewport.1.min(image.height) as f64,
    );
    let height = first.height as f64 * width / first.width as f64;
    raster::resize(&first, width, height)
}

/// Cell size, then each image's source rectangle in its own pixels.
pub type StitchGeometry = ((f64, f64), Vec<[f64; 4]>);

/// Scale and crop for a stitch: image `i` is scaled by `matches[0] /
/// matches[i]`, so the shared element has one width everywhere; `crop` is in
/// that common (first image) pixel space. Returns the cell size and, per
/// image, the source rectangle in its own pixels.
pub fn stitch_geometry(
    sizes: &[(f64, f64)],
    matches: &[f64],
    crop: Option<[f64; 4]>,
) -> Result<StitchGeometry, String> {
    let scales: Vec<f64> = matches.iter().map(|m| matches[0] / m).collect();
    let scaled: Vec<(f64, f64)> = sizes
        .iter()
        .zip(&scales)
        .map(|((w, h), s)| (w * s, h * s))
        .collect();
    let region = match crop {
        Some(c) => {
            for (i, (w, h)) in scaled.iter().enumerate() {
                if c[0] + c[2] > w + 1e-6 || c[1] + c[3] > h + 1e-6 {
                    return Err(format!(
                        "--crop {},{},{},{} 超出第 {} 张图（统一比例后 {}×{}）；缩小裁切范围，或先去掉 --crop 查看公共区域",
                        c[0], c[1], c[2], c[3], i + 1, w.round(), h.round()
                    ));
                }
            }
            c
        }
        None => {
            let w = scaled.iter().map(|s| s.0).fold(f64::INFINITY, f64::min);
            let h = scaled.iter().map(|s| s.1).fold(f64::INFINITY, f64::min);
            [0.0, 0.0, w, h]
        }
    };
    let sources = scales
        .iter()
        .map(|s| [region[0] / s, region[1] / s, region[2] / s, region[3] / s])
        .collect();
    Ok(((region[2], region[3]), sources))
}

/// One element of a composed image, in CSS pixels at scale 1.
pub enum Piece {
    /// `crop` (source pixels) of an image of `natural` size, drawn into the box.
    Image {
        src: String,
        natural: (f64, f64),
        crop: [f64; 4],
        rect: [f64; 4],
    },
    Placeholder {
        rect: [f64; 4],
        text: String,
    },
    Frame {
        rect: [f64; 4],
    },
    Badge {
        x: f64,
        y: f64,
        text: String,
    },
    Text {
        x: f64,
        y: f64,
        width: f64,
        text: String,
    },
}

pub struct Layout {
    pub width: f64,
    pub height: f64,
    pub pieces: Vec<Piece>,
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn png_url(bytes: &[u8]) -> String {
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

fn html(layout: &Layout) -> String {
    let mut body = String::new();
    let place = |r: &[f64; 4]| {
        format!(
            "left:{}px;top:{}px;width:{}px;height:{}px",
            r[0], r[1], r[2], r[3]
        )
    };
    for piece in &layout.pieces {
        body.push_str(&match piece {
            Piece::Image { src, natural, crop, rect } => {
                let s = rect[2] / crop[2];
                format!(
                    "<div class=p style=\"{};overflow:hidden\"><img src=\"{src}\" style=\"position:absolute;left:{}px;top:{}px;width:{}px;height:{}px\"></div>",
                    place(rect), -crop[0] * s, -crop[1] * s, natural.0 * s, natural.1 * s
                )
            }
            Piece::Placeholder { rect, text } => format!(
                "<div class=\"p hole\" style=\"{}\">{}</div>",
                place(rect),
                escape(text)
            ),
            Piece::Frame { rect } => format!("<div class=\"p box\" style=\"{}\"></div>", place(rect)),
            Piece::Badge { x, y, text } => format!(
                "<div class=\"p badge\" style=\"left:{x}px;top:{y}px\">{}</div>",
                escape(text)
            ),
            Piece::Text { x, y, width, text } => format!(
                "<div class=\"p label\" style=\"left:{x}px;top:{y}px;width:{width}px\">{}</div>",
                escape(text)
            ),
        });
    }
    format!(
        "<!doctype html><meta charset=utf-8><style>html,body{{margin:0;background:#ececea;color:#333;font:13px/20px -apple-system,\"PingFang SC\",\"Noto Sans CJK SC\",\"Microsoft YaHei\",sans-serif}}\
.p{{position:absolute}}.label{{white-space:nowrap;overflow:hidden;text-overflow:ellipsis}}\
.hole{{display:grid;place-items:center;background:#d6d6d2;color:#666}}\
.box{{box-sizing:border-box;border:2px solid #e8175d;border-radius:3px}}\
.badge{{min-width:22px;height:22px;padding:0 6px;box-sizing:border-box;border-radius:11px;background:#e8175d;color:#fff;font-weight:600;line-height:22px;text-align:center;box-shadow:0 0 0 2px #fff}}</style><body>{body}</body>"
    )
}

/// Render a layout to PNG in a fresh page of `browser`.
pub fn compose(browser: &mut Browser, layout: &Layout) -> Result<Vec<u8>, String> {
    let (w, h) = (layout.width.ceil(), layout.height.ceil());
    if w > MAX_SIDE || h > MAX_SIDE {
        return Err(format!("派生图尺寸 {w}×{h} 超过每边 {MAX_SIDE}px，未产出"));
    }
    let mut page = browser.new_isolated_page().map_err(|e| e.message)?;
    let result = (|| {
        page.set_device_metrics(w as u32, h as u32, 1.0)?;
        let document = serde_json::to_string(&html(layout)).unwrap_or_default();
        page.evaluate_value(&format!(
            "(async (html) => {{ document.open(); document.write(html); document.close();\
              await Promise.all([...document.images].map((i) => i.decode()));\
              await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))); return true; }})({document})"
        ))?;
        page.screenshot(None, false)
    })();
    page.close();
    let b64 = result.map_err(|e| format!("派生图合成失败：{}", e.message))?;
    base64::engine::general_purpose::STANDARD
        .decode(b64.as_bytes())
        .map_err(|e| format!("派生图合成失败：{e}"))
}

const MARGIN: f64 = 16.0;
const GAP: f64 = 12.0;
const LABEL: f64 = 22.0;

/// A labelled grid of equally sized cells.
fn grid(cells: Vec<(String, Option<Piece>)>, cell: (f64, f64), columns: usize) -> Layout {
    let columns = columns.clamp(1, cells.len().max(1));
    let rows = cells.len().div_ceil(columns);
    let mut pieces = Vec::new();
    for (i, (label, content)) in cells.into_iter().enumerate() {
        let x = MARGIN + (i % columns) as f64 * (cell.0 + GAP);
        let y = MARGIN + (i / columns) as f64 * (cell.1 + LABEL + GAP);
        pieces.push(Piece::Text {
            x,
            y,
            width: cell.0,
            text: label,
        });
        let rect = [x, y + LABEL, cell.0, cell.1];
        pieces.push(match content {
            Some(Piece::Image {
                src, natural, crop, ..
            }) => Piece::Image {
                src,
                natural,
                crop,
                rect,
            },
            _ => Piece::Placeholder {
                rect,
                text: "无帧".into(),
            },
        });
    }
    Layout {
        width: MARGIN * 2.0 + columns as f64 * cell.0 + (columns - 1) as f64 * GAP,
        height: MARGIN * 2.0 + rows as f64 * (cell.1 + LABEL) + rows.saturating_sub(1) as f64 * GAP,
        pieces,
    }
}

/// Sequence frames: one cell per sampled instant, labelled `+<ms> ms`.
/// `frames` are base64 JPEGs of `device` size; `cells` are offsets from the
/// window start with the chosen frame.
pub fn frames_layout(
    cells: &[(u64, Option<&str>)],
    device: (f64, f64),
    viewport_css_width: u32,
) -> Layout {
    let width = (viewport_css_width as f64).min(480.0);
    let size = (width, width * device.1 / device.0);
    let items = cells
        .iter()
        .map(|(ms, frame)| {
            let label = match frame {
                Some(_) => format!("+{ms} ms"),
                None => format!("+{ms} ms · 无帧"),
            };
            let piece = frame.map(|data| Piece::Image {
                src: format!("data:image/jpeg;base64,{data}"),
                natural: device,
                crop: [0.0, 0.0, device.0, device.1],
                rect: [0.0; 4],
            });
            (label, piece)
        })
        .collect();
    grid(items, size, 4)
}

/// A screenshot with numbered 2 px boxes; `marks` are `(number, box)` in
/// image pixels.
pub fn mark_layout(png: &[u8], size: (f64, f64), marks: &[(usize, [f64; 4])]) -> Layout {
    let mut pieces = vec![Piece::Image {
        src: png_url(png),
        natural: size,
        crop: [0.0, 0.0, size.0, size.1],
        rect: [0.0, 0.0, size.0, size.1],
    }];
    for (number, m) in marks {
        let x = (m[0] - 2.0).clamp(0.0, size.0 - 4.0);
        let y = (m[1] - 2.0).clamp(0.0, size.1 - 4.0);
        let w = (m[2] + 4.0).min(size.0 - x).max(4.0);
        let h = (m[3] + 4.0).min(size.1 - y).max(4.0);
        pieces.push(Piece::Frame { rect: [x, y, w, h] });
        pieces.push(Piece::Badge {
            x: (x - 11.0).clamp(0.0, (size.0 - 22.0).max(0.0)),
            y: (y - 11.0).clamp(0.0, (size.1 - 22.0).max(0.0)),
            text: number.to_string(),
        });
    }
    Layout {
        width: size.0,
        height: size.1,
        pieces,
    }
}

/// The first viewport of each state's screenshot side by side, named.
pub fn sheet_layout(
    items: &[(String, &[u8], (f64, f64))],
    viewport: (f64, f64),
    viewport_css_width: u32,
) -> Layout {
    let width = if viewport_css_width >= 600 {
        480.0
    } else {
        240.0
    };
    let crop_h = items
        .iter()
        .map(|(_, _, size)| size.1.min(viewport.1))
        .fold(viewport.1, f64::min);
    let cell = (width, width * crop_h / viewport.0);
    let cells = items
        .iter()
        .map(|(label, png, size)| {
            let crop = [0.0, 0.0, viewport.0.min(size.0), crop_h];
            let piece = Piece::Image {
                src: png_url(png),
                natural: *size,
                crop,
                rect: [0.0; 4],
            };
            (label.clone(), Some(piece))
        })
        .collect();
    grid(cells, cell, 4)
}

pub const STITCH_USAGE: &str = "用法：phoenix-ui capture stitch <图片> <图片>... --match <px,px,...> [--crop X,Y,W,H] [--label a,b,...] [--columns N] --out <文件.png>

把修改前后的同一区域截图按共同元素统一比例后拼成一张，带标签。
  --match <px,...>   每张图里同一个共同元素的像素宽度，按图片顺序；第 i 张缩放比 = 第 1 个值 / 第 i 个值
  --crop X,Y,W,H     在统一比例后的坐标（即第 1 张图的像素空间）里取的区域；超出任一张图即报错且不写文件；
                     不给时取各图统一比例后的公共左上区域
  --label a,b,...    每张图的标签，默认 1,2,...
  --columns N        列数，默认 2
  --out <文件.png>   输出文件
只接受 PNG；需要本机 Chrome、Chromium、Edge 或 Brave。退出码：0 已写出；1 参数、图片或浏览器问题，未写文件。
";

/// `phoenix-ui capture stitch`.
pub fn run_stitch(args: &[String], io: &mut Io) -> i32 {
    match stitch(args, io) {
        Ok(Some(line)) => {
            io.out(&format!("{line}\n"));
            0
        }
        Ok(None) => {
            io.out(STITCH_USAGE);
            0
        }
        Err(message) => {
            io.err(&format!("capture stitch: {message}\n"));
            1
        }
    }
}

fn numbers(value: &str, flag: &str) -> Result<Vec<f64>, String> {
    value
        .split(',')
        .map(|v| match v.trim().parse::<f64>() {
            Ok(n) if n.is_finite() && n >= 0.0 => Ok(n),
            _ => Err(format!("{flag} 应为逗号分隔的非负数，收到 {value}")),
        })
        .collect()
}

fn stitch(args: &[String], io: &mut Io) -> Result<Option<String>, String> {
    let mut images = Vec::new();
    let (mut matches, mut crop, mut labels, mut columns, mut out) = (None, None, None, None, None);
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        i += 1;
        if arg == "--help" || arg == "-h" {
            return Ok(None);
        }
        if !arg.starts_with('-') {
            images.push(io.cwd.join(arg));
            continue;
        }
        let (name, inline) = match arg.split_once('=') {
            Some((n, v)) => (n, Some(v.to_string())),
            None => (arg, None),
        };
        if !matches!(
            name,
            "--match" | "--crop" | "--label" | "--columns" | "--out"
        ) {
            return Err(format!("未知选项 {name}"));
        }
        let value = match inline {
            Some(v) => v,
            None => {
                let v = args
                    .get(i)
                    .cloned()
                    .ok_or_else(|| format!("{name} 需要一个值"))?;
                i += 1;
                v
            }
        };
        match name {
            "--match" => matches = Some(numbers(&value, name)?),
            "--crop" => crop = Some(numbers(&value, name)?),
            "--label" => labels = Some(value.split(',').map(str::to_string).collect::<Vec<_>>()),
            "--columns" => {
                columns = Some(
                    value
                        .parse::<usize>()
                        .ok()
                        .filter(|c| *c >= 1)
                        .ok_or("--columns 应为正整数")?,
                )
            }
            _ => out = Some(io.cwd.join(value)),
        }
    }
    if images.len() < 2 {
        return Err("至少给两张图片".into());
    }
    let out = out.ok_or("缺少 --out <文件.png>")?;
    let matches = matches.ok_or("缺少 --match：每张图里同一个共同元素的像素宽度")?;
    if matches.len() != images.len() || matches.iter().any(|m| *m <= 0.0) {
        return Err(format!(
            "--match 需要 {} 个正数，与图片一一对应",
            images.len()
        ));
    }
    let crop = match crop {
        None => None,
        Some(c) if c.len() == 4 && c[2] > 0.0 && c[3] > 0.0 => Some([c[0], c[1], c[2], c[3]]),
        Some(_) => return Err("--crop 写成 X,Y,W,H，宽高为正".into()),
    };
    let labels = labels.unwrap_or_else(|| (1..=images.len()).map(|n| n.to_string()).collect());
    if labels.len() != images.len() {
        return Err(format!(
            "--label 需要 {} 个标签，与图片一一对应",
            images.len()
        ));
    }
    let mut loaded = Vec::new();
    for (n, path) in images.iter().enumerate() {
        let bytes = std::fs::read(path)
            .map_err(|e| format!("第 {} 张图片 {} 读取失败：{e}", n + 1, path.display()))?;
        let image = impeccable_comp::png_io::decode_png(&bytes)
            .map_err(|_| format!("第 {} 张图片 {} 不是 PNG", n + 1, path.display()))?
            .image;
        loaded.push((bytes, (image.width as f64, image.height as f64)));
    }
    let sizes: Vec<(f64, f64)> = loaded.iter().map(|(_, s)| *s).collect();
    let (cell, sources) = stitch_geometry(&sizes, &matches, crop)?;
    let cells = loaded
        .iter()
        .zip(sources)
        .zip(&labels)
        .map(|(((bytes, size), source), label)| {
            let piece = Piece::Image {
                src: png_url(bytes),
                natural: *size,
                crop: source,
                rect: [0.0; 4],
            };
            (label.clone(), Some(piece))
        })
        .collect();
    let layout = grid(cells, cell, columns.unwrap_or(2));
    let mut browser = impeccable_browser::BrowserEngine::new(io.env.clone())
        .launch()
        .map_err(|e| format!("浏览器不可用：{}", e.message))?;
    let composed = compose(&mut browser, &layout);
    browser.close();
    let png = composed?;
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("无法创建目录 {}：{e}", parent.display()))?;
    }
    std::fs::write(&out, &png).map_err(|e| format!("无法写入 {}：{e}", out.display()))?;
    Ok(Some(format!(
        "stitch: {} ({}×{}, 单格 {}×{})",
        display(&out, &io.cwd),
        layout.width.ceil(),
        layout.height.ceil(),
        cell.0.round(),
        cell.1.round()
    )))
}

fn display(path: &Path, cwd: &Path) -> String {
    path.strip_prefix(cwd).unwrap_or(path).display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_are_sampled_evenly_and_never_from_the_future() {
        let times = [10.00, 10.05, 10.32, 10.60];
        // Window 10.0–10.6 s in 4 cells: 0, 200, 400, 600 ms.
        assert_eq!(
            sample_frames(&times, 10.0, 10.6, 4),
            [Some(0), Some(1), Some(2), Some(3)]
        );
        // The 200 ms cell holds the 50 ms frame; it never jumps to 320 ms.
        assert_eq!(sample_frames(&times, 10.0, 10.6, 4)[1], Some(1));
        // Instants before the first frame have no frame instead of a later one.
        assert_eq!(
            sample_frames(&[10.3, 10.5], 10.0, 10.6, 4),
            [None, None, Some(0), Some(1)]
        );
        assert_eq!(sample_frames(&[], 1.0, 2.0, 3), [None, None, None]);
    }

    #[test]
    fn stitch_scales_to_the_shared_element_and_checks_the_crop() {
        // The shared element is 320 px in both: same scale.
        let ((w, h), sources) = stitch_geometry(
            &[(1440.0, 900.0), (1440.0, 900.0)],
            &[320.0, 320.0],
            Some([0.0, 0.0, 1440.0, 600.0]),
        )
        .unwrap();
        assert_eq!((w, h), (1440.0, 600.0));
        assert_eq!(sources[1], [0.0, 0.0, 1440.0, 600.0]);
        // A 2x screenshot whose element is 640 px is halved; the crop maps back to its pixels.
        let (_, sources) = stitch_geometry(
            &[(1440.0, 900.0), (2880.0, 1800.0)],
            &[320.0, 640.0],
            Some([100.0, 50.0, 400.0, 300.0]),
        )
        .unwrap();
        assert_eq!(sources[1], [200.0, 100.0, 800.0, 600.0]);
        // Without a crop: the common top-left region after scaling.
        let ((w, h), _) =
            stitch_geometry(&[(1440.0, 900.0), (1200.0, 1000.0)], &[300.0, 300.0], None).unwrap();
        assert_eq!((w, h), (1200.0, 900.0));
        // Out of range on any image is an error, not a clipped picture.
        let err = stitch_geometry(
            &[(1440.0, 900.0), (1440.0, 500.0)],
            &[320.0, 320.0],
            Some([0.0, 0.0, 1440.0, 600.0]),
        )
        .unwrap_err();
        assert!(err.contains("超出第 2 张图"), "{err}");
    }

    #[test]
    fn thumbnails_follow_the_viewport_width() {
        assert_eq!(thumb_width(1440), 480.0);
        assert_eq!(thumb_width(600), 480.0);
        assert_eq!(thumb_width(390), 180.0);
        let tall = raster::create_image(780, 3000, [255, 255, 255, 255]);
        let thumb = thumbnail(&tall, (780, 1688), 180.0);
        assert_eq!((thumb.width, thumb.height), (180, 390));
    }
}
