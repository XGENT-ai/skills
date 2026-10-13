#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const files = [];
for (const entry of fs.readdirSync(path.join(root, 'test'), { withFileTypes: true })) {
  if (entry.isFile() && entry.name.endsWith('.test.js')) files.push(`test/${entry.name}`);
  if (entry.isDirectory()) {
    for (const name of fs.readdirSync(path.join(root, 'test', entry.name))) {
      if (name.endsWith('.test.js')) files.push(`test/${entry.name}/${name}`);
    }
  }
}

function run(command, args, env = process.env) {
  const result = spawnSync(command, args, { cwd: root, stdio: 'inherit', env });
  if (result.error) console.error(result.error.message);
  if (result.status !== 0) process.exit(result.status || 1);
}

run(process.execPath, ['--test', ...files.sort()]);
// Python tests and their subprocesses exchange UTF-8 text on every host.
const pythonEnv = { ...process.env, PYTHONUTF8: '1', PYTHONIOENCODING: 'utf-8' };
if (process.platform === 'win32') {
  // The system bash.exe may launch WSL; use the Bash shipped with native Git.
  const git = spawnSync('git', ['--exec-path'], { encoding: 'utf8' });
  if (git.error || git.status !== 0) throw git.error || new Error('Cannot locate Git Bash for Python tests.');
  const bashDir = path.resolve(git.stdout.trim(), '../../../bin');
  if (!fs.existsSync(path.join(bashDir, 'bash.exe'))) throw new Error(`Git Bash is missing from ${bashDir}.`);
  pythonEnv.PATH = [bashDir, pythonEnv.PATH].join(path.delimiter);
}
for (const dir of ['test/codex', 'test/dev-plan', 'test/skills', 'skills/agi-mode/evals']) {
  run('python3', ['-m', 'unittest', 'discover', '-s', dir, '-p', 'test_*.py'], pythonEnv);
}
