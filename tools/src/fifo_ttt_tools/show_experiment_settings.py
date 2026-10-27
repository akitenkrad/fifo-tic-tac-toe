"""Display a run or sweep configuration.

Usage:
    fifo-ttt-tools show-experiment-settings --results-dir results/latest
    fifo-ttt-tools show-experiment-settings --results-dir results/20260101_120000 --json
"""
from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path


def _resolve_results_dir(arg: str) -> Path:
    """Resolve relative paths from the current directory and follow symlinks."""
    p = Path(arg)
    if not p.is_absolute():
        p = Path.cwd() / arg
    return Path(os.path.realpath(p))


def _find_config_file(results_dir: Path) -> tuple[Path, str]:
    run_cfg = results_dir / "config.json"
    sweep_cfg = results_dir / "sweep_config.json"
    if run_cfg.exists():
        return run_cfg, "run"
    if sweep_cfg.exists():
        return sweep_cfg, "sweep"
    raise FileNotFoundError(
        f"Configuration file not found: {results_dir}\n"
        f"  Expected: config.json (run) or sweep_config.json (sweep)"
    )


def render_config(cfg: dict, source: Path, kind: str) -> str:
    lines: list[str] = []
    lines.append("=" * 80)
    lines.append(f"Run configuration ({kind})")
    lines.append("=" * 80)
    lines.append(f"Source: {source}")
    lines.append("-" * 80)
    for key, value in cfg.items():
        lines.append(f"{key:<20}: {value}")
    lines.append("=" * 80)
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="fifo-ttt-tools show-experiment-settings",
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--results-dir", "--results_dir",
        required=True,
        help="Run directory containing config.json or sweep_config.json",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Print JSON instead of the formatted view",
    )
    args = parser.parse_args(argv)

    results_dir = _resolve_results_dir(args.results_dir)
    if not results_dir.exists():
        print(f"Error: directory does not exist: {results_dir}", file=sys.stderr)
        return 1

    cfg_path, kind = _find_config_file(results_dir)
    with cfg_path.open() as f:
        cfg = json.load(f)

    if args.json:
        payload = {"source": str(cfg_path), "kind": kind, "config": cfg}
        print(json.dumps(payload, indent=2, ensure_ascii=False))
    else:
        print(render_config(cfg, cfg_path, kind))
    return 0


if __name__ == "__main__":
    sys.exit(main())
