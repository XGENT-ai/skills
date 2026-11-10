//! impeccable-browser: the URL engine of `impeccable detect`, ported from
//! `cli/engine/engines/browser/detect-url.mjs` (+ `engines/visual/
//! screenshot-contrast.mjs`). Instead of puppeteer it discovers an installed
//! Chromium-based browser ([`discovery`]), drives it over CDP ([`cdp`]) with
//! puppeteer's launch flags and page setup, and — since triage D2 — injects
//! only the plain-JS snapshot producer, runs the rule core natively over
//! [`impeccable_core::browser::snapshot::SnapshotDom`] ([`snapshot_engine`]),
//! and maps the findings into [`Finding`]s exactly as `detectUrl` does. No
//! WebAssembly runs next to the page, so the scan no longer needs
//! `Page.setBypassCSP` and a strict-CSP site is scanned passively.
//!
//! Wired into `impeccable detect` through [`impeccable_detect::UrlEngine`]:
//! a single URL uses `detect_url` (`waitUntil: 'networkidle0'`, `settleMs:
//! 0`); several URLs share one browser through [`SharedBrowser`]
//! (`createBrowserDetector()`: `waitUntil: 'load'`, `settleMs: 100`).

pub mod cdp;
pub mod discovery;
pub mod html_snapshot;
pub mod network_log;
pub mod response_capture;
pub mod screenshot_contrast;
pub mod snapshot_engine;

use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Duration;

use impeccable_core::browser::driver::{collect_browser_findings, serialize_findings};
use impeccable_core::browser::page_checks::measure_hidden_text_dom;
use impeccable_core::checks::measures::{check_content_hidden_at_rest, ContentHiddenInput};
use impeccable_core::findings::{try_finding, Finding};
use impeccable_detect::design_system::DesignSystem;
use impeccable_detect::engines::{EngineError, ScanOptions, SharedBrowser, UrlEngine};
use serde_json::{json, Value};

use cdp::{Browser, CdpError, Page, Viewport};

/// puppeteer's default `page.goto` timeout the JS passes explicitly.
const NAVIGATION_TIMEOUT: Duration = Duration::from_millis(30000);

/// The browser engine. Holds the process environment it reads for browser
/// discovery (`PHOENIX_UI_BROWSER`, `PUPPETEER_EXECUTABLE_PATH`,
/// `CHROME_PATH`, standard locations), sandbox flags (`CI`,
/// `PUPPETEER_DANGEROUS_NO_SANDBOX`), and `HOME`/`PATH` for the search.
pub struct BrowserEngine {
    env: HashMap<String, String>,
}

impl BrowserEngine {
    pub fn new(env: HashMap<String, String>) -> Self {
        BrowserEngine { env }
    }

    /// An engine reading the real process environment.
    pub fn from_process_env() -> Self {
        BrowserEngine::new(impeccable_common::process_env())
    }

    /// JS `launchArgs = process.env.CI ? ['--no-sandbox','--disable-setuid-sandbox'] : []`.
    fn launch_args(&self) -> Vec<String> {
        match self.env.get("CI") {
            Some(v) if !v.is_empty() => vec![
                "--no-sandbox".to_string(),
                "--disable-setuid-sandbox".to_string(),
            ],
            _ => Vec::new(),
        }
    }

    fn dangerous_no_sandbox(&self) -> bool {
        self.env
            .get("PUPPETEER_DANGEROUS_NO_SANDBOX")
            .map(String::as_str)
            == Some("true")
    }

    /// `launchBrowser()`: discover, then launch headless.
    pub fn launch(&self) -> Result<Browser, EngineError> {
        let exe = discovery::find_browser(&self.env).map_err(EngineError::new)?;
        Browser::launch(&exe, &self.launch_args(), self.dangerous_no_sandbox())
            .map_err(|e| EngineError::new(e.message))
    }
}

impl UrlEngine for BrowserEngine {
    fn detect_url(&self, url: &str, options: &ScanOptions) -> Result<Vec<Finding>, EngineError> {
        detect_url_impl(self, url, options, "networkidle0", 0, None)
    }

    fn open_shared(&self) -> Option<Box<dyn SharedBrowser + '_>> {
        Some(Box::new(SharedBrowserHandle {
            engine: self,
            browser: RefCell::new(None),
            launch_error: RefCell::new(None),
        }))
    }
}

