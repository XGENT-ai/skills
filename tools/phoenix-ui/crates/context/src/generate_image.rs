//! JS: generate-image.mjs -> `impeccable generate-image`

use crate::jsp;
use crate::util::{iso_now, json_pretty, node_read_error, Env};
use impeccable_common::Io;
use serde_json::{Map, Value};

pub const DEFAULT_MODEL: &str = "gpt-image-2.5-flare";

fn arg(args: &[String], name: &str) -> Option<String> {
    let i = args.iter().position(|a| a == &format!("--{}", name))?;
    let v = args.get(i + 1)?;
    if !v.is_empty() && !v.starts_with("--") {
        Some(v.clone())
    } else {
        None
    }
}

pub fn run(args: &[String], io: &mut Io) -> i32 {
    run_with_api_base(args, io, "https://api.openai.com/v1")
}

fn run_with_api_base(args: &[String], io: &mut Io, api_base: &str) -> i32 {
    let background = arg(args, "background");
    if args.iter().any(|a| a == "--background") && !matches!(background.as_deref(), Some("transparent" | "opaque" | "auto")) {
        io.err("generate-image: --background must be transparent, opaque, or auto.\n");
        return 1;
    }
    if background.is_some() && arg(args, "out").is_some_and(|out| !out.to_ascii_lowercase().ends_with(".png")) {
        io.err("generate-image: --background requires a .png --out path.\n");
        return 1;
    }
    let cwd = io.cwd.to_string_lossy().into_owned();
    let env: Env = io.env.clone();
    let abs = |p: &str| jsp::resolve(&cwd, &[p]);
    let read_prompt_file = |io: &mut Io, pf: &str| -> Result<String, i32> {
        match std::fs::read(abs(pf)) {
            Ok(b) => Ok(String::from_utf8_lossy(&b).into_owned()),
            Err(e) => {
                io.err(&format!("Error: {}\n", node_read_error(pf, &e)));
                Err(1)
            }
        }
    };
    let Some(key) = env.get("OPENAI_API_KEY").filter(|k| !k.is_empty()).cloned() else {
        io.err("generate-image: OPENAI_API_KEY is not set; use the harness-native image tool instead.\n");
        return 1;
    };
    let prompt = match arg(args, "prompt-file") {
        Some(pf) => match read_prompt_file(io, &pf) {
            Ok(p) => Some(p),
            Err(c) => return c,
        },
        None => arg(args, "prompt"),
    };
    let out = arg(args, "out");
    let (Some(prompt), Some(out)) = (prompt.filter(|p| !p.is_empty()), out) else {
        io.err("generate-image: --prompt (or --prompt-file) and --out are required.\n");
        return 1;
    };
    let size = arg(args, "size").unwrap_or_else(|| "1536x1024".into());
    let quality = arg(args, "quality").unwrap_or_else(|| "medium".into());
    let model = arg(args, "model").unwrap_or_else(|| DEFAULT_MODEL.into());
    let mut refs: Vec<String> = Vec::new();
    for i in 0..args.len() {
        if args[i] == "--ref" {
            if let Some(n) = args.get(i + 1) {
                if !n.is_empty() && !n.starts_with("--") {
                    refs.push(n.clone());
                }
            }
        }
    }
    let agent = crate::http::agent_builder().build();
    let response = if !refs.is_empty() {
        let boundary = format!("----impeccable{:x}", crate::util::now_ms() as u64);
        let mut body: Vec<u8> = Vec::new();
        let mut field = |name: &str, value: &str| {
            body.extend_from_slice(format!("--{}\r\nContent-Disposition: form-data; name=\"{}\"\r\n\r\n{}\r\n", boundary, name, value).as_bytes());
        };
        field("model", &model);
        field("prompt", &prompt);
        field("size", &size);
        field("quality", &quality);
        field("n", "1");
        if let Some(background) = &background {
            field("background", background);
            field("output_format", "png");
        }
        for r in &refs {
            let bytes = match std::fs::read(abs(r)) {
                Ok(b) => b,
                Err(e) => {
                    io.err(&format!("Error: {}\n", node_read_error(r, &e)));
                    return 1;
                }
            };
            let ty = if r.ends_with(".png") {
                "image/png"
            } else if r.ends_with(".webp") {
                "image/webp"
            } else {
                "image/jpeg"
            };
            let filename = r.rsplit('/').next().unwrap_or(r);
            body.extend_from_slice(
                format!("--{}\r\nContent-Disposition: form-data; name=\"image[]\"; filename=\"{}\"\r\nContent-Type: {}\r\n\r\n", boundary, filename, ty).as_bytes(),
            );
            body.extend_from_slice(&bytes);
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());
        agent
            .post(&format!("{api_base}/images/edits"))
            .set("Authorization", &format!("Bearer {}", key))
            .set("Content-Type", &format!("multipart/form-data; boundary={}", boundary))
            .send_bytes(&body)
    } else {
        let mut m = Map::new();
        m.insert("model".into(), Value::String(model.clone()));
        m.insert("prompt".into(), Value::String(prompt.clone()));
        m.insert("size".into(), Value::String(size.clone()));
        m.insert("quality".into(), Value::String(quality.clone()));
        m.insert("n".into(), Value::from(1));
        if let Some(background) = &background {
            m.insert("background".into(), Value::String(background.clone()));
            m.insert("output_format".into(), Value::String("png".into()));
        }
        agent
            .post(&format!("{api_base}/images/generations"))
            .set("Authorization", &format!("Bearer {}", key))
            .set("content-type", "application/json")
            .send_string(&serde_json::to_string(&Value::Object(m)).unwrap())
    };
    let (status, text) = match response {
        Ok(r) => {
            let st = r.status();
            (st, r.into_string().unwrap_or_default())
        }
        Err(ureq::Error::Status(code, r)) => (code, r.into_string().unwrap_or_default()),
        Err(e) => {
            io.err(&format!("TypeError: fetch failed: {}\n", e));
            return 1;
        }
    };
    if !(200..300).contains(&status) {
        let snippet: String = text.chars().take(300).collect();
        io.err(&format!("generate-image: API error {}: {}\n", status, snippet));
        return 1;
    }
    let json: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    let b64 = json.get("data").and_then(|d| d.get(0)).and_then(|d| d.get("b64_json")).and_then(|b| b.as_str()).filter(|s| !s.is_empty());
    let Some(b64) = b64 else {
        io.err("generate-image: no image in response\n");
        return 1;
    };
    let bytes = base64_decode(b64);
    let _ = std::fs::write(abs(&out), bytes);
    // best-effort embed + sidecar
    // JS-PARITY: generate-image.mjs#676 reports whether the embed actually
    // succeeded. The install-path-with-spaces half of #676 is a JS-only
    // subprocess concern (fileURLToPath vs URL.pathname); the engine embeds
    // in-process, so only the success tracking and message carry over here.
    let embedded;
    {
        let mut sub_io = Io::captured("", io.cwd.clone(), io.env.clone()).0;
        let ret = crate::embed_prompt::run(&[out.clone(), "--prompt".to_string(), prompt.clone()], &mut sub_io);
        embedded = ret == 0;
        if !embedded {
            io.err("generate-image: failed to embed prompt in the image\n");
        }
        let mut m = Map::new();
        m.insert("prompt".into(), Value::String(prompt.clone()));
        m.insert("createdAt".into(), Value::String(iso_now()));
        m.insert(
            "tool".into(),
            Value::String("phoenix-ui generate-image".into()),
        );
        m.insert("model".into(), Value::String(model.clone()));
        if let Some(background) = &background {
            m.insert("background".into(), Value::String(background.clone()));
            m.insert("outputFormat".into(), Value::String("png".into()));
        }
        if !refs.is_empty() {
            m.insert("refs".into(), Value::Array(refs.iter().cloned().map(Value::String).collect()));
        }
        let _ = std::fs::write(abs(&format!("{}.json", out)), json_pretty(&Value::Object(m)));
    }
    io.out(&format!(
        "IMAGE: {} ({}, {}, {}, billed to your OpenAI key); {} at {}.json\n",
        out,
        size,
        quality,
        model,
        if embedded { "prompt embedded + sidecar" } else { "sidecar" },
        out
    ));
    0
}

