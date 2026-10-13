/**
 * Plan §6 row 5 「缺卡图、键盘与窄屏选择」, Chrome: the direction chooser that
 * `phoenix-ui serve-question` serves, with cards that have no image.
 *
 * The cards come from the bundled concept catalog
 * (skills/phoenix-ui/src/scripts/data/catalog/concept-ingredients.json),
 * which carries no artwork, so a catalog round is a text-only round.
 *
 *   - text-only round, desktop 1440x900: every card is a text card with the
 *     same anatomy; Tab reaches every "choose" button with a visible focus
 *     ring that nothing covers; Enter picks one; `--wait` collects it.
 *   - the same round, narrow 375x812 touch viewport: the same keyboard walk,
 *     then a tap picks a card.
 *   - broken-image round: a comp that never lands, an inspiration URL that
 *     fails, an inspiration path that does not exist, a declined card with a
 *     failing thumb. Every card stays choosable at once; the failing art
 *     collapses to an honest label; after the page's 4-minute patience (run
 *     on Playwright's fake clock) the pending comp falls back to a text card;
 *     a keyboard pick lands while that is going on.
 *
 * Images only ever point at this machine (a closed loopback port or a missing
 * file); nothing external is requested.
 *
 * Run (cwd tools/phoenix-ui):
 *   IMPECCABLE_BIN=<phoenix-ui engine> PHOENIX_E2E_CHROME=<chrome> \
 *   PHOENIX_E2E_EVIDENCE_DIR=<abs dir> node --test --test-concurrency=1 \
 *     tests/live-e2e/phoenix-question.test.mjs
 */

