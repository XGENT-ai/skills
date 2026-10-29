// Public foreground CLI + real loopback sockets; these checks do not open a browser.
#[path = "support/preview_fixture.rs"]
mod preview_fixture;
use preview_fixture::{source, Session, Workspace};
use serde_json::Value;
use std::{fs, time::Duration};

fn published(session: &Session, event: &Value, output: &std::path::Path, label: &str) -> Vec<u8> {
    let generation = Session::generation(event);
    assert_eq!(event["lastSuccessfulGeneration"], generation, "{event}");
    let bytes = fs::read(output).unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains(label));
    let response = session.artifact();
    assert_eq!(response.status, 200, "{response:?}");
    response.safe_headers();
    assert_eq!(
        response.body, bytes,
        "HTTP must serve exactly the committed output bytes"
    );
    let state = session.status();
    assert_eq!(state["lastSuccessfulGeneration"], generation, "{state}");
    assert_eq!(state["status"], "ready", "{state}");
    assert_eq!(state["lastAttemptStatus"], "pass", "{state}");
    assert_eq!(state["artifactAvailable"], true, "{state}");
    assert_eq!(state["artifact"]["generation"], generation, "{state}");
    assert_eq!(
        state["artifact"]["sha256"],
        phoenix_diagram::input::sha256(&bytes),
        "{state}"
    );
    assert_eq!(
        state["artifact"]["inputSha256"],
        phoenix_diagram::input::sha256(&source(label)),
        "{state}"
    );
    assert_eq!(state["artifact"]["bytes"], bytes.len(), "{state}");
    let text = String::from_utf8_lossy(&bytes);
    for forbidden in [
        session.url.as_str(),
        session.url.path(),
        event["sessionId"].as_str().unwrap(),
    ] {
        assert!(
            !text.contains(forbidden),
            "session detail leaked into formal HTML: {forbidden}"
        );
    }
    bytes
}
fn no_visual_evidence(root: &Workspace) {
    for entry in fs::read_dir(&root.0).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        assert!(
            ![
                ".browser-check.json",
                ".visual-check.json",
                ".visual-review.json"
            ]
            .iter()
            .any(|suffix| name.ends_with(suffix)),
            "Preview invented visual evidence: {name}"
        );
    }
}

#[test]
fn invalid_first_input_waits_then_edits_keep_last_good_and_repair_advances() {
    let root = Workspace::new();
    let input = root.path("input.json");
    let output = root.path("preview.html");
    fs::write(&input, b"{").unwrap();
    let mut session = Session::start(&root, &input, &output);
    let failed = session.next("failed", 0);
    assert!(
        failed["diagnostics"]
            .as_array()
            .is_some_and(|diagnostics| !diagnostics.is_empty()),
        "{failed}"
    );
    assert_eq!(failed["lastSuccessfulGeneration"], 0);
    assert!(
        !output.exists(),
        "invalid first source must not create an empty success"
    );
    let state = session.status();
    assert_eq!(state["lastSuccessfulGeneration"], 0, "{state}");
    assert!(
        matches!(state["status"].as_str(), Some("waiting" | "error")),
        "{state}"
    );
    assert_eq!(state["artifactAvailable"], false, "{state}");
    assert!(state["artifact"].is_null(), "{state}");
    assert_eq!(session.artifact().status, 404);
    let shell = session.request("GET", session.url.path(), &[]);
    assert_eq!(shell.status, 200);
    shell.safe_headers();
    assert!(String::from_utf8_lossy(&shell.body)
        .to_ascii_lowercase()
        .contains("waiting"));
    root.replace(&input, &source("FirstStable"));
    let first = session.next("updated", Session::generation(&failed));
    let first_bytes = published(&session, &first, &output, "FirstStable");
    let first_generation = Session::generation(&first);
    assert!(!String::from_utf8_lossy(&first_bytes).contains(input.to_str().unwrap()));
    // More than one native 800ms poll plus the 400ms stability window: absence
    // is observed through live status and event reads, rather than one sleep.
    fs::write(&input, source("FirstStable")).unwrap();
    session.unchanged_for(
        first_generation,
        first_generation,
        Duration::from_millis(2200),
    );
    assert_eq!(fs::read(&output).unwrap(), first_bytes);
    fs::write(&input, b"{\"schema_version\":").unwrap();
    let partial = session.next("failed", first_generation);
    assert_eq!(partial["lastSuccessfulGeneration"], first_generation);
    assert_eq!(fs::read(&output).unwrap(), first_bytes);
    assert_eq!(session.artifact().body, first_bytes);
    fs::remove_file(&input).unwrap();
    let deleted = session.next("failed", Session::generation(&partial));
    assert_eq!(deleted["lastSuccessfulGeneration"], first_generation);
    assert_eq!(fs::read(&output).unwrap(), first_bytes);
    assert_eq!(session.artifact().body, first_bytes);
    root.replace(&input, &source("RecoveredStable"));
    let fixed = session.next("updated", Session::generation(&deleted));
    let final_bytes = published(&session, &fixed, &output, "RecoveredStable");
    assert_ne!(final_bytes, first_bytes);
    session.stop(libc::SIGINT, 0);
    assert_eq!(fs::read(&output).unwrap(), final_bytes);
    no_visual_evidence(&root);
}

