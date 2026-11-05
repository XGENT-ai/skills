# Phoenix Diagram notices

Phoenix Diagram 的部分源码、schema 与阅读器派生自 Archify（MIT），保留以下署名：

```text
Copyright (c) 2026 tt-a1i (Archify)
Copyright (c) 2025 Cocoon AI
```

MIT 全文保存在 [viewer/LICENSE.txt](viewer/LICENSE.txt)。阅读器模板嵌入的 JetBrains Mono 字体采用 SIL Open Font License 1.1，全文保存在 [viewer/JetBrainsMono-OFL.txt](viewer/JetBrainsMono-OFL.txt)。

statechart 的图投影、区域条件、失真回退、父状态提升与报告结构派生自 pstack 的 `skills/statechart`（MIT），保留署名 `Copyright (c) 2026 Lauren Tan`；MIT 全文保存在 [assets/licenses/pstack-statechart-MIT.txt](assets/licenses/pstack-statechart-MIT.txt)。

原生文本分段模块将 [ICU 77.1 官方 release-77-1 来源](https://github.com/unicode-org/icu/tree/release-77-1/icu4c/source)中的词边界规则、字典分派、CJK 成本分段和东南亚三词前瞻算法适配为 Rust，并使用 ICU4X 2.0 代际的 Unicode 16.0 compiled dictionaries 与规范化数据。修改包括 Rust 存储、UTF-8 字节偏移和原生词典调用；原始来源文件保持未修改。12 个原始文件的官方取回 URL、逐文件 SHA-256 和修改声明见 [文本来源清单](tests/fixtures/text-segmentation/sources.json)，清单本身及许可副本的固定身份由 `scripts/check-phoenix-diagram-source.mjs` 校验，其中路径以 `tools/phoenix-diagram/` 为基准。

原生 SystemV 时间模块将该 ICU 77.1 来源的 [zoneinfo64.txt](https://github.com/unicode-org/icu/blob/release-77-1/icu4c/source/data/misc/zoneinfo64.txt) 中六个夏令时区适配为 Rust：`SystemV/AST4ADT`、`SystemV/CST6CDT`、`SystemV/EST5EDT`、`SystemV/MST7MDT`、`SystemV/PST8PDT` 和 `SystemV/YST9YDT`。修改包括归一化的 150 项历史转换表和 1977 年起的最终年度规则；原始文件保持未修改，其头部 tz 版本为 2025a。官方取回 URL、原始字节大小与 SHA-256、六区选择及规则见 [时间来源清单](tests/fixtures/format/system-v-sources.json)，其固定身份由 `scripts/check-phoenix-diagram-source.mjs` 校验；只采用这六区的数据，不替换既有 IANA 时区库。

ICU 适配及数据的 Unicode-3.0 许可和完整第三方 notices 保存在 [assets/icu77-LICENSE.txt](assets/icu77-LICENSE.txt)，该文件是官方 ICU 77.1 许可的原字节副本。开发期来源校验离线检查清单、每个原始文件及许可副本，来源缺失或字节变化时直接失败，不下载、恢复或刷新基线。这些原始开发 fixtures 不进入消费端 npm 包。

六个锁定 Rust 依赖缺少独立许可文件的情形，由 [assets/license-sources.json](assets/license-sources.json) 和 [assets/licenses/](assets/licenses/) 中四份未修改的全文补齐。清单固定 crate 版本、Cargo.lock 归档 SHA-256、repository、`.cargo_vcs_info.json` 的 commit/path、原 Cargo/README 适用性证据，以及全文的原始 URL、大小和 SHA-256；其固定身份与全文来源由 `scripts/check-phoenix-diagram-source.mjs` 校验。它们是发行构建的固定输入，普通来源检查、prepare 前置检查和 build 都不隐式下载、修补或更新这些许可。

`jsonschema-regex` / `jsonschema-value` 0.58.6 共用同一仓库提交 `55ac1664384793c5d3b20745d3f9c169dfe2c06f` 的完整 MIT 许可，保留 Dmitry Dygalo 的该提交署名。`uuid-simd` / `vsimd` 0.8.0 共用提交 `d74c030d9dc4f3cae02146d1f497ff62726ef09a` 的完整 MIT 许可，保留 Nugine 署名。`r-efi` 5.3.0 选择其 `MIT OR Apache-2.0 OR LGPL-2.1-or-later` 中的 MIT 分支，原样保留提交 `97b55bed1c2c91dcbf787674849f05337ff80b33` 的整份 `AUTHORS`，其中包含完整 MIT 许可及该版本版权；其中 Apache/LGPL 的摘要声明不计为这两个分支的完整许可。

`selectors` 0.38.0 的适用性来自 Stylo 提交 `572ecba2d1600e7c3d490586692a209faf703baa` 的 Cargo 声明及 `selectors/lib.rs` 原始 MPL-2.0 文件头；该提交未携带 MPL 全文。清单分别保留该源码文件和文件头的 SHA-256，并从其明确指向的 Mozilla 官方 MPL 2.0 取得全文：字节来自 [Mozilla Bedrock 固定提交中的版本 2.0 文本](https://github.com/mozilla/bedrock/blob/a15178c3c7c976c67b3641af77cae0b66093a175/media/MPL/2.0/index.txt)。这份通用许可正文独立于 Stylo 源码提交，不借用其他项目的 MIT/Apache 版权或宣称 Stylo 仓库自带该全文。

原生固定品牌目录派生自 Archify 3.1.0 的 `generated-brand-marks.mjs`（目录中的 `source.path`/`sha256` 记录原文件），保留 107 项的图形和原始 provenance。Simple Icons 16.28.0 集合的 CC0 许可不替代各品牌记录的单独条款、商标或其他权利。目录及完整许可来源清单见 [assets/brand-license-sources.json](assets/brand-license-sources.json)，其固定身份由 `scripts/check-phoenix-diagram-source.mjs` 校验。八份全文保留集合 CC0、JavaScript MIT 及原目录已记载的 CC0、CC-BY-4.0、CC-BY-SA-3.0、CC-BY-SA-4.0、CC-BY-NC-SA-4.0 和 Apache-2.0；原始 Archify notices 与 Simple Icons DISCLAIMER 同时保留。未记录单独许可的品牌不据此推定为 CC0，也不宣称已获特定使用授权。

构建逐项核品牌目录、许可清单、全文和 notices 的摘要。发行法律资源离线保存原目录 provenance、原始清单、全部全文与 notices；普通作图不联网补材料。

时间线 IANA 命名时区由 `chrono-tz` 0.10.4 编译期内置的 tzdata 2025b 计算，上述六个 ICU 专属 SystemV 时区使用独立原生规则；`chrono` 0.4.45 负责日期与固定偏移。两者的 MIT/Apache-2.0 全文随 Cargo.lock 从各自 crate 收集。tz 代码与数据按其 `tz/LICENSE` 属于公有领域，`chrono-tz` 的 LICENSE 同样声明其许可不适用于 tzdb 文件。
