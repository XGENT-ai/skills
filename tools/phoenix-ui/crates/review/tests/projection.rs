use phoenix_ui_review::projection::project_block;

#[test]
fn unrelated_edits_and_crlf_bytes_survive_reprojection() {
    let original = "# 实施记录\r\n\r\n人工正文\r\n".as_bytes();
    let first = project_block(original, "cycle-r1", None, "第一轮记录\n".as_bytes()).unwrap();
    assert!(first.bytes.starts_with(original));
    let mut edited = "# 实施记录（用户编辑）\r\n\r\n".as_bytes().to_vec();
    edited.extend_from_slice(&first.bytes);
    edited.extend_from_slice(b"\r\nUnrelated manual tail\r\n");
    let next = project_block(
        &edited,
        "cycle-r1",
        Some(&first.block_hash),
        "更新后的记录\n".as_bytes(),
    )
    .unwrap();
    let old_text = String::from_utf8(edited).unwrap();
    let expected = old_text.replace("第一轮记录\n", "更新后的记录\n");
    assert_eq!(next.bytes, expected.as_bytes());
    assert_ne!(first.block_hash, next.block_hash);
}

#[test]
fn hand_editing_a_generated_block_requires_explicit_import() {
    let first = project_block(b"manual\n", "cycle-r1", None, b"verified\n").unwrap();
    let edited = String::from_utf8(first.bytes.clone())
        .unwrap()
        .replace("verified", "hand-edited");
    assert_eq!(
        project_block(
            edited.as_bytes(),
            "cycle-r1",
            Some(&first.block_hash),
            b"new\n"
        )
        .unwrap_err()
        .code,
        3
    );
    assert_eq!(
        project_block(&first.bytes, "cycle-r1", None, b"new\n")
            .unwrap_err()
            .code,
        3
    );
}

#[test]
fn missing_partial_and_duplicate_boundaries_cannot_be_silently_recreated() {
    let first = project_block(b"", "cycle-r1", None, b"body\n").unwrap();
    assert_eq!(
        project_block(
            b"no generated block\n",
            "cycle-r1",
            Some(&first.block_hash),
            b"next\n"
        )
        .unwrap_err()
        .code,
        3
    );
    let begin_only = b"<!-- phoenix-ui:cycle-r1:begin -->\nbody\n";
    assert_eq!(
        project_block(begin_only, "cycle-r1", None, b"next\n")
            .unwrap_err()
            .code,
        3
    );
    let mut duplicate = first.bytes.clone();
    duplicate.extend_from_slice(&first.bytes);
    assert_eq!(
        project_block(&duplicate, "cycle-r1", Some(&first.block_hash), b"next\n")
            .unwrap_err()
            .code,
        3
    );
}

#[test]
fn another_cycle_block_is_preserved_byte_for_byte() {
    let first = project_block(b"manual\n", "cycle-a-r1", None, b"first cycle\n").unwrap();
    let second = project_block(&first.bytes, "cycle-b-r1", None, b"second cycle\n").unwrap();
    let revised = project_block(
        &second.bytes,
        "cycle-a-r1",
        Some(&first.block_hash),
        b"changed first cycle\n",
    )
    .unwrap();
    let unchanged = project_block(
        &revised.bytes,
        "cycle-b-r1",
        Some(&second.block_hash),
        b"second cycle\n",
    )
    .unwrap();
    assert_eq!(unchanged.bytes, revised.bytes);
}

#[test]
fn block_identity_and_content_cannot_inject_boundaries() {
    for id in ["", "cycle-->bad", "cycle\nother", "cycle--bad"] {
        assert_eq!(project_block(b"", id, None, b"body\n").unwrap_err().code, 2);
    }
    assert_eq!(
        project_block(b"", "cycle-r1", None, b"<!-- phoenix-ui:cycle-r1:end -->")
            .unwrap_err()
            .code,
        2
    );
}
