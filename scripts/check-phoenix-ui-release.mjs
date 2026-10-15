import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { loadMigrationSource, materializeMigrationFixture, verifyPublishedPackage } from '../test/phoenix-ui/fixtures/migration/materialize.mjs';

// Install the packed release tarball like a consumer: fresh install with the engine downloaded from its
// public URL, and 0.3.0/0.6.0 migrations with rollback, on Node 18 and the maintenance Node.
// Usage: node scripts/check-phoenix-ui-release.mjs --tarball TGZ --tarball-sha256 HEX --manifest VERSION.json
//   --manifest-sha256 HEX --node18 NODE --node25 NODE --output-dir DIR [--legacy-dir DIR]
const here = path.dirname(fileURLToPath(import.meta.url)), root = path.resolve(here, '..');
const bootstrap = createRequire(import.meta.url)(path.join(root, 'skills/phoenix-ui/src/scripts/phoenix-bootstrap.cjs'));
const paths = ['--tarball', '--manifest', '--node18', '--node25'];
const hashes = ['--tarball-sha256', '--manifest-sha256'];
const options = {};
for (let i = 2; i < process.argv.length; i++) {
  const flag = process.argv[i];
  assert([...paths, ...hashes, '--output-dir', '--legacy-dir'].includes(flag) && process.argv[i + 1] && !options[flag], `Invalid argument ${flag}`); options[flag] = process.argv[++i];
}
for (const flag of [...paths, ...hashes, '--output-dir']) assert(options[flag], `Required ${flag}`);
const sha = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const json = value => JSON.stringify(value, null, 2) + '\n';
for (const flag of paths) { options[flag] = path.resolve(options[flag]); assert(fs.lstatSync(options[flag]).isFile() && !fs.lstatSync(options[flag]).isSymbolicLink(), `Expected regular file ${flag}`); }
for (const flag of hashes) assert(/^[a-f0-9]{64}$/.test(options[flag]), `Invalid ${flag}`);
for (const flag of ['--tarball', '--manifest']) assert.equal(sha(fs.readFileSync(options[flag])), options[`${flag}-sha256`], `Input digest differs ${flag}`);
const preflight = [];
function member(archive, relative) {
  const argv = ['-xOf', archive, `package/${relative}`], result = spawnSync('tar', argv, { maxBuffer: 32 * 1024 * 1024 });
  preflight.push({ program: 'tar', argv, exitCode: result.status, error: result.error?.message || null });
  assert.equal(result.status, 0, `Missing tarball member ${relative}`); return result.stdout;
}
const B = bootstrap.manifestAt(options['--manifest']);
assert.equal(B.data.distribution, 'release', 'Final B must use the frozen release manifest');
assert(member(options['--tarball'], 'dist/phoenix-ui/VERSION.json').equals(B.bytes), 'Final tarball manifest bytes differ');
const pkg = JSON.parse(member(options['--tarball'], 'package.json'));
assert.equal(pkg.name, '@xgent-ai/skills'); assert.equal(pkg.version, B.data.npmPackageVersion);
assert.deepEqual(Object.keys(B.data.engines).sort(), ['darwin-arm64', 'darwin-x64', 'linux-arm64', 'linux-x64']);
for (const engine of Object.values(B.data.engines)) assert(engine.url.startsWith('https://'), 'Release engines must have public HTTPS URLs');
const out = path.resolve(options['--output-dir']); fs.mkdirSync(out, { recursive: false });
const raw = path.join(out, 'raw-inputs'); fs.mkdirSync(raw);
// Published 0.3.0/0.6.0 bytes: a verified local copy if present, else the npm registry, both pinned by SHA-256.
const oldSources = [];
for (const version of ['0.3.0', '0.6.0']) {
  const source = loadMigrationSource(version), archive = path.join(raw, `legacy-${version}.tgz`);
  const cached = options['--legacy-dir'] && path.join(path.resolve(options['--legacy-dir']), source.provenance.npm.filename);
  if (cached && fs.existsSync(cached)) fs.copyFileSync(cached, archive);
  else {
    const response = await fetch(source.provenance.npm.url, { signal: AbortSignal.timeout(300000) });
    assert(response.ok, `Download ${source.provenance.npm.url} failed: HTTP ${response.status}`);
    fs.writeFileSync(archive, Buffer.from(await response.arrayBuffer()));
  }
  assert.equal(sha(fs.readFileSync(archive)), source.provenance.npm.sha256, `Published ${version} tarball differs`);
  oldSources.push({ version, source, archive });
}
const inputReport = { scope: 'release-check', B: { tarball: options['--tarball'], tarballSha256: options['--tarball-sha256'], manifestSha256: B.hash }, preflight,
  nodeExecutables: [18, 25].map(major => ({ major, path: options[`--node${major}`], sha256: sha(fs.readFileSync(options[`--node${major}`])) })),
  legacy: oldSources.map(({ version, source }) => ({ version, sha256: source.provenance.npm.sha256 })) };
