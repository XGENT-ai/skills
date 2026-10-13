#!/usr/bin/env node
/**
 * Plan §6 rows 2, 3 and 5 in real Safari (macOS), driven over W3C WebDriver
 * (`safaridriver -p`) with phoenix-safari-webdriver.mjs. Everything that is
 * not the browser comes from the Chrome harness (phoenix-harness.mjs): fixture
 * staging, helper, inject, dev server, deterministic agent, engine verbs.
 * The assertions mirror the Chrome tests:
 *
 *   row2  phoenix-disconnect.test.mjs  stop --keep-inject / SIGKILL show the
 *         restart notice and recover; a normal stop removes the script.
 *   row3  phoenix-variants.test.mjs (row 3)  discard restores byte-for-byte;
 *         accept leaves only the chosen variant; stop leaves no script/CSP
 *         residue; moved workspaces never get written at the old path.
 *   row5  phoenix-question.test.mjs  text cards comparable and choosable;
 *         keyboard reaches every choose button with a visible ring and Enter
 *         picks; desktop and ~375px; broken art never blocks a pick.
 *
 * Safari has no init scripts, CDP or network log. The overlay is read with
 * the Phoenix globals (__PHOENIX_UI_LIVE_*) directly; after each load the page
 * gets a small recorder that wraps window.fetch (POST /events, /answer),
 * console and error/CSP events, so request counts and violations are only
 * seen from that moment on.
 *
 * Run (cwd tools/phoenix-ui):
 *   IMPECCABLE_BIN=<phoenix-ui engine> node tests/live-e2e/phoenix-safari-walkthrough.mjs \
 *     [row2] [row3] [row5] [--wait-automation=<min>] [--poll=<s>] [--busy-wait=<min>]
 *   node tests/live-e2e/phoenix-safari-walkthrough.mjs --probe
 *
 * Evidence: local/phoenix-ui/m3-safari/<row>/ (PHOENIX_SAFARI_EVIDENCE_DIR).
 * Exit: 0 all pass, 1 a row failed, 2 Safari automation unavailable.
 */

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, renameSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  KEYS, SAFARIDRIVER, freePort, newSafariSession, probeAutomation, startSafariDriver,
} from './phoenix-safari-webdriver.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const CHECKOUT_ROOT = resolve(__dirname, '..', '..', '..', '..');
const OUT = process.env.PHOENIX_SAFARI_EVIDENCE_DIR || join(CHECKOUT_ROOT, 'local', 'phoenix-ui', 'm3-safari');
// Harness evidence helpers (writeEvidence/evidencePath) land here too.
process.env.PHOENIX_E2E_EVIDENCE_DIR = OUT;

const ROW_DIRS = { row2: 'row2-disconnect', row3: 'row3-accept-discard-move', row5: 'row5-question-cards' };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const iso = () => new Date().toISOString();

// ---------------------------------------------------------------------------
// Arguments, logging, evidence
// ---------------------------------------------------------------------------

const argv = process.argv.slice(2);
const flag = (name, dflt) => {
  const hit = argv.find((a) => a === `--${name}` || a.startsWith(`--${name}=`));
  if (!hit) return dflt;
  return hit.includes('=') ? hit.slice(hit.indexOf('=') + 1) : true;
};
const PROBE = flag('probe', false) === true;
const WAIT_AUTOMATION_MIN = Number(flag('wait-automation', 0));
const POLL_S = Number(flag('poll', 60));
const BUSY_WAIT_MIN = Number(flag('busy-wait', 30));
const rowsWanted = argv.filter((a) => !a.startsWith('--'));
const ROWS = rowsWanted.length ? rowsWanted : ['row2', 'row3', 'row5'];
for (const r of ROWS) if (!ROW_DIRS[r]) { console.error(`unknown row ${r}; use row2 row3 row5`); process.exit(64); }

let currentLog = [];
function log(msg) {
  const line = `[${iso()}] ${msg}`;
  currentLog.push(line);
  console.log(line);
}

function outPath(...parts) {
  const file = join(OUT, ...parts);
  mkdirSync(dirname(file), { recursive: true });
  return file;
}

function writeJson(file, data) {
  writeFileSync(file, JSON.stringify(data, null, 2) + '\n');
  return file;
}

function slug(s) { return String(s).replace(/[^a-z0-9]+/gi, '-').replace(/^-|-$/g, '').toLowerCase(); }

function safariAppVersion() {
  try {
    return execFileSync('defaults', ['read', '/Applications/Safari.app/Contents/Info.plist', 'CFBundleShortVersionString'], { encoding: 'utf-8' }).trim();
  } catch { return null; }
}

function allTrue(checks) { return Object.values(checks).every((v) => v === true); }

/** Other live-e2e phoenix tests (this runner excluded). */
function otherPhoenixTests() {
  let out = '';
  try { out = execFileSync('pgrep', ['-fl', 'tests/live-e2e/phoenix-'], { encoding: 'utf-8' }); } catch { return []; }
  return out.split('\n').filter(Boolean).map((l) => {
    const sp = l.indexOf(' ');
    return { pid: Number(l.slice(0, sp)), cmd: l.slice(sp + 1) };
  }).filter((p) => p.pid !== process.pid && !p.cmd.includes('phoenix-safari-'));
}

async function waitUntilIdle(label) {
  const started = Date.now();
  let others = otherPhoenixTests();
  while (others.length) {
    if (Date.now() - started > BUSY_WAIT_MIN * 60_000) {
      throw new Error(`other phoenix live-e2e tests still running after ${BUSY_WAIT_MIN} min: ${others.map((o) => o.pid).join(',')}`);
    }
    log(`${label}: waiting for other phoenix live-e2e tests (${others.map((o) => o.pid).join(',')})`);
    await sleep(30_000);
    others = otherPhoenixTests();
  }
  return { waitedMs: Date.now() - started };
}

// ---------------------------------------------------------------------------
// In-page helpers (prelude available to every script) and the page recorder
// ---------------------------------------------------------------------------

const PRELUDE = `
function liveRoot() {
  const core = window.__PHOENIX_UI_LIVE_CHROME_CORE__;
  return (core && core.root && core.root()) || window.__PHOENIX_UI_LIVE_UI_ROOT__ || null;
}
function liveQ(sel) {
  const r = liveRoot();
  return (r && r.querySelector && r.querySelector(sel)) || document.querySelector(sel);
}
function deepHit(x, y) {
  let el = document.elementFromPoint(x, y);
  while (el && el.shadowRoot) {
    const inner = el.shadowRoot.elementFromPoint(x, y);
    if (!inner || inner === el) break;
    el = inner;
  }
  return el;
}
function findBarButton(m) {
  const bar = liveQ('#impeccable-live-bar');
  if (!bar) return null;
  return [...bar.querySelectorAll('button')].find((b) => {
    const t = b.textContent || '';
    if (m.kind === 'regex') return new RegExp(m.source, m.flags).test(t);
    if (m.kind === 'aria') return (b.getAttribute('aria-label') || '') === m.value;
    return t.includes(m.value);
  }) || null;
}
function liveTarget(t) {
  if (t.selector) return liveQ(t.selector);
  if (t.bar) return findBarButton(t.bar);
  return null;
}
`;

function installPageRecorder() {
  if (window.__phoenixSafariRec) return false;
  const rec = { installedAt: Date.now(), posts: [], csp: [], errors: [], logs: [] };
  window.__phoenixSafariRec = rec;
  const origFetch = window.fetch;
  window.fetch = function recordedFetch(input, init) {
    try {
      const url = new URL(String((input && input.url) || input), location.href);
      const method = String((init && init.method) || (input && input.method) || 'GET').toUpperCase();
      if (method === 'POST' && ['/events', '/answer'].includes(url.pathname)) {
        let body = null;
        try { body = JSON.parse(init && init.body); } catch (e) { /* not JSON */ }
        rec.posts.push({ t: Date.now(), path: url.pathname, port: url.port, type: (body && body.type) || null,
          id: (body && body.id) || null, optionId: (body && body.optionId) || null, body: url.pathname === '/answer' ? (init && init.body) : null });
      }
    } catch (e) { /* never break the page */ }
    return origFetch.apply(this, arguments);
  };
  document.addEventListener('securitypolicyviolation', (e) => rec.csp.push({ t: Date.now(), directive: e.violatedDirective, blocked: e.blockedURI, source: e.sourceFile || null }));
  window.addEventListener('error', (e) => {
    const resource = !!(e.target && e.target !== window);
    rec.errors.push({ t: Date.now(), resource, text: resource ? `load failed: ${e.target.tagName} ${e.target.src || e.target.href || ''}` : String(e.message) });
  }, true);
  window.addEventListener('unhandledrejection', (e) => rec.errors.push({ t: Date.now(), resource: false, text: `unhandledrejection: ${String((e.reason && e.reason.message) || e.reason)}` }));
  for (const level of ['log', 'warn', 'error']) {
    const orig = console[level];
    console[level] = function recordedConsole() {
      try {
        const text = Array.prototype.map.call(arguments, (a) => String(a)).join(' ');
        if (level === 'error' || /Phoenix UI|impeccable/i.test(text)) rec.logs.push({ t: Date.now(), level, text: text.slice(0, 500) });
      } catch (e) { /* ignore */ }
      return orig.apply(this, arguments);
    };
  }
  return true;
}

function drainPageRecorder() {
  const r = window.__phoenixSafariRec;
  if (!r) return null;
  return { posts: r.posts.splice(0), csp: r.csp.splice(0), errors: r.errors.splice(0), logs: r.logs.splice(0) };
}

function newPageLog() { return { posts: [], csp: [], errors: [], logs: [] }; }

async function drain(wd, pageLog, rec) {
  const got = await wd.exec(drainPageRecorder).catch(() => null);
  if (!got) return pageLog;
  for (const k of Object.keys(pageLog)) {
    for (const item of got[k] || []) pageLog[k].push(rec ? JSON.parse(rec.clean(JSON.stringify(item))) : item);
  }
  return pageLog;
}

