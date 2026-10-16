use super::*;
use crate::check::FileIdentity;
use crate::input::sha256;
use std::fs;

struct Cleanup(std::path::PathBuf);
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
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
        viewer_revision: "phoenix-v1".into(),
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
fn strict_rejects_a_real_unfinished_delivery_receipt_publication() {
    use crate::output::{self, FileBinding};
    let fixture = super::test_support::Fixture::new();
    let delivered = deliver(&fixture.request(), &fixture.output);
    assert_eq!(
        delivered.status,
        Status::Pass,
        "{:?}",
        delivered.diagnostics
    );
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
        output::test_support::publish(&sidecar, &bytes, &[&source, &artifact], &mut |phase, _| {
            if phase == output::test_support::Phase::BeforeBackupCleanup {
                Err(std::io::Error::other("injected receipt cleanup failure"))
            } else {
                Ok(())
            }
        })
        .unwrap()
    };
    assert!(!publication.cleanup_complete);
    assert!(publication.recovery_directory.unwrap().exists());
    let checked = check(&fixture.output, true, &fixture.engine);
    assert_eq!(
        checked.status,
        Status::Failed,
        "A valid-looking receipt cannot prove current delivery while its publication is unfinished"
    );
    assert_eq!(checked.exit_code(), 3);
    assert!(checked
        .diagnostics
        .iter()
        .any(|d| d.code == "provenance/unfinished-publication"));
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
