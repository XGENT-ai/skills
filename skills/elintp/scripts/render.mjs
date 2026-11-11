#!/usr/bin/env node
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseDraft, escapeHTML as esc } from './draft.mjs';
import { interfaceText } from './templates.mjs';

const ASSETS = join(dirname(fileURLToPath(import.meta.url)), '..', 'assets');
function fail(lines, code) {
  console.error(lines.join('\n'));
  process.exit(code);
}
const args = process.argv.slice(2);
if (args.length !== 2) fail(['用法：node render.mjs <内容稿.md> <输出.html>'], 1);
let text;
try { text = readFileSync(args[0], 'utf8'); }
catch (error) { fail([`无法读取内容稿：${error.message}`], 1); }
const { meta, lead, sections, errors, warnings } = parseDraft(text, { cwd: process.cwd() });
if (errors.length) fail(errors, 2);
const ui = interfaceText(meta.lang);
const outline = sections.length < 3 ? '' : `<nav class="outline" aria-label="${ui.outline}"><h2>${ui.outline}</h2><ol>${sections.map((section, i) => `<li><a href="#s${i + 1}">${esc(section.title)}</a></li>`).join('')}</ol></nav>`;
const parts = {
  LANG: esc(meta.lang), TITLE: esc(meta.title), TEMPLATE: meta.template, SKIP: ui.skip,
  HEAD: `<div class="page-meta"><span>◇ ${ui[meta.template]}</span>${meta.progress ? `<span>◷ ${ui[meta.progress]}</span>` : ''}</div><h1>${esc(meta.title)}</h1>${meta.subtitle ? `<p class="subtitle">${esc(meta.subtitle)}</p>` : ''}<button class="theme-toggle" type="button" hidden>${ui.theme}</button>`,
  LEAD: lead.join('\n'), OUTLINE: outline,
  SECTIONS: sections.map((section, i) => `<section id="s${i + 1}" data-slot="${section.slot}">${section.slot === 'more' ? '' : `<p class="slot-label">${ui[section.slot]}</p>`}<h2>${esc(section.title)}</h2>${section.blocks.join('\n')}</section>`).join('\n'),
  SOURCES: meta.source.length ? `<footer class="sources"><h2>${ui.sources}</h2>${meta.source.map(source => `<p>${esc(source)}</p>`).join('')}</footer>` : '',
};
let page;
try {
  parts.JS = readFileSync(join(ASSETS, 'page.js'), 'utf8');
  parts.CSS = readFileSync(join(ASSETS, 'page.css'), 'utf8');
  page = readFileSync(join(ASSETS, 'page.html'), 'utf8').replace(/\/\*__([A-Z]+)__\*\//g, (_, key) => parts[key]);
} catch (error) { fail([`无法读取模板资源：${error.message}`], 1); }
const output = resolve(args[1]);
try { mkdirSync(dirname(output), { recursive: true }); writeFileSync(output, page); }
catch (error) { fail([`无法写入页面：${error.message}`], 1); }
if (warnings.length) console.error(warnings.join('\n'));
console.log(`PAGE: ${output}`);
