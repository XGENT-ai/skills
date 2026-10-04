---
schema_version: 3
event_id: "<新 UUID，与文件名相同>"
object_id: "<沿用对象 UUID；新对象才生成>"
kind: "<prd / plan / review>"
title: "<可读标题>"
artifact: "<仓根相对路径或稳定 URL>"
facet: "<document / readiness / implementation / review / disposition 等>"
scope: "<稳定范围，如 plan、M2、F-01>"
state: "<state-model.md 对应 kind/facet 的固定状态码，不能确认用 unknown>"
source_state: null
evidence_status: "<confirmed / provisional / stale / conflict / unknown>"
observed_at: "<ISO 8601，带时区>"
predecessors: []
baseline:
  head: "<完整 SHA 或 unknown>"
  branch: "<分支、detached 或 unknown>"
  worktree: "<clean / dirty / unknown>"
sources:
  - path: "<仓根相对路径或稳定 URL>"
    revision: "<Git 版本、sha256:内容指纹或 unknown>"
    anchor: "<稳定章节/检查/问题定位>"
relations: []
---

# <本次事实摘要>

- 事实与证据：<本次读到/验证了什么；评审写种类、所审对象版本和范围，区别于观察 checkout>
- 校准依据：<规范状态与来源自述的差异、证据覆盖与原文滞后；一致时简述依据>
- 未完项与限制：<缺证、未同步、待决；没有则写无，不填猜测>
- 下一步及解除条件：<动作 + 可观察完成判据；无后续则说明>

<!-- 创建事件时替换占位并移除此注释。source_state 有原文状态则保留其文字，否则 null；可选 candidate_state 只能写模型中的对应枚举。kind=review 时补 review_type: prd / dev-plan / code / other。sources.path、artifact 均按仓根解析；正文 Markdown 链接按本文件目录解析。relations 如 {type: covers, target: 对象UUID, scope: FR-1}；评审对所审对象用 reviews，复审对前轮用 rechecks。 -->
