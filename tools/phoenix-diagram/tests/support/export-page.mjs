// Fixed page probes adapted from frozen Archify 3.1.0 export browser tests.
// MIT: ../../viewer/LICENSE.txt. Phoenix changes: actual download/clipboard
// delegates, independent namespace, canonical facts and real decoder evidence.

export function installExportProbe({ suppressClipboard = false } = {}) {
  const state = { urls: new Map(), downloads: [], errors: [], writes: [], constructions: [], tracks: [], recorders: [], printEvents: [], motionEvents: [], readerInputs: [], inCopyHandler: false, suppressClipboard };
  window.__phoenixDiagramExportProbe = state;
  addEventListener('error', event => state.errors.push(String(event.message)));
  addEventListener('unhandledrejection', event => state.errors.push(String(event.reason)));
  for (const type of ['beforeprint', 'afterprint']) addEventListener(type, () => state.printEvents.push({ type }));
  matchMedia('(prefers-reduced-motion: reduce)').addEventListener('change', event => state.motionEvents.push({ type: 'reduced-motion', trusted: event.isTrusted, matches: event.matches }));
  document.addEventListener('visibilitychange', event => state.motionEvents.push({ type: 'visibilitychange', trusted: event.isTrusted, hidden: document.hidden, visibilityState: document.visibilityState }));
  for (const type of ['click', 'keydown', 'input', 'pointerdown', 'pointermove', 'pointerup']) document.addEventListener(type, event => {
    if (type === 'pointermove' && !(event.buttons & 1)) return;
    const element = event.target instanceof Element ? event.target : document.activeElement;
    const target = element?.closest('button,input,[data-node-id],[data-view],[data-preset-value]') || element;
    state.readerInputs.push({ type, trusted: event.isTrusted, key: event.key || '', target: { id: target?.id || '', view: target?.getAttribute('data-view') ?? null, nodeId: target?.getAttribute('data-node-id') ?? null, preset: target?.getAttribute('data-preset-value') ?? null } });
  }, true);
  const create = URL.createObjectURL.bind(URL), revoke = URL.revokeObjectURL.bind(URL);
  URL.createObjectURL = function (blob) { const url = create(blob); state.urls.set(url, { blob, revoked: 0 }); return url; };
  URL.revokeObjectURL = function (url) { const record = state.urls.get(url); if (record) record.revoked++; return revoke(url); };
  const click = HTMLAnchorElement.prototype.click;
  HTMLAnchorElement.prototype.click = function () {
    if (this.download) state.downloads.push({ name: this.download, url: this.href, blob: state.urls.get(this.href)?.blob, anchor: this });
    return click.apply(this, arguments); // Chrome must perform the real download.
  };
  const capture = HTMLCanvasElement.prototype.captureStream;
  if (capture) HTMLCanvasElement.prototype.captureStream = function () { const stream = capture.apply(this, arguments); state.tracks.push(...stream.getTracks()); return stream; };
  if (typeof MediaRecorder !== 'undefined') {
    const Recorder = MediaRecorder;
    window.MediaRecorder = new Proxy(Recorder, { construct(target, args) { const recorder = Reflect.construct(target, args); state.recorders.push(recorder); return recorder; } });
  }
  document.addEventListener('click', event => { if (event.target.closest('button[data-action="copy"]')) state.inCopyHandler = true; }, true);
  document.addEventListener('click', () => { state.inCopyHandler = false; });
  if (suppressClipboard) {
    state.originalClipboardCapability = typeof ClipboardItem !== 'undefined' && !!navigator.clipboard?.write;
    window.ClipboardItem = undefined;
  } else if (typeof ClipboardItem !== 'undefined') {
    const Item = ClipboardItem;
    window.ClipboardItem = new Proxy(Item, { construct(target, args) {
      const value = args[0]?.['image/png'];
      state.constructions.push({ gesture: navigator.userActivation?.isActive === true, inHandler: state.inCopyHandler, promise: !!value && typeof value.then === 'function' });
      return Reflect.construct(target, args);
    } });
  }
  if (navigator.clipboard?.write) {
    const write = navigator.clipboard.write.bind(navigator.clipboard);
    Object.defineProperty(navigator.clipboard, 'write', { configurable: true, value(items) {
      const observation = { state: 'pending', gesture: navigator.userActivation?.isActive === true }; state.writes.push(observation);
      try {
        return Promise.resolve(write(items)).then(value => { observation.state = 'resolved'; return value; }, error => { observation.state = 'rejected'; observation.error = error.name; throw error; });
      } catch (error) { observation.state = 'rejected'; observation.error = error.name; throw error; }
    } });
  }
}

