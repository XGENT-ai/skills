'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const vm = require('node:vm');
const { createHash } = require('node:crypto');
const { gunzipSync } = require('node:zlib');

const root = path.resolve(__dirname, '../..');
const workspace = path.join(root, 'tools/phoenix-diagram');
const modulePath = '../../scripts/assemble-phoenix-diagram-reader.mjs';
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const slots = ['LOCALE', 'PRESET', 'TITLE', 'HEADING', 'SUBTITLE', 'SVG', 'CARDS', 'METADATA'];
const read = file => fs.readFileSync(path.join(workspace, file), 'utf8');

function frozenMember(name) {
  const archive = gunzipSync(fs.readFileSync(path.join(workspace, 'upstream/archify-3.1.0-source.tar.gz')));
  for (let at = 0; at + 512 <= archive.length && archive[at] !== 0;) {
    const header = archive.subarray(at, at + 512);
    const field = (start, size) => header.subarray(start, start + size).toString().replace(/\0.*$/s, '');
    const prefix = field(345, 155);
    const member = (prefix ? prefix + '/' : '') + field(0, 100);
    const size = parseInt(field(124, 12), 8);
    if (member === name) return archive.subarray(at + 512, at + 512 + size).toString();
    at += 512 + Math.ceil(size / 512) * 512;
  }
  throw new Error(`Missing frozen test source ${name}`);
}

