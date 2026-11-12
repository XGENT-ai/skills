//! The comparison page's loopback server: the shell page, each HTML
//! proposal's own directory behind an isolating policy, the reachability
//! probe for loopback proposals, and the keyed answer.

use super::payload::{Lang, Proposal, Round, Source};
use super::{answer_file, state_file};
use crate::serve_question::{
    allowed_host, open_system_browser, parse_request_url, query_param, reject_detached_post,
    sleep_ms, write_atomic,
};
use crate::util::{json_compact, now_ms, safe_read};
use impeccable_common::Io;
use serde_json::{json, Value};
use std::io::Read;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const PAGE_HTML: &str = include_str!("page.html");
const PAGE_JS: &str = include_str!("page.js");
const PAGE_CSS: &str = include_str!("page.css");
const KIT_TOKENS: &str = include_str!("../../../../ui/component-review/vendor/kinpaku-tokens.css");
const KIT: &str = include_str!("../../../../ui/component-review/vendor/kinpaku-kit.css");

const ANSWER_MAX_BYTES: u64 = 64 * 1024;
const PROBE_TIMEOUT: Duration = Duration::from_millis(800);

const PROPOSAL_CSP: &str = "default-src 'none'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; media-src 'self'; script-src 'none'; connect-src 'none'; base-uri 'none'; form-action 'none'; sandbox; frame-ancestors 'self'";
const INTERACTIVE_CSP: &str = "default-src 'none'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; media-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; connect-src 'none'; base-uri 'none'; form-action 'none'; sandbox allow-scripts; frame-ancestors 'self'";

pub struct Options {
    pub qdir: String,
    pub key: String,
    pub timeout_sec: f64,
    pub idle_grace_ms: f64,
    pub open: bool,
}

/// What the shell may frame: itself plus each loopback proposal's origin.
fn shell_csp(round: &Round) -> String {
    let mut frames: Vec<String> = Vec::new();
    for p in &round.proposals {
        if let Source::Url {
            scheme, host, port, ..
        } = &p.source
        {
            // CSP host sources cannot name an IPv6 literal, so a [::1]
            // proposal is admitted by its scheme.
            let source = if host.starts_with('[') {
                format!("{}:", scheme)
            } else {
                format!("{}://{}:{}", scheme, host, port)
            };
            if !frames.contains(&source) {
                frames.push(source);
            }
        }
    }
    let frames: String = frames.iter().map(|f| format!(" {}", f)).collect();
    format!("default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-src 'self'{}; base-uri 'none'; form-action 'none'; frame-ancestors 'none'", frames)
}

fn respond(req: tiny_http::Request, code: u16, body: Vec<u8>, mime: &str, csp: Option<&str>) {
    let mut response = tiny_http::Response::from_data(body).with_status_code(code);
    let mut headers = vec![
        ("Content-Type", mime),
        ("Cache-Control", "no-store"),
        ("X-Content-Type-Options", "nosniff"),
        ("Referrer-Policy", "no-referrer"),
    ];
    if let Some(csp) = csp {
        headers.push(("Content-Security-Policy", csp));
    }
    // A sandboxed proposal has no same-origin identity, so its fonts load cross-origin.
    if mime.starts_with("font/") {
        headers.push(("Access-Control-Allow-Origin", "*"));
    }
    for (k, v) in headers {
        if let Ok(h) = tiny_http::Header::from_bytes(k, v) {
            response = response.with_header(h);
        }
    }
    let _ = req.respond(response);
}

fn content_type(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        _ => "application/octet-stream",
    }
}

fn percent_decode(raw: &str) -> Option<String> {
    let input = raw.as_bytes();
    let mut bytes = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        if input[i] == b'%' {
            let hex = std::str::from_utf8(input.get(i + 1..i + 3)?).ok()?;
            bytes.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            bytes.push(input[i]);
            i += 1;
        }
    }
    String::from_utf8(bytes).ok()
}

