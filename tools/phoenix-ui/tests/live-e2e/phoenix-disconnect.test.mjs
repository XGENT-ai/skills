/**
 * Live server stop and abnormal exit as the open page sees them.
 *
 * - `live-server stop --keep-inject` and a SIGKILLed helper leave live.js in
 *   the page: the page must say the server stopped and how to restart, and a
 *   restart plus reload must reconnect.
 * - A normal `live-server stop` removes the injected script from the source.
 *
 * Run:
 *   IMPECCABLE_BIN=<phoenix-ui engine> node --test --test-concurrency=1 \
 *     tests/live-e2e/phoenix-disconnect.test.mjs
 */

import { after, before, it } from 'node:test';
import assert from 'node:assert/strict';

import {
  bootPhoenixSession, browserInfo, createRecorder, engine, engineInfo, evidencePath, injectScript, launchChrome,
  pidAlive, processRecord, removeNodeModulesTemplates, sourceState, startHelper, waitFor, writeEvidence,
} from './phoenix-harness.mjs';
import { installLiveQueryHelpers, waitForHandshake } from './ui.mjs';

const TOAST = '#impeccable-live-toast';
// EventSource retries about every 3 s and the page gives up after 20 of them.
const NOTICE_TIMEOUT_MS = 150_000;

let browser;
let session;
const rec = createRecorder('disconnect');
const results = [];
const meta = {};

async function toastText(page) {
  await installLiveQueryHelpers(page);
  return page.evaluate((sel) => window.__impeccableLiveQuery(sel)?.textContent ?? null, TOAST);
}

async function waitForNotice(page) {
  return waitFor(async () => {
    const text = await toastText(page);
    return text && /live server stopped/i.test(text) ? text : null;
  }, { label: 'disconnect notice', timeout: NOTICE_TIMEOUT_MS, interval: 250 });
}

async function restartAndReconnect() {
  const restarted = startHelper(session.tmp);
  rec.trackLive(restarted);
  session.live = restarted;
  injectScript(session.tmp, restarted);
  await session.page.reload({ waitUntil: 'domcontentloaded' });
  await waitForHandshake(session.page, { timeout: 30_000 });
  return restarted;
}

before(async () => {
  meta.engine = engineInfo();
  browser = await launchChrome();
  meta.browser = browserInfo(browser);
  session = await bootPhoenixSession({ browser, rec });
  meta.tmp = session.tmp;
  await waitForHandshake(session.page);
});

after(async () => {
  rec.save('events/disconnect.json');
  if (session) {
    await session.teardown();
    meta.sourceAfterTeardown = sourceState(session.tmp);
    session.remove();
  }
  writeEvidence('events/disconnect-results.json', { meta, results });
  if (browser) await browser.close();
  removeNodeModulesTemplates();
});

it('a stopped helper whose script stays tells the page it disconnected and how to restart', async () => {
  const { page, tmp } = session;
  const before = session.live;
  const stop = engine('live-server', ['stop', '--keep-inject'], { cwd: tmp });
  const started = Date.now();
  await waitFor(() => !pidAlive(before.pid), { label: 'helper exit', timeout: 15_000 });
  const text = await waitForNotice(page);
  const noticeAfterMs = Date.now() - started;
  await page.screenshot({ path: evidencePath('screenshots/disconnect-keep-inject.png') });
  const restarted = await restartAndReconnect();
  results.push({ case: 'stop --keep-inject', stop: { code: stop.code, stdout: stop.stdout }, noticeAfterMs, text,
    before: processRecord('stopped helper', before), restarted: processRecord('restarted helper', restarted) });
  assert.equal(stop.code, 0, stop.stderr);
  assert.match(text, /\/phoenix-ui live/);
  assert.match(text, /reload this page/);
});

it('a killed helper shows the same notice and the next start recovers', async () => {
  const { page, tmp } = session;
  const before = session.live;
  process.kill(before.pid, 'SIGKILL');
  const started = Date.now();
  await waitFor(() => !pidAlive(before.pid), { label: 'helper killed', timeout: 15_000 });
  const text = await waitForNotice(page);
  const noticeAfterMs = Date.now() - started;
  await page.screenshot({ path: evidencePath('screenshots/disconnect-killed.png') });
  const status = engine('live-status', [], { cwd: tmp });
  const restarted = await restartAndReconnect();
  results.push({ case: 'SIGKILL', noticeAfterMs, text, statusAfterKill: { code: status.code, json: status.json, stdout: status.stdout },
    restart: restarted.result.json, restarted: processRecord('restarted helper', restarted) });
  assert.match(text, /\/phoenix-ui live/);
});

it('a normal stop removes the injected script from the source', async () => {
  const { tmp } = session;
  const before = session.live;
  const stop = engine('live-server', ['stop'], { cwd: tmp });
  await waitFor(() => !pidAlive(before.pid), { label: 'helper exit', timeout: 15_000 });
  const state = sourceState(tmp);
  results.push({ case: 'stop', stop: { code: stop.code, stdout: stop.stdout }, sourceAfterStop: state });
  assert.equal(stop.code, 0, stop.stderr);
  assert.doesNotMatch(state.diff, /live\.js/);
});
