#!/usr/bin/env node
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { workspace, preflight, buildEnvironment, offlineNetworkProfile, testNetworkProfile } from './phoenix-build-tools.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const prepared = preflight();
const env = buildEnvironment(prepared);
let runtimeDir;
function command(command, args, options = {}) {
  const result = spawnSync(command, args, { cwd: workspace, env, stdio: 'inherit', ...options });
  if (result.error || result.status !== 0) throw result.error || new Error(`${command} exited ${result.status}`);
  return result;
}
function cargo(args, options = {}) {
  const invocation = ['exec', '--no-deps', '--', 'mbx', ...args];
  const profile = args[0] === 'test' ? testNetworkProfile : offlineNetworkProfile;
  const { browserAcceptance = false, ...commandOptions } = options;
  return process.platform === 'darwin' && !browserAcceptance
    ? command('sandbox-exec', ['-p', profile, 'mise', ...invocation], commandOptions)
    : command('mise', invocation, commandOptions);
}
try {
  cargo(['xtask', 'bundle', '--check']);
  const engineBuild = cargo(['build', '-p', 'phoenix-ui', '--bin', 'phoenix-ui', '--locked', '--offline', '--message-format=json'],
    { stdio: ['ignore', 'pipe', 'inherit'], encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  const builtEngine = engineBuild.stdout.trim().split('\n').map(line => JSON.parse(line))
    .find(record => record.reason === 'compiler-artifact' && record.target.name === 'phoenix-ui' && record.executable)?.executable;
  if (!builtEngine) throw new Error('Cargo did not report the current native engine artifact.');
  // Later mbx commands can collect the managed target that owns this artifact.
  // Keep the same compiler bytes alive for Node installation and oracle replay.
  runtimeDir = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-runtime-'));
  const engine = path.join(runtimeDir, path.basename(builtEngine));
  fs.copyFileSync(builtEngine, engine);
  fs.chmodSync(engine, 0o755);
  env.PHOENIX_UI_BIN = engine;
  command(process.execPath, ['--test', '--test-name-pattern=critique collision inputs', path.join(root, 'test/phoenix-ui/oracle.test.js')]);
  // Node installer → Rust installer with this engine; old packages come from preparation.
  command(process.execPath, ['--test', path.join(root, 'test/phoenix-ui/install.test.js')]);
  // Serial execution avoids the upstream ephemeral-port test racing a new listener.
  cargo(['test', '--workspace', '--exclude', 'impeccable-browser', '--exclude', 'phoenix-ui', '--locked', '--offline', '--', '--test-threads=1']);
  // Chrome cannot initialize its own macOS sandbox beneath sandbox-exec.
  // Keep browser acceptance in Chrome's sandbox; Cargo stays locked/offline.
  cargo(['test', '-p', 'impeccable-browser', '-p', 'phoenix-ui', '--locked', '--offline', '--', '--test-threads=1'],
    { browserAcceptance: true });
  command(process.execPath, [path.join(root, 'scripts/check-phoenix-lints.mjs')]);
  cargo(['xtask', 'bundle', '--pure']);
  const artifact = cargo(['build', '-p', 'impeccable-wasm', '--example', 'replay_vectors', '--locked', '--offline', '--message-format=json'],
    { stdio: ['ignore', 'pipe', 'inherit'], encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  const executable = artifact.stdout.trim().split('\n').map(line => JSON.parse(line))
    .find(record => record.reason === 'compiler-artifact' && record.target.name === 'replay_vectors' && record.executable)?.executable;
  if (!executable || !fs.existsSync(executable)) throw new Error('Cargo did not report the native vector replay artifact.');
  command(process.execPath, [path.join(root, 'scripts/check-phoenix-wasm.mjs')],
    { env: { ...env, PHOENIX_UI_VECTORS_BIN: executable } });
  command(process.execPath, [path.join(root, 'scripts/check-phoenix-oracle.mjs'), '--bin', engine,
    '--include-actual', '--output', path.join(root, 'local/phoenix-ui/ci-prepare',
      `${prepared.target || `${process.platform}-${process.arch}`}-oracle.json`)]);
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
} finally {
  if (runtimeDir) fs.rmSync(runtimeDir, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 });
}
