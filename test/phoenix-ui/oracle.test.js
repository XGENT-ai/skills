const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const test = require('node:test');

const root = path.resolve(__dirname, '../..');
const adapter = import('../../tools/phoenix-ui/tests/oracle/adapters/phoenix.mjs');
const harness = import('../../tools/phoenix-ui/tests/oracle/lib.mjs');

test('strict oracle verifies every pinned source, golden and fixture without an accepted-case parser', async () => {
  const { verifyCorpus, metadata } = await adapter;
  const { allCases } = await harness;
  const corpus = verifyCorpus();
  const upstream = JSON.parse(fs.readFileSync(path.join(root, 'tools/phoenix-ui/UPSTREAM.json')));
  assert.equal(corpus.sourceCommit, upstream.sourceCommit);
  assert.equal(corpus.files, Object.keys(upstream.importedFiles).filter((f) =>
    f.startsWith('tests/oracle/') || f.startsWith('tests/fixtures/')).length);
  assert.equal((await allCases()).length, metadata.caseCount);
  const checker = fs.readFileSync(path.join(root, 'scripts/check-phoenix-oracle.mjs'), 'utf8');
  assert.doesNotMatch(checker, /loadAcceptedDeltas|accepted\.has|writeGolden|record\.mjs/);
});

test('input adaptations preserve the complete fixed case inventory and all step verbs', async () => {
  const { loadPhoenixCases, metadata } = await adapter;
  const { allCases } = await harness;
  const original = await allCases();
  const { cases } = await loadPhoenixCases('/tmp/oracle-test-not-an-executable');
  const inventory = (items) => items.map((c) => [c.id, c.sourceFile, c.verb,
    c.steps?.map((s) => [s.verb, s.daemon])]);
  assert.deepEqual(inventory(cases), inventory(original));
  assert.equal(new Set(cases.map((c) => c.id)).size, original.length);
  const fake = cases.find((c) => c.id === 'genimg-fake-svg');
  assert.equal(fake.env.PHOENIX_UI_IMAGE_GEN_FAKE, '1');
  assert.equal(fake.env.OPENAI_API_KEY, null);
  assert.equal(cases.find((c) => c.id === 'hook-env-disabled').env.PHOENIX_UI_HOOK_DISABLED, '1');
  assert.ok(metadata.preservedIdentifiers.includes('retired hook.mjs migration inputs'));
  const stale = cases.find((c) => c.id === 'context-stale-hook-manifest');
  assert.match(stale.setup.toString(), /adaptWorkspace/);
  const sourcePatch = metadata.inputs.find((p) => p.path.endsWith('/context.mjs'));
  assert.ok(sourcePatch.edits.some((e) => e.before === 'skills/impeccable/scripts/impeccable'));
  assert.ok(!sourcePatch.edits.some((e) => e.before.includes('hook.mjs')));
});

test('exact input patches reject changed anchors, overlaps and changed patch hashes', async () => {
  const { applyEdits, metadata, sha256 } = await adapter;
  assert.throws(() => applyEdits('abc', [{ offset: 1, before: 'x', after: 'y' }], 'test'), /invalid exact edit/);
  assert.throws(() => applyEdits('abc', [{ offset: 0, before: 'ab', after: 'x' },
    { offset: 1, before: 'b', after: 'y' }], 'test'), /invalid exact edit/);
  for (const entry of [...metadata.inputs, ...metadata.workspaces]) {
    const before = fs.readFileSync(path.join(root, 'tools/phoenix-ui', entry.path), 'utf8');
    assert.equal(sha256(before), entry.beforeSha256);
    const after = applyEdits(before, entry.edits, entry.path);
    assert.equal(sha256(after), entry.afterSha256);
    const mutated = [...entry.edits.map((e) => ({ ...e }))];
    mutated[0].after += 'mutation';
    assert.notEqual(sha256(applyEdits(before, mutated, entry.path)), entry.afterSha256);
  }
});

