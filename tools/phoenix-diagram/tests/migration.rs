//! Public migration entry safety gate; filesystem/fault matrix shares install fixtures.
use std::{fs, process::Command};

#[test]
fn unconfirmed_migration_is_explicit_and_does_not_create_state() {
    let project = std::env::temp_dir().join(format!(
        "phoenix-diagram-migration-confirm-{}",
        std::process::id()
    ));
    fs::create_dir(&project).unwrap();
    let result = Command::new(
        std::env::var_os("PHOENIX_DIAGRAM_BIN")
            .unwrap_or_else(|| env!("CARGO_BIN_EXE_phoenix-diagram").into()),
    )
    .args(["migrate", "--from-archify", "--project"])
    .arg(&project)
    .args(["--bundle-root", "/missing-bundle", "--json"])
    .output()
    .unwrap();
    let receipt: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(result.status.code(), Some(3), "{receipt}");
    assert!(receipt
        .to_string()
        .contains("install/confirmation-required"));
    assert_eq!(fs::read_dir(&project).unwrap().count(), 0);
    fs::remove_dir(&project).unwrap();
}
