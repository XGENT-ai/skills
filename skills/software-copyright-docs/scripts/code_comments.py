#!/usr/bin/env python3
"""Remove emoji and normalize Chinese quotes inside source comments.

Emoji in comments reads as AI-written code in copyright material, and 「」 is
not the Chinese double quote reviewers expect. Strings and UI text are kept
as-is, so only comment spans are rewritten. Comment detection is a lexical
heuristic: emoji and 「」 only occur in comments or strings, so a misread span
can at worst leave an emoji in place or clean a string.
"""

from __future__ import annotations

import re
from dataclasses import dataclass


EMOJI_CHARS = (
    "\U0001F000-\U0001FAFF"  # pictographs, emoticons, flags, skin tones
    "\u2600-\u27BF"  # misc symbols and dingbats
    "\u2B05-\u2B07\u2B1B\u2B1C\u2B50\u2B55"
    "\u231A\u231B\u2328\u23CF\u23E9-\u23F3\u23F8-\u23FA"
    "\uFE0E\uFE0F\u200D\u20E3"  # variation selectors, ZWJ, keycap
    "\U000E0020-\U000E007F"  # tag sequences
)
EMOJI_UNIT = f"(?:[0-9#*]\uFE0F?\u20E3|[{EMOJI_CHARS}])"
EMOJI_RE = re.compile(f"([ \\t]*){EMOJI_UNIT}+(?:[ \\t]+{EMOJI_UNIT}+)*([ \\t]*)")
QUOTE_MAP = str.maketrans({"「": "“", "」": "”"})


@dataclass(frozen=True)
class Syntax:
    line: tuple[str, ...] = ()
    block: tuple[tuple[str, str], ...] = ()
    # (delimiter, may span lines); longer delimiters must come first.
    quotes: tuple[tuple[str, bool], ...] = (('"', False), ("'", False))
    docstrings: bool = False


C_BLOCK = (("/*", "*/"),)
JS_QUOTES = (("`", True), ('"', False), ("'", False))
PY_QUOTES = (('"""', True), ("'''", True), ('"', False), ("'", False))

C_LIKE = Syntax(line=("//",), block=C_BLOCK, quotes=JS_QUOTES)
MARKUP = Syntax(line=("//",), block=(("<!--", "-->"),) + C_BLOCK, quotes=JS_QUOTES)
HASH = Syntax(line=("#",), quotes=PY_QUOTES)
DEFAULT = Syntax(line=("//", "#"), block=C_BLOCK + (("<!--", "-->"),), quotes=JS_QUOTES)

SYNTAX_BY_SUFFIX: dict[str, Syntax] = {
    **dict.fromkeys(
        (".js", ".jsx", ".ts", ".tsx", ".mjs", ".cjs", ".java", ".kt", ".kts", ".go", ".rs",
         ".c", ".cc", ".cpp", ".cxx", ".h", ".hh", ".hpp", ".cs", ".swift", ".dart",
         ".scala", ".groovy", ".gradle", ".gml"),
        C_LIKE,
    ),
    ".css": Syntax(block=C_BLOCK),
    **dict.fromkeys((".scss", ".sass", ".less"), Syntax(line=("//",), block=C_BLOCK)),
    **dict.fromkeys((".html", ".htm", ".xml", ".vue", ".svelte", ".astro"), MARKUP),
    **dict.fromkeys((".py", ".pyw"), Syntax(line=("#",), quotes=PY_QUOTES, docstrings=True)),
    **dict.fromkeys((".rb", ".sh", ".bash", ".zsh", ".yaml", ".yml", ".toml", ".r", ".pl", ".gd"), HASH),
    ".ps1": Syntax(line=("#",), block=(("<#", "#>"),)),
    ".php": Syntax(line=("//", "#"), block=C_BLOCK, quotes=JS_QUOTES),
    ".sql": Syntax(line=("--",), block=C_BLOCK),
    ".lua": Syntax(line=("--",), block=(("--[[", "]]"),)),
}


def _replace_emoji(match: re.Match[str]) -> str:
    text, start, end = match.string, match.start(), match.end()
    before = text[start - 1] if start else "\n"
    after = text[end] if end < len(text) else "\n"
    # Keep indentation and the gap before following text ("// 🚀 启动" -> "// 启动"),
    # but not trailing gaps ("完成 ✅" -> "完成") or gaps the emoji never had.
    keep = before == "\n" or (match.group(2) and after != "\n")
    return match.group(1) if keep else ""


def _is_line_start(text: str, index: int) -> bool:
    line_start = text.rfind("\n", 0, index) + 1
    return not text[line_start:index].strip()


def _comment_spans(text: str, syntax: Syntax) -> list[tuple[int, int, str | None]]:
    """Return (start, end, line_token) for each comment; line_token is None for blocks."""
    spans: list[tuple[int, int, str | None]] = []
    i = 0
    n = len(text)
    while i < n:
        block = next(((s, e) for s, e in syntax.block if text.startswith(s, i)), None)
        if block is None and syntax.docstrings and _is_line_start(text, i):
            block = next(((q, q) for q, _ in syntax.quotes[:2] if text.startswith(q, i)), None)
        if block is not None:
            end = text.find(block[1], i + len(block[0]))
            end = n if end < 0 else end + len(block[1])
            spans.append((i, end, None))
            i = end
            continue
        token = next((t for t in syntax.line if text.startswith(t, i)), None)
        # Shell "$#" and "${#var}" are not comments.
        if token is not None and not (token == "#" and i and text[i - 1] in "${"):
            end = text.find("\n", i)
            end = n if end < 0 else end
            spans.append((i, end, token))
            i = end
            continue
        quote = next(((q, multi) for q, multi in syntax.quotes if text.startswith(q, i)), None)
        if quote is None:
            i += 1
            continue
        delimiter, multiline = quote
        i += len(delimiter)
        while i < n:
            if text[i] == "\\":
                i += 2
            elif text.startswith(delimiter, i):
                i += len(delimiter)
                break
            elif text[i] == "\n" and not multiline:
                break
            else:
                i += 1
    return spans


def clean_comments(text: str, suffix: str) -> str:
    syntax = SYNTAX_BY_SUFFIX.get(suffix.lower(), DEFAULT)
    parts: list[str] = []
    cursor = 0
    for start, end, token in _comment_spans(text, syntax):
        original = text[start:end]
        cleaned = EMOJI_RE.sub(_replace_emoji, original).translate(QUOTE_MAP)
        parts.append(text[cursor:start])
        if token is not None and cleaned != original and not cleaned.strip().replace(token, "").strip("/!"):
            # The comment held only emoji; drop its marker and the gap before it.
            parts[-1] = parts[-1].rstrip(" \t")
        else:
            parts.append(cleaned)
        cursor = end
    parts.append(text[cursor:])
    return "".join(parts)
