# 交付与实际审核

读本文件处理 finalize、诊断、实际图像审核、review-record、环境失败或输出恢复。命令使用 `SKILL.md` 中已解析的 `PD`，从用户工作目录执行。

## 四门与身份

```sh
"$PD" finalize <type> candidate.json diagram.html --json
```

固定顺序为 `validate → deliver → check → browser-check`。首个非 pass 阶段后其余阶段保持 `not-run`。`validate` 做输入/布局与独立 artifact 检查；`deliver` 发布本次 HTML；`check` 严格绑定当前交付；`browser-check` 检查浏览器实测并采集当前截图。读取每个阶段真实 status，不能补造通过。

stdout 是单一 JSON。保留首次成功自动验收的 `deliveryReceiptId`、`browserReceiptIdentity`、`candidate` / `artifact` 的 `path` 与 `identity:{sha256,bytes}`、`configSha256` 和完整 `engine`。HTML 已提交不等于整体完成；自动四门通过后，`visualReview.status` 仍为 `required-pending`，`deliveryStatus` 为 `incomplete`。

从 `evidence.summaryReceipt.path` 或 `evidence.summaryPath` 定位本次 summary，从 `evidence.browserReceipt.path` 读取绑定的浏览器回执。summary 在输出旁，`--out-dir` 指定截图/浏览器证据目录；按回执给出的真实路径读取，勿猜测文件名或跨图混用。

失败时使用当前 summary 的 diagnostics。`producedByThisRun: false` 或 validate/deliver 未过时，不对输出路径执行视觉验收：该路径可能仍是旧的 last-good 文件。修复 candidate 后重新 finalize，使用新的 `--out-dir` 留存新证据；原 candidate、HTML、配置、引擎或截图身份变化后，旧审核不能继承。仅为取完整状态或登记看图结果不得再次 finalize。

## 浏览器与实际看图

自动检查覆盖 1440×900、1600×1000、1920×1080、2048×1320 桌面；实际审核使用回执中的四张截图：1440×900 与 2048×1320 各 light/dark。打开每张当前图片，检查主图、全部标签/notes、卡片、关系遮挡、层级和控件。截图文件、digest、DOM 数值或测试输出都不代表已经观察。

每个桌面尺寸必须无水平溢出。纵向滚动仅按 Reader 声明和测量接受：`intrinsic-height` / `width-first` 的自动画布，或 Architecture 显式 viewBox 的 `authored-height`，需完整内容留在 panel 内，无内部滚动或裁剪，页面允许纵向滚动，且回执确认可读并给出 `verticalScrollAccepted: true` / `overflowDisposition: "readable-vertical-scroll"`。未知声明、水平溢出、不可读文字和 chrome 遮挡仍失败。不能手工改 DOM 属性伪造该例外。

定向诊断可用下列公开命令；它们不替代同一交付的四门 summary：

```sh
"$PD" validate <type> candidate.json --quality showcase --layout-json --json
"$PD" check diagram.html --strict-provenance --json
"$PD" browser-check diagram.html --out-dir evidence-2 --json
"$PD" visual-check diagram.html --out-dir evidence-3 --json
```

环境 `unavailable`、执行或断言 `failed`、后续 `not-run` 与未实际看图必须分别报告。自动门失败先修该门；没有图像能力时不得从测量推测 `passed`。不自动安装引擎或浏览器，也不打开用户现有浏览器、启动 preview。

## 生成完整审核草稿

以下命令只从**当前自动四门已通过**的 summary/browser 回执复制身份，创建一个新文件。先将 summary 和 review 路径换成真实路径。Node.js 18+ 即可运行，不需要仓库或 Rust。它不会替你看图，草稿中的空 reviewer、`observed: false` 和 `verdict: "unavailable"` 必须按实际情况处理；`status: "pass"` 是记录协议字段，不是视觉通过。

