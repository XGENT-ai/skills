#!/usr/bin/env node
// 用法：node render-report.mjs <批次目录>/report.json
// 校验模板数据并在同目录生成 index.html；字段定义见 references/report-format.md。
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, extname, isAbsolute, join, normalize, resolve, sep } from 'node:path';

const VIDEO_EXTENSIONS = new Set(['.webm', '.mp4']);
const CHART_EXTENSIONS = new Set(['.html', '.htm']);
const MAX_FILES_PER_KIND = 20;

const STYLE = `
  :root { color-scheme: light dark; --bg:#f6f7f9; --paper:#fff; --ink:#1b2029; --muted:#5a6473; --line:#dbe0e7; --warn:#a15c00; }
  @media (prefers-color-scheme: dark) { :root { --bg:#14181f; --paper:#1d232c; --ink:#eef1f6; --muted:#a9b3c1; --line:#333c49; --warn:#f0b35a; } }
  * { box-sizing:border-box; }
  body { margin:0; background:var(--bg); color:var(--ink); font:16px/1.6 system-ui,sans-serif; }
  main { max-width:1280px; margin:0 auto; padding:32px 16px 64px; }
  h1 { font-size:28px; margin:0 0 8px; }
  h2 { font-size:20px; margin:0 0 8px; }
  h3 { font-size:14px; margin:16px 0 4px; color:var(--muted); }
  p, ul, ol { margin:8px 0; }
  a { color:inherit; }
  .meta { color:var(--muted); font-size:14px; }
  .meta dt { font-weight:600; }
  .meta dd { margin:0 0 4px; overflow-wrap:anywhere; }
  article, section { background:var(--paper); border:1px solid var(--line); border-radius:8px; padding:20px; margin-top:20px; }
  .media { display:grid; gap:12px; margin-top:12px; }
  .media > * { min-width:0; }
  @media (min-width:1100px) { .media.both { grid-template-columns:1fr 1fr; align-items:start; } }
  video, iframe { display:block; width:100%; border:1px solid var(--line); border-radius:6px; }
  video { background:#000; }
  iframe { height:480px; background:#fff; }
  .expand { font-size:14px; }
  .note { color:var(--warn); font-size:14px; }
  code { font-size:14px; overflow-wrap:anywhere; }
`;

function fail(lines) {
  console.error(lines.join('\n'));
  process.exit(2);
}

const isText = value => typeof value === 'string' && value.trim() !== '';
const isTextList = value => Array.isArray(value) && value.every(isText);

function validate(report, baseDir) {
  const problems = [];
  const counts = { 视频: 0, 图: 0 };
  const checkFile = (value, label, extensions, kind) => {
    if (!isText(value)) return problems.push(`${label} 须是文件路径`);
    const inside = !isAbsolute(value) && !normalize(value).startsWith(`..${sep}`) && normalize(value) !== '..';
    if (!inside) return problems.push(`${label} 须在批次目录内：${value}`);
    if (!extensions.has(extname(value).toLowerCase())) return problems.push(`${label} 的类型不允许：${value}`);
    if (!existsSync(join(baseDir, value))) return problems.push(`${label} 找不到文件：${value}`);
    counts[kind] += 1;
  };

  if (report?.schemaVersion !== 1) problems.push('schemaVersion 须是 1');
  for (const field of ['batchId', 'title']) if (!isText(report?.[field])) problems.push(`缺少 ${field}`);
  if (report?.summary !== undefined && !isText(report.summary)) problems.push('summary 须是非空文字');
  for (const field of ['source', 'baseline', 'environment']) if (!isText(report?.meta?.[field])) problems.push(`缺少 meta.${field}`);
  if (report?.overviewChart !== undefined) checkFile(report.overviewChart, 'overviewChart', CHART_EXTENSIONS, '图');

  if (!Array.isArray(report?.items) || !report.items.length) problems.push('items 须是非空数组');
  const ids = new Set();
  for (const [index, item] of (Array.isArray(report?.items) ? report.items : []).entries()) {
    const label = `items[${index}]`;
    for (const field of ['id', 'name', 'summary']) if (!isText(item?.[field])) problems.push(`${label} 缺少 ${field}`);
    if (isText(item?.id) && !/^[A-Za-z0-9_-]+$/.test(item.id)) problems.push(`${label}.id 只能用字母、数字、- 和 _`);
    if (ids.has(item?.id)) problems.push(`${label}.id 重复：${item.id}`);
    ids.add(item?.id);
    for (const field of ['howTo', 'limits']) {
      if (item?.[field] !== undefined && !(isTextList(item[field]) && item[field].length)) problems.push(`${label}.${field} 须是非空文字列表`);
    }
    if (item?.note !== undefined && !isText(item.note)) problems.push(`${label}.note 须是非空文字`);
    if (item?.video !== undefined) checkFile(item.video, `${label}.video`, VIDEO_EXTENSIONS, '视频');
    if (item?.chart !== undefined) checkFile(item.chart, `${label}.chart`, CHART_EXTENSIONS, '图');
  }

  if (report?.sections !== undefined && !Array.isArray(report.sections)) problems.push('sections 须是数组');
  for (const [index, section] of (Array.isArray(report?.sections) ? report.sections : []).entries()) {
    if (!isText(section?.title)) problems.push(`sections[${index}] 缺少 title`);
    if (!(isTextList(section?.items) && section.items.length)) problems.push(`sections[${index}].items 须是非空文字列表`);
  }
  for (const [kind, count] of Object.entries(counts)) {
    if (count > MAX_FILES_PER_KIND) problems.push(`${kind}最多 ${MAX_FILES_PER_KIND} 个，现有 ${count} 个`);
  }
  return problems;
}

