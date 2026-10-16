// Phoenix Reader boundary tests, adapted from the frozen Archify 3.1.0
// export-browser, export-cleanup-browser and motion-governor-browser tests.
// MIT notices remain in ../../viewer/LICENSE.txt. The DOM/media providers here
// are deterministic fixtures: these tests do not certify Chrome or media bytes.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import { Element, Events, observe, runtime } from './fixtures/runtime.mjs';

const allReleased = environment => {
  assert.ok(environment.urls.size > 0);
  assert.ok([...environment.urls.values()].every(url => url.revocations === 1), 'each owned Blob URL is released exactly once');
  assert.ok(environment.streams.every(stream => stream.track.state === 'ended' && stream.track.stops === 1));
  for (const recorder of environment.recorders) {
    assert.equal(recorder.ondataavailable, null);
    assert.equal(recorder.onerror, null);
    assert.equal(recorder.onstop, null);
  }
};

function chromeLayout({ stageBottom = 1045, restingTop = 1055 } = {}) {
  const bounds = (left, top, width, height) => ({ x: left, y: top, left, top, right: left + width, bottom: top + height, width, height });
  const html = new Element('html');
  const panel = html.appendChild(new Element('div', { class: 'diagram-container' }));
  const svg = panel.appendChild(new Element('svg', { viewBox: '0 0 786 652', 'data-view-scale': '1' }));
  svg.appendChild(new Element('path', { d: 'M484 517 L484 533 L770 533 L770 119 L386 119', 'marker-end': 'url(#arrowhead-dashed)' }));
  const nav = panel.appendChild(new Element('nav', { class: 'diagram-nav' }));
  for (const name of ['Path', 'Lens', 'Zoom out', 'Zoom in', 'Fit', 'Reset', 'Fullscreen']) nav.appendChild(new Element('button', { 'aria-label': name }));
  const window = new Events();
  window.innerWidth = 1440; window.innerHeight = 900;
  const state = { scroll: 0, print: false, scale: 1, translateY: 0, obstruction: 0 };
  const reserve = () => Number.parseFloat(panel.style.getPropertyValue('--phoenix-diagram-nav-reserve')) || 0;
  const lift = () => Number.parseFloat(panel.style.getPropertyValue('--phoenix-diagram-dock-lift')) || 0;
  panel.getBoundingClientRect = () => bounds(32, 58 - state.scroll, 1376, restingTop + 59 - 58 + reserve());
  svg.getBoundingClientRect = () => bounds(130.5, 67 - state.scroll + state.translateY, 1179 * state.scale, (stageBottom - 67) * state.scale);
  nav.getBoundingClientRect = () => bounds(1056, restingTop + reserve() - state.scroll - lift() + state.obstruction, 335, 42);
  window.getComputedStyle = element => ({ display: state.print && element === nav ? 'none' : 'block', visibility: 'visible',
    transform: element === svg ? `matrix(${state.scale},0,0,${state.scale},0,${state.translateY})` : 'none' });
  window.matchMedia = query => ({ matches: query === 'print' && state.print });
  const frames = new Map(); let nextFrame = 0;
  const context = vm.createContext({ window, document: { documentElement: html, querySelector: selector => html.querySelector(selector) },
    PhoenixDiagram: {}, phoenixDiagramLayoutOwners: {},
    requestAnimationFrame: callback => { frames.set(++nextFrame, callback); return nextFrame; }, cancelAnimationFrame: id => frames.delete(id) });
  vm.runInContext(fs.readFileSync(new URL('../../viewer/viewer-chrome-layout.js', import.meta.url), 'utf8'), context);
  const flush = () => {
    for (let count = 0; frames.size; count++) {
      assert.ok(count < 30, 'actual chrome controller must settle without rail/lift feedback');
      const pending = [...frames]; frames.clear();
      for (const [, callback] of pending) callback();
    }
  };
  flush();
  return { html, panel, svg, nav, window, state, reserve, lift, flush, api: context.PhoenixDiagram.viewerChromeLayout };
}

