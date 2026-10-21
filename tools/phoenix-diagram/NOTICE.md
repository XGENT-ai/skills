# Phoenix Diagram notices

Phoenix Diagram 的 Archify 参考代码、schema、JSON 语料和阅读器来源保存在 [upstream/SOURCE.json](upstream/SOURCE.json)。3.1.0 来源是用户提供的本地快照，未核对上游提交；legacy 来源固定在 XGENT-ai/skills 提交 `4a65a1525f9ed03c775a0e18935901e1edbb773f`。

Archify 源码采用 MIT License，保留以下署名：

```text
Copyright (c) 2026 tt-a1i (Archify)
Copyright (c) 2025 Cocoon AI
```

MIT 全文保存在 [upstream/LICENSE-archify-MIT.txt](upstream/LICENSE-archify-MIT.txt)，也保留在完整归档及 legacy 子集中。

Archify 阅读器模板嵌入的 JetBrains Mono 字体采用 SIL Open Font License 1.1。全文保存在 [upstream/JetBrainsMono-OFL.txt](upstream/JetBrainsMono-OFL.txt)，归档和 legacy 子集内也保留原许可。

原生文本分段模块将 [ICU 77.1 官方 release-77-1 来源](https://github.com/unicode-org/icu/tree/release-77-1/icu4c/source)中的词边界规则、字典分派、CJK 成本分段和东南亚三词前瞻算法适配为 Rust，并使用 ICU4X 2.0 代际的 Unicode 16.0 compiled dictionaries 与规范化数据。修改包括 Rust 存储、UTF-8 字节偏移和原生词典调用；原始来源文件保持未修改。12 个原始文件的官方取回 URL、逐文件 SHA-256 和修改声明见 [文本来源清单](tests/fixtures/text-segmentation/sources.json)，清单本身及许可副本的冻结身份登记在 [upstream/SOURCE.json](upstream/SOURCE.json) 的 `nativeText` 条目，其中路径以 `tools/phoenix-diagram/` 为基准。

ICU 适配及数据的 Unicode-3.0 许可和完整第三方 notices 保存在 [assets/icu77-LICENSE.txt](assets/icu77-LICENSE.txt)，该文件是官方 ICU 77.1 许可的原字节副本。开发期来源校验离线检查清单、每个原始文件及许可副本，来源缺失或字节变化时直接失败，不下载、恢复或刷新基线。这些原始开发 fixtures 不进入消费端 npm 包。

六个锁定 Rust 依赖缺少独立许可文件的情形，由 [assets/license-sources.json](assets/license-sources.json) 和 [assets/licenses/](assets/licenses/) 中四份未修改的全文补齐。清单固定 crate 版本、Cargo.lock 归档 SHA-256、repository、`.cargo_vcs_info.json` 的 commit/path、原 Cargo/README 适用性证据，以及全文的原始 URL、大小和 SHA-256；其冻结身份与全文来源登记在 `upstream/SOURCE.json` 的 `rustLicenses` 条目。它们是发行构建的固定输入，普通来源检查、prepare 前置检查和 build 都不隐式下载、修补或更新这些许可。

`jsonschema-regex` / `jsonschema-value` 0.58.6 共用同一仓库提交 `55ac1664384793c5d3b20745d3f9c169dfe2c06f` 的完整 MIT 许可，保留 Dmitry Dygalo 的该提交署名。`uuid-simd` / `vsimd` 0.8.0 共用提交 `d74c030d9dc4f3cae02146d1f497ff62726ef09a` 的完整 MIT 许可，保留 Nugine 署名。`r-efi` 5.3.0 选择其 `MIT OR Apache-2.0 OR LGPL-2.1-or-later` 中的 MIT 分支，原样保留提交 `97b55bed1c2c91dcbf787674849f05337ff80b33` 的整份 `AUTHORS`，其中包含完整 MIT 许可及该版本版权；其中 Apache/LGPL 的摘要声明不计为这两个分支的完整许可。

`selectors` 0.38.0 的适用性来自 Stylo 提交 `572ecba2d1600e7c3d490586692a209faf703baa` 的 Cargo 声明及 `selectors/lib.rs` 原始 MPL-2.0 文件头；该提交未携带 MPL 全文。清单分别保留该源码文件和文件头的 SHA-256，并从其明确指向的 Mozilla 官方 MPL 2.0 取得全文：字节来自 [Mozilla Bedrock 固定提交中的版本 2.0 文本](https://github.com/mozilla/bedrock/blob/a15178c3c7c976c67b3641af77cae0b66093a175/media/MPL/2.0/index.txt)。这份通用许可正文独立于 Stylo 源码提交，不借用其他项目的 MIT/Apache 版权或宣称 Stylo 仓库自带该全文。

完整快照和 legacy oracle 的导入闭包包含历史第三方品牌数据及对应原始 notices，供开发期参考。冻结源码独立于消费者运行资源，来源归档不进入 npm 包。P1 的运行资源不携带 Simple Icons 或官方品牌数据。

时间线时区由 `chrono-tz` 0.10.4 编译期内置的 tzdata 2025b 计算，`chrono` 0.4.45 负责日期与固定偏移；两者的 MIT/Apache-2.0 全文随 Cargo.lock 从各自 crate 收集。tz 代码与数据按其 `tz/LICENSE` 属于公有领域，`chrono-tz` 的 LICENSE 同样声明其许可不适用于 tzdb 文件。
