use phoenix_ui_review::capture::{
    append_capture, captures_complete, CaptureAttempt, CaptureStatus, CellKey,
};

fn cell() -> CellKey {
    CellKey {
        unit_id: "U1".into(),
        state_id: "empty".into(),
        viewport_id: "mobile".into(),
        theme_id: "light".into(),
        step_id: "initial".into(),
    }
}
fn entry(attempt: u32, hash: char, status: CaptureStatus) -> CaptureAttempt {
    CaptureAttempt {
        cell: cell(),
        attempt,
        content_hash: hash.to_string().repeat(64),
        status,
    }
}

#[test]
fn missing_then_recapture_appends_without_erasing_the_attempt() {
    let matrix = vec![cell()];
    let mut history = Vec::new();
    let missing = entry(
        1,
        'a',
        CaptureStatus::Missing {
            reason: "login did not complete".into(),
        },
    );
    assert!(append_capture(&matrix, &mut history, missing.clone(), false).unwrap());
    assert!(!captures_complete(&matrix, &history));
    assert!(append_capture(
        &matrix,
        &mut history,
        entry(2, 'b', CaptureStatus::Captured),
        false
    )
    .unwrap());
    assert_eq!(history[0], missing);
    assert_eq!(history.len(), 2);
    assert!(captures_complete(&matrix, &history));
}

#[test]
fn same_cell_attempt_and_content_is_idempotent() {
    let matrix = vec![cell()];
    let mut history = Vec::new();
    let first = entry(1, 'a', CaptureStatus::Captured);
    assert!(append_capture(&matrix, &mut history, first.clone(), false).unwrap());
    assert!(!append_capture(&matrix, &mut history, first, false).unwrap());
    assert_eq!(history.len(), 1);
}

#[test]
fn different_content_cannot_replace_an_attempt_or_relabel_missing_as_captured() {
    let matrix = vec![cell()];
    let mut history = Vec::new();
    let missing = entry(
        1,
        'a',
        CaptureStatus::Missing {
            reason: "tool unavailable".into(),
        },
    );
    append_capture(&matrix, &mut history, missing.clone(), false).unwrap();
    assert!(append_capture(
        &matrix,
        &mut history,
        entry(1, 'b', CaptureStatus::Captured),
        false
    )
    .is_err());
    assert!(append_capture(
        &matrix,
        &mut history,
        entry(1, 'a', CaptureStatus::Captured),
        false
    )
    .is_err());
    assert_eq!(history, [missing]);
}

#[test]
fn a_new_attempt_can_legitimately_have_the_same_image_hash() {
    let matrix = vec![cell()];
    let mut history = Vec::new();
    for attempt in [1, 2] {
        assert!(append_capture(
            &matrix,
            &mut history,
            entry(attempt, 'a', CaptureStatus::Captured),
            false
        )
        .unwrap());
    }
    assert_eq!(history.len(), 2);
}

#[test]
fn latest_missing_does_not_fall_back_to_an_old_capture() {
    let matrix = vec![cell()];
    let mut history = Vec::new();
    append_capture(
        &matrix,
        &mut history,
        entry(1, 'a', CaptureStatus::Captured),
        false,
    )
    .unwrap();
    append_capture(
        &matrix,
        &mut history,
        entry(
            2,
            'b',
            CaptureStatus::Missing {
                reason: "recapture failed".into(),
            },
        ),
        false,
    )
    .unwrap();
    assert!(!captures_complete(&matrix, &history));
}

#[test]
fn all_five_identity_components_define_a_matrix_cell() {
    let original = cell();
    for field in 0..5 {
        let mut changed = original.clone();
        match field {
            0 => changed.unit_id.push('2'),
            1 => changed.state_id.push('2'),
            2 => changed.viewport_id.push('2'),
            3 => changed.theme_id.push('2'),
            _ => changed.step_id.push('2'),
        }
        let matrix = vec![original.clone(), changed.clone()];
        let mut history = Vec::new();
        append_capture(
            &matrix,
            &mut history,
            entry(1, 'a', CaptureStatus::Captured),
            false,
        )
        .unwrap();
        assert!(!captures_complete(&matrix, &history));
        let mut second = entry(1, 'b', CaptureStatus::Captured);
        second.cell = changed;
        append_capture(&matrix, &mut history, second, false).unwrap();
        assert!(captures_complete(&matrix, &history));
    }
    assert!(!captures_complete(&[], &[]));
}

#[test]
fn attempt_numbers_are_contiguous_and_inputs_match_the_frozen_matrix() {
    let matrix = vec![cell()];
    let mut history = Vec::new();
    for attempt in [0, 2] {
        assert!(append_capture(
            &matrix,
            &mut history,
            entry(attempt, 'a', CaptureStatus::Captured),
            false
        )
        .is_err());
    }
    let mut unknown = entry(1, 'a', CaptureStatus::Captured);
    unknown.cell.unit_id = "U2".into();
    assert!(append_capture(&matrix, &mut history, unknown, false).is_err());
    let mut malformed = entry(1, 'a', CaptureStatus::Captured);
    malformed.content_hash = "unknown".into();
    assert!(append_capture(&matrix, &mut history, malformed, false).is_err());
    assert!(append_capture(
        &matrix,
        &mut history,
        entry(
            1,
            'a',
            CaptureStatus::Missing {
                reason: String::new()
            }
        ),
        false
    )
    .is_err());
    assert!(history.is_empty());
}

#[test]
fn sealed_rounds_reject_even_duplicate_capture_writes() {
    let matrix = vec![cell()];
    let first = entry(1, 'a', CaptureStatus::Captured);
    let mut history = vec![first.clone()];
    assert!(append_capture(&matrix, &mut history, first, true).is_err());
    assert!(append_capture(
        &matrix,
        &mut history,
        entry(2, 'b', CaptureStatus::Captured),
        true
    )
    .is_err());
    assert_eq!(history.len(), 1);
}
