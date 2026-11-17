# Portal 约定素材

以下为合并到目标 `AGENTS.md` 的候选内容；保留条件、按语义去重，链接按目标环境重新定位。

## Portal 技能索引

按任务读取对应 skill，不一次加载全部。缺失时说明并提示安装，不声称已使用。

| 任务 | Skill |
| --- | --- |
| App 前后端开发、manifest、鉴权、配置分层、跨 App 交换与日志 | [portal-external-app](../../portal-external-app/SKILL.md) |
| 共享 onebox 初始化、注册、启动、体检与排错 | [portal-dev-setup](../../portal-dev-setup/SKILL.md) |
| 前端产物、后端镜像引用、清单提案与发布确认 | [xgent-app-release](../../xgent-app-release/SKILL.md) |
| App 镜像构建、私有 Harbor 推送与 CI | [xgent-image-push](../../xgent-image-push/SKILL.md) |

安装 `@xgent/*` 只需读 [xgent-app-release / references/private-npm.md](../../xgent-app-release/references/private-npm.md)，无需加载完整发布流程。

## Portal 开发约定

Odyssey（ODC）：`RULE` 声明规则；`MUST` 必须，`MUST_NOT` 禁止，`SHOULD` 通常应遵守，`PREFER` 表示偏好；`WHEN` / `UNLESS` 限定条件，列表逐项适用。

**本文 `formal` 代码围栏中的内容是以 DSL 形式编写的执行指令，必须按各条规则的义务强度、适用条件与明确例外遵循。`formal` 代码围栏仅用于排版，不表示这些指令是示例或可选参考。**

### 本地联调：共享 onebox

```formal
RULE @portal_local_development:
  WHEN: portal_app_local_development_or_debugging
  MUST: ["先读 portal-dev-setup，按其脚本与现有配置接入本机共享 xgent-onebox；先查已有栈，一台机器复用一盒。",
         "App 所需 PG、Redis、RustFS、Kafka 等基础设施使用共享 onebox 的实例，地址与端口从实际配置读取。",
         "镜像仓库、凭证与目录取自现有配置；缺凭证请用户找开发团队。",
         "文档与行为不符先核对当前镜像版本；区分 onebox 与 App 的 registry 项目，App 镜像架构须匹配运行平台。"]
  MUST_NOT: [initialize_second_onebox_when_existing,
             "连接供 Portal 自身调试的独立 PG/Redis/RustFS/Kafka，或为 App 另起一套替代 onebox。",
             clone_or_modify_portal_for_app_debugging,
             rebuild_portal_images_without_instruction,
             guess_credentials_or_connection_values,
             use_development_oauth_or_keys_in_production,
             commit_credentials]

RULE @portal_local_database:
  WHEN: local_development_requires_app_database
  MUST: ["按本仓配置核对 onebox 中所需 App 库，创建或迁移前确认目标；onebox 不自动创建每个出仓 App 的库。",
         "onebox 没有部署控制器；换入带新迁移的 App 镜像后，按本仓迁移入口显式执行。"]
  MUST_NOT: health_green_implies_schema_current

RULE @portal_integration_acceptance:
  WHEN: portal_app_integration_or_local_verification
  MUST: ["本地 iframe 握手、TDT、consent 与服务反代必须接真实共享 onebox 验证。",
         "注册先于反代启动；有市场卡片的 App 先确认演示租户市场中可见，再检查安装、授权、宿主入口与服务路由。",
         "404/502 先运行 onebox doctor，按 portal-dev-setup 排查注册、反代与后端。",
         "navItems、helpEntry 等安装期快照变更后，经租户管理员市场更新入口同步已安装 App，再验宿主行为。",
         "将首次同意屏的授权并打开纳入走查；onebox 无部署控制器，任务停在 deploying 时以实际宿主链路判断可用性。",
         "有本仓 onebox 冒烟入口时运行，再从宿主验证实际功能；无前端服务验证真实授权与服务路由。"]
  MUST_NOT: [frontend_dev_server_proves_portal_integration,
             dev_registration_implies_installed_snapshot_updated]
```

### 私有 npm 依赖

