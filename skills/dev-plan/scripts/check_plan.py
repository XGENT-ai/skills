#!/usr/bin/env python3
"""检查默认骨架的核心表、ID 定义/引用和模板残留；进度与路径由已有脚本检查。

用法：python3 check_plan.py <计划.md>。仅依赖标准库，ERROR 返回 1，WARN 需人工复核。
自定义模板/编号须人工核对同等规则；结构通过不证明设计、来源或验收真实有效。
识别 R/NFR/C/A/V/T-数字、M数字及账本登记的同形上游前缀；代码块、路径与链接
目标不参与 ID 校验。正文和行内代码中的已知骨架占位报错，自定义疑似占位告警。
"""

import re
import sys
from pathlib import Path


ID = re.compile(r"(?:(?:[A-Z][A-Z0-9]*-)+\d+|M\d+)")
EMPTY = {"", "—", "-", "TBD", "TODO", "待补", "待填写"}
SCHEMAS = {
    "账本": ("ID", ("ID", "类型", "来源", "内容", "状态")),
    "检查": ("检查 ID", ("检查 ID", "需求/验收行与可观察结果", "命令/入口与环境", "执行时点")),
    "里程碑": ("#", ("#", "里程碑", "前置依赖", "内容与并行边界", "验证/退出条件")),
    "任务": ("任务 ID", ("任务 ID", "所属里程碑", "交付物/影响路径", "前置成果", "检查 ID")),
}


def visible(text):
    # 链接文字参与校验，目标路径中的编号不参与。
    return re.sub(r"\[([^\]]+)\]\([^)]*\)", r"\1", text)


def plain(text):
    return re.sub(r"[*`]", "", visible(text)).strip()


def prose_lines(text):
    fence = None
    for number, line in enumerate(text.splitlines(), 1):
        match = re.match(r"^\s*(`{3,}|~{3,})(.*)$", line)
        if match:
            marker, suffix = match.groups()
            if fence is None:
                fence = marker
            elif marker[0] == fence[0] and len(marker) >= len(fence) and not suffix.strip():
                fence = None
            continue
        if fence is None:
            yield number, line


def cells(line):
    return [plain(cell) for cell in re.split(r"(?<!\\)\|", line.strip().strip("|"))]


def tables(lines):
    for index, (number, line) in enumerate(lines[:-1]):
        if not line.strip().startswith("|"):
            continue
        separator = cells(lines[index + 1][1])
        if not separator or not all(re.fullmatch(r":?-{3,}:?", cell) for cell in separator):
            continue
        header = cells(line)
        rows = []
        for row_number, row in lines[index + 2:]:
            if not row.strip().startswith("|"):
                break
            rows.append((row_number, cells(row)))
        yield number, header, rows


