'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const root = path.resolve(__dirname, '..');
const npm = process.env.npm_execpath || path.resolve(path.dirname(process.execPath),
  process.platform === 'win32' ? 'node_modules/npm/bin/npm-cli.js' : '../lib/node_modules/npm/bin/npm-cli.js');
const npx = path.join(path.dirname(npm), 'npx-cli.js');

test('npx infers the packed installer without changing project dependencies', (t) => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'xgent npm entrypoint '));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  const env = { ...process.env, PATH: [path.dirname(process.execPath), process.env.PATH].join(path.delimiter),
    npm_config_cache: path.join(dir, 'npm-cache'), npm_config_update_notifier: 'false' };
  const run = (script, args, cwd = dir) => {
    const result = spawnSync(process.execPath, [script, ...args], { cwd, env, encoding: 'utf8', timeout: 60000 });
    assert.equal(result.status, 0, result.stderr || result.error?.message);
    return result;
  };
  const packed = JSON.parse(run(npm, ['pack', root, '--ignore-scripts', '--offline', '--json', '--pack-destination', dir]).stdout);
  const archive = `file:${path.join(dir, packed[0].filename)}`;
  const project = path.join(dir, 'project');
  fs.mkdirSync(project);
  const packageJson = '{"name":"consumer","version":"1.0.0","private":true}\n';
  const lock = '{"name":"consumer","version":"1.0.0","lockfileVersion":3,"packages":{}}\n';
  fs.writeFileSync(path.join(project, 'package.json'), packageJson);
  fs.writeFileSync(path.join(project, 'package-lock.json'), lock);
  const installed = run(npx, ['--offline', '--yes', archive, 'install', '--no-phoenix-ui', '--no-xgent-init'], project);
  assert.match(installed.stdout, /完成: /);
  assert.equal(fs.readFileSync(path.join(project, '.claude/hooks/xgent-statusline.js'), 'utf8'),
    fs.readFileSync(path.join(root, '.claude/hooks/xgent-statusline.js'), 'utf8'));
  const explicit = run(npx, ['--offline', '--yes', `--package=${archive}`, 'xgent-skills', '--help'], project);
  assert.match(explicit.stdout, /用法: npx @xgent-ai\/skills/);
  const runtime = run(npx, ['--offline', '--yes', `--package=${archive}`, 'node', '-p', 'process.execPath'], project);
  assert.equal(fs.realpathSync(runtime.stdout.trim()), fs.realpathSync(process.execPath));
  assert.equal(fs.readFileSync(path.join(project, 'package.json'), 'utf8'), packageJson);
  assert.equal(fs.readFileSync(path.join(project, 'package-lock.json'), 'utf8'), lock);
  assert.equal(fs.existsSync(path.join(project, 'node_modules')), false);
});
