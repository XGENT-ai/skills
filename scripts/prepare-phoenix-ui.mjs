#!/usr/bin/env node
// Network access belongs to this explicit preparation entry point. Build/test
// scripts only consume the resulting pinned tools and dependency caches.
import fs from 'node:fs';
import path from 'node:path';
import https from 'node:https';
import { createHash } from 'node:crypto';
import { gunzipSync } from 'node:zlib';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { workspace, versions, miseEnvironment, preflight } from './phoenix-build-tools.mjs';

const root = path.resolve(workspace, '../..');
const toolRoot = path.join(root, 'local/phoenix-ui/build-tools');
const evidenceRoot = path.join(root, 'local/phoenix-ui/ci-prepare');
export const assets = JSON.parse(fs.readFileSync(path.join(workspace, 'build-tool-assets.json')));
export const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const json = value => JSON.stringify(value, null, 2) + '\n';

export function nativePlatform(platform = process.platform, arch = process.arch) {
  const name = `${platform === 'win32' ? 'windows' : platform}-${arch}`;
  if (!assets.platforms[name]) throw new Error(`No native Phoenix build for ${platform}/${arch}. Windows ARM runs the separately verified x64 fallback smoke.`);
  return name;
}

export function validatePins() {
  if (assets.schemaVersion !== 1) throw new Error('Unsupported build-tool asset schema');
  for (const tool of ['wasmBindgen', 'binaryen']) {
    if (String(assets.tools[tool].version) !== String(versions[tool])) throw new Error(`${tool} asset catalog differs from build-tools.lock.json`);
    if (Object.keys(assets.tools[tool].assets).sort().join() !== Object.keys(assets.platforms).sort().join()) throw new Error(`${tool} must pin all five native platforms`);
    for (const asset of Object.values(assets.tools[tool].assets)) {
      if (!Number.isSafeInteger(asset.size) || asset.size <= 0 || asset.size > 128 * 1024 * 1024
          || !/^[a-f0-9]{64}$/.test(asset.sha256) || !asset.url.startsWith(`${assets.tools[tool].release.replace('/tag/', '/download/')}/`)
          || new URL(asset.url).protocol !== 'https:') throw new Error(`Invalid pinned build-tool asset ${asset.name}`);
    }
  }
}

export function verifyArchive(bytes, asset) {
  if (bytes.length !== asset.size || sha256(bytes) !== asset.sha256) throw new Error(`Archive size/SHA-256 differs: ${asset.name}`);
  return bytes;
}

// Node 25's built-in agent opts into the user's HTTP(S)_PROXY/NO_PROXY settings
// without changing global agents or the build/test process environment.
function requestAsset(url, options, redirects = 0) {
  const agent = new https.Agent({ proxyEnv: process.env });
  return new Promise((resolve, reject) => {
    const request = https.get(url, { ...options, agent }, response => {
      response.once('close', () => agent.destroy());
      const status = response.statusCode;
      if ([301, 302, 303, 307, 308].includes(status) && response.headers.location) {
        response.destroy();
        try {
          const next = new URL(response.headers.location, url);
          if (next.protocol !== 'https:' || redirects >= 5) throw new Error('Build-tool redirect must remain HTTPS and bounded');
          resolve(requestAsset(next.href, options, redirects + 1));
        } catch (error) { reject(error); }
      } else {
        const ok = status >= 200 && status < 300;
        if (!ok) response.destroy();
        resolve({ ok, status, url, body: response });
      }
    });
    request.once('error', error => { agent.destroy(); reject(error); });
  });
}

export async function downloadAsset(asset, request = requestAsset) {
  if (new URL(asset.url).protocol !== 'https:') throw new Error('Build-tool downloads require HTTPS');
  const response = await request(asset.url, { signal: AbortSignal.timeout(60000) });
  if (!response.ok || (response.url && new URL(response.url).protocol !== 'https:')) throw new Error(`Build-tool download failed: ${asset.name} (${response.status})`);
  const chunks = []; let size = 0;
  for await (const chunk of response.body) {
    size += chunk.length;
    if (size > asset.size) throw new Error(`Download exceeded pinned size: ${asset.name}`);
    chunks.push(Buffer.from(chunk));
  }
  return verifyArchive(Buffer.concat(chunks), asset);
}

