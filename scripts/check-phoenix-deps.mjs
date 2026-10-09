#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { createHash } from 'node:crypto';
import { gunzipSync } from 'node:zlib';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { workspace, miseEnvironment, versions } from './phoenix-build-tools.mjs';

const root = path.resolve(workspace, '../..');
const prepared = path.join(root, 'local/phoenix-ui/dependency-audit');
const dbRoot = path.join(prepared, 'advisory-dbs');
const dbPath = path.join(dbRoot, 'advisory-db-3157b0e258782691');
export const databasePin = {
  url: 'https://github.com/RustSec/advisory-db',
  revision: 'b8a1a33e246a0a9a3b5f377248c41a503defec74',
  tree: '7310ad041f7925accfc258bec7fb4f76fb7ff4a9',
};
export const modernScreenshot = {
  name: 'modern-screenshot', version: '4.7.0', license: 'MIT',
  importedPath: 'skill/scripts/modern-screenshot.umd.js',
  sourcePath: 'package/dist/index.js',
  metadataUrl: 'https://registry.npmjs.org/modern-screenshot/4.7.0',
  tarballUrl: 'https://registry.npmjs.org/modern-screenshot/-/modern-screenshot-4.7.0.tgz',
  tarballSha256: 'f0a936790389088dccfaec198f904339878a0cf8e875745bb262ca761c804dc3',
  integrity: 'sha512-9YxN+ddPSMMlhylOv25VHzXrl9u67QRxoh7+SEewGtgUw7t6hHTrjptSDJUSne9oG4Xk/h2cwG15nIt4Hc9ujg==',
  sha256: 'bb36665889124a0b6e15f16045265737449c3bdcf2712cdb08af3cfa01563e2b',
  licenseSha256: '1daae2db27daba18a7e21bb56e12e19b8de6c1197658921f09bbe550f7106579',
};
const npmEndpoint = 'https://registry.npmjs.org/-/npm/v1/security/advisories/bulk';
const maxAgeDays = 30;
const nodeLicenses = new Set(['MIT', 'Apache-2.0', 'ISC', 'BSD-2-Clause', 'BSD-3-Clause']);
const extraNodeLicenses = {
  'puppeteer@25.1.0': {
    gitHead: 'ede66693bc0a6d9a8029f66288d13e26966173ce',
    url: 'https://raw.githubusercontent.com/puppeteer/puppeteer/ede66693bc0a6d9a8029f66288d13e26966173ce/LICENSE',
    sha256: 'a27ca07269b3518550b2e83aed13eadd7d14d924b5864e14889b40cf227530ca',
  },
  'puppeteer-core@25.1.0': { alias: 'puppeteer@25.1.0' },
  '@puppeteer/browsers@3.0.4': { alias: 'puppeteer@25.1.0' },
  // Both fixed source trees declare MIT in README/package.json, without a LICENSE file.
  'is-reference@3.0.3': {
    gitHead: '8bb053129bfabe2f6a7d7ed050159d67ebe82829',
    url: 'https://github.com/Rich-Harris/is-reference/blob/8bb053129bfabe2f6a7d7ed050159d67ebe82829/README.md#license',
    declarationOnly: true,
  },
  'locate-character@3.0.0': {
    gitHead: '4f08a59ec248121f7002abd02ee7b94e8eda06bc',
    url: 'https://github.com/Rich-Harris/locate-character/blob/4f08a59ec248121f7002abd02ee7b94e8eda06bc/README.md#license',
    declarationOnly: true,
  },
};

export const sha256 = value => createHash('sha256').update(value).digest('hex');
const readJson = filename => JSON.parse(fs.readFileSync(filename, 'utf8'));
const writeJson = (filename, value) => fs.writeFileSync(filename, `${JSON.stringify(value, null, 2)}\n`);
const fileHash = filename => sha256(fs.readFileSync(filename));
const requireEqual = (actual, expected, message) => { if (actual !== expected) throw new Error(message); };

// bun.lock is JSON with trailing commas; never evaluate it as JavaScript.
export function parseBunLock(text) {
  const clean = text.replace(/"(?:\\.|[^"\\])*"|,(?=\s*[}\]])/g, token => token === ',' ? '' : token);
  const lock = JSON.parse(clean);
  if (lock.lockfileVersion !== 1 || !lock.packages || typeof lock.packages !== 'object') throw new Error('Unsupported bun.lock schema');
  return Object.entries(lock.packages).map(([key, value]) => {
    if (!Array.isArray(value) || value.length !== 4 || value[1] !== '' || !/^sha512-/.test(value[3])) {
      throw new Error(`Unsupported/non-npm bun.lock dependency: ${key}`);
    }
    const match = /^(.+)@(\d+\.\d+\.\d+(?:[-+][\w.-]+)?)$/.exec(value[0]);
    if (!match) throw new Error(`Unpinned bun.lock dependency: ${key}`);
    return { key, name: match[1], version: match[2], integrity: value[3] };
  }).sort((a, b) => a.key.localeCompare(b.key, 'en'));
}

