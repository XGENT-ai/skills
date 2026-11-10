//! Scripts `capture` runs in the page. Kept as Rust constants: `.js` files
//! under `crates/` are inventoried as upstream Impeccable sources.

/// `function(args, ...nodes)`: find the element a locator names and report
/// where an action would land. `nodes` are the accessibility-tree matches
/// for a role locator. `args`: `{ loc, scroll, pointer, op }`.
pub const LOCATE: &str = r#"function (args, ...nodes) {
  const loc = args.loc;
  const norm = (t) => String(t || '').trim().replace(/\s+/g, ' ');
  const skip = (e) => !!e.closest('script,style,template,noscript,head');
  const INTERACTIVE = 'a,button,input,select,textarea,summary,label,[role],[tabindex]';
  const docOrder = (a, b) => (a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING ? -1 : 1);
  const hidden = (el) => {
    for (let e = el; e; e = e.parentElement) {
      if (getComputedStyle(e).display === 'none') return '隐藏（' + (e === el ? '' : '祖先 ') + 'display:none）';
    }
    const style = getComputedStyle(el);
    if (style.visibility === 'hidden' || style.visibility === 'collapse') return '隐藏（visibility:' + style.visibility + '）';
    for (let e = el; e; e = e.parentElement) {
      if (Number(getComputedStyle(e).opacity) === 0) return '透明（' + (e === el ? '' : '祖先 ') + 'opacity:0）';
    }
    const r = el.getBoundingClientRect();
    if (!r.width || !r.height) return '零尺寸（' + r.width + '×' + r.height + '）';
    return null;
  };
  const textOf = (e) => norm(e.innerText || e.getAttribute('aria-label') || e.value || '');
  let all;
  if (loc.css !== undefined) {
    try { all = [...document.querySelectorAll(loc.css)]; }
    catch (error) { return { status: 'invalid', reason: '不是合法的 CSS 选择器：' + loc.css }; }
  } else if (loc.text !== undefined) {
    const wanted = norm(loc.text);
    const pool = [...document.body.querySelectorAll('*')].filter((e) => !skip(e));
    const pick = (test) => {
      const matched = pool.filter((e) => test(norm(e.innerText)));
      const inner = matched.filter((e) => !matched.some((o) => o !== e && e.contains(o)));
      const lifted = inner.map((e) => {
        const a = e.closest(INTERACTIVE);
        return a && test(norm(a.innerText)) ? a : e;
      });
      return [...new Set(lifted)].sort(docOrder);
    };
    const exact = pick((t) => t === wanted);
    const contains = pick((t) => t.includes(wanted));
    const anyVisible = (list) => list.some((e) => !hidden(e));
    all = anyVisible(exact) ? exact : anyVisible(contains) ? contains : exact.length ? exact : contains;
  } else {
    all = nodes.filter((e) => e instanceof Element && e.getRootNode() === document);
  }
  const visible = all.filter((e) => !hidden(e));
  const nth = loc.nth || 1;
  if (visible.length < nth) {
    if (all.length) {
      return { status: 'hidden', count: all.length, reason: visible.length
        ? '只有 ' + visible.length + ' 个可见匹配，没有第 ' + nth + ' 个'
        : '找到 ' + all.length + ' 个匹配，但都不可见：' + hidden(all[0]) };
    }
    const hints = [];
    if (loc.text !== undefined) {
      const wanted = norm(loc.text);
      const words = wanted.split(' ').filter(Boolean);
      const related = (t) => t.includes(wanted) || wanted.includes(t) || words.some((w) => t.includes(w) || w.includes(t));
      const seen = new Set();
      for (const e of document.body.querySelectorAll(INTERACTIVE)) {
        if (skip(e) || hidden(e)) continue;
        const t = textOf(e);
        if (!t || t.length > 60 || seen.has(t) || !related(t)) continue;
        seen.add(t);
        hints.push(JSON.stringify({ text: t }));
        if (hints.length >= 3) break;
      }
    }
    let iframes = 0, shadowRoots = 0;
    const roots = [document];
    for (let i = 0; i < roots.length && i < 200; i++) {
      for (const e of roots[i].querySelectorAll('*')) {
        if (e.localName === 'iframe' || e.localName === 'frame') iframes++;
        if (e.shadowRoot) { shadowRoots++; roots.push(e.shadowRoot); }
      }
    }
    return { status: 'missing', hints, iframes, shadowRoots };
  }
  const el = visible[nth - 1];
  if (args.scroll) {
    const r = el.getBoundingClientRect();
    if (r.top < 0 || r.left < 0 || r.bottom > innerHeight || r.right > innerWidth) {
      el.scrollIntoView({ block: 'center', inline: 'center', behavior: 'instant' });
    }
  }
  const r = el.getBoundingClientRect();
  if (args.scroll && (r.bottom <= 0 || r.right <= 0 || r.top >= innerHeight || r.left >= innerWidth)) {
    return { status: 'hidden', count: all.length, reason: '在视口外，滚动后仍不在视口内' };
  }
  const x = Math.max(1, Math.min(innerWidth - 1, r.left + r.width / 2));
  const y = Math.max(1, Math.min(innerHeight - 1, r.top + r.height / 2));
  const warnings = [];
  if (args.pointer) {
    const top = document.elementFromPoint(x, y);
    if (top && top !== el && !el.contains(top)) {
      const name = top.localName + (top.id ? '#' + top.id : '') + [...top.classList].slice(0, 2).map((c) => '.' + c).join('');
      warnings.push('目标中心被 ' + name + ' 遮挡，动作照常执行');
    }
  }
  if (args.op) {
    const disabled = [];
    if (el.matches(':disabled')) disabled.push('disabled');
    if ((el.getAttribute('aria-disabled') || '').toLowerCase() === 'true') disabled.push('aria-disabled="true"');
    if (getComputedStyle(el).pointerEvents === 'none') disabled.push('pointer-events:none');
    if (disabled.length) warnings.push('目标处于禁用状态（' + disabled.join('、') + '），动作照常执行');
  }
  let observed = null;
  if (args.op === 'fill') {
    el.focus();
    // Typed text never reaches the segmented editors of these inputs; set the value the way a picker would.
    if (el instanceof HTMLInputElement && /^(date|time|datetime-local|month|week|color|range)$/.test(el.type)) {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(el, args.value);
      if (el.value !== args.value) return { status: 'invalid', reason: '<input type="' + el.type + '"> 不接受这个值，需用它的值格式（如 date 2026-11-20、time 09:30、color #2457c5）' };
      el.dispatchEvent(new Event('input', { bubbles: true }));
      el.dispatchEvent(new Event('change', { bubbles: true }));
      observed = '原生 <input type="' + el.type + '"> 直接设值';
    } else if (typeof el.select === 'function') el.select();
    else if (el.isContentEditable) {
      const range = document.createRange();
      range.selectNodeContents(el);
      const selection = getSelection();
      selection.removeAllRanges();
      selection.addRange(range);
    }
  } else if (args.op === 'select') {
    if (!(el instanceof HTMLSelectElement)) return { status: 'invalid', reason: 'select 只能用于原生 <select>，目标是 <' + el.localName + '>' };
    const options = [...el.options];
    const option = options.find((o) => o.value === args.value) || options.find((o) => norm(o.textContent) === norm(args.value));
    if (!option) return { status: 'invalid', reason: '找不到选项 ' + JSON.stringify(args.value) + '（可选：' + options.slice(0, 8).map((o) => norm(o.textContent)).join('、') + '）' };
    el.value = option.value;
    el.dispatchEvent(new Event('input', { bubbles: true }));
    el.dispatchEvent(new Event('change', { bubbles: true }));
    observed = '选中「' + norm(option.textContent) + '」';
  } else if (args.op === 'scroll') {
    el.scrollBy({ left: args.dx || 0, top: args.dy || 0, behavior: 'instant' });
  }
  return {
    status: 'ok', x, y, count: visible.length, warnings, observed,
    doc: { x: r.left + scrollX, y: r.top + scrollY, width: r.width, height: r.height },
  };
}"#;

