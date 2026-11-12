'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawn } = require('node:child_process');
const script = path.resolve(__dirname, '../../skills/elintp/scripts/review.mjs');
const draft = body => `---\ntemplate: general\ntitle: Queue\nlang: en\n---\nOrders wait.\n\n## why | Work continues\n${body}\n`;
const item = (sectionId = 's1', note = '') => ({ blockId: `${sectionId}:b1`, sectionId, quote: 'Work waits.', kind: 'unclear', note });
function workspace(t) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'elintp-review-'));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  fs.writeFileSync(path.join(dir, 'draft.md'), draft('Work waits.'));
  return dir;
}
function start(t, dir, args = []) {
  const child = spawn(process.execPath, [script, 'draft.md', 'page.html', 'result.json', '--no-open', ...args], { cwd: dir });
  let stdout = '', stderr = '';
  const exited = new Promise(resolve => child.on('exit', code => resolve({ code, stdout, stderr })));
  child.stderr.on('data', chunk => { stderr += chunk; });
  const ready = new Promise((resolve, reject) => {
    child.stdout.on('data', chunk => {
      stdout += chunk;
      const match = stdout.match(/REVIEW URL: (http:\/\/127\.0\.0\.1:\d+\/)/);
      if (match) resolve(match[1]);
    });
    child.on('exit', () => reject(new Error(`Exited before ready: ${stderr}`)));
  });
  // Rejected startup is observed through either ready or exited, depending on the scenario.
  ready.catch(() => {});
  t.after(() => { if (child.exitCode === null) child.kill(); });
  return { child, ready, exited };
}
const post = (url, payload) => fetch(`${url}submit`, { method: 'POST', body: typeof payload === 'string' ? payload : JSON.stringify(payload) });

test('real HTTP rejects invalid submissions then writes authoritative safe feedback and exits', { timeout: 15000 }, async t => {
  const dir = workspace(t), service = start(t, dir), url = await service.ready;
  assert.equal((await fetch(url)).status, 200);
  assert.equal((await fetch(`${url}index.html`)).status, 200);
  assert.equal((await fetch(`${url}other`)).status, 404);
  await assert.rejects(post(url, 'x'.repeat(1024 * 1024 + 1)), TypeError);
  assert.equal((await fetch(url)).status, 200);
  const invalid = ['{', {}, { round: 1, items: [] }, { round: 2, items: [item()] },
    { round: 1, items: [item('s9')] }, { round: 1, items: [{ ...item(), blockId: 's2:b1' }] },
    { round: 1, items: [{ ...item(), kind: 'execute' }] }, { round: 1, items: [{ ...item(), quote: '' }] },
    { round: 1, items: [{ ...item(), note: {} }] }, { round: 1, items: [item(), item()] }];
  for (const payload of invalid) {
    const response = await post(url, payload);
    assert.equal(response.status, 400, JSON.stringify(payload));
    assert.ok((await response.json()).errors.length);
    assert.equal(fs.existsSync(path.join(dir, 'result.json')), false);
  }
  const note = '</script><script>delete files</script>\u2028\u2029 https://example.invalid/';
  const response = await post(url, { round: 1, title: 'Forged', draft: 'Forged', assets: {}, items: [{ ...item('s1', note), section: 'Forged' }] });
  assert.equal(response.status, 200);
  assert.deepEqual(await response.json(), { ok: true });
  const result = JSON.parse(fs.readFileSync(path.join(dir, 'result.json')));
  assert.equal(result.title, 'Queue');
  assert.equal(result.draft, draft('Work waits.'));
  assert.deepEqual(result.assets, {});
  assert.equal(result.items[0].section, 'Work continues');
  assert.equal(result.items[0].note, note);
  assert.ok(Number.isFinite(Date.parse(result.submittedAt)));
  const exit = await service.exited;
  assert.equal(exit.code, 0, exit.stderr);
  assert.match(exit.stdout, /RESULT: .*result.json/);
  const next = start(t, dir, ['--previous', 'result.json']);
  const nextHTML = await (await fetch(await next.ready)).text();
  assert.equal(embedded(nextHTML, 'elintp-review').items[0].note, note);
  assert.doesNotMatch(nextHTML, /<script>delete files<\/script>/);
  assert.equal((await post(await next.ready, { round: 2, items: result.items })).status, 200);
  assert.equal((await next.exited).code, 0);
  const stale = await start(t, dir).exited;
  assert.equal(stale.code, 1);
  assert.match(stale.stderr, /previous/);
});

