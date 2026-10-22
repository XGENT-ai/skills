#!/usr/bin/env node
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { gunzipSync } from 'node:zlib';

export const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
export const workspace = path.join(root, 'tools/phoenix-diagram');
const upstreamRoot = path.join(workspace, 'upstream');
const referenceSha256 = '4f892b0c68fef5d21788264ba6a43fe9e1d377d3c2d5b558f6aa346784238395';
const archiveSha256 = '6831ba7f7d9d5c0fd473e317bd9b513f2581b9d4d63bd157d1de4680af3b32a4';
const legacySha256 = '9b02b58082c1872f611fff050b225026e108ca53efa738da429965db9966d273';
const legacyRevision = '4a65a1525f9ed03c775a0e18935901e1edbb773f';
const nativeTextIdentity = {
  repository: 'https://github.com/unicode-org/icu',
  release: 'release-77-1',
  icu4xRepository: 'https://github.com/unicode-org/icu4x',
  icu4xRelease: 'icu@2.0.0',
  unicodeVersion: '16.0',
  compiledDataGeneration: 'ICU4X 2.0 / CLDR 47',
  manifest: 'tests/fixtures/text-segmentation/sources.json',
  manifestSize: 3284,
  manifestSha256: '28c4de15cce787d92c638a9d54bdbc12c329e30caba55e3935f1b33515f14aff',
  fileCount: 12,
  license: 'assets/icu77-LICENSE.txt',
  licenseSize: 26487,
  licenseSha256: '451167c55c0fa447cc2d5632714f5e3c567fe4f1e1badefab2c1333852198aca',
  licenseSpdx: 'Unicode-3.0',
  modification: 'Rust ports of word rules, dictionary dispatch, CJK cost segmentation and Southeast Asian three-word lookahead; original source fixtures are unmodified.',
};
const rustLicenseIdentity = {
  manifest: 'assets/license-sources.json',
  manifestSize: 13949,
  manifestSha256: 'b8a8e29bf8b295f85426c4ffe2e1b25620b2a105865cc0e946db645ef4d8b48e',
  crateCount: 6,
  fileCount: 4,
  fullTexts: [
    { path: 'assets/licenses/jsonschema-0.58.6-MIT.txt', license: 'MIT', size: 1075,
      sha256: '117829c3ca21efb132d81a44b55363d395ab8eea18526873bc828da4c0e5f038',
      url: 'https://raw.githubusercontent.com/Stranger6667/jsonschema/55ac1664384793c5d3b20745d3f9c169dfe2c06f/LICENSE' },
    { path: 'assets/licenses/r-efi-5.3.0-MIT-AUTHORS.txt', license: 'MIT', size: 3693,
      sha256: 'ff92bed461f50338dd703a9ba9aee496a425957873df1d91192776e4bdf5dda7',
      url: 'https://raw.githubusercontent.com/r-efi/r-efi/97b55bed1c2c91dcbf787674849f05337ff80b33/AUTHORS' },
    { path: 'assets/licenses/selectors-0.38.0-MPL-2.0.txt', license: 'MPL-2.0', size: 16726,
      sha256: '3f3d9e0024b1921b067d6f7f88deb4a60cbe7a78e76c64e3f1d7fc3b779b9d04',
      url: 'https://raw.githubusercontent.com/mozilla/bedrock/a15178c3c7c976c67b3641af77cae0b66093a175/media/MPL/2.0/index.txt' },
    { path: 'assets/licenses/uuid-simd-0.8.0-MIT.txt', license: 'MIT', size: 1062,
      sha256: '71674605ec4c087fe9eb534e3e4f9e26eb2e4aabcd76a29fd156c6a844d44b3d',
      url: 'https://raw.githubusercontent.com/Nugine/simd/d74c030d9dc4f3cae02146d1f497ff62726ef09a/LICENSE' },
  ],
};
const referenceRoots = ['archify', 'viewer', 'scripts', 'test', 'package.json', 'package-lock.json', 'LICENSE', 'THIRD_PARTY_NOTICES.md'];
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const canonical = value => Buffer.from(JSON.stringify(value), 'utf8');
const comparePaths = (a, b) => Buffer.compare(Buffer.from(a[0]), Buffer.from(b[0]));