test('Viewer chrome leaves the real long workflow stage clear and reports its actual displayed dock', () => {
  // Captured 1440x900 geometry: the old controller lifted the 1055..1097
  // resting dock by 213px to 842..884, over the route at screen y=866.5.
  const environment = chromeLayout();
  const { api, nav, svg, state, window, flush, lift } = environment;
  const authored = svg.outerHTML;
  assert.equal(lift(), 0, 'the 10px resting stage gap has no space for an upward lift');
  assert.equal(nav.getBoundingClientRect().top, 1055);
  assert.equal(api.dockRect().top, nav.getBoundingClientRect().top);
  assert.equal(api.receipt().stageGap, 10);
  assert.equal(api.receipt().stageIntersectionArea, 0);
  assert.ok(nav.getBoundingClientRect().top > window.innerHeight, 'long-panel controls stay in the bottom rail');
  state.scroll = 250; window.emit('scroll'); flush();
  assert.equal(nav.getBoundingClientRect().top, 805, 'the same dock is reachable by document scrolling');
  assert.equal(api.stageRect().bottom, 795);
  assert.equal(api.dockRect().top, 805);
  assert.equal(api.receipt().stageGap, 10);
  assert.equal(nav.children.length, 7);
  assert.equal(svg.outerHTML, authored, 'dock layout never rewrites paths, markers, labels or the viewBox');
});

test('Viewer chrome lift uses only spare stage clearance, including a fractional cap', () => {
  const environment = chromeLayout({ stageBottom: 850.5 });
  assert.equal(environment.lift(), 194, 'integer lift cannot round through the 10px stage boundary');
  assert.equal(environment.api.dockRect().top, 861);
  assert.equal(environment.api.receipt().stageGap, 10.5);
  assert.equal(environment.api.receipt().stageIntersectionArea, 0);
  const short = chromeLayout({ stageBottom: 600, restingTop: 610 });
  assert.equal(short.lift(), 0);
  assert.equal(short.api.dockRect().top, 610);
  assert.equal(short.api.receipt().stageGap, 10);
});

test('Viewer chrome actual screen receipts expose a later dock obstruction', () => {
  const environment = chromeLayout({ stageBottom: 800 });
  const { api, nav, svg, state } = environment;
  state.scale = 2; state.translateY = -40; svg.setAttribute('data-view-scale', '2');
  state.obstruction = -60;
  const actual = nav.getBoundingClientRect();
  assert.equal(api.dockRect().top, actual.top);
  const receipt = api.measure();
  assert.equal(receipt.stageGap, actual.top - api.stageRect().bottom);
  assert.ok(receipt.stageGap < 0, 'a real intrusion is reported instead of substituting a virtual resting dock');
  assert.ok(receipt.stageIntersectionArea > 0);
});

test('Viewer chrome preserves its baseline rail through camera, reset, print, embed and mobile modes', async () => {
  const environment = chromeLayout({ stageBottom: 800, restingTop: 780 });
  const { api, html, svg, state, window, flush, reserve, lift } = environment;
  assert.equal(reserve(), 30);
  assert.equal(api.receipt().baselineStageGap, -20);
  state.scale = 2; state.translateY = -40; svg.setAttribute('data-view-scale', '2'); api.schedule(); flush();
  assert.equal(reserve(), 30);
  assert.equal(api.stageRect().bottom, 800);
  assert.equal(api.receipt().stageGap, 10);
  for (const mode of ['print', 'embed', 'mobile']) {
    if (mode === 'print') state.print = true;
    if (mode === 'embed') html.setAttribute('data-embed', 'true');
    if (mode === 'mobile') window.innerWidth = 390;
    api.schedule(); flush();
    assert.equal(reserve(), 0, `${mode} clears the desktop rail`);
    assert.equal(lift(), 0);
    state.print = false; html.removeAttribute('data-embed'); window.innerWidth = 1440;
    await api.reprobe(); flush();
    assert.equal(reserve(), 30, `${mode} restores the established camera baseline`);
    assert.equal(api.receipt().stageGap, 10);
  }
  state.scale = 1; state.translateY = 0; svg.setAttribute('data-view-scale', '1'); api.schedule(); flush();
  assert.equal(reserve(), 30, 'Camera Reset retains the established rail');
  assert.equal(api.receipt().baselineStageGap, -20);
  assert.equal(api.receipt().stageGap, 10);
});