/// `function(args, ...nodes)`: visible, main-document accessibility matches
/// of a role, ranked by how close their names are to `args.want`.
pub const ROLE_HINTS: &str = r#"function (args, ...nodes) {
  const norm = (t) => String(t || '').trim().replace(/\s+/g, ' ');
  const want = norm(args.want);
  const visible = (e) => {
    if (!(e instanceof Element) || e.getRootNode() !== document) return false;
    const r = e.getBoundingClientRect();
    return r.width > 0 && r.height > 0 && e.checkVisibility({ checkOpacity: true, checkVisibilityCSS: true });
  };
  const score = (n) => {
    if (!want) return 0;
    if (n.includes(want) || want.includes(n)) return 3;
    const words = want.split(' ');
    if (words.some((w) => w && (n.includes(w) || w.includes(n)))) return 2;
    return [...new Set(want)].filter((c) => c.trim() && n.includes(c)).length / want.length;
  };
  const names = [];
  nodes.forEach((e, i) => {
    const name = norm(args.names[i]);
    if (name && visible(e) && !names.includes(name)) names.push(name);
  });
  return names.map((n, i) => [score(n), -i, n]).sort((a, b) => b[0] - a[0] || b[1] - a[1]).map((x) => x[2]).slice(0, 3);
}"#;

