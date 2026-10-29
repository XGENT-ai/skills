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
  return spawnSync(process.execPath, [path.join(pkg.root, 'bin/xgent-skills.js'), 'install', project, '--no-phoenix-diagram', '--no-xgent-init',
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

test('--keep-impeccable installs Phoenix beside customized project and personal tools', { skip }, (t) => {
  const pkg = packageFor(t, engine);
  const project = sandbox(t, 'phoenix coexist project');
  const old = '.agents/skills/impeccable/SKILL.md';
  for (const root of [project, pkg.home]) {
    fs.mkdirSync(path.dirname(path.join(root, old)), { recursive: true });
    fs.writeFileSync(path.join(root, old), 'customized Impeccable tool\n');
  }
  const hooks = { hooks: { Stop: [{ hooks: [{ type: 'command', command: '.agents/skills/impeccable/scripts/impeccable hook' }] }] } };
  fs.mkdirSync(path.join(project, '.codex'));
  fs.writeFileSync(path.join(project, '.codex/hooks.json'), JSON.stringify(hooks));
  const result = install(pkg, project, ['--keep-impeccable', '--providers=codex']);
  assert.equal(result.status, 0, result.stdout + result.stderr);
  for (const root of [project, pkg.home]) assert.equal(fs.readFileSync(path.join(root, old), 'utf8'), 'customized Impeccable tool\n');
  const commands = JSON.stringify(JSON.parse(fs.readFileSync(path.join(project, '.codex/hooks.json'))).hooks);
  assert.match(commands, /impeccable/);
  assert.match(commands, /phoenix-ui/);
  assert.ok(fs.existsSync(path.join(project, '.phoenix-ui/install/active.json')));
});

test('--replace-impeccable deletes old project and personal tools without a tool backup', { skip }, (t) => {
  const pkg = packageFor(t, engine);
  const project = sandbox(t, 'phoenix replace project');
  for (const root of [project, pkg.home]) {
    fs.mkdirSync(path.join(root, '.agents/skills/impeccable'), { recursive: true });
    fs.writeFileSync(path.join(root, '.agents/skills/impeccable/SKILL.md'), 'old tool to delete\n');
    fs.mkdirSync(path.join(root, '.codex'), { recursive: true });
    fs.writeFileSync(path.join(root, '.codex/hooks.json'), JSON.stringify({ hooks: { Stop: [{ matcher: 'keep matcher', hooks: [
      { type: 'command', command: '.agents/skills/impeccable/scripts/impeccable hook' },
      { type: 'command', command: 'node ".agents/skills/impeccable/scripts/hook.mjs"' },
      { type: 'command', command: 'keep-other-hook' },
    ] }] }, custom: true }));
  }
  const result = install(pkg, project, ['--replace-impeccable', '--providers=codex']);
  assert.equal(result.status, 0, result.stdout + result.stderr);
  for (const root of [project, pkg.home]) {
    assert.equal(fs.existsSync(path.join(root, '.agents/skills/impeccable')), false);
    const config = JSON.parse(fs.readFileSync(path.join(root, '.codex/hooks.json')));
    assert.equal(config.custom, true);
    assert.match(JSON.stringify(config.hooks), /keep-other-hook/);
    assert.doesNotMatch(JSON.stringify(config.hooks), /impeccable/);
  }
  assert.doesNotMatch(result.stdout, /迁移  认领|保留一个迁移主版本/);
  const active = JSON.parse(fs.readFileSync(path.join(project, '.phoenix-ui/install/active.json')));
  assert.ok(active.ownership.every(item => !item.path.includes('impeccable')));
});

test('source-only Phoenix copies are reinstalled without a receipt conflict or an old-tool backup', { skip }, (t) => {
  const pkg = packageFor(t, engine);
  const project = sandbox(t, 'phoenix source project');
  fs.cpSync(path.join(repository, 'skills/phoenix-ui'), path.join(project, '.agents/skills/phoenix-ui'), { recursive: true });
  fs.writeFileSync(path.join(project, '.agents/skills/phoenix-ui/source-only-marker'), 'old source copy\n');
  const result = install(pkg, project, ['--keep-impeccable', '--providers=codex']);
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.equal(fs.existsSync(path.join(project, '.agents/skills/phoenix-ui/source-only-marker')), false);
  assert.ok(fs.existsSync(path.join(project, '.phoenix-ui/install/active.json')));
  assert.match(result.stdout, /重新安装.*\.agents\/skills\/phoenix-ui/);
});

test('non-interactive install keeps Impeccable by default', { skip }, (t) => {
  const pkg = packageFor(t, engine);
  const project = sandbox(t, 'phoenix default coexist');
  const old = path.join(project, '.claude/skills/impeccable/SKILL.md');
  fs.mkdirSync(path.dirname(old), { recursive: true });
  fs.writeFileSync(old, 'keep this tool\n');
  const result = install(pkg, project);
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.equal(fs.readFileSync(old, 'utf8'), 'keep this tool\n');
  assert.match(result.stdout, /保留.*impeccable/);
});

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

for (const shape of ['large', 'deep']) {
  test(`default install keeps Phoenix enabled without scanning a ${shape} business tree`, { skip }, async (t) => {
    const pkg = packageFor(t, engine);
    const fixture = shape === 'deep' ? await oldProject(t) : null;
    const project = fixture?.projectRoot || sandbox(t, 'phoenix large project');
    const source = path.join(project, 'src');
    fs.mkdirSync(source);
    if (shape === 'large') {
      for (let i = 0; i <= 10000; i += 1) fs.writeFileSync(path.join(source, `${i}.js`), 'export default true;\n');
    } else {
      const nested = path.join(source, ...Array(33).fill('a'));
      fs.mkdirSync(nested, { recursive: true });
      fs.writeFileSync(path.join(nested, 'page.html'), '<!-- impeccable-live-start -->');
    }
    const before = tree(source);
    const first = install(pkg, project, [], fixture?.homeRoot || pkg.home);
    assert.equal(first.status, 0, first.stdout + first.stderr);
    assert.match(first.stdout, /: 已安装/);
    assert.ok(fs.existsSync(path.join(project, '.phoenix-ui/install/active.json')));
    if (fixture) assert.match(first.stdout, /保留.*impeccable/);
    const second = install(pkg, project, [], fixture?.homeRoot || pkg.home);
    assert.equal(second.status, 0, second.stdout + second.stderr);
    assert.match(second.stdout, /: 未变\n/);
    assert.deepEqual(tree(source), before, 'installation must preserve the business source tree');
  });
}

test('explicit legacy migration through the npm bin still rolls back byte-for-byte', { skip }, async (t) => {
  const pkg = packageFor(t, engine);
  const fixture = await oldProject(t, ['claude-shared-and-local', 'custom-allow', 'unrelated-entry']);
  const xgent = install(pkg, fixture.projectRoot, ['--no-phoenix-ui'], fixture.homeRoot);
  assert.equal(xgent.status, 0, xgent.stdout + xgent.stderr);
  const original = tree(fixture.projectRoot);
  const env = { ...process.env, HOME: fixture.homeRoot, USERPROFILE: fixture.homeRoot, PHOENIX_UI_HOME: pkg.cache };
  const ready = spawnSync(process.execPath, [path.join(pkg.root, 'bin/phoenix-ui.js'), 'engine', 'install', '--project', fixture.projectRoot, '--release-dir', pkg.release], { encoding: 'utf8', env });
  assert.equal(ready.status, 0, ready.stdout + ready.stderr);
  const result = spawnSync(process.execPath, [path.join(pkg.root, 'bin/phoenix-ui.js'), 'migrate', '--from', 'impeccable', '--project', fixture.projectRoot,
    '--bundle-root', path.join(pkg.root, 'dist/phoenix-ui/bundle'), '--auxiliary-root', path.join(pkg.root, 'skills')], { encoding: 'utf8', env });
  assert.equal(result.status, 0, result.stdout + result.stderr + (result.error?.message || ''));
  assert.match(result.stdout, /installed/);
  assert.match(result.stdout, /AGENTS\.md: line\(s\) 3 mention the old impeccable entry/);
  const transition = fs.readFileSync(path.join(fixture.projectRoot, '.claude/skills/impeccable/SKILL.md'), 'utf8');
  assert.match(transition, /\/phoenix-ui/);
  const again = install(pkg, fixture.projectRoot, ['--replace-impeccable'], fixture.homeRoot);
  assert.equal(again.status, 0, again.stdout + again.stderr);
  assert.match(again.stdout, /: 未变\n/);
  const id = /--rollback (phoenix-\S+)/.exec(result.stdout)[1];
  const rollback = spawnSync(process.execPath, [path.join(pkg.root, 'bin/phoenix-ui.js'), 'migrate', '--rollback', id, '--project', fixture.projectRoot], {
    encoding: 'utf8', timeout: 600000,
    env: { ...process.env, HOME: fixture.homeRoot, USERPROFILE: fixture.homeRoot, PHOENIX_UI_HOME: pkg.cache },
  });
  assert.equal(rollback.status, 0, rollback.stdout + rollback.stderr + (rollback.error?.message || ''));
  assert.deepEqual(tree(fixture.projectRoot, (rel) => rel.startsWith('.phoenix-ui/')), original);
});

test('an unrelated hook conflict stops installation before deleting a source-only Phoenix copy', { skip }, async (t) => {
  const pkg = packageFor(t, engine);
  const fixture = await oldProject(t, ['modified-skill']);
  fs.cpSync(path.join(repository, 'skills/phoenix-ui'), path.join(fixture.projectRoot, '.agents/skills/phoenix-ui'), { recursive: true });
  fs.writeFileSync(path.join(fixture.projectRoot, '.codex/hooks.json'), '[]\n');
  const original = tree(fixture.projectRoot);
  const result = install(pkg, fixture.projectRoot, [], fixture.homeRoot);
  assert.equal(result.status, 3, result.stdout + result.stderr);
  assert.match(result.stdout, /: 冲突,未激活/);
  assert.match(result.stdout, /  冲突  /);
  assert.match(result.stderr, /Phoenix UI 预检发现冲突,未修改项目/);
  assert.deepEqual(tree(fixture.projectRoot), original);
});

test('repeated installs keep the Phoenix hooks and project state unchanged', { skip }, (t) => {
  const pkg = packageFor(t, engine);
  const project = sandbox(t, 'phoenix repeat project');
  const flags = ['--providers=claude,codex'];
  let first;
  for (let i = 0; i < 2; i += 1) {
    const result = install(pkg, project, flags);
    assert.equal(result.status, 0, result.stdout + result.stderr);
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
