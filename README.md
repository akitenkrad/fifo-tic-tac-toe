# fifo-ttt

TODO: このリポジトリが再現する対象，または検証する研究課題を記述する．Cargo workspace + uv workspace の 2 プロジェクト構成．

## 構成

- `simulation/` — Rust シミュレーション．クレート `fifo-ttt-simulation`，バイナリ `fifo-ttt`．
- `tools/` — Python 製可視化・分析ツール．パッケージ `fifo-ttt-tools`（src レイアウト，`tools/src/fifo_ttt_tools/`）．
- `results/` — シミュレーション結果の出力先（gitignore 対象）．

## ビルド & 実行

全コマンドは workspace ルート（本ディレクトリ）から実行する．

```bash
# Rust シミュレーション
cargo build --release
cargo run --release -- run --help
cargo run --release -- sweep --help

# Python ツール
uv sync
uv run fifo-ttt-tools --help
uv run fifo-ttt-tools visualize --help
uv run fifo-ttt-tools visualize-sweep --help
uv run fifo-ttt-tools show-experiment-settings --results-dir results/latest
```

`visualize-sweep` はパラメータ依存図に加えて，各パラメータ組み合わせを格子状に並べた
進行アニメーション (`sweep_grid_animation.gif`) を生成する．これは sweep を
`--snapshot-interval N` (N>0) 付きで実行した場合のみ生成される（スナップショットが
無ければ警告のみ出してスキップ）．

## テスト & Lint

```bash
cargo test
cargo clippy
cargo fmt --check
```

## 出力構造

各実行はタイムスタンプ付きサブディレクトリに保存される．`results/latest` は最新の実行へのシンボリックリンク．

```
results/
├── latest -> 20260101_120000
├── 20260101_120000/
│   ├── metrics.csv
│   ├── config.json          ← 実行時の設定値（show-experiment-settings で表示可能）
│   └── snapshots/
└── ...
```

## 参照

- 出典: TBD（再現なら元論文，新規実験なら研究計画）
- 設計ノート: TBD
