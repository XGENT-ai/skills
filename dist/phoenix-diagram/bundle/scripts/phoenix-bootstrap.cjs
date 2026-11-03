'use strict';
// SPDX-License-Identifier: Apache-2.0
// Adapted from Phoenix UI's phoenix-bootstrap.cjs, engine 0.1.0, repository
// 4a65a1525f9ed03c775a0e18935901e1edbb773f, SHA-256
// 8b1723fb51e6aad9bfc0bc5ba8e09b5a373b0e33c251331f6ca14ff6b1c261ef.
// Its Apache-2.0 Impeccable provenance is retained in tools/phoenix-ui/NOTICE.md.
// Phoenix changes: diagram-only identities and cache, Unix targets and
// explicit engine acquisition. No hooks, renderer, installer or update lookup.

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
const TOOL_VERSION = '0.2.0';
const TARGETS = ['darwin-arm64', 'darwin-x64', 'linux-x64', 'linux-arm64'];
const PROBE = `phoenix-diagram-engine ${TOOL_VERSION} input=1 receipt=1 viewer=phoenix-v2\n`;
const MAX_ENGINE = 256 * 1024 * 1024;
const USAGE = 'Usage: phoenix-diagram engine install --project DIR [--release-dir DIR] [--json]';
const sha256 = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const failure = (message, code = 4) => Object.assign(new Error(message), { code });
const encoded = value => Buffer.from(JSON.stringify(value) + '\n');

function platformTarget(platform = process.platform, arch = process.arch) {
  const target = `${platform}-${arch}`;
  if (!TARGETS.includes(target)) throw failure(`Unsupported Phoenix Diagram platform ${platform}/${arch}; the engine supports the four declared macOS/Linux targets.`);
  return target;
}
function readFile(file, limit) {
  if (fs.statSync(file).size > limit) throw failure(`File exceeds its size bound: ${file}`);
  return fs.readFileSync(file);
}
function manifestAt(file) {
  const bytes = readFile(path.resolve(file), 1024 * 1024);
  const data = JSON.parse(bytes);
  if (data.tool !== TOOL || data.toolVersion !== TOOL_VERSION || !data.engines) throw failure(`Phoenix Diagram manifest names another tool or version: ${file}`);
  return { data, bytes };
}
function manifestPath(options) {
  return options.manifestPath || path.join(options.skillDir || path.join(__dirname, '..'), 'ENGINE.json');
}
function cacheRoot(env = process.env) { return path.resolve(env.PHOENIX_DIAGRAM_HOME || path.join(os.homedir(), '.phoenix-diagram')); }
function enginePath(manifest, target, env = process.env) {
  const meta = manifest.data.engines[target];
  if (!meta) throw failure(`This package has no engine for ${target}.`);
  return path.join(cacheRoot(env), 'bin', manifest.data.toolVersion, target, meta.sha256, TOOL);
}
function verifyEngine(file, manifest, target, env = process.env) {
  const meta = manifest.data.engines[target];
  const bytes = readFile(file, MAX_ENGINE);
  if (bytes.length !== meta.size || sha256(bytes) !== meta.sha256) throw failure('Phoenix Diagram binary digest or size differs from VERSION.');
  const probe = spawnSync(file, ['engine-probe'], { env, encoding: 'utf8', timeout: 3000, maxBuffer: 4096 });
  if (probe.error || probe.status !== 0 || probe.stdout !== PROBE) throw failure(`Phoenix Diagram engine probe identity differs: ${JSON.stringify({ error: probe.error?.code || null, status: probe.status, signal: probe.signal, stdout: probe.stdout, stderr: probe.stderr })}`);
  return file;
}

