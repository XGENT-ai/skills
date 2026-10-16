// Small deterministic DOM/media boundary fixture for actual Phoenix Reader JS.
// Test cases are adapted from the frozen Archify 3.1.0 MIT browser tests;
// no fake media bytes from this fixture are evidence of a real encoded file.
import fs from 'node:fs';
import vm from 'node:vm';
import { fileURLToPath } from 'node:url';

const viewer = fileURLToPath(new URL('../../../viewer/', import.meta.url));
const source = name => fs.readFileSync(viewer + name, 'utf8');
const escape = value => String(value).replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');

export class Events {
  listeners = new Map();
  addEventListener(type, callback) { if (!this.listeners.has(type)) this.listeners.set(type, new Set()); this.listeners.get(type).add(callback); }
  removeEventListener(type, callback) { this.listeners.get(type)?.delete(callback); }
  emit(type, extra = {}) {
    const event = { type, target: this, preventDefault() {}, stopPropagation() {}, ...extra };
    for (const callback of [...(this.listeners.get(type) || [])]) callback.call(this, event);
  }
}

function style() {
  const values = new Map();
  const methods = {
    getPropertyValue: name => values.get(name) || '',
    setProperty: (name, value) => { values.set(name, String(value)); },
    removeProperty: name => { const value = values.get(name) || ''; values.delete(name); return value; },
  };
  const dash = name => String(name).replace(/[A-Z]/g, letter => '-' + letter.toLowerCase());
  return new Proxy(methods, {
    get(target, key) { if (key === 'cssText') return [...values].map(([name, value]) => `${name}: ${value};`).join(' '); return target[key] || values.get(dash(key)) || ''; },
    set(_, key, value) {
      if (key === 'cssText') { values.clear(); for (const item of String(value).split(';')) { const at = item.indexOf(':'); if (at >= 0) values.set(item.slice(0, at).trim(), item.slice(at + 1).trim()); } }
      else values.set(dash(key), String(value));
      return true;
    },
  });
}

function matchesSingle(node, selector) {
  selector = selector.trim();
  if (!selector || selector === '*') return true;
  const not = [...selector.matchAll(/:not\(([^)]+)\)/g)].map(match => match[1]);
  if (not.some(value => matchesSingle(node, value))) return false;
  selector = selector.replace(/:not\([^)]+\)/g, '');
  const attributes = [...selector.matchAll(/\[([^\]=*\s]+)(\*?=)?(?:"([^"]*)"|'([^']*)'|([^\]]*))?\]/g)];
  for (const [, name, operator, quoted, single, bare] of attributes) {
    const actual = node.getAttribute(name), expected = quoted ?? single ?? bare;
    if (actual === null || operator === '=' && actual !== expected || operator === '*=' && !actual.includes(expected)) return false;
  }
  selector = selector.replace(/\[[^\]]*\]/g, '');
  const id = selector.match(/#([\w-]+)/)?.[1];
  if (id && node.id !== id) return false;
  if ([...selector.matchAll(/\.([\w-]+)/g)].some(match => !node.classList.contains(match[1]))) return false;
  const tag = selector.match(/^[\w-]+/)?.[0];
  return !tag || node.localName === tag.toLowerCase();
}

