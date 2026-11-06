// Deterministic layout measurements execute the actual Reader module. These
// fixtures reproduce its eligibility/fit decisions, not Chrome CSS acceptance.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import { Element, Events } from './fixtures/runtime.mjs';

const viewerSource = fs.readFileSync(new URL('../../viewer/reader-layout.js', import.meta.url), 'utf8');
const rect = (width, height, top = 0) => ({ x: 0, y: top, left: 0, top, right: width, bottom: top + height, width, height });

function reader({ width = 1440, height = 900, viewBoxWidth = 820, viewBoxHeight = 920,
  fit = 'contain', embed = false, present = false, print = false, source = viewerSource,
  primaryFont = 14, edgeFont = 10, sublabelFont = null, wrapped = false } = {}) {
  const html = new Element('html'), body = new Element('body'); html.appendChild(body);
  if (embed) html.setAttribute('data-embed', 'true');
  if (present) html.setAttribute('data-present', 'true');
  const shell = body.appendChild(new Element('div', { class: 'container' }));
  const header = shell.appendChild(new Element('div', { class: 'header' }));
  const diagram = shell.appendChild(new Element('div', { class: 'diagram-container' }));
  const svg = diagram.appendChild(new Element('svg', { viewBox: `0 0 ${viewBoxWidth} ${viewBoxHeight}`, 'data-reader-fit': fit }));
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
  // This fixture models the measured chrome: 32px buttons, 1px borders and
  // the existing 10px stage gap. It does not implement Chrome's CSS engine.
  const spacing = () => ({ body: 6, headerMargin: 8, top: 8, bottom: 10 + 32 + 4 * 2 + 2 + 16 + 1 });
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
  };
}

for (const [kind, viewBoxWidth, viewBoxHeight, fit] of [
  ['fixed sequence', 820, 920, 'contain'], ['fixed canvas', 980, 680, ''],
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

test('fixed-canvas fitting stops at the readable floor and exposes unresolved overflow', () => {
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

test('non-wide fixed canvases fit by height with contain or missing reader fit', () => {
  for (const fit of ['contain', '']) {
    for (const [width, height] of [[1440, 900], [1600, 1000], [1920, 1080], [2048, 1320]]) {
      const environment = reader({ fit, width, height });
      const before = environment.svg.outerHTML;
      environment.flush();
      assert.equal(environment.api.active(), true, JSON.stringify({ fit, width, height }));
      assert.ok(environment.svg.getBoundingClientRect().bottom < height);
      assert.ok(environment.projectedMinimum() >= 6);
      assert.equal(environment.html.getAttribute('data-reader-overflow'), null);
      assert.equal(environment.svg.outerHTML, before, 'Reader preserves all authored SVG geometry and text');
    }
    const tall = reader({ fit, viewBoxHeight: 5000 });
    const before = tall.svg.outerHTML;
    tall.flush();
    assert.equal(tall.api.active(), true);
    assert.ok(tall.projectedMinimum() >= 6 && tall.projectedMinimum() < 6.1);
    assert.ok(tall.svg.getBoundingClientRect().bottom > 900);
    assert.equal(tall.html.getAttribute('data-reader-overflow'), 'authored');
    assert.equal(tall.svg.outerHTML, before);
  }
});

test('fixed-canvas eligibility preserves narrow, embedded, presentation and print exclusions', () => {
  for (const options of [{ width: 1023 }, { width: 390 }, { embed: true }, { present: true }, { print: true }]) {
    const environment = reader(options);
    const before = environment.svg.outerHTML;
    environment.flush();
    assert.equal(environment.api.active(), false, JSON.stringify(options));
    assert.equal(environment.svg.outerHTML, before);
    assert.equal(environment.html.style.getPropertyValue('--phoenix-diagram-reader-width'), '');
  }
});
