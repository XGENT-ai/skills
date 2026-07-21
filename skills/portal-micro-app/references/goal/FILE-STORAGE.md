# FILE-STORAGE · 文件管理存储多 Provider 支持（Tencent COS + AWS S3）

> 站在 `goal/PLAN-4.md`（文件管理 App · 独立后端 `@xgent/files-server` · 对象存储）之上。
> 本期目标：让租户管理员在**配置存储时可选择 provider**（腾讯云 COS / AWS S3 / 通用 S3 兼容），而非只能填一个裸 endpoint。

---

## 0. 范围与核心判断

### 0.1 核心判断（决定整体工作量）

**AWS S3、腾讯云 COS、MinIO 三者都是 S3 协议（SigV4 + REST）兼容存储**，而 files-server 当前已用 `@aws-sdk/client-s3` + `@aws-sdk/s3-request-presigner`（见 `apps/files-server/src/lib/s3.ts`）。因此本期**不是多驱动重写**，而是在现有单驱动之上加一层 **provider profile（厂商画像）**：

- 同一个 `S3Client`，靠 **endpoint / region / forcePathStyle** 三个参数的不同组合即可指向三家。
- provider 的真正价值是：**配置期的字段语义、默认值推导、校验规则、UI 提示** 因厂商而异——把"凭感觉填 endpoint"变成"选 provider→填厂商术语字段"。

> 符合 CLAUDE.md「Simplicity First / Surgical Changes」：复用现有驱动，不引入 `@aws-sdk` 之外的 SDK（不装 `cos-nodejs-sdk-v5`），改动面收敛在 *profile 推导 + 校验 + 表单*。

### 0.2 决策表

| # | 决策点 | 选择 | 含义 |
| --- | --- | --- | --- |
| 1 | 驱动 | **复用 `@aws-sdk/client-s3` 单驱动** | 不为 COS/AWS 各引入专用 SDK；三家共用 S3Client，参数化区分。 |
| 2 | provider 取值 | `s3` \| `aws` \| `cos` | `s3` = 现状（MinIO / 自建 / 其他兼容），**向后兼容**已落库的 `provider:"s3"` 行；新增 `aws`、`cos` 两个 profile。 |
| 3 | 每租户配置数量 | **仍为单条**（`storage_configs.tenantId` 主键不变） | "配置时选 provider" = 该租户的唯一存储指向哪一家，不做多 provider 并存。 |
| 4 | endpoint 处理 | **保存时按 provider 推导出具体 endpoint 落库** | `aws`/`cos` 的 endpoint 表单可留空，后端按 region 推导；`endpoint` 列保持 `NOT NULL`，**无需改表结构**。 |
| 5 | 凭证流向 | **不变**（PLAN-4 §0.2b）：浏览器 → files-server 直送，AES-GCM 落 files-server 库，`apps/api` 永不见 SK | 仅字段**标签**随 provider 变（COS 用 SecretId/SecretKey，AWS 用 Access Key ID/Secret Access Key）。 |

### 0.3 关键取舍（显式记录，不默默处理）

- **a. provider 是"画像"不是"驱动"。** 一旦某家 COS/AWS 出现 S3 SDK 无法覆盖的私有行为（如 COS 的批量/STS 临时密钥），届时才下沉为独立驱动。本期不预留该抽象（无投机抽象）。
- **b. 真·云端验证无法进 CI。** 仓库里没有真实 AWS / 腾讯云凭证。所以本期**自动化验证只覆盖**：profile 推导（endpoint/forcePathStyle/校验）的纯逻辑 + 浏览器里 provider 切换的 UX + 对**本地 MinIO**（走 `s3` profile）的真实直传链路。真实 AWS/COS 的连通+直传留**手动冒烟清单**（§7 末），由有凭证者执行后回填结论。
- **c. 校验"够用即可"。** COS bucket 必带 `-<appid>` 后缀 → 做硬校验给明确报错；其余（region 合法性等）只做最小校验，把"连不上"交给保存时的连通探针（`probeAndPrepareBucket`）兜底，错误信息透传。

### 0.4 明确不在本期

