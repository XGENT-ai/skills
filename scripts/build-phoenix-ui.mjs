#!/usr/bin/env node
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { workspace, preflight, buildEnvironment, versions, offlineNetworkProfile } from './phoenix-build-tools.mjs';
import bootstrap from '../skills/phoenix-ui/src/scripts/phoenix-bootstrap.cjs';

const root = path.resolve(workspace, '../..');
const sha = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const json = value => JSON.stringify(value, null, 2) + '\n';
const args = process.argv.slice(2);
let development = false, outputDir;
for (let i = 0; i < args.length; i++) {
  if (args[i] === '--development') development = true;
  else if (args[i] === '--output-dir' && args[i + 1]) outputDir = path.resolve(args[++i]);
  else throw new Error('Usage: node scripts/build-phoenix-ui.mjs --development [--output-dir DIR]');
}
assert(development, 'A one-platform build is development-only; release manifests require the complete CI artifacts');
outputDir ||= path.join(root, 'local/phoenix-ui/development-release');
const prepared = preflight(), env = buildEnvironment(prepared);
assert.equal(prepared.target, prepared.host, 'Development builds must run on the native target host');

function command(program, args, options = {}) {
  const result = spawnSync(program, args, { cwd: workspace, env, encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024, ...options });
  if (result.error || result.status !== 0) throw result.error || new Error(result.stderr || `${program} exited ${result.status}`);
  if (result.stderr) process.stderr.write(result.stderr);
  return result.stdout;
}

function cargo(args) {
  const invocation = ['exec', '--no-deps', '--', 'mbx', ...args];
  return process.platform === 'darwin'
    ? command('sandbox-exec', ['-p', offlineNetworkProfile, 'mise', ...invocation])
    : command('mise', invocation);
}

function git(args) { return command('git', args).trim(); }

function sourceInputs() {
  const paths = ['Cargo.toml', 'Cargo.lock', 'build-tools.lock.json', '.cargo/config.toml'];
  const walk = dir => {
    for (const entry of fs.readdirSync(path.join(workspace, dir), { withFileTypes: true })) {
      const file = `${dir}/${entry.name}`;
      if (entry.isDirectory()) walk(file);
      else if (entry.isFile()) paths.push(file);
      else throw new Error(`Build input is not a regular file: ${file}`);
    }
  };
  walk('crates');
  walk('browser-bundle');
  walk('ui');
  walk('scripts');
  const entries = paths.map(file => [`tools/phoenix-ui/${file}`, sha(fs.readFileSync(path.join(workspace, file)))]);
  const external = directory => {
    for (const entry of fs.readdirSync(path.join(root, directory), { withFileTypes: true })) {
      const file = `${directory}/${entry.name}`;
      if (entry.isDirectory()) external(file);
      else {
        assert(entry.isFile(), `Build input is not a regular file: ${file}`);
        entries.push([file, sha(fs.readFileSync(path.join(root, file)))]);
      }
    }
  };
  external('skills/phoenix-ui/src');
  for (const file of ['package.json', 'mise.toml', 'rust-toolchain.toml', 'scripts/build-phoenix-ui.mjs',
    'scripts/phoenix-build-tools.mjs', 'scripts/phoenix-wasm-pack.mjs']) {
    entries.push([file, sha(fs.readFileSync(path.join(root, file)))]);
  }
  return Object.fromEntries(entries.sort(([a], [b]) => a < b ? -1 : 1));
}

try {
  command('mise', ['exec', '--no-deps', '--', 'bun', 'scripts/render.mjs', '--without-engine']);
  cargo(['xtask', 'bundle']);
  cargo(['xtask', 'bundle', '--check']);
  const inputs = sourceInputs();
  const inputSha256 = sha(JSON.stringify(inputs));
  const build = cargo(['build', '-p', 'phoenix-ui', '--bin', 'phoenix-ui', '--release', '--locked', '--offline', '--message-format=json']);
  const executable = build.trim().split('\n').map(line => JSON.parse(line))
    .find(record => record.reason === 'compiler-artifact' && record.target.kind.includes('bin')
      && record.target.name === 'phoenix-ui' && record.executable)?.executable;
  assert(executable && fs.existsSync(executable), 'Cargo must report the actual native executable');
  assert.equal(command(executable, ['engine-probe']).trim(), 'phoenix-ui-engine 0.1.0');
  assert.equal(sha(JSON.stringify(sourceInputs())), inputSha256, 'Source changed during native build');
  const bytes = fs.readFileSync(executable), target = bootstrap.platformTarget();
  const asset = `phoenix-ui-${target}${target === 'windows-x64' ? '.exe' : ''}`;
  const bundleSha256 = sha(fs.readFileSync(path.join(root, 'vendor/phoenix-ui/bundle/manifest.json')));
  const packageVersion = JSON.parse(fs.readFileSync(path.join(root, 'package.json'))).version;
  const sourceCommit = git(['rev-parse', 'HEAD']);
  const sourceTree = git(['rev-parse', 'HEAD^{tree}']);
  const sourceState = git(['status', '--porcelain']).length ? 'dirty' : 'clean';
  const engine = { asset, sha256: sha(bytes), size: bytes.length };
  const manifest = { schemaVersion: 1, distribution: 'development', toolVersion: '0.1.0', npmPackageVersion: packageVersion,
    sourceCommit, sourceTree, sourceState, sourceInputSha256: inputSha256,
    bundleSchema: 1, bundleSha256, reviewSchema: 1, engines: { [target]: engine } };
  const manifestBytes = Buffer.from(json(manifest));
  fs.mkdirSync(outputDir, { recursive: true });
  fs.writeFileSync(path.join(outputDir, asset), bytes, { mode: 0o755 });
  fs.writeFileSync(path.join(outputDir, 'VERSION.json'), manifestBytes);
  fs.writeFileSync(path.join(root, 'vendor/phoenix-ui/VERSION.json'), manifestBytes);
  for (const name of ['THIRD-PARTY.json', 'THIRD-PARTY-NOTICES.md', 'THIRD-PARTY-LICENSES']) {
    const source = path.join(workspace, name);
    for (const destination of [outputDir, path.join(root, 'vendor/phoenix-ui')]) {
      fs.rmSync(path.join(destination, name), { force: true, recursive: true });
      fs.cpSync(source, path.join(destination, name), { recursive: true });
    }
  }
  command('mise', ['exec', '--no-deps', '--', 'bun', 'scripts/render.mjs']);
  command('mise', ['exec', '--no-deps', '--', 'bun', 'scripts/render.mjs', '--check']);
  bootstrap.verifyEngine(path.join(outputDir, asset), bootstrap.manifestAt(path.join(outputDir, 'VERSION.json')), target, env);
  const record = { schemaVersion: 1, sourceCommit, sourceTree, sourceState, inputSha256, inputs,
    target, host: prepared.host, tools: versions, binary: engine, bundleSha256,
    checks: { engineIdentity: 'passed', browserInputs: 'passed', skillGeneration: 'passed',
      network: process.platform === 'darwin' ? 'OS-denied during Cargo build' : 'Cargo offline; OS observation required in CI',
      fullRuntimeTests: 'not-run-by-build-script' } };
  fs.writeFileSync(path.join(outputDir, 'build-record.json'), json(record));
  console.log(json({ status: 'development-built', target, binary: path.join(outputDir, asset),
    manifestSha256: sha(manifestBytes), binarySha256: engine.sha256, bundleSha256, sourceState }));
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
