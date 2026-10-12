use crate::{
    capture::CellKey,
    matrix::{FrozenMatrix, Viewport},
    storage::{error, hash, StoreError},
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{Cursor, Read},
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkingTree {
    Clean,
    Dirty { sha256: String },
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TestedVersion {
    pub head: Option<String>,
    pub working_tree: WorkingTree,
    pub build_id: Option<String>,
}

impl TestedVersion {
    pub fn validate(&self) -> Result<(), StoreError> {
        if self
            .head
            .as_ref()
            .is_some_and(|head| head.len() != 40 || !head.bytes().all(|c| c.is_ascii_hexdigit()))
            || self
                .build_id
                .as_ref()
                .is_some_and(|id| id.trim().is_empty())
        {
            return Err(error(
                2,
                "Version needs a full HEAD or an explicit build ID",
            ));
        }
        if matches!(&self.working_tree, WorkingTree::Dirty{sha256} if !digest(sha256)) {
            return Err(error(
                2,
                "Dirty version fingerprint is not a SHA-256 digest",
            ));
        }
        Ok(())
    }
    pub fn known(&self) -> bool {
        self.validate().is_ok()
            && (self.build_id.is_some()
                || (self.head.is_some() && !matches!(self.working_tree, WorkingTree::Unknown)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileEvidence {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    Browser,
    Simulator,
    Native,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Interaction {
    pub action: String,
    pub observed: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HumanCheck {
    pub opened: bool,
    pub current_product: bool,
    pub current_version: bool,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Observation {
    Missing {
        reason: String,
    },
    Captured {
        image: FileEvidence,
        origin: Origin,
        tool: String,
        address: String,
        #[serde(rename = "takenAt")]
        taken_at: String,
        viewport: Box<Viewport>,
        #[serde(rename = "testedVersion")]
        tested_version: Box<TestedVersion>,
        interactions: Vec<Interaction>,
        #[serde(rename = "humanCheck")]
        human_check: HumanCheck,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureInput {
    pub schema_version: u32,
    pub round_id: String,
    pub cell: CellKey,
    pub attempt: u32,
    pub observation: Observation,
}

#[derive(Debug)]
pub struct VerifiedCapture {
    pub input: CaptureInput,
    pub content_hash: String,
    pub image_pixels: Option<[u32; 2]>,
    pub version_known: bool,
}

pub struct CaptureContext<'a> {
    pub project: &'a Path,
    pub round_directory: &'a Path,
    pub round_id: &'a str,
    pub matrix: &'a FrozenMatrix,
    pub version: &'a TestedVersion,
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

fn normal_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path.components().all(
            |part| matches!(part,Component::Normal(name) if !name.to_string_lossy().contains(':')),
        )
}

fn source_path(root: &Path, relative: &Path) -> Result<PathBuf, StoreError> {
    if !normal_path(relative) {
        return Err(error(2, "Capture paths must stay inside the project"));
    }
    let mut current = root.to_path_buf();
    for part in relative.components() {
        current.push(part.as_os_str());
        if fs::symlink_metadata(&current)
            .map_err(|e| error(4, e))?
            .file_type()
            .is_symlink()
        {
            return Err(error(2, "Capture files cannot follow symbolic links"));
        }
    }
    if !current.is_file() {
        return Err(error(2, "Capture image is not a regular file"));
    }
    Ok(current)
}

pub(crate) fn timestamp(value: &str) -> bool {
    if !value.is_ascii() || !(20..=128).contains(&value.len()) {
        return false;
    }
    let Some((date, time)) = value.split_once('T') else {
        return false;
    };
    let digits =
        |value: &str, len: usize| value.len() == len && value.bytes().all(|c| c.is_ascii_digit());
    let date: Vec<_> = date.split('-').collect();
    if date.len() != 3 || !digits(date[0], 4) || !digits(date[1], 2) || !digits(date[2], 2) {
        return false;
    }
    let (year, month, day) = (
        date[0].parse::<u32>().unwrap(),
        date[1].parse::<u32>().unwrap(),
        date[2].parse::<u32>().unwrap(),
    );
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) => {
            29
        }
        2 => 28,
        _ => return false,
    };
    if day == 0 || day > days {
        return false;
    }
    let clock = if let Some(clock) = time.strip_suffix('Z') {
        clock
    } else {
        let Some(offset) = time.rfind(['+', '-']) else {
            return false;
        };
        let zone = &time[offset + 1..];
        let Some((hours, minutes)) = zone.split_once(':') else {
            return false;
        };
        if !digits(hours, 2)
            || !digits(minutes, 2)
            || hours.parse::<u32>().unwrap() > 23
            || minutes.parse::<u32>().unwrap() > 59
        {
            return false;
        }
        &time[..offset]
    };
    let clock: Vec<_> = clock.split(':').collect();
    if clock.len() != 3
        || !digits(clock[0], 2)
        || !digits(clock[1], 2)
        || clock[0].parse::<u32>().unwrap() > 23
        || clock[1].parse::<u32>().unwrap() > 59
    {
        return false;
    }
    let seconds = if let Some((seconds, fraction)) = clock[2].split_once('.') {
        if fraction.is_empty() || !fraction.bytes().all(|c| c.is_ascii_digit()) {
            return false;
        }
        seconds
    } else {
        clock[2]
    };
    digits(seconds, 2) && seconds.parse::<u32>().unwrap() <= 60
}

fn png_pixels(file: &Path, sha256: &str, viewport: &Viewport) -> Result<[u32; 2], StoreError> {
    const MAX_BYTES: usize = 128 * 1024 * 1024;
    let mut bytes = Vec::new();
    File::open(file)
        .map_err(|e| error(4, e))?
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| error(4, e))?;
    if bytes.len() > MAX_BYTES {
        return Err(error(
            2,
            "Capture image exceeds the decoding resource limit",
        ));
    }
    if !digest(sha256) || hash(&bytes) != sha256 {
        return Err(error(2, "Capture image hash does not match the file"));
    }
    let mut decoder = png::Decoder::new(Cursor::new(&bytes));
    decoder.set_limits(png::Limits { bytes: MAX_BYTES });
    let mut reader = decoder.read_info().map_err(|e| error(2, e))?;
    if reader.info().animation_control.is_some() {
        return Err(error(2, "Capture PNG must contain one static screenshot"));
    }
    let (width, height) = (reader.info().width, reader.info().height);
    let expected = [
        (f64::from(viewport.width) * viewport.dpr).round() as u32,
        (f64::from(viewport.height) * viewport.dpr).round() as u32,
    ];
    if width != expected[0]
        || (!viewport.full_page && height != expected[1])
        || (viewport.full_page && height < expected[1])
    {
        return Err(error(
            2,
            "Capture image dimensions do not match the frozen viewport and DPR",
        ));
    }
    let size = reader
        .output_buffer_size()
        .filter(|size| *size <= MAX_BYTES)
        .ok_or_else(|| error(2, "Capture decoded pixels exceed the resource limit"))?;
    reader
        .next_frame(&mut vec![0; size])
        .map_err(|e| error(2, e))?;
    reader.finish().map_err(|e| error(2, e))?;
    Ok([width, height])
}

pub fn verify(
    input: CaptureInput,
    context: &CaptureContext<'_>,
) -> Result<VerifiedCapture, StoreError> {
    context.matrix.validate()?;
    context.version.validate()?;
    let number = context
        .round_id
        .strip_prefix('r')
        .and_then(|n| n.parse::<u32>().ok());
    if number.is_none_or(|n| n == 0 || format!("r{n}") != context.round_id)
        || !normal_path(context.round_directory)
        || context
            .round_directory
            .file_name()
            .and_then(|name| name.to_str())
            != Some(context.round_id)
        || !context
            .round_directory
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".ui-captures"))
    {
        return Err(error(
            2,
            "Capture context needs the current round directory",
        ));
    }
    if input.schema_version != 1 || input.round_id != context.round_id {
        return Err(error(2, "Capture input is not for this round or schema"));
    }
    let filename = context.matrix.screenshot_name(&input.cell, input.attempt)?;
    let cell = context
        .matrix
        .cells
        .iter()
        .find(|cell| cell.key == input.cell)
        .unwrap();
    let pixels = match &input.observation {
        Observation::Missing { reason } => {
            if reason.trim().is_empty() {
                return Err(error(2, "Missing capture needs an explicit reason"));
            }
            None
        }
        Observation::Captured {
            image,
            tool,
            address,
            taken_at,
            viewport,
            tested_version,
            interactions,
            human_check,
            ..
        } => {
            if viewport.as_ref() != &cell.viewport || tested_version.as_ref() != context.version {
                return Err(error(
                    2,
                    "Capture cannot change the frozen viewport or tested version",
                ));
            }
            if tool.trim().is_empty() || url::Url::parse(address).is_err() || !timestamp(taken_at) {
                return Err(error(
                    2,
                    "Capture needs its actual tool, address and RFC3339 time",
                ));
            }
            if !human_check.opened
                || !human_check.current_product
                || !human_check.current_version
                || human_check.notes.trim().is_empty()
            {
                return Err(error(
                    2,
                    "Captured images need the host's explicit per-image provenance check",
                ));
            }
            if (input.cell.step_id != "initial" && interactions.is_empty())
                || interactions
                    .iter()
                    .any(|step| step.action.trim().is_empty() || step.observed.trim().is_empty())
            {
                return Err(error(
                    2,
                    "Interaction frames need actions and observed results",
                ));
            }
            let expected = context.round_directory.join(filename);
            if Path::new(&image.path) != expected {
                return Err(error(
                    2,
                    "Capture image must use this round's preallocated attempt name",
                ));
            }
            let root = fs::canonicalize(context.project).map_err(|e| error(4, e))?;
            Some(png_pixels(
                &source_path(&root, Path::new(&image.path))?,
                &image.sha256,
                viewport,
            )?)
        }
    };
    let content_hash = hash(&serde_json::to_vec(&input).map_err(|e| error(2, e))?);
    Ok(VerifiedCapture {
        input,
        content_hash,
        image_pixels: pixels,
        version_known: context.version.known(),
    })
}