function readFile(file, description) {
  let stat;
  try { stat = fs.lstatSync(file); }
  catch (error) {
    if (error.code === 'ENOENT') throw new Error(`Missing ${description}: ${file}. Retrieve the pinned upstream bytes; do not replace the baseline.`);
    throw error;
  }
  if (!stat.isFile() || stat.isSymbolicLink()) throw new Error(`Expected regular ${description}: ${file}`);
  return fs.readFileSync(file);
}

function safeRelative(name) {
  if (!name || /[\\:\x00-\x1f\x7f]/.test(name) || name.startsWith('/')
      || name.split('/').some(part => !part || part === '.' || part === '..')) throw new Error(`Unsafe source member: ${name}`);
  return name;
}

function treeManifest(directory, roots = fs.readdirSync(directory)) {
  const stat = fs.lstatSync(directory);
  if (!stat.isDirectory() || stat.isSymbolicLink()) throw new Error(`Source tree root must be a regular directory: ${directory}`);
  const rows = [];
  const visit = relative => {
    const file = path.join(directory, ...safeRelative(relative).split('/')), stat = fs.lstatSync(file);
    if (stat.isDirectory() && !stat.isSymbolicLink()) {
      for (const child of fs.readdirSync(file)) visit(`${relative}/${child}`);
    } else if (stat.isFile() && !stat.isSymbolicLink()) {
      const bytes = fs.readFileSync(file);
      rows.push([relative, bytes.length, sha256(bytes)]);
    } else throw new Error(`Source tree contains a non-regular file: ${relative}`);
  };
  for (const relative of roots) visit(relative);
  return rows.sort(comparePaths);
}

function readManifest(upstream, name, expected, label) {
  const bytes = readFile(path.join(upstream, name), `${label.toLowerCase()} manifest`);
  const rows = JSON.parse(bytes);
  if (!Array.isArray(rows) || !canonical(rows).equals(bytes) || sha256(bytes) !== expected) {
    throw new Error(`${label} manifest is not the pinned canonical JSON`);
  }
  let previous;
  for (const row of rows) {
    if (!Array.isArray(row) || row.length !== 3 || typeof row[0] !== 'string'
        || !Number.isSafeInteger(row[1]) || row[1] < 0 || !/^[a-f0-9]{64}$/.test(row[2])) throw new Error(`Invalid ${label.toLowerCase()} manifest row`);
    safeRelative(row[0]);
    if (previous && comparePaths(previous, row) >= 0) throw new Error(`Unsorted or duplicate ${label.toLowerCase()} manifest member`);
    previous = row;
  }
  return rows;
}

// Reconstruct the file set from validated tar headers and exact member bytes;
// checking never extracts into, repairs, or refreshes the source checkout.
function archiveManifest(bytes) {
  if (bytes.readUInt32LE(4) !== 0) throw new Error('Frozen reference gzip mtime must be zero');
  const tar = gunzipSync(bytes, { maxOutputLength: 64 * 1024 * 1024 });
  const rows = [], members = new Map(); let ended = false;
  const field = (header, offset, size) => header.subarray(offset, offset + size).toString('utf8').replace(/\0.*$/s, '');
  const octal = text => {
    if (!/^[0-7]+$/.test(text.trim())) throw new Error('Invalid frozen tar number');
    return Number.parseInt(text.trim(), 8);
  };
  for (let offset = 0; offset + 512 <= tar.length;) {
    const header = tar.subarray(offset, offset + 512);
    if (header.every(byte => byte === 0)) {
      if (offset + 1024 > tar.length || tar.subarray(offset).some(byte => byte !== 0)) throw new Error('Invalid frozen tar footer');
      ended = true; break;
    }
    const checksum = header.reduce((sum, byte, index) => sum + (index >= 148 && index < 156 ? 32 : byte), 0);
    if (checksum !== octal(field(header, 148, 8))) throw new Error('Frozen tar header checksum differs');
    const prefix = field(header, 345, 155);
    const name = safeRelative(`${prefix ? prefix + '/' : ''}${field(header, 0, 100)}`);
    if (field(header, 156, 1) !== '0' || field(header, 157, 100)
        || field(header, 257, 6) !== 'ustar' || field(header, 263, 2) !== '00'
        || octal(field(header, 100, 8)) !== 0o644 || octal(field(header, 108, 8)) !== 0
        || octal(field(header, 116, 8)) !== 0 || octal(field(header, 136, 12)) !== 0
        || field(header, 265, 32) || field(header, 297, 32)) throw new Error(`Frozen tar metadata differs: ${name}`);
    if (members.has(name)) throw new Error(`Duplicate frozen tar member: ${name}`);
    const size = octal(field(header, 124, 12)), next = offset + 512 + Math.ceil(size / 512) * 512;
    if (next > tar.length) throw new Error(`Truncated frozen tar member: ${name}`);
    const content = tar.subarray(offset + 512, offset + 512 + size);
    rows.push([name, size, sha256(content)]); members.set(name, content); offset = next;
  }
  if (!ended || !rows.length) throw new Error('Missing frozen tar footer or members');
  return { rows: rows.sort(comparePaths), members };
}

