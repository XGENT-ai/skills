#!/usr/bin/env bun
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { readSourceFiles } from './lib/utils.js';
import { createTransformer } from './lib/transformers/factory.js';
import { PROVIDERS } from './lib/transformers/providers.js';

const workspace = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const repository = path.resolve(workspace, '../..');
const source = path.join(repository, 'skills/phoenix-ui/src');
const runtimeManifestPath = 'skills/phoenix-ui/scripts/ENGINE.json';
const sha = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const json = value => JSON.stringify(value, null, 2) + '\n';

function filesIn(root) {
  const files = [];
  if (!fs.existsSync(root)) return files;
  const walk = (dir) => {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true }).sort((a, b) => a.name < b.name ? -1 : 1)) {
      const file = path.join(dir, entry.name);
      if (entry.isDirectory()) walk(file);
      else {
        assert(entry.isFile(), `Generated input/output must be a regular file: ${file}`);
        files.push(path.relative(root, file).split(path.sep).join('/'));
      }
    }
  };
  walk(root);
  return files;
}

function tree(root) {
  return Object.fromEntries(filesIn(root).map(relative => [relative, sha(fs.readFileSync(path.join(root, relative)))]));
}

function inputs() {
  const paths = filesIn(source).map(file => `skills/phoenix-ui/src/${file}`);
  paths.push('tools/phoenix-ui/scripts/render.mjs', 'tools/phoenix-ui/scripts/lib/utils.js',
    'tools/phoenix-ui/scripts/lib/skill-categories.js',
    'tools/phoenix-ui/scripts/lib/transformers/factory.js',
    'tools/phoenix-ui/scripts/lib/transformers/providers.js',
    'tools/phoenix-ui/scripts/lib/transformers/hooks.js',
    'tools/phoenix-ui/crates/hook/src/missing_launcher.cjs',
    'tools/phoenix-ui/LICENSE', 'tools/phoenix-ui/NOTICE.md',
    'tools/phoenix-ui/build-tools.lock.json', 'tools/phoenix-ui/mise.toml',
    'tools/phoenix-ui/package.json', 'tools/phoenix-ui/bun.lock');
  const hashes = Object.fromEntries(paths.sort().map(file => [file, sha(fs.readFileSync(path.join(repository, file)))]));
  return { hashes, digest: sha(JSON.stringify(hashes)) };
}

function render(stage, input) {
  const { skills } = readSourceFiles(workspace);
  assert.equal(skills.length, 1, 'Phoenix has exactly one source skill');
  assert.equal(skills[0].name, 'phoenix-ui');
  assert.equal(skills[0].internal, true);
  skills[0].metadata = { version: fs.readFileSync(path.join(source, 'scripts/VERSION'), 'utf8').trim(),
    source: 'skills/phoenix-ui/src', source_sha256: input.digest };
  const version = skills[0].metadata.version;
  const dist = path.join(stage, 'dist');
  for (const config of Object.values(PROVIDERS)) createTransformer(config)(skills, dist, { skillsVersion: version });
  createTransformer({ provider: 'generic', configDir: '', displayName: 'Generic',
    providerTags: ['codex'], placeholderProvider: 'codex', scriptsPath: '<skill-base-dir>/scripts',
    frontmatterFields: ['internal', 'license', 'metadata'], versionInMetadata: true, writeOpenAIMetadata: true,
  })(skills, dist, { skillsVersion: version });
  const generic = path.join(stage, 'skill');
  fs.cpSync(path.join(dist, 'generic/skills/phoenix-ui'), generic, { recursive: true });
  fs.rmSync(path.join(dist, 'generic'), { recursive: true });
  const vendor = path.join(stage, 'vendor');
  const blobs = path.join(vendor, 'bundle/blobs');
  fs.mkdirSync(blobs, { recursive: true });
  const providers = {};
  for (const config of Object.values(PROVIDERS).sort((a, b) => a.configDir < b.configDir ? -1 : 1)) {
    const root = path.join(dist, config.provider, config.configDir);
    const files = {}, exec = [];
    for (const relative of filesIn(root)) {
      // Codex hooks live in .codex; the runtime skill is installed in .agents.
      if (config.provider === 'codex' && relative.startsWith('skills/')) continue;
      assert.notEqual(relative, runtimeManifestPath, 'ENGINE is derived after bundle hashing');
      const bytes = fs.readFileSync(path.join(root, relative)), hash = sha(bytes);
      files[relative] = hash;
      fs.writeFileSync(path.join(blobs, hash), bytes);
      if (relative.endsWith('/scripts/phoenix-ui')) exec.push(relative);
    }
    providers[config.configDir] = { files, exec };
  }
  const manifest = { schema: 1, derivedRuntimeManifest: runtimeManifestPath, providers };
  const bytes = Buffer.from(json(manifest));
  fs.writeFileSync(path.join(vendor, 'bundle/manifest.json'), bytes);
  for (const file of ['LICENSE', 'NOTICE.md']) fs.copyFileSync(path.join(workspace, file), path.join(vendor, file));
  return { generic, dist, vendor, bundleSha256: sha(bytes), version };
}

function addEngineManifest(output, withoutEngine) {
  if (withoutEngine) return;
  const file = path.join(repository, 'vendor/phoenix-ui/VERSION.json');
  // Use the same consumer validator, including all-platform requirements.
  const { manifestAt } = require(path.join(source, 'scripts/phoenix-bootstrap.cjs'));
  const manifest = manifestAt(file);
  assert.equal(manifest.data.bundleSha256, output.bundleSha256, 'VERSION refers to stale provider sources; rebuild the package');
  assert.equal(manifest.data.toolVersion, output.version);
  fs.copyFileSync(file, path.join(output.vendor, 'VERSION.json'));
  const dirs = [output.generic, ...Object.values(PROVIDERS).map(config =>
    path.join(output.dist, config.provider, config.configDir, 'skills/phoenix-ui'))];
  for (const dir of dirs) fs.writeFileSync(path.join(dir, 'scripts/ENGINE.json'), manifest.bytes);
}

