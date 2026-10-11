//! A JSON/JSONC span editor. Only owned values and their comma are edited;
//! whitespace, comments, spelling and numbers in unowned text remain byte exact.
use crate::{Error, ErrorKind, Identity, JsonPatch, JsonSlot, JsonStep, Result};
use serde::{Deserialize, Serialize};
use serde_json::{value::RawValue, Value};
use std::ops::Range;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct RecordedPatch {
    pub patch: JsonPatch,
    pub before_raw: Option<String>,
    pub structural: bool,
    pub identity: Option<Identity>,
}

struct Member {
    key: String,
    start: usize,
    node: Node,
}

enum Kind {
    Object(Vec<Member>),
    Array(Vec<Node>),
    Leaf,
}

struct Node {
    span: Range<usize>,
    kind: Kind,
    commas: Vec<usize>,
}

struct Document {
    text: String,
    masked: String,
    root: Node,
}

fn invalid(message: impl Into<String>) -> Error {
    Error::conflict(format!("shared JSON: {}", message.into()))
}

/// Replace comments with equally many bytes of whitespace, so all spans still
/// refer to the original document. Unterminated block comments are rejected.
fn mask_comments(text: &str) -> Result<String> {
    let mut bytes = text.as_bytes().to_vec();
    let (mut i, mut quoted, mut escaped) = (0, false, false);
    while i < bytes.len() {
        let b = bytes[i];
        if quoted {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                quoted = false;
            }
            i += 1;
            continue;
        }
        if b == b'"' {
            quoted = true;
            i += 1;
            continue;
        }
        if b == b'/' && bytes.get(i + 1) == Some(&b'/') {
            while i < bytes.len() && !matches!(bytes[i], b'\r' | b'\n') {
                bytes[i] = b' ';
                i += 1;
            }
        } else if b == b'/' && bytes.get(i + 1) == Some(&b'*') {
            bytes[i] = b' ';
            bytes[i + 1] = b' ';
            i += 2;
            let mut closed = false;
            while i < bytes.len() {
                if bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/') {
                    bytes[i] = b' ';
                    bytes[i + 1] = b' ';
                    i += 2;
                    closed = true;
                    break;
                }
                if !matches!(bytes[i], b'\r' | b'\n') {
                    bytes[i] = b' ';
                }
                i += 1;
            }
            if !closed {
                return Err(invalid("unterminated block comment"));
            }
        } else {
            i += 1;
        }
    }
    String::from_utf8(bytes).map_err(|e| invalid(e.to_string()))
}

struct Parser<'a> {
    text: &'a str,
    position: usize,
}

impl Parser<'_> {
    fn space(&mut self) {
        while self
            .text
            .as_bytes()
            .get(self.position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.position += 1;
        }
    }

    fn raw(&mut self) -> Result<Range<usize>> {
        self.space();
        let start = self.position;
        let mut stream =
            serde_json::Deserializer::from_str(&self.text[start..]).into_iter::<Box<RawValue>>();
        stream
            .next()
            .ok_or_else(|| invalid("missing value"))?
            .map_err(|e| invalid(e.to_string()))?;
        self.position += stream.byte_offset();
        Ok(start..self.position)
    }

    fn node(&mut self) -> Result<Node> {
        self.space();
        let start = self.position;
        let Some(&opening) = self.text.as_bytes().get(start) else {
            return Err(invalid("missing value"));
        };
        if !matches!(opening, b'{' | b'[') {
            return Ok(Node {
                span: self.raw()?,
                kind: Kind::Leaf,
                commas: Vec::new(),
            });
        }
        let mut objects: Vec<Member> = Vec::new();
        let mut arrays = Vec::new();
        let mut commas = Vec::new();
        self.position += 1;
        self.space();
        let closing = if opening == b'{' { b'}' } else { b']' };
        if self.text.as_bytes().get(self.position) != Some(&closing) {
            loop {
                if opening == b'{' {
                    let key_span = self.raw()?;
                    let key: String = serde_json::from_str(&self.text[key_span.clone()])
                        .map_err(|e| invalid(e.to_string()))?;
                    if objects.iter().any(|m| m.key == key) {
                        return Err(invalid(format!("duplicate object key {key:?}")));
                    }
                    self.space();
                    if self.text.as_bytes().get(self.position) != Some(&b':') {
                        return Err(invalid("missing colon"));
                    }
                    self.position += 1;
                    objects.push(Member {
                        key,
                        start: key_span.start,
                        node: self.node()?,
                    });
                } else {
                    arrays.push(self.node()?);
                }
                self.space();
                if self.text.as_bytes().get(self.position) == Some(&closing) {
                    break;
                }
                if self.text.as_bytes().get(self.position) != Some(&b',') {
                    return Err(invalid("missing comma"));
                }
                commas.push(self.position);
                self.position += 1;
            }
        }
        self.position += 1;
        Ok(Node {
            span: start..self.position,
            kind: if opening == b'{' {
                Kind::Object(objects)
            } else {
                Kind::Array(arrays)
            },
            commas,
        })
    }
}