- COS/AWS 的 **STS 临时密钥 / 角色扮演 / 服务端 KMS 加密**；只支持长期 AK/SK。
- 存量数据在 provider 间**迁移/搬运**（换 provider/桶 = 指向新桶，旧对象不自动搬）——但 UI 会在改动已配置的存储指向时**显式提醒用户数据不会自动迁移**（见 §5）。
- 多 provider 并存、按空间/文件路由到不同 provider。
- Cloudflare R2 / 阿里 OSS / MinIO 以外的第四家（设计上 `s3` 通用 profile 已能兜，但不在本期显式列为选项）。

---

## 1. 现状盘点（as-is）

| 层 | 位置 | 现状 |
| --- | --- | --- |
| 表 | `apps/files-server/src/db/schema.ts:10` `storageConfigs` | 单条/租户：`provider`(默认 `s3`)、`endpoint`、`region`、`bucket`、`accessKeyId`、`secretKeyEnc`、`forcePathStyle`(默认 true)、`prefix`。**`provider` 列已存在但全程写死 `"s3"`。** |
| 驱动 | `apps/files-server/src/lib/s3.ts` | `buildClient()`→`S3Client`（endpoint/region/forcePathStyle/credentials）；`presignPut/presignGet/deleteObject`；`probeAndPrepareBucket()`（HeadBucket→缺桶 dev 自动建→`PutBucketCors`）。**未设 checksum 行为**（见 §6.1 陷阱）。 |
| API | `apps/files-server/src/modules/storage.ts` | `GET/PUT/DELETE /api/v1/files/storage`（`requireAdmin`）。`dto()` 与 upsert 的 `values` 都**硬编码 `provider:"s3"`**（:20、:63）。 |
| 共享类型 | `packages/shared/src/dto.ts:259` `StorageConfigDTO` | `provider: "s3"`（字面量，需放宽为联合类型）。 |
| 门户 API 客户端 | `apps/web/src/lib/files-config-api.ts` | 浏览器直连 files-server（门户先 mint 一个 aud=appKey 的 admin files-TDT）。`putStorage` 入参里**无 `provider`**。 |
| 配置 UI | `apps/web/src/pages/admin/FilesStorageSection.tsx` | 应用管理 → 文件管理 的「文件存储」段：endpoint/region/bucket/accessKey/secretKey/forcePathStyle/prefix 平铺表单。**无 provider 选择器。** |
| dev 种子 | `apps/files-server/scripts/bootstrap.ts`、`scripts/_helpers.ts` `MINIO_CONFIG` | 给晨光租户种本地 MinIO 配置。 |
| 验证 | `apps/files-server/scripts/verify-storage.ts` | 对真实本地 MinIO 跑 CRUD + 探针 + SK 不回显 + 非管理员拦截。 |

---

## 2. 目标设计（to-be）

### 2.1 Provider Profile 表（实现的事实来源）

| provider | endpoint（表单留空时推导） | region 示例 | forcePathStyle 默认 | bucket 规则 | 凭证术语（UI 标签） | 保存时自动建桶 |
| --- | --- | --- | --- | --- | --- | --- |
| `s3`（MinIO/自建/通用兼容，= 现状） | **必填**，如 `http://localhost:9000` | `us-east-1` | `true` | 任意 | Access Key / Secret Key | dev 允许（保留现状） |
| `aws`（AWS S3） | 留空 → `https://s3.<region>.amazonaws.com` | `us-east-1` / `ap-east-1` | `false` | 全局唯一 DNS 合规名 | Access Key ID / Secret Access Key | **否**（生产桶须预建） |
| `cos`（腾讯云 COS） | 留空 → `https://cos.<region>.myqcloud.com` | `ap-guangzhou` / `ap-singapore` | `false` | **必须 `<name>-<appid>`**（如 `media-1250000000`） | SecretId / SecretKey | **否** |

> endpoint 给具体值 + `forcePathStyle:false` 时，aws-sdk 会生成虚拟主机式 URL：`https://<bucket>.s3.<region>.amazonaws.com` / `https://<bucket>.cos.<region>.myqcloud.com`——COS/AWS 的标准形态。MinIO 走 path-style（`forcePathStyle:true`）。

