'use strict';
// Node installer → Rust transactional installer, with the engine under test.
// test:phoenix supplies PHOENIX_UI_BIN; the lightweight suite has no engine.
const test = require('node:test');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const repository = path.resolve(__dirname, '../..');
const engine = process.env.PHOENIX_UI_BIN;
const skip = engine ? false : 'explicit local Phoenix engine (PHOENIX_UI_BIN) needed; npm run test:phoenix supplies it';
const sha256 = (bytes) => crypto.createHash('sha256').update(bytes).digest('hex');
const materializer = import('./fixtures/migration/materialize.mjs');

function sandbox(t, name) {
  const dir = fs.mkdtempSync(path.join(fs.realpathSync(os.tmpdir()), `${name} `));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  return dir;
}

function link(target, destination) {
  fs.symlinkSync(target, destination, process.platform === 'win32' ? 'junction' : 'dir');
}

/**
 * A package copy whose manifest pins `executable`. The bin scripts and bundle are
 * copied so that their package root is this copy; skills and hooks link to the
 * repository's own bytes.
 */
function packageFor(t, executable) {
  const root = sandbox(t, 'phoenix package');
  const bin = path.join(root, 'bin');
  fs.mkdirSync(bin);
  for (const name of ['xgent-skills.js', 'phoenix-ui.js']) fs.copyFileSync(path.join(repository, 'bin', name), path.join(bin, name));
  for (const dir of ['.claude', 'hooks', 'skills']) link(path.join(repository, dir), path.join(root, dir));
  const vendor = path.join(root, 'dist', 'phoenix-ui');
  fs.mkdirSync(vendor, { recursive: true });
  // The installer refuses a symlinked bundle root.
  fs.cpSync(path.join(repository, 'dist/phoenix-ui/bundle'), path.join(vendor, 'bundle'), { recursive: true });
  const target = process.platform === 'win32' ? 'windows-x64' : `${process.platform}-${process.arch}`;
  const asset = `phoenix-ui-${target}${target.startsWith('windows-') ? '.exe' : ''}`;
  const bytes = fs.readFileSync(executable);
  const manifest = { ...JSON.parse(fs.readFileSync(path.join(repository, 'dist/phoenix-ui/VERSION.json'))),
    distribution: 'development', engines: { [target]: { asset, sha256: sha256(bytes), size: bytes.length } } };
  const manifestBytes = JSON.stringify(manifest, null, 2) + '\n';
  const release = path.join(root, 'release');
  fs.mkdirSync(release);
  for (const directory of [vendor, release]) fs.writeFileSync(path.join(directory, 'VERSION.json'), manifestBytes);
  fs.writeFileSync(path.join(release, asset), bytes, { mode: 0o755 });
  for (const name of ['THIRD-PARTY.json', 'THIRD-PARTY-NOTICES.md', 'THIRD-PARTY-LICENSES']) {
    fs.cpSync(path.join(repository, 'dist/phoenix-ui', name), path.join(release, name), { recursive: true });
  }
  return { root, release, cache: sandbox(t, 'phoenix cache'), home: sandbox(t, 'phoenix home') };
}

function install(pkg, project, flags = [], home = pkg.home) {
  return spawnSync(process.execPath, [path.join(pkg.root, 'bin/xgent-skills.js'), 'install', project, '--no-xgent-init',
    `--release-dir=${pkg.release}`, ...flags], {
    encoding: 'utf8', input: '', timeout: 600000,
    env: { ...process.env, HOME: home, USERPROFILE: home, PHOENIX_UI_HOME: pkg.cache },
  });
}

function tree(root, exclude = () => false) {
  const out = {};
  const walk = (dir) => {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const file = path.join(dir, entry.name);
      const rel = path.relative(root, file).split(path.sep).join('/');
      if (exclude(rel)) continue;
      if (entry.isDirectory()) walk(file);
      else out[rel] = entry.isSymbolicLink() ? `link:${fs.readlinkSync(file)}` : sha256(fs.readFileSync(file));
    }
  };
  walk(root);
  return out;
}

