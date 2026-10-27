"""Render preregistered result tables as Markdown and CSV."""
from __future__ import annotations

import io
from pathlib import Path

import pandas as pd

from fifo_ttt_tools.labels_ja import label_ja
from fifo_ttt_tools.load import figures_dir, load_principal_variations


def _display_cap(value: str) -> str:
    return label_ja(f"cap.{value}")


def _display_value(value: float) -> str:
    return label_ja({-1: "value.loss", 0: "value.draw", 1: "value.win"}[int(value)])


def _display_node_cap(value: float) -> str:
    cap = int(value)
    rendered = "10^9" if cap == 1_000_000_000 else f"{cap:,}"
    return label_ja("table.cap_exceeded").format(cap=rendered)


def _markdown(table: pd.DataFrame) -> str:
    headers = [str(column) for column in table.columns]
    lines = [
        "| " + " | ".join(headers) + " |",
        "| " + " | ".join("---" for _ in headers) + " |",
    ]
    for row in table.itertuples(index=False, name=None):
        lines.append("| " + " | ".join("" if pd.isna(value) else str(value) for value in row) + " |")
    return "\n".join(lines) + "\n"


def render_result_table(frame: pd.DataFrame) -> tuple[str, str]:
    """Render rules by all cap pairs with value and optional distance."""
    rows = []
    for row in frame.itertuples():
        rows.append(
            {
                "rule": row.rule.upper(),
                "kx": _display_cap(row.kx),
                "ko": _display_cap(row.ko),
                "value": _display_value(row.value),
                "plies": "" if pd.isna(getattr(row, "plies", None)) else int(row.plies),
            }
        )
    table = pd.DataFrame(rows)
    markdown = _markdown(table.rename(columns={"rule": "Rule", "kx": "kX", "ko": "kO"}))
    csv_buffer = io.StringIO()
    table.to_csv(csv_buffer, index=False)
    return markdown, csv_buffer.getvalue()


def render_horizon_table(frame: pd.DataFrame) -> tuple[str, str]:
    """Render the three horizons beside the unbounded X-first results."""
    frame = frame[frame["first"] == "x"]
    columns = ["rule", "first", "horizon", "kx", "ko", "value", "plies"]
    table = frame.reindex(columns=columns).copy()
    table["horizon"] = table["horizon"].map(lambda value: "none" if pd.isna(value) else int(value))
    table["value"] = table["value"].astype(int)
    table["plies"] = table["plies"].astype("Int64")
    markdown = _markdown(table)
    csv_buffer = io.StringIO()
    table.to_csv(csv_buffer, index=False)
    return markdown, csv_buffer.getvalue()


def render_first_player_table(frame: pd.DataFrame) -> tuple[str, str]:
    """Render X-first and O-first unbounded results."""
    frame = frame[frame["horizon"].isna()]
    columns = ["rule", "first", "kx", "ko", "value", "plies"]
    table = frame.reindex(columns=columns).copy()
    table["value"] = table["value"].astype(int)
    table["plies"] = table["plies"].astype("Int64")
    markdown = _markdown(table)
    csv_buffer = io.StringIO()
    table.to_csv(csv_buffer, index=False)
    return markdown, csv_buffer.getvalue()


def render_naive_table(solve: pd.DataFrame, naive: pd.DataFrame) -> tuple[str, str]:
    """Render reachable states beside completed or capped naive node counts."""
    left = solve[["rule", "kx", "ko", "n_states"]]
    right = naive[["rule", "kx", "ko", "naive_nodes", "naive_capped", "node_cap"]]
    table = left.merge(right, on=["rule", "kx", "ko"], how="inner")
    table["naive_nodes_display"] = table.apply(
        lambda row: _display_node_cap(row["node_cap"])
        if bool(row["naive_capped"])
        else str(int(row["naive_nodes"])),
        axis=1,
    )
    output = table[["rule", "kx", "ko", "n_states", "naive_nodes_display", "naive_capped"]].copy()
    output["n_states"] = output["n_states"].astype(int)
    output["naive_capped"] = output["naive_capped"].astype(int)
    markdown = _markdown(output)
    csv_buffer = io.StringIO()
    output.to_csv(csv_buffer, index=False)
    return markdown, csv_buffer.getvalue()


def write_tables(
    frame: pd.DataFrame,
    run_dir: str | Path,
    *,
    naive: pd.DataFrame | None = None,
) -> list[Path]:
    """Write the main, sensitivity, and optional naive comparison tables."""
    output_dir = figures_dir(run_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    outputs: list[Path] = []
    baseline = frame[(frame["first"] == "x") & frame["horizon"].isna()]
    for stem, rendered in [
        ("table_values", render_result_table(baseline)),
        ("table_horizons", render_horizon_table(frame)),
        ("table_first_player", render_first_player_table(frame)),
        ("table_principal_variations", _render_plain(load_principal_variations(frame))),
    ]:
        for suffix, contents in zip(["md", "csv"], rendered, strict=True):
            path = output_dir / f"{stem}.{suffix}"
            path.write_text(contents)
            outputs.append(path)
    if naive is not None:
        rendered = render_naive_table(frame[(frame["first"] == "x") & frame["horizon"].isna()], naive)
        for suffix, contents in zip(["md", "csv"], rendered, strict=True):
            path = output_dir / f"table_naive.{suffix}"
            path.write_text(contents)
            outputs.append(path)
    return outputs


def _render_plain(table: pd.DataFrame) -> tuple[str, str]:
    csv_buffer = io.StringIO()
    table.to_csv(csv_buffer, index=False)
    return _markdown(table), csv_buffer.getvalue()
