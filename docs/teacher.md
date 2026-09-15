# DL Suisho teacher

## Download

DL Suisho is by tayayan. The author's
[2025-08-17 video](https://www.youtube.com/watch?v=Z-wpDN-mBHI) links to
[DL Suisho 15b.zip](https://drive.google.com/file/d/11e9-IhuUmZ68LaWE00m-_HFacsz8RgAX/view).
The download checked for this preparation contains `DLSuisho15b/eval/model.onnx`.

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| `model.onnx` | 57,135,724 | `4c9e656f01b3ec692478965ee222a2116b73310368ed5aa499f1be1451484c0d` |

The adapter verifies the public DL Suisho 15b SHA-256 before loading.
Its 2187 outputs are mapped to the fly's 1496 internal action labels.

## Interface

- `input1`: float32 `[N, 62, 9, 9]`
- `input2`: float32 `[N, 57, 9, 9]`
- `output_policy`: logits `[N, 2187]`
- Value output: unused

The adapter uses a batch of one, on ONNX Runtime's CPU provider.
`cshogi.dlshogi.make_input_features` encodes the position. Legality and the
mapping of moves to both label formats come from `rsshogi` 1.2.4.

For every legal move `m`:

1. Obtain `c = CompactMoveLabel::from_move32(m, turn)` in `[0, 1496)`.
2. Obtain the corresponding expanded label `e = c.expand()` in `[0, 2187)`.
3. Check `e` against `cshogi.dlshogi.make_move_label(m, turn)`.
4. Read teacher logit `z[e]`.
5. Normalize **over legal moves only**: `p(m) = exp(z[e]/T) / sum_legal exp(z/T)`.
6. Store `[c, p(m)]` in the training record. The chosen move gets reward `p(m)`.

Both label spaces use the side-to-move perspective. Promotions and drops
retain distinct labels. Stable softmax subtracts the maximum legal logit.

`scripts/demo_teacher.py` is shared by training preparation and the demo.
The default teacher temperature is 1.0; set the same `--temperature` for
`prepare_teacher.py` and `serve_demo.py`. Teacher-data manifests store
its hash, temperature and output hash.

The fly selects its move using its saved synaptic gains. The teacher then
computes external feedback for that move. During play, gains remain fixed.