impl Document {
    fn parse(text: String) -> Result<Self> {
        let masked = mask_comments(&text)?;
        serde_json::from_str::<Box<RawValue>>(&masked).map_err(|e| invalid(e.to_string()))?;
        let root = Parser {
            text: &masked,
            position: 0,
        }
        .node()?;
        if !matches!(root.kind, Kind::Object(_)) {
            return Err(invalid("root must be an object"));
        }
        Ok(Self { text, masked, root })
    }

    fn same(&self, node: &Node, value: &Value) -> bool {
        match (&node.kind, value) {
            (Kind::Object(members), Value::Object(values)) => {
                members.len() == values.len()
                    && members
                        .iter()
                        .all(|m| values.get(&m.key).is_some_and(|v| self.same(&m.node, v)))
            }
            (Kind::Array(nodes), Value::Array(values)) => {
                nodes.len() == values.len()
                    && nodes.iter().zip(values).all(|(n, v)| self.same(n, v))
            }
            (Kind::Leaf, _) => serde_json::from_str::<Value>(&self.masked[node.span.clone()])
                .is_ok_and(|v| v == *value),
            _ => false,
        }
    }

    fn identity(&self, node: &Node, identity: &Identity) -> bool {
        let Kind::Object(members) = &node.kind else {
            return false;
        };
        identity
            .required_keys
            .iter()
            .all(|key| members.iter().any(|m| m.key == *key))
            && identity.fields.iter().all(|(key, value)| {
                let member = members.iter().find(|m| m.key == *key);
                match (member, value) {
                    (None, None) => true,
                    (Some(m), Some(v)) => self.same(&m.node, v),
                    _ => false,
                }
            })
    }

    fn child<'a>(&'a self, parent: &'a Node, step: &JsonStep) -> Result<Option<&'a Node>> {
        match (step, &parent.kind) {
            (JsonStep::Key(key), Kind::Object(members)) => {
                Ok(members.iter().find(|m| m.key == *key).map(|m| &m.node))
            }
            (JsonStep::Match(identity), Kind::Array(nodes)) => {
                let candidates: Vec<_> = nodes
                    .iter()
                    .filter(|n| self.identity(n, identity))
                    .collect();
                if candidates.len() > 1 {
                    return Err(invalid(format!("ambiguous matcher identity {identity:?}")));
                }
                Ok(candidates.first().copied())
            }
            _ => Err(invalid(format!("wrong parent type at {step:?}"))),
        }
    }

    /// Walks `path[..depth]`. Several groups may share one matcher identity
    /// (often no matcher at all); then the owned handler `value` selects the
    /// one group whose final `hooks` key holds it.
    fn resolve(
        &self,
        path: &[JsonStep],
        depth: usize,
        value: Option<&Value>,
    ) -> Result<Option<&Node>> {
        let mut node = &self.root;
        for (i, step) in path[..depth].iter().enumerate() {
            let next = match (step, &node.kind) {
                (JsonStep::Match(identity), Kind::Array(nodes)) => {
                    let candidates: Vec<_> = nodes
                        .iter()
                        .filter(|n| self.identity(n, identity))
                        .collect();
                    if candidates.len() <= 1 {
                        candidates.first().copied()
                    } else {
                        let holders: Vec<_> = match (value, &path[i + 1..]) {
                            (Some(value), [JsonStep::Key(key)]) => candidates
                                .into_iter()
                                .filter(|n| self.holds(n, key, value))
                                .collect(),
                            _ => Vec::new(),
                        };
                        if holders.len() != 1 {
                            return Err(invalid(format!(
                                "ambiguous matcher identity {identity:?}"
                            )));
                        }
                        Some(holders[0])
                    }
                }
                _ => self.child(node, step)?,
            };
            let Some(next) = next else {
                return Ok(None);
            };
            node = next;
        }
        Ok(Some(node))
    }

