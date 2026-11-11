#!/usr/bin/env node
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { dirname, resolve } from 'node:path';
import { renderPage, validateAssets } from './render.mjs';
import { parseDraft } from './draft.mjs';
import { interfaceText } from './templates.mjs';

const USAGE = '用法：node review.mjs <内容稿.md> <输出.html> <结果.json> [--previous <上一结果.json>] [--port <n>] [--no-open]';
const kinds = new Set(['unclear', 'question', 'suggestion']);
function fail(message, code = 1) { console.error(message); process.exit(code); }

function validateItems(items, sections, ui) {
  if (!Array.isArray(items) || !items.length) return [ui.badItems];
  const seen = new Set();
  for (const item of items) {
    if (!item || !sections.has(item.sectionId) || typeof item.blockId !== 'string'
      || !new RegExp(`^${item.sectionId}:b[1-9][0-9]*$`).test(item.blockId)
      || seen.has(item.blockId) || !kinds.has(item.kind)
      || typeof item.quote !== 'string' || !item.quote.trim()
      || (item.note !== undefined && typeof item.note !== 'string')) return [ui.badItem];
    seen.add(item.blockId);
  }
  return [];
}
function sectionNames(page) {
  return new Map([['lead', page.meta.title], ...page.sections.map((section, i) => [`s${i + 1}`, section.title])]);
}
function nextRound(previous, draft, cwd) {
  if (previous?.schemaVersion !== 1 || !Number.isSafeInteger(previous?.round) || previous.round < 1
    || typeof previous.title !== 'string' || typeof previous.draft !== 'string') throw new Error('上一结果的 schemaVersion/title/round/draft 无效。');
  validateAssets(previous.assets);
  const old = parseDraft(previous.draft, { cwd, assets: previous.assets });
  if (old.errors.length || previous.title !== old.meta.title
    || Object.keys(old.assets).some(ref => !Object.hasOwn(previous.assets, ref))
    || validateItems(previous.items, sectionNames(old), interfaceText(old.meta.lang).review).length
    || previous.items.some(item => item.section !== sectionNames(old).get(item.sectionId))) throw new Error('上一结果的内容稿、资源或标注无效。');
  const current = parseDraft(draft, { cwd, assets: previous.assets });
  const assets = { ...current.assets };
  for (const ref of Object.keys(assets)) {
    // A missing original directory must not discard a submitted resource snapshot.
    try { assets[ref] = readFileSync(resolve(cwd, ref)).toString('base64'); }
    catch { /* Keep the snapshot (including its missing marker). */ }
  }
  const page = parseDraft(draft, { cwd, assets });
  if (page.errors.length) fail(page.errors.join('\n'), 2);
  const problem = (area, message) => fail(`${area}：${message}。改法：恢复原文/原资源或先让审阅者标注；标题与结构调整须另起完整生成请求。`, 2);
  for (const field of ['template', 'title', 'lang', 'subtitle', 'source', 'progress']) {
    if (JSON.stringify(old.meta[field]) !== JSON.stringify(page.meta[field])) problem(field, '局部重讲不能改变页头');
  }
  if (JSON.stringify(old.sections.map(({ slot, title }) => [slot, title])) !== JSON.stringify(page.sections.map(({ slot, title }) => [slot, title]))) problem('sections', '局部重讲不能改变小节数量、顺序、slot 或标题');
  const fingerprint = (region, snapshot) => JSON.stringify([region.raw, region.references.map(ref => [ref, snapshot[ref] === null ? null : createHash('sha256').update(Buffer.from(snapshot[ref], 'base64')).digest('hex')])]);
  const changed = new Set();
  const before = [old.regions.lead, ...old.regions.sections];
  [page.regions.lead, ...page.regions.sections].forEach((region, i) => {
    if (fingerprint(before[i], old.assets) !== fingerprint(region, page.assets)) {
      const id = i ? `s${i}` : 'lead';
      if (!previous.items.some(item => item.sectionId === id)) problem(id, '未标注区域发生正文或资源变化');
      changed.add(id);
    }
  });
  return { round: previous.round + 1, items: previous.items.filter(item => !changed.has(item.sectionId)), assets: page.assets };
}
function send(res, status, body, type = 'application/json; charset=utf-8') {
  res.writeHead(status, { 'content-type': type, 'cache-control': 'no-store' });
  res.end(type.startsWith('application/json') ? JSON.stringify(body) : body);
}
function openBrowser(url) {
  const [command, args] = process.platform === 'darwin' ? ['open', [url]]
    : process.platform === 'win32' ? ['cmd', ['/c', 'start', '', url]] : ['xdg-open', [url]];
  const child = spawn(command, args, { detached: true, stdio: 'ignore' });
  child.on('error', () => console.error('无法自动打开浏览器，请打开 REVIEW URL。'));
  child.unref();
}
function main(args) {
  const positional = [];
  let port = 0, noOpen = false, previousPath = null;
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--no-open') noOpen = true;
    else if (args[i] === '--previous') {
      previousPath = args[++i];
      if (!previousPath || previousPath.startsWith('--')) fail(USAGE);
    } else if (args[i] === '--port') {
      const value = args[++i];
      if (!/^\d+$/.test(value || '')) fail(USAGE);
      port = Number(value);
    } else if (args[i].startsWith('--')) fail(USAGE);
    else positional.push(args[i]);
  }
  if (positional.length !== 3 || !Number.isInteger(port) || port > 65535) fail(USAGE);
  const [draftPath, outputPath, resultPath] = positional.map(file => resolve(file));
  if (!previousPath && existsSync(resultPath)) fail('结果路径已存在，请换路径或显式使用 --previous。');
  const draft = readFileSync(draftPath, 'utf8');
  const next = previousPath ? nextRound(JSON.parse(readFileSync(previousPath, 'utf8')), draft, process.cwd()) : { round: 1, items: [], assets: null };
  const { round, items, assets } = next;
  const pageKey = createHash('sha256').update(JSON.stringify([outputPath, draft])).digest('hex');
  const page = renderPage(draft, { cwd: process.cwd(), assets, review: { round, pageKey, items } });
  if (page.errors.length) fail(page.errors.join('\n'), 2);
  mkdirSync(dirname(outputPath), { recursive: true });
  writeFileSync(outputPath, page.html);
  if (page.warnings.length) console.error(page.warnings.join('\n'));
  const sections = sectionNames(page);
  const ui = interfaceText(page.meta.lang).review;
  const server = createServer((req, res) => {
    if (req.method === 'GET' && ['/', '/index.html'].includes(req.url)) return send(res, 200, page.html, 'text/html; charset=utf-8');
    if (req.method !== 'POST' || req.url !== '/submit') return send(res, 404, 'Not found', 'text/plain');
    let size = 0;
    const chunks = [];
    req.on('data', chunk => {
      size += chunk.length;
      if (size > 1024 * 1024) req.destroy();
      else chunks.push(chunk);
    });
    req.on('end', () => {
      let submission;
      try { submission = JSON.parse(Buffer.concat(chunks).toString('utf8')); }
      catch { return send(res, 400, { errors: [ui.badJSON] }); }
      if (submission?.round !== round) return send(res, 400, { errors: [ui.badRound] });
      const errors = validateItems(submission.items, sections, ui);
      if (errors.length) return send(res, 400, { errors });
      const record = {
        schemaVersion: 1, title: page.meta.title, round, submittedAt: new Date().toISOString(), draft, assets: page.assets,
        items: submission.items.map(({ blockId, sectionId, quote, kind, note }) => ({
          blockId, sectionId, section: sections.get(sectionId), quote, kind, ...(note === undefined ? {} : { note }),
        })),
      };
      try { writeFileSync(resultPath, `${JSON.stringify(record, null, 2)}\n`); }
      catch { return send(res, 500, { errors: [ui.writeError] }); }
      send(res, 200, { ok: true });
      console.log(`RESULT: ${resultPath}`);
      server.close();
      server.closeIdleConnections?.();
    });
  });
  server.on('error', error => fail(error.message));
  server.listen(port, '127.0.0.1', () => {
    const url = `http://127.0.0.1:${server.address().port}/`;
    console.log(`PAGE: ${outputPath}\nREVIEW URL: ${url}`);
    if (!noOpen) openBrowser(url);
  });
}
try { main(process.argv.slice(2)); }
catch (error) { fail(error.message); }
