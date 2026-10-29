import json
import shlex
import subprocess

from evolution import Experiment, freeze
from protocol import canonical, prepare
from remote import grading_payload
from test_quality import QualityFixture


class VerifierContextTests(QualityFixture):
    def setUp(self):
        super().setUp()
        self.evaluation = self.quality_spec()

    def job(self, names):
        path = self.evaluation / "rubric.json"
        rubric = json.loads(path.read_bytes())
        rubric["criteria"][0]["context_paths"] = ["patch.diff", *names]
        path.write_bytes(canonical(rubric))
        return prepare(self.write())

    def test_declared_observations_reach_quality_context_without_private_or_extra_files(self):
        name = "verifier/runtime.json"
        job = self.job([name])
        output = self.root / "run"
        engine = Experiment(job, output, freeze(job, output), None)
        graded = self.root / "graded"
        graded.mkdir()
        (graded / "reward.txt").write_bytes(b"1\n")
        observed = '{"node":"v25.2.1","commands":[{"exit_code":0}]}\n'
        (graded / "public-context.json").write_bytes(canonical({"files": {
            name: observed, "verifier/undeclared.json": "PRIVATE_EXTRA_CANARY",
            "repo/private-answer.txt": "PRIVATE_ANSWER_CANARY",
        }, "omissions": []}))
        context = engine.quality_context(job.spec["cases"][0], b"patch\n", graded, {},
                                         {name: b"stale provided observation"})
        self.assertEqual(context[name], observed.encode())
        self.assertEqual(context["verifier.txt"], b"1\n")
        self.assertFalse(any(b"PRIVATE_" in raw for raw in context.values()))

    def test_missing_public_observation_cannot_fall_back_to_artifact_claims(self):
        name = "verifier/runtime.json"
        job = self.job([name])
        output = self.root / "run"
        engine = Experiment(job, output, freeze(job, output), None)
        graded = self.root / "graded"
        graded.mkdir()
        (graded / "reward.txt").write_bytes(b"1\n")
        for snapshot in [None, {"files": {}, "omissions": [name]}]:
            if snapshot is not None:
                (graded / "public-context.json").write_bytes(canonical(snapshot))
            with self.subTest(snapshot=snapshot):
                context = engine.quality_context(job.spec["cases"][0], b"patch\n", graded, {},
                                                 {name: b"self-reported success"})
                self.assertNotIn(name, context)

    def test_unrequested_observation_snapshot_is_not_export_authority(self):
        job = self.job([])
        output = self.root / "run"
        engine = Experiment(job, output, freeze(job, output), None)
        graded = self.root / "graded"
        graded.mkdir()
        (graded / "public-context.json").write_bytes(canonical({"files": {
            "verifier/runtime.json": "UNREQUESTED_CANARY",
        }, "omissions": []}))
        context = engine.quality_context(job.spec["cases"][0], b"patch\n", graded, {})
        self.assertNotIn("verifier/runtime.json", context)
        payload = grading_payload(job, job.spec["cases"][0], b"")
        self.assertNotIn("public-context.json", payload["command"]["capture"]["files"])

    def test_post_verifier_collection_preserves_exit_and_rejects_unsafe_observation_paths(self):
        names = ["verifier/runtime.json", "verifier/missing.json", "verifier/link.json",
                 "verifier/linked/nested.json", "verifier/binary.bin"]
        job = self.job(names)
        payload = grading_payload(job, job.spec["cases"][0], b"")
        self.assertIn("public-context.json", payload["command"]["capture"]["files"])
        public = self.root.resolve() / "public"
        public.mkdir()
        private = self.root.resolve() / "private"
        private.mkdir()
        (private / "answer.json").write_text("PRIVATE_FILE_CANARY")
        (private / "nested.json").write_text("PRIVATE_DIRECTORY_CANARY")
        (public / "link.json").symlink_to(private / "answer.json")
        (public / "linked").symlink_to(private, target_is_directory=True)
        (public / "binary.bin").write_bytes(b"\xff")
        grader = self.root / "test.sh"
        snapshot = self.root / "observations.json"
        for status in [0, 1]:
            grader.write_text("#!/bin/sh\nprintf 'observed after verifier\\r\\n' > "
                              + shlex.quote(str(public / "runtime.json")) + f"\nexit {status}\n")
            script = payload["command"]["args"][2]
            for before, after in [
                ("/logs/verifier/public-context.json", str(snapshot)),
                ("/logs/verifier/public", str(public)),
                ("/logs/verifier/quality-context.json", str(self.root / "repository.json")),
                ("/tests/test.sh", str(grader)),
            ]:
                script = script.replace(before, after)
            observed = subprocess.run(["bash", "-c", script], cwd=self.repo, capture_output=True)
            with self.subTest(status=status):
                self.assertEqual(observed.returncode, status, observed.stderr)
                result = json.loads(snapshot.read_bytes())
                self.assertEqual(result["files"], {"verifier/runtime.json": "observed after verifier\r\n"})
                self.assertEqual(set(result["omissions"]), set(names) - {"verifier/runtime.json"})
                self.assertNotIn("PRIVATE_", snapshot.read_text())

    def test_public_observation_root_cannot_be_a_symlink_to_private_material(self):
        name = "verifier/runtime.json"
        job = self.job([name])
        payload = grading_payload(job, job.spec["cases"][0], b"")
        private = self.root.resolve() / "private"
        private.mkdir()
        (private / "runtime.json").write_text("PRIVATE_ROOT_CANARY")
        public = self.root.resolve() / "public"
        public.symlink_to(private, target_is_directory=True)
        grader = self.root / "test.sh"
        grader.write_text("#!/bin/sh\nexit 0\n")
        snapshot = self.root / "observations.json"
        script = payload["command"]["args"][2]
        for before, after in [
            ("/logs/verifier/public-context.json", str(snapshot)),
            ("/logs/verifier/public", str(public)),
            ("/logs/verifier/quality-context.json", str(self.root / "repository.json")),
            ("/tests/test.sh", str(grader)),
        ]:
            script = script.replace(before, after)
        observed = subprocess.run(["bash", "-c", script], cwd=self.repo, capture_output=True)
        self.assertEqual(observed.returncode, 0, observed.stderr)
        result = json.loads(snapshot.read_bytes())
        self.assertEqual(result, {"files": {}, "omissions": [name]})
        self.assertNotIn("PRIVATE_ROOT_CANARY", snapshot.read_text())