// ---------------------------------------------------------------------------
// Live overlay driving (Safari equivalents of ui.mjs)
// ---------------------------------------------------------------------------

const BAR_ID = '#impeccable-live-bar';
const GLOBAL_BAR_ID = '#impeccable-live-global-bar';
const PICK_TOGGLE = '#impeccable-live-pick-toggle';
const GO_LABEL = 'Generate variants';

function barMatch(label) {
  if (label instanceof RegExp) return { kind: 'regex', source: label.source, flags: label.flags };
  if (label && typeof label === 'object' && label.ariaLabel) return { kind: 'aria', value: label.ariaLabel };
  return { kind: 'text', value: String(label) };
}

function locateLive(target) {
  const el = liveTarget(target);
  if (!el) return null;
  const r = el.getBoundingClientRect();
  const cs = getComputedStyle(el);
  const cx = r.left + r.width / 2;
  const cy = r.top + r.height / 2;
  const hit = deepHit(cx, cy);
  return {
    box: { x: r.left, y: r.top, w: r.width, h: r.height, cx, cy },
    visible: r.width > 0 && r.height > 0 && cs.visibility !== 'hidden' && cs.display !== 'none',
    disabled: !!el.disabled || cs.pointerEvents === 'none',
    inViewport: cx >= 0 && cy >= 0 && cx < innerWidth && cy < innerHeight,
    hitsSelf: !!hit && (hit === el || el.contains(hit)),
    hit: hit ? `${hit.tagName.toLowerCase()}${hit.id ? '#' + hit.id : ''}` : null,
    text: (el.textContent || el.getAttribute('aria-label') || '').trim().slice(0, 40),
  };
}

function jsClickLive(target) {
  const el = liveTarget(target);
  if (!el || el.disabled) return false;
  el.click();
  return true;
}

async function waitFor(fn, opts) {
  const { waitFor: wf } = await harness();
  return wf(fn, opts);
}

/**
 * A real pointer click on a live control at its on-screen centre once it is
 * visible, enabled and on top; a DOM click only as the last resort (the Chrome
 * helpers' dispatch fallback). Every click is logged with its method.
 */
async function clickLive(ctx, target, label) {
  const { wd } = ctx;
  let last = null;
  for (let i = 0; i < 6; i++) {
    last = await wd.exec(locateLive, target).catch((e) => ({ error: e.message }));
    if (last && last.visible && !last.disabled && last.inViewport && last.hitsSelf) {
      // Hover can reflow the toolbar; wait for its target to stop moving.
      await wd.pointerMove(last.box.cx, last.box.cy);
      await sleep(250);
      const hovered = await wd.exec(locateLive, target);
      if (!hovered || !hovered.visible || hovered.disabled || !hovered.inViewport || !hovered.hitsSelf
          || Math.abs(hovered.box.cx - last.box.cx) > 1 || Math.abs(hovered.box.cy - last.box.cy) > 1) {
        ctx.clicks.push({ t: iso(), label, method: 'reposition', before: last.box, after: hovered?.box });
        continue;
      }
      await wd.pointerClick(last.box.cx, last.box.cy);
      ctx.clicks.push({ t: iso(), label, method: 'pointer', x: Math.round(last.box.cx), y: Math.round(last.box.cy), hit: last.hit });
      return 'pointer';
    }
    await sleep(500);
  }
  for (let i = 0; i < 3; i++) {
    if (await wd.exec(jsClickLive, target).catch(() => false)) {
      ctx.clicks.push({ t: iso(), label, method: 'dom-click', reason: last });
      log(`click ${label}: pointer target not actionable, used DOM click (${JSON.stringify(last)})`);
      return 'dom-click';
    }
    await sleep(500);
  }
  throw new Error(`could not click ${label}: ${JSON.stringify(last)}`);
}

const clickBar = (ctx, label) => clickLive(ctx, { bar: barMatch(label) }, String(label));

async function waitForHandshake(ctx, { timeout = 30_000 } = {}) {
  const { wd } = ctx;
  await waitFor(() => wd.exec((sel) => window.__PHOENIX_UI_LIVE_INIT__ === true && !!liveQ(sel), GLOBAL_BAR_ID).catch(() => false),
    { timeout, interval: 200, label: 'live handshake' });
  await sleep(250);
  await wd.exec(installPageRecorder);
}

async function ensurePickActive(ctx) {
  const { wd } = ctx;
  const isActive = () => wd.exec((sel) => liveQ(sel)?.dataset.active === 'true', PICK_TOGGLE).catch(() => false);
  if (await isActive()) return;
  await clickLive(ctx, { selector: PICK_TOGGLE }, 'pick toggle');
  await waitFor(isActive, { timeout: 5_000, label: 'pick mode active' });
}

function barVisible() {
  const bar = liveQ('#impeccable-live-bar');
  if (!bar || bar.style.display === 'none') return false;
  const r = bar.getBoundingClientRect();
  return r.width > 0 && r.height > 0;
}

async function resetPickMode(ctx) {
  const { wd } = ctx;
  await wd.press(KEYS.ESCAPE).catch(() => {});
  await sleep(100);
  await wd.press(KEYS.ESCAPE).catch(() => {});
  await sleep(100);
  await wd.exec((sel) => {
    const btn = liveQ(sel);
    if (!btn) return;
    if (btn.dataset.active === 'true') btn.click();
    btn.click();
  }, PICK_TOGGLE).catch(() => {});
  await sleep(300);
}

async function pickElement(ctx, selector) {
  const { wd } = ctx;
  for (let attempt = 0; attempt < 3; attempt++) {
    await waitFor(() => wd.exec((s) => !!document.querySelector(s), selector).catch(() => false), { timeout: 5_000, label: selector });
    await ensurePickActive(ctx);
    await wd.exec(() => { const a = liveQ('#impeccable-live-annot'); if (a) a.style.display = 'none'; }).catch(() => {});
    const box = await wd.exec((s) => {
      const r = document.querySelector(s).getBoundingClientRect();
      return { cx: r.left + r.width / 2, cy: r.top + r.height / 2 };
    }, selector);
    await wd.pointerMove(box.cx, box.cy);
    await sleep(80);
    await wd.pointerClick(box.cx, box.cy);
    ctx.clicks.push({ t: iso(), label: `pick ${selector}`, method: 'pointer', x: Math.round(box.cx), y: Math.round(box.cy) });
    const visible = await waitFor(() => wd.exec(barVisible).catch(() => false), { timeout: 5_000, label: 'configure bar' }).then(() => true, () => false);
    if (visible) break;
    if (attempt === 2) {
      const snap = await wd.exec((s) => ({ state: window.__PHOENIX_UI_LIVE_STATE__ || null, pick: liveQ('#impeccable-live-pick-toggle')?.dataset.active || null,
        debug: window.__PHOENIX_UI_LIVE_CHROME_CORE__?.debugState?.() || null, target: !!document.querySelector(s) }), selector).catch((e) => ({ error: e.message }));
      throw new Error(`pick did not open configure bar for ${selector}: ${JSON.stringify(snap)}`);
    }
    await resetPickMode(ctx);
  }
  await waitFor(() => wd.exec((label) => !!findBarButton({ kind: 'aria', value: label }), GO_LABEL).catch(() => false),
    { timeout: 5_000, label: 'Go button in configure bar' });
}

function goAdvanced(goLabel) {
  const bar = liveQ('#impeccable-live-bar');
  if (!bar) return false;
  const text = bar.textContent || '';
  if (/Generating\b/.test(text)) return true;
  if (/\d+\s*\/\s*\d+/.test(text)) return true;
  return ![...bar.querySelectorAll('button')].some((b) => (b.getAttribute('aria-label') || '') === goLabel);
}

async function clickGo(ctx) {
  for (let attempt = 0; attempt < 3; attempt++) {
    await clickBar(ctx, { ariaLabel: GO_LABEL });
    const ok = await waitFor(() => ctx.wd.exec(goAdvanced, GO_LABEL).catch(() => false), { timeout: 3_000, label: 'Go advanced' })
      .then(() => true, () => false);
    if (ok) return;
    await sleep(500);
  }
  throw new Error('Go click did not leave configure mode');
}

function cyclingReady(expected) {
  const bar = liveQ('#impeccable-live-bar');
  if (!bar) return false;
  const m = (bar.textContent || '').match(/(\d+)\s*\/\s*(\d+)/);
  if (!m) return false;
  const wrapper = liveQ('[data-impeccable-variants]');
  const arrived = wrapper ? wrapper.querySelectorAll('[data-impeccable-variant]:not([data-impeccable-variant="original"])').length : 0;
  return parseInt(m[2], 10) === expected && arrived >= expected;
}

async function waitForCycling(ctx, expected, { timeout = 60_000 } = {}) {
  await waitFor(() => ctx.wd.exec(cyclingReady, expected).catch(() => false), { timeout, interval: 200, label: `cycling ${expected}` });
  const cleared = await waitFor(() => ctx.wd.exec(() => !liveQ('#impeccable-live-shader')).catch(() => false), { timeout: 5_000, label: 'shader cleared' })
    .then(() => true, () => false);
  if (!cleared) throw new Error('CYCLING reached but the generating shader overlay (#impeccable-live-shader) is still up');
}

function visibleVariant() {
  const wrapper = liveQ('[data-impeccable-variants]');
  if (wrapper) {
    const variants = [...wrapper.querySelectorAll('[data-impeccable-variant]:not([data-impeccable-variant="original"])')];
    const v = variants.find((x) => getComputedStyle(x).display !== 'none');
    const idx = v ? parseInt(v.dataset.impeccableVariant || '0', 10) : 0;
    if (idx > 0) return idx;
  }
  const bar = liveQ('#impeccable-live-bar');
  const m = bar ? (bar.textContent || '').match(/(\d+)\s*\/\s*(\d+)/) : null;
  return m ? parseInt(m[1], 10) : null;
}

function variantSettled(expected, count) {
  const bar = liveQ('#impeccable-live-bar');
  if (!bar || !(bar.textContent || '').includes(`${expected}/${count}`)) return false;
  const d = window.__PHOENIX_UI_LIVE_CHROME_CORE__?.debugState?.();
  return !d || d.visibleVariant === expected;
}

