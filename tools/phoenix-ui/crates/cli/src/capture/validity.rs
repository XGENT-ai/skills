//! Self-check of a main screenshot. A shot that fails any rule is written as
//! `<stem>.invalid.png` and must not reach a reviewer. Whether the picture is
//! the state its file name claims stays the caller's call.

use impeccable_comp::raster::Image;

/// What the page measured at the moment of the screenshot.
#[derive(Debug, Clone, Default)]
pub struct ShotMeasure {
    pub finite_animations: u64,
    pub text_chars: u64,
    pub transparent_chars: u64,
    /// Visible canvases ≥32×32 in CSS pixels, relative to the shot origin.
    pub canvases: Vec<CanvasBox>,
    pub root_luminance: f64,
}

#[derive(Debug, Clone)]
pub struct CanvasBox {
    pub selector: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Framing {
    /// The current viewport.
    Viewport,
    /// From the document top, viewport-wide, `doc_height` CSS pixels tall.
    Full { doc_height: f64, top: f64 },
    /// An element's border box (CSS pixels as clipped).
    Element { width: f64, height: f64 },
}

#[derive(Debug, Clone, Copy)]
pub struct ShotSpec {
    pub framing: Framing,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub dpr: f64,
}

const TOLERANCE: i64 = 2;

/// Reasons the shot is invalid; empty means valid.
pub fn check(image: &Image, measure: &ShotMeasure, spec: &ShotSpec) -> Vec<String> {
    let mut reasons = Vec::new();
    if measure.finite_animations > 0 {
        reasons.push(format!(
            "动画未稳定（{} 个仍在运行）；加大 --settle-ms 或用 --reduced-motion",
            measure.finite_animations
        ));
    }
    if measure.text_chars >= 20 && measure.transparent_chars * 2 >= measure.text_chars {
        reasons.push("主要内容透明，入场动效未稳定或停在隐藏态".to_string());
    }
    if image.width > 0 && image.height > 0 && uniform(image, 0, 0, image.width, image.height) {
        reasons.push("整张为单一颜色，页面未绘出".to_string());
    } else if measure.root_luminance >= 0.2 && black_band(image) * 4 >= image.height {
        reasons.push("出现大面积全黑区域".to_string());
    }
    for canvas in &measure.canvases {
        let px = |v: f64| (v * spec.dpr).round().max(0.0) as usize;
        let (x, y) = (px(canvas.x), px(canvas.y));
        let x1 = px(canvas.x + canvas.width).min(image.width);
        let y1 = px(canvas.y + canvas.height).min(image.height);
        if x1 > x && y1 > y && uniform(image, x, y, x1 - x, y1 - y) {
            reasons.push(format!("画布未绘出：{}", canvas.selector));
        }
    }
    if let Some(reason) = framing_error(image, spec) {
        reasons.push(reason);
    }
    reasons
}

fn framing_error(image: &Image, spec: &ShotSpec) -> Option<String> {
    let device = |css: f64| (css * spec.dpr).round() as i64;
    let (w, h) = (image.width as i64, image.height as i64);
    let near = |a: i64, b: i64| (a - b).abs() <= 1;
    let (ew, eh, what) = match spec.framing {
        Framing::Viewport => (
            device(spec.viewport_width as f64),
            device(spec.viewport_height as f64),
            "视口图",
        ),
        Framing::Full { doc_height, top } => {
            if top != 0.0 {
                return Some(format!("整页图没有从文档顶部开始（起点 y={top}）"));
            }
            (
                device(spec.viewport_width as f64),
                device(doc_height),
                "整页图",
            )
        }
        Framing::Element { width, height } => (device(width), device(height), "元素图"),
    };
    let ok = match spec.framing {
        Framing::Viewport => w == ew && h == eh,
        _ => near(w, ew) && near(h, eh),
    };
    (!ok).then(|| format!("{what}尺寸 {w}×{h} 与预期 {ew}×{eh} 不符"))
}

/// Every pixel within ±2 of one color on each channel.
fn uniform(image: &Image, x: usize, y: usize, width: usize, height: usize) -> bool {
    let mut lo = [255i64; 3];
    let mut hi = [0i64; 3];
    for row in y..y + height {
        let start = (row * image.width + x) * 4;
        for px in image.data[start..start + width * 4].as_chunks::<4>().0 {
            for c in 0..3 {
                let v = px[c] as i64;
                lo[c] = lo[c].min(v);
                hi[c] = hi[c].max(v);
            }
            if (0..3).any(|c| hi[c] - lo[c] > TOLERANCE * 2) {
                return false;
            }
        }
    }
    true
}

/// Height of the tallest run of rows whose every pixel is near black.
fn black_band(image: &Image) -> usize {
    let mut best = 0;
    let mut run = 0;
    for row in image.data.chunks_exact(image.width * 4) {
        if row
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| p[0] <= 8 && p[1] <= 8 && p[2] <= 8)
        {
            run += 1;
            best = best.max(run);
        } else {
            run = 0;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use impeccable_comp::raster::{create_image, fill_rect};

    fn spec(framing: Framing) -> ShotSpec {
        ShotSpec {
            framing,
            viewport_width: 100,
            viewport_height: 80,
            dpr: 1.0,
        }
    }

    fn textured(width: usize, height: usize) -> Image {
        let mut image = create_image(width, height, [255, 255, 255, 255]);
        fill_rect(
            &mut image,
            10.0,
            10.0,
            30.0,
            10.0,
            [20.0, 40.0, 200.0, 255.0],
        );
        image
    }

    fn measure() -> ShotMeasure {
        ShotMeasure {
            root_luminance: 1.0,
            ..Default::default()
        }
    }

    #[test]
    fn a_normal_viewport_shot_is_valid() {
        assert!(check(&textured(100, 80), &measure(), &spec(Framing::Viewport)).is_empty());
    }

    #[test]
    fn single_color_and_black_band() {
        let white = create_image(100, 80, [254, 255, 253, 255]);
        assert_eq!(
            check(&white, &measure(), &spec(Framing::Viewport)),
            ["整张为单一颜色，页面未绘出"]
        );
        let mut banded = textured(100, 80);
        fill_rect(&mut banded, 0.0, 40.0, 100.0, 20.0, [0.0, 0.0, 0.0, 255.0]);
        assert_eq!(
            check(&banded, &measure(), &spec(Framing::Viewport)),
            ["出现大面积全黑区域"]
        );
        // A dark page may be black by design.
        let dark = ShotMeasure {
            root_luminance: 0.05,
            ..Default::default()
        };
        assert!(check(&banded, &dark, &spec(Framing::Viewport)).is_empty());
        // Under a quarter of the height is not a band.
        let mut thin = textured(100, 80);
        fill_rect(&mut thin, 0.0, 40.0, 100.0, 19.0, [0.0, 0.0, 0.0, 255.0]);
        assert!(check(&thin, &measure(), &spec(Framing::Viewport)).is_empty());
    }

    #[test]
    fn blank_canvas_is_named() {
        let mut m = measure();
        m.canvases.push(CanvasBox {
            selector: "#chart".into(),
            x: 50.0,
            y: 40.0,
            width: 40.0,
            height: 32.0,
        });
        assert_eq!(
            check(&textured(100, 80), &m, &spec(Framing::Viewport)),
            ["画布未绘出：#chart"]
        );
        m.canvases[0].x = 0.0;
        m.canvases[0].y = 0.0;
        assert!(check(&textured(100, 80), &m, &spec(Framing::Viewport)).is_empty());
    }

    #[test]
    fn framing_follows_the_shot_type() {
        let tall = textured(100, 300);
        let full = spec(Framing::Full {
            doc_height: 300.0,
            top: 0.0,
        });
        assert!(
            check(&tall, &measure(), &full).is_empty(),
            "a full page may be taller than the viewport"
        );
        assert_eq!(
            check(&tall, &measure(), &spec(Framing::Viewport)),
            ["视口图尺寸 100×300 与预期 100×80 不符"]
        );
        let shifted = spec(Framing::Full {
            doc_height: 300.0,
            top: 120.0,
        });
        assert_eq!(
            check(&tall, &measure(), &shifted),
            ["整页图没有从文档顶部开始（起点 y=120）"]
        );
        let element = ShotSpec {
            framing: Framing::Element {
                width: 40.0,
                height: 30.0,
            },
            dpr: 2.0,
            ..spec(Framing::Viewport)
        };
        assert!(check(&textured(80, 60), &measure(), &element).is_empty());
        assert_eq!(
            check(&textured(40, 30), &measure(), &element),
            ["元素图尺寸 40×30 与预期 80×60 不符"]
        );
    }

    #[test]
    fn unsettled_and_transparent_content() {
        let m = ShotMeasure {
            finite_animations: 2,
            text_chars: 40,
            transparent_chars: 20,
            root_luminance: 1.0,
            ..Default::default()
        };
        assert_eq!(
            check(&textured(100, 80), &m, &spec(Framing::Viewport)),
            [
                "动画未稳定（2 个仍在运行）；加大 --settle-ms 或用 --reduced-motion",
                "主要内容透明，入场动效未稳定或停在隐藏态",
            ]
        );
        let few = ShotMeasure {
            text_chars: 19,
            transparent_chars: 19,
            root_luminance: 1.0,
            ..Default::default()
        };
        assert!(check(&textured(100, 80), &few, &spec(Framing::Viewport)).is_empty());
    }
}
