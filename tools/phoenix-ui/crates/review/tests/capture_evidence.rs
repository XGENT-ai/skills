use phoenix_ui_review::{
    capture_evidence::*,
    matrix::{freeze, Identity, State, Unit, Viewport},
    storage::StoreError,
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn id(value: &str) -> Identity {
    Identity {
        id: value.into(),
        label: value.into(),
    }
}
struct Fixture {
    root: PathBuf,
    matrix: phoenix_ui_review::matrix::FrozenMatrix,
    version: TestedVersion,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "phoenix-capture-evidence-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let matrix = freeze(
            vec![Unit {
                identity: id("U1"),
                states: vec![State {
                    identity: id("empty"),
                    steps: vec![id("initial")],
                }],
                viewports: vec![Viewport {
                    id: "mobile".into(),
                    width: 2,
                    height: 3,
                    dpr: 2.0,
                    full_page: false,
                }],
                themes: vec![id("light")],
            }],
            None,
        )
        .unwrap();
        Self {
            root,
            matrix,
            version: TestedVersion {
                head: Some("a".repeat(40)),
                working_tree: WorkingTree::Dirty {
                    sha256: "b".repeat(64),
                },
                build_id: None,
            },
        }
    }
    fn context(&self) -> CaptureContext<'_> {
        CaptureContext {
            project: &self.root,
            round_directory: Path::new("review.ui-captures/r2"),
            round_id: "r2",
            matrix: &self.matrix,
            version: &self.version,
        }
    }
    fn image(&self, attempt: u32, width: u32, height: u32) -> FileEvidence {
        let path = format!(
            "review.ui-captures/r2/{}",
            self.matrix
                .screenshot_name(&self.matrix.cells[0].key, attempt)
                .unwrap()
        );
        let file = self.root.join(&path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, width, height);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer
                .write_image_data(&vec![127; (width * height * 3) as usize])
                .unwrap();
            writer.finish().unwrap();
        }
        fs::write(file, &bytes).unwrap();
        FileEvidence {
            path,
            sha256: digest(&bytes),
        }
    }
    fn captured(&self, attempt: u32, width: u32, height: u32) -> CaptureInput {
        CaptureInput {
            schema_version: 1,
            round_id: "r2".into(),
            cell: self.matrix.cells[0].key.clone(),
            attempt,
            observation: Observation::Captured {
                image: self.image(attempt, width, height),
                origin: Origin::Browser,
                tool: "real capture fixture".into(),
                address: "http://localhost:3000/orders".into(),
                taken_at: "2026-10-10T11:00:00Z".into(),
                viewport: Box::new(self.matrix.cells[0].viewport.clone()),
                tested_version: Box::new(self.version.clone()),
                interactions: vec![],
                human_check: HumanCheck {
                    opened: true,
                    current_product: true,
                    current_version: true,
                    notes: "Library fixture, not product acceptance".into(),
                },
            },
        }
    }
    fn missing(&self) -> CaptureInput {
        CaptureInput {
            schema_version: 1,
            round_id: "r2".into(),
            cell: self.matrix.cells[0].key.clone(),
            attempt: 1,
            observation: Observation::Missing {
                reason: "browser unavailable".into(),
            },
        }
    }
    fn verify(&self, input: CaptureInput) -> Result<VerifiedCapture, StoreError> {
        verify(input, &self.context())
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn invalid(result: Result<VerifiedCapture, StoreError>) {
    assert_eq!(result.unwrap_err().code, 2);
}

#[test]
fn png_dimensions_and_dpr_are_read_from_the_actual_file() {
    let f = Fixture::new();
    let input = f.captured(1, 4, 6);
    let result = f.verify(input.clone()).unwrap();
    assert_eq!(result.input, input);
    assert_eq!(result.image_pixels, Some([4, 6]));
    assert!(result.version_known);
    assert_eq!(
        result.content_hash,
        digest(&serde_json::to_vec(&input).unwrap())
    );
}
#[test]
fn missing_records_need_a_reason_and_do_not_invent_an_image() {
    let f = Fixture::new();
    let result = f.verify(f.missing()).unwrap();
    assert_eq!(result.image_pixels, None);
    let mut input = f.missing();
    input.observation = Observation::Missing { reason: " ".into() };
    invalid(f.verify(input));
}
#[test]
fn old_round_paths_and_unallocated_attempt_names_are_rejected() {
    let f = Fixture::new();
    for path in [
        "review.ui-captures/r1/U1-empty-2-light.png",
        "review.ui-captures/r2/other.png",
        "../outside.png",
        "/tmp/outside.png",
    ] {
        let mut input = f.captured(1, 4, 6);
        if let Observation::Captured { image, .. } = &mut input.observation {
            image.path = path.into();
        }
        invalid(f.verify(input));
    }
    let mut input = f.captured(1, 4, 6);
    input.attempt = 2;
    invalid(f.verify(input));
}
#[test]
fn recapture_names_are_new_files_even_when_the_pixels_are_identical() {
    let f = Fixture::new();
    let first = f.verify(f.captured(1, 4, 6)).unwrap();
    let second = f.verify(f.captured(2, 4, 6)).unwrap();
    assert_eq!(first.image_pixels, second.image_pixels);
    assert_ne!(first.content_hash, second.content_hash);
    if let (Observation::Captured { image: a, .. }, Observation::Captured { image: b, .. }) =
        (first.input.observation, second.input.observation)
    {
        assert_eq!(a.sha256, b.sha256);
        assert_ne!(a.path, b.path);
    } else {
        panic!("expected captures")
    }
}
#[test]
fn input_identity_and_schema_cannot_select_a_different_frozen_round() {
    let f = Fixture::new();
    for field in 0..4 {
        let mut input = f.missing();
        match field {
            0 => input.round_id = "r1".into(),
            1 => input.schema_version = 2,
            2 => input.cell.unit_id = "U2".into(),
            _ => input.attempt = 0,
        };
        invalid(f.verify(input));
    }
}
#[test]
fn input_cannot_change_viewport_binding_or_the_frozen_build() {
    let f = Fixture::new();
    for field in 0..3 {
        let mut input = f.captured(1, 4, 6);
        if let Observation::Captured {
            viewport,
            tested_version,
            ..
        } = &mut input.observation
        {
            match field {
                0 => viewport.dpr = 1.0,
                1 => viewport.height = 6,
                _ => tested_version.head = Some("c".repeat(40)),
            };
        }
        invalid(f.verify(input));
    }
}
#[test]
fn incorrect_hashes_or_changed_registered_files_are_rejected() {
    let f = Fixture::new();
    let mut input = f.captured(1, 4, 6);
    if let Observation::Captured { image, .. } = &mut input.observation {
        image.sha256 = "e".repeat(64);
    }
    invalid(f.verify(input));
    let input = f.captured(1, 4, 6);
    f.verify(input.clone()).unwrap();
    if let Observation::Captured { image, .. } = &input.observation {
        fs::write(f.root.join(&image.path), b"changed").unwrap();
    }
    invalid(f.verify(input));
}
#[test]
fn a_header_without_valid_pixels_is_not_a_capture() {
    let f = Fixture::new();
    let mut input = f.captured(1, 4, 6);
    if let Observation::Captured { image, .. } = &mut input.observation {
        let file = f.root.join(&image.path);
        let mut bytes = fs::read(&file).unwrap();
        bytes.truncate(33);
        fs::write(file, &bytes).unwrap();
        image.sha256 = digest(&bytes);
    }
    invalid(f.verify(input));
}
#[test]
fn wrong_dimensions_and_unapproved_full_page_height_are_rejected() {
    let f = Fixture::new();
    for (width, height) in [(2, 3), (4, 5), (4, 9), (8, 6)] {
        invalid(f.verify(f.captured(1, width, height)));
    }
    let mut f = Fixture::new();
    f.matrix.units[0].viewports[0].full_page = true;
    f.matrix = freeze(f.matrix.units.clone(), None).unwrap();
    f.verify(f.captured(1, 4, 9)).unwrap();
    invalid(f.verify(f.captured(1, 4, 5)));
}
#[test]
fn unopened_or_unverified_product_images_cannot_be_marked_captured() {
    let f = Fixture::new();
    for field in 0..4 {
        let mut input = f.captured(1, 4, 6);
        if let Observation::Captured { human_check, .. } = &mut input.observation {
            match field {
                0 => human_check.opened = false,
                1 => human_check.current_product = false,
                2 => human_check.current_version = false,
                _ => human_check.notes.clear(),
            };
        }
        invalid(f.verify(input));
    }
}
#[test]
fn tools_addresses_and_capture_times_must_be_explicit() {
    let f = Fixture::new();
    for field in 0..5 {
        let mut input = f.captured(1, 4, 6);
        if let Observation::Captured {
            tool,
            address,
            taken_at,
            ..
        } = &mut input.observation
        {
            match field {
                0 => tool.clear(),
                1 => address.clear(),
                2 => *address = "not an address".into(),
                3 => taken_at.clear(),
                _ => *taken_at = "2026-02-30T11:00:00Z".into(),
            };
        }
        invalid(f.verify(input));
    }
}
#[test]
fn unknown_versions_remain_explicit_instead_of_becoming_a_verified_build() {
    let mut f = Fixture::new();
    f.version = TestedVersion {
        head: None,
        working_tree: WorkingTree::Unknown,
        build_id: None,
    };
    let verified = f.verify(f.captured(1, 4, 6)).unwrap();
    assert!(!verified.version_known);
    assert!(!TestedVersion {
        head: Some("a".repeat(40)),
        working_tree: WorkingTree::Unknown,
        build_id: None
    }
    .known());
    assert!(TestedVersion {
        head: None,
        working_tree: WorkingTree::Unknown,
        build_id: Some("build-782".into())
    }
    .known());
}
#[test]
fn malformed_versions_and_synthetic_origins_fail_schema_validation() {
    for head in ["abc", " ", &"G".repeat(40)] {
        let version = TestedVersion {
            head: Some(head.into()),
            working_tree: WorkingTree::Clean,
            build_id: None,
        };
        assert_eq!(version.validate().unwrap_err().code, 2);
    }
    let f = Fixture::new();
    let mut value = serde_json::to_value(f.captured(1, 4, 6)).unwrap();
    value["observation"]["origin"] = "design_export".into();
    assert!(serde_json::from_value::<CaptureInput>(value).is_err());
    let mut value = serde_json::to_value(f.missing()).unwrap();
    value["observation"]["image"] = "previous.png".into();
    assert!(serde_json::from_value::<CaptureInput>(value).is_err());
}
#[test]
fn interaction_frames_require_actions_and_observed_results() {
    let mut f = Fixture::new();
    f.matrix.units[0].states[0].steps[0].id = "03".into();
    f.matrix = freeze(f.matrix.units.clone(), None).unwrap();
    let mut input = f.captured(1, 4, 6);
    invalid(f.verify(input.clone()));
    if let Observation::Captured { interactions, .. } = &mut input.observation {
        interactions.push(Interaction {
            action: "click submit".into(),
            observed: "progress shown".into(),
        });
    }
    f.verify(input.clone()).unwrap();
    if let Observation::Captured { interactions, .. } = &mut input.observation {
        interactions[0].observed.clear();
    }
    invalid(f.verify(input));
}
#[test]
fn utc_and_offset_timestamps_keep_their_original_observation_text() {
    let f = Fixture::new();
    for stamp in [
        "2024-02-29T00:00:00.125Z",
        "2026-10-10T11:00:00+08:00",
        "2026-10-10T11:00:00-04:30",
    ] {
        let mut input = f.captured(1, 4, 6);
        if let Observation::Captured { taken_at, .. } = &mut input.observation {
            *taken_at = stamp.into();
        }
        assert_eq!(f.verify(input.clone()).unwrap().input, input);
    }
}
#[cfg(unix)]
#[test]
fn links_cannot_relabel_old_files_as_current_round_images() {
    let f = Fixture::new();
    let input = f.captured(1, 4, 6);
    if let Observation::Captured { image, .. } = &input.observation {
        let old = f.root.join("old.png");
        let current = f.root.join(&image.path);
        fs::rename(&current, &old).unwrap();
        std::os::unix::fs::symlink(old, current).unwrap();
    }
    invalid(f.verify(input));
}