for (const [name, file] of [['B.tgz', options['--tarball']], ['B.VERSION.json', options['--manifest']]]) fs.copyFileSync(file, path.join(raw, name), fs.constants.COPYFILE_EXCL);
const sourceBindings = new Map([[path.join(raw, 'B.tgz'), options['--tarball-sha256']], ...oldSources.map(source => [source.archive, source.source.provenance.npm.sha256])]);
for (const [file, expected] of [...sourceBindings, [path.join(raw, 'B.VERSION.json'), B.hash]]) assert.equal(sha(fs.readFileSync(file)), expected, 'Copied input bytes differ');
const record = { ...inputReport, status: 'running', out, commands: [], groups: [], startedAt: new Date().toISOString() };
const save = () => fs.writeFileSync(path.join(out, 'results.json'), json(record));
function run(name, program, argv, cwd = out, env = process.env, expected = 0) {
  const result = spawnSync(program, argv, { cwd, env, encoding: 'utf8', input: '', timeout: 600000, maxBuffer: 64 * 1024 * 1024 });
  const prefix = path.join(out, `${record.commands.length}-${name}`);
  fs.writeFileSync(`${prefix}.stdout`, result.stdout || ''); fs.writeFileSync(`${prefix}.stderr`, result.stderr || '');
  record.commands.push({ name, program, argv, cwd, exitCode: result.status, signal: result.signal, error: result.error?.message || null, expected,
    stdout: `${prefix}.stdout`, stderr: `${prefix}.stderr`, sourceTarballSha256: argv.map(argument => sourceBindings.get(argument)).find(Boolean) || null, environment: { PHOENIX_UI_HOME: env.PHOENIX_UI_HOME, sandboxHome: env.HOME, npmCache: env.npm_config_cache } }); save();
  assert.equal(result.status, expected, `${name}: ${result.stdout}\n${result.stderr}\n${result.error?.message || ''}`); return result;
}
// Same byte/link tree assertion as install.test.js; .phoenix-ui journal history is retained.
function tree(directory, exclude = () => false) {
  const entries = {};
  const walk = dir => { for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const file = path.join(dir, entry.name), relative = path.relative(directory, file).split(path.sep).join('/');
    if (exclude(relative)) continue;
    if (entry.isDirectory()) walk(file); else entries[relative] = entry.isSymbolicLink() ? `link:${fs.readlinkSync(file)}` : sha(fs.readFileSync(file));
  } }; walk(directory); return entries;
}
const outsideHistory = relative => relative.startsWith('.phoenix-ui/');
function snapshot(base, phase, project) {
  const destination = path.join(base, phase); fs.mkdirSync(destination);
  fs.cpSync(project, path.join(destination, 'project'), { recursive: true, verbatimSymlinks: true });
  fs.writeFileSync(path.join(destination, 'tree.json'), json(tree(project))); return tree(project, outsideHistory);
}
function environment(base, sandboxHome = path.join(base, 'user home')) {
  for (const dir of [base, sandboxHome]) fs.mkdirSync(dir, { recursive: true });
  const userConfig = path.join(base, 'empty-user.npmrc'); fs.writeFileSync(userConfig, '');
  const globalConfig = path.join(base, 'empty-global.npmrc'); fs.writeFileSync(globalConfig, '');
  // Real public URLs: keep the maintainer's proxy settings.
  return { ...process.env, HOME: sandboxHome, USERPROFILE: sandboxHome, PHOENIX_UI_HOME: path.join(base, 'phoenix cache'), npm_config_cache: path.join(base, 'npm-cache'),
    npm_config_userconfig: userConfig, npm_config_globalconfig: globalConfig };
}
function npmInstall(node, archive, destination, env, name) {
  fs.mkdirSync(destination); fs.writeFileSync(path.join(destination, 'package.json'), '{"private":true}\n');
  const npm = path.resolve(path.dirname(node), '../lib/node_modules/npm/bin/npm-cli.js');
  run(`${name}-npm-install`, node, [npm, 'install', '--offline', '--no-audit', '--no-fund', archive], destination, env);
  const pkg = path.join(destination, 'node_modules/@xgent-ai/skills');
  for (const bin of ['phoenix-ui.js', 'xgent-skills.js']) sourceBindings.set(path.join(pkg, 'bin', bin), sourceBindings.get(archive));
  return pkg;
}
function install(node, pkg, project, env, name, release) {
  return run(name, node, [path.join(pkg, 'bin/xgent-skills.js'), 'install', project, '--no-xgent-init', ...(release ? [`--release-dir=${release}`] : [])], project, env);
}
function rollback(node, pkg, project, env, output, name) {
  const id = /--rollback (phoenix-\S+)/.exec(output.stdout)?.[1]; assert(id, 'Missing actual rollback transaction');
  run(name, node, [path.join(pkg, 'bin/phoenix-ui.js'), 'migrate', '--rollback', id, '--project', project], project, env); return id;
}
try {
const packages = path.join(out, 'old-packages'); fs.mkdirSync(packages);
for (const source of oldSources) {
  const destination = path.join(packages, source.version); fs.mkdirSync(destination);
  run(`extract-${source.version}`, 'tar', ['-xzf', source.archive, '-C', destination]);
  verifyPublishedPackage(path.join(destination, 'package'), source.source);
}
for (const major of [18, 25]) {
  const node = options[`--node${major}`], base = path.join(out, `node${major}`); fs.mkdirSync(base);
  const setupEnv = environment(path.join(base, 'package setup'));
  const info = JSON.parse(run(`node${major}-identity`, node, ['-p', 'JSON.stringify({version:process.version,target:process.platform+"-"+process.arch})'], base, setupEnv).stdout);
  assert(info.version.startsWith(`v${major}.`), 'Wrong supplied Node major'); assert(B.data.engines[info.target], 'Release does not support this Node host');
  const pkg = npmInstall(node, path.join(raw, 'B.tgz'), path.join(base, 'B consumer'), setupEnv, `node${major}-B`);
  assert(fs.readFileSync(path.join(pkg, 'dist/phoenix-ui/VERSION.json')).equals(B.bytes));
  const bBin = path.join(pkg, 'bin/phoenix-ui.js');
  for (const kind of ['fresh', 'legacy-0.3.0', 'legacy-0.6.0']) {
    const groupBase = path.join(base, kind); fs.mkdirSync(groupBase);
    const group = { node: info, kind, status: 'running', BtarballSha256: options['--tarball-sha256'] }; record.groups.push(group); save();
    try {
      let project, env;
      if (kind.startsWith('legacy-')) {
        const fixture = materializeMigrationFixture({ fixtureRoot: path.join(groupBase, 'fixture'), packageVersion: kind.slice(7), packagesDir: packages, providers: ['.claude', '.agents'], channel: 'xgent-installer', scenarios: ['claude-shared-and-local', 'custom-allow', 'unrelated-entry'] });
        project = fixture.projectRoot; env = environment(groupBase, fixture.homeRoot); fs.writeFileSync(path.join(groupBase, 'fixture.json'), json(fixture));
      } else {
        project = path.join(groupBase, 'project with spaces'); fs.mkdirSync(project); env = environment(groupBase);
        fs.writeFileSync(path.join(project, 'package.json'), '{"private":true}\n'); fs.writeFileSync(path.join(project, 'index.html'), '<html><body>Consumer input</body></html>\n');
      }
      const original = snapshot(groupBase, 'before', project);
      if (kind === 'fresh') {
        run(`node${major}-missing-engine`, node, [bBin, 'context', '--json'], project, env, 4);
        const engine = JSON.parse(run(`node${major}-HTTP-engine-install`, node, [bBin, 'engine', 'install', '--project', project, '--json'], project, env).stdout);
        assert.equal(engine.status, 'installed'); assert.equal(sha(fs.readFileSync(engine.binary)), B.data.engines[info.target].sha256);
      }
      const first = install(node, pkg, project, env, `node${major}-${kind}-install`);
      assert.match(first.stdout, new RegExp(`Phoenix UI ${B.data.toolVersion.replaceAll('.', '\\.')} \\(@xgent-ai/skills@[^)]+\\): 已安装`));
      assert.match(first.stdout, /完成: /); assert.match(first.stdout, /回滚  npm exec --package=@xgent-ai\/skills@\S+ -- phoenix-ui migrate --rollback phoenix-\S+ --project /);
      if (kind.startsWith('legacy-')) {
        assert.match(first.stdout, /迁移  认领 impeccable 旧版安装/);
        assert.match(first.stdout, /命令  \.agents: \$impeccable → \$phoenix-ui;\.claude: \/impeccable → \/phoenix-ui/);
        assert.match(first.stdout, /说明  AGENTS\.md: line\(s\) 3 mention the old impeccable entry/);
        assert.match(first.stdout, /重载  \.agents\/\.codex: 在 Codex 中运行 \/hooks/);
        assert.match(fs.readFileSync(path.join(project, '.claude/skills/impeccable/SKILL.md'), 'utf8'), /\/phoenix-ui/);
      } else if (kind === 'fresh') {
        assert.match(first.stdout, /命令  \.claude: \/phoenix-ui/); assert.doesNotMatch(first.stdout, /impeccable →/); assert.match(first.stdout, /重载  \.claude: 重启 Claude Code 会话/);
        assert(first.stdout.indexOf('.claude/hooks/xgent-statusline.js') < first.stdout.indexOf(`Phoenix UI ${B.data.toolVersion}`));
      }
      for (const file of ['.claude/skills/phoenix-ui/SKILL.md', '.claude/skills/review-ui/SKILL.md', '.phoenix-ui/install/active.json']) assert(fs.existsSync(path.join(project, file)), file);
      assert(fs.readFileSync(path.join(project, '.claude/skills/phoenix-ui/scripts/ENGINE.json')).equals(B.bytes));
      group.installedBEngineSha256 = sha(fs.readFileSync(bootstrap.enginePath(B, info.target, env)));
      assert.equal(group.installedBEngineSha256, B.data.engines[info.target].sha256);
      const installed = snapshot(groupBase, 'installed', project);
      const second = install(node, pkg, project, env, `node${major}-${kind}-rerun`);
      assert.match(second.stdout, /: 未变\n/); assert.doesNotMatch(second.stdout, /^\s*(?:事务\s+phoenix-\S+;备份\s+|回滚\s+.+--rollback\s+phoenix-\S+)/m); assert.deepEqual(tree(project, outsideHistory), installed);
      assert.equal(run(`node${major}-${kind}-version`, node, [bBin, '--version'], project, env).stdout.trim(), B.data.toolVersion);
      assert(JSON.parse(run(`node${major}-${kind}-context`, node, [bBin, 'context', '--json'], project, env).stdout));
      run(`node${major}-${kind}-bundle-check`, node, [bBin, 'skills', 'check', '--bundle-root', path.join(pkg, 'dist/phoenix-ui/bundle'), '--json'], project, env);
      group.transaction = rollback(node, pkg, project, env, first, `node${major}-${kind}-rollback`);
      const restored = snapshot(groupBase, 'rolled-back', project);
      if (kind === 'fresh') { for (const [file, digest] of Object.entries(original)) assert.equal(restored[file], digest); assert(!fs.existsSync(path.join(project, '.claude/skills/phoenix-ui'))); }
      else assert.deepEqual(restored, original);
      group.status = 'expected-consumer-outcomes-observed';
    } catch (error) { group.status = 'failed'; group.error = error.message; process.exitCode = 1; }
    save();
  }
}
} catch (error) { record.error = error.message; process.exitCode = 1; }
record.status = !record.error && record.groups.length === 6 && record.groups.every(group => group.status === 'expected-consumer-outcomes-observed') ? 'release-check-passed' : 'failed';
record.finishedAt = new Date().toISOString(); save();
if (record.status !== 'release-check-passed') process.exitCode = 1;
console.log(json({ out, status: record.status, tarballSha256: options['--tarball-sha256'], manifestSha256: B.hash, groups: record.groups }));