```formal
RULE @portal_npm:
  WHEN: installing_xgent_npm_dependencies
  MUST: ["@xgent/* 从平台私有 registry 安装；先读索引中的 private-npm.md，按实际配置获取短期只读令牌。",
         "缺发布凭证请开发者从门户生成配置文件；版本未发布请平台团队发布，受阻时停止依赖安装。"]
  MUST_NOT: [vendor_sdk_sources_or_artifacts,
             substitute_file_link_or_tar_dependencies,
             reimplement_existing_sdk_to_bypass_installation,
             request_cloud_credentials_for_package_installation]
```

### 前端 UI 开发与验收（有前端时）

```formal
RULE @portal_ui_design:
  WHEN: frontend_page_component_layout_interaction_or_visual_changes
  MUST: ["设计使用 phoenix-ui，先读已有产品与设计上下文，检查视觉层级、信息架构、间距对齐、配色、动效、可访问性及空/错误态。",
         "缺 phoenix-ui 时说明缺失并提示安装，不声称已经使用。"]

RULE @portal_ui_acceptance:
  WHEN: frontend_page_component_layout_interaction_or_visual_changes
  MUST: ["使用当前 Agent 可用的真实浏览器通道：Chrome extension、kimi-webbridge、Computer Use 或 Playwright。",
         "从已有 Portal 宿主入口进入 App，确认最新产物，走通主路径与关键边界；首次接入覆盖市场卡片、安装和授权。",
         "检查改动涉及的导航、面包屑、关键操作、保存结果、空/错误态和权限边界；涉及响应式、主题或语言时检查对应状态。",
         "查看 console 与失败请求，保留实际步骤、结果及必要截图作为证据。",
         "环境起不来先按现有配置排查；仍受阻明确写未在浏览器中验证，说明原因与所需协助。"]
  MUST_NOT: [standalone_or_dev_server_as_only_acceptance,
             typecheck_lint_test_build_or_diff_replaces_browser,
             claim_ui_verified_without_browser_evidence]
```

### API：HTTP 状态只反映传输/路由层

```formal
RULE @portal_http_status:
  WHEN: implementing_app_business_http_api
  MUST: ["404 仅表示 endpoint 不存在；数据不存在用 200 + 业务字段，如 data: null。",
         "业务成功统一 200，含新建、更新、删除、覆写，不用 201/204。",
         "业务失败（业务校验不过、状态非法、资源冲突）用 200 + 响应体错误结构。",
         "4xx/5xx 仅用于参数不正确、未登录、未授权或真正的传输/服务故障。"]
```

### 性能：请求与 worker 热路径

```formal
RULE @portal_hot_path_batching:
  WHEN: request_or_worker_hot_path
  MUST: ["同批 id 用一次 SQL 批量查询（如 inArray）后内存映射；Redis 用 mget，同请求/批次同一事实只查一次。",
         "批量写用多行 INSERT + RETURNING，按业务键映射；onConflictDoNothing 只返回插入行，冲突部分另查。",
         "超过 1,000 行分块，且遵守 PG 单语句 65,535 参数上限。"]
  MUST_NOT: [per_item_await_queries_in_loops,
             small_tables_or_batches_excuse_n_plus_one]

RULE @portal_hot_path_cache:
  WHEN: request_or_worker_hot_path
  MUST: ["读多写少的配置/字典使用进程内短 TTL 缓存；管理改动须即时生效的配置在写侧显式 invalidateXxxCache()。",
         "App 名、图标、颜色等纯展示快照用 30s TTL，不要求写侧失效（worker 可能跨进程）。",
         "发送、扣费、授权前的当下资格（成员 active、App 仍安装、端点归属）用带索引的新鲜查询，批次仍须批量获取。",
         "/health、背压等热入口的 count(*) / min() 聚合共用短 TTL 缓存，不每次现算。"]
  MUST_NOT: cache_current_eligibility_before_side_effects

RULE @portal_hot_path_transactions:
  WHEN: request_or_worker_hot_path
  MUST: [transactions_cover_writes_only,
         rendering_decryption_and_provider_calls_outside_transactions]
  MUST_NOT: external_calls_inside_transactions

RULE @portal_hot_path_measurement:
  WHEN: hot_path_changed
  MUST: ["提供同参数下的前后压测数字（吞吐或 p95），写入提交说明；无适用压测的数据库路径至少提供前后 EXPLAIN。"]
  MUST_NOT: typecheck_lint_or_verify_as_performance_evidence
```
