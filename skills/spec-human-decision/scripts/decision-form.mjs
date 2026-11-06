#!/usr/bin/env node
// 用法：node decision-form.mjs <form.json> <result.json> [--port <n>] [--no-open]
// 校验表单数据，在 127.0.0.1 提供确认页；用户提交后把结果写入 result.json 并退出。
// 数据与结果格式见 references/form-data.md。
import { spawn } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const ASSETS = join(dirname(fileURLToPath(import.meta.url)), '..', 'assets');
const TEMPLATES = { requirements: 'requirements-form.html', 'dev-plan': 'dev-plan-form.html' };
const DECISIONS = new Set(['confirm', 'reject', 'modify']);
const MAX_BODY = 1024 * 1024;
const USAGE = '用法：node decision-form.mjs <form.json> <result.json> [--port <n>] [--no-open]';

const isText = value => typeof value === 'string' && value.trim() !== '';
const isTextList = value => Array.isArray(value) && value.length > 0 && value.every(isText);

function fail(lines, code = 2) {
  console.error(lines.join('\n'));
  process.exit(code);
}

function checkFields(item, label, required, optionalText, problems) {
  for (const field of required) if (!isText(item[field])) problems.push(`${label} 缺少 ${field}`);
  for (const field of optionalText) {
    if (item[field] !== undefined && !isText(item[field])) problems.push(`${label}.${field} 须是非空文字`);
  }
}

function checkIdList(item, field, label, ids, problems) {
  if (item[field] === undefined) return;
  if (!isTextList(item[field])) return problems.push(`${label}.${field} 须是非空编号列表`);
  for (const ref of item[field]) {
    if (!ids.has(ref)) problems.push(`${label}.${field} 引用了不存在的条目：${ref}`);
    if (ref === item.id) problems.push(`${label}.${field} 不能引用自身`);
  }
}

function validateForm(form) {
  const problems = [];
  if (form?.schemaVersion !== 1) problems.push('schemaVersion 须是 1');
  if (!Object.hasOwn(TEMPLATES, form?.template)) problems.push('template 须是 requirements 或 dev-plan');
  if (form?.locale !== undefined && !['zh-CN', 'en'].includes(form.locale)) problems.push('locale 须是 zh-CN 或 en');
  if (!Number.isInteger(form?.round) || form.round < 1) problems.push('round 须是不小于 1 的整数');
  if (!isText(form?.title)) problems.push('缺少 title');
  if (form?.scope !== undefined && !isText(form.scope)) problems.push('scope 须是非空文字');
  if (!Array.isArray(form?.documents) || !form.documents.length) problems.push('documents 须是非空数组');
  for (const [index, doc] of (Array.isArray(form?.documents) ? form.documents : []).entries()) {
    checkFields(doc ?? {}, `documents[${index}]`, ['type', 'path', 'version'], [], problems);
  }

  const items = Array.isArray(form?.items) ? form.items : [];
  if (!items.length) problems.push('items 须是非空数组');
  const ids = new Set();
  for (const [index, item] of items.entries()) {
    if (!isText(item?.id)) continue;
    if (ids.has(item.id)) problems.push(`items[${index}].id 重复：${item.id}`);
    ids.add(item.id);
  }

  for (const [index, raw] of items.entries()) {
    const item = raw ?? {};
    const label = `items[${index}]`;
    const kind = form?.template === 'dev-plan' ? item.kind : 'feature';
    if (form?.template === 'dev-plan' && !['decision', 'feature'].includes(item.kind)) {
      problems.push(`${label}.kind 须是 decision 或 feature`);
      continue;
    }
    if (form?.template === 'requirements' && item.kind !== undefined) problems.push(`${label} 在 requirements 模板中不能有 kind`);
    checkFields(item, label, ['id', 'title', 'source'], ['impact'], problems);

    if (kind === 'decision') {
      checkFields(item, label, ['choice', 'why'], ['cost'], problems);
      checkIdList(item, 'affects', label, ids, problems);
      if (item.alternatives !== undefined) {
        const names = new Set();
        if (!Array.isArray(item.alternatives) || !item.alternatives.length) problems.push(`${label}.alternatives 须是非空数组`);
        for (const [i, alt] of (Array.isArray(item.alternatives) ? item.alternatives : []).entries()) {
          checkFields(alt ?? {}, `${label}.alternatives[${i}]`, ['name', 'tradeoff'], [], problems);
          if (names.has(alt?.name)) problems.push(`${label}.alternatives 名称重复：${alt.name}`);
          names.add(alt?.name);
        }
      }
    } else if (form?.template === 'dev-plan') {
      checkFields(item, label, ['what', 'how'], ['why', 'milestone', 'verification'], problems);
      checkIdList(item, 'dependsOn', label, ids, problems);
    } else {
      checkFields(item, label, ['what', 'why'], ['phase', 'priority'], problems);
      checkIdList(item, 'dependsOn', label, ids, problems);
      if (item.acceptance !== undefined && !isTextList(item.acceptance)) problems.push(`${label}.acceptance 须是非空文字列表`);
      if (item.details !== undefined) {
        if (!Array.isArray(item.details) || !item.details.length) problems.push(`${label}.details 须是非空数组`);
        for (const [i, detail] of (Array.isArray(item.details) ? item.details : []).entries()) {
          checkFields(detail ?? {}, `${label}.details[${i}]`, ['label', 'text'], [], problems);
        }
      }
    }

    if (item.revision !== undefined) {
      const revision = item.revision ?? {};
      if (form?.round === 1) problems.push(`${label}.revision 只用于第二轮及以后`);
      if (!['modify', 'dependency'].includes(revision.reason)) problems.push(`${label}.revision.reason 须是 modify 或 dependency`);
      checkFields(revision, `${label}.revision`, ['before', 'after'], ['note', 'cause'], problems);
    }
  }
  return problems;
}

