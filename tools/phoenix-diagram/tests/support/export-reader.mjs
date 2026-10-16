// Maintenance-only Reader interaction acceptance; no production dependency.
import { setTimeout as delay } from 'node:timers/promises';
import { awaitReader } from './export-page.mjs';
export const REQUIRED_READER_INTERACTIONS = Object.freeze(['camera', 'search-empty', 'search-label', 'search-id', 'search-no-match', 'details', 'keyboard', 'style']);
export async function sendReaderKey(send, key, code, modifiers = 0, commands = []) {
  const windowsVirtualKeyCode = { Tab: 9, Enter: 13, Escape: 27, Backspace: 8, a: 65 }[key];
  const event = { key, code, modifiers, ...(windowsVirtualKeyCode ? { windowsVirtualKeyCode } : {}) };
  await send('Input.dispatchKeyEvent', { type: 'keyDown', ...event, ...(key === 'Enter' ? { text: '\r', unmodifiedText: '\r' } : {}), ...(commands.length ? { commands } : {}) });
  await send('Input.dispatchKeyEvent', { type: 'keyUp', ...event });
}

const same = (left, right) => JSON.stringify(left) === JSON.stringify(right);
function requireThat(condition, message) { if (!condition) throw new Error('Reader interaction: ' + message); }
function native(sample, type, field, value) { return sample.inputs.some(input => input.trusted === true && input.type === type && (!field || (field === 'key' ? input.key : input.target?.[field]) === value)); }
function closed(state) { return state?.focus === null && state.details?.hidden === true; }
function detail(state, id, label) { return state?.focus === id && state.details?.hidden === false && state.details.painted === true && state.details.id === id && state.details.label === label; }
function geometry(sample, state) { return state && same(sample.before.geometry, state.geometry); }
function completeGeometry(value) {
  const finite = value => typeof value === 'string' && value.trim().length > 0 && Number.isFinite(Number(value));
  const box = typeof value?.viewBox === 'string' ? value.viewBox.trim().split(/[\s,]+/).map(Number) : [];
  return box.length === 4 && box.every(Number.isFinite) && box[2] > 0 && box[3] > 0
    && Array.isArray(value.nodes) && value.nodes.length > 0 && value.nodes.every(node => typeof node.id === 'string' && node.id.length > 0 && typeof node.label === 'string' && ['x', 'y', 'width', 'height'].every(key => finite(node.body?.[key])) && Number(node.body.width) > 0 && Number(node.body.height) > 0)
    && Array.isArray(value.edges) && value.edges.every(edge => (typeof edge.d === 'string' && edge.d.trim().length > 0) || (typeof edge.points === 'string' && edge.points.trim().length > 0) || ['x1', 'y1', 'x2', 'y2'].every(key => finite(edge[key])))
    && Array.isArray(value.text) && value.text.length > 0 && value.text.every(text => typeof text.text === 'string' && finite(text.x) && finite(text.y));
}
function painted(state) { return ['scale', 'x', 'y'].every(key => Number.isFinite(state?.camera?.[key]) && Number.isFinite(state?.paintedCamera?.[key]) && Math.abs(state.camera[key] - state.paintedCamera[key]) <= (key === 'scale' ? 0.002 : 0.5)); }
function verifyOne(sample) {
  requireThat(completeGeometry(sample?.before?.geometry) && geometry(sample, sample.after), 'authored geometry changed or is incomplete');
  requireThat(Array.isArray(sample.inputs) && sample.inputs.length > 0 && sample.inputs.every(input => input.trusted === true), 'missing actual trusted browser input');
  const result = sample.result;
  requireThat(result && typeof result === 'object', 'missing measured result');
  if (sample.name === 'camera') {
    requireThat(closed(sample.before), 'initial no-selection detail state is inaccurate');
    for (const value of ['in', 'out', 'reset']) requireThat(native(sample, 'click', 'view', value), 'camera control was not clicked: ' + value);
    for (const type of ['pointerdown', 'pointermove', 'pointerup']) requireThat(native(sample, type), 'native canvas drag was not observed');
    for (const name of ['zoomIn', 'beforePan', 'afterPan', 'zoomOut', 'reset']) requireThat(geometry(sample, result[name]) && painted(result[name]), 'camera painted state/geometry was not measured: ' + name);
    requireThat(result.zoomIn.camera.scale > sample.before.camera.scale && result.zoomOut.camera.scale < result.zoomIn.camera.scale, 'zoom controls did not change painted scale');
    requireThat(Math.hypot(result.afterPan.camera.x - result.beforePan.camera.x, result.afterPan.camera.y - result.beforePan.camera.y) > 3 && Math.hypot(result.afterPan.paintedCamera.x - result.beforePan.paintedCamera.x, result.afterPan.paintedCamera.y - result.beforePan.paintedCamera.y) > 3, 'drag did not move the painted canvas');
    requireThat(result.reset.camera.scale === 1 && result.reset.camera.x === 0 && result.reset.camera.y === 0 && painted(sample.after), 'reset did not restore the camera');
  } else if (sample.name === 'search-empty') {
    requireThat(native(sample, 'click', 'id', 'btn-node-finder') && sample.after.finder.open && sample.after.finder.query === '' && sample.after.finder.emptyHidden && sample.after.finder.status.length > 0 && same(sample.after.finder.results.map(item => item.id).sort(), result.expectedIds.slice().sort()), 'empty search did not expose all actual nodes');
  } else if (sample.name === 'search-label' || sample.name === 'search-id') {
    requireThat(native(sample, 'input', 'id', 'node-finder-input') && sample.after.finder.open && sample.after.finder.query === result.query && sample.after.finder.results.some(item => item.id === result.expectedId && item.label === result.label) && sample.after.finder.emptyHidden, 'authored label/stable id search did not find its node');
    const node = sample.before.geometry.nodes.find(node => node.id === result.expectedId);
    requireThat(node?.label === result.label && result.query === (sample.name === 'search-id' ? result.expectedId : result.label), 'search query was not an actual authored label/id');
    if (sample.name === 'search-id') requireThat(result.distinctFromLabel === !result.label.toLowerCase().includes(result.expectedId.toLowerCase()), 'id/label evidence distinction is inaccurate');
  } else if (sample.name === 'search-no-match') {
    requireThat(native(sample, 'input', 'id', 'node-finder-input') && sample.after.finder.open && sample.after.finder.query === result.query && sample.after.finder.results.length === 0 && !sample.after.finder.emptyHidden && sample.after.finder.emptyText.length > 0 && !sample.after.finder.emptyText.startsWith('viewer.') && /0/.test(sample.after.finder.status) && sample.before.focus === sample.after.focus, 'no-match feedback or selection state is inaccurate');
  } else if (sample.name === 'details') {
    requireThat(detail(result.opened, result.expectedId, result.label) && detail(result.keyboardOpened, result.expectedId, result.label) && closed(result.escapeClosed) && closed(result.keyboardClosed) && closed(sample.after), 'details did not open with authored identity or close');
    requireThat(native(sample, 'click', 'nodeId', result.expectedId) && native(sample, 'keydown', 'key', 'Escape') && native(sample, 'keydown', 'key', 'Tab') && native(sample, 'keydown', 'key', 'Enter') && result.closeFocusPath.some(item => item.id === 'btn-focus-clear'), 'details close was not reached/activated by native keyboard input');
  } else if (sample.name === 'keyboard') {
    requireThat(native(sample, 'keydown', 'key', 'Tab') && native(sample, 'keydown', 'key', 'Enter') && native(sample, 'keydown', 'key', 'Escape'), 'keyboard controls were not exercised natively');
    requireThat(result.reached.some(item => item.id === 'btn-preset') && result.reached.some(item => item.id === 'btn-node-finder') && result.reached.some(item => item.view === 'in') && result.finderOpened.finder.open && !result.finderClosed.finder.open && result.finderClosed.active.id === 'btn-node-finder', 'keyboard did not reach main controls or restore focus after Escape');
  } else if (sample.name === 'style') {
    requireThat(native(sample, 'click', 'id', 'btn-preset') && native(sample, 'click', 'preset', result.to) && native(sample, 'click', 'preset', result.from), 'style was not switched/restored through actual controls');
    requireThat(result.from === sample.before.preset && result.from !== result.to && result.changed.preset === result.to && result.changed.htmlPreset === result.to && result.changed.presetChecked && result.restored.preset === result.from && sample.after.preset === result.from && geometry(sample, result.changed) && geometry(sample, result.restored), 'style switch changed geometry or did not change/restore style');
  } else requireThat(false, 'unknown required scope');
}
export function verifyReaderInteractions(value) {
  requireThat(value?.status === 'pass' && Array.isArray(value.observations) && value.observations.length === REQUIRED_READER_INTERACTIONS.length && new Set(value.observations.map(sample => sample.name)).size === REQUIRED_READER_INTERACTIONS.length, 'required scope is incomplete');
  for (const name of REQUIRED_READER_INTERACTIONS) { const sample = value.observations.find(sample => sample.name === name); requireThat(sample?.status === 'pass', 'required scope did not pass: ' + name); verifyOne(sample); }
  return true;
}

