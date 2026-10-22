#!/usr/bin/env node
// Only explicit maintenance preparation installs tools or fetches dependencies.
// Browser preparation and the native Intel mbx bootstrap adapt the minimal
// Apache-2.0 patterns in prepare-phoenix-ui.mjs; no UI tool preflight is loaded.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash, randomBytes } from 'node:crypto';
import https from 'node:https';
import { gunzipSync } from 'node:zlib';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { root, workspace, loadVersions, miseEnvironment, preflight, verifyFrozenSources, browserPin, discoverBrowser } from './check-phoenix-diagram-source.mjs';
import { readSnapshot, bound } from './test-phoenix-diagram-core.mjs';

export function npmPin() {
  const pin = JSON.parse(fs.readFileSync(path.join(workspace, 'build-tools.lock.json'))).consumerNpm;
  if (pin?.package !== 'npm' || pin.version !== '10.5.0' || pin.url !== 'https://registry.npmjs.org/npm/-/npm-10.5.0.tgz'
      || !Number.isSafeInteger(pin.size) || pin.size <= 0 || !/^[a-f0-9]{64}$/.test(pin.sha256) || !/^sha512-[A-Za-z0-9+/]{86}==$/.test(pin.integrity)) throw new Error('Invalid fixed consumer npm pin');
  return pin;
}

async function downloadNpm(pin) {
  const agent = new https.Agent({ proxyEnv: process.env });
  return await new Promise((resolve, reject) => {
    const request = https.get(pin.url, { agent, signal: AbortSignal.timeout(60000) }, response => {
      if (response.statusCode !== 200) { response.destroy(); reject(new Error(`npm archive download failed (${response.statusCode})`)); return; }
      const chunks = []; let length = 0;
      response.on('data', chunk => { length += chunk.length; if (length > pin.size) response.destroy(new Error('npm archive exceeds pinned size')); else chunks.push(chunk); });
      response.on('end', () => resolve(Buffer.concat(chunks))); response.on('error', reject); response.on('close', () => agent.destroy());
    });
    request.on('error', error => { agent.destroy(); reject(error); });
  });
}

// npm ships its own node_modules. The product candidate reader deliberately
// forbids that tree, so this reader accepts only the pinned npm package root.
function npmMembers(bytes, pin) {
  if (bytes.length !== pin.size || createHash('sha256').update(bytes).digest('hex') !== pin.sha256
      || 'sha512-' + createHash('sha512').update(bytes).digest('base64') !== pin.integrity) throw new Error('npm archive size/SHA/integrity differs');
  const data = gunzipSync(bytes, { maxOutputLength: 64 * 1024 * 1024 }), members = new Map(), names = new Set(); let ended = false;
  for (let at = 0; at + 512 <= data.length;) {
    const header = data.subarray(at, at + 512), field = (start, length) => header.subarray(start, start + length).toString().replace(/\0.*$/s, '');
    if (header.every(byte => byte === 0)) { if (at + 1024 > data.length || data.subarray(at).some(byte => byte !== 0)) throw new Error('Invalid npm archive footer'); ended = true; break; }
    const number = (start, length) => { const text = field(start, length).trim(); if (!/^[0-7]+$/.test(text)) throw new Error('Invalid npm archive number'); return parseInt(text, 8); };
    const name = (field(345, 155) ? field(345, 155) + '/' : '') + field(0, 100), normalized = name.replace(/\/$/, ''), type = header[156], size = number(124, 12), mode = number(100, 8);
    let checksum = 0; for (let i = 0; i < 512; i++) checksum += i >= 148 && i < 156 ? 32 : header[i];
    if (checksum !== number(148, 8) || mode & 0o7000 || !Number.isSafeInteger(size) || size < 0 || at + 512 + size > data.length
        || ![0, 48, 53].includes(type) || normalized !== 'package' && !normalized.startsWith('package/')
        || /[\\:\0-\x1f\x7f]/.test(normalized) || normalized.split('/').some(part => !part || part === '.' || part === '..') || names.has(normalized)) throw new Error('Unsafe npm archive member/checksum/type');
    names.add(normalized);
    if (type !== 53) { if (normalized === 'package') throw new Error('Unsafe npm archive root'); members.set(normalized.slice(8), { bytes: data.subarray(at + 512, at + 512 + size), mode: mode & 0o111 ? 0o755 : 0o644 }); }
    else if (size !== 0) throw new Error('Invalid npm archive directory');
    at += 512 + Math.ceil(size / 512) * 512;
  }
  if (!ended || !members.size) throw new Error('Incomplete npm archive');
  for (const name of members.keys()) for (let parent = path.posix.dirname(name); parent !== '.'; parent = path.posix.dirname(parent)) if (members.has(parent)) throw new Error('npm archive file/directory conflict');
  const pkg = JSON.parse(members.get('package.json')?.bytes.toString() || 'null');
  if (pkg?.name !== 'npm' || pkg.version !== pin.version || pkg.bin?.npm !== 'bin/npm-cli.js' || pkg.bin?.npx !== 'bin/npx-cli.js'
      || !members.has('bin/npm-cli.js') || !members.has('bin/npx-cli.js')) throw new Error('npm archive package/bin identity differs');
  return members;
}