async function cycleToVariant(ctx, target, count) {
  let visible = await ctx.wd.exec(visibleVariant).catch(() => null);
  let steps = 0;
  while (visible !== target) {
    if (steps++ > count + 6) throw new Error(`variant ${target} did not become visible; last=${visible}`);
    const forward = visible == null || visible < target;
    const next = visible == null ? 1 : (forward ? visible + 1 : visible - 1);
    await clickBar(ctx, forward ? '→' : '←');
    await waitFor(() => ctx.wd.exec(variantSettled, next, count).catch(() => false), { timeout: 15_000, label: `variant ${next}/${count} settled` });
    visible = await ctx.wd.exec(visibleVariant).catch(() => null);
  }
  return visible;
}

async function clickAccept(ctx, { expectedVariant }) {
  for (let i = 0; i < 5; i++) {
    const cur = await ctx.wd.exec(visibleVariant).catch(() => null);
    if (cur === expectedVariant) break;
    if (cur == null) { await sleep(300); continue; }
    await clickBar(ctx, cur < expectedVariant ? '→' : '←');
    await sleep(300);
  }
  const cur = await ctx.wd.exec(visibleVariant).catch(() => null);
  if (cur !== expectedVariant) throw new Error(`expected visible variant ${expectedVariant} before accept, got ${cur}`);
  return clickBar(ctx, /Accept/);
}

const clickDiscard = (ctx) => clickBar(ctx, '✕');

async function waitForBarHidden(ctx, { timeout = 15_000 } = {}) {
  await waitFor(() => ctx.wd.exec(() => { const b = liveQ('#impeccable-live-bar'); return !b || b.style.display === 'none'; }).catch(() => false),
    { timeout, interval: 150, label: 'bar hidden' });
}

function liveDebugInPage() {
  const d = window.__PHOENIX_UI_LIVE_CHROME_CORE__?.debugState?.();
  return d ? { state: d.state, sessionId: d.currentSessionId, expected: d.expectedVariants, arrived: d.arrivedVariants,
    visible: d.visibleVariant, barText: d.barText, barConnected: d.barConnected } : null;
}

function domWrappersInPage() {
  return { wrappers: document.querySelectorAll('[data-impeccable-variants]').length, h1: document.querySelector('h1.hero-title')?.textContent ?? null };
}

function toastInPage() {
  const t = liveQ('#impeccable-live-toast');
  if (!t) return null;
  const r = t.getBoundingClientRect();
  return { text: t.textContent || '', visible: t.style.display !== 'none' && r.width > 0 && r.height > 0 && getComputedStyle(t).visibility !== 'hidden' };
}

// ---------------------------------------------------------------------------
// Lazy harness (it arms the live-server reaper on import)
// ---------------------------------------------------------------------------

let H = null;
let S = null;
let A = null;
async function harness() {
  if (!H) {
    H = await import('./phoenix-harness.mjs');
    S = await import('./session.mjs');
    A = await import('./agent.mjs');
  }
  return H;
}

// ---------------------------------------------------------------------------
// Row bookkeeping
// ---------------------------------------------------------------------------

function newRow(key, wd, meta) {
  return {
    data: { row: key, dir: relative(CHECKOUT_ROOT, join(OUT, ROW_DIRS[key])), status: 'running', startedAt: iso(), meta, cases: [], limitations: [] },
    ctx: { wd, clicks: [], dir: ROW_DIRS[key], key },
  };
}

async function runCase(row, name, fn) {
  const entry = { case: name, startedAt: iso() };
  row.data.cases.push(entry);
  try {
    const checks = await fn(entry);
    entry.checks = checks;
    entry.pass = allTrue(checks);
    entry.failedChecks = Object.entries(checks).filter(([, v]) => v !== true).map(([k]) => k);
  } catch (err) {
    entry.pass = false;
    entry.error = err.stack || String(err);
    try { await row.ctx.wd.screenshot(outPath(row.ctx.dir, `error-${slug(name)}.png`)); } catch { /* no page */ }
  }
  entry.finishedAt = iso();
  log(`${entry.pass ? 'PASS' : 'FAIL'} ${row.ctx.key}: ${name}${entry.failedChecks?.length ? ` failed=${entry.failedChecks.join(',')}` : ''}${entry.error ? ` error=${entry.error.split('\n')[0]}` : ''}`);
  return entry;
}

const shot = (row, name) => row.ctx.wd.screenshot(outPath(row.ctx.dir, `${name}.png`)).catch((e) => log(`screenshot ${name} failed: ${e.message}`));

function isOurTmp(path) {
  return typeof path === 'string' && path.startsWith(tmpdir()) && basename(path).startsWith('phoenix-e2e-');
}

function removeTmp(path) {
  if (isOurTmp(path) && existsSync(path)) rmSync(path, { recursive: true, force: true });
}

/** The helper pid really is this engine's live server before it gets SIGKILL. */
function isOurHelper(pid) {
  try {
    const cmd = execFileSync('ps', ['-o', 'ppid=,command=', '-p', String(pid)], { encoding: 'utf-8' }).trim();
    return cmd.includes('live-server') || cmd.includes(H.ENGINE_BIN);
  } catch { return false; }
}

// ---------------------------------------------------------------------------
// Row 2: disconnect
// ---------------------------------------------------------------------------

const NOTICE_TIMEOUT_MS = 150_000;

async function row2(row) {
  const { wd } = row.ctx;
  const rec = H.createRecorder('safari-disconnect');
  const pageLog = newPageLog();
  const logs = [];
  row.data.limitations.push('No CDP in Safari: SSE frames and browser->helper requests are not recorded; page console lines are recorded from page load on via a console wrapper.');
  let session;
  try {
    session = await H.bootPhoenixSession({ browser: null, rec, log: (m) => logs.push({ t: rec.t(), m }) });
    row.data.meta.tmp = session.tmp;
    await wd.navigate(session.url);
    await waitForHandshake(row.ctx);
    await shot(row, 'disconnect-0-connected');

    const waitForNotice = async () => waitFor(async () => {
      const t = await wd.exec(toastInPage).catch(() => null);
      return t && /live server stopped/i.test(t.text) ? t : null;
    }, { label: 'disconnect notice', timeout: NOTICE_TIMEOUT_MS, interval: 250 }).catch(() => null);

    const restartAndReconnect = async () => {
      const restarted = H.startHelper(session.tmp);
      rec.trackLive(restarted);
      session.live = restarted;
      H.injectScript(session.tmp, restarted);
      await drain(wd, pageLog, rec);
      await wd.refresh();
      const ok = await waitForHandshake(row.ctx, { timeout: 30_000 }).then(() => true, () => false);
      return { restarted, ok };
    };

    await runCase(row, 'stop --keep-inject', async (e) => {
      const before = session.live;
      const stop = H.engine('live-server', ['stop', '--keep-inject'], { cwd: session.tmp });
      const started = Date.now();
      await waitFor(() => !H.pidAlive(before.pid), { label: 'helper exit', timeout: 15_000 });
      const notice = await waitForNotice();
      const noticeAfterMs = Date.now() - started;
      await shot(row, 'disconnect-keep-inject');
      await drain(wd, pageLog, rec);
      const { restarted, ok } = await restartAndReconnect();
      Object.assign(e, { stop: { code: stop.code, stdout: stop.stdout }, noticeAfterMs, notice,
        before: H.processRecord('stopped helper', before), restarted: H.processRecord('restarted helper', restarted) });
      return {
        stopExitedZero: stop.code === 0,
        noticeShown: notice !== null,
        noticeVisible: notice?.visible === true,
        noticeNamesRestartCommand: /\/phoenix-ui live/.test(notice?.text || ''),
        noticeSaysReload: /reload this page/.test(notice?.text || ''),
        reconnectedAfterRestart: ok,
      };
    });

    await runCase(row, 'SIGKILL', async (e) => {
      const before = session.live;
      if (!isOurHelper(before.pid)) throw new Error(`pid ${before.pid} is not this run's live server; not killing`);
      process.kill(before.pid, 'SIGKILL');
      const started = Date.now();
      await waitFor(() => !H.pidAlive(before.pid), { label: 'helper killed', timeout: 15_000 });
      const notice = await waitForNotice();
      const noticeAfterMs = Date.now() - started;
      await shot(row, 'disconnect-killed');
      await drain(wd, pageLog, rec);
      const status = H.engine('live-status', [], { cwd: session.tmp });
      const { restarted, ok } = await restartAndReconnect();
      Object.assign(e, { noticeAfterMs, notice, statusAfterKill: { code: status.code, json: status.json, stdout: status.stdout },
        restart: restarted.result.json, restarted: H.processRecord('restarted helper', restarted) });
      return {
        noticeShown: notice !== null,
        noticeVisible: notice?.visible === true,
        noticeNamesRestartCommand: /\/phoenix-ui live/.test(notice?.text || ''),
        reconnectedAfterRestart: ok,
      };
    });

    await runCase(row, 'stop', async (e) => {
      const before = session.live;
      const stop = H.engine('live-server', ['stop'], { cwd: session.tmp });
      await waitFor(() => !H.pidAlive(before.pid), { label: 'helper exit', timeout: 15_000 });
      const state = H.sourceState(session.tmp);
      Object.assign(e, { stop: { code: stop.code, stdout: stop.stdout }, sourceAfterStop: state });
      return { stopExitedZero: stop.code === 0, noLiveScriptInSource: !/live\.js/.test(state.diff) };
    });
  } finally {
    await drain(wd, pageLog, rec);
    await wd.navigate('about:blank').catch(() => {});
    if (session) {
      await session.teardown();
      row.data.meta.sourceAfterTeardown = H.sourceState(session.tmp);
      session.remove();
    }
    writeJson(outPath(row.ctx.dir, 'events.json'), { agentEvents: rec.agentEvents, notes: rec.notes, page: pageLog, agentLog: logs, clicks: row.ctx.clicks });
  }
}

