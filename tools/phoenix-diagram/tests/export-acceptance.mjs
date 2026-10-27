// Real-browser maintenance controller. No renderer/consumer runtime dependency.
// Browser probes derive from frozen Archify 3.1.0 export tests (MIT); see
// ../viewer/LICENSE.txt and support/export-browser.mjs for the transport source.
import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { pathToFileURL } from 'node:url';
import { setTimeout as delay } from 'node:timers/promises';
import { DEFAULT_TIMEOUTS, discoverBrowser, ExportBrowser } from './support/export-browser.mjs';
import { runExplore, verifyExplore } from './support/explore-reader.mjs';
import { installExportProbe, awaitReader, readerClickPoint, snapshotReader, printMotionSnapshot, motionSuppressionSnapshot, inspectBlob, readResources, contentObservation } from './support/export-page.mjs';
import { REQUIRED_READER_INTERACTIONS, runReaderInteractions } from './support/export-reader.mjs';

export const REQUIRED_EXPORT_SCOPE = Object.freeze({ themes: ['dark', 'light'], modes: ['still', 'live'], formats: ['svg', 'png', 'jpeg', 'webp', 'webm'], clipboard: ['success', 'permission-denied', 'capability-unavailable'], print: { themes: ['dark', 'light'], modes: ['still', 'live'], method: 'cdp-print-media-and-native-print-to-pdf' }, readerInteractions: REQUIRED_READER_INTERACTIONS,
  motionSuppression: { themes: ['dark', 'light'], modes: ['still', 'live'], conditions: ['reduced-motion', 'page-hidden'], methods: { 'reduced-motion': 'cdp-prefers-reduced-motion', 'page-hidden': 'cdp-owned-target-visibility' } } });
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');

export function verifyCanonicalFacts(actual, expected) {
  if (!actual || !expected) throw new Error('Canonical SVG is missing authored facts');
  for (const field of ['viewBox', 'nodes', 'edges', 'text']) if (JSON.stringify(actual[field]) !== JSON.stringify(expected[field])) throw new Error('Canonical SVG changed authored ' + field);
}
export function verifyPrintSample(sample) {
  if (!['still', 'live'].includes(sample?.mode) || sample.before?.mode !== sample.mode || !equalState(sample.before, sample.during) || !equalState(sample.before, sample.after)) throw new Error('Print changed authored Reader state or failed to restore its mode');
  const { motionBefore: before, motionDuring: during, motionAfter: after } = sample;
  if (before?.printMedia !== false || during?.printMedia !== true || after?.printMedia !== false || ![before, during, after].every(value => Number.isInteger(value.running) && value.running >= 0) || during.running !== 0 || !(during.staticCount > 0) || during.hiddenStatic !== 0) throw new Error('Print media did not pause motion while retaining the static diagram');
  if (sample.motionApplicable && (!(before.running > 0) || !(after.running > 0))) throw new Error('Print did not restore observable Live motion');
  if (sample.mode === 'still' && (before.running || after.running)) throw new Error('Print changed the Still motion preference');
  const events = sample.nativePrint?.events, pdf = sample.nativePrint?.pdf;
  if (!Array.isArray(events) || events.length !== 2 || events[0].type !== 'beforeprint' || events[1].type !== 'afterprint' || pdf?.signature !== 'PDF' || !pdf.eof || !(pdf.pages > 0) || !(pdf.identity?.bytes > 32) || !/^[a-f0-9]{64}$/.test(pdf.identity?.sha256 || '') || typeof pdf.path !== 'string') throw new Error('Print lacks actual native lifecycle events or valid PDF bytes');
  return true;
}

function verifyMotionSuppressionSample(sample) {
  const need = (value, message) => { if (!value) throw new Error('Motion suppression ' + message); };
  const { before, during, after, mode, condition } = sample, snapshots = [before, during, after];
  need(sample.status === 'pass' && sample.method === REQUIRED_EXPORT_SCOPE.motionSuppression.methods[condition], 'sample is incomplete or uses an unrecognized method');
  for (const state of snapshots) {
    const reader = state?.reader;
    need(reader && ['mode', 'owner', 'focus', 'view', 'viewBox', 'preset', 'transform', 'clip', 'nodes', 'edges', 'text'].every(field => Object.hasOwn(reader, field)) && typeof reader.viewBox === 'string' && reader.viewBox.length > 0 && Array.isArray(reader.nodes) && reader.nodes.length > 0 && reader.nodes.every(node => typeof node.id === 'string' && node.id && Array.isArray(node.text)) && Array.isArray(reader.edges) && reader.edges.every(edge => typeof edge.geometry === 'string' && typeof edge.from === 'string' && typeof edge.to === 'string') && Array.isArray(reader.text) && reader.text.length > 0 && reader.text.every(text => typeof text === 'string') && typeof reader.eligibleMotion === 'boolean', 'lacks complete authored Reader facts');
    need(Number.isInteger(state.animationCount) && state.animationCount >= 0 && Number.isInteger(state.running) && state.running >= 0 && state.running <= state.animationCount && typeof state.capable === 'boolean' && state.printMedia === false, 'lacks actual screen animation observations');
    need(Number.isInteger(state.meaning?.total) && state.meaning.total > 0 && Array.isArray(state.meaning.visible) && state.meaning.visible.length > 0 && new Set(state.meaning.visible).size === state.meaning.visible.length && state.meaning.visible.every(index => Number.isInteger(index) && index >= 0 && index < state.meaning.total), 'lacks painted static meaning');
  }
  need(before.reader.mode === mode && during.reader.mode === 'still' && after.reader.mode === mode, 'did not pause and restore the selected mode');
  for (const state of [during, after]) {
    need(['owner', 'focus', 'view', 'viewBox', 'preset', 'transform', 'clip', 'nodes', 'edges', 'text', 'eligibleMotion'].every(field => JSON.stringify(before.reader[field]) === JSON.stringify(state.reader[field])), 'changed authored facts or Reader exploration');
    need(state.capable === before.capable && state.preference === before.preference && state.meaning.total === before.meaning.total && before.meaning.visible.every(index => state.meaning.visible.includes(index)), 'lost static meaning or changed the saved preference');
  }
  need(before.preference === (mode === 'live' ? 'live' : null), 'does not bind the selected user preference');
  need([before, after].every(state => state.reducedMotion === false && state.hidden === false && state.visibilityState === 'visible' && state.documentHidden === null && state.button?.disabled === false && state.button.pressed === (mode === 'live' ? 'true' : 'false')), 'did not restore actual visible, unrestricted playback');
  need(during.running === 0 && during.button?.pressed === 'false', 'left running motion during suppression');
  const applicable = mode === 'live' && ((before.capable && before.reader.eligibleMotion) || before.animationCount > 0);
  need(sample.motionApplicable === applicable && (!applicable || before.running > 0 && after.running > 0) && (mode !== 'still' || before.running === 0 && after.running === 0), 'lacks a positive Live pause/resume or changed Still playback');
  need(Array.isArray(sample.inputs) && sample.inputs.some(input => input.type === 'click' && input.trusted === true && input.target?.id === 'btn-motion'), 'lacks a trusted user playback selection');
  need(Array.isArray(sample.events), 'lacks native transition events');
  if (condition === 'reduced-motion') {
    const events = sample.events.filter(event => event.type === 'reduced-motion');
    need(during.reducedMotion === true && during.hidden === false && during.visibilityState === 'visible' && during.documentHidden === null && during.button.disabled === true && events.length === 2 && events.every(event => event.trusted === true) && events[0].matches === true && events[1].matches === false, 'did not observe the actual reduced-motion transition and restoration');
  } else {
    const events = sample.events.filter(event => event.type === 'visibilitychange'), targets = sample.ownedTargets;
    need(condition === 'page-hidden' && targets && ['subject', 'cover', 'browserContext'].every(key => typeof targets[key] === 'string' && targets[key]) && targets.subject !== targets.cover, 'does not identify isolated owned visibility targets');
    need(during.reducedMotion === false && during.hidden === true && during.visibilityState === 'hidden' && during.documentHidden === 'true' && during.button.disabled === false && events.length === 2 && events.every(event => event.trusted === true) && events[0].hidden === true && events[0].visibilityState === 'hidden' && events[1].hidden === false && events[1].visibilityState === 'visible', 'did not observe an actual hidden document and its native restoration');
  }
}

