#!/usr/bin/env node
// 用法：node record-demo.mjs <feature>.demo.mjs --url <已运行应用的地址>
// 视频写到演示文件旁的 <feature>.webm；任一步失败时不产出视频，只留 <feature>.failed.png。
import { existsSync, renameSync, rmSync } from 'node:fs';
import { createRequire } from 'node:module';
import { basename, dirname, join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

// style 选项在 1.59 尚不存在，且会被静默忽略，画面上会露出定位器文本。
const MIN_PLAYWRIGHT = [1, 64];
const DEFAULT_VIEWPORT = { width: 1280, height: 720 };
const CAPTION_STYLE = 'position:fixed;left:50%;transform:translateX(-50%);max-width:80%;padding:10px 18px;'
  + 'border-radius:8px;background:rgba(17,17,17,.88);color:#fff;font:600 20px/1.4 system-ui,sans-serif;text-align:center;';

function fail(message) {
  console.error(message);
  process.exit(2);
}

function parseArgs(argv) {
  const [demoPath, flag, url, ...rest] = argv;
  if (!demoPath || flag !== '--url' || !url || rest.length) {
    fail('用法：node record-demo.mjs <feature>.demo.mjs --url <已运行应用的地址>');
  }
  if (!demoPath.endsWith('.demo.mjs')) fail(`演示文件须以 .demo.mjs 结尾：${demoPath}`);
  if (!existsSync(demoPath)) fail(`找不到演示文件：${demoPath}`);
  if (!URL.canParse(url)) fail(`--url 不是合法地址：${url}`);
  return { demoPath: resolve(demoPath), url };
}

function validateDemo(demo, demoPath) {
  const problems = [];
  if (typeof demo?.title !== 'string' || !demo.title) problems.push('缺少 title');
  if (typeof demo?.path !== 'string') problems.push('缺少 path');
  if (demo?.setup !== undefined && typeof demo.setup !== 'function') problems.push('setup 须是函数');
  if (demo?.ready !== undefined && typeof demo.ready !== 'function') problems.push('ready 须是函数');
  if (demo?.viewport !== undefined && !(demo.viewport?.width > 0 && demo.viewport?.height > 0)) {
    problems.push('viewport 须有正数 width 和 height');
  }
  if (!Array.isArray(demo?.steps) || !demo.steps.length) problems.push('steps 须是非空数组');
  for (const [index, step] of (Array.isArray(demo?.steps) ? demo.steps : []).entries()) {
    const label = `步骤 ${index + 1}`;
    if (typeof step.caption !== 'string' || !step.caption) problems.push(`${label} 缺少 caption`);
    if (step.do !== undefined && typeof step.do !== 'function') problems.push(`${label} 的 do 须是函数`);
    if (typeof step.see !== 'function' && typeof step.gone !== 'function') {
      problems.push(`${label} 须用 see 或 gone 声明可观察的结果`);
    }
    if (!step.do && typeof step.see !== 'function') problems.push(`${label} 没有操作时须有 see`);
    if (step.captionAt !== undefined && step.captionAt !== 'top') problems.push(`${label} 的 captionAt 只能是 'top'`);
  }
  if (problems.length) fail(`${demoPath} 不合法：\n- ${problems.join('\n- ')}`);
}

// 演示文件常在项目子目录，从运行目录解析才能用上项目已有的 Playwright；
// 项目没有时用 PLAYWRIGHT_ROOT 指向项目外的临时安装，避免给项目加依赖。
function loadPlaywright() {
  const root = process.env.PLAYWRIGHT_ROOT ? resolve(process.env.PLAYWRIGHT_ROOT) : process.cwd();
  const require = createRequire(join(root, 'package.json'));
  for (const name of ['playwright-core', 'playwright']) {
    let playwright;
    try {
      playwright = require(name);
    } catch (error) {
      if (error.code === 'MODULE_NOT_FOUND') continue;
      throw error;
    }
    const version = require(`${name}/package.json`).version;
    const [major, minor] = version.split('.').map(Number);
    if (major > MIN_PLAYWRIGHT[0] || (major === MIN_PLAYWRIGHT[0] && minor >= MIN_PLAYWRIGHT[1])) return playwright;
    fail(`${name} ${version} 低于 ${MIN_PLAYWRIGHT.join('.')}：用 PLAYWRIGHT_ROOT 指向项目外安装的新版本`);
  }
  fail(`在 ${root} 找不到 Playwright：用 PLAYWRIGHT_ROOT 指向项目外的临时安装`);
}

const escapeHtml = text => text.replace(/[&<>]/g, char => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;' }[char]));

async function runStep(page, step, timeout) {
  const see = step.see?.(page);
  const gone = step.gone?.(page);
  // 操作前结果已成立的断言证明不了这一步做成了什么，只会让演示看起来通过。
  if (step.do) {
    if (see && await see.isVisible()) throw new Error('see 在操作前就已可见，换一个由这步操作产生的结果');
    if (gone && !await gone.isVisible()) throw new Error('gone 在操作前就不可见，换一个这步操作会移除的元素');
    await step.do(page);
  }
  await see?.waitFor({ state: 'visible', timeout });
  // isVisible 不管元素是否在视口内，不滚动的话结果可能根本没进画面。
  await see?.scrollIntoViewIfNeeded({ timeout });
  await gone?.waitFor({ state: 'hidden', timeout });
}

async function record(playwright, demo, url, outputBase) {
  const video = `${outputBase}.recording.webm`;
  const browser = await playwright.chromium.launch();
  try {
    const viewport = demo.viewport ?? DEFAULT_VIEWPORT;
    const context = await browser.newContext({ baseURL: url, viewport });
    const page = await context.newPage();
    // 登录等准备放在录制之前，密码输入不会进入不打码的视频。
    await demo.setup?.(page);
    // 等到内容就绪再开录，否则视频首帧是空白页，索引页的预览图也跟着空白。
    await page.goto(demo.path);
    await demo.ready?.(page).waitFor({ state: 'visible', timeout: 30000 });
    await page.screencast.start({ path: video, size: viewport });
    const startedAt = Date.now();
    await page.screencast.showActions({
      duration: 800,
      style: { title: 'display:none', highlight: 'outline:3px solid #e5484d;outline-offset:2px' },
    });
    await page.screencast.showChapter(demo.title, { duration: 1500 });

    for (const [index, step] of demo.steps.entries()) {
      const label = `步骤 ${index + 1}/${demo.steps.length}「${step.caption}」`;
      const edge = step.captionAt === 'top' ? 'top' : 'bottom';
      const caption = await page.screencast.showOverlay(`<div style="${CAPTION_STYLE}${edge}:28px;">${escapeHtml(step.caption)}</div>`);
      await page.waitForTimeout(Math.max(1000, step.caption.length * 80));
      try {
        await runStep(page, step, step.timeout ?? 15000);
      } catch (error) {
        await caption.dispose();
        await page.screenshot({ path: `${outputBase}.failed.png` });
        throw new Error(`${label} 失败：${error.message}`, { cause: error });
      }
      // 打印结果出现的时刻，供逐步抽帧检查。
      console.log(`${((Date.now() - startedAt) / 1000).toFixed(1)}s ok ${label}`);
      await page.waitForTimeout(step.hold ?? 1200);
      await caption.dispose();
    }
    await page.screencast.stop();
    await browser.close();
    renameSync(video, `${outputBase}.webm`);
    rmSync(`${outputBase}.failed.png`, { force: true });
    console.log(`${outputBase}.webm`);
  } catch (error) {
    await browser.close();
    rmSync(video, { force: true });
    throw error;
  }
}

const { demoPath, url } = parseArgs(process.argv.slice(2));
const { default: demo } = await import(pathToFileURL(demoPath).href);
validateDemo(demo, demoPath);
const playwright = loadPlaywright();
const outputBase = join(dirname(demoPath), basename(demoPath, '.demo.mjs'));
try {
  await record(playwright, demo, url, outputBase);
} catch (error) {
  console.error(error.message);
  process.exit(1);
}
