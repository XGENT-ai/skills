'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const load = () => import('../../tools/phoenix-diagram/tests/support/p3-provider.mjs');

test('P3 cannot pass an empty two-host manifest', async () => {
  const {checkP3ProviderManifest} = await load();
  const report = checkP3ProviderManifest(
      {schemaVersion: 1, kind: 'phoenix-diagram-p3-provider-manifest', providers: []}, {});
  assert.equal(report.status, 'incomplete');
  assert(report.diagnostics.length > 0);
});

const path = require('node:path');
const fs = require('node:fs');
const os = require('node:os');
const {createHash} = require('node:crypto');
const root = path.resolve(__dirname, '../..');
const identity = value => ({sha256: createHash('sha256').update(value).digest('hex'), bytes: value.length});
const plain = value => Buffer.from(JSON.stringify(value) + '\n');
const physical = {
  device: '1',
  inode: '2',
  uid: 501,
  mode: 0o700
};
const fileBinding = {
  ...physical,
  links: 1,
  mode: 0o755
};
const scenarioManifest = () =>
    JSON.parse(fs.readFileSync(path.join(root, 'tools/phoenix-diagram/tests/provider/p3-scenarios.json')));
const helpers = async () => ({
  module: await load(),
  nativeProof: (await import('../../scripts/aggregate-phoenix-diagram-candidate.mjs')).P3_NATIVE_PROOF,
  provider: await import('../../tools/phoenix-diagram/tests/support/capture-p3-provider.mjs'),
  stdio: await import('../../tools/phoenix-diagram/tests/support/capture-p3-stdio.mjs')
});
function memory() {
  const files = new Map();
  return {
    files,
    put(file, value) {
      const content = Buffer.isBuffer(value) ? value :
          typeof value === 'string'          ? Buffer.from(value) :
                                               plain(value),
            reference = {path: file, identity: identity(content)};
      files.set(file, content);
      return reference;
    },
    bytesFor(ref) {
      assert(files.has(ref.path), 'missing fixture original ' + ref.path);
      return files.get(ref.path);
    }
  };
}
function events(memory, filename, rows) {
  const raw = Buffer.from(rows.map(value => JSON.stringify(value) + '\n').join('')),
        ref = memory.put(filename, raw);
  let offset = 0;
  const anchors = rows.map(row => {
    const bytes = plain(row), anchor = {offset, length: bytes.length, sha256: identity(bytes).sha256};
    offset += bytes.length;
    return anchor;
  });
  return {raw, ref, anchors};
}
async function sessionFixture(id = 'fallback-elintp') {
  const {
    module,
    nativeProof,
    provider
  } = await helpers(),
  mem = memory(), cwd = '/fixture/project', sessionId = '11111111-1111-4111-8111-111111111111',
  caller = id === 'fallback-documentation' ? 'documentation-writer' :
      id === 'erd-missing-facts'           ? 'phoenix-diagram' :
                                             'elintp',
  member = `skills/${caller}/SKILL.md`, source = fs.readFileSync(path.join(root, member)),
  file = mem.put(`${cwd}/.claude/${member}`, source),
  providerCollector = mem.put(
      '/fixture/harness/' + provider.COLLECTOR_MEMBER,
      fs.readFileSync(path.join(root, provider.COLLECTOR_MEMBER))),
  program = {
    path: '/fixture/bin/claude',
    identity: identity(Buffer.from('fixture client')),
    binding: fileBinding
  },
  version = mem.put(
      '/fixture/capture/version.json',
      {program, argv: ['--version'], exitCode: 0, signal: null, stdout: '2.1.312 (Claude Code)'});
  const rows = [
    {type: 'system', subtype: 'init', session_id: sessionId, model: 'fixture-observed-model'}, {
      type: 'assistant',
      message: {
        role: 'assistant',
        content: [{type: 'tool_use', id: 'read1', name: 'Read', input: {file_path: file.path}}]
      }
    },
    {
      type: 'user',
      message:
          {role: 'user', content: [{type: 'tool_result', tool_use_id: 'read1', content: source.toString()}]}
    },
    {
      type: 'assistant',
      message: {
        role: 'assistant',
        content: [{
          type: 'tool_use',
          id: 'discover1',
          name: 'Bash',
          input: {command: `ls '${cwd}/.agents/skills/phoenix-diagram/SKILL.md'`}
        }]
      }
    },
    {
      type: 'user',
      message: {
        role: 'user',
        content: [{
          type: 'tool_result',
          tool_use_id: 'discover1',
          content: 'No such file or directory',
          is_error: true
        }]
      }
    },
    {
      type: 'assistant',
      message: {
        role: 'assistant',
        content: [{
          type: 'text',
          text: 'Please provide minimum and maximum participation and nullability. I will stop here.'
        }]
      }
    },
    {type: 'result', is_error: false, session_id: sessionId}
  ];
  const transcript = events(mem, '/fixture/capture/stdout.jsonl', rows), observation = {
    schemaVersion: 1,
    kind: 'phoenix-diagram-p3-project-observation',
    cwd,
    project: physical,
    entries: [{path: `.claude/${member}`, kind: 'file', identity: file.identity, mode: 0o644}]
  },
        before = mem.put('/fixture/capture/before.json', observation),
        after = mem.put('/fixture/capture/after.json', observation), dto = {
          schemaVersion: 1,
          kind: provider.KIND,
          provider: 'claude-code',
          scenarioId: id,
          sessionId,
          cwd,
          project: physical,
          origin: true,
          previousCapture: null,
          collector: providerCollector,
          node: {path: '/usr/bin/node', version: process.version},
          program,
          entryPath: program.path,
          argv: provider.buildArguments(
              {
                provider: 'claude-code',
                cwd,
                endpointPolicy: 'inherit-config',
                model: 'fixture-requested-model'
              },
              {sessionId}),
          runtime: {
            policy: 'inherit-config',
            requestedModel: 'fixture-requested-model',
            endpoint: {category: 'client-default', origin: null, modelProvider: null},
            sources: [{source: 'request', key: 'model', value: 'fixture-requested-model'}],
            credentialSource: 'existing client authentication'
          },
          exitCode: 0,
          signal: null,
          timedOut: false,
          stdin: mem.put('/fixture/capture/stdin.txt', 'fixture prompt'),
          stdout: transcript.ref,
          stderr: mem.put('/fixture/capture/stderr.txt', ''),
          version,
          transcript: transcript.ref,
          format: 'claude-stream-json',
          transcriptStatus: 'captured',
          lineIndex: mem.put('/fixture/capture/index.json', provider.lineIndex(transcript.raw)),
          observedModels: [{value: 'fixture-observed-model', anchor: transcript.anchors[0]}],
          projectBefore: before,
          projectAfter: after,
          startedAt: '2026-10-29T00:00:00.000Z'
        };
  const capture = mem.put('/fixture/capture/capture.json', dto), row = {
    id,
    sessionId,
    evidence: {
      kind: id,
      discovery: [
        {captureIndex: 0, call: transcript.anchors[3], result: transcript.anchors[4], toolUseId: 'discover1'}
      ]
    }
  },
        session = {
          id: sessionId,
          provider: 'claude-code',
          scenarioId: id,
          cwd,
          project: physical,
          providerCollector,
          captures: [{capture, transcript: transcript.ref, version}],
          loadedReads: [{
            captureIndex: 0,
            call: transcript.anchors[1],
            result: transcript.anchors[2],
            toolUseId: 'read1',
            file,
            packageMember: member,
            quote: source.toString().slice(0, 80)
          }],
          commands: []
        };
  const engine = {sha256: 'a'.repeat(64), size: 10}, cohort = {
    source: {commit: 'b'.repeat(40), inputSha256: 'c'.repeat(64)},
    bundleSha256: 'd'.repeat(64),
    versionSha256: 'e'.repeat(64),
    engines: {'darwin-arm64': engine}
  },
        context = {
          provider: 'claude-code',
          scenarioManifest: scenarioManifest(),
          collectorIdentity: {
            provider: providerCollector.identity,
            stdio: identity(
                fs.readFileSync(path.join(root, 'tools/phoenix-diagram/tests/support/capture-p3-stdio.mjs')))
          },
          nativeProof,
          bytesFor: mem.bytesFor,
          tarInventory: new Map([[member, {sha256: file.identity.sha256, size: file.identity.bytes}]]),
          cohort
        };
  if (id === 'erd-missing-facts')
    session.installedReceipt = mem.put('/fixture/original-active.json', {
      schemaVersion: 1,
      tool: 'phoenix-diagram',
      project: {device: physical.device, inode: physical.inode},
      bundleManifestSha256: cohort.bundleSha256,
      engineManifestSha256: cohort.versionSha256,
      engine:
          {sourceInputSha256: cohort.source.inputSha256, binary: {sha256: engine.sha256, bytes: engine.size}},
      ownership: [{path: `.claude/${member}`, identity: file.identity}]
    });
  return {
    module,
    provider,
    mem,
    session,
    row,
    context,
    dto,
    rows,
    transcript,
    observation,
    reseal() {
      session.captures[0].capture = mem.put('/fixture/capture/capture.json', dto);
    }
  };
}

