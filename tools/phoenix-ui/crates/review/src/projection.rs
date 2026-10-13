use crate::storage::StoreError;
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub struct Projection {
    pub bytes: Vec<u8>,
    pub block_hash: String,
}

fn error(code: u8, message: &str) -> StoreError {
    StoreError {
        code,
        message: message.into(),
    }
}

fn positions(source: &[u8], marker: &[u8]) -> Vec<usize> {
    source
        .windows(marker.len())
        .enumerate()
        .filter_map(|(index, bytes)| (bytes == marker).then_some(index))
        .collect()
}

pub fn project_block(
    source: &[u8],
    block_id: &str,
    expected_hash: Option<&str>,
    body: &[u8],
) -> Result<Projection, StoreError> {
    if block_id.is_empty()
        || block_id.contains("--")
        || !block_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err(error(2, "Generated block identity is invalid"));
    }
    let begin = format!("<!-- phoenix-ui:{block_id}:begin -->");
    let end = format!("<!-- phoenix-ui:{block_id}:end -->");
    if !positions(body, begin.as_bytes()).is_empty() || !positions(body, end.as_bytes()).is_empty()
    {
        return Err(error(
            2,
            "Generated content cannot contain its own boundaries",
        ));
    }
    let mut block = begin.as_bytes().to_vec();
    block.push(b'\n');
    block.extend_from_slice(body);
    if !block.ends_with(b"\n") {
        block.push(b'\n');
    }
    block.extend_from_slice(end.as_bytes());
    block.push(b'\n');
    let block_hash = format!("{:x}", Sha256::digest(&block));
    let starts = positions(source, begin.as_bytes());
    let ends = positions(source, end.as_bytes());
    let bytes = match (starts.as_slice(), ends.as_slice()) {
        ([], []) if expected_hash.is_none() => {
            let mut bytes = source.to_vec();
            if !bytes.is_empty() {
                if !bytes.ends_with(b"\n") {
                    bytes.push(b'\n');
                }
                if !bytes.ends_with(b"\n\n") {
                    bytes.push(b'\n');
                }
            }
            bytes.extend_from_slice(&block);
            bytes
        }
        ([start], [end_start]) if start < end_start => {
            let after = end_start + end.len();
            if (*start > 0 && source[start - 1] != b'\n')
                || source.get(start + begin.len()) != Some(&b'\n')
                || source.get(after) != Some(&b'\n')
            {
                return Err(error(3, "Generated boundaries were edited"));
            }
            let current_hash = format!("{:x}", Sha256::digest(&source[*start..after + 1]));
            if expected_hash != Some(current_hash.as_str()) {
                return Err(error(
                    3,
                    "Generated block changed; preserve the edit and import it explicitly",
                ));
            }
            let mut bytes = source[..*start].to_vec();
            bytes.extend_from_slice(&block);
            bytes.extend_from_slice(&source[after + 1..]);
            bytes
        }
        _ => {
            return Err(error(
                3,
                "Generated block is missing, duplicated, or incomplete",
            ))
        }
    };
    Ok(Projection { bytes, block_hash })
}

/// Restores one well-formed generated block to `body` whatever its current text.
/// Callers must first verify that the edited bytes were saved elsewhere.
pub fn rebase_block(source: &[u8], block_id: &str, body: &[u8]) -> Result<Projection, StoreError> {
    let begin = format!("<!-- phoenix-ui:{block_id}:begin -->");
    let end = format!("<!-- phoenix-ui:{block_id}:end -->");
    match (
        positions(source, begin.as_bytes()).as_slice(),
        positions(source, end.as_bytes()).as_slice(),
    ) {
        ([start], [end_start]) if start < end_start && source.len() > end_start + end.len() => {
            let current = format!(
                "{:x}",
                Sha256::digest(&source[*start..end_start + end.len() + 1])
            );
            project_block(source, block_id, Some(&current), body)
        }
        _ => Err(error(
            3,
            "Generated block is missing, duplicated, or incomplete",
        )),
    }
}