// ---------------------------------------------------------------------------
// Row 3: accept / discard / move workspace
// ---------------------------------------------------------------------------

const FIXTURE = 'vite8-react-plain';
const APP = 'src/App.jsx';
const INDEX = 'index.html';
const ORIGINAL_H1 = '<h1 className="hero-title">Vite 8 Fixture</h1>';
const CSP = "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self' ws:";
const RESIDUE = /data-impeccable|impeccable-variants|impeccable-carbonize|@scope|data-impeccable-css/;

const headBytes = (cwd, file) => execFileSync('git', ['show', `HEAD:${file}`], { cwd });
const sameAsHead = (cwd, file) => Buffer.compare(readFileSync(join(cwd, file)), headBytes(cwd, file)) === 0;
const readText = (cwd, file) => readFileSync(join(cwd, file), 'utf-8');

function movedPath(path, tag) {
  return join(tmpdir(), `phoenix-e2e-safari-moved-${tag}-${basename(path).slice('phoenix-e2e-safari-'.length)}`);
}

function listFiles(root, sub) {
  const dir = join(root, sub);
  if (!existsSync(dir)) return [];
  const out = [];
  const walk = (d) => {
    for (const name of readdirSync(d)) {
      const p = join(d, name);
      if (statSync(p).isDirectory()) walk(p);
      else out.push(relative(root, p));
    }
  };
  walk(dir);
  return out.sort();
}

function receipts(cwd) {
  return listFiles(cwd, '.phoenix-ui/live/accept-receipts').map((file) => {
    try { return { file, ...JSON.parse(readText(cwd, file)) }; } catch { return { file, unreadable: true }; }
  });
}

/** The deterministic agent; variant N says " vN" in its text. */
function markedAgent(rec, control) {
  const base = A.createFakeAgent();
  const mark = (html, n) => html.replace(/>([^<]*)</, (_m, text) => `>${text.replace(/ v\d+$/, '')} v${n}<`);
  return {
    ...base,
    async generateVariants(event, context) {
      control.generateCalls.push({ t: rec.t(), id: event.id, count: event.count });
      rec.agentEvents.push({ t: rec.t(), event: 'agent.generate.payload', id: event.id, count: event.count });
      const out = await base.generateVariants(event, context);
      return { ...out, variants: out.variants.map((v, i) => ({ ...v, innerHtml: mark(v.innerHtml, i + 1) })) };
    },
  };
}

async function row3(row) {
  const { wd } = row.ctx;
  const ctx = row.ctx;
  const rec = H.createRecorder('safari-row3');
  const logs = [];
  const control = { generateCalls: [] };
  const pageLog = newPageLog();
  const paths = [];
  const steps = [];
  let s = null;
  let fixture;
  let acceptedApp = null;
  row.data.limitations.push('No CDP in Safari: accept/discard/generate POSTs and CSP violations come from an in-page fetch wrapper and securitypolicyviolation listener installed right after the handshake (load-time violations before that are not seen; a blocked live.js would fail the handshake instead).');

  const postsOf = async (type) => { if (s?.page) await drain(wd, pageLog, rec); return pageLog.posts.filter((p) => p.path === '/events' && p.type === type); };

  const bootAt = async (tmp) => {
    const b = { tmp, live: null, dev: null, agent: null, page: false };
    b.live = H.startHelper(tmp);
    rec.trackLive(b.live);
    b.inject = H.injectScript(tmp, b.live);
    b.dev = S.startDevServer(tmp, fixture.runtime);
    try {
      const { port } = await b.dev.ready;
      b.url = `http://127.0.0.1:${port}/`;
    } catch (err) {
      await shutdown(b, { helper: 'stop' });
      throw err;
    }
    b.agent = H.startAgent({ cwd: tmp, live: b.live, rec, agent: markedAgent(rec, control), log: (m) => logs.push({ t: rec.t(), m }) });
    await wd.navigate(b.url);
    b.page = true;
    await waitForHandshake(ctx);
    return b;
  };

  const shutdown = async (b, { helper = 'stop' } = {}) => {
    const out = {};
    if (b.page) {
      await drain(wd, pageLog, rec);
      await wd.navigate('about:blank').catch(() => {});
      b.page = false;
    }
    try { if (b.agent) await b.agent.stop(); } catch { /* stopped */ }
    b.agent = null;
    try { if (b.dev?.child) await S.stopDevServer(b.dev.child); } catch { /* gone */ }
    if (b.live?.pid && H.pidAlive(b.live.pid)) {
      if (helper === 'stop') {
        const res = H.engine('live-server', ['stop'], { cwd: b.tmp });
        out.stop = { code: res.code, stdout: res.stdout, stderr: res.stderr };
        await waitFor(() => !H.pidAlive(b.live.pid), { label: 'helper exit', timeout: 15_000 }).catch(() => {});
      } else if (helper === 'kill') {
        if (!isOurHelper(b.live.pid)) throw new Error(`pid ${b.live.pid} is not this run's live server; not killing`);
        process.kill(b.live.pid, 'SIGKILL');
        await waitFor(() => !H.pidAlive(b.live.pid), { label: 'helper killed', timeout: 15_000 }).catch(() => {});
        out.killed = true;
      }
      if (helper !== 'none' && H.pidAlive(b.live.pid) && isOurHelper(b.live.pid)) {
        try { process.kill(b.live.pid, 'SIGKILL'); } catch { /* gone */ }
      }
    }
    return out;
  };

  try {
    const staged = H.stagePhoenixFixture(FIXTURE, { prefix: 'phoenix-e2e-safari-' });
    fixture = staged.fixture;
    const file = join(staged.tmp, INDEX);
    writeFileSync(file, readFileSync(file, 'utf-8').replace('<meta charset="UTF-8" />',
      `<meta charset="UTF-8" />\n    <meta http-equiv="Content-Security-Policy" content="${CSP}" />`));
    H.git(staged.tmp, ['commit', '-qam', 'csp meta']);
    paths.push(staged.tmp);
    s = await bootAt(staged.tmp);
    steps.push({ step: 'boot', tmp: s.tmp, inject: s.inject.json, index: readText(s.tmp, INDEX), helper: H.processRecord('helper', s.live) });

    await runCase(row, 'discard restores the original source byte-for-byte and leaves no wrapper', async (e) => {
      const { tmp } = s;
      const before = H.sourceState(tmp);
      await pickElement(ctx, 'h1.hero-title');
      await clickGo(ctx);
      await waitForCycling(ctx, 3, { timeout: 60_000 });
      await cycleToVariant(ctx, 2, 3);
      const during = { source: readText(tmp, APP), debug: await wd.exec(liveDebugInPage) };
      await shot(row, 'discard-1-cycling-v2');
      await clickDiscard(ctx);
      await waitForBarHidden(ctx);
      const restored = await waitFor(() => sameAsHead(tmp, APP), { label: 'App.jsx back to HEAD', timeout: 20_000 }).catch(() => false);
      const dom = await waitFor(async () => { const d = await wd.exec(domWrappersInPage); return d.wrappers === 0 ? d : null; },
        { label: 'no wrapper in DOM', timeout: 10_000 }).catch(() => wd.exec(domWrappersInPage));
      await shot(row, 'discard-2-after');
      const sessionId = during.debug?.sessionId;
      const receipt = receipts(tmp).find((r) => r.id === sessionId) || null;
      const accepts = await postsOf('accept');
      Object.assign(e, { sessionId, before, after: H.sourceState(tmp), dom, receipt, debugDuring: during.debug });
      return {
        cycledThreeVariants: [1, 2, 3].every((n) => during.source.includes(`Vite 8 Fixture v${n}`)),
        appByteIdenticalToHead: restored === true,
        noResidueInApp: !RESIDUE.test(readText(tmp, APP)),
        noWrapperInDom: dom.wrappers === 0,
        originalTextInDom: dom.h1 === 'Vite 8 Fixture',
        discardReceipt: receipt?.operation === 'discard',
        noAcceptPosted: accepts.length === 0,
      };
    });

    await runCase(row, 'accept leaves only the chosen variant and no variant, carbonize or scope residue', async (e) => {
      const { tmp } = s;
      await pickElement(ctx, 'h1.hero-title');
      await clickGo(ctx);
      await waitForCycling(ctx, 3, { timeout: 60_000 });
      await cycleToVariant(ctx, 2, 3);
      const debug = await wd.exec(liveDebugInPage);
      await shot(row, 'accept-1-cycling-v2');
      await clickAccept(ctx, { expectedVariant: 2 });
      await waitForBarHidden(ctx, { timeout: 20_000 });
      const sessionId = debug?.sessionId;
      const settled = await waitFor(() => {
        const src = readText(tmp, APP);
        return !RESIDUE.test(src) && src.includes('Vite 8 Fixture v2') ? src : null;
      }, { label: 'accepted source settled', timeout: 30_000 }).catch(() => null);
      const completed = await waitFor(() => logs.some((l) => l.m === `completed carbonize session ${sessionId}`),
        { label: 'live-complete', timeout: 30_000 }).catch(() => false);
      const dom = await waitFor(async () => { const d = await wd.exec(domWrappersInPage); return d.wrappers === 0 && d.h1 === 'Vite 8 Fixture v2' ? d : null; },
        { label: 'accepted DOM', timeout: 15_000 }).catch(() => wd.exec(domWrappersInPage));
      await shot(row, 'accept-2-after');
      const app = readText(tmp, APP);
      const head = headBytes(tmp, APP).toString('utf-8');
      const chosenLine = '<h1 className="hero-title">Vite 8 Fixture v2</h1>';
      const receipt = receipts(tmp).find((r) => r.id === sessionId) || null;
      const status = H.engine('live-status', [], { cwd: tmp });
      const accepts = await postsOf('accept');
      acceptedApp = readFileSync(join(tmp, APP));
      Object.assign(e, { sessionId, after: H.sourceState(tmp), dom, receipt, liveStatus: status.json ?? status.stdout, acceptPosts: accepts });
      return {
        sourceSettled: settled !== null,
        liveCompleteRan: completed === true,
        chosenVariantPresent: app.includes(chosenLine),
        otherVariantsAbsent: !/Vite 8 Fixture v[13]\b/.test(app),
        noResidueInApp: !RESIDUE.test(app),
        onlyTheH1Changed: app.replace(chosenLine, ORIGINAL_H1) === head,
        noWrapperInDom: dom.wrappers === 0,
        chosenTextInDom: dom.h1 === 'Vite 8 Fixture v2',
        acceptReceipt: receipt?.operation === 'accept' && String(receipt?.variantId) === '2',
        oneAcceptPosted: accepts.length === 1,
      };
    });

    await runCase(row, 'live-server stop leaves no injected script or CSP residue', async (e) => {
      const { tmp } = s;
      const helper = s.live;
      const injected = readText(tmp, INDEX);
      const out = await shutdown(s, { helper: 'stop' });
      const index = readText(tmp, INDEX);
      const status = H.git(tmp, ['status', '--porcelain', '--untracked-files=all']);
      const tracked = status.split('\n').filter((l) => l && !l.startsWith('??'));
      const untracked = status.split('\n').filter((l) => l.startsWith('?? ')).map((l) => l.slice(3));
      const cspViolations = pageLog.csp.concat(pageLog.logs.filter((c) => /Content Security Policy|Refused to/i.test(c.text)));
      Object.assign(e, { stop: out.stop, injectedIndex: injected, indexAfterStop: index, gitStatus: status, cspViolations,
        gitInfoExcludeAfterStop: readText(tmp, '.git/info/exclude'), helper: H.processRecord('stopped helper', helper) });
      return {
        stopExitedZero: out.stop?.code === 0,
        helperGone: !H.pidAlive(helper.pid),
        injectHadPatchedCsp: injected.includes('data-impeccable-csp-original') && injected.includes(`http://localhost:${helper.port}`),
        indexByteIdenticalToHead: sameAsHead(tmp, INDEX),
        noCspMarker: !index.includes('data-impeccable-csp-original'),
        noLiveScript: !/live\.js|impeccable-live|phoenix-ui-live/i.test(index),
        onlyAppTracked: tracked.length === 1 && tracked[0] === ` M ${APP}`,
        untrackedOnlyUnderPhoenixDir: untracked.every((f) => f.startsWith('.phoenix-ui/')),
        noCspViolations: cspViolations.length === 0,
      };
    });

    await runCase(row, 'after the project moves, live runs from the new path and never writes the old one', async (e) => {
      const oldPath = s.tmp;
      const moved = movedPath(oldPath, 'a');
      renameSync(oldPath, moved);
      paths.push(moved);
      s = await bootAt(moved);
      await pickElement(ctx, 'h1.hero-title');
      await clickGo(ctx);
      await waitForCycling(ctx, 3, { timeout: 60_000 });
      await cycleToVariant(ctx, 3, 3);
      await shot(row, 'move-1-cycling');
      await clickDiscard(ctx);
      await waitForBarHidden(ctx);
      const restored = await waitFor(() => Buffer.compare(readFileSync(join(moved, APP)), acceptedApp) === 0,
        { label: 'App.jsx back to the accepted bytes', timeout: 20_000 }).catch(() => false);
      const status = H.engine('live-status', [], { cwd: moved });
      const serverJson = existsSync(join(moved, '.phoenix-ui/live/server.json')) ? readText(moved, '.phoenix-ui/live/server.json') : null;
      const helper = s.live;
      const out = await shutdown(s, { helper: 'kill' });
      const indexAfterKill = readText(moved, INDEX);
      Object.assign(e, { oldPath, moved, liveStatus: status.json ?? status.stdout, serverJson, helper: H.processRecord('killed helper', helper),
        out, indexAfterKill, phoenixFiles: listFiles(moved, '.phoenix-ui') });
      return {
        oldPathAbsent: !existsSync(oldPath),
        discardRestoredAcceptedBytes: restored === true,
        noResidueInApp: !RESIDUE.test(readText(moved, APP)),
        statusNamesNoOldPath: !String(status.stdout).includes(oldPath),
        killLeftInjection: /data-impeccable-csp-original/.test(indexAfterKill),
      };
    });

    await runCase(row, 'after an abnormal exit and another move, live recovers at the new path and stops clean', async (e) => {
      const [first, second] = paths;
      const moved2 = movedPath(first, 'b');
      renameSync(second, moved2);
      paths.push(moved2);
      const statusBefore = H.engine('live-status', [], { cwd: moved2 });
      const journal = existsSync(join(moved2, '.phoenix-ui/live/inject-journal.json')) ? readText(moved2, '.phoenix-ui/live/inject-journal.json') : null;
      s = await bootAt(moved2);
      const start = s.live.result.json;
      const inject = s.inject.json;
      await shot(row, 'move-2-recovered');
      const helper = s.live;
      const out = await shutdown(s, { helper: 'stop' });
      const index = readText(moved2, INDEX);
      const phoenixFiles = listFiles(moved2, '.phoenix-ui');
      const oldForms = [first, second].flatMap((p) => [p, `/private${p}`]);
      const mentionsOld = phoenixFiles.filter((f) => {
        try { const t = readText(moved2, f); return oldForms.some((p) => t.includes(p)); } catch { return false; }
      });
      Object.assign(e, { first, second, moved2, statusBefore: statusBefore.json ?? statusBefore.stdout, journalBefore: journal, start, inject,
        stop: out.stop, helper: H.processRecord('stopped helper', helper), indexAfterStop: index,
        gitStatus: H.git(moved2, ['status', '--porcelain', '--untracked-files=all']), phoenixFiles, filesMentioningOldPaths: mentionsOld });
      return {
        firstPathAbsent: !existsSync(first),
        secondPathAbsent: !existsSync(second),
        stopExitedZero: out.stop?.code === 0,
        indexByteIdenticalToHead: sameAsHead(moved2, INDEX),
        appKeepsAcceptedBytes: acceptedApp !== null && Buffer.compare(readFileSync(join(moved2, APP)), acceptedApp) === 0,
      };
    });
  } finally {
    if (s) {
      const out = await shutdown(s, { helper: 'stop' }).catch((err) => ({ error: err.message }));
      steps.push({ step: 'final teardown', ...out, source: existsSync(s.tmp) ? H.sourceState(s.tmp) : null });
    }
    writeJson(outPath(row.ctx.dir, 'events.json'), { paths, steps, agentEvents: rec.agentEvents, page: pageLog, agentLog: logs,
      generateCalls: control.generateCalls, clicks: row.ctx.clicks });
    for (const p of paths) removeTmp(p);
  }
}

