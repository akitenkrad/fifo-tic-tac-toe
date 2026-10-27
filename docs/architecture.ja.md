[English](architecture.md) | **日本語**

# 構成

[← README に戻る](../README.ja.md)

リポジトリは Cargo workspace (`simulation/`．Rust のソルバと CLI) と uv workspace (`tools/`．Python の図と表) からなる．

## Rust のモジュール (`simulation/src/`)

| モジュール | 役割 |
|---|---|
| `config.rs` | 規則 `t1`/`t2`/`t3`，上限 (`1`〜`4`，`inf`)，条件 (規則・上限・先手・手数の上限) |
| `state.rs` | `u64` に詰めた局面 (各プレイヤーの手の古い順と手番)，8 本の一列 |
| `rules.rs` | 合法手と，ただ 1 つの遷移関数 `step`．**ゲームの規則はここにしか無い** |
| `graph.rs` | 到達局面の幅優先の列挙と，前向き・逆向きの辺 |
| `retro.rs` | 後退解析 (値と手数) |
| `iterate.rs` | 値反復 (独立な検査) と手数の上限 `H` 版のソルバ |
| `symmetry.rs` | 盤の D4 対称性と局面の正準化 |
| `analysis.rs` | 対称性でまとめた初期局面の最善手 |
| `naive.rs` | 経路上の繰り返しで打ち切る素朴なネガマックス (ノード数の上限つき) |
| `checks.rs` | 検査 `v1`〜`v11` とアンカーの照合 |
| `world.rs`，`mechanisms.rs`，`policy.rs`，`simulation.rs` | socsim のエンジンで対局を回す部分 |
| `record.rs` | run・指標・事象・成果物を runvault に書く |
| `main.rs` | CLI `fifo-ttt` |

ソルバ・素朴な探索・socsim の対局は，どれも `rules::legal_moves` と `rules::step` を呼ぶ．3 つの規則の実装は 1 か所だけである．

## socsim での対局

対局は [socsim](https://github.com/akitenkrad/rs-social-simulation-tools) (`socsim-core`，`socsim-engine`) で回す．

- `GameWorld` が 1 局の世界の状態である．
- 各ステップで 3 つの Mechanism が動く．`ChooseMove` (Decision) が手番の側の方策にマスを尋ね，`ApplyMove` (Interaction) が `rules::step` を適用し，`Referee` (PostStep) が一列・打てる手が無い・局面の繰り返しで対局を終える．
- 方策は観測を受け取ってマスを返す．`OptimalPolicy` は解いた最善手を打ち，`UniformRandomPolicy` は run の seed から作った乱数で合法手を一様に選び，`ScriptedPolicy` はテストで決まった手順を再生する．

最善手どうしの対局は `solve` が記録する手順 (principal variation) になり，最善手 対 ランダムの対局は `play` が回す．

## runvault への記録

`show` 以外のサブコマンドは [runvault](https://github.com/akitenkrad/rs-runvault) に結果を記録する．run はサブコマンドごとに 1 つで，`all` だけは sweep の親 run と子 run 15 個を書く．対局の事象は [runvault-socsim](https://github.com/akitenkrad/rs-runvault-socsim) を通して書く．run ディレクトリの中身は次のとおり．

| ファイル | 中身 |
|---|---|
| `config.json` | 規則・先手・手数の上限・上限の値．`naive`/`play` ではノード数の上限・局数・方策・seed |
| `metrics.csv` | 指標 1 つにつき 1 行．条件ごとの指標は `{指標}.kx{a}_ko{b}` (例 `value.kx3_ko3`) |
| `events.jsonl` | 記録した対局の着手 (`x.fifo-tic-tac-toe.ply`) と終局 (`x.fifo-tic-tac-toe.game_end`) |
| `artifacts/best_moves.csv` | 条件ごとの初手の最善手 (`kx, ko, class_id, cells, value_after_x, plies_after`．`cells` は 0 始まりで `|` 区切り) |
| `logs/` | 進捗と検査の詳細 |

指標の名前と意味は [`runvault.toml`](../runvault.toml) に宣言している．値は `X` から見て書く (`+1` 勝ち，`0` 引き分け，`−1` 負け)．`plies.{cond}` は引き分けなら書かず，`n_games.{cond}` は局面のグラフに閉路があれば書かず，`naive_nodes.{cond}` は素朴な探索が上限に達したら書かない．
