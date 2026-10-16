// Deterministic layout measurements execute the actual Reader module. These
// fixtures reproduce its eligibility/fit decisions, not Chrome CSS acceptance.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import { gunzipSync } from 'node:zlib';
import { Element, Events } from './fixtures/runtime.mjs';
import { namespaceSource } from '../../../../scripts/assemble-phoenix-diagram-reader.mjs';

const viewerSource = fs.readFileSync(new URL('../../viewer/reader-layout.js', import.meta.url), 'utf8');
const viewerCss = fs.readFileSync(new URL('../../viewer/viewer.css', import.meta.url), 'utf8');
const compactMedia = '@media screen and (min-width: 1024px) and (max-height: 920px) {';
const compactSelector = 'html[data-reader-layout="adaptive"]:not([data-diagram-shape="wide"]):not([data-embed="true"]):not([data-present="true"]):has(.diagram-container svg[data-input-contract="legacy-2.17"])';
function compactRules(css) {
  const at = css.indexOf(compactMedia);
  if (at < 0) return null;
  const end = css.indexOf('\n}', at);
  const block = css.slice(at, end + 2);
  const rules = {};
  for (const target of ['body', ':is(.header, .header-row)', '.diagram-container', '.diagram-nav']) {
    const start = block.indexOf(`${compactSelector} ${target} {`);
    assert.ok(start >= 0, `the compact scope must be explicit for ${target}`);
    const body = block.slice(block.indexOf('{', start) + 1, block.indexOf('}', start));
    rules[target] = Object.fromEntries(body.split(';').map(value => value.trim()).filter(Boolean)
      .map(value => value.split(':').map(part => part.trim())));
  }
  return { block, rules };
}
function frozenSource() {
  const archive = gunzipSync(fs.readFileSync(new URL('../../upstream/archify-3.1.0-source.tar.gz', import.meta.url)));
  for (let at = 0; at + 512 <= archive.length && archive[at];) {
    const header = archive.subarray(at, at + 512);
    const field = (start, size) => header.subarray(start, start + size).toString().replace(/\0.*$/s, '');
    const name = (field(345, 155) ? field(345, 155) + '/' : '') + field(0, 100);
    const size = Number.parseInt(field(124, 12), 8);
    if (name === 'viewer/reader-layout.js') return namespaceSource(archive.subarray(at + 512, at + 512 + size).toString());
    at += 512 + Math.ceil(size / 512) * 512;
  }
  throw new Error('Missing frozen Reader Layout module');
}
const frozen = frozenSource();
const rect = (width, height, top = 0) => ({ x: 0, y: top, left: 0, top, right: width, bottom: top + height, width, height });