import { after, before, describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { browserInfo, engine, engineInfo, evidencePath, launchChrome, waitFor, writeEvidence } from './phoenix-harness.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const CHECKOUT_ROOT = resolve(__dirname, '..', '..', '..', '..');
const CATALOG = join(CHECKOUT_ROOT, 'skills', 'phoenix-ui', 'src', 'scripts', 'data', 'catalog', 'concept-ingredients.json');
const ROW5 = 'row5-question-cards';
const DESKTOP = { width: 1440, height: 900 };
const NARROW = { width: 375, height: 812 };
const DEAL_SETTLE_MS = 2_500;
const PALETTES = [
  ['#eef1ee', '#1d2421', '#2357c6'],
  ['#fbf7ef', '#2a2620', '#b5832a'],
  ['#f2f4f7', '#101828', '#0f766e'],
  ['#f7f3f0', '#2b2118', '#9a3412'],
];

const meta = {};
const results = [];
const projects = [];
let browser;
let closedPort;

// ---------------------------------------------------------------------------
// Payloads from the bundled catalog
// ---------------------------------------------------------------------------

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

function brokenImageRound() {
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

// ---------------------------------------------------------------------------
// serve-question lifecycle
// ---------------------------------------------------------------------------

function startQuestion(name, payload) {
  const cwd = mkdtempSync(join(tmpdir(), `phoenix-e2e-question-${name}-`));
  projects.push(cwd);
  writeFileSync(join(cwd, 'payload.json'), JSON.stringify(payload, null, 2));
  const key = `row5-${name}`;
  const res = engine('serve-question', ['--start', '--no-open', '--key', key, '--payload', 'payload.json', '--timeout', '600'],
    { cwd, env: { PHOENIX_UI_QUESTION_FORCE: '1' } });
  const url = res.stdout.match(/QUESTION URL: (\S+)/)?.[1];
  if (res.code !== 0 || !url) throw new Error(`serve-question --start failed: ${JSON.stringify(res)}`);
  const state = JSON.parse(readFileSync(join(cwd, '.phoenix-ui', 'questions', `${key}.state.json`), 'utf-8'));
  return { cwd, key, url, pid: state.pid, start: { code: res.code, stdout: res.stdout } };
}

function waitAnswer(q) {
  const res = engine('serve-question', ['--wait', '--key', q.key, '--poll', '15'], { cwd: q.cwd });
  const line = res.stdout.match(/^ANSWER: (.*)$/m)?.[1];
  return { code: res.code, stdout: res.stdout, answer: line ? JSON.parse(line) : null };
}

function stopQuestion(q) {
  if (!q) return null;
  if (!existsSync(join(q.cwd, '.phoenix-ui', 'questions', `${q.key}.state.json`))) return null;
  return engine('serve-question', ['--stop', '--key', q.key], { cwd: q.cwd }).stdout;
}

function pidGone(pid) {
  try { process.kill(pid, 0); return false; } catch { return true; }
}

async function findClosedPort() {
  const srv = createServer();
  await new Promise((r) => srv.listen(0, '127.0.0.1', r));
  const { port } = srv.address();
  await new Promise((r) => srv.close(r));
  return port;
}

async function openQuestion(q, { viewport, touch = false, reducedMotion, clock = false } = {}) {
  const ctx = await browser.newContext({
    viewport, deviceScaleFactor: touch ? 2 : 1, isMobile: touch, hasTouch: touch,
    ...(reducedMotion ? { reducedMotion } : {}),
  });
  const page = await ctx.newPage();
  const net = { answers: [], imageResponses: [], failed: [], console: [] };
  page.on('request', (req) => {
    if (req.method() === 'POST' && new URL(req.url()).pathname === '/answer') net.answers.push({ at: Date.now(), body: req.postData() });
  });
  page.on('response', (res) => {
    if (/\/img\/|hero-|board/.test(res.url())) net.imageResponses.push({ url: res.url(), status: res.status() });
  });
  page.on('requestfailed', (req) => net.failed.push({ url: req.url(), error: req.failure()?.errorText || null }));
  page.on('console', (m) => { if (m.type() === 'error') net.console.push({ text: m.text(), url: m.location()?.url || null }); });
  page.on('pageerror', (e) => net.console.push({ text: `pageerror: ${e.message}`, url: null }));
  if (clock) await page.clock.install();
  await page.goto(q.url, { waitUntil: 'domcontentloaded' });
  return { ctx, page, net };
}

// ---------------------------------------------------------------------------
// Page probes
// ---------------------------------------------------------------------------

/** Every card as the user sees it. */
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

/** The focused element: identity, focus ring, and whether anything hides it. */
function describeFocus() {
  const el = document.activeElement;
  if (!el || el === document.body || el === document.documentElement) return { tag: 'BODY' };
  const r = el.getBoundingClientRect();
  const cs = getComputedStyle(el);
  const inset = 6;
  const pts = [[r.left + r.width / 2, r.top + r.height / 2], [r.left + inset, r.top + inset], [r.right - inset, r.top + inset],
    [r.left + inset, r.bottom - inset], [r.right - inset, r.bottom - inset]];
  const hitOf = ([x, y]) => document.elementFromPoint(x, y);
  const centreHit = hitOf(pts[0]);
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
    // A back face that is turned away (or a front that is) is not on screen:
    // backface-visibility hides it, but its controls still take focus.
    onHiddenFace: (faceName === 'back' && !flipped) || (faceName === 'front' && flipped),
    text: (el.textContent || el.getAttribute('aria-label') || el.placeholder || '').trim().slice(0, 40),
    focusVisible: el.matches(':focus-visible'),
    outline: { style: cs.outlineStyle, width: cs.outlineWidth, color: cs.outlineColor, offset: cs.outlineOffset },
    rect: { x: Math.round(r.left), y: Math.round(r.top), w: Math.round(r.width), h: Math.round(r.height) },
    inViewport: r.left >= 0 && r.top >= 0 && r.right <= innerWidth && r.bottom <= innerHeight,
    coveredBy,
    coveredByCentre,
    underFade,
  };
}

/** The focus after any focus scroll / snap has come to rest. */
async function settledFocus(page) {
  await page.waitForTimeout(250);
  let last = await page.evaluate(describeFocus);
  for (let i = 0; i < 15; i++) {
    await page.waitForTimeout(200);
    const cur = await page.evaluate(describeFocus);
    if (JSON.stringify(cur.rect) === JSON.stringify(last.rect)) return cur;
    last = cur;
  }
  return last;
}

/** Tab from the top of the page until focus leaves the document. */
async function tabWalk(page, { label, shots = false, max = 40 } = {}) {
  const stops = [];
  for (let i = 0; i < max; i++) {
    await page.keyboard.press('Tab');
    const stop = await settledFocus(page);
    if (stop.tag === 'BODY') {
      if (stops.length) break;
      continue;
    }
    stops.push(stop);
    if (shots && (stop.cls === 'choose' || stop.onHiddenFace)) {
      const name = `${label}-focus-${String(stops.length).padStart(2, '0')}-${stop.cardId || stop.id || 'page'}-${stop.face || 'page'}-${(stop.cls || stop.tag).replace(/\s+/g, '.')}`;
      await page.screenshot({ path: evidencePath(ROW5, `${name}.png`) });
    }
  }
  return stops;
}

function focusRingOk(stop) {
  return stop.focusVisible && stop.outline.style !== 'none' && parseFloat(stop.outline.width) >= 2;
}

function chooseStops(stops) {
  return stops.filter((s) => s.tag === 'BUTTON' && s.cls === 'choose' && !s.onHiddenFace);
}

/** Not hidden: wholly inside the viewport, centre not under another element or a visible edge fade. */
function focusNotObscured(stop) {
  const centreCovered = stop.coveredBy.length > 0 && stop.coveredByCentre;
  return stop.inViewport && !centreCovered && !stop.underFade;
}

/** Keyboard checks shared by every round. */
function keyboardChecks(cards, stops) {
  const chooses = chooseStops(stops);
  const reached = new Set(chooses.map((s) => s.dataId));
  const bad = chooses.filter((s) => !focusRingOk(s) || !focusNotObscured(s));
  return {
    everyCardChoosableByTab: cards.every((c) => reached.has(c.id)),
    everyChooseFocusRingVisible: chooses.length > 0 && chooses.every(focusRingOk),
    everyFocusedChooseOnScreen: chooses.length > 0 && bad.length === 0,
    everyStopHasFocusRing: stops.every((s) => s.outline.style !== 'none' && s.focusVisible),
    noFocusOnHiddenCardFace: stops.every((s) => !s.onHiddenFace),
    bad,
    partiallyCovered: chooses.filter((s) => s.coveredBy.length > 0).map((s) => ({ dataId: s.dataId, coveredBy: s.coveredBy })),
  };
}

async function tabTo(page, predicate, max = 40) {
  for (let i = 0; i < max; i++) {
    await page.keyboard.press('Tab');
    const stop = await settledFocus(page);
    if (predicate(stop)) return stop;
  }
  return null;
}

function splitChecks(all) {
  const { bad, partiallyCovered, ...checks } = all;
  return { checks, bad, partiallyCovered };
}

/** Console errors other than Chrome's own /favicon.ico probe (the page declares none). */
function pageErrors(net) {
  return net.console.filter((c) => !/\/favicon\.ico$/.test(c.url || ''));
}

function allTrue(checks) {
  return Object.fromEntries(Object.keys(checks).map((k) => [k, true]));
}

before(async () => {
  meta.engine = engineInfo();
  browser = await launchChrome();
  meta.browser = browserInfo(browser);
  meta.startedAt = new Date().toISOString();
  meta.catalog = CATALOG;
  closedPort = await findClosedPort();
  meta.closedPort = closedPort;
});

after(async () => {
  if (browser) await browser.close();
  writeEvidence(`${ROW5}/results.json`, { meta, results });
  for (const p of projects) {
    if (p.startsWith(tmpdir()) && basename(p).startsWith('phoenix-e2e-question-')) rmSync(p, { recursive: true, force: true });
  }
});

describe('row 5: cards without images, keyboard and narrow selection', () => {
  it('desktop: text-only catalog cards are comparable, reachable by Tab with a visible ring, and Enter picks one', async () => {
    const payload = textOnlyRound();
    const q = startQuestion('desktop', payload);
    const run = { case: 'desktop text-only', url: q.url, payload, start: q.start };
    results.push(run);
    let opened;
    try {
      opened = await openQuestion(q, { viewport: DESKTOP });
      const { page, net } = opened;
      await page.waitForTimeout(DEAL_SETTLE_MS);
      const cards = await page.evaluate(readCards);
      await page.screenshot({ path: evidencePath(ROW5, 'desktop-1-cards.png') });
      await page.screenshot({ path: evidencePath(ROW5, 'desktop-1-cards-full.png'), fullPage: true });
      const stops = await tabWalk(page, { label: 'desktop', shots: true });
      const kb = splitChecks(keyboardChecks(cards, stops));
      const target = payload.options[1].id;
      await page.reload({ waitUntil: 'domcontentloaded' });
      await page.waitForTimeout(DEAL_SETTLE_MS);
      const focused = await tabTo(page, (s) => s.cls === 'choose' && s.dataId === target);
      await page.keyboard.press('Enter');
      await waitFor(() => net.answers.length > 0, { label: 'answer POST', timeout: 10_000 }).catch(() => null);
      const done = await waitFor(() => page.evaluate(() => document.querySelector('.done')?.textContent || null),
        { label: 'choice recorded', timeout: 10_000 }).catch(() => null);
      await page.screenshot({ path: evidencePath(ROW5, 'desktop-2-chosen.png') });
      const collected = waitAnswer(q);
      const sameAnatomy = cards.filter((c) => !c.declined).every((c) => ['First viewport', 'Risk'].every((f) => c.facts.includes(f)));
      const checks = {
        allCardsRendered: cards.length === payload.options.length,
        allTextOnly: cards.every((c) => c.textOnly && c.media.length === 0 && !c.hasBack),
        labelsAndThesesShown: cards.every((c) => c.label && c.thesis),
        sameAnatomyAcrossCards: sameAnatomy && cards.every((c) => c.swatches > 0),
        everyChooseEnabled: cards.every((c) => c.choose && !c.choose.disabled && c.choose.w > 0),
        ...kb.checks,
        enterOnFocusedChoosePosted: focused !== null && net.answers.length === 1 && JSON.parse(net.answers[0].body).optionId === target,
        choiceConfirmedOnPage: typeof done === 'string' && /Choice recorded/.test(done),
        waitCollectedTheChoice: collected.code === 0 && collected.answer?.optionId === target,
        noPageErrors: pageErrors(net).length === 0,
        serverExited: await waitFor(() => pidGone(q.pid), { label: 'server exit', timeout: 10_000 }).catch(() => false),
      };
      Object.assign(run, { checks, cards, stops, badStops: kb.bad, partiallyCovered: kb.partiallyCovered, focusedBeforeEnter: focused, answers: net.answers, done, collected, console: net.console });
      assert.deepEqual(checks, allTrue(checks));
    } finally {
      if (opened) await opened.ctx.close();
      run.stop = stopQuestion(q);
    }
  });

  it('narrow: the same text cards stack, stay reachable by Tab with a visible ring, and a tap picks one', async () => {
    const payload = textOnlyRound();
    const q = startQuestion('narrow', payload);
    const run = { case: 'narrow text-only', url: q.url, payload, start: q.start };
    results.push(run);
    let opened;
    try {
      opened = await openQuestion(q, { viewport: NARROW, touch: true });
      const { page, net } = opened;
      await page.waitForTimeout(DEAL_SETTLE_MS);
      const cards = await page.evaluate(readCards);
      const layout = await page.evaluate(() => ({
        scrollWidth: document.documentElement.scrollWidth, innerWidth,
        deckVertical: matchMedia('(max-aspect-ratio: 1/1)').matches,
      }));
      await page.screenshot({ path: evidencePath(ROW5, 'narrow-1-cards.png') });
      await page.screenshot({ path: evidencePath(ROW5, 'narrow-1-cards-full.png'), fullPage: true });
      const stops = await tabWalk(page, { label: 'narrow', shots: true });
      const kb = splitChecks(keyboardChecks(cards, stops));
      const target = payload.options[2].id;
      await page.reload({ waitUntil: 'domcontentloaded' });
      await page.waitForTimeout(DEAL_SETTLE_MS);
      const button = page.locator(`.card[data-id="${target}"] .face.front button.choose`);
      let tapError = null;
      try {
        await button.scrollIntoViewIfNeeded();
        await page.waitForTimeout(600);
        await page.screenshot({ path: evidencePath(ROW5, 'narrow-2-before-tap.png') });
        await button.tap({ timeout: 10_000 });
      } catch (err) {
        tapError = err.message.split('\n').slice(0, 6).join('\n');
      }
      await waitFor(() => net.answers.length > 0, { label: 'answer POST', timeout: 10_000 }).catch(() => null);
      const done = await waitFor(() => page.evaluate(() => document.querySelector('.done')?.textContent || null),
        { label: 'choice recorded', timeout: 10_000 }).catch(() => null);
      await page.screenshot({ path: evidencePath(ROW5, 'narrow-3-chosen.png') });
      const collected = waitAnswer(q);
      const xs = cards.map((c) => c.cardBox.x);
      const checks = {
        allCardsRendered: cards.length === payload.options.length,
        allTextOnly: cards.every((c) => c.textOnly && c.media.length === 0),
        // Cards sit at a slight fan angle, so their boxes differ by a few px.
        deckStacksVertically: layout.deckVertical && Math.max(...xs) - Math.min(...xs) <= 16
          && cards.every((c, i) => c.cardBox.w <= NARROW.width && (i === 0 || c.cardBox.y > cards[i - 1].cardBox.y)),
        noHorizontalPageScroll: layout.scrollWidth <= layout.innerWidth,
        everyChooseEnabled: cards.every((c) => c.choose && !c.choose.disabled && c.choose.w > 0),
        ...kb.checks,
        tapOnChoosePosted: tapError === null && net.answers.length === 1 && JSON.parse(net.answers[0].body).optionId === target,
        choiceConfirmedOnPage: typeof done === 'string' && /Choice recorded/.test(done),
        waitCollectedTheChoice: collected.code === 0 && collected.answer?.optionId === target,
        noPageErrors: pageErrors(net).length === 0,
      };
      Object.assign(run, { checks, cards, layout, stops, badStops: kb.bad, partiallyCovered: kb.partiallyCovered, tapError, answers: net.answers, done, collected, console: net.console });
      assert.deepEqual(checks, allTrue(checks));
    } finally {
      if (opened) await opened.ctx.close();
      run.stop = stopQuestion(q);
    }
  });

  it('broken images: every card stays choosable, failing art is labeled, a pending comp falls back to text, and a keyboard pick lands', async () => {
    const payload = brokenImageRound();
    const q = startQuestion('broken', payload);
    const run = { case: 'broken images', url: q.url, payload, start: q.start };
    results.push(run);
    const [assigned, brokenHero, missingBoard, declined] = payload.options;
    let opened;
    try {
      // Reduced motion skips the deal animation, whose timers the fake clock
      // would otherwise hold.
      opened = await openQuestion(q, { viewport: DESKTOP, reducedMotion: 'reduce', clock: true });
      const { page, net } = opened;
      const settled = await waitFor(async () => {
        const cards = await page.evaluate(readCards);
        const hero = cards.find((c) => c.id === brokenHero.id);
        const decl = cards.find((c) => c.id === declined.id);
        return hero?.media.some((m) => m.label === 'artwork unavailable') && decl?.inspo === 0 ? cards : null;
      }, { label: 'broken art settled', timeout: 15_000 }).catch(() => null);
      const initial = settled || await page.evaluate(readCards);
      await page.screenshot({ path: evidencePath(ROW5, 'broken-1-initial.png') });
      const byId = (cards, id) => cards.find((c) => c.id === id);
      const stopsWhilePending = await tabWalk(page, { label: 'broken-pending', shots: true });
      const kbPending = splitChecks(keyboardChecks(initial, stopsWhilePending));
      // The page's patience: a comp that has not landed after four quiet
      // minutes gives way to the card's text.
      await page.clock.fastForward('04:10');
      const fallback = await waitFor(async () => {
        const c = byId(await page.evaluate(readCards), assigned.id);
        return c && c.textOnly && c.media.length === 0 ? c : null;
      }, { label: 'pending comp fell back to text', timeout: 10_000 }).catch(() => null);
      const afterFallback = await page.evaluate(readCards);
      await page.screenshot({ path: evidencePath(ROW5, 'broken-2-after-fallback.png') });
      const focused = await tabTo(page, (s) => s.cls === 'choose' && s.dataId === brokenHero.id);
      await page.keyboard.press('Enter');
      await waitFor(() => net.answers.length > 0, { label: 'answer POST', timeout: 10_000 }).catch(() => null);
      const done = await waitFor(() => page.evaluate(() => document.querySelector('.done')?.textContent || null),
        { label: 'choice recorded', timeout: 10_000 }).catch(() => null);
      await page.screenshot({ path: evidencePath(ROW5, 'broken-3-chosen.png') });
      const collected = waitAnswer(q);
      const a0 = byId(initial, assigned.id);
      const b0 = byId(initial, brokenHero.id);
      const c0 = byId(initial, missingBoard.id);
      const d0 = byId(initial, declined.id);
      const checks = {
        allCardsRendered: initial.length === payload.options.length,
        pendingCompShowsProgressNotVoid: !!a0?.media.some((m) => /comp-pending/.test(m.cls)),
        failingArtLabeledUnavailable: !!b0?.media.some((m) => /unavailable/.test(m.cls) && m.label === 'artwork unavailable' && !m.img),
        missingLocalArtIsTextCard: !!c0?.textOnly && c0.media.length === 0,
        failingThumbRemoved: d0?.inspo === 0,
        noBrokenImageGlyphs: initial.every((c) => c.brokenImgs === 0),
        everyChooseEnabledWhileArtFails: initial.every((c) => c.choose && !c.choose.disabled && c.choose.w > 0),
        everyCardChoosableByTabWhilePending: kbPending.checks.everyCardChoosableByTab,
        focusRingVisibleWhilePending: kbPending.checks.everyChooseFocusRingVisible && kbPending.checks.everyFocusedChooseOnScreen,
        noFocusOnHiddenCardFace: kbPending.checks.noFocusOnHiddenCardFace,
        pendingCompFellBackToText: fallback !== null && ['First viewport', 'The case', 'Risk'].every((f) => fallback.facts.includes(f)) && !fallback.hasBack,
        keyboardPickPosted: focused !== null && net.answers.length === 1 && JSON.parse(net.answers[0].body).optionId === brokenHero.id,
        choiceConfirmedOnPage: typeof done === 'string' && /Choice recorded/.test(done),
        waitCollectedTheChoice: collected.code === 0 && collected.answer?.optionId === brokenHero.id,
        onlyLoopbackRequests: net.failed.every((f) => /^http:\/\/127\.0\.0\.1:/.test(f.url)),
      };
      Object.assign(run, { checks, initial, afterFallback, stopsWhilePending, badStops: kbPending.bad, partiallyCovered: kbPending.partiallyCovered, fallback, focusedBeforeEnter: focused,
        answers: net.answers, done, collected, imageResponses: net.imageResponses, failedRequests: net.failed, console: net.console });
      assert.deepEqual(checks, allTrue(checks));
    } finally {
      if (opened) await opened.ctx.close();
      run.stop = stopQuestion(q);
    }
  });
});