export function inspectNpm(npmCliPath, { pin = npmPin() } = {}) {
  if (!path.isAbsolute(npmCliPath) || fs.realpathSync(npmCliPath) !== npmCliPath) throw new Error('Use the original absolute prepared npm CLI');
  const packageRoot = path.dirname(path.dirname(npmCliPath)), directory = path.dirname(packageRoot), stat = fs.lstatSync(directory);
  noSymlinkParents(directory);
  if (npmCliPath !== path.join(directory, 'package/bin/npm-cli.js') || !stat.isDirectory() || stat.uid !== process.getuid() || (stat.mode & 0o7777) !== 0o700) throw new Error('npm tool directory must be owned 0700');
  const archive = readSnapshot(path.join(directory, `npm-${pin.version}.tgz`)), members = npmMembers(archive.content, pin), inventory = [];
  const visit = (parent, prefix = '') => {
    for (const name of fs.readdirSync(parent).sort()) {
      const relative = prefix ? `${prefix}/${name}` : name, file = path.join(parent, name), stat = fs.lstatSync(file);
      if (stat.isDirectory() && !stat.isSymbolicLink()) { if (![...members.keys()].some(key => key.startsWith(relative + '/'))) throw new Error('Extra npm package directory'); visit(file, relative); }
      else { const actual = readSnapshot(file), expected = members.get(relative); if (!expected || actual.binding.uid !== process.getuid() || actual.binding.mode !== expected.mode || !actual.content.equals(expected.bytes)) throw new Error(`npm package member changed: ${relative}`); inventory.push([relative, expected.mode, actual.identity]); }
    }
  };
  visit(packageRoot); if (inventory.length !== members.size) throw new Error('npm package members are missing');
  return { version: pin.version, directory, archive: bound(archive), cli: bound(readSnapshot(npmCliPath)), npx: bound(readSnapshot(path.join(packageRoot, 'bin/npx-cli.js'))), inventorySha256: createHash('sha256').update(JSON.stringify(inventory)).digest('hex') };
}