function verifyNativeTextSources(upstream, identity) {
  if (!Object.entries(nativeTextIdentity).every(([key, value]) => identity?.[key] === value)) {
    throw new Error('Frozen native text identity differs from the pinned baseline');
  }
  const project = path.resolve(upstream, '..');
  const manifestPath = path.join(project, nativeTextIdentity.manifest);
  const bytes = readFile(manifestPath, 'native text manifest');
  if (bytes.length !== nativeTextIdentity.manifestSize || sha256(bytes) !== nativeTextIdentity.manifestSha256) {
    throw new Error('Native text manifest differs from the pinned source bytes');
  }
  const manifest = JSON.parse(bytes);
  if (manifest.format !== 1 || manifest.modification !== nativeTextIdentity.modification
      || manifest.license !== 'ICU77-LICENSE' || !Array.isArray(manifest.files)
      || manifest.files.length !== nativeTextIdentity.fileCount) throw new Error('Invalid pinned native text manifest');
  const seen = new Set();
  let originalLicense;
  for (const entry of manifest.files) {
    if (typeof entry.file !== 'string' || !/^[a-f0-9]{64}$/.test(entry.sha256) || typeof entry.url !== 'string') {
      throw new Error('Invalid native text source entry');
    }
    safeRelative(entry.file);
    if (seen.has(entry.file)) throw new Error(`Duplicate native text source: ${entry.file}`);
    seen.add(entry.file);
    const original = readFile(path.join(path.dirname(manifestPath), entry.file), 'native text source');
    if (sha256(original) !== entry.sha256) throw new Error(`Native text source SHA-256 differs: ${entry.file}`);
    if (entry.file === manifest.license) originalLicense = original;
  }
  const license = readFile(path.join(project, nativeTextIdentity.license), 'native text license');
  if (!originalLicense || license.length !== nativeTextIdentity.licenseSize
      || sha256(license) !== nativeTextIdentity.licenseSha256 || !license.equals(originalLicense)) {
    throw new Error('Native text license differs from the pinned ICU77 source bytes');
  }
  return { manifest: nativeTextIdentity.manifest, manifestSha256: nativeTextIdentity.manifestSha256,
    fileCount: seen.size, release: nativeTextIdentity.release, unicodeVersion: nativeTextIdentity.unicodeVersion,
    compiledDataGeneration: nativeTextIdentity.compiledDataGeneration,
    license: nativeTextIdentity.license, licenseSha256: nativeTextIdentity.licenseSha256,
    licenseSpdx: nativeTextIdentity.licenseSpdx, modification: manifest.modification };
}

