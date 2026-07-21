# FILE-PREVIEW · 文件管理「分格式可扩展预览」开发计划

> 站在 `goal/PLAN-4.md`（文件管理独立后端 App）+ `goal/FILE-STORAGE.md`（多云存储 profile）+ 已落地的**处理器（processor）流水线**之上。
> PLAN-4 §0.3 把「在线预览渲染（Office/PDF 查看器）」显式排除在外，只给 presigned 下载/原生预览。本计划补上这块：为 files-server 增加一套**针对不同格式、可扩展的预览能力**，与现有「处理器」**同构而正交**——处理器把文件**转换/抽取成新文件**（`parse`→markdown、`image-compress`→压缩图），预览则把文件**呈现给人看**（不污染用户文件列表）。
> 核心约束:预览能力（「预览插件」）必须能被**任何依赖文件服务的下游 App** 调用——既能拿到机器可读的「预览描述符」，也能直接复用现成的渲染组件 / 嵌入式预览页,无需各自重造 pdf.js / mammoth / SheetJS。

---

## 0. 范围与决策

### 0.1 与现有「处理器」的关系（同构而正交）

| 维度 | 处理器 processor（已存在） | 预览 preview（本计划） |
| --- | --- | --- |
| 目的 | 转换/抽取 → **产出新文件**或写外部系统 | 呈现给人看 → **不产出用户文件** |
| 注册表 | `PROCESSORS: ProcessorDef[]`（`apps/files-server/src/modules/processors.ts`） | `PREVIEW_PROVIDERS: PreviewProviderDef[]`（新，同文件夹同范式） |
| 派发 | `POST /api/v1/files/:id/process` | `GET /api/v1/files/:id/preview` |
| 列举/可用性 | `GET /api/v1/files/processors` | `GET /api/v1/files/preview/providers` |
| 产物落点 | `saveDerivedFile`（**写进空间，成为用户文件**） | `file_renditions`（**缓存渲染件,按源 digest 失效,不入文件列表**） |
| 依赖解析 | token-exchange（omni-parser / knowledge）+ 可用性 reason | 独立 `PreviewUnavailableReason`(同名值对齐,见 §2.3);视觉转换走**基础设施转换器**(非 App,无 consent) |
| DTO | `ProcessorDTO` / `ProcessResultDTO`（`packages/shared/dto.ts`） | `PreviewDescriptor` / `PreviewProviderDTO`（新增同文件） |

> 一句话:**预览 = 一个返回「预览描述符」的只读 API + 一套按格式分插件的渲染器 + 必要时的服务端渲染件缓存**。处理器的 `parse`(omni-parser 文本抽取)可作为**冷门格式的文字兜底**被预览复用,但**保真视觉**(PDF 渲染、PPT 翻页)走专门的渲染件管线。

### 0.2 关键决策（推荐项 + 取舍,显式记录）

