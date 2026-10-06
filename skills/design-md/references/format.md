# 文档格式与验证

先满足项目已采用的格式。`DESIGN.md` 这个文件名不意味着一定使用某个外部 schema，也不意味着可以重排已有文档。

## 使用 Google DESIGN.md 格式时

参考 [官方规范](https://github.com/google-labs-code/design.md/blob/main/docs/spec.md) 和 [CLI 文档](https://github.com/google-labs-code/design.md/blob/main/README.md)。下列是编写要点，运行时仍以所用版本为准：

- YAML 放机器可读的规范值，正文说明用途和应用方式。值不存在时不为了填模板而创建。
- `colors`、`rounded`、`spacing` 按规范组织命名值；`typography` 中每个命名样式是属性映射，常用 `fontFamily`、`fontSize`、`fontWeight`、`lineHeight`、`letterSpacing`。
- 使用 `{path.to.token}` 引用时检查目标存在并避免循环；规范中的复合值支持范围与目标导出格式都要核对。
- 保留原单位；将工具类换成具体值前先解析项目真实配置，不能假设框架默认值。
- 所用版本无法表达主题、响应式或特定规则时，在对应正文中保留已知信息并说明导出局限；不要删信息或自行发明看似受支持的 schema。

不复制外部站点的整份设计系统来填项目空缺。重建的角色、原站实测值和用户接受的新决策需保持可区分。

## 检查与可选导出

优先使用项目已有版本和命令。无现成 CLI、但任务确需格式检查时，可以临时调用官方工具，不添加产品依赖或修改锁文件；记录实际版本。以下为官方命令形状，文件路径按任务替换：

```bash
npx @google/design.md spec
npx @google/design.md lint DESIGN.md
npx @google/design.md export --format css-tailwind DESIGN.md
```

按目标选导出：Tailwind v4 为 `css-tailwind`，v3 为 `json-tailwind`，DTCG 为 `dtcg`。仅需 Markdown 时，不强制安装或执行导出工具。

结构和语义分开核验：lint 报告中的错误、警告需逐项判定；导出还要查看输入中被目标格式支持的颜色、字体、尺寸等是否实际出现，值、引用和模式是否保留。缺项不能因退出码为零就算通过；不支持的模式不能称为已导出。临时导出用于核验，只有用户要求交付时才保存到项目。

工具不可用时仍可交付有来源的文档或草稿，说明哪些结构检查已完成、哪些兼容性未验证。不要通过删有效内容来消除工具报错。

## 方法来源

[ibelick/create-design-md](https://github.com/ibelick/ui-skills/blob/main/skills/create-design-md/SKILL.md)提供了从仓库与 URL 分别取证、验证输出的参考。本技能按本仓需求独立编写，保留项目自有格式，区分既定规则与实现偏差，不把外部 schema 或工具设为所有任务的前提。
