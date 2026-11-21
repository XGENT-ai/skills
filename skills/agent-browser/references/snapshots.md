# 快照与 ref：从页面读到可执行的证据

`agent-browser snapshot` 读取当前页面的可访问性树。浏览器把页面解释为按钮、标题、输入框、状态区等节点；缩进表达它们在这棵树里的从属关系。一次快照回答的是「在当前会话、标签页、frame 和过滤条件下，浏览器此刻暴露了什么」。像素位置、被遮挡的区域和绘在 canvas 上的文字需要另外观察。

## 读树：从一行按钮找到一笔订单

设用户要保存订单 B，列表里 A、B 各有一个 `Save`。下面是本地订单页在保存前的快照片段，取自 `snapshot -s "#B"`：

```text
- article "Order B" [ref=e2]
  - heading "Order B" [level=2, ref=e8]
  - button "Save" [ref=e9] hint="in:Order B Open B"
  - status
    - StaticText "Open B"
```

- **角色与名称**：`button` 是角色，`Save` 是可访问名称。名称可能来自按钮文字、关联标签或 `aria-label`。它告诉你控件做什么，父级 `article "Order B"` 告诉你这次操作影响谁。
- **属性与值**：`level=2` 是标题层级。其他节点可带输入值、勾选或禁用状态。一次填表要核对当前值时可用 `get value @eN`；要判断能否提交可用 `is enabled @eN`。
- **ref 与普通文字**：把 `[ref=e9]` 写作 `@e9` 传给命令。这里的回执文字没有 ref，但仍可作为状态证据。ref 的数字既不表示行号，也不表示业务 ID。
- **hint**：部分版本会给同名或无名控件附上 `in:` 上下文，也可能显示 `icon:`、`img:`、`data-testid:`、`id:` 或 `name:`。这些是辨认线索。例中的 `find role button … --name` 匹配的是 `Save`；把 hint 填进 `--name` 会查错字段。遇到没有 hint 的版本，仍可用父行辨认。

```formal
WORKFLOW @target_from_snapshot:
  TRIGGER: action_requires_element
  DO: [identify_requested_record, inspect_role_name_and_ancestors,
       resolve_one_control_in_current_context, use_observed_ref]
RULE @ambiguous_control:
  WHEN: multiple_controls_still_match
  MUST: obtain_distinguishing_context_before_action
  MUST_NOT: select_by_ref_number_or_incidental_order
```

## 选视图：取决于接下来要作的判断

下表省略命令前缀 `agent-browser`。在命名会话中沿用同一个 `--session`。

| 现场问题 | 从哪里开始 | 信息不够时怎么调整 |
| --- | --- | --- |
| 下一步能点、选、填什么？ | `snapshot -i` 筛出交互目标及相关上下文 | 要解释错误提示、回执或只读正文时，去掉 `-i` |
| 页面太长，但已经知道要查哪个区域 | `snapshot -s "#orders"`。可以加 `-i` 专看该区域的操作 | `-s` 接受 CSS 选择器。选择器匹配多处时，先核对业务记录；匹配零处时退回上层，检查标签页、frame 和容器是否加载 |
| 只想看大纲或减少空层级 | `snapshot -d 2` 限深；`snapshot -c` 压缩结构 | 深度截断可能留下 `status` 而略去子文本。确认状态时去掉限深；紧凑输出缺字时，对同一业务容器取完整快照 |
| 需要辨认同名链接去向 | `snapshot -i -u` 同时列出链接目标 | 结合所在行与 URL 判断；如任务只要求读地址，取 `get attr @eN href` 即可 |
| 要在脚本里处理多个候选 | `snapshot --json`；成功响应中读 `data.snapshot` 和 `data.refs` | `refs` 以 `eN` 为键，可查 role/name；两个 Save 的 role/name 可以完全相同。祖先关系和可能出现的 hint 要回到 `data.snapshot` 中读 |

**版本差异会改变证据。** 同一测试页保存 B 后，`0.33.2` 的 `snapshot -c -s "#B"` 省略了状态区，`0.43.0` 保留了 `status → StaticText "Draft B"`。两个版本的 `snapshot -s "#B"` 都读到了回执。在新版中，`-i` 还可能把 “Draft B” 带进按钮 hint；正文检索命中这几个字时，仍需看它属于哪个节点。

## ref 何时还能用

