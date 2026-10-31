/* Standalone comparison navigation. Snapshot bytes remain immutable; only
   their live Reader documents receive theme, camera and sizing updates. */
(() => {
  'use strict';
  const root = document.documentElement;
  const tabs = [...document.querySelectorAll('[data-comparison-select]')];
  const frames = [...document.querySelectorAll('iframe[data-comparison-view]')];
  const rows = [...document.querySelectorAll('[data-comparison-change]')];
  const status = document.querySelector('[data-comparison-status]');
  const overviewLabel = status.textContent;
  const previous = document.querySelector('[data-comparison-previous]');
  const next = document.querySelector('[data-comparison-next]');
  const overview = document.querySelector('[data-comparison-overview]');
  const themeButton = document.querySelector('[data-comparison-theme]');
  const loaded = new Map();
  let activeView = 'delta';
  let activeIndex = -1;
  let generation = 0;
  let measuring = false;
  let pending = Promise.resolve();
  let theme = new URLSearchParams(location.search).get('theme');
  if (theme !== 'dark' && theme !== 'light') {
    try { theme = localStorage.getItem('phoenix-diagram-comparison-theme'); } catch (_) {}
    if (theme !== 'dark' && theme !== 'light') theme = matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
  }
  const tick = () => new Promise(resolve => requestAnimationFrame(resolve));
  const currentFrame = () => frames.find(frame => frame.dataset.comparisonView === activeView);

  function syncTheme(frame) {
    const doc = frame.contentDocument;
    if (!doc?.documentElement) return;
    if (doc.documentElement.dataset.theme !== theme) {
      const control = frame.contentWindow.PhoenixDiagram?.theme;
      if (control?.toggle) control.toggle();
      else doc.documentElement.dataset.theme = theme;
    }
  }
  function applyTheme() {
    root.dataset.theme = theme;
    themeButton.setAttribute('aria-pressed', String(theme === 'light'));
    frames.forEach(syncTheme);
  }
  async function measure() {
    const token = ++generation;
    const frame = currentFrame();
    root.dataset.comparisonReady = 'false';
    frame.dataset.comparisonReady = 'false';
    measuring = true;
    try {
      await loaded.get(frame);
      if (frame.contentDocument.fonts?.ready) await frame.contentDocument.fonts.ready;
      syncTheme(frame);
      const minHeight = Math.max(480, innerHeight - Math.max(0, frame.getBoundingClientRect().top) - 24);
      let last = '';
      let stable = 0;
      for (let pass = 0; pass < 40; pass++) {
        if (token !== generation) return;
        const doc = frame.contentDocument;
        const borderHeight = Math.max(0, frame.offsetHeight - frame.clientHeight);
        const height = Math.ceil(Math.max(minHeight, Math.max(doc.body?.scrollHeight || 0, doc.documentElement.scrollHeight) + borderHeight));
        if (Math.abs(frame.getBoundingClientRect().height - height) > 1) frame.style.height = `${height}px`;
        await tick(); await tick();
        if (token !== generation) return;
        const actual = [frame.clientWidth, frame.clientHeight, doc.documentElement.scrollHeight, doc.body?.scrollHeight || 0].join(':');
        stable = actual === last ? stable + 1 : 0;
        last = actual;
        if (stable >= 2) {
          if (doc.documentElement.scrollHeight > frame.clientHeight + 1) continue;
          frame.dataset.comparisonReady = 'true';
          root.dataset.comparisonReady = 'true';
          root.removeAttribute('data-comparison-error');
          return;
        }
      }
      throw new Error('Comparison view layout did not settle');
    } finally {
      if (token === generation) measuring = false;
    }
  }
  function requestStable() {
    pending = measure().catch(error => {
      root.dataset.comparisonReady = 'false';
      root.dataset.comparisonError = 'layout-unavailable';
      throw error;
    });
    // Keep event-triggered failures observable through whenStable, without an
    // unhandled rejection when no automation or user action is awaiting them.
    pending.catch(() => {});
    return pending;
  }
  frames.forEach(frame => {
    let resolve;
    const ready = new Promise(done => { resolve = done; });
    loaded.set(frame, ready);
    const onLoad = () => {
      const doc = frame.contentDocument;
      if (!doc?.head || !doc.querySelector('#phoenix-diagram-metadata')) return;
      // The parent expands to include the Reader index. A viewport-sized
      // diagram stage would grow again after each iframe height adjustment.
      if (!doc.getElementById('phoenix-comparison-layout')) {
        const style = doc.createElement('style');
        style.id = 'phoenix-comparison-layout';
        style.textContent = '.diagram-container{min-height:0!important}';
        doc.head.appendChild(style);
      }
      syncTheme(frame);
      resolve();
      if (typeof ResizeObserver === 'function') {
        let previousSize = '';
        new ResizeObserver(() => {
          const size = [doc.body.scrollHeight, doc.documentElement.scrollHeight, frame.clientWidth].join(':');
          if (size !== previousSize) {
            previousSize = size;
            if (frame === currentFrame() && !measuring) requestStable();
          }
        }).observe(doc.body);
      }
      if (frame === currentFrame()) requestStable();
    };
    frame.addEventListener('load', onLoad);
    if (frame.contentDocument?.readyState === 'complete') onLoad();
  });
  function show(id, focus = false) {
    if (!frames.some(frame => frame.dataset.comparisonView === id)) return;
    activeView = id;
    tabs.forEach(tab => {
      const selected = tab.dataset.comparisonSelect === id;
      tab.setAttribute('aria-selected', String(selected));
      tab.tabIndex = selected ? 0 : -1;
      document.getElementById(tab.getAttribute('aria-controls')).hidden = !selected;
      if (selected && focus) tab.focus();
    });
    currentFrame().contentWindow?.dispatchEvent(new Event('resize'));
    requestStable();
  }
  tabs.forEach((tab, index) => {
    tab.addEventListener('click', () => show(tab.dataset.comparisonSelect));
    tab.addEventListener('keydown', event => {
      const step = event.key === 'ArrowRight' ? 1 : event.key === 'ArrowLeft' ? -1 : 0;
      let target = step ? (index + step + tabs.length) % tabs.length : event.key === 'Home' ? 0 : event.key === 'End' ? tabs.length - 1 : -1;
      if (target < 0) return;
      event.preventDefault();
      show(tabs[target].dataset.comparisonSelect, true);
    });
  });
  function targets(frame, row) {
    const doc = frame.contentDocument;
    if (!doc) return [];
    if (frame.dataset.comparisonView === 'delta') return [...doc.querySelectorAll('[data-delta-key]')].filter(element => element.dataset.deltaKey === row.dataset.comparisonChange);
    const id = row.dataset.comparisonId;
    const kind = row.dataset.comparisonKind;
    if (kind === 'component') return [...doc.querySelectorAll('[data-node-id]')].filter(element => element.getAttribute('data-node-id') === id);
    if (kind === 'relationship') return [...doc.querySelectorAll('[data-edge-id]')].filter(element => element.getAttribute('data-edge-id') === id);
    const metadata = JSON.parse(doc.querySelector('#phoenix-diagram-metadata').textContent);
    const boundaries = metadata.facts?.boundaries || [];
    const index = boundaries.findIndex(boundary => `${boundary.kind}:${boundary.label}` === id);
    return [...doc.querySelectorAll('[data-composition-frame-id]')].filter(element => element.getAttribute('data-composition-frame-id') === `boundary-${index}`);
  }
  function buttons() {
    previous.disabled = activeIndex <= 0;
    next.disabled = rows.length === 0 || activeIndex === rows.length - 1;
    overview.disabled = activeIndex < 0;
  }
  function clear() {
    frames.forEach(frame => frame.contentDocument?.querySelectorAll('[data-delta-review-current]').forEach(element => element.removeAttribute('data-delta-review-current')));
    rows.forEach(row => row.removeAttribute('aria-current'));
  }
  async function locate(index) {
    if (index < 0 || index >= rows.length) return;
    const row = rows[index];
    await pending;
    let elements = targets(currentFrame(), row);
    if (!elements.length) { show('delta'); await pending; elements = targets(currentFrame(), row); }
    clear();
    elements.forEach(element => element.setAttribute('data-delta-review-current', 'true'));
    const bounds = elements.filter(element => typeof element.getBBox === 'function').map(element => element.getBBox()).filter(box => box.width > 0 || box.height > 0);
    if (bounds.length) {
      const left = Math.min(...bounds.map(box => box.x)); const right = Math.max(...bounds.map(box => box.x + box.width));
      const top = Math.min(...bounds.map(box => box.y)); const bottom = Math.max(...bounds.map(box => box.y + box.height));
      currentFrame().contentWindow.PhoenixDiagram?.view?.centerAt((left + right) / 2, (top + bottom) / 2, { minimumScale: 1, scale: 1.25, instant: matchMedia('(prefers-reduced-motion: reduce)').matches });
    }
    row.setAttribute('aria-current', 'step');
    activeIndex = index;
    status.textContent = `${index + 1} / ${rows.length} · ${row.dataset.comparisonLabel}`;
    buttons();
    await requestStable();
  }
  rows.forEach((row, index) => row.addEventListener('click', () => { locate(index).catch(() => {}); }));
  previous.addEventListener('click', () => { locate(activeIndex - 1).catch(() => {}); });
  next.addEventListener('click', () => { locate(activeIndex + 1).catch(() => {}); });
  overview.addEventListener('click', () => {
    clear(); activeIndex = -1; status.textContent = overviewLabel; buttons();
    currentFrame().contentWindow.PhoenixDiagram?.view?.reset();
    requestStable();
  });
  themeButton.addEventListener('click', () => {
    theme = theme === 'dark' ? 'light' : 'dark';
    try { localStorage.setItem('phoenix-diagram-comparison-theme', theme); } catch (_) {}
    applyTheme(); requestStable();
  });
  addEventListener('resize', () => requestStable());
  window.PhoenixComparison = {
    select: show,
    whenStable: async () => {
      for (let attempts = 0; attempts < 20; attempts++) {
        const observed = pending; await observed;
        if (observed === pending && root.dataset.comparisonReady === 'true') return { viewId: activeView, width: currentFrame().clientWidth, height: currentFrame().clientHeight };
      }
      throw new Error('Comparison view changed while waiting for layout');
    }
  };
  applyTheme(); buttons(); requestStable();
})();