// Transport copied from Phoenix UI's verified bootstrap. Engine URLs and every
// redirect must stay on HTTPS.
function proxyFor(url, env) {
  const noProxy = env.no_proxy || env.NO_PROXY || '';
  const hostname = url.hostname.replace(/^\[|\]$/g, '').toLowerCase();
  if (noProxy.split(',').some(value => {
    const item = value.trim().toLowerCase();
    const [host, port] = item.split(':');
    return item === '*' || ((!port || port === (url.port || '443'))
      && (hostname === host || (host?.startsWith('.') && hostname.endsWith(host))));
  })) return null;
  const value = env.https_proxy || env.HTTPS_PROXY || env.all_proxy || env.ALL_PROXY || env.http_proxy || env.HTTP_PROXY;
  if (!value) return null;
  const proxy = new URL(value);
  if (!['http:', 'https:'].includes(proxy.protocol)) throw failure('Phoenix Diagram supports HTTP or HTTPS proxies.');
  return proxy;
}
async function responseFor(url, env) {
  const proxy = proxyFor(url, env);
  let connection;
  if (proxy) {
    const authorization = proxy.username ? `Basic ${Buffer.from(`${decodeURIComponent(proxy.username)}:${decodeURIComponent(proxy.password)}`).toString('base64')}` : null;
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
    const request = https.get(url, options, resolve);
    request.setTimeout(60000, () => request.destroy(failure('Engine download timed out.')));
    request.once('error', reject);
  });
}
function httpsUrl(value) {
  const url = new URL(value);
  if (url.protocol !== 'https:') throw failure('Engine download requires HTTPS and refuses downgrade redirects.');
  return url;
}
async function download(meta, env) {
  if (!meta.url) throw failure('This engine has no download URL; supply a matching --release-dir.');
  let url = httpsUrl(meta.url);
  for (let redirects = 0; redirects <= 5; redirects++) {
    const res = await responseFor(url, env);
    if ([301, 302, 303, 307, 308].includes(res.statusCode)) {
      res.resume();
      if (!res.headers.location) throw failure('Engine download redirect has no location.');
      url = httpsUrl(new URL(res.headers.location, url).href); continue;
    }
    if (res.statusCode !== 200) { res.resume(); throw failure(`Engine download HTTP ${res.statusCode}.`); }
    const chunks = []; let length = 0;
    for await (const chunk of res) {
      length += chunk.length;
      if (length > meta.size) { res.destroy(); throw failure('Engine download exceeded the pinned size.'); }
      chunks.push(chunk);
    }
    return Buffer.concat(chunks);
  }
  throw failure('Engine redirect limit exceeded.');
}
async function installEngine(manifest, target, releaseDir, env = process.env) {
  // Validate an explicit offline request before considering an existing cache.
  if (releaseDir && !readFile(path.join(releaseDir, 'VERSION.json'), 1024 * 1024).equals(manifest.bytes)) throw failure('Offline VERSION.json differs from this package.');
  const destination = enginePath(manifest, target, env);
  try { return verifyEngine(destination, manifest, target, env); } catch { /* Explicit acquisition replaces a missing or bad cache. */ }
  const meta = manifest.data.engines[target];
  const bytes = releaseDir ? readFile(path.join(releaseDir, meta.asset), MAX_ENGINE) : await download(meta, env);
  if (bytes.length !== meta.size || sha256(bytes) !== meta.sha256) throw failure('Engine digest/size differs from the pinned VERSION manifest.');
  fs.mkdirSync(path.dirname(destination), { recursive: true, mode: 0o700 });
  const temporary = `${destination}.${process.pid}.part`;
  try {
    fs.writeFileSync(temporary, bytes, { mode: 0o700 });
    verifyEngine(temporary, manifest, target, env);
    fs.renameSync(temporary, destination);
  } finally { fs.rmSync(temporary, { force: true }); }
  return destination;
}

function projectRoot(start) {
  const initial = path.resolve(start);
  if (!fs.statSync(initial, { throwIfNoEntry: false })?.isDirectory()) throw failure(`Project is not an existing directory: ${start}`, 2);
  for (let directory = initial; ; directory = path.dirname(directory)) {
    if (fs.existsSync(path.join(directory, '.git'))) return directory;
    if (path.dirname(directory) === directory) return initial;
  }
}
async function preflight(options = {}) {
  const env = { ...process.env, ...options.env };
  const skillDir = path.resolve(options.skillDir || path.join(__dirname, '..'));
  const manifest = manifestAt(manifestPath(options));
  const target = platformTarget();
  const project = projectRoot(options.project || process.cwd());
  const binary = options.allowInstall === true
    ? await installEngine(manifest, target, options.releaseDir && path.resolve(options.releaseDir), env)
    : verifyEngine(enginePath(manifest, target, env), manifest, target, env);
  return { enginePath: binary, bundleRoot: path.resolve(options.bundleRoot || skillDir), version: manifest.data, manifest, target, project, skillDir };
}
function shellQuote(value) { return `'${String(value).replaceAll("'", "'\"'\"'")}'`; }
function setupCommand(manifest, project, options = {}) {
  if (manifest?.data.distribution === 'release') return `npm exec --package=@xgent-ai/skills@${manifest.data.npmPackageVersion} -- phoenix-diagram engine install --project ${shellQuote(project)}`;
  const launcher = options.packageJsonPath
    ? path.join(path.dirname(path.resolve(options.packageJsonPath)), 'bin/phoenix-diagram.js')
    : path.join(options.skillDir || path.join(__dirname, '..'), 'scripts/phoenix-launcher.cjs');
  return `${shellQuote(process.execPath)} ${shellQuote(path.resolve(launcher))} engine install --project ${shellQuote(project)} --release-dir ${shellQuote(options.releaseDir || 'MATCHING_RELEASE_DIR')}`;
}
function knownManifest(options) {
  try { return manifestAt(manifestPath(options)); } catch { return undefined; }
}
function help(options) {
  const manifest = knownManifest(options);
  return `phoenix-diagram ${TOOL_VERSION}${manifest ? ` (@xgent-ai/skills ${manifest.data.npmPackageVersion})` : ''}\n`
    + 'Native diagrams: guide, brands, validate, deliver, compare, preview, check, browser-check, visual-check, finalize, review-record, doctor, install.\n'
    + `${USAGE}\n`
    + 'Engine acquisition is explicit. Ordinary commands only verify an existing engine and never download or use a Node renderer.\n'
    + `${setupCommand(manifest, options.project || process.cwd(), options)}\n`;
}