test('case-specific state identity patches keep all unrelated golden fields and fail new differences', async () => {
  const { metadata, readGolden, phoenixExpected, diffResults } = await adapter;
  for (const entry of metadata.expectations) {
    const golden = readGolden(entry.id), expected = phoenixExpected(entry.id, golden);
    if (metadata.invalidCatalogDirectoryCases.includes(entry.id)) {
      assert.equal(golden.exit, 0, entry.id);
      assert.equal(expected.exit, 2, entry.id);
      assert.equal(expected.stdout, '', entry.id);
      assert.equal(expected.stderr, 'concept-seed: missing or unreadable catalog file <WS>/no-such-catalog/concept-ingredients.json\n', entry.id);
    } else assert.equal(expected.exit, golden.exit, entry.id);
    assert.equal(expected.signal, golden.signal, entry.id);
    assert.equal(expected.steps?.length, golden.steps?.length, entry.id);
    assert.deepEqual(readGolden(entry.id), golden, `${entry.id}: immutable golden`);
    const actual = structuredClone(expected);
    if (actual.steps) actual.steps[0].stderr += 'new unexplained difference';
    else actual.stderr += 'new unexplained difference';
    assert.ok(diffResults(expected, actual).length, entry.id);
  }
  // DELTAS.md explicitly says this detector section is no longer accepted.
  const id = 'detect-fixture-json-should-pass-html';
  const golden = readGolden(id), bad = structuredClone(golden);
  bad.stdout += 'unexplained';
  assert.ok(diffResults(phoenixExpected(id, golden), bad).length);
});

test('missing live target values only adapt the public prefix and still reject changed errors', async () => {
  const { metadata, readGolden, phoenixExpected, diffResults, sha256 } = await adapter;
  const ids = ['live-accept-target-missing-value', 'live-inject-target-missing-value',
    'live-status-target-missing-value', 'live-server-target-missing-value', 'live-wrap-target-missing-value'];
  const before = '[impeccable live]', after = '[phoenix-ui live]';
  for (const id of ids) {
    const golden = readGolden(id), expected = phoenixExpected(id, golden);
    const entry = metadata.expectations.find((item) => item.id === id);
    assert.equal(entry.fields.length, 1, id);
    const field = entry.fields[0];
    assert.deepEqual(field.path, ['stderr'], id);
    assert.equal(field.beforeSha256, sha256(golden.stderr), id);
    assert.equal(field.afterSha256, sha256(expected.stderr), id);
    assert.deepEqual(field.edits.map(({ offset, before, after }) => ({ offset, before, after })),
      [{ offset: 0, before, after }], id);
    assert.deepEqual(field.edits[0].source, [{ file: 'crates/live/src/roots.rs', line: 491 }], id);
    for (const source of field.edits[0].source) {
      const line = fs.readFileSync(path.join(root, 'tools/phoenix-ui', source.file), 'utf8').split('\n')[source.line - 1];
      assert.ok(line.includes('"[phoenix-ui live] {}\\n"'), id);
    }
    assert.equal(expected.stderr, after + golden.stderr.slice(before.length), id);
    assert.equal(expected.stdout, golden.stdout, id);
    assert.equal(expected.exit, 1, id);
    assert.equal(expected.exit, golden.exit, id);
    assert.equal(expected.signal, golden.signal, id);
    assert.deepEqual(expected.files, Object.fromEntries(Object.entries(golden.files)
      .map(([file, bytes]) => [entry.fileKeys?.[file] || file, bytes])), id);
    for (const change of [
      { stderr: golden.stderr },
      { stderr: expected.stderr.replace('path value', 'unexplained value') },
      { stdout: 'unexplained output\n' },
      { exit: 2 },
      { files: { ...expected.files, 'unexplained.txt': 'new file' } },
    ]) assert.ok(diffResults(expected, { ...expected, ...change }).length, id);
    assert.deepEqual(readGolden(id), golden, `${id}: immutable golden`);
  }
});