export function verifyMotionSuppression(value) {
  if (value?.status !== 'pass' || !Array.isArray(value.observations) || value.observations.length !== 8) throw new Error('Motion suppression scope is incomplete');
  const seen = new Set(), scope = REQUIRED_EXPORT_SCOPE.motionSuppression;
  for (const sample of value.observations) {
    const key = `${sample.theme}:${sample.mode}:${sample.condition}`;
    if (!scope.themes.includes(sample.theme) || !scope.modes.includes(sample.mode) || !scope.conditions.includes(sample.condition) || seen.has(key)) throw new Error('Motion suppression scope is duplicated or unrecognized');
    seen.add(key); verifyMotionSuppressionSample(sample);
  }
  return true;
}

export function artifactIdentity(file) {
  const resolved = path.resolve(file), stat = fs.lstatSync(resolved);
  if (!stat.isFile() || stat.isSymbolicLink()) throw new Error('Acceptance input must be an ordinary HTML file');
  const bytes = fs.readFileSync(resolved);
  return { path: resolved, sha256: sha256(bytes), bytes: bytes.length };
}
export function validateInput(options) {
  if (!options || typeof options.artifactPath !== 'string' || !/\.html?$/i.test(options.artifactPath) || typeof options.evidenceDir !== 'string') throw new Error('artifactPath HTML and evidenceDir are required');
  const timeouts = { ...DEFAULT_TIMEOUTS, ...options.timeouts };
  for (const [name, value] of Object.entries(timeouts)) if (!Number.isFinite(value) || value <= 0) throw new Error(`Invalid ${name} timeout`);
  if (options.expectedLiteral !== undefined && typeof options.expectedLiteral !== 'string') throw new Error('expectedLiteral must be text');
  if (options.sentinelName !== undefined && !/^__phoenixDiagram[A-Za-z0-9_]+$/.test(options.sentinelName)) throw new Error('sentinelName must be a fixed Phoenix test identifier');
  if (options.offline !== undefined && typeof options.offline !== 'boolean') throw new Error('offline must be boolean');
  return timeouts;
}
// CDP network emulation is browser-level evidence; it is not an OS IP sandbox.
export async function configureNetwork(send) {
  await send('Network.enable');
  await send('Network.emulateNetworkConditions', { offline: true, latency: 0, downloadThroughput: -1, uploadThroughput: -1 });
}
export function verifyFileSignature(bytes, format, mime) {
  if (!Buffer.isBuffer(bytes) || !bytes.length) throw new Error('Export file is empty');
  const normalized = String(mime).split(';')[0].trim().toLowerCase();
  const expected = { png: 'image/png', jpeg: 'image/jpeg', webp: 'image/webp', svg: 'image/svg+xml', webm: 'video/webm' }[format];
  if (!expected || normalized !== expected) throw new Error(`Export MIME mismatch: ${format} received ${mime}`);
  let signature;
  if (format === 'png' && bytes.subarray(0, 8).equals(Buffer.from('89504e470d0a1a0a', 'hex')) && bytes.length >= 24) signature = 'PNG';
  if (format === 'jpeg' && bytes.length >= 4 && bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff && bytes[bytes.length - 2] === 0xff && bytes[bytes.length - 1] === 0xd9) signature = 'JPEG';
  if (format === 'webp' && bytes.length >= 16 && bytes.toString('ascii', 0, 4) === 'RIFF' && bytes.toString('ascii', 8, 12) === 'WEBP' && bytes.readUInt32LE(4) + 8 === bytes.length && /^VP8[ LX]$/.test(bytes.toString('ascii', 12, 16))) signature = 'RIFF/WEBP';
  if (format === 'svg' && /^(?:\uFEFF)?\s*(?:<\?xml\s[^?]+\?>\s*)?(?:<!--[\s\S]*?-->\s*)*<svg(?=[\s/>])/.test(bytes.toString('utf8'))) signature = 'XML/SVG';
  if (format === 'webm' && bytes.subarray(0, 4).equals(Buffer.from('1a45dfa3', 'hex')) && bytes.subarray(0, 512).includes(Buffer.from('webm'))) signature = 'EBML/WebM';
  if (!signature) throw new Error(`Export has no valid ${format} byte signature`);
  return signature;
}

