# 全新上下文复放

供维护这三个设计 skill 时使用，不是产品任务的运行步骤。准备输入、源码结构检查、仓内描述路由、显式执行和 installed-provider 接受是不同证据；未运行的层级保持「待执行」。不在当前编写会话里自评成行为通过。

## 1. 仓内描述路由

从三个 `evals/scenarios.md` 的自然请求表选一个请求，原样放入 `NATURAL_REQUEST`；预期路由留在评测侧。每个请求使用新的隔离目录。下面命令读取实际源码，记录 hash，只把三个名称/完整 description 与请求交给候选；这只测源码描述之间的区分，不代表真实安装目录的发现能力。

```bash
export SOURCE_TREE="$(git rev-parse --show-toplevel)"
export SOURCE_CASE_DIR="$(mktemp -d)"
export NATURAL_REQUEST='表格操作方式已经定了，只想改善密度、颜色和层级，帮我找局部参考。'
python3 - <<'PY'
import hashlib, json, os, re
from pathlib import Path
import yaml
source, case = Path(os.environ['SOURCE_TREE']), Path(os.environ['SOURCE_CASE_DIR'])
catalog, receipts = [], []
for name in ('design-md', 'design-reference', 'ui-pattern-research'):
    path = source / 'skills' / name / 'SKILL.md'
    body = path.read_text()
    metadata = yaml.safe_load(re.match(r'\A---\n(.*?)\n---\n', body, re.S).group(1))
    catalog.append({'name': metadata['name'], 'description': metadata['description']})
    receipts.append({'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()})
payload = {'catalog': catalog, 'request': os.environ['NATURAL_REQUEST']}
instruction = '只在给出的源码候选中选择最合适的一个；均不适用时选 other。不执行任务。返回 JSON：selected、reason。\n'
(case / 'input.txt').write_text(instruction + json.dumps(payload, ensure_ascii=False) + '\n')
(case / 'source-receipt.json').write_text(json.dumps(receipts, indent=2) + '\n')
PY
```

确认实际 runner 支持 fresh/ephemeral 启动后执行。以下是本地 `codex exec --help` 可核对的调用形状；不要用 resume/fork，也不要在请求中显式点名预期 skill。

```bash
codex exec --ephemeral --skip-git-repo-check --sandbox read-only \
  -C "$SOURCE_CASE_DIR" --json --output-last-message "$SOURCE_CASE_DIR/response.txt" \
  - < "$SOURCE_CASE_DIR/input.txt" > "$SOURCE_CASE_DIR/events.jsonl" 2> "$SOURCE_CASE_DIR/stderr.log"
run_status=$?
printf '%s\n' "$run_status" > "$SOURCE_CASE_DIR/exit-status.txt"
```

实际运行时记录 runner 版本、模型/effort、候选目录、源码 receipt、请求、退出码、原始输出和完整轨迹。逐题比对评测侧预期，选对并不证明技能执行正确。若未限制读取范围，只能标为开放材料评测，不能称盲测；候选未运行则只记录输入已准备。

## 2. 显式执行

选择对应表中的「Phoenix 执行边界」，为每题复制等价产品材料到独立可写目录：原 PRODUCT/DESIGN/brief、相关代码/接口、已有冻结目标与接受决定。缺 CLI 用例让候选可用工具中确实没有 CLI；JSON 用例提供真实公开入口的输出，若使用固定输出替身要在记录中标明。不要把应观察行为、失败信号或先前结果交给候选。

输入仅包含原始请求、正常材料位置、实际能力及「本轮直接读取 `<SOURCE_TREE>/skills/<name>/SKILL.md` 后执行」；保留该路径/hash，不能用安装缓存代替源码。复用上面的 fresh 启动形状，工作目录改为该题材料目录；读写任务使用 runner 支持的 workspace-write 沙箱。开始前记录材料 hash，结束后保存文件 diff 和工具轨迹，核对原文读取、接受来源、profile/hash/token 处理及冻结目标是否保持在本次授权内。

没有 CLI 的会话与相同原材料的 JSON 会话分别运行；不要只测试「描述中有关键词」。accepted、present、stale、带接受记录的 missing、ambiguous 分别供料，version/definition mismatch 也分开供料。原定义不存在时只验证如实报告比较缺口，不能期待工具凭候选恢复旧规则。退出成功不自动算通过；没有候选轨迹则是未执行。

## 3. 已安装 provider 接受

由集成阶段按真实候选目录重新运行原始自然请求与设计交接；记录 provider、安装路径/源码 hash、实际加载来源、请求、动作和输出。源码目录输入不能证明同名安装副本未遮蔽，显式加载也不能证明自动发现。已有七条 provider 行为路线及 V-6/V-9 仍按计划单独验收；此处命令准备与结构检查不关闭那些门槛。
