// Real-browser maintenance controller. No renderer/consumer runtime dependency.
// Browser probes derive from frozen Archify 3.1.0 export tests (MIT); see
// ../viewer/LICENSE.txt and support/export-browser.mjs for the transport source.
import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { pathToFileURL } from 'node:url';
import { setTimeout as delay } from 'node:timers/promises';
import { DEFAULT_TIMEOUTS, discoverBrowser, ExportBrowser } from './support/export-browser.mjs';
import { installExportProbe, awaitReader, snapshotReader, printMotionSnapshot, inspectBlob, readResources, contentObservation } from './support/export-page.mjs';
import { REQUIRED_READER_INTERACTIONS, runReaderInteractions } from './support/export-reader.mjs';

export const REQUIRED_EXPORT_SCOPE = Object.freeze({ themes: ['dark', 'light'], modes: ['still', 'live'], formats: ['svg', 'png', 'jpeg', 'webp', 'webm'], clipboard: ['success', 'permission-denied', 'capability-unavailable'], print: { themes: ['dark', 'light'], modes: ['still', 'live'], method: 'cdp-print-media-and-native-print-to-pdf' }, readerInteractions: REQUIRED_READER_INTERACTIONS });
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
  const directory = makeDirectory(options.evidenceDir, kind === 'exports' ? 'exports' : kind === 'interactions' ? 'reader-interactions' : 'content');
  const report = { schemaVersion: 1, tool: 'phoenix-diagram', kind: kind === 'exports' ? 'real-browser-export' : kind === 'interactions' ? 'real-browser-reader-interactions' : 'real-browser-content', status: 'not-run', artifact, browser: null, actualBrowserVersion: null, timeouts,
    scope: kind === 'exports' ? REQUIRED_EXPORT_SCOPE : kind === 'interactions' ? { readerInteractions: REQUIRED_READER_INTERACTIONS } : { content: true },
    exports: kind === 'exports' ? REQUIRED_EXPORT_SCOPE.themes.flatMap(theme => REQUIRED_EXPORT_SCOPE.modes.flatMap(mode => REQUIRED_EXPORT_SCOPE.formats.map(format => ({ theme, mode, format, status: 'not-run' })))) : [],
    clipboard: kind === 'exports' ? { success: { status: 'not-run' }, permissionDenied: { status: 'not-run' }, capabilityUnavailable: { status: 'not-run' } } : {},
    readerInteractions: { status: 'not-run', observations: REQUIRED_READER_INTERACTIONS.map(name => ({ name, status: 'not-run' })) },
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
      report, directory, artifact, evaluate, send, dialogs, downloads, contextId, timeouts,
      async load(theme = 'dark', suppressClipboard = false) {
        if (startup) await send('Page.removeScriptToEvaluateOnNewDocument', { identifier: startup });
        startup = (await send('Page.addScriptToEvaluateOnNewDocument', { source: '(' + installExportProbe.toString() + ')(' + JSON.stringify({ suppressClipboard }) + ');' })).identifier;
        const url = new URL(pathToFileURL(artifact.path)); url.searchParams.set('theme', theme);
        const before = loads, navigation = await send('Page.navigate', { url: url.href });
        if (navigation.errorText) throw new Error('Export page navigation: ' + navigation.errorText);
        frameId = navigation.frameId;
        await waitFor(() => loads > before, timeouts.commandMs, 'Page load');
        await evaluate(awaitReader);
        const frames = await send('Page.getFrameTree'); frameId = frames.frameTree.frame.id;
        return evaluate(snapshotReader);
      },
      async click(selector) {
        const point = await evaluate(selector => {
          const node = document.querySelector(selector);
          if (!node || node.disabled || node.hidden) throw new Error('Unavailable control: ' + selector);
          node.scrollIntoView({ block: 'center', inline: 'center' });
          const rect = node.getBoundingClientRect();
          if (!(rect.width > 0 && rect.height > 0)) throw new Error('Control is not painted: ' + selector);
          return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
        }, selector);
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
      async resources(label) {
        let last;
        try {
          return await waitFor(async () => { last = await evaluate(readResources); return resourceClean(last) ? last : null; }, timeouts.commandMs, 'Export resource cleanup');
        } finally { if (last) report.resources.observations.push({ label, clean: resourceClean(last), ...last }); }
      },
    };
    stage = kind === 'exports' ? 'exports' : kind === 'interactions' ? 'reader-interactions' : 'content';
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
