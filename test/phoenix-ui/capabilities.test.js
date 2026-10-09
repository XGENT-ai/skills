const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const test = require('node:test');
const { pathToFileURL } = require('node:url');

const workspace = path.resolve(__dirname, '../../tools/phoenix-ui');
const repository = path.resolve(workspace, '../..');
const inventory = () => JSON.parse(fs.readFileSync(path.join(workspace, 'capabilities.json'), 'utf8'));
const sourcePath = (file) => {
  const entry = inventory().current?.sourceFiles?.[file];
  return entry ? path.resolve(repository, entry.currentPath) : path.join(workspace, file);
};
const read = (file) => fs.readFileSync(sourcePath(file), 'utf8');
const sha256 = (value) => crypto.createHash('sha256').update(value).digest('hex');
const strings = (text) => [...text.matchAll(/"([^"\\]*(?:\\.[^"\\]*)*)"/g)].map((m) => JSON.parse(m[0]));
const sorted = (values) => [...new Set(values)].sort();

function arms(text) {
  return [...text.matchAll(/^\s*("[^"\n]*"(?:\s*\|\s*"[^"\n]*")*)\s*=>/gm)]
    .flatMap((m) => strings(m[1]));
}

function constant(text, name) {
  const match = text.match(new RegExp(`(?:pub )?const ${name}:[\\s\\S]*?=\\s*&?\\[([\\s\\S]*?)\\];`));
  assert.ok(match, `${name} must exist in the source`);
  return strings(match[1]);
}

function production(text) {
  // Some inline test modules precede more production functions. Keep both
  // sides and preserve line numbers while removing those top-level modules.
  return text.replace(/#\[cfg\(test\)\]\s*mod\s+\w+\s*\{[\s\S]*?^\}/gm,
    (module) => module.replace(/[^\n]/g, ' '));
}

function rustFiles(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const file = path.join(dir, entry.name);
    if (entry.isDirectory()) return rustFiles(file);
    return entry.name.endsWith('.rs') && !/(?:^|_)tests\.rs$/.test(entry.name) ? [file] : [];
  });
}