// ---------------------------------------------------------------------------
// Row 5: question cards without images, keyboard, narrow
// ---------------------------------------------------------------------------

const CATALOG = join(CHECKOUT_ROOT, 'skills', 'phoenix-ui', 'src', 'scripts', 'data', 'catalog', 'concept-ingredients.json');
const DESKTOP = { width: 1440, height: 900 };
const NARROW = { width: 375, height: 812 };
const DEAL_SETTLE_MS = 2_500;
const PALETTES = [
  ['#eef1ee', '#1d2421', '#2357c6'],
  ['#fbf7ef', '#2a2620', '#b5832a'],
  ['#f2f4f7', '#101828', '#0f766e'],
  ['#f7f3f0', '#2b2118', '#9a3412'],
];

function catalogCards() {
  const catalog = JSON.parse(readFileSync(CATALOG, 'utf-8'));
  return catalog.families.slice(0, 4).map((family, i) => {
    const c = family.concepts[0];
    return {
      id: c.id,
      label: c.source.title.replace(/^Phoenix original concept:\s*/, ''),
      lineage: family.label,
      thesis: c.thesis,
      palette: PALETTES[i],
      materials: c.tags,
      viewport: c.form,
      risk: `Reads generic when ${c.tags[0].replace(/-/g, ' ')} is not backed by real data.`,
    };
  });
}

function textOnlyRound() {
  const [a, b, c, d] = catalogCards();
  return {
    title: 'Choose the visual world',
    question: 'This round has no card art: compare the directions by their facts.',
    options: [
      { ...a, kicker: 'THE ROLL' },
      { ...b, kicker: 'PHOENIX’S PICK' },
      { ...c, verdict: 'competitive', case: 'Fuses cleanly with the brief; loses narrowly on clarity.' },
      { ...d, verdict: 'declined', case: 'Fuses poorly with the audience.', kept: 'Its evidence-first ordering.' },
    ],
    reroll: { registers: ['safer', 'bolder'] },
    canon: true,
    steer: true,
  };
}

function brokenImageRound(closedPort) {
  const [a, b, c, d] = catalogCards();
  const broken = (name) => `http://127.0.0.1:${closedPort}/${name}`;
  return {
    title: 'Choose the visual world',
    question: 'Card art is missing or failing on purpose.',
    options: [
      { ...a, kicker: 'THE ROLL', case: 'Leads on evidence density.', comp: '.phoenix-ui/mocks/decision/never-lands.webp' },
      { ...b, kicker: 'PHOENIX’S PICK', hero: broken('hero-b.webp') },
      { ...c, verdict: 'competitive', case: 'Fuses cleanly; loses narrowly on clarity.', board: '.phoenix-ui/catalog-art/missing-board.webp' },
      { ...d, verdict: 'declined', case: 'Fuses poorly with the audience.', kept: 'Its evidence-first ordering.', hero: broken('hero-d.webp') },
    ],
    reroll: { registers: ['safer', 'bolder'] },
    steer: true,
  };
}