const escapeHtml = text => text.replace(/[&<>"']/g, char => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[char]));
const attr = value => escapeHtml(value);

// 只支持行内代码、粗体和 http(s) 链接，其余按原文显示；SPMS 页面按同样规则显示。
function inline(text) {
  const parts = [];
  const hold = html => `\u0000${parts.push(html) - 1}\u0000`;
  const held = escapeHtml(text)
    .replace(/`([^`]+)`/g, (_, code) => hold(`<code>${code}</code>`))
    .replace(/\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)/g, (_, label, url) => hold(`<a href="${url}" rel="noreferrer">${label}</a>`))
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
  return held.replace(/\u0000(\d+)\u0000/g, (_, index) => parts[Number(index)]);
}

const list = (tag, items) => `<${tag}>${items.map(item => `<li>${inline(item)}</li>`).join('')}</${tag}>`;
const chart = (file, title) => `<div><iframe sandbox="allow-scripts" src="${attr(file)}" title="${attr(title)}"></iframe><p class="expand"><a href="${attr(file)}" target="_blank" rel="noreferrer">放大查看</a></p></div>`;

function renderItem(item) {
  const media = [];
  if (item.video) media.push(`<video src="${attr(item.video)}" controls preload="metadata"></video>`);
  if (item.chart) media.push(chart(item.chart, `${item.name} 的图`));
  return `<article id="${attr(item.id)}">
    <h2>${inline(item.name)}</h2>
    <p>${inline(item.summary)}</p>
    ${item.howTo ? `<h3>怎么用</h3>${list('ol', item.howTo)}` : ''}
    ${media.length ? `<div class="media${media.length === 2 ? ' both' : ''}">${media.join('')}</div>` : ''}
    ${item.limits ? `<h3>限制</h3>${list('ul', item.limits)}` : ''}
    ${item.note ? `<p class="note">${inline(item.note)}</p>` : ''}
  </article>`;
}

function render(report) {
  return `<!doctype html>
<html lang="zh-CN">
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escapeHtml(report.title)}</title>
<style>${STYLE}</style>
<main>
  <h1>${inline(report.title)}</h1>
  ${report.summary ? `<p>${inline(report.summary)}</p>` : ''}
  <dl class="meta">
    <dt>来源</dt><dd>${inline(report.meta.source)}</dd>
    <dt>基线</dt><dd>${inline(report.meta.baseline)}</dd>
    <dt>环境与数据</dt><dd>${inline(report.meta.environment)}</dd>
  </dl>
  <nav><ul>${report.items.map(item => `<li><a href="#${attr(item.id)}">${escapeHtml(item.name)}</a></li>`).join('')}</ul></nav>
  ${report.overviewChart ? `<section><h2>总览</h2>${chart(report.overviewChart, '总览图')}</section>` : ''}
  ${report.items.map(renderItem).join('\n')}
  ${(report.sections ?? []).map(section => `<section><h2>${inline(section.title)}</h2>${list('ul', section.items)}</section>`).join('\n')}
</main>
</html>
`;
}

const [reportPath, ...rest] = process.argv.slice(2);
if (!reportPath || rest.length) fail(['用法：node render-report.mjs <批次目录>/report.json']);
if (!existsSync(reportPath)) fail([`找不到数据文件：${reportPath}`]);
let report;
try {
  report = JSON.parse(readFileSync(reportPath, 'utf8'));
} catch (error) {
  fail([`${reportPath} 不是合法 JSON：${error.message}`]);
}
const baseDir = dirname(resolve(reportPath));
const problems = validate(report, baseDir);
if (problems.length) fail([`${reportPath} 不合法：`, ...problems.map(problem => `- ${problem}`)]);
const output = join(baseDir, 'index.html');
writeFileSync(output, render(report));
console.log(output);
