<p align="center"><img src="docs/assets/hero.svg" width="100%"></p>

**English** | [日本語](README.ja.md)

# fifo-tic-tac-toe

Solving tic-tac-toe where the oldest mark vanishes. Each player may keep at most `k` marks on the board; placing one more removes that player's oldest mark, first in, first out. Because marks disappear, positions can repeat and a game may never end, so ordinary backward induction does not apply. This repository enumerates every reachable state (the board plus each player's marks in age order), solves all of them by retrograde analysis with endless play scored as a draw, and does so for every cap pair `k_X, k_O ∈ {1, 2, 3, 4, ∞}` under three rules for when the oldest mark disappears. The solver is written in Rust; games are played on [socsim](https://github.com/akitenkrad/rs-social-simulation-tools) and every run is recorded with [runvault](https://github.com/akitenkrad/rs-runvault).

## Quick start

Requires a stable Rust toolchain; the figures additionally need [uv](https://docs.astral.sh/uv/). Run from the repository root:

```bash
# Solve the 25 cap pairs of the main rule (t3) and record a scratch run
cargo run --release -- solve --rule t3 --scratch

# Show one condition: value, distance and optimal first moves
cargo run --release -- show --rule t3 --kx 3 --ko 3 --scratch

# Run the tests
cargo test --release

# Draw the figures and tables from the latest scratch runs
uv sync
uv run fifo-ttt-tools plot --rule t3 --scratch
```

## Documentation

- [Rules](docs/rules.md) — caps, the three timing rules `t1`/`t2`/`t3`, how a game ends
- [Usage](docs/usage.md) — the `fifo-ttt` subcommands, scratch and production runs, the Python tools
- [Method](docs/method.md) — state representation, retrograde analysis, the move horizon, the naive search, and the checks
- [Architecture](docs/architecture.md) — source layout, the socsim game loop, and what each run records

## License

[MIT](LICENSE)
