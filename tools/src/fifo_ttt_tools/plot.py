"""Generate the two preregistered figure alternatives."""
from __future__ import annotations

import os
from pathlib import Path

import matplotlib.pyplot as plt
from matplotlib.patches import Patch, Rectangle
import pandas as pd

from fifo_ttt_tools.labels_ja import label_ja
from fifo_ttt_tools.load import figures_dir
from fifo_ttt_tools.style import (
    ACCENT,
    ACCENT_FILL,
    BACKGROUND,
    HAIRLINE,
    INK,
    MUTED,
    NEUTRAL_FILL,
    TABLE_RULE,
    apply_editorial_minimal,
)

CAPS = ["1", "2", "3", "4", "inf"]
RULES = ["t1", "t2", "t3"]


def _save_both(fig: plt.Figure, output_dir: Path, stem: str) -> tuple[Path, Path]:
    output_dir.mkdir(parents=True, exist_ok=True)
    png = output_dir / f"{stem}.png"
    svg = output_dir / f"{stem}.svg"
    fig.savefig(png, dpi=180, bbox_inches="tight", pad_inches=0.18)
    fig.savefig(svg, bbox_inches="tight", pad_inches=0.18)
    plt.close(fig)
    return png, svg


def plot_lines(frame: pd.DataFrame, run_dir: str | os.PathLike[str], main_rule: str = "t3") -> tuple[Path, Path]:
    """Plot diagonal values and decisive distances for all available rules."""
    apply_editorial_minimal()
    data = frame[(frame["first"] == "x") & frame["horizon"].isna()]
    fig, (value_axis, plies_axis) = plt.subplots(
        2,
        1,
        figsize=(7.2, 6.2),
        sharex=True,
        gridspec_kw={"height_ratios": [1.35, 1.0]},
    )
    styles = {
        "t1": (INK, "o", "-"),
        "t2": (MUTED, "s", "--"),
        "t3": (INK, "D", "-"),
    }
    color, marker, linestyle = styles[main_rule]
    styles[main_rule] = (ACCENT, marker, linestyle)
    value_lines = []
    annotations: set[tuple[int, int]] = set()
    for rule in RULES:
        diagonal = data[(data["rule"] == rule) & (data["kx"] == data["ko"])].copy()
        if diagonal.empty:
            continue
        diagonal["position"] = diagonal["kx"].map({cap: index for index, cap in enumerate(CAPS)})
        diagonal = diagonal.sort_values("position")
        color, marker, linestyle = styles[rule]
        value_line, = value_axis.plot(
            diagonal["position"],
            diagonal["value"],
            color=color,
            marker=marker,
            linestyle=linestyle,
            markersize=5,
            label=label_ja(f"rule.{rule}"),
        )
        value_lines.append(value_line)
        decisive_plies = diagonal["plies"].where(diagonal["value"].abs() == 1)
        plies_axis.plot(
            diagonal["position"],
            decisive_plies,
            color=color,
            marker=marker,
            linestyle=linestyle,
            markersize=5,
            label=label_ja(f"rule.{rule}"),
        )
        for position, plies in zip(diagonal["position"], decisive_plies, strict=True):
            if pd.notna(plies):
                annotations.add((int(position), int(plies)))

    for position, plies in sorted(annotations):
        plies_axis.annotate(
            str(plies),
            (position, plies),
            xytext=(0, 7),
            textcoords="offset points",
            ha="center",
            va="bottom",
            fontsize=9,
            color=MUTED,
        )

    value_axis.set_yticks(
        [-1, 0, 1],
        [label_ja("value.loss"), label_ja("value.draw"), label_ja("value.win")],
    )
    value_axis.set_ylim(-1.18, 1.18)
    value_axis.set_ylabel(label_ja("axis.value_x"))
    value_axis.grid(axis="y", color=HAIRLINE, linewidth=0.6)
    value_axis.tick_params(axis="x", labelbottom=False, length=0)
    value_axis.spines["bottom"].set_visible(False)

    plies_axis.set_xticks(range(5), [label_ja(f"cap.{cap}") for cap in CAPS])
    plies_axis.set_xlabel(label_ja("axis.k"))
    plies_axis.set_ylabel(label_ja("axis.plies"))
    plies_axis.set_yticks(sorted({plies for _, plies in annotations}))
    plies_axis.grid(axis="y", color=HAIRLINE, linewidth=0.6)
    plies_axis.tick_params(axis="x", length=0)
    plies_axis.margins(y=0.25)

    fig.legend(handles=value_lines, loc="lower center", bbox_to_anchor=(0.5, 0.015), ncol=3)
    fig.subplots_adjust(bottom=0.20, left=0.17, right=0.97, top=0.97, hspace=0.17)
    return _save_both(fig, figures_dir(run_dir), "figure_a_diagonal_values")