function errorRecord(error, stage) { return { stage, code: error.code || 'acceptance/failed', message: error.message || String(error) }; }
async function waitFor(predicate, timeoutMs, label) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) { const result = await predicate(); if (result) return result; await delay(25); }
  const error = new Error(`${label} timed out after ${timeoutMs}ms`); error.code = 'acceptance/timeout'; throw error;
}
function makeDirectory(parent, name) {
  fs.mkdirSync(parent, { recursive: true });
  const directory = path.join(fs.realpathSync(parent), name);
  fs.mkdirSync(directory, { mode: 0o700 }); // Exclusive: never replace earlier evidence.
  return directory;
}
function equalState(before, after) {
  return ['mode', 'owner', 'focus', 'view', 'viewBox', 'preset', 'transform', 'clip', 'nodes', 'edges', 'text'].every(field => JSON.stringify(before[field]) === JSON.stringify(after[field]));
}
function resourceClean(resources) { return resources.anchorsAttached === 0 && resources.urls.every(url => url.revocations === 1) && resources.tracks.every(state => state === 'ended') && resources.recorders.every(recorder => recorder.state === 'inactive' && recorder.listenersCleared) && resources.errors.length === 0; }

async function withBrowserPage(options, kind, run) {
  const timeouts = validateInput(options), artifact = artifactIdentity(options.artifactPath);
  const directory = makeDirectory(options.evidenceDir, kind === 'exports' ? 'exports' : kind === 'interactions' ? 'reader-interactions' : kind === 'explore' ? 'explore' : 'content');
  const report = { schemaVersion: 1, tool: 'phoenix-diagram', kind: kind === 'exports' ? 'real-browser-export' : kind === 'interactions' ? 'real-browser-reader-interactions' : kind === 'explore' ? 'real-browser-explore' : 'real-browser-content', status: 'not-run', artifact, browser: null, actualBrowserVersion: null, timeouts,
    scope: kind === 'exports' ? REQUIRED_EXPORT_SCOPE : kind === 'interactions' ? { readerInteractions: REQUIRED_READER_INTERACTIONS } : { content: true },
    exports: kind === 'exports' ? REQUIRED_EXPORT_SCOPE.themes.flatMap(theme => REQUIRED_EXPORT_SCOPE.modes.flatMap(mode => REQUIRED_EXPORT_SCOPE.formats.map(format => ({ theme, mode, format, status: 'not-run' })))) : [],
    clipboard: kind === 'exports' ? { success: { status: 'not-run' }, permissionDenied: { status: 'not-run' }, capabilityUnavailable: { status: 'not-run' } } : {},
    readerInteractions: { status: 'not-run', observations: REQUIRED_READER_INTERACTIONS.map(name => ({ name, status: 'not-run' })) },
    motionSuppression: kind === 'exports' ? { status: 'not-run', observations: REQUIRED_EXPORT_SCOPE.motionSuppression.themes.flatMap(theme => REQUIRED_EXPORT_SCOPE.motionSuppression.modes.flatMap(mode => REQUIRED_EXPORT_SCOPE.motionSuppression.conditions.map(condition => ({ theme, mode, condition, status: 'not-run' })))) } : undefined,
    stateRestoration: { status: 'not-run', observations: [], print: { status: 'not-run', method: REQUIRED_EXPORT_SCOPE.print.method, observations: [] } }, resources: { status: 'not-run', requests: [], loadingFailed: [], observations: [] },
    offline: { requested: options.offline === true, status: 'not-run', method: 'cdp-network-emulation', configured: false, remoteRequests: [] }, errors: [], receiptPath: path.join(directory, 'index.json') };
  let browser, cdp = options.cdp, contextId, targetId, sessionId, unsubscribe, stage = 'browser/discovery';
  const requests = [], loadingFailed = [], downloads = new Map(), dialogs = [];
  let loads = 0, frameId;
  try {
    if (!cdp) {
      const browserPath = discoverBrowser(options); browser = new ExportBrowser(browserPath, { timeouts });
      const version = await browser.ready; cdp = browser.cdp;
      report.browser = { ...version, executable: browserPath, node: process.version, transport: 'owned-debugging-pipe', sandbox: true };
    } else {
      report.browser = { ...await cdp.send('Browser.getVersion', {}, undefined, timeouts.startupMs), node: process.version, transport: 'injected-cdp' };
    }
    if (!/(?:Chrome|Chromium)\//.test(report.browser.product || '')) throw new Error('CDP endpoint did not identify a real Chrome/Chromium browser');
    report.actualBrowserVersion = report.browser.product;
    if (typeof cdp.subscribe !== 'function') throw new Error('CDP adapter must provide event subscription for actual download/network evidence');
    const send = (method, params = {}) => cdp.send(method, params, /^(?:Browser|Target)\./.test(method) ? undefined : sessionId, timeouts.commandMs);
    stage = 'browser/page';
    contextId = (await send('Target.createBrowserContext', { disposeOnDetach: true }, undefined)).browserContextId;
    targetId = (await send('Target.createTarget', { url: 'about:blank', browserContextId: contextId }, undefined)).targetId;
    sessionId = (await send('Target.attachToTarget', { targetId, flatten: true }, undefined)).sessionId;
    unsubscribe = cdp.subscribe(message => {
      const params = message.params || {};
      if (message.sessionId === sessionId) {
        if (message.method === 'Page.loadEventFired') loads++;
        if (message.method === 'Network.requestWillBeSent') requests.push({ url: params.request?.url || '', type: params.type, requestId: params.requestId });
        if (message.method === 'Network.loadingFailed') loadingFailed.push(params);
        if (message.method === 'Page.javascriptDialogOpening') {
          dialogs.push({ type: params.type, message: params.message });
          send('Page.handleJavaScriptDialog', { accept: true }).catch(error => report.errors.push(errorRecord(error, 'browser/dialog')));
        }
      }
      if (message.method === 'Browser.downloadWillBegin' && params.frameId === frameId) downloads.set(params.guid, { ...params, state: 'pending' });
      if (message.method === 'Browser.downloadProgress' && downloads.has(params.guid)) Object.assign(downloads.get(params.guid), params);
    });
    await send('Page.enable'); await send('Runtime.enable');
    await configureNetwork(send);
    report.offline.configured = true;
    await send('Emulation.setDeviceMetricsOverride', { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false });
    await send('Emulation.setFocusEmulationEnabled', { enabled: true });
    await send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: 'no-preference' }] });
    const downloadDirectory = path.join(directory, 'downloads'); fs.mkdirSync(downloadDirectory, { mode: 0o700 });
    await send('Browser.setDownloadBehavior', { behavior: 'allowAndName', downloadPath: downloadDirectory, eventsEnabled: true, browserContextId: contextId }, undefined);
    const evaluate = async (fn, argument) => {
      const expression = '(' + fn.toString() + ')(' + (argument === undefined ? '' : JSON.stringify(argument)) + ')';
      const response = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true, userGesture: false });
      if (response.exceptionDetails) throw new Error(response.exceptionDetails.exception?.description || response.exceptionDetails.text || 'Export page evaluation failed');
      return response.result?.value;
    };
    let startup;
    const page = {
      report, directory, artifact, evaluate, send, dialogs, downloads, contextId, targetId, timeouts,
      async load(theme = 'dark', suppressClipboard = false, { query = {}, hash = '' } = {}) {
        if (startup) await send('Page.removeScriptToEvaluateOnNewDocument', { identifier: startup });
        startup = (await send('Page.addScriptToEvaluateOnNewDocument', { source: '(' + installExportProbe.toString() + ')(' + JSON.stringify({ suppressClipboard }) + ');' })).identifier;
        const url = new URL(pathToFileURL(artifact.path)); url.searchParams.set('theme', theme);
        for (const [name, value] of Object.entries(query)) url.searchParams.set(name, value);
        url.hash = hash;
        if (hash || Object.keys(query).length) {
          // A hash-only change is a same-document navigation without a load event.
          const blank = loads; await send('Page.navigate', { url: 'about:blank' });
          await waitFor(() => loads > blank, timeouts.commandMs, 'Blank page load');
        }
        const before = loads, navigation = await send('Page.navigate', { url: url.href });
        if (navigation.errorText) throw new Error('Export page navigation: ' + navigation.errorText);
        frameId = navigation.frameId;
        await waitFor(() => loads > before, timeouts.commandMs, 'Page load');
        await evaluate(awaitReader);
        const frames = await send('Page.getFrameTree'); frameId = frames.frameTree.frame.id;
        return evaluate(snapshotReader);
      },
      async click(selector) {
        const point = await evaluate(readerClickPoint, selector);
        await send('Input.dispatchMouseEvent', { type: 'mousePressed', ...point, button: 'left', clickCount: 1 });
        await send('Input.dispatchMouseEvent', { type: 'mouseReleased', ...point, button: 'left', clickCount: 1 });
      },
      async download(format, basename, expected) {
        const capability = await evaluate(format => { const item = document.querySelector('button[data-format="' + format + '"]'); return { available: !!item && !item.disabled, reason: item?.title || '' }; }, format);
        if (!capability.available) return { format, status: 'unavailable', reason: capability.reason };
        const beforeCount = await evaluate(() => __phoenixDiagramExportProbe.downloads.length);
        await evaluate(() => PhoenixDiagram.exportMenu.open()); await page.click('button[data-format="' + format + '"]');
        const captured = await waitFor(() => evaluate(before => {
          const state = __phoenixDiagramExportProbe, download = state.downloads[before];
          if (download) return { url: download.url, name: download.name, bytes: download.blob?.size, mime: download.blob?.type };
          const error = document.documentElement.getAttribute('data-last-export-error');
          if (error) throw new Error(error); return null;
        }, beforeCount), timeouts.actionMs, format + ' download');
        const completed = await waitFor(() => {
          const entry = [...downloads.values()].find(value => value.url === captured.url);
          if (entry?.state === 'canceled') throw new Error('Chrome canceled ' + format + ' download');
          return entry?.state === 'completed' ? entry : null;
        }, timeouts.actionMs, format + ' download completion');
        if (!/^[a-zA-Z0-9-]+$/.test(completed.guid)) throw new Error('Unsafe Chrome download identifier');
        const file = path.join(downloadDirectory, completed.guid);
        await waitFor(() => fs.existsSync(file), timeouts.commandMs, 'Downloaded file');
        const stat = fs.lstatSync(file); if (!stat.isFile() || stat.isSymbolicLink() || stat.nlink !== 1) throw new Error('Download is not an owned ordinary file');
        const bytes = fs.readFileSync(file), signature = verifyFileSignature(bytes, format, captured.mime);
        const decoded = await evaluate(inspectBlob, { url: captured.url, kind: format, expected, decodeMs: timeouts.decodeMs });
        if (format === 'svg') { verifyCanonicalFacts(decoded.canonical?.facts, expected); delete decoded.canonical.facts; }
        if (decoded.sha256 !== sha256(bytes) || decoded.bytes !== bytes.length) throw new Error('Actual downloaded file differs from the encoded Blob');
        const output = path.join(directory, basename + '.' + format); fs.writeFileSync(output, bytes, { flag: 'wx', mode: 0o600 });
        const receipt = await evaluate(() => Object.fromEntries(Array.from(document.documentElement.attributes).filter(attribute => attribute.name.startsWith('data-last-export-')).map(attribute => [attribute.name, attribute.value])));
        if (receipt['data-last-export-format'] !== format || receipt['data-last-export-canonical'] !== 'true') throw new Error('Reader did not report a canonical actual export');
        return { format, status: 'pass', path: output, identity: { sha256: sha256(bytes), bytes: bytes.length }, signature, download: { guid: completed.guid, name: captured.name }, ...decoded, receipt };
      },
      // A Route/Reach share card: a scoped, non-canonical PNG from the export menu.
      async card(action, basename) {
        const beforeCount = await evaluate(() => __phoenixDiagramExportProbe.downloads.length);
        await evaluate(() => PhoenixDiagram.exportMenu.open()); await page.click('button[data-action="' + action + '"]');
        const captured = await waitFor(() => evaluate(before => {
          const state = __phoenixDiagramExportProbe, download = state.downloads[before];
          if (download) return { url: download.url, name: download.name, bytes: download.blob?.size, mime: download.blob?.type };
          const error = document.documentElement.getAttribute('data-last-export-error');
          if (error) throw new Error(error); return null;
        }, beforeCount), timeouts.actionMs, action + ' download');
        const completed = await waitFor(() => {
          const entry = [...downloads.values()].find(value => value.url === captured.url);
          if (entry?.state === 'canceled') throw new Error('Chrome canceled ' + action);
          return entry?.state === 'completed' ? entry : null;
        }, timeouts.actionMs, action + ' download completion');
        if (!/^[a-zA-Z0-9-]+$/.test(completed.guid)) throw new Error('Unsafe Chrome download identifier');
        const file = path.join(downloadDirectory, completed.guid);
        await waitFor(() => fs.existsSync(file), timeouts.commandMs, 'Downloaded card');
        const bytes = fs.readFileSync(file), signature = verifyFileSignature(bytes, 'png', captured.mime);
        const decoded = await evaluate(inspectBlob, { url: captured.url, kind: 'png', expected: {}, decodeMs: timeouts.decodeMs });
        if (decoded.sha256 !== sha256(bytes) || decoded.bytes !== bytes.length) throw new Error('Actual downloaded card differs from the encoded Blob');
        const output = path.join(directory, basename + '.png'); fs.writeFileSync(output, bytes, { flag: 'wx', mode: 0o600 });
        const receipt = await evaluate(() => Object.fromEntries(Array.from(document.documentElement.attributes).filter(attribute => attribute.name.startsWith('data-last-export-')).map(attribute => [attribute.name, attribute.value])));
        return { action, status: 'pass', path: output, identity: { sha256: sha256(bytes), bytes: bytes.length }, signature, download: { guid: completed.guid, name: captured.name }, ...decoded, receipt };
      },
      async resources(label) {
        let last;
        try {
          return await waitFor(async () => { last = await evaluate(readResources); return resourceClean(last) ? last : null; }, timeouts.commandMs, 'Export resource cleanup');
        } finally { if (last) report.resources.observations.push({ label, clean: resourceClean(last), ...last }); }
      },
    };
    stage = kind === 'exports' ? 'exports' : kind === 'interactions' ? 'reader-interactions' : kind === 'explore' ? 'explore' : 'content';
    await run(page);
    report.resources.requests = requests;
    report.resources.loadingFailed = loadingFailed;
    const remote = requests.filter(request => /^(?:https?|wss?):/i.test(request.url));
    report.offline.remoteRequests = remote;
    if (remote.length) throw new Error('Reader attempted remote requests: ' + remote.map(request => request.url).join(', '));
    if (loadingFailed.length) throw new Error('Reader resources failed to load: ' + loadingFailed.map(entry => entry.requestId + ':' + entry.errorText).join(', '));
    if (report.offline.requested) report.offline.status = 'pass';
    report.resources.status = 'pass';
    report.status = 'pass';
  } catch (error) {
    report.status = /(?:^|\/)unavailable$/.test(error.code || '') ? 'unavailable' : 'failed'; report.errors.push(errorRecord(error, stage)); report.resources.requests = requests;
    report.resources.loadingFailed = loadingFailed;
    report.offline.remoteRequests = requests.filter(request => /^(?:https?|wss?):/i.test(request.url));
    if (report.offline.requested && report.offline.configured) report.offline.status = 'failed';
  } finally {
    unsubscribe?.();
    try { if (targetId) await cdp.send('Target.closeTarget', { targetId }, undefined, timeouts.commandMs); }
    catch (error) { report.errors.push(errorRecord(error, 'browser/target-cleanup')); report.resources.status = 'failed'; report.status = 'failed'; }
    try { if (contextId) await cdp.send('Target.disposeBrowserContext', { browserContextId: contextId }, undefined, timeouts.commandMs); report.resources.contextClosed = !!contextId; }
    catch (error) { report.errors.push(errorRecord(error, 'browser/context-cleanup')); report.resources.status = 'failed'; report.status = 'failed'; }
    report.resources.browserOwned = !!browser;
    try { if (browser) await browser.close(); report.resources.browserClosed = browser ? true : null; }
    catch (error) { report.errors.push(errorRecord(error, 'browser/process-cleanup')); report.resources.browserClosed = false; report.resources.status = 'failed'; report.status = 'failed'; }
    try { if (JSON.stringify(artifactIdentity(artifact.path)) !== JSON.stringify(artifact)) throw new Error('Artifact identity changed during export acceptance'); }
    catch (error) { report.errors.push(errorRecord(error, 'artifact/binding')); report.status = 'failed'; }
    fs.writeFileSync(report.receiptPath, JSON.stringify(report, null, 2) + '\n', { flag: 'wx', mode: 0o600 });
  }
  return report;
}

