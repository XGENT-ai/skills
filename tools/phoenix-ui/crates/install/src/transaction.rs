use crate::{
    disk::{self, Snapshot, State},
    json::{self, RecordedPatch},
    sha256, Change, Checkpoint, Error, ErrorKind, ExpectedFile, Manifest, Permissions, Plan,
    Result, RollbackPolicy,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

const ACTIVE: &str = ".phoenix-ui/install/active.json";
const JOURNALS: &str = ".phoenix-ui/install/journals";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ownership {
    pub path: String,
    pub sha256: Option<String>,
    pub shared_entries: bool,
    pub rollback: RollbackPolicy,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Receipt {
    pub schema_version: u32,
    pub transaction_id: String,
    pub manifest: Manifest,
    pub plan_sha256: String,
    pub journal: String,
    pub ownership_sha256: String,
    pub ownership: Vec<Ownership>,
}

#[derive(Debug)]
pub struct VerifiedInstallation {
    pub receipt: Receipt,
    pub receipt_sha256: String,
    pub json_patches: BTreeMap<String, Vec<crate::JsonPatch>>,
}

#[derive(Debug, Serialize)]
pub struct Preflight {
    pub transaction_id: String,
    pub writes: Vec<String>,
    pub shared_json: Vec<String>,
    pub removals: Vec<String>,
    pub already_applied: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Recovery {
    Clean,
    RolledBack(String),
    Activated(String),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum Phase {
    Applying,
    Committed,
    RollingBack,
    RolledBack,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Operation {
    path: String,
    stage: String,
    before: Option<State>,
    after: Option<State>,
    rollback: RollbackPolicy,
    patches: Option<Vec<RecordedPatch>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Journal {
    schema_version: u32,
    transaction_id: String,
    manifest: Manifest,
    plan_sha256: String,
    ownership_sha256: String,
    phase: Phase,
    completed: usize,
    pending: Option<usize>,
    pending_receipt: bool,
    remaining: usize,
    undo_pending: Option<Undo>,
    receipt_before: Option<State>,
    receipt_after: State,
    operations: Vec<Operation>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Undo {
    index: usize,
    after: Option<State>,
}

struct Prepared {
    journal: Journal,
    before: Vec<Option<Snapshot>>,
    after: Vec<Option<Vec<u8>>>,
    receipt_before: Option<Snapshot>,
    receipt_bytes: Vec<u8>,
    receipt: Receipt,
}

struct Restore {
    path: String,
    expected: Option<State>,
    bytes: Option<Vec<u8>>,
    permissions: Permissions,
    changed: bool,
    stage: String,
    stage_state: Option<State>,
}

pub struct Project {
    root: PathBuf,
}

fn valid_hash(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 80
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err(Error::new(
            ErrorKind::InvalidPlan,
            "transaction ID must contain 1..80 ASCII letters, digits, '-' or '_'",
        ));
    }
    Ok(())
}

fn manifest(manifest: &Manifest) -> Result<()> {
    if manifest.version.is_empty() || !valid_hash(&manifest.sha256) || manifest.providers.is_empty()
    {
        return Err(Error::new(
            ErrorKind::InvalidPlan,
            "manifest requires version, SHA-256 and providers",
        ));
    }
    Ok(())
}

fn expected_hash(expected: &ExpectedFile) -> Result<()> {
    if let ExpectedFile::Hash(hash) = expected {
        if !valid_hash(hash) {
            return Err(Error::new(
                ErrorKind::InvalidPlan,
                "expected file hash must be an exact lowercase SHA-256",
            ));
        }
    }
    Ok(())
}

fn owned_path(path: &str) -> Result<()> {
    disk::relative(path)?;
    if ["AGENTS.md", "CLAUDE.md", "DESIGN.md", "PRODUCT.md"].contains(&path)
        || path == ".phoenix-ui/install"
        || path.starts_with(".phoenix-ui/install/")
        || path == ".phoenix-ui/migrations"
        || path.starts_with(".phoenix-ui/migrations/")
    {
        return Err(Error::new(
            ErrorKind::InvalidPlan,
            format!("not an installer-owned target: {path}"),
        ));
    }
    Ok(())
}

fn journal_path(id: &str) -> String {
    format!("{JOURNALS}/{id}.json")
}
fn backups(id: &str) -> String {
    format!(".phoenix-ui/migrations/{id}")
}
fn backup(id: &str, index: usize, suffix: &str) -> String {
    format!("{}/{index}.{suffix}", backups(id))
}
fn receipt_backup(id: &str, suffix: &str) -> String {
    format!("{}/receipt.{suffix}", backups(id))
}
fn encoded<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec_pretty(value).map_err(|e| Error::new(ErrorKind::InvalidPlan, e.to_string()))
}
fn stage(path: &str, id: &str) -> String {
    let (parent, name) = path
        .rsplit_once('/')
        .map_or(("", path), |(parent, name)| (parent, name));
    format!(
        "{}{}.{name}.phoenix-stage-{id}",
        parent,
        if parent.is_empty() { "" } else { "/" }
    )
}

fn ownership_hash(
    manifest: &Manifest,
    operations: &[Operation],
    receipt_before: Option<&State>,
) -> Result<String> {
    Ok(sha256(&encoded(&(manifest, operations, receipt_before))?))
}

impl Project {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root
            .as_ref()
            .canonicalize()
            .map_err(|e| Error::io(root.as_ref(), e))?;
        if !root.is_dir() {
            return Err(Error::new(
                ErrorKind::InvalidPlan,
                "project root is not a directory",
            ));
        }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn active_receipt_verified(&self) -> Result<Option<Receipt>> {
        self.verified_installation()
            .map(|active| active.map(|active| active.receipt))
    }

    /// Verify the active receipt against its journal and current ownership.
    /// Mutable retained state and unrelated shared JSON entries are not owned.
    pub fn verified_installation(&self) -> Result<Option<VerifiedInstallation>> {
        if let Some(pending) = self.pending()? {
            return Err(Error::new(
                ErrorKind::RecoveryRequired,
                format!(
                    "recover interrupted transaction {} before installing",
                    pending.transaction_id
                ),
            ));
        }
        let Some(snapshot) = disk::read(&self.root, ACTIVE)? else {
            return Ok(None);
        };
        let receipt: Receipt =
            serde_json::from_slice(&snapshot.bytes).map_err(|e| Error::conflict(e.to_string()))?;
        if receipt.schema_version != 1 {
            return Err(Error::conflict("active receipt schema is unsupported"));
        }
        id(&receipt.transaction_id).map_err(|e| Error::conflict(e.to_string()))?;
        let journal = self
            .read_journal(&receipt.transaction_id)?
            .ok_or_else(|| Error::conflict("active transaction journal is missing"))?;
        if journal.phase != Phase::Committed
            || snapshot.state != journal.receipt_after
            || receipt.journal != journal_path(&receipt.transaction_id)
        {
            return Err(Error::conflict(
                "active receipt does not match its committed journal",
            ));
        }
        self.installed(&journal, true)?;
        let json_patches = journal
            .operations
            .iter()
            .filter_map(|operation| {
                operation.patches.as_ref().map(|patches| {
                    (
                        operation.path.clone(),
                        patches
                            .iter()
                            .filter(|record| !record.structural)
                            .map(|record| record.patch.clone())
                            .collect(),
                    )
                })
            })
            .collect();
        Ok(Some(VerifiedInstallation {
            receipt,
            receipt_sha256: snapshot.state.hash,
            json_patches,
        }))
    }

    /// No lock file, stage, directory, backup or journal is created here.
    pub fn preflight(&self, plan: &Plan) -> Result<Preflight> {
        if let Some(pending) = self.pending()? {
            return Err(Error::new(
                ErrorKind::RecoveryRequired,
                format!(
                    "recover interrupted transaction {} before installing",
                    pending.transaction_id
                ),
            ));
        }
        if self.already_applied(plan)?.is_some() {
            return Ok(Preflight {
                transaction_id: plan.transaction_id.clone(),
                writes: Vec::new(),
                shared_json: Vec::new(),
                removals: Vec::new(),
                already_applied: true,
            });
        }
        let prepared = self.prepare(plan)?;
        let mut report = Preflight {
            transaction_id: plan.transaction_id.clone(),
            writes: Vec::new(),
            shared_json: Vec::new(),
            removals: Vec::new(),
            already_applied: false,
        };
        for op in prepared.journal.operations {
            if op.patches.is_some() {
                report.shared_json.push(op.path);
            } else if op.after.is_some() {
                report.writes.push(op.path);
            } else {
                report.removals.push(op.path);
            }
        }
        Ok(report)
    }

    pub fn apply(&self, plan: &Plan) -> Result<Receipt> {
        self.apply_with(plan, |_| {})
    }

    pub fn apply_with(&self, plan: &Plan, mut observer: impl FnMut(Checkpoint)) -> Result<Receipt> {
        // A failed first preflight must leave even Phoenix's state paths absent.
        self.preflight(plan)?;
        let _lock = disk::lock(&self.root)?;
        if let Some(receipt) = self.already_applied(plan)? {
            return Ok(receipt);
        }
        // Re-read everything after the caller's Node phase and under exclusion.
        let mut prepared = self.prepare(plan)?;
        self.save_backups(&prepared)?;
        self.save_journal(&prepared.journal)?;
        observer(Checkpoint::JournalReady);
        for i in 0..prepared.journal.operations.len() {
            prepared.journal.pending = Some(i);
            self.save_journal(&prepared.journal)?;
            let op = &prepared.journal.operations[i];
            let permissions = op
                .after
                .as_ref()
                .or(op.before.as_ref())
                .map(|s| s.permissions.clone())
                .unwrap_or_else(|| Permissions::file(0o644));
            disk::replace(
                &self.root,
                &op.path,
                op.before.as_ref(),
                prepared.after[i].as_deref(),
                &permissions,
                Some(&op.stage),
                &mut || observer(Checkpoint::BeforeReplace(op.path.clone())),
            )?;
            observer(Checkpoint::AfterReplace(op.path.clone()));
            prepared.journal.completed = i + 1;
            prepared.journal.pending = None;
            self.save_journal(&prepared.journal)?;
        }
        prepared.journal.pending_receipt = true;
        self.save_journal(&prepared.journal)?;
        observer(Checkpoint::BeforeReceipt);
        self.installed(&prepared.journal, false)?;
        disk::replace(
            &self.root,
            ACTIVE,
            prepared.journal.receipt_before.as_ref(),
            Some(&prepared.receipt_bytes),
            &prepared.journal.receipt_after.permissions,
            Some(&stage(ACTIVE, &plan.transaction_id)),
            &mut || observer(Checkpoint::BeforeReplace(ACTIVE.into())),
        )?;
        observer(Checkpoint::AfterReceipt);
        prepared.journal.phase = Phase::Committed;
        prepared.journal.pending_receipt = false;
        self.save_journal(&prepared.journal)?;
        Ok(prepared.receipt)
    }

    /// Complete the commit if the last receipt was written; otherwise compensate
    /// the interrupted transaction. New managed drift is a conflict, never force.
    pub fn recover(&self) -> Result<Recovery> {
        let Some(_) = self.pending()? else {
            return Ok(Recovery::Clean);
        };
        let _lock = disk::lock(&self.root)?;
        let Some(mut journal) = self.pending()? else {
            return Ok(Recovery::Clean);
        };
        let active = disk::read(&self.root, ACTIVE)?;
        if journal.phase == Phase::Applying
            && journal.pending_receipt
            && active.as_ref().map(|s| &s.state) == Some(&journal.receipt_after)
        {
            journal.phase = Phase::Committed;
            journal.pending_receipt = false;
            self.save_journal(&journal)?;
            return Ok(Recovery::Activated(journal.transaction_id));
        }
        self.rollback_locked(&mut journal, &mut |_| {})?;
        Ok(Recovery::RolledBack(journal.transaction_id))
    }

    pub fn rollback(&self, transaction_id: &str) -> Result<()> {
        self.rollback_with(transaction_id, |_| {})
    }

    pub fn rollback_with(
        &self,
        transaction_id: &str,
        mut observer: impl FnMut(Checkpoint),
    ) -> Result<()> {
        id(transaction_id)?;
        let mut journal = self
            .read_journal(transaction_id)?
            .ok_or_else(|| Error::conflict("transaction journal is missing"))?;
        if journal.phase == Phase::RolledBack {
            return Ok(());
        }
        let _lock = disk::lock(&self.root)?;
        if self
            .pending()?
            .is_some_and(|j| j.transaction_id != transaction_id)
        {
            return Err(Error::new(
                ErrorKind::RecoveryRequired,
                "another interrupted transaction must be recovered first",
            ));
        }
        journal = self
            .read_journal(transaction_id)?
            .ok_or_else(|| Error::conflict("journal disappeared"))?;
        self.rollback_locked(&mut journal, &mut observer)
    }

    fn prepare(&self, plan: &Plan) -> Result<Prepared> {
        id(&plan.transaction_id)?;
        manifest(&plan.manifest)?;
        expected_hash(&plan.expected_receipt)?;
        if let Some(pending) = self.pending()? {
            return Err(Error::new(
                ErrorKind::RecoveryRequired,
                format!(
                    "recover interrupted transaction {} before installing",
                    pending.transaction_id
                ),
            ));
        }
        if self.read_journal(&plan.transaction_id)?.is_some() {
            return Err(Error::conflict("transaction ID is already used"));
        }
        let backup_dir = disk::checked(&self.root, &backups(&plan.transaction_id))?;
        if backup_dir.exists() {
            return Err(Error::conflict(
                "transaction backups already exist; retain them and use a new transaction ID",
            ));
        }
        let receipt_before = disk::read(&self.root, ACTIVE)?;
        let receipt_stage = stage(ACTIVE, &plan.transaction_id);
        if disk::read(&self.root, &receipt_stage)?.is_some() {
            return Err(Error::conflict(format!(
                "staging path already exists: {receipt_stage}"
            )));
        }
        disk::expected(ACTIVE, &plan.expected_receipt, receipt_before.as_ref())?;
        if let Some(snapshot) = &receipt_before {
            let _: Receipt = serde_json::from_slice(&snapshot.bytes)
                .map_err(|e| Error::conflict(format!("active receipt is invalid: {e}")))?;
        }
        for path in [
            ACTIVE.to_string(),
            journal_path(&plan.transaction_id),
            format!("{}/probe", backups(&plan.transaction_id)),
        ] {
            disk::writable(&self.root, &path)?;
        }
        let mut seen = BTreeSet::new();
        for change in &plan.changes {
            let path = change.path();
            owned_path(path)?;
            let stage = stage(path, &plan.transaction_id);
            if disk::read(&self.root, &stage)?.is_some() {
                return Err(Error::conflict(format!(
                    "staging path already exists: {stage}"
                )));
            }
            if seen.iter().any(|previous: &&str| {
                path == *previous
                    || path.starts_with(&format!("{previous}/"))
                    || previous.starts_with(&format!("{path}/"))
            }) {
                return Err(Error::new(
                    ErrorKind::InvalidPlan,
                    format!("duplicate target or file ancestor: {path}"),
                ));
            }
            seen.insert(path);
        }
        let mut changes: Vec<_> = plan.changes.iter().collect();
        // Install payloads, switch shared entries, then retire old paths.
        changes.sort_by_key(|c| match c {
            Change::Exclusive { after: Some(_), .. } => 0,
            Change::SharedJson { .. } => 1,
            Change::Exclusive { after: None, .. } => 2,
        });
        let (mut operations, mut before, mut after) = (Vec::new(), Vec::new(), Vec::new());
        for change in changes {
            let path = change.path();
            let stage = stage(path, &plan.transaction_id);
            disk::writable(&self.root, path)?;
            let snapshot = disk::read(&self.root, path)?;
            let (bytes, state, rollback, patches) = match change {
                Change::Exclusive {
                    expected,
                    after,
                    rollback,
                    ..
                } => {
                    expected_hash(expected)?;
                    disk::expected(path, expected, snapshot.as_ref())?;
                    if let Some(payload) = after {
                        if !valid_hash(&payload.sha256)
                            || payload.sha256 != sha256(&payload.bytes)
                            || payload.permissions.unix_mode > 0o7777
                        {
                            return Err(Error::new(
                                ErrorKind::InvalidPlan,
                                format!("{path}: payload hash or permissions differ"),
                            ));
                        }
                        (
                            Some(payload.bytes.clone()),
                            Some(State {
                                hash: payload.sha256.clone(),
                                permissions: payload.permissions.native(),
                            }),
                            *rollback,
                            None,
                        )
                    } else {
                        (None, None, *rollback, None)
                    }
                }
                Change::SharedJson {
                    expected, patches, ..
                } => {
                    expected_hash(expected)?;
                    disk::expected(path, expected, snapshot.as_ref())?;
                    if patches.is_empty() {
                        return Err(Error::new(
                            ErrorKind::InvalidPlan,
                            "shared JSON change requires entry patches",
                        ));
                    }
                    let initial = snapshot
                        .as_ref()
                        .map_or(b"{}\n".as_slice(), |s| s.bytes.as_slice());
                    let (bytes, patches) = json::prepare(initial, patches)?;
                    let permissions = snapshot
                        .as_ref()
                        .map_or_else(|| Permissions::file(0o644), |s| s.state.permissions.clone());
                    let state = State {
                        hash: sha256(&bytes),
                        permissions,
                    };
                    (
                        Some(bytes),
                        Some(state),
                        RollbackPolicy::Restore,
                        Some(patches),
                    )
                }
            };
            operations.push(Operation {
                path: path.into(),
                stage,
                before: snapshot.as_ref().map(|s| s.state.clone()),
                after: state,
                rollback,
                patches,
            });
            before.push(snapshot);
            after.push(bytes);
        }
        let plan_sha256 = sha256(&encoded(plan)?);
        let ownership_sha256 = ownership_hash(
            &plan.manifest,
            &operations,
            receipt_before.as_ref().map(|s| &s.state),
        )?;
        let receipt = Receipt {
            schema_version: 1,
            transaction_id: plan.transaction_id.clone(),
            manifest: plan.manifest.clone(),
            plan_sha256: plan_sha256.clone(),
            journal: journal_path(&plan.transaction_id),
            ownership_sha256: ownership_sha256.clone(),
            ownership: operations
                .iter()
                .map(|o| Ownership {
                    path: o.path.clone(),
                    sha256: o.after.as_ref().map(|s| s.hash.clone()),
                    shared_entries: o.patches.is_some(),
                    rollback: o.rollback,
                })
                .collect(),
        };
        let receipt_bytes = encoded(&receipt)?;
        let journal = Journal {
            schema_version: 1,
            transaction_id: plan.transaction_id.clone(),
            manifest: plan.manifest.clone(),
            plan_sha256,
            ownership_sha256,
            phase: Phase::Applying,
            completed: 0,
            pending: None,
            pending_receipt: false,
            remaining: 0,
            undo_pending: None,
            receipt_before: receipt_before.as_ref().map(|s| s.state.clone()),
            receipt_after: State {
                hash: sha256(&receipt_bytes),
                permissions: Permissions::file(0o600),
            },
            operations,
        };
        Ok(Prepared {
            journal,
            before,
            after,
            receipt_before,
            receipt_bytes,
            receipt,
        })
    }

    fn save_backups(&self, prepared: &Prepared) -> Result<()> {
        let id = &prepared.journal.transaction_id;
        disk::mkdir(&self.root, &backups(id), true)?;
        for (i, snapshot) in prepared.before.iter().enumerate() {
            if let Some(snapshot) = snapshot {
                disk::create(
                    &self.root,
                    &backup(id, i, "before"),
                    &snapshot.bytes,
                    &snapshot.state.permissions,
                )?;
            }
        }
        for (i, bytes) in prepared.after.iter().enumerate() {
            if let Some(bytes) = bytes {
                disk::create(
                    &self.root,
                    &backup(id, i, "after"),
                    bytes,
                    &Permissions::file(0o600),
                )?;
            }
        }
        if let Some(snapshot) = &prepared.receipt_before {
            disk::create(
                &self.root,
                &receipt_backup(id, "before"),
                &snapshot.bytes,
                &snapshot.state.permissions,
            )?;
        }
        disk::create(
            &self.root,
            &receipt_backup(id, "after"),
            &prepared.receipt_bytes,
            &Permissions::file(0o600),
        )
    }

    fn save_journal(&self, journal: &Journal) -> Result<()> {
        disk::mkdir(&self.root, JOURNALS, true)?;
        let path = journal_path(&journal.transaction_id);
        let current = disk::read(&self.root, &path)?;
        if let Some(snapshot) = &current {
            let previous: Journal = serde_json::from_slice(&snapshot.bytes)
                .map_err(|e| Error::conflict(format!("journal changed during transaction: {e}")))?;
            if previous.ownership_sha256 != journal.ownership_sha256
                || previous.ownership_sha256
                    != ownership_hash(
                        &previous.manifest,
                        &previous.operations,
                        previous.receipt_before.as_ref(),
                    )?
            {
                return Err(Error::conflict(
                    "journal ownership changed during transaction",
                ));
            }
        }
        disk::replace(
            &self.root,
            &path,
            current.as_ref().map(|s| &s.state),
            Some(&encoded(journal)?),
            &Permissions::file(0o600),
            None,
            &mut || {},
        )
    }

    fn read_journal(&self, transaction_id: &str) -> Result<Option<Journal>> {
        id(transaction_id)?;
        let Some(snapshot) = disk::read(&self.root, &journal_path(transaction_id))? else {
            return Ok(None);
        };
        let journal: Journal = serde_json::from_slice(&snapshot.bytes)
            .map_err(|e| Error::conflict(format!("journal {transaction_id} is invalid: {e}")))?;
        if journal.schema_version != 1
            || journal.transaction_id != transaction_id
            || !valid_hash(&journal.plan_sha256)
            || journal.completed > journal.operations.len()
            || journal.remaining > journal.operations.len()
            || journal
                .pending
                .is_some_and(|i| i != journal.completed || i >= journal.operations.len())
            || (journal.pending_receipt
                && (journal.completed != journal.operations.len() || journal.pending.is_some()))
            || journal.undo_pending.as_ref().is_some_and(|u| {
                journal.phase != Phase::RollingBack
                    || Some(u.index) != journal.remaining.checked_sub(1)
            })
        {
            return Err(Error::conflict("journal state is invalid"));
        }
        manifest(&journal.manifest)?;
        if journal.ownership_sha256
            != ownership_hash(
                &journal.manifest,
                &journal.operations,
                journal.receipt_before.as_ref(),
            )?
        {
            return Err(Error::conflict("journal ownership digest differs"));
        }
        let mut paths = BTreeSet::new();
        for op in &journal.operations {
            owned_path(&op.path)?;
            if op.stage != stage(&op.path, transaction_id) {
                return Err(Error::conflict("journal stage path differs"));
            }
            if !paths.insert(&op.path) {
                return Err(Error::conflict("duplicate journal target"));
            }
            for state in [op.before.as_ref(), op.after.as_ref()]
                .into_iter()
                .flatten()
            {
                if !valid_hash(&state.hash) || state.permissions.unix_mode > 0o7777 {
                    return Err(Error::conflict("journal file state is invalid"));
                }
            }
            if let Some(patches) = &op.patches {
                json::validate_records(patches)?;
            }
        }
        if !valid_hash(&journal.receipt_after.hash) {
            return Err(Error::conflict("journal receipt hash is invalid"));
        }
        Ok(Some(journal))
    }

    fn pending(&self) -> Result<Option<Journal>> {
        let dir = disk::checked(&self.root, JOURNALS)?;
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(Error::io(&dir, e)),
        };
        let mut pending = None;
        for entry in entries {
            let entry = entry.map_err(|e| Error::io(&dir, e))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(transaction_id) = name.strip_suffix(".json") else {
                continue;
            };
            let journal = self
                .read_journal(transaction_id)?
                .ok_or_else(|| Error::conflict("journal disappeared during preflight"))?;
            if matches!(journal.phase, Phase::Applying | Phase::RollingBack) {
                if pending.is_some() {
                    return Err(Error::conflict("multiple interrupted install journals"));
                }
                pending = Some(journal);
            }
        }
        Ok(pending)
    }

    fn already_applied(&self, plan: &Plan) -> Result<Option<Receipt>> {
        id(&plan.transaction_id)?;
        let Some(journal) = self.read_journal(&plan.transaction_id)? else {
            return Ok(None);
        };
        if journal.phase != Phase::Committed {
            return Ok(None);
        }
        if journal.plan_sha256 != sha256(&encoded(plan)?) {
            return Err(Error::conflict(
                "transaction ID is bound to a different plan",
            ));
        }
        let receipt = disk::read(&self.root, ACTIVE)?
            .ok_or_else(|| Error::conflict("active receipt disappeared"))?;
        if receipt.state != journal.receipt_after {
            return Err(Error::conflict("another transaction is active"));
        }
        self.installed(&journal, true)?;
        serde_json::from_slice(&receipt.bytes)
            .map(Some)
            .map_err(|e| Error::conflict(e.to_string()))
    }

    fn installed(&self, journal: &Journal, retain_may_change: bool) -> Result<()> {
        for op in &journal.operations {
            if retain_may_change && op.rollback == RollbackPolicy::Retain {
                continue;
            }
            let current = disk::read(&self.root, &op.path)?;
            if let Some(patches) = &op.patches {
                json::installed(
                    &current
                        .ok_or_else(|| Error::conflict("installed shared file disappeared"))?
                        .bytes,
                    patches,
                )?;
            } else if current.as_ref().map(|s| &s.state) != op.after.as_ref() {
                return Err(Error::conflict(format!(
                    "{}: installed ownership changed",
                    op.path
                )));
            }
        }
        Ok(())
    }

    fn verified_backup(&self, path: &str, expected: Option<&State>) -> Result<Option<Vec<u8>>> {
        let Some(expected) = expected else {
            return Ok(None);
        };
        let snapshot = disk::read(&self.root, path)?
            .ok_or_else(|| Error::conflict(format!("backup missing: {path}")))?;
        if snapshot.state != *expected {
            return Err(Error::conflict(format!(
                "backup bytes or permissions differ: {path}"
            )));
        }
        Ok(Some(snapshot.bytes))
    }

    fn restore(&self, journal: &Journal, i: usize) -> Result<Restore> {
        let op = &journal.operations[i];
        let original = self.verified_backup(
            &backup(&journal.transaction_id, i, "before"),
            op.before.as_ref(),
        )?;
        let current = disk::read(&self.root, &op.path)?;
        let expected = current.as_ref().map(|s| s.state.clone());
        let mut permissions = op
            .before
            .as_ref()
            .or(op.after.as_ref())
            .map_or_else(|| Permissions::file(0o644), |s| s.permissions.clone());
        let stage_state = disk::read(&self.root, &op.stage)?.map(|s| s.state);
        if let Some(state) = &stage_state {
            if Some(state) != op.after.as_ref()
                && !journal
                    .undo_pending
                    .as_ref()
                    .is_some_and(|u| u.index == i && u.after.as_ref() == Some(state))
            {
                return Err(Error::conflict(format!(
                    "{}: staging file drifted",
                    op.stage
                )));
            }
        }
        if op.rollback == RollbackPolicy::Retain
            || expected == op.before
            || journal
                .undo_pending
                .as_ref()
                .is_some_and(|u| u.index == i && u.after == expected)
        {
            return Ok(Restore {
                path: op.path.clone(),
                expected,
                bytes: original,
                permissions,
                changed: false,
                stage: op.stage.clone(),
                stage_state,
            });
        }
        let bytes = if expected == op.after {
            original
        } else if let Some(patches) = &op.patches {
            let current = current
                .as_ref()
                .ok_or_else(|| Error::conflict(format!("{}: shared file disappeared", op.path)))?;
            permissions = current.state.permissions.clone();
            if op
                .after
                .as_ref()
                .is_some_and(|s| s.hash == current.state.hash)
            {
                original
            } else {
                Some(json::inverse(&current.bytes, patches)?)
            }
        } else {
            return Err(Error::conflict(format!(
                "{}: installed file drifted; rollback would overwrite new bytes or permissions",
                op.path
            )));
        };
        let changed = current.as_ref().map(|s| s.bytes.as_slice()) != bytes.as_deref()
            || expected
                .as_ref()
                .is_some_and(|s| s.permissions != permissions);
        Ok(Restore {
            path: op.path.clone(),
            expected,
            bytes,
            permissions,
            changed,
            stage: op.stage.clone(),
            stage_state,
        })
    }

    fn rollback_locked(
        &self,
        journal: &mut Journal,
        observer: &mut dyn FnMut(Checkpoint),
    ) -> Result<()> {
        if journal.phase == Phase::RolledBack {
            return Ok(());
        }
        let active = disk::read(&self.root, ACTIVE)?;
        let active_state = active.as_ref().map(|s| s.state.clone());
        if journal.phase == Phase::Committed
            && active_state.as_ref() != Some(&journal.receipt_after)
        {
            return Err(Error::conflict(
                "cannot roll back a superseded active transaction",
            ));
        }
        if active_state != journal.receipt_before
            && active_state.as_ref() != Some(&journal.receipt_after)
        {
            return Err(Error::conflict("active receipt was modified or replaced"));
        }
        let receipt_original = self.verified_backup(
            &receipt_backup(&journal.transaction_id, "before"),
            journal.receipt_before.as_ref(),
        )?;
        let receipt_stage = stage(ACTIVE, &journal.transaction_id);
        let receipt_stage_state = disk::read(&self.root, &receipt_stage)?.map(|s| s.state);
        if receipt_stage_state.as_ref().is_some_and(|state| {
            state != &journal.receipt_after && Some(state) != journal.receipt_before.as_ref()
        }) {
            return Err(Error::conflict(format!(
                "{receipt_stage}: staging file drifted"
            )));
        }
        let remaining = match journal.phase {
            Phase::Committed => journal.operations.len(),
            Phase::Applying => journal.completed + usize::from(journal.pending.is_some()),
            Phase::RollingBack => journal.remaining,
            Phase::RolledBack => 0,
        };
        // Validate every restore and backup before changing any project entry.
        let mut restores = Vec::new();
        for i in (0..remaining).rev() {
            restores.push(self.restore(journal, i)?);
        }
        if let Some(state) = &receipt_stage_state {
            disk::replace(
                &self.root,
                &receipt_stage,
                Some(state),
                None,
                &state.permissions,
                None,
                &mut || {},
            )?;
        }
        journal.phase = Phase::RollingBack;
        journal.remaining = remaining;
        journal.pending = None;
        journal.pending_receipt = false;
        self.save_journal(journal)?;
        for restore in restores {
            if let Some(state) = &restore.stage_state {
                disk::replace(
                    &self.root,
                    &restore.stage,
                    Some(state),
                    None,
                    &state.permissions,
                    None,
                    &mut || {},
                )?;
            }
            if restore.changed {
                journal.undo_pending = Some(Undo {
                    index: journal.remaining - 1,
                    after: restore.bytes.as_ref().map(|bytes| State {
                        hash: sha256(bytes),
                        permissions: restore.permissions.clone(),
                    }),
                });
                self.save_journal(journal)?;
                observer(Checkpoint::BeforeRollback(restore.path.clone()));
                disk::replace(
                    &self.root,
                    &restore.path,
                    restore.expected.as_ref(),
                    restore.bytes.as_deref(),
                    &restore.permissions,
                    Some(&restore.stage),
                    &mut || observer(Checkpoint::BeforeReplace(restore.path.clone())),
                )?;
                observer(Checkpoint::AfterRollback(restore.path));
            }
            journal.remaining -= 1;
            journal.undo_pending = None;
            self.save_journal(journal)?;
        }
        if active_state != journal.receipt_before {
            let permissions = journal
                .receipt_before
                .as_ref()
                .map_or_else(|| Permissions::file(0o600), |s| s.permissions.clone());
            disk::replace(
                &self.root,
                ACTIVE,
                active_state.as_ref(),
                receipt_original.as_deref(),
                &permissions,
                Some(&receipt_stage),
                &mut || observer(Checkpoint::BeforeReplace(ACTIVE.into())),
            )?;
        }
        journal.phase = Phase::RolledBack;
        self.save_journal(journal)
    }
}