| # | 决策点 | 选择（已确认） | 取舍说明 |
| --- | --- | --- | --- |
| 1 | **两档保真度** | **客户端原生/库渲染(默认) + 服务端渲染件(兜底)** 双档 | 绝大多数格式(图片/PDF/音视频/markdown/**docx/pptx/xlsx**)浏览器侧用稳定库**即时渲染、零服务端算力、零新基础设施**;只有浏览器彻底搞不定的(**非 web 视频海报、heic**)才落少量服务端转换。先交付客户端档(Phase 1)立刻可用。 |
| 2 | **渲染器以「共享组件包」交付** | **`@xgent/file-preview`(React 渲染器库) + 嵌入式预览页双通道** | 「预览插件被其他 App 调用」的本质:别让每个下游 App 重写 pdf.js/docx 渲染。共享组件给「想原生嵌入」的 App;嵌入式 iframe 给「零打包、零 CSP 改动」的 App,**iframe 为默认推荐**(决策 #6)。 |
| 3 | **渲染件存储** | **新表 `file_renditions`(缓存语义,按源 `digest` 失效)** | 不复用处理器的 `saveDerivedFile`——预览件是**派生缓存**不是用户内容,塞进文件列表会污染、且无法按内容变更自动失效。存租户自有 bucket 保留前缀 `_previews/<fileId>/<kind>`。 |
| 4 | **Office 范围与方式** | **本期只做 `pptx` / `docx`,且都走客户端库;legacy `.ppt`/`.doc`/`.xls` 暂不预览(下载卡片);不引入 office→PDF 转换器** | 砍掉 Gotenberg/LibreOffice 整块基础设施 —— Phase 2 大幅瘦身。**取舍(显式)**:客户端 pptx 渲染库的保真度**不如**「转 PDF + pdf.js」路线(复杂动画/嵌入字体/SmartArt 可能走样),这是为「零转换器基础设施」付的代价;未来要高保真,可加 Gotenberg→PDF 路线**平滑升级**(描述符 `renderer` 从 `slides` 切到 `pdf`,下游无感)。 |
| 5 | **不新增 scope** | **预览复用 `files.read`** | 预览是读的衍生;渲染件写进租户自有 bucket 是缓存非新授权。下游 App 已能 `files.read` 就能预览,**零授权面扩张**。 |
| 6 | **跨应用 UI 默认通道** | **嵌入式 iframe(通道 C)** | 下游零打包、零 bucket CSP 改动、安全集中、重型库只一份。共享组件(通道 B)仍交付,留给要紧耦合的 App。 |
| 7 | **非 web 视频(mov/avi/mkv)** | **仅 ffmpeg 抽首帧海报 + 引导下载,不在线转码** | 转码 mp4/HLS 成本高,留作后续异步任务。海报是唯一保留的服务端视频处理。 |

> 以上 7 条均已确认,无遗留待决项。唯一保留的服务端转换基础设施是**轻量 ffmpeg(视频海报)**,office 转换器(Gotenberg)**不在本期**。

### 0.4 明确不在本期

- 协同批注/划词评论/版本对比预览;预览内编辑(只读呈现)。
- 全文 OCR 入索引(归 omni-parser/处理器);音频转写、视频字幕。
- 在线视频转码流水线、HLS 切片(Phase 2 仅首帧海报);mermaid/复杂 LaTeX 渲染(markdown 仅基础 + 可选 KaTeX)。
- 预览级权限再细分(预览沿用 `files.read` + 空间成员/分享角色,不引入「可看不可下」的新 ACL,除非 §6 分享档需要)。

---

## 1. 架构总览

```
                          ┌────────────────────────────────────────────┐
  下游 App (knowledge/     │  files-server  /api/v1/files/:id/preview     │
  mistakes/exam…)         │  ── gate(files.read) ──────────────────────  │
        │                 │  1. 查 files 行 → 命中 PREVIEW_PROVIDERS     │
        │ sdk.files.preview│  2. native/client 档 → presignGet(原件)      │
        │ 或 token-exchange │  3. rendition 档 → 查 file_renditions 缓存   │
        ▼                 │     ├ ready  → presignGet(渲染件)            │
   PreviewDescriptor ◀────┤     ├ 缺/过期 → 触发转换器 → pending(轮询)    │
   {renderer, status,     │     └ failed → reason / 文字兜底             │
    source, rendition,    └────────────────────────────────────────────┘
    poster, meta, caps}            │ 渲染件落 租户bucket: _previews/<id>/<kind>
        │
        ▼  三条「被调用」通道(§6)
   ┌─────────────────────┬───────────────────────────┬────────────────────────┐
   │ A. 描述符 API        │ B. 共享渲染器组件          │ C. 嵌入式预览页         │
   │ sdk.files.preview() │ @xgent/file-preview        │ files-app /preview/:id  │
   │ / exchange 直调      │ <FilePreview descriptor/>  │ 或公开 /p/:token        │
   │ → 机器可读           │ → 下游自渲染(原生嵌入)     │ → iframe,零重写         │
   └─────────────────────┴───────────────────────────┴────────────────────────┘
```

**三个一等公民:**
1. **预览描述符 `PreviewDescriptor`**——唯一跨进程契约。任何调用方拿到它就知道「用哪个渲染器、源在哪(presigned)、好了没」。
2. **服务端提供者注册表 `PREVIEW_PROVIDERS`**——按 MIME/扩展名分插件,镜像 `PROCESSORS`,声明 `strategy/renderer/rendition`。
3. **客户端渲染器注册表(`@xgent/file-preview`)**——按 `renderer` 键分组件(pdf/image/markdown/...),下游可 `registerRenderer()` 覆盖/扩展。服务端、客户端**双向可扩展**。

---

## 2. 数据模型

### 2.1 files-server 新表(独立迁移 `0001_file_renditions`)

| 表 | 关键列 | 说明 |
| --- | --- | --- |
| `file_renditions` | `id, tenantId, fileId fk(→files,cascade), kind('pdf'｜'thumbnail'｜'poster'｜'mp4'｜'text'), objectKey, mime, status('pending'｜'ready'｜'failed'), sourceDigest, pages int null, width int null, height int null, meta jsonb, attempts int not null default 0, lastError text null, createdAt, updatedAt`;唯一 `(fileId, kind)`;索引 `(tenantId,fileId)` | 派生渲染件**缓存**。`sourceDigest` ≠ `files.digest` ⇒ 缓存失效需重生(文件被覆盖上传后)。`status=pending` 兼作**防并发踩踏锁**(同 fileId+kind 只生一份)。 |

- **不改 `files` 表**。渲染件 object key 用保留前缀 `_previews/<fileId>/<kind>.<ext>`,落**租户自有 bucket**(隔离天然成立),`GET /api/v1/files` 列表查询排除 `_previews/` 前缀(本就不建 files 行,自然不出现)。
- 删源文件 → cascade 删 `file_renditions` 行 + best-effort 删 bucket 内 `_previews/<fileId>/` 对象。
- **并发与恢复(防踩踏锁的完整语义,别只写「pending 占位」)**:
  - **原子抢锁**:`INSERT ... ON CONFLICT (fileId, kind) DO UPDATE SET status='pending', attempts=attempts+1, updatedAt=now() WHERE file_renditions.status IN ('failed') OR (file_renditions.status='pending' AND file_renditions.updatedAt < now() - PENDING_STALE_TTL) OR file_renditions.sourceDigest <> $digest` —— 用 `RETURNING` 判断本请求是否抢到锁(抢到 → 去转换;没抢到 → 返回 `processing` 让客户端轮询)。
  - **stale pending 抢占**:进程崩溃/超时会留下永久 `pending`。`PENDING_STALE_TTL`(如 5min)过后允许另一请求抢占重试,`attempts` 自增。
  - **失败退避**:`attempts >= PREVIEW_MAX_ATTEMPTS`(如 3)→ 锁定 `failed` + `lastError`,不再自动重试(`?regenerate=1` 可人工重置);避免坏文件无限重转。

### 2.2 shared 增量(`packages/shared`)

- `dto.ts`:
  ```ts
  // none = 命不中任何 provider / 不可预览(配 renderer:"download")
  export type PreviewStrategy = "native" | "client" | "rendition" | "none";
  // 客户端渲染器键(与 @xgent/file-preview 组件一一对应)
  export type PreviewRenderer =
    | "image" | "svg" | "pdf" | "audio" | "video"
    | "markdown" | "code" | "csv" | "docx" | "slides" | "sheet" | "epub"
    | "text" | "download";          // slides = pptx; download = 不可预览,给下载卡片
  export type PreviewStatus = "ready" | "processing" | "unsupported" | "failed";

  // 预览自有的不可用原因词表(不再借用 ProcessorUnavailableReason —— 见 §2.3 取舍)
  export type PreviewUnavailableReason =
    | "not_installed" | "needs_consent" | "endpoint_not_configured" | "unreachable"  // app 依赖(omni-parser 文字兜底)
    | "converter_not_configured" | "converter_unreachable"                          // infra 转换器(ffmpeg)
    | "too_large" | "unsupported_format";

  export interface PreviewDescriptor {
    fileId: string; name: string; contentType: string; size: number; digest: string | null;
    renderer: PreviewRenderer; strategy: PreviewStrategy; status: PreviewStatus;
    source?:    { url: string; mime: string; expiresAt: string };          // 原件 presigned(inline disposition,§4.1)
    rendition?: { kind: string; url: string; mime: string; expiresAt: string }; // 渲染件 presigned(inline)
    poster?:    { url: string };                                            // 缩略图/海报(可选)
    meta?: Record<string, unknown>;        // width/height/duration/pageCount/sheetNames…
    reason?: PreviewUnavailableReason;     // 仅 status 为 unsupported/failed/processing-阻塞 时给出
    capabilities?: { download: boolean; print: boolean };                   // 按调用者角色/分享档收口
  }

  export interface PreviewProviderDTO {
    key: string; accepts: string[]; mimePrefixes: string[];
    strategy: PreviewStrategy; renderer: PreviewRenderer;
    rendition: { kind: string; via: "infra" | { app: string }; targetMime: string } | null;
    available: boolean;
    reason: PreviewUnavailableReason | null;
  }
  ```
- `errors.ts`:`PREVIEW_UNSUPPORTED`、`RENDITION_FAILED`、`CONVERTER_UNREACHABLE`、`FILE_TOO_LARGE_TO_PREVIEW`。**HTTP 码与 `reason` 词表一一对照**(errors 是传输/抛错通道,`reason` 是描述符内的业务态;两者命名对齐避免漂移)。
- `constants.ts`:`PREVIEW_MAX_INLINE_BYTES`(文本/markdown/docx/csv 客户端拉全文上限)、`PREVIEW_MAX_RENDITION_BYTES`(允许送转换器的源上限)、`PREVIEW_RENDITION_PREFIX = "_previews"`、`RENDITION_KINDS`、`PENDING_STALE_TTL`(stale pending 抢占阈值)、`PREVIEW_MAX_ATTEMPTS`(渲染件重试上限)。

### 2.3 取舍:预览 reason 独立成词表,不借用 `ProcessorUnavailableReason`

预览的不可用面**比处理器宽**(多了 `too_large`、`unsupported_format`、`converter_unreachable`,而处理器的语义集是「依赖 App 可用性」)。早期借用 `ProcessorUnavailableReason` 会让前端分支不稳定(provider reason 没有 `too_large`、errors 里的 `CONVERTER_UNREACHABLE` 又不在 reason 里)。故**独立** `PreviewUnavailableReason`,把 app 依赖原因(与处理器同名的四个)+ infra 转换器原因 + 资源/格式原因**收进一个稳定枚举**,前端一处 switch 全覆盖。两词表同名项(`not_installed` 等)保持字面一致,便于复用 UI 文案。

---

## 3. 预览提供者注册表（按格式分插件,镜像 `ProcessorDef`）

`apps/files-server/src/modules/preview.ts`,与 `processors.ts` 同范式:

```ts
interface PreviewProviderDef {
  key: string;                    // "image" | "pdf" | "office-pdf" | "docx" | ...
  accepts: string[];              // 小写扩展名
  mimePrefixes: string[];         // 例 ["image/"]、["video/"];与 accepts 取并集匹配
  strategy: PreviewStrategy;      // native | client | rendition
  renderer: PreviewRenderer;      // 客户端用哪个组件渲染
  rendition?: {                   // 仅 strategy==="rendition"
    kind: string;                 // "pdf" | "poster" | "mp4" | "text"
    via: "infra" | { app: ExchangeAudience };  // infra=转换器sidecar(无consent);app=token-exchange
    targetMime: string;
  };
}
```

### 3.1 内置提供者表（按格式 → 策略/渲染器/库）

| 格式 | 扩展名 | 策略 | renderer | 库（稳定现成） | 备注 |
| --- | --- | --- | --- | --- | --- |
| 图片 | jpg/jpeg/png/gif/webp/avif/bmp/ico | native | image | 浏览器原生 `<img>` | 缩放/平移/适配;无服务端 |
| 矢量图 | svg | client | svg | **DOMPurify**(SVG profile) | 取文本→卫生化→渲染,**XSS 收口** |
| HEIC/HEIF | heic/heif | rendition | image | 服务端 sharp/`@jsquash` 转 jpg（或客户端 heic2any) | 浏览器不原生 |
| PDF | pdf | client | pdf | **pdfjs-dist**(Mozilla) | 分页/缩放/文本层/打印;range 流式 |
| 音频 | mp3/wav/ogg/m4a/aac/flac | native | audio | 原生 `<audio>` | flac/m4a 视浏览器;波形 wavesurfer 可后续 |
| 视频(web) | mp4/webm/ogg | native | video | 原生 `<video>` | range 流式;海报可选 |
| 视频(非web) | mov/avi/mkv/wmv/flv | rendition | video | **ffmpeg**(服务端,抽首帧海报) | 仅海报 + 引导下载,不在线转码(决策 0.2#7) |
| Markdown | md/markdown | client | markdown | **markdown-it + DOMPurify + highlight.js**(+ 可选 KaTeX) | 卫生化必做 |
| 文本/代码 | txt/log/json/xml/yaml/源码(js/ts/py/...) | client | code | **highlight.js**(或 shiki) | 行号/语法高亮;超 `PREVIEW_MAX_INLINE_BYTES` 截断+提示 |
| CSV/TSV | csv/tsv | client | csv | **papaparse** | 虚拟滚动表格 |
| Word | docx | client | docx | **docx-preview**(保真排版) 或 mammoth(语义 HTML) | 输出经 DOMPurify |
| PPT | pptx | client | slides | **pptx-preview**(或同类客户端 pptx 库) | **保真有限**(决策 0.2#4),复杂动画/SmartArt 可能走样;未来可升级 PDF 路线 |
| Excel | xlsx | client | sheet | **SheetJS(xlsx,CE)** → HTML 表 | 多 sheet 切页;xls 同库可读,可顺带开 |
| EPUB | epub | client | epub | **epub.js**(可选,延后) | 翻页/目录 |
| 旧二进制 office | doc / ppt / xls | none | download | 本期不预览 → 下载卡片 | 决策 0.2#4;未来加 office→PDF 转换器再开 |
| 兜底 | 其它/未知 | rendition? | text/download | omni-parser `parse`(文字摘要,若已装) 否则 download 卡片 | 复用处理器做文字兜底 |

> **无 office→PDF 转换器**(决策 0.2#4):docx/pptx/xlsx 一律走客户端库即时渲染,legacy `.doc/.ppt/.xls` 给下载卡片。代价是 pptx 客户端保真不及 PDF 路线 —— 这是为「零转换器基础设施」付的显式取舍,描述符 `renderer:"slides"` 预留了未来切 `pdf`(Gotenberg 路线)的平滑升级位,下游无感。唯一服务端转换是 **ffmpeg 视频海报 + heic→jpg**。

### 3.2 提供者解析与可用性

- 解析:按 `extOf(name)` 命中 `accepts`,再按 `contentType` 命中 `mimePrefixes`,扩展名优先。命不中 → `{strategy:"none", renderer:"download", status:"unsupported", reason:"unsupported_format"}`(永远有兜底,不报错;契约自洽 —— `none` 策略配 `download` 渲染器)。
- 可用性(仅 rendition 档):`via:"infra"` → 看 env 转换器 URL 是否配置(否则 `converter_not_configured`);`via:{app}` → 复用 `exchangeToken` 解析(`needs_consent`/`not_installed`/...)。`GET /api/v1/files/preview/providers` 列举所有提供者 + 每租户可用性(镜像 `/processors`),供下游做能力探测/UI 置灰。

---

## 4. files-server 机制

### 4.1 描述符端点 `GET /api/v1/files/:id/preview`

> **前置改动(P1,必做):预览必须用 inline disposition 的 presign。** 现有 `presignGet(tenantId, objectKey, filename?)`(`apps/files-server/src/lib/s3.ts:195`)**只要传 `filename` 就强制 `Content-Disposition: attachment`** —— 预览端若直接复用,浏览器会当**下载**处理,PDF/图片/视频无法 inline 渲染。故给 s3 层加 inline 能力:把签名扩成 `presignGet(tenantId, objectKey, opts?: { filename?: string; disposition?: "inline" | "attachment"; responseContentType?: string })`(默认仍 `attachment` 保持下载端不变;预览传 `disposition:"inline"`,并对存成 `application/octet-stream` 的对象用 `responseContentType` 覆写为真实 MIME,确保浏览器 inline 渲染)。下载端 `GET /:id/download` 行为不变。

`gate(headers, "files.read")` → 查 `files` 行 + 空间成员校验(沿用现有 `requireSpaceRole(viewer)`)→ 命中 provider:

- **native / client 档**:`presignGet(原件, {disposition:"inline", responseContentType:mime})` → 回 `{renderer, strategy, status:"ready", source:{url,mime,expiresAt}, meta}`。**零服务端算力**,描述符即时返回。
- **rendition 档**:查 `file_renditions(fileId, kind)`:
  - `ready` 且 `sourceDigest===files.digest` → `presignGet(渲染件, {disposition:"inline"})` → `{rendition:{...}, status:"ready"}`。
  - 缺失/`sourceDigest` 不符/`failed`(未超重试上限)→ **原子抢锁置 `pending`(§2.1 并发语义)** → 异步触发转换(§4.2)→ 即时返回 `{status:"processing"}`,客户端轮询同端点;未抢到锁的并发请求也直接收 `processing`。
  - `failed` 且 `attempts ≥ PREVIEW_MAX_ATTEMPTS` → `{status:"failed", reason, capabilities:{download:true}}` + 若 omni-parser 可用则降级到 `text` 渲染器(文字摘要)。
- 查询参数:`?kind=poster|thumbnail|text`(指定渲染件)、`?regenerate=1`(强制重生,管理用)。
- `capabilities` 按调用者角色/分享档收口(viewer 分享可 `print:false, download:false`,见 §6.3)。

### 4.2 渲染件管线（rendition）

> 本期 rendition 档**只剩三类**(office 转换器已按决策 0.2#4 砍掉):

- **infra ffmpeg**(非 web 视频 → 首帧海报、heic → jpg):env `PREVIEW_MEDIA_CONVERTER_URL`(或 files-server 内置/sidecar ffmpeg 调用)抽首帧/转缩图 → `putObject(_previews/<fileId>/poster.jpg)` → `file_renditions` 转 `ready`。视频**只产海报不转码**(决策 0.2#7)。
- **app 依赖兜底**(冷门/未知格式 → 文字):复用 `exchangeToken(...,"omni-parser")` 跑 `parse` 拿 markdown,落 `kind:"text"` 渲染件,交 `text` 渲染器。
- **缩略图**(任意图片/PDF/视频 → 列表网格小图,Phase 3):in-process `@jsquash`/`sharp`(图片)、pdf.js 首页(PDF)、ffmpeg 首帧(视频)。
- **执行模型**:**懒生成 + digest 缓存**为基线(首次预览触发,之后命中缓存)。可选**预生成**:复用 `file.uploaded` 上传 Hook,上传后台预先生成海报/缩略图(Phase 3,降低首屏等待)。
- **限流/超时/上限**:源 > `PREVIEW_MAX_RENDITION_BYTES` → `too_large`(给下载卡片);ffmpeg 超时 `AbortSignal.timeout`;失败落 `failed` + `error`,不抛 5xx(业务态 200 + 描述符,符合项目 API 约定)。

### 4.3 安全（详见 §7）

卫生化(SVG/markdown/docx→HTML 走 DOMPurify)、presign 短时效 + 复用空间/分享鉴权、转换器**只接受同 bucket presigned 或字节**(防 SSRF)、size 上限、渲染件租户隔离。

---

## 5. 客户端渲染器 `@xgent/file-preview`（共享组件包,可被任何 App 复用）

新 workspace `packages/file-preview`(React + 上述库),**这就是「能被其他 App 调用的预览插件」本体**:

- 导出 `<FilePreview descriptor={PreviewDescriptor} locale theme onDownload? />`——按 `descriptor.renderer` 派发到对应子组件(`PdfRenderer`/`ImageRenderer`/`MarkdownRenderer`/`DocxRenderer`/`SheetRenderer`/`CsvRenderer`/`AudioRenderer`/`VideoRenderer`/`CodeRenderer`/`SvgRenderer`/`TextRenderer`/`DownloadCard`)。
- 导出 `<FilePreviewById fileId sdk />`——薄封装:内部 `sdk.files.preview(id)` 取描述符 + 轮询 `processing` + 渲染,下游一行接入。**`sdk.files.preview` 在 Phase 0 就位**(不拖到后期),组件与 SDK 同期落地、同一闭环验证。
- **客户端可扩展**:`registerRenderer(rendererKey, Component)` / `overrideRenderer(...)`——下游 App 可注入自定义渲染器(对称于服务端 `PREVIEW_PROVIDERS`)。
- **组件边界统一卫生化**:`SvgRenderer`/`MarkdownRenderer`/`DocxRenderer`/`SheetRenderer` 凡产 HTML 一律在组件内过 DOMPurify(库输出不可信),调用方无需关心。
- **依赖懒加载 + 打包预算**:pdf.js / SheetJS / docx-preview / pptx-preview 等重型库 `dynamic import`,按命中的 renderer 才拉。**pdf.js worker 明确打包策略**(Vite `new Worker(new URL('pdfjs-dist/build/pdf.worker.min.mjs', import.meta.url))` 单独 chunk + 运行环境 `worker-src blob:`);给包设 size budget(§12)。
- **体验底线(impeccable)**:跟随宿主主题(`.dark`/`data-theme`)与语言;loading/empty/error/unsupported/processing(轮询进度)态;键盘(PDF 翻页、图片缩放)、reduced-motion、可访问性(alt/aria)、大文件渐进加载。
- 自带 zh-CN/en/zh-TW 词典(对齐 files-app i18n 做法)。

> 取舍:共享组件让下游**原生嵌入**(无 iframe),但下游需打包重型库 + 放行 bucket 的 `img-src/media-src/connect-src/worker-src`(+ `blob:`)CSP(presigned 直取 + pdf.js worker)。不想承担的 App 走 §6.3 嵌入式 iframe(零打包、零 CSP 改动)。

---

## 6. 跨应用调用（核心需求:预览插件被依赖文件服务的 App 调用）

三条通道,按下游形态选,共享同一个 `PreviewDescriptor` 契约:

### 6.1 通道 A · 描述符 API（机器可读,所有调用方）

- **嵌入式 App(浏览器侧)**:`sdk.files.preview(id, opts?)` → 经 host-proxy `callService("files", "/api/v1/files/:id/preview")`(零控制面 CORS,host 挂 TDT)。SDK 新增:
  ```ts
  files.preview(id: string, opts?: { kind?: string; signal?: AbortSignal }): Promise<PreviewDescriptor>;
  files.previewProviders(): Promise<PreviewProviderDTO[]>;
  ```
- **独立后端下游(服务端到服务端)**:token-exchange 出 `aud=files` TDT(scope 交集含 `files.read`)→ 直调 `GET /api/v1/files/:id/preview`。零新机制(沿用 PLAN-4 §3.2)。
- 返回描述符后,下游可自行渲染(通道 B)或直接用 `source.url`/`rendition.url` 自处理。

### 6.2 通道 B · 共享渲染器组件（原生嵌入）

下游 `import { FilePreviewById } from "@xgent/file-preview"` → `<FilePreviewById fileId sdk={portalSdk}/>`。一行拿到全格式预览,无 iframe。适合**深度集成**(如 knowledge 在文档详情页内嵌预览、mistakes 预览错题附件)。

### 6.3 通道 C · 嵌入式预览页（零重写,推荐默认）

- files-app 新增路由 `/preview/:id`(走 host-app route-sync,支持深链)——一个**纯预览壳**,内部就是 `<FilePreviewById>`。下游 `iframe src` 指向它(门户 host 已放行 files-app `frame-src`),或经 host-proxy 同源 `/apps/files/preview/:id`。**下游零打包、零 bucket CSP 改动**——重型库与 presigned 直取都关在 files-app 这一份里。
- **公开预览**(无 TDT):扩展 PLAN-4 现有分享 `POST /s/:token`——`viewer` 角色已约定回「预览元数据」,本期落实为返回 `PreviewDescriptor`(`capabilities.download` 按分享档),配独立公开页 `/p/:token`。让分享链接打开即预览(不强制下载),且可设「只看不可下」。

> 默认推荐**通道 C**(集成成本最低、安全集中、库只一份);通道 B 留给要紧耦合的 App;通道 A 是两者的底座。三者都不需要新 scope(§0.2#5)。

---

## 7. 安全（上传文件不可信,逐项收口）

- **注入卫生化**:SVG / markdown 渲染 HTML / docx→HTML **一律 DOMPurify**(SVG 用 SVG profile,禁 `<script>`/事件属性/`<foreignObject>`);markdown 链接 `rel=noopener`、禁 `javascript:`。这是上传文件最大的 XSS 面。
- **presign 收口**:渲染源/件 presigned GET 短时效(沿用 15min);预览鉴权 = 下载鉴权(空间成员/分享角色),`viewer` 仅得预览不一定得下载(`capabilities`)。
- **SSRF**:转换器 sidecar **只接受 files-server 传入的字节或同 bucket presigned URL**,绝不接受任意外链;转换器置内网、不暴露公网。
- **资源上限**:`PREVIEW_MAX_INLINE_BYTES`(客户端拉全文)、`PREVIEW_MAX_RENDITION_BYTES`(送转换器),超限给下载卡片;pdf.js range 流式避免整文件入内存;转换器超时熔断。
- **隔离渲染**:嵌入式预览页 `iframe sandbox`(允许 scripts,禁 top-navigation)+ 严格 CSP;渲染件落**租户自有 bucket**,跨租户天然隔离。
- **缓存投毒**:渲染件按 `sourceDigest` 键定,源被覆盖上传(digest 变)→ 旧渲染件自动失效重生,杜绝「看到的是旧内容」。

---

## 8. 前端（files-app 预览 UI · 强制两步法:impeccable 设计 + Chrome 真浏览器验证）

- **文件列表/网格**:接 `descriptor.poster`/thumbnail 渲染缩略图(替代纯图标);hover 快速预览。
- **预览页 `/preview/:id`**:全屏 `<FilePreviewById>`——顶栏(文件名/大小/下载/分享/打印)+ 主区按 renderer 渲染 + 处理中轮询进度 + 不支持态(下载卡片)。
- **文件菜单**:在现有「下载/处理/分享/...」旁加「预览」(`FileView.tsx` 的 `FileMenu`),打开预览页或抽屉。
- **转换器状态**:管理员设置页显示 ffmpeg(视频海报/heic)连通性(若启用)。
- 跟随宿主主题/语言实时切换;三语 parity;空/错/载入/处理态齐全。

---

## 9. Seed / env / 基础设施

- **env(files-server)**:`PREVIEW_MEDIA_CONVERTER_URL`(或内置 ffmpeg 开关;空=非 web 视频/heic 落 `converter_not_configured`,给下载卡片)、`PREVIEW_MAX_INLINE_BYTES`、`PREVIEW_MAX_RENDITION_BYTES`、`PREVIEW_RENDITION_PREFIX=_previews`。**无 office 转换器 env**(决策 0.2#4)。
- **基础设施(可选,Phase 2)**:仅需轻量 **ffmpeg**(视频海报/heic)——files-server 镜像内置 ffmpeg 二进制,或 `docker-compose` 加一个小 media-convert sidecar(profile `preview`,内网);本地 dev 不强制起(海报置灰/降级)。**不再需要 Gotenberg/LibreOffice。**
- **bucket CORS**:PLAN-4 §4.2 保存存储配置时已 `PutBucketCors`;需**确认放行 `GET` + `Range` + 暴露 `Content-Range`/`Accept-Ranges`**(pdf.js 流式必需),不足则补。
- **CSP —— files-app 自身先要改(P2,别只盯下游)**:当前 files-app `index.html` 的 CSP **只把 bucket origin 放在 `connect-src`,`img-src` 是 `'self' data: blob:`、没有 `media-src`/`worker-src`** —— 直接预览会把图片/SVG(`<img src=presigned>`)、音视频(`<audio>/<video src=presigned>`)、pdf.js worker 挡掉。Phase 0 必须把 files-app CSP 扩成:
  - `img-src 'self' data: blob: <bucket origins>`(图片/SVG/缩略图/海报)、
  - `media-src 'self' blob: <bucket origins>`(音视频,新增指令)、
  - `connect-src`(已含 bucket)再 `+= blob:`(pdf.js / docx-preview / SheetJS fetch ArrayBuffer)、
  - `worker-src 'self' blob:`(**pdf.js worker**,否则 worker 起不来;`worker-src` 缺省回落 `script-src` 不含 `blob:`)。
  - bucket origins 同 §FILE-STORAGE:dev minio `http://localhost:9000` + 云域(`https://*.myqcloud.com`/`https://*.amazonaws.com`/自定义 generic-s3 host)。
- **CSP —— 下游 App**:走**通道 C(iframe)零改动**(媒体取数关在 files-app 这份 CSP 里);只有走**通道 B(共享组件)**的下游才需各自 `img-src/media-src/connect-src/worker-src += bucket origin + blob:`(这是默认推荐通道 C 的硬理由之一)。
- **根脚本**:`db:files:generate|migrate` 跑新迁移 `0001_file_renditions`;`dev:all` 无新增前端进程(预览页在 files-app 内)。

---

## 10. 验证

- **files-server 脚本**(`apps/files-server/scripts/`,对真 minio):
  - `verify-preview-native.ts`:图片/pdf/音频/视频/markdown/text/csv/**docx/pptx/xlsx** → 描述符 `status:ready` + `source.url` presigned 可取 + renderer 正确(docx→`docx`、pptx→`slides`、xlsx→`sheet`)+ **`source.url` 响应头是 `Content-Disposition: inline`(非 attachment)**(P1 回归)。
  - `verify-preview-contract.ts`:命不中 → `{strategy:"none", renderer:"download", status:"unsupported", reason:"unsupported_format"}` 自洽;`reason` 取值全在 `PreviewUnavailableReason` 内;每个 provider 的 `strategy`↔`renderer` 组合合法。
  - `verify-preview-rendition.ts`:非 web 视频 → `processing`→轮询→`ready` + 海报可取;heic→jpg;`sourceDigest` 缓存命中;覆盖上传(digest 变)→ 重生;**并发两请求只触发一次转换(原子抢锁)**;**死 `pending` 过 `PENDING_STALE_TTL` 被抢占重试**;`attempts ≥ MAX` 锁 `failed`;ffmpeg 未配 → `converter_not_configured` 给下载卡片。
  - `verify-preview-security.ts`:含 `<script>` 的 svg/markdown/docx/**xlsx(公式注入/HTML)** 经卫生化后无可执行节点;超 `MAX` → `too_large` 下载卡片;`viewer` 分享 `capabilities.download:false`。
  - `verify-preview-exchange.ts`:下游(mistakes/knowledge)token-exchange `aud=files` → `GET /:id/preview` 命中;无授权被拒 = **跨应用闭环**。
  - `verify-preview-public.ts`:`/s/:token`(viewer)→ 公开预览描述符 + 过期/口令态。
- **Chrome 真浏览器**(memory:cross-origin iframe DOM 对扩展不透明 → **直接开 files-app `/preview/:id` 同源验证渲染**,非经门户 iframe):pdf 翻页/缩放、图片缩放、markdown 渲染、docx 排版、xlsx 多 sheet、pptx 翻页(客户端)、音视频播放、非 web 视频海报、旧二进制/不支持态下载卡片、切换语言/主题实时生效。
- `bun run typecheck` 全 workspace 绿;i18n 三语 parity(files-app + `@xgent/file-preview`);`bun run verify:all` 回归不破。
- **打包预算验收**:files-app + `@xgent/file-preview` 主 chunk 不含 pdfjs-dist/SheetJS/docx-preview/pptx-preview(均独立 lazy chunk);files-app bundle size budget 不超阈值(CI 卡)。

---

## 11. 分期（建议落地顺序）

> **原则(P1 修正):Phase 0 = 可运行的端到端最小闭环,不是「只搭契约骨架」。** 每加一种格式都用同一个入口验证,避免拖到后期才暴露跨应用调用 / CSP / route-sync 问题。故把 SDK 方法、最小渲染器、files-app `/preview/:id` 一并提到 Phase 0。

1. **Phase 0 · 最小闭环(server descriptor → SDK → renderer → 预览页,全链路打通)**:
   - shared(`PreviewDescriptor`/`PreviewProviderDTO`/`PreviewUnavailableReason`/errors/constants);
   - **s3 层 inline-disposition 改造**(§4.1 前置改动);files-server `0001_file_renditions` 迁移 + `preview.ts` 骨架 + `PREVIEW_PROVIDERS` + `GET /:id/preview`(先支持 **image/pdf** 两个 native/client 提供者 + `download` 兜底)+ `GET /preview/providers`;
   - **portal-sdk `files.preview` / `files.previewProviders`**(经 host-proxy `callService`);files-app CSP 放行(§9,img/media/connect/worker + bucket + blob:);
   - `packages/file-preview` renderer 注册表 + **最小 `image`/`pdf`/`download` 三个渲染器** + `<FilePreviewById sdk/>`;
   - **files-app `/preview/:id` 页面**(host-app route-sync)。
   → 闭环冒烟:portal 内打开 files-app `/preview/:id` 渲染一张图 / 一个 PDF,跑通「描述符→SDK→组件→CSP→route-sync」全链路。
2. **Phase 1 · 客户端档补齐余下格式**:svg(卫生化)/audio/video/markdown/code/csv/**docx/pptx/xlsx**/text 渲染器 + 卫生化 + presign 接线(沿用 Phase 0 闭环入口逐格式验证)。→ `verify-preview-native` + `verify-preview-security` + Chrome 验证。**此期连同 Phase 0 已覆盖用户列举的 pdf/图片/音频/视频/markdown/word/ppt(pptx)/excel。**
3. **Phase 2 · 渲染件管线(轻量,无 office 转换器)**:`file_renditions` 缓存 + 并发抢锁/stale 恢复(§2.1)+ **ffmpeg 视频海报 + heic→jpg** + omni-parser 文字兜底 + 懒生成 + digest 失效 + `processing` 轮询。→ `verify-preview-rendition`。
4. **Phase 3 · 跨应用补全 + 缩略图**:补 token-exchange 直调路径 + 共享组件供下游 import + 公开 `/p/:token`(扩展分享);文件网格缩略图(上传 Hook 预生成 poster/thumbnail)。→ `verify-preview-exchange` + `verify-preview-public`。(SDK/组件/预览页本体已在 Phase 0 就位,此期只补「被外部 App 调用」的剩余面。)
5. **Phase 4 · 收尾**:i18n parity、files-app 预览 UI 打磨(impeccable + Chrome)、README/CHECKLIST、`verify:all` 回归。

---

## 12. 关键复用点 & 实现期坑位

**复用**:`processors.ts` 的注册表/派发/可用性范式;`s3.ts` 的 `presignGet`/`getObjectBytes`/`putObject`/多云 profile;`exchange.ts` 的 `exchangeToken` + reason 词表(冷门格式文字兜底);`saveDerivedFile` 的反例(预览**不**这么存);MicroAppHost host-proxy + route-sync;PLAN-4 分享 `/s/:token` 公开访问范式;files-app shadcn + i18n 模板。

**坑位**:
- **`presignGet` 默认是 `attachment`**(传 `filename` 即强制下载,`s3.ts:195`)——预览**必须**走 inline 变体(§4.1 前置改动),否则 PDF/图片/视频被浏览器当下载,白屏。这是动手第一步,别等渲染不出来才回头改 s3 层。
- **pdf.js 流式需 bucket CORS 放行 `Range` + 暴露 `Content-Range`**——typecheck 查不出,只有真浏览器拉大 PDF 才暴露;`PutBucketCors` 模板要带上。
- **files-app 自身 CSP 要先改**(§9)——`img-src` 缺 bucket origin、无 `media-src`、无 `worker-src blob:` ⇒ 图片/音视频/pdf.js worker 直接被挡。这是 Phase 0 闭环跑不通的头号原因,且 typecheck 查不出。
- **卫生化在组件边界统一做**——SVG、markdown→HTML、**docx-preview / SheetJS 产出的 HTML** 都是不可信渲染面,都要过 DOMPurify(SVG 用显式 SVG profile);别假设「库的输出是干净的」。XSS 主面。
- **渲染件缓存按 `sourceDigest` 失效 + 并发抢锁要原子**——`pending` 占位必须走 §2.1 的 `INSERT ... ON CONFLICT` 原子抢锁 + stale TTL 抢占,否则进程崩溃留死 `pending`(永远 `processing`),或并发同时转一份(踩踏)。
- **转换器是基础设施不是 App**——别走 token-exchange/consent;只接受字节/同 bucket presigned 防 SSRF;未配置要优雅降级(`converter_not_configured` → 下载/文字兜底),不报 5xx。
- **通道 B 的下游需放行 bucket CSP**(presigned 直取 + `worker-src blob:`),通道 C 不需要——这是默认推荐 C 的硬理由;别让下游踩 CSP 坑才发现。
- **chrome 扩展看不到跨源 iframe DOM**(memory 已记)——验证渲染开 files-app `/preview/:id` 同源页,别经门户 iframe。
- **files-server start 无 --watch**;改 preview 模块需重启。迁移走独立链(`FILES_DATABASE_URL`),勿与门户 `db:generate` 混跑。
- **重型库打包预算写进验收**:`pdfjs-dist` worker(Vite `?worker`/`new Worker(new URL(...))` 配置 + `worker-src blob:`)、SheetJS CE、docx-preview、pptx-preview 一律 `dynamic import` + 单独 chunk;给 files-app 设 **bundle size budget**(超阈值 CI 失败),否则首屏被拖垮。
- **`pptx-preview` 生态稳定性需实现前 spike**——客户端 pptx 库成熟度参差(决策 0.2#4 已标保真有限)。Phase 1 动手前先拿真实 pptx 样本验证渲染质量/体积/维护活跃度;不达标就**降级为 `download` 卡片**(不要硬上一个会大面积走样的库),等未来 office→PDF 路线再补保真预览。

---

## 13. 需求 → 设计映射（自检）

| 需求 | 落点 |
| --- | --- |
| pdf / 图片 / 音频 / 视频 / markdown / word(docx) / **ppt(pptx)** / excel(xlsx) 预览 | §3.1 客户端档(pdfjs-dist / 原生 / markdown-it / docx-preview / pptx-preview / SheetJS);Phase 1 |
| 非 web 视频海报 / heic | §4.2 ffmpeg 渲染件(仅海报,不转码);Phase 2 |
| legacy `.doc` / `.ppt` / `.xls` | 本期不预览 → 下载卡片(决策 0.2#4;未来加 office→PDF 转换器再开) |
| 「针对不同格式的可扩展」 | §3 服务端 `PREVIEW_PROVIDERS`(镜像 `ProcessorDef`)+ §5 客户端 `registerRenderer` 双向可扩展 |
| 「尽可能用现成稳定库」 | §3.1 库选型表(pdfjs-dist/markdown-it/docx-preview/pptx-preview/SheetJS/papaparse/DOMPurify/highlight.js/epub.js + ffmpeg 视频海报) |
| 「被其他依赖文件服务的应用调用」 | §6 三通道:A 描述符 API(`sdk.files.preview` + token-exchange)/ B 共享组件 `@xgent/file-preview` / C 嵌入式预览页 + 公开分享预览 |
| 与现有处理器并存不冲突 | §0.1 同构而正交;§2.1 渲染件存 `file_renditions`(非 `saveDerivedFile`),不污染文件列表 |
| 安全 | §7 卫生化 + presign 收口 + SSRF + 上限 + 隔离 + digest 失效 |
