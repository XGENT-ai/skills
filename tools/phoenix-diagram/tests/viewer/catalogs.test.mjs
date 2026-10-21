// Reader interface language: the normalized five-language catalog and the
// assembled template's runtime selection (meta.locale case-insensitively,
// then valid meta.translations, then the bundled catalog, then English),
// Phoenix identity in share-card badges and the editorial plate, and the
// catalog keys added for timeline precision and source evidence.
import test from 'node:test';
import assert from 'node:assert/strict';
import { bootReader, bundledCatalogs, templateSource } from './fixtures/reader.mjs';
import * as fixtures from './fixtures/diagrams.mjs';

const LOCALES = ['en', 'zh-CN', 'zh-TW', 'es', 'ko'];
const placeholders = message => [...new Set(message.match(/\{[a-zA-Z0-9_]+\}/g) || [])].sort();
const fill = (message, values = {}) => message.replace(/\{([a-zA-Z0-9_]+)\}/g, (match, name) => Object.hasOwn(values, name) ? String(values[name]) : match);
const message = (locale, key, values) => fill(bundledCatalogs[locale][key], values);
const counted = (locale, key, count) => message(locale, key + (count === 1 ? '.one' : '.other'), { count });
const ADDED_KEYS = ['timeline.precision.day', 'timeline.precision.dayDisclosure', 'viewer.passport.evidence.unverified', 'viewer.passport.evidence.unsourced',
  'viewer.passport.evidence.localOnly', ...Object.keys(bundledCatalogs.en).filter(key => key.startsWith('viewer.passport.evidence.reason.'))];

async function reader(meta, options = {}) {
  const env = bootReader({ svg: fixtures.graph(), metadata: fixtures.metadata('architecture', meta), ...options });
  await env.settle(40);
  assert.deepEqual(env.errors, []);
  return env;
}

const visible = env => ({
  locale: env.api.locale,
  lang: env.html.getAttribute('lang'),
  instruction: env.$('[data-phoenix-i18n="viewer.lens.instruction"]').textContent,
  placeholder: env.$('#node-finder-input').getAttribute('placeholder'),
  plate: env.$('.diagram-container').getAttribute('data-preset-badge-editorial-plate'),
});
const bundled = locale => ({ locale, lang: locale, instruction: message(locale, 'viewer.lens.instruction'), placeholder: message(locale, 'viewer.finder.placeholder'), plate: message(locale, 'viewer.preset.badge.editorialPlate') });

test('the five catalogs share one key set and one placeholder set per key', () => {
  assert.deepEqual(Object.keys(bundledCatalogs), LOCALES);
  const keys = Object.keys(bundledCatalogs.en).sort();
  assert.ok(keys.length > 400);
  for (const locale of LOCALES) {
    assert.deepEqual(Object.keys(bundledCatalogs[locale]).sort(), keys, `${locale} keys`);
    for (const key of keys) {
      const value = bundledCatalogs[locale][key];
      assert.equal(typeof value, 'string', `${locale} ${key}`);
      assert.ok(value.length, `${locale} ${key} is not empty`);
      assert.deepEqual(placeholders(value), placeholders(bundledCatalogs.en[key]), `${locale} ${key} placeholders`);
      assert.doesNotMatch(value, /archify/i, `${locale} ${key} carries the Phoenix identity`);
    }
  }
});

test('the template embeds exactly the normalized catalog', () => {
  const start = templateSource.indexOf('var phoenixDiagramBundledCatalogs = ');
  assert.ok(start > 0);
  const end = templateSource.indexOf(';\n', start);
  const embedded = JSON.parse(templateSource.slice(start + 'var phoenixDiagramBundledCatalogs = '.length, end));
  assert.deepEqual(embedded, bundledCatalogs);
});

test('share-card badges and the editorial plate carry the Phoenix Diagram identity in every language', () => {
  for (const locale of LOCALES) {
    const catalog = bundledCatalogs[locale];
    assert.match(catalog['viewer.export.card.routeBadge'], /^PHOENIX DIAGRAM · .+ · \{hops\}$/, locale);
    assert.match(catalog['viewer.export.card.reachBadge'], /^PHOENIX DIAGRAM · .*\{direction\}/, locale);
    assert.match(catalog['viewer.preset.badge.editorialPlate'], /^PHOENIX DIAGRAM \/ /, locale);
  }
  assert.equal(bundledCatalogs.en['viewer.export.card.routeBadge'], 'PHOENIX DIAGRAM · ROUTE · {hops}');
  assert.equal(bundledCatalogs.en['viewer.export.card.reachBadge'], 'PHOENIX DIAGRAM · {direction} REACH');
});

