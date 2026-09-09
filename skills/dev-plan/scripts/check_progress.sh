#!/usr/bin/env bash
# 用法: check_progress.sh <计划.md> [仓库根,默认 .]
# 检查计划顶部「实施进度」的内部一致性 —— 实施期每回写一次跑一次,写计划时也跑一次(应报 0/N)。
# 它查得出「回写了但对不上」(n/N 与里程碑数不符、最近完成与完成记录不符、证据或基线留空、
# 模板占位残留);查不出「压根没回写」—— 那个看末尾的陈旧提示,以及最终由人判断。
set -euo pipefail
plan="${1:?用法: check_progress.sh <计划.md> [仓库根]}"; root="${2:-.}"
[ -f "$plan" ] || { echo "ERROR: 计划文件不存在: $plan" >&2; exit 1; }

errs=0; warns=0
err()  { echo "ERROR: $*"; errs=$((errs + 1)); }
warn() { echo "WARN:  $*"; warns=$((warns + 1)); }
trim() { printf '%s' "$1" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//'; }

# 取出一节:从标题行的下一行起,到下一个同级或更高级标题为止。
section() {
  awk -v re="$1" '
    /^#+[ \t]/ {
      lvl = length($1)
      if (inseg && lvl <= start) exit
      if (!inseg && $0 ~ re) { inseg = 1; start = lvl; next }
    }
    inseg { print }
  ' "$2"
}
# 表中以 M<数字> 开头的行(「评审整改」这类非里程碑行不计入)
mrows() { grep -E '^\|[[:space:]]*\**M[0-9]+' <<<"$1" || true; }

prog="$(section '^#+[ \t].*实施进度' "$plan")"
snap="$(section '^#+[ \t].*恢复快照' "$plan")"
recs="$(section '^#+[ \t].*完成记录' "$plan")"
mile="$(section '^#+[ \t].*里程碑' "$plan")"

[ -n "$prog" ] || err "找不到「实施进度」节 —— 跨对话恢复的唯一入口缺失"
[ -n "$snap" ] || err "找不到「恢复快照」"
[ -n "$recs" ] || err "找不到「完成记录」"
[ -n "$mile" ] || err "找不到里程碑表"

# ① 快照七个字段齐全
for f in 最近更新 当前进度 当前状态 最近完成 下一步 当前阻塞 代码基线; do
  grep -q "$f" <<<"$snap" || err "恢复快照缺字段:$f"
done

# ② 快照里的模板占位没填(骨架的 <...> 应当被真实内容替换)。
#    只查快照:完成记录的证据正文里出现 <img> / <select> 这类尖括号是正常的。先剥掉行内代码。
while IFS= read -r l; do
  [ -n "$l" ] && err "恢复快照残留模板占位:$(trim "$l")"
done < <(sed 's/`[^`]*`//g' <<<"$snap" | grep -E '<[^<>]+>' || true)

# ③ n/N 与两张表对齐
mcount="$(mrows "$mile" | grep -c . || true)"
rcount="$(mrows "$recs" | grep -c . || true)"
cur="$(grep -m1 '当前进度' <<<"$snap" || true)"
done_n=""; total_n=""
if [[ "$cur" =~ ([0-9]+)[[:space:]]*/[[:space:]]*([0-9]+) ]]; then
  done_n="${BASH_REMATCH[1]}"; total_n="${BASH_REMATCH[2]}"
else
  err "「当前进度」不是 n/N 形状,新对话读不出进度:$(trim "$cur")"
fi
if [ -n "$total_n" ]; then
  [ "$mcount" -gt 0 ] || err "里程碑表里没有 M<数字> 行,无法核对 N"
  [ "$mcount" -eq 0 ] || [ "$total_n" -eq "$mcount" ] \
    || err "「当前进度」的 N=$total_n,里程碑表却有 $mcount 行 —— 计划改过里程碑但没同步快照"
  [ "$done_n" -eq "$rcount" ] \
    || err "「当前进度」说完成 $done_n 个,完成记录却有 $rcount 行里程碑 —— 少记或多记了一行"
fi

# ④ 最近完成 与 完成记录 对齐
lastdone="$(trim "$(grep -m1 '最近完成' <<<"$snap" | sed 's/.*最近完成[：:]//')")"
if [ "${done_n:-0}" = "0" ]; then
  case "$lastdone" in 无|—|-|"") ;; *) warn "0 个里程碑完成,但「最近完成」写着:$lastdone" ;; esac
else
  grep -q '尚未完成任何里程碑' <<<"$recs" \
    && err "已完成 $done_n 个里程碑,完成记录却还留着「尚未完成任何里程碑」占位行"
  case "$lastdone" in 无|—|-|"") err "已完成 $done_n 个里程碑,「最近完成」却写「$lastdone」" ;; esac
fi

# ⑤ 每行完成记录都要有可复核的证据与基线
while IFS= read -r row; do
  [ -n "$row" ] || continue
  IFS='|' read -r _ c_m c_t c_s c_v c_b _rest <<<"$row"
  m="$(trim "${c_m:-}")"
  for pair in "完成时间:${c_t:-}" "完成摘要:${c_s:-}" "验证证据:${c_v:-}" "代码基线:${c_b:-}"; do
    name="${pair%%:*}"; val="$(trim "${pair#*:}")"
    case "$val" in ""|—|-|待补|TBD|N/A) err "完成记录 $m 的「$name」是空的($val) —— 未通过退出条件就不该记完成" ;; esac
  done
done < <(mrows "$recs")

# ⑥ 陈旧提示:代码改了、计划没动
if command -v git >/dev/null 2>&1 && git -C "$root" rev-parse --git-dir >/dev/null 2>&1; then
  planrel="$(git -C "$root" ls-files --full-name --error-unmatch "$plan" 2>/dev/null || true)"
  dirty="$(git -C "$root" status --porcelain 2>/dev/null | cut -c4- | sed 's/.* -> //' || true)"
  if [ -n "$dirty" ]; then
    n="$(grep -c . <<<"$dirty" || true)"
    if [ -n "$planrel" ] && grep -qxF "$planrel" <<<"$dirty"; then :
    elif [ -n "$done_n" ] && [ -n "$total_n" ] && [ "$done_n" -lt "$total_n" ]; then
      warn "还剩 $((total_n - done_n)) 个里程碑未完成,工作树有 $n 个未提交改动,而本计划文件没被改过 —— 若其中已完成某个里程碑,先回写「实施进度」再继续"
    fi
  fi
fi

echo "---"
echo "进度:${done_n:-?}/${total_n:-?} · 里程碑表 $mcount 行 · 完成记录 $rcount 行 · ERROR $errs · WARN $warns"
[ "$errs" -eq 0 ] || exit 1
