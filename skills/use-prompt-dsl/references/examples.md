# Prompt DSL 示例

按当前任务选择相关示例，保留用户自己的目标、标准与交付范围。这里的循环次数、风格和检查项是示例参数。

## UI 开发：延续社区提示词

```text
::OBJECTIVE{site_design|pri:OVERRIDE_ALL}
  target: 将当前页面打磨到获奖作品级视觉水准
  ACCEPT: 所有 CHECK 均通过，按实际画面与交互判断没有尚未处理的可见差距
  BENCHMARK: Awwwards | Webby Awards | FWA 获奖作品；实际选取与当前产品相适配的参考

::INPUT{current_page}
  source: 待提供：当前项目与可运行页面入口

::CONSTRAINT{product}
  保留既定产品目标、品牌与必须支持的功能；以适合当前产品的方式采用视觉参考

::CHECK{typography|standard:award_tier}
::CHECK{whitespace|standard:award_tier}
::CHECK{visual_hierarchy|standard:award_tier}
::CHECK{color|standard:award_tier}
::CHECK{motion|standard:award_tier}
::CHECK{micro_interaction|standard:award_tier}
::CHECK{responsive|standard:award_tier}
::CHECK{originality|standard:award_tier}

::FLOW{build}
  [READ] @current_page 与已有产品、设计约束
  [DO] 查阅适配当前产品的参考，确定设计方向并完成页面初次实现

::LOOP{self_review|trigger:on_completion|max:3}
  [READ] 当前页面的真实渲染画面，并实测相关交互与响应式状态
  [COMPARE] 逐项核对全部 CHECK，与实际查看过的 BENCHMARK 参考比较
  [IF] 存在 UNKNOWN => [DO] 尝试取得缺少的证据
  [IF] 仍存在 UNKNOWN => [BREAK] 列出尚无法验证的维度
  [IF] gap found => [PATCH] 修正具体差距 => [VERIFY] 受影响的检查 => [CONTINUE]
  [IF] no gap => [Ω]

::OUTPUT{page}
  include: 页面实现、实际参照、关键设计取舍、验证证据与未完成项（如有）
```

这保留了原提示词的高标准、八项检查与完成后返工。`award_tier` 由执行者结合产品和实际参照解释；某项不适用时给出依据，不能仅因没有实现或没有观察就标为通过。

## 文档改写：限定范围并逐项核对

```text
::OBJECTIVE{rewrite}
  target: 将 @draft 改写为客户可读的产品说明
  ACCEPT: 非技术读者能理解产品用途、操作步骤和限制，所有事实与原稿一致

::INPUT{draft}
  value: 待提供：原稿正文

::CONSTRAINT{facts}
  不增加原稿未承诺的功能、效果、日期或价格

::CHECK{accuracy}
  OBSERVE: 将改写稿中的事实主张逐项对应到 @draft
  PASS: 每项事实有原稿依据，限定条件没有遗漏

::CHECK{readability}
  OBSERVE: 检查用途、步骤与限制是否容易找到，必要术语是否有解释
  PASS: 表达直接、指代明确，读者无需了解实现细节即可完成操作

::LOOP{edit|max:2}
  [READ] 当前改写稿与 @draft
  [VERIFY] @accuracy 与 @readability
  [IF] 任一 CHECK 为 UNKNOWN => [BREAK] 说明缺少的原稿信息
  [IF] gap found => [PATCH] 修正事实或表达问题 => [CONTINUE]
  [IF] no gap => [Ω]

::OUTPUT{copy}
  format: 中文产品说明
```

## 操作示例

```text
$use-prompt-dsl compose 为摄影师作品集写开发提示词，黑白风格、移动端优先，需要完成后逐项检查
$use-prompt-dsl execute 按下面的 DSL 修订指定文件并核对结果：……
$use-prompt-dsl explain 解释下面的循环在何时停止：……
```
