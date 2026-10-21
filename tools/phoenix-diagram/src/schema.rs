use crate::input::{DiagramType, InputContract};
use jsonschema::{Registry, Validator};
use serde_json::Value;
use std::sync::OnceLock;

const LEGACY_COMMON: &str = include_str!("../upstream/legacy-2.17/schemas/common.schema.json");
const LEGACY: [(DiagramType, &str); 5] = [
    (
        DiagramType::Architecture,
        include_str!("../upstream/legacy-2.17/schemas/architecture.schema.json"),
    ),
    (
        DiagramType::Workflow,
        include_str!("../upstream/legacy-2.17/schemas/workflow.schema.json"),
    ),
    (
        DiagramType::Sequence,
        include_str!("../upstream/legacy-2.17/schemas/sequence.schema.json"),
    ),
    (
        DiagramType::Dataflow,
        include_str!("../upstream/legacy-2.17/schemas/dataflow.schema.json"),
    ),
    (
        DiagramType::Lifecycle,
        include_str!("../upstream/legacy-2.17/schemas/lifecycle.schema.json"),
    ),
];
const PHOENIX_COMMON: &str =
    include_str!("../../../skills/phoenix-diagram/schemas/common.schema.json");
const PHOENIX: [(DiagramType, &str); 10] = [
    (
        DiagramType::Architecture,
        include_str!("../../../skills/phoenix-diagram/schemas/architecture.schema.json"),
    ),
    (
        DiagramType::Workflow,
        include_str!("../../../skills/phoenix-diagram/schemas/workflow.schema.json"),
    ),
    (
        DiagramType::Sequence,
        include_str!("../../../skills/phoenix-diagram/schemas/sequence.schema.json"),
    ),
    (
        DiagramType::Dataflow,
        include_str!("../../../skills/phoenix-diagram/schemas/dataflow.schema.json"),
    ),
    (
        DiagramType::Lifecycle,
        include_str!("../../../skills/phoenix-diagram/schemas/lifecycle.schema.json"),
    ),
    (
        DiagramType::Erd,
        include_str!("../../../skills/phoenix-diagram/schemas/erd.schema.json"),
    ),
    (
        DiagramType::Class,
        include_str!("../../../skills/phoenix-diagram/schemas/class.schema.json"),
    ),
    (
        DiagramType::Tree,
        include_str!("../../../skills/phoenix-diagram/schemas/tree.schema.json"),
    ),
    (
        DiagramType::Timeline,
        include_str!("../../../skills/phoenix-diagram/schemas/timeline.schema.json"),
    ),
    (
        DiagramType::Waterfall,
        include_str!("../../../skills/phoenix-diagram/schemas/waterfall.schema.json"),
    ),
];

fn sources(contract: InputContract) -> (&'static str, &'static [(DiagramType, &'static str)]) {
    match contract {
        InputContract::Legacy217 => (LEGACY_COMMON, &LEGACY),
        InputContract::PhoenixV1 => (PHOENIX_COMMON, &PHOENIX),
    }
}

// The common schema comes first; each type schema follows in table order.
fn schemas(contract: InputContract) -> Result<Vec<Value>, String> {
    let (common, types) = sources(contract);
    let mut schemas: Vec<Value> = std::iter::once(common)
        .chain(types.iter().map(|(_, bytes)| *bytes))
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
    let index = sources(contract)
        .1
        .iter()
        .position(|(value, _)| *value == kind)
        .ok_or("The input contract has no schema for this diagram type")?;
    Ok(&validators[index])
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
        let original: Value = serde_json::from_str(PHOENIX_COMMON).unwrap();
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