function safeRelative(name) {
  if (!name || /[\\:\x00-\x1f\x7f]/.test(name) || name.startsWith('/')
      || name.split('/').some(part => !part || part === '.' || part === '..' || /[. ]$/.test(part)
        || /^(?:con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)/i.test(part))) throw new Error(`Unsafe archive path: ${name}`);
  return name;
}

// These fixed tool archives contain regular files/directories. Refuse links,
// devices and extended headers instead of letting system tar interpret them.
export function archiveEntries(bytes, asset) {
  verifyArchive(bytes, asset);
  const tar = gunzipSync(bytes, { maxOutputLength: 1024 * 1024 * 1024 });
  const entries = [], names = new Map(); let end = false;
  const field = (header, start, size) => header.subarray(start, start + size).toString('utf8').replace(/\0.*$/s, '');
  const octal = text => {
    if (!/^[0-7]+$/.test(text.trim())) throw new Error('Invalid tar number');
    return Number.parseInt(text.trim(), 8);
  };
  for (let offset = 0; offset + 512 <= tar.length;) {
    const header = tar.subarray(offset, offset + 512);
    if (header.every(byte => byte === 0)) {
      if (offset + 1024 > tar.length || tar.subarray(offset).some(byte => byte !== 0)) throw new Error('Invalid tar footer');
      end = true; break;
    }
    const checksum = header.reduce((sum, byte, i) => sum + (i >= 148 && i < 156 ? 32 : byte), 0);
    if (checksum !== octal(field(header, 148, 8))) throw new Error('Tar header checksum differs');
    const type = field(header, 156, 1) || '0';
    if (!['0', '5'].includes(type) || field(header, 157, 100)) throw new Error(`Unsupported archive member type ${type}`);
    const prefix = field(header, 345, 155);
    const name = safeRelative(`${prefix ? prefix + '/' : ''}${field(header, 0, 100)}`.replace(/\/$/, ''));
    if (name !== asset.archiveRoot && !name.startsWith(asset.archiveRoot + '/')) throw new Error(`Unexpected archive root: ${name}`);
    const key = name.toLowerCase();
    if (names.has(key)) throw new Error(`Duplicate archive path: ${name}`);
    const size = octal(field(header, 124, 12));
    if (type === '5' && size !== 0) throw new Error('Directory contains archive data');
    const next = offset + 512 + Math.ceil(size / 512) * 512;
    if (next > tar.length) throw new Error('Truncated tar member');
    const entry = { name, directory: type === '5', mode: octal(field(header, 100, 8)), bytes: tar.subarray(offset + 512, offset + 512 + size) };
    entries.push(entry); names.set(key, entry); offset = next;
  }
  if (!end || !entries.length) throw new Error('Missing tar footer or members');
  for (const entry of entries) {
    const parts = entry.name.split('/'); parts.pop();
    while (parts.length) {
      if (names.get(parts.join('/').toLowerCase())?.directory === false) throw new Error('Archive file is used as a parent directory');
      parts.pop();
    }
  }
  if (!entries.some(entry => entry.name === `${asset.archiveRoot}/${asset.executable}` && !entry.directory)) throw new Error('Pinned tool executable is missing from archive');
  return entries;
}

function noSymlinkParents(directory) {
  for (let current = path.resolve(directory);;) {
    try {
      const stat = fs.lstatSync(current);
      if (!stat.isDirectory() || stat.isSymbolicLink()) throw new Error(`Unsafe tool destination: ${current}`);
    } catch (error) { if (error.code !== 'ENOENT') throw error; }
    const parent = path.dirname(current); if (parent === current) break; current = parent;
  }
}