export function verifyRustLicenseSources({ project = workspace, identity } = {}) {
  if (identity === undefined) identity = JSON.parse(readFile(path.join(project, 'upstream/SOURCE.json'), 'Rust license source identity')).rustLicenses;
  if (!identity || !canonical(identity).equals(canonical(rustLicenseIdentity))) {
    throw new Error('Frozen Rust license source identity differs from the pinned baseline');
  }
  for (const relative of ['assets', 'assets/licenses']) {
    const directory = path.join(project, relative);
    let stat;
    try { stat = fs.lstatSync(directory); }
    catch (error) {
      if (error.code === 'ENOENT') throw new Error(`Missing Rust license directory: ${directory}`);
      throw error;
    }
    if (!stat.isDirectory() || stat.isSymbolicLink()) throw new Error(`Expected regular Rust license directory: ${directory}`);
  }
  const bytes = readFile(path.join(project, identity.manifest), 'Rust license source manifest');
  if (bytes.length !== identity.manifestSize || sha256(bytes) !== identity.manifestSha256) {
    throw new Error('Rust license source manifest differs from the pinned source bytes');
  }
  const manifest = JSON.parse(bytes);
  if (manifest.schemaVersion !== 1 || !Array.isArray(manifest.rust) || manifest.rust.length !== identity.crateCount) {
    throw new Error('Invalid Rust license source manifest');
  }
  const lock = readFile(path.join(project, 'Cargo.lock'), 'Rust license Cargo.lock').toString('utf8');
  const packages = lock.split(/^\[\[package\]\]$/m).slice(1);
  const field = (block, name) => new RegExp(`^${name} = "([^"\\r\\n]+)"$`, 'm').exec(block)?.[1];
  const files = new Map(), crates = new Set();
  for (const entry of manifest.rust) {
    const key = `${entry.name}@${entry.version}`;
    const matching = packages.filter(block => field(block, 'name') === entry.name && field(block, 'version') === entry.version);
    if (crates.has(key) || matching.length !== 1 || field(matching[0], 'checksum') !== entry.checksum
        || field(matching[0], 'source') !== 'registry+https://github.com/rust-lang/crates.io-index') {
      throw new Error(`Rust license crate pin differs from Cargo.lock: ${key}`);
    }
    crates.add(key);
    if (!Array.isArray(entry.files) || !entry.files.length || !entry.selectedLicense) throw new Error(`Missing Rust license text: ${key}`);
    for (const source of entry.files) {
      safeRelative(source.path); safeRelative(source.sourcePath);
      if (!/^licenses\/[^/]+\.txt$/.test(source.path) || !/^https:\/\/github\.com\/[^/]+\/[^/]+$/.test(source.repository)
          || !/^[a-f0-9]{40}$/.test(source.revision)
          || source.url !== `https://raw.githubusercontent.com/${source.repository.slice('https://github.com/'.length)}/${source.revision}/${source.sourcePath}`) {
        throw new Error(`Invalid Rust license source URL or path: ${source.path}`);
      }
      const pinned = identity.fullTexts.find(file => file.path === `assets/${source.path}`);
      if (!pinned || pinned.size !== source.size || pinned.sha256 !== source.sha256 || pinned.url !== source.url
          || pinned.license !== entry.selectedLicense) throw new Error(`Rust license text identity differs: ${source.path}`);
      const content = readFile(path.join(project, 'assets', source.path), 'Rust license text');
      if (content.length !== pinned.size || sha256(content) !== pinned.sha256) {
        throw new Error(`Rust license text size/SHA-256 differs: ${source.path}`);
      }
      files.set(source.path, source.sha256);
    }
    const notice = entry.applicability?.sourceNotice;
    if (notice) {
      const header = Buffer.from(notice.header, 'utf8');
      if (notice.repository !== entry.repository || notice.revision !== entry.revision
          || header.length !== notice.headerSize || sha256(header) !== notice.headerSha256
          || !notice.header.includes(notice.licenseReference)) throw new Error(`Rust license source notice differs: ${key}`);
    }
  }
  if (files.size !== identity.fileCount) throw new Error('Rust license source file set differs from the pinned baseline');
  return { manifest: identity.manifest, manifestSha256: identity.manifestSha256, crateCount: crates.size, fileCount: files.size };
}

