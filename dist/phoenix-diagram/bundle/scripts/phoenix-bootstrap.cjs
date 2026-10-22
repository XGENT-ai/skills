'use strict';
// SPDX-License-Identifier: Apache-2.0
// Adapted from Phoenix UI's phoenix-bootstrap.cjs, engine 0.1.0, repository
// 4a65a1525f9ed03c775a0e18935901e1edbb773f, SHA-256
// 8b1723fb51e6aad9bfc0bc5ba8e09b5a373b0e33c251331f6ca14ff6b1c261ef.
// Its Apache-2.0 Impeccable provenance is retained in tools/phoenix-ui/NOTICE.md.
// Phoenix changes: diagram-only identities/resources/cache, Unix targets,
// explicit acquisition, complete offline validation and installed ownership.
// No hooks, renderer, UI installer, migration or update lookup is included.

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const crypto = require('node:crypto');
const http = require('node:http');
const https = require('node:https');
const tls = require('node:tls');
const net = require('node:net');
const { spawn, spawnSync } = require('node:child_process');

const TOOL = 'phoenix-diagram';
const TOOL_VERSION = '0.1.0';
const TARGETS = ['darwin-arm64', 'darwin-x64', 'linux-x64', 'linux-arm64'];
const PROVIDERS = ['.agent', '.agents', '.claude', '.codex', '.cursor', '.dsh', '.gemini', '.github', '.grok', '.hermes', '.kiro', '.opencode', '.pi', '.qoder', '.rovodev', '.trae', '.trae-cn', '.veto', '.vibe'];
const SHA = /^[a-f0-9]{64}$/;
const SEMVER = /^\d+\.\d+\.\d+(?:-[a-zA-Z0-9.-]+)?$/;
const MAX_ENGINE = 256 * 1024 * 1024;
const sha256 = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const failure = (message, code = 4) => Object.assign(new Error(message), { code });
const encoded = value => Buffer.from(JSON.stringify(value) + '\n');
const object = value => value && typeof value === 'object' && !Array.isArray(value);