export function parseCargoLock(text) {
  const packages = text.split(/^\[\[package\]\]\s*$/m).slice(1).map(block => {
    // Only the pinned scalar fields are needed, not a general TOML parser.
    const get = key => {
      const match = new RegExp(`^${key} = ("[^"\\n]*")$`, 'm').exec(block);
      return match ? JSON.parse(match[1]) : undefined;
    };
    return { name: get('name'), version: get('version'), source: get('source'), checksum: get('checksum') };
  });
  if (!packages.length || packages.some(p => !p.name || !p.version || (p.source && !p.checksum))) throw new Error('Incomplete Cargo.lock inventory');
  return packages;
}

export function licenseAllowed(expression, allowed = nodeLicenses) {
  if (typeof expression !== 'string' || !expression.trim()) return false;
  const tokens = expression.match(/\(|\)|[A-Za-z0-9.+-]+/g) ?? [];
  if (tokens.join('') !== expression.replace(/\s/g, '')) return false;
  let index = 0;
  function atom() {
    if (tokens[index] === '(') {
      index++; const result = or();
      if (tokens[index++] !== ')') throw new Error('Unbalanced SPDX expression');
      return result;
    }
    let license = tokens[index++];
    if (!license || ['AND', 'OR', 'WITH', ')'].includes(license)) throw new Error('Invalid SPDX expression');
    if (tokens[index] === 'WITH') { index++; license += ` WITH ${tokens[index++]}`; }
    return allowed.has(license);
  }
  function and() { let result = atom(); while (tokens[index] === 'AND') { index++; const next = atom(); result = result && next; } return result; }
  function or() { let result = and(); while (tokens[index] === 'OR') { index++; const next = and(); result = result || next; } return result; }
  try { const result = or(); return index === tokens.length && result; } catch { return false; }
}

export function auditRequest(packages) {
  const request = {};
  for (const { name, version } of [...packages, modernScreenshot].sort((a, b) => a.name.localeCompare(b.name, 'en'))) {
    request[name] ??= []; if (!request[name].includes(version)) request[name].push(version);
  }
  return request;
}

export function validateNpmSnapshot(snapshot, packages, bunLockHash, now = new Date()) {
  requireEqual(snapshot.bunLockSha256, bunLockHash, 'npm advisory snapshot belongs to another bun.lock; explicitly prepare it again');
  requireEqual(snapshot.bunVersion, versions.bun, 'npm advisory snapshot used another Bun version');
  requireEqual(snapshot.endpoint, npmEndpoint, 'Unexpected npm advisory endpoint');
  requireEqual(JSON.stringify(snapshot.request), JSON.stringify(auditRequest(packages)), 'npm advisory request omitted or changed locked dependencies');
  const age = (now - new Date(snapshot.queriedAt)) / 86400000;
  if (!Number.isFinite(age) || age < 0 || age > maxAgeDays) throw new Error('npm advisory snapshot is expired or has an invalid acquisition time; explicitly prepare it again');
  for (const [label, response, hash] of [
    ['npm bulk', snapshot.response, snapshot.responseSha256], ['bun audit', snapshot.bunResponse, snapshot.bunResponseSha256],
  ]) {
    if (!response || Array.isArray(response) || typeof response !== 'object' || Object.values(response).some(v => !Array.isArray(v))) throw new Error(`${label}: invalid advisory response`);
    requireEqual(sha256(JSON.stringify(response)), hash, `${label}: advisory response hash differs`);
    if (Object.values(response).some(v => v.length)) throw new Error(`${label}: unresolved security advisories; no automatic exemptions`);
  }
  if (snapshot.bunExitCode !== 0 || snapshot.skippedRegistries) throw new Error('Bun audit failed or skipped registries');
  return { packages: packages.length, additionalBundledPackages: 1, findings: 0, queriedAt: snapshot.queriedAt };
}

