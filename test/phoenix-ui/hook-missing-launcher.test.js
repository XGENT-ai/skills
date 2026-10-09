const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const crypto = require('node:crypto');
const { spawn, spawnSync } = require('node:child_process');

const nodeDir = path.dirname(process.execPath);
const sentinelPath = path.resolve(__dirname, '../../tools/phoenix-ui/crates/hook/src/missing_launcher.cjs');
const sentinel = fs.readFileSync(sentinelPath, 'utf8').split(/\r?\n/).map(line => line.trim()).join(' ').trim();
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

function markerFile(f, provider, session = '', version = '0.1.0', project = fs.realpathSync(f.project)) {
  const key = crypto.createHash('sha256').update(JSON.stringify([project, version, provider, session])).digest('hex');
  return path.join(f.cache, 'unavailable', `${key}.json`);
}

function records(f) {
  const dir = path.join(f.cache, 'unavailable');
  return fs.existsSync(dir) ? fs.readdirSync(dir).map(name => JSON.parse(fs.readFileSync(path.join(dir, name), 'utf8'))) : [];
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

function context(result, provider) {
  const json = JSON.parse(result.stdout);
  if (provider === 'cursor') {
    assert.deepEqual(Object.keys(json), ['additional_context']);
    return json.additional_context;
  }
  if (provider === 'github') {
    assert.deepEqual(Object.keys(json), ['additionalContext']);
    return json.additionalContext;
  }
  assert.deepEqual(Object.keys(json), ['hookSpecificOutput']);
  assert.deepEqual(Object.keys(json.hookSpecificOutput), ['hookEventName', 'additionalContext']);
  return json.hookSpecificOutput.additionalContext;
}

test('entire launcher directory missing allows edit and reminds once', posix, async t => {
  const f = fixture(t);
  const command = await commandFor(f, 'codex');
  assert.equal(fs.existsSync(path.join(f.project, '.agents')), false);
  const event = JSON.stringify({ hook_event_name: 'PostToolUse', session_id: 'one' });
  const first = shell(f, command, event);
  assert.equal(first.status, 0, first.stderr);
  assert.match(first.stdout, /Phoenix UI unavailable:/);
  assert.match(first.stdout, /Reinstall the pinned @xgent-ai\/skills package/);
  const marker = markerFile(f, 'codex', 'one');
  assert.equal(fs.existsSync(marker), true);
  assert.equal(fs.statSync(marker).mode & 0o777, 0o600);
  const second = shell(f, command, event);
  assert.equal(second.status, 0, second.stderr);
  assert.equal(second.stdout, '');
  assert.equal(second.stderr, '');
});

test('all project providers preserve their nonblocking response shape and quoted paths', posix, async t => {
  const f = fixture(t);
  for (const provider of ['claude', 'codex', 'cursor', 'github', 'grok', 'gemini']) {
    const command = await commandFor(f, provider);
    const event = JSON.stringify({ hookEventName: 'PostToolUse', session_id: 'shared', cwd: f.project });
    const first = shell(f, command, event);
    assert.equal(first.status, 0, `${provider}: ${first.stderr}`);
    assert.match(context(first, provider), /Reinstall the pinned @xgent-ai\/skills package/);
    assert.equal(first.stderr, '', provider);
    assert.equal(fs.existsSync(markerFile(f, provider, 'shared')), true, provider);
    const repeated = shell(f, command, event);
    assert.equal(repeated.status, 0, repeated.stderr);
    assert.equal(repeated.stdout + repeated.stderr, '', provider);
  }
  assert.equal(records(f).length, 6);
  assert.equal(fs.existsSync(path.join(f.project, '.phoenix-ui')), false);
  assert.equal(fs.existsSync(path.join(f.project, '.impeccable')), false);
});

test('session aliases, precedence, environment provider and no-session dedup agree with bootstrap', posix, async t => {
  const f = fixture(t);
  const command = await commandFor(f, 'codex');
  const event = { hook_event_name: 'PostToolUse' };
  for (const [extra, env] of [
    [{ session_id: 'same' }, {}],
    [{ sessionId: 'same' }, {}],
    [{}, { CODEX_SESSION_ID: 'same' }],
    [{}, { CLAUDE_SESSION_ID: 'same' }],
  ]) {
    const result = shell(f, command, JSON.stringify({ ...event, ...extra }), env);
    assert.equal(result.status, 0, result.stderr);
  }
  assert.equal(records(f).length, 1);
  const prioritized = shell(f, command, JSON.stringify({ ...event, session_id: 'primary', sessionId: 'secondary' }), {
    CODEX_SESSION_ID: 'env-codex', CLAUDE_SESSION_ID: 'env-claude', PHOENIX_UI_PROVIDER_ID: 'cursor',
  });
  assert.equal(prioritized.status, 0, prioritized.stderr);
  assert.match(context(prioritized, 'cursor'), /unavailable/);
  assert.equal(fs.existsSync(markerFile(f, 'cursor', 'primary')), true);
  const noSession = shell(f, command, '{}');
  assert.equal(noSession.status, 0, noSession.stderr);
  assert.match(context(noSession, 'codex'), /unavailable/);
  assert.equal(shell(f, command, '{}').stdout, '');
  assert.equal(fs.existsSync(markerFile(f, 'codex')), true);
});

test('Stop uses stderr only, shares the session key, and never blocks', posix, async t => {
  const f = fixture(t);
  for (const provider of ['claude', 'codex', 'grok']) {
    const command = await commandFor(f, provider, 'Stop');
    const first = shell(f, command, JSON.stringify({ hookEventName: 'sToP', sessionId: 'stop' }));
    assert.equal(first.status, 0, first.stderr);
    assert.equal(first.stdout, '');
    assert.match(first.stderr, /Reinstall the pinned @xgent-ai\/skills package/);
    const edit = shell(f, command, JSON.stringify({ hook_event_name: 'PostToolUse', session_id: 'stop' }));
    assert.equal(edit.status, 0, edit.stderr);
    assert.equal(edit.stdout + edit.stderr, '');
  }
});

test('invalid, primitive, empty and incorrectly typed stdin remain nonblocking', posix, async t => {
  for (const input of ['', 'not-json', 'null', '[]', '42', '"text"', '{"hook_event_name":42}', '{"hook_event_name":{},"hookEventName":"BeforeTool"}']) {
    const f = fixture(t);
    const command = await commandFor(f, 'codex');
    const first = shell(f, command, input);
    assert.equal(first.status, 0, `${input}: ${first.stderr}`);
    assert.match(context(first, 'codex'), /unavailable/);
    assert.equal(shell(f, command, input).stdout, '');
  }
});

test('realpath and repository traversal dedup symlink aliases and nested cwd on macOS', posix, async t => {
  const f = fixture(t);
  const nested = path.join(f.project, 'src', 'nested');
  fs.mkdirSync(nested, { recursive: true });
  const alias = path.join(f.root, 'alias');
  fs.symlinkSync(f.project, alias);
  const command = await commandFor(f, 'codex');
  const first = shell(f, command, JSON.stringify({ cwd: path.join(alias, 'src', 'nested'), session_id: 'path' }));
  assert.equal(first.status, 0, first.stderr);
  assert.match(first.stdout, /unavailable/);
  const same = shell(f, command, JSON.stringify({ cwd: f.project, session_id: 'path' }));
  assert.equal(same.status, 0, same.stderr);
  assert.equal(same.stdout, '');
  assert.deepEqual(records(f).map(record => record.project), [fs.realpathSync(f.project)]);
});

test('missing cwd uses the original cwd and pinned version, matching bootstrap failure', posix, async t => {
  const f = fixture(t);
  const command = await commandFor(f, 'codex');
  const result = shell(f, command, JSON.stringify({ cwd: path.join(f.project, 'deleted'), session_id: 'cwd' }));
  assert.equal(result.status, 0, result.stderr);
  assert.equal(fs.existsSync(markerFile(f, 'codex', 'cwd')), true);
  assert.equal(records(f)[0].version, '0.1.0');
});

test('project and version are independent dedup dimensions without writing project state', posix, async t => {
  const f = fixture(t);
  const state = path.join(f.project, '.phoenix-ui', 'config.json');
  const legacy = path.join(f.project, '.impeccable', 'config.json');
  for (const file of [state, legacy]) {
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, 'user-owned bytes\n');
  }
  const other = path.join(f.root, 'other');
  fs.mkdirSync(other);
  const command = version => `node -e ${quote(sentinel)} -- ${quote(version)} 'codex'`;
  for (const [version, cwd] of [['0.1.0', f.project], ['0.2.0', f.project], ['0.1.0', other]]) {
    const result = shell(f, command(version), JSON.stringify({ cwd, session_id: 'version' }));
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /unavailable/);
  }
  assert.equal(records(f).length, 3);
  for (const file of [state, legacy]) assert.equal(fs.readFileSync(file, 'utf8'), 'user-owned bytes\n');
});

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
  assert.equal(fs.existsSync(f.cache), false, 'guard must not consume stdin or create a cache for an existing launcher');
});

