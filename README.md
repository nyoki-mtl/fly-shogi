# Fly Meijin

[English README](README_en.md)

ショウジョウバエの神経接続データ「MaleCNS」を使った、将棋を指すスパイキング神経モデルです。

https://github.com/user-attachments/assets/c0ad55f8-f49c-4d77-b448-cada76cee129

キノコ体出力ニューロン（MBON）に入る既存の結合重みをL-BFGS-Bで最適化したバージョンと、ドーパミンによる可塑性を模した局所学習でKCからMBONへの結合重みを更新したバージョンの2種類を用意しています。
デフォルトはL-BFGS-B版です。

## セットアップ

[Rust](https://rustup.rs/)、[uv](https://docs.astral.sh/uv/)、[Node.js 22以降](https://nodejs.org/)が必要です。

```sh
uv sync --locked
npm ci
npm run build:board
uv run --locked python scripts/download_data.py
uv run --locked python scripts/prepare_graph.py
```

## 対局

[GitHub Releases](https://github.com/nyoki-mtl/fly-shogi/releases/latest)から`fly-meijin.json`をダウンロードし、`models/fly-meijin.json`へ配置してください。
[学習](#学習)で案内するDL水匠も配置します。
モデルの詳細は[モデルカード](docs/model-card.md)にまとめています。

```sh
uv run --locked python scripts/serve_demo.py --model models/fly-meijin.json --teacher models/dl-suisho/model.onnx
```

[http://127.0.0.1:8765/](http://127.0.0.1:8765/)でデモが立ち上がります。

神経の発光はシミュレーションの発火数を表示します。
蜜の量と**External reward**は、その手にDL水匠が割り当てた確率に対応します。
金色の波で外部報酬が脳へ届く様子を表現しています。

## 学習

教師モデルには、たややん氏の[DL水匠15b](https://drive.google.com/file/d/11e9-IhuUmZ68LaWE00m-_HFacsz8RgAX/view)を使います（[オリジナルの解説動画](https://www.youtube.com/watch?v=Z-wpDN-mBHI)）。
ZIP内の`DLSuisho15b/eval/model.onnx`を`models/dl-suisho/model.onnx`へ配置してください。

学習する局面は利用者が用意します。
JSON Lines形式で、各行に`sfen`、`split`（`train`または`validation`）、任意の整数`seed`（既定値101）を指定します。
学習用と検証用は対局単位で分けます。
入力例は[tests/positions.jsonl](tests/positions.jsonl)にあります。
以下は、このサンプルに含まれる4局面を使う動作確認の手順です。

### L-BFGS-B

```sh
uv run --locked python scripts/prepare_circuit.py
uv run --locked python scripts/prepare_teacher.py --teacher models/dl-suisho/model.onnx --positions tests/positions.jsonl
uv run --locked python scripts/train_lbfgs.py --maxiter 2
uv run --locked python scripts/evaluate.py --model work_dir/lbfgs-training/final.json --count 2
```

77,364本の既存結合を最適化します。
実際の学習では`--positions`を自分の局面ファイルに変更し、`--maxiter`で反復上限を指定します（既定値100）。
教師データの出力先を変えた場合は、学習時に`--teacher-data PATH`を渡します。
各処理の`--output`には新しいディレクトリを指定してください。

`final.json`が最終モデル、`best.json`が検証成績で選んだモデルです。
`best.json`は全回路でも再評価し、結果を`full-validation.json`へ保存します。
追加学習は`--initial PATH --output NEW_DIRECTORY`で指定できます。

### ドーパミンを模した局所学習

同じ教師データを使い、61,210本のKCからMBONへの結合を更新します。
選んだ手にDL水匠が割り当てた確率を報酬に使います。

```sh
uv run --locked python scripts/prepare_circuit.py --method dopamine --output work_dir/dopamine-circuit
uv run --locked python scripts/train.py
```

`--epochs`で反復回数、`--eta`で更新幅を指定します（既定値1、0.3）。
学習済みモデルは`work_dir/dopamine-training/final.json`へ保存します。
再開時は`settings.json`をチェックポイントと同じ場所に置き、`--resume --initial PATH --output NEW_DIRECTORY`を指定します。

どちらのモデルも、対局コマンドの`--model`へ渡せます。
詳しい仕組みは[学習方式](docs/method.md)を参照してください。

## 開発

```sh
cargo test --locked
uv run --locked python -m unittest discover -s tests -p "test_*.py"
npx tsc --noEmit
node --test tests/brain_timing.test.cjs
```

## ライセンス

[GPL-3.0-only](LICENSE)

## 参考資料と謝辞

- [MaleCNS公式プロジェクト](https://male-cns.janelia.org/)／[データの取得元](https://male-cns.janelia.org/download/)：神経接続、細胞の注釈、細胞体の座標を使用しています。
  FlyEM（HHMI Janelia）、University of Cambridge、MRC Laboratory of Molecular Biology、Google Researchと共同研究者の成果です。
- Berg et al. (2026), [Sexual dimorphism in the complete connectome of the Drosophila male central nervous system](https://research.google/pubs/sexual-dimorphism-in-the-complete-connectome-of-the-drosophila-male-central-nervous-system/), *Cell*：MaleCNSの接続図を報告した論文です。
- Shiu et al. (2024), [A Drosophila computational brain model reveals sensorimotor processing](https://doi.org/10.1038/s41586-024-07763-9), *Nature*：発火モデルの参考研究です。
  [著者の実装](https://github.com/philshiu/Drosophila_brain_model)も参照しています。
- Handler et al. (2019), [Distinct Dopamine Receptor Pathways Underlie the Temporal Sensitivity of Associative Learning](https://doi.org/10.1016/j.cell.2019.05.040), *Cell*：刺激とドーパミンの時間関係を考える際の参考研究です。
- [Doomfly](https://github.com/nftechie/doomfly)：データ変換と数値実装を検討する際に参照しました。
  入力ファイルのハッシュを照合する参考にもしています。
- [shogi-images](https://github.com/sunfish-shogi/shogi-images)：hitomoji駒画像（[CC0 1.0](web/pieces/hitomoji/LICENSE)）。
