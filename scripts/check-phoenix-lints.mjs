#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { workspace, miseEnvironment } from './phoenix-build-tools.mjs';
import { additions, clippyDiagnostics, formatDiagnostics } from './lib/phoenix-lint-baseline.mjs';

const baseline = JSON.parse(fs.readFileSync(path.join(workspace, 'lint-baseline.json')));
const env = miseEnvironment({ CARGO_NET_OFFLINE: 'true' });
function run(args) {
  const result = spawnSync('mise', ['exec', '--no-deps', '--', 'mbx', ...args], {
    cwd: workspace, env, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024,
  });
  if (result.error) throw result.error;
  return result;
}

const fmt = run(['fmt', '--all', '--check']);
const clippy = run(['clippy', '--workspace', '--all-targets', '--locked', '--offline', '--message-format=json', '--',
  ...baseline.forcedWarningLints.flatMap(code => ['--force-warn', code])]);
const diagnostics = clippyDiagnostics(clippy.stdout, workspace);
const format = formatDiagnostics(fmt.stdout + fmt.stderr, workspace);
const newFmt = additions(format, baseline.fmt);
const newClippy = additions(diagnostics.counts, baseline.clippy);
if (diagnostics.errors.length || newFmt.length || newClippy.length || clippy.status !== 0
    || ![0, 1].includes(fmt.status) || (fmt.status === 1 && !Object.keys(format).length)) {
  console.error(JSON.stringify({ errors: diagnostics.errors, addedFormatHunks: newFmt.length, addedClippyDiagnostics: newClippy.length,
    newFmt: newFmt.slice(0, 20), newClippy: newClippy.slice(0, 20), fmtExit: fmt.status, clippyExit: clippy.status }, null, 2));
  if (clippy.status !== 0) console.error(clippy.stderr);
  process.exit(1);
}
console.log(`No added diagnostics; inherited rustfmt differences and ${Object.values(diagnostics.counts).reduce((a, b) => a + b, 0)} clippy diagnostics remain recorded.`);
