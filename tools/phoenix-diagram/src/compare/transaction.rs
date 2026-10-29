//! Two descriptor-bound outputs share one durable comparison decision.
use crate::output::{
    self, Directory, ErrorKind, FileBinding, FileIdentity, OutputError, OutputResult, OutputSlot,
    Phase, PublicState, SlotLock,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const PREFIX: &str = ".phoenix-diagram-compare-";
const LOCK_TARGET: &str = "comparison-journal";
const RECORD_LIMIT: u64 = 256 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Physical {
    device: String,
    inode: String,
}
impl Physical {
    fn directory(directory: &Directory) -> Self {
        Self {
            device: directory.device.to_string(),
            inode: directory.inode.to_string(),
        }
    }
    fn file(file: &std::fs::File) -> OutputResult<Self> {
        let metadata = file
            .metadata()
            .map_err(|e| OutputError::io("compare/record-metadata", e))?;
        Ok(Self {
            device: metadata.dev().to_string(),
            inode: metadata.ino().to_string(),
        })
    }
    fn matches_file(&self, binding: &FileBinding) -> bool {
        self.device == binding.identity().device && self.inode == binding.identity().inode
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Child {
    directory: PathBuf,
    physical: Physical,
    record: FileIdentity,
    candidate: FileIdentity,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Target {
    requested_path: PathBuf,
    path: PathBuf,
    name: String,
    before: Option<FileIdentity>,
    backup: Option<FileIdentity>,
    published: Option<FileIdentity>,
    restored: Option<FileIdentity>,
    final_identity: Option<FileIdentity>,
    children: Vec<Child>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Record {
    schema_version: u32,
    tool: String,
    kind: String,
    transaction_id: String,
    sequence: usize,
    phase: String,
    parent_path: PathBuf,
    parent: Physical,
    directory: Physical,
    record_identity: Physical,
    previous: Option<FileIdentity>,
    targets: [Target; 2],
    checksum: String,
}
impl Record {
    fn checksum(&self) -> OutputResult<String> {
        let mut record = self.clone();
        record.checksum.clear();
        serde_json::to_vec(&record)
            .map(|bytes| sha256(&bytes))
            .map_err(|e| OutputError::new(ErrorKind::Io, "compare/record-json", e.to_string()))
    }
}

struct Journal {
    parent: Arc<Directory>,
    directory: Arc<Directory>,
    lock: SlotLock,
    record: Record,
    binding: Option<FileBinding>,
}
impl Journal {
    fn verify(&self) -> OutputResult<()> {
        self.parent.verify()?;
        self.directory.verify()?;
        self.lock.verify()?;
        let name = format!("{PREFIX}{}", self.record.transaction_id);
        let entry = self
            .parent
            .entry(&name)?
            .ok_or_else(|| conflict("compare/journal-changed"))?;
        if entry.kind != "directory"
            || entry.uid != output::uid()
            || entry.mode != 0o700
            || Physical::directory(&self.parent) != self.record.parent
            || Physical::directory(&self.directory) != self.record.directory
            || entry.device.to_string() != self.record.directory.device
            || entry.inode.to_string() != self.record.directory.inode
            || self.parent.real != self.record.parent_path
            || self.directory.real != self.parent.real.join(name)
        {
            return Err(conflict("compare/journal-changed"));
        }
        for target in &self.record.targets {
            let (parent, name) = Directory::for_path(&target.requested_path)?;
            if !parent.same_directory(&self.parent)
                || parent.real != self.record.parent_path
                || name != target.name
                || target.path != self.parent.real.join(&target.name)
            {
                return Err(conflict("compare/output-parent-changed"));
            }
        }
        if let Some(binding) = &self.binding {
            binding.verify()?;
        }
        let allowed = allowed_names(&self.record);
        if let Some(name) = directory_names(&self.directory)?
            .into_iter()
            .find(|name| !allowed.contains(name))
        {
            let mut error = conflict("compare/unknown-journal-entry");
            error.preserved.push(self.directory.real.join(name));
            return Err(error);
        }
        Ok(())
    }
    fn save(&mut self) -> OutputResult<()> {
        self.verify()?;
        let name = format!("record-{:08}.json", self.record.sequence);
        let mut file = self.directory.create_file(&name, &[], 0o600)?;
        self.record.record_identity = Physical::file(&file)?;
        self.record.previous = self
            .binding
            .as_ref()
            .map(|binding| binding.identity().clone());
        self.record.checksum = self.record.checksum()?;
        let mut bytes = serde_json::to_vec(&self.record)
            .map_err(|e| OutputError::new(ErrorKind::Io, "compare/record-json", e.to_string()))?;
        bytes.push(b'\n');
        if bytes.len() as u64 > RECORD_LIMIT {
            return Err(conflict("compare/journal-too-large"));
        }
        file.write_all(&bytes)
            .map_err(|e| OutputError::io("compare/record-write", e))?;
        file.sync_all()
            .map_err(|e| OutputError::io("compare/record-sync", e))?;
        self.directory.sync()?;
        self.parent.sync()?;
        let binding = FileBinding::from_file(file, self.directory.clone(), &name, 1, true)?;
        if binding.bytes() != bytes {
            return Err(conflict("compare/journal-changed"));
        }
        self.binding = Some(binding);
        self.record.sequence += 1;
        self.verify()
    }
    fn record_child(
        &mut self,
        index: usize,
        directory: &Path,
        restoring: bool,
    ) -> OutputResult<()> {
        let child_dir = Arc::new(Directory::open(directory)?);
        let candidate = FileBinding::at(child_dir.clone(), "candidate", 1, true)?;
        let child_record =
            FileBinding::at(child_dir.clone(), "publication-recovery-v1.json", 1, true)?;
        let target = &mut self.record.targets[index];
        if restoring {
            target.restored = Some(candidate.identity().clone());
        } else {
            target.published = Some(candidate.identity().clone());
        }
        target.children.push(Child {
            directory: child_dir.real.clone(),
            physical: Physical::directory(&child_dir),
            record: child_record.identity().clone(),
            candidate: candidate.identity().clone(),
        });
        self.save()
    }
    fn verify_backups(&self, allow_absent: bool) -> OutputResult<()> {
        self.verify()?;
        for (index, target) in self.record.targets.iter().enumerate() {
            let name = format!("before-{index}");
            let retired = format!("{name}.retired");
            let present = self.directory.entry(&name)?.is_some();
            let retired_present = self.directory.entry(&retired)?.is_some();
            if present && retired_present {
                return Err(conflict("compare/ambiguous-backup"));
            }
            match (&target.backup, present || retired_present) {
                (Some(expected), true) => {
                    bound_expected(
                        self.directory.clone(),
                        if present { &name } else { &retired },
                        expected,
                    )?;
                }
                (Some(_), false) if allow_absent => {}
                (None, false) => {}
                _ => return Err(conflict("compare/backup-changed")),
            }
        }
        Ok(())
    }
    fn cleanup(&self) -> OutputResult<()> {
        self.verify_backups(true)?;
        for (index, target) in self.record.targets.iter().enumerate() {
            if let Some(expected) = &target.backup {
                remove_expected(&self.directory, &format!("before-{index}"), expected)?;
            }
            for child in &target.children {
                remove_expected(
                    &self.directory,
                    &quarantine_name(&child.candidate),
                    &child.candidate,
                )?;
            }
        }
        self.verify()
    }
}

#[derive(Debug)]
pub struct PairPublication {
    pub html: FileBinding,
    pub receipt: FileBinding,
    pub transaction_id: String,
    pub recovery_directory: PathBuf,
}
pub struct PairTransaction {
    slots: [OutputSlot; 2],
    journal: Journal,
}
impl PairTransaction {
    pub fn id(&self) -> &str {
        &self.journal.record.transaction_id
    }
    pub fn recovery_directory(&self) -> &Path {
        &self.journal.directory.real
    }
    pub fn commit(
        self,
        html: &[u8],
        receipt: &[u8],
        protected: &[&FileBinding],
    ) -> OutputResult<PairPublication> {
        let html_target = self.slots[0].path();
        let receipt_target = self.slots[1].path();
        self.commit_inner(html, receipt, protected, &mut |phase, directory| {
            let (name, target) = match phase {
                PairPhase::BeforeMember(1) => ("compare-before-second-member", &receipt_target),
                PairPhase::BeforeCommitted => ("compare-before-committed", &html_target),
                PairPhase::Committed => ("compare-committed", &html_target),
                _ => return Ok(()),
            };
            crate::output::acceptance_checkpoint(name, target, directory)
        })
    }
    fn commit_inner(
        mut self,
        html: &[u8],
        receipt: &[u8],
        protected: &[&FileBinding],
        observer: PairObserver<'_>,
    ) -> OutputResult<PairPublication> {
        let operation = (|| {
            self.journal.verify_backups(false)?;
            for slot in &self.slots {
                slot.verify_snapshot()?;
            }
            for source in protected {
                source.verify()?;
            }
            self.journal.record.phase = "applying".into();
            self.journal.save()?;
            for (index, bytes) in [html, receipt].into_iter().enumerate() {
                emit(
                    observer,
                    PairPhase::BeforeMember(index),
                    self.recovery_directory(),
                )?;
                let mode = self.journal.record.targets[index]
                    .before
                    .as_ref()
                    .map_or(0o644, |identity| identity.mode);
                let journal = &mut self.journal;
                let publication = output::publish_observed(
                    &self.slots[index],
                    bytes,
                    mode,
                    protected,
                    &mut |phase, observation| {
                        if phase == Phase::RecoveryDurable {
                            journal
                                .record_child(index, &observation.directory, false)
                                .map_err(io_error)?;
                        }
                        emit(
                            observer,
                            PairPhase::Publication(index, phase),
                            &journal.directory.real,
                        )
                        .map_err(io_error)
                    },
                )?;
                if !publication.cleanup_complete {
                    return Err(conflict("compare/member-cleanup-required"));
                }
                emit(
                    observer,
                    PairPhase::MemberPublished(index),
                    self.recovery_directory(),
                )?;
            }
            for source in protected {
                source.verify()?;
            }
            let html = FileBinding::capture(&self.slots[0].path())?;
            let receipt = FileBinding::capture(&self.slots[1].path())?;
            for (target, binding) in self
                .journal
                .record
                .targets
                .iter_mut()
                .zip([&html, &receipt])
            {
                if !target
                    .published
                    .as_ref()
                    .is_some_and(|identity| exact(identity, binding.identity()))
                {
                    return Err(conflict("compare/published-binding-changed"));
                }
                target.final_identity = Some(binding.identity().clone());
            }
            emit(
                observer,
                PairPhase::BeforeCommitted,
                self.recovery_directory(),
            )?;
            for source in protected {
                source.verify().map_err(|mut error| {
                    error.code = "input/changed".into();
                    error
                })?;
            }
            html.verify()?;
            receipt.verify()?;
            self.journal.verify_backups(false)?;
            self.journal.record.phase = "committed".into();
            self.journal.save()?;
            emit(observer, PairPhase::Committed, self.recovery_directory())?;
            self.journal.cleanup()?;
            html.verify()?;
            receipt.verify()?;
            Ok(PairPublication {
                html,
                receipt,
                transaction_id: self.id().into(),
                recovery_directory: self.recovery_directory().into(),
            })
        })();
        match operation {
            Ok(publication) => Ok(publication),
            Err(mut error) => {
                let directory = self.recovery_directory().to_owned();
                let committed = self
                    .journal
                    .binding
                    .as_ref()
                    .and_then(|binding| serde_json::from_slice::<Record>(binding.bytes()).ok())
                    .is_some_and(|record| record.phase == "committed");
                drop(self);
                let recovery = recover(&directory);
                error.recovery_directory = Some(directory.clone());
                error.committed = committed;
                if recovery.exit_code() != 0 {
                    error.kind = ErrorKind::RecoveryRequired;
                    error.preserved = recovery.preserved;
                }
                if committed {
                    error.kind = ErrorKind::RecoveryRequired;
                }
                Err(error)
            }
        }
    }
}

pub fn begin(
    html: &Path,
    receipt: &Path,
    protected: &[&FileBinding],
) -> OutputResult<PairTransaction> {
    let (parent, html_name) = Directory::for_path(html)?;
    let (other, receipt_name) = Directory::for_path(receipt)?;
    if !parent.same_directory(&other) || parent.real != other.real {
        return Err(conflict("compare/output-parent-mismatch"));
    }
    if html_name == receipt_name {
        return Err(conflict("compare/output-alias"));
    }
    let mut paths = [(0, html), (1, receipt)];
    paths.sort_by_key(|(_, path)| parent.real.join(path.file_name().unwrap()));
    let first = output::capture_slot(paths[0].1, protected)?;
    let mut guarded = protected.to_vec();
    if let Some(previous) = first.previous() {
        guarded.push(previous);
    }
    let second = output::capture_slot(paths[1].1, &guarded)?;
    let slots = if paths[0].0 == 0 {
        [first, second]
    } else {
        [second, first]
    };
    for slot in &slots {
        if !slot.pending_publications()?.is_empty() {
            return Err(conflict("compare/pending-output-publication"));
        }
    }
    reject_pending_pairs(&parent, [&html_name, &receipt_name])?;
    let id = output::transaction_id()?;
    let directory = parent.create_directory(&format!("{PREFIX}{id}"))?;
    let lock = SlotLock::acquire(directory.clone(), LOCK_TARGET)?;
    let mut targets = Vec::new();
    for (index, slot) in slots.iter().enumerate() {
        let backup = if let Some(previous) = slot.previous() {
            previous.verify()?;
            let name = format!("before-{index}");
            let file = directory.create_file(&name, previous.bytes(), 0o600)?;
            let backup = FileBinding::from_file(file, directory.clone(), &name, 1, true)?;
            if backup.bytes() != previous.bytes() {
                return Err(conflict("compare/backup-changed"));
            }
            Some(backup.identity().clone())
        } else {
            None
        };
        targets.push(Target {
            requested_path: slot.requested_path().into(),
            path: slot.path(),
            name: slot.path().file_name().unwrap().to_str().unwrap().into(),
            before: slot.previous().map(|binding| binding.identity().clone()),
            backup,
            published: None,
            restored: None,
            final_identity: None,
            children: vec![],
        });
    }
    let record = Record {
        schema_version: 1,
        tool: "phoenix-diagram".into(),
        kind: "architecture-comparison-publication".into(),
        transaction_id: id,
        sequence: 0,
        phase: "prepared".into(),
        parent_path: parent.real.clone(),
        parent: Physical::directory(&parent),
        directory: Physical::directory(&directory),
        record_identity: Physical {
            device: "0".into(),
            inode: "0".into(),
        },
        previous: None,
        targets: targets.try_into().unwrap(),
        checksum: String::new(),
    };
    let mut journal = Journal {
        parent,
        directory,
        lock,
        record,
        binding: None,
    };
    journal.save()?;
    Ok(PairTransaction { slots, journal })
}

fn read_journal(path: &Path) -> OutputResult<Journal> {
    let (parent, name) = Directory::for_path(path)?;
    let id = name
        .strip_prefix(PREFIX)
        .filter(|id| valid_id(id))
        .ok_or_else(|| conflict("compare/invalid-recovery-directory"))?;
    let entry = parent
        .entry(&name)?
        .ok_or_else(|| conflict("compare/recovery-directory-absent"))?;
    if entry.kind != "directory" || entry.uid != output::uid() || entry.mode != 0o700 {
        return Err(conflict("compare/journal-not-private"));
    }
    let directory = Arc::new(Directory::open(&parent.real.join(&name))?);
    let lock = SlotLock::acquire_existing(directory.clone(), LOCK_TARGET)?;
    let names = directory_names(&directory)?;
    let records: Vec<_> = names
        .iter()
        .filter(|name| name.starts_with("record-"))
        .cloned()
        .collect();
    if records.is_empty() {
        return Err(conflict("compare/journal-missing"));
    }
    let mut previous: Option<FileBinding> = None;
    let mut current = None;
    for (sequence, name) in records.iter().enumerate() {
        if name != &format!("record-{sequence:08}.json") {
            return Err(conflict("compare/journal-sequence"));
        }
        let binding = FileBinding::at(directory.clone(), name, 1, true)?;
        if binding.identity().mode != 0o600 || binding.identity().bytes > RECORD_LIMIT {
            return Err(conflict("compare/journal-not-private"));
        }
        let mut record: Record = serde_json::from_slice(binding.bytes())
            .map_err(|_| conflict("compare/journal-json"))?;
        if record.schema_version != 1
            || record.tool != "phoenix-diagram"
            || record.kind != "architecture-comparison-publication"
            || record.transaction_id != id
            || record.sequence != sequence
            || record.checksum != record.checksum()?
            || !record.record_identity.matches_file(&binding)
            || record.previous.as_ref() != previous.as_ref().map(|binding| binding.identity())
            || !matches!(
                record.phase.as_str(),
                "prepared" | "applying" | "recovering" | "committed" | "recovered"
            )
        {
            return Err(conflict("compare/journal-binding-changed"));
        }
        record.sequence += 1;
        current = Some(record);
        previous = Some(binding);
    }
    let journal = Journal {
        parent,
        directory,
        lock,
        record: current.unwrap(),
        binding: previous,
    };
    journal.verify()?;
    if journal.record.targets[0].name == journal.record.targets[1].name {
        return Err(conflict("compare/output-alias"));
    }
    Ok(journal)
}

fn allowed_names(record: &Record) -> BTreeSet<String> {
    let mut names = BTreeSet::from([
        lock_name(),
        "before-0".into(),
        "before-1".into(),
        "before-0.retired".into(),
        "before-1.retired".into(),
    ]);
    names.extend((0..record.sequence).map(|sequence| format!("record-{sequence:08}.json")));
    for target in &record.targets {
        for child in &target.children {
            names.insert(quarantine_name(&child.candidate));
            names.insert(format!("{}.retired", quarantine_name(&child.candidate)));
        }
    }
    names
}

fn reject_pending_pairs(parent: &Directory, names: [&str; 2]) -> OutputResult<()> {
    for name in directory_names(parent)?
        .into_iter()
        .filter(|name| name.starts_with(PREFIX))
    {
        let journal = read_journal(&parent.real.join(name))?;
        if !matches!(journal.record.phase.as_str(), "committed" | "recovered")
            && journal
                .record
                .targets
                .iter()
                .any(|target| names.contains(&target.name.as_str()))
        {
            let mut error = conflict("compare/pending-transaction");
            error.recovery_directory = Some(journal.directory.real.clone());
            return Err(error);
        }
    }
    Ok(())
}

pub fn verify_committed(
    directory: &Path,
    html: &FileBinding,
    receipt: &FileBinding,
) -> OutputResult<()> {
    let journal = read_journal(directory)?;
    if journal.record.phase != "committed" {
        return Err(conflict("compare/not-committed"));
    }
    let mut order = [0, 1];
    order.sort_by_key(|index| &journal.record.targets[*index].path);
    let guards = order.map(|index| {
        SlotLock::acquire_existing(journal.parent.clone(), &journal.record.targets[index].name)
    });
    let [first, second] = guards;
    let guards = [first?, second?];
    journal.verify_backups(true)?;
    for (target, binding) in journal.record.targets.iter().zip([html, receipt]) {
        binding.verify()?;
        if binding.path() != target.path
            || !target
                .final_identity
                .as_ref()
                .is_some_and(|expected| exact(expected, binding.identity()))
        {
            return Err(conflict("compare/committed-binding-changed"));
        }
    }
    for guard in &guards {
        guard.verify()?;
    }
    journal.verify()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryReceipt {
    pub schema_version: u32,
    pub tool: String,
    pub tool_version: String,
    pub command: String,
    pub status: String,
    pub diagnostics: crate::diagnostic::Diagnostics,
    pub transaction_id: Option<String>,
    pub recovery_directory: PathBuf,
    pub public_states: BTreeMap<PathBuf, PublicState>,
    pub preserved: Vec<PathBuf>,
    #[serde(skip)]
    exit: u8,
}
impl RecoveryReceipt {
    pub fn exit_code(&self) -> u8 {
        self.exit
    }
}

pub fn recover(directory: &Path) -> RecoveryReceipt {
    let mut result = RecoveryReceipt {
        schema_version: 1,
        tool: "phoenix-diagram".into(),
        tool_version: crate::TOOL_VERSION.into(),
        command: "recover-compare".into(),
        status: "recovery-required".into(),
        diagnostics: vec![],
        transaction_id: None,
        recovery_directory: directory.into(),
        public_states: BTreeMap::new(),
        preserved: vec![directory.into()],
        exit: 3,
    };
    let mut trusted_parent = None;
    let operation = (|| {
        let mut journal = read_journal(directory)?;
        result.transaction_id = Some(journal.record.transaction_id.clone());
        result.recovery_directory = journal.directory.real.clone();
        trusted_parent = Some(journal.parent.clone());
        for target in &journal.record.targets {
            result.public_states.insert(
                target.path.clone(),
                output::public_state(&journal.parent, &target.name),
            );
        }
        let terminal = matches!(journal.record.phase.as_str(), "committed" | "recovered");
        journal.verify_backups(terminal)?;
        reconcile_children(&mut journal, terminal)?;
        let slots = recovery_slots(&journal)?;
        if terminal {
            for (target, slot) in journal.record.targets.iter().zip(&slots) {
                verify_final(target, slot)?;
            }
        } else {
            for (target, slot) in journal.record.targets.iter().zip(&slots) {
                verify_allowed(target, slot)?;
            }
            journal.record.phase = "recovering".into();
            journal.save()?;
            for index in (0..2).rev() {
                let target = &journal.record.targets[index];
                let already_old = match (target.before.as_ref(), slots[index].previous()) {
                    (Some(before), Some(current)) => {
                        exact(before, current.identity())
                            || target
                                .restored
                                .as_ref()
                                .is_some_and(|identity| exact(identity, current.identity()))
                    }
                    (None, None) => true,
                    _ => false,
                };
                if already_old {
                    continue;
                }
                if let Some(before) = &target.before {
                    let mode = before.mode;
                    let backup = bound_expected(
                        journal.directory.clone(),
                        &format!("before-{index}"),
                        target.backup.as_ref().unwrap(),
                    )?;
                    let publication = output::publish_observed(
                        &slots[index],
                        backup.bytes(),
                        mode,
                        &[],
                        &mut |phase, observation| {
                            if phase == Phase::RecoveryDurable {
                                journal
                                    .record_child(index, &observation.directory, true)
                                    .map_err(io_error)?;
                            }
                            Ok(())
                        },
                    )?;
                    if !publication.cleanup_complete {
                        return Err(conflict("compare/restore-cleanup-required"));
                    }
                } else if let Some(current) = slots[index].previous() {
                    quarantine(&journal, index, current.identity())?;
                }
            }
            for target in &mut journal.record.targets {
                target.final_identity = match output::public_state(&journal.parent, &target.name) {
                    PublicState::Absent if target.before.is_none() => None,
                    PublicState::File { identity }
                        if target
                            .before
                            .as_ref()
                            .is_some_and(|before| exact(before, &identity))
                            || target
                                .restored
                                .as_ref()
                                .is_some_and(|restored| exact(restored, &identity)) =>
                    {
                        Some(identity)
                    }
                    _ => return Err(conflict("compare/restoration-binding-changed")),
                };
            }
            journal.record.phase = "recovered".into();
            journal.save()?;
        }
        journal.cleanup()?;
        for target in &journal.record.targets {
            result.public_states.insert(
                target.path.clone(),
                output::public_state(&journal.parent, &target.name),
            );
        }
        result.status = if journal.record.phase == "committed" {
            "committed"
        } else {
            "recovered"
        }
        .into();
        result.exit = 0;
        result.preserved.clear();
        Ok(())
    })();
    if let Err(mut error) = operation {
        error.kind = ErrorKind::RecoveryRequired;
        error.recovery_directory = Some(result.recovery_directory.clone());
        result.preserved.extend(error.preserved.clone());
        let mut diagnostic =
            crate::diagnostic::Diagnostic::error(None, &error.code, "", error.message);
        diagnostic.stage = "recover-compare".into();
        diagnostic.evidence = serde_json::json!({"kind":error.kind,"recoveryDirectory":error.recovery_directory,"preserved":error.preserved});
        result.diagnostics.push(diagnostic);
    }
    if let Some(parent) = trusted_parent {
        for (path, state) in &mut result.public_states {
            *state = match parent.verify() {
                Ok(()) => {
                    output::public_state(&parent, path.file_name().unwrap().to_str().unwrap())
                }
                Err(error) => PublicState::Unknown { reason: error.code },
            };
        }
    }
    result
}

fn recovery_slots(journal: &Journal) -> OutputResult<[OutputSlot; 2]> {
    let targets = &journal.record.targets;
    let mut order = [0, 1];
    order.sort_by_key(|index| &targets[*index].path);
    let first = output::capture_slot(&targets[order[0]].requested_path, &[])?;
    let protected: Vec<_> = first.previous().into_iter().collect();
    let second = output::capture_slot(&targets[order[1]].requested_path, &protected)?;
    Ok(if order[0] == 0 {
        [first, second]
    } else {
        [second, first]
    })
}

fn reconcile_children(journal: &mut Journal, terminal: bool) -> OutputResult<()> {
    for index in 0..2 {
        let target = &journal.record.targets[index];
        let pending = output::pending_existing_publications(&target.path)?;
        for pending in pending {
            let child = target
                .children
                .iter()
                .find(|child| child.directory == pending.recovery_directory)
                .ok_or_else(|| conflict("compare/unrelated-publication-preserved"))?;
            let child_record_path = pending
                .witness
                .clone()
                .unwrap_or_else(|| child.directory.join("publication-recovery-v1.json"));
            let record = FileBinding::capture(&child_record_path)?;
            if !exact(&child.record, record.identity()) {
                return Err(conflict("compare/child-record-changed"));
            }
            if pending.witness.is_none() {
                let child_directory = Directory::open(&child.directory)?;
                if Physical::directory(&child_directory) != child.physical {
                    return Err(conflict("compare/child-directory-changed"));
                }
            }
            if terminal {
                return Err(conflict("compare/committed-child-pending"));
            }
            if pending.phase != "cleanup" {
                let lock = SlotLock::acquire_existing(journal.parent.clone(), &target.name)?;
                if let PublicState::File { identity } =
                    output::public_state(&journal.parent, &target.name)
                {
                    if output::same_identity(&identity, &child.candidate) {
                        quarantine(journal, index, &child.candidate)?;
                    }
                }
                drop(lock);
            }
            let recovered = output::recover_publication(&child.directory);
            if recovered.exit_code() != 0 && recovered.code != "output/committed-output-preserved" {
                let mut error = conflict("compare/child-recovery-required");
                error.preserved = recovered.preserved;
                return Err(error);
            }
            if output::pending_existing_publications(&target.path)?
                .iter()
                .any(|pending| pending.recovery_directory == child.directory)
            {
                return Err(conflict("compare/child-recovery-incomplete"));
            }
        }
    }
    Ok(())
}

fn verify_allowed(target: &Target, slot: &OutputSlot) -> OutputResult<()> {
    match slot.previous() {
        None => Ok(()),
        Some(binding)
            if [&target.before, &target.published, &target.restored]
                .into_iter()
                .flatten()
                .any(|identity| exact(identity, binding.identity())) =>
        {
            Ok(())
        }
        _ => {
            let mut error = conflict("compare/later-claimant-preserved");
            error.preserved.push(target.path.clone());
            Err(error)
        }
    }
}
fn verify_final(target: &Target, slot: &OutputSlot) -> OutputResult<()> {
    match (&target.final_identity, slot.previous()) {
        (None, None) => Ok(()),
        (Some(expected), Some(binding)) if exact(expected, binding.identity()) => Ok(()),
        _ => {
            let mut error = conflict("compare/final-binding-changed");
            error.preserved.push(target.path.clone());
            Err(error)
        }
    }
}
fn quarantine(journal: &Journal, index: usize, expected: &FileIdentity) -> OutputResult<()> {
    journal.verify()?;
    let target = &journal.record.targets[index];
    let name = quarantine_name(expected);
    let entry = journal
        .parent
        .entry(&target.name)?
        .ok_or_else(|| conflict("compare/published-file-absent"))?;
    let binding = FileBinding::at(journal.parent.clone(), &target.name, entry.links, true)?;
    if !output::same_identity(expected, binding.identity()) {
        return Err(conflict("compare/later-claimant-preserved"));
    }
    output::rename_no_clobber(&journal.parent, &target.name, &journal.directory, &name)?;
    if let Err(mut error) = binding.verify_at(&journal.directory, &name, entry.links) {
        let _ = output::rename_no_clobber(&journal.directory, &name, &journal.parent, &target.name);
        error.preserved.push(journal.directory.real.join(name));
        return Err(error);
    }
    Ok(())
}
fn bound_expected(
    directory: Arc<Directory>,
    name: &str,
    expected: &FileIdentity,
) -> OutputResult<FileBinding> {
    let preserved = directory.real.join(name);
    let binding = FileBinding::at(directory, name, 1, true).map_err(|mut error| {
        error.preserved.push(preserved.clone());
        error
    })?;
    if !exact(expected, binding.identity()) {
        let mut error = conflict("compare/private-binding-changed");
        error.preserved.push(preserved);
        return Err(error);
    }
    Ok(binding)
}
fn remove_expected(
    directory: &Arc<Directory>,
    name: &str,
    expected: &FileIdentity,
) -> OutputResult<()> {
    let retired = format!("{name}.retired");
    let present = directory.entry(name)?.is_some();
    let retired_present = directory.entry(&retired)?.is_some();
    if present && retired_present {
        return Err(conflict("compare/ambiguous-private-cleanup"));
    }
    if !present && !retired_present {
        return Ok(());
    }
    let binding = bound_expected(
        directory.clone(),
        if present { name } else { &retired },
        expected,
    )?;
    if present {
        output::rename_no_clobber(directory, name, directory, &retired)?;
    }
    if let Err(mut error) = binding.verify_at(directory, &retired, 1) {
        let _ = output::rename_no_clobber(directory, &retired, directory, name);
        error.preserved.push(directory.real.join(retired));
        return Err(error);
    }
    directory.unlink_private(&retired, false)
}
fn directory_names(directory: &Directory) -> OutputResult<BTreeSet<String>> {
    directory.verify()?;
    let names = std::fs::read_dir(&directory.real)
        .map_err(|e| OutputError::io("compare/directory-read", e))?
        .map(|entry| {
            entry
                .map_err(|e| OutputError::io("compare/directory-read", e))?
                .file_name()
                .into_string()
                .map_err(|_| conflict("compare/non-utf8-journal-entry"))
        })
        .collect();
    directory.verify()?;
    names
}
fn lock_name() -> String {
    format!(
        ".phoenix-diagram-lock-{}.lock",
        sha256(LOCK_TARGET.as_bytes())
    )
}
fn quarantine_name(identity: &FileIdentity) -> String {
    format!("quarantine-{}-{}", identity.device, identity.inode)
}
fn valid_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn exact(left: &FileIdentity, right: &FileIdentity) -> bool {
    left.links == right.links && output::same_identity(left, right)
}
fn conflict(code: &str) -> OutputError {
    OutputError::conflict(code)
}
fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn io_error(error: OutputError) -> std::io::Error {
    std::io::Error::other(format!("{}: {}", error.code, error.message))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum PairPhase {
    BeforeMember(usize),
    Publication(usize, Phase),
    MemberPublished(usize),
    BeforeCommitted,
    Committed,
}
type PairObserver<'a> = &'a mut dyn FnMut(PairPhase, &Path) -> std::io::Result<()>;
fn emit(observer: PairObserver<'_>, phase: PairPhase, directory: &Path) -> OutputResult<()> {
    observer(phase, directory)
        .map_err(|error| OutputError::io("compare/observer-interruption", error))
}
#[cfg(test)]
#[allow(dead_code)] // Used by the integration test's private module copy.
pub(crate) mod test_support {
    pub(crate) fn commit(
        transaction: super::PairTransaction,
        html: &[u8],
        receipt: &[u8],
        protected: &[&super::FileBinding],
        observer: super::PairObserver<'_>,
    ) -> super::OutputResult<super::PairPublication> {
        transaction.commit_inner(html, receipt, protected, observer)
    }
}