export async function prepareNpm({ directory, pin = npmPin(), download = downloadNpm } = {}) {
  noSymlinkParents(directory); const stat = fs.lstatSync(directory);
  if (!stat.isDirectory() || stat.uid !== process.getuid() || (stat.mode & 0o7777) !== 0o700 || fs.readdirSync(directory).length) throw new Error('Use a fresh owned 0700 npm tool directory');
  const record = { schemaVersion: 1, tool: 'phoenix-diagram', kind: 'consumer-npm-preparation', status: 'preparing', version: pin.version, node: process.version, pin, commands: [] };
  try {
    const bytes = await download(pin), members = npmMembers(bytes, pin), archive = path.join(directory, `npm-${pin.version}.tgz`), packageRoot = path.join(directory, 'package');
    fs.writeFileSync(archive, bytes, { flag: 'wx', mode: 0o600 }); fs.mkdirSync(packageRoot, { mode: 0o755 });
    for (const [name, entry] of members) { const file = path.join(packageRoot, name); fs.mkdirSync(path.dirname(file), { recursive: true, mode: 0o755 }); fs.writeFileSync(file, entry.bytes, { flag: 'wx', mode: entry.mode }); fs.chmodSync(file, entry.mode); }
    const checked = inspectNpm(path.join(packageRoot, 'bin/npm-cli.js'), { pin }), home = path.join(directory, 'home'), cache = path.join(directory, 'cache'), user = path.join(directory, 'user.npmrc'), global = path.join(directory, 'global.npmrc');
    for (const file of [home, cache]) fs.mkdirSync(file, { mode: 0o700 }); for (const file of [user, global]) fs.writeFileSync(file, '', { flag: 'wx', mode: 0o600 });
    const npmEnvironment = { npm_config_cache: cache, npm_config_userconfig: user, npm_config_globalconfig: global, npm_config_update_notifier: 'false', npm_config_audit: 'false', npm_config_fund: 'false', npm_config_ignore_scripts: 'true', npm_config_logs_max: '10' };
    record.npmEnvironment = npmEnvironment;
    const env = { HOME: home, PATH: path.dirname(process.execPath), ...npmEnvironment };
    const args = [checked.cli.path, '--version'], result = spawnSync(process.execPath, args, { cwd: directory, env, encoding: 'utf8', timeout: 15000, maxBuffer: 1024 * 1024 });
    record.commands.push({ program: process.execPath, args, cwd: directory, exitCode: result.status, signal: result.signal, stdout: result.stdout || '', stderr: result.stderr || '' });
    if (result.error || result.status !== 0 || result.signal || result.stdout.trim() !== pin.version) throw result.error || new Error('Prepared npm version observation differs');
    const current = inspectNpm(checked.cli.path, { pin }); if (JSON.stringify(current) !== JSON.stringify(checked)) throw new Error('Prepared npm changed during version observation');
    Object.assign(record, { status: 'prepared', npmCliPath: checked.cli.path, actualVersion: result.stdout.trim(), preparedTool: checked }); return record;
  } catch (error) { record.status = 'failed'; record.error = error.message; throw error; }
  finally { fs.writeFileSync(path.join(directory, 'preparation.json'), JSON.stringify(record, null, 2) + '\n', { flag: 'wx', mode: 0o600 }); }
}

export function preparationCommands(versions, lockfileExists, { updateLock = false, platform, boxingtonAvailable = true } = {}) {
  const bootstrap = platform === 'darwin-x64' && !boxingtonAvailable
    ? JSON.parse(fs.readFileSync(path.join(root, 'tools/phoenix-ui/build-tool-assets.json'))).nativeBootstrap['darwin-x64'] : null;
  if (bootstrap && (bootstrap.tool !== 'mr-boxington' || bootstrap.package !== 'mbx' || bootstrap.version !== versions.mrBoxington
      || bootstrap.url !== 'https://github.com/jdx/mr-boxington' || !/^[a-f0-9]{40}$/.test(bootstrap.revision))) throw new Error('Invalid pinned Intel Diagram mbx bootstrap');
  const bootstrapRoot = path.join(root, 'local/phoenix-diagram/build-tools/darwin-x64', `mr-boxington-${versions.mrBoxington}`);
  return [
    ...(bootstrap ? [
      ['mise', ['install', `rust@${versions.rust}`, `node@${versions.node}`]],
      ['mise', ['exec', '--no-deps', '--', 'rustup', 'run', versions.rust, 'cargo', 'install', '--locked', '--git', bootstrap.url, '--rev', bootstrap.revision, '--root', bootstrapRoot, bootstrap.package]],
      ['mise', ['link', `mr-boxington@${versions.mrBoxington}`, bootstrapRoot]],
    ] : []),
    ['mise', ['install', `rust@${versions.rust}`, `node@${versions.node}`, `mr-boxington@${versions.mrBoxington}`]],
    ['mise', ['exec', '--no-deps', '--', 'rustup', 'component', 'add', '--toolchain', versions.rust, 'rustfmt', 'clippy']],
    ...(!lockfileExists || updateLock ? [['mise', ['exec', '--no-deps', '--', 'mbx', 'generate-lockfile']]] : []),
    ['mise', ['exec', '--no-deps', '--', 'mbx', 'fetch', '--locked']],
  ];
}

