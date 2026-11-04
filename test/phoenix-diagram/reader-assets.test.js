'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const vm = require('node:vm');
const { createHash } = require('node:crypto');

const root = path.resolve(__dirname, '../..');
const workspace = path.join(root, 'tools/phoenix-diagram');
const modulePath = '../../scripts/assemble-phoenix-diagram-reader.mjs';
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const slots = ['LOCALE', 'PRESET', 'TITLE', 'HEADING', 'SUBTITLE', 'SVG', 'CARDS', 'METADATA'];
const read = file => fs.readFileSync(path.join(workspace, file), 'utf8');

function sandbox(t) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-reader-assets-'));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  fs.cpSync(path.join(workspace, 'viewer'), path.join(directory, 'viewer'), { recursive: true });
  fs.mkdirSync(path.join(directory, 'assets'));
  for (const file of ['template.html', 'catalogs.json']) {
    fs.copyFileSync(path.join(workspace, 'assets', file), path.join(directory, 'assets', file));
  }
  return directory;
}

test('reader is a complete offline Phoenix artifact with exactly eight native slots', () => {
  const html = read('assets/template.html');
  assert.deepEqual([...html.matchAll(/\{\{PHOENIX:([^}]+)\}\}/g)].map(match => match[1]).sort(), [...slots].sort());
  assert.equal((html.match(/id="phoenix-diagram-metadata"/g) || []).length, 1);
  assert.match(html, /window\.PhoenixDiagram\s*=/);
  assert.match(html, /name="generator" content="phoenix-diagram 0\.2\.0"/);
  assert.doesNotMatch(html, /\{\{i18n:|\[PROJECT NAME\]|\bvar Archify\b|localStorage[^\n]*archify-/);
  assert.doesNotMatch(html, /<(?:script|link)\b[^>]*(?:src|href)\s*=\s*["'](?:https?:|\/\/)/i);
  assert.doesNotMatch(html, /@import\s|url\(["']?https?:|\b(?:fetch|XMLHttpRequest|WebSocket|EventSource|sendBeacon)\s*\(/);
  assert.doesNotMatch(html, /\.innerHTML\s*=|\.outerHTML\s*=|insertAdjacentHTML\s*\(/);
});

test('assembly is deterministic, checks without writes, and rejects stale output', async t => {
  const { assembleReader } = await import(modulePath);
  const directory = sandbox(t);
  const files = ['template.html', 'catalogs.json'];
  const before = files.map(file => hash(fs.readFileSync(path.join(directory, 'assets', file))));
  const checked = assembleReader({ workspace: directory, check: true });
  assert.deepEqual(checked.assets.map(asset => [asset.path, asset.sha256]), files.map((file, index) => [`assets/${file}`, before[index]]));
  assert.deepEqual(files.map(file => hash(fs.readFileSync(path.join(directory, 'assets', file)))), before);
  fs.appendFileSync(path.join(directory, 'assets/template.html'), '\nchanged');
  assert.throws(() => assembleReader({ workspace: directory, check: true }), /stale/i);
  assembleReader({ workspace: directory });
  assert.deepEqual(files.map(file => hash(fs.readFileSync(path.join(directory, 'assets', file)))), before);
});

test('assembly rejects missing, duplicate, unknown and wrongly nested fragments before writing', async t => {
  const { assembleReader } = await import(modulePath);
  for (const mutation of ['missing', 'duplicate', 'unknown', 'nested']) {
    const directory = sandbox(t);
    const template = path.join(directory, 'viewer/template.source.html');
    const marker = '/* PHOENIX:EXPORT */';
    if (mutation === 'missing') fs.writeFileSync(template, fs.readFileSync(template, 'utf8').replace(marker, ''));
    if (mutation === 'duplicate') fs.appendFileSync(template, '\n' + marker);
    if (mutation === 'unknown') fs.appendFileSync(template, '\n/* PHOENIX:UNRECOGNIZED */');
    if (mutation === 'nested') fs.appendFileSync(path.join(directory, 'viewer/focus.js'), '\n/* PHOENIX:CAMERA */');
    const before = hash(fs.readFileSync(path.join(directory, 'assets/template.html')));
    assert.throws(() => assembleReader({ workspace: directory }), /marker|fragment/i, mutation);
    assert.equal(hash(fs.readFileSync(path.join(directory, 'assets/template.html'))), before);
  }
});

test('classic script order and private canonical/recorder cleanup survive assembly', () => {
  const html = read('assets/template.html');
  const order = ['PhoenixDiagram.preset =', 'PhoenixDiagram.theme =', 'PhoenixDiagram.motion =', 'PhoenixDiagram.motionGovernor =', 'PhoenixDiagram.sourceEvidence =', 'PhoenixDiagram.focus =', 'PhoenixDiagram.intentTrace =', 'PhoenixDiagram.readerLayout =', 'PhoenixDiagram.viewerChromeLayout =', 'PhoenixDiagram.view =', 'PhoenixDiagram.radar =', 'PhoenixDiagram.presentation =', 'PhoenixDiagram.finder =', 'PhoenixDiagram.routeProbe =', 'PhoenixDiagram.semanticLens ='];
  let previous = -1;
  for (const item of order) {
    const at = html.indexOf(item);
    assert.ok(at > previous, `${item} must initialize after its dependencies`);
    previous = at;
  }
  assert.match(html, /function cleanExportClone\(clone\)/);
  assert.match(html, /canonicalStateClean\s*=\s*cleanExportClone\(clone\)/);
  assert.doesNotMatch(html, /PhoenixDiagram\.cleanExportClone/);
  for (const script of html.matchAll(/<script([^>]*)>([\s\S]*?)<\/script>/g)) {
    if (!/application\/json/.test(script[1])) new vm.Script(script[2]);
  }
});

test('embedded font bytes, full OFL and source MIT notices survive assembly and SVG export assets', () => {
  const source = read('viewer/template.source.html');
  const html = read('assets/template.html');
  const fonts = text => [...text.matchAll(/data:font\/woff2;base64,([A-Za-z0-9+/=]+)/g)].map(match => hash(Buffer.from(match[1], 'base64')));
  assert.equal(fonts(source).length, 6);
  assert.deepEqual(fonts(html), fonts(source));
  assert.match(html, /Copyright \(c\) 2026 tt-a1i \(Archify\)/);
  assert.match(html, /Copyright \(c\) 2025 Cocoon AI/);
  assert.match(html, /SIL OPEN FONT LICENSE Version 1\.1 - 26 February 2007/);
  assert.match(html, /THE FONT SOFTWARE IS PROVIDED "AS IS"/);
  assert.match(read('assets/svg-definitions.svg'), /id="arrowhead-dashed"/);
  assert.match(read('assets/node-card-style.svg'), /svg\[data-node-ui\]/);
});

test('motion reads only Phoenix storage and first readers remain Still until Live is stored', () => {
  const motion = read('viewer/motion-governor.js');
  const key = motion.match(/var STORAGE_KEY = '([^']+)'/)[1];
  assert.equal(key, 'phoenix-diagram-motion');
  const readStored = motion.match(/function readStored\(\) \{[\s\S]*?\n {6}\}/)[0];
  const intent = motion.match(/readerPaused = (readStored\(\) !== 'live');/)[1];
  for (const [stored, paused] of [[null, true], ['still', true], ['unknown', true], ['live', false]]) {
    const seen = [];
    assert.equal(vm.runInNewContext(readStored + '\n' + intent, { STORAGE_KEY: key, localStorage: { getItem(name) { seen.push(name); return stored; } } }), paused);
    assert.deepEqual(seen, [key]);
  }
  assert.equal(vm.runInNewContext(readStored + '\n' + intent, { STORAGE_KEY: key, localStorage: { getItem() { throw new Error('denied'); } } }), true);
});

test('native metadata locale adaptation keeps author strings in safe text and attribute sinks', () => {
  const html = read('assets/template.html');
  const start = html.indexOf('    var PhoenixDiagram = window.PhoenixDiagram = {};');
  const bootstrap = html.slice(start, html.indexOf('    function viewerText(', start));
  assert.ok(start > 0 && bootstrap.length > 1000);
  const english = JSON.parse(read('viewer/locales/en.json'));
  const chinese = JSON.parse(read('viewer/locales/zh-CN.json'));
  const key = 'viewer.theme.dark';
  const hostile = '</script><img src=x onerror=alert(1)>';
  for (const [meta, expected, locale] of [
    [{ locale: 'zh-CN' }, chinese[key], 'zh-CN'],
    [{ translations: { [key]: hostile } }, english[key], 'en'],
    [{ locale: 'en', translations: { [key]: hostile } }, hostile, 'en'],
    [{ locale: 'en', translations: { [key]: '{1}' } }, english[key], 'en'],
    [{ locale: 'fr', translations: { [key]: 'Personnalisé' } }, 'Personnalisé', 'fr'],
  ]) {
    const text = { getAttribute() { return key; }, textContent: '' };
    Object.defineProperty(text, 'innerHTML', { set() { throw new Error('unsafe HTML sink'); } });
    const attributes = {};
    const control = { attributes: [{ name: 'data-phoenix-i18n-aria-label', value: key }], setAttribute(name, value) { attributes[name] = value; } };
    const rootAttributes = {};
    const document = {
      getElementById(id) { assert.equal(id, 'phoenix-diagram-metadata'); return { textContent: JSON.stringify({ meta, graph: { nodes: [{ label: hostile }] } }) }; },
      documentElement: { setAttribute(name, value) { rootAttributes[name] = value; } },
      querySelectorAll(selector) { return selector === '[data-phoenix-i18n]' ? [text] : [control]; },
    };
    const context = { window: {}, document };
    vm.runInNewContext(bootstrap, context);
    assert.equal(text.textContent, expected);
    assert.equal(attributes['aria-label'], expected);
    assert.equal(context.window.PhoenixDiagram.locale, locale);
    assert.equal(rootAttributes.lang, locale);
    assert.equal(context.window.PhoenixDiagram.metadata.graph.nodes[0].label, hostile);
    assert.equal(context.phoenixDiagramI18nData.messages['viewer.export.downloadPngInstead'],
      meta.locale === 'zh-CN' ? chinese['viewer.export.downloadPngInstead'] : english['viewer.export.downloadPngInstead']);
  }
});

test('template and native renderer read one normalized five-language catalog', async t => {
  const { assembleReader } = await import(modulePath);
  const catalogs = JSON.parse(read('assets/catalogs.json'));
  const embedded = /var phoenixDiagramBundledCatalogs = (\{.*?\});\n/s.exec(read('assets/template.html'))?.[1];
  assert.deepEqual(JSON.parse(embedded), catalogs);
  assert.deepEqual(Object.keys(catalogs), ['en', 'zh-CN', 'zh-TW', 'es', 'ko']);
  assert.equal(catalogs.en['viewer.export.card.routeBadge'], 'PHOENIX DIAGRAM · ROUTE · {hops}');
  assert.equal(catalogs.ko['viewer.preset.badge.editorialPlate'], 'PHOENIX DIAGRAM / 도판 04');
  for (const [locale, catalog] of Object.entries(catalogs)) {
    assert.deepEqual(Object.keys(catalog).sort(), Object.keys(catalogs.en).sort(), locale);
    assert.ok(Object.values(catalog).every(message => !/archify/i.test(message)), locale);
  }
  for (const [mutation, locale] of [['archify', 'en'], ['placeholder', 'es'], ['missing', 'ko']]) {
    const directory = sandbox(t), file = path.join(directory, `viewer/locales/${locale}.json`);
    const catalog = JSON.parse(fs.readFileSync(file, 'utf8'));
    if (mutation === 'archify') catalog['timeline.axis'] = 'Archify clock';
    if (mutation === 'placeholder') catalog['timeline.precision.day'] = 'solo {fecha}';
    if (mutation === 'missing') delete catalog['timeline.precision.day'];
    fs.writeFileSync(file, JSON.stringify(catalog, null, 2) + '\n');
    assert.throws(() => assembleReader({ workspace: directory, check: true }), /Archify|placeholders|key set/, mutation);
  }
});