function platformTarget(platform = process.platform, arch = process.arch) {
  const target = `${platform}-${arch}`;
  if (!TARGETS.includes(target)) throw failure(`Unsupported Phoenix Diagram platform ${platform}/${arch}; the engine supports the four declared macOS/Linux targets.`);
  return target;
}
function packageVersion(options = {}) {
  if (options.packageJsonPath) {
    const data = JSON.parse(readRegular(path.resolve(options.packageJsonPath), 1024 * 1024).bytes);
    if (data.name !== '@xgent-ai/skills' || !SEMVER.test(data.version || '')) throw failure('Invalid owning npm package identity.');
    return data.version;
  }
  // This relative location is the owning npm package, not an application's
  // package.json above an installed provider skill.
  const file = path.resolve(__dirname, '../../..', 'package.json');
  if (fs.existsSync(file)) {
    const data = JSON.parse(readRegular(file, 1024 * 1024).bytes);
    if (data.name === '@xgent-ai/skills' && SEMVER.test(data.version || '')) return data.version;
  }
  return undefined;
}
function readRegular(file, limit, mode, digestOnly = false) {
  const stat = fs.lstatSync(file, { bigint: true });
  if (!stat.isFile() || stat.isSymbolicLink() || stat.nlink !== 1n) throw failure(`Expected a regular, singly linked file, not a symlink: ${file}`);
  if (stat.size > BigInt(limit) || (mode !== undefined && Number(stat.mode & 0o777n) !== mode)) throw failure(`File size or mode differs from the pinned resource: ${file}`);
  const fd = fs.openSync(file, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW);
  try {
    const opened = fs.fstatSync(fd, { bigint: true });
    let bytes; let digest; let size;
    if (digestOnly) {
      const hash = crypto.createHash('sha256'); const chunk = Buffer.allocUnsafe(64 * 1024); size = 0;
      for (;;) {
        const length = fs.readSync(fd, chunk, 0, chunk.length, null);
        if (!length) break;
        if ((size += length) > limit) throw failure(`Resource exceeds its size bound: ${file}`);
        hash.update(chunk.subarray(0, length));
      }
      digest = hash.digest('hex');
    } else { bytes = fs.readFileSync(fd); size = bytes.length; }
    const after = fs.fstatSync(fd, { bigint: true });
    if (opened.dev !== stat.dev || opened.ino !== stat.ino || after.size !== stat.size || size > limit || BigInt(size) !== stat.size || opened.mtimeNs !== after.mtimeNs || opened.ctimeNs !== after.ctimeNs) throw failure(`Resource changed during verification: ${file}`);
    return { bytes, stat, size, digest };
  } finally { fs.closeSync(fd); }
}
function relative(value) {
  return typeof value === 'string' && value.length > 0 && !value.includes('\\') && !value.includes(':') && !value.includes('\0')
    && value.split('/').every(part => part && part !== '.' && part !== '..' && !/[. ]$/.test(part));
}
function childFile(root, relativePath) {
  if (!relative(relativePath)) throw failure(`Unsafe relative resource path: ${relativePath}`);
  let directory = path.resolve(root);
  const parts = relativePath.split('/');
  for (const part of [null, ...parts.slice(0, -1)]) {
    if (part !== null) directory = path.join(directory, part);
    const stat = fs.lstatSync(directory);
    if (!stat.isDirectory() || stat.isSymbolicLink()) throw failure(`Resource parent is not a regular directory: ${directory}`);
  }
  return path.join(directory, parts[parts.length - 1]);
}
function allowed(name, derived = false) {
  if (!relative(name)) return false;
  if (['SKILL.md', 'agents/openai.yaml', 'schemas/README.md', 'scripts/phoenix-diagram', 'scripts/phoenix-launcher.cjs', 'scripts/phoenix-bootstrap.cjs'].includes(name)) return true;
  if (name === 'ENGINE.json') return derived;
  const parts = name.split('/');
  return parts.length === 2 && (['schemas', 'examples'].includes(parts[0]) && parts[1].endsWith('.json') && parts[1].length > 5
    || parts[0] === 'references' && parts[1].endsWith('.md') && parts[1].length > 3);
}
function checkedUrl(value, distribution) {
  const url = new URL(value);
  const local = ['127.0.0.1', '[::1]', 'localhost'].includes(url.hostname);
  if (url.username || url.password || (url.protocol !== 'https:' && !(distribution === 'candidate' && url.protocol === 'http:' && local))) throw failure('Engine download requires credential-free HTTPS; loopback HTTP is allowed only for candidate verification.');
  return url;
}
function manifestAt(file, options = {}) {
  file = path.resolve(file);
  const { bytes } = readRegular(file, 1024 * 1024);
  const data = JSON.parse(bytes);
  const keys = ['schemaVersion', 'distribution', 'tool', 'toolVersion', 'npmPackageVersion', 'sourceCommit', 'sourceInputSha256', 'inputContractSchema', 'receiptSchema', 'viewerRevision', 'bundleSha256', 'engines'];
  const npmVersion = options.packageJsonPath || path.basename(file) === 'VERSION.json' ? packageVersion(options) : undefined;
  if (!object(data) || Object.keys(data).some(key => !keys.includes(key)) || data.schemaVersion !== 1 || data.tool !== TOOL || data.toolVersion !== TOOL_VERSION
    || !['development', 'candidate', 'release'].includes(data.distribution) || !SEMVER.test(data.npmPackageVersion || '') || npmVersion && data.npmPackageVersion !== npmVersion
    || !/^[a-f0-9]{40}$/.test(data.sourceCommit || '') || !SHA.test(data.sourceInputSha256 || '') || !SHA.test(data.bundleSha256 || '')
    || data.inputContractSchema !== 1 || data.receiptSchema !== 1 || data.viewerRevision !== 'phoenix-v1' || !object(data.engines)) throw failure('Invalid Phoenix Diagram package manifest or fixed package/version/contract identity.');
  const targets = Object.keys(data.engines);
  if (!targets.length || targets.some(target => !TARGETS.includes(target)) || data.distribution !== 'development' && TARGETS.some(target => !targets.includes(target))) throw failure('Engine manifest is missing a supported platform or names an unknown target.');
  for (const [target, engine] of Object.entries(data.engines)) {
    if (!object(engine) || Object.keys(engine).some(key => !['asset', 'size', 'sha256', 'url'].includes(key)) || engine.asset !== `${TOOL}-${target}` || !SHA.test(engine.sha256 || '') || !Number.isSafeInteger(engine.size) || engine.size <= 0 || engine.size > MAX_ENGINE) throw failure(`Invalid engine metadata for ${target}.`);
    if (data.distribution !== 'development' && !engine.url) throw failure(`Pinned engine URL is missing for ${target}.`);
    if (engine.url) checkedUrl(engine.url, data.distribution);
  }
  return { data, bytes, hash: sha256(bytes), path: file };
}
function inventory(root, expected) {
  let count = 0;
  function visit(prefix) {
    const directory = prefix ? path.dirname(childFile(root, `${prefix}/placeholder`)) : root;
    for (const name of fs.readdirSync(directory)) {
      if (++count > 50000) throw failure('Bundle inventory exceeds its bound.');
      const namePath = prefix ? `${prefix}/${name}` : name;
      const file = childFile(root, namePath);
      const stat = fs.lstatSync(file);
      if (stat.isDirectory() && !stat.isSymbolicLink() && [...expected].some(item => item.startsWith(`${namePath}/`))) visit(namePath);
      else if (!stat.isFile() || stat.isSymbolicLink() || !expected.has(namePath)) throw failure(`Unlisted, extra or nonregular bundle entry: ${namePath}`);
    }
  }
  visit('');
}
function verifyBundle(root, manifest) {
  root = path.resolve(root);
  const raw = readRegular(childFile(root, 'manifest.json'), 16 * 1024 * 1024).bytes;
  const data = JSON.parse(raw);
  if (sha256(raw) !== manifest.data.bundleSha256 || !object(data) || Object.keys(data).some(key => !['schemaVersion', 'tool', 'version', 'files'].includes(key)) || data.schemaVersion !== 1 || data.tool !== TOOL || data.version !== TOOL_VERSION || !Array.isArray(data.files) || !data.files.length || data.files.length > 10000) throw failure('Bundle manifest identity or digest differs from VERSION.');
  const names = new Set(); const folded = new Set(); let total = 0;
  for (const entry of data.files) {
    if (!object(entry) || Object.keys(entry).some(key => !['path', 'size', 'sha256', 'mode'].includes(key)) || !allowed(entry.path) || folded.has(entry.path.toLowerCase()) || !SHA.test(entry.sha256 || '') || !Number.isSafeInteger(entry.size) || entry.size < 0 || entry.size > 64 * 1024 * 1024 || ![0o644, 0o755].includes(entry.mode)) throw failure(`Unsafe bundle file or duplicate entry: ${entry.path}`);
    if ((total += entry.size) > MAX_ENGINE) throw failure('Bundle exceeds its size bound.');
    const { bytes } = readRegular(childFile(root, entry.path), 64 * 1024 * 1024, entry.mode);
    if (bytes.length !== entry.size || sha256(bytes) !== entry.sha256) throw failure(`Bundle file digest or size differs: ${entry.path}`);
    names.add(entry.path); folded.add(entry.path.toLowerCase());
  }
  for (const required of ['SKILL.md', 'scripts/phoenix-diagram', 'scripts/phoenix-launcher.cjs', 'scripts/phoenix-bootstrap.cjs']) if (!names.has(required)) throw failure(`Incomplete bundle: ${required}`);
  const engine = readRegular(childFile(root, 'ENGINE.json'), 1024 * 1024, 0o644).bytes;
  if (!engine.equals(manifest.bytes)) throw failure('Bundle ENGINE.json must be a byte-for-byte copy of VERSION.json.');
  names.add('manifest.json'); names.add('ENGINE.json'); inventory(root, names);
  return { root, data, bytes: raw, hash: sha256(raw) };
}
function legalResources(root) {
  const files = new Map();
  for (const name of ['LICENSE', 'NOTICE.md', 'THIRD-PARTY.json', 'THIRD-PARTY-NOTICES.md']) {
    const bytes = readRegular(childFile(root, name), 64 * 1024 * 1024).bytes;
    if (!bytes.length) throw failure(`Empty license resource: ${name}`);
    files.set(name, bytes);
  }
  const catalog = JSON.parse(files.get('THIRD-PARTY.json'));
  function visit(value) {
    if (!object(value) && !Array.isArray(value)) return;
    if (object(value) && typeof value.path === 'string' && value.path.startsWith('THIRD-PARTY-LICENSES/')) {
      if (!SHA.test(value.sha256 || '')) throw failure(`Invalid license digest: ${value.path}`);
      const bytes = readRegular(childFile(root, value.path), 64 * 1024 * 1024).bytes;
      if (!bytes.length || sha256(bytes) !== value.sha256) throw failure(`License digest differs: ${value.path}`);
      files.set(value.path, bytes);
    }
    for (const child of Object.values(value)) visit(child);
  }
  visit(catalog);
  if (files.size === 4) throw failure('THIRD-PARTY catalog has no full license resources.');
  const licenses = new Set([...files.keys()].filter(name => name.startsWith('THIRD-PARTY-LICENSES/')).map(name => name.slice('THIRD-PARTY-LICENSES/'.length)));
  inventory(path.join(root, 'THIRD-PARTY-LICENSES'), licenses);
  return files;
}
function verifyOffline(releaseDir, manifest, packageBundle) {
  const offline = manifestAt(path.join(releaseDir, 'VERSION.json'));
  if (!offline.bytes.equals(manifest.bytes)) throw failure('Offline VERSION manifest differs byte-for-byte from this npm package.');
  if (manifest.data.distribution !== 'development' && TARGETS.some(target => !manifest.data.engines[target])) throw failure('Offline release must contain all four platform assets.');
  const bundle = verifyBundle(path.join(releaseDir, 'bundle'), manifest);
  if (packageBundle && !bundle.bytes.equals(packageBundle.bytes)) throw failure('Offline bundle manifest differs from the package.');
  for (const target of Object.keys(manifest.data.engines)) {
    const meta = manifest.data.engines[target];
    const { size, digest } = readRegular(childFile(releaseDir, meta.asset), MAX_ENGINE, undefined, true);
    if (size !== meta.size || digest !== meta.sha256) throw failure(`Offline engine digest or size differs: ${target}`);
  }
  const expected = path.basename(manifest.path) === 'VERSION.json' ? legalResources(path.dirname(manifest.path)) : null;
  const actual = legalResources(releaseDir);
  if (expected && (expected.size !== actual.size || [...expected].some(([name, bytes]) => !actual.get(name)?.equals(bytes)))) throw failure('Offline license resources differ from this package.');
  return offline;
}
function cacheRoot(env = process.env) { return path.resolve(env.PHOENIX_DIAGRAM_HOME || path.join(os.homedir(), '.phoenix-diagram')); }
function enginePath(manifest, target, env = process.env) {
  const meta = manifest.data.engines[target];
  if (!meta) throw failure(`This package has no verified engine for ${target}.`);
  return path.join(cacheRoot(env), 'bin', manifest.data.toolVersion, target, meta.sha256, TOOL);
}
function verifyEngine(file, manifest, target, env = process.env) {
  const meta = manifest.data.engines[target];
  if (!meta) throw failure(`No pinned engine for ${target}.`);
  const { size, digest, stat } = readRegular(file, MAX_ENGINE, undefined, true);
  if (size !== meta.size || digest !== meta.sha256 || Number(stat.mode & 0o111n) === 0) throw failure('Phoenix Diagram binary digest, size or executable mode differs from VERSION.');
  const probe = spawnSync(file, ['engine-probe'], { env, encoding: 'utf8', timeout: 3000, maxBuffer: 4096, windowsHide: true });
  if (probe.error || probe.status !== 0 || probe.stdout !== `phoenix-diagram-engine ${TOOL_VERSION} input=1 receipt=1 viewer=phoenix-v1\n`) throw failure(`Phoenix Diagram engine probe identity differs: ${JSON.stringify({ error: probe.error?.code || null, status: probe.status, signal: probe.signal, stdout: probe.stdout, stderr: probe.stderr })}`);
  const after = fs.lstatSync(file, { bigint: true });
  if (after.dev !== stat.dev || after.ino !== stat.ino || after.size !== stat.size || after.mtimeNs !== stat.mtimeNs || after.ctimeNs !== stat.ctimeNs) throw failure('Phoenix Diagram binary changed during its probe.');
  return file;
}

