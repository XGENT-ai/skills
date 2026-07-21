# FILE-ORGANIZE · XGENT.ai Portal · 文件整理（目录树 · 两级标签+Tag cloud · Grid 缩略图 · files 既有 App 增量）

> 站在 `goal/PLAN-4.md`（文件管理 App 本体）、`goal/FILE-STORAGE-2.md`（应用存储 + `requireActor` + `viaAppKey`）、`goal/FILE-PREVIEW.md` / `FILE-PREVIEW-2.md`（file_renditions 懒生成管线 + ffmpeg poster）之上。本版已并入首轮设计评审 8 条 + **第二轮评审**（缩略图先入队后占锁、默认分类 `isSystem` 身份、多级 mkdir 递归、个人标签不进 searchText/旧列、标签数量契约、空目录隐式子树判定等）+ **产品拍板**：共享标签仅租户 admin 创建；`file_tags.meta` 固定 schema（置信度/兴趣度/匹配度等，可加性升级）。
>
> 本期交付：**文件管理的「整理」能力**——① 个人/团队空间内**新建目录**并用**目录树**浏览；② **两级标签体系**（标签分类 > 标签；租户共享词典 + 个人增补不共享；tagging 记录可携带 KV 元数据）+ **Tag cloud** 筛选；③ Grid 视图对**图片+视频**显示服务端缩略图。
>
> **应用存储（`kind=app` 空间）的目录树/标签 UI、AI 自动打标签、目录级权限、`files.tags` 旧列的物理删除——不在本期**（§0.3）。本期只做用户视角空间（personal/team）的整理面；`files.folder` 字符串与 `files.tags` 契约对既有消费方与滚动部署中的旧进程保持完全兼容。

---

## 0. 本期范围与决策

文件整理不是新产品，是 files 的**组织维度补全**：`files.folder`（schema.ts:75，presign 已接收落库 files.ts:82,115）与 `files.tags text[]`（schema.ts:84，GIN 索引已有 0000:102）两根「只写不读」的柱子早已立好，本期的工作量不在「新建存储」，而在 **① 目录从扁平字符串升级为可显式创建/可浏览的树（空目录要能存在，隐式路径要连得通）**、**② 标签从自由字符串数组升级为「分类 > 标签」词典 + tagging 关系（带 meta），同时不震断 5 个既有消费方的字符串契约与「先迁移后重启」的部署序**、**③ 新增独立的 thumbnail 生成机制（有界并发），并消灭 Grid 每文件一次 preview 调用的 N+1**、**④ 前端从零建三个整理面（树/标签/cloud）**。关键架构决策：

| # | 决策点 | 选择 | 含义 |
| --- | --- | --- | --- |
| 1 | 形态 | **files-server / files-app 纯增量** | 零新端口（:4100/:5303 不变，`apps/web/vite.config.ts:16-21`）、零新部署件、零新 scope（复用 `files.read`/`files.write`）、零 web/proxy/CSP 改动（grid 已渲染 presigned `<img>`，img-src 已通）。root 脚本仅增 verify 挂链。 |
| 2 | 目录模型 | **物化路径**：`files.folder` 路径字符串继续是文件的唯一目录指针；新增 `folders` 表只登记「显式创建的目录」（让空目录存在） | 与现状及 library/git 消费方已用的字符串 folder（`apps/library-server/src/lib/files.ts:172-179`、`apps/git-server/src/lib/files.ts:111-138`）同源；目录树 = 显式行 ∪ 全部已知路径的**祖先前缀闭包**（消费方可直接写 `/资料/合同/` 而不产生 `/资料/`，读侧必须把每个路径展开成全链）；子树操作 = 事务内字符串前缀重写。**不引入 folderId 外键**。 |
| 3 | 目录覆盖 | **仅 personal/team 空间**；app 空间拒绝 folders 端点（`SPACE_KIND_FORBIDDEN`） | 用户拍板（个人+团队空间）。服务态 app actor 经 presign 写 folder 字符串的现状**不动**；list `folder` 精确过滤参数对 app 空间也可用（纯过滤，无树）。 |
| 4 | 标签模型 | **三表：`tag_categories`（租户级，sort，**`isSystem`**）> `tags`（scope=tenant 共享 / personal 个人不共享）> `file_tags`（tagging：fileId+tagId+**固定 schema meta**）** | 用户拍板模型。字典范式照 qbank `tags` 表（`apps/qbank-server/src/db/schema.ts:42-60`），关联表范式照 `resource_tag_links`（`apps/library-server/src/db/schema.ts:115-134`）。**共享标签创建/改/删 = 租户 admin**；个人标签仅 owner 自建自用。tagging meta 为**版本化固定 schema**（置信度/兴趣度/匹配度 + source/model 等，§0.2g），**API-only，UI 本期不编辑**；升级只允许**增 key**。默认分类靠 **`isSystem=true` 稳定身份**（§0.2i）。 |
| 5 | 存量 `files.tags text[]` | **Expand/Contract 两期**：本期 expand——0004 建表+回填、**全部读写路径切到词典**（读从 `file_tags` 组装、写以 `file_tags` 为主并**同步双写旧列保鲜**）、旧列保留不 drop；**contract（drop 列）留到下一期**（§0.3），届时先重跑幂等回填调和再 drop | 部署序是「先 migrate 后重启/滚动」（`deploy/pm2/deploy.sh:322-326` Phase 4 先于 pm2 reload），本期 drop 列会让旧进程立即报错；且 `processors.ts:87` 仍写 `tags: []`、`fileDTO`（files.ts:31）读 `f.tags`——读写切换必须覆盖全部调用点（§4、M2）。`FileDTO.tags` 恒为名称数组（**仅含当前用户可见标签名**），`finalize`/`PATCH` 的 `tags: string[]` 契约不变（最多 **24 个** 名称，与现状 `slice(0, 24)` 一致）。 |
| 6 | 缩略图 | **独立 `lib/thumbnails.ts` 机制（ensure/get），不依赖 preview provider 的 strategy 分支**；`kind:"thumbnail"`（`RENDITION_KINDS` 已含，constants.ts:345）；ffmpeg/sidecar 转换姿势参数化宽度（320px）；**进程内 FIFO 队列 + semaphore 有界并发**；**先入队、worker 内再占锁**（§0.2d） | 已核实：原生图片与 Web 视频走 `native` 分支直接返回原文件（preview.ts:290-299），`?kind=` 仅在既有 rendition provider 内生效——「天然可用」不成立，必须显式分支。已核实：转换会把完整对象读入内存并 spawn ffmpeg（preview.ts:183,189），per-file 锁不限制跨文件并发——list 只入队，由队列控速（§0.2d、§4）。 |
| 7 | 前端 IA | **空间视图加左目录树面板**（`TreeItem` 字段形状照抄 `apps/knowledge-app/src/components/Tree.tsx:10-18`，**数据一次拉全**，不必抄 knowledge 的懒加载 `loadChildren`）；新增「标签」nav → `/tags` 页（cloud 筛选 + 词典管理）；grid `Thumb` 扩展视频并优先用 `thumbUrl` | nav/manifest 增量照 FILE-STORAGE-2 姿势（`apps/api/src/db/provisioning.ts:70+` navItems + `manifests.ts:110-115` FILES_MANIFEST pages 各加一行）。全部弹窗 in-DOM（iframe 内 `prompt/confirm` 被忽略，memory: microapp-ui-gotchas）。 |
| 8 | 多租户 | **强制租户隔离** | 三张新表全带 `tenantId`，所有查询按 gate claims 收口；他租户资源一律 `NOT_FOUND`（门户硬约束）。 |

