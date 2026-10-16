// Phoenix Reader boundary tests, adapted from the frozen Archify 3.1.0
// export-browser, export-cleanup-browser and motion-governor-browser tests.
// MIT notices remain in ../../viewer/LICENSE.txt. The DOM/media providers here
// are deterministic fixtures: these tests do not certify Chrome or media bytes.
import test from 'node:test';
import assert from 'node:assert/strict';
import { Element, observe, runtime } from './fixtures/runtime.mjs';

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
