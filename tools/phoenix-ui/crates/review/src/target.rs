use crate::storage::{error, hash, StoreError};
use pulldown_cmark::{BrokenLink, Event, HeadingLevel, LinkType, Options, Parser, Tag, TagEnd};
use sha2::{Digest, Sha256};
use std::{
    collections::{btree_map::Entry, BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Debug)]
pub struct ExternalSource {
    pub path: PathBuf,
    pub logical_path: String,
}

#[derive(Debug)]
pub struct Section {
    pub name: String,
    pub text: String,
    pub references: Vec<Reference>,
}

#[derive(Debug)]
pub struct Reference {
    pub id: String,
    pub destination: String,
    pub title: String,
}

#[derive(Debug)]
pub struct Attachment {
    pub logical_path: String,
    pub source: PathBuf,
    pub sha256: Option<String>,
    pub bytes: Option<Vec<u8>>,
}

#[derive(Debug)]
pub struct TargetSnapshot {
    pub algorithm: &'static str,
    pub fingerprint: String,
    pub source_hash: String,
    pub sections: Vec<Section>,
    pub attachments: Vec<Attachment>,
    pub missing: Vec<String>,
    /// Missing items cited only in rows of the unit table, by unit ID. Any other
    /// missing item, such as one in the shared expectations, affects every unit.
    pub unit_missing: BTreeMap<String, BTreeSet<String>>,
}

impl TargetSnapshot {
    /// The missing expectation sources `unit` depends on.
    pub fn missing_for<'a>(&'a self, unit: &'a str) -> impl Iterator<Item = &'a String> {
        self.missing.iter().filter(move |item| {
            self.unit_missing
                .get(*item)
                .is_none_or(|units| units.contains(unit))
        })
    }
}

pub fn freeze(
    target: &Path,
    project: &Path,
    external: &[ExternalSource],
) -> Result<TargetSnapshot, StoreError> {
    let root = fs::canonicalize(project).map_err(|e| error(4, e))?;
    let target = fs::canonicalize(target).map_err(|e| error(4, e))?;
    if !target.starts_with(&root) {
        return Err(error(2, "Target must belong to the project"));
    }
    let sources = registered_sources(&root, external)?;
    let bytes = fs::read(&target).map_err(|e| error(4, e))?;
    let source_hash = hash(&bytes);
    let text = String::from_utf8(bytes)
        .map_err(|e| error(2, e))?
        .replace("\r\n", "\n");
    let ranges = section_ranges(&text)?;
    let mut sections = ranges
        .iter()
        .map(|(name, start, end)| Section {
            name: name.clone(),
            text: text[*start..*end].to_string(),
            references: Vec::new(),
        })
        .collect::<Vec<_>>();
    let rows = unit_rows(&text, &ranges);
    // `None` marks a missing item that some citation outside a unit row shares.
    let mut owners = BTreeMap::<String, Option<BTreeSet<String>>>::new();
    let mut missing = Vec::new();
    for name in ["单元", "规范要点"] {
        if !sections.iter().any(|section| section.name == name) {
            missing.push(format!("Missing expectation section: {name}"));
        }
    }
    let callback = |link: BrokenLink<'_>| {
        Some((
            format!("phoenix-unresolved:{}", link.reference).into(),
            "".into(),
        ))
    };
    let parser = Parser::new_with_broken_link_callback(
        &text,
        Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS,
        Some(callback),
    );
    let mut attachments = BTreeMap::<String, Attachment>::new();
    for (event, range) in parser.into_offset_iter() {
        let (image, link_type, destination, title, id) = match event {
            Event::Start(Tag::Image {
                link_type,
                dest_url,
                title,
                id,
            }) => (true, link_type, dest_url, title, id),
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }) => (false, link_type, dest_url, title, id),
            _ => continue,
        };
        let Some(index) = ranges
            .iter()
            .position(|(_, start, end)| range.start >= *start && range.start < *end)
        else {
            continue;
        };
        // An ordinary bracketed phrase is not a broken shortcut link.
        if !image && link_type == LinkType::ShortcutUnknown {
            continue;
        }
        let row = rows
            .iter()
            .find(|(start, end, _)| range.start >= *start && range.start < *end)
            .map(|(_, _, unit)| unit.as_str());
        let mut note = |message: String| {
            match (owners.entry(message.clone()), row) {
                (Entry::Vacant(entry), Some(unit)) => {
                    entry.insert(Some(BTreeSet::from([unit.to_owned()])));
                }
                (Entry::Occupied(mut entry), Some(unit)) => {
                    if let Some(units) = entry.get_mut() {
                        units.insert(unit.to_owned());
                    }
                }
                (entry, None) => *entry.or_default() = None,
            }
            missing.push(message);
        };
        if !id.is_empty() {
            sections[index].references.push(Reference {
                id: id.into_string(),
                destination: destination.to_string(),
                title: title.into_string(),
            });
        }
        if let Some(reference) = destination.strip_prefix("phoenix-unresolved:") {
            note(format!("Unresolved target reference: {reference}"));
            continue;
        }
        if destination.starts_with('#') || destination.is_empty() {
            continue;
        }
        let uri = url::Url::parse(&destination).ok();
        if destination.starts_with("//") || uri.as_ref().is_some_and(|url| url.scheme() != "file") {
            if image {
                note(format!(
                    "Remote target image needs a local export: {destination}"
                ));
            }
            continue;
        }
        let local = if let Some(uri) = uri {
            uri.to_file_path()
                .map_err(|_| error(2, "Invalid local file URL"))?
        } else {
            let name = destination.split(['#', '?']).next().unwrap();
            let decoded = percent_encoding::percent_decode_str(name)
                .decode_utf8()
                .map_err(|e| error(2, e))?;
            if decoded.contains('\0') {
                return Err(error(2, "Invalid target path"));
            }
            target.parent().unwrap().join(decoded.as_ref())
        };
        let attachment = read_attachment(&root, &normalize(&local), &sources)?;
        if attachment.sha256.is_none() {
            note(format!(
                "Unreadable target attachment: {}",
                attachment.logical_path
            ));
        }
        match attachments.entry(attachment.logical_path.clone()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(attachment);
            }
            std::collections::btree_map::Entry::Occupied(entry) => {
                if entry.get().source != attachment.source {
                    return Err(error(2, "Target sources share a logical attachment path"));
                }
            }
        }
    }
    let attachments = attachments.into_values().collect::<Vec<_>>();
    let mut digest = Sha256::new();
    part(&mut digest, "phoenix-ui-target-v1");
    digest.update((sections.len() as u64).to_be_bytes());
    for section in &sections {
        part(&mut digest, &section.name);
        part(&mut digest, &section.text);
        digest.update((section.references.len() as u64).to_be_bytes());
        for reference in &section.references {
            part(&mut digest, &reference.id);
            part(&mut digest, &reference.destination);
            part(&mut digest, &reference.title);
        }
    }
    digest.update((attachments.len() as u64).to_be_bytes());
    for attachment in &attachments {
        part(&mut digest, &attachment.logical_path);
        part(&mut digest, attachment.sha256.as_deref().unwrap_or(""));
    }
    Ok(TargetSnapshot {
        algorithm: "phoenix-ui-target-v1",
        fingerprint: format!("{:x}", digest.finalize()),
        source_hash,
        sections,
        attachments,
        missing,
        unit_missing: owners
            .into_iter()
            .filter_map(|(item, units)| Some((item, units?)))
            .collect(),
    })
}

