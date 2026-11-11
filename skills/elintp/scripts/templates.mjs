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