    fn holds(&self, group: &Node, key: &str, value: &Value) -> bool {
        let Kind::Object(members) = &group.kind else {
            return false;
        };
        members.iter().find(|m| m.key == key).is_some_and(
            |m| matches!(&m.node.kind, Kind::Array(items) if items.iter().any(|n| self.same(n, value))),
        )
    }

    /// Groups reachable through `path`, every match of a matcher identity
    /// included, whose final key holds `value`.
    fn holders(&self, path: &[JsonStep], value: &Value) -> usize {
        let mut nodes = vec![&self.root];
        for (i, step) in path.iter().enumerate() {
            let last = i + 1 == path.len();
            let mut next = Vec::new();
            for node in nodes {
                match (step, &node.kind) {
                    (JsonStep::Key(key), Kind::Object(_)) if last => {
                        if self.holds(node, key, value) {
                            next.push(node);
                        }
                    }
                    (JsonStep::Key(key), Kind::Object(members)) => {
                        next.extend(members.iter().filter(|m| m.key == *key).map(|m| &m.node));
                    }
                    (JsonStep::Match(identity), Kind::Array(items)) => {
                        next.extend(items.iter().filter(|n| self.identity(n, identity)));
                    }
                    _ => {}
                }
            }
            nodes = next;
        }
        nodes.len()
    }

    fn target(
        &self,
        parent: &Node,
        slot: &JsonSlot,
        value: Option<&Value>,
        identity: Option<&Identity>,
    ) -> Result<Option<usize>> {
        match (slot, &parent.kind) {
            (JsonSlot::Member(key), Kind::Object(members)) => {
                Ok(members.iter().position(|m| m.key == *key))
            }
            (JsonSlot::Element(_), Kind::Array(nodes)) => {
                let candidates: Vec<_> = nodes
                    .iter()
                    .enumerate()
                    .filter(|(_, n)| {
                        if let Some(identity) = identity {
                            self.identity(n, identity)
                        } else {
                            value.is_some_and(|v| self.same(n, v))
                        }
                    })
                    .map(|(i, _)| i)
                    .collect();
                if candidates.len() > 1 {
                    return Err(invalid("ambiguous owned array entry"));
                }
                Ok(candidates.first().copied())
            }
            _ => Err(invalid("wrong patch container type")),
        }
    }

    fn entry<'a>(&self, parent: &'a Node, index: usize) -> &'a Node {
        match &parent.kind {
            Kind::Object(m) => &m[index].node,
            Kind::Array(n) => &n[index],
            Kind::Leaf => unreachable!(),
        }
    }

    fn edit(
        &self,
        parent: &Node,
        slot: &JsonSlot,
        index: Option<usize>,
        raw: Option<&str>,
    ) -> Result<String> {
        let mut edits: Vec<(Range<usize>, String)> = Vec::new();
        if let Some(index) = index {
            let node = self.entry(parent, index);
            if let Some(raw) = raw {
                edits.push((node.span.clone(), raw.to_string()));
            } else {
                let start = match &parent.kind {
                    Kind::Object(m) => m[index].start,
                    _ => node.span.start,
                };
                edits.push((start..node.span.end, String::new()));
                if let Some(comma) = parent
                    .commas
                    .get(index)
                    .or_else(|| index.checked_sub(1).and_then(|i| parent.commas.get(i)))
                {
                    edits.push((*comma..*comma + 1, String::new()));
                }
            }
        } else if let Some(raw) = raw {
            match (&parent.kind, slot) {
                (Kind::Object(members), JsonSlot::Member(key)) => {
                    let member = format!("{}:{raw}", serde_json::to_string(key).unwrap());
                    let (position, prefix) = members
                        .last()
                        .map_or((parent.span.start + 1, ""), |m| (m.node.span.end, ","));
                    edits.push((position..position, format!("{prefix}{member}")));
                }
                (Kind::Array(nodes), JsonSlot::Element(index)) => {
                    let index = (*index).min(nodes.len());
                    if let Some(next) = nodes.get(index) {
                        edits.push((next.span.start..next.span.start, format!("{raw},")));
                    } else {
                        let (position, prefix) = nodes
                            .last()
                            .map_or((parent.span.start + 1, ""), |n| (n.span.end, ","));
                        edits.push((position..position, format!("{prefix}{raw}")));
                    }
                }
                _ => return Err(invalid("wrong insertion container")),
            }
        }
        edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
        let mut output = self.text.clone();
        for (range, replacement) in edits {
            output.replace_range(range, &replacement);
        }
        Ok(output)
    }
}

