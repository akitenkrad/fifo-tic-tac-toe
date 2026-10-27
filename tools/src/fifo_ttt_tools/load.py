"""Load long-form runvault records into analysis tables."""
from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
from pathlib import Path
from typing import Iterable

import pandas as pd

try:
    from runvault.read import (
        config_parameters as _rv_config_parameters,
        events_table as _rv_events_table,
        figures_dir as _rv_figures_dir,
        run_scope_metrics as _rv_run_scope_metrics,
    )
except ImportError:
    _rv_config_parameters = None
    _rv_events_table = None
    _rv_figures_dir = None
    _rv_run_scope_metrics = None

CONDITION_RE = re.compile(r"^(?P<metric>[a-z0-9_]+)\.kx(?P<kx>1|2|3|4|inf)_ko(?P<ko>1|2|3|4|inf)$")


def _config_parameters(run_dir: Path) -> dict:
    if _rv_config_parameters is not None:
        return dict(_rv_config_parameters(run_dir))
    document = json.loads((run_dir / "config.json").read_text())
    return dict(document.get("parameters", document))


def _run_scope_metrics(run_dir: Path) -> dict[str, float]:
    if _rv_run_scope_metrics is not None:
        return dict(_rv_run_scope_metrics(run_dir))
    metrics = pd.read_csv(run_dir / "metrics.csv")
    if "step" in metrics:
        metrics = metrics[metrics["step"].isna()]
    if "scope" in metrics:
        metrics = metrics[metrics["scope"] == "run"]
    return {str(row["name"]): float(row["value"]) for _, row in metrics.iterrows()}


def _events_table(run_dir: Path) -> pd.DataFrame:
    if _rv_events_table is not None:
        return _rv_events_table(run_dir, kind=None)
    rows = [json.loads(line) for line in (run_dir / "events.jsonl").read_text().splitlines() if line.strip()]
    return pd.DataFrame(rows)


def figures_dir(run_dir: str | os.PathLike[str]) -> Path:
    """Return runvault's post-run figure directory outside the run directory."""
    path = Path(run_dir).resolve()
    if _rv_figures_dir is not None:
        return Path(_rv_figures_dir(path))
    if (path / "run.json").exists():
        return path.parent / "figures" / path.name
    return path / "figures"


def runvault_paths(subcommand: str, *, scratch: bool = False, results_root: str = "results") -> list[Path]:
    """List finished run directories through the runvault CLI."""
    binary = shutil.which("runvault")
    if binary is None:
        raise RuntimeError("The runvault command was not found; pass --run-dir explicitly")
    command = [
        binary,
        "path",
        "--results-root",
        results_root,
        "--experiment",
        "fifo-ttt",
        "--subcommand",
        subcommand,
        "--finished",
    ]
    if scratch:
        command.append("--scratch")
    completed = subprocess.run(command, check=True, capture_output=True, text=True)
    return [Path(line) for line in completed.stdout.splitlines() if line.strip()]


def _latest_matching_runs(
    run_dirs: Iterable[str | os.PathLike[str]],
    *,
    rule: str | None,
    first: str | None,
    horizon: int | None | object,
) -> list[Path]:
    matches: dict[tuple[str, str, int | None], Path] = {}
    for raw_path in run_dirs:
        path = Path(raw_path)
        parameters = _config_parameters(path)
        key = (str(parameters.get("rule")), str(parameters.get("first", "x")), parameters.get("horizon"))
        if rule is not None and key[0] != rule:
            continue
        if first is not None and key[1] != first:
            continue
        if horizon is not _ANY_HORIZON and key[2] != horizon:
            continue
        previous = matches.get(key)
        if previous is None or path.name > previous.name:
            matches[key] = path
    return sorted(matches.values())


_ANY_HORIZON = object()


