# 旧版迁移素材与项目 fixture

这些素材供计划 V-1 建立来源、V-4/V-5 重建旧项目使用。生产迁移器尚需另行验证认领、事务、激活和回滚；生成项目或通过这里的测试不证明迁移成功。

`migration-sources/0.3.0.json` 至 `0.6.0.json` 以真实已发布 npm tarball 为主要字节证据。记录包含 tarball 的 SHA-256、SHA-1、SHA-512 SRI、长度，安装器与 VERSION/manifest 摘要，每个 provider 的全部文件摘要与执行位，以及原始 hook 和该版安装器实际 `rewriteHookValue` 的输出。认领文件不应只检查 `exec` 清单；SKILL、reference、agent、command 等非执行文件也有摘要。

| npm 版本 | skill / engine | 归档内已验证文件 | 精确匹配迁移输入的 Git commit | Git 对照的限制 |
| --- | --- | --- | --- | --- |
| 0.3.0 | 4.3.1 / 0.1.5 | 620 | `f1e255432167e0502e1d254dc2c2c22da296625b` | 原版本候选 `7b9d026…` 的 installer/VERSION 不同，且当时是未去重 bundle，没有 manifest |
| 0.4.0 | 4.5.0 / 0.1.11 | 1045 | `47244247536e1cf20f4f4fea8b5be077a8658a2e` | 原版本候选 `54c565c…` 的 installer 不同；精确匹配迁移输入的 Git 树 package.json 仍是 0.3.0 |
| 0.5.0 | 4.5.0 / 0.1.11 | 1051 | `19999ebfc0a7eb213ed2716b649b444ef97d4bd9` | 只核对迁移输入，不声称整个 npm 发布树由此生成 |
| 0.6.0 | 4.5.0 / 0.1.11 | 1065 | `4d70f3373e465c82f00e7724ee0f229a56f69cb9` | 同上 |

Git 精确对照范围为 installer、VERSION.json、bundle manifest 和归档中的全部 bundle blobs，共分别 354、415、415、415 个文件。`provenance.git.comparison` 保留原版本候选的逐文件匹配结果；`sourceCommit` 指上述迁移输入匹配提交，不能解释为整个 tarball 的提交。采集脚本不依赖 Git 分支名或 latest。

## 准备和核验

大文件保存在忽略的 `local/phoenix-ui/`，不把整个旧 skill 复制进测试目录。准备阶段允许显式下载四个固定旧包；fixture 生成和测试阶段不联网。已有这些包时直接执行采集与测试命令。

```sh
mkdir -p local/phoenix-ui/published
npm pack @xgent-ai/skills@0.3.0 @xgent-ai/skills@0.4.0 @xgent-ai/skills@0.5.0 @xgent-ai/skills@0.6.0 --ignore-scripts --pack-destination local/phoenix-ui --json > local/phoenix-ui/published-packages.json
python3 - <<'PY'
import json, pathlib, tarfile
root = pathlib.Path('local/phoenix-ui')
for package in json.loads((root / 'published-packages.json').read_text()):
    target = root / 'published' / package['version']
    target.mkdir(parents=True, exist_ok=True)
    with tarfile.open(root / package['filename'], 'r:gz') as archive:
        archive.extractall(target, filter='data')
PY
node scripts/collect-phoenix-migration-sources.mjs local/phoenix-ui/published
```

重新采集需要本仓完整的上述 Git 对象和固定额外 skill 提交。脚本校验 npm pack 的完整性信息、归档与展开目录的文件名单和每个文件字节、全部 blob 内容地址，以及 Git 中的迁移输入。它只求值已固定安装器的 CLI dispatch 之前的声明并调用纯 `rewriteHookValue`；不运行旧安装命令、下载 engine、启动 provider 或访问用户级目录。

## 生成项目

```sh
node test/phoenix-ui/fixtures/migration/materialize.mjs local/phoenix-ui/example-migration 0.3.0 .claude,.agents claude-shared-and-local,mixed-handlers,custom-allow xgent-installer
```

目标必须是空目录。输出包含 `project with spaces/`、隔离的 `user home/`、按场景生成的 `skills-sh source/` 和目录外层的 `fixture.json`。结果中的 `projectRoot/homeRoot` 供后续测试传入；生成器不修改进程 HOME、不写真实个人配置，不创建运行进程。`fixture.json` 保存 provider、场景、最终项目文件摘要和外部/个人副本路径；它不是生产安装 receipt。

也可直接导入：

```js
import { materializeMigrationFixture } from './test/phoenix-ui/fixtures/migration/materialize.mjs';

const fixture = materializeMigrationFixture({
  fixtureRoot: '/tmp/my-empty-fixture',
  packageVersion: '0.6.0',
  providers: ['.claude', '.agents', '.cursor'],
  channel: 'xgent-installer',
  scenarios: ['additional-skills', 'symlink-skill', 'personal-shadow'],
});
```