export function cargoDiagnostics(stdout, stderr) {
  const entries = `${stdout}\n${stderr}`.split(/\r?\n/).filter(Boolean).map(line => JSON.parse(line));
  const summary = entries.find(e => e.type === 'summary')?.fields;
  if (!summary?.advisories || !summary?.licenses) throw new Error('cargo-deny did not complete both advisories and licenses');
  const findings = entries.filter(e => e.type === 'diagnostic' && ['error', 'warning'].includes(e.fields?.severity)).map(({ fields: f }) => ({
    severity: f.severity, code: f.code, message: f.message,
    packages: (f.graphs ?? []).map(g => g.Krate),
    ...(f.advisory ? { id: f.advisory.id, aliases: f.advisory.aliases, url: f.advisory.url } : {}),
    ...(f.notes ? { notes: f.notes.filter(n => n.startsWith('Solution:')) } : {}),
  }));
  return { summary, findings };
}

export function tarFiles(archive) {
  const data = gunzipSync(archive);
  const files = new Map();
  let nextPath;
  for (let offset = 0; offset + 512 <= data.length;) {
    const header = data.subarray(offset, offset + 512);
    if (header.every(byte => byte === 0)) break;
    const string = (start, end) => header.subarray(start, end).toString().replace(/\0.*$/, '');
    const prefix = string(345, 500);
    const name = nextPath ?? `${prefix ? `${prefix}/` : ''}${string(0, 100)}`;
    const size = parseInt(header.subarray(124, 136).toString().replace(/\0.*$/, '').trim(), 8);
    if (!Number.isSafeInteger(size) || size < 0 || offset + 512 + size > data.length) throw new Error('Invalid npm tarball');
    const body = data.subarray(offset + 512, offset + 512 + size);
    const type = header[156];
    if ([0, 48].includes(type)) {
      if (files.has(name)) throw new Error(`Duplicate tarball member ${name}`);
      files.set(name, body); nextPath = undefined;
    } else if (type === 76) nextPath = body.toString().replace(/\0.*$/, '');
    else if (type === 120) {
      // POSIX extended headers, including long paths used by some crates.
      for (let at = 0; at < body.length;) {
        const space = body.indexOf(32, at); const length = Number(body.subarray(at, space).toString());
        if (!Number.isSafeInteger(length) || length <= space - at || at + length > body.length) throw new Error('Invalid tar extended header');
        const record = body.subarray(space + 1, at + length - 1).toString();
        if (record.startsWith('path=')) nextPath = record.slice(5);
        at += length;
      }
    } else if (type !== 53) throw new Error(`Unsupported tarball member type ${type}`);
    if (name.startsWith('/') || name.split('/').includes('..')) throw new Error('Unsafe tarball path');
    offset += 512 + Math.ceil(size / 512) * 512;
  }
  return files;
}

export function tarMember(archive, wanted) {
  const member = tarFiles(archive).get(wanted);
  if (!member) throw new Error(`Missing npm tarball member ${wanted}`);
  return member;
}

export function verifyCrate(directory, locked) {
  const registry = path.basename(path.dirname(directory));
  const filename = `${locked.name}-${locked.version}.crate`;
  const archiveFile = path.resolve(directory, '../../../cache', registry, filename);
  if (!fs.existsSync(archiveFile)) throw new Error(`Missing cached ${filename}; explicitly prepare with cargo fetch --locked before checking`);
  const archive = fs.readFileSync(archiveFile);
  requireEqual(sha256(archive), locked.checksum, `${filename}: archive differs from Cargo.lock checksum`);
  const prefix = `${locked.name}-${locked.version}/`;
  const original = new Set();
  for (const [name, bytes] of tarFiles(archive)) {
    if (!name.startsWith(prefix)) throw new Error(`${filename}: unexpected archive root`);
    const relative = name.slice(prefix.length);
    original.add(relative);
    requireEqual(fileHash(path.join(directory, relative)), sha256(bytes), `${filename}: modified registry source ${relative}`);
  }
  // Verify the archive itself, including caches without .cargo-checksum.json.
  // Cargo creates these two bookkeeping files outside the published archive.
  for (const full of walk(directory)) {
    const relative = path.relative(directory, full).split(path.sep).join('/');
    if (!original.has(relative) && !['.cargo-ok', '.cargo-checksum.json'].includes(relative)) throw new Error(`${filename}: extra registry source ${relative}`);
  }
}

