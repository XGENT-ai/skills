import copy
import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "check_evidence.py"


class EvidenceChecks(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / "export.py"
        self.source.write_text("result = 'csv'\n")
        self.log = self.root / "export.log"
        self.log.write_text("observed expected CSV\n")
        self.contract = self.root / "contract.json"
        self.contract.write_text(json.dumps({
            "goal": "export CSV",
            "requirements": [{"id": "csv", "checks": ["export"]}],
        }))
        self.receipts = self.root / "receipts.json"
        self.data = {
            "contract_sha256": self.sha(self.contract),
            "checks": [{
                "id": "export", "status": "passed",
                "observation": "exported expected CSV through the command",
                "inputs": [self.fingerprint(self.source)],
                "artifacts": [self.fingerprint(self.log)],
            }],
        }

    @staticmethod
    def sha(path):
        return hashlib.sha256(path.read_bytes()).hexdigest()

    def fingerprint(self, path):
        return {"path": path.name, "sha256": self.sha(path)}

    def run_check(self):
        self.receipts.write_text(json.dumps(self.data))
        result = subprocess.run([
            sys.executable, str(SCRIPT), "check", "--root", str(self.root),
            "--contract", str(self.contract), "--receipts", str(self.receipts),
        ], capture_output=True, text=True, check=False)
        return result.returncode, json.loads(result.stdout)

    def test_consistent_receipt(self):
        code, result = self.run_check()
        self.assertEqual(code, 0)
        self.assertTrue(result["records_consistent"])
        self.assertNotIn("task_complete", result)

    def test_missing_required_check(self):
        self.data["checks"] = []
        code, result = self.run_check()
        self.assertEqual(code, 1)
        self.assertIn("export: missing required check", result["issues"])

    def test_failed_check(self):
        self.data["checks"][0]["status"] = "failed"
        code, result = self.run_check()
        self.assertEqual(code, 1)
        self.assertFalse(result["records_consistent"])

    def test_changed_input_invalidates_receipt(self):
        self.source.write_text("result = 'wrong'\n")
        code, result = self.run_check()
        self.assertEqual(code, 1)
        self.assertIn("export.inputs: changed file export.py", result["issues"])

    def test_unrelated_change_does_not_invalidate_receipt(self):
        (self.root / "notes.txt").write_text("unrelated")
        self.assertEqual(self.run_check()[0], 0)

    def test_changed_evidence_invalidates_receipt(self):
        self.log.write_text("replacement log\n")
        code, result = self.run_check()
        self.assertEqual(code, 1)
        self.assertIn("export.artifacts: changed file export.log", result["issues"])

    def test_missing_evidence_invalidates_receipt(self):
        self.log.unlink()
        code, result = self.run_check()
        self.assertEqual(code, 1)
        self.assertIn("export.artifacts: missing file: export.log", result["issues"])

    def test_modified_contract_invalidates_receipt(self):
        self.contract.write_text(json.dumps({
            "goal": "different goal",
            "requirements": [{"id": "csv", "checks": ["export"]}],
        }))
        code, result = self.run_check()
        self.assertEqual(code, 1)
        self.assertIn("contract changed or receipt is bound to another contract", result["issues"])

    def test_duplicate_check_cannot_hide_failure(self):
        other = copy.deepcopy(self.data["checks"][0])
        other["status"] = "failed"
        self.data["checks"].append(other)
        code, result = self.run_check()
        self.assertEqual(code, 2)
        self.assertIn("checks: duplicate id export", result["issues"])

    def test_outside_file_is_rejected(self):
        with tempfile.TemporaryDirectory() as outside:
            secret = Path(outside) / "outside.txt"
            secret.write_text("outside workspace")
            (self.root / "linked.txt").symlink_to(secret)
            self.data["checks"][0]["inputs"] = [{
                "path": "linked.txt", "sha256": self.sha(secret),
            }]
            code, result = self.run_check()
            self.assertEqual(code, 1)
            self.assertIn("export.inputs: path escapes root: linked.txt", result["issues"])

    def test_artifact_is_not_its_own_input(self):
        self.data["checks"][0]["artifacts"] = [self.fingerprint(self.source)]
        code, result = self.run_check()
        self.assertEqual(code, 1)
        self.assertIn("export: input and evidence must be separate files", result["issues"])

    def test_missing_inputs_is_invalid(self):
        self.data["checks"][0]["inputs"] = []
        self.assertEqual(self.run_check()[0], 2)

    def test_fingerprint_cli_uses_real_content(self):
        result = subprocess.run([
            sys.executable, str(SCRIPT), "fingerprint", "--root", str(self.root), "export.py",
        ], capture_output=True, text=True, check=True)
        self.assertEqual(json.loads(result.stdout), [self.fingerprint(self.source)])


if __name__ == "__main__":
    unittest.main()