fn validate_patch(patch: &JsonPatch) -> Result<()> {
    if patch.before.is_none() && patch.after.is_none() {
        return Err(Error::new(
            ErrorKind::InvalidPlan,
            "JSON patch must own or change an entry",
        ));
    }
    for step in &patch.parent {
        if let JsonStep::Match(identity) = step {
            if identity.fields.is_empty() && identity.required_keys.is_empty() {
                return Err(Error::new(
                    ErrorKind::InvalidPlan,
                    "matcher identity cannot be empty",
                ));
            }
        }
    }
    Ok(())
}

fn forward_one(
    document: &Document,
    patch: &JsonPatch,
    structural: bool,
    identity: Option<Identity>,
) -> Result<(String, RecordedPatch)> {
    let parent = document
        .resolve(
            &patch.parent,
            patch.parent.len(),
            patch.before.as_ref().or(patch.after.as_ref()),
        )?
        .ok_or_else(|| invalid(format!("missing patch parent {:?}", patch.parent)))?;
    let index = document.target(parent, &patch.slot, patch.before.as_ref(), None)?;
    let before_node = index.map(|i| document.entry(parent, i));
    if match (before_node, &patch.before) {
        (None, None) => false,
        (Some(n), Some(v)) => !document.same(n, v),
        _ => true,
    } {
        return Err(invalid(format!(
            "owned before value differs at {:?}/{:?}",
            patch.parent, patch.slot
        )));
    }
    let assertion = patch.before == patch.after;
    if matches!(patch.slot, JsonSlot::Element(_))
        && !assertion
        && document
            .target(parent, &patch.slot, patch.after.as_ref(), None)?
            .is_some()
    {
        return Err(invalid("new owned entry already exists"));
    }
    let before_raw = before_node.map(|n| document.text[n.span.clone()].to_string());
    let raw = patch
        .after
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap());
    let output = if assertion {
        document.text.clone()
    } else {
        document.edit(parent, &patch.slot, index, raw.as_deref())?
    };
    let mut recorded = patch.clone();
    if let (JsonSlot::Element(_), Some(index)) = (&patch.slot, index) {
        recorded.slot = JsonSlot::Element(index);
    }
    Ok((
        output,
        RecordedPatch {
            patch: recorded,
            before_raw,
            structural,
            identity,
        },
    ))
}

fn empty_for(next: Option<&JsonStep>, slot: &JsonSlot) -> Value {
    match next {
        Some(JsonStep::Match(_)) => Value::Array(Vec::new()),
        Some(JsonStep::Key(_)) => Value::Object(Default::default()),
        None => match slot {
            JsonSlot::Element(_) => Value::Array(Vec::new()),
            JsonSlot::Member(_) => Value::Object(Default::default()),
        },
    }
}