async function permission(page, setting) {
  for (const name of ['clipboard-read', 'clipboard-write']) await page.send('Browser.setPermission', { permission: { name }, setting, browserContextId: page.contextId }, undefined);
}

async function runMotionSuppression(page, onProgress) {
  const { report, evaluate, send } = page, scope = REQUIRED_EXPORT_SCOPE.motionSuppression;
  const observe = async () => evaluate(motionSuppressionSnapshot, await evaluate(snapshotReader));
  report.motionSuppression.status = 'incomplete';
  for (const sample of report.motionSuppression.observations) {
    const { theme, mode, condition } = sample;
    onProgress?.({ stage: 'motion-suppression', theme, mode, condition });
    sample.status = 'incomplete'; sample.method = scope.methods[condition];
    await send('Target.activateTarget', { targetId: page.targetId });
    await page.load(theme);
    const inputStart = await evaluate(() => __phoenixDiagramExportProbe.readerInputs.length);
    // Every sample records an explicit native selection, including Still. The
    // fresh-reader default is checked earlier, before any saved preference.
    if ((await evaluate(snapshotReader)).mode === mode) await page.click('#btn-motion');
    await page.click('#btn-motion');
    await evaluate(awaitReader);
    sample.before = await observe();
    sample.inputs = await evaluate(start => __phoenixDiagramExportProbe.readerInputs.slice(start), inputStart);
    sample.motionApplicable = mode === 'live' && ((sample.before.capable && sample.before.reader.eligibleMotion) || sample.before.animationCount > 0);
    const eventStart = await evaluate(() => __phoenixDiagramExportProbe.motionEvents.length);
    let coverTargetId;
    try {
      if (condition === 'reduced-motion') {
        // This proves the browser media-query branch, not a host OS setting.
        await send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: 'reduce' }] });
      } else {
        await send('Emulation.setFocusEmulationEnabled', { enabled: false });
        coverTargetId = (await send('Target.createTarget', { url: 'about:blank', browserContextId: page.contextId, background: false })).targetId;
        sample.ownedTargets = { subject: page.targetId, cover: coverTargetId, browserContext: page.contextId };
        await send('Target.activateTarget', { targetId: coverTargetId });
      }
      await waitFor(async () => {
        // Wait for delivery before reading governor.mode(): its live branch
        // itself reads MediaQueryList.matches, ahead of the queued change.
        const nativeEvent = await evaluate(({ start, condition }) => __phoenixDiagramExportProbe.motionEvents.slice(start).some(event => event.trusted === true && (condition === 'reduced-motion' ? event.type === 'reduced-motion' && event.matches === true : event.type === 'visibilitychange' && event.hidden === true && event.visibilityState === 'hidden')), { start: eventStart, condition });
        if (!nativeEvent) return false;
        sample.during = await observe();
        const state = sample.during, suppressed = condition === 'reduced-motion' ? state.reducedMotion === true && state.button.disabled === true : state.hidden === true && state.visibilityState === 'hidden' && state.documentHidden === 'true';
        return suppressed && state.reader.mode === 'still' && state.running === 0;
      }, page.timeouts.actionMs, 'Actual ' + condition + ' suppression');
    } finally {
      if (condition === 'reduced-motion') {
        await send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: 'no-preference' }] });
      } else {
        try {
          await send('Target.activateTarget', { targetId: page.targetId });
          // Never manufacture visibility with a property override, dispatched
          // event or focus emulation. It must return before re-enabling focus.
          await waitFor(() => evaluate(() => !document.hidden && document.visibilityState === 'visible'), page.timeouts.actionMs, 'Actual visible document restoration');
        } finally {
          try { if (coverTargetId) await send('Target.closeTarget', { targetId: coverTargetId }); }
          finally { await send('Emulation.setFocusEmulationEnabled', { enabled: true }); }
        }
      }
      await evaluate(awaitReader);
      sample.after = await observe();
      sample.events = await evaluate(start => __phoenixDiagramExportProbe.motionEvents.slice(start), eventStart);
    }
    sample.status = 'pass';
    try { verifyMotionSuppressionSample(sample); } catch (error) { sample.status = 'incomplete'; throw error; }
    await page.resources(`${theme}-${mode}-${condition}`);
  }
  report.motionSuppression.status = 'pass';
  verifyMotionSuppression(report.motionSuppression);
}

