use super::*;
use crate::check::FileIdentity;
use crate::input::sha256;
use std::fs;

#[test]
fn private_snapshot_preparation_uses_bound_bytes_without_reopening_the_candidate() {
    let fixture = super::test_support::Fixture::new();
    let bytes = fs::read(&fixture.candidate).unwrap();
    let original = super::prepare::prepare(&fixture.request());
    assert_eq!(
        original.receipt.status,
        Status::Pass,
        "{:?}",
        original.receipt.diagnostics
    );
    fs::remove_file(&fixture.candidate).unwrap();
    let snapshot = super::prepare::prepare_bytes(&fixture.request(), &bytes);
    assert_eq!(
        snapshot.receipt.status,
        Status::Pass,
        "{:?}",
        snapshot.receipt.diagnostics
    );
    assert!(snapshot.source.is_none());
    assert!(snapshot.receipt.candidate.is_none());
    assert_eq!(
        snapshot.receipt.config_sha256,
        original.receipt.config_sha256
    );
    assert_eq!(
        snapshot.compiled.unwrap().html,
        original.compiled.unwrap().html
    );
}

struct Cleanup(std::path::PathBuf);
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct PreExecChild {
    socket: std::os::unix::net::UnixStream,
    thread: Option<std::thread::JoinHandle<std::io::Result<std::process::ExitStatus>>>,
}
impl PreExecChild {
    fn start() -> Self {
        use std::os::fd::AsRawFd;
        use std::os::unix::process::CommandExt;
        let (socket, child_socket) = std::os::unix::net::UnixStream::pair().unwrap();
        socket
            .set_read_timeout(Some(std::time::Duration::from_secs(10)))
            .unwrap();
        socket
            .set_write_timeout(Some(std::time::Duration::from_secs(10)))
            .unwrap();
        let parent_fd = socket.as_raw_fd();
        let child_fd = child_socket.as_raw_fd();
        let thread = std::thread::spawn(move || {
            let mut command = std::process::Command::new("/usr/bin/true");
            // No allocation, logging, or locks after fork. Closing the inherited
            // parent endpoint lets parent cleanup wake this read with EOF.
            unsafe {
                command.pre_exec(move || {
                    let ready = b'R';
                    let mut release = 0_u8;
                    if libc::close(parent_fd) != 0
                        || libc::write(child_fd, (&ready as *const u8).cast(), 1) != 1
                        || libc::read(child_fd, (&mut release as *mut u8).cast(), 1) != 1
                        || release != b'G'
                        || libc::close(child_fd) != 0
                    {
                        return Err(std::io::Error::from_raw_os_error(libc::EIO));
                    }
                    Ok(())
                });
            }
            let status = command.status();
            drop(child_socket);
            status
        });
        Self {
            socket,
            thread: Some(thread),
        }
    }
    fn ready(&mut self) -> std::io::Result<()> {
        use std::io::Read;
        let mut byte = [0_u8];
        self.socket.read_exact(&mut byte)?;
        if byte != *b"R" {
            return Err(std::io::Error::other("Unexpected pre-exec handshake"));
        }
        Ok(())
    }
    fn release(mut self) -> std::io::Result<std::process::ExitStatus> {
        use std::io::Write;
        let released = self.socket.write_all(b"G");
        let _ = self.socket.shutdown(std::net::Shutdown::Write);
        let status = self
            .thread
            .take()
            .unwrap()
            .join()
            .map_err(|_| std::io::Error::other("Pre-exec child thread panicked"));
        released?;
        status?
    }
}
impl Drop for PreExecChild {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = self.socket.shutdown(std::net::Shutdown::Both);
            let _ = thread.join();
        }
    }
}

