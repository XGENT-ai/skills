#!/usr/bin/env node
// Diagram-only adaptation of scripts/build-phoenix-ui.mjs and the Cargo
// license collector in scripts/check-phoenix-deps.mjs (Apache-2.0 UI base).
// Phoenix changes: one native diagram package, no Bun/WASM or implicit fetch,
// exact Rust source tuples, isolated output and full offline license resources.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { gunzipSync } from 'node:zlib';
import bootstrap from '../skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs';
import { root, workspace, preflight, miseEnvironment, verifyFrozenSources } from './check-phoenix-diagram-source.mjs';

export const TARGETS = Object.freeze({
  'darwin-arm64': 'aarch64-apple-darwin',
  'darwin-x64': 'x86_64-apple-darwin',
  'linux-x64': 'x86_64-unknown-linux-gnu',
  'linux-arm64': 'aarch64-unknown-linux-gnu',
});
const TOOL = 'phoenix-diagram';
const TOOL_VERSION = '0.1.0';
const VIEWER_REVISION = 'phoenix-v1';
const DERIVED_ENGINE = 'skills/phoenix-diagram/ENGINE.json';
const HASH = /^[a-f0-9]{64}$/;
const COMMIT = /^[a-f0-9]{40}$/;
const CRATES = 'registry+https://github.com/rust-lang/crates.io-index';
const RUST_LICENSE_FILES = ['Apache-2.0.txt', 'BSD-2-Clause.txt', 'CC-BY-SA-4.0.txt', 'GCC-exception-3.1.txt',
  'GPL-2.0-only.txt', 'GPL-3.0-or-later.txt', 'ISC.txt', 'LLVM-exception.txt', 'MIT.txt', 'NCSA.txt', 'OFL-1.1.txt', 'Unicode-3.0.txt'];
const OFFLINE_NETWORK_PROFILE = '(version 1) (allow default) (deny network-outbound (remote ip)) (deny network-inbound (local ip))';
const encode = value => Buffer.from(JSON.stringify(value));
const pretty = value => Buffer.from(JSON.stringify(value, null, 2) + '\n');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const identity = bytes => ({ size: bytes.length, sha256: sha(bytes) });
const compare = (a, b) => Buffer.compare(Buffer.from(a), Buffer.from(b));
const rowsFor = files => [...files].map(([name, bytes]) => [name, bytes.length, sha(bytes)]).sort((a, b) => compare(a[0], b[0]));
const recordFor = (name, bytes) => ({ path: name, ...identity(bytes) });