async function oldProject(t, scenarios = [], providers = ['.claude', '.agents']) {
  const { materializeMigrationFixture } = await materializer;
  const fixture = materializeMigrationFixture({ fixtureRoot: path.join(sandbox(t, 'phoenix migration'), 'fixture'),
    packageVersion: '0.6.0', providers, channel: 'xgent-installer', scenarios });
  return fixture;
}

test('fresh install activates Phoenix after the XGENT part and a rerun changes nothing', { skip }, (t) => {
  const pkg = packageFor(t, engine);
  const project = sandbox(t, 'phoenix fresh project');
  const first = install(pkg, project);
  assert.equal(first.status, 0, first.stdout + first.stderr);
  assert.match(first.stdout, /Phoenix UI 0\.1\.0 \(@xgent-ai\/skills@[^)]+\): 已安装/);
  assert.match(first.stdout, /命令  \.claude: \/phoenix-ui/);
  assert.doesNotMatch(first.stdout, /impeccable →/);
  assert.match(first.stdout, /回滚  npm exec --package=@xgent-ai\/skills@\S+ -- phoenix-ui migrate --rollback phoenix-\S+ --project /);
  assert.match(first.stdout, /重载  \.claude: 重启 Claude Code 会话/);
  assert.ok(first.stdout.indexOf('.claude/hooks/xgent-statusline.js') < first.stdout.indexOf('Phoenix UI 0.1.0'));
  assert.match(first.stdout, /完成: /);
  for (const file of ['.claude/skills/phoenix-ui/SKILL.md', '.claude/skills/review-ui/SKILL.md', '.phoenix-ui/install/active.json']) {
    assert.ok(fs.existsSync(path.join(project, file)), file);
  }
  const before = tree(project, (rel) => rel.startsWith('.phoenix-ui/'));
  const second = install(pkg, project);
  assert.equal(second.status, 0, second.stdout + second.stderr);
  assert.match(second.stdout, /: 未变\n/);
  assert.doesNotMatch(second.stdout, /回滚|保留/);
  assert.deepEqual(tree(project, (rel) => rel.startsWith('.phoenix-ui/')), before);
});

test('an old 0.6.0 project migrates through the Node installer and rolls back byte-for-byte through the npm bin', { skip }, async (t) => {
  const pkg = packageFor(t, engine);
  const fixture = await oldProject(t, ['claude-shared-and-local', 'custom-allow', 'unrelated-entry']);
  const original = tree(fixture.projectRoot);
  const result = install(pkg, fixture.projectRoot, [], fixture.homeRoot);
  assert.equal(result.status, 0, result.stdout + result.stderr + (result.error?.message || ''));
  assert.match(result.stdout, /迁移  认领 impeccable 旧版安装/);
  assert.match(result.stdout, /命令  \.agents: \$impeccable → \$phoenix-ui;\.claude: \/impeccable → \/phoenix-ui/);
  assert.match(result.stdout, /说明  AGENTS\.md: line\(s\) 3 mention the old impeccable entry/);
  assert.match(result.stdout, /重载  \.agents\/\.codex: 在 Codex 中运行 \/hooks/);
  const transition = fs.readFileSync(path.join(fixture.projectRoot, '.claude/skills/impeccable/SKILL.md'), 'utf8');
  assert.match(transition, /\/phoenix-ui/);
  const id = /--rollback (phoenix-\S+)/.exec(result.stdout)[1];
  const rollback = spawnSync(process.execPath, [path.join(pkg.root, 'bin/phoenix-ui.js'), 'migrate', '--rollback', id, '--project', fixture.projectRoot], {
    encoding: 'utf8', timeout: 600000,
    env: { ...process.env, HOME: fixture.homeRoot, USERPROFILE: fixture.homeRoot, PHOENIX_UI_HOME: pkg.cache },
  });
  assert.equal(rollback.status, 0, rollback.stdout + rollback.stderr + (rollback.error?.message || ''));
  assert.deepEqual(tree(fixture.projectRoot, (rel) => rel.startsWith('.phoenix-ui/')), original);
});

