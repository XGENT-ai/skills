# Phoenix Diagram Reader 来源

Reader 移植自用户提供的 Archify 3.1.0 本地快照，来源未核对上游提交。固定内容摘要为 `4f892b0c68fef5d21788264ba6a43fe9e1d377d3c2d5b558f6aa346784238395`；原文件路径及摘要见 [source-manifest.json](source-manifest.json)，完整归档与校验入口见 [../upstream/README.md](../upstream/README.md)。

Copyright (c) 2026 tt-a1i (Archify)；Copyright (c) 2025 Cocoon AI。保留 [MIT 全文](LICENSE.txt)。Phoenix 修改了 JavaScript、CSS、存储及产品命名空间，适配原生场景的八个模板插槽与单份元数据，加入安全的内嵌语言目录解析。相机、焦点、搜索、Motion Governor、私有 canonical clone 清理及导出绘制算法保持原源码字节。

Export 的必要边界适配记录于 [export-adaptations.json](export-adaptations.json)：raster/share-card 使用每次调用的单一终态清理，拒绝零字节 Blob，清除 Image 回调并仅释放自己的 URL；MediaRecorder 请求 stop 后若 2 秒内没有终态事件，则以 `TimeoutError` 结束并走同一资源清理；复制不支持或失败时提示可以使用现有 PNG 下载动作。仅 en/zh-CN 追加 `viewer.export.downloadPngInstead`，其余目录沿现有英文 fallback，不据此宣称全语言质量通过。记录按原归档的命名空间转换结果保存精确修改片段、字节偏移与摘要；测试重建整份 Export 源码，校验这些片段之外的全部字节，未豁免整个文件。

普通窄屏的导航留白按 [css-adaptations.json](css-adaptations.json) 增加 4px：原底部留白 68px 减去导航 bottom 8px 与实际高度 54px 只剩 6px，真实 Chrome 基线未达到已有 10px 间距要求。仅将该规则的 `4.25rem` 改为 `4.5rem`；其余 CSS 正文按冻结来源、命名空间转换与单个精确片段重建核对，未降低测量阈值。

六份 JetBrains Mono variable WOFF2 子集保持来源的确切字节；版权与 [SIL OFL 1.1 全文](JetBrainsMono-OFL.txt) 同时保留在模板内，导出沿原序列化路径将字体及许可嵌入 SVG。未引入品牌标识数据、远程字体、上游更新检查或远程运行资源。

`assets/svg-definitions.svg` 来自冻结 `archify/renderers/shared/utils.mjs::renderDefinitions`，两份输入合同的定义完全相同。`assets/node-card-style.svg` 来自 3.1.0 的 `node-card-style.mjs::nodeCardStyle`；调用方仅为 Phoenix 合同的 architecture/workflow 启用 `data-node-ui`。生成后的 HTML 内含 MIT 全文与 OFL；消费者不需要 Node 或源码树。

语义护照的来源显示适配记录于 [focus-adaptations.json](focus-adaptations.json)：来源证据改由原生元数据的 `sourceEvidence` 提供，按已核对、待核实（附原因）与未提供来源三态显示，只有已核对且为 web 模式的来源生成外链。测试同样重建整份 focus.js 源码并校验这些片段之外的全部字节。

`semantic-lens.js` 的外部点击判定改为“点在触发按钮内”：装配器把按钮文字包进可切换语言的 span，真实点击落在 span 上时原判定会把刚打开的筛选面板立即关闭。

`tree-branches.js` 每次更新时重新查询带祖先标记的元素，使启动后才克隆的关系点击区随分支一起折叠与展开；打印期间显示完整树，打印结束恢复阅读状态。