// Transport copied from Phoenix UI's verified bootstrap; the destination and
// every redirect remain subject to this tool's stricter distribution rule.
function proxyFor(url, env) {
  const noProxy = env.no_proxy || env.NO_PROXY || '';
  const hostname = url.hostname.replace(/^\[|\]$/g, '').toLowerCase();
  if (noProxy.split(',').some(value => {
    const item = value.trim().toLowerCase();
    const [host, port] = item.split(':');
    return item === '*' || ((!port || port === (url.port || (url.protocol === 'https:' ? '443' : '80')))
      && (hostname === host || (host?.startsWith('.') && hostname.endsWith(host))));
  })) return null;
  const value = url.protocol === 'https:'
    ? env.https_proxy || env.HTTPS_PROXY || env.all_proxy || env.ALL_PROXY || env.http_proxy || env.HTTP_PROXY
    : env.http_proxy || env.HTTP_PROXY || env.all_proxy || env.ALL_PROXY;
  if (!value) return null;
  const proxy = new URL(value);
  if (!['http:', 'https:'].includes(proxy.protocol)) throw failure('Phoenix Diagram supports HTTP or HTTPS proxies.');
  return proxy;
}
async function responseFor(url, env) {
  const proxy = proxyFor(url, env);
  const authorization = proxy?.username ? `Basic ${Buffer.from(`${decodeURIComponent(proxy.username)}:${decodeURIComponent(proxy.password)}`).toString('base64')}` : null;
  let connection;
  if (proxy && url.protocol === 'https:') {
    connection = await new Promise((resolve, reject) => {
      const request = (proxy.protocol === 'https:' ? https : http).request(proxy, {
        method: 'CONNECT', path: `${url.hostname}:${url.port || 443}`,
        rejectUnauthorized: true,
        headers: authorization ? { 'Proxy-Authorization': authorization } : {},
      });
      request.setTimeout(60000, () => request.destroy(failure('Engine proxy timed out.')));
      request.on('connect', (res, socket, head) => {
        if (res.statusCode !== 200) { socket.destroy(); reject(failure(`Engine proxy HTTP ${res.statusCode}.`)); return; }
        if (head.length) socket.unshift(head);
        const hostname = url.hostname.replace(/^\[|\]$/g, '');
        let secure;
        try { secure = tls.connect({ socket, host: hostname, rejectUnauthorized: true, ...(net.isIP(hostname) ? {} : { servername: hostname }) }); }
        catch (error) { socket.destroy(); reject(error); return; }
        secure.setTimeout(60000, () => secure.destroy(failure('Engine proxy TLS timed out.')));
        secure.once('secureConnect', () => resolve(secure)); secure.once('error', reject);
      });
      request.once('error', reject); request.end();
    });
  }
  return new Promise((resolve, reject) => {
    // Leaving agent unset makes Node use this verified CONNECT socket. With
    // agent:false Node constructs another agent and can bypass the tunnel.
    const options = connection ? { createConnection: () => connection, rejectUnauthorized: true } : { rejectUnauthorized: true };
    const destination = proxy && url.protocol === 'http:' ? proxy : url;
    const client = proxy && url.protocol === 'http:' ? (proxy.protocol === 'https:' ? https : http) : url.protocol === 'https:' ? https : http;
    if (proxy && url.protocol === 'http:') Object.assign(options, { path: url.href, headers: { Host: url.host, ...(authorization ? { 'Proxy-Authorization': authorization } : {}) } });
    const request = client.get(destination, options, resolve);
    request.setTimeout(60000, () => request.destroy(failure('Engine download timed out.')));
    request.once('error', reject);
  });
}
async function download(meta, distribution, env) {
  let url = checkedUrl(meta.url, distribution);
  for (let redirects = 0; redirects <= 5; redirects++) {
    const res = await responseFor(url, env);
    if ([301, 302, 303, 307, 308].includes(res.statusCode)) {
      res.resume();
      if (!res.headers.location || redirects === 5) throw failure('Invalid engine download redirect or redirect limit exceeded.');
      const next = checkedUrl(new URL(res.headers.location, url).href, distribution);
      if (url.protocol === 'https:' && next.protocol !== 'https:') throw failure('Engine download refused an HTTPS downgrade.');
      url = next; continue;
    }
    if (res.statusCode !== 200) { res.resume(); throw failure(`Engine download HTTP ${res.statusCode}.`); }
    const chunks = []; let length = 0;
    for await (const chunk of res) {
      length += chunk.length;
      if (length > meta.size) { res.destroy(); throw failure('Engine download exceeded the pinned size.'); }
      chunks.push(chunk);
    }
    const bytes = Buffer.concat(chunks);
    if (bytes.length !== meta.size || sha256(bytes) !== meta.sha256) throw failure('Engine download failed the pinned digest/size check.');
    return bytes;
  }
  throw failure('Engine redirect limit exceeded.');
}
async function retryStagedFile(operation) {
  for (let attempt = 0;; attempt++) {
    try { return operation(); } catch (error) {
      if (!['EBUSY', 'EPERM'].includes(error.code) || attempt === 5) throw error;
      await new Promise(resolve => setTimeout(resolve, 25 * 2 ** attempt));
    }
  }
}
async function installEngine(manifest, target, project, releaseDir, env = process.env) {
  // Validate an explicit offline request before considering an existing cache.
  const packageBundle = path.basename(manifest.path) === 'VERSION.json' ? verifyBundle(path.join(path.dirname(manifest.path), 'bundle'), manifest) : null;
  if (packageBundle) legalResources(path.dirname(manifest.path));
  if (releaseDir) verifyOffline(path.resolve(releaseDir), manifest, packageBundle);
  const destination = enginePath(manifest, target, env);
  try { return verifyEngine(destination, manifest, target, env); } catch { /* Only explicit acquisition may repair a bad cache. */ }
  const meta = manifest.data.engines[target];
  let bytes;
  if (releaseDir) bytes = readRegular(childFile(releaseDir, meta.asset), MAX_ENGINE).bytes;
  else {
    if (!meta.url) throw failure('This development engine has no URL; supply a complete verified --release-dir.');
    bytes = await download(meta, manifest.data.distribution, env);
  }
  if (bytes.length !== meta.size || sha256(bytes) !== meta.sha256) throw failure('Engine digest/size differs from the fixed VERSION manifest.');
  const home = cacheRoot(env);
  fs.mkdirSync(home, { recursive: true, mode: 0o700 });
  const relativeDirectory = path.relative(home, path.dirname(destination));
  let directory = home;
  for (const part of relativeDirectory.split(path.sep)) {
    const stat = fs.lstatSync(directory);
    if (!stat.isDirectory() || stat.isSymbolicLink()) throw failure(`Unsafe engine cache directory: ${directory}`);
    directory = path.join(directory, part); fs.mkdirSync(directory, { recursive: true, mode: 0o700 });
  }
  if (fs.lstatSync(directory).isSymbolicLink()) throw failure('Unsafe content-addressed engine cache.');
  const temporary = `${destination}.${process.pid}.${crypto.randomBytes(6).toString('hex')}.part`;
  let created = false; let installError;
  try {
    fs.writeFileSync(temporary, bytes, { flag: 'wx', mode: 0o700 }); created = true;
    verifyEngine(temporary, manifest, target, env);
    await retryStagedFile(() => fs.renameSync(temporary, destination));
    verifyEngine(destination, manifest, target, env);
  } catch (error) { installError = error; throw error; }
  finally {
    if (created && fs.existsSync(temporary)) {
      try { await retryStagedFile(() => fs.unlinkSync(temporary)); }
      catch (error) { if (!installError) throw error; installError.message += ` Staging cleanup failed: ${error.message}`; }
    }
  }
  return destination;
}

