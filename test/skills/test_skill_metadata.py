from pathlib import Path
import re
import unittest

try:
    import yaml
except ImportError:
    yaml = None


ROOT = Path(__file__).resolve().parents[2]
SKILLS = ROOT / "skills"
# 通用校验器（如 Codex 的 quick_validate.py）不认 argument-hint 等本仓有意使用的字段。
FRONTMATTER_KEYS = {
    "name",
    "description",
    "argument-hint",
    "disable-model-invocation",
    "license",
    "metadata",
    "compatibility",
    "allowed-tools",
    "internal",
}
OPENAI_KEYS = {"interface", "policy"}
FRONTMATTER = re.compile(r"\A---\n(.*?)\n---\n", re.S)


@unittest.skipIf(yaml is None, "PyYAML 未安装")
class SkillMetadataTest(unittest.TestCase):
    def test_skill_frontmatter(self):
        paths = sorted(SKILLS.glob("*/SKILL.md"))
        self.assertTrue(paths)
        for path in paths:
            name = path.parent.name
            with self.subTest(skill=name):
                match = FRONTMATTER.match(path.read_text(encoding="utf-8"))
                self.assertIsNotNone(match, "SKILL.md 须以 YAML frontmatter 开头")
                data = yaml.safe_load(match.group(1))
                self.assertIsInstance(data, dict)
                self.assertEqual(data.get("name"), name)
                description = data.get("description")
                self.assertIsInstance(description, str)
                self.assertTrue(description.strip())
                self.assertEqual(set(data) - FRONTMATTER_KEYS, set())

    def test_openai_yaml(self):
        for path in sorted(SKILLS.glob("*/agents/openai.yaml")):
            name = path.parents[1].name
            with self.subTest(skill=name):
                data = yaml.safe_load(path.read_text(encoding="utf-8"))
                self.assertIsInstance(data, dict)
                self.assertEqual(set(data) - OPENAI_KEYS, set())
                interface = data.get("interface") or {}
                self.assertTrue(interface.get("display_name"))
                self.assertTrue(interface.get("short_description"))
                prompt = interface.get("default_prompt")
                if prompt is not None:
                    self.assertIn(f"${name}", prompt)


if __name__ == "__main__":
    unittest.main()