test('Reader legend corners fit the viewport or retain the canonical SVG without paint or clip exemptions', async t => {
  const source = fs.readFileSync(new URL('../../viewer/reader-layout.js', import.meta.url), 'utf8');
  const probe = fs.readFileSync(new URL('../../src/browser/probe.js', import.meta.url), 'utf8');
  const block = (text, start, end) => {
    const from = text.indexOf(start), to = text.indexOf(end, from);
    assert.ok(from >= 0 && to > from, `actual source block ${start} must be present`);
    return text.slice(from, to);
  };
  const bounds = (left, top, width, height) => ({ x: left, y: top, left, top, right: left + width, bottom: top + height, width, height });
  const html = new Element('html', { 'data-reader-area': 'true' });
  const diagram = html.appendChild(new Element('div', { class: 'diagram-container' }));
  const svg = diagram.appendChild(new Element('svg', { viewBox: '0 0 841 652', 'data-view-scale': '1' }));
  const legend = svg.appendChild(new Element('g', { 'data-legend': '', transform: 'translate(40 572)' }));
  const label = legend.appendChild(new Element('text', {}, 'Authored legend'));
  const nav = diagram.appendChild(new Element('div', { class: 'diagram-nav' }));
  const viewBox = { width: 841, height: 652 };
  svg.clientWidth = 1261.5; svg.clientHeight = 978;
  let panel = bounds(32, 58, 1376, 1056);
  html.getBoundingClientRect = () => bounds(0, 0, 1440, 1433);
  diagram.getBoundingClientRect = () => panel;
  svg.getBoundingClientRect = () => bounds(200, 67, 1000, 978);
  legend.getBoundingClientRect = () => bounds(panel.left + 15, panel.bottom - 129, 180, 60);
  label.getBoundingClientRect = () => bounds(panel.left + 15, panel.bottom - 80, 120, 20);
  label.getClientRects = () => [label.getBoundingClientRect()];
  legend.getBBox = () => ({ x: 40, y: 542, width: 120, height: 40 });
  svg.getScreenCTM = () => ({ a: 1, b: 0, c: 0, d: 1, e: 0, f: 0,
    inverse() { return this; }, translate(x, y) { this.e += this.a * x; this.f += this.d * y; return this; },
    scale(value) { this.a *= value; this.d *= value; return this; } });
  for (const element of [label, legend, svg, diagram, html]) {
    Object.defineProperty(element, 'parentElement', { get: () => element.parentNode });
  }
  const computed = element => ({ display: 'block', visibility: 'visible', opacity: '1', fill: '#123456', fillOpacity: '1', stroke: 'none', strokeOpacity: '1', strokeWidth: '0',
    borderLeftWidth: '1', borderRightWidth: '1', borderTopWidth: '1', borderBottomWidth: '1',
    paddingLeft: '14', paddingRight: '14', paddingTop: '8', paddingBottom: '68',
    overflowX: element.style.overflow || 'visible', overflowY: element.style.overflow || 'visible' });
  const context = vm.createContext({ html, diagram, svg, legend, viewBox, automaticSequence: false, readingScroll: true,
    eligible: () => true, number: value => Number.parseFloat(value) || 0, visible: element => Boolean(element && !element.hidden),
    window: { innerWidth: 1440, innerHeight: 900, getComputedStyle: computed }, getComputedStyle: computed,
    rect: element => element.getBoundingClientRect() });
  // Execute the real assembly input's corner owner. The full Reader fit loop
  // has its separate deterministic fixture in reader-layout.test.mjs.
  vm.runInContext(block(source, '      function clearLegend()', '      // Rail modes:'), context);
  nav.hidden = true;
  for (const [name, rectangle, migrated] of [
    ['fitting corner', bounds(32, 58, 1376, 722), true],
    ['long panel below the first screen', bounds(32, 58, 1376, 1056), false],
    ['corner beyond the right viewport edge', bounds(1350, 58, 550, 722), false],
    ['corner beyond the left viewport edge', bounds(-60, 58, 1376, 722), false],
    ['corner above the viewport', bounds(32, -250, 1376, 300), false],
    ['legend wider than its panel', bounds(32, 58, 100, 722), false],
  ]) {
    await t.test(name, () => {
      panel = rectangle;
      legend.setAttribute('data-reader-legend-corner', '');
      legend.style.setProperty('--phoenix-diagram-reader-legend-transform', 'stale-reader-transform');
      vm.runInContext('syncLegend()', context);
      assert.equal(legend.hasAttribute('data-reader-legend-corner'), migrated);
      assert.equal(Boolean(legend.style.getPropertyValue('--phoenix-diagram-reader-legend-transform')), migrated);
      assert.equal(legend.getAttribute('transform'), 'translate(40 572)');
      assert.equal(svg.getAttribute('viewBox'), '0 0 841 652');
      assert.equal(label.textContent, 'Authored legend');
    });
  }
  const probeChecks = [
    block(probe, '  var contains = function', '  var outerHeight = function'),
    block(probe, '  var overflowClips = function', '  // Reading Depth'),
    block(probe, '  var paint = function', '  var observeText = function'),
    block(probe, '  var canvasTextChecks =', '  var cards ='),
  ].join('\n');
  await t.test('a fitting corner still rejects actual SVG ancestor clipping', () => {
    panel = bounds(32, 58, 1376, 722); svg.style.overflow = 'hidden';
    vm.runInContext('syncLegend(); var svgRect = rect(svg), diagramRect = rect(diagram);', context);
    assert.equal(legend.hasAttribute('data-reader-legend-corner'), true);
    vm.runInContext(probeChecks, context);
    assert.ok(context.canvasTextClips.some(entry => entry.failures.some(failure => failure.reason === 'ancestor-overflow-x')));
    assert.equal(svg.style.overflow, 'hidden');
  });
  await t.test('a canonical fallback still rejects a hidden authored legend', () => {
    panel = bounds(32, 58, 1376, 1056); svg.style.overflow = 'visible'; legend.hidden = true;
    vm.runInContext('syncLegend(); var svgRect = rect(svg), diagramRect = rect(diagram);', context);
    vm.runInContext(probeChecks, context);
    assert.ok(context.canvasTextClips.some(entry => entry.failures.some(failure => failure.reason === 'legend-text-not-painted')));
    assert.equal(legend.hidden, true);
  });
});