function run(command, args, env = {}, cwd = workspace) {
  const result = spawnSync(command, args, { cwd, env: miseEnvironment(env), encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  if (result.error) throw result.error;
  return result;
}
const tool = (args, env = {}) => run('mise', ['exec', '--no-deps', '--', ...args], env);
function successful(result, name) {
  if (result.status !== 0) throw new Error(`${name} exited ${result.status}: ${result.stderr.trim()}`);
  return result.stdout.trim();
}
function deny(args, offline) {
  const name = process.platform === 'win32' ? 'cargo-deny.exe' : 'cargo-deny';
  const local = path.join(prepared, 'bin', name);
  // Resolve an already installed executable before mise removes Cargo's global
  // bin directory from PATH to select the Rust wrapper. Never install it here.
  const existing = (process.env.PATH ?? '').split(path.delimiter).map(p => path.join(p, name)).find(p => fs.existsSync(p) && fs.statSync(p).isFile());
  const binary = process.env.PHOENIX_CARGO_DENY ?? (fs.existsSync(local) ? local : existing ?? name);
  return tool([binary, '--manifest-path', path.join(workspace, 'Cargo.toml'), '--config', path.join(workspace, 'deny.toml'), ...args], {
    PHOENIX_ADVISORY_DB_ROOT: dbRoot,
    ...(offline ? { CARGO_NET_OFFLINE: 'true' } : {}),
  });
}
function preflight() {
  requireEqual(versions.cargoDeny, '0.20.2', 'The cargo-deny implementation/configuration requires the 0.20.2 tool pin');
  for (const [name, expected] of [['rustc', versions.rust], ['cargo', versions.rust], ['node', versions.node], ['bun', versions.bun]]) {
    const actual = successful(tool([name, '--version']), `${name} version`);
    if (!new RegExp(`(?:^|\\s|v)${expected.replaceAll('.', '\\.')}(?:\\s|$)`).test(actual)) throw new Error(`Expected ${name} ${expected}, got ${actual}; prepare pinned tools explicitly`);
  }
  requireEqual(successful(deny(['--version'], true), 'cargo-deny version'), 'cargo-deny 0.20.2', 'Expected cargo-deny 0.20.2; prepare the exact tool explicitly');
}
function databaseState() {
  const git = args => successful(run('git', ['-C', dbPath, ...args]), 'RustSec database git');
  const state = { url: databasePin.url, revision: git(['rev-parse', 'HEAD']), tree: git(['rev-parse', 'HEAD^{tree}']),
    commitAt: git(['show', '-s', '--format=%cI', 'HEAD']), treeListingSha256: sha256(git(['ls-tree', '-r', 'HEAD'])) };
  validateDatabase(state, git(['status', '--porcelain', '--untracked-files=all']));
  return state;
}

export function validateDatabase(state, dirty, now = new Date()) {
  requireEqual(state.revision, databasePin.revision, 'Wrong RustSec database revision; explicitly prepare the pinned database');
  requireEqual(state.tree, databasePin.tree, 'Wrong RustSec database tree');
  requireEqual(dirty, '', 'RustSec database has modified/untracked files');
  const age = (now - new Date(state.commitAt)) / 86400000;
  if (!Number.isFinite(age) || age < 0 || age > maxAgeDays) throw new Error('Pinned RustSec database content is older than 30 days; review/update the pin explicitly');
}
async function download(url) {
  const response = await fetch(url, { signal: AbortSignal.timeout(60000) });
  if (!response.ok) throw new Error(`Explicit preparation request failed: ${url} (${response.status})`);
  return Buffer.from(await response.arrayBuffer());
}

async function prepare() {
  preflight();
  fs.mkdirSync(prepared, { recursive: true });
  successful(deny(['fetch', 'db'], false), 'explicit RustSec fetch');
  successful(run('git', ['-C', dbPath, 'checkout', '--detach', databasePin.revision]), 'pin RustSec database');
  const database = databaseState();
  const databasePreparedAt = new Date().toISOString();
  const archive = await download(modernScreenshot.tarballUrl);
  requireEqual(sha256(archive), modernScreenshot.tarballSha256, 'modern-screenshot npm archive SHA-256 differs');
  requireEqual(`sha512-${createHash('sha512').update(archive).digest('base64')}`, modernScreenshot.integrity, 'modern-screenshot npm SRI differs');
  requireEqual(sha256(tarMember(archive, modernScreenshot.sourcePath)), modernScreenshot.sha256, 'modern-screenshot bundled file differs from pinned npm package');
  const license = tarMember(archive, 'package/LICENSE');
  requireEqual(sha256(license), modernScreenshot.licenseSha256, 'modern-screenshot license differs');
  fs.writeFileSync(path.join(prepared, `${modernScreenshot.licenseSha256}.txt`), license);
  const extraProvenance = [];
  for (const [spec, source] of Object.entries(extraNodeLicenses)) {
    const real = source.alias ? extraNodeLicenses[source.alias] : source;
    const at = spec.lastIndexOf('@');
    const metadataUrl = `https://registry.npmjs.org/${encodeURIComponent(spec.slice(0, at))}/${spec.slice(at + 1)}`;
    const metadata = JSON.parse(await download(metadataUrl));
    requireEqual(metadata.gitHead, real.gitHead, `${spec}: npm source commit differs`);
    if (!real.declarationOnly && !fs.existsSync(path.join(prepared, `${real.sha256}.txt`))) {
      const text = await download(real.url); requireEqual(sha256(text), real.sha256, `${spec}: source license hash differs`);
      fs.writeFileSync(path.join(prepared, `${real.sha256}.txt`), text);
    }
    extraProvenance.push({ spec, metadataUrl, integrity: metadata.dist.integrity, gitHead: metadata.gitHead, license: metadata.license, ...real });
  }
  writeJson(path.join(prepared, 'extra-node-licenses.json'), extraProvenance);
  const bunLock = fs.readFileSync(path.join(workspace, 'bun.lock'), 'utf8');
  const packages = parseBunLock(bunLock);
  const request = auditRequest(packages);
  const queriedAt = new Date().toISOString();
  // A separate, explicit complete request also covers the library outside bun.lock
  // and prevents scoped-registry omissions from looking like a successful audit.
  const response = await fetch(npmEndpoint, {
    method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(request), signal: AbortSignal.timeout(60000),
  });
  if (!response.ok) throw new Error(`npm advisory preparation failed (${response.status})`);
  const responseJson = await response.json();
  const bun = tool(['bun', 'audit', '--json']);
  const bunResponse = JSON.parse(bun.stdout);
  const npm = { endpoint: npmEndpoint, queriedAt, request, response: responseJson, responseSha256: sha256(JSON.stringify(responseJson)),
    bunLockSha256: sha256(bunLock), bunVersion: versions.bun, bunResponse, bunResponseSha256: sha256(JSON.stringify(bunResponse)),
    bunExitCode: bun.status, skippedRegistries: /skip(?:ping|ped).*registr/i.test(bun.stderr),
  };
  writeJson(path.join(prepared, 'prepared.json'), { schemaVersion: 1, database, databasePreparedAt, npm });
  console.log(`Explicit preparation complete: RustSec ${database.revision}; npm ${Object.keys(request).length} package names queried at ${queriedAt}.`);
}

function licenseFiles(directory) {
  return fs.readdirSync(directory).filter(name => /^(?:licen[cs]e|copying|notice|copyright|authors)(?:$|[._-])/i.test(name)
    || /^ThirdPartyNotices\.txt$/i.test(name)).filter(name => fs.statSync(path.join(directory, name)).isFile()).sort();
}
function walk(directory) {
  return fs.readdirSync(directory, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name, 'en')).flatMap(entry => {
    const filename = path.join(directory, entry.name);
    return entry.isDirectory() ? walk(filename) : entry.isFile() ? [filename] : [];
  });
}

export function materialScripts(repoRoot = root, runtimeRoot = workspace) {
  const source = path.join(repoRoot, 'skills/phoenix-ui/src/scripts');
  return fs.existsSync(source) ? source : path.join(runtimeRoot, 'skill/scripts');
}

export function collectInventory(metadata, cargoLock, nodePackages) {
  const texts = new Map();
  const preserve = (filename, location) => {
    const content = fs.readFileSync(filename); const hash = sha256(content);
    texts.set(hash, content);
    return { path: `THIRD-PARTY-LICENSES/${hash}.txt`, sha256: hash, originalLocation: location };
  };
  const runtimeLicense = preserve(path.join(workspace, 'LICENSE'), 'tools/phoenix-ui/LICENSE');
  const runtimeNotice = preserve(path.join(workspace, 'NOTICE.md'), 'tools/phoenix-ui/NOTICE.md');
  const upstream = readJson(path.join(workspace, 'UPSTREAM.json'));
  const byName = new Map(metadata.packages.map(p => [`${p.name}@${p.version}`, p]));
  if (byName.size !== cargoLock.length) throw new Error('Cargo metadata did not cover every locked package (including optional, build, dev and target dependencies)');
  const rust = cargoLock.map(locked => {
    const p = byName.get(`${locked.name}@${locked.version}`);
    if (!p || (p.source ?? undefined) !== locked.source) throw new Error(`Cargo metadata/lock mismatch: ${locked.name}@${locked.version}`);
    const directory = path.dirname(p.manifest_path);
    if (locked.source && locked.source !== 'registry+https://github.com/rust-lang/crates.io-index') throw new Error(`Unsupported Cargo source: ${locked.source}`);
    const sourceUrl = locked.source ? `https://static.crates.io/crates/${p.name}/${p.name}-${p.version}.crate` : `${upstream.repository}/tree/${upstream.sourceCommit}/${path.relative(workspace, directory).split(path.sep).join('/')}`;
    let licenses;
    let licenseLocations;
    if (locked.source) {
      verifyCrate(directory, locked);
      const files = licenseFiles(directory);
      if (p.license_file && !files.includes(p.license_file)) files.push(p.license_file);
      licenses = files.map(filename => preserve(path.join(directory, filename), `https://docs.rs/crate/${p.name}/${p.version}/source/${filename}`));
      licenseLocations = [`https://docs.rs/crate/${p.name}/${p.version}/source/Cargo.toml.orig`];
      if (!licenses.length) {
        // selectors explicitly links the MPL text from each source header.
        // The winapi target archives point to Apache/MIT in src/lib.rs; r-efi has AUTHORS.
        const locations = ['src/lib.rs', 'lib.rs', 'README.md', 'AUTHORS'].filter(f => fs.existsSync(path.join(directory, f)));
        licenseLocations.push(...locations.map(f => `https://docs.rs/crate/${p.name}/${p.version}/source/${f}`));
        if (p.license === 'MPL-2.0') {
          const mpl = metadata.packages.find(q => q.license === 'MPL-2.0' && fs.existsSync(path.join(path.dirname(q.manifest_path), 'LICENSE')));
          if (!mpl) throw new Error('Missing complete MPL-2.0 terms');
          licenses.push(preserve(path.join(path.dirname(mpl.manifest_path), 'LICENSE'), 'https://www.mozilla.org/en-US/MPL/2.0/'));
        }
      }
    } else { licenses = [runtimeLicense, runtimeNotice]; licenseLocations = ['tools/phoenix-ui/LICENSE']; }
    return { name: p.name, version: p.version, license: p.license, sourceUrl, ...(locked.checksum ? { checksum: locked.checksum, sourceUnmodified: true } : {
      sourceEvidence: 'Imported Apache-2.0 base; local workspace version/modifications are not an upstream release',
      localManifest: path.relative(root, p.manifest_path).split(path.sep).join('/'), manifestSha256: fileHash(p.manifest_path),
    }), licenses, licenseLocations };
  });
  const extra = new Map(readJson(path.join(prepared, 'extra-node-licenses.json')).map(p => [p.spec, p]));
  const node = nodePackages.map(locked => {
    const directory = path.join(workspace, 'node_modules', locked.key);
    const p = readJson(path.join(directory, 'package.json'));
    requireEqual(`${p.name}@${p.version}`, `${locked.name}@${locked.version}`, `Installed Node package differs from bun.lock: ${locked.key}`);
    if (!licenseAllowed(p.license)) throw new Error(`Rejected/unknown Node license: ${locked.name}@${locked.version}: ${JSON.stringify(p.license)}`);
    const metadataUrl = `https://registry.npmjs.org/${encodeURIComponent(p.name)}/${p.version}`;
    const licenses = licenseFiles(directory).map(filename => preserve(path.join(directory, filename), `npm:${p.name}@${p.version}/${filename}`));
    const source = extra.get(`${p.name}@${p.version}`);
    const licenseLocations = [metadataUrl];
    if (!licenses.length) {
      if (!source) throw new Error(`No preserved license or pinned source location for ${p.name}@${p.version}`);
      requireEqual(source.integrity, locked.integrity, `${p.name}: source provenance differs from bun.lock`);
      if (source.declarationOnly) {
        licenses.push(preserve(path.join(directory, 'package.json'), `npm:${p.name}@${p.version}/package.json`));
        licenseLocations.push(source.url);
      } else licenses.push(preserve(path.join(prepared, `${source.sha256}.txt`), source.url));
    }
    return { name: p.name, version: p.version, license: p.license, integrity: locked.integrity, sourceUrl: metadataUrl, licenses, licenseLocations,
      ...(source?.declarationOnly ? { licenseEvidence: 'MIT declaration in pinned README/package.json; upstream supplies no separate LICENSE/copyright text' } : {}),
    };
  });
  const bundledLicense = preserve(path.join(prepared, `${modernScreenshot.licenseSha256}.txt`), `npm:modern-screenshot@4.7.0/package/LICENSE`);
  requireEqual(bundledLicense.sha256, modernScreenshot.licenseSha256, 'Bundled modern-screenshot license changed');
  requireEqual(fileHash(path.join(materialScripts(), 'modern-screenshot.umd.js')), modernScreenshot.sha256, 'Missing/modified bundled modern-screenshot source');
  const folders = ['browser-bundle', 'skill/scripts', 'ui/component-review', 'crates'].map(imported => ({
    imported, directory: imported === 'skill/scripts' ? materialScripts() : path.join(workspace, imported),
  }));
  const resources = folders.flatMap(({ imported, directory }) => walk(directory).map(filename => ({
    filename, importedPath: `${imported}/${path.relative(directory, filename).split(path.sep).join('/')}`,
  })))
    .filter(({ importedPath: filename }) => {
      return /\.(?:js|css|ttf|otf|woff2?|svelte|ts|html)$/.test(filename) && !/(?:^|\/)tests?\//.test(filename) && !/\.test\.ts$/.test(filename);
    })
    .map(({ filename, importedPath }) => {
      const relative = path.relative(root, filename).split(path.sep).join('/');
      const hash = fileHash(filename);
      let license = 'Apache-2.0'; let licenses = [runtimeLicense, runtimeNotice];
      if (importedPath === modernScreenshot.importedPath) {
        requireEqual(hash, modernScreenshot.sha256, 'Unknown/modified bundled modern-screenshot; update its provenance and advisory query explicitly');
        license = 'MIT'; licenses = [bundledLicense];
      } else if (/\.(?:ttf|otf|woff2?)$/.test(relative)) {
        const ofl = filename.replace(/\.[^.]+$/, '-OFL.txt');
        if (!fs.existsSync(ofl) || !fs.readFileSync(ofl, 'utf8').includes('SIL OPEN FONT LICENSE Version 1.1')) throw new Error(`Missing OFL license for ${relative}`);
        requireEqual(hash, upstream.importedFiles[importedPath], `Modified/unknown font ${relative}; reserved-name/source review required`);
        license = 'OFL-1.1'; licenses = [preserve(ofl, path.relative(root, ofl).split(path.sep).join('/'))];
      }
      return { path: relative, importedPath, sha256: hash, license, licenses, sourceUrl: `${upstream.repository}/blob/${upstream.sourceCommit}/${importedPath}`,
        upstreamSha256: upstream.importedFiles[importedPath] ?? null, modifiedSinceImport: hash !== upstream.importedFiles[importedPath],
        ...(importedPath.startsWith('ui/component-review/vendor/') ? { origin: 'Redistributed in the pinned Apache-2.0 repository; sync script identifies the original private impeccable-site repo' } : {}),
        ...(/\/(?:component-review|detect-antipatterns-browser)\.js$/.test(relative) ? { additionalLicenses: 'Generated asset: also distribute all Rust/Node notices in this inventory; build freshness is a separate check' } : {}),
        ...(importedPath === modernScreenshot.importedPath ? { npmSource: modernScreenshot } : {}),
      };
    });
  return { catalog: { schemaVersion: 1, inputs: {
    cargoLockSha256: fileHash(path.join(workspace, 'Cargo.lock')), bunLockSha256: fileHash(path.join(workspace, 'bun.lock')),
    policySha256: fileHash(path.join(workspace, 'deny.toml')),
    checkerSha256: fileHash(fileURLToPath(import.meta.url)), buildToolsSha256: fileHash(path.join(workspace, 'build-tools.lock.json')),
  }, rust, node, bundled: [{ ...modernScreenshot, path: `${path.relative(root, materialScripts()).split(path.sep).join('/')}/modern-screenshot.umd.js`, licenses: [bundledLicense] }], resources }, texts };
}

export function assertDistribution(directory, catalog, notices = fs.readFileSync(path.join(workspace, 'THIRD-PARTY-NOTICES.md'))) {
  const required = new Map([['THIRD-PARTY.json', sha256(`${JSON.stringify(catalog, null, 2)}\n`)], ['THIRD-PARTY-NOTICES.md', sha256(notices)]]);
  for (const entry of [...catalog.rust, ...catalog.node, ...catalog.bundled, ...catalog.resources]) {
    for (const license of entry.licenses) required.set(license.path, license.sha256);
    if (entry.license === 'MPL-2.0' && (!entry.sourceUnmodified || !/^[a-f0-9]{64}$/.test(entry.checksum ?? '')
      || entry.sourceUrl !== `https://static.crates.io/crates/${entry.name}/${entry.name}-${entry.version}.crate` || !entry.licenses.length)) {
      throw new Error(`Missing exact MPL corresponding source: ${entry.name}`);
    }
  }
  for (const [filename, hash] of required) {
    const full = path.join(directory, filename);
    if (!fs.existsSync(full) || fileHash(full) !== hash) throw new Error(`Distribution is missing or changed: ${filename}`);
  }
  return { status: 'passed', requiredFiles: required.size, directory: path.relative(root, directory).split(path.sep).join('/') };
}

async function check(write, distributionDirectory) {
  preflight();
  const receipt = readJson(path.join(prepared, 'prepared.json'));
  const database = databaseState();
  requireEqual(JSON.stringify(database), JSON.stringify(receipt.database), 'Prepared RustSec database evidence changed');
  const nodePackages = parseBunLock(fs.readFileSync(path.join(workspace, 'bun.lock'), 'utf8'));
  const npm = validateNpmSnapshot(receipt.npm, nodePackages, fileHash(path.join(workspace, 'bun.lock')));
  const metadata = JSON.parse(successful(tool(['cargo', 'metadata', '--locked', '--offline', '--all-features', '--format-version=1'], { CARGO_NET_OFFLINE: 'true' }), 'offline Cargo metadata'));
  const inventory = collectInventory(metadata, parseCargoLock(fs.readFileSync(path.join(workspace, 'Cargo.lock'), 'utf8')), nodePackages);
  const catalogFile = path.join(workspace, 'THIRD-PARTY.json');
  if (write) {
    fs.mkdirSync(path.join(workspace, 'THIRD-PARTY-LICENSES'), { recursive: true });
    for (const [hash, text] of inventory.texts) fs.writeFileSync(path.join(workspace, `THIRD-PARTY-LICENSES/${hash}.txt`), text);
    writeJson(catalogFile, inventory.catalog);
  } else requireEqual(JSON.stringify(readJson(catalogFile)), JSON.stringify(inventory.catalog), 'Third-party inventory is stale; regenerate it explicitly with --write');
  for (const [hash] of inventory.texts) requireEqual(fileHash(path.join(workspace, `THIRD-PARTY-LICENSES/${hash}.txt`)), hash, 'Preserved third-party license text changed');
  const command = ['--all-features', '--frozen', '--format', 'json', 'check', 'advisories', 'licenses'];
  const result = deny(command, true);
  const rust = cargoDiagnostics(result.stdout, result.stderr);
  const blockers = rust.findings.filter(f => f.severity === 'error');
  if (result.status !== 0 && !blockers.length) blockers.push({ code: 'check-failed', message: `cargo-deny exited ${result.status}` });
  const distribution = distributionDirectory ? assertDistribution(path.resolve(distributionDirectory), inventory.catalog) : { status: 'not-run', reason: 'No binary/npm distribution directory was supplied' };
  const report = { schemaVersion: 1, checkedAt: new Date().toISOString(), status: blockers.length ? 'blocked' : 'passed',
    scope: 'Locked Rust (all features/targets/build/dev), Node/Bun build dependencies and bundled fonts/JS; final package/platform acceptance is separate',
    tools: { cargoDeny: versions.cargoDeny, rust: versions.rust, node: versions.node, bun: versions.bun },
    database: { ...database, preparedAt: receipt.databasePreparedAt },
    inputs: inventory.catalog.inputs, npm: { ...npm, endpoint: receipt.npm.endpoint, requestSha256: sha256(JSON.stringify(receipt.npm.request)), responseSha256: receipt.npm.responseSha256, bunResponseSha256: receipt.npm.bunResponseSha256 },
    rust: { exitCode: result.status, ...rust }, inventory: { rustPackages: inventory.catalog.rust.length, nodePackages: nodePackages.length, bundledPackages: 1, resources: inventory.catalog.resources.length, preservedTexts: inventory.texts.size },
    commands: { prepare: 'node scripts/check-phoenix-deps.mjs --prepare', check: 'node scripts/check-phoenix-deps.mjs', rust: `mise exec --no-deps -- cargo-deny --manifest-path Cargo.toml --config deny.toml ${command.join(' ')}`, nodePreparation: 'mise exec --no-deps -- bun audit --json + explicit npm bulk request for every locked/bundled package' },
    distribution, blockers,
  };
  if (write) writeJson(path.join(workspace, 'dependency-audit.json'), report);
  console.log(JSON.stringify(report, null, 2));
  if (blockers.length) process.exitCode = 1;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  const distributionIndex = args.indexOf('--distribution-dir');
  const directory = distributionIndex >= 0 ? args[distributionIndex + 1] : undefined;
  const known = new Set(['--prepare', '--write', '--distribution-dir', ...(directory ? [directory] : [])]);
  try {
    if (args.some(a => !known.has(a)) || (distributionIndex >= 0 && !directory) || (args.includes('--prepare') && (args.includes('--write') || directory))) throw new Error('Usage: node scripts/check-phoenix-deps.mjs [--prepare | --write] [--distribution-dir DIR]');
    if (args.includes('--prepare')) await prepare(); else await check(args.includes('--write'), directory);
  } catch (error) {
    if (args.includes('--write')) writeJson(path.join(workspace, 'dependency-audit.json'), {
      schemaVersion: 1, checkedAt: new Date().toISOString(), status: 'blocked', error: error.message.replaceAll(root, '<repo>').replaceAll(os.homedir(), '<home>'),
      scope: 'Preconditions or dependency checks failed; this invocation has no passing audit evidence',
    });
    console.error(error.message); process.exitCode = 1;
  }
}