/// `createBrowserDetector()`: one browser for many URLs, a fresh page per
/// URL. The browser launches lazily on the first scan; a launch failure is
/// remembered and reported for every URL (the JS throws once before the
/// loop and exits 1; the detect seam has no fatal path for `open_shared`,
/// so the failure surfaces per URL as `Error: ...` instead).
pub struct SharedBrowserHandle<'a> {
    engine: &'a BrowserEngine,
    browser: RefCell<Option<Browser>>,
    launch_error: RefCell<Option<String>>,
}

impl SharedBrowser for SharedBrowserHandle<'_> {
    fn detect_url(&self, url: &str, options: &ScanOptions) -> Result<Vec<Finding>, EngineError> {
        self.ensure_launched()?;
        let mut guard = self.browser.borrow_mut();
        let browser = guard.as_mut().expect("launched above");
        detect_url_impl(self.engine, url, options, "load", 100, Some(browser))
    }

    fn close(&self) {
        if let Some(b) = self.browser.borrow_mut().take() {
            b.close();
        }
    }

    fn ensure_launched(&self) -> Result<(), EngineError> {
        if let Some(msg) = self.launch_error.borrow().as_ref() {
            return Err(EngineError::new(msg.clone()));
        }
        if self.browser.borrow().is_some() {
            return Ok(());
        }
        match self.engine.launch() {
            Ok(b) => {
                *self.browser.borrow_mut() = Some(b);
                Ok(())
            }
            Err(e) => {
                *self.launch_error.borrow_mut() = Some(e.message.clone());
                Err(e)
            }
        }
    }
}

/// JS detect-url.mjs `credentials` from `splitScanUrl`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanCredentials {
    pub username: String,
    pub password: String,
}

/// JS `decodeUrlComponent`: `decodeURIComponent` with a fall-through to the
/// raw value when decoding fails.
// JS-PARITY: decodeURIComponent throws on a malformed escape and the JS
// returns the raw value; percent_decode leaves malformed escapes literal,
// which yields the same string for the common cases.
fn decode_url_component(value: &str) -> String {
    match percent_encoding::percent_decode_str(value).decode_utf8() {
        Ok(s) => s.into_owned(),
        Err(_) => value.to_string(),
    }
}

/// JS detect-url.mjs#splitScanUrl(url): strip basic-auth userinfo from the
/// scan target (so it never reaches goto targets or finding output) and hand
/// back http/https credentials separately (issue #657).
pub fn split_scan_url(url: &str) -> (String, Option<ScanCredentials>) {
    let Ok(mut parsed) = url::Url::parse(url) else {
        return (url.to_string(), None);
    };
    if parsed.username().is_empty() && parsed.password().unwrap_or("").is_empty() {
        return (url.to_string(), None);
    }
    let credentials = match parsed.scheme() {
        "http" | "https" => Some(ScanCredentials {
            username: decode_url_component(parsed.username()),
            password: decode_url_component(parsed.password().unwrap_or("")),
        }),
        _ => None,
    };
    let _ = parsed.set_username("");
    let _ = parsed.set_password(None);
    (parsed.as_str().to_string(), credentials)
}

/// `serializeDesignSystemForBrowser(designSystem)`.
pub fn serialize_design_system_for_browser(ds: Option<&DesignSystem>) -> Value {
    let Some(ds) = ds else { return Value::Null };
    if !ds.present {
        return Value::Null;
    }
    let colors: Vec<Value> = ds
        .allowed_color_keys
        .iter()
        .map(|(_, entry)| &entry.color)
        .filter(|c| c.r.is_finite() && c.g.is_finite() && c.b.is_finite())
        .map(|c| json!({ "r": c.r, "g": c.g, "b": c.b }))
        .collect();
    let radii: Vec<Value> = ds
        .allowed_radii
        .iter()
        .map(|r| r.px)
        .filter(|px| px.is_finite())
        .map(|px| json!(px))
        .collect();
    json!({
        "present": true,
        "hasFonts": ds.has_fonts,
        "allowedFonts": ds.allowed_fonts,
        "hasColors": ds.has_colors,
        "allowedColors": colors,
        "hasRadii": ds.has_radii,
        "allowedRadii": radii,
        "hasPillRadius": ds.has_pill_radius,
        "declaredSelectors": ds.declared_selectors,
    })
}

/// A pre-registry finding as `detectUrl` accumulates them.
struct RawResult {
    id: String,
    snippet: String,
    ignore_value: String,
    severity: String,
}