function validateResult(result, form) {
  const problems = [];
  const answers = Array.isArray(result?.items) ? result.items : null;
  if (!answers) return ['items 须是数组'];
  const byId = new Map(form.items.map(item => [item.id, item]));
  const seen = new Set();
  for (const [index, answer] of answers.entries()) {
    const label = `items[${index}]`;
    const item = byId.get(answer?.id);
    if (!item) {
      problems.push(`${label}.id 不属于本表单：${answer?.id}`);
      continue;
    }
    if (seen.has(item.id)) problems.push(`${label}.id 重复：${item.id}`);
    seen.add(item.id);
    if (answer.decision !== null && !DECISIONS.has(answer.decision)) problems.push(`${item.id} 的 decision 无效`);
    if (answer.note !== undefined && typeof answer.note !== 'string') problems.push(`${item.id} 的 note 须是文字`);
    if (answer.alternative !== undefined) {
      const names = (item.alternatives ?? []).map(alt => alt.name);
      if (answer.decision !== 'modify' || !names.includes(answer.alternative)) problems.push(`${item.id} 的 alternative 无效`);
    }
    if (answer.decision === 'modify' && !isText(answer.note) && answer.alternative === undefined) {
      problems.push(`${item.id} 选择修改时须附说明`);
    }
  }
  for (const id of byId.keys()) if (!seen.has(id)) problems.push(`缺少条目 ${id} 的答复`);
  return problems;
}

// JSON 嵌入 <script> 时转义 <、U+2028/2029，避免提前闭合标签或破坏脚本。
const embedJson = value => JSON.stringify(value).replace(/</g, '\\u003c').replace(/\u2028/g, '\\u2028').replace(/\u2029/g, '\\u2029');

function renderForm(form) {
  const fill = (text, token, value) => text.split(token).join(value);
  let html = readFileSync(join(ASSETS, TEMPLATES[form.template]), 'utf8');
  html = fill(html, '/*__FORM_CSS__*/', readFileSync(join(ASSETS, 'form.css'), 'utf8'));
  html = fill(html, '/*__FORM_CORE__*/', readFileSync(join(ASSETS, 'form-core.js'), 'utf8'));
  return fill(html, '__FORM_DATA__', embedJson(form));
}

function openBrowser(url) {
  const [command, args] = process.platform === 'darwin' ? ['open', [url]]
    : process.platform === 'win32' ? ['cmd', ['/c', 'start', '', url]]
      : ['xdg-open', [url]];
  const child = spawn(command, args, { stdio: 'ignore', detached: true });
  child.on('error', () => console.log('无法自动打开浏览器，请手动打开上面的地址。'));
  child.unref();
}

function send(res, status, type, body) {
  res.writeHead(status, { 'content-type': type, 'cache-control': 'no-store' });
  res.end(body);
}

function main(argv) {
  const args = [...argv];
  const noOpen = args.includes('--no-open');
  const portIndex = args.indexOf('--port');
  const port = portIndex === -1 ? 0 : Number(args[portIndex + 1]);
  const positional = args.filter((arg, index) => arg !== '--no-open' && (portIndex === -1 || (index !== portIndex && index !== portIndex + 1)));
  if (positional.length !== 2 || !Number.isInteger(port) || port < 0 || port > 65535) fail([USAGE]);
  const [formPath, resultPath] = positional;

  let form;
  try {
    form = JSON.parse(readFileSync(formPath, 'utf8'));
  } catch (error) {
    fail([`无法读取表单数据 ${formPath}：${error.message}`]);
  }
  const problems = validateForm(form);
  if (problems.length) fail([`${formPath} 不合法：`, ...problems.map(problem => `- ${problem}`)]);
  const html = renderForm(form);

  const server = createServer((req, res) => {
    if (req.method === 'GET' && (req.url === '/' || req.url === '/index.html')) return send(res, 200, 'text/html; charset=utf-8', html);
    if (req.method !== 'POST' || req.url !== '/submit') return send(res, 404, 'text/plain; charset=utf-8', 'Not found');
    const chunks = [];
    let size = 0;
    req.on('data', chunk => {
      size += chunk.length;
      if (size > MAX_BODY) req.destroy();
      else chunks.push(chunk);
    });
    req.on('end', () => {
      let result;
      try {
        result = JSON.parse(Buffer.concat(chunks).toString('utf8'));
      } catch {
        return send(res, 400, 'application/json', JSON.stringify({ errors: ['提交内容不是合法 JSON'] }));
      }
      const errors = validateResult(result, form);
      if (errors.length) return send(res, 400, 'application/json', JSON.stringify({ errors }));
      const record = {
        schemaVersion: 1,
        form: { template: form.template, round: form.round, title: form.title, documents: form.documents },
        submittedAt: new Date().toISOString(),
        items: result.items,
      };
      writeFileSync(resultPath, `${JSON.stringify(record, null, 2)}\n`);
      send(res, 200, 'application/json', JSON.stringify({ ok: true }));
      console.log(`RESULT: ${resultPath}`);
      server.close();
      server.closeAllConnections?.();
    });
  });
  server.on('error', error => fail([`无法启动本地服务：${error.message}`]));
  server.listen(port, '127.0.0.1', () => {
    const url = `http://127.0.0.1:${server.address().port}/`;
    console.log(`FORM URL: ${url}`);
    if (!noOpen) openBrowser(url);
  });
}

main(process.argv.slice(2));
