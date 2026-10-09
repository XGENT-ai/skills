import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { sha256 } from '../../../../scripts/collect-phoenix-migration-sources.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const repository = path.resolve(here, '../../../..');
const sourcesDir = path.join(repository, 'tools/phoenix-ui/migration-sources');
const scenarioNames = new Set([
  'unmodified', 'modified-skill', 'copied-skill', 'symlink-skill', 'mixed-handlers', 'custom-allow',
  'modified-handler', 'legacy-js-hooks', 'pin-shortcut', 'modified-pin', 'live-residue',
  'personal-shadow', 'unknown-upstream', 'unrelated-entry', 'claude-shared-and-local',
  'additional-skills', 'modified-auxiliary', 'symlink-auxiliary', 'skills-only',
]);
const readJson = file => JSON.parse(fs.readFileSync(file, 'utf8'));
const clone = value => JSON.parse(JSON.stringify(value));

export function loadMigrationSource(version) {
  return readJson(path.join(sourcesDir, `${version}.json`));
}

function checkedBytes(file, expected) {
  const bytes = fs.readFileSync(file);
  if (sha256(bytes) !== expected) throw new Error(`Fixture input hash differs: ${file}`);
  return bytes;
}

export function verifyPublishedPackage(pkg, source) {
  for (const [rel, expected] of Object.entries({ 'bin/xgent-skills.js': source.installerSha256,
    'vendor/impeccable/VERSION.json': source.versionSha256,
    'vendor/impeccable/bundle/manifest.json': source.bundleManifestSha256 })) checkedBytes(path.join(pkg, rel), expected);
  if (readJson(path.join(pkg, 'package.json')).version !== source.packageVersion) throw new Error('Fixture package version differs');
  const manifest = readJson(path.join(pkg, 'vendor/impeccable/bundle/manifest.json'));
  const blobs = new Set(Object.values(manifest.providers).flatMap(provider => Object.values(provider.files)));
  for (const hash of blobs) checkedBytes(path.join(pkg, 'vendor/impeccable/bundle/blobs', hash), hash);
  for (const [rel, hash] of Object.entries(source.xgent?.hookFiles || {})) checkedBytes(path.join(pkg, rel), hash);
  return { providers: Object.keys(manifest.providers).length, blobs: blobs.size };
}

function additionalBytes(source, rel, directory) {
  if (directory) return checkedBytes(path.join(directory, rel), source.files[rel]);
  const result = spawnSync('git', ['show', `${source.sourceCommit}:skills/${rel}`], { cwd: repository, maxBuffer: 8 * 1024 * 1024 });
  if (result.error || result.status !== 0) throw new Error(`Missing fixed external skill ${source.sourceCommit}:skills/${rel}; provide additionalSkillsDir`);
  if (sha256(result.stdout) !== source.files[rel]) throw new Error(`External skill source differs: ${rel}`);
  return result.stdout;
}

function editHandlers(value, edit) {
  if (Array.isArray(value)) value.forEach(item => editHandlers(item, edit));
  else if (value && typeof value === 'object') {
    for (const key of ['command', 'commandWindows', 'bash', 'powershell']) {
      if (typeof value[key] === 'string') value[key] = edit(value[key], key);
    }
    for (const [key, item] of Object.entries(value)) {
      if (!['command', 'commandWindows', 'bash', 'powershell'].includes(key)) editHandlers(item, edit);
    }
  }
}

function addNeighbors(value) {
  const handler = { type: 'command', command: 'node "custom hook with spaces.js"', timeout: 7,
    fixtureUnknown: { keep: true } };
  for (const entries of Object.values(value.hooks || {})) {
    for (const entry of entries) {
      entry.fixtureMatcherNote = 'preserve this surrounding entry';
      if (Array.isArray(entry.hooks)) entry.hooks.push(clone(handler));
    }
    if (entries.some(entry => !Array.isArray(entry.hooks))) entries.push(clone(handler));
  }
}