fn cdp_err(e: CdpError) -> EngineError {
    EngineError::new(e.message)
}

fn js_str(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => {
            impeccable_core::js::number_to_string(n.as_f64().unwrap_or(f64::NAN))
        }
        Some(other) => other.to_string(),
    }
}

/// JS `x || ''` on a JSON value.
fn js_str_or_empty(v: Option<&Value>) -> String {
    match v {
        Some(Value::Bool(false)) | Some(Value::Null) | None => String::new(),
        Some(Value::Number(n)) if n.as_f64() == Some(0.0) => String::new(),
        other => js_str(other),
    }
}

/// `detectUrl(url, options)` with the wait/settle defaults the caller picks
/// and an optional shared browser (`options.browser`).
fn detect_url_impl(
    engine: &BrowserEngine,
    url: &str,
    options: &ScanOptions,
    wait_until: &str,
    settle_ms: u64,
    external: Option<&mut Browser>,
) -> Result<Vec<Finding>, EngineError> {
    // JS `const { href: url, credentials } = splitScanUrl(rawUrl)` (issue #657):
    // everything below (goto, finding output) sees the
    // redacted href only.
    let (url, credentials) = split_scan_url(url);
    let url = url.as_str();
    let credentials = credentials.as_ref();
    let (vw, vh) = options.viewport.unwrap_or((1280, 800));
    let viewport = Viewport {
        width: vw,
        height: vh,
    };
    let owns_browser = external.is_none();
    let mut owned: Option<Browser> = None;
    let browser: &mut Browser = match external {
        Some(b) => b,
        None => owned.insert(engine.launch()?),
    };

    let page = browser.new_page().map_err(cdp_err);
    let scanned = match page {
        Ok(page) => scan_page(
            page,
            url,
            credentials,
            options,
            wait_until,
            settle_ms,
            viewport,
        ),
        Err(e) => Err(e),
    };
    // finally: close page (inside scan_page) and the browser when owned.
    if owns_browser {
        if let Some(b) = owned.take() {
            b.close();
        }
    }
    scanned
}

/// Everything between `newPage` and the `finally` that closes the page.
#[allow(clippy::too_many_arguments)]
fn scan_page(
    mut page: Page<'_>,
    url: &str,
    credentials: Option<&ScanCredentials>,
    options: &ScanOptions,
    wait_until: &str,
    settle_ms: u64,
    viewport: Viewport,
) -> Result<Vec<Finding>, EngineError> {
    let outcome = scan_page_inner(
        &mut page,
        url,
        credentials,
        options,
        wait_until,
        settle_ms,
        viewport,
    );
    page.close();
    outcome
}

#[allow(clippy::too_many_arguments)]
fn scan_page_inner(
    page: &mut Page<'_>,
    url: &str,
    credentials: Option<&ScanCredentials>,
    options: &ScanOptions,
    wait_until: &str,
    settle_ms: u64,
    viewport: Viewport,
) -> Result<Vec<Finding>, EngineError> {
    page.set_viewport(viewport).map_err(cdp_err)?;
    // JS `await applyOriginScopedAuth(page, url, credentials)` (issue #657).
    if let Some(creds) = credentials {
        page.apply_origin_scoped_auth(url, &creds.username, &creds.password)
            .map_err(cdp_err)?;
    }
    page.goto(url, wait_until, NAVIGATION_TIMEOUT)
        .map_err(cdp_err)?;
    if settle_ms > 0 {
        std::thread::sleep(Duration::from_millis(settle_ms));
    }
    scan_loaded_page(page, url, options, viewport)
}