> 命名与不变量：目录路径**严格规范**——写入（mkdir/rename/presign folder/PATCH folder）只接受「规整路径」：首尾各一个 `/`、段非空（`a//b` 拒绝）、段禁 `\` `.` `..` 与控制字符、单段 ≤100、全长 ≤255（与 presign 现有 `slice(0,255)` 对齐）；非法一律 `FOLDER_INVALID_PATH`，**不做宽松归一**。存量数据读侧原样呈现（不追溯规范化）；非法历史路径**可读不可改**（UI 禁止重命名/移动并提示）。**根目录 `/` 不可变**：不可改名/移动/删除（`FOLDER_ROOT_IMMUTABLE`），恒存在。默认分类展示名初值 `未分类`（租户数据，非 i18n key），身份靠 `isSystem=true`（§0.2i）。标签契约：每文件最多 **24 个** 名称（与现状一致）；单名 trim 后长度 ≤64、空名丢弃。缩略图规格：jpeg、宽 ≤320、`_previews/<fileId>/thumbnail.jpg`。

### 0.1 最重要边界：files 拥有组织维度，消费方拥有业务语义

| | **files（本期）** | **消费方 App（library/git/spms/todo/task）** | **平台门户（apps/api）** |
| --- | --- | --- | --- |
| 拥有 | 目录树/标签词典/tagging/缩略图的存储、闸与 API | 各自业务实体的 folder 字符串/tags 字符串写法（现状不动） | files listing 的 nav/manifest 一行增量 + verify-all SUITE 一行 |
| 红线 | — | **不读 file_tags/folders 内部表**（只经 REST）；不感知词典存在 | 目录/标签**不进平台 ACL**（沿用 FILE-STORAGE-2「文件管理自己的颗粒度」立场，§0.3） |

一句话：**目录与标签是 files 的内部组织维度——消费方的字符串写法零改动、由 files 服务端默默归一；平台只多一个导航入口和一条 verify 套件。**

### 0.2 关键取舍（显式记录）

- **a. 物化路径而非目录实体指针（决策 2）。** 选它因为 `files.folder` 已是事实模型且两个消费方在写。代价：① 目录重命名/移动 = 事务内对 `folders` + `files` 两表做子树前缀重写，子树大时写放大；② 隐式目录无登记行，树连通性靠读侧祖先闭包补齐，重命名隐式目录时才物化落行。缓解：重写全部包在一个 tx；`files (tenant_id,space_id,folder)` 新索引；`expandAncestors` 在读侧唯一函数完成。
- **b. 标签退役走 expand/contract 而非本期 drop（决策 5）。** 已核实部署「先 migrate 后重启」（deploy.sh:322-326）——本期 drop 列 = 旧进程立即报错，评审 P0。选：本期 expand（建表、回填、读写全部切词典、**写路径双写旧列保鲜**，旧进程在滚动窗口内读旧列与新读路径结果一致）；contract（drop）下一期。代价：① 双写是过渡税——每条写路径要同时维护 `file_tags` 与 `files.tags` 名称数组（收敛在 `replaceFileTags` 唯一函数，processors 派生写空集同理）；② 部署窗口内**旧进程的写只落旧列**，词典会漏——缓解：回填脚本独立成 `scripts/backfill-tags.ts`（幂等、`ON CONFLICT DO NOTHING`），0004 迁移内跑一次，下一期 contract 前再跑一次调和；③ list/get 多一次按页 `inArray(fileIds)` 组装（照 `apps/library-server/src/lib/cards.ts:53-64` 范式）。回归门：verify-search `?tags=` 断言原样绿 + processors 派生回归 + 双写一致性断言（**双写名称集 = 该文件上 `scope=tenant` 标签名**，见 §0.2e）。
- **c. 共享标签 CRUD 收敛为租户 admin（产品拍板，原开放问题①）。** 词典是**租户级共享资产**——`POST/PATCH/DELETE` 凡 `scope=tenant`（及分类管理）一律 `requireAdmin`；成员（非 admin）**不可**经 HTTP 建共享标签，只能**选用**已有共享标签，或建/管自己的 `scope=personal`。代价：① admin 未预置词典时成员侧共享可选集为空，只能先用个人标签；② 冷启动依赖 admin 或字符串路径 `ensureTagsByNames`（见下）。缓解：`GET /tags/categories` 仍 ensure 默认 system 分类；**`ensureTagsByNames`（finalize/PATCH 字符串 tags）继续在 system 分类下 get-or-create `scope=tenant` 标签**——这是消费方旧契约（spms/todo/task 任意字符串 tags 可落库）的**有意例外**，不经 HTTP 词典闸，避免震消费方；UI「新建共享标签」仅 admin 可见。权限矩阵见 §4 / §6 第 1 条。
- **d. 缩略图：list 只入队、队列控速；先入队、worker 内再占锁（决策 6，评审 P0-3 + 第二轮 P0）。** 已核实转换全量读对象入内存 + spawn ffmpeg（preview.ts:183,189）、per-file 锁不限制跨文件并发——一页 200 个缺失项直接 detached = 200 个 ffmpeg。选：进程内 FIFO 队列 + semaphore（`THUMB_CONVERT_CONCURRENCY`，默认 2）；源大小上限沿用 `PREVIEW_MAX_RENDITION_BYTES`（200MB，constants.ts:339，超限不生成）；队列上限 `THUMB_QUEUE_MAX`（默认 500）；attempts 上限沿用 `PREVIEW_MAX_ATTEMPTS=3`；防同文件踩踏沿用 `acquireRenditionLock`（preview.ts:202-227）。**顺序钉死（避免「先锁后跳过」卡死 pending）：**
  1. eligible 且非 fresh-ready；
  2. 进程内已在队 / 去重 Set 命中 → 跳过（无害）；
  3. 队列满（`THUMB_QUEUE_MAX`）→ **不入队、不写 pending**（纯背压，下页请求自然补）；
  4. 入队成功后返回；**此时尚未占 DB 锁**；
  5. worker 出队 → `acquireRenditionLock` → 赢则 `convertToImage(width=320)` → 输/超限/失败按既有 rendition 语义处理。
  
  代价：① 进程内队列不跨副本共享——多副本各自入队，锁保证同文件只转一次，队列重复入队是无害冗余（入队前 in-memory Set 去重减噪，赢锁者才真转）；② 首刷大图目录渐现而非全现；③ 本地 ffmpeg 路径仍是全量读入内存（semaphore × 200MB 封顶内存尖峰，sidecar 路径无此问题）。显式承认。
- **e. 个人标签的计数与展示仅对 owner 本人；且不进旧列双写 / searchText。** `file_tags` 上的个人标签在 DTO 组装、cloud 计数、`tagIds` 过滤三处统一收口为「`scope=tenant` ∪（`personal` ∧ `ownerUserId=当前用户`）」。**额外：`replaceFileTags` 双写 `files.tags` 与重建 `searchText` 时，名称集只含 `scope=tenant` 的标签**——个人标签只活在 `file_tags`，避免他人 `?q=` 全文或旧进程读旧列时「搜到/看到」个人标签名（第二轮 P1）。代价：① 同一文件不同用户看到的 DTO.tags 不同（产品语义如此）；② expand 窗口内旧进程读旧列看不到个人标签（可接受——个人标签本就不是旧契约的一等公民）；③ 个人标签随 owner 离租成为孤儿数据。缓解：孤儿不显示不计数（查询天然过滤），治理清理出本期；显式承认。
- **f. 目录删除仅限空目录（无直接文件且无隐式/显式子路径）。** 空判定钉死：① 直接文件 = `files.folder = path` 计数；② 子目录 = 存在 `folders.path` 或 `files.folder` 以 `path` 为**严格前缀**（路径恒带尾 `/`，前缀匹配安全，覆盖「只有 `/a/b/` 文件、无 `/a/b/` 行」的隐式子树）。代价：删非空目录须先搬空，体验硬。缓解：`FOLDER_NOT_EMPTY` 错误体带直接文件数/子目录数，UI 明示；递归删除/回收站出本期（§0.3）。
- **g. tagging meta 固定 schema（产品拍板，原开放问题③），API-only，可加性演进。** 列类型仍为 `jsonb`，写入经 `parseFileTagMeta(raw)` 唯一校验函数；**未知 key 拒绝**（`TAG_META_INVALID`），禁止自由塞字段。本期 **UI 不展示不编辑**。演进规则：系统升级**只增可选 key**（旧行缺新 key = 未设置），不改名/不删 key/不改既有 key 语义；`schemaVersion` 标识结构代际（初值 1）。代价：AI/调用方必须跟白名单；过严时要发版才能加字段（产品接受）。缓解：初版 key 覆盖打标评分 + 溯源（§2 meta 表）；单条序列化 ≤2KB；分数一律 `[0,1]` 有限 number。
- **h. 名称契约拆两个函数（评审 P1-6）。** 旧契约只携带名称，而新唯一键允许跨分类同名。定死：**读路径 `?tags=name` 走只读 `findTagIdsByNames`**——跨全部可见分类匹配同名标签 id 并集（保持 text[] 的名称语义），未知名 → 空集 → 空结果，**GET 绝不落库**；**写路径（finalize/PATCH 携带名称）才走 `ensureTagsByNames`**——名称 trim / 去空 / 单名 ≤64 / **最多 24 个**（与现状 `slice(0, 24)` 数组长度契约一致，**不是**单名长度 24）→ 默认 system 分类 get-or-create（scope=tenant）→ tagIds。PUT `/:id/tags` 走 tagId，不经名称。代价：同名多分类时按名过滤会命中多个标签（any-of 并集），语义与旧 text[] 完全一致；按 id 过滤（`tagIds`）才有分类精度。显式承认。
- **i. 默认分类稳定身份（第二轮 P1）。** `tag_categories.isSystem boolean notNull default false`；每租户至多一个 `isSystem=true`（部分唯一索引或应用层 ensure）。展示名初值 `未分类`，admin **可改展示名**，**不可删** system 分类（`TAG_CATEGORY_SYSTEM_IMMUTABLE`）。`ensureDefaultCategory(tenant)`：幂等确保 system 行存在——回填、`ensureTagsByNames`、`GET /tags/categories` 入口均调用（新租户冷启动不依赖 admin 先建分类）。`ensureTagsByNames` 按 `isSystem` 找默认分类，**不按展示名**，避免改名后分叉出第二个「未分类」。
- **j. 多级建目录：POST 默认递归补齐祖先（第二轮 P1）。** `POST /folders` body `{spaceId, path, recursive?: boolean}`，**`recursive` 默认 `true`**：事务内对 `expandAncestors(path)` 中每个非根前缀 ensure 显式行（已存在跳过），再 ensure 目标 path；与 UI「一次输入 `资料/合同`」及 §7.3 走查一致。`recursive:false` 时父路径须已在树视图中（显式或隐式均可），否则 `FOLDER_PARENT_MISSING`。幂等：目标已存在 → `existed:true`。
- **k. 目录树一次拉全 + 软上限。** GET `/folders` 一次返回全树（前端非 lazy）。单空间节点数软上限 `FOLDER_TREE_MAX_NODES`（默认 5000）：超限仍返回已收集节点并带 `truncated:true`（实现期写入响应 envelope 或 meta 字段），文档声明；分页/虚拟滚动出本期。代价：超大空间首屏变重。缓解：软上限 + 前缀索引支撑；显式承认。

### 0.3 明确不在本期

- **`files.tags` 旧列的物理删除（contract 迁移，未来 0005）**——本期 expand 双写保鲜；下一期确认全部环境运行新代码后，先重跑 `scripts/backfill-tags.ts` 调和再 drop。
- **应用存储（app 空间）的目录树/标签 UI**——服务态写 folder 字符串的现状不动；树/词典端点对 app 空间拒绝。去向：应用自己管理语义，二期再议。
- **跨空间移动文件、目录递归删除、回收站/软删**——DELETE 现状即硬删（files.ts:302-310），不放大。去向：二期。
- **AI 自动打标签**（置信度/兴趣度的生产者）——tagging meta 是为它预留的契约，本体依赖 Agent/LLM 底座另立项。
- **tagging meta 的 UI 编辑/展示**——API-only（§0.2g）。
- **目录级权限**（folder 路径参与 space ACL 判定）——FILE-STORAGE-2 §0.5 同款立场，无投机抽象。
- **标签层级超过两级、标签合并/批量重命名等词典治理工具**——分类>标签两级为止。
- **公开分享（`/s/:token`）内的目录树浏览与标签**——公开面零改动。
- **pdf 首屏/音频封面缩略图**——FILE-PREVIEW 既有后续方向，不并入。
- **目录树拖拽移动**——本期移动走对话框选目录；拖拽为 UI 增强 backlog。
- **跨副本共享的分布式转换队列**——进程内队列 + DB 锁已够（§0.2d）；如需集中调度另立基础设施项。
- **目录树分页 / 懒加载**——本期一次拉全 + 软上限（§0.2k）。

---

## 1. 拓扑与工作区增量（零新进程）

```
apps/files-server/            ★全部为增量（dev :4100，DB xgent-files）
  src/db/schema.ts            + folders / tag_categories / tags / file_tags 四表；files.tags 列保留（§0 决策 5）
  src/db/migrations/0004_organize.sql   手写（四表 + folder 索引 + tags 回填）+ meta/_journal.json 手登记
  src/modules/folders.ts      新 · 目录树/建/改/删端点
  src/modules/tags.ts         新 · 分类/标签词典 + cloud + tagging 端点
  src/lib/organize.ts         新 · 路径严格校验 / 祖先闭包 / 子树前缀重写 tx / 名称两函数(只读 find + 写 ensure) / ensureDefaultCategory / 可见集收口
  src/lib/thumbnails.ts       新 · ensure/get + 进程内 FIFO 队列 + semaphore；先入队、worker 内占锁
  src/modules/files.ts        list 增 folder/tagIds/withThumbs；PATCH 增 folder；tags 读写切词典(双写旧列)；fileDTO 全部调用点
  src/modules/processors.ts   saveDerivedFile 走统一写路径(file_tags 空集 + tags: [] 照旧)
  src/modules/preview.ts      `?kind=thumbnail` 显式分支(先于 provider 解析)；convertToImage 抽宽度参数并导出/抽到可复用处（禁止 thumbnails 复制一份 ffmpeg）
  scripts/{verify-folders,verify-tags,verify-thumbnails,verify-organize,backfill-tags}.ts   新
