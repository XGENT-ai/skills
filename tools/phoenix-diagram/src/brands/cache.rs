use super::{
    capture::{validate_image, Captured},
    network, BrandError, ResolvedBrand, ResourceIdentity,
};
use crate::{
    input::sha256,
    output::{self, FileBinding},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};
use url::Url;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Envelope {
    schema_version: u32,
    url_sha256: String,
    source_url: String,
    resolved_url: String,
    media_type: String,
    bytes: usize,
    sha256: String,
    data_base64: String,
}
fn io<T>(_: T) -> BrandError {
    BrandError::new("brand/io", "Cannot access the local brand cache")
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}
fn path(cache: &Path, url_sha: &str, sha: &str) -> PathBuf {
    cache.join(format!("{url_sha}-{sha}.json"))
}
fn resolved(e: Envelope) -> ResolvedBrand {
    ResolvedBrand {
        title: Url::parse(&e.source_url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_owned))
            .unwrap_or_default(),
        mark: None,
        data_base64: Some(e.data_base64),
        resource: ResourceIdentity {
            id: format!("url:{}:{}", e.url_sha256, e.sha256),
            sha256: e.sha256,
            bytes: e.bytes,
            media_type: e.media_type,
            url_sha256: Some(e.url_sha256),
            source_url: e.source_url,
            resolved_url: e.resolved_url,
            catalog_version: None,
        },
    }
}
pub(super) fn verify_resolved(
    brand: &ResolvedBrand,
    url: &Url,
    sha: &str,
) -> Result<(), BrandError> {
    let invalid = || {
        BrandError::new(
            "brand/digest-mismatch",
            "Cached brand resource does not match its pinned identity",
        )
    };
    let r = &brand.resource;
    let url_sha = sha256(url.as_str().as_bytes());
    if !digest(sha)
        || r.url_sha256.as_deref() != Some(&url_sha)
        || r.sha256 != sha
        || r.source_url != network::public_url(url)
        || r.id != format!("url:{url_sha}:{sha}")
        || brand.mark.is_some()
        || r.catalog_version.is_some()
    {
        return Err(invalid());
    }
    let final_url = network::normalize_url(&r.resolved_url)?;
    if network::public_url(&final_url) != r.resolved_url {
        return Err(invalid());
    }
    let data = STANDARD
        .decode(brand.data_base64.as_deref().ok_or_else(invalid)?)
        .map_err(|_| invalid())?;
    if data.len() != r.bytes || sha256(&data) != sha {
        return Err(invalid());
    }
    validate_image(&r.media_type, &data)
}
pub(super) fn load(value: &Value, cache: &Path) -> Result<ResolvedBrand, BrandError> {
    let url = network::normalize_url(value["url"].as_str().unwrap_or_default())?;
    let sha = value["sha256"]
        .as_str()
        .filter(|s| digest(s))
        .ok_or_else(|| {
            BrandError::new(
                "brand/digest-mismatch",
                "A pinned lowercase SHA256 is required",
            )
        })?;
    let file = path(cache, &sha256(url.as_str().as_bytes()), sha);
    let binding = FileBinding::capture(&file).map_err(|_| {
        BrandError::new(
            "brand/cache-missing",
            "Pinned brand cache is missing or unsafe; explicitly capture the official URL",
        )
    })?;
    if binding.bytes().len() > 1_500_000 {
        return Err(BrandError::new(
            "brand/invalid-image",
            "Cached resource exceeds the image size limit",
        ));
    }
    let e: Envelope = serde_json::from_slice(binding.bytes())
        .map_err(|_| BrandError::new("brand/digest-mismatch", "Invalid cache envelope"))?;
    if e.schema_version != 1 {
        return Err(BrandError::new(
            "brand/digest-mismatch",
            "Unsupported cache schema",
        ));
    }
    let brand = resolved(e);
    verify_resolved(&brand, &url, sha)?;
    binding.verify().map_err(io)?;
    Ok(brand)
}
fn private_directory(path: &Path) -> Result<(), BrandError> {
    if path.is_dir() {
        let meta = fs::symlink_metadata(path).map_err(io)?;
        if meta.file_type().is_symlink() {
            return Err(io("symbolic cache"));
        }
        return Ok(());
    }
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        private_directory(parent)?;
    }
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path).map_err(io)
}
pub(super) fn store(
    url: &Url,
    captured: &Captured,
    cache: &Path,
) -> Result<(ResolvedBrand, PathBuf), BrandError> {
    validate_image(&captured.media_type, &captured.bytes)?;
    let e = Envelope {
        schema_version: 1,
        url_sha256: sha256(url.as_str().as_bytes()),
        source_url: network::public_url(url),
        resolved_url: network::public_url(&network::normalize_url(&captured.resolved_url)?),
        media_type: captured.media_type.clone(),
        bytes: captured.bytes.len(),
        sha256: sha256(&captured.bytes),
        data_base64: STANDARD.encode(&captured.bytes),
    };
    let target = path(cache, &e.url_sha256, &e.sha256);
    let brand = resolved(Envelope {
        schema_version: e.schema_version,
        url_sha256: e.url_sha256.clone(),
        source_url: e.source_url.clone(),
        resolved_url: e.resolved_url.clone(),
        media_type: e.media_type.clone(),
        bytes: e.bytes,
        sha256: e.sha256.clone(),
        data_base64: e.data_base64.clone(),
    });
    verify_resolved(&brand, url, &brand.resource.sha256)?;
    private_directory(cache)?;
    let slot = output::capture_slot(&target, &[]).map_err(io)?;
    if target.exists() {
        let old = FileBinding::capture(&target).map_err(io)?;
        let existing: Envelope = serde_json::from_slice(old.bytes()).map_err(io)?;
        if existing.schema_version != 1 || resolved(existing) != brand {
            return Err(BrandError::new(
                "brand/conflict",
                "An existing cache identity differs; it was preserved",
            ));
        }
        old.verify().map_err(io)?;
    } else {
        output::publish_bound_with_mode(&slot, &serde_json::to_vec(&e).map_err(io)?, 0o600, &[])
            .map_err(io)?;
    }
    Ok((brand, target))
}