function relative(name) {
  assert(typeof name === 'string' && name && !name.startsWith('/') && !/[\\:\0-\x1f\x7f]/.test(name)
    && !name.split('/').some(part => !part || part === '.' || part === '..'), `Unsafe relative path: ${name}`);
  return name;
}
function regular(file, { links, executable = false, mode } = {}) {
  const entry = fs.lstatSync(file, { bigint: true });
  assert(entry.isFile() && !entry.isSymbolicLink(), `Expected regular, non-symbolic file: ${file}`);
  if (links !== undefined) assert.equal(entry.nlink, BigInt(links), `Unexpected hard links: ${file}`);
  if (mode !== undefined) assert.equal(Number(entry.mode & 0o777n), mode, `File mode differs: ${file}`);
  if (executable) assert((entry.mode & 0o111n) !== 0n && (entry.mode & 0o7000n) === 0n, `Expected executable ordinary file: ${file}`);
  const descriptor = fs.openSync(file, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW | fs.constants.O_NONBLOCK);
  try {
    const before = fs.fstatSync(descriptor, { bigint: true });
    assert(before.dev === entry.dev && before.ino === entry.ino, `File changed before read: ${file}`);
    const bytes = fs.readFileSync(descriptor), after = fs.fstatSync(descriptor, { bigint: true }), current = fs.lstatSync(file, { bigint: true });
    for (const key of ['dev', 'ino', 'size', 'mode', 'nlink', 'mtimeNs', 'ctimeNs']) {
      assert.equal(after[key], before[key], `File changed during read: ${file}`);
      assert.equal(current[key], after[key], `File slot changed during read: ${file}`);
    }
    return { bytes, stat: after };
  } finally { fs.closeSync(descriptor); }
}
function walk(directory, prefix = '') {
  const stat = fs.lstatSync(directory);
  assert(stat.isDirectory() && !stat.isSymbolicLink(), `Nonregular or symbolic directory: ${directory}`);
  const files = [];
  for (const encoded of fs.readdirSync(directory, { encoding: 'buffer' })) {
    const name = new TextDecoder('utf-8', { fatal: true }).decode(encoded);
    const member = prefix ? `${prefix}/${name}` : name;
    relative(member);
    const full = path.join(directory, name), child = fs.lstatSync(full);
    if (child.isDirectory() && !child.isSymbolicLink()) files.push(...walk(full, member));
    else {
      assert(child.isFile() && !child.isSymbolicLink(), `Nonregular or symbolic source entry: ${full}`);
      files.push(member);
    }
  }
  return files.sort(compare);
}
function nativeInputRoots(repoRoot) {
  const source = regular(path.join(repoRoot, 'tools/phoenix-diagram/build.rs')).bytes.toString('utf8');
  const body = /const INPUTS:\s*&\[&str\]\s*=\s*&\[([\s\S]*?)\];/.exec(source)?.[1];
  assert(body, 'Native build.rs input roots are unavailable');
  const strings = [...body.matchAll(/"([^"\\]*)"/g)];
  assert(strings.length && /^[\s,]*$/.test(body.replace(/"[^"\\]*"/g, '')), 'Unsupported native build.rs input-root declaration');
  assert(source.includes(`Path::new("${DERIVED_ENGINE}")`), 'Native derived ENGINE exclusion differs');
  return strings.map(match => relative(match[1]));
}

export function sourceInputs(repoRoot = root) {
  const roots = nativeInputRoots(repoRoot), names = new Set();
  for (const name of roots) {
    const file = path.join(repoRoot, name), stat = fs.lstatSync(file);
    assert(!stat.isSymbolicLink(), `Symbolic native source root: ${name}`);
    if (stat.isDirectory()) for (const child of walk(file)) names.add(`${name}/${child}`);
    else { assert(stat.isFile(), `Nonregular native source root: ${name}`); names.add(name); }
  }
  names.delete(DERIVED_ENGINE);
  const entries = [...names].sort(compare).map(name => { const bytes = regular(path.join(repoRoot, name)).bytes; return [name, bytes.length, sha(bytes)]; });
  const bytes = encode(entries);
  return { roots, entries, bytes, sha256: sha(bytes) };
}
export function readSourceInputManifest(bytes) {
  const entries = JSON.parse(bytes);
  assert(Array.isArray(entries) && entries.length && encode(entries).equals(bytes), 'Source inputs must be canonical JSON without trailing bytes');
  let previous;
  for (const row of entries) {
    assert(Array.isArray(row) && row.length === 3 && Number.isSafeInteger(row[1]) && row[1] >= 0 && HASH.test(row[2]), 'Invalid source input tuple');
    relative(row[0]);
    assert(previous === undefined || compare(previous, row[0]) < 0, 'Source inputs contain duplicate or unsorted paths');
    previous = row[0];
  }
  return { entries, bytes, sha256: sha(bytes) };
}
export function assertSourceInputsUnchanged(before, after) {
  assert(before.bytes.equals(after.bytes) && before.sha256 === after.sha256, 'Source changed during native build');
}

export function selectCompilerArtifact(stdout, nativeWorkspace = workspace) {
  let records;
  try { records = stdout.split(/\r?\n/).filter(Boolean).map(line => JSON.parse(line)); }
  catch (error) { throw new Error(`Cargo returned invalid JSON: ${error.message}`); }
  const artifacts = records.filter(record => record.reason === 'compiler-artifact' && record.target?.name === TOOL
    && record.target.kind?.includes('bin') && record.profile?.test !== true && typeof record.executable === 'string'
    && record.executable && (!record.manifest_path || path.resolve(record.manifest_path) === path.join(nativeWorkspace, 'Cargo.toml')));
  assert.equal(artifacts.length, 1, 'Cargo must report exactly one native diagram binary artifact');
  const artifact = artifacts[0];
  assert(path.isAbsolute(artifact.executable), 'Cargo binary artifact path must be absolute');
  const directories = [...new Set(records.filter(record => record.reason === 'build-script-executed'
    && record.package_id === artifact.package_id && typeof record.out_dir === 'string').map(record => record.out_dir))];
  assert(directories.length === 1 && path.isAbsolute(directories[0]), 'Cargo must report the native package build-script output');
  return { executable: artifact.executable, outDir: directories[0], artifact };
}

function bundleAllowed(name) {
  if (['SKILL.md', 'agents/openai.yaml', 'schemas/README.md', 'scripts/phoenix-diagram', 'scripts/phoenix-launcher.cjs', 'scripts/phoenix-bootstrap.cjs'].includes(name)) return true;
  const parts = name.split('/');
  return parts.length === 2 && (['schemas', 'examples'].includes(parts[0]) && parts[1].endsWith('.json')
    || parts[0] === 'references' && parts[1].endsWith('.md'));
}
export function bundleFiles(repoRoot = root) {
  const directory = path.join(repoRoot, 'skills/phoenix-diagram'), files = new Map(), descriptors = [];
  const folded = new Set();
  for (const name of walk(directory)) {
    if (name === 'ENGINE.json' || path.posix.basename(name) === '.DS_Store') continue;
    assert(bundleAllowed(name) && !folded.has(name.toLowerCase()), `Unsupported or duplicate skill allowlist entry: ${name}`);
    folded.add(name.toLowerCase());
    const { bytes, stat } = regular(path.join(directory, name));
    assert((stat.mode & 0o7000n) === 0n, `Unsupported special skill permissions: ${name}`);
    const mode = stat.mode & 0o111n ? 0o755 : 0o644;
    files.set(name, bytes); descriptors.push({ path: name, ...identity(bytes), mode });
  }
  for (const name of ['SKILL.md', 'scripts/phoenix-diagram', 'scripts/phoenix-launcher.cjs', 'scripts/phoenix-bootstrap.cjs',
    ...['common', 'architecture', 'workflow', 'sequence', 'dataflow', 'lifecycle'].map(kind => `schemas/${kind}.schema.json`)]) {
    assert(files.has(name), `Missing required diagram bundle source: ${name}`);
  }
  const manifest = { schemaVersion: 1, tool: TOOL, version: TOOL_VERSION, files: descriptors }, bytes = pretty(manifest);
  return { manifest, bytes, sha256: sha(bytes), files };
}

function command(program, args, { cwd = root, env = process.env, allowFailure = false, timeout } = {}) {
  const result = spawnSync(program, args, { cwd, env, timeout, encoding: 'utf8', maxBuffer: 128 * 1024 * 1024 });
  if (result.error) throw result.error;
  if (!allowFailure && result.status !== 0) throw new Error(result.stderr || `${program} failed (${result.status})`);
  return result;
}
export function gitIdentity(repoRoot = root) {
  const git = args => command('git', args, { cwd: repoRoot }).stdout.trim();
  const sourceCommit = git(['rev-parse', 'HEAD']), sourceTree = git(['rev-parse', 'HEAD^{tree}']);
  assert(COMMIT.test(sourceCommit) && COMMIT.test(sourceTree), 'Git source commit/tree identity is unavailable');
  const status = command('git', ['status', '--porcelain=v1', '-z', '--untracked-files=all'], { cwd: repoRoot }).stdout;
  const sourceState = status.length ? 'dirty' : 'clean';
  return { sourceCommit, sourceTree, sourceState, sourceReference: sourceState === 'dirty' ? `dirty@${sourceCommit}` : sourceCommit, statusSha256: sha(status) };
}

export function parseCargoLock(text) {
  const packages = text.split(/^\[\[package\]\]\s*$/m).slice(1).map(block => {
    const get = key => { const value = new RegExp(`^${key} = ("[^"\\n]*")$`, 'm').exec(block); return value ? JSON.parse(value[1]) : undefined; };
    return { name: get('name'), version: get('version'), source: get('source'), checksum: get('checksum') };
  });
  const keys = new Set();
  assert(packages.length, 'Cargo.lock inventory is empty');
  for (const entry of packages) {
    assert(entry.name && entry.version && (!entry.source || entry.source === CRATES && HASH.test(entry.checksum)), 'Invalid Cargo.lock inventory or unsupported source');
    const key = `${entry.name}@${entry.version}`; assert(!keys.has(key), `Duplicate Cargo.lock package: ${key}`); keys.add(key);
  }
  return packages;
}

// The complete archive is checksum-bound before any registry source is trusted.
function archiveFiles(archive) {
  const data = gunzipSync(archive), files = new Map(); let nextPath;
  for (let offset = 0; offset + 512 <= data.length;) {
    const header = data.subarray(offset, offset + 512);
    if (header.every(byte => byte === 0)) return files;
    const string = (start, end) => header.subarray(start, end).toString('utf8').replace(/\0.*$/s, '');
    const prefix = string(345, 500), name = nextPath ?? `${prefix ? prefix + '/' : ''}${string(0, 100)}`;
    const size = Number.parseInt(string(124, 136).trim(), 8), type = header[156];
    assert(Number.isSafeInteger(size) && size >= 0 && offset + 512 + size <= data.length, 'Invalid cached crate archive size');
    const body = data.subarray(offset + 512, offset + 512 + size);
    if (type === 0 || type === 48) { relative(name); assert(!files.has(name), `Duplicate cached crate member: ${name}`); files.set(name, body); nextPath = undefined; }
    else if (type === 76) nextPath = body.toString('utf8').replace(/\0.*$/s, '');
    else if (type === 120) {
      for (let at = 0; at < body.length;) {
        const space = body.indexOf(32, at), length = Number(body.subarray(at, space).toString());
        assert(Number.isSafeInteger(length) && length > space - at && at + length <= body.length, 'Invalid cached crate extended header');
        const record = body.subarray(space + 1, at + length - 1).toString('utf8');
        if (record.startsWith('path=')) nextPath = record.slice(5);
        at += length;
      }
    } else assert(type === 53, `Unsupported cached crate member type: ${type}`);
    offset += 512 + Math.ceil(size / 512) * 512;
  }
  throw new Error('Cached crate archive footer is missing');
}
function registryFiles(pkg, locked) {
  const directory = path.dirname(pkg.manifest_path), registry = path.basename(path.dirname(directory));
  const archive = regular(path.resolve(directory, '../../../cache', registry, `${pkg.name}-${pkg.version}.crate`)).bytes;
  assert.equal(sha(archive), locked.checksum, `Cached crate archive differs from Cargo.lock: ${pkg.name}@${pkg.version}`);
  const prefix = `${pkg.name}-${pkg.version}/`, files = new Map();
  for (const [name, bytes] of archiveFiles(archive)) {
    assert(name.startsWith(prefix), `Unexpected cached crate archive root: ${name}`);
    const member = relative(name.slice(prefix.length));
    assert(regular(path.join(directory, member)).bytes.equals(bytes), `Modified registry source: ${pkg.name}/${member}`);
    files.set(member, bytes);
  }
  for (const name of walk(directory)) assert(files.has(name) || ['.cargo-ok', '.cargo-checksum.json'].includes(name), `Extra registry source: ${pkg.name}/${name}`);
  return files;
}
function descriptorBytes(bytes, descriptor, label) {
  assert(bytes && bytes.length === descriptor.size && sha(bytes) === descriptor.sha256, `Pinned license source differs: ${label}`);
}
function fixedSource(descriptor) {
  assert(COMMIT.test(descriptor.revision) && typeof descriptor.repository === 'string' && descriptor.repository.startsWith('https://github.com/'), 'License source is not a fixed repository revision');
  const url = new URL(descriptor.url);
  assert(url.protocol === 'https:' && !url.username && !url.password && url.pathname.includes(`/${descriptor.revision}/`), 'License source URL is not fixed to its own revision');
}
function pinnedLicenses(repoRoot, pkg, locked, archive, pin) {
  assert(pin.name === pkg.name && pin.version === pkg.version && pin.checksum === locked.checksum && pin.license === pkg.license && pin.repository === pkg.repository, `Pinned license package identity differs: ${pkg.name}`);
  const vcs = JSON.parse(archive.get('.cargo_vcs_info.json'));
  assert(vcs.git?.sha1 === pin.revision && vcs.path_in_vcs === pin.pathInVcs, `Pinned license revision differs: ${pkg.name}`);
  const applicability = pin.applicability;
  assert(applicability && sha(archive.get('Cargo.toml')) === applicability.cargoManifestSha256 && sha(archive.get('.cargo_vcs_info.json')) === applicability.vcsInfoSha256, `Pinned Cargo license applicability differs: ${pkg.name}`);
  for (const key of ['packageManifest', 'readme', 'sourceNotice']) {
    const descriptor = applicability[key]; if (!descriptor) continue;
    fixedSource(descriptor);
    assert(descriptor.repository === pin.repository && descriptor.revision === pin.revision, `License applicability repository differs: ${pkg.name}`);
    const prefix = pin.pathInVcs ? `${pin.pathInVcs}/` : '';
    assert(descriptor.sourcePath.startsWith(prefix), `License applicability path differs: ${pkg.name}`);
    const member = key === 'packageManifest' ? 'Cargo.toml.orig' : descriptor.sourcePath.slice(prefix.length);
    const bytes = archive.get(member); descriptorBytes(bytes, descriptor, `${pkg.name}/${member}`);
    if (key === 'sourceNotice') {
      const header = Buffer.from(descriptor.header);
      assert(header.length === descriptor.headerSize && sha(header) === descriptor.headerSha256 && bytes.subarray(0, header.length).equals(header), `Pinned license notice differs: ${pkg.name}`);
    }
  }
  if (applicability.workspaceManifest) {
    fixedSource(applicability.workspaceManifest);
    assert(applicability.workspaceManifest.repository === pin.repository && applicability.workspaceManifest.revision === pin.revision,
      `Pinned workspace license applicability differs: ${pkg.name}`);
  }
  assert(Array.isArray(pin.files) && pin.files.length && typeof pin.selectedLicense === 'string' && pin.license.includes(pin.selectedLicense), `Pinned full license selection is missing: ${pkg.name}`);
  return pin.files.map(descriptor => {
    fixedSource(descriptor);
    const name = relative(descriptor.path); assert(name.startsWith('licenses/'), 'Pinned license resource must remain in assets/licenses');
    const bytes = regular(path.join(repoRoot, 'tools/phoenix-diagram/assets', name)).bytes;
    descriptorBytes(bytes, descriptor, name);
    return { bytes, source: descriptor };
  });
}

export function collectLicenses({ repoRoot = root, metadata, locked, rustSysroot }) {
  const packages = new Map((metadata?.packages || []).map(pkg => [`${pkg.name}@${pkg.version}`, pkg]));
  assert(packages.size === locked.length && metadata.packages.length === packages.size, 'Cargo metadata does not cover the complete locked package inventory');
  const files = new Map(), texts = new Map();
  const preserve = (bytes, originalLocation, destination) => {
    assert(bytes?.length, `Missing full license resource: ${originalLocation}`);
    const sha256 = sha(bytes), name = destination ?? `THIRD-PARTY-LICENSES/${sha256}.txt`;
    relative(name); assert(name.startsWith('THIRD-PARTY-LICENSES/'), 'Full terms must remain in the legal resource directory');
    assert(!texts.has(name) || texts.get(name).equals(bytes), `Conflicting full license resource: ${name}`);
    texts.set(name, bytes); return { path: name, size: bytes.length, sha256, originalLocation };
  };
  assert(typeof rustSysroot === 'string' && path.isAbsolute(rustSysroot), 'A prepared pinned Rust sysroot is required for standard-library resources');
  const rustVersion = JSON.parse(regular(path.join(repoRoot, 'tools/phoenix-diagram/build-tools.lock.json')).bytes).rust;
  const rustDocs = path.join(rustSysroot, 'share/doc/rust'), rustPrefix = `THIRD-PARTY-LICENSES/rust-std-${rustVersion}`;
  assert.deepEqual(walk(path.join(rustDocs, 'licenses')), RUST_LICENSE_FILES, 'Prepared Rust standard-library license resources differ');
  const stdlib = { name: 'Rust standard library', version: rustVersion, license: 'MIT OR Apache-2.0; additional per-file and dependency terms in COPYRIGHT-library.html',
    source: { repository: 'https://github.com/rust-lang/rust', version: rustVersion, location: `rust@${rustVersion}/share/doc/rust` },
    licenses: ['COPYRIGHT-library.html', ...RUST_LICENSE_FILES.map(name => `licenses/${name}`)].map(name =>
      preserve(regular(path.join(rustDocs, name)).bytes, `rust@${rustVersion}/share/doc/rust/${name}`, `${rustPrefix}/${name}`)) };
  const runtime = [];
  for (const [name, license, source] of [
    ['Archify', 'MIT', 'tools/phoenix-diagram/upstream/LICENSE-archify-MIT.txt'],
    ['JetBrains Mono', 'OFL-1.1', 'tools/phoenix-diagram/upstream/JetBrainsMono-OFL.txt'],
    ['Phoenix UI CDP/install adaptations', 'Apache-2.0', 'tools/phoenix-diagram/src/browser/LICENSE-APACHE-2.0'],
    ['ICU 77.1 native text adaptation', 'Unicode-3.0', 'tools/phoenix-diagram/assets/icu77-LICENSE.txt'],
  ]) runtime.push({ name, license, licenses: [preserve(regular(path.join(repoRoot, source)).bytes, source)] });
  const uiSource = JSON.parse(regular(path.join(repoRoot, 'tools/phoenix-ui/UPSTREAM.json')).bytes);
  assert(uiSource.license === 'Apache-2.0' && COMMIT.test(uiSource.engineCommit), 'Apache adaptation source identity is unavailable');
  Object.assign(runtime.find(item => item.license === 'Apache-2.0'), {
    source: { repository: uiSource.repository, engineCommit: uiSource.engineCommit,
      localRepository: 'https://github.com/XGENT-ai/skills', localRevision: '4a65a1525f9ed03c775a0e18935901e1edbb773f',
      paths: ['tools/phoenix-ui/crates/browser/src/cdp.rs', 'tools/phoenix-ui/crates/browser/src/discovery.rs',
        ...['cli', 'transaction', 'disk', 'plan'].map(name => `tools/phoenix-ui/crates/install/src/${name}.rs`)] },
    modification: 'Diagram-only CDP subset with bounded startup/commands, owned profiles and complete-height PNG capture; project-local installer with independent state, file allowlist and journal ordering. No UI runtime dependency.',
    notices: [preserve(regular(path.join(repoRoot, 'tools/phoenix-ui/NOTICE.md')).bytes, 'tools/phoenix-ui/NOTICE.md')],
  });
  runtime.push(stdlib);
  const pinsFile = path.join(repoRoot, 'tools/phoenix-diagram/assets/license-sources.json');
  const pins = JSON.parse(regular(pinsFile).bytes); assert(pins.schemaVersion === 1 && Array.isArray(pins.rust), 'Invalid pinned full-license source manifest');
  const pinByName = new Map(pins.rust.map(pin => [`${pin.name}@${pin.version}`, pin]));
  assert(pinByName.size === pins.rust.length, 'Duplicate pinned full-license source');
  for (const key of pinByName.keys()) assert(locked.some(pkg => `${pkg.name}@${pkg.version}` === key), `Pinned full-license package is absent from Cargo.lock: ${key}`);
  const rust = locked.map(lock => {
    const key = `${lock.name}@${lock.version}`, pkg = packages.get(key);
    assert(pkg && (pkg.source ?? undefined) === lock.source && typeof pkg.license === 'string' && pkg.license.trim(), `Cargo lock/metadata license mismatch: ${key}`);
    if (!lock.source) {
      assert(key === `${TOOL}@${TOOL_VERSION}` && path.resolve(pkg.manifest_path) === path.join(repoRoot, 'tools/phoenix-diagram/Cargo.toml'), 'Unexpected local Rust package');
      return { name: pkg.name, version: pkg.version, license: pkg.license, source: 'local', manifest: 'tools/phoenix-diagram/Cargo.toml', licenses: [preserve(regular(path.join(repoRoot, 'LICENSE')).bytes, 'LICENSE')] };
    }
    const archive = registryFiles(pkg, lock), pin = pinByName.get(key);
    const rootNames = [...archive.keys()].filter(name => !name.includes('/') && /^(?:licen[cs]e|copying|unlicense)(?:$|[._-])/i.test(name)).sort(compare);
    if (pkg.license_file) {
      const member = relative(path.isAbsolute(pkg.license_file) ? path.relative(path.dirname(pkg.manifest_path), pkg.license_file).split(path.sep).join('/') : pkg.license_file);
      assert(archive.has(member), `Declared license file is absent from the locked crate: ${key}`);
      if (!rootNames.includes(member)) rootNames.push(member);
    }
    const supplemental = [...archive.keys()].filter(name => !name.includes('/') && /^(?:notice|copyright|authors)(?:$|[._-])/i.test(name)).sort(compare);
    const materials = pin ? pinnedLicenses(repoRoot, pkg, lock, archive, pin) : rootNames.map(name => ({ bytes: archive.get(name), source: { url: `https://docs.rs/crate/${pkg.name}/${pkg.version}/source/${name}` } }));
    assert(materials.length, `Missing complete license text for ${key}; explicitly prepare fixed full-license sources`);
    const licenses = materials.map(material => ({ ...preserve(material.bytes, material.source.url), source: material.source }));
    const notices = supplemental.map(name => preserve(archive.get(name), `https://docs.rs/crate/${pkg.name}/${pkg.version}/source/${name}`));
    return { name: pkg.name, version: pkg.version, license: pkg.license, source: lock.source, checksum: lock.checksum,
      repository: pkg.repository, licenses, ...(notices.length ? { notices } : {}), ...(pin ? { selectedLicense: pin.selectedLicense, selection: pin.selection, applicability: pin.applicability } : {}) };
  });
  rust.sort((a, b) => compare(`${a.name}@${a.version}`, `${b.name}@${b.version}`));
  const catalog = { schemaVersion: 1, tool: TOOL, runtime, rust };
  files.set('LICENSE', regular(path.join(repoRoot, 'LICENSE')).bytes);
  files.set('NOTICE.md', regular(path.join(repoRoot, 'tools/phoenix-diagram/NOTICE.md')).bytes);
  files.set('THIRD-PARTY.json', pretty(catalog));
  const notices = ['# Phoenix Diagram third-party materials', '', 'Full texts below are preserved offline; SPDX declarations alone are not substituted for missing terms.', '',
    ...runtime.map(item => `- ${item.name}${item.version ? `@${item.version}` : ''}: ${item.license}; ${item.licenses.map(file => file.path).join(', ')}${item.modification ? `\n  Source: ${item.source.repository} engine ${item.source.engineCommit}; Phoenix source ${item.source.localRevision}.\n  Modifications: ${item.modification}\n  Original notices: ${item.notices.map(file => file.path).join(', ')}` : ''}`), '',
    ...rust.map(pkg => `- ${pkg.name}@${pkg.version}: declared ${pkg.license}${pkg.selectedLicense ? `; selected ${pkg.selectedLicense}` : ''}; ${pkg.licenses.map(file => file.path).join(', ')}${pkg.selection ? `\n  ${pkg.selection}` : ''}`), ''];
  files.set('THIRD-PARTY-NOTICES.md', Buffer.from(notices.join('\n')));
  for (const [name, bytes] of [...texts].sort(([a], [b]) => compare(a, b))) files.set(name, bytes);
  return { catalog, files, sha256: sha(encode(rowsFor(files))) };
}

export function readLicenseResources(directory, locked, { repoRoot = root } = {}) {
  const files = new Map();
  for (const name of ['LICENSE', 'NOTICE.md', 'THIRD-PARTY.json', 'THIRD-PARTY-NOTICES.md']) {
    const bytes = regular(path.join(directory, name), { links: 1, mode: 0o644 }).bytes;
    assert(bytes.length, `Missing full legal resource: ${name}`); files.set(name, bytes);
  }
  const catalog = JSON.parse(files.get('THIRD-PARTY.json'));
  assert(catalog.schemaVersion === 1 && catalog.tool === TOOL && Array.isArray(catalog.rust) && Array.isArray(catalog.runtime), 'Invalid full-license catalog');
  const expected = new Map(locked.map(pkg => [`${pkg.name}@${pkg.version}`, pkg])), found = new Set();
  for (const pkg of catalog.rust) {
    const key = `${pkg.name}@${pkg.version}`, lock = expected.get(key);
    assert(lock && !found.has(key) && pkg.source === (lock.source ?? 'local') && pkg.checksum === lock.checksum
      && Array.isArray(pkg.licenses) && pkg.licenses.length, `Full license catalog omitted, duplicated or changed a locked package: ${key}`);
    found.add(key);
  }
  assert.equal(found.size, expected.size, 'Full license catalog does not cover every locked Rust package');
  const requiredRuntime = ['Archify', 'JetBrains Mono', 'Phoenix UI CDP/install adaptations', 'ICU 77.1 native text adaptation', 'Rust standard library'];
  assert.deepEqual(catalog.runtime.map(item => item.name).sort(compare), requiredRuntime.sort(compare), 'Full runtime license catalog differs');
  for (const item of [...catalog.runtime, ...catalog.rust]) {
    assert(typeof item.license === 'string' && item.license && Array.isArray(item.licenses) && item.licenses.length, `Missing complete terms: ${item.name}`);
    for (const term of [...item.licenses, ...(item.notices ?? [])]) assert(term && typeof term.path === 'string' && term.path.startsWith('THIRD-PARTY-LICENSES/')
      && HASH.test(term.sha256) && Number.isSafeInteger(term.size) && term.size > 0 && typeof term.originalLocation === 'string' && term.originalLocation, `Missing complete license resource: ${item.name}`);
  }
  const rustVersion = JSON.parse(regular(path.join(repoRoot, 'tools/phoenix-diagram/build-tools.lock.json')).bytes).rust;
  const stdlib = catalog.runtime.find(item => item.name === 'Rust standard library'), rustPrefix = `THIRD-PARTY-LICENSES/rust-std-${rustVersion}/`;
  assert.equal(stdlib.version, rustVersion, 'Rust standard-library terms differ from the toolchain pin');
  assert.deepEqual(stdlib.licenses.map(file => file.path).sort(compare), ['COPYRIGHT-library.html', ...RUST_LICENSE_FILES.map(name => `licenses/${name}`)].map(name => rustPrefix + name).sort(compare), 'Complete Rust standard-library terms or relative structure are missing');
  function visit(value) {
    if (!value || typeof value !== 'object') return;
    if (typeof value.path === 'string' && value.path.startsWith('THIRD-PARTY-LICENSES/')) {
      relative(value.path); assert(HASH.test(value.sha256) && (value.path === `THIRD-PARTY-LICENSES/${value.sha256}.txt`
        || stdlib.licenses.some(file => file.path === value.path)), 'Unsafe full-license resource');
      const bytes = regular(path.join(directory, value.path), { links: 1, mode: 0o644 }).bytes;
      assert(bytes.length && bytes.length === value.size && sha(bytes) === value.sha256, `Full license resource digest differs: ${value.path}`);
      files.set(value.path, bytes);
    }
    for (const child of Object.values(value)) visit(child);
  }
  visit(catalog);
  assert(files.size > 4, 'Full-license catalog has no complete terms');
  const actual = walk(path.join(directory, 'THIRD-PARTY-LICENSES')).map(name => `THIRD-PARTY-LICENSES/${name}`);
  assert.deepEqual(actual, [...files.keys()].filter(name => name.startsWith('THIRD-PARTY-LICENSES/')).sort(compare), 'Extra or missing full-license resources');
  return { catalog, files, sha256: sha(encode(rowsFor(files))) };
}

function maintenanceSources(repoRoot) {
  return ['tools/phoenix-diagram/build-tools.lock.json', 'tools/phoenix-diagram/upstream/SOURCE.json',
    'scripts/build-phoenix-diagram.mjs', 'scripts/check-phoenix-diagram-source.mjs', 'mise.toml', 'rust-toolchain.toml',
    'tools/phoenix-ui/build-tool-assets.json', 'tools/phoenix-ui/UPSTREAM.json', 'tools/phoenix-ui/NOTICE.md'].sort(compare).map(name => recordFor(name, regular(path.join(repoRoot, name)).bytes));
}
export function observedToolVersions(details) {
  const version = value => /(?:^|\s)v?(\d+\.\d+\.\d+)\b/.exec(value)?.[1];
  return { rust: version(details.rustc), cargo: version(details.cargo), node: version(details.node), mrBoxington: version(details.mbx) };
}
function validateTools(tools, repoRoot) {
  const pins = JSON.parse(regular(path.join(repoRoot, 'tools/phoenix-diagram/build-tools.lock.json')).bytes);
  const expected = { rust: pins.rust, node: pins.node, mrBoxington: pins.mrBoxington };
  assert.deepEqual(tools?.pins, expected, 'Build tool pins differ from the source lock');
  assert.deepEqual(tools.observed, { rust: pins.rust, cargo: pins.rust, node: pins.node, mrBoxington: pins.mrBoxington }, 'Observed build tools differ from the fixed versions');
  assert.deepEqual(tools.sources, maintenanceSources(repoRoot), 'Build maintenance-source bytes changed');
  for (const key of ['rustc', 'cargo', 'node', 'mbx']) assert(typeof tools.details?.[key] === 'string' && tools.details[key].trim(), `Missing observed tool version: ${key}`);
  assert.deepEqual(observedToolVersions(tools.details), tools.observed, 'Observed tool details differ from their parsed versions');
}
function viewerIdentity(repoRoot) {
  return { revision: VIEWER_REVISION, template: recordFor('tools/phoenix-diagram/assets/template.html', regular(path.join(repoRoot, 'tools/phoenix-diagram/assets/template.html')).bytes),
    sourceManifest: recordFor('tools/phoenix-diagram/viewer/source-manifest.json', regular(path.join(repoRoot, 'tools/phoenix-diagram/viewer/source-manifest.json')).bytes) };
}
function expectedProbe() { return `${TOOL}-engine ${TOOL_VERSION} input=1 receipt=1 viewer=${VIEWER_REVISION}\n`; }
export function validateEngine(engine, sourceSha, binary) {
  assert(engine && typeof engine === 'object', 'Native doctor engine identity is missing');
  for (const [field, actual, expected] of [
    ['sourceInputSha256', engine.sourceInputSha256, sourceSha], ['inputContractSchema', engine.inputContractSchema, 1],
    ['receiptSchema', engine.receiptSchema, 1], ['viewerRevision', engine.viewerRevision, VIEWER_REVISION],
    ['binary.sha256', engine.binary?.sha256, binary.sha256], ['binary.bytes', engine.binary?.bytes, binary.size],
  ]) assert.equal(actual, expected, `Native engine ${field} differs: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
}
export function probeNativeArtifact(executable, { binaryBytes, sourceSha, target, env = process.env }) {
  const original = regular(executable, { executable: true });
  assert(original.bytes.equals(binaryBytes), 'Native binary differs from the frozen compiler-artifact bytes before probing');
  // Cargo may hardlink its public executable and deps artifact. The runtime's
  // current_exe binding intentionally requires one link; probe an owned copy
  // of the exact compiler bytes instead of weakening that runtime contract.
  const directory = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-diagram-native-probe-')));
  const probeExecutable = path.join(directory, TOOL);
  try {
    fs.chmodSync(directory, 0o700);
    const descriptor = fs.openSync(probeExecutable, fs.constants.O_WRONLY | fs.constants.O_CREAT | fs.constants.O_EXCL | fs.constants.O_NOFOLLOW, 0o700);
    try { fs.writeFileSync(descriptor, binaryBytes); fs.fchmodSync(descriptor, 0o700); } finally { fs.closeSync(descriptor); }
    assert(regular(probeExecutable, { links: 1, mode: 0o700, executable: true }).bytes.equals(binaryBytes), 'Private native probe bytes differ');
    const probe = command(probeExecutable, ['engine-probe'], { env, timeout: 15000 });
    assert.equal(probe.stdout, expectedProbe(), 'Native engine probe identity differs');
    const doctorResult = command(probeExecutable, ['doctor', '--json'], { env, allowFailure: true, timeout: 15000 });
    let doctor;
    const binary = { asset: `${TOOL}-${target}`, ...identity(binaryBytes) };
    try {
      doctor = JSON.parse(doctorResult.stdout);
      assert.equal(doctor.toolVersion, TOOL_VERSION, 'Native doctor tool version differs');
      validateEngine(doctor.engine, sourceSha, binary);
    } catch (error) {
      throw new Error(`${error.message}; native doctor ${JSON.stringify({ exitCode: doctorResult.status, signal: doctorResult.signal, diagnostics: doctor?.diagnostics || null, stderr: doctorResult.stderr, ...(!doctor ? { stdout: doctorResult.stdout.slice(0, 4096) } : {}) })}`);
    }
    assert(regular(probeExecutable, { links: 1, mode: 0o700, executable: true }).bytes.equals(binaryBytes), 'Private native probe bytes changed');
    assert(regular(executable, { executable: true }).bytes.equals(binaryBytes), 'Native binary changed while probing');
    return { probe, doctorResult, doctor, binary, compilerLinks: Number(original.stat.nlink), probeExecutable };
  } finally { fs.rmSync(directory, { recursive: true, force: true }); }
}

export function validateDevelopmentArtifact(directory, { repoRoot = root, requireClean = false } = {}) {
  directory = path.resolve(directory);
  const versionFile = path.join(directory, 'VERSION.json');
  if (!fs.existsSync(versionFile)) throw new Error(`Missing development VERSION.json: ${directory}`);
  const versionBytes = regular(versionFile, { links: 1, mode: 0o644 }).bytes, version = JSON.parse(versionBytes);
  assert(version.distribution === 'development' && version.tool === TOOL && version.toolVersion === TOOL_VERSION
    && version.engines && Object.keys(version.engines).length === 1, 'Artifact must be a one-platform development VERSION');
  const manifest = bootstrap.manifestAt(versionFile, { packageJsonPath: path.join(repoRoot, 'package.json') });
  const target = Object.keys(version.engines)[0]; assert(TARGETS[target], 'Unsupported native artifact platform');
  const receipt = JSON.parse(regular(path.join(directory, 'build-receipt.json'), { links: 1, mode: 0o644 }).bytes);
  assert(receipt.schemaVersion === 1 && receipt.tool === TOOL && receipt.toolVersion === TOOL_VERSION && receipt.distribution === 'development'
    && receipt.npmPackageVersion === version.npmPackageVersion && receipt.target === target && receipt.host === TARGETS[target], 'Build receipt package or native host identity differs');
  assert(COMMIT.test(receipt.sourceCommit) && COMMIT.test(receipt.sourceTree) && receipt.sourceCommit === version.sourceCommit
    && ['clean', 'dirty'].includes(receipt.sourceState) && receipt.sourceReference === (receipt.sourceState === 'clean' ? receipt.sourceCommit : `dirty@${receipt.sourceCommit}`), 'Build receipt hides dirty or invalid source identity');
  if (requireClean) assert.equal(receipt.sourceState, 'clean', 'A dirty development build cannot become a candidate');
  const sourceInputsBytes = regular(path.join(directory, 'source-inputs.json'), { links: 1, mode: 0o644 }).bytes;
  const inputs = readSourceInputManifest(sourceInputsBytes), currentInputs = sourceInputs(repoRoot);
  assertSourceInputsUnchanged(inputs, currentInputs);
  assert(receipt.sourceInputSha256 === inputs.sha256 && version.sourceInputSha256 === inputs.sha256, 'Build source-input identity differs');
  assert.deepEqual(receipt.sourceInputs, recordFor('source-inputs.json', sourceInputsBytes), 'Build source-input file binding differs');
  validateTools(receipt.tools, repoRoot);
  assert.deepEqual(receipt.viewer, viewerIdentity(repoRoot), 'Reader identity differs from the source snapshot');
  const engine = version.engines[target], binaryPath = path.join(directory, engine.asset);
  const binaryBytes = regular(binaryPath, { links: 1, mode: 0o755, executable: true }).bytes;
  const binary = { asset: engine.asset, ...identity(binaryBytes) };
  assert.deepEqual(binary, engine, 'Native binary file differs from VERSION');
  assert.deepEqual(receipt.binary, binary, 'Native binary receipt differs');
  const native = receipt.native;
  assert(native?.status === 'pass' && native.host === receipt.host && native.target === receipt.host && native.sourceInputsMatch === 'pass'
    && native.engineProbe?.status === 'pass' && native.engineProbe.stdout === expectedProbe() && native.engineDoctor?.status === 'pass'
    && native.engineDoctor.toolVersion === version.toolVersion, 'Native build/run evidence is incomplete or names another host/tool version');
  validateEngine(native.engineDoctor.engine, inputs.sha256, binary);
  for (const check of ['lockedOffline', 'sourceStable', 'engineProbe', 'sourceInputs']) assert.equal(receipt.checks?.[check], 'pass', `Build did not complete ${check}`);
  assert.equal(receipt.checks.fullRuntimeTests, 'not-run-by-build-script', 'A build cannot claim runtime acceptance');
  const bundleRoot = path.join(directory, 'bundle'); bootstrap.verifyBundle(bundleRoot, manifest);
  const bundleBytes = regular(path.join(bundleRoot, 'manifest.json'), { links: 1, mode: 0o644 }).bytes;
  const bundleManifest = JSON.parse(bundleBytes), expectedBundle = bundleFiles(repoRoot);
  assert(bundleBytes.equals(expectedBundle.bytes), 'Bundle bytes differ from the source skill');
  assert.deepEqual(receipt.bundle, recordFor('bundle/manifest.json', bundleBytes), 'Bundle receipt identity differs');
  const bundleMap = new Map(bundleManifest.files.map(file => [file.path, regular(path.join(bundleRoot, relative(file.path)), { links: 1, mode: file.mode }).bytes]));
  const legal = readLicenseResources(directory, parseCargoLock(regular(path.join(repoRoot, 'tools/phoenix-diagram/Cargo.lock')).bytes.toString('utf8')), { repoRoot });
  assert.deepEqual(receipt.legal, { sha256: legal.sha256, files: rowsFor(legal.files).map(([name, size, sha256]) => ({ path: name, size, sha256 })) }, 'License inventory differs from build receipt');
  const expectedFiles = ['VERSION.json', 'source-inputs.json', 'build-receipt.json', engine.asset, 'bundle/manifest.json', 'bundle/ENGINE.json',
    ...[...bundleMap.keys()].map(name => `bundle/${name}`), ...legal.files.keys()].sort(compare);
  assert.deepEqual(walk(directory), expectedFiles, 'Development artifact contains unexpected or missing files');
  return { directory, target, version, versionBytes, receipt, sourceInputs: inputs,
    bundle: { manifest: bundleManifest, bytes: bundleBytes, sha256: sha(bundleBytes), files: bundleMap },
    licenses: legal.files, licenseSha256: legal.sha256, binary: { path: binaryPath, ...binary, bytes: binaryBytes } };
}

function outputSlot(outputDir, repoRoot) {
  outputDir = path.resolve(outputDir);
  let parent = outputDir, tail = [];
  while (!fs.existsSync(parent)) {
    try { fs.lstatSync(parent); throw new Error('Build output has a dangling symbolic ancestor'); } catch (error) { if (error.code !== 'ENOENT') throw error; }
    tail.unshift(path.basename(parent)); parent = path.dirname(parent);
  }
  const physical = path.join(fs.realpathSync(parent), ...tail);
  for (const input of nativeInputRoots(repoRoot)) {
    const source = fs.realpathSync(path.resolve(repoRoot, input));
    assert(physical !== source && !physical.startsWith(source + path.sep), 'Build output must not be inside a native source-input root');
  }
  if (fs.existsSync(outputDir)) {
    const stat = fs.lstatSync(outputDir, { bigint: true });
    assert(stat.isDirectory() && !stat.isSymbolicLink() && fs.readdirSync(outputDir).length === 0, 'Build output directory must be new or empty and non-symbolic');
    return { outputDir: physical, existing: { dev: stat.dev, ino: stat.ino } };
  }
  // lstat catches dangling symbolic entries that existsSync intentionally hides.
  try { fs.lstatSync(outputDir); throw new Error('Build output entry already exists'); } catch (error) { if (error.code !== 'ENOENT') throw error; }
  return { outputDir: physical, existing: null };
}
function publishFiles(slot, files) {
  if (slot.existing) {
    const stat = fs.lstatSync(slot.outputDir, { bigint: true });
    assert(stat.isDirectory() && !stat.isSymbolicLink() && stat.dev === slot.existing.dev && stat.ino === slot.existing.ino && fs.readdirSync(slot.outputDir).length === 0, 'Build output was claimed or changed during compilation');
  } else {
    fs.mkdirSync(path.dirname(slot.outputDir), { recursive: true });
    fs.mkdirSync(slot.outputDir, { mode: 0o755 });
  }
  const directories = new Set(['']);
  for (const [name, { bytes, mode }] of files) {
    relative(name);
    const parts = name.split('/'); parts.pop(); let prefix = '';
    for (const part of parts) {
      prefix = prefix ? `${prefix}/${part}` : part;
      if (!directories.has(prefix)) { fs.mkdirSync(path.join(slot.outputDir, prefix), { mode: 0o755 }); directories.add(prefix); }
    }
    const descriptor = fs.openSync(path.join(slot.outputDir, name), fs.constants.O_WRONLY | fs.constants.O_CREAT | fs.constants.O_EXCL | fs.constants.O_NOFOLLOW, mode);
    try { fs.writeFileSync(descriptor, bytes); fs.fchmodSync(descriptor, mode); fs.fsyncSync(descriptor); } finally { fs.closeSync(descriptor); }
  }
}
function buildCommand(args, env) {
  const invocation = ['exec', '--no-deps', '--', 'mbx', ...args];
  // Copied from Phoenix UI's offline Cargo fence; no browser or tool downloads.
  return process.platform === 'darwin'
    ? command('sandbox-exec', ['-p', OFFLINE_NETWORK_PROFILE, 'mise', ...invocation], { cwd: workspace, env })
    : command('mise', invocation, { cwd: workspace, env });
}
function observeTools(prepared, env, repoRoot) {
  const details = Object.fromEntries([['rustc', 'rust'], ['cargo', 'cargo'], ['node', 'node'], ['mbx', 'mrBoxington']].map(([program]) => [program, command('mise', ['exec', '--no-deps', '--', program, '--version'], { cwd: workspace, env }).stdout.trim()]));
  const tools = { pins: { rust: prepared.versions.rust, node: prepared.versions.node, mrBoxington: prepared.versions.mrBoxington },
    observed: observedToolVersions(details), details, sources: maintenanceSources(repoRoot) };
  validateTools(tools, repoRoot); return tools;
}
function browserObservation(doctor, target, env, repoRoot) {
  const source = JSON.parse(regular(path.join(repoRoot, 'tools/phoenix-ui/build-tool-assets.json')).bytes).linuxArmBrowser;
  assert(source?.package === 'playwright' && source.revision && HASH.test(source.sha256), 'Pinned Linux ARM browser source is unavailable');
  const observation = { status: doctor.browser?.status || 'unavailable', source, verification: 'not-run-by-build-script' };
  if (observation.status === 'available') {
    const executable = fs.realpathSync(doctor.browser.path), before = regular(executable, { executable: true }).bytes;
    const version = command(executable, ['--version'], { env, timeout: 15000 }).stdout.trim();
    assert(version && regular(executable).bytes.equals(before), 'Browser changed during version observation');
    observation.path = executable; observation.version = version; observation.identity = identity(before);
    if (target === 'linux-arm64') assert(observation.identity.sha256 === source.sha256 && version === source.actualVersion, 'Linux ARM browser differs from the explicit preparation pin');
  }
  return observation;
}

export function buildDevelopment({ outputDir, repoRoot = root }) {
  assert(repoRoot === root, 'Native builds use this checked-out diagram workspace');
  assert(typeof outputDir === 'string' && outputDir, '--output-dir is required');
  const slot = outputSlot(outputDir, repoRoot), target = `${process.platform}-${process.arch}`;
  assert(TARGETS[target], `Unsupported native diagram platform: ${target}`);
  const prepared = preflight({ checkDependencies: false });
  assert.equal(prepared.host, TARGETS[target], 'Development builds must use the native platform host');
  assert.equal(process.versions.node, prepared.versions.node, 'Run the maintenance builder with the pinned mise Node version');
  const env = miseEnvironment({ CARGO_NET_OFFLINE: 'true' }), tools = observeTools(prepared, env, repoRoot);
  const frozenSources = verifyFrozenSources(), inputs = sourceInputs(repoRoot), bundle = bundleFiles(repoRoot), source = gitIdentity(repoRoot), viewer = viewerIdentity(repoRoot);
  const locked = parseCargoLock(regular(path.join(workspace, 'Cargo.lock')).bytes.toString('utf8'));
  const metadata = JSON.parse(buildCommand(['metadata', '--manifest-path', path.join(workspace, 'Cargo.toml'), '--locked', '--offline', '--all-features', '--format-version', '1'], env).stdout);
  const rustSysroot = command('mise', ['exec', '--no-deps', '--', 'rustc', '--print', 'sysroot'], { cwd: workspace, env, timeout: 15000 }).stdout.trim();
  const legal = collectLicenses({ repoRoot, metadata, locked, rustSysroot });
  const args = ['build', '--manifest-path', path.join(workspace, 'Cargo.toml'), '--bin', TOOL, '--release', '--target', prepared.host, '--locked', '--offline', '--message-format=json'];
  const result = buildCommand(args, env); if (result.stderr) process.stderr.write(result.stderr);
  const artifact = selectCompilerArtifact(result.stdout, workspace), binaryBytes = regular(artifact.executable, { executable: true }).bytes;
  const nativeInputs = regular(path.join(artifact.outDir, 'source-inputs.json')).bytes;
  assert(nativeInputs.equals(inputs.bytes), 'Rust build.rs source-input bytes differ from the Node snapshot');
  const { probe, doctorResult, doctor, binary, compilerLinks } = probeNativeArtifact(artifact.executable, { binaryBytes, sourceSha: inputs.sha256, target, env });
  if (compilerLinks !== 1) process.stderr.write(`Phoenix Diagram native artifact: compilerLinks=${compilerLinks}, probeLinks=1, bytes=${binary.size}, sha256=${binary.sha256}\n`);
  const browser = browserObservation(doctor, target, env, repoRoot);
  assertSourceInputsUnchanged(inputs, sourceInputs(repoRoot));
  assert(bundleFiles(repoRoot).bytes.equals(bundle.bytes), 'Bundle source or executable modes changed during compilation');
  assert.deepEqual(maintenanceSources(repoRoot), tools.sources, 'Build maintenance inputs changed during compilation');
  const finalSource = gitIdentity(repoRoot);
  assert(source.sourceCommit === finalSource.sourceCommit && source.sourceTree === finalSource.sourceTree, 'Git HEAD/tree changed during native build');
  if (finalSource.sourceState === 'dirty') { source.sourceState = 'dirty'; source.sourceReference = `dirty@${source.sourceCommit}`; }
  const npmPackageVersion = JSON.parse(regular(path.join(repoRoot, 'package.json')).bytes).version;
  const version = { schemaVersion: 1, distribution: 'development', tool: TOOL, toolVersion: TOOL_VERSION, npmPackageVersion,
    sourceCommit: source.sourceCommit, sourceInputSha256: inputs.sha256, inputContractSchema: 1, receiptSchema: 1, viewerRevision: VIEWER_REVISION,
    bundleSha256: bundle.sha256, engines: { [target]: binary } }, versionBytes = pretty(version);
  const receipt = { schemaVersion: 1, distribution: 'development', tool: TOOL, toolVersion: TOOL_VERSION, npmPackageVersion, target, host: prepared.host,
    ...source, sourceInputSha256: inputs.sha256, sourceInputs: recordFor('source-inputs.json', inputs.bytes), binary,
    bundle: recordFor('bundle/manifest.json', bundle.bytes), viewer, tools, browser, frozenSources,
    legal: { sha256: legal.sha256, files: rowsFor(legal.files).map(([name, size, sha256]) => ({ path: name, size, sha256 })) },
    native: { status: 'pass', host: prepared.host, target: prepared.host, command: ['mbx', ...args], sourceInputsMatch: 'pass',
      engineProbe: { status: 'pass', stdout: probe.stdout }, engineDoctor: { status: 'pass', toolVersion: doctor.toolVersion, engine: doctor.engine }, environmentDoctorExitCode: doctorResult.status },
    checks: { lockedOffline: 'pass', sourceStable: 'pass', engineProbe: 'pass', sourceInputs: 'pass',
      network: process.platform === 'darwin' ? 'OS-denied during Cargo commands' : 'Cargo offline; OS observation belongs to CI acceptance', fullRuntimeTests: 'not-run-by-build-script' } };
  const files = new Map([[binary.asset, { bytes: binaryBytes, mode: 0o755 }], ['VERSION.json', { bytes: versionBytes, mode: 0o644 }],
    ['source-inputs.json', { bytes: inputs.bytes, mode: 0o644 }], ['bundle/manifest.json', { bytes: bundle.bytes, mode: 0o644 }], ['bundle/ENGINE.json', { bytes: versionBytes, mode: 0o644 }]]);
  for (const descriptor of bundle.manifest.files) files.set(`bundle/${descriptor.path}`, { bytes: bundle.files.get(descriptor.path), mode: descriptor.mode });
  for (const [name, bytes] of legal.files) files.set(name, { bytes, mode: 0o644 });
  // The receipt is written last. A partial directory is never a built artifact.
  files.set('build-receipt.json', { bytes: pretty(receipt), mode: 0o644 });
  publishFiles(slot, files);
  validateDevelopmentArtifact(slot.outputDir, { repoRoot });
  return { status: 'development-built', target, directory: slot.outputDir, sourceState: source.sourceState, sourceInputSha256: inputs.sha256,
    binarySha256: binary.sha256, bundleSha256: bundle.sha256, licenseSha256: legal.sha256, fullRuntimeTests: 'not-run-by-build-script' };
}

export function main(args = process.argv.slice(2)) {
  if (args.length === 1 && args[0] === '--help') {
    process.stdout.write('Usage: node scripts/build-phoenix-diagram.mjs --development --output-dir DIR\nBuilds one native development artifact, offline, into a new or empty directory. Does not publish or install.\n'); return 0;
  }
  let development = false, outputDir;
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--development' && !development) development = true;
    else if (args[i] === '--output-dir' && !outputDir && args[i + 1] && !args[i + 1].startsWith('--')) outputDir = path.resolve(args[++i]);
    else throw new Error('Usage: --development --output-dir DIR; unknown, duplicate or incomplete build argument');
  }
  assert(development && outputDir, 'A native one-platform build requires --development --output-dir DIR');
  process.stdout.write(pretty(buildDevelopment({ outputDir }))); return 0;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { process.exitCode = main(); }
  catch (error) { process.stderr.write(`Phoenix Diagram build: ${error.message}\n`); process.exitCode = 1; }
}
