# Fixture UI target

## 单元

| 单元 | 入口 | 必需状态 | 视口 | 主题 | 来源 |
| --- | --- | --- | --- | --- | --- |
| U1 任务列表 | / | default、empty、error | desktop 1440×900、mobile 390×844 | light、dark | [PRODUCT.md](PRODUCT.md)、[DESIGN.md](DESIGN.md) |
| U2 筛选弹窗 | / | dialog-open、dialog-closed | desktop 1440×900、mobile 390×844 | light、dark | [DESIGN.md](DESIGN.md) |

## 规范要点

按 [DESIGN.md](DESIGN.md) 的既定规则保留字体、语义、焦点、主题和窄屏行为。按 [seed.json](seed.json) 核对任务内容。错误页和空状态不得登记为 default 实拍。

## 采集

运行 `node server.cjs`，从 stdout 读取实际 loopback URL。查询参数 `state=empty`、`state=error`、`theme=dark` 固定状态；默认种子每次启动相同。通过 Filter/Done 或 Escape 验证弹窗和焦点。

## 变更记录

初始 fixture 目标；后续改变期望须登记用户决定并重建基线。
