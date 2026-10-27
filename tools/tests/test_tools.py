from __future__ import annotations

import json
from pathlib import Path

import pandas as pd
import pytest

from fifo_ttt_tools.labels_ja import label_ja
from fifo_ttt_tools.load import load_naive_runs, load_solve_runs
from fifo_ttt_tools.plot import plot_lines, plot_table
from fifo_ttt_tools import style
from fifo_ttt_tools.table import render_naive_table, render_result_table, write_tables


def _write_solve_run(path: Path, rule: str) -> None:
    path.mkdir(parents=True)
    (path / "run.json").write_text(json.dumps({"subcommand": "solve", "run_slug": path.name}))
    (path / "config.json").write_text(
        json.dumps(
            {
                "parameters": {
                    "rule": rule,
                    "first": "x",
                    "horizon": None,
                    "k_values": [1, 2, 3, 4, "inf"],
                }
            }
        )
    )
    rows = []
    for kx in ["1", "2", "3", "4", "inf"]:
        for ko in ["1", "2", "3", "4", "inf"]:
            value = 1 if kx == "3" and ko == "3" else 0
            rows.extend(
                [
                    {"scope": "run", "step": None, "name": f"value.kx{kx}_ko{ko}", "value": value},
                    {"scope": "run", "step": None, "name": f"n_states.kx{kx}_ko{ko}", "value": 10},
                ]
            )
            if value:
                rows.append({"scope": "run", "step": None, "name": f"plies.kx{kx}_ko{ko}", "value": 7})
    pd.DataFrame(rows).to_csv(path / "metrics.csv", index=False)
    events = [
        {"schema": "x.fifo-tic-tac-toe.ply", "unit_id": "pv.kx3_ko3", "t": 1, "cell": 4},
        {"schema": "x.fifo-tic-tac-toe.game_end", "unit_id": "pv.kx3_ko3", "t": 7, "reason": "line"},
    ]
    (path / "events.jsonl").write_text("".join(json.dumps(event) + "\n" for event in events))


def _write_naive_run(path: Path, node_cap: int) -> None:
    path.mkdir(parents=True)
    (path / "config.json").write_text(
        json.dumps({"parameters": {"rule": "t3", "first": "x", "horizon": None, "node_cap": node_cap}})
    )
    pd.DataFrame(
        [
            {"scope": "run", "step": None, "name": "naive_capped.kx1_ko1", "value": 1},
            {"scope": "run", "step": None, "name": "naive_capped.kx1_ko2", "value": 0},
            {"scope": "run", "step": None, "name": "naive_nodes.kx1_ko2", "value": 42},
        ]
    ).to_csv(path / "metrics.csv", index=False)


def test_load_plot_and_table_from_runvault_layout(tmp_path: Path) -> None:
    runs = []
    for rule in ["t1", "t2", "t3"]:
        run = tmp_path / "fifo-ttt" / f"run-{rule}"
        _write_solve_run(run, rule)
        runs.append(run)

    frame = load_solve_runs(run_dirs=runs)
    assert len(frame) == 75
    assert set(frame["rule"]) == {"t1", "t2", "t3"}

    line_outputs = plot_lines(frame, runs[-1])
    table_outputs = plot_table(frame, runs[-1])
    for output in [*line_outputs, *table_outputs]:
        assert output.exists()
        assert output.suffix in {".png", ".svg"}

    markdown, csv_text = render_result_table(frame)
    assert "| Rule |" in markdown
    assert "rule,kx,ko" in csv_text
    table_files = write_tables(frame, runs[-1])
    assert len(table_files) == 8
    assert all(path.exists() for path in table_files)


def test_unknown_figure_label_is_an_error() -> None:
    with pytest.raises(KeyError):
        label_ja("not.registered")


def test_missing_japanese_font_is_an_error(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(style.font_manager.fontManager, "ttflist", [])
    with pytest.raises(RuntimeError, match="No supported Japanese font"):
        style.select_japanese_font()


def test_naive_default_selects_billion_node_run_and_explicit_cap_is_rendered(tmp_path: Path) -> None:
    small = tmp_path / "naive-small"
    full = tmp_path / "naive-full"
    _write_naive_run(small, 10_000_000)
    _write_naive_run(full, 1_000_000_000)

    selected = load_naive_runs(run_dirs=[small, full])
    assert selected["run_dir"].unique().tolist() == [str(full)]

    explicit = load_naive_runs(run_dirs=[small], node_cap=None)
    solve = pd.DataFrame(
        [
            {"rule": "t3", "kx": "1", "ko": "1", "n_states": 154},
            {"rule": "t3", "kx": "1", "ko": "2", "n_states": 200},
        ]
    )
    markdown, _ = render_naive_table(solve, explicit)
    assert label_ja("table.cap_exceeded").format(cap="10,000,000") in markdown
