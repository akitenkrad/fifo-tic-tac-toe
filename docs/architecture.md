**English** | [日本語](architecture.ja.md)

# Architecture

[← Back to README](../README.md)

The repository is a Cargo workspace (`simulation/`, the Rust solver and CLI) plus a uv workspace (`tools/`, the Python figures and tables).

## Rust modules (`simulation/src/`)

| Module | Role |
|---|---|
| `config.rs` | Rules `t1`/`t2`/`t3`, caps (`1`–`4`, `inf`), conditions (rule, caps, first player, horizon) |
| `state.rs` | Packed state (`u64`): each player's marks in age order and the player to move; the eight lines |
| `rules.rs` | Legal moves and the single transition function `step`. **The only place the game rules live** |
| `graph.rs` | Breadth-first enumeration of reachable states with forward and reverse edges |
| `retro.rs` | Retrograde analysis: values and distances |
| `iterate.rs` | Value iteration (independent check) and the horizon-`H` solver |
| `symmetry.rs` | D4 board symmetries and canonical states |
| `analysis.rs` | Optimal first moves grouped by symmetry |
| `naive.rs` | Naive negamax with path-repetition cut-off and a node cap |
| `checks.rs` | Checks `v1`–`v11` and the reference anchors |
| `world.rs`, `mechanisms.rs`, `policy.rs`, `simulation.rs` | Playing games on the socsim engine |
| `record.rs` | Writing runs, metrics, events and artifacts to runvault |
| `main.rs` | The `fifo-ttt` command-line interface |

The solver, the naive search and the socsim games all call `rules::legal_moves` and `rules::step`, so the three timing rules are implemented once.

## Games on socsim

Games are played with [socsim](https://github.com/akitenkrad/rs-social-simulation-tools) (`socsim-core`, `socsim-engine`):

- `GameWorld` is the world state of one game.
- Three mechanisms run each step: `ChooseMove` (decision phase) asks the current player's policy for a cell, `ApplyMove` (interaction phase) applies `rules::step`, and `Referee` (post-step phase) ends the game on a line, on no legal move, or on a repeated state.
- A policy receives an observation and returns a cell. `OptimalPolicy` plays the solved optimal moves, `UniformRandomPolicy` picks a legal move uniformly with the run's seeded RNG, and `ScriptedPolicy` replays a fixed sequence in tests.

Optimal-versus-optimal games produce the principal variation recorded by `solve`; optimal-versus-random games are run by `play`.

## Recording with runvault

Every subcommand except `show` records its results with [runvault](https://github.com/akitenkrad/rs-runvault): one run each, except `all`, which writes a sweep parent and 15 child runs; game events go through [runvault-socsim](https://github.com/akitenkrad/rs-runvault-socsim). A run directory contains:

| File | Contents |
|---|---|
| `config.json` | Rule, first player, horizon, caps, and (for `naive`/`play`) node cap, number of games, policies and seed |
| `metrics.csv` | One row per metric; per-condition metrics are named `{metric}.kx{a}_ko{b}` (for example `value.kx3_ko3`) |
| `events.jsonl` | Moves (`x.fifo-tic-tac-toe.ply`) and game ends (`x.fifo-tic-tac-toe.game_end`) of the recorded games |
| `artifacts/best_moves.csv` | Optimal first moves per condition (`kx, ko, class_id, cells, value_after_x, plies_after`; cells are 0-based and `|`-separated) |
| `logs/` | Progress and check details |

The metric names and their meaning are declared in [`runvault.toml`](../runvault.toml). Values are reported from `X`'s point of view (`+1` win, `0` draw, `−1` loss). `plies.{cond}` is omitted for draws, `n_games.{cond}` is omitted when the state graph has cycles, and `naive_nodes.{cond}` is omitted when the naive search hit the node cap.