function reader({ width = 1440, height = 900, viewBoxWidth = 820, viewBoxHeight = 920,
  contract = 'legacy-2.17', fit = 'legacy', embed = false, present = false, print = false, source = viewerSource,
  primaryFont = 14, edgeFont = 10, sublabelFont = null, wrapped = false, cssSource = viewerCss } = {}) {
  const html = new Element('html'), body = new Element('body'); html.appendChild(body);
  if (embed) html.setAttribute('data-embed', 'true');
  if (present) html.setAttribute('data-present', 'true');
  const shell = body.appendChild(new Element('div', { class: 'container' }));
  const header = shell.appendChild(new Element('div', { class: 'header' }));
  const diagram = shell.appendChild(new Element('div', { class: 'diagram-container' }));
  const svg = diagram.appendChild(new Element('svg', { viewBox: `0 0 ${viewBoxWidth} ${viewBoxHeight}`, 'data-reader-fit': fit }));
  if (contract != null) svg.setAttribute('data-input-contract', contract);
  svg.viewBox = { baseVal: { x: 0, y: 0, width: viewBoxWidth, height: viewBoxHeight } };
  const node = svg.appendChild(new Element('g', { 'data-node-id': 'a' }));
  node.appendChild(new Element('rect', { x: '30', y: '40', width: '100', height: '60' }));
  node.appendChild(new Element('text', { 'data-node-label': '', 'font-size': String(primaryFont) }, 'Authored node'));
  if (sublabelFont != null) node.appendChild(new Element('text', { 'data-detail': 'context', 'font-size': String(sublabelFont) }, 'Authored sublabel'));
  const edge = svg.appendChild(new Element('g', { 'data-edge-from': 'a', 'data-edge-to': 'b' }));
  edge.appendChild(new Element('polyline', { points: '130,70 200,70 200,180' }));
  edge.appendChild(new Element('text', { 'data-detail': 'context', 'font-size': String(edgeFont) }, 'Authored relation'));
  const rail = shell.appendChild(new Element('aside', { class: 'reader-rail' }));
  const cards = rail.appendChild(new Element('div', { class: 'cards' }));
  cards.appendChild(new Element('article', {}, 'Authored notes'));
  const document = new Events();
  Object.assign(document, { documentElement: html, body, querySelector: selector => html.querySelector(selector),
    getElementById: id => html.querySelector('#' + id) });
  const metric = name => Number.parseFloat(html.style.getPropertyValue(name)) || 0;
  const compact = compactRules(cssSource);
  const compactApplied = () => Boolean(compact && !print && width >= 1024 && height <= 920
    && html.getAttribute('data-reader-layout') === 'adaptive' && html.getAttribute('data-diagram-shape') !== 'wide'
    && html.getAttribute('data-embed') !== 'true' && html.getAttribute('data-present') !== 'true'
    && svg.getAttribute('data-input-contract') === 'legacy-2.17' && svg.closest('.diagram-container') === diagram);
  // This fixture models the measured chrome: 32px buttons, 1px borders and
  // the existing 10px stage gap. It does not implement Chrome's CSS engine.
  const spacing = () => {
    const rules = compactApplied() ? compact.rules : null;
    const body = rules ? Number.parseFloat(rules.body['padding-block']) : 6;
    const headerMargin = rules ? Number.parseFloat(rules[':is(.header, .header-row)']['margin-bottom']) : 8;
    const top = rules ? Number.parseFloat(rules['.diagram-container']['padding-top']) : 8;
    const navPadding = rules ? Number.parseFloat(rules['.diagram-nav'].padding) : 4;
    const navBottom = rules ? Number.parseFloat(rules['.diagram-nav'].bottom) : 16;
    return { body, headerMargin, top, bottom: 10 + 32 + navPadding * 2 + 2 + navBottom + 1 };
  };
  const svgWidth = () => metric('--phoenix-diagram-diagram-max-width') || (metric('--phoenix-diagram-reader-width') || width - 64) - 30;
  const svgHeight = () => svgWidth() * viewBoxHeight / viewBoxWidth;
  const diagramHeight = () => Math.max(svgHeight() + spacing().top + 1 + spacing().bottom, metric('--phoenix-diagram-diagram-min-height'));
  const panelTop = () => spacing().body + 44 + spacing().headerMargin;
  header.getBoundingClientRect = () => rect(width - 64, 44, spacing().body);
  svg.getBoundingClientRect = () => rect(svgWidth(), svgHeight(), panelTop() + spacing().top + 1);
  diagram.getBoundingClientRect = () => rect(metric('--phoenix-diagram-reader-width') || width - 64, diagramHeight(), panelTop());
  rail.getBoundingClientRect = cards.getBoundingClientRect = () => rect(width - 64, 144, panelTop() + diagramHeight());
  shell.getBoundingClientRect = () => rect(metric('--phoenix-diagram-reader-width') || width - 64, 44 + spacing().headerMargin + diagramHeight() + 144, spacing().body);
  for (const element of [html, body]) {
    Object.defineProperty(element, 'scrollWidth', { get: () => width });
    Object.defineProperty(element, 'scrollHeight', { get: () => Math.ceil(spacing().body * 2 + 44 + spacing().headerMargin + diagramHeight() + 144) });
  }
  const styles = element => ({ display: 'block', marginTop: '0', marginBottom: element === header ? String(spacing().headerMargin) : '0',
    paddingTop: element === body ? String(spacing().body) : element === diagram ? String(spacing().top) : '0',
    paddingBottom: element === body ? String(spacing().body) : element === diagram ? String(spacing().bottom - 1) : '0',
    paddingLeft: element === body ? '32' : element === diagram ? '14' : '0',
    paddingRight: element === body ? '32' : element === diagram ? '14' : '0',
    borderTopWidth: element === diagram ? '1' : '0', borderBottomWidth: element === diagram ? '1' : '0',
    borderLeftWidth: element === diagram ? '1' : '0', borderRightWidth: element === diagram ? '1' : '0' });
  const window = Object.assign(new Events(), { innerWidth: width, innerHeight: height, scrollY: 0,
    getComputedStyle: styles, matchMedia: () => ({ matches: print }) });
  const frames = new Map(); let next = 0;
  const context = { document, window, PhoenixDiagram: {}, phoenixDiagramLayoutOwners: {},
    localStorage: { getItem() { return null; }, setItem() {} }, viewerText: key => key,
    requestAnimationFrame(callback) { const id = ++next; frames.set(id, callback); return id; },
    cancelAnimationFrame(id) { frames.delete(id); } };
  vm.runInNewContext(source, context);
  if (wrapped) {
    const stage = new Element('div', { class: 'diagram-stage' });
    stage.appendChild(svg); diagram.appendChild(stage);
  }
  return { html, svg, api: context.PhoenixDiagram.readerLayout,
    flush() {
      for (let turn = 0; turn < 10; turn++) {
        const tasks = [...frames]; frames.clear();
        if (!tasks.length) return;
        for (const [, callback] of tasks) callback();
      }
      throw new Error('Reader fixture did not settle');
    },
    state() { return [html.style.cssText, [...html.attrs], svg.outerHTML, JSON.parse(JSON.stringify(context.PhoenixDiagram.readerLayout.receipt()))]; },
    projectedMinimum() { return Math.min(primaryFont, edgeFont, sublabelFont ?? Infinity) * svgWidth() / viewBoxWidth; },
    compactApplied,
    mainFlowBottom() { return panelTop() + diagramHeight() + spacing().body; },
  };
}

