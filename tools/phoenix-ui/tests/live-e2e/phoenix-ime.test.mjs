/**
 * IME Enter guard in the Phoenix live overlay, driven through Chrome's own
 * composition pipeline (CDP Input.imeSetComposition / Input.insertText), so
 * the page sees the composition and keyboard events Chrome produces rather
 * than synthetic KeyboardEvents.
 *
 * For the annotation pin input, the configure input, the insert input and the
 * page-level Steer chat input, every ordering composes a sentence in two IME compositions, presses the
 * candidate-commit Enter after each one, then presses a plain Enter:
 *
 *   chrome        keydown(229, isComposing) -> compositionend -> keyup(13)
 *   safari        compositionend -> keydown(229, !isComposing) -> keyup(13)
 *   composingOnly keydown(13, isComposing) -> compositionend -> keyup(13)
 *
 * The commit Enters must not submit; the plain Enter must submit exactly
 * once with the whole sentence. A Chromium composition is not a human IME
 * session and not Safari: `safari` only reproduces Safari's event ordering.
 * Chrome runs headed with its window off-screen (see `before`).
 *
 * Run:
 *   IMPECCABLE_BIN=<phoenix-ui engine> node --test --test-concurrency=1 \
 *     tests/live-e2e/phoenix-ime.test.mjs
 */

import { after, before, describe, it } from 'node:test';
import assert from 'node:assert/strict';

import {
  bootPhoenixSession, browserInfo, createRecorder, engineInfo, launchChrome, recordingAgent,
  removeNodeModulesTemplates, sourceState, waitFor, writeEvidence,
} from './phoenix-harness.mjs';
import {
  clickDiscard, pickElement, waitForBarHidden, waitForCycling, waitForHandshake, enableInsertMode,
  installLiveQueryHelpers, preparePageForBarInteraction, waitForSteerInputFocused, waitForSteerUnlocked,
} from './ui.mjs';

const PARTS = [
  { text: '把标题', steps: ['b', 'ba', 'babiao', 'babiaoti', '把标题'] },
  { text: '改成红色', steps: ['g', 'gai', 'gaicheng', 'gaichenghongse', '改成红色'] },
];
const FULL = PARTS.map((p) => p.text).join('');
const SETTLE_MS = 400;

const ORDERINGS = {
  async chrome(cdp, part) {
    await compose(cdp, part.steps);
    await enterDown(cdp, 229);
    await cdp.send('Input.insertText', { text: part.text });
    await enterUp(cdp);
  },
  async safari(cdp, part) {
    await compose(cdp, part.steps);
    await cdp.send('Input.insertText', { text: part.text });
    await enterDown(cdp, 229);
    await enterUp(cdp);
  },
  async composingOnly(cdp, part) {
    await compose(cdp, part.steps);
    await enterDown(cdp, 13);
    await cdp.send('Input.insertText', { text: part.text });
    await enterUp(cdp);
  },
};

async function compose(cdp, steps) {
  for (const text of steps) {
    await cdp.send('Input.imeSetComposition', { text, selectionStart: text.length, selectionEnd: text.length });
  }
}

async function enterDown(cdp, keyCode) {
  await cdp.send('Input.dispatchKeyEvent', {
    type: 'rawKeyDown', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: keyCode, nativeVirtualKeyCode: keyCode,
  });
}

async function enterUp(cdp) {
  await cdp.send('Input.dispatchKeyEvent', {
    type: 'keyUp', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13, nativeVirtualKeyCode: 13,
  });
}

function installDomEventLogInPage() {
  if (window.__phoenixImeLog) return;
  window.__phoenixImeLog = [];
  for (const type of ['keydown', 'keyup', 'beforeinput', 'input', 'compositionstart', 'compositionupdate', 'compositionend']) {
    window.addEventListener(type, (e) => {
      const target = e.composedPath()[0];
      if (!target || !/^(INPUT|TEXTAREA)$/.test(target.tagName || '')) return;
      const entry = {
        t: Math.round(performance.now()),
        type,
        target: target.id || target.placeholder || target.tagName,
        key: e.key ?? null,
        code: e.code ?? null,
        keyCode: e.keyCode ?? null,
        isComposing: e.isComposing ?? null,
        inputType: e.inputType ?? null,
        data: e.data ?? null,
        value: target.value,
      };
      window.__phoenixImeLog.push(entry);
      // Read after dispatch finished: did the overlay consume the key?
      setTimeout(() => {
        entry.defaultPrevented = e.defaultPrevented;
        entry.targetConnectedAfter = target.isConnected;
      }, 0);
    }, true);
  }
}