/// `async (limitMs)`: wait until no finite animation is running (or the
/// budget ends), then two frames. Resolves to the finite animations left.
pub const SETTLE: &str = r#"async (limit) => {
  const finite = () => document.getAnimations().filter((a) => (a.playState === 'running' || a.pending)
    && a.effect && Number.isFinite(a.effect.getComputedTiming().endTime));
  const start = performance.now();
  while (finite().length && performance.now() - start < limit) {
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  return finite().length;
}"#;

/// Running finite animations right now (for the sequence-frame window).
pub const FINITE_ANIMATIONS: &str = r#"document.getAnimations().filter((a) => (a.playState === 'running' || a.pending)
  && a.effect && Number.isFinite(a.effect.getComputedTiming().endTime)).length"#;

/// Note that a CSS animation or transition started (they may end before a
/// poll sees them). Installed before navigation for entrance frames.
pub const MOTION_COUNTER: &str = r#"(() => {
  if (!window.__phoenixMotion) {
    window.__phoenixMotion = { seen: false };
    const seen = () => { window.__phoenixMotion.seen = true; };
    addEventListener('animationstart', seen, true);
    addEventListener('transitionrun', seen, true);
  }
  return true;
})()"#;

pub const MOTION_SEEN: &str = "!!(window.__phoenixMotion && window.__phoenixMotion.seen)";

/// The `type` of a focused native input that typed text cannot reach, or `""`.
pub const FOCUSED_NATIVE_INPUT: &str = r#"(() => { const e = document.activeElement; return e instanceof HTMLInputElement && /^(date|time|datetime-local|month|week|color|range)$/.test(e.type) ? e.type : ''; })()"#;

pub const TWO_FRAMES: &str = r#"new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve(true))))"#;

