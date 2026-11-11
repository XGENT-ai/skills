#!/usr/bin/env node
// 用法：SPMS_MCP_URL=<SPMS MCP 地址> SPMS_TOKEN=<Agent 令牌> node publish-report.mjs <批次目录>/report.json --project <项目 id>
// 把一批演示发布成 SPMS 模板报告：先按模板校验，再 report_template_begin → 逐个 PUT 引用的文件 → report_template_commit。
// 只上传 report.json 引用的视频和图；失败时不改动批次目录里的任何文件。
import { createHash } from 'node:crypto';
import { createReadStream, existsSync, readFileSync, statSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { mediaPaths, validate } from './render-report.mjs';

const CONCURRENCY = 4;

function fail(lines, code = 1) {
  console.error(lines.join('\n'));
  process.exit(code);
}

function parseArgs(argv) {
  const [reportPath, flag, projectId, ...rest] = argv;
  if (!reportPath || flag !== '--project' || !projectId || rest.length) {
    fail(['用法：node publish-report.mjs <批次目录>/report.json --project <项目 id>'], 2);
  }
  return { reportPath, projectId };
}

const sha256 = path =>
  new Promise((done, reject) => {
    const hash = createHash('sha256');
    createReadStream(path).on('data', chunk => hash.update(chunk)).on('end', () => done(hash.digest('hex'))).on('error', reject);
  });

// MCP Streamable HTTP：服务端可能直接回 JSON，也可能回一段 SSE，取最后一条 data。
async function callTool(url, token, name, args) {
  const response = await fetch(url, {
    method: 'POST',
    headers: { 'content-type': 'application/json', accept: 'application/json, text/event-stream', authorization: `Bearer ${token}` },
    body: JSON.stringify({ jsonrpc: '2.0', id: Date.now(), method: 'tools/call', params: { name, arguments: args } }),
  });
  const text = await response.text();
  const payload = text.trimStart().startsWith('{') ? text : text.split('\n').filter(line => line.startsWith('data:')).pop()?.slice(5);
  let body;
  try {
    body = JSON.parse(payload ?? '');
  } catch {
    fail([`${name} 没有返回可读的结果（HTTP ${response.status}）`]);
  }
  if (body.error) fail([`${name} 失败：${body.error.message ?? JSON.stringify(body.error)}`]);
  const result = body.result;
  const content = result?.content?.[0]?.text;
  if (result?.isError) {
    let error;
    try {
      error = JSON.parse(content);
    } catch {
      fail([`${name} 失败：${content}`]);
    }
    const lines = [`${name} 失败：${error.code ?? ''} ${error.message ?? ''}`.trim()];
    for (const item of error.details?.errors ?? []) lines.push(`- ${item.field}：${item.message}`);
    for (const path of error.details?.missing ?? []) lines.push(`- 未上传：${path}`);
    fail(lines);
  }
  return result.structuredContent ?? JSON.parse(content);
}

async function forEachLimited(items, run) {
  let next = 0;
  await Promise.all(
    Array.from({ length: Math.min(CONCURRENCY, items.length) }, async () => {
      while (next < items.length) await run(items[next++]);
    }),
  );
}

const { reportPath, projectId } = parseArgs(process.argv.slice(2));
const url = process.env.SPMS_MCP_URL;
const token = process.env.SPMS_TOKEN;
if (!url || !token) fail(['需要环境变量 SPMS_MCP_URL 和 SPMS_TOKEN（项目登记的 SPMS Agent 令牌）'], 2);
if (!existsSync(reportPath)) fail([`找不到数据文件：${reportPath}`], 2);

let report;
try {
  report = JSON.parse(readFileSync(reportPath, 'utf8'));
} catch (error) {
  fail([`${reportPath} 不是合法 JSON：${error.message}`], 2);
}
const baseDir = dirname(resolve(reportPath));
const problems = validate(report, baseDir);
if (problems.length) fail([`${reportPath} 不合法，未发布：`, ...problems.map(problem => `- ${problem}`)], 2);

const files = await Promise.all(
  mediaPaths(report).map(async path => {
    const full = join(baseDir, path);
    return { path, full, size: statSync(full).size, sha256: await sha256(full) };
  }),
);

const begin = await callTool(url, token, 'report_template_begin', {
  projectId,
  report,
  files: files.map(({ path, size, sha256: digest }) => ({ path, size, sha256: digest })),
});

const byPath = new Map(files.map(file => [file.path, file]));
const failedUploads = [];
await forEachLimited(begin.uploads, async upload => {
  const response = await fetch(upload.uploadUrl, { method: 'PUT', headers: upload.headers, body: readFileSync(byPath.get(upload.path).full) }).catch(
    error => ({ ok: false, status: error.message }),
  );
  if (!response.ok) failedUploads.push(`${upload.path}（${response.status}）`);
});
if (failedUploads.length) {
  fail([`上传失败，报告未发布：`, ...failedUploads.map(item => `- ${item}`), `上传地址 ${begin.uploadUrlTtlSeconds} 秒内有效，过期后重新运行本脚本。`]);
}

const result = await callTool(url, token, 'report_template_commit', { publishId: begin.publishId });
console.log(`报告编号：${result.key}`);
console.log(`报告链接：${result.url}`);
if (result.outcome === 'replaced') {
  console.log(`结果：替换了同批次的已有报告（原标题「${result.replaced.title}」，上次更新 ${result.replaced.updatedAt}）`);
} else {
  console.log('结果：新建');
}