test('first Still, explicit Live and namespaced persisted preference use the actual Governor API', () => {
  const storage = new Map([['archify-motion', 'live']]);
  const first = runtime({ storage }); first.governorModule();
  assert.equal(first.api.motionGovernor.mode(), 'still');
  assert.equal(first.html.getAttribute('data-motion'), 'still');
  first.motionButton.emit('click');
  assert.equal(first.api.motionGovernor.mode(), 'live');
  assert.equal(storage.get('phoenix-diagram-motion'), 'live');
  assert.ok(first.storageCalls.every(([, key]) => key === 'phoenix-diagram-motion'));
  const reload = runtime({ storage }); reload.governorModule();
  assert.equal(reload.api.motionGovernor.mode(), 'live');
  reload.motionButton.emit('click');
  assert.equal(reload.api.motionGovernor.mode(), 'still');
  assert.equal(storage.has('phoenix-diagram-motion'), false);
  const last = runtime({ storage }); last.governorModule();
  assert.equal(last.api.motionGovernor.mode(), 'still');
});

test('hidden/reduced motion and nested print/export suspensions restore Live without rewriting preference', () => {
  const environment = runtime(); environment.governorModule();
  const { api, html, media, document, storage, motionButton } = environment;
  api.motionGovernor.resume();
  assert.equal(html.getAttribute('data-ambient-motion'), 'running');
  document.hidden = true; document.emit('visibilitychange');
  assert.equal(api.motionGovernor.mode(), 'still');
  assert.equal(html.getAttribute('data-document-hidden'), 'true');
  document.hidden = false; document.emit('visibilitychange');
  assert.equal(api.motionGovernor.mode(), 'live');
  media.matches = true; media.emit('change');
  assert.equal(api.motionGovernor.mode(), 'still');
  assert.equal(motionButton.disabled, true);
  media.matches = false; media.emit('change');
  assert.equal(api.motionGovernor.mode(), 'live');
  assert.equal(motionButton.disabled, false);
  const releasePrint = api.motionGovernor.suspend('print');
  const releaseExport = api.motionGovernor.suspend('export');
  const releaseSecondPrint = api.motionGovernor.suspend('print');
  assert.equal(api.motionGovernor.mode(), 'still');
  assert.equal(releasePrint(), true); assert.equal(releasePrint(), false);
  releaseExport(); assert.equal(api.motionGovernor.mode(), 'still');
  releaseSecondPrint(); assert.equal(api.motionGovernor.mode(), 'live');
  assert.equal(storage.get('phoenix-diagram-motion'), 'live');
  // This checks the public suspension mechanism. Native print CSS and actual
  // browser beforeprint/afterprint events remain a separate browser gate.
});