export async function runExportAcceptance(options) {
  return withBrowserPage(options, 'exports', async page => {
    const { report, evaluate } = page;
    await runReaderInteractions(page, options.onProgress);
    let first = true;
    for (const theme of REQUIRED_EXPORT_SCOPE.themes) {
      const initial = await page.load(theme);
      if (first && initial.mode !== 'still') throw new Error('A new Reader did not start in Still mode'); first = false;
      for (const mode of REQUIRED_EXPORT_SCOPE.modes) {
        if ((await evaluate(snapshotReader)).mode !== mode) await page.click('#btn-motion');
        await evaluate(() => {
          const first = document.querySelector('.diagram-container > svg [data-node-id]')?.getAttribute('data-node-id');
          if (first) PhoenixDiagram.focus.set(first, { toggle: false, updateUrl: false });
          PhoenixDiagram.view.zoomIn();
        });
        await evaluate(awaitReader); const expected = await evaluate(snapshotReader);
        if (expected.mode !== mode) throw new Error('Motion mode did not match the requested user action');
        for (const format of REQUIRED_EXPORT_SCOPE.formats) {
          options.onProgress?.({ stage: 'export', theme, mode, format });
          const exported = await page.download(format, `${theme}-${mode}-${format}`, expected);
          Object.assign(report.exports.find(scope => scope.theme === theme && scope.mode === mode && scope.format === format), exported);
          const after = await evaluate(snapshotReader), restored = equalState(expected, after);
          report.stateRestoration.observations.push({ theme, mode, format, restored });
          if (!restored) throw new Error('Export changed the live Reader exploration/motion state');
        }
        await page.resources(`${theme}-${mode}`);
      }
    }
    report.stateRestoration.print.status = 'failed';
    for (const theme of REQUIRED_EXPORT_SCOPE.print.themes) {
      await page.load(theme);
      for (const mode of REQUIRED_EXPORT_SCOPE.print.modes) {
        options.onProgress?.({ stage: 'print', theme, mode });
        if ((await evaluate(snapshotReader)).mode !== mode) await page.click('#btn-motion');
        await evaluate(awaitReader);
        const before = await evaluate(snapshotReader), motionBefore = await evaluate(printMotionSnapshot);
        const sample = { theme, mode, motionApplicable: mode === 'live' && motionBefore.capable && before.eligibleMotion, before, motionBefore };
        report.stateRestoration.print.observations.push(sample);
        try {
          await page.send('Emulation.setEmulatedMedia', { media: 'print', features: [{ name: 'prefers-reduced-motion', value: 'no-preference' }] });
          await evaluate(awaitReader);
          sample.during = await evaluate(snapshotReader); sample.motionDuring = await evaluate(printMotionSnapshot);
          const count = await evaluate(() => __phoenixDiagramExportProbe.printEvents.length);
          const printed = await page.send('Page.printToPDF', { printBackground: true, preferCSSPageSize: true });
          const bytes = Buffer.from(printed.data || '', 'base64'), file = path.join(page.directory, `${theme}-${mode}-print.pdf`);
          fs.writeFileSync(file, bytes, { flag: 'wx', mode: 0o600 });
          sample.nativePrint = { events: await evaluate(count => __phoenixDiagramExportProbe.printEvents.slice(count), count), pdf: { path: file, identity: { sha256: sha256(bytes), bytes: bytes.length }, signature: bytes.subarray(0, 5).toString() === '%PDF-' ? 'PDF' : 'invalid', eof: bytes.subarray(-64).includes(Buffer.from('%%EOF')), pages: (bytes.toString('latin1').match(/\/Type\s*\/Page\b/g) || []).length } };
        } finally {
          await page.send('Emulation.setEmulatedMedia', { media: '', features: [{ name: 'prefers-reduced-motion', value: 'no-preference' }] });
          await evaluate(awaitReader);
          sample.after = await evaluate(snapshotReader); sample.motionAfter = await evaluate(printMotionSnapshot);
        }
        verifyPrintSample(sample); sample.status = 'pass';
        await page.resources(`${theme}-${mode}-print`);
      }
    }
    report.stateRestoration.print.status = 'pass';
    await runMotionSuppression(page, options.onProgress);
    await page.load('dark');
    await permission(page, 'granted');
    const clipboardState = await evaluate(snapshotReader);
    const capability = (await evaluate(snapshotReader)).clipboard;
    if (!(capability.item && capability.write && capability.read)) { const error = new Error('Native clipboard success/read capability is unavailable'); error.code = 'acceptance/clipboard/unavailable'; throw error; }
    await evaluate(() => PhoenixDiagram.exportMenu.open());
    await page.click('button[data-action="copy"]');
    const success = await waitFor(() => evaluate(() => {
      const state = __phoenixDiagramExportProbe, write = state.writes[state.writes.length - 1];
      if (write?.state === 'rejected') throw new Error('Native PNG clipboard write rejected: ' + write.error);
      return write?.state === 'resolved' ? { write, constructions: state.constructions.slice() } : null;
    }), page.timeouts.actionMs, 'Native PNG clipboard write');
    if (!success.constructions[0]?.gesture || !success.constructions[0]?.inHandler || !success.constructions[0]?.promise) throw new Error('ClipboardItem was not constructed synchronously in the native click');
    const clipboardBlob = await evaluate(async () => {
      const items = await navigator.clipboard.read();
      const item = items.find(item => item.types.includes('image/png')); if (!item) throw new Error('Native clipboard contains no PNG');
      const blob = await item.getType('image/png'); const url = URL.createObjectURL(blob); return { url };
    });
    let clipboard;
    try { clipboard = await evaluate(inspectBlob, { url: clipboardBlob.url, kind: 'png', expected: await evaluate(snapshotReader), returnBase64: true, decodeMs: page.timeouts.decodeMs }); }
    finally { await evaluate(url => URL.revokeObjectURL(url), clipboardBlob.url); }
    const clipboardBytes = Buffer.from(clipboard.base64, 'base64'); delete clipboard.base64;
    const signature = verifyFileSignature(clipboardBytes, 'png', clipboard.mime), file = path.join(page.directory, 'clipboard.png');
    if (sha256(clipboardBytes) !== clipboard.sha256) throw new Error('Clipboard bytes changed during transfer');
    fs.writeFileSync(file, clipboardBytes, { flag: 'wx', mode: 0o600 });
    report.clipboard.success = { status: 'pass', path: file, ...clipboard, signature, gesture: success.constructions[0] };
    await permission(page, 'denied'); const priorDialogs = page.dialogs.length;
    await evaluate(() => PhoenixDiagram.exportMenu.open()); await page.click('button[data-action="copy"]');
    const denied = await waitFor(() => evaluate(() => { const write = __phoenixDiagramExportProbe.writes.at(-1); return write?.state === 'rejected' ? write : null; }), page.timeouts.actionMs, 'Native clipboard permission rejection');
    if (!['NotAllowedError', 'SecurityError'].includes(denied.error)) throw new Error('Clipboard rejected for a reason other than the denied native permission: ' + denied.error);
    await waitFor(() => page.dialogs.length > priorDialogs, page.timeouts.actionMs, 'Clipboard denial feedback');
    const hint = await evaluate(() => viewerText('viewer.export.downloadPngInstead'));
    if (hint === 'viewer.export.downloadPngInstead' || !page.dialogs.at(-1).message.includes(hint)) throw new Error('Clipboard denial did not identify the existing PNG download path');
    report.clipboard.permissionDenied = { status: 'pass', injected: false, error: denied.error, feedback: page.dialogs.at(-1).message };
    const clipboardRestored = equalState(clipboardState, await evaluate(snapshotReader));
    report.stateRestoration.observations.push({ label: 'clipboard-native', restored: clipboardRestored });
    if (!clipboardRestored) throw new Error('Clipboard action changed live Reader state');
    await page.resources('clipboard-native');
    await page.load('dark', true);
    const unavailable = await evaluate(() => { const copy = document.querySelector('button[data-action="copy"]'), png = document.querySelector('button[data-format="png"]'); return { disabled: copy.disabled, title: copy.title, pngAvailable: !png.disabled, nativeCapabilityBeforeSuppression: __phoenixDiagramExportProbe.originalClipboardCapability, hint: viewerText('viewer.export.downloadPngInstead') }; });
    if (!unavailable.disabled || !unavailable.pngAvailable || !unavailable.title.includes(unavailable.hint)) throw new Error('Capability-unavailable UI lost its PNG fallback');
    const fallback = await page.download('png', 'clipboard-unavailable-fallback', await evaluate(snapshotReader));
    if (fallback.status !== 'pass') throw new Error('Clipboard-unavailable PNG fallback did not export');
    report.clipboard.capabilityUnavailable = { status: 'pass', injected: true, boundary: 'ClipboardItem capability suppression only; all PNG encoding/download/decoding remained native', ...unavailable, fallback };
    await page.resources('clipboard-unavailable');
    if (report.exports.length !== 20 || report.exports.some(result => result.status !== 'pass')) { const error = new Error('Required real export scope was incomplete or unavailable'); error.code = 'acceptance/format/unavailable'; throw error; }
    report.stateRestoration.status = 'pass';
  });
}

