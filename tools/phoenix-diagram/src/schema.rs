use crate::input::{DiagramType, InputContract};
use jsonschema::{Registry, Validator};
use serde_json::Value;
use std::sync::OnceLock;

const LEGACY: [&str; 6] = [
    include_str!("../upstream/legacy-2.17/schemas/common.schema.json"),
    include_str!("../upstream/legacy-2.17/schemas/architecture.schema.json"),
    include_str!("../upstream/legacy-2.17/schemas/workflow.schema.json"),
    include_str!("../upstream/legacy-2.17/schemas/sequence.schema.json"),
    include_str!("../upstream/legacy-2.17/schemas/dataflow.schema.json"),
    include_str!("../upstream/legacy-2.17/schemas/lifecycle.schema.json"),
];
const PHOENIX: [&str; 6] = [
    include_str!("../../../skills/phoenix-diagram/schemas/common.schema.json"),
    include_str!("../../../skills/phoenix-diagram/schemas/architecture.schema.json"),
    include_str!("../../../skills/phoenix-diagram/schemas/workflow.schema.json"),
    include_str!("../../../skills/phoenix-diagram/schemas/sequence.schema.json"),
    include_str!("../../../skills/phoenix-diagram/schemas/dataflow.schema.json"),
    include_str!("../../../skills/phoenix-diagram/schemas/lifecycle.schema.json"),
];

fn schemas(contract: InputContract) -> Result<Vec<Value>, String> {
    let source = match contract {
        InputContract::Legacy217 => LEGACY,
        InputContract::PhoenixV1 => PHOENIX,
    };
    let mut schemas: Vec<Value> = source
        .iter()
        .map(|bytes| serde_json::from_str(bytes).map_err(|error| error.to_string()))
        .collect::<Result<_, _>>()?;
    if contract == InputContract::PhoenixV1 {
        let pattern = schemas[0]
            .pointer_mut("/$defs/portableOutputPath/allOf/2/not/pattern")
            .ok_or("Missing the frozen portableOutputPath surrogate constraint")?;
        if pattern != r"[\uD800-\uDFFF]" {
            return Err("The frozen surrogate constraint changed".into());
        }
        // serde_json strings contain Unicode scalars: unpaired surrogates cannot
        // reach schema validation. Preserve every other path constraint.
        *pattern = Value::String("(?!)".into());
    }
    Ok(schemas)
}

fn build(contract: InputContract) -> Result<Vec<Validator>, String> {
    let schemas = schemas(contract)?;
    let mut registry = Registry::new();
    for schema in &schemas {
        registry = registry
            .add(
                schema["$id"].as_str().ok_or("Missing embedded schema id")?,
                schema.clone(),
            )
            .map_err(|error| error.to_string())?;
    }
    let registry = registry.prepare().map_err(|error| error.to_string())?;
    schemas[1..]
        .iter()
        .map(|schema| {
            jsonschema::draft202012::options()
                .offline()
                .with_registry(&registry)
                .build(schema)
                .map_err(|error| error.to_string())
        })
        .collect()
}

pub(crate) fn validator(
    kind: DiagramType,
    contract: InputContract,
) -> Result<&'static Validator, &'static str> {
    static LEGACY_VALIDATORS: OnceLock<Result<Vec<Validator>, String>> = OnceLock::new();
    static PHOENIX_VALIDATORS: OnceLock<Result<Vec<Validator>, String>> = OnceLock::new();
    let cache = match contract {
        InputContract::Legacy217 => &LEGACY_VALIDATORS,
        InputContract::PhoenixV1 => &PHOENIX_VALIDATORS,
    };
    let validators = cache
        .get_or_init(|| build(contract))
        .as_ref()
        .map_err(String::as_str)?;
    Ok(&validators[DiagramType::ALL
        .iter()
        .position(|value| *value == kind)
        .expect("supported diagram type")])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn registry_never_retrieves_external_http_or_file_references() {
        for reference in ["https://example.invalid/schema.json", "file:///etc/passwd"] {
            assert!(jsonschema::draft202012::options()
                .offline()
                .build(&json!({"$ref":reference}))
                .is_err());
        }
    }

    #[test]
    fn adaptation_changes_only_the_exact_surrogate_pattern() {
        let original: Value = serde_json::from_str(PHOENIX[0]).unwrap();
        let mut adapted = schemas(InputContract::PhoenixV1).unwrap().remove(0);
        *adapted
            .pointer_mut("/$defs/portableOutputPath/allOf/2/not/pattern")
            .unwrap() = original
            .pointer("/$defs/portableOutputPath/allOf/2/not/pattern")
            .unwrap()
            .clone();
        assert_eq!(original, adapted);
    }
}
