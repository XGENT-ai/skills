#!/usr/bin/env node
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import bootstrap from '../skills/phoenix-ui/src/scripts/phoenix-bootstrap.cjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const json = value => JSON.stringify(value, null, 2) + '\n';

export function assertStaticDetection(output) {
  const findings = JSON.parse(output);
  assert(Array.isArray(findings) && findings.some(finding => finding.antipattern === 'overused-font'), 'Static HTML detector must report the seeded rule');
}

export function validateRuntimeArtifact(releaseDir, target) {
  const manifest = bootstrap.manifestAt(path.join(releaseDir, 'VERSION.json'));
  const engine = manifest.data.engines[target];
  assert(engine, `No verified artifact for ${target}`);
  const file = path.join(releaseDir, engine.asset);
  const stat = fs.lstatSync(file);
  assert(stat.isFile() && !stat.isSymbolicLink(), 'Runtime artifact must be a regular file');
  assert.equal(stat.size, engine.size, 'Runtime artifact size differs');
  assert.equal(bootstrap.sha256(fs.readFileSync(file)), engine.sha256, 'Runtime artifact SHA-256 differs');
  const record = JSON.parse(fs.readFileSync(path.join(releaseDir, 'build-record.json')));
  assert.equal(record.target, target, 'Compiler build record target differs');
  assert.equal(record.binary.sha256, engine.sha256, 'Compiler build record binary differs');
  assert.equal(record.binary.size, engine.size, 'Compiler build record binary size differs');
  assert.equal(record.bundleSha256, manifest.data.bundleSha256, 'Compiler build record bundle differs');
  return { manifest, file };
}

export function stageRuntime(releaseDir, packageRoot = root) {
  const dest = path.join(releaseDir, 'runtime');
  assert(!fs.existsSync(dest), 'Runtime staging already exists');
  const sourceSkill = path.join(packageRoot, 'skills/phoenix-ui');
  assert(fs.existsSync(path.join(sourceSkill, 'scripts/phoenix-bootstrap.cjs')), 'Build must generate the portable skill before smoke');
  fs.mkdirSync(path.join(dest, 'bin'), { recursive: true });
  fs.copyFileSync(path.join(packageRoot, 'bin/phoenix-ui.js'), path.join(dest, 'bin/phoenix-ui.js'));
  fs.cpSync(sourceSkill, path.join(dest, 'skills/phoenix-ui'), {
    recursive: true, dereference: false, filter: file => file !== path.join(sourceSkill, 'src'),
  });
  const vendor = path.join(dest, 'vendor/phoenix-ui');
  fs.mkdirSync(vendor, { recursive: true });
  fs.cpSync(path.join(packageRoot, 'vendor/phoenix-ui/bundle'), path.join(vendor, 'bundle'), { recursive: true });
  for (const name of ['VERSION.json', 'THIRD-PARTY.json', 'THIRD-PARTY-NOTICES.md', 'THIRD-PARTY-LICENSES']) {
    fs.cpSync(path.join(releaseDir, name), path.join(vendor, name), { recursive: true });
  }
  for (const name of ['LICENSE', 'NOTICE.md']) fs.copyFileSync(path.join(packageRoot, 'vendor/phoenix-ui', name), path.join(vendor, name));
  const manifestBytes = fs.readFileSync(path.join(releaseDir, 'VERSION.json'));
  assert(fs.readFileSync(path.join(dest, 'skills/phoenix-ui/scripts/ENGINE.json')).equals(manifestBytes), 'Generated ENGINE.json differs from the artifact VERSION bytes');
  return dest;
}

