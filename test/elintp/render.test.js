'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const script = path.resolve(__dirname, '../../skills/elintp/scripts/render.mjs');
const draft = (body, meta = '') => `---\ntemplate: general\ntitle: Queue\nlang: en\n${meta}---\nOrders wait in a queue.\n\n## why | Work continues after submission\n${body}\n`;
function workspace(t) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'elintp-'));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  const output = path.join(dir, 'output.html');
  return { dir, output, run(text) {
    fs.writeFileSync(path.join(dir, 'draft.md'), text);
    return spawnSync(process.execPath, [script, 'draft.md', output], { cwd: dir, encoding: 'utf8' });
  } };
}
test('format errors report lines and repairs together without touching an existing page', t => {
  const w = workspace(t);
  assert.equal(w.run(draft('Delivered.')).status, 0);
  const before = fs.readFileSync(w.output);
  const cases = [
    ['frontmatter', draft('Text.', 'unknown: value\n'), /L5:.*frontmatter/],
    ['lead', draft('Text.').replace('Orders wait in a queue.', ''), /L\d+:.*引言/],
    ['slot', draft('Text.').replace('why |', 'unknown |'), /L8:.*slot/],
    ['table', draft('| A | B |\n| --- | --- |\n| 1 | 2 | 3 |'), /L11:.*列/],
    ['link', draft('[Run](javascript:alert(1))'), /L9:.*协议/],
    ['image', draft('![Queue](https://example.com/a.png)'), /L9:.*本地/],
    ['container', draft(':::metrics\n3 | only two\n:::'), /L10:.*三/],
    ['figure', draft(':::figure Queue\nsvg: absent.svg\n:::'), /L10:.*读取/],
  ];
  for (const [label, text, pattern] of cases) {
    const result = w.run(text);
    assert.equal(result.status, 2, label + result.stderr);
    assert.match(result.stderr, pattern, label);
    assert.ok(result.stderr.trim().split('\n').every(line => /^L\d+: .+。改法：.+/.test(line)), label);
    assert.deepEqual(fs.readFileSync(w.output), before, label);
  }
  const two = w.run(draft('| A | B |\n| --- | --- |\n| 1 | 2 | 3 |\n\n## unknown | Other\nText.'));
  assert.equal(two.stderr.trim().split('\n').length, 2);
  assert.match(two.stderr, /^L11:.*\nL13:/);
});

test('invalid plan progress remains a format error', t => {
  const w = workspace(t);
  const result = w.run(draft('Text.', 'progress: constructor\n').replace('template: general', 'template: plan'));
  assert.equal(result.status, 2, result.stderr);
  assert.match(result.stderr, /progress.*改法/);
  assert.equal(fs.existsSync(w.output), false);
});

test('SVG rejects executable elements, handlers, style and external resources', t => {
  const w = workspace(t);
  const attacks = [
    '<script>alert(1)</script>', '<g onload="alert(1)"/>', '<foreignObject/>',
    '<use href="https://example.com/a.svg#x"/>', '<rect fill="url(https://example.com/a.svg#x)"/>',
    '<rect style="fill:red"/>', '<use href="&#106;avascript:alert(1)"/>',
    '<rect fill="url(&#104;ttps://example.com/a.svg)"/>',
    '<g xmlns="http://www.w3.org/1999/xhtml"><script/></g>',
  ];
  for (const attack of attacks) {
    fs.writeFileSync(path.join(w.dir, 'figure.svg'), `<svg viewBox="0 0 10 10">\n${attack}\n</svg>`);
    const result = w.run(draft(':::figure Queue\nsvg: figure.svg\n:::'));
    assert.equal(result.status, 2, attack + result.stderr);
    assert.match(result.stderr, /L10:.*SVG.*L2.*改法/);
    assert.equal(fs.existsSync(w.output), false);
  }
});

