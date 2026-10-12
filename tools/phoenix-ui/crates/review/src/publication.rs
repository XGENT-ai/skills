use crate::storage::{error, hash, snapshot, CycleLock, Snapshot, StoreError};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct ReportFile {
    pub path: PathBuf,
    pub expected_hash: Option<String>,
    pub bytes: Vec<u8>,
}

#[derive(Deserialize)]
struct RoundHeader {
    finalization: Option<Finalization>,
}

#[derive(Deserialize)]
struct Finalization {
    status: String,
}

pub(crate) fn is_finalized(bytes: &[u8], code: u8) -> Result<bool, StoreError> {
    let header: RoundHeader = serde_json::from_slice(bytes).map_err(|e| error(code, e))?;
    Ok(header.finalization.is_some_and(|f| f.status == "finalized"))
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Write {
    path: PathBuf,
    before: Option<Vec<u8>>,
    bytes: Vec<u8>,
    sha256: String,
}

impl Write {
    fn before_hash(&self) -> Option<String> {
        self.before.as_deref().map(hash)
    }
}

#[derive(Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum Operation {
    #[default]
    Finalize,
    Draft,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Journal {
    schema_version: u32,
    #[serde(default)]
    operation: Operation,
    round: PathBuf,
    writes: Vec<Write>,
    sources: Vec<(PathBuf, String)>,
}

impl CycleLock {
    fn bytes(&self, relative: &Path) -> Result<Option<Vec<u8>>, StoreError> {
        match fs::read(self.path(relative)?) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(error(4, e)),
        }
    }

    /// The primary state file of an unfinished publication, if any.
    pub(crate) fn pending_publication(&self) -> Result<Option<PathBuf>, StoreError> {
        Ok(self.journal()?.map(|(journal, _)| journal.round))
    }

    pub(crate) fn publication_pending(&self) -> Result<bool, StoreError> {
        Ok(self.bytes(&self.publication_path())?.is_some())
    }

    fn check_file(&self, path: &Path, expected: Option<&str>) -> Result<(), StoreError> {
        let current = self.bytes(path)?.as_deref().map(hash);
        if current.as_deref() != expected {
            return Err(error(3, "A publication file changed"));
        }
        Ok(())
    }

    fn validate_journal(&self, journal: &Journal, code: u8) -> Result<(), StoreError> {
        if journal.schema_version != 1 || journal.writes.is_empty() {
            return Err(error(code, "Unsupported or incomplete publication journal"));
        }
        let journal_path = self.path(&self.publication_path())?;
        let mut paths = HashSet::new();
        for write in &journal.writes {
            let path = self.path(&write.path)?;
            if path == journal_path || !paths.insert(path) || hash(&write.bytes) != write.sha256 {
                return Err(error(code, "Invalid publication path or content digest"));
            }
        }
        for (path, _) in &journal.sources {
            if paths.contains(&self.path(path)?) || self.path(path)? == journal_path {
                return Err(error(code, "Frozen sources cannot be publication outputs"));
            }
        }
        let round = journal.writes.last().unwrap();
        if round.path != journal.round {
            return Err(error(
                code,
                "Round commit must be the last publication write",
            ));
        }
        let new = snapshot(round.bytes.clone(), code)?;
        let old = round
            .before
            .as_ref()
            .map(|bytes| snapshot(bytes.clone(), code))
            .transpose()?;
        let expected_revision = old
            .as_ref()
            .map_or(Some(1), |old| old.revision.checked_add(1));
        let old_finalized = round
            .before
            .as_deref()
            .map(|bytes| is_finalized(bytes, code))
            .transpose()?
            .unwrap_or(false);
        let new_finalized = is_finalized(&round.bytes, code)?;
        let transition = match journal.operation {
            Operation::Finalize => old.is_some() && new_finalized,
            Operation::Draft => !new_finalized,
        };
        if expected_revision != Some(new.revision) || old_finalized || !transition {
            return Err(error(
                code,
                "Publication must advance exactly one revision with a valid draft/finalize transition",
            ));
        }
        Ok(())
    }

    fn journal(&self) -> Result<Option<(Journal, String)>, StoreError> {
        let Some(bytes) = self.bytes(&self.publication_path())? else {
            return Ok(None);
        };
        let journal = serde_json::from_slice(&bytes).map_err(|e| error(4, e))?;
        self.validate_journal(&journal, 4)?;
        Ok(Some((journal, hash(&bytes))))
    }

    pub fn prepare_publication(
        &self,
        round: &Path,
        expected: &Snapshot,
        finalized: &[u8],
        reports: &[ReportFile],
        sources: &[(PathBuf, String)],
    ) -> Result<(), StoreError> {
        self.prepare(
            round,
            Some(expected),
            finalized,
            reports,
            sources,
            Operation::Finalize,
        )
    }

    pub fn prepare_draft(
        &self,
        primary: &Path,
        expected: Option<&Snapshot>,
        next: &[u8],
        files: &[ReportFile],
        sources: &[(PathBuf, String)],
    ) -> Result<(), StoreError> {
        self.prepare(primary, expected, next, files, sources, Operation::Draft)
    }

    fn prepare(
        &self,
        round: &Path,
        expected: Option<&Snapshot>,
        next: &[u8],
        reports: &[ReportFile],
        sources: &[(PathBuf, String)],
        operation: Operation,
    ) -> Result<(), StoreError> {
        if self.publication_pending()? {
            return Err(error(3, "A pending publication must be recovered first"));
        }
        self.compare(round, expected, sources)?;
        if let Some(expected) = expected {
            if is_finalized(&expected.bytes, 4)? {
                return Err(error(3, "Finalized round facts are immutable"));
            }
        }
        let mut writes = Vec::new();
        let mut paths = HashSet::from([self.path(round)?]);
        let journal_path = self.path(&self.publication_path())?;
        for report in reports {
            let path = self.path(&report.path)?;
            if path == journal_path || !paths.insert(path) {
                return Err(error(
                    2,
                    "Report paths must be distinct from internal state",
                ));
            }
            let before = self.bytes(&report.path)?;
            if before.as_deref().map(hash) != report.expected_hash {
                return Err(error(3, "A publication file changed"));
            }
            writes.push(Write {
                path: report.path.clone(),
                before,
                bytes: report.bytes.clone(),
                sha256: hash(&report.bytes),
            });
        }
        writes.push(Write {
            path: round.to_owned(),
            before: expected.map(|old| old.bytes.clone()),
            bytes: next.to_vec(),
            sha256: hash(next),
        });
        let journal = Journal {
            schema_version: 1,
            operation,
            round: round.to_owned(),
            writes,
            sources: sources.to_vec(),
        };
        self.validate_journal(&journal, 2)?;
        let bytes = serde_json::to_vec_pretty(&journal).map_err(|e| error(2, e))?;
        self.write_atomic(&self.publication_path(), &bytes, || {
            self.check_file(&self.publication_path(), None)?;
            self.verify_sources(sources)?;
            for write in &journal.writes {
                self.check_file(&write.path, write.before_hash().as_deref())?;
            }
            Ok(())
        })
    }

    fn check_journal(&self, journal: &Journal, digest: &str) -> Result<(), StoreError> {
        self.check_file(&self.publication_path(), Some(digest))?;
        self.verify_sources(&journal.sources)?;
        for write in &journal.writes {
            let current = self.bytes(&write.path)?.as_deref().map(hash);
            if current != write.before_hash() && current.as_deref() != Some(&write.sha256) {
                return Err(error(3, "Publication conflicts with a later file edit"));
            }
        }
        Ok(())
    }

    pub fn recover_publication(&self) -> Result<Option<Snapshot>, StoreError> {
        self.recover_with(|_, _| {})
    }

    fn recover_with(
        &self,
        mut after_write: impl FnMut(usize, &Path),
    ) -> Result<Option<Snapshot>, StoreError> {
        let Some((journal, digest)) = self.journal()? else {
            return Ok(None);
        };
        self.check_journal(&journal, &digest)?;
        after_write(0, &self.publication_path());
        for (index, write) in journal.writes.iter().enumerate() {
            let current = self.bytes(&write.path)?.as_deref().map(hash);
            if current.as_deref() != Some(&write.sha256) {
                self.write_atomic(&write.path, &write.bytes, || {
                    self.check_journal(&journal, &digest)?;
                    self.check_file(&write.path, current.as_deref())
                })?;
            }
            after_write(index + 1, &write.path);
        }
        self.check_journal(&journal, &digest)?;
        fs::remove_file(self.path(&self.publication_path())?).map_err(|e| error(4, e))?;
        self.read(&journal.round)
    }

    pub fn read_finalized(&self, round: &Path) -> Result<Option<Snapshot>, StoreError> {
        let journal = self.journal()?;
        let Some(snapshot) = self.read(round)? else {
            return Ok(None);
        };
        if !is_finalized(&snapshot.bytes, 4)? {
            return Ok(None);
        }
        if let Some((journal, digest)) = journal {
            if journal.round == round {
                self.check_journal(&journal, &digest)?;
                for write in &journal.writes {
                    self.check_file(&write.path, Some(&write.sha256))?;
                }
            }
        }
        Ok(Some(snapshot))
    }
}

#[cfg(test)]
mod tests;