export class Element extends Events {
  constructor(tag, attributes = {}, text = '') {
    super(); this.localName = tag.toLowerCase(); this.tagName = tag; this.attrs = new Map(); this.children = []; this.parentNode = null; this.style = style(); this.ownText = text; this.hidden = false; this.disabled = false;
    for (const [name, value] of Object.entries(attributes)) this.setAttribute(name, value);
    this.classList = { contains: name => this.className.split(/\s+/).includes(name), add: (...names) => { this.className = [...new Set([...this.className.split(/\s+/).filter(Boolean), ...names])].join(' '); }, remove: (...names) => { this.className = this.className.split(/\s+/).filter(name => !names.includes(name)).join(' '); } };
  }
  get id() { return this.getAttribute('id') || ''; } set id(value) { this.setAttribute('id', value); }
  get className() { return this.getAttribute('class') || ''; } set className(value) { this.setAttribute('class', value); }
  get attributes() { return [...this.attrs].map(([name, value]) => ({ name, value })); }
  get dataset() { return Object.fromEntries([...this.attrs].filter(([name]) => name.startsWith('data-')).map(([name, value]) => [name.slice(5).replace(/-([a-z])/g, (_, letter) => letter.toUpperCase()), value])); }
  get firstChild() { return this.children[0] || null; }
  get nextSibling() { if (!this.parentNode) return null; return this.parentNode.children[this.parentNode.children.indexOf(this) + 1] || null; }
  get nextElementSibling() { return this.nextSibling; }
  get previousElementSibling() { if (!this.parentNode) return null; return this.parentNode.children[this.parentNode.children.indexOf(this) - 1] || null; }
  get textContent() { return this.ownText + this.children.map(child => child.textContent).join(''); }
  set textContent(value) { this.ownText = String(value); this.children = []; }
  setAttribute(name, value) { if (name === 'style') this.style.cssText = value; else this.attrs.set(name, String(value)); }
  getAttribute(name) { return name === 'style' ? this.style.cssText || null : this.attrs.get(name) ?? null; }
  hasAttribute(name) { return this.getAttribute(name) !== null; }
  removeAttribute(name) { if (name === 'style') this.style.cssText = ''; else this.attrs.delete(name); }
  appendChild(node) { node.remove(); node.parentNode = this; this.children.push(node); return node; }
  insertBefore(node, before) { if (!before) return this.appendChild(node); node.remove(); const at = this.children.indexOf(before); if (at < 0) throw new Error('fixture insertBefore target missing'); node.parentNode = this; this.children.splice(at, 0, node); return node; }
  removeChild(node) { const at = this.children.indexOf(node); if (at < 0) throw new Error('fixture removeChild target missing'); this.children.splice(at, 1); node.parentNode = null; return node; }
  remove() { if (this.parentNode) this.parentNode.removeChild(this); }
  contains(node) { return node === this || this.children.some(child => child.contains(node)); }
  matches(selector) { return selector.split(',').some(part => matchesSingle(this, part)); }
  closest(selector) { for (let node = this; node; node = node.parentNode) if (node.matches(selector)) return node; return null; }
  querySelectorAll(selector) { const result = []; for (const child of this.children) { if (child.matches(selector)) result.push(child); result.push(...child.querySelectorAll(selector)); } return result; }
  querySelector(selector) { return this.querySelectorAll(selector)[0] || null; }
  cloneNode(deep) { const clone = new Element(this.localName, Object.fromEntries(this.attrs), this.ownText); clone.style.cssText = this.style.cssText; if (this.viewBox) clone.viewBox = structuredClone(this.viewBox); if (deep) for (const child of this.children) clone.appendChild(child.cloneNode(true)); return clone; }
  focus() { this.document.activeElement = this; }
  getCTM() { return null; }
  getBBox() { return { x: Number(this.getAttribute('x') || 0), y: Number(this.getAttribute('y') || 0), width: Number(this.getAttribute('width') || 100), height: Number(this.getAttribute('height') || 40) }; }
  getTotalLength() { return 100; }
  getPointAtLength(distance) { return { x: distance, y: 70 }; }
  get outerHTML() { const attributes = [...this.attrs]; if (this.style.cssText) attributes.push(['style', this.style.cssText]); return `<${this.localName}${attributes.map(([key, value]) => ` ${key}="${escape(value)}"`).join('')}>${escape(this.ownText)}${this.children.map(child => child.outerHTML).join('')}</${this.localName}>`; }
}