// Fixed engine's default user layouts, without process-env overrides.
function personalSkillsRelative(provider) {
  if (provider === '.agent') return '.gemini/config/skills';
  if (provider === '.pi') return '.pi/agent/skills';
  if (provider === '.opencode') return '.config/opencode/skills';
  return `${provider}/skills`;
}

function liveResidue(project, write) {
  const id = 'ab12cd34';
  const originalCsp = "default-src 'self'; script-src 'self'";
  write('index.html', `<html><head><meta http-equiv="Content-Security-Policy" content="${originalCsp} http://localhost:8412 'wasm-unsafe-eval'" data-impeccable-csp-original="${Buffer.from(originalCsp).toString('base64')}"></head><body>\n<!-- impeccable-live-start -->\n<script src="http://localhost:8412/live.js?token=fixture-only"></script>\n<!-- impeccable-live-end -->\n<div data-impeccable-variants="${id}" data-impeccable-variant-count="1">\n<!-- impeccable-variants-start ${id} -->\n<div data-impeccable-variant="original">Original</div>\n<!-- Variants: insert below this line -->\n<div data-impeccable-variant="1">Variant</div>\n<!-- impeccable-variants-end ${id} -->\n</div></body></html>\n`);
  write('.impeccable/live/config.json', JSON.stringify({ files: ['index.html'], insertBefore: '</body>', commentSyntax: 'html', cspChecked: true }));
  write('.impeccable/live/inject-journal.json', JSON.stringify({ version: 1, appRoot: project,
    framework: 'static-html', port: 8412, pid: 2147483647, recordedAt: '2026-10-08T00:00:00.000Z',
    artifacts: [{ kind: 'patched', path: 'index.html', patch: 'live-tag', markers: ['impeccable-live-start', 'data-impeccable-csp-original'] }] }));
  // Deliberately stale; this fixture starts no process and gives no permission to kill one.
  write('.impeccable/live/server.json', JSON.stringify({ pid: 2147483647, port: 0, token: 'fixture-only' }));
  write('.impeccable-live.json', JSON.stringify({ pid: 2147483647, port: 0, token: 'legacy-fixture-only' }));
  write(`.impeccable/live/sessions/${id}.jsonl`, JSON.stringify({ seq: 1, id, type: 'generate',
    ts: '2026-10-08T00:00:00.000Z', event: { id, type: 'generate', count: 1, file: 'index.html' } }) + '\n');
}

