'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const path = require('node:path');

test('web fixture starts on loopback with a stable seed and distinct default, empty and error states', async t => {
  const server = spawn(process.execPath, [path.join(__dirname, 'fixtures/web/server.cjs')], { env: { ...process.env, PORT: '0' } });
  t.after(() => server.kill());
  const address = await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.stdout.once('data', data => resolve(JSON.parse(data.toString())));
    server.once('exit', code => reject(new Error(`fixture exited ${code}`)));
  });
  assert.match(address.url, /^http:\/\/127\.0\.0\.1:\d+$/);
  const first = await fetch(`${address.url}/api/tasks`).then(r => r.json());
  assert.equal(first.seed, 'phoenix-ui-v1');
  assert.equal(first.tasks.length, 3);
  assert.deepEqual(await fetch(`${address.url}/api/tasks`).then(r => r.json()), first);
  assert.deepEqual((await fetch(`${address.url}/api/tasks?state=empty`).then(r => r.json())).tasks, []);
  assert.equal((await fetch(`${address.url}/api/tasks?state=error`)).status, 503);
  assert.equal((await fetch(`${address.url}/`)).status, 200);
});
