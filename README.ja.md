# FIFO 三目並べ

このリポジトリは，手の保持上限を持つ三目並べを，3 種類の手順規則について全列挙して解く．Rust 実装は手を置いた順序を局面に保持し，`{1, 2, 3, 4, infinity}` の全上限ペアを解析し，独立な方法で解を検査して，runvault に記録する．

## コマンド

```bash
cargo build --release
cargo test --release
cargo run --release -- solve --rule t3 --scratch
cargo run --release -- show --rule t3 --kx 3 --ko 3 --scratch
```

すべてのコマンドで `--scratch` と `--prod` のどちらか一方を必ず指定する．開発中は `--scratch` を使う．`show` は run を作らない．

公開実装は `simulation/` に置く．`tools/` パッケージは，後続の解析・可視化フェーズで扱う．
