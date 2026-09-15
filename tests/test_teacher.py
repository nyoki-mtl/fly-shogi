"""Contract checks using generated positions, without a teacher download."""
import json
from pathlib import Path
import random
import subprocess
import sys
import tempfile
import unittest

import cshogi
import numpy as np
from cshogi.dlshogi import make_move_label

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts'))
from demo_teacher import DLSuishoTeacher, legal_softmax
from runtime import build_binary


class TeacherContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        fixtures = [json.loads(s) for s in (ROOT / 'tests/positions.jsonl').read_text().splitlines()]
        board, rng = cshogi.Board(), random.Random(73)
        # Ordinary generated positions add captures, sliders and both orientations.
        for _ in range(80):
            fixtures.append(dict(sfen=board.sfen()))
            legal = list(board.legal_moves)
            if not legal:
                break
            board.push(rng.choice(legal))
        with tempfile.TemporaryDirectory() as temp:
            source, output = Path(temp) / 'positions.jsonl', Path(temp) / 'labels.jsonl'
            source.write_text(''.join(json.dumps(r)+'\n' for r in fixtures))
            subprocess.run([str(build_binary('export-policy-labels')), str(source), str(output)], check=True)
            cls.rows = [json.loads(s) for s in output.read_text().splitlines()]

    def test_labels_match_independent_dlshogi_convention(self):
        promotions, drops, sides = set(), set(), set()
        for row in self.rows:
            board = cshogi.Board(row['sfen'])
            sides.add(board.turn)
            compact, expanded = set(), set()
            for c, e, usi in row['legal_labels']:
                self.assertEqual(make_move_label(board.move_from_usi(usi), board.turn), e)
                self.assertTrue(0 <= c < 1496 and 0 <= e < 2187)
                self.assertNotIn(c, compact)
                self.assertNotIn(e, expanded)
                compact.add(c)
                expanded.add(e)
                if usi.endswith('+'):
                    promotions.add(board.turn)
                if '*' in usi:
                    drops.add(board.turn)
        self.assertEqual(sides, {0, 1})
        self.assertEqual(promotions, sides)
        self.assertEqual(drops, sides)

    def test_mask_before_softmax_and_temperature(self):
        legal = self.rows[0]['legal_labels']
        logits = np.full(2187, 10000.0)  # Illegal labels must contribute no mass.
        logits[[e for _, e, _ in legal]] = 0
        logits[legal[-1][1]] = np.log(3)
        p = legal_softmax(logits, legal)
        self.assertAlmostEqual(p[-1], 3 / (len(legal) + 2))
        self.assertAlmostEqual(sum(p), 1)
        warmer = legal_softmax(logits, legal, 2)
        self.assertLess(warmer[-1], p[-1])
        self.assertTrue(np.all(p > 0))

    def test_invalid_outputs_are_rejected(self):
        legal = self.rows[0]['legal_labels']
        for logits, labels, temperature in [
            (np.zeros(1496), legal, 1), (np.full(2187, np.nan), legal, 1),
            (np.zeros(2187), [], 1), (np.zeros(2187), legal + legal[:1], 1),
            (np.zeros(2187), legal, 0), (np.zeros(2187), legal, float('nan')),
        ]:
            with self.assertRaises(ValueError):
                legal_softmax(logits, labels, temperature)

    def test_non_public_teacher_is_rejected_before_loading(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / 'another-model.onnx'
            path.write_bytes(b'not the public DL Suisho model')
            with self.assertRaisesRegex(ValueError, 'public DL Suisho 15b'):
                DLSuishoTeacher(path)


if __name__ == '__main__':
    unittest.main()
