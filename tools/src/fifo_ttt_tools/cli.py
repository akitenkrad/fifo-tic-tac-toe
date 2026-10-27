"""Command-line interface for FIFO tic-tac-toe analysis tools."""
from __future__ import annotations

import argparse
from pathlib import Path

from fifo_ttt_tools.load import load_naive_runs, load_solve_runs
from fifo_ttt_tools.plot import plot_lines, plot_table
from fifo_ttt_tools.table import write_tables


def _common_run_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--run-dir", action="append", type=Path, help="Solve run directory; may be repeated")
    parser.add_argument("--scratch", action="store_true", help="Resolve scratch runs instead of production runs")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="fifo-ttt-tools", description="FIFO tic-tac-toe analysis tools")
    commands = parser.add_subparsers(dest="command", required=True)

    plot_parser = commands.add_parser("plot", help="Generate both preregistered figure alternatives")
    _common_run_arguments(plot_parser)
    plot_parser.add_argument("--rule", choices=["t1", "t2", "t3"], default="t3", help="Main rule; all rules remain visible for comparison")

    table_parser = commands.add_parser("table", help="Generate Markdown and CSV result tables")
    _common_run_arguments(table_parser)
    table_parser.add_argument("--naive-run-dir", action="append", type=Path, help="Naive run directory; may be repeated")
    table_parser.add_argument("--naive", action="store_true", help="Include the naive-node comparison table")

    settings_parser = commands.add_parser("show-experiment-settings", help="Display a runvault config envelope")
    settings_parser.add_argument("--results-dir", required=True)
    settings_parser.add_argument("--json", action="store_true")
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    if args.command == "show-experiment-settings":
        from fifo_ttt_tools.show_experiment_settings import main as settings_main

        forwarded = ["--results-dir", args.results_dir]
        if args.json:
            forwarded.append("--json")
        return settings_main(forwarded)

    frame = load_solve_runs(first=None, run_dirs=args.run_dir, scratch=args.scratch, all_horizons=True)
    anchor = Path(frame.iloc[-1]["run_dir"])
    if args.command == "plot":
        if args.rule not in set(frame["rule"]):
            raise ValueError(f"No solve run was loaded for main rule {args.rule}")
        outputs = [*plot_lines(frame, anchor, args.rule), *plot_table(frame, anchor, args.rule)]
    else:
        naive = None
        if args.naive or args.naive_run_dir:
            node_cap = None if args.naive_run_dir else 1_000_000_000
            naive = load_naive_runs(run_dirs=args.naive_run_dir, scratch=args.scratch, node_cap=node_cap)
        outputs = write_tables(frame, anchor, naive=naive)
    for output in outputs:
        print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
