//! Region capture over a frozen static-HTML snapshot.
use impeccable_browser::html_snapshot::HtmlSnapshot;
use impeccable_comp_verbs::asset_capture::capture_sha256 as hash;
use serde_json::{json, Value};
use std::{collections::BTreeSet, sync::Arc};

pub use impeccable_browser::html_snapshot::SnapshotSelection;

/// Most raster regions one native capture measures in a frame. The browser
/// adapter batches a frame's regions into one document and refuses a larger
/// batch, so no native capture, in process or through the host service, ever
/// carries more receipts per frame than this.
pub const MAX_CAPTURE_REGIONS: usize = 32;

/// Every requested region shares one frozen snapshot, server and browser
/// document. Fresh in-process diagnostic evidence; persisted receipts are
/// deliberately not inputs.
pub fn capture_regions(
    snapshot: &Arc<HtmlSnapshot>,
    renderer: &mut dyn impeccable_comp_verbs::asset_capture::AssetRenderer,
    spec_path: &str,
    region_ids: &[&str],
    reduced_motion: bool,
) -> Result<Vec<impeccable_comp_verbs::asset_capture::AssetCapture>, String> {
    capture_regions_at_viewport(
        snapshot,
        renderer,
        spec_path,
        region_ids,
        reduced_motion,
        None,
    )
}

/// Desktop/mobile captures retain the comp-owned region coordinates scaled
/// to the requested width. Reflow fidelity remains a separate gate concern.
pub fn capture_regions_at_viewport(
    snapshot: &Arc<HtmlSnapshot>,
    renderer: &mut dyn impeccable_comp_verbs::asset_capture::AssetRenderer,
    spec_path: &str,
    region_ids: &[&str],
    reduced_motion: bool,
    viewport: Option<[u32; 2]>,
) -> Result<Vec<impeccable_comp_verbs::asset_capture::AssetCapture>, String> {
    use impeccable_comp_verbs::asset_capture::AssetCaptureRequest;
    if region_ids.is_empty()
        || region_ids.len() > MAX_CAPTURE_REGIONS
        || region_ids.iter().copied().collect::<BTreeSet<_>>().len() != region_ids.len()
    {
        return Err(format!(
            "capture requires 1 to {MAX_CAPTURE_REGIONS} distinct regions"
        ));
    }
    let spec: Value = serde_json::from_slice(
        snapshot
            .bytes(spec_path)
            .ok_or("spec is not bound to snapshot")?,
    )
    .map_err(|e| e.to_string())?;
    let reference = spec["comp"].as_str().ok_or("spec has no reference")?;
    let width = u32::try_from(
        spec["compSize"]["width"]
            .as_u64()
            .ok_or("missing reference width")?,
    )
    .map_err(|e| e.to_string())?;
    let height = u32::try_from(
        spec["compSize"]["height"]
            .as_u64()
            .ok_or("missing reference height")?,
    )
    .map_err(|e| e.to_string())?;
    let scale = viewport.map(|v| v[0] as f64 / width as f64).unwrap_or(1.);
    let [width, height] = viewport.unwrap_or([width, height]);
    let server = snapshot.serve_open()?;
    let mut requests = Vec::new();
    for region_id in region_ids {
        let regions: Vec<_> = spec["regions"]
            .as_array()
            .ok_or("missing regions")?
            .iter()
            .filter(|r| r["id"] == *region_id)
            .collect();
        if regions.len() != 1 {
            return Err("region must resolve uniquely in bound spec".into());
        }
        let region = regions[0];
        let asset = region["plate"].as_str().ok_or("region has no asset path")?;
        if !snapshot.is_served(asset) {
            return Err("required asset is not served by snapshot".into());
        }
        let request = AssetCaptureRequest {
            url: server.entry_url(),
            viewport: [width, height],
            reduced_motion,
            expected_box: {
                let mut b: impeccable_comp_verbs::asset_capture::CaptureBox =
                    serde_json::from_value(region["px"].clone()).map_err(|e| e.to_string())?;
                b.x *= scale;
                b.y *= scale;
                b.w *= scale;
                b.h *= scale;
                b
            },
            reference_bytes: snapshot
                .bytes(reference)
                .ok_or("reference is not bound to snapshot")?
                .to_vec(),
            asset_bytes: snapshot
                .bytes(asset)
                .ok_or("asset is not bound to snapshot")?
                .to_vec(),
        };
        request.validate()?;
        requests.push(request);
    }
    let mut captures = if requests.len() == 1 {
        vec![renderer.capture(&requests[0])?]
    } else {
        renderer.capture_batch(&requests)?
    };
    if captures.len() != requests.len() {
        return Err("native capture omitted requested regions".into());
    }
    for ((capture, request), region_id) in captures.iter_mut().zip(&requests).zip(region_ids) {
        let receipt = &capture.receipt;
        if receipt["status"] == "captured" || receipt["stableCapture"] == true {
            let entry = snapshot.bytes(snapshot.entry()).unwrap();
            // Text responses arrive decoded; bind them to the text these bytes decode to.
            let document = if receipt["documentResponseText"] == true {
                hash(&impeccable_browser::response_capture::decoded_text(entry))
            } else {
                hash(entry)
            };
            let expected = json!({
                "resolvedUrl": request.url,
                "documentResponseSha256": document,
                "assetSha256": hash(&request.asset_bytes),
                "referenceSha256": hash(&request.reference_bytes),
                "viewport": {"width": width, "height": height, "dpr": 1},
                "expectedBox": request.expected_box,
                "reducedMotion": reduced_motion,
            });
            let mismatches: Vec<_> = expected
                .as_object()
                .unwrap()
                .iter()
                .filter(|(key, value)| receipt[*key] != **value)
                .map(|(key, _)| key.as_str())
                .collect();
            if !mismatches.is_empty() {
                return Err(format!(
                    "native capture does not match frozen inputs and document ({region_id}: {})",
                    mismatches.join(", ")
                ));
            }
        }
        capture.receipt["regionId"] = json!(region_id);
        capture.receipt["inputSnapshot"] =
            json!({"digest":snapshot.digest(),"manifest":snapshot.manifest()});
    }
    Ok(captures)
}