test('all components render safely and reproducibly with embedded local assets', t => {
  const w = workspace(t);
  fs.writeFileSync(path.join(w.dir, 'figure.svg'), '<?xml version="1.0"?>\n<!-- note -->\n<svg viewBox="0 0 120 40"><defs><marker id="arrow"><path d="M0 0 L4 2 L0 4"/></marker></defs><path d="M0 20 H120" stroke="currentColor" marker-end="url(#arrow)"/><text x="0" y="12" class="f-fg">Queue</text></svg>');
  fs.writeFileSync(path.join(w.dir, 'image.png'), Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aS1cAAAAASUVORK5CYII=', 'base64'));
  const input = draft('<script>alert(1)</script> <img src=x onerror=alert(1)>\n\n:::conclusion Result\n**Orders** wait.\n- Parent\n  - Child\n:::\n\n:::reminder Limit\nCapacity is finite.\n:::\n\n:::metrics\n200ms | Wait | Before processing\n:::\n\n| State :: Result of check | Limit |\n| --- | --- |\n| [ok] Ready | [warn] Limited |\n| [no] Blocked | 10 |\n\n1. Submit\n2. Wait\n\n![A local pixel](image.png)\n\n![Missing sample](missing.png)\n\n:::figure Queue flow\nsvg: figure.svg\nA local diagram.\n:::\n\n```html\n<div>literal</div>\n```');
  const result = w.run(input);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout, `PAGE: ${w.output}\n`);
  assert.match(result.stderr, /^WARN L\d+:/);
  const html = fs.readFileSync(w.output, 'utf8');
  for (const pattern of [/&lt;script&gt;alert\(1\)&lt;\/script&gt;/, /&lt;img src=x onerror=alert\(1\)&gt;/, /data:image\/png;base64,/, /role="img" aria-label="Queue flow"/, /Image missing.*missing.png/, /✓.*Ready/, /✗.*Blocked/, /!.*Limited/, /Column notes/, /200ms/, /<ol class="steps">/, /&lt;div&gt;literal/]) assert.match(html, pattern);
  assert.doesNotMatch(html, /<script>alert|<img src=x|<\?xml|<!-- note -->/);
  assert.equal(w.run(input).status, 0);
  assert.equal(fs.readFileSync(w.output, 'utf8'), html);
});

test('figures that reuse SVG ids keep their own definitions', t => {
  const w = workspace(t);
  const figure = shape => `<svg viewBox="0 0 40 40" xmlns:xlink="http://www.w3.org/1999/xlink" aria-labelledby="s1"><title id="s1">Shape</title><defs>${shape}<linearGradient id="paint"><stop offset="0"/></linearGradient><marker id="tip"><path d="M0 0 L4 2 L0 4"/></marker><clipPath id="clip"><rect width="40" height="40"/></clipPath></defs><g clip-path="url(#clip)"><use href="#shape" fill="url(#paint)"/><use xlink:href="#shape"/><path d="M0 20 H40" marker-end="url(#tip)"/></g></svg>`;
  fs.writeFileSync(path.join(w.dir, 'square.svg'), figure('<rect id="shape" width="20" height="20"/>'));
  fs.writeFileSync(path.join(w.dir, 'triangle.svg'), figure('<polygon id="shape" points="10,0 20,20 0,20"/>'));
  const input = draft(':::figure Square\nsvg: square.svg\n:::\n\n:::figure Triangle\nsvg: triangle.svg\n:::');
  const result = w.run(input);
  assert.equal(result.status, 0, result.stderr);
  const html = fs.readFileSync(w.output, 'utf8');
  const idsIn = text => [...text.matchAll(/ id="([^"]+)"/g)].map(match => match[1]);
  assert.equal(new Set(idsIn(html)).size, idsIn(html).length, idsIn(html).join(' '));
  const figures = html.match(/<svg[\s\S]*?<\/svg>/g);
  assert.match(figures[0], /<rect id=/);
  assert.match(figures[1], /<polygon id=/);
  for (const svg of figures) {
    const own = new Set(idsIn(svg));
    const references = [...svg.matchAll(/href="#([^"]+)"|url\(#([^)]+)\)|aria-labelledby="([^"]+)"/g)].map(match => match[1] || match[2] || match[3]);
    assert.equal(references.length, 6);
    for (const reference of references) assert.ok(own.has(reference), reference);
  }
  assert.equal(w.run(input).status, 0);
  assert.equal(fs.readFileSync(w.output, 'utf8'), html);
});

const embedded = (html, id) => JSON.parse(html.match(new RegExp(`<script type="application/json" id="${id}">([\\s\\S]*?)</script>`))[1]);

