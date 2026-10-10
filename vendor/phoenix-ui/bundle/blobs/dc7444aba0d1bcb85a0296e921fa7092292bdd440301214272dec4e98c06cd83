'use strict';

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const crypto = require('node:crypto');
const http = require('node:http');
const https = require('node:https');
const tls = require('node:tls');
const { spawn, spawnSync } = require('node:child_process');

const TARGETS = ['darwin-arm64', 'darwin-x64', 'linux-x64', 'linux-arm64', 'windows-x64'];
const HOOKS = new Set(['hook', 'hook-before-edit']);
const sha256 = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const failure = (message, code = 4) => Object.assign(new Error(message), { code });

function platformTarget(platform = process.platform, arch = process.arch) {
  if (platform === 'win32' && ['x64', 'arm64'].includes(arch)) return 'windows-x64';
  const target = `${platform}-${arch}`;
  if (!TARGETS.includes(target)) throw failure(`Unsupported Phoenix platform ${platform}/${arch}; use xgent-skills install --no-phoenix-ui for other features.`);
  return target;
}

function manifestAt(file) {
  const bytes = fs.readFileSync(file);
  const data = JSON.parse(bytes);
  if (data.schemaVersion !== 1 || data.bundleSchema !== 1 || data.reviewSchema !== 1
      || !/^\d+\.\d+\.\d+(?:-[a-zA-Z0-9.-]+)?$/.test(data.toolVersion || '')
      || !/^\d+\.\d+\.\d+(?:-[a-zA-Z0-9.-]+)?$/.test(data.npmPackageVersion || '')
      || !/^[a-f0-9]{40}$/.test(data.sourceCommit || '') || !/^[a-f0-9]{64}$/.test(data.bundleSha256 || '')) {
    throw failure('Invalid Phoenix package manifest. Reinstall the matching npm package.');
  }
  const targets = Object.keys(data.engines || {});
  if (!targets.length || targets.some(target => !TARGETS.includes(target))) throw failure('Invalid engine target manifest.');
  if (data.distribution !== 'development' && TARGETS.some(target => !targets.includes(target))) throw failure('Release manifest is missing a supported platform.');
  for (const [target, engine] of Object.entries(data.engines)) {
    const asset = `phoenix-ui-${target}${target.startsWith('windows-') ? '.exe' : ''}`;
    if (engine.asset !== asset || !/^[a-f0-9]{64}$/.test(engine.sha256 || '')
        || !Number.isSafeInteger(engine.size) || engine.size <= 0 || engine.size > 256 * 1024 * 1024) {
      throw failure(`Invalid engine metadata for ${target}.`);
    }
    if (data.distribution !== 'development' && !engine.url) throw failure(`Release engine URL is missing for ${target}.`);
    if (engine.url) checkedUrl(engine.url);
  }
  return { data, hash: sha256(bytes), bytes };
}

function checkedUrl(value) {
  const url = new URL(value);
  if (url.username || url.password || (url.protocol !== 'https:'
      && !(url.protocol === 'http:' && ['127.0.0.1', '[::1]', 'localhost'].includes(url.hostname)))) {
    throw failure('Engine download requires HTTPS (loopback HTTP is allowed for local verification).');
  }
  return url;
}

function cacheRoot(env = process.env) {
  return path.resolve(env.PHOENIX_UI_HOME || path.join(os.homedir(), '.phoenix-ui'));
}

function enginePath(manifest, target, env = process.env) {
  return path.join(cacheRoot(env), 'bin', manifest.data.toolVersion, target, target.startsWith('windows-') ? 'phoenix-ui.exe' : 'phoenix-ui');
}

function projectRoot(start) {
  const initial = fs.realpathSync(path.resolve(start));
  let dir = initial;
  for (;;) {
    if (fs.existsSync(path.join(dir, '.phoenix-ui/install/receipt.json')) || fs.existsSync(path.join(dir, '.git'))) return dir;
    const parent = path.dirname(dir);
    if (parent === dir) return initial;
    dir = parent;
  }
}

function verifyReceipt(project, manifest, target) {
  const file = path.join(project, '.phoenix-ui/install/receipt.json');
  if (!fs.existsSync(file)) return;
  const receipt = JSON.parse(fs.readFileSync(file, 'utf8'));
  if (receipt.schemaVersion !== 1 || receipt.manifestSha256 !== manifest.hash
      || receipt.toolVersion !== manifest.data.toolVersion || receipt.npmPackageVersion !== manifest.data.npmPackageVersion
      || receipt.engine?.target !== target || receipt.engine?.sha256 !== manifest.data.engines[target]?.sha256) {
    throw failure(`Project receipt requires a different Phoenix package. Run @xgent-ai/skills@${receipt.npmPackageVersion || manifest.data.npmPackageVersion} for this project.`);
  }
}