def check(text):
    lines = list(prose_lines(text))
    findings = []
    definitions = {}
    found = set()

    def report(level, number, message):
        findings.append((level, number, message))

    for number, header, rows in tables(lines):
        if "任务 ID" in header:
            kind = "任务"
        elif "执行时点" in header or "命令/入口与环境" in header:
            kind = "检查"
        elif "里程碑" in header:
            kind = "里程碑"
        elif "类型" in header and "来源" in header:
            kind = "账本"
        else:
            continue  # NFR 展开、映射和进度记录等表不声明 ID。
        found.add(kind)
        id_column, required = SCHEMAS[kind]
        missing = set(required) - set(header)
        if missing:
            report("ERROR", number, f"{kind}表缺少字段：{', '.join(sorted(missing))}")
            continue
        if not rows:
            report("ERROR", number, f"{kind}表没有条目；不需要任务表时删除整节")
        for row_number, row in rows:
            if len(row) != len(header):
                report("ERROR", row_number, f"{kind}表列数不一致（内容中的 | 须转义）")
                continue
            values = dict(zip(header, row))
            for column in required:
                value = values[column]
                if column in {"前置依赖", "前置成果"} and value in {"—", "-"}:
                    continue
                if value in EMPTY:
                    report("ERROR", row_number, f"{kind}表字段未填：{column}")
            identifier = values[id_column]
            if not identifier or identifier in {"—", "-"}:
                continue
            pattern = {"检查": r"V-\d+", "里程碑": r"M\d+", "任务": r"T-\d+"}.get(kind)
            if pattern and not re.fullmatch(pattern, identifier):
                report("ERROR", row_number, f"{kind} ID 格式无效：{identifier}")
            elif not ID.fullmatch(identifier):
                report("WARN", row_number, f"自定义 ID 须人工核对引用：{identifier}")
            if identifier in definitions:
                report("ERROR", row_number, f"ID 重复定义：{identifier}（首次在第 {definitions[identifier]} 行）")
            else:
                definitions[identifier] = row_number
            if kind == "任务" and not re.fullmatch(r"M\d+", values["所属里程碑"]):
                report("ERROR", row_number, "任务须指定一个所属里程碑 ID")
            if kind == "任务" and not re.search(r"\bV-\d+\b", values["检查 ID"]):
                report("ERROR", row_number, "任务须引用验证表中的检查 ID")

    for kind in ("账本", "检查", "里程碑"):
        if kind not in found:
            report("ERROR", 0, f"缺少{kind}定义表；自定义模板请人工核对同等规则")

    prefixes = {"R", "NFR", "C", "A", "V", "T"}
    prefixes.update(value.rsplit("-", 1)[0] for value in definitions if "-" in value and ID.fullmatch(value))
    id_token = r"(?:M\d+|(?:" + "|".join(re.escape(p) for p in sorted(prefixes)) + r")-\d+)"
    left, right = r"(?<![A-Za-z0-9_./\\-])", r"(?![A-Za-z0-9_./\\-])"
    reference = re.compile(left + id_token + right)
    slash_list = re.compile(left + id_token + r"(?:/" + id_token + r")+" + right)
    skeleton = Path(__file__).resolve().parent.parent / "references/plan-skeleton.md"
    placeholders = set(re.findall(r"<[^<>\n]+>", skeleton.read_text(encoding="utf-8")))
    fields = {}
    for number, line in lines:
        # 行内代码只有整个片段为 ID 时才参与引用扫描；普通路径、命令不参与。
        text_line = visible(line)
        references = re.sub(r"(`+)(.*?)\1", lambda m: m[2] if ID.fullmatch(m[2]) else "", text_line)
        references = slash_list.sub(lambda m: m[0].replace("/", "、"), references)
        for match in reference.finditer(references):
            if match[0] not in definitions:
                report("ERROR", number, f"ID 引用未定义或未在账本登记来源：{match[0]}")
        for token in set(re.findall(r"<[^<>\n]+>", text_line)):
            if token in placeholders:
                # 协议中的泛型路径说明是合法行内代码，不是待填字段。
                if token in {"<计划名>", "<n>"} and token not in references:
                    continue
                report("ERROR", number, f"残留骨架占位/写作指引：{token}")
            elif token in references and re.search(r"[\u4e00-\u9fff]", token):
                report("WARN", number, f"疑似自定义占位，请人工核实：{token}")
        if re.search(r"\b(?:TBD|TODO)\b", references):
            report("WARN", number, "疑似未填内容 TBD/TODO，请人工核实")
        field = re.match(r"^(?:>\s*)?(?:-\s*)?(计划状态|调查基线)[：:]\s*(.*)$", plain(line))
        if field:
            fields[field[1]] = (number, field[2].rstrip("。"))
    for field in ("计划状态", "调查基线"):
        number, value = fields.get(field, (0, ""))
        if value in EMPTY:
            report("ERROR", number, f"缺少或未填：{field}")
        elif field == "计划状态" and value not in {"Ready", "Proposed", "Blocked"}:
            report("ERROR", number, f"计划状态无效：{value}")
    return findings


def main():
    if len(sys.argv) != 2:
        print("用法：python3 check_plan.py <计划.md>", file=sys.stderr)
        return 1
    plan = Path(sys.argv[1])
    try:
        findings = check(plan.read_text(encoding="utf-8"))
    except OSError as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    for level, number, message in findings:
        print(f"{level}: {plan}:{number}: {message}")
    errors = sum(level == "ERROR" for level, _, _ in findings)
    warnings = len(findings) - errors
    print(f"计划结构与引用：ERROR {errors} · WARN {warnings}（不证明设计或验收有效）")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