#[test]
fn an_original_strict_failure_is_not_replaced_by_a_later_successful_reread() {
    let directory =
        std::env::temp_dir().join(format!("phoenix-original-failure-{}", std::process::id()));
    fs::create_dir(&directory).unwrap();
    let _cleanup = Cleanup(directory.clone());
    let candidate = directory.join("candidate.json");
    let output = directory.join("diagram.html");
    fs::write(&candidate,br#"{"schema_version":1,"diagram_type":"architecture","meta":{"title":"Strict failure","output":"diagram.html","input_contract":"phoenix-v1","legend":{"mode":"hidden"}},"components":[{"id":"service","type":"backend","label":"Service","pos":[120,120]}],"connections":[]}"#).unwrap();
    let options = CompileOptions::default();
    let engine = EngineIdentity {
        source_input_sha256: sha256(b"source test fixture"),
        input_contract_schema: 1,
        receipt_schema: 1,
        viewer_revision: "phoenix-v2".into(),
        binary: FileIdentity::of(&fs::read(std::env::current_exe().unwrap()).unwrap()),
    };
    let assets = EmbeddedAssets {
        template: include_str!("../../assets/template.html"),
    };
    let delivered = deliver(
        &CompileRequest {
            kind: DiagramType::Architecture,
            candidate: &candidate,
            options: &options,
            engine: &engine,
            assets: &assets,
            repo_root: None,
            brand_cache: None,
        },
        &output,
    );
    assert_eq!(
        delivered.status,
        Status::Pass,
        "{:?}",
        delivered.diagnostics
    );
    let absent = directory.join("candidate-away.json");
    fs::rename(&candidate, &absent).unwrap();
    let failed = bound_artifact_observed(&output, &engine, || {
        fs::rename(&absent, &candidate).unwrap()
    })
    .unwrap_err();
    assert!(std::mem::size_of_val(&failed) <= 128);
    assert_eq!(
        failed.status,
        Status::Failed,
        "The original failed gate must survive a later restored input"
    );
    assert!(failed
        .diagnostics
        .iter()
        .any(|d| d.code == "provenance/candidate-binding"));
    assert_eq!(
        check(&output, true, &engine).status,
        Status::Pass,
        "A separate command may verify the restored input"
    );
}

#[test]
fn small_failure_handles_preserve_the_failed_check_json_contract() {
    // Wire-format fixture only; it does not claim an executed native gate.
    let engine = EngineIdentity {
        source_input_sha256: "a".repeat(64),
        input_contract_schema: 1,
        receipt_schema: 1,
        viewer_revision: "phoenix-v2".into(),
        binary: FileIdentity {
            sha256: "b".repeat(64),
            bytes: 7,
        },
    };
    assert!(std::mem::size_of::<support::Result<()>>() <= 128);
    let failure = support::fail("check", "fixture/failure", "Fixture failure", 4);
    let receipt = provenance::failed(&engine, true, failure);
    assert_eq!(receipt.exit_code(), 4);
    assert_eq!(
        serde_json::to_value(receipt).unwrap(),
        serde_json::json!({
            "schemaVersion":1,"tool":"phoenix-diagram","toolVersion":crate::TOOL_VERSION,
            "command":"check","status":"failed",
            "diagnostics":[{
                "stage":"check","code":"fixture/failure","severity":"error",
                "message":"Fixture failure","subject":{"diagramType":null,"path":""},
                "evidence":null,"supportedFixes":[]
            }],
            "provenance":"unknown","scope":"strict-provenance",
            "candidate":null,"artifact":null,"configSha256":null,
            "engine":{
                "sourceInputSha256":"a".repeat(64),"inputContractSchema":1,
                "receiptSchema":1,"viewerRevision":"phoenix-v2",
                "binary":{"sha256":"b".repeat(64),"bytes":7}
            },
            "deliveryReceiptId":null,"deliveryReceipt":null,"checks":[],"composition":null
        })
    );
}

#[test]
fn strict_rejects_a_real_unfinished_delivery_receipt_publication() {
    unfinished_receipt_publication(false);
}

#[test]
fn strict_rejects_unfinished_receipt_while_an_unrelated_child_is_pre_exec() {
    // Real process/lock protocol regression, not provider or browser acceptance.
    unfinished_receipt_publication(true);
}

fn unfinished_receipt_publication(inherit_lock: bool) {
    use crate::output::{self, FileBinding};
    let fixture = super::test_support::Fixture::new();
    let delivered = deliver(&fixture.request(), &fixture.output);
    assert_eq!(
        delivered.status,
        Status::Pass,
        "{:?}",
        delivered.diagnostics
    );
    let mut child = None;
    let mut ready = Ok(());
    let publication = {
        let source = FileBinding::capture(&fixture.candidate).unwrap();
        let artifact = FileBinding::capture(&fixture.output).unwrap();
        let slot = output::capture_slot(&fixture.output, &[&source]).unwrap();
        let sidecar = slot
            .capture_related(
                &sidecar_paths(&fixture.output, None).delivery,
                &[&source, &artifact],
            )
            .unwrap();
        let bytes = sidecar.previous().unwrap().bytes().to_vec();
        let publication = output::test_support::publish(
            &sidecar,
            &bytes,
            &[&source, &artifact],
            &mut |phase, _| {
                if phase == output::test_support::Phase::BeforeBackupCleanup {
                    Err(std::io::Error::other("injected receipt cleanup failure"))
                } else {
                    Ok(())
                }
            },
        )
        .unwrap();
        if inherit_lock {
            let mut held = PreExecChild::start();
            ready = held.ready();
            child = Some(held);
        }
        publication
    };
    // The child still holds its inherited lock descriptor before exec, while
    // all parent slot guards from the publication scope have been dropped.
    let checked = check(&fixture.output, true, &fixture.engine);
    let child_status = child.map(PreExecChild::release);
    assert!(ready.is_ok(), "Pre-exec child readiness: {ready:?}");
    if let Some(status) = child_status {
        assert!(status.unwrap().success());
    }
    assert!(!publication.cleanup_complete);
    assert!(publication.recovery_directory.unwrap().exists());
    assert_eq!(
        checked.status,
        Status::Failed,
        "A valid-looking receipt cannot prove current delivery while its publication is unfinished"
    );
    assert_eq!(checked.exit_code(), 3);
    assert!(
        checked
            .diagnostics
            .iter()
            .any(|d| d.code == "provenance/unfinished-publication"),
        "Actual strict diagnostics: {:?}",
        checked.diagnostics
    );
}

#[test]
fn strict_preserves_an_unknown_recovery_namespace_claimant() {
    let fixture = super::test_support::Fixture::new();
    assert_eq!(
        deliver(&fixture.request(), &fixture.output).status,
        Status::Pass
    );
    let unknown = fixture
        .root
        .join(format!(".phoenix-diagram-recovery-{}", "f".repeat(32)));
    fs::write(&unknown, b"foreign claimant").unwrap();
    let html = fs::read(&fixture.output).unwrap();
    let checked = check(&fixture.output, true, &fixture.engine);
    assert_eq!(checked.status, Status::Failed);
    assert_eq!(checked.exit_code(), 3);
    assert_eq!(fs::read(&unknown).unwrap(), b"foreign claimant");
    assert_eq!(fs::read(&fixture.output).unwrap(), html);
}
