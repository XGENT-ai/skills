#!/usr/bin/env node
// Full maintenance regression. Real browser/export/provider acceptance has
// separate evidence; protocol fixtures never stand in for those gates.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { root, workspace, preflight, miseEnvironment, verifyFrozenSources } from './check-phoenix-diagram-source.mjs';
import { selectCompilerArtifact } from './build-phoenix-diagram.mjs';

function command(program, args, { cwd = root, env, ...options } = {}) {
  const result = spawnSync(program, args, { cwd, env, stdio: 'inherit', ...options });
  assert(!result.error && result.status === 0, result.error?.message || `${program} exited ${result.status}`);
  return result;
}
export function main(args = process.argv.slice(2)) {
  if (args.length === 1 && args[0] === '--help') {
    console.log('npm run test:diagram\nFull locked/offline diagram Rust, Node and Reader regression.\nRun explicit prepare first; this command does not install tools or download dependencies.\nReal Chrome/export/provider and final-package acceptance remain separate.');
    return 0;
  }
  assert(args.length === 0, 'Usage: node scripts/test-phoenix-diagram-native.mjs [--help]');
  const prepared = preflight();
  verifyFrozenSources();
  const env = { ...miseEnvironment({ CARGO_NET_OFFLINE: 'true' }), RUST_TEST_THREADS: '4' };
  const cargo = (args, options = {}) => command('mise', ['exec', '--no-deps', '--', 'mbx', ...args], { cwd: workspace, env, ...options });
  command(process.execPath, [path.join(root, 'scripts/assemble-phoenix-diagram-reader.mjs'), '--check'], { env });
  cargo(['fmt', '--', '--check']);
  cargo(['clippy', '--locked', '--offline', '--all-targets', '--', '-D', 'warnings']);
  const build = cargo(['build', '--bin', 'phoenix-diagram', '--locked', '--offline', '--message-format=json'], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'inherit'], maxBuffer: 64 * 1024 * 1024 });
  const built = selectCompilerArtifact(build.stdout, workspace);
  const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-diagram-native-regression-'));
  try {
    const binary = path.join(temporary, 'phoenix-diagram');
    fs.copyFileSync(built.executable, binary);
    fs.chmodSync(binary, 0o700);
    env.PHOENIX_DIAGRAM_BIN = binary;
    cargo(['test', '--locked', '--offline', '--no-fail-fast', '--', '--test-threads=4']);
    const tests = fs.readdirSync(path.join(root, 'test/phoenix-diagram'))
      .filter(name => name.endsWith('.test.js')).sort()
      .map(name => path.join(root, 'test/phoenix-diagram', name));
    command(process.execPath, ['--test', '--test-concurrency=1', ...tests], { env });
    command(process.execPath, ['--test',
      path.join(root, 'tools/phoenix-diagram/upstream/source-check.test.mjs'),
      path.join(root, 'tools/phoenix-diagram/upstream/prepare.test.mjs'),
      path.join(root, 'tools/phoenix-diagram/upstream/license-sources.test.mjs'),
      path.join(root, 'tools/phoenix-diagram/tests/viewer/unit.test.mjs'),
      path.join(root, 'tools/phoenix-diagram/tests/viewer/explore.test.mjs'),
      path.join(root, 'tools/phoenix-diagram/tests/viewer/catalogs.test.mjs'),
      path.join(root, 'tools/phoenix-diagram/tests/viewer/evidence.test.mjs'),
      path.join(root, 'tools/phoenix-diagram/tests/export-acceptance.test.mjs'),
    ], { env });
    console.log(JSON.stringify({ schemaVersion: 1, tool: 'phoenix-diagram', command: 'test-native', status: 'pass',
      scope: 'maintenance Rust/Node/Reader regression', realBrowser: 'not-run', providerSessions: 'not-run', tools: prepared }));
  } finally {
    fs.rmSync(temporary, { recursive: true, force: true });
  }
  return 0;
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { process.exitCode = main(); }
  catch (error) { console.error(error.message); process.exitCode = 1; }
}
