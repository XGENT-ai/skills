// Boots the assembled Phoenix Reader (assets/template.html) over a small
// deterministic DOM: an HTML parser for the template and native SVG fixtures,
// a selector engine, capture/bubble event dispatch and the browser globals the
// Reader calls. Layout values are fixed, so tests built on this fixture prove
// Reader state transitions and receipts, not rendering, timing or media bytes.
import fs from 'node:fs';
import vm from 'node:vm';
import { Clock, Element as FixtureElement } from './runtime.mjs';

const assets = new URL('../../../assets/', import.meta.url);
export const templateSource = fs.readFileSync(new URL('template.html', assets), 'utf8');
export const bundledCatalogs = JSON.parse(fs.readFileSync(new URL('catalogs.json', assets), 'utf8'));
export const plain = value => value === undefined ? undefined : JSON.parse(JSON.stringify(value));

const SVG_NS = 'http://www.w3.org/2000/svg';
const HTML_NS = 'http://www.w3.org/1999/xhtml';
const VOID = new Set(['area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input', 'link', 'meta', 'source', 'track', 'wbr']);
const RAW = new Set(['script', 'style', 'textarea', 'title']);
const ENTITIES = { amp: '&', lt: '<', gt: '>', quot: '"', apos: "'", nbsp: '\u00a0', times: '\u00d7' };
const NON_BUBBLING = new Set(['focus', 'blur', 'load', 'hashchange', 'resize', 'scroll', 'mouseenter', 'mouseleave', 'pointerenter', 'pointerleave', 'beforeprint', 'afterprint']);

function decode(text) {
  return text.replace(/&(#x[0-9a-f]+|#\d+|[a-z]+);/gi, (match, name) => {
    if (name[0] === '#') return String.fromCodePoint(name[1].toLowerCase() === 'x' ? Number.parseInt(name.slice(2), 16) : Number(name.slice(1)));
    return Object.hasOwn(ENTITIES, name) ? ENTITIES[name] : match;
  });
}

// ---- Selectors -----------------------------------------------------------

function closingParen(text, open) {
  let depth = 0, quote = null;
  for (let index = open; index < text.length; index++) {
    const character = text[index];
    if (quote) { if (character === quote) quote = null; continue; }
    if (character === '"' || character === "'") quote = character;
    else if (character === '(') depth++;
    else if (character === ')' && --depth === 0) return index;
  }
  throw new Error('Unbalanced selector: ' + text);
}

function splitList(text) {
  const parts = [];
  let depth = 0, quote = null, start = 0;
  for (let index = 0; index < text.length; index++) {
    const character = text[index];
    if (quote) { if (character === quote) quote = null; continue; }
    if (character === '"' || character === "'") quote = character;
    else if (character === '(' || character === '[') depth++;
    else if (character === ')' || character === ']') depth--;
    else if (character === ',' && depth === 0) { parts.push(text.slice(start, index)); start = index + 1; }
  }
  parts.push(text.slice(start));
  return parts.map(part => part.trim()).filter(Boolean);
}

function parseComplex(text) {
  const parts = [];
  let current = '', depth = 0, quote = null, pending = null;
  const flush = () => {
    if (!current.trim()) return;
    parts.push({ compound: current.trim(), combinator: parts.length ? pending || ' ' : null });
    current = ''; pending = null;
  };
  for (const character of text) {
    if (quote) { current += character; if (character === quote) quote = null; continue; }
    if (character === '"' || character === "'") { quote = character; current += character; continue; }
    if (character === '(' || character === '[') depth++;
    if (character === ')' || character === ']') depth--;
    if (depth === 0 && (character === '>' || character === '+' || character === '~')) { flush(); pending = character; continue; }
    if (depth === 0 && /\s/.test(character)) { flush(); continue; }
    current += character;
  }
  flush();
  return parts;
}

const parsed = new Map();
function selectorList(text) {
  if (!parsed.has(text)) parsed.set(text, splitList(text).map(parseComplex));
  return parsed.get(text);
}

const isElement = node => node instanceof FixtureElement;

