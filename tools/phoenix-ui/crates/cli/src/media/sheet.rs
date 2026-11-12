use std::io::{Cursor, Read};
use std::path::Path;

use impeccable_common::jsp;
use impeccable_comp::{png_io, raster};
use serde_json::{json, Value};

use super::{index, look, sources};

const PREVIEW_LIMIT: u64 = 8 * 1024 * 1024;

fn download(agent: &ureq::Agent, url: &str) -> Result<raster::Image, String> {
    let url = sources::normalize_url(url)?;
    let response = agent.get(&url).call().map_err(|e| e.to_string())?;
    if response
        .header("Content-Length")
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|length| length > PREVIEW_LIMIT)
    {
        return Err("预览超过 8 MB".into());
    }
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(PREVIEW_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("无法读取预览：{e}"))?;
    if bytes.len() as u64 > PREVIEW_LIMIT {
        return Err("预览超过 8 MB".into());
    }
    let image = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| format!("无法识别预览格式：{e}"))?
        .decode()
        .map_err(|e| format!("预览无法解码：{e}"))?
        .to_rgba8();
    Ok(raster::Image {
        width: image.width() as usize,
        height: image.height() as usize,
        data: image.into_raw(),
    })
}

pub(super) fn render(
    agent: &ureq::Agent,
    area: &Path,
    cwd: &Path,
    items: &mut [Value],
) -> Result<(Value, Vec<Value>), String> {
    if items.is_empty() {
        return Ok((Value::Null, Vec::new()));
    }
    let mut previews: Vec<Result<raster::Image, String>> =
        items.iter().map(|_| Err(String::new())).collect();
    std::thread::scope(|scope| {
        for (item, result) in items.iter().zip(previews.iter_mut()) {
            scope.spawn(move || {
                *result = item["previewUrl"]
                    .as_str()
                    .ok_or_else(|| "缺少预览地址".to_string())
                    .and_then(|url| download(agent, url));
            });
        }
    });
    let columns = items.len().min(4);
    let rows = items.len().div_ceil(columns);
    let mut canvas = raster::create_image(columns * 360, rows * 300, [237, 237, 237, 255]);
    let mut errors = Vec::new();
    let mut cells = Vec::new();
    for (i, (item, preview)) in items.iter_mut().zip(previews).enumerate() {
        let x = (i % columns * 360) as f64;
        let y = (i / columns * 300) as f64;
        match preview {
            Ok(image) => {
                item["look"] = look::describe(&image);
                let thumbnail = raster::fit(&image, 344.0, 238.0, true);
                raster::blit(
                    &mut canvas,
                    &thumbnail,
                    x + 8.0 + (344.0 - thumbnail.width as f64) / 2.0,
                    y + 8.0 + (238.0 - thumbnail.height as f64) / 2.0,
                );
            }
            Err(message) => {
                item["look"] = Value::Null;
                errors.push(json!({
                    "source": item["provider"],
                    "message": format!("{}: {message}", item["ref"].as_str().unwrap_or(""))
                }));
                raster::draw_text(
                    &mut canvas,
                    "NO PREVIEW",
                    x + 120.0,
                    y + 118.0,
                    [100.0, 100.0, 100.0, 255.0],
                    2.0,
                );
            }
        }
        let (grade, color) = match item["use"].as_str() {
            Some("page") => ("PAGE", [30.0, 123.0, 70.0, 255.0]),
            Some("reference") => ("REF", [168.0, 50.0, 45.0, 255.0]),
            _ => ("CHECK", [178.0, 106.0, 0.0, 255.0]),
        };
        let reference = item["ref"].as_str().unwrap_or("");
        let width = item["originalWidth"]
            .as_u64()
            .ok_or_else(|| format!("{reference} 缺少原图宽度"))?;
        let height = item["originalHeight"]
            .as_u64()
            .ok_or_else(|| format!("{reference} 缺少原图高度"))?;
        let label = format!("{reference}  {grade}  {width}X{height}");
        raster::fill_rect(&mut canvas, x + 8.0, y + 252.0, 344.0, 26.0, color);
        let scale = 2.0_f64.min(336.0 / (label.len() as f64 * 6.0));
        raster::draw_label(
            &mut canvas,
            &label,
            x + 10.0,
            y + 255.0,
            [255.0; 4],
            color,
            scale,
            2.0,
        );
        raster::draw_text(
            &mut canvas,
            item["provider"].as_str().unwrap_or(""),
            x + 10.0,
            y + 284.0,
            [70.0, 70.0, 70.0, 255.0],
            1.5,
        );
        cells.push(item["ref"].clone());
    }
    index::atomic_write(area, "candidates.png", &png_io::encode_png(&canvas, &[])?)?;
    let cwd = cwd.to_string_lossy();
    let file = jsp::relative(&cwd, &cwd, &area.join("candidates.png").to_string_lossy());
    Ok((
        json!({"file": file, "columns": columns, "cells": cells}),
        errors,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Duration;

    #[test]
    fn sheet_keeps_failed_previews_as_labelled_cells() {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", server.server_addr());
        let png =
            png_io::encode_png(&raster::create_image(16, 8, [230, 230, 230, 255]), &[]).unwrap();
        let worker = std::thread::spawn(move || {
            for request in server.incoming_requests().take(2) {
                if request.url() == "/image.png" {
                    request
                        .respond(tiny_http::Response::from_data(png.clone()))
                        .unwrap();
                } else {
                    request.respond(tiny_http::Response::empty(404)).unwrap();
                }
            }
        });
        let cwd = std::env::temp_dir().join(format!(
            "phoenix-media-sheet-{}-{}",
            std::process::id(),
            impeccable_common::now_ms()
        ));
        let area = cwd.join(".phoenix-ui/media");
        fs::create_dir_all(&area).unwrap();
        let mut items = vec![
            json!({"ref":"C1", "provider":"commons", "use":"page", "previewUrl":format!("{origin}/image.png"), "originalWidth":2048, "originalHeight":1024}),
            json!({"ref":"C2", "provider":"commons", "use":"reference", "previewUrl":format!("{origin}/missing.png"), "originalWidth":1000, "originalHeight":800}),
        ];
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(3))
            .build();
        let (sheet, errors) = render(&agent, &area, &cwd, &mut items).unwrap();
        worker.join().unwrap();
        assert_eq!(
            sheet,
            json!({"file":".phoenix-ui/media/candidates.png", "columns":2, "cells":["C1", "C2"]})
        );
        assert_eq!(items[0]["look"]["paper"], true);
        assert_eq!(items[1]["look"], Value::Null);
        assert_eq!(errors.len(), 1);
        assert!(errors[0]["message"].as_str().unwrap().contains("C2"));
        let image = image::open(area.join("candidates.png")).unwrap().to_rgba8();
        assert_eq!(image.dimensions(), (720, 300));
        assert_eq!(image.get_pixel(350, 260).0, [30, 123, 70, 255]);
        assert_eq!(image.get_pixel(710, 260).0, [168, 50, 45, 255]);
        assert_eq!(image.get_pixel(180, 120).0, [230, 230, 230, 255]);
        fs::remove_dir_all(cwd).unwrap();
    }
}
