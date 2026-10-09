'use strict';

const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const crypto = require('node:crypto');

const root = path.resolve(__dirname, '../..');
const modulePromise = import('./fixtures/migration/materialize.mjs');
const hash = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const json = file => JSON.parse(fs.readFileSync(file, 'utf8'));
const versions = ['0.3.0', '0.4.0', '0.5.0', '0.6.0'];

function sandbox(t) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-migration-'));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  return dir;
}

function smallPackage(t) {
  const directory = sandbox(t);
  const packagesDir = path.join(directory, 'packages');
  const pkg = path.join(packagesDir, '0.6.0/package');
  const source = json(path.join(root, 'tools/phoenix-ui/migration-sources/0.6.0.json'));
  const blobDir = path.join(pkg, 'vendor/impeccable/bundle/blobs');
  fs.mkdirSync(blobDir, { recursive: true });
  function blob(bytes) {
    const sha = hash(bytes);
    fs.writeFileSync(path.join(blobDir, sha), bytes);
    return sha;
  }
  const manifest = { schema: 1, providers: {} };
  for (const [provider, entry] of Object.entries(source.providers)) {
    const files = {
      'skills/impeccable/SKILL.md': blob(`---\nname: impeccable\n---\nSmall test input for ${provider}.\n`),
      'skills/impeccable/reference/polish.md': blob(`Non-executable reference for ${provider}.\n`),
      'skills/impeccable/scripts/impeccable': blob('#!/bin/sh\n# Synthetic test launcher, never executed.\n'),
      'skills/impeccable/scripts/impeccable.cmd': blob('@REM Synthetic test launcher, never executed.\n'),
    };
    for (const rel of Object.keys(entry.files).filter(rel => /^(agents|commands)\/[^/]+$/.test(rel))) files[rel] = blob(`Synthetic test agent or command: ${rel}\n`);
    const raw = entry.hooks.find(hook => hook.shape === 'bundle');
    if (raw && entry.hookArtifact.source.startsWith(`${provider}/`)) files[entry.hookArtifact.source.slice(provider.length + 1)] = blob(JSON.stringify(raw.value, null, 2) + '\n');
    entry.files = files;
    manifest.providers[provider] = { files, exec: entry.exec };
  }
  const installer = '// Synthetic test input, not a released installer.\n';
  const version = JSON.stringify({ skillVersion: source.skillVersion, engineVersion: source.engineVersion }) + '\n';
  const manifestBytes = JSON.stringify(manifest) + '\n';
  fs.mkdirSync(path.join(pkg, 'bin'), { recursive: true });
  fs.writeFileSync(path.join(pkg, 'bin/xgent-skills.js'), installer);
  fs.writeFileSync(path.join(pkg, 'vendor/impeccable/VERSION.json'), version);
  fs.writeFileSync(path.join(pkg, 'vendor/impeccable/bundle/manifest.json'), manifestBytes);
  fs.writeFileSync(path.join(pkg, 'package.json'), JSON.stringify({ version: '0.6.0' }));
  const xgent = '.claude/hooks/xgent-statusline.js';
  fs.mkdirSync(path.dirname(path.join(pkg, xgent)), { recursive: true });
  fs.writeFileSync(path.join(pkg, xgent), '// Synthetic statusline test input.\n');
  source.installerSha256 = hash(installer);
  source.versionSha256 = hash(version);
  source.bundleManifestSha256 = hash(manifestBytes);
  source.xgent.hookFiles = { [xgent]: hash(fs.readFileSync(path.join(pkg, xgent))) };
  const additionalSkillsDir = path.join(directory, 'additional');
  const additionalSource = { sourceCommit: 'synthetic-test-input', files: {} };
  for (const name of ['design-md', 'design-reference', 'ui-pattern-research', 'review-ui', 'apply-ui-review']) {
    for (const rel of [`${name}/SKILL.md`, `${name}/references/contract.md`]) {
      const bytes = `---\nname: ${name}\n---\nSmall external test input: ${rel}\n`;
      fs.mkdirSync(path.dirname(path.join(additionalSkillsDir, rel)), { recursive: true });
      fs.writeFileSync(path.join(additionalSkillsDir, rel), bytes);
      additionalSource.files[rel] = hash(bytes);
    }
  }
  return { directory, packagesDir, source, additionalSource, additionalSkillsDir };
}