def load_solve_runs(
    rule: str | None = None,
    first: str | None = "x",
    horizon: int | None | object = None,
    *,
    run_dirs: Iterable[str | os.PathLike[str]] | None = None,
    scratch: bool = False,
    all_horizons: bool = False,
) -> pd.DataFrame:
    """Load the latest solve run for each matching rule and sensitivity setting."""
    candidates = list(run_dirs) if run_dirs is not None else runvault_paths("solve", scratch=scratch)
    horizon_filter = _ANY_HORIZON if all_horizons else horizon
    selected = _latest_matching_runs(candidates, rule=rule, first=first, horizon=horizon_filter)
    if not selected:
        raise FileNotFoundError("No matching solve run was found")
    return _load_condition_runs(selected, required_metric="value")


def load_naive_runs(
    rule: str | None = None,
    *,
    run_dirs: Iterable[str | os.PathLike[str]] | None = None,
    scratch: bool = False,
    node_cap: int | None = 1_000_000_000,
) -> pd.DataFrame:
    """Load the latest naive run for each rule at the requested node cap."""
    candidates = list(run_dirs) if run_dirs is not None else runvault_paths("naive", scratch=scratch)
    if node_cap is not None:
        candidates = [
            path
            for path in candidates
            if int(_config_parameters(Path(path)).get("node_cap", -1)) == node_cap
        ]
    selected = _latest_matching_runs(candidates, rule=rule, first="x", horizon=None)
    if not selected:
        raise FileNotFoundError("No matching naive run was found")
    return _load_condition_runs(selected, required_metric="naive_capped")


def _load_condition_runs(run_dirs: Iterable[Path], required_metric: str) -> pd.DataFrame:
    rows: list[dict] = []
    for run_dir in run_dirs:
        parameters = _config_parameters(run_dir)
        metrics = _run_scope_metrics(run_dir)
        conditions: dict[tuple[str, str], dict] = {}
        for name, value in metrics.items():
            match = CONDITION_RE.match(name)
            if match is None:
                continue
            condition = (match.group("kx"), match.group("ko"))
            conditions.setdefault(condition, {})[match.group("metric")] = value
        for (kx, ko), values in conditions.items():
            if required_metric not in values:
                continue
            rows.append(
                {
                    "run_dir": str(run_dir),
                    "rule": str(parameters["rule"]),
                    "first": str(parameters.get("first", "x")),
                    "horizon": parameters.get("horizon"),
                    "node_cap": parameters.get("node_cap"),
                    "kx": kx,
                    "ko": ko,
                    **values,
                }
            )
    frame = pd.DataFrame(rows)
    if frame.empty:
        raise ValueError(f"No {required_metric} condition metrics were found")
    order = {"1": 1, "2": 2, "3": 3, "4": 4, "inf": 5}
    return frame.assign(_kx=frame["kx"].map(order), _ko=frame["ko"].map(order)).sort_values(
        ["rule", "first", "horizon", "_kx", "_ko"], na_position="first"
    ).drop(columns=["_kx", "_ko"]).reset_index(drop=True)


def load_principal_variations(frame: pd.DataFrame, rule: str = "t3") -> pd.DataFrame:
    """Load one recorded optimal-policy line for each decisive main-rule condition."""
    selected = frame[
        (frame["rule"] == rule)
        & (frame["first"] == "x")
        & frame["horizon"].isna()
        & (frame["value"] != 0)
    ]
    rows: list[dict] = []
    for run_dir_text in selected["run_dir"].drop_duplicates():
        run_dir = Path(run_dir_text)
        events = _events_table(run_dir)
        for condition in selected[selected["run_dir"] == run_dir_text].itertuples():
            unit_id = f"pv.kx{condition.kx}_ko{condition.ko}"
            game = events[events["unit_id"] == unit_id].sort_values("t")
            plies = game[game["schema"] == "x.fifo-tic-tac-toe.ply"]
            endings = game[game["schema"] == "x.fifo-tic-tac-toe.game_end"]
            if endings.empty:
                raise ValueError(f"Missing game_end event for {unit_id} in {run_dir}")
            rows.append(
                {
                    "rule": condition.rule,
                    "kx": condition.kx,
                    "ko": condition.ko,
                    "value": int(condition.value),
                    "plies": int(condition.plies),
                    "pv": "-".join(str(int(cell)) for cell in plies["cell"]),
                    "reason": str(endings.iloc[-1]["reason"]),
                }
            )
    return pd.DataFrame(rows)
