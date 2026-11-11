export const TEMPLATES = {
  general: { slots: ['why', 'scale', 'risks', 'more'], required: ['why'] },
  prd: { slots: ['gain', 'delta', 'rollout', 'more'], required: ['gain', 'delta', 'rollout'] },
  crd: { slots: ['ask', 'stance', 'agreed', 'open', 'more'], required: ['ask', 'stance', 'agreed', 'open'] },
  plan: {
    todo: { slots: ['build', 'route', 'more'], required: ['build', 'route'] },
    partial: { slots: ['built', 'verify', 'build', 'route', 'gaps', 'records', 'more'], required: ['built', 'verify', 'build', 'route'] },
    done: { slots: ['built', 'verify', 'gaps', 'records', 'more'], required: ['built', 'verify', 'gaps'] },
  },
};

export const UI = {
  'zh-CN': {
    general: '技术说明', prd: '产品需求', crd: '跨项目需求', plan: '开发计划',
    todo: '计划记录：未完成', partial: '计划记录：做了一半', done: '计划记录：已完成',
    why: '为什么成立', scale: '量级', risks: '风险与前提', gain: '会多出什么', delta: '和现在差在哪', rollout: '建议怎么实施',
    ask: '谁向谁要什么', stance: '对方的立场与答复', agreed: '已经谈定', open: '还没谈定与下一步',
    build: '要开发什么', route: '技术路线与代价', built: '做出了什么', verify: '怎么自己验证', gaps: '遗漏与后续', records: '过程记录',
    outline: '大纲', theme: '切换亮暗', notes: '表头释义', sources: '出处', skip: '跳到正文', missing: '图片缺失', table: '数据表',
  },
  'zh-TW': {
    general: '技術說明', prd: '產品需求', crd: '跨專案需求', plan: '開發計畫',
    todo: '計畫記錄：未完成', partial: '計畫記錄：做了一半', done: '計畫記錄：已完成',
    why: '為什麼成立', scale: '量級', risks: '風險與前提', gain: '會多出什麼', delta: '和現在差在哪', rollout: '建議怎麼實施',
    ask: '誰向誰要什麼', stance: '對方的立場與答覆', agreed: '已經談定', open: '還沒談定與下一步',
    build: '要開發什麼', route: '技術路線與代價', built: '做出了什麼', verify: '怎麼自己驗證', gaps: '遺漏與後續', records: '過程記錄',
    outline: '大綱', theme: '切換亮暗', notes: '表頭釋義', sources: '出處', skip: '跳到正文', missing: '圖片缺失', table: '資料表',
  },
  en: {
    general: 'Explainer', prd: 'Product requirements', crd: 'Cross-project request', plan: 'Development plan',
    todo: 'Plan record: not done', partial: 'Plan record: partly done', done: 'Plan record: done',
    why: 'Why it holds', scale: 'Scale', risks: 'Risks and conditions', gain: 'What is new', delta: 'What changes', rollout: 'How to roll out',
    ask: 'Who asks whom for what', stance: "The other side’s position", agreed: 'Agreed', open: 'Still open and next step',
    build: 'What will be built', route: 'Approach and cost', built: 'What was built', verify: 'How to check it yourself', gaps: 'Gaps and follow-ups', records: 'Process records',
    outline: 'Outline', theme: 'Switch theme', notes: 'Column notes', sources: 'Sources', skip: 'Skip to content', missing: 'Image missing', table: 'Data table',
  },
};

export function interfaceText(lang) {
  const tag = lang.toLowerCase();
  if (['zh', 'zh-cn', 'zh-sg'].includes(tag) || tag.startsWith('zh-hans')) return UI['zh-CN'];
  if (['zh-tw', 'zh-hk', 'zh-mo'].includes(tag) || tag.startsWith('zh-hant')) return UI['zh-TW'];
  return UI.en;
}

UI['zh-CN'].review = {
  toggle: '标注内容', submit: '提交给 Agent', clear: '清除全部', empty: '先选择内容块并添加标注',
  count: '条标注', annotate: '添加标注', edit: '编辑标注', remove: '删除标注',
  unclear: '没看懂', question: '疑问', suggestion: '建议', note: '一句说明（可选）',
  save: '保存', cancel: '取消', kind: '标注类型', confirmClear: '清除全部标注？',
  storage: '当前浏览器不能保存审阅草稿，请不要刷新，先提交',
  sending: '正在提交…', sent: '已提交给 Agent', offline: '本机服务不可达，标注已保留。请让 Agent 用原端口重开本轮后再提交。',
  badJSON: '提交内容不是合法 JSON', badRound: '轮次不匹配，请打开当前审阅页',
  badItems: '至少添加一条标注后再提交', badItem: '标注的内容块、小节、类型或文字无效，请核对后再提交',
  writeError: '结果文件写入失败，请让 Agent 检查结果路径后重试',
};
UI['zh-TW'].review = {
  toggle: '標註內容', submit: '提交給 Agent', clear: '清除全部', empty: '先選擇內容區塊並新增標註',
  count: '條標註', annotate: '新增標註', edit: '編輯標註', remove: '刪除標註',
  unclear: '沒看懂', question: '疑問', suggestion: '建議', note: '一句說明（選填）',
  save: '儲存', cancel: '取消', kind: '標註類型', confirmClear: '清除全部標註？',
  storage: '目前瀏覽器無法儲存審閱草稿，請勿重新整理，先提交',
  sending: '正在提交…', sent: '已提交給 Agent', offline: '本機服務無法連線，標註已保留。請讓 Agent 用原連接埠重開本輪後再提交。',
  badJSON: '提交內容不是合法 JSON', badRound: '輪次不符，請開啟目前審閱頁',
  badItems: '至少新增一條標註後再提交', badItem: '標註的區塊、小節、類型或文字無效，請核對後再提交',
  writeError: '結果檔案寫入失敗，請讓 Agent 檢查結果路徑後重試',
};
UI.en.review = {
  toggle: 'Annotate content', submit: 'Submit to Agent', clear: 'Clear all', empty: 'Choose a content block and add feedback first',
  count: 'annotations', annotate: 'Add annotation', edit: 'Edit annotation', remove: 'Delete annotation',
  unclear: 'Unclear', question: 'Question', suggestion: 'Suggestion', note: 'One-line note (optional)',
  save: 'Save', cancel: 'Cancel', kind: 'Annotation type', confirmClear: 'Clear all annotations?',
  storage: 'This browser cannot save review drafts. Please submit before refreshing.',
  sending: 'Submitting…', sent: 'Submitted to Agent', offline: 'Local service is unreachable. Annotations are kept. Ask the Agent to reopen this round on the original port, then submit again.',
  badJSON: 'Submission is not valid JSON', badRound: 'Round mismatch. Open the current review page.',
  badItems: 'Add at least one annotation before submitting', badItem: 'Invalid annotation block, section, type or text. Check it and submit again.',
  writeError: 'Could not write the result file. Ask the Agent to check its path, then retry.',
};