/// The rule scan of a page that is already loaded, shared by `detect <url>`
/// and `capture`, so both report the same findings for the same page state.
/// `url` becomes each finding's `file`.
pub fn scan_loaded_page(
    page: &mut Page<'_>,
    url: &str,
    options: &ScanOptions,
    viewport: Viewport,
) -> Result<Vec<Finding>, EngineError> {
    // Inject the plain-JS snapshot producer (no WebAssembly runs in the page).
    snapshot_engine::ensure_snapshot_js(page).map_err(cdp_err)?;

    let config = snapshot_engine::browser_config(
        serialize_design_system_for_browser(options.design_system.as_deref()),
        options.rule_pack,
    );

    // Deterministic pass: capture the page and run the rule core natively over
    // the snapshot (hit-test misses answered to a fixpoint). serialize_findings
    // reproduces the same per-finding fields (type/detail/ignoreValue/severity)
    // and order the in-page bundle's `impeccableDetect({ serialize: true })`
    // produced; the group selectors feed the visual pass below.
    let dom = snapshot_engine::capture_snapshot(page).map_err(cdp_err)?;
    let collected =
        snapshot_engine::resolve_needs(&dom, page, |d| collect_browser_findings(d, &config))
            .map_err(cdp_err)?;
    let serialized_groups: Vec<Value> = serialize_findings(&dom, &collected.groups)
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut results = Vec::new();
    for group in &serialized_groups {
        let Some(findings) = group.get("findings").and_then(Value::as_array) else {
            continue;
        };
        for f in findings {
            results.push(RawResult {
                id: js_str(f.get("type")),
                snippet: js_str(f.get("detail")),
                ignore_value: js_str_or_empty(f.get("ignoreValue")),
                severity: js_str_or_empty(f.get("severity")),
            });
        }
    }

    // content-hidden-at-rest: reveal sweep, then one post-reveal capture the
    // hidden-text measure and the visual pass share (the reveal sweep leaves the
    // page revealed and scrolled to the top — the scroll-0 snapshot the in-page
    // path measured and analyzed).
    reveal_sweep(page).map_err(cdp_err)?;
    let base = snapshot_engine::capture_snapshot(page).map_err(cdp_err)?;

    let measured = snapshot_engine::resolve_needs(&base, page, |d| measure_hidden_text_dom(d))
        .map_err(cdp_err)?;
    let input = ContentHiddenInput {
        total_chars: measured.total_chars,
        hidden_chars: measured.hidden_chars,
        hidden_samples: measured.hidden_samples,
    };
    results.extend(
        check_content_hidden_at_rest(&input)
            .into_iter()
            .map(|f| RawResult {
                id: f.id,
                snippet: f.snippet,
                ignore_value: String::new(),
                severity: String::new(),
            }),
    );

    for message in page.page_errors().into_iter().take(3) {
        results.push(RawResult {
            id: "script-error".to_string(),
            snippet: message,
            ignore_value: String::new(),
            severity: String::new(),
        });
    }

    let analyses =
        snapshot_engine::analyze_visual_contrast(page, &base, 12.0, true).map_err(cdp_err)?;
    let visual = run_visual_contrast_fallback(page, &analyses, &serialized_groups, viewport)?;
    results.extend(visual);

    let mut findings = Vec::with_capacity(results.len());
    for r in results {
        let Some(mut item) = try_finding(&r.id, url, &r.snippet, 0.0) else {
            // JS: `finding()` dereferences an unknown registry entry.
            return Err(EngineError::new(
                "Cannot read properties of undefined (reading 'name')",
            ));
        };
        if !r.ignore_value.is_empty() {
            item.extras
                .insert("ignoreValue".into(), Value::String(r.ignore_value));
        }
        if !r.severity.is_empty() && r.severity != item.severity {
            item.severity = r.severity;
        }
        impeccable_core::findings::derive_advisory_flag(&mut item);
        findings.push(item);
    }
    Ok(findings)
}

/// The `measureContentHiddenAfterReveal` reveal sweep: scroll the page top to
/// bottom (revealing lazy / on-scroll content), then back to the top and
/// settle. The hidden-text measure then runs natively over a fresh capture.
pub fn reveal_sweep(page: &mut Page<'_>) -> Result<(), CdpError> {
    page.evaluate_value(
        r#"(async () => {
    const step = Math.max(200, Math.floor(window.innerHeight * 0.7));
    const max = Math.max(
      document.documentElement.scrollHeight || 0,
      document.body?.scrollHeight || 0,
    );
    for (let y = 0; y <= max; y += step) {
      window.scrollTo({ top: y, left: 0, behavior: 'instant' });
      await new Promise(resolve => requestAnimationFrame(() => setTimeout(resolve, 40)));
    }
    window.scrollTo({ top: 0, left: 0, behavior: 'instant' });
    await new Promise(resolve => setTimeout(resolve, 700));
  })()"#,
    )?;
    Ok(())
}

