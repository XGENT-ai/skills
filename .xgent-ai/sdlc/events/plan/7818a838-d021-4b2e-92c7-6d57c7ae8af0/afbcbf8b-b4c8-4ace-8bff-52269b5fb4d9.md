---
schema_version: 3
event_id: "afbcbf8b-b4c8-4ace-8bff-52269b5fb4d9"
object_id: "7818a838-d021-4b2e-92c7-6d57c7ae8af0"
kind: "plan"
title: "Phoenix UI · M1 基线收口，进入 M2"
artifact: "docs/plan/phoenix-ui.md"
facet: "implementation"
scope: "plan"
state: "dev-plan-in-progress"
source_state: "实施中；1/6"
evidence_status: "confirmed"
observed_at: "2026-10-08T19:12:49+08:00"
predecessors: ["53ddb5a9-1d41-43ae-bcf4-6358206082ae"]
baseline:
  head: "b2289fa05f1e4f0251bc3a86e3c04f182aba156e"
  branch: "main"
  worktree: "dirty"
sources:
  - path: "docs/plan/phoenix-ui.md"
    revision: "sha256:78109be51c95eb02118e62c1de8b9e61b4a09b866083ea70884e8ae291b6b45f"
    anchor: "实施进度、M1、V-1"
  - path: "docs/plan/phoenix-ui.records/M1.md"
    revision: "sha256:f4709927439ae464dc4dfb49c101c6051c99418de47a5f243ea694378dd263da"
    anchor: "交付与验证、原版失败分型与 M2 前置"
---

# M1 固定来源与基线已收口

- 事实与证据：[计划](../../../../../docs/plan/phoenix-ui.md)进度 1/6；[M1记录](../../../../../docs/plan/phoenix-ui.records/M1.md)逐项保存固定来源、真实旧包/Web fixture、mise/mbx、build/test/oracle/lint/WASM与性能结果。
- 校准依据：V-1 建立原版基线已完成；原版失败、忽略及缺录制向量有据分型，native/WASM 27 数值差异未当作 V-2 通过。仍有 M2–M6 必需工作，规范状态为 in-progress。
- 未完项与限制：未提交、未推送，原始日志/大包在忽略的本地目录，仅当前checkout可复现；结构化来源/结果/hash已落盘。跨平台、provider真会话、迁移器与完整UI循环尚未验证；当前无已证实阻塞。原 readiness/评审处置保持独立，不重复评审。
- 下一步及解除条件：M2 接管公共身份/状态根/网络与生成器，关闭数值/就绪竞态和已知补丁缺口，开发tarball与五平台CI候选可审阅后请求推送授权。