/// The file a proposal's page may load from its own directory, or None when
/// it must not be served: traversal, dot segments, backslashes, NUL,
/// undecodable bytes, a missing file, or a symlink that leaves the directory.
fn read_within(root: &Path, encoded: &str) -> Option<(PathBuf, Vec<u8>)> {
    let decoded = percent_decode(encoded)?;
    if decoded.is_empty() || decoded.contains(['\0', '\\']) {
        return None;
    }
    if decoded
        .split('/')
        .any(|segment| segment.is_empty() || segment.starts_with('.'))
    {
        return None;
    }
    let real_root = std::fs::canonicalize(root).ok()?;
    let real = std::fs::canonicalize(root.join(&decoded)).ok()?;
    if !real.starts_with(&real_root) || !real.is_file() {
        return None;
    }
    let bytes = std::fs::read(&real).ok()?;
    Some((real, bytes))
}

fn encode_segment(name: &str) -> String {
    name.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

fn page_data(round: &Round, key: &str) -> Value {
    let proposals: Vec<Value> = round
        .proposals
        .iter()
        .enumerate()
        .map(|(index, p)| {
            let (kind, src, interactive, pixel_ratio) = match &p.source {
                Source::Html { file, interactive } => {
                    let name = file.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    ("html", format!("/p/{}/{}", index, encode_segment(name)), *interactive, 1.0)
                }
                Source::Image { pixel_ratio, .. } => ("image", format!("/img/{}", index), false, *pixel_ratio),
                Source::Url { url, .. } => ("url", url.clone(), false, 1.0),
            };
            json!({
                "index": index, "id": p.id, "name": p.name, "kind": kind, "src": src,
                "baseline": p.baseline, "interactive": interactive, "pixelRatio": pixel_ratio,
                "concept": p.concept, "typography": p.typography, "palette": p.palette, "traits": p.traits,
            })
        })
        .collect();
    json!({
        "key": key,
        "title": round.title,
        "question": round.question,
        "lang": if round.lang == Lang::En { "en" } else { "zh-CN" },
        "serve": round.serve.as_ref().map(|s| json!({ "command": s.command, "cwd": s.cwd })),
        "proposals": proposals,
    })
}

fn shell_page(round: &Round, key: &str) -> String {
    // JSON in a script element is data; escape HTML delimiters so it cannot close the element.
    let safe = json_compact(&page_data(round, key))
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e");
    PAGE_HTML.replace("__COMPARE_JSON__", &safe)
}

/// One TCP connect to the proposal's loopback host and port; no HTTP is spoken.
fn reachable(host: &str, port: u16) -> bool {
    let name = host.trim_start_matches('[').trim_end_matches(']');
    let Ok(addrs) = (name, port).to_socket_addrs() else {
        return false;
    };
    addrs
        .into_iter()
        .any(|addr| TcpStream::connect_timeout(&addr, PROBE_TIMEOUT).is_ok())
}

fn status(round: &Round) -> Value {
    let proposals: Vec<Value> = round
        .proposals
        .iter()
        .filter_map(|p| {
            let Source::Url { host, port, .. } = &p.source else {
                return None;
            };
            let available = reachable(host, *port);
            let reason = match (available, round.lang) {
                (true, _) => Value::Null,
                (false, Lang::Zh) => Value::String(format!("{}:{} 没有响应", host, port)),
                (false, Lang::En) => Value::String(format!("{}:{} is not responding", host, port)),
            };
            Some(json!({ "id": p.id, "available": available, "reason": reason }))
        })
        .collect();
    json!({ "proposals": proposals })
}

fn header_value(req: &tiny_http::Request, name: &'static str) -> Option<String> {
    req.headers()
        .iter()
        .find(|h| h.field.equiv(name))
        .map(|h| h.value.as_str().to_string())
}

fn write_state(file: &str, mutate: impl FnOnce(&mut serde_json::Map<String, Value>)) {
    let mut state = safe_read(file)
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    mutate(&mut state);
    let _ = write_atomic(file, &json_compact(&Value::Object(state)));
}

/// `/p/<index>/<path>` or `/img/<index>`: the proposal and the rest of the path.
fn proposal_route<'a>(round: &'a Round, rest: &'a str) -> Option<(&'a Proposal, &'a str)> {
    let (index, tail) = rest.split_once('/').unwrap_or((rest, ""));
    if index.is_empty() || !index.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let proposal = round.proposals.get(index.parse::<usize>().ok()?)?;
    Some((proposal, tail))
}