export function verifyFrozenSources({ upstream = upstreamRoot, localReference = path.join(root, 'local/archify-main') } = {}) {
  const source = JSON.parse(readFile(path.join(upstream, 'SOURCE.json'), 'source identity'));
  if (source.schemaVersion !== 1 || source.reference?.contentSha256 !== referenceSha256
      || source.reference.archiveSha256 !== archiveSha256 || source.reference.archiveSize !== 5372910
      || source.reference.archive !== 'archify-3.1.0-source.tar.gz' || source.reference.manifest !== 'archify-3.1.0-manifest.json'
      || source.reference.fileCount !== 498 || JSON.stringify(source.reference.roots) !== JSON.stringify(referenceRoots)
      || source.legacy?.revision !== legacyRevision || source.legacy.contentSha256 !== legacySha256
      || source.legacy.directory !== 'legacy-2.17' || source.legacy.manifest !== 'legacy-2.17-manifest.json'
      || source.legacy.fileCount !== 70) throw new Error('Frozen source identity differs from the pinned baseline');
  const reference = readManifest(upstream, source.reference.manifest, referenceSha256, 'Reference');
  const archive = readFile(path.join(upstream, source.reference.archive), 'frozen reference archive');
  if (archive.length !== source.reference.archiveSize || sha256(archive) !== archiveSha256) throw new Error('Frozen reference archive size/SHA-256 differs');
  const recovered = archiveManifest(archive);
  if (!canonical(recovered.rows).equals(canonical(reference))) throw new Error('Restored reference differs from the frozen source manifest');
  const legacy = readManifest(upstream, source.legacy.manifest, legacySha256, 'Legacy');
  if (!canonical(treeManifest(path.join(upstream, source.legacy.directory))).equals(canonical(legacy))) throw new Error('Frozen legacy tree differs from its source manifest');
  for (const [name, member] of [['LICENSE-archify-MIT.txt', 'LICENSE'], ['JetBrainsMono-OFL.txt', 'archify/assets/JetBrainsMono-OFL.txt']]) {
    if (!readFile(path.join(upstream, name), 'frozen license').equals(recovered.members.get(member))) throw new Error(`Frozen license differs: ${name}`);
  }
  const nativeText = verifyNativeTextSources(upstream, source.nativeText);
  const rustLicenses = verifyRustLicenseSources({ project: path.resolve(upstream, '..'), identity: source.rustLicenses });
  let localStatus = 'absent; archive is authoritative';
  if (localReference && fs.existsSync(localReference)) {
    try {
      if (!canonical(treeManifest(localReference, referenceRoots)).equals(canonical(reference))) throw new Error('Content differs');
    } catch (error) { throw new Error(`Local reference differs from frozen source: ${error.message}`); }
    localStatus = 'matches frozen archive';
  }
  return {
    reference: { contentSha256: referenceSha256, archiveSha256, fileCount: recovered.rows.length, localStatus },
    legacy: { revision: legacyRevision, contentSha256: legacySha256, fileCount: legacy.length },
    nativeText,
    rustLicenses,
  };
}

export function loadVersions() {
  const versions = JSON.parse(readFile(path.join(workspace, 'build-tools.lock.json'), 'diagram build-tool lock'));
  if (versions.schemaVersion !== 1 || !['rust', 'node', 'mrBoxington'].every(key => /^\d+\.\d+\.\d+$/.test(versions[key]))) throw new Error('Invalid diagram build-tool lock');
  const miseConfig = fs.readFileSync(path.join(root, 'mise.toml'), 'utf8');
  const rustConfig = fs.readFileSync(path.join(root, 'rust-toolchain.toml'), 'utf8');
  for (const [key, version] of [['node', versions.node], ['mr-boxington', versions.mrBoxington]]) {
    if (!miseConfig.includes(`${key} = "${version}"`)) throw new Error(`Diagram ${key} pin differs from mise.toml`);
  }
  if (!miseConfig.includes(`version = "${versions.rust}"`) || !rustConfig.includes(`channel = "${versions.rust}"`)) throw new Error('Diagram Rust pin differs from repository toolchain');
  return versions;
}

