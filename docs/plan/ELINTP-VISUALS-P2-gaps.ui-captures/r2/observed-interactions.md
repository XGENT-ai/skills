# R2 · 有效交互观察记录

本文件只汇总真实页面中已执行的动作与观察结果，并标明来源；R2 没有把这些复用操作冒充为重新执行。

## 1. U1 · 桌面亮色主动提交（apply R1 证据）

- 来源图片：[U1-submitted-after-1440-light.png](../r1-apply/U1-submitted-after-1440-light.png)。
- 实际入口：`http://127.0.0.1:55336/`；1440×900、DPR 1、light；2026-11-12T03:11:23.264Z。
- 已执行：开启“标注内容”；分别给普通段落、无图注现有图、无图注缺失图添加疑问 / 没看懂 / 建议及一句说明；刷新确认恢复“3 条标注”；主动点击“提交给 Agent”。
- 实际观察：HTTP 200；可见 `.review-status` 为“已提交给当前 Agent”，该元素 `aria-live="polite"`；提交后工具条按钮禁用；视口宽与页面 `scrollWidth` 均为 1440；无 `pageerror`。
- 实际结果文件：[after-1440-light.result.json](../r1-apply/logs/after-1440-light.result.json)。以下仅省略大段 `draft` 与 `assets` 内容字节，标题、轮次及全部提交项字段按原结果转录：

```json
{
  "schemaVersion": 1,
  "title": "空图注图片的审阅与离线交付",
  "round": 1,
  "submittedAt": "2026-11-12T03:11:23.208Z",
  "items": [
    {
      "blockId": "s1:b1",
      "sectionId": "s1",
      "section": "已交付的内容块与真实截图",
      "quote": "这是旧版已经能够登记的普通段落，用来核对升级前后的块身份与标注位置。",
      "kind": "question",
      "note": "请说明这段话"
    },
    {
      "blockId": "s1:b4",
      "sectionId": "s1",
      "section": "已交付的内容块与真实截图",
      "quote": "docs/diagrams/agi-mode-evolution-loop.visual-check.1440x900.light.png",
      "kind": "unclear",
      "note": "请重讲这张无图注图片"
    },
    {
      "blockId": "s2:b6",
      "sectionId": "s2",
      "section": "实际审阅与提交",
      "quote": "docs/diagrams/missing/empty-caption  diagram  with-a-very-long-name-that-exceeds-eighty-unicode-characters.png",
      "kind": "suggestion",
      "note": "请补图，保留这个完整原引用"
    }
  ]
}
```

## 2. U1 · 窄屏暗色主动提交（apply R1 证据）

- 来源图片：[U1-submitted-after-360-dark.png](../r1-apply/U1-submitted-after-360-dark.png)。
- 实际入口：`http://127.0.0.1:55356/`；360×800、DPR 1、dark；2026-11-12T03:11:24.398Z。
- 已执行：与第 1 节相同，分别添加三条标注、刷新恢复后主动提交。
- 实际观察：HTTP 200；可见状态为“已提交给当前 Agent”，`aria-live="polite"`；提交后工具条按钮禁用；视口宽与页面 `scrollWidth` 均为 360，状态文字完整可见；无 `pageerror`。
- 实际结果文件：[after-360-dark.result.json](../r1-apply/logs/after-360-dark.result.json)。其顶层 `title` 为“空图注图片的审阅与离线交付”，`round` 为 1；三项 `blockId`、`sectionId`、`section`、`quote`、`kind`、`note` 与第 1 节逐项相同，`submittedAt` 为 `2026-11-12T03:11:24.356Z`。

## 3. U1 · 未受影响状态（历史复用 R1）

原始记录：[R1 observed-interactions.md](../r1/observed-interactions.md) 的 U1 操作 1–12、21。R2 未重新执行这些动作。

- 操作 1–3：旧结果续接及同轮草稿升级后，普通段落保持 `s1:b1`，旧说明仍附着原块；两张空图注图片有独立控件。
- 操作 4–8：桌面/窄屏、亮暗四种条件下，键盘焦点依次到“跳到正文”“切换亮暗”“＋ 标注内容”；无标注时提交禁用并显示原因；两个空图注图片按钮的 `aria-label` 均带完整原引用。
- 操作 7：窄屏暗色编辑器中的长引用长度为 110，含连续双空格；输入框焦点轮廓可见，Escape 关闭后焦点返回。
- 操作 9–10：禁用脚本时正文保留、审阅工具条不出现；打印实际生成 PDF。
- 操作 11：三条标注刷新后仍恢复。
- 操作 12：R1 当时的成功状态为“已提交给 Agent”；该历史状态只作对照，不代表当前成功文案。
- 操作 21：编辑、删除、清空均有实际状态反馈；服务停止后提交出现 `role=alert` 错误，标注与说明保留，控件恢复可用，未生成结果文件。

## 4. U2 · 已通过单元继承材料（历史复用 R1）

原始记录：[R1 observed-interactions.md](../r1/observed-interactions.md) 的 U2 操作 13–20。R2 未重新执行，也不要求本轮独立评审重新给 U2 打分。实际记录覆盖离线重开、亮暗切换与持久化、大纲键盘定位、缺失原引用、无脚本和打印。