test('cache write failure still permits events; persistent dedup is unavailable', posix, async t => {
  const f = fixture(t);
  fs.writeFileSync(f.cache, 'not a directory');
  const command = await commandFor(f, 'codex');
  for (let i = 0; i < 2; i++) {
    const result = shell(f, command, '{"session_id":"read-only"}');
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /unavailable/);
  }
  assert.equal(fs.readFileSync(f.cache, 'utf8'), 'not a directory');
});

test('missing Node in the missing-launcher branch cannot block an edit', posix, async t => {
  const f = fixture(t);
  const command = await commandFor(f, 'codex');
  const result = shell(f, command, '{}', { PATH: path.join(f.root, 'no-executables') });
  assert.equal(result.status, 0, result.stderr);
  assert.equal(fs.existsSync(f.cache), false);
});

test('concurrent events create exactly one reminder in the shared cache', posix, async t => {
  const f = fixture(t);
  const command = await commandFor(f, 'codex');
  const results = await Promise.all(Array.from({ length: 6 }, () => new Promise((resolve, reject) => {
    const child = spawn('/bin/sh', ['-c', command], { cwd: f.project, env: f.env, stdio: ['pipe', 'pipe', 'pipe'] });
    let stdout = '', stderr = '';
    child.stdout.on('data', chunk => { stdout += chunk; });
    child.stderr.on('data', chunk => { stderr += chunk; });
    child.once('error', reject);
    child.once('close', status => resolve({ status, stdout, stderr }));
    child.stdin.end('{"session_id":"concurrent"}');
  })));
  for (const result of results) assert.equal(result.status, 0, result.stderr);
  assert.equal(results.filter(result => result.stdout.includes('unavailable')).length, 1);
  assert.equal(records(f).length, 1);
});

