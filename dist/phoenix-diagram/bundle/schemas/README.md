# Phoenix Diagram 输入 schema

本目录是 Phoenix Diagram 的输入 schema，包含 architecture、workflow、sequence、dataflow、lifecycle、erd、class、tree、timeline、waterfall、statechart 十一类及 common 定义；`$id` 使用 Phoenix Diagram 基址，`$ref` 为相对引用。timeline 事件可选 `precision: "day" | "time"`（缺省 `time`），`day` 要求 `at` 在作者偏移下为 12:00。

各类 `meta` 只接受 schema 列出的字段，未列出的字段按 schema 错误拒绝。lifecycle 使用 v3；workflow 接受 v1/v2，其余九类为 v1。Phoenix 在 schema 之后对每类 `meta.output` 执行可移植输出路径检查。

原生输入模块先执行对应 schema，再检查实体与关系身份、引用、主路径及区间，以及各类语义门：ER 的列、外键目标、自连与网格占格，类图的种类约束与继承环，树的单根、缺父与环，时间线的时区、时间戳、泳道与日精度，耗时瀑布的计时矛盾、缺失结束与父链；statechart 的变量、参数、表达式类型、域与 view 引用（`statechart/*`）。容器各有独立身份域：合法的 lane/phase/group 可以与节点同名。通过输入闸只证明输入符合 schema 与语义规则，布局、HTML、浏览器和图像审核另行执行。

Rust registry 内嵌且离线。common 的 `portableOutputPath/allOf/2/not/pattern` 在内存中改成永不匹配的正则；Rust 字符串不能含未配对 surrogate，其他路径限制保持生效。本目录不修改该原 pattern。

品牌由固定目录或摘要固定的本地资源支持，普通编译不联网。仓库依据（`meta.repository` 与 `sources`）交付时按 `--repo-root` 核对，结构错误以 `repository/*` 在输入阶段失败。

从仓库根执行输入回归：

```sh
mise exec --no-deps -- mbx test --manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline --test input --lib
```

`tests/fixtures/input/schema-oracle.json` 是固定的回归基线，生产编译不运行 Node renderer；需要更新时直接修改并说明原因。
