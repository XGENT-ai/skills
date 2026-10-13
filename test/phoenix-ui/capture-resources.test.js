'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

function captureFonts(sheets, requests) {
  const source = fs.readFileSync(path.resolve(__dirname, '../../skills/phoenix-ui/src/scripts/live-browser.js'), 'utf8');
  const start = source.indexOf('  const FONT_EXT_RE');
  const end = source.indexOf('  // True if `s`', start);
  assert(start > 0 && end > start);
  const context = vm.createContext({ URL, Uint8Array, Set, Map,
    btoa: text => Buffer.from(text, 'binary').toString('base64'),
    location: { href: 'http://localhost:4000/page', origin: 'http://localhost:4000' },
    document: { styleSheets: sheets },
    fetch: async (url, options) => {
      requests.push({ url, options });
      return { ok: true, arrayBuffer: async () => Uint8Array.of(1, 2, 3).buffer, text: async () => '' };
    },
  });
  vm.runInContext(source.slice(start, end), context);
  return context;
}

function sheet(css, href = 'http://localhost:4000/styles/app.css') {
  return { href, cssRules: [{ constructor: { name: 'CSSFontFaceRule' }, cssText: css }] };
}

test('capture does not fetch inaccessible external font stylesheets', async () => {
  const requests = [], warnings = new Set();
  const context = captureFonts([{ href: 'https://fonts.example.test/css', get cssRules() { throw new Error('SecurityError'); } }], requests);
  const css = await context.collectFontCssText(warnings);
  assert.deepEqual(requests, []);
  assert(css.length > 0, 'A nonempty font override prevents the library from importing remote CSS');
  assert(warnings.size > 0);
});

test('capture omits external font files with an explicit limitation', async () => {
  const requests = [], warnings = new Set();
  const context = captureFonts([sheet('@font-face {font-family: Remote;src:url(https://fonts.example.test/a.woff2)}')], requests);
  const css = await context.collectFontCssText(warnings);
  assert.deepEqual(requests, []);
  assert(!css.includes('fonts.example.test'));
  assert(warnings.size > 0);
});

test('capture resolves project-relative fonts and embeds only verified same-origin responses', async () => {
  const requests = [], warnings = new Set();
  const context = captureFonts([sheet('@font-face {font-family: Local;src:url(../fonts/a.woff2)}')], requests);
  const css = await context.collectFontCssText(warnings);
  assert.equal(requests.length, 1);
  assert.equal(requests[0].url, 'http://localhost:4000/fonts/a.woff2');
  assert.equal(requests[0].options.redirect, 'error');
  assert(css.includes('data:font/woff2;base64,AQID'));
  assert.equal(warnings.size, 0);
});

test('capture keeps data and local font sources without making requests', async () => {
  const requests = [], warnings = new Set();
  const face = '@font-face {font-family: Local;src:local(Arial),url(data:font/woff2;base64,AQID)}';
  const context = captureFonts([sheet(face)], requests);
  const css = await context.collectFontCssText(warnings);
  assert(css.includes(face));
  assert.deepEqual(requests, []);
  assert.equal(warnings.size, 0);
});

function media(tagName, attributes, currentSrc = '') {
  return { tagName, currentSrc, getAttribute: name => attributes[name] || null };
}

test('capture excludes remote media before cloning, including alternate image and video sources', () => {
  const context = captureFonts([], []), warnings = new Set();
  for (const node of [
    media('IMG', { src: 'https://images.example.test/a.png' }),
    media('IMG', { src: '/a.png', srcset: '/a.png 1x, //images.example.test/a.png 2x' }, 'http://localhost:4000/a.png'),
    media('VIDEO', { src: '/video.mp4', poster: 'https://images.example.test/poster.png' }),
    media('IFRAME', { src: 'https://frames.example.test/' }),
    media('image', { href: 'https://images.example.test/a.svg' }),
    media('OBJECT', { data: 'https://media.example.test/part.svg' }),
    media('LINK', { href: 'https://styles.example.test/theme.css' }),
  ]) assert.equal(context.captureNodeUsesLocalResources(node, warnings), false);
  assert(warnings.size > 0);
  for (const node of [media('DIV', {}), media('IMG', { src: '/a.png' }), media('image', { href: '#icon' })]) {
    assert.equal(context.captureNodeUsesLocalResources(node, warnings), true);
  }
});

test('capture rejects a remote selected root before invoking the screenshot library', async () => {
  const context = captureFonts([], []);
  const source = fs.readFileSync(path.resolve(__dirname, '../../skills/phoenix-ui/src/scripts/live-browser.js'), 'utf8');
  const start = source.indexOf('  async function captureElementToBlob');
  const end = source.indexOf('  async function captureAndEmit', start);
  vm.runInContext(source.slice(start, end), context);
  context.remote = media('IMG', { src: 'https://images.example.test/a.png' });
  await assert.rejects(vm.runInContext('captureElementToBlob(remote, null, null)', context), /External media cannot be captured locally/);
});