function projectRoot(start) {
  let initial;
  try { initial = fs.realpathSync(path.resolve(start)); }
  catch (error) {
    if (error.code === 'ENOENT' || error.code === 'ENOTDIR') throw failure(`Project is not an existing directory: ${start}`, 2);
    throw error;
  }
  let directory = initial;
  if (!fs.statSync(initial).isDirectory()) throw failure(`Project is not a directory: ${start}`, 2);
  for (;;) {
    if (exists(path.join(directory, '.phoenix-diagram/install/active.json')) || exists(path.join(directory, '.git'))) return directory;
    const parent = path.dirname(directory);
    if (parent === directory) return initial;
    directory = parent;
  }
}
function physical(stat) { return { device: stat.dev.toString(), inode: stat.ino.toString() }; }
function identity(snapshot) {
  const { stat, bytes } = snapshot;
  return { ...physical(stat), links: Number(stat.nlink), mode: Number(stat.mode & 0o777n), uid: Number(stat.uid), sha256: sha256(bytes), bytes: bytes.length };
}
function same(left, right) { return JSON.stringify(left) === JSON.stringify(right); }
function sealed(file) {
  const snapshot = readRegular(file, MAX_ENGINE, 0o600);
  const data = JSON.parse(snapshot.bytes);
  if (!object(data) || !SHA.test(data.checksum || '') || sha256(encoded({ ...data, checksum: '' })) !== data.checksum || !snapshot.bytes.equals(encoded(data))) throw failure(`Invalid canonical installation ownership/journal checksum: ${file}`);
  if (snapshot.stat.uid !== BigInt(process.getuid())) throw failure(`Installation state belongs to a different owner: ${file}`);
  return { ...snapshot, data };
}
function managed(name) {
  if (!relative(name)) return false;
  const [provider, skills, tool, ...rest] = name.split('/');
  return PROVIDERS.includes(provider) && provider !== '.codex' && skills === 'skills' && tool === TOOL && allowed(rest.join('/'), true);
}
function verifiedJournalPublicationEntry(journals, name) {
  const parent = fs.lstatSync(journals, { bigint: true });
  if (!parent.isDirectory() || parent.isSymbolicLink() || parent.uid !== BigInt(process.getuid()) || (parent.mode & 0o7777n) !== 0o700n) throw failure('Installation journal directory is not private.');
  if (name === '.phoenix-diagram-output-cleanup') {
    const directory = childFile(journals, name), stat = fs.lstatSync(directory, { bigint: true });
    if (!stat.isDirectory() || stat.isSymbolicLink() || stat.uid !== parent.uid || (stat.mode & 0o7777n) !== 0o700n) throw failure('Unknown journal publication cleanup directory.');
    const marker = readRegular(childFile(directory, 'owner-v1.json'), 16 * 1024, 0o600);
    const owner = JSON.parse(marker.bytes);
    const expected = { schemaVersion: 1, tool: TOOL, kind: 'output-cleanup-directory', parentPath: journals, parent: physical(parent), directory: physical(stat), recordIdentity: physical(marker.stat), checksum: '' };
    expected.checksum = sha256(Buffer.from(JSON.stringify(expected)));
    if (marker.stat.uid !== parent.uid || !marker.bytes.equals(encoded(expected)) || !same(owner, expected)) throw failure('Journal publication cleanup owner binding or checksum differs.');
    if (fs.readdirSync(directory).some(entry => entry !== 'owner-v1.json')) throw failure('Journal publication cleanup requires explicit native recovery; unknown records are preserved.');
    return true;
  }
  if (name.startsWith('.phoenix-diagram-lock-')) {
    const marker = readRegular(childFile(journals, name), 16 * 1024, 0o600), lock = JSON.parse(marker.bytes);
    if (!/^[a-f0-9]{32}\.json$/.test(lock.name || '') || name !== `.phoenix-diagram-lock-${sha256(lock.name)}.lock` || !exists(childFile(journals, lock.name))) throw failure('Unknown journal publication lock target or filename.');
    const expected = { schemaVersion: 1, tool: TOOL, kind: 'output-lock', name: lock.name, parentDevice: parent.dev.toString(), parentInode: parent.ino.toString() };
    if (marker.stat.uid !== parent.uid || !marker.bytes.equals(Buffer.from(JSON.stringify(expected)))) throw failure('Journal publication lock identity or bytes differ.');
    return true;
  }
  return false;
}
function verifyReceipt(project, manifest, options = {}) {
  const receiptPath = '.phoenix-diagram/install/active.json';
  const file = path.join(project, receiptPath);
  if (!exists(file)) {
    if (options.installedSkillRoot) throw failure('Installed skill has no diagram ownership receipt.');
    return null;
  }
  const snapshot = sealed(childFile(project, receiptPath));
  const active = snapshot.data; const version = manifest.data;
  const transaction = active.transactionId;
  if (active.schemaVersion !== 1 || active.tool !== TOOL || active.toolVersion !== TOOL_VERSION || !/^[a-f0-9]{32}$/.test(transaction || '') || !same(active.project, physical(fs.statSync(project, { bigint: true })))
    || active.npmPackageVersion !== version.npmPackageVersion || active.bundleManifestSha256 !== version.bundleSha256 || active.engineManifestSha256 !== manifest.hash
    || active.engine?.sourceInputSha256 !== version.sourceInputSha256 || active.engine?.inputContractSchema !== 1 || active.engine?.receiptSchema !== 1 || active.engine?.viewerRevision !== 'phoenix-v1'
    || !Object.values(version.engines).some(meta => meta.sha256 === active.engine?.binary?.sha256 && meta.size === active.engine?.binary?.bytes)
    || !Array.isArray(active.providers) || !active.providers.length || new Set(active.providers).size !== active.providers.length || active.providers.some(provider => !PROVIDERS.includes(provider))
    || active.journal !== `.phoenix-diagram/install/journals/${transaction}.json` || active.backup !== `.phoenix-diagram/install/transactions/${transaction}/before`
    || !Array.isArray(active.ownership) || !active.ownership.length || active.ownership.length > 200000
    || active.planSha256 !== sha256(encoded([active.bundleManifestSha256, active.engineManifestSha256, active.providers, active.ownership]))) throw failure('Project ownership requires a different or invalid Phoenix Diagram package/contract identity.');
  const journal = sealed(childFile(project, active.journal)).data;
  if (journal.schemaVersion !== 1 || journal.tool !== TOOL || journal.transactionId !== transaction || journal.phase !== 'committed' || journal.pending !== null || !Array.isArray(journal.operations) || !journal.operations.length || journal.completed !== journal.operations.length
    || !same(journal.project, active.project) || !same(journal.active, active)
    || !same(journal.transaction, physical(fs.statSync(path.dirname(path.join(project, active.backup)), { bigint: true })))
    || journal.operations[journal.operations.length - 1].path !== receiptPath || !same(journal.operations[journal.operations.length - 1].after, identity(snapshot))) throw failure('Installed ownership is not bound to a completed native journal.');
  let previous; const owned = new Map();
  for (const entry of active.ownership) {
    if (!managed(entry.path) || !active.providers.includes(entry.path.split('/')[0]) || previous && Buffer.compare(Buffer.from(previous), Buffer.from(entry.path)) >= 0 || ![0o644, 0o755].includes(entry.identity?.mode)) throw failure(`Unsafe managed ownership path: ${entry.path}`);
    const actual = readRegular(childFile(project, entry.path), MAX_ENGINE, entry.identity.mode);
    if (!same(identity(actual), entry.identity)) throw failure(`Managed ownership identity or digest changed: ${entry.path}`);
    owned.set(entry.path, entry.identity); previous = entry.path;
  }
  for (const [index, operation] of journal.operations.entries()) {
    if (operation.stagePath !== `after/${index}` || operation.retiredPath !== `retired/${index}` || operation.undoPath !== `undo/${index}` || operation.backupPath !== (operation.before ? `before/${index}` : null)) throw failure('Journal staging/backup paths differ from native ownership.');
    if (index + 1 < journal.operations.length && (!managed(operation.path) || !same(owned.get(operation.path), operation.after))) throw failure(`Journal operation differs from managed ownership: ${operation.path}`);
  }
  const journals = path.join(project, '.phoenix-diagram/install/journals');
  for (const name of fs.readdirSync(journals)) {
    if (verifiedJournalPublicationEntry(journals, name)) continue;
    if (!/^[a-f0-9]{32}\.json$/.test(name)) throw failure(`Unknown installation journal entry: ${name}`);
    const other = sealed(childFile(journals, name)).data;
    if (!['committed', 'recovered'].includes(other.phase)) throw failure('Diagram installation requires explicit native recovery before ordinary use.');
  }
  if (options.installedSkillRoot) {
    const relativeRoot = path.relative(project, options.installedSkillRoot).split(path.sep).join('/');
    if (!owned.has(`${relativeRoot}/ENGINE.json`) || !owned.has(`${relativeRoot}/SKILL.md`)) throw failure('Launcher skill root is not managed by this project ownership record.');
    const engine = readRegular(childFile(options.installedSkillRoot, 'ENGINE.json'), 1024 * 1024).bytes;
    if (!engine.equals(manifest.bytes)) throw failure('Installed ENGINE bytes differ from their native ownership identity.');
    inventory(options.installedSkillRoot, new Set([...owned.keys()].filter(name => name.startsWith(`${relativeRoot}/`)).map(name => name.slice(relativeRoot.length + 1))));
  }
  return active;
}

