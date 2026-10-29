# 品牌资源

仅在用户明确要求品牌标识时使用。品牌只补身份，不能替代组件标签、职责或 type 语义标识；不从组件名称猜品牌或官方网站。明确要求标识不等于授权联网采集；只有用户明确要求采集、并给出公共 HTTP(S) URL 时才运行 capture。

命令沿主 SKILL 中已解析的 `PD`，从用户工作目录执行。普通 validate/deliver/finalize/compare/preview 只读取固定目录或已固定的本地缓存，不联网补资源，不隐式安装或更新引擎。

## 离线查询与使用

```sh
"$PD" brands "<用户指定的品牌查询>" --json
```

省略 query 列出固定目录；查询匹配 ID、title、category、aliases、domains 的子串。返回 `catalogVersion`、`query`、`matches`、`diagnostics`，每个 match 有 `id`、`title`、`category`、`aliases`、`domains`、`provenance`。空 matches 就是无匹配，不能按相似名称换成别的品牌。

无目录匹配且用户未提供官方 URL 时，省略这项可选品牌标识，保留原职责、标签并说明未加入标识；不猜站点、不下载，也不为可选品牌扩展业务澄清。已有输入明确固定 brand 而资源缺失或变更时，按真实诊断失败，不能删除该字段来过检。

在原图对应元素的 `brand` 字段使用已核实的 canonical ID。原五类的支持位置为 architecture.components、workflow.nodes、sequence.participants、dataflow.nodes、lifecycle.states；ER、class、tree、timeline、waterfall 不扩加 brand 字段。在brand输入或capture中，已知目录域名的HTTP(S) URL可离线解析为内置品牌，仍优先使用回执给出的ID；brands查询本身只做上述子串检索。

## 明确采集官方 URL

先把 `OFFICIAL_URL` 设为用户给出的实际 URL，`BRAND_CACHE` 设为本次选择的缓存目录；不执行含占位符的命令。

```sh
"$PD" brands capture "$OFFICIAL_URL" --brand-cache "$BRAND_CACHE" --json
```

省略 `--brand-cache` 时，缓存为当前工作目录下 `.phoenix-diagram/brands/v1`。成功后把回执的 `brand` 值原样用于对应元素：

- 已知目录域名：`brand` 为字符串 ID；使用内置向量，零网络，`cachePath` 为 null。
- 其他获准 URL：`brand` 为 `{ "url": "…", "sha256": "…" }`；URL 与64位小写 SHA-256 必须一起固定，后续编译使用同一缓存目录。

`brand` 不是整个回执，不把 `resource`、`cachePath` 或 dataBase64 填入输入。未知 URL 的裸字符串不会触发采集，也不能通过删除 brand 来绕过失败。资源变更时，只在已获明确采集授权的范围内重新采集并交接新的摘要；不静默刷新。

CaptureReceipt 的公共头为 `schemaVersion/tool/toolVersion/command/status/diagnostics`，command 为 `brands capture`，status 为 pass/failed；另有 `brand`、`resource`、`cachePath`，失败时不返回已激活资源。`resource` 包含 `id/sha256/bytes/mediaType/urlSha256/sourceUrl/resolvedUrl/catalogVersion`；不适用项可为 null。公共来源和 HTML 内资源事实去掉 URL query/fragment；消费缓存仍按 capture 返回的 brand 身份，不自行改写 URL 或摘要。

网络仅读取指定 URL、受检查的重定向，以及该页 head 明确声明的 HTTP(S) 图标链接或 favicon；声明的 CDN 图标也逐请求检查，不另行发现其他品牌网站、不执行页面脚本。只收 MIME/签名/尺寸/边界相符的 PNG、JPEG、WebP、ICO；网络 SVG 不接受，可信内置向量仍走离线路径。总期限8秒，HTML head上限256KiB，图像1MiB、宽高各不超过4096；安全限制不能用私网开关、代理或随意换图绕过。userinfo、非标准端口、私有/回环/保留地址及不安全 DNS 结果均拒绝。

## 诊断与交付

| code | 处理 |
| --- | --- |
| `brand/unknown` | 核对用户指定身份和目录结果，不猜替代品牌。 |
| `brand/unpinned-url` | 未固定的非目录 URL；仅获明确授权后采集。 |
| `brand/cache-missing` | 同一 URL/摘要的安全缓存不可用；不在普通生成中联网。 |
| `brand/digest-mismatch` | URL、摘要或缓存身份不符；保留现有材料，核对原回执。 |
| `brand/blocked` | URL/DNS/IP安全边界拒绝，不绕过。 |
| `brand/capture-unavailable` | 资源或网络不可获得，报告实际缺口，不换来源。 |
| `brand/invalid-image` | MIME、字节、尺寸或边界不符，不冒充已采集成功。 |

品牌失败的退出码区分输入/策略1、归属冲突3、I/O4、资源或缓存不可获得5；参数错误为2，仍须读 diagnostics。图标占用标题轨容量，固定几何不足应修获准几何或报告诊断，不缩字、删标签或改用户 pins 过检。

继续正常 finalize、实际打开当前截图、review-record，并回读同一 summary；采集成功不代表图解或来源整体已通过。保留主 SKILL 的强事实规则、语言选择和同一失败门两次停滞即停止的规则；品牌请求不授权补造业务事实。
