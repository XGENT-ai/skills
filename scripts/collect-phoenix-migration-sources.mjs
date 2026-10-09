#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import vm from 'node:vm';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
export const sha256 = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const commits = {
  '0.3.0': { version: '7b9d026f3e0f98b6bffb6a5359500b4a6371ecd6', files: 'f1e255432167e0502e1d254dc2c2c22da296625b' },
  '0.4.0': { version: '54c565c1045fc409e122f6cb4741d1416ef3a2ac', files: '47244247536e1cf20f4f4fea8b5be077a8658a2e' },
  '0.5.0': { version: '19999ebfc0a7eb213ed2716b649b444ef97d4bd9', files: '19999ebfc0a7eb213ed2716b649b444ef97d4bd9' },
  '0.6.0': { version: '4d70f3373e465c82f00e7724ee0f229a56f69cb9', files: '4d70f3373e465c82f00e7724ee0f229a56f69cb9' },
};
const publishedSha256 = {
  '0.3.0': 'bcc80efc6f0df3cc7f91fd38af1fcbdd39f38701e62c7bebfdca640f0b1f7027',
  '0.4.0': '8a00ca29d5b2efd99bb79beb788bfcae02e7c294a559aa210f752c748b6c062d',
  '0.5.0': '74376bcf6ea3227857653b184e21fb2cbcc85058a503346ca878aa2d40934119',
  '0.6.0': 'e4971f94d2e1b0d767d1ed18c1d1446822f19d53a870b850dbf2675d6828c679',
};

export function installerDeclarations(pkg) {
  const file = path.join(pkg, 'bin/xgent-skills.js');
  const installer = fs.readFileSync(file, 'utf8');
  const boundary = installer.indexOf('const [cmd, ...args] = process.argv.slice(2);');
  if (boundary < 0) throw new Error(`Unrecognized installer: ${file}`);
  // Only declarations are evaluated; install(), engine fetch and CLI dispatch never run.
  const context = vm.createContext({ require: createRequire(file), __dirname: path.dirname(file),
    process: { env: {}, argv: [], platform: process.platform, arch: process.arch }, console, Buffer });
  vm.runInContext(installer.slice(0, boundary) + '\nglobalThis.result = { rewriteHookValue, HOOK_ARTIFACTS, LEGACY_HOOK_SCRIPT_MARKERS, MANAGED_SETTINGS };', context);
  return context.result;
}

function gitFile(commit, file, optional = false) {
  const result = spawnSync('git', ['show', `${commit}:${file}`], { cwd: root, maxBuffer: 8 * 1024 * 1024 });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    if (optional) return null;
    throw new Error(`Missing fixed Git input ${commit}:${file}`);
  }
  return result.stdout;
}

function verifyArchive(archive, pkg, capture) {
  const bytes = fs.readFileSync(archive);
  const integrity = `sha512-${crypto.createHash('sha512').update(bytes).digest('base64')}`;
  const shasum = crypto.createHash('sha1').update(bytes).digest('hex');
  if (sha256(bytes) !== publishedSha256[capture.version] || capture.integrity !== integrity
    || capture.shasum !== shasum || capture.size !== bytes.length) {
    throw new Error(`Published tarball integrity mismatch: ${archive}`);
  }
  // Check extracted bytes against the verified archive, including non-executable skills.
  const result = spawnSync('python3', ['-c', `
import hashlib, json, pathlib, sys, tarfile
pkg = pathlib.Path(sys.argv[2])
files = {}
with tarfile.open(sys.argv[1], 'r:gz') as archive:
    for entry in archive:
        rel = pathlib.PurePosixPath(entry.name)
        if not rel.parts or rel.parts[0] != 'package' or '..' in rel.parts or rel.is_absolute():
            raise ValueError('unsafe archive path: ' + entry.name)
        if entry.isdir():
            continue
        if not entry.isfile():
            raise ValueError('unsupported archive entry: ' + entry.name)
        relative = str(pathlib.PurePosixPath(*rel.parts[1:]))
        packed = archive.extractfile(entry).read()
        actual = (pkg / relative).read_bytes()
        if actual != packed or relative in files:
            raise ValueError('extracted bytes differ or duplicate: ' + relative)
        files[relative] = {'sha256': hashlib.sha256(packed).hexdigest(), 'size': len(packed)}
actual_files = set()
for entry in pkg.rglob('*'):
    if entry.is_symlink():
        raise ValueError('unexpected extracted symlink: ' + str(entry))
    if entry.is_file():
        actual_files.add(str(entry.relative_to(pkg)))
if actual_files != set(files):
    raise ValueError('extracted file list differs from archive')
print(json.dumps(files, sort_keys=True))
`, archive, pkg], { encoding: 'utf8', maxBuffer: 8 * 1024 * 1024 });
  if (result.error || result.status !== 0) throw new Error(result.error?.message || result.stderr.trim());
  const files = JSON.parse(result.stdout);
  if (Object.keys(files).length !== capture.entryCount) throw new Error(`Archive entry count differs: ${archive}`);
  if (Object.values(files).reduce((sum, file) => sum + file.size, 0) !== capture.unpackedSize) throw new Error(`Archive unpacked size differs: ${archive}`);
  return { files, evidence: { filename: path.basename(archive), url: `https://registry.npmjs.org/@xgent-ai/skills/-/skills-${capture.version}.tgz`,
    sha256: sha256(bytes), shasum, integrity, size: bytes.length, unpackedSize: capture.unpackedSize,
    verifiedFileCount: Object.keys(files).length, extractedBytesVerified: true } };
}

