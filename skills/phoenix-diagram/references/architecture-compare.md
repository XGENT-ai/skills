# 架构比较

仅在用户明确要求比较两份架构时使用。两份输入都必须是 `architecture`；其他图型继续使用各自的单图流程。比较只陈述输入中可追溯的变化，不推断运行影响、因果、风险或可合并性。

## 输入与生成

保留原 JSON。组件通过 `components[].id` 对应，连接通过作者明确写入的 `connections[].id` 对应，边界通过 `kind + label` 对应。边界键须唯一，两份输入至少共享一个组件 ID；两侧都写仓库时必须属于同一仓库。连接缺稳定 ID 时，普通单图仍可使用，比较会拒绝；由作者确认对应关系后再补 ID，不能按数组位置、标签相似度或端点猜同一关系。

```sh
"$PD" compare architecture base.json head.json comparison.html --json
"$PD" compare architecture base.json head.json comparison.html \
  --base-contract legacy-2.17 --head-contract phoenix-v1 \
  --quality showcase --repo-root "<仓库根>" --brand-cache "<本地品牌缓存>" --json
```

合同选择按每侧明确标记处理，不重写源文件。`--quality` 与输入已有质量标记冲突时会失败；不要为换取比较通过而降低质量。仓库来源和品牌继续使用已固定的本地身份，普通比较不联网采集资源。

默认比较回执为 `comparison.html.compare.json`；`--receipt PATH` 可指定同一物理父目录中的另一个文件。两份输入、HTML 和比较回执必须是四个不同的普通文件，拒绝符号链接或硬链接别名。

比较回执包含双侧原始/语义摘要、编译配置、引擎、来源、品牌资源、比较 IR、HTML 身份和成对事务位置。`status: pass` 只表示本次比较和双成果发布已成功，`browserStatus: not-run`、`visualReview.status: required-pending`、`deliveryStatus: incomplete` 仍须如实保留。它不等于默认交付完成。

## 阅读三个视图

Before、Delta、After 分别展示原版、变化和新版。Delta 保留完整的新版事实，旧位置或已移除内容以幽灵层作对照；这些辅助图元不成为新版 Reader 的真实拓扑。点击变化列表定位相应对象，保留未变事实和每侧完整标签、来源及 Reader 控件。无变化时仍可读取原版和新版。

字段变化分类为 semantic、evidence、geometry、topology、scope。来源变化与移动、改线等几何变化分开；数组重排、对象键顺序、来源条目顺序或 `meta.output` 改动不自行成为业务变化。仅在两侧来源已验证、仓库相同且提交已固定时为 `proofLevel: revision-pinned`，其他情况为 `authored`。

## 自动检查与实际看图

```sh
"$PD" check comparison.html --strict-provenance --json
"$PD" visual-check comparison.html --out-dir compare-evidence-01 --json
```

按 `visual-check` 返回的 `evidence.summaryReceipt` 和 `evidence.browserReceipt` 中的真实路径读取文件；两者都含 `identity: {sha256, bytes}`，不要猜文件名。比较的独立 summary 为 `kind: architecture-comparison-summary`，包含完整 `comparison` 绑定，以及真实的 `check`、`visual-check` 两门。它不复用普通 finalize 的四门 summary，也不改写原 HTML 或原比较回执。

每个视图分别测量四个桌面窗口及窄屏，暗亮主题分别检查。实际看图使用浏览器回执中的 12 张当前截图：每个视图的 1440×900、2048×1320 各 light/dark。逐张打开，核对主图、标签、卡片、关系遮挡、层级和控件；Delta 还须核旧位置、删除标记和变化定位与输入相符。DOM 数值、截图存在或自动测试不能替代观察。

自动两门均通过后，`automatedStatus` 才为 `pass`，视觉仍是 `required-pending`、交付仍为 `incomplete`。缺工具、浏览器失败、某视图缺测或尚未实际看图分别报告，不沿用普通单图或另一次比较的审核。

## 创建完整审核草稿

将以下 summary 路径换成 `visual-check` 回执中的真实值。Node.js 18+ 可运行，不需要源码或 Rust。示例只复制绑定，不能替代打开图片；空 reviewer、`observed: false` 和 `verdict: unavailable` 必须按实际观察填写。

