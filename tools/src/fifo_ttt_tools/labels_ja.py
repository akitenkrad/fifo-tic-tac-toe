"""Strict Japanese labels used by generated figures and tables."""

LABELS_JA: dict[str, str] = {
    "axis.k": "手の保持上限 k（kX = kO）",
    "axis.kx": "X の保持上限 kX",
    "axis.ko": "O の保持上限 kO",
    "axis.value_x": "X から見た値",
    "axis.plies": "決着までの手数",
    "legend.rules": "規則",
    "note.plies": "数字は決着までの手数",
    "rule.t1": "規則 T1",
    "rule.t2": "規則 T2",
    "rule.t3": "規則 T3",
    "value.win": "勝ち",
    "value.draw": "引き分け",
    "value.loss": "負け",
    "value.win_short": "勝",
    "value.draw_short": "引分",
    "value.loss_short": "負",
    "value.first_win": "先手勝ち",
    "value.second_win": "後手勝ち",
    "unit.plies": "{plies} 手",
    "cap.1": "1",
    "cap.2": "2",
    "cap.3": "3",
    "cap.4": "4",
    "cap.inf": "∞",
    "table.cap_exceeded": "上限 {cap} 超",
    "table.unjudged": "未判定",
    "table.matched": "一致",
}


def label_ja(key: str) -> str:
    """Return a registered Japanese label and reject silent English fallback."""
    return LABELS_JA[key]