// Fixed browser probe; coordinates are authored SVG attributes, independent of
// camera/style presentation. Runtime badge/overlay text has explicit ownership.
export function readerInteractionSnapshot() {
  const api = PhoenixDiagram, svg = document.querySelector('.diagram-container > svg');
  const runtime = '[data-ambient-flow-overlay], [data-intent-trace-overlay], [data-route-probe-overlay], [data-route-journey-overlay], [data-semantic-lens-overlay], [data-relationship-pulse-overlay], [data-relationship-hit-overlay], [data-legend-bridge-runtime]';
  const attribute = (element, names) => Object.fromEntries(names.map(name => [name, element?.getAttribute(name) ?? null]));
  const active = document.activeElement;
  const identify = element => ({ id: element?.id || '', view: element?.getAttribute('data-view') ?? null, nodeId: element?.getAttribute('data-node-id') ?? null, tag: element?.tagName || '' });
  const chip = document.getElementById('focus-chip'), box = chip.getBoundingClientRect(), style = getComputedStyle(svg).transform;
  const matrix = style === 'none' ? new DOMMatrixReadOnly() : new DOMMatrixReadOnly(style);
  const current = api.preset.current();
  return { geometry: { viewBox: svg.getAttribute('viewBox'),
    nodes: Array.from(svg.querySelectorAll('g[data-node-id]')).filter(element => !element.closest(runtime)).map(element => ({ id: element.getAttribute('data-node-id'), label: element.getAttribute('data-node-label'), body: attribute(element.querySelector('rect[data-graph-role="node-body"]:not(.c-mask)'), ['x', 'y', 'width', 'height', 'rx', 'ry']) })),
    edges: Array.from(svg.querySelectorAll('path[data-edge-from],line[data-edge-from],polyline[data-edge-from]')).filter(element => !element.closest(runtime)).map(element => attribute(element, ['data-edge-key', 'data-edge-id', 'data-edge-from', 'data-edge-to', 'd', 'points', 'x1', 'y1', 'x2', 'y2'])),
    text: Array.from(svg.querySelectorAll('text')).filter(element => !element.closest(runtime)).map(element => ({ text: element.textContent, ...attribute(element, ['x', 'y', 'font-size', 'text-anchor']) })) },
    camera: api.view.state(), paintedCamera: { scale: matrix.a, x: matrix.e, y: matrix.f }, focus: api.focus.active(),
    finder: { open: !document.getElementById('node-finder').hidden, query: document.getElementById('node-finder-input').value, results: Array.from(document.querySelectorAll('#node-finder-results .node-finder-result')).map(element => ({ id: element.getAttribute('data-node-id'), label: element.querySelector('strong')?.textContent || '' })), emptyHidden: document.getElementById('node-finder-empty').hidden, emptyText: document.getElementById('node-finder-empty').textContent, status: document.getElementById('node-finder-status').textContent },
    details: { hidden: chip.hidden, painted: !chip.hidden && box.width > 0 && box.height > 0 && box.bottom > 0 && box.right > 0 && box.left < innerWidth && box.top < innerHeight, id: document.getElementById('focus-id').textContent, label: document.getElementById('focus-label').textContent },
    active: identify(active), preset: current, htmlPreset: document.documentElement.getAttribute('data-preset'), presetChecked: Array.from(document.querySelectorAll('[data-preset-value]')).find(element => element.getAttribute('data-preset-value') === current)?.getAttribute('aria-checked') === 'true' };
}

