//! Public migration entry safety gate; filesystem/fault matrix shares install fixtures.
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt},
    path::PathBuf,
    process::Command,
};

#[test]
fn unconfirmed_migration_is_explicit_and_does_not_create_state() {
    let project = std::env::temp_dir().join(format!(
        "phoenix-diagram-migration-confirm-{}",
        std::process::id()
    ));
    fs::create_dir(&project).unwrap();
    let (executable, engine_root) = match std::env::var_os("PHOENIX_DIAGRAM_BIN") {
        Some(path) => (PathBuf::from(path), None),
        None => {
            let root = project.with_extension("engine");
            fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
            let executable = root.join("phoenix-diagram");
            // Copy in a child so a parallel fork cannot retain a write fd.
            assert!(Command::new("/bin/cp")
                .arg(env!("CARGO_BIN_EXE_phoenix-diagram"))
                .arg(&executable)
                .status()
                .unwrap()
                .success());
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
            assert_eq!(fs::symlink_metadata(&executable).unwrap().nlink(), 1);
            (executable, Some(root))
        }
    };
    let result = Command::new(executable)
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
    if let Some(root) = engine_root {
        fs::remove_dir_all(root).unwrap();
    }
}