pub fn serve(round: Round, opts: Options, io: &mut Io) -> i32 {
    let server = match tiny_http::Server::http(("127.0.0.1", 0)) {
        Ok(s) => s,
        Err(e) => {
            io.err(&format!("compare：无法监听本机端口：{}\n", e));
            return 1;
        }
    };
    let Some(port) = server.server_addr().to_ip().map(|a| a.port()) else {
        io.err("compare：无法监听本机端口\n");
        return 1;
    };
    let url = format!("http://127.0.0.1:{}/", port);
    let state_path = state_file(&opts.qdir, &opts.key);
    let _ = std::fs::create_dir_all(&opts.qdir);
    if let Err(e) = write_atomic(
        &state_path,
        &json_compact(&json!({ "pid": std::process::id(), "port": port, "url": url })),
    ) {
        io.err(&format!(
            "compare：无法写入状态文件 {}：{}\n",
            state_path, e
        ));
        return 1;
    }
    if opts.open {
        open_system_browser(&url, &io.env);
    }

    let last_beat: Arc<Mutex<Option<f64>>> = Arc::new(Mutex::new(None));
    {
        let last_beat = last_beat.clone();
        let started_at = now_ms();
        let (timeout_sec, idle_grace_ms) = (opts.timeout_sec, opts.idle_grace_ms);
        std::thread::spawn(move || loop {
            sleep_ms(2000);
            let seen = last_beat.lock().map(|b| *b).unwrap_or(None);
            match seen {
                None if timeout_sec > 0.0 && now_ms() - started_at > timeout_sec * 1000.0 => {
                    println!("compare: timed out with no answer");
                    std::process::exit(2);
                }
                Some(beat) if now_ms() - beat > idle_grace_ms => {
                    println!("compare: the page stopped beating and never came back; exiting");
                    std::process::exit(2);
                }
                _ => {}
            }
        });
    }

    let shell = shell_csp(&round);
    let shell = Some(shell.as_str());
    let mut last_beat_write: Option<f64> = None;
    for mut request in server.incoming_requests() {
        let method = request.method().as_str().to_string();
        let raw_url = request.url().to_string();
        if !allowed_host(header_value(&request, "Host").as_deref(), port) {
            respond(request, 403, Vec::new(), "text/plain", None);
            continue;
        }
        let Some((path, query)) = parse_request_url(&raw_url) else {
            respond(request, 400, Vec::new(), "text/plain", None);
            continue;
        };
        let keyed = query_param(&query, "key").as_deref() == Some(opts.key.as_str());
        match (method.as_str(), path.as_str()) {
            ("GET", "/") => respond(
                request,
                200,
                shell_page(&round, &opts.key).into_bytes(),
                "text/html; charset=utf-8",
                shell,
            ),
            ("GET", "/compare.css") => respond(
                request,
                200,
                [KIT_TOKENS, KIT, PAGE_CSS].join("\n").into_bytes(),
                "text/css; charset=utf-8",
                shell,
            ),
            ("GET", "/compare.js") => respond(
                request,
                200,
                PAGE_JS.as_bytes().to_vec(),
                "text/javascript; charset=utf-8",
                shell,
            ),
            ("GET", "/fonts/albertsans.ttf") => respond(
                request,
                200,
                include_bytes!("../../../../ui/component-review/fonts/albertsans.ttf").to_vec(),
                "font/ttf",
                shell,
            ),
            ("GET", "/fonts/alumnisans.ttf") => respond(
                request,
                200,
                include_bytes!("../../../../ui/component-review/fonts/alumnisans.ttf").to_vec(),
                "font/ttf",
                shell,
            ),
            ("GET", "/fonts/jetbrainsmono.ttf") => respond(
                request,
                200,
                include_bytes!("../../../../ui/component-review/fonts/jetbrainsmono.ttf").to_vec(),
                "font/ttf",
                shell,
            ),
            ("GET", "/status") => {
                if keyed {
                    respond(
                        request,
                        200,
                        json_compact(&status(&round)).into_bytes(),
                        "application/json",
                        shell,
                    );
                } else {
                    respond(request, 401, Vec::new(), "text/plain", shell);
                }
            }
            ("GET", p) if p.starts_with("/p/") => {
                let found = proposal_route(&round, &p[3..]).and_then(|(proposal, tail)| {
                    let Source::Html { file, interactive } = &proposal.source else {
                        return None;
                    };
                    let (real, bytes) = read_within(file.parent()?, tail)?;
                    Some((real, bytes, *interactive))
                });
                match found {
                    Some((real, bytes, interactive)) => {
                        let csp = if interactive {
                            INTERACTIVE_CSP
                        } else {
                            PROPOSAL_CSP
                        };
                        respond(request, 200, bytes, content_type(&real), Some(csp));
                    }
                    None => respond(request, 404, Vec::new(), "text/plain", Some(PROPOSAL_CSP)),
                }
            }
            ("GET", p) if p.starts_with("/img/") => {
                let found = proposal_route(&round, &p[5..]).and_then(|(proposal, tail)| {
                    let Source::Image { file, .. } = &proposal.source else {
                        return None;
                    };
                    if !tail.is_empty() {
                        return None;
                    }
                    Some((content_type(file), std::fs::read(file).ok()?))
                });
                match found {
                    Some((mime, bytes)) => respond(request, 200, bytes, mime, shell),
                    None => respond(request, 404, Vec::new(), "text/plain", shell),
                }
            }
            ("POST", "/heartbeat") | ("POST", "/answer") => {
                let origin = header_value(&request, "Origin");
                if let Some(code) =
                    reject_detached_post(Some(&opts.key), origin.as_deref(), &query, port)
                {
                    respond(request, code, Vec::new(), "text/plain", None);
                    continue;
                }
                if path == "/heartbeat" {
                    respond(request, 204, Vec::new(), "text/plain", None);
                    let now = now_ms();
                    if let Ok(mut beat) = last_beat.lock() {
                        *beat = Some(now);
                    }
                    if last_beat_write.is_none_or(|w| now - w > 4000.0) {
                        last_beat_write = Some(now);
                        write_state(&state_path, |s| {
                            s.insert("lastBeat".into(), Value::from(now as i64));
                        });
                    }
                    continue;
                }
                let mut body = Vec::new();
                let _ = request
                    .as_reader()
                    .take(ANSWER_MAX_BYTES + 1)
                    .read_to_end(&mut body);
                let parsed = if body.len() as u64 > ANSWER_MAX_BYTES {
                    None
                } else {
                    serde_json::from_slice::<Value>(&body)
                        .ok()
                        .and_then(|v| v.as_object().cloned())
                };
                let chosen = parsed.as_ref().and_then(|a| {
                    let id = a.get("proposalId")?.as_str()?;
                    round.proposals.iter().find(|p| p.id == id)
                });
                let (Some(parsed), Some(chosen)) = (parsed.as_ref(), chosen) else {
                    respond(request, 400, Vec::new(), "text/plain", None);
                    continue;
                };
                let note = |k: &str| {
                    parsed
                        .get(k)
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .unwrap_or("")
                        .to_string()
                };
                let answer = json!({
                    "proposalId": chosen.id, "name": chosen.name, "baseline": chosen.baseline,
                    "liked": note("liked"), "disliked": note("disliked"),
                });
                // Answer only once the choice is on disk: a 200 promises the waiter a complete answer.
                if write_atomic(
                    &answer_file(&opts.qdir, &opts.key),
                    &format!("{}\n", json_compact(&answer)),
                )
                .is_err()
                {
                    respond(request, 500, Vec::new(), "text/plain", None);
                    continue;
                }
                respond(
                    request,
                    200,
                    b"{\"ok\":true}".to_vec(),
                    "application/json",
                    None,
                );
                sleep_ms(150);
                return 0;
            }
            _ => respond(request, 404, Vec::new(), "text/plain", shell),
        }
    }
    0
}