test('the added timeline and source-evidence keys resolve in every language at runtime', async () => {
  assert.equal(ADDED_KEYS.filter(key => key.startsWith('viewer.passport.evidence.reason.')).length, 12);
  for (const locale of LOCALES) {
    const env = await reader({ locale });
    for (const key of [...ADDED_KEYS, 'viewer.export.downloadPngInstead']) {
      const resolved = env.window.viewerText(key);
      assert.notEqual(resolved, key, `${locale} ${key}`);
      assert.equal(resolved, bundledCatalogs[locale][key], `${locale} ${key}`);
      if (locale !== 'en' && key !== 'viewer.export.downloadPngInstead') assert.notEqual(resolved, bundledCatalogs.en[key], `${locale} ${key} is translated`);
    }
  }
});

test('meta.locale selects a bundled catalog case-insensitively and sets html lang', async () => {
  for (const [requested, locale] of [['en', 'en'], ['zh-cn', 'zh-CN'], ['ZH-TW', 'zh-TW'], ['ES', 'es'], ['Ko', 'ko']]) {
    const env = await reader({ locale: requested });
    assert.deepEqual(visible(env), bundled(locale), requested);
  }
});

test('unsupported or absent locales fall back to English', async () => {
  for (const meta of [{}, { locale: 'fr' }, { locale: 'es-MX' }, { locale: 'zh' }, { locale: '' }, { locale: 42 }, { translations: { 'viewer.lens.instruction': 'No locale, no override.' } }]) {
    const env = await reader(meta, { lang: 'xx' });
    assert.deepEqual(visible(env), bundled('en'), JSON.stringify(meta));
  }
});

test('valid meta.translations override the bundled catalog; invalid entries are ignored', async () => {
  const french = await reader({ locale: 'fr', translations: {
    'viewer.lens.instruction': 'Choisissez jusqu’à deux types.',
    'viewer.route.result.title': '{source} vers {target}',
    'viewer.route.unreachable': 'Impossible : {cible}',
    'viewer.finder.placeholder': '',
    'not.a.catalog.key': 'ignored',
  } });
  assert.deepEqual(visible(french), { ...bundled('en'), locale: 'fr', lang: 'fr', instruction: 'Choisissez jusqu’à deux types.' });
  assert.equal(french.window.viewerText('viewer.route.result.title', { source: 'A', target: 'B' }), 'A vers B');
  assert.equal(french.window.viewerText('viewer.route.unreachable', { label: 'X' }), message('en', 'viewer.route.unreachable', { label: 'X' }), 'a placeholder mismatch keeps the English message');
  assert.equal(french.window.viewerText('not.a.catalog.key'), 'not.a.catalog.key');

  const onlyInvalid = await reader({ locale: 'fr', translations: { 'viewer.route.unreachable': 'Impossible : {cible}', 'viewer.finder.placeholder': '' } });
  assert.deepEqual(visible(onlyInvalid), bundled('en'), 'without a usable override an unsupported locale falls back to English');

  const spanish = await reader({ locale: 'es', translations: { 'viewer.lens.instruction': 'Elige dos.' } });
  assert.deepEqual(visible(spanish), { ...bundled('es'), instruction: 'Elige dos.' }, 'an override wins over the bundled catalog');
});

test('route and reach share cards draw the Phoenix Diagram badge in each language', async () => {
  for (const locale of LOCALES) {
    const env = await reader({ locale });
    env.api.routeProbe.begin({ source: 'a' });
    env.api.routeProbe.choose('d');
    const route = env.api.exportMenu.downloadRouteShareCard();
    await env.settle(100);
    await route;
    const hops = counted(locale, 'viewer.export.card.hop', 3).toUpperCase();
    assert.ok(env.drawn.includes(message(locale, 'viewer.export.card.routeBadge', { hops })), `${locale} route badge`);
    env.api.routeProbe.clear();
    env.api.focus.set('a');
    env.api.focus.reach('downstream');
    const reach = env.api.exportMenu.downloadReachShareCard();
    await env.settle(100);
    await reach;
    const direction = message(locale, 'viewer.export.direction.downstream').toUpperCase();
    assert.ok(env.drawn.includes(message(locale, 'viewer.export.card.reachBadge', { direction })), `${locale} reach badge`);
    assert.deepEqual(env.drawn.filter(value => /archify/i.test(value)), []);
    assert.equal(env.downloads.length, 2);
  }
});