/** Every card as the user sees it (same probe as the Chrome test). */
function readCards() {
  return [...document.querySelectorAll('.grid > .card')].map((card) => {
    const front = card.querySelector('.face.front');
    const choose = front?.querySelector('button.choose');
    const r = choose?.getBoundingClientRect();
    const cr = card.getBoundingClientRect();
    return {
      id: card.dataset.id,
      declined: card.classList.contains('declined'),
      textOnly: !!front?.classList.contains('text-only'),
      hasBack: !!card.querySelector('.face.back'),
      label: front?.querySelector('h2')?.textContent ?? null,
      thesis: front?.querySelector('.thesis')?.textContent ?? null,
      facts: [...(front?.querySelectorAll('.fact .fact-label') || [])].map((f) => f.textContent),
      swatches: front?.querySelectorAll('.swatches i').length ?? 0,
      media: [...(front?.querySelectorAll('.media') || [])].map((m) => ({
        cls: m.className,
        img: !!m.querySelector(':scope > img:not([hidden])'),
        label: m.querySelector('.media-label')?.textContent ?? null,
        note: m.querySelector('.comp-note')?.textContent ?? null,
      })),
      inspo: front?.querySelectorAll('.inspo, .pip').length ?? 0,
      brokenImgs: [...card.querySelectorAll('img')].filter((img) => img.getAttribute('src') && img.complete && img.naturalWidth === 0 && !img.hidden).length,
      choose: choose ? { text: choose.textContent, disabled: choose.disabled, w: r.width, h: r.height } : null,
      cardBox: { x: Math.round(cr.left), y: Math.round(cr.top), w: Math.round(cr.width), h: Math.round(cr.height) },
    };
  });
}

/** The focused element: identity, focus ring (outline and box-shadow), and whether anything hides it. */
function describeFocus() {
  const el = document.activeElement;
  if (!el || el === document.body || el === document.documentElement) return { tag: 'BODY' };
  const r = el.getBoundingClientRect();
  const cs = getComputedStyle(el);
  const inset = 6;
  const pts = [[r.left + r.width / 2, r.top + r.height / 2], [r.left + inset, r.top + inset], [r.right - inset, r.top + inset],
    [r.left + inset, r.bottom - inset], [r.right - inset, r.bottom - inset]];
  const centreHit = document.elementFromPoint(pts[0][0], pts[0][1]);
  const coveredByCentre = !!centreHit && centreHit !== el && !el.contains(centreHit);
  const coveredBy = pts.map(([x, y]) => {
    const hit = document.elementFromPoint(x, y);
    if (!hit || hit === el || el.contains(hit)) return null;
    return `${hit.tagName.toLowerCase()}${hit.id ? '#' + hit.id : ''}${hit.className && typeof hit.className === 'string' ? '.' + hit.className.trim().split(/\s+/).join('.') : ''}`;
  }).filter(Boolean);
  const cx = r.left + r.width / 2;
  const cy = r.top + r.height / 2;
  const underFade = [...document.querySelectorAll('.fade')].some((f) => {
    const fr = f.getBoundingClientRect();
    return parseFloat(getComputedStyle(f).opacity) > 0.5 && cx >= fr.left && cx <= fr.right && cy >= fr.top && cy <= fr.bottom;
  });
  const card = el.closest('.card');
  const face = el.closest('.face');
  const flipped = !!card?.classList.contains('flipped');
  const faceName = face ? (face.classList.contains('back') ? 'back' : 'front') : null;
  return {
    tag: el.tagName,
    cls: typeof el.className === 'string' ? el.className : null,
    id: el.id || null,
    dataId: el.dataset?.id || null,
    cardId: card?.dataset.id || null,
    face: faceName,
    flipped,
    onHiddenFace: (faceName === 'back' && !flipped) || (faceName === 'front' && flipped),
    text: (el.textContent || el.getAttribute('aria-label') || el.placeholder || '').trim().slice(0, 40),
    focusVisible: el.matches(':focus-visible'),
    outline: { style: cs.outlineStyle, width: cs.outlineWidth, color: cs.outlineColor, offset: cs.outlineOffset },
    boxShadow: cs.boxShadow,
    rect: { x: Math.round(r.left), y: Math.round(r.top), w: Math.round(r.width), h: Math.round(r.height) },
    inViewport: r.left >= 0 && r.top >= 0 && r.right <= innerWidth && r.bottom <= innerHeight,
    coveredBy,
    coveredByCentre,
    underFade,
  };
}

const outlineRing = (s) => s.outline && s.outline.style !== 'none' && parseFloat(s.outline.width) >= 2;
const shadowRing = (s) => !!s.boxShadow && s.boxShadow !== 'none';
const focusRingOk = (s) => s.focusVisible && (outlineRing(s) || shadowRing(s));
const chooseStops = (stops) => stops.filter((s) => s.tag === 'BUTTON' && s.cls === 'choose' && !s.onHiddenFace);
const focusNotObscured = (s) => s.inViewport && !(s.coveredBy.length > 0 && s.coveredByCentre) && !s.underFade;

function keyboardChecks(cards, stops) {
  const chooses = chooseStops(stops);
  const reached = new Set(chooses.map((s) => s.dataId));
  const bad = chooses.filter((s) => !focusRingOk(s) || !focusNotObscured(s));
  return {
    checks: {
      everyCardChoosableByKeyboard: cards.length > 0 && cards.every((c) => reached.has(c.id)),
      everyChooseFocusRingVisible: chooses.length > 0 && chooses.every(focusRingOk),
      everyFocusedChooseOnScreen: chooses.length > 0 && bad.length === 0,
      everyStopHasFocusRing: stops.length > 0 && stops.every((s) => s.focusVisible && (outlineRing(s) || shadowRing(s))),
      noFocusOnHiddenCardFace: stops.every((s) => !s.onHiddenFace),
    },
    bad,
    partiallyCovered: chooses.filter((s) => s.coveredBy.length > 0).map((s) => ({ dataId: s.dataId, coveredBy: s.coveredBy })),
    chooseRings: chooses.map((s) => ({ dataId: s.dataId, outline: s.outline, boxShadow: s.boxShadow, focusVisible: s.focusVisible })),
  };
}

const KEY_MODES = { tab: [], 'option-tab': [KEYS.ALT] };

async function settledFocus(wd) {
  await sleep(250);
  let last = await wd.exec(describeFocus);
  for (let i = 0; i < 15; i++) {
    await sleep(200);
    const cur = await wd.exec(describeFocus);
    if (JSON.stringify(cur.rect) === JSON.stringify(last.rect)) return cur;
    last = cur;
  }
  return last;
}

async function tabWalk(row, { label, mode = 'tab', shots = false, max = 40 } = {}) {
  const { wd } = row.ctx;
  const stops = [];
  let bodyInARow = 0;
  for (let i = 0; i < max; i++) {
    await wd.press(KEYS.TAB, { modifiers: KEY_MODES[mode] });
    const stop = await settledFocus(wd);
    if (stop.tag === 'BODY') {
      if (stops.length) break;
      if (++bodyInARow >= 4) break; // keyboard never entered the page
      continue;
    }
    stops.push(stop);
    if (shots && (stop.cls === 'choose' || stop.onHiddenFace)) {
      const name = `${label}-focus-${String(stops.length).padStart(2, '0')}-${stop.cardId || stop.id || 'page'}-${stop.face || 'page'}-${(stop.cls || stop.tag).replace(/\s+/g, '.')}`;
      await shot(row, name);
    }
  }
  return stops;
}

async function tabTo(row, predicate, mode, max = 40) {
  for (let i = 0; i < max; i++) {
    await row.ctx.wd.press(KEYS.TAB, { modifiers: KEY_MODES[mode] });
    const stop = await settledFocus(row.ctx.wd);
    if (predicate(stop)) return stop;
  }
  return null;
}

/**
 * Safari's own keyboard model: with the macOS/Safari defaults Tab skips
 * buttons and links, and Option-Tab reaches them. A plain HTML page shows
 * which keys reach a <button> in this browser, so the product is judged
 * against the same keys.
 */
async function keyboardControl(row) {
  const { wd } = row.ctx;
  const html = '<!doctype html><title>tab control</title><input id="field" aria-label="field"> <button id="b1">one</button> <button id="b2">two</button> <a id="link" href="#x">link</a>';
  const out = {};
  for (const mode of Object.keys(KEY_MODES)) {
    await wd.navigate(`data:text/html;charset=utf-8,${encodeURIComponent(html)}`);
    await sleep(300);
    const seen = [];
    for (let i = 0; i < 8; i++) {
      await wd.press(KEYS.TAB, { modifiers: KEY_MODES[mode] });
      await sleep(150);
      const id = await wd.exec(() => (document.activeElement && document.activeElement !== document.body ? document.activeElement.id : 'BODY'));
      seen.push(id);
      if (id === 'BODY' && seen.length > 1 && seen[seen.length - 2] !== 'BODY') break;
    }
    out[mode] = { sequence: seen, reachesButtons: seen.includes('b1') && seen.includes('b2') };
  }
  await wd.navigate('about:blank');
  return out;
}

function startQuestion(projects, name, payload) {
  const cwd = mkdtempSync(join(tmpdir(), `phoenix-e2e-safari-question-${name}-`));
  projects.push(cwd);
  writeFileSync(join(cwd, 'payload.json'), JSON.stringify(payload, null, 2));
  const key = `row5-safari-${name}`;
  const res = H.engine('serve-question', ['--start', '--no-open', '--key', key, '--payload', 'payload.json', '--timeout', '900'],
    { cwd, env: { PHOENIX_UI_QUESTION_FORCE: '1' } });
  const url = res.stdout.match(/QUESTION URL: (\S+)/)?.[1];
  if (res.code !== 0 || !url) throw new Error(`serve-question --start failed: ${JSON.stringify(res)}`);
  const state = JSON.parse(readFileSync(join(cwd, '.phoenix-ui', 'questions', `${key}.state.json`), 'utf-8'));
  return { cwd, key, url, pid: state.pid, start: { code: res.code, stdout: res.stdout } };
}

function waitAnswer(q) {
  const res = H.engine('serve-question', ['--wait', '--key', q.key, '--poll', '15'], { cwd: q.cwd });
  const line = res.stdout.match(/^ANSWER: (.*)$/m)?.[1];
  return { code: res.code, stdout: res.stdout, answer: line ? JSON.parse(line) : null };
}

