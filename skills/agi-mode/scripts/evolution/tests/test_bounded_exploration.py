"""Offline checks for one-candidate exploration without confirmation or adoption."""
import copy
import difflib
import unittest
from unittest.mock import patch

import test_calibration
from test_quality import QualityFixture, rubric
from test_search import QualityRecordedRemote
from evolution import Experiment, freeze, restore_job
from protocol import ProtocolError, bundle_hash, canonical, prepare, sha256, validate_measurement


class BoundedDiagnosticTests(unittest.TestCase):
    def diagnose(self, *, policy=True, mutate=None, remaining=10000, noise=None):
        helper = test_calibration.BaselineDiagnosticTests()
        spec, rows, hashes = helper.baseline(groups=1)
        spec['mode'] = 'search'
        if policy:
            spec['measurement']['search_policy'] = 'bounded_exploration_v1'
        if mutate:
            mutate(spec, rows)
        return helper.diagnose(spec, rows, hashes, remaining=remaining, noise=noise)

    def test_precision_only_failure_gets_separate_permission_without_claiming_strict_pass(self):
        result = self.diagnose()
        self.assertFalse(result['proceed'])
        self.assertEqual(result['reason'], 'precision_budget_insufficient')
        self.assertTrue(result['search_permission']['allowed'])
        self.assertEqual(result['search_permission']['scope'], 'one_candidate_no_adoption')
        self.assertNotIn('search_permission', self.diagnose(policy=False))

    def test_budget_noise_unknown_saturation_and_hard_failure_remain_closed(self):
        def success(value):
            return lambda spec, rows: [r.update(task_success=value) for r in rows]
        def gates(spec, rows):
            rows[0]['failed_gates'] = ['correctness']
        def guardrail(spec, rows):
            rows[0]['checks']['authorization'] = False
        def unknown(spec, rows):
            rows[0].update(score=None, task_quality=None, quality_status='unknown', unknown_criteria=['regression'])
            rows[0]['quality_components']['regression'].update(status='unknown', level=None)
        def generation_noise(spec, rows):
            rows[-1].update(score=1, task_quality=1)
            rows[-1]['quality_components']['regression']['level'] = 1
        def saturated(spec, rows):
            for row in rows:
                row.update(score=1, task_quality=1)
                row['quality_components']['regression']['level'] = 1
        def insufficient_headroom(spec, rows):
            for row in rows:
                row.update(score=.98, task_quality=.98)
                row['quality_components']['regression']['level'] = .96
        for options in ({'remaining': 10}, {'noise': {'status': 'valid', 'disagreements': 1, 'max_difference': .5}},
                        {'mutate': success(0)}, {'mutate': gates}, {'mutate': guardrail}, {'mutate': unknown},
                        {'mutate': generation_noise}, {'mutate': saturated},
                        {'mutate': insufficient_headroom}):
            with self.subTest(options=options):
                self.assertFalse(self.diagnose(**options)['search_permission']['allowed'])