export function snapshotReader() {
  const api = window.PhoenixDiagram, svg = document.querySelector('.diagram-container > svg');
  if (!api || !svg || !api.exportMenu || !api.motionGovernor) throw new Error('Phoenix Reader API/main SVG was not initialized');
  const overlay = '[data-ambient-flow-overlay], [data-intent-trace-overlay], [data-route-probe-overlay], [data-route-journey-overlay], [data-semantic-lens-overlay], [data-relationship-pulse-overlay], [data-relationship-hit-overlay], [data-legend-bridge-runtime]';
  const nodes = Array.from(svg.querySelectorAll('[data-node-id]')).map(node => ({ id: node.getAttribute('data-node-id'), label: node.getAttribute('data-node-label'), text: Array.from(node.querySelectorAll('text')).map(text => text.textContent) }));
  const edges = Array.from(svg.querySelectorAll('path[data-edge-from], line[data-edge-from], polyline[data-edge-from]')).filter(node => !node.closest(overlay)).map(node => ({ key: node.getAttribute('data-edge-key'), id: node.getAttribute('data-edge-id'), from: node.getAttribute('data-edge-from'), to: node.getAttribute('data-edge-to'), geometry: node.getAttribute('d') || node.getAttribute('points') || [node.getAttribute('x1'), node.getAttribute('y1'), node.getAttribute('x2'), node.getAttribute('y2')].join(',') }));
  return { mode: api.motionGovernor.mode(), owner: api.motionGovernor.owner(), focus: api.focus?.active?.() ?? null, view: api.view?.state?.() ?? null,
    viewBox: svg.getAttribute('viewBox'), preset: svg.getAttribute('data-preset'), transform: svg.style.getPropertyValue('transform'), clip: svg.style.getPropertyValue('clip-path'), nodes, edges,
    text: Array.from(svg.querySelectorAll('text')).filter(node => !node.closest(overlay)).map(node => node.textContent),
    eligibleMotion: svg.querySelectorAll('[data-animate="edge"], [data-node-id][data-animate="node"]').length > 0,
    clipboard: { item: typeof ClipboardItem !== 'undefined', write: typeof navigator.clipboard?.write === 'function', read: typeof navigator.clipboard?.read === 'function' },
    errors: window.__phoenixDiagramExportProbe?.errors || [],
  };
}

export async function awaitReader() {
  await document.fonts.ready;
  if (!window.PhoenixDiagram) throw new Error('Missing window.PhoenixDiagram');
  for (const owner of [PhoenixDiagram.readerLayout, PhoenixDiagram.viewerChromeLayout]) if (owner?.whenStable) await owner.whenStable();
  if (PhoenixDiagram.layoutStability?.whenStable) await PhoenixDiagram.layoutStability.whenStable();
  await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  return true;
}

