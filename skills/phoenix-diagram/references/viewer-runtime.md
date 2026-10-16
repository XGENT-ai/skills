# 阅读器运行约定

仅在用户请求 Reader 交互、样式、motion、导出或需要实际排查页面时读本文件。日常写稿按主入口和交付合同进行。

独立 HTML 包含主题与 classic 等已有风格、pan/zoom、按标签/稳定 id 搜索、焦点详情及阅读控件。切换颜色主题保留当前视觉风格，阅读交互不改作者坐标和关系。初次无保存偏好时为 Still，用户主动选 Live 才保存；reduced motion、hidden、print/export 暂停临时动作，完整静态含义持续可读。偏好属于 Phoenix 命名空间，不读取旧 Archify 偏好。

P1 的界面语言范围为 en / zh-CN，其他主要语言需明确固定 UI 与 `<html lang>` 英文 fallback。作者文本不自动翻译。保留其余 catalogs 不代表完整五语质量通过。

桌面按真实内容组织纵向节奏，无水平溢出；只接受 Reader 已声明且浏览器实测通过的页面纵向滚动。不得添加内部图解滚动、裁剪、拉伸 SVG 高度或缩字来伪造通过。控制、空搜索结果、关闭焦点及静态 SVG 均应可读可操作；实际范围与当前状态以[交付合同](delivery-contract.md)的回执和看图结果为准。

用户要求导出时，使用界面已提供的 canonical 导出动作。完整导出保留所有节点、关系、notes、图例及所选视觉风格，使用私有 SVG clone 清理相机、搜索/焦点/路线和临时播放叠层；不可把过滤后的残图交付为完整图。

- SVG 包含双主题样式与字体；PNG/JPEG/WebP 锁定当前主题/风格；WebM 保持完整静态背景，动画不改变拓扑。
- 先做 capability probe。成功需非空 Blob、真实 MIME/签名/尺寸及可解码结果；后缀不能证明格式。`failed` / `unavailable` / `cancelled` 必须保持真实终态，后来 stop/dataavailable 不得覆盖失败或下载空文件。
- PNG 复制只在真实用户手势和 Clipboard 权限允许时进行；拒绝或不支持时给出 PNG 下载途径，不伪造复制成功。
- 动作完成或失败后恢复本次探索/暂停状态，只清理自己的 URL、anchor、track、timer 和 listener。

浏览器公开命名空间为 `window.PhoenixDiagram`，不使用 Archify fallback。Route/Reach 卡片、深链接等参考代码的存在不代表 P1 已验收；不要替用户自动扩展为这些工作。需要排查或导出时保持主体静态 SVG 和全部文字可读，不能以 runtime 失败为由删去语义。
