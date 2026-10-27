**English** | [日本語](method.ja.md)

# Solving method and checks

[← Back to README](../README.md)

## State

A state is `(q_X, q_O, p)`: each player's marks ordered from oldest to newest, and the player to move. A side with cap `inf` never loses a mark, so its marks are stored as an unordered set. The whole state is packed into a `u64`.

## Enumeration

Starting from the empty board with `X` to move, every reachable state is enumerated breadth first, together with its forward edges (moves) and reverse edges. An edge on which the mover completes a line is stored as a winning edge. Under `t2` the completed line may vanish from the next state (the oldest mark is removed after the check), so the win is attached to the edge rather than read from the resulting board.

State counts include the empty board and terminal boards. For classical tic-tac-toe (`k_X = k_O = inf`) this gives the well-known 5,478 positions.

## Retrograde analysis

Each state keeps a counter of successors that are not yet known to be wins for the opponent.

1. A state with a winning edge is a win for the player to move.
2. When a state becomes a **loss**, every predecessor becomes a **win**.
3. When a state becomes a **win**, the counter of each predecessor is decreased; a predecessor whose counter reaches zero becomes a **loss**.
4. States are processed in breadth-first (FIFO) order, so the first distance assigned is the optimal one: the winner wins as fast as possible and the loser holds out as long as possible.
5. States that never receive a value are **draws**. This is how endless play is scored without any move limit.

The value of the initial state (from `X`'s point of view) and its distance in plies are the main results. The program also lists the optimal first moves, grouped by board symmetry.

## Move horizon

With `--horizon H`, the game is solved by backward induction on states extended with the number of remaining plies; an undecided game at zero remaining plies is a draw. This is the same recurrence as value iteration stopped after `H` rounds. The set of states from which a side can force a win within `H` plies only grows with `H`, so the value converges monotonically to the unbounded value and does not oscillate with the parity of `H`.

## Naive search

`naive` solves the same conditions with a plain negamax: no memoization, no pruning, and a child is scored as a draw when its state already appears on the current search path. Nodes are the root plus every generated child (terminal children and cut-off repeats included); for classical tic-tac-toe this is the 549,946 nodes of the full game tree. The search uses an explicit stack and stops at `--node-cap` nodes (default 10^9). Conditions that hit the cap are recorded as capped; their node count and value are not reported.

## Checks

Every run records its checks as `check.{id}` metrics (1 = pass, 0 = fail) and `checks_failed`. A failing check closes the run as `failed`; there is no option to skip checks.

| ID | Check |
|---|---|
| `v1` | Classical tic-tac-toe (`inf`, `inf`): 5,478 states, 765 after symmetry reduction, 255,168 games, under all three rules |
| `v2` | Classical tic-tac-toe is a draw |
| `v3` | Caps that cannot complete a line (≤ 2 under `t1`/`t3`, ≤ 1 under `t2`) give draws in every state; under `t2`, cap 2 can complete a line |
| `v4` | Solving the D4-canonical state graph gives, for every raw state, the same value and distance as its canonical representative |
| `v5` | An independent value iteration, sharing only the move rules and the state list, gives the same value and distance in every state |
| `v6` | Local consistency: a win at distance `n` has a move to a loss at distance `n − 1`; every move from a loss at distance `n` reaches a win at distance at most `n − 1` |
| `v7` | Optimal play by both sides, run through the socsim game loop, ends with the solved result in the solved number of plies; a drawn game ends by a repeated state or a drawn terminal, never by the safety cutoff |
| `v8` | With a horizon `H`, every state has the unbounded value if its unbounded distance is at most the remaining plies, and is a draw otherwise |
| `v9` | `O` first with caps `(k_X, k_O)` equals the negated value of `X` first with caps `(k_O, k_X)`, with the same distance |
| `v10` | Wherever the naive search finishes under the node cap, its initial value matches retrograde analysis; classical tic-tac-toe takes exactly 549,946 nodes |
| `v11` | Optimal play against a uniformly random opponent never does worse than the solved value, and wins within the solved distance |
| `r41` | No non-terminal state contains a line, and every winning edge completes exactly the mover's line |

The anchors for `v1`, `v2` and `v10` are stored in [`reference.csv`](../reference.csv).