function stopQuestion(q) {
  if (!q) return null;
  if (!existsSync(join(q.cwd, '.phoenix-ui', 'questions', `${q.key}.state.json`))) return null;
  return H.engine('serve-question', ['--stop', '--key', q.key], { cwd: q.cwd }).stdout;
}

const pidGone = (pid) => { try { process.kill(pid, 0); return false; } catch { return true; } };

async function openQuestion(row, q, viewport) {
  const { wd } = row.ctx;
  await wd.navigate('about:blank');
  const vp = await wd.setViewport(viewport.width, viewport.height);
  await wd.navigate(q.url);
  await wd.exec(installPageRecorder);
  return { vp, loadedAt: Date.now() };
}

async function reloadQuestion(wd) {
  await wd.refresh();
  const at = Date.now();
  await wd.exec(installPageRecorder);
  await sleep(DEAL_SETTLE_MS);
  return at;
}

const answersOf = (log) => log.posts.filter((p) => p.path === '/answer');
const pageErrorsOf = (log) => log.errors.filter((e) => !e.resource).concat(log.logs.filter((l) => l.level === 'error'));
const doneText = (wd) => wd.exec(() => document.querySelector('.done')?.textContent || null).catch(() => null);

/** Walk with plain Tab; if that skips the choose buttons, walk again with Option-Tab (Safari's full keyboard access). */
async function keyboardWalk(row, cards, label) {
  const walks = {};
  walks.tab = await tabWalk(row, { label: `${label}-tab`, mode: 'tab', shots: true });
  let kb = keyboardChecks(cards, walks.tab);
  let mode = 'tab';
  let reloadedAt = null;
  if (!kb.checks.everyCardChoosableByKeyboard) {
    reloadedAt = await reloadQuestion(row.ctx.wd);
    walks['option-tab'] = await tabWalk(row, { label: `${label}-option-tab`, mode: 'option-tab', shots: true });
    kb = keyboardChecks(cards, walks['option-tab']);
    mode = 'option-tab';
  }
  return { walks, kb, mode, reloadedAt };
}

async function row5(row) {
  const { wd } = row.ctx;
  const projects = [];
  row.data.limitations.push(
    'Safari desktop has no touch input: the narrow-case tap is a pointer click (touch tried first) at a ~375px-wide window.',
    'No Playwright clock in Safari: the broken-image case waits the page\'s real 4-minute patience.',
    'No CDP/network log: POST /answer comes from an in-page fetch wrapper; page errors are window error/unhandledrejection events and console.error from load on; "only loopback" uses the Resource Timing list.',
  );
  try {
    row.data.meta.keyboardControl = await keyboardControl(row).catch((err) => ({ error: err.message }));
    log(`keyboard control: ${JSON.stringify(row.data.meta.keyboardControl)}`);
    // null when the control page could not run: then only plain Tab can pass.
    const plainTabReachesButtons = row.data.meta.keyboardControl.tab?.reachesButtons ?? true;

    await runCase(row, 'desktop: text-only catalog cards are comparable, reachable by keyboard with a visible ring, and Enter picks one', async (e) => {
      const payload = textOnlyRound();
      const q = startQuestion(projects, 'desktop', payload);
      Object.assign(e, { url: q.url, payload, start: q.start });
      try {
        const { vp } = await openQuestion(row, q, DESKTOP);
        e.viewport = vp;
        await sleep(DEAL_SETTLE_MS);
        const cards = await wd.exec(readCards);
        await shot(row, 'desktop-1-cards');
        const { walks, kb, mode } = await keyboardWalk(row, cards, 'desktop');
        const target = payload.options[1].id;
        await reloadQuestion(wd);
        const focused = await tabTo(row, (s) => s.cls === 'choose' && s.dataId === target, mode);
        await shot(row, 'desktop-2-focused-before-enter');
        await wd.press(KEYS.ENTER);
        const net = newPageLog();
        await waitFor(async () => answersOf(await drain(wd, net)).length > 0, { label: 'answer POST', timeout: 10_000 }).catch(() => null);
        const done = await waitFor(() => doneText(wd), { label: 'choice recorded', timeout: 10_000 }).catch(() => null);
        await drain(wd, net);
        await shot(row, 'desktop-3-chosen');
        const collected = waitAnswer(q);
        const answers = answersOf(net);
        const sameAnatomy = cards.filter((c) => !c.declined).every((c) => ['First viewport', 'Risk'].every((f) => c.facts.includes(f)));
        Object.assign(e, { keyboardMode: mode, cards, walks, badStops: kb.bad, partiallyCovered: kb.partiallyCovered, chooseRings: kb.chooseRings,
          focusedBeforeEnter: focused, answers, done, collected, page: net });
        return {
          desktopViewport: vp.inner.w === DESKTOP.width,
          allCardsRendered: cards.length === payload.options.length,
          allTextOnly: cards.every((c) => c.textOnly && c.media.length === 0 && !c.hasBack),
          labelsAndThesesShown: cards.every((c) => c.label && c.thesis),
          sameAnatomyAcrossCards: sameAnatomy && cards.every((c) => c.swatches > 0),
          everyChooseEnabled: cards.every((c) => c.choose && !c.choose.disabled && c.choose.w > 0),
          ...kb.checks,
          keyboardModeMatchesSafariControl: mode === 'tab' || !plainTabReachesButtons,
          enterOnFocusedChoosePosted: focused !== null && answers.length === 1 && JSON.parse(answers[0].body).optionId === target,
          choiceConfirmedOnPage: typeof done === 'string' && /Choice recorded/.test(done),
          waitCollectedTheChoice: collected.code === 0 && collected.answer?.optionId === target,
          noPageErrors: pageErrorsOf(net).length === 0,
          serverExited: await waitFor(() => pidGone(q.pid), { label: 'server exit', timeout: 10_000 }).catch(() => false),
        };
      } finally {
        e.stop = stopQuestion(q);
      }
    });

    await runCase(row, 'narrow: the same text cards stack, stay reachable by keyboard with a visible ring, and a click picks one', async (e) => {
      const payload = textOnlyRound();
      const q = startQuestion(projects, 'narrow', payload);
      Object.assign(e, { url: q.url, payload, start: q.start });
      try {
        const { vp } = await openQuestion(row, q, NARROW);
        e.viewport = vp;
        await sleep(DEAL_SETTLE_MS);
        const cards = await wd.exec(readCards);
        const layout = await wd.exec(() => ({ scrollWidth: document.documentElement.scrollWidth, innerWidth, innerHeight,
          deckVertical: matchMedia('(max-aspect-ratio: 1/1)').matches }));
        await shot(row, 'narrow-1-cards');
        const { walks, kb, mode } = await keyboardWalk(row, cards, 'narrow');
        const target = payload.options[2].id;
        await reloadQuestion(wd);
        const sel = `.card[data-id="${target}"] .face.front button.choose`;
        await wd.exec((s) => { const b = document.querySelector(s); if (b.scrollIntoViewIfNeeded) b.scrollIntoViewIfNeeded(); else b.scrollIntoView({ block: 'nearest' }); }, sel);
        await sleep(600);
        await shot(row, 'narrow-2-before-tap');
        const box = await wd.exec((s) => {
          const b = document.querySelector(s);
          const r = b.getBoundingClientRect();
          const cx = r.left + r.width / 2;
          const cy = r.top + r.height / 2;
          const hit = document.elementFromPoint(cx, cy);
          return { cx, cy, hitsSelf: !!hit && (hit === b || b.contains(hit)), hit: hit ? hit.tagName + '.' + hit.className : null };
        }, sel);
        const net = newPageLog();
        let tapMethod = null;
        let tapError = null;
        try {
          await wd.pointerClick(box.cx, box.cy, { pointerType: 'touch' });
          tapMethod = 'touch';
          await waitFor(async () => answersOf(await drain(wd, net)).length > 0, { label: 'answer after touch', timeout: 3_000 });
        } catch (err) {
          if (answersOf(net).length === 0) {
            tapError = `touch: ${err.message.split('\n')[0]}`;
            try {
              await wd.pointerClick(box.cx, box.cy);
              tapMethod = 'pointer-mouse';
            } catch (err2) { tapError += ` | mouse: ${err2.message.split('\n')[0]}`; }
          }
        }
        await waitFor(async () => answersOf(await drain(wd, net)).length > 0, { label: 'answer POST', timeout: 10_000 }).catch(() => null);
        const done = await waitFor(() => doneText(wd), { label: 'choice recorded', timeout: 10_000 }).catch(() => null);
        await drain(wd, net);
        await shot(row, 'narrow-3-chosen');
        const collected = waitAnswer(q);
        const answers = answersOf(net);
        const xs = cards.map((c) => c.cardBox.x);
        Object.assign(e, { keyboardMode: mode, cards, layout, walks, badStops: kb.bad, partiallyCovered: kb.partiallyCovered, chooseRings: kb.chooseRings,
          tapBox: box, tapMethod, tapError, answers, done, collected, page: net });
        return {
          narrowViewport: vp.inner.w <= 400,
          allCardsRendered: cards.length === payload.options.length,
          allTextOnly: cards.every((c) => c.textOnly && c.media.length === 0),
          deckStacksVertically: layout.deckVertical && Math.max(...xs) - Math.min(...xs) <= 16
            && cards.every((c, i) => c.cardBox.w <= layout.innerWidth && (i === 0 || c.cardBox.y > cards[i - 1].cardBox.y)),
          noHorizontalPageScroll: layout.scrollWidth <= layout.innerWidth,
          everyChooseEnabled: cards.every((c) => c.choose && !c.choose.disabled && c.choose.w > 0),
          ...kb.checks,
          keyboardModeMatchesSafariControl: mode === 'tab' || !plainTabReachesButtons,
          tapOnChoosePosted: tapMethod !== null && answers.length === 1 && JSON.parse(answers[0].body).optionId === target,
          choiceConfirmedOnPage: typeof done === 'string' && /Choice recorded/.test(done),
          waitCollectedTheChoice: collected.code === 0 && collected.answer?.optionId === target,
          noPageErrors: pageErrorsOf(net).length === 0,
        };
      } finally {
        e.stop = stopQuestion(q);
      }
    });

    await runCase(row, 'broken images: every card stays choosable, failing art is labeled, a pending comp falls back to text, and a keyboard pick lands', async (e) => {
      const closedPort = await freePort();
      const payload = brokenImageRound(closedPort);
      const q = startQuestion(projects, 'broken', payload);
      Object.assign(e, { url: q.url, payload, start: q.start, closedPort });
      const [assigned, brokenHero, missingBoard, declined] = payload.options;
      const byId = (cards, id) => cards.find((c) => c.id === id);
      try {
        const { vp, loadedAt } = await openQuestion(row, q, DESKTOP);
        e.viewport = vp;
        await sleep(DEAL_SETTLE_MS);
        const settled = await waitFor(async () => {
          const cards = await wd.exec(readCards);
          const hero = byId(cards, brokenHero.id);
          const decl = byId(cards, declined.id);
          return hero?.media.some((m) => m.label === 'artwork unavailable') && decl?.inspo === 0 ? cards : null;
        }, { label: 'broken art settled', timeout: 15_000 }).catch(() => null);
        const initial = settled || await wd.exec(readCards);
        await shot(row, 'broken-1-initial');
        const { walks, kb, mode, reloadedAt } = await keyboardWalk(row, initial, 'broken-pending');
        const pageStart = reloadedAt || loadedAt;
        // The page's own patience: four quiet minutes, in real time here.
        const fallback = await waitFor(async () => {
          const c = byId(await wd.exec(readCards), assigned.id);
          return c && c.textOnly && c.media.length === 0 ? c : null;
        }, { label: 'pending comp fell back to text', timeout: Math.max(30_000, pageStart + 300_000 - Date.now()), interval: 2_000 }).catch(() => null);
        const fallbackAfterMs = Date.now() - pageStart;
        const afterFallback = await wd.exec(readCards);
        await shot(row, 'broken-2-after-fallback');
        await wd.exec(() => { if (document.activeElement && document.activeElement.blur) document.activeElement.blur(); window.scrollTo(0, 0); });
        const focused = await tabTo(row, (s) => s.cls === 'choose' && s.dataId === brokenHero.id, mode);
        await wd.press(KEYS.ENTER);
        const net = newPageLog();
        await waitFor(async () => answersOf(await drain(wd, net)).length > 0, { label: 'answer POST', timeout: 10_000 }).catch(() => null);
        const done = await waitFor(() => doneText(wd), { label: 'choice recorded', timeout: 10_000 }).catch(() => null);
        await drain(wd, net);
        await shot(row, 'broken-3-chosen');
        const resources = await wd.exec(() => performance.getEntriesByType('resource').map((r) => r.name)).catch(() => []);
        const collected = waitAnswer(q);
        const answers = answersOf(net);
        const a0 = byId(initial, assigned.id);
        const b0 = byId(initial, brokenHero.id);
        const c0 = byId(initial, missingBoard.id);
        const d0 = byId(initial, declined.id);
        Object.assign(e, { keyboardMode: mode, initial, afterFallback, walks, badStops: kb.bad, partiallyCovered: kb.partiallyCovered, chooseRings: kb.chooseRings,
          fallback, fallbackAfterMs, focusedBeforeEnter: focused, answers, done, collected, resources, page: net });
        return {
          allCardsRendered: initial.length === payload.options.length,
          pendingCompShowsProgressNotVoid: !!a0?.media.some((m) => /comp-pending/.test(m.cls)),
          failingArtLabeledUnavailable: !!b0?.media.some((m) => /unavailable/.test(m.cls) && m.label === 'artwork unavailable' && !m.img),
          missingLocalArtIsTextCard: !!c0?.textOnly && c0.media.length === 0,
          failingThumbRemoved: d0?.inspo === 0,
          noBrokenImageGlyphs: initial.every((c) => c.brokenImgs === 0),
          everyChooseEnabledWhileArtFails: initial.every((c) => c.choose && !c.choose.disabled && c.choose.w > 0),
          everyCardChoosableByKeyboardWhilePending: kb.checks.everyCardChoosableByKeyboard,
          focusRingVisibleWhilePending: kb.checks.everyChooseFocusRingVisible && kb.checks.everyFocusedChooseOnScreen,
          noFocusOnHiddenCardFace: kb.checks.noFocusOnHiddenCardFace,
          keyboardModeMatchesSafariControl: mode === 'tab' || !plainTabReachesButtons,
          pendingCompFellBackToText: fallback !== null && ['First viewport', 'The case', 'Risk'].every((f) => fallback.facts.includes(f)) && !fallback.hasBack,
          keyboardPickPosted: focused !== null && answers.length === 1 && JSON.parse(answers[0].body).optionId === brokenHero.id,
          choiceConfirmedOnPage: typeof done === 'string' && /Choice recorded/.test(done),
          waitCollectedTheChoice: collected.code === 0 && collected.answer?.optionId === brokenHero.id,
          onlyLoopbackRequests: resources.every((u) => /^(https?:\/\/(127\.0\.0\.1|localhost)(:\d+)?\/|data:|blob:)/.test(u)),
        };
      } finally {
        e.stop = stopQuestion(q);
      }
    });
  } finally {
    await wd.navigate('about:blank').catch(() => {});
    for (const p of projects) {
      if (p.startsWith(tmpdir()) && basename(p).startsWith('phoenix-e2e-safari-question-')) rmSync(p, { recursive: true, force: true });
    }
  }
}