### 2.2 推导函数（新增，files-server 侧）

在新文件 `lib/providers.ts`（比塞进 `s3.ts` 更内聚）新增纯函数：

```ts
export type StorageProvider = "s3" | "aws" | "cos";

// 把 (provider, 用户输入) 归一化成可落库的 effective 配置。
export function resolveProfile(input: {
  provider: StorageProvider;
  endpoint?: string; region: string; bucket: string;
  forcePathStyle?: boolean;
}): { endpoint: string; forcePathStyle: boolean } {
  switch (input.provider) {
    case "aws":
      return {
        endpoint: input.endpoint?.trim() || `https://s3.${input.region}.amazonaws.com`,
        forcePathStyle: input.forcePathStyle ?? false,
      };
    case "cos":
      return {
        endpoint: input.endpoint?.trim() || `https://cos.${input.region}.myqcloud.com`,
        forcePathStyle: input.forcePathStyle ?? false,
      };
    default: // "s3"
      return { endpoint: (input.endpoint ?? "").trim(), forcePathStyle: input.forcePathStyle ?? true };
  }
}

// provider 专属校验（在通用 endpoint/bucket/AK 必填之外）。
export function validateProfile(p: StorageProvider, v: { region: string; bucket: string }): void {
  if ((p === "aws" || p === "cos") && !v.region) throw new AppError("VALIDATION_FAILED", "该 provider 需填写 region");
  if (p === "cos" && !/-\d{6,}$/.test(v.bucket))
    throw new AppError("VALIDATION_FAILED", "COS bucket 需含 APPID 后缀，例如 media-1250000000");
}
```

> `s3` 分支与现状完全等价 → 存量行与现有 dev 流程零回归。

---

## 3. 数据模型变更

**无需 migration。** `provider` 列已存在（`text NOT NULL DEFAULT 's3'`），`endpoint` 保持 `NOT NULL`（aws/cos 落推导后的具体值）。仅放宽 TS 类型：

- `apps/files-server/src/db/schema.ts:12`：`provider` 给上 `$type<StorageProvider>()`（运行期不变，仅类型）。
- `packages/shared/src/dto.ts:260`：`provider: "s3"` → `provider: StorageProvider`（在 shared 导出 `StorageProvider` 联合类型）。

---

## 4. 后端变更（files-server）

| 文件 | 改动 |
| --- | --- |
| `lib/providers.ts`（新） | `StorageProvider` 类型 + `resolveProfile()` + `validateProfile()`（§2.2）。 |
| `lib/s3.ts` | ① `buildClient()` 与 `probeAndPrepareBucket()` 的 `S3Client` 统一加 **checksum 兜底**（§6.1）：`requestChecksumCalculation: "WHEN_REQUIRED"`、`responseChecksumValidation: "WHEN_REQUIRED"`。② `probeAndPrepareBucket()` 增 `provider` 入参——**仅 `s3` 在缺桶时自动建桶**，`aws`/`cos` 缺桶直接报 `STORAGE_UNREACHABLE`（生产桶不该被代码偷偷创建）。③ region 不匹配（AWS `301 PermanentRedirect`）给清晰报错（§6.2）。 |
| `modules/storage.ts` | ① PUT 入参收 `provider`（默认沿用 existing 或 `"s3"`）。② 调 `validateProfile` + `resolveProfile` 得 effective endpoint/forcePathStyle 再探针、再 upsert——`values` 的 `provider` 改为实际值（删除 :63 的硬编码）。③ `dto()` 返回 `c.provider`（删除 :20 的硬编码）。④ **`region` 的 `"us-east-1"` 兜底只对 `s3` 生效**：现有 :50 无条件 `?? "us-east-1"` 会让 `validateProfile` 的"aws/cos 缺 region"判断永远命中不了，并使 COS 静默推导出不存在的 `cos.us-east-1.myqcloud.com`。改为 aws/cos 不兜底默认、缺 region 即报 `VALIDATION_FAILED`。 |
| `lib/env.ts` / `scripts/bootstrap.ts` / `scripts/_helpers.ts` | dev 种子 `MINIO_CONFIG` 显式带 `provider:"s3"`（其余不变）。 |

> 探针逻辑（HeadBucket + 缺桶处理 + PutBucketCors）已存在，只在其上分流 provider；CORS 仍 best-effort（云控制台可能另管，保持非致命——见 `s3.ts:117`）。

---

## 5. 前端变更（门户配置 UI）

> CLAUDE.md 强制：前端 UI **设计阶段用 `impeccable` skill**，**验证阶段用 Chrome extension** 真浏览器跑通，缺一不可。

| 文件 | 改动 |
| --- | --- |
| `packages/shared/src/dto.ts` | 导出 `StorageProvider`；`StorageConfigDTO.provider` 放宽（§3）。 |
| `apps/web/src/lib/files-config-api.ts` | `putStorage` 入参 + 出参类型加 `provider`。 |
| `apps/web/src/pages/admin/FilesStorageSection.tsx` | 段首加 **provider 选择器**（`s3` / `aws` / `cos`，用 atoms 的 `Segmented<T>`（`atoms.tsx:216`，3 互斥项天然契合）或沿用其他 admin 页既有的原生 `<select>` 模式）；表单**随 provider 自适应**：<br>· 切换 provider 时设默认值（forcePathStyle、region 占位、清不适用字段）；<br>· `aws`/`cos` 的 endpoint 字段标"可留空，按 region 推导"占位；<br>· 凭证字段标签随 provider 变（COS=SecretId/SecretKey，AWS=Access Key ID/Secret Access Key）；<br>· region 字段给厂商示例占位（`ap-guangzhou` / `us-east-1`）；<br>· COS 选中时给 `bucket` 的 `-appid` 内联提示。 |
| `apps/web/src/lib/i18n.tsx` | 新增 `appForm.files.provider*`：provider 标签×3、三家选项名、各字段的 provider 态提示。补齐所有 locale（与现有 portal 词条数对齐，保持 ×N parity）。 |

**UI 行为约定**（写进实现，避免歧义）：
- 进入页面 → `getStorage` 回来后 hydrate，provider 选择器回显 `cfg.data.provider`（存量 `s3` 行正常回显为「通用 S3 兼容」）。
- 切 provider **只改表单态与默认值，不立即落库**；落库仍由「保存并测试连接」触发（沿用现有 `save` mutation + 连通探针）。
- **endpoint 字段对 aws/cos 是"可选高级覆盖"**：hydrate 时**不要**把库里存的推导值塞回 endpoint 输入框（保持空，把推导值作只读 hint/placeholder 展示）。否则用户改 region 后，输入框里残留的旧推导 endpoint 会让 `resolveProfile` 走"显式 endpoint"分支、不再按新 region 重导 → 落库 stale endpoint。`s3` 仍按现状 hydrate（endpoint 必填）。
- SK 字段保持"留空=保持原密钥"语义（现状 `secretKeep` 占位）。
- **改变存储指向的迁移提醒**：当该租户**已配置**存储（`configured===true`）且本次表单把**存储指向**改成了不同目标时，在「保存并测试连接」前展示**警示文案**："切换存储指向后，旧存储里的已有文件不会自动迁移到新位置，原文件将无法在本应用中访问。" 实现取最简：表单态与已加载配置 diff 出"指向已变"→在保存按钮上方渲染一条 warning 提示（不阻断），无需新建确认弹窗组件。
  - **触发项** = `provider` / `bucket` / `endpoint` 任一变化，或 `aws`/`cos` 下 `region` 变化（region 决定推导出的 endpoint，且云端 bucket 绑定 region → 等同换桶）。
  - **不触发**：`prefix`、`accessKeyId`、`secretKey` 变化——**`prefix` 改动不会让旧文件失联**，因为对象的**完整带前缀 key 已落库**（PLAN-4 gotcha；`presignGet`/`deleteObject` 用库里存的全 key，见 `s3.ts:141/150`），改 prefix 只影响新上传的 key 布局；改 AK/SK 是同一桶换凭证。

---

## 6. 关键风险与陷阱（必须正面处理）

### 6.1 ⚠️ aws-sdk v3 默认 checksum 会打断跨厂商预签名直传（头号风险）
当前安装版本 `@aws-sdk/client-s3@3.1060.0`（远高于声明的 `^3.717.0`）。自 ~3.729 起 SDK 默认 `requestChecksumCalculation:"WHEN_SUPPORTED"`，会给 `PutObject` 注入 `x-amz-checksum-crc32` 并**计入预签名 URL 的签名**；而我们的 `presignPut` 只让客户端回送 `Content-Type`（`s3.ts:133`）→ 与 MinIO / COS / 旧版 S3 兼容实现会出现 **SignatureDoesNotMatch / 直传失败**。
- **动作（Phase 0 必做）**：先验证当前本地 MinIO 直传是否仍通（PLAN-4 验过，但那时锁定版本可能 < 3.729，后续 `bun install` 已把版本抬到 3.1060）。
- **修复**：所有 `S3Client`（`buildClient` + 探针）显式设 `requestChecksumCalculation:"WHEN_REQUIRED"` + `responseChecksumValidation:"WHEN_REQUIRED"`，回退到旧行为。这是 COS/AWS 直传能成的前提，**不是可选优化**。

### 6.2 AWS region 不匹配 → `301 PermanentRedirect`
AWS 桶有归属 region，client 的 region 必须与桶一致，否则 HeadBucket/上传报 301。`probeAndPrepareBucket` 现有 catch 把非 404 一律当 `STORAGE_UNREACHABLE`——对 301 要给**专门文案**（"region 与 bucket 实际所在区域不一致"），否则用户难定位。

### 6.3 COS bucket 必带 APPID、走虚拟主机式
COS bucket 真名是 `<name>-<appid>`；region 用 COS 区域串（`ap-guangzhou` 等）；`forcePathStyle:false` → `https://<bucket>.cos.<region>.myqcloud.com`。§2.2 的 `validateProfile` 硬校验后缀给明确报错。