export function printMotionSnapshot() {
  const svg = document.querySelector('.diagram-container > svg');
  if (!svg || typeof svg.getAnimations !== 'function') throw new Error('Print motion observation is unavailable');
  const runtime = '[data-ambient-flow-overlay], [data-intent-trace-overlay], [data-route-probe-overlay], [data-route-journey-overlay], [data-semantic-lens-overlay], [data-relationship-pulse-overlay], [data-relationship-hit-overlay], [data-legend-bridge-runtime]';
  // Tree toggles are Reader controls; their child-count badges stay hidden while a branch is open.
  const controls = '[data-tree-toggle], [data-tree-expand-all]';
  const meaning = Array.from(svg.querySelectorAll('[data-node-id], path[data-edge-from], line[data-edge-from], polyline[data-edge-from], text')).filter(element => !element.closest(runtime) && !element.closest(controls));
  const hidden = element => {
    for (let current = element; current; current = current.parentElement) {
      const style = getComputedStyle(current);
      if (style.display === 'none' || ['hidden', 'collapse'].includes(style.visibility) || Number(style.opacity) === 0) return true;
      if (current === svg) break;
    }
    return false;
  };
  return { printMedia: matchMedia('print').matches, capable: PhoenixDiagram.motionGovernor.capable === true,
    running: svg.getAnimations({ subtree: true }).filter(animation => animation.playState === 'running').length,
    staticCount: meaning.length, hiddenStatic: meaning.filter(hidden).length };
}

// Synchronous observation also works in a genuinely hidden document, where
// requestAnimationFrame and Reader layout-stability promises may be suspended.
export function motionSuppressionSnapshot(reader) {
  const svg = document.querySelector('.diagram-container svg'), html = document.documentElement, button = document.querySelector('#btn-motion');
  if (!svg || !button || typeof svg.getAnimations !== 'function') throw new Error('Motion suppression observation is unavailable');
  const runtime = '[data-ambient-flow-overlay], [data-intent-trace-overlay], [data-route-probe-overlay], [data-route-journey-overlay], [data-semantic-lens-overlay], [data-relationship-pulse-overlay], [data-relationship-hit-overlay], [data-legend-bridge-runtime]';
  const meaning = Array.from(svg.querySelectorAll('[data-node-id], path[data-edge-from], line[data-edge-from], polyline[data-edge-from], text')).filter(element => !element.closest(runtime));
  const visible = element => {
    for (let current = element; current; current = current.parentElement) {
      const style = getComputedStyle(current);
      if (style.display === 'none' || ['hidden', 'collapse'].includes(style.visibility) || Number(style.opacity) === 0) return false;
      if (current === svg) break;
    }
    return true;
  };
  const animations = svg.getAnimations({ subtree: true });
  return { reader, reducedMotion: matchMedia('(prefers-reduced-motion: reduce)').matches, hidden: document.hidden, visibilityState: document.visibilityState, printMedia: matchMedia('print').matches,
    documentHidden: html.getAttribute('data-document-hidden'), ambientMotion: html.getAttribute('data-ambient-motion'), capable: PhoenixDiagram.motionGovernor.capable === true,
    preference: localStorage.getItem('phoenix-diagram-motion'), button: { disabled: button.disabled, pressed: button.getAttribute('aria-pressed') },
    animationCount: animations.length, running: animations.filter(animation => animation.playState === 'running').length, meaning: { total: meaning.length, visible: meaning.flatMap((element, index) => visible(element) ? [index] : []) } };
}

export function readResources() {
  const state = window.__phoenixDiagramExportProbe;
  return { urls: Array.from(state.urls.values()).map(value => ({ bytes: value.blob.size, type: value.blob.type, revocations: value.revoked })),
    tracks: state.tracks.map(track => track.readyState), recorders: state.recorders.map(recorder => ({ state: recorder.state, listenersCleared: recorder.ondataavailable === null && recorder.onerror === null && recorder.onstop === null })),
    anchorsAttached: state.downloads.filter(download => download.anchor.isConnected).length, errors: state.errors.slice() };
}