/** Reconstruct bytes in an empty sandbox; no installer, provider or engine is executed. */
export function materializeMigrationFixture({ fixtureRoot, packageVersion = '0.6.0', providers = ['.claude'],
  packagesDir = process.env.PHOENIX_MIGRATION_PACKAGES || path.join(repository, 'local/phoenix-ui/published'),
  channel = 'xgent-installer', scenarios = [], source = loadMigrationSource(packageVersion),
  additionalSource = readJson(path.join(here, 'additional-skills.json')), additionalSkillsDir,
  pins = readJson(path.join(here, 'pins.json')) } = {}) {
  if (!fixtureRoot) throw new Error('fixtureRoot must be an empty sandbox directory');
  const sandbox = path.resolve(fixtureRoot);
  if (fs.existsSync(sandbox) && fs.readdirSync(sandbox).length) throw new Error(`Fixture sandbox is not empty: ${sandbox}`);
  if (!['xgent-installer', 'bundle', 'skills-only'].includes(channel)) throw new Error(`Unknown fixture channel: ${channel}`);
  const selected = [...new Set(providers)];
  if (!selected.length || selected.some(provider => !source.providers[provider])) throw new Error('Unknown or empty fixture providers');
  if (channel === 'xgent-installer' && selected.some(provider => source.providers[provider].installerSelectable === false)) {
    throw new Error('.codex is hook-only in the published npm installer; select .agents or use channel=bundle');
  }
  const active = new Set(scenarios);
  for (const scenario of active) if (!scenarioNames.has(scenario)) throw new Error(`Unknown fixture scenario: ${scenario}`);
  const skillsOnly = channel === 'skills-only' || active.has('skills-only');
  const pkg = path.resolve(packagesDir, packageVersion, 'package');
  if (!skillsOnly) {
    if (!fs.existsSync(pkg)) throw new Error(`Prepare verified old packages in ${packagesDir}; see tools/phoenix-ui/migration-fixtures.md`);
    verifyPublishedPackage(pkg, source);
  }
  const projectRoot = path.join(sandbox, 'project with spaces');
  const homeRoot = path.join(sandbox, 'user home');
  const externalRoot = path.join(sandbox, 'skills-sh source');
  const inventory = { projectFiles: {}, hookFiles: [], externalSkills: [], personalSkills: [] };
  function put(base, rel, bytes, mode = 0o644) {
    const file = path.join(base, rel);
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, bytes, { mode });
    fs.chmodSync(file, mode);
    if (base === projectRoot) inventory.projectFiles[rel] = sha256(bytes);
    return file;
  }
  const write = (rel, bytes, mode) => put(projectRoot, rel, bytes, mode);
  fs.mkdirSync(homeRoot, { recursive: true });
  write('package.json', JSON.stringify({ name: 'migration-fixture', private: true }) + '\n');
  write('PRODUCT.md', '# Product\n\nUser-authored requirements: keep these original bytes.\n');
  write('DESIGN.md', '# Design\n\nUser-authored rules: keep these original bytes.\n');
  write('AGENTS.md', '# Project instructions\n\nUse /impeccable polish for existing UI.\n');
  write('CLAUDE.md', '# Project notes\n\nThe old design entry is /impeccable.\n');
  for (const provider of selected) write(`${provider}/skills/user-helper/SKILL.md`, '---\nname: user-helper\n---\nUser-authored skill; never replace or remove.\n');

  if (!skillsOnly && channel === 'xgent-installer') {
    for (const [rel, hash] of Object.entries(source.xgent?.hookFiles || {})) write(rel, checkedBytes(path.join(pkg, rel), hash), 0o755);
    write('.claude/settings.json', JSON.stringify({ ...clone(source.xgent?.managedSettings || {}), fixtureSharedSetting: 'retain' }, null, 2) + '\n');
  }
  for (const provider of selected) {
    const entry = source.providers[provider];
    if (skillsOnly) continue;
    for (const [rel, hash] of Object.entries(entry.files)) {
      if (channel === 'xgent-installer' && !rel.startsWith('skills/') && !/^(agents|commands)\/[^/]+$/.test(rel)) continue;
      let executable = entry.exec?.includes(rel);
      if (channel === 'xgent-installer' && (/^skills\/[^/]+\/scripts\/impeccable(?:\.cmd)?$/.test(rel) || /^skills\/[^/]+\/scripts\/bin\//.test(rel))) executable = true;
      write(`${provider}/${rel}`, checkedBytes(path.join(pkg, 'vendor/impeccable/bundle/blobs', hash), hash), executable ? 0o755 : 0o644);
    }
    const hook = entry.hooks.find(item => item.shape === channel);
    // Universal .agents has no hook file; its raw Codex manifest belongs to .codex.
    if (!hook || (channel === 'bundle' && (!entry.hookArtifact.source.startsWith(`${provider}/`)
      || !entry.files[entry.hookArtifact.source.slice(provider.length + 1)]))) continue;
    const rel = channel === 'bundle' ? entry.hookArtifact.source : entry.hookArtifact.destination;
    write(rel, JSON.stringify(hook.value, null, 2) + '\n');
    inventory.hookFiles.push(rel);
  }

  const needsHooks = ['mixed-handlers', 'custom-allow', 'modified-handler', 'legacy-js-hooks', 'unrelated-entry', 'claude-shared-and-local'].some(name => active.has(name));
  if (needsHooks && !inventory.hookFiles.length) throw new Error('Selected fixture providers have no hook manifest');
  if (active.has('claude-shared-and-local')) {
    if (!selected.includes('.claude')) throw new Error('claude-shared-and-local requires .claude');
    const entry = source.providers['.claude'];
    const fresh = clone(entry.hooks.find(item => item.shape === 'xgent-installer').value);
    for (const rel of [entry.hookArtifact.shared, entry.hookArtifact.destination]) {
      const value = fs.existsSync(path.join(projectRoot, rel)) ? readJson(path.join(projectRoot, rel)) : {};
      value.hooks = fresh.hooks;
      value.fixtureScope = rel.endsWith('settings.local.json') ? 'local' : 'shared';
      write(rel, JSON.stringify(value, null, 2) + '\n');
      if (!inventory.hookFiles.includes(rel)) inventory.hookFiles.push(rel);
    }
  }
  for (const rel of inventory.hookFiles) {
    const value = readJson(path.join(projectRoot, rel));
    if (active.has('legacy-js-hooks')) {
      for (const [event, entries] of Object.entries(value.hooks)) {
        editHandlers(entries, command => {
          const provider = command.includes('.agents/skills/') ? '.agents' : rel.split('/')[0];
          const prefix = provider === '.claude' ? '${CLAUDE_PROJECT_DIR}/' : provider === '.github' ? '$(git rev-parse --show-toplevel)/' : '';
          const script = event === 'SessionStart' ? 'hook-probe.mjs' : event === 'Stop' ? 'hook-stop.mjs'
            : event === 'preToolUse' ? 'hook-before-edit.mjs' : event === 'PostToolUse' ? 'hook-after-edit.mjs' : 'hook.mjs';
          return `node "${prefix}${provider}/skills/impeccable/scripts/${script}"`;
        });
      }
    }
    if (active.has('mixed-handlers')) addNeighbors(value);
    if (active.has('custom-allow')) value.permissions = { allow: ['Read', 'Bash(git status:*)'], fixturePolicy: 'keep' };
    if (active.has('unrelated-entry')) {
      value.fixtureUnknown = { arbitrary: ['preserve', 42] };
      const event = Object.keys(value.hooks)[0];
      value.hooks[event].push({ type: 'command', command: 'echo "impeccable notes are user data"', fixtureUnrelated: true });
    }
    if (active.has('modified-handler')) {
      let changed = false;
      editHandlers(value.hooks, command => {
        if (changed) return command;
        changed = true;
        return `echo "user wrapper" && ${command}`;
      });
    }
    write(rel, JSON.stringify(value, null, 2) + '\n');
  }

  for (const provider of selected) {
    const skill = `${provider}/skills/impeccable`;
    if (!skillsOnly && active.has('modified-skill')) fs.appendFileSync(path.join(projectRoot, skill, 'SKILL.md'), '\n<!-- user-edited old design skill -->\n');
    if (!skillsOnly && active.has('unknown-upstream')) fs.appendFileSync(path.join(projectRoot, skill, 'scripts/impeccable'), '\n# unsupported upstream launcher bytes\n');
    if (!skillsOnly && (active.has('copied-skill') || active.has('symlink-skill'))) {
      const external = path.join(externalRoot, provider, 'impeccable');
      fs.mkdirSync(path.dirname(external), { recursive: true });
      fs.cpSync(path.join(projectRoot, skill), external, { recursive: true });
      if (active.has('symlink-skill')) {
        fs.rmSync(path.join(projectRoot, skill), { recursive: true });
        fs.symlinkSync(process.platform === 'win32' ? external : path.relative(path.dirname(path.join(projectRoot, skill)), external), path.join(projectRoot, skill), process.platform === 'win32' ? 'junction' : 'dir');
      }
      inventory.externalSkills.push({ provider, name: 'impeccable', path: external, installation: active.has('symlink-skill') ? 'symlink' : 'copy' });
    }
    if (!skillsOnly && (active.has('pin-shortcut') || active.has('modified-pin'))) {
      const codex = provider === '.agents' || provider === '.codex';
      const pin = provider === '.opencode' ? `${provider}/commands/impeccable-polish.md` : `${provider}/skills/polish/SKILL.md`;
      const bytes = provider === '.opencode' ? pins.templates.opencodeCommand : codex ? pins.templates.codexSkill : pins.templates.skill;
      write(pin, bytes + (active.has('modified-pin') ? '\nUser changed this pinned command.\n' : ''));
    }
    if (!skillsOnly && active.has('personal-shadow')) {
      const personal = path.join(homeRoot, personalSkillsRelative(provider), 'impeccable');
      fs.mkdirSync(path.dirname(personal), { recursive: true });
      fs.cpSync(path.join(projectRoot, skill), personal, { recursive: true, dereference: true });
      inventory.personalSkills.push({ provider, name: 'impeccable', path: personal });
    }
  }
  const extra = skillsOnly || ['additional-skills', 'modified-auxiliary', 'symlink-auxiliary'].some(name => active.has(name));
  if (extra) {
    for (const provider of selected) {
      for (const [rel, hash] of Object.entries(additionalSource.files)) {
        const bytes = additionalBytes(additionalSource, rel, additionalSkillsDir);
        put(externalRoot, rel, bytes);
        write(`${provider}/skills/${rel}`, bytes);
        if (sha256(bytes) !== hash) throw new Error(`Additional source differs: ${rel}`);
      }
      const names = [...new Set(Object.keys(additionalSource.files).map(rel => rel.split('/')[0]))];
      for (const name of names) {
        const rel = `${provider}/skills/${name}`;
        if (active.has('symlink-auxiliary')) {
          const external = path.join(externalRoot, name);
          fs.rmSync(path.join(projectRoot, rel), { recursive: true });
          fs.symlinkSync(process.platform === 'win32' ? external : path.relative(path.dirname(path.join(projectRoot, rel)), external), path.join(projectRoot, rel), process.platform === 'win32' ? 'junction' : 'dir');
        }
        if (active.has('modified-auxiliary')) fs.appendFileSync(path.join(projectRoot, rel, 'SKILL.md'), '\nUser edited this external auxiliary skill.\n');
        inventory.externalSkills.push({ provider, name, path: path.join(externalRoot, name), installation: active.has('symlink-auxiliary') ? 'symlink' : 'copy', sourceCommit: additionalSource.sourceCommit });
        if (active.has('personal-shadow')) {
          const personal = path.join(homeRoot, personalSkillsRelative(provider), name);
          fs.mkdirSync(path.dirname(personal), { recursive: true });
          fs.cpSync(path.join(projectRoot, rel), personal, { recursive: true, dereference: true });
          inventory.personalSkills.push({ provider, name, path: personal });
        }
      }
    }
  }
  if (active.has('live-residue')) liveResidue(projectRoot, write);
  for (const rel of Object.keys(inventory.projectFiles)) inventory.projectFiles[rel] = sha256(fs.readFileSync(path.join(projectRoot, rel)));
  const result = { schemaVersion: 1, packageVersion, channel: skillsOnly ? 'skills-only' : channel,
    scenarios: [...active], providers: selected, projectRoot, homeRoot, externalRoot, inventory,
    limitations: active.has('legacy-js-hooks') ? ['Synthetic commands use installer-recognized JS markers; released JS script bodies are unavailable and are not claimed.'] : [] };
  put(sandbox, 'fixture.json', JSON.stringify(result, null, 2) + '\n');
  return result;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [destination, version = '0.6.0', providerList = '.claude', scenarioList = 'unmodified', channel = 'xgent-installer'] = process.argv.slice(2);
  if (!destination) throw new Error('Usage: node test/phoenix-ui/fixtures/migration/materialize.mjs EMPTY_DIR [VERSION] [PROVIDERS] [SCENARIOS] [CHANNEL]');
  const result = materializeMigrationFixture({ fixtureRoot: destination, packageVersion: version,
    providers: providerList.split(','), scenarios: scenarioList.split(','), channel });
  console.log(JSON.stringify({ projectRoot: result.projectRoot, homeRoot: result.homeRoot,
    fixture: path.join(path.resolve(destination), 'fixture.json'), projectFiles: Object.keys(result.inventory.projectFiles).length }));
}
