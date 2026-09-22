#!/usr/bin/env node
// publish-vendor-r2 — 把 vendor/impeccable/engine 下的二进制传到 Cloudflare R2,
// 并把公共下载地址回写进 vendor/impeccable/VERSION.json。维护者在跑完
// scripts/vendor-impeccable.mjs 之后执行(vendor 脚本会重写 VERSION.json,
// 顺序反了就把 url 冲掉了):
//
//   node scripts/publish-vendor-r2.mjs
//
// engine 二进制不随 npm 包分发(见 package.json 的 files),`xgent-skills install`
// 按 VERSION.json 里的 url + sha256 去 R2 取,所以这一步是发版的必经环节。
// 凭据放在仓库根的 .env.cf(已 gitignore),需要系统 curl 与 aws CLI。

import { spawnSync } from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const ENV_FILE = path.join(REPO_ROOT, '.env.cf');
const VENDOR_DIR = path.join(REPO_ROOT, 'vendor', 'impeccable');
const ENGINE_DIR = path.join(VENDOR_DIR, 'engine');

function die(message) {
  console.error(`错误: ${message}`);
  process.exit(1);
}

// 用户设了代理就必须走代理,不能绕过去直连。curl 按 scheme 分:http_proxy 只管
// http://,而我们的地址都是 https://,所以只设了 HTTP_PROXY 的机器上它会直连 ——
// 那不是用户的本意。这里把它补成 https_proxy 交给 curl。已经设了 https/all 的
// 就原样不动,no_proxy 照旧由 curl 自己判。
function proxyEnv() {
  const env = process.env;
  if (env.https_proxy || env.HTTPS_PROXY || env.all_proxy || env.ALL_PROXY) return env;
  const http = env.http_proxy || env.HTTP_PROXY;
  return http ? { ...env, https_proxy: http } : env;
}

// .env.cf 是给 shell `source` 用的,这里只认 `export KEY=VALUE` 一种写法,
// 值两边的引号与行尾空白都去掉。
function readEnvFile(file) {
  if (!fs.existsSync(file)) die(`没有 ${path.relative(REPO_ROOT, file)},R2 凭据从哪来?`);
  const env = {};
  for (const line of fs.readFileSync(file, 'utf8').split('\n')) {
    const match = line.match(/^\s*(?:export\s+)?([A-Z0-9_]+)\s*=\s*(.*)$/);
    if (!match) continue;
    env[match[1]] = match[2].trim().replace(/^(['"])(.*)\1$/, '$2');
  }
  return env;
}

function requireVar(env, name) {
  const value = env[name];
  if (!value) die(`.env.cf 里缺 ${name}`);
  return value;
}

function sha256(buf) {
  return crypto.createHash('sha256').update(buf).digest('hex');
}

const env = readEnvFile(ENV_FILE);
const bucket = requireVar(env, 'AGENT_RELEASE_R2_BUCKET');
const endpoint = requireVar(env, 'AGENT_RELEASE_R2_ENDPOINT');
const publicBase = requireVar(env, 'AGENT_RELEASE_R2_PUBLIC_BASE').replace(/\/+$/, '');
const prefix = (env.AGENT_RELEASE_R2_PREFIX || '').replace(/^\/+/, '').replace(/\/*$/, '/');

if (spawnSync('aws', ['--version'], { stdio: 'ignore' }).status !== 0) {
  die('需要 aws CLI 来传 R2,请先安装(brew install awscli)');
}

const awsEnv = {
  // botocore 和 curl 一样按 scheme 认代理,所以这里也从 proxyEnv() 起底。
  ...proxyEnv(),
  AWS_ACCESS_KEY_ID: requireVar(env, 'AGENT_RELEASE_R2_ACCESS_KEY'),
  AWS_SECRET_ACCESS_KEY: requireVar(env, 'AGENT_RELEASE_R2_SECRET_KEY'),
  AWS_DEFAULT_REGION: 'auto',
  // aws CLI v2.23+ 默认给每个上传加 CRC32 校验头,R2 上会 400;要求它只在
  // 必要时算校验和。老版本不认这个变量,忽略掉也不影响。
  AWS_REQUEST_CHECKSUM_CALCULATION: 'when_required',
};

const versionFile = path.join(VENDOR_DIR, 'VERSION.json');
if (!fs.existsSync(versionFile)) die('没有 vendor/impeccable/VERSION.json,先跑 node scripts/vendor-impeccable.mjs');
const meta = JSON.parse(fs.readFileSync(versionFile, 'utf8'));

for (const [target, engine] of Object.entries(meta.engines)) {
  const file = path.join(ENGINE_DIR, target, 'impeccable');
  if (!fs.existsSync(file)) die(`VERSION.json 登记了 ${target},但 vendor/impeccable/engine/${target}/impeccable 不在,先跑 node scripts/vendor-impeccable.mjs`);
  const bin = fs.readFileSync(file);
  // 传之前对一遍 VERSION.json 的校验和:vendor 目录是手改过的也能在这一步拦住。
  if (sha256(bin) !== engine.sha256) die(`${target} 的 sha256 与 VERSION.json 不符,拒绝上传`);

  const key = `${prefix}impeccable/engine/v${meta.engineVersion}/${target}/impeccable`;
  const upload = spawnSync(
    'aws',
    ['s3', 'cp', file, `s3://${bucket}/${key}`, '--endpoint-url', endpoint, '--content-type', 'application/octet-stream', '--only-show-errors'],
    { stdio: ['ignore', 'inherit', 'inherit'], env: awsEnv },
  );
  if (upload.status !== 0) die(`上传 ${target} 失败(aws 退出码 ${upload.status})`);

  // 回读公共地址:既验证 PUBLIC_BASE + 前缀拼对了,也确认桶确实是公开的
  // —— 装机端只会走这条路,这里不试一次就等着用户报 404。
  const url = `${publicBase}/${key}`;
  const check = spawnSync('curl', ['-fsSL', '--retry', '3', '--retry-delay', '2', '--max-time', '600', url], { maxBuffer: 1 << 30, encoding: 'buffer', env: proxyEnv() });
  if (check.status !== 0) die(`回读 ${url} 失败(curl 退出码 ${check.status})`);
  if (sha256(check.stdout) !== engine.sha256) die(`回读 ${url} 的 sha256 与 VERSION.json 不符`);

  engine.url = url;
  console.log(`  上传  engine/${target} (${(bin.length / 1024 / 1024).toFixed(1)} MB) → ${url}`);
}

meta.publishedAt = new Date().toISOString();
fs.writeFileSync(versionFile, JSON.stringify(meta, null, 2) + '\n');
console.log(`完成: engine v${meta.engineVersion} 已上架,下载地址写进 vendor/impeccable/VERSION.json`);