pub(crate) fn prepare(
    bytes: &[u8],
    patches: &[JsonPatch],
) -> Result<(Vec<u8>, Vec<RecordedPatch>)> {
    let mut text = std::str::from_utf8(bytes)
        .map_err(|e| invalid(e.to_string()))?
        .to_string();
    let mut recorded = Vec::new();
    Document::parse(text.clone())?;
    for patch in patches {
        validate_patch(patch)?;
        let owned = patch.before.as_ref().or(patch.after.as_ref());
        for (i, step) in patch.parent.iter().enumerate() {
            let document = Document::parse(text.clone())?;
            let prefix = &patch.parent[..i];
            let parent = document
                .resolve(&patch.parent, i, owned)?
                .ok_or_else(|| invalid("missing ancestor"))?;
            if document.resolve(&patch.parent, i + 1, owned)?.is_some() {
                continue;
            }
            if patch.before.is_some() {
                return Err(invalid(format!("missing owned entry parent {step:?}")));
            }
            let (slot, value, identity) = match step {
                JsonStep::Key(key) => (
                    JsonSlot::Member(key.clone()),
                    empty_for(patch.parent.get(i + 1), &patch.slot),
                    None,
                ),
                JsonStep::Match(identity) => {
                    let Kind::Array(nodes) = &parent.kind else {
                        return Err(invalid("matcher parent is not array"));
                    };
                    let mut value = serde_json::Map::new();
                    for (key, expected) in &identity.fields {
                        if let Some(expected) = expected {
                            value.insert(key.clone(), expected.clone());
                        }
                    }
                    if let Some(JsonStep::Key(key)) = patch.parent.get(i + 1) {
                        value
                            .entry(key.clone())
                            .or_insert_with(|| empty_for(patch.parent.get(i + 2), &patch.slot));
                    }
                    if identity
                        .required_keys
                        .iter()
                        .any(|key| !value.contains_key(key))
                    {
                        return Err(Error::new(
                            ErrorKind::InvalidPlan,
                            "cannot create matcher identity with unspecified required keys",
                        ));
                    }
                    (
                        JsonSlot::Element(nodes.len()),
                        Value::Object(value),
                        Some(identity.clone()),
                    )
                }
            };
            let scaffold = JsonPatch {
                parent: prefix.to_vec(),
                slot,
                before: None,
                after: Some(value),
            };
            let (next, record) = forward_one(&document, &scaffold, true, identity)?;
            text = next;
            recorded.push(record);
        }
        let document = Document::parse(text)?;
        let (next, record) = forward_one(&document, patch, false, None)?;
        text = next;
        recorded.push(record);
    }
    Document::parse(text.clone())?;
    Ok((text.into_bytes(), recorded))
}

pub(crate) fn inverse(bytes: &[u8], patches: &[RecordedPatch]) -> Result<Vec<u8>> {
    let mut text = std::str::from_utf8(bytes)
        .map_err(|e| invalid(e.to_string()))?
        .to_string();
    for record in patches.iter().rev() {
        let document = Document::parse(text)?;
        let patch = &record.patch;
        let owned = patch.after.as_ref().or(patch.before.as_ref());
        let Some(parent) = document.resolve(&patch.parent, patch.parent.len(), owned)? else {
            if record.structural {
                text = document.text;
                continue;
            }
            return Err(invalid(format!(
                "missing owned entry parent {:?}",
                patch.parent
            )));
        };
        let after_index = document.target(
            parent,
            &patch.slot,
            patch.after.as_ref(),
            record.identity.as_ref(),
        )?;
        if record.structural {
            if let Some(index) = after_index {
                if patch
                    .after
                    .as_ref()
                    .is_some_and(|v| document.same(document.entry(parent, index), v))
                {
                    text = document.edit(parent, &patch.slot, Some(index), None)?;
                } else {
                    text = document.text;
                }
            } else {
                text = document.text;
            }
            continue;
        }
        let before_index = document.target(parent, &patch.slot, patch.before.as_ref(), None)?;
        if patch.before == patch.after {
            if !after_index.is_some_and(|index| {
                document.same(document.entry(parent, index), patch.after.as_ref().unwrap())
            }) {
                return Err(invalid("owned assertion entry changed"));
            }
            text = document.text;
            continue;
        }
        match &patch.slot {
            JsonSlot::Member(_) => {
                let current = after_index.map(|i| document.entry(parent, i));
                let same_after = match (current, &patch.after) {
                    (None, None) => true,
                    (Some(n), Some(v)) => document.same(n, v),
                    _ => false,
                };
                let same_before = match (current, &patch.before) {
                    (None, None) => true,
                    (Some(n), Some(v)) => document.same(n, v),
                    _ => false,
                };
                if same_after {
                    text = document.edit(
                        parent,
                        &patch.slot,
                        after_index,
                        record.before_raw.as_deref(),
                    )?;
                } else if same_before {
                    text = document.text;
                } else {
                    return Err(invalid(format!(
                        "owned member changed at {:?}/{:?}",
                        patch.parent, patch.slot
                    )));
                }
            }
            JsonSlot::Element(_) => {
                if after_index.is_some() && before_index.is_some() {
                    return Err(invalid("both before and after owned entries exist"));
                }
                if after_index.is_some() {
                    text = document.edit(
                        parent,
                        &patch.slot,
                        after_index,
                        record.before_raw.as_deref(),
                    )?;
                } else if before_index.is_some() {
                    text = document.text;
                } else if patch.after.is_none() {
                    text =
                        document.edit(parent, &patch.slot, None, record.before_raw.as_deref())?;
                } else {
                    return Err(invalid(format!(
                        "owned handler changed at {:?}",
                        patch.parent
                    )));
                }
            }
        }
    }
    Document::parse(text.clone())?;
    Ok(text.into_bytes())
}