/// `(region)`: one consistent measurement of the area a screenshot covers.
/// `region` is `[x, y, width, height]` in document CSS pixels, `'full'` for
/// the whole viewport-wide document, or null for the current viewport.
pub const MEASURE: &str = r#"(region) => {
  const root = document.documentElement;
  const docWidth = Math.max(root.scrollWidth, document.body ? document.body.scrollWidth : 0);
  const docHeight = Math.max(root.scrollHeight, document.body ? document.body.scrollHeight : 0);
  const [rx, ry, rw, rh] = region === 'full' ? [0, 0, innerWidth, docHeight] : region || [scrollX, scrollY, innerWidth, innerHeight];
  const overlaps = (x, y, w, h) => w > 0 && h > 0 && x < rx + rw && x + w > rx && y < ry + rh && y + h > ry;
  const opacity = new Map();
  const cumulative = (el) => {
    if (!el) return 1;
    if (opacity.has(el)) return opacity.get(el);
    const value = Number(getComputedStyle(el).opacity) * cumulative(el.parentElement);
    opacity.set(el, value);
    return value;
  };
  let textChars = 0, transparentChars = 0;
  if (document.body) {
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      const parent = node.parentElement;
      const chars = node.data.replace(/\s+/g, '').length;
      if (!chars || !parent || parent.closest('script,style,template,noscript')) continue;
      if (getComputedStyle(parent).visibility !== 'visible') continue;
      const range = document.createRange();
      range.selectNodeContents(node);
      const inside = [...range.getClientRects()].some((r) => overlaps(r.left + scrollX, r.top + scrollY, r.width, r.height));
      if (!inside) continue;
      textChars += chars;
      if (cumulative(parent) < 0.05) transparentChars += chars;
    }
  }
  const path = (el) => {
    if (el.id) return '#' + CSS.escape(el.id);
    const parts = [];
    for (let e = el; e && e !== root; e = e.parentElement) {
      const same = e.parentElement ? [...e.parentElement.children].filter((s) => s.localName === e.localName) : [e];
      parts.unshift(e.localName + (same.length > 1 ? ':nth-of-type(' + (same.indexOf(e) + 1) + ')' : ''));
      if (e.id) { parts[0] = '#' + CSS.escape(e.id); break; }
    }
    return parts.join(' > ');
  };
  const canvases = [...document.querySelectorAll('canvas')].flatMap((c) => {
    const r = c.getBoundingClientRect();
    const x = r.left + scrollX, y = r.top + scrollY;
    if (r.width < 32 || r.height < 32 || !overlaps(x, y, r.width, r.height)) return [];
    if (!c.checkVisibility({ checkOpacity: true, checkVisibilityCSS: true })) return [];
    return [{ selector: path(c), x: x - rx, y: y - ry, width: r.width, height: r.height }];
  });
  const failedImages = [...document.images]
    .filter((i) => i.complete && i.naturalWidth === 0 && (i.getAttribute('src') || i.currentSrc))
    .map((i) => String(i.currentSrc || i.src || i.getAttribute('src')).slice(0, 300));
  const parse = (c) => {
    const m = String(c).match(/rgba?\(([^)]+)\)/);
    if (!m) return null;
    const p = m[1].split(/[ ,/]+/).filter(Boolean).map(Number);
    return { rgb: p.slice(0, 3), a: p.length > 3 ? p[3] : 1 };
  };
  let background = parse(getComputedStyle(root).backgroundColor);
  if (!background || background.a === 0) background = document.body ? parse(getComputedStyle(document.body).backgroundColor) : null;
  let luminance;
  if (background && background.a > 0) {
    const lin = (v) => { v /= 255; return v <= 0.04045 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4); };
    const [r, g, b] = background.rgb.map(lin);
    luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
  } else {
    const scheme = getComputedStyle(root).colorScheme || '';
    const dark = scheme.includes('dark') && (!scheme.includes('light') || matchMedia('(prefers-color-scheme: dark)').matches);
    luminance = dark ? 0.01 : 1;
  }
  const finite = document.getAnimations().filter((a) => (a.playState === 'running' || a.pending)
    && a.effect && Number.isFinite(a.effect.getComputedTiming().endTime)).length;
  return {
    finiteAnimations: finite, textChars, transparentChars, canvases, failedImages,
    rootLuminance: luminance, docWidth, docHeight, viewportWidth: innerWidth, viewportHeight: innerHeight,
    scrollX, scrollY,
    overflow: { overflow: root.scrollWidth > innerWidth + 1, scrollWidth: root.scrollWidth, viewportWidth: innerWidth },
  };
}"#;

/// Text masking for the mask variant. Colors stay: icons, borders and
/// decorations painted with currentColor must remain visible.
pub const MASK_CSS: &str = "*,*::before,*::after{text-shadow:none!important;-webkit-text-fill-color:transparent!important;caret-color:transparent!important}::placeholder{color:transparent!important}svg text,svg tspan{fill:transparent!important;stroke:transparent!important}";