```sh
node - "<比较summary真实路径>" "comparison-review-01.json" <<'NODE'
const fs = require('node:fs');
const assert = require('node:assert/strict');
const [summaryPath, recordPath] = process.argv.slice(2);
const summary = JSON.parse(fs.readFileSync(summaryPath, 'utf8'));
const names = ['check', 'visual-check'];
if (summary.kind !== 'architecture-comparison-summary' || summary.automatedStatus !== 'pass'
    || summary.stages.length !== 2 || !names.every((name, index) =>
      summary.stages[index].name === name && summary.stages[index].status === 'pass')) {
  throw new Error('Both current comparison automated gates must pass before review.');
}
const browserRef = summary.evidence.browserReceipt;
const browser = JSON.parse(fs.readFileSync(browserRef.path, 'utf8'));
assert.deepStrictEqual(browser.comparison, summary.comparison);
if (browser.status !== 'pass' || browser.screenshots.length !== 12) {
  throw new Error('Twelve current screenshots from all three views are required.');
}
const expected = new Set(['before', 'delta', 'after'].flatMap(view =>
  ['1440x900', '2048x1320'].flatMap(size => ['light', 'dark'].map(theme => `${view}/${size}/${theme}`))));
const actual = browser.screenshots.map(shot =>
  `${shot.viewId}/${shot.viewport.width}x${shot.viewport.height}/${shot.theme}`);
if (new Set(actual).size !== 12 || actual.some(key => !expected.has(key))) {
  throw new Error('Comparison screenshot view, viewport or theme coverage differs.');
}
const record = {
  schemaVersion: 1,
  kind: 'architecture-comparison-review',
  command: 'review-record',
  comparison: summary.comparison,
  browserReceipt: browserRef,
  screenshotIdentities: browser.screenshots.map(shot => ({
    viewId: shot.viewId,
    file: { path: shot.path, identity: shot.identity },
    viewport: shot.viewport,
    theme: shot.theme,
    observed: false,
    observations: ''
  })),
  reviewer: '',
  scope: ['diagram', 'labels', 'cards', 'relationship-occlusion', 'hierarchy', 'controls'],
  verdict: 'unavailable',
  findings: ['Image review has not been performed.']
};
fs.writeFileSync(recordPath, JSON.stringify(record, null, 2) + '\n', { flag: 'wx' });
NODE
```

填写真实 reviewer；已打开的每张图写 `observed: true` 与非空 `observations`。没有缺陷才能用 `verdict: passed`、`findings: []`；有缺陷用 `failed` 并写具体 findings；无法看图用 `unavailable` 并写实际限制。所有记录都必须保留双输入完整绑定与 12 张原件身份，不加未知字段，不用单图记录替代。

```sh
"$PD" review-record comparison.html --record comparison-review-01.json --json
```

读取返回的 `evidence.summaryReceipt.path`。只有同一 `comparison` 的两门为 pass、`automatedStatus: pass`、`visualReview.status: passed`、`deliveryStatus: complete`，身份仍匹配当前输入、配置、资源、引擎、IR、HTML、比较回执和截图，才能报告完成。`failed` 或 `unavailable` 的观察结果保持 `incomplete`。

相同已封存记录可幂等重试。若封存完成但 summary 更新失败，保留同一记录字节，解决摘要冲突或输出恢复后重试 `review-record`；不要再次 compare/visual-check 取得另一个身份。不同记录不覆盖已封存记录；输入或浏览器证据发生变化，需要新的对应审核。

## 成对恢复

HTML 和比较 JSON 通过一个专用私有 journal 发布。短暂看见其中一个文件不表示成对成功；失败或中断后保留真实状态与恢复位置。读取回执的 `transaction.directory` 或诊断中的实际 `recoveryDirectory`，确认属于本次已授权输出后，显式传该目录：

```sh
"$PD" recover-compare "<确切私有比较事务目录>" --json
```

未 committed 的事务恢复旧对，已 committed 的事务只清理本事务临时物。后来修改、重建目录、未知 claimant 或备份变化均保留并报告 `recovery-required`，重复恢复同一事务应幂等。stdout 丢失时，在原输出父目录只读定位 `.phoenix-diagram-compare-<id>` 并核其目标；多个候选须明确选择，不按修改时间推定，不改记录或删除备份强行通过。恢复只处理文件事务，不自动补浏览器或实际看图通过。