const ROW_FNS = { row2, row3, row5 };

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function acquireAutomation(driver) {
  const deadline = Date.now() + WAIT_AUTOMATION_MIN * 60_000;
  const attempts = [];
  let otherErrors = 0;
  for (;;) {
    const probe = await probeAutomation(driver);
    attempts.push({ at: iso(), available: probe.available, message: probe.message || null });
    if (probe.available) return { available: true, attempts, capabilities: probe.capabilities };
    if (!probe.automationOff && ++otherErrors > 3) return { available: false, attempts, message: probe.message };
    if (Date.now() + POLL_S * 1000 > deadline) return { available: false, attempts, message: probe.message };
    log(`Safari automation unavailable (${probe.automationOff ? 'remote automation off' : probe.message}); retry in ${POLL_S}s`);
    await sleep(POLL_S * 1000);
  }
}

async function main() {
  mkdirSync(OUT, { recursive: true });
  const driver = await startSafariDriver();
  log(`safaridriver ${SAFARIDRIVER} pid=${driver.pid} port=${driver.port}`);
  const summary = { startedAt: iso(), engine: process.env.IMPECCABLE_BIN || null, safari: safariAppVersion(), driver: { bin: SAFARIDRIVER, port: driver.port }, rows: {} };
  let exitCode = 0;
  try {
    const auto = await acquireAutomation(driver);
    summary.automation = { available: auto.available, attempts: auto.attempts.length, lastMessage: auto.message || null, firstAt: auto.attempts[0]?.at, lastAt: auto.attempts.at(-1)?.at };
    if (PROBE) {
      writeJson(outPath('probe.json'), { ...summary, attempts: auto.attempts, capabilities: auto.capabilities || null });
      log(`probe: ${auto.available ? 'available' : 'unavailable'}`);
      return auto.available ? 0 : 2;
    }
    if (!auto.available) {
      for (const key of ROWS) {
        const data = { row: key, status: 'env-missing', at: iso(), reason: auto.message, attempts: auto.attempts,
          howToEnable: 'Run `sudo safaridriver --enable` (or Safari > Settings > Advanced > Show features for web developers, then Develop > Allow Remote Automation).' };
        writeJson(outPath(ROW_DIRS[key], 'results.json'), data);
        summary.rows[key] = { status: 'env-missing', results: relative(CHECKOUT_ROOT, join(OUT, ROW_DIRS[key], 'results.json')) };
      }
      return 2;
    }
    await harness();
    const engine = H.engineInfo();
    summary.engineInfo = engine;
    for (const key of ROWS) {
      currentLog = [];
      const busy = await waitUntilIdle(key);
      let wd = null;
      let row = null;
      try {
        wd = await newSafariSession(driver, { prelude: PRELUDE });
        const caps = wd.capabilities;
        const meta = { engine, browser: { name: caps.browserName, version: caps.browserVersion, platform: caps.platformName, safariApp: safariAppVersion(),
          driver: SAFARIDRIVER, driverPort: driver.port }, busyWaitMs: busy.waitedMs };
        row = newRow(key, wd, meta);
        log(`${key}: Safari ${caps.browserVersion} session ${wd.id}`);
        await ROW_FNS[key](row);
        row.data.status = row.data.cases.length && row.data.cases.every((c) => c.pass) ? 'pass' : 'fail';
      } catch (err) {
        if (!row) row = newRow(key, wd, {});
        row.data.status = 'error';
        row.data.error = err.stack || String(err);
        log(`${key}: ERROR ${err.message}`);
      } finally {
        if (wd) await wd.delete().catch((err) => log(`delete session: ${err.message}`));
        row.data.finishedAt = iso();
        row.data.clicks = row.ctx.clicks;
        writeJson(outPath(ROW_DIRS[key], 'results.json'), row.data);
        writeFileSync(outPath(ROW_DIRS[key], 'run.log'), currentLog.join('\n') + '\n');
        writeFileSync(outPath(ROW_DIRS[key], 'safaridriver.log'), driver.log() + '\n');
        summary.rows[key] = { status: row.data.status, cases: row.data.cases.map((c) => ({ case: c.case, pass: c.pass, failedChecks: c.failedChecks || [], error: c.error?.split('\n')[0] || null })),
          results: relative(CHECKOUT_ROOT, join(OUT, ROW_DIRS[key], 'results.json')) };
        if (row.data.status !== 'pass') exitCode = 1;
      }
    }
    return exitCode;
  } finally {
    summary.finishedAt = iso();
    if (!PROBE) writeJson(outPath('summary.json'), summary);
    if (H) H.removeNodeModulesTemplates();
    await driver.stop();
  }
}

main().then((code) => process.exit(code), (err) => {
  console.error(err.stack || err);
  process.exit(1);
});