function assertProviderBytes(result, source, channel) {
  for (const provider of result.providers) {
    const entry = source.providers[provider];
    for (const [rel, sha] of Object.entries(entry.files)) {
      if (channel === 'xgent-installer' && !rel.startsWith('skills/') && !/^(agents|commands)\/[^/]+$/.test(rel)) continue;
      assert.equal(hash(fs.readFileSync(path.join(result.projectRoot, provider, rel))), sha, `${provider}/${rel}`);
    }
    assert.match(fs.readFileSync(path.join(result.projectRoot, provider, 'skills/user-helper/SKILL.md'), 'utf8'), /User-authored skill/);
  }
  for (const rel of result.inventory.hookFiles) {
    const owner = Object.values(source.providers).find(entry => entry.hookArtifact?.[channel === 'bundle' ? 'source' : 'destination'] === rel
      && entry.hooks.some(hook => hook.shape === channel));
    assert.ok(owner, rel);
    assert.deepEqual(json(path.join(result.projectRoot, rel)), owner.hooks.find(hook => hook.shape === channel).value);
  }
}

test('source records distinguish tarball evidence from Git comparison and retain raw Codex hooks', () => {
  for (const version of versions) {
    const source = json(path.join(root, `tools/phoenix-ui/migration-sources/${version}.json`));
    assert.equal(source.provenance.primary, 'verified-published-npm-tarball');
    assert.equal(source.provenance.npm.extractedBytesVerified, true);
    assert.match(source.provenance.npm.sha256, /^[a-f0-9]{64}$/);
    assert.match(source.provenance.npm.integrity, /^sha512-/);
    assert.equal(source.provenance.git.allMigrationFilesMatch, true);
    assert.equal(source.sourceCommit, source.provenance.git.migrationFilesCommit);
    assert.equal(source.skillVersion, version === '0.3.0' ? '4.3.1' : '4.5.0');
    assert.equal(source.engineVersion, version === '0.3.0' ? '0.1.5' : '0.1.11');
    assert.equal(Object.keys(source.providers).length, 19);
    assert.equal(source.providers['.codex'].installerSelectable, false);
    const raw = source.providers['.codex'].hooks.find(hook => hook.shape === 'bundle');
    assert.equal(hash(JSON.stringify(raw.value, null, 2) + '\n'), raw.blobSha256);
    const rewritten = source.providers['.agents'].hooks.find(hook => hook.shape === 'xgent-installer').value;
    assert.match(JSON.stringify(rewritten), /\.agents\/skills\/impeccable/);
    assert.doesNotMatch(JSON.stringify(rewritten), /\.codex\/skills\/impeccable/);
  }
  const early = json(path.join(root, 'tools/phoenix-ui/migration-sources/0.3.0.json'));
  const fourth = json(path.join(root, 'tools/phoenix-ui/migration-sources/0.4.0.json'));
  assert.equal(early.provenance.git.comparison['vendor/impeccable/bundle/manifest.json'].gitSha256, null);
  assert.equal(early.provenance.git.comparison['bin/xgent-skills.js'].matches, false);
  assert.equal(fourth.provenance.git.comparison['bin/xgent-skills.js'].matches, false);
  assert.equal(fourth.provenance.git.gitPackageVersion, '0.3.0');
});

test('small packages materialize all 19 provider layouts with non-executable files and both hook channels', async t => {
  const { materializeMigrationFixture } = await modulePromise;
  const input = smallPackage(t);
  for (const channel of ['xgent-installer', 'bundle']) {
    const providers = Object.keys(input.source.providers).filter(provider => channel === 'bundle' || provider !== '.codex');
    const result = materializeMigrationFixture({ ...input, fixtureRoot: path.join(input.directory, channel), providers, channel });
    assertProviderBytes(result, input.source, channel);
    assert.match(result.projectRoot, /project with spaces$/);
    if (channel === 'xgent-installer') {
      assert.ok(fs.existsSync(path.join(result.projectRoot, '.agents/skills/impeccable/SKILL.md')));
      assert.equal(fs.existsSync(path.join(result.projectRoot, '.codex/skills/impeccable/SKILL.md')), false);
      assert.ok(fs.existsSync(path.join(result.projectRoot, '.codex/hooks.json')));
      assert.deepEqual(json(path.join(result.projectRoot, '.claude/settings.json')).statusLine, input.source.xgent.managedSettings.statusLine);
      if (process.platform !== 'win32') assert.equal(fs.statSync(path.join(result.projectRoot, '.claude/skills/impeccable/scripts/impeccable.cmd')).mode & 0o777, 0o755);
    }
  }
});

