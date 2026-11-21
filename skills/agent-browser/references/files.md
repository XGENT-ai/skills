# 文件与剪贴板：让交付物到达指定位置

先明确用户期待的结果。选择了输入文件、网页完成了导入、导出的文件能在指定位置打开，是三个不同的验收点。下文命令省略 `agent-browser` 前缀；所有网页操作都沿用目标会话。

```formal
RULE @upload_result:
  WHEN: website_import_requested
  MUST: verify_site_receipt_for_expected_files
RULE @download_trigger:
  WHEN: download_already_triggered
  MUST: observe_existing_download
  MUST_NOT: click_export_again_while_status_unknown
RULE @artifact_delivery:
  WHEN: local_file_requested
  MUST: [identify_storage_owner, confirm_readable_final_file]
RULE @clipboard_target:
  WHEN: clipboard_transfer_requested
  MUST: [confirm_selection_or_focus, verify_destination_content]
```

## 上传：从输入控件到业务回执

1. 用 `snapshot -i` 找到文件输入框。页面若有“选择文件”装饰按钮，确认实际目标是 `input[type=file]`；同页有多个上传区时，在指定表单内查找。
2. 单文件示例为 `upload @e4 ./orders.csv`。支持多选的输入框可用 `upload @e4 ./front.png ./back.png` 一次选择两份文件。路径带空格时分别加引号。
3. 若用户仅要求选文件，按页面所列文件名和数量核对。若要求导入订单，继续执行已授权的导入步骤，核对行数、失败明细或对应批次回执。页面已经显示成功时，无需再次提交。

## 下载只触发一次

| 当前状态 | 本地由 CLI 启动的浏览器 | CDP、自动连接或云浏览器 |
| --- | --- | --- |
| 已找到导出控件，还没有点击 | `download @e7 ./orders.csv` 会点击并保存 | 点击一次目标控件，再用无路径的 `wait --download` 查看浏览器报告的位置 |
| 前一步已经触发了下载 | `wait --download ./orders.csv --timeout 30000` 等待并收取结果 | 用 `wait --download` 收取结果。端点决定文件名；带目标路径的下载命令可能被拒绝 |
| 需要本地交付 | 在**关闭浏览器前**打开指定文件核对类型与业务内容。例如用 CSV 解析器确认表头和所选订单 | 检查报告路径是否能由当前机器读取。若文件只在远端，通过该服务已有的取件方式交付；未取回时报告远端位置和缺少的本地文件 |

本地浏览器可在启动时设置 `--download-path <目录>`。默认临时目录可能随会话关闭而清理。若等待超时，先确认站点是在生成导出任务、显示错误，还是已经开始下载；再次点击可能创建第二个导出任务。

## 截图、PDF 与本地预览

| 要交付什么 | 示例 | 应核对什么 |
| --- | --- | --- |
| 当前视口或完整长页 | `screenshot review.png`、`screenshot --full review.png` | 目标结果已显示，遮罩已消失；打开生成的图像检查范围。需要定位图标时见[标注截图](snapshots.md#iframe无名图标与视觉内容) |
| 重复观察页面，只保存变化 | 版本支持时用 `screenshot --if-changed --json`；`--threshold 0.01` 将不超过 1% 的像素变化视为未变 | 第一次会建立基线，后续查看 `changed`、`revision`、`pixelChangeRatio` 和实际返回的路径。未变时可能没有新路径，应引用之前已保存的文件 |
| 固定版式文档 | `pdf summary.pdf` | 打开 PDF 或读取其页数与目标内容。页面的打印样式可能改变分页，PNG 看起来正常不足以证明 PDF 完整 |
| 本机网页或 PDF | 用本地 Chromium 打开相应的 `file://` 地址。可信本地页确需通过 fetch/XHR 读取相邻资源时，可在该专用会话启动时设置 `--allow-file-access` | 文件必须位于浏览器能访问的位置；远端浏览器的文件系统与调用端可能不同 |

截图格式可设为 `--screenshot-format jpeg` 并指定 `--screenshot-quality 80`。在支持选项终止符的版本中，若文件名以短横线开头，可用 `pdf -- --draft.pdf`，或用明确的相对路径 `pdf ./--draft.pdf`。

**本地文件显示为空时，先查当前页。** 在 macOS 的一次对照中，`0.33.2` 用 `--allow-file-access open` 打开了 HTML，随后省略这个启动参数时，`get url` 返回了 `about:blank`。同一会话继续带 `--allow-file-access snapshot` 可以读到页面标题。`0.43.0` 在该示例中能保留设置，也读到了 HTML 通过 fetch 加载的相邻文本。导出之前先确认标题或正文，可以避免交付一个格式正确、内容空白的 PDF。

## 剪贴板：选区、焦点、结果

- **写入指定文本**：`clipboard write "订单 B"` 会更改剪贴板。目标只是填写一个输入框时，`fill @eN "订单 B"` 可以直接完成。
- **验证复制按钮**：触发用户指定的复制操作后，用 `clipboard read` 检查复制内容。使用 `clipboard copy` 时先建立目标选区，该命令模拟平台复制快捷键。
- **验证粘贴行为**：先 `focus @eN`，再 `clipboard paste`，最后用 `get value @eN` 或编辑器的结果快照核对。读取的是浏览器当前上下文的剪贴板；远程会话中应在实际目标页面验证。

来源：[文件与剪贴板](https://agent-browser.dev/files)。
