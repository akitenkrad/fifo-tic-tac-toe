"""Generate the two preregistered figure alternatives."""
from __future__ import annotations

import os
from pathlib import Path

import matplotlib.pyplot as plt
import pandas as pd

from fifo_ttt_tools.labels_ja import label_ja
from fifo_ttt_tools.load import figures_dir
from fifo_ttt_tools.style import ACCENT, HAIRLINE, INK, MUTED, TABLE_RULE, apply_editorial_minimal

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
    """Plot diagonal cap values for all available rules."""
    apply_editorial_minimal()
    data = frame[(frame["first"] == "x") & frame["horizon"].isna()]
    fig, axis = plt.subplots(figsize=(7.2, 4.8))
    styles = {
        "t1": (INK, "o", "-"),
        "t2": (MUTED, "s", "--"),
        "t3": (INK, "D", "-"),
    }
    color, marker, linestyle = styles[main_rule]
    styles[main_rule] = (ACCENT, marker, linestyle)
    annotation_offsets = {"t1": (5, -14), "t2": (-12, 8), "t3": (7, 8)}
    for rule in RULES:
        diagonal = data[(data["rule"] == rule) & (data["kx"] == data["ko"])].copy()
        if diagonal.empty:
            continue
        diagonal["position"] = diagonal["kx"].map({cap: index for index, cap in enumerate(CAPS)})
        diagonal = diagonal.sort_values("position")
        color, marker, linestyle = styles[rule]
        axis.plot(
            diagonal["position"],
            diagonal["value"],
            color=color,
            marker=marker,
            linestyle=linestyle,
            markersize=5,
            label=label_ja(f"rule.{rule}"),
        )
        for row in diagonal.itertuples():
            if pd.notna(getattr(row, "plies", None)):
                axis.annotate(
                    str(int(row.plies)),
                    (row.position, row.value),
                    xytext=annotation_offsets[rule],
                    textcoords="offset points",
                    fontsize=9,
                    color=MUTED,
                )
    axis.set_xticks(range(5), [label_ja(f"cap.{cap}") for cap in CAPS], fontfamily="Georgia")
    axis.set_yticks(
        [-1, 0, 1],
        [label_ja("value.loss"), label_ja("value.draw"), label_ja("value.win")],
    )
    axis.set_xlabel(label_ja("axis.k"))
    axis.set_ylabel(label_ja("axis.value_x"))
    axis.grid(axis="y", color=HAIRLINE, linewidth=0.6)
    axis.legend(loc="upper center", bbox_to_anchor=(0.5, -0.2), ncol=3)
    fig.subplots_adjust(bottom=0.28, left=0.16, right=0.97, top=0.95)
    return _save_both(fig, figures_dir(run_dir), "figure_a_diagonal_values")


def plot_table(frame: pd.DataFrame, run_dir: str | os.PathLike[str], main_rule: str = "t3") -> tuple[Path, Path]:
    """Plot one 5-by-5 value-and-distance table for each rule."""
    apply_editorial_minimal()
    data = frame[(frame["first"] == "x") & frame["horizon"].isna()]
    fig, axes = plt.subplots(1, 3, figsize=(13.8, 5.2), sharex=True, sharey=True)
    for axis, rule in zip(axes, RULES, strict=True):
        subset = data[data["rule"] == rule].set_index(["kx", "ko"])
        axis.set_xlim(0, 5)
        axis.set_ylim(5, 0)
        for boundary in range(6):
            color = HAIRLINE if boundary in {0, 5} else TABLE_RULE
            width = 1.1 if boundary in {0, 5} else 0.6
            axis.plot([0, 5], [boundary, boundary], color=color, linewidth=width)
        for row_index, kx in enumerate(CAPS):
            for column_index, ko in enumerate(CAPS):
                if (kx, ko) not in subset.index:
                    raise ValueError(f"Missing condition for {rule}, kx={kx}, ko={ko}")
                row = subset.loc[(kx, ko)]
                value_key = {-1: "loss_short", 0: "draw_short", 1: "win_short"}[int(row["value"])]
                text = label_ja(f"value.{value_key}")
                if pd.notna(row.get("plies")):
                    text += f"\n{int(row['plies'])}"
                axis.text(column_index + 0.5, row_index + 0.5, text, ha="center", va="center", fontsize=11)
        heading_color = ACCENT if rule == main_rule else INK
        axis.text(0.5, 1.04, label_ja(f"rule.{rule}"), transform=axis.transAxes, ha="center", color=heading_color)
        axis.set_xticks([index + 0.5 for index in range(5)], [label_ja(f"cap.{cap}") for cap in CAPS])
        axis.set_yticks([index + 0.5 for index in range(5)], [label_ja(f"cap.{cap}") for cap in CAPS])
        axis.tick_params(length=0)
        for spine in axis.spines.values():
            spine.set_visible(False)
    axes[0].set_ylabel(label_ja("axis.kx"))
    axes[1].set_xlabel(label_ja("axis.ko"))
    fig.text(0.5, 0.025, label_ja("note.plies"), ha="center", color=MUTED, fontsize=11)
    fig.subplots_adjust(bottom=0.14, left=0.08, right=0.98, top=0.88, wspace=0.16)
    return _save_both(fig, figures_dir(run_dir), "figure_b_value_tables")