test('a user-modified old skill blocks Phoenix before any project write, including the XGENT part', { skip }, async (t) => {
  const pkg = packageFor(t, engine);
  const fixture = await oldProject(t, ['modified-skill']);
  const original = tree(fixture.projectRoot);
  const result = install(pkg, fixture.projectRoot, ['--claude-context-goal-guard'], fixture.homeRoot);
  assert.equal(result.status, 3, result.stdout + result.stderr);
  assert.match(result.stdout, /: 冲突,未激活/);
  assert.match(result.stdout, /  冲突  /);
  assert.match(result.stderr, /Phoenix UI 预检发现冲突,未修改项目/);
  assert.deepEqual(tree(fixture.projectRoot), original);
});

test('both goal guards and the Phoenix hooks keep exactly one handler each across repeated installs', { skip }, (t) => {
  const pkg = packageFor(t, engine);
  const project = sandbox(t, 'phoenix guard project');
  const flags = ['--providers=claude,codex', '--claude-context-goal-guard', ...(spawnSync('python3', ['--version']).status === 0 ? ['--context-goal-guard'] : [])];
  let first;
  for (let i = 0; i < 2; i += 1) {
    const result = install(pkg, project, flags);
    assert.equal(result.status, 0, result.stdout + result.stderr);
    const local = JSON.parse(fs.readFileSync(path.join(project, '.claude/settings.local.json'), 'utf8'));
    for (const event of ['PostToolBatch', 'Stop']) {
      assert.equal(local.hooks[event].flatMap((entry) => entry.hooks).filter((handler) => handler.command.includes('context-goal-guard.js')).length, 1, event);
    }
    const handlers = [path.join(project, '.claude/settings.json'), path.join(project, '.claude/settings.local.json'), path.join(project, '.codex/hooks.json')]
      .filter((file) => fs.existsSync(file))
      .flatMap((file) => Object.values(JSON.parse(fs.readFileSync(file, 'utf8')).hooks || {}).flat().flatMap((entry) => entry.hooks || []));
    assert.ok(handlers.some((handler) => /skills\/phoenix-ui\/scripts\/phoenix-ui/.test(handler.command)), 'Phoenix design hook');
    const snapshot = tree(project, (rel) => rel.startsWith('.phoenix-ui/'));
    if (first) assert.deepEqual(snapshot, first);
    first = snapshot;
  }
});

test('a Rust apply failure after the XGENT part leaves Phoenix inactive, reports an unfinished install and a rerun completes', {
  skip: skip || (process.platform === 'win32' && 'the failing engine stand-in is a POSIX shell script'),
}, (t) => {
  const stand = path.join(sandbox(t, 'phoenix failing engine'), 'phoenix-ui');
  fs.writeFileSync(stand, `#!/bin/sh\ncase " $* " in\n  *" install "*) case " $* " in *" --dry-run "*) ;; *) echo "simulated apply failure" >&2; exit 4;; esac;;\nesac\nexec '${engine.replaceAll("'", "'\\''")}' "$@"\n`, { mode: 0o755 });
  const failing = packageFor(t, stand);
  const project = sandbox(t, 'phoenix interrupted project');
  const first = install(failing, project);
  assert.equal(first.status, 4, first.stdout + first.stderr);
  assert.match(first.stderr, /simulated apply failure/);
  assert.match(first.stdout, /未完成: XGENT 部分已安装;Phoenix UI 待恢复\(安装被中断,可能已部分写入\)/);
  assert.doesNotMatch(first.stdout, /^完成: /m);
  assert.ok(fs.existsSync(path.join(project, '.claude/hooks/xgent-statusline.js')));
  assert.equal(fs.existsSync(path.join(project, '.phoenix-ui/install/active.json')), false);
  const second = install(packageFor(t, engine), project);
  assert.equal(second.status, 0, second.stdout + second.stderr);
  assert.match(second.stdout, /: 已安装/);
  assert.ok(fs.existsSync(path.join(project, '.phoenix-ui/install/active.json')));
});