### 6.4 CORS 可能由云控制台托管
`PutBucketCors` 对 AWS/COS 经 S3 API 可用，但企业桶常在控制台统一配 CORS。保持现有 **best-effort 非致命**（`s3.ts:117` 已 try/catch+warn）；UI 在保存成功但 CORS 失败时不报错（现状即如此），必要时文案提示"如直传被 CORS 拦，请在云控制台放行门户域名"。

### 6.5 向后兼容
存量 `provider:"s3"` 行 + dev MinIO 流程必须零回归——`resolveProfile` 的 `s3` 分支与现逻辑逐字等价是硬性要求（§7 Phase 1 回归项）。

---

## 7. 分阶段实施 + 验证

> 每阶段「→ verify:」即成功判据（CLAUDE.md §4 Goal-Driven）。

**Phase 0 · 预检（半天）**
- 跑 `bun run files:bootstrap` + `apps/files-server` 现有 `verify-upload` / `verify-storage` 对本地 MinIO。
- → verify：现状全绿则记录基线；**若直传已挂**，先落 §6.1 checksum 兜底再继续（它本就是本期必做项，提前做）。

**Phase 1 · 后端 profile（核心，1–2 天）**
- 新增 `lib/providers.ts`；`s3.ts` 加 checksum 兜底 + provider 化探针；`modules/storage.ts` 收/存/回 `provider` + 校验推导；schema/`dto.ts` 放宽类型；bootstrap 带 `provider:"s3"`。
- → verify：`verify-storage.ts` 扩展**纯逻辑用例**（不需真云）：`resolveProfile` 三家 endpoint/forcePathStyle 推导正确、`validateProfile` 对缺 region / COS 无 appid 报 `VALIDATION_FAILED`、`s3` 分支对 MinIO **回归全绿**、`provider` 正确落库与回显。全部 workspace `tsc` 绿。