test('canonical public SVG export cleans a private clone and preserves every authored node/note/legend', async () => {
  const environment = runtime(); environment.exportModule();
  const { svg, api } = environment;
  svg.style.setProperty('transform', 'translate(10px, 20px) scale(1.8)');
  svg.style.setProperty('clip-path', 'inset(4px)');
  for (const name of ['data-view-scale', 'data-focus-active', 'data-lens-active', 'data-route-active', 'data-share-route']) svg.setAttribute(name, 'true');
  const authored = svg.querySelector('[data-node-id="a"]');
  authored.setAttribute('data-focus-selected', 'true'); authored.setAttribute('aria-pressed', 'true');
  authored.setAttribute('data-source-evidence-count', '2'); authored.setAttribute('data-source-evidence-original-label', ''); authored.setAttribute('aria-label', 'temporary');
  authored.querySelector('rect').setAttribute('transform', 'translate(3 4)');
  for (const name of ['data-ambient-flow-overlay', 'data-intent-trace-overlay', 'data-route-probe-overlay', 'data-route-journey-overlay', 'data-semantic-lens-overlay']) svg.appendChild(new Element('g', { [name]: '' }));
  const before = svg.outerHTML;
  const outcome = await environment.outcome(api.exportMenu.run('svg'));
  assert.equal(outcome.state, 'resolved');
  assert.equal(svg.outerHTML, before, 'export must not rewrite the live exploration state');
  assert.equal(environment.html.getAttribute('data-last-export-canonical'), 'true');
  assert.equal(environment.downloads.length, 1);
  const clone = environment.serialized.at(-1);
  assert.deepEqual(clone.querySelectorAll('[data-node-id]').map(node => node.getAttribute('data-node-id')), ['a', 'b', 'isolated']);
  assert.equal(clone.querySelector('[data-node-id="a"]').getAttribute('aria-label'), '');
  assert.equal(clone.querySelector('[data-node-id="a"]').getAttribute('aria-pressed'), 'false');
  assert.equal(clone.querySelector('[data-node-id="a"]').querySelector('rect').getAttribute('transform'), 'translate(3 4)');
  assert.match(clone.textContent, /Keep the entire authored note 中文/);
  assert.match(clone.textContent, /Backend 后端/);
  assert.equal(clone.querySelectorAll('[data-ambient-flow-overlay], [data-intent-trace-overlay], [data-route-probe-overlay], [data-route-journey-overlay], [data-semantic-lens-overlay], [data-focus-selected], [data-detail]').length, 0);
  assert.equal(clone.style.getPropertyValue('transform'), '');
  assert.equal(clone.style.getPropertyValue('clip-path'), '');
  assert.equal(environment.downloads[0].anchor.parentNode, null);
  await environment.clock.advance(1000); allReleased(environment);
});

for (const failure of ['null', 'throw', 'empty']) {
  test(`raster ${failure} encoder output fails without downloads or stale success receipt`, async () => {
    const environment = runtime({ toBlob: failure }); environment.exportModule();
    const before = environment.svg.outerHTML;
    const result = await environment.outcome(environment.api.exportMenu.run('png'));
    assert.equal(result.state, 'resolved', 'public menu handles its error');
    assert.equal(environment.html.getAttribute('data-last-export-format'), null);
    assert.equal(environment.html.getAttribute('data-last-export-error-format'), 'png');
    assert.equal(environment.downloads.length, 0);
    assert.equal(environment.alerts.length, 1);
    assert.equal(environment.svg.outerHTML, before);
    allReleased(environment);
  });
}

test('completed raster releases its image callbacks, source URL and temporary download anchor', async () => {
  const environment = runtime(); environment.exportModule();
  await environment.outcome(environment.api.exportMenu.run('png'));
  assert.equal(environment.downloads.length, 1);
  for (const image of environment.images) { assert.equal(image.onload, null); assert.equal(image.onerror, null); }
  assert.equal(environment.downloads[0].anchor.parentNode, null);
  allReleased(environment);
});

