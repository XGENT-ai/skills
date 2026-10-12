use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CellKey {
    pub unit_id: String,
    pub state_id: String,
    pub viewport_id: String,
    pub theme_id: String,
    pub step_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureStatus {
    Captured,
    Missing { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureAttempt {
    pub cell: CellKey,
    pub attempt: u32,
    pub content_hash: String,
    pub status: CaptureStatus,
}

fn valid_matrix(matrix: &[CellKey]) -> bool {
    !matrix.is_empty()
        && matrix.iter().enumerate().all(|(index, cell)| {
            [
                &cell.unit_id,
                &cell.state_id,
                &cell.viewport_id,
                &cell.theme_id,
                &cell.step_id,
            ]
            .iter()
            .all(|id| !id.trim().is_empty())
                && !matrix[..index].contains(cell)
        })
}

fn latest<'a>(history: &'a [CaptureAttempt], cell: &CellKey) -> Option<&'a CaptureAttempt> {
    history
        .iter()
        .filter(|entry| entry.cell == *cell)
        .max_by_key(|entry| entry.attempt)
}

pub fn append_capture(
    matrix: &[CellKey],
    history: &mut Vec<CaptureAttempt>,
    entry: CaptureAttempt,
    finalized: bool,
) -> Result<bool, &'static str> {
    if finalized {
        return Err("Finalized rounds cannot accept capture writes");
    }
    if !valid_matrix(matrix) || !matrix.contains(&entry.cell) || entry.attempt == 0 {
        return Err("Capture must identify a cell in the frozen matrix and a positive attempt");
    }
    if entry.content_hash.len() != 64
        || !entry
            .content_hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("Capture content hash must be a canonical SHA-256 digest");
    }
    if matches!(&entry.status, CaptureStatus::Missing { reason } if reason.trim().is_empty()) {
        return Err("Missing captures need an explicit reason");
    }
    if let Some(existing) = history
        .iter()
        .find(|old| old.cell == entry.cell && old.attempt == entry.attempt)
    {
        return if existing == &entry {
            Ok(false)
        } else {
            Err("A capture attempt cannot be overwritten")
        };
    }
    let next = latest(history, &entry.cell)
        .map_or(Some(1), |old| old.attempt.checked_add(1))
        .ok_or("Capture attempt exceeds the storage limit")?;
    if entry.attempt != next {
        return Err("Capture attempts must advance exactly one step");
    }
    history.push(entry);
    Ok(true)
}

pub fn captures_complete(matrix: &[CellKey], history: &[CaptureAttempt]) -> bool {
    valid_matrix(matrix)
        && matrix.iter().all(|cell| {
            latest(history, cell).is_some_and(|entry| entry.status == CaptureStatus::Captured)
        })
}