test('the capability ledger is anchored to the actual import manifest', () => {
  const cap = inventory();
  const upstream = JSON.parse(read('UPSTREAM.json'));
  const lexical = Object.fromEntries(Object.keys(upstream.importedFiles).sort().map((name) => [name, upstream.importedFiles[name]]));
  assert.equal(sha256(JSON.stringify(lexical)), upstream.importManifestSha256);
  assert.equal(cap.baseline.sourceCommit, upstream.sourceCommit);
  assert.equal(cap.baseline.engineCommit, upstream.engineCommit);
  assert.equal(cap.baseline.skillVersion, upstream.skillVersion);
  assert.equal(cap.baseline.engineVersion, upstream.engineVersion);
  assert.equal(sha256(`${cap.baseline.engineVersion}\n`), upstream.importedFiles.ENGINE_VERSION);
  assert.equal(cap.baseline.importManifestSha256, upstream.importManifestSha256);
  assert.equal(cap.baseline.importedFileCount, Object.keys(upstream.importedFiles).length);
  for (const [file, hash] of Object.entries(cap.sourceFiles)) {
    assert.equal(upstream.importedFiles[file], hash, `unanchored source: ${file}`);
    assert.ok(fs.existsSync(sourcePath(file)), `missing current source: ${file}`);
    const mapping = cap.current.sourceFiles[file];
    assert.equal(mapping.importedPath, `tools/phoenix-ui/${file}`);
    assert.equal(mapping.baselineSha256, hash);
    assert.equal(mapping.currentSha256, sha256(fs.readFileSync(sourcePath(file))), `stale current source: ${file}`);
  }
  for (const [file, entry] of Object.entries(cap.current.additionalSourceFiles)) {
    assert.equal(entry.sha256, sha256(fs.readFileSync(path.join(repository, file))), file);
    if (entry.importedPath) {
      assert.equal(entry.importedPath, file);
      assert.equal(entry.baselineSha256, upstream.importedFiles[file.slice('tools/phoenix-ui/'.length)]);
    }
  }
  assert.equal(cap.validation.runtime, '未验证');
  assert.equal(cap.validation.providerSessions, '未验证');
  assert.equal((read('compatibility.md').match(/# Phoenix UI 能力与差异账本/g) || []).length, 1);
});

test('root, live and skills routers have no missing or invented capability entries', () => {
  const cap = inventory();
  assert.deepEqual(sorted(cap.cli.root.flatMap((entry) => entry.names)), sorted(arms(read('crates/cli/src/main.rs').split('/// The npm')[0])));
  assert.deepEqual(sorted(cap.cli.live.flatMap((entry) => entry.names)), sorted(arms(read('crates/live/src/lib.rs'))));
  const skills = read('crates/skills/src/commands.rs').split('fn ctx(')[0];
  assert.deepEqual(sorted(cap.cli.skills.actions), sorted(arms(skills)));
  assert.deepEqual(sorted(cap.cli.ignores.actions), sorted(arms(read('crates/detect/src/ignores.rs').split('type R<T>')[0])));
  assert.deepEqual(sorted(cap.cli.hooks.actions), sorted(constant(read('crates/hook/src/admin.rs'), 'ACTIONS')));
  assert.deepEqual(sorted(cap.cli.surfaceBrief.actions), sorted([...read('crates/context/src/surface_brief_cli.rs').matchAll(/Some\("([^"\n]+)"\)\s*=>/g)].map((m) => m[1])));
  assert.deepEqual(sorted(cap.cli.componentReview.actions), sorted([...production(read('crates/context/src/component_review/mod.rs')).matchAll(/Some\("([^"\n]+)"\)/g)].map((m) => m[1])));
  assert.ok(read('crates/cli/src/main.rs').includes('v.starts_with("live")'));
  assert.ok(read('crates/cli/src/main.rs').includes('looks_like_detect_target'));
  const rootNames = cap.cli.root.flatMap((entry) => entry.names);
  for (const entry of cap.cli.live) {
    for (const name of entry.names.filter((name) => !name.startsWith('live'))) {
      assert.equal(rootNames.includes(name), false, `${name} is an internal live alias, not a root command`);
    }
  }
  for (const entry of [...cap.cli.root, ...cap.cli.live]) {
    const [file, line] = entry.currentSource.split(':');
    const arm = read(file).split('\n')[Number(line) - 1];
    assert.ok(arm.includes('=>'), entry.currentSource);
    for (const name of entry.names) assert.ok(arm.includes(`"${name}"`), entry.currentSource);
  }
  for (const key of ['skills', 'ignores', 'hooks', 'surfaceBrief', 'componentReview', 'critiqueStorage', 'pin']) {
    const [file, line] = cap.cli[key].currentSource.split(':');
    assert.match(read(file).split('\n')[Number(line) - 1], /^pub fn run(?:_with_capturer)?\(/, cap.cli[key].currentSource);
  }
});

test('design command table, metadata, pin support and mode constants agree', () => {
  const cap = inventory();
  const skill = read('skill/SKILL.src.md');
  const table = [...skill.matchAll(/^\| `([a-z][a-z-]*)(?: [^`]*)?` \| ([^|]+) \| ([^|]+) \| (.+) \|$/gm)];
  assert.deepEqual(sorted(cap.designCommands.map((entry) => entry.name)), sorted(table.map((m) => m[1])));
  assert.deepEqual(sorted(table.map((m) => m[1])), sorted(Object.keys(JSON.parse(read('skill/scripts/command-metadata.json')))));
  assert.deepEqual(sorted(table.map((m) => m[1])), sorted(constant(read('crates/context/src/pin.rs'), 'VALID_COMMANDS')));
  assert.deepEqual(cap.designAliases, [{ name: 'teach', target: 'init' }]);
  assert.ok(skill.includes('`teach` aliases `init`'));
  assert.deepEqual(cap.modes.names, constant(read('crates/context/src/catalog.rs'), 'SEED_MODES'));
  assert.deepEqual(cap.modes.tiers, constant(read('crates/context/src/catalog.rs'), 'WELL_TIERS'));
  assert.deepEqual(cap.modes.systemPrefixes, constant(read('crates/context/src/catalog.rs'), 'SYSTEM_PREFIXES'));
  for (const command of cap.designCommands) {
    assert.ok(fs.existsSync(sourcePath(command.reference)), command.reference);
    assert.equal(command.importedPath, `tools/phoenix-ui/${command.reference}`);
    assert.equal(command.currentPath, path.relative(repository, sourcePath(command.reference)));
    assert.equal(command.verification.status, '未验证');
  }
});

test('build-phase actions and phase order match its production dispatcher', () => {
  const cap = inventory();
  const build = production(read('crates/comp-verbs/src/build_phase.rs'));
  const cli = build.split('pub fn run_with_renderer')[1];
  const early = [...cli.matchAll(/if cmd == "([^"]+)"/g)].map((m) => m[1]);
  assert.deepEqual(sorted(cap.cli.buildPhase.entries.map((entry) => entry.name)), sorted([...early, ...arms(cli)]));
  for (const entry of cap.cli.buildPhase.entries) {
    const [file, line] = entry.currentSource.split(':');
    const arm = read(file).split('\n')[Number(line) - 1];
    assert.ok(arm.includes(`"${entry.name}"`) && (arm.includes('=>') || arm.includes('if cmd ==')), entry.currentSource);
  }
  assert.deepEqual(cap.cli.buildPhase.phases, constant(build, 'PHASES'));
  const finish = cli.match(/if !matches!\(disposition, ([^\n]+)\) \{/);
  assert.ok(finish);
  assert.deepEqual(cap.cli.buildPhase.finishDispositions, strings(finish[1]));
});

test('fixed catalog fixtures keep their hashes and current explicit loader keeps all four inputs', () => {
  const cap = inventory();
  const upstream = JSON.parse(read('UPSTREAM.json'));
  const dataFiles = Object.keys(upstream.importedFiles).filter((file) => /(?:concept|composition)-(?:ingredients|reviews)\.json$/.test(file));
  assert.deepEqual(sorted(dataFiles), sorted(cap.directionLibrary.fixtures.flatMap((entry) => [entry.path, entry.reviews])));
  const loader = read('crates/context/src/concept_seed.rs').split('fn load_local(')[1].split('fn builtin_catalog(')[0];
  assert.deepEqual(cap.directionLibrary.loaderInputs, strings(loader).filter((name) => name.endsWith('.json')));
  for (const entry of cap.directionLibrary.fixtures) {
    const data = JSON.parse(read(entry.path));
    const reviews = JSON.parse(read(entry.reviews));
    const items = entry.kind === 'concept' ? data.families.flatMap((family) => family.concepts) : data.compositions;
    assert.equal(sha256(read(entry.path)), upstream.importedFiles[entry.path]);
    assert.equal(sha256(read(entry.reviews)), upstream.importedFiles[entry.reviews]);
    assert.equal(entry.schemaVersion, data.schemaVersion);
    assert.equal(entry.reviewSchemaVersion, reviews.schemaVersion);
    assert.equal(entry.catalogVersion, data.catalogVersion);
    assert.equal(entry.familyCount, data.families.length);
    assert.equal(entry.wellCount, data.wells?.length || 0);
    assert.equal(entry.entryCount, items.length);
    for (const status of ['approved', 'rejected']) {
      assert.equal(entry[`${status}Count`], items.filter((item) => reviews.reviews[item.id].status === status).length);
    }
  }
});

test('the built-in original catalog matches both loader sources, review hashes and strict mode/scope quotas', () => {
  const cap = inventory();
  const current = cap.current.directionLibrary;
  const seed = production(read('crates/context/src/concept_seed.rs'));
  const builtin = seed.split('fn builtin_catalog(')[1].split('struct RollData')[0];
  const included = [...builtin.matchAll(/include_str!\("([^"]+)"\)/g)]
    .map((match) => path.relative(repository, path.resolve(path.dirname(sourcePath('crates/context/src/concept_seed.rs')), match[1])));
  assert.deepEqual(included, current.loader.builtinInputs);
  assert.deepEqual(current.loader.explicitInputs, cap.directionLibrary.loaderInputs);
  assert.match(seed, /let local = match env\.get\("PHOENIX_UI_CATALOG_DIR"\)\.filter\(\|v\| !v\.is_empty\(\)\) \{\s*Some\(dir\) => load_local\(dir\),\s*None => builtin_catalog\(\),\s*\}\s*\.map_err\(\|message\| \(2, message\)\)\?;/);
  assert.match(seed, /select_approved_challengers\(scope, key, reroll, mode, &local\.concepts\)\s*\.map_err\(\|message\| \(1, message\)\)\?;/);
  assert.equal(current.loader.default, 'built-in-original');
  assert.ok(seed.includes(`env.get("${current.loader.explicitDirectoryEnvironment}").filter(|v| !v.is_empty())`));
  const data = current.artifacts.map((entry) => {
    const bytes = fs.readFileSync(path.join(repository, entry.path));
    assert.equal(bytes.length, entry.bytes);
    assert.equal(sha256(bytes), entry.sha256, entry.path);
    return JSON.parse(bytes);
  });
  const [concepts, conceptReviews, compositions, compositionReviews] = data;
  assert.deepEqual(data.map((entry) => entry.schemaVersion), [8, 2, 2, 2]);
  assert.ok(data.every((entry) => entry.catalogVersion === current.catalogVersion && entry.authoringStatus === 'draft'));
  const wells = new Map(concepts.wells.map((well) => [well.id, well.tier]));
  const entries = concepts.families.flatMap((family) => family.concepts.map((entry) => ({ ...entry, tier: wells.get(family.well) })));
  assert.equal(entries.length, current.counts.concepts);
  assert.equal(concepts.wells.length, current.counts.wells);
  assert.equal(concepts.families.length, current.counts.conceptFamilies);
  assert.equal(compositions.compositions.length, current.counts.compositions);
  assert.deepEqual(sorted(Object.keys(conceptReviews.reviews)), sorted(entries.map((entry) => entry.id)));
  assert.deepEqual(sorted(Object.keys(compositionReviews.reviews)), sorted(compositions.compositions.map((entry) => entry.id)));
  const contentHash = (entry, field) => sha256([
    entry.form, entry.lineage, JSON.stringify(entry.tags), JSON.stringify(entry[field]), entry.spark, entry.webLeverage,
  ].join('\n')).slice(0, 12);
  for (const [items, reviews, field] of [[entries, conceptReviews, 'system'], [compositions.compositions, compositionReviews, 'grammar']]) {
    for (const entry of items) {
      const review = reviews.reviews[entry.id];
      assert.equal(contentHash(entry, field), review.formHash, entry.id);
      assert.equal(review.status, 'approved');
      assert.equal(review.reviewedBy, current.review.reviewedBy);
      assert.ok(Number.isFinite(Date.parse(review.reviewedAt)));
      assert.equal(review.rating, 2);
      assert.equal(review.breadth, 'general');
      assert.equal(review.allowedModes.length, 1);
      assert.ok(cap.modes.names.includes(review.allowedModes[0]));
      assert.ok(review.note.includes('未获用户接受且运行时验收尚未完成'));
      assert.equal(entry.source.kind, 'original');
      assert.equal(entry.source.license, 'Apache-2.0');
      assert.ok(entry.applicability && entry.thesis && entry.source.title);
      assert.equal(/https?:\/\/|"cardBoard"|"cardHero"/.test(JSON.stringify(entry)), false);
      if (field === 'system') {
        assert.equal(entry.strength, 'dual');
        assert.deepEqual(entry.system.map((rule, i) => rule.slice(0, cap.modes.systemPrefixes[i].length)), cap.modes.systemPrefixes);
      } else {
        assert.deepEqual(review.allowedModes, [entry.surface]);
        assert.ok(entry.provenance);
      }
    }
  }
  const selection = production(read('crates/context/src/roll_selection.rs'));
  assert.ok(selection.includes('[("operate", [("graphic", 5), ("interaction", 1), ("atmosphere", 0)])]'));
  assert.ok(selection.includes('const DEFAULT_TIER_QUOTA: usize = 2;'));
  assert.ok(selection.includes('if scope == "direction" { ["world", "dual"] } else { ["composition", "dual"] }'));
  const actual = [];
  for (const mode of cap.modes.names) {
    for (const scope of ['direction', 'surface']) {
      const wanted = scope === 'direction' ? ['world', 'dual'] : ['composition', 'dual'];
      const eligible = entries.filter((entry) => wanted.includes(entry.strength) && conceptReviews.reviews[entry.id].allowedModes.includes(mode));
      const byTier = Object.fromEntries(cap.modes.tiers.map((tier) => [tier, eligible.filter((entry) => entry.tier === tier).length]));
      const required = mode === 'operate' ? { graphic: 5, interaction: 1, atmosphere: 0 } : { graphic: 2, interaction: 2, atmosphere: 2 };
      assert.deepEqual(byTier, required, `${mode}/${scope}`);
      actual.push({ mode, scope, eligible: eligible.length, byTier, required, pass: true });
    }
    assert.equal(compositions.compositions.filter((entry) => entry.surface === mode).length, current.selection.compositions[mode]);
  }
  assert.deepEqual(actual, current.selection.eligible);
  assert.equal(current.validation.rust, '未验证');
  assert.equal(current.validation.projectAcceptance, '未验证');
});

test('all 19 provider outputs are distinguished from native install and pin scans', async () => {
  const cap = inventory();
  const { PROVIDERS } = await import(pathToFileURL(path.join(workspace, 'scripts/lib/transformers/providers.js')));
  const { hooksJsonFor } = await import(pathToFileURL(path.join(workspace, 'scripts/lib/transformers/hooks.js')));
  const bundle = JSON.parse(fs.readFileSync(path.resolve(workspace, '../../vendor/impeccable/bundle/manifest.json'), 'utf8'));
  const installDirs = constant(read('crates/skills/src/providers.rs'), 'PROVIDER_DIRS');
  const pinDirs = constant(read('crates/context/src/pin.rs'), 'HARNESS_DIRS');
  const contextDirs = [...read('crates/context/src/provider.rs').split('fn provider_from_skill_dir(')[1]
    .split('pub fn detect(')[0].matchAll(/^\s*"(\.[^"\n]+)"\s*=>/gm)].map((m) => m[1]);
  assert.equal(Object.keys(PROVIDERS).length, 19);
  assert.deepEqual(sorted(cap.providers.map((entry) => entry.key)), sorted(Object.keys(PROVIDERS)));
  assert.deepEqual(sorted(cap.providers.map((entry) => entry.configDir)), sorted(Object.keys(bundle.providers)));
  assert.ok(cap.providers.every((entry) => entry.sourceScope === 'fixed-upstream'));
  assert.deepEqual(sorted(cap.current.providers.map((entry) => entry.key)), sorted(Object.keys(PROVIDERS)));
  assert.deepEqual(cap.current.providerCounts, {
    generated: Object.keys(PROVIDERS).length,
    nativeInstallScan: installDirs.length,
    nativeContextRecognition: contextDirs.length,
    nativePinScan: pinDirs.length,
  });
  for (const provider of cap.current.providers) {
    const config = PROVIDERS[provider.key];
    assert.equal(provider.configDir, config.configDir);
    assert.equal(provider.nativeInstallScan, installDirs.includes(provider.configDir));
    assert.equal(provider.nativeContextRecognition, contextDirs.includes(provider.configDir));
    assert.equal(provider.nativePinScan, pinDirs.includes(provider.configDir));
    assert.equal(provider.agentFormat, config.agentFormat || (['codex', 'agents'].includes(provider.key) ? 'codex-skill-toml' : null));
    const hooks = config.emitHooks ? hooksJsonFor(config.emitHooks, { configDir: config.configDir }) : null;
    assert.deepEqual(provider.hookEvents, hooks ? Object.keys(hooks.hooks) : []);
    assert.equal(provider.hooksManifest, config.hooksManifestRel || null);
    assert.equal(provider.runtimeStatus, '未验证');
  }
});

test('current generated provider paths and bundle bytes agree with source metadata and original catalog payloads', async () => {
  const cap = inventory();
  const current = cap.current.generatedBundle;
  const { PROVIDERS } = await import(pathToFileURL(path.join(workspace, 'scripts/lib/transformers/providers.js')));
  const { hooksJsonFor } = await import(pathToFileURL(path.join(workspace, 'scripts/lib/transformers/hooks.js')));
  const manifestBytes = fs.readFileSync(path.join(repository, current.manifest));
  const bundle = JSON.parse(manifestBytes);
  assert.equal(sha256(manifestBytes), current.manifestSha256);
  const generationBytes = fs.readFileSync(path.join(repository, current.generationManifest));
  const generated = JSON.parse(generationBytes);
  assert.equal(sha256(generationBytes), current.generationManifestSha256);
  assert.equal(generated.inputSha256, current.inputSha256);
  assert.equal(generated.bundleSha256, current.manifestSha256);
  assert.equal(generated.engineManifest, current.engineManifest);
  assert.equal(current.engineManifest, 'verified');
  const { manifestAt } = require(path.join(repository, 'skills/phoenix-ui/src/scripts/phoenix-bootstrap.cjs'));
  const engine = manifestAt(path.join(repository, current.engineManifestEvidence.path));
  assert.equal(engine.hash, current.engineManifestEvidence.sha256);
  assert.equal(engine.data.distribution, current.engineManifestEvidence.distribution);
  assert.equal(engine.data.sourceInputSha256, current.engineManifestEvidence.sourceInputSha256);
  assert.equal(engine.data.toolVersion, cap.current.engineVersion);
  assert.equal(engine.data.bundleSha256, current.manifestSha256);
  assert.deepEqual(engine.data.engines, current.engineManifestEvidence.engines);
  assert.equal(sha256(fs.readFileSync(path.join(repository, bundle.derivedRuntimeManifest))), engine.hash);
  assert.equal(generated.outputs.skill['scripts/ENGINE.json'], engine.hash);
  assert.equal(Object.values(generated.outputs).reduce((n, files) => n + Object.keys(files).length, 0), current.outputsCount);
  for (const [file, hash] of Object.entries(generated.inputs)) {
    assert.equal(sha256(fs.readFileSync(path.join(repository, file))), hash, file);
  }
  assert.deepEqual(sorted(Object.keys(bundle.providers)), sorted(cap.current.providers.map((provider) => provider.configDir)));
  const blob = (hash) => {
    const bytes = fs.readFileSync(path.join(repository, path.dirname(current.manifest), 'blobs', hash));
    assert.equal(sha256(bytes), hash);
    return bytes;
  };
  let copies = 0;
  for (const provider of cap.current.providers) {
    const config = PROVIDERS[provider.key];
    const payload = bundle.providers[provider.bundle.configDir];
    assert.equal(provider.generated.root, `tools/phoenix-ui/dist/providers/${provider.key}/${config.configDir}`);
    const generatedPath = (file) => file.slice('tools/phoenix-ui/dist/providers/'.length);
    assert.ok(generated.outputs.providers[generatedPath(provider.generated.skill)]);
    for (const launcher of provider.generated.launchers) assert.ok(generated.outputs.providers[generatedPath(launcher)]);
    assert.equal(generated.outputs.providers[generatedPath(`${provider.generated.root}/skills/phoenix-ui/scripts/ENGINE.json`)], engine.hash);
    if (provider.generated.hooks) assert.ok(generated.outputs.providers[generatedPath(provider.generated.hooks)]);
    assert.equal(provider.bundle.files, Object.keys(payload.files).length);
    for (const hash of Object.values(payload.files)) blob(hash);
    const skillPayload = bundle.providers[provider.bundle.skillConfigDir];
    assert.ok(skillPayload.files[provider.bundle.skill]);
    for (const launcher of provider.bundle.launchers) assert.ok(skillPayload.files[launcher]);
    assert.equal(provider.bundle.skillConfigDir, provider.key === 'codex' ? '.agents' : config.configDir);
    assert.equal(provider.bundle.hookOnly, provider.key === 'codex');
    if (provider.bundle.hookOnly) assert.deepEqual(Object.keys(payload.files), ['hooks.json']);
    assert.deepEqual(provider.bundle.subagentFiles, Object.keys(skillPayload.files)
      .filter((name) => name.startsWith('agents/') || name.startsWith('skills/phoenix-ui/subagents/')).sort());
    for (const entry of cap.current.directionLibrary.artifacts) {
      const name = path.basename(entry.path);
      assert.equal(skillPayload.files[`${provider.bundle.catalogDir}/${name}`], entry.sha256);
      const generatedName = `${provider.key}/${config.configDir}/skills/phoenix-ui/scripts/data/catalog/${name}`;
      assert.equal(generated.outputs.providers[generatedName], entry.sha256);
      if (!provider.bundle.hookOnly) copies++;
    }
    if (config.emitHooks) {
      const emitted = hooksJsonFor(config.emitHooks, { configDir: config.hookSkillsDir || config.configDir });
      assert.deepEqual(JSON.parse(blob(payload.files[provider.hooksManifest])), emitted, provider.key);
    }
  }
  assert.equal(copies, current.bundledCatalogCopies);
  assert.equal(current.bundledSkillPayloads, Object.values(bundle.providers).filter((entry) => entry.files['skills/phoenix-ui/SKILL.md']).length);
  assert.equal(current.runtimeStatus, '未验证');
});

test('current environment lookups are enumerated separately from the fixed source ledger', () => {
  const cap = inventory();
  const found = new Set();
  const files = fs.readdirSync(path.join(workspace, 'crates')).flatMap((crate) => {
    const src = path.join(workspace, 'crates', crate, 'src');
    return fs.existsSync(src) ? rustFiles(src) : [];
  });
  for (const file of files) {
    const text = production(fs.readFileSync(file, 'utf8'));
    for (const match of text.matchAll(/(?:\.get\(\s*"|(?:std::)?env::var(?:_os)?\(\s*"|\bio\.env\(\s*")([A-Z][A-Z0-9_]+)"/g)) found.add(match[1]);
    for (const match of text.matchAll(/"((?:IMPECCABLE|PHOENIX_UI)_[A-Z0-9_]+)"/g)) found.add(match[1]);
    if (file.endsWith('component_review/mod.rs')) {
      for (const match of text.matchAll(/\bset\("([A-Z][A-Z0-9_]+)"/g)) found.add(match[1]);
    }
  }
  assert.deepEqual(sorted(cap.current.environment.map((entry) => entry.name)), sorted(found));
  for (const entry of cap.environment) {
    assert.ok(entry.sources.length > 0, entry.name);
    assert.ok(['retain', 'replace', 'remove'].includes(entry.disposition));
    assert.equal(entry.sourceScope, 'fixed-upstream');
    const upstream = JSON.parse(read('UPSTREAM.json'));
    for (const source of entry.sources) {
      const file = source.split(':')[0];
      assert.equal(cap.sourceFiles[file], upstream.importedFiles[file], source);
    }
  }
  for (const entry of cap.current.environment) {
    assert.ok(['phoenix-runtime', 'prepared-build-tool', 'host-platform', 'explicit-image-generation-api'].includes(entry.classification));
    assert.equal(entry.runtimeStatus, '未验证');
    for (const source of entry.sources) {
      const [file, line] = source.split(':');
      assert.ok(read(file).split('\n')[Number(line) - 1].includes(entry.name), source);
    }
  }
  const cargo = cap.environment.find((entry) => entry.name === 'CARGO_TARGET_DIR');
  assert.equal(cargo.classification, 'cargo-standard-build-option');
  for (const file of ['crates/bundle/src/lib.rs', 'crates/xtask/src/main.rs']) {
    assert.equal(production(read(file)).includes('"CARGO_TARGET_DIR"'), false, file);
  }
});

test('the current public identity and host copy selectors match their actual interfaces', () => {
  const current = inventory().current;
  assert.equal(current.engineVersion, read('ENGINE_VERSION').trim());
  assert.equal(current.cliVersion, current.engineVersion);
  const packageVersion = read('Cargo.toml').split('[workspace.package]')[1].match(/^version = "([^"]+)"/m)[1];
  assert.equal(current.cliVersion, packageVersion);
  const cli = read('crates/cli/Cargo.toml');
  assert.equal(cli.match(/^name = "([^"]+)"/m)[1], current.cliPackage);
  assert.equal(cli.split('[[bin]]')[1].match(/^name = "([^"]+)"/m)[1], current.cliBinary);
  assert.ok(read('crates/cli/src/main.rs').includes('pub const CLI_VERSION: &str = VERSION;'));
  assert.equal(current.engineProbe, `phoenix-ui-engine ${current.engineVersion}`);
  assert.match(production(read('crates/context/src/provider.rs')), /let command_prefix = if matches!\(id\.as_str\(\), "codex" \| "agents"\) \{\s*"\$"\s*\} else \{\s*"\/"\s*\}/);
  assert.equal(current.providerCommandPrefix.codex, '$');
  assert.equal(current.providerCommandPrefix.agents, '$');
  assert.equal(current.providerCommandPrefix.other, '/');
  const copy = production(read('crates/live/src/copy_edit_agent.rs'));
  const selector = copy.split('pub fn choose_copy_edit_agent(')[1].split('pub fn host_copy_edit_requested')[0];
  const modes = [...selector.matchAll(/^\s*("[^"\n]*"(?:\s*\|\s*"[^"\n]*")*)(?:\s+if[^=\n]*)?\s*=>/gm)].flatMap((m) => strings(m[1]));
  assert.deepEqual(modes, current.copyEdit.selectorModes);
  for (const external of current.copyEdit.externalModes) assert.equal(modes.includes(external), false);
  const commands = [...copy.matchAll(/Command::new\(([^\n]+)\)/g)].map((m) => m[1]);
  assert.ok(commands.length > 0);
  assert.ok(commands.every((command) => command === 'proc::node_exe()'));
});

test('the two E12 patch candidates have pinned input and patch hashes', () => {
  const cap = inventory();
  const upstream = JSON.parse(read('UPSTREAM.json'));
  const sources = JSON.parse(read('patches/sources.json'));
  const bundle = JSON.parse(fs.readFileSync(path.resolve(workspace, '../../vendor/impeccable/bundle/manifest.json'), 'utf8'));
  assert.equal(sources.candidateCommit, '83dc4b60ca4c3a13ea3cde748804858d37ef47c9');
  assert.deepEqual(cap.backports.map((entry) => entry.id), ['E12-IME', 'E12-CODEX-WINDOWS']);
  for (const patch of sources.patches) {
    assert.equal(sha256(read(patch.file)), patch.sha256, patch.file);
    assert.equal(cap.backports.find((entry) => entry.id === patch.id).patchSha256, patch.sha256);
    for (const file of patch.files) {
      if (file.path === '.codex/hooks.json') {
        assert.equal(bundle.providers['.codex'].files['hooks.json'], file.beforeSha256);
      } else {
        assert.equal(upstream.importedFiles[file.path], file.beforeSha256);
      }
      for (const edit of file.edits) {
        assert.equal(sha256(edit.before), edit.beforeSha256);
        assert.equal(sha256(edit.after), edit.afterSha256);
      }
    }
    assert.equal(patch.status, 'candidate-not-applied');
  }
});

test('review patches reconstruct exactly the recorded candidate files in memory', () => {
  const sources = JSON.parse(read('patches/sources.json'));
  for (const patch of sources.patches) {
    const chunks = read(patch.file).split(/(?=^--- a\/)/m).filter((chunk) => chunk.trim());
    assert.equal(chunks.length, patch.files.length);
    for (const chunk of chunks) {
      const lines = chunk.trimEnd().split('\n');
      const file = patch.files.find((entry) => lines[0] === `--- a/${entry.path}`);
      assert.ok(file);
      assert.equal(lines[1], `+++ b/${file.path}`);
      // Immutable published bytes and the pinned emitter snapshot survive
      // subsequent source moves, branding and applied fixes.
      const blob = path.join(repository, 'vendor/impeccable/bundle/blobs', file.beforeSha256);
      const snapshot = path.join(workspace, 'patches/source-snapshots', file.beforeSha256);
      const before = file.path === '.codex/hooks.json' ? file.edits[0].before
        : fs.existsSync(blob) ? fs.readFileSync(blob, 'utf8')
          : fs.existsSync(snapshot) ? fs.readFileSync(snapshot, 'utf8') : read(file.path);
      assert.equal(sha256(before), file.beforeSha256);
      const input = before.trimEnd().split('\n');
      const output = [];
      let cursor = 0;
      for (let i = 2; i < lines.length;) {
        const hunk = lines[i++].match(/^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@/);
        assert.ok(hunk);
        const target = Number(hunk[1]) - 1;
        assert.ok(target >= cursor);
        output.push(...input.slice(cursor, target));
        cursor = target;
        assert.equal(output.length, Number(hunk[3]) - 1);
        let removed = 0;
        let added = 0;
        while (i < lines.length && !lines[i].startsWith('@@')) {
          const line = lines[i++];
          assert.ok([' ', '+', '-'].includes(line[0]));
          if (line[0] !== '+') {
            assert.equal(input[cursor++], line.slice(1));
            removed++;
          }
          if (line[0] !== '-') {
            output.push(line.slice(1));
            added++;
          }
        }
        assert.equal(removed, Number(hunk[2] ?? 1));
        assert.equal(added, Number(hunk[4] ?? 1));
      }
      output.push(...input.slice(cursor));
      assert.equal(sha256(`${output.join('\n')}\n`), file.candidateSha256);
    }
  }
});

test('the isolated IME helper handles composition and Safari 229 without submitting', () => {
  const sources = JSON.parse(read('patches/sources.json'));
  const edits = sources.patches.find((entry) => entry.id === 'E12-IME').files[0].edits;
  const helper = edits.find((edit) => edit.after.includes('function isImeKeydown'));
  const isImeKeydown = Function(`${helper.after}\nreturn isImeKeydown;`)();
  assert.equal(isImeKeydown({ isComposing: true, keyCode: 13 }), true);
  assert.equal(isImeKeydown({ isComposing: false, keyCode: 229 }), true);
  assert.equal(isImeKeydown({ isComposing: false, keyCode: 13 }), false);
  assert.equal(edits.filter((edit) => edit.after.includes("e.key === 'Enter' && !isImeKeydown(e)")).length, 3);
  const browser = read('skill/scripts/live-browser.js');
  const current = inventory().current.backports.find((entry) => entry.id === 'E12-IME');
  const appliedHelper = browser.match(/function isImeKeydown\(e\) \{[\s\S]*?\n\s*\}/);
  assert.ok(appliedHelper);
  const applied = Function(`${appliedHelper[0]}\nreturn isImeKeydown;`)();
  assert.deepEqual([
    applied({ isComposing: true, keyCode: 13 }),
    applied({ isComposing: false, keyCode: 229 }),
    applied({ isComposing: false, keyCode: 13 }),
  ], [true, true, false]);
  const lines = [...browser.matchAll(/e\.key === 'Enter' && !isImeKeydown\(e\)/g)]
    .map((match) => browser.slice(0, match.index).split('\n').length);
  assert.deepEqual(current.enterGuardLines, lines);
  assert.equal(current.verification.browser, '未验证');
});

test('the Windows emitter candidate preserves both hook events and shell quoting', () => {
  const sources = JSON.parse(read('patches/sources.json'));
  const patch = sources.patches.find((entry) => entry.id === 'E12-CODEX-WINDOWS');
  const emitter = patch.files.find((entry) => entry.path.endsWith('transformers/hooks.js'));
  const code = emitter.edits[0].after.replace('export const', 'const');
  const emit = Function(`${code}\nreturn windowsLauncherCommand;`)();
  assert.equal(emit('.agents/skills/impeccable/scripts/impeccable.cmd'), 'cmd /c if exist ".agents\\skills\\impeccable\\scripts\\impeccable.cmd" ".agents\\skills\\impeccable\\scripts\\impeccable.cmd" hook');
  assert.equal(emit('C:/a b/impeccable.cmd', 'hook'), 'cmd /c if exist "C:\\a b\\impeccable.cmd" "C:\\a b\\impeccable.cmd" hook');
  const generated = patch.files.find((entry) => entry.path === '.codex/hooks.json');
  const before = JSON.parse(generated.edits[0].before);
  const after = JSON.parse(generated.edits[0].after);
  for (const event of ['PostToolUse', 'Stop']) {
    const oldHandler = before.hooks[event][0].hooks[0];
    const newHandler = after.hooks[event][0].hooks[0];
    assert.deepEqual({ ...newHandler, commandWindows: oldHandler.commandWindows }, oldHandler);
    assert.equal(newHandler.commandWindows, emit('.codex/skills/impeccable/scripts/impeccable.cmd'));
  }
});
