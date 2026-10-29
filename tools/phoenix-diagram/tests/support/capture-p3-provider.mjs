#!/usr/bin/env node
// Starts only the selected native client. Never returns a semantic/visual verdict.
// Self-contained authenticated harness member; all dependencies are Node builtins.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {createHash, randomUUID} from 'node:crypto';
import {spawn, spawnSync} from 'node:child_process';
import {isDeepStrictEqual as same} from 'node:util';
import {fileURLToPath} from 'node:url';

export const COLLECTOR_MEMBER = 'tools/phoenix-diagram/tests/support/capture-p3-provider.mjs';
export const KIND = 'phoenix-diagram-p3-native-capture';
export const HELP =
    'Usage: node capture-p3-provider.mjs --plan|--execute REQUEST.json\nStructured P3 request only; inherits this process existing client service/auth configuration, never installs a client or changes global settings. --plan does not start a provider.\n';
const hash = bytes => ({sha256: createHash('sha256').update(bytes).digest('hex'), bytes: bytes.length}),
      encode = value => Buffer.from(JSON.stringify(value, null, 2) + '\n');
function need(value, message) {
  if (!value) throw new Error('P3 provider capture: ' + message);
}
function physical(stat) {
  return {
    device: String(stat.dev),
    inode: String(stat.ino),
    links: Number(stat.nlink),
    mode: Number(stat.mode & 0o7777n),
    uid: Number(stat.uid)
  };
}
function snapshot(file) {
  const before = fs.lstatSync(file, {bigint: true});
  need(
      before.isFile() && !before.isSymbolicLink() && before.nlink === 1n &&
          Number(before.uid) === process.getuid() && fs.realpathSync(file) === file,
      'owned canonical single-link file required');
  const fd = fs.openSync(file, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW | fs.constants.O_NONBLOCK);
  try {
    const opened = fs.fstatSync(fd, {bigint: true}), content = fs.readFileSync(fd),
          after = fs.fstatSync(fd, {bigint: true}), current = fs.lstatSync(file, {bigint: true});
    need(
        same(physical(before), physical(opened)) && same(physical(opened), physical(after)) &&
            same(physical(after), physical(current)) && opened.mtimeNs === after.mtimeNs &&
            opened.ctimeNs === after.ctimeNs && after.size === BigInt(content.length),
        'file changed while reading');
    return {path: file, identity: hash(content), binding: physical(after), content};
  } finally {
    fs.closeSync(fd);
  }
}
function ref({content, ...value}) {
  return value;
}
function readBound(value) {
  const file = snapshot(value.path);
  need(same(ref(file), value), 'bound evidence changed');
  return file;
}
function directory(file) {
  const stat = fs.lstatSync(file, {bigint: true});
  need(
      stat.isDirectory() && !stat.isSymbolicLink() && fs.realpathSync(file) === file &&
          Number(stat.uid) === process.getuid(),
      'owned canonical project directory required');
  const {links, ...binding} = physical(stat);
  return binding;
}
function write(file, value) {
  const fd = fs.openSync(
      file, fs.constants.O_WRONLY | fs.constants.O_CREAT | fs.constants.O_EXCL | fs.constants.O_NOFOLLOW,
      0o600);
  try {
    fs.writeFileSync(fd, Buffer.isBuffer(value) ? value : encode(value));
    fs.fsyncSync(fd);
  } finally {
    fs.closeSync(fd);
  }
  return ref(snapshot(file));
}
function sourceOrigin(value) {
  const url = new URL(value);
  need(['http:', 'https:'].includes(url.protocol), 'unsupported configured endpoint protocol');
  return url.origin;
}
// Only public service-selection keys are read. Tokens and complete configuration
// bytes are neither returned nor copied into evidence.
function tomlSelection(file) {
  const result = {};
  if (!fs.existsSync(file)) return result;
  let section = '';
  for (const line of fs.readFileSync(file, 'utf8').split(/\r?\n/)) {
    const heading = /^\s*\[([^\]]+)\]\s*(?:#.*)?$/.exec(line);
    if (heading) {
      section = heading[1];
      continue;
    }
    const row = /^\s*(model|model_provider|base_url|wire_api)\s*=\s*(.*?)\s*$/.exec(line);
    if (!row) continue;
    let value;
    try {
      value = row[2].startsWith('"') ? JSON.parse(row[2]) :
          /^'[^']*'$/.test(row[2])   ? row[2].slice(1, -1) :
                                       null;
    } catch {
    }
    need(typeof value === 'string', 'unsupported public TOML service-selection syntax');
    result[(section ? section + '.' : '') + row[1]] = value;
  }
  return result;
}
export function observeRuntime(provider, env = process.env, requestedModel, cwd = process.cwd()) {
  const sources = [], baseEnv = provider === 'codex' ? 'OPENAI_BASE_URL' : 'ANTHROPIC_BASE_URL',
        modelEnv = provider === 'codex' ? 'OPENAI_MODEL' : 'ANTHROPIC_MODEL';
  let model = env[modelEnv], endpoint = env[baseEnv], modelProvider = null;
  if (endpoint) sources.push({source: 'environment', key: baseEnv, origin: sourceOrigin(endpoint)});
  if (model) sources.push({source: 'environment', key: modelEnv, value: model});
  if (provider === 'codex') {
    const file = path.join(env.CODEX_HOME || path.join(os.homedir(), '.codex'), 'config.toml'),
          values = tomlSelection(file);
    const localFile = path.join(cwd, '.codex/config.toml'), local = tomlSelection(localFile);
    for (const key of ['model_provider', 'base_url'])
      need(
          !local[key] || !values[key] || local[key] === values[key],
          'conflicting public client service selection requires an explicit isolated configuration');
    modelProvider = local.model_provider || values.model_provider || null;
    if (!model && (local.model || values.model)) {
      model = local.model || values.model;
      sources.push({source: local.model ? localFile : file, key: 'model', value: model});
    }
    if (modelProvider) {
      sources.push({source: file, key: 'model_provider', value: modelProvider});
      const key = `model_providers.${modelProvider}.base_url`;
      if (!endpoint && (local[key] || values[key])) {
        endpoint = local[key] || values[key];
        sources.push({source: local[key] ? localFile : file, key, origin: sourceOrigin(endpoint)});
      }
    }
    need(
        !modelProvider || modelProvider === 'openai' || endpoint,
        'custom provider endpoint is not observable');
  } else {
    need(
        !['CLAUDE_CODE_USE_BEDROCK', 'CLAUDE_CODE_USE_VERTEX', 'CLAUDE_CODE_USE_FOUNDRY'].some(
            key => env[key] && !['0', 'false'].includes(env[key])),
        'third-party client transport requires its own observed endpoint policy');
    const base = env.CLAUDE_CONFIG_DIR || path.join(os.homedir(), '.claude');
    for (const file
             of [path.join(base, 'settings.json'), path.join(cwd, '.claude/settings.json'),
                 path.join(cwd, '.claude/settings.local.json')])
      if (fs.existsSync(file)) {
        let value;
        try {
          value = JSON.parse(fs.readFileSync(file, 'utf8'));
        } catch {
          need(false, 'cannot parse the configured client public settings');
        }
        const settings = value.env || {};
        if (!model && (settings.ANTHROPIC_MODEL || value.model)) {
          model = settings.ANTHROPIC_MODEL || value.model;
          sources.push(
              {source: file, key: settings.ANTHROPIC_MODEL ? 'env.ANTHROPIC_MODEL' : 'model', value: model});
        }
        if (settings.ANTHROPIC_BASE_URL && endpoint)
          need(
              sourceOrigin(settings.ANTHROPIC_BASE_URL) === sourceOrigin(endpoint),
              'conflicting public client endpoint configuration');
        if (!endpoint && settings.ANTHROPIC_BASE_URL) {
          endpoint = settings.ANTHROPIC_BASE_URL;
          sources.push({source: file, key: 'env.ANTHROPIC_BASE_URL', origin: sourceOrigin(endpoint)});
        }
      }
  }
  if (requestedModel) {
    model = requestedModel;
    sources.push({source: 'request', key: 'model', value: model});
  }
  return {
    policy: 'inherit-config',
    requestedModel: model || 'client-default',
    endpoint: {
      category: endpoint ? 'custom-endpoint' : 'client-default',
      origin: endpoint ? sourceOrigin(endpoint) : null,
      modelProvider
    },
    sources,
    credentialSource:
        'existing client authentication and inherited process environment; secret values not captured'
  };
}
export function buildArguments(request, {sessionId, previousId} = {}) {
  need(
      ['codex', 'claude-code'].includes(request.provider) && request.endpointPolicy === 'inherit-config',
      'explicit provider and inherit-config policy required');
  need(
      !request.model || typeof request.model === 'string' && request.model && !/[\0\r\n]/.test(request.model),
      'invalid model');
  const model = request.model ? ['--model', request.model] : [];
  if (request.provider === 'claude-code')
    return [
      '--print', '--verbose', '--output-format', 'stream-json', '--setting-sources', 'user,project,local',
      '--strict-mcp-config', '--no-chrome', '--tools', 'Read,Write,Edit,Bash,Glob,Grep,Skill',
      '--allowedTools', 'Read,Write,Edit,Bash,Glob,Grep,Skill', '--permission-mode', 'dontAsk', ...model,
      ...(previousId ? ['--resume', previousId] : ['--session-id', sessionId])
    ];
  const common =
      ['--json', '--ignore-rules', '--skip-git-repo-check', '--config', 'allow_login_shell=false', ...model];
  return previousId ? ['exec', 'resume', ...common, previousId, '-'] : [
    'exec', ...common, '--color', 'never', '--sandbox', 'danger-full-access', '--config',
    'project_doc_max_bytes=0', '--config', 'features.memories=false', '--config',
    'features.memory_tool=false', '--cd', request.cwd, '-'
  ];
}
function projectObservation(cwd, project) {
  const entries = [];
  function walk(folder, prefix = '') {
    for (const name of fs.readdirSync(folder).sort()) {
      const relative = prefix ? prefix + '/' + name : name, file = path.join(folder, name),
            stat = fs.lstatSync(file);
      need(entries.length < 100000, 'project observation limit exceeded');
      if (stat.isDirectory() && !stat.isSymbolicLink())
        walk(file, relative);
      else if (stat.isFile() && !stat.isSymbolicLink()) {
        const value = snapshot(file);
        entries.push({path: relative, kind: 'file', identity: value.identity, mode: value.binding.mode});
      } else
        entries.push({path: relative, kind: stat.isSymbolicLink() ? 'symlink' : 'special'});
    }
  }
  walk(cwd);
  return {schemaVersion: 1, kind: 'phoenix-diagram-p3-project-observation', cwd, project, entries};
}
export function lineIndex(raw) {
  const lines = [];
  let offset = 0;
  while (offset < raw.length) {
    const end = raw.indexOf(10, offset), limit = end < 0 ? raw.length : end + 1,
          bytes = raw.subarray(offset, limit);
    if (bytes.toString('utf8').trim()) {
      const event = JSON.parse(bytes);
      lines.push({offset, length: bytes.length, sha256: hash(bytes).sha256, type: event.type});
    }
    offset = limit;
  }
  return lines;
}
function sessionEvents(raw) {
  return raw.toString('utf8').split('\n').filter(Boolean).map(line => JSON.parse(line));
}
function modelObservations(raw, provider, minimumOffset = 0) {
  const index = lineIndex(raw), events = sessionEvents(raw), rows = [];
  for (let at = 0; at < events.length; at++) {
    const event = events[at],
          model = provider === 'claude-code' ? event.model || event.message?.model :
        event.type === 'turn_context'        ? event.payload?.model :
        event.type === 'session_meta'        ? event.payload?.model :
                                               null;
    if (typeof model === 'string' && model && index[at].offset >= minimumOffset)
      rows.push({value: model, anchor: index[at]});
  }
  return rows;
}
function nativeSessionFile(id, startedAt, env) {
  const root = path.join(env.CODEX_HOME || path.join(os.homedir(), '.codex'), 'sessions'), matches = [];
  for (const delta of [-86400000, 0, 86400000]) {
    const date = new Date(startedAt + delta);
    for (const utc of [false, true]) {
      const year = utc ? date.getUTCFullYear() : date.getFullYear(),
            month = (utc ? date.getUTCMonth() : date.getMonth()) + 1,
            day = utc ? date.getUTCDate() : date.getDate(),
            folder =
                path.join(root, String(year), String(month).padStart(2, '0'), String(day).padStart(2, '0'));
      if (fs.existsSync(folder))
        for (const name of fs.readdirSync(folder))
          if (name.endsWith('-' + id + '.jsonl') && !matches.includes(path.join(folder, name)))
            matches.push(path.join(folder, name));
    }
  }
  need(matches.length === 1, 'unique current native Codex session file unavailable');
  return matches[0];
}
export async function captureProvider(request, {env = process.env} = {}) {
  need(
      request?.schemaVersion === 1 && request.kind === 'phoenix-diagram-p3-provider-request' &&
          typeof request.scenarioId === 'string',
      'invalid P3 capture request');
  need(
      !Object.hasOwn(request, 'argv') && !Object.hasOwn(request, 'environment'),
      'request cannot supply arbitrary argv or persist environment');
  const project = directory(request.cwd);
  need(
      path.isAbsolute(request.outputDir) && path.normalize(request.outputDir) === request.outputDir &&
          !fs.existsSync(request.outputDir) && !request.outputDir.startsWith(request.cwd + path.sep),
      'use a new output directory outside the project');
  directory(path.dirname(request.outputDir));
  need(
      path.isAbsolute(request.program) &&
          ['codex', 'codex.js', 'claude'].includes(path.basename(request.program)),
      'select an explicit native codex/claude entry');
  const program = snapshot(fs.realpathSync(request.program)), self = snapshot(fileURLToPath(import.meta.url)),
        prompt = readBound(request.prompt),
        runtime = observeRuntime(request.provider, env, request.model, request.cwd);
  let previous = null;
  if (request.previousCapture) {
    previous = JSON.parse(readBound(request.previousCapture).content);
    need(
        previous.kind === KIND && previous.provider === request.provider &&
            previous.scenarioId === request.scenarioId && previous.cwd === request.cwd &&
            same(previous.project, project) && same(previous.collector?.identity, self.identity) &&
            previous.sessionId && previous.exitCode === 0,
        'continuation is not this successful case/session/project');
  }
  const sessionId = previous?.sessionId || randomUUID(),
        argv = buildArguments(request, {sessionId, previousId: previous?.sessionId});
  fs.mkdirSync(request.outputDir, {mode: 0o700});
  const outputBinding = directory(request.outputDir),
        stdin = write(path.join(request.outputDir, 'stdin.txt'), prompt.content),
        origin = write(
            path.join(request.outputDir, 'project-before.json'), projectObservation(request.cwd, project));
  const versionResult =
      spawnSync(program.path, ['--version'], {cwd: request.cwd, env, encoding: 'utf8', timeout: 30000});
  const version = write(path.join(request.outputDir, 'version.json'), {
    program: ref(program),
    entryPath: request.program,
    argv: ['--version'],
    exitCode: versionResult.status,
    signal: versionResult.signal,
    stdout: versionResult.stdout || '',
    stderr: versionResult.stderr || ''
  });
  need(
      versionResult.status === 0 &&
          (request.provider === 'codex' ? /codex/i : /Claude Code/).test(versionResult.stdout),
      'actual native client version failed');
  const stdoutPath = path.join(request.outputDir, 'stdout.jsonl'),
        stderrPath = path.join(request.outputDir, 'stderr.txt'),
        fds = [stdoutPath, stderrPath].map(
            file => fs.openSync(
                file,
                fs.constants.O_RDWR | fs.constants.O_CREAT | fs.constants.O_EXCL | fs.constants.O_NOFOLLOW,
                0o600)),
        startedAt = Date.now();
  let result, timedOut = false;
  const childEnv = {
    ...env,
    ...(request.provider === 'claude-code' ?
            {CLAUDE_CODE_DISABLE_AUTO_MEMORY: '1', CLAUDE_CODE_DISABLE_CLAUDE_MDS: '1'} :
            {})
  };
  try {
    result = await new Promise((resolve, reject) => {
      const child = spawn(
                program.path, argv,
                {cwd: request.cwd, env: childEnv, stdio: ['pipe', ...fds], detached: true}),
            kill = signal => {
              if (child.exitCode === null && child.signalCode === null) try {
                  process.kill(-child.pid, signal);
                } catch (error) {
                  if (error.code !== 'ESRCH') reject(error);
                }
            }, interrupt = () => kill('SIGTERM'), timer = setTimeout(() => {
                                                    timedOut = true;
                                                    kill('SIGKILL');
                                                  }, 3600000), cleanup = () => {
              clearTimeout(timer);
              process.removeListener('SIGTERM', interrupt);
              process.removeListener('SIGINT', interrupt);
            };
      process.once('SIGTERM', interrupt);
      process.once('SIGINT', interrupt);
      child.once('error', error => {
        cleanup();
        reject(error);
      });
      child.stdin.on('error', () => kill('SIGTERM'));
      child.once('close', (exitCode, signal) => {
        cleanup();
        resolve({exitCode, signal});
      });
      child.stdin.end(prompt.content);
    });
  } finally {
    for (const fd of fds) {
      fs.fsyncSync(fd);
      fs.closeSync(fd);
    }
  }
  need(
      same(directory(request.cwd), project) && same(directory(request.outputDir), outputBinding) &&
          same(ref(snapshot(program.path)), ref(program)) && same(ref(snapshot(self.path)), ref(self)),
      'provider/project/collector changed');
  const stdout = ref(snapshot(stdoutPath)), stderr = ref(snapshot(stderrPath)), dto = {
    schemaVersion: 1,
    kind: KIND,
    provider: request.provider,
    scenarioId: request.scenarioId,
    collector: ref(self),
    node: {path: fs.realpathSync(process.execPath), version: process.version},
    program: ref(program),
    entryPath: request.program,
    argv,
    cwd: request.cwd,
    project,
    origin: !previous,
    previousCapture: request.previousCapture || null,
    stdin,
    stdout,
    stderr,
    version,
    runtime,
    startedAt: new Date(startedAt).toISOString(),
    finishedAt: new Date().toISOString(),
    ...result,
    timedOut,
    projectBefore: origin,
    projectAfter:
        write(path.join(request.outputDir, 'project-after.json'), projectObservation(request.cwd, project))
  };
  try {
    const events = sessionEvents(snapshot(stdoutPath).content),
          start = events.find(
              event => request.provider === 'codex' ? event.type === 'thread.started' :
                                                      event.type === 'system' && event.subtype === 'init'),
          id = start?.thread_id || start?.session_id;
    need(
        id && (!previous || id === sessionId) && (request.provider !== 'claude-code' || id === sessionId),
        'native session start identity differs');
    dto.sessionId = id;
    if (request.provider === 'codex') {
      const source = snapshot(nativeSessionFile(id, startedAt, env));
      dto.nativeSessionFile = source.path;
      dto.transcriptSource = ref(source);
      dto.transcript = write(path.join(request.outputDir, 'session.jsonl'), source.content);
      dto.format = 'codex-session-json';
    } else {
      dto.transcript = stdout;
      dto.format = 'claude-stream-json';
    }
    dto.lineIndex =
        write(path.join(request.outputDir, 'line-index.json'), lineIndex(readBound(dto.transcript).content));
    const currentTranscript = readBound(dto.transcript).content;
    const earlierTranscript =
        previous && request.provider === 'codex' ? readBound(previous.transcript).content : null;
    if (earlierTranscript)
      need(
          currentTranscript.subarray(0, earlierTranscript.length).equals(earlierTranscript),
          'continued native transcript lost its original prefix');
    dto.observedModels =
        modelObservations(currentTranscript, request.provider, earlierTranscript?.length || 0);
    dto.transcriptStatus = dto.observedModels.length ? 'captured' : 'model-unavailable';
  } catch (error) {
    dto.transcriptStatus = 'unavailable';
    dto.transcriptError =
        'Native transcript/session/model verification unavailable; retain original private streams';
  }
  const capture = write(path.join(request.outputDir, 'capture.json'), dto);
  return {capture, dto};
}
export async function main(argv = process.argv.slice(2)) {
  if (argv.length === 1 && argv[0] === '--help') {
    process.stdout.write(HELP);
    return 0;
  }
  try {
    need(
        argv.length === 2 && ['--plan', '--execute'].includes(argv[0]),
        'use --plan or --execute with one request');
    const request = JSON.parse(snapshot(path.resolve(argv[1])).content);
    if (argv[0] === '--plan') {
      const args = buildArguments(request, {sessionId: '00000000-0000-4000-8000-000000000000'});
      process.stdout.write(JSON.stringify({
        status: 'prepared-only',
        providerStarted: false,
        provider: request.provider,
        argv: args,
        runtime: observeRuntime(request.provider, process.env, request.model, request.cwd)
      }) + '\n');
      return 0;
    }
    const {capture, dto} = await captureProvider(request);
    process.stdout.write(JSON.stringify({
      capture,
      sessionId: dto.sessionId || null,
      exitCode: dto.exitCode,
      transcriptStatus: dto.transcriptStatus
    }) + '\n');
    return dto.exitCode === 0 && dto.signal === null && !dto.timedOut ? 0 : 1;
  } catch (error) {
    process.stderr.write(
        (error.message.startsWith('P3 provider capture:') ?
             error.message :
             'P3 provider capture: cannot read or parse the requested input') +
        '\n');
    return 4;
  }
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url))
  process.exitCode = await main();