apps/files-app/               ★全部为增量（dev :5303）
  src/components/FolderTree.tsx(新) / TagPickerDialog.tsx(新) / TagsView.tsx(新)
  src/App.tsx                 currentFolder 状态 + /tags 路由 + withThumbs 列表 + 上传带 folder
  src/components/FileView.tsx Thumb 扩展视频、优先 thumbUrl、标签徽标改自词典组装值
  src/lib/i18n.ts             三语 parity 增量
packages/portal-sdk/src/index.ts   FileListQuery + folder/tagIds/withThumbs；sdk.files.folders.*/tags.*；rename body + folder
packages/shared/src/dto.ts    FolderDTO / TagCategoryDTO / TagDTO / FileTagDTO / TagCloudItemDTO；FileDTO + thumbUrl?
apps/api/                     (小) apps/api/src/db/provisioning.ts files.navItems + seed.ts 同步 + FILES_MANIFEST pages += tags
                              + scripts/verify-all.ts SUITE 增 files·organize 行(评审 P1-8)
docs/Files文件服务接入指引.md  §8.1 list 参数表 + §8 新小节 + §14 错误码 + 附录 A（实现后回填）
```

`dev:all`、部署三套件（compose/pm2/helm）、Caddy/vite 代理、CSP、files listing 的 scopes **全部零改动**。root 脚本增 `files:verify:organize`（三套件串联）；`verify:all` 的真实挂点是 `apps/api/scripts/verify-all.ts:26` 的硬编码 SUITE 数组（评审已核实），增一行 `{ label:"files · organize (folders/tags/thumbnails 契约)", dir:"apps/files-server", script:"scripts/verify-organize.ts" }`——`verify-organize.ts` 为三套件串联入口（folders+tags+thumbnails 契约级部分）。

新 env（files-server）：`THUMB_CONVERT_CONCURRENCY`（默认 2，转换并发上限）、`THUMB_QUEUE_MAX`（默认 500，进程内队列长度上限=背压）、`FOLDER_TREE_MAX_NODES`（默认 5000，树节点软上限）。缩略图源大小上限直接沿用 `PREVIEW_MAX_RENDITION_BYTES`，不新增。

## 2. 数据模型增量（DB `xgent-files`，本期只有一个手写迁移 `0004_organize.sql` = expand）

| 表 | 关键列 | 说明 |
| --- | --- | --- |
| `folders` ★ | id uuid pk · tenantId · spaceId FK→spaces cascade · **path text**（规范化全路径） · name text · createdBy text null · createdAt | 显式目录登记；`UNIQUE(space_id, path)`；`INDEX(tenant_id, space_id)`。隐式目录（有文件无行）不落此表，读侧祖先闭包补齐（§4）。 |
| `tag_categories` ★ | id · tenantId · name · **sort int notNull default 0** · **isSystem boolean notNull default false** · createdAt/updatedAt | 字典表带 sort（CLAUDE.md 规范）；`UNIQUE(tenant_id, name)`；**每租户至多一个 isSystem=true**（部分唯一索引 `UNIQUE(tenant_id) WHERE is_system` 或 ensure 应用层双保险）。system 行不可删；展示名可改。 |
| `tags` ★ | id · tenantId · categoryId FK cascade · name · **scope text default 'tenant'**（`tenant`/`personal`） · **ownerUserId text notNull default ''**（''=共享；仿 qbank `parentId default ''` 规避 NULL 唯一键） · sort default 0 · createdBy text null · createdAt/updatedAt | `UNIQUE(tenant_id, category_id, owner_user_id, name)`（**跨分类允许同名**，§0.2h）；`INDEX(tenant_id)`。查询共享标签用 `owner_user_id = ''`，**不要** `IS NULL`。 |
| `file_tags` ★ | id · tenantId · fileId FK cascade · tagId FK cascade · **meta jsonb notNull default '{}'**（**固定 schema**，§2 下表；≤2KB） · createdBy text null · createdAt | tagging 关系；`UNIQUE(file_id, tag_id)`；`INDEX(tenant_id, tag_id)`（cloud 计数）、`INDEX(tenant_id, file_id)`（按页组装）。 |
| `files` 变更 | **0004 + `INDEX(tenant_id, space_id, folder)`**；`tags text[]` 与 `files_tags_gin` **本期保留不 drop**（expand/contract，§0 决策 5） | folder 过滤/子树重写/隐式目录 distinct 全部钉在该索引。 |

- **回填（0004 内 + 独立脚本 `scripts/backfill-tags.ts`，同一 SQL，幂等）**：每租户 `ensureDefaultCategory`（`isSystem=true`，name 初值 `未分类`，sort=0）→ 每 `(tenant_id, unnest(files.tags))` distinct 名称在默认 system 分类下建 `scope='tenant'` 标签 → 按 `(fileId, 名称)` 插 `file_tags`（`ON CONFLICT DO NOTHING`，`meta='{}'`）。**不动 `files.tags` 列本身**。部署窗口内旧进程写入由下一期 contract 前重跑该脚本调和（§0.2b）。
- **双写约定（本期）**：所有 tagging 写路径（finalize/PATCH/PUT/processors 派生）经 `replaceFileTags` 唯一函数——以 `file_tags` 为主；同步把 **`scope=tenant` 的名称数组**写回 `files.tags` 旧列保鲜；`searchText = [name, ...tenantTagNames].join(" ")`（**不含个人标签名**，§0.2e）。读路径（DTO 组装）一律从 `file_tags` 取**当前 actor 可见**名称。
- **`file_tags.meta` 固定 schema（`schemaVersion=1`，键名英文稳定，文档给中文语义）**：

| key | 类型 | 必填 | 语义 | 校验 |
| --- | --- | --- | --- | --- |
| `schemaVersion` | int | 否（缺省=1） | meta 结构代际；升级只增 key 时递增 | ≥1 的整数 |
| `confidence` | number | 否 | **置信度**：模型/规则认为「该标签适用」的把握 | 有限 number，∈ [0, 1] |
| `interest` | number | 否 | **兴趣度**：对当前用户/场景的相关或推荐权重 | 有限 number，∈ [0, 1] |
| `match` | number | 否 | **匹配度**：内容与标签定义/查询意图的贴合程度 | 有限 number，∈ [0, 1] |
| `source` | string enum | 否 | **来源**：谁写下这条 tagging | 仅 `user` \| `ai` \| `import` \| `system` |
| `model` | string | 否 | **模型标识**（如 `gpt-x@2026-07` / 内部 modelId） | 非空 trim 后 ≤64 |
| `reason` | string | 否 | **简短依据**（为何打该标签，供审计/调试） | trim 后 ≤200，禁止控制字符 |
| `scoredAt` | string | 否 | **评分时间** ISO-8601 | 可被 `Date.parse` 解析 |

  写入规则：`parseFileTagMeta`——① 必须是 plain object（非 array）；② **仅允许上表 key**，多出的 key → `TAG_META_INVALID`；③ 逐 key 类型/范围校验失败 → 同错；④ `JSON.stringify` 后 ≤2KB；⑤ 全空 object `{}` 合法（人工打标默认）。读路径原样返回已存 meta（缺 key 不补默认分数）。  
  **推荐保留、首版不入库的后续候选**（升级时按需 `schemaVersion=2+` 增 key，本期校验不接受）：`agentRunId`（一次自动打标任务 id）、`promptHash`（提示词指纹）、`locale`（打标所用语言）、`rank`（同文件多标签排序）。  
  **三个分数的分工（给 AI 立项对齐）**：`confidence` =「标得对不对」；`match` =「和内容/查询贴不贴」；`interest` =「对当前主体值不值得推」——可只填子集。
- 约定：跨库软引用沿用（`createdBy` 存 userId 文本无 FK）；`tag_categories`/`tags` 是字典表带 `sort`（`orderBy(asc(sort), asc(name))`）；列表接口沿用既有 `{items,total}`+limit/offset 形态（files list 非 `Page<T>` 是现状，不借本期改）。`FileTagMeta` 类型进 `packages/shared`（与 DTO 同处），files-server 与未来 AI job 共用。
- 迁移手写 + 手登记 `meta/_journal.json`（drizzle-kit generate 对 files-server 不可用，memory: plan-4-implementation）。

## 3. 集成与契约（消费方影响，全部已核实）

| 消费方 | 调用面（路径） | 本期影响 |
| --- | --- | --- |
| library-server | list(q,type,spaceId)/get/download/preview/presign(**带 folder**)/finalize（`src/lib/files.ts:114-189,245-262`） | **零改动**。宽松 DTO（`[k:string]:unknown`）天然容忍新字段；folder 字符串继续透传。 |
| git-server | list(q,spaceId)/get/download/presign/finalize（`src/lib/files.ts:66-138`） | **零改动**（同上）。 |
| spms/todo-server | presign/finalize(**带 tags**)/preview/download/delete（spms `src/lib/files.ts:64-108`） | **零改动**。finalize 字符串 tags 内部 `ensureTagsByNames`（§0.2h）。 |
| task-app | `sdk.files.list({via,q,limit})` + `upload({tags})`（`MaterialPicker.tsx:75,85`） | **零改动**（SDK 签名仅加可选字段）。 |
| apps/web | MicroAppHost 透明代理 + storage/hooks 管理 | **零改动**。 |
| knowledge/omni-parser/exam/agent/qbank/lms/sms | 非 files 消费方 | **零改动**。 |
| 门户 apps/api | files listing navItems（`apps/api/src/db/provisioning.ts`）+ seed（`seed.ts` listing 块）+ `FILES_MANIFEST.pages`（`manifests.ts:110-115`）+ `scripts/verify-all.ts` SUITE（:26） | **精确增量**：navItem/manifest page 各一行（`nav-tags`/`tags` 页，nav-only，真闸在 files-server scope，与 app-storage 页同立场）+ SUITE 一行（§1）。零新 scope。 |

- 硬前提（契约红线）：list 新参数全部**可选**；默认行为不变（`status='active'`、无参全量、`{items,total}` 形状、sort 默认 `-createdAt`）；`FileDTO.tags` 恒为名称数组；`?tags=name` 保持名称语义（跨分类同名并集，§0.2h）；字符串 tags 写路径最多 24 个名称。
- usage 计量/通知/审计：无新横切面接入（目录/标签操作不产生新计量指标；审计沿用既有请求日志）。
- list `withThumbs=1` 是可选副作用：只入队、有界、失败/未配置不拖垮 list（无 thumbUrl、不 500）；接入指引写清。

## 4. 服务端机制（files-server，前缀 `/api/v1/files`）

- **gate/闸**：全部端点走既有 `gate(headers, scope)`（`src/lib/gate.ts:98`）；读=`files.read`，写=`files.write`；**空间内**操作（folders 写、文件 tagging、PATCH folder）走 `requireActor` + 空间角色（读=空间任意角色，写=editor，viewer 拒）；**词典是租户级对象**（无空间角色概念）：分类 CRUD = `requireAdmin`；**共享标签（scope=tenant）创建/改/删 = `requireAdmin`**；个人标签 = owner 本人——矩阵与 §6 第 1 条一致。业务错误一律 200 + `{ok:false,error}`，错误码沿用 ALL_CAPS 风格。
- **`lib/organize.ts`**（唯一事实函数群，路由与 files.ts/finalize/processors 共用，不做第二条路径）：
  - `normalizeFolderPath(raw)`：**严格校验**——拒空段（`a//b`）、`.`/`..` 段、`\`、控制字符、单段 >100、全长 >255；通过者拼回首尾各一 `/`；非法 → `FOLDER_INVALID_PATH`。不做宽松归一（评审 P1-5）。推论：存量非规整路径（presign 历史上不做规范化）**可读不可改**——rename/move 的 from/to 均须过此校验。
  - `expandAncestors(paths[])`：每个路径展开为全部祖先前缀集合（`/资料/合同/` → `/`、`/资料/`、`/资料/合同/`）——目录树连通性的唯一实现（评审 P1-4）。
  - `ensureDefaultCategory(tenant)`：幂等确保 `isSystem=true` 分类（§0.2i）。
  - `visibleTagsWhere(actor)`：`scope='tenant' OR (scope='personal' AND owner_user_id=<me>)`——DTO 组装/cloud/tagIds 过滤三处唯一收口（§0.2e）。
  - `findTagIdsByNames(tenant, actor, names[])`：**只读**——跨全部可见分类按名匹配，返回同名标签 id 并集；未知名 → 空集；**绝不落库**（评审 P1-6）。list `?tags=` 专用。
  - `ensureTagsByNames(tenant, names[])`：**仅写路径**——trim / 去空 / 单名 ≤64 / 去重 / **最多 24 个** → `ensureDefaultCategory` → 逐名称在 system 分类 get-or-create（scope='tenant'，唯一键冲突即取既有）→ tagIds。finalize/PATCH 名称 tags 专用。
  - `parseFileTagMeta(raw)`：固定 schema 校验（§2 表）；非法 → `TAG_META_INVALID`。
  - `replaceFileTags(tx, tenant, fileId, tagIds, metas, createdBy)`：delete-all + insert `file_tags`（每条 meta 经 `parseFileTagMeta`）+ **双写 `files.tags` = 其中 scope=tenant 的名称数组** + **`searchText` 仅用 tenant 名称**（§0.2e）——tagging 任何变更走这里（§0.2b）。
  - `renameFolderSubtree(tx, tenant, spaceId, from, to)`：`folders` 行（缺失的 from 行先物化）+ `files.folder` 前缀重写，同 tx；`from`/`to` 均不得为根（`FOLDER_ROOT_IMMUTABLE`）。前缀匹配依赖路径尾 `/`，避免 `/a/` 误伤 `/ab/`。
  - `folderIsEmpty(tenant, spaceId, path)`：直接文件数 + 严格前缀子路径数（§0.2f）；供 DELETE 使用。
