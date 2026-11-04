# Phoenix Diagram Reader 维护源

在仓库根用固定 Node 装配或只读核对：

```sh
mise exec --no-deps -- node scripts/assemble-phoenix-diagram-reader.mjs
mise exec --no-deps -- node scripts/assemble-phoenix-diagram-reader.mjs --check
mise exec --no-deps -- node --test test/phoenix-diagram/reader-assets.test.js
```

`template.source.html`、CSS 与 14 个 classic JavaScript 片段是维护源；`assets/template.html` 与 `assets/catalogs.json` 是供 Rust 嵌入的生成物，不要直接编辑。装配器按固定顺序把片段拼进模板，Export 保持唯一的私有 Export Cleanup 子片段；`--check` 不写文件，只核对生成物是否与维护源一致。

`locales/` 下的五个语言目录是唯一的目录来源，新文案直接写进各目录。装配器生成唯一的 `assets/catalogs.json`，模板嵌入同一对象，Rust `src/i18n.rs` 也读取它；`--check` 断言五目录键集合与占位符逐键一致且无 Archify 残留。初始可读英文在装配时生成，启动后按元数据中的语言与有效覆写通过 `textContent`、安全属性更新，不将作者内容写入 HTML sink。

`assets/svg-definitions.svg` 与 `assets/node-card-style.svg` 直接维护，Rust 渲染时原样嵌入。

唯一公共运行命名空间是 `window.PhoenixDiagram`。主图保持 `.diagram-container > svg`；节点、关系与图例使用原有语义属性。原生渲染器只替换 `LOCALE/PRESET/TITLE/HEADING/SUBTITLE/SVG/CARDS/METADATA` 八个唯一 `{{PHOENIX:...}}` 插槽。原始作者事实可由 `PhoenixDiagram.metadata` 读取；它不构成已核实的仓库来源证据。

首次播放状态为 Still；只有用户选择 Live 才存入独立的 `phoenix-diagram-motion` 偏好。

局部行为入口为 `mise exec --no-deps -- node --test tools/phoenix-diagram/tests/viewer/unit.test.mjs`。它执行实际 Reader API，DOM、canvas、Image 与编码器边界由确定性 fixture 提供；不能证明真实浏览器权限、打印样式或媒体文件可解码。

语义护照的来源显示按原生元数据 `sourceEvidence` 分三态；类型标签只显示作者给出的类型，SVG 根带 `data-node-kind-derived` 时 `data-node-kind` 是渲染器派生的配色槽，不作类型显示；护照自动摆放在本栏内避开聚焦节点，窄屏放不下时在图下方预留空间。窄屏（≤720px）底部工具栏完整收入舞台及护照预留空间的样式在 `template.source.html` 的 `phoenix-diagram-narrow-stage` 样式块。

来源与许可证见 [NOTICE.md](NOTICE.md)。