for (const [kind, viewBoxWidth, viewBoxHeight, fit] of [
  ['legacy sequence', 820, 920, 'legacy'], ['legacy lifecycle', 980, 680, ''],
]) {
  test(`${kind} enters measured desktop fitting without changing its fixed geometry`, () => {
    for (const [width, height] of [[1440, 900], [1600, 1000], [1920, 1080], [2048, 1320]]) {
      const environment = reader({ width, height, viewBoxWidth, viewBoxHeight, fit });
      const before = environment.svg.outerHTML;
      environment.flush();
      assert.equal(environment.api.active(), true, `${width}x${height}`);
      assert.ok(environment.svg.getBoundingClientRect().bottom < height, `${kind} main SVG must fit`);
      assert.ok(environment.projectedMinimum() >= 6);
      assert.equal(environment.html.getAttribute('data-reader-overflow'), null);
      assert.equal(environment.svg.outerHTML, before, 'Reader cannot rewrite viewBox, points, labels, or reader-fit');
    }
  });
}

test('legacy fitting stops at the readable floor and exposes unresolved overflow', () => {
  const environment = reader({ viewBoxHeight: 5000 });
  const before = environment.svg.outerHTML;
  environment.flush();
  assert.equal(environment.api.active(), true);
  assert.ok(environment.projectedMinimum() >= 6);
  assert.ok(environment.projectedMinimum() < 6.1);
  assert.ok(environment.svg.getBoundingClientRect().bottom > 900);
  assert.equal(environment.html.getAttribute('data-reader-overflow'), 'authored');
  assert.equal(environment.svg.outerHTML, before);
});

test('wide legacy and other contracts retain their exact frozen Reader decisions', () => {
  for (const options of [
    { viewBoxWidth: 1800, viewBoxHeight: 800 }, { viewBoxWidth: 1550, viewBoxHeight: 1000 },
    { contract: null }, { contract: 'phoenix-v1' },
    { contract: 'unknown' }, { contract: 'phoenix-v1', fit: 'intrinsic-height' },
    { contract: 'phoenix-v1', fit: 'width-first' }, { fit: 'intrinsic-height' },
  ]) {
    const current = reader(options), reference = reader({ ...options, source: frozen });
    current.flush(); reference.flush();
    assert.deepEqual(current.state(), reference.state(), JSON.stringify(options));
  }
});

test('legacy eligibility preserves narrow, embedded, presentation and print exclusions', () => {
  for (const options of [{ width: 1023 }, { width: 390 }, { embed: true }, { present: true }, { print: true }]) {
    const environment = reader(options);
    const before = environment.svg.outerHTML;
    environment.flush();
    assert.equal(environment.api.active(), false, JSON.stringify(options));
    assert.equal(environment.svg.outerHTML, before);
    assert.equal(environment.html.style.getPropertyValue('--phoenix-diagram-reader-width'), '');
  }
});

test('7px legacy context fits the low desktop chrome budget at the unchanged 6px floor', () => {
  for (const wrapped of [false, true]) {
    const environment = reader({ primaryFont: 11, edgeFont: 9, sublabelFont: 7, wrapped });
    const before = environment.svg.outerHTML;
    environment.flush();
    assert.ok(environment.projectedMinimum() >= 6);
    assert.ok(environment.mainFlowBottom() <= 900, `main chrome bottom ${environment.mainFlowBottom()}`);
    assert.equal(environment.compactApplied(), true);
    assert.equal(environment.svg.outerHTML, before);
  }
});

test('compact CSS uses only existing native portrait classification and preserves every exclusion', () => {
  const compact = compactRules(viewerCss);
  assert.ok(compact, 'native legacy compact spacing must be present');
  assert.deepEqual(compact.rules, {
    body: { 'padding-block': '2px' }, ':is(.header, .header-row)': { 'margin-bottom': '0' },
    '.diagram-container': { 'padding-top': '2px' }, '.diagram-nav': { bottom: '4px', padding: '1px' },
  });
  const originalCss = viewerCss.replace(compact.block, '');
  for (const options of [
    { width: 1023 }, { height: 921 }, { contract: null }, { contract: 'phoenix-v1' },
    { viewBoxWidth: 1550, viewBoxHeight: 1000 }, { embed: true }, { present: true }, { print: true },
  ]) {
    const current = reader(options), reference = reader({ ...options, cssSource: originalCss });
    current.flush(); reference.flush();
    assert.equal(current.compactApplied(), false, JSON.stringify(options));
    assert.deepEqual(current.state(), reference.state(), JSON.stringify(options));
  }
});
