"""Editorial-minimal matplotlib styling."""
from __future__ import annotations

from matplotlib import font_manager, rcParams

BACKGROUND = "#fcfbf8"
INK = "#2b2926"
MUTED = "#8a8579"
HAIRLINE = "#d8d3c8"
SERIF_NUMERAL = "#cdc7ba"
ACCENT = "#d94f30"
ACCENT_TEXT = "#b03c22"
ACCENT_FILL = "#f6e3dd"
NEUTRAL_FILL = "#e4dfd4"
TABLE_RULE = "#ece8df"

JAPANESE_FONT_FALLBACKS = [
    "Hiragino Sans",
    "Noto Sans CJK JP",
    "IPAexGothic",
    "Yu Gothic",
    "MS Gothic",
    "Arial Unicode MS",
]


def select_japanese_font() -> str:
    """Select the first installed font in the declared Japanese fallback list."""
    installed = {font.name for font in font_manager.fontManager.ttflist}
    for family in JAPANESE_FONT_FALLBACKS:
        if family in installed:
            return family
    raise RuntimeError(
        "No supported Japanese font is installed; tried: "
        + ", ".join(JAPANESE_FONT_FALLBACKS)
    )


def apply_editorial_minimal() -> str:
    """Apply the project figure tokens and return the selected Japanese font."""
    japanese_font = select_japanese_font()
    rcParams.update(
        {
            "figure.facecolor": BACKGROUND,
            "axes.facecolor": BACKGROUND,
            "savefig.facecolor": BACKGROUND,
            "text.color": INK,
            "axes.labelcolor": INK,
            "axes.edgecolor": HAIRLINE,
            "axes.linewidth": 0.6,
            "axes.spines.top": False,
            "axes.spines.right": False,
            "font.family": "sans-serif",
            "font.sans-serif": JAPANESE_FONT_FALLBACKS,
            "font.size": 14,
            "xtick.color": MUTED,
            "ytick.color": MUTED,
            "xtick.labelsize": 11,
            "ytick.labelsize": 11,
            "legend.frameon": False,
            "legend.fontsize": 11,
            "lines.linewidth": 1.5,
            "svg.fonttype": "none",
        }
    )
    return japanese_font
