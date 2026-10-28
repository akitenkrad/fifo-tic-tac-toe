[English](usage.md) | **日本語**

# 使い方

[← README に戻る](../README.ja.md)

## 必要なもの

- 安定版の Rust ツールチェーン (`simulation/` 用)．
- [uv](https://docs.astral.sh/uv/) と Python 3.11 以上 (`tools/` の図と表用)．
- run をディレクトリではなく名前で引きたい場合は runvault の CLI．

## scratch と本番の run

`fifo-ttt` のどのサブコマンドも，`--scratch` と `--prod` のどちらか一方を必ず指定する．開発中や試行では `--scratch` を使う．scratch の run は `results/_scratch/` の下に書かれ，本番の解析からは外れる．`--prod` は `results/fifo-ttt/` に書く．`show` は標準出力に出すだけで，run を作らない．

## Rust の CLI (`fifo-ttt`)

リポジトリの直下で実行する．

```bash
cargo build --release
cargo test --release
```

### `solve`: 1 つの規則を解く

1 つの規則の 25 条件をすべて解き，最善手どうしの手順を socsim で回し，検査を走らせて run を 1 つ記録する．

```bash
cargo run --release -- solve --rule t3 --scratch
cargo run --release -- solve --rule t3 --horizon 20 --scratch   # games unresolved after 20 plies are draws
cargo run --release -- solve --rule t3 --first o --scratch      # O moves first
```

| オプション | 意味 |
|---|---|
| `--rule {t1,t2,t3}` | 消えるタイミングの規則 ([ルール](rules.ja.md) を参照) |
| `--first {x,o}` | 先手 (既定 `x`) |
| `--horizon N` | `N` 手で決着しなければ引き分けとする．付けなければ終わらない対局を引き分けとする |

### `all`: 全体の sweep

3 規則 × {上限なし，`H = 20`，`50`，`100`，`O` 先手} の 15 点の sweep を回す．親 run 1 つと子 run 15 個を記録し，子 run ごと・条件ごとに進捗を出す．

```bash
cargo run --release -- all --scratch
```

### `naive`: 素朴な探索

1 つの規則の 25 条件を，探索中の経路上の繰り返しで打ち切る素朴なネガマックスで解く ([解き方](method.ja.md#素朴な探索) を参照)．局面のグラフに閉路がある条件は，たいていノード数の上限を超える．既定の上限 10^9 ノードで 1 規則を回すと数分かかる．

```bash
cargo run --release -- naive --rule t3 --scratch
cargo run --release -- naive --rule t3 --node-cap 10000000 --scratch
```

| オプション | 意味 |
|---|---|
| `--rule {t1,t2,t3}` | 消えるタイミングの規則 |
| `--node-cap N` | 1 条件あたりに生成する探索木のノード数の上限 (既定 `1000000000`) |

### `play`: 最善手 対 ランダム

すべての条件で，最善手の方策と一様ランダムの相手を先後入れ替えて対局させる．

```bash
cargo run --release -- play --scratch
```

| オプション | 意味 |
|---|---|
| `--rule {t1,t2,t3}` | 消えるタイミングの規則 (既定 `t3`) |
| `--x`，`--o {optimal,random}` | 各プレイヤーの方策 (既定は `optimal` と `random`) |
| `--games N` | 条件と先後の組ごとの局数 (既定 `100`) |
| `--seed S` | 親 seed．対局ごとの seed はここから導く (既定 `20261027`) |
| `--max-plies N` | 打ち切りの手数 (引き分けとして数える)．既定では条件ごとに到達局面数 + 1 |

### `show`: 1 条件を確かめる

1 条件の値・手数・初手の最善手を，run を作らずに表示する．

```bash
cargo run --release -- show --rule t3 --kx 3 --ko 3 --scratch
```

`--kx` と `--ko` には `1`，`2`，`3`，`4`，`inf` を渡す．

## Python のツール (`fifo-ttt-tools`)

ツールは runvault を通して run を読み，図と表を run ディレクトリの中ではなく隣 (`results/<実験名>/figures/<run>/`) に書く．

```bash
uv sync
uv run pytest
uv run fifo-ttt-tools plot --rule t3 --scratch
uv run fifo-ttt-tools table --scratch
uv run fifo-ttt-tools table --naive --scratch
```

| コマンド | 出力 |
|---|---|
| `plot` | 図の 2 案 (PNG と SVG)．(a) 両者共通の上限 `k` に対する値と決着手数を規則ごとの線で重ねたもの，(b) `--rule` で選んだ規則の値を塗り分け，手数を記した 5 × 5 の表．図のラベルは日本語 |
| `table` | Markdown と CSV の表．値と手数，手数の上限の感度，先手の入れ替え，最善手の手順 |
| `table --naive` | 到達局面数と素朴な探索のノード数を並べた表を加える (既定の上限 10^9 で回した run だけを使う) |
| `show-experiment-settings --results-dir DIR` | run の設定を表示する |

`plot` と `table` は最新の run を自動で選ぶ．特定の run を使うときは `--run-dir` (と `--naive-run-dir`) を渡す．`--scratch` を付けなければ本番の run を読む．