/// `runVisualContrastFallback(page, serializedGroups, options, profile,
/// target)`: the JS post-processing of the analytic/canvas analyses
/// (`analyzeVisualContrast`, computed natively in [`snapshot_engine`]) plus the
/// screenshot pixel fallback for candidates the analyses left unresolved.
fn run_visual_contrast_fallback(
    page: &mut Page<'_>,
    browser_analyses: &[Value],
    serialized_groups: &[Value],
    viewport: Viewport,
) -> Result<Vec<RawResult>, EngineError> {
    let existing_low_contrast: Vec<String> = serialized_groups
        .iter()
        .filter(|g| {
            g.get("findings")
                .and_then(Value::as_array)
                .map(|fs| {
                    fs.iter()
                        .any(|f| f.get("type").and_then(Value::as_str) == Some("low-contrast"))
                })
                .unwrap_or(false)
        })
        .filter_map(|g| g.get("selector").and_then(Value::as_str))
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();

    let mut findings: Vec<RawResult> = browser_analyses
        .iter()
        .filter(|r| {
            truthy(r.get("finding"))
                && !existing_low_contrast
                    .iter()
                    .any(|s| Some(s.as_str()) == r.get("selector").and_then(Value::as_str))
        })
        .filter_map(|r| r.get("finding"))
        .map(|f| RawResult {
            id: js_str(f.get("id")),
            snippet: js_str(f.get("snippet")),
            ignore_value: String::new(),
            severity: String::new(),
        })
        .collect();

    // JS `candidates = browserAnalyses.length ? browserAnalyses : collect(...)`.
    // An analysis is the candidate spread with its result, so the analyses are
    // the candidate list; when there are none, there are none to collect.
    let candidates: &[Value] = browser_analyses;

    let browser_resolved: Vec<String> = browser_analyses
        .iter()
        .filter(|r| {
            matches!(
                r.get("status").and_then(Value::as_str),
                Some("fail") | Some("pass")
            )
        })
        .filter_map(|r| r.get("selector").and_then(Value::as_str))
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    let filtered: Vec<&Value> = candidates
        .iter()
        .filter(|c| {
            let sel = c.get("selector").and_then(Value::as_str);
            !existing_low_contrast
                .iter()
                .any(|s| Some(s.as_str()) == sel)
                && !browser_resolved.iter().any(|s| Some(s.as_str()) == sel)
        })
        .collect();
    for candidate in filtered {
        let found = screenshot_contrast::capture_visual_contrast_candidate(
            page,
            candidate,
            viewport.width as f64,
        )
        .map_err(cdp_err)?;
        findings.extend(found.map(|f| RawResult {
            id: f.id.to_string(),
            snippet: f.snippet,
            ignore_value: String::new(),
            severity: String::new(),
        }));
    }
    Ok(findings)
}

fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().map(|f| f != 0.0 && !f.is_nan()).unwrap_or(false),
        Some(Value::String(s)) => !s.is_empty(),
        Some(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn design_system_serialization_shape() {
        assert!(serialize_design_system_for_browser(None).is_null());
        let ds = DesignSystem::default();
        assert!(serialize_design_system_for_browser(Some(&ds)).is_null());
    }

    // Expected values come from tests/detect-url-launch.test.mjs (issue #657).
    #[test]
    fn split_scan_url_matches_js() {
        let creds = |u: &str, p: &str| {
            Some(ScanCredentials {
                username: u.to_string(),
                password: p.to_string(),
            })
        };
        assert_eq!(
            split_scan_url("https://user:pass@example.com"),
            ("https://example.com/".to_string(), creds("user", "pass"))
        );
        assert_eq!(
            split_scan_url("https://user:p%40ss@example.com/path?q=1"),
            (
                "https://example.com/path?q=1".to_string(),
                creds("user", "p@ss")
            )
        );
        assert_eq!(
            split_scan_url("https://user@example.com"),
            ("https://example.com/".to_string(), creds("user", ""))
        );
        assert_eq!(
            split_scan_url("http://:secret@host.com/"),
            ("http://host.com/".to_string(), creds("", "secret"))
        );
        assert_eq!(
            split_scan_url("https://example.com"),
            ("https://example.com".to_string(), None)
        );
        assert_eq!(
            split_scan_url("https://example.com/path?email=a@b.com"),
            ("https://example.com/path?email=a@b.com".to_string(), None)
        );
        assert_eq!(
            split_scan_url("https://user:pass@[::1]:8080/x"),
            ("https://[::1]:8080/x".to_string(), creds("user", "pass"))
        );
        assert_eq!(
            split_scan_url("file:///tmp/a.html"),
            ("file:///tmp/a.html".to_string(), None)
        );
        assert_eq!(
            split_scan_url("not a url"),
            ("not a url".to_string(), None)
        );
    }
}