async function drainDomLog(page) {
  await page.waitForTimeout(20);
  return page.evaluate(() => window.__phoenixImeLog.splice(0));
}

async function liveState(page) {
  return page.evaluate(() => window.__PHOENIX_UI_LIVE_CHROME_CORE__?.debugState?.()?.state ?? null);
}

async function activeInput(page) {
  return page.evaluate(() => {
    const el = window.__PHOENIX_UI_LIVE_CHROME_CORE__?.activeElementDeep?.();
    return el ? { tag: el.tagName, id: el.id || null, placeholder: el.placeholder || null, value: el.value ?? null } : null;
  });
}

let browser;
let session;
let cdp;
const rec = createRecorder('ime');
const results = [];
const meta = {};

before(async () => {
  meta.engine = engineInfo();
  // Headless Chrome 154 freezes the renderer on the second CDP IME
  // composition in a page, even on a bare <input>; a headed, off-screen
  // window composes normally.
  browser = await launchChrome({ headless: false, args: ['--window-position=-2400,-2400', '--window-size=1280,900'] });
  meta.browser = browserInfo(browser);
  // Steer is recorded and acknowledged without a source edit, so the staged
  // project stays clean for the variant cases that follow.
  const agent = {
    ...recordingAgent(rec),
    async handleSteer(event) {
      rec.agentEvents.push({ t: rec.t(), event: 'agent.steer.payload', id: event.id, message: event.message ?? null });
      return { message: 'IME steer received' };
    },
  };
  session = await bootPhoenixSession({ browser, rec, agent });
  meta.tmp = session.tmp;
  meta.live = { pid: session.live.pid, port: session.live.port };
  meta.devPort = session.devPort;
  await waitForHandshake(session.page);
  await session.page.evaluate(installDomEventLogInPage);
  cdp = await session.page.context().newCDPSession(session.page);
});

after(async () => {
  rec.save('events/ime.json');
  writeEvidence('events/ime-results.json', { meta, results });
  if (session) {
    meta.sourceAfterSuite = sourceState(session.tmp);
    await session.teardown();
    meta.sourceAfterTeardown = sourceState(session.tmp);
    writeEvidence('events/ime-results.json', { meta, results });
    session.remove();
  }
  if (browser) await browser.close();
  removeNodeModulesTemplates();
});

async function resetToIdle(page) {
  for (let i = 0; i < 3; i++) {
    const state = await liveState(page);
    if (state === 'PICKING' || state === 'IDLE') break;
    await page.keyboard.press('Escape');
    await page.waitForTimeout(200);
  }
  await waitFor(() => sourceState(session.tmp).diff === sourceState(session.tmp).diff
    && !/data-impeccable-variants/.test(sourceState(session.tmp).diff), { label: 'clean source', timeout: 15_000 });
  await session.page.evaluate(() => window.__phoenixImeLog.splice(0));
}

/** Compose both parts; after each commit Enter nothing may have been sent. */
async function composeSentence({ page, ordering, countRequests, expectFocus, expectState = 'CONFIGURING' }) {
  const before = countRequests();
  const steps = [];
  let committed = '';
  for (const part of PARTS) {
    await ORDERINGS[ordering](cdp, part);
    await page.waitForTimeout(SETTLE_MS);
    committed += part.text;
    const focus = await activeInput(page);
    const state = await liveState(page);
    const dom = await drainDomLog(page);
    steps.push({ part: part.text, requestsSoFar: countRequests() - before, state, focus, dom });
    assert.equal(countRequests(), before, `${ordering}: commit Enter after "${part.text}" must not submit`);
    assert.equal(state, expectState, `${ordering}: state unchanged after commit Enter`);
    assert.ok(expectFocus(focus), `${ordering}: focus stays in the input (${JSON.stringify(focus)})`);
    assert.equal(focus.value, committed, `${ordering}: input holds the committed text`);
  }
  return { before, steps };
}

async function discardAndWaitClean(page) {
  await waitForCycling(page, 3, { timeout: 60_000 });
  await clickDiscard(page);
  await waitForBarHidden(page, { timeout: 15_000 });
  await waitFor(() => sourceState(session.tmp).diff.includes('data-impeccable') ? null : true, { label: 'discard restored source', timeout: 20_000 });
}