fn base64_decode(s: &str) -> Vec<u8> {
    let table = |c: u8| -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a' + 26) as u32),
            b'0'..=b'9' => Some((c - b'0' + 52) as u32),
            b'+' | b'-' => Some(62),
            b'/' | b'_' => Some(63),
            _ => None,
        }
    };
    let mut out = Vec::new();
    let mut acc: u32 = 0;
    let mut bits = 0;
    for b in s.bytes() {
        let Some(v) = table(b) else { continue };
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xff) as u8);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn round_trip(edit: bool, override_model: Option<&str>, background: Option<&str>) {
        let _proxy_lock = crate::http::PROXY_ENV_LOCK.lock().unwrap();
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let api_base = format!("http://{}", server.server_addr());
        let temp = std::env::temp_dir().join(format!("impeccable-image-{}-{}", std::process::id(), server.server_addr().to_ip().unwrap().port()));
        std::fs::create_dir_all(&temp).unwrap();
        std::fs::write(temp.join("ref.png"), b"reference").unwrap();
        let handle = std::thread::spawn(move || {
            let mut request = server.recv_timeout(Duration::from_secs(10)).unwrap().expect("image request");
            let path = request.url().to_string();
            let content_type = request.headers().iter().find(|h| h.field.equiv("Content-Type")).unwrap().value.to_string();
            let mut body = String::new();
            // Multipart carries binary PNG bytes; preserve ASCII fields for inspection.
            let mut bytes = Vec::new();
            request.as_reader().read_to_end(&mut bytes).unwrap();
            body.push_str(&String::from_utf8_lossy(&bytes));
            request.respond(tiny_http::Response::from_string(r#"{"data":[{"b64_json":"iVBORw0KGgoAAAANSUhEUgAAAAQAAAABCAYAAAD5PA/NAAAAGklEQVR4nGP4////f7mAigYGBgaG/////wMAUdQJXhk2RAEAAAAASUVORK5CYII="}]}"#)).unwrap();
            (path, content_type, body)
        });
        let env = Env::from([("OPENAI_API_KEY".into(), "test-key".into())]);
        let (mut io, captured) = Io::captured("", temp.clone(), env);
        let mut args: Vec<String> = ["--prompt", "Comp regression", "--out", "comp.png", "--quality", "high"].iter().map(|s| s.to_string()).collect();
        if edit {
            args.extend(["--ref".into(), "ref.png".into()]);
        }
        if let Some(model) = override_model {
            args.extend(["--model".into(), model.into()]);
        }
        if let Some(background) = background {
            args.extend(["--background".into(), background.into()]);
        }
        let exit = run_with_api_base(&args, &mut io, &api_base);
        let (path, content_type, body) = handle.join().unwrap();
        let sidecar: Value = serde_json::from_slice(&std::fs::read(temp.join("comp.png.json")).unwrap()).unwrap();
        let image = std::fs::read(temp.join("comp.png")).unwrap();
        std::fs::remove_dir_all(&temp).unwrap();
        assert_eq!(exit, 0);
        let model = override_model.unwrap_or("gpt-image-2.5-flare");
        if edit {
            assert_eq!(path, "/images/edits");
            assert!(content_type.starts_with("multipart/form-data; boundary="));
            assert!(body.contains(&format!("name=\"model\"\r\n\r\n{model}\r\n")));
            assert!(body.contains("name=\"image[]\"; filename=\"ref.png\""));
            assert_eq!(sidecar["refs"], serde_json::json!(["ref.png"]));
            if let Some(background) = background {
                assert!(body.contains(&format!("name=\"background\"\r\n\r\n{background}\r\n")));
                assert!(body.contains("name=\"output_format\"\r\n\r\npng\r\n"));
            } else {
                assert!(!body.contains("name=\"background\""));
            }
        } else {
            assert_eq!(path, "/images/generations");
            assert_eq!(content_type, "application/json");
            let body: Value = serde_json::from_str(&body).unwrap();
            let mut expected = serde_json::json!({"model": model, "prompt": "Comp regression", "size": "1536x1024", "quality": "high", "n": 1});
            if let Some(background) = background {
                expected["background"] = background.into();
                expected["output_format"] = "png".into();
            }
            assert_eq!(body, expected);
        }
        if let Some(background) = background {
            assert_eq!(sidecar["background"], background);
            assert_eq!(sidecar["outputFormat"], "png");
        } else {
            assert!(sidecar.get("background").is_none());
        }
        // The server's PNG contains clear, partial, near-opaque and opaque pixels.
        // Embedding may add metadata before IEND, but must preserve all image chunks.
        let original = base64_decode("iVBORw0KGgoAAAANSUhEUgAAAAQAAAABCAYAAAD5PA/NAAAAGklEQVR4nGP4////f7mAigYGBgaG/////wMAUdQJXhk2RAEAAAAASUVORK5CYII=");
        assert!(image.starts_with(&original[..original.len() - 12]));
        assert_eq!(sidecar["model"], model);
        assert_eq!(sidecar["prompt"], "Comp regression");
        assert!(image.starts_with(b"\x89PNG\r\n\x1a\n"));
        let stdout = String::from_utf8(captured.stdout.borrow().clone()).unwrap();
        assert!(stdout.contains(&format!("{model}, billed to your OpenAI key")));
        assert!(stdout.contains("prompt embedded + sidecar"));
    }

    #[test]
    fn generation_uses_image_25_and_records_model() {
        round_trip(false, None, None);
    }

    #[test]
    fn reference_edit_uses_image_25_and_records_model() {
        round_trip(true, None, None);
    }

    #[test]
    fn generation_accepts_model_override() {
        round_trip(false, Some("gpt-image-2"), None);
    }

    #[test]
    fn reference_edit_accepts_sunburst_override() {
        round_trip(true, Some("gpt-image-2.5-sunburst"), None);
    }

    #[test]
    fn transparent_generation_preserves_alpha_and_provenance() {
        round_trip(false, None, Some("transparent"));
    }

    #[test]
    fn transparent_edit_preserves_alpha_and_provenance() {
        round_trip(true, Some("gpt-image-2.5-sunburst"), Some("transparent"));
    }

    #[test]
    fn opaque_background_is_explicit() {
        round_trip(false, None, Some("opaque"));
    }

    #[test]
    fn invalid_background_requests_fail_before_network_or_output() {
        for flags in [
            vec!["--background"],
            vec!["--background", "white"],
            vec!["--background", "transparent", "--out", "cutout.jpg"],
            vec!["--background", "opaque", "--out", "hero.webp"],
            vec!["--background", "auto", "--out", "hero.svg"],
        ] {
            let (mut io, captured) = Io::captured("", std::env::temp_dir(), Env::new());
            let args = flags.iter().map(|s| s.to_string()).collect::<Vec<_>>();
            assert_eq!(run(&args, &mut io), 1);
            let stderr = String::from_utf8(captured.stderr.borrow().clone()).unwrap();
            assert!(stderr.contains("--background"), "{stderr}");
            assert!(!stderr.contains("OPENAI_API_KEY"));
        }
    }
}