- **`lib/thumbnails.ts`**（评审 P0-2/P0-3 + 第二轮锁序；与 preview provider 机制正交）：
  - `thumbEligible(f)`：`image/*`（jpeg/png/webp/gif/bmp + heic/heif/tiff）或 `video/*` 且 `size ≤ PREVIEW_MAX_RENDITION_BYTES`。
  - `getThumbnailUrl(tenant, f)`：`file_renditions (fileId,'thumbnail')` fresh ready（status=ready 且 `sourceDigest === f.digest`）→ presigned inline URL；否则 null。
  - `ensureThumbnails(tenant, files[])`：**只负责入队意图**（§0.2d 五步）——eligible 且非 fresh-ready → 进程内去重 → 队列未满则入队（**不**调用 `acquireRenditionLock`）→ 队列满则跳过且**不写 pending**。list/preview 路由只调用 ensure，**不直接扇出转换**。
  - 队列 worker：semaphore `THUMB_CONVERT_CONCURRENCY`（默认 2）逐 job——**出队后** `acquireRenditionLock`（preview.ts:202-227 同姿势，含 `PENDING_STALE_TTL_MS` 僵尸回收与 `PREVIEW_MAX_ATTEMPTS` 上限）→ 赢锁者转换；转换复用从 preview **抽出的** `convertToImage(..., { width: 320 })`（ffmpeg `scale='min(320,iw)':-1`；sidecar `POST /poster` 加可选 `width`，不识别时按其默认尺寸返回，前端不感知）；产物写 `_previews/<fileId>/thumbnail.jpg` + 翻 `file_renditions` 行（同 `runConversion` 姿势，preview.ts:230-245）。converter 未配置（`PREVIEW_MEDIA_CONVERTER_URL` 未设）→ 不入队不报错。