export function browserPreparationCommands(platform, { directory = path.join(root, 'local/phoenix-diagram/build-tools'), npmCliPath } = {}) {
  if (platform !== 'linux-arm64') return [];
  const pin = browserPin(), prefix = path.join(directory, `playwright-${pin.packageVersion}`);
  const npm = npmCliPath || path.resolve(path.dirname(process.execPath), '../lib/node_modules/npm/bin/npm-cli.js');
  return [
    [process.execPath, [npm, 'install', '--prefix', prefix, '--save-exact', '--ignore-scripts', '--no-audit', '--no-fund', `playwright@${pin.packageVersion}`]],
    [process.execPath, [path.join(prefix, 'node_modules/playwright/cli.js'), 'install', '--with-deps', '--no-shell', 'chromium']],
  ];
}

export function verifyPreparedBrowser(browser, platform) {
  if (browser?.status !== 'available' || !path.isAbsolute(browser.executable || '') || !Number.isSafeInteger(browser.bytes) || browser.bytes <= 0
      || !/^[a-f0-9]{64}$/.test(browser.sha256) || !/^(?:Google Chrome(?: Canary)?|Chromium|Chrome) \d+\.\d+\.\d+\.\d+/.test(browser.actualVersion)
      || browser.verification !== 'not-run') throw new Error('Prepared browser identity is incomplete; no browser acceptance was run');
  if (platform === 'linux-arm64') {
    const pin = browserPin();
    for (const key of ['package', 'packageVersion', 'revision', 'expectedVersion', 'actualVersion', 'sha256', 'source']) {
      if (browser[key] !== pin[key]) throw new Error(`Prepared Chromium identity differs from the fixed pin: ${key}`);
    }
  }
  return browser;
}

function noSymlinkParents(directory) {
  for (let current = path.resolve(directory);; current = path.dirname(current)) {
    try { const stat = fs.lstatSync(current); if (!stat.isDirectory() || stat.isSymbolicLink()) throw new Error(`Preparation directory is nonregular or symbolic: ${current}`); }
    catch (error) { if (error.code !== 'ENOENT') throw error; }
    if (path.dirname(current) === current) return;
  }
}
function browserBytes(file) {
  const before = fs.statSync(file, { bigint: true });
  if (!before.isFile() || (before.mode & 0o111n) === 0n) throw new Error('Prepared browser is not an executable ordinary file');
  const bytes = fs.readFileSync(file), after = fs.statSync(file, { bigint: true });
  for (const key of ['dev', 'ino', 'size', 'mode', 'mtimeNs', 'ctimeNs']) if (before[key] !== after[key]) throw new Error('Browser changed during its identity read');
  return bytes;
}