export async function runReaderInteractionAcceptance(options) {
  return withBrowserPage(options, 'interactions', async page => {
    await runReaderInteractions(page, options.onProgress);
    page.report.stateRestoration = { status: 'not-applicable', observations: [] };
  });
}

// Exploration plus, for diagrams with authored relations, one Route and one Reach card.
export async function runExploreAcceptance(options) {
  return withBrowserPage(options, 'explore', async page => {
    await page.load('dark');
    const metadata = await page.evaluate(() => JSON.parse(document.getElementById('phoenix-diagram-metadata').textContent));
    const exploration = await runExplore(page, metadata, options.onProgress);
    verifyExplore(exploration);
    page.report.explore = exploration;
    page.report.cards = [];
    const plan = exploration.plan;
    if (plan.pathPair && plan.reachNode) {
      await page.load('dark', false, { hash: 'route=' + plan.pathPair.join('~') });
      page.report.cards.push(await page.card('route-share-card', 'route-share-card'));
      await page.load('dark', false, { hash: 'focus=' + plan.reachNode + '&reach=downstream' });
      page.report.cards.push(await page.card('reach-share-card', 'downstream-reach-share-card'));
      for (const [card, variant, suffix] of [[page.report.cards[0], 'route', '-route-share-card.png'], [page.report.cards[1], 'reach', '-downstream-reach-share-card.png']]) {
        if (card.receipt['data-last-export-canonical'] !== 'false' || card.receipt['data-last-export-variant'] !== variant || !card.download.name.endsWith(suffix)) throw new Error('Share card is not a scoped non-canonical ' + variant + ' export');
      }
    }
    page.report.stateRestoration = { status: 'not-applicable', observations: [] };
  });
}