async function main(args, options = {}) {
  const wantsHelp = args.includes('--help') || args.includes('-h');
  if (!args.length || wantsHelp && args[0] === 'engine') { process.stdout.write(help(options)); return 0; }
  if (args.length === 1 && ['--version', '-V', 'version'].includes(args[0])) { process.stdout.write(help(options).split('\n')[0] + '\n'); return 0; }
  const env = { ...process.env, ...options.env };
  const json = args.includes('--json');
  let manifest; let target; let project = options.project || process.cwd();
  try {
    if (args[0] === 'engine') {
      if (args[1] !== 'install') throw failure(USAGE, 2);
      let releaseDir; let requested;
      for (let i = 2; i < args.length; i++) {
        if (args[i] === '--json') continue;
        if (!['--project', '--release-dir'].includes(args[i]) || !args[i + 1] || args[i + 1].startsWith('--')) throw failure(USAGE, 2);
        const flag = args[i++];
        if (flag === '--project') { if (requested) throw failure('Duplicate --project.', 2); requested = args[i]; }
        else { if (releaseDir) throw failure('Duplicate --release-dir.', 2); releaseDir = args[i]; }
      }
      if (!requested) throw failure('Engine installation requires an explicit --project DIR.', 2);
      project = requested;
      const ready = await preflight({ ...options, project, releaseDir, env, allowInstall: true });
      manifest = ready.manifest; target = ready.target;
      const result = { schemaVersion: 1, tool: TOOL, toolVersion: TOOL_VERSION, command: 'engine install', status: 'installed', scope: 'engine-cache', diagnostics: [], npmPackageVersion: ready.version.npmPackageVersion, target, enginePath: ready.enginePath, project: ready.project };
      process.stdout.write(json ? encoded(result) : `Phoenix Diagram engine ${TOOL_VERSION} ready: ${ready.enginePath}\n`);
      return 0;
    }
    const ready = await preflight({ ...options, env });
    manifest = ready.manifest; target = ready.target; project = ready.project;
    const childArgs = args[0] === 'install' && !args.some(arg => arg === '--bundle-root' || arg.startsWith('--bundle-root=')) ? [...args, '--bundle-root', ready.bundleRoot] : args;
    const childEnv = { ...env, PHOENIX_DIAGRAM_BUNDLE_ROOT: ready.bundleRoot, PHOENIX_DIAGRAM_SETUP_COMMAND: setupCommand(manifest, project, options) };
    const code = await new Promise((resolve, reject) => {
      const child = spawn(ready.enginePath, childArgs, { env: childEnv, stdio: 'inherit' });
      const interrupt = () => child.kill('SIGINT'); const terminate = () => child.kill('SIGTERM');
      process.on('SIGINT', interrupt); process.on('SIGTERM', terminate);
      child.once('error', reject);
      child.once('close', (code, signal) => {
        process.removeListener('SIGINT', interrupt); process.removeListener('SIGTERM', terminate);
        resolve(code ?? (signal === 'SIGINT' ? 130 : 143));
      });
    });
    if (wantsHelp && args[0].startsWith('-') && code === 0) process.stdout.write(`\nBootstrap: ${USAGE}\n${setupCommand(manifest, project, options)}\n`);
    return code;
  } catch (error) {
    if (wantsHelp) { process.stdout.write(help(options)); return 0; }
    manifest ||= knownManifest(options);
    const recovery = setupCommand(manifest, project, options);
    process.stderr.write(`Phoenix Diagram: ${error.message}\n${recovery}\n`);
    if (json) process.stdout.write(encoded({ schemaVersion: 1, tool: TOOL, toolVersion: TOOL_VERSION, command: args.slice(0, args[0] === 'engine' ? 2 : 1).join(' '), status: 'unavailable', diagnostics: [{ stage: 'bootstrap', code: 'bootstrap/unavailable', severity: 'error', message: error.message }], target, setupCommand: recovery }));
    return error.code === 2 || error.code === 3 ? error.code : 4;
  }
}

module.exports = { TARGETS, platformTarget, manifestAt, enginePath, preflight, setupCommand, shellQuote, proxyFor, main };
if (require.main === module) main(process.argv.slice(2)).then(code => { process.exitCode = code; }).catch(error => { process.stderr.write(`Phoenix Diagram: ${error.message}\n`); process.exitCode = 4; });
