use super::*;
use std::io::Write;
use std::os::unix::fs::MetadataExt;

const RECORD: &str = "publication-recovery-v1.json";
const PREFIX: &str = ".phoenix-diagram-recovery-";
const CLEANUP: &str = ".phoenix-diagram-output-cleanup";
const OWNER: &str = "owner-v1.json";
const LIMIT: u64 = 16 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Physical {
    device: String,
    inode: String,
}
impl Physical {
    fn directory(dir: &Directory) -> Self {
        Self {
            device: dir.device.to_string(),
            inode: dir.inode.to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CleanupOwner {
    schema_version: u32,
    tool: String,
    kind: String,
    parent_path: PathBuf,
    parent: Physical,
    directory: Physical,
    record_identity: Physical,
    checksum: String,
}
impl CleanupOwner {
    fn checksum(&self) -> OutputResult<String> {
        let mut value = self.clone();
        value.checksum.clear();
        let bytes = serde_json::to_vec(&value).map_err(|e| {
            OutputError::new(ErrorKind::Io, "output/owner-serialization", e.to_string())
        })?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }
}

struct CleanupDirectory {
    parent: Arc<Directory>,
    directory: Arc<Directory>,
    marker: FileBinding,
    owner: CleanupOwner,
}
impl CleanupDirectory {
    fn verify(&self) -> OutputResult<()> {
        self.parent.verify()?;
        verify_private(&self.directory)?;
        self.marker.verify_at(&self.directory, OWNER, 1)?;
        if self.directory.real != self.parent.real.join(CLEANUP)
            || self.owner.schema_version != 1
            || self.owner.tool != "phoenix-diagram"
            || self.owner.kind != "output-cleanup-directory"
            || self.owner.parent_path != self.parent.real
            || self.owner.parent.device != self.parent.device.to_string()
            || self.owner.parent.inode != self.parent.inode.to_string()
            || self.owner.directory.device != self.directory.device.to_string()
            || self.owner.directory.inode != self.directory.inode.to_string()
            || self.owner.record_identity.device != self.marker.identity.device
            || self.owner.record_identity.inode != self.marker.identity.inode
            || self.marker.identity.uid != unix::uid()
            || self.marker.identity.mode != 0o600
            || self.owner.checksum != self.owner.checksum()?
        {
            return Err(OutputError::conflict(
                "output/cleanup-owner-binding-mismatch",
            ));
        }
        Ok(())
    }
}

fn cleanup_directory(
    parent: &Arc<Directory>,
    create: bool,
) -> OutputResult<Option<CleanupDirectory>> {
    let path = parent.real.join(CLEANUP);
    let operation = (|| {
        parent.verify()?;
        let (directory, created) = match parent.entry(CLEANUP)? {
            Some(entry) => {
                if entry.kind != "directory" || entry.mode != 0o700 || entry.uid != unix::uid() {
                    return Err(OutputError::conflict("output/unknown-cleanup-directory"));
                }
                let directory = Arc::new(Directory::open(&path)?);
                if directory.device != entry.device || directory.inode != entry.inode {
                    return Err(OutputError::conflict("output/cleanup-directory-changed"));
                }
                (directory, false)
            }
            None if !create => return Ok(None),
            None => match parent.create_directory(CLEANUP) {
                Ok(directory) => (directory, true),
                Err(error) => {
                    if parent.entry(CLEANUP)?.is_some() {
                        return cleanup_directory(parent, false);
                    }
                    return Err(error);
                }
            },
        };
        verify_private(&directory)?;
        let marker = if created {
            let mut file = directory.create_file(OWNER, &[], 0o600)?;
            let metadata = file
                .metadata()
                .map_err(|e| OutputError::io("output/owner-metadata", e))?;
            let mut owner = CleanupOwner {
                schema_version: 1,
                tool: "phoenix-diagram".into(),
                kind: "output-cleanup-directory".into(),
                parent_path: parent.real.clone(),
                parent: Physical::directory(parent),
                directory: Physical::directory(&directory),
                record_identity: Physical {
                    device: metadata.dev().to_string(),
                    inode: metadata.ino().to_string(),
                },
                checksum: String::new(),
            };
            owner.checksum = owner.checksum()?;
            let mut bytes = serde_json::to_vec(&owner).map_err(|e| {
                OutputError::new(ErrorKind::Io, "output/owner-serialization", e.to_string())
            })?;
            bytes.push(b'\n');
            file.write_all(&bytes)
                .map_err(|e| OutputError::io("output/owner-write", e))?;
            file.sync_all()
                .map_err(|e| OutputError::io("output/owner-sync", e))?;
            directory.sync()?;
            parent.sync()?;
            let marker = FileBinding::from_file(file, directory.clone(), OWNER, 1, true)?;
            if marker.bytes() != bytes {
                return Err(OutputError::conflict("output/cleanup-owner-changed"));
            }
            marker
        } else {
            let entry = directory
                .entry(OWNER)?
                .ok_or_else(|| OutputError::conflict("output/cleanup-owner-absent"))?;
            if entry.kind != "file"
                || entry.links != 1
                || entry.mode != 0o600
                || entry.uid != unix::uid()
                || entry.bytes > LIMIT
            {
                return Err(OutputError::conflict("output/cleanup-owner-not-private"));
            }
            FileBinding::at(directory.clone(), OWNER, 1, true)?
        };
        if marker.identity.bytes > LIMIT {
            return Err(OutputError::conflict("output/cleanup-owner-too-large"));
        }
        let owner = serde_json::from_slice(marker.bytes())
            .map_err(|_| OutputError::conflict("output/unknown-cleanup-owner"))?;
        let cleanup = CleanupDirectory {
            parent: parent.clone(),
            directory,
            marker,
            owner,
        };
        cleanup.verify()?;
        Ok(Some(cleanup))
    })();
    operation.map_err(|mut error: OutputError| {
        error.preserved.push(path);
        error
    })
}

fn cleanup_names(
    cleanup: Option<&CleanupDirectory>,
) -> OutputResult<std::collections::BTreeSet<String>> {
    let mut names = std::collections::BTreeSet::new();
    let Some(cleanup) = cleanup else {
        return Ok(names);
    };
    cleanup.verify()?;
    let entries = std::fs::read_dir(&cleanup.directory.real)
        .map_err(|e| OutputError::io("output/cleanup-directory-read", e))?;
    for entry in entries {
        let entry = entry.map_err(|e| OutputError::io("output/cleanup-directory-read", e))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Err(namespace_error(entry.path()));
        };
        if name == OWNER {
            continue;
        }
        if !name.strip_suffix(".json").is_some_and(valid_id) {
            return Err(namespace_error(entry.path()));
        }
        names.insert(name.to_owned());
    }
    cleanup.verify()?;
    Ok(names)
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RecoveryDirectory {
    name: String,
    identity: Physical,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Target {
    requested_path: PathBuf,
    commit_path: PathBuf,
    parent_path: PathBuf,
    parent: Physical,
    name: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Record {
    schema_version: u32,
    tool: String,
    kind: String,
    transaction_id: String,
    target: Target,
    recovery: RecoveryDirectory,
    backup: Option<FileIdentity>,
    candidate: FileIdentity,
    record_identity: Physical,
    checksum: String,
}
impl Record {
    fn checksum(&self) -> OutputResult<String> {
        let mut value = self.clone();
        value.checksum.clear();
        let bytes = serde_json::to_vec(&value).map_err(|e| {
            OutputError::new(ErrorKind::Io, "output/record-serialization", e.to_string())
        })?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }
    fn bytes(&mut self) -> OutputResult<Vec<u8>> {
        self.checksum = self.checksum()?;
        let mut bytes = serde_json::to_vec(self).map_err(|e| {
            OutputError::new(ErrorKind::Io, "output/record-serialization", e.to_string())
        })?;
        bytes.push(b'\n');
        if bytes.len() as u64 > LIMIT {
            return Err(OutputError::new(
                ErrorKind::Invalid,
                "output/recovery-record-too-large",
                "Recovery record exceeds 16 KiB",
            ));
        }
        Ok(bytes)
    }
    fn parse(bytes: &[u8]) -> OutputResult<Self> {
        if bytes.len() as u64 > LIMIT {
            return Err(OutputError::conflict("output/recovery-record-too-large"));
        }
        let record: Self = serde_json::from_slice(bytes)
            .map_err(|_| OutputError::conflict("output/unknown-recovery-record"))?;
        let decimal = |s: &str| s.parse::<u64>().is_ok_and(|n| n.to_string() == s);
        let identity = |i: &FileIdentity| {
            decimal(&i.device)
                && decimal(&i.inode)
                && i.inode != "0"
                && i.links == 1
                && i.mode <= 0o777
                && i.uid == unix::uid()
                && i.sha256.len() == 64
                && i.sha256.bytes().all(|b| b.is_ascii_hexdigit())
        };
        let physical = |p: &Physical| decimal(&p.device) && decimal(&p.inode) && p.inode != "0";
        if record.schema_version != 1
            || record.tool != "phoenix-diagram"
            || record.kind != "output-publication"
            || !valid_id(&record.transaction_id)
            || record.recovery.name != format!("{PREFIX}{}", record.transaction_id)
            || record.target.name.is_empty()
            || record.target.name.contains('/')
            || matches!(record.target.name.as_str(), "." | "..")
            || !record.target.requested_path.is_absolute()
            || !record.target.commit_path.is_absolute()
            || !record.target.parent_path.is_absolute()
            || record.target.parent_path.join(&record.target.name) != record.target.commit_path
            || !physical(&record.target.parent)
            || !physical(&record.recovery.identity)
            || !physical(&record.record_identity)
            || !identity(&record.candidate)
            || !record.backup.as_ref().is_none_or(identity)
            || record.checksum != record.checksum()?
        {
            return Err(OutputError::conflict("output/unknown-recovery-record"));
        }
        Ok(record)
    }
    fn witness(&self) -> String {
        format!("{PREFIX}{}.cleanup.json", self.transaction_id)
    }
}
fn valid_id(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn emit(observer: Observer<'_>, phase: Phase, paths: &Observation) -> OutputResult<()> {
    observer(phase, paths).map_err(|e| OutputError::io("output/observer-interruption", e))?;
    acceptance::checkpoint(&format!("{phase:?}"), &paths.target, &paths.directory)
        .map_err(|e| OutputError::io("output/acceptance-checkpoint", e))
}
fn info_matches(expected: &FileIdentity, actual: &FileIdentity, links: u64) -> bool {
    actual.links == links && same_identity(expected, actual)
}
fn bind_expected(
    parent: Arc<Directory>,
    name: &str,
    expected: &FileIdentity,
    links: u64,
) -> OutputResult<FileBinding> {
    let binding = FileBinding::at(parent, name, links, true)?;
    if !info_matches(expected, &binding.identity, links) {
        return Err(OutputError::conflict("output/recovery-binding-mismatch"));
    }
    Ok(binding)
}
fn verify_private(directory: &Directory) -> OutputResult<()> {
    directory.verify()?;
    let metadata = directory
        .file
        .metadata()
        .map_err(|e| OutputError::io("output/recovery-directory-metadata", e))?;
    if metadata.uid() != unix::uid() || metadata.mode() & 0o7777 != 0o700 {
        return Err(OutputError::conflict(
            "output/recovery-directory-not-private",
        ));
    }
    Ok(())
}
fn verify_record(
    record: &Record,
    binding: &FileBinding,
    parent: &Directory,
    directory: Option<&Directory>,
) -> OutputResult<()> {
    parent.verify()?;
    binding.verify_at(&binding.parent, &binding.name, 1)?;
    if binding.identity.mode != 0o600
        || binding.identity.uid != unix::uid()
        || binding.identity.device != record.record_identity.device
        || binding.identity.inode != record.record_identity.inode
        || record.target.parent.device != parent.device.to_string()
        || record.target.parent.inode != parent.inode.to_string()
        || record.target.parent_path != parent.real
    {
        return Err(OutputError::conflict(
            "output/recovery-record-binding-mismatch",
        ));
    }
    let (actual, name) = Directory::for_path(&record.target.requested_path)?;
    if !actual.same_directory(parent)
        || actual.real != record.target.parent_path
        || name != record.target.name
    {
        return Err(OutputError::conflict("output/recovery-slot-changed"));
    }
    if let Some(directory) = directory {
        verify_private(directory)?;
        if directory.device.to_string() != record.recovery.identity.device
            || directory.inode.to_string() != record.recovery.identity.inode
            || directory.real != parent.real.join(&record.recovery.name)
        {
            return Err(OutputError::conflict(
                "output/recovery-directory-binding-mismatch",
            ));
        }
    }
    Ok(())
}

fn namespace_error(path: PathBuf) -> OutputError {
    let mut error = OutputError::conflict("output/unknown-recovery-namespace");
    error.preserved.push(path);
    error
}

fn namespace_names(parent: &Directory) -> OutputResult<std::collections::BTreeSet<String>> {
    use std::os::unix::ffi::OsStrExt;
    let mut names = std::collections::BTreeSet::new();
    let entries = std::fs::read_dir(&parent.real)
        .map_err(|e| OutputError::io("output/recovery-directory-read", e))?;
    for entry in entries {
        let entry = entry.map_err(|e| OutputError::io("output/recovery-directory-read", e))?;
        let name = entry.file_name();
        if !name.as_bytes().starts_with(PREFIX.as_bytes()) {
            continue;
        }
        let Some(name) = name.to_str() else {
            return Err(namespace_error(entry.path()));
        };
        let suffix = name.strip_prefix(PREFIX).unwrap();
        let id = suffix.strip_suffix(".cleanup.json").unwrap_or(suffix);
        if !valid_id(id) {
            return Err(namespace_error(entry.path()));
        }
        names.insert(name.to_owned());
    }
    Ok(names)
}

fn pending_record(
    slot: &OutputSlot,
    id: &str,
    cleanup: Option<&CleanupDirectory>,
) -> OutputResult<Option<PendingPublication>> {
    let name = format!("{PREFIX}{id}");
    let path = slot.parent.real.join(&name);
    let witness = format!("{name}.cleanup.json");
    let operation = (|| {
        let directory = if let Some(entry) = slot.parent.entry(&name)? {
            if entry.kind != "directory" || entry.uid != unix::uid() || entry.mode != 0o700 {
                return Err(namespace_error(path.clone()));
            }
            let directory = Arc::new(Directory::open(&path)?);
            if directory.device != entry.device || directory.inode != entry.inode {
                return Err(OutputError::conflict("output/recovery-directory-changed"));
            }
            verify_private(&directory)?;
            Some(directory)
        } else {
            None
        };
        let witness_exists = slot.parent.entry(&witness)?.is_some();
        let main_exists = match &directory {
            Some(directory) => directory.entry(RECORD)?.is_some(),
            None => false,
        };
        let quarantined = match cleanup {
            Some(cleanup) => cleanup.directory.entry(&format!("{id}.json"))?.is_some(),
            None => false,
        };
        if usize::from(main_exists) + usize::from(witness_exists) + usize::from(quarantined) > 1 {
            return Err(OutputError::conflict("output/ambiguous-recovery-records"));
        }
        if quarantined && directory.is_some() {
            return Err(OutputError::conflict(
                "output/cleanup-directory-still-present",
            ));
        }
        let (record_parent, record_name) = if main_exists {
            (directory.as_ref().unwrap().clone(), RECORD.to_owned())
        } else if witness_exists {
            (slot.parent.clone(), witness.clone())
        } else if quarantined {
            let cleanup = cleanup.unwrap();
            cleanup.verify()?;
            (cleanup.directory.clone(), format!("{id}.json"))
        } else {
            return Err(namespace_error(path.clone()));
        };
        let entry = record_parent
            .entry(&record_name)?
            .ok_or_else(|| OutputError::conflict("output/recovery-record-absent"))?;
        if entry.kind != "file"
            || entry.links != 1
            || entry.mode != 0o600
            || entry.uid != unix::uid()
            || entry.bytes > LIMIT
        {
            return Err(OutputError::conflict("output/recovery-record-not-private"));
        }
        let binding = FileBinding::at(record_parent.clone(), &record_name, 1, true)?;
        if binding.identity.mode != 0o600 || binding.identity.bytes > LIMIT {
            return Err(OutputError::conflict("output/recovery-record-not-private"));
        }
        let record = Record::parse(binding.bytes())?;
        if record.transaction_id != id || record.recovery.name != name {
            return Err(OutputError::conflict("output/recovery-name-mismatch"));
        }
        verify_record(&record, &binding, &slot.parent, directory.as_deref())?;
        if record.target.name != slot.name {
            return Ok(None);
        }
        Ok(Some(PendingPublication {
            target: record.target.commit_path,
            recovery_directory: path.clone(),
            witness: (witness_exists || quarantined).then(|| record_parent.real.join(&record_name)),
            phase: if witness_exists || quarantined {
                "cleanup"
            } else {
                "prepared"
            }
            .into(),
        }))
    })();
    operation.map_err(|mut error: OutputError| {
        error.public_state = unix::public_state(&slot.parent, &slot.name);
        error.recovery_directory = Some(path.clone());
        if !error.preserved.contains(&path) {
            error.preserved.push(path);
        }
        if slot.parent.entry(&witness).ok().flatten().is_some() {
            error.preserved.push(slot.parent.real.join(witness));
        }
        if let Some(cleanup) = cleanup {
            if cleanup
                .directory
                .entry(&format!("{id}.json"))
                .ok()
                .flatten()
                .is_some()
            {
                error
                    .preserved
                    .push(cleanup.directory.real.join(format!("{id}.json")));
            }
        }
        error
    })
}

pub(super) fn pending(slot: &OutputSlot) -> OutputResult<Vec<PendingPublication>> {
    slot.verify_guard()?;
    let names = namespace_names(&slot.parent)?;
    let cleanup = cleanup_directory(&slot.parent, false)?;
    let quarantined_names = cleanup_names(cleanup.as_ref())?;
    let mut ids: std::collections::BTreeSet<_> = names
        .iter()
        .map(|name| {
            name.strip_prefix(PREFIX)
                .unwrap()
                .strip_suffix(".cleanup.json")
                .unwrap_or_else(|| name.strip_prefix(PREFIX).unwrap())
        })
        .collect();
    ids.extend(
        quarantined_names
            .iter()
            .map(|name| name.strip_suffix(".json").unwrap()),
    );
    let mut pending = Vec::new();
    for id in ids {
        if let Some(record) = pending_record(slot, id, cleanup.as_ref())? {
            pending.push(record);
        }
    }
    let current_names = namespace_names(&slot.parent)?;
    let current_cleanup = cleanup_directory(&slot.parent, false)?;
    let current_quarantined_names = cleanup_names(current_cleanup.as_ref())?;
    if let Some(cleanup) = &cleanup {
        cleanup.verify()?;
    }
    slot.verify_guard()?;
    if current_names != names || current_quarantined_names != quarantined_names {
        return Err(OutputError::conflict("output/recovery-namespace-changed"));
    }
    Ok(pending)
}

// A public name is moved no-clobber into the private transaction directory.
// Only the moved inode proven by the still-open binding may then be unlinked.
fn retire(
    binding: &FileBinding,
    source: &Directory,
    name: &str,
    private: &Directory,
    retired: &str,
    links: u64,
) -> OutputResult<()> {
    binding.verify_at(source, name, links)?;
    verify_private(private)?;
    unix::rename_no_clobber(source, name, private, retired)?;
    if let Err(error) = binding.verify_at(private, retired, links) {
        let restored = unix::rename_no_clobber(private, retired, source, name);
        return Err(if restored.is_ok() {
            error
        } else {
            OutputError::new(
                ErrorKind::RecoveryRequired,
                "output/displaced-claimant-preserved",
                "A later claimant remains in private quarantine; no unknown object was removed",
            )
        });
    }
    private.unlink_private(retired, false)?;
    Ok(())
}
fn remove_private(
    binding: &FileBinding,
    private: &Directory,
    name: &str,
    retired: &str,
    links: u64,
) -> OutputResult<()> {
    retire(binding, private, name, private, retired, links)
}
fn optional_name(directory: &Directory, names: &[&str]) -> OutputResult<Option<String>> {
    let mut present = vec![];
    for name in names {
        if directory.entry(name)?.is_some() {
            present.push((*name).to_owned());
        }
    }
    if present.len() > 1 {
        return Err(OutputError::conflict("output/ambiguous-private-names"));
    }
    Ok(present.pop())
}
fn cleanup_candidate(
    record: &Record,
    directory: Arc<Directory>,
    public_links: u64,
) -> OutputResult<()> {
    if let Some(name) = optional_name(
        &directory,
        &["candidate", "candidate-retired", "published-retired"],
    )? {
        let binding = bind_expected(
            directory.clone(),
            &name,
            &record.candidate,
            public_links + 1,
        )?;
        if name == "candidate" {
            remove_private(
                &binding,
                &directory,
                &name,
                "candidate-retired",
                public_links + 1,
            )?;
        } else {
            verify_private(&directory)?;
            binding.verify_at(&directory, &name, public_links + 1)?;
            directory.unlink_private(&name, false)?;
        }
    }
    Ok(())
}

fn finish_record(
    record: &Record,
    record_binding: &FileBinding,
    parent: Arc<Directory>,
    directory: Option<Arc<Directory>>,
    paths: &Observation,
    observer: Observer<'_>,
) -> OutputResult<()> {
    emit(observer, Phase::BeforeRecordCleanup, paths)?;
    verify_record(record, record_binding, &parent, directory.as_deref())?;
    let witness = record.witness();
    let quarantined_name = format!("{}.json", record.transaction_id);
    let quarantined = record_binding.name == quarantined_name
        && record_binding.parent.real == parent.real.join(CLEANUP);
    if record_binding.name == RECORD {
        unix::rename_no_clobber(&record_binding.parent, RECORD, &parent, &witness)?;
        record_binding.verify_at(&parent, &witness, 1)?;
    } else if quarantined {
        if directory.is_some() {
            return Err(OutputError::conflict(
                "output/cleanup-directory-still-present",
            ));
        }
    } else if record_binding.name != witness || record_binding.parent.real != parent.real {
        return Err(OutputError::conflict("output/unknown-cleanup-witness"));
    }
    emit(observer, Phase::AfterRecordCleanup, paths)?;
    if let Some(directory) = directory {
        emit(observer, Phase::BeforeDirectoryCleanup, paths)?;
        verify_record(
            record,
            record_binding_at(record_binding, &parent, &witness)
                .as_ref()
                .unwrap_or(record_binding),
            &parent,
            Some(&directory),
        )?;
        parent.unlink_private(&record.recovery.name, true)?;
    }
    emit(observer, Phase::AfterDirectoryCleanup, paths)?;
    parent.verify()?;
    let cleanup = cleanup_directory(&parent, true)?
        .ok_or_else(|| OutputError::conflict("output/cleanup-directory-changed"))?;
    cleanup.verify()?;
    if !quarantined {
        record_binding.verify_at(&parent, &witness, 1)?;
        emit(observer, Phase::BeforeWitnessDelete, paths)?;
        unix::rename_no_clobber(&parent, &witness, &cleanup.directory, &quarantined_name)?;
    } else {
        record_binding.verify_at(&cleanup.directory, &quarantined_name, 1)?;
        emit(observer, Phase::BeforeWitnessDelete, paths)?;
    }
    emit(observer, Phase::AfterWitnessQuarantine, paths)?;
    cleanup.verify()?;
    if let Err(error) = record_binding.verify_at(&cleanup.directory, &quarantined_name, 1) {
        if !quarantined {
            let _ =
                unix::rename_no_clobber(&cleanup.directory, &quarantined_name, &parent, &witness);
        }
        return Err(error);
    }
    if parent.entry(&witness)?.is_some() {
        return Err(OutputError::conflict("output/cleanup-witness-reclaimed"));
    }
    cleanup.directory.unlink_private(&quarantined_name, false)?;
    cleanup.verify()?;
    if parent.entry(&witness)?.is_some() {
        return Err(OutputError::conflict("output/cleanup-witness-reclaimed"));
    }
    Ok(())
}
// verify_at supports the deliberate witness alias without replacing its fd.
fn record_binding_at(
    binding: &FileBinding,
    parent: &Arc<Directory>,
    name: &str,
) -> Option<FileBinding> {
    FileBinding::at(parent.clone(), name, 1, true)
        .ok()
        .filter(|b| same_identity(&binding.identity, &b.identity))
}

struct Transaction {
    directory: Arc<Directory>,
    record: Record,
    record_binding: FileBinding,
    stage: FileBinding,
    paths: Observation,
}
fn prepare(
    slot: &OutputSlot,
    bytes: &[u8],
    mode: u32,
    observer: Observer<'_>,
) -> OutputResult<Transaction> {
    let transaction_id = unix::transaction_id()?;
    let name = format!("{PREFIX}{transaction_id}");
    let directory = slot.parent.create_directory(&name)?;
    let paths = Observation {
        target: slot.path(),
        directory: directory.real.clone(),
        #[cfg(test)]
        stage: directory.real.join("candidate"),
        backup: directory.real.join("previous"),
    };
    let operation = (|| {
        let file = directory.create_file("candidate", bytes, mode)?;
        emit(observer, Phase::StageCreated, &paths)?;
        let stage = FileBinding::from_file(file, directory.clone(), "candidate", 1, true)?;
        if stage.content != bytes {
            return Err(OutputError::conflict("output/stage-content-changed"));
        }
        if let Err(error) = emit(observer, Phase::StageSynced, &paths) {
            if remove_private(&stage, &directory, "candidate", "candidate-retired", 1).is_ok() {
                let _ = slot.parent.unlink_private(&name, true);
            }
            return Err(error);
        }
        let mut file = directory.create_file(RECORD, &[], 0o600)?;
        let entry = file
            .metadata()
            .map_err(|e| OutputError::io("output/recovery-record-metadata", e))?;
        emit(observer, Phase::BeforeRecordWrite, &paths)?;
        let mut record = Record {
            schema_version: 1,
            tool: "phoenix-diagram".into(),
            kind: "output-publication".into(),
            transaction_id,
            target: Target {
                requested_path: slot.requested_path.clone(),
                commit_path: slot.path(),
                parent_path: slot.parent.real.clone(),
                parent: Physical::directory(&slot.parent),
                name: slot.name.clone(),
            },
            recovery: RecoveryDirectory {
                name,
                identity: Physical::directory(&directory),
            },
            backup: slot.target.as_ref().map(|b| b.identity.clone()),
            candidate: stage.identity.clone(),
            record_identity: Physical {
                device: entry.dev().to_string(),
                inode: entry.ino().to_string(),
            },
            checksum: String::new(),
        };
        let content = record.bytes()?;
        file.write_all(&content)
            .map_err(|e| OutputError::io("output/recovery-record-write", e))?;
        file.sync_all()
            .map_err(|e| OutputError::io("output/recovery-record-sync", e))?;
        directory.sync()?;
        slot.parent.sync()?;
        let record_binding = FileBinding::from_file(file, directory.clone(), RECORD, 1, true)?;
        if record_binding.content != content {
            return Err(OutputError::conflict(
                "output/recovery-record-content-changed",
            ));
        }
        verify_record(&record, &record_binding, &slot.parent, Some(&directory))?;
        Ok(Transaction {
            directory: directory.clone(),
            record,
            record_binding,
            stage,
            paths: paths.clone(),
        })
    })();
    operation.map_err(|mut error: OutputError| {
        error.public_state = unix::public_state(&slot.parent, &slot.name);
        if slot
            .parent
            .entry(
                paths
                    .directory
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(""),
            )
            .ok()
            .flatten()
            .is_some()
        {
            error.recovery_directory = Some(paths.directory.clone());
            error.preserved = vec![paths.directory.clone()];
        }
        error
    })
}

fn rollback(slot: &OutputSlot, transaction: &Transaction, mut error: OutputError) -> OutputError {
    let directory = &transaction.directory;
    let mut restored = false;
    if let Some(previous) = &slot.target {
        if previous.verify_at(&slot.parent, &slot.name, 2).is_ok() {
            if let Ok(Some(name)) = optional_name(directory, &["previous", "previous-retired"]) {
                if previous.verify_at(directory, &name, 2).is_ok() {
                    let removed = if name == "previous" {
                        remove_private(previous, directory, &name, "previous-retired", 2)
                    } else {
                        directory.unlink_private(&name, false)
                    };
                    if removed.is_ok() && previous.verify_at(&slot.parent, &slot.name, 1).is_ok() {
                        restored = true;
                    }
                }
            }
        }
        if let Ok(Some(entry)) = slot.parent.entry(&slot.name) {
            if entry.device.to_string() == transaction.stage.identity.device
                && entry.inode.to_string() == transaction.stage.identity.inode
            {
                let links = entry.links;
                let _ = retire(
                    &transaction.stage,
                    &slot.parent,
                    &slot.name,
                    directory,
                    "published-retired",
                    links,
                );
            }
        }
        if matches!(slot.parent.entry(&slot.name), Ok(None)) {
            if let Ok(Some(name)) = optional_name(directory, &["previous", "previous-retired"]) {
                if bind_expected(directory.clone(), &name, &previous.identity, 1).is_ok()
                    && unix::link(directory, &name, &slot.parent, &slot.name).is_ok()
                {
                    if previous.verify_at(&slot.parent, &slot.name, 2).is_ok()
                        && previous.verify_at(directory, &name, 2).is_ok()
                    {
                        if directory.unlink_private(&name, false).is_ok() {
                            restored = true;
                        }
                    }
                }
            }
        }
        if previous.verify_at(&slot.parent, &slot.name, 1).is_ok() {
            restored = true;
        }
    } else if matches!(
        unix::public_state(&slot.parent, &slot.name),
        PublicState::Absent
    ) {
        restored = true;
    }
    if restored && cleanup_candidate(&transaction.record, directory.clone(), 0).is_ok() {
        let _ = finish_record(
            &transaction.record,
            &transaction.record_binding,
            slot.parent.clone(),
            Some(directory.clone()),
            &transaction.paths,
            &mut |_, _| Ok(()),
        );
    }
    error.public_state = unix::public_state(&slot.parent, &slot.name);
    if slot
        .parent
        .entry(&transaction.record.recovery.name)
        .ok()
        .flatten()
        .is_some()
        || slot
            .parent
            .entry(&transaction.record.witness())
            .ok()
            .flatten()
            .is_some()
    {
        error.recovery_directory = Some(transaction.paths.directory.clone());
        error.preserved = vec![
            transaction.paths.directory.clone(),
            transaction.paths.backup.clone(),
        ];
        if !restored {
            error.kind = ErrorKind::RecoveryRequired;
        }
    }
    error
}

pub(super) fn publish_with_mode(
    slot: &OutputSlot,
    bytes: &[u8],
    mode: u32,
    protected: &[&FileBinding],
    observer: Observer<'_>,
) -> OutputResult<Publication> {
    if mode > 0o777 {
        return Err(OutputError::new(
            ErrorKind::Invalid,
            "output/invalid-mode",
            "Publication mode must contain only ordinary file permission bits",
        ));
    }
    publish_mode(slot, bytes, mode, protected, observer)
}

pub(super) fn publish(
    slot: &OutputSlot,
    bytes: &[u8],
    protected: &[&FileBinding],
    observer: Observer<'_>,
) -> OutputResult<Publication> {
    let mode = slot.target.as_ref().map_or(0o644, |b| b.identity.mode);
    publish_with_mode(slot, bytes, mode, protected, observer)
}

fn verify_final_state(
    lock: &SlotLock,
    parent: &Directory,
    name: &str,
    binding: Option<&FileBinding>,
) -> OutputResult<PublicState> {
    lock.verify()?;
    parent.verify()?;
    if let Some(binding) = binding {
        binding.verify_at(parent, name, 1)?;
        let mut identity = binding.identity.clone();
        identity.links = 1;
        Ok(PublicState::File { identity })
    } else if parent.entry(name)?.is_none() {
        Ok(PublicState::Absent)
    } else {
        Err(OutputError::conflict("output/final-binding-changed"))
    }
}

fn current_state(parent: &Directory, name: &str) -> PublicState {
    match parent.verify() {
        Ok(()) => unix::public_state(parent, name),
        Err(error) => PublicState::Unknown { reason: error.code },
    }
}

fn publish_mode(
    slot: &OutputSlot,
    bytes: &[u8],
    mode: u32,
    protected: &[&FileBinding],
    observer: Observer<'_>,
) -> OutputResult<Publication> {
    slot.verify(protected)?;
    cleanup_directory(&slot.parent, false)?;
    let transaction = prepare(slot, bytes, mode, observer)?;
    let operation = (|| {
        slot.verify(protected)?;
        transaction
            .stage
            .verify_at(&transaction.directory, "candidate", 1)?;
        verify_record(
            &transaction.record,
            &transaction.record_binding,
            &slot.parent,
            Some(&transaction.directory),
        )?;
        if let Some(previous) = &slot.target {
            unix::link(&slot.parent, &slot.name, &transaction.directory, "previous")?;
            previous.verify_at(&slot.parent, &slot.name, 2)?;
            previous.verify_at(&transaction.directory, "previous", 2)?;
            previous
                .file
                .sync_all()
                .map_err(|e| OutputError::io("output/backup-sync", e))?;
            transaction.directory.sync()?;
            slot.parent.sync()?;
        }
        emit(observer, Phase::RecoveryDurable, &transaction.paths)?;
        if let Some(previous) = &slot.target {
            emit(observer, Phase::BeforeRetire, &transaction.paths)?;
            slot.lock.verify()?;
            slot.parent.verify()?;
            for input in protected {
                input.verify().map_err(|mut e| {
                    e.code = "input/changed".into();
                    e
                })?;
            }
            previous.verify_at(&slot.parent, &slot.name, 2)?;
            previous.verify_at(&transaction.directory, "previous", 2)?;
            transaction
                .stage
                .verify_at(&transaction.directory, "candidate", 1)?;
            retire(
                previous,
                &slot.parent,
                &slot.name,
                &transaction.directory,
                "retired",
                2,
            )?;
            previous.verify_at(&transaction.directory, "previous", 1)?;
        }
        emit(observer, Phase::AfterRetire, &transaction.paths)?;
        emit(observer, Phase::BeforePublishLink, &transaction.paths)?;
        slot.lock.verify()?;
        slot.parent.verify()?;
        for input in protected {
            input.verify().map_err(|mut e| {
                e.code = "input/changed".into();
                e
            })?;
        }
        transaction
            .stage
            .verify_at(&transaction.directory, "candidate", 1)?;
        verify_record(
            &transaction.record,
            &transaction.record_binding,
            &slot.parent,
            Some(&transaction.directory),
        )?;
        unix::link(
            &transaction.directory,
            "candidate",
            &slot.parent,
            &slot.name,
        )?;
        emit(observer, Phase::AfterPublishLink, &transaction.paths)?;
        slot.lock.verify()?;
        slot.parent.verify()?;
        transaction.stage.verify_at(&slot.parent, &slot.name, 2)?;
        transaction
            .stage
            .verify_at(&transaction.directory, "candidate", 2)?;
        emit(observer, Phase::BeforeStageCleanup, &transaction.paths)?;
        slot.parent.verify()?;
        transaction.stage.verify_at(&slot.parent, &slot.name, 2)?;
        remove_private(
            &transaction.stage,
            &transaction.directory,
            "candidate",
            "candidate-retired",
            2,
        )?;
        transaction.stage.verify_at(&slot.parent, &slot.name, 1)?;
        Ok(())
    })();
    if let Err(error) = operation {
        return Err(rollback(slot, &transaction, error));
    }
    let cleanup = (|| {
        emit(observer, Phase::BeforeBackupCleanup, &transaction.paths)?;
        slot.lock.verify()?;
        slot.parent.verify()?;
        transaction.stage.verify_at(&slot.parent, &slot.name, 1)?;
        if let Some(previous) = &slot.target {
            remove_private(
                previous,
                &transaction.directory,
                "previous",
                "previous-retired",
                1,
            )?;
        }
        finish_record(
            &transaction.record,
            &transaction.record_binding,
            slot.parent.clone(),
            Some(transaction.directory.clone()),
            &transaction.paths,
            observer,
        )
    })();
    let cleanup_errors = cleanup
        .as_ref()
        .err()
        .map(|e| vec![e.code.clone()])
        .unwrap_or_default();
    let recovery_directory = cleanup
        .is_err()
        .then(|| transaction.paths.directory.clone());
    let final_state = (|| {
        for input in protected {
            input.verify().map_err(|mut error| {
                error.code = "input/changed".into();
                error
            })?;
        }
        verify_final_state(
            &slot.lock,
            &slot.parent,
            &slot.name,
            Some(&transaction.stage),
        )
    })()
    .map_err(|mut error: OutputError| {
        error.committed = true;
        error.public_state = current_state(&slot.parent, &slot.name);
        error.recovery_directory = recovery_directory.clone();
        error.preserved = vec![slot.path()];
        if let Some(path) = &recovery_directory {
            error.preserved.push(path.clone());
        }
        error
    })?;
    Ok(Publication {
        path: slot.path(),
        identity: transaction.stage.identity.clone(),
        cleanup_complete: cleanup.is_ok(),
        recovery_directory,
        public_state: final_state,
        cleanup_errors,
    })
}

fn result(
    directory: &Path,
    status: RecoveryStatus,
    code: &str,
    message: impl Into<String>,
    state: PublicState,
    preserved: Vec<PathBuf>,
) -> RecoveryResult {
    RecoveryResult {
        status,
        code: code.into(),
        message: message.into(),
        target: None,
        public_state: state,
        recovery_directory: directory.to_owned(),
        preserved,
    }
}
fn recovery_error(
    directory: &Path,
    error: OutputError,
    parent: Option<(&Directory, &str)>,
) -> RecoveryResult {
    let status = match error.kind {
        ErrorKind::Invalid => RecoveryStatus::Invalid,
        ErrorKind::Io => RecoveryStatus::Failed,
        ErrorKind::RecoveryRequired => RecoveryStatus::RecoveryRequired,
        _ => RecoveryStatus::Preserved,
    };
    let state = parent
        .map(|(p, n)| current_state(p, n))
        .unwrap_or(error.public_state);
    result(
        directory,
        status,
        &error.code,
        error.message,
        state,
        vec![directory.to_owned()],
    )
}

pub(super) fn recover(requested: &Path, observer: Observer<'_>) -> RecoveryResult {
    let path = match unix::absolute(requested) {
        Ok(p) => p,
        Err(error) => return recovery_error(requested, error, None),
    };
    if let Some(id) = path
        .file_name()
        .and_then(|s| s.to_str())
        .and_then(|name| name.strip_prefix(PREFIX))
        .filter(|id| valid_id(id))
    {
        let witness = path.with_file_name(format!("{PREFIX}{id}.cleanup.json"));
        let cleanup = path.with_file_name(CLEANUP);
        let absent = |path: &Path| {
            std::fs::symlink_metadata(path).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
        };
        if absent(&path) && absent(&witness) && absent(&cleanup) {
            return result(
                &path,
                RecoveryStatus::Absent,
                "output/recovery-absent",
                "The explicit recovery directory and its deterministic cleanup records are absent",
                PublicState::Unknown {
                    reason: "No retained recovery record".into(),
                },
                vec![],
            );
        }
    }
    let (parent, name) = match Directory::for_path(&path) {
        Ok(value) => value,
        Err(error) => return recovery_error(&path, error, None),
    };
    let Some(id) = name.strip_prefix(PREFIX).filter(|id| valid_id(id)) else {
        return recovery_error(
            &path,
            OutputError::new(
                ErrorKind::Invalid,
                "output/invalid-recovery-directory",
                "Expected one explicit .phoenix-diagram-recovery-<transactionId> directory",
            ),
            None,
        );
    };
    let witness = format!("{PREFIX}{id}.cleanup.json");
    let entry = match parent.entry(&name) {
        Ok(e) => e,
        Err(e) => return recovery_error(&path, e, None),
    };
    let directory = if let Some(entry) = entry {
        if entry.kind != "directory" || entry.uid != unix::uid() || entry.mode != 0o700 {
            return recovery_error(
                &path,
                OutputError::conflict("output/recovery-directory-not-private"),
                None,
            );
        }
        match Directory::open(&path) {
            Ok(dir) => Some(Arc::new(dir)),
            Err(e) => return recovery_error(&path, e, None),
        }
    } else {
        None
    };
    let witness_exists = match parent.entry(&witness) {
        Ok(e) => e.is_some(),
        Err(e) => return recovery_error(&path, e, None),
    };
    let cleanup = match cleanup_directory(&parent, false) {
        Ok(cleanup) => cleanup,
        Err(error) => return recovery_error(&path, error, None),
    };
    let quarantined_name = format!("{id}.json");
    let quarantined = match &cleanup {
        Some(cleanup) => match cleanup.directory.entry(&quarantined_name) {
            Ok(entry) => entry.is_some(),
            Err(error) => return recovery_error(&path, error, None),
        },
        None => false,
    };
    if directory.is_none() && !witness_exists && !quarantined {
        return result(
            &path,
            RecoveryStatus::Absent,
            "output/recovery-absent",
            "The explicit recovery directory and its deterministic cleanup records are absent",
            PublicState::Unknown {
                reason: "No retained recovery record".into(),
            },
            vec![],
        );
    }
    let main_exists = match &directory {
        Some(directory) => match directory.entry(RECORD) {
            Ok(entry) => entry.is_some(),
            Err(error) => return recovery_error(&path, error, None),
        },
        None => false,
    };
    if usize::from(main_exists) + usize::from(witness_exists) + usize::from(quarantined) > 1 {
        return recovery_error(
            &path,
            OutputError::conflict("output/ambiguous-recovery-records"),
            None,
        );
    }
    if quarantined && directory.is_some() {
        return recovery_error(
            &path,
            OutputError::conflict("output/cleanup-directory-still-present"),
            None,
        );
    }
    let (record_parent, record_name) = if main_exists {
        (directory.as_ref().unwrap().clone(), RECORD.to_owned())
    } else if witness_exists {
        (parent.clone(), witness)
    } else if quarantined {
        (
            cleanup.as_ref().unwrap().directory.clone(),
            quarantined_name,
        )
    } else {
        return recovery_error(
            &path,
            OutputError::conflict("output/recovery-record-absent"),
            None,
        );
    };
    let mut target_for_error = None;
    let mut trusted_target = None;
    let operation = (|| {
        let entry = record_parent
            .entry(&record_name)?
            .ok_or_else(|| OutputError::conflict("output/recovery-record-absent"))?;
        if entry.kind != "file"
            || entry.links != 1
            || entry.mode != 0o600
            || entry.uid != unix::uid()
            || entry.bytes > LIMIT
        {
            return Err(OutputError::conflict("output/recovery-record-not-private"));
        }
        let record_binding = FileBinding::at(record_parent.clone(), &record_name, 1, true)?;
        if record_binding.identity.mode != 0o600 || record_binding.identity.bytes > LIMIT {
            return Err(OutputError::conflict("output/recovery-record-not-private"));
        }
        let record = Record::parse(record_binding.bytes())?;
        if record.transaction_id != id || record.recovery.name != name {
            return Err(OutputError::conflict("output/recovery-name-mismatch"));
        }
        verify_record(&record, &record_binding, &parent, directory.as_deref())?;
        if let Some(cleanup) = &cleanup {
            cleanup.verify()?;
        }
        target_for_error = Some(record.target.name.clone());
        trusted_target = Some(record.target.commit_path.clone());
        let lock = SlotLock::acquire(parent.clone(), &record.target.name)?;
        verify_record(&record, &record_binding, &parent, directory.as_deref())?;
        lock.verify()?;
        let paths = Observation {
            target: record.target.commit_path.clone(),
            directory: path.clone(),
            #[cfg(test)]
            stage: path.join("candidate"),
            backup: path.join("previous"),
        };
        // A witness exists only after candidate/backup cleanup. The journal's
        // inode is unchanged by its durable no-clobber move to this sibling.
        if record_name == record.witness() || quarantined {
            let state = unix::public_state(&parent, &record.target.name);
            let restored = match (&record.backup, &state) {
                (Some(old), PublicState::File { identity }) => info_matches(old, identity, 1),
                (None, PublicState::Absent) => true,
                _ => false,
            };
            let committed = matches!(&state,PublicState::File{identity}if info_matches(&record.candidate,identity,1));
            if !restored && !committed {
                return Err(OutputError::conflict("output/cleanup-target-changed"));
            }
            let current = match &state {
                PublicState::File { identity } => Some(bind_expected(
                    parent.clone(),
                    &record.target.name,
                    identity,
                    1,
                )?),
                _ => None,
            };
            finish_record(
                &record,
                &record_binding,
                parent.clone(),
                directory.clone(),
                &paths,
                observer,
            )?;
            let state = verify_final_state(&lock, &parent, &record.target.name, current.as_ref())?;
            return Ok(result(
                &path,
                if restored {
                    RecoveryStatus::Recovered
                } else {
                    RecoveryStatus::Preserved
                },
                if restored {
                    "output/recovered"
                } else {
                    "output/committed-output-preserved"
                },
                "Cleanup of the already verified publication state is complete; this does not create a delivery receipt",
                state,
                if committed {
                    vec![record.target.commit_path.clone()]
                } else {
                    vec![]
                },
            ));
        }
        let directory = directory
            .clone()
            .ok_or_else(|| OutputError::conflict("output/recovery-directory-absent"))?;
        let current = if let Some(previous) = &record.backup {
            if directory.entry("retired")?.is_some() {
                let retired = bind_expected(directory.clone(), "retired", previous, 2)?;
                retired.verify_at(&directory, "previous", 2)?;
                directory.unlink_private("retired", false)?;
            }
            let backup_name = optional_name(&directory, &["previous", "previous-retired"])?;
            let state = unix::public_state(&parent, &record.target.name);
            match state {
                PublicState::Absent => {
                    let name = backup_name
                        .as_deref()
                        .ok_or_else(|| OutputError::conflict("output/recovery-backup-missing"))?;
                    let backup = bind_expected(directory.clone(), name, previous, 1)?;
                    emit(observer, Phase::BeforeRecoveryLink, &paths)?;
                    lock.verify()?;
                    verify_record(&record, &record_binding, &parent, Some(&directory))?;
                    backup.verify_at(&directory, name, 1)?;
                    unix::link(&directory, name, &parent, &record.target.name)?;
                    emit(observer, Phase::AfterRecoveryLink, &paths)?;
                    lock.verify()?;
                    parent.verify()?;
                    backup.verify_at(&parent, &record.target.name, 2)?;
                    backup.verify_at(&directory, name, 2)?;
                    if name == "previous" {
                        remove_private(&backup, &directory, name, "previous-retired", 2)?;
                    } else {
                        directory.unlink_private(name, false)?;
                    }
                }
                PublicState::File { identity }
                    if info_matches(
                        previous,
                        &identity,
                        if backup_name.is_some() { 2 } else { 1 },
                    ) =>
                {
                    if let Some(name) = backup_name {
                        let backup = bind_expected(directory.clone(), &name, previous, 2)?;
                        backup.verify_at(&parent, &record.target.name, 2)?;
                        if name == "previous" {
                            remove_private(&backup, &directory, &name, "previous-retired", 2)?;
                        } else {
                            directory.unlink_private(&name, false)?;
                        }
                    }
                }
                _ => return Err(OutputError::conflict("output/recovery-target-present")),
            }
            let current = FileBinding::at(parent.clone(), &record.target.name, 1, true)?;
            if !info_matches(previous, &current.identity, 1) {
                return Err(OutputError::conflict(
                    "output/recovery-final-binding-changed",
                ));
            }
            Some(current)
        } else {
            if !matches!(
                unix::public_state(&parent, &record.target.name),
                PublicState::Absent
            ) {
                return Err(OutputError::conflict("output/recovery-target-present"));
            }
            None
        };
        emit(observer, Phase::AfterRecoveryBackupCleanup, &paths)?;
        lock.verify()?;
        verify_record(&record, &record_binding, &parent, Some(&directory))?;
        cleanup_candidate(&record, directory.clone(), 0)?;
        finish_record(
            &record,
            &record_binding,
            parent.clone(),
            Some(directory),
            &paths,
            observer,
        )?;
        let state = verify_final_state(&lock, &parent, &record.target.name, current.as_ref())?;
        Ok(result(
            &path,
            RecoveryStatus::Recovered,
            "output/recovered",
            "The original output bytes or original absent slot were recovered and private cleanup is complete",
            state,
            vec![],
        ))
    })();
    let mut outcome = match operation {
        Ok(result) => result,
        Err(error) => recovery_error(
            &path,
            error,
            target_for_error.as_deref().map(|n| (parent.as_ref(), n)),
        ),
    };
    outcome.target = trusted_target;
    outcome
}
