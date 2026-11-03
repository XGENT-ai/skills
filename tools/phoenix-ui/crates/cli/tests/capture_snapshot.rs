use impeccable::capture_snapshot::{capture_regions, SnapshotSelection};
use impeccable_browser::html_snapshot::HtmlSnapshot;
use impeccable_comp_verbs::asset_capture::{AssetCapture, AssetRenderer};

fn capture_region(
    snapshot: &std::sync::Arc<HtmlSnapshot>,
    renderer: &mut dyn AssetRenderer,
    region_id: &str,
) -> Result<AssetCapture, String> {
    capture_regions(
        snapshot,
        renderer,
        ".phoenix-ui/spec.json",
        &[region_id],
        true,
    )
    .map(|mut c| c.remove(0))
}
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "capture-snapshot-{}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("assets")).unwrap();
        fs::create_dir_all(root.join(".phoenix-ui")).unwrap();
        for (name, bytes) in [
            ("index.html", "<img src='assets/art.png'>"),
            ("other.html", "other page"),
            ("assets/art.png", "asset bytes"),
            (".phoenix-ui/spec.json", "{}"),
            (".phoenix-ui/comp.png", "comp bytes"),
            (".env", "private"),
        ] {
            fs::write(root.join(name), bytes).unwrap();
        }
        Self(root)
    }
    fn selection(&self) -> SnapshotSelection {
        SnapshotSelection {
            root: self.0.clone(),
            entry: "index.html".into(),
            served: vec!["index.html".into(), "assets/art.png".into()],
            bound: vec![
                ".phoenix-ui/spec.json".into(),
                ".phoenix-ui/comp.png".into(),
            ],
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn fresh_capture_ignores_saved_receipts_and_rejects_wrong_documents() {
    use impeccable_comp_verbs::asset_capture::{
        AssetCapture, AssetCaptureRequest, AssetRenderer, capture_sha256,
    };
    use serde_json::json;
    use std::sync::Arc;
    let f = Fixture::new();
    fs::write(f.0.join(".phoenix-ui/spec.json"),serde_json::to_vec(&json!({"comp":".phoenix-ui/comp.png","compSize":{"width":200,"height":200},"regions":[{"id":"art","plate":"assets/art.png","px":{"x":20,"y":20,"w":80,"h":80}}]})).unwrap()).unwrap();
    struct Fake {
        entry: Vec<u8>,
        wrong: bool,
        partial: bool,
        calls: usize,
    }
    impl AssetRenderer for Fake {
        fn capture(&mut self, r: &AssetCaptureRequest) -> Result<AssetCapture, String> {
            self.calls += 1;
            assert!(r.url.starts_with("http://127.0.0.1:"));
            assert!(r.url.ends_with("/index.html"));
            Ok(AssetCapture {
                receipt: json!({"status":if self.partial {"unavailable"} else {"captured"},"stableCapture":true,"resolvedUrl":if self.wrong {"http://other/page"} else {&r.url},"documentResponseSha256":capture_sha256(&self.entry),"assetSha256":capture_sha256(&r.asset_bytes),"referenceSha256":capture_sha256(&r.reference_bytes),"viewport":{"width":200,"height":200,"dpr":1},"expectedBox":r.expected_box,"reducedMotion":true}),
                images: vec![],
            })
        }
    }
    let s = Arc::new(HtmlSnapshot::freeze(f.selection()).unwrap());
    let mut renderer = Fake {
        entry: s.bytes("index.html").unwrap().to_vec(),
        wrong: false,
        partial: false,
        calls: 0,
    };
    fs::write(
        f.0.join(".phoenix-ui/receipt.json"),
        br#"{"status":"captured","visible":true}"#,
    )
    .unwrap();
    let capture = capture_region(&s, &mut renderer, "art").unwrap();
    assert_eq!(capture.receipt["inputSnapshot"]["digest"], s.digest());
    assert_eq!(renderer.calls, 1);
    renderer.wrong = true;
    let error = capture_region(&s, &mut renderer, "art").err().unwrap();
    assert!(error.contains("art: resolvedUrl"), "{error}");
    renderer.partial = true;
    assert!(
        capture_region(&s, &mut renderer, "art").is_err(),
        "partial stable evidence must also bind the document"
    );
    renderer.partial = false;
    renderer.wrong = false;
    renderer.entry = b"different entry".to_vec();
    assert!(capture_region(&s, &mut renderer, "art").is_err());
}

#[test]
fn native_browser_capture_is_bound_to_frozen_entry_spec_and_asset() {
    use impeccable::asset_capture::CdpAssetRenderer;
    use impeccable_comp::{png_io, raster};
    use std::sync::Arc;
    let env = std::env::vars().collect();
    if impeccable_browser::discovery::find_browser(&env).is_err() {
        eprintln!("skip: browser unavailable");
        return;
    }
    let f = Fixture::new();
    fs::write(f.0.join("index.html"),"<!doctype html><style>body{margin:0;background:white}img{position:absolute;left:20px;top:20px;width:80px;height:80px}</style><img src='assets/art.png'>").unwrap();
    let image = png_io::encode_png(&raster::create_image(32, 32, [231, 60, 30, 255]), &[]).unwrap();
    fs::write(f.0.join("assets/art.png"), &image).unwrap();
    fs::write(f.0.join(".phoenix-ui/comp.png"), &image).unwrap();
    fs::write(f.0.join(".phoenix-ui/spec.json"),br#"{"comp":".phoenix-ui/comp.png","compSize":{"width":200,"height":200},"regions":[{"id":"art","plate":"assets/art.png","px":{"x":20,"y":20,"w":80,"h":80}}]}"#).unwrap();
    let snapshot = Arc::new(HtmlSnapshot::freeze(f.selection()).unwrap());
    let capture =
        capture_region(&snapshot, &mut CdpAssetRenderer::from_process_env(), "art").unwrap();
    assert_eq!(capture.receipt["status"], "captured", "{}", capture.receipt);
    assert_eq!(
        capture.receipt["combinedContribution"]["changedPixelsInRegion"],
        6400
    );
    assert_eq!(
        capture.receipt["inputSnapshot"]["digest"],
        snapshot.digest()
    );
}

#[test]
fn native_capture_binds_bom_entry_to_its_frozen_bytes() {
    use impeccable::asset_capture::CdpAssetRenderer;
    use impeccable_comp::{png_io, raster};
    use std::sync::Arc;
    if impeccable_browser::discovery::find_browser(&std::env::vars().collect()).is_err() {
        eprintln!("skip: browser unavailable");
        return;
    }
    let f = Fixture::new();
    // CDP returns this document as decoded text, without its BOM.
    fs::write(f.0.join("index.html"),b"\xEF\xBB\xBF<!doctype html>\r\n<!-- caf\xC3\xA9 --><style>body{margin:0;background:white}img{position:absolute;left:20px;top:20px;width:80px;height:80px}</style><img src='assets/art.png'>").unwrap();
    let image = png_io::encode_png(&raster::create_image(32, 32, [231, 60, 30, 255]), &[]).unwrap();
    fs::write(f.0.join("assets/art.png"), &image).unwrap();
    fs::write(f.0.join(".phoenix-ui/comp.png"), &image).unwrap();
    fs::write(f.0.join(".phoenix-ui/spec.json"),br#"{"comp":".phoenix-ui/comp.png","compSize":{"width":200,"height":200},"regions":[{"id":"art","plate":"assets/art.png","px":{"x":20,"y":20,"w":80,"h":80}}]}"#).unwrap();
    let snapshot = Arc::new(HtmlSnapshot::freeze(f.selection()).unwrap());
    let capture =
        capture_region(&snapshot, &mut CdpAssetRenderer::from_process_env(), "art").unwrap();
    assert_eq!(capture.receipt["status"], "captured", "{}", capture.receipt);
    assert_eq!(capture.receipt["documentResponseText"], true);
}

#[test]
fn batch_uses_one_document_and_rejects_mixed_inputs() {
    use impeccable::asset_capture::CdpAssetRenderer;
    use impeccable_comp::{png_io, raster};
    use impeccable_comp_verbs::asset_capture::{AssetCaptureRequest, AssetRenderer, CaptureBox};
    use std::sync::Arc;
    let env = std::env::vars().collect();
    if impeccable_browser::discovery::find_browser(&env).is_err() {
        eprintln!("skip: browser unavailable");
        return;
    }
    let f = Fixture::new();
    // A page-generated token must be identical for every measured region in the batch.
    fs::write(f.0.join("index.html"),"<!doctype html><style>body{margin:0;background:white}img{position:absolute;width:80px;height:80px}</style><img src='assets/art.png' style='left:20px;top:20px'><img src='assets/art.png' style='left:110px;top:110px'><script>document.body.dataset.visit=crypto.randomUUID()</script>").unwrap();
    let image = png_io::encode_png(&raster::create_image(32, 32, [231, 60, 30, 255]), &[]).unwrap();
    fs::write(f.0.join("assets/art.png"), &image).unwrap();
    fs::write(f.0.join(".phoenix-ui/comp.png"), &image).unwrap();
    fs::write(f.0.join(".phoenix-ui/spec.json"),br#"{"comp":".phoenix-ui/comp.png","compSize":{"width":200,"height":200},"regions":[{"id":"first","plate":"assets/art.png","px":{"x":20,"y":20,"w":80,"h":80}},{"id":"second","plate":"assets/art.png","px":{"x":110,"y":110,"w":80,"h":80}}]}"#).unwrap();
    let snapshot = Arc::new(HtmlSnapshot::freeze(f.selection()).unwrap());
    let server = snapshot.serve_open().unwrap();
    let make = |x| AssetCaptureRequest {
        url: server.entry_url(),
        viewport: [200, 200],
        reduced_motion: true,
        reference_bytes: image.clone(),
        asset_bytes: image.clone(),
        expected_box: CaptureBox {
            x,
            y: x,
            w: 80.,
            h: 80.,
        },
    };
    let mut renderer = CdpAssetRenderer::from_process_env();
    let captures = capture_regions(
        &snapshot,
        &mut renderer,
        ".phoenix-ui/spec.json",
        &["first", "second"],
        true,
    )
    .unwrap();
    assert_eq!(captures.len(), 2);
    for capture in &captures {
        assert_eq!(capture.receipt["status"], "captured", "{}", capture.receipt);
        assert_eq!(
            capture.receipt["combinedContribution"]["changedPixelsInRegion"],
            6400
        );
        assert_eq!(
            capture.receipt["domSha256"],
            captures[0].receipt["domSha256"]
        );
        assert_eq!(
            capture.receipt["screenshotSha256"],
            captures[0].receipt["screenshotSha256"]
        );
        assert_eq!(
            capture.receipt["captureDocument"],
            captures[0].receipt["captureDocument"]
        );
    }
    let mut wrong = make(110.);
    wrong.url.push_str("?other-entry");
    assert!(renderer.capture_batch(&[make(20.), wrong]).is_err());
}

#[test]
fn viewport_capture_does_not_switch_responsive_picture_sources() {
    use impeccable::asset_capture::CdpAssetRenderer;
    use impeccable_comp::{png_io, raster};
    use std::sync::Arc;
    if impeccable_browser::discovery::find_browser(&std::env::vars().collect()).is_err() {
        return;
    }
    let f = Fixture::new();
    fs::write(f.0.join("index.html"), "<!doctype html><style>body{margin:0;min-height:2400px;background:white}img{position:absolute;left:20px;top:20px;width:80px;height:80px}</style><picture><source media='(max-width:900px)' srcset='assets/mobile.png'><img src='assets/art.png'></picture>").unwrap();
    let image = png_io::encode_png(&raster::create_image(32, 32, [231, 60, 30, 255]), &[]).unwrap();
    let mobile =
        png_io::encode_png(&raster::create_image(32, 32, [30, 60, 231, 255]), &[]).unwrap();
    fs::write(f.0.join("assets/art.png"), &image).unwrap();
    fs::write(f.0.join("assets/mobile.png"), mobile).unwrap();
    fs::write(f.0.join(".phoenix-ui/comp.png"), &image).unwrap();
    fs::write(f.0.join(".phoenix-ui/spec.json"), br#"{"comp":".phoenix-ui/comp.png","compSize":{"width":1440,"height":960},"regions":[{"id":"art","plate":"assets/art.png","px":{"x":20,"y":20,"w":80,"h":80}}]}"#).unwrap();
    let mut selection = f.selection();
    selection.served.push("assets/mobile.png".into());
    let snapshot = Arc::new(HtmlSnapshot::freeze(selection).unwrap());
    let captures = capture_regions(
        &snapshot,
        &mut CdpAssetRenderer::from_process_env(),
        ".phoenix-ui/spec.json",
        &["art"],
        true,
    )
    .unwrap();
    for capture in &captures {
        assert_eq!(capture.receipt["status"], "captured", "{}", capture.receipt);
        assert_eq!(
            capture.receipt["combinedContribution"]["changedPixelsInRegion"],
            6400
        );
        assert_eq!(capture.receipt["batchStabilityVerified"], true);
        let responses = capture.receipt["settlingResponses"].as_array().unwrap();
        assert!(
            !responses
                .iter()
                .any(|r| r["url"].as_str().unwrap_or("").ends_with("mobile.png"))
        );
    }
}
