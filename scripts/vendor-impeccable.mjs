#!/usr/bin/env node
// vendor-impeccable — 把上游 impeccable 的 skill bundle 与 engine 二进制抓进
// vendor/impeccable/,供 `xgent-skills install` 安装。维护者手动执行:
//
//   node scripts/vendor-impeccable.mjs
//
// 本脚本会重写 VERSION.json,所以跑完接着跑 scripts/publish-vendor-r2.mjs:
// engine 二进制不随 npm 包分发,得传上 R2 并把下载地址写回 VERSION.json。
//
// bundle 不按原样收:上游给 19 个 harness 目录各放了一整套 skill,实测 83% 的
// 字节是同一批文件(光 font-index.json 就 1.1 MB × 19)。这里按内容去重存成
// blobs/<sha256> + manifest.json,安装时再按清单还原,展开后 38 MB → 6 MB。
//
// bundle 按上游的 ed25519 签名(scripts/bundle-signing-keys.json 的公钥)验签,
// engine 二进制按 .sha256 旁文件校验;任何一步不过就中止,不写 vendor/。
// 零依赖,需要 Node >= 18 与系统 curl、unzip。下载一律走 curl:它认
// http_proxy/https_proxy 环境变量(Node 的 fetch 不认),自带重试,
// 正是墙内抓 GitHub release 需要的。

import { spawnSync } from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const VENDOR_DIR = path.join(REPO_ROOT, 'vendor', 'impeccable');
const BUNDLE_URL = 'https://impeccable.style/api/download/bundle/universal';
const RELEASE_PREFIX = 'https://github.com/pbakaus/impeccable/releases/download';
// 只收 macOS:团队清一色 mac 开发。其它平台上 launcher 会照旧自己联网下载。
// 要加平台就往这里添上游 release 的资产名后缀(如 linux-x64),Windows 的资产
// 还带 .exe 后缀,得连带改下面的命名。
const ENGINE_TARGETS = ['darwin-arm64', 'darwin-x64'];

// 上游 crates/skills/src/bundle_signature.rs 编译进二进制的信任根,
// 内容同 impeccable 仓库的 scripts/bundle-signing-keys.json。
const TRUSTED_KEYS = {
  'release-2026-09': '7433133bb92da2c0da4186925f36dbd36219c11192451df56f25fd6a23dd7db9',
};

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

let downloadSeq = 0;

function curlTo(url, dest, what) {
  const result = spawnSync('curl', ['-fsSL', '--retry', '3', '--retry-delay', '2', '--max-time', '600', '-o', dest, url], { stdio: ['ignore', 'ignore', 'inherit'], env: proxyEnv() });
  if (result.status !== 0) die(`下载 ${what} 失败(curl 退出码 ${result.status}): ${url}`);
}

// 小文件下载后直接读进内存;大的 zip / 二进制也走同一条路,15~40 MB 无所谓。
function download(url, what) {
  const dest = path.join(staging, `download-${downloadSeq++}`);
  curlTo(url, dest, what);
  const buf = fs.readFileSync(dest);
  fs.rmSync(dest, { force: true });
  return buf;
}

// api 接口只给 302,版本号得从 Location 里读,所以这一跳不能跟随重定向。
function resolveRedirect(url) {
  const result = spawnSync('curl', ['-fsS', '--retry', '3', '--retry-delay', '2', '--max-time', '120', '-o', '/dev/null', '-w', '%{redirect_url}', url], { encoding: 'utf8', env: proxyEnv() });
  if (result.status !== 0) die(`解析 ${url} 的重定向失败(curl 退出码 ${result.status})`);
  return result.stdout.trim();
}

function sha256(buf) {
  return crypto.createHash('sha256').update(buf).digest('hex');
}

