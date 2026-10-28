**English** | [日本語](usage.ja.md)

# Usage

[← Back to README](../README.md)

## Requirements

- A stable Rust toolchain (for `simulation/`).
- [uv](https://docs.astral.sh/uv/) and Python 3.11 or later (for the figures and tables in `tools/`).
- The runvault CLI, if you want to look up runs by name rather than by directory.

## Scratch and production runs

Every subcommand of `fifo-ttt` requires exactly one of `--scratch` or `--prod`. Use `--scratch` while developing or exploring: scratch runs are written under `results/_scratch/` and are excluded from production analyses. `--prod` writes to `results/fifo-ttt/`. `show` prints to standard output and does not create a run.

## Rust CLI (`fifo-ttt`)

Run from the repository root:

```bash
cargo build --release
cargo test --release
```

### `solve`: solve one rule

Solves all 25 cap pairs for one rule, plays the optimal line through socsim, runs the checks and records one run.

```bash
cargo run --release -- solve --rule t3 --scratch
cargo run --release -- solve --rule t3 --horizon 20 --scratch   # games unresolved after 20 plies are draws
cargo run --release -- solve --rule t3 --first o --scratch      # O moves first
```

| Option | Meaning |
|---|---|
| `--rule {t1,t2,t3}` | Timing rule (see [rules](rules.md)) |
| `--first {x,o}` | Player that moves first (default `x`) |
| `--horizon N` | Treat games unresolved after `N` plies as draws; without it, endless play is a draw |

### `all`: the full sweep

Runs the 15-point sweep: 3 rules × {no horizon, `H = 20`, `50`, `100`, `O` first}. One parent run and 15 child runs are recorded; progress is printed per child run and per condition.

```bash
cargo run --release -- all --scratch
```

### `naive`: naive search

Solves the 25 cap pairs of one rule with a plain negamax that cuts off repeated states on the current path (see [method](method.md#naive-search)). Conditions whose state graph has cycles typically exceed the node cap; a full run with the default cap of 10^9 nodes takes a few minutes.

```bash
cargo run --release -- naive --rule t3 --scratch
cargo run --release -- naive --rule t3 --node-cap 10000000 --scratch
```

| Option | Meaning |
|---|---|
| `--rule {t1,t2,t3}` | Timing rule |
| `--node-cap N` | Maximum generated search-tree nodes per cap pair (default `1000000000`) |

### `play`: optimal against random

Plays the optimal policy against a uniformly random opponent in both seat assignments, for every cap pair.

```bash
cargo run --release -- play --scratch
```

| Option | Meaning |
|---|---|
| `--rule {t1,t2,t3}` | Timing rule (default `t3`) |
| `--x`, `--o {optimal,random}` | Policy of each player (defaults `optimal` and `random`) |
| `--games N` | Games per cap pair and seat assignment (default `100`) |
| `--seed S` | Master seed; one seed per game is derived from it (default `20261027`) |
| `--max-plies N` | Ply cutoff, counted as a draw; by default each condition uses its reachable-state count plus one |

### `show`: inspect one condition

Prints the value, distance and optimal first moves of one condition without creating a run.

```bash
cargo run --release -- show --rule t3 --kx 3 --ko 3 --scratch
```

`--kx` and `--ko` take `1`, `2`, `3`, `4` or `inf`.

## Python tools (`fifo-ttt-tools`)

The tools read runs through runvault and write figures and tables next to, not inside, the run directories (`results/<experiment>/figures/<run>/`).

```bash
uv sync
uv run pytest
uv run fifo-ttt-tools plot --rule t3 --scratch
uv run fifo-ttt-tools table --scratch
uv run fifo-ttt-tools table --naive --scratch
```

| Command | Output |
|---|---|
| `plot` | Two figure alternatives, PNG and SVG: (a) value and decisive distance against the common cap `k`, with one line per rule, (b) a value-colored 5 × 5 table with distance for the rule selected by `--rule`. Figure labels are in Japanese |
| `table` | Markdown and CSV tables: values and distances, horizon sensitivity, first-player swap, optimal lines |
| `table --naive` | Adds the comparison of reachable states and naive-search nodes (only runs with the default node cap of 10^9 are used) |
| `show-experiment-settings --results-dir DIR` | Prints the configuration of a run |

`plot` and `table` pick the latest runs automatically; pass `--run-dir` (and `--naive-run-dir`) to use specific runs. Without `--scratch` they read production runs.
