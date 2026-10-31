use phoenix_diagram::{
    check::FileIdentity,
    compare::render::{parse, views},
    render::escape_text,
};
use serde_json::json;

fn wrapper() -> (Vec<u8>, Vec<u8>) {
    let source = b"<!doctype html>\r\n<html><body>Original & exact\r\n</body></html>".to_vec();
    let metadata = json!({"schemaVersion":1,"artifactKind":"architecture-comparison","receiptFilename":"result.html.compare.json","receiptId":"receipt-abc","comparisonIr":{"changes":{"components":[],"connections":[],"boundaries":[]}},"viewIdentities":{"before":FileIdentity::of(&source),"delta":FileIdentity::of(&source),"after":FileIdentity::of(&source)}});
    let srcdoc = escape_text(std::str::from_utf8(&source).unwrap()).replace('\r', "&#13;");
    let mut html=format!("<!doctype html><html><head><meta charset=\"utf-8\"></head><body><script id=\"phoenix-diagram-comparison\" type=\"application/json\">{metadata}</script>");
    for id in ["before", "delta", "after"] {
        html.push_str(&format!(
            "<iframe data-comparison-view=\"{id}\" srcdoc=\"{srcdoc}\"></iframe>"
        ));
    }
    html.push_str("</body></html>");
    (html.into_bytes(), source)
}
#[test]
fn views_preserve_bound_srcdoc_bytes_including_carriage_returns() {
    let (html, source) = wrapper();
    let parsed = parse(&html).unwrap();
    assert_eq!(parsed.receipt_filename, "result.html.compare.json");
    assert_eq!(parsed.receipt_id, "receipt-abc");
    let views = views(&html).unwrap();
    assert_eq!(views.len(), 3);
    for id in ["before", "delta", "after"] {
        assert_eq!(views[id], source);
    }
}
#[test]
fn comparison_parser_rejects_tampered_missing_duplicate_external_or_unsafe_bindings() {
    let (html, _) = wrapper();
    let html = String::from_utf8(html).unwrap();
    let cases = [
        html.replacen("Original &amp; exact", "Changed &amp; exact", 1),
        html.replacen(
            "data-comparison-view=\"after\"",
            "data-comparison-view=\"before\"",
            1,
        ),
        html.replacen(
            "<iframe data-comparison-view=\"before\"",
            "<iframe src=\"https://example.com\" data-comparison-view=\"before\"",
            1,
        ),
        html.replace("result.html.compare.json", "../result.html.compare.json"),
        html.replace("receipt-abc", "receipt/unsafe"),
        html.replace(
            "</body>",
            "<script src=\"https://example.com/control.js\"></script></body>",
        ),
        html.replace("</body>", "<iframe srcdoc=\"extra\"></iframe></body>"),
        html.replace("\"schemaVersion\":1", "\"schemaVersion\":2"),
    ];
    for changed in cases {
        assert!(
            parse(changed.as_bytes()).is_err(),
            "accepted corrupted comparison"
        );
    }
}
