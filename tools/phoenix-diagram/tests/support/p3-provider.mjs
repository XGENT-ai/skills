// Pure verification shared by the final-candidate controller and aggregator.
// Native transcript parsing is supplied by the existing aggregator; no cycle.
// A record has package:{version,tarball}, source/candidateSha256, 18 sessions,
// and 18 scenarios. Each session owns captures:[{capture,transcript,version}],
// loadedReads, collector:{nodePath,script}, providerCollector, and commands:
// [{captureIndex,proof,capture,expectedInterruption?}]. Scenarios bind the frozen
// prompt/parameters/preambleParameters/materials, a final response anchor,
// evidence, and an independent review FileRef. FileRefs always carry the original
// path and byte identity; archived bytesFor must never silently reopen a newer file.
import {createHash} from 'node:crypto';
import path from 'node:path';
import {isDeepStrictEqual as same} from 'node:util';

import {buildArguments, COLLECTOR_MEMBER as PROVIDER_MEMBER, KIND as PROVIDER_KIND, lineIndex} from './capture-p3-provider.mjs';
import {COLLECTOR_MEMBER as STDIO_MEMBER, KIND as STDIO_KIND, parseCaptureArgs} from './capture-p3-stdio.mjs';

export {PROVIDER_MEMBER, STDIO_MEMBER};

export const P3_SCENARIO_IDS = Object.freeze([
  'brand-builtin', 'brand-no-url', 'brand-capture', 'brand-missing', 'preview-explicit', 'normal-no-preview',
  'compare-valid', 'compare-invalid', 'migrate-known', 'migrate-conflict', 'migrate-recover',
  'legacy-dispositions', 'caller-elintp', 'caller-documentation', 'fallback-elintp', 'fallback-documentation',
  'workflow-facts', 'erd-missing-facts'
]);
const PROVIDERS = ['codex', 'claude-code'], HASH = /^[a-f0-9]{64}$/,
      FALLBACK = new Set(['fallback-elintp', 'fallback-documentation']),
      MIGRATION = new Set(['migrate-known', 'migrate-conflict', 'migrate-recover']);