function matchCompound(node, compound, scope) {
  let rest = compound;
  const tag = rest.match(/^(\*|[a-zA-Z][\w-]*)/);
  if (tag) {
    if (tag[1] !== '*' && node.localName !== tag[1].toLowerCase()) return false;
    rest = rest.slice(tag[0].length);
  }
  while (rest) {
    let match;
    if ((match = rest.match(/^#([\w-]+)/))) {
      if (node.id !== match[1]) return false;
    } else if ((match = rest.match(/^\.([\w-]+)/))) {
      if (!node.className.split(/\s+/).includes(match[1])) return false;
    } else if ((match = rest.match(/^\[\s*([^\]\s~|^$*=]+)\s*(?:([~|^$*]?=)\s*(?:"([^"]*)"|'([^']*)'|([^\]\s]*)))?\s*\]/))) {
      const [, name, operator, double, single, bare] = match;
      const actual = node.getAttribute(name), expected = double ?? single ?? bare;
      if (actual === null) return false;
      if (operator === '=' && actual !== expected) return false;
      if (operator === '*=' && !actual.includes(expected)) return false;
      if (operator === '^=' && !actual.startsWith(expected)) return false;
      if (operator === '$=' && !actual.endsWith(expected)) return false;
      if (operator === '~=' && !actual.split(/\s+/).includes(expected)) return false;
      if (operator === '|=' && actual !== expected && !actual.startsWith(expected + '-')) return false;
    } else if ((match = rest.match(/^:([\w-]+)\(/))) {
      const end = closingParen(rest, match[0].length - 1);
      const argument = rest.slice(match[0].length, end);
      const name = match[1];
      if (name === 'not') { if (matchesList(node, argument, scope)) return false; }
      else if (name === 'is' || name === 'where') { if (!matchesList(node, argument, scope)) return false; }
      else if (name === 'has') { if (!node.querySelector(argument.trim().replace(/^>/, ':scope >'))) return false; }
      else throw new Error('Unsupported selector pseudo-class: ' + name);
      rest = rest.slice(end + 1);
      continue;
    } else if ((match = rest.match(/^:([\w-]+)/))) {
      const name = match[1];
      if (name === 'scope') { if (node !== scope) return false; }
      else if (name === 'disabled') { if (!node.disabled) return false; }
      else if (name === 'enabled') { if (node.disabled) return false; }
      else if (name === 'focus') { if (node.document?.activeElement !== node) return false; }
      else if (name === 'focus-within') { if (!node.contains(node.document?.activeElement)) return false; }
      else if (name === 'hover' || name === 'focus-visible' || name === 'active') return false;
      else if (name === 'first-child') { if (node.parentNode?.children?.[0] !== node) return false; }
      else if (name === 'last-child') { if (node.parentNode?.children?.at(-1) !== node) return false; }
      else throw new Error('Unsupported selector pseudo-class: ' + name);
    } else throw new Error('Unsupported selector: ' + compound);
    rest = rest.slice(match[0].length);
  }
  return true;
}

function matchParts(node, parts, index, scope) {
  if (!matchCompound(node, parts[index].compound, scope)) return false;
  if (index === 0) return true;
  const combinator = parts[index].combinator;
  if (combinator === '>') return isElement(node.parentNode) && matchParts(node.parentNode, parts, index - 1, scope);
  if (combinator === ' ') {
    for (let parent = node.parentNode; isElement(parent); parent = parent.parentNode) if (matchParts(parent, parts, index - 1, scope)) return true;
    return false;
  }
  const siblings = node.parentNode ? node.parentNode.children : [];
  const at = siblings.indexOf(node);
  if (combinator === '+') return at > 0 && matchParts(siblings[at - 1], parts, index - 1, scope);
  return siblings.slice(0, Math.max(0, at)).some(sibling => matchParts(sibling, parts, index - 1, scope));
}

function matchesList(node, text, scope) {
  return selectorList(text).some(parts => matchParts(node, parts, parts.length - 1, scope));
}

function descendants(root, out = []) {
  for (const child of root.children) { out.push(child); descendants(child, out); }
  return out;
}

// ---- Events --------------------------------------------------------------

const listenerMethods = {
  addEventListener(type, callback, options) {
    if (!callback) return;
    const capture = typeof options === 'boolean' ? options : Boolean(options && options.capture);
    const once = Boolean(options && typeof options === 'object' && options.once);
    if (!this.handlers) this.handlers = new Map();
    if (!this.handlers.has(type)) this.handlers.set(type, []);
    const list = this.handlers.get(type);
    if (!list.some(entry => entry.callback === callback && entry.capture === capture)) list.push({ callback, capture, once });
  },
  removeEventListener(type, callback, options) {
    const capture = typeof options === 'boolean' ? options : Boolean(options && options.capture);
    const list = this.handlers?.get(type);
    if (!list) return;
    const at = list.findIndex(entry => entry.callback === callback && entry.capture === capture);
    if (at >= 0) list.splice(at, 1);
  },
};

function invoke(node, event, phase) {
  const list = node.handlers?.get(event.type);
  if (!list) return;
  event.currentTarget = node;
  for (const entry of [...list]) {
    if (phase === 'capture' && !entry.capture) continue;
    if (phase === 'bubble' && entry.capture) continue;
    if (!list.includes(entry)) continue;
    if (entry.once) list.splice(list.indexOf(entry), 1);
    if (typeof entry.callback === 'function') entry.callback.call(node, event);
    else entry.callback.handleEvent(event);
    if (event.immediateStopped) return;
  }
}

export function dispatch(target, type, init = {}) {
  const document = target.nodeType === 9 ? target : target.document;
  const path = [];
  for (let node = target; node; node = node.parentNode) path.push(node);
  if (isElement(target) && target.isConnected) path.push(document, document.defaultView);
  else if (target === document) path.push(document.defaultView);
  const event = {
    type, target, currentTarget: null, bubbles: init.bubbles ?? !NON_BUBBLING.has(type), cancelable: true,
    defaultPrevented: false, stopped: false, immediateStopped: false, isTrusted: true, detail: 0, button: 0, pointerId: 1, pointerType: 'mouse',
    ...init,
    preventDefault() { this.defaultPrevented = true; },
    stopPropagation() { this.stopped = true; },
    stopImmediatePropagation() { this.stopped = true; this.immediateStopped = true; },
    composedPath() { return path.slice(); },
  };
  for (const node of path.slice(1).reverse()) { invoke(node, event, 'capture'); if (event.stopped) return event; }
  invoke(target, event, 'target');
  if (event.stopped || !event.bubbles) return event;
  for (const node of path.slice(1)) { invoke(node, event, 'bubble'); if (event.stopped) break; }
  return event;
}

// ---- Geometry ------------------------------------------------------------

class Matrix {
  constructor(a = 1, b = 0, c = 0, d = 1, e = 0, f = 0) { Object.assign(this, { a, b, c, d, e, f }); }
  multiply(m) { return new Matrix(this.a * m.a + this.c * m.b, this.b * m.a + this.d * m.b, this.a * m.c + this.c * m.d, this.b * m.c + this.d * m.d, this.a * m.e + this.c * m.f + this.e, this.b * m.e + this.d * m.f + this.f); }
  inverse() { const det = this.a * this.d - this.b * this.c || 1; return new Matrix(this.d / det, -this.b / det, -this.c / det, this.a / det, (this.c * this.f - this.d * this.e) / det, (this.b * this.e - this.a * this.f) / det); }
  translate(x = 0, y = 0) { return this.multiply(new Matrix(1, 0, 0, 1, x, y)); }
  scale(value = 1) { return this.multiply(new Matrix(value, 0, 0, value, 0, 0)); }
}
const point = (x = 0, y = 0) => ({ x, y, matrixTransform(m) { return point(m.a * this.x + m.c * this.y + m.e, m.b * this.x + m.d * this.y + m.f); } });
const rect = (x, y, width, height) => ({ x, y, left: x, top: y, right: x + width, bottom: y + height, width, height });

function polyline(element) {
  const numbers = (element.getAttribute('d') || element.getAttribute('points') || '').match(/-?\d*\.?\d+(?:e[-+]?\d+)?/gi) || [];
  const points = [];
  for (let index = 0; index + 1 < numbers.length; index += 2) points.push({ x: Number(numbers[index]), y: Number(numbers[index + 1]) });
  if (element.localName === 'line') return [{ x: Number(element.getAttribute('x1') || 0), y: Number(element.getAttribute('y1') || 0) }, { x: Number(element.getAttribute('x2') || 0), y: Number(element.getAttribute('y2') || 0) }];
  return points;
}

// ---- Nodes ---------------------------------------------------------------

function readerStyle() {
  const values = new Map(), priorities = new Map();
  const dash = name => String(name).startsWith('--') ? String(name) : String(name).replace(/[A-Z]/g, letter => '-' + letter.toLowerCase());
  const methods = {
    getPropertyValue: name => values.get(name) || '',
    getPropertyPriority: name => priorities.get(name) || '',
    setProperty: (name, value, priority) => { if (value === null || value === undefined || value === '') { values.delete(name); priorities.delete(name); return; } values.set(name, String(value)); if (priority) priorities.set(name, priority); else priorities.delete(name); },
    removeProperty: name => { const value = values.get(name) || ''; values.delete(name); priorities.delete(name); return value; },
  };
  return new Proxy(methods, {
    get(target, key) {
      if (key === 'cssText') return [...values].map(([name, value]) => `${name}: ${value}${priorities.has(name) ? ' !' + priorities.get(name) : ''};`).join(' ');
      if (typeof key !== 'string') return undefined;
      return target[key] || values.get(dash(key)) || '';
    },
    set(_, key, value) {
      if (key === 'cssText') {
        values.clear(); priorities.clear();
        for (const item of String(value).split(';')) {
          const at = item.indexOf(':');
          if (at < 0) continue;
          let text = item.slice(at + 1).trim();
          const important = /!\s*important$/i.test(text);
          if (important) text = text.replace(/!\s*important$/i, '').trim();
          methods.setProperty(item.slice(0, at).trim(), text, important ? 'important' : '');
        }
      } else methods.setProperty(dash(key), value);
      return true;
    },
  });
}

class ReaderNode extends FixtureElement {
  constructor(document, tag, attributes = {}, text = '', svg = false) {
    super(tag, {}, text);
    this.style = readerStyle();
    this.document = document;
    this.namespaceURI = svg ? SVG_NS : HTML_NS;
    this.tagName = svg ? tag : tag.toUpperCase();
    this.scrollTop = 0; this.scrollLeft = 0;
    const base = this.classList;
    this.classList = { ...base,
      toggle: (name, force) => { const on = force === undefined ? !base.contains(name) : Boolean(force); if (on) base.add(name); else base.remove(name); return on; },
    };
    for (const [name, value] of Object.entries(attributes)) this.setAttribute(name, value);
  }
  get hidden() { return this.hasAttribute('hidden'); }
  set hidden(value) { if (!this.attrs) return; if (value) this.setAttribute('hidden', ''); else this.removeAttribute('hidden'); }
  get disabled() { return this.hasAttribute('disabled'); }
  set disabled(value) { if (!this.attrs) return; if (value) this.setAttribute('disabled', ''); else this.removeAttribute('disabled'); }
  get title() { return this.getAttribute('title') || ''; } set title(value) { this.setAttribute('title', value); }
  get href() { return this.getAttribute('href') ?? undefined; } set href(value) { this.setAttribute('href', value); }
  get rel() { return this.getAttribute('rel') || ''; } set rel(value) { this.setAttribute('rel', value); }
  get type() { return this.getAttribute('type') || ''; } set type(value) { this.setAttribute('type', value); }
  get download() { return this.getAttribute('download') || ''; } set download(value) { this.setAttribute('download', value); }
  get value() { return this.fixtureValue ?? this.getAttribute('value') ?? ''; } set value(value) { this.fixtureValue = String(value); }
  get childNodes() { return this.children; }
  get isConnected() { for (let node = this; node; node = node.parentNode) if (node === this.document?.documentElement) return true; return false; }
  get isContentEditable() { return false; }
  get offsetWidth() { return this.getBoundingClientRect().width; }
  get offsetHeight() { return this.getBoundingClientRect().height; }
  get clientWidth() { return this.getBoundingClientRect().width; }
  get clientHeight() { return this.getBoundingClientRect().height; }
  get offsetLeft() { return 0; }
  get offsetTop() { return 0; }
  get scrollWidth() { return this.clientWidth; }
  get scrollHeight() { return this.clientHeight; }
  setAttribute(name, value) {
    const old = this.getAttribute(name);
    super.setAttribute(name, value);
    if (name === 'viewBox') { const [x, y, width, height] = String(value).trim().split(/[\s,]+/).map(Number); this.viewBox = { baseVal: { x, y, width, height } }; }
    this.document?.mutated?.(this, name, old);
  }
  removeAttribute(name) {
    if (!this.hasAttribute(name)) return;
    const old = this.getAttribute(name);
    super.removeAttribute(name);
    this.document?.mutated?.(this, name, old);
  }
  toggleAttribute(name, force) { const on = force === undefined ? !this.hasAttribute(name) : Boolean(force); if (on) this.setAttribute(name, ''); else this.removeAttribute(name); return on; }
  appendChild(node) { const result = super.appendChild(node); this.document?.childList?.(this); return result; }
  insertBefore(node, before) { const result = super.insertBefore(node, before); this.document?.childList?.(this); return result; }
  removeChild(node) { const result = super.removeChild(node); this.document?.childList?.(this); return result; }
  matches(selector) { return matchesList(this, selector, this); }
  closest(selector) { for (let node = this; isElement(node); node = node.parentNode) if (matchesList(node, selector, node)) return node; return null; }
  querySelectorAll(selector) { const lists = selectorList(selector); return descendants(this).filter(node => lists.some(parts => matchParts(node, parts, parts.length - 1, this))); }
  querySelector(selector) { return this.querySelectorAll(selector)[0] || null; }
  cloneNode(deep) {
    const clone = new ReaderNode(this.document, this.namespaceURI === SVG_NS ? this.tagName : this.localName, Object.fromEntries(this.attrs), this.ownText, this.namespaceURI === SVG_NS);
    clone.style.cssText = this.style.cssText;
    if (deep) for (const child of this.children) clone.appendChild(child.cloneNode(true));
    return clone;
  }
  emit(type, extra = {}) { return dispatch(this, type, extra); }
  click() {
    const event = dispatch(this, 'click', { detail: 1 });
    if (this.localName === 'a' && this.hasAttribute('download') && !event.defaultPrevented) this.document.downloads.push({ filename: this.download, url: this.href, blob: this.document.objectUrls.get(this.href)?.blob, anchor: this });
  }
  focus() {
    const document = this.document, previous = document.activeElement;
    if (previous === this) return;
    document.activeElement = this;
    if (isElement(previous)) { dispatch(previous, 'blur', { relatedTarget: this }); dispatch(previous, 'focusout', { relatedTarget: this }); }
    dispatch(this, 'focus', { relatedTarget: previous }); dispatch(this, 'focusin', { relatedTarget: previous });
  }
  blur() { if (this.document.activeElement !== this) return; this.document.activeElement = this.document.body; dispatch(this, 'blur'); dispatch(this, 'focusout'); }
  select() {} scrollTo() {} setPointerCapture() {} releasePointerCapture() {}
  getBoundingClientRect() {
    if (this.hidden || !this.isConnected) return rect(0, 0, 0, 0);
    if (this.localName === 'html' || this.localName === 'body') return rect(0, 0, 1440, 900);
    if (this.classList.contains('diagram-container')) return rect(32, 58, 1376, 820);
    if (this.namespaceURI === SVG_NS) { const box = this.getBBox(); return rect(box.x, box.y, box.width, box.height); }
    return rect(0, 0, 0, 0);
  }
  getBBox() {
    const number = name => Number(this.getAttribute(name) || 0);
    if (this.localName === 'svg') { const box = this.viewBox?.baseVal || { x: 0, y: 0, width: 0, height: 0 }; return { ...box }; }
    if (this.localName === 'rect' || this.localName === 'image' || this.localName === 'foreignobject') return { x: number('x'), y: number('y'), width: number('width'), height: number('height') };
    if (this.localName === 'circle') { const r = number('r'); return { x: number('cx') - r, y: number('cy') - r, width: 2 * r, height: 2 * r }; }
    if (this.localName === 'text' || this.localName === 'tspan') return { x: number('x'), y: number('y') - 10, width: this.textContent.length * 6, height: 12 };
    if (['path', 'line', 'polyline', 'polygon'].includes(this.localName)) {
      const points = polyline(this);
      if (!points.length) return { x: 0, y: 0, width: 0, height: 0 };
      const xs = points.map(p => p.x), ys = points.map(p => p.y);
      return { x: Math.min(...xs), y: Math.min(...ys), width: Math.max(...xs) - Math.min(...xs), height: Math.max(...ys) - Math.min(...ys) };
    }
    const boxes = this.children.filter(child => child.namespaceURI === SVG_NS && !['title', 'desc', 'style', 'defs', 'marker'].includes(child.localName)).map(child => child.getBBox()).filter(box => box.width || box.height);
    if (!boxes.length) return { x: 0, y: 0, width: 0, height: 0 };
    const left = Math.min(...boxes.map(b => b.x)), top = Math.min(...boxes.map(b => b.y));
    return { x: left, y: top, width: Math.max(...boxes.map(b => b.x + b.width)) - left, height: Math.max(...boxes.map(b => b.y + b.height)) - top };
  }
  getTotalLength() {
    const points = polyline(this);
    let length = 0;
    for (let index = 1; index < points.length; index++) length += Math.hypot(points[index].x - points[index - 1].x, points[index].y - points[index - 1].y);
    return length;
  }
  getPointAtLength(distance) {
    const points = polyline(this);
    if (!points.length) return point();
    let remaining = Math.max(0, distance);
    for (let index = 1; index < points.length; index++) {
      const a = points[index - 1], b = points[index], step = Math.hypot(b.x - a.x, b.y - a.y);
      if (remaining <= step && step > 0) return point(a.x + (b.x - a.x) * remaining / step, a.y + (b.y - a.y) * remaining / step);
      remaining -= step;
    }
    return point(points.at(-1).x, points.at(-1).y);
  }
  getCTM() { return new Matrix(); }
  getScreenCTM() { return new Matrix(); }
  createSVGPoint() { return point(); }
}
Object.assign(ReaderNode.prototype, listenerMethods);

// ---- Markup --------------------------------------------------------------

function parseMarkup(document, markup, parent) {
  const stack = [{ node: parent, svg: parent.namespaceURI === SVG_NS }];
  const lower = markup.toLowerCase();
  const token = /<!--[\s\S]*?-->|<!doctype[^>]*>|<\/([a-zA-Z][\w:-]*)\s*>|<([a-zA-Z][\w:-]*)((?:\s+[^\s"'>/=]+(?:\s*=\s*(?:"[^"]*"|'[^']*'|[^\s"'=<>`]+))?)*)\s*(\/?)>|[^<]+|</gi;
  let match;
  while ((match = token.exec(markup))) {
    const [whole, closing, opening, rawAttributes, selfClosing] = match;
    const top = stack.at(-1);
    if (whole.startsWith('<!')) continue;
    if (closing) {
      const name = closing.toLowerCase();
      const at = stack.findLastIndex(entry => entry.node.localName === name);
      if (at > 0) stack.length = at;
      continue;
    }
    if (opening) {
      const attributes = {};
      for (const [, name, double, single, bare] of rawAttributes.matchAll(/([^\s"'>/=]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?/g)) attributes[name] = decode(double ?? single ?? bare ?? '');
      const svg = top.svg || opening.toLowerCase() === 'svg';
      const node = new ReaderNode(document, svg ? opening : opening.toLowerCase(), attributes, '', svg);
      top.node.appendChild(node);
      const name = opening.toLowerCase();
      if (RAW.has(name) && !selfClosing) {
        const end = lower.indexOf('</' + name, token.lastIndex);
        if (end < 0) throw new Error('Unterminated ' + name);
        node.ownText = name === 'title' || name === 'textarea' ? decode(markup.slice(token.lastIndex, end)) : markup.slice(token.lastIndex, end);
        token.lastIndex = markup.indexOf('>', end) + 1;
        continue;
      }
      if (!selfClosing && !(VOID.has(name) && !svg)) stack.push({ node, svg: svg && name !== 'foreignobject' });
      continue;
    }
    const text = decode(whole);
    if (text.trim()) top.node.ownText += text;
  }
  return parent;
}

function slot(source, name, value) {
  const marker = '{{PHOENIX:' + name + '}}';
  const parts = source.split(marker);
  if (parts.length !== 2) throw new Error('Template must contain exactly one ' + marker);
  return parts.join(value);
}

const escapeText = value => String(value).replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');

function between(source, start, end, from = 0) {
  const at = source.indexOf(start, from);
  if (at < 0) throw new Error('Missing template boundary ' + start);
  const to = source.indexOf(end, at + start.length);
  if (to < 0) throw new Error('Missing template boundary ' + end);
  return source.slice(at + start.length, to);
}

// ---- Boot ----------------------------------------------------------------

export function bootReader(options = {}) {
  const metadata = options.metadata || {};
  const meta = metadata.meta || {};
  const title = meta.title || 'Fixture Diagram';
  let html = templateSource;
  for (const [name, value] of [
    ['LOCALE', escapeText(options.lang || 'en')], ['PRESET', escapeText(meta.visual_preset || 'classic')], ['TITLE', escapeText(title)], ['HEADING', escapeText(title)],
    ['SUBTITLE', meta.subtitle ? '<p class="subtitle">' + escapeText(meta.subtitle) + '</p>' : ''], ['SVG', options.svg],
    ['CARDS', '<div class="cards"></div>'],
    ['METADATA', JSON.stringify(metadata).replaceAll('<', '\\u003c').replaceAll('>', '\\u003e').replaceAll('&', '\\u0026')],
  ]) html = slot(html, name, value);

  const clock = new Clock();
  const objectUrls = new Map(), downloads = [], alerts = [], errors = [], serialized = [], drawn = [];
  const storage = new Map();
  const observers = [];
  const media = { '(prefers-reduced-motion: reduce)': false, print: false, '(prefers-color-scheme: light)': false, '(hover: hover) and (pointer: fine)': true };
  const document = Object.assign(Object.create(listenerMethods), {
    nodeType: 9, hidden: false, objectUrls, downloads,
    fonts: { ready: Promise.resolve(), status: 'loaded' },
    styleSheets: [{ cssRules: [{ type: 1, selectorText: ':root', cssText: ':root { --bg: #101827; --text: #ffffff; }', style: { cssText: '--bg: #101827; --text: #ffffff;' } }] }],
    execCommand: () => true,
  });
  const element = (tag, attributes, svg) => new ReaderNode(document, tag, attributes, '', svg);
  const root = element('html', {});
  const htmlTag = html.match(/<html([^>]*)>/i);
  for (const [, name, value] of htmlTag[1].matchAll(/([^\s"'>/=]+)="([^"]*)"/g)) root.setAttribute(name, decode(value));
  document.documentElement = root;
  const headStart = html.indexOf('<head>'), mainScript = html.indexOf('\n  <script>\n    var PhoenixDiagram');
  if (headStart < 0 || mainScript < 0) throw new Error('Unexpected assembled template layout');
  const shell = element('div', {});
  parseMarkup(document, html.slice(headStart, mainScript) + '</body>', shell);
  for (const child of [...shell.children]) root.appendChild(child);
  document.head = root.querySelector('head');
  document.body = root.querySelector('body');
  document.activeElement = document.body;
  document.title = title;
  Object.defineProperty(document, 'defaultView', { get: () => window });
  document.getElementById = id => root.querySelector('#' + id.replace(/([^\w-])/g, '\\$1'));
  document.querySelector = selector => document.querySelectorAll(selector)[0] || null;
  document.querySelectorAll = selector => { const lists = selectorList(selector); return [root, ...descendants(root)].filter(node => lists.some(parts => matchParts(node, parts, parts.length - 1, root))); };
  document.createElement = tag => {
    if (String(tag).toLowerCase() === 'canvas') return new Canvas();
    return element(String(tag).toLowerCase(), {});
  };
  document.createElementNS = (namespace, tag) => element(tag, {}, namespace === SVG_NS);
  document.mutated = (target, attributeName, oldValue) => {
    for (const observer of observers) for (const entry of observer.entries) {
      if (!entry.options.attributes && !entry.options.attributeFilter) continue;
      if (entry.options.attributeFilter && !entry.options.attributeFilter.includes(attributeName)) continue;
      if (entry.target !== target && !(entry.options.subtree && entry.target.contains(target))) continue;
      observer.queue({ type: 'attributes', target, attributeName, oldValue });
    }
  };
  document.childList = target => {
    for (const observer of observers) for (const entry of observer.entries) {
      if (!entry.options.childList) continue;
      if (entry.target !== target && !(entry.options.subtree && entry.target.contains(target))) continue;
      observer.queue({ type: 'childList', target });
    }
  };

  class Canvas extends ReaderNode {
    constructor() {
      super(document, 'canvas', {});
      this.width = 300; this.height = 150;
      this.context = new Proxy({ canvas: this, measureText: text => ({ width: String(text).length * 8 }), fillText: text => { drawn.push(String(text)); } },
        { get: (target, key) => key in target ? target[key] : (() => {}), set: (target, key, value) => { target[key] = value; return true; } });
    }
    getContext() { return this.context; }
    toDataURL(mime = 'image/png') { return 'data:' + mime + ';base64,fixture'; }
    toBlob(callback, mime = 'image/png') { queueMicrotask(() => callback(new Blob(['fixture raster bytes'], { type: mime }))); }
    captureStream(fps) { const track = { state: 'live', stop() { this.state = 'ended'; } }; return { fps, getTracks: () => [track] }; }
  }
  class Image {
    constructor() { this.onload = null; this.onerror = null; }
    set src(value) { this.url = value; clock.timeout(() => { this.onload?.(); }, 0, 'image'); }
    get src() { return this.url; }
  }
  class MediaRecorder {
    static isTypeSupported() { return true; }
    constructor(stream, config) { this.stream = stream; this.mimeType = config.mimeType; this.state = 'inactive'; }
    start() { this.state = 'recording'; }
    requestData() {}
    stop() { this.state = 'inactive'; queueMicrotask(() => { this.ondataavailable?.({ data: new Blob(['fixture encoded bytes'], { type: this.mimeType }) }); this.onstop?.(); }); }
  }
  class MutationObserver {
    constructor(callback) { this.callback = callback; this.entries = []; this.records = []; observers.push(this); }
    observe(target, options = {}) { this.entries.push({ target, options }); }
    disconnect() { this.entries = []; this.records = []; }
    takeRecords() { const records = this.records; this.records = []; return records; }
    queue(record) {
      this.records.push(record);
      if (this.records.length > 1) return;
      queueMicrotask(() => { const records = this.takeRecords(); if (records.length) this.callback(records, this); });
    }
  }
  class ResizeObserver { observe() {} unobserve() {} disconnect() {} }
  class URLWithObjects extends URL {
    static createObjectURL(blob) { const url = 'blob:fixture/' + (objectUrls.size + 1); objectUrls.set(url, { blob, revocations: 0 }); return url; }
    static revokeObjectURL(url) { if (objectUrls.has(url)) objectUrls.get(url).revocations++; }
  }
  const mediaQuery = query => Object.assign(Object.create(listenerMethods), { media: query, get matches() { return Boolean(media[query]); },
    addListener(callback) { this.addEventListener('change', callback); }, removeListener(callback) { this.removeEventListener('change', callback); } });
  const computed = element => {
    const defaults = { display: element?.hidden ? 'none' : 'block', visibility: 'visible', opacity: '1', position: 'static', transform: 'none', overflow: 'visible', overflowX: 'visible', overflowY: 'visible',
      fontFamily: 'sans-serif', fontSize: '16px', lineHeight: 'normal', backgroundColor: '#101827', backgroundImage: 'none', stroke: '#22d3ee', strokeWidth: '1.5', fill: '#123456', fillOpacity: '1', strokeOpacity: '1' };
    const variables = { '--bg': root.getAttribute('data-theme') === 'light' ? '#ffffff' : '#101827', '--text': '#ffffff', '--text-muted': '#94a3b8', '--panel': '#1a2435', '--panel-border': '#334155',
      '--frontend-stroke': '#22d3ee', '--backend-stroke': '#34d399', '--database-stroke': '#a78bfa' };
    return new Proxy({}, { get(_, key) {
      if (key === 'getPropertyValue') return name => element?.style?.getPropertyValue?.(name) || variables[name] || '';
      if (typeof key !== 'string') return undefined;
      return element?.style?.getPropertyValue?.(key.replace(/[A-Z]/g, letter => '-' + letter.toLowerCase())) || defaults[key] || '';
    } });
  };

  let url = new URL('file:///fixture/diagram.html' + (options.search || '') + (options.hash || ''));
  const windowTarget = Object.create(listenerMethods);
  const location = {
    get href() { return url.href; }, get pathname() { return url.pathname; }, get search() { return url.search; }, get hash() { return url.hash; },
    set hash(value) { const before = url.href; url.hash = value; if (url.href !== before) clock.timeout(() => dispatch(windowTarget, 'hashchange'), 0, 'hashchange'); },
  };
  // replaceState never fires hashchange, as in browsers.
  const history = { replaceState(_, __, value) { if (value !== undefined && value !== null) url = new URL(value, url.href); } };
  const activation = { active: false };
  const window = Object.assign(windowTarget, {
    document, location, history, innerWidth: 1440, innerHeight: 900, scrollX: 0, scrollY: 0,
    navigator: { clipboard: { writeText: () => Promise.resolve(), write: () => Promise.resolve() }, userActivation: { get isActive() { return activation.active; } } },
    localStorage: { getItem: key => storage.get(key) ?? null, setItem: (key, value) => { storage.set(key, String(value)); }, removeItem: key => { storage.delete(key); } },
    matchMedia: mediaQuery, getComputedStyle: computed, scrollTo() {},
    requestAnimationFrame: callback => clock.timeout(() => callback(clock.now), 16, 'frame'), cancelAnimationFrame: id => clock.clear(id),
    setTimeout: (callback, delay) => clock.timeout(callback, delay), clearTimeout: id => clock.clear(id),
    performance: { now: () => clock.now }, queueMicrotask,
    console: { error: (...args) => errors.push(args), warn: (...args) => errors.push(args), log() {}, info() {}, debug() {} },
    alert: message => alerts.push(String(message)),
    URL: URLWithObjects, URLSearchParams, Blob, Image, MediaRecorder, MutationObserver, ResizeObserver, ClipboardItem: class { constructor(data) { this.data = data; } },
    HTMLCanvasElement: Canvas, Node: FixtureElement,
    XMLSerializer: class { serializeToString(node) { serialized.push(node); return node.outerHTML; } },
  });
  // The window is the script global, so its listener methods must be own properties.
  window.addEventListener = listenerMethods.addEventListener.bind(window);
  window.removeEventListener = listenerMethods.removeEventListener.bind(window);
  window.window = window;
  const context = vm.createContext(window);
  vm.runInContext(between(html, '<script>', '</script>', headStart), context, { filename: 'template.html#head' });
  vm.runInContext(between(html, '<script>', '</script>\n</body>', mainScript), context, { filename: 'template.html#reader' });
  dispatch(document, 'DOMContentLoaded');
  dispatch(windowTarget, 'load');

  const resolve = target => typeof target === 'string' ? document.querySelector(target) : target;
  const env = {
    window, document, html: root, clock, downloads, alerts, errors, serialized, drawn,
    get api() { return window.PhoenixDiagram; },
    get svg() { return document.querySelector('.diagram-container > svg'); },
    get hash() { return url.hash; },
    get href() { return url.href; },
    $: selector => document.querySelector(selector),
    $$: selector => document.querySelectorAll(selector),
    node: id => document.querySelector('[data-node-id="' + id + '"]'),
    async settle(ms = 40) { await clock.advance(ms); },
    click(target, init = {}) {
      const node = resolve(target);
      if (!node) throw new Error('No click target ' + target);
      activation.active = true;
      try { dispatch(node, 'pointerdown', init); dispatch(node, 'pointerup', init); return dispatch(node, 'click', { detail: 1, ...init }); }
      finally { activation.active = false; }
    },
    key(key, target = document.activeElement || document.body, init = {}) { return dispatch(resolve(target), 'keydown', { key, ...init }); },
    async navigate(hash) { location.hash = hash; await clock.advance(40); },
    setMedia(query, matches) { media[query] = matches; },
  };
  return env;
}