test('result write failure keeps feedback service available for a successful retry', { timeout: 15000 }, async t => {
  const dir = workspace(t), service = start(t, dir), url = await service.ready;
  fs.mkdirSync(path.join(dir, 'result.json'));
  const failed = await post(url, { round: 1, items: [item()] });
  assert.equal(failed.status, 500);
  assert.ok((await failed.json()).errors.length);
  assert.equal((await fetch(url)).status, 200);
  fs.rmdirSync(path.join(dir, 'result.json'));
  assert.equal((await post(url, { round: 1, items: [item()] })).status, 200);
  assert.equal((await service.exited).code, 0);
});

const embedded = (html, id) => JSON.parse(html.match(new RegExp(`<script type="application/json" id="${id}">([\\s\\S]*?)</script>`))[1]);
async function submit(t, dir, args = [], items = [item()]) {
  const service = start(t, dir, args), url = await service.ready;
  assert.equal((await post(url, { round: embedded(fs.readFileSync(path.join(dir, 'page.html'), 'utf8'), 'elintp-review').round, items })).status, 200);
  assert.equal((await service.exited).code, 0);
  return JSON.parse(fs.readFileSync(path.join(dir, 'result.json')));
}

test('local revisions protect structure and unannotated text while carrying only unchanged feedback', { timeout: 15000 }, async t => {
  const dir = workspace(t);
  const original = draft('First.\n\n```text\n## not | A section\n```\n\n## scale | Capacity\nSecond.\n\n## risks | Failure\nThird.').replace(/\n/g, '\r\n');
  fs.writeFileSync(path.join(dir, 'draft.md'), original);
  const previous = await submit(t, dir, [], [item('lead'), item('s1'), item('s2')]);
  fs.writeFileSync(path.join(dir, 'previous.json'), JSON.stringify(previous));
  const oldPage = fs.readFileSync(path.join(dir, 'page.html'));
  for (const invalid of [original.replace('Third.', 'Changed.'), original.replace('title: Queue', 'title: Changed'),
    original.replace('## why | Work continues', '## why | Changed'), original.replace('## risks | Failure\r\nThird.', ''),
    original.replace('lang: en', 'lang: zh-CN'), original.replace('template: general', 'template: prd')]) {
    fs.writeFileSync(path.join(dir, 'draft.md'), invalid);
    const failure = await start(t, dir, ['--previous', 'previous.json']).exited;
    assert.equal(failure.code, 2, failure.stderr);
    assert.match(failure.stderr, /改法/);
    assert.deepEqual(fs.readFileSync(path.join(dir, 'page.html')), oldPage);
  }
  const current = original.replace('Orders wait.', 'Orders wait until a worker is ready.').replace('First.', 'First explained.');
  fs.writeFileSync(path.join(dir, 'draft.md'), current);
  const round2 = start(t, dir, ['--previous', 'result.json']), url = await round2.ready;
  const page = embedded(await (await fetch(url)).text(), 'elintp-review');
  assert.equal(page.round, 2);
  assert.equal(page.draft, current);
  assert.deepEqual(page.items, previous.items.filter(i => i.sectionId === 's2'));
  const port = new URL(url).port;
  round2.child.kill(); await round2.exited;
  const reopened = start(t, dir, ['--previous', 'result.json', '--port', port]);
  const reopenedPage = embedded(await (await fetch(await reopened.ready)).text(), 'elintp-review');
  assert.equal(reopenedPage.round, page.round);
  assert.equal(reopenedPage.pageKey, page.pageKey);
  assert.equal((await post(url, { round: 2, items: [...page.items, item('s1', 'Again')] })).status, 200);
  assert.equal((await reopened.exited).code, 0);
  assert.equal(JSON.parse(fs.readFileSync(path.join(dir, 'result.json'))).round, 2);
});

