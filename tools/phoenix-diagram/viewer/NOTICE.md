# Phoenix Diagram Reader 来源

Reader 派生自 Archify 3.1.0（MIT）。Copyright (c) 2026 tt-a1i (Archify)；Copyright (c) 2025 Cocoon AI。保留 [MIT 全文](LICENSE.txt)。各片段头注释写明对应的原文件与 Phoenix Diagram 的修改。

Phoenix Diagram 修改了 JavaScript、CSS、存储及产品命名空间，适配原生场景的八个模板插槽与单份元数据，加入安全的内嵌语言目录解析。Export 的 raster/share-card 使用每次调用的单一终态清理，拒绝零字节 Blob，清除 Image 回调并仅释放自己的 URL；MediaRecorder 请求 stop 后若 2 秒内没有终态事件，则以 `TimeoutError` 结束并走同一资源清理；复制不支持或失败时提示可以使用现有 PNG 下载动作。普通窄屏的导航底部留白为 `4.5rem`，保留导航与内容之间 10px 的间距。

语义护照的来源证据由原生元数据的 `sourceEvidence` 提供，按已核对、待核实（附原因）与未提供来源三态显示，只有已核对且为 web 模式的来源生成外链。`semantic-lens.js` 的外部点击判定改为“点在触发按钮内”，避免点在可切换语言的文字 span 上时立即关闭面板。`tree-branches.js` 每次更新时重新查询带祖先标记的元素，使启动后才克隆的关系点击区随分支一起折叠与展开；打印期间显示完整树，打印结束恢复阅读状态。

六份 JetBrains Mono variable WOFF2 子集保持原字节；版权与 [SIL OFL 1.1 全文](JetBrainsMono-OFL.txt) 同时保留在模板内，导出沿原序列化路径将字体及许可嵌入 SVG。未引入品牌标识数据、远程字体、更新检查或远程运行资源。

`assets/svg-definitions.svg` 派生自 Archify `renderers/shared/utils.mjs::renderDefinitions`，`assets/node-card-style.svg` 派生自 `renderers/shared/node-card-style.mjs::nodeCardStyle`；调用方仅为 architecture/workflow 启用 `data-node-ui`。生成后的 HTML 内含 MIT 全文与 OFL；消费者不需要 Node 或源码树。
