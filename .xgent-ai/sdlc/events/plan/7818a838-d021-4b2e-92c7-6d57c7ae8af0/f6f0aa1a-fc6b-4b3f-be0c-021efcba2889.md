---
schema_version: 3
event_id: f6f0aa1a-fc6b-4b3f-be0c-021efcba2889
object_id: 7818a838-d021-4b2e-92c7-6d57c7ae8af0
kind: plan
title: Phoenix UI 第十次Linux全组与Windows构建通过
artifact: docs/plan/phoenix-ui.md
facet: implementation
scope: plan
state: dev-plan-in-progress
source_state: 进行中；第十次Linux x64全组成功，Windows light/build/smoke通过、完整runtime仍运行，其他平台及后置smoke待结果；M5纯库41项局部通过，M4/M5保持隔离
evidence_status: confirmed
observed_at: '2026-10-10T03:35:14+08:00'
predecessors:
- e89a7fe9-ad1b-4223-adb2-c73ad4d6efda
baseline:
  head: 6aec456772e9511ef43c29991a22ff999167e13e
  branch: phoenix-ui/m2-verification-20261009
  worktree: dirty
sources:
- path: docs/plan/phoenix-ui.md
  revision: sha256:18975023ec99d8d74070b4346cbd043ad9718591bd21544532d38638a2a908c2
  anchor: 当前状态与本次实际检查范围
- path: docs/plan/phoenix-ui.records/M2.md
  revision: sha256:73d0f3f210d1c75007a4b87e5f7c4a30c808afdbfef175b8e07e6931f438448e
  anchor: 当前状态与本次实际检查范围
- path: docs/plan/phoenix-ui.records/M5.md
  revision: sha256:3e6ff28cda1b3281ca0109d6c86dbf5a7ae0e0b21cc10873148ef8066bad24a7
  anchor: 当前状态与本次实际检查范围
- path: local/phoenix-ui/ci-m2-37979057972-midrun-2.json
  revision: sha256:3270d2383f4d706f2117a6df29de0e3c59ca07bb27076d5f6066aec5555cda87
  anchor: 当前状态与本次实际检查范围
- path: local/phoenix-ui/ci-m2-linux-x64-37979057972.log
  revision: sha256:5a06d1d0643dc5be9b9fac0cee5d18c59130967770293bb70627b166603fc71c
  anchor: 当前状态与本次实际检查范围
- path: local/phoenix-ui/m5-projection-source-check.json
  revision: sha256:c8449fcdd752485a35bbd7d0472f4d8b2ead92b60a6d57d49b5b52c55a2f5815
  anchor: 当前状态与本次实际检查范围
- path: local/phoenix-ui/m5-projection-final.log
  revision: sha256:926abd8bf7099a20d8d71e73b4672076868f11beb4a2345a5f4c5c424fbfb0ee
  anchor: 当前状态与本次实际检查范围
- path: local/phoenix-ui/m5-projection-clippy.log
  revision: sha256:64672c21a3307edd12e3230f364a7c5616d9d5eb63524035e3f7e0bf4b0797ac
  anchor: 当前状态与本次实际检查范围
---

# 第十次仍运行

- 同一推送head6aec456，19:33:16Z Linux x64完整成功，原始日志有75组Rust865/0/9和WASM8474/72零差异；Windows light/build/smoke原机成功、runtime仍运行，其他平台及两后置smoke待结果。
- M5纯生成块投影5项与完整新crate41项、严格clippy通过；块外字节保留、块内手改冲突覆盖，17份源码及快照已保存。journal、完整round/CLI、import及真实V-7/V-8尚缺，保持B隔离。
- 仍Ready/in-progress/1/6。当前三份文档源版本冻结于local/phoenix-ui/ci10-linux-source-versions；本次只写进度，local日志及B源码仅原工作树可见。