// 上游 bundle_signature.rs::verify_reader 的等价实现:签名覆盖 keyId /
// 版本 / 文件名 / 字节数 / sha256,再拿 sha256 和字节数对上真正的 zip。
function verifyBundleSignature(zip, signatureText, version) {
  let envelope;
  try {
    envelope = JSON.parse(signatureText);
  } catch {
    die('bundle 签名不是合法 JSON');
  }
  if (envelope.schema !== 1 || envelope.artifact !== 'universal.zip' || envelope.version !== version) {
    die(`bundle 签名与请求的 release 不匹配: ${JSON.stringify(envelope)}`);
  }
  const publicKeyHex = TRUSTED_KEYS[envelope.keyId];
  if (!publicKeyHex) die(`未知的 bundle 签名密钥: ${envelope.keyId}(上游轮换了密钥,需更新 TRUSTED_KEYS)`);
  // 裸 ed25519 公钥包一层 SPKI 头交给 node:crypto。
  const der = Buffer.concat([Buffer.from('302a300506032b6570032100', 'hex'), Buffer.from(publicKeyHex, 'hex')]);
  const publicKey = crypto.createPublicKey({ key: der, format: 'der', type: 'spki' });
  const payload = `impeccable-skill-bundle-v1\n${envelope.keyId}\nskill-v${envelope.version}\n${envelope.artifact}\n${envelope.size}\n${envelope.sha256}\n`;
  if (!crypto.verify(null, Buffer.from(payload), publicKey, Buffer.from(envelope.signature, 'hex'))) {
    die('bundle 签名验证失败');
  }
  if (zip.length !== envelope.size || sha256(zip) !== envelope.sha256) {
    die('bundle 的字节数或 sha256 与签名不符');
  }
  return envelope;
}

// zip 里的 unix 模式位在解压后必须留住:scripts/impeccable 是 hook 每次编辑
// 都要执行的 launcher,丢了 +x 就每次都 Permission denied。
function unzipTo(zipPath, destDir) {
  if (spawnSync('unzip', ['-v'], { stdio: 'ignore' }).status !== 0) {
    die('需要系统 unzip 命令来解压 bundle,请先安装');
  }
  const result = spawnSync('unzip', ['-q', '-o', zipPath, '-d', destDir], { stdio: 'inherit' });
  if (result.status !== 0) die('解压 bundle 失败');
}

function listFiles(dir, prefix = '') {
  const files = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const rel = prefix ? `${prefix}/${entry.name}` : entry.name;
    if (entry.isDirectory()) files.push(...listFiles(path.join(dir, entry.name), rel));
    else files.push(rel);
  }
  return files;
}

// 把解开的 bundle 转成「blobs/<sha256> 存内容 + manifest.json 记每个 harness 的
// path → sha」。各 harness 之间差的只是少数文件(路径 token 与按 harness 改写过的
// 措辞),其余都是同一份,去重后只剩一份。执行位单独记 exec,zip 里目前只有
// skills/impeccable/scripts/impeccable 带 +x。
function buildStore(bundleStage, storeDir) {
  const blobsDir = path.join(storeDir, 'blobs');
  fs.mkdirSync(blobsDir, { recursive: true });
  const providers = {};
  let total = 0;
  for (const entry of fs.readdirSync(bundleStage, { withFileTypes: true })) {
    if (!entry.isDirectory() || !entry.name.startsWith('.')) continue;
    const dir = path.join(bundleStage, entry.name);
    const files = {};
    const exec = [];
    for (const rel of listFiles(dir)) {
      const full = path.join(dir, rel);
      const buf = fs.readFileSync(full);
      const sha = sha256(buf);
      const blob = path.join(blobsDir, sha);
      if (!fs.existsSync(blob)) fs.writeFileSync(blob, buf);
      files[rel] = sha;
      if (fs.statSync(full).mode & 0o111) exec.push(rel);
      total += buf.length;
    }
    providers[entry.name] = exec.length > 0 ? { files, exec } : { files };
  }
  fs.writeFileSync(path.join(storeDir, 'manifest.json'), JSON.stringify({ schema: 1, providers }, null, 2) + '\n');
  const unique = fs.readdirSync(blobsDir).reduce((n, f) => n + fs.statSync(path.join(blobsDir, f)).size, 0);
  console.log(`  去重  ${Object.keys(providers).length} 个 harness,${(total / 1024 / 1024).toFixed(1)} MB → ${(unique / 1024 / 1024).toFixed(1)} MB`);
  return providers;
}

function ensureExecutable(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      ensureExecutable(full);
    } else if (entry.name === 'impeccable' && path.basename(dir) === 'scripts') {
      fs.chmodSync(full, 0o755);
    }
  }
}