test('shared/local Claude and nested/flat provider hooks contain adjacent user handlers, allows and unknown fields', async t => {
  const { materializeMigrationFixture } = await modulePromise;
  const input = smallPackage(t);
  const result = materializeMigrationFixture({ ...input, fixtureRoot: path.join(input.directory, 'mixed'),
    providers: ['.claude', '.agents', '.cursor', '.github', '.grok'],
    scenarios: ['claude-shared-and-local', 'mixed-handlers', 'custom-allow', 'unrelated-entry'] });
  assert.ok(result.inventory.hookFiles.includes('.claude/settings.json'));
  assert.ok(result.inventory.hookFiles.includes('.claude/settings.local.json'));
  for (const rel of result.inventory.hookFiles) {
    const value = json(path.join(result.projectRoot, rel));
    assert.deepEqual(value.permissions.allow, ['Read', 'Bash(git status:*)']);
    assert.deepEqual(value.fixtureUnknown, { arbitrary: ['preserve', 42] });
    const entries = Object.values(value.hooks).flat();
    assert.ok(entries.some(entry => entry.fixtureUnrelated));
    assert.ok(entries.some(entry => entry.fixtureMatcherNote));
    assert.match(JSON.stringify(entries), /custom hook with spaces\.js/);
    assert.match(JSON.stringify(entries), /fixtureUnknown/);
  }
});

test('modifications change actual skill, launcher, handler and pin bytes; pristine neighbors remain', async t => {
  const { materializeMigrationFixture } = await modulePromise;
  const input = smallPackage(t);
  const result = materializeMigrationFixture({ ...input, fixtureRoot: path.join(input.directory, 'modified'),
    providers: ['.claude', '.agents', '.opencode'], scenarios: ['modified-skill', 'modified-handler', 'unknown-upstream', 'modified-pin'] });
  for (const provider of result.providers) {
    assert.notEqual(hash(fs.readFileSync(path.join(result.projectRoot, provider, 'skills/impeccable/SKILL.md'))), input.source.providers[provider].files['skills/impeccable/SKILL.md']);
    assert.match(fs.readFileSync(path.join(result.projectRoot, provider, 'skills/impeccable/scripts/impeccable'), 'utf8'), /unsupported upstream launcher bytes/);
    assert.equal(hash(fs.readFileSync(path.join(result.projectRoot, provider, 'skills/impeccable/reference/polish.md'))), input.source.providers[provider].files['skills/impeccable/reference/polish.md']);
    const pin = provider === '.opencode' ? 'commands/impeccable-polish.md' : 'skills/polish/SKILL.md';
    assert.match(fs.readFileSync(path.join(result.projectRoot, provider, pin), 'utf8'), /User changed this pinned command/);
  }
  assert.match(JSON.stringify(json(path.join(result.projectRoot, '.codex/hooks.json'))), /user wrapper/);
  assert.match(fs.readFileSync(path.join(result.projectRoot, 'PRODUCT.md'), 'utf8'), /keep these original bytes/);
});

test('copied and linked old skills have actual separate sources; personal discovery stays in the sandbox', async t => {
  const { materializeMigrationFixture } = await modulePromise;
  const input = smallPackage(t);
  for (const scenario of ['copied-skill', 'symlink-skill']) {
    const result = materializeMigrationFixture({ ...input, fixtureRoot: path.join(input.directory, scenario),
      scenarios: [scenario, 'personal-shadow'] });
    const installed = path.join(result.projectRoot, '.claude/skills/impeccable');
    const external = result.inventory.externalSkills[0].path;
    assert.equal(fs.lstatSync(installed).isSymbolicLink(), scenario === 'symlink-skill');
    assert.notEqual(installed, external);
    assert.equal(hash(fs.readFileSync(path.join(installed, 'SKILL.md'))), hash(fs.readFileSync(path.join(external, 'SKILL.md'))));
    assert.equal(result.inventory.personalSkills.length, 1);
    assert.ok(result.inventory.personalSkills[0].path.startsWith(result.homeRoot + path.sep));
    assert.ok(fs.existsSync(path.join(result.inventory.personalSkills[0].path, 'SKILL.md')));
  }
  const overridden = materializeMigrationFixture({ ...input, fixtureRoot: path.join(input.directory, 'home-layouts'),
    providers: ['.agent', '.pi', '.opencode'], scenarios: ['personal-shadow'] });
  for (const rel of ['.gemini/config/skills/impeccable', '.pi/agent/skills/impeccable', '.config/opencode/skills/impeccable']) {
    assert.ok(fs.existsSync(path.join(overridden.homeRoot, rel, 'SKILL.md')), rel);
  }
});