test('frozen P3 manifest has the same 18 complete topics/materials for both hosts', async () => {
  const {module} = await helpers(), manifest = scenarioManifest();
  assert.equal(module.checkP3ScenarioManifest(manifest), manifest);
  assert.equal(manifest.rows.length, 18);
  const altered = structuredClone(manifest);
  altered.rows[0].prompt += 'hidden change';
  assert.throws(() => module.checkP3ScenarioManifest(altered), /prompt identity/);
  const sampled = structuredClone(manifest);
  sampled.providers.codex.pop();
  assert.throws(() => module.checkP3ScenarioManifest(sampled), /same 18/);
});

test('P3 coverage is a bijection of 18 fresh origins and physical projects', async () => {
  const {module} = await helpers(), record = {
    scenarios: module.P3_SCENARIO_IDS.map((id, i) => ({id, sessionId: 'session-' + i})),
    sessions: module.P3_SCENARIO_IDS.map((id, i) => ({
                                           id: 'session-' + i,
                                           scenarioId: id,
                                           cwd: '/fixture/' + i,
                                           project: {...physical, inode: String(i + 1)}
                                         }))
  };
  assert.equal(module.checkP3Coverage(record), true);
  const shared = structuredClone(record);
  shared.sessions[1].project = shared.sessions[0].project;
  assert.throws(() => module.checkP3Coverage(shared), /shared across/);
  const reused = structuredClone(record);
  reused.scenarios[1].sessionId = reused.scenarios[0].sessionId;
  assert.throws(() => module.checkP3Coverage(reused), /another scenario/);
});

