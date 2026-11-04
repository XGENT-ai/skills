#!/usr/bin/env node
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

export const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
export const workspace = path.join(root, 'tools/phoenix-diagram');
// These four identities are the only pins; checking never refreshes them from the
// files they describe.
export const brandSourceIdentity = {
  manifest: 'assets/brand-license-sources.json',
  manifestSize: 4654,
  manifestSha256: 'b093424c16d2e0483e3156cb3ba762d5b8229b20dab3efaea3666418ac6a86ea',
  catalog: { path: 'assets/brand-marks.json', size: 167862,
    sha256: '905be7402246ae3810ff349eabab78044d2dfe01f53402562819ef5177b555ed', count: 107 },
  licenseCount: 8,
  noticeCount: 2,
};
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
const nativeTimeIdentity = {
  repository: 'https://github.com/unicode-org/icu',
  release: 'release-77-1',
  tzVersion: '2025a',
  manifest: 'tests/fixtures/format/system-v-sources.json',
  manifestSize: 1933,
  manifestSha256: '5cca4def77aea4452b0877c10caee53d08ab0414e7a42706edffb9a6b6da0067',
  fileCount: 1,
  zoneCount: 6,
  license: 'assets/icu77-LICENSE.txt',
  licenseSize: 26487,
  licenseSha256: '451167c55c0fa447cc2d5632714f5e3c567fe4f1e1badefab2c1333852198aca',
  licenseSpdx: 'Unicode-3.0',
  modification: 'Rust adaptation of the six SystemV daylight-saving zones using normalized historical transitions and their final annual rule; the original ICU source fixture is unmodified.',
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
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const canonical = value => Buffer.from(JSON.stringify(value), 'utf8');

function readFile(file, description) {
  try { return fs.readFileSync(file); }
  catch (error) {
    if (error.code === 'ENOENT') throw new Error(`Missing ${description}: ${file}. Retrieve the pinned bytes; do not replace the baseline.`);
    throw error;
  }
}

function safeRelative(name) {
  if (!name || /[\\:\x00-\x1f\x7f]/.test(name) || name.startsWith('/')
      || name.split('/').some(part => !part || part === '.' || part === '..')) throw new Error(`Unsafe source member: ${name}`);
  return name;
}

function verifyNativeTextSources(project) {
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

function verifyNativeTimeSources(project) {
  const manifestPath = path.join(project, nativeTimeIdentity.manifest);
  const bytes = readFile(manifestPath, 'native time manifest');
  if (bytes.length !== nativeTimeIdentity.manifestSize || sha256(bytes) !== nativeTimeIdentity.manifestSha256) {
    throw new Error('Native time manifest differs from the pinned source bytes');
  }
  const manifest = JSON.parse(bytes);
  if (manifest.format !== 1 || manifest.repository !== nativeTimeIdentity.repository
      || manifest.release !== nativeTimeIdentity.release || manifest.tzVersion !== nativeTimeIdentity.tzVersion
      || manifest.modification !== nativeTimeIdentity.modification || manifest.zones?.length !== nativeTimeIdentity.zoneCount
      || manifest.license?.path !== nativeTimeIdentity.license || manifest.license.spdx !== nativeTimeIdentity.licenseSpdx
      || manifest.license.size !== nativeTimeIdentity.licenseSize || manifest.license.sha256 !== nativeTimeIdentity.licenseSha256) {
    throw new Error('Invalid pinned native time manifest');
  }
  const original = readFile(path.join(path.dirname(manifestPath), safeRelative(manifest.source.file)), 'native time source');
  if (original.length !== manifest.source.size || sha256(original) !== manifest.source.sha256) {
    throw new Error('Native time source size/SHA-256 differs from the pinned ICU77 bytes');
  }
  const license = readFile(path.join(project, nativeTimeIdentity.license), 'native time license');
  if (license.length !== nativeTimeIdentity.licenseSize || sha256(license) !== nativeTimeIdentity.licenseSha256) {
    throw new Error('Native time license differs from the pinned ICU77 source bytes');
  }
  return { manifest: nativeTimeIdentity.manifest, manifestSha256: nativeTimeIdentity.manifestSha256,
    sourceSha256: manifest.source.sha256, fileCount: nativeTimeIdentity.fileCount, zoneCount: manifest.zones.length,
    release: nativeTimeIdentity.release, tzVersion: nativeTimeIdentity.tzVersion,
    license: nativeTimeIdentity.license, licenseSha256: nativeTimeIdentity.licenseSha256,
    licenseSpdx: nativeTimeIdentity.licenseSpdx, modification: manifest.modification };
}

export function verifyRustLicenseSources({ project = workspace } = {}) {
  const identity = rustLicenseIdentity;
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

export function readBrandLicenseSources({ project = workspace } = {}) {
  const pinned = (name, size, digest, label) => {
    const bytes = readFile(path.join(project, name), label);
    if (bytes.length !== size || sha256(bytes) !== digest) throw new Error(`Pinned brand ${label} bytes differ: ${name}`);
    return bytes;
  };
  const manifestBytes = pinned(brandSourceIdentity.manifest, brandSourceIdentity.manifestSize, brandSourceIdentity.manifestSha256, 'license manifest');
  const manifest = JSON.parse(manifestBytes), catalog = brandSourceIdentity.catalog;
  const catalogBytes = pinned(catalog.path, catalog.size, catalog.sha256, 'catalog');
  const files = new Map([[brandSourceIdentity.manifest, manifestBytes], [catalog.path, catalogBytes]]);
  for (const resource of [...manifest.licenses, ...manifest.notices]) {
    const name = `assets/${safeRelative(resource.path)}`;
    files.set(name, pinned(name, resource.size, resource.sha256, 'license resource'));
  }
  return { manifest, manifestBytes, catalog: JSON.parse(catalogBytes), files };
}

export function verifyBrandLicenseSources({ project = workspace } = {}) {
  const { manifest, catalog } = readBrandLicenseSources({ project });
  if (catalog.marks.length !== brandSourceIdentity.catalog.count || !canonical(catalog.source).equals(canonical(manifest.reference))) {
    throw new Error('Native brand catalog differs from its recorded source');
  }
  return structuredClone(brandSourceIdentity);
}

export function verifySources({ project = workspace } = {}) {
  return {
    nativeText: verifyNativeTextSources(project),
    nativeTime: verifyNativeTimeSources(project),
    rustLicenses: verifyRustLicenseSources({ project }),
    nativeBrands: verifyBrandLicenseSources({ project }),
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
      const sources = verifySources(), prepared = preflight();
      const tools = { versions: prepared.versions, host: prepared.host, cargo: prepared.cargo, dependencies: prepared.dependencies };
      console.log(JSON.stringify({ status: 'verified', sources, tools }, null, 2));
    } catch (error) { console.error(error.message); process.exitCode = 1; }
  }
}
