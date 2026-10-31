# 输入与展示兼容

导入已有 JSON 或解释兼容回执时读取。本约定不授权改写原输入、迁移安装或替用户补事实。

| 输入 | 合同 |
| --- | --- |
| 新稿，全部十类 | 明确写 `meta.input_contract: "phoenix-v1"` |
| 未标记的 architecture、workflow、sequence、dataflow、lifecycle 旧稿 | `legacy-2.17`，保留原字节及默认几何 |
| 已知来自 Archify 3.1 的原五类稿件 | 显式传 `--input-contract phoenix-v1`，不为导入改写原文件 |
| erd、class、tree、timeline、waterfall | 只有 `phoenix-v1` |

显式合同、质量参数与输入已有声明冲突时会拒绝；不能通过覆盖参数消除已声明的语义。五份 `examples/legacy.*.json` 保留固定 2.17 原例字节，只供兼容输入使用，不是新稿默认值。

读取 deliver 阶段的 `receipt.compatibility`，不要仅看退出码：

- `coreCompatibility` 表示核心输入兼容结果，不能证明旧 Reader 的全部展示能力已复刻。
- `dispositions` 按 JSON pointer 给出 `path`、`disposition`、`reason`。`brand` 为 `supported`，由固定目录或摘要固定的本地资源解析；缺失资源仍须按实际诊断处理。
- `/meta/views` 为 `unsupported-display`：旧 Guided Views 不重建，当前 Reader 提供导航。
- `legacy-2.17` 的 `/meta/animation` 为 `ignored-display`：旧播放偏好不覆盖首次 Still。
- 有非 `supported` 项时 `status` 为 `partial`；这些是终局处理，`deferredFields` 为空，不再承诺某个后续阶段补齐。

在交付答复中逐项说明非 `supported` 字段及原因。保留原 JSON 和已有独立 HTML，不把不支持的展示改成卡片、删掉必要事实或宣称全量兼容。仓库来源核对、时间证据及语言回退仍分别按各自回执交接。