// The first enabled Reach control of the open details takes a trusted tap on a
// touch screen, or a click. A panel taller than its room is first scrolled by a
// real gesture; the control's center must hit the control, not the navigation.
export async function operateReach(page, id, touch) {
  const locate = () => page.evaluate(() => {
    const section = document.getElementById('focus-reach'), button = ['downstream', 'upstream'].map(direction => document.getElementById('btn-reach-' + direction)).find(button => !button.disabled);
    if (section.hidden || !button) return null;
    const box = button.getBoundingClientRect(), panel = document.getElementById('focus-chip').getBoundingClientRect();
    const x = box.left + box.width / 2, y = box.top + box.height / 2, hit = document.elementFromPoint(x, y);
    return { direction: button.id.slice('btn-reach-'.length), x, y, inPanel: y > panel.top && y < panel.bottom, onControl: Boolean(hit && button.contains(hit)), hit: hit ? hit.id || hit.localName : null, from: { x: Math.round(panel.left + panel.width / 2), y: Math.round(panel.top + 24) } };
  });
  let control = await locate(), scrolled = false;
  if (!control) return { status: 'not-applicable' };
  if (!control.inPanel) {
    await page.send('Input.synthesizeScrollGesture', { ...control.from, yDistance: -Math.ceil(control.y - control.from.y), gestureSourceType: touch ? 'touch' : 'mouse', speed: 1200 });
    control = await locate(); scrolled = true;
  }
  if (!control.inPanel || !control.onControl) throw new Error('Reach ' + control.direction + ' of ' + id + ' cannot be operated; its center hits ' + control.hit);
  const point = { x: control.x, y: control.y };
  if (touch) {
    await page.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ ...point, radiusX: 1, radiusY: 1, force: 1, id: 1 }] });
    await page.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  } else {
    await page.send('Input.dispatchMouseEvent', { type: 'mousePressed', ...point, button: 'left', buttons: 1, clickCount: 1 });
    await page.send('Input.dispatchMouseEvent', { type: 'mouseReleased', ...point, button: 'left', buttons: 0, clickCount: 1 });
  }
  const reach = await waitFor(() => page.evaluate(direction => { const reach = PhoenixDiagram.focus.reachability(); return reach && reach.direction === direction ? reach : null; }, control.direction), 2000, 'Trusted ' + (touch ? 'tap' : 'click') + ' on Reach ' + control.direction + ' of ' + id);
  if (reach.originId !== id) throw new Error('Reach ' + control.direction + ' started from ' + reach.originId + ' instead of ' + id);
  return { status: 'pass', direction: control.direction, input: touch ? 'touch' : 'mouse', scrolled, nodeIds: reach.nodeIds };
}