pub(super) fn bindings(
    brands: &super::ResolvedBrands,
    cache: &Path,
) -> Result<Vec<FileBinding>, BrandError> {
    let mut files = vec![];
    for brand in brands.values().filter(|b| b.mark.is_none()) {
        let r = &brand.resource;
        let file = FileBinding::capture(&path(
            cache,
            r.url_sha256.as_deref().unwrap_or_default(),
            &r.sha256,
        ))
        .map_err(io)?;
        let e: Envelope = serde_json::from_slice(file.bytes()).map_err(io)?;
        if e.schema_version != 1 || resolved(e) != *brand {
            return Err(BrandError::new(
                "brand/digest-mismatch",
                "The cache changed while preparing the diagram",
            ));
        }
        file.verify().map_err(io)?;
        files.push(file);
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    #[test]
    fn cache_is_private_pinned_and_never_overwrites_a_changed_identity() {
        let temp = std::env::temp_dir().join(format!(
            "phoenix-brand-cache-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let url =
            network::normalize_url("https://example.com/icon.png?token=secret#session").unwrap();
        let captured = Captured {
            source_url: network::public_url(&url),
            resolved_url: "https://example.com/assets/icon.png?ignored".into(),
            media_type: "image/png".into(),
            bytes: include_bytes!("../../tests/fixtures/brands/official-test.png").to_vec(),
        };
        let (brand, file) = store(&url, &captured, &temp).unwrap();
        let value = serde_json::json!({"url":url.as_str(),"sha256":brand.resource.sha256});
        assert_eq!(load(&value, &temp).unwrap(), brand);
        assert!(!fs::read_to_string(&file).unwrap().contains("secret"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&file).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        let original = FileBinding::capture(&file).unwrap();
        store(&url, &captured, &temp).unwrap();
        original.verify().unwrap();
        let mut other_url = url.clone();
        other_url.set_query(Some("token=other"));
        let other = serde_json::json!({"url":other_url.as_str(),"sha256":brand.resource.sha256});
        assert_eq!(load(&other, &temp).unwrap_err().code, "brand/cache-missing");
        fs::write(&file, b"later claimant").unwrap();
        assert!(load(&value, &temp).is_err());
        assert!(store(&url, &captured, &temp).is_err());
        assert_eq!(fs::read(&file).unwrap(), b"later claimant");
        fs::remove_dir_all(temp).unwrap();
    }
}