export function runSmoke(releaseDir, target, { fallback = false, stage = false } = {}) {
  assert.equal(bootstrap.platformTarget(), target, 'Smoke must execute the artifact on the intended host');
  if (fallback) assert(process.platform === 'win32' && process.arch === 'arm64', 'Windows ARM fallback requires native ARM64 Node on windows-11-arm');
  const record = { schemaVersion: 1, startedAt: new Date().toISOString(), status: 'running', target,
    host: { platform: process.platform, architecture: process.arch, node: process.version },
    mode: fallback ? 'windows-arm64-x64-fallback' : 'native', checks: [],
    browserAcceptance: 'not-run', providerSessionAcceptance: 'not-run', paidAcceptance: 'not-run' };
  let temporary;
  try {
    const { manifest, file } = validateRuntimeArtifact(releaseDir, target);
    record.manifestSha256 = manifest.hash; record.binarySha256 = manifest.data.engines[target].sha256;
    const runtime = stage ? stageRuntime(releaseDir) : path.join(releaseDir, 'runtime');
    assert(fs.readFileSync(path.join(runtime, 'vendor/phoenix-ui/VERSION.json')).equals(manifest.bytes), 'Downloaded runtime VERSION bytes differ');
    const bundle = fs.readFileSync(path.join(runtime, 'vendor/phoenix-ui/bundle/manifest.json'));
    assert.equal(bootstrap.sha256(bundle), manifest.data.bundleSha256, 'Runtime bundle digest differs');
    if (process.platform !== 'win32') fs.chmodSync(file, 0o755); // Artifacts do not preserve Unix execute modes.
    temporary = fs.mkdtempSync(path.join(fs.realpathSync(os.tmpdir()), 'phoenix ci smoke space '));
    const project = path.join(temporary, 'project'), home = path.join(temporary, 'home'), cache = path.join(temporary, 'engine cache');
    fs.mkdirSync(path.join(project, '.git'), { recursive: true }); fs.mkdirSync(home);
    fs.writeFileSync(path.join(project, 'PRODUCT.md'), '# Product\nPhoenix CI local fixture.\n');
    fs.writeFileSync(path.join(project, 'DESIGN.md'), '# Design\nUse Georgia for the fixture.\n');
    fs.writeFileSync(path.join(project, 'page.html'), '<!doctype html><html><head><style>body{font-family:Inter,sans-serif}</style></head><body><main><h1>CI fixture</h1></main></body></html>');
    const env = { ...process.env, HOME: home, USERPROFILE: home, PHOENIX_UI_HOME: cache,
      PHOENIX_UI_PROVIDER_ID: 'claude-code', PHOENIX_UI_SKILL_DIR: path.join(runtime, 'skills/phoenix-ui'),
      PHOENIX_UI_NO_STALENESS_CHECK: '1', CARGO_NET_OFFLINE: 'true',
      HTTPS_PROXY: 'http://127.0.0.1:9', HTTP_PROXY: 'http://127.0.0.1:9', ALL_PROXY: 'http://127.0.0.1:9',
      https_proxy: 'http://127.0.0.1:9', http_proxy: 'http://127.0.0.1:9', all_proxy: 'http://127.0.0.1:9', NO_PROXY: '', no_proxy: '' };
    delete env.PHOENIX_UI_BUNDLE_PATH;
    const call = (name, executable, args, expected, input) => {
      const result = spawnSync(executable, args, { cwd: project, env, input, encoding: 'utf8', timeout: 30000, maxBuffer: 4 * 1024 * 1024, windowsHide: true });
      record.checks.push({ name, argv: args, exitCode: result.status, error: result.error?.message });
      assert(!result.error && result.status === expected, `${name} failed: ${result.error?.message || result.stderr || result.stdout} (exit ${result.status})`);
      return result.stdout;
    };
    assert.equal(call('engine-probe', file, ['engine-probe'], 0).trim(), `phoenix-ui-engine ${manifest.data.toolVersion}`);
    assert.equal(call('version', file, ['--version'], 0).trim(), manifest.data.toolVersion);
    assert.match(call('root-help', file, ['--help'], 0), /Usage: phoenix-ui/);
    const skills = JSON.parse(call('local-command-catalog', file, ['skills', 'help', '--json'], 0));
    assert.equal(skills.status, 'available'); assert.equal(Object.keys(skills.commands).length, 24);
    assertStaticDetection(call('static-html-detect', file, ['detect', '--no-config', '--json', 'page.html'], 2));
    assert.match(call('local-context', file, ['context', '--target', project], 0), /PRODUCT\.md[\s\S]*Phoenix CI local fixture/);
    call('hook-session-start', file, ['hook'], 0, JSON.stringify({ cwd: project, session_id: 'phoenix-ci', hook_event_name: 'SessionStart' }));
    const launcher = path.join(runtime, 'bin/phoenix-ui.js');
    const node = (name, args, expected) => call(name, process.execPath, [launcher, ...args], expected);
    assert.equal(JSON.parse(node('node-missing-engine', ['context', '--json'], 4)).status, 'unavailable');
    assert.equal(JSON.parse(node('node-explicit-offline-install', ['engine', 'install', '--project', project, '--release-dir', releaseDir, '--json'], 0)).status, 'installed');
    assert.equal(node('node-engine-version', ['--version'], 0).trim(), manifest.data.toolVersion);
    assert.match(node('node-context', ['context', '--target', project], 0), /Phoenix CI local fixture/);
    assert.equal(JSON.parse(node('local-bundle-check', ['skills', 'check', '--bundle-root', path.join(runtime, 'vendor/phoenix-ui/bundle'), '--json'], 0)).status, 'not-installed');
    record.status = 'passed'; record.finishedAt = new Date().toISOString();
    return record;
  } catch (error) { record.status = 'failed'; record.error = error.message; throw error; }
  finally {
    fs.writeFileSync(path.join(releaseDir, fallback ? 'windows-arm-fallback-smoke.json' : `runtime-smoke-node${process.versions.node.split('.')[0]}.json`), json(record));
    if (temporary) fs.rmSync(temporary, { recursive: true, force: true });
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const args = process.argv.slice(2); let releaseDir, target, fallback = false, stage = false;
    for (let i = 0; i < args.length; i++) {
      if (args[i] === '--release-dir' && args[i + 1]) releaseDir = path.resolve(args[++i]);
      else if (args[i] === '--target' && args[i + 1]) target = args[++i];
      else if (args[i] === '--windows-arm-fallback') fallback = true;
      else if (args[i] === '--stage-runtime') stage = true;
      else throw new Error('Unknown smoke argument');
    }
    assert(releaseDir && target, 'Usage: node scripts/smoke-phoenix-ui.mjs --release-dir DIR --target TARGET [--stage-runtime | --windows-arm-fallback]');
    console.log(json(runSmoke(releaseDir, target, { fallback, stage })));
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