export function extractArchive(bytes, asset, destination) {
  const entries = archiveEntries(bytes, asset); // Entire archive validated before writing.
  noSymlinkParents(path.dirname(destination));
  let existing;
  try { existing = fs.lstatSync(destination); } catch (error) { if (error.code !== 'ENOENT') throw error; }
  if (existing) {
    if (!existing.isDirectory() || existing.isSymbolicLink()) throw new Error('Unsafe existing tool destination');
    const expected = new Map(entries.filter(entry => !entry.directory).map(entry => [entry.name.slice(asset.archiveRoot.length + 1), entry]));
    const walk = directory => {
      noSymlinkParents(directory);
      for (const name of fs.readdirSync(directory)) {
        const file = path.join(directory, name), stat = fs.lstatSync(file);
        if (stat.isDirectory() && !stat.isSymbolicLink()) walk(file);
        else {
          const relative = path.relative(destination, file).split(path.sep).join('/'), entry = expected.get(relative);
          if (!stat.isFile() || !entry || sha256(fs.readFileSync(file)) !== sha256(entry.bytes)) throw new Error(`Existing tool tree differs: ${relative}`);
          expected.delete(relative);
        }
      }
    };
    walk(destination);
    if (expected.size) throw new Error('Existing tool tree is incomplete');
  } else {
    fs.mkdirSync(path.dirname(destination), { recursive: true });
    const staging = fs.mkdtempSync(path.join(path.dirname(destination), '.phoenix-tool-'));
    try {
      for (const entry of entries) {
        const file = path.join(staging, entry.name);
        fs.mkdirSync(entry.directory ? file : path.dirname(file), { recursive: true });
        if (!entry.directory) fs.writeFileSync(file, entry.bytes, { flag: 'wx', mode: entry.mode & 0o111 ? 0o755 : 0o644 });
      }
      fs.renameSync(path.join(staging, asset.archiveRoot), destination);
    } finally { fs.rmSync(staging, { recursive: true, force: true }); }
  }
  return path.join(destination, ...safeRelative(asset.executable).split('/'));
}

export function preparationCommands(platform) {
  const nativeTarget = assets.platforms[platform]?.rustTarget;
  if (!nativeTarget) throw new Error('Unsupported native preparation platform');
  return [
    ['mise', ['install']],
    ['mise', ['exec', '--no-deps', '--', 'rustup', 'target', 'add', '--toolchain', versions.rust, nativeTarget, versions.wasmTarget]],
    ['mise', ['exec', '--no-deps', '--', 'rustup', 'component', 'add', '--toolchain', versions.rust, 'rustfmt', 'clippy']],
    ['mise', ['exec', '--no-deps', '--', 'mbx', 'doctor']],
    // The audit checks the entire lock, including dependencies of other hosts.
    ['mise', ['exec', '--no-deps', '--', 'mbx', 'fetch', '--locked']],
    ['mise', ['exec', '--no-deps', '--', 'bun', 'install', '--frozen-lockfile']],
  ];
}

