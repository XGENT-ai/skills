// 两个模板共用的状态与提交逻辑；模板只负责呈现。
(() => {
  const data = JSON.parse(document.getElementById('form-data').textContent);
  const STRINGS = {
    'zh-CN': {
      confirm: '确认', reject: '驳回', modify: '修改',
      what: '是什么', why: '为什么', how: '怎么做', acceptance: '验收', impact: '影响', verification: '验证',
      choice: '选定方案', alternatives: '备选方案', cost: '代价与风险', affects: '影响功能', dependsOn: '依赖',
      decisions: '技术架构决策', features: '功能点', round: '第 {n} 轮',
      modifyPlaceholder: '必填：希望怎么改', optionalPlaceholder: '可选：补充说明', rejectPlaceholder: '可选：驳回理由',
      useAlternative: '改用备选方案', noAlternative: '不改方案，只按说明修改',
      noteRequired: '选择修改时须附说明', flagRejectedNote: '上游已驳回，本项可能不再成立', flagModifiedNote: '上游已修改，本项可能受影响', pendingResult: '待决',
      dependents: '被 {ids} 依赖',
      before: '修订前', after: '修订后', yourNote: '修改说明', backBecause: '回来的原因',
      reasonModify: '你上一轮要求修改，这是修订稿', reasonDependency: '受上一轮其他决定影响',
      submit: '提交决定', submitting: '正在提交…', reviewTitle: '提交前核对', reviewBack: '返回修改', reviewSend: '确认提交',
      reviewPending: '{n} 项未决，提交后保持待决', reviewInvalid: '{n} 项选了修改但没写说明，补齐后才能提交',
      reviewFlags: '{n} 项受其他决定影响，请确认它们的决定仍然成立',
      submitted: '已提交', submittedHint: '结果已交给 agent，可以关闭此页。', signedBy: '提交时间',
      submitFailed: '提交失败：{msg}', offline: '本地服务已关闭，请让 agent 重新生成表单。',
      decided: '已决 {n} / {total}', draftSaved: '草稿已自动保存在本浏览器',
      colId: '编号', colItem: '条目', colDecisionItem: '决策', tradeoff: '取舍', option: '方案', colFlag: '标记', colDecision: '结论',
      metaDoc: '文档', metaVersion: '版本', metaScope: '本次范围', metaRound: '轮次',
      kindRequirements: '需求确认单', kindPlan: '开发计划确认单', roundOne: '首轮确认', roundLater: '第 {n} 轮 · 修订稿与受影响条目',
      tallyPending: '待决', tallyFlagged: '受影响', tallyInvalid: '缺修改说明', nextPending: '下一个待决',
      decisionOf: '{id} 的结论', noteModify: '修改说明（必填）', noteModifyAlt: '补充说明（可选）', noteReject: '驳回理由（可选）',
      reviewCounts: '确认 {c} · 驳回 {r} · 修改 {m}',
    },
    en: {
      confirm: 'Confirm', reject: 'Reject', modify: 'Modify',
      what: 'What', why: 'Why', how: 'How', acceptance: 'Acceptance', impact: 'Impact', verification: 'Verification',
      choice: 'Chosen option', alternatives: 'Alternatives', cost: 'Cost and risk', affects: 'Affects', dependsOn: 'Depends on',
      decisions: 'Architecture decisions', features: 'Features', round: 'Round {n}',
      modifyPlaceholder: 'Required: how should it change?', optionalPlaceholder: 'Optional note', rejectPlaceholder: 'Optional: reason for rejecting',
      useAlternative: 'Switch to an alternative', noAlternative: 'Keep the option, change as described',
      noteRequired: 'Modify needs a note', flagRejectedNote: 'Upstream rejected; this may no longer hold', flagModifiedNote: 'Upstream modified; this may be affected', pendingResult: 'Pending',
      dependents: 'Needed by {ids}',
      before: 'Before', after: 'After', yourNote: 'Your note', backBecause: 'Why it is back',
      reasonModify: 'You asked for a change last round; this is the revision', reasonDependency: 'Affected by another decision last round',
      submit: 'Submit decisions', submitting: 'Submitting…', reviewTitle: 'Check before submitting', reviewBack: 'Keep editing', reviewSend: 'Submit now',
      reviewPending: '{n} undecided; they stay pending', reviewInvalid: '{n} marked Modify without a note; add notes to submit',
      reviewFlags: '{n} affected by other decisions; make sure those decisions still hold',
      submitted: 'Submitted', submittedHint: 'The result is with the agent. You can close this page.', signedBy: 'Submitted at',
      submitFailed: 'Submit failed: {msg}', offline: 'The local server has stopped. Ask the agent to generate the form again.',
      decided: '{n} of {total} decided', draftSaved: 'Draft saved in this browser',
      colId: 'ID', colItem: 'Item', colDecisionItem: 'Architecture decision', tradeoff: 'Trade-off', option: 'Option', colFlag: 'Flags', colDecision: 'Decision',
      metaDoc: 'Document', metaVersion: 'Version', metaScope: 'Scope', metaRound: 'Round',
      kindRequirements: 'Requirements confirmation', kindPlan: 'Development plan confirmation', roundOne: 'First pass', roundLater: 'Round {n} · revisions and affected items',
      tallyPending: 'Pending', tallyFlagged: 'Affected', tallyInvalid: 'Missing notes', nextPending: 'Next pending',
      decisionOf: 'Decision for {id}', noteModify: 'What should change (required)', noteModifyAlt: 'Extra note (optional)', noteReject: 'Reason (optional)',
      reviewCounts: '{c} confirmed · {r} rejected · {m} modified',
    },
  };
  const strings = STRINGS[data.locale] ?? STRINGS['zh-CN'];
  const t = (key, vars = {}) => strings[key].replace(/\{(\w+)\}/g, (_, name) => vars[name]);
  document.documentElement.lang = data.locale ?? 'zh-CN';

  // 依赖与决策影响都算上游：上游被驳回或修改时给下游打标。
  const upstream = new Map(data.items.map(item => [item.id, new Set(item.dependsOn ?? [])]));
  for (const item of data.items) for (const id of item.affects ?? []) upstream.get(id).add(item.id);
  const downstream = new Map(data.items.map(item => [item.id, []]));
  for (const [id, ups] of upstream) for (const up of ups) downstream.get(up).push(id);

  const fingerprint = JSON.stringify([data.title, data.round, data.documents, data.items.map(item => [item.id, item.revision?.after ?? item.what ?? item.choice])]);
  let hash = 0;
  for (const char of fingerprint) hash = (hash * 31 + char.codePointAt(0)) >>> 0;
  const storageKey = `spec-human-decision:${hash.toString(36)}`;

  const state = new Map(data.items.map(item => [item.id, { decision: null, note: '', alternative: undefined }]));
  try {
    const saved = JSON.parse(localStorage.getItem(storageKey) ?? '{}');
    for (const [id, answer] of Object.entries(saved)) if (state.has(id)) state.set(id, { ...state.get(id), ...answer });
  } catch { /* 隐私模式或存储被禁用时不保留草稿 */ }

  const listeners = new Set();
  let phase = 'editing';
  const notify = changed => listeners.forEach(fn => fn(changed));

  const invalid = id => {
    const answer = state.get(id);
    return answer.decision === 'modify' && !answer.note.trim() && answer.alternative === undefined;
  };
  const flags = id => [...upstream.get(id)]
    .filter(up => ['reject', 'modify'].includes(state.get(up).decision))
    .map(up => ({ id: up, decision: state.get(up).decision }));
  const counts = () => {
    const result = { confirm: 0, reject: 0, modify: 0, pending: 0, invalid: 0, flagged: 0, total: data.items.length };
    for (const [id, answer] of state) {
      result[answer.decision ?? 'pending'] += 1;
      if (invalid(id)) result.invalid += 1;
      if (flags(id).length) result.flagged += 1;
    }
    return result;
  };

  function set(id, patch) {
    if (phase !== 'editing') return;
    const next = { ...state.get(id), ...patch };
    if (next.decision !== 'modify') next.alternative = undefined;
    state.set(id, next);
    try { localStorage.setItem(storageKey, JSON.stringify(Object.fromEntries(state))); } catch { /* 同上 */ }
    notify([id, ...downstream.get(id)]);
  }

  async function submit() {
    if (counts().invalid) return { errors: [t('noteRequired')] };
    phase = 'submitting';
    notify([]);
    const body = {
      items: data.items.map(item => {
        const { decision, note, alternative } = state.get(item.id);
        return { id: item.id, decision, ...(note.trim() ? { note: note.trim() } : {}), ...(alternative !== undefined ? { alternative } : {}) };
      }),
    };
    try {
      const response = await fetch('/submit', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) });
      const reply = await response.json();
      if (!response.ok) throw new Error(reply.errors.join('；'));
      phase = 'submitted';
      try { localStorage.removeItem(storageKey); } catch { /* 同上 */ }
      notify([]);
      return { ok: true };
    } catch (error) {
      phase = 'editing';
      notify([]);
      return { errors: [error instanceof TypeError ? t('offline') : t('submitFailed', { msg: error.message })] };
    }
  }

  // ---- 共用界面：页眉、三线表行、标记列、结论控件与签署栏 ----
  const ICONS = {
    confirm: '<path d="M5 12.5l4.5 4.5L19 7.5"/>',
    reject: '<path d="M6 6l12 12M18 6L6 18"/>',
    modify: '<path d="M4 20h4L19 9l-4-4L4 16z"/><path d="M13.5 6.5l4 4"/>',
    flagReject: '<path d="M12 4l9 16H3z"/>',
    flagModify: '<path d="M12 3l9 9-9 9-9-9z"/>',
    next: '<path d="M12 5v14M6 13l6 6 6-6"/>',
    send: '<path d="M4 12h14M13 6l6 6-6 6"/>',
  };
  function icon(name) {
    const span = document.createElement('span');
    span.innerHTML = `<svg class="icon" viewBox="0 0 24 24" aria-hidden="true">${ICONS[name]}</svg>`;
    return span.firstChild;
  }
  function h(tag, props = {}, children = []) {
    const node = document.createElement(tag);
    for (const [key, value] of Object.entries(props)) {
      if (value === undefined || value === null || value === false) continue;
      if (key === 'class') node.className = value;
      else if (key === 'text') node.textContent = value;
      else if (key.startsWith('on')) node.addEventListener(key.slice(2), value);
      else node.setAttribute(key, value === true ? '' : value);
    }
    for (const child of [children].flat(Infinity)) {
      if (child === undefined || child === null || child === false) continue;
      node.append(child instanceof Node ? child : String(child));
    }
    return node;
  }
  const MISSING = /^(文档未说明|计划未说明|not stated)/i;
  // facts([[标签, 值]])：值为文字、文字列表或节点；空值跳过。
  function facts(entries) {
    const list = h('dl', { class: 'facts' });
    for (const [key, value] of entries) {
      if (value === undefined || value === null || (Array.isArray(value) && !value.length)) continue;
      const content = Array.isArray(value) ? h('ul', {}, value.map(line => h('li', { text: line })))
        : value instanceof Node ? value
          : h('span', { class: MISSING.test(value) ? 'missing' : undefined, text: value });
      list.append(h('dt', { text: key }), h('dd', {}, content));
    }
    return list;
  }
  const sep = data.locale === 'en' ? ', ' : '、';
  const refs = ids => ids?.length ? h('span', { class: 'refs', text: ids.join(sep) }) : undefined;
  const tags = values => {
    const present = values.filter(Boolean);
    return present.length ? h('span', { class: 'tags' }, present.map(value => h('span', { class: 'tag', text: value }))) : undefined;
  };

  const rows = new Map();
  let showErrors = false;

  function revisionBlock(item) {
    const revision = item.revision;
    if (!revision) return undefined;
    const why = revision.reason === 'modify' ? t('reasonModify') : revision.cause ?? t('reasonDependency');
    return h('div', { class: 'revision' }, [
      h('p', { class: 'why' }, [h('b', { text: `${t('backBecause')}：` }), why]),
      h('div', { class: 'diff' }, [
        h('div', { class: 'before' }, [h('span', { text: t('before') }), h('p', { text: revision.before })]),
        h('div', { class: 'after' }, [h('span', { text: t('after') }), h('p', { text: revision.after })]),
      ]),
      revision.note ? h('p', { class: 'quote' }, [`${t('yourNote')}：`, `「${revision.note}」`]) : undefined,
    ]);
  }

  function decideCell(item) {
    const name = `decision-${item.id}`;
    const choices = h('fieldset', { class: 'choices' }, [
      h('legend', { text: t('decisionOf', { id: item.id }) }),
      ['confirm', 'reject', 'modify'].map(decision => h('label', { class: `choice ${decision}` }, [
        h('input', { type: 'radio', name, value: decision, onchange: () => set(item.id, { decision }) }),
        h('span', {}, [icon(decision), t(decision)]),
      ])),
    ]);
    const alts = item.alternatives?.length ? h('fieldset', { class: 'alts' }, [
      h('legend', { text: t('useAlternative') }),
      h('label', {}, [h('input', { type: 'radio', name: `${name}-alt`, value: '', onchange: () => set(item.id, { alternative: undefined }) }), t('noAlternative')]),
      item.alternatives.map(alt => h('label', {}, [
        h('input', { type: 'radio', name: `${name}-alt`, value: alt.name, onchange: () => set(item.id, { alternative: alt.name }) }),
        alt.name,
      ])),
    ]) : undefined;
    const noteId = `note-${item.id}`;
    const textarea = h('textarea', { id: noteId, rows: 3, 'aria-describedby': `${noteId}-error`, oninput: event => set(item.id, { note: event.target.value }), onblur: () => update(item.id) });
    const note = h('div', { class: 'note' }, [
      alts,
      h('label', { for: noteId }),
      textarea,
      h('p', { class: 'error', id: `${noteId}-error`, text: t('noteRequired') }),
    ]);
    return h('div', { class: 'row-decide' }, [choices, note]);
  }

  function renderRow(item, body) {
    const row = h('article', { class: 'row', id: `item-${item.id}`, 'aria-labelledby': `title-${item.id}` }, [
      h('div', { class: 'row-id' }, [h('span', { class: 'id', text: item.id }), h('span', { class: 'src', text: item.source })]),
      h('div', { class: 'row-body' }, [body(item), revisionBlock(item)]),
      h('div', { class: 'row-flag' }),
      decideCell(item),
    ]);
    rows.set(item.id, row);
    return row;
  }

  function update(id) {
    const row = rows.get(id);
    const answer = state.get(id);
    const locked = phase !== 'editing';
    const choices = row.querySelector('.choices');
    choices.classList.toggle('decided', answer.decision !== null);
    choices.dataset.pending = t('pendingResult');
    for (const input of row.querySelectorAll('.choices input')) {
      input.checked = input.value === answer.decision;
      input.disabled = locked;
    }
    for (const input of row.querySelectorAll('.alts input')) {
      input.checked = input.value === (answer.alternative ?? '');
      input.disabled = locked;
    }
    const note = row.querySelector('.note');
    const textarea = note.querySelector('textarea');
    note.classList.toggle('open', answer.decision === 'modify' || answer.decision === 'reject');
    const alts = note.querySelector('.alts');
    if (alts) alts.hidden = answer.decision !== 'modify';
    const labelKey = answer.decision === 'reject' ? 'noteReject' : answer.alternative !== undefined ? 'noteModifyAlt' : 'noteModify';
    note.querySelector('label[for]').textContent = t(labelKey);
    textarea.placeholder = t(answer.decision === 'reject' ? 'rejectPlaceholder' : answer.alternative !== undefined ? 'optionalPlaceholder' : 'modifyPlaceholder');
    if (textarea.value !== answer.note) textarea.value = answer.note;
    textarea.disabled = locked;
    const bad = invalid(id) && (showErrors || (document.activeElement !== textarea && textarea.dataset.touched === '1'));
    if (document.activeElement === textarea) textarea.dataset.touched = '1';
    textarea.setAttribute('aria-invalid', String(bad));

    const flagCell = row.querySelector('.row-flag');
    const before = flagCell.dataset.flags ?? '';
    const current = flags(id);
    const key = current.map(flag => flag.id + flag.decision).join();
    if (key !== before) {
      flagCell.dataset.flags = key;
      flagCell.replaceChildren(...current.map(flag => h('div', { class: `flag ${flag.decision}` }, [
        h('strong', {}, [icon(flag.decision === 'reject' ? 'flagReject' : 'flagModify'), `${flag.id} ${t(flag.decision)}`]),
        h('span', { text: t(flag.decision === 'reject' ? 'flagRejectedNote' : 'flagModifiedNote') }),
      ])));
      const deps = downstream.get(id);
      if (deps.length) flagCell.append(h('div', { class: 'flag-note', text: t('dependents', { ids: deps.join(sep) }) }));
      return key !== '' || before !== '';
    }
    if (!flagCell.childElementCount && downstream.get(id).length) {
      flagCell.append(h('div', { class: 'flag-note', text: t('dependents', { ids: downstream.get(id).join(sep) }) }));
    }
    return false;
  }

  const firstWhere = test => data.items.find(item => test(item.id))?.id;
  const nextPending = () => {
    const ids = data.items.filter(item => state.get(item.id).decision === null).map(item => item.id);
    if (!ids.length) return undefined;
    const below = ids.find(id => rows.get(id).getBoundingClientRect().top > 120);
    return below ?? ids[0];
  };
  function jump(id) {
    const row = rows.get(id);
    row.scrollIntoView({ behavior: matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth', block: 'start' });
    row.querySelector('.choices input:checked, .choices input')?.focus({ preventScroll: true });
  }
  const jumpLink = (id, text) => id ? h('a', { href: `#item-${id}`, onclick: event => { event.preventDefault(); jump(id); } }, text) : text;

  let tally, progress, nextButton, submitButton, review, signInner;
  function renderHead(kindKey) {
    const docs = data.documents;
    tally = h('ul', { class: 'tally', 'aria-live': 'polite' });
    return h('header', { class: 'head' }, h('div', { class: 'head-inner' }, [
      h('h1', { text: data.title }),
      h('p', { class: 'kind', text: `${t(kindKey)} · ${data.round === 1 ? t('roundOne') : t('roundLater', { n: data.round })}` }),
      h('dl', { class: 'meta' }, [
        h('div', {}, [h('dt', { text: t('metaDoc') }), h('dd', {}, docs.map((doc, index) => [index ? h('br') : null, `${doc.type} · `, h('span', { class: 'mono', text: doc.path })]))]),
        h('div', {}, [h('dt', { text: t('metaVersion') }), h('dd', {}, docs.map((doc, index) => [index ? h('br') : null, h('span', { class: 'mono', text: doc.version })]))]),
        data.scope ? h('div', {}, [h('dt', { text: t('metaScope') }), h('dd', { text: data.scope })]) : null,
        h('div', {}, [h('dt', { text: t('metaRound') }), h('dd', { text: t('round', { n: data.round }) })]),
      ]),
      tally,
    ]));
  }

  function renderSign() {
    progress = h('div', { class: 'progress', 'aria-live': 'polite' });
    nextButton = h('button', { class: 'btn', type: 'button', onclick: () => { const id = nextPending(); if (id) jump(id); } }, [icon('next'), t('nextPending')]);
    submitButton = h('button', { class: 'btn primary', type: 'button', onclick: openReview }, [icon('send'), t('submit')]);
    review = h('section', { class: 'review', 'aria-labelledby': 'review-title' });
    signInner = h('div', { class: 'sign-inner' }, [progress, h('div', { class: 'actions' }, [nextButton, submitButton]), review]);
    return h('footer', { class: 'sign' }, signInner);
  }

  function openReview() {
    showErrors = true;
    data.items.forEach(item => update(item.id));
    renderReview([]);
    review.classList.add('open');
    review.querySelector('button.primary, button')?.focus();
  }

  function renderReview(errors) {
    const c = counts();
    const lines = [
      h('li', { text: t('reviewCounts', { c: c.confirm, r: c.reject, m: c.modify }) }),
      c.pending ? h('li', {}, jumpLink(firstWhere(id => state.get(id).decision === null), t('reviewPending', { n: c.pending }))) : null,
      c.invalid ? h('li', { class: 'bad' }, jumpLink(firstWhere(invalid), t('reviewInvalid', { n: c.invalid }))) : null,
      c.flagged ? h('li', {}, jumpLink(firstWhere(id => flags(id).length > 0), t('reviewFlags', { n: c.flagged }))) : null,
    ];
    const send = h('button', { class: 'btn primary', type: 'button', disabled: c.invalid > 0 || phase === 'submitting', onclick: async () => {
      const outcome = await submit();
      if (outcome.errors) renderReview(outcome.errors);
    } }, [icon('send'), phase === 'submitting' ? t('submitting') : t('reviewSend')]);
    review.replaceChildren(...[
      h('h2', { id: 'review-title', text: t('reviewTitle') }),
      h('ul', {}, lines),
      errors.length ? h('p', { class: 'errors', role: 'alert', text: errors.join('；') }) : null,
      h('div', { class: 'actions' }, [h('button', { class: 'btn', type: 'button', onclick: () => review.classList.remove('open') }, t('reviewBack')), send]),
    ].filter(Boolean));
  }

  function refresh(changed) {
    const ids = changed.length ? changed : data.items.map(item => item.id);
    for (const id of ids) {
      const flagged = update(id);
      if (flagged && changed.length && id !== changed[0]) {
        const row = rows.get(id);
        row.classList.add('pulse');
        requestAnimationFrame(() => requestAnimationFrame(() => row.classList.remove('pulse')));
      }
    }
    const c = counts();
    const pendingId = nextPending();
    tally.replaceChildren(...[
      h('li', { class: 'lead' }, [h('b', { text: c.pending }), jumpLink(pendingId, t('tallyPending'))]),
      h('li', { class: 'lead' }, [h('b', { text: c.flagged }), jumpLink(firstWhere(id => flags(id).length > 0), t('tallyFlagged'))]),
      c.invalid ? h('li', {}, [h('b', { text: c.invalid }), jumpLink(firstWhere(invalid), t('tallyInvalid'))]) : null,
      h('li', {}, [h('b', { text: c.confirm }), t('confirm')]),
      h('li', {}, [h('b', { text: c.reject }), t('reject')]),
      h('li', {}, [h('b', { text: c.modify }), t('modify')]),
    ].filter(Boolean));
    const decided = c.total - c.pending;
    progress.replaceChildren(
      h('strong', { text: `${decided} / ${c.total}` }),
      h('small', { text: t('draftSaved') }),
      h('div', { class: 'bar', role: 'progressbar', 'aria-valuemin': 0, 'aria-valuemax': c.total, 'aria-valuenow': decided, 'aria-label': t('decided', { n: decided, total: c.total }) }, h('i', { style: `transform:scaleX(${decided / c.total})` })),
    );
    nextButton.disabled = !pendingId || phase !== 'editing';
    submitButton.disabled = phase !== 'editing';
    if (review.classList.contains('open') && phase === 'editing') renderReview([...review.querySelectorAll('.errors')].map(node => node.textContent));
    if (phase === 'submitting' && review.classList.contains('open')) renderReview([]);
    if (phase === 'submitted') {
      document.body.classList.add('submitted');
      signInner.replaceChildren(h('dl', { class: 'done', role: 'status' }, [
        h('div', {}, [h('dt', { text: t('submitted') }), h('dd', { text: t('reviewCounts', { c: counts().confirm, r: counts().reject, m: counts().modify }) })]),
        h('div', {}, [h('dt', { text: t('signedBy') }), h('dd', { class: 'mono', text: new Date().toLocaleString(data.locale ?? 'zh-CN') })]),
        h('p', { text: t('submittedHint') }),
      ]));
    }
  }

  // mount({ kind, sections: [{ title, itemLabel?, items }], body(item) })：模板提供分区与条目正文。
  function mount({ kind, sections, body }) {
    const sheet = h('main', { class: 'sheet' }, sections.filter(section => section.items.length).map(section => h('section', { class: 'section', 'aria-label': section.title }, [
      h('h2', {}, [section.title, h('small', { text: `${section.items.length}` })]),
      h('div', { class: 'table' }, [
        h('div', { class: 'cols', 'aria-hidden': 'true' }, [
          h('span', { text: t('colId') }), h('span', { text: section.itemLabel ?? t('colItem') }), h('span', { class: 'c-flag', text: t('colFlag') }), h('span', { text: t('colDecision') }),
        ]),
        section.items.map(item => renderRow(item, body)),
      ]),
    ])));
    document.body.replaceChildren(renderHead(kind), sheet, renderSign());
    subscribe(refresh);
    refresh([]);
  }

  function subscribe(fn) { listeners.add(fn); }

  window.DecisionForm = { data, t, h, facts, refs, tags, mount };
})();