test('guard and restored bootstrap share markers; verified installation clears them', posix, async t => {
  const f = fixture(t);
  const command = await commandFor(f, 'codex');
  f.env.PHOENIX_UI_PROVIDER_ID = 'codex';
  const event = JSON.stringify({ cwd: f.project, session_id: 'restore', hook_event_name: 'PostToolUse' });
  const missing = shell(f, command, event);
  assert.equal(missing.status, 0, missing.stderr);
  assert.match(missing.stdout, /Reinstall the pinned @xgent-ai\/skills package/);
  const scripts = restoreLauncher(f, '.agents', 'agents');
  const release = path.join(f.root, 'release');
  fs.mkdirSync(release);
  const target = platformTarget();
  const asset = `phoenix-ui-${target}`;
  // This executable verifies the bootstrap protocol, not a native Rust build.
  const bytes = Buffer.from(`#!/usr/bin/env node\nif (process.argv[2] === 'engine-probe') { console.log('phoenix-ui-engine 0.1.0'); } else { console.log(JSON.stringify({ argv: process.argv.slice(2), provider: process.env.PHOENIX_UI_PROVIDER_ID })); }\n`);
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
  const absentEngine = shell(f, command, event);
  assert.equal(absentEngine.status, 0, absentEngine.stderr);
  assert.equal(absentEngine.stdout + absentEngine.stderr, '', 'restored bootstrap must see the outer guard marker');
  assert.equal(records(f).length, 1, 'no competing dedup directory/key');
  const missingCwd = JSON.stringify({ cwd: path.join(f.project, 'deleted'), session_id: 'deleted' });
  const bootstrapFailure = shell(f, command, missingCwd);
  assert.equal(bootstrapFailure.status, 0, bootstrapFailure.stderr);
  assert.equal(fs.existsSync(markerFile(f, 'codex', 'deleted')), true, 'bootstrap pins version before project resolution');
  const unrelated = markerFile(f, 'codex', 'keep', '0.2.0');
  fs.writeFileSync(unrelated, JSON.stringify({ project: fs.realpathSync(f.project), version: '0.2.0', provider: 'codex' }));
  const installCommand = `${quote(path.join(scripts, 'phoenix-ui'))} engine install --project ${quote(f.project)} --release-dir ${quote(release)} --json`;
  const install = shell(f, installCommand, '');
  assert.equal(install.status, 0, install.stderr);
  assert.equal(JSON.parse(install.stdout).status, 'installed');
  assert.equal(fs.existsSync(markerFile(f, 'codex', 'restore')), false);
  assert.equal(fs.existsSync(markerFile(f, 'codex', 'deleted')), false);
  assert.equal(fs.existsSync(unrelated), true, 'installation preserves other versions');
  assert.equal(fs.existsSync(path.join(f.project, '.phoenix-ui')), false, 'engine install must not write project configuration');
  const ready = shell(f, command, event);
  assert.equal(ready.status, 0, ready.stderr);
  assert.deepEqual(JSON.parse(ready.stdout), { argv: ['hook'], provider: 'codex' });
  fs.rmSync(path.dirname(scripts), { recursive: true });
  const missingAgain = shell(f, command, event);
  assert.equal(missingAgain.status, 0, missingAgain.stderr);
  assert.match(missingAgain.stdout, /Reinstall the pinned @xgent-ai\/skills package/);
  assert.equal(shell(f, command, event).stdout, '');
});