test('fallback really reads the caller from the tgz without a PD receipt or pdCall', async () => {
  const f = await sessionFixture();
  assert.equal(f.module.checkP3Session(f.session, f.row, f.context).parts.length, 1);
  f.session.installedReceipt = {path: '/fake', identity: {sha256: 'f'.repeat(64), bytes: 0}};
  assert.throws(() => f.module.checkP3Session(f.session, f.row, f.context), /Fallback cannot fabricate/);
});

test('caller bytes cannot masquerade as PD ownership and must match authenticated tgz', async () => {
  const f = await sessionFixture();
  f.context.tarInventory.set('skills/elintp/SKILL.md', {sha256: '0'.repeat(64), size: 10});
  assert.throws(() => f.module.checkP3Session(f.session, f.row, f.context), /authenticated tgz/);
});

test('observed model must be a real native metadata line, requested model is separate', async () => {
  const f = await sessionFixture();
  assert.notEqual(f.dto.runtime.requestedModel, f.dto.observedModels[0].value);
  f.module.checkP3Session(f.session, f.row, f.context);
  f.dto.observedModels[0].value = 'invented';
  f.reseal();
  assert.throws(() => f.module.checkP3Session(f.session, f.row, f.context), /Observed model/);
});

test('same-case confirmation may continue but history and other cases cannot seed an origin', async () => {
  const f = await sessionFixture(), origin = f.session.captures[0].capture,
        continued = structuredClone(f.dto);
  continued.origin = false;
  continued.previousCapture = origin;
  continued.argv = f.provider.buildArguments(
      {
        provider: 'claude-code',
        cwd: f.session.cwd,
        endpointPolicy: 'inherit-config',
        model: 'fixture-requested-model'
      },
      {previousId: f.session.id});
  const continuation = f.mem.put('/fixture/continued/capture.json', continued);
  f.session.captures.push(
      {capture: continuation, transcript: continued.transcript, version: continued.version});
  assert.equal(f.module.checkP3Session(f.session, f.row, f.context).parts.length, 2);
  continued.scenarioId = 'migrate-known';
  f.session.captures[1].capture = f.mem.put(continuation.path, continued);
  assert.throws(() => f.module.checkP3Session(f.session, f.row, f.context), /identity\/project differs/);
  f.session.captures.pop();
  f.dto.argv = continued.argv;
  f.reseal();
  assert.throws(() => f.module.checkP3Session(f.session, f.row, f.context), /fresh origin/);
});

test('ERD clarification stops without candidate writes, including write-then-delete tool calls', async () => {
  const f = await sessionFixture('erd-missing-facts'),
        captured = f.module.checkP3Session(f.session, f.row, f.context),
        response = 'Please provide participation and nullability. I will stop here.', row = {
          ...f.row,
          evidence: {
            kind: f.row.id,
            questions: ['participation and nullability'],
            stoppedQuote: 'I will stop here.'
          }
        };
  f.module.checkP3ScenarioOutcome(row, captured, [], response, f.context);
  captured.parts[0].events.push({
    type: 'assistant',
    message: {
      role: 'assistant',
      content: [{
        type: 'tool_use',
        id: 'write-deleted',
        name: 'Write',
        input: {file_path: '/fixture/project/candidate.json', content: '{}'}
      }]
    }
  });
  assert.throws(
      () => f.module.checkP3ScenarioOutcome(row, captured, [], response, f.context), /wrote a candidate/);
});