test('p2 final content and resource snapshots round-trip safely without original files', async t => {
  const w = workspace(t), text = draft('</script><script>literal</script> \u2028\u2029\n\n![Pixel](pixel.png)\n\n![Missing](absent.png)\n\n:::figure Reader\nreader: reader.html\n:::');
  const assets = { 'pixel.png': Buffer.from([0, 1, 2, 255]).toString('base64'), 'absent.png': null,
    'reader.html': Buffer.from('<!doctype html><html><head></head><body>Reader original</body></html>').toString('base64') };
  const { renderPage } = await import(script);
  const review = renderPage(text, { cwd: w.dir, assets, review: { round: 2, pageKey: 'key', items: [] } });
  fs.writeFileSync(w.output, review.html);
  fs.writeFileSync(path.join(w.dir, 'draft.md'), text);
  // Same input/output HTML path proves the snapshot is consumed before replacement.
  const result = spawnSync(process.execPath, [script, 'draft.md', w.output, '--assets-from', w.output], { cwd: w.dir, encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
  const html = fs.readFileSync(w.output, 'utf8');
  assert.equal(embedded(html, 'elintp-draft'), text);
  assert.deepEqual(embedded(html, 'elintp-assets'), assets);
  assert.match(html, /data:image\/png;base64,AAEC\/w==/);
  assert.match(html, /Reader original/);
  assert.doesNotMatch(html, /<script>literal<\/script>/);
  const old = html;
  fs.writeFileSync(path.join(w.dir, 'bad.json'), JSON.stringify({ assets: { 'pixel.png': 'bad!' } }));
  const bad = spawnSync(process.execPath, [script, 'draft.md', w.output, '--assets-from', 'bad.json'], { cwd: w.dir, encoding: 'utf8' });
  assert.equal(bad.status, 1);
  assert.equal(fs.readFileSync(w.output, 'utf8'), old);
});

test('p2 final and review use identical reading content with isolated mode resources', async t => {
  const w = workspace(t), text = draft('Current content.\n\n## scale | Scale\n200 tasks.\n\n## risks | Risk\nCapacity.', 'source: Original\n');
  const { renderPage } = await import(script);
  const review = renderPage(text, { cwd: w.dir, review: { round: 2, pageKey: 'private-key', items: [{ note: 'PRIVATE_FEEDBACK' }] } }).html;
  const final = renderPage(text, { cwd: w.dir }).html;
  for (const tag of ['header', 'main', 'footer']) {
    const pattern = new RegExp(`<${tag}[^>]*>[\\s\\S]*?</${tag}>`);
    assert.equal(final.match(pattern)[0], review.match(pattern)[0]);
  }
  assert.equal(embedded(review, 'elintp-review').draft, text);
  assert.doesNotMatch(review, /id="elintp-draft"/);
  assert.match(review, /fetch\('\/submit'/);
  assert.doesNotMatch(final, /elintp-review|review-toolbar|\/submit|PRIVATE_FEEDBACK|private-key/);
  assert.match(final, /theme-toggle/);
  assert.match(final, /class="outline"/);
});

test('p2 final output is deterministic and snapshot bytes override current files', async t => {
  const w = workspace(t), text = draft('![Pixel](pixel.png)'), image = Buffer.from('reviewed bytes');
  const assets = { 'pixel.png': image.toString('base64') };
  fs.writeFileSync(path.join(w.dir, 'pixel.png'), 'changed outside review');
  fs.writeFileSync(path.join(w.dir, 'draft.md'), text);
  fs.writeFileSync(path.join(w.dir, 'result.json'), JSON.stringify({ assets }));
  const { renderPage } = await import(script);
  const expected = renderPage(text, { cwd: w.dir, assets }).html;
  for (const source of ['result.json', 'snapshot.html']) {
    fs.writeFileSync(path.join(w.dir, 'snapshot.html'), expected);
    const result = spawnSync(process.execPath, [script, 'draft.md', w.output, '--assets-from', source], { cwd: w.dir, encoding: 'utf8' });
    assert.equal(result.status, 0, result.stderr);
    assert.equal(fs.readFileSync(w.output, 'utf8'), expected);
  }
});