- **路由清单**：

| 方法 + 路径 | 闸 | 说明 |
| --- | --- | --- |
| GET `/folders?spaceId=` | read + 空间角色 | 树数据 = 显式行 ∪ `expandAncestors(显式行 ∪ DISTINCT files.folder)`——**祖先闭包保证连通**（评审 P1-4）；非显式行标 `explicit:false`；逐项附**直接文件数**（`group by folder` 计数桶，照 qbank `routes/meta.ts:20-33` 范式）；恒含根 `/`。节点数 > `FOLDER_TREE_MAX_NODES` → `truncated:true`（§0.2k）。app 空间 → `SPACE_KIND_FORBIDDEN`。 |
| POST `/folders` | write + editor | `{spaceId, path, recursive?: boolean}`（严格校验；**recursive 默认 true**，§0.2j）；已存在 → 幂等返回（`existed:true`）；`/` → `existed:true`；`recursive:false` 且父不在树 → `FOLDER_PARENT_MISSING`；app 空间拒。 |
| PATCH `/folders` | write + editor | `{spaceId, path, name}` → 重命名（`renameFolderSubtree` tx；目标已存在 → `FOLDER_EXISTS`；`path='/'` → `FOLDER_ROOT_IMMUTABLE`）。 |
| DELETE `/folders` | write + editor | `{spaceId, path}`；`/` → `FOLDER_ROOT_IMMUTABLE`；`folderIsEmpty` 否则 `FOLDER_NOT_EMPTY`（带直接文件数/子路径数，§0.2f）。 |
| GET `/tags/categories` | read | 入口先 `ensureDefaultCategory`；分类（sort 序）嵌套可见标签（`visibleTagsWhere`）；DTO 含 `isSystem`。 |
| POST `/tags/categories` | write + **admin** | `{name, sort?}`；**不可**客户端设 isSystem；23505 → `TAG_NAME_CONFLICT`。 |
| PATCH/DELETE `/tags/categories/:id` | write + **admin** | 改名/sort 允许（含 system 的展示名）；**删除 system → `TAG_CATEGORY_SYSTEM_IMMUTABLE`**；非 system 删除须空（无标签），否则 `TAG_CATEGORY_NOT_EMPTY`。 |
| POST `/tags` | write +（**tenant → admin**；**personal → user actor=owner**） | `{categoryId, name, scope, sort?}`；`scope=tenant`（缺省）→ `requireAdmin`；`scope=personal` → owner=me（§0.2c）。成员建共享 → `FORBIDDEN`。 |
| PATCH/DELETE `/tags/:id` | write +（tenant 标签=**admin**；personal=**owner 本人**） | 改删；删除 cascade `file_tags` 并对受影响文件走 `replaceFileTags` 等价重建（searchText + 双写旧列，仅 tenant 名）。 |
| GET `/tags/cloud?spaceId?` | read | 可见标签 × 可见文件（缺省 = 全部 accessibleSpaceIds；传 `spaceId` 则单空间收口；status=active）的 `count(*)` 计数桶；返回 `[{tagId,name,categoryId,categoryName,scope,count}]`。 |
| GET `/:id/tags` | read + 空间角色 | 该文件可见标签（含 meta，schema 原样）。 |
| PUT `/:id/tags` | write + editor | `{items:[{tagId, meta?}]}` 全量替换（`replaceFileTags`；tagId 须在可见集；`meta` 经 `parseFileTagMeta`，缺省 `{}`）。人工 UI 打标传 `{}` 或 `{source:"user"}` 即可。 |
| GET `` （list）增参 | read（现状） | `folder=<path>`（精确，须同传 spaceId）；`tagIds=a,b`（any-of，EXISTS `file_tags`，id 先过可见集）；`withThumbs=1` → 本页 eligible 项 `ensureThumbnails`（只入队）+ fresh-ready 项附 `thumbUrl`；`?tags=` 名称过滤**保持**（只读 `findTagIdsByNames` 并集，§0.2h）。**`via` + folder/tagIds 组合时 user actor 的 `ownerUserId=me` 守护不可绕**（§10）。 |
| PATCH `/:id` 增字段 | write + editor（现状） | body 增 `folder`（严格校验后更新；同空间内移动）；`tags` 字符串数组语义不变（最多 24 个，走 `ensureTagsByNames` + 双写）。 |
| GET `/:id/preview?kind=thumbnail` | read（现状闸） | **显式分支（先于 provider 解析）**：eligible → `ensureThumbnails`（入队）+ 按 rendition 状态回 descriptor（ready→`source.url` 指向缩略图 / processing / failed 兜底 download card）；ineligible 或未带 `kind` → 既有 `buildDescriptor` 逻辑零改动（评审 P0-2）。 |

- **DTO 组装调用点（M2 必须全覆盖，评审 P0-1）**：`fileDTO`（files.ts:20-36）当前直读 `f.tags`——改为注入组装好的**可见**名称数组。调用点：files.ts 的 list（整页 `inArray` 组装）、GET `/:id`、PATCH 响应、finalize 响应；processors.ts 的派生返回。public/shares 不暴露 tags（现状子集，不动）。

## 5. 前端（files-app，强制两步法：impeccable 设计 + Chrome 真浏览器验证）

- **SDK 接入**：沿用现有 main.tsx 分流；`/tags` 进 `spaceForRoute` 等价路由表 + `init.route` 深链 + `sdk.onRoute`（handler 幂等、不回 routeSync 回声，memory: host-app-route-sync）。
- **视图与信息架构**：
  - **空间视图（personal/team）**：左 `FolderTree`（GET 一次拉全；`TreeItem` 字段形状照抄 knowledge `Tree.tsx:10-18`——label=目录名、count=直接文件数、`explicit:false` 节点灰显；**不抄懒加载**）+ 新建/重命名/删除目录的 **in-DOM modal**（iframe 内 `prompt/confirm` 失效，memory: microapp-ui-gotchas）+ 工具栏「移动到…」对话框（复用树做单选）；选中目录 → list `folder` 过滤 + 面包屑（`/` 段可点）；上传（按钮/拖拽/空态**三入口**）带当前 folder 进 presign。新建目录：支持多级路径输入，一次 POST（服务端 `recursive` 默认 true）。非法历史路径节点：禁止改名/移动，提示只读。
  - **`/tags` 页（重头戏）**：上部 Tag cloud——按分类分组，标签字号按 count 分档（如 5 档线性映射）、个人标签带「个人」徽标；点击多选高亮 → 下部文件区复用 `FileView`（`tagIds` any-of 过滤、跨个人+团队可见空间）；右侧/抽屉管理区：**分类 CRUD 与共享标签增改删仅 admin**；普通成员仅可管理个人标签、选用已有共享标签；新建入口按角色分支（admin：共享+个人；成员：仅个人）；system 分类展示「系统」且无删除入口。
  - **Grid 缩略图**：`Thumb` 扩展——`image/*` 与 `video/*` 优先 `item.thumbUrl`（list 恒带 `withThumbs=1`），缺失时**不主动**打 preview N+1（避免抵消有界队列）；直接回退 `iconForType`。可选：用户打开预览时再走 preview。`<img loading="lazy">` 现状保留；list 视图维持图标（不变）。
  - **文件标签编辑**：grid/list 行操作 + 多选批量入口 → `TagPickerDialog`（分组多选范式照抄 qbank `QuestionEditor.tsx:542-584` TagPicker；面板内「新建」：admin 可选共享/个人，成员仅个人）。保存：单文件 = PUT 全量替换为面板选中集（meta 默认 `{}` 或 `{source:"user"}`，**不在 UI 编辑分数**）；**批量打标 = 对每个文件 `existing ∪ selected` 再 PUT**（给多文件追加同一批标签的心智；「设为仅这些标签」不做，避免误清空）。
  - impeccable 把关点：树与文件区的主次层级、空目录/无标签/无结果三态空态、目录操作危险确认（删除）复用 ConfirmDestructive 姿势、cloud 视觉噪声控制（count=0 标签降透明度不隐藏——便于点选打标后的回流）。
- **三语 i18n**：`lib/i18n.ts` 三份平铺词典同步增 key（tree.* / tags.* / cloud.* / thumb 相关），zh-CN/en/zh-TW parity（租户数据——目录名/标签名——不做 i18n）。

## 6. 安全清单（验收断言的来源）