async function commandFixture(
    {name = 'brands', previewEvents, mode = 'installed', reply, argvOverride, textStdout, exitCode = 0} =
        {}) {
  const f = await sessionFixture(), {stdio} = await helpers(), mem = f.mem, context = f.context,
        cwd = f.session.cwd,
        collector = mem.put(
            '/fixture/harness/' + stdio.COLLECTOR_MEMBER,
            fs.readFileSync(path.join(root, stdio.COLLECTOR_MEMBER))),
        launcherBytes = fs.readFileSync(path.join(root, 'skills/phoenix-diagram/scripts/phoenix-diagram')),
        launcher = {
          path: cwd + '/.claude/skills/phoenix-diagram/scripts/phoenix-diagram',
          identity: identity(launcherBytes),
          binding: fileBinding
        };
  context.tarInventory.set(
      'skills/phoenix-diagram/scripts/phoenix-diagram',
      {sha256: launcher.identity.sha256, size: launcher.identity.bytes});
  const candidate = {
    sourceInputSha256: context.cohort.source.inputSha256,
    bundleSha256: context.cohort.bundleSha256,
    engines: context.cohort.engines
  },
        candidateVersion = mem.put('/fixture/stdio/version.json', candidate);
  context.cohort.versionSha256 = candidateVersion.identity.sha256;
  const engine = Object.values(context.cohort.engines)[0], activeValue = {
    schemaVersion: 1,
    tool: 'phoenix-diagram',
    engineManifestSha256: context.cohort.versionSha256,
    bundleManifestSha256: context.cohort.bundleSha256,
    engine: {
      sourceInputSha256: context.cohort.source.inputSha256,
      binary: {sha256: engine.sha256, bytes: engine.size}
    },
    ownership: [{
      path: '.claude/skills/phoenix-diagram/scripts/phoenix-diagram',
      identity: {...fileBinding, ...launcher.identity}
    }]
  },
        activeBytes = plain(activeValue),
        state = phase => ({
          active: {
            path: cwd + '/.phoenix-diagram/install/active.json',
            state: 'present',
            parent: {path: cwd, binding: physical},
            identity: identity(activeBytes),
            binding: fileBinding,
            snapshot: mem.put(`/fixture/stdio/${phase}-active.json`, activeBytes)
          },
          launchers: ['.agents', '.claude', '.codex'].map(
              host => ({
                path: `${cwd}/${host}/skills/phoenix-diagram/scripts/phoenix-diagram`,
                state: 'absent',
                parent: {path: cwd, binding: physical}
              }))
        });
  const argv = argvOverride ||
      (name === 'brands' ?
           ['brands', 'capture', 'https://example.test/icon.png', '--json'] :
           name === 'migrate' ?
           ['migrate', '--from-archify', '--project', cwd, '--confirm-remove-archify', '--json'] :
           ['preview', 'architecture', 'input.json', 'output.html', '--port', '0']);
  const defaultReply = {
    schemaVersion: 1,
    tool: 'phoenix-diagram',
    toolVersion: '0.2.0',
    command: name === 'brands' ? 'brands capture' : name,
    status: 'pass',
    diagnostics: [],
    brand: {url: 'https://example.test/icon.png', sha256: '1'.repeat(64)}
  };
  const dto = {
    schemaVersion: 1,
    kind: stdio.KIND,
    tool: 'phoenix-diagram',
    mode,
    cwd,
    project: physical,
    collector,
    node: {path: '/usr/bin/node', version: process.version},
    program: mode === 'migration' ? {
      path: '/fixture/release/phoenix-diagram-darwin-arm64',
      identity: {sha256: engine.sha256, bytes: engine.size},
      binding: fileBinding
    } :
                                    launcher,
    argv,
    candidateVersion,
    installationBefore: state('before'),
    installationAfter: state('after'),
    stdout: mem.put(
        '/fixture/stdio/stdout',
        textStdout !== undefined ? Buffer.from(textStdout) :
            previewEvents ? Buffer.from(previewEvents.map(value => JSON.stringify(value) + '\n').join('')) :
                            plain(reply || defaultReply)),
    stderr: mem.put('/fixture/stdio/stderr', ''),
    exitCode,
    signal: null,
    startedAt: '2026-10-29T01:00:00.000Z'
  };
  const capture = mem.put('/fixture/stdio/capture.json', dto),
        options =
            [
              '--mode', mode, '--program', dto.program.path, '--cwd', cwd, '--candidate-version',
              '/fixture/source/VERSION.json', '--stdout', dto.stdout.path, '--stderr', dto.stderr.path,
              '--evidence-dir', '/fixture/stdio', '--', ...argv
            ],
        commandText = [dto.node.path, collector.path, ...options].map(value => '\'' + value + '\'').join(' '),
        rows =
            [
              {
                type: 'assistant',
                message: {
                  role: 'assistant',
                  content: [{type: 'tool_use', id: 'pd1', name: 'Bash', input: {command: commandText}}]
                }
              },
              {
                type: 'user',
                message: {
                  role: 'user',
                  content: [{
                    type: 'tool_result',
                    tool_use_id: 'pd1',
                    content: JSON.stringify({...dto, captureFile: capture})
                  }]
                }
              }
            ],
        native = events(mem, '/fixture/pd-proof.jsonl', rows), captured = {
          ...f.session,
          scenarioId: name === 'migrate' ? 'migrate-known' : 'brand-capture',
          parts:
              [{session: f.session, provider: 'claude-code', raw: native.raw, events: rows, capture: {cwd}}]
        },
        command = {
          captureIndex: 0,
          capture,
          proof: {call: native.anchors[0], result: native.anchors[1], toolUseId: 'pd1'}
        };
  return {...f, dto, captured, command, context};
}