function verifyEngine(file, manifest, target, env = process.env) {
  const meta = manifest.data.engines[target];
  if (!meta) throw failure(`This development package has no verified engine for ${target}.`);
  const stat = fs.lstatSync(file);
  if (!stat.isFile() || stat.size !== meta.size || sha256(fs.readFileSync(file)) !== meta.sha256) throw failure('Phoenix binary digest or size differs from the package manifest.');
  const probe = spawnSync(file, ['engine-probe'], { env, encoding: 'utf8', timeout: 3000, maxBuffer: 4096, windowsHide: true });
  if (probe.error || probe.status !== 0 || probe.stdout.trim() !== `phoenix-ui-engine ${manifest.data.toolVersion}`) {
    throw failure(`Phoenix engine identity or version differs from the package manifest. Probe: ${JSON.stringify({ error: probe.error?.code || null, status: probe.status, signal: probe.signal, stdout: probe.stdout, stderr: probe.stderr })}`);
  }
  return file;
}

function setupCommand(manifest, project) {
  return `npm exec --package=@xgent-ai/skills@${manifest.data.npmPackageVersion} -- phoenix-ui engine install --project ${shellQuote(project)}`;
}

function shellQuote(value) {
  return process.platform === 'win32' ? `'${value.replaceAll("'", "''")}'` : `'${value.replaceAll("'", "'\"'\"'")}'`;
}

function proxyFor(url, env) {
  const noProxy = env.no_proxy || env.NO_PROXY || '';
  const hostname = url.hostname.replace(/^\[|\]$/g, '');
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
  if (!['http:', 'https:'].includes(proxy.protocol)) throw failure('Phoenix engine install supports HTTP or HTTPS proxies.');
  return proxy;
}

async function responseFor(url, env) {
  const proxy = proxyFor(url, env);
  const authorization = proxy?.username ? `Basic ${Buffer.from(`${decodeURIComponent(proxy.username)}:${decodeURIComponent(proxy.password)}`).toString('base64')}` : null;
  const transport = url.protocol === 'https:' ? https : http;
  let connection;
  if (proxy && url.protocol === 'https:') {
    connection = await new Promise((resolve, reject) => {
      const request = (proxy.protocol === 'https:' ? https : http).request(proxy, {
        method: 'CONNECT', path: `${url.hostname}:${url.port || 443}`,
        headers: authorization ? { 'Proxy-Authorization': authorization } : {},
      });
      request.setTimeout(60000, () => request.destroy(failure('Engine proxy timed out.')));
      request.on('connect', (res, socket, head) => {
        if (res.statusCode !== 200) { socket.destroy(); reject(failure(`Engine proxy HTTP ${res.statusCode}.`)); return; }
        if (head.length) socket.unshift(head);
        const secure = tls.connect({ socket, servername: url.hostname });
        secure.once('secureConnect', () => resolve(secure));
        secure.once('error', reject);
      });
      request.once('error', reject);
      request.end();
    });
  }
  return new Promise((resolve, reject) => {
    const options = connection ? { agent: false, createConnection: () => connection } : {};
    const destination = proxy && url.protocol === 'http:' ? proxy : url;
    const client = proxy && url.protocol === 'http:' ? (proxy.protocol === 'https:' ? https : http) : transport;
    if (proxy && url.protocol === 'http:') Object.assign(options, { path: url.href, headers: { Host: url.host, ...(authorization ? { 'Proxy-Authorization': authorization } : {}) } });
    const request = client.get(destination, options, resolve);
    request.setTimeout(60000, () => request.destroy(failure('Engine download timed out.')));
    request.once('error', reject);
  });
}

