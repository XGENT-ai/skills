---
schema_version: 3
event_id: e89a7fe9-ad1b-4223-adb2-c73ad4d6efda
object_id: 7818a838-d021-4b2e-92c7-6d57c7ae8af0
kind: plan
title: Phoenix UI 第十次原机轻量通过与隔离补采核心
artifact: docs/plan/phoenix-ui.md
facet: implementation
scope: plan
state: dev-plan-in-progress
source_state: 进行中；第十次已推送，Windows轻量测试通过并进入构建，五平台完整runtime及后置smoke待结果；M5纯库36项局部通过，M4/M5保持隔离
evidence_status: confirmed
observed_at: '2026-10-10T03:25:04+08:00'
predecessors:
- 30e911a8-d345-4b07-9b33-2d7d7e667514
baseline:
  head: 6aec456772e9511ef43c29991a22ff999167e13e
  branch: phoenix-ui/m2-verification-20261009
  worktree: dirty
sources:
- path: docs/plan/phoenix-ui.md
  revision: sha256:6ca48ae69f1b7a853e111ba3440b4c608e673ace5d6a35222d49a39cef905fd3
  anchor: 当前状态与本次实际检查范围
- path: docs/plan/phoenix-ui.records/M2.md
  revision: sha256:cf0dbec6206a636622ac44b32c353de5368384c441cd05e7f8ac52d084f6ded6
  anchor: 当前状态与本次实际检查范围
- path: docs/plan/phoenix-ui.records/M5.md
  revision: sha256:cedfa200ad17ef0ec812477d0cbb78f3686c4de14775055d5daa77084f36af7f
  anchor: 当前状态与本次实际检查范围
- path: local/phoenix-ui/ci-m2-37977469565.json
  revision: sha256:c92114d685af21cb300bc0663e19ed1b1d8491bb3e7c4a8483659adbe17dbb7a
  anchor: 当前状态与本次实际检查范围
- path: local/phoenix-ui/ci-m2-linux-x64-37977469565.log
  revision: sha256:9474613aea5515b7caf415af83adc3dea91c378888715374f90c45e39a23d8de
  anchor: 当前状态与本次实际检查范围
- path: local/phoenix-ui/ci-m2-linux-arm-37977469565.log
  revision: sha256:77fe6e31f3dce4a54ab66d0640699481754c3c1f3dbf451125fe08673dcf5e40
  anchor: 当前状态与本次实际检查范围
- path: local/phoenix-ui/ci-m2-darwin-arm-37977469565.log
  revision: sha256:274114365b0d0d5fd440d85782e6fc428faef1ad52505b923d763557f52cac88
  anchor: 当前状态与本次实际检查范围
- path: local/phoenix-ui/ci-m2-darwin-x64-37977469565.log
  revision: sha256:db3d991adda239196f1d2dc66faea3a7a45c42c4fbd7c0d0e57638b570da0225
  anchor: 当前状态与本次实际检查范围
- path: local/phoenix-ui/ci-m2-windows-37977469565.log
  revision: sha256:b5fe2b5918e94b4ad2499a3959043975d30934d9a17a01106261cea124d915b9
  anchor: 当前状态与本次实际检查范围
- path: local/phoenix-ui/ci-m2-37979057972-start.json
  revision: sha256:9f2d761d39bbf26aa480bf6271968ba7e6ed492999bc3d58625df6788a806889
  anchor: 当前状态与本次实际检查范围
- path: local/phoenix-ui/ci-m2-37979057972-midrun.json
  revision: sha256:c0f323c22d1885631e81ff485570b4eb2f953a2e2e2cee4ce66a860a6f48da1a
  anchor: 当前状态与本次实际检查范围
- path: local/phoenix-ui/m5-capture-source-check.json
  revision: sha256:90cd3d234d4fc99ba85c55f22ce9f383af79c3acb9d4ae1fb5add27ae248ebe0
  anchor: 当前状态与本次实际检查范围
- path: local/phoenix-ui/m5-capture-final.log
  revision: sha256:5c9cc0e5d8759b2679228db19d3f00ef5dfc3a4ec72f2219eab11e4d95caa71d
  anchor: 当前状态与本次实际检查范围
- path: local/phoenix-ui/m5-capture-clippy.log
  revision: sha256:b1f521f806dd928d3ad4592a66450419c98240c61ddf6fa9c76bd48d24d2a25b
  anchor: 当前状态与本次实际检查范围
---

# 第十次运行中与局部补采

- 第九次实际终态cancelled，Windows失败、四个非Windows及两后置smoke取消；五平台完整job日志保存。第十次6aec456已正常推送，19:23:12Z Windows light实际success并进入构建；五平台完整runtime和后置smoke待结果。
- M5补采8项、存储8项、完整纯库36项与严格clippy通过；15份源码及原文快照已保存，Node/Python沿未受影响原范围复用。原图真实性、viewport尺寸、完整round/CLI、journal和封存持久化均待实现。
- 纠正前序事件M5来源中锁恢复的「新进程」表述：实际是父进程在持锁子进程被终止且wait后重新取锁，当前M5正文已写准；不改历史事件或冻结版本。
- 仍Ready/in-progress/1/6。当前三份文档原文冻结于local/phoenix-ui/ci10-start-source-versions；local证据及B源码只在原工作树可见，A/B隔离保留。