const staging = fs.mkdtempSync(path.join(os.tmpdir(), 'xgent-vendor-impeccable-'));
try {
  // 1) 解析 release。
  const location = resolveRedirect(BUNDLE_URL);
  if (!location) die('bundle 下载地址没有返回重定向');
  const match = location.match(/^https:\/\/github\.com\/pbakaus\/impeccable\/releases\/download\/skill-v(\d+\.\d+\.\d+)\/universal\.zip$/);
  if (!match) die(`bundle 重定向到了预期之外的地址: ${location}`);
  const skillVersion = match[1];
  console.log(`bundle  skill-v${skillVersion}`);

  // 2) 下载并验签。
  const zip = download(location, 'bundle');
  const signature = download(`${location}.sig.json`, 'bundle 签名');
  const envelope = verifyBundleSignature(zip, signature.toString('utf8'), skillVersion);
  console.log(`  验签  ${envelope.keyId} / sha256 ${envelope.sha256}`);

  // 3) 解压到 staging,读出 engine 版本。
  const zipPath = path.join(staging, 'universal.zip');
  const bundleStage = path.join(staging, 'bundle');
  fs.writeFileSync(zipPath, zip);
  unzipTo(zipPath, bundleStage);
  const versionFile = path.join(bundleStage, '.claude', 'skills', 'impeccable', 'scripts', 'VERSION');
  if (!fs.existsSync(versionFile)) die('bundle 里没有 .claude/skills/impeccable/scripts/VERSION');
  const engineVersion = fs.readFileSync(versionFile, 'utf8').trim();
  console.log(`engine  v${engineVersion}`);
  ensureExecutable(bundleStage);
  const storeStage = path.join(staging, 'store');
  buildStore(bundleStage, storeStage);

  // 4) engine 二进制:逐个平台下载 + 按 .sha256 旁文件校验。
  const engineStage = path.join(staging, 'engine');
  const engines = {};
  for (const target of ENGINE_TARGETS) {
    const asset = `impeccable-${target}`;
    const url = `${RELEASE_PREFIX}/engine-v${engineVersion}/${asset}`;
    const bin = download(url, asset);
    const expected = download(`${url}.sha256`, `${asset}.sha256`).toString('utf8').trim().split(/\s+/)[0].toLowerCase();
    const actual = sha256(bin);
    if (!expected) die(`${asset} 没有 .sha256 旁文件,拒绝收录未校验的二进制`);
    if (actual !== expected) die(`${asset} 校验和不符: 期望 ${expected},实得 ${actual}`);
    const dir = path.join(engineStage, target);
    fs.mkdirSync(dir, { recursive: true });
    fs.writeFileSync(path.join(dir, 'impeccable'), bin, { mode: 0o755 });
    engines[target] = { asset, sha256: actual, size: bin.length };
    console.log(`  收录  engine/${target} (${(bin.length / 1024 / 1024).toFixed(1)} MB)`);
  }

  // 5) 上游许可与三方声明,随源码一起分发。
  const licenseText = download(`https://raw.githubusercontent.com/pbakaus/impeccable/skill-v${skillVersion}/LICENSE`, 'LICENSE');
  const noticeText = download(`https://raw.githubusercontent.com/pbakaus/impeccable/skill-v${skillVersion}/NOTICE.md`, 'NOTICE.md');

  // 6) 全部就绪后才落盘,失败不会留下半个 vendor/。
  fs.rmSync(VENDOR_DIR, { recursive: true, force: true });
  fs.mkdirSync(VENDOR_DIR, { recursive: true });
  fs.renameSync(storeStage, path.join(VENDOR_DIR, 'bundle'));
  fs.renameSync(engineStage, path.join(VENDOR_DIR, 'engine'));
  fs.writeFileSync(path.join(VENDOR_DIR, 'LICENSE'), licenseText);
  fs.writeFileSync(path.join(VENDOR_DIR, 'NOTICE.md'), noticeText);
  fs.writeFileSync(
    path.join(VENDOR_DIR, 'VERSION.json'),
    JSON.stringify(
      {
        source: 'https://github.com/pbakaus/impeccable',
        license: 'Apache-2.0',
        skillVersion,
        engineVersion,
        bundle: { url: location, keyId: envelope.keyId, sha256: envelope.sha256, size: envelope.size },
        engines,
        fetchedAt: new Date().toISOString(),
      },
      null,
      2,
    ) + '\n',
  );
  console.log(`完成: vendor/impeccable (skill ${skillVersion} / engine ${engineVersion})`);
} finally {
  fs.rmSync(staging, { recursive: true, force: true });
}
