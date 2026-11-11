(() => {
  'use strict';
  const data = JSON.parse(document.getElementById('elintp-review').textContent);
  const ui = data.ui, key = `elintp-review:${data.pageKey}:r${data.round}`;
  const catalog = [], registered = new Set();
  function register(element, sectionId, useReference = false) {
    const text = useReference ? element.dataset.ref : element.matches('figure') ? element.querySelector('figcaption').innerText : element.innerText;
    const quote = useReference ? text : [...text.replace(/\s+/g, ' ').trim()].slice(0, 80).join('');
    if (!quote) return;
    const number = catalog.filter(block => block.sectionId === sectionId).length + 1;
    catalog.push({ element, sectionId, blockId: `${sectionId}:b${number}`, quote });
    registered.add(element);
  }
  document.querySelectorAll('.lead > p').forEach(element => register(element, 'lead'));
  document.querySelectorAll('main > section').forEach(section => {
    for (const element of section.children) {
      if (element.matches('ul,ol')) element.querySelectorAll('li').forEach(li => register(li, section.id));
      else if (element.matches('p:not(.slot-label),h3,pre,figure,.metrics,.table-scroll,aside')) register(element, section.id);
    }
  });
  document.querySelectorAll('main > section').forEach(section => {
    for (const element of section.children) {
      if (element.matches('figure[data-ref]') && !registered.has(element)) register(element, section.id, true);
    }
  });

  function node(tag, className, text) {
    const element = document.createElement(tag);
    if (className) element.className = className;
    if (text !== undefined) element.textContent = text;
    return element;
  }
  function button(text, action) {
    const element = node('button', '', text);
    element.type = 'button';
    element.addEventListener('click', action);
    return element;
  }
  const toolbar = node('div', 'review-toolbar');
  const toggle = button(`＋ ${ui.toggle}`, () => { enabled = !enabled; closeEditor(); refresh(); });
  toggle.setAttribute('aria-pressed', 'false');
  const submit = button(`↑ ${ui.submit}`, submitItems);
  const clear = button(`× ${ui.clear}`, () => {
    if (!confirm(ui.confirmClear)) return;
    items = []; closeEditor(); persist(); refresh();
  });
  const status = node('span', 'review-status');
  status.setAttribute('aria-live', 'polite');
  const alert = node('p', 'review-error');
  alert.setAttribute('role', 'alert');
  const storageAlert = node('p', 'review-error');
  storageAlert.setAttribute('role', 'alert');
  toolbar.append(toggle, submit, clear, status, alert, storageAlert);
  document.querySelector('.page-head').after(toolbar);

  let items = data.items, enabled = false, busy = false, sent = false, editor = null;
  try {
    const stored = localStorage.getItem(key);
    if (stored !== null) {
      const value = JSON.parse(stored);
      if (!Array.isArray(value.items)) throw new SyntaxError('Invalid draft');
      items = value.items;
    }
  } catch { storageAlert.textContent = ui.storage; }
  function persist() {
    try { localStorage.setItem(key, JSON.stringify({ items })); }
    catch { storageAlert.textContent = ui.storage; }
  }
  for (const block of catalog) {
    block.control = node(block.element.matches('li') ? 'span' : 'div', 'review-block-control');
    block.annotate = button('', () => edit(block));
    block.summary = node('span', 'review-summary');
    block.remove = button(`× ${ui.remove}`, () => {
      items = items.filter(item => item.blockId !== block.blockId);
      closeEditor(); persist(); refresh(); block.annotate.focus();
    });
    block.control.append(block.annotate, block.summary, block.remove);
    if (block.element.matches('li')) block.element.prepend(block.control);
    else block.element.before(block.control);
  }
  function closeEditor() {
    if (editor) editor.remove();
    editor = null;
  }
  function edit(block) {
    closeEditor();
    const current = items.find(item => item.blockId === block.blockId);
    editor = node('form', 'review-editor');
    const quote = node('p', 'review-quote', block.quote);
    const fieldset = node('fieldset');
    fieldset.append(node('legend', '', ui.kind));
    for (const kind of ['unclear', 'question', 'suggestion']) {
      const label = node('label');
      const radio = node('input');
      radio.type = 'radio'; radio.name = 'kind'; radio.value = kind;
      radio.checked = kind === (current?.kind || 'unclear');
      label.append(radio, document.createTextNode(ui[kind]));
      fieldset.append(label);
    }
    const label = node('label', 'review-note-label', ui.note);
    const note = node('input');
    note.type = 'text'; note.value = current?.note || '';
    label.append(note);
    const actions = node('div', 'review-actions');
    const save = node('button', '', `✓ ${ui.save}`); save.type = 'submit';
    const cancel = button(ui.cancel, () => { closeEditor(); block.annotate.focus(); });
    actions.append(save, cancel);
    editor.append(quote, fieldset, label, actions);
    editor.addEventListener('submit', event => {
      event.preventDefault();
      const kind = editor.querySelector('input[name="kind"]:checked').value;
      items = items.filter(item => item.blockId !== block.blockId);
      items.push({ blockId: block.blockId, sectionId: block.sectionId, quote: block.quote, kind, ...(note.value ? { note: note.value } : {}) });
      closeEditor(); persist(); refresh(); block.annotate.focus();
    });
    editor.addEventListener('keydown', event => {
      if (event.key === 'Escape') { closeEditor(); block.annotate.focus(); }
    });
    block.control.append(editor);
    editor.querySelector('input:checked').focus();
  }
  function refresh() {
    toggle.setAttribute('aria-pressed', String(enabled));
    toggle.disabled = busy || sent;
    clear.hidden = !items.length;
    clear.disabled = busy || sent;
    submit.disabled = busy || sent || !items.length;
    status.textContent = sent ? ui.sent : busy ? ui.sending : items.length ? `${items.length} ${ui.count}` : ui.empty;
    for (const block of catalog) {
      const item = items.find(item => item.blockId === block.blockId);
      block.control.hidden = !enabled;
      block.annotate.textContent = item ? `✎ ${ui.edit} · ${ui[item.kind]}` : `＋ ${ui.annotate}`;
      block.annotate.setAttribute('aria-label', `${block.annotate.textContent}: ${block.quote}`);
      block.summary.textContent = item?.note || '';
      block.remove.hidden = !item;
      block.control.querySelectorAll('button,input').forEach(element => { element.disabled = busy || sent; });
    }
  }
  async function submitItems() {
    busy = true; alert.textContent = ''; refresh();
    let response;
    try {
      response = await fetch('/submit', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ round: data.round, items }) });
    } catch (error) {
      if (!(error instanceof TypeError)) throw error;
      alert.textContent = ui.offline; busy = false; refresh(); return;
    }
    const result = await response.json();
    if (!response.ok) alert.textContent = result.errors.join('\n');
    else {
      sent = true; closeEditor();
      storageAlert.textContent = '';
      try { localStorage.removeItem(key); }
      catch { /* The submitted result is durable; browser storage is no longer needed. */ }
    }
    busy = false; refresh();
  }
  refresh();
})();