/// Body rows of the unit table with the unit ID each row defines, as byte ranges
/// of `text`, so a source cited in one row is attributed to that unit alone.
fn unit_rows(text: &str, ranges: &[(String, usize, usize)]) -> Vec<(usize, usize, String)> {
    let Some((_, start, end)) = ranges.iter().find(|(name, _, _)| name == "单元") else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    let mut column = None;
    let mut row = None::<(usize, Vec<String>)>;
    let mut cell = None::<String>;
    for (event, range) in
        Parser::new_ext(&text[*start..*end], Options::ENABLE_TABLES).into_offset_iter()
    {
        match event {
            Event::Start(Tag::TableHead | Tag::TableRow) => row = Some((range.start, Vec::new())),
            Event::Start(Tag::TableCell) => cell = Some(String::new()),
            Event::Text(value) | Event::Code(value) => {
                if let Some(cell) = &mut cell {
                    cell.push_str(&value);
                }
            }
            Event::End(TagEnd::TableCell) => {
                if let (Some(text), Some((_, cells))) = (cell.take(), &mut row) {
                    cells.push(text.trim().to_owned());
                }
            }
            Event::End(TagEnd::TableHead) => {
                column = row
                    .take()
                    .and_then(|(_, cells)| cells.iter().position(|name| name == "单元"));
            }
            Event::End(TagEnd::TableRow) => {
                let unit = row.take().zip(column).and_then(|((from, cells), column)| {
                    let id = cells.get(column)?.split_whitespace().next()?;
                    Some((start + from, start + range.end, id.to_owned()))
                });
                rows.extend(unit);
            }
            _ => (),
        }
    }
    rows
}

fn part(digest: &mut Sha256, value: &str) {
    digest.update((value.len() as u64).to_be_bytes());
    digest.update(value.as_bytes());
}

fn normalize(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => (),
            Component::ParentDir => {
                result.pop();
            }
            other => result.push(other.as_os_str()),
        }
    }
    result
}