for (const fault of ['image', 'context-null', 'capture', 'constructor', 'start', 'error', 'stop', 'empty']) {
  test(`WebM ${fault} boundary rejects and releases only the action's resources`, async () => {
    const options = fault === 'image' ? { image: 'error' } : fault === 'context-null' ? { context: 'null' } : fault === 'capture' ? { capture: 'throw' } : { recorder: fault };
    const environment = runtime(options); environment.exportModule();
    const before = environment.svg.outerHTML;
    const outcome = await environment.outcome(environment.api.motion.recordWebm({ duration: 250, fps: 10 }));
    assert.equal(outcome.state, 'rejected');
    assert.equal(environment.svg.outerHTML, before);
    assert.equal(environment.downloads.length, 0);
    assert.equal(environment.clock.tasks.size, 0, 'recording timers/frames must not remain after failure');
    for (const image of environment.images) { assert.equal(image.onload, null); assert.equal(image.onerror, null); }
    allReleased(environment);
  });
}

test('MediaRecorder error remains terminal after captured late data/stop/error callbacks and cancelled timers', async () => {
  const environment = runtime(); environment.exportModule();
  const outcome = observe(environment.api.motion.recordWebm({ duration: 250, fps: 10 }));
  await environment.clock.advance(0);
  const recorder = environment.recorders[0];
  const callbacks = { data: recorder.ondataavailable, stop: recorder.onstop, error: recorder.onerror };
  callbacks.error({ error: new Error('first terminal error') });
  await environment.clock.advance(0);
  assert.equal(outcome.state, 'rejected'); assert.equal(outcome.value.message, 'first terminal error');
  const stops = recorder.stopCalls;
  callbacks.data({ data: new Blob(['late nonempty data']) }); callbacks.stop(); callbacks.error({ error: new Error('late error') });
  await environment.clock.advance(10000);
  assert.equal(outcome.state, 'rejected'); assert.equal(outcome.value.message, 'first terminal error');
  assert.equal(recorder.stopCalls, stops);
  assert.equal(environment.clock.tasks.size, 0);
  allReleased(environment);
});

test('a recorder that never emits stop reaches a bounded failure and cancels capture', async () => {
  const environment = runtime({ recorder: 'no-stop' }); environment.exportModule();
  const outcome = observe(environment.api.motion.recordWebm({ duration: 250, fps: 10 }));
  await environment.clock.advance(0);
  const recorder = environment.recorders[0], lateStop = recorder.onstop, lateData = recorder.ondataavailable;
  await environment.clock.advance(10000);
  assert.equal(outcome.state, 'rejected', 'a stopped encoder cannot leave an action pending forever');
  assert.equal(outcome.value.name, 'TimeoutError');
  lateData({ data: new Blob(['late encoded data']) }); lateStop();
  await environment.clock.advance(0);
  assert.equal(outcome.state, 'rejected'); assert.equal(outcome.value.name, 'TimeoutError');
  assert.equal(environment.clock.tasks.size, 0);
  allReleased(environment);
});

test('normal/flush-failure WebM completion cleans resources and overlapping actions remain isolated', async () => {
  for (const recorder of [undefined, 'flush']) {
    const environment = runtime({ recorder }); environment.exportModule();
    environment.governorModule();
    environment.api.motionGovernor.resume();
    const claim = environment.api.motionGovernor.claim('focus');
    environment.svg.style.setProperty('transform', 'translate(10px, 20px) scale(1.4)');
    environment.svg.setAttribute('data-focus-active', 'true');
    const before = environment.svg.outerHTML;
    const outcome = await environment.outcome(environment.api.motion.recordWebm({ duration: 250, fps: 10 }));
    assert.equal(outcome.state, 'resolved'); assert.ok(outcome.value.size > 0);
    assert.equal(environment.svg.outerHTML, before);
    assert.equal(environment.api.motionGovernor.mode(), 'live');
    assert.equal(environment.api.motionGovernor.owner(), 'focus');
    assert.equal(environment.api.motionGovernor.release(claim), true);
    assert.equal(environment.clock.tasks.size, 0); allReleased(environment);
  }
  const environment = runtime(); environment.exportModule();
  const first = observe(environment.api.motion.recordWebm({ duration: 250, fps: 10 }));
  const second = observe(environment.api.motion.recordWebm({ duration: 500, fps: 20 }));
  await environment.clock.advance(0);
  const [one, two] = environment.recorders;
  one.onerror({ error: new Error('first action fails') });
  await environment.clock.advance(0);
  assert.equal(first.state, 'rejected'); assert.equal(second.state, 'pending');
  assert.equal(one.stream.track.state, 'ended'); assert.equal(two.stream.track.state, 'live');
  assert.equal(environment.urls.get(environment.images[0].url).revocations, 1);
  assert.equal(environment.urls.get(environment.images[1].url).revocations, 0);
  await environment.clock.advance(1000);
  assert.equal(second.state, 'resolved'); allReleased(environment);
});