// Read only: Diagram consumes the existing fixed browser source, independently
// of the UI's Bun/WASM preparation and acceptance evidence.
export function browserPin(repoRoot = root) {
  const pin = JSON.parse(readFile(path.join(repoRoot, 'tools/phoenix-ui/build-tool-assets.json'), 'browser source pins')).linuxArmBrowser;
  if (pin?.package !== 'playwright' || !/^\d+\.\d+\.\d+$/.test(pin.packageVersion) || !/^\d+$/.test(pin.revision)
      || !/^\d+\.\d+\.\d+\.\d+$/.test(pin.expectedVersion) || !/^Chromium \d+\.\d+\.\d+\.\d+$/.test(pin.actualVersion)
      || !/^[a-f0-9]{64}$/.test(pin.sha256)) throw new Error('Invalid fixed Diagram Chromium pin');
  const url = new URL(pin.source);
  if (url.protocol !== 'https:' || url.username || url.password || url.search || url.hash
      || !url.pathname.endsWith(`/builds/chromium/${pin.revision}/chromium-linux-arm64.zip`)) throw new Error('Chromium source must retain its fixed HTTPS revision URL');
  return pin;
}

// Search adapted from Phoenix UI browser/discovery.rs (Apache-2.0), Copyright
// 2025 Paul Bakaus, via XGENT-ai/skills 4a65a1525f9ed03c775a0e18935901e1edbb773f.
// Diagram changes: independent override and read-only preparation telemetry.
export function discoverBrowser({ platform = process.platform, env = process.env, candidates } = {}) {
  const executable = file => { try { const stat = fs.statSync(file); return stat.isFile() && (stat.mode & 0o111) !== 0; } catch { return false; } };
  const configured = env.PHOENIX_DIAGRAM_BROWSER;
  if (configured !== undefined) {
    if (!configured || !executable(configured)) return { status: 'failed', verification: 'not-run', diagnostic: { code: 'browser/configured-path', message: 'PHOENIX_DIAGRAM_BROWSER is not an executable ordinary file' } };
    return { status: 'available', path: fs.realpathSync(configured), source: 'configured', verification: 'not-run' };
  }
  if (candidates === undefined) {
    candidates = [];
    if (platform.startsWith('darwin')) {
      for (const bundle of ['Google Chrome.app/Contents/MacOS/Google Chrome', 'Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary', 'Chromium.app/Contents/MacOS/Chromium']) {
        for (const directory of ['/Applications', ...(env.HOME ? [path.join(env.HOME, 'Applications')] : [])]) candidates.push(path.join(directory, bundle));
      }
    } else {
      // Prefer the actual Chrome ELF over its distribution shell wrapper.
      candidates.push('/opt/google/chrome/chrome');
      for (const name of ['google-chrome', 'google-chrome-stable', 'chromium', 'chromium-browser']) {
        for (const directory of (env.PATH || '').split(path.delimiter).filter(Boolean)) candidates.push(path.join(directory, name));
      }
    }
  }
  const found = candidates.find(executable);
  return found ? { status: 'available', path: fs.realpathSync(found), source: 'discovered', verification: 'not-run' }
    : { status: 'unavailable', verification: 'not-run', diagnostic: { code: 'browser/unavailable', message: 'No installed Chrome or Chromium executable was found' } };
}

export function miseEnvironment(overrides = {}) {
  const env = { ...process.env, ...overrides, MISE_AUTO_INSTALL: 'false', MISE_EXEC_AUTO_INSTALL: 'false' };
  for (const key of Object.keys(env)) if (key.startsWith('__MISE') && key !== '__MISE_DIFF') delete env[key];
  const cargoBins = [path.join(os.homedir(), '.cargo/bin'), process.env.CARGO_HOME && path.join(process.env.CARGO_HOME, 'bin')].filter(Boolean);
  env.PATH = (env.PATH || '').split(path.delimiter).filter(directory => !cargoBins.includes(directory)).join(path.delimiter);
  return env;
}