| 两次命令之间发生了什么 | 对下一次操作的影响 |
| --- | --- |
| 同一文档重取快照，原 DOM 节点仍在 | 该元素的 ref 可以保持。仅修改旁边一段文字，无需因此假定所有 ref 都失效。虚拟可访问性节点的 ref 只在产生它的快照内解释 |
| 前端用新节点替换按钮，角色和名称相同 | CLI 在角色和名称能唯一识别时可能重新定位。即使旧 ref 的一次读取成功，也要核对新节点所属记录；列表排序、同名操作或弹窗变化时重新取快照 |
| 页面或 iframe 导航、刷新，或切了标签页 | 在目标上下文重新获取 ref。旧编号在另一份快照出现，并不能证明它仍指向原来的元素 |
| 指定区域取快照失败，或标注截图失败 | 这次调用没有产生新映射。当前文档未变时上次成功的引用可能仍有效；若页面正在替换，等预期标题、区域或正文出现后再取。不要从失败输出猜编号 |

```formal
RULE @refresh_reference:
  WHEN: ||[navigation, frame_navigation, tab_switch, target_replacement]
  MUST: match_target_again_in_destination_context
RULE @incomplete_observation:
  WHEN: expected_node_missing_from_filtered_snapshot
  MUST: inspect_scope_depth_context_and_readiness
  MUST_NOT: infer_absence_from_filtered_view
```

## 一个闭环：保存 B，证明 A 没变

1. **建立初始证据。** 从完整树确认 B 的状态是 `Open B`。测试隔离性时也记录 A 的状态。示例中的 `#B` 是测试页面已知的容器选择器；真实页面没有已知选择器时，先读完整树中的标题、行或列表项。
2. **执行一次。** 从 B 的子树得到按钮 ref；若当前输出确实是上例，执行 `click @e9`。保存后等待预期回执，例如 `wait --text "Draft B"`。文本等待只是读取结果的时机。
3. **用相同业务范围验收。** 执行 `snapshot -s "#B"`，确认 B 下的 `status` 从 `Open B` 变为 `Draft B`。若任务还要求 A 保持原状态，再读 A 的状态区。若要求证明已经持久保存，则重新打开该订单，核对读回的状态。
4. **结果不明时。** 超时后先读当前 URL、弹窗、错误提示和记录状态。若已保存就交付回执；若结果仍不可判定，报告最后观察到的状态，保留现场。

需要比较较长的树时，可在动作前存 `agent-browser snapshot -s "#B" > before.txt`，动作后用 `agent-browser diff snapshot --baseline before.txt -s "#B"`。前后使用相同范围和过滤条件。比较时可能同时看到按钮编号变化和回执文字变化。本例能证明目标状态改变的是 B 的回执从 `Open B` 变为 `Draft B`；仅有编号、加载提示或按钮禁用的变化，还需继续核对结果。

## iframe、无名图标与视觉内容

- **iframe**：若主树中的 `Iframe "Editor"` 下已经列出 `textbox "Memo"`，直接使用子控件 ref 填写即可。只看编辑器时，以当前 frame 节点的 ref 运行 `frame @eN`，再取快照；完成后 `frame main` 回到外页并重新定位。自动展开有层级限制，空 frame 或无法读取的跨域子树也可能缺席。此时先确认 frame 已加载、当前上下文正确，再判断可观察范围。
- **无名图标、遮挡、canvas**：在支持标注的浏览器上运行 `screenshot --annotate controls.png`，从本次输出的图例把像素位置与 `@eN` 对上。标注截图本身可以生成供后续操作使用的 ref。canvas 内自绘的每一项未必有独立节点；只有图像线索而无法确定操作目标时，保留视觉证据和不确定项。
- **从页面表现转向组件原因**：若完整快照已证明回执缺失，而任务还要解释 React 页面为何没有更新，转入[React 运行时诊断](react.md)。快照里的 `@eN` 用于页面交互，组件检查使用 React 树给出的 fiber ID。

```formal
REQUIREMENT @snapshot_evidence:
  MUST: verify_requested_state_on_intended_record
  ACCEPTANCE:
    GIVEN: multiple_records_share_action_name
    WHEN: requested_record_changes
    EXPECT: result_observed_under_matching_record
```

选项及版本行为可核对[快照参考](https://agent-browser.dev/snapshots)与[快照差异说明](https://agent-browser.dev/diffing)。
