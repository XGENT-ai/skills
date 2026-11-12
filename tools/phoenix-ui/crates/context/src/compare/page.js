(() => {
  'use strict';

  const data = JSON.parse(document.getElementById('compare-data').textContent);
  const en = data.lang === 'en';
  const keyQ = '?key=' + encodeURIComponent(data.key);

  // [zh-CN, en]
  const TEXT = {
    title: ['方案对比', 'Compare proposals'],
    viewLabel: ['视图', 'View'],
    viewSide: ['并排', 'Side by side'],
    viewSingle: ['单个', 'Single'],
    vpLabel: ['视口', 'Viewport'],
    vpDesktop: ['桌面', 'Desktop'],
    vpMobile: ['手机', 'Mobile'],
    zoomLabel: ['缩放', 'Zoom'],
    zoomFit: ['适应', 'Fit'],
    tabsLabel: ['方案', 'Proposals'],
    baseline: ['现状', 'Current'],
    shotNote: ['截图 · 按原图显示', 'Screenshot · shown as captured'],
    openTab: ['在新标签页打开', 'Open in new tab'],
    choose: ['选择', 'Choose'],
    keep: ['保持现状', 'Keep current'],
    liked: ['喜欢它什么', 'What do you like about it'],
    disliked: ['不喜欢什么', 'What don’t you like'],
    confirm: ['确认选择', 'Confirm choice'],
    cancel: ['取消', 'Cancel'],
    unavailable: ['不可用：', 'Unavailable: '],
    serveIn: ['在 {cwd} 运行 {command} 后重试', 'Run {command} in {cwd}, then retry'],
    serveRun: ['运行 {command} 后重试', 'Run {command}, then retry'],
    serveNone: ['启动你的开发服务器后重试', 'Start your dev server, then retry'],
    retry: ['重试', 'Retry'],
    done: ['已选择：', 'Chosen: '],
    doneClose: ['可以关闭此页，回到对话。', 'You can close this page and return to the conversation.'],
    failed: ['提交失败：对比页服务已停止，请回到对话重新打开', 'Could not submit: the compare server has stopped. Return to the conversation and reopen it.'],
    hint: [
      '← → 切换方案 · 1–9 直接跳到对应方案 · 回车选择当前方案 · Esc 关闭面板 · 点过方案画面后，点一下页面空白处再用键盘',
      '← → switch proposals · 1–9 jump to a proposal · Enter chooses the current one · Esc closes the panel · after clicking inside a proposal, click a blank part of the page before using the keyboard',
    ],
  };
  const t = (k) => TEXT[k][en ? 1 : 0];

  const VIEWPORTS = { desktop: [1440, 900], mobile: [390, 844] };
  const state = { view: 'side', viewport: 'desktop', zoom: 'fit', current: 0, submitted: false };

  const $ = (id) => document.getElementById(id);
  const stage = $('stage');
  const columns = $('columns');
  const tabs = $('tabs');
  const panel = $('panel');
  const liked = $('liked');
  const disliked = $('disliked');
  const panelError = $('panel-error');

  const make = (tag, cls, text) => {
    const n = document.createElement(tag);
    if (cls) n.className = cls;
    if (text != null) n.textContent = text;
    return n;
  };

  // Proposal names and details are untrusted text: textContent only.
  const seqNode = (p, i) => (p.baseline ? make('span', 'badge', t('baseline')) : make('span', 'seq', String(i + 1)));

  const cols = data.proposals.map((p, i) => {
    const el = make('section', 'col');
    el.dataset.proposal = p.id;
    el.setAttribute('aria-labelledby', 'name-' + i);

    const head = make('header', 'col-head');
    head.appendChild(seqNode(p, i));
    const name = make('h2', 'name', p.name);
    name.id = 'name-' + i;
    head.appendChild(name);
    if (p.kind === 'image') head.appendChild(make('span', 'note', t('shotNote')));
    let openLink = null;
    if (p.kind === 'url' && /^https?:\/\//.test(p.src)) {
      openLink = make('a', 'open', t('openTab'));
      openLink.href = p.src;
      openLink.target = '_blank';
      openLink.rel = 'noopener noreferrer';
      head.appendChild(openLink);
    }
    el.appendChild(head);

    const frame = make('div', 'frame is-pending');
    el.appendChild(frame);

    const info = make('div', 'info');
    if (p.concept) info.appendChild(make('p', 'concept', p.concept));
    if (p.typography) info.appendChild(make('p', 'typography', p.typography));
    if (p.palette && p.palette.length) {
      const list = make('ul', 'palette');
      p.palette.forEach((value) => {
        const chip = make('li');
        chip.setAttribute('role', 'img');
        chip.setAttribute('aria-label', value);
        chip.title = value;
        // CSSOM assignment drops values the browser does not parse as a colour.
        chip.style.backgroundColor = value;
        if (chip.style.backgroundColor) list.appendChild(chip);
      });
      if (list.childNodes.length) info.appendChild(list);
    }
    if (p.traits && p.traits.length) {
      const list = make('ul', 'traits');
      p.traits.forEach((trait) => list.appendChild(make('li', null, trait)));
      info.appendChild(list);
    }
    if (info.childNodes.length) el.appendChild(info);

    const row = make('div', 'choose-row');
    const chooseBtn = make('button', 'ks-button ks-button-secondary', t(p.baseline ? 'keep' : 'choose'));
    chooseBtn.type = 'button';
    chooseBtn.dataset.choose = p.id;
    chooseBtn.setAttribute('aria-label', t(p.baseline ? 'keep' : 'choose') + (en ? ': ' : '：') + p.name);
    row.appendChild(chooseBtn);
    el.appendChild(row);
    columns.appendChild(el);

    const tab = make('button', 'tab');
    tab.type = 'button';
    tab.dataset.tab = p.id;
    tab.appendChild(seqNode(p, i));
    tab.appendChild(make('span', null, p.name));
    tabs.appendChild(tab);

    const c = { p, index: i, el, frame, chooseBtn, tab, openLink, scaler: null, iframe: null, img: null, notice: null, available: undefined };
    chooseBtn.addEventListener('click', () => openPanel(i, chooseBtn));
    tab.addEventListener('click', () => setCurrent(i, false));
    el.addEventListener('pointerdown', () => setCurrent(i, false));
    return c;
  });

  function mountIframe(c) {
    const f = document.createElement('iframe');
    f.title = c.p.name;
    if (c.p.kind === 'html') f.setAttribute('sandbox', c.p.interactive ? 'allow-scripts' : '');
    else f.setAttribute('referrerpolicy', 'no-referrer');
    f.addEventListener('load', () => c.frame.classList.remove('is-pending'));
    c.scaler = make('div', 'scaler');
    c.scaler.appendChild(f);
    c.frame.classList.add('is-pending');
    c.frame.appendChild(c.scaler);
    c.iframe = f;
    f.src = c.p.src;
  }

  function mountImage(c) {
    const img = document.createElement('img');
    img.alt = c.p.name;
    img.addEventListener('load', () => {
      c.frame.classList.remove('is-pending');
      layout();
    });
    c.img = img;
    c.frame.appendChild(img);
    img.src = c.p.src;
  }

  // Inserts each {name} of the template as a <code> element.
  function fillTemplate(parent, template, parts) {
    template.split(/(\{\w+\})/).forEach((piece) => {
      const m = /^\{(\w+)\}$/.exec(piece);
      if (m) parent.appendChild(make('code', null, parts[m[1]]));
      else if (piece) parent.appendChild(document.createTextNode(piece));
    });
  }

  function setAvailability(c, available, reason) {
    c.available = available;
    if (available) {
      if (c.notice) {
        c.notice.remove();
        c.notice = null;
      }
      if (!c.iframe) mountIframe(c);
      return;
    }
    if (c.scaler) {
      c.scaler.remove();
      c.scaler = null;
      c.iframe = null;
    }
    c.frame.classList.remove('is-pending');
    if (!c.notice) {
      c.notice = make('div', 'unavailable');
      c.notice.appendChild(make('p', 'why'));
      const how = make('p');
      const serve = data.serve;
      if (serve && serve.command) {
        if (serve.cwd) fillTemplate(how, t('serveIn'), serve);
        else fillTemplate(how, t('serveRun'), serve);
      } else {
        how.textContent = t('serveNone');
      }
      c.notice.appendChild(how);
      const retry = make('button', 'ks-button ks-button-secondary', t('retry'));
      retry.type = 'button';
      retry.dataset.retry = c.p.id;
      retry.addEventListener('click', refreshStatus);
      c.notice.appendChild(retry);
      c.frame.appendChild(c.notice);
    }
    c.notice.firstChild.textContent = t('unavailable') + (reason || '');
  }

  let statusTimer = 0;
  function refreshStatus() {
    clearTimeout(statusTimer);
    if (state.submitted) return;
    fetch('/status' + keyQ)
      .then((r) => (r.ok ? r.json() : Promise.reject(new Error('status'))))
      .then((body) => {
        const byId = {};
        (body.proposals || []).forEach((s) => { byId[s.id] = s; });
        cols.forEach((c) => {
          const s = c.p.kind === 'url' && byId[c.p.id];
          if (s) setAvailability(c, s.available === true, s.reason);
        });
        layout();
      })
      .catch(() => {})
      .then(() => {
        const waiting = cols.some((c) => c.p.kind === 'url' && !c.available);
        if (waiting && !state.submitted) statusTimer = setTimeout(refreshStatus, 5000);
      });
  }

  // Iframes render at the real viewport size and are scaled to the column,
  // so a narrow column shows the true layout smaller instead of reflowing it.
  function layout() {
    const actual = state.view === 'single' && state.zoom === 'actual';
    const vp = VIEWPORTS[state.viewport];
    cols.forEach((c) => {
      const width = c.frame.clientWidth;
      if (!width) return;
      if (c.iframe) {
        const scale = actual ? 1 : Math.min(1, width / vp[0]);
        c.iframe.style.width = vp[0] + 'px';
        c.iframe.style.height = vp[1] + 'px';
        c.iframe.style.transform = 'scale(' + scale + ')';
        c.scaler.style.width = vp[0] * scale + 'px';
        c.scaler.style.height = vp[1] * scale + 'px';
      } else if (c.img && c.img.naturalWidth) {
        const natural = c.img.naturalWidth / (c.p.pixelRatio || 1);
        const shown = state.view === 'side' ? width : actual ? natural : Math.min(width, natural);
        c.img.style.width = shown + 'px';
      }
    });
  }

  function syncSwitch(id, value, disabled) {
    $(id).querySelectorAll('.opt').forEach((b) => {
      b.setAttribute('aria-checked', String(b.dataset.value === value));
      b.disabled = disabled || state.submitted;
    });
  }

  function render() {
    stage.dataset.view = state.view;
    tabs.hidden = state.view !== 'single';
    syncSwitch('view', state.view, false);
    syncSwitch('viewport', state.viewport, false);
    syncSwitch('zoom', state.zoom, state.view !== 'single');
    cols.forEach((c) => {
      const on = c.index === state.current;
      if (on) {
        c.el.setAttribute('aria-current', 'true');
        c.tab.setAttribute('aria-current', 'true');
      } else {
        c.el.removeAttribute('aria-current');
        c.tab.removeAttribute('aria-current');
      }
    });
    layout();
  }

  function setCurrent(i, fromKeyboard) {
    if (state.submitted) return;
    if (fromKeyboard) document.body.dataset.kb = '1';
    if (i === state.current && !fromKeyboard) return;
    state.current = i;
    render();
    if (fromKeyboard && state.view === 'side') cols[i].el.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  }

  function bindSwitch(id, label, field) {
    const group = $(id);
    group.setAttribute('aria-label', label);
    group.addEventListener('click', (e) => {
      const b = e.target.closest('.opt');
      if (!b || b.disabled) return;
      state[field] = b.dataset.value;
      render();
    });
  }

  // ---- choice panel ----
  let picking = null;
  let trigger = null;
  let sending = false;

  function openPanel(i, from) {
    if (state.submitted || panel.open) return;
    if (picking !== cols[i]) {
      liked.value = '';
      disliked.value = '';
    }
    picking = cols[i];
    trigger = from || picking.chooseBtn;
    $('panel-title').textContent = picking.p.name;
    panelError.hidden = true;
    panel.showModal();
    liked.focus();
  }

  panel.addEventListener('close', () => {
    if (trigger && !state.submitted) trigger.focus();
  });
  $('cancel').addEventListener('click', () => panel.close());
  $('panel-form').addEventListener('submit', (e) => {
    e.preventDefault();
    if (sending || !picking) return;
    sending = true;
    $('confirm').disabled = true;
    fetch('/answer' + keyQ, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ proposalId: picking.p.id, liked: liked.value, disliked: disliked.value }),
    })
      .then((r) => {
        if (!r.ok) throw new Error('answer');
        finish(picking);
      })
      .catch(() => {
        sending = false;
        $('confirm').disabled = false;
        panelError.textContent = t('failed');
        panelError.hidden = false;
      });
  });

  function finish(c) {
    state.submitted = true;
    clearInterval(beatTimer);
    clearTimeout(statusTimer);
    panel.close();
    c.el.classList.add('is-chosen');
    $('done-choice').textContent = t('done') + c.p.name;
    $('done').hidden = false;
    document.querySelectorAll('button, textarea').forEach((n) => { n.disabled = true; });
    cols.forEach((x) => {
      if (!x.openLink) return;
      x.openLink.removeAttribute('href');
      x.openLink.setAttribute('aria-disabled', 'true');
    });
    layout();
  }

  // ---- keyboard ----
  document.addEventListener('keydown', (e) => {
    if (state.submitted || panel.open || e.altKey || e.ctrlKey || e.metaKey) return;
    const tag = e.target && e.target.tagName;
    if (tag === 'TEXTAREA' || tag === 'INPUT' || tag === 'SELECT') return;
    if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') {
      const next = state.current + (e.key === 'ArrowRight' ? 1 : -1);
      setCurrent(Math.max(0, Math.min(cols.length - 1, next)), true);
      e.preventDefault();
    } else if (/^[1-9]$/.test(e.key)) {
      if (Number(e.key) <= cols.length) setCurrent(Number(e.key) - 1, true);
    } else if (e.key === 'Enter' && tag !== 'BUTTON' && tag !== 'A') {
      openPanel(state.current);
      e.preventDefault();
    }
  });
  document.addEventListener('pointerdown', () => { delete document.body.dataset.kb; }, true);

  // ---- heartbeat: tells the server the page is still open ----
  const beat = () => {
    try {
      if (navigator.sendBeacon('/heartbeat' + keyQ)) return;
    } catch (_) { /* fall through to fetch */ }
    fetch('/heartbeat' + keyQ, { method: 'POST' }).catch(() => {});
  };
  const beatTimer = setInterval(beat, 4000);
  beat();

  // ---- start ----
  document.documentElement.lang = en ? 'en' : 'zh-CN';
  const title = data.title || t('title');
  document.title = title + ' · Phoenix UI';
  $('title').textContent = title;
  if (data.question) {
    $('question').textContent = data.question;
    $('question').hidden = false;
  }
  document.querySelectorAll('[data-t]').forEach((n) => { n.textContent = t(n.dataset.t); });
  $('hint').textContent = t('hint');
  tabs.setAttribute('aria-label', t('tabsLabel'));
  bindSwitch('view', t('viewLabel'), 'view');
  bindSwitch('viewport', t('vpLabel'), 'viewport');
  bindSwitch('zoom', t('zoomLabel'), 'zoom');

  cols.forEach((c) => {
    if (c.p.kind === 'html') mountIframe(c);
    else if (c.p.kind === 'image') mountImage(c);
  });
  render();
  if (cols.some((c) => c.p.kind === 'url')) refreshStatus();
  window.addEventListener('resize', layout);
})();