test('brands capture binds the two-word command to actual argv and original collector bytes', async () => {
  const f = await commandFixture();
  assert.equal(f.module.checkP3Command(f.captured, f.command, f.context).receipt.command, 'brands capture');
  const wrong = await commandFixture({
    reply: {
      schemaVersion: 1,
      tool: 'phoenix-diagram',
      toolVersion: '0.2.0',
      command: 'brands',
      status: 'pass',
      diagnostics: []
    }
  });
  assert.throws(
      () => wrong.module.checkP3Command(wrong.captured, wrong.command, wrong.context), /header differs/);
  f.context.collectorIdentity.stdio = {sha256: '0'.repeat(64), bytes: 0};
  assert.throws(() => f.module.checkP3Command(f.captured, f.command, f.context), /authenticated harness/);
});

test('preview original stdout is NDJSON with monotone state and real terminal stop', async () => {
  const header = {
    schemaVersion: 1,
    tool: 'phoenix-diagram',
    toolVersion: '0.2.0',
    command: 'preview',
    status: 'ready',
    diagnostics: [],
    sessionId: 'preview-session'
  },
        events =
            [
              {...header, event: 'started', generation: 0, lastSuccessfulGeneration: 0},
              {...header, event: 'updated', generation: 1, lastSuccessfulGeneration: 1},
              {...header, event: 'failed', generation: 2, lastSuccessfulGeneration: 1},
              {...header, event: 'updated', generation: 3, lastSuccessfulGeneration: 3}, {
                ...header,
                event: 'stopped',
                generation: 3,
                lastSuccessfulGeneration: 3,
                shutdownComplete: true,
                exitCode: 0
              }
            ],
        f = await commandFixture({name: 'preview', previewEvents: events});
  assert.equal(f.module.checkP3Command(f.captured, f.command, f.context).events.length, 5);
  events.at(-1).shutdownComplete = false;
  const bad = await commandFixture({name: 'preview', previewEvents: events});
  assert.throws(
      () => bad.module.checkP3Command(bad.captured, bad.command, bad.context), /stop is not confirmed/);
});

test(
    'migration uses fixed native engine and keeps distinct before/after installation snapshots', async () => {
      const reply = {
        schemaVersion: 1,
        tool: 'phoenix-diagram',
        toolVersion: '0.2.0',
        command: 'migrate',
        status: 'installed',
        diagnostics: [],
        legacyBackupPolicy: 'none',
        legacyRestoration: 'not-supported'
      },
            f = await commandFixture({name: 'migrate', mode: 'migration', reply});
      assert.equal(f.module.checkP3Command(f.captured, f.command, f.context).receipt.status, 'installed');
      const wrong = await commandFixture({name: 'migrate', mode: 'installed', reply});
      assert.throws(
          () => wrong.module.checkP3Command(wrong.captured, wrong.command, wrong.context),
          /fixed candidate engine/);
    });

test(
    'provider launcher inherits service selection, separates requested models, and never serializes tokens',
    async () => {
      const {provider} = await helpers(),
            tmp = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'p3-public-runtime-')));
      try {
        fs.mkdirSync(path.join(tmp, '.claude'));
        fs.writeFileSync(
            path.join(tmp, '.claude/settings.json'),
            JSON.stringify({env: {ANTHROPIC_AUTH_TOKEN: 'fixture-secret-not-for-evidence'}}));
        const runtime = provider.observeRuntime(
            'claude-code', {
              CLAUDE_CONFIG_DIR: path.join(tmp, '.claude'),
              ANTHROPIC_BASE_URL: 'https://user:password@example.test/v1?token=fixture-secret#fragment',
              ANTHROPIC_AUTH_TOKEN: 'fixture-secret-not-for-evidence'
            },
            'requested-model', tmp);
        assert.equal(runtime.endpoint.origin, 'https://example.test');
        assert.equal(runtime.requestedModel, 'requested-model');
        assert(!JSON.stringify(runtime).includes('fixture-secret'));
        assert(!JSON.stringify(runtime).includes('password'));
        const args = provider.buildArguments(
            {provider: 'claude-code', endpointPolicy: 'inherit-config', model: 'requested-model'},
            {previousId: 'same-session'});
        assert(args.includes('--resume'));
        assert(!args.includes('--no-session-persistence'));
        assert.throws(
            () => provider.buildArguments({provider: 'codex', endpointPolicy: 'replace-service'}),
            /inherit-config/);
      } finally {
        fs.rmSync(tmp, {recursive: true, force: true});
      }
    });