test('live generate help adapts both browser keys without changing its other verdicts or action identifiers', async () => {
  const { metadata, readGolden, phoenixExpected, diffResults, sha256, applyEdits } = await adapter;
  const id = 'live-generate-local-verdicts', golden = readGolden(id), expected = phoenixExpected(id, golden);
  const field = metadata.expectations.find((item) => item.id === id).fields
    .find((item) => item.path.join('/') === 'steps/0/stdout');
  const edits = field.edits.filter((edit) => edit.before === 'IMPECCABLE_BROWSER');
  assert.equal(edits.length, 2);
  assert.deepEqual(edits.map((edit) => edit.offset).sort((a, b) => a - b),
    [...golden.steps[0].stdout.matchAll(/IMPECCABLE_BROWSER/g)].map((match) => match.index));
  assert.ok(edits.every((edit) => edit.after === 'PHOENIX_UI_BROWSER'));
  assert.deepEqual(edits.map((edit) => edit.source), [
    [{ file: 'crates/live/src/live_generate.rs', line: 30 }],
    [{ file: 'crates/live/src/live_generate.rs', line: 35 }],
  ]);
  for (const edit of edits) for (const source of edit.source) {
    const line = fs.readFileSync(path.join(root, 'tools/phoenix-ui', source.file), 'utf8').split('\n')[source.line - 1];
    assert.ok(line.includes('PHOENIX_UI_BROWSER'));
  }
  assert.equal(field.beforeSha256, sha256(golden.steps[0].stdout));
  assert.equal(field.afterSha256, sha256(expected.steps[0].stdout));
  assert.deepEqual(expected.steps.slice(1), golden.steps.slice(1));
  assert.equal(expected.steps[0].stderr, golden.steps[0].stderr);
  assert.equal(expected.steps[0].exit, golden.steps[0].exit);
  assert.equal(expected.steps[0].signal, golden.steps[0].signal);
  assert.match(expected.steps[0].stdout, /default: impeccable\)/);
  assert.ok(JSON.parse(expected.steps[3].stdout).validActions.includes('impeccable'));
  const beforeKeyAdaptation = applyEdits(golden.steps[0].stdout, field.edits.filter((edit) => !edits.includes(edit)), id);
  assert.equal(expected.steps[0].stdout.replaceAll('PHOENIX_UI_BROWSER', 'IMPECCABLE_BROWSER'), beforeKeyAdaptation);
  for (const mutate of [
    (result) => { result.steps[0].stdout = result.steps[0].stdout.replace('PHOENIX_UI_BROWSER', 'IMPECCABLE_BROWSER'); },
    (result) => { result.steps[0].stdout = result.steps[0].stdout.replace('then BROWSER', 'then UNEXPLAINED_BROWSER'); },
    (result) => { result.steps[0].stdout = result.steps[0].stdout.replace('default: impeccable', 'default: phoenix-ui'); },
    (result) => { result.steps[1].stdout += 'unexplained verdict'; },
    (result) => { result.steps[0].exit = 1; },
  ]) {
    const actual = structuredClone(expected);
    mutate(actual);
    assert.ok(diffResults(expected, actual).length);
  }
});

test('the fixed oracle has no invented coverage for live stream or startup diagnostic prefixes', async () => {
  const { allCases } = await harness;
  const { readGolden } = await adapter;
  const cases = await allCases();
  const liveVerbs = new Set(['live', 'live-target', 'live-status', 'live-resume', 'live-complete',
    'live-inject', 'live-wrap', 'live-insert', 'live-accept', 'live-server', 'live-generate',
    'live-poll', 'live-commit-manual-edits', 'live-discard-manual-edits']);
  for (const item of cases) {
    for (const invocation of item.steps || [item]) {
      assert.equal(invocation.args?.includes('--stream') || false, false, item.id);
      if (invocation.verb?.startsWith('live')) assert.ok(liveVerbs.has(invocation.verb), item.id);
    }
    const golden = readGolden(item.id);
    for (const output of golden.steps || [golden]) {
      assert.doesNotMatch(output.stdout + output.stderr, /\[impeccable-poll\]|\[impeccable\]|not implemented yet/, item.id);
    }
  }
});

test('invalid flag exits remain historical and only four explicit missing catalogs change exit semantics', async () => {
  const { metadata,readGolden,phoenixExpected,diffResults } = await adapter;
  const directoryIds = ['seed-degraded-direction','seed-degraded-surface','seed-degraded-safer','seed-degraded-bolder'];
  assert.deepEqual(metadata.invalidCatalogDirectoryCases,directoryIds);
  const scalarIds = metadata.expectations.filter((e) => e.values?.length).map((e) => e.id).sort();
  assert.deepEqual(scalarIds,[...directoryIds].sort());
  for (const id of ['seed-scope-invalid','seed-reroll-invalid','seed-register-invalid','seed-register-without-reroll','seed-register-surface','seed-mode-invalid','seed-grain-invalid','seed-platform-invalid','seed-candidate-count-invalid']) {
    const expected=phoenixExpected(id,readGolden(id));
    assert.equal(expected.exit,1,id);
    assert.ok(diffResults(expected,{...expected,exit:2}).length,id);
  }
});

