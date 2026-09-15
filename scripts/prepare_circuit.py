"""Prepare neutral existing synapses for L-BFGS-B or dopamine learning."""
import json
import argparse
import struct
from pathlib import Path
import numpy as np
import pyarrow.feather as feather

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--method', choices=['lbfgs', 'dopamine'], default='lbfgs')
    parser.add_argument('--data-root', type=Path, default=ROOT)
    parser.add_argument('--output', type=Path, default=ROOT / 'work_dir/circuit')
    parser.add_argument('--strong-fraction', type=float, default=0.0)
    args = parser.parse_args()
    if not 0 <= args.strong_fraction <= 1:
        parser.error('strong-fraction must be in [0,1]')
    out = args.output
    out.mkdir(parents=True, exist_ok=False)
    graph = args.data_root / 'work_dir/malecns-csr/graph.bin'
    with graph.open('rb') as f:
        magic, n, e = struct.unpack('<8sQQ', f.read(24))
    assert magic == b'FLYCSR01'
    ids = np.memmap(graph, '<u8', 'r', offset=24, shape=(n,))
    offsets = np.memmap(graph, '<u8', 'r', offset=24+8*n, shape=(n+1,))
    posts = np.memmap(graph, '<u4', 'r', offset=24+8*n+8*(n+1), shape=(e,))
    weights = np.memmap(graph, '<i4', 'r', offset=24+8*n+8*(n+1)+4*e, shape=(e,))
    rows = feather.read_table(args.data_root / 'data/raw/malecns-v1.0/body-annotations-male-cns-v1.0-minconf-0.5.feather', columns=['bodyId', 'class', 'type']).to_pylist()
    lookup = {int(body): i for i, body in enumerate(ids)}
    kc = sorted(lookup[r['bodyId']] for r in rows if r['class'] == 'Kenyon_Cell' and r['bodyId'] in lookup)
    mbon = sorted(lookup[r['bodyId']] for r in rows if r['class'] == 'MBON' and r['bodyId'] in lookup)
    slot = {node: i for i, node in enumerate(mbon)}
    edges, pres, groups = [], [], []
    for neuron in kc:
        for edge in range(int(offsets[neuron]), int(offsets[neuron+1])):
            post = int(posts[edge])
            if post in slot and weights[edge] > 0:
                edges.append(edge)
                pres.append(neuron)
                groups.append(slot[post])
    model = dict(version='kc-mbon-v2', kc=kc, mbon=mbon, edges=edges,
                 pres=pres, groups=groups, gains=[1.0]*len(edges), code_seed=73, steps=400)
    model.update(samples=0, updates=0, time_bins=4, categorical_input=True,
                 kc_input=True, kc_replicas=8, structured_code=True)
    if args.method == 'lbfgs':
        edges = np.flatnonzero(np.isin(posts, mbon) & (weights > 0))
        pres = np.searchsorted(offsets, edges, side='right') - 1
        model.update(version='mbon-input-v1', sources=np.unique(pres).tolist(),
                     edges=edges.tolist(), pres=pres.tolist(),
                     groups=[slot[int(posts[e])] for e in edges], gains=[1.0]*len(edges))
        (out / 'initial.json').write_text(json.dumps(model, separators=(',', ':')), encoding='utf-8')
        print(json.dumps(dict(method='lbfgs', plastic_edges=len(edges), sources=len(model['sources']), mbon=len(mbon))))
        return
    slot = {node: i for i, node in enumerate(model['mbon'])}
    channels = []
    annotations = []
    for row in rows:
        if row['class'] != 'DAN' or row['bodyId'] not in lookup:
            continue
        neuron = lookup[row['bodyId']]
        targets = []
        for edge in range(int(offsets[neuron]), int(offsets[neuron+1])):
            post = int(posts[edge])
            if post in slot and weights[edge] != 0:
                targets.append((slot[post], abs(int(weights[edge]))))
        if targets:
            cutoff = max(w for _, w in targets) * args.strong_fraction
            targets = [(g, w) for g, w in targets if w >= cutoff]
            total = sum(w for _, w in targets)
            channels.append(dict(neuron=neuron, targets=[(g, w/total) for g, w in targets]))
            annotations.append(dict(neuron=neuron, body_id=row['bodyId'], type=row['type']))
    covered = sorted({g for channel in channels for g, _ in channel['targets']})
    for name, data in [('initial.json', model), ('channels.json', channels),
                       ('anatomy.json', dict(channels=annotations, covered_mbon_groups=covered,
                        strong_fraction=args.strong_fraction,
                        targeting='DAN targets at or above strong_fraction of strongest contact; normalized within each DAN; targeting proxy, not verified compartment annotation'))]:
        (out / name).write_text(json.dumps(data, separators=(',', ':')), encoding='utf-8')
    print(json.dumps(dict(plastic_edges=len(model['edges']), channels=len(channels), covered_mbon=len(covered), mbon=len(slot))))


if __name__ == '__main__':
    main()
