# Phoenix Diagram 输入 schema

本目录供新稿 `phoenix-v1` 合同使用，包含 architecture、workflow、sequence、dataflow、lifecycle、erd、class、tree、timeline、waterfall 十类及 common 定义。来源是固定 Archify 3.1.0 快照，来源身份与完整许可见 [NOTICE](../../../tools/phoenix-diagram/NOTICE.md)。相对参考 schema 的改动只有两处：十类 `meta.properties.input_contract` 增加常量 `phoenix-v1`；timeline 事件增加可选 `precision: "day" | "time"`（缺省 `time`），`day` 要求 `at` 在作者偏移下为 12:00。

新稿保存 `meta.input_contract: "phoenix-v1"`。原五类未标识的输入默认采用 `legacy-2.17`，其原 schema 冻结在 [upstream/legacy-2.17/schemas](../../../tools/phoenix-diagram/upstream/legacy-2.17/schemas/)；erd、class、tree、timeline、waterfall 只有 `phoenix-v1`，未标识即按它读取，显式或标识为 `legacy-2.17` 报 `input/contract-unsupported`。显式合同与持久标识冲突会失败，不能借同一个 `schema_version` 猜测来源。legacy lifecycle 使用 v1，新合同使用 v3；workflow 保留 v1/v2，其余八类为 v1。冻结 validator 在 schema 之后对每类 `meta.output` 执行可移植输出路径检查，Phoenix 在 `phoenix-v1` 输入门同样执行。

原生输入模块先执行对应 schema，再检查实体与关系身份、引用、主路径及区间，以及新五类沿参考保留诊断码的语义门：ER 的列、外键目标、自连与网格占格，类图的种类约束与继承环，树的单根、缺父与环，时间线的时区、时间戳、泳道与日精度，耗时瀑布的计时矛盾、缺失结束与父链。容器各有独立身份域：合法的 lane/phase/group/view 可以与节点同名。通过输入闸只证明输入符合合同，布局、HTML、浏览器和图像审核另行执行。

Rust registry 内嵌且离线，两份合同分别构造。新 common 的 `portableOutputPath/allOf/2/not/pattern` 在内存中改成永不匹配的正则；Rust 字符串不能含未配对 surrogate，其他路径限制保持生效。本目录与冻结源码均不修改该原 pattern。

品牌由固定目录或摘要固定的本地资源支持，普通编译不联网；旧 guided views 返回 `unsupported-display`，legacy 播放偏好返回 `ignored-display`，不覆盖首次 Still。这些终局处理列在 `compatibility.dispositions`，`deferredFields` 为空不表示旧展示能力全部复刻；原文与原始字节保留。仓库依据（`meta.repository` 与 `sources`）交付时按 `--repo-root` 核对，结构错误以 `repository/*` 在输入阶段失败。

从仓库根执行输入回归：

```sh
mise exec --no-deps -- mbx test --manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline --test input_contract --lib
```

旧、新 Node schema oracle 仅用于维护测试，生产编译不运行 Node renderer。重新生成语料可从校验后的归档解包目录读取：

```sh
node tools/phoenix-diagram/tests/generate-input-oracle.mjs --reference-root local/phoenix-diagram/reference/archify
```
