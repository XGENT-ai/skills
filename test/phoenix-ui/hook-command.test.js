const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const nodeDir = path.dirname(process.execPath);
const bootstrapPath = path.resolve(__dirname, '../../skills/phoenix-ui/src/scripts/phoenix-bootstrap.cjs');
const { sha256, platformTarget } = require(bootstrapPath);
const hooks = import('../../tools/phoenix-ui/scripts/lib/transformers/hooks.js');
const posix = { skip: process.platform === 'win32' };
const quote = value => `'${value.replaceAll("'", "'\"'\"'")}'`;

function fixture(t) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix hook space \' " '));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const project = path.join(root, 'project \' "');
  const cache = path.join(root, 'cache');
  fs.mkdirSync(path.join(project, '.git'), { recursive: true });
  return { root, project, cache, env: {
    ...process.env,
    PATH: `${nodeDir}${path.delimiter}${process.env.PATH || ''}`,
    PHOENIX_UI_HOME: cache,
    PHOENIX_UI_PROVIDER_ID: '',
    CODEX_SESSION_ID: '',
    CLAUDE_SESSION_ID: '',
    CLAUDE_PROJECT_DIR: '',
  } };
}

function shell(f, command, input = '{}', env = {}) {
  return spawnSync('/bin/sh', ['-c', command], {
    cwd: f.project, env: { ...f.env, ...env }, input, encoding: 'utf8',
  });
}

function restoreLauncher(f, providerDir, rawProvider = 'source') {
  const scripts = path.join(f.project, providerDir, 'skills', 'phoenix-ui', 'scripts');
  fs.mkdirSync(scripts, { recursive: true });
  for (const name of ['phoenix-bootstrap.cjs', 'phoenix-ui', 'VERSION']) {
    fs.copyFileSync(path.join(path.dirname(bootstrapPath), name), path.join(scripts, name));
  }
  const launcherSource = fs.readFileSync(path.join(path.dirname(bootstrapPath), 'phoenix-launcher.cjs'), 'utf8');
  assert(launcherSource.includes("const provider = 'source';"));
  fs.writeFileSync(path.join(scripts, 'phoenix-launcher.cjs'), launcherSource.replace("const provider = 'source';", `const provider = '${rawProvider}';`));
  fs.chmodSync(path.join(scripts, 'phoenix-ui'), 0o755);
  return scripts;
}

async function commandFor(f, provider, event = '') {
  const h = await hooks;
  if (provider === 'claude') {
    f.env.CLAUDE_PROJECT_DIR = f.project;
    return h.buildClaudeSettingsManifest().hooks[event || 'PostToolUse'][0].hooks[0].command;
  }
  if (provider === 'codex') return h.buildCodexHooksManifest('.agents').hooks[event || 'PostToolUse'][0].hooks[0].command;
  if (provider === 'cursor') return h.buildCursorHooksManifest().hooks.preToolUse[0].command;
  if (provider === 'github') {
    const git = spawnSync('git', ['init', '--quiet', f.project], { env: f.env, encoding: 'utf8' });
    assert.equal(git.status, 0, git.stderr);
    return h.buildGitHubHooksManifest().hooks.postToolUse[0].bash;
  }
  if (provider === 'grok') return h.buildGrokHooksManifest().hooks[event || 'PostToolUse'][0].hooks[0].command;
  const command = h.buildGeminiHooksManifest().hooks[event || 'BeforeTool'][0].hooks[0].command;
  // Gemini substitutes an already shell-quoted project path before bash -c.
  return command.replaceAll('$GEMINI_PROJECT_DIR', quote(f.project));
}

test('existing launchers receive exact stdin, original verb and failure status', posix, async t => {
  const f = fixture(t);
  const input = Buffer.from('invalid event\nwith \' " and a NUL\0\n');
  for (const provider of ['claude', 'codex', 'cursor', 'github', 'grok', 'gemini']) {
    const command = await commandFor(f, provider);
    const providerDir = provider === 'codex' ? '.agents' : `.${provider}`;
    const launcher = path.join(f.project, providerDir, 'skills', 'phoenix-ui', 'scripts', 'phoenix-ui');
    fs.mkdirSync(path.dirname(launcher), { recursive: true });
    fs.writeFileSync(launcher, '#!/bin/sh\ncat > "$PHOENIX_UI_TEST_STDIN"\nprintf \'%s\' "$1" > "$PHOENIX_UI_TEST_ARGS"\nexit 7\n', { mode: 0o755 });
    const stdinFile = path.join(f.root, `${provider}-stdin`);
    const argsFile = path.join(f.root, `${provider}-args`);
    const result = shell(f, command, input, { PHOENIX_UI_TEST_STDIN: stdinFile, PHOENIX_UI_TEST_ARGS: argsFile });
    assert.equal(result.status, 7, `${provider}: ${result.stderr}`);
    assert.equal(result.stdout + result.stderr, '', provider);
    assert.deepEqual(fs.readFileSync(stdinFile), input, provider);
    assert.equal(fs.readFileSync(argsFile, 'utf8'), provider === 'cursor' ? 'hook-before-edit' : 'hook', provider);
  }
});

test('a hook engine that exits before reading the whole event keeps the hook result', posix, async t => {
  const f = fixture(t);
  const command = await commandFor(f, 'codex');
  f.env.PHOENIX_UI_PROVIDER_ID = 'codex';
  const scripts = restoreLauncher(f, '.agents', 'agents');
  const release = path.join(f.root, 'release');
  fs.mkdirSync(release);
  const target = platformTarget();
  const asset = `phoenix-ui-${target}`;
  // A protocol fixture engine that answers the hook without reading its input.
  const bytes = Buffer.from(`#!/usr/bin/env node\nif (process.argv[2] === 'engine-probe') { console.log('phoenix-ui-engine 0.1.0'); } else { console.log(JSON.stringify({ argv: process.argv.slice(2) })); process.exit(0); }\n`);
  const manifest = {
    schemaVersion: 1, distribution: 'development', toolVersion: '0.1.0', npmPackageVersion: '0.7.0-rc.0',
    sourceCommit: 'a'.repeat(40), bundleSchema: 1, bundleSha256: 'b'.repeat(64), reviewSchema: 1,
    engines: { [target]: { asset, sha256: sha256(bytes), size: bytes.length } },
  };
  const manifestBytes = JSON.stringify(manifest, null, 2) + '\n';
  fs.writeFileSync(path.join(scripts, 'ENGINE.json'), manifestBytes);
  fs.writeFileSync(path.join(release, 'VERSION.json'), manifestBytes);
  fs.writeFileSync(path.join(release, asset), bytes, { mode: 0o700 });
  fs.writeFileSync(path.join(release, 'THIRD-PARTY.json'), '{}\n');
  fs.writeFileSync(path.join(release, 'THIRD-PARTY-NOTICES.md'), 'Protocol fixture.\n');
  fs.mkdirSync(path.join(release, 'THIRD-PARTY-LICENSES'));
  const installCommand = `${quote(path.join(scripts, 'phoenix-ui'))} engine install --project ${quote(f.project)} --release-dir ${quote(release)} --json`;
  const install = shell(f, installCommand, '');
  assert.equal(install.status, 0, install.stderr);
  // Larger than any pipe buffer, so the write is still pending when the engine exits.
  const event = JSON.stringify({ cwd: f.project, session_id: 'large', hook_event_name: 'PostToolUse', tool_input: { content: 'x'.repeat(4 * 1024 * 1024) } });
  const result = shell(f, command, event);
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(JSON.parse(result.stdout), { argv: ['hook'] });
});
