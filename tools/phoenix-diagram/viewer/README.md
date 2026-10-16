# Phoenix Diagram Reader 维护源

在仓库根用固定 Node 装配或只读核对：

```sh
mise exec --no-deps -- node scripts/assemble-phoenix-diagram-reader.mjs
mise exec --no-deps -- node scripts/assemble-phoenix-diagram-reader.mjs --check
mise exec --no-deps -- node --test test/phoenix-diagram/reader-assets.test.js
```

`template.source.html`、CSS 与 14 个 classic JavaScript 片段是维护源；`assets/template.html` 是供 Rust 嵌入的生成物。装配沿冻结脚本的顺序，Export 保持唯一的私有 Export Cleanup 子片段。`guided-views.js` 未进入参考脚本的装配列表，也未进入此运行资源。五个语言目录均保留；en/zh-CN 追加一条 PNG 下载替代提示，其余三份 catalog 保持原字节并使用现有英文 fallback。初始可读英文在装配时生成，启动后按元数据中的语言与有效覆写通过 `textContent`、安全属性更新，不将作者内容写入 HTML sink。

唯一公共运行命名空间是 `window.PhoenixDiagram`。主图保持 `.diagram-container > svg`；节点、关系与图例使用原有语义属性。原生渲染器只替换 `LOCALE/PRESET/TITLE/HEADING/SUBTITLE/SVG/CARDS/METADATA` 八个唯一 `{{PHOENIX:...}}` 插槽。原始作者事实可由 `PhoenixDiagram.metadata` 读取；它不构成已核实的仓库来源证据。

首次播放状态为 Still；只有用户选择 Live 才存入独立的 `phoenix-diagram-motion` 偏好。完整 Export 与私有 canonical clone 清理、录制单次终态资源清理保持冻结来源实现。静态资源测试只证明装配、来源保留及这些局部合同；实际浏览器布局、剪贴板、图像与 WebM 导出由 M4 的独立浏览器验收负责。

局部行为入口为 `mise exec --no-deps -- node --test tools/phoenix-diagram/tests/viewer/unit.test.mjs`。必要修补涉及零字节图片、raster 单次清理、编码器 stop 终态的 2 秒等待预算，以及复制失败/不支持时联系已有 PNG 下载动作的提示；精确差异见 `export-adaptations.json`。该入口执行实际 Reader API，DOM、canvas、Image 与编码器边界由确定性 fixture 提供；它不能证明真实浏览器权限、打印样式或媒体文件可解码。

来源、修改及许可证见 [NOTICE.md](NOTICE.md)。不要直接编辑生成物；装配器的 `--check` 不写文件、不恢复来源、不安装工具或下载资源。

`css-adaptations.json` 限定普通窄屏底部留白增加 4px，以保留已有 10px 导航间距；静态测试重建其余完整 CSS 正文。真实浏览器重验与截图审核仍由 M4 收口负责。
