# Dopamine-inspired checkpoint (alternative model)

This earlier local reward-learning model is retained as an alternative.
The default demo uses the L-BFGS-B model described in `docs/model-card.md`.

Weights are available as a separate bundle from [GitHub Releases](https://github.com/nyoki-mtl/fly-shogi/releases/latest), with a model card,
checksums and distribution terms. See [release layout](release.md).

| Property | Value |
| --- | --- |
| Suggested filename | `fly-meijin.json` |
| SHA-256 | `6a1ff055a7ad580e7b7df0caa486cd0c8f4e1a97c8a01d8f47d937a42787c889` |
| Format | `kc-mbon-v2`, JSON circuit gains and fixed interface configuration |
| Dataset | MaleCNS v1.0; graph hash in `config/data-sources.lock.json` source pipeline |
| Graph SHA-256 | `03a0ba851c614c5d14302fbfa12eda957299723ab290bbb71db83bb70eb3e127` |
| Cumulative processed samples / updates | 90,112 / 90,112 |
| Plastic / non-neutral edges | 61,210 / 56,290 |
| Input / output | Categorical KC input, 8 replicas; 4 time bins, fixed 1496 labels |
| Original reward teacher | Private human-policy fine-tune, legal softmax at T=1 |
| Original teacher SHA-256 | `e1d5ff676e74692506ccc1723ab7d7b08487cb3373a62a36e41074805b5ce5aa` |

## Evaluation

Selection used 45/256 (17.58%) teacher agreement. A broader 1,024-position
check, including the selection positions, gave 150/1024 (14.65%).
These rates measure matches with the original private teacher.

## Use and training

The public training path uses DL Suisho and user-supplied positions with the
same architecture and reward-learning procedure. Load saved gains with
`serve_demo.py --model PATH --teacher PATH`; gains remain fixed during play.
To continue training, retain `settings.json` beside the checkpoint and use
`train.py --resume --initial PATH` with a new output directory.

The model includes graph indices and derives from the MaleCNS data. Preserve
the FlyEM and collaborators attribution, source URL, CC BY 4.0 notice and
description of changes when distributing it. Its gains, artificial stimulus
mapping and fixed readout are additions to the biological graph.