function exists(file) {
  try { fs.lstatSync(file); return true; } catch (error) { if (error.code === 'ENOENT') return false; throw error; }
}
async function preflight(options = {}) {
  const env = { ...process.env, ...options.env };
  const skillDir = path.resolve(options.skillDir || path.join(__dirname, '..'));
  const versionManifestPath = path.resolve(options.manifestPath || path.join(skillDir, 'ENGINE.json'));
  const manifest = manifestAt(versionManifestPath, options);
  const target = platformTarget();
  let project = projectRoot(options.project || process.cwd());
  let bundleRoot;
  if (options.bundleRoot) {
    bundleRoot = path.resolve(options.bundleRoot);
    verifyBundle(bundleRoot, manifest);
    legalResources(path.dirname(versionManifestPath));
    if (options.releaseDir && options.allowInstall !== true) verifyOffline(path.resolve(options.releaseDir), manifest, verifyBundle(bundleRoot, manifest));
    // An explicit package upgrade lets native --dry-run validate the prior
    // ownership. Ordinary use never opts out of the current receipt binding.
    if (options.checkReceipt !== false) verifyReceipt(project, manifest);
  } else {
    if (!options.project && !exists(path.join(project, '.phoenix-diagram/install/active.json'))) project = projectRoot(skillDir);
    verifyReceipt(project, manifest, { installedSkillRoot: skillDir });
    bundleRoot = skillDir;
    if (options.releaseDir && options.allowInstall !== true) verifyOffline(path.resolve(options.releaseDir), manifest, null);
  }
  const binary = options.allowInstall === true
    ? await installEngine(manifest, target, project, options.releaseDir && path.resolve(options.releaseDir), env)
    : verifyEngine(enginePath(manifest, target, env), manifest, target, env);
  return { enginePath: binary, bundleRoot, versionManifestPath, version: manifest.data, manifest, target, project, skillDir };
}
function shellQuote(value) { return `'${String(value).replaceAll("'", "'\"'\"'")}'`; }
function setupCommand(manifest, project, options = {}) {
  if (!manifest) {
    try { manifest = manifestAt(options.manifestPath || path.join(options.skillDir || path.join(__dirname, '..'), 'ENGINE.json'), options); }
    catch { /* Unknown distribution must not imply a registry release. */ }
  }
  if (manifest?.data.distribution === 'release') return `npm exec --package=@xgent-ai/skills@${manifest.data.npmPackageVersion} -- phoenix-diagram engine install --project ${shellQuote(project)}`;
  const launcher = options.packageJsonPath
    ? path.join(path.dirname(path.resolve(options.packageJsonPath)), 'bin/phoenix-diagram.js')
    : path.join(options.skillDir || path.join(__dirname, '..'), 'scripts/phoenix-launcher.cjs');
  return `${shellQuote(process.execPath)} ${shellQuote(path.resolve(launcher))} engine install --project ${shellQuote(project)} --release-dir ${shellQuote(options.releaseDir || 'MATCHING_RELEASE_DIR')}`;
}
function help(options, args = []) {
  let version;
  try { version = packageVersion(options); } catch { /* Help is available even when package metadata is damaged. */ }
  return `phoenix-diagram ${TOOL_VERSION}${version ? ` (@xgent-ai/skills ${version})` : ''}\n`
    + 'Native diagrams: guide, validate, deliver, check, browser-check, visual-check, finalize, review-record, recover-output, doctor, install.\n'
    + 'Usage: phoenix-diagram engine install --project DIR [--release-dir DIR] [--json]\n'
    + 'Engine acquisition is explicit. Ordinary commands only verify an existing engine and never download or use a Node renderer.\n'
    + (args[0] === 'recover-output' ? 'Recovery: recover-output <original-private-recovery-directory> --json\nIn the original output parent P, records move from P/.phoenix-diagram-recovery-<id>/publication-recovery-v1.json to sibling P/.phoenix-diagram-recovery-<id>.cleanup.json, then P/.phoenix-diagram-output-cleanup/<id>.json. The fixed cleanup directory includes its owned owner-v1.json. Always pass the original recovery directory explicitly, even if it has disappeared. Unknown or ambiguous multiple record locations are preserved and refused; recovery does not scan to choose a transaction. Native recovery preserves later claimants; it does not replace delivery checks or image review.\n' : '')
    + `${setupCommand(null, options.project || process.cwd(), options)}\n`;
}
async function main(args, options = {}) {
  if (!Array.isArray(args) || args.some(arg => typeof arg !== 'string')) throw failure('CLI arguments must be a string array.', 2);
  const wantsHelp = args.includes('--help') || args.includes('-h');
  if (!args.length || wantsHelp && args[0] === 'engine') { process.stdout.write(help(options, args)); return 0; }
  if (args.length === 1 && ['--version', '-V', 'version'].includes(args[0])) { process.stdout.write(help(options).split('\n')[0] + '\n'); return 0; }
  const env = { ...process.env, ...options.env };
  const json = args.includes('--json');
  let manifest; let target; let project = options.project || process.cwd();
  try {
    if (args[0] === 'engine' && args[1] !== 'install') throw failure('Usage: phoenix-diagram engine install --project DIR [--release-dir DIR] [--json]', 2);
    if (args[0] === 'engine') {
      let releaseDir; let requested;
      for (let i = 2; i < args.length; i++) {
        if (args[i] === '--json') continue;
        if (!['--project', '--release-dir'].includes(args[i]) || !args[i + 1] || args[i + 1].startsWith('--')) throw failure('Usage: phoenix-diagram engine install --project DIR [--release-dir DIR] [--json]', 2);
        const flag = args[i++];
        if (flag === '--project') { if (requested) throw failure('Duplicate --project.', 2); requested = args[i]; }
        else { if (releaseDir) throw failure('Duplicate --release-dir.', 2); releaseDir = path.resolve(args[i]); }
      }
      if (!requested) throw failure('Engine installation requires an explicit --project DIR.', 2);
      project = projectRoot(requested);
      const ready = await preflight({ ...options, project, releaseDir, env, allowInstall: true, checkReceipt: true });
      manifest = ready.manifest; target = ready.target;
      const result = { schemaVersion: 1, tool: TOOL, toolVersion: TOOL_VERSION, command: 'engine install', status: 'installed', scope: 'engine-cache', diagnostics: [], npmPackageVersion: ready.version.npmPackageVersion, target, enginePath: ready.enginePath, project: ready.project };
      process.stdout.write(json ? encoded(result) : `Phoenix Diagram engine ${TOOL_VERSION} ready: ${ready.enginePath}\n`);
      return 0;
    }
    const ready = await preflight({ ...options, env, allowInstall: false, checkReceipt: args[0] === 'install' ? false : true });
    manifest = ready.manifest; target = ready.target; project = ready.project;
    const childArgs = [...args];
    if (args[0] === 'install' && !args.some(arg => arg === '--bundle-root' || arg.startsWith('--bundle-root='))) childArgs.push('--bundle-root', ready.bundleRoot);
    const childEnv = { ...env, PHOENIX_DIAGRAM_BUNDLE_ROOT: ready.bundleRoot, PHOENIX_DIAGRAM_SETUP_COMMAND: setupCommand(manifest, project, options) };
    const code = await new Promise((resolve, reject) => {
      const child = spawn(ready.enginePath, childArgs, { env: childEnv, stdio: 'inherit', windowsHide: true });
      const interrupt = () => child.kill('SIGINT'); const terminate = () => child.kill('SIGTERM');
      process.on('SIGINT', interrupt); process.on('SIGTERM', terminate);
      child.once('error', reject);
      child.once('close', (code, signal) => {
        process.removeListener('SIGINT', interrupt); process.removeListener('SIGTERM', terminate);
        resolve(code ?? (signal === 'SIGINT' ? 130 : 143));
      });
    });
    if (wantsHelp && args[0].startsWith('-') && code === 0) process.stdout.write(`\nBootstrap: phoenix-diagram engine install --project DIR [--release-dir DIR] --json\n${setupCommand(manifest, project, options)}\n`);
    return code;
  } catch (error) {
    if (wantsHelp) { process.stdout.write(help(options, args)); return 0; }
    try { manifest ||= manifestAt(options.manifestPath || path.join(options.skillDir || path.join(__dirname, '..'), 'ENGINE.json'), options); } catch { /* Recovery never trusts malformed metadata. */ }
    const recovery = setupCommand(manifest, project, options);
    process.stderr.write(`Phoenix Diagram: ${error.message}\n${recovery}\n`);
    if (json) process.stdout.write(encoded({ schemaVersion: 1, tool: TOOL, toolVersion: TOOL_VERSION, command: args.slice(0, args[0] === 'engine' ? 2 : 1).join(' '), status: 'unavailable', diagnostics: [{ stage: 'bootstrap', code: 'bootstrap/unavailable', severity: 'error', message: error.message }], target, setupCommand: recovery }));
    return error.code === 2 || error.code === 3 ? error.code : 4;
  }
}

module.exports = { TARGETS, platformTarget, manifestAt, verifyBundle, verifyReceipt, verifyEngine, enginePath, installEngine, preflight, setupCommand, shellQuote, proxyFor, checkedUrl, sha256, main };
if (require.main === module) main(process.argv.slice(2)).then(code => { process.exitCode = code; }).catch(error => { process.stderr.write(`Phoenix Diagram: ${error.message}\n`); process.exitCode = 4; });
