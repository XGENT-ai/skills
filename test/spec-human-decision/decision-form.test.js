'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const http = require('node:http');
const { spawn } = require('node:child_process');

const script = path.resolve(__dirname, '../../skills/spec-human-decision/scripts/decision-form.mjs');

const requirementsForm = () => ({
  schemaVersion: 1,
  template: 'requirements',
  round: 1,
  title: 'Export',
  documents: [{ type: 'PRD', path: 'docs/prd.md', version: 'v1' }],
  items: [
    { id: 'FR-01', title: 'Filter', source: '§1', what: 'Filter orders', why: 'Find a month' },
    { id: 'FR-02', title: 'Export', source: '§2', what: 'Download </script><b>csv</b>', why: 'Reconcile', dependsOn: ['FR-01'] },
  ],
});

const planForm = () => ({
  schemaVersion: 1,
  template: 'dev-plan',
  round: 1,
  title: 'Plan',
  documents: [{ type: 'Dev-plan', path: 'docs/plan.md', version: 'v1' }],
  items: [
    { id: 'AD-1', kind: 'decision', title: 'Where', source: '§2', choice: 'Queue', why: 'Timeouts',
      alternatives: [{ name: 'Sync', tradeoff: 'Simple' }], affects: ['F-1'] },
    { id: 'F-1', kind: 'feature', title: 'Export', source: 'M1', what: 'CSV', how: 'Worker' },
  ],
});

function workspace(t, form) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'spec-human-decision-'));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  const formPath = path.join(dir, 'form.json');
  fs.writeFileSync(formPath, JSON.stringify(form));
  return { formPath, resultPath: path.join(dir, 'result.json') };
}

function start(t, form) {
  const { formPath, resultPath } = workspace(t, form);
  const child = spawn(process.execPath, [script, formPath, resultPath, '--no-open'], { stdio: ['ignore', 'pipe', 'pipe'] });
  t.after(() => child.kill());
  const exited = new Promise(resolve => child.on('exit', code => resolve(code)));
  const url = new Promise((resolve, reject) => {
    let out = '';
    child.stdout.on('data', chunk => {
      out += chunk;
      const match = out.match(/FORM URL: (\S+)/);
      if (match) resolve(match[1]);
    });
    child.on('exit', code => reject(new Error(`exited ${code}`)));
  });
  return { url, exited, resultPath };
}

function request(url, method = 'GET', body) {
  return new Promise((resolve, reject) => {
    const req = http.request(url, { method, headers: { 'content-type': 'application/json' } }, res => {
      let text = '';
      res.on('data', chunk => { text += chunk; });
      res.on('end', () => resolve({ status: res.statusCode, text }));
    });
    req.on('error', reject);
    req.end(body === undefined ? undefined : JSON.stringify(body));
  });
}

function runInvalid(t, form) {
  const { formPath, resultPath } = workspace(t, form);
  return new Promise(resolve => {
    const child = spawn(process.execPath, [script, formPath, resultPath, '--no-open']);
    let err = '';
    child.stderr.on('data', chunk => { err += chunk; });
    child.on('exit', code => resolve({ code, err }));
  });
}

for (const [name, form] of [['requirements', requirementsForm], ['dev-plan', planForm]]) {
  test(`${name} form serves the page with the data embedded safely`, async t => {
    const { url } = start(t, form());
    const page = await request(await url);
    assert.equal(page.status, 200);
    const embedded = page.text.match(/<script type="application\/json" id="form-data">([\s\S]*?)<\/script>/);
    assert.ok(embedded, 'data block present');
    assert.deepEqual(JSON.parse(embedded[1]), form());
    assert.ok(!page.text.includes('__FORM_DATA__') && !page.text.includes('/*__FORM_CORE__*/') && !page.text.includes('/*__FORM_CSS__*/'));
  });
}

test('submission is validated, written once, and stops the server', async t => {
  const { url, exited, resultPath } = start(t, requirementsForm());
  const submit = `${await url}submit`;

  const missingNote = await request(submit, 'POST', { items: [{ id: 'FR-01', decision: 'modify' }, { id: 'FR-02', decision: null }] });
  assert.equal(missingNote.status, 400);
  assert.match(missingNote.text, /FR-01/);
  const missingItem = await request(submit, 'POST', { items: [{ id: 'FR-01', decision: 'confirm' }] });
  assert.equal(missingItem.status, 400);
  assert.match(missingItem.text, /FR-02/);
  assert.equal(fs.existsSync(resultPath), false);

  const ok = await request(submit, 'POST', { items: [
    { id: 'FR-01', decision: 'modify', note: 'Add store filter' },
    { id: 'FR-02', decision: null },
  ] });
  assert.equal(ok.status, 200);
  assert.equal(await exited, 0);
  const result = JSON.parse(fs.readFileSync(resultPath, 'utf8'));
  assert.equal(result.form.title, 'Export');
  assert.deepEqual(result.items, [{ id: 'FR-01', decision: 'modify', note: 'Add store filter' }, { id: 'FR-02', decision: null }]);
});

test('an architecture decision can be modified by choosing a listed alternative only', async t => {
  const { url, exited, resultPath } = start(t, planForm());
  const submit = `${await url}submit`;
  const unknown = await request(submit, 'POST', { items: [{ id: 'AD-1', decision: 'modify', alternative: 'Cron' }, { id: 'F-1', decision: 'confirm' }] });
  assert.equal(unknown.status, 400);
  const ok = await request(submit, 'POST', { items: [{ id: 'AD-1', decision: 'modify', alternative: 'Sync' }, { id: 'F-1', decision: 'confirm' }] });
  assert.equal(ok.status, 200);
  assert.equal(await exited, 0);
  assert.equal(JSON.parse(fs.readFileSync(resultPath, 'utf8')).items[0].alternative, 'Sync');
});

test('invalid form data is rejected before serving', async t => {
  const dangling = requirementsForm();
  dangling.items[1].dependsOn = ['FR-09'];
  dangling.items.push({ ...dangling.items[0] });
  const { code, err } = await runInvalid(t, dangling);
  assert.equal(code, 2);
  assert.match(err, /FR-09/);
  assert.match(err, /重复/);

  const plan = planForm();
  delete plan.items[1].how;
  plan.items[0].revision = { reason: 'modify', before: 'a', after: 'b' };
  const second = await runInvalid(t, plan);
  assert.equal(second.code, 2);
  assert.match(second.err, /how/);
  assert.match(second.err, /revision/);
});