export class Clock {
  now = 0; next = 1; tasks = new Map(); cancelled = [];
  timeout(callback, delay = 0, kind = 'timer') { const id = this.next++; this.tasks.set(id, { callback, time: this.now + Number(delay), kind }); return id; }
  clear(id) { this.tasks.delete(id); this.cancelled.push(id); }
  async advance(duration) {
    const until = this.now + duration; let count = 0;
    for (;;) {
      await Promise.resolve(); await Promise.resolve();
      const next = [...this.tasks].filter(([, task]) => task.time <= until).sort((a, b) => a[1].time - b[1].time || a[0] - b[0])[0];
      if (!next) break;
      if (++count > 100000) throw new Error('fixture timer loop');
      this.now = next[1].time; this.tasks.delete(next[0]); next[1].callback(this.now);
    }
    this.now = until;
    for (let index = 0; index < 8; index++) await Promise.resolve();
  }
}

export function observe(promise) {
  const result = { state: 'pending', value: null };
  Promise.resolve(promise).then(value => { result.state = 'resolved'; result.value = value; }, error => { result.state = 'rejected'; result.value = error; });
  return result;
}

export function runtime(options = {}) {
  const clock = new Clock(), urls = new Map(), images = [], streams = [], recorders = [], canvases = [], downloads = [], alerts = [], logs = [], copies = [], serialized = [];
  const document = new Events();
  const element = (tag, attributes = {}, text = '') => { const node = new Element(tag, attributes, text); node.document = document; return node; };
  const html = element('html', { 'data-theme': 'dark', 'data-preset': 'classic' }), head = element('head'), body = element('body'); html.appendChild(head); html.appendChild(body);
  const svg = element('svg', { viewBox: '0 0 320 180', 'data-animation': 'trace', 'data-preset': 'classic' }); svg.viewBox = { baseVal: { x: 0, y: 0, width: 320, height: 180 } };
  const panel = element('div', { class: 'diagram-container' }); panel.appendChild(svg); body.appendChild(panel);
  for (const [id, x] of [['a', 10], ['b', 180], ['isolated', 240]]) {
    const node = element('g', { 'data-node-id': id, 'data-node-label': `Node ${id}`, 'aria-label': `Node ${id}`, 'aria-pressed': 'false' });
    node.appendChild(element('rect', { x, y: 10, width: 60, height: 40, class: 'c-backend', 'data-animate': 'node', style: '--step: 0' }));
    node.appendChild(element('text', {}, `完整 ${id}`)); svg.appendChild(node);
  }
  svg.appendChild(element('path', { 'data-edge-from': 'a', 'data-edge-to': 'b', 'data-edge-key': '0', d: 'M 70 70 L 180 70', 'data-animate': 'edge', class: 'a-default', style: '--step: 0', 'marker-end': 'url(#arrowhead)' }));
  svg.appendChild(element('text', { 'data-detail': 'fine' }, 'Keep the entire authored note 中文'));
  const legend = element('g', { 'data-legend': '', 'data-legend-semantic-kind': 'backend' }); legend.appendChild(element('text', {}, 'Backend 后端')); svg.appendChild(legend);
  const header = element('div', { class: 'header' }), heading = element('h1', {}, 'Fixture diagram'); header.appendChild(heading); body.appendChild(header);
  const menu = element('div', { id: 'export-menu' }), button = element('button', { id: 'btn-export' }); body.appendChild(menu); body.appendChild(button);
  for (const format of ['svg', 'svg-light', 'svg-dark', 'png', 'jpeg', 'webp', 'webm']) menu.appendChild(element('button', { role: 'menuitem', 'data-format': format }));
  for (const action of ['copy', 'route-share-card', 'reach-share-card']) menu.appendChild(element('button', { role: 'menuitem', 'data-action': action }));
  const motionButton = element('button', { id: 'btn-motion' }), motionLabel = element('span', { id: 'motion-label' }); body.appendChild(motionButton); body.appendChild(motionLabel);
  const font = element('style', { id: 'phoenix-diagram-fonts' }, source('template.source.html').match(/<style id="phoenix-diagram-fonts">([\s\S]*?)<\/style>/)[1]); body.appendChild(font);
  Object.assign(document, { documentElement: html, head, body, title: 'Fixture Diagram', hidden: false, activeElement: body, fonts: { ready: Promise.resolve() }, styleSheets: [{ cssRules: [{ type: 1, selectorText: ':root', cssText: ':root { --bg: #101827; --text: #ffffff; }' }] }] });
  document.getElementById = id => html.querySelector('#' + id);
  document.querySelector = selector => selector === '.diagram-container svg' || selector === '.diagram-container > svg' ? svg : selector === '.header h1' ? heading : selector === '.header .subtitle' ? null : html.querySelector(selector);
  document.querySelectorAll = selector => html.querySelectorAll(selector);
  const computed = node => ({ backgroundColor: '#101827', backgroundImage: 'none', fontFamily: 'sans-serif', stroke: '#22d3ee', strokeWidth: '1.5', getPropertyValue(name) { return node.style?.getPropertyValue(name) || ({ '--bg': node.getAttribute?.('data-theme') === 'light' ? '#ffffff' : '#101827', '--text': '#ffffff', '--panel': '#1a2435', '--frontend-stroke': '#22d3ee' })[name] || ''; } });
  class Canvas extends Element {
    constructor() { super('canvas'); this.document = document; this.context = new Proxy({ canvas: this, measureText: text => ({ width: String(text).length * 8 }) }, { get: (target, key) => target[key] ?? (() => {}) }); canvases.push(this); }
    getContext() { if (options.context === 'throw') throw new Error('canvas context failure'); return options.context === 'null' ? null : this.context; }
    toDataURL(mime) { return 'data:' + mime + ';base64,fixture'; }
    toBlob(callback, mime) { if (options.toBlob === 'throw') throw new Error('encoder failure'); queueMicrotask(() => callback(options.toBlob === 'null' ? null : new Blob(options.toBlob === 'empty' ? [] : ['fixture raster bytes'], { type: mime }))); }
    captureStream(fps) { if (options.capture === 'throw') throw new Error('capture failure'); const track = { state: 'live', stops: 0, stop() { this.stops++; this.state = 'ended'; } }; const stream = { fps, track, getTracks() { return [track]; } }; streams.push(stream); return stream; }
  }
  document.createElement = tag => {
    if (tag === 'canvas') return new Canvas();
    const node = element(tag);
    if (tag === 'a') node.click = () => downloads.push({ filename: node.download, url: node.href, blob: urls.get(node.href)?.blob, anchor: node });
    return node;
  };
  document.createElementNS = (_, tag) => element(tag);
  class Image {
    constructor() { this.onload = null; this.onerror = null; images.push(this); }
    set src(value) { this.url = value; if (options.image === 'throw') throw new Error('image source failure'); if (options.image === 'pending') return; clock.timeout(() => { if (options.image === 'error') this.onerror?.(new Error('image decode failure')); else this.onload?.(); }, 0, 'image'); }
  }
  class Recorder {
    static isTypeSupported() { return options.recorder !== 'unsupported'; }
    constructor(stream, config) { if (options.recorder === 'constructor') throw new Error('recorder constructor failure'); this.stream = stream; this.mimeType = config.mimeType; this.state = 'inactive'; this.stopCalls = 0; this.flushCalls = 0; recorders.push(this); }
    start() { if (options.recorder === 'start') throw new Error('recorder start failure'); this.state = 'recording'; if (options.recorder === 'error') clock.timeout(() => this.onerror?.({ error: new Error('recorder async failure') }), 20, 'recorder-event'); }
    requestData() { this.flushCalls++; if (options.recorder === 'flush') throw new Error('flush failure'); }
    stop() { this.stopCalls++; if (options.recorder === 'stop') throw new Error('recorder stop failure'); this.state = 'inactive'; if (options.recorder === 'no-stop') return; queueMicrotask(() => { if (options.recorder !== 'empty') this.ondataavailable?.({ data: new Blob(['fixture encoded bytes'], { type: this.mimeType }) }); this.onstop?.(); }); }
  }
  const storage = options.storage || new Map(); const storageCalls = [];
  const media = new Events(); media.matches = !!options.reduced;
  const window = new Events(); window.location = { search: '' }; window.matchMedia = () => media;
  window.getComputedStyle = computed;
  const activation = { active: false };
  class ClipboardItem {
    constructor(data) { const value = data['image/png']; copies.push({ kind: value && typeof value.then === 'function' ? 'promise' : 'blob', gesture: activation.active, value }); if (options.clipboard === 'fallback' && typeof value?.then === 'function') throw new Error('promise clipboard unsupported'); this.value = value; }
  }
  const clipboard = options.clipboard === 'unsupported' ? undefined : { write(items) { copies.push({ kind: 'write', gesture: activation.active }); if (options.clipboard === 'denied') return Promise.reject(new Error('clipboard permission denied')); return Promise.resolve(items[0].value).then(blob => { copies.push({ kind: 'written', blob }); }); } };
  const context = { window, document, navigator: { clipboard, userActivation: { get isActive() { return activation.active; } } }, Blob, Image, MediaRecorder: Recorder, HTMLCanvasElement: Canvas, ClipboardItem,
    URL: { createObjectURL(blob) { const url = 'blob:fixture/' + (urls.size + 1); urls.set(url, { blob, revocations: 0 }); return url; }, revokeObjectURL(url) { if (!urls.has(url)) throw new Error('revoking another action URL'); urls.get(url).revocations++; } },
    XMLSerializer: class { serializeToString(node) { serialized.push(node); return node.outerHTML; } }, URLSearchParams,
    getComputedStyle: computed, performance: { now: () => clock.now }, console: { error: error => logs.push(error), warn: error => logs.push(error), log() {} }, alert: message => alerts.push(message),
    setTimeout: (callback, delay) => clock.timeout(callback, delay), clearTimeout: id => clock.clear(id), requestAnimationFrame: callback => clock.timeout(callback, 16, 'frame'), cancelAnimationFrame: id => clock.clear(id),
    localStorage: { getItem(key) { storageCalls.push(['get', key]); return storage.get(key) ?? null; }, setItem(key, value) { storageCalls.push(['set', key, value]); storage.set(key, value); }, removeItem(key) { storageCalls.push(['remove', key]); storage.delete(key); } },
    PhoenixDiagram: { locale: options.locale || 'en' }, hasDrawableGeometry: node => !!node?.getAttribute('d'), queueMicrotask,
  };
  if (options.clipboard === 'unsupported') delete context.ClipboardItem;
  const sandbox = vm.createContext(context);
  // Execute the actual classic-scope translation helpers, so feedback tests
  // observe real bundled text rather than treating an i18n key as feedback.
  const shell = source('template.source.html');
  const helpers = shell.slice(shell.indexOf('    function viewerText('), shell.indexOf('    function viewerKindLabel('));
  context.phoenixDiagramI18nData = { messages: Object.assign({}, JSON.parse(source('locales/en.json')), JSON.parse(source('locales/' + (options.locale || 'en') + '.json'))) };
  vm.runInContext(helpers, sandbox);
  function exportModule() { vm.runInContext(source('export.js').replace('/* PHOENIX:EXPORT_CLEANUP */', () => source('export-cleanup.js')), sandbox); }
  function governorModule() { vm.runInContext(source('motion-governor.js'), sandbox); }
  return { context, api: context.PhoenixDiagram, document, html, svg, panel, menu, button, motionButton, media, window, storage, storageCalls, clock, urls, images, streams, recorders, canvases, downloads, alerts, logs, copies, serialized, exportModule, governorModule,
    async outcome(promise, duration = 1500) { const result = observe(promise); await clock.advance(duration); return result; },
    clickCopy() { activation.active = true; try { menu.emit('click', { target: menu.querySelector('button[data-action="copy"]') }); } finally { activation.active = false; } },
  };
}