class BoundedExecutionTests(QualityFixture):
    def setUp(self):
        super().setUp()
        self.compare()
        self.quality_spec()
        heldout = rubric('confirmation-case')
        for item in heldout['criteria']:
            item['public_requirement_ref'] = 'instruction.md: Fix a different heldout bug.'
        (self.root/'evaluation/heldout.json').write_bytes(canonical(heldout))
        self.spec['evaluation']['paths'].append('heldout.json')
        self.spec['evaluation']['rubrics']['confirmation-case'] = 'heldout.json'
        self.spec.update(mode='search')
        self.spec.pop('candidate_condition')
        self.spec['conditions'] = self.spec['conditions'][:1]
        self.spec['measurement']['search_policy'] = 'bounded_exploration_v1'
        self.spec['guardrails'] = [{'name': 'task_contract', 'minimum': 1}]
        self.spec['authority'].update(allow_apply=False, write_paths=[])
        method = self.root/'method.md'
        method.write_text('Propose one mechanism for a measured weakness.')
        self.spec['optimizer'] = {'model': 'gpt-6.1-sol', 'effort': 'high',
                                  'method_path': str(method), 'method_sha256': sha256(method.read_bytes())}

    def test_protocol_rejects_other_modes_multiple_candidates_and_write_authority(self):
        validate_measurement(self.spec)
        for changes in ({'mode': 'calibrate'}, {'mode': 'compare'}, {'max_candidates': 2},
                        {'allow_apply': True}, {'write_paths': ['AGENTS.md']},
                        {'search_policy': True}, {'search_policy': 'unbounded'}):
            value = copy.deepcopy(self.spec)
            for key, item in changes.items():
                target = (value['budget'] if key == 'max_candidates' else value['authority']
                          if key in {'allow_apply', 'write_paths'} else value['measurement']
                          if key == 'search_policy' else value)
                target[key] = item
            with self.subTest(changes=changes), self.assertRaises(ProtocolError):
                validate_measurement(value)

    def test_failed_calibration_still_stops_before_the_optimizer(self):
        job = prepare(self.write())
        output = self.root/'failed-calibration'
        state = freeze(job, output)
        state['calibration_status'] = 'failed'
        engine = Experiment(job, output, state, QualityRecordedRemote())
        engine.calibrate = lambda: None
        engine.generate = lambda *args: self.fail('failed calibration must not generate')
        self.assertEqual(engine.run()['reason'], 'quality_calibration_failed')
        self.assertEqual(state['candidates_used'], 0)

    def test_initial_baseline_gaps_cannot_reuse_the_earlier_aa_permission(self):
        for gap in ('unknown', 'failure', 'guardrail'):
            with self.subTest(gap=gap):
                job = prepare(self.write())
                output = self.root/('initial-gap-' + gap)
                state = freeze(job, output)
                state.update(calibration_passed=True, calibration_status='passed',
                             quality_calibration_certificate={'status': 'passed', 'certificate_sha256': 'e'*64})
                engine = Experiment(job, output, state, QualityRecordedRemote())
                engine.calibrated_judge_signature = lambda: 'fixture-config'
                original_rows = engine.rows
                def rows():
                    values = original_rows()
                    for row in values:
                        if row.get('comparison_id') == 'initial' and row['candidate_id'] == engine.baseline:
                            if gap == 'unknown':
                                row.update(score=None, task_quality=None, quality_status='unknown', unknown_criteria=['regression'])
                                row['quality_components']['regression'].update(status='unknown', level=None)
                            elif gap == 'failure':
                                row.update(score=0, task_quality=0, task_success=0)
                            else:
                                row['checks']['task_contract'] = False
                    return values
                engine.rows = rows
                engine.generate = lambda *args: self.fail('unusable initial baseline must stop before generating')
                with patch('evolution.judge_payload_signature', return_value='fixture-config'):
                    report = engine.run()
                self.assertTrue(state['baseline_diagnostic']['search_permission']['allowed'])
                self.assertEqual(report['reason'], 'bounded_exploration_feedback_unusable')
                self.assertEqual(state['candidates_used'], 0)

    def test_one_engine_attempt_and_resume_never_reach_confirmation_or_adoption(self):
        self.exercise_engine()

    def test_search_only_is_allowed_only_for_bounded_exploration(self):
        self.spec['cases'] = [case for case in self.spec['cases'] if case['split'] == 'search']
        ids = {case['case_id'] for case in self.spec['cases']}
        self.spec['measurement']['weights'] = {key: value for key, value in self.spec['measurement']['weights'].items() if key in ids}
        self.spec['evaluation']['rubrics'] = {key: value for key, value in self.spec['evaluation']['rubrics'].items() if key in ids}
        self.spec['measurement']['search_policy'] = 'strict'
        with self.assertRaisesRegex(ProtocolError, 'heldout confirmation'):
            prepare(self.write())
        self.spec['measurement']['search_policy'] = 'bounded_exploration_v1'
        self.exercise_engine()

    def test_strict_proceed_cannot_bypass_a_denied_bounded_permission(self):
        job = prepare(self.write())
        output = self.root/'denied-bounded-permission'
        state = freeze(job, output)
        diagnostic = BoundedDiagnosticTests().diagnose()
        diagnostic.update(proceed=True, reason='quality_search_can_proceed')
        diagnostic['search_permission']['allowed'] = False
        state.update(calibration_passed=True, calibration_status='passed', aa_done=True,
                     baseline_diagnostic=diagnostic)
        engine = Experiment(job, output, state, QualityRecordedRemote())
        engine.evaluate = lambda *args, **kwargs: self.fail('denied bounded permission must stop all further evaluation')
        self.assertEqual(engine.run()['reason'], 'bounded_exploration_not_permitted')
        self.assertEqual(state['candidates_used'], 0)

    def test_even_a_favorable_comparison_cannot_advance_any_best_or_confirmation(self):
        self.exercise_engine(favorable=True)

    def exercise_engine(self, favorable=False):
        job = prepare(self.write())
        output = self.root/'bounded-run'
        state = freeze(job, output)
        state.update(calibration_passed=True, calibration_status='passed',
                     quality_calibration_certificate={'status': 'passed', 'certificate_sha256': 'e'*64})
        remote = QualityRecordedRemote()
        engine = Experiment(job, output, state, remote)
        # This test isolates search control flow; existing signature tests cover
        # the real judge-configuration binding. All provider output is synthetic.
        engine.calibrated_judge_signature = lambda: 'fixture-config'
        generated = []
        engine.policy_check = lambda candidate: {'passed': True, 'checks': []}
        def generate(parent):
            generated.append(bundle_hash(parent))
            old = parent['AGENTS.md']['content'].decode()
            new = old + 'Use public requirements when adding regression assertions.\n'
            patch = ''.join(difflib.unified_diff(old.splitlines(True), new.splitlines(True),
                                               fromfile='a/AGENTS.md', tofile='b/AGENTS.md')).encode()
            return {'parent_hash': bundle_hash(parent), 'hypothesis': 'Bind regression assertions to public requirements',
                    'expected_effect': 'Protect the documented boundary', 'counterexample': 'A boundary remains untested',
                    'changed_paths': ['AGENTS.md'], 'mechanism': {'criterion_id': 'regression',
                    'patch_sha256': sha256(b'delivered patch\n'), 'cause': 'Observed regression criterion is zero',
                    'expected_criteria': ['regression'], 'regression_risk': 'No extra authority'}}, patch
        engine.generate = generate
        if favorable:
            original_comparison = engine.comparison
            engine.comparison = lambda *args, **kwargs: ({'verdict': 'keep', 'reason': 'synthetic_favorable_comparison'}
                if args[0] == 'attempt-1' else original_comparison(*args, **kwargs))
        with patch('evolution.judge_payload_signature', return_value='fixture-config'):
            report = engine.run()
        self.assertEqual(len(generated), 1, {key: report.get(key) for key in ('reason', 'baseline_diagnostic')})
        self.assertEqual(state['candidates_used'], 1)
        self.assertEqual(len(state['attempts']), 1)
        self.assertEqual(report['reason'], 'bounded_exploration_complete')
        self.assertEqual(report['verdict'], 'inconclusive')
        self.assertEqual(state['search_best'], state['initial_baseline'])
        self.assertIsNone(state['confirmed_best'])
        self.assertFalse(state['confirmation_exposed'])
        self.assertFalse(report['effect_eligible'])
        self.assertTrue(all(row.get('split') != 'confirmation' for row in engine.rows()))
        # A known failed comparison control is usable evidence; a measurement
        # gap is not. The parent still has to satisfy its own hard floors.
        original_rows = engine.rows
        rows = copy.deepcopy(original_rows())
        control = {**copy.deepcopy(rows[0]), 'trial_id': 'failed-absence-control',
                   'candidate_id': 'absent', 'candidate_hash': 'f'*64,
                   'status': 'behavior_failure', 'score': 0, 'task_quality': 0, 'task_success': 0}
        control['checks']['task_contract'] = False
        engine.rows = lambda: rows + [control]
        self.assertTrue(engine.bounded_feedback_usable())
        control.update(score=None, task_quality=None, quality_status='unknown')
        self.assertFalse(engine.bounded_feedback_usable())
        prior = {**copy.deepcopy(rows[0]), 'trial_id': 'superseded-infrastructure',
                 'status': 'infra_invalid', 'score': None, 'task_quality': None,
                 'task_success': None, 'quality_status': 'unknown'}
        rows[0]['retry_of'] = prior['trial_id']
        engine.rows = lambda: rows + [prior]
        self.assertTrue(engine.bounded_feedback_usable())
        engine.rows = original_rows
        before = (set(remote.submissions), state['deadline'], state['trials_used'])
        restored_job, restored_state = restore_job(output)
        resumed = Experiment(restored_job, output, restored_state, remote)
        resumed.generate = lambda *args: self.fail('resume must not generate another candidate')
        self.assertEqual(resumed.run(), report)
        self.assertEqual((set(remote.submissions), restored_state['deadline'], restored_state['trials_used']), before)