export async function runReaderInteractions(page, onProgress) {
  const report = page.report.readerInteractions;
  report.status = 'failed';
  await page.load('dark');
  const snapshot = () => page.evaluate(readerInteractionSnapshot);
  async function stable() {
    await page.evaluate(awaitReader);
    const deadline = Date.now() + page.timeouts.commandMs;
    for (;;) { const value = await snapshot(); if (painted(value)) return value; if (Date.now() >= deadline) throw new Error('Reader painted camera did not settle'); await delay(25); }
  }
  const key = (key, code, modifiers = 0, commands = []) => sendReaderKey(page.send, key, code, modifiers, commands);
  async function search(query) {
    await page.click('#node-finder-input'); await key('a', 'KeyA', process.platform === 'darwin' ? 4 : 2, ['selectAll']); await key('Backspace', 'Backspace');
    requireThat((await snapshot()).finder.query === '', 'native search replacement did not clear the actual input');
    if (query) await page.send('Input.insertText', { text: query });
  }
  async function tabUntil(predicate) {
    const reached = [];
    for (let index = 0; index < 80; index++) { await key('Tab', 'Tab'); const active = (await snapshot()).active; reached.push(active); if (predicate(active)) return reached; }
    throw new Error('Reader keyboard did not reach its actual control');
  }
  const nodes = (await snapshot()).geometry.nodes;
  const labelNode = nodes.find(node => node.label?.trim()), idNode = nodes.find(node => node.id && !node.label.toLowerCase().includes(node.id.toLowerCase())) || labelNode;
  requireThat(labelNode && idNode, 'actual SVG has no searchable label/id');
  async function observe(name, action) {
    const sample = report.observations.find(sample => sample.name === name); sample.status = 'failed';
    onProgress?.({ stage: 'reader-interaction', name });
    sample.before = await stable(); const start = await page.evaluate(() => __phoenixDiagramExportProbe.readerInputs.length);
    sample.result = await action(); sample.after = await stable();
    sample.inputs = await page.evaluate(start => __phoenixDiagramExportProbe.readerInputs.slice(start), start);
    verifyOne(sample); sample.status = 'pass';
  }
  await observe('camera', async () => {
    await page.click('[data-view="in"]'); await stable(); await page.click('[data-view="in"]'); const zoomIn = await stable(), beforePan = zoomIn;
    const from = await page.evaluate(() => {
      const container = document.querySelector('.diagram-container'), box = container.getBoundingClientRect();
      for (const y of [0.12, 0.85, 0.5]) for (const x of [0.12, 0.85, 0.5]) {
        const point = { x: box.left + box.width * x, y: box.top + box.height * y }, target = document.elementFromPoint(point.x, point.y);
        if (target?.closest('.diagram-container') === container && !target.closest('.diagram-nav,.focus-chip,.node-finder,.diagram-guide,.overview-map,.route-probe,.semantic-lens,[data-node-id],[data-relationship-hit-key]')) return point;
      }
      throw new Error('Reader canvas has no actual unblocked drag surface');
    });
    await page.send('Input.dispatchMouseEvent', { type: 'mousePressed', ...from, button: 'left', buttons: 1, clickCount: 1 });
    await page.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: from.x - 36, y: from.y - 20, button: 'none', buttons: 1 });
    await page.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: from.x - 36, y: from.y - 20, button: 'left', buttons: 0, clickCount: 1 });
    const afterPan = await stable(); await page.click('[data-view="out"]'); const zoomOut = await stable(); await page.click('[data-view="reset"]'); const reset = await stable();
    return { zoomIn, beforePan, afterPan, zoomOut, reset, drag: { from, to: { x: from.x - 36, y: from.y - 20 } } };
  });
  await observe('search-empty', async () => { await page.click('#btn-node-finder'); return { expectedIds: nodes.map(node => node.id) }; });
  await observe('search-label', async () => { await search(labelNode.label); return { query: labelNode.label, expectedId: labelNode.id, label: labelNode.label }; });
  await observe('search-id', async () => { await search(idNode.id); return { query: idNode.id, expectedId: idNode.id, label: idNode.label, distinctFromLabel: !idNode.label.toLowerCase().includes(idNode.id.toLowerCase()) }; });
  await observe('search-no-match', async () => { const query = '__phoenix_diagram_no_match_' + page.artifact.sha256.slice(0, 16); await search(query); return { query }; });
  await observe('details', async () => {
    await search(labelNode.label); await key('Enter', 'Enter'); const opened = await stable(); await key('Escape', 'Escape'); const escapeClosed = await stable();
    await page.click('g[data-node-id="' + labelNode.id + '"]'); const keyboardOpened = await stable();
    const closeFocusPath = await tabUntil(active => active.id === 'btn-focus-clear'); await key('Enter', 'Enter'); const keyboardClosed = await stable();
    return { expectedId: labelNode.id, label: labelNode.label, opened, escapeClosed, keyboardOpened, keyboardClosed, closeFocusPath };
  });
  await page.load('dark');
  await observe('keyboard', async () => {
    const reached = []; let finderOpened, finderClosed;
    for (let index = 0; index < 80; index++) {
      await key('Tab', 'Tab'); const active = (await snapshot()).active; reached.push(active);
      if (active.id === 'btn-node-finder' && !finderOpened) { await key('Enter', 'Enter'); finderOpened = await stable(); await key('Escape', 'Escape'); finderClosed = await stable(); }
      if (reached.some(item => item.id === 'btn-preset') && finderClosed && active.view === 'in') break;
    }
    return { reached, finderOpened, finderClosed };
  });
  await observe('style', async () => {
    const from = (await snapshot()).preset, to = await page.evaluate(from => Array.from(document.querySelectorAll('[data-preset-value]')).map(element => element.getAttribute('data-preset-value')).find(value => value !== from), from);
    requireThat(to, 'no actual alternate style control'); await page.click('#btn-preset'); await page.click('[data-preset-value="' + to + '"]'); const changed = await stable();
    await page.click('#btn-preset'); await page.click('[data-preset-value="' + from + '"]'); const restored = await stable();
    return { from, to, changed, restored };
  });
  report.status = 'pass'; verifyReaderInteractions(report); await page.resources('reader-interactions');
}