export function prepareBrowser({ platform = `${process.platform}-${process.arch}`, env = miseEnvironment(), directory = path.join(root, 'local/phoenix-diagram/build-tools'), evidenceRoot = path.join(root, 'local/phoenix-diagram/ci-prepare', platform), npmCliPath } = {}) {
  if (!['darwin-arm64', 'darwin-x64', 'linux-x64', 'linux-arm64'].includes(platform)) throw new Error('Unsupported native Diagram browser preparation platform');
  noSymlinkParents(evidenceRoot); fs.mkdirSync(evidenceRoot, { recursive: true });
  const id = randomBytes(12).toString('hex'), receiptPath = path.join(evidenceRoot, `browser-${id}.json`), environmentFile = path.join(evidenceRoot, `browser-${id}.env.json`);
  const record = { schemaVersion: 1, tool: 'phoenix-diagram', kind: 'browser-preparation', platform, status: 'preparing', startedAt: new Date().toISOString(), commands: [], browserAcceptance: 'not-run' };
  const preparedEnv = { ...env }, run = (program, args, capture = false) => {
    const command = { program, args }; record.commands.push(command);
    const result = spawnSync(program, args, { cwd: root, env: preparedEnv, stdio: capture ? 'pipe' : 'inherit', encoding: 'utf8', timeout: capture ? 15000 : undefined, maxBuffer: 64 * 1024 * 1024 });
    command.exitCode = result.status;
    if (result.error || result.status !== 0) throw result.error || new Error(`${program} browser preparation failed (${result.status}): ${result.stderr || ''}`);
    return result.stdout;
  };
  try {
    let found, packageIdentity = {};
    if (platform === 'linux-arm64') {
      if (preparedEnv.PHOENIX_DIAGRAM_BROWSER !== undefined) {
        const configured = discoverBrowser({ env: preparedEnv, candidates: [] });
        if (configured.status !== 'available') throw new Error(`${configured.diagnostic.code}: ${configured.diagnostic.message}`);
      }
      const pin = browserPin(), suffix = `/builds/chromium/${pin.revision}/chromium-linux-arm64.zip`;
      noSymlinkParents(directory); fs.mkdirSync(directory, { recursive: true });
      preparedEnv.PLAYWRIGHT_BROWSERS_PATH = path.join(directory, 'browsers');
      preparedEnv.PLAYWRIGHT_DOWNLOAD_HOST = pin.source.slice(0, -suffix.length);
      preparedEnv.PLAYWRIGHT_CHROMIUM_DOWNLOAD_HOST = preparedEnv.PLAYWRIGHT_DOWNLOAD_HOST;
      preparedEnv.PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD = '1';
      for (const [program, args] of browserPreparationCommands(platform, { directory, npmCliPath })) run(program, args);
      const packageRoot = path.join(directory, `playwright-${pin.packageVersion}`, 'node_modules');
      const packageVersion = JSON.parse(fs.readFileSync(path.join(packageRoot, 'playwright/package.json'))).version;
      const definition = JSON.parse(fs.readFileSync(path.join(packageRoot, 'playwright-core/browsers.json'))).browsers.find(item => item.name === 'chromium');
      const executable = run(process.execPath, ['-e', 'process.stdout.write(require(process.argv[1]).chromium.executablePath())', path.join(packageRoot, 'playwright')], true).trim();
      found = discoverBrowser({ env: { PHOENIX_DIAGRAM_BROWSER: executable }, candidates: [] });
      packageIdentity = { package: pin.package, packageVersion, revision: definition?.revision, expectedVersion: definition?.browserVersion, source: pin.source };
    } else found = discoverBrowser({ platform, env: preparedEnv });
    if (found.status !== 'available') throw new Error(`${found.diagnostic.code}: ${found.diagnostic.message}`);
    const bytes = browserBytes(found.path), actualVersion = run(found.path, ['--version'], true).trim();
    if (!browserBytes(found.path).equals(bytes)) throw new Error('Browser changed during version observation');
    record.browser = verifyPreparedBrowser({ status: 'available', executable: found.path, bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex'), actualVersion, ...packageIdentity, verification: 'not-run' }, platform);
    if (/[\r\n\0]/.test(found.path)) throw new Error('Browser path cannot be represented safely in the prepared environment');
    const environment = { PHOENIX_DIAGRAM_BROWSER: found.path };
    fs.writeFileSync(environmentFile, JSON.stringify(environment, null, 2) + '\n', { flag: 'wx', mode: 0o600 });
    if (process.env.GITHUB_ENV) fs.appendFileSync(process.env.GITHUB_ENV, `PHOENIX_DIAGRAM_BROWSER=${found.path}\n`);
    const display = file => file.startsWith(root + path.sep) ? path.relative(root, file) : file;
    record.status = 'prepared'; record.finishedAt = new Date().toISOString(); record.environmentFile = display(environmentFile);
    return { ...record, receipt: display(receiptPath), environment };
  } catch (error) { record.status = 'failed'; record.error = error.message; throw error; }
  finally { record.finishedAt = new Date().toISOString(); fs.writeFileSync(receiptPath, JSON.stringify(record, null, 2) + '\n', { flag: 'wx', mode: 0o600 }); }
}

export function prepare({ updateLock = false } = {}) {
  const sources = verifyFrozenSources(), versions = loadVersions();
  const platform = `${process.platform === 'win32' ? 'windows' : process.platform}-${process.arch}`;
  const evidenceRoot = path.join(root, 'local/phoenix-diagram/ci-prepare');
  const record = { schemaVersion: 1, startedAt: new Date().toISOString(), platform, status: 'preparing', updateLock, tools: versions, sources, commands: [] };
  fs.mkdirSync(evidenceRoot, { recursive: true });
  const env = miseEnvironment({ CARGO_NET_OFFLINE: 'false' });
  try {
    const selected = platform === 'darwin-x64' ? spawnSync('mise', ['ls', '--current', '--json'], { cwd: workspace, env, encoding: 'utf8' }) : null;
    if (selected?.error || selected && selected.status !== 0) throw selected.error || new Error(selected.stderr || 'Cannot inspect installed Intel mbx');
    const boxingtonAvailable = !selected || Boolean(JSON.parse(selected.stdout)['mr-boxington']?.some(tool => tool.version === versions.mrBoxington && tool.installed));
    for (const [program, args] of preparationCommands(versions, fs.existsSync(path.join(workspace, 'Cargo.lock')), { updateLock, platform, boxingtonAvailable })) {
      const command = { program, args }; record.commands.push(command);
      const result = spawnSync(program, args, { cwd: workspace, env, stdio: 'inherit' });
      command.exitCode = result.status;
      if (result.error || result.status !== 0) throw result.error || new Error(`${program} ${args.join(' ')} failed (${result.status})`);
    }
    record.preflight = preflight();
    const browser = prepareBrowser({ platform, env });
    record.browser = browser.browser; record.browserReceipt = browser.receipt; record.environmentFile = browser.environmentFile;
    record.cargoLockSha256 = createHash('sha256').update(fs.readFileSync(path.join(workspace, 'Cargo.lock'))).digest('hex');
    record.status = 'prepared';
    record.finishedAt = new Date().toISOString();
    console.log(JSON.stringify({ status: record.status, platform, cargoLockSha256: record.cargoLockSha256, evidence: path.relative(root, path.join(evidenceRoot, `${platform}.json`)) }, null, 2));
    return record;
  } catch (error) { record.status = 'failed'; record.error = error.message; throw error; }
  finally { fs.writeFileSync(path.join(evidenceRoot, `${platform}.json`), JSON.stringify(record, null, 2) + '\n'); }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  if (args.length === 1 && ['--prepare-browser', '--prepare-consumer'].includes(args[0])) {
    try {
      // A consumer receives an immutable maintenance harness, without source or
      // Rust. Keep browser tools and even failed preparation evidence outside it.
      const parent = fs.realpathSync(process.env.RUNNER_TEMP || os.tmpdir()); noSymlinkParents(parent);
      const consumer = args[0] === '--prepare-consumer', scratch = fs.mkdtempSync(path.join(parent, consumer ? 'phoenix-diagram-consumer-preparation-' : 'phoenix-diagram-browser-preparation-')), evidenceRoot = path.join(scratch, 'evidence');
      if (process.env.GITHUB_OUTPUT) fs.appendFileSync(process.env.GITHUB_OUTPUT, `browser_evidence_dir=${evidenceRoot}\n`);
      if (process.env.GITHUB_ENV) fs.appendFileSync(process.env.GITHUB_ENV, `PHOENIX_DIAGRAM_BROWSER_PREPARATION_EVIDENCE=${evidenceRoot}\n`);
      let npm;
      if (consumer) {
        const directory = path.join(scratch, 'npm'); fs.mkdirSync(directory, { mode: 0o700 });
        if (process.env.GITHUB_OUTPUT) fs.appendFileSync(process.env.GITHUB_OUTPUT, `npm_evidence_dir=${directory}\n`);
        npm = await prepareNpm({ directory });
        if (process.env.GITHUB_OUTPUT) fs.appendFileSync(process.env.GITHUB_OUTPUT, `npm_cli_path=${npm.npmCliPath}\n`);
      }
      const record = prepareBrowser({ directory: path.join(scratch, 'tools'), evidenceRoot, npmCliPath: npm?.npmCliPath, ...(npm ? { env: miseEnvironment(npm.npmEnvironment) } : {}) }); console.log(JSON.stringify(consumer ? { npm, browser: record } : record, null, 2));
    }
    catch (error) { console.error(error.message); process.exitCode = 1; }
  } else if (args[0] !== '--prepare' || args.length > 2 || (args.length === 2 && args[1] !== '--update-lock')) {
    console.error('Usage: node scripts/prepare-phoenix-diagram.mjs --prepare [--update-lock]\n       node scripts/prepare-phoenix-diagram.mjs --prepare-browser|--prepare-consumer'); process.exitCode = 2;
  } else {
    try { prepare({ updateLock: args[1] === '--update-lock' }); }
    catch (error) { console.error(error.message); process.exitCode = 1; }
  }
}