**Phase 2 · 共享类型 + 门户 API（0.5 天）**
- `StorageProvider` 导出；`files-config-api.ts` 传 `provider`。
- → verify：`tsc` 绿；门户对本地 MinIO 经 UI 存取仍正常（provider=`s3`）。

**Phase 3 · 配置 UI（1 天，强制 impeccable + Chrome）**
- 先 `impeccable` 出 provider 选择器 + 自适应表单设计（信息层级/默认值/占位/校验态/空态）。
- 实现 `FilesStorageSection` provider 化 + i18n ×N parity。
- → verify：**Chrome extension** 在应用管理→文件管理：①选 `s3` 存本地 MinIO→已连接（回归）；②切 `aws`/`cos`→表单字段/标签/占位/校验态随之变；③COS 填无 appid bucket→前/后端报清晰错；④在已配置态下改 provider/bucket→出现"数据不自动迁移"提醒，纯改 region/AK 不出现；⑤i18n 切英文文案到位。截图留档。

**Phase 4 · 真云手动冒烟（需真实凭证，由有凭证者执行后回填）**
- AWS S3：建桶→填 `aws` profile（region 正确）→保存连通→门户/微应用走一次 presign PUT→finalize→GET 下载。
- 腾讯云 COS：建桶（`<name>-<appid>`）→填 `cos` profile→同上整链路。
- → verify：两家各跑通"上传→列表→下载→删除"；故意填错 region/appid 看报错是否可懂。**结论回填到本节**。

