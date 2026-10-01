# 前端归档与同步验收

适用于带同源前端的应用发布，以及「manifest 已有但前端包缺失」的调查。
服务型应用没有前端，不要求 dist；有前端的应用不能用「只发 manifest/镜像」代替前端交付。

## 三处存储的区别

| 位置 | 内容 | 能证明什么 |
| --- | --- | --- |
| App 仓/CI 的持久制品存储 | 本次实际上传的原始 tgz、SHA-256 与发布记录 | 可以重传完全相同的字节 |
| 目标门户发布面 | 生效清单、解压后的前端及当前原始归档 | 当前应用的实际交付 |
| MANIFEST_STORE 的 catalog | 净化后的公开 manifest，没有前端包 | 可以查清单，不能证明可同步 |

跨平台同步源读取主平台的已生效 listing。若 TARGET_XGENT_PLATFORM 与主平台不同，
向 MANIFEST_STORE 投清单不会把 tgz 发到主平台，也不会让主平台 listing 生效。
主平台上的发布使用绑定该 App 的 xrel_ 走发布提案；xcat_ 只能读同步源，不能上传前端包。

## 先留存实际上传的包

CLI 的 `--dist dist/` 会生成临时 tgz，命令结束后删除它。预检也只生成临时包。
二者都不是持久归档。先预检 dist 目录，再显式打包并将文件交给 CLI：

```bash
set -e
# LISTING_KEY、VER 已从本次配置与版本确定；不把令牌取出写入发布记录。
RELEASE_DIR=".xgent-releases/$LISTING_KEY/$VER"
DIST_ARCHIVE="$RELEASE_DIR/dist.tgz"
mkdir -p "$RELEASE_DIR"
if test -e "$DIST_ARCHIVE"; then
  echo "已有发布包：核对记录后复用，或使用新版本；不要覆盖原归档" >&2
  exit 1
fi
tar czf "$DIST_ARCHIVE" -C dist .
gzip -t "$DIST_ARCHIVE"
shasum -a 256 "$DIST_ARCHIVE" > "$DIST_ARCHIVE.sha256"
bunx @xgent/release-cli publish --version "$VER" --dist "$DIST_ARCHIVE" \
  --manifest deploy/portal/app.manifest.json
```

发布后把目标门户、key、version、proposalId、包摘要及提交/审批结果写入项目现有发布记录，
并核对发布响应的 distDigest 与包摘要一致。pending 时保留包并报告待审，不能报告已生效。
将 `.xgent-releases/` 加入项目 gitignore；产物不提交源码仓。CI 必须将包、校验文件和发布记录
上传到项目已有的持久制品存储，记录下载位置；仅留 runner 临时目录不算归档。
该目录与存储中不放发布令牌或配置文件。

## 可同步性单独验收

主平台的同步源需要部署级 xcat_ 读取权限，App 发布用的 xrel_ 无此权限。
已有权限时只读检查；没有权限时由平台管理员核验，明确记录未验证，不自行签发令牌或改平台设置。

1. 查询 `GET /api/federation/apps/:key`，要求 Envelope.ok 为 true、data 非 null、
   syncable 为 true、version 与本次已生效版一致；带前端应用的 distDigest 必须非空且等于上传包摘要。
   列表 `/api/federation/apps` 不检查归档，不能用列表的 syncable 代替条目检查。
2. 下载 `GET /api/federation/apps/:key/dist` 的实际 tgz，拒绝 JSON 错误体；复核 SHA-256
   等于条目 distDigest 与留存包摘要，并验证压缩包完整、根下有 index.html。
3. 分别记录发布、审批/部署/网关、浏览器主路径、原始包留存、主平台可同步性结果。
   页面能开或 status 的 digest 正确，都不能代替第 1、2 项。

## 调查 DIST_ARCHIVE_MISSING

先核对当前 distDigest、发布时点、当时门户版本与包留存记录，再检查主平台本地/平台池的当前归档。
不要把这个错误直接归因于 App 团队漏传 dist：摘要存在通常说明曾上传过，可能是历史服务端删除了包。
旧版门户发布成功会删除提案归档，控制台直传也未保留原包；升级后的归档保留逻辑不会恢复已删历史包。

若多个 App 同时缺包，整理所有受影响 key、版本、摘要、发布路径与原因，核对归档保留功能的
上线时间；修正发布/验收流程，不能只修一条后宣称整个流程恢复。
已有原始包时可以提出同字节重传方案；没有原始包时，解压目录重打包会改变摘要，不能冒称恢复原归档。
