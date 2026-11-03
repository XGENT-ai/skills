//! Browser measurements and screenshots of one written HTML file.

mod cdp;
pub mod comparison;
mod discovery;
mod evidence;
mod observation;

use crate::check::FileIdentity;
use crate::diagnostic::{Diagnostic, Diagnostics};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundFile {
    pub path: PathBuf,
    pub identity: FileIdentity,
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
        let assertions = |code: &str| {
            code.strip_prefix("browser/")
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

pub fn check_browser(artifact: &Path, options: &BrowserOptions) -> BrowserReceipt {
    let mut receipt = BrowserReceipt {
        schema_version: 1,
        tool: "phoenix-diagram".into(),
        tool_version: crate::TOOL_VERSION.into(),
        command: "browser-check".into(),
        status: BrowserStatus::Failed,
        artifact_path: artifact.to_owned(),
        artifact: FileIdentity::of(b""),
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
    // Chrome loads a file URL, which must name the file absolutely.
    let artifact = match std::fs::canonicalize(artifact).and_then(|path| {
        let bytes = std::fs::read(&path)?;
        Ok((path, bytes))
    }) {
        Ok((path, bytes)) => {
            receipt.artifact_path = path.clone();
            receipt.artifact = FileIdentity::of(&bytes);
            path
        }
        Err(error) => {
            append_failure(
                &mut receipt,
                cdp::Error::new(
                    "browser/artifact",
                    "read",
                    format!("{}: {error}", artifact.display()),
                ),
            );
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
        match evidence::Evidence::new(&artifact, options.out_dir.as_deref(), None) {
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
            return receipt;
        }
    };
    let inspected = (|| {
        receipt.browser_version = Some(browser.version()?);
        browser.open_page()?;
        for viewport in DESKTOP_VIEWPORTS.into_iter().chain([NARROW_VIEWPORT]) {
            for theme in [Theme::Light, Theme::Dark] {
                let (metrics, mut events) = browser.inspect(&artifact, viewport, theme)?;
                let full_page = metrics
                    .get("fullScrollHeight")
                    .and_then(Value::as_f64)
                    .is_some_and(|height| height > f64::from(viewport.height));
                if let Some(evidence) = evidence.as_mut() {
                    if evidence::requires_capture(viewport, theme) {
                        let png = browser.screenshot(viewport, full_page)?;
                        evidence.write_png(viewport, theme, full_page, &png)?;
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
    if let Some(evidence) = evidence {
        receipt.screenshots = evidence.screenshots;
        let captured = receipt.screenshots.len() == 4;
        receipt.checks.push(BrowserCheck{name:"screenshot-capture".into(),status:if captured{CheckStatus::Pass}else{CheckStatus::Failed},scope:json!({"required":4,"themes":["light","dark"],"viewports":[DESKTOP_VIEWPORTS[0],DESKTOP_VIEWPORTS[3]],"longDiagrams":"full-page"}),evidence:json!({"captured":receipt.screenshots.len()})});
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_browser_error_preserves_public_cleanup_diagnostics() {
        let mut receipt = check_browser(
            Path::new("/diagnostic-only/diagram.html"),
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