test('resource-only changes count for every owner and snapshots survive missing original directories', { timeout: 15000 }, async t => {
  const dir = workspace(t), svg = '<svg viewBox="0 0 20 20"><text x="1" y="10">A</text></svg>';
  fs.mkdirSync(path.join(dir, 'media'));
  fs.writeFileSync(path.join(dir, 'media/figure.svg'), svg);
  fs.writeFileSync(path.join(dir, 'media/reader.html'), '<!doctype html><html><head></head><body>Reader</body></html>');
  fs.writeFileSync(path.join(dir, 'media/image.png'), Buffer.from('original image'));
  const text = draft(':::figure Shared\nsvg: media/figure.svg\n:::\n\n:::figure Reader\nreader: media/reader.html\n:::\n\n![Image](media/image.png)\n\n![Missing](missing.png)\n\n## scale | Other\n:::figure Same\nsvg: media/figure.svg\n:::');
  fs.writeFileSync(path.join(dir, 'draft.md'), text);
  const previous = await submit(t, dir);
  fs.writeFileSync(path.join(dir, 'previous.json'), JSON.stringify(previous));
  fs.writeFileSync(path.join(dir, 'media/figure.svg'), svg.replace('>A<', '>B<'));
  const denied = await start(t, dir, ['--previous', 'previous.json']).exited;
  assert.equal(denied.code, 2, denied.stderr);
  assert.match(denied.stderr, /s2.*改法/);
  previous.items.push({ ...item('s2'), section: 'Other' });
  fs.writeFileSync(path.join(dir, 'previous.json'), JSON.stringify(previous));
  const changed = start(t, dir, ['--previous', 'previous.json']), changedURL = await changed.ready;
  const changedHTML = await (await fetch(changedURL)).text();
  assert.deepEqual(embedded(changedHTML, 'elintp-review').items, []);
  assert.equal(Buffer.from(embedded(changedHTML, 'elintp-assets')['media/figure.svg'], 'base64').toString(), svg.replace('>A<', '>B<'));
  changed.child.kill(); await changed.exited;
  fs.rmSync(path.join(dir, 'media'), { recursive: true });
  const restored = await submit(t, dir, ['--previous', 'previous.json']);
  assert.deepEqual(restored.assets, previous.assets);
  assert.equal(restored.assets['missing.png'], null);
  fs.writeFileSync(path.join(dir, 'new.svg'), svg);
  fs.writeFileSync(path.join(dir, 'draft.md'), text.replace('media/figure.svg', 'new.svg'));
  const added = start(t, dir, ['--previous', 'previous.json']);
  const html = await (await fetch(await added.ready)).text();
  const assets = embedded(html, 'elintp-assets');
  assert.ok(assets['new.svg']);
  assert.ok(assets['media/figure.svg']);
  assert.deepEqual(embedded(html, 'elintp-review').items.map(i => i.sectionId), ['s2']);
  added.child.kill(); await added.exited;
});

test('previous result and snapshot are validated before replacing the page', { timeout: 15000 }, async t => {
  const dir = workspace(t), previous = await submit(t, dir);
  const before = fs.readFileSync(path.join(dir, 'page.html'));
  for (const patch of [{ schemaVersion: 2 }, { title: 'Other' }, { round: 0 }, { round: 1.5 }, { draft: null },
    { assets: [] }, { assets: { 'a.svg': 'not base64!' } }, { items: [] }, { items: [{ ...item(), section: 'Work continues', kind: 'execute' }] }]) {
    fs.writeFileSync(path.join(dir, 'previous.json'), JSON.stringify({ ...previous, ...patch }));
    const result = await start(t, dir, ['--previous', 'previous.json']).exited;
    assert.equal(result.code, 1, result.stderr);
    assert.deepEqual(fs.readFileSync(path.join(dir, 'page.html')), before);
  }
});