test('P3 migration collector seals before bytes before the child changes active/launcher', async () => {
  const {stdio} = await helpers(),
        base = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'p3-stdio-transition-'))),
        project = path.join(base, 'project'), out = path.join(project, 'stdio');
  fs.mkdirSync(project, {mode: 0o700});
  fs.mkdirSync(out, {mode: 0o700});
  fs.mkdirSync(path.join(project, '.phoenix-diagram/install'), {recursive: true, mode: 0o700});
  fs.mkdirSync(path.join(project, '.agents/skills/phoenix-diagram/scripts'), {recursive: true, mode: 0o700});
  const active = path.join(project, '.phoenix-diagram/install/active.json'),
        launcher = path.join(project, '.agents/skills/phoenix-diagram/scripts/phoenix-diagram');
  fs.writeFileSync(active, '{"before":true}\n');
  fs.writeFileSync(launcher, 'owned before launcher\n');
  const
      fakeEngine = path.join(base, 'fixture-engine'),
      script = Buffer.from(
          '#!/bin/sh\nprintf \'{"after":true}\\n\' > .phoenix-diagram/install/new-active.json\nmv .phoenix-diagram/install/new-active.json .phoenix-diagram/install/active.json\nrm .agents/skills/phoenix-diagram/scripts/phoenix-diagram\nprintf \'{"schemaVersion":1,"tool":"phoenix-diagram","toolVersion":"0.2.0","command":"migrate","status":"installed","diagnostics":[]}\\n\'\n');
  fs.writeFileSync(fakeEngine, script, {mode: 0o755});
  const version = path.join(base, 'VERSION.json');
  fs.writeFileSync(
      version, JSON.stringify({engines: {fixture: {sha256: identity(script).sha256, size: script.length}}}));
  try {
    const dto = await stdio.captureStdio({
      mode: 'migration',
      program: fakeEngine,
      cwd: project,
      candidateVersion: version,
      stdout: path.join(out, 'stdout'),
      stderr: path.join(out, 'stderr'),
      evidenceDir: path.join(out, 'evidence'),
      argv: ['migrate', '--json']
    });
    assert.equal(dto.exitCode, 0);
    assert.equal(fs.readFileSync(dto.installationBefore.active.snapshot.path, 'utf8'), '{"before":true}\n');
    assert.equal(fs.readFileSync(dto.installationAfter.active.snapshot.path, 'utf8'), '{"after":true}\n');
    assert.equal(dto.installationBefore.launchers[0].state, 'present');
    assert.equal(dto.installationAfter.launchers[0].state, 'absent');
    assert.notEqual(dto.installationBefore.active.binding.inode, dto.installationAfter.active.binding.inode);
    assert.equal(
        JSON.parse(fs.readFileSync(dto.captureFile.path)).installationBefore.active.identity.sha256,
        identity(Buffer.from('{"before":true}\n')).sha256);
    await assert.rejects(
        () => stdio.captureStdio({
          mode: 'migration',
          program: fakeEngine,
          cwd: project,
          candidateVersion: version,
          stdout: path.join(out, 'stdout'),
          stderr: path.join(out, 'stderr'),
          evidenceDir: path.join(out, 'evidence'),
          argv: ['migrate']
        }),
        /new project-local capture slots/);
  } finally {
    fs.rmSync(base, {recursive: true, force: true});
  }
});

test('P3 native provider launcher plan constructs client commands without starting a provider', async () => {
  const {provider} = await helpers();
  const args = provider.buildArguments(
      {provider: 'codex', cwd: '/fixture/new-case', endpointPolicy: 'inherit-config', model: 'fixture-model'},
      {sessionId: 'fresh'});
  assert.equal(args[0], 'exec');
  assert(args.includes('--json'));
  assert(!args.includes('--ignore-user-config'));
  assert(!args.includes('--ephemeral'));
  assert(!args.includes('resume'));
  const continued = provider.buildArguments(
      {provider: 'codex', cwd: '/fixture/new-case', endpointPolicy: 'inherit-config'},
      {previousId: 'same-session'});
  assert.deepEqual(continued.slice(0, 2), ['exec', 'resume']);
  assert(continued.includes('same-session'));
  assert(!continued.includes('--last'));
});