function verifyContent(output) {
  for (const root of [output.generic, output.dist]) {
    for (const file of filesIn(root).filter(file => /\.(?:md|yaml|toml)$/.test(file))) {
      const content = fs.readFileSync(path.join(root, file), 'utf8');
      assert(!/\{\{(?:model|config_file|ask_instruction|command_prefix|available_commands|command_hint|scripts_path|reference_path)\}\}/.test(content), `Unrendered placeholder: ${file}`);
      assert(!/^\s*<\/?(?:codex|claude|claude-code|cursor|agents|github|veto)>\s*$/m.test(content), `Unrendered provider block: ${file}`);
    }
  }
  assert.deepEqual(fs.readdirSync(path.join(output.generic, 'agents')), ['openai.yaml']);
  assert.equal(filesIn(path.join(output.generic, 'subagents/codex')).length, 4);
}

function generatedRecord(output, input, withoutEngine) {
  return { schemaVersion: 1, generator: 'Bun 1.3.14', inputSha256: input.digest, inputs: input.hashes,
    bundleSha256: output.bundleSha256, engineManifest: withoutEngine ? 'not-generated' : 'verified',
    outputs: { skill: tree(output.generic), providers: tree(output.dist), bundle: tree(path.join(output.vendor, 'bundle')),
      notices: Object.fromEntries(['LICENSE', 'NOTICE.md'].map(file => [file, sha(fs.readFileSync(path.join(output.vendor, file)))])) } };
}

function publish(output, target) {
  fs.mkdirSync(target.skill, { recursive: true });
  for (const name of ['SKILL.md', 'reference', 'scripts', 'agents', 'subagents']) {
    fs.rmSync(path.join(target.skill, name), { recursive: true, force: true });
    fs.cpSync(path.join(output.generic, name), path.join(target.skill, name), { recursive: true });
  }
  fs.copyFileSync(path.join(output.generic, 'GENERATED.json'), path.join(target.skill, 'GENERATED.json'));
  fs.rmSync(target.dist, { recursive: true, force: true });
  fs.cpSync(output.dist, target.dist, { recursive: true });
  fs.mkdirSync(target.vendor, { recursive: true });
  fs.rmSync(path.join(target.vendor, 'bundle'), { recursive: true, force: true });
  fs.cpSync(path.join(output.vendor, 'bundle'), path.join(target.vendor, 'bundle'), { recursive: true });
  for (const file of ['LICENSE', 'NOTICE.md']) fs.copyFileSync(path.join(output.vendor, file), path.join(target.vendor, file));
  if (fs.existsSync(path.join(output.vendor, 'VERSION.json'))) {
    fs.copyFileSync(path.join(output.vendor, 'VERSION.json'), path.join(target.vendor, 'VERSION.json'));
  }
}

async function main() {
  assert.equal(process.versions.bun, '1.3.14', 'Render with the pinned Bun 1.3.14');
  const args = process.argv.slice(2);
  let check = false, withoutEngine = false, outputRoot;
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--check') check = true;
    else if (args[i] === '--without-engine') withoutEngine = true;
    else if (args[i] === '--output-root' && args[i + 1]) outputRoot = path.resolve(args[++i]);
    else throw new Error('Usage: bun scripts/render.mjs [--check] [--without-engine] [--output-root DIR]');
  }
  assert(!(check && withoutEngine), 'Freshness checks require a verified engine manifest');
  const target = outputRoot ? { skill: path.join(outputRoot, 'skill'), dist: path.join(outputRoot, 'dist'), vendor: path.join(outputRoot, 'vendor') }
    : { skill: path.join(repository, 'skills/phoenix-ui'), dist: path.join(workspace, 'dist/providers'), vendor: path.join(repository, 'vendor/phoenix-ui') };
  const stage = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-render-'));
  try {
    const input = inputs(), output = render(stage, input);
    addEngineManifest(output, withoutEngine);
    verifyContent(output);
    const record = generatedRecord(output, input, withoutEngine);
    fs.writeFileSync(path.join(output.generic, 'GENERATED.json'), json(record));
    assert.equal(inputs().digest, input.digest, 'Source changed during rendering');
    if (check) {
      assert.deepEqual(JSON.parse(fs.readFileSync(path.join(target.skill, 'GENERATED.json'))), record, 'Generated sources are stale');
      const current = Object.fromEntries(Object.entries(tree(target.skill)).filter(([file]) => file !== 'GENERATED.json' && !file.startsWith('src/') && file !== '.npmignore'));
      assert.deepEqual(current, record.outputs.skill, 'Generated skill differs from its source');
      assert.deepEqual(tree(target.dist), record.outputs.providers, 'Generated provider files are stale');
      assert.deepEqual(tree(path.join(target.vendor, 'bundle')), record.outputs.bundle, 'Generated bundle is stale');
      for (const [file, hash] of Object.entries(record.outputs.notices)) {
        assert.equal(sha(fs.readFileSync(path.join(target.vendor, file))), hash, `Generated notice differs: ${file}`);
      }
    } else publish(output, target);
    console.log(json({ status: check ? 'fresh' : withoutEngine ? 'rendered-without-engine' : 'rendered', providers: 19,
      inputSha256: input.digest, bundleSha256: output.bundleSha256 }));
  } finally { fs.rmSync(stage, { recursive: true, force: true }); }
}

await main();