test('external auxiliary non-executable skills support skills-only, copies and modified symlinks', async t => {
  const { materializeMigrationFixture } = await modulePromise;
  const input = smallPackage(t);
  for (const channel of ['skills-only', 'xgent-installer']) {
    const result = materializeMigrationFixture({ ...input, fixtureRoot: path.join(input.directory, channel), channel,
      providers: ['.claude', '.agents'], scenarios: ['additional-skills'] });
    for (const provider of result.providers) {
      for (const [rel, expected] of Object.entries(input.additionalSource.files)) assert.equal(hash(fs.readFileSync(path.join(result.projectRoot, provider, 'skills', rel))), expected);
    }
    assert.equal(result.inventory.hookFiles.length === 0, channel === 'skills-only');
    assert.equal(fs.existsSync(path.join(result.projectRoot, '.claude/skills/impeccable')), channel !== 'skills-only');
  }
  const linked = materializeMigrationFixture({ ...input, fixtureRoot: path.join(input.directory, 'linked-aux'),
    scenarios: ['symlink-auxiliary', 'modified-auxiliary', 'personal-shadow'] });
  const installed = path.join(linked.projectRoot, '.claude/skills/review-ui');
  assert.equal(fs.lstatSync(installed).isSymbolicLink(), true);
  assert.match(fs.readFileSync(path.join(installed, 'SKILL.md'), 'utf8'), /User edited this external auxiliary skill/);
  assert.notEqual(hash(fs.readFileSync(path.join(installed, 'SKILL.md'))), input.additionalSource.files['review-ui/SKILL.md']);
});

test('legacy markers, frozen pin templates and stale live sources are materialized without launching a process', async t => {
  const { materializeMigrationFixture } = await modulePromise;
  const input = smallPackage(t);
  const result = materializeMigrationFixture({ ...input, fixtureRoot: path.join(input.directory, 'residue'),
    providers: ['.claude', '.agents', '.cursor', '.github', '.opencode'], scenarios: ['legacy-js-hooks', 'pin-shortcut', 'live-residue'] });
  const commands = result.inventory.hookFiles.map(rel => JSON.stringify(json(path.join(result.projectRoot, rel)))).join('\n');
  for (const marker of ['hook-probe.mjs', 'hook-after-edit.mjs', 'hook-stop.mjs', 'hook-before-edit.mjs', 'hook.mjs']) assert.ok(commands.includes(marker), marker);
  assert.equal(result.limitations.length, 1);
  const pins = json(path.join(root, 'test/phoenix-ui/fixtures/migration/pins.json'));
  assert.equal(fs.readFileSync(path.join(result.projectRoot, '.claude/skills/polish/SKILL.md'), 'utf8'), pins.templates.skill);
  assert.equal(fs.readFileSync(path.join(result.projectRoot, '.agents/skills/polish/SKILL.md'), 'utf8'), pins.templates.codexSkill);
  assert.equal(fs.readFileSync(path.join(result.projectRoot, '.opencode/commands/impeccable-polish.md'), 'utf8'), pins.templates.opencodeCommand);
  const html = fs.readFileSync(path.join(result.projectRoot, 'index.html'), 'utf8');
  for (const marker of ['impeccable-live-start', 'data-impeccable-csp-original', 'impeccable-variants-start ab12cd34']) assert.ok(html.includes(marker));
  const journal = json(path.join(result.projectRoot, '.impeccable/live/inject-journal.json'));
  assert.equal(journal.appRoot, result.projectRoot);
  assert.equal(journal.artifacts[0].patch, 'live-tag');
  assert.equal(json(path.join(result.projectRoot, '.impeccable/live/server.json')).port, 0);
  assert.ok(fs.existsSync(path.join(result.projectRoot, '.impeccable/live/sessions/ab12cd34.jsonl')));
});

