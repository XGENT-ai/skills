//! Read-only browser evidence for one already bound delivery.

mod cdp;
mod discovery;
mod evidence;
mod observation;

use crate::check::FileIdentity;
use crate::diagnostic::{Diagnostic, Diagnostics};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundFile {
    pub path: PathBuf,
    pub identity: FileIdentity,
}

/// Delivery constructs this only after the strict provenance gate succeeds.
/// Paths name the resolved ordinary files; delivery retains any requested alias.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundArtifact {
    pub path: PathBuf,
    pub candidate: BoundFile,
    pub artifact: FileIdentity,
    pub config_sha256: String,
    pub engine: Value,
    pub delivery_receipt_id: String,
}

#[derive(Clone, Debug)]
pub struct BrowserOptions {
    pub executable: Option<PathBuf>,
    pub capture: bool,
    pub out_dir: Option<PathBuf>,
    pub startup_timeout: Duration,
    pub command_timeout: Duration,
}

impl Default for BrowserOptions {
    fn default() -> Self {
        Self {
            executable: None,
            capture: false,
            out_dir: None,
            startup_timeout: Duration::from_secs(90),
            command_timeout: Duration::from_secs(15),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BrowserStatus {
    Pass,
    Failed,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CheckStatus {
    Pass,
    Failed,
    NotRun,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

pub const DESKTOP_VIEWPORTS: [Viewport; 4] = [
    Viewport {
        width: 1440,
        height: 900,
    },
    Viewport {
        width: 1600,
        height: 1000,
    },
    Viewport {
        width: 1920,
        height: 1080,
    },
    Viewport {
        width: 2048,
        height: 1320,
    },
];
pub const NARROW_VIEWPORT: Viewport = Viewport {
    width: 390,
    height: 844,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
}

pub const BROWSER_MEASUREMENT_CHECKS: [&str; 10] = [
    "viewport-containment",
    "theme-state",
    "rendered-dom",
    "projected-text",
    "text-containment",
    "card-clearance",
    "viewer-chrome-clearance",
    "control-hit-rects",
    "runtime-errors",
    "offline-resources",
];

impl Theme {
    fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserCheck {
    pub name: String,
    pub status: CheckStatus,
    pub scope: Value,
    pub evidence: Value,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserCoverage {
    pub viewport: Viewport,
    pub theme: Theme,
    pub status: BrowserStatus,
    pub vertical_scroll_accepted: bool,
    pub overflow_disposition: String,
    pub metrics: Value,
    pub checks: Vec<BrowserCheck>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Screenshot {
    pub path: PathBuf,
    pub identity: FileIdentity,
    pub viewport: Viewport,
    pub width: u32,
    pub height: u32,
    pub theme: Theme,
    pub full_page: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserReceipt {
    pub schema_version: u32,
    pub tool: String,
    pub tool_version: String,
    pub command: String,
    pub status: BrowserStatus,
    pub artifact_path: PathBuf,
    pub artifact: FileIdentity,
    pub candidate: BoundFile,
    pub config_sha256: String,
    pub engine: Value,
    pub delivery_receipt_id: String,
    pub browser_version: Option<Value>,
    pub coverage: Vec<BrowserCoverage>,
    pub checks: Vec<BrowserCheck>,
    pub screenshots: Vec<Screenshot>,
    pub diagnostics: Diagnostics,
}

impl BrowserReceipt {
    /// Environment/ownership failures take priority over measured defects.
    pub fn exit_code(&self) -> u8 {
        if self.status == BrowserStatus::Pass {
            return 0;
        }
        if self.status == BrowserStatus::Unavailable {
            return 5;
        }
        if self
            .diagnostics
            .iter()
            .any(|entry| entry.code == "browser/options")
        {
            return 2;
        }
        if self
            .diagnostics
            .iter()
            .any(|entry| entry.code == "browser/evidence-conflict")
        {
            return 3;
        }
        let assertions = |code: &str| {
            matches!(
                code,
                "browser/artifact-changed"
                    | "browser/candidate-changed"
                    | "browser/evidence-changed"
            ) || code
                .strip_prefix("browser/")
                .is_some_and(|name| BROWSER_MEASUREMENT_CHECKS.contains(&name))
        };
        if self
            .diagnostics
            .iter()
            .any(|entry| !assertions(&entry.code))
        {
            4
        } else {
            1
        }
    }
}

pub fn check_browser(artifact: &BoundArtifact, options: &BrowserOptions) -> BrowserReceipt {
    let mut receipt = BrowserReceipt {
        schema_version: 1,
        tool: "phoenix-diagram".into(),
        tool_version: crate::TOOL_VERSION.into(),
        command: "browser-check".into(),
        status: BrowserStatus::Failed,
        artifact_path: artifact.path.clone(),
        artifact: artifact.artifact.clone(),
        candidate: artifact.candidate.clone(),
        config_sha256: artifact.config_sha256.clone(),
        engine: artifact.engine.clone(),
        delivery_receipt_id: artifact.delivery_receipt_id.clone(),
        browser_version: None,
        coverage: Vec::new(),
        checks: observation::pending_checks(),
        screenshots: Vec::new(),
        diagnostics: Vec::new(),
    };
    if options.startup_timeout.is_zero()
        || options.startup_timeout > Duration::from_secs(90)
        || options.command_timeout.is_zero()
        || options.command_timeout > Duration::from_secs(15)
    {
        append_failure(&mut receipt, cdp::Error::new("browser/options", "options", "Browser deadlines must be positive and bounded by startup=90 seconds and command=15 seconds"));
        return receipt;
    }
    let reads = (|| {
        let html = evidence::BoundRead::capture(
            &artifact.path,
            &artifact.artifact,
            "browser/artifact-changed",
        )?;
        let candidate = evidence::BoundRead::capture(
            &artifact.candidate.path,
            &artifact.candidate.identity,
            "browser/candidate-changed",
        )?;
        Ok::<_, cdp::Error>((html, candidate))
    })();
    let (html, candidate) = match reads {
        Ok(reads) => reads,
        Err(error) => {
            append_failure(&mut receipt, error);
            return receipt;
        }
    };
    let executable = match discovery::find(options.executable.as_deref()) {
        Ok(path) => path,
        Err(error) => {
            if error.code == "browser/unavailable" {
                receipt.status = BrowserStatus::Unavailable;
            }
            append_failure(&mut receipt, error);
            return receipt;
        }
    };
    let mut evidence = if options.capture {
        match evidence::Evidence::reserve(&artifact.path, options.out_dir.as_deref()) {
            Ok(evidence) => Some(evidence),
            Err(error) => {
                append_failure(&mut receipt, error);
                return receipt;
            }
        }
    } else {
        None
    };
    let mut browser = match cdp::Browser::launch(
        &executable,
        options.startup_timeout,
        options.command_timeout,
    ) {
        Ok(browser) => browser,
        Err(error) => {
            append_failure(&mut receipt, error);
            cleanup_evidence(&mut receipt, &mut evidence);
            return receipt;
        }
    };
    let inspected = (|| {
        receipt.browser_version = Some(browser.version()?);
        browser.open_page()?;
        for viewport in DESKTOP_VIEWPORTS.into_iter().chain([NARROW_VIEWPORT]) {
            for theme in [Theme::Light, Theme::Dark] {
                html.verify()?;
                candidate.verify()?;
                let (metrics, mut events) = browser.inspect(&artifact.path, viewport, theme)?;
                html.verify()?;
                candidate.verify()?;
                let full_page = metrics
                    .get("fullScrollHeight")
                    .and_then(Value::as_f64)
                    .is_some_and(|height| height > f64::from(viewport.height));
                if let Some(evidence) = evidence.as_mut() {
                    if evidence.requires_capture(viewport, theme) {
                        let png = browser.screenshot(viewport, full_page)?;
                        evidence.stage_png(viewport, theme, full_page, &png)?;
                        html.verify()?;
                        candidate.verify()?;
                    }
                }
                events.extend(browser.take_events());
                let coverage = observation::classify(viewport, theme, metrics, &events);
                receipt.coverage.push(coverage);
            }
        }
        Ok::<_, cdp::Error>(())
    })();
    let closed = browser.close();
    let trailing = browser.take_events();
    if let Some(last) = receipt.coverage.last_mut() {
        let observed =
            observation::classify(last.viewport, last.theme, last.metrics.clone(), &trailing);
        for check in observed
            .checks
            .into_iter()
            .filter(|check| check.status == CheckStatus::Failed)
        {
            if let Some(previous) = last
                .checks
                .iter_mut()
                .find(|previous| previous.name == check.name)
            {
                if previous.status == CheckStatus::Pass {
                    *previous = check;
                    last.status = BrowserStatus::Failed;
                }
            }
        }
    }
    receipt.checks = observation::summarize(&receipt.coverage);
    let inspected_ok = inspected.is_ok();
    if let Err(error) = inspected {
        append_failure(&mut receipt, error);
    }
    let cleanup_ok = closed.is_ok();
    if let Err(error) = closed {
        append_failure(&mut receipt, error);
    }
    receipt.checks.push(BrowserCheck {
        name: "browser-cleanup".into(),
        status: if cleanup_ok {
            CheckStatus::Pass
        } else {
            CheckStatus::Failed
        },
        scope: json!({"ownedChild":true,"ownedProfile":true}),
        evidence: Value::Null,
    });
    let current = html.verify().and_then(|_| candidate.verify());
    let current_ok = current.is_ok();
    if let Err(error) = current {
        append_failure(&mut receipt, error);
    }
    receipt.checks.push(BrowserCheck {
        name: "artifact-current".into(),
        status: if current_ok {
            CheckStatus::Pass
        } else {
            CheckStatus::Failed
        },
        scope: json!({"artifactAndCandidate":true,"beforeAndAfter":true,"physicalFile":true}),
        evidence: Value::Null,
    });
    let mut captures_ok = !options.capture;
    if inspected_ok && cleanup_ok && current_ok {
        if let Some(evidence) = evidence.as_mut() {
            match evidence.publish() {
                Ok(screenshots) => {
                    receipt.screenshots = screenshots;
                    captures_ok = true;
                }
                Err(error) => append_failure(&mut receipt, error),
            }
        }
    }
    if options.capture {
        receipt.checks.push(BrowserCheck{name:"screenshot-capture".into(),status:if captures_ok{CheckStatus::Pass}else{CheckStatus::Failed},scope:json!({"required":4,"themes":["light","dark"],"viewports":[DESKTOP_VIEWPORTS[0],DESKTOP_VIEWPORTS[3]],"longDiagrams":"full-page"}),evidence:json!({"captured":receipt.screenshots.len()})});
    }
    cleanup_evidence(&mut receipt, &mut evidence);
    if let Err(error) = html.verify().and_then(|_| candidate.verify()) {
        if let Some(check) = receipt
            .checks
            .iter_mut()
            .find(|check| check.name == "artifact-current")
        {
            check.status = CheckStatus::Failed;
        }
        append_failure(&mut receipt, error);
    }
    for coverage in &receipt.coverage {
        for check in &coverage.checks {
            if check.status == CheckStatus::Failed {
                let mut diagnostic = Diagnostic::error(
                    None,
                    &format!("browser/{}", check.name),
                    "",
                    format!(
                        "{} failed at {}x{} ({})",
                        check.name,
                        coverage.viewport.width,
                        coverage.viewport.height,
                        coverage.theme.as_str()
                    ),
                );
                diagnostic.stage = "browser".into();
                diagnostic.evidence = json!({"viewport":coverage.viewport,"theme":coverage.theme,"scope":check.scope,"measurement":check.evidence});
                receipt.diagnostics.push(diagnostic);
            }
        }
    }
    if inspected_ok
        && cleanup_ok
        && current_ok
        && captures_ok
        && receipt.coverage.len() == 10
        && receipt
            .checks
            .iter()
            .all(|check| check.status == CheckStatus::Pass)
        && receipt.diagnostics.is_empty()
    {
        receipt.status = BrowserStatus::Pass;
    }
    receipt
}

fn append_failure(receipt: &mut BrowserReceipt, error: cdp::Error) {
    let mut diagnostic = Diagnostic::error(None, error.code, "", error.message);
    diagnostic.stage = "browser".into();
    diagnostic.evidence = json!({"phase":error.stage,"retainedPath":error.retained});
    if let Some(resources) = error.resources {
        diagnostic.evidence["resources"] = *resources;
    }
    receipt.diagnostics.push(diagnostic);
}

fn cleanup_evidence(receipt: &mut BrowserReceipt, evidence: &mut Option<evidence::Evidence>) {
    if let Some(evidence) = evidence.as_mut() {
        for error in evidence.cleanup() {
            append_failure(receipt, error);
        }
    }
}

/// Read-only discovery for doctor; never downloads or opens a browser.
pub fn discover_browser() -> Result<PathBuf, Box<Diagnostic>> {
    discovery::find(None).map_err(|error| {
        let mut diagnostic = Diagnostic::error(None, error.code, "", error.message);
        diagnostic.stage = "browser".into();
        Box::new(diagnostic)
    })
}

/// Report discovery separately from browser evidence, without launching it.
pub fn doctor_browser() -> Value {
    match discover_browser() {
        Ok(path) => {
            json!({"status":"available","path":path.to_string_lossy(),"diagnostics":[],"browserVerification":"not-run"})
        }
        Err(diagnostic) => {
            json!({"status":if diagnostic.code=="browser/unavailable"{"unavailable"}else{"failed"},"diagnostics":[diagnostic],"browserVerification":"not-run"})
        }
    }
}

/// Validate the exact automatic scope before delivery accepts stored evidence.
/// The caller must separately verify provenance and every screenshot's bytes.
pub fn validate_receipt_coverage(receipt: &BrowserReceipt, capture: bool) -> bool {
    use std::collections::HashSet;
    if receipt.schema_version != 1
        || receipt.tool != "phoenix-diagram"
        || receipt.tool_version != crate::TOOL_VERSION
        || !matches!(receipt.command.as_str(), "browser-check" | "visual-check")
        || receipt.status != BrowserStatus::Pass
        || !receipt
            .browser_version
            .as_ref()
            .and_then(|version| version["product"].as_str())
            .is_some_and(|product| !product.trim().is_empty())
        || !receipt.diagnostics.is_empty()
        || receipt.coverage.len() != 10
    {
        return false;
    }
    let expected: HashSet<_> = DESKTOP_VIEWPORTS
        .into_iter()
        .chain([NARROW_VIEWPORT])
        .flat_map(|viewport| [Theme::Light, Theme::Dark].map(|theme| (viewport, theme)))
        .collect();
    let actual: HashSet<_> = receipt
        .coverage
        .iter()
        .map(|entry| (entry.viewport, entry.theme))
        .collect();
    if actual != expected {
        return false;
    }
    for entry in &receipt.coverage {
        if entry.status != BrowserStatus::Pass
            || *entry
                != observation::classify(entry.viewport, entry.theme, entry.metrics.clone(), &[])
        {
            return false;
        }
    }
    let mut expected_checks = observation::summarize(&receipt.coverage);
    expected_checks.push(BrowserCheck {
        name: "browser-cleanup".into(),
        status: CheckStatus::Pass,
        scope: json!({"ownedChild":true,"ownedProfile":true}),
        evidence: Value::Null,
    });
    expected_checks.push(BrowserCheck {
        name: "artifact-current".into(),
        status: CheckStatus::Pass,
        scope: json!({"artifactAndCandidate":true,"beforeAndAfter":true,"physicalFile":true}),
        evidence: Value::Null,
    });
    if capture {
        expected_checks.push(BrowserCheck{name:"screenshot-capture".into(),status:CheckStatus::Pass,scope:json!({"required":4,"themes":["light","dark"],"viewports":[DESKTOP_VIEWPORTS[0],DESKTOP_VIEWPORTS[3]],"longDiagrams":"full-page"}),evidence:json!({"captured":4})});
    }
    if receipt.checks != expected_checks {
        return false;
    }
    if !capture {
        return receipt.screenshots.is_empty();
    }
    if receipt.screenshots.len() != 4 {
        return false;
    }
    let expected: HashSet<_> = [DESKTOP_VIEWPORTS[0], DESKTOP_VIEWPORTS[3]]
        .into_iter()
        .flat_map(|viewport| [Theme::Light, Theme::Dark].map(|theme| (viewport, theme)))
        .collect();
    let mut actual = HashSet::new();
    let mut paths = HashSet::new();
    for screenshot in &receipt.screenshots {
        let viewport = screenshot.viewport;
        if !actual.insert((viewport, screenshot.theme))
            || !paths.insert(&screenshot.path)
            || screenshot.identity.bytes == 0
            || screenshot.identity.sha256.len() != 64
            || !screenshot
                .identity
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return false;
        }
        let Some(coverage) = receipt
            .coverage
            .iter()
            .find(|entry| entry.viewport == viewport && entry.theme == screenshot.theme)
        else {
            return false;
        };
        let Some(height) = coverage.metrics["fullScrollHeight"]
            .as_f64()
            .filter(|height| height.is_finite())
        else {
            return false;
        };
        if screenshot.full_page != (height > f64::from(viewport.height)) {
            return false;
        }
        if screenshot.width != viewport.width
            || (!screenshot.full_page && screenshot.height != viewport.height)
            || screenshot.full_page && screenshot.height < viewport.height
            || screenshot.full_page && f64::from(screenshot.height) < height.ceil() - 1.0
        {
            return false;
        }
    }
    actual == expected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_browser_error_preserves_public_cleanup_diagnostics() {
        let identity = FileIdentity {
            sha256: "0".repeat(64),
            bytes: 0,
        };
        let artifact = BoundArtifact {
            path: "/diagnostic-only/diagram.html".into(),
            candidate: BoundFile {
                path: "/diagnostic-only/candidate.json".into(),
                identity: identity.clone(),
            },
            artifact: identity,
            config_sha256: "1".repeat(64),
            engine: Value::Null,
            delivery_receipt_id: "diagnostic-only".into(),
        };
        let mut receipt = check_browser(
            &artifact,
            &BrowserOptions {
                startup_timeout: Duration::ZERO,
                ..BrowserOptions::default()
            },
        );
        receipt.diagnostics.clear();
        let resources = json!({"processSpawned":true,"childPid":42,"profile":"/diagnostic-only/profile","startupState":"waiting-for-devtools-endpoint","cleanupStatus":"failed","whileHandling":{"code":"browser/startup-timeout","phase":"startup","message":"deadline expired"}});
        let mut error = cdp::Error::new("browser/cleanup", "cleanup", "owned profile retained");
        assert!(std::mem::size_of_val(&error) <= 96);
        error.retained = Some("/diagnostic-only/profile".into());
        error.resources = Some(Box::new(resources.clone()));
        append_failure(&mut receipt, error);
        let diagnostic = serde_json::to_value(&receipt.diagnostics[0]).unwrap();
        assert_eq!(diagnostic["code"], "browser/cleanup");
        assert_eq!(diagnostic["stage"], "browser");
        assert_eq!(diagnostic["message"], "owned profile retained");
        assert_eq!(
            diagnostic["evidence"],
            json!({"phase":"cleanup","retainedPath":"/diagnostic-only/profile","resources":resources})
        );
        assert_eq!(receipt.exit_code(), 4);
        assert_eq!(
            serde_json::to_value(Box::new(receipt.diagnostics[0].clone())).unwrap(),
            diagnostic
        );
    }
}
