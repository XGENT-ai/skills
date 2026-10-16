#!/usr/bin/env node
// Only explicit maintenance preparation installs tools or fetches dependencies.
// Browser preparation and the native Intel mbx bootstrap adapt the minimal
// Apache-2.0 patterns in prepare-phoenix-ui.mjs; no UI tool preflight is loaded.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash, randomBytes } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { root, workspace, loadVersions, miseEnvironment, preflight, verifyFrozenSources, browserPin, discoverBrowser } from './check-phoenix-diagram-source.mjs';

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

export function browserPreparationCommands(platform, { directory = path.join(root, 'local/phoenix-diagram/build-tools') } = {}) {
  if (platform !== 'linux-arm64') return [];
  const pin = browserPin(), prefix = path.join(directory, `playwright-${pin.packageVersion}`);
  const npm = path.resolve(path.dirname(process.execPath), '../lib/node_modules/npm/bin/npm-cli.js');
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

export function prepareBrowser({ platform = `${process.platform}-${process.arch}`, env = miseEnvironment(), directory = path.join(root, 'local/phoenix-diagram/build-tools'), evidenceRoot = path.join(root, 'local/phoenix-diagram/ci-prepare', platform) } = {}) {
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
      for (const [program, args] of browserPreparationCommands(platform, { directory })) run(program, args);
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
  if (args.length === 1 && args[0] === '--prepare-browser') {
    try {
      // A consumer receives an immutable maintenance harness, without source or
      // Rust. Keep browser tools and even failed preparation evidence outside it.
      const parent = fs.realpathSync(process.env.RUNNER_TEMP || os.tmpdir()); noSymlinkParents(parent);
      const scratch = fs.mkdtempSync(path.join(parent, 'phoenix-diagram-browser-preparation-')), evidenceRoot = path.join(scratch, 'evidence');
      if (process.env.GITHUB_OUTPUT) fs.appendFileSync(process.env.GITHUB_OUTPUT, `browser_evidence_dir=${evidenceRoot}\n`);
      if (process.env.GITHUB_ENV) fs.appendFileSync(process.env.GITHUB_ENV, `PHOENIX_DIAGRAM_BROWSER_PREPARATION_EVIDENCE=${evidenceRoot}\n`);
      const record = prepareBrowser({ directory: path.join(scratch, 'tools'), evidenceRoot }); console.log(JSON.stringify(record, null, 2));
    }
    catch (error) { console.error(error.message); process.exitCode = 1; }
  } else if (args[0] !== '--prepare' || args.length > 2 || (args.length === 2 && args[1] !== '--update-lock')) {
    console.error('Usage: node scripts/prepare-phoenix-diagram.mjs --prepare [--update-lock]\n       node scripts/prepare-phoenix-diagram.mjs --prepare-browser'); process.exitCode = 2;
  } else {
    try { prepare({ updateLock: args[1] === '--update-lock' }); }
    catch (error) { console.error(error.message); process.exitCode = 1; }
  }
}