for (const mode of ['promise', 'fallback', 'denied', 'unsupported']) {
  test(`PNG clipboard ${mode} keeps synchronous gesture construction and reports its actual outcome`, async () => {
    const environment = runtime({ clipboard: mode }); environment.exportModule();
    const copy = environment.menu.querySelector('button[data-action="copy"]');
    if (mode === 'unsupported') { assert.equal(copy.disabled, true); assert.equal(environment.copies.length, 0); return; }
    environment.clickCopy();
    assert.equal(environment.copies[0].kind, 'promise');
    assert.equal(environment.copies[0].gesture, true);
    await environment.clock.advance(1700);
    await environment.clock.advance(32); // Paint the toast queued by the resolved clipboard Promise.
    const kinds = environment.copies.map(call => call.kind);
    assert.deepEqual(kinds, mode === 'fallback' ? ['promise', 'blob', 'write', 'written'] : mode === 'denied' ? ['promise', 'write'] : ['promise', 'write', 'written']);
    if (mode === 'fallback') assert.equal(environment.copies[1].gesture, false);
    assert.equal(environment.alerts.length, mode === 'denied' ? 1 : 0);
    const toast = environment.document.querySelector('.phoenix-diagram-toast');
    if (mode === 'denied') { assert.doesNotMatch(toast.textContent, /Copied PNG/); assert.match(environment.alerts[0], /Copy failed/); }
    else assert.match(toast.textContent, /Copied PNG to clipboard/);
    assert.equal(environment.html.getAttribute('data-last-export-format'), null);
    assert.equal(environment.downloads.length, 0);
    allReleased(environment);
  });
}

for (const [locale, hint] of [['en', 'You can download PNG from the Export menu instead.'], ['zh-CN', '也可以在导出菜单中下载 PNG。']]) {
  test(`${locale} clipboard denial/unavailability gives its real catalog PNG fallback and keeps that action usable`, async () => {
    const unavailable = runtime({ locale, clipboard: 'unsupported' }); unavailable.exportModule();
    const copy = unavailable.menu.querySelector('button[data-action="copy"]');
    assert.equal(copy.disabled, true);
    assert.ok(copy.title.includes(hint), `unsupported copy feedback must say: ${hint}`);
    assert.equal(unavailable.menu.querySelector('button[data-format="png"]').disabled, false);
    if (locale === 'en') {
      for (const otherLocale of ['es', 'ko', 'zh-TW']) {
        const fallback = runtime({ locale: otherLocale, clipboard: 'unsupported' }); fallback.exportModule();
        assert.ok(fallback.menu.querySelector('button[data-action="copy"]').title.includes(hint), `${otherLocale} uses the explicit English fallback`);
      }
    }
    const denied = runtime({ locale, clipboard: 'denied' }); denied.exportModule();
    denied.clickCopy(); await denied.clock.advance(1700); await denied.clock.advance(32);
    assert.equal(denied.alerts.length, 1);
    assert.ok(denied.alerts[0].includes(hint), `denied copy feedback must say: ${hint}`);
    assert.equal(denied.menu.querySelector('button[data-format="png"]').disabled, false);
    const outcome = await denied.outcome(denied.api.exportMenu.run('png'));
    assert.equal(outcome.state, 'resolved');
    assert.equal(denied.downloads.length, 1);
    assert.ok(denied.downloads[0].blob.size > 0);
    allReleased(denied);
  });
}