test('restored launcher deduplicates the guard before a valid engine manifest is available', posix, async t => {
  for (const [provider, rawProvider, providerDir] of [
    ['claude', 'claude-code', '.claude'], ['codex', 'agents', '.agents'],
  ]) {
    const f = fixture(t);
    const nested = path.join(f.project, 'src', 'nested');
    fs.mkdirSync(nested, { recursive: true });
    const command = await commandFor(f, provider);
    const event = JSON.stringify({ cwd: nested, session_id: 'partial-restore' });
    const missing = shell(f, command, event);
    assert.equal(missing.status, 0, missing.stderr);
    assert.match(missing.stdout, /unavailable/);
    const scripts = restoreLauncher(f, providerDir, rawProvider);
    for (const bytes of [null, '{broken manifest']) {
      if (bytes !== null) fs.writeFileSync(path.join(scripts, 'ENGINE.json'), bytes);
      const restored = shell(f, command, event);
      assert.equal(restored.status, 0, restored.stderr);
      assert.equal(restored.stdout + restored.stderr, '', 'partial restoration must reuse the pinned project/provider/session key');
      assert.deepEqual(records(f), [{ project: fs.realpathSync(f.project), version: '0.1.0', provider }]);
    }
  }
});

for (const [provider, rawProvider, providerDir] of [
  ['claude', 'claude-code', '.claude'], ['codex', 'agents', '.agents'],
]) {
  test(`${rawProvider} restored launcher deduplicates the original ${provider} guard reminder`, posix, async t => {
    const f = fixture(t);
    const command = await commandFor(f, provider);
    const event = JSON.stringify({ cwd: f.project, session_id: 'alias' });
    const missing = shell(f, command, event);
    assert.equal(missing.status, 0, missing.stderr);
    assert.match(missing.stdout, /unavailable/);
    const scripts = restoreLauncher(f, providerDir, rawProvider);
    const target = platformTarget();
    fs.writeFileSync(path.join(scripts, 'ENGINE.json'), JSON.stringify({
      schemaVersion: 1, distribution: 'development', toolVersion: '0.1.0', npmPackageVersion: '0.7.0-rc.0',
      sourceCommit: 'a'.repeat(40), bundleSchema: 1, bundleSha256: 'b'.repeat(64), reviewSchema: 1,
      engines: { [target]: { asset: `phoenix-ui-${target}`, sha256: 'd'.repeat(64), size: 1 } },
    }));
    const restored = shell(f, command, event);
    assert.equal(restored.status, 0, restored.stderr);
    assert.equal(restored.stdout + restored.stderr, '', 'provider alias must reuse the guard key');
    assert.deepEqual(records(f), [{ project: fs.realpathSync(f.project), version: '0.1.0', provider }]);
    const explicitAlias = shell(f, command, event, { PHOENIX_UI_PROVIDER_ID: rawProvider });
    assert.equal(explicitAlias.status, 0, explicitAlias.stderr);
    assert.equal(explicitAlias.stdout + explicitAlias.stderr, '');
  });
}

test('embedded source obeys Node 18 and Windows quotation constraints', async () => {
  assert.equal(/["`$%]/.test(sentinel), false, 'cmd/PowerShell source quoting must remain literal');
  const h = await hooks;
  const command = h.buildCodexHooksManifest('.agents').hooks.PostToolUse[0].hooks[0].commandWindows;
  assert.match(command, /^node -e "/);
  assert.match(command, /\.agents\\skills\\phoenix-ui\\scripts\\phoenix-ui\.cmd/);
  assert(command.includes('node -e '), 'Windows missing-launcher path must use the same sentinel');
  assert(Buffer.byteLength(command) < 8191, 'cmd.exe command length contract');
});