// Focus each requested node and capture its details panel for actual viewing,
// with the source evidence state the Reader displays, then operate its Reach.
// `evidence: false` accepts details without source evidence; a node may name
// its own viewport, and a narrow one (720px or less) is a touch screen.
export async function runPassportAcceptance(options) {
  return withBrowserPage(options, 'explore', async page => {
    page.report.passports = [];
    let current = '1440x900';
    for (const { id, theme = 'dark', viewport = { width: 1440, height: 900 } } of options.nodes) {
      const size = viewport.width + 'x' + viewport.height, touch = viewport.width <= 720;
      if (size !== current) {
        await page.send('Emulation.setDeviceMetricsOverride', { width: viewport.width, height: viewport.height, deviceScaleFactor: 1, mobile: touch });
        await page.send('Emulation.setTouchEmulationEnabled', touch ? { enabled: true, maxTouchPoints: 1 } : { enabled: false });
        current = size;
      }
      await page.load(theme, false, { hash: 'focus=' + id });
      await delay(400);
      const state = await page.evaluate(id => {
        const evidence = document.getElementById('focus-evidence'), rows = Array.from(document.querySelectorAll('#focus-evidence-links > *'));
        return { focus: PhoenixDiagram.focus.active(), detailsHidden: document.getElementById('focus-chip').hidden, hidden: evidence.hidden,
          state: evidence.getAttribute('data-evidence-state'), status: evidence.querySelector('.semantic-passport-evidence-status').textContent,
          repository: document.getElementById('focus-repository').textContent, repositoryHref: document.getElementById('focus-repository').getAttribute('href'),
          rows: rows.map(row => ({ tag: row.localName, state: row.getAttribute('data-evidence-state'), href: row.getAttribute('href'), text: row.textContent })), label: document.querySelector('[data-node-id="' + id + '"]').getAttribute('aria-label'),
          context: document.getElementById('focus-context').textContent,
          // Title, detail, metadata, status, repository, path and pending reason must be readable in full, not ellipsized.
          clipped: Array.from(document.querySelectorAll('#relationship-lens-title, #focus-detail, #focus-passport-meta > *, #focus-evidence .semantic-passport-evidence-status, #focus-evidence .semantic-passport-repository, #focus-evidence .semantic-passport-source strong, #focus-evidence .semantic-passport-source small')).filter(element => element.scrollWidth > element.clientWidth + 1).map(element => element.textContent) };
      }, id);
      if (state.focus !== id || state.detailsHidden || (state.hidden && options.evidence !== false)) throw new Error('Details did not show' + (options.evidence === false ? '' : ' source evidence') + ' for ' + id);
      if (state.clipped.length) throw new Error('Details text is clipped for ' + id + ': ' + state.clipped.join(' | '));
      const { data } = await page.send('Page.captureScreenshot', { format: 'png' });
      const bytes = Buffer.from(data, 'base64'), file = path.join(page.directory, `passport-${id}-${theme}${size === '1440x900' ? '' : '-' + size}.png`);
      fs.writeFileSync(file, bytes, { flag: 'wx', mode: 0o600 });
      const reach = await operateReach(page, id, touch);
      page.report.passports.push({ id, theme, viewport, ...state, reach, screenshot: { path: file, identity: { sha256: sha256(bytes), bytes: bytes.length } } });
    }
    page.report.stateRestoration = { status: 'not-applicable', observations: [] };
  });
}

export async function runContentAcceptance(options) {
  if (typeof options?.expectedLiteral !== 'string' || !options.expectedLiteral.length) throw new Error('Content acceptance requires the authored expectedLiteral');
  return withBrowserPage(options, 'content', async page => {
    await page.load('dark');
    const result = await page.evaluate(contentObservation, { expectedLiteral: options.expectedLiteral, sentinelName: options.sentinelName });
    const passed = !result.sentinelDefined && result.literalPresent && result.errors.length === 0;
    page.report.content = { status: passed ? 'pass' : 'failed', ...result };
    if (!passed) throw new Error('HTML content/sentinel acceptance failed');
    await page.resources('content');
    page.report.stateRestoration = { status: 'not-applicable', observations: [] };
  });
}