def plot_table(frame: pd.DataFrame, run_dir: str | os.PathLike[str], main_rule: str = "t3") -> tuple[Path, Path]:
    """Plot the selected rule as a 5-by-5 value-and-distance table."""
    apply_editorial_minimal()
    data = frame[(frame["first"] == "x") & frame["horizon"].isna()]
    subset = data[data["rule"] == main_rule].set_index(["kx", "ko"])
    fig, axis = plt.subplots(figsize=(7.4, 6.8))
    axis.set_xlim(0, 5)
    axis.set_ylim(5, 0)
    axis.set_aspect("equal")

    fill_colors = {-1: NEUTRAL_FILL, 0: BACKGROUND, 1: ACCENT_FILL}
    value_keys = {-1: "value.second_win", 0: "value.draw", 1: "value.first_win"}
    for row_index, kx in enumerate(CAPS):
        for column_index, ko in enumerate(CAPS):
            if (kx, ko) not in subset.index:
                raise ValueError(f"Missing condition for {main_rule}, kx={kx}, ko={ko}")
            row = subset.loc[(kx, ko)]
            value = int(row["value"])
            axis.add_patch(
                Rectangle(
                    (column_index, row_index),
                    1,
                    1,
                    facecolor=fill_colors[value],
                    edgecolor="none",
                )
            )
            text = label_ja(value_keys[value])
            if value != 0 and pd.notna(row.get("plies")):
                text += "\n" + label_ja("unit.plies").format(plies=int(row["plies"]))
            axis.text(
                column_index + 0.5,
                row_index + 0.5,
                text,
                ha="center",
                va="center",
                fontsize=10.5,
                linespacing=1.35,
            )

    for boundary in range(6):
        axis.plot([0, 5], [boundary, boundary], color=TABLE_RULE, linewidth=0.7)

    axis.set_title(label_ja(f"rule.{main_rule}"), color=INK, fontsize=13, pad=13)
    axis.set_xticks([index + 0.5 for index in range(5)], [label_ja(f"cap.{cap}") for cap in CAPS])
    axis.set_yticks([index + 0.5 for index in range(5)], [label_ja(f"cap.{cap}") for cap in CAPS])
    axis.set_xlabel(label_ja("axis.ko"), labelpad=10)
    axis.set_ylabel(label_ja("axis.kx"), labelpad=12)
    axis.tick_params(length=0)
    for spine in axis.spines.values():
        spine.set_visible(False)

    legend_handles = [
        Patch(facecolor=ACCENT_FILL, edgecolor="none", label=label_ja("value.first_win")),
        Patch(facecolor=NEUTRAL_FILL, edgecolor="none", label=label_ja("value.second_win")),
        Patch(facecolor=BACKGROUND, edgecolor=TABLE_RULE, linewidth=0.7, label=label_ja("value.draw")),
    ]
    axis.legend(
        handles=legend_handles,
        loc="upper center",
        bbox_to_anchor=(0.5, -0.20),
        ncol=3,
        handlelength=1.7,
        columnspacing=1.8,
    )
    fig.subplots_adjust(bottom=0.24, left=0.18, right=0.97, top=0.91)
    return _save_both(fig, figures_dir(run_dir), "figure_b_value_tables")