test('origin prompt is the complete frozen neutral protocol plus topic without an extra prefix', async () => {
  const {module} = await helpers(), manifest = scenarioManifest();
  const row = {
    id: 'fallback-elintp',
    parameters: {},
    preambleParameters: {
      PROJECT: '/fixture/project',
      NODE: '/usr/bin/node',
      COLLECTOR: '/fixture/harness/tools/phoenix-diagram/tests/support/capture-p3-stdio.mjs',
      VERSION: '/fixture/candidate/VERSION.json',
      STDIO: '/fixture/project/stdio'
    }
  };
  const prompt = module.renderP3OriginPrompt(manifest, row);
  assert(prompt.endsWith(manifest.rows.find(value => value.id === row.id).prompt));
  assert(prompt.includes('如果任务需要执行'));
  assert(!prompt.includes('{{'));
  assert.throws(
      () => module.renderP3OriginPrompt(
          manifest, {...row, preambleParameters: {...row.preambleParameters, hidden: 'answer'}}),
      /parameters differ/);
});

test(
    'Claude caller Skill injection binds the complete caller body instead of PD-only ownership', async () => {
      const f = await sessionFixture(), read = f.session.loadedReads[0],
            source = f.mem.bytesFor(read.file).toString(),
            body = /^---\n[\s\S]*?\n---\n\n([\s\S]*)$/.exec(source)[1];
      const rebuild = truncated => {
        const rows = [
          {type: 'system', subtype: 'init', session_id: f.session.id, model: 'fixture-observed-model'}, {
            type: 'assistant',
            session_id: f.session.id,
            message: {
              role: 'assistant',
              content: [{type: 'tool_use', id: 'read1', name: 'Skill', input: {skill: 'elintp'}}]
            }
          },
          {
            type: 'user',
            session_id: f.session.id,
            tool_use_result: {success: true, commandName: 'elintp'},
            message: {
              role: 'user',
              content: [{type: 'tool_result', tool_use_id: 'read1', content: 'Launching skill: elintp'}]
            }
          },
          {
            type: 'user',
            session_id: f.session.id,
            isSynthetic: true,
            message: {
              role: 'user',
              content: [{
                type: 'text',
                text: 'Base directory for this skill: ' + path.posix.dirname(read.file.path) + '\n\n' +
                    (truncated ? body.slice(0, -10) : body)
              }]
            }
          },
          ...f.rows.slice(3)
        ];
        const transcript = events(f.mem, '/fixture/capture/stdout.jsonl', rows);
        f.dto.stdout = transcript.ref;
        f.dto.transcript = transcript.ref;
        f.dto.lineIndex = f.mem.put('/fixture/capture/index.json', f.provider.lineIndex(transcript.raw));
        f.dto.observedModels[0].anchor = transcript.anchors[0];
        f.session.captures[0].transcript = transcript.ref;
        Object.assign(read, {
          call: transcript.anchors[1],
          result: transcript.anchors[2],
          injection: transcript.anchors[3],
          quote: body.slice(0, 80)
        });
        Object.assign(
            f.row.evidence.discovery[0], {call: transcript.anchors[4], result: transcript.anchors[5]});
        f.reseal();
      };
      rebuild(false);
      f.module.checkP3Session(f.session, f.row, f.context);
      rebuild(true);
      assert.throws(
          () => f.module.checkP3Session(f.session, f.row, f.context), /complete authenticated source/);
    });

test('migration can truthfully use fixed receipts without inventing installed PD reads', async () => {
  const f = await sessionFixture('migrate-conflict'), rows = [f.rows[0], f.rows[5], f.rows[6]],
        transcript = events(f.mem, '/fixture/capture/stdout.jsonl', rows);
  f.dto.stdout = transcript.ref;
  f.dto.transcript = transcript.ref;
  f.dto.lineIndex = f.mem.put('/fixture/capture/index.json', f.provider.lineIndex(transcript.raw));
  f.dto.observedModels[0].anchor = transcript.anchors[0];
  f.session.captures[0].transcript = transcript.ref;
  f.session.loadedReads = [];
  const observation = {...f.observation, entries: []};
  f.dto.projectBefore = f.mem.put('/fixture/capture/before.json', observation);
  f.dto.projectAfter = f.mem.put('/fixture/capture/after.json', observation);
  f.reseal();
  assert.equal(f.module.checkP3Session(f.session, f.row, f.context).parts.length, 1);
  assert.equal(f.session.installedReceipt, undefined);
});

test(
    'all actual PD producers must be sealed, so omitted preview calls cannot disappear from a record',
    async () => {
      const f = await commandFixture();
      assert.equal(f.module.checkP3RecordedCommands(f.captured, [f.command], f.context), true);
      assert.throws(
          () => f.module.checkP3RecordedCommands(f.captured, [], f.context), /absent from the recorded/);
      f.captured.parts[0].events.push({
        type: 'assistant',
        message: {
          role: 'assistant',
          content: [{
            type: 'tool_use',
            id: 'unsealed-preview',
            name: 'Bash',
            input: {command: 'phoenix-diagram preview architecture input.json output.html'}
          }]
        }
      });
      assert.throws(() => f.module.checkP3RecordedCommands(f.captured, [f.command], f.context), /unsealed/);
    });