1. **权限矩阵（与 §4 路由表逐行一致）**：空间内写操作（folders 建/改/删、PATCH folder、PUT 文件 tags）= 空间 editor，viewer 一律 `FORBIDDEN`；词典（租户级，无空间角色）：分类 CRUD=admin（system 分类不可删），**共享标签 CRUD=admin**，个人标签 CRUD=owner 本人；成员可选用共享标签打到文件上（空间 editor），不可建共享词典项。
2. 租户隔离：伪造他租户 spaceId/fileId/tagId → `NOT_FOUND`（不泄露存在性）；cloud 计数、tagIds 过滤、tags DTO 组装均按 claims.tenantId 收口。
3. 个人标签收口（`visibleTagsWhere` + 双写/searchText 策略）：他人（含同空间成员、admin）看不到、计不到、过滤不出；**`?q=` 全文也不因个人标签名命中**（个人名不进 searchText）；旧列双写不含个人名。
4. 服务态 app actor 对 folders 端点/词典管理端点一律拒（`requireUser` 挡词典、`SPACE_KIND_FORBIDDEN` 挡 app 空间树）；服务态 finalize 字符串 tags 归一走内部 `ensureTagsByNames`（可 get-or-create tenant 标签，§0.2c 有意例外），不经过 HTTP 词典闸。
5. 路径注入：`normalizeFolderPath` 严格拒 `..`/空段/`\`/控制字符；根 `/` 不可变；folder 不进 objectKey（现状保持），对象存储 key 不受目录影响。
6. **读路径零副作用（标签）**：`?tags=name` 走只读 `findTagIdsByNames`，任何 GET 不为未知标签落库（评审 P1-6）。（`GET /tags/categories` 的 `ensureDefaultCategory` 是幂等种子，不属「按查询参数建标签」。）
7. **转换资源收口（评审 P0-3 + 锁序）**：转换并发 ≤ `THUMB_CONVERT_CONCURRENCY`；源 > `PREVIEW_MAX_RENDITION_BYTES` 不转换；队列 ≤ `THUMB_QUEUE_MAX`；**队列满不写 pending**；attempts ≤ `PREVIEW_MAX_ATTEMPTS` 后锁定 failed；锁仅在 worker 出队后获取。
8. `thumbUrl` 为短 TTL presigned GET（与 preview 同 TTL），不落日志；thumbnail 对象在 `_previews/` 保留前缀下，永不进文件列表（现状机制）。
9. **meta 固定 schema**：仅 §2 白名单 key；分数 ∈[0,1]；未知 key / 类型错误 → `TAG_META_INVALID`；≤2KB；UI 不渲染 meta（无注入面）；`?tags=`/`tagIds`/`folder` 参数全部参数化查询（drizzle，无拼接 SQL）。

## 7. 验证

### 7.1 verify 脚本（`apps/files-server/scripts/`，复用 `_helpers.ts` 范式；per-run 唯一 id 防孤儿）

- `verify-folders.ts`（**核心**）：mkdir/幂等再建/**多级 recursive 默认 true**（一次 POST `/资料/合同/` → 祖先与目标均显式）/`recursive:false` 父缺失拒/**严格路径矩阵**（`a//b`、`.`、`..`、`\`、控制字符、超长一律拒）/**根不可变**（PATCH/DELETE `/` → `FOLDER_ROOT_IMMUTABLE`）/树 union（显式 + presign 写出的隐式 + 根）+ **祖先闭包**（只写一个深层隐式 `/a/b/c/` → 树含 `/a/`、`/a/b/` 完整链且标 `explicit:false`）/重命名子树（含隐式物化 + 文件 folder 前缀重写 + 目标冲突 `FOLDER_EXISTS` + 前缀不误伤 `/ab/`）/删除空目录闸（**仅有深层文件时父目录非空** + `FOLDER_NOT_EMPTY` 计数）/PATCH 文件移目录/list `folder` 过滤精确性/viewer 写拒/app 空间 `SPACE_KIND_FORBIDDEN`/他租户 spaceId → NOT_FOUND。
- `verify-tags.ts`（**核心**）：**空租户 `GET /tags/categories` 自动有 system 默认分类**；改展示名后 `ensureTagsByNames` **不**新建第二个默认分类；删 system → `TAG_CATEGORY_SYSTEM_IMMUTABLE`；分类 CRUD + admin 闸/权限矩阵逐行（§6 第 1 条：**成员不可建/改/删共享标签**、admin 可建共享、个人标签 owner 限定）/共享+个人可见性矩阵/finalize 字符串 tags get-or-create（task-app 形态；**最多 24 个**；**非 admin 经字符串路径仍可间接触发 tenant 标签**——§0.2c 例外回归）/PUT 替换 + **meta schema 矩阵**（合法 `{confidence,interest,match,source,model,reason,scoredAt,schemaVersion}` 落库；未知 key / 分数越界 / 非 object → `TAG_META_INVALID`；≤2KB；`{}` 合法）/删标签 cascade + searchText 重建 + **双写旧列一致**（`files.tags` === 该文件 `scope=tenant` 名称集；**个人标签名不在旧列**）/**个人标签名不进 searchText**（他人 `?q=个人标签名` 不命中）/cloud 计数（个人标签不混入他人）/`tagIds` any-of 过滤/**`?tags=` 回归三断言**：名称过滤同 verify-search、**未知名称查询不落库**（GET 后 tags 表行数不变）、**跨分类同名命中并集**/DTO.tags 名称数组形状/**processors 派生回归**（派生文件 `file_tags` 空集 + DTO.tags=[]）/他租户全拒。
- `verify-thumbnails.ts`（**核心**）：图片上传 → list `withThumbs` →（converter=auto 时）轮询至 thumbUrl ready + rendition 行断言 + 320px 上限断言；同 digest 二次请求不重复转换（锁）；重传新 digest → 失效重生；**并发有界**（观测同时在跑转换 ≤ `THUMB_CONVERT_CONCURRENCY`）；**超限源不入队**；**队列满背压：满时不新增 pending 行**（`THUMB_QUEUE_MAX=1` 起多文件，断言未入队者无新 pending）；**`?kind=thumbnail` 对 native 图片显式分支可用**（评审 P0-2 回归）；converter 未配置 → 无 thumbUrl 且不 500 不入队。视频用例挂 `files:verify:preview`（真 ffmpeg 依赖——要求 **files-server 进程本身** `PREVIEW_MEDIA_CONVERTER_URL=auto` 启动，memory: file-preview-2-implementation）；契约级用例进 `verify:all`。
- **`verify-organize.ts` 额外契约**：`via=app/<appKey>`（或 `via=app`）+ `folder`/`tagIds` 组合时 user actor 仍强制 `ownerUserId=me`（§10 坑落断言，不可只靠记忆）。
- **回归门（每里程碑必跑）**：`verify-upload/spaces/share/search/fingerprint/exchange-files/app-storage` + preview 六套件全绿——尤其 `verify-search` 的 `?tags=` 断言**原样通过**（旧列保留 + 只读名称解析双重保障）。

### 7.2 门户/跨 App 脚本

- 复用既有 `files:verify:app-storage`、`files:verify:preview` 不动；新增 root `files:verify:organize`；**真实挂点**（评审 P1-8）：`apps/api/scripts/verify-all.ts:26` SUITE 数组增一行 files·organize 套件（`scripts/verify-organize.ts` 串联 folders+tags+thumbnails 契约级部分）。

### 7.3 Chrome 真浏览器走查（强制，前端两步法第二步）

登录 → 进文件管理 → 个人空间：新建目录一次输入 `资料/合同`（多级，服务端 recursive）→ 上传文件进当前目录（三入口均带 folder）→ 树浏览展开/选中/计数 → 移动文件到另一目录 → 重命名目录（文件跟随）→ 删空目录/删非空目录见计数提示 → **admin** 预置共享标签 → 给文件打标签（选用共享 + 新建个人）→ 成员账号确认**无**「新建共享」入口且 POST 被拒 → 批量多选追加标签 → `/tags` 页 cloud 按分类呈现、点击多选筛选出正确文件、个人标签带徽标、system 分类不可删 → 换一用户（同租户）看不到该个人标签 → Grid 切图片+视频目录见缩略图渐现（无 thumbUrl 时图标占位，不狂打 preview）→ 团队空间全量复走 → 切 en/zh-TW 界面 parity → 暗色模式 → 另一租户登录互不可见 → viewer 角色入口隐藏/直打 URL 被拒。截图留档；环境起不来则显式标注「未在浏览器验证」。

### 7.4 静态校验

- `bun run typecheck` 全 workspace 绿；i18n 三语 parity（含 `apps/web` 侧 nav 标签）。

## 8. 里程碑（每步可验证，上一步不绿不进下一步）

| # | 里程碑 | 内容 | 验证 |
| --- | --- | --- | --- |
| M1 | 迁移 + 目录后端 | `0004_organize.sql` 四表（含 `isSystem`）+ folder 索引 + **tags 回填**（expand，不碰旧列）；`lib/organize.ts` 路径严格校验/祖先闭包/递归 mkdir/空目录前缀判定/子树函数；folders 四端点（含根保护、recursive 默认 true）；list `folder`；PATCH `folder` | `verify-folders.ts` 绿（含多级 recursive/严格路径/根不可变/祖先链/隐式子树非空）；既有 files 全套件回归绿 |
| M2 | 标签后端（最大回归面） | 词典端点组 + cloud + tagging；`ensureDefaultCategory`；共享标签 **admin 闸**；`findTagIdsByNames`/`ensureTagsByNames` 拆分（24 个名称契约）；`parseFileTagMeta` 固定 schema；finalize/PATCH/processors 统一走 `replaceFileTags`（**双写仅 tenant 名 + searchText 不含个人**）；**fileDTO 全部调用点切组装**（§4 清单）；list `tagIds` + `?tags=` 只读兼容 | `verify-tags.ts` 绿（含 admin 共享闸/meta schema 矩阵/system 分类/双写一致/个人不进 searchText/不落库/跨分类同名/processors 派生）；**verify-search 原样全绿** + 既有全套件回归绿 |
| M3 | 缩略图 | `lib/thumbnails.ts`（ensure 只入队 + worker 内占锁 + FIFO + semaphore）；preview `?kind=thumbnail` 显式分支；`convertToImage` 宽度参数化并抽出复用；list `withThumbs`；DTO/SDK `thumbUrl`；`THUMB_*` env | `verify-thumbnails.ts` 绿（契约级，含有界并发/超限/**队列满不写 pending**/native 分支）+ preview 六套件回归绿；converter=auto 环境真转换走查 |
| M4 | 前端 + 门户 nav + 文档 | SDK 增量、FolderTree/TagsView/TagPickerDialog、Thumb 扩展（优先 thumbUrl、缺省不 N+1 preview）、nav/manifest 各一行、**`verify-all.ts` SUITE 一行**、i18n×3、§1 文档回填、root `files:verify:organize`（含 via+folder 守护） | §7.3 浏览器全链路 + `bun run verify:all` 全绿（含新 SUITE 行）+ `typecheck` 绿 |

