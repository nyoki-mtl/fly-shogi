# 学習済みモデルの配布

[GitHub Releases](https://github.com/nyoki-mtl/fly-shogi/releases/latest)から学習済みモデルをダウンロードしてください。

| ファイル | 内容 |
| --- | --- |
| `fly-meijin.json` | 結合重みと固定入出力の設定 |
| `MODEL_CARD.md` | 学習の由来と評価結果 |
| `MODEL_LICENSE.txt` | 重みの配布条件 |
| `SHA256SUMS` | 各ファイルのSHA-256 |

標準はL-BFGS-B版です。
開発用局面で30.27%、未使用の別棋譜で26.39%の教師一致率でした。
ドーパミン学習版は専用モデルカードと追加学習用の`settings.json`を添えて、別のモデルとして配布します。

## English summary

Publish the model, model card, distribution terms and checksums as GitHub
Releases assets tied to the source version. The default is the L-BFGS-B
checkpoint. The dopamine model has a separate card and resume `settings.json`.
