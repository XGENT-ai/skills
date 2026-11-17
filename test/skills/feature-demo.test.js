import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import http from 'node:http';
import net from 'node:net';
import { spawn, spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const renderer = path.join(root, 'skills/feature-demo/scripts/render-report.mjs');
const publisher = path.join(root, 'skills/feature-demo/scripts/publish-report.mjs');
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
  if (result.status === 0) result.html = fs.readFileSync(path.join(dir, 'index.html'), 'utf8');
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

test('reader page keeps usage and limits without exposing provenance metadata', () => {
  const r = batch(base([item('filter', {
    name: '任务筛选', howTo: ['选择负责人'], limits: ['仅显示有权限的任务'],
  })], {
    meta: { source: 'private-plan.md', baseline: 'internal-commit', environment: 'internal-process-evidence' },
  }), {});
  assert.equal(r.status, 0, r.stderr);
  assert.match(r.html, /选择负责人/);
  assert.match(r.html, /仅显示有权限的任务/);
  assert.match(r.html, /href="#filter"/);
  assert.doesNotMatch(r.html, /private-plan|internal-commit|internal-process-evidence/);
});

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

// 发布脚本用异步 spawn：spawnSync 会卡住同进程里的桩服务。
function publish(url, token) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'feature-demo-'));
  const reportPath = path.join(dir, 'report.json');
  fs.writeFileSync(reportPath, JSON.stringify(base([item('a')])));
  const child = spawn(process.execPath, [publisher, reportPath, '--project', 'p1'], {
    env: { ...process.env, SPMS_MCP_URL: url, SPMS_TOKEN: token },
  });
  let stderr = '';
  child.stderr.setEncoding('utf8').on('data', (chunk) => (stderr += chunk));
  return new Promise((done) =>
    child.on('close', (status) => {
      fs.rmSync(dir, { recursive: true, force: true });
      done({ status, stderr });
    }),
  );
}

const listen = (server) => new Promise((done) => server.listen(0, '127.0.0.1', () => done(server.address().port)));

test('publish reports 连不上 SPMS with the address and reason when nothing listens', async () => {
  const idle = net.createServer();
  const port = await listen(idle);
  await new Promise((done) => idle.close(done));
  const url = `http://127.0.0.1:${port}/mcp`;
  const r = await publish(url, 'secret-token-value');
  assert.equal(r.status, 1, r.stderr);
  assert.ok(r.stderr.includes(`连不上 SPMS（${url}）：ECONNREFUSED`), r.stderr);
  assert.doesNotMatch(r.stderr, /TypeError|^\s+at /m);
  assert.ok(!r.stderr.includes('secret-token-value'), r.stderr);
});

test('publish says 不确定报告是否已发布 when 连不上 SPMS during commit', async () => {
  const begin = { publishId: 'p', uploads: [], uploadUrlTtlSeconds: 900 };
  let calls = 0;
  const stub = http.createServer((req, res) => {
    if (++calls > 1) return req.socket.destroy();
    res.setHeader('content-type', 'application/json');
    res.end(JSON.stringify({ jsonrpc: '2.0', id: 1, result: { content: [{ type: 'text', text: JSON.stringify(begin) }] } }));
  });
  const port = await listen(stub);
  const r = await publish(`http://127.0.0.1:${port}/mcp`, 'secret-token-value');
  await new Promise((done) => stub.close(done));
  assert.equal(calls, 2);
  assert.equal(r.status, 1, r.stderr);
  assert.match(r.stderr, /连不上 SPMS/);
  assert.match(r.stderr, /不确定报告是否已发布/);
  assert.doesNotMatch(r.stderr, /TypeError|^\s+at /m);
});
