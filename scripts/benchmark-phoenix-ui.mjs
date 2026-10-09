#!/usr/bin/env node
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { performance } from 'node:perf_hooks';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const binary = process.env.PHOENIX_UI_BENCH_BIN;
if (!binary || !fs.existsSync(binary)) throw new Error('Set PHOENIX_UI_BENCH_BIN to the staged native artifact.');
if (!['darwin', 'linux'].includes(process.platform)) throw new Error('This RSS probe requires /usr/bin/time on macOS or Linux.');
const fixture = path.join(root, 'test/phoenix-ui/fixtures/web');
const files = fs.readdirSync(fixture).sort();
const hash = crypto.createHash('sha256');
for (const file of files) hash.update(file + '\0').update(fs.readFileSync(path.join(fixture, file)));
const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix benchmark '));
const project = path.join(temp, 'project with spaces');
const home = path.join(temp, 'isolated home');
fs.cpSync(fixture, project, { recursive: true });
fs.mkdirSync(home);
const env = { ...process.env, HOME: home, USERPROFILE: home, NO_COLOR: '1',
  IMPECCABLE_NO_UPDATE_CHECK: '1', IMPECCABLE_NO_TELEMETRY: '1', DO_NOT_TRACK: '1' };
const cases = [
  { name: 'startup', args: ['engine-probe'], exits: [0] },
  { name: 'detect', args: ['detect', 'index.html', '--json'], exits: [0, 2] },
  { name: 'hook', args: ['hook'], exits: [0], stdin: JSON.stringify({ session_id: 'benchmark',
    cwd: project, hook_event_name: 'PostToolUse', tool_name: 'Edit', tool_input: { file_path: path.join(project, 'index.html') } }) },
];
const output = { schemaVersion: 1, platform: process.platform, arch: process.arch,
  binarySha256: crypto.createHash('sha256').update(fs.readFileSync(binary)).digest('hex'),
  fixtureSha256: hash.digest('hex'), samples: 7, hookState: 'fresh project state for every sample', results: [] };
try {
  for (const c of cases) {
    const times = [], rss = [];
    for (let i = 0; i < output.samples; i++) {
      for (const state of ['.impeccable', '.phoenix-ui']) fs.rmSync(path.join(project, state), { recursive: true, force: true });
      const before = performance.now();
      const result = spawnSync('/usr/bin/time', [process.platform === 'darwin' ? '-l' : '-v', binary, ...c.args],
        { cwd: project, env, input: c.stdin || '', encoding: 'utf8', timeout: 30_000 });
      times.push(performance.now() - before);
      if (result.error || !c.exits.includes(result.status)) throw result.error || new Error(`${c.name}: exit ${result.status}: ${result.stderr}`);
      const match = process.platform === 'darwin'
        ? /([0-9]+)\s+maximum resident set size/.exec(result.stderr)
        : /Maximum resident set size \(kbytes\):\s*([0-9]+)/.exec(result.stderr);
      if (!match) throw new Error(`RSS was not reported for ${c.name}`);
      rss.push(Number(match[1]) * (process.platform === 'darwin' ? 1 : 1024));
    }
    output.results.push({ name: c.name, medianMs: [...times].sort((a, b) => a - b)[3],
      maxRssBytes: Math.max(...rss), durationMs: times, peakRssBytes: rss });
  }
  console.log(JSON.stringify(output, null, 2));
} finally {
  fs.rmSync(temp, { recursive: true, force: true });
}