test('Darwin IP isolation is restricted to four static native invocations and preserves the original argv', async () => {
  const { isolatedCases,nativeInvocation,offlineNetworkProfile,metadata,loadPhoenixCases }=await adapter;
  assert.deepEqual(isolatedCases,metadata.nativeNetworkPreconditions.ids);
  assert.equal(isolatedCases.length,4);
  const {cases}=await loadPhoenixCases('/tmp/precise-native-engine');
  const argv=['/tmp/precise-native-engine','detect','fixture','--json'];
  for (const item of cases) {
    const actual=nativeInvocation(item.id,argv,'darwin');
    if (isolatedCases.includes(item.id)) {
      assert.equal(item.verb,'detect');
      assert.equal(item.steps,undefined);
      assert.deepEqual(actual,['/usr/bin/sandbox-exec','-p',offlineNetworkProfile,...argv]);
    } else assert.strictEqual(actual,argv,item.id);
    assert.strictEqual(nativeInvocation(item.id,argv,'linux'),argv,item.id);
    assert.strictEqual(nativeInvocation(item.id,argv,'win32'),argv,item.id);
  }
  const fixed=fs.readFileSync(path.join(root,'tools/phoenix-ui/tests/oracle/lib.mjs'),'utf8');
  assert.match(fixed,/spawn\(argv\[0\], argv\.slice\(1\)/);
});

test('path-derived generated markers are exact hashes of the immutable original and adapted inputs', async () => {
  const {metadata,sha256,readGolden,phoenixExpected,diffResults}=await adapter;
  for (const proof of metadata.digestProofs) {
    assert.equal(sha256(JSON.stringify(proof.inputBefore)).slice(0,16),proof.before);
    assert.equal(sha256(JSON.stringify(proof.inputAfter)).slice(0,16),proof.after);
    const expected=phoenixExpected(proof.id,readGolden(proof.id));
    const bad=structuredClone(expected);
    const key=Object.keys(bad.files).find((p)=>p.includes(proof.after)) || '.phoenix-ui/questions/k1.hand.json';
    bad.files[key]+='changed unrelated bytes';
    assert.ok(diffResults(expected,bad).length,proof.id);
  }
});

test('the insufficient fixture exposes the same five eligible candidates without an invented full deal', async () => {
  const {metadata,readGolden,phoenixExpected,diffResults}=await adapter;
  const catalog=JSON.parse(fs.readFileSync(path.join(root,'tools/phoenix-ui/tests/fixtures/concept-catalog/concept-ingredients.json')));
  const reviews=JSON.parse(fs.readFileSync(path.join(root,'tools/phoenix-ui/tests/fixtures/concept-catalog/concept-reviews.json'))).reviews;
  const tiers=new Map(catalog.wells.map((w)=>[w.id,w.tier]));
  const eligible=catalog.families.flatMap((f)=>f.concepts.map((c)=>({...c,tier:tiers.get(f.well)})))
    .filter((c)=>reviews[c.id]?.status==='approved'&&['dual','surface'].includes(c.strength));
  assert.equal(eligible.filter((c)=>c.tier==='graphic').length,4);
  assert.equal(eligible.filter((c)=>c.tier==='interaction').length,2);
  const identities=(stdout)=>[...stdout.matchAll(/^     SOURCE ID: (.+)$/gm)].map((m)=>m[1]);
  for(const id of metadata.insufficientFixtureCases) {
    const old=readGolden(id),expected=phoenixExpected(id,old);
    assert.match(expected.stdout,/^INSUFFICIENT: status=insufficient; requested=6; available=5;/);
    assert.deepEqual(identities(expected.stdout),identities(old.stdout));
    assert.equal(identities(expected.stdout).length,5);
    assert.ok(identities(expected.stdout).every((id)=>eligible.some((c)=>c.id===id)));
    assert.doesNotMatch(expected.stdout,/DEALT INDICES|PHOENIX UI PICK|QUALITY BAR:|https:\/\//);
    assert.match(expected.stdout,/behavior\.\n\nMODE RULES \(operate,/);
    assert.ok(diffResults(expected,{...expected,stdout:expected.stdout.replace('available=5','available=6')}).length);
    assert.ok(diffResults(expected,{...expected,stdout:expected.stdout.replace(/SOURCE ID: .+/, 'SOURCE ID: invented-candidate')}).length);
  }
});

test('doctor preserves retired live state and presents it only as a user-owned migration decision', async () => {
  const {metadata,readGolden,phoenixExpected,diffResults}=await adapter;
  for(const id of metadata.legacyDoctorCases) {
    const old=readGolden(id),expected=phoenixExpected(id,old);
    for(const [file,bytes] of Object.entries(old.files||{}).filter(([file])=>file.startsWith('.impeccable-live'))) assert.equal(expected.files[file],bytes,id);
    for(const step of expected.steps||[expected]) {
      if(step.stdout.startsWith('{')) {
        const report=JSON.parse(step.stdout),legacy=report.findings.filter((f)=>f.id==='legacy-live-state');
        assert.equal(legacy.length,1,id);
        assert.equal(legacy[0].severity,'mention',id);
        assert.match(legacy[0].fix,/does not read or delete original live sessions\./);
        if(report.fixes) assert.equal(report.fixes.skipped.find((f)=>f.id==='legacy-live-state').reason,'needs a decision from the user');
      } else {
        assert.match(step.stdout,/worth saying \(\d+\):/);
        assert.match(step.stdout,/Original live-mode state is preserved at/);
        assert.doesNotMatch(step.stdout,/delete by hand once no live session is running/);
      }
    }
    const bad=structuredClone(expected);
    (bad.steps?bad.steps[0]:bad).stdout+='unrelated finding removed or changed';
    assert.ok(diffResults(expected,bad).length,id);
  }
});

test('current hook refresh and pins retain unclaimed legacy inputs and private source identities', async () => {
  const {metadata,readGolden,phoenixExpected,diffResults}=await adapter;
  const repair=phoenixExpected('hadmin-on-repairs-existing-manifest',readGolden('hadmin-on-repairs-existing-manifest'));
  const entries=JSON.parse(repair.files['.claude/settings.local.json']).hooks.PostToolUse;
  assert.equal(entries[0].hooks[0].command,'node old/skills/impeccable/scripts/hook.mjs');
  assert.equal(entries[1].hooks[0].command,'echo other');
  assert.match(entries[2].hooks[0].command,/skills\/phoenix-ui\/scripts\/phoenix-ui/);
  const alias=phoenixExpected('pin-i-impeccable-alias',readGolden('pin-i-impeccable-alias'));
  assert.equal(alias.files['.codex/skills/shape/SKILL.md'],undefined);
  assert.equal(alias.files['.codex/skills/i-impeccable/SKILL.md'],readGolden('pin-i-impeccable-alias').files['.codex/skills/i-impeccable/SKILL.md']);
  assert.ok(diffResults(alias,{...alias,files:{...alias.files,'.codex/skills/shape/SKILL.md':'unexpected shortcut'}}).length);
  for(const id of ['live-insert-svelte-component','live-accept-svelte-insert-empty-variant']) {
    const old=readGolden(id),expected=phoenixExpected(id,old);
    assert.ok(Object.values(expected.files).some((s)=>s.includes('.impeccable-insert-preview')));
    assert.ok(!Object.values(expected.files).some((s)=>s.includes('.phoenix-ui-insert-preview')));
  }
  const nuxt=phoenixExpected('live-inject-nuxt-srcdir',readGolden('live-inject-nuxt-srcdir'));
  assert.match(nuxt.files['app/plugins/impeccable-live.client.ts'],/script\.dataset\.impeccableLiveNuxt/);
  for(const entry of metadata.expectations) assert.equal(new Set(entry.fields.map((p)=>JSON.stringify(p.path.map(String)))).size,entry.fields.length,entry.id);
});

test('the hook administration executable mask handles quoted and bare current binary paths', async () => {
  const {loadPhoenixCases}=await adapter;
  const {runCase}=await loadPhoenixCases(process.execPath);
  const actual=runCase({id:'adapter-hook-command-mask-control',verb:'command-mask.js',setup(ws) {
    fs.writeFileSync(path.join(ws,'command-mask.js'),'const p=process.execPath;process.stdout.write([p+" hooks", "\\\'"+p+"\\\' hooks", "\\\""+p+"\\\" hooks"].join("\\n")+"\\n");');
  }},{impl:'bin',bin:process.execPath});
  assert.equal(actual.stdout,'<HOOK_ADMIN_CMD>\n<HOOK_ADMIN_CMD>\n<HOOK_ADMIN_CMD>\n');
  assert.equal(actual.exit,0);
  assert.equal(actual.stderr,'');
});

test('the only stale Italic delta is proven by the unchanged single-file golden', async () => {
  const { metadata, readGolden, phoenixExpected, diffResults } = await adapter;
  const delta = metadata.italic, historical = readGolden(delta.id);
  const expected = phoenixExpected(delta.id, historical);
  const oldFindings = JSON.parse(historical.stdout), findings = JSON.parse(expected.stdout);
  assert.equal(oldFindings.length, 436);
  assert.equal(findings.length, oldFindings.length + 1);
  assert.deepEqual(findings.filter((f) => f.snippet === delta.finding.snippet), [delta.finding]);
  assert.deepEqual(findings.filter((f) => f.snippet !== delta.finding.snippet), oldFindings);
  assert.deepEqual({ ...expected, stdout: '' }, { ...historical, stdout: '' });
  assert.deepEqual(diffResults(expected, expected), []);
  for (const mutate of [
    (list) => list.filter((f) => f.snippet !== delta.finding.snippet),
    (list) => [...list, delta.finding],
    (list) => list.map((f) => f.snippet === delta.finding.snippet ? { ...f, severity: 'error' } : f),
    (list) => list.slice(1),
    (list) => [{ ...list[0], snippet: 'changed unrelated finding' }, ...list.slice(1)],
  ]) {
    const bad = structuredClone(expected);
    bad.stdout = JSON.stringify(mutate(findings), null, 2) + '\n';
    assert.ok(diffResults(expected, bad).length);
  }
  const badExit = { ...expected, exit: 0 }, badError = { ...expected, stderr: 'new error\n' };
  assert.ok(diffResults(expected, badExit).length);
  assert.ok(diffResults(expected, badError).length);
});

test('the external-project cases retain their independent fixed DESIGN/config inputs', async () => {
  const { metadata, loadPhoenixCases } = await adapter;
  const { cases } = await loadPhoenixCases('/tmp/oracle-test-not-an-executable');
  const fixture = metadata.externalProjectFixture;
  assert.deepEqual(fixture.ids, ['detect-unknown-flag', 'detect-config-cross-project']);
  for (const id of fixture.ids) {
    const item = cases.find((c) => c.id === id);
    assert.ok(item.args.includes(`<WS>/${fixture.directory}/${fixture.target}`));
    assert.deepEqual(item.normalize.at(-1), [`<WS>/${fixture.directory}`, 'g', '<REPO>']);
  }
  assert.ok(fixture.files.some((f) => f.originalPath === 'DESIGN.md'));
  assert.ok(fixture.files.some((f) => f.originalPath === '.impeccable/config.json' && f.currentPath === '.phoenix-ui/config.json'));
  assert.ok(fixture.files.some((f) => f.originalPath === '.impeccable/design.json' && f.currentPath === '.phoenix-ui/design.json'));
});

test('the adapted harness really snapshots the renamed preview tree and compares process output', async () => {
  const { loadPhoenixCases, diffResults } = await adapter;
  const { runCase } = await loadPhoenixCases(process.execPath);
  const actual = runCase({
    id: 'adapter-preview-snapshot-control', verb: 'snapshot-control.js',
    setup(ws) {
      fs.writeFileSync(path.join(ws, 'snapshot-control.js'), 'process.stdout.write("controlled output\\n");');
      const preview = path.join(ws, 'node_modules/.phoenix-ui-live/session/v1.svelte');
      fs.mkdirSync(path.dirname(preview), { recursive: true });
      fs.writeFileSync(preview, '<p>Preview contract</p>\n');
    },
    files: ['node_modules/.phoenix-ui-live/**'],
  }, { impl: 'bin', bin: process.execPath });
  const expected = { stdout: 'controlled output\n', stderr: '', exit: 0, signal: null,
    files: { 'node_modules/.phoenix-ui-live/session/v1.svelte': '<p>Preview contract</p>\n' } };
  assert.deepEqual(actual, expected);
  assert.ok(diffResults(expected, { ...actual, files: {} }).length);
  assert.ok(diffResults(expected, { ...actual, stdout: 'different output\n' }).length);
});

test('the new checker refuses missing binaries instead of claiming a pass', async () => {
  const { checkOracle } = await import('../../scripts/check-phoenix-oracle.mjs');
  await assert.rejects(checkOracle({}), /explicit --bin/);
});
