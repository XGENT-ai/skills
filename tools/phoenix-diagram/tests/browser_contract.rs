#[path = "support/browser_protocol.rs"]
mod browser_protocol;
use browser_protocol::{assert_owned_browser_cleaned, launcher, Mode, ProtocolFixture};

use phoenix_diagram::browser::{
    check_browser, validate_receipt_coverage, BoundArtifact, BoundFile, BrowserOptions,
    BrowserStatus,
};
use phoenix_diagram::check::FileIdentity;
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

struct TempDir(PathBuf);
impl TempDir {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "phoenix-diagram-browser-test-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn artifact(&self) -> BoundArtifact {
        let candidate = self.0.join("candidate.json");
        let path = self.0.join("diagram.html");
        fs::write(&candidate, b"{}").unwrap();
        fs::write(&path, b"<!doctype html><title>Test</title>").unwrap();
        BoundArtifact {
            path,
            candidate: BoundFile {
                path: candidate,
                identity: FileIdentity::of(b"{}"),
            },
            artifact: FileIdentity::of(b"<!doctype html><title>Test</title>"),
            config_sha256: "a".repeat(64),
            engine: json!({"sourceInputSha256":"b".repeat(64),"inputContractSchema":1,"viewerRevision":"phoenix-v1","receiptSchema":1}),
            delivery_receipt_id: "delivery-test".into(),
        }
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn deadlines_are_bounded_and_do_not_disable_the_sandbox() {
    let options = BrowserOptions::default();
    assert_eq!(options.startup_timeout, Duration::from_secs(90));
    assert_eq!(options.command_timeout, Duration::from_secs(15));
    assert!(options.executable.is_none());
    let temp = TempDir::new();
    let artifact = temp.artifact();
    for timeout in [Duration::ZERO, Duration::from_secs(91)] {
        let receipt = check_browser(
            &artifact,
            &BrowserOptions {
                startup_timeout: timeout,
                ..Default::default()
            },
        );
        assert_eq!(receipt.exit_code(), 2);
    }
}

#[test]
fn an_explicit_missing_browser_is_a_configuration_failure() {
    let temp = TempDir::new();
    let artifact = temp.artifact();
    let receipt = check_browser(
        &artifact,
        &BrowserOptions {
            executable: Some(temp.0.join("no-browser")),
            ..Default::default()
        },
    );
    assert_eq!(receipt.status, BrowserStatus::Failed);
    assert_eq!(receipt.exit_code(), 4);
    assert!(receipt
        .diagnostics
        .iter()
        .any(|entry| entry.code == "browser/configured-path"));
    assert!(receipt.coverage.is_empty());
    assert!(receipt.screenshots.is_empty());
}

#[test]
fn artifact_and_candidate_hash_mismatches_stop_before_browser_launch() {
    let temp = TempDir::new();
    let artifact = temp.artifact();
    fs::write(&artifact.path, b"changed artifact").unwrap();
    let receipt = check_browser(
        &artifact,
        &BrowserOptions {
            executable: Some(temp.0.join("no-browser")),
            ..Default::default()
        },
    );
    assert!(receipt
        .diagnostics
        .iter()
        .any(|entry| entry.code == "browser/artifact-changed"));
    assert_eq!(receipt.exit_code(), 1);
    fs::write(&artifact.path, b"<!doctype html><title>Test</title>").unwrap();
    fs::write(&artifact.candidate.path, b"changed candidate").unwrap();
    let receipt = check_browser(&artifact, &BrowserOptions::default());
    assert!(receipt
        .diagnostics
        .iter()
        .any(|entry| entry.code == "browser/candidate-changed"));
    assert!(receipt.coverage.is_empty());
}

#[test]
fn startup_timeout_stops_only_the_owned_child_and_profile() {
    for (case, telemetry, timeout) in [
        ("before-spawn", true, Duration::from_nanos(1)),
        ("waiting-for-devtools", true, Duration::from_millis(500)),
        (
            "without-fixture-telemetry",
            false,
            Duration::from_millis(500),
        ),
    ] {
        let temp = TempDir::new();
        let artifact = temp.artifact();
        let executable = launcher(&temp.0, None);
        if !telemetry {
            // A real child can hang before emitting any startup telemetry.
            // Its resources must still be observable and cleaned by the owner.
            fs::write(&executable, "#!/bin/sh\nexec /bin/sleep 30\n").unwrap();
        }
        let started = std::time::Instant::now();
        let receipt = check_browser(
            &artifact,
            &BrowserOptions {
                executable: Some(executable),
                startup_timeout: timeout,
                command_timeout: Duration::from_millis(100),
                ..Default::default()
            },
        );
        assert_eq!(receipt.status, BrowserStatus::Failed, "{case}");
        assert_eq!(receipt.exit_code(), 4, "{case}");
        let failure = receipt
            .diagnostics
            .iter()
            .find(|entry| entry.code == "browser/startup-timeout")
            .unwrap_or_else(|| panic!("{case}: {:?}", receipt.diagnostics));
        let resources = &failure.evidence["resources"];
        assert_eq!(resources["cleanupStatus"], "pass", "{case}: {resources}");
        let profile = PathBuf::from(
            resources["profile"]
                .as_str()
                .expect("actual native profile binding"),
        );
        assert!(
            !profile.exists(),
            "{case}: profile was retained: {}",
            profile.display()
        );
        match resources["processSpawned"]
            .as_bool()
            .expect("actual spawn state")
        {
            true => {
                let pid = resources["childPid"].as_u64().expect("actual child PID") as i32;
                assert_eq!(
                    unsafe { libc::kill(pid, 0) },
                    -1,
                    "{case}: owned child is still alive"
                );
                assert_eq!(resources["startupState"], "waiting-for-devtools");
                // Corroborate actual shell telemetry when that write finished;
                // it is not a precondition for the native timeout/cleanup.
                if let Ok(raw) = fs::read_to_string(temp.0.join("child-pid")) {
                    if let Ok(observed) = raw.trim().parse::<i32>() {
                        assert_eq!(pid, observed);
                    }
                }
                if let Ok(raw) = fs::read_to_string(temp.0.join("profile-path")) {
                    if !raw.is_empty() {
                        assert_eq!(profile, PathBuf::from(raw));
                    }
                }
            }
            false => {
                assert!(resources["childPid"].is_null());
                assert_eq!(resources["startupState"], "before-spawn");
            }
        }
        if case == "before-spawn" {
            assert_eq!(resources["processSpawned"], false);
        }
        if !telemetry {
            assert!(!temp.0.join("profile-path").exists());
            assert!(!temp.0.join("child-pid").exists());
        }
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(
            fs::read(&artifact.path).unwrap(),
            b"<!doctype html><title>Test</title>"
        );
    }
}

#[test]
fn protocol_failure_and_timeout_are_failed_and_release_owned_resources() {
    for (mode, code) in [
        (Mode::ProtocolError, "browser/protocol"),
        (Mode::ProtocolTimeout, "browser/cdp-timeout"),
    ] {
        let temp = TempDir::new();
        let artifact = temp.artifact();
        let fixture = ProtocolFixture::new(mode);
        let executable = launcher(&temp.0, Some(&fixture.endpoint));
        let receipt = check_browser(
            &artifact,
            &BrowserOptions {
                executable: Some(executable),
                command_timeout: Duration::from_millis(100),
                ..Default::default()
            },
        );
        assert_eq!(receipt.status, BrowserStatus::Failed);
        assert_eq!(receipt.exit_code(), 4);
        assert!(
            receipt.diagnostics.iter().any(|entry| entry.code == code),
            "{:?}",
            receipt.diagnostics
        );
        assert!(receipt.coverage.is_empty());
        assert_owned_browser_cleaned(&temp.0);
    }
}

#[test]
fn protocol_fixture_records_all_scopes_and_exclusively_seals_four_long_captures() {
    let temp = TempDir::new();
    let artifact = temp.artifact();
    let fixture = ProtocolFixture::new(Mode::Complete {
        remote: false,
        change_artifact: None,
        long: true,
    });
    let executable = launcher(&temp.0, Some(&fixture.endpoint));
    let out_dir = temp.0.join("evidence");
    let receipt = check_browser(
        &artifact,
        &BrowserOptions {
            executable: Some(executable),
            capture: true,
            out_dir: Some(out_dir.clone()),
            ..Default::default()
        },
    );
    assert_eq!(
        receipt.status,
        BrowserStatus::Pass,
        "{:?}",
        receipt.diagnostics
    );
    assert_eq!(receipt.exit_code(), 0);
    assert_eq!(receipt.coverage.len(), 10);
    assert_eq!(receipt.screenshots.len(), 4);
    assert!(validate_receipt_coverage(&receipt, true));
    for screenshot in &receipt.screenshots {
        assert_eq!(
            FileIdentity::of(&fs::read(&screenshot.path).unwrap()),
            screenshot.identity
        );
        assert!(screenshot.full_page);
    }
    assert_eq!(fs::read_dir(&out_dir).unwrap().count(), 4);
    assert_owned_browser_cleaned(&temp.0);
    let commands = fixture.commands.lock().unwrap();
    assert_eq!(
        commands
            .iter()
            .filter(|request| request["method"] == "Page.navigate")
            .count(),
        10
    );
    assert_eq!(
        commands
            .iter()
            .filter(|request| request["method"] == "Page.captureScreenshot")
            .count(),
        4
    );
    assert!(commands
        .iter()
        .filter(|request| request["method"] == "Page.captureScreenshot")
        .all(|request| request["params"]["captureBeyondViewport"] == true));
    let mut missing = receipt.clone();
    missing.coverage.pop();
    assert!(!validate_receipt_coverage(&missing, true));
    let mut duplicate = receipt.clone();
    duplicate.coverage[1] = duplicate.coverage[0].clone();
    assert!(!validate_receipt_coverage(&duplicate, true));
    let mut wrong_scope = receipt.clone();
    wrong_scope.checks[0].scope = json!({"requiredViewports":1,"executedViewports":1});
    assert!(!validate_receipt_coverage(&wrong_scope, true));
    let mut clipped = receipt.clone();
    clipped.screenshots[0].full_page = false;
    assert!(!validate_receipt_coverage(&clipped, true));
    let mut missing_version = receipt.clone();
    missing_version.browser_version = None;
    assert!(!validate_receipt_coverage(&missing_version, true));
}

#[test]
fn full_page_capture_preserves_the_stable_reader_and_restores_script_execution() {
    let temp = TempDir::new();
    let fixture = ProtocolFixture::new(Mode::Complete {
        remote: false,
        change_artifact: None,
        long: true,
    });
    let receipt = check_browser(
        &temp.artifact(),
        &BrowserOptions {
            executable: Some(launcher(&temp.0, Some(&fixture.endpoint))),
            capture: true,
            out_dir: Some(temp.0.join("evidence")),
            ..Default::default()
        },
    );
    assert_eq!(
        receipt.status,
        BrowserStatus::Pass,
        "{:?}",
        receipt.diagnostics
    );
    assert_eq!(receipt.screenshots.len(), 4);
    assert_owned_browser_cleaned(&temp.0);
    let commands = fixture.commands.lock().unwrap();
    for (index, command) in commands.iter().enumerate() {
        if command["method"] != "Page.captureScreenshot" {
            continue;
        }
        let suspend = &commands[index - 1];
        assert_eq!(suspend["method"], "Emulation.setScriptExecutionDisabled");
        assert_eq!(suspend["params"]["value"], true);
        let restore = &commands[index + 1];
        assert_eq!(restore["method"], "Emulation.setScriptExecutionDisabled");
        assert_eq!(restore["params"]["value"], false);
        let settled = &commands[index + 2];
        assert_eq!(settled["method"], "Runtime.evaluate");
        assert_eq!(settled["params"]["awaitPromise"], true);
        let expression = settled["params"]["expression"].as_str().unwrap();
        assert!(expression.contains("layoutStability.whenStable"));
        assert!(expression.contains("await"));
        assert_eq!(suspend["sessionId"], command["sessionId"]);
        assert_eq!(restore["sessionId"], command["sessionId"]);
    }
}

#[test]
fn full_page_protection_leaves_short_viewport_captures_unchanged() {
    let temp = TempDir::new();
    let fixture = ProtocolFixture::new(Mode::Complete {
        remote: false,
        change_artifact: None,
        long: false,
    });
    let receipt = check_browser(
        &temp.artifact(),
        &BrowserOptions {
            executable: Some(launcher(&temp.0, Some(&fixture.endpoint))),
            capture: true,
            out_dir: Some(temp.0.join("evidence")),
            ..Default::default()
        },
    );
    assert_eq!(
        receipt.status,
        BrowserStatus::Pass,
        "{:?}",
        receipt.diagnostics
    );
    assert!(receipt.screenshots.iter().all(|image| !image.full_page));
    assert_owned_browser_cleaned(&temp.0);
    assert!(!fixture
        .commands
        .lock()
        .unwrap()
        .iter()
        .any(|command| { command["method"] == "Emulation.setScriptExecutionDisabled" }));
}

fn assert_full_page_failure(mode: Mode, messages: &[&str]) {
    let temp = TempDir::new();
    let fixture = ProtocolFixture::new(mode);
    let out_dir = temp.0.join("evidence");
    let receipt = check_browser(
        &temp.artifact(),
        &BrowserOptions {
            executable: Some(launcher(&temp.0, Some(&fixture.endpoint))),
            capture: true,
            out_dir: Some(out_dir.clone()),
            ..Default::default()
        },
    );
    assert_eq!(receipt.status, BrowserStatus::Failed);
    assert_ne!(receipt.exit_code(), 0);
    assert!(receipt.screenshots.is_empty());
    assert!(!out_dir.exists() || fs::read_dir(&out_dir).unwrap().next().is_none());
    assert_owned_browser_cleaned(&temp.0);
    assert!(
        fixture.commands.lock().unwrap().iter().any(|command| {
            command["method"] == "Emulation.setScriptExecutionDisabled"
                && command["params"]["value"] == false
        }),
        "script restoration must be attempted even when capture fails"
    );
    let diagnostics = serde_json::to_string(&receipt.diagnostics).unwrap();
    for message in messages {
        assert!(diagnostics.contains(message), "{diagnostics}");
    }
}

#[test]
fn full_page_capture_error_still_restores_scripts() {
    assert_full_page_failure(
        Mode::CaptureFailure {
            restore_fails: false,
        },
        &["fixture capture rejected"],
    );
}

#[test]
fn full_page_script_restore_error_cannot_publish_a_passed_capture() {
    assert_full_page_failure(
        Mode::ScriptRestoreFailure,
        &["fixture script restore rejected"],
    );
}

#[test]
fn full_page_script_restore_error_retains_the_capture_error() {
    assert_full_page_failure(
        Mode::CaptureFailure {
            restore_fails: true,
        },
        &[
            "fixture capture rejected",
            "fixture script restore rejected",
        ],
    );
}

#[test]
fn full_page_restored_layout_error_cannot_publish_a_passed_capture() {
    assert_full_page_failure(
        Mode::RestoredLayoutFailure,
        &["fixture restored layout did not stabilize"],
    );
}

#[test]
fn remote_requests_fail_and_during_inspection_changes_cannot_receive_evidence() {
    for remote in [true, false] {
        let temp = TempDir::new();
        let artifact = temp.artifact();
        let fixture = ProtocolFixture::new(Mode::Complete {
            remote,
            change_artifact: if remote {
                None
            } else {
                Some(artifact.path.clone())
            },
            long: false,
        });
        let executable = launcher(&temp.0, Some(&fixture.endpoint));
        let receipt = check_browser(
            &artifact,
            &BrowserOptions {
                executable: Some(executable),
                ..Default::default()
            },
        );
        assert_eq!(receipt.status, BrowserStatus::Failed);
        assert_eq!(receipt.exit_code(), 1);
        assert!(
            receipt.diagnostics.iter().any(|entry| entry.code
                == if remote {
                    "browser/offline-resources"
                } else {
                    "browser/artifact-changed"
                }),
            "{:?}",
            receipt.diagnostics
        );
        assert!(!validate_receipt_coverage(&receipt, false));
        assert_owned_browser_cleaned(&temp.0);
    }
}

#[test]
fn any_existing_screenshot_entry_is_preserved_without_launching() {
    use std::os::unix::fs::symlink;
    for kind in ["regular", "symlink", "hardlink", "directory"] {
        let temp = TempDir::new();
        let artifact = temp.artifact();
        let executable = launcher(&temp.0, None);
        let out_dir = temp.0.join("evidence");
        fs::create_dir(&out_dir).unwrap();
        let path = out_dir.join("diagram.visual-check.1440x900.light.png");
        let original = temp.0.join("original");
        fs::write(&original, b"preserve me").unwrap();
        match kind {
            "regular" => fs::write(&path, b"preserve me").unwrap(),
            "symlink" => symlink(&original, &path).unwrap(),
            "hardlink" => fs::hard_link(&original, &path).unwrap(),
            _ => fs::create_dir(&path).unwrap(),
        }
        let receipt = check_browser(
            &artifact,
            &BrowserOptions {
                executable: Some(executable),
                capture: true,
                out_dir: Some(out_dir.clone()),
                ..Default::default()
            },
        );
        assert_eq!(receipt.status, BrowserStatus::Failed);
        assert_eq!(receipt.exit_code(), 3);
        assert!(receipt
            .diagnostics
            .iter()
            .any(|entry| entry.code == "browser/evidence-conflict"));
        assert!(!temp.0.join("child-pid").exists());
        assert_eq!(fs::read(&original).unwrap(), b"preserve me");
        assert_eq!(fs::read_dir(&out_dir).unwrap().count(), 1);
        if kind != "directory" {
            assert_eq!(fs::read(&path).unwrap(), b"preserve me");
        } else {
            assert!(path.is_dir());
        }
    }
}

/// This real DOM counterexample belongs to the core-stage browser run. It is
/// ignored in the protocol-only local suite and never installs a browser.
#[test]
#[ignore = "real Chrome and native Reader acceptance run only at the core stage"]
fn real_dom_css_hiding_a_node_path_or_required_label_is_rejected() {
    use phoenix_diagram::{
        input::{parse_input, CompileOptions, DiagramType, Quality},
        layout,
        render::{self, EmbeddedAssets},
    };
    use std::os::unix::fs::OpenOptionsExt;
    let executable = phoenix_diagram::browser::discover_browser()
        .expect("the core stage requires installed Chrome");
    let temp = TempDir::new();
    let document = json!({"schema_version":1,"diagram_type":"architecture","meta":{"title":"Browser content fixture","output":"diagram.html","input_contract":"phoenix-v1","quality_profile":"standard","legend":{"mode":"hidden"}},"components":[{"id":"client","type":"frontend","label":"Client","sublabel":"Request owner","tag":"api-v1","pos":[60,100],"size":[160,80]},{"id":"service","type":"backend","label":"Service","pos":[360,100],"size":[160,80]}],"connections":[{"id":"request","from":"client","to":"service","label":"request","variant":"emphasis"}]});
    let source = serde_json::to_vec(&document).unwrap();
    let candidate = temp.0.join("candidate.json");
    fs::write(&candidate, &source).unwrap();
    let options = CompileOptions::default();
    let parsed = parse_input(&source, DiagramType::Architecture, &options).unwrap();
    let scene = layout::compile(&parsed, &options).unwrap();
    let assets = EmbeddedAssets::builtin();
    let baseline = render::render(&scene, &assets).unwrap();
    let check = phoenix_diagram::check::check_html(&baseline, Quality::Standard);
    assert_eq!(
        check.status, "pass",
        "native fixture precondition: {:?}",
        check.diagnostics
    );
    let engine = json!({"sourceInputSha256":env!("PHOENIX_DIAGRAM_SOURCE_INPUT_SHA256"),"inputContractSchema":1,"receiptSchema":1,"viewerRevision":"phoenix-v1","binary":FileIdentity::of(&fs::read(std::env::current_exe().unwrap()).unwrap())});
    let inspect = |name: &str, bytes: &[u8]| {
        let path = temp.0.join(format!("{name}.html"));
        fs::write(&path, bytes).unwrap();
        let artifact = BoundArtifact {
            path,
            candidate: BoundFile {
                path: candidate.clone(),
                identity: FileIdentity::of(&source),
            },
            artifact: FileIdentity::of(bytes),
            config_sha256: parsed.config_sha256.clone(),
            engine: engine.clone(),
            delivery_receipt_id: format!("browser-content-fixture-{name}"),
        };
        let receipt = check_browser(
            &artifact,
            &BrowserOptions {
                executable: Some(executable.clone()),
                ..Default::default()
            },
        );
        // Keep the full owned observation outside TempDir's automatic cleanup;
        // concise test output points to it instead of truncating DOM evidence.
        let evidence_path = temp
            .0
            .with_extension(format!("{name}.browser-evidence.json"));
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&evidence_path)
            .unwrap();
        serde_json::to_writer_pretty(file, &receipt).unwrap();
        eprintln!(
            "real browser {name}: {:?}; evidence={}",
            receipt.status,
            evidence_path.display()
        );
        for coverage in &receipt.coverage {
            for check in &coverage.checks {
                if check.status == phoenix_diagram::browser::CheckStatus::Failed {
                    let evidence = &check.evidence;
                    let concise = match check.name.as_str() {
                        "offline-resources" => {
                            json!({"fontsStatus":evidence["fontsStatus"],"remoteRequests":evidence["remoteRequests"],"loadingFailedCount":evidence["loadingFailed"].as_array().map(Vec::len),"firstLoadingFailed":evidence["loadingFailed"].as_array().and_then(|entries|entries.first())})
                        }
                        "rendered-dom" => {
                            json!({"semanticVisibility":{"ok":evidence["semanticVisibility"]["ok"],"initialDetailLevel":evidence["semanticVisibility"]["initialDetailLevel"],"inspectionDetailLevel":evidence["semanticVisibility"]["inspectionDetailLevel"],"failures":evidence["semanticVisibility"]["failures"].as_array().map(|entries|entries.iter().take(3).collect::<Vec<_>>())}})
                        }
                        _ => evidence.clone(),
                    };
                    eprintln!(
                        "  {}x{} {:?} {}: {}",
                        coverage.viewport.width,
                        coverage.viewport.height,
                        coverage.theme,
                        check.name,
                        concise
                    );
                }
            }
        }
        receipt
    };
    let complete = inspect("baseline", &baseline);
    assert_eq!(
        complete.status,
        BrowserStatus::Pass,
        "real Reader baseline must pass before evaluating CSS counterexamples: {:?}",
        complete
            .diagnostics
            .iter()
            .map(|entry| (&entry.code, &entry.message))
            .collect::<Vec<_>>()
    );
    assert!(validate_receipt_coverage(&complete, false));
    for coverage in &complete.coverage {
        let visibility = &coverage.metrics["semanticVisibility"];
        assert_eq!(visibility["ok"], true);
        assert_eq!(visibility["initial"]["nodes"].as_array().unwrap().len(), 2);
        assert_eq!(visibility["initial"]["edges"].as_array().unwrap().len(), 1);
        assert_eq!(visibility["inspectionDetailLevel"], "full");
        assert_eq!(
            visibility["inspection"]["context"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        let fine = visibility["inspection"]["fine"].as_array().unwrap();
        assert_eq!(fine.len(), 1);
        assert_eq!(fine[0]["text"], "api-v1");
        assert_eq!(fine[0]["visible"], true);
        assert_eq!(visibility["initial"]["fine"][0]["visible"], false);
        assert_eq!(visibility["restoration"]["ok"], true);
    }
    let html = String::from_utf8(baseline).unwrap();
    let map = html.replacen("</body>", "<script>window.addEventListener('load',function(){window.PhoenixDiagram.view.zoomOut();});</script></body>", 1);
    let map_receipt = inspect("initial-map", map.as_bytes());
    assert_eq!(map_receipt.status, BrowserStatus::Pass);
    assert!(validate_receipt_coverage(&map_receipt, false));
    for coverage in &map_receipt.coverage {
        let visibility = &coverage.metrics["semanticVisibility"];
        assert_eq!(visibility["initialDetailLevel"], "map");
        for kind in ["context", "fine"] {
            assert!(visibility["initial"][kind]
                .as_array()
                .unwrap()
                .iter()
                .all(|entry| entry["visible"] == false));
            assert!(visibility["inspection"][kind]
                .as_array()
                .unwrap()
                .iter()
                .all(|entry| entry["visible"] == true));
        }
        assert_eq!(visibility["inspectionDetailLevel"], "full");
        assert_eq!(visibility["restoration"]["ok"], true);
    }
    let mut rejected = Vec::new();
    let mut counterexamples: Vec<_> = [
        (
            "hidden-node",
            r#"[data-node-id="client"]{opacity:0!important}"#,
        ),
        (
            "hidden-path",
            r#"[data-graph-role="edge"]{opacity:0!important}"#,
        ),
        (
            "hidden-label",
            r#"g[data-detail="context"][data-edge-key="0"]>text{opacity:0!important}"#,
        ),
    ]
    .into_iter()
    .map(|(name, css)| {
        (
            name,
            html.replacen("</head>", &format!("<style>{css}</style></head>"), 1),
        )
    })
    .collect();
    counterexamples.extend([
        (
            "missing-node-identity",
            html.replacen(" data-node-id=\"client\"", "", 1),
        ),
        (
            "missing-edge-identity",
            html.replacen("data-edge-from=\"client\"", "", 1),
        ),
    ]);
    for (name, mutant) in counterexamples {
        let receipt = inspect(name, mutant.as_bytes());
        let rendered_dom_rejected = receipt.coverage.len() == 10
            && receipt.coverage.iter().all(|coverage| {
                coverage.checks.iter().any(|check| {
                    check.name == "rendered-dom"
                        && check.status == phoenix_diagram::browser::CheckStatus::Failed
                })
            });
        rejected.push((name, receipt.status, rendered_dom_rejected));
    }
    for (name, status, rendered_dom_rejected) in &rejected {
        eprintln!(
            "content counterexample {name}: {status:?}, rendered-dom rejected={rendered_dom_rejected}"
        );
    }
    for (name, status, rendered_dom_rejected) in rejected {
        assert!(
            status == BrowserStatus::Failed && rendered_dom_rejected,
            "necessary content must stay visible and bound to its identity ({name}); status={status:?}, rendered-dom rejected={rendered_dom_rejected}"
        );
    }
}

/// Real Reader geometry: neither a migrated legend nor normal-flow notes may
/// disappear from the measurement set when the SVG occupies a smaller panel.
#[test]
#[ignore = "real Chrome and native Reader acceptance run only at the core stage"]
fn real_dom_reader_legend_and_normal_flow_notes_are_measured_without_clip_exemptions() {
    use phoenix_diagram::{
        input::{parse_input, CompileOptions, DiagramType, Quality},
        layout,
        render::{self, EmbeddedAssets},
    };
    use std::os::unix::fs::OpenOptionsExt;
    let executable = phoenix_diagram::browser::discover_browser()
        .expect("the core stage requires installed Chrome");
    let temp = TempDir::new();
    let notes = vec!["These authored notes follow the diagram in normal document flow."; 5];
    let document = json!({"schema_version":1,"diagram_type":"architecture","meta":{"title":"Reader geometry fixture","output":"diagram.html","input_contract":"phoenix-v1","quality_profile":"standard"},"components":[{"id":"client","type":"frontend","label":"Client","pos":[60,100],"size":[160,80]},{"id":"service","type":"backend","label":"Service","pos":[360,100],"size":[160,80]}],"connections":[{"id":"request","from":"client","to":"service","label":"request"}],"cards":[{"dot":"cyan","title":"First notes","items":notes},{"dot":"emerald","title":"Second notes","items":notes},{"dot":"rose","title":"Third notes","items":notes}]});
    let source = serde_json::to_vec(&document).unwrap();
    let candidate = temp.0.join("candidate.json");
    fs::write(&candidate, &source).unwrap();
    let options = CompileOptions::default();
    let parsed = parse_input(&source, DiagramType::Architecture, &options).unwrap();
    let scene = layout::compile(&parsed, &options).unwrap();
    let baseline = render::render(&scene, &EmbeddedAssets::builtin()).unwrap();
    assert_eq!(
        phoenix_diagram::check::check_html(&baseline, Quality::Standard).status,
        "pass"
    );
    let engine = json!({"sourceInputSha256":env!("PHOENIX_DIAGRAM_SOURCE_INPUT_SHA256"),"inputContractSchema":1,"receiptSchema":1,"viewerRevision":"phoenix-v1","binary":FileIdentity::of(&fs::read(std::env::current_exe().unwrap()).unwrap())});
    let inspect = |name: &str, bytes: &[u8]| {
        let path = temp.0.join(format!("{name}.html"));
        fs::write(&path, bytes).unwrap();
        let receipt = check_browser(
            &BoundArtifact {
                path,
                candidate: BoundFile {
                    path: candidate.clone(),
                    identity: FileIdentity::of(&source),
                },
                artifact: FileIdentity::of(bytes),
                config_sha256: parsed.config_sha256.clone(),
                engine: engine.clone(),
                delivery_receipt_id: format!("browser-reader-geometry-{name}"),
            },
            &BrowserOptions {
                executable: Some(executable.clone()),
                ..Default::default()
            },
        );
        let evidence = temp
            .0
            .with_extension(format!("{name}.browser-evidence.json"));
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&evidence)
            .unwrap();
        serde_json::to_writer_pretty(file, &receipt).unwrap();
        eprintln!(
            "real Reader {name}: {:?}; evidence={}",
            receipt.status,
            evidence.display()
        );
        receipt
    };
    let complete = inspect("geometry-baseline", &baseline);
    assert_eq!(complete.status, BrowserStatus::Pass);
    assert!(validate_receipt_coverage(&complete, false));
    for coverage in &complete.coverage {
        assert_eq!(coverage.metrics["readerFlow"]["unclipped"], true);
        if coverage.viewport.width >= 960 {
            let checks = coverage.metrics["canvasTextChecks"].as_array().unwrap();
            let legend: Vec<_> = checks
                .iter()
                .filter(|entry| entry["container"] == "reader-panel")
                .collect();
            assert_eq!(
                legend.len(),
                5,
                "title, both kinds and both runtime counts stay checked"
            );
            assert!(legend
                .iter()
                .all(|entry| entry["failures"].as_array().unwrap().is_empty()));
        } else {
            assert!(coverage.metrics["fullScrollHeight"].as_f64().unwrap() > 844.0);
            assert_eq!(coverage.metrics["scrollHeight"], 844);
            assert_eq!(coverage.metrics["readerFlow"]["notesBelowDiagram"], true);
            assert!(
                !coverage.vertical_scroll_accepted,
                "notes do not create a diagram scroll exception"
            );
        }
    }
    let html = String::from_utf8(baseline).unwrap();
    for (name, css, check_name, desktop_only) in [
        (
            "legend-outside-panel",
            "[data-reader-legend-corner]{transform:translateY(200vh)!important}",
            "text-containment",
            true,
        ),
        (
            "legend-svg-ancestor-clip",
            ".diagram-container>svg{overflow:hidden!important}",
            "text-containment",
            true,
        ),
        (
            "legend-count-hidden",
            "[data-legend-count-badge] text{opacity:0!important}",
            "text-containment",
            false,
        ),
        (
            "notes-document-clip",
            "html,body{overflow-y:hidden!important;height:100%!important}",
            "viewport-containment",
            false,
        ),
    ] {
        let mutant = html.replacen("</head>", &format!("<style>{css}</style></head>"), 1);
        let receipt = inspect(name, mutant.as_bytes());
        assert_eq!(receipt.status, BrowserStatus::Failed, "{name}");
        assert_eq!(receipt.coverage.len(), 10, "{name}");
        let mut checked_scopes = 0;
        for coverage in &receipt.coverage {
            if desktop_only && coverage.viewport.width < 960 {
                continue;
            }
            // This notes counterexample is the 390x844 boundary only. A larger
            // viewport can still contain all notes despite disabled scrolling.
            if name == "notes-document-clip" {
                if coverage.viewport.width != 390 || coverage.viewport.height != 844 {
                    continue;
                }
                assert!(coverage.metrics["readerFlow"]["notes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|note| note["bounds"]["bottom"].as_f64().unwrap() > 844.0));
                assert_eq!(coverage.metrics["readerFlow"]["unclipped"], false);
            }
            checked_scopes += 1;
            assert!(
                coverage.checks.iter().any(|check| check.name == check_name
                    && check.status == phoenix_diagram::browser::CheckStatus::Failed),
                "{name} {:?}: {check_name}",
                coverage.viewport
            );
        }
        if name == "notes-document-clip" {
            assert_eq!(checked_scopes, 2, "390x844 in both themes");
        }
    }
}