test('damaged inputs, hook-only provider misuse and occupied destinations fail visibly', async t => {
  const { materializeMigrationFixture } = await modulePromise;
  const input = smallPackage(t);
  const destination = path.join(input.directory, 'occupied');
  fs.mkdirSync(destination);
  fs.writeFileSync(path.join(destination, 'keep.txt'), 'keep original');
  assert.throws(() => materializeMigrationFixture({ ...input, fixtureRoot: destination }), /not empty/);
  assert.equal(fs.readFileSync(path.join(destination, 'keep.txt'), 'utf8'), 'keep original');
  assert.throws(() => materializeMigrationFixture({ ...input, fixtureRoot: path.join(input.directory, 'codex'), providers: ['.codex'] }), /hook-only/);
  const blob = Object.values(input.source.providers['.claude'].files)[0];
  fs.appendFileSync(path.join(input.packagesDir, '0.6.0/package/vendor/impeccable/bundle/blobs', blob), 'damage');
  const pristine = path.join(input.directory, 'pristine');
  assert.throws(() => materializeMigrationFixture({ ...input, fixtureRoot: pristine }), /hash differs/);
  assert.equal(fs.existsSync(pristine), false);
});

// Explicit M1/M3 acceptance: missing historical packages fails, never silently skips.
if (process.env.PHOENIX_MIGRATION_REAL === '1') {
  test('verified published 0.3–0.6 bytes materialize all 76 provider recipes in every applicable channel', async t => {
    const { materializeMigrationFixture, loadMigrationSource, verifyPublishedPackage } = await modulePromise;
    const directory = sandbox(t);
    const packagesDir = process.env.PHOENIX_MIGRATION_PACKAGES || path.join(root, 'local/phoenix-ui/published');
    for (const version of versions) {
      const source = loadMigrationSource(version);
      verifyPublishedPackage(path.join(packagesDir, version, 'package'), source);
      for (const provider of Object.keys(source.providers)) {
        for (const channel of provider === '.codex' ? ['bundle'] : ['xgent-installer', 'bundle']) {
          const fixtureRoot = path.join(directory, `${version}-${provider}-${channel}`);
          const result = materializeMigrationFixture({ fixtureRoot, packagesDir, packageVersion: version, providers: [provider], channel });
          assertProviderBytes(result, source, channel);
          fs.rmSync(fixtureRoot, { recursive: true });
        }
      }
    }
  });

  test('real old packages produce user modifications, external skills, hooks and live residue for migration acceptance', async t => {
    const { materializeMigrationFixture } = await modulePromise;
    const directory = sandbox(t);
    for (const version of ['0.3.0', '0.6.0']) {
      const result = materializeMigrationFixture({ fixtureRoot: path.join(directory, version), packageVersion: version,
        providers: ['.claude', '.agents', '.cursor', '.github', '.grok', '.opencode'],
        scenarios: ['claude-shared-and-local', 'mixed-handlers', 'custom-allow', 'unrelated-entry', 'modified-skill',
          'modified-handler', 'symlink-skill', 'pin-shortcut', 'live-residue', 'additional-skills', 'personal-shadow'] });
      for (const rel of result.inventory.hookFiles) {
        const value = json(path.join(result.projectRoot, rel));
        assert.match(JSON.stringify(value.hooks), /user wrapper/);
        assert.match(JSON.stringify(value.hooks), /custom hook with spaces\.js/);
        assert.deepEqual(value.permissions.allow, ['Read', 'Bash(git status:*)']);
      }
      assert.ok(fs.lstatSync(path.join(result.projectRoot, '.claude/skills/impeccable')).isSymbolicLink());
      assert.match(fs.readFileSync(path.join(result.projectRoot, '.agents/skills/impeccable/SKILL.md'), 'utf8'), /user-edited old design skill/);
      assert.ok(result.inventory.externalSkills.some(skill => skill.name === 'review-ui'));
      assert.ok(fs.existsSync(path.join(result.projectRoot, '.impeccable/live/inject-journal.json')));
    }
  });
}
