#!/usr/bin/env python3
"""Regenerate unrounded source SVG fixtures using pinned Node only.

The product resolves quality as CompileOptions.quality, then authored metadata,
then showcase. Project that effective value into a temporary source-renderer
input; retain the original authored input in the corpus unchanged.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
from html.parser import HTMLParser

ROOT = Path(__file__).resolve().parents[5]
DESTINATION = Path(__file__).with_name("oracle.json")
CONTRACTS = {
    "legacy-2.17": ROOT / "tools/phoenix-diagram/upstream/legacy-2.17",
    "phoenix-v1": ROOT / "local/phoenix-diagram/reference/archify",
}


def rect(attributes):
    return {key: float(attributes[key]) for key in ("x", "y", "width", "height")}


class SvgGeometry(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.stack = []
        self.section = ""
        self.result = {"participants": [], "messages": [], "segments": [], "activations": [], "lifelines": [], "legends": []}
        self.active_text = None
        self.active_span = None

    def handle_comment(self, data):
        self.section = data.strip()

    def handle_starttag(self, tag, attributes):
        a = dict(attributes)
        context = self.stack[-1][1].copy() if self.stack else {}
        if tag == "svg":
            self.result["viewBox"] = [float(value) for value in a["viewbox"].split()]
            self.result["readerFit"] = a.get("data-reader-fit")
            self.result["columnFit"] = a.get("data-sequence-column-fit")
        if tag == "g":
            if "data-node-id" in a:
                context = {"node": len(self.result["participants"])}
                self.result["participants"].append({"id": a["data-node-id"], "texts": []})
            if "data-edge-key" in a:
                context = {"edge": int(a["data-edge-key"])}
                while len(self.result["messages"]) <= context["edge"]:
                    self.result["messages"].append({"texts": [], "noteTexts": [], "backdrop": None})
            if "data-segment-id" in a:
                context = {"segment": int(a["data-segment-id"])}
            if "data-legend" in a:
                context = {"legend": None}
            if "data-legend-semantic-kind" in a:
                context = {"legend": len(self.result["legends"])}
                self.result["legends"].append({"kind": a["data-legend-semantic-kind"], "x": float(a["data-legend-x"]), "baseline": float(a["data-legend-baseline"]), "width": float(a["data-legend-width"]), "texts": []})
        if tag == "rect":
            if "node" in context and "bounds" not in self.result["participants"][context["node"]]:
                self.result["participants"][context["node"]]["bounds"] = rect(a)
            if a.get("data-graph-role") == "structural-frame":
                index = int(a["data-composition-frame-id"])
                while len(self.result["segments"]) <= index:
                    self.result["segments"].append({"texts": []})
                self.result["segments"][index]["bounds"] = rect(a)
            if "segment" in context:
                self.result["segments"][context["segment"]]["labelPlate"] = rect(a)
            if "edge" in context:
                message = self.result["messages"][context["edge"]]
                if a.get("data-graph-role") == "message-note-background":
                    message["backdrop"] = rect(a)
                elif "labelPlate" not in message:
                    message["labelPlate"] = rect(a)
            if self.section == "Activations" and not context and a.get("class") == "c-mask":
                self.result["activations"].append(rect(a))
        if tag == "path":
            if "data-composition-points" in a:
                self.result["messages"][context["edge"]]["points"] = [[float(value) for value in point.split(",")] for point in a["data-composition-points"].split(";")]
            if a.get("stroke-dasharray") == "3,7" and a.get("stroke-width") == "0.8":
                values = [float(value) for value in re.findall(r"[-+]?(?:\d+(?:\.\d*)?|\.\d+)(?:e[-+]?\d+)?", a["d"], re.I)]
                self.result["lifelines"].append([values[:2], values[2:]])
        if tag == "text":
            self.active_text = {"content": "", "position": [float(a["x"]), float(a["y"])], "fontSize": float(a["font-size"]), "anchor": a.get("text-anchor", "start"), "owner": context.copy(), "lines": []}
        if tag == "tspan" and self.active_text is not None:
            y = self.active_text["lines"][-1]["position"][1] if self.active_text["lines"] else self.active_text["position"][1]
            self.active_span = {"content": "", "position": [float(a["x"]), y + float(a.get("dy", 0))], "fontSize": self.active_text["fontSize"], "anchor": self.active_text["anchor"]}
        self.stack.append((tag, context))

    def handle_endtag(self, tag):
        if tag == "tspan" and self.active_span is not None:
            self.active_text["lines"].append(self.active_span)
            self.active_span = None
        if tag == "text" and self.active_text is not None:
            text = self.active_text
            owner = text.pop("owner")
            lines = text.pop("lines") or [text]
            if "node" in owner:
                self.result["participants"][owner["node"]]["texts"].extend(lines)
            elif "edge" in owner:
                key = "noteTexts" if text["fontSize"] == 7 else "texts"
                self.result["messages"][owner["edge"]][key].extend(lines)
            elif "segment" in owner:
                self.result["segments"][owner["segment"]]["texts"].extend(lines)
            elif "legend" in owner:
                if owner["legend"] is None:
                    self.result["legendTitle"] = lines[0]
                else:
                    self.result["legends"][owner["legend"]]["texts"].extend(lines)
            self.active_text = None
        for index in range(len(self.stack) - 1, -1, -1):
            if self.stack[index][0] == tag:
                self.stack = self.stack[:index]
                break

    def handle_data(self, data):
        if self.active_span is not None:
            self.active_span["content"] += data
        elif self.active_text is not None:
            self.active_text["content"] += data


def spec(count=3, messages=None, **meta):
    return {"schema_version": 1, "diagram_type": "sequence", "meta": {"title": "Sequence oracle", "output": "diagram.html", **meta},
            "participants": [{"id": f"p{i}", "type": "backend", "label": f"P{i}"} for i in range(count)],
            "messages": messages if messages is not None else [{"id": "request", "from": "p0", "to": "p1", "y": 200, "label": "request"}]}


def cases():
    rows = []
    add = lambda name, value: rows.append({"name": name, "input": value})
    for count in [2, 3, 5, 6, 7, 8]:
        for mode in [None, "fixed", "spread"]:
            meta = {} if mode is None else {"column_fit": mode}
            add(f"columns-{count}-{mode or 'omitted'}", spec(count, **meta))
    for mode in [None, "fixed", "spread"]:
        add(f"authored-width-{mode or 'omitted'}", spec(5, viewBox=[1320.5, 620.25], **({} if mode is None else {"column_fit": mode})))
    for quality in [None, "standard", "showcase"]:
        for mode in ["fixed", "spread"]:
            value = spec(3, column_fit=mode, **({} if quality is None else {"quality_profile": quality}))
            value["participants"][0]["sublabel"] = "HTTP frontend"
            value["participants"][2]["sublabel"] = "SQL storage"
            value["messages"] += [{"from": "p2", "to": "p0", "y": 300.5, "label": "return", "variant": "return", "note": "from replica"}]
            value["activations"] = [{"participant": "p2", "from": 190.5, "to": 340.75, "type": "database"}]
            value["segments"] = [{"label": "Read phase", "from": 165.5, "to": 360.75}]
            add(f"timeline-{quality or 'omitted'}-{mode}", value)
    long_note = "this note explains the whole retry and backoff policy in a lot of detail so it is very long"
    notes = [long_note, "see https://api.example.com/v2/users/profile/settings/notifications/email-preferences", "x" * 160,
             "inner.run 成功或抛错都会 close；外层 init 失败发生在 try 之前。",
             "评分：会话 ID 匹配 +4、匹配 file-edit seq +4、非空事件 +1；取最高且大于 0 的结果。",
             "快照按 commit hash 精确优先，再前缀匹配；未命中保留空摘录，不生成推测对话。",
             "首先说明 aaaaaaaaaaae\u0301成功或抛错都会 close；外层 init 失败发生在 try 之前。",
             "family 👩🏽‍💻 and 👨‍👩‍👧‍👦 stay whole while paths/a_b-c?x=y&z=2 wrap",
             "two\u00a0words\u0085stay\ufeffwith\u2028unicode whitespace"]
    for index, note in enumerate(notes):
        add(f"note-{index}", spec(7 if index >= 3 else 5, messages=[{"id": "annotated", "from": "p0", "to": "p1", "y": 200, "label": "lookup", "note": note}], quality_profile="showcase", column_fit="spread"))
    for count in [6, 7]:
        add(f"reverse-skip-note-{count}", spec(count, messages=[{"from": f"p{count - 1}", "to": "p0", "y": 200, "label": "return", "note": "from replica", "variant": "return"}], quality_profile="showcase"))
    for start in [144, 145]:
        value = spec(5, messages=[{"from": "p0", "to": "p1", "y": start + 30, "label": "ping", "note": "keep this annotation"}], quality_profile="showcase", viewBox=[920, 760])
        value["segments"] = [{"from": start, "to": 400, "label": "构建期：读取固定文件；两种构建分支"}]
        add(f"header-segment-{start}", value)
    value = spec(3, messages=[{"from": "p0", "to": "p1", "y": 200, "label": "request", "note": long_note}], quality_profile="standard", column_fit="fixed")
    value["segments"] = [{"from": 250, "to": 360, "label": "Fallback"}]
    add("note-frame-backdrop", value)
    for key, value in [("label", "Payment Gateway Service"), ("sublabel", "Readable authorization protocol")]:
        value_spec = spec(5, quality_profile="showcase")
        value_spec["participants"][1][key] = value
        add(f"automatic-{key}-width", value_spec)
    for locale in ["en", "zh-CN", "zh-TW", "es", "ko"]:
        add(f"legend-{locale}", spec(3, quality_profile="showcase", locale=locale, legend={"mode": "all"}))
    add("legend-hidden", spec(3, quality_profile="showcase", legend={"mode": "hidden"}))
    add("legend-override", spec(3, quality_profile="showcase", legend={"entries": {"default": {"label": "Application call"}, "return": {"visible": True, "label": "Result"}}}))
    for source_name, source in CONTRACTS.items():
        for name in ["cache-miss-request", "async-job-roundtrip"]:
            add(f"{source_name}-{name}", json.loads((source / f"examples/{name}.sequence.json").read_text()))
    add("negative-self-message", spec(2, messages=[{"from": "p0", "to": "p0", "y": 200, "label": "internal"}]))
    add("negative-short-canvas", spec(2, viewBox=[920, 320]))
    add("negative-crowded-messages", spec(3, messages=[{"from": "p0", "to": "p1", "y": 200, "label": "request"}, {"from": "p1", "to": "p0", "y": 220, "label": "reply"}], quality_profile="showcase"))
    add("negative-note-overlap", spec(5, messages=[{"from": "p0", "to": "p1", "y": 200, "label": "request", "note": long_note}, {"from": "p1", "to": "p0", "y": 230, "label": "reply"}], quality_profile="showcase"))
    add("negative-note-canvas", spec(3, messages=[{"from": "p0", "to": "p1", "y": 660, "label": "late", "note": long_note * 3}], quality_profile="showcase", column_fit="spread", viewBox=[920, 760], legend={"mode": "hidden"}))
    value = spec(3, messages=[{"from": "p0", "to": "p1", "y": 200, "label": "request", "note": long_note}], quality_profile="showcase", column_fit="fixed")
    value["segments"] = [{"from": 280, "to": 360, "label": "Fallback"}]
    add("negative-segment-note-label", value)
    for name, option, persisted in [
        ("quality-default", None, None),
        ("quality-explicit-standard", "standard", None),
        ("quality-explicit-showcase", "showcase", None),
        ("quality-persisted-showcase", None, "showcase"),
    ]:
        value = spec(3, **({"quality_profile": persisted} if persisted else {}))
        value["messages"][0]["note"] = "Keep the authored note and timeline position"
        row = {"name": name, "input": value}
        if option is not None:
            row["optionsQuality"] = option
        rows.append(row)
    # Source regression probes exercise title identity, bounded inside slots,
    # header strips, horizontal fallback and its activation-bar obstacles.
    neutral = {"schema_version": 1, "diagram_type": "sequence",
               "meta": {"title": "Segment strip", "output": "diagram.html", "quality_profile": "showcase", "locale": "zh-CN"},
               "participants": [{"id": key, "type": kind, "label": label} for key, kind, label in
                                [("a", "external", "甲方"), ("b", "backend", "乙方"), ("c", "cloud", "丙方"), ("d", "backend", "丁方")]],
               "segments": [{"from": 145, "to": 205, "label": "准备阶段确认目标以后开始前"}],
               "messages": [{"from": "a", "to": "b", "y": 170, "label": "请求开始以后等待返回值"}]}
    for mode in ["fixed", "spread"]:
        for quality in ["standard", "showcase"]:
            value = json.loads(json.dumps(neutral))
            value["meta"].update(column_fit=mode, quality_profile=quality, viewBox=[920, 760])
            add(f"segment-horizontal-{mode}-{quality}", value)
    value = json.loads(json.dumps(neutral))
    value["activations"] = [{"participant": "b", "from": 145, "to": 200}]
    add("segment-horizontal-activation", value)
    value = json.loads(json.dumps(neutral))
    value["segments"][0]["label"] = "阶段" * 35
    add("segment-horizontal-exhausted", value)
    for quality in ["standard", "showcase"]:
        for border in [200, 202]:
            value = spec(2, messages=[{"from": "p0", "to": "p1", "y": 200, "label": "ping"}], quality_profile=quality, viewBox=[920, 760])
            value["segments"] = [{"from": 140, "to": border, "label": "Phase"}]
            add(f"segment-border-{border}-{quality}", value)
    source_fixtures = ROOT / "local/phoenix-diagram/reference/test/fixtures/sequence-first-draft"
    value = json.loads((source_fixtures / "scan-to-pay.sequence.json").read_text())
    add("segment-abutting-original", value)
    value = json.loads(json.dumps(value))
    for message in value["messages"]:
        if message["y"] >= 352:
            message["y"] += 12
    value["segments"][-1]["to"] += 12
    add("segment-abutting-spaced", value)
    value = json.loads((source_fixtures / "resumable-upload.sequence.json").read_text())
    ys = {"PATCH": 300, "UploadPart": 320, "HEAD": 332, "Complete": 364, "scan": 404, "clean": 436, "insert": 468, "204": 500}
    for message in value["messages"]:
        for prefix, y in ys.items():
            if message["label"].startswith(prefix):
                message["y"] = y
                break
    value["segments"] = [{"from": 140, "to": 280, "label": "Create"}, {"from": 286, "to": 384, "label": "Transfer + resume"}, {"from": 390, "to": 540, "label": "Finalize"}]
    add("segment-packed-three-phases", value)
    for mode in ["all", "auto"]:
        add(f"legend-too-wide-{mode}", spec(3, viewBox=[640, 760], legend={"mode": mode, "entries": {"default": {"label": "界" * 80}}}))
    return rows


def generate():
    identity = json.loads(subprocess.check_output(["mise", "exec", "--no-deps", "--", "node", "-p", "JSON.stringify({node:process.versions.node,icu:process.versions.icu,unicode:process.versions.unicode,locale:new Intl.Segmenter().resolvedOptions().locale,executable:process.execPath})"], cwd=ROOT, text=True))
    assert {key: identity[key] for key in ("node", "icu", "unicode", "locale")} == {"node": "25.2.1", "icu": "77.1", "unicode": "16.0", "locale": "en-US"}, identity
    node = identity.pop("executable")
    subprocess.run([node, "--input-type=module", "-e", "import {verifyFrozenSources} from './scripts/check-phoenix-diagram-source.mjs';verifyFrozenSources({localReference:'local/phoenix-diagram/reference'});"], cwd=ROOT, check=True)
    wrapper = "import{pathToFileURL}from'node:url';const[r,i,o]=process.argv.slice(1);process.argv=[process.execPath,r,i,o];try{await import(pathToFileURL(r).href)}catch(e){process.stderr.write('PHOENIX_ORACLE_FAILURE '+JSON.stringify(e.archifyDiagnostics||[{code:'source/validation',message:e.message}])+'\\n');process.exitCode=1}"
    rows = cases()
    environment = {key: value for key, value in os.environ.items() if key not in ["ARCHIFY_QUALITY_PROFILE", "ARCHIFY_REPO_ROOT"]}
    with tempfile.TemporaryDirectory(prefix="phoenix-sequence-oracle-") as temporary:
        input_path = Path(temporary) / "input.json"
        output_path = Path(temporary) / "output.html"
        for row in rows:
            effective_quality = row.get("optionsQuality", row["input"]["meta"].get("quality_profile", "showcase"))
            row["effectiveQuality"] = effective_quality
            source_input = json.loads(json.dumps(row["input"]))
            source_input["meta"]["quality_profile"] = effective_quality
            input_path.write_text(json.dumps(source_input, ensure_ascii=False))
            row["oracles"] = {}
            for contract, source in CONTRACTS.items():
                if output_path.exists():
                    output_path.unlink()
                result = subprocess.run([node, "--input-type=module", "-e", wrapper, str(source / "renderers/sequence/render-sequence.mjs"), str(input_path), str(output_path)], cwd=temporary, env=environment, capture_output=True, text=True)
                if result.returncode:
                    marker = "PHOENIX_ORACLE_FAILURE "
                    assert marker in result.stderr, result.stderr
                    row["oracles"][contract] = {"success": False, "diagnostics": json.loads(result.stderr.split(marker, 1)[1].strip())}
                    continue
                html = output_path.read_text()
                middle = html.index("<!-- Time Segments -->")
                start = html.rfind("<svg", 0, middle)
                end = html.index("</svg>", middle) + len("</svg>")
                parser = SvgGeometry()
                parser.feed(html[start:end])
                geometry = parser.result
                assert len(geometry["participants"]) == len(row["input"]["participants"]), row["name"]
                assert len(geometry["messages"]) == len(row["input"]["messages"]), row["name"]
                row["oracles"][contract] = {"success": True, **geometry}
    return {"referenceContentSha256": "4f892b0c68fef5d21788264ba6a43fe9e1d377d3c2d5b558f6aa346784238395", "legacyRevision": "4a65a1525f9ed03c775a0e18935901e1edbb773f", "nodeIdentity": identity, "geometryEvidence": "Exact unrounded SVG attributes, text and tspan line positions from the frozen source renderers. No layout receipt rounding.", "qualityProjection": "CompileOptions.quality > authored meta.quality_profile > showcase; projected only into the temporary source invocation, with corpus input bytes retained.", "cases": rows}


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--check", action="store_true")
    arguments = parser.parse_args()
    if arguments.write == arguments.check:
        parser.error("choose exactly one of --write or --check")
    result = generate()
    encoded = json.dumps(result, ensure_ascii=False, indent=2) + "\n"
    if arguments.write:
        DESTINATION.write_text(encoded)
    elif DESTINATION.read_text() != encoded:
        raise SystemExit("Sequence source oracle changed; inspect the pinned source and tool identity before updating")
    print(json.dumps({"cases": len(result["cases"]), "acceptedSourceArtifacts": sum(oracle["success"] for row in result["cases"] for oracle in row["oracles"].values()), "nodeIdentity": result["nodeIdentity"]}))
