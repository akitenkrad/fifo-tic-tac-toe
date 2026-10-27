**English** | [日本語](rules.ja.md)

# Rules of FIFO tic-tac-toe

[← Back to README](../README.md)

## Board and players

Cells are numbered 1 to 9 from the top left:

```text
1 | 2 | 3
--+---+--
4 | 5 | 6
--+---+--
7 | 8 | 9
```

`X` moves first unless `--first o` is given. The eight lines are the three rows, the three columns and the two diagonals. A player wins by completing a line with their own marks.

## Caps and removal

Each player `i` has a cap `k_i` on the number of marks they may keep on the board. When a player who already has `k_i` marks places another one, their **oldest** mark is removed, so each player's marks behave like a first-in, first-out queue. A player with cap `inf` never loses a mark; with `k_X = k_O = inf` the game is classical tic-tac-toe.

The program solves every pair `k_X, k_O ∈ {1, 2, 3, 4, inf}`, which gives 25 conditions per rule.

## Three timing rules

When the oldest mark disappears changes the game, so three rules are solved:

| Rule | Order within one move | Effect |
|---|---|---|
| `t1` | remove the oldest mark → place → check for a line | The mover may play on the cell that was just vacated |
| `t2` | place → check for a line → remove the oldest mark | The mark about to vanish still counts toward a line |
| `t3` | place → remove the oldest mark → check for a line | Only marks left on the board count |

`t3` is the main rule of the experiment; `t1` and `t2` are solved for comparison.

Under `t1` and `t3`, a player with cap 2 or less can never complete a line. Under `t2`, the check sees `k_i + 1` marks, so only a player with cap 1 cannot complete a line.

## End of the game

- **Line.** The game ends as soon as the mover completes a line; the mover wins. Only the mover can complete a line, because only the mover's marks change.
- **No legal move.** If the player to move has no empty cell to play on, the game is a draw. Under `t1`, a player at the cap removes a mark first, so they always have a cell to play on even when the board looks full.
- **Endless play.** Because marks disappear, a position can recur and a game may never end. A game in which neither side can force a win is a **draw**. The solver does not impose a move limit for this; positions whose value is never decided by retrograde analysis are draws (see [method](method.md)).

A position is therefore not just the board: it is the board, each player's marks **in age order**, and the player to move. Two positions with the same board but a different oldest mark can have different outcomes.

## Sensitivity variants

- **Move horizon `H`.** `solve --horizon H` replaces "endless play is a draw" with "a game not decided within `H` plies is a draw". The experiment uses `H = 20, 50, 100`.
- **Second player starts.** `solve --first o` lets `O` move first. Values are still reported from `X`'s point of view.
