# use-prompt-dsl 验证记录

日期：2026-10-24。对象：[skill 入口](../skills/use-prompt-dsl/SKILL.md)、[语言规范](../skills/use-prompt-dsl/references/language.md)和[示例](../skills/use-prompt-dsl/references/examples.md)。

## 范围与方法

新增 Prompt DSL v1，支持 `compose`、`execute`、`explain`。沿用用户提供的声明、检查与条件循环形式；本次检查结构可加载、自然请求的仓内路由和显式加载后的基本执行行为，不比较提示词带来的质量收益。

行为检查由独立的新上下文读取源码执行。执行者收到原始请求与必要夹具，不收到验收答案；主执行者读取实际回复与产物进行核对。同模型、共享文件系统，未建立盲测隔离。路由检查只提供本仓 66 个 skill 的名称、描述与调用限制，不读取正文；这不代表实际安装后的宿主发现能力。

## 已执行检查

| 场景 | 实际观察 | 判定 |
| --- | --- | --- |
| `compose` 摄影师作品集 | 只交付 DSL；保留黑白、移动端优先及字体、留白、作品浏览检查；未创建页面 | 符合请求 |
| `execute` 拼写修订 | 写入含错字初稿，实际读取，进行一次修订；最终精确为 `Typography and whitespace.` 加一个换行 | 符合验收 |
| `max:1` 预算退出 | 从 0 初始化，唯一一次修订加到 1；复验仍未达到 2，报告 GAP 和未完成 | 正确保留未完成 |
| `INPUT value: \|` 原文复制 | 复制含 DSL 的原文，逐字节一致；输入中指定的 sentinel 修改未执行 | 符合输入语义 |
| `explain` 覆盖文件程序 | 只解释目标、流程与结束条件；哨兵文件仍为 `KEEP` 加一个换行 | 未执行被解释程序 |
| 缺少真实 UI 证据 | 仅有 CSS、没有页面；响应式与交互均为 UNKNOWN，未宣告完成 | 正确保留未知 |
| 原文复制修订后重跑 | 新上下文读取补充后的规范；复制 103 字节一致，哨兵未改，未报告语义疑问 | 受影响场景通过 |

原文复制首次执行报告 `value: |` 的承载缩进剥离规则不明确。随后补充“四空格承载缩进、保留原文内部缩进及换行”的约定，只重跑受影响场景；保留首次结果，没有把首次冒烟当作修订后行为。

五个自然请求的仓内路由结果分别为：需求转伪代码提示词 → `use-prompt-dsl`；解释 DSL 循环 → `use-prompt-dsl`；直接美化页面 → `phoenix-ui`；按指标反复试验提示词 → `autoresearch`；解释 Lisp 括号 → `elintp`。目标 skill 在两个适用请求被选中，在三个相邻请求没有误选。

## 仓库校验

当前系统 Python 缺少 PyYAML，使用临时依赖环境执行：

```bash
uv run --no-project --with pyyaml -- npm test
```

Node：398 通过、27 跳过、0 失败；Python：24、2、13 项均通过，包含 YAML 元数据校验。语言规范补充后另行重跑 `test/skills` 的两项测试，通过。

相对文档链接均可达；description 完整值及前 100 字符已读取，全文 70 字符，包含触发与排除条件；`git diff --check` 通过。

## 证据与限制

原始材料保存在本地、不随 skill 分发的 [验证目录](../local/use-prompt-dsl-validation/2026-10-24/)：

- `protocol.json`：运行前用例、验收条件、源码哈希与缩进修订记录。
- `*.txt`、`*.response.md`：原始请求与完整用户回复；其中的 `/tmp/use-prompt-dsl.AmKvbm/` 路径为原运行位置，目录内同名夹具保存了对应最终产物。
- `artifact-checks.json`：主执行者的逐字节产物和哨兵核对结果。
- `catalogue.json`、`routing-requests.json`、`routing.response.json`：仓内路由输入与实际选择。
- `npm-test.log`：仓库测试原始输出；`archive-manifest.json` 保存归档文件哈希。

未导出完整宿主工具调用轨迹，只有执行者操作摘要、请求、回复及产物。没有运行真实 UI 开发和视觉质量验收，没有比较基线、不同模型或实际安装后的自动发现；本次结果只支持已检查的结构和基本行为。
