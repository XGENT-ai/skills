import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const renderer = path.join(root, 'skills/feature-demo/scripts/render-report.mjs');
const MiB = 1024 * 1024;

// 批次目录里的媒体用稀疏文件：体积上限按文件大小判定，不需要真写几百 MiB。
function batch(report, files) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'feature-demo-'));
  for (const [name, size] of Object.entries(files)) {
    fs.mkdirSync(path.dirname(path.join(dir, name)), { recursive: true });
    fs.writeFileSync(path.join(dir, name), '');
    fs.truncateSync(path.join(dir, name), size);
  }
  const reportPath = path.join(dir, 'report.json');
  fs.writeFileSync(reportPath, JSON.stringify(report));
  const result = spawnSync(process.execPath, [renderer, reportPath], { encoding: 'utf8' });
  fs.rmSync(dir, { recursive: true, force: true });
  return result;
}

const base = (items, extra = {}) => ({
  schemaVersion: 1,
  batchId: 'DEMO',
  title: '演示',
  meta: { source: 'plan.md', baseline: 'abc123', environment: '本地' },
  items,
  ...extra,
});
const item = (id, fields = {}) => ({ id, name: id, summary: '说明', ...fields });

test('accepts files exactly at the per-file limits', () => {
  const r = batch(base([item('a', { video: 'a.webm', chart: 'a.html' })]), { 'a.webm': 100 * MiB, 'a.html': 25 * MiB });
  assert.equal(r.status, 0, r.stderr);
});

test('rejects a video one byte over 100MiB and a chart one byte over 25MiB', () => {
  const r = batch(base([item('a', { video: 'a.webm', chart: 'a.html' })]), { 'a.webm': 100 * MiB + 1, 'a.html': 25 * MiB + 1 });
  assert.equal(r.status, 2);
  assert.match(r.stderr, /items\[0\]\.video.*100MiB/);
  assert.match(r.stderr, /items\[0\]\.chart.*25MiB/);
});

test('rejects a title longer than 200 characters', () => {
  const r = batch(base([item('a')], { title: '标'.repeat(201) }), {});
  assert.equal(r.status, 2);
  assert.match(r.stderr, /title.*200/);
});

test('rejects field data larger than 2MiB', () => {
  const r = batch(base([item('a', { summary: 'x'.repeat(2 * MiB) })]), {});
  assert.equal(r.status, 2);
  assert.match(r.stderr, /2MiB/);
});

test('counts a file referenced twice once toward the 500MiB total', () => {
  const files = { 'v1.webm': 100 * MiB, 'v2.webm': 100 * MiB, 'v3.webm': 100 * MiB, 'v4.webm': 100 * MiB, 'v5.webm': 99 * MiB };
  const items = ['v1', 'v2', 'v3', 'v4', 'v5'].map((v) => item(v, { video: `${v}.webm` }));
  items.push(item('again', { video: 'v5.webm' }));
  assert.equal(batch(base(items), files).status, 0);
});

test('rejects fields plus files above 500MiB', () => {
  const files = { 'v1.webm': 100 * MiB, 'v2.webm': 100 * MiB, 'v3.webm': 100 * MiB, 'v4.webm': 100 * MiB, 'v5.webm': 100 * MiB };
  const r = batch(base(['v1', 'v2', 'v3', 'v4', 'v5'].map((v) => item(v, { video: `${v}.webm` }))), files);
  assert.equal(r.status, 2);
  assert.match(r.stderr, /500MiB/);
});

test('rejects paths with dot segments or backslashes even when they stay inside the batch', () => {
  const r = batch(base([item('a', { video: 'videos/../a.webm' }), item('b', { chart: 'charts\\b.html' })]), {
    'a.webm': 10,
    'charts\\b.html': 10,
  });
  assert.equal(r.status, 2);
  assert.match(r.stderr, /items\[0\]\.video/);
  assert.match(r.stderr, /items\[1\]\.chart/);
});

test('accepts a leading ./ in paths', () => {
  assert.equal(batch(base([item('a', { video: './a.webm' })]), { 'a.webm': 10 }).status, 0);
});
