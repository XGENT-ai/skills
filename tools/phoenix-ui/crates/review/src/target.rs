use crate::storage::{error, hash, StoreError};
use pulldown_cmark::{BrokenLink, Event, HeadingLevel, LinkType, Options, Parser, Tag, TagEnd};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
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
        if !id.is_empty() {
            sections[index].references.push(Reference {
                id: id.into_string(),
                destination: destination.to_string(),
                title: title.into_string(),
            });
        }
        if let Some(reference) = destination.strip_prefix("phoenix-unresolved:") {
            missing.push(format!("Unresolved target reference: {reference}"));
            continue;
        }
        if destination.starts_with('#') || destination.is_empty() {
            continue;
        }
        let uri = url::Url::parse(&destination).ok();
        if destination.starts_with("//") || uri.as_ref().is_some_and(|url| url.scheme() != "file") {
            if image {
                missing.push(format!(
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
            missing.push(format!(
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
    })
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
