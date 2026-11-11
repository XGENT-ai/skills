#!/usr/bin/env node
import { readFileSync, writeFileSync, mkdirSync, realpathSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseDraft, escapeHTML as esc } from './draft.mjs';
import { interfaceText } from './templates.mjs';

const ASSETS = join(dirname(fileURLToPath(import.meta.url)), '..', 'assets');
function fail(lines, code) {
  console.error(lines.join('\n'));
  process.exit(code);
}
export const embedJson = value => JSON.stringify(value).replace(/</g, '\\u003c').replace(/\u2028/g, '\\u2028').replace(/\u2029/g, '\\u2029');

export function validateAssets(assets) {
  if (!assets || Array.isArray(assets) || typeof assets !== 'object'
    || Object.values(assets).some(value => value !== null && (typeof value !== 'string' || Buffer.from(value, 'base64').toString('base64') !== value))) {
    throw new Error('资源快照须为原引用到 base64 字节或 null 的对象。');
  }
  return assets;
}

export function readAssetSnapshot(file) {
  const text = readFileSync(file, 'utf8');
  const data = text.trimStart().startsWith('{') ? JSON.parse(text).assets
    : JSON.parse(text.match(/<script type="application\/json" id="elintp-assets">([\s\S]*?)<\/script>/)?.[1] || 'null');
  return validateAssets(data);
}

export function renderPage(text, { cwd, assets = null, review = null }) {
  const parsed = parseDraft(text, { cwd, assets });
  const { meta, lead, sections, errors } = parsed;
  if (errors.length) return { ...parsed, html: '' };
  const ui = interfaceText(meta.lang);
  const outline = sections.length < 3 ? '' : `<nav class="outline" aria-label="${ui.outline}"><h2>${ui.outline}</h2><ol>${sections.map((section, i) => `<li><a href="#s${i + 1}">${esc(section.title)}</a></li>`).join('')}</ol></nav>`;
  const parts = {
    LANG: esc(meta.lang), TITLE: esc(meta.title), TEMPLATE: meta.template, SKIP: ui.skip,
    HEAD: `<div class="page-meta"><span>◇ ${ui[meta.template]}</span>${meta.progress ? `<span>◷ ${ui[meta.progress]}</span>` : ''}</div><h1>${esc(meta.title)}</h1>${meta.subtitle ? `<p class="subtitle">${esc(meta.subtitle)}</p>` : ''}<button class="theme-toggle" type="button" hidden>${ui.theme}</button>`,
    LEAD: lead.join('\n'), OUTLINE: outline,
    SECTIONS: sections.map((section, i) => `<section id="s${i + 1}" data-slot="${section.slot}">${section.slot === 'more' ? '' : `<p class="slot-label">${ui[section.slot]}</p>`}<h2>${esc(section.title)}</h2>${section.blocks.join('\n')}</section>`).join('\n'),
    SOURCES: meta.source.length ? `<footer class="sources"><h2>${ui.sources}</h2>${meta.source.map(source => `<p>${esc(source)}</p>`).join('')}</footer>` : '',
  };
  parts.DATA = `<script type="application/json" id="elintp-assets">${embedJson(parsed.assets)}</script>`;
  parts.REVIEWCSS = '';
  parts.REVIEWJS = '';
  if (review) {
    parts.DATA += `<script type="application/json" id="elintp-review">${embedJson({ schemaVersion: 1, title: meta.title, ...review, draft: text, ui: ui.review })}</script>`;
    parts.REVIEWCSS = `<style>${readFileSync(join(ASSETS, 'review.css'), 'utf8')}</style>`;
    parts.REVIEWJS = `<script>${readFileSync(join(ASSETS, 'review.js'), 'utf8')}</script>`;
  } else parts.DATA += `<script type="application/json" id="elintp-draft">${embedJson(text)}</script>`;
  parts.JS = readFileSync(join(ASSETS, 'page.js'), 'utf8');
  parts.CSS = readFileSync(join(ASSETS, 'page.css'), 'utf8');
  const html = readFileSync(join(ASSETS, 'page.html'), 'utf8').replace(/\/\*__([A-Z]+)__\*\//g, (_, key) => parts[key]);
  return { ...parsed, html };
}

function main(args) {
  if (args.length !== 2 && !(args.length === 4 && args[2] === '--assets-from')) fail(['用法：node render.mjs <内容稿.md> <输出.html> [--assets-from <页面.html或结果.json>]'], 1);
  try {
    const result = renderPage(readFileSync(args[0], 'utf8'), { cwd: process.cwd(), assets: args.length === 4 ? readAssetSnapshot(args[3]) : null });
    if (result.errors.length) fail(result.errors, 2);
    const output = resolve(args[1]);
    mkdirSync(dirname(output), { recursive: true });
    writeFileSync(output, result.html);
    if (result.warnings.length) console.error(result.warnings.join('\n'));
    console.log(`PAGE: ${output}`);
  } catch (error) { fail([error.message], 1); }
}
if (process.argv[1] && process.argv[1] !== '-' && realpathSync(process.argv[1]) === fileURLToPath(import.meta.url)) main(process.argv.slice(2));
