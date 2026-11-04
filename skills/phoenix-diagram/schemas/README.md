# Phoenix Diagram 输入 schema

本目录供新稿 `phoenix-v1` 合同使用，包含 architecture、workflow、sequence、dataflow、lifecycle、erd、class、tree、timeline、waterfall 十类及 common 定义。schema 派生自 Archify 3.1.0（MIT），许可署名见 [NOTICE](../../../tools/phoenix-diagram/NOTICE.md)；`$id` 使用 Phoenix Diagram 自有基址，`$ref` 为相对引用。相对原 schema 的改动：十类 `meta.properties.input_contract` 增加常量 `phoenix-v1`；timeline 事件增加可选 `precision: "day" | "time"`（缺省 `time`），`day` 要求 `at` 在作者偏移下为 12:00。

新稿保存 `meta.input_contract: "phoenix-v1"`。全部类型只有 `phoenix-v1`：原五类未标识时报 `input/contract-required`（可用 `--input-contract phoenix-v1` 显式声明），erd、class、tree、timeline、waterfall 未标识即按它读取；标识为 `legacy-2.17` 报 `input/unknown-contract`，不能借同一个 `schema_version` 猜测来源。lifecycle 使用 v3；workflow 保留 v1/v2，其余八类为 v1。Phoenix 在 schema 之后对每类 `meta.output` 执行可移植输出路径检查。

原生输入模块先执行对应 schema，再检查实体与关系身份、引用、主路径及区间，以及新五类沿参考保留诊断码的语义门：ER 的列、外键目标、自连与网格占格，类图的种类约束与继承环，树的单根、缺父与环，时间线的时区、时间戳、泳道与日精度，耗时瀑布的计时矛盾、缺失结束与父链。容器各有独立身份域：合法的 lane/phase/group/view 可以与节点同名。通过输入闸只证明输入符合合同，布局、HTML、浏览器和图像审核另行执行。

Rust registry 内嵌且离线。新 common 的 `portableOutputPath/allOf/2/not/pattern` 在内存中改成永不匹配的正则；Rust 字符串不能含未配对 surrogate，其他路径限制保持生效。本目录不修改该原 pattern。

品牌由固定目录或摘要固定的本地资源支持，普通编译不联网；旧 guided views 返回 `unsupported-display`。这些终局处理列在 `compatibility.dispositions`，`deferredFields` 为空不表示旧展示能力全部复刻；原文与原始字节保留。仓库依据（`meta.repository` 与 `sources`）交付时按 `--repo-root` 核对，结构错误以 `repository/*` 在输入阶段失败。

从仓库根执行输入回归：

```sh
mise exec --no-deps -- mbx test --manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline --test input_contract --lib
```

`tests/fixtures/input/schema-oracle.json` 是固定的回归基线，生产编译不运行 Node renderer；需要更新时直接修改并说明原因。