async function download(meta, env) {
  let url = checkedUrl(meta.url);
  for (let redirects = 0; redirects <= 5; redirects++) {
    const res = await responseFor(url, env);
    if ([301, 302, 303, 307, 308].includes(res.statusCode)) {
      res.resume();
      if (!res.headers.location || redirects === 5) throw failure('Invalid engine download redirect.');
      const next = checkedUrl(new URL(res.headers.location, url).href);
      if (url.protocol === 'https:' && next.protocol !== 'https:') throw failure('Engine download refused an HTTPS downgrade.');
      url = next;
      continue;
    }
    if (res.statusCode !== 200) { res.resume(); throw failure(`Engine download HTTP ${res.statusCode}.`); }
    const chunks = [];
    let length = 0;
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

function reminderFile(project, version, event, env) {
  const session = event.session_id || event.sessionId || env.CODEX_SESSION_ID || env.CLAUDE_SESSION_ID || '';
  const provider = hookProvider(env, event);
  const key = sha256(JSON.stringify([project, version, provider, session]));
  return path.join(cacheRoot(env), 'unavailable', `${key}.json`);
}

function hookProvider(env, event) {
  const provider = env.PHOENIX_UI_PROVIDER_ID || event.harness || 'source';
  return provider === 'claude-code' ? 'claude' : provider === 'agents' ? 'codex' : provider;
}

function unavailableHook(project, manifest, event, env, message, version = manifest?.data.toolVersion || 'missing-manifest') {
  const marker = reminderFile(project, version, event, env);
  try {
    fs.mkdirSync(path.dirname(marker), { recursive: true });
    fs.writeFileSync(marker, JSON.stringify({ project, version, provider: hookProvider(env, event) }), { flag: 'wx', mode: 0o600 });
  } catch (error) {
    if (error.code === 'EEXIST') return;
    // A read-only cache still permits the edit; it cannot persist deduplication.
  }
  const text = `Phoenix UI unavailable: ${message} ${manifest ? setupCommand(manifest, project) : 'Reinstall the pinned @xgent-ai/skills package.'}`;
  const eventName = [event.hook_event_name, event.hookEventName].find(name => typeof name === 'string' && name) || 'PostToolUse';
  const provider = hookProvider(env, event);
  if (eventName.toLowerCase() === 'stop') { process.stderr.write(`${text}\n`); return; }
  const output = provider === 'cursor' ? { additional_context: text }
    : provider === 'github' ? { additionalContext: text }
      : { hookSpecificOutput: { hookEventName: eventName, additionalContext: text } };
  process.stdout.write(`${JSON.stringify(output)}\n`);
}

function clearReminders(project, manifest, env) {
  const dir = path.join(cacheRoot(env), 'unavailable');
  if (!fs.existsSync(dir)) return;
  for (const name of fs.readdirSync(dir)) {
    const file = path.join(dir, name);
    try {
      const record = JSON.parse(fs.readFileSync(file, 'utf8'));
      if (record.project === project && record.version === manifest.data.toolVersion) fs.unlinkSync(file);
    } catch { /* An unrelated marker does not affect the installed engine. */ }
  }
}

async function retryStagedFile(operation) {
  for (let attempt = 0;; attempt++) {
    try { return operation(); } catch (error) {
      if (!['EBUSY', 'EPERM'].includes(error.code) || attempt === 5) throw error;
      await new Promise(resolve => setTimeout(resolve, 25 * 2 ** attempt));
    }
  }
}

async function installEngine(manifest, target, project, releaseDir, env) {
  verifyReceipt(project, manifest, target);
  const meta = manifest.data.engines[target];
  if (!meta) throw failure(`No ${target} engine in this package manifest.`);
  const dest = enginePath(manifest, target, env);
  try { verifyEngine(dest, manifest, target, env); clearReminders(project, manifest, env); return dest; } catch { /* Explicit install repairs a missing or corrupt cache. */ }
  let bytes;
  if (releaseDir) {
    const release = manifestAt(path.join(releaseDir, 'VERSION.json'));
    if (release.hash !== manifest.hash) throw failure('Offline release manifest differs from this npm package.');
    for (const name of ['THIRD-PARTY.json', 'THIRD-PARTY-NOTICES.md', 'THIRD-PARTY-LICENSES']) {
      if (!fs.existsSync(path.join(releaseDir, name))) throw failure(`Offline release is missing ${name}.`);
    }
    bytes = fs.readFileSync(path.join(releaseDir, meta.asset));
  } else {
    if (!meta.url) throw failure('Development engine has no download URL; use --release-dir with its verified local artifacts.');
    bytes = await download(meta, env);
  }
  if (bytes.length !== meta.size || sha256(bytes) !== meta.sha256) throw failure('Engine digest/size differs from the fixed package manifest.');
  fs.mkdirSync(path.dirname(dest), { recursive: true, mode: 0o700 });
  const temp = `${dest}.${process.pid}.${crypto.randomBytes(6).toString('hex')}.part${process.platform === 'win32' ? '.exe' : ''}`;
  let created = false;
  let installError;
  try {
    fs.writeFileSync(temp, bytes, { flag: 'wx', mode: 0o700 });
    created = true;
    verifyEngine(temp, manifest, target, env);
    await retryStagedFile(() => fs.renameSync(temp, dest));
  } catch (error) {
    installError = error;
    throw error;
  } finally {
    if (created && fs.existsSync(temp)) {
      try { await retryStagedFile(() => fs.unlinkSync(temp)); } catch (error) {
        if (!installError) throw error;
        installError.message += ` Cleanup of the staged engine failed: ${error.message}`;
      }
    }
  }
  clearReminders(project, manifest, env);
  return dest;
}

async function main(args, options = {}) {
  const env = { ...process.env, ...options.env };
  const hook = HOOKS.has(args[0]);
  let input;
  let event = {};
  if (hook) {
    input = fs.readFileSync(0);
    try {
      const parsed = JSON.parse(input.toString('utf8') || '{}');
      if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) event = parsed;
    } catch { /* The native hook owns malformed event handling. */ }
  }
  let manifest;
  let target;
  let project = process.cwd();
  const manifestPath = options.manifestPath || path.join(__dirname, 'ENGINE.json');
  const json = args.includes('--json');
  try {
    manifest = manifestAt(manifestPath);
    project = projectRoot(env.CLAUDE_PROJECT_DIR || event.cwd || project);
    target = platformTarget();
    if (args[0] === 'engine' && args[1] === 'install') {
      let releaseDir;
      for (let i = 2; i < args.length; i++) {
        if (args[i] === '--json') continue;
        if (!['--project', '--release-dir'].includes(args[i]) || !args[i + 1]) throw failure('Usage: phoenix-ui engine install [--project DIR] [--release-dir DIR] [--json]', 2);
        const flag = args[i++];
        if (flag === '--project') {
          const requested = path.resolve(args[i]);
          if (!fs.existsSync(requested) || !fs.statSync(requested).isDirectory()) throw failure(`Project directory does not exist: ${requested}`, 2);
          project = projectRoot(requested);
        }
        else releaseDir = path.resolve(args[i]);
      }
      const binary = await installEngine(manifest, target, project, releaseDir, env);
      process.stdout.write(json ? `${JSON.stringify({ status: 'installed', toolVersion: manifest.data.toolVersion, target, binary })}\n` : `Phoenix UI ${manifest.data.toolVersion} ready: ${binary}\n`);
      return 0;
    }
    verifyReceipt(project, manifest, target);
    const binary = verifyEngine(enginePath(manifest, target, env), manifest, target, env);
    clearReminders(project, manifest, env);
    const childEnv = { ...env, PHOENIX_UI_SELF: options.self || process.argv[1], PHOENIX_UI_SKILL_DIR: options.skillDir || path.resolve(__dirname, '..') };
    const childArgs = [...args];
    const skillsVerb = args[0] === 'skills' ? args[1] : args[0];
    if (options.bundleRoot && ['install', 'update', 'link', 'check'].includes(skillsVerb) && !args.some(a => a === '--bundle-root' || a.startsWith('--bundle-root='))) childArgs.push('--bundle-root', options.bundleRoot);
    return await new Promise((resolve, reject) => {
      const child = spawn(binary, childArgs, { env: childEnv, stdio: hook ? ['pipe', 'inherit', 'inherit'] : 'inherit', windowsHide: true });
      if (hook) child.stdin.end(input);
      const interrupt = () => child.kill('SIGINT');
      const terminate = () => child.kill('SIGTERM');
      process.on('SIGINT', interrupt);
      process.on('SIGTERM', terminate);
      child.once('error', reject);
      child.once('close', (code, signal) => {
        process.removeListener('SIGINT', interrupt);
        process.removeListener('SIGTERM', terminate);
        resolve(code ?? (signal === 'SIGINT' ? 130 : 143));
      });
    });
  } catch (error) {
    if (hook) {
      let version = manifest?.data.toolVersion || 'missing-manifest';
      if (!manifest) {
        try { project = projectRoot(env.CLAUDE_PROJECT_DIR || event.cwd || project); } catch { /* Keep the original cwd when unavailable. */ }
        try {
          // VERSION only identifies the reminder; it never authorizes an engine.
          const pinned = fs.readFileSync(path.join(path.dirname(manifestPath), 'VERSION'), 'utf8').trim();
          if (/^\d+\.\d+\.\d+(?:-[a-zA-Z0-9.-]+)?$/.test(pinned)) version = pinned;
        } catch { /* An incomplete install may lack both metadata files. */ }
      }
      unavailableHook(project, manifest, event, env, error.message, version);
      return 0;
    }
    const command = manifest ? setupCommand(manifest, project) : 'Reinstall the pinned @xgent-ai/skills package.';
    process.stderr.write(`Phoenix UI: ${error.message}\n${command}\n`);
    if (json) process.stdout.write(`${JSON.stringify({ status: 'unavailable', toolVersion: manifest?.data.toolVersion, target, message: error.message, setupCommand: command })}\n`);
    return error.code === 2 || error.code === 3 ? error.code : 4;
  }
}

module.exports = { TARGETS, platformTarget, manifestAt, enginePath, verifyEngine, verifyReceipt, setupCommand, shellQuote, proxyFor, installEngine, sha256, main };
if (require.main === module) main(process.argv.slice(2)).then(code => { process.exitCode = code; }).catch(error => { process.stderr.write(`${error.message}\n`); process.exitCode = 4; });
