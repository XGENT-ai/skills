// Unit preflight for the real-browser controller. These tests never start
// Chrome and never stand in for actual download, clipboard or decoder proof.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import vm from 'node:vm';
import { PassThrough } from 'node:stream';
import { EventEmitter } from 'node:events';
import { createHash } from 'node:crypto';
import { artifactIdentity, validateInput, verifyFileSignature, verifyCanonicalFacts, verifyPrintSample, configureNetwork, runExportAcceptance, runContentAcceptance, REQUIRED_EXPORT_SCOPE } from './export-acceptance.mjs';
import { DEFAULT_TIMEOUTS, ownedProfile, discoverBrowser, browserArguments, PipeCdp } from './support/export-browser.mjs';
import * as probes from './support/export-page.mjs';
import { REQUIRED_READER_INTERACTIONS, sendReaderKey, verifyReaderInteractions } from './support/export-reader.mjs';
import * as acceptance from './export-acceptance.mjs';
import { bootReader, bundledCatalogs, dispatch, plain } from './viewer/fixtures/reader.mjs';
import * as diagrams from './viewer/fixtures/diagrams.mjs';

function temporary(t) { const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-export-controller-')); t.after(() => fs.rmSync(directory, { recursive: true, force: true })); return directory; }

test('configuration requires fixed HTML/text inputs and finite explicit deadlines', () => {
  const options = { artifactPath: '/tmp/diagram.html', evidenceDir: '/tmp/evidence' };
  assert.deepEqual(validateInput(options), DEFAULT_TIMEOUTS);
  assert.equal(validateInput({ ...options, expectedLiteral: '</script><script>globalThis.__phoenixDiagramXss=1</script> $&/{{PHOENIX:SVG}}' }).commandMs, 15000);
  for (const invalid of [{}, { ...options, artifactPath: '/tmp/file.js' }, { ...options, timeouts: { actionMs: Infinity } }, { ...options, timeouts: { startupMs: 0 } }, { ...options, expectedLiteral: {} }, { ...options, sentinelName: 'x);alert(1)//' }, { ...options, offline: 'true' }]) assert.throws(() => validateInput(invalid));
  assert.deepEqual(REQUIRED_EXPORT_SCOPE.formats, ['svg', 'png', 'jpeg', 'webp', 'webm']);
  assert.equal(REQUIRED_EXPORT_SCOPE.themes.length * REQUIRED_EXPORT_SCOPE.modes.length * REQUIRED_EXPORT_SCOPE.formats.length, 20);
  assert.deepEqual(REQUIRED_EXPORT_SCOPE.motionSuppression?.conditions, ['reduced-motion', 'page-hidden']);
});

test('byte signature preflight rejects a suffix/MIME claim and empty/wrong bytes', () => {
  // A fixed valid one-pixel PNG tests only the signature helper, not a Reader
  // export or browser decoder. Real acceptance additionally requires both.
  const png = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Y9ZQmcAAAAASUVORK5CYII=', 'base64');
  assert.equal(verifyFileSignature(png, 'png', 'image/png'), 'PNG');
  assert.throws(() => verifyFileSignature(png, 'jpeg', 'image/jpeg'), /signature/);
  assert.throws(() => verifyFileSignature(png, 'png', 'image/jpeg'), /MIME/);
  for (const format of REQUIRED_EXPORT_SCOPE.formats) assert.throws(() => verifyFileSignature(Buffer.alloc(0), format, 'image/png'), /empty/);
  assert.throws(() => verifyFileSignature(Buffer.from('<svg-broken'), 'svg', 'image/svg+xml'), /signature/);
  assert.throws(() => verifyFileSignature(Buffer.from('1a45dfa3', 'hex'), 'webm', 'video/webm'), /signature/);
});

test('offline configuration sends the actual CDP network policy and propagates rejection', async () => {
  const calls = [], send = async (method, params = {}) => { calls.push({ method, params }); };
  await configureNetwork(send);
  assert.deepEqual(calls, [
    { method: 'Network.enable', params: {} },
    { method: 'Network.emulateNetworkConditions', params: { offline: true, latency: 0, downloadThroughput: -1, uploadThroughput: -1 } },
  ]);
  await assert.rejects(configureNetwork(async method => { if (method === 'Network.emulateNetworkConditions') throw new Error('protocol-only refusal'); }), /protocol-only refusal/);
});

test('explicit browser discovery never falls back and arguments retain the sandbox/private profile', t => {
  const directory = temporary(t);
  assert.throws(() => discoverBrowser({ browserPath: path.join(directory, 'absent'), env: { PATH: process.env.PATH } }), error => error.code === 'browser/configured-path');
  assert.throws(() => discoverBrowser({ env: { PHOENIX_DIAGRAM_BROWSER: path.join(directory, 'absent') } }), error => error.code === 'browser/configured-path');
  assert.equal(discoverBrowser({ browserPath: process.execPath }), fs.realpathSync(process.execPath));
  const args = browserArguments(directory);
  assert.ok(args.includes('--remote-debugging-pipe')); assert.ok(args.includes('--user-data-dir=' + directory)); assert.ok(!args.includes('--no-sandbox')); assert.ok(args.includes('--use-mock-keychain'));
  assert.equal(DEFAULT_TIMEOUTS.startupMs, 90000); assert.equal(DEFAULT_TIMEOUTS.commandMs, 15000);
});

test('profile cleanup refuses substitution and preserves the later directory', async t => {
  const parent = temporary(t), profile = ownedProfile(parent), displaced = profile.path + '.displaced';
  fs.renameSync(profile.path, displaced); fs.mkdirSync(profile.path); fs.writeFileSync(path.join(profile.path, 'later-owner'), 'keep');
  await assert.rejects(profile.cleanup(), /identity changed/);
  assert.equal(fs.readFileSync(path.join(profile.path, 'later-owner'), 'utf8'), 'keep'); assert.ok(fs.existsSync(displaced));
  const ordinary = ownedProfile(parent); await ordinary.cleanup(); await ordinary.cleanup(); assert.equal(fs.existsSync(ordinary.path), false);
});

test('Pipe CDP framing/event subscription is local transport evidence only', async () => {
  const child = new EventEmitter(); child.stdio = [null, null, new PassThrough(), new PassThrough(), new PassThrough()];
  const cdp = new PipeCdp(child, { commandMs: 50 });
  const sent = []; child.stdio[3].on('data', bytes => sent.push(JSON.parse(bytes.toString().replace(/\0$/, ''))));
  const pending = cdp.send('Runtime.evaluate', { expression: '1' }, 'own-session');
  assert.equal(sent[0].sessionId, 'own-session');
  child.stdio[4].write('{"id":1,"res'); child.stdio[4].write('ult":{"value":1}}\0');
  assert.deepEqual(await pending, { value: 1 });
  const events = [], stop = cdp.subscribe(event => events.push(event));
  child.stdio[4].write('{"method":"Browser.downloadProgress","params":{"state":"completed"}}\0'); stop();
  assert.equal(events.length, 1); assert.equal(events[0].params.state, 'completed');
  const rejected = cdp.send('Page.navigate'); cdp.failAll(new Error('owned transport ended')); await assert.rejects(rejected, /owned transport ended/);
  for (const stream of child.stdio.filter(Boolean)) stream.destroy();
});

test('CDP deadlines identify the actual request and hash expressions without exposing JS', async () => {
  const child = new EventEmitter(); child.stdio = [null, null, new PassThrough(), new PassThrough(), new PassThrough()];
  const cdp = new PipeCdp(child, { commandMs: 10 }), expression = 'Promise.resolve("private-expression-sentinel")';
  const results = await Promise.allSettled([
    cdp.send('Runtime.evaluate', { expression, awaitPromise: true }, 'own-session'),
    cdp.send('Page.getFrameTree', {}, 'own-session'),
  ]);
  const error = results[0].reason;
  assert.equal(error.code, 'browser/command-timeout');
  assert.equal(error.message, 'Export Chrome deadline: Runtime.evaluate timed out after 10ms\n');
  assert.deepEqual(error.cdp, { method: 'Runtime.evaluate', id: 1, sessionId: 'own-session', timeoutMs: 10,
    expressionSha256: createHash('sha256').update(expression).digest('hex') });
  assert.deepEqual(results[1].reason.cdp, { method: 'Page.getFrameTree', id: 2, sessionId: 'own-session', timeoutMs: 10 });
  assert(!JSON.stringify({ ...error, message: error.message }).includes('private-expression-sentinel'));
  assert.equal(cdp.pending.size, 0); assert.equal(DEFAULT_TIMEOUTS.commandMs, 15000);
  for (const stream of child.stdio.filter(Boolean)) stream.destroy();
});

test('canceled downloads retain only the original progress identity and byte counts', () => {
  const entry = { guid: 'owned-guid', state: 'canceled', receivedBytes: 17, totalBytes: 29,
    url: 'private-download-sentinel', suggestedFilename: 'private-file-sentinel' };
  const error = acceptance.canceledDownloadError('Chrome canceled webm download', entry);
  assert.equal(error.message, 'Chrome canceled webm download'); assert.equal(error.code, undefined);
  assert.deepEqual(error.download, { guid: entry.guid, state: entry.state, receivedBytes: 17, totalBytes: 29 });
  assert(!JSON.stringify(error).includes('private-'));
});

test('failed page receipts preserve safe CDP, function and download metadata without retrying', async t => {
  const directory = temporary(t), artifactPath = path.join(directory, 'fixture.html'); fs.writeFileSync(artifactPath, '<p>Failure propagation preflight only</p>');
  for (const kind of ['timeout', 'canceled']) {
    const listeners = new Set(), calls = [], cdp = {
      subscribe(callback) { listeners.add(callback); return () => listeners.delete(callback); },
      async send(method, params = {}, sessionId, timeoutMs) {
        calls.push({ method, params });
        if (method === 'Browser.getVersion') return { product: 'Chrome/unit-preflight' };
        if (method === 'Target.createBrowserContext') return { browserContextId: 'own-context' };
        if (method === 'Target.createTarget') return { targetId: 'own-target' };
        if (method === 'Target.attachToTarget') return { sessionId: 'own-session' };
        if (method === 'Page.addScriptToEvaluateOnNewDocument') return { identifier: 'own-script' };
        if (method === 'Page.navigate') { for (const listener of listeners) listener({ method: 'Page.loadEventFired', sessionId }); return { frameId: 'own-frame' }; }
        if (method === 'Runtime.evaluate') {
          const error = kind === 'timeout' ? new Error('Export Chrome deadline: Runtime.evaluate timed out after 15000ms\n') : acceptance.canceledDownloadError('Chrome canceled webm download', { guid: 'own-guid', state: 'canceled', receivedBytes: 17, totalBytes: 29 });
          if (kind === 'timeout') { error.code = 'browser/command-timeout'; error.cdp = { method, id: calls.length, sessionId, timeoutMs, expressionSha256: createHash('sha256').update(params.expression).digest('hex'), expression: 'private-expression-sentinel' }; }
          else Object.assign(error.download || (error.download = {}), { url: 'private-download-sentinel', suggestedFilename: 'private-file-sentinel' });
          throw error;
        }
        return {};
      },
    };
    // A rejecting synthetic adapter exercises failure serialization only.
    const report = await runContentAcceptance({ artifactPath, evidenceDir: path.join(directory, kind), expectedLiteral: 'Failure propagation preflight only', cdp });
    const record = report.errors[0];
    assert.equal(report.status, 'failed'); assert.equal(record.code, kind === 'timeout' ? 'browser/command-timeout' : 'acceptance/failed');
    assert.deepEqual(record.evaluation, { fnName: 'awaitReader', sourceSha256: createHash('sha256').update(probes.awaitReader.toString()).digest('hex') });
    if (kind === 'timeout') assert.deepEqual(record.cdp, { method: 'Runtime.evaluate', id: calls.findIndex(call => call.method === 'Runtime.evaluate') + 1, sessionId: 'own-session', timeoutMs: 15000, expressionSha256: createHash('sha256').update(calls.find(call => call.method === 'Runtime.evaluate').params.expression).digest('hex') });
    else assert.deepEqual(record.download, { guid: 'own-guid', state: 'canceled', receivedBytes: 17, totalBytes: 29 });
    assert.equal(calls.filter(call => call.method === 'Runtime.evaluate').length, 1);
    assert(!JSON.stringify(record).includes('private-'));
    assert.equal(artifactIdentity(artifactPath).sha256, report.artifact.sha256);
  }
});

test('fixed browser probes parse without evaluating any authored literal', () => {
  for (const probe of Object.values(probes)) new vm.Script('(' + probe.toString() + ')');
  const literal = '</script><script>globalThis.__phoenixDiagramXss=1</script> $&/{{PHOENIX:SVG}}';
  const expression = '(' + probes.contentObservation.toString() + ')(' + JSON.stringify({ expectedLiteral: literal, sentinelName: '__phoenixDiagramXss' }) + ')';
  new vm.Script(expression);
  assert.equal(globalThis.__phoenixDiagramXss, undefined);
});

test('trusted click coordinates follow the completed camera and scroll-driven panel placement', async () => {
  let frame = 0, scrolled = false;
  const button = {
    scrollIntoView() { scrolled = true; },
    getBoundingClientRect() {
      return { left: 191.5, top: frame >= 5 ? 340 : 434, width: 125.5, height: 32 };
    },
  };
  const stage = { hasAttribute: name => name === 'data-camera-transaction' && frame < 3 };
  const context = {
    document: { querySelector: selector => selector === '.diagram-container' ? stage : button },
    requestAnimationFrame(callback) { queueMicrotask(() => callback(++frame * 16)); },
  };
  const point = await vm.runInNewContext('(' + probes.readerClickPoint.toString() + ')("#btn-reach-downstream")', context);
  assert.equal(scrolled, true);
  assert.deepEqual({ ...point }, { x: 254.25, y: 356 }, 'a delayed native press must still hit the final button, not its old location');
});

test('Reader snapshots exclude only explicitly marked runtime text and retain authored numeric legend text', () => {
  const texts = [
    { textContent: 'Legend', closest: () => null },
    { textContent: '2', closest: () => null },
    { textContent: '2', closest: selector => selector.includes('[data-legend-bridge-runtime]') ? {} : null },
  ];
  const svg = { getAttribute: name => name === 'viewBox' ? '0 0 500 300' : null, style: { getPropertyValue: () => '' }, querySelectorAll: selector => selector === 'text' ? texts : [] };
  const api = { exportMenu: {}, motionGovernor: { mode: () => 'still', owner: () => '' } };
  const snapshot = vm.runInNewContext('(' + probes.snapshotReader.toString() + ')()', { window: { PhoenixDiagram: api }, navigator: {}, document: { querySelector: () => svg } });
  assert.deepEqual(Array.from(snapshot.text), ['Legend', '2']);
});

test('canonical export gate rejects missing authored legend text, including numeric text matching a runtime count', () => {
  const expected = { viewBox: '0 0 500 300', nodes: [{ id: 'a', label: 'A', text: ['A'] }], edges: [], text: ['A', 'Legend', '2'] };
  assert.doesNotThrow(() => verifyCanonicalFacts(structuredClone(expected), expected));
  for (const text of [['A', '2'], ['A', 'Legend']]) assert.throws(() => verifyCanonicalFacts({ ...expected, text }, expected), /text/);
});

test('print acceptance rejects missing native events, ongoing print motion and unrestored authored state', () => {
  const facts = { mode: 'live', owner: '', focus: null, view: null, viewBox: '0 0 500 300', preset: 'classic', transform: '', clip: '', nodes: [{ id: 'a', text: ['A'] }], edges: [], text: ['A', 'Legend'] };
  const sample = { mode: 'live', motionApplicable: true, before: facts, during: structuredClone(facts), after: structuredClone(facts), motionBefore: { printMedia: false, running: 1 }, motionDuring: { printMedia: true, running: 0, staticCount: 3, hiddenStatic: 0 }, motionAfter: { printMedia: false, running: 1 }, nativePrint: { events: [{ type: 'beforeprint' }, { type: 'afterprint' }], pdf: { path: '/owned/live.pdf', identity: { sha256: 'a'.repeat(64), bytes: 200 }, signature: 'PDF', eof: true, pages: 1 } } };
  assert.doesNotThrow(() => verifyPrintSample(sample));
  const invalid = [
    value => { value.nativePrint.events = []; },
    value => { value.motionDuring.printMedia = false; },
    value => { value.motionDuring.running = 1; },
    value => { value.motionDuring.hiddenStatic = 1; },
    value => { value.motionAfter.running = 0; },
    value => { value.after.text.pop(); },
    value => { value.nativePrint.pdf.signature = 'not-pdf'; },
  ];
  for (const mutate of invalid) { const value = structuredClone(sample); mutate(value); assert.throws(() => verifyPrintSample(value), /[Pp]rint/); }
});

function motionSuppressionFixture(staticOnly = false) {
  const observations = ['dark', 'light'].flatMap(theme => ['still', 'live'].flatMap(mode => ['reduced-motion', 'page-hidden'].map(condition => {
    const snapshot = suppressed => ({ reader: { mode: suppressed ? 'still' : mode, owner: '', focus: null, view: null, viewBox: '0 0 500 300', preset: 'classic', transform: '', clip: '', nodes: [{ id: 'a', label: 'A', text: ['A'] }], edges: [], text: ['A', 'Legend'], eligibleMotion: !staticOnly },
      reducedMotion: suppressed && condition === 'reduced-motion', hidden: suppressed && condition === 'page-hidden', visibilityState: suppressed && condition === 'page-hidden' ? 'hidden' : 'visible', printMedia: false,
      documentHidden: suppressed && condition === 'page-hidden' ? 'true' : null, capable: true, preference: mode === 'live' ? 'live' : null,
      button: { disabled: suppressed && condition === 'reduced-motion', pressed: !suppressed && mode === 'live' ? 'true' : 'false' }, animationCount: staticOnly ? 0 : 2, running: !staticOnly && !suppressed && mode === 'live' ? 2 : 0, meaning: { total: 3, visible: [0, 1, 2] } });
    return { theme, mode, condition, status: 'pass', method: condition === 'reduced-motion' ? 'cdp-prefers-reduced-motion' : 'cdp-owned-target-visibility', motionApplicable: !staticOnly && mode === 'live', before: snapshot(false), during: snapshot(true), after: snapshot(false),
      inputs: [{ type: 'click', trusted: true, target: { id: 'btn-motion' } }], ownedTargets: condition === 'page-hidden' ? { subject: 'owned-subject', cover: 'owned-cover', browserContext: 'owned-context' } : undefined,
      events: condition === 'reduced-motion' ? [{ type: 'reduced-motion', trusted: true, matches: true }, { type: 'reduced-motion', trusted: true, matches: false }] : [{ type: 'visibilitychange', trusted: true, hidden: true, visibilityState: 'hidden' }, { type: 'visibilitychange', trusted: true, hidden: false, visibilityState: 'visible' }] };
  })));
  return { status: 'pass', observations };
}

test('motion suppression requires eight native transitions, complete static facts and unchanged user preference', () => {
  assert.equal(typeof acceptance.verifyMotionSuppression, 'function');
  const verify = acceptance.verifyMotionSuppression, valid = motionSuppressionFixture();
  assert.doesNotThrow(() => verify(valid));
  assert.doesNotThrow(() => verify(motionSuppressionFixture(true)), 'a genuinely static SVG needs no invented animation');
  for (const missing of [undefined, { status: 'not-run' }, { status: 'pass', observations: [] }]) assert.throws(() => verify(missing), /motion suppression/i);
  for (const mutate of [
    v => { v.observations.pop(); },
    v => { v.observations[1] = structuredClone(v.observations[0]); },
    v => { v.observations[0].status = 'incomplete'; },
    v => { v.observations[0].method = 'VM-matchMedia-stub'; },
    v => { v.observations[0].events[0].trusted = false; },
    v => { v.observations[0].during.reducedMotion = false; },
    v => { v.observations[0].during.button.disabled = false; },
    v => { v.observations[1].during.hidden = false; },
    v => { v.observations[1].during.visibilityState = 'visible'; },
    v => { v.observations[1].events[0].hidden = false; },
    v => { v.observations[1].ownedTargets.cover = v.observations[1].ownedTargets.subject; },
    v => { v.observations[2].motionApplicable = false; },
    v => { v.observations[2].before.running = 0; },
    v => { v.observations[2].during.running = 1; },
    v => { v.observations[2].after.running = 0; },
    v => { v.observations[0].before.running = 1; },
    v => { v.observations[0].during.preference = 'live'; },
    v => { v.observations[0].after.reader.text.pop(); },
    v => { v.observations[0].during.reader.viewBox = '0 0 1 1'; },
    v => { v.observations[0].during.meaning.visible.pop(); },
    v => { for (const key of ['before', 'during', 'after']) delete v.observations[0][key].reader.nodes; },
    v => { v.observations[0].inputs[0].trusted = false; },
    v => { v.observations[0].events.reverse(); },
    v => { v.observations[0].after.reader.mode = 'live'; },
  ]) { const invalid = structuredClone(valid); mutate(invalid); assert.throws(() => verify(invalid), /motion suppression/i); }
});

test('Reader interaction gate rejects absent scope, fabricated input, wrong search/detail feedback and changed authored geometry', () => {
  const geometry = { viewBox: '0 0 500 300', nodes: [{ id: 'a', label: 'Alpha', body: { x: '20', y: '30', width: '100', height: '50' } }], edges: [], text: [{ text: 'Alpha', x: '70', y: '55' }] };
  const state = { geometry, camera: { scale: 1, x: 0, y: 0 }, paintedCamera: { scale: 1, x: 0, y: 0 }, focus: null, finder: { open: false, query: '', results: [], emptyHidden: true, status: '' }, details: { hidden: true, painted: false, id: '', label: '' }, preset: 'classic', htmlPreset: 'classic', active: { id: 'btn-node-finder', view: null } };
  const input = (type, target = {}, key = '') => ({ type, trusted: true, target, key });
  const open = { ...state, finder: { open: true, query: '', results: [{ id: 'a', label: 'Alpha' }], emptyHidden: true, status: '1 node available' } };
  const selected = { ...state, focus: 'a', details: { hidden: false, painted: true, id: 'a', label: 'Alpha' } };
  const camera = { ...state, camera: { scale: 1.5, x: -30, y: -20 }, paintedCamera: { scale: 1.5, x: -30, y: -20 } };
  const observations = [
    { name: 'camera', before: state, after: state, result: { zoomIn: camera, beforePan: camera, afterPan: { ...camera, camera: { scale: 1.5, x: -50, y: -40 }, paintedCamera: { scale: 1.5, x: -50, y: -40 } }, zoomOut: { ...camera, camera: { scale: 1.25, x: -20, y: -10 }, paintedCamera: { scale: 1.25, x: -20, y: -10 } }, reset: state }, inputs: [input('click', { view: 'in' }), input('click', { view: 'out' }), input('click', { view: 'reset' }), input('pointerdown'), input('pointermove'), input('pointerup')] },
    { name: 'search-empty', before: state, after: open, result: { expectedIds: ['a'] }, inputs: [input('click', { id: 'btn-node-finder' })] },
    { name: 'search-label', before: open, after: { ...open, finder: { ...open.finder, query: 'Alpha' } }, result: { query: 'Alpha', expectedId: 'a', label: 'Alpha' }, inputs: [input('input', { id: 'node-finder-input' })] },
    { name: 'search-id', before: open, after: { ...open, finder: { ...open.finder, query: 'a' } }, result: { query: 'a', expectedId: 'a', label: 'Alpha', distinctFromLabel: false }, inputs: [input('input', { id: 'node-finder-input' })] },
    { name: 'search-no-match', before: open, after: { ...open, finder: { open: true, query: '__absent__', results: [], emptyHidden: false, emptyText: 'No matching nodes', status: '0 of 1 nodes' } }, result: { query: '__absent__' }, inputs: [input('input', { id: 'node-finder-input' })] },
    { name: 'details', before: open, after: state, result: { expectedId: 'a', label: 'Alpha', opened: selected, escapeClosed: state, keyboardOpened: selected, keyboardClosed: state, closeFocusPath: [{ id: 'btn-focus-clear' }] }, inputs: [input('keydown', {}, 'Enter'), input('keydown', {}, 'Escape'), input('click', { nodeId: 'a' }), input('keydown', {}, 'Tab'), input('keydown', {}, 'Enter')] },
    { name: 'keyboard', before: state, after: state, result: { reached: [{ id: 'btn-preset' }, { id: 'btn-node-finder' }, { view: 'in' }], finderOpened: open, finderClosed: state }, inputs: [input('keydown', {}, 'Tab'), input('keydown', {}, 'Enter'), input('keydown', {}, 'Escape')] },
    { name: 'style', before: state, after: state, result: { from: 'classic', to: 'blueprint', changed: { ...state, preset: 'blueprint', htmlPreset: 'blueprint', presetChecked: true }, restored: state }, inputs: [input('click', { id: 'btn-preset' }), input('click', { preset: 'blueprint' }), input('click', { preset: 'classic' })] },
  ].map(value => ({ ...value, status: 'pass' }));
  const value = JSON.parse(JSON.stringify({ status: 'pass', observations }));
  assert.equal(observations.length, REQUIRED_READER_INTERACTIONS.length);
  assert.doesNotThrow(() => verifyReaderInteractions(value));
  for (const mutate of [
    v => { v.observations.pop(); },
    v => { v.observations[0].result.afterPan.camera = v.observations[0].result.beforePan.camera; },
    v => { v.observations[0].inputs[3].trusted = false; },
    v => { v.observations[2].after.finder.results = []; },
    v => { v.observations[4].after.finder.emptyHidden = true; },
    v => { v.observations[5].result.escapeClosed.focus = 'a'; },
    v => { v.observations[6].result.reached = [{ id: 'btn-preset' }]; },
    v => { v.observations[7].result.changed.geometry.nodes[0].body.x = '999'; },
    v => { const erase = value => { if (!value || typeof value !== 'object') return; if (value.geometry) value.geometry.nodes.forEach(node => { node.body = {}; }); Object.values(value).forEach(erase); }; erase(v); },
  ]) { const invalid = structuredClone(value); mutate(invalid); assert.throws(() => verifyReaderInteractions(invalid), /[Rr]eader/); }
});

test('native Reader search replacement sends the browser editing command on keyDown only', async () => {
  const calls = [];
  await sendReaderKey(async (method, params) => calls.push({ method, params }), 'a', 'KeyA', 4, ['selectAll']);
  assert.deepEqual(calls.map(call => ({ method: call.method, type: call.params.type, key: call.params.key, modifiers: call.params.modifiers, commands: call.params.commands })), [
    { method: 'Input.dispatchKeyEvent', type: 'keyDown', key: 'a', modifiers: 4, commands: ['selectAll'] },
    { method: 'Input.dispatchKeyEvent', type: 'keyUp', key: 'a', modifiers: 4, commands: undefined },
  ]);
});

test('native Reader Enter carries its character so browser button activation is exercised', async () => {
  const calls = [];
  await sendReaderKey(async (method, params) => calls.push({ method, params }), 'Enter', 'Enter');
  assert.deepEqual(calls.map(call => ({ type: call.params.type, text: call.params.text })), [{ type: 'keyDown', text: '\r' }, { type: 'keyUp', text: undefined }]);
});

test('details Reach takes a trusted tap or click on its own control, after a real scroll when the panel hides it', async () => {
  const control = (inPanel, hit = 'btn-reach-downstream') => ({ direction: 'downstream', x: 221, y: 375, inPanel, onControl: hit === 'btn-reach-downstream', hit, from: { x: 195, y: 183 } });
  const page = (located, reach = { direction: 'downstream', originId: 'orders', nodeIds: ['orders', 'customer'] }) => {
    const calls = [], queue = [...located];
    // Locating takes no argument; the Reach state is read for one direction.
    return { calls, send: async (method, params) => calls.push([method, params.type || params.gestureSourceType]), evaluate: async (_fn, direction) => direction === undefined ? queue.shift() : reach };
  };
  const narrow = page([control(false), control(true)]);
  assert.deepEqual(await acceptance.operateReach(narrow, 'orders', true), { status: 'pass', direction: 'downstream', input: 'touch', scrolled: true, nodeIds: ['orders', 'customer'] });
  assert.deepEqual(narrow.calls, [['Input.synthesizeScrollGesture', 'touch'], ['Input.dispatchTouchEvent', 'touchStart'], ['Input.dispatchTouchEvent', 'touchEnd']]);
  const desktop = page([control(true)]);
  assert.equal((await acceptance.operateReach(desktop, 'orders', false)).input, 'mouse');
  assert.deepEqual(desktop.calls, [['Input.dispatchMouseEvent', 'mousePressed'], ['Input.dispatchMouseEvent', 'mouseReleased']]);
  // A control under the navigation is never tapped; no control means nothing to operate.
  const covered = page([control(false, 'btn-node-finder'), control(true, 'btn-node-finder')]);
  await assert.rejects(acceptance.operateReach(covered, 'orders', true), /its center hits btn-node-finder/);
  assert.ok(covered.calls.every(([method]) => method === 'Input.synthesizeScrollGesture'));
  assert.deepEqual(await acceptance.operateReach(page([null]), 'orders', true), { status: 'not-applicable' });
  await assert.rejects(acceptance.operateReach(page([control(true)], { direction: 'downstream', originId: 'customer', nodeIds: [] }), 'orders', true), /started from customer instead of orders/);
});

test('missing configured browser yields an explicit non-pass receipt with no exported files', async t => {
  const directory = temporary(t), file = path.join(directory, 'artifact.html'); fs.writeFileSync(file, '<!doctype html><title>immutable fixture</title>');
  const before = artifactIdentity(file);
  const result = await runExportAcceptance({ artifactPath: file, evidenceDir: directory, browserPath: path.join(directory, 'missing-chrome'), offline: true });
  assert.equal(result.status, 'failed'); assert.equal(result.browser, null); assert.equal(result.exports.length, 20); assert.ok(result.exports.every(scope => scope.status === 'not-run')); assert.equal(result.resources.status, 'not-run');
  assert.deepEqual(result.offline, { requested: true, status: 'not-run', method: 'cdp-network-emulation', configured: false, remoteRequests: [] });
  assert.equal(result.clipboard.success.status, 'not-run'); assert.equal(result.clipboard.permissionDenied.status, 'not-run'); assert.equal(result.clipboard.capabilityUnavailable.status, 'not-run');
  assert.equal(result.motionSuppression?.status, 'not-run');
  assert.equal(result.motionSuppression?.observations.length, 8);
  assert.equal(result.errors[0].code, 'browser/configured-path'); assert.deepEqual(result.artifact, before); assert.deepEqual(artifactIdentity(file), before);
  assert.deepEqual(JSON.parse(fs.readFileSync(result.receiptPath, 'utf8')).artifact, before);
  const receiptBytes = fs.readFileSync(result.receiptPath);
  await assert.rejects(runExportAcceptance({ artifactPath: file, evidenceDir: directory, browserPath: path.join(directory, 'missing-chrome') }), /EEXIST/);
  assert.ok(fs.readFileSync(result.receiptPath).equals(receiptBytes));
  await assert.rejects(runContentAcceptance({ artifactPath: file, evidenceDir: directory }), /expectedLiteral/);
});

// Controller-level checks of the assembled Reader's export module over the
// deterministic DOM fixture. Canvas, Image and MediaRecorder are fixtures, so
// these receipts and file names say nothing about real encoded files.
const RECEIPT = ['format', 'canonical', 'variant', 'route-state-clean', 'reach-state-clean', 'width', 'height', 'error-format', 'error'];
const exportReceipt = env => Object.fromEntries(RECEIPT.map(name => [name, env.html.getAttribute('data-last-export-' + name)]));
const catalogText = (key, values = {}) => bundledCatalogs.en[key].replace(/\{([a-zA-Z0-9_]+)\}/g, (match, name) => Object.hasOwn(values, name) ? String(values[name]) : match);
const authoredFacts = svg => ({
  nodes: svg.querySelectorAll('[data-node-id]').map(node => node.getAttribute('data-node-id')),
  edges: svg.querySelectorAll('[data-edge-from][data-edge-to]').map(edge => [edge.getAttribute('data-edge-key'), edge.getAttribute('data-edge-from'), edge.getAttribute('data-edge-to')].join(' ')),
});
const EXPLORATION_STATE = '[data-share-route], [data-share-reach], [data-share-route-match], [data-share-reach-match], [data-route-match], [data-reach-match], [data-focus-match], [data-focus-selected], [data-tree-hidden], [data-tree-collapsed], [data-route-probe-overlay], [data-relationship-hit-overlay]';

async function bootDiagram(svg, type, meta = {}) {
  const env = bootReader({ svg, metadata: diagrams.metadata(type, meta) });
  await env.settle(60);
  assert.deepEqual(env.errors, []);
  return env;
}
async function settled(env, action, duration = 120) {
  const pending = action();
  await env.settle(duration);
  return pending;
}
function shareItems(env) {
  env.api.exportMenu.open();
  const state = ['route-share-card', 'reach-share-card'].map(action => {
    const item = env.$(`#export-menu button[data-action="${action}"]`);
    return { action, hidden: item.hidden, disabled: item.disabled };
  });
  env.api.exportMenu.close();
  return state;
}

test('Route and Reach share cards report non-canonical variant receipts and named files; a later canonical export is the complete diagram', async () => {
  const env = await bootDiagram(diagrams.graph(), 'architecture', { title: 'Payments Platform' });
  const live = authoredFacts(env.svg);
  env.api.routeProbe.begin({ source: 'a' });
  env.api.routeProbe.choose('d');
  await settled(env, () => env.api.exportMenu.downloadRouteShareCard());
  assert.deepEqual(exportReceipt(env), { format: 'share-card', canonical: 'false', variant: 'route', 'route-state-clean': 'true', 'reach-state-clean': null, width: '1200', height: '630', 'error-format': null, error: null });
  assert.ok(Number(env.html.getAttribute('data-last-export-bytes')) > 0);
  assert.equal(env.downloads.at(-1).filename, 'payments-platform-route-share-card.png');
  const routeCard = env.serialized.at(-1);
  assert.equal(routeCard.hasAttribute('data-share-route'), true);
  assert.deepEqual(routeCard.querySelectorAll('[data-node-id][data-share-route-match]').map(node => node.getAttribute('data-node-id')), ['a', 'b', 'c', 'd']);
  assert.deepEqual(routeCard.querySelectorAll('[data-edge-key][data-share-route-match]').map(edge => edge.getAttribute('data-edge-key')), ['0', '1', '2']);
  assert.equal(routeCard.querySelector('[data-share-route-start]').getAttribute('data-node-id'), 'a');
  assert.equal(routeCard.querySelector('[data-share-route-end]').getAttribute('data-node-id'), 'd');
  assert.equal(routeCard.querySelectorAll('[data-route-match], [data-route-probe-overlay]').length, 0, 'the card is rebuilt from the snapshot, not from live route marks');
  assert.ok(env.api.routeProbe.result(), 'exporting a card keeps the reader on its route');
  assert.equal(env.downloads.at(-1).blob.type, 'image/png');

  await settled(env, () => env.api.exportMenu.run('svg'));
  assert.deepEqual(exportReceipt(env), { format: 'svg', canonical: 'true', variant: null, 'route-state-clean': null, 'reach-state-clean': null, width: null, height: null, 'error-format': null, error: null });
  assert.equal(env.downloads.at(-1).filename, 'payments-platform.svg');
  const canonical = env.serialized.at(-1);
  assert.deepEqual(authoredFacts(canonical), live);
  assert.equal(canonical.querySelectorAll(EXPLORATION_STATE).length, 0);

  env.api.routeProbe.clear();
  for (const [origin, direction] of [['d', 'upstream'], ['a', 'downstream']]) {
    env.api.focus.set(origin);
    assert.equal(env.api.focus.reach(direction), true);
    await settled(env, () => env.api.exportMenu.downloadReachShareCard());
    assert.deepEqual(exportReceipt(env), { format: 'share-card', canonical: 'false', variant: 'reach', 'route-state-clean': null, 'reach-state-clean': 'true', width: '1200', height: '630', 'error-format': null, error: null });
    assert.equal(env.downloads.at(-1).filename, `payments-platform-${direction}-reach-share-card.png`);
    const reachCard = env.serialized.at(-1);
    assert.equal(reachCard.getAttribute('data-share-reach'), direction);
    assert.equal(reachCard.querySelector('[data-share-reach-origin]').getAttribute('data-node-id'), origin);
  }
  assert.equal(env.api.focus.reachability().direction, 'downstream');
  for (const format of ['png', 'svg-light']) {
    await settled(env, () => env.api.exportMenu.run(format));
    assert.equal(exportReceipt(env).canonical, 'true');
    assert.equal(exportReceipt(env).variant, null);
    assert.deepEqual(authoredFacts(env.serialized.at(-1)), live);
    assert.equal(env.serialized.at(-1).querySelectorAll(EXPLORATION_STATE).length, 0);
  }
  assert.deepEqual(env.downloads.map(download => download.filename), ['payments-platform-route-share-card.png', 'payments-platform.svg', 'payments-platform-upstream-reach-share-card.png',
    'payments-platform-downstream-reach-share-card.png', 'payments-platform.png', 'payments-platform-light.svg']);
});

test('without a valid snapshot the share-card items stay hidden and exports report routeRequired/reachRequired', async () => {
  const env = await bootDiagram(diagrams.graph(), 'architecture');
  assert.deepEqual(shareItems(env), [{ action: 'route-share-card', hidden: true, disabled: true }, { action: 'reach-share-card', hidden: true, disabled: true }]);
  await assert.rejects(env.api.exportMenu.shareCard({ variant: 'route' }), { message: catalogText('viewer.export.routeRequired') });
  await assert.rejects(env.api.exportMenu.shareCard({ variant: 'reach' }), { message: catalogText('viewer.export.reachRequired') });
  await assert.rejects(env.api.exportMenu.shareCard({ variant: 'canonical' }), { message: catalogText('viewer.export.unknownVariant', { variant: 'canonical' }) });

  env.click('#export-menu button[data-action="route-share-card"]');
  await env.settle(60);
  assert.deepEqual(env.alerts, [], 'a hidden menu item does nothing');
  for (const [run, required, failed] of [['downloadRouteShareCard', 'viewer.export.routeRequired', 'viewer.export.routeFailed'], ['downloadReachShareCard', 'viewer.export.reachRequired', 'viewer.export.reachFailed']]) {
    await settled(env, () => env.api.exportMenu[run]());
    assert.equal(env.alerts.at(-1), catalogText(failed, { message: catalogText(required) }));
    assert.deepEqual(exportReceipt(env), { format: null, canonical: null, variant: null, 'route-state-clean': null, 'reach-state-clean': null, width: null, height: null, 'error-format': 'share-card', error: catalogText(required) });
  }
  assert.equal(env.downloads.length, 0);

  const conflicted = await bootDiagram(diagrams.graph({ nodes: [...diagrams.GRAPH_NODES, { id: 'b', label: 'Bravo copy', kind: 'backend', x: 240, y: 400 }] }), 'architecture');
  conflicted.api.routeProbe.begin({ source: 'a' });
  assert.equal(conflicted.api.routeProbe.choose('d'), true);
  conflicted.api.focus.set('a', { preserveRoute: true });
  conflicted.api.focus.reach('downstream');
  assert.deepEqual(shareItems(conflicted).map(item => item.hidden), [true, true]);
  await assert.rejects(conflicted.api.exportMenu.shareCard({ variant: 'route' }), { message: catalogText('viewer.export.routeRequired') });
  await assert.rejects(conflicted.api.exportMenu.shareCard({ variant: 'reach' }), { message: catalogText('viewer.export.reachRequired') });
  assert.equal(conflicted.downloads.length, 0);
});

const NEW_TYPES = [
  ['erd', () => diagrams.entityGraph('erd')], ['class', () => diagrams.entityGraph('class')], ['tree', () => diagrams.tree()],
  ['timeline', () => diagrams.timeline()], ['waterfall', () => diagrams.waterfall()],
];
const FORMAT_FILES = { svg: '.svg', 'svg-light': '-light.svg', 'svg-dark': '-dark.svg', png: '.png', jpeg: '.jpeg', webp: '.webp', webm: '.webm' };

for (const [type, build] of NEW_TYPES) {
  test(`${type} offers every export format with canonical receipts and its own file name`, async () => {
    const env = await bootDiagram(build(), type, { title: `Checkout ${type}` });
    assert.equal(env.svg.getAttribute('data-diagram-type'), type);
    assert.equal(env.svg.getAttribute('data-animation'), 'trace', 'export keys motion on data-animation, not on the diagram type or choreography');
    const formats = env.$$('#export-menu button[data-format]').map(item => [item.getAttribute('data-format'), item.disabled]);
    assert.deepEqual(formats, [['png', false], ['jpeg', false], ['webp', false], ['svg', false], ['svg-light', false], ['svg-dark', false], ['webm', false]]);
    assert.equal(env.api.motion.canRecord(), true);
    const live = authoredFacts(env.svg);
    for (const format of Object.keys(FORMAT_FILES)) {
      await settled(env, () => env.api.exportMenu.run(format), format === 'webm' ? 9000 : 120);
      assert.deepEqual(exportReceipt(env), { format: format.startsWith('svg') ? 'svg' : format, canonical: 'true', variant: null, 'route-state-clean': null, 'reach-state-clean': null, width: null, height: null, 'error-format': null, error: null }, `${type} ${format}`);
      assert.equal(env.downloads.at(-1).filename, `checkout-${type}${FORMAT_FILES[format]}`);
      assert.deepEqual(authoredFacts(env.serialized.at(-1)), live, `${type} ${format} keeps every authored node and relationship`);
    }
    assert.equal(env.downloads.length, 7);
    assert.deepEqual(env.alerts, []);
  });
}

test('a diagram without trace motion keeps the static formats and disables WebM', async () => {
  const env = await bootDiagram(diagrams.timeline().replace(' data-animation="trace"', ''), 'timeline');
  const webm = env.$('#export-menu button[data-format="webm"]');
  assert.equal(webm.disabled, true);
  assert.equal(webm.title, catalogText('viewer.export.unsupported'));
  assert.deepEqual(env.$$('#export-menu button[data-format]').filter(item => item.disabled).map(item => item.getAttribute('data-format')), ['webm']);
});

test('tree exports write the complete hierarchy while the live view keeps its collapsed branches', async () => {
  const env = await bootDiagram(diagrams.tree(), 'tree', { title: 'Org Tree' });
  env.api.treeBranches.collapse('payments');
  assert.deepEqual(plain(env.api.treeBranches.collapsedIds()).sort(), ['ops', 'payments']);
  const hiddenLive = env.svg.querySelectorAll('[data-node-id][data-tree-hidden]').map(node => node.getAttribute('data-node-id'));
  assert.deepEqual(hiddenLive, ['card', 'wallet', 'refunds']);
  for (const format of ['svg', 'png', 'svg-dark']) {
    await settled(env, () => env.api.exportMenu.run(format));
    const clone = env.serialized.at(-1);
    assert.equal(exportReceipt(env).canonical, 'true');
    assert.equal(clone.hasAttribute('data-tree-any-collapsed'), false);
    assert.equal(clone.querySelectorAll('[data-tree-hidden], [data-tree-collapsed]').length, 0, `${format} contains every branch`);
    assert.deepEqual(clone.querySelectorAll('[data-tree-toggle]').map(toggle => [toggle.getAttribute('aria-expanded'), toggle.getAttribute('aria-label') === toggle.getAttribute('data-tree-label-collapse')]), [['true', true], ['true', true], ['true', true]]);
    assert.equal(clone.querySelectorAll('[data-node-id]').length, 6);
    assert.equal(clone.querySelectorAll('[data-edge-from]').length, 5);
  }
  assert.deepEqual(plain(env.api.treeBranches.collapsedIds()).sort(), ['ops', 'payments'], 'export never changes the reading state');
  assert.deepEqual(env.svg.querySelectorAll('[data-node-id][data-tree-hidden]').map(node => node.getAttribute('data-node-id')), hiddenLive);
  assert.deepEqual(env.downloads.map(download => download.filename), ['org-tree.svg', 'org-tree.png', 'org-tree-dark.svg']);
});

test('printing a collapsed tree shows every branch and restores the reading state afterwards', async () => {
  const env = await bootDiagram(diagrams.tree(), 'tree');
  env.api.treeBranches.collapse('payments');
  env.setMedia('print', true);
  dispatch(env.window, 'beforeprint');
  await env.settle(40);
  assert.equal(env.svg.querySelectorAll('[data-node-id][data-tree-hidden], [data-edge-from][data-tree-hidden], [data-tree-collapsed]').length, 0, 'print shows the complete tree');
  assert.equal(env.svg.hasAttribute('data-tree-any-collapsed'), false);
  env.setMedia('print', false);
  dispatch(env.window, 'afterprint');
  await env.settle(40);
  assert.deepEqual(plain(env.api.treeBranches.collapsedIds()).sort(), ['ops', 'payments']);
  assert.deepEqual(env.svg.querySelectorAll('[data-node-id][data-tree-hidden]').map(node => node.getAttribute('data-node-id')), ['card', 'wallet', 'refunds']);
});