async function prepare() {
  validatePins(); const platform = nativePlatform();
  if (process.env.PHOENIX_UI_BUILD_TARGET && process.env.PHOENIX_UI_BUILD_TARGET !== assets.platforms[platform].rustTarget) throw new Error('Preparation requires the runner native Rust target');
  const record = { schemaVersion: 1, startedAt: new Date().toISOString(), platform, status: 'preparing', tools: versions, commands: [], archives: [] };
  noSymlinkParents(evidenceRoot); fs.mkdirSync(evidenceRoot, { recursive: true });
  const recordPath = path.join(evidenceRoot, `${platform}.json`);
  const env = miseEnvironment({ CARGO_NET_OFFLINE: 'false', PUPPETEER_SKIP_DOWNLOAD: 'true', PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD: '1' });
  const run = (program, args, capture = false) => {
    record.commands.push({ program, args });
    const result = spawnSync(program, args, { cwd: workspace, env, stdio: capture ? 'pipe' : 'inherit', encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
    Object.assign(record.commands.at(-1), { exitCode: result.status, error: result.error?.message });
    if (result.error || result.status !== 0) throw result.error || new Error(`${program} ${args.join(' ')} failed (${result.status}): ${result.stderr || ''}`);
    return result.stdout;
  };
  try {
    for (const [program, args] of preparationCommands(platform)) run(program, args);
    for (const [tool, variable] of [['wasmBindgen', 'WASM_BINDGEN'], ['binaryen', 'WASM_OPT']]) {
      const asset = assets.tools[tool].assets[platform];
      const archiveRecord = { name: asset.name, url: asset.url, size: asset.size, sha256: asset.sha256, status: 'downloading' };
      record.archives.push(archiveRecord);
      const bytes = await downloadAsset(asset);
      env[variable] = extractArchive(bytes, asset, path.join(toolRoot, platform, asset.archiveRoot));
      archiveRecord.status = 'verified-and-extracted'; archiveRecord.executable = env[variable];
    }
    const suffix = process.platform === 'win32' ? '.exe' : '';
    const candidates = [process.env.PHOENIX_CARGO_DENY, path.join(toolRoot, 'cargo-deny/bin', `cargo-deny${suffix}`),
      ...(process.env.PATH || '').split(path.delimiter).map(directory => path.join(directory, `cargo-deny${suffix}`))].filter(Boolean);
    env.PHOENIX_CARGO_DENY = candidates.find(file => {
      if (!fs.existsSync(file) || !fs.lstatSync(file).isFile()) return false;
      const result = spawnSync(file, ['--version'], { encoding: 'utf8' });
      return !result.error && result.status === 0 && result.stdout.trim() === `cargo-deny ${versions.cargoDeny}`;
    });
    if (!env.PHOENIX_CARGO_DENY) {
      run('mise', ['exec', '--no-deps', '--', 'cargo', 'install', '--locked', '--version', versions.cargoDeny, '--root', path.join(toolRoot, 'cargo-deny'), 'cargo-deny']);
      env.PHOENIX_CARGO_DENY = path.join(toolRoot, 'cargo-deny/bin', `cargo-deny${suffix}`);
    }
    if (run(env.PHOENIX_CARGO_DENY, ['--version'], true).trim() !== `cargo-deny ${versions.cargoDeny}`) throw new Error('Prepared cargo-deny version differs');
    // The existing audit entry point owns RustSec pins, license materials and
    // advisory snapshots. Preparation does not silently waive its failures.
    run('mise', ['exec', '--no-deps', '--', 'node', path.join(root, 'scripts/check-phoenix-deps.mjs'), '--prepare']);
    Object.assign(process.env, env); preflight();
    const preparedEnv = Object.fromEntries(['WASM_BINDGEN', 'WASM_OPT', 'PHOENIX_CARGO_DENY'].map(name => [name, env[name]]));
    fs.writeFileSync(path.join(evidenceRoot, `${platform}.env.json`), json(preparedEnv));
    if (process.env.GITHUB_ENV) fs.appendFileSync(process.env.GITHUB_ENV, Object.entries(preparedEnv).map(([name, value]) => `${name}=${value}\n`).join(''));
    record.status = 'prepared'; record.finishedAt = new Date().toISOString();
    console.log(json({ status: record.status, platform, environmentFile: path.join(evidenceRoot, `${platform}.env.json`) }));
  } catch (error) { record.status = 'failed'; record.error = error.message; throw error; }
  finally { fs.writeFileSync(recordPath, json(record)); }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  if (args.length === 1 && args[0] === '--prepare') prepare().catch(error => { console.error(error.message); process.exitCode = 1; });
  else if (args.length === 0 || (args.length === 1 && args[0] === '--help')) console.log('Usage: node scripts/prepare-phoenix-ui.mjs --prepare\nExplicit online preparation: mise install, Rust targets/components, locked Cargo fetch, frozen Bun install (no browser downloads), verified WASM tools and dependency-audit preparation.\nBuild/test remain offline. Local shell: export the entries printed in local/phoenix-ui/ci-prepare/<platform>.env.json.');
  else { console.error('Usage: node scripts/prepare-phoenix-ui.mjs --prepare'); process.exitCode = 1; }
}