#[test]
fn successive_real_source_replacements_eventually_publish_only_the_latest_stable_bytes() {
    let root = Workspace::new();
    let input = root.path("input.json");
    let output = root.path("preview.html");
    fs::write(&input, source("InitialStable")).unwrap();
    let mut session = Session::start(&root, &input, &output);
    let initial = session.next("updated", 0);
    published(&session, &initial, &output, "InitialStable");
    root.replace(&input, &source("IntermediateStable"));
    let checking = session.next("checking", Session::generation(&initial));
    for index in 0..20 {
        root.replace(&input, &source(&format!("TransientSource{index}")));
    }
    root.replace(&input, &source("NewestStable"));
    let newest = session.next("updated", Session::generation(&checking));
    let bytes = published(&session, &newest, &output, "NewestStable");
    assert!(!String::from_utf8_lossy(&bytes).contains("IntermediateStable"));
    // This proves the public final state under real churn. The deterministic
    // old-worker completion order is covered by the controller's unit seam.
    let generation = Session::generation(&newest);
    session.unchanged_for(generation, generation, Duration::from_millis(1600));
    assert_eq!(fs::read(&output).unwrap(), bytes);
    session.stop(libc::SIGTERM, 0);
    assert_eq!(fs::read(&output).unwrap(), bytes);
}

#[test]
fn a_later_output_claimant_is_preserved_and_cannot_be_served_as_old_success() {
    let root = Workspace::new();
    let input = root.path("input.json");
    let output = root.path("preview.html");
    fs::write(&input, source("OwnedFirst")).unwrap();
    let mut session = Session::start(&root, &input, &output);
    let initial = session.next("updated", 0);
    let original = published(&session, &initial, &output, "OwnedFirst");
    let foreign = b"later claimant must survive\n";
    root.replace(&output, foreign);
    root.replace(&input, &source("MustNotOverwriteClaimant"));
    let failed = session.next("failed", 0);
    assert_eq!(
        failed["lastSuccessfulGeneration"], initial["generation"],
        "{failed}"
    );
    assert_eq!(fs::read(&output).unwrap(), foreign);
    let artifact = session.artifact();
    assert_eq!(
        artifact.status, 409,
        "Replaced output must invalidate HTTP artifact: {artifact:?}"
    );
    assert_ne!(artifact.body, original);
    root.replace(&input, &source("StillMustNotOverwrite"));
    let until = std::time::Instant::now() + Duration::from_millis(1600);
    while std::time::Instant::now() < until {
        assert_eq!(fs::read(&output).unwrap(), foreign);
        assert_eq!(session.artifact().status, 409);
        std::thread::sleep(Duration::from_millis(50));
    }
    session.stop(libc::SIGTERM, 3);
    assert_eq!(fs::read(&output).unwrap(), foreign);
}