test(
    'ordinary tasks cannot authorize preview, capture or comparison through their evidence record',
    async () => {
      const f = await sessionFixture(), captured = f.module.checkP3Session(f.session, f.row, f.context);
      for (const argv of [['preview'], ['brands', 'capture'], ['compare']])
        assert.throws(
            () => f.module.checkP3ScenarioOutcome(
                {id: 'normal-no-preview', evidence: {kind: 'normal-no-preview'}}, captured, [{argv}],
                'fixture response', f.context),
            /explicitly/);
    });

test('legacy topic actually carries brand, declared repository sources, views and animation', () => {
  const row = scenarioManifest().rows.find(value => value.id === 'legacy-dispositions'),
        input = JSON.parse(row.materials[0].content);
  assert(input.components.some(value => value.brand === 'postgresql'));
  assert(input.components.some(value => value.sources?.length));
  assert(input.meta.repository.url);
  assert(input.meta.views.length);
  assert.equal(input.meta.animation, 'trace');
});

test('ERD clarification cannot hide write-then-delete inside an unknown JavaScript executor', async () => {
  const f = await sessionFixture('erd-missing-facts'),
        captured = f.module.checkP3Session(f.session, f.row, f.context),
        row = {...f.row, evidence: {kind: f.row.id, questions: ['participation'], stoppedQuote: 'stop here'}};
  captured.parts[0].events.push({
    type: 'response_item',
    payload: {
      type: 'function_call',
      call_id: 'unknown-exec',
      name: 'functions.exec',
      arguments: JSON.stringify(
          {code: 'const fd=fs.openSync(candidate,"w");fs.closeSync(fd);fs.unlinkSync(candidate);'})
    }
  });
  assert.throws(
      () => f.module.checkP3ScenarioOutcome(row, captured, [], 'participation; stop here', f.context),
      /verifiable read-only/);
});

test('migration text help seals stdout without counting as installation or recovery', async () => {
  const options = {
    name: 'migrate',
    mode: 'migration',
    argvOverride: ['migrate', '--help'],
    textStdout: 'Usage: phoenix-diagram migrate --from-archify | --recover | --rollback\n'
  };
  const f = await commandFixture(options), result = f.module.checkP3Command(f.captured, f.command, f.context);
  assert.equal(result.help, true);
  assert.equal(result.receipt, undefined);
  assert.equal(f.context.bytesFor(result.stdout).toString(), options.textStdout);
  assert.throws(
      () => f.module.checkP3ScenarioOutcome(
          {id: 'migrate-conflict', evidence: {kind: 'migrate-conflict'}}, f.captured, [result], 'help only',
          f.context),
      /Migration/);
  const shortHelp = await commandFixture({...options, argvOverride: ['migrate', '-h']});
  assert.equal(
      shortHelp.module.checkP3Command(shortHelp.captured, shortHelp.command, shortHelp.context).help, true);
  const wrongEngine = await commandFixture(options);
  wrongEngine.context.cohort.engines['darwin-arm64'] = {sha256: '0'.repeat(64), size: 10};
  assert.throws(
      () => wrongEngine.module.checkP3Command(wrongEngine.captured, wrongEngine.command, wrongEngine.context),
      /Migration help/);
  const wrongScenario = await commandFixture(options);
  wrongScenario.captured.scenarioId = 'normal-no-preview';
  assert.throws(
      () => wrongScenario.module.checkP3Command(
          wrongScenario.captured, wrongScenario.command, wrongScenario.context),
      /Migration help/);
  const failed = await commandFixture({...options, argvOverride: ['migrate', '-h'], exitCode: 1});
  assert.throws(
      () => failed.module.checkP3Command(failed.captured, failed.command, failed.context), /Migration help/);
});

test('installed text help retains candidate ownership checks and does not count as a preview', async () => {
  for (const argvOverride of [['guide', '--help'], ['preview', '-h'], ['--help']]) {
    const f = await commandFixture({argvOverride, textStdout: 'Usage: phoenix-diagram\n'});
    const result = f.module.checkP3Command(f.captured, f.command, f.context);
    assert.equal(result.help, true); assert.equal(result.receipt, undefined); assert.equal(result.events, undefined);
  }
  const wrong = await commandFixture({argvOverride: ['guide', '--help'], textStdout: 'Usage\n'});
  wrong.context.tarInventory.set('skills/phoenix-diagram/scripts/phoenix-diagram', {sha256: '0'.repeat(64), size: 10});
  assert.throws(() => wrong.module.checkP3Command(wrong.captured, wrong.command, wrong.context), /Launcher|candidate/);
  const failed = await commandFixture({argvOverride: ['guide', '--help'], textStdout: 'Usage\n', exitCode: 1});
  assert.throws(() => failed.module.checkP3Command(failed.captured, failed.command, failed.context), /help/i);
});