**验收环境注**：契约级 verify 全程无外部依赖（CI 可跑）；真 ffmpeg 转换验证放 `files:verify:preview` 门槛（需 `PREVIEW_MEDIA_CONVERTER_URL=auto` 的 files-server）；浏览器走查为 M4 强制项。

## 9. 风险与开放问题

### 9.1 已知风险（设计已收口，实现期按门验收）

- **最大回归风险 = 标签读写切换 + 双写（M2，评审 P0-1）**：以「verify-search 原样绿 + processors 派生回归 + 双写一致性断言（仅 tenant 名）+ 个人标签 searchText 断言 + 既有全套件绿」为门；回填幂等可重跑；旧列本期不 drop，任何切换缺陷都可回退到旧列读取（代码开关而非数据迁移）。
- **转换资源耗尽（评审 P0-3）**：队列 + semaphore + 大小上限 + attempts 上限 + **满队列不占锁**；进程内队列不跨副本是有意取舍（§0.2d），锁保证同文件不重复转换。
- **目录重命名写放大**（§0.2a）：tx 内两表前缀重写；单空间文件量万级内可接受，显式承认；不引入异步任务。
- **ffmpeg 环境依赖**：converter 未配置时缩略图静默缺席（图标占位、不入队）是声明过的降级，非缺陷；文档写清开启方式。

### 9.2 已拍板（写入设计，实现按此执行）

#### ✅ ① 共享标签（`scope=tenant`）谁可以创建？→ **租户管理员**

| | |
| --- | --- |
| **拍板** | **HTTP 词典面：共享标签创建/改/删 = 租户 admin**（`requireAdmin`）。个人标签仍 owner 自建。成员只能**选用**已有共享标签打到文件上（空间 editor），或建个人标签。 |
| **与字符串路径的关系** | **`ensureTagsByNames`（finalize/PATCH 字符串 tags）不收紧**：继续在 system 分类 get-or-create `scope=tenant`，保证 spms/todo/task 等消费方旧契约。这是**有意双轨**：人机 UI 治理严、机器字符串路径兼容宽。 |
| **实现落点** | §0.2c · §4 POST `/tags` · §5 TagsView/TagPicker · §6 第 1 条 · `verify-tags` 成员建共享拒 / admin 可建 / 字符串路径例外。 |

#### ✅ ③ tagging `meta` → **固定 schema，可加性升级**

| | |
| --- | --- |
| **拍板** | 固定白名单 schema（§2 表）；未知 key 拒绝；系统升级**只增可选 key**，`schemaVersion` 递增。本期 UI 不编辑。 |
| **初版 key（产品指定 + 补充）** | 见下表；完整校验在 §2。 |
| **实现落点** | `parseFileTagMeta` · `packages/shared` 类型 · PUT meta · `verify-tags` schema 矩阵。 |

**初版 meta key 一览（实现白名单）：**

| key | 中文 | 为何要 |
| --- | --- | --- |
| `confidence` | 置信度 | 「标得对不对」——模型/规则把握（产品指定） |
| `interest` | 兴趣度 | 「值不值得推给当前主体」（产品指定） |
| `match` | 匹配度 | 「和内容/查询贴不贴」（产品指定） |
| `source` | 来源 | 区分 `user` / `ai` / `import` / `system`，过滤与审计必需 |
| `model` | 模型标识 | AI 可复现、可对比版本 |
| `reason` | 简短依据 | 调试与人工抽检（≤200 字） |
| `scoredAt` | 评分时间 | 过期重打、时序分析 |
| `schemaVersion` | 结构版本 | 可加性演进的显式代际 |

**已记录、首版不入库的后续候选**（升级再开）：`agentRunId`、`promptHash`、`locale`、`rank`。

---

### 9.3 仍开放（未拍板则按默认实现）

#### ② 默认 system 分类的展示名与是否允许改名？

| | |
| --- | --- |
| **问题本质** | 回填与 `ensureTagsByNames` 需要一个「按名建标签」的落点分类。身份已用 `isSystem=true` 钉死（改名不会分叉第二个默认分类）；仍需定**展示文案**与**admin 能否改这个名字**。 |
| **选项 A（本期默认）** | 初值展示名固定中文 **`未分类`**（租户行数据，**不是** i18n key）；admin **可改**展示名（如改成「通用」「General」）；**不可删** system 行。界面三语只翻译壳层文案（「系统分类」徽标等），不翻译该租户自定义名。 |
| **选项 B** | 初值仍为 `未分类`，但 system 分类展示名 **锁定不可改**（PATCH name → `TAG_CATEGORY_SYSTEM_IMMUTABLE` 或仅允许改 sort）。 |
| **选项 C** | 初值按租户语言偏好多种子（zh→未分类 / en→Uncategorized）——**不推荐**：租户可切语言，分类名是数据不是 UI 词条，多种子易产生「两个看起来像默认」的困惑；且 `UNIQUE(tenant_id, name)` 与历史回填更复杂。 |
| **代价 A** | 多租户之间、同一租户改名前后，列表里默认分类「长得不一样」；文档/培训需说明「带系统标记的那一条才是默认落点」。 |
| **代价 B** | 英文界面用户会一直看到中文「未分类」（除非另做显示层映射，本期不做）。更干净、零歧义。 |
| **对代码的影响面** | 仅 PATCH `/tags/categories/:id` 对 `isSystem` 是否允许改 `name`；回填 SQL 初值字符串；UI 是否隐藏 system 的「重命名」。 |
| **建议** | **A**：与「租户数据可运营」一致；身份不依赖名字。若品牌/合规要求默认桶名称全局统一，再选 B。 |
| **拍板时机** | **M2 前**；不拍板则按 A 实现。 |

---

#### ④ 目录树是否要做拖拽移动（文件/目录）？

| | |
| --- | --- |
| **问题本质** | 整理效率：对话框「移动到…」完整但多一步；拖拽更顺手，但 iframe 内 DnD、权限提示、与多选/滚动的交互成本显著，且易与「仅空目录可删」等规则缠在一起。 |
| **选项 A（本期默认）** | **不做拖拽**。文件移动 = 工具栏/行操作「移动到…」+ 树单选对话框；目录「移动/重命名」走 PATCH 名称（同级改名）或后续再议「移动子树到另一父路径」（若本期 PATCH 仅改 leaf name，跨父移动 = renameFolderSubtree 的 from/to 全路径，UI 用对话框选目标父路径即可，仍非拖拽）。 |
| **选项 B** | 本期做文件拖到目录节点；目录拖拽改父路径（子树 rename）一并做。 |
| **选项 C** | 只做文件拖拽，目录仍对话框——半套交互，一致性差，**不推荐**除非有明确工作量盒。 |
| **代价 A** | 重度整理用户多点几次；无拖拽 discoverability。 |
| **代价 B** | 前端工作量与无障碍/触控/iframe 边界上升；需处理拖到无权限目录、拖到自身子树（环）、拖时树滚动等；验收面扩大，可能挤占 M4。 |
| **对代码的影响面** | 选 A：零额外 API（已有 PATCH folder / renameFolderSubtree）。选 B：主要是 `FolderTree`/`FileView` DnD + 冲突与权限 toast；后端可不变。 |
| **建议** | **A 出本期**；把拖拽列为 UI 增强 backlog。若用户验收强烈要求，单独立项或 M4 后增量，不阻塞目录/标签/缩略图主路径。 |
| **拍板时机** | **M4 开工前**；默认不拖拽即可开工 M1–M3。 |

---

#### ⑤ `files.tags` 旧列 contract（物理 DROP）排期锚点？

