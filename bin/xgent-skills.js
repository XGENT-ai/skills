#!/usr/bin/env node
// xgent-skills — 为目标项目安装 XGENT 的 Claude Code hooks 并启用相关 settings。
// 零依赖,Node >= 18。

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const PKG_ROOT = path.join(__dirname, '..');
const HOOKS_SRC_DIR = path.join(PKG_ROOT, '.claude', 'hooks');

// 写入目标项目 .claude/settings.json 的配置,按顶层 key 合并:
// 这里列出的 key 以本包为准覆盖,其余已有配置保持不动。
const MANAGED_SETTINGS = {
  statusLine: {
    type: 'command',
    command: 'node "$CLAUDE_PROJECT_DIR/.claude/hooks/xgent-statusline.js"',
    padding: 0,
  },
};

function usage() {
  console.log(`用法: npx @xgent-ai/skills <command>

命令:
  install [dir]   为目标项目(默认当前目录)安装 .claude/hooks 下的全部
                  hook,并在 .claude/settings.json 中启用对应配置
  help            显示本帮助
`);
}

function install(dirArg) {
  const targetDir = path.resolve(dirArg || process.cwd());
  if (!fs.existsSync(targetDir) || !fs.statSync(targetDir).isDirectory()) {
    console.error(`错误: 目标目录不存在: ${targetDir}`);
    process.exit(1);
  }

  const hooksDestDir = path.join(targetDir, '.claude', 'hooks');
  fs.mkdirSync(hooksDestDir, { recursive: true });
  for (const name of fs.readdirSync(HOOKS_SRC_DIR)) {
    const content = fs.readFileSync(path.join(HOOKS_SRC_DIR, name));
    const dest = path.join(hooksDestDir, name);
    let status = '安装';
    if (fs.existsSync(dest)) {
      status = content.equals(fs.readFileSync(dest)) ? '未变' : '更新';
    }
    if (status !== '未变') {
      fs.writeFileSync(dest, content, { mode: 0o755 });
    }
    console.log(`  ${status}  .claude/hooks/${name}`);
  }

  const settingsPath = path.join(targetDir, '.claude', 'settings.json');
  let settings = {};
  if (fs.existsSync(settingsPath)) {
    try {
      settings = JSON.parse(fs.readFileSync(settingsPath, 'utf8'));
    } catch {
      console.error(`错误: ${settingsPath} 不是合法 JSON,未修改该文件,请先手动修复。`);
      process.exit(1);
    }
  }
  const before = JSON.stringify(settings);
  Object.assign(settings, MANAGED_SETTINGS);
  if (JSON.stringify(settings) === before) {
    console.log('  未变  .claude/settings.json');
  } else {
    fs.writeFileSync(settingsPath, JSON.stringify(settings, null, 2) + '\n');
    console.log('  写入  .claude/settings.json (statusLine)');
  }

  console.log('  提示  项目文档由 xgent-init skill 生成(npx skills add XGENT-ai/skills --skill xgent-init)');
  console.log(`完成: ${targetDir}`);
}

const [cmd, ...args] = process.argv.slice(2);
switch (cmd) {
  case 'install':
    install(args[0]);
    break;
  case undefined:
  case 'help':
  case '--help':
  case '-h':
    usage();
    break;
  default:
    console.error(`未知命令: ${cmd}\n`);
    usage();
    process.exit(1);
}