> **结论（2026-06-13）：待有凭证者执行。** 仓库内无真实 AWS/腾讯云长期 AK/SK（§0.3b），真云连通+直传无法在本地/CI 复现。已完成的等价验证：
> - **profile 推导/校验纯逻辑**（`resolveProfile`/`validateProfile` 三家 endpoint·forcePathStyle·region·COS-APPID）→ `verify-storage.ts` 25/25 全绿。
> - **本地 MinIO 走 `s3` profile 的真实直传**（presign→PUT→finalize→GET→delete）→ `verify-upload.ts` 10/10 全绿，§6.1 checksum 兜底已设、回归无破。
> - **门户配置 UI**（provider 切换/自适应表单/COS-APPID 报错/迁移提醒/中英文）→ Chrome 真浏览器逐项验证通过（zh-CN + en）。
>
> 有 AWS/COS 凭证者执行上面两条整链路后，把"上传→列表→下载→删除"结果与故意填错 region/appid 的报错可读性回填此处即可。

---

## 8. Done 标准

- [x] 后端 `provider ∈ {s3,aws,cos}` 全链路（校验→推导→探针→落库→回显）落地，`provider` 不再硬编码。
- [x] §6.1 checksum 兜底已设，本地 MinIO 预签名直传回归绿（`verify-upload` 10/10）。
- [x] `verify-storage.ts` 新增 profile 用例全绿（25/25）；workspace `tsc` 绿（18 包）；PLAN-4 既有验证无回归。
- [x] 配置 UI 有 provider 选择器 + 自适应表单 + 改变存储指向时的"数据不自动迁移"提醒，经 impeccable 设计 + Chrome 真浏览器验证（zh-CN + en 逐项通过）。
- [x] i18n ×N parity 保持（新增 11 词条 × 3 locale）。
- [x] §7 Phase 4 真云冒烟结论已回填（标注"待有凭证者执行" + 等价验证清单）。

---

## 9. 改动文件清单（速查）

| 文件 | 类型 |
| --- | --- |
| `apps/files-server/src/lib/providers.ts` | 新增 |
| `apps/files-server/src/lib/s3.ts` | 改（checksum 兜底 + provider 化探针） |
| `apps/files-server/src/modules/storage.ts` | 改（收/存/回 provider + 校验推导，删硬编码） |
| `apps/files-server/src/db/schema.ts` | 改（`provider` 加 `$type`，无 migration） |
| `apps/files-server/scripts/{bootstrap,_helpers,verify-storage}.ts` | 改（带 provider + 新增用例） |
| `packages/shared/src/dto.ts` | 改（导出 `StorageProvider`，放宽 DTO） |
| `apps/web/src/lib/files-config-api.ts` | 改（传 provider） |
| `apps/web/src/pages/admin/FilesStorageSection.tsx` | 改（provider 选择器 + 自适应表单） |
| `apps/web/src/lib/i18n.tsx` | 改（provider 相关词条 ×N） |