#[test]
fn loopback_routes_reject_foreign_hosts_origins_paths_and_mutations() {
    let root = Workspace::new();
    let input = root.path("input.json");
    let output = root.path("preview.html");
    fs::write(&input, source("HttpStable")).unwrap();
    let secret = root.path("secret.txt");
    fs::write(&secret, b"private unrelated file").unwrap();
    let mut session = Session::start(&root, &input, &output);
    let updated = session.next("updated", 0);
    let bytes = published(&session, &updated, &output, "HttpStable");
    let status_path = session.route("status");
    let artifact_path = session.route("artifact");
    let status = session.request("GET", &status_path, &[]);
    assert_eq!(status.status, 200);
    status.safe_headers();
    for path in [
        session.url.path(),
        status_path.as_str(),
        artifact_path.as_str(),
    ] {
        let head = session.request("HEAD", path, &[]);
        assert_eq!(head.status, 200, "{head:?}");
        assert!(head.body.is_empty());
        head.safe_headers();
    }
    for host in [
        "evil.invalid".to_owned(),
        format!("127.0.0.1:{}", session.url.port().unwrap().wrapping_add(1)),
        format!("localhost:{}", session.url.port().unwrap()),
    ] {
        let response = session.raw(&format!(
            "GET {status_path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"
        ));
        assert_eq!(response.status, 403, "{response:?}");
    }
    let no_host = session.raw(&format!(
        "GET {status_path} HTTP/1.1\r\nConnection: close\r\n\r\n"
    ));
    assert_eq!(no_host.status, 403);
    let duplicate_host = session.raw(&format!(
        "GET {status_path} HTTP/1.1\r\nHost: {}\r\nHost: {}\r\nConnection: close\r\n\r\n",
        session.host(),
        session.host()
    ));
    assert_eq!(duplicate_host.status, 400);
    let oversized = session.raw(&format!(
        "GET {status_path} HTTP/1.1\r\nHost: {}\r\nX-Oversized: {}\r\nConnection: close\r\n\r\n",
        session.host(),
        "x".repeat(9000)
    ));
    assert!(
        oversized.status == 431,
        "8KiB request header cap: {oversized:?}"
    );
    for origin in ["https://evil.invalid", "null"] {
        let response = session.request("GET", &status_path, &[("Origin", origin)]);
        assert_eq!(response.status, 403, "{response:?}");
    }
    let own_origin = format!("http://{}", session.host());
    assert_eq!(
        session
            .request("GET", &status_path, &[("Origin", &own_origin)])
            .status,
        200
    );
    for path in [
        "/".to_owned(),
        "/status".into(),
        "/secret.txt".into(),
        format!("{}../secret.txt", session.url.path()),
        format!("{}%2e%2e/secret.txt", session.url.path()),
        format!("{}artifact/../../secret.txt", session.url.path()),
        format!("{}artifact%2f..%2fsecret.txt", session.url.path()),
    ] {
        let response = session.request("GET", &path, &[]);
        assert!(response.status == 404, "{path}: {response:?}");
        assert!(!response
            .body
            .windows(b"private unrelated file".len())
            .any(|w| w == b"private unrelated file"));
    }
    for method in ["POST", "PUT", "DELETE"] {
        let response = session.request(method, &artifact_path, &[]);
        assert_eq!(response.status, 405, "{response:?}");
    }
    assert_eq!(fs::read(&output).unwrap(), bytes);
    assert_eq!(fs::read(&secret).unwrap(), b"private unrelated file");
    session.stop(libc::SIGINT, 0);
}

#[test]
fn sigterm_closes_even_a_partially_read_http_connection_and_keeps_success() {
    use std::{io::Write, net::TcpStream};
    let root = Workspace::new();
    let input = root.path("input.json");
    let output = root.path("preview.html");
    fs::write(&input, source("StopStable")).unwrap();
    let mut session = Session::start(&root, &input, &output);
    let initial = session.next("updated", 0);
    let bytes = published(&session, &initial, &output, "StopStable");
    let mut unfinished = TcpStream::connect(session.address()).unwrap();
    unfinished
        .set_read_timeout(Some(Duration::from_millis(300)))
        .unwrap();
    unfinished.write_all(b"GET / HTTP/1.1\r\nHost:").unwrap();
    session.stop(libc::SIGTERM, 0);
    assert_eq!(fs::read(&output).unwrap(), bytes);
    let mut buffer = vec![];
    use std::io::Read;
    let result = unfinished.read_to_end(&mut buffer);
    assert!(
        result.is_ok()
            || result
                .as_ref()
                .is_err_and(|error| error.kind() == std::io::ErrorKind::ConnectionReset),
        "A stopped session left a live request connection: {result:?}"
    );
}

#[test]
fn preexisting_output_is_not_adopted_as_this_sessions_first_success() {
    let root = Workspace::new();
    let input = root.path("input.json");
    let output = root.path("preview.html");
    let previous = b"<html><body>existing file without this session's success</body></html>";
    fs::write(&input, b"{").unwrap();
    fs::write(&output, previous).unwrap();
    let mut session = Session::start(&root, &input, &output);
    let failure = session.next("failed", 0);
    assert_eq!(failure["lastSuccessfulGeneration"], 0);
    let state = session.status();
    assert_eq!(state["lastSuccessfulGeneration"], 0);
    assert_eq!(state["artifactAvailable"], false);
    assert!(state["artifact"].is_null());
    assert_eq!(session.artifact().status, 404);
    assert_eq!(fs::read(&output).unwrap(), previous);
    session.stop(libc::SIGTERM, 0);
    assert_eq!(fs::read(&output).unwrap(), previous);
    no_visual_evidence(&root);
}