export async function inspectBlob({ url, kind, expected, decodeMs = 12000, returnBase64 = false }) {
  const state = window.__phoenixDiagramExportProbe;
  const blob = state.urls.get(url)?.blob;
  if (!blob || !blob.size) throw new Error('Missing/nonempty actual exported Blob');
  const bytes = await blob.arrayBuffer();
  const digest = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))).map(value => value.toString(16).padStart(2, '0')).join('');
  const bounded = (promise, stage) => new Promise((resolve, reject) => {
    const timer = setTimeout(() => { const error = new Error(stage + ' decoder timed out'); error.name = 'TimeoutError'; reject(error); }, decodeMs);
    Promise.resolve(promise).then(value => { clearTimeout(timer); resolve(value); }, error => { clearTimeout(timer); reject(error); });
  });
  let result = { mime: blob.type, bytes: blob.size, sha256: digest };
  if (kind === 'svg') {
    const text = await blob.text(), root = new DOMParser().parseFromString(text, 'image/svg+xml');
    if (root.querySelector('parsererror') || root.documentElement.localName !== 'svg') throw new Error('Export is not a valid standalone SVG');
    const svg = root.documentElement;
    const transient = '[data-ambient-flow-overlay], [data-focus-selected], [data-focus-match], [data-intent-trace-overlay], [data-route-probe-overlay], [data-route-journey-overlay], [data-semantic-lens-overlay], [data-lens-match], [data-reach-match], [data-reader-legend-corner], [data-detail], [data-legend-bridge-runtime]';
    const actual = { viewBox: svg.getAttribute('viewBox'), nodes: Array.from(svg.querySelectorAll('[data-node-id]')).map(node => ({ id: node.getAttribute('data-node-id'), label: node.getAttribute('data-node-label'), text: Array.from(node.querySelectorAll('text')).map(text => text.textContent) })),
      edges: Array.from(svg.querySelectorAll('path[data-edge-from], line[data-edge-from], polyline[data-edge-from]')).map(node => ({ key: node.getAttribute('data-edge-key'), id: node.getAttribute('data-edge-id'), from: node.getAttribute('data-edge-from'), to: node.getAttribute('data-edge-to'), geometry: node.getAttribute('d') || node.getAttribute('points') || [node.getAttribute('x1'), node.getAttribute('y1'), node.getAttribute('x2'), node.getAttribute('y2')].join(',') })),
      text: Array.from(svg.querySelectorAll('text')).map(node => node.textContent) };
    if (svg.querySelector(transient) || svg.style.transform || svg.style.clipPath || ['data-view-scale', 'data-focus-active', 'data-route-active', 'data-lens-active', 'data-reach-active'].some(name => svg.hasAttribute(name))) throw new Error('Canonical SVG retained transient Reader state');
    const font = svg.querySelector('style')?.textContent || '';
    if (!font.includes('SIL OPEN FONT LICENSE') || !font.includes('data:font/woff2;base64,') || !font.includes('prefers-color-scheme: light')) throw new Error('Standalone SVG lost embedded font/license or dual-theme styles');
    result.canonical = { complete: true, utf8: text.startsWith('<?xml version="1.0" encoding="UTF-8"?>'), nodeCount: actual.nodes.length, edgeCount: actual.edges.length, textCount: actual.text.length, preset: svg.getAttribute('data-preset'), facts: actual };
    if (!result.canonical.utf8 || result.canonical.preset !== expected.preset) throw new Error('Standalone SVG lost UTF-8 declaration or the chosen preset');
    const image = new Image(), imageUrl = URL.createObjectURL(blob);
    try { await bounded(new Promise((resolve, reject) => { image.onload = resolve; image.onerror = () => reject(new Error('SVG Image decode failed')); image.src = imageUrl; }), 'SVG'); result.decode = { valid: true, width: image.naturalWidth, height: image.naturalHeight }; }
    finally { image.onload = image.onerror = null; image.src = ''; URL.revokeObjectURL(imageUrl); }
  } else if (kind === 'webm') {
    const video = document.createElement('video'); video.muted = true; video.playsInline = true;
    const videoUrl = URL.createObjectURL(blob); let frameCallback = 0;
    try {
      await bounded(new Promise((resolve, reject) => { video.onloadeddata = resolve; video.onerror = () => reject(new Error('WebM decoder error ' + video.error?.code)); video.src = videoUrl; video.load(); }), 'WebM');
      if (!(video.videoWidth > 0 && video.videoHeight > 0)) throw new Error('WebM has no decoded video dimensions');
      const canvas = document.createElement('canvas'); canvas.width = 160; canvas.height = 90; const context = canvas.getContext('2d', { willReadFrequently: true });
      const pixels = () => { context.drawImage(video, 0, 0, canvas.width, canvas.height); return new Uint8Array(context.getImageData(0, 0, canvas.width, canvas.height).data); };
      const first = pixels();
      await video.play();
      await bounded(new Promise((resolve, reject) => {
        if (typeof video.requestVideoFrameCallback !== 'function') { reject(new Error('requestVideoFrameCallback is unavailable')); return; }
        const sample = (_, frame) => { if (frame.mediaTime >= 2.25 || video.ended) resolve(); else frameCallback = video.requestVideoFrameCallback(sample); }; frameCallback = video.requestVideoFrameCallback(sample);
      }), 'WebM frame');
      const second = pixels(); let changed = 0; for (let index = 0; index < first.length; index++) if (first[index] !== second[index]) changed++;
      if (expected.eligibleMotion && changed === 0) throw new Error('WebM decoded frames contain no motion');
      result.decode = { valid: true, width: video.videoWidth, height: video.videoHeight, framesObserved: 2, changedSamples: changed, motionApplicable: expected.eligibleMotion };
    } finally { if (frameCallback && video.cancelVideoFrameCallback) video.cancelVideoFrameCallback(frameCallback); video.pause(); video.onloadeddata = video.onerror = null; video.removeAttribute('src'); video.load(); URL.revokeObjectURL(videoUrl); }
  } else {
    const image = await bounded(createImageBitmap(blob), kind);
    try {
      const canvas = document.createElement('canvas'); canvas.width = 64; canvas.height = 64;
      const context = canvas.getContext('2d', { willReadFrequently: true }); context.drawImage(image, 0, 0, 64, 64);
      const pixels = context.getImageData(0, 0, 64, 64).data; let varied = false;
      for (let index = 4; index < pixels.length; index += 4) if (pixels[index] !== pixels[0] || pixels[index + 1] !== pixels[1] || pixels[index + 2] !== pixels[2] || pixels[index + 3] !== pixels[3]) { varied = true; break; }
      if (!varied) throw new Error('Decoded raster is uniformly blank');
      result.decode = { valid: true, width: image.width, height: image.height, nonuniform: true };
    } finally { image.close(); }
  }
  if (!(result.decode.width > 0 && result.decode.height > 0)) throw new Error('Decoder returned empty dimensions');
  if (returnBase64) result.base64 = await new Promise((resolve, reject) => { const reader = new FileReader(); reader.onload = () => resolve(String(reader.result).split(',')[1]); reader.onerror = reject; reader.readAsDataURL(blob); });
  return result;
}

export function contentObservation({ expectedLiteral, sentinelName = '__phoenixDiagramXss' } = {}) {
  if (!window.PhoenixDiagram) throw new Error('Missing Phoenix Reader');
  const texts = Array.from(document.querySelectorAll('.header h1, .header .subtitle, .diagram-container > svg text')).map(node => node.textContent);
  return { sentinelDefined: Object.prototype.hasOwnProperty.call(globalThis, sentinelName), literalPresent: expectedLiteral === undefined || texts.some(text => text.includes(expectedLiteral)), errors: window.__phoenixDiagramExportProbe?.errors || [], locale: PhoenixDiagram.locale };
}