function sandbox(t) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-reader-assets-'));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  fs.cpSync(path.join(workspace, 'viewer'), path.join(directory, 'viewer'), { recursive: true });
  fs.mkdirSync(path.join(directory, 'assets'));
  for (const file of ['template.html', 'catalogs.json', 'svg-definitions.svg', 'node-card-style.svg']) {
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
  const files = ['template.html', 'catalogs.json', 'svg-definitions.svg', 'node-card-style.svg'];
  const before = files.map(file => hash(fs.readFileSync(path.join(directory, 'assets', file))));
  assembleReader({ workspace: directory, check: true });
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

test('assembly rejects Reader Layout changes outside its two audited narrow hunks before writing', async t => {
  const { assembleReader } = await import(modulePath);
  for (const mutation of ['append', 'prefix', 'manifest', 'readability-floor']) {
    const directory = sandbox(t), reader = path.join(directory, 'viewer/reader-layout.js');
    if (mutation === 'append') fs.appendFileSync(reader, '\nPhoenixDiagram.unrecordedReaderChange = true;\n');
    if (mutation === 'prefix') fs.writeFileSync(reader, 'PhoenixDiagram.unrecordedReaderChange = true;\n' + fs.readFileSync(reader, 'utf8'));
    if (mutation === 'manifest') fs.appendFileSync(path.join(directory, 'viewer/reader-layout-adaptations.json'), '\n');
    if (mutation === 'readability-floor') fs.writeFileSync(reader, fs.readFileSync(reader, 'utf8').replace('MIN_PROJECTED_NODE_TEXT_PX = 6;', 'MIN_PROJECTED_NODE_TEXT_PX = 1;'));
    const before = hash(fs.readFileSync(path.join(directory, 'assets/template.html')));
    assert.throws(() => assembleReader({ workspace: directory }), /Reader Layout.*(?:source|adaptation|frozen)/i, mutation);
    assert.equal(hash(fs.readFileSync(path.join(directory, 'assets/template.html'))), before);
  }
});

test('assembly rejects Chrome Layout changes outside its audited lift and screen-measurement hunks before writing', async t => {
  const { assembleReader } = await import(modulePath);
  for (const mutation of ['append', 'prefix', 'manifest', 'stage-boundary', 'virtual-dock', 'virtual-receipt']) {
    const directory = sandbox(t), chrome = path.join(directory, 'viewer/viewer-chrome-layout.js');
    if (mutation === 'append') fs.appendFileSync(chrome, '\nPhoenixDiagram.unrecordedChromeChange = true;\n');
    if (mutation === 'prefix') fs.writeFileSync(chrome, 'PhoenixDiagram.unrecordedChromeChange = true;\n' + fs.readFileSync(chrome, 'utf8'));
    if (mutation === 'manifest') fs.appendFileSync(path.join(directory, 'viewer/chrome-layout-adaptations.json'), '\n');
    if (mutation === 'stage-boundary') fs.writeFileSync(chrome, fs.readFileSync(chrome, 'utf8').replace('rest.top - stage.bottom - SAFE_GAP', 'rest.top - stage.bottom + SAFE_GAP'));
    if (mutation === 'virtual-dock') fs.writeFileSync(chrome, fs.readFileSync(chrome, 'utf8').replace('dockRect: function () { return nav ? nav.getBoundingClientRect()', 'dockRect: function () { return nav ? restingNavRect()'));
    if (mutation === 'virtual-receipt') fs.writeFileSync(chrome, fs.readFileSync(chrome, 'utf8').replace('var cameraNavRect = nav.getBoundingClientRect();', 'var cameraNavRect = restingNavRect();'));
    const before = hash(fs.readFileSync(path.join(directory, 'assets/template.html')));
    assert.throws(() => assembleReader({ workspace: directory }), /Chrome Layout.*(?:source|adaptation|frozen)/i, mutation);
    assert.equal(hash(fs.readFileSync(path.join(directory, 'assets/template.html'))), before);
  }
});

test('assembly rejects changes outside audited CSS adaptations before writing', async t => {
  const { assembleReader } = await import(modulePath);
  for (const mutation of ['append', 'prefix', 'manifest', 'narrow']) {
    const directory = sandbox(t), css = path.join(directory, 'viewer/viewer.css');
    if (mutation === 'append') fs.appendFileSync(css, '\nbody { font-size: 1px; }\n');
    if (mutation === 'prefix') fs.writeFileSync(css, 'body { font-size: 1px; }\n' + fs.readFileSync(css, 'utf8'));
    if (mutation === 'manifest') fs.appendFileSync(path.join(directory, 'viewer/css-adaptations.json'), '\n');
    if (mutation === 'narrow') fs.writeFileSync(css, fs.readFileSync(css, 'utf8').replace('padding: 0.75rem 0.75rem 4.5rem;', 'padding: 0.75rem 0.75rem 4.25rem;'));
    const before = hash(fs.readFileSync(path.join(directory, 'assets/template.html')));
    assert.throws(() => assembleReader({ workspace: directory }), /Reader CSS.*(?:source|adaptation|frozen)/i, mutation);
    assert.equal(hash(fs.readFileSync(path.join(directory, 'assets/template.html'))), before);
  }
});

test('classic script order and private canonical/recorder cleanup retain the frozen implementations', async () => {
  const { namespaceSource } = await import(modulePath);
  for (const file of ['export.js', 'export-cleanup.js', 'motion-governor.js', 'focus.js', 'reader-layout.js', 'viewer-camera.js']) {
    const actual = read('viewer/' + file);
    const frozen = frozenMember('viewer/' + file);
    let expected = namespaceSource(frozen);
    if (file === 'export.js') {
      const adaptation = JSON.parse(read('viewer/export-adaptations.json'));
      assert.equal(adaptation.sourcePath, 'viewer/export.js');
      assert.equal(adaptation.sourceSha256, hash(frozen));
      assert.equal(adaptation.normalizedSourceSha256, hash(expected));
      const original = Buffer.from(expected), parts = [];
      const ranges = {
        'rasterize terminal cleanup': [expected.indexOf('      function rasterize(format)'), expected.indexOf('      function fitCanvasText')],
        'share-card terminal cleanup': [expected.indexOf('      function renderShareCard(options)'), expected.indexOf('      function rasterizeShareCard')],
        'MediaRecorder stop deadline': [expected.indexOf('        return new Promise(function (resolve, reject) {', expected.indexOf('        var motionScene = createMotionScene(svg);')), expected.indexOf('      var menu = document.getElementById')],
        'clipboard capability feedback': [expected.indexOf('      items().forEach(function (it)'), expected.indexOf('      // ---- Toast')],
        'clipboard failure PNG guidance': [expected.indexOf('      function runCopy()'), expected.indexOf("      menu.addEventListener('click'")],
      };
      let consumed = 0;
      for (const hunk of adaptation.hunks) {
        const at = hunk.sourceByteOffset, before = Buffer.from(hunk.sourceText);
        assert.ok(at >= consumed, 'audited Export adaptations must not overlap');
        const range = ranges[hunk.scope];
        assert.ok(range, 'every Export change needs a named narrow lifetime scope');
        assert.ok(at >= Buffer.byteLength(expected.slice(0, range[0])) && at + before.length <= Buffer.byteLength(expected.slice(0, range[1])), 'unrelated source functions and drawing/clipboard logic remain byte-exact');
        assert.ok(original.subarray(at, at + before.length).equals(before));
        parts.push(original.subarray(consumed, at), Buffer.from(hunk.replacementText));
        consumed = at + before.length;
      }
      parts.push(original.subarray(consumed));
      expected = Buffer.concat(parts).toString();
      assert.equal(hash(expected), adaptation.adaptedBodySha256);
    }
    if (file === 'reader-layout.js') {
      const adaptation = JSON.parse(read('viewer/reader-layout-adaptations.json'));
      assert.equal(adaptation.sourcePath, 'viewer/reader-layout.js');
      assert.equal(adaptation.sourceSha256, hash(frozen));
      assert.equal(adaptation.normalizedSourceSha256, hash(expected));
      assert.equal(adaptation.hunks.length, 2);
      const [legacy, viewport] = adaptation.hunks;
      assert.equal(legacy.scope, 'native legacy portrait height fitting');
      assert.equal(legacy.sourceText, '      var WIDE_RATIO = 1.55;\n');
      assert.match(legacy.replacementText, /svg\.getAttribute\('data-input-contract'\) === 'legacy-2\.17' && ratio < WIDE_RATIO/);
      assert.equal(viewport.scope, 'reader legend viewport containment');
      assert.equal(viewport.sourceText, '        // spare corner cannot hold them clear of the dock and content area.\n'
        + '        if (left + box.width * scale > right || top < rect.top + number(style.paddingTop) + number(style.borderTopWidth) ||\n');
      assert.match(viewport.replacementText, /left < 0 \|\| top < 0 \|\| left \+ box\.width \* scale > window\.innerWidth \|\| bottom > window\.innerHeight/);
      const original = Buffer.from(expected), parts = []; let consumed = 0;
      for (const hunk of adaptation.hunks) {
        const before = Buffer.from(hunk.sourceText), at = hunk.sourceByteOffset;
        assert.ok(at >= consumed, 'audited Reader Layout adaptations must not overlap');
        assert.ok(original.subarray(at, at + before.length).equals(before));
        assert.equal(original.subarray(0, at).toString().split('\n').length, hunk.sourceLine);
        parts.push(original.subarray(consumed, at), Buffer.from(hunk.replacementText)); consumed = at + before.length;
      }
      parts.push(original.subarray(consumed));
      expected = Buffer.concat(parts).toString();
      assert.equal(hash(expected), adaptation.adaptedBodySha256);
    }
    if (file === 'focus.js') {
      const adaptation = JSON.parse(read('viewer/focus-adaptations.json'));
      assert.equal(adaptation.sourcePath, 'viewer/focus.js');
      assert.equal(adaptation.sourceSha256, hash(frozen));
      assert.equal(adaptation.normalizedSourceSha256, hash(expected));
      const bindings = expected.indexOf("      var evidence = document.getElementById('focus-evidence');");
      const linksLine = expected.indexOf("      var evidenceLinks = document.getElementById('focus-evidence-links');\n");
      const ranges = [[bindings, linksLine + "      var evidenceLinks = document.getElementById('focus-evidence-links');\n".length],
        [expected.indexOf('      function renderSourceEvidence(id) {'), expected.indexOf('      function renderPassport(id, node) {')]];
      assert.ok(ranges.flat().every(at => at >= 0));
      const original = Buffer.from(expected), parts = []; let consumed = 0;
      for (const hunk of adaptation.hunks) {
        const before = Buffer.from(hunk.sourceText), at = hunk.sourceByteOffset;
        assert.equal(hunk.scope, 'passport source evidence states');
        assert.ok(at >= consumed, 'audited focus adaptations must not overlap');
        assert.ok(ranges.some(([start, end]) => at >= Buffer.byteLength(expected.slice(0, start)) && at + before.length <= Buffer.byteLength(expected.slice(0, end))),
          'focus adaptations stay inside the evidence bindings and the source evidence renderer');
        assert.ok(original.subarray(at, at + before.length).equals(before));
        parts.push(original.subarray(consumed, at), Buffer.from(hunk.replacementText)); consumed = at + before.length;
      }
      parts.push(original.subarray(consumed));
      expected = Buffer.concat(parts).toString();
      assert.equal(hash(expected), adaptation.adaptedBodySha256);
    }
    assert.ok(actual.endsWith(expected), `${file} must retain its complete source body outside recorded adaptations`);
  }
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
  const source = frozenMember('viewer/template.source.html');
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

test('bounded dock lift and actual screen measurements are the only Chrome Layout body adaptations', async () => {
  const { namespaceSource } = await import(modulePath);
  const frozen = frozenMember('viewer/viewer-chrome-layout.js'), normalized = namespaceSource(frozen);
  const adaptation = JSON.parse(read('viewer/chrome-layout-adaptations.json'));
  assert.equal(adaptation.schemaVersion, 1);
  assert.equal(adaptation.sourcePath, 'viewer/viewer-chrome-layout.js');
  assert.equal(adaptation.sourceSha256, hash(frozen));
  assert.equal(adaptation.normalizedSourceSha256, hash(normalized));
  assert.deepEqual(adaptation.hunks.map(hunk => [hunk.scope, hunk.sourceLine, hunk.sourceByteOffset]), [
    ['bounded stage-clear dock lift', 144, 5604],
    ['camera screen-coordinate receipt', 187, 7647],
    ['baseline screen-coordinate receipt', 229, 9839],
    ['displayed dock stability snapshot', 284, 12146],
    ['public displayed dock rectangle', 348, 15215],
  ]);
  const original = Buffer.from(normalized), parts = []; let consumed = 0;
  for (const hunk of adaptation.hunks) {
    const at = hunk.sourceByteOffset, before = Buffer.from(hunk.sourceText);
    assert.ok(at >= consumed);
    assert.ok(original.subarray(at, at + before.length).equals(before));
    assert.equal(original.subarray(0, at).toString().split('\n').length, hunk.sourceLine);
    parts.push(original.subarray(consumed, at), Buffer.from(hunk.replacementText)); consumed = at + before.length;
  }
  parts.push(original.subarray(consumed));
  const expected = Buffer.concat(parts).toString();
  assert.equal(hash(expected), adaptation.adaptedBodySha256);
  assert.equal(read('viewer/viewer-chrome-layout.js').replace(/^\/\*[\s\S]*?\*\/\n/, ''), expected);
});

test('narrow dock clearance and native legacy compact chrome are the only CSS body adaptations', async () => {
  const { namespaceSource } = await import(modulePath);
  const frozen = frozenMember('viewer/viewer.css');
  const normalized = namespaceSource(frozen);
  const adaptation = JSON.parse(read('viewer/css-adaptations.json'));
  assert.equal(adaptation.sourcePath, 'viewer/viewer.css');
  assert.equal(adaptation.sourceSha256, hash(frozen));
  assert.equal(adaptation.normalizedSourceSha256, hash(normalized));
  assert.equal(adaptation.hunks.length, 2);
  const narrow = adaptation.hunks.find(hunk => hunk.scope === 'ordinary narrow navigation clearance');
  assert.equal(narrow.sourceLine, 3688);
  assert.equal(narrow.sourceByteOffset, 131207);
  assert.equal(narrow.sourceText, '  html:not([data-embed="true"]) .diagram-container {\n    padding: 0.75rem 0.75rem 4.25rem;\n  }');
  assert.equal(narrow.replacementText, narrow.sourceText.replace('4.25rem', '4.5rem'));
  const compact = adaptation.hunks.find(hunk => hunk.scope === 'native legacy portrait compact chrome');
  assert.match(compact.replacementText, /@media screen and \(min-width: 1024px\) and \(max-height: 920px\)/);
  assert.match(compact.replacementText, /:has\(\.diagram-container svg\[data-input-contract="legacy-2\.17"\]\)/);
  const source = Buffer.from(normalized), parts = []; let consumed = 0;
  for (const hunk of adaptation.hunks) {
    const at = hunk.sourceByteOffset, before = Buffer.from(hunk.sourceText);
    assert.ok(at >= consumed, 'CSS adaptations cannot overlap');
    assert.ok(source.subarray(at, at + before.length).equals(before));
    parts.push(source.subarray(consumed, at), Buffer.from(hunk.replacementText)); consumed = at + before.length;
  }
  parts.push(source.subarray(consumed));
  const expected = Buffer.concat(parts).toString();
  assert.equal(hash(expected), adaptation.adaptedBodySha256);
  assert.ok(read('viewer/viewer.css').endsWith(expected), 'every other CSS rule must retain the frozen namespace-adapted bytes');
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
  for (const mutation of ['replace', 'placeholder', 'missing']) {
    const directory = sandbox(t), file = path.join(directory, 'viewer/catalog-adaptations.json');
    const adaptations = JSON.parse(fs.readFileSync(file, 'utf8'));
    if (mutation === 'replace') adaptations.messages['timeline.axis'] = { en: 'Clock' };
    if (mutation === 'placeholder') adaptations.messages['timeline.precision.day'].es = 'solo {fecha}';
    if (mutation === 'missing') delete adaptations.messages['timeline.precision.day'].ko;
    fs.writeFileSync(file, JSON.stringify(adaptations));
    assert.throws(() => assembleReader({ workspace: directory, check: true }), /replace|placeholders|key set/, mutation);
  }
});