`packagesDir` 默认 `local/phoenix-ui/published`，也可通过 `PHOENIX_MIGRATION_PACKAGES` 或函数参数指定。所有使用的包输入、hook 文件和 blob 都在生成前核对摘要；缺素材、摘要不同、未知 provider/channel、占用目录均报错。

| channel / 场景 | 实际生成行为 |
| --- | --- |
| `xgent-installer` | 18 个可选 provider 的已发布 npm 布局；展开全部 skill、直属 agents/commands，应用实际改写 hook；复制原 XGENT Claude hooks/statusline。Codex skill 在 `.agents`，hook 在 `.codex/hooks.json` |
| `bundle` | 19 个 provider 的原始 universal bundle 布局；`.codex` 的原始 skill/hook 可单独生成，`.agents` 本身没有原始 hook 文件 |
| `skills-only` | 仅模拟用户通过 skills.sh/复制安装五个辅助 skill；没有旧 runtime/hook。无需旧 npm 包，但仍核对额外 skill 摘要 |
| `claude-shared-and-local` | 两处 Claude settings 同时含旧 hook，用于验证全量发现/去重；这是迁移前遗留状态，不声称旧安装器正常新装会留下两份 |
| `mixed-handlers`、`custom-allow`、`unrelated-entry` | 在真实嵌套/平面 hook 结构中加入相邻用户 command、matcher 周边字段、allow 和未知字段；包含仅把 impeccable 当用户文字的无关 command |
| `modified-skill`、`modified-handler`、`unknown-upstream` | 真正修改 SKILL、复合 shell handler 或 launcher 字节；不只改变 recipe 标签 |
| `copied-skill`、`symlink-skill` | 独立外部旧 skill 副本；symlink 指向本 sandbox 内、项目外的来源，Windows 使用 junction |
| `additional-skills`、`modified-auxiliary`、`symlink-auxiliary` | 从固定 XGENT Git 提交读取五个辅助 skill 全部 19 个文件，生成外部复制、用户修改、符号链接及组合情形 |
| `personal-shadow` | 只在隔离 home 内生成旧主 skill 和已选额外辅助 skill 副本；沿固定 engine 的默认 user-skills 目录，含 Antigravity/Pi/OpenCode 的目录差异；不假定各 provider 的实际加载优先级 |
| `pin-shortcut`、`modified-pin` | 精确使用固定 4.5.0/0.1.11 oracle 的 `/polish`、`$polish` 和 OpenCode command 模板；修改型追加用户内容 |
| `legacy-js-hooks` | 按已发布安装器登记的五个 JS marker 生成合成 hook command。四个 tarball 没有这些 script body，fixture 不声称它们是可自动删除的已知发布文件 |
| `live-residue` | 生成有效注入 config/journal、源码 script/CSP/variants marker、未完成 session journal 及新旧 server 残留；PID 为虚构的高值、port 为 0，不启动服务 |

四版已发布 bundle 都只有一个 `impeccable` 主 skill，安装器不支持 `--with-impeccable-skills`。这里的额外辅助 skill 来源为固定 Git `b2289fa05f1e4f0251bc3a86e3c04f182aba156e`，在 `additional-skills.json` 标为用户经其他渠道安装；不能声称它们由旧 npm flag 安装。所有项目另带一个用户 skill 和原始产品/设计/agent 文档，供迁移后逐字保留检查。

## 测试与证据范围

```sh
# 无需历史大包的轻量行为测试，也随 npm test 执行。
node --test test/phoenix-ui/migration-fixtures.test.js

# 显式真实旧包验收；缺少任一旧包会失败。
PHOENIX_MIGRATION_REAL=1 node --test test/phoenix-ui/migration-fixtures.test.js
```

轻量测试用小型自建输入包验证真实目录、全部 19 provider 布局、非执行文件、JSON 混合结构、修改字节、copy/link、隔离个人副本、pin/live 和损坏输入；其内容明确是测试数据，不称为旧发布字节。显式验收另外核对四版真实 package 输入并生成 76 个 provider recipe 的全部 148 个适用 channel 组合，再验证 0.3.0/0.6.0 的真实复合场景。测试不执行 launcher/engine。

2026-10-08 本机 macOS arm64 结果：轻量 8/8；带 `PHOENIX_MIGRATION_REAL=1` 为 10/10，均零失败、零跳过。上述命令不是 Windows junction、user 路径环境变量覆盖、运行中的 live、provider 加载/信任、原 engine 启动或迁移/回滚验证；这些检查仍属于 V-4/V-5。JS body 缺口和早期版本 pin 模板是否相同需独立证据，不能从 marker 或当前 golden 推断。