```sh
node - "<当前summary路径>" "review-01.json" <<'NODE'
const fs = require('node:fs');
const [summaryPath, recordPath] = process.argv.slice(2);
const summary = JSON.parse(fs.readFileSync(summaryPath, 'utf8'));
const names = ['validate', 'deliver', 'check', 'browser-check'];
if (summary.automatedStatus !== 'pass' || summary.stages.length !== 4
    || !names.every(name => summary.stages.filter(stage => stage.name === name && stage.status === 'pass').length === 1)) {
  throw new Error('The current four automated gates must pass before review.');
}
const browser = JSON.parse(fs.readFileSync(summary.evidence.browserReceipt.path, 'utf8'));
if (browser.screenshots.length !== 4) throw new Error('Four current screenshots are required.');
const bound = file => ({ path: file.path, identity: file.identity });
const record = {
  schemaVersion: 1,
  tool: 'phoenix-diagram',
  toolVersion: summary.toolVersion,
  command: 'review-record',
  status: 'pass',
  diagnostics: [],
  artifact: bound(summary.artifact),
  candidate: bound(summary.candidate),
  configSha256: summary.configSha256,
  engine: summary.engine,
  deliveryReceiptId: summary.deliveryReceiptId,
  browserReceiptIdentity: summary.browserReceiptIdentity,
  screenshotIdentities: browser.screenshots.map(screenshot => ({
    file: bound(screenshot),
    viewport: screenshot.viewport,
    theme: screenshot.theme,
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

实际打开四张图片后，填写真实 reviewer；逐张写 `observed: true` 和非空 `observations`，描述实际看到的上述六类内容。无缺陷才能设 `verdict: "passed"`、`findings: []`；有缺陷用 `failed` 并列具体 findings。确实没有图像工具/审核者时使用 `unavailable`、真实 reviewer 标识与非空限制说明，保持未完成。不要新增未知 JSON 字段、改身份、改 viewport/theme 或把观察复制到未打开的截图。

## 原身份收口

```sh
"$PD" review-record diagram.html --record review-01.json --json
```

读取返回的 `evidence.summaryReceipt.path`，再读该交付的当前 summary。核对四门全部 pass、`automatedStatus: "pass"`、`visualReview.status: "passed"`、`deliveryStatus: "complete"`；`deliveryReceiptId`、`browserReceiptIdentity`、输入/HTML/配置/引擎绑定须与首次审核材料一致。仅凭 review-record 退出码或截图存在不能称完成。

相同已封存记录可幂等重试；若记录已保存但 summary 写失败，保留同一记录字节并重试 review-record。不要再次 deliver/finalize 取得另一个身份，也不要覆盖已封存的不同记录。用户要求或实际缺陷导致改 candidate 时，生成新交付和新证据，再做对应审核。

## 输出恢复

`commitState` / diagnostics 表明 `recovery-required` 时，报告实际 public 文件状态及保留记录。在原输出父目录执行 `ls -la "<原输出父目录>"`，只读核对记录中的 target。设原输出父目录为 `P`、记录内 transactionId 为 `ID`，同一事务的记录可能处于以下三处：

1. `P/.phoenix-diagram-recovery-ID/publication-recovery-v1.json`。
2. 清理中的 sibling witness：`P/.phoenix-diagram-recovery-ID.cleanup.json`。
3. 最后清理 quarantine：`P/.phoenix-diagram-output-cleanup/ID.json`。固定目录为当前用户 0700，保留自身绑定的 0600 `owner-v1.json`；有效 owner 的空壳允许保留，不等于 pending。

对已授权、与本次目标绑定的事务，**始终显式传原 recovery-ID 目录，即使该目录已不存在**。不能把 witness 或 quarantine JSON 路径当作参数；命令只从这个原目录路径导出同 ID 的另外两处记录。

```sh
"$PD" recover-output "P/.phoenix-diagram-recovery-ID" --json
```

将 P/ID 换为已识别的真实值。多个记录必须明确选择，不扫描后自动挑事务，不按目录时间推断。未知 owner、额外条目、替换对象或多处记录歧义均保留拒绝；不得删除、移动或重写它们来强行通过。

读取返回的 `publicState`、`preserved`、`recoveryStatus`、diagnostics 和退出码。原目录不存在但可信 witness/quarantine 记录仍在时须继续由命令验证，不能自行宣称 absent。恢复只处理文件事务，仍需适用的交付检查与实际图像审核；后来的 claimant 或绑定改变应被保留并如实报告。
