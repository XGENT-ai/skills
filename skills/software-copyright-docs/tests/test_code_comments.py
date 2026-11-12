from __future__ import annotations

import json
import shutil
import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1] / "scripts"
sys.path.insert(0, str(SCRIPTS))

from code_comments import clean_comments  # noqa: E402
from extract_code_material import collect_code_lines  # noqa: E402


class CommentCleanupTests(unittest.TestCase):
    def test_c_like_comments_are_cleaned_but_strings_are_kept(self) -> None:
        source = (
            "// 🚀 启动服务\n"
            "const tip = '✅ 保存「成功」';\n"
            "/*\n"
            " * ✨ 读取「用户」配置\n"
            " */\n"
            "run(); // 完成✅ 下一步\n"
        )
        self.assertEqual(
            clean_comments(source, ".ts"),
            "// 启动服务\n"
            "const tip = '✅ 保存「成功」';\n"
            "/*\n"
            " * 读取“用户”配置\n"
            " */\n"
            "run(); // 完成下一步\n",
        )

    def test_python_hash_comments_and_docstrings_are_cleaned(self) -> None:
        source = (
            'def load():\n'
            '    """🔍 加载「订单」"""\n'
            '    marker = "# 🚀 不是注释"\n'
            "    sql = '''\n"
            "    -- 🎯 模板文本\n"
            "    '''\n"
            "    return marker  # 👍 返回\n"
        )
        self.assertEqual(
            clean_comments(source, ".py"),
            'def load():\n'
            '    """加载“订单”"""\n'
            '    marker = "# 🚀 不是注释"\n'
            "    sql = '''\n"
            "    -- 🎯 模板文本\n"
            "    '''\n"
            "    return marker  # 返回\n",
        )

    def test_emoji_only_comments_are_removed_with_their_marker(self) -> None:
        source = "# ✅\nvalue = 1  # 🎉\n// ---\n"
        self.assertEqual(clean_comments(source, ".py"), "\nvalue = 1\n// ---\n")

    def test_emoji_sequences_are_removed_completely(self) -> None:
        source = "// 👨\u200d💻 ❤️ 👍🏽 🇨🇳 1️⃣ ok\n"
        self.assertEqual(clean_comments(source, ".js"), "// ok\n")

    def test_markup_comments_are_cleaned_but_text_is_kept(self) -> None:
        source = "<!-- 🎉 首页「入口」 -->\n<span>🎉 完成</span>\n"
        self.assertEqual(
            clean_comments(source, ".vue"),
            "<!-- 首页“入口” -->\n<span>🎉 完成</span>\n",
        )

    def test_extracted_material_uses_cleaned_comments(self) -> None:
        project = Path(__file__).resolve().parent / ".tmp-code-comments"
        project.mkdir(exist_ok=True)
        self.addCleanup(shutil.rmtree, project)
        (project / "app.js").write_text("// 🚀\nstart(); // ✅ 启动「服务」\n", encoding="utf-8")
        selection = project / "selection.json"
        selection.write_text(json.dumps({"files": [{"path": "app.js", "selected": True}]}), encoding="utf-8")

        lines, _ = collect_code_lines(project, selection)

        self.assertEqual(lines, ["// File: app.js", "start(); // 启动“服务”"])


if __name__ == "__main__":
    unittest.main()
