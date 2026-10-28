from __future__ import annotations

import json
from pathlib import Path
import re

from matplotlib import colors
import numpy as np
import pandas as pd
import pytest

from fifo_ttt_tools import plot as plot_module
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


def _figure_frame() -> pd.DataFrame:
    diagonal = {
        "t1": [(0, None), (1, 7), (0, None), (1, 9), (0, None)],
        "t2": [(0, None), (1, 5), (1, 9), (1, 11), (0, None)],
        "t3": [(0, None), (0, None), (1, 13), (1, 11), (0, None)],
    }
    rows = []
    for rule_index, rule in enumerate(["t1", "t2", "t3"]):
        for row_index, kx in enumerate(["1", "2", "3", "4", "inf"]):
            for column_index, ko in enumerate(["1", "2", "3", "4", "inf"]):
                value = [-1, 0, 1][(rule_index + row_index + column_index) % 3]
                plies = None if value == 0 else 5 + row_index + column_index
                if kx == ko:
                    value, plies = diagonal[rule][row_index]
                rows.append(
                    {
                        "rule": rule,
                        "first": "x",
                        "horizon": None,
                        "kx": kx,
                        "ko": ko,
                        "value": value,
                        "plies": plies,
                    }
                )
    return pd.DataFrame(rows)


def _capture_figures(monkeypatch: pytest.MonkeyPatch) -> list[tuple[object, str]]:
    captured: list[tuple[object, str]] = []

    def capture(fig: object, output_dir: Path, stem: str) -> tuple[Path, Path]:
        captured.append((fig, stem))
        return output_dir / f"{stem}.png", output_dir / f"{stem}.svg"

    monkeypatch.setattr(plot_module, "_save_both", capture)
    return captured


def _visible_texts(fig: object) -> list[str]:
    texts = [text.get_text() for text in fig.texts]
    for axis in fig.axes:
        texts.extend([axis.get_title(), axis.get_xlabel(), axis.get_ylabel()])
        texts.extend(text.get_text() for text in axis.texts)
        texts.extend(text.get_text() for text in axis.get_xticklabels())
        texts.extend(text.get_text() for text in axis.get_yticklabels())
        if axis.get_legend() is not None:
            texts.extend(text.get_text() for text in axis.get_legend().get_texts())
    for legend in fig.legends:
        texts.extend(text.get_text() for text in legend.get_texts())
    return [text for text in texts if text]


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


def test_plot_lines_uses_two_panels_and_breaks_distance_lines_at_draws(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    captured = _capture_figures(monkeypatch)

    plot_lines(_figure_frame(), tmp_path)

    fig, stem = captured.pop()
    assert stem == "figure_a_diagonal_values"
    assert len(fig.axes) == 2
    value_axis, plies_axis = fig.axes
    assert len(value_axis.lines) == len(plies_axis.lines) == 3
    assert len(value_axis.texts) == 0
    assert sum(axis.get_legend() is not None for axis in fig.axes) + len(fig.legends) == 1
    for value_line, plies_line in zip(value_axis.lines, plies_axis.lines, strict=True):
        assert value_line.get_label() == plies_line.get_label()
        assert value_line.get_color() == plies_line.get_color()
        assert value_line.get_marker() == plies_line.get_marker()
        assert value_line.get_linestyle() == plies_line.get_linestyle()
        values = np.asarray(value_line.get_ydata(), dtype=float)
        plies = np.asarray(plies_line.get_ydata(), dtype=float)
        assert np.array_equal(np.isnan(plies), values == 0)
    assert [text.get_text() for text in plies_axis.texts].count("11") == 1


def test_plot_table_defaults_to_one_rule_with_value_fills_and_row_rules(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    captured = _capture_figures(monkeypatch)

    plot_table(_figure_frame(), tmp_path)

    fig, stem = captured.pop()
    assert stem == "figure_b_value_tables"
    assert len(fig.axes) == 1
    axis = fig.axes[0]
    cells = [
        patch
        for patch in axis.patches
        if patch.get_width() == pytest.approx(1) and patch.get_height() == pytest.approx(1)
    ]
    assert len(cells) == 25
    assert {colors.to_hex(cell.get_facecolor()) for cell in cells} == {
        style.ACCENT_FILL,
        style.BACKGROUND,
        style.NEUTRAL_FILL,
    }
    assert len(axis.lines) == 6
    assert all(np.array_equal(line.get_xdata(), [0, 5]) for line in axis.lines)
    assert all(line.get_color() == style.TABLE_RULE for line in axis.lines)
    assert len(axis.texts) == 25
    assert any(text.get_text() == f"{label_ja('value.first_win')}\n{label_ja('unit.plies').format(plies=13)}" for text in axis.texts)
    assert all("\n" not in text.get_text() for text in axis.texts if text.get_text() == label_ja("value.draw"))
    assert sum(item.get_legend() is not None for item in fig.axes) + len(fig.legends) == 1


def test_plot_fixed_text_is_loaded_from_labels_ja(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    captured = _capture_figures(monkeypatch)

    def sentinel_label(key: str) -> str:
        if key == "unit.plies":
            return "LABEL:unit.plies:{plies}"
        return f"LABEL:{key}"

    monkeypatch.setattr(plot_module, "label_ja", sentinel_label)
    plot_lines(_figure_frame(), tmp_path)
    plot_table(_figure_frame(), tmp_path)

    assert len(captured) == 2
    for fig, _ in captured:
        for text in _visible_texts(fig):
            assert text.startswith("LABEL:") or re.fullmatch(r"-?\d+(?:\.\d+)?", text), text


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
