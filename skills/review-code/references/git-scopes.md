# 输入范围与 Git 取证

开始时读取 `git rev-parse --show-toplevel`、`git status --short` 和 HEAD；如无 HEAD，单独处理首次提交前的工作树。命令从仓库根执行。下面尖括号是占位符，不可照抄；验证 ref 并解析成 SHA 后再使用。用户路径放在 `--` 后，正确引用空格和特殊字符；脚本收集文件名用 `-z`，不按空格拆分。

## 四类输入

| 输入 | 基线 → 目标 | 取证方式与边界 |
| --- | --- | --- |
| uncommitted，或未提交的指定文件 | HEAD → 当前工作树，另核对 index | `git diff --no-ext-diff --no-textconv HEAD --` 看 tracked 净差异；`git diff --cached` 和 `git diff` 区分 staged/unstaged；`git ls-files --others --exclude-standard -z` 枚举并读取 untracked |
| 单 commit C | C 的第一父提交 → C | 先 `git rev-list --parents -n 1 <C>`；普通提交用 `git diff <C^1> <C> --`。根提交用 `git show --root --format= --no-ext-diff --no-textconv <C> --`；merge commit 默认第一父提交并注明，用户指定其他父提交时从其要求 |
| commit 范围 A..B | A 的树 → B 的树 | `git diff <A> <B> --` 审查累计净效果，不叠加逐 commit 的中间问题；不混入当前工作树。A 是基线，不包含 A 自身引入的改动 |
| 相对基准分支 BASE | merge-base(HEAD, BASE) → 当前工作树 | 固定 BASE 和 HEAD 的 SHA，用 `git merge-base --all <HEAD> <BASE>` 确认唯一共同祖先，再 `git diff <merge-base> --`。沿用原生 review 的 tracked 工作树语义，包含 staged/unstaged；不得误写成只审 committed |

所有实际 diff/show 命令均加 `--no-ext-diff --no-textconv`，避免仓库自定义 diff 驱动执行。先用 `--name-status -M` 盘点增删改与重命名，再读取 patch 和必要全文。

### 未提交与分支模式

- staged 和 unstaged 可能相互抵消，即 `git diff HEAD` 为空而 index 非空。仍检查两层；仅存在于 index 的缺陷标注为“只提交 staged 内容时触发”，不能说当前工作树也有该缺陷。若用户只要求 staged，则基线为 HEAD、目标为 index，用 `git show :<path>` 读取完整目标文件。
- 未初始化 HEAD 时，以空树为基线，读取 staged 新文件、工作树及 untracked；不要因 `git diff HEAD` 失败判定无变更。可用 `git diff --cached` 配合文件清单和全文。
- 分支模式默认审查 tracked 工作树差异。先声明这一范围；untracked 不在该 diff 中，但应枚举，新增调用依赖它们时作为上下文读取。用户要求把新文件也纳入审查时，明确增加 untracked 范围并记录清单；未纳入的必要新文件形成覆盖限制，不能宣称完整实现已审。
- 用户明确要求“只审已提交分支变更”时，用 `git diff <merge-base> <HEAD> --`。本地 dirty 内容仅记录，不作为目标代码。用户要求分支 tip 之间直接比较时用两端树差异，明确它不同于共同祖先比较。
- untracked 按新增文件完整审查；忽略文件不自动纳入，除非用户明确点名。二进制、生成文件或子模块无法深入检查时，记录实际检查的指针/来源与缺口，不静默视为无风险。

### 范围、父提交和 ref 异常

- `A...B` 明确解释为 `merge-base(A, B) → B`，不等同 `A..B`。用户说“从 A 到 B，包含 A”时，以 `A^1 → B` 为范围；A 为根提交则以空树为基线。自然语言未说明是否包含起点且无法推断时询问。
- 单 commit 为 merge 时，第一父提交差异是默认审查对象；不要依赖可能隐藏普通变更的 combined diff。冲突解决逻辑按需与其他父提交对照，记录实际审查父提交。
- ref 验证可用 `git rev-parse --verify --end-of-options '<ref>^{commit}'`。不存在、浅克隆历史不足、无共同祖先或存在多个 merge-base 时，不擅自退回 HEAD~1 或任选基线；说明缺口并澄清/按已有授权补齐历史。
- unmerged index 要明确记录；可以审查确定部分，但 unresolved conflict 不等于完整可运行快照，不给出无保留正确结论。
- 用户限定文件时只对限定文件的变更出 findings，读取相关文件证明影响；另发现范围外线索可列覆盖建议，不静默扩大任务。

## 固定版本与行号

- 记录原始输入和解析后的完整 SHA、实际比较方式、dirty 状态、纳入文件清单；工作树/index/untracked 使用内容 hash 或等价可复查快照，不能只靠 `git status` 判断内容未变。包括影响结论的上下游文件与附带计划。
- 历史版本的代码、调用方、测试与契约从目标树读取，如 `git show <target>:<path>`、`git grep <pattern> <target> --`；基线证据从 base 读取。执行环境的当前指令仍须遵守，涉及历史仓库约定时标明所用版本。
- 不为读历史代码而 checkout、reset 或 stash 用户工作树。需要运行目标版本时用隔离临时检出或快照；确保命令在该版本执行，并记录快照路径和 SHA。
- 默认使用目标侧的短行范围；删除行/删除文件可用基线侧，必须标记 old/base 及 SHA。重命名附旧路径；index 定位明确标 staged。引用历史版本时附 `SHA:path:line`，不要把当前文件链接冒充历史行内容。
- 结束前比较固定基线与相关内容指纹。外部修改需重审受影响部分；写报告、临时验证产物不得污染本轮输入，也不能因此把用户的新改动一起排除。