| | |
| --- | --- |
| **问题本质** | 本期 expand：新代码读写词典并双写旧列；旧进程滚动窗口仍可能只写旧列。**过早 DROP** → 未升级实例立刻报错。**过晚 DROP** → 双写税与「两套真相」长期存在，回填脚本要反复跑。 |
| **选项 A（建议默认）** | 本期上线后 **至少观察一个完整发布周期**（或全部环境——dev/staging/prod——确认已无旧二进制）→ 再发 **contract 小版本**：先重跑 `scripts/backfill-tags.ts` 调和 → 监控双写一致/只读旧列流量为 0 → 迁移 `0005` DROP `files.tags` 与 GIN → 删除 `replaceFileTags` 双写分支与旧列相关断言。 |
| **选项 B** | 与 FILE-ORGANIZE **同大版本内**紧接着发 contract（例如生产全量滚动完成后 24–72h 内）。更快去掉双写，但回滚窗口窄：一旦漏网旧实例或外部脚本直读旧列会炸。 |
| **选项 C** | 无限期保留旧列双写，不排 contract。最省事，技术债永久化；**不推荐**除非有未知外部 SQL 依赖 `files.tags`。 |
| **验收/发布门（无论 A/B）** | ① 所有部署单元已跑新版本 ≥1 个周期；② contract 前 backfill 幂等全绿；③ 抽检 `file_tags` 与旧列 tenant 名一致；④ 无只读旧列的旁路作业（BI/手工 SQL）或已改读 API；⑤ DROP 后 verify-search/全套件仍绿（名称过滤已只走 `file_tags`）。 |
| **对代码的影响面** | **不在本期交付**。需另开 goal/PR：迁移 0005、去掉双写、DTO/文档删除「旧列保鲜」表述、verify 去掉双写断言。 |
| **建议** | **A**。与部署「先 migrate 后滚动」对称：expand 从容、contract 在观测后做。B 仅在有强运维窗口且实例清单可控时采用。 |
| **拍板时机** | **本期上线后**由运维+研发定 contract 窗口即可；**不阻塞 FILE-ORGANIZE 实现**。计划在 §0.3 / 本节留锚，避免被人误以为本期要 DROP。 |

---

#### ⑥（可选补充）Tag cloud / 筛选的「多标签」语义：any-of 还是 all-of？

| | |
| --- | --- |
| **问题本质** | 列表 `tagIds=a,b` 与 cloud 多选筛选，用户可能理解为「带其中任一」或「同时带全部」。旧契约 `?tags=` 使用 `arrayOverlaps`，语义是 **any-of（并集）**。 |
| **选项 A（本期默认，与旧契约一致）** | **any-of**：命中任一选中标签即入结果。 |
| **选项 B** | **all-of**：必须同时具备所有选中标签。 |
| **建议** | **A**，并在 UI 文案写明「符合任一标签」。若产品强要 all-of，可加 `tagMode=all|any` 查询参数（默认 any），工作量小但要进 verify 与文档——**仅在验收投诉语义时再加**，不预做。 |
| **拍板时机** | 可与 ① 一并确认；不拍板则 A。 |

---

### 9.4 议题一览（给评审/例会用）

| # | 议题 | 状态 | 结论 / 默认 | 阻塞 | 主要改动面 |
| --- | --- | --- | --- | --- | --- |
| ① | 共享标签谁可创建 | **已拍板** | 租户 admin 创建/改/删；`ensureTagsByNames` 仍 get-or-create | — | POST `/tags` + UI |
| ② | 默认分类展示名 / 可否改名 | 开放 | 默认：初值 `未分类`，可改名，不可删 | M2 前 | PATCH categories + UI |
| ③ | meta 策略 | **已拍板** | 固定 schema（置信度/兴趣度/匹配度 + source/model/…）；只增 key | — | `parseFileTagMeta` + shared 类型 |
| ④ | 目录树拖拽 | 开放 | 默认不做，对话框移动 | M4 前 | 纯前端（若要做） |
| ⑤ | DROP `files.tags` 排期 | 开放 | 默认下一发布周期 + backfill 后 contract | 不阻塞本期 | 未来 0005 + 去双写 |
| ⑥ | 多标签筛选 any/all | 开放 | 默认 any-of（与旧 overlaps 一致） | 可选 | list 查询 + 文案 |


## 10. 端口 / 环境 / 已知坑（沿用既有教训）

- files-server :4100、files-app :5303（`apps/web/vite.config.ts:16-21`）；**旧文档的 5176 端口已过期**（memory: plan-4-implementation）。
- 迁移一律**手写 + 手登记 `meta/_journal.json`**（drizzle-kit generate 对 files-server 不可用，memory: plan-4/file-preview/file-storage-2-impl）。**P0**
- **部署是「先 migrate 后重启/滚动」**（Phase 4 migrate 先于 pm2 reload，`deploy/pm2/deploy.sh:322-326`；K8s 同序）——schema 变更必须 expand/contract：本期绝不 drop 旧列、绝不改旧列类型（本轮评审 P0-1）。**P0**
- `db:seed` 重生成租户 UUID 后必须重跑 `files:bootstrap`（`storage_configs` 按 tenantId 键），否则存储失联、全部 verify 红（memory: reseed-recovery-and-verify-all）。**P0**
- iframe 内 `window.prompt/confirm/alert` 被忽略 → 新建目录/重命名/打标签全部用 in-DOM modal（memory: microapp-ui-gotchas）。**P0（前端）**
- Tailwind `/alpha` 修饰符对主题 var() 色静默失效 → 用 `opacity-NN`（memory: microapp-ui-gotchas）。
- host↔app 路由：`sdk.onRoute` handler 必须幂等且不得回 `routeSync`；新路由维度（`/tags`、folder 深链）先匹配具体再匹配列表（memory: host-app-route-sync）。
- presign 必须存带租户前缀的完整 objectKey（memory: plan-4-implementation）——本期不改 objectKey 逻辑，回归用例覆盖。
- rendition 锁姿势固定 `INSERT…ON CONFLICT … setWhere` + `.returning()`（memory: file-preview-implementation）；thumbnail **worker 内**复用该姿势，ensure 路径不得提前占锁。**P0（第二轮）**
- OSS 禁 `response-content-type` override → presign inline 走 `ProviderTraits` 口子（memory: file-storage-multiprovider-impl）。
- 真转换验证要求 **files-server 进程本身** `PREVIEW_MEDIA_CONVERTER_URL=auto` 启动（只设在 verify 脚本无效；dev:all 默认 OFF）（memory: file-preview-2-implementation）。
- list `via=app/<appKey>` 对用户 actor 必须附 `ownerUserId=me`（memory: file-storage-2-impl）——本期新增的 folder/tagIds 过滤与 via 组合时该守护不可绕；**`verify-organize` 断言**。

## 11. 需求 → 设计映射（自检）

| # | 需求 | 本期落点 |
| --- | --- | --- |
| R1 | 支持新建目录 | §2 `folders` 表 + §4 POST `/folders`（严格校验/**recursive 默认 true**/幂等/根保护）+ §5 FolderTree 新建 modal → `verify-folders.ts` |
| R2 | 为文件打标签 | §0 决策 4 三表 + 共享标签 admin 闸 + `ensureDefaultCategory` + §4 PUT `/:id/tags`（固定 schema meta）+ §5 TagPickerDialog（批量=union 再 PUT）→ `verify-tags.ts` |
| R3 | 通过目录树浏览文件 | §4 GET `/folders`（显式 ∪ 祖先闭包 + 计数 + 软上限）+ list `folder` 过滤 + §5 左树右文件 → `verify-folders.ts` + §7.3 |
| R4 | 通过 Tag cloud 筛选文件 | §4 GET `/tags/cloud` + list `tagIds` + §5 `/tags` 页（cloud 多选 → FileView）→ `verify-tags.ts` + §7.3 |
| R5 | Grid 媒体类型缩略图 | §0 决策 6 / §0.2d `lib/thumbnails.ts`（先入队后占锁 + 有界并发）+ list `withThumbs` + §5 Thumb 扩展（图片+视频，缺省不 N+1）→ `verify-thumbnails.ts` |
| — | 多租户（门户硬约束） | §0 决策 8 + §6 第 2 条 → verify-folders/tags 租户矩阵 |
| — | 业务状态一律 200 + `{ok,data}` | §4 gate 注 + 全部新错误码走错误体 |
| — | 字典表 `sort` 规范 | §2 tag_categories/tags `sort` + orderBy(sort,name) |
| — | 三语 i18n + 前端两步法 | §5 + §7.3/§7.4 |
| — | 既有消费方 + 滚动部署零回归 | §3 契约表 + §0 决策 5 expand/contract + §7.1 回归门（verify-search/processors 等全套件） |
| — | 个人标签隐私（展示/过滤/搜索） | §0.2e + §6 第 3 条 + verify-tags 个人名不进 searchText/旧列 |
| — | **不在本期**：drop 旧列（contract）、app 空间树/标签、AI 打标、递归删除/回收站、meta UI、目录级权限、拖拽移动、分享面变更、分布式队列、树懒加载 | §0.3 明确出表 |

---

*本文档为 FILE-ORGANIZE 的设计与实施计划（**V3.1**：第二轮评审修订 + 开放问题 ①③ 产品拍板）。若代码与文档不符，以 `apps/files-server/src/`、`apps/files-app/src/` 与 `packages/` 的实现为准。`docs/Files文件服务接入指引.md` 相关章节按 M4（实现并验证通过后）回填。*
