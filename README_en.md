# Fly Meijin

[日本語 README](README.md)

Play shogi against a spiking neural model built on the MaleCNS fly connectome.

Two versions are available: one optimizes existing mushroom body output neuron
(MBON) input connections with L-BFGS-B; the other updates KC-to-MBON connections
through dopamine-inspired local learning. L-BFGS-B is the default.
Its teacher move agreement was **30.27%** on development positions and **26.39%**
on previously unused games checked after selection.

## Setup

[Rust](https://rustup.rs/), [uv](https://docs.astral.sh/uv/), and
[Node.js 22+](https://nodejs.org/) are required.

```sh
uv sync --locked
npm ci
npm run build:board
uv run --locked python scripts/download_data.py
uv run --locked python scripts/prepare_graph.py
```

## Play

Download `fly-meijin.json` from [GitHub Releases](https://github.com/nyoki-mtl/fly-shogi/releases/latest)
and save it as `models/fly-meijin.json`. Obtain DL Suisho as described under
[Training](#training). See the [model card](docs/model-card.md) for details.

```sh
uv run --locked python scripts/serve_demo.py --model models/fly-meijin.json --teacher models/dl-suisho/model.onnx
```

The demo runs at [localhost:8765](http://127.0.0.1:8765/).

Neural glow displays simulated spike counts. The nectar amount and
**External reward** show DL Suisho's probability for the fly's chosen move.
A gold wave illustrates the external reward reaching the brain.

## Training

Use tayayan's [DL Suisho 15b](https://drive.google.com/file/d/11e9-IhuUmZ68LaWE00m-_HFacsz8RgAX/view)
([distribution announcement](https://www.youtube.com/watch?v=Z-wpDN-mBHI)).
Extract `DLSuisho15b/eval/model.onnx` to `models/dl-suisho/model.onnx`.

Supply your own positions as JSON Lines, with `sfen`, `split` (`train` or
`validation`), and an optional integer `seed` (default 101). Split by game.
See [tests/positions.jsonl](tests/positions.jsonl) for sample input.
The following commands use its four positions for an installation check.

### L-BFGS-B

```sh
uv run --locked python scripts/prepare_circuit.py
uv run --locked python scripts/prepare_teacher.py --teacher models/dl-suisho/model.onnx --positions tests/positions.jsonl
uv run --locked python scripts/train_lbfgs.py --maxiter 2
uv run --locked python scripts/evaluate.py --model work_dir/lbfgs-training/final.json --count 2
```

This optimizes 77,364 existing connections. For training, replace `--positions`
with your own file and set the iteration limit with `--maxiter` (default 100).
Pass `--teacher-data PATH` if you changed the teacher output directory.
Choose a new `--output` directory for each run.

`final.json` is the last model; `best.json` is selected by validation performance.
The selected model is also evaluated with the full circuit in `full-validation.json`.
Continue with `--initial PATH --output NEW_DIRECTORY`.

### Dopamine-inspired local learning

Use the same teacher data to update 61,210 KC-to-MBON connections, with the
teacher's probability for the sampled move as the reward.

```sh
uv run --locked python scripts/prepare_circuit.py --method dopamine --output work_dir/dopamine-circuit
uv run --locked python scripts/train.py
```

`--epochs` sets the number of passes and `--eta` the update scale (defaults 1
and 0.3). The model is saved to `work_dir/dopamine-training/final.json`.
To resume, keep `settings.json` beside the checkpoint and pass
`--resume --initial PATH --output NEW_DIRECTORY`.

Pass either model to the demo's `--model` option.
See [learning methods](docs/method.md) for details.

## Development

```sh
cargo test --locked
uv run --locked python -m unittest discover -s tests -p "test_*.py"
npx tsc --noEmit
node --test tests/brain_timing.test.cjs
```

## License

[GPL-3.0-only](LICENSE)

## References and acknowledgements

- [MaleCNS project](https://male-cns.janelia.org/) / [official downloads](https://male-cns.janelia.org/download/): connectivity, cell annotations and soma coordinates. Credit to FlyEM (HHMI Janelia), University of Cambridge, MRC Laboratory of Molecular Biology, Google Research and collaborators.
- Berg et al. (2026), [Sexual dimorphism in the complete connectome of the Drosophila male central nervous system](https://research.google/pubs/sexual-dimorphism-in-the-complete-connectome-of-the-drosophila-male-central-nervous-system/), *Cell*: the MaleCNS connectome.
- Shiu et al. (2024), [A Drosophila computational brain model reveals sensorimotor processing](https://doi.org/10.1038/s41586-024-07763-9), *Nature*: a reference for spiking dynamics, alongside the [authors' implementation](https://github.com/philshiu/Drosophila_brain_model).
- Handler et al. (2019), [Distinct Dopamine Receptor Pathways Underlie the Temporal Sensitivity of Associative Learning](https://doi.org/10.1016/j.cell.2019.05.040), *Cell*: background for stimulus/reward timing.
- [Doomfly](https://github.com/nftechie/doomfly): a reference when considering data conversion and numerical implementation, and a cross-check for input file hashes.
