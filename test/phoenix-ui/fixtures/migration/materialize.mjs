// Rebuilds a project as a published Impeccable-era @xgent-ai/skills release
// installed it, for the release check's coexistence and replacement groups.
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../../..');
const sourcesDir = path.join(repository, 'tools/phoenix-ui/migration-sources');
const scenarioNames = new Set(['custom-allow', 'live-residue', 'personal-shadow', 'unrelated-entry', 'claude-shared-and-local']);
const sha256 = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
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
export function materializeMigrationFixture({ fixtureRoot, packageVersion, providers, packagesDir, scenarios = [] }) {
  const source = loadMigrationSource(packageVersion);
  const sandbox = path.resolve(fixtureRoot);
  if (fs.existsSync(sandbox) && fs.readdirSync(sandbox).length) throw new Error(`Fixture sandbox is not empty: ${sandbox}`);
  const selected = [...new Set(providers)];
  if (!selected.length || selected.some(provider => !source.providers[provider])) throw new Error('Unknown or empty fixture providers');
  if (selected.some(provider => source.providers[provider].installerSelectable === false)) throw new Error('.codex is hook-only in the published npm installer; select .agents');
  const active = new Set(scenarios);
  for (const scenario of active) if (!scenarioNames.has(scenario)) throw new Error(`Unknown fixture scenario: ${scenario}`);
  const pkg = path.resolve(packagesDir, packageVersion, 'package');
  verifyPublishedPackage(pkg, source);
  const projectRoot = path.join(sandbox, 'project with spaces');
  const homeRoot = path.join(sandbox, 'user home');
  const inventory = { projectFiles: {}, hookFiles: [], personalSkills: [] };
  function write(rel, bytes, mode = 0o644) {
    const file = path.join(projectRoot, rel);
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, bytes, { mode });
    fs.chmodSync(file, mode);
    inventory.projectFiles[rel] = sha256(bytes);
  }
  fs.mkdirSync(homeRoot, { recursive: true });
  write('package.json', JSON.stringify({ name: 'migration-fixture', private: true }) + '\n');
  write('PRODUCT.md', '# Product\n\nUser-authored requirements: keep these original bytes.\n');
  write('DESIGN.md', '# Design\n\nUser-authored rules: keep these original bytes.\n');
  write('AGENTS.md', '# Project instructions\n\nUse /impeccable polish for existing UI.\n');
  write('CLAUDE.md', '# Project notes\n\nThe old design entry is /impeccable.\n');
  for (const provider of selected) write(`${provider}/skills/user-helper/SKILL.md`, '---\nname: user-helper\n---\nUser-authored skill; never replace or remove.\n');
  for (const [rel, hash] of Object.entries(source.xgent?.hookFiles || {})) write(rel, checkedBytes(path.join(pkg, rel), hash), 0o755);
  write('.claude/settings.json', JSON.stringify({ ...clone(source.xgent?.managedSettings || {}), fixtureSharedSetting: 'retain' }, null, 2) + '\n');
  for (const provider of selected) {
    const entry = source.providers[provider];
    for (const [rel, hash] of Object.entries(entry.files)) {
      if (!rel.startsWith('skills/') && !/^(agents|commands)\/[^/]+$/.test(rel)) continue;
      const executable = entry.exec?.includes(rel) || /^skills\/[^/]+\/scripts\/impeccable(?:\.cmd)?$/.test(rel) || /^skills\/[^/]+\/scripts\/bin\//.test(rel);
      write(`${provider}/${rel}`, checkedBytes(path.join(pkg, 'vendor/impeccable/bundle/blobs', hash), hash), executable ? 0o755 : 0o644);
    }
    const hook = entry.hooks.find(item => item.shape === 'xgent-installer');
    if (!hook) continue;
    write(entry.hookArtifact.destination, JSON.stringify(hook.value, null, 2) + '\n');
    inventory.hookFiles.push(entry.hookArtifact.destination);
  }
  if (active.has('claude-shared-and-local')) {
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
    if (active.has('custom-allow')) value.permissions = { allow: ['Read', 'Bash(git status:*)'], fixturePolicy: 'keep' };
    if (active.has('unrelated-entry')) {
      value.fixtureUnknown = { arbitrary: ['preserve', 42] };
      const event = Object.keys(value.hooks)[0];
      value.hooks[event].push({ type: 'command', command: 'echo "impeccable notes are user data"', fixtureUnrelated: true });
    }
    write(rel, JSON.stringify(value, null, 2) + '\n');
  }
  if (active.has('personal-shadow')) {
    for (const provider of selected) {
      const personal = path.join(homeRoot, personalSkillsRelative(provider), 'impeccable');
      fs.mkdirSync(path.dirname(personal), { recursive: true });
      fs.cpSync(path.join(projectRoot, provider, 'skills/impeccable'), personal, { recursive: true, dereference: true });
      inventory.personalSkills.push({ provider, name: 'impeccable', path: personal });
    }
  }
  if (active.has('live-residue')) liveResidue(projectRoot, write);
  for (const rel of Object.keys(inventory.projectFiles)) inventory.projectFiles[rel] = sha256(fs.readFileSync(path.join(projectRoot, rel)));
  return { schemaVersion: 1, packageVersion, scenarios: [...active], providers: selected, projectRoot, homeRoot, inventory };
}
