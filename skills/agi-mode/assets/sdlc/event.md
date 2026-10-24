---
schema_version: 3
event_id: "<新 UUID，与文件名相同>"
object_id: "<沿用对象 UUID；新对象才生成>"
kind: "<prd / plan / review>"
title: "<可读标题>"
artifact: "<仓根相对路径或稳定 URL>"
facet: "<document / readiness / implementation / review / disposition 等>"
scope: "<稳定范围，如 plan、M2、F-01>"
state: "<state-model.md 对应 kind/facet 的固定状态码：按判定顺序取最强可支撑状态，没有可读依据才用 unknown>"
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

- 独立留存理由：<不可变审计要求的出处；或原记录、可取回版本及当前页均不足以保留的具体并发/纠错事实；不能仅写里程碑完成、提交或状态更新>
- 必须保留的事实或差异：<仅写独立留存所需内容；已有实施、验收、评审及处置事实直接链接原记录章节和版本，不再概述一遍>
- 直接影响与未解项：<本事实改变的状态/范围、缺证或解除条件；原记录已有的只引用，无新增则省略>

<!-- 创建前先核对原记录；仅在仓内协议要求独立留存时使用，无法说明留存理由则不创建。普通进度和最终完成直接更新对象状态页，审计另有必填内容时从其要求。创建时替换占位并移除此注释。source_state 有原文状态则保留，否则 null；candidate_state 仅在 state 为 unknown 时使用对应枚举。kind=review 时补 review_type: prd / dev-plan / code / other。sources.path、artifact 按仓根解析，正文链接按本文件目录解析。relations 如 {type: covers, target: 对象UUID, scope: FR-1}；评审用 reviews，复审用 rechecks。 -->
