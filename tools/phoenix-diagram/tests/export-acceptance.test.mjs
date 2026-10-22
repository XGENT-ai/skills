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
import { artifactIdentity, validateInput, verifyFileSignature, verifyCanonicalFacts, verifyPrintSample, configureNetwork, runExportAcceptance, runContentAcceptance, REQUIRED_EXPORT_SCOPE } from './export-acceptance.mjs';
import { DEFAULT_TIMEOUTS, ownedProfile, discoverBrowser, browserArguments, PipeCdp } from './support/export-browser.mjs';
import * as probes from './support/export-page.mjs';
import { REQUIRED_READER_INTERACTIONS, sendReaderKey, verifyReaderInteractions } from './support/export-reader.mjs';
import * as acceptance from './export-acceptance.mjs';

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
  assert.ok(args.includes('--remote-debugging-pipe')); assert.ok(args.includes('--user-data-dir=' + directory)); assert.ok(!args.includes('--no-sandbox'));
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

test('fixed browser probes parse without evaluating any authored literal', () => {
  for (const probe of Object.values(probes)) new vm.Script('(' + probe.toString() + ')');
  const literal = '</script><script>globalThis.__phoenixDiagramXss=1</script> $&/{{PHOENIX:SVG}}';
  const expression = '(' + probes.contentObservation.toString() + ')(' + JSON.stringify({ expectedLiteral: literal, sentinelName: '__phoenixDiagramXss' }) + ')';
  new vm.Script(expression);
  assert.equal(globalThis.__phoenixDiagramXss, undefined);
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