function generatePosts() {
  return rec.eventPosts('generate');
}

function agentGenerates() {
  return rec.agentEvents.filter((e) => e.event === 'agent.generate.payload');
}

function agentSteers() {
  return rec.agentEvents.filter((e) => e.event === 'agent.steer.payload');
}

for (const ordering of Object.keys(ORDERINGS)) {
  describe(`IME ${ordering} ordering`, () => {
    it('configure input: commit Enter keeps composing, plain Enter sends one full request', async () => {
      const page = session.page;
      await resetToIdle(page);
      await pickElement(page, 'h1.hero-title');
      await page.locator('#impeccable-live-input').click();
      await drainDomLog(page);
      const agentBefore = agentGenerates().length;
      const composed = await composeSentence({
        page, ordering,
        countRequests: () => generatePosts().length,
        expectFocus: (f) => f?.id === 'impeccable-live-input',
      });
      await page.keyboard.press('Enter');
      await waitFor(() => generatePosts().length > composed.before, { label: 'generate POST' });
      await page.waitForTimeout(1_500);
      const sent = generatePosts().slice(composed.before);
      const dom = await drainDomLog(page);
      await waitFor(() => agentGenerates().length > agentBefore, { label: 'agent leased generate', timeout: 20_000 });
      const leased = agentGenerates().slice(agentBefore);
      results.push({ ordering, input: 'configure', composed: composed.steps, plainEnterDom: dom, sent: sent.map((r) => r.body), leased });
      assert.equal(sent.length, 1, 'exactly one generate request');
      assert.equal(sent[0].body.freeformPrompt, FULL, 'request carries the whole sentence');
      assert.equal(leased.length, 1, 'the agent leased exactly one generate');
      assert.equal(leased[0].freeformPrompt, FULL);
      await discardAndWaitClean(page);
    });

    it('annotation pin input: commit Enter keeps editing, plain Enter commits the full note, Go sends it once', async () => {
      const page = session.page;
      await resetToIdle(page);
      await pickElement(page, 'h1.hero-title');
      const rect = await waitFor(() => page.evaluate(() => {
        const root = window.__PHOENIX_UI_LIVE_CHROME_CORE__.root();
        const el = root.querySelector('#impeccable-live-annot') || document.querySelector('#impeccable-live-annot');
        if (!el || el.style.display === 'none') return null;
        const r = el.getBoundingClientRect();
        return r.width > 20 && r.height > 20 ? { x: r.left, y: r.top, w: r.width, h: r.height } : null;
      }), { label: 'annotation overlay' });
      // The bar focuses its configure input ~60 ms after the pick; a pin opened
      // before that would lose focus to it at once.
      await waitFor(() => activeInput(page).then((f) => f?.id === 'impeccable-live-input' ? f : null), { label: 'configure input autofocus' });
      await page.mouse.click(rect.x + Math.min(28, rect.w * 0.2), rect.y + Math.min(20, rect.h * 0.35));
      await waitFor(() => activeInput(page).then((f) => f?.placeholder === 'Note…' ? f : null), { label: 'pin input focused' });
      await drainDomLog(page);
      const agentBefore = agentGenerates().length;
      const composed = await composeSentence({
        page, ordering,
        countRequests: () => rec.helperRequests((r) => r.method === 'POST').length,
        expectFocus: (f) => f?.placeholder === 'Note…',
      });
      await page.keyboard.press('Enter');
      await page.waitForTimeout(SETTLE_MS);
      const pinDom = await drainDomLog(page);
      const pins = await page.evaluate(() => {
        const root = window.__PHOENIX_UI_LIVE_CHROME_CORE__.root();
        const pinsEl = root.querySelector('#impeccable-live-annot-pins');
        return { text: pinsEl?.textContent || '', inputs: pinsEl?.querySelectorAll('input').length ?? null };
      });
      const postsAfterPin = rec.helperRequests((r) => r.method === 'POST').length - composed.before;
      assert.equal(pins.inputs, 0, 'plain Enter closed the pin editor');
      assert.ok(pins.text.includes(FULL), `pin shows the whole note (${pins.text})`);
      assert.equal(postsAfterPin, 0, 'committing a pin sends nothing by itself');

      const genBefore = generatePosts().length;
      await page.locator('#impeccable-live-bar button[aria-label="Generate variants"]').click();
      await waitFor(() => generatePosts().length > genBefore, { label: 'generate POST' });
      await page.waitForTimeout(1_500);
      const sent = generatePosts().slice(genBefore);
      await waitFor(() => agentGenerates().length > agentBefore, { label: 'agent leased generate', timeout: 20_000 });
      const leased = agentGenerates().slice(agentBefore);
      results.push({ ordering, input: 'annotation', composed: composed.steps, plainEnterDom: pinDom, pins, sent: sent.map((r) => r.body), leased });
      assert.equal(sent.length, 1, 'exactly one generate request');
      assert.deepEqual(sent[0].body.comments.map((c) => c.text), [FULL], 'one comment with the whole note');
      assert.equal(leased.length, 1);
      assert.deepEqual(leased[0].comments.map((c) => c.text), [FULL]);
      await discardAndWaitClean(page);
    });

    it('insert input: commit Enter keeps composing, plain Enter sends one full insert request', async () => {
      const page = session.page;
      await resetToIdle(page);
      await enableInsertMode(page);
      const box = await page.locator('section#features').boundingBox();
      const x = box.x + box.width / 2;
      const y = box.y + box.height - 4;
      await page.mouse.move(x, y);
      await page.waitForTimeout(150);
      await page.mouse.click(x, y);
      await page.locator('#impeccable-live-insert-input').click();
      await drainDomLog(page);
      const agentBefore = agentGenerates().length;
      const composed = await composeSentence({
        page, ordering,
        countRequests: () => generatePosts().length,
        expectFocus: (f) => f?.id === 'impeccable-live-insert-input',
      });
      await page.keyboard.press('Enter');
      await waitFor(() => generatePosts().length > composed.before, { label: 'generate POST' });
      await page.waitForTimeout(1_500);
      const sent = generatePosts().slice(composed.before);
      const dom = await drainDomLog(page);
      await waitFor(() => agentGenerates().length > agentBefore, { label: 'agent leased generate', timeout: 20_000 });
      const leased = agentGenerates().slice(agentBefore);
      results.push({ ordering, input: 'insert', composed: composed.steps, plainEnterDom: dom, sent: sent.map((r) => r.body), leased });
      assert.equal(sent.length, 1, 'exactly one generate request');
      assert.equal(sent[0].body.mode, 'insert');
      assert.equal(sent[0].body.freeformPrompt, FULL, 'request carries the whole sentence');
      assert.equal(leased.length, 1);
      assert.equal(leased[0].freeformPrompt, FULL);
      await discardAndWaitClean(page);
    });

    it('page chat input: commit Enter keeps composing, plain Enter sends one full steer', async () => {
      const page = session.page;
      await resetToIdle(page);
      await installLiveQueryHelpers(page);
      await preparePageForBarInteraction(page);
      await page.locator('#impeccable-live-page-chat').click();
      await waitForSteerInputFocused(page);
      await drainDomLog(page);
      const idleState = await liveState(page);
      const sourceBefore = sourceState(session.tmp);
      const agentBefore = agentSteers().length;
      const composed = await composeSentence({
        page, ordering,
        countRequests: () => rec.eventPosts('steer').length,
        expectFocus: (f) => f?.id === 'impeccable-live-page-chat-input',
        expectState: idleState,
      });
      await page.keyboard.press('Enter');
      await waitFor(() => rec.eventPosts('steer').length > composed.before, { label: 'steer POST' });
      await page.waitForTimeout(1_500);
      const sent = rec.eventPosts('steer').slice(composed.before);
      const dom = await drainDomLog(page);
      await waitFor(() => agentSteers().length > agentBefore, { label: 'agent leased steer', timeout: 20_000 });
      await waitForSteerUnlocked(page, { timeout: 20_000 });
      const leased = agentSteers().slice(agentBefore);
      results.push({ ordering, input: 'page-chat', composed: composed.steps, plainEnterDom: dom, sent: sent.map((r) => r.body), leased });
      assert.equal(sent.length, 1, 'exactly one steer request');
      assert.equal(sent[0].body.message, FULL, 'steer carries the whole sentence');
      assert.equal(leased.length, 1, 'the agent leased exactly one steer');
      assert.equal(leased[0].message, FULL);
      assert.deepEqual(sourceState(session.tmp), sourceBefore, 'acknowledged steer left the project unchanged');
    });
  });
}
