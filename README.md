# FIFO Tic-Tac-Toe

This repository enumerates and solves finite-memory tic-tac-toe under three move-order rules. The Rust implementation keeps move order in the game state, solves every cap pair in `{1, 2, 3, 4, infinity}`, checks the solver with independent methods, and records runs through runvault.

## Commands

```bash
cargo build --release
cargo test --release
cargo run --release -- solve --rule t3 --scratch
cargo run --release -- show --rule t3 --kx 3 --ko 3 --scratch
```

Every command requires exactly one of `--scratch` or `--prod`. Use `--scratch` during development. `show` does not create a run.

The public implementation is in `simulation/`. The `tools/` package is reserved for later analysis and visualization work.