pub(crate) fn validate_records(patches: &[RecordedPatch]) -> Result<()> {
    for record in patches {
        validate_patch(&record.patch)?;
        match (&record.before_raw, &record.patch.before) {
            (None, None) => {}
            (Some(raw), Some(expected)) => {
                let value: Value = serde_json::from_str(&mask_comments(raw)?)
                    .map_err(|e| invalid(e.to_string()))?;
                if value != *expected {
                    return Err(invalid("journal before text differs from its owned value"));
                }
            }
            _ => return Err(invalid("journal before text is missing or unexpected")),
        }
        if record.structural
            && (record.patch.before.is_some()
                || !matches!(record.patch.after, Some(Value::Object(_) | Value::Array(_))))
        {
            return Err(invalid("journal scaffold is invalid"));
        }
    }
    Ok(())
}

pub(crate) fn installed(bytes: &[u8], patches: &[RecordedPatch]) -> Result<()> {
    let document = Document::parse(
        std::str::from_utf8(bytes)
            .map_err(|e| invalid(e.to_string()))?
            .to_string(),
    )?;
    for record in patches.iter().filter(|r| !r.structural) {
        let patch = &record.patch;
        // A retired handler only has to stay absent from every same-identity group.
        if let (None, Some(before)) = (&patch.after, &patch.before) {
            if patch
                .parent
                .iter()
                .any(|step| matches!(step, JsonStep::Match(_)))
            {
                if document.holders(&patch.parent, before) > 0 {
                    return Err(invalid("retired entry reappeared"));
                }
                continue;
            }
        }
        let parent = document
            .resolve(&patch.parent, patch.parent.len(), patch.after.as_ref())?
            .ok_or_else(|| invalid("installed matcher disappeared"))?;
        let after = document.target(parent, &patch.slot, patch.after.as_ref(), None)?;
        if let Some(value) = &patch.after {
            if !after.is_some_and(|i| document.same(document.entry(parent, i), value)) {
                return Err(invalid("installed owned entry changed"));
            }
        } else if document
            .target(parent, &patch.slot, patch.before.as_ref(), None)?
            .is_some()
        {
            return Err(invalid("retired entry reappeared"));
        }
    }
    Ok(())
}

/// Read-only classification uses the same JSONC and duplicate-key validation.
/// The resulting values are for exact structure comparisons, never whole-file writes.
pub(crate) fn parse_value(bytes: &[u8]) -> Result<Value> {
    let text = std::str::from_utf8(bytes).map_err(|e| invalid(e.to_string()))?;
    let document = Document::parse(text.to_string())?;
    serde_json::from_str(&document.masked).map_err(|e| invalid(e.to_string()))
}
