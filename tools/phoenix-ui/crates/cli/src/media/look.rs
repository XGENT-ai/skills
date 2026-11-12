use impeccable_comp::raster::{resize, Image};
use serde_json::{json, Value};

struct Region {
    mean: f64,
    deviation: f64,
    saturation: f64,
    gradient: f64,
    rgb: [f64; 3],
}

fn luminance(pixel: &[u8]) -> f64 {
    (0.2126 * pixel[0] as f64 + 0.7152 * pixel[1] as f64 + 0.0722 * pixel[2] as f64) / 255.0
}

fn region(image: &Image, x0: usize, y0: usize, x1: usize, y1: usize) -> Region {
    let (mut sum, mut square, mut saturation, mut gradient) = (0.0, 0.0, 0.0, 0.0);
    let mut rgb = [0.0; 3];
    let mut pairs = 0;
    for y in y0..y1 {
        for x in x0..x1 {
            let offset = (y * image.width + x) * 4;
            let pixel = &image.data[offset..offset + 3];
            let value = luminance(pixel);
            let max = pixel[0].max(pixel[1]).max(pixel[2]) as f64;
            let min = pixel[0].min(pixel[1]).min(pixel[2]) as f64;
            sum += value;
            square += value * value;
            if max > 0.0 {
                saturation += (max - min) / max;
            }
            for (total, channel) in rgb.iter_mut().zip(pixel) {
                *total += *channel as f64;
            }
            if x + 1 < x1 {
                gradient += (value - luminance(&image.data[offset + 4..offset + 7])).abs();
                pairs += 1;
            }
            if y + 1 < y1 {
                let below = offset + image.width * 4;
                gradient += (value - luminance(&image.data[below..below + 3])).abs();
                pairs += 1;
            }
        }
    }
    let count = ((x1 - x0) * (y1 - y0)) as f64;
    let mean = sum / count;
    Region {
        mean,
        deviation: (square / count - mean * mean).max(0.0).sqrt(),
        saturation: saturation / count,
        gradient: gradient / pairs as f64,
        rgb: rgb.map(|channel| channel / count),
    }
}

fn hex(rgb: [f64; 3]) -> String {
    format!(
        "#{:02x}{:02x}{:02x}",
        rgb[0].round() as u8,
        rgb[1].round() as u8,
        rgb[2].round() as u8
    )
}

pub(super) fn describe(image: &Image) -> Value {
    let thumbnail = resize(image, 48.0, 48.0);
    let all = region(&thumbnail, 0, 0, 48, 48);
    let strips = [
        ("top", region(&thumbnail, 0, 0, 48, 3)),
        ("right", region(&thumbnail, 45, 0, 48, 48)),
        ("bottom", region(&thumbnail, 0, 45, 48, 48)),
        ("left", region(&thumbnail, 0, 0, 3, 48)),
    ];
    let edge_flat = strips
        .iter()
        .all(|(_, strip)| strip.deviation < 0.06 && strip.gradient < 0.02);
    let edge_mean = strips.iter().map(|(_, strip)| strip.mean).sum::<f64>() / 4.0;
    let edge_saturation = strips
        .iter()
        .map(|(_, strip)| strip.saturation)
        .sum::<f64>()
        / 4.0;
    let edge_rgb =
        [0, 1, 2].map(|i| strips.iter().map(|(_, strip)| strip.rgb[i]).sum::<f64>() / 4.0);
    let regions = [
        ("top", region(&thumbnail, 0, 0, 48, 16)),
        ("bottom", region(&thumbnail, 0, 32, 48, 48)),
        ("left", region(&thumbnail, 0, 0, 16, 48)),
        ("right", region(&thumbnail, 32, 0, 48, 48)),
        ("center", region(&thumbnail, 16, 16, 32, 32)),
    ];
    let calm: Vec<&str> = regions
        .iter()
        .filter(|(_, region)| region.deviation < 0.09 && region.gradient < 0.02)
        .map(|(name, _)| *name)
        .collect();
    let edges: serde_json::Map<String, Value> = strips
        .iter()
        .map(|(name, strip)| ((*name).into(), json!(hex(strip.rgb))))
        .collect();
    json!({
        "tone": if all.mean < 0.33 { "dark" } else if all.mean > 0.66 { "light" } else { "mid" },
        "mono": all.saturation < 0.1,
        "edges": edges,
        "edge": hex(edge_rgb),
        "edgeFlat": edge_flat,
        "paper": edge_flat && edge_mean > 0.75 && edge_saturation < 0.2,
        "calm": calm,
        "ratio": (image.width as f64 / image.height as f64 * 100.0).round() / 100.0
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use impeccable_comp::raster::create_image;
    use serde_json::json;

    #[test]
    fn light_gray_paper_has_calm_regions_and_matching_edges() {
        let image = create_image(96, 48, [230, 230, 230, 255]);
        let look = describe(&image);
        assert_eq!(look["paper"], true);
        assert_eq!(look["tone"], "light");
        assert_eq!(look["mono"], true);
        assert_eq!(look["edgeFlat"], true);
        assert_eq!(look["edge"], "#e6e6e6");
        assert_eq!(
            look["edges"],
            json!({"top":"#e6e6e6", "right":"#e6e6e6", "bottom":"#e6e6e6", "left":"#e6e6e6"})
        );
        assert_eq!(
            look["calm"],
            json!(["top", "bottom", "left", "right", "center"])
        );
        assert_eq!(look["ratio"], 2.0);
    }

    #[test]
    fn noisy_bottom_leaves_only_the_top_calm() {
        let mut image = create_image(48, 48, [230, 230, 230, 255]);
        for y in 24..48 {
            for x in 0..48 {
                let shade = if (x + y) % 2 == 0 { 0 } else { 255 };
                let start = (y * 48 + x) * 4;
                image.data[start..start + 3].fill(shade);
            }
        }
        let look = describe(&image);
        assert_eq!(look["paper"], false);
        assert_eq!(look["edgeFlat"], false);
        assert_eq!(look["calm"], json!(["top"]));
        assert_ne!(look["edges"]["top"], look["edges"]["bottom"]);
    }
}
