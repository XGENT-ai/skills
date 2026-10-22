# Phoenix Diagram 输入 schema

本目录供新稿 `phoenix-v1` 合同使用，包含 architecture、workflow、sequence、dataflow、lifecycle 五类及 common 定义。来源是固定 Archify 3.1.0 快照，来源身份与完整许可见 [NOTICE](../../../tools/phoenix-diagram/NOTICE.md)。相对参考 schema 的改动仅为五类 `meta.properties.input_contract` 增加常量 `phoenix-v1`。

新稿保存 `meta.input_contract: "phoenix-v1"`。未标识的输入默认采用 `legacy-2.17`，其原 schema 冻结在 [upstream/legacy-2.17/schemas](../../../tools/phoenix-diagram/upstream/legacy-2.17/schemas/)。显式合同与持久标识冲突会失败，不能借同一个 `schema_version` 猜测来源。legacy lifecycle 使用 v1，新合同使用 v3；workflow 保留 v1/v2，其他三类为 v1。

原生输入模块先执行对应 schema，再检查实体与关系身份、引用、主路径及区间。容器各有独立身份域：合法的 lane/phase/group/view 可以与节点同名。通过输入闸只证明输入符合合同，布局、HTML、浏览器和图像审核另行执行。

Rust registry 内嵌且离线，两份合同分别构造。新 common 的 `portableOutputPath/allOf/2/not/pattern` 在内存中改成永不匹配的正则；Rust 字符串不能含未配对 surrogate，其他路径限制保持生效。本目录与冻结源码均不修改该原 pattern。

品牌、仓库依据、guided views 和旧播放偏好按字段返回延期处置，原文与原始字节保留。schema 中保留这些字段不表示 P1 已交付其运行能力。

从仓库根执行输入回归：

```sh
mise exec --no-deps -- mbx test --manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline --test input_contract --lib
```

旧、新 Node schema oracle 仅用于维护测试，生产编译不运行 Node renderer。重新生成语料可从校验后的归档解包目录读取：

```sh
node tools/phoenix-diagram/tests/generate-input-oracle.mjs --reference-root local/phoenix-diagram/reference/archify
```
