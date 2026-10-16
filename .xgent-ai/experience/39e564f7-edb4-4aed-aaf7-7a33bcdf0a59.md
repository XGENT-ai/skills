# npm 默认入口修复

任务 ID：39e564f7-edb4-4aed-aaf7-7a33bcdf0a59

## E001 — 直接启动 CLI 会漏掉 npx 入口选择
- 状态：pending
- 线索：0.7.2 有两个不同 bin 且都不叫 skills，npx 无法选择入口；旧发布检查直接调用 Node 未发现。补 skills 别名后，离线打包调用与显式 xgent-skills 调用通过。
- 定位：package.json、scripts/check-phoenix-ui-release.mjs、test/npm-entrypoint.test.js

## E002 — 从 npx 验收目标项目依赖不变
- 状态：pending
- 线索：用户指出 npm install 会污染项目依赖。完整验收改走 npx 默认安装并验证 package.json、package-lock.json 与 node_modules；修复包的 Phoenix 安装和重装已通过。
- 定位：test/npm-entrypoint.test.js、scripts/check-phoenix-ui-release.mjs、local/npm-entrypoint-fix/full-npx-1792049484384/results.json

## E003 — npx 子进程可能换用另一版 Node
- 状态：pending
- 线索：用 Node 18 启动 npx 时，继承 PATH 的子进程实测为 Node 19；前置 Node 18 的 bin 后才是 18。验收需固定 PATH 并核对 npx 内的 process.execPath。
- 定位：test/npm-entrypoint.test.js、scripts/check-phoenix-ui-release.mjs

## E004 — 发布包故障需要续接补丁发布
- 状态：pending
- 线索：0.7.2 安装修复只交付源码和临时命令后，用户追问重新发布；本任务续接 0.7.3 的生成文件同步、CI 和正式发布。
- 定位：package.json、scripts/release-phoenix-ui.mjs

## E005 — 版本同步还需刷新当前能力账本摘要
- 状态：pending
- 线索：0.7.3 开发构建已同步 VERSION/ENGINE/GENERATED，但 npm test 因 capabilities.json 的旧摘要失败；按实际生成文件刷新 current.generatedBundle 后专项测试通过，历史快照保留。
- 定位：tools/phoenix-ui/capabilities.json、test/phoenix-ui/capabilities.test.js
