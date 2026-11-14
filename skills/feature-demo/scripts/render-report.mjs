#!/usr/bin/env node
// 用法：node render-report.mjs <批次目录>/report.json
// 校验模板数据并在同目录生成 index.html；字段定义见 references/report-format.md。
import { existsSync, readFileSync, realpathSync, statSync, writeFileSync } from 'node:fs';
import { dirname, extname, join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const VIDEO_EXTENSIONS = new Set(['.webm', '.mp4']);
const CHART_EXTENSIONS = new Set(['.html', '.htm']);
const MAX_FILES_PER_KIND = 20;
// 与 SPMS 模板报告的发布上限一致，生成时就拦下发布会被拒的批次。
const MiB = 1024 * 1024;
const MAX_TITLE_LENGTH = 200;
const MAX_FILE_BYTES = { 视频: 100 * MiB, 图: 25 * MiB };
const MAX_DATA_BYTES = 2 * MiB;
const MAX_TOTAL_BYTES = 500 * MiB;

const STYLE = `
  :root { color-scheme:light dark; --bg:#fafaf8; --paper:#fff; --ink:#202c29; --muted:#576761; --line:#d5ded8; --accent:#28634d; --soft:#eaf1ec; }
  @media (prefers-color-scheme:dark) { :root { --bg:#151c19; --paper:#1c2520; --ink:#ecf1ed; --muted:#adbbb2; --line:#39483e; --accent:#a4d8b8; --soft:#273c30; } }
  * { box-sizing:border-box; }
  html { scroll-padding-top:24px; }
  body { margin:0; background:var(--bg); color:var(--ink); font:16px/1.7 system-ui,"PingFang SC","Microsoft YaHei",sans-serif; overflow-wrap:anywhere; }
  ::selection { background:var(--accent); color:var(--bg); }
  main { max-width:1248px; margin:0 auto; padding:40px 40px 64px; }
  h1,h2,h3 { text-wrap:balance; line-height:1.3; }
  h1 { font-size:28px; letter-spacing:-.025em; margin:0; }
  h2 { font-size:22px; letter-spacing:-.015em; margin:0 0 12px; }
  h3 { font-size:16px; margin:0 0 12px; }
  p,ul,ol { margin:0; }
  p,li { text-wrap:pretty; }
  ul,ol { padding-inline-start:24px; }
  li + li { margin-top:10px; }
  a { color:var(--accent); text-underline-offset:4px; }
  a:hover { text-decoration-thickness:2px; }
  :focus-visible { outline:3px solid var(--accent); outline-offset:5px; }
  .intro { max-width:800px; }
  .lead { color:var(--muted); font-size:17px; margin-top:12px; max-width:38em; }
  nav { margin-top:32px; border-block:1px solid var(--line); padding:12px 0; }
  nav ul { display:flex; flex-wrap:wrap; gap:4px 24px; padding:0; list-style:none; }
  nav li + li { margin:0; }
  nav a { display:block; padding:10px 0; text-decoration:none; font-weight:600; }
  nav a:hover { text-decoration:underline; }
  article { padding:48px 0; border-bottom:1px solid var(--line); scroll-margin-top:24px; }
  .feature-summary { color:var(--muted); max-width:40em; margin-bottom:24px; }
  .feature-body { display:grid; gap:32px; align-items:start; }
  .feature-body > * { min-width:0; }
  .media { display:grid; gap:24px; }
  video,iframe { display:block; width:100%; border:1px solid var(--line); border-radius:12px; }
  video { background:#000; aspect-ratio:16/9; object-fit:contain; }
  iframe { height:480px; background:var(--paper); }
  .guide { max-width:40em; }
  .guide > div + div { margin-top:28px; }
  .guide li::marker { color:var(--accent); font-weight:600; }
  .limits { color:var(--muted); font-size:14px; }
  .note { background:var(--soft); padding:12px 16px; border-radius:8px; font-size:14px; margin-top:20px; }
  details { border-top:1px solid var(--line); padding-top:12px; }
  summary { cursor:pointer; color:var(--accent); font-weight:600; padding:8px 0; }
  details[open] summary { margin-bottom:12px; }
  .expand a { display:inline-block; padding:10px 0; font-size:14px; }
  .overview { margin-top:32px; }
  .extra { padding-top:40px; max-width:800px; }
  .extra h2 { font-size:22px; }
  code { background:var(--soft); padding:2px 5px; border-radius:4px; font-size:.9em; }
  @media (min-width:960px) { .feature-body.has-media.has-guide { grid-template-columns:minmax(0,2fr) minmax(240px,1fr); } }
  @media (max-width:600px) {
    main { padding:24px 20px 48px; }
    h1 { font-size:24px; }
    h2 { font-size:20px; }
    .lead { font-size:16px; margin-top:12px; }
    nav { margin-top:24px; }
    nav ul { gap:0 20px; }
    article { padding:32px 0; }
    .feature-body { gap:24px; }
    iframe { height:360px; }
  }
`;

function fail(lines) {
  console.error(lines.join('\n'));
  process.exit(2);
}

const isText = value => typeof value === 'string' && value.trim() !== '';
const isTextList = value => Array.isArray(value) && value.every(isText);

// 去掉开头的 ./ 后按 / 分段；不接受绝对路径、盘符、反斜杠和 .、.. 或空段，与 SPMS 的路径规则一致。
export function batchPath(value) {
  let path = value;
  while (path.startsWith('./')) path = path.slice(2);
  const segments = path.split('/');
  if (path.includes('\\') || /^[A-Za-z]:/.test(path) || segments.some(segment => segment === '' || segment === '.' || segment === '..')) return null;
  return path;
}

// 字段引用的全部媒体，按规范化后的路径去重。
export function mediaPaths(report) {
  const values = [report.overviewChart, ...report.items.flatMap(item => [item.video, item.chart])];
  return [...new Set(values.filter(value => value !== undefined).map(batchPath))];
}

export function validate(report, baseDir) {
  const problems = [];
  const counts = { 视频: 0, 图: 0 };
  const sizes = new Map();
  const checkFile = (value, label, extensions, kind) => {
    if (!isText(value)) return problems.push(`${label} 须是文件路径`);
    const path = batchPath(value);
    if (!path) return problems.push(`${label} 须是批次目录内的相对路径，不能含 .、.. 段或反斜杠：${value}`);
    if (!extensions.has(extname(path).toLowerCase())) return problems.push(`${label} 的类型不允许：${value}`);
    if (!existsSync(join(baseDir, path))) return problems.push(`${label} 找不到文件：${value}`);
    counts[kind] += 1;
    const size = statSync(join(baseDir, path)).size;
    if (size > MAX_FILE_BYTES[kind]) problems.push(`${label} 超过${kind}单文件上限 ${MAX_FILE_BYTES[kind] / MiB}MiB：${value}`);
    sizes.set(path, size);
  };

  if (report?.schemaVersion !== 1) problems.push('schemaVersion 须是 1');
  for (const field of ['batchId', 'title']) if (!isText(report?.[field])) problems.push(`缺少 ${field}`);
  if (isText(report?.title) && report.title.length > MAX_TITLE_LENGTH) problems.push(`title 不能超过 ${MAX_TITLE_LENGTH} 个字符`);
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
  const dataBytes = Buffer.byteLength(JSON.stringify(report));
  if (dataBytes > MAX_DATA_BYTES) problems.push(`report.json 的字段数据不能超过 ${MAX_DATA_BYTES / MiB}MiB`);
  // 同一文件被多处引用只计一次。
  const totalBytes = dataBytes + [...sizes.values()].reduce((sum, size) => sum + size, 0);
  if (totalBytes > MAX_TOTAL_BYTES) problems.push(`字段与全部引用文件合计不能超过 ${MAX_TOTAL_BYTES / MiB}MiB`);
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
  if (item.video) media.push(`<video src="${attr(item.video)}" aria-label="${attr(item.name)}操作演示" controls playsinline preload="metadata"></video>`);
  if (item.chart) media.push(`<details><summary>查看流程图</summary>${chart(item.chart, `${item.name} 的图`)}</details>`);
  const guide = [
    item.howTo ? `<div><h3>怎么用</h3>${list('ol', item.howTo)}</div>` : '',
    item.limits ? `<div class="limits"><h3>使用限制</h3>${list('ul', item.limits)}</div>` : '',
    item.note ? `<p class="note">${inline(item.note)}</p>` : '',
  ].filter(Boolean).join('');
  return `<article id="${attr(item.id)}">
    <h2>${inline(item.name)}</h2>
    <p class="feature-summary">${inline(item.summary)}</p>
    <div class="feature-body${media.length ? ' has-media' : ''}${guide ? ' has-guide' : ''}">
      ${media.length ? `<div class="media">${media.join('')}</div>` : ''}
      ${guide ? `<div class="guide">${guide}</div>` : ''}
    </div>
  </article>`;
}

function render(report) {
  return `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escapeHtml(report.title)}</title>
<style>${STYLE}</style>
</head>
<body>
<main>
  <header class="intro">
    <h1>${inline(report.title)}</h1>
    ${report.summary ? `<p class="lead">${inline(report.summary)}</p>` : ''}
  </header>
  <nav aria-label="功能导航"><ul>${report.items.map(item => `<li><a href="#${attr(item.id)}">${escapeHtml(item.name)}</a></li>`).join('')}</ul></nav>
  ${report.overviewChart ? `<details class="overview"><summary>查看功能总览</summary>${chart(report.overviewChart, '总览图')}</details>` : ''}
  ${report.items.map(renderItem).join('\n')}
  ${(report.sections ?? []).map(section => `<section class="extra"><h2>${inline(section.title)}</h2>${list('ul', section.items)}</section>`).join('\n')}
</main>
</body>
</html>
`;
}

// 作为模块被 publish-report.mjs 引用时不执行命令行入口。
if (process.argv[1] && import.meta.url === pathToFileURL(realpathSync(process.argv[1])).href) {
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
}
