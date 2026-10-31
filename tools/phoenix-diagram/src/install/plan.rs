// Provider names/defaults adapted from Phoenix UI install/cli.rs (Apache-2.0).
// Diagram writes only skills/phoenix-diagram; Codex's .codex half has no hooks.
use super::bundle::{self, Bundle};
use super::disk::{self, Directory, Snapshot};
use super::state::{self, Active, Inventory, Owned};
use super::{
    InstallConflict, InstallReceipt, InstallRequest, InstallResult, InstallStatus, InstallWrite,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub(super) const PROVIDERS: [&str; 19] = [
    ".agent",
    ".agents",
    ".claude",
    ".codex",
    ".cursor",
    ".dsh",
    ".gemini",
    ".github",
    ".grok",
    ".hermes",
    ".kiro",
    ".opencode",
    ".pi",
    ".qoder",
    ".rovodev",
    ".trae",
    ".trae-cn",
    ".veto",
    ".vibe",
];
pub(super) struct Change {
    pub path: String,
    pub bytes: Vec<u8>,
    pub mode: u32,
    pub before: Option<Snapshot>,
}
pub(super) struct Prepared {
    pub project: Arc<Directory>,
    pub bundle: Bundle,
    pub receipt: InstallReceipt,
    pub inventory: Inventory,
    pub changes: Vec<Change>,
    pub carried: BTreeMap<String, crate::output::FileIdentity>,
    pub directories: BTreeMap<String, Arc<Directory>>,
}
fn providers(requested: Option<&[String]>, project: &Arc<Directory>) -> InstallResult<Vec<String>> {
    let mut result = BTreeSet::new();
    if let Some(tokens) = requested {
        if tokens.is_empty() {
            return Err(super::invalid(
                "install/empty-providers",
                "Choose at least one provider",
            ));
        }
        for token in tokens {
            let token = token.trim();
            let provider = if token == "codex" {
                ".agents".into()
            } else if token.starts_with('.') {
                token.to_owned()
            } else {
                format!(".{token}")
            };
            if !PROVIDERS.contains(&provider.as_str()) {
                return Err(super::invalid("install/unknown-provider", token));
            }
            result.insert(provider);
        }
    } else {
        for provider in PROVIDERS {
            if project.child(provider)?.is_some() {
                result.insert(if provider == ".codex" {
                    ".agents".into()
                } else {
                    provider.into()
                });
            }
        }
        if result.is_empty() {
            result.insert(".claude".into());
        }
    }
    if result.contains(".agents") || result.contains(".codex") {
        result.insert(".agents".into());
        result.insert(".codex".into());
    }
    Ok(result.into_iter().collect())
}
fn add(receipt: &mut InstallReceipt, error: super::InstallError, path: &str) {
    receipt.conflicts.push(InstallConflict {
        path: path.into(),
        code: error.code,
        message: error.message,
    });
}
fn inventory_skill(
    project: &Arc<Directory>,
    root: &str,
    allowed: &BTreeSet<String>,
    receipt: &mut InstallReceipt,
    count: &mut usize,
) -> InstallResult<()> {
    let Some(directory) = project.descend(root)? else {
        return Ok(());
    };
    for name in directory.list()? {
        *count += 1;
        if *count > 250_000 {
            return Err(disk::conflict("install/project-inventory-incomplete", root));
        }
        let path = format!("{root}/{name}");
        let entry = directory
            .entry(&name)?
            .ok_or_else(|| disk::conflict("install/project-changed", &path))?;
        if entry.kind == libc::S_IFDIR
            && allowed
                .iter()
                .any(|file| file.starts_with(&format!("{path}/")))
        {
            inventory_skill(project, &path, allowed, receipt, count)?;
        } else if entry.kind != libc::S_IFREG || !allowed.contains(&path) {
            receipt.conflicts.push(InstallConflict {
                path: path.clone(),
                code: "install/extra-file".into(),
                message: "Unknown files, directories, or links in the diagram skill are preserved"
                    .into(),
            });
        }
    }
    Ok(())
}
pub(super) fn prepare(request: &InstallRequest) -> InstallResult<Prepared> {
    let project = Directory::open(&request.project)?;
    let bundle = Bundle::load(request)?;
    prepare_loaded(request, project, bundle)
}
pub(super) fn prepare_loaded(
    request: &InstallRequest,
    project: Arc<Directory>,
    bundle: Bundle,
) -> InstallResult<Prepared> {
    let mut receipt = super::receipt("install", &project.path);
    receipt.engine = Some(bundle.engine.clone());
    receipt.npm_package_version = Some(bundle.version.npm_package_version.clone());
    receipt.bundle_manifest_sha256 = Some(bundle.manifest_sha256.clone());
    receipt.version_manifest_sha256 = Some(bundle.version_sha256.clone());
    let recovery_entry = if bundle.version.distribution == "release" {
        format!(
            "npm exec --package=@xgent-ai/skills@{} -- phoenix-diagram",
            bundle.version.npm_package_version
        )
    } else {
        super::quote(&request.engine)
    };
    receipt.recovery_command = Some(format!(
        "{recovery_entry} install --project {} --bundle-root {} --json",
        super::quote(&project.path),
        super::quote(&bundle.root.path)
    ));
    let mut changes = vec![];
    let mut carried = BTreeMap::new();
    let mut inventory = Inventory::default();
    let mut directories = BTreeMap::new();
    let preparation = (|| -> InstallResult<()> {
        receipt.providers = providers(request.providers.as_deref(), &project)?;
        inventory = state::inspect(&project)?;
        if !inventory.pending.is_empty() || !inventory.publications.is_empty() {
            receipt.status = InstallStatus::RecoveryRequired;
            for journal in &inventory.pending {
                receipt.transaction_id = Some(journal.transaction_id.clone());
                receipt.journal = Some(project.path.join(format!(
                    "{}/{id}.json",
                    state::JOURNALS,
                    id = journal.transaction_id
                )));
                receipt.backup = Some(project.path.join(&journal.active.backup));
            }
            return Ok(());
        }
        let owned: BTreeMap<String, _> = inventory
            .active
            .as_ref()
            .map(|(active, _)| {
                active
                    .ownership
                    .iter()
                    .map(|o| (o.path.clone(), o.identity.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let mut allowed: BTreeSet<_> = owned.keys().cloned().collect();
        for provider in receipt.providers.iter().filter(|p| p.as_str() != ".codex") {
            for path in bundle.files.keys() {
                allowed.insert(format!("{provider}/skills/phoenix-diagram/{path}"));
            }
        }
        // Committed ownership is tied to the original physical file. A later
        // replacement is preserved even when its bytes and mode are identical.
        for (path, identity) in &owned {
            match project.read_path(path)? {
                Some(current) if disk::matches(identity, &current.identity) => { carried.insert(path.clone(), current.identity); },
                _ => receipt.conflicts.push(InstallConflict { path: path.clone(), code: "install/managed-file-modified".into(), message: "A previously managed file changed or is missing; the installer preserves it".into() }),
            }
        }
        let mut roots = BTreeSet::new();
        for path in &allowed {
            let provider = path.split('/').next().expect("managed path");
            roots.insert(format!("{provider}/skills/phoenix-diagram"));
        }
        for root in roots {
            inventory_skill(&project, &root, &allowed, &mut receipt, &mut 0)?;
        }
        for provider in receipt.providers.iter().filter(|p| p.as_str() != ".codex") {
            for (path, source) in &bundle.files {
                let target = format!("{provider}/skills/phoenix-diagram/{path}");
                let current = project.read_path(&target)?;
                if current.is_some() && !owned.contains_key(&target) {
                    receipt.conflicts.push(InstallConflict {
                        path: target,
                        code: "install/unowned-file".into(),
                        message:
                            "Existing bytes have no valid diagram ownership and are not adopted"
                                .into(),
                    });
                    continue;
                }
                let mut ancestor = project.clone();
                let mut relative = String::new();
                for part in target.rsplit_once('/').expect("target parent").0.split('/') {
                    ancestor.writable()?;
                    let Some(next) = ancestor.child(part)? else {
                        break;
                    };
                    if next.physical.device != project.physical.device {
                        return Err(disk::conflict("install/cross-device-parent", &target));
                    }
                    if !relative.is_empty() {
                        relative.push('/');
                    }
                    relative.push_str(part);
                    if let Some(previous) = directories.insert(relative.clone(), next.clone()) {
                        previous.verify()?;
                        if previous.physical != next.physical {
                            return Err(disk::conflict("install/parent-changed", &relative));
                        }
                    }
                    ancestor = next;
                }
                ancestor.writable()?;
                if current.as_ref().is_some_and(|file| {
                    file.identity.sha256 == source.identity.sha256
                        && file.identity.bytes == source.identity.bytes
                        && file.identity.mode == source.identity.mode
                }) {
                    receipt.unchanged.push(target);
                } else {
                    receipt.writes.push(InstallWrite {
                        path: target.clone(),
                        after: bundle::digest(&source.bytes),
                        mode: source.identity.mode,
                    });
                    changes.push(Change {
                        path: target,
                        bytes: source.bytes.clone(),
                        mode: source.identity.mode,
                        before: current,
                    });
                }
            }
        }
        if let Some((active, _)) = &inventory.active {
            if changes.is_empty()
                && active.bundle_manifest_sha256 == bundle.manifest_sha256
                && active.engine_manifest_sha256 == bundle.version_sha256
                && active.npm_package_version == bundle.version.npm_package_version
            {
                receipt.status = InstallStatus::Unchanged;
                receipt.transaction_id = Some(active.transaction_id.clone());
                receipt.journal = Some(project.path.join(&active.journal));
                receipt.backup = Some(project.path.join(&active.backup));
            }
        }
        Ok(())
    })();
    if let Err(error) = preparation {
        if error.kind == super::InstallErrorKind::Invalid {
            return Err(error);
        }
        if error.kind == super::InstallErrorKind::Io {
            return Err(error);
        }
        add(&mut receipt, error, &request.project.display().to_string());
    }
    if !receipt.conflicts.is_empty() {
        receipt.status = InstallStatus::Conflict;
        receipt.writes.clear();
        changes.clear();
    }
    super::diagnostics(&mut receipt);
    Ok(Prepared {
        project,
        bundle,
        receipt,
        inventory,
        changes,
        carried,
        directories,
    })
}

pub(super) fn verify_inventory(project: &Arc<Directory>, active: &Active) -> InstallResult<()> {
    let allowed: BTreeSet<_> = active.ownership.iter().map(|o| o.path.clone()).collect();
    let roots: BTreeSet<_> = active
        .ownership
        .iter()
        .map(|o| {
            format!(
                "{}/skills/phoenix-diagram",
                o.path.split('/').next().expect("owned provider")
            )
        })
        .collect();
    let mut receipt = super::receipt("install", &project.path);
    for root in roots {
        inventory_skill(project, &root, &allowed, &mut receipt, &mut 0)?;
    }
    if let Some(conflict) = receipt.conflicts.first() {
        return Err(disk::conflict(&conflict.code, &conflict.path));
    }
    Ok(())
}

pub(super) fn active(
    prepared: &Prepared,
    id: &str,
    ownership: BTreeMap<String, crate::output::FileIdentity>,
) -> InstallResult<Active> {
    let mut providers: BTreeSet<_> = prepared.receipt.providers.iter().cloned().collect();
    if let Some((old, _)) = &prepared.inventory.active {
        providers.extend(old.providers.iter().cloned());
    }
    let ownership: Vec<_> = ownership
        .into_iter()
        .map(|(path, identity)| Owned { path, identity })
        .collect();
    let plan_sha256 = bundle::hash(&state::encoded(&(
        &prepared.bundle.manifest_sha256,
        &prepared.bundle.version_sha256,
        &providers,
        &ownership,
    ))?);
    let mut active = Active {
        schema_version: 2,
        tool: "phoenix-diagram".into(),
        tool_version: crate::TOOL_VERSION.into(),
        npm_package_version: prepared.bundle.version.npm_package_version.clone(),
        engine: prepared.bundle.engine.clone(),
        bundle_manifest_sha256: prepared.bundle.manifest_sha256.clone(),
        engine_manifest_sha256: prepared.bundle.version_sha256.clone(),
        providers: providers.into_iter().collect(),
        transaction_id: id.into(),
        journal: format!("{}/{id}.json", state::JOURNALS),
        backup: format!("{}/{id}/before", state::TRANSACTIONS),
        project: prepared.project.physical.clone(),
        ownership,
        plan_sha256,
        checksum: String::new(),
    };
    active.seal()?;
    Ok(active)
}