const VISUAL = new Set([
  'brand-builtin', 'brand-no-url', 'brand-capture', 'preview-explicit', 'normal-no-preview', 'compare-valid',
  'legacy-dispositions', 'caller-elintp', 'caller-documentation', 'fallback-elintp', 'fallback-documentation',
  'workflow-facts'
]);
const hash = bytes => ({sha256: createHash('sha256').update(bytes).digest('hex'), bytes: bytes.length});
const MANAGER_MARKER = hash(Buffer.from('{"skills":{"archify":{}}}\n'));
function need(value, message) {
  if (!value) {
    const error = new Error(message);
    error.code = 'p3/provider-incomplete';
    throw error;
  }
}
function object(value) {
  return value && typeof value === 'object' && !Array.isArray(value);
}
function identity(value) {
  return value && HASH.test(value.sha256) && Number.isSafeInteger(value.bytes) && value.bytes >= 0;
}
function reference(value) {
  return object(value) && typeof value.path === 'string' && path.isAbsolute(value.path) &&
      identity(value.identity);
}
function bytes(referenceValue, context, label) {
  need(reference(referenceValue), label + ' reference missing');
  const value = context.bytesFor(referenceValue);
  need(
      Buffer.isBuffer(value) && same(hash(value), referenceValue.identity),
      label + ' original bytes/digest differ');
  return value;
}
function json(referenceValue, context, label) {
  const original = bytes(referenceValue, context, label);
  try {
    return JSON.parse(original);
  } catch {
    need(false, label + ' is not valid JSON');
  }
}
function sameRef(a, b) {
  return reference(a) && reference(b) && a.path === b.path && same(a.identity, b.identity);
}
function expectedFile(referenceValue, member, context, label) {
  const value = bytes(referenceValue, context, label), expected = context.tarInventory.get(member);
  need(
      expected && same(hash(value), {sha256: expected.sha256, bytes: expected.size ?? expected.bytes}),
      label + ' differs from authenticated tgz member ' + member);
  return value;
}
function physical(value) {
  return object(value) && typeof value.device === 'string' && typeof value.inode === 'string' &&
      Number.isInteger(value.uid) && Number.isInteger(value.mode);
}
function sameProject(a, b) {
  return a?.device === b?.device && a?.inode === b?.inode && a?.uid === b?.uid && a?.mode === b?.mode;
}
function safeOrigin(value) {
  try {
    const url = new URL(value);
    return ['http:', 'https:'].includes(url.protocol) && url.origin === value && !url.username &&
        !url.password && !url.search && !url.hash;
  } catch {
    return false;
  }
}
function noSecrets(value) {
  const scan = current => {
    if (!object(current) && !Array.isArray(current)) return;
    for (const [key, child] of Object.entries(current)) {
      need(
          !/(?:auth_token|api_key|password|secret|authorization)$/i.test(key),
          'Secret-bearing metadata key is forbidden');
      if (typeof child === 'string')
        need(
            !/(?:Bearer\s+\S+|[?&](?:token|key|auth)=)/i.test(child),
            'Sensitive runtime metadata is forbidden');
      scan(child);
    }
  };
  scan(value);
}
export function checkP3ScenarioManifest(manifest) {
  need(
      manifest?.schemaVersion === 1 && manifest.kind === 'phoenix-diagram-p3-provider-scenarios' &&
          manifest.version === 'p3-v1',
      'P3 scenario manifest kind/version differs');
  need(same(manifest.rows?.map(row => row.id), P3_SCENARIO_IDS), 'P3 needs the frozen 18 scenarios in order');
  need(
      same(Object.keys(manifest.providers || {}).sort(), [...PROVIDERS].sort()) &&
          PROVIDERS.every(provider => same(manifest.providers[provider], P3_SCENARIO_IDS)),
      'Both hosts require the same 18 scenarios');
  const preamble = manifest.sessionPreamble;
  need(
      preamble?.version === 'p3-v1' && typeof preamble.content === 'string' &&
          same(hash(Buffer.from(preamble.content)), preamble.identity) &&
          same(preamble.parameters, ['PROJECT', 'NODE', 'COLLECTOR', 'VERSION', 'STDIO']),
      'Frozen neutral preamble differs');
  for (const row of manifest.rows) {
    need(
        row.version === 'p3-v1' && typeof row.prompt === 'string' && row.prompt.trim() &&
            same(hash(Buffer.from(row.prompt)), row.promptIdentity),
        'Frozen prompt identity differs');
    need(
        Array.isArray(row.parameters) && new Set(row.parameters).size === row.parameters.length &&
            same(
                [...new Set([...row.prompt.matchAll(/\{\{([A-Z_]+)\}\}/g)].map(match => match[1]))].sort(),
                [...row.parameters].sort()),
        'Prompt parameters differ');
    need(
        Array.isArray(row.materials) &&
            new Set(row.materials.map(value => value.name)).size === row.materials.length,
        'Frozen materials missing/duplicated');
    for (const material of row.materials) {
      need(
          /^[a-zA-Z0-9][a-zA-Z0-9._-]*$/.test(material.name) && identity(material.identity),
          'Unsafe material/identity');
      need(
          typeof material.content === 'string' ?
              !material.packageMember && same(hash(Buffer.from(material.content)), material.identity) :
              typeof material.packageMember === 'string' &&
                  material.packageMember.startsWith('skills/phoenix-diagram/examples/'),
          'Frozen material bytes/source differ');
    }
  }
  return manifest;
}
export function checkP3Coverage(record) {
  need(
      Array.isArray(record?.scenarios) &&
          same(record.scenarios.map(row => row.id).sort(), [...P3_SCENARIO_IDS].sort()),
      'P3 needs exactly 18 unique scenario rows');
  need(
      Array.isArray(record.sessions) && record.sessions.length === 18 &&
          new Set(record.sessions.map(row => row.id)).size === 18,
      'Each P3 case requires its own fresh native session');
  const projects = new Set();
  for (const session of record.sessions) {
    const rows = record.scenarios.filter(row => row.sessionId === session.id);
    need(rows.length === 1 && rows[0].id === session.scenarioId, 'A session cannot cover another scenario');
    need(
        path.isAbsolute(session.cwd || '') && physical(session.project),
        'Physical neutral project binding missing');
    const key = session.project.device + ':' + session.project.inode;
    need(!projects.has(key), 'A P3 origin project is shared across scenarios');
    projects.add(key);
  }
  return true;
}
function checkCandidate(record, context) {
  const cohort = context.cohort;
  need(
      cohort && context.tarInventory instanceof Map && typeof context.bytesFor === 'function',
      'Authenticated candidate context missing');
  const version = json(record.package?.version, context, 'Candidate VERSION'),
        tarball = bytes(record.package?.tarball, context, 'Candidate tgz');
  need(
      record.package.version.identity.sha256 === cohort.versionSha256 &&
          hash(tarball).sha256 === cohort.tgzSha256 && version.bundleSha256 === cohort.bundleSha256 &&
          version.sourceInputSha256 === cohort.source.inputSha256 &&
          version.sourceCommit === cohort.source.commit &&
          version.npmPackageVersion === cohort.npmPackageVersion && same(version.engines, cohort.engines),
      'Provider candidate/source/bundle/engines differ');
  need(
      record.candidateSha256 === cohort.tgzSha256 && same(record.source, cohort.source),
      'Provider record is not this candidate');
  return version;
}
function checkCollector(file, which, context) {
  const expected = context.collectorIdentity?.[which];
  need(
      identity(expected) && same(file?.identity, expected) &&
          same(hash(bytes(file, context, which + ' collector')), expected),
      'Collector is not the authenticated harness member');
  const member = which === 'stdio' ? STDIO_MEMBER : PROVIDER_MEMBER;
  need(file.path.endsWith('/' + member), 'Wrong authenticated collector member path');
}
function modelOf(event, provider) {
  return provider === 'claude-code'                                  ? event.model || event.message?.model :
      event.type === 'turn_context' || event.type === 'session_meta' ? event.payload?.model :
                                                                       null;
}
function checkRuntime(dto, raw, provider, context) {
  const runtime = dto.runtime;
  noSecrets(runtime);
  need(
      runtime?.policy === 'inherit-config' && typeof runtime.requestedModel === 'string' &&
          runtime.requestedModel.trim() && typeof runtime.credentialSource === 'string' &&
          runtime.credentialSource.trim(),
      'Actual requested model/runtime source missing');
  need(
      ['client-default', 'custom-endpoint'].includes(runtime.endpoint?.category) &&
          Array.isArray(runtime.sources),
      'Actual endpoint category/source missing');
  need(
      runtime.endpoint.category === 'custom-endpoint' ? safeOrigin(runtime.endpoint.origin) &&
              runtime.sources.some(row => row.origin === runtime.endpoint.origin) :
                                                        runtime.endpoint.origin === null,
      'Endpoint origin/source differs');
  for (const source of runtime.sources)
    need(
        typeof source.source === 'string' && typeof source.key === 'string' &&
            (!source.origin || safeOrigin(source.origin)),
        'Unsafe service-selection source');
  const modelAt = dto.argv.indexOf('--model');
  if (modelAt >= 0)
    need(dto.argv[modelAt + 1] === runtime.requestedModel, 'Requested model differs from actual argv');
  else
    need(
        runtime.requestedModel === 'client-default' ||
            runtime.sources.some(row => row.value === runtime.requestedModel),
        'Inherited requested model has no observed source');
  need(
      dto.transcriptStatus === 'captured' && Array.isArray(dto.observedModels) &&
          dto.observedModels.length > 0,
      'Observed native model unavailable');
  for (const row of dto.observedModels) {
    const event = context.nativeProof.anchor(raw, row.anchor);
    need(
        typeof row.value === 'string' && row.value && modelOf(event, provider) === row.value,
        'Observed model is not anchored in native transcript');
  }
}
function projectObservation(refValue, dto, context, label) {
  const value = json(refValue, context, label);
  need(
      value.schemaVersion === 1 && value.kind === 'phoenix-diagram-p3-project-observation' &&
          value.cwd === dto.cwd && sameProject(value.project, dto.project) && Array.isArray(value.entries) &&
          new Set(value.entries.map(row => row.path)).size === value.entries.length,
      'Original project observation missing');
  for (const row of value.entries)
    need(
        typeof row.path === 'string' && !row.path.startsWith('/') &&
            !row.path.split('/').some(part => !part || part === '.' || part === '..') &&
            ['file', 'symlink', 'special'].includes(row.kind) &&
            (row.kind !== 'file' || identity(row.identity)),
        'Unsafe project observation entry');
  return value;
}
function resumeArg(argv, provider, id) {
  return provider === 'claude-code' ? argv.includes('--resume') && argv[argv.indexOf('--resume') + 1] === id :
                                      argv[0] === 'exec' && argv[1] === 'resume' && argv.includes(id);
}
function proofContext(session, part, context) {
  need(
      Number.isSafeInteger(part?.captureIndex) && part.captureIndex >= 0 &&
          part.captureIndex < session.parts.length,
      'Native proof capture index missing');
  return session.parts[part.captureIndex];
}
function injectedPublicSkillBody(part, read, source, pair, context) {
  if (read.packageMember === 'skills/phoenix-diagram/SKILL.md')
    return context.nativeProof.injectedSkillBody(part, read, source, pair);
  const skill = /^skills\/(elintp|documentation-writer)\/SKILL\.md$/.exec(read.packageMember)?.[1];
  const body = /^---\n[\s\S]*?\n---\n\n([\s\S]*)$/.exec(source)?.[1];
  const args = pair.input?.args;
  const hasArgs = typeof args === 'string' && args.length > 0;
  need(
      skill && part.provider === 'claude-code' && pair.name === 'Skill' &&
          same(pair.input, hasArgs ? {skill, args} : {skill}) &&
          read.file.path === path.posix.join(part.capture.cwd, '.claude/skills', skill, 'SKILL.md') &&
          typeof body === 'string' && read.injection &&
          read.result.offset + read.result.length === read.injection.offset,
      'Actual caller Skill injection missing');
  const call = context.nativeProof.anchor(part.raw, read.call),
        result = context.nativeProof.anchor(part.raw, read.result),
        injection = context.nativeProof.anchor(part.raw, read.injection);
  need(
      call.type === 'assistant' && result.type === 'user' && result.tool_use_result?.success === true &&
          result.tool_use_result.commandName === skill && !pair.error &&
          pair.output === 'Launching skill: ' + skill &&
          [call, result, injection].every(event => event.session_id === part.session.id) &&
          injection.type === 'user' && injection.isSynthetic === true && injection.message?.role === 'user' &&
          injection.message.content?.length === 1 && injection.message.content[0].type === 'text' &&
          injection.message.content[0].text ===
              `Base directory for this skill: ${path.posix.dirname(read.file.path)}\n\n${body}` +
                  (hasArgs ? '\n\nARGUMENTS: ' + args : ''),
      'Caller Skill injection differs from complete authenticated source');
  return body;
}
export function checkP3Session(session, row, context) {
  const provider = context.provider || session.provider,
        required = context.scenarioManifest.rows.find(value => value.id === row.id);
  need(
      PROVIDERS.includes(provider) && required && session.scenarioId === row.id &&
          session.id === row.sessionId && session.provider === provider,
      'Native session/provider/scenario differs');
  need(
      context.nativeProof?.anchor && context.nativeProof.proofPair && context.nativeProof.calls,
      'Neutral native transcript parser missing');
  checkCollector(session.providerCollector, 'provider', context);
  need(Array.isArray(session.captures) && session.captures.length > 0, 'Original native captures missing');
  const parts = [];
  let previous = null;
  for (const [index, segment] of session.captures.entries()) {
    const dto = json(segment.capture, context, 'Original provider capture');
    need(
        dto.schemaVersion === 1 && dto.kind === PROVIDER_KIND && dto.provider === provider &&
            dto.scenarioId === row.id && dto.sessionId === session.id && dto.cwd === session.cwd &&
            sameProject(dto.project, session.project),
        'Native capture identity/project differs');
    checkCollector(dto.collector, 'provider', context);
    need(sameRef(dto.collector, session.providerCollector), 'Provider capture used another collector');
    need(
        dto.exitCode === 0 && dto.signal === null && !dto.timedOut && Array.isArray(dto.argv) &&
            dto.argv.every(value => typeof value === 'string'),
        'Native provider did not complete');
    need(
        index === 0 ? dto.origin === true && !dto.previousCapture &&
                !dto.argv.some(
                    value => ['resume', '--resume', '--continue', '-r', '-c', '--last', '--fork-session',
                              'fork']
                                 .includes(value)) :
                      dto.origin === false && sameRef(dto.previousCapture, previous) &&
                resumeArg(dto.argv, provider, session.id),
        'Session is not a fresh origin or exact same-case continuation');
    const modelAt = dto.argv.indexOf('--model'),
          actualModel = modelAt < 0 ? undefined : dto.argv[modelAt + 1];
    need(
        same(
            dto.argv,
            buildArguments(
                {provider, cwd: dto.cwd, endpointPolicy: 'inherit-config', model: actualModel},
                {sessionId: session.id, previousId: index ? session.id : undefined})),
        'Native client argv was not constructed by the fixed launcher');
    const version = json(dto.version, context, 'Actual CLI version');
    need(
        version.exitCode === 0 && version.signal === null && same(version.program, dto.program) &&
            same(version.argv, ['--version']) &&
            (provider === 'codex' ? /codex/i : /Claude Code/).test(version.stdout || ''),
        'Actual CLI version missing');
    bytes(dto.stdin, context, 'Original provider stdin');
    const stdout = bytes(dto.stdout, context, 'Original provider stdout');
    bytes(dto.stderr, context, 'Original provider stderr');
    need(
        sameRef(segment.transcript, dto.transcript) && sameRef(segment.version, dto.version),
        'Transcript/version selection differs');
    const raw = bytes(dto.transcript, context, 'Original native transcript'),
          events = raw.toString('utf8').split('\n').filter(Boolean).map(line => JSON.parse(line));
    need(
        same(json(dto.lineIndex, context, 'Native line index'), lineIndex(raw)),
        'Native transcript line anchors differ');
    if (provider === 'claude-code')
      need(
          dto.format === 'claude-stream-json' && raw.equals(stdout),
          'Claude transcript is not original stdout');
    else
      need(
          dto.format === 'codex-session-json' && dto.nativeSessionFile === dto.transcriptSource?.path &&
              same(dto.transcriptSource.identity, dto.transcript.identity),
          'Codex native session source missing');
    const start = events.find(
              event => provider === 'claude-code' ? event.type === 'system' && event.subtype === 'init' :
                                                    event.type === 'session_meta'),
          id = start?.session_id || start?.payload?.id;
    need(
        id === session.id && (provider === 'claude-code' || start.payload.cwd === session.cwd) &&
            events.some(
                event => provider === 'claude-code' ?
                    event.type === 'result' && event.session_id === session.id && event.is_error === false :
                    event.type === 'event_msg' && event.payload?.type === 'task_complete'),
        'Native session start/completion missing');
    checkRuntime(dto, raw, provider, context);
    if (index > 0 && provider === 'codex') {
      const old = parts.at(-1).raw;
      need(
          raw.subarray(0, old.length).equals(old) &&
              dto.observedModels.every(value => value.anchor.offset >= old.length),
          'Continuation model/transcript evidence reuses an earlier turn');
    }
    const before = projectObservation(dto.projectBefore, dto, context, 'Project before'),
          after = projectObservation(dto.projectAfter, dto, context, 'Project after');
    parts.push({
      session,
      provider,
      raw,
      events,
      capture: dto,
      version,
      before,
      after,
      collectorIdentity: context.collectorIdentity.stdio
    });
    previous = segment.capture;
  }
  const materialPaths = new Map(required.materials.map(value => [value.name, value.identity]));
  for (const [name, expected] of materialPaths)
    need(parts[0].before.entries.some(entry => entry.path === name && entry.kind === 'file' &&
            entry.mode === 0o644 && same(entry.identity, expected)),
        'Initial project material differs from its frozen original');
  const callers = ['caller-elintp', 'fallback-elintp'].includes(row.id)   ? 'elintp' :
      ['caller-documentation', 'fallback-documentation'].includes(row.id) ? 'documentation-writer' :
                                                                            null;
  for (const entry of parts[0].before.entries) {
    const skill = /^\.(?:agents|claude|codex)\/skills\/([^/]+)\//.exec(entry.path)?.[1];
    need(
        materialPaths.has(entry.path) || /^\.phoenix-diagram\/(?:install|brands)\//.test(entry.path) ||
            /^\.claude\/settings(?:\.local)?\.json$/.test(entry.path) ||
            entry.path === '.codex/config.toml' || skill === 'phoenix-diagram' || skill === callers ||
            MIGRATION.has(row.id) && skill === 'archify' ||
            row.id === 'migrate-conflict' && entry.path === 'skills-lock.json' &&
                entry.kind === 'file' && entry.mode === 0o644 && same(entry.identity, MANAGER_MARKER),
        'P3 origin is not a neutral project with only authorized setup/materials');
  }
  const captured =
      {...session, parts, parameters: row.parameters || {}, preambleParameters: row.preambleParameters || {}};
  for (const part of parts)
    for (const event of part.events.filter(context.nativeProof.toolEvent)) {
      const text = context.nativeProof.toolCallText(event);
      need(
          !/\/(?:renderers|tools\/phoenix-diagram\/src)\/|p3-scenarios\.json|(?:\/tests\/provider\/.*facts|\.xgent-ai\/|\/memories\/)/
               .test(text),
          'Provider read implementation, hidden answers or personal memory');
    }
  const reads = session.loadedReads;
  need(
      Array.isArray(reads) && (reads.length || MIGRATION.has(row.id)),
      'Actual installed skill reads missing');
  for (const read of reads) {
    const part = proofContext(captured, read, context), pair = context.nativeProof.proofPair(part, read),
          source =
              expectedFile(read.file, read.packageMember, context, 'Loaded skill resource').toString('utf8');
    const projectRead = read.file.path.startsWith(session.cwd + '/') &&
        read.file.path.endsWith('/' + read.packageMember.slice(7));
    const releaseRead = MIGRATION.has(row.id) && row.parameters?.RELEASE_DIR &&
        read.packageMember.startsWith('skills/phoenix-diagram/') &&
        read.file.path.startsWith(row.parameters.RELEASE_DIR + '/') &&
        read.file.path.endsWith('/' + read.packageMember.slice('skills/phoenix-diagram/'.length));
    need(
        read.packageMember.startsWith('skills/') && (projectRead || releaseRead),
        'Loaded skill path is outside this project or authorized migration release');
    const output = read.injection || pair.name === 'Skill' ?
        injectedPublicSkillBody(part, read, source, pair, context) :
        pair.output;
    need(
        !pair.error && (pair.exitCode === undefined || pair.exitCode === 0) &&
            typeof read.quote === 'string' && read.quote.length >= 40 && source.includes(read.quote) &&
            output.includes(read.quote),
        'Actual skill read/tool result differs');
  }
  const caller = row.id.endsWith('elintp') ? 'elintp' :
      row.id.endsWith('documentation')     ? 'documentation-writer' :
                                             null;
  if (caller)
    need(
        reads.some(read => read.packageMember === `skills/${caller}/SKILL.md`),
        'Actual caller skill was not loaded');
  if (FALLBACK.has(row.id)) {
    need(
        !session.installedReceipt && !session.pdCall &&
            (!session.commands || session.commands.length === 0) &&
            !reads.some(read => read.packageMember.startsWith('skills/phoenix-diagram/')),
        'Fallback cannot fabricate an installed PD or native calls');
    for (const part of parts)
      need(
          [...part.before.entries, ...part.after.entries].every(
              value => !/(?:^|\/)skills\/phoenix-diagram\/|^\.phoenix-diagram\/install\/active\.json$/.test(
                  value.path)),
          'Fallback project actually contains PD');
    need(
        Array.isArray(row.evidence?.discovery) && row.evidence.discovery.length > 0,
        'Actual missing-tool discovery is absent');
    for (const proof of row.evidence.discovery) {
      const pair = context.nativeProof.proofPair(proofContext(captured, proof, context), proof);
      need(
          typeof pair.output === 'string' && pair.output.trim() &&
              /phoenix-diagram/.test(context.nativeProof.text(pair.input)),
          'Missing-tool discovery has no original tool call/result');
    }
  } else if (!MIGRATION.has(row.id)) {
    const installed = json(session.installedReceipt, context, 'Original installed receipt');
    need(
        installed.schemaVersion === 1 && installed.tool === 'phoenix-diagram' &&
            installed.project?.device === session.project.device &&
            installed.project?.inode === session.project.inode &&
            installed.bundleManifestSha256 === context.cohort.bundleSha256 &&
            installed.engine?.sourceInputSha256 === context.cohort.source.inputSha256 &&
            Object.values(context.cohort.engines)
                .some(engine => same(installed.engine.binary, {sha256: engine.sha256, bytes: engine.size})) &&
            installed.engineManifestSha256 === context.cohort.versionSha256 &&
            installed.npmPackageVersion === context.cohort.npmPackageVersion,
        'Installed PD does not belong to this candidate/project');
    need(
        reads.some(read => read.packageMember === 'skills/phoenix-diagram/SKILL.md'),
        'Installed PD skill not loaded');
    for (const read of reads.filter(value => value.packageMember.startsWith('skills/phoenix-diagram/'))) {
      const relative = path.posix.relative(session.cwd, read.file.path);
      need(
          installed.ownership?.some(
              owned => owned.path === relative &&
                  same({sha256: owned.identity.sha256, bytes: owned.identity.bytes}, read.file.identity)),
          'PD loaded resource is not owned');
    }
  }
  return captured;
}
function checkObservation(value, context, label) {
  need(
      value && ['present', 'absent'].includes(value.state) && path.isAbsolute(value.path || '') &&
          value.parent && physical(value.parent.binding),
      label + ' state missing');
  if (value.state === 'present') {
    need(
        identity(value.identity) && physical(value.binding) &&
            same(hash(bytes(value.snapshot, context, label + ' original snapshot')), value.identity),
        label + ' before/after bytes differ');
  } else
    need(
        !value.snapshot && !value.identity && !value.binding,
        label + ' absent state contains invented bytes');
}
function checkInstallation(state, cwd, context, label) {
  need(
      state?.active?.path === path.join(cwd, '.phoenix-diagram/install/active.json') &&
          Array.isArray(state.launchers) && state.launchers.length === 3,
      label + ' installation paths differ');
  checkObservation(state.active, context, label + ' active');
  for (const [at, row] of state.launchers.entries()) {
    need(
        row.path ===
            path.join(
                cwd, ['.agents', '.claude', '.codex'][at], 'skills/phoenix-diagram/scripts/phoenix-diagram'),
        label + ' launcher path differs');
    checkObservation(row, context, label + ' launcher');
  }
}
function withoutSnapshots(state) {
  const copy = structuredClone(state);
  for (const row of [copy.active, ...copy.launchers]) delete row.snapshot;
  return copy;
}
function commandHeader(receipt, command) {
  need(
      receipt?.schemaVersion === 1 && receipt.tool === 'phoenix-diagram' && receipt.toolVersion === '0.2.0' &&
          receipt.command === command && Array.isArray(receipt.diagnostics),
      'Original command receipt header differs');
}
export function checkP3Command(captured, command, context) {
  const part = proofContext(captured, command, context),
        pair = context.nativeProof.proofPair(part, command.proof),
        dto = json(command.capture, context, 'Original PD capture');
  need(
      dto.schemaVersion === 1 && dto.kind === STDIO_KIND && dto.tool === 'phoenix-diagram' &&
          dto.cwd === captured.cwd && sameProject(dto.project, captured.project),
      'PD capture belongs to another session/project');
  checkCollector(dto.collector, 'stdio', context);
  const words = context.nativeProof.executedWords(context.nativeProof.commandOf(pair));
  need(
      words[0] === dto.node?.path && words[1] === dto.collector.path,
      'PD command did not execute the authenticated collector');
  const options = parseCaptureArgs(words.slice(2));
  if (captured.preambleParameters?.NODE)
    need(
        words[0] === captured.preambleParameters.NODE && words[1] === captured.preambleParameters.COLLECTOR &&
            options.candidateVersion === captured.preambleParameters.VERSION &&
            [options.stdout, options.stderr, options.evidenceDir].every(
                value => value.startsWith(captured.preambleParameters.STDIO + '/')),
        'PD invocation differs from the neutral collection protocol');
  need(
      options.program === dto.program?.path && options.cwd === dto.cwd && same(options.argv, dto.argv) &&
          options.mode === dto.mode && options.stdout === dto.stdout?.path &&
          options.stderr === dto.stderr?.path &&
          sameRef(
              command.capture,
              {path: path.join(options.evidenceDir, 'capture.json'), identity: command.capture.identity}),
      'PD argv/capture slots differ');
  const lines = pair.output.split('\n')
                    .filter(Boolean)
                    .flatMap(line => {
                      try {
                        return [JSON.parse(line)];
                      } catch {
                        return [];
                      }
                    })
                    .filter(value => value.kind === STDIO_KIND);
  need(
      lines.length === 1 && same(lines[0], {...dto, captureFile: command.capture}),
      'Original tool output does not contain this sealed PD capture');
  const interrupted = command.expectedInterruption === true;
  need(
      interrupted ?
          captured.scenarioId === 'migrate-recover' && dto.argv[0] === 'migrate' &&
              dto.argv.includes('--confirm-remove-archify') && dto.exitCode === null &&
              ['SIGTERM', 'SIGKILL'].includes(dto.signal) &&
              (pair.exitCode === undefined ? captured.provider === 'claude-code' : pair.exitCode === 4) :
          Number.isInteger(dto.exitCode) && dto.exitCode >= 0 && dto.exitCode <= 255 && dto.signal === null &&
              (pair.exitCode === undefined ? captured.provider === 'claude-code' :
                                             pair.exitCode === dto.exitCode) &&
              (pair.errorFlag === undefined || pair.errorFlag === (dto.exitCode !== 0)),
      'Native exit or signal differs');
  const version = json(dto.candidateVersion, context, 'Command candidate VERSION');
  need(
      dto.candidateVersion.identity.sha256 === context.cohort.versionSha256 &&
          version.sourceInputSha256 === context.cohort.source.inputSha256 &&
          version.bundleSha256 === context.cohort.bundleSha256,
      'Command candidate VERSION differs');
  bytes(dto.stderr, context, 'Original command stderr');
  const stdout = bytes(dto.stdout, context, 'Original command stdout');
  checkInstallation(dto.installationBefore, dto.cwd, context, 'Before');
  checkInstallation(dto.installationAfter, dto.cwd, context, 'After');
  const name = dto.argv[0];
  const helpRequested = dto.argv.includes('--help') || dto.argv.includes('-h');
  need(
      [
        'engine', 'doctor', 'guide', 'brands', 'validate', 'deliver', 'check', 'browser-check',
        'visual-check', 'finalize', 'review-record', 'recover-output', 'preview', 'compare',
        'recover-compare', 'migrate', '--help', '-h'
      ].includes(name),
      'Unsupported P3 native command');
  if (helpRequested && dto.mode === 'migration') {
    need(
        MIGRATION.has(captured.scenarioId) &&
            dto.exitCode === 0 && dto.signal === null && stdout.length > 0 &&
            (!captured.parameters?.ENGINE || captured.parameters.ENGINE === dto.program.path) &&
            Object.values(context.cohort.engines)
                .some(engine => same(dto.program.identity, {sha256: engine.sha256, bytes: engine.size})) &&
            same(withoutSnapshots(dto.installationBefore), withoutSnapshots(dto.installationAfter)),
        'Migration help requires its own scenario, fixed candidate engine, exit zero and unchanged state');
    return {dto, argv: dto.argv, help: true, stdout: dto.stdout, exitCode: 0};
  }
  if (name === 'migrate' && !helpRequested) {
    const at = dto.argv.indexOf('--project');
    need(
        at >= 0 && path.posix.resolve(dto.cwd, dto.argv[at + 1] || '') === captured.cwd,
        'Migration argv names another project');
    if (captured.parameters?.PROJECT)
      need(
          captured.parameters.PROJECT === captured.cwd && captured.parameters.ENGINE === dto.program.path,
          'Migration differs from the authored scope/engine');
    if (dto.argv.includes('--from-archify') && captured.parameters?.RELEASE_DIR) {
      const bundleAt = dto.argv.indexOf('--bundle-root');
      const bundleRoot = path.posix.resolve(dto.cwd, dto.argv[bundleAt + 1] || '');
      need(
          bundleAt >= 0 &&
              (bundleRoot === captured.parameters.RELEASE_DIR ||
               bundleRoot.startsWith(captured.parameters.RELEASE_DIR + '/')),
          'Migration uses another release directory');
    }
    need(
        MIGRATION.has(captured.scenarioId) && dto.mode === 'migration' &&
            Object.values(context.cohort.engines)
                .some(engine => same(dto.program.identity, {sha256: engine.sha256, bytes: engine.size})),
        'Migration did not use the fixed candidate engine');
  } else {
    need(
        dto.mode === 'installed' &&
            same(withoutSnapshots(dto.installationBefore), withoutSnapshots(dto.installationAfter)),
        'Ordinary command changed its installation');
    const active = dto.installationBefore.active;
    need(active.state === 'present', 'Original command installation absent');
    const installed = json(active.snapshot, context, 'Command original active'),
          relative = path.posix.relative(dto.cwd, dto.program.path),
          owner = installed.ownership?.find(value => value.path === relative);
    need(
        /^\.(?:agents|claude)\/skills\/phoenix-diagram\/scripts\/phoenix-diagram$/.test(relative) && owner &&
            Object.entries({...dto.program.binding, ...dto.program.identity})
                .every(([key, value]) => owner.identity[key] === value) &&
            installed.bundleManifestSha256 === context.cohort.bundleSha256 &&
            installed.engineManifestSha256 === context.cohort.versionSha256 &&
            installed.npmPackageVersion === context.cohort.npmPackageVersion,
        'Executed launcher was not owned by this candidate');
    const expected = context.tarInventory.get('skills/phoenix-diagram/scripts/phoenix-diagram');
    need(
        expected &&
            same(dto.program.identity, {sha256: expected.sha256, bytes: expected.size ?? expected.bytes}) &&
            installed.engine?.sourceInputSha256 === context.cohort.source.inputSha256 &&
            Object.values(context.cohort.engines)
                .some(engine => same(installed.engine.binary, {sha256: engine.sha256, bytes: engine.size})),
        'Launcher or installed engine differs from authenticated candidate');
  }
  if (helpRequested) {
    need(dto.exitCode === 0 && dto.signal === null && stdout.length > 0, 'Installed help must finish with its actual text and exit zero');
    return {dto, argv: dto.argv, help: true, stdout: dto.stdout, exitCode: 0};
  }
  if (interrupted) return {dto, argv: dto.argv, interrupted: true, exitCode: null};
  if (name === 'preview') {
    const events = stdout.toString('utf8').split('\n').filter(Boolean).map(line => JSON.parse(line));
    need(
        events.length >= 2 && events[0].event === 'started' && events.at(-1).event === 'stopped',
        'Preview is not a complete NDJSON session');
    let generation = -1, successful = 0;
    const id = events[0].sessionId;
    for (const event of events) {
      commandHeader(event, 'preview');
      need(
          event.sessionId === id && typeof id === 'string' && id && Number.isSafeInteger(event.generation) &&
              event.generation >= generation && Number.isSafeInteger(event.lastSuccessfulGeneration) &&
              event.lastSuccessfulGeneration >= successful &&
              event.lastSuccessfulGeneration <= event.generation &&
              ['started', 'checking', 'updated', 'failed', 'stopped'].includes(event.event),
          'Preview session/generation regressed');
      generation = event.generation;
      successful = event.lastSuccessfulGeneration;
    }
    need(
        events.at(-1).shutdownComplete === true && events.at(-1).exitCode === dto.exitCode,
        'Preview stop is not confirmed');
    return {dto, argv: dto.argv, events, exitCode: dto.exitCode};
  }
  const receipt = JSON.parse(stdout.toString('utf8')),
        expected = name === 'brands' && dto.argv[1] === 'capture' ? 'brands capture' : name;
  commandHeader(receipt, expected);
  if (name === 'migrate' &&
      (dto.argv.includes('--dry-run') ||
       dto.argv.includes('--from-archify') && !dto.argv.includes('--confirm-remove-archify')))
    need(
        same(withoutSnapshots(dto.installationBefore), withoutSnapshots(dto.installationAfter)),
        'Migration changed installation state before confirmation or during dry-run');
  need(
      dto.exitCode === 0 ? !['failed', 'unavailable'].includes(receipt.status) : receipt.status !== 'pass',
      'Native command status contradicts its process exit');
  return {dto, argv: dto.argv, receipt, exitCode: dto.exitCode};
}
export function checkP3RecordedCommands(captured, entries, context) {
  const claims = new Set(entries.map(value => `${value.captureIndex}/${value.proof?.toolUseId}`));
  need(claims.size === entries.length, 'A native producer is claimed more than once');
  for (const [index, part] of captured.parts.entries()) {
    for (const event of part.events)
      for (const call of context.nativeProof.calls(event)) {
        const command = context.nativeProof.commandOf(call);
        if (!command) continue;
        let words;
        try {
          words = context.nativeProof.executedWords(command);
        } catch {
          need(
              !/(?:\$PD|phoenix-diagram[^\s]*['"]?\s+(?:brands|preview|compare|migrate|finalize|deliver|validate))/
                   .test(command),
              'Native PD command bypassed the fixed collector');
          continue;
        }
        const collected = words[1]?.endsWith('/' + STDIO_MEMBER);
        if (collected && words.length === 3 && words[2] === '--help') continue;
        const direct =
            /(?:^|\/)(?:phoenix-diagram(?:-(?:darwin|linux)-(?:arm64|x64))?|phoenix-diagram\.js)$/.test(
                words[0] || '') ||
            /(?:^|\/)phoenix-diagram\.js$/.test(words[1] || '');
        need(
            !direct && (!collected || claims.has(`${index}/${call.id}`)),
            'Native PD command is unsealed or absent from the recorded command set');
      }
  }
  return true;
}
function renderPrompt(required, parameters) {
  need(
      object(parameters) && same(Object.keys(parameters).sort(), [...required.parameters].sort()),
      'Actual prompt parameters differ');
  return required.prompt.replace(/\{\{([A-Z_]+)\}\}/g, (_, key) => {
    need(
        typeof parameters[key] === 'string' && parameters[key] && !/[\0\r\n]/.test(parameters[key]),
        'Unsafe prompt parameter');
    return parameters[key];
  });
}
export function renderP3OriginPrompt(manifest, row) {
  checkP3ScenarioManifest(manifest);
  const required = manifest.rows.find(value => value.id === row.id);
  need(required, 'Unknown P3 authored topic');
  return renderPrompt(
             {prompt: manifest.sessionPreamble.content, parameters: manifest.sessionPreamble.parameters},
             row.preambleParameters || {}) +
      renderPrompt(required, row.parameters || {});
}
function png(file, context) {
  const value = bytes(file, context, 'Observed original image');
  need(
      value.length > 8 && value.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])),
      'Original screenshot is not PNG');
}
function evidenceReferences(value, context) {
  if (Array.isArray(value)) {
    for (const child of value) evidenceReferences(child, context);
  } else if (object(value)) {
    if (Object.hasOwn(value, 'path') && Object.hasOwn(value, 'identity'))
      bytes(value, context, 'Original outcome evidence');
    else
      for (const child of Object.values(value)) evidenceReferences(child, context);
  }
}
function reviewScenario(row, captured, commands, materialRefs, context) {
  const review = json(row.review, context, 'Independent scenario review');
  need(
      review.schemaVersion === 1 && review.kind === 'phoenix-diagram-p3-scenario-review' &&
          review.provider === captured.provider && review.scenarioId === row.id &&
          review.sessionId === captured.id && review.verdict === 'passed' &&
          typeof review.reviewer === 'string' && review.reviewer.trim() &&
          typeof review.observations === 'string' && review.observations.trim(),
      'Independent per-scenario semantic review missing or non-passing');
  need(
      same(review.prompt, row.prompt) && same(review.materials, materialRefs) &&
          same(review.commandCaptures, commands.map(value => value.capture)) &&
          same(review.nativeCaptures, captured.captures.map(value => value.capture)),
      'Semantic review belongs to different original evidence');
  need(
      Array.isArray(review.scope) &&
          ['authorization', 'facts', 'outcome'].every(scope => review.scope.includes(scope)),
      'Semantic review scope incomplete');
  need(Array.isArray(review.images), 'Image observation list missing');
  if (VISUAL.has(row.id)) need(review.images.length > 0, 'Actual image review is still required');
  for (const image of review.images) {
    png(image.file, context);
    need(
        image.observed === true && typeof image.observations === 'string' && image.observations.trim(),
        'Actual original image observation missing');
  }
  return review;
}
function checkCompletedDelivery(row, receipts, context, comparison) {
  const summary = json(row.evidence.summary, context, 'Current delivery summary'),
        review = receipts.find(
            value => value.command === 'review-record' && value.status === 'pass' &&
                value.visualReview?.status === 'passed' && value.deliveryStatus === 'complete' &&
                sameRef(value.evidence?.summaryReceipt, row.evidence.summary));
  need(
      review && summary.visualReview?.status === 'passed' && summary.deliveryStatus === 'complete' &&
          summary.automatedStatus === 'pass',
      'Mandatory visual review/current complete summary missing');
  const expected = comparison ? ['check', 'visual-check'] : ['validate', 'deliver', 'check', 'browser-check'];
  need(
      same(summary.stages?.map(value => value.name), expected) &&
          summary.stages.every(value => value.status === 'pass'),
      'Current summary gates incomplete');
  if (comparison) {
    need(
        summary.kind === 'architecture-comparison-summary' && summary.command === 'visual-check',
        'Wrong comparison summary');
    const original = receipts.find(
        value => value.command === 'compare' && value.kind === 'architecture-comparison' &&
            value.status === 'pass');
    need(
        original?.receiptId === summary.comparison?.receiptId &&
            same(original.artifact, summary.comparison.artifact),
        'Comparison summary changed original identity');
  } else {
    const original = receipts.find(value => value.command === 'finalize' && value.automatedStatus === 'pass');
    need(
        original && original.deliveryReceiptId === summary.deliveryReceiptId &&
            same(original.stages, summary.stages) &&
            same(original.browserReceiptIdentity, summary.browserReceiptIdentity),
        'Reviewed summary differs from original finalize');
  }
  const browser = json(summary.evidence?.browserReceipt, context, 'Original browser receipt');
  need(
      browser.status === 'pass' && Array.isArray(browser.screenshots) &&
          browser.screenshots.length === (comparison ? 12 : 4),
      'Original browser screenshots missing');
  const observed = json(row.review, context, 'Independent image review').images;
  for (const shot of browser.screenshots) {
    png(shot, context);
    need(
        observed.some(
            value => sameRef(value.file, shot) && value.observed === true && value.observations.trim()),
        'A required native screenshot was not independently observed');
  }
  const artifact = comparison ? summary.comparison.artifact : summary.artifact;
  bytes(artifact, context, 'Delivered original HTML');
}
export function checkP3ScenarioOutcome(row, captured, results, response, context) {
  // Public help is retained among all original captures/review bindings. It is
  // not an operation, a user confirmation, or evidence of a completed capability.
  results = results.filter(value => !value.help);
  const evidence = row.evidence;
  evidenceReferences(evidence, context);
  need(evidence?.kind === row.id, 'P3 outcome scope differs');
  const receipts = results.flatMap(value => value.receipt ? [value.receipt] : []),
        args = results.map(value => value.argv), has = name => args.some(value => value[0] === name),
        failed = receipts.some(value => value.status === 'failed' || value.status === 'unavailable');
  need(
      !args.some(value => value[0] === 'engine' && ['install', 'update'].includes(value[1])),
      'This scenario did not authorize engine installation/update');
  need(
      row.id === 'brand-capture' || !args.some(value => value[0] === 'brands' && value[1] === 'capture'),
      'This scenario did not explicitly authorize brand capture');
  need(row.id === 'preview-explicit' || !has('preview'), 'This scenario did not explicitly request preview');
  need(
      ['compare-valid', 'compare-invalid'].includes(row.id) || !has('compare'),
      'This scenario did not explicitly request comparison');
  if (row.id === 'brand-capture') {
    const authorized =
        context.scenarioManifest.rows.find(value => value.id === row.id)
            .prompt
            .match(
                /https:\/\/raw\.githubusercontent\.com\/XGENT-ai\/skills\/[a-f0-9]{40}\/tools\/phoenix-diagram\/tests\/fixtures\/brands\/official-test\.png/)
            ?.[0];
    const capture = results.find(value => value.argv[0] === 'brands' && value.argv[1] === 'capture');
    need(
        authorized && capture?.argv[2] === authorized &&
            capture.receipt?.brand?.sha256 ===
                '8a1035d9172bf23b4444d8409418141e7bb39271deb17786c3860ef1defd494e' &&
            capture.receipt.resource?.bytes === 125 && capture.receipt.resource?.mediaType === 'image/png',
        'Capture differs from the explicitly authorized fixed resource');
  }
  if (FALLBACK.has(row.id)) {
    need(results.length === 0, 'Fallback ran native PD');
    const output = bytes(evidence.document, context, 'Fallback document').toString('utf8');
    need(
        row.id === 'fallback-elintp' ? /<(?:svg|html)\b/i.test(output) : /```mermaid\b/.test(output),
        'Caller fallback document missing');
    need(
        !/phoenix-diagram[^\n]*(?:校验通过|native passed)/i.test(response),
        'Fallback falsely claims native success');
    return;
  }
  if (row.id === 'erd-missing-facts') {
    need(
        results.length === 0 && Array.isArray(evidence.questions) && evidence.questions.length > 0 &&
            evidence.questions.every(
                value => typeof value === 'string' && value.trim() && response.includes(value)) &&
            typeof evidence.stoppedQuote === 'string' && evidence.stoppedQuote.trim() &&
            response.includes(evidence.stoppedQuote),
        'ERD necessary questions/stopping response missing');
    for (const part of captured.parts) {
      need(same(part.before.entries, part.after.entries), 'Clarification wrote project files');
      for (const event of part.events)
        for (const call of context.nativeProof.calls(event)) {
          need(
              !/^(?:Write|Edit|write_file|apply_patch|functions\.apply_patch)$/i.test(call.name),
              'Clarification wrote a candidate');
          need(
              /^(?:Read|Glob|Grep|Skill|Bash|bash|exec_command|functions\.exec_command|view_image|functions\.view_image)$/
                  .test(call.name),
              'Clarification used an executable tool without a verifiable read-only operation');
          const callText = context.nativeProof.text(call.input);
          need(
              !/apply_patch|writeFile|write_file|(?:^|\W)(?:touch|mkdir|cp|mv|rm|tee)(?:\s|$)/.test(callText),
              'Clarification includes a candidate-writing operation');
          if (/^(?:Bash|bash|exec_command|functions\.exec_command)$/.test(call.name))
            need(
                /^\s*(?:pwd|ls|rg|cat|head|test)\b/.test(context.nativeProof.commandOf(call) || '') &&
                    !/[;|&<>`$\n]/.test(context.nativeProof.commandOf(call) || '') &&
                    !/--pre(?:=|\s)/.test(callText),
                'Clarification shell command is not observably read-only');
        }
    }
    return;
  }
  if (row.id === 'preview-explicit') {
    const previews = results.filter(value => value.events);
    need(previews.length === 1, 'Explicit preview original NDJSON missing');
    const events = previews[0].events, first = events.findIndex(value => value.event === 'updated'),
          bad = events.findIndex((value, at) => at > first && value.event === 'failed'),
          latest = events.findLastIndex((value, at) => at < bad && value.event === 'updated'),
          fixed = events.findIndex((value, at) => at > bad && value.event === 'updated');
    need(
        first >= 0 && latest > first && bad > latest && fixed > bad &&
            events[latest].lastSuccessfulGeneration > events[first].lastSuccessfulGeneration &&
            events.slice(bad, fixed).filter(value => value.event === 'failed').every(value =>
                value.lastSuccessfulGeneration === events[latest].lastSuccessfulGeneration) &&
            events[fixed].lastSuccessfulGeneration > events[latest].lastSuccessfulGeneration,
        'Preview did not demonstrate last-good failure/recovery');
    need(!previews[0].argv.includes('--open'), 'Preview opened a browser contrary to the request');
    need(reference(evidence.lifecycle), 'Actual preview HTTP/artifact/port observation missing');
    const observed = json(evidence.lifecycle, context, 'Preview lifecycle observations');
    need(
        observed.kind === 'phoenix-diagram-p3-preview-observation' &&
            observed.sessionId === events[0].sessionId && observed.portClosed === true &&
            reference(observed.servedArtifact) && reference(observed.publishedArtifact) &&
            bytes(observed.servedArtifact, context, 'HTTP original artifact')
                .equals(bytes(observed.publishedArtifact, context, 'Published preview artifact')),
        'Preview artifact bytes or closed port observations missing');
    return;
  }
  if (MIGRATION.has(row.id)) {
    need(
        results.length > 0 && results.every(value => value.argv[0] === 'migrate') &&
            receipts.every(
                value => value.legacyBackupPolicy === 'none' && value.legacyRestoration === 'not-supported'),
        'Migration backups/restoration misrepresented');
    for (const result of results.filter(
             value => value.argv.includes('--from-archify') && !value.argv.includes('--dry-run'))) {
      need(
          result.argv.includes('--confirm-remove-archify') || result.exitCode !== 0,
          'Migration applied without explicit confirmation');
      if (result.argv.includes('--confirm-remove-archify')) {
        if (row.id === 'migrate-known')
          need(
              evidence.confirmationCapture > 0,
              'Known migration did not wait for the user confirmation turn');
        need(
            Number.isSafeInteger(evidence.confirmationCapture) && evidence.confirmationCapture >= 0 &&
                evidence.confirmationCapture < captured.parts.length,
            'Original user confirmation capture missing');
        const stdin = bytes(
                          captured.parts[evidence.confirmationCapture].capture.stdin, context,
                          'Original user confirmation')
                          .toString('utf8');
        need(
            typeof evidence.confirmationQuote === 'string' && evidence.confirmationQuote.trim() &&
                stdin.includes(evidence.confirmationQuote),
            'User confirmation is absent from native stdin');
        need(
            evidence.confirmationCapture <= result.captureIndex &&
                captured.parts[evidence.confirmationCapture].capture.startedAt <= result.dto.startedAt,
            'Migration preceded user confirmation');
      }
    }
    if (row.id === 'migrate-known')
      need(
          has('migrate') && receipts.some(value => value.status === 'installed') &&
              args.some(value => value.includes('--dry-run')),
          'Known migration did not show and commit its scope');
    if (row.id === 'migrate-conflict')
      need(
          results.every(value => value.exitCode !== 0 || value.argv.includes('--dry-run')) &&
              results.every(
                  value => same(
                      withoutSnapshots(value.dto.installationBefore),
                      withoutSnapshots(value.dto.installationAfter))),
          'Conflicting migration changed installation state');
    if (row.id === 'migrate-recover')
      need(
          results.some(value => value.interrupted) && args.some(value => value.includes('--recover')) &&
              args.some(value => value.includes('--rollback')) && reference(evidence.interruption),
          'Actual interruption/recover/rollback chain missing');
    need(reference(evidence.stateObservations), 'Migration before/after protected/legacy states missing');
    const states = json(evidence.stateObservations, context, 'Migration state observations');
    need(
        states.kind === 'phoenix-diagram-p3-migration-observation' && states.project === captured.cwd &&
            states.legacyBackupCreated === false && states.legacyRestored === false &&
            Array.isArray(states.removed) && Array.isArray(states.remaining) &&
            Array.isArray(states.protected) && states.protected.length > 0 &&
            states.protected.every(value => same(value.before, value.after)),
        'Migration exact remaining/protected state observations missing');
    if (row.id === 'migrate-recover') {
      const recovered =
          results.find(value => value.argv.includes('--recover') && value.receipt?.status === 'recovered');
      const rolled = results.find(
          value => value.argv.includes('--rollback') &&
              ['rolled-back', 'conflict'].includes(value.receipt?.status));
      need(
          recovered && rolled && recovered.receipt.transactionId === states.transactionId &&
              rolled.argv[rolled.argv.indexOf('--rollback') + 1] === states.transactionId,
          'Recovery/rollback is not the same actual interrupted transaction');
    }
    return;
  }
  if (row.id === 'brand-missing' || row.id === 'compare-invalid') {
    need(failed && results.some(value => value.exitCode !== 0), 'Expected failure was not observed');
    if (row.id === 'brand-missing') {
      const original = row.materials.find(value => value.name === 'input.json');
      need(
          original &&
              receipts.some(
                  value => value.candidate && same(value.candidate.identity, original.file.identity)),
          'Resource failure did not use the original fixed candidate');
      need(
          JSON.stringify(receipts).includes('brand/cache-missing') ||
              JSON.stringify(receipts).includes('brand/digest-mismatch'),
          'Failure was unrelated to the pinned resource');
    }
    if (row.id === 'brand-missing')
      need(
          !args.some(value => value[0] === 'brands' && value[1] === 'capture'),
          'Missing fixed resource was silently recaptured');
    else {
      need(
          results.some(
              value => value.argv[0] === 'compare' && value.exitCode !== 0 &&
                  value.receipt?.kind === 'architecture-comparison'),
          'Failure was not an actual comparison');
      const pair = json(evidence.pairObservations, context, 'Old pair/recovery observations');
      need(
          pair.kind === 'phoenix-diagram-p3-comparison-failure-observation' &&
              ['artifact', 'receipt'].every(
                  side => bytes(pair.before?.[side], context, 'Old pair ' + side)
                              .equals(bytes(pair.after?.[side], context, 'Preserved pair ' + side))),
          'Invalid comparison did not preserve the old output pair');
    }
    return;
  }
  need(results.length > 0, 'Actual native delivery commands missing');
  if (row.id === 'compare-valid') {
    need(
        receipts.some(
            value => value.command === 'compare' && value.kind === 'architecture-comparison' &&
                value.status === 'pass'),
        'Actual comparison receipt missing');
    const compared = receipts.find(
        value => value.command === 'compare' && value.kind === 'architecture-comparison' &&
            value.status === 'pass');
    for (const side of ['base', 'head']) {
      const material = row.materials.find(value => value.name === side + '.json');
      need(
          material && same(compared[side]?.candidate?.identity, material.file.identity),
          'Comparison used a different authored input');
    }
    checkCompletedDelivery(row, receipts, context, true);
  } else
    checkCompletedDelivery(row, receipts, context, false);
  if (row.id === 'brand-no-url') {
    const summary = json(evidence.summary, context, 'Brand omission summary');
    const candidate = json(summary.candidate, context, 'Actual optional-brand candidate');
    need(!JSON.stringify(candidate).includes('"brand":'), 'Optional unknown brand was not omitted');
  }
  if (row.id === 'brand-capture')
    need(
        receipts.some(
            value => value.command === 'brands capture' && value.status === 'pass' && value.brand?.sha256) &&
            reference(evidence.exports) && reference(evidence.offline),
        'Actual capture/export/offline evidence missing');
  if (['normal-no-preview', 'brand-no-url', 'brand-builtin'].includes(row.id))
    need(
        !has('preview') && !args.some(value => value[0] === 'brands' && value[1] === 'capture'),
        'Unrequested preview/network capture occurred');
  if (row.id.startsWith('caller-')) {
    const document = bytes(evidence.document, context, 'Final caller document').toString('utf8');
    need(reference(evidence.documentCheck), 'Final assembled document has no separate check');
    json(evidence.documentCheck, context, 'Final document observations');
    need(
        row.id === 'caller-elintp' ?
            /<iframe\b[^>]*\bsrcdoc=/i.test(document) :
            /!\[[^\]]*\]\([^)]*\)/.test(document) && /\[[^\]]+\]\([^)]*\.html/.test(document),
        'Final caller embed/screenshot/interactive link missing');
  }
  if (row.id === 'legacy-dispositions') {
    need(
        reference(evidence.originalInput) && reference(evidence.finalInput) &&
            bytes(evidence.originalInput, context, 'Original legacy JSON')
                .equals(bytes(evidence.finalInput, context, 'Final legacy JSON')),
        'Legacy input was rewritten');
  }
}
export function checkP3ProviderRecord(record, context) {
  const provider = context.provider;
  need(
      PROVIDERS.includes(provider) && record?.schemaVersion === 1 &&
          record.kind === 'phoenix-diagram-p3-provider-acceptance' && record.profile === 'p3' &&
          record.provider === provider,
      'P3 provider record kind/profile differs');
  checkP3ScenarioManifest(context.scenarioManifest);
  checkP3Coverage(record);
  checkCandidate(record, context);
  const summaries = [];
  for (const row of record.scenarios) {
    const session = record.sessions.find(value => value.id === row.sessionId);
    const protocol = row.preambleParameters;
    need(
        protocol?.PROJECT === session.cwd && path.isAbsolute(protocol.NODE || '') &&
            path.basename(protocol.NODE) === 'node' && session.collector?.nodePath === protocol.NODE &&
            session.collector.script?.path === protocol.COLLECTOR &&
            protocol.VERSION === record.package.version.path && typeof protocol.STDIO === 'string' &&
            protocol.STDIO.startsWith(session.cwd + '/'),
        'Neutral collection protocol paths are not bound to this case/candidate');
    checkCollector(session.collector.script, 'stdio', context);
    const captured = checkP3Session(session, row, context),
          required = context.scenarioManifest.rows.find(value => value.id === row.id),
          prompt = bytes(row.prompt, context, 'Actual authored prompt').toString('utf8');
    need(
        prompt === renderPrompt(required, row.parameters || {}) &&
            bytes(captured.parts[0].capture.stdin, context, 'Native original prompt').toString('utf8') ===
                renderP3OriginPrompt(context.scenarioManifest, row),
        'Frozen authored prompt absent from actual origin stdin');
    need(
        Array.isArray(row.materials) && row.materials.length === required.materials.length,
        'Actual prompt materials missing');
    const materials = [];
    for (const material of required.materials) {
      const actual = row.materials.find(value => value.name === material.name);
      need(actual && same(actual.file.identity, material.identity), 'Actual material identity differs');
      const content = material.packageMember ?
          expectedFile(actual.file, material.packageMember, context, 'Prompt package material') :
          bytes(actual.file, context, 'Prompt material');
      if (material.content !== undefined)
        need(content.equals(Buffer.from(material.content)), 'Prompt material bytes differ');
      materials.push(actual.file);
    }
    const responsePart = proofContext(captured, row.response, context),
          response = context.nativeProof.assistantText(
              context.nativeProof.anchor(responsePart.raw, row.response.anchor), provider),
          commandEntries = session.commands || [],
          results = commandEntries.map(
              command =>
                  ({...checkP3Command(captured, command, context), captureIndex: command.captureIndex}));
    need(
        row.response.captureIndex === captured.parts.length - 1 &&
            commandEntries.every(
                command => command.captureIndex < row.response.captureIndex ||
                    row.response.anchor.offset >
                        (command.proof.polls?.at(-1)?.result || command.proof.result).offset),
        'Scenario response predates the final capture or a native result');
    checkP3RecordedCommands(captured, commandEntries, context);
    reviewScenario(row, captured, commandEntries, materials, context);
    checkP3ScenarioOutcome(row, captured, results, response, context);
    summaries.push({
      id: row.id,
      sessionId: session.id,
      status: 'pass',
      captureCount: session.captures.length,
      observedModels: [...new Set(
          captured.parts.flatMap(value => value.capture.observedModels.map(model => model.value)))],
      endpointCategories: [...new Set(captured.parts.map(value => value.capture.runtime.endpoint.category))]
    });
  }
  return {
    provider,
    status: 'pass',
    sessionIds: record.sessions.map(value => value.id),
    projectPaths: record.sessions.map(value => value.cwd),
    physicalProjects:
        record.sessions.map(value => ({device: value.project.device, inode: value.project.inode})),
    scenarioIds: record.scenarios.map(value => value.id),
    scenarios: summaries,
    scope:
        '18 distinct fresh native origins; original captures, authenticated candidate/callers/collectors, explicit continuations and independent semantic/image observations'
  };
}
export function checkP3ProviderManifest(manifest, context) {
  const report = {
    schemaVersion: 1,
    kind: 'phoenix-diagram-p3-provider-report',
    profile: 'p3',
    status: 'incomplete',
    providers: [],
    diagnostics: []
  };
  try {
    need(
        manifest?.schemaVersion === 1 && manifest.kind === 'phoenix-diagram-p3-provider-manifest' &&
            Array.isArray(manifest.providers) &&
            same(manifest.providers.map(value => value.provider).sort(), [...PROVIDERS].sort()),
        'Both P3 provider records are required exactly once');
    const origins = new Set(), projects = new Set();
    for (const entry of manifest.providers) {
      try {
        const record = json(entry.record, context, 'Original provider record'),
              checked = checkP3ProviderRecord(record, {...context, provider: entry.provider});
        for (const session of record.sessions) {
          const key = session.project.device + ':' + session.project.inode;
          need(!origins.has(session.id) && !projects.has(key), 'P3 origins/projects are shared across hosts');
          origins.add(session.id);
          projects.add(key);
        }
        report.providers.push(checked);
      } catch (error) {
        report.providers.push({provider: entry.provider, status: 'incomplete'});
        report.diagnostics.push(
            {provider: entry.provider, code: error.code || 'p3/provider-incomplete', message: error.message});
      }
    }
    if (report.providers.length === 2 && report.providers.every(value => value.status === 'pass') &&
        report.diagnostics.length === 0)
      report.status = 'pass';
  } catch (error) {
    report.diagnostics.push({code: error.code || 'p3/provider-incomplete', message: error.message});
  }
  return report;
}