fn logical(path: &Path) -> Result<String, StoreError> {
    let mut parts = Vec::new();
    for component in path.components() {
        let Component::Normal(name) = component else {
            return Err(error(
                2,
                "Logical target paths must stay inside the snapshot",
            ));
        };
        let name = name
            .to_str()
            .ok_or_else(|| error(2, "Target paths must be UTF-8"))?;
        if name.contains(':') {
            return Err(error(
                2,
                "Logical target paths cannot contain a drive or stream",
            ));
        }
        parts.push(name);
    }
    if parts.is_empty() {
        return Err(error(2, "Logical target path is empty"));
    }
    Ok(parts.join("/"))
}

fn registered_sources(
    root: &Path,
    external: &[ExternalSource],
) -> Result<BTreeMap<PathBuf, String>, StoreError> {
    let mut sources = BTreeMap::new();
    let mut names = std::collections::BTreeSet::new();
    for entry in external {
        let name = logical(Path::new(&entry.logical_path.replace('\\', "/")))?;
        let source = fs::canonicalize(&entry.path).map_err(|e| error(4, e))?;
        if !names.insert(name.clone()) || sources.insert(source.clone(), name.clone()).is_some() {
            return Err(error(2, "Duplicate external target source"));
        }
        if fs::canonicalize(root.join(&name)).is_ok_and(|path| path != source) {
            return Err(error(2, "External source aliases a project file"));
        }
    }
    Ok(sources)
}

fn read_attachment(
    root: &Path,
    path: &Path,
    external: &BTreeMap<PathBuf, String>,
) -> Result<Attachment, StoreError> {
    let canonical = match fs::canonicalize(path) {
        Ok(path) => Some(path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(error(4, e)),
    };
    let (logical_path, source) = if let Some(canonical) = canonical {
        if canonical.starts_with(root) {
            let relative = path
                .strip_prefix(root)
                .unwrap_or_else(|_| canonical.strip_prefix(root).unwrap());
            (logical(relative)?, canonical)
        } else if let Some(name) = external.get(&canonical) {
            (name.clone(), canonical)
        } else {
            return Err(error(
                2,
                "Outside target attachments need an explicit source registration",
            ));
        }
    } else {
        let mut resolved = path.to_path_buf();
        let mut suffix = Vec::new();
        while !resolved.exists() {
            let name = resolved
                .file_name()
                .ok_or_else(|| error(2, "Target attachment has no existing project ancestor"))?
                .to_os_string();
            suffix.push(name);
            resolved.pop();
        }
        resolved = fs::canonicalize(resolved).map_err(|e| error(4, e))?;
        for name in suffix.into_iter().rev() {
            resolved.push(name);
        }
        let relative = if let Ok(relative) = path.strip_prefix(root) {
            relative
        } else {
            resolved.strip_prefix(root).map_err(|_| {
                error(
                    2,
                    "Outside target attachments need an explicit source registration",
                )
            })?
        };
        if !resolved.starts_with(root) {
            return Err(error(
                2,
                "Outside target attachments need an explicit source registration",
            ));
        }
        (logical(relative)?, resolved)
    };
    let bytes = if source.is_file() {
        Some(fs::read(&source).map_err(|e| error(4, e))?)
    } else {
        None
    };
    Ok(Attachment {
        logical_path,
        sha256: bytes.as_ref().map(|bytes| hash(bytes)),
        bytes,
        source,
    })
}

pub(crate) fn section_ranges(text: &str) -> Result<Vec<(String, usize, usize)>, StoreError> {
    let mut headings = Vec::new();
    let mut depth = 0;
    let mut current: Option<(usize, HeadingLevel, String)> = None;
    for (event, range) in Parser::new_ext(text, Options::ENABLE_TABLES).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. })
                if depth == 0 && matches!(level, HeadingLevel::H1 | HeadingLevel::H2) =>
            {
                current = Some((range.start, level, String::new()));
                depth += 1;
            }
            Event::Start(_) => depth += 1,
            Event::Text(value) | Event::Code(value) => {
                if let Some((_, _, name)) = &mut current {
                    name.push_str(&value);
                }
            }
            Event::End(tag) => {
                if depth == 1 && matches!(tag, TagEnd::Heading(HeadingLevel::H1 | HeadingLevel::H2))
                {
                    headings.push(current.take().unwrap());
                }
                depth -= 1;
            }
            _ => (),
        }
    }
    let mut sections = Vec::new();
    for (index, (start, level, name)) in headings.iter().enumerate() {
        if *level != HeadingLevel::H2 || !["单元", "规范要点"].contains(&name.as_str()) {
            continue;
        }
        if sections.iter().any(|(old, _, _)| old == name) {
            return Err(error(2, "Duplicate expectation section"));
        }
        sections.push((
            name.clone(),
            *start,
            headings
                .get(index + 1)
                .map_or(text.len(), |heading| heading.0),
        ));
    }
    Ok(sections)
}
