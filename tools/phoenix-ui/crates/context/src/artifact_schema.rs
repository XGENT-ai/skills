//! JS: lib/artifact-schema.mjs

pub const DESIGN_SIDECAR_SCHEMA_VERSION: i64 = 2;

/// JS: readSidecarSchemaVersion
pub fn read_sidecar_schema_version(sidecar: Option<&serde_json::Value>) -> Option<i64> {
    let v = sidecar?.as_object()?.get("schemaVersion")?;
    if let Some(i) = v.as_i64() {
        return Some(i);
    }
    if let Some(f) = v.as_f64() {
        if f.fract() == 0.0 && f.is_finite() {
            return Some(f as i64);
        }
    }
    None
}