export function collectSources(packages, archives = path.dirname(path.resolve(packages))) {
  const captures = JSON.parse(fs.readFileSync(path.join(archives, 'published-packages.json')));
  const out = path.join(root, 'tools/phoenix-ui/migration-sources');
  const fixtureDir = path.join(root, 'test/phoenix-ui/fixtures/migration');
  fs.mkdirSync(out, { recursive: true });
  fs.mkdirSync(fixtureDir, { recursive: true });
  const recipes = [];
  for (const [version, commit] of Object.entries(commits)) {
    const pkg = path.resolve(packages, version, 'package');
    const capture = captures.find(entry => entry.version === version && entry.name === '@xgent-ai/skills');
    if (!capture) throw new Error(`Missing npm pack evidence for ${version}`);
    const archive = verifyArchive(path.join(archives, capture.filename), pkg, capture);
    const metaFile = 'vendor/impeccable/VERSION.json';
    const manifestFile = 'vendor/impeccable/bundle/manifest.json';
    const installerFile = 'bin/xgent-skills.js';
    const meta = JSON.parse(fs.readFileSync(path.join(pkg, metaFile)));
    const manifest = JSON.parse(fs.readFileSync(path.join(pkg, manifestFile)));
    if (JSON.parse(fs.readFileSync(path.join(pkg, 'package.json'))).version !== version) throw new Error(`Wrong package version ${version}`);
    const declarations = installerDeclarations(pkg);
    const comparisons = {};
    for (const file of [installerFile, metaFile, manifestFile]) {
      const gitBytes = gitFile(commit.version, file, true);
      const npmSha256 = archive.files[file].sha256;
      comparisons[file] = { npmSha256, gitSha256: gitBytes && sha256(gitBytes), matches: gitBytes !== null && sha256(gitBytes) === npmSha256 };
    }
    const migrationFiles = Object.entries(archive.files).filter(([file]) =>
      [installerFile, metaFile, manifestFile].includes(file) || file.startsWith('vendor/impeccable/bundle/blobs/'));
    for (const [file, entry] of migrationFiles) {
      if (sha256(gitFile(commit.files, file)) !== entry.sha256) throw new Error(`Git migration input differs: ${commit.files}:${file}`);
    }
    const blobHashes = new Set(Object.values(manifest.providers).flatMap(entry => Object.values(entry.files)));
    for (const hash of blobHashes) {
      const bytes = fs.readFileSync(path.join(pkg, 'vendor/impeccable/bundle/blobs', hash));
      if (sha256(bytes) !== hash) throw new Error(`Corrupt bundle blob ${version}/${hash}`);
    }
    const providers = {};
    for (const [provider, entry] of Object.entries(manifest.providers)) {
      const artifact = declarations.HOOK_ARTIFACTS[provider] || (provider === '.codex'
        ? { src: ['.codex', 'hooks.json'], dest: ['.codex', 'hooks.json'] } : undefined);
      let hooks = [];
      if (artifact) {
        const rel = artifact.src.slice(1).join('/');
        const hash = manifest.providers[artifact.src[0]].files[rel];
        if (hash) {
          const raw = JSON.parse(fs.readFileSync(path.join(pkg, 'vendor/impeccable/bundle/blobs', hash)));
          hooks = [{ shape: 'bundle', blobSha256: hash, value: raw }];
          if (provider !== '.codex') hooks.push({ shape: 'xgent-installer',
            value: JSON.parse(JSON.stringify(declarations.rewriteHookValue(raw, provider))) });
        }
      }
      providers[provider] = { ...entry, installerSelectable: provider !== '.codex',
        hookArtifact: artifact && { source: artifact.src.join('/'), destination: artifact.dest.join('/'), shared: artifact.shared?.join('/') }, hooks };
      recipes.push({ packageVersion: version, provider, source: `${version}.json`,
        channels: provider === '.codex' ? ['bundle'] : ['xgent-installer', 'bundle'],
        hookShapes: hooks.map(h => h.shape), scenarios: ['unmodified', 'modified-skill', 'copied-skill', 'symlink-skill'] });
    }
    const result = { schemaVersion: 1, packageVersion: version, sourceCommit: commit.files,
      skillVersion: meta.skillVersion, engineVersion: meta.engineVersion,
      installerSha256: archive.files[installerFile].sha256, versionSha256: archive.files[metaFile].sha256,
      bundleManifestSha256: archive.files[manifestFile].sha256, tarball: archive.evidence.url,
      provenance: { primary: 'verified-published-npm-tarball', npm: archive.evidence,
        git: { versionReference: commit.version, comparison: comparisons, migrationFilesCommit: commit.files,
          gitPackageVersion: JSON.parse(gitFile(commit.files, 'package.json')).version,
          scope: 'installer, VERSION.json, bundle manifest and all published bundle blobs; not the complete npm package tree',
          matchedFileCount: migrationFiles.length, allMigrationFilesMatch: true } },
      legacyHooks: { markers: Array.from(declarations.LEGACY_HOOK_SCRIPT_MARKERS),
        evidence: installerFile, releasedScriptBodiesAvailable: false },
      xgent: { managedSettings: JSON.parse(JSON.stringify(declarations.MANAGED_SETTINGS)),
        hookFiles: Object.fromEntries(Object.entries(archive.files).filter(([file]) => file.startsWith('.claude/hooks/'))
          .map(([file, entry]) => [file, entry.sha256])) }, providers };
    fs.writeFileSync(path.join(out, `${version}.json`), JSON.stringify(result, null, 2) + '\n');
  }
  const additionalCommit = 'b2289fa05f1e4f0251bc3a86e3c04f182aba156e';
  const names = ['design-md', 'design-reference', 'ui-pattern-research', 'review-ui', 'apply-ui-review'];
  const listing = spawnSync('git', ['ls-tree', '-r', '--name-only', additionalCommit, ...names.map(name => `skills/${name}`)], { cwd: root, encoding: 'utf8' });
  if (listing.error || listing.status !== 0) throw new Error('Cannot read fixed additional skill sources');
  const files = Object.fromEntries(listing.stdout.trim().split('\n').map(file => [file.slice('skills/'.length), sha256(gitFile(additionalCommit, file))]));
  fs.writeFileSync(path.join(fixtureDir, 'additional-skills.json'), JSON.stringify({ schemaVersion: 1,
    sourceRepository: 'https://github.com/XGENT-ai/skills', sourceCommit: additionalCommit,
    channel: 'user-installed-skills-sh-or-copy', installedByOldNpmInstaller: false, files }, null, 2) + '\n');
  const upstream = JSON.parse(fs.readFileSync(path.join(root, 'tools/phoenix-ui/UPSTREAM.json')));
  const pinCases = ['pin-polish', 'pin-opencode-project'];
  const pinGoldens = Object.fromEntries(pinCases.map(name => {
    const rel = `tests/oracle/golden/${name}.json`;
    const bytes = fs.readFileSync(path.join(root, 'tools/phoenix-ui', rel));
    if (sha256(bytes) !== upstream.importedFiles[rel]) throw new Error(`Fixed pin golden changed: ${rel}`);
    return [name, JSON.parse(bytes).files];
  }));
  fs.writeFileSync(path.join(fixtureDir, 'pins.json'), JSON.stringify({ schemaVersion: 1,
    sourceCommit: upstream.sourceCommit, skillVersion: upstream.skillVersion, engineVersion: upstream.engineVersion,
    sourcePath: 'crates/context/src/pin.rs', command: 'polish',
    templates: { skill: pinGoldens['pin-polish']['.claude/skills/polish/SKILL.md'],
      codexSkill: pinGoldens['pin-polish']['.agents/skills/polish/SKILL.md'],
      opencodeCommand: pinGoldens['pin-opencode-project']['.opencode/commands/impeccable-polish.md'] } }, null, 2) + '\n');
  fs.writeFileSync(path.join(fixtureDir, 'matrix.json'), JSON.stringify({ schemaVersion: 1, recipes,
    additionalScenarios: ['mixed-handlers', 'custom-allow', 'modified-handler', 'legacy-js-hooks', 'pin-shortcut', 'modified-pin',
      'live-residue', 'personal-shadow', 'unknown-upstream', 'unrelated-entry', 'claude-shared-and-local',
      'additional-skills', 'modified-auxiliary', 'symlink-auxiliary', 'skills-only'] }, null, 2) + '\n');
  return { packageSources: Object.keys(commits).length, providerRecipes: recipes.length };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (!process.argv[2]) throw new Error('Usage: node scripts/collect-phoenix-migration-sources.mjs EXTRACTED_PACKAGES_DIR [TARBALLS_DIR]');
  console.log(JSON.stringify(collectSources(process.argv[2], process.argv[3])));
}
