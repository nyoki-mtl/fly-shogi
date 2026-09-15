# Fly Meijin: L-BFGS-B model (default model)

The default pretrained fly uses L-BFGS-B to optimize existing MBON input
connections. Weights are available from [GitHub Releases](https://github.com/nyoki-mtl/fly-shogi/releases/latest), with a model card,
checksums and distribution terms.

| Property | Value |
| --- | --- |
| Filename | `fly-meijin.json` |
| Size | 4,662,557 bytes |
| SHA-256 | `f7c656ffec0767b724b89ed266a827c216f496dc32258d944ebea287dcb13209` |
| Format | `mbon-input-v1`, JSON gains and fixed interface configuration |
| Method | L-BFGS-B on cached presynaptic activity, followed by full-circuit validation |
| Biological substrate | MaleCNS v1.0; 166,700 retained neurons, 25,582,938 directed edges |
| Graph SHA-256 | `03a0ba851c614c5d14302fbfa12eda957299723ab290bbb71db83bb70eb3e127` |
| Plastic connections | 77,364 existing excitatory MBON input edges |
| Neurons at plastic connections | 8,374 sources, 97 MBONs; 4,064 KC input neurons |
| Input / output | Categorical KC input, 8 replicas; 4 time bins, fixed 1496 labels |
| Cumulative sample counter / update counter | 26,272,864 / 227; counts include repeated optimizer passes |
| Original teacher | YaneuraOu teacher moves in the private experiment corpus |
| Public teacher for new training and demo feedback | Separately downloaded public DL Suisho 15b |

## Evaluation

The selected checkpoint used a 500,000-position training collection and earlier
training stages. Full-circuit evaluation, without teacher assistance at inference:

| Split | Top-1 teacher agreement | Cross-entropy at T=0.02 |
| --- | --- | --- |
| Development, used for selection | 310 / 1,024 = **30.27%** | 2.69974 |
| Previously unused games, checked after selection | 1,081 / 4,096 = **26.39%** | 2.87322 |

The independent set contains positions from 568 games, separate from training
and development. These agreement rates measure matches with the original
teacher. New training uses public DL Suisho and user-supplied positions.

## Use and training

Load with `serve_demo.py --model PATH --teacher PATH`. The fly chooses legal
moves from saved gains. DL Suisho supplies external feedback, illustrated by
nectar and a gold wave. Gains remain fixed during play.

`prepare_circuit.py` and `train_lbfgs.py` implement the architecture and
optimization method. Further training uses `--initial PATH`, user-supplied
teacher data and a new output directory. Activity is recomputed from the model.
The historical checkpoint used a private corpus and several initialization stages.

The alternative local reward-learning model has its own teacher, training
history and evaluation sets, described in `docs/dopamine-model-card.md`.

## Attribution

Derived from [MaleCNS](https://male-cns.janelia.org/) data by FlyEM (HHMI Janelia),
University of Cambridge, MRC Laboratory of Molecular Biology, Google Research
and collaborators, under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
Changes include graph indexing, artificial input/output interfaces and learned
synaptic gains. Preserve this attribution and the description of changes.