export function mise(args, options = {}) {
  const result = spawnSync('mise', ['exec', '--no-deps', '--', ...args], {
    cwd: workspace, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024, ...options,
    env: miseEnvironment({ CARGO_NET_OFFLINE: 'true', ...options.env }),
  });
  if (result.error || result.status !== 0) throw new Error(result.error?.message || result.stderr || `${args.join(' ')} failed (${result.status})`);
  return result.stdout;
}

export function preflight({ checkDependencies = true } = {}) {
  const versions = loadVersions();
  const selected = JSON.parse(mise(['mise', 'ls', '--current', '--json']));
  for (const [tool, version] of [['rust', versions.rust], ['node', versions.node], ['mr-boxington', versions.mrBoxington]]) {
    if (!selected[tool]?.some(entry => entry.version === version && entry.installed && entry.active)) throw new Error(`Missing installed mise tool ${tool}@${version}; explicitly run node scripts/prepare-phoenix-diagram.mjs --prepare`);
  }
  for (const [command, version] of [['rustc', versions.rust], ['cargo', versions.rust], ['node', versions.node], ['mbx', versions.mrBoxington]]) {
    const output = mise([command, '--version']);
    if (!new RegExp(`(?:^|\\s|v)${version.replaceAll('.', '\\.')}\\b`).test(output)) throw new Error(`${command}: expected ${version}, got ${output.trim()}`);
  }
  const host = /^host: (.+)$/m.exec(mise(['rustc', '-vV']))?.[1];
  if (!host) throw new Error('Rust host target is unavailable');
  if (process.env.PHOENIX_DIAGRAM_BUILD_TARGET && process.env.PHOENIX_DIAGRAM_BUILD_TARGET !== host) throw new Error('Diagram preparation/build must use the runner native Rust target');
  const components = mise(['rustup', 'component', 'list', '--installed']);
  if (!['rustfmt', 'clippy'].every(component => components.includes(`${component}-${host}`))) throw new Error('Missing rustfmt/clippy; explicitly prepare diagram build tools');
  const doctor = mise(['mbx', 'doctor']);
  if (!doctor.includes('managed targets enabled') || !doctor.includes('0 failures')) throw new Error('mbx managed targets must be enabled and doctor must have no failures');
  const tools = JSON.parse(mise(['node', '-e', `
    const fs = require('node:fs'), path = require('node:path');
    const find = name => process.env.PATH.split(path.delimiter).map(directory => path.join(directory, name + (process.platform === 'win32' ? '.exe' : ''))).find(file => fs.existsSync(file));
    const cargo = find('cargo');
    console.log(JSON.stringify({ cargo, realCargo: fs.realpathSync(cargo), mise: find('mise'), path: process.env.PATH }));
  `]));
  if (!/[\\/]command-wrappers[\\/]/.test(tools.cargo)) throw new Error('Cargo must use the mise mr_boxington command wrapper');
  if (process.platform === 'win32') {
    const shim = path.join(path.dirname(fs.realpathSync(tools.mise)), 'mise-shim.exe');
    if (!fs.readFileSync(shim).equals(fs.readFileSync(tools.cargo))) throw new Error('Cargo wrapper differs from the mise native shim');
  } else if (!/^mise(?:\.exe)?$/.test(path.basename(tools.realCargo))) throw new Error('Cargo wrapper does not resolve to mise');
  if (checkDependencies) {
    readFile(path.join(workspace, 'Cargo.lock'), 'diagram Cargo.lock');
    mise(['mbx', 'metadata', '--locked', '--offline', '--format-version', '1']);
  }
  return { versions, host, cargo: tools.cargo, path: tools.path, dependencies: checkDependencies ? 'locked and cached offline' : 'not checked' };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (process.argv.length !== 3 || process.argv[2] !== '--check') {
    console.error('Usage: node scripts/check-phoenix-diagram-source.mjs --check'); process.exitCode = 2;
  } else {
    try {
      const sources = verifyFrozenSources(), prepared = preflight();
      const tools = { versions: prepared.versions, host: prepared.host, cargo: prepared.cargo, dependencies: prepared.dependencies };
      console.log(JSON.stringify({ status: 'verified', sources, tools }, null, 2));
    } catch (error) { console.error(error.message); process.exitCode = 1; }
  }
}
