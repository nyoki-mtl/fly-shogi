"""Integration check against a running demo, with an explicitly supplied teacher."""
import argparse
import json
from pathlib import Path
import sys
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts'))
from demo_teacher import DLSuishoTeacher
from runtime import digest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--url', default='http://127.0.0.1:8765')
    parser.add_argument('--teacher', type=Path, required=True)
    parser.add_argument('--model', type=Path, required=True)
    parser.add_argument('--teacher-data', type=Path, required=True)
    args = parser.parse_args()
    model_hash = digest(args.model)
    manifest = json.loads((args.teacher_data / 'manifest.json').read_text())
    teacher = DLSuishoTeacher(args.teacher, temperature=manifest['temperature'])
    assert teacher.sha256 == manifest['teacher_sha256']

    def api(action, moves=None, **extra):
        request = urllib.request.Request(args.url + '/api',
            data=json.dumps(dict(action=action, moves=moves or [], **extra)).encode(),
            headers={'Content-Type': 'application/json'})
        return json.load(urllib.request.urlopen(request, timeout=60))

    def rejected(action, moves=None, **extra):
        try:
            api(action, moves, **extra)
        except urllib.error.HTTPError as error:
            assert error.code == 400
        else:
            raise AssertionError('Invalid action was accepted')

    maximum_error = 0
    rows = [json.loads(s) for s in (args.teacher_data / 'teacher.jsonl').read_text().splitlines()]
    for row in rows:
        state = api('state', sfen=row['sfen'])['state']
        expected = dict(row['teacher_policy'])
        probabilities = teacher.policy(state)
        for c, _, move in state['legal_labels']:
            maximum_error = max(maximum_error, abs(probabilities[move] - expected[c]))
    assert maximum_error < 1e-12
    rejected('think')
    rejected('play', move_usi='7g7d')
    exercise = '4k4/9/4P4/9/9/9/9/9/4K4 b R2P2p 1'
    for move, square, piece in [('5c5b+', '5b', '+P'), ('R*2e', '2e', 'R')]:
        state = api('play', sfen=exercise, move_usi=move)['state']
        assert any(c['square'] == square and c['piece'] == piece for c in state['cells'])
    moves, rewards = [], []
    for _ in range(3):
        near = api('state', moves)['state']
        probabilities = teacher.policy(near)
        human = api('play', moves, move_usi=max(probabilities, key=probabilities.get))
        moves = human['state']['moves']
        rejected('teacher', moves)
        rejected('play', moves, move_usi=human['state']['legal'][0])
        fly = api('think', moves)
        assert fly['move'] in human['state']['legal']
        expected = teacher.policy(human['state'])[fly['move']]
        assert fly['feedback']['external_reward'] == expected
        assert len(fly['telemetry']['brain_activity']) == 4
        moves = fly['state']['moves']
        rewards.append(expected)
    assert digest(args.model) == model_hash
    print(json.dumps(dict(legal_plies=len(moves), teacher_max_error=maximum_error,
                         rewards=rewards, model_unchanged=True), indent=2))


if __name__ == '__main__':
    main()
