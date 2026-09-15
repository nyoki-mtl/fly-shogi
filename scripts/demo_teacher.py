"""Shared DL Suisho adapter: 2187 logits -> legal probabilities -> 1496 labels."""

import cshogi
import numpy as np
import onnxruntime as ort
from cshogi.dlshogi import make_input_features, make_move_label
from runtime import DL_SUISHO_SHA256, digest


def legal_softmax(logits, legal, temperature=1.0):
    """Select rsshogi's expanded labels BEFORE softmax; never reshape/truncate."""
    if not np.isfinite(temperature) or temperature <= 0:
        raise ValueError("Temperature must be finite and positive")
    if logits.shape != (2187,) or not np.isfinite(logits).all():
        raise ValueError("Expected 2187 finite policy logits")
    if not legal:
        raise ValueError("No legal teacher moves")
    compact = [c for c, _, _ in legal]
    expanded = [e for _, e, _ in legal]
    if (len(set(compact)) != len(compact) or len(set(expanded)) != len(expanded)
            or any(not 0 <= c < 1496 for c in compact)
            or any(not 0 <= e < 2187 for e in expanded)):
        raise ValueError("Invalid or colliding policy labels")
    z = np.asarray(logits[expanded], dtype=np.float64)
    probabilities = np.exp((z - z.max()) / temperature)
    return probabilities / probabilities.sum()


class DLSuishoTeacher:
    def __init__(self, model, temperature=1.0):
        if not np.isfinite(temperature) or temperature <= 0:
            raise ValueError("Temperature must be finite and positive")
        self.temperature = temperature
        self.sha256 = digest(model)
        if self.sha256 != DL_SUISHO_SHA256:
            raise ValueError("Expected the public DL Suisho 15b model. See README.md for its download and SHA-256.")
        options = ort.SessionOptions()
        options.intra_op_num_threads = 4
        self.session = ort.InferenceSession(str(model), sess_options=options,
                                            providers=['CPUExecutionProvider'])
        inputs = self.session.get_inputs()
        if [(i.name, i.shape[1:], i.type) for i in inputs] != [
                ('input1', [62, 9, 9], 'tensor(float)'),
                ('input2', [57, 9, 9], 'tensor(float)')]:
            raise ValueError("Expected dlshogi float32 inputs input1[N,62,9,9], input2[N,57,9,9]")
        if not any(o.name == 'output_policy' and o.shape[1:] == [2187]
                   for o in self.session.get_outputs()):
            raise ValueError("Expected output_policy[N,2187] logits")

    def policy(self, position):
        board = cshogi.Board(position['sfen'])
        legal = position['legal_labels']  # Both labels and legality come from rsshogi.
        for _, label, usi in legal:
            if make_move_label(board.move_from_usi(usi), board.turn) != label:
                raise ValueError("Teacher policy label convention mismatch")
        x1 = np.zeros((1, 62, 9, 9), np.float32)
        x2 = np.zeros((1, 57, 9, 9), np.float32)
        make_input_features(board, x1[0], x2[0])
        logits = self.session.run(['output_policy'], {'input1': x1, 'input2': x2})[0]
        if logits.shape != (1, 2187):
            raise ValueError("Invalid teacher policy logits")
        probabilities = legal_softmax(logits[0], legal, self.temperature)
        return {usi: float(p) for (_, _, usi), p in zip(legal, probabilities)}

    def feedback(self, position, move, predicted):
        probability = self.policy(position)[move]
        return {'move': move, 'external_reward': probability, 'teacher_probability': probability,
                'predicted_reward': predicted, 'prediction_error': probability - predicted}
